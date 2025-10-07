use std::collections::HashMap;

#[derive(Debug)]
pub struct ServerConfig {
    pub host: String,
    pub ports: Vec<u16>,
    pub routes: Vec<Route>,
    pub error_pages: HashMap<u16, String>,
    pub client_max_body_size: usize,
}

#[derive(Debug)]
pub struct Route {
    pub path: String,
    pub methods: Vec<String>,
    pub root: String,
    pub index: String,
    pub cgi_map: HashMap<String, String>,
    pub autoindex: bool,
}

pub fn parse_config(config_content: &str) -> Result<ServerConfig, String> {
    let mut lines = config_content.lines().map(|s| s.trim()).filter(|s| !s.is_empty());

    if lines.next() != Some("server {") {
        return Err("Expected 'server {'".to_string());
    }

    let mut host = String::new();
    let mut ports = Vec::new();
    let mut routes = Vec::new();
    let mut error_pages = HashMap::new();
    let mut client_max_body_size = 0;

    while let Some(line) = lines.next() {
        if line == "}" {
            break;
        }

        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 2 {
            continue;
        }

        match parts[0] {
            "host:" => host = parts[1].to_string(),
            "port:" => ports.push(parts[1].parse().map_err(|_| "Invalid port".to_string())?),
            "error_page_404:" => {
                error_pages.insert(404, parts[1].to_string());
            }
            "client_max_body_size:" => {
                client_max_body_size = parts[1].parse().map_err(|_| "Invalid client_max_body_size".to_string())?;
            }
            "route" => {
                if parts.len() != 3 || parts[2] != "{" {
                    return Err(format!("Invalid route definition: {}", line));
                }
                let path = parts[1].to_string();

                let mut methods = Vec::new();
                let mut root = String::new();
                let mut index = String::new();
                let mut cgi_map = HashMap::new();
                let mut autoindex = false;

                while let Some(route_line) = lines.next() {
                    if route_line == "}" {
                        break;
                    }
                    let route_parts: Vec<&str> = route_line.split_whitespace().collect();
                    if route_parts.len() < 2 {
                        continue;
                    }
                    match route_parts[0] {
                        "methods:" => methods = route_parts[1..].iter().map(|s| s.to_string()).collect(),
                        "root:" => root = route_parts[1].to_string(),
                        "index:" => index = route_parts[1].to_string(),
                        "autoindex:" => autoindex = route_parts[1] == "on",
                        "cgi_extension:" => {
                            if route_parts.len() == 3 {
                                cgi_map.insert(route_parts[1].to_string(), route_parts[2].to_string());
                            }
                        }
                        _ => {}
                    }
                }
                routes.push(Route {
                    path,
                    methods,
                    root,
                    index,
                    cgi_map,
                    autoindex,
                });
            }
            _ => {}
        }
    }

    Ok(ServerConfig {
        host,
        ports,
        routes,
        error_pages,
        client_max_body_size,
    })
}