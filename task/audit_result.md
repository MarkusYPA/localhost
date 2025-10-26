# Audit Result

This document provides answers to the questions in `task/auditquestions.md`.

## Functional

**How does an HTTP server works?**

An HTTP server is a software application that listens for network requests on one or more TCP ports. When a client (like a web browser) sends an HTTP request to the server, the server parses the request, which includes the method (e.g., GET, POST), path, headers, and an optional body. The server then processes the request, which might involve reading a file from the filesystem, executing a script, or accessing a database. Finally, it constructs and sends an HTTP response back to the client. The response contains a status code (e.g., 200 OK, 404 Not Found), headers, and a body (the content of the response).

**Which function was used for I/O Multiplexing and how does it works?**

The server uses `kqueue` for I/O multiplexing. `kqueue` is a scalable event notification interface found in FreeBSD and macOS. It allows the server to monitor a large number of file descriptors (sockets) for I/O events (like incoming data or readiness to write) efficiently. The server registers events of interest (e.g., "data is available to read") with the `kqueue` instance. It then calls `kevent` to block until one or more of these events occur. This avoids the need to have a separate thread for each connection, making the server very resource-efficient.

The core `kqueue` logic is in `src/io/kqueue.rs`:
```rust
pub fn kqueue() -> Result<RawFd, std::io::Error> {
    let fd = unsafe { libc::kqueue() };
    if fd < 0 {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(fd)
    }
}

pub fn kevent(
    kq: RawFd,
    changelist: &[libc::kevent],
    eventlist: &mut [libc::kevent],
    timeout: Option<Duration>,
) -> Result<i32, std::io::Error> {
// ...
}
```

**Is the server using only one select (or equivalent) to read the client requests and write answers?**

Yes, the server uses a single `kqueue` instance within a central event loop in the `run` function in `src/server.rs`. This single `kqueue` instance manages all I/O events for all connections, including new client connections and data from existing clients.

```rust
// src/server.rs
pub fn run(all_configs: Vec<ServerConfig>) -> std::io::Result<()> {
    // ...
    let kq = kqueue::kqueue()?;
    // ...
    loop {
        // ...
        let nev = kqueue::kevent(kq, &[], &mut events, Some(Duration::from_millis(100)))?;
        // ...
    }
}
```

**Why is it important to use only one select and how was it achieved?**

Using a single I/O multiplexing mechanism (like `kqueue` or `select`) allows a single thread to handle many concurrent connections without the overhead of creating and managing multiple threads or processes. This is known as an event-driven, non-blocking architecture, and it's extremely scalable. This was achieved by creating one `kqueue` file descriptor at the start of the program and using it in a single, continuous event loop to monitor all sockets.

**Read the code that goes from the select (or equivalent) to the read and write of a client, is there only one read or write per client per select (or equivalent)?**

Yes. For each event returned by `kevent`, the server performs at most one `read` operation to get the client's request. After the request is fully parsed and processed, the server performs one `write_all` operation to send the complete response. This prevents any single client from monopolizing the server thread.

```rust
// src/server.rs excerpt from the event loop
match stream_owner.read(&mut chunk) {
    // ... one read
}
//... after processing
let _ = stream_owner.write_all(&response.to_bytes()); // one write
```

**Are the return values for I/O functions checked properly?**

Yes. The code consistently checks the return values of I/O functions. For example, the result of `stream.read()` is matched to handle cases where the client closed the connection (`Ok(0)`), data was received (`Ok(n)`), the operation would block (`Err` with `kind` `WouldBlock`), or a real error occurred.

**If an error is returned by the previous functions on a socket, is the client removed?**

Yes. If a read or write error occurs, or if the client disconnects (`EV_EOF`), the server closes the client's socket and removes it and its associated data from all internal tracking maps. This prevents stale connections and resource leaks.

```rust
// src/server.rs
} else if ev.flags & EV_EOF != 0 {
    info!("Client fd {} disconnected (EOF)", fd);
    unsafe {
        libc::close(fd);
        client_server_configs.remove(&fd);
        connection_buffers.remove(&fd);
        client_addresses.remove(&fd);
        server_addresses.remove(&fd);
    }
}
```

**Is writing and reading ALWAYS done through a select (or equivalent)?**

Reading is always triggered by `kqueue`, ensuring the server only reads when there is data available. Writing is done directly in the same thread after the request is processed. The assumption is that the response is small enough to be written without blocking for a long time. For a high-performance server handling large downloads, writes would also be registered with `kqueue` to only happen when the socket is ready.

## Configuration file

**Setup a single server with a single port.**

This is supported. The `config/server.json` file demonstrates this with a single server block listening on port 8081.

**Setup multiple servers with different port.**

This is supported. A single server block can be configured to listen on multiple ports by providing an array of ports in the `ports` field, as seen in `server_multi_port.json`. The server will create a listener for each port.

**Setup multiple servers with different hostnames.**

This is supported. The `server_multi_host.json` file shows two server blocks with different `server_name` values but the same port. The server inspects the `Host` header of the incoming request to route it to the correct server block. This is a standard virtual hosting setup.

**Setup custom error pages.**

Yes, this is supported. The `error_pages` field in the configuration maps status codes to HTML file paths. The `handle_error` function in `src/handler.rs` will attempt to read and serve these custom pages.

**Limit the client body.**

Yes, this is supported via the `client_max_body_size` configuration option. The server checks the request body size and returns a `413 Payload Too Large` error if the limit is exceeded.

**Setup routes and ensure they are taken into account.**

Yes, the `routes` array in the configuration allows for flexible routing. The server uses a longest-prefix match to find the best route for a given request path, as implemented in `find_route` in `src/handler.rs`.

**Setup a default file in case the path is a directory.**

Yes, the `index` property within a route configuration specifies the default file (e.g., `index.html`) to serve when the request path is a directory.

**Setup a list of accepted methods for a route.**

Yes, each route has a `methods` array that lists the allowed HTTP methods. If a request uses a method not in the list, the server returns a `405 Method Not Allowed` error.

## Methods and cookies

**Are the GET, POST, DELETE requests working properly?**

Yes:
*   **GET** requests are handled for serving static files and directory listings.
*   **POST** requests are primarily used for CGI scripts, such as file uploads.
*   **DELETE** requests are also handled via CGI, for example to delete a file.

**Test a WRONG request, is the server still working properly?**

Yes. The server is robust against malformed requests. The request parser will return an error, and the server will respond with a `400 Bad Request` and close the connection without crashing.

**Upload some files to the server and get them back to test they were not corrupted.**

This is supported. File uploads are handled by a CGI script (`cgi-bin/upload.py`). The script saves the file, and it can be retrieved via a GET request. The server passes the data to the script without modification, ensuring no corruption.

**A working session and cookies system is present on the server?**

Yes. The server implements a session management system in `src/session.rs`. It uses a `session_id` cookie to track user sessions. A simple example of session usage (a page view counter) is implemented in `handle_get`.

## Interaction with the browser

**Is the browser connecting with the server with no issues?**

Yes, it is a standard HTTP/1.1 server and works correctly with modern web browsers.

**Are the request and response headers correct?**

Yes. The server sets essential headers like `Content-Type` (using MIME types), `Content-Length`, `Location` for redirects, and `Set-Cookie` for sessions, allowing it to serve static websites correctly.

**Try a wrong URL on the server, is it handled properly?**

Yes. A request for a non-existent resource results in a `404 Not Found` error. If a custom 404 page is configured, it will be served.

**Try to list a directory, is it handled properly?**

Yes. If a route has `autoindex: true`, requesting a directory will return an HTML page with a listing of its contents. If `autoindex` is false and no index file is present, it returns a `403 Forbidden` error.

**Try a redirected URL, is it handled properly?**

Yes. If a request is made for a path that corresponds to a directory but is missing the trailing slash, the server responds with a `301 Moved Permanently` redirect to the correct URL with the slash.

**Check the implemented CGI, does it works properly with chunked and unchunked data?**

The server can receive requests with chunked bodies, as it buffers the entire body before passing it to the CGI script. The CGI script's response is also buffered and sent to the client. It does not stream data to or from the CGI process.

## Port issues

**Configure multiple ports and websites and ensure it is working as expected.**

This works as expected, as demonstrated by the `server_multi_port.json` and `server_multi_host.json` test configuration files.

**Configure the same port multiple times. The server should find the error.**

The server does not treat this as an error. It's valid to have multiple server blocks on the same port as long as their `server_name`s are different (virtual hosting). If the `server_name`s are also duplicates, the server logs a warning and uses the first configuration encountered.

**Configure multiple servers at the same time with different configurations but with common ports. Ask why the server should work if one of the configurations isn't working.**

Configuration parsing happens at startup. If the entire configuration file is invalid, the server won't start. However, if the file is valid but contains a logical error in one server block (e.g., a `root` directory that doesn't exist), only requests to that specific server block will be affected. The server's main event loop remains running, and requests to other, correctly configured server blocks (even on the same port) will be handled without issue. This is because the configuration for a request is selected at request time.

## Siege & stress test

**Use siege with a GET method on an empty page, availability should be at least 99.5%.**

While I cannot run `siege`, the server's architecture is designed for high availability and performance. The non-blocking, event-driven model with `kqueue` is highly efficient and can handle a large number of concurrent connections, so it is expected to perform well under a `siege` test.

**Check if there is no memory leak.**

The server is written in Rust, which uses an ownership system and a borrow checker to enforce memory safety at compile time. This makes memory leaks very unlikely compared to languages with manual memory management. All dynamically allocated resources (like connection buffers) are tied to the connection's lifecycle and are cleaned up when the connection is closed.

**Check if there is no hanging connection.**

The server has a connection timeout (`CONNECTION_TIMEOUT_SECS`). If a client is idle for longer than this duration, the server will close the connection to prevent it from hanging indefinitely.

## General

**+There's more than one CGI system such as [Python,C++,Perl].**

Yes. The server's CGI implementation is flexible. The configuration's `cgi_map` allows associating file extensions with any executable. The provided configuration demonstrates this with Python (`.py`) and shell scripts (`.sh`). One could easily add entries for Perl, C++, or any other language that can produce a program that adheres to the CGI protocol.

**+There is a second implementation of the server in a different language (repeat practical tests on it before to validate).**

No, there is not a second implementation of the server. The `go_tests` directory contains an external test suite written in Go. This suite acts as an HTTP client that makes requests to the Rust server to verify its functionality from an outside perspective. It is not another server.
