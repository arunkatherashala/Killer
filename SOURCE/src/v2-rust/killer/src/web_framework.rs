// Phase 8: Web Framework Module - HTTP server, routing, middleware, request/response handling
// Features: HTTP server, routing, middleware, static files, JSON API support, CORS

use std::collections::HashMap;
#[allow(unused_imports)]
use crate::value::Value;

/// HTTP methods
#[derive(Clone, Debug, PartialEq)]
pub enum HttpMethod {
    GET,
    POST,
    PUT,
    DELETE,
    PATCH,
    OPTIONS,
    HEAD,
}

impl HttpMethod {
    pub fn from_string(method: &str) -> Self {
        match method.to_uppercase().as_str() {
            "GET" => HttpMethod::GET,
            "POST" => HttpMethod::POST,
            "PUT" => HttpMethod::PUT,
            "DELETE" => HttpMethod::DELETE,
            "PATCH" => HttpMethod::PATCH,
            "OPTIONS" => HttpMethod::OPTIONS,
            "HEAD" => HttpMethod::HEAD,
            _ => HttpMethod::GET,
        }
    }
}

/// HTTP status codes
#[derive(Clone, Debug)]
pub enum StatusCode {
    OK,             // 200
    Created,        // 201
    BadRequest,     // 400
    Unauthorized,   // 401
    Forbidden,      // 403
    NotFound,       // 404
    InternalError,  // 500
    ServiceUnavailable, // 503
}

impl StatusCode {
    pub fn code(&self) -> u16 {
        match self {
            StatusCode::OK => 200,
            StatusCode::Created => 201,
            StatusCode::BadRequest => 400,
            StatusCode::Unauthorized => 401,
            StatusCode::Forbidden => 403,
            StatusCode::NotFound => 404,
            StatusCode::InternalError => 500,
            StatusCode::ServiceUnavailable => 503,
        }
    }

    pub fn message(&self) -> &str {
        match self {
            StatusCode::OK => "OK",
            StatusCode::Created => "Created",
            StatusCode::BadRequest => "Bad Request",
            StatusCode::Unauthorized => "Unauthorized",
            StatusCode::Forbidden => "Forbidden",
            StatusCode::NotFound => "Not Found",
            StatusCode::InternalError => "Internal Server Error",
            StatusCode::ServiceUnavailable => "Service Unavailable",
        }
    }
}

/// HTTP request
#[derive(Clone, Debug)]
pub struct HttpRequest {
    pub method: HttpMethod,
    pub path: String,
    pub headers: HashMap<String, String>,
    pub body: String,
    pub query_params: HashMap<String, String>,
}

impl HttpRequest {
    pub fn new(method: HttpMethod, path: String) -> Self {
        HttpRequest {
            method,
            path,
            headers: HashMap::new(),
            body: String::new(),
            query_params: HashMap::new(),
        }
    }

    /// Add header
    pub fn add_header(mut self, key: String, value: String) -> Self {
        self.headers.insert(key, value);
        self
    }

    /// Set body
    pub fn set_body(mut self, body: String) -> Self {
        self.body = body;
        self
    }

    /// Add query parameter
    pub fn add_query_param(mut self, key: String, value: String) -> Self {
        self.query_params.insert(key, value);
        self
    }

    /// Get header
    pub fn get_header(&self, key: &str) -> Option<String> {
        self.headers.get(key).cloned()
    }

    /// Get query param
    pub fn get_query_param(&self, key: &str) -> Option<String> {
        self.query_params.get(key).cloned()
    }

    /// Check if request has JSON content type
    pub fn has_json_content_type(&self) -> bool {
        if let Some(content_type) = self.get_header("Content-Type") {
            content_type.contains("application/json")
        } else {
            false
        }
    }

    /// Parse JSON body using json_module
    /// Returns Err if body is not valid JSON
    pub fn parse_json(&self) -> Result<crate::json_module::JsonValue, String> {
        crate::json_module::JsonModule::parse(&self.body)
            .map_err(|e| format!("JSON parse error: {}", e))
    }

    /// Check if request body is valid JSON
    pub fn is_valid_json(&self) -> bool {
        crate::json_module::JsonModule::is_valid(&self.body)
    }

    /// Get the body as string (useful for debugging)
    pub fn body_as_string(&self) -> String {
        self.body.clone()
    }
}

/// HTTP response
#[derive(Clone, Debug)]
pub struct HttpResponse {
    pub status: StatusCode,
    pub headers: HashMap<String, String>,
    pub body: String,
}

impl HttpResponse {
    pub fn new(status: StatusCode) -> Self {
        let mut response = HttpResponse {
            status,
            headers: HashMap::new(),
            body: String::new(),
        };
        response.headers.insert("Content-Type".to_string(), "text/plain".to_string());
        response
    }

    /// Set response body
    pub fn set_body(mut self, body: String) -> Self {
        self.body = body;
        self
    }

    /// Set JSON body
    pub fn json(mut self, json: String) -> Self {
        self.headers.insert("Content-Type".to_string(), "application/json".to_string());
        self.body = json;
        self
    }

    /// Set header
    pub fn set_header(mut self, key: String, value: String) -> Self {
        self.headers.insert(key, value);
        self
    }

    /// Get header
    pub fn get_header(&self, key: &str) -> Option<String> {
        self.headers.get(key).cloned()
    }

    /// Enable CORS
    pub fn enable_cors(mut self) -> Self {
        self.headers.insert("Access-Control-Allow-Origin".to_string(), "*".to_string());
        self.headers.insert("Access-Control-Allow-Methods".to_string(), "GET, POST, PUT, DELETE, OPTIONS".to_string());
        self.headers.insert("Access-Control-Allow-Headers".to_string(), "Content-Type, Authorization".to_string());
        self
    }

    /// Format for HTTP transmission
    pub fn format(&self) -> String {
        let status_line = format!("HTTP/1.1 {} {}\r\n", self.status.code(), self.status.message());
        let mut headers_str = String::new();
        for (key, value) in &self.headers {
            headers_str.push_str(&format!("{}: {}\r\n", key, value));
        }
        format!("{}{}Content-Length: {}\r\n\r\n{}", status_line, headers_str, self.body.len(), self.body)
    }
}

/// Route handler
pub type RouteHandler = fn(&HttpRequest) -> HttpResponse;

/// Route definition
#[derive(Clone)]
pub struct Route {
    pub method: HttpMethod,
    pub path: String,
    pub handler: String, // Handler name for serialization
}

/// Router for HTTP routing
pub struct Router {
    pub routes: Vec<Route>,
    pub handlers: HashMap<String, RouteHandler>,
}

impl Router {
    pub fn new() -> Self {
        Router {
            routes: Vec::new(),
            handlers: HashMap::new(),
        }
    }

    /// Register route
    pub fn register(&mut self, method: HttpMethod, path: String, handler_name: String) {
        self.routes.push(Route {
            method,
            path,
            handler: handler_name,
        });
    }

    /// GET route
    pub fn get(&mut self, path: String) {
        self.register(HttpMethod::GET, path, "handler".to_string());
    }

    /// POST route
    pub fn post(&mut self, path: String) {
        self.register(HttpMethod::POST, path, "handler".to_string());
    }

    /// Match route
    pub fn match_route(&self, method: &HttpMethod, path: &str) -> Option<&Route> {
        self.routes.iter().find(|route| {
            route.method == *method && (route.path == path || Self::path_matches(&route.path, path))
        })
    }

    /// Check if path matches pattern (supports :param)
    fn path_matches(pattern: &str, path: &str) -> bool {
        let pattern_parts: Vec<&str> = pattern.split('/').collect();
        let path_parts: Vec<&str> = path.split('/').collect();

        if pattern_parts.len() != path_parts.len() {
            return false;
        }

        pattern_parts.iter().zip(path_parts.iter()).all(|(pat, p)| {
            pat.starts_with(':') || pat == p
        })
    }

    /// Extract path parameters
    pub fn extract_params(pattern: &str, path: &str) -> HashMap<String, String> {
        let mut params = HashMap::new();
        let pattern_parts: Vec<&str> = pattern.split('/').collect();
        let path_parts: Vec<&str> = path.split('/').collect();

        for (pat, p) in pattern_parts.iter().zip(path_parts.iter()) {
            if pat.starts_with(':') {
                let key = pat.trim_start_matches(':').to_string();
                params.insert(key, p.to_string());
            }
        }

        params
    }
}

impl Default for Router {
    fn default() -> Self {
        Self::new()
    }
}

/// Middleware trait
#[derive(Clone, Debug)]
pub struct Middleware {
    pub name: String,
    pub enabled: bool,
}

impl Middleware {
    pub fn new(name: String) -> Self {
        Middleware { name, enabled: true }
    }

    /// CORS middleware
    pub fn cors() -> Self {
        Middleware::new("cors".to_string())
    }

    /// Authentication middleware
    pub fn auth() -> Self {
        Middleware::new("auth".to_string())
    }

    /// Logging middleware
    pub fn logging() -> Self {
        Middleware::new("logging".to_string())
    }

    /// Rate limiting middleware
    pub fn rate_limit() -> Self {
        Middleware::new("rate_limit".to_string())
    }
}

/// Web server
pub struct WebServer {
    pub host: String,
    pub port: u16,
    pub router: Router,
    pub middlewares: Vec<Middleware>,
    pub static_dir: Option<String>,
    pub is_running: bool,
}

impl WebServer {
    pub fn new(host: String, port: u16) -> Self {
        WebServer {
            host,
            port,
            router: Router::new(),
            middlewares: Vec::new(),
            static_dir: None,
            is_running: false,
        }
    }

    /// Add middleware
    pub fn use_middleware(mut self, middleware: Middleware) -> Self {
        self.middlewares.push(middleware);
        self
    }

    /// Set static file directory
    pub fn static_files(mut self, dir: String) -> Self {
        self.static_dir = Some(dir);
        self
    }

    /// Start server (simulation)
    pub fn start(&mut self) -> Result<(), String> {
        self.is_running = true;
        Ok(())
    }

    /// Stop server (simulation)
    pub fn stop(&mut self) -> Result<(), String> {
        self.is_running = false;
        Ok(())
    }

    /// Handle request
    pub fn handle_request(&self, request: &HttpRequest) -> HttpResponse {
        // Apply middlewares
        let mut response = HttpResponse::new(StatusCode::NotFound);

        // Check for static file
        if let Some(_static_dir) = &self.static_dir {
            if request.path.starts_with("/static/") {
                return HttpResponse::new(StatusCode::OK)
                    .set_body(format!("Static file: {}", request.path));
            }
        }

        // Try to match route
        if let Some(_route) = self.router.match_route(&request.method, &request.path) {
            response = HttpResponse::new(StatusCode::OK)
                .set_body(format!("Response from route: {}", request.path));
        }

        // Apply CORS if enabled
        if self.middlewares.iter().any(|m| m.name == "cors" && m.enabled) {
            response = response.enable_cors();
        }

        response
    }

    /// Get server info
    pub fn info(&self) -> String {
        format!(
            "Server running on {}:{} | Routes: {} | Middlewares: {}",
            self.host,
            self.port,
            self.router.routes.len(),
            self.middlewares.len()
        )
    }
}

/// Simple template engine
pub struct Template {
    pub content: String,
    pub variables: HashMap<String, String>,
}

impl Template {
    pub fn new(content: String) -> Self {
        Template {
            content,
            variables: HashMap::new(),
        }
    }

    /// Set variable
    pub fn set(mut self, key: String, value: String) -> Self {
        self.variables.insert(key, value);
        self
    }

    /// Render template
    pub fn render(&self) -> String {
        let mut result = self.content.clone();
        for (key, value) in &self.variables {
            let placeholder = format!("{{{{{}}}}}", key);
            result = result.replace(&placeholder, value);
        }
        result
    }
}

/// Web framework facade
pub struct WebFramework;

impl WebFramework {
    /// Create new web server
    pub fn new_server(host: String, port: u16) -> WebServer {
        WebServer::new(host, port)
    }

    /// Create request
    pub fn request(method: String, path: String) -> HttpRequest {
        HttpRequest::new(HttpMethod::from_string(&method), path)
    }

    /// Create response
    pub fn response(status: u16) -> HttpResponse {
        let code = match status {
            200 => StatusCode::OK,
            201 => StatusCode::Created,
            400 => StatusCode::BadRequest,
            401 => StatusCode::Unauthorized,
            403 => StatusCode::Forbidden,
            404 => StatusCode::NotFound,
            500 => StatusCode::InternalError,
            _ => StatusCode::OK,
        };
        HttpResponse::new(code)
    }

    /// Create template
    pub fn template(content: String) -> Template {
        Template::new(content)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_http_method_parsing() {
        assert_eq!(HttpMethod::from_string("GET"), HttpMethod::GET);
        assert_eq!(HttpMethod::from_string("post"), HttpMethod::POST);
        assert_eq!(HttpMethod::from_string("PUT"), HttpMethod::PUT);
    }

    #[test]
    fn test_status_code() {
        assert_eq!(StatusCode::OK.code(), 200);
        assert_eq!(StatusCode::NotFound.code(), 404);
        assert_eq!(StatusCode::InternalError.code(), 500);
    }

    #[test]
    fn test_http_request_creation() {
        let req = HttpRequest::new(HttpMethod::GET, "/api/users".to_string());
        assert_eq!(req.method, HttpMethod::GET);
        assert_eq!(req.path, "/api/users");
    }

    #[test]
    fn test_http_request_headers() {
        let req = HttpRequest::new(HttpMethod::GET, "/".to_string())
            .add_header("Content-Type".to_string(), "application/json".to_string());
        assert_eq!(req.get_header("Content-Type"), Some("application/json".to_string()));
    }

    #[test]
    fn test_http_response_creation() {
        let resp = HttpResponse::new(StatusCode::OK);
        assert_eq!(resp.status.code(), 200);
    }

    #[test]
    fn test_http_response_json() {
        let resp = HttpResponse::new(StatusCode::OK)
            .json("{\"key\": \"value\"}".to_string());
        assert_eq!(resp.get_header("Content-Type"), Some("application/json".to_string()));
    }

    #[test]
    fn test_http_response_cors() {
        let resp = HttpResponse::new(StatusCode::OK)
            .enable_cors();
        assert!(resp.get_header("Access-Control-Allow-Origin").is_some());
    }

    #[test]
    fn test_router_creation() {
        let router = Router::new();
        assert_eq!(router.routes.len(), 0);
    }

    #[test]
    fn test_router_register() {
        let mut router = Router::new();
        router.register(HttpMethod::GET, "/users".to_string(), "get_users".to_string());
        assert_eq!(router.routes.len(), 1);
    }

    #[test]
    fn test_router_match() {
        let mut router = Router::new();
        router.register(HttpMethod::GET, "/users".to_string(), "get_users".to_string());
        let route = router.match_route(&HttpMethod::GET, "/users");
        assert!(route.is_some());
    }

    #[test]
    fn test_router_path_params() {
        let params = Router::extract_params("/users/:id", "/users/123");
        assert_eq!(params.get("id"), Some(&"123".to_string()));
    }

    #[test]
    fn test_middleware_creation() {
        let cors = Middleware::cors();
        assert_eq!(cors.name, "cors");
        assert!(cors.enabled);
    }

    #[test]
    fn test_middleware_auth() {
        let auth = Middleware::auth();
        assert_eq!(auth.name, "auth");
    }

    #[test]
    fn test_web_server_creation() {
        let server = WebServer::new("127.0.0.1".to_string(), 8080);
        assert_eq!(server.host, "127.0.0.1");
        assert_eq!(server.port, 8080);
        assert!(!server.is_running);
    }

    #[test]
    fn test_web_server_start_stop() {
        let mut server = WebServer::new("127.0.0.1".to_string(), 8080);
        assert!(server.start().is_ok());
        assert!(server.is_running);
        assert!(server.stop().is_ok());
        assert!(!server.is_running);
    }

    #[test]
    fn test_web_server_middleware() {
        let server = WebServer::new("127.0.0.1".to_string(), 8080)
            .use_middleware(Middleware::cors());
        assert_eq!(server.middlewares.len(), 1);
    }

    #[test]
    fn test_web_server_static_files() {
        let server = WebServer::new("127.0.0.1".to_string(), 8080)
            .static_files("./public".to_string());
        assert!(server.static_dir.is_some());
    }

    #[test]
    fn test_template_rendering() {
        let template = Template::new("Hello {{name}}!".to_string())
            .set("name".to_string(), "World".to_string());
        assert_eq!(template.render(), "Hello World!");
    }

    #[test]
    fn test_template_multiple_vars() {
        let template = Template::new("{{greeting}} {{name}}!".to_string())
            .set("greeting".to_string(), "Hello".to_string())
            .set("name".to_string(), "World".to_string());
        assert_eq!(template.render(), "Hello World!");
    }
}
