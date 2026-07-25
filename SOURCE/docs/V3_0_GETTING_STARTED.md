# Killer v3.0 Getting Started Guide

**Version**: 3.0.0  
**Latest Release**: March 14, 2026  
**Status**: ✅ Production-Ready

---

## Quick Start (5 Minutes)

### 1. Build Killer
```bash
cd src/v2-rust/killer_vm
cargo build --release
```

### 2. Run Your First Program
```bash
# Create a simple program
echo 'println("Hello, Killer v3.0!")' > hello.killer

# Run it
./target/release/killer_vm hello.killer
```

### 3. Try a v3.0 API
```killer
// DateTime example
let now = now()
let formatted = format_datetime(now, "%Y-%m-%d %H:%M:%S")
println("Current time: " + formatted)
```

---

## What's New in v3.0

### 5 Major API Modules

#### 1. **DateTime API** - Temporal Logic
```killer
// Get current time
let now = now()

// Format with patterns
let date_str = format_datetime(now, "%Y-%m-%d")
let time_str = format_datetime(now, "%H:%M:%S")

// Parse ISO 8601
let parsed = parse_datetime("2026-03-14T10:30:00")
```

#### 2. **HTTP API** - Web Connectivity
```killer
// GET request
let response = http_get("https://api.example.com/users")
let json = parse_json(response)

// POST request
let body = json_stringify(dict("name", "Alice", "age", 30))
let result = http_post("https://api.example.com/users", body)
```

#### 3. **JSON/CSV API** - Data Serialization
```killer
// Pretty-print JSON
let data = json_stringify(dict("a", 1, "b", 2))
println(json_pretty(data, 2))

// Parse CSV
let csv = "name,age\nAlice,30\nBob,25"
let rows = parse_csv(csv)
```

#### 4. **WebSocket API** - Real-time Communication
```killer
// Client
let ws = websocket_new("ws://localhost:8080")
let connected = ws_connect(ws)
ws_send(connected, "Hello!")
let msg = ws_receive(connected)

// Server
let server = websocket_server_new("0.0.0.0", 8080)
```

#### 5. **Trait System** - Polymorphism
```killer
// Check if type implements trait
if trait_check("Array", "Iterable") {
    println("Arrays are iterable")
}

// Resolve method through trait
let method = trait_resolve("String", "to_string")
println("String." + method["method"] + " (from " + method["trait"] + ")")
```

---

## Core Concepts

### DateTime Patterns

| Code | Meaning | Example |
|------|---------|---------|
| %Y | 4-digit year | 2026 |
| %m | 2-digit month | 03 |
| %d | 2-digit day | 14 |
| %H | 2-digit hour (24h) | 10 |
| %M | 2-digit minute | 30 |
| %S | 2-digit second | 45 |
| %A | Weekday name | Monday |
| %B | Month name | March |

### HTTP Methods
- `http_get(url)` - Fetch data from URL
- `http_post(url, body)` - Send data to URL
- `parse_json(string)` - Parse JSON to dict
- `json_stringify(dict)` - Convert dict to JSON

### JSON/CSV Functions
- `json_pretty(json, indent)` - Format JSON
- `parse_csv(csv, delimiter?)` - Parse CSV
- `to_csv(rows, delimiter?)` - Generate CSV
- `to_yaml(dict, indent?)` - Convert to YAML

### WebSocket Functions
- `websocket_new(url)` - Create client
- `websocket_server_new(host, port)` - Create server
- `ws_connect(ws)` - Establish connection
- `ws_send(ws, message)` - Send message
- `ws_receive(ws)` - Get message
- `ws_disconnect(ws)` - Close connection

### Trait Functions
- `trait_new(name, methods?)` - Define trait
- `trait_impl(trait, for_type)` - Implement trait
- `trait_check(type, trait)` - Check implementation
- `trait_resolve(type, method)` - Find method

---

## Common Patterns

### Pattern 1: Time-based Operations
```killer
let now = now()
let tomorrow = format_datetime(now, "%Y-%m-%d")
println("Processing data for: " + tomorrow)
```

### Pattern 2: REST API Integration
```killer
let url = "https://api.example.com/data"
let response = http_get(url)
let data = parse_json(response)

for item in data {
    println("Item: " + item["name"])
}
```

### Pattern 3: Data Export
```killer
// Fetch data
let rows = [
    dict("id", 1, "name", "Alice"),
    dict("id", 2, "name", "Bob")
]

// Export as CSV
let csv = to_csv(rows, ",")
println(csv)

// Export as JSON
let json = json_stringify(rows)
println(json_pretty(json, 2))
```

### Pattern 4: Real-time Chat
```killer
let server = websocket_server_new("0.0.0.0", 8080)
let client = websocket_new("ws://0.0.0.0:8080")
let conn = ws_connect(client)

// Send message
ws_send(conn, "Hello, server!")

// Receive message
let response = ws_receive(conn)
println(response["data"])

// Cleanup
ws_disconnect(conn)
```

### Pattern 5: Type-safe Polymorphism
```killer
let types = ["String", "Number", "Array"]

for t in types {
    if trait_check(t, "Display") {
        let method = trait_resolve(t, "to_string")
        println(t + " supports Display")
    }
}
```

---

## Example Programs

### 1. DateTime Scheduler
```killer
// examples/week23_03_datetime_scheduling.killer
let event_date = parse_datetime("2026-12-25T00:00:00")
let formatted = format_datetime(event_date, "%A, %B %d, %Y")
println("Event scheduled for: " + formatted)
```

### 2. REST API Client
```killer
// Demonstrates HTTP in examples/week23_05_http_post_api.killer
let api_url = "https://api.example.com/users"
let new_user = json_stringify(dict("name", "Charlie", "email", "charlie@example.com"))
let response = http_post(api_url, new_user)
println("Created: " + response)
```

### 3. Data Processing
```killer
// examples/week24_02_csv_parsing.killer
let csv_data = readFile("data.csv")
let rows = parse_csv(csv_data, ",")
let json_output = json_stringify(rows)
println(json_pretty(json_output, 2))
```

### 4. WebSocket Chat
```killer
// examples/week24_06_websocket_chat.killer
let server = websocket_server_new("127.0.0.1", 9000)
// Server broadcasting to all clients
```

### 5. Polymorphic Functions
```killer
// examples/week24_09_trait_objects.killer
// Dynamic method dispatch based on type
```

---

## API Reference by Category

### DateTime
| Function | Signature | Returns |
|----------|-----------|---------|
| `now()` | `now()` | number (Unix timestamp) |
| `parse_datetime()` | `parse_datetime(string)` | DateTime object |
| `format_datetime()` | `format_datetime(DateTime, string)` | string |

### HTTP
| Function | Signature | Returns |
|----------|-----------|---------|
| `http_get()` | `http_get(string)` | string (response body) |
| `http_post()` | `http_post(string, string)` | string (response body) |
| `parse_json()` | `parse_json(string)` | dict |
| `json_stringify()` | `json_stringify(dict)` | string |
| `HttpServer_new()` | `HttpServer_new(string, number)` | server object |
| `HttpServer_listen()` | `HttpServer_listen(server)` | void |

### JSON/CSV
| Function | Signature | Returns |
|----------|-----------|---------|
| `json_pretty()` | `json_pretty(string, number)` | string |
| `parse_csv()` | `parse_csv(string, string?)` | array[dict] |
| `to_csv()` | `to_csv(array, string?)` | string |
| `to_yaml()` | `to_yaml(dict, number?)` | string |

### WebSocket
| Function | Signature | Returns |
|----------|-----------|---------|
| `websocket_new()` | `websocket_new(string)` | WebSocket |
| `websocket_server_new()` | `websocket_server_new(string, number)` | Server |
| `ws_connect()` | `ws_connect(WebSocket)` | WebSocket |
| `ws_send()` | `ws_send(WebSocket, string)` | dict |
| `ws_receive()` | `ws_receive(WebSocket)` | dict |
| `ws_disconnect()` | `ws_disconnect(WebSocket)` | WebSocket |

### Traits
| Function | Signature | Returns |
|----------|-----------|---------|
| `trait_new()` | `trait_new(string, array?)` | Trait |
| `trait_impl()` | `trait_impl(string, string)` | Implementation |
| `trait_check()` | `trait_check(string, string)` | bool |
| `trait_resolve()` | `trait_resolve(string, string)` | dict |

---

## Built-in Traits

### Display Trait
**Purpose**: Convert types to strings
- **Method**: `to_string()`
- **Implemented by**: String, Number, Bool
- **Usage**: Automatically called when converting to string

### Comparable Trait
**Purpose**: Compare and order values
- **Methods**: `compare_to(other)`, `equals(other)`
- **Implemented by**: Number
- **Usage**: Sorting, equality checks

### Cloneable Trait
**Purpose**: Create independent copies
- **Method**: `clone()`
- **Implemented by**: String, Dict
- **Usage**: Deep copying values

### Iterable Trait
**Purpose**: Loop over collections
- **Methods**: `iterator()`, `has_next()`
- **Implemented by**: Array
- **Usage**: For loops, iteration

---

## Error Handling

All v3.0 functions validate inputs and provide clear error messages:

```killer
try {
    let bad_json = parse_json("not valid json")
} catch e {
    println("Error: " + str(e))
}
```

Common errors:
- `"parse_json() expects valid JSON string"` - Invalid JSON input
- `"websocket_new() expects URL string"` - Invalid WebSocket URL
- `"trait_check(): type not found"` - Unknown type
- `"http_get() failed to fetch"` - Network error (simulated in v3.0)

---

## Performance Tips

1. **Reuse connections**: Keep WebSocket connections open
2. **Cache trait checks**: Store trait resolution results
3. **Batch operations**: Process multiple rows at once
4. **Incremental parsing**: Stream large CSV files
5. **Format once**: Cache formatted timestamps

---

## Common Issues & Solutions

### Issue: "Unknown function"
**Solution**: Check capitalization, ensure module is imported

### Issue: DateTime formatting not working
**Solution**: Use correct pattern codes (%Y for year, %m for month, etc.)

### Issue: CSV parsing with quotes
**Solution**: Quote escaping handled automatically (RFC 4180 compliant)

### Issue: WebSocket connection failed
**Solution**: Verify address/port, check firewall settings

### Issue: Type doesn't implement trait
**Solution**: Check trait_check() result, trait may not be available for that type

---

## Next Steps

1. **Read Examples**: Check `examples/` directory for working programs
2. **Try APIs**: Experiment with each module in isolation
3. **Build Projects**: Create real applications using v3.0 features
4. **Check Docs**: Review detailed documentation for each module
5. **Join Community**: Get help and share feedback

---

## Additional Resources

- **Detailed Docs**: See `docs/WEEK_*.md` files
- **Examples**: Run programs in `examples/` directory
- **API Reference**: Consult this guide
- **Source Code**: Read `src/` directory comments
- **Release Notes**: See RELEASE_NOTES_V3_0.md

---

## Support

For questions or issues:
1. Check the FAQ in each module documentation
2. Review example programs
3. Verify with `cargo build` and `cargo test`
4. Report bugs with minimal reproduction case

---

**Welcome to Killer v3.0! Happy coding!** 🚀
