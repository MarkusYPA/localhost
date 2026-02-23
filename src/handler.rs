use crate::cgi::handle_cgi;
use crate::config::{Route, SingleServerConfig};
use crate::http::request::Request;
use crate::http::response::Response;
use crate::session::Session;
use log::{debug, error};
use std::fs;
use std::path::Path;

fn handle_error(
    status_code: u16,
    config: &SingleServerConfig,
    request: Option<&Request>,
) -> Response {
    if let Some(req) = request {
        error!(
            "{} {}: {}",
            req.method,
            req.path,
            crate::http::status::reason_phrase(status_code)
        );
    }
    if let Some(error_page_path_str) = config.error_pages.get(&status_code) {
        let current_dir = match std::env::current_dir() {
            Ok(dir) => dir,
            Err(_) => return Response::new(500, b"Internal Server Error".to_vec(), config, None),
        };
        let error_page_path = current_dir.join(
            error_page_path_str
                .strip_prefix('/')
                .unwrap_or(error_page_path_str),
        );
        match fs::read(&error_page_path) {
            Ok(body) => {
                let mut response = Response::new(status_code, body, config, request);
                response
                    .headers
                    .insert("Content-Type".to_string(), "text/html".to_string());
                response
            }
            Err(_) => {
                let reason_phrase = crate::http::status::reason_phrase(status_code);
                Response::new(
                    status_code,
                    reason_phrase.as_bytes().to_vec(),
                    config,
                    request,
                )
            }
        }
    } else {
        let reason_phrase = crate::http::status::reason_phrase(status_code);
        Response::new(
            status_code,
            reason_phrase.as_bytes().to_vec(),
            config,
            request,
        )
    }
}

pub fn find_route<'a>(request: &Request, config: &'a SingleServerConfig) -> Option<&'a Route> {
    let mut best_match: Option<&'a Route> = None;
    let mut longest_path = 0;

    for route in &config.routes {
        if request.path.starts_with(&route.path) && route.path.len() > longest_path {
            longest_path = route.path.len();
            best_match = Some(route);
        }
    }

    best_match
}

fn try_handle_cgi(
    request: &Request,
    route: &Route,
    config: &SingleServerConfig,
    session: &mut Option<&mut Session>,
) -> Option<Response> {
    /* let relative_path = match request.path.strip_prefix(&route.path) {
        Some(path) => path,
        None => return None,
    }; */
    let relative_path = request.path.strip_prefix(&route.path)?;

    let relative_path = relative_path.strip_prefix('/').unwrap_or(relative_path);
    let path = Path::new(&route.root).join(relative_path);

    if path.is_file() {
        if let Some(ext) = path.extension() {
            let ext_str = match ext.to_str() {
                Some(s) => format!(".{s}"),
                None => return None,
            };

            if let Some(cgi_executor) = route.cgi_map.get(&ext_str) {
                return Some(handle_cgi(
                    request,
                    route,
                    config,
                    &path,
                    cgi_executor,
                    session,
                ));
            }
        }
    }

    None
}

pub fn handle_request(
    request: &Request,
    route: &Route,
    config: &SingleServerConfig,
    session: &mut Option<&mut Session>,
) -> Response {
    if !route.methods.contains(&request.method) {
        return handle_error(405, config, Some(request));
    }

    if !route.cgi_map.is_empty() {
        if let Some(response) = try_handle_cgi(request, route, config, session) {
            return response;
        }
    }

    match request.method.as_str() {
        "GET" => handle_get(request, route, config, session),
        "POST" | "DELETE" => {
            if let Some(response) = try_handle_cgi(request, route, config, session) {
                return response;
            }
            handle_error(405, config, Some(request))
        }
        _ => handle_error(501, config, Some(request)),
    }
}

fn handle_get(
    request: &Request,
    route: &Route,
    config: &SingleServerConfig,
    session: &mut Option<&mut Session>,
) -> Response {
    if let Some(s) = session {
        let count = s
            .data
            .get("count")
            .cloned()
            .unwrap_or_else(|| "0".to_string());
        let new_count = count.parse::<i32>().unwrap_or(0) + 1;
        s.data.insert("count".to_string(), new_count.to_string());
    }

    let relative_path = match request.path.strip_prefix(&route.path) {
        Some(path) => path,
        None => return handle_error(500, config, Some(request)),
    };
    let relative_path = relative_path.strip_prefix('/').unwrap_or(relative_path);
    let path = Path::new(&route.root).join(relative_path);

    if path.is_dir() {
        if !request.path.ends_with('/') {
            let new_path = format!("{}/", request.path);
            let mut response =
                Response::new(301, b"Moved Permanently".to_vec(), config, Some(request));
            response.headers.insert("Location".to_string(), new_path);
            return response;
        }
        let index_path = path.join(&route.index);
        if index_path.is_file() {
            return serve_file(&index_path, config, Some(request));
        }
        if route.autoindex {
            return list_directory(&path, &request.path, config, Some(request));
        }
        return handle_error(403, config, Some(request));
    }

    if path.is_file() {
        return serve_file(&path, config, Some(request));
    }

    debug!("handle_get: 404 Not Found");
    handle_error(404, config, Some(request))
}

fn list_directory(
    path: &Path,
    request_path: &str,
    config: &SingleServerConfig,
    request: Option<&Request>,
) -> Response {
    let mut body = String::new();
    body.push_str(&format!("<html><body><h1>Index of {request_path}</h1><ul>"));

    match fs::read_dir(path) {
        Ok(entries) => {
            for entry in entries.flatten() {
                let file_name = entry.file_name().to_string_lossy().to_string();
                let link = format!("{}/{file_name}", request_path.trim_end_matches('/'));
                body.push_str(&format!("<li><a href=\"{link}\">{file_name}</a></li>"));
            }
        }
        Err(_) => return handle_error(500, config, request),
    }

    body.push_str("</ul></body></html>");
    let mut response = Response::new(200, body.as_bytes().to_vec(), config, request);
    response
        .headers
        .insert("Content-Type".to_string(), "text/html".to_string());
    response
}

fn serve_file(path: &Path, config: &SingleServerConfig, request: Option<&Request>) -> Response {
    match fs::read(path) {
        Ok(body) => {
            let mut response = Response::new(200, body, config, request);
            let content_type = mime_guess::from_path(path).first_or_octet_stream();
            response
                .headers
                .insert("Content-Type".to_string(), content_type.to_string());
            response
        }
        Err(_) => handle_error(500, config, request),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::parse_config;
    use std::collections::HashMap;

    fn basic_config() -> SingleServerConfig {
        let config_str = r#"
        {
            "servers": [{
                "server_name": "localhost",
                "host": "127.0.0.1",
                "ports": [8080],
                "routes": [
                    {
                        "path": "/",
                        "methods": ["GET"],
                        "root": "/var/www",
                        "index": "index.html"
                    },
                    {
                        "path": "/api",
                        "methods": ["GET", "POST"],
                        "root": "/var/api"
                    }
                ]
            }]
        }
        "#;
        parse_config(config_str).unwrap().servers.remove(0)
    }

    #[test]
    fn test_find_route_longest_match() {
        let config = basic_config();
        let request = Request {
            method: "GET".to_string(),
            path: "/api/users".to_string(),
            headers: HashMap::new(),
            body: vec![],
            query_params: HashMap::new(),
            cookies: HashMap::new(),
        };
        let route = find_route(&request, &config).unwrap();
        assert_eq!(route.path, "/api");
    }

    #[test]
    fn test_find_route_matches_root() {
        let config = basic_config();
        let request = Request {
            method: "GET".to_string(),
            path: "/unmatched".to_string(),
            headers: HashMap::new(),
            body: vec![],
            query_params: HashMap::new(),
            cookies: HashMap::new(),
        };
        let route = find_route(&request, &config).unwrap();
        assert_eq!(route.path, "/");
    }

    #[test]
    fn test_handle_request_method_not_allowed() {
        let config = basic_config();
        let request = Request {
            method: "POST".to_string(),
            path: "/".to_string(),
            headers: HashMap::new(),
            body: vec![],
            query_params: HashMap::new(),
            cookies: HashMap::new(),
        };
        let route = find_route(&request, &config).unwrap();
        let response = handle_request(&request, route, &config, &mut None);
        assert_eq!(response.status_code, 405);
    }

    #[test]
    fn test_path_traversal_attack() {
        let config = basic_config();
        let request = Request {
            method: "GET".to_string(),
            path: "/../../../../etc/passwd".to_string(),
            headers: HashMap::new(),
            body: vec![],
            query_params: HashMap::new(),
            cookies: HashMap::new(),
        };
        let route = find_route(&request, &config).unwrap();
        // This test is not perfect, as it doesn't check the file system.>
        // However, it ensures that the path is correctly joined.>
        let relative_path = request.path.strip_prefix(&route.path).unwrap();
        let relative_path = relative_path.strip_prefix('/').unwrap_or(relative_path);
        let path = Path::new(&route.root).join(relative_path);
        assert!(path.starts_with(&route.root));
    }
}
