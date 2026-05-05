# Functionality

This document details the features and configuration schema of the HTTP Server.

## Features

- **HTTP/1.1 Compliance**: Manages GET, POST, and DELETE methods.
- **Non-blocking I/O**: Utilizes `kqueue` (or `epoll` on Linux) for efficient event handling.
- **CGI Support**: Executes external scripts (e.g., Python) for dynamic content generation.
- **Configurable**: Supports custom hosts, multiple ports, routes, error pages, and client body size limits.
- **Session Management**: Handles cookies and sessions.
- **Robust Error Handling**: Provides custom error pages for various HTTP status codes.

## Configuration Schema

The server's behavior is defined by one or more JSON configuration files.

### Configuration Directives

- `servers`: An array of server block configurations.
- `server_name`: The server name (e.g., `localhost`).
- `host`: The IP address the server will bind to.
- `ports`: An array of ports the server will listen on.
- `error_pages`: A map of HTTP status codes to custom error page paths (e.g., `"404": "/errors/404.html"`).
- `client_max_body_size`: Maximum allowed size for client request bodies in bytes.
- `routes`: An array of route configurations.
    - `path`: The URL path for the route.
    - `methods`: An array of allowed HTTP methods (e.g., `["GET", "POST"]`).
    - `root`: Filesystem path to serve content from for this route.
    - `index`: Default file to serve if the URL is a directory (e.g., `index.html`).
    - `autoindex`: `true` or `false` to enable/disable directory listing.
    - `cgi_map`: A map associating file extensions with CGI executors (e.g., `{".py": "python3"}`).
    - `cgi_timeout`: The maximum time in milliseconds for a CGI script to run before it is terminated. Defaults to 5000.

## Usage

### Running the Server

```bash
cargo run -- --config <config_path>
```

The server can load configurations from a directory or a single JSON file.
