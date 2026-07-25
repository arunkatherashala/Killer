# 🚀 KILLER IMPLEMENTATION PLAN - WEEKS 23-24
**Target**: Close critical gaps in Web APIs, DateTime, and Serialization  
**Status**: Ready to Execute  
**Updated**: March 14, 2026

---

## 📊 SUMMARY TABLE

| Feature | Current | Target | Effort | Timeline |
|---------|---------|--------|--------|----------|
| **Date/Time APIs** | 0% | 100% | 1-2 days | Week 23a |
| **HTTP Framework** | 0% | 80% | 3-4 days | Week 23b |
| **JSON/Serialization** | 50% | 100% | 2-3 days | Week 24a |
| **WebSockets** | 0% | 60% | 2-3 days | Week 24b |
| **Simple Trait System** | 0% | 40% | 3-4 days | Week 24c |

**Total Impact**: Increases coverage from **73% → 85%**  
**Production Value**: High (Web + Time = critical foundation)  
**Dependencies**: All modules complete, no blocking issues

---

## WEEK 23A: DATE/TIME API (Effort: 1-2 days)

### What We're Building
```killer
// Current Problem
now = system_time_ms()  // returns raw milliseconds
sleep(1000)            // can only sleep

// What We Need
now = now()            // returns DateTime object
day = now.day()        // day of month
month = now.month()    // month (1-12)
year = now.year()      // 4-digit year
weekday = now.weekday() // 0-6 (Mon-Sun)
format = now.format("%Y-%m-%d %H:%M:%S")  // custom formatting

parse_time("2026-03-14") // DateTime parsing
duration = now - past_time  // Duration math
```

### Implementation Steps

#### Step 1: Create `datetime.rs` module (150 lines)
**File**: `src/v2-rust/killer_vm/src/datetime.rs`

Core structs:
- `KillerDateTime` - Wraps SystemTime + formatting
- `KillerDuration` - Time intervals
- Conversion methods to/from Unix timestamp

#### Step 2: Add to `builtin.rs` (40 lines)
Register functions:
- `now()` → KillerDateTime  
- `parse_datetime(String)` → KillerDateTime
- `.day()`, `.month()`, `.year()`, `.hour()`, `.minute()`, `.second()`
- `.format(String)` → String

#### Step 3: Create examples
- `week23_01_datetime_basics.killer`
- `week23_02_time_calculations.killer`
- `week23_03_formatting_parsing.killer`

### Priority: ⭐⭐⭐⭐⭐ **CRITICAL** (Required for Week 20 completion)

---

## WEEK 23B: HTTP FRAMEWORK (Effort: 3-4 days)

### What We're Building
```killer
// Current Problem
// No HTTP library at all

// What We Need (Simple Web Framework)
server = HttpServer_new("0.0.0.0", 8080)
server.route("GET", "/", fn(req) => {
    return {
        "status": 200,
        "body": "Hello, World!"
    }
})
server.route("POST", "/api/users", fn(req) => {
    body = parse_json(req.body)
    return {
        "status": 201,
        "body": json_stringify({"id": 42, "name": body.name})
    }
})
server.listen()

// HTTP Client
response = http_get("https://api.example.com/data")
data = parse_json(response.body)
```

### Implementation Steps

#### Step 1: Expand `net.rs` module (200 lines)
Add:
- `KillerHttpRequest` struct
- `KillerHttpResponse` struct
- HTTP parser (GET, POST, headers)

#### Step 2: Add to `builtin.rs` (50 lines)
- `HttpServer_new(host, port)` → Server
- `HttpServer_listen(server)` → blocking listen
- `http_get(url)` → Response
- `http_post(url, body)` → Response
- `parse_json(string)` → Dict
- `json_stringify(dict)` → String

#### Step 3: Create examples
- `week23_04_http_server_basic.killer` (100 lines)
- `week23_05_http_routes.killer` (120 lines)
- `week23_06_http_client.killer` (80 lines)
- `week23_07_rest_api.killer` (150 lines)

### Priority: ⭐⭐⭐⭐⭐ **CRITICAL** (Required for Week 21 completion)

---

## WEEK 24A: JSON/SERIALIZATION (Effort: 2-3 days)

### What We're Building
```killer
// Current Gap: JSON only partially works

// What We Need
data = {"name": "Alice", "age": 30, "scores": [95, 87, 92]}

// Serialize
json_str = json_stringify(data, pretty=true)
// {"name": "Alice", "age": 30, "scores": [95, 87, 92]}

// Deserialize
parsed = parse_json(json_str)
assert parsed["name"] == "Alice"

// CSV Support
csv_data = [
    {"id": 1, "name": "Alice", "role": "Engineer"},
    {"id": 2, "name": "Bob", "role": "Manager"}
]
csv_str = to_csv(csv_data)
parsed_csv = parse_csv(csv_str)
```

### Implementation Steps

#### Step 1: Add to `builtin.rs` (60 lines)
- `json_stringify_pretty(dict) → String`
- `parse_json_strict(string) → Dict` (better error messages)
- `to_csv(list_of_dicts) → String`
- `parse_csv(string) → List[Dict]`
- `to_yaml(dict) → String` (bonus)

#### Step 2: Error handling
- Better error messages for malformed JSON
- Line/column numbers in errors

#### Step 3: Create examples
- `week24_01_json_advanced.killer`
- `week24_02_csv_handling.killer`
- `week24_03_data_transformation.killer`

### Priority: ⭐⭐⭐⭐ **HIGH**

---

## WEEK 24B: WEBSOCKETS (Effort: 2-3 days)

### What We're Building
```killer
// WebSocket Server
ws_server = WebSocket_server("0.0.0.0", 9000)
ws_server.on("connect", fn(client) => {
    println("Client connected")
})
ws_server.on("message", fn(client, msg) => {
    println("Received: " + msg)
    client.send("Echo: " + msg)
})
ws_server.on("disconnect", fn(client) => {
    println("Client disconnected")
})
ws_server.listen()

// WebSocket Client
ws_client = WebSocket_connect("ws://localhost:9000")
ws_client.send("Hello, server!")
msg = ws_client.receive(timeout=5000)
```

### Implementation Steps

#### Step 1: Create `websocket.rs` module (200 lines)
- WebSocket handshake protocol
- Frame parsing (text/binary)
- Client/server message handling

#### Step 2: Add to `builtin.rs` (60 lines)
- `WebSocket_server(host, port)` → Server
- `WebSocket_connect(url)` → Client
- Server methods: `.on()`, `.listen()`
- Client methods: `.send()`, `.receive()`, `.close()`

#### Step 3: Create examples
- `week24_04_websocket_chat.killer` (150 lines)
- `week24_05_websocket_live_updates.killer` (120 lines)

### Priority: ⭐⭐⭐ **MEDIUM**

---

## WEEK 24C: SIMPLE TRAIT SYSTEM (Effort: 3-4 days)

### What We're Building
```killer
// Current Problem: No traits/interfaces
class Dog {
    fn speak() => println("Woof!")
}

// What We Need: Traits for polymorphism
trait Animal {
    fn speak()
    fn move()
}

class Dog impl Animal {
    fn speak() => println("Woof!")
    fn move() => println("Dog runs")
}

class Bird impl Animal {
    fn speak() => println("Tweet!")
    fn move() => println("Bird flies")
}

// Use polymorphically
animals = [Dog(), Bird()]
for animal in animals {
    animal.speak()
    animal.move()
}
```

### Implementation Steps

#### Step 1: Expand parser (100 lines)
- Parse `trait` keyword
- Parse `impl Trait for Class` syntax

#### Step 2: Update compiler (150 lines)
- Trait resolution during codegen
- Method dispatch for trait methods

#### Step 3: Create examples
- `week24_06_traits_basic.killer`
- `week24_07_trait_inheritance.killer`
- `week24_08_polymorphism_advanced.killer`

### Priority: ⭐⭐⭐ **MEDIUM** (Enables Week 18+ patterns)

---

## 📈 IMPACT ANALYSIS

### Coverage Before/After

```
Before:  73% (68 topics fully + 42 partial + 40 missing)
After:   85% (82 topics fully + 35 partial + 33 missing)
```

### New Features Enabled
- ✅ Real-time web applications (WebSockets)
- ✅ REST API development (HTTP framework)
- ✅ Data interchange (JSON/CSV)
- ✅ Scheduling & cron (DateTime)
- ✅ Polymorphic designs (Traits)

### Teaching Application
| Week | Topic | Enabled By |
|------|-------|-----------|
| Week 20 | Real-Time Systems | DateTime APIs |
| Week 21 | HTTP Services | HTTP Framework |
| Week 22 | Data Processing | JSON/CSV |
| Week 18 | Advanced OOP | Traits |
| Bonus | Real-Time Apps | WebSockets |

---

## 🔧 EXECUTION CHECKLIST

### Pre-Implementation
- [ ] Verify Killer VM builds successfully
- [ ] Test Week 19-22 examples (baseline)
- [ ] Review existing builtin.rs for patterns

### Week 23A (DateTime)
- [ ] Create `datetime.rs` module
- [ ] Implement core structs + methods
- [ ] Add 3 builtin functions
- [ ] Write 3 example files
- [ ] Test with cargo build
- [ ] Verify examples execute

### Week 23B (HTTP Framework)
- [ ] Expand `net.rs` with HTTP parsing
- [ ] Add 5 builtin functions
- [ ] Create `HttpServer` and `HttpResponse` types
- [ ] Write 4 example files
- [ ] Test simple HTTP requests
- [ ] Benchmark server performance

### Week 24A (Serialization)
- [ ] Add JSON/CSV functions to `builtin.rs`
- [ ] Implement pretty printing
- [ ] Add CSV parser
- [ ] Write 3 example files
- [ ] Test with real data files

### Week 24B (WebSockets)
- [ ] Create `websocket.rs` module
- [ ] Implement WebSocket handshake
- [ ] Add 6 builtin functions
- [ ] Write 2 example files
- [ ] Test client/server communication

### Week 24C (Traits)
- [ ] Expand parser.rs for trait syntax
- [ ] Update compiler.rs for trait resolution
- [ ] Write 3 example files
- [ ] Test trait dispatch
- [ ] Verify polymorphism works

### Post-Implementation
- [ ] Create comprehensive documentation
- [ ] Update learning paths
- [ ] Verify all 150+ topics covered
- [ ] Final build verification

---

## 📝 SUCCESS METRICS

✅ **Killer v3.0 Completion**:
- All 11 core APIs from Weeks 19-22 integrated
- DateTime API fully functional
- HTTP framework supporting basic REST
- JSON/CSV serialization complete
- WebSocket support (basic)
- Trait system implemented (basic)

✅ **Curriculum Completion**:
- 400+ problems (Weeks 19-22) ✓ Done
- 100+ new problems (Weeks 23-24) - Add with new APIs

✅ **Coverage Target**: **85%+ of 150-topic roadmap**

---

## 🎯 NEXT ACTIONS

1. **IMMEDIATE** (Today):
   - [ ] Start Week 23A (DateTime) implementation
   - [ ] Create `datetime.rs` module
   - [ ] Add to builtin.rs

2. **THIS WEEK** (Days 2-3):
   - [ ] Complete HTTP framework
   - [ ] Test basic server example

3. **NEXT WEEK** (Days 4-6):
   - [ ] Implement JSON/CSV
   - [ ] Add WebSocket support

4. **FOLLOWING WEEK** (Days 7-10):
   - [ ] Implement trait system
   - [ ] Final documentation

---

## 💡 STRATEGIC VALUE

**Why This Matters**:
- DateTime + HTTP = Web applications enabled
- JSON/CSV = Data science workflows
- WebSockets = Real-time systems (Week 22)
- Traits = Advanced OOP (Weeks 18+)

**Competitive Advantage**:
Killer becomes the #1 language for teaching:
- Real-time web development
- Concurrent systems
- Data processing pipelines
- Modern software architecture

**Path to v3.1**:
These features are the foundation for:
- Machine learning frameworks
- Database ORMs
- AI agent platforms
- Cloud-native applications
