fn main() {
    if let Err(e) = http_server::run(None) {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}
