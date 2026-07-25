# Killer v3.0 Tutorial Guide

**Practical Examples for Every API | Step-by-Step Walkthroughs**

---

## Table of Contents

1. [DateTime API Tutorial](#datetime-api-tutorial)
2. [HTTP Framework Tutorial](#http-framework-tutorial)
3. [JSON/CSV Tutorial](#jsoncsv-tutorial)
4. [WebSocket Tutorial](#websocket-tutorial)
5. [Trait System Tutorial](#trait-system-tutorial)
6. [Complete Project Example](#complete-project-example)

---

## DateTime API Tutorial

### 1️⃣ Getting Current Time

**Use Case**: Log timestamps for events, measure elapsed time

```killer
// Get current Unix timestamp
let current_time = now()
println("Current timestamp: " + str(current_time))

// Output: Current timestamp: 1710432000.123456
```

**How It Works**:
- `now()` returns current time as a number (Unix timestamp)
- Includes milliseconds precision
- Suitable for all timing needs

### 2️⃣ Parsing Dates

**Use Case**: Convert user input dates into timestamps

```killer
// Parse a date string
let date_str = "2024-03-14"
let parsed = parse_datetime(date_str, "%Y-%m-%d")
println("Parsed date: " + str(parsed))

// Output: Parsed date: 1710374400
```

**Supported Format Codes**:
- `%Y` — 4-digit year (2024)
- `%m` — 2-digit month (03)
- `%d` — 2-digit day (14)
- `%H` — Hour (0-23)
- `%M` — Minute (0-59)
- `%S` — Second (0-59)

**Common Patterns**:
```killer
// ISO format
let iso = parse_datetime("2024-03-14T15:30:45", "%Y-%m-%dT%H:%M:%S")

// US format
let us = parse_datetime("03/14/2024", "%m/%d/%Y")

// Long format
let long = parse_datetime("March 14, 2024", "%B %d, %Y")
```

### 3️⃣ Formatting Timestamps

**Use Case**: Display dates in user-friendly format

```killer
let timestamp = now()

// Format as date
let date_str = format_datetime(timestamp, "%Y-%m-%d")
println("Today: " + date_str)  // Today: 2024-03-14

// Format with time
let datetime_str = format_datetime(timestamp, "%Y-%m-%d %H:%M:%S")
println("Now: " + datetime_str)  // Now: 2024-03-14 15:30:45

// Day of week
let day_str = format_datetime(timestamp, "%A")
println("Day: " + day_str)  // Day: Thursday

// Month name
let month_str = format_datetime(timestamp, "%B")
println("Month: " + month_str)  // Month: March
```

**Common Patterns**:
```killer
// Simple date: 03/14/2024
let short = format_datetime(timestamp, "%m/%d/%Y")

// Readable: March 14, 2024
let readable = format_datetime(timestamp, "%B %d, %Y")

// Log format: 2024-03-14 15:30
let log = format_datetime(timestamp, "%Y-%m-%d %H:%M")

// Full with day: Thursday, March 14, 2024
let full = format_datetime(timestamp, "%A, %B %d, %Y")
```

### 4️⃣ Measuring Elapsed Time

**Use Case**: Performance measurement, timing operations

```killer
// Start timer
let start = now()

// Do some work...
let i = 0
while i < 1000 {
    let _ = i * 2
    let i = i + 1
}

// End timer
let end = now()
let elapsed = end - start

println("Operation took: " + str(elapsed) + " seconds")
```

**Complete Example**:
```killer
fn measure_performance(iterations: number) {
    let start = now()
    
    // Simulate work
    let i = 0
    while i < iterations {
        let result = i * i
        let i = i + 1
    }
    
    let end = now()
    let elapsed = end - start
    let per_op = elapsed / iterations * 1000  // Convert to microseconds
    
    println("Total time: " + str(elapsed) + "s")
    println("Operations: " + str(iterations))
    println("Time per op: " + str(per_op) + "µs")
}

measure_performance(100000)
```

---

## HTTP Framework Tutorial

### 1️⃣ Making GET Requests

**Use Case**: Fetch data from APIs, retrieve web content

```killer
// Simple GET request
let response = http_get("https://api.example.com/users")

println("Response received")
println("Status: " + str(response))  // Mock returns success indicator
```

**Working with JSON Responses**:
```killer
// Get JSON data
let response = http_get("https://api.example.com/user/123")

// Parse the response
let user = parse_json(response)

// Access fields
println("User ID: " + str(user["id"]))
println("Name: " + str(user["name"]))
```

### 2️⃣ Making POST Requests

**Use Case**: Submit data to APIs, send form data

```killer
// Simple POST
let payload = {
    "username": "alice",
    "email": "alice@example.com",
    "active": true
}

let response = http_post("https://api.example.com/users", payload)
println("User created: " + str(response))
```

**Complex POST Example**:
```killer
// Create event with nested data
let event = {
    "title": "Team Meeting",
    "date": "2024-03-14",
    "time": "14:00",
    "attendees": [
        {"name": "Alice", "email": "alice@example.com"},
        {"name": "Bob", "email": "bob@example.com"}
    ],
    "location": {
        "city": "San Francisco",
        "building": "HQ",
        "room": "101"
    }
}

let response = http_post("https://api.example.com/events", event)
println("Event created")
```

### 3️⃣ JSON Serialization

**Use Case**: Convert data to JSON for transmission

```killer
// Simple object
let person = {
    "id": 1,
    "name": "Alice",
    "role": "Engineer"
}

let json = json_stringify(person)
println(json)
// Output: {"id":1,"name":"Alice","role":"Engineer"}
```

**Pretty-Printing for Readability**:
```killer
let person = {
    "id": 1,
    "name": "Alice",
    "address": {
        "street": "123 Main St",
        "city": "San Francisco"
    }
}

// Pretty print with 2-space indent
let pretty = json_pretty(person, 2)
println(pretty)

/* Output:
{
  "id": 1,
  "name": "Alice",
  "address": {
    "street": "123 Main St",
    "city": "San Francisco"
  }
}
*/
```

### 4️⃣ Building a Simple HTTP Server

**Use Case**: Create API endpoints, handle requests

```killer
// Create server
let server = HttpServer_new("127.0.0.1:8000")
println("Server created on http://127.0.0.1:8000")

// Listen for requests (in real implementation)
let result = HttpServer_listen(server)
println("Server listening...")
```

**Practical Server Example**:
```killer
fn start_api_server() {
    // Create server
    let server = HttpServer_new("127.0.0.1:3000")
    println("API Server starting on http://127.0.0.1:3000")
    
    // Start listening
    let listening = HttpServer_listen(server)
    
    if listening {
        println("✓ Server running")
        println("  GET  /api/users")
        println("  POST /api/users")
        println("  GET  /api/status")
    } else {
        println("✗ Failed to start server")
    }
}

start_api_server()
```

### 5️⃣ Complete Request/Response Cycle

**Use Case**: Realistic API usage pattern

```killer
fn handle_user_creation() {
    // Prepare request
    let user_data = {
        "name": "Charlie",
        "email": "charlie@example.com",
        "registration_date": now()
    }
    
    // Send POST request
    let response = http_post("https://api.example.com/users", user_data)
    
    // Parse response
    let result = parse_json(response)
    
    // Prepare response object
    let api_response = {
        "success": true,
        "message": "User created",
        "user_id": result["id"],
        "timestamp": now()
    }
    
    // Return as JSON
    let json_response = json_stringify(api_response)
    return json_response
}

println(handle_user_creation())
```

---

## JSON/CSV Tutorial

### 1️⃣ Parsing JSON

**Use Case**: Load configuration files, process API responses

```killer
// Parse simple JSON
let json_str = "{\"name\": \"Alice\", \"age\": 30}"
let person = parse_json(json_str)

println("Name: " + str(person["name"]))
println("Age: " + str(person["age"]))
```

**Complex JSON**:
```killer
let config_json = "{
  \"server\": {
    \"host\": \"localhost\",
    \"port\": 8000
  },
  \"database\": {
    \"type\": \"postgres\",
    \"connections\": 10
  }
}"

let config = parse_json(config_json)
let host = config["server"]["host"]
let port = config["server"]["port"]

println("Connecting to " + host + ":" + str(port))
```

### 2️⃣ Working with CSV Data

**Use Case**: Process spreadsheets, import/export data

```killer
// Parse CSV data
let csv_str = "id,name,email
1,Alice,alice@example.com
2,Bob,bob@example.com
3,Charlie,charlie@example.com"

let rows = parse_csv(csv_str, ",")

// Process each row
let i = 0
while i < len(rows) {
    let row = rows[i]
    println("ID: " + row[0] + ", Name: " + row[1] + ", Email: " + row[2])
    let i = i + 1
}
```

**With Different Delimiters**:
```killer
// Tab-separated values
let tsv = "id\tname\temail
1\tAlice\talice@example.com
2\tBob\tbob@example.com"

let rows = parse_csv(tsv, "\t")
println("Parsed " + str(len(rows)) + " rows")

// Pipe-separated values
let psv = "product|price|quantity
Widget|9.99|100
Gadget|19.99|50"

let data = parse_csv(psv, "|")
```

### 3️⃣ Converting to CSV

**Use Case**: Export data to spreadsheet format

```killer
// Array of user data
let users = [
    ["1", "Alice", "Engineer"],
    ["2", "Bob", "Manager"],
    ["3", "Charlie", "Analyst"]
]

// Convert to CSV
let csv = to_csv(users, ",")
println(csv)

/* Output:
1,Alice,Engineer
2,Bob,Manager
3,Charlie,Analyst
*/
```

**With Headers**:
```killer
fn export_users_to_csv(users_array) {
    // Prepare data with headers
    let data = [
        ["ID", "Name", "Title"],  // Header row
        ["1", "Alice", "Engineer"],
        ["2", "Bob", "Manager"],
        ["3", "Charlie", "Analyst"]
    ]
    
    // Convert to CSV
    let csv = to_csv(data, ",")
    
    // Could save to file here
    return csv
}
```

### 4️⃣ Pretty-Printing JSON

**Use Case**: Debug output, save readable configurations

```killer
let config = {
    "app": {
        "name": "MyApp",
        "version": "3.0.0"
    },
    "features": [
        "DateTime API",
        "HTTP Framework",
        "JSON/CSV Support",
        "WebSocket",
        "Trait System"
    ]
}

// Different indent levels
println("2-space indent:")
println(json_pretty(config, 2))

println("\n4-space indent:")
println(json_pretty(config, 4))

println("\nTab indent:")
let tab_version = json_pretty(config, "\t")
println(tab_version)
```

### 5️⃣ Data Format Conversion

**Use Case**: Migrate between formats, transform data

```killer
fn convert_csv_to_json(csv_string) {
    // Parse CSV
    let rows = parse_csv(csv_string, ",")
    
    // Get headers
    let headers = rows[0]
    
    // Convert to JSON objects
    let json_data = []
    let i = 1
    while i < len(rows) {
        let row = rows[i]
        let obj = {}
        
        // Build object from headers and values
        let j = 0
        while j < len(headers) {
            let obj[headers[j]] = row[j]
            let j = j + 1
        }
        
        let json_data = json_data + [obj]
        let i = i + 1
    }
    
    return json_stringify(json_data)
}

let csv = "id,name,email
1,Alice,alice@example.com
2,Bob,bob@example.com"

let json = convert_csv_to_json(csv)
println(json)
```

### 6️⃣ YAML Export

**Use Case**: Configuration files, readable data dumps

```killer
let settings = {
    "database": {
        "host": "localhost",
        "port": 5432,
        "name": "myapp_db"
    },
    "logging": {
        "level": "info",
        "output": "file"
    }
}

let yaml = to_yaml(settings)
println(yaml)

/* Output format:
database:
  host: localhost
  port: 5432
  name: myapp_db
logging:
  level: info
  output: file
*/
```

---

## WebSocket Tutorial

### 1️⃣ Creating WebSocket Client

**Use Case**: Connect to real-time services

```killer
// Create a client socket
let ws = websocket_new("ws://localhost:8000/chat")
println("WebSocket client created")

// Connect to server
let connected = ws_connect(ws)
if connected {
    println("✓ Connected to server")
} else {
    println("✗ Connection failed")
}
```

### 2️⃣ Sending Messages

**Use Case**: Send real-time updates

```killer
let ws = websocket_new("ws://localhost:8000/updates")
let conn = ws_connect(ws)

if conn {
    // Send text message
    let sent = ws_send(ws, "Hello from Killer!")
    
    // Send JSON message
    let message = {
        "type": "status",
        "status": "online",
        "timestamp": now()
    }
    let json_msg = json_stringify(message)
    let sent2 = ws_send(ws, json_msg)
    
    println("Messages sent")
}
```

### 3️⃣ Receiving Messages

**Use Case**: Handle real-time updates

```killer
let ws = websocket_new("ws://localhost:8000/notifications")
let conn = ws_connect(ws)

if conn {
    // Receive message
    let message = ws_receive(ws)
    println("Received: " + message)
    
    // Parse if JSON
    let data = parse_json(message)
    println("Message type: " + str(data["type"]))
}
```

### 4️⃣ WebSocket Server

**Use Case**: Create real-time endpoints

```killer
// Create server
let server = websocket_server_new("127.0.0.1:8000")
println("WebSocket server created on ws://127.0.0.1:8000")

// Listen for connections
let listening = HttpServer_listen(server)  // In real impl, would be ws_listen
println("✓ Server listening for connections")
```

### 5️⃣ Chat Application Example

**Use Case**: Real-time messaging

```killer
fn simple_chat_client(username) {
    // Connect to chat server
    let ws = websocket_new("ws://chat.example.com/chat")
    let connected = ws_connect(ws)
    
    if !connected {
        println("Failed to connect to chat")
        return
    }
    
    // Send join message
    let join_msg = {
        "type": "join",
        "username": username,
        "timestamp": now()
    }
    let sent = ws_send(ws, json_stringify(join_msg))
    println("Joined chat as " + username)
    
    // Listen for messages
    let msg = ws_receive(ws)
    let data = parse_json(msg)
    
    if data["type"] == "message" {
        println(data["username"] + ": " + data["text"])
    }
    
    // Disconnect
    let disconnected = ws_disconnect(ws)
    println("Disconnected from chat")
}

simple_chat_client("Alice")
```

### 6️⃣ Multi-Client Scenario

**Use Case**: Broadcasting updates to multiple clients

```killer
fn broadcast_update(update_data) {
    // Connect multiple clients
    let clients = []
    let i = 0
    while i < 5 {
        let ws = websocket_new("ws://localhost:8000/broadcast")
        let conn = ws_connect(ws)
        let clients = clients + [ws]
        let i = i + 1
    }
    
    // Broadcast to all clients
    let update_msg = json_stringify(update_data)
    let j = 0
    while j < len(clients) {
        let sent = ws_send(clients[j], update_msg)
        let j = j + 1
    }
    
    println("Broadcast sent to " + str(len(clients)) + " clients")
    
    // Cleanup
    let k = 0
    while k < len(clients) {
        let disc = ws_disconnect(clients[k])
        let k = k + 1
    }
}

let update = {
    "type": "price_update",
    "product": "Widget",
    "price": 9.99,
    "timestamp": now()
}

broadcast_update(update)
```

---

## Trait System Tutorial

### 1️⃣ Defining Traits

**Use Case**: Define interfaces for polymorphism

```killer
// Define a Display trait
let display_trait = trait_new("Display", [
    "to_string",
    "format"
])

println("Display trait defined")
```

**Practical Example**:
```killer
// Define a Comparable trait for ordering
let comparable = trait_new("Comparable", [
    "compare_to",
    "is_equal",
    "is_less_than",
    "is_greater_than"
])

// Define a Serializable trait
let serializable = trait_new("Serializable", [
    "to_json",
    "from_json",
    "to_csv"
])
```

### 2️⃣ Implementing Traits

**Use Case**: Add trait support to types

```killer
// Implement Display for String type
let _ = trait_impl("Display", "String")

// Implement Comparable for Number type
let _ = trait_impl("Comparable", "Number")

// Implement Serializable for Dict type
let _ = trait_impl("Serializable", "Dict")
```

### 3️⃣ Checking Trait Implementation

**Use Case**: Verify capability before using

```killer
// Check if String implements Display
let is_displayable = trait_check("String", "Display")

if is_displayable {
    println("✓ String supports Display trait")
} else {
    println("✗ String does not support Display trait")
}

// Check multiple implementations
let types = ["String", "Number", "Array", "Dict"]
let i = 0
while i < len(types) {
    let type_name = types[i]
    if trait_check(type_name, "Display") {
        println(type_name + " supports Display")
    }
    let i = i + 1
}
```

### 4️⃣ Resolving Methods

**Use Case**: Get method information at runtime

```killer
// Resolve Display method for String
let to_str_method = trait_resolve("String", "to_string")

if to_str_method != null {
    println("✓ String.to_string() method found")
} else {
    println("✗ Method not found")
}
```

### 5️⃣ Polymorphic Function Example

**Use Case**: Write generic functions that work with multiple types

```killer
fn display_object(obj, type_name) {
    // Check if type supports Display trait
    if trait_check(type_name, "Display") {
        // Resolve the method
        let method = trait_resolve(type_name, "to_string")
        
        if method != null {
            println("✓ Can display " + type_name)
            return str(obj)
        }
    }
    
    println("✗ " + type_name + " does not support Display")
    return null
}

// Use with different types
let name = "Alice"
let age = 30
let scores = [95, 87, 92]

println(display_object(name, "String"))
println(display_object(age, "Number"))
println(display_object(scores, "Array"))
```

### 6️⃣ Complete Polymorphic Workflow

**Use Case**: Type-safe polymorphism with trait bounds

```killer
fn process_comparable_objects(objects, type_name) {
    // Verify type supports comparison
    let is_comparable = trait_check(type_name, "Comparable")
    
    if !is_comparable {
        println("✗ " + type_name + " does not support Comparable trait")
        return
    }
    
    // Resolve comparison method
    let compare_method = trait_resolve(type_name, "compare_to")
    
    if compare_method == null {
        println("✗ Comparison method not found")
        return
    }
    
    println("✓ Type " + type_name + " supports comparison")
    println("  Objects: " + str(objects))
    println("  Ready for sorting/comparison operations")
}

// Test with different types
process_comparable_objects([1, 2, 3], "Array")
process_comparable_objects([3.14, 2.71, 1.41], "Array")
process_comparable_objects(["apple", "banana", "cherry"], "Array")
```

---

## Complete Project Example

### Real-World: User Management API with WebSocket

```killer
// Complete user management system with real-time updates

fn create_api_server() {
    // Create server
    let server = HttpServer_new("127.0.0.1:3000")
    println("User Management API Server")
    println("=" * 40)
    
    // Define data
    let users = [
        {"id": 1, "name": "Alice", "role": "Admin"},
        {"id": 2, "name": "Bob", "role": "User"},
        {"id": 3, "name": "Charlie", "role": "Moderator"}
    ]
    
    // API endpoints
    println("POST   /api/users          - Create new user")
    println("GET    /api/users          - List all users")
    println("GET    /api/users/{id}     - Get user details")
    println("")
    
    // WebSocket endpoint
    println("WS     /ws/notifications   - Real-time updates")
    println("")
    
    // Start server
    let listening = HttpServer_listen(server)
    
    if listening {
        println("✓ Server running on http://127.0.0.1:3000")
    }
}

fn handle_create_user(request_body) {
    // Parse request
    let user_data = parse_json(request_body)
    
    // Validate
    if user_data["name"] == null || user_data["role"] == null {
        return json_stringify({
            "success": false,
            "error": "Missing required fields"
        })
    }
    
    // Create user
    let new_user = {
        "id": now(),  // Use timestamp as ID
        "name": user_data["name"],
        "role": user_data["role"],
        "created": now()
    }
    
    // Return response
    return json_stringify({
        "success": true,
        "user": new_user,
        "message": "User created"
    })
}

fn notify_subscribers(event_type, event_data) {
    // Create notification
    let notification = {
        "type": event_type,
        "data": event_data,
        "timestamp": now()
    }
    
    // Connect to notification server
    let ws = websocket_new("ws://localhost:3000/ws/notifications")
    let connected = ws_connect(ws)
    
    if connected {
        // Send notification
        let msg = json_stringify(notification)
        let sent = ws_send(ws, msg)
        
        if sent {
            println("✓ Notification sent: " + event_type)
        }
        
        // Disconnect
        let _ = ws_disconnect(ws)
    }
}

// Main execution
fn main() {
    println("Starting User Management System v3.0")
    println("Features: DateTime, HTTP, JSON/CSV, WebSocket, Traits")
    println("")
    
    // Start API server
    create_api_server()
    println("")
    
    // Test user creation
    let new_user = {
        "name": "Diana",
        "role": "Developer"
    }
    
    let response = handle_create_user(json_stringify(new_user))
    println("Create response: " + response)
    println("")
    
    // Notify subscribers
    notify_subscribers("user_created", new_user)
}

main()
```

---

## Practice Exercises

### Exercise 1: Date Range Reporting
Create a function that generates a CSV report of events within a date range.

```killer
fn generate_event_report(start_date, end_date) {
    // Parse dates
    let start = parse_datetime(start_date, "%Y-%m-%d")
    let end = parse_datetime(end_date, "%Y-%m-%d")
    
    // TODO: Filter events, create CSV report
}
```

### Exercise 2: JSON Configuration Manager
Build system that reads, validates, and updates JSON config files.

```killer
fn load_config(filename) {
    // TODO: Read file, parse JSON, return config object
}

fn validate_config(config) {
    // TODO: Check required fields, return validation result
}
```

### Exercise 3: WebSocket Message Router
Create router that distributes events to multiple clients.

```killer
fn route_message(message, target_clients) {
    // TODO: Parse message, verify clients, send to all
}
```

---

## Summary

You now have practical knowledge of:

- ✅ **DateTime API**: Timestamping, parsing, formatting, measuring time
- ✅ **HTTP Framework**: GET/POST requests, JSON serialization, servers
- ✅ **JSON/CSV**: Parsing, conversion, pretty-printing, YAML export
- ✅ **WebSocket**: Client/server, sending/receiving, broadcasting
- ✅ **Trait System**: Definition, implementation, checking, resolution

**Next Steps**:
1. Run the example programs in `examples/`
2. Modify examples to experiment
3. Combine APIs in your own projects
4. Check [V3_0_API_QUICK_REFERENCE.md](V3_0_API_QUICK_REFERENCE.md) for detailed function signatures
5. Review [V3_0_BENCHMARK_SUMMARY.md](V3_0_BENCHMARK_SUMMARY.md) for performance considerations

Happy coding! 🚀
