//! **Defer Blocks** — Angular @defer equivalent + route resolvers.
//!
//! Lazy template regions with trigger conditions (viewport, idle, timer,
//! interaction, hover). Route resolvers for pre-fetching data.

use std::collections::HashMap;

// ══════════════════════════════════════════════════════════════════════════════
// Defer Triggers
// ══════════════════════════════════════════════════════════════════════════════

/// When a defer block should begin loading.
#[derive(Debug, Clone, PartialEq)]
pub enum DeferTrigger {
    OnViewport,
    OnIdle,
    OnTimer(u64),          // ms
    OnInteraction(String), // element id
    OnHover(String),       // element id
    Immediate,
    /// Prefetch condition (separate from display trigger)
    Prefetch(Box<DeferTrigger>),
}

/// State of a defer block.
#[derive(Debug, Clone, PartialEq)]
pub enum DeferState {
    Placeholder,
    Loading,
    Loaded,
    Error(String),
}

/// A @defer block definition.
#[derive(Debug, Clone)]
pub struct DeferBlock {
    pub id: String,
    pub trigger: DeferTrigger,
    pub state: DeferState,
    pub minimum_ms: Option<u64>,
    pub placeholder_content: Option<String>,
    pub loading_content: Option<String>,
    pub error_content: Option<String>,
}

impl DeferBlock {
    pub fn new(id: &str, trigger: DeferTrigger) -> Self {
        DeferBlock {
            id: id.into(), trigger, state: DeferState::Placeholder,
            minimum_ms: None, placeholder_content: None,
            loading_content: None, error_content: None,
        }
    }

    pub fn with_minimum(mut self, ms: u64) -> Self { self.minimum_ms = Some(ms); self }
    pub fn with_placeholder(mut self, content: &str) -> Self { self.placeholder_content = Some(content.into()); self }
    pub fn with_loading(mut self, content: &str) -> Self { self.loading_content = Some(content.into()); self }
    pub fn with_error(mut self, content: &str) -> Self { self.error_content = Some(content.into()); self }

    /// Trigger loading.
    pub fn start_loading(&mut self) {
        if self.state == DeferState::Placeholder {
            self.state = DeferState::Loading;
        }
    }

    pub fn complete(&mut self) { self.state = DeferState::Loaded; }
    pub fn fail(&mut self, err: &str) { self.state = DeferState::Error(err.into()); }

    pub fn display_content(&self) -> &str {
        match &self.state {
            DeferState::Placeholder => self.placeholder_content.as_deref().unwrap_or(""),
            DeferState::Loading => self.loading_content.as_deref().unwrap_or("Loading..."),
            DeferState::Loaded => "[loaded content]",
            DeferState::Error(e) => self.error_content.as_deref().unwrap_or(e.as_str()),
        }
    }
}

/// Manages all defer blocks in a template.
#[derive(Debug, Default)]
pub struct DeferManager {
    pub blocks: HashMap<String, DeferBlock>,
}

impl DeferManager {
    pub fn new() -> Self { Self::default() }

    pub fn register(&mut self, block: DeferBlock) {
        self.blocks.insert(block.id.clone(), block);
    }

    /// Check if a viewport event should trigger any blocks.
    pub fn check_viewport(&mut self, visible_ids: &[&str]) {
        for id in visible_ids {
            if let Some(block) = self.blocks.get_mut(*id) {
                if block.trigger == DeferTrigger::OnViewport {
                    block.start_loading();
                }
            }
        }
    }

    /// Check idle trigger.
    pub fn check_idle(&mut self) {
        for block in self.blocks.values_mut() {
            if block.trigger == DeferTrigger::OnIdle {
                block.start_loading();
            }
        }
    }

    /// Check timer triggers given elapsed ms.
    pub fn check_timers(&mut self, elapsed_ms: u64) {
        for block in self.blocks.values_mut() {
            if let DeferTrigger::OnTimer(threshold) = block.trigger {
                if elapsed_ms >= threshold {
                    block.start_loading();
                }
            }
        }
    }

    pub fn loading_count(&self) -> usize {
        self.blocks.values().filter(|b| b.state == DeferState::Loading).count()
    }

    pub fn loaded_count(&self) -> usize {
        self.blocks.values().filter(|b| b.state == DeferState::Loaded).count()
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Route Resolvers
// ══════════════════════════════════════════════════════════════════════════════

/// Data resolved before a route activates.
#[derive(Debug, Clone)]
pub enum ResolvedData {
    None,
    Value(String),
    Error(String),
}

/// A route resolver that pre-fetches data.
#[derive(Debug, Clone)]
pub struct RouteResolver {
    pub name: String,
    pub route_pattern: String,
    pub data_key: String,
    pub resolved: ResolvedData,
}

impl RouteResolver {
    pub fn new(name: &str, pattern: &str, key: &str) -> Self {
        RouteResolver { name: name.into(), route_pattern: pattern.into(), data_key: key.into(), resolved: ResolvedData::None }
    }

    pub fn resolve(&mut self, value: &str) {
        self.resolved = ResolvedData::Value(value.into());
    }

    pub fn fail(&mut self, err: &str) {
        self.resolved = ResolvedData::Error(err.into());
    }

    pub fn is_resolved(&self) -> bool { matches!(self.resolved, ResolvedData::Value(_)) }
}

/// Registry of route resolvers.
#[derive(Debug, Default)]
pub struct ResolverRegistry {
    pub resolvers: HashMap<String, Vec<RouteResolver>>,
}

impl ResolverRegistry {
    pub fn new() -> Self { Self::default() }

    pub fn add(&mut self, resolver: RouteResolver) {
        self.resolvers.entry(resolver.route_pattern.clone()).or_default().push(resolver);
    }

    /// Get resolvers for a route, pre-fetch data before activation.
    pub fn resolve_route(&mut self, pattern: &str) -> Vec<&RouteResolver> {
        self.resolvers.get(pattern).map(|v| v.iter().collect()).unwrap_or_default()
    }

    /// Check if all resolvers for a route are done.
    pub fn all_resolved(&self, pattern: &str) -> bool {
        self.resolvers.get(pattern).map(|v| v.iter().all(|r| r.is_resolved())).unwrap_or(true)
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Change Detection
// ══════════════════════════════════════════════════════════════════════════════

/// Change detection strategy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeDetectionStrategy {
    Default,
    OnPush,
}

/// Component change detection state.
#[derive(Debug, Clone)]
pub struct ChangeDetector {
    pub component_id: String,
    pub strategy: ChangeDetectionStrategy,
    pub dirty: bool,
    pub check_count: u64,
}

impl ChangeDetector {
    pub fn new(id: &str, strategy: ChangeDetectionStrategy) -> Self {
        ChangeDetector { component_id: id.into(), strategy, dirty: true, check_count: 0 }
    }

    /// Mark component for check (like Angular markForCheck).
    pub fn mark_for_check(&mut self) { self.dirty = true; }

    /// Run change detection.
    pub fn detect_changes(&mut self) -> bool {
        self.check_count += 1;
        match self.strategy {
            ChangeDetectionStrategy::Default => {
                // Default: always check
                self.dirty = false;
                true
            }
            ChangeDetectionStrategy::OnPush => {
                // OnPush: only check if dirty
                if self.dirty {
                    self.dirty = false;
                    true
                } else {
                    false
                }
            }
        }
    }

    /// Detach from change detection (manual mode).
    pub fn detach(&mut self) {
        self.strategy = ChangeDetectionStrategy::OnPush;
        self.dirty = false;
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// @switch Control Flow
// ══════════════════════════════════════════════════════════════════════════════

/// A @switch block for template control flow.
#[derive(Debug, Clone)]
pub struct SwitchBlock {
    pub expression_value: String,
    pub cases: Vec<(String, String)>,  // (match_value, template_content)
    pub default: Option<String>,
}

impl SwitchBlock {
    pub fn new(value: &str) -> Self {
        SwitchBlock { expression_value: value.into(), cases: Vec::new(), default: None }
    }

    pub fn case(mut self, match_val: &str, content: &str) -> Self {
        self.cases.push((match_val.into(), content.into()));
        self
    }

    pub fn default(mut self, content: &str) -> Self {
        self.default = Some(content.into());
        self
    }

    /// Evaluate: return the matching case content.
    pub fn evaluate(&self) -> Option<&str> {
        for (val, content) in &self.cases {
            if val == &self.expression_value {
                return Some(content.as_str());
            }
        }
        self.default.as_deref()
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Tests
// ══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defer_viewport_trigger() {
        let mut mgr = DeferManager::new();
        mgr.register(DeferBlock::new("comments", DeferTrigger::OnViewport));
        mgr.check_viewport(&["comments"]);
        assert_eq!(mgr.blocks["comments"].state, DeferState::Loading);
    }

    #[test]
    fn defer_timer_trigger() {
        let mut mgr = DeferManager::new();
        mgr.register(DeferBlock::new("analytics", DeferTrigger::OnTimer(1000)));
        mgr.check_timers(500);
        assert_eq!(mgr.blocks["analytics"].state, DeferState::Placeholder);
        mgr.check_timers(1000);
        assert_eq!(mgr.blocks["analytics"].state, DeferState::Loading);
    }

    #[test]
    fn defer_idle_trigger() {
        let mut mgr = DeferManager::new();
        mgr.register(DeferBlock::new("footer", DeferTrigger::OnIdle));
        mgr.check_idle();
        assert_eq!(mgr.blocks["footer"].state, DeferState::Loading);
    }

    #[test]
    fn defer_lifecycle() {
        let mut block = DeferBlock::new("heavy", DeferTrigger::OnViewport)
            .with_placeholder("Skeleton")
            .with_loading("Loading...")
            .with_error("Failed");
        assert_eq!(block.display_content(), "Skeleton");
        block.start_loading();
        assert_eq!(block.display_content(), "Loading...");
        block.complete();
        assert_eq!(block.state, DeferState::Loaded);
    }

    #[test]
    fn route_resolver_basic() {
        let mut reg = ResolverRegistry::new();
        reg.add(RouteResolver::new("userLoader", "/users/:id", "user"));
        assert!(!reg.all_resolved("/users/:id"));
        if let Some(resolvers) = reg.resolvers.get_mut("/users/:id") {
            resolvers[0].resolve("{\"name\":\"Alice\"}");
        }
        assert!(reg.all_resolved("/users/:id"));
    }

    #[test]
    fn change_detection_default() {
        let mut cd = ChangeDetector::new("app", ChangeDetectionStrategy::Default);
        assert!(cd.detect_changes()); // always checks
        assert!(cd.detect_changes()); // always checks again
    }

    #[test]
    fn change_detection_onpush() {
        let mut cd = ChangeDetector::new("list", ChangeDetectionStrategy::OnPush);
        assert!(cd.detect_changes()); // first time, was dirty=true
        assert!(!cd.detect_changes()); // not dirty now
        cd.mark_for_check();
        assert!(cd.detect_changes()); // dirty again
    }

    #[test]
    fn switch_block() {
        let sw = SwitchBlock::new("warning")
            .case("info", "<div class='info'>Info</div>")
            .case("warning", "<div class='warn'>Warning!</div>")
            .case("error", "<div class='err'>Error</div>")
            .default("<div>Unknown</div>");
        assert_eq!(sw.evaluate(), Some("<div class='warn'>Warning!</div>"));
    }

    #[test]
    fn switch_block_default() {
        let sw = SwitchBlock::new("unknown")
            .case("a", "A")
            .default("fallback");
        assert_eq!(sw.evaluate(), Some("fallback"));
    }

    #[test]
    fn switch_block_no_match() {
        let sw = SwitchBlock::new("x").case("a", "A");
        assert_eq!(sw.evaluate(), None);
    }
}
