// Killer Language HTTP Server Bindings
// Adds http:: namespace with server, route, request, response functions

use crate::http_server::HttpServer;
use crate::web_framework::{HttpRequest, HttpResponse, StatusCode};
use crate::value::Value;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// HTTP server instance wrapper for Killer scripts
pub struct KillerHttpServer {
    pub server: Arc<Mutex<HttpServer>>,
}

impl KillerHttpServer {
    pub fn new(host: &str, port: u16) -> Self {
        KillerHttpServer {
            server: Arc::new(Mutex::new(HttpServer::new(host, port))),
        }
    }

    /// Start listening on the configured address
    pub fn listen(&self) -> Result<String, String> {
        let mut server = self.server.lock().unwrap();
        server.listen()?;
        Ok(format!("Listening on {}:{}", "127.0.0.1", 8080))
    }

    /// Handle a single connection
    pub fn accept_one(&self) -> Result<String, String> {
        let mut server = self.server.lock().unwrap();
        server.accept_one()?;
        Ok("Connection handled".to_string())
    }

    /// Register a GET route
    pub fn get(&self, path: &str) -> Result<(), String> {
        let server = self.server.lock().unwrap();
        server.on_route("GET", path, |req: &HttpRequest| {
            HttpResponse::new(StatusCode::OK)
                .set_body(format!("GET {}", req.path))
        })
    }

    /// Register a POST route
    pub fn post(&self, path: &str) -> Result<(), String> {
        let server = self.server.lock().unwrap();
        server.on_route("POST", path, |req: &HttpRequest| {
            HttpResponse::new(StatusCode::OK)
                .set_body(format!("POST {}\nBody: {}", req.path, req.body))
        })
    }
}

/// Test helpers for HTTP server
pub mod test_helpers {
    use super::*;

    pub fn create_test_server() -> KillerHttpServer {
        KillerHttpServer::new("127.0.0.1", 8080)
    }

    pub fn test_get_route() -> Result<(), String> {
        let server = create_test_server();
        server.get("/")?;
        Ok(())
    }

    pub fn test_post_route() -> Result<(), String> {
        let server = create_test_server();
        server.post("/api/data")?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_http_server_creation() {
        let server = KillerHttpServer::new("127.0.0.1", 8080);
        // Should not panic
        assert!(server.server.is_poisoned() == false);
    }

    #[test]
    fn test_http_server_get_route() {
        let server = KillerHttpServer::new("127.0.0.1", 8080);
        assert!(server.get("/").is_ok());
    }

    #[test]
    fn test_http_server_post_route() {
        let server = KillerHttpServer::new("127.0.0.1", 8080);
        assert!(server.post("/api/users").is_ok());
    }
}
