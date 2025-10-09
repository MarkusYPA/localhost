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
            "error_page_400:" => {
                error_pages.insert(400, parts[1].to_string());
            }
            "error_page_403:" => {
                error_pages.insert(403, parts[1].to_string());
            }
            "error_page_405:" => {
                error_pages.insert(405, parts[1].to_string());
            }
            "error_page_413:" => {
                error_pages.insert(413, parts[1].to_string());
            }
            "error_page_500:" => {
                error_pages.insert(500, parts[1].to_string());
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

                let mut route_ended_correctly = false;
                while let Some(route_line) = lines.next() {
                    if route_line == "}" {
                        route_ended_correctly = true;
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
                if !route_ended_correctly {
                    return Err(format!("Invalid route definition: missing '}}' for route {}", path));
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


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_config() {
        let config_str = r#"
server {
    host: 127.0.0.1
    port: 8080
    client_max_body_size: 1024
}
"#;
        let config = parse_config(config_str).unwrap();
        assert_eq!(config.host, "127.0.0.1");
        assert_eq!(config.ports, vec![8080]);
        assert_eq!(config.client_max_body_size, 1024);
    }

    #[test]
    fn test_config_with_route() {
        let config_str = r#"
server {
    host: 127.0.0.1
    port: 8080
    route / {
        methods: GET POST
        root: /var/www
        index: index.html
        autoindex: on
    }
}
"#;
        let config = parse_config(config_str).unwrap();
        assert_eq!(config.routes.len(), 1);
        let route = &config.routes[0];
        assert_eq!(route.path, "/");
        assert_eq!(route.methods, vec!["GET", "POST"]);
        assert_eq!(route.root, "/var/www");
        assert_eq!(route.index, "index.html");
        assert!(route.autoindex);
    }

    #[test]
    fn test_config_with_cgi() {
        let config_str = r#"
server {
    host: 127.0.0.1
    port: 8080
    route /cgi-bin {
        methods: GET POST
        root: /var/cgi
        cgi_extension: .py python3
    }
}
"#;
        let config = parse_config(config_str).unwrap();
        assert_eq!(config.routes.len(), 1);
        let route = &config.routes[0];
        assert_eq!(route.path, "/cgi-bin");
        assert_eq!(route.cgi_map.get(".py").unwrap(), "python3");
    }

    #[test]
    fn test_invalid_config() {
        let config_str = r#"
server {
    host: 127.0.0.1
    port: not-a-number
}
"#;
        let config = parse_config(config_str);
        assert!(config.is_err());
    }
}