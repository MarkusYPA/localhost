pub mod cgi;
pub mod config;
pub mod handler;
pub mod http;
pub mod io;
pub mod server;
pub mod session;

pub fn run(config_path_arg: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
    let default_path = "config".to_string();
    let path_str = config_path_arg.unwrap_or(&default_path);
    let path = std::path::PathBuf::from(path_str);

    let mut server_configs = Vec::new();

    if path.is_dir() {
        for entry in std::fs::read_dir(&path)? {
            let entry = entry?;
            let file_path = entry.path();

            if file_path.is_file() && file_path.extension().map_or(false, |ext| ext == "json") {
                let file_name = file_path.file_name().unwrap().to_string_lossy().to_string();
                match std::fs::read_to_string(&file_path) {
                    Ok(content) => {
                        match config::parse_config(&content) {
                            Ok(server_config) => {
                                server_configs.push(server_config);
                                println!("Successfully loaded config: {}", file_name);
                            },
                            Err(e) => {
                                eprintln!("Error parsing config file {}: {}", file_name, e);
                            }
                        }
                    },
                    Err(e) => {
                        eprintln!("Error reading config file {}: {}", file_name, e);
                    }
                }
            }
        }
    } else if path.is_file() && path.extension().map_or(false, |ext| ext == "json") {
        let file_name = path.file_name().unwrap().to_string_lossy().to_string();
        match std::fs::read_to_string(&path) {
            Ok(content) => {
                match config::parse_config(&content) {
                    Ok(server_config) => {
                        server_configs.push(server_config);
                        println!("Successfully loaded config: {}", file_name);
                    },
                    Err(e) => {
                        eprintln!("Error parsing config file {}: {}", file_name, e);
                    }
                }
            },
            Err(e) => {
                eprintln!("Error reading config file {}: {}", file_name, e);
            }
        }
    } else {
        eprintln!("Error: Provided config path '{}' is neither a directory nor a .json file.", path_str);
        return Err(format!("Invalid config path: {}", path_str).into());
    }

    if server_configs.is_empty() {
        eprintln!("No valid server configurations found in {}. Exiting.", path_str);
        return Ok(());
    }

    server::run(server_configs).map_err(|e| e.into())
}
