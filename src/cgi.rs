use crate::config::{Route, ServerConfig};
use crate::http::request::Request;
use crate::http::response::Response;
use std::collections::HashMap;
use std::ffi::CString;
use std::os::fd::FromRawFd;
use std::path::Path;

use crate::session::Session;

fn to_cstring<S: AsRef<[u8]>>(s: S) -> Result<CString, Response> {
    CString::new(s.as_ref())
        .map_err(|_| Response::new(500, b"Internal Server Error: Invalid CString".to_vec()))
}

pub fn handle_cgi(
    request: &Request,
    route: &Route,
    config: &ServerConfig,
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
    _config: &ServerConfig,
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
        ))
    } else if pid == 0 {
        // Child process

        unsafe {
            libc::close(pipe_stdin[1]); // Close write end of stdin pipe
            libc::dup2(pipe_stdin[0], libc::STDIN_FILENO);
            libc::close(pipe_stdin[0]);

            libc::close(pipe_stdout[0]); // Close read end of stdout pipe
            libc::dup2(pipe_stdout[1], libc::STDOUT_FILENO);
            libc::close(pipe_stdout[1]);

            let cgi_path_str = cgi_path.to_str().unwrap_or_default();
            let cgi_path_c = to_cstring(cgi_path_str)?;
            let cgi_executor_c = to_cstring(cgi_executor)?;

            let args = [
                cgi_executor_c.as_ptr(),
                cgi_path_c.as_ptr(),
                std::ptr::null(),
            ];

            let path_info = to_cstring(request.path.as_bytes())?;
            let request_method = to_cstring(request.method.as_bytes())?;
            let query_string = to_cstring(
                request
                    .query_params
                    .iter()
                    .map(|(k, v)| format!("{k}={v}"))
                    .collect::<Vec<String>>()
                    .join("&")
                    .as_bytes(),
            )?;
            let content_type = to_cstring(
                request
                    .headers
                    .get("Content-Type")
                    .map(|s| s.as_bytes())
                    .unwrap_or_default(),
            )?;
            let content_length = to_cstring(request.body.len().to_string().as_bytes())?;

            let mut env_vars = Vec::new();
            env_vars.push(to_cstring(format!(
                "PATH_INFO={}",
                path_info.to_str().unwrap_or_default()
            ))?);
            env_vars.push(to_cstring(format!(
                "REQUEST_METHOD={}",
                request_method.to_str().unwrap_or_default()
            ))?);
            env_vars.push(to_cstring(format!(
                "QUERY_STRING={}",
                query_string.to_str().unwrap_or_default()
            ))?);
            env_vars.push(to_cstring(format!(
                "CONTENT_TYPE={}",
                content_type.to_str().unwrap_or_default()
            ))?);
            env_vars.push(to_cstring(format!(
                "CONTENT_LENGTH={}",
                content_length.to_str().unwrap_or_default()
            ))?);

            let mut env_ptrs: Vec<*const libc::c_char> =
                env_vars.iter().map(|c_str| c_str.as_ptr()).collect();
            env_ptrs.push(std::ptr::null());

            libc::execve(cgi_path_c.as_ptr(), args.as_ptr(), env_ptrs.as_ptr());
            // If execve returns, an error occurred
            libc::_exit(1);
        }
    } else {
        // Parent process

        unsafe {
            libc::close(pipe_stdin[0]); // Close read end of stdin pipe
            libc::close(pipe_stdout[1]); // Close write end of stdout pipe

            let stdin_writer = std::fs::File::from_raw_fd(pipe_stdin[1]);
            let stdout_reader = std::fs::File::from_raw_fd(pipe_stdout[0]);

            // Write request body to CGI stdin

            drop(stdin_writer); // Close stdin pipe for CGI

            // Read CGI stdout
            let cgi_output = Vec::new();

            drop(stdout_reader); // Close stdout pipe for CGI

            let mut status = 0;
            libc::waitpid(pid, &mut status, 0);


            // Parse CGI output into HTTP response
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



            let mut response = Response::new(200, body);
            for (key, value) in headers {
                response.headers.insert(key, value);
            }
            Ok(response)
        }
    }
}
