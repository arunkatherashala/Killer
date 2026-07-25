// Weeks 15-18: Advanced Optimization & Cloud Deployment Exercises
// Microservices, cloud deployment, performance optimization, production systems

use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use std::fmt;

// ============================================================================
// EXERCISE 1: Service Design & Decomposition
// ============================================================================

/// Service trait - base for all microservices
pub trait Service: Send + Sync {
    fn name(&self) -> &str;
    fn health_check(&self) -> bool;
    fn shutdown(&mut self);
}

/// User Service - handles user management
pub struct UserService {
    users: Arc<Mutex<HashMap<u64, User>>>,
    next_id: Arc<Mutex<u64>>,
}

#[derive(Clone, Debug)]
pub struct User {
    pub id: u64,
    pub name: String,
    pub email: String,
}

impl UserService {
    pub fn new() -> Self {
        UserService {
            users: Arc::new(Mutex::new(HashMap::new())),
            next_id: Arc::new(Mutex::new(1)),
        }
    }

    pub fn create_user(&self, name: String, email: String) -> Result<User, String> {
        let mut id_lock = self.next_id.lock().unwrap();
        let id = *id_lock;
        *id_lock += 1;

        let user = User { id, name, email };
        self.users.lock().unwrap().insert(id, user.clone());
        Ok(user)
    }

    pub fn get_user(&self, id: u64) -> Option<User> {
        self.users.lock().unwrap().get(&id).cloned()
    }

    pub fn list_users(&self) -> Vec<User> {
        self.users.lock().unwrap().values().cloned().collect()
    }
}

impl Service for UserService {
    fn name(&self) -> &str {
        "UserService"
    }

    fn health_check(&self) -> bool {
        self.users.lock().is_ok()
    }

    fn shutdown(&mut self) {
        self.users.lock().unwrap().clear();
    }
}

// ============================================================================
// EXERCISE 2: API Design & Contracts
// ============================================================================

/// API Request/Response for service communication
#[derive(Clone, Debug)]
pub enum ApiRequest {
    CreateUser { name: String, email: String },
    GetUser { id: u64 },
    ListUsers,
    CreateOrder { user_id: u64, items: Vec<String> },
}

#[derive(Clone, Debug)]
pub enum ApiResponse {
    User(User),
    UserList(Vec<User>),
    Order(Order),
    Error(String),
    Success,
}

#[derive(Clone, Debug)]
pub struct Order {
    pub id: u64,
    pub user_id: u64,
    pub items: Vec<String>,
    pub total: f64,
}

/// Service gateway - handles API routing
pub struct ServiceGateway {
    user_service: Arc<UserService>,
    order_service: Arc<OrderService>,
    request_count: Arc<Mutex<u64>>,
    error_count: Arc<Mutex<u64>>,
}

impl ServiceGateway {
    pub fn new(user_service: Arc<UserService>, order_service: Arc<OrderService>) -> Self {
        ServiceGateway {
            user_service,
            order_service,
            request_count: Arc::new(Mutex::new(0)),
            error_count: Arc::new(Mutex::new(0)),
        }
    }

    pub fn handle_request(&self, req: ApiRequest) -> ApiResponse {
        *self.request_count.lock().unwrap() += 1;

        match req {
            ApiRequest::CreateUser { name, email } => {
                match self.user_service.create_user(name, email) {
                    Ok(user) => ApiResponse::User(user),
                    Err(e) => {
                        *self.error_count.lock().unwrap() += 1;
                        ApiResponse::Error(e)
                    }
                }
            }
            ApiRequest::GetUser { id } => {
                match self.user_service.get_user(id) {
                    Some(user) => ApiResponse::User(user),
                    None => {
                        *self.error_count.lock().unwrap() += 1;
                        ApiResponse::Error(format!("User {} not found", id))
                    }
                }
            }
            ApiRequest::ListUsers => ApiResponse::UserList(self.user_service.list_users()),
            ApiRequest::CreateOrder { user_id, items } => {
                match self.order_service.create_order(user_id, items) {
                    Ok(order) => ApiResponse::Order(order),
                    Err(e) => {
                        *self.error_count.lock().unwrap() += 1;
                        ApiResponse::Error(e)
                    }
                }
            }
        }
    }

    pub fn metrics(&self) -> Metrics {
        let requests = *self.request_count.lock().unwrap();
        let errors = *self.error_count.lock().unwrap();
        Metrics {
            requests,
            errors,
            error_rate: if requests > 0 {
                (errors as f64) / (requests as f64)
            } else {
                0.0
            },
        }
    }
}

#[derive(Debug, Clone)]
pub struct Metrics {
    pub requests: u64,
    pub errors: u64,
    pub error_rate: f64,
}

// ============================================================================
// EXERCISE 3: Order Service with Distributed Data
// ============================================================================

pub struct OrderService {
    orders: Arc<Mutex<HashMap<u64, Order>>>,
    next_id: Arc<Mutex<u64>>,
    users: Arc<Mutex<HashMap<u64, User>>>, // reference to user data
}

impl OrderService {
    pub fn new(users: Arc<Mutex<HashMap<u64, User>>>) -> Self {
        OrderService {
            orders: Arc::new(Mutex::new(HashMap::new())),
            next_id: Arc::new(Mutex::new(1)),
            users,
        }
    }

    pub fn create_order(&self, user_id: u64, items: Vec<String>) -> Result<Order, String> {
        // Verify user exists (distributed data concern)
        {
            let users = self.users.lock().unwrap();
            if !users.contains_key(&user_id) {
                return Err(format!("User {} not found", user_id));
            }
        }

        let mut id_lock = self.next_id.lock().unwrap();
        let id = *id_lock;
        *id_lock += 1;

        let total = items.len() as f64 * 10.0; // simple pricing
        let order = Order { id, user_id, items, total };

        self.orders.lock().unwrap().insert(id, order.clone());
        Ok(order)
    }

    pub fn get_order(&self, id: u64) -> Option<Order> {
        self.orders.lock().unwrap().get(&id).cloned()
    }

    pub fn list_orders(&self) -> Vec<Order> {
        self.orders.lock().unwrap().values().cloned().collect()
    }
}

impl Service for OrderService {
    fn name(&self) -> &str {
        "OrderService"
    }

    fn health_check(&self) -> bool {
        self.orders.lock().is_ok() && self.users.lock().is_ok()
    }

    fn shutdown(&mut self) {
        self.orders.lock().unwrap().clear();
    }
}

// ============================================================================
// EXERCISE 4: Caching Layer (Week 17)
// ============================================================================

pub struct Cache<K: Clone + Eq + std::hash::Hash, V: Clone> {
    data: Arc<Mutex<HashMap<K, CacheEntry<V>>>>,
    max_size: usize,
    hits: Arc<Mutex<u64>>,
    misses: Arc<Mutex<u64>>,
}

#[derive(Clone)]
struct CacheEntry<V: Clone> {
    value: V,
    created_at: Instant,
    ttl: Duration,
}

impl<K: Clone + Eq + std::hash::Hash, V: Clone> Cache<K, V> {
    pub fn new(max_size: usize) -> Self {
        Cache {
            data: Arc::new(Mutex::new(HashMap::new())),
            max_size,
            hits: Arc::new(Mutex::new(0)),
            misses: Arc::new(Mutex::new(0)),
        }
    }

    pub fn get(&self, key: &K) -> Option<V> {
        let mut cache = self.data.lock().unwrap();

        if let Some(entry) = cache.get(key) {
            if entry.created_at.elapsed() < entry.ttl {
                *self.hits.lock().unwrap() += 1;
                return Some(entry.value.clone());
            } else {
                cache.remove(key);
            }
        }

        *self.misses.lock().unwrap() += 1;
        None
    }

    pub fn put(&self, key: K, value: V, ttl: Duration) {
        let mut cache = self.data.lock().unwrap();

        if cache.len() >= self.max_size && !cache.contains_key(&key) {
            // Simple eviction: remove oldest entry
            if let Some(oldest_key) = cache.keys().next().cloned() {
                cache.remove(&oldest_key);
            }
        }

        cache.insert(
            key,
            CacheEntry {
                value,
                created_at: Instant::now(),
                ttl,
            },
        );
    }

    pub fn stats(&self) -> CacheStats {
        let hits = *self.hits.lock().unwrap();
        let misses = *self.misses.lock().unwrap();
        let total = hits + misses;

        CacheStats {
            hits,
            misses,
            hit_rate: if total > 0 {
                (hits as f64) / (total as f64)
            } else {
                0.0
            },
            size: self.data.lock().unwrap().len(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct CacheStats {
    pub hits: u64,
    pub misses: u64,
    pub hit_rate: f64,
    pub size: usize,
}

// ============================================================================
// EXERCISE 5: Circuit Breaker Pattern (Week 16)
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    Closed,   // Normal operation
    Open,     // Failing, reject requests
    HalfOpen, // Testing if recovered
}

pub struct CircuitBreaker {
    state: Arc<Mutex<CircuitState>>,
    failure_count: Arc<Mutex<u32>>,
    success_count: Arc<Mutex<u32>>,
    failure_threshold: u32,
    success_threshold: u32,
    timeout: Duration,
    last_failure: Arc<Mutex<Option<Instant>>>,
}

impl CircuitBreaker {
    pub fn new(failure_threshold: u32, success_threshold: u32, timeout: Duration) -> Self {
        CircuitBreaker {
            state: Arc::new(Mutex::new(CircuitState::Closed)),
            failure_count: Arc::new(Mutex::new(0)),
            success_count: Arc::new(Mutex::new(0)),
            failure_threshold,
            success_threshold,
            timeout,
            last_failure: Arc::new(Mutex::new(None)),
        }
    }

    pub fn call<F, T>(&self, f: F) -> Result<T, String>
    where
        F: FnOnce() -> Result<T, String>,
    {
        let state = *self.state.lock().unwrap();

        match state {
            CircuitState::Closed => {
                match f() {
                    Ok(result) => {
                        *self.failure_count.lock().unwrap() = 0;
                        Ok(result)
                    }
                    Err(e) => {
                        let mut failures = self.failure_count.lock().unwrap();
                        *failures += 1;

                        if *failures >= self.failure_threshold {
                            *self.state.lock().unwrap() = CircuitState::Open;
                            *self.last_failure.lock().unwrap() = Some(Instant::now());
                        }

                        Err(e)
                    }
                }
            }
            CircuitState::Open => {
                if let Some(last_failure) = *self.last_failure.lock().unwrap() {
                    if last_failure.elapsed() >= self.timeout {
                        *self.state.lock().unwrap() = CircuitState::HalfOpen;
                        *self.success_count.lock().unwrap() = 0;
                        // Retry
                        self.call(f)
                    } else {
                        Err("Circuit breaker open".to_string())
                    }
                } else {
                    Err("Circuit breaker open".to_string())
                }
            }
            CircuitState::HalfOpen => {
                match f() {
                    Ok(result) => {
                        let mut success = self.success_count.lock().unwrap();
                        *success += 1;

                        if *success >= self.success_threshold {
                            *self.state.lock().unwrap() = CircuitState::Closed;
                            *self.failure_count.lock().unwrap() = 0;
                        }

                        Ok(result)
                    }
                    Err(e) => {
                        *self.state.lock().unwrap() = CircuitState::Open;
                        *self.last_failure.lock().unwrap() = Some(Instant::now());
                        Err(e)
                    }
                }
            }
        }
    }

    pub fn state(&self) -> CircuitState {
        *self.state.lock().unwrap()
    }
}

// ============================================================================
// EXERCISE 6: Load Balancer (Week 16)
// ============================================================================

pub struct LoadBalancer<T: Clone> {
    servers: Vec<T>,
    current: Arc<Mutex<usize>>,
}

impl<T: Clone> LoadBalancer<T> {
    pub fn new(servers: Vec<T>) -> Self {
        if servers.is_empty() {
            panic!("At least one server required");
        }

        LoadBalancer {
            servers,
            current: Arc::new(Mutex::new(0)),
        }
    }

    pub fn next(&self) -> T {
        let mut idx = self.current.lock().unwrap();
        let server = self.servers[*idx].clone();
        *idx = (*idx + 1) % self.servers.len();
        server
    }

    pub fn select_least_loaded(&self, loads: &[u32]) -> T {
        let min_idx = loads
            .iter()
            .enumerate()
            .min_by_key(|(_, &load)| load)
            .map(|(idx, _)| idx)
            .unwrap_or(0);

        self.servers[min_idx].clone()
    }
}

// ============================================================================
// EXERCISE 7: Distributed Request Tracing (Week 15)
// ============================================================================

#[derive(Clone, Debug)]
pub struct TraceContext {
    pub trace_id: String,
    pub span_id: String,
    pub parent_span_id: Option<String>,
}

impl TraceContext {
    pub fn new() -> Self {
        use std::collections::hash_map::RandomState;
        use std::hash::{BuildHasher, Hash, Hasher};

        let mut hasher = RandomState::new().build_hasher();
        std::time::SystemTime::now().hash(&mut hasher);

        let trace_id = format!("{:x}", hasher.finish());
        let span_id = format!("{:x}", hasher.finish());

        TraceContext {
            trace_id,
            span_id,
            parent_span_id: None,
        }
    }

    pub fn child_span(&self) -> TraceContext {
        use std::collections::hash_map::RandomState;
        use std::hash::{BuildHasher, Hash, Hasher};

        let mut hasher = RandomState::new().build_hasher();
        std::time::SystemTime::now().hash(&mut hasher);

        TraceContext {
            trace_id: self.trace_id.clone(),
            span_id: format!("{:x}", hasher.finish()),
            parent_span_id: Some(self.span_id.clone()),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Span {
    pub context: TraceContext,
    pub operation: String,
    pub start: Instant,
    pub duration: Duration,
    pub status: String,
}

pub struct TracingCollector {
    spans: Arc<Mutex<Vec<Span>>>,
}

impl TracingCollector {
    pub fn new() -> Self {
        TracingCollector {
            spans: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn record_span(&self, span: Span) {
        self.spans.lock().unwrap().push(span);
    }

    pub fn get_trace(&self, trace_id: &str) -> Vec<Span> {
        self.spans
            .lock()
            .unwrap()
            .iter()
            .filter(|s| s.context.trace_id == trace_id)
            .cloned()
            .collect()
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_service() {
        let service = UserService::new();

        let user = service.create_user("Alice".to_string(), "alice@example.com".to_string());
        assert!(user.is_ok());

        let user = user.unwrap();
        assert_eq!(user.id, 1);
        assert_eq!(user.name, "Alice");

        let retrieved = service.get_user(1);
        assert!(retrieved.is_some());
    }

    #[test]
    fn test_service_gateway() {
        let user_service = Arc::new(UserService::new());
        let order_service = Arc::new(OrderService::new(user_service.users.clone()));
        let gateway = ServiceGateway::new(user_service, order_service);

        let resp = gateway.handle_request(ApiRequest::CreateUser {
            name: "Bob".to_string(),
            email: "bob@example.com".to_string(),
        });

        match resp {
            ApiResponse::User(_) => assert!(true),
            _ => panic!("Expected User response"),
        }

        let metrics = gateway.metrics();
        assert_eq!(metrics.requests, 1);
        assert_eq!(metrics.errors, 0);
    }

    #[test]
    fn test_order_service_with_dependency() {
        let user_service = Arc::new(UserService::new());
        let _ = user_service.create_user("Charlie".to_string(), "charlie@example.com".to_string());

        let order_service = OrderService::new(user_service.users.clone());
        let order = order_service.create_order(1, vec!["item1".to_string(), "item2".to_string()]);

        assert!(order.is_ok());
        assert_eq!(order.unwrap().total, 20.0);
    }

    #[test]
    fn test_cache_put_get() {
        let cache: Cache<String, String> = Cache::new(10);

        cache.put(
            "key1".to_string(),
            "value1".to_string(),
            Duration::from_secs(10),
        );

        let retrieved = cache.get(&"key1".to_string());
        assert_eq!(retrieved, Some("value1".to_string()));

        let stats = cache.stats();
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.misses, 0);
    }

    #[test]
    fn test_cache_expiration() {
        let cache: Cache<String, String> = Cache::new(10);

        cache.put(
            "key1".to_string(),
            "value1".to_string(),
            Duration::from_millis(1),
        );

        std::thread::sleep(Duration::from_millis(2));

        let retrieved = cache.get(&"key1".to_string());
        assert_eq!(retrieved, None);

        let stats = cache.stats();
        assert_eq!(stats.misses, 1);
    }

    #[test]
    fn test_circuit_breaker_closed() {
        let breaker = CircuitBreaker::new(3, 2, Duration::from_secs(1));

        let result = breaker.call(|| Ok("success".to_string()));
        assert!(result.is_ok());
        assert_eq!(breaker.state(), CircuitState::Closed);
    }

    #[test]
    fn test_circuit_breaker_opens() {
        let breaker = CircuitBreaker::new(2, 2, Duration::from_secs(1));

        let _ = breaker.call(|| Err("failed".to_string()));
        let _ = breaker.call(|| Err("failed".to_string()));

        assert_eq!(breaker.state(), CircuitState::Open);

        let result = breaker.call(|| Ok("success".to_string()));
        assert!(result.is_err());
    }

    #[test]
    fn test_load_balancer_round_robin() {
        let lb = LoadBalancer::new(vec![1, 2, 3]);

        assert_eq!(lb.next(), 1);
        assert_eq!(lb.next(), 2);
        assert_eq!(lb.next(), 3);
        assert_eq!(lb.next(), 1);
    }

    #[test]
    fn test_trace_context() {
        let trace = TraceContext::new();
        assert!(!trace.trace_id.is_empty());
        assert!(!trace.span_id.is_empty());
        assert!(trace.parent_span_id.is_none());

        let child = trace.child_span();
        assert_eq!(child.trace_id, trace.trace_id);
        assert_ne!(child.span_id, trace.span_id);
        assert_eq!(child.parent_span_id, Some(trace.span_id.clone()));
    }
}
