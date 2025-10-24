use crate::config::{Route, SingleServerConfig};
use crate::http::request::Request;
use crate::http::response::Response;
use std::collections::HashMap;
use std::ffi::CString;
use std::io::{Read, Write};
use std::os::fd::FromRawFd;
use std::path::Path;
use std::time::Duration;

use crate::io::kqueue::wait_for_pid;
use crate::session::Session;

/// Converts a Rust string slice to a CString, handling potential null bytes.
/// Returns a `Result` which is an `Err` containing a 500 Internal Server Error
/// response if the conversion fails.
fn to_cstring<S: AsRef<[u8]>>(s: S) -> Result<CString, Response> {
    CString::new(s.as_ref())
        .map_err(|_| Response::new(500, b"Internal Server Error: Invalid CString".to_vec(), "close".to_string()))
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

/// Handles the execution of a CGI script.
///
/// This function sets up pipes for inter-process communication, forks a new process,
/// and executes the CGI script in the child process while the parent handles I/O.
fn handle_cgi_internal(
    request: &Request,
    _route: &Route,
    config: &SingleServerConfig,
    cgi_path: &Path,
    cgi_executor: &str,
    _session: &mut Option<&mut Session>,
) -> Result<Response, Response> {
    // Create pipes for stdin and stdout communication with the CGI script.
    // pipe_stdin[0] is read end, pipe_stdin[1] is write end.
    // pipe_stdout[0] is read end, pipe_stdout[1] is write end.
    let mut pipe_stdin = [0; 2];
    let mut pipe_stdout = [0; 2];

    // Create stdin pipe for the CGI process
    if unsafe { libc::pipe(pipe_stdin.as_mut_ptr()) } < 0 {
        return Err(Response::new(
            500,
            b"CGI Error: Failed to create stdin pipe".to_vec(),
            config.connection_type.clone(),
        ));
    }
    // Create stdout pipe for the CGI process
    if unsafe { libc::pipe(pipe_stdout.as_mut_ptr()) } < 0 {
        unsafe {
            // Close previously created stdin pipes if stdout pipe creation fails
            libc::close(pipe_stdin[0]);
            libc::close(pipe_stdin[1]);
        }
        return Err(Response::new(
            500,
            b"CGI Error: Failed to create stdout pipe".to_vec(),
            config.connection_type.clone(),
        ));
    }

    // Fork a new process. pid will be 0 in the child, >0 in the parent (child's PID), and -1 on error.
    let pid = unsafe { libc::fork() };

    if pid < 0 {
        // Fork failed, close all pipe file descriptors
        unsafe {
            libc::close(pipe_stdin[0]);
            libc::close(pipe_stdin[1]);
            libc::close(pipe_stdout[0]);
            libc::close(pipe_stdout[1]);
        }
        Err(Response::new(
            500,
            b"CGI Error: Failed to fork process".to_vec(),
            config.connection_type.clone(),
        ))
    } else if pid == 0 {
        // Child process: This is where the CGI script will be executed.

        unsafe {
            // Close the write end of the stdin pipe and redirect the read end to STDIN
            libc::close(pipe_stdin[1]);
            libc::dup2(pipe_stdin[0], libc::STDIN_FILENO);
            libc::close(pipe_stdin[0]);

            // Close the read end of the stdout pipe and redirect the write end to STDOUT
            libc::close(pipe_stdout[0]);
            libc::dup2(pipe_stdout[1], libc::STDOUT_FILENO);
            libc::close(pipe_stdout[1]);

            // Prepare CGI script path and executor as CStrings
            let cgi_path_str = cgi_path.to_str().unwrap_or_default();
            let cgi_path_c = to_cstring(cgi_path_str)?;
            let cgi_executor_c = to_cstring(cgi_executor)?;

            // Arguments for execve: first is the executor, second is the script path, then null
            let args = [
                cgi_executor_c.as_ptr(),
                cgi_path_c.as_ptr(),
                std::ptr::null(),
            ];

            // Prepare environment variables for the CGI script
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
                    .get("content-type")
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

            // Convert environment variables to a suitable format for execve
            let mut env_ptrs: Vec<*const libc::c_char> =
                env_vars.iter().map(|c_str| c_str.as_ptr()).collect();
            env_ptrs.push(std::ptr::null()); // execve expects a null-terminated array

            // Execute the CGI script. If this call succeeds, the child process will be replaced
            // by the CGI script. If it returns, an error occurred.
            libc::execve(cgi_path_c.as_ptr(), args.as_ptr(), env_ptrs.as_ptr());
            // If execve returns, an error occurred, so exit the child process.
            libc::_exit(1);
        }
    } else {
        // Parent process: This process manages the child CGI script.

        unsafe {
            // Close the unused ends of the pipes in the parent process
            libc::close(pipe_stdin[0]);
            libc::close(pipe_stdout[1]);

            // Create Rust File objects from the raw file descriptors for easier I/O
            let mut stdin_writer = std::fs::File::from_raw_fd(pipe_stdin[1]);
            let mut stdout_reader = std::fs::File::from_raw_fd(pipe_stdout[0]);

            // Write the request body to the CGI script's stdin
            // Error handling for write operation
            if let Err(e) = stdin_writer.write_all(&request.body) {
                // Log error, but don't fail the entire request yet
                eprintln!("CGI Parent: Error writing to stdin pipe: {e}");
            }
            drop(stdin_writer); // Close stdin pipe for CGI, signaling EOF to the child

            // Wait for the child process to exit and get its status
            let timeout = Duration::from_millis(config.cgi_timeout);
            match wait_for_pid(pid, timeout) {
                Ok(true) => {
                    // Child process exited in time
                    let mut status = 0;
                    libc::waitpid(pid, &mut status, 0);
                }
                Ok(false) => {
                    // Child process timed out
                    libc::kill(pid, libc::SIGKILL);
                    libc::waitpid(pid, &mut 0, 0); // Clean up the zombie process
                    return Err(Response::new(
                        504,
                        b"CGI Error: Script timed out".to_vec(),
                        config.connection_type.clone(),
                    ));
                }
                Err(e) => {
                    // Error waiting for child process
                    return Err(Response::new(
                        500,
                        format!("CGI Error: Failed to wait for child process: {e}").into_bytes(),
                        config.connection_type.clone(),
                    ));
                }
            }

            // Read the CGI script's stdout
            let mut cgi_output = Vec::new();
            // Error handling for read operation
            if let Err(e) = stdout_reader.read_to_end(&mut cgi_output) {
                // Log error, but don't fail the entire request yet
                eprintln!("CGI Parent: Error reading from stdout pipe: {e}");
            }
            drop(stdout_reader); // Close stdout pipe for CGI

            // Parse the CGI script's output into an HTTP response
            let cgi_output_str = String::from_utf8_lossy(&cgi_output);

            let all_lines: Vec<&str> = cgi_output_str.lines().collect();

            let mut headers = HashMap::new();
            let mut body_start = 0;
            // Parse headers from CGI output (lines before the first empty line)
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

            // Extract the body, which is everything after the headers and the empty line
            let body = all_lines
                .into_iter()
                .skip(body_start)
                .collect::<Vec<&str>>()
                .join("\n")
                .into_bytes();

            // Create a new HTTP response from the CGI output
            let mut status_code = 200;
            if let Some(status_header) = headers.remove("Status") {
                if let Some(code_str) = status_header.split_whitespace().next() {
                    if let Ok(code) = code_str.parse::<u16>() {
                        status_code = code;
                    }
                }
            }

            let mut response = Response::new(status_code, body, config.connection_type.clone());
            for (key, value) in headers {
                response.headers.insert(key, value);
            }
            Ok(response)
        }
    }
}
