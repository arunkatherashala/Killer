//! **Error Boundaries** — React-style error catching at component render boundaries.
//!
//! Provides: `ErrorBoundaryDef`, `ErrorFallback`, `ErrorRecovery`.
//! Catches panics/errors during widget render and replaces broken subtree with
//! a fallback UI. Supports retry, reset, and error reporting.

use super::patch::{Widget, Severity};

// ══════════════════════════════════════════════════════════════════════════════
// Error info
// ══════════════════════════════════════════════════════════════════════════════

/// Captured error from a component render failure.
#[derive(Debug, Clone)]
pub struct RenderError {
    pub component_id: String,
    pub message: String,
    pub stack: Vec<String>,
    pub timestamp_ms: u64,
    pub retry_count: u32,
}

impl RenderError {
    pub fn new(component_id: &str, message: &str) -> Self {
        RenderError {
            component_id: component_id.into(),
            message: message.into(),
            stack: Vec::new(),
            timestamp_ms: 0,
            retry_count: 0,
        }
    }

    pub fn with_stack(mut self, frames: Vec<String>) -> Self {
        self.stack = frames;
        self
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Fallback strategies
// ══════════════════════════════════════════════════════════════════════════════

/// What to show when a component errors.
#[derive(Debug, Clone)]
pub enum ErrorFallback {
    /// Show an Alert widget with the error message.
    Alert,
    /// Show a custom widget tree.
    Custom(Vec<Widget>),
    /// Show nothing (hide broken component).
    Hidden,
    /// Show a retry button plus error message.
    RetryButton { label: String },
    /// Show a minimal text placeholder.
    Placeholder(String),
}

impl Default for ErrorFallback {
    fn default() -> Self { ErrorFallback::Alert }
}

/// Recovery strategy after an error.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RecoveryStrategy {
    /// Don't retry, show fallback permanently.
    None,
    /// Retry on user action (click retry button).
    Manual,
    /// Auto-retry up to N times with exponential backoff.
    AutoRetry { max_retries: u32, base_delay_ms: u64 },
}

impl Default for RecoveryStrategy {
    fn default() -> Self { RecoveryStrategy::Manual }
}

// ══════════════════════════════════════════════════════════════════════════════
// Error Boundary Definition
// ══════════════════════════════════════════════════════════════════════════════

/// An error boundary wraps a subtree, catching render errors.
pub struct ErrorBoundaryDef {
    pub id: String,
    pub fallback: ErrorFallback,
    pub recovery: RecoveryStrategy,
    pub on_error: Option<Box<dyn Fn(&RenderError) + Send + Sync>>,
    state: BoundaryState,
    errors: Vec<RenderError>,
    max_errors: usize,
}

#[derive(Debug, Clone, PartialEq)]
#[allow(dead_code)]
enum BoundaryState {
    Normal,
    Error,
    Recovering,
}

impl ErrorBoundaryDef {
    pub fn new(id: &str) -> Self {
        ErrorBoundaryDef {
            id: id.into(),
            fallback: ErrorFallback::default(),
            recovery: RecoveryStrategy::default(),
            on_error: None,
            state: BoundaryState::Normal,
            errors: Vec::new(),
            max_errors: 50,
        }
    }

    pub fn with_fallback(mut self, fb: ErrorFallback) -> Self {
        self.fallback = fb;
        self
    }

    pub fn with_recovery(mut self, r: RecoveryStrategy) -> Self {
        self.recovery = r;
        self
    }

    pub fn with_on_error<F: Fn(&RenderError) + Send + Sync + 'static>(mut self, f: F) -> Self {
        self.on_error = Some(Box::new(f));
        self
    }

    /// Check if boundary is in error state.
    pub fn has_error(&self) -> bool { self.state == BoundaryState::Error }

    /// Get all captured errors.
    pub fn errors(&self) -> &[RenderError] { &self.errors }

    /// Get the most recent error.
    pub fn last_error(&self) -> Option<&RenderError> { self.errors.last() }

    /// Reset the boundary to normal state (user clicked retry).
    pub fn reset(&mut self) {
        self.state = BoundaryState::Normal;
    }

    /// Catch an error from a child render.
    pub fn catch_error(&mut self, error: RenderError) {
        if let Some(ref cb) = self.on_error {
            cb(&error);
        }
        self.errors.push(error);
        if self.errors.len() > self.max_errors {
            self.errors.remove(0);
        }
        self.state = BoundaryState::Error;
    }

    /// Try to render children, catching errors. Returns fallback on failure.
    pub fn try_render<F>(&mut self, render_fn: F) -> Vec<Widget>
    where F: FnOnce() -> Result<Vec<Widget>, RenderError>
    {
        match self.state {
            BoundaryState::Error => self.render_fallback(),
            BoundaryState::Normal | BoundaryState::Recovering => {
                match render_fn() {
                    Ok(widgets) => {
                        self.state = BoundaryState::Normal;
                        widgets
                    }
                    Err(err) => {
                        self.catch_error(err);
                        self.render_fallback()
                    }
                }
            }
        }
    }

    /// Should auto-retry based on recovery strategy?
    pub fn should_retry(&self) -> bool {
        match self.recovery {
            RecoveryStrategy::AutoRetry { max_retries, .. } => {
                if let Some(last) = self.errors.last() {
                    last.retry_count < max_retries
                } else {
                    false
                }
            }
            _ => false,
        }
    }

    /// Get the delay before next retry (exponential backoff).
    pub fn retry_delay_ms(&self) -> u64 {
        match self.recovery {
            RecoveryStrategy::AutoRetry { base_delay_ms, .. } => {
                let count = self.errors.last().map(|e| e.retry_count).unwrap_or(0);
                base_delay_ms * 2u64.pow(count)
            }
            _ => 0,
        }
    }

    fn render_fallback(&self) -> Vec<Widget> {
        let msg = self.errors.last()
            .map(|e| e.message.clone())
            .unwrap_or_else(|| "Unknown error".into());

        match &self.fallback {
            ErrorFallback::Alert => vec![
                Widget::Alert {
                    id: format!("{}_error", self.id),
                    message: format!("Error: {}", msg),
                    severity: Severity::Error,
                    dismissible: true,
                }
            ],
            ErrorFallback::Custom(widgets) => widgets.clone(),
            ErrorFallback::Hidden => vec![],
            ErrorFallback::RetryButton { label } => vec![
                Widget::Alert {
                    id: format!("{}_error", self.id),
                    message: msg,
                    severity: Severity::Error,
                    dismissible: false,
                },
                Widget::Button {
                    id: format!("{}_retry", self.id),
                    label: label.clone(),
                    variant: super::patch::ButtonVariant::Primary,
                    disabled: false,
                },
            ],
            ErrorFallback::Placeholder(text) => vec![
                Widget::Label {
                    id: format!("{}_placeholder", self.id),
                    text: text.clone(),
                },
            ],
        }
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// ErrorBoundaryManager — tracks all boundaries in an app
// ══════════════════════════════════════════════════════════════════════════════

/// Manages a collection of error boundaries across the component tree.
pub struct ErrorBoundaryManager {
    boundaries: Vec<ErrorBoundaryDef>,
    global_errors: Vec<RenderError>,
}

impl ErrorBoundaryManager {
    pub fn new() -> Self {
        ErrorBoundaryManager {
            boundaries: Vec::new(),
            global_errors: Vec::new(),
        }
    }

    pub fn add(&mut self, boundary: ErrorBoundaryDef) {
        self.boundaries.push(boundary);
    }

    pub fn get(&self, id: &str) -> Option<&ErrorBoundaryDef> {
        self.boundaries.iter().find(|b| b.id == id)
    }

    pub fn get_mut(&mut self, id: &str) -> Option<&mut ErrorBoundaryDef> {
        self.boundaries.iter_mut().find(|b| b.id == id)
    }

    /// Report an unhandled error (no boundary caught it).
    pub fn report_unhandled(&mut self, error: RenderError) {
        self.global_errors.push(error);
    }

    /// All boundaries currently in error state.
    pub fn errored_boundaries(&self) -> Vec<&ErrorBoundaryDef> {
        self.boundaries.iter().filter(|b| b.has_error()).collect()
    }

    /// Reset all boundaries.
    pub fn reset_all(&mut self) {
        for b in &mut self.boundaries {
            b.reset();
        }
    }

    /// Total error count across all boundaries.
    pub fn total_errors(&self) -> usize {
        self.boundaries.iter().map(|b| b.errors.len()).sum::<usize>() + self.global_errors.len()
    }

    pub fn boundary_count(&self) -> usize { self.boundaries.len() }
}

impl Default for ErrorBoundaryManager {
    fn default() -> Self { Self::new() }
}

// ══════════════════════════════════════════════════════════════════════════════
// Tests
// ══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normal_render() {
        let mut boundary = ErrorBoundaryDef::new("test");
        let result = boundary.try_render(|| {
            Ok(vec![Widget::Label { id: "ok".into(), text: "Hello".into() }])
        });
        assert_eq!(result.len(), 1);
        assert!(!boundary.has_error());
    }

    #[test]
    fn catch_error_shows_fallback() {
        let mut boundary = ErrorBoundaryDef::new("test");
        let result = boundary.try_render(|| {
            Err(RenderError::new("child", "Something broke"))
        });
        assert!(boundary.has_error());
        assert_eq!(boundary.errors().len(), 1);
        // Default fallback is Alert
        assert_eq!(result.len(), 1);
        if let Widget::Alert { severity, .. } = &result[0] {
            assert_eq!(*severity, Severity::Error);
        } else {
            panic!("Expected Alert widget");
        }
    }

    #[test]
    fn retry_button_fallback() {
        let mut boundary = ErrorBoundaryDef::new("test")
            .with_fallback(ErrorFallback::RetryButton { label: "Retry".into() });
        let _ = boundary.try_render(|| Err(RenderError::new("c", "fail")));
        assert!(boundary.has_error());
        // Reset and retry
        boundary.reset();
        assert!(!boundary.has_error());
        let result = boundary.try_render(|| {
            Ok(vec![Widget::Label { id: "ok".into(), text: "Recovered".into() }])
        });
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn hidden_fallback() {
        let mut boundary = ErrorBoundaryDef::new("test")
            .with_fallback(ErrorFallback::Hidden);
        let result = boundary.try_render(|| Err(RenderError::new("c", "fail")));
        assert!(result.is_empty());
    }

    #[test]
    fn custom_fallback() {
        let custom = vec![Widget::Label { id: "fb".into(), text: "Custom error".into() }];
        let mut boundary = ErrorBoundaryDef::new("test")
            .with_fallback(ErrorFallback::Custom(custom));
        let result = boundary.try_render(|| Err(RenderError::new("c", "fail")));
        assert_eq!(result.len(), 1);
        if let Widget::Label { text, .. } = &result[0] {
            assert_eq!(text, "Custom error");
        }
    }

    #[test]
    fn auto_retry_strategy() {
        let mut boundary = ErrorBoundaryDef::new("test")
            .with_recovery(RecoveryStrategy::AutoRetry { max_retries: 3, base_delay_ms: 100 });
        let _ = boundary.try_render(|| {
            Err(RenderError::new("c", "transient error"))
        });
        assert!(boundary.should_retry());
        assert_eq!(boundary.retry_delay_ms(), 100);
    }

    #[test]
    fn error_callback() {
        use std::sync::{Arc, Mutex};
        let captured = Arc::new(Mutex::new(Vec::new()));
        let cap2 = captured.clone();
        let mut boundary = ErrorBoundaryDef::new("test")
            .with_on_error(move |err| { cap2.lock().unwrap().push(err.message.clone()); });
        let _ = boundary.try_render(|| Err(RenderError::new("c", "boom")));
        assert_eq!(captured.lock().unwrap().len(), 1);
        assert_eq!(captured.lock().unwrap()[0], "boom");
    }

    #[test]
    fn manager_tracks_boundaries() {
        let mut mgr = ErrorBoundaryManager::new();
        mgr.add(ErrorBoundaryDef::new("b1"));
        mgr.add(ErrorBoundaryDef::new("b2"));
        assert_eq!(mgr.boundary_count(), 2);
        
        mgr.get_mut("b1").unwrap().catch_error(RenderError::new("c", "err1"));
        assert_eq!(mgr.errored_boundaries().len(), 1);
        assert_eq!(mgr.total_errors(), 1);
        
        mgr.reset_all();
        assert_eq!(mgr.errored_boundaries().len(), 0);
    }

    #[test]
    fn placeholder_fallback() {
        let mut boundary = ErrorBoundaryDef::new("test")
            .with_fallback(ErrorFallback::Placeholder("Loading failed".into()));
        let result = boundary.try_render(|| Err(RenderError::new("c", "fail")));
        assert_eq!(result.len(), 1);
        if let Widget::Label { text, .. } = &result[0] {
            assert_eq!(text, "Loading failed");
        }
    }

    #[test]
    fn error_with_stack() {
        let err = RenderError::new("comp", "null ref")
            .with_stack(vec!["render()".into(), "App.mount()".into()]);
        assert_eq!(err.stack.len(), 2);
        assert_eq!(err.component_id, "comp");
    }
}
