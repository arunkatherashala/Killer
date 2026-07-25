//! **React 19 Actions** — Form actions, useFormStatus, useOptimistic.
//!
//! Server actions, optimistic state updates, form status tracking,
//! and action-based mutations with automatic revalidation.

use std::collections::HashMap;

// ══════════════════════════════════════════════════════════════════════════════
// Form Status
// ══════════════════════════════════════════════════════════════════════════════

/// Status of a form action submission.
#[derive(Debug, Clone, PartialEq)]
pub enum FormActionStatus {
    Idle,
    Pending,
    Success(String),
    Error(String),
}

/// Tracks in-flight form action state (React useFormStatus equivalent).
#[derive(Debug, Clone)]
pub struct FormStatus {
    pub pending: bool,
    pub data: Option<HashMap<String, String>>,
    pub method: Option<String>,
    pub action: Option<String>,
}

impl FormStatus {
    pub fn idle() -> Self {
        FormStatus { pending: false, data: None, method: None, action: None }
    }
    pub fn submitting(action: &str, method: &str, data: HashMap<String, String>) -> Self {
        FormStatus { pending: true, data: Some(data), method: Some(method.into()), action: Some(action.into()) }
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Optimistic State
// ══════════════════════════════════════════════════════════════════════════════

/// Optimistic state manager (React useOptimistic equivalent).
#[derive(Debug, Clone)]
pub struct OptimisticState<T: Clone> {
    pub current: T,
    pub optimistic: Option<T>,
    pub pending_action: bool,
}

impl<T: Clone> OptimisticState<T> {
    pub fn new(initial: T) -> Self {
        OptimisticState { current: initial, optimistic: None, pending_action: false }
    }

    /// Apply an optimistic update (shows immediately before server confirms).
    pub fn apply_optimistic(&mut self, value: T) {
        self.optimistic = Some(value);
        self.pending_action = true;
    }

    /// Confirm the server response (replaces current, clears optimistic).
    pub fn confirm(&mut self, server_value: T) {
        self.current = server_value;
        self.optimistic = None;
        self.pending_action = false;
    }

    /// Reject the optimistic update (reverts to current).
    pub fn reject(&mut self) {
        self.optimistic = None;
        self.pending_action = false;
    }

    /// Get the displayed value (optimistic if pending, else current).
    pub fn display(&self) -> &T {
        self.optimistic.as_ref().unwrap_or(&self.current)
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Server Actions
// ══════════════════════════════════════════════════════════════════════════════

/// An action that runs on the server and returns a result.
#[derive(Debug, Clone)]
pub struct ServerAction {
    pub name: String,
    pub method: String,
    pub endpoint: String,
    pub revalidate_paths: Vec<String>,
}

impl ServerAction {
    pub fn new(name: &str, endpoint: &str) -> Self {
        ServerAction {
            name: name.into(),
            method: "POST".into(),
            endpoint: endpoint.into(),
            revalidate_paths: Vec::new(),
        }
    }

    pub fn with_revalidate(mut self, path: &str) -> Self {
        self.revalidate_paths.push(path.into());
        self
    }
}

/// Action dispatcher — manages form submissions and server actions.
#[derive(Debug)]
pub struct ActionDispatcher {
    pub actions: HashMap<String, ServerAction>,
    pub submissions: Vec<ActionSubmission>,
    pub status: FormActionStatus,
}

#[derive(Debug, Clone)]
pub struct ActionSubmission {
    pub action_name: String,
    pub form_data: HashMap<String, String>,
    pub status: FormActionStatus,
    pub timestamp_ms: u64,
}

impl ActionDispatcher {
    pub fn new() -> Self {
        ActionDispatcher {
            actions: HashMap::new(),
            submissions: Vec::new(),
            status: FormActionStatus::Idle,
        }
    }

    pub fn register(&mut self, action: ServerAction) {
        self.actions.insert(action.name.clone(), action);
    }

    /// Submit a form action (begins pending state).
    pub fn submit(&mut self, action_name: &str, data: HashMap<String, String>) -> Option<usize> {
        if !self.actions.contains_key(action_name) { return None; }
        self.status = FormActionStatus::Pending;
        let idx = self.submissions.len();
        self.submissions.push(ActionSubmission {
            action_name: action_name.into(),
            form_data: data,
            status: FormActionStatus::Pending,
            timestamp_ms: 0,
        });
        Some(idx)
    }

    /// Complete a submission with success.
    pub fn complete_success(&mut self, idx: usize, result: &str) {
        if let Some(sub) = self.submissions.get_mut(idx) {
            sub.status = FormActionStatus::Success(result.into());
        }
        self.status = FormActionStatus::Idle;
    }

    /// Complete a submission with error.
    pub fn complete_error(&mut self, idx: usize, error: &str) {
        if let Some(sub) = self.submissions.get_mut(idx) {
            sub.status = FormActionStatus::Error(error.into());
        }
        self.status = FormActionStatus::Error(error.into());
    }

    pub fn is_pending(&self) -> bool { self.status == FormActionStatus::Pending }

    /// Get the paths that need revalidation after a successful action.
    pub fn revalidation_paths(&self, action_name: &str) -> Vec<String> {
        self.actions.get(action_name).map(|a| a.revalidate_paths.clone()).unwrap_or_default()
    }
}

impl Default for ActionDispatcher {
    fn default() -> Self { Self::new() }
}

// ══════════════════════════════════════════════════════════════════════════════
// Tests
// ══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn form_status_idle() {
        let s = FormStatus::idle();
        assert!(!s.pending);
    }

    #[test]
    fn form_status_submitting() {
        let mut data = HashMap::new();
        data.insert("email".into(), "test@x.com".into());
        let s = FormStatus::submitting("/api/login", "POST", data);
        assert!(s.pending);
        assert_eq!(s.action.as_deref(), Some("/api/login"));
    }

    #[test]
    fn optimistic_state() {
        let mut state = OptimisticState::new(vec!["a".to_string(), "b".to_string()]);
        assert_eq!(state.display().len(), 2);

        // Optimistic add
        state.apply_optimistic(vec!["a".to_string(), "b".to_string(), "c".to_string()]);
        assert_eq!(state.display().len(), 3);
        assert!(state.pending_action);

        // Server confirms
        state.confirm(vec!["a".to_string(), "b".to_string(), "c".to_string()]);
        assert_eq!(state.current.len(), 3);
        assert!(!state.pending_action);
    }

    #[test]
    fn optimistic_reject() {
        let mut state = OptimisticState::new(10i64);
        state.apply_optimistic(20);
        assert_eq!(*state.display(), 20);
        state.reject();
        assert_eq!(*state.display(), 10);
    }

    #[test]
    fn server_action_registration() {
        let mut d = ActionDispatcher::new();
        d.register(ServerAction::new("createPost", "/api/posts").with_revalidate("/posts"));
        assert!(d.actions.contains_key("createPost"));
    }

    #[test]
    fn action_submit_lifecycle() {
        let mut d = ActionDispatcher::new();
        d.register(ServerAction::new("login", "/api/auth"));
        let mut data = HashMap::new();
        data.insert("user".into(), "admin".into());
        let idx = d.submit("login", data).unwrap();
        assert!(d.is_pending());

        d.complete_success(idx, "token_abc");
        assert!(!d.is_pending());
        assert!(matches!(&d.submissions[idx].status, FormActionStatus::Success(r) if r == "token_abc"));
    }

    #[test]
    fn action_submit_error() {
        let mut d = ActionDispatcher::new();
        d.register(ServerAction::new("save", "/api/save"));
        let idx = d.submit("save", HashMap::new()).unwrap();
        d.complete_error(idx, "network error");
        assert!(matches!(&d.status, FormActionStatus::Error(e) if e == "network error"));
    }

    #[test]
    fn action_unknown_rejected() {
        let mut d = ActionDispatcher::new();
        assert!(d.submit("nonexistent", HashMap::new()).is_none());
    }

    #[test]
    fn revalidation_paths() {
        let mut d = ActionDispatcher::new();
        d.register(ServerAction::new("update", "/api/update")
            .with_revalidate("/dashboard")
            .with_revalidate("/profile"));
        let paths = d.revalidation_paths("update");
        assert_eq!(paths.len(), 2);
    }
}
