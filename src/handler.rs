use crate::config::{Route, ServerConfig};
use crate::http::request::Request;
use crate::http::response::Response;
use crate::session::Session;
use std::fs;
use std::path::Path;
use crate::cgi::handle_cgi;

fn handle_error(status_code: u16, config: &ServerConfig) -> Response {
    if let Some(error_page_path_str) = config.error_pages.get(&status_code) {
        let current_dir = match std::env::current_dir() {
            Ok(dir) => dir,
            Err(_) => return Response::new(500, b"Internal Server Error".to_vec()),
        };
        let error_page_path = current_dir.join(error_page_path_str.strip_prefix('/').unwrap_or(error_page_path_str));
        match fs::read(&error_page_path) {
            Ok(body) => {
                let mut response = Response::new(status_code, body);
                response.headers.insert("Content-Type".to_string(), "text/html".to_string());
                response
            }
            Err(_) => {
                let reason_phrase = crate::http::status::reason_phrase(status_code);
                Response::new(status_code, reason_phrase.as_bytes().to_vec())
            }
        }
    } else {
        let reason_phrase = crate::http::status::reason_phrase(status_code);
        Response::new(status_code, reason_phrase.as_bytes().to_vec())
    }
}

pub fn find_route<'a>(request: &Request, config: &'a ServerConfig) -> Option<&'a Route> {
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

pub fn handle_request(request: &Request, route: &Route, config: &ServerConfig, session: &mut Option<&mut Session>) -> Response {
    if !route.methods.contains(&request.method) {
        return handle_error(405, config);
    }

    match request.method.as_str() {
        "GET" => handle_get(request, route, config, session),
        _ => handle_error(501, config),
    }
}

fn handle_get(request: &Request, route: &Route, config: &ServerConfig, session: &mut Option<&mut Session>) -> Response {
    if let Some(s) = session {
        let count = s.data.get("count").cloned().unwrap_or_else(|| "0".to_string());
        let new_count = count.parse::<i32>().unwrap_or(0) + 1;
        s.data.insert("count".to_string(), new_count.to_string());
        println!("Session count: {}", new_count);
    }

    let relative_path = match request.path.strip_prefix(&route.path) {
        Some(path) => path,
        None => return handle_error(500, config),
    };
    let relative_path = relative_path.strip_prefix('/').unwrap_or(relative_path);
    let path = Path::new(&route.root).join(relative_path);
    println!("handle_get: path = {:?}", path);

    if path.is_dir() {
        println!("handle_get: path is dir");
        let index_path = path.join(&route.index);
        if index_path.is_file() {
            println!("handle_get: index file found");
            return serve_file(&index_path, config);
        }
        // TODO: Directory listing
        return handle_error(403, config);
    }

    if path.is_file() {
        println!("handle_get: path is file");
        if let Some(ext) = path.extension() {
            let ext_str = match ext.to_str() {
                Some(s) => format!(".{}", s),
                None => return handle_error(400, config),
            };
            println!("handle_get: file extension = {}", ext_str);
            if let Some(cgi_executor) = route.cgi_map.get(&ext_str) {
                println!("handle_get: CGI executor found = {}", cgi_executor);
                return handle_cgi(request, route, config, &path, cgi_executor, session);
            } else {
                println!("handle_get: No CGI executor found for extension {:?}", ext);
                return serve_file(&path, config);
            }
        } else {
            println!("handle_get: No file extension found");
            return serve_file(&path, config);
        }
    }

    println!("handle_get: 404 Not Found");
    handle_error(404, config)
}

fn serve_file(path: &Path, config: &ServerConfig) -> Response {
    match fs::read(path) {
        Ok(body) => {
            let mut response = Response::new(200, body);
            let content_type = mime_guess::from_path(path).first_or_octet_stream();
            response.headers.insert("Content-Type".to_string(), content_type.to_string());
            response
        }
        Err(_) => handle_error(500, config),
    }
}

