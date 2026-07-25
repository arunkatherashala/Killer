//! **Dependency Injection** — Angular-style IoC container.
//!
//! `Container` with scopes (Singleton, Transient, Scoped).
//! `Provider` registration with factory functions.
//! Hierarchical containers (parent-child) for module isolation.
//!
//! Zero external deps.

use std::any::Any;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

// ══════════════════════════════════════════════════════════════════════════════
// Service value type (type-erased)
// ══════════════════════════════════════════════════════════════════════════════

/// Type-erased service value stored in the container.
#[derive(Clone)]
pub enum ServiceValue {
    Str(String),
    Int(i64),
    Float(f64),
    Bool(bool),
    List(Vec<ServiceValue>),
    Map(HashMap<String, ServiceValue>),
    Boxed(Arc<dyn Any + Send + Sync>),
}

impl ServiceValue {
    pub fn as_str(&self) -> Option<&str> {
        if let ServiceValue::Str(s) = self { Some(s) } else { None }
    }
    pub fn as_int(&self) -> Option<i64> {
        if let ServiceValue::Int(n) = self { Some(*n) } else { None }
    }
    pub fn as_bool(&self) -> Option<bool> {
        if let ServiceValue::Bool(b) = self { Some(*b) } else { None }
    }
}

impl std::fmt::Debug for ServiceValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ServiceValue::Str(s) => write!(f, "Str({:?})", s),
            ServiceValue::Int(n) => write!(f, "Int({})", n),
            ServiceValue::Float(n) => write!(f, "Float({})", n),
            ServiceValue::Bool(b) => write!(f, "Bool({})", b),
            ServiceValue::List(l) => write!(f, "List({} items)", l.len()),
            ServiceValue::Map(m) => write!(f, "Map({} keys)", m.len()),
            ServiceValue::Boxed(_) => write!(f, "Boxed(...)"),
        }
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Scope
// ══════════════════════════════════════════════════════════════════════════════

/// Lifecycle scope for a service.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Scope {
    /// One instance shared globally.
    Singleton,
    /// New instance every time it's requested.
    Transient,
    /// One instance per scope/module.
    Scoped,
}

// ══════════════════════════════════════════════════════════════════════════════
// Provider
// ══════════════════════════════════════════════════════════════════════════════

type FactoryFn = Arc<dyn Fn(&Container) -> ServiceValue + Send + Sync>;

struct Provider {
    scope: Scope,
    factory: FactoryFn,
    instance: Option<ServiceValue>,
}

// ══════════════════════════════════════════════════════════════════════════════
// Container
// ══════════════════════════════════════════════════════════════════════════════

/// IoC container with hierarchical scopes.
pub struct Container {
    providers: Arc<Mutex<HashMap<String, Provider>>>,
    parent: Option<Arc<Container>>,
    name: String,
}

impl Container {
    pub fn new() -> Self {
        Container {
            providers: Arc::new(Mutex::new(HashMap::new())),
            parent: None,
            name: "root".into(),
        }
    }

    pub fn named(name: &str) -> Self {
        Container {
            providers: Arc::new(Mutex::new(HashMap::new())),
            parent: None,
            name: name.into(),
        }
    }

    /// Create a child container that inherits from this one.
    pub fn child(parent: Arc<Container>, name: &str) -> Self {
        Container {
            providers: Arc::new(Mutex::new(HashMap::new())),
            parent: Some(parent),
            name: name.into(),
        }
    }

    /// Register a constant value.
    pub fn register_value(&self, name: &str, value: ServiceValue) {
        let stored = value.clone();
        let mut providers = self.providers.lock().unwrap();
        providers.insert(name.to_string(), Provider {
            scope: Scope::Singleton,
            factory: Arc::new(move |_| stored.clone()),
            instance: Some(value),
        });
    }

    /// Register a singleton factory (created once, reused).
    pub fn register_singleton<F: Fn(&Container) -> ServiceValue + Send + Sync + 'static>(
        &self, name: &str, factory: F
    ) {
        let mut providers = self.providers.lock().unwrap();
        providers.insert(name.to_string(), Provider {
            scope: Scope::Singleton,
            factory: Arc::new(factory),
            instance: None,
        });
    }

    /// Register a transient factory (new instance each resolve).
    pub fn register_transient<F: Fn(&Container) -> ServiceValue + Send + Sync + 'static>(
        &self, name: &str, factory: F
    ) {
        let mut providers = self.providers.lock().unwrap();
        providers.insert(name.to_string(), Provider {
            scope: Scope::Transient,
            factory: Arc::new(factory),
            instance: None,
        });
    }

    /// Resolve a service by name.
    pub fn resolve(&self, name: &str) -> Option<ServiceValue> {
        // First pass: check for cached instance or clone factory for lazy init.
        let (scope, factory) = {
            let providers = self.providers.lock().unwrap();
            if let Some(provider) = providers.get(name) {
                if let Some(ref inst) = provider.instance {
                    return Some(inst.clone());
                }
                (provider.scope, provider.factory.clone())
            } else {
                // Not found locally — check parent
                drop(providers);
                return if let Some(ref parent) = self.parent {
                    parent.resolve(name)
                } else {
                    None
                };
            }
        };
        // Lock is dropped here — safe to call factory (which may re-lock).
        let value = factory(self);
        // For singletons/scoped, cache the result.
        if scope == Scope::Singleton || scope == Scope::Scoped {
            let mut providers = self.providers.lock().unwrap();
            if let Some(p) = providers.get_mut(name) {
                p.instance = Some(value.clone());
            }
        }
        Some(value)
    }

    /// Check if a service is registered (locally or in parent).
    pub fn has(&self, name: &str) -> bool {
        if self.providers.lock().unwrap().contains_key(name) { return true; }
        if let Some(ref parent) = self.parent { parent.has(name) } else { false }
    }

    /// List all registered service names.
    pub fn service_names(&self) -> Vec<String> {
        self.providers.lock().unwrap().keys().cloned().collect()
    }

    pub fn name(&self) -> &str { &self.name }
}

impl Default for Container {
    fn default() -> Self { Self::new() }
}

// ══════════════════════════════════════════════════════════════════════════════
// Module — group of related services
// ══════════════════════════════════════════════════════════════════════════════

/// A module groups related service registrations.
pub struct Module {
    pub name: String,
    registrations: Vec<(String, Scope, Arc<dyn Fn(&Container) -> ServiceValue + Send + Sync>)>,
}

impl Module {
    pub fn new(name: &str) -> Self {
        Module { name: name.to_string(), registrations: Vec::new() }
    }

    pub fn provide<F: Fn(&Container) -> ServiceValue + Send + Sync + 'static>(
        mut self, name: &str, scope: Scope, factory: F
    ) -> Self {
        self.registrations.push((name.to_string(), scope, Arc::new(factory)));
        self
    }

    /// Apply all registrations to a container.
    pub fn apply_to(self, container: &Container) {
        for (name, scope, factory) in self.registrations {
            let mut providers = container.providers.lock().unwrap();
            providers.insert(name, Provider {
                scope,
                factory,
                instance: None,
            });
        }
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Tests
// ══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn register_and_resolve_value() {
        let c = Container::new();
        c.register_value("db_url", ServiceValue::Str("postgres://localhost/app".into()));
        let val = c.resolve("db_url").unwrap();
        assert_eq!(val.as_str(), Some("postgres://localhost/app"));
    }

    #[test]
    fn singleton_created_once() {
        let c = Container::new();
        let call_count = Arc::new(Mutex::new(0));
        let cc = call_count.clone();
        c.register_singleton("counter", move |_| {
            let mut n = cc.lock().unwrap();
            *n += 1;
            ServiceValue::Int(*n)
        });
        let v1 = c.resolve("counter").unwrap();
        let v2 = c.resolve("counter").unwrap();
        assert_eq!(v1.as_int(), v2.as_int()); // Same instance
        assert_eq!(*call_count.lock().unwrap(), 1); // Factory called once
    }

    #[test]
    fn transient_creates_new_each_time() {
        let c = Container::new();
        let call_count = Arc::new(Mutex::new(0));
        let cc = call_count.clone();
        c.register_transient("id", move |_| {
            let mut n = cc.lock().unwrap();
            *n += 1;
            ServiceValue::Int(*n)
        });
        let v1 = c.resolve("id").unwrap();
        let v2 = c.resolve("id").unwrap();
        assert_ne!(v1.as_int(), v2.as_int()); // Different instances
    }

    #[test]
    fn child_inherits_parent() {
        let parent = Arc::new(Container::new());
        parent.register_value("api_key", ServiceValue::Str("secret".into()));
        let child = Container::child(parent.clone(), "child");
        child.register_value("db", ServiceValue::Str("sqlite".into()));

        // Child can see parent's services
        assert!(child.has("api_key"));
        assert_eq!(child.resolve("api_key").unwrap().as_str(), Some("secret"));
        // Parent can't see child's services
        assert!(!parent.has("db"));
    }

    #[test]
    fn child_overrides_parent() {
        let parent = Arc::new(Container::new());
        parent.register_value("env", ServiceValue::Str("production".into()));
        let child = Container::child(parent.clone(), "test");
        child.register_value("env", ServiceValue::Str("testing".into()));

        assert_eq!(child.resolve("env").unwrap().as_str(), Some("testing"));
        assert_eq!(parent.resolve("env").unwrap().as_str(), Some("production"));
    }

    #[test]
    fn module_applies_services() {
        let auth_module = Module::new("auth")
            .provide("jwt_secret", Scope::Singleton, |_| {
                ServiceValue::Str("my-secret-key".into())
            })
            .provide("token_ttl", Scope::Singleton, |_| {
                ServiceValue::Int(3600)
            });

        let c = Container::new();
        auth_module.apply_to(&c);
        assert_eq!(c.resolve("jwt_secret").unwrap().as_str(), Some("my-secret-key"));
        assert_eq!(c.resolve("token_ttl").unwrap().as_int(), Some(3600));
    }

    #[test]
    fn service_depends_on_another() {
        let c = Container::new();
        c.register_value("db_host", ServiceValue::Str("localhost".into()));
        c.register_value("db_port", ServiceValue::Int(5432));
        c.register_singleton("db_url", |container| {
            let host = container.resolve("db_host").unwrap();
            let port = container.resolve("db_port").unwrap();
            ServiceValue::Str(format!("postgres://{}:{}/app",
                host.as_str().unwrap_or("?"), port.as_int().unwrap_or(0)))
        });
        let url = c.resolve("db_url").unwrap();
        assert_eq!(url.as_str(), Some("postgres://localhost:5432/app"));
    }

    #[test]
    fn missing_service_returns_none() {
        let c = Container::new();
        assert!(c.resolve("nonexistent").is_none());
        assert!(!c.has("nonexistent"));
    }

    #[test]
    fn service_names_list() {
        let c = Container::new();
        c.register_value("a", ServiceValue::Int(1));
        c.register_value("b", ServiceValue::Int(2));
        let names = c.service_names();
        assert_eq!(names.len(), 2);
        assert!(names.contains(&"a".to_string()));
        assert!(names.contains(&"b".to_string()));
    }
}
