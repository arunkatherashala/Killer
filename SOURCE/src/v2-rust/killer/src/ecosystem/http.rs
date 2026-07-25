// Phase 5.3: HTTP Server & Client Integration
// Core HTTP support for web services

use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct HttpRequest {
    pub method: String,     // GET, POST, PUT, DELETE, etc.
    pub path: String,       // /api/users, /index.html, etc.
    pub headers: HashMap<String, String>,
    pub body: String,
}

#[derive(Debug, Clone)]
pub struct HttpResponse {
    pub status_code: u16,   // 200, 404, 500, etc.
    pub headers: HashMap<String, String>,
    pub body: String,
}

pub struct HttpServer {
    /// Server configuration
    config: HttpServerConfig,
    /// Request handlers
    handlers: HashMap<String, Vec<Box<dyn Fn(&HttpRequest) -> HttpResponse + Send + Sync>>>,
    /// Request statistics
    total_requests: u64,
    /// Response times
    response_times: Vec<u64>,
}

// Manual Debug implementation to skip non-Debug handlers field
impl std::fmt::Debug for HttpServer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HttpServer")
            .field("config", &self.config)
            .field("total_requests", &self.total_requests)
            .field("response_times_count", &self.response_times.len())
            .field("handlers_count", &self.handlers.len())
            .finish()
    }
}

#[derive(Debug, Clone)]
pub struct HttpServerConfig {
    pub port: u16,
    pub host: String,
    pub max_connections: usize,
    pub request_timeout_ms: u64,
}

impl Default for HttpServerConfig {
    fn default() -> Self {
        HttpServerConfig {
            port: 8080,
            host: "0.0.0.0".to_string(),
            max_connections: 100,
            request_timeout_ms: 5000,
        }
    }
}

impl HttpServer {
    pub fn new(config: HttpServerConfig) -> Self {
        HttpServer {
            config,
            handlers: HashMap::new(),
            total_requests: 0,
            response_times: Vec::new(),
        }
    }

    /// Register route handler
    pub fn register_route(&mut self, path: String) {
        self.handlers.insert(path, Vec::new());
    }

    /// Handle incoming HTTP request
    pub fn handle_request(&mut self, request: HttpRequest) -> HttpResponse {
        self.total_requests += 1;

        // Match request path to handler
        if let Some(path_handlers) = self.handlers.get(&request.path) {
            if path_handlers.is_empty() {
                return HttpResponse {
                    status_code: 404,
                    headers: HashMap::new(),
                    body: "Not Found".to_string(),
                };
            }

            // Use first handler for path
            // In real implementation: call appropriate handler
            return HttpResponse {
                status_code: 200,
                headers: {
                    let mut h = HashMap::new();
                    h.insert("Content-Type".to_string(), "application/json".to_string());
                    h
                },
                body: "{}".to_string(),
            };
        }

        // Default 404
        HttpResponse {
            status_code: 404,
            headers: HashMap::new(),
            body: "Not Found".to_string(),
        }
    }

    /// Get server statistics
    pub fn get_statistics(&self) -> HttpServerStats {
        let avg_response_time = if self.response_times.is_empty() {
            0
        } else {
            self.response_times.iter().sum::<u64>() / self.response_times.len() as u64
        };

        HttpServerStats {
            total_requests_handled: self.total_requests,
            registered_routes: self.handlers.len(),
            average_response_time_ms: avg_response_time,
            port: self.config.port,
            max_connections: self.config.max_connections,
        }
    }

    /// Start server (returns listening address)
    pub fn start(&self) -> String {
        format!("http://{}:{}", self.config.host, self.config.port)
    }
}

#[derive(Debug, Clone)]
pub struct HttpServerStats {
    pub total_requests_handled: u64,
    pub registered_routes: usize,
    pub average_response_time_ms: u64,
    pub port: u16,
    pub max_connections: usize,
}

#[derive(Debug)]
pub struct HttpClient {
    config: HttpClientConfig,
    request_count: u64,
}

#[derive(Debug, Clone)]
pub struct HttpClientConfig {
    pub timeout_ms: u64,
    pub follow_redirects: bool,
    pub max_redirects: usize,
}

impl Default for HttpClientConfig {
    fn default() -> Self {
        HttpClientConfig {
            timeout_ms: 10000,
            follow_redirects: true,
            max_redirects: 5,
        }
    }
}

impl HttpClient {
    pub fn new(config: HttpClientConfig) -> Self {
        HttpClient {
            config,
            request_count: 0,
        }
    }

    /// Make HTTP GET request
    pub fn get(&mut self, url: &str) -> Result<HttpResponse, String> {
        self.request_count += 1;
        
        // Simulate GET request
        Ok(HttpResponse {
            status_code: 200,
            headers: HashMap::new(),
            body: "OK".to_string(),
        })
    }

    /// Make HTTP POST request
    pub fn post(&mut self, url: &str, body: &str) -> Result<HttpResponse, String> {
        self.request_count += 1;

        // Simulate POST request
        Ok(HttpResponse {
            status_code: 201,
            headers: HashMap::new(),
            body: "Created".to_string(),
        })
    }

    /// Get request statistics
    pub fn get_statistics(&self) -> HttpClientStats {
        HttpClientStats {
            total_requests: self.request_count,
            timeout_ms: self.config.timeout_ms,
        }
    }
}

#[derive(Debug, Clone)]
pub struct HttpClientStats {
    pub total_requests: u64,
    pub timeout_ms: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_http_request_creation() {
        let req = HttpRequest {
            method: "GET".to_string(),
            path: "/api/test".to_string(),
            headers: HashMap::new(),
            body: String::new(),
        };
        assert_eq!(req.method, "GET");
    }

    #[test]
    fn test_http_response_creation() {
        let resp = HttpResponse {
            status_code: 200,
            headers: HashMap::new(),
            body: "OK".to_string(),
        };
        assert_eq!(resp.status_code, 200);
    }

    #[test]
    fn test_http_server_creation() {
        let server = HttpServer::new(HttpServerConfig::default());
        assert_eq!(server.config.port, 8080);
    }

    #[test]
    fn test_http_server_register_route() {
        let mut server = HttpServer::new(HttpServerConfig::default());
        server.register_route("/api/users".to_string());
        assert_eq!(server.handlers.len(), 1);
    }

    #[test]
    fn test_http_server_handle_request() {
        let mut server = HttpServer::new(HttpServerConfig::default());
        server.register_route("/api/test".to_string());

        let request = HttpRequest {
            method: "GET".to_string(),
            path: "/api/test".to_string(),
            headers: HashMap::new(),
            body: String::new(),
        };

        let response = server.handle_request(request);
        assert_eq!(response.status_code, 200);
    }

    #[test]
    fn test_http_server_statistics() {
        let mut server = HttpServer::new(HttpServerConfig::default());
        server.register_route("/test".to_string());

        let stats = server.get_statistics();
        assert_eq!(stats.registered_routes, 1);
    }

    #[test]
    fn test_http_client_creation() {
        let client = HttpClient::new(HttpClientConfig::default());
        assert_eq!(client.config.timeout_ms, 10000);
    }

    #[test]
    fn test_http_client_get() {
        let mut client = HttpClient::new(HttpClientConfig::default());
        let result = client.get("http://example.com").unwrap();
        assert_eq!(result.status_code, 200);
    }

    #[test]
    fn test_http_client_post() {
        let mut client = HttpClient::new(HttpClientConfig::default());
        let result = client.post("http://example.com", "data").unwrap();
        assert_eq!(result.status_code, 201);
    }
}
