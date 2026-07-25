//! **SPA router** — client-side navigation with parameterized routes, guards, and history.
//!
//! Pattern-based matching: `/users/:id`, `/posts/:id/comments`, `/settings/*`.
//! Route guards can block navigation (auth, unsaved changes, etc.).
//! History stack for back/forward navigation.

use std::collections::HashMap;

// ── Route pattern ────────────────────────────────────────────────────────────

pub type RouteId = u64;

/// A registered route definition.
#[derive(Debug, Clone)]
pub struct RouteDef {
    pub id: RouteId,
    /// Pattern like "/users/:id" or "/settings/*"
    pub pattern: String,
    /// Action tag for the component/handler to render.
    pub action: String,
    /// Guard action tags (legacy string guards — evaluated by contains("block")).
    pub guards: Vec<String>,
    /// Real predicate-based guards.
    pub route_guards: Vec<RouteGuard>,
    /// Parsed segments for matching.
    segments: Vec<RouteSegment>,
}

#[derive(Debug, Clone, PartialEq)]
enum RouteSegment {
    Literal(String),
    Param(String),
    Wildcard,
}

fn parse_segments(pattern: &str) -> Vec<RouteSegment> {
    pattern.trim_matches('/')
        .split('/')
        .filter(|s| !s.is_empty())
        .map(|seg| {
            if seg == "*" {
                RouteSegment::Wildcard
            } else if let Some(name) = seg.strip_prefix(':') {
                RouteSegment::Param(name.to_string())
            } else {
                RouteSegment::Literal(seg.to_string())
            }
        })
        .collect()
}

/// Result of matching a path against a route pattern.
#[derive(Debug, Clone)]
pub struct RouteMatch {
    pub route_id: RouteId,
    pub action: String,
    pub params: HashMap<String, String>,
    /// Query parameters (?key=value&...).
    pub query: HashMap<String, String>,
    /// Wildcard capture (everything after `*` segment).
    pub wildcard: Option<String>,
}

/// Guard evaluation result.
#[derive(Debug, Clone, PartialEq)]
pub enum GuardResult {
    Allow,
    Block(String),
}

/// A route guard: predicate that decides whether navigation proceeds.
#[derive(Debug, Clone)]
pub struct RouteGuard {
    pub name: String,
    /// Predicate kind: "allow_all", "block_all", "require_param:key", "custom".
    pub kind: GuardKind,
}

#[derive(Debug, Clone, PartialEq)]
pub enum GuardKind {
    /// Always allows navigation.
    AllowAll,
    /// Always blocks navigation.
    BlockAll,
    /// Blocks unless the given route param exists.
    RequireParam(String),
    /// Blocks unless the given query param exists.
    RequireQuery(String),
    /// Custom: evaluated by the caller via the action tag.
    Custom(String),
}

impl RouteGuard {
    pub fn evaluate(&self, route_match: &RouteMatch) -> GuardResult {
        match &self.kind {
            GuardKind::AllowAll => GuardResult::Allow,
            GuardKind::BlockAll => GuardResult::Block(format!("guard '{}' blocks all", self.name)),
            GuardKind::RequireParam(param) => {
                if route_match.params.contains_key(param) {
                    GuardResult::Allow
                } else {
                    GuardResult::Block(format!("guard '{}': missing param '{}'", self.name, param))
                }
            }
            GuardKind::RequireQuery(key) => {
                if route_match.query.contains_key(key) {
                    GuardResult::Allow
                } else {
                    GuardResult::Block(format!("guard '{}': missing query '{}'", self.name, key))
                }
            }
            GuardKind::Custom(action) => {
                // Custom guards: if the action tag contains "block", block; else allow.
                // In real usage the VM would evaluate the action tag.
                if action.contains("block") {
                    GuardResult::Block(format!("guard '{}' custom blocked", self.name))
                } else {
                    GuardResult::Allow
                }
            }
        }
    }
}

// ── Navigation event ─────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct NavigationEvent {
    pub from: Option<String>,
    pub to: String,
    pub params: HashMap<String, String>,
    pub blocked_by: Option<String>,
}

// ── Router ───────────────────────────────────────────────────────────────────

/// SPA router with route matching, guards, and history stack.
#[derive(Debug)]
pub struct Router {
    routes: Vec<RouteDef>,
    next_id: RouteId,
    /// Current active path.
    pub current_path: String,
    /// Current matched route.
    pub current_match: Option<RouteMatch>,
    /// Navigation history (for back/forward).
    pub history: Vec<String>,
    /// Current position in history.
    pub history_index: usize,
    /// Navigation event log.
    pub nav_log: Vec<NavigationEvent>,
    /// Not-found action (404 handler).
    pub not_found_action: String,
    /// Global guards (run on every navigation) — legacy string form.
    pub global_guards: Vec<String>,
    /// Global predicate-based guards.
    pub global_route_guards: Vec<RouteGuard>,
}

impl Router {
    pub fn new() -> Self {
        Self {
            routes: Vec::new(),
            next_id: 1,
            current_path: "/".to_string(),
            current_match: None,
            history: vec!["/".to_string()],
            history_index: 0,
            nav_log: Vec::new(),
            not_found_action: "not_found".to_string(),
            global_guards: Vec::new(),
            global_route_guards: Vec::new(),
        }
    }

    /// Add a route. Returns the route ID.
    pub fn add_route(&mut self, pattern: &str, action: &str) -> RouteId {
        let id = self.next_id;
        self.next_id += 1;
        let segments = parse_segments(pattern);
        self.routes.push(RouteDef {
            id,
            pattern: pattern.to_string(),
            action: action.to_string(),
            guards: Vec::new(),
            route_guards: Vec::new(),
            segments,
        });
        id
    }

    /// Add a route with guards.
    pub fn add_guarded_route(&mut self, pattern: &str, action: &str, guards: Vec<String>) -> RouteId {
        let id = self.add_route(pattern, action);
        if let Some(route) = self.routes.iter_mut().find(|r| r.id == id) {
            route.guards = guards;
        }
        id
    }

    /// Add a global guard (legacy string form).
    pub fn add_global_guard(&mut self, guard: String) {
        self.global_guards.push(guard);
    }

    /// Add a real predicate-based guard to a route.
    pub fn add_route_guard(&mut self, route_id: RouteId, guard: RouteGuard) {
        if let Some(route) = self.routes.iter_mut().find(|r| r.id == route_id) {
            route.route_guards.push(guard);
        }
    }

    /// Add a global predicate-based guard.
    pub fn add_global_route_guard(&mut self, guard: RouteGuard) {
        self.global_route_guards.push(guard);
    }

    /// Match a path against all routes. Returns the first match.
    /// Supports query parameters: `/path?key=val&key2=val2`.
    pub fn match_path(&self, path: &str) -> Option<RouteMatch> {
        let (path_part, query) = parse_path_and_query(path);
        let path_segs: Vec<&str> = path_part.trim_matches('/').split('/').filter(|s| !s.is_empty()).collect();

        for route in &self.routes {
            if let Some(mut m) = try_match(route, &path_segs) {
                m.query = query.clone();
                return Some(m);
            }
        }
        None
    }

    /// Navigate to a new path. Returns the navigation event.
    /// Evaluates both legacy string guards and real predicate-based guards.
    pub fn navigate(&mut self, path: &str) -> NavigationEvent {
        let path = normalize_path(path);
        let old = Some(self.current_path.clone());

        // Build a provisional match for guard evaluation
        let provisional_match = self.match_path(&path);

        // Check global legacy guards
        for guard in &self.global_guards {
            if guard.contains("block") {
                let ev = NavigationEvent {
                    from: old,
                    to: path.clone(),
                    params: HashMap::new(),
                    blocked_by: Some(guard.clone()),
                };
                self.nav_log.push(ev.clone());
                return ev;
            }
        }

        // Check global predicate-based guards
        if let Some(ref m) = provisional_match {
            for guard in &self.global_route_guards {
                if let GuardResult::Block(reason) = guard.evaluate(m) {
                    let ev = NavigationEvent {
                        from: old,
                        to: path.clone(),
                        params: m.params.clone(),
                        blocked_by: Some(reason),
                    };
                    self.nav_log.push(ev.clone());
                    return ev;
                }
            }
        }

        // Check route-specific guards
        if let Some(m) = provisional_match {
            let route = self.routes.iter().find(|r| r.id == m.route_id).cloned();
            if let Some(route) = route {
                // Legacy string guards
                for guard in &route.guards {
                    if guard.contains("block") {
                        let ev = NavigationEvent {
                            from: old,
                            to: path.clone(),
                            params: m.params.clone(),
                            blocked_by: Some(guard.clone()),
                        };
                        self.nav_log.push(ev.clone());
                        return ev;
                    }
                }
                // Predicate-based guards
                for guard in &route.route_guards {
                    if let GuardResult::Block(reason) = guard.evaluate(&m) {
                        let ev = NavigationEvent {
                            from: old,
                            to: path.clone(),
                            params: m.params.clone(),
                            blocked_by: Some(reason),
                        };
                        self.nav_log.push(ev.clone());
                        return ev;
                    }
                }
            }
            self.current_path = path.clone();
            self.current_match = Some(m.clone());

            // Update history
            // Truncate forward history if we're not at the end
            self.history.truncate(self.history_index + 1);
            self.history.push(path.clone());
            self.history_index = self.history.len() - 1;

            let ev = NavigationEvent {
                from: old,
                to: path,
                params: m.params,
                blocked_by: None,
            };
            self.nav_log.push(ev.clone());
            ev
        } else {
            // 404
            self.current_path = path.clone();
            self.current_match = None;
            let ev = NavigationEvent {
                from: old,
                to: path,
                params: HashMap::new(),
                blocked_by: None,
            };
            self.nav_log.push(ev.clone());
            ev
        }
    }

    /// Go back in history. Returns true if successful.
    pub fn back(&mut self) -> bool {
        if self.history_index > 0 {
            self.history_index -= 1;
            let path = self.history[self.history_index].clone();
            self.current_path = path.clone();
            self.current_match = self.match_path(&path);
            true
        } else {
            false
        }
    }

    /// Go forward in history. Returns true if successful.
    pub fn forward(&mut self) -> bool {
        if self.history_index + 1 < self.history.len() {
            self.history_index += 1;
            let path = self.history[self.history_index].clone();
            self.current_path = path.clone();
            self.current_match = self.match_path(&path);
            true
        } else {
            false
        }
    }

    /// Number of registered routes.
    pub fn route_count(&self) -> usize {
        self.routes.len()
    }

    /// Dump routes as JSON.
    pub fn routes_json(&self) -> String {
        let mut s = String::from("[\n");
        for (i, r) in self.routes.iter().enumerate() {
            if i > 0 { s.push_str(",\n"); }
            s.push_str(&format!(
                "  {{\"id\": {}, \"pattern\": \"{}\", \"action\": \"{}\", \"guards\": {:?}}}",
                r.id, r.pattern, r.action, r.guards
            ));
        }
        s.push_str("\n]");
        s
    }
}

impl Default for Router {
    fn default() -> Self { Self::new() }
}

fn normalize_path(path: &str) -> String {
    // Strip query string for normalization, re-append
    let (path_part, query_part) = if let Some(idx) = path.find('?') {
        (&path[..idx], Some(&path[idx..]))
    } else {
        (path, None)
    };
    let mut p = path_part.to_string();
    if !p.starts_with('/') { p.insert(0, '/'); }
    if p.len() > 1 && p.ends_with('/') { p.pop(); }
    if let Some(q) = query_part { p.push_str(q); }
    p
}

/// Parse path and query parameters from a URL path.
fn parse_path_and_query(full_path: &str) -> (String, HashMap<String, String>) {
    if let Some(idx) = full_path.find('?') {
        let path = full_path[..idx].to_string();
        let query_str = &full_path[idx + 1..];
        let mut query = HashMap::new();
        for pair in query_str.split('&') {
            if pair.is_empty() { continue; }
            if let Some(eq) = pair.find('=') {
                let key = &pair[..eq];
                let val = &pair[eq + 1..];
                query.insert(key.to_string(), val.to_string());
            } else {
                query.insert(pair.to_string(), String::new());
            }
        }
        (path, query)
    } else {
        (full_path.to_string(), HashMap::new())
    }
}

fn try_match(route: &RouteDef, path_segs: &[&str]) -> Option<RouteMatch> {
    let mut params = HashMap::new();
    let mut wildcard = None;

    let route_segs = &route.segments;

    for (i, rseg) in route_segs.iter().enumerate() {
        match rseg {
            RouteSegment::Wildcard => {
                // Capture remaining path
                wildcard = Some(path_segs[i..].join("/"));
                return Some(RouteMatch {
                    route_id: route.id,
                    action: route.action.clone(),
                    params,
                    query: HashMap::new(),
                    wildcard,
                });
            }
            RouteSegment::Literal(expected) => {
                if i >= path_segs.len() || path_segs[i] != expected.as_str() {
                    return None;
                }
            }
            RouteSegment::Param(name) => {
                if i >= path_segs.len() {
                    return None;
                }
                params.insert(name.clone(), path_segs[i].to_string());
            }
        }
    }

    // Exact length match (unless wildcard already returned)
    if route_segs.len() != path_segs.len() {
        return None;
    }

    Some(RouteMatch {
        route_id: route.id,
        action: route.action.clone(),
        params,
        query: HashMap::new(),
        wildcard,
    })
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_route_match() {
        let mut router = Router::new();
        router.add_route("/", "home");
        router.add_route("/about", "about");
        let m = router.match_path("/about").unwrap();
        assert_eq!(m.action, "about");
    }

    #[test]
    fn param_route() {
        let mut router = Router::new();
        router.add_route("/users/:id", "user_detail");
        let m = router.match_path("/users/42").unwrap();
        assert_eq!(m.action, "user_detail");
        assert_eq!(m.params["id"], "42");
    }

    #[test]
    fn multi_params() {
        let mut router = Router::new();
        router.add_route("/posts/:pid/comments/:cid", "comment");
        let m = router.match_path("/posts/5/comments/10").unwrap();
        assert_eq!(m.params["pid"], "5");
        assert_eq!(m.params["cid"], "10");
    }

    #[test]
    fn wildcard_route() {
        let mut router = Router::new();
        router.add_route("/files/*", "file_browser");
        let m = router.match_path("/files/docs/readme.md").unwrap();
        assert_eq!(m.wildcard.as_deref(), Some("docs/readme.md"));
    }

    #[test]
    fn no_match_returns_none() {
        let mut router = Router::new();
        router.add_route("/about", "about");
        assert!(router.match_path("/contact").is_none());
    }

    #[test]
    fn navigate_updates_current() {
        let mut router = Router::new();
        router.add_route("/", "home");
        router.add_route("/about", "about");
        router.navigate("/about");
        assert_eq!(router.current_path, "/about");
        assert_eq!(router.current_match.as_ref().unwrap().action, "about");
    }

    #[test]
    fn guard_blocks_navigation() {
        let mut router = Router::new();
        router.add_guarded_route("/admin", "admin", vec!["auth_block".into()]);
        let ev = router.navigate("/admin");
        assert!(ev.blocked_by.is_some());
        assert_ne!(router.current_path, "/admin");
    }

    #[test]
    fn history_back_forward() {
        let mut router = Router::new();
        router.add_route("/", "home");
        router.add_route("/a", "a");
        router.add_route("/b", "b");
        router.navigate("/a");
        router.navigate("/b");
        assert_eq!(router.current_path, "/b");
        assert!(router.back());
        assert_eq!(router.current_path, "/a");
        assert!(router.forward());
        assert_eq!(router.current_path, "/b");
    }

    #[test]
    fn global_guard_blocks_all() {
        let mut router = Router::new();
        router.add_route("/a", "a");
        router.add_global_guard("maintenance_block".into());
        let ev = router.navigate("/a");
        assert!(ev.blocked_by.is_some());
    }

    #[test]
    fn query_params_parsed() {
        let mut router = Router::new();
        router.add_route("/search", "search");
        let m = router.match_path("/search?q=hello&page=2").unwrap();
        assert_eq!(m.action, "search");
        assert_eq!(m.query["q"], "hello");
        assert_eq!(m.query["page"], "2");
    }

    #[test]
    fn query_params_in_navigate() {
        let mut router = Router::new();
        router.add_route("/search", "search");
        let ev = router.navigate("/search?q=test&limit=10");
        assert!(ev.blocked_by.is_none());
        let m = router.current_match.as_ref().unwrap();
        assert_eq!(m.query["q"], "test");
        assert_eq!(m.query["limit"], "10");
    }

    #[test]
    fn real_guard_block_all() {
        let mut router = Router::new();
        let id = router.add_route("/admin", "admin");
        router.add_route_guard(id, RouteGuard {
            name: "auth".into(),
            kind: GuardKind::BlockAll,
        });
        let ev = router.navigate("/admin");
        assert!(ev.blocked_by.is_some());
        assert!(ev.blocked_by.unwrap().contains("blocks all"));
    }

    #[test]
    fn real_guard_require_query() {
        let mut router = Router::new();
        let id = router.add_route("/export", "export");
        router.add_route_guard(id, RouteGuard {
            name: "needs_token".into(),
            kind: GuardKind::RequireQuery("token".into()),
        });
        // Without token — blocked
        let ev = router.navigate("/export");
        assert!(ev.blocked_by.is_some());
        // With token — allowed
        let ev2 = router.navigate("/export?token=abc123");
        assert!(ev2.blocked_by.is_none());
    }

    #[test]
    fn global_predicate_guard() {
        let mut router = Router::new();
        router.add_route("/a", "a");
        router.add_global_route_guard(RouteGuard {
            name: "maintenance".into(),
            kind: GuardKind::BlockAll,
        });
        let ev = router.navigate("/a");
        assert!(ev.blocked_by.is_some());
    }
}
