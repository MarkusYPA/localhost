use std::thread;
use std::time::Duration;
use http_server;
use lazy_static::lazy_static;
use std::sync::Once;

static START: Once = Once::new();

lazy_static! {
    static ref SERVER_THREAD: thread::JoinHandle<()> = {
        thread::spawn(|| {
            http_server::run();
        })
    };
}

fn setup() {
    START.call_once(|| {
        // force the lazy_static to be initialized
        let _ = &*SERVER_THREAD;
        // Give the server a moment to start
        thread::sleep(Duration::from_secs(1));
    });
}

#[test]
fn test_get_index() {
    setup();
    // Make a request to the server
    let resp = reqwest::blocking::get("http://127.0.0.1:8081/").unwrap();

    // Check the status code
    assert_eq!(resp.status(), 200);

    // Check the body
    let body = resp.text().unwrap();
    assert_eq!(body, "<html><body><h1>Hello from index.html!</h1></body></html>");
}

#[test]
fn test_404_not_found() {
    setup();
    // Make a request to a non-existent file
    let resp = reqwest::blocking::get("http://127.0.0.1:8081/non-existent-file").unwrap();

    // Check the status code
    assert_eq!(resp.status(), 404);
}