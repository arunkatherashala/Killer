// ================================================================
// MIDDLEWARE - Phase 24.3
// Request/response filtering, logging, compression, security, rate limiting
// ================================================================

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;

/// Middleware trait for request/response processing
pub trait Middleware: Send + Sync {
    fn process_request(&self, req: &mut String) -> Result<(), String>;
    fn process_response(&self, res: &mut String) -> Result<(), String>;
}

/// Rate limit entry
#[derive(Clone, Debug)]
pub struct RateLimitEntry {
    pub ip: String,
    pub count: u32,
    pub reset_time: u64,
}

/// Middleware configuration
pub struct MiddlewareConfig {
    pub enable_cors: bool,
    pub enable_logging: bool,
    pub enable_compression: bool,
    pub enable_security: bool,
    pub enable_rate_limit: bool,
    pub rate_limit_requests: u32,
    pub rate_limit_window: u64,
}

pub struct MiddlewareSolver;

impl MiddlewareSolver {
    // ================================================================
    // MIDDLEWARE PIPELINE (1-10)
    // ================================================================

    /// Problem 1: Create middleware configuration
    pub fn new_config() -> MiddlewareConfig {
        MiddlewareConfig {
            enable_cors: false,
            enable_logging: false,
            enable_compression: false,
            enable_security: false,
            enable_rate_limit: false,
            rate_limit_requests: 100,
            rate_limit_window: 60,
        }
    }

    /// Problem 2: Enable CORS middleware
    pub fn enable_cors(config: &mut MiddlewareConfig) {
        config.enable_cors = true;
    }

    /// Problem 3: Enable logging middleware
    pub fn enable_logging(config: &mut MiddlewareConfig) {
        config.enable_logging = true;
    }

    /// Problem 4: Enable compression middleware
    pub fn enable_compression(config: &mut MiddlewareConfig) {
        config.enable_compression = true;
    }

    /// Problem 5: Enable security middleware
    pub fn enable_security(config: &mut MiddlewareConfig) {
        config.enable_security = true;
    }

    /// Problem 6: Enable rate limiting middleware
    pub fn enable_rate_limiting(config: &mut MiddlewareConfig, requests: u32, window: u64) {
        config.enable_rate_limit = true;
        config.rate_limit_requests = requests;
        config.rate_limit_window = window;
    }

    /// Problem 7: Disable middleware
    pub fn disable_middleware(config: &mut MiddlewareConfig, name: &str) {
        match name {
            "cors" => config.enable_cors = false,
            "logging" => config.enable_logging = false,
            "compression" => config.enable_compression = false,
            "security" => config.enable_security = false,
            "rate_limit" => config.enable_rate_limit = false,
            _ => {}
        }
    }

    /// Problem 8: Get middleware status
    pub fn middleware_status(config: &MiddlewareConfig, name: &str) -> bool {
        match name {
            "cors" => config.enable_cors,
            "logging" => config.enable_logging,
            "compression" => config.enable_compression,
            "security" => config.enable_security,
            "rate_limit" => config.enable_rate_limit,
            _ => false,
        }
    }

    /// Problem 9: Apply middleware chain
    pub fn apply_middleware_chain(
        request: String,
        config: &MiddlewareConfig,
    ) -> Result<String, String> {
        let mut req = request;
        
        if config.enable_logging {
            req = Self::process_logging(&req)?;
        }
        if config.enable_cors {
            req = Self::process_cors(&req)?;
        }
        if config.enable_security {
            req = Self::process_security(&req)?;
        }
        if config.enable_compression {
            req = Self::process_compression(&req)?;
        }
        
        Ok(req)
    }

    /// Problem 10: Process middleware in reverse order for response
    pub fn apply_response_middleware_chain(
        response: String,
        config: &MiddlewareConfig,
    ) -> Result<String, String> {
        let mut res = response;
        
        if config.enable_compression {
            res = Self::process_response_compression(&res)?;
        }
        if config.enable_security {
            res = Self::process_response_security(&res)?;
        }
        if config.enable_cors {
            res = Self::process_response_cors(&res)?;
        }
        
        Ok(res)
    }

    // ================================================================
    // CORS MIDDLEWARE (11-18)
    // ================================================================

    /// Problem 11: Process CORS request
    pub fn process_cors(request: &str) -> Result<String, String> {
        Ok(format!("CORS:{}", request))
    }

    /// Problem 12: Process CORS response
    pub fn process_response_cors(response: &str) -> Result<String, String> {
        Ok(format!("{}:CORS_RESPONSE", response))
    }

    /// Problem 13: Add CORS headers
    pub fn add_cors_headers(headers: &mut HashMap<String, String>, origin: &str) {
        headers.insert("access-control-allow-origin".to_string(), origin.to_string());
        headers.insert("access-control-allow-methods".to_string(), "GET, POST, PUT, DELETE, PATCH, OPTIONS".to_string());
        headers.insert("access-control-allow-headers".to_string(), "Content-Type, Authorization".to_string());
        headers.insert("access-control-max-age".to_string(), "86400".to_string());
    }

    /// Problem 14: Check CORS preflight
    pub fn is_cors_preflight(method: &str, headers: &HashMap<String, String>) -> bool {
        method == "OPTIONS" && headers.contains_key("origin")
    }

    /// Problem 15: Build CORS preflight response
    pub fn build_cors_preflight_response() -> String {
        "HTTP/1.1 204 No Content\r\nAccess-Control-Allow-Origin: *\r\n\r\n".to_string()
    }

    /// Problem 16: Validate CORS origin
    pub fn validate_cors_origin(origin: &str, allowed_origins: &[String]) -> bool {
        allowed_origins.contains(&origin.to_string()) || allowed_origins.contains(&"*".to_string())
    }

    /// Problem 17: Parse Allow-Origin header
    pub fn parse_allow_origin(header: &str) -> Vec<String> {
        header.split(',')
            .map(|s| s.trim().to_string())
            .collect()
    }

    /// Problem 18: Get CORS headers for response
    pub fn get_cors_headers(origin: &str) -> HashMap<String, String> {
        let mut headers = HashMap::new();
        Self::add_cors_headers(&mut headers, origin);
        headers
    }

    // ================================================================
    // LOGGING MIDDLEWARE (19-26)
    // ================================================================

    /// Problem 19: Process logging request
    pub fn process_logging(request: &str) -> Result<String, String> {
        Ok(format!("LOG:{}", request))
    }

    /// Problem 20: Create request log entry
    pub fn log_request(method: &str, path: &str, ip: &str) -> String {
        format!("[{}] {} {} {}", Self::chrono_like_now(), method, path, ip)
    }

    /// Problem 21: Log response
    pub fn log_response(status: u16, duration_ms: u64) -> String {
        format!("Response: {} ({}ms)", status, duration_ms)
    }

    /// Problem 22: Get chrono-like timestamp
    fn chrono_like_now() -> String {
        "2024-01-01T12:00:00Z".to_string()
    }

    /// Problem 23: Format access log
    pub fn format_access_log(method: &str, path: &str, status: u16, duration_ms: u64, ip: &str) -> String {
        format!("{} - {} {} [{}] {} ({}ms)",
            Self::chrono_like_now(),
            ip,
            method,
            path,
            status,
            duration_ms
        )
    }

    /// Problem 24: Parse log level
    pub fn parse_log_level(level: &str) -> u8 {
        match level {
            "DEBUG" => 0,
            "INFO" => 1,
            "WARN" => 2,
            "ERROR" => 3,
            _ => 1,
        }
    }

    /// Problem 25: Should log based on level
    pub fn should_log(current: u8, required: u8) -> bool {
        current >= required
    }

    /// Problem 26: Sanitize log output
    pub fn sanitize_log(text: &str) -> String {
        text.replace("\n", "\\n")
            .replace("\r", "\\r")
            .replace("\"", "\\\"")
    }

    // ================================================================
    // COMPRESSION MIDDLEWARE (27-34)
    // ================================================================

    /// Problem 27: Process compression request
    pub fn process_compression(request: &str) -> Result<String, String> {
        Ok(format!("GZIP:{}", request))
    }

    /// Problem 28: Process response compression
    pub fn process_response_compression(response: &str) -> Result<String, String> {
        Ok(format!("{}:GZIP_RESPONSE", response))
    }

    /// Problem 29: Should compress response
    pub fn should_compress(content_type: &str, content_length: usize) -> bool {
        content_length > 1024 && (
            content_type.contains("text/") ||
            content_type.contains("application/json") ||
            content_type.contains("application/xml")
        )
    }

    /// Problem 30: Get Accept-Encoding
    pub fn get_accept_encoding(headers: &HashMap<String, String>) -> Vec<String> {
        headers.get("accept-encoding")
            .map(|h| h.split(',').map(|s| s.trim().to_string()).collect())
            .unwrap_or_default()
    }

    /// Problem 31: Add compression headers
    pub fn add_compression_headers(headers: &mut HashMap<String, String>, encoding: &str) {
        headers.insert("content-encoding".to_string(), encoding.to_string());
        headers.insert("vary".to_string(), "Accept-Encoding".to_string());
    }

    /// Problem 32: Choose encoding
    pub fn choose_encoding(accepted: &[String]) -> Option<String> {
        for enc in accepted {
            if enc.contains("gzip") {
                return Some("gzip".to_string());
            }
        }
        if accepted.contains(&"deflate".to_string()) {
            return Some("deflate".to_string());
        }
        None
    }

    /// Problem 33: Compress payload
    pub fn compress_payload(data: &[u8]) -> Vec<u8> {
        // Simulated compression
        data.to_vec()
    }

    /// Problem 34: Decompress payload
    pub fn decompress_payload(data: &[u8]) -> Result<Vec<u8>, String> {
        // Simulated decompression
        Ok(data.to_vec())
    }

    // ================================================================
    // SECURITY MIDDLEWARE (35-42)
    // ================================================================

    /// Problem 35: Process security request
    pub fn process_security(request: &str) -> Result<String, String> {
        Ok(format!("SECURITY:{}", request))
    }

    /// Problem 36: Process security response
    pub fn process_response_security(response: &str) -> Result<String, String> {
        Ok(format!("{}:SECURITY_RESPONSE", response))
    }

    /// Problem 37: Add security headers
    pub fn add_security_headers(headers: &mut HashMap<String, String>) {
        headers.insert("x-content-type-options".to_string(), "nosniff".to_string());
        headers.insert("x-frame-options".to_string(), "DENY".to_string());
        headers.insert("x-xss-protection".to_string(), "1; mode=block".to_string());
        headers.insert("strict-transport-security".to_string(), "max-age=31536000".to_string());
    }

    /// Problem 38: Validate CSRF token
    pub fn validate_csrf_token(token: &str, expected: &str) -> bool {
        token == expected
    }

    /// Problem 39: Generate CSRF token
    pub fn generate_csrf_token() -> String {
        "csrf_token_".to_string() + &format!("{:x}", 12345)
    }

    /// Problem 40: Check SQL injection
    pub fn check_sql_injection(input: &str) -> bool {
        let dangerous = vec!["'; DROP", "UNION SELECT", "--", "/*"];
        dangerous.iter().any(|d| input.to_uppercase().contains(d))
    }

    /// Problem 41: Sanitize input
    pub fn sanitize_input(input: &str) -> String {
        input.replace("'", "''")
            .replace("\"", "\\\"")
            .replace(";", "")
    }

    /// Problem 42: Add CSP header
    pub fn add_csp_header(headers: &mut HashMap<String, String>) {
        headers.insert("content-security-policy".to_string(), "default-src 'self'".to_string());
    }

    // ================================================================
    // RATE LIMITING (43-50)
    // ================================================================

    /// Problem 43: Create rate limiter store
    pub fn new_rate_limiter() -> Arc<Mutex<HashMap<String, RateLimitEntry>>> {
        Arc::new(Mutex::new(HashMap::new()))
    }

    /// Problem 44: Check rate limit
    pub fn check_rate_limit(
        limiter: &Arc<Mutex<HashMap<String, RateLimitEntry>>>,
        ip: &str,
        max_requests: u32,
        window: u64,
        now: u64,
    ) -> Result<bool, String> {
        let mut store = limiter.lock().map_err(|e| e.to_string())?;
        
        if let Some(entry) = store.get_mut(ip) {
            if now < entry.reset_time {
                if entry.count >= max_requests {
                    return Ok(false);
                }
                entry.count += 1;
            } else {
                entry.count = 1;
                entry.reset_time = now + window;
            }
        } else {
            store.insert(ip.to_string(), RateLimitEntry {
                ip: ip.to_string(),
                count: 1,
                reset_time: now + window,
            });
        }
        
        Ok(true)
    }

    /// Problem 45: Get remaining requests
    pub fn get_remaining_requests(
        limiter: &Arc<Mutex<HashMap<String, RateLimitEntry>>>,
        ip: &str,
        max_requests: u32,
    ) -> Result<u32, String> {
        let store = limiter.lock().map_err(|e| e.to_string())?;
        
        if let Some(entry) = store.get(ip) {
            Ok(max_requests.saturating_sub(entry.count))
        } else {
            Ok(max_requests)
        }
    }

    /// Problem 46: Get reset time
    pub fn get_reset_time(
        limiter: &Arc<Mutex<HashMap<String, RateLimitEntry>>>,
        ip: &str,
    ) -> Result<Option<u64>, String> {
        let store = limiter.lock().map_err(|e| e.to_string())?;
        
        Ok(store.get(ip).map(|e| e.reset_time))
    }

    /// Problem 47: Add rate limit headers
    pub fn add_rate_limit_headers(
        headers: &mut HashMap<String, String>,
        limit: u32,
        remaining: u32,
        reset: u64,
    ) {
        headers.insert("x-ratelimit-limit".to_string(), limit.to_string());
        headers.insert("x-ratelimit-remaining".to_string(), remaining.to_string());
        headers.insert("x-ratelimit-reset".to_string(), reset.to_string());
    }

    /// Problem 48: Build rate limit exceeded response
    pub fn rate_limit_exceeded_response(reset: u64) -> String {
        format!("HTTP/1.1 429 Too Many Requests\r\nRetry-After: {}\r\n\r\n", reset)
    }

    /// Problem 49: Cleanup expired entries
    pub fn cleanup_expired_entries(
        limiter: &Arc<Mutex<HashMap<String, RateLimitEntry>>>,
        now: u64,
    ) -> Result<(), String> {
        let mut store = limiter.lock().map_err(|e| e.to_string())?;
        store.retain(|_, entry| entry.reset_time > now);
        Ok(())
    }

    /// Problem 50: Get limiter stats
    pub fn get_limiter_stats(
        limiter: &Arc<Mutex<HashMap<String, RateLimitEntry>>>,
    ) -> Result<(usize, u32), String> {
        let store = limiter.lock().map_err(|e| e.to_string())?;
        let total_ips = store.len();
        let total_requests: u32 = store.values().map(|e| e.count).sum();
        Ok((total_ips, total_requests))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_middleware_config() {
        let config = MiddlewareSolver::new_config();
        assert!(!config.enable_cors);
    }

    #[test]
    fn test_enable_cors() {
        let mut config = MiddlewareSolver::new_config();
        MiddlewareSolver::enable_cors(&mut config);
        assert!(config.enable_cors);
    }

    #[test]
    fn test_middleware_status() {
        let mut config = MiddlewareSolver::new_config();
        MiddlewareSolver::enable_logging(&mut config);
        assert!(MiddlewareSolver::middleware_status(&config, "logging"));
    }

    #[test]
    fn test_cors_headers() {
        let mut headers = HashMap::new();
        MiddlewareSolver::add_cors_headers(&mut headers, "http://example.com");
        assert!(headers.contains_key("access-control-allow-origin"));
    }

    #[test]
    fn test_format_access_log() {
        let log = MiddlewareSolver::format_access_log("GET", "/api", 200, 50, "127.0.0.1");
        assert!(log.contains("GET"));
    }

    #[test]
    fn test_rate_limit_check() {
        let limiter = MiddlewareSolver::new_rate_limiter();
        let result = MiddlewareSolver::check_rate_limit(&limiter, "127.0.0.1", 100, 60, 1000);
        assert!(result.is_ok());
    }

    #[test]
    fn test_rate_limit_exceeded() {
        let limiter = MiddlewareSolver::new_rate_limiter();
        for _ in 0..5 {
            let _ = MiddlewareSolver::check_rate_limit(&limiter, "127.0.0.1", 5, 60, 1000);
        }
        let result = MiddlewareSolver::check_rate_limit(&limiter, "127.0.0.1", 5, 60, 1000);
        assert_eq!(result.unwrap(), false);
    }

    #[test]
    fn test_sql_injection_check() {
        assert!(MiddlewareSolver::check_sql_injection("'; DROP TABLE users"));
        assert!(!MiddlewareSolver::check_sql_injection("normal input"));
    }

    #[test]
    fn test_csrf_token() {
        let token = MiddlewareSolver::generate_csrf_token();
        assert!(!token.is_empty());
    }

    #[test]
    fn test_middleware_chain() {
        let mut config = MiddlewareSolver::new_config();
        MiddlewareSolver::enable_cors(&mut config);
        MiddlewareSolver::enable_logging(&mut config);
        let result = MiddlewareSolver::apply_middleware_chain("request".to_string(), &config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_response_middleware_chain() {
        let mut config = MiddlewareSolver::new_config();
        MiddlewareSolver::enable_security(&mut config);
        let result = MiddlewareSolver::apply_response_middleware_chain("response".to_string(), &config);
        assert!(result.is_ok());
    }
}
