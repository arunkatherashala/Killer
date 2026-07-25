// HTTP Server Implementation - Real TCP/HTTP networking
// Bridges web_framework types with actual TCP sockets and HTTP protocol handling

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use crate::web_framework::{HttpRequest, HttpResponse, HttpMethod, Router, StatusCode};

/// Parse HTTP request from raw bytes
pub fn parse_http_request(raw_request: &str) -> Result<HttpRequest, String> {
    let mut lines = raw_request.lines();

    // Parse request line (GET /path HTTP/1.1)
    let request_line = lines
        .next()
        .ok_or("Empty request")?;

    let parts: Vec<&str> = request_line.split_whitespace().collect();
    if parts.len() < 3 {
        return Err("Invalid request line: missing method, path, or version".to_string());
    }

    // Validate HTTP method
    let method_str = parts[0];
    const VALID_METHODS: &[&str] = &["GET", "POST", "PUT", "DELETE", "PATCH", "OPTIONS", "HEAD"];
    let method_upper = method_str.to_uppercase();
    if !VALID_METHODS.contains(&method_upper.as_str()) && !method_upper.starts_with("X-") {
        return Err(format!("Invalid HTTP method: {}", method_str));
    }

    let path = parts[1];

    // Validate path starts with /
    if !path.starts_with('/') {
        return Err("Invalid request path: must start with /".to_string());
    }

    // Validate HTTP version format
    let version = parts[2];
    if !version.contains("HTTP/") {
        return Err("Invalid HTTP version format".to_string());
    }

    // Parse path and query parameters
    let (path_only, query_params) = if let Some(idx) = path.find('?') {
        let p = &path[..idx];
        let qs = &path[idx + 1..];
        let mut params = HashMap::new();
        for param in qs.split('&') {
            if let Some(eq_idx) = param.find('=') {
                let key = param[..eq_idx].to_string();
                let val = param[eq_idx + 1..].to_string();
                params.insert(key, val);
            }
        }
        (p.to_string(), params)
    } else {
        (path.to_string(), HashMap::new())
    };

    let mut request = HttpRequest::new(HttpMethod::from_string(method_str), path_only);

    // Add query params
    for (key, value) in query_params {
        request = request.add_query_param(key, value);
    }

    // Parse headers
    let mut _body_start = 0;
    let mut _header_count = 0;
    for (idx, line) in lines.by_ref().enumerate() {
        if line.is_empty() {
            _body_start = idx;
            break;
        }
        _header_count += 1;

        // Parse header (Key: Value)
        if let Some(colon_idx) = line.find(':') {
            let key = line[..colon_idx].trim().to_string();
            let value = line[colon_idx + 1..].trim().to_string();
            request = request.add_header(key, value);
        }
    }

    // Parse body (for POST requests)
    let remaining: String = lines.collect::<Vec<_>>().join("\n");
    if !remaining.is_empty() {
        request = request.set_body(remaining);
    }

    Ok(request)
}

/// HTTP Server with actual TCP networking
#[allow(dead_code)]
pub struct HttpServer {
    host: String,
    port: u16,
    listener: Option<TcpListener>,
    router: Arc<Mutex<Router>>,
    request_handlers: Arc<Mutex<HashMap<String, Box<dyn Fn(&HttpRequest) -> HttpResponse + Send>>>>,
    max_connections: usize,
}

impl HttpServer {
    /// Create a new HTTP server
    pub fn new(host: &str, port: u16) -> Self {
        HttpServer {
            host: host.to_string(),
            port,
            listener: None,
            router: Arc::new(Mutex::new(Router::new())),
            request_handlers: Arc::new(Mutex::new(HashMap::new())),
            max_connections: 100,
        }
    }

    /// Register a route handler
    pub fn on_route<F>(&self, method: &str, path: &str, handler: F) -> Result<(), String>
    where
        F: Fn(&HttpRequest) -> HttpResponse + Send + 'static,
    {
        let method_enum = match method.to_uppercase().as_str() {
            "GET" => HttpMethod::GET,
            "POST" => HttpMethod::POST,
            "PUT" => HttpMethod::PUT,
            "DELETE" => HttpMethod::DELETE,
            "PATCH" => HttpMethod::PATCH,
            "OPTIONS" => HttpMethod::OPTIONS,
            "HEAD" => HttpMethod::HEAD,
            _ => return Err("Unknown HTTP method".to_string()),
        };

        let handler_key = format!("{}:{}", method, path);

        // Register in router
        {
            let mut router = self.router.lock().unwrap();
            router.register(method_enum, path.to_string(), handler_key.clone());
        }

        // Store handler function
        {
            let mut handlers = self.request_handlers.lock().unwrap();
            handlers.insert(handler_key, Box::new(handler));
        }

        Ok(())
    }

    /// Start listening on the configured host:port
    pub fn listen(&mut self) -> Result<(), String> {
        let addr = format!("{}:{}", self.host, self.port);
        let listener = TcpListener::bind(&addr)
            .map_err(|e| format!("Failed to bind to {}: {}", addr, e))?;
        
        self.listener = Some(listener);
        Ok(())
    }

    /// Accept a single connection and handle it
    pub fn accept_one(&mut self) -> Result<(), String> {
        if let Some(ref listener) = self.listener {
            match listener.accept() {
                Ok((stream, addr)) => {
                    eprintln!("Connection from: {}", addr);
                    self.handle_connection(stream)?;
                    Ok(())
                }
                Err(e) => Err(format!("Failed to accept connection: {}", e)),
            }
        } else {
            Err("Server not listening".to_string())
        }
    }

    /// Handle a single TCP connection
    fn handle_connection(&self, mut stream: TcpStream) -> Result<(), String> {
        // Read request from socket
        let mut buffer = [0; 8192];
        let bytes_read = stream
            .read(&mut buffer)
            .map_err(|e| format!("Failed to read from socket: {}", e))?;

        if bytes_read == 0 {
            return Ok(()); // Connection closed
        }

        let request_str = String::from_utf8_lossy(&buffer[..bytes_read]).to_string();

        // Parse HTTP request
        let request = match parse_http_request(&request_str) {
            Ok(req) => req,
            Err(e) => {
                eprintln!("Failed to parse request: {}", e);
                let response = HttpResponse::new(StatusCode::BadRequest)
                    .set_body("Bad Request".to_string());
                self.write_response(&mut stream, &response)?;
                return Ok(());
            }
        };

        // Route the request
        let response = self.route_request(&request);

        // Write response to socket
        self.write_response(&mut stream, &response)?;

        Ok(())
    }

    /// Route HTTP request to appropriate handler
    fn route_request(&self, request: &HttpRequest) -> HttpResponse {
        let router = self.router.lock().unwrap();
        
        // Try to match route
        if let Some(route) = router.match_route(&request.method, &request.path) {
            let handler_key = &route.handler;
            let handlers = self.request_handlers.lock().unwrap();
            
            if let Some(handler) = handlers.get(handler_key) {
                return handler(request);
            }
        }

        // Route not found
        HttpResponse::new(StatusCode::NotFound)
            .set_body(format!("Not Found: {} {}", 
                match &request.method {
                    HttpMethod::GET => "GET",
                    HttpMethod::POST => "POST",
                    HttpMethod::PUT => "PUT",
                    HttpMethod::DELETE => "DELETE",
                    HttpMethod::PATCH => "PATCH",
                    HttpMethod::OPTIONS => "OPTIONS",
                    HttpMethod::HEAD => "HEAD",
                },
                request.path))
    }

    /// Write HTTP response to socket
    fn write_response(&self, stream: &mut TcpStream, response: &HttpResponse) -> Result<(), String> {
        let response_text = response.format();
        stream
            .write_all(response_text.as_bytes())
            .map_err(|e| format!("Failed to write response: {}", e))?;
        
        stream
            .flush()
            .map_err(|e| format!("Failed to flush response: {}", e))?;

        Ok(())
    }

    /// Run server in a loop (blocking)
    pub fn run(&mut self) -> Result<(), String> {
        self.listen()?;
        eprintln!("Server listening on {}:{}", self.host, self.port);

        if let Some(ref listener) = self.listener {
            loop {
                match listener.accept() {
                    Ok((stream, addr)) => {
                        eprintln!("Connection from: {}", addr);
                        
                        // Clone data for thread
                        let router = Arc::clone(&self.router);
                        let handlers = Arc::clone(&self.request_handlers);

                        // Handle in new thread
                        thread::spawn(move || {
                            if let Err(e) = Self::handle_connection_static(stream, &router, &handlers) {
                                eprintln!("Connection error: {}", e);
                            }
                        });
                    }
                    Err(e) => {
                        eprintln!("Failed to accept connection: {}", e);
                    }
                }
            }
        } else {
            Err("Internal error: listener not initialized".to_string())
        }
    }

    /// Static connection handler for threading
    fn handle_connection_static(
        mut stream: TcpStream,
        router: &Arc<Mutex<Router>>,
        handlers: &Arc<Mutex<HashMap<String, Box<dyn Fn(&HttpRequest) -> HttpResponse + Send>>>>,
    ) -> Result<(), String> {
        // Read request
        let mut buffer = [0; 8192];
        let bytes_read = stream
            .read(&mut buffer)
            .map_err(|e| format!("Read error: {}", e))?;

        if bytes_read == 0 {
            return Ok(());
        }

        let request_str = String::from_utf8_lossy(&buffer[..bytes_read]).to_string();

        // Parse request
        let request = match parse_http_request(&request_str) {
            Ok(req) => req,
            Err(_) => {
                let response = HttpResponse::new(StatusCode::BadRequest)
                    .set_body("Bad Request".to_string());
                let _ = stream.write_all(response.format().as_bytes());
                let _ = stream.flush();
                return Ok(());
            }
        };

        // Route request
        let router_guard = router.lock().unwrap();
        let response = if let Some(route) = router_guard.match_route(&request.method, &request.path) {
            let handler_key = &route.handler;
            let handlers_guard = handlers.lock().unwrap();
            
            if let Some(handler) = handlers_guard.get(handler_key) {
                handler(&request)
            } else {
                HttpResponse::new(StatusCode::InternalError)
                    .set_body("Handler not found".to_string())
            }
        } else {
            HttpResponse::new(StatusCode::NotFound)
                .set_body(format!("Not Found: {}", request.path))
        };

        drop(router_guard); // Release lock before writing

        // Write response
        stream
            .write_all(response.format().as_bytes())
            .map_err(|e| format!("Write error: {}", e))?;
        stream
            .flush()
            .map_err(|e| format!("Flush error: {}", e))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_request() {
        let raw = "GET / HTTP/1.1\r\nHost: localhost\r\n\r\n";
        let req = parse_http_request(raw).unwrap();
        assert_eq!(req.method, HttpMethod::GET);
        assert_eq!(req.path, "/");
        assert_eq!(req.get_header("Host"), Some("localhost".to_string()));
    }

    #[test]
    fn test_parse_post_request() {
        let raw = "POST /api/users HTTP/1.1\r\nHost: localhost\r\nContent-Length: 13\r\n\r\n{\"name\":\"test\"}";
        let req = parse_http_request(raw).unwrap();
        assert_eq!(req.method, HttpMethod::POST);
        assert_eq!(req.path, "/api/users");
        assert_eq!(req.body, "{\"name\":\"test\"}");
    }

    #[test]
    fn test_parse_with_query_params() {
        let raw = "GET /api/users?id=123&name=test HTTP/1.1\r\n\r\n";
        let req = parse_http_request(raw).unwrap();
        assert_eq!(req.path, "/api/users");
        assert_eq!(req.get_query_param("id"), Some("123".to_string()));
        assert_eq!(req.get_query_param("name"), Some("test".to_string()));
    }

    #[test]
    fn test_http_server_creation() {
        let server = HttpServer::new("127.0.0.1", 8080);
        assert_eq!(server.host, "127.0.0.1");
        assert_eq!(server.port, 8080);
    }

    #[test]
    fn test_response_formatting() {
        let response = HttpResponse::new(StatusCode::OK)
            .set_body("Hello World".to_string());
        let formatted = response.format();
        assert!(formatted.contains("200 OK"));
        assert!(formatted.contains("Hello World"));
    }
}
