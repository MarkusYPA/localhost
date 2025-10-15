fn main() {
    let args: Vec<String> = std::env::args().collect();
    let config_path = if args.len() > 2 && args[1] == "--config" {
        Some(args[2].as_str())
    } else {
        None
    };

    if let Err(e) = http_server::run(config_path) {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}
