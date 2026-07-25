use crate::value::Value;
use crate::error::VmError;
use std::collections::VecDeque;

/// Represents a single try/catch/finally block frame
#[derive(Clone, Debug)]
pub struct TryFrame {
    pub catch_target: usize,
    pub finally_target: usize,
}

/// Manages exception handling and try/catch/finally block stack
pub struct ExceptionManager {
    try_stack: VecDeque<TryFrame>,
    current_error: Option<Value>,
}

impl Default for ExceptionManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ExceptionManager {
    /// Create a new exception manager
    pub fn new() -> Self {
        Self {
            try_stack: VecDeque::new(),
            current_error: None,
        }
    }

    /// Enter a try block and push frame onto stack
    pub fn push_try_frame(&mut self, catch_target: usize, finally_target: usize) {
        self.try_stack.push_back(TryFrame {
            catch_target,
            finally_target,
        });
    }

    /// Exit the current try block (successful path)
    pub fn pop_try_frame(&mut self) -> Result<(), VmError> {
        if !self.try_stack.is_empty() {
            self.try_stack.pop_front();
            self.current_error = None;
            Ok(())
        } else {
            Err(VmError::runtime_error(
                "Attempted to exit try block when none active".to_string(),
            ))
        }
    }

    /// Throw an exception
    pub fn throw(&mut self, value: Value) -> Result<Option<usize>, VmError> {
        self.current_error = Some(value.clone());

        if let Some(try_frame) = self.try_stack.back() {
            // Check for catch block
            if try_frame.catch_target != usize::MAX {
                return Ok(Some(try_frame.catch_target));
            }
            // Fall back to finally block
            if try_frame.finally_target != usize::MAX {
                return Ok(Some(try_frame.finally_target));
            }
        }

        // No handler available
        Err(VmError::runtime_error(format!(
            "Uncaught exception: {}",
            value
        )))
    }

    /// Get the current error value
    pub fn current_error(&self) -> Option<&Value> {
        self.current_error.as_ref()
    }

    /// Take the current error value (consumes it)
    pub fn take_error(&mut self) -> Option<Value> {
        self.current_error.take()
    }

    /// Clear the current error
    pub fn clear_error(&mut self) {
        self.current_error = None;
    }

    /// Check if there's an active try block
    pub fn has_active_try(&self) -> bool {
        !self.try_stack.is_empty()
    }

    /// Get the number of active try blocks
    pub fn depth(&self) -> usize {
        self.try_stack.len()
    }

    /// Reset all state
    pub fn reset(&mut self) {
        self.try_stack.clear();
        self.current_error = None;
    }

    /// Handle arithmetic errors (division by zero, modulo by zero, etc.)
    /// Returns Ok(Some(target)) if there's a catch/finally to jump to
    /// Returns Ok(None) if the error should be propagated as fatal
    /// Returns Err if the error cannot be handled
    pub fn handle_arithmetic_error(&mut self, error_msg: &str) -> Result<Option<usize>, VmError> {
        if let Some(try_frame) = self.try_stack.back() {
            self.current_error = Some(Value::Str(error_msg.to_string()));
            
            if try_frame.catch_target != usize::MAX {
                return Ok(Some(try_frame.catch_target));
            }
            if try_frame.finally_target != usize::MAX {
                return Ok(Some(try_frame.finally_target));
            }
        }
        
        Err(VmError::runtime_error(error_msg.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_push_and_pop_try_frames() {
        let mut em = ExceptionManager::new();
        assert_eq!(em.depth(), 0);

        em.push_try_frame(10, 20);
        assert_eq!(em.depth(), 1);
        assert!(em.has_active_try());

        let result = em.pop_try_frame();
        assert!(result.is_ok());
        assert_eq!(em.depth(), 0);
        assert!(!em.has_active_try());
    }

    #[test]
    fn test_throw_with_catch_target() {
        let mut em = ExceptionManager::new();
        em.push_try_frame(10, usize::MAX);

        let target = em.throw(Value::Str("test error".to_string()));
        assert!(target.is_ok());
        assert_eq!(target.unwrap(), Some(10));
    }

    #[test]
    fn test_throw_with_finally_target() {
        let mut em = ExceptionManager::new();
        em.push_try_frame(usize::MAX, 20);

        let target = em.throw(Value::Str("test error".to_string()));
        assert!(target.is_ok());
        assert_eq!(target.unwrap(), Some(20));
    }

    #[test]
    fn test_throw_uncaught() {
        let mut em = ExceptionManager::new();
        let result = em.throw(Value::Str("uncaught".to_string()));
        assert!(result.is_err());
    }

    #[test]
    fn test_arithmetic_error_handling() {
        let mut em = ExceptionManager::new();
        em.push_try_frame(15, usize::MAX);

        let result = em.handle_arithmetic_error("Division by zero");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Some(15));
        assert!(em.current_error().is_some());
    }

    #[test]
    fn test_error_value_lifecycle() {
        let mut em = ExceptionManager::new();
        assert!(em.current_error().is_none());

        em.throw(Value::Str("test".to_string())).ok();
        assert!(em.current_error().is_some());

        let error = em.take_error();
        assert!(error.is_some());
        assert!(em.current_error().is_none());
    }
}
