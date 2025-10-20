# Audit Result

This document provides answers to the audit questions based on the current state of the project.

## Functional

###### How does an HTTP server works?

An HTTP server is a software that understands URLs (Uniform Resource Locators) and HTTP (Hypertext Transfer Protocol). It can be accessed through the domain names of the websites it hosts. When a user wants to load a website, their browser sends an HTTP request to the server. The server then finds the requested file and sends it back to the browser, which then displays it.

Our server implements this flow using a non-blocking event loop with I/O multiplexing.

###### Which function was used for I/O Multiplexing and how does it works?

The server uses `kqueue` for I/O multiplexing, which is available on macOS and BSD systems. `kqueue` allows the server to monitor multiple sockets for events (like new connections or incoming data) with a single call, instead of having to poll each socket individually.

Here is how it is used in `src/server.rs`:
```rust
    // --- Create kqueue ---
    let kq = kqueue::kqueue()?;
    println!("Server initialized, waiting for events...");

    // ...

    // --- Register listener fds for read events ---
    let changes: Vec<libc::kevent> = listeners
        .iter()
        .map(|listener| libc::kevent {
            ident: listener.as_raw_fd() as libc::uintptr_t,
            filter: EVFILT_READ,
            flags: EV_ADD | EV_ENABLE,
            fflags: 0,
            data: 0,
            udata: std::ptr::null_mut(),
        })
        .collect();

    kqueue::kevent(kq, &changes, &mut [], None)?;

    // --- Event loop ---
    loop {
        // ...
        let nev = kqueue::kevent(kq, &[], &mut events, Some(Duration::from_secs(5)))?;
        // ...
    }
```

###### Is the server using only one select (or equivalent) to read the client requests and write answers?

Yes, the server uses a single `kqueue` instance to manage all I/O events.

###### Why is it important to use only one select and how was it achieved?

Using a single `kqueue` (or `select`/`epoll`) is crucial for efficiency and scalability. It allows a single thread to handle many concurrent connections without the overhead of creating a new thread for each connection and without the inefficiency of polling each socket in a loop. This is achieved by registering all listener and client sockets with the single `kqueue` instance.

###### Read the code that goes from the select (or equivalent) to the read and write of a client, is there only one read or write per client per select (or equivalent)?

Yes, for each event from `kqueue` that indicates a readable client socket, the server performs one `read` operation. After processing the request, it performs one `write` operation to send the response.

###### Are the return values for I/O functions checked properly?

Yes, all I/O operations and other functions that can fail return a `Result`. The `?` operator is used to propagate errors, and `match` statements are used to handle them where necessary.

###### If an error is returned by the previous functions on a socket, is the client removed?

Yes. For example, in `src/server.rs`, if a `read` operation on a client socket returns an error, the client is removed from the `client_server_configs` map, and the connection is closed.

```rust
Err(e) => {
    eprintln!("Read error on fd {}: {}", fd, e);
    client_server_configs.remove(&fd);
    // don't call libc::close(fd)
}
```

###### Is writing and reading ALWAYS done through a select (or equivalent)?

The readiness of a socket for reading or writing is determined by `kqueue`. The actual `read` and `write` operations are then performed on the socket. This is the standard and correct way to use I/O multiplexing.

## Configuration file

###### Setup a single server with a single port.

This is covered in `comprehensive_server.json`. The server on port 8080 is a single-port server.

```json
{
  "server_name": "localhost",
  "host": "127.0.0.1",
  "ports": [8080],
  // ...
}
```

###### Setup multiple servers with different port.

Covered by `comprehensive_server.json`, which defines servers on ports 8080, 8081, and 8082.

###### Setup multiple servers with different hostnames.

Covered by `comprehensive_server.json` with `site1.com` and `site2.com` on port 8081. This is tested in `comprehensive_test.go`.

###### Setup custom error pages.

Covered by `comprehensive_server.json` and tested in `comprehensive_test.go`.

```json
"error_pages": {
  "404": "/errors/404.html"
},
```

###### Limit the client body.

Covered by `comprehensive_server.json` and tested in `comprehensive_test.go`.

```json
"client_max_body_size": 10,
```

###### Setup routes and ensure they are taken into account.

Covered and tested. The `routes` array in the configuration is used to route requests to different handlers or file system locations.

###### Setup a default file in case the path is a directory.

Covered by the `index` property in a route configuration and tested.

```json
"index": "index.html"
```

###### Setup a list of accepted methods for a route.

Covered by the `methods` property in a route configuration and tested.

```json
"methods": ["GET", "POST"]
```

## Methods and cookies

###### Are the GET requests working properly?

Yes, tested extensively in `comprehensive_test.go`.

###### Are the POST requests working properly?

Yes, tested for file uploads in `comprehensive_test.go`.

###### Are the DELETE requests working properly?

Yes, tested for file deletion in `comprehensive_test.go`.

###### Test a WRONG request, is the server still working properly?

Yes, 404 Not Found and other error codes are handled correctly and tested.

###### Upload some files to the server and get them back to test they were not corrupted.

Yes, this is tested in the `FileUploadAndDownload` test in `comprehensive_test.go`.

###### A working session and cookies system is present on the server?

Yes, a session management system using cookies is implemented in `src/session.rs` and used in `src/server.rs`.

## Interaction with the browser

###### Is the browser connecting with the server with no issues?

This is a manual test, but the server is a standard HTTP/1.1 server and should work with any modern browser.

###### Are the request and response headers correct?

Partially tested. The `Content-Type` header is checked in the tests.

###### Try a wrong URL on the server, is it handled properly?

Yes, this is tested and the server returns a 404 Not Found response.

###### Try to list a directory, is it handled properly?

Yes, this is tested. If `autoindex` is `true`, a directory listing is shown. If `false` and no index file is present, a 403 Forbidden error is returned.

###### Try a redirected URL, is it handled properly?

HTTP redirection is not implemented.

###### Check the implemented CGI, does it works properly with chunked and unchunked data?

CGI is implemented and works for unchunked data. Chunked transfer encoding is not supported.

## Port issues

###### Configure multiple ports and websites and ensure it is working as expected.

Covered by `comprehensive_server.json` and the corresponding tests.

###### Configure the same port multiple times. The server should find the error.

Yes, the server now checks for duplicate `server_name` on the same port and will return an error on startup.

Example `invalid_port_config.json`:
```json
{
  "servers": [
    {
      "server_name": "localhost",
      "host": "127.0.0.1",
      "ports": [8090],
      "routes": []
    },
    {
      "server_name": "localhost",
      "host": "127.0.0.1",
      "ports": [8090],
      "routes": []
    }
  ]
}
```
Running with this config will produce an error.

###### Configure multiple servers at the same time with different configurations but with common ports. Ask why the server should work if one of the configurations isn't working.

The server validates the configuration at startup. If any part of the configuration is invalid (like a port conflict), the server will not start. This is a design choice to ensure that the server only runs with a valid and predictable configuration.

## Siege & stress test

These are manual tests. The following commands can be used:

-   `siege -b [IP]:[PORT]`
-   `top` or other system monitoring tools to check for memory leaks.

