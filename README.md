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
├── server.conf         # Example configuration file
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

The server's behavior is defined by the `server.conf` file. Here's an example of the configuration format:

```
server {
    host: 127.0.0.1
    port: 8081
    server_name: localhost
    error_page_404: /errors/404.html
    error_page_400: /errors/400.html
    error_page_403: /errors/403.html
    error_page_405: /errors/405.html
    error_page_413: /errors/413.html
    error_page_500: /errors/500.html
    client_max_body_size: 1048576
    
    route / {
        methods: GET POST
        root: ./www
        index: index.html
        autoindex: on
    }
    
    route /upload {
        methods: POST
        root: /var/uploads
    }
    
    route /cgi-bin {
        methods: GET POST
        root: ./cgi-bin
        cgi_extension: .py python3
    }

    route /site {
        methods: GET
        root: ./www/site
        index: index.html
        autoindex: on
    }
}
```

### Configuration Directives

-   `host`: The IP address the server will bind to.
-   `port`: One or more ports the server will listen on.
-   `server_name`: The server name (e.g., `localhost`).
-   `error_page_<STATUS_CODE>`: Path to custom error pages (e.g., `error_page_404: /errors/404.html`).
-   `client_max_body_size`: Maximum allowed size for client request bodies in bytes.
-   `route <PATH> { ... }`: Defines a route block with the following directives:
    -   `methods`: Space-separated list of allowed HTTP methods (e.g., `GET POST`).
    -   `root`: Filesystem path to serve content from for this route.
    -   `index`: Default file to serve if the URL is a directory (e.g., `index.html`).
    -   `autoindex`: `on` or `off` to enable/disable directory listing.
    -   `cgi_extension`: Associates a file extension with a CGI executor (e.g., `.py python3`).

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

The server will load its configuration from `server.conf` in the project root.

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
