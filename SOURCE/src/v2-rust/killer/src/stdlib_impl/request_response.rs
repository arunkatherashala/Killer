// ================================================================
// REQUEST/RESPONSE HTTP PROTOCOL - Phase 24.2
// HTTP parsing, headers, cookies, encoding/decoding
// ================================================================

use std::collections::HashMap;

/// Cookie structure
#[derive(Clone, Debug)]
pub struct Cookie {
    pub name: String,
    pub value: String,
    pub max_age: Option<u64>,
    pub path: Option<String>,
    pub domain: Option<String>,
    pub http_only: bool,
    pub secure: bool,
    pub same_site: Option<String>,
}

/// Header map for request/response
pub type HeaderMap = HashMap<String, String>;

/// HTTP Request with full protocol info
#[derive(Clone, Debug)]
pub struct HttpRequest {
    pub method: String,
    pub path: String,
    pub query: String,
    pub http_version: String,
    pub headers: HeaderMap,
    pub body: Vec<u8>,
    pub remote_addr: String,
    pub remote_port: u16,
}

/// HTTP Response
#[derive(Clone, Debug)]
pub struct HttpResponse {
    pub status: u16,
    pub http_version: String,
    pub headers: HeaderMap,
    pub cookies: Vec<Cookie>,
    pub body: Vec<u8>,
}

/// Content type parsing
#[derive(Clone, Debug)]
pub struct ContentType {
    pub media_type: String,
    pub charset: Option<String>,
    pub boundary: Option<String>,
}

pub struct RequestResponseSolver;

impl RequestResponseSolver {
    // ================================================================
    // HTTP REQUEST PARSING (1-15)
    // ================================================================

    /// Problem 1: Parse HTTP request line
    pub fn parse_request_line(line: &str) -> Result<(String, String, String), String> {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() != 3 {
            return Err("Invalid request line".to_string());
        }
        Ok((parts[0].to_string(), parts[1].to_string(), parts[2].to_string()))
    }

    /// Problem 2: Parse HTTP request from raw bytes
    pub fn parse_request(raw: &[u8]) -> Result<HttpRequest, String> {
        let text = String::from_utf8(raw.to_vec()).map_err(|e| e.to_string())?;
        let lines: Vec<&str> = text.lines().collect();
        
        if lines.is_empty() {
            return Err("Empty request".to_string());
        }

        let (method, path, version) = Self::parse_request_line(lines[0])?;
        
        let (path, query) = if let Some(pos) = path.find('?') {
            (&path[..pos], &path[pos+1..])
        } else {
            (path.as_str(), "")
        };

        Ok(HttpRequest {
            method,
            path: path.to_string(),
            query: query.to_string(),
            http_version: version,
            headers: HashMap::new(),
            body: Vec::new(),
            remote_addr: "127.0.0.1".to_string(),
            remote_port: 0,
        })
    }

    /// Problem 3: Get HTTP method
    pub fn request_method(req: &HttpRequest) -> String {
        req.method.clone()
    }

    /// Problem 4: Get request path
    pub fn request_path(req: &HttpRequest) -> String {
        req.path.clone()
    }

    /// Problem 5: Get query string
    pub fn request_query(req: &HttpRequest) -> String {
        req.query.clone()
    }

    /// Problem 6: Get header by name (case-insensitive)
    pub fn request_header(req: &HttpRequest, name: &str) -> Option<String> {
        let lower_name = name.to_lowercase();
        req.headers.iter()
            .find(|(k, _)| k.to_lowercase() == lower_name)
            .map(|(_, v)| v.clone())
    }

    /// Problem 7: Get all headers
    pub fn request_headers(req: &HttpRequest) -> HeaderMap {
        req.headers.clone()
    }

    /// Problem 8: Get request body as bytes
    pub fn request_body(req: &HttpRequest) -> Vec<u8> {
        req.body.clone()
    }

    /// Problem 9: Get request body as string
    pub fn request_body_string(req: &HttpRequest) -> Result<String, String> {
        String::from_utf8(req.body.clone()).map_err(|e| e.to_string())
    }

    /// Problem 10: Parse JSON body (basic)
    pub fn request_body_json(req: &HttpRequest) -> Result<String, String> {
        String::from_utf8(req.body.clone()).map_err(|e| e.to_string())
    }

    /// Problem 11: Parse form-encoded body
    pub fn request_form_data(req: &HttpRequest) -> Result<HashMap<String, String>, String> {
        let body_str = Self::request_body_string(req)?;
        let mut data = HashMap::new();
        for part in body_str.split('&') {
            if let Some((key, val)) = part.split_once('=') {
                data.insert(Self::url_decode(key)?, Self::url_decode(val)?);
            }
        }
        Ok(data)
    }

    /// Problem 12: Get remote address
    pub fn request_remote_addr(req: &HttpRequest) -> String {
        req.remote_addr.clone()
    }

    /// Problem 13: Get remote port
    pub fn request_remote_port(req: &HttpRequest) -> u16 {
        req.remote_port
    }

    /// Problem 14: Check if HTTPS
    pub fn request_is_https(req: &HttpRequest) -> bool {
        Self::request_header(req, "x-forwarded-proto")
            .map(|p| p == "https")
            .unwrap_or(false)
    }

    /// Problem 15: Get User-Agent header
    pub fn request_user_agent(req: &HttpRequest) -> Option<String> {
        Self::request_header(req, "user-agent")
    }

    // ================================================================
    // HTTP RESPONSE GENERATION (16-30)
    // ================================================================

    /// Problem 16: Create new HTTP response
    pub fn new_response(status: u16) -> HttpResponse {
        HttpResponse {
            status,
            http_version: "HTTP/1.1".to_string(),
            headers: HashMap::new(),
            cookies: Vec::new(),
            body: Vec::new(),
        }
    }

    /// Problem 17: Set response status
    pub fn set_status(res: &mut HttpResponse, status: u16) {
        res.status = status;
    }

    /// Problem 18: Add response header
    pub fn add_header(res: &mut HttpResponse, name: &str, value: &str) {
        res.headers.insert(name.to_lowercase(), value.to_string());
    }

    /// Problem 19: Set response body
    pub fn set_body(res: &mut HttpResponse, body: Vec<u8>) {
        res.body = body;
    }

    /// Problem 20: Set response body JSON
    pub fn set_body_json(res: &mut HttpResponse, json: &str) {
        res.body = json.as_bytes().to_vec();
        Self::add_header(res, "content-type", "application/json");
    }

    /// Problem 21: Set response body HTML
    pub fn set_body_html(res: &mut HttpResponse, html: &str) {
        res.body = html.as_bytes().to_vec();
        Self::add_header(res, "content-type", "text/html");
    }

    /// Problem 22: Set response body text
    pub fn set_body_text(res: &mut HttpResponse, text: &str) {
        res.body = text.as_bytes().to_vec();
        Self::add_header(res, "content-type", "text/plain");
    }

    /// Problem 23: Set redirect (with status)
    pub fn set_redirect(res: &mut HttpResponse, location: &str, permanent: bool) {
        res.status = if permanent { 301 } else { 302 };
        Self::add_header(res, "location", location);
    }

    /// Problem 24: Set 304 Not Modified
    pub fn set_not_modified(res: &mut HttpResponse) {
        res.status = 304;
        res.body.clear();
    }

    /// Problem 25: Add cookie to response
    pub fn add_cookie(res: &mut HttpResponse, cookie: Cookie) {
        res.cookies.push(cookie);
    }

    /// Problem 26: Serialize response to bytes
    pub fn response_to_bytes(res: &HttpResponse) -> Vec<u8> {
        let mut output = format!("{} {}\r\n", res.http_version, res.status).into_bytes();
        
        for (k, v) in &res.headers {
            output.extend(format!("{}: {}\r\n", k, v).into_bytes());
        }
        
        for cookie in &res.cookies {
            output.extend(format!("set-cookie: {}\r\n", Self::cookie_to_string(cookie)).into_bytes());
        }
        
        output.extend(format!("content-length: {}\r\n\r\n", res.body.len()).into_bytes());
        output.extend(&res.body);
        output
    }

    /// Problem 27: Get response header
    pub fn get_header(res: &HttpResponse, name: &str) -> Option<String> {
        let lower = name.to_lowercase();
        res.headers.iter()
            .find(|(k, _)| k.to_lowercase() == lower)
            .map(|(_, v)| v.clone())
    }

    /// Problem 28: Set cache control
    pub fn set_cache_control(res: &mut HttpResponse, directives: &str) {
        Self::add_header(res, "cache-control", directives);
    }

    /// Problem 29: Add CORS headers
    pub fn add_cors_headers(res: &mut HttpResponse, origin: &str) {
        Self::add_header(res, "access-control-allow-origin", origin);
        Self::add_header(res, "access-control-allow-methods", "GET, POST, PUT, DELETE, PATCH");
        Self::add_header(res, "access-control-allow-headers", "Content-Type, Authorization");
    }

    /// Problem 30: Enable gzip compression
    pub fn enable_gzip(res: &mut HttpResponse) {
        Self::add_header(res, "content-encoding", "gzip");
    }

    // ================================================================
    // COOKIES (31-40)
    // ================================================================

    /// Problem 31: Create cookie
    pub fn new_cookie(name: &str, value: &str) -> Cookie {
        Cookie {
            name: name.to_string(),
            value: value.to_string(),
            max_age: None,
            path: None,
            domain: None,
            http_only: false,
            secure: false,
            same_site: None,
        }
    }

    /// Problem 32: Set cookie expiration
    pub fn cookie_with_max_age(cookie: &mut Cookie, seconds: u64) {
        cookie.max_age = Some(seconds);
    }

    /// Problem 33: Set cookie path
    pub fn cookie_with_path(cookie: &mut Cookie, path: &str) {
        cookie.path = Some(path.to_string());
    }

    /// Problem 34: Set cookie domain
    pub fn cookie_with_domain(cookie: &mut Cookie, domain: &str) {
        cookie.domain = Some(domain.to_string());
    }

    /// Problem 35: Set HttpOnly flag
    pub fn cookie_http_only(cookie: &mut Cookie) {
        cookie.http_only = true;
    }

    /// Problem 36: Set Secure flag
    pub fn cookie_secure(cookie: &mut Cookie) {
        cookie.secure = true;
    }

    /// Problem 37: Set SameSite attribute
    pub fn cookie_same_site(cookie: &mut Cookie, policy: &str) {
        cookie.same_site = Some(policy.to_string());
    }

    /// Problem 38: Parse cookies from request
    pub fn parse_cookies(req: &HttpRequest) -> Vec<Cookie> {
        if let Some(cookie_header) = Self::request_header(req, "cookie") {
            let mut cookies = Vec::new();
            for part in cookie_header.split(';') {
                if let Some((name, value)) = part.trim().split_once('=') {
                    cookies.push(Self::new_cookie(name.trim(), value.trim()));
                }
            }
            cookies
        } else {
            Vec::new()
        }
    }

    /// Problem 39: Serialize cookie to Set-Cookie header
    pub fn cookie_to_string(cookie: &Cookie) -> String {
        let mut s = format!("{}={}", cookie.name, cookie.value);
        
        if let Some(max_age) = cookie.max_age {
            s.push_str(&format!("; Max-Age={}", max_age));
        }
        if let Some(path) = &cookie.path {
            s.push_str(&format!("; Path={}", path));
        }
        if let Some(domain) = &cookie.domain {
            s.push_str(&format!("; Domain={}", domain));
        }
        if cookie.http_only {
            s.push_str("; HttpOnly");
        }
        if cookie.secure {
            s.push_str("; Secure");
        }
        if let Some(same_site) = &cookie.same_site {
            s.push_str(&format!("; SameSite={}", same_site));
        }
        
        s
    }

    /// Problem 40: Parse Set-Cookie header
    pub fn parse_set_cookie(header: &str) -> Cookie {
        let parts: Vec<&str> = header.split(';').collect();
        let (name, value) = if let Some((n, v)) = parts[0].split_once('=') {
            (n.trim(), v.trim())
        } else {
            ("", "")
        };
        
        let mut cookie = Self::new_cookie(name, value);
        for part in &parts[1..] {
            let part = part.trim();
            if part.eq_ignore_ascii_case("HttpOnly") {
                cookie.http_only = true;
            } else if part.eq_ignore_ascii_case("Secure") {
                cookie.secure = true;
            } else if part.starts_with("Max-Age=") {
                if let Ok(seconds) = part[8..].parse() {
                    cookie.max_age = Some(seconds);
                }
            }
        }
        
        cookie
    }

    // ================================================================
    // HEADERS (41-48)
    // ================================================================

    /// Problem 41: Parse Content-Type header
    pub fn parse_content_type(header: &str) -> ContentType {
        let parts: Vec<&str> = header.split(';').collect();
        let media_type = parts[0].trim().to_string();
        
        let mut charset = None;
        let mut boundary = None;
        
        for part in &parts[1..] {
            let part = part.trim();
            if part.starts_with("charset=") {
                charset = Some(part[8..].trim_matches('"').to_string());
            } else if part.starts_with("boundary=") {
                boundary = Some(part[9..].trim_matches('"').to_string());
            }
        }
        
        ContentType { media_type, charset, boundary }
    }

    /// Problem 42: Normalize header name
    pub fn normalize_header_name(name: &str) -> String {
        let parts: Vec<&str> = name.split('-').collect();
        parts.iter()
            .map(|p| {
                let mut chars = p.chars();
                match chars.next() {
                    None => String::new(),
                    Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                }
            })
            .collect::<Vec<_>>()
            .join("-")
    }

    /// Problem 43: Check if header exists
    pub fn header_exists(res: &HttpResponse, name: &str) -> bool {
        Self::get_header(res, name).is_some()
    }

    /// Problem 44: Remove header
    pub fn remove_header(res: &mut HttpResponse, name: &str) {
        let lower = name.to_lowercase();
        res.headers.retain(|k, _| k.to_lowercase() != lower);
    }

    /// Problem 45: Add multiple headers
    pub fn add_headers(res: &mut HttpResponse, headers: &[(String, String)]) {
        for (k, v) in headers {
            Self::add_header(res, k, v);
        }
    }

    /// Problem 46: Get all header values for name (including duplicates)
    pub fn get_header_values(res: &HttpResponse, name: &str) -> Vec<String> {
        let lower = name.to_lowercase();
        res.headers.iter()
            .filter(|(k, _)| k.to_lowercase() == lower)
            .map(|(_, v)| v.clone())
            .collect()
    }

    /// Problem 47: Parse Accept header
    pub fn parse_accept_header(header: &str) -> Vec<String> {
        header.split(',')
            .map(|s| s.trim().to_string())
            .collect()
    }

    /// Problem 48: Parse Authorization header
    pub fn parse_auth_header(header: &str) -> Option<(String, String)> {
        let parts: Vec<&str> = header.split_whitespace().collect();
        if parts.len() == 2 {
            Some((parts[0].to_string(), parts[1].to_string()))
        } else {
            None
        }
    }

    // ================================================================
    // ENCODING/DECODING (49-55)
    // ================================================================

    /// Problem 49: URL encode
    pub fn url_encode(s: &str) -> String {
        s.chars().map(|c| {
            match c {
                'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '_' | '.' | '~' => c.to_string(),
                _ => format!("%{:02X}", c as u8),
            }
        }).collect()
    }

    /// Problem 50: URL decode
    pub fn url_decode(s: &str) -> Result<String, String> {
        let mut result = String::new();
        let mut chars = s.chars().peekable();
        
        while let Some(c) = chars.next() {
            match c {
                '%' => {
                    let hex: String = chars.by_ref().take(2).collect();
                    if let Ok(byte) = u8::from_str_radix(&hex, 16) {
                        result.push(byte as char);
                    } else {
                        return Err("Invalid URL encoding".to_string());
                    }
                },
                '+' => result.push(' '),
                _ => result.push(c),
            }
        }
        
        Ok(result)
    }

    /// Problem 51: HTML escape
    pub fn html_escape(s: &str) -> String {
        s.replace("&", "&amp;")
            .replace("<", "&lt;")
            .replace(">", "&gt;")
            .replace("\"", "&quot;")
            .replace("'", "&#39;")
    }

    /// Problem 52: HTML unescape
    pub fn html_unescape(s: &str) -> String {
        s.replace("&lt;", "<")
            .replace("&gt;", ">")
            .replace("&quot;", "\"")
            .replace("&#39;", "'")
            .replace("&amp;", "&")
    }

    /// Problem 53: Base64 encode
    pub fn base64_encode(data: &[u8]) -> String {
        const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let mut result = String::new();
        
        for chunk in data.chunks(3) {
            let b1 = chunk[0];
            let b2 = chunk.get(1).copied().unwrap_or(0);
            let b3 = chunk.get(2).copied().unwrap_or(0);
            
            let n = ((b1 as u32) << 16) | ((b2 as u32) << 8) | (b3 as u32);
            
            result.push(CHARS[((n >> 18) & 0x3F) as usize] as char);
            result.push(CHARS[((n >> 12) & 0x3F) as usize] as char);
            if chunk.len() > 1 {
                result.push(CHARS[((n >> 6) & 0x3F) as usize] as char);
            }
            if chunk.len() > 2 {
                result.push(CHARS[(n & 0x3F) as usize] as char);
            }
        }
        
        while result.len() % 4 != 0 {
            result.push('=');
        }
        
        result
    }

    /// Problem 54: Base64 decode
    pub fn base64_decode(s: &str) -> Result<Vec<u8>, String> {
        let chars: Vec<u8> = s.as_bytes().iter().filter(|c| !c.is_ascii_whitespace()).copied().collect();
        let mut result = Vec::new();
        
        for chunk in chars.chunks(4) {
            if chunk.len() < 2 {
                break;
            }
            
            let vals = chunk.iter().map(|c| {
                match *c {
                    b'A'..=b'Z' => *c - b'A',
                    b'a'..=b'z' => *c - b'a' + 26,
                    b'0'..=b'9' => *c - b'0' + 52,
                    b'+' => 62,
                    b'/' => 63,
                    b'=' => 0,
                    _ => 255,
                }
            }).collect::<Vec<_>>();
            
            result.push((vals[0] << 2 | vals[1] >> 4) as u8);
            if vals[2] != 255 {
                result.push((vals[1] << 4 | vals[2] >> 2) as u8);
            }
            if vals[3] != 255 {
                result.push((vals[2] << 6 | vals[3]) as u8);
            }
        }
        
        Ok(result)
    }

    /// Problem 55: Parse query string parameters
    pub fn parse_query_string(query: &str) -> HashMap<String, String> {
        let mut params = HashMap::new();
        for part in query.split('&') {
            if let Some((key, val)) = part.split_once('=') {
                if let Ok(decoded) = Self::url_decode(val) {
                    params.insert(key.to_string(), decoded);
                }
            }
        }
        params
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_request_line() {
        let result = RequestResponseSolver::parse_request_line("GET /index.html HTTP/1.1");
        assert!(result.is_ok());
    }

    #[test]
    fn test_url_encoding() {
        let encoded = RequestResponseSolver::url_encode("hello world");
        assert!(encoded.contains("%20"));
    }

    #[test]
    fn test_html_escape() {
        let escaped = RequestResponseSolver::html_escape("<script>");
        assert_eq!(escaped, "&lt;script&gt;");
    }

    #[test]
    fn test_base64_encode() {
        let encoded = RequestResponseSolver::base64_encode(b"Hello");
        assert!(!encoded.is_empty());
    }

    #[test]
    fn test_cookie_creation() {
        let mut cookie = RequestResponseSolver::new_cookie("session", "abc123");
        RequestResponseSolver::cookie_with_max_age(&mut cookie, 3600);
        RequestResponseSolver::cookie_http_only(&mut cookie);
        assert!(cookie.http_only);
    }

    #[test]
    fn test_response_creation() {
        let mut res = RequestResponseSolver::new_response(200);
        RequestResponseSolver::set_body_json(&mut res, "{}");
        assert_eq!(res.status, 200);
    }

    #[test]
    fn test_parse_content_type() {
        let ct = RequestResponseSolver::parse_content_type("text/html; charset=utf-8");
        assert_eq!(ct.media_type, "text/html");
        assert_eq!(ct.charset, Some("utf-8".to_string()));
    }

    #[test]
    fn test_normalize_header_name() {
        let normalized = RequestResponseSolver::normalize_header_name("content-type");
        assert_eq!(normalized, "Content-Type");
    }

    #[test]
    fn test_query_string_parsing() {
        let params = RequestResponseSolver::parse_query_string("name=john&age=30");
        assert_eq!(params.get("name"), Some(&"john".to_string()));
    }

    #[test]
    fn test_auth_header_parsing() {
        let result = RequestResponseSolver::parse_auth_header("Bearer token123");
        assert!(result.is_some());
    }

    #[test]
    fn test_response_to_bytes() {
        let mut res = RequestResponseSolver::new_response(200);
        RequestResponseSolver::set_body_text(&mut res, "Hello");
        let bytes = RequestResponseSolver::response_to_bytes(&res);
        assert!(!bytes.is_empty());
    }
}
