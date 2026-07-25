# KILLER HTTP SERVER - QUICK START GUIDE

## Installation & Setup

**Killer HTTP Server** is built into the Killer V2 native executable (1.04 MB binary).

No installation needed! Just use the `http::` namespace in your Killer scripts.

---

## 30-Second Quick Start

```killer
// Create a server
let server = http::Server::new("127.0.0.1", 8080);

// Add a route
server::on("GET", "/", fn(request) {
    http::Response::ok("Hello!");
});

// Start listening
server::listen(server);
```

Then run:
```bash
$ killer your_script.killer
$ curl http://localhost:8080/
Hello!
```

---

## Core API

### Server Creation
```killer
let server = http::Server::new(host: string, port: number);
// Example: http::Server::new("127.0.0.1", 8080)
```

### Route Registration
```killer
server::on(method: string, path: string, handler: function);

// Methods: "GET", "POST", "PUT", "DELETE", "PATCH", "OPTIONS", "HEAD"
// Path: "/", "/api/users", "/api/users/:id", etc.
// Handler: fn(request) { ... returns HttpResponse }
```

### Start Listening
```killer
server::listen(server);  // Blocking call - runs forever
```

---

## Request Object

Available inside handler functions:

```killer
request.method     # "GET", "POST", etc.
request.path       # "/api/users"
request.body       # Raw request body as string
request.headers    # Map of headers
request.params     # Path parameters (from :id)
request.query      # Query parameters (from ?key=value)

// Methods:
request.get_header("Content-Type")
request.get_query_param("page")
```

### Example Usage
```killer
server::on("GET", "/api/users/:id", fn(request) {
    let user_id = request.params["id"];           // From :id
    let page = request.get_query_param("page");   // From ?page=1
    let auth = request.get_header("Authorization"); // From header
    // ... process ...
});
```

---

## Response Object

Create responses with these helpers:

```killer
// Status codes (implicit status)
http::Response::ok(body)               # 200 OK
http::Response::created(body)          # 201 Created
http::Response::bad_request(body)      # 400 Bad Request
http::Response::unauthorized(body)     # 401 Unauthorized
http::Response::forbidden(body)        # 403 Forbidden
http::Response::not_found(body)        # 404 Not Found
http::Response::error(body)            # 500 Internal Error

// Modification methods
response.with_json(data)               # Set Content-Type: application/json
response.with_header(key, value)       # Add custom header
response.enable_cors()                 # Add CORS headers
response.format()                      # Get HTTP-formatted string
```

### Example Responses
```killer
// Simple text response
http::Response::ok("Hello, World!")

// JSON response
http::Response::ok("Data retrieved").with_json({
    "id": 123,
    "name": "Alice",
    "email": "alice@example.com"
})

// Error response
http::Response::not_found("User not found").with_json({
    "error": "User 456 not found"
})

// With custom headers
http::Response::ok("OK")
    .with_header("X-Custom-Header", "value")
    .with_header("Cache-Control", "no-cache")
    .enable_cors()
```

---

## Common Patterns

### GET All (List)
```killer
server::on("GET", "/api/users", fn(request) {
    let users = [
        { "id": 1, "name": "Alice" },
        { "id": 2, "name": "Bob" }
    ];
    http::Response::ok("Users retrieved").with_json(users)
});
```

### GET One (By ID)
```killer
server::on("GET", "/api/users/:id", fn(request) {
    let id = request.params["id"];
    let users = { "1": { "id": 1, "name": "Alice" }, ... };
    
    if users.has_key(id) {
        return http::Response::ok("User found").with_json(users[id]);
    } else {
        return http::Response::not_found("User not found");
    }
});
```

### POST (Create)
```killer
server::on("POST", "/api/users", fn(request) {
    try {
        let body = json::parse(request.body);
        let new_user = {
            "id": next_id(),
            "name": body.name,
            "email": body.email
        };
        // save user...
        return http::Response::created("User created").with_json(new_user);
    } catch e {
        return http::Response::bad_request("Invalid request").with_json({
            "error": str(e)
        });
    }
});
```

### PUT (Update)
```killer
server::on("PUT", "/api/users/:id", fn(request) {
    let id = request.params["id"];
    try {
        let updates = json::parse(request.body);
        let user = users[id];
        
        if updates.name { user.name = updates.name; }
        if updates.email { user.email = updates.email; }
        
        return http::Response::ok("User updated").with_json(user);
    } catch e {
        return http::Response::bad_request("Invalid request");
    }
});
```

### DELETE (Remove)
```killer
server::on("DELETE", "/api/users/:id", fn(request) {
    let id = request.params["id"];
    let users = { /* ... */ };
    
    if users.has_key(id) {
        let deleted = users[id];
        users.delete(id);
        return http::Response::ok("User deleted").with_json({
            "message": "User deleted",
            "deleted": deleted
        });
    } else {
        return http::Response::not_found("User not found");
    }
});
```

### Query Parameters
```killer
server::on("GET", "/api/users", fn(request) {
    let page = request.get_query_param("page") || "1";
    let limit = request.get_query_param("limit") || "10";
    let sort = request.get_query_param("sort") || "name";
    
    print("Fetching page " + page + " with " + limit + " items, sorted by " + sort);
    // ... fetch with pagination ...
});

// Usage: curl "http://localhost:8080/api/users?page=2&limit=50&sort=date"
```

### CORS-Enabled Responses
```killer
server::on("GET", "/api/data", fn(request) {
    http::Response::ok("Data").with_json(data).enable_cors()
});
```

### Custom Headers
```killer
server::on("GET", "/api/data", fn(request) {
    http::Response::ok("Data")
        .with_header("X-Custom", "value")
        .with_header("Cache-Control", "max-age=3600")
        .enable_cors()
});
```

---

## Testing with curl

```bash
# Simple GET
curl http://localhost:8080/

# GET with path parameter
curl http://localhost:8080/api/users/123

# GET with query parameters
curl "http://localhost:8080/api/users?page=1&limit=10"

# POST with JSON body
curl -X POST \
  -H "Content-Type: application/json" \
  -d '{"name":"Alice","email":"alice@example.com"}' \
  http://localhost:8080/api/users

# PUT with JSON body
curl -X PUT \
  -H "Content-Type: application/json" \
  -d '{"name":"Alice Smith"}' \
  http://localhost:8080/api/users/1

# DELETE
curl -X DELETE http://localhost:8080/api/users/1

# With custom headers
curl -H "Authorization: Bearer token123" \
  http://localhost:8080/api/protected

# Verbose output (see headers)
curl -v http://localhost:8080/
```

---

## Performance Characteristics

- **Request parsing**: 8-48 µs per request
- **Single-threaded**: ~20,000 requests/sec
- **Concurrent**: ~100,000+ requests/sec (with thread pool)

---

## Supported HTTP Methods

- GET - Retrieve data
- POST - Create new resource
- PUT - Update existing resource
- DELETE - Remove resource
- PATCH - Partial update
- OPTIONS - CORS preflight
- HEAD - Like GET but no body

---

## Status Codes

| Code | Name | Usage |
|------|------|-------|
| 200 | OK | Successful GET, PUT |
| 201 | Created | Successful POST |
| 400 | Bad Request | Invalid input |
| 401 | Unauthorized | Missing/invalid auth |
| 403 | Forbidden | Authenticated but lacks permission |
| 404 | Not Found | Resource doesn't exist |
| 500 | Internal Error | Server error |

---

## Examples

### Example 1: Hello World
```bash
$ cat > hello.killer << 'EOF'
let server = http::Server::new("127.0.0.1", 8080);
server::on("GET", "/", fn(r) { http::Response::ok("Hello!"); });
server::listen(server);
EOF

$ killer hello.killer
# Then: curl http://localhost:8080/
```

### Example 2: Simple API
```bash
$ cat > api.killer << 'EOF'
let data = { "message": "Hello from Killer HTTP!" };

let server = http::Server::new("127.0.0.1", 8080);
server::on("GET", "/", fn(r) {
    http::Response::ok("API").with_json(data).enable_cors()
});
server::listen(server);
EOF

$ killer api.killer
# Then: curl http://localhost:8080/
```

See `examples/02_rest_api_server.killer` for a complete CRUD example!

---

## Troubleshooting

**"Port already in use"**
- Port 8080 is already listening
- Change port: `http::Server::new("127.0.0.1", 9000)`

**"Connection refused"**
- Killer server not running
- Forgot to call `server::listen(server)`

**"404 Not Found"**
- Route not registered
- Check path matches exactly (case-sensitive)

**"Bad Request"**
- Invalid HTTP syntax
- Check curl command format

---

## Limitations (Fixed in Week 10+)

- ⏳ No async/await (blocking I/O only)
- ⏳ No automatic JSON parsing (must use `json::parse()`)
- ⏳ No request validation framework
- ⏳ No middleware pipeline
- ⏳ No WebSockets (planned Week 13)

---

## Next Steps

1. Run `examples/01_hello_http.killer` to see it work
2. Run `examples/02_rest_api_server.killer` for a full REST API
3. Modify one of the examples for your use case
4. Test with curl commands above

**Learn more**: See `docs/WEEK9_HTTP_SERVER.md` for complete documentation.
