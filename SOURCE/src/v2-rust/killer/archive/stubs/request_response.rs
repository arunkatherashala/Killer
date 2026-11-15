//! HTTP Request/Response — Low-level HTTP primitives

#[derive(Debug)]
pub struct HttpRequest { pub method: String, pub path: String }

#[derive(Debug)]
pub struct HttpResponse { pub status: u16, pub body: String }

impl HttpResponse {
    pub fn new(status: u16) -> Self { Self { status, body: String::new() } }
}
