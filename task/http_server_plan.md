# HTTP Server Implementation Plan (Rust)

## Project Overview
Build a production-ready HTTP/1.1 server in Rust with single-threaded, non-blocking I/O using epoll, supporting static file serving, CGI execution, and extensive configuration options.

---

## Phase 1: Foundation & Architecture (Days 1-3)

### 1.1 Research & Setup
- **Study HTTP/1.1 RFC 9112** - understand request/response format, methods, headers, chunked encoding
- **Review epoll/kqueue APIs** - understand edge-triggered vs level-triggered modes
- **Setup project structure**:
  ```
  src/
  ├── main.rs
  ├── server.rs          # Main server loop
  ├── config.rs          # Configuration parser
  ├── http/
  │   ├── mod.rs
  │   ├── request.rs     # HTTP request parser
  │   ├── response.rs    # HTTP response builder
  │   └── status.rs      # Status codes
  ├── io/
  │   ├── mod.rs
  │   ├── epoll.rs       # Epoll wrapper
  │   └── buffer.rs      # Non-blocking buffer management
  ├── handler.rs         # Request routing & handling
  ├── cgi.rs             # CGI execution
  └── session.rs         # Cookie & session management
  ```

### 1.2 Core Data Structures
- **ServerConfig**: host, ports, routes, error pages, body size limit
- **Route**: path, methods, root directory, default file, CGI mapping, directory listing
- **Connection**: socket fd, state machine (reading request/writing response), buffers
- **Request**: method, path, headers, body, query params
- **Response**: status, headers, body

---

## Phase 2: Configuration Parser (Days 4-5)

### 2.1 Design Configuration Format
Choose simple format (JSON/TOML-like or custom):
```
server {
    host: 127.0.0.1
    port: 8080
    server_name: localhost
    error_page_404: /errors/404.html
    client_max_body_size: 1048576
    
    route / {
        methods: GET POST
        root: /var/www
        index: index.html
        autoindex: on
    }
    
    route /upload {
        methods: POST
        root: /var/uploads
    }
    
    route /cgi-bin {
        methods: GET POST
        root: /var/cgi
        cgi_extension: .py python3
    }
}
```

### 2.2 Implementation Tasks
- Parse configuration file into ServerConfig structs
- Validate: no duplicate ports, valid paths, proper route definitions
- Support multiple servers on different ports
- Handle default server selection (first server for host:port)
- Create default error pages if not specified

---

## Phase 3: Event Loop & I/O Multiplexing (Days 6-9)

### 3.1 Epoll Wrapper
- Create safe Rust wrapper around `libc::epoll_create1`, `epoll_ctl`, `epoll_wait`
- Support both server listening sockets and client connection sockets
- Use **edge-triggered mode** for efficiency
- Single epoll instance for all I/O operations

### 3.2 Connection State Machine
States: `ReadingRequest` → `ProcessingRequest` → `WritingResponse` → `KeepAlive/Close`

```rust
enum ConnectionState {
    ReadingRequestLine,
    ReadingHeaders,
    ReadingBody { expected: usize, received: usize },
    Processing,
    WritingResponse { sent: usize },
    KeepAlive,
}
```

### 3.3 Main Event Loop
```
1. epoll_wait() with timeout (for request timeouts)
2. For each event:
   - If listening socket: accept() new connection, add to epoll
   - If client socket readable: read_nonblocking(), parse, advance state
   - If client socket writable: write_nonblocking(), advance state
3. Check timeouts, close stale connections
4. Loop forever (never crash)
```

### 3.4 Non-Blocking I/O
- All `read()`/`write()` calls handle `EAGAIN`/`EWOULDBLOCK`
- Buffer partial reads/writes
- Never block the event loop

---

## Phase 4: HTTP Request Parser (Days 10-12)

### 4.1 Request Line Parsing
- Parse `METHOD PATH HTTP/1.1\r\n`
- Support GET, POST, DELETE methods
- Extract path and query string

### 4.2 Header Parsing
- Parse `Header: Value\r\n` lines until `\r\n\r\n`
- Store in HashMap<String, String>
- Handle multi-line headers
- Extract critical headers: Content-Length, Transfer-Encoding, Host, Cookie

### 4.3 Body Handling
- **Unchunked**: Read `Content-Length` bytes
- **Chunked**: Parse chunk size, read chunk, repeat until `0\r\n\r\n`
- Enforce `client_max_body_size` limit (return 413 if exceeded)
- Handle incomplete reads (store state for next epoll event)

### 4.4 Request Validation
- Return 400 for malformed requests
- Return 405 for unsupported methods
- Return 413 for body too large
- Return 505 for unsupported HTTP version

---

## Phase 5: HTTP Response Builder (Days 13-14)

### 5.1 Status Line & Headers
```
HTTP/1.1 200 OK\r\n
Content-Type: text/html\r\n
Content-Length: 1234\r\n
Connection: keep-alive\r\n
\r\n
```

### 5.2 Response Types
- **Static files**: Read file, set Content-Type based on extension
- **Directory listing**: Generate HTML with file list (if autoindex on)
- **Redirects**: 301/302 with Location header
- **Error pages**: Load custom error page or generate default

### 5.3 Keep-Alive
- Support `Connection: keep-alive` for HTTP/1.1
- Close connection if `Connection: close` or HTTP/1.0

---

## Phase 6: Request Handler & Router (Days 15-17)

### 6.1 Route Matching
- Find matching route by longest prefix match
- Apply route-specific settings (methods, root, index)
- Handle default server selection based on Host header

### 6.2 Method Handlers

**GET**:
- Resolve path to filesystem (handle `..` attacks)
- If directory: serve index file or directory listing
- If file: send file with proper Content-Type
- Return 404 if not found

**POST**:
- Parse body (form data or file upload)
- Save uploaded files to configured directory
- Return 201 or redirect

**DELETE**:
- Check if method allowed for route
- Delete file if exists and permitted
- Return 204 or 403

### 6.3 Security
- Prevent directory traversal (`../../../etc/passwd`)
- Validate file paths stay within route root
- Return 403 for forbidden paths

---

## Phase 7: CGI Implementation (Days 18-20)

### 7.1 CGI Selection
- Match file extension (`.py`, `.php`, etc.) to CGI executable
- Pass file path as first argument
- Set environment variables: `PATH_INFO`, `REQUEST_METHOD`, `QUERY_STRING`, `CONTENT_TYPE`, `CONTENT_LENGTH`

### 7.2 CGI Execution
- Fork child process with `libc::fork()`
- Setup pipes for stdin/stdout
- Execute CGI script: `execve(cgi_path, [file_path], env)`
- Parent: write request body to stdin pipe, read output from stdout pipe
- Parse CGI output for headers and body
- Timeout CGI execution (kill child if too slow)

### 7.3 Chunked CGI Output
- Support CGI scripts that output chunks
- Parse `Transfer-Encoding: chunked` from CGI headers

---

## Phase 8: Sessions & Cookies (Days 21-22)

### 8.1 Cookie Parsing
- Extract `Cookie` header from request
- Parse `name=value; name2=value2` format

### 8.2 Session Management
- Generate unique session IDs (UUID or random hex)
- Store sessions in HashMap<SessionId, SessionData>
- Set `Set-Cookie: session_id=xxx; HttpOnly; Path=/` in response
- Expire old sessions (configurable timeout)

### 8.3 Session Data
- Store key-value pairs per session
- Example: login status, user preferences, cart items

---

## Phase 9: Error Handling & Robustness (Days 23-24)

### 9.1 Error Pages
- Create default HTML pages for 400, 403, 404, 405, 413, 500
- Load custom error pages from config
- Never panic - always return proper error response

### 9.2 Timeouts
- Track connection start time
- Close connections exceeding timeout (30-60 seconds)
- Check timeouts on each event loop iteration

### 9.3 Resource Limits
- Limit max concurrent connections
- Close oldest idle connections if limit reached
- Monitor memory usage

### 9.4 Error Recovery
- Wrap all operations in Result<T, Error>
- Log errors but never crash
- Close problematic connections gracefully

---

## Phase 10: Testing & Validation (Days 25-28)

### 10.1 Unit Tests
- HTTP parser: valid/invalid requests, chunked encoding
- Configuration parser: valid/invalid configs
- Route matching: edge cases
- Security: path traversal attempts

### 10.2 Integration Tests
- Start server, send requests via `reqwest` crate
- Test all methods: GET, POST, DELETE
- Test file uploads and downloads
- Test CGI execution
- Test cookies and sessions
- Test error codes

### 10.3 Browser Testing
- Serve a complete static website (HTML, CSS, JS, images)
- Test in Chrome/Firefox with DevTools
- Verify headers, status codes, redirects
- Test directory listing

### 10.4 Stress Testing
```bash
# Availability test
siege -b http://127.0.0.1:8080 -t 30s

# Expected: Availability >= 99.5%
# Monitor with: top, valgrind --leak-check=full
```

### 10.5 Memory Leak Testing
```bash
valgrind --leak-check=full --show-leak-kinds=all ./target/release/http_server
# Run requests and verify no leaks
```

---

## Phase 11: Optimization & Polish (Days 29-30)

### 11.1 Performance
- Use `Vec<u8>` for buffers (avoid string allocations)
- Reuse connection objects (object pool)
- Minimize allocations in hot paths
- Profile with `perf` or `flamegraph`

### 11.2 Documentation
- Add comments to complex logic
- Write README with usage examples
- Document configuration format

### 11.3 Code Quality
- Run `clippy` for lints
- Format with `rustfmt`
- Remove debug prints
- Handle all `unwrap()` calls

---

## Key Implementation Tips

### Event Loop Pattern
```rust
let epoll_fd = epoll_create1(0);
let mut connections: HashMap<RawFd, Connection> = HashMap::new();

loop {
    let events = epoll_wait(epoll_fd, timeout);
    
    for event in events {
        let fd = event.fd;
        
        if is_listening_socket(fd) {
            let client_fd = accept(fd);
            set_nonblocking(client_fd);
            epoll_add(epoll_fd, client_fd, EPOLLIN | EPOLLET);
            connections.insert(client_fd, Connection::new());
        } else {
            let conn = connections.get_mut(&fd).unwrap();
            
            if event.readable() {
                match conn.read_nonblocking() {
                    Ok(Progress::NeedMore) => continue,
                    Ok(Progress::RequestComplete) => {
                        conn.process_request();
                        epoll_mod(epoll_fd, fd, EPOLLOUT | EPOLLET);
                    }
                    Err(e) => {
                        close_connection(fd);
                        connections.remove(&fd);
                    }
                }
            }
            
            if event.writable() {
                match conn.write_nonblocking() {
                    Ok(Progress::NeedMore) => continue,
                    Ok(Progress::ResponseComplete) => {
                        if conn.keep_alive {
                            epoll_mod(epoll_fd, fd, EPOLLIN | EPOLLET);
                            conn.reset();
                        } else {
                            close_connection(fd);
                            connections.remove(&fd);
                        }
                    }
                    Err(e) => {
                        close_connection(fd);
                        connections.remove(&fd);
                    }
                }
            }
        }
    }
    
    // Timeout check
    let now = Instant::now();
    connections.retain(|fd, conn| {
        if now - conn.last_activity > TIMEOUT {
            close(*fd);
            false
        } else {
            true
        }
    });
}
```

### Critical Requirements Checklist
- ✅ Single thread, single process
- ✅ One epoll instance, used for ALL I/O
- ✅ Non-blocking I/O everywhere
- ✅ Never crash (no panics, handle all errors)
- ✅ Request timeouts
- ✅ Multiple servers, multiple ports
- ✅ HTTP/1.1 compliant
- ✅ GET, POST, DELETE methods
- ✅ File uploads
- ✅ Cookies & sessions
- ✅ Custom error pages (400, 403, 404, 405, 413, 500)
- ✅ Chunked and unchunked requests
- ✅ CGI execution (at least one language)
- ✅ Configuration file parsing
- ✅ 99.5%+ availability under stress
- ✅ No memory leaks

---

## Bonus Features (Optional)

1. **Multiple CGI Support**: Python, PHP, Perl
2. **Second Implementation**: Rewrite in C/C++ for comparison
3. **Advanced Features**: HTTPS support, HTTP/2, WebSockets

---

## Estimated Timeline: 30 days
- Foundation: 3 days
- Config: 2 days
- Event Loop: 4 days
- HTTP Parser: 3 days
- Response Builder: 2 days
- Request Handler: 3 days
- CGI: 3 days
- Sessions: 2 days
- Error Handling: 2 days
- Testing: 4 days
- Optimization: 2 days

**Total: ~30 days of focused work**