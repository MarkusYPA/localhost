# Memory Leak Testing Instructions

To perform memory leak testing, we will use the AddressSanitizer (ASan), which is built into the Rust compiler.

## Steps

1. **Run the tests with the AddressSanitizer enabled:**
   You can do this by setting the `RUSTFLAGS` environment variable when running `cargo test`:
   ```
   RUSTFLAGS="-Zsanitizer=address" cargo test
   ```

2. **Run the server with the AddressSanitizer enabled:**
   To run the server with the sanitizer, you can use the following command:
   ```
   RUSTFLAGS="-Zsanitizer=address" cargo run
   ```

3. **Interpret the output:**
   If the AddressSanitizer detects a memory leak, it will print a detailed report to the console when the program exits. The report will include a stack trace of where the leaked memory was allocated.

4. **Stress the server:**
   To effectively test for memory leaks, you should exercise the server with a variety of requests while it is running with the sanitizer. The `curl` commands you ran are a good start, but to be more thorough, I would recommend running the full integration test suite or the `siege` command in a separate terminal while the sanitized server is running:
   ```
   cargo test
   ```
   or
   ```
   siege -b -t30s http://127.0.0.1:8081/
   ```

5. **Stop the server and check the output:**
   After the tests or `siege` have finished, stop the server with `Ctrl+C`, and you will see the AddressSanitizer report in the server's terminal if any leaks were detected.
