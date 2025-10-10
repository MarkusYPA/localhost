use std::collections::HashMap;

#[derive(Debug)]
pub struct Request {
    pub method: String,
    pub path: String,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
    pub query_params: HashMap<String, String>,
    pub cookies: HashMap<String, String>,
}

impl<'a> From<&'a [u8]> for Request {
    fn from(buffer: &'a [u8]) -> Self {
        let mut request = Request {
            method: String::new(),
            path: String::new(),
            headers: HashMap::new(),
            body: Vec::new(),
            query_params: HashMap::new(),
            cookies: HashMap::new(),
        };

        let mut header_end = 0;
        for i in 0..buffer.len() - 3 {
            if buffer[i] == b'\r'
                && buffer[i + 1] == b'\n'
                && buffer[i + 2] == b'\r'
                && buffer[i + 3] == b'\n'
            {
                header_end = i + 4;
                break;
            }
        }

        let headers_str = std::str::from_utf8(&buffer[..header_end]).unwrap_or("");
        let mut lines = headers_str.lines();

        if let Some(request_line) = lines.next() {
            let mut parts = request_line.split_whitespace();
            request.method = parts.next().unwrap_or("").to_string();
            let full_path = parts.next().unwrap_or("").to_string();
            let mut path_parts = full_path.split('?');
            request.path = path_parts.next().unwrap_or("").to_string();
            if let Some(query) = path_parts.next() {
                for pair in query.split('&') {
                    let mut key_value = pair.split('=');
                    if let (Some(key), Some(value)) = (key_value.next(), key_value.next()) {
                        request
                            .query_params
                            .insert(key.to_string(), value.to_string());
                    }
                }
            }
        }

        for line in lines {
            if line.is_empty() {
                break;
            }
            let mut parts = line.splitn(2, ": ");
            if let (Some(key), Some(value)) = (parts.next(), parts.next()) {
                let header_key = key.to_string();
                let header_value = value.trim().to_string();
                if header_key == "Cookie" {
                    for cookie_pair in header_value.split(';') {
                        let mut cookie_parts = cookie_pair.trim().splitn(2, '=');
                        if let (Some(cookie_name), Some(cookie_value)) =
                            (cookie_parts.next(), cookie_parts.next())
                        {
                            request
                                .cookies
                                .insert(cookie_name.to_string(), cookie_value.to_string());
                        }
                    }
                }
                request.headers.insert(header_key, header_value);
            }
        }

        if let Some(content_length) = request.headers.get("Content-Length") {
            if let Ok(length) = content_length.parse::<usize>() {
                if header_end + length <= buffer.len() {
                    request.body = buffer[header_end..header_end + length].to_vec();
                }
            }
        }

        request
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_get_request() {
        let request_str = b"GET / HTTP/1.1\r\nHost: localhost\r\n\r\n";
        let request = Request::from(request_str.as_ref());
        assert_eq!(request.method, "GET");
        assert_eq!(request.path, "/");
        assert_eq!(request.headers.get("Host").unwrap(), "localhost");
    }

    #[test]
    fn test_get_request_with_query_params() {
        let request_str = b"GET /path?key1=value1&key2=value2 HTTP/1.1\r\nHost: localhost\r\n\r\n";
        let request = Request::from(request_str.as_ref());
        assert_eq!(request.method, "GET");
        assert_eq!(request.path, "/path");
        assert_eq!(request.query_params.get("key1").unwrap(), "value1");
        assert_eq!(request.query_params.get("key2").unwrap(), "value2");
    }

    #[test]
    fn test_post_request_with_body() {
        let request_str =
            b"POST /path HTTP/1.1\r\nHost: localhost\r\nContent-Length: 13\r\n\r\nHello, world!";
        let request = Request::from(request_str.as_ref());
        assert_eq!(request.method, "POST");
        assert_eq!(request.path, "/path");
        assert_eq!(request.headers.get("Content-Length").unwrap(), "13");
        assert_eq!(request.body, b"Hello, world!");
    }

    #[test]
    fn test_request_with_cookies() {
        let request_str =
            b"GET / HTTP/1.1\r\nHost: localhost\r\nCookie: key1=value1; key2=value2\r\n\r\n";
        let request = Request::from(request_str.as_ref());
        assert_eq!(request.cookies.get("key1").unwrap(), "value1");
        assert_eq!(request.cookies.get("key2").unwrap(), "value2");
    }
}
