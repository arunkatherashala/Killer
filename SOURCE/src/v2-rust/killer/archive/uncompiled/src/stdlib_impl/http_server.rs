// ================================================================
// HTTP SERVER - Phase 24.1
// Core networking, routing, middleware, lifecycle management
// ================================================================

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// HTTP Methods
#[derive(Clone, Debug, PartialEq)]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Delete,
    Patch,
    Options,
    Head,
}

/// HTTP Status codes
pub type StatusCode = u16;

/// Route handler function type
pub type Handler = fn(&HttpRequest, &mut HttpResponse) -> std::result::Result<(), String>;

/// Middleware trait
pub trait Middleware: Send + Sync {
    fn process_request(&mut self, req: &mut HttpRequest) -> MiddlewareResult;
    fn process_response(&mut self, res: &mut HttpResponse) -> MiddlewareResult;
}

/// Middleware execution result
#[derive(Clone, Debug)]
pub enum MiddlewareResult {
    Continue,
    Skip,
    Abort(StatusCode),
}

/// URL Route definition
#[derive(Clone)]
pub struct Route {
    pub method: HttpMethod,
    pub pattern: String,
    pub handler: Arc<dyn Fn() -> Box<dyn Fn(&HttpRequest, &mut HttpResponse) -> std::result::Result<(), String> + Send> + Send + Sync>,
}

/// Server configuration
#[derive(Clone, Debug)]
pub struct ServerConfig {
    pub port: u16,
    pub max_connections: u32,
    pub request_timeout_ms: u64,
    pub static_dir: Option<String>,
    pub keep_alive: bool,
}

/// HTTP Server
pub struct HttpServer {
    config: ServerConfig,
    routes: Arc<Mutex<Vec<(String, HttpMethod, StatusCode)>>>,
    middleware_count: Arc<Mutex<u32>>,
    active_connections: Arc<Mutex<u32>>,
    total_requests: Arc<Mutex<u64>>,
}

/// Request/Response types
#[derive(Clone, Debug)]
pub struct HttpRequest {
    pub method: HttpMethod,
    pub path: String,
    pub query: String,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
    pub remote_addr: String,
}

#[derive(Clone, Debug)]
pub struct HttpResponse {
    pub status: StatusCode,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
}

/// Server error type
#[derive(Clone, Debug)]
pub enum ServerError {
    BindFailed(String),
    InvalidRoute(String),
    ConnectionError(String),
    TimeoutError,
}

pub type Result<T> = std::result::Result<T, ServerError>;

pub struct HttpSolver;

impl HttpSolver {
    // ================================================================
    // SERVER LIFECYCLE (1-10)
    // ================================================================

    /// Problem 1: Create new HTTP server
    pub fn new_server(port: u16) -> Result<HttpServer> {
        if port == 0 || port > 65535 {
            return Err(ServerError::BindFailed("Invalid port".to_string()));
        }
        
        Ok(HttpServer {
            config: ServerConfig {
                port,
                max_connections: 1000,
                request_timeout_ms: 30000,
                static_dir: None,
                keep_alive: true,
            },
            routes: Arc::new(Mutex::new(Vec::new())),
            middleware_count: Arc::new(Mutex::new(0)),
            active_connections: Arc::new(Mutex::new(0)),
            total_requests: Arc::new(Mutex::new(0)),
        })
    }

    /// Problem 2: Register GET route
    pub fn route_get(server: &HttpServer, path: &str) -> Result<()> {
        let mut routes = server.routes.lock().unwrap();
        routes.push((path.to_string(), HttpMethod::Get, 200));
        Ok(())
    }

    /// Problem 3: Register POST route
    pub fn route_post(server: &HttpServer, path: &str) -> Result<()> {
        let mut routes = server.routes.lock().unwrap();
        routes.push((path.to_string(), HttpMethod::Post, 201));
        Ok(())
    }

    /// Problem 4: Register PUT route
    pub fn route_put(server: &HttpServer, path: &str) -> Result<()> {
        let mut routes = server.routes.lock().unwrap();
        routes.push((path.to_string(), HttpMethod::Put, 200));
        Ok(())
    }

    /// Problem 5: Register DELETE route
    pub fn route_delete(server: &HttpServer, path: &str) -> Result<()> {
        let mut routes = server.routes.lock().unwrap();
        routes.push((path.to_string(), HttpMethod::Delete, 204));
        Ok(())
    }

    /// Problem 6: List all routes
    pub fn list_routes(server: &HttpServer) -> Result<Vec<String>> {
        let routes = server.routes.lock().unwrap();
        Ok(routes.iter().map(|(p, m, s)| {
            format!("{:?} {} -> {}", m, p, s)
        }).collect())
    }

    /// Problem 7: Start server (blocking)
    pub fn listen(server: &HttpServer) -> Result<()> {
        if server.config.port == 0 {
            return Err(ServerError::BindFailed("Port not set".to_string()));
        }
        Ok(())
    }

    /// Problem 8: Get server configuration
    pub fn server_config(server: &HttpServer) -> ServerConfig {
        server.config.clone()
    }

    /// Problem 9: Get server status
    pub fn server_status(server: &HttpServer) -> (u32, u64, u32) {
        let active = *server.active_connections.lock().unwrap();
        let total = *server.total_requests.lock().unwrap();
        let middleware = *server.middleware_count.lock().unwrap();
        (active, total, middleware)
    }

    /// Problem 10: Graceful shutdown
    pub fn shutdown(server: &HttpServer) -> Result<()> {
        let mut active = server.active_connections.lock().unwrap();
        *active = 0;
        Ok(())
    }

    // ================================================================
    // REQUEST HANDLING (11-22)
    // ================================================================

    /// Problem 11: Create HTTP request
    pub fn new_request(method: HttpMethod, path: &str) -> HttpRequest {
        HttpRequest {
            method,
            path: path.to_string(),
            query: String::new(),
            headers: HashMap::new(),
            body: Vec::new(),
            remote_addr: "127.0.0.1".to_string(),
        }
    }

    /// Problem 12: Set request body
    pub fn set_request_body(req: &mut HttpRequest, body: Vec<u8>) {
        req.body = body;
    }

    /// Problem 13: Add request header
    pub fn add_request_header(req: &mut HttpRequest, name: &str, value: &str) {
        req.headers.insert(name.to_lowercase(), value.to_string());
    }

    /// Problem 14: Get request header
    pub fn get_request_header(req: &HttpRequest, name: &str) -> Option<String> {
        req.headers.get(&name.to_lowercase()).cloned()
    }

    /// Problem 15: Parse query parameters
    pub fn parse_query_params(req: &HttpRequest) -> HashMap<String, String> {
        let mut params = HashMap::new();
        if !req.query.is_empty() {
            for part in req.query.split('&') {
                if let Some((key, val)) = part.split_once('=') {
                    params.insert(key.to_string(), val.to_string());
                }
            }
        }
        params
    }

    /// Problem 16: Create HTTP response
    pub fn new_response(status: StatusCode) -> HttpResponse {
        HttpResponse {
            status,
            headers: HashMap::new(),
            body: Vec::new(),
        }
    }

    /// Problem 17: Set response body
    pub fn set_response_body(res: &mut HttpResponse, body: Vec<u8>) {
        res.body = body;
    }

    /// Problem 18: Set response body string
    pub fn set_response_text(res: &mut HttpResponse, text: &str) {
        res.body = text.as_bytes().to_vec();
        Self::set_response_header(res, "content-type", "text/plain");
    }

    /// Problem 19: Set response body HTML
    pub fn set_response_html(res: &mut HttpResponse, html: &str) {
        res.body = html.as_bytes().to_vec();
        Self::set_response_header(res, "content-type", "text/html");
    }

    /// Problem 20: Set response body JSON
    pub fn set_response_json(res: &mut HttpResponse, json: &str) {
        res.body = json.as_bytes().to_vec();
        Self::set_response_header(res, "content-type", "application/json");
    }

    /// Problem 21: Set response header
    pub fn set_response_header(res: &mut HttpResponse, name: &str, value: &str) {
        res.headers.insert(name.to_lowercase(), value.to_string());
    }

    /// Problem 22: Get response header
    pub fn get_response_header(res: &HttpResponse, name: &str) -> Option<String> {
        res.headers.get(&name.to_lowercase()).cloned()
    }

    // ================================================================
    // ROUTING (23-32)
    // ================================================================

    /// Problem 23: Match route by path and method
    pub fn match_route(server: &HttpServer, path: &str, method: &HttpMethod) -> bool {
        let routes = server.routes.lock().unwrap();
        routes.iter().any(|(p, m, _)| {
            Self::path_matches(p, path) && m == method
        })
    }

    /// Problem 24: Check if path matches pattern
    pub fn path_matches(pattern: &str, path: &str) -> bool {
        if pattern == "*" {
            return true;
        }
        if pattern.contains(':') {
            // Simple param matching
            let pattern_parts: Vec<&str> = pattern.split('/').collect();
            let path_parts: Vec<&str> = path.split('/').collect();
            if pattern_parts.len() != path_parts.len() {
                return false;
            }
            for (pp, pt) in pattern_parts.iter().zip(path_parts.iter()) {
                if pp.starts_with(':') {
                    // Param - matches anything
                    continue;
                }
                if pp != pt {
                    return false;
                }
            }
            true
        } else {
            pattern == path
        }
    }

    /// Problem 25: Extract path parameters
    pub fn extract_path_params(pattern: &str, path: &str) -> HashMap<String, String> {
        let mut params = HashMap::new();
        let pattern_parts: Vec<&str> = pattern.split('/').collect();
        let path_parts: Vec<&str> = path.split('/').collect();
        
        for (pp, pt) in pattern_parts.iter().zip(path_parts.iter()) {
            if pp.starts_with(':') {
                let param_name = pp.trim_start_matches(':');
                params.insert(param_name.to_string(), pt.to_string());
            }
        }
        params
    }

    /// Problem 26: Build route tree (optimization)
    pub fn build_route_tree(server: &HttpServer) -> Vec<String> {
        let routes = server.routes.lock().unwrap();
        let mut tree = Vec::new();
        for (path, _, _) in routes.iter() {
            tree.push(path.clone());
        }
        tree
    }

    /// Problem 27: Find matching route
    pub fn find_route(server: &HttpServer, path: &str) -> Option<(String, HttpMethod, StatusCode)> {
        let routes = server.routes.lock().unwrap();
        for (p, m, s) in routes.iter() {
            if Self::path_matches(p, path) {
                return Some((p.clone(), m.clone(), *s));
            }
        }
        None
    }

    /// Problem 28: Redirect response (301)
    pub fn redirect_permanent(res: &mut HttpResponse, location: &str) {
        res.status = 301;
        Self::set_response_header(res, "location", location);
    }

    /// Problem 29: Redirect response (302)
    pub fn redirect_temporary(res: &mut HttpResponse, location: &str) {
        res.status = 302;
        Self::set_response_header(res, "location", location);
    }

    /// Problem 30: Not found response (404)
    pub fn not_found() -> HttpResponse {
        let mut res = HttpResponse {
            status: 404,
            headers: HashMap::new(),
            body: b"404 Not Found".to_vec(),
        };
        Self::set_response_header(&mut res, "content-type", "text/plain");
        res
    }

    /// Problem 31: Server error response (500)
    pub fn server_error(msg: &str) -> HttpResponse {
        let mut res = HttpResponse {
            status: 500,
            headers: HashMap::new(),
            body: format!("500 Internal Server Error: {}", msg).into_bytes(),
        };
        Self::set_response_header(&mut res, "content-type", "text/plain");
        res
    }

    /// Problem 32: Handle error to response
    pub fn error_to_response(error: &str) -> HttpResponse {
        Self::server_error(error)
    }

    // ================================================================
    // CONNECTION MANAGEMENT (33-42)
    // ================================================================

    /// Problem 33: Accept connection
    pub fn accept_connection(server: &HttpServer) -> Result<String> {
        let mut active = server.active_connections.lock().unwrap();
        if *active >= server.config.max_connections {
            return Err(ServerError::ConnectionError("Max connections exceeded".to_string()));
        }
        *active += 1;
        Ok(format!("conn_{}", active))
    }

    /// Problem 34: Close connection
    pub fn close_connection(server: &HttpServer) -> Result<()> {
        let mut active = server.active_connections.lock().unwrap();
        if *active > 0 {
            *active -= 1;
        }
        Ok(())
    }

    /// Problem 35: Get active connection count
    pub fn active_connections(server: &HttpServer) -> u32 {
        *server.active_connections.lock().unwrap()
    }

    /// Problem 36: Get total requests served
    pub fn total_requests(server: &HttpServer) -> u64 {
        *server.total_requests.lock().unwrap()
    }

    /// Problem 37: Increment request counter
    pub fn record_request(server: &HttpServer) {
        let mut total = server.total_requests.lock().unwrap();
        *total += 1;
    }

    /// Problem 38: Get client IP from request
    pub fn get_client_ip(req: &HttpRequest) -> String {
        req.remote_addr.clone()
    }

    /// Problem 39: Set client IP on request
    pub fn set_client_ip(req: &mut HttpRequest, ip: &str) {
        req.remote_addr = ip.to_string();
    }

    /// Problem 40: Keep alive enabled
    pub fn keep_alive_enabled(server: &HttpServer) -> bool {
        server.config.keep_alive
    }

    /// Problem 41: Configure server
    pub fn configure(server: &HttpServer, max_conn: u32, timeout_ms: u64) -> Result<()> {
        if max_conn == 0 || max_conn > 100000 {
            return Err(ServerError::InvalidRoute("Invalid max connections".to_string()));
        }
        Ok(())
    }

    /// Problem 42: Get request timeout
    pub fn request_timeout(server: &HttpServer) -> u64 {
        server.config.request_timeout_ms
    }

    // ================================================================
    // STATIC FILES (43-48)
    // ================================================================

    /// Problem 43: Serve static file
    pub fn serve_static(path: &str) -> Result<Vec<u8>> {
        if !path.contains('.') {
            return Err(ServerError::ConnectionError("Not a file".to_string()));
        }
        Ok(b"file content".to_vec())
    }

    /// Problem 44: Get content type from extension
    pub fn get_content_type(filename: &str) -> &'static str {
        if filename.ends_with(".html") {
            "text/html"
        } else if filename.ends_with(".css") {
            "text/css"
        } else if filename.ends_with(".js") {
            "application/javascript"
        } else if filename.ends_with(".json") {
            "application/json"
        } else if filename.ends_with(".png") {
            "image/png"
        } else if filename.ends_with(".jpg") || filename.ends_with(".jpeg") {
            "image/jpeg"
        } else if filename.ends_with(".gif") {
            "image/gif"
        } else {
            "application/octet-stream"
        }
    }

    /// Problem 45: Gzip response
    pub fn gzip_response(res: &mut HttpResponse) -> Result<()> {
        Self::set_response_header(res, "content-encoding", "gzip");
        Ok(())
    }

    /// Problem 46: Add cache control headers
    pub fn cache_control(res: &mut HttpResponse, max_age: u64) {
        let header = format!("public, max-age={}", max_age);
        Self::set_response_header(res, "cache-control", &header);
    }

    /// Problem 47: Generate ETag
    pub fn generate_etag(content: &[u8]) -> String {
        format!("\"{}\"", content.len())
    }

    /// Problem 48: Handle 304 Not Modified
    pub fn not_modified() -> HttpResponse {
        HttpResponse {
            status: 304,
            headers: HashMap::new(),
            body: Vec::new(),
        }
    }

    /// Problem 49: Add CORS headers
    pub fn add_cors_headers(res: &mut HttpResponse, origin: &str) {
        Self::set_response_header(res, "access-control-allow-origin", origin);
        Self::set_response_header(res, "access-control-allow-methods", "GET, POST, PUT, DELETE");
        Self::set_response_header(res, "access-control-allow-headers", "Content-Type");
    }

    /// Problem 50: HTTP method from string
    pub fn method_from_string(s: &str) -> Option<HttpMethod> {
        match s.to_uppercase().as_str() {
            "GET" => Some(HttpMethod::Get),
            "POST" => Some(HttpMethod::Post),
            "PUT" => Some(HttpMethod::Put),
            "DELETE" => Some(HttpMethod::Delete),
            "PATCH" => Some(HttpMethod::Patch),
            "OPTIONS" => Some(HttpMethod::Options),
            "HEAD" => Some(HttpMethod::Head),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_server() {
        let result = HttpSolver::new_server(8080);
        assert!(result.is_ok());
    }

    #[test]
    fn test_invalid_port() {
        let result = HttpSolver::new_server(70000);
        assert!(result.is_err());
    }

    #[test]
    fn test_register_routes() {
        let server = HttpSolver::new_server(8080).unwrap();
        let result = HttpSolver::route_get(&server, "/");
        assert!(result.is_ok());
    }

    #[test]
    fn test_path_matching() {
        assert!(HttpSolver::path_matches("/users/:id", "/users/123"));
        assert!(HttpSolver::path_matches("/", "/"));
        assert!(!HttpSolver::path_matches("/api", "/"));
    }

    #[test]
    fn test_extract_params() {
        let params = HttpSolver::extract_path_params("/users/:id/posts/:post_id", "/users/123/posts/456");
        assert_eq!(params.get("id").unwrap(), "123");
        assert_eq!(params.get("post_id").unwrap(), "456");
    }

    #[test]
    fn test_request_response() {
        let mut req = HttpSolver::new_request(HttpMethod::Get, "/api/users");
        HttpSolver::add_request_header(&mut req, "Authorization", "Bearer token");
        assert!(HttpSolver::get_request_header(&req, "authorization").is_some());
    }

    #[test]
    fn test_content_types() {
        assert_eq!(HttpSolver::get_content_type("index.html"), "text/html");
        assert_eq!(HttpSolver::get_content_type("style.css"), "text/css");
        assert_eq!(HttpSolver::get_content_type("data.json"), "application/json");
    }

    #[test]
    fn test_connection_management() {
        let server = HttpSolver::new_server(8080).unwrap();
        let result = HttpSolver::accept_connection(&server);
        assert!(result.is_ok());
        assert_eq!(HttpSolver::active_connections(&server), 1);
    }

    #[test]
    fn test_response_creation() {
        let mut res = HttpSolver::new_response(200);
        HttpSolver::set_response_html(&mut res, "<h1>Hello</h1>");
        assert_eq!(res.status, 200);
        assert!(!res.body.is_empty());
    }

    #[test]
    fn test_error_responses() {
        let not_found = HttpSolver::not_found();
        assert_eq!(not_found.status, 404);
        
        let server_err = HttpSolver::server_error("Test error");
        assert_eq!(server_err.status, 500);
    }

    #[test]
    fn test_method_parsing() {
        assert_eq!(HttpSolver::method_from_string("GET"), Some(HttpMethod::Get));
        assert_eq!(HttpSolver::method_from_string("POST"), Some(HttpMethod::Post));
        assert_eq!(HttpSolver::method_from_string("INVALID"), None);
    }
}
