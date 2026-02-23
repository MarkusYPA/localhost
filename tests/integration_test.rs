use http_server;
use lazy_static::lazy_static;
use std::sync::Once;
use std::thread;
use std::time::Duration;

static START: Once = Once::new();

lazy_static! {
    static ref SERVER_THREAD: thread::JoinHandle<()> = {
        thread::spawn(|| {
            let config_content = std::fs::read_to_string("comprehensive_server.json").unwrap();
            let server_configs = vec![http_server::config::parse_config(&config_content).unwrap()];
            if let Err(e) = http_server::run(server_configs) {
                eprintln!("Server error: {}", e);
                std::process::exit(1);
            }
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
    assert!(body.contains("Hello from site 1!"));
}

#[test]
fn test_404_not_found() {
    setup();
    // Make a request to a non-existent file
    let resp = reqwest::blocking::get("http://127.0.0.1:8081/non-existent-file").unwrap();

    // Check the status code
    assert_eq!(resp.status(), 404);
}

#[test]
fn test_serve_static_website() {
    setup();

    // Test the index.html file
    let resp = reqwest::blocking::get("http://127.0.0.1:8081/site/").unwrap();
    assert_eq!(resp.status(), 200);
    let body = resp.text().unwrap();
    assert!(body.contains("<h1>Welcome to the test website!</h1>"));

    // Test the style.css file
    let resp = reqwest::blocking::get("http://127.0.0.1:8081/site/style.css").unwrap();
    assert_eq!(resp.status(), 200);
    let body = resp.text().unwrap();
    assert!(body.contains("background-color: #f0f0f0;"));

    // Test the script.js file
    let resp = reqwest::blocking::get("http://127.0.0.1:8081/site/script.js").unwrap();
    assert_eq!(resp.status(), 200);
    let body = resp.text().unwrap();
    assert!(body.contains("console.log(\"Hello from script.js!\");"));

    // Test the image.png file
    let resp = reqwest::blocking::get("http://127.0.0.1:8081/site/image.png").unwrap();
    assert_eq!(resp.status(), 200);
    assert_eq!(resp.headers()["content-type"], "image/png");
}

#[test]
fn test_routing() {
    setup();

    // Test the /site/ route
    let resp = reqwest::blocking::get("http://127.0.0.1:8081/site/").unwrap();
    assert_eq!(resp.status(), 200);
    let body = resp.text().unwrap();
    assert!(body.contains("<h1>Welcome to the test website!</h1>"));

    // Test the /cgi-bin/ route
    let resp = reqwest::blocking::get("http://127.0.0.1:8081/cgi-bin/test.py").unwrap();
    assert_eq!(resp.status(), 200);
    let body = resp.text().unwrap();
    assert!(body.contains("Hello from Python CGI!"));
}

#[test]
fn test_bash_cgi() {
    setup();
    // Test the /cgi-bin/ route
    let resp = reqwest::blocking::get("http://127.0.0.1:8081/cgi-bin/hello.sh").unwrap();
    assert_eq!(resp.status(), 200);
    let body = resp.text().unwrap();
    assert!(body.contains("<h1>Hello from Bash CGI!</h1>"));
}

#[test]
fn test_method_not_allowed() {
    setup();

    // Make a POST request to a route that only accepts GET
    let client = reqwest::blocking::Client::new();
    let resp = client.post("http://127.0.0.1:8081/site/").send().unwrap();
    assert_eq!(resp.status(), 405);
}
