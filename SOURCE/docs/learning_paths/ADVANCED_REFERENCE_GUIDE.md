# Weeks 15-18 Reference Guide: Advanced Architecture & Production Systems
## Patterns, Solutions, Debugging, Best Practices

---

# Part 1: Microservices Architecture Patterns

## 1.1 Service Decomposition Patterns

### Pattern 1: By Business Capability
```
Monolith:
  authentication/
  users/
  products/
  orders/
  payments/
  notifications/
  all in ONE database

Microservices:
  UserService (auth, profiles)
    - Technology: Node.js, PostgreSQL
    - Team: User team (2 people)
  ProductService (catalog)
    - Technology: Python, MongoDB
    - Team: Catalog team (3 people)
  OrderService (orders, orchestration)
    - Technology: Rust, PostgreSQL
    - Team: Order team (4 people)

Benefits:
  ✓ Each team owns full stack
  ✓ Different tech for different needs
  ✓ Scale independently
  ✓ Deploy independently
```

### Pattern 2: By Subdomain (DDD)
```
Core Domain (builds differentiation):
  ✓ Investment analysis
  ✓ Portfolio optimization
  ✓ Risk assessment

Supporting Domain (necessary, not differentiating):
  ✓ User management
  ✓ Billing
  ✓ Reporting

Generic Domain (existing solutions):
  ✓ Authentication (OAuth)
  ✓ Logging (ELK stack)
  ✓ Monitoring (Prometheus)

Recommendation:
  - Invest in core domain
  - Buy/use generic domain
  - Outsource supporting if possible
```

### Pattern 3: By Scalability Requirements
```
Low traffic, tightly coupled:
  → Monolith ok

Different scaling needs:
  SearchService (100K req/sec):
    → ElasticSearch, Redis caching
  PaymentService (1K req/sec, high reliability):
    → Scaled down, but HA (3x redundancy)
  AnalyticsService (batch, 1K events/sec):
    → Kafka, Spark, daily aggregation
```

## 1.2 Data Management Patterns

### Pattern: Event Sourcing
**Problem**: Order state changes are lost
**Solution**: Store ALL events

```rust
// Traditional: store current state
Order {
  id: 1,
  status: "SHIPPED",  // Where did PENDING, CONFIRMED go?
  total: 100.0,
}

// Event sourcing: store all events
OrderEvents {
  1. Event::OrderCreated { id: 1, user_id: 5, total: 100.0 },
  2. Event::OrderConfirmed { id: 1 },
  3. Event::PaymentProcessed { id: 1 },
  4. Event::ItemsShipped { id: 1 },
  // Rebuild: apply in order → status SHIPPED
}

// Benefits:
// - Full audit trail (why did this change?)
// - Replay to different state (what was status on 2pm?)
// - Integrate with others (all changes visible as events)
```

### Pattern: CQRS (Command Query Responsibility Separation)
**Problem**: Write-optimized DB (normalized) is slow for reads

```rust
// Write Model (normalized for write efficiency)
WriteDB {
  orders table (normalized),
  order_items (separate table)
}

// Publish events
event_bus.publish(OrderCreated { ... });

// Read Model (denormalized for read speed)
ReadCache {
  orders_with_items: {
    id: 1,
    user_name: "Alice",
    items: [
      { product: "Book", qty: 2, price: 20 },
      { product: "Pen", qty: 1, price: 5 },
    ],
    total: 45,
    status: "CONFIRMED",
  }
}

// Query reads from cache (instant)
// Speed trade-off: slight delay in consistency (eventual)
```

### Pattern: Saga for Distributed Transactions
**Problem**: ACID transactions span multiple services/databases

**Solution**: Choreography (event-driven)
```
1. OrderService: order created → publish OrderCreated
2. PaymentService: hears OrderCreated
   → Charge card → publish PaymentProcessed
3. InventoryService: hears PaymentProcessed
   → Reserve items → publish ItemsReserved
4. OrderService: hears ItemsReserved
   → Mark order CONFIRMED → publish OrderConfirmed
5. NotificationService: hears OrderConfirmed
   → Send confirmation email

Compensation (rollback):
  If PaymentService fails:
    → Publish PaymentFailed
    → InventoryService: unreserve items
    → OrderService: mark order CANCELLED
```

## 1.3 Service Communication Patterns

### Pattern 1: Request-Response (Synchronous)
```rust
// UserService exposes REST endpoint
GET /api/users/123 → { id: 123, name: "Bob" }

// OrderService calls it synchronously
let user = http::get("http://user-service/api/users/123").await?;

Problems:
  - Network latency (if UserService slow, OrderService slow)
  - Cascading failures (UserService down → OrderService down)
  - Tight coupling

Solutions:
  - Circuit breakers (detect failure, fail fast)
  - Timeouts (don't wait forever)
  - Bulkheads (limit threads/connections)
```

### Pattern 2: Async Messages (Using Kafka)
```rust
// OrderService: create order
order_service.create_order(user_id, items);
// Publish event
event_bus.publish(OrderCreated { order_id: 1, items, ... });
// Return immediately
return OrderResponse { order_id: 1, status: "Processing" };

// Later, other services process:
// PaymentService listening:
kafka.subscribe("order-created", |event| {
  process_payment(event.order_id);
});

Benefits:
  ✓ Decoupled (services don't know about each other)
  ✓ Scalable (can add new services without changing others)
  ✓ Resilient (if PaymentService down, keep accepting orders)
  ✓ Replay (new service subscribes, processes past events)

Challenges:
  - Eventual consistency (slight delay in credit card charge)
  - Debugging (distributed, not single call stack)
  - Ordering (kafka partitions guarantee ordering per partition)
```

### Pattern 3: Service Discovery
**Problem**: Services move (cloud, autoscaling)

```rust
// Bad: hard-coded
const USER_SERVICE = "192.168.1.10:8080";

// Good: dynamic
ServiceDiscovery {
  register("user-service", "192.168.1.10:8080");
  // Later: service moves
  register("user-service", "192.168.1.15:8080");
  
  // Clients query
  endpoints = discover("user-service");
  // Returns: ["192.168.1.15:8080"]
}

Implementations:
  - Consul: manual register/deregister
  - Kubernetes: automatic (ReplicaSet manages pods)
  - AWS: ELB + Route 53
  - DNS-based: service-name.namespace.svc.cluster.local
```

---

# Part 2: Cloud Deployment Patterns

## 2.1 Container & Orchestration

### Pattern: Multi-Stage Docker Build
```dockerfile
# Stage 1: Build
FROM rust:1.70 as builder
WORKDIR /app
COPY src/ src/
RUN cargo build --release
# Result: 2GB+ (compiler + dependencies)

# Stage 2: Runtime
FROM debian:bookworm-slim
COPY --from=builder /app/target/release/api /app/api
CMD ["/app/api"]
# Result: 200MB (only binary + runtime libs)

# Benefits:
# - Smaller image (smaller downloads, faster deployments)
# - Smaller attack surface
# - Faster startup
```

### Pattern: Blue-Green Deployment
```
Current production (BLUE):
  - 5 servers running v1.0
  - Handling all traffic
  - Stable, proven

New version (GREEN):
  - 5 servers running v1.1
  - Warmed up, ready
  - No traffic yet

Switch:
  - Load balancer: redirect all traffic to GREEN
  - Instant switchover (no gradual)
  - Rollback: switch back to BLUE (instant)

Advantages:
  ✓ Zero downtime
  ✓ Instant rollback
  ✓ Test GREEN before traffic

Disadvantages:
  ✗ 2x resources temporarily
  ✗ Database migration risk
```

### Pattern: Canary Deployment
```
Push v1.1:
  - 1% of users → new version
  - Monitor: errors, latency, crashes
  - 0% errors? → 10%
  - 10% errors? Rollback

Gradual roll: 1%, 5%, 25%, 50%, 100%

Time to full deploy: 30-60 minutes
Advantages:
  ✓ Catch errors before all users
  ✓ Small blast radius
  ✓ Real traffic testing

Disadvantages:
  ✗ Slower (gradual)
  ✗ More complex (multiple versions)
```

## 2.2 Infrastructure as Code

### Pattern: Terraform for AWS
```hcl
# Define cloud infrastructure as code

resource "aws_vpc" "main" {
  cidr_block = "10.0.0.0/16"
}

resource "aws_subnet" "az1" {
  vpc_id = aws_vpc.main.id
  cidr_block = "10.0.1.0/24"
  availability_zone = "us-east-1a"
}

resource "aws_instance" "web" {
  count = 3
  ami = "ami-0c55b159cbfafe1f0"
  instance_type = "t3.medium"
  subnet_id = aws_subnet.az1.id
}

Benefits:
  ✓ Version control (git history of infra)
  ✓ Reproducible (same config = same resources)
  ✓ Idempotent (run multiple times safely)
  ✓ Destroy/recreate (test disaster recovery)
```

## 2.3 CI/CD Pipeline Patterns

### Pattern: GitHub Actions
```yaml
name: Deploy

on:
  push:
    branches: [main]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - name: Run tests
        run: cargo test --release
      
      - name: Build
        run: cargo build --release
      
      - name: Security scan
        run: cargo audit
      
      - name: Benchmark
        run: cargo bench
  
  deploy:
    needs: test
    runs-on: ubuntu-latest
    steps:
      - name: Build Docker image
        run: docker build -t user-service:${{ github.sha }} .
      
      - name: Push to ECR
        run: aws ecr push user-service:${{ github.sha }}
      
      - name: Deploy to Kubernetes
        run: kubectl set image deployment/user-service user-service=user-service:${{ github.sha }}

Benefits:
  ✓ Automated testing (no manual testing)
  ✓ Automated deployment (push = deploy)
  ✓ Audit trail (git history)
  ✓ Rollback (previous git commit)
```

---

# Part 3: Performance Optimization Patterns

## 3.1 Caching Patterns

### Pattern: Multi-Level Cache
```
L1: In-process (HashMaps)
  - Speed: < 1µs
  - Size: 10-100MB
  - Example: ProductService caches product list

L2: Distributed cache (Redis)
  - Speed: 1-5ms
  - Size: 1-100GB
  - Example: Order summaries, user profiles

L3: Database
  - Speed: 10-100ms
  - Size: unlimited
  - Example: Full data

Query path:
  1. Check L1 (in-process)
    → Hit: return instantly
  2. Check L2 (Redis)
    → Hit: return in 1-5ms, populate L1
  3. Query database
    → Return in 10-100ms, populate L1 + L2

Cache invalidation:
  - TTL: expire after time (user profile expires after 1 hour)
  - Event-driven: when user updates profile, invalidate
  - LRU: remove least recently used item when full
```

### Pattern: Cache-Aside vs Write-Through
```
CACHE-ASIDE (Read through):
  1. Client checks cache
  2. Miss → query database
  3. Insert into cache
  4. Return to client
  5. Later updates to database → client responsible for invalidating cache

WRITE-THROUGH:
  1. Client writes to cache
  2. Cache writes to database
  3. Return to client
  4. All reads hit cache (if miss, slow first read)

WRITE-BEHIND (Write-back):
  1. Client writes to cache (return immediately)
  2. Cache asynchronously writes to database
  3. Fast writes, but risk losing data if cache dies

Best: Cache-aside + event-driven invalidation
```

## 3.2 Database Optimization

### Pattern: Indexing Strategy
```sql
-- Slow query:
SELECT * FROM orders WHERE user_id = 123 AND status = 'CONFIRMED';
-- Scans entire table (1000ms)

-- Add index:
CREATE INDEX idx_orders_user_status ON orders(user_id, status);
-- Now: 5ms (seeks directly)

Index guidelines:
  ✓ Index on foreign keys (WHERE user_id = ?)
  ✓ Index on sort columns (ORDER BY created_at)
  ✓ Composite index for common WHERE + ORDER BY combos
  ✗ Don't over-index (slows writes, wastes space)
  ✗ Don't index low-cardinality columns (enum with 3 values)
```

### Pattern: Replication
```
Master-Slave:
  Master (write): receives updates → replicates to slaves
  Slaves (read): serve read queries
  
  Scaling:
    - Writes: only master (bottleneck)
    - Reads: scale horizontally (more slaves)
  
  HA:
    - Master dies: promote oldest slave to master (seconds)
    - Data loss: depends on replication lag

Multi-master (Cassandra, Dynamo):
  - Every node accepts writes
  - Nodes reconcile (eventual consistency)
  - Can survive multiple node failures
  - Trade-off: complex conflict resolution
```

## 3.3 Monitoring & Debugging

### Pattern: RED Metrics
```
Rate: requests per second
  - Normal: 1000 req/sec
  - Alert: > 5000 req/sec (unusual spike)

Errors: failed requests
  - Track: 500 errors, timeouts, circuit breaker rejections
  - Alert: > 0.5% error rate

Duration: latency
  - p50 (median): 50ms
  - p95 (95th percentile): 200ms
  - p99 (99th percentile): 500ms
  - Alert: p99 > 1000ms (degraded)

Example health dashboard:
  ✓ API: 1000 req/sec, 0.1% errors, p99=80ms
  ✗ PaymentService: 10 req/sec, 2% errors, p99=2000ms (degraded!)
  → Page on-call engineer
```

### Pattern: Distributed Tracing
```
One user creates order:
  GET /orders (api-gateway)
    → POST /orders (order-service)
      → GET /users/123 (user-service)
      → POST /payments (payment-service)
        → POST /charge (stripe external)
      → POST /inventory (inventory-service)
    → POST /notify (notification-service)

Without tracing: "order is slow" (where?)
With tracing: "charge request to Stripe took 2sec"

Tools:
  - Jaeger: open-source, complex
  - Datadog: managed, expensive
  - AWS X-Ray: AWS native

Setup:
  1. Generate trace ID on entry
  2. Pass X-Trace-ID header to all downstream calls
  3. Each service logs with trace ID
  4. Collector aggregates all logs by trace ID
```

---

# Part 4: Operational Excellence

## 4.1 SLO & Error Budgets

### Concept: Service Level Indicator (SLI)
```
SLI = (successful requests) / (total requests)

Example:
  - Total requests: 1,000,000 / month
  - Errors (5xx, timeouts): 100
  - SLI = (1,000,000 - 100) / 1,000,000 = 99.99%
```

### Concept: Service Level Objective (SLO)
```
SLO = target SLI

Examples:
  - 99% uptime (99.x% availability) = 43 minutes / month downtime
  - 99.9% uptime = 4 minutes / month downtime
  - 99.99% uptime = 26 seconds / month downtime

Choose based on:
  - Payment: 99.99% (money is involved)
  - Search: 99% (user can retry)
  - Analytics: 95% (batch, not time-sensitive)
```

### Concept: Error Budget
```
SLO: 99.99% uptime
Error budget: 0.01% = 26 seconds / month

How to use:
  - Risky deploy (not fully tested): uses 5 seconds
  - Breaking change: uses 10 seconds
  - Careful optimization: uses 2 seconds
  
When budget exhausted:
  ✓ No risky deploys
  ✓ Focus on stability
  ✓ Run chaos experiments only off-hours

Benefit:
  ✗ Avoids motivation: "stay at 99% for month"
  ✓ Enables innovation: "we have 26 seconds to spend"
```

## 4.2 Runbooks & Playbooks

### Example Runbook: Database Replication Lag
```
🚨 ALERT: Payment DB replication lag > 30 seconds

IMPACT:
  - Payment writes succeed, but reads might be stale
  - User sees "order status: pending" but actually charged
  - Low impact (lag recovers within seconds)

DIAGNOSIS:
  1. Check Prometheus: show 'mysql_replica_lag_seconds'
  2. Check MySQL: SHOW SLAVE STATUS\G
  3. Check network: ping slave (latency high?)

REMEDIATION:
  IMMEDIATE (< 2 minutes):
    1. Stop writes to master: SET GLOBAL read_only = ON;
    2. Wait for replica to catch up
    3. Resume writes: SET GLOBAL read_only = OFF;
  
  IF ABOVE FAILS (< 5 minutes):
    1. Promote slave to new master
    2. Failover clients to new master
    3. Page database team
  
  IF MULTIPLE REPLICAS DOWN (< 10 minutes):
    1. Trigger pagerduty escalation
    2. Disable payment service (queue requests)
    3. Restore from backups

PREVENTION:
  - Monitor replication lag < 1 second
  - Failover test monthly
  - Network upgrade (upgrade link to slave)
```

---

# Part 5: Common Mistakes & Solutions

## Mistake 1: Too Many Microservices Too Early
```
Company: 5 people -> split into 10 services
Reality:
  - 5 people can't operate 10 services
  - Each service = operational burden
  - Debugging becomes nightmare

Solution:
  - Start: monolith (1 service)
  - When: >20 people OR clear domain separation
  - Build: only the services you can operate
  
Formula: 10*people = max services (5 people → 50 services max)
```

## Mistake 2: Ignoring Operational Complexity
```
"We built the service! Ship it!"
Reality:
  - Crashes at 3am (on-call engineer woken)
  - Cascades failures (one service down = 10 services timeout)
  - Slow deploys (2 hours for one status code fix)
  - Can't debug (where did the request go?)

Solution (before deploy):
  ✓ Health checks (respond to /health endpoint)
  ✓ Graceful shutdown (finish in-flight requests)
  ✓ Circuit breakers (fail fast, not cascade)
  ✓ Distributed tracing (track requests)
  ✓ Metrics collected (RED: rate, errors, duration)
  ✓ Runbooks written (how to remediate)
```

## Mistake 3: Eventual Consistency Surprises
```
Scenario:
  User: "I created order, why can't I see it?"
  
Your system:
  1. OrderService creates order (returns immediately)
  2. Event published (async)
  3. OrderUI queries read cache (doesn't have it yet)
  4. User sees: "No orders"
  
Solution:
  - UI shows: "Computing..." while events propagate
  - Retry with exponential backoff
  - Cache by user_id (popular queries cached, ready)
  - Accept: 100-500ms lag (reasonable for eventual consistency)
```

## Mistake 4: Cascading Failures
```
System:
  UserService → OrderService → PaymentService

Failure scenario:
  1. PaymentService slow (external API slow)
  2. OrderService calls PaymentService, times out (30s)
  3. OrderService threads exhausted (all waiting)
  4. OrderService rejects new requests
  5. UserService calls OrderService, times out
  6. UserService fails
  7. Whole system down

Solution: Bulkheads
  OrderService:
    - 100 worker threads total
    - 20 reserved for PaymentService calls
    - 80 for other operations
    - If PaymentService slow: 20 threads hang, 80 keep working
  
Code:
  circuit_breaker.call(|| {
    payment_service.charge(amount)
  }, timeout: 5s)?;
  // Fail fast after 5s, don't cascade
```

## Mistake 5: No Chaos Testing
```
"Our system is HA, we're safe"

Reality:
  - Network partition (50% packet loss)
  - Slow database (tail latency: 95th percentile slow)
  - Memory leak in one service
  - Misconfigured load balancer

Solution: Chaos engineering
  Monthly tests:
    ✓ Kill random pod (can system recover?)
    ✓ Introduce 200ms latency (how does it behave?)
    ✓ Fill disk 90% (do we gracefully degrade?)
    ✓ Network partition (does failover work?)
    
  Tools: Gremlin, AWS FIS, Kubernetes chaos
  
  Benefit:
    → Find failures in controlled way
    → Fix before customer sees them
    → Ops team trained (when real failure, team calm)
```

---

# Part 6: Technology Selection Guide

## Choosing between Sync vs Async Communication

|  | Sync (gRPC/REST) | Async (Kafka/Queue) |
|--|--|--|
| **Latency** | Low (10-100ms) | High (100ms-hours) |
| **Coupling** | Tight (A calls B) | Loose (A publishes, B subscribes) |
| **Debugging** | Easy (call stack) | Hard (distributed) |
| **Failure handling** | Circuit breaker | Retry/dead letter queue |
| **Ordering** | Per request | Per partition |
| **Best for** | Real-time (search, checkout) | Eventual consistency (email, analytics) |
| **Scale** | 1000s req/sec per service | 100K+ events/sec |
| **Example** | GET user profile | Order placed → send email |

## Choosing between Event Sourcing vs CRUD

|  | Event Sourcing | Traditional CRUD |
|--|--|--|
| **State storage** | All events (immutable) | Current state only |
| **Audit trail** | Automatic (all events) | Manual (audit table) |
| **Debugging** | Replay to any point in time | Guess what happened |
| **Snapshot** | Optional (needed for performance) | Not applicable |
| **Complexity** | High | Low |
| **Learning curve** | Steep | Shallow |
| **Best for** | Financial, audit-heavy, temporal queries | CRUD apps, simple data |
| **Example** | Bank: every transaction an event | Blog: posts, comments |

## Choosing Cache Strategy

|  | In-Process | Distributed (Redis) | Database |
|--|--|--|--|
| **Speed** | < 1µs | 1-5ms | 10-100ms |
| **Size** | 100MB-1GB | 10GB-1TB | Unlimited |
| **Failure** | One machine | All get old data | Slow |
| **Invalidation** | Manual | TTL/event-driven | Fresh |
| **Use case** | Hot data (top products) | Shared across servers | Source of truth |

---

# Part 7: Production Readiness Checklist

Before deploying to production:

## Reliability
- [ ] Unit tests (90%+ coverage)
- [ ] Integration tests (critical paths)
- [ ] E2E tests (user scenarios)
- [ ] Load testing (target load, 2x load)
- [ ] Chaos testing (failure scenarios)
- [ ] Failover testing (can system recover?)
- [ ] Rollback testing (can we rollback?)

## Observability
- [ ] Logging (structured JSON logs)
- [ ] Metrics (RED: rate, errors, duration)
- [ ] Tracing (distributed tracing set up)
- [ ] Dashboards (health at a glance)
- [ ] Alerting (problems detected before users see)

## Operations
- [ ] Runbooks (how to operate)
- [ ] Playbooks (how to recover)
- [ ] On-call process (who is on-call?)
- [ ] Incident response (blameless postmortems)
- [ ] Capacity planning (how many servers needed?)
- [ ] Scaling strategy (how to handle 10x traffic?)

## Security
- [ ] Encryption in transit (HTTPS)
- [ ] Encryption at rest (password hashing, DB encryption)
- [ ] Authentication (who are you?)
- [ ] Authorization (what are you allowed to do?)
- [ ] Audit logging (who did what when?)
- [ ] Vulnerability scanning (CVEs in dependencies)

## Compliance
- [ ] Data retention (how long to keep logs?)
- [ ] Data deletion (GDPR right to be forgotten)
- [ ] Access control (who can access what?)
- [ ] Audit trail (prove who accessed what)
- [ ] Backup (can recover from disaster?)
- [ ] Disaster recovery plan (tested quarterly)

---

# Part 8: Real-World System Examples

## Example 1: Uber
```
Services:
  - Matching (complex algorithm, ML)
  - Geolocation (real-time tracking)
  - Payment (transactional)
  - Driver Management
  - Passenger Management
  - Dispatch
  - Analytics

Technologies:
  - Cassandra (geolocation timeseries)
  - MySQL (transactional, payment)
  - Kafka (events: trips, payments)
  - Redis (caching, real-time matching)

Scale:
  - 100M+ daily active users
  - 1000s of requests per second
  - 99.999% uptime (5 nines)

Challenges:
  - Real-time matching (match rider + driver in < 1 second)
  - Geographic consistency (same ride info across regions)
  - Payment at scale (idempotent charges)
```

## Example 2: Netflix
```
Services: 700+ microservices (not recommended for most!)

Key decisions:
  - Asynchronous communication (mostly Kafka)
  - Event sourcing (all content changes events)
  - Cassandra (highly available, no single master)
  - Resilience: Hystrix, Ribbon, Eureka
  - Chaos engineering (Simian Army kills random servers)

Scale:
  - 250M+ subscribers
  - 1B+ requests per day
  - Streams to 190+ countries

Lessons:
  - Only Netflix scale can justify this complexity
  - Built own tools (Hystrix, Ribbon, Eureka)
  - Failure is normal (chaos test daily)
```

## Example 3: Stripe
```
Payment processing (must be reliable):

Requirements:
  - 99.999% uptime (5 nines) ≈ 26 seconds downtime per month
  - < 100ms latency (users waiting)
  - PCI-DSS compliance (credit card industry standard)
  - Fraud prevention (machine learning)

Architecture:
  - Multi-region active-active (process in multiple datacenters)
  - Synchronous for critical paths (payment processing)
  - Strong consistency (payments can't get lost)
  - Comprehensive monitoring (10s of thousands of metrics)

Lessons:
  - Some services need strong consistency + synchronous
  - Operational excellence (1000s of people on-call rotations)
  - Redundancy everywhere (everything has backup)
```
