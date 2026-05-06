# Testing Protocols

This document describes the testing methodology and failure mode analysis for the HTTP Server.

## 1. Unit & Integration Tests (Rust)

These tests are self-contained and manage the server lifecycle internally.
```bash
cargo test
```

## 2. Go Integration Tests (Black-box)

These tests act as a client, verifying the server's behavior from the outside.
1. Start the server: `cargo run -- --config comprehensive_server.json`
2. Run tests: `cd go_tests/ && go test -v .`

## 3. Memory Leak Testing (ASan)

Utilizes AddressSanitizer built into the Rust compiler.
```bash
RUSTFLAGS="-Zsanitizer=address" cargo run
```
Stop the server with `Ctrl+C` to see the leak report.

## 4. Stress Testing

Use `siege` to verify stability under high concurrency:
```bash
siege -b -t30s http://127.0.0.1:8081/
```

## 5. Failure Mode Analysis

- **Invalid Config**: Tested via `cargo run -- --config test_configs/`. The server should log errors for invalid files but continue loading valid ones.
- **Protocol Violations**: Verified via Go integration tests (e.g., malformed headers, dual CL+TE).
- **CGI Timeouts**: Configurable via `cgi_timeout` in `server.json`.
