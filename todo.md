# Project TODO List

## Core Server
- [x] Implement a single-threaded, non-blocking event loop using `kqueue`.
- [x] Handle multiple listeners on different ports.
- [ ] Implement robust error handling to prevent crashes.
- [x] Implement connection timeout for idle clients.
- [ ] Implement a limit for maximum concurrent connections.
- [ ] Handle graceful shutdown.

## HTTP
- [x] Parse HTTP/1.1 requests (method, path, headers, body).
- [x] Build HTTP/1.1 responses.
- [x] Implement `GET` method for static files.
- [ ] Implement `POST` method for file uploads.
- [ ] Implement `DELETE` method for deleting files.
- [ ] Handle chunked transfer encoding for requests.
- [ ] Handle chunked transfer encoding for responses.
- [x] Implement session management with cookies.
- [x] Provide default error pages for 400, 403, 404, 405, 413, 500.
- [ ] Implement HTTP redirections.

## CGI
- [ ] Implement CGI execution by forking a new process.
- [ ] Pass environment variables to the CGI script (e.g., `PATH_INFO`, `REQUEST_METHOD`).
- [ ] Handle CGI script timeout.
- [ ] Support at least one CGI language (e.g., Python).
- [ ] Bonus: Support more CGI languages.

## Configuration
- [x] Parse configuration from a JSON file.
- [x] Configure host and multiple ports.
- [x] Configure routes with methods, root, and index files.
- [x] Configure custom error pages.
- [x] Configure client body size limit.
- [ ] Configure directory listing on/off.
- [ ] Configure HTTP redirections.
- [ ] Handle default server selection based on `server_name`.

## Testing
- [ ] Create comprehensive unit tests for all modules.
- [ ] Create integration tests for all HTTP methods and features.
- [ ] Perform stress testing with `siege` to ensure >99.5% availability.
- [ ] Test for memory leaks using `valgrind` or AddressSanitizer.
- [ ] Test with a real browser to ensure compatibility.
