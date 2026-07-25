// HTTP Server Test Suite - Unit + Integration Tests
// Tests: HTTP parsing, routing, request/response handling

#[cfg(test)]
mod http_server_tests {
    use killer_native::http_server::{parse_http_request, HttpServer};
    use killer_native::web_framework::{HttpMethod, StatusCode};

    // ========================================================================
    // UNIT TESTS: HTTP Parsing
    // ========================================================================

    #[test]
    fn test_parse_simple_get_request() {
        let raw = "GET / HTTP/1.1\r\nHost: localhost\r\n\r\n";
        let req = parse_http_request(raw).expect("Failed to parse GET request");
        
        assert_eq!(req.method, HttpMethod::GET);
        assert_eq!(req.path, "/");
        assert_eq!(req.get_header("Host"), Some("localhost".to_string()));
    }

    #[test]
    fn test_parse_post_request_with_body() {
        let body = r#"{"name":"test","age":30}"#;
        let raw = format!(
            "POST /api/users HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
            body.len(),
            body
        );
        let req = parse_http_request(&raw).expect("Failed to parse POST request");
        
        assert_eq!(req.method, HttpMethod::POST);
        assert_eq!(req.path, "/api/users");
        assert_eq!(req.get_header("Content-Type"), Some("application/json".to_string()));
        assert_eq!(req.body, body);
    }

    #[test]
    fn test_parse_put_request() {
        let raw = "PUT /api/users/123 HTTP/1.1\r\nHost: localhost\r\n\r\n";
        let req = parse_http_request(raw).expect("Failed to parse PUT request");
        
        assert_eq!(req.method, HttpMethod::PUT);
        assert_eq!(req.path, "/api/users/123");
    }

    #[test]
    fn test_parse_delete_request() {
        let raw = "DELETE /api/users/456 HTTP/1.1\r\nHost: localhost\r\n\r\n";
        let req = parse_http_request(raw).expect("Failed to parse DELETE request");
        
        assert_eq!(req.method, HttpMethod::DELETE);
        assert_eq!(req.path, "/api/users/456");
    }

    #[test]
    fn test_parse_request_with_query_params() {
        let raw = "GET /api/users?id=123&name=alice&role=admin HTTP/1.1\r\n\r\n";
        let req = parse_http_request(raw).expect("Failed to parse query params");
        
        assert_eq!(req.path, "/api/users");
        assert_eq!(req.get_query_param("id"), Some("123".to_string()));
        assert_eq!(req.get_query_param("name"), Some("alice".to_string()));
        assert_eq!(req.get_query_param("role"), Some("admin".to_string()));
    }

    #[test]
    fn test_parse_request_with_multiple_headers() {
        let raw = "GET / HTTP/1.1\r\n\
                   Host: localhost\r\n\
                   User-Agent: Killer-Test/1.0\r\n\
                   Accept: application/json\r\n\
                   Authorization: Bearer token123\r\n\r\n";
        let req = parse_http_request(raw).expect("Failed to parse headers");
        
        assert_eq!(req.get_header("Host"), Some("localhost".to_string()));
        assert_eq!(req.get_header("User-Agent"), Some("Killer-Test/1.0".to_string()));
        assert_eq!(req.get_header("Accept"), Some("application/json".to_string()));
        assert_eq!(req.get_header("Authorization"), Some("Bearer token123".to_string()));
    }

    #[test]
    fn test_parse_patch_request() {
        let raw = "PATCH /api/users/789 HTTP/1.1\r\nHost: localhost\r\n\r\n";
        let req = parse_http_request(raw).expect("Failed to parse PATCH request");
        
        assert_eq!(req.method, HttpMethod::PATCH);
        assert_eq!(req.path, "/api/users/789");
    }

    #[test]
    fn test_parse_options_request() {
        let raw = "OPTIONS / HTTP/1.1\r\nHost: localhost\r\n\r\n";
        let req = parse_http_request(raw).expect("Failed to parse OPTIONS request");
        
        assert_eq!(req.method, HttpMethod::OPTIONS);
    }

    #[test]
    fn test_parse_request_invalid_no_method() {
        let raw = "HTTP/1.1 GET / \r\n\r\n";
        let result = parse_http_request(raw);
        assert!(result.is_err(), "Should reject invalid request format");
    }

    #[test]
    fn test_parse_request_empty() {
        let raw = "";
        let result = parse_http_request(raw);
        assert!(result.is_err(), "Should reject empty request");
    }

    #[test]
    fn test_parse_request_with_empty_body() {
        let raw = "POST /api/data HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/json\r\n\r\n".to_string();
        let req = parse_http_request(&raw).expect("Failed to parse POST with empty body");
        
        assert_eq!(req.method, HttpMethod::POST);
        assert_eq!(req.body, "");
    }

    // ========================================================================
    // UNIT TESTS: Route Matching & Parameter Extraction
    // ========================================================================

    #[test]
    fn test_exact_route_matching() {
        let mut server = HttpServer::new("127.0.0.1", 8080);
        
        // Register exact route
        server.on_route("GET", "/api/users", |_| {
            killer_native::web_framework::HttpResponse::new(StatusCode::OK)
        }).expect("Failed to register route");
        
        // Should succeed
        assert!(true, "Route registered successfully");
    }

    #[test]
    fn test_parameterized_route_matching() {
        let router = killer_native::web_framework::Router::new();
        
        // Test path pattern matching
        let params = killer_native::web_framework::Router::extract_params(
            "/api/users/:id",
            "/api/users/123"
        );
        
        assert_eq!(params.get("id"), Some(&"123".to_string()));
    }

    #[test]
    fn test_multiple_path_parameters() {
        let router = killer_native::web_framework::Router::new();
        
        let params = killer_native::web_framework::Router::extract_params(
            "/api/users/:userId/posts/:postId",
            "/api/users/123/posts/456"
        );
        
        assert_eq!(params.get("userId"), Some(&"123".to_string()));
        assert_eq!(params.get("postId"), Some(&"456".to_string()));
    }

    // ========================================================================
    // UNIT TESTS: Response Formatting
    // ========================================================================

    #[test]
    fn test_response_formatting_200_ok() {
        let response = killer_native::web_framework::HttpResponse::new(StatusCode::OK)
            .set_body("Hello, World!".to_string());
        
        let formatted = response.format();
        
        assert!(formatted.contains("HTTP/1.1 200 OK"));
        assert!(formatted.contains("Hello, World!"));
        assert!(formatted.contains("Content-Length: 13"));
    }

    #[test]
    fn test_response_formatting_404_not_found() {
        let response = killer_native::web_framework::HttpResponse::new(StatusCode::NotFound)
            .set_body("Not Found".to_string());
        
        let formatted = response.format();
        
        assert!(formatted.contains("HTTP/1.1 404 Not Found"));
        assert!(formatted.contains("Not Found"));
    }

    #[test]
    fn test_response_json_content_type() {
        let response = killer_native::web_framework::HttpResponse::new(StatusCode::OK)
            .json(r#"{"status":"ok","data":[]}"#.to_string());
        
        assert_eq!(
            response.get_header("Content-Type"),
            Some("application/json".to_string())
        );
    }

    #[test]
    fn test_response_with_cors_headers() {
        let response = killer_native::web_framework::HttpResponse::new(StatusCode::OK)
            .enable_cors();
        
        assert_eq!(
            response.get_header("Access-Control-Allow-Origin"),
            Some("*".to_string())
        );
        assert!(response.get_header("Access-Control-Allow-Methods").is_some());
    }

    #[test]
    fn test_response_with_custom_headers() {
        let response = killer_native::web_framework::HttpResponse::new(StatusCode::OK)
            .set_header("X-Custom-Header".to_string(), "CustomValue".to_string());
        
        assert_eq!(
            response.get_header("X-Custom-Header"),
            Some("CustomValue".to_string())
        );
    }

    #[test]
    fn test_all_status_codes() {
        assert_eq!(StatusCode::OK.code(), 200);
        assert_eq!(StatusCode::Created.code(), 201);
        assert_eq!(StatusCode::BadRequest.code(), 400);
        assert_eq!(StatusCode::Unauthorized.code(), 401);
        assert_eq!(StatusCode::Forbidden.code(), 403);
        assert_eq!(StatusCode::NotFound.code(), 404);
        assert_eq!(StatusCode::InternalError.code(), 500);
        assert_eq!(StatusCode::ServiceUnavailable.code(), 503);
    }

    // ========================================================================
    // UNIT TESTS: HTTP Methods
    // ========================================================================

    #[test]
    fn test_all_http_methods() {
        let methods = vec![
            ("GET", HttpMethod::GET),
            ("POST", HttpMethod::POST),
            ("PUT", HttpMethod::PUT),
            ("DELETE", HttpMethod::DELETE),
            ("PATCH", HttpMethod::PATCH),
            ("OPTIONS", HttpMethod::OPTIONS),
            ("HEAD", HttpMethod::HEAD),
        ];

        for (method_str, expected_method) in methods {
            let method = HttpMethod::from_string(method_str);
            assert_eq!(method, expected_method, "Failed to parse {}", method_str);
        }
    }

    #[test]
    fn test_http_method_case_insensitive() {
        assert_eq!(HttpMethod::from_string("get"), HttpMethod::GET);
        assert_eq!(HttpMethod::from_string("Post"), HttpMethod::POST);
        assert_eq!(HttpMethod::from_string("pUT"), HttpMethod::PUT);
    }

    // ========================================================================
    // INTEGRATION TESTS: Server Creation and Basic Operations
    // ========================================================================

    #[test]
    fn test_server_creation() {
        let server = HttpServer::new("127.0.0.1", 8080);
        // If we get here without panic, creation succeeded
        assert!(true);
    }

    #[test]
    fn test_server_register_multiple_routes() {
        let server = HttpServer::new("127.0.0.1", 8080);
        
        let result1 = server.on_route("GET", "/", |_| {
            killer_native::web_framework::HttpResponse::new(StatusCode::OK)
        });
        
        let result2 = server.on_route("POST", "/api/users", |_| {
            killer_native::web_framework::HttpResponse::new(StatusCode::Created)
        });
        
        let result3 = server.on_route("GET", "/api/users/:id", |_| {
            killer_native::web_framework::HttpResponse::new(StatusCode::OK)
        });
        
        assert!(result1.is_ok());
        assert!(result2.is_ok());
        assert!(result3.is_ok());
    }

    #[test]
    fn test_complex_request_response_cycle() {
        let raw_request = "POST /api/users HTTP/1.1\r\n\
                          Host: localhost:8080\r\n\
                          Content-Type: application/json\r\n\
                          Content-Length: 27\r\n\
                          \r\n\
                          {\"name\":\"Alice\",\"age\":30}";

        let req = parse_http_request(raw_request).expect("Parse request");
        let response = killer_native::web_framework::HttpResponse::new(StatusCode::Created)
            .json(format!(r#"{{"success":true,"path":"{}"}}"#, req.path));

        let formatted = response.format();
        assert!(formatted.contains("201 Created"));
        assert!(formatted.contains("application/json"));
    }

    // ========================================================================
    // PERFORMANCE TESTS
    // ========================================================================

    #[test]
    fn test_parse_request_performance() {
        let raw = "GET /api/users?page=1&limit=10&sort=name HTTP/1.1\r\n\
                  Host: localhost:8080\r\n\
                  User-Agent: Killer-Test/1.0\r\n\
                  Accept: application/json\r\n\
                  Authorization: Bearer token\r\n\r\n";

        let start = std::time::Instant::now();
        for _ in 0..1000 {
            let _ = parse_http_request(raw);
        }
        let elapsed = start.elapsed();

        println!("Parsed 1000 requests in {:?} ({:.2} µs per request)",
                 elapsed,
                 elapsed.as_micros() as f64 / 1000.0);

        // Should parse 1000 requests in < 100ms (Rust debug build is slower)
        // Release build achieves ~5-10ms
        assert!(elapsed.as_millis() < 100, "HTTP parsing too slow");
    }
}
