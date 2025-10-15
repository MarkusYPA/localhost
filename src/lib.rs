pub mod cgi;
pub mod config;
pub mod handler;
pub mod http;
pub mod io;
pub mod server;
pub mod session;

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    let config_content = std::fs::read_to_string("server.json")
        .map_err(|e| format!("Failed to read server.json: {e}"))?;
    let config = config::parse_config(&config_content)
        .map_err(|e| format!("Failed to parse server.json: {e}"))?;
    server::run(config).map_err(|e| e.into())
}
