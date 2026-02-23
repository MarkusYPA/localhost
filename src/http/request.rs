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

// Helper to find end of headers
fn find_header_end(buffer: &[u8]) -> Option<usize> {
    buffer.windows(4).position(|window| window == b"\r\n\r\n")
}

// Helper to parse chunked body
fn parse_chunked_body(body_buffer: &[u8]) -> Result<Option<(Vec<u8>, usize)>, &str> {
    let mut dechunked_body = Vec::new();
    let mut cursor = 0;

    loop {
        // Find the end of the chunk size line
        if let Some(i) = body_buffer[cursor..].windows(2).position(|w| w == b"\r\n") {
            let size_line_end = cursor + i;
            let size_str = std::str::from_utf8(&body_buffer[cursor..size_line_end])
                .map_err(|_| "Invalid UTF-8 in chunk size")?;
            let chunk_size =
                usize::from_str_radix(size_str, 16).map_err(|_| "Invalid chunk size")?;

            cursor = size_line_end + 2; // Move past \r\n

            if chunk_size == 0 {
                // End of chunks
                if body_buffer.len() >= cursor + 2 && &body_buffer[cursor..cursor + 2] == b"\r\n" {
                    return Ok(Some((dechunked_body, cursor + 2)));
                } else {
                    return Ok(None); // Incomplete final chunk terminator
                }
            }

            let chunk_end = cursor + chunk_size;
            if body_buffer.len() < chunk_end + 2 {
                return Ok(None); // Incomplete chunk data
            }

            dechunked_body.extend_from_slice(&body_buffer[cursor..chunk_end]);

            // Move cursor past chunk data and trailing \r\n
            cursor = chunk_end + 2;
        } else {
            return Ok(None); // Incomplete chunk size line
        }
    }
}

pub fn parse_request_from_buffer(buffer: &[u8]) -> Result<Option<(Request, usize)>, &str> {
    let header_end = match find_header_end(buffer) {
        Some(pos) => pos + 4,
        None => return Ok(None), // Incomplete headers
    };

    let headers_str =
        std::str::from_utf8(&buffer[..header_end]).map_err(|_| "Invalid UTF-8 in headers")?;
    let mut lines = headers_str.lines();

    let mut request = Request {
        method: String::new(),
        path: String::new(),
        headers: HashMap::new(),
        body: Vec::new(),
        query_params: HashMap::new(),
        cookies: HashMap::new(),
    };

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
    } else {
        return Err("Empty request");
    }

    for line in lines {
        if line.is_empty() {
            break;
        }
        let mut parts = line.splitn(2, ": ");
        if let (Some(key), Some(value)) = (parts.next(), parts.next()) {
            let header_key = key.to_lowercase();
            let header_value = value.trim().to_string();
            if header_key == "cookie" {
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

    let body_buffer = &buffer[header_end..];
    let consumed: usize;

    if request
        .headers
        .get("transfer-encoding")
        .is_some_and(|v| v.eq_ignore_ascii_case("chunked"))
    {
        match parse_chunked_body(body_buffer)? {
            Some((body, body_len)) => {
                request.body = body;
                consumed = header_end + body_len;
            }
            None => return Ok(None), // Incomplete chunked body
        }
    } else if let Some(content_length_str) = request.headers.get("content-length") {
        let content_length = content_length_str
            .parse::<usize>()
            .map_err(|_| "Invalid Content-Length")?;
        if body_buffer.len() >= content_length {
            request.body = body_buffer[..content_length].to_vec();
            consumed = header_end + content_length;
        } else {
            return Ok(None); // Incomplete body
        }
    } else {
        // No body
        consumed = header_end;
    }

    Ok(Some((request, consumed)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_get_request() {
        let request_str = b"GET / HTTP/1.1\r\nHost: localhost\r\n\r\n";
        let (request, consumed) = parse_request_from_buffer(request_str).unwrap().unwrap();
        assert_eq!(request.method, "GET");
        assert_eq!(request.path, "/");
        assert_eq!(request.headers.get("host").unwrap(), "localhost");
        assert_eq!(consumed, request_str.len());
    }

    #[test]
    fn test_get_request_with_query_params() {
        let request_str = b"GET /path?key1=value1&key2=value2 HTTP/1.1\r\nHost: localhost\r\n\r\n";
        let (request, consumed) = parse_request_from_buffer(request_str).unwrap().unwrap();
        assert_eq!(request.method, "GET");
        assert_eq!(request.path, "/path");
        assert_eq!(request.query_params.get("key1").unwrap(), "value1");
        assert_eq!(request.query_params.get("key2").unwrap(), "value2");
        assert_eq!(consumed, request_str.len());
    }

    #[test]
    fn test_post_request_with_body() {
        let request_str =
            b"POST /path HTTP/1.1\r\nHost: localhost\r\nContent-Length: 13\r\n\r\nHello, world!";
        let (request, consumed) = parse_request_from_buffer(request_str).unwrap().unwrap();
        assert_eq!(request.method, "POST");
        assert_eq!(request.path, "/path");
        assert_eq!(request.headers.get("content-length").unwrap(), "13");
        assert_eq!(request.body, b"Hello, world!");
        assert_eq!(consumed, request_str.len());
    }

    #[test]
    fn test_request_with_cookies() {
        let request_str =
            b"GET / HTTP/1.1\r\nHost: localhost\r\nCookie: key1=value1; key2=value2\r\n\r\n";
        let (request, _consumed) = parse_request_from_buffer(request_str).unwrap().unwrap();
        assert_eq!(request.cookies.get("key1").unwrap(), "value1");
        assert_eq!(request.cookies.get("key2").unwrap(), "value2");
    }

    #[test]
    fn test_incomplete_request() {
        let request_str = b"GET / HTTP/1.1\r\nHost: localhost"; // Missing \r\n\r\n
        let result = parse_request_from_buffer(request_str).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_post_incomplete_body() {
        let request_str =
            b"POST /path HTTP/1.1\r\nHost: localhost\r\nContent-Length: 13\r\n\r\nHello";
        let result = parse_request_from_buffer(request_str).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_chunked_request() {
        let request_str = b"POST /chunked HTTP/1.1\r\nHost: localhost\r\nTransfer-Encoding: chunked\r\n\r\n7\r\nMozilla\r\n9\r\nDeveloper\r\n7\r\nNetwork\r\n0\r\n\r\n";
        let (request, consumed) = parse_request_from_buffer(request_str).unwrap().unwrap();
        assert_eq!(request.body, b"MozillaDeveloperNetwork");
        assert_eq!(consumed, request_str.len());
    }

    #[test]
    fn test_chunked_incomplete() {
        let request_str = b"POST /chunked HTTP/1.1\r\nHost: localhost\r\nTransfer-Encoding: chunked\r\n\r\n7\r\nMozilla\r\n";
        let result = parse_request_from_buffer(request_str).unwrap();
        assert!(result.is_none());
    }
}
