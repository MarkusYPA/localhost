fn main() {
    if let Err(e) = http_server::run() {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}