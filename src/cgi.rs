use crate::config::{Route, SingleServerConfig};
use crate::http::request::Request;
use crate::http::response::Response;
use crate::io::kqueue::wait_for_pid;
use crate::session::Session;
use log::error;
use std::collections::HashMap;
use std::ffi::CString;
use std::io::{Read, Write};
use std::os::fd::FromRawFd;
use std::path::Path;
use std::time::Duration;

fn to_cstring<S: AsRef<[u8]>>(s: S) -> Result<CString, ()> {
    CString::new(s.as_ref()).map_err(|_| ())
}

pub fn handle_cgi(
    request: &Request,
    route: &Route,
    config: &SingleServerConfig,
    cgi_path: &Path,
    cgi_executor: &str,
    session: &mut Option<&mut Session>,
) -> Response {
    match handle_cgi_internal(request, route, config, cgi_path, cgi_executor, session) {
        Ok(resp) => resp,
        Err(resp) => resp,
    }
}

fn handle_cgi_internal(
    request: &Request,
    _route: &Route,
    config: &SingleServerConfig,
    cgi_path: &Path,
    cgi_executor: &str,
    _session: &mut Option<&mut Session>,
) -> Result<Response, Response> {
    let mut pipe_stdin = [0; 2];
    let mut pipe_stdout = [0; 2];

    if unsafe { libc::pipe(pipe_stdin.as_mut_ptr()) } < 0 {
        return Err(Response::new(
            500,
            b"CGI Error: Failed to create stdin pipe".to_vec(),
            config,
            Some(request),
        ));
    }
    if unsafe { libc::pipe(pipe_stdout.as_mut_ptr()) } < 0 {
        unsafe {
            libc::close(pipe_stdin[0]);
            libc::close(pipe_stdin[1]);
        }
        return Err(Response::new(
            500,
            b"CGI Error: Failed to create stdout pipe".to_vec(),
            config,
            Some(request),
        ));
    }

    let pid = unsafe { libc::fork() };

    if pid < 0 {
        unsafe {
            libc::close(pipe_stdin[0]);
            libc::close(pipe_stdin[1]);
            libc::close(pipe_stdout[0]);
            libc::close(pipe_stdout[1]);
        }
        Err(Response::new(
            500,
            b"CGI Error: Failed to fork process".to_vec(),
            config,
            Some(request),
        ))
    } else if pid == 0 {
        unsafe {
            libc::close(pipe_stdin[1]);
            libc::dup2(pipe_stdin[0], libc::STDIN_FILENO);
            libc::close(pipe_stdin[0]);

            libc::close(pipe_stdout[0]);
            libc::dup2(pipe_stdout[1], libc::STDOUT_FILENO);
            libc::close(pipe_stdout[1]);

            let cgi_path_str = cgi_path.to_str().unwrap_or_default();
            let cgi_path_c = to_cstring(cgi_path_str).map_err(|_| libc::_exit(1))?;
            let cgi_executor_c = to_cstring(cgi_executor).map_err(|_| libc::_exit(1))?;

            let args = [
                cgi_executor_c.as_ptr(),
                cgi_path_c.as_ptr(),
                std::ptr::null(),
            ];

            let path_info = to_cstring(request.path.as_bytes()).map_err(|_| libc::_exit(1))?;
            let request_method = to_cstring(request.method.as_bytes()).map_err(|_| libc::_exit(1))?;
            let query_string = to_cstring(
                request
                    .query_params
                    .iter()
                    .map(|(k, v)| format!("{k}={v}"))
                    .collect::<Vec<String>>()
                    .join("&")
                    .as_bytes(),
            )
            .map_err(|_| libc::_exit(1))?;
            let content_type = to_cstring(
                request
                    .headers
                    .get("content-type")
                    .map(|s| s.as_bytes())
                    .unwrap_or_default(),
            )
            .map_err(|_| libc::_exit(1))?;
            let content_length = to_cstring(request.body.len().to_string().as_bytes()).map_err(|_| libc::_exit(1))?;

            let mut env_vars = Vec::new();
            env_vars.push(
                to_cstring(format!(
                    "PATH_INFO={}",
                    path_info.to_str().unwrap_or_default()
                ))
                .map_err(|_| libc::_exit(1))?,
            );
            env_vars.push(
                to_cstring(format!(
                    "REQUEST_METHOD={}",
                    request_method.to_str().unwrap_or_default()
                ))
                .map_err(|_| libc::_exit(1))?,
            );
            env_vars.push(
                to_cstring(format!(
                    "QUERY_STRING={}",
                    query_string.to_str().unwrap_or_default()
                ))
                .map_err(|_| libc::_exit(1))?,
            );
            env_vars.push(
                to_cstring(format!(
                    "CONTENT_TYPE={}",
                    content_type.to_str().unwrap_or_default()
                ))
                .map_err(|_| libc::_exit(1))?,
            );
            env_vars.push(
                to_cstring(format!(
                    "CONTENT_LENGTH={}",
                    content_length.to_str().unwrap_or_default()
                ))
                .map_err(|_| libc::_exit(1))?,
            );

            let mut env_ptrs: Vec<*const libc::c_char> =
                env_vars.iter().map(|c_str| c_str.as_ptr()).collect();
            env_ptrs.push(std::ptr::null());

            libc::execve(cgi_path_c.as_ptr(), args.as_ptr(), env_ptrs.as_ptr());
            libc::_exit(1);
        }
    } else {
        unsafe {
            libc::close(pipe_stdin[0]);
            libc::close(pipe_stdout[1]);

            let mut stdin_writer = std::fs::File::from_raw_fd(pipe_stdin[1]);
            let mut stdout_reader = std::fs::File::from_raw_fd(pipe_stdout[0]);

            if let Err(e) = stdin_writer.write_all(&request.body) {
                error!("CGI Parent: Error writing to stdin pipe: {e}");
            }
            drop(stdin_writer);

            let timeout = Duration::from_millis(config.cgi_timeout);
            match wait_for_pid(pid, timeout) {
                Ok(true) => {
                    let mut status = 0;
                    libc::waitpid(pid, &mut status, 0);
                }
                Ok(false) => {
                    libc::kill(pid, libc::SIGKILL);
                    libc::waitpid(pid, &mut 0, 0);
                    return Err(Response::new(
                        504,
                        b"CGI Error: Script timed out".to_vec(),
                        config,
                        Some(request),
                    ));
                }
                Err(e) => {
                    return Err(Response::new(
                        500,
                        format!("CGI Error: Failed to wait for child process: {e}").into_bytes(),
                        config,
                        Some(request),
                    ));
                }
            }

            let mut cgi_output = Vec::new();
            if let Err(e) = stdout_reader.read_to_end(&mut cgi_output) {
                error!("CGI Parent: Error reading from stdout pipe: {e}");
            }
            drop(stdout_reader);

            let cgi_output_str = String::from_utf8_lossy(&cgi_output);

            let all_lines: Vec<&str> = cgi_output_str.lines().collect();

            let mut headers = HashMap::new();
            let mut body_start = 0;
            for (i, line) in all_lines.iter().enumerate() {
                if line.is_empty() {
                    body_start = i + 1;
                    break;
                }
                let mut parts = line.splitn(2, ": ");
                if let (Some(key), Some(value)) = (parts.next(), parts.next()) {
                    headers.insert(key.to_string(), value.to_string());
                }
            }

            let body = all_lines
                .into_iter()
                .skip(body_start)
                .collect::<Vec<&str>>()
                .join("\n")
                .into_bytes();

            let mut status_code = 200;
            if let Some(status_header) = headers.remove("Status") {
                if let Some(code_str) = status_header.split_whitespace().next() {
                    if let Ok(code) = code_str.parse::<u16>() {
                        status_code = code;
                    }
                }
            }

            let mut response = Response::new(status_code, body, config, Some(request));
            for (key, value) in headers {
                response.headers.insert(key, value);
            }
            Ok(response)
        }
    }
}
