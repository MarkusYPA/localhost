pub mod cgi;
pub mod config;
pub mod handler;
pub mod http;
pub mod io;
pub mod server;
pub mod session;

use clap::Parser;
use config::ServerConfig;
use log::{error, info, warn};
use std::fs;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    #[arg(short, long, default_value = "config/server.json")]
    pub config: String,

    #[arg(short, long)]
    pub debug: bool,
}

pub fn setup_logger(debug: bool) -> Result<(), fern::InitError> {
    let base_config = fern::Dispatch::new().format(|out, message, record| {
        out.finish(format_args!(
            "{} [{}] {} - {}",
            chrono::Local::now().format("%Y-%m-%dT%H:%M:%S"),
            record.level(),
            record.target(),
            message
        ))
    });

    let file_config = fern::Dispatch::new()
        .level(log::LevelFilter::Debug)
        .chain(fern::log_file("./logs/server.log")?);

    let stdout_config = fern::Dispatch::new()
        .level(if debug {
            log::LevelFilter::Debug
        } else {
            log::LevelFilter::Info
        })
        .chain(std::io::stdout());

    base_config
        .chain(file_config)
        .chain(stdout_config)
        .apply()?;

    Ok(())
}

pub fn init(args: &Args) -> Result<Vec<ServerConfig>, Box<dyn std::error::Error>> {
    if let Err(e) = setup_logger(args.debug) {
        return Err(format!("Error setting up logger: {}", e).into());
    }

    let path = PathBuf::from(&args.config);
    let mut server_configs = Vec::new();

    if path.is_dir() {
        for entry in fs::read_dir(&path)? {
            let entry = entry?;
            let file_path = entry.path();
            //if file_path.is_file() && file_path.extension().map_or(false, |ext| ext == "json") {
            if file_path.is_file() && file_path.extension().is_some_and(|ext| ext == "json") {
                let file_name = file_path.file_name().unwrap().to_string_lossy().to_string();
                match fs::read_to_string(&file_path) {
                    Ok(content) => match config::parse_config(&content) {
                        Ok(server_config) => {
                            server_configs.push(server_config);
                            info!("Successfully loaded config: {}", file_name);
                        }
                        Err(e) => {
                            warn!("Error parsing config file {}: {}", file_name, e);
                        }
                    },
                    Err(e) => {
                        error!("Error reading config file {}: {}", file_name, e);
                    }
                }
            }
        }
    } else if path.is_file() {
        let content = fs::read_to_string(&path)?;
        let server_config = config::parse_config(&content)?;
        server_configs.push(server_config);
    } else {
        return Err(format!(
            "Config path '{}' is not a valid file or directory",
            args.config
        )
        .into());
    }

    Ok(server_configs)
}

pub fn run(all_configs: Vec<ServerConfig>) -> Result<(), Box<dyn std::error::Error>> {
    if all_configs.is_empty() {
        return Err("No server configurations provided. Exiting.".into());
    }

    server::run(all_configs).map_err(|e| e.into())
}
