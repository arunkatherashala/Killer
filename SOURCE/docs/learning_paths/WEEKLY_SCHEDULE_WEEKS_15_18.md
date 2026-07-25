# Weeks 15-18 Weekly Schedule: Advanced Optimizations & Cloud Deployment
## 900 Hours Total | Production-Grade Systems

---

# WEEK 15: MICROSERVICES ARCHITECTURE
**75 hours | Core Concepts: Service Decomposition, API Design, Distributed Data**

## Weekly Overview

### Learning Path
```
Monday: Monolith Decomposition (15h)
Tuesday: Distributed Data & Saga (15h)
Wednesday: Communication Patterns (15h)
Thursday: API Design & Contracts (15h)
Friday: Capstone - E-Commerce Microservices (15h)
```

### Time Allocation
- Lectures/Concepts: 25 hours (33%)
- Hands-on Exercises: 35 hours (47%)
- Capstone Project: 15 hours (20%)

---

## MONDAY: Service Decomposition (15 hours)

### 09:00-11:00 | Understanding Monoliths (2h)

**Concepts**
- Monolith characteristics: tightly coupled, single deployment
- Technical debt in monoliths
- Why decompose: scalability, team velocity, failure isolation

**Problems**
```
15.1.1: Identify service boundaries in a monolithic e-commerce app
  - Current: User, Product, Order, Payment in single codebase
  - Task: Decompose by business domain
  - Expected: UserService, ProductService, OrderService, PaymentService

15.1.2: By team ownership
  - Task: Design services to match organization structure
  - Expected: Platform team owns core services, feature teams own domain

15.1.3: Data isolation
  - Task: Identify shared databases and separate
  - Expected: Database per service + event sourcing
```

**Hands-on**
```rust
// Given a monolithic User/Order/Payment system
// Exercise: Extract UserService, OrderService independently

pub struct UserService {
    users: HashMap<u64, User>,
}

pub struct OrderService {
    // Should NOT directly access UserService.users!
    // Use async calls or event-driven communication
}
```

### 11:00-13:00 | Decomposition Strategies (2h)

**Concepts**
- Strangler Fig pattern: gradually replace parts
- Anti-corruption layer: protect new services
- Domain-Driven Design: bounded contexts

**Problems**
```
15.1.4: Strangler pattern
  - Implement service alongside monolith
  - Route percentage of traffic to new service
  - Gradually increase traffic percentage

15.1.5: Anti-corruption layer
  - Old system speaks OLD_API
  - New system speaks NEW_API
  - AC layer translates between them

15.1.6: Bounded contexts
  - User domain (authentication, profile, preferences)
  - Product domain (catalog, search, recommendations)
  - Order domain (creation, fulfillment, shipping)
  - Define clear boundaries
```

**Hands-on**
```rust
// Anti-corruption layer example
pub struct UserServiceAdapter {
    old_service: OldUserService,
    new_service: NewUserService,
}

impl UserServiceAdapter {
    fn get_user(&self, id: u64) -> NewUser {
        let old_user = self.old_service.get(id);
        // Translate old format to new format
        translate(old_user)
    }
}
```

### 13:00-14:00 | LUNCH (1h)

### 14:00-17:00 | Service Communication Foundations (3h)

**Concepts**
- Synchronous vs asynchronous
- Request-response patterns
- Event-driven patterns
- Service discovery

**Problems**
```
15.1.7: Synchronous communication
  - REST endpoints between services
  - gRPC for high-performance
  - Challenge: cascading failures

15.1.8: Asynchronous communication
  - Message queues (RabbitMQ, SQS)
  - Event streams (Kafka)
  - Challenge: eventual consistency

15.1.9: Service discovery
  - Hard-coded: IP:Port (bad for cloud)
  - Dynamic: Consul, Eureka (register/deregister)
  - DNS-based: Kubernetes services

15.1.10: Failure cascades
  - Exercise: One service slow → affects all downstream
  - Solution: Timeouts, retries, circuit breakers
```

**Hands-on**
```rust
// Communication between services
// UserService -> OrderService -> PaymentService

// Synchronous
async fn create_order(user_id: u64, items: Vec<Item>) {
    // 1. Verify user exists (call UserService)
    let user = user_service.get(user_id).await?;
    // 2. Calculate total
    let total = calculate_total(&items);
    // 3. Call PaymentService
    payment_service.charge(user.payment_method, total).await?;
    // Chain of calls: risk of cascade failure
}

// Asynchronous (event-driven)
async fn create_order(user_id: u64, items: Vec<Item>) {
    // 1. Create order locally
    let order = Order::new(user_id, items);
    order_repo.save(&order);
    
    // 2. Publish event
    event_bus.publish(OrderCreated { order_id: order.id });
    
    // 3. Return immediately
    // PaymentService listens and processes async
}
```

### 17:00-18:00 | Exercise 1: User Service (1h)

**Task**: Build isolated UserService
```rust
pub struct UserService {
    users: Arc<Mutex<HashMap<u64, User>>>,
    next_id: Arc<Mutex<u64>>,
}

// Implement:
// - create_user(name, email) -> User
// - get_user(id) -> Option<User>
// - list_users() -> Vec<User>
// - health_check() -> bool
```

**Solution**: See `advanced_exercises.rs` UserService.

### 18:00-19:00 | Evening Review & Setup (1h)
- Review microservice principles
- Begin thinking about your domain
- Prepare for Monday capstone

---

## TUESDAY: Distributed Data & Saga Pattern (15 hours)

### 09:00-11:00 | Database per Service (2h)

**Concepts**
- Problem: Monolith has ONE database
- Solution: Database per service
- Challenge: Distributed transactions

**Problems**
```
15.3.1: Design database schema for each service
  - UserService: users, profiles, preferences
  - OrderService: orders, order_items
  - PaymentService: payments, transactions
  - ProductService: products, inventory

15.3.2: Data duplication
  - OrderService needs user name, email
  - But UserService owns user data
  - Solution: Event sourcing (copy data on change)

15.3.3: Query across services
  - "Get all orders for user"
  - Requires OrderService + UserService communication
  - Challenge: consistency

15.3.4: Data consistency levels
  - Strong: ACID (database-level)
  - Eventual: updates propagate asynchronously
```

**Hands-on**
```rust
// OrderService should NOT query UserService directly
// Instead, duplicate user data via events

pub struct OrderService {
    orders: HashMap<u64, Order>,
    users: HashMap<u64, UserSnapshot>, // Cache of user data
}

// When UserService publishes UserUpdated event:
async fn on_user_updated(event: UserUpdated) {
    // Update local cache
    self.users.insert(event.user_id, event.snapshot);
    
    // Now OrderService can query locally
    // Trade-off: eventual consistency (small delay)
}
```

### 11:00-13:00 | Saga Pattern (2h)

**Concepts**
- Distributed transactions don't work (2PC is dead)
- Saga: Long-running transaction coordinated by events
- Choreography: Services listen to events
- Orchestration: Coordinator service orchestrates

**Problems**
```
15.3.5: Order creation saga (choreography)
  1. OrderService creates order (PENDING)
  2. Publishes OrderCreated event
  3. PaymentService receives, charges card
  4. Publishes PaymentProcessed event
  5. InventoryService receives, reserves items
  6. Publishes ItemsReserved event
  7. OrderService receives, marks CONFIRMED

Failure case:
  - If PaymentService fails: publishes PaymentFailed
  - All services must compensate (refund, unreserve)

15.3.6: Compensation logic
  - Create order -> on failure, delete order
  - Charge card -> on failure, refund
  - Reserve inventory -> on failure, release inventory

15.3.7: Idempotence
  - What if PaymentFailed event is received twice?
  - Solution: Idempotent handlers
  - Check: "Has this been processed?"
```

**Hands-on**
```rust
// Saga execution
pub async fn create_order_saga(order: Order) {
    // Step 1: Create order locally
    store_order(&order);
    event_bus.publish(OrderCreated { order_id: order.id });
    
    // Step 2: Wait for compensations or success
    // (Other services process async)
}

// Payment Service listening
pub async fn handle_order_created(event: OrderCreated) {
    match process_payment(&event) {
        Ok(_) => event_bus.publish(PaymentProcessed { order_id }),
        Err(_) => {
            event_bus.publish(PaymentFailed { order_id });
            // Compensation will happen
        }
    }
}

// Order Service compensating
pub async fn handle_payment_failed(event: PaymentFailed) {
    // Compensation: cancel order
    cancel_order(event.order_id);
    event_bus.publish(OrderCancelled { order_id });
}
```

### 13:00-14:00 | LUNCH (1h)

### 14:00-16:00 | Event Sourcing (2h)

**Concepts**
- Traditional: store current state (last order status = CONFIRMED)
- Event sourcing: store all events (OrderCreated, PaymentProcessed, OrderConfirmed)
- Rebuild state by replaying events

**Problems**
```
15.3.8: Event sourcing design
  - Store: [OrderCreated(id=1), PaymentProcessed(id=1), ItemsReserved(id=1)]
  - Rebuild: apply each event in order
  - Result: order status = CONFIRMED

15.3.9: Event versioning
  - Original: {"order_id": 1, "total": 100}
  - New version needs: {"order_id": 1, "total": 100, "currency": "USD"}
  - Challenge: handle both old and new in code

15.3.10: Snapshots
  - Full replay can be slow
  - Solution: Periodically snapshot state
  - Example: Event #1-1000 → Snapshot, then #1001+
```

**Hands-on**
```rust
pub enum OrderEvent {
    Created { id: u64, user_id: u64, items: Vec<Item> },
    PaymentProcessed { id: u64 },
    ItemsReserved { id: u64 },
    OrderConfirmed { id: u64 },
    OrderCancelled { id: u64 },
}

pub struct OrderAggregate {
    events: Vec<OrderEvent>,
    state: OrderState,
}

impl OrderAggregate {
    pub fn apply_event(&mut self, event: OrderEvent) {
        match &event {
            OrderEvent::Created { .. } => self.state = OrderState::Pending,
            OrderEvent::PaymentProcessed { .. } => {
                // State transitions
            }
            // ...
        }
        self.events.push(event);
    }
}
```

### 16:00-17:00 | CQRS Pattern (1h)

**Concepts**
- CQRS: Command Query Responsibility Segregation
- Commands: Write operations (create order)
- Queries: Read operations (list orders)
- Separate: Data stores optimized for each

**Problems**
```
15.3.11: Read vs Write optimization
  - Write: normalize for correctness (event store)
  - Read: denormalize for speed (read model cache)

15.3.12: Synchronization
  - When order is created → update write model
  - Publish event → event handler updates read model
  - Query reads from optimized read model

15.3.13: Eventual consistency
  - Write completes immediately
  - Read catches up asynchronously
  - UI must handle: "Just created, might not appear yet"
```

### 17:00-18:00 | Exercise 2: OrderService with Events (1h)

**Task**: Implement OrderService with saga pattern
```rust
pub struct OrderService {
    orders: HashMap<u64, Order>,
    events: Vec<OrderEvent>,
}

// Implement:
// - create_order() -> publishes OrderCreated
// - handle_payment_processed()
// - handle_payment_failed() -> compensates
// - get_order_status() (from events)
```

### 18:00-19:00 | Problem Set 15.3 (1h)
- Solve remaining "Distributed Data" problems
- Questions: When to use events vs saga? Eventual consistency trade-offs?

---

## WEDNESDAY: Communication Patterns (15 hours)

### 09:00-11:00 | REST vs gRPC (2h)

**Concepts & Problems** (15.2, 15.4)
- REST: Human readable, stateless, HTTP-based
- gRPC: Type-safe, binary, multiplexed, low-latency
- Choose based on use-case

**Hands-on Design**
```
User Service:
- CreateUser (write) → gRPC (efficiency, internal)
- GetUser (read) → REST (external API, browser)

Payment Service:
- ProcessPayment (critical) → gRPC (reliability)
- GetPaymentHistory (read) → REST

Inventory Service:
- ReserveItems (write, frequent) → gRPC (latency)
- ListInventory (public API) → REST
```

### 11:00-13:00 | Message Brokers & Queues (2h)

**Concepts**
- RabbitMQ: AMQP, reliable, complex
- Kafka: Event streaming, scalable, ordered
- SQS: Managed queue, simple
- Choose: reliability vs complexity vs cost

**Example**
```rust
// Kafka for order processing
kafka.publish("orders", OrderCreated { ... });

// Kafka guarantees:
// - All subscribers see ALL events
// - Order preserved per partition
// - Replayed by new subscribers

// RabbitMQ for notifications
mq.send("notify.queue", SendEmail { user_id, ... });
// - Point-to-point
// - Acknowledged delivery
// - Processed once
```

### 13:00-14:00 | LUNCH (1h)

### 14:00-16:00 | Choreography vs Orchestration (2h)

**Concepts**
- Choreography: Services listen to events, react independently
- Orchestration: Coordinator tells services what to do
- Choose: Choreography (simple, scalable) vs Orchestration (centralized control)

**Example**
```
CHOREOGRAPHY (Order Creation):
1. OrderService: creates order → publishes OrderCreated
2. PaymentService: hears OrderCreated → charges card
3. InventoryService: hears PaymentSucceeded → reserves items
4. NotificationService: hears OrderConfirmed → sends email

Pros: No coordinator, scales well
Cons: Hard to debug (where did my order go?), circular deps

ORCHESTRATION (Order Saga Coordinator):
1. OrderService: create order → calls SagaCoordinator
2. Coordinator: calls PaymentService("charge")
3. Coordinator: calls InventoryService("reserve")
4. Coordinator: calls NotificationService("send_email")
5. If any fails: rollback in reverse order

Pros: Clear flow, easy to debug
Cons: Single point of failure, coordinator scaling challenge
```

### 16:00-17:00 | Observability in Distributed Systems (1h)

**Concepts**
- You CAN'T log into every service anymore
- Solution: Distributed tracing (X-Trace-ID propagation)
- Each request has unique ID following through all services

**Problems** (15.5)
```
15.5.1: Trace ID propagation
  1. Client makes request with X-Trace-ID: "abc123"
  2. UserService sees "abc123", logs with it
  3. Calls OrderService, passes X-Trace-ID: "abc123"
  4. OrderService logs with "abc123"
  5. → Log system can reconstruct full flow
```

### 17:00-18:00 | Exercise 3: Service Communication (1h)

**Task**: Implement both choreography and orchestration patterns

### 18:00-19:00 | Problem Set 15.4-15.5 (1h)

---

## THURSDAY: API Design & Contracts (15 hours)

### 09:00-11:00 | Versioning & Contracts (2h)

**Problems** (15.2)
```
15.2.1: Version in URL
  - v1: /api/v1/users
  - v2: /api/v2/users (different response)
  - Pros: Clear separation
  - Cons: Two code paths

15.2.2: Version negotiation
  - Accept-Version: v1 header
  - Server responds based on header
  - Pros: Single endpoint
  - Cons: More complex

15.2.3: Backward compatibility
  - Old clients must work with new server
  - Don't remove fields, only add
  - Add new fields as optional

15.2.4: Deprecation
  - Communicate to clients: "Remove v1 by Dec 2024"
  - Grace period: 6-12 months
  - Monitor usage before removal
```

**Hands-on**
```rust
// Version negotiation
pub enum ApiVersion {
    V1,
    V2,
}

pub struct User {
    pub id: u64,
    pub name: String,
    pub email: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phone: Option<String>, // New in V2
}

// Serialize based on version
fn serialize_user(user: &User, version: ApiVersion) -> JsonValue {
    match version {
        ApiVersion::V1 => {
            // Old format: exclude phone
            json!({
                "id": user.id,
                "name": user.name,
                "email": user.email,
            })
        }
        ApiVersion::V2 => {
            // New format: include phone
            serde_json::to_value(user).unwrap()
        }
    }
}
```

### 11:00-13:00 | OpenAPI & Code Generation (2h)

**Concepts**
- OpenAPI (Swagger) spec defines all endpoints
- Code generation: generate client libraries, docs
- Single source of truth

**Example OpenAPI Spec**
```yaml
openapi: 3.0.0
info:
  title: User Service API
  version: 1.0.0

paths:
  /api/users:
    post:
      summary: Create a new user
      requestBody:
        required: true
        content:
          application/json:
            schema:
              $ref: '#/components/schemas/User'
      responses:
        '201':
          description: User created
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/User'

components:
  schemas:
    User:
      type: object
      required: [name, email]
      properties:
        id:
          type: integer
        name:
          type: string
        email:
          type: string
```

### 13:00-14:00 | LUNCH (1h)

### 14:00-16:00 | Error Handling & Resilience (2h)

**Problems** (15.2, 15.4)
```
15.2.5: Standard error responses
  {
    "error": {
      "code": "INVALID_USER",
      "message": "User not found",
      "details": { ... }
    }
  }

15.2.6: Retry logic
  - Immediate retry for transient errors (network)
  - Exponential backoff: 1s, 2s, 4s, 8s...
  - Max retries: 3-5

15.2.7: Idempotence
  - Retirement shouldn't duplicate work
  - Solution: Idempotency key
  - Client: POST /orders with Idempotency-Key: "abc123"
  - Server: Remember "abc123" → same response
```

**Hands-on**
```rust
pub struct RetryPolicy {
    max_retries: u32,
    backoff: Duration,
}

pub async fn call_with_retry<F, T>(
    f: F,
    policy: &RetryPolicy,
) -> Result<T>
where
    F: Fn() -> futures::future::BoxFuture<'static, Result<T>>,
{
    let mut retries = 0;
    loop {
        match f().await {
            Ok(result) => return Ok(result),
            Err(e) if is_retryable(&e) && retries < policy.max_retries => {
                retries += 1;
                let wait_time = policy.backoff * 2_u32.pow(retries - 1);
                tokio::time::sleep(wait_time).await;
            }
            Err(e) => return Err(e),
        }
    }
}
```

### 16:00-17:00 | Rate Limiting & Quotas (1h)

**Concepts**
- Rate limiting: Max 1000 req/min per client
- Token bucket: Replenish X tokens every second
- Quota: Monthly limit (1M API calls/month)

### 17:00-18:00 | Exercise 4: API Gateway (1h)

**Task**: Build API gateway with versioning, rate limiting, circuit breaker

### 18:00-19:00 | Problem Set 15.2 (1h)

---

## FRIDAY: Capstone - Microservice E-Commerce (15 hours)

### 09:00-11:00 | Design Phase (2h)

**Task**: Design complete microservice system

**System Requirements**
```
1. Users can register, login, manage profile
2. Browse products, add to cart
3. Create orders with payment
4. Track order status
5. Get notifications (email/SMS)
6. Analytics: which products sell

Design:
- 7 services: User, Product, Cart, Order, Payment, Notification, Analytics
- 7 databases: each service owns its data
- Message bus: Kafka for events
- API Gateway: routes requests, handles auth
- Service Discovery: dynamic registration
```

**Architecture Diagram**
```
Client
  ↓
API Gateway (Auth, Routing, Rate Limit)
  ├→ UserService (users, profiles)
  ├→ ProductService (products, search)
  ├→ OrderService (orders, saga orchestration)
  ├→ PaymentService (payments, async)
  ├→ NotificationService (emails, sms)
  └→ AnalyticsService (read events, aggregates)

All services publish events → Kafka
All services subscribe to events → Kafka
```

### 11:00-13:00 | Service Implementation (2h)

**UserService**
```rust
pub struct UserService { ... }

impl UserService {
    pub async fn register(&self, name: String, email: String) -> Result<User>;
    pub async fn get_profile(&self, user_id: u64) -> Result<User>;
    pub async fn update_profile(&self, user_id: u64, updates: Updates) -> Result<User>;
}
```

**ProductService**
```rust
pub struct ProductService { ... }

impl ProductService {
    pub async fn search(&self, query: String) -> Vec<Product>;
    pub async fn get_product(&self, id: u64) -> Product;
    pub async fn list_products(&self, page: u32) -> Page<Product>;
}
```

**OrderService**
```rust
pub struct OrderService { ... }

impl OrderService {
    pub async fn create_order(
        &self,
        user_id: u64,
        items: Vec<OrderItem>,
    ) -> Result<Order>;
    // Internally calls PaymentService, notifies customers
}
```

### 13:00-14:00 | LUNCH (1h)

### 14:00-17:00 | Integration & Testing (3h)

**Integration**
- All services communicate via Kafka
- Each publishes events
- Others subscribe

**Testing**
```
Unit tests: Each service independently
Integration tests: Service + message bus
End-to-end: Full system

Example E2E test:
1. Register user
2. Create order
3. Verify Payment service was called
4. Verify email was queued
5. Verify Analytics recorded event
```

**Canary Deployment**
```
1. Deploy new version to 1% of servers
2. Monitor errors, latency
3. 0% errors? → 10%
4. 10% errors? → Rollback
5. Gradually increase: 25%, 50%, 100%
```

### 17:00-19:00 | Presentation & Retrospective (2h)

**What You Built**
- 7 microservices with clear responsibilities
- Event-driven communication
- Graceful degradation (one service down ≠ whole system down)
- Supports millions of users

**Lessons**
- Benefits: Team autonomy, independent scaling, technology diversity
- Costs: Operational complexity, debugging is harder, eventual consistency
- When to use: >20 people, multiple domains, different tech needs

**Next: Week 16 - How to deploy this to Kubernetes**

---

# WEEK 16: CLOUD DEPLOYMENT & OPERATIONS
**75 hours | Platform: Docker, Kubernetes, AWS**

## Monday-Tuesday: Containerization & Docker (30 hours)
- Build Docker images
- Multi-stage builds (small images)
- Push to registry (ECR, DockerHub)
- Security scanning (CVEs)
- Run containers locally

## Wednesday: Kubernetes Deployment (15 hours)
- Write deployment manifests (YAML)
- Service discovery
- Rolling updates
- StatefulSets (for databases)
- Network policies

## Thursday-Friday: Cloud Platforms & CI/CD (30 hours)
- AWS: EC2, ECS, Lambda
- Terraform: Infrastructure as Code
- GitHub Actions: CI/CD pipeline
- Deploy Week 15 services to cloud

---

# WEEK 17: PERFORMANCE TUNING & OPTIMIZATION
**75 hours | Profile, Optimize, Scale**

## Monday-Tuesday: Profiling & Metrics (30 hours)
- CPU profiling: flamegraph
- Memory profiling: allocations
- Database profiling: slow queries
- Distributed tracing: request flows

## Wednesday: Caching & Database Optimization (15 hours)
- Redis: multi-level caching
- Query optimization: indexes
- Connection pooling
- Read replicas

## Thursday-Friday: Load Testing & Optimization (30 hours)
- k6 / Apache JMeter: load generation
- Target: 10K-100K req/sec
- Identify bottlenecks
- Optimize critical paths

---

# WEEK 18: INTEGRATION & PRODUCTION SYSTEMS
**75 hours | Enterprise-Grade Reliability**

## Monday-Tuesday: System Design (30 hours)
- Design from scratch: 1M concurrent users
- Trade-off analysis
- Cost modeling
- Scalability planning

## Wednesday-Thursday: Production Hardening (30 hours)
- Runbooks: how to operate
- Disaster recovery: multi-region failover
- Health checks: 99.99% uptime
- Compliance: GDPR, PCI-DSS

## Friday: Capstone & Future (15h)
- Present production system
- Identify future improvements
- Plan advanced learning path
