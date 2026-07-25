# Killer v3.0 API Quick Reference Card

**Version**: 3.0.0 | **Date**: March 14, 2026 | **Status**: Production-Ready

---

## DateTime Module (3 Functions)

### now() → number
```killer
let timestamp = now()  // Unix timestamp (seconds since epoch)
```

### parse_datetime(input: string) → DateTime
```killer
let dt = parse_datetime("2026-03-14T10:30:45")  // ISO 8601 format
```

### format_datetime(dt, pattern: string) → string
```killer
let formatted = format_datetime(now(), "%Y-%m-%d %H:%M:%S")
// Patterns: %Y=year, %m=month, %d=day, %H=hour, %M=minute, %S=second
//          %A=weekday, %B=month_name
```

---

## HTTP Module (6 Functions)

### http_get(url: string) → string
```killer
let response = http_get("https://api.example.com/data")
```

### http_post(url: string, body: string) → string
```killer
let json_body = json_stringify(dict("key", "value"))
let response = http_post("https://api.example.com/data", json_body)
```

### parse_json(json_string: string) → dict
```killer
let data = parse_json('{"name":"Alice","age":30}')
// Returns: {"name": "Alice", "age": "30"}
```

### json_stringify(dict) → string
```killer
let json = json_stringify(dict("a", 1, "b", 2))
// Returns: '{"a":1,"b":2}'
```

### HttpServer_new(host: string, port: number) → Server
```killer
let server = HttpServer_new("127.0.0.1", 8080)
```

### HttpServer_listen(server) → void
```killer
HttpServer_listen(server)  // Start listening for connections
```

---

## JSON/CSV Module (4 Functions)

### json_pretty(json: string, indent: number) → string
```killer
let pretty = json_pretty('{"a":1,"b":2}', 2)
// Returns formatted JSON with 2-space indentation
```

### parse_csv(csv: string, delimiter?: string) → array[dict]
```killer
let rows = parse_csv("name,age\nAlice,30\nBob,25")
// Returns: [{name: "Alice", age: "30"}, {name: "Bob", age: "25"}]
// Default delimiter: comma (,)
```

### to_csv(rows: array[dict], delimiter?: string) → string
```killer
let csv = to_csv([dict("name", "Alice"), dict("name", "Bob")])
// Returns: "name\nAlice\nBob"
```

### to_yaml(dict, indent?: number) → string
```killer
let yaml = to_yaml(dict("name", "Alice", "age", 30))
// Returns YAML format string
```

---

## WebSocket Module (6 Functions)

### websocket_new(url: string) → WebSocket
```killer
let ws = websocket_new("ws://localhost:8080")
```

### websocket_server_new(host: string, port: number) → Server
```killer
let server = websocket_server_new("0.0.0.0", 8080)
```

### ws_connect(ws: WebSocket) → WebSocket
```killer
let connected_ws = ws_connect(ws)  // Establish connection
```

### ws_send(ws: WebSocket, message: string) → dict
```killer
let result = ws_send(connected_ws, "Hello!")
// Returns: {status: "sent", message: "...", timestamp: "..."}
```

### ws_receive(ws: WebSocket) → dict
```killer
let msg = ws_receive(connected_ws)
// Returns: {type: "message", data: "...", timestamp: "..."}
```

### ws_disconnect(ws: WebSocket) → WebSocket
```killer
let disconnected = ws_disconnect(connected_ws)
```

---

## Trait System Module (4 Functions)

### trait_new(name: string, methods?: array) → Trait
```killer
let Display = trait_new("Display", ["to_string"])
```

### trait_impl(trait: string, for_type: string) → Implementation
```killer
let impl = trait_impl("Display", "String")
```

### trait_check(type: string, trait: string) → bool
```killer
let is_display = trait_check("String", "Display")  // true
let is_iterable = trait_check("String", "Iterable")  // false
```

### trait_resolve(type: string, method: string) → dict
```killer
let method = trait_resolve("Array", "iterator")
// Returns: {trait: "Iterable", method: "iterator", resolved: true}
```

---

## Built-in Traits Quick Reference

| Trait | Methods | Implementations |
|-------|---------|-----------------|
| **Display** | `to_string()` | String, Number, Bool |
| **Comparable** | `compare_to(other)`, `equals(other)` | Number |
| **Cloneable** | `clone()` | String, Dict |
| **Iterable** | `iterator()`, `has_next()` | Array |

---

## Common Patterns Cheat Sheet

### Get Current Time
```killer
let now = now()
let date_str = format_datetime(now, "%Y-%m-%d")
```

### Fetch & Parse JSON API
```killer
let response = http_get("https://api.example.com/data")
let data = parse_json(response)
```

### Send JSON Data
```killer
let payload = json_stringify(dict("name", "Alice"))
let response = http_post("https://api.example.com/users", payload)
```

### Parse CSV File
```killer
let csv_content = "col1,col2\nval1,val2"
let rows = parse_csv(csv_content, ",")
```

### Pretty Print JSON
```killer
let data = dict("a", 1, "b", 2)
let pretty = json_pretty(json_stringify(data), 2)
```

### Use WebSocket
```killer
let ws = websocket_new("ws://localhost:8080")
let connected = ws_connect(ws)
ws_send(connected, "message")
```

### Check Type Capabilities
```killer
if trait_check("MyType", "Display") {
    println("Type supports Display")
}
```

---

## Error Handling Template

```killer
try {
    // Try operation
    let data = parse_json(invalid_json)
} catch error {
    println("Error: " + str(error))
}
```

---

## Type System Reference

### Killer Value Types
- `Number` - 64-bit floating point
- `String` - UTF-8 encoded text
- `Bool` - true/false
- `Null` - null value
- `Array` - ordered collection [1,2,3]
- `Dict` - key-value mapping {a:1,b:2}
- `Closure` - first-class function

### Return Types Summary
| Module | Return Types |
|--------|--------------|
| DateTime | number, DateTime |
| HTTP | string, dict |
| JSON/CSV | string, array[dict], dict |
| WebSocket | WebSocket, Server, dict |
| Traits | Trait, bool, dict |

---

## Performance Notes

- **Trait Resolution**: O(1) cached lookup
- **JSON Parsing**: O(n) where n = JSON length
- **CSV Parsing**: O(m*k) where m = rows, k = columns
- **WebSocket**: Message queuing enabled
- **DateTime Formatting**: Cached pattern compilation

---

## Version Information

**Killer v3.0.0**
- **Release Date**: March 14, 2026
- **Coverage**: 80% of 150 topics
- **Modules**: 5 major (DateTime, HTTP, JSON/CSV, WebSocket, Traits)
- **Functions**: 23 builtin
- **Status**: Production-ready
- **Build**: gcc, clang, MSVC compatible

---

## Migration from v2.x

**No breaking changes!** All v2.x code works in v3.0.

New in v3.0:
- DateTime: `now()`, `parse_datetime()`, `format_datetime()`
- HTTP: `http_get()`, `http_post()`, `parse_json()`, `json_stringify()`
- HTTP Server: `HttpServer_new()`, `HttpServer_listen()`
- JSON/CSV: `json_pretty()`, `parse_csv()`, `to_csv()`, `to_yaml()`
- WebSocket: `websocket_new()`, `websocket_server_new()`, `ws_*` functions
- Traits: `trait_new()`, `trait_impl()`, `trait_check()`, `trait_resolve()`

---

## Quick Links

- **Full Documentation**: See `docs/` directory
- **Examples**: See `examples/` directory  
- **Getting Started**: V3_0_GETTING_STARTED.md
- **Release Notes**: RELEASE_NOTES_V3_0.md
- **Module Docs**: WEEK_*_COMPLETION.md files

---

**Keep this card handy for quick API reference!** 📋
