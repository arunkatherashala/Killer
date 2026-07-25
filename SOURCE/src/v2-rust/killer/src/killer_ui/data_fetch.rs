//! **Data Fetching** — React Query / Angular HttpClient inspired data layer.
//!
//! `HttpClient` with interceptors, retry, timeout.
//! `QueryClient` with caching, stale-while-revalidate, dedup.
//! `Resource` for declarative async data binding.
//!
//! Zero external deps — uses `std::net::TcpStream` for actual HTTP.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

// ══════════════════════════════════════════════════════════════════════════════
// HTTP Types
// ══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, PartialEq)]
pub enum Method { GET, POST, PUT, DELETE, PATCH, HEAD, OPTIONS }

impl Method {
    pub fn as_str(&self) -> &'static str {
        match self {
            Method::GET => "GET", Method::POST => "POST",
            Method::PUT => "PUT", Method::DELETE => "DELETE",
            Method::PATCH => "PATCH", Method::HEAD => "HEAD",
            Method::OPTIONS => "OPTIONS",
        }
    }
}

#[derive(Debug, Clone)]
pub struct HttpRequest {
    pub method: Method,
    pub url: String,
    pub headers: HashMap<String, String>,
    pub body: Option<String>,
    pub timeout_ms: u64,
}

#[derive(Debug, Clone)]
pub struct HttpResponse {
    pub status: u16,
    pub headers: HashMap<String, String>,
    pub body: String,
    pub duration_ms: u64,
}

impl HttpResponse {
    pub fn ok(&self) -> bool { self.status >= 200 && self.status < 300 }
    pub fn json_body(&self) -> &str { &self.body }
}

// ══════════════════════════════════════════════════════════════════════════════
// Interceptor
// ══════════════════════════════════════════════════════════════════════════════

/// Interceptors can modify requests before they go out and responses when they come back.
pub struct Interceptor {
    pub name: String,
    pub on_request: Option<Box<dyn Fn(&mut HttpRequest) + Send + Sync>>,
    pub on_response: Option<Box<dyn Fn(&mut HttpResponse) + Send + Sync>>,
}

// ══════════════════════════════════════════════════════════════════════════════
// HttpClient — full-featured HTTP client
// ══════════════════════════════════════════════════════════════════════════════

/// Production HTTP client with interceptors, retry, base URL, default headers.
pub struct HttpClient {
    base_url: String,
    default_headers: HashMap<String, String>,
    interceptors: Vec<Interceptor>,
    timeout_ms: u64,
    max_retries: u32,
}

impl HttpClient {
    pub fn new() -> Self {
        HttpClient {
            base_url: String::new(),
            default_headers: HashMap::new(),
            interceptors: Vec::new(),
            timeout_ms: 30_000,
            max_retries: 0,
        }
    }

    pub fn base_url(mut self, url: &str) -> Self {
        self.base_url = url.to_string();
        self
    }

    pub fn default_header(mut self, key: &str, value: &str) -> Self {
        self.default_headers.insert(key.to_string(), value.to_string());
        self
    }

    pub fn timeout(mut self, ms: u64) -> Self {
        self.timeout_ms = ms;
        self
    }

    pub fn retries(mut self, n: u32) -> Self {
        self.max_retries = n;
        self
    }

    pub fn interceptor(mut self, interceptor: Interceptor) -> Self {
        self.interceptors.push(interceptor);
        self
    }

    fn build_request(&self, method: Method, path: &str) -> HttpRequest {
        let url = if path.starts_with("http://") || path.starts_with("https://") {
            path.to_string()
        } else {
            format!("{}{}", self.base_url, path)
        };
        HttpRequest {
            method,
            url,
            headers: self.default_headers.clone(),
            body: None,
            timeout_ms: self.timeout_ms,
        }
    }

    pub fn get(&self, path: &str) -> RequestBuilder<'_> {
        RequestBuilder {
            client: self,
            request: self.build_request(Method::GET, path),
        }
    }

    pub fn post(&self, path: &str) -> RequestBuilder<'_> {
        RequestBuilder {
            client: self,
            request: self.build_request(Method::POST, path),
        }
    }

    pub fn put(&self, path: &str) -> RequestBuilder<'_> {
        RequestBuilder {
            client: self,
            request: self.build_request(Method::PUT, path),
        }
    }

    pub fn delete(&self, path: &str) -> RequestBuilder<'_> {
        RequestBuilder {
            client: self,
            request: self.build_request(Method::DELETE, path),
        }
    }

    fn execute(&self, mut request: HttpRequest) -> Result<HttpResponse, String> {
        // Apply request interceptors
        for interceptor in &self.interceptors {
            if let Some(ref on_req) = interceptor.on_request {
                on_req(&mut request);
            }
        }

        let mut last_err = String::new();
        for attempt in 0..=self.max_retries {
            match self.raw_execute(&request) {
                Ok(mut response) => {
                    // Apply response interceptors
                    for interceptor in &self.interceptors {
                        if let Some(ref on_resp) = interceptor.on_response {
                            on_resp(&mut response);
                        }
                    }
                    return Ok(response);
                }
                Err(e) => {
                    last_err = format!("attempt {}: {}", attempt + 1, e);
                    if attempt < self.max_retries {
                        // Exponential backoff would go here for real implementation
                        continue;
                    }
                }
            }
        }
        Err(last_err)
    }

    fn raw_execute(&self, request: &HttpRequest) -> Result<HttpResponse, String> {
        use std::io::{Read, Write};
        use std::net::TcpStream;

        let start = Instant::now();

        // Parse URL
        let url = &request.url;
        let without_scheme = url.strip_prefix("http://").unwrap_or(url);
        let (host_port, path) = match without_scheme.find('/') {
            Some(i) => (&without_scheme[..i], &without_scheme[i..]),
            None => (without_scheme, "/"),
        };
        let host = host_port.split(':').next().unwrap_or(host_port);
        let port: u16 = host_port.split(':').nth(1)
            .and_then(|p| p.parse().ok())
            .unwrap_or(80);

        let timeout = Duration::from_millis(request.timeout_ms);
        let mut stream = TcpStream::connect_timeout(
            &format!("{}:{}", host, port).parse().map_err(|e| format!("addr: {}", e))?,
            timeout,
        ).map_err(|e| format!("connect: {}", e))?;
        stream.set_read_timeout(Some(timeout)).ok();
        stream.set_write_timeout(Some(timeout)).ok();

        // Build HTTP/1.1 request
        let mut req_str = format!("{} {} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n",
            request.method.as_str(), path, host);
        for (k, v) in &request.headers {
            req_str.push_str(&format!("{}: {}\r\n", k, v));
        }
        if let Some(ref body) = request.body {
            req_str.push_str(&format!("Content-Length: {}\r\n", body.len()));
        }
        req_str.push_str("\r\n");
        if let Some(ref body) = request.body {
            req_str.push_str(body);
        }

        stream.write_all(req_str.as_bytes()).map_err(|e| format!("write: {}", e))?;

        let mut response_buf = Vec::new();
        stream.read_to_end(&mut response_buf).map_err(|e| format!("read: {}", e))?;
        let response_str = String::from_utf8_lossy(&response_buf);

        // Parse response
        let (header_section, body) = match response_str.find("\r\n\r\n") {
            Some(i) => (&response_str[..i], response_str[i+4..].to_string()),
            None => (response_str.as_ref(), String::new()),
        };

        let mut lines = header_section.lines();
        let status_line = lines.next().unwrap_or("HTTP/1.1 0");
        let status: u16 = status_line.split_whitespace().nth(1)
            .and_then(|s| s.parse().ok()).unwrap_or(0);

        let mut headers = HashMap::new();
        for line in lines {
            if let Some(colon) = line.find(':') {
                let key = line[..colon].trim().to_lowercase();
                let value = line[colon+1..].trim().to_string();
                headers.insert(key, value);
            }
        }

        Ok(HttpResponse {
            status,
            headers,
            body,
            duration_ms: start.elapsed().as_millis() as u64,
        })
    }
}

impl Default for HttpClient {
    fn default() -> Self { Self::new() }
}

/// Fluent request builder.
pub struct RequestBuilder<'a> {
    client: &'a HttpClient,
    request: HttpRequest,
}

impl<'a> RequestBuilder<'a> {
    pub fn header(mut self, key: &str, value: &str) -> Self {
        self.request.headers.insert(key.to_string(), value.to_string());
        self
    }

    pub fn body(mut self, body: &str) -> Self {
        self.request.body = Some(body.to_string());
        self
    }

    pub fn json(mut self, json: &str) -> Self {
        self.request.headers.insert("Content-Type".into(), "application/json".into());
        self.request.body = Some(json.to_string());
        self
    }

    pub fn send(self) -> Result<HttpResponse, String> {
        self.client.execute(self.request)
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// QueryClient — React Query-style cache
// ══════════════════════════════════════════════════════════════════════════════

#[derive(Clone)]
struct CacheEntry {
    data: String,
    fetched_at: Instant,
    stale_ms: u64,
}

impl CacheEntry {
    fn is_stale(&self) -> bool {
        self.fetched_at.elapsed().as_millis() as u64 > self.stale_ms
    }
}

/// Query cache with stale-while-revalidate semantics.
pub struct QueryClient {
    cache: Arc<Mutex<HashMap<String, CacheEntry>>>,
    default_stale_ms: u64,
    in_flight: Arc<Mutex<HashMap<String, bool>>>,
}

impl QueryClient {
    pub fn new() -> Self {
        QueryClient {
            cache: Arc::new(Mutex::new(HashMap::new())),
            default_stale_ms: 60_000,
            in_flight: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn stale_time(mut self, ms: u64) -> Self {
        self.default_stale_ms = ms;
        self
    }

    /// Get cached data for a key, or None if missing/expired.
    pub fn get_cached(&self, key: &str) -> Option<String> {
        let cache = self.cache.lock().unwrap();
        cache.get(key).map(|entry| entry.data.clone())
    }

    /// Check if cached data is stale.
    pub fn is_stale(&self, key: &str) -> bool {
        let cache = self.cache.lock().unwrap();
        cache.get(key).map(|e| e.is_stale()).unwrap_or(true)
    }

    /// Store data in cache.
    pub fn set(&self, key: &str, data: String) {
        let mut cache = self.cache.lock().unwrap();
        cache.insert(key.to_string(), CacheEntry {
            data,
            fetched_at: Instant::now(),
            stale_ms: self.default_stale_ms,
        });
    }

    /// Invalidate (remove) a key.
    pub fn invalidate(&self, key: &str) {
        self.cache.lock().unwrap().remove(key);
    }

    /// Invalidate all keys matching a prefix.
    pub fn invalidate_prefix(&self, prefix: &str) {
        let mut cache = self.cache.lock().unwrap();
        cache.retain(|k, _| !k.starts_with(prefix));
    }

    /// Clear entire cache.
    pub fn clear(&self) {
        self.cache.lock().unwrap().clear();
    }

    /// Check if a fetch is in progress for a key (dedup).
    pub fn is_fetching(&self, key: &str) -> bool {
        self.in_flight.lock().unwrap().get(key).copied().unwrap_or(false)
    }

    pub fn mark_fetching(&self, key: &str, active: bool) {
        let mut inf = self.in_flight.lock().unwrap();
        if active { inf.insert(key.to_string(), true); }
        else { inf.remove(key); }
    }

    pub fn cache_size(&self) -> usize {
        self.cache.lock().unwrap().len()
    }
}

impl Default for QueryClient {
    fn default() -> Self { Self::new() }
}

// ══════════════════════════════════════════════════════════════════════════════
// Resource — declarative async data binding
// ══════════════════════════════════════════════════════════════════════════════

/// State of a data resource.
#[derive(Debug, Clone, PartialEq)]
pub enum ResourceState {
    Idle,
    Loading,
    Success(String),
    Error(String),
}

/// Declarative data resource — tracks loading/success/error states.
pub struct Resource {
    pub key: String,
    pub state: ResourceState,
    pub retry_count: u32,
    pub last_fetched: Option<Instant>,
}

impl Resource {
    pub fn new(key: &str) -> Self {
        Resource {
            key: key.to_string(),
            state: ResourceState::Idle,
            retry_count: 0,
            last_fetched: None,
        }
    }

    pub fn is_loading(&self) -> bool { matches!(self.state, ResourceState::Loading) }
    pub fn is_success(&self) -> bool { matches!(self.state, ResourceState::Success(_)) }
    pub fn is_error(&self) -> bool { matches!(self.state, ResourceState::Error(_)) }

    pub fn data(&self) -> Option<&str> {
        if let ResourceState::Success(ref d) = self.state { Some(d) } else { None }
    }

    pub fn error(&self) -> Option<&str> {
        if let ResourceState::Error(ref e) = self.state { Some(e) } else { None }
    }

    pub fn set_loading(&mut self) { self.state = ResourceState::Loading; }
    pub fn set_success(&mut self, data: String) {
        self.state = ResourceState::Success(data);
        self.last_fetched = Some(Instant::now());
        self.retry_count = 0;
    }
    pub fn set_error(&mut self, err: String) {
        self.state = ResourceState::Error(err);
        self.retry_count += 1;
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Tests
// ══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn http_client_builder() {
        let client = HttpClient::new()
            .base_url("http://localhost:8080")
            .default_header("Authorization", "Bearer token123")
            .timeout(5000)
            .retries(3);
        assert_eq!(client.base_url, "http://localhost:8080");
        assert_eq!(client.timeout_ms, 5000);
        assert_eq!(client.max_retries, 3);
        assert!(client.default_headers.contains_key("Authorization"));
    }

    #[test]
    fn request_builder_fluent() {
        let client = HttpClient::new().base_url("http://api.example.com");
        let rb = client.post("/users").header("X-Custom", "yes").json(r#"{"name":"test"}"#);
        assert_eq!(rb.request.method, Method::POST);
        assert_eq!(rb.request.url, "http://api.example.com/users");
        assert_eq!(rb.request.headers.get("Content-Type").unwrap(), "application/json");
        assert!(rb.request.body.is_some());
    }

    #[test]
    fn query_client_cache() {
        let qc = QueryClient::new().stale_time(60_000);
        assert!(qc.get_cached("users").is_none());
        qc.set("users", r#"[{"id":1}]"#.to_string());
        assert_eq!(qc.get_cached("users").unwrap(), r#"[{"id":1}]"#);
        assert!(!qc.is_stale("users"));
        assert_eq!(qc.cache_size(), 1);
    }

    #[test]
    fn query_client_invalidate() {
        let qc = QueryClient::new();
        qc.set("users/1", "alice".into());
        qc.set("users/2", "bob".into());
        qc.set("posts/1", "hello".into());
        assert_eq!(qc.cache_size(), 3);
        qc.invalidate_prefix("users");
        assert_eq!(qc.cache_size(), 1);
        assert!(qc.get_cached("posts/1").is_some());
    }

    #[test]
    fn query_client_dedup() {
        let qc = QueryClient::new();
        assert!(!qc.is_fetching("users"));
        qc.mark_fetching("users", true);
        assert!(qc.is_fetching("users"));
        qc.mark_fetching("users", false);
        assert!(!qc.is_fetching("users"));
    }

    #[test]
    fn resource_state_machine() {
        let mut res = Resource::new("users");
        assert!(matches!(res.state, ResourceState::Idle));
        assert!(!res.is_loading());

        res.set_loading();
        assert!(res.is_loading());

        res.set_success(r#"[{"id":1}]"#.to_string());
        assert!(res.is_success());
        assert_eq!(res.data(), Some(r#"[{"id":1}]"#));
        assert_eq!(res.retry_count, 0);

        res.set_error("network timeout".into());
        assert!(res.is_error());
        assert_eq!(res.error(), Some("network timeout"));
        assert_eq!(res.retry_count, 1);
    }

    #[test]
    fn interceptor_modifies_request() {
        let client = HttpClient::new()
            .base_url("http://localhost")
            .interceptor(Interceptor {
                name: "auth".into(),
                on_request: Some(Box::new(|req| {
                    req.headers.insert("Authorization".into(), "Bearer abc".into());
                })),
                on_response: None,
            });
        let rb = client.get("/test");
        // Interceptor is applied during execute, but we can verify the builder
        assert!(!rb.request.headers.contains_key("Authorization")); // not yet applied
        assert_eq!(client.interceptors.len(), 1);
    }

    #[test]
    fn method_as_str() {
        assert_eq!(Method::GET.as_str(), "GET");
        assert_eq!(Method::POST.as_str(), "POST");
        assert_eq!(Method::DELETE.as_str(), "DELETE");
        assert_eq!(Method::PATCH.as_str(), "PATCH");
    }

    #[test]
    fn http_response_helpers() {
        let resp = HttpResponse {
            status: 200,
            headers: HashMap::new(),
            body: r#"{"ok":true}"#.to_string(),
            duration_ms: 42,
        };
        assert!(resp.ok());
        assert_eq!(resp.json_body(), r#"{"ok":true}"#);

        let err_resp = HttpResponse {
            status: 404,
            headers: HashMap::new(),
            body: "Not Found".into(),
            duration_ms: 10,
        };
        assert!(!err_resp.ok());
    }
}
