use crate::config::SingleServerConfig;
use crate::http::request::Request;
use std::collections::HashMap;

#[derive(Debug)]
pub struct Response {
    pub status_code: u16,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
}

impl Response {
    pub fn new(
        status_code: u16,
        body: Vec<u8>,
        config: &SingleServerConfig,
        request: Option<&Request>,
    ) -> Self {
        let mut headers = HashMap::new();
        let mut connection_type = config.connection_type.clone();

        if let Some(req) = request {
            if let Some(conn_header) = req.headers.get("connection") {
                if conn_header.eq_ignore_ascii_case("close") {
                    connection_type = "close".to_string();
                }
            }
        }

        headers.insert("Connection".to_string(), connection_type);
        headers.insert("Content-Length".to_string(), body.len().to_string());

        Self {
            status_code,
            headers,
            body,
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let reason_phrase = crate::http::status::reason_phrase(self.status_code);
        let mut response_str = format!("HTTP/1.1 {} {}\r\n", self.status_code, reason_phrase);
        for (key, value) in &self.headers {
            response_str.push_str(&format!("{}: {}\r\n", key, value));
        }
        response_str.push_str("\r\n");

        let mut response_bytes = response_str.as_bytes().to_vec();
        response_bytes.extend_from_slice(&self.body);
        response_bytes
    }
}
