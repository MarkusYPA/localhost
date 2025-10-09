pub const HTTP_OK: u16 = 200;
pub const HTTP_CREATED: u16 = 201;
pub const HTTP_NO_CONTENT: u16 = 204;
pub const HTTP_BAD_REQUEST: u16 = 400;
pub const HTTP_FORBIDDEN: u16 = 403;
pub const HTTP_NOT_FOUND: u16 = 404;
pub const HTTP_METHOD_NOT_ALLOWED: u16 = 405;
pub const HTTP_PAYLOAD_TOO_LARGE: u16 = 413;
pub const HTTP_INTERNAL_SERVER_ERROR: u16 = 500;

pub fn reason_phrase(status_code: u16) -> &'static str {
    match status_code {
        200 => "OK",
        201 => "Created",
        204 => "No Content",
        400 => "Bad Request",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        413 => "Payload Too Large",
        500 => "Internal Server Error",
        501 => "Not Implemented",
        _ => "Unknown",
    }
}
