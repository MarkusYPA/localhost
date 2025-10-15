pub mod cgi;
pub mod config;
pub mod handler;
pub mod http;
pub mod io;
pub mod server;
pub mod session;

pub fn run(config_path: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
    let config_path = config_path.unwrap_or("server.json");
    let config_content = std::fs::read_to_string(config_path)
        .map_err(|e| format!("Failed to read {config_path}: {e}"))?;
    let config = config::parse_config(&config_content)
        .map_err(|e| format!("Failed to parse server.json: {e}"))?;
    server::run(config).map_err(|e| e.into())
}
