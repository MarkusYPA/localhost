
use http_server;
use lazy_static::lazy_static;
use std::sync::Once;
use std::thread;
use std::time::Duration;

lazy_static! {
    static ref SERVER_THREAD: thread::JoinHandle<()> = {
        thread::spawn(|| {
            if let Err(e) = http_server::run(Some("server_error_page.json")) {
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
fn test_custom_error_page() {
    setup();

    let client = reqwest::blocking::Client::new();

    // Make a request to a non-existent page
    let resp = client
        .get("http://127.0.0.1:8085/non_existent_page")
        .send()
        .unwrap();
    assert_eq!(resp.status(), 404);
    let body = resp.text().unwrap();
    assert!(body.contains("The page you requested could not be found."));
}
