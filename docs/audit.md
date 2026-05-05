# Audit Criteria

This document outlines the evaluation criteria for the Localhost project, based on the 01Edu specification.

## 1. Functional Analysis

- **HTTP Mechanics**: Can you justify architectural choices for the HTTP stack?
- **I/O Multiplexing**: How is `kqueue` (or equivalent) implemented?
- **Single Event Loop**: Does the server use only one select/poll/kqueue to manage all clients?
- **Non-blocking Ops**: Is every read and write operation non-blocking?
- **Error Handling**: Are sockets properly removed from the event loop on error?

## 2. Configuration Verification

Verify that the following directives are respected:
- Single and multiple port bindings.
- Hostname-based virtual hosting (`Host` header resolution).
- Custom error pages (e.g., 404, 403).
- `client_max_body_size` enforcement.
- Route-specific rules (accepted methods, root directories, index files).

## 3. Method & Protocol Testing

- **GET**: Proper serving of static files and directory indexing.
- **POST**: Correct handling of payloads and file uploads.
- **DELETE**: Proper resource removal and permission checks.
- **Cookies/Sessions**: Functional session tracking and `Set-Cookie` header validation.

## 4. CGI Verification

- Support for multiple CGI executors (Python, Bash, etc.).
- Correct handling of **Chunked** vs **Unchunked** input data.
- Environment variable propagation to the CGI child process.

## 5. Resilience & Stress

- **Siege**: Should maintain >99.5% availability under heavy load.
- **Memory Leaks**: Zero leaks detected via ASan.
- **Hanging Connections**: No stagnant file descriptors or orphaned processes.
- **Config Robustness**: One bad server block should not prevent the rest of the process from starting.
