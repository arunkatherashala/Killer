# Week 21 Weekly Schedule: Network Services
## 75 Hours | Async Integration, HTTP, WebSockets, RPC

---

# OVERVIEW
Building on Week 19 (pools) and Week 20 (latency), Week 21 adds network capabilities: HTTP servers, WebSockets, RPC, request routing.

**Key Challenge**: No native async runtime in Killer. Solution: Manual async patterns + tokio integration (via Rust delegation).

---

# WEEKLY STRUCTURE

**Monday (15h)**: Network Fundamentals & Socket Programming
- TCP/IP basics, connection lifecycle, protocol design
- Problems 21.1.1-30: Socket creation, sends, receives, timeouts, error handling

**Tuesday (15h)**: HTTP Protocol & Server Building
- HTTP verbs (GET, POST), headers, response codes
- Problems 21.2.1-30: HTTP parsing, routing, response generation, cookies, compression

**Wednesday (15h)**: WebSockets & Real-Time Communication
- WebSocket upgrade, frame parsing, bidirectional streams
- Problems 21.3.1-30: WebSocket server, chat app, broadcast patterns, rate limiting

**Thursday (15h)**: Microservice Architecture
- Service discovery, load balancing, distributed tracing
- Problems 21.4.1-30: Multi-service system, health checks, circuit breakers, request routing

**Friday (15h)**: Capstone - 5-Service Microcluster
- Coordinator, 4 worker services, inter-service communication
- All via socket/HTTP, <200ms latency p99, 1000 requests/sec total

---

# KEY PATTERNS

**Actor-Based HTTP Server**
```
┌──────────────────────┐
│ HTTP Listener        │ (listens port 8080)
└──────────┬───────────┘
           │
        Connection handlers (per-connection actor)
        ├─ Read HTTP request
        ├─ Route to handler
        ├─ Execute (via actor pool)
        ├─ Send response
        └─ Close or keep-alive
```

**WebSocket Upgrade**
```
HTTP GET /ws
Upgrade: websocket
───────────────────→
         ←─────────────────
HTTP 101 Switching Protocols

Now bidirectional frames (text, binary, ping/pong)
```

**Microservices**
```
      Client
        │
        ▼
  ┌─────────────┐
  │ API Gateway │ (entry point)
  └─────────────┘
        │
   ┌────┼────┬─────┐
   ▼    ▼    ▼     ▼
  Order Auth User Config
  (each service: HTTP endpoint)
```

---

# TECHNICAL CHALLENGES

1. **No std:: HTTP library** → Parse HTTP manually
2. **No async runtime** → Fake async with actor pools
3. **No TLS** → Plain sockets for now
4. **No service discovery** → Manual registration

---

# SUCCESS METRICS

- 5 services coordinating
- 1000 requests/sec total throughput
- P99 latency < 200ms (include network)
- No dropped connections
- Graceful service restart
- Health checks working

---

