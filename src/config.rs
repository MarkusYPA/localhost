use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
pub struct VirtualServer {
    pub host: String,
    pub routes: Vec<Route>,
}

#[derive(Debug, Deserialize)]
pub struct ServerConfig {
    pub ports: Vec<u16>,
    pub servers: Vec<VirtualServer>,
    #[serde(default)]
    pub error_pages: HashMap<u16, String>,
    #[serde(default = "default_client_max_body_size")]
    pub client_max_body_size: usize,
}

fn default_client_max_body_size() -> usize {
    1024 * 1024 // 1MB
}

#[derive(Debug, Deserialize)]
pub struct Route {
    pub path: String,
    pub methods: Vec<String>,
    pub root: String,
    #[serde(default)]
    pub index: String,
    #[serde(default)]
    pub cgi_map: HashMap<String, String>,
    #[serde(default)]
    pub autoindex: bool,
}

pub fn parse_config(config_content: &str) -> Result<ServerConfig, String> {
    serde_json::from_str(config_content).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_config() {
        let config_str = r#"
        {
            "ports": [8080],
            "client_max_body_size": 1024,
            "servers": [
                {
                    "host": "127.0.0.1",
                    "routes": []
                }
            ]
        }
        "#;
        let config = parse_config(config_str).unwrap();
        assert_eq!(config.ports, vec![8080]);
        assert_eq!(config.client_max_body_size, 1024);
        assert_eq!(config.servers[0].host, "127.0.0.1");
    }

    #[test]
    fn test_config_with_route() {
        let config_str = r#"
        {
            "ports": [8080],
            "servers": [
                {
                    "host": "127.0.0.1",
                    "routes": [
                        {
                            "path": "/",
                            "methods": ["GET", "POST"],
                            "root": "/var/www",
                            "index": "index.html",
                            "autoindex": true
                        }
                    ]
                }
            ]
        }
        "#;
        let config = parse_config(config_str).unwrap();
        assert_eq!(config.servers[0].routes.len(), 1);
        let route = &config.servers[0].routes[0];
        assert_eq!(route.path, "/");
        assert_eq!(route.methods, vec!["GET", "POST"]);
        assert_eq!(route.root, "/var/www");
        assert_eq!(route.index, "index.html");
        assert!(route.autoindex);
    }

    #[test]
    fn test_config_with_cgi() {
        let config_str = r#"
        {
            "ports": [8080],
            "servers": [
                {
                    "host": "127.0.0.1",
                    "routes": [
                        {
                            "path": "/cgi-bin",
                            "methods": ["GET", "POST"],
                            "root": "/var/cgi",
                            "cgi_map": {
                                ".py": "python3"
                            }
                        }
                    ]
                }
            ]
        }
        "#;
        let config = parse_config(config_str).unwrap();
        assert_eq!(config.servers[0].routes.len(), 1);
        let route = &config.servers[0].routes[0];
        assert_eq!(route.path, "/cgi-bin");
        assert_eq!(route.cgi_map.get(".py").unwrap(), "python3");
    }

    #[test]
    fn test_invalid_config() {
        let config_str = r#"
        {
            "ports": "not-a-number",
            "servers": []
        }
        "#;
        let config = parse_config(config_str);
        assert!(config.is_err());
    }
}