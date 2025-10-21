use std::collections::HashMap;

#[derive(Debug)]
pub struct Response {
    pub status_code: u16,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
}

impl Response {
    pub fn new(status_code: u16, body: Vec<u8>) -> Self {
        let mut headers = HashMap::new();
        headers.insert("Content-Length".to_string(), body.len().to_string());
        headers.insert("Connection".to_string(), "keep-alive".to_string());

        Response {
            status_code,
            headers,
            body,
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let status_line = format!(
            "HTTP/1.1 {} {}\r\n",
            self.status_code,
            crate::http::status::reason_phrase(self.status_code)
        );
        let mut headers = String::new();
        for (key, value) in &self.headers {
            headers.push_str(&format!("{key}: {value}\r\n"));
        }

        let mut response = Vec::new();
        response.extend_from_slice(status_line.as_bytes());
        response.extend_from_slice(headers.as_bytes());
        response.extend_from_slice(b"\r\n");
        response.extend_from_slice(&self.body);

        response
    }
}
