// src/v2-rust/killer_vm/src/http.rs
// HTTP module for Killer language
// Provides HttpRequest, HttpResponse, and basic HTTP parsing

use std::collections::HashMap;

/// HTTP request representation
#[derive(Clone, Debug)]
pub struct HttpRequest {
    pub method: String,
    pub path: String,
    pub version: String,
    pub headers: HashMap<String, String>,
    pub body: String,
}

impl HttpRequest {
    /// Create a new HTTP request
    pub fn new(method: &str, path: &str) -> Self {
        HttpRequest {
            method: method.to_string(),
            path: path.to_string(),
            version: "HTTP/1.1".to_string(),
            headers: HashMap::new(),
            body: String::new(),
        }
    }

    /// Add a header to the request
    pub fn add_header(&mut self, key: &str, value: &str) {
        self.headers.insert(key.to_lowercase(), value.to_string());
    }

    /// Get a header value
    pub fn get_header(&self, key: &str) -> Option<String> {
        self.headers.get(&key.to_lowercase()).cloned()
    }

    /// Get Content-Length header
    pub fn content_length(&self) -> usize {
        self.get_header("content-length")
            .and_then(|val| val.parse::<usize>().ok())
            .unwrap_or(0)
    }

    /// Check if request body is JSON
    pub fn is_json(&self) -> bool {
        self.get_header("content-type")
            .map(|ct| ct.contains("application/json"))
            .unwrap_or(false)
    }

    /// Convert request to string format
    pub fn to_string_http(&self) -> String {
        let mut request_line = format!("{} {} {}\r\n", self.method, self.path, self.version);
        
        for (key, value) in &self.headers {
            request_line.push_str(&format!("{}: {}\r\n", key, value));
        }
        
        request_line.push_str("\r\n");
        request_line.push_str(&self.body);
        
        request_line
    }
}

/// HTTP response representation
#[derive(Clone, Debug)]
pub struct HttpResponse {
    pub status_code: u16,
    pub status_text: String,
    pub version: String,
    pub headers: HashMap<String, String>,
    pub body: String,
}

impl HttpResponse {
    /// Create a new HTTP response with status code
    pub fn new(status_code: u16) -> Self {
        let status_text = match status_code {
            200 => "OK",
            201 => "Created",
            204 => "No Content",
            400 => "Bad Request",
            401 => "Unauthorized",
            403 => "Forbidden",
            404 => "Not Found",
            500 => "Internal Server Error",
            502 => "Bad Gateway",
            503 => "Service Unavailable",
            _ => "Unknown",
        };
        
        let mut response = HttpResponse {
            status_code,
            status_text: status_text.to_string(),
            version: "HTTP/1.1".to_string(),
            headers: HashMap::new(),
            body: String::new(),
        };
        
        response.add_header("Content-Type", "text/plain");
        response.add_header("Server", "Killer-HTTP/3.0");
        
        response
    }

    /// Create a JSON response
    pub fn json(status_code: u16, json_body: &str) -> Self {
        let mut response = HttpResponse::new(status_code);
        response.add_header("Content-Type", "application/json");
        response.body = json_body.to_string();
        response
    }

    /// Create a text response
    pub fn text(status_code: u16, text: &str) -> Self {
        let mut response = HttpResponse::new(status_code);
        response.add_header("Content-Type", "text/plain");
        response.body = text.to_string();
        response
    }

    /// Create an HTML response
    pub fn html(status_code: u16, html: &str) -> Self {
        let mut response = HttpResponse::new(status_code);
        response.add_header("Content-Type", "text/html");
        response.body = html.to_string();
        response
    }

    /// Add a header to the response
    pub fn add_header(&mut self, key: &str, value: &str) {
        self.headers.insert(key.to_lowercase(), value.to_string());
    }

    /// Get a header value
    pub fn get_header(&self, key: &str) -> Option<String> {
        self.headers.get(&key.to_lowercase()).cloned()
    }

    /// Set response body
    pub fn set_body(&mut self, body: &str) {
        self.body = body.to_string();
        self.add_header("Content-Length", &body.len().to_string());
    }

    /// Convert response to HTTP string format
    pub fn to_string_http(&self) -> String {
        let mut response_line = format!(
            "{} {} {}\r\n",
            self.version, self.status_code, self.status_text
        );
        
        // Add Content-Length if body exists
        let body_to_send = if self.body.is_empty() {
            self.body.clone()
        } else {
            self.body.clone()
        };
        
        for (key, value) in &self.headers {
            response_line.push_str(&format!("{}: {}\r\n", key, value));
        }
        
        response_line.push_str("\r\n");
        response_line.push_str(&body_to_send);
        
        response_line
    }
}

/// Parse HTTP request from raw string
pub fn parse_http_request(raw_request: &str) -> Result<HttpRequest, String> {
    let lines: Vec<&str> = raw_request.lines().collect();
    
    if lines.is_empty() {
        return Err("Empty request".to_string());
    }
    
    // Parse request line
    let request_line_parts: Vec<&str> = lines[0].split_whitespace().collect();
    if request_line_parts.len() < 3 {
        return Err("Invalid request line".to_string());
    }
    
    let method = request_line_parts[0].to_uppercase();
    let path = request_line_parts[1];
    let version = request_line_parts[2];
    
    let mut request = HttpRequest::new(&method, path);
    request.version = version.to_string();
    
    // Parse headers
    let mut header_end = 1;
    for i in 1..lines.len() {
        let line = lines[i];
        if line.is_empty() {
            header_end = i;
            break;
        }
        
        if let Some(colon_pos) = line.find(':') {
            let key = line[..colon_pos].trim();
            let value = line[colon_pos + 1..].trim();
            request.add_header(key, value);
        }
    }
    
    // Parse body
    if header_end + 1 < lines.len() {
        request.body = lines[header_end + 1..].join("\n");
    }
    
    Ok(request)
}

/// Simple JSON-like object representation
#[derive(Clone, Debug)]
pub struct JsonValue {
    pub data: String,
}

/// Parse simple JSON to dict (basic implementation)
pub fn parse_json_basic(json_str: &str) -> Result<HashMap<String, String>, String> {
    let mut result = HashMap::new();
    
    // Simple JSON parser for {"key": "value", ...} format
    let json_str = json_str.trim();
    
    if !json_str.starts_with('{') || !json_str.ends_with('}') {
        return Err("Invalid JSON format".to_string());
    }
    
    let content = &json_str[1..json_str.len()-1];
    
    for pair in content.split(',') {
        let pair = pair.trim();
        if let Some(colon_pos) = pair.find(':') {
            let key_part = pair[..colon_pos].trim();
            let value_part = pair[colon_pos + 1..].trim();
            
            // Remove quotes from key
            let key = if key_part.starts_with('"') && key_part.ends_with('"') {
                key_part[1..key_part.len()-1].to_string()
            } else {
                key_part.to_string()
            };
            
            // Remove quotes from value
            let value = if value_part.starts_with('"') && value_part.ends_with('"') {
                value_part[1..value_part.len()-1].to_string()
            } else {
                value_part.to_string()
            };
            
            result.insert(key, value);
        }
    }
    
    Ok(result)
}

/// Convert dict to JSON string (basic implementation)
pub fn dict_to_json(dict: &HashMap<String, String>) -> String {
    let mut json = String::from("{");
    let entries: Vec<String> = dict
        .iter()
        .map(|(k, v)| format!("\"{}\":\"{}\"", k, v))
        .collect();
    json.push_str(&entries.join(","));
    json.push('}');
    json
}

/// HTTP server simulation (returns canned responses for demo)
#[derive(Clone, Debug)]
pub struct KillerHttpServer {
    pub host: String,
    pub port: u16,
    pub running: bool,
}

impl KillerHttpServer {
    /// Create a new HTTP server
    pub fn new(host: &str, port: u16) -> Self {
        KillerHttpServer {
            host: host.to_string(),
            port,
            running: false,
        }
    }

    /// Start the server (simulated)
    pub fn listen(&mut self) -> Result<(), String> {
        self.running = true;
        Ok(())
    }

    /// Stop the server
    pub fn stop(&mut self) {
        self.running = false;
    }
}

/// Simple HTTP client response (simulated for v3.0)
#[derive(Clone, Debug)]
pub struct HttpClientResponse {
    pub status_code: u16,
    pub headers: HashMap<String, String>,
    pub body: String,
}

impl HttpClientResponse {
    /// Create a new response
    pub fn new(status_code: u16, body: &str) -> Self {
        let mut headers = HashMap::new();
        headers.insert("content-type".to_string(), "text/plain".to_string());
        
        HttpClientResponse {
            status_code,
            headers,
            body: body.to_string(),
        }
    }
}

/// Simulate an HTTP GET request (v3.0: returns mock response)
pub fn http_get_request(url: &str) -> Result<HttpClientResponse, String> {
    // v3.0: Return a mock response for demo purposes
    // v3.1+: Will implement actual HTTP client
    
    if url.contains("error") {
        return Ok(HttpClientResponse::new(500, "{\"error\":\"Server error\"}"));
    }
    
    Ok(HttpClientResponse::new(
        200,
        "{\"status\":\"ok\",\"message\":\"Demo response\"}",
    ))
}

/// Simulate an HTTP POST request (v3.0: returns mock response)
pub fn http_post_request(_url: &str, body: &str) -> Result<HttpClientResponse, String> {
    // v3.0: Return a mock response for demo purposes
    // v3.1+: Will implement actual HTTP client
    
    if body.contains("error") {
        return Ok(HttpClientResponse::new(400, "{\"error\":\"Bad request\"}"));
    }
    
    Ok(HttpClientResponse::new(
        201,
        "{\"status\":\"created\",\"id\":\"12345\"}",
    ))
}
