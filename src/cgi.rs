use crate::config::Route;
use crate::http::request::Request;
use crate::http::response::Response;
use std::path::Path;
use std::collections::HashMap;
use std::ffi::CString;
use std::io::{Read, Write};
use std::os::fd::FromRawFd;

pub fn handle_cgi(
    request: &Request,
    _route: &Route,
    cgi_path: &Path,
    cgi_executor: &str,
) -> Response {
    let mut pipe_stdin = [0; 2];
    let mut pipe_stdout = [0; 2];

    if unsafe { libc::pipe(pipe_stdin.as_mut_ptr()) } < 0 {
        return Response::new(500, b"CGI Error: Failed to create stdin pipe".to_vec());
    }
    if unsafe { libc::pipe(pipe_stdout.as_mut_ptr()) } < 0 {
        unsafe { libc::close(pipe_stdin[0]); libc::close(pipe_stdin[1]); }
        return Response::new(500, b"CGI Error: Failed to create stdout pipe".to_vec());
    }

    let pid = unsafe { libc::fork() };

    if pid < 0 {
        unsafe {
            libc::close(pipe_stdin[0]); libc::close(pipe_stdin[1]);
            libc::close(pipe_stdout[0]); libc::close(pipe_stdout[1]);
        }
        return Response::new(500, b"CGI Error: Failed to fork process".to_vec());
    } else if pid == 0 { // Child process
        println!("CGI Child process started, PID: {}", unsafe { libc::getpid() });
        unsafe {
            libc::close(pipe_stdin[1]); // Close write end of stdin pipe
            libc::dup2(pipe_stdin[0], libc::STDIN_FILENO);
            libc::close(pipe_stdin[0]);

            libc::close(pipe_stdout[0]); // Close read end of stdout pipe
            libc::dup2(pipe_stdout[1], libc::STDOUT_FILENO);
            libc::close(pipe_stdout[1]);

            let cgi_path_c = CString::new(cgi_path.to_str().unwrap()).unwrap();
            let _cgi_executor_c = CString::new(cgi_executor).unwrap();

            let args = [
                cgi_path_c.as_ptr(),
                std::ptr::null()
            ];

            let path_info = CString::new(request.path.as_str()).unwrap();
            let request_method = CString::new(request.method.as_str()).unwrap();
            let query_string = CString::new(request.query_params.iter().map(|(k,v)| format!("{k}={v}")).collect::<Vec<String>>().join("&")).unwrap();
            let content_type = CString::new(request.headers.get("Content-Type").unwrap_or(&"".to_string()).as_str()).unwrap();
            let content_length = CString::new(request.body.len().to_string()).unwrap();

            let mut env_vars = Vec::new();
            env_vars.push(CString::new(format!("PATH_INFO={}", path_info.to_str().unwrap())).unwrap());
            env_vars.push(CString::new(format!("REQUEST_METHOD={}", request_method.to_str().unwrap())).unwrap());
            env_vars.push(CString::new(format!("QUERY_STRING={}", query_string.to_str().unwrap())).unwrap());
            env_vars.push(CString::new(format!("CONTENT_TYPE={}", content_type.to_str().unwrap())).unwrap());
            env_vars.push(CString::new(format!("CONTENT_LENGTH={}", content_length.to_str().unwrap())).unwrap());

            let mut env_ptrs: Vec<*const libc::c_char> = env_vars.iter().map(|c_str| c_str.as_ptr()).collect();
            env_ptrs.push(std::ptr::null());

            libc::execve(cgi_path_c.as_ptr(), args.as_ptr(), env_ptrs.as_ptr());
            // If execve returns, an error occurred
            libc::_exit(1);
        }
    } else { // Parent process
        println!("CGI Parent process, child PID: {}", pid);
        unsafe {
            libc::close(pipe_stdin[0]); // Close read end of stdin pipe
            libc::close(pipe_stdout[1]); // Close write end of stdout pipe

            let mut stdin_writer = std::fs::File::from_raw_fd(pipe_stdin[1]);
            let mut stdout_reader = std::fs::File::from_raw_fd(pipe_stdout[0]);

            // Write request body to CGI stdin
            if let Err(e) = stdin_writer.write_all(&request.body) {
                eprintln!("CGI Parent: Error writing to stdin pipe: {}", e);
            }
            drop(stdin_writer); // Close stdin pipe for CGI

            // Read CGI stdout
            let mut cgi_output = Vec::new();
            if let Err(e) = stdout_reader.read_to_end(&mut cgi_output) {
                eprintln!("CGI Parent: Error reading from stdout pipe: {}", e);
            }
            drop(stdout_reader); // Close stdout pipe for CGI

            let mut status = 0;
            libc::waitpid(pid, &mut status, 0);
            println!("CGI Parent: Child process exited with status: {}", status);

            // Parse CGI output into HTTP response
            let cgi_output_str = String::from_utf8_lossy(&cgi_output);
            println!("CGI Parent: Raw CGI output:\n{}", cgi_output_str);
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

            let body = all_lines.into_iter().skip(body_start).collect::<Vec<&str>>().join("\n").into_bytes();
            println!("CGI Parent: Parsed Headers: {:?}", headers);
            println!("CGI Parent: Parsed Body: {:?}", String::from_utf8_lossy(&body));

            let mut response = Response::new(200, body);
            for (key, value) in headers {
                response.headers.insert(key, value);
            }
            response
        }
    }
}