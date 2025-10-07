use std::fs;

pub mod cgi;
pub mod config;
pub mod handler;
pub mod http;
pub mod io;
pub mod server;
pub mod session;

fn main() {
    let config_content = fs::read_to_string("server.conf").expect("Should have been able to read the file");
    let config = config::parse_config(&config_content).unwrap();

    if let Err(e) = server::run(config) {
        eprintln!("Error: {}", e);
    }
}
