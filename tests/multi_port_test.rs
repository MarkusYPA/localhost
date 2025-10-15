use http_server;
use lazy_static::lazy_static;
use std::sync::Once;
use std::thread;
use std::time::Duration;

lazy_static! {
    static ref SERVER_THREAD: thread::JoinHandle<()> = {
        thread::spawn(|| {
            if let Err(e) = http_server::run(Some("server_multi_port.json")) {
                eprintln!("Server error: {}", e);
                std::process::exit(1);
            }
        })
    };
}

static START: Once = Once::new();

fn setup() {
    START.call_once(|| {
        // force the lazy_static to be initialized
        let _ = &*SERVER_THREAD;
        // Give the server a moment to start
        thread::sleep(Duration::from_secs(1));
    });
}

#[test]
fn test_multi_port() {
    setup();

    let client = reqwest::blocking::Client::new();

    // Make a request to the first port
    let resp1 = client.get("http://127.0.0.1:8081/").send().unwrap();
    assert_eq!(resp1.status(), 200);
    let body1 = resp1.text().unwrap();
    assert!(body1.contains("Hello from www/index.html"));

    // Make a request to the second port
    let resp2 = client.get("http://127.0.0.1:8082/").send().unwrap();
    assert_eq!(resp2.status(), 200);
    let body2 = resp2.text().unwrap();
    assert!(body2.contains("Hello from www/index.html"));
}
