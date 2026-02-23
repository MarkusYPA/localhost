use http_server;
use lazy_static::lazy_static;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::sync::Once;
use std::thread;
use std::time::{Duration, Instant};

// This setup is copied from integration_test.rs to ensure the server is started once.
static START: Once = Once::new();

lazy_static! {
    static ref SERVER_THREAD: thread::JoinHandle<()> = {
        thread::spawn(|| {
            let config_content = std::fs::read_to_string("comprehensive_server.json").unwrap();
            let server_configs = vec![http_server::config::parse_config(&config_content).unwrap()];
            if let Err(e) = http_server::run(server_configs) {
                eprintln!("Server error in test: {}", e);
                std::process::exit(1);
            }
        })
    };
}

fn setup() {
    START.call_once(|| {
        // Force the lazy_static to be initialized and start the server.
        let _ = &*SERVER_THREAD;
        // Give the server a moment to start up.
        thread::sleep(Duration::from_secs(2));
    });
}

#[test]
fn test_slow_client_read() {
    setup();

    let mut stream = TcpStream::connect("127.0.0.1:8081").expect("Failed to connect to server");
    stream
        .set_nonblocking(true)
        .expect("Failed to set non-blocking");

    // Send a request for a file we know the content of.
    let request = b"GET /site/index.html HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n";
    stream.write_all(request).expect("Failed to write request");

    let mut response_buffer = Vec::new();
    let mut byte_buffer = [0; 1]; // Read one byte at a time
    let start_time = Instant::now();
    let timeout = Duration::from_secs(45);

    loop {
        if start_time.elapsed() > timeout {
            panic!("Test timed out while waiting for response.");
        }

        match stream.read(&mut byte_buffer) {
            Ok(n) => {
                if n == 0 {
                    // End of stream, server closed the connection.
                    break;
                }
                // Successfully read n bytes.
                response_buffer.extend_from_slice(&byte_buffer[..n]);
                // Simulate a slow client by sleeping after reading each byte.
                thread::sleep(Duration::from_millis(5));
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                // No data available right now, sleep and try again.
                thread::sleep(Duration::from_millis(100));
                continue;
            }
            Err(e) => {
                panic!("Failed to read from stream: {}", e);
            }
        }
    }

    let response_str = String::from_utf8_lossy(&response_buffer);

    // Verify that we received a valid HTTP response and the expected content.
    assert!(response_str.starts_with("HTTP/1.1 200 OK"));
    assert!(response_str.contains("<h1>Welcome to the test website!</h1>"));
    println!("Successfully received full response from server with slow reading.");
}
