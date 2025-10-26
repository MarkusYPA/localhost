# HTTP Server

This project implements a single-threaded, non-blocking HTTP/1.1 server in Rust, designed to serve static web pages and execute CGI scripts. It supports a flexible configuration system for defining hosts, ports, routes, error pages, and more.

## Features

-   **HTTP/1.1 Compliance**: Manages GET, POST, and DELETE methods.
-   **Non-blocking I/O**: Utilizes `kqueue` (or `epoll` on Linux) for efficient event handling.
-   **CGI Support**: Executes external scripts (e.g., Python) for dynamic content generation.
-   **Configurable**: Supports custom hosts, multiple ports, routes, error pages, and client body size limits.
-   **Session Management**: Handles cookies and sessions.
-   **Robust Error Handling**: Provides custom error pages for various HTTP status codes.

## Project Structure

```
.
├── Cargo.toml
├── src/
│   ├── cgi.rs          # CGI execution logic
│   ├── config.rs       # Configuration parsing
│   ├── handler.rs      # Request routing and handling
│   ├── http/           # HTTP protocol related structures
│   │   ├── mod.rs
│   │   ├── request.rs  # HTTP request parser
│   │   ├── response.rs # HTTP response builder
│   │   └── status.rs   # HTTP status codes
│   ├── io/             # I/O multiplexing (kqueue/epoll) wrapper
│   │   ├── buffer.rs
│   │   ├── kqueue.rs
│   │   └── mod.rs
│   ├── main.rs         # Main entry point (binary crate)
│   ├── lib.rs          # Library crate for core server logic
│   ├── server.rs       # Main server event loop
│   └── session.rs      # Cookie and session management
├── config/             # Example configuration files
│   └── server.json
├── tests/              # Integration tests
│   └── integration_test.rs
├── www/                # Static web content
│   └── index.html
└── errors/             # Custom error pages
    ├── 400.html
    ├── 403.html
    ├── 404.html
    └── ...
```

## Configuration

The server's behavior is defined by one or more JSON configuration files. These files specify server blocks, hosts, ports, routes, error pages, and more.

Here's an example of a JSON configuration file:

```json
{
  "servers": [
    {
      "server_name": "localhost",
      "host": "127.0.0.1",
      "ports": [8080],
      "client_max_body_size": 1024,
      "routes": [
        {
          "path": "/",
          "methods": ["GET"],
          "root": "www",
          "index": "index.html"
        }
      ]
    }
  ]
}
```

### Configuration Directives

-   `servers`: An array of server block configurations.
-   `server_name`: The server name (e.g., `localhost`).
-   `host`: The IP address the server will bind to.
-   `ports`: An array of ports the server will listen on.
-   `error_pages`: A map of HTTP status codes to custom error page paths (e.g., `"404": "/errors/404.html"`).
-   `client_max_body_size`: Maximum allowed size for client request bodies in bytes.
-   `routes`: An array of route configurations.
    -   `path`: The URL path for the route.
    -   `methods`: An array of allowed HTTP methods (e.g., `["GET", "POST"]`).
    -   `root`: Filesystem path to serve content from for this route.
    -   `index`: Default file to serve if the URL is a directory (e.g., `index.html`).
    -   `autoindex`: `true` or `false` to enable/disable directory listing.
    -   `cgi_map`: A map associating file extensions with CGI executors (e.g., `{".py": "python3"}`).
    -   `cgi_timeout`: The maximum time in milliseconds for a CGI script to run before it is terminated. Defaults to 5000.

## Usage

### Building the Server

To build the server in debug mode:

```bash
cargo build
```

To build the server in release mode (optimized):

```bash
cargo build --release
```

### Running the Server

To run the server:

```bash
cargo run
```

By default, the server will attempt to load configuration files from a directory named `config` in the project root. You can specify a different configuration source using the `--config` flag:

-   **To load configurations from a directory (e.g., `test_configs/`):**

    ```bash
    cargo run -- --config test_configs/
    ```

    The server will attempt to load all `.json` files within the specified directory. Invalid configuration files will be logged as errors, but the server will continue to load valid ones.

-   **To load a single configuration file (e.g., `server.json`):**

    ```bash
    cargo run -- --config comprehensive_server.json
    ```

    The server will load only the specified `.json` file. If the file is invalid, the server will exit with an error.

## Testing

### Unit Tests

To run unit tests:

```bash
cargo test --lib
```

### Integration Tests

Integration tests are located in the `tests/` directory and verify the server's behavior end-to-end. To run integration tests:

```bash
cargo test --test integration_test
```

### Golang tests

In order to make sure the auditor can check tests behavior, some tests are written on Golang.

Start the server in one terminal with the comprehensive configuration:
```bash
cargo run --release -- --config comprehensive_server.json
```

Run tests in another terminal:
```bash
cd go_tests/
go test
```

### Incorrect cofiguration test

Test for configuration errors — one bad virtual host or configuration block shouldn't crash or disable the entire process

Start the server in the terminal with specifying test_configs folder:
```bash
cargo run --release -- --config test_configs/
```

The following output is expected, showing that failed configs do not crash the entire process:

```
Successfully loaded config: server.json
Error parsing config file invalid_config.json: invalid type: string "client_max_body_size", expected u16 at line 7 column 28
Successfully loaded config: duplicate_server.json
Warning: Duplicate server_name 'localhost' found for the same port. The first defined server will be used.
Server listening on 127.0.0.1:8090
Server listening on 127.0.0.1:8081
Server initialized, waiting for events...
```

### Testing chunked requests

Send chuncked POST request to /cgi-bin/echo.py using this command:
```bash
printf "POST /cgi-bin/echo.py HTTP/1.1\r\nHost: localhost:8081\r\nTransfer-Encoding: chunked\r\nContent-Type: text/plain\r\nConnection: close\r\n\r\n5\r\nHello\r\n7\r\n World\!\r\n0\r\n\r\n" | nc localhost 8081
```

Expected result - unchunked body "Hello world!": 

```
HTTP/1.1 200 OK
Connection: keep-alive
Content-Length: 12
Content-Type: text/plain
Set-Cookie: session_id=6a8f5157-17b4-4456-9740-cffc0d002d27; HttpOnly; Path=/

Hello World!
```


### Memory Leak Testing (macOS)

On macOS, `valgrind` is not well-supported. Instead, you can use AddressSanitizer (ASan), which is built into the Rust compiler.

1.  **Build and run with ASan:**
    ```bash
    RUSTFLAGS="-Zsanitizer=address" cargo +nightly run -Zbuild-std --target aarch64-apple-darwin
    ```
    (Replace `aarch64-apple-darwin` with your target triple if different, e.g., `x86_64-apple-darwin`)

2.  **Run tests against the server (in a separate terminal):**
    ```bash
    RUSTFLAGS="-Zsanitizer=address" cargo +nightly test -Zbuild-std --target aarch64-apple-darwin
    ```
    or stress test with `siege`:
    ```bash
    siege -b -t30s http://127.0.0.1:8081/
    ```

3.  **Check ASan output:** After stopping the server (Ctrl+C), ASan will print a detailed memory leak report to the server's terminal if any are detected.

### Stress Testing

To perform stress testing, you can use `siege`. First, ensure `siege` is installed (e.g., `brew install siege` on macOS).

Then, run the server in one terminal:

```bash
cargo run
```

In a separate terminal, run `siege`:

```bash
siege -b -t30s http://127.0.0.1:8081/
```

This will bombard your server with requests for 30 seconds. Monitor the server's stability and performance.
