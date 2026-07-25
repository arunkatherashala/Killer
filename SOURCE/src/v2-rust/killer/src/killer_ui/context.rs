//! **Context** — React-like Context API for prop-drilling avoidance.
//!
//! `ContextProvider`: stores a value accessible by any descendant.
//! `ContextConsumer`: reads nearest ancestor provider's value.
//! `ContextStore`: manages multiple named contexts with scoped nesting.

use std::collections::HashMap;

// ══════════════════════════════════════════════════════════════════════════════
// Context Value
// ══════════════════════════════════════════════════════════════════════════════

/// Dynamically-typed context value.
#[derive(Debug, Clone, PartialEq)]
pub enum ContextValue {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    Str(String),
    List(Vec<ContextValue>),
    Map(Vec<(String, ContextValue)>),
}

impl ContextValue {
    pub fn as_str(&self) -> Option<&str> {
        if let ContextValue::Str(s) = self { Some(s) } else { None }
    }
    pub fn as_int(&self) -> Option<i64> {
        if let ContextValue::Int(i) = self { Some(*i) } else { None }
    }
    pub fn as_bool(&self) -> Option<bool> {
        if let ContextValue::Bool(b) = self { Some(*b) } else { None }
    }
    pub fn as_float(&self) -> Option<f64> {
        if let ContextValue::Float(f) = self { Some(*f) } else { None }
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Context Definition
// ══════════════════════════════════════════════════════════════════════════════

/// A named context with a default value.
#[derive(Debug, Clone)]
pub struct ContextDef {
    pub name: String,
    pub default_value: ContextValue,
}

impl ContextDef {
    pub fn new(name: &str, default: ContextValue) -> Self {
        ContextDef { name: name.into(), default_value: default }
    }
}

/// A provider instance that overrides a context value for its subtree.
#[derive(Debug, Clone)]
pub struct ContextProvider {
    pub context_name: String,
    pub value: ContextValue,
    pub component_id: String,
    pub depth: u32,
}

// ══════════════════════════════════════════════════════════════════════════════
// Context Store
// ══════════════════════════════════════════════════════════════════════════════

/// Manages all contexts: definitions, provider stack, and consumer resolution.
pub struct ContextStore {
    definitions: HashMap<String, ContextDef>,
    /// Stack of active providers per context name (deepest last).
    provider_stacks: HashMap<String, Vec<ContextProvider>>,
}

impl ContextStore {
    pub fn new() -> Self {
        ContextStore {
            definitions: HashMap::new(),
            provider_stacks: HashMap::new(),
        }
    }

    /// Create (register) a context with a default value.
    pub fn create_context(&mut self, name: &str, default: ContextValue) {
        self.definitions.insert(name.into(), ContextDef::new(name, default));
    }

    /// Push a provider onto the stack (entering a provider's subtree).
    pub fn push_provider(&mut self, context_name: &str, value: ContextValue, component_id: &str, depth: u32) {
        let provider = ContextProvider {
            context_name: context_name.into(),
            value,
            component_id: component_id.into(),
            depth,
        };
        self.provider_stacks
            .entry(context_name.into())
            .or_default()
            .push(provider);
    }

    /// Pop the most recent provider (leaving a provider's subtree).
    pub fn pop_provider(&mut self, context_name: &str) -> Option<ContextProvider> {
        self.provider_stacks.get_mut(context_name)?.pop()
    }

    /// Read the current context value (nearest provider, or default).
    pub fn use_context(&self, context_name: &str) -> ContextValue {
        if let Some(stack) = self.provider_stacks.get(context_name) {
            if let Some(provider) = stack.last() {
                return provider.value.clone();
            }
        }
        self.definitions.get(context_name)
            .map(|d| d.default_value.clone())
            .unwrap_or(ContextValue::Null)
    }

    /// Check if a context is defined.
    pub fn has_context(&self, name: &str) -> bool {
        self.definitions.contains_key(name)
    }

    /// Get the provider depth for a context (0 if no provider active).
    pub fn provider_depth(&self, context_name: &str) -> u32 {
        self.provider_stacks.get(context_name)
            .and_then(|s| s.last())
            .map(|p| p.depth)
            .unwrap_or(0)
    }

    /// Update the value of the current provider without push/pop.
    pub fn set_context_value(&mut self, context_name: &str, value: ContextValue) -> bool {
        if let Some(stack) = self.provider_stacks.get_mut(context_name) {
            if let Some(provider) = stack.last_mut() {
                provider.value = value;
                return true;
            }
        }
        false
    }

    pub fn context_count(&self) -> usize { self.definitions.len() }
    pub fn active_provider_count(&self) -> usize {
        self.provider_stacks.values().map(|s| s.len()).sum()
    }
}

impl Default for ContextStore {
    fn default() -> Self { Self::new() }
}

// ══════════════════════════════════════════════════════════════════════════════
// Tests
// ══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_context_value() {
        let mut store = ContextStore::new();
        store.create_context("theme", ContextValue::Str("light".into()));
        assert_eq!(store.use_context("theme"), ContextValue::Str("light".into()));
    }

    #[test]
    fn provider_overrides_default() {
        let mut store = ContextStore::new();
        store.create_context("theme", ContextValue::Str("light".into()));
        store.push_provider("theme", ContextValue::Str("dark".into()), "app", 1);
        assert_eq!(store.use_context("theme"), ContextValue::Str("dark".into()));
    }

    #[test]
    fn nested_providers() {
        let mut store = ContextStore::new();
        store.create_context("locale", ContextValue::Str("en".into()));
        store.push_provider("locale", ContextValue::Str("fr".into()), "page", 1);
        store.push_provider("locale", ContextValue::Str("de".into()), "widget", 2);
        assert_eq!(store.use_context("locale"), ContextValue::Str("de".into()));
        store.pop_provider("locale");
        assert_eq!(store.use_context("locale"), ContextValue::Str("fr".into()));
        store.pop_provider("locale");
        assert_eq!(store.use_context("locale"), ContextValue::Str("en".into()));
    }

    #[test]
    fn undefined_context_returns_null() {
        let store = ContextStore::new();
        assert_eq!(store.use_context("missing"), ContextValue::Null);
    }

    #[test]
    fn set_context_value() {
        let mut store = ContextStore::new();
        store.create_context("count", ContextValue::Int(0));
        store.push_provider("count", ContextValue::Int(1), "counter", 1);
        assert!(store.set_context_value("count", ContextValue::Int(42)));
        assert_eq!(store.use_context("count"), ContextValue::Int(42));
    }

    #[test]
    fn multiple_contexts() {
        let mut store = ContextStore::new();
        store.create_context("theme", ContextValue::Str("light".into()));
        store.create_context("auth", ContextValue::Bool(false));
        store.push_provider("auth", ContextValue::Bool(true), "app", 1);
        assert_eq!(store.context_count(), 2);
        assert_eq!(store.use_context("theme"), ContextValue::Str("light".into()));
        assert_eq!(store.use_context("auth"), ContextValue::Bool(true));
    }

    #[test]
    fn context_value_accessors() {
        assert_eq!(ContextValue::Str("hi".into()).as_str(), Some("hi"));
        assert_eq!(ContextValue::Int(42).as_int(), Some(42));
        assert_eq!(ContextValue::Bool(true).as_bool(), Some(true));
        assert_eq!(ContextValue::Float(3.14).as_float(), Some(3.14));
        assert_eq!(ContextValue::Null.as_str(), None);
    }
}
