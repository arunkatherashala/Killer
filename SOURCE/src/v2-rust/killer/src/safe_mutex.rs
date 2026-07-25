/// Safe Mutex Wrapper - v4.3 Enhancement
/// Prevents panics from mutex poison by providing Result-based API
/// Purpose: Replace all .lock().unwrap() calls with safe alternatives

use std::sync::{Arc, Mutex, MutexGuard};
use std::fmt;

/// Custom error type for safe mutex operations
#[derive(Debug, Clone)]
pub enum MutexError {
    /// Mutex was poisoned by another thread
    Poisoned,
    /// Timeout waiting for lock
    Timeout,
    /// Other error
    Other(String),
}

impl fmt::Display for MutexError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MutexError::Poisoned => write!(f, "Mutex poisoned by panicking thread"),
            MutexError::Timeout => write!(f, "Mutex lock timeout"),
            MutexError::Other(msg) => write!(f, "Mutex error: {}", msg),
        }
    }
}

impl std::error::Error for MutexError {}

/// Result type for mutex operations
pub type MutexResult<T> = Result<T, MutexError>;

/// Safe wrapper around Mutex<T>
pub struct SafeMutex<T> {
    inner: Arc<Mutex<T>>,
}

impl<T> Clone for SafeMutex<T> {
    fn clone(&self) -> Self {
        SafeMutex {
            inner: Arc::clone(&self.inner),
        }
    }
}

impl<T> SafeMutex<T> {
    /// Create a new safe mutex
    pub fn new(value: T) -> Self {
        SafeMutex {
            inner: Arc::new(Mutex::new(value)),
        }
    }

    /// Try to lock the mutex, recovering from poison if possible
    pub fn lock_safe(&self) -> MutexResult<MutexGuard<'_, T>> {
        match self.inner.lock() {
            Ok(guard) => Ok(guard),
            Err(poison) => {
                // Recover from poison and return the guard anyway
                Ok(poison.into_inner())
            }
        }
    }

    /// Try to lock and apply a function, handling poison
    pub fn with_lock<F, R>(&self, f: F) -> MutexResult<R>
    where
        F: FnOnce(&mut T) -> R,
    {
        match self.lock_safe() {
            Ok(mut guard) => Ok(f(&mut guard)),
            Err(e) => Err(e),
        }
    }

    /// Try to read a value
    pub fn read(&self) -> MutexResult<T>
    where
        T: Clone,
    {
        self.with_lock(|value| value.clone())
    }

    /// Try to write a value
    pub fn write(&self, value: T) -> MutexResult<()> {
        self.with_lock(|target| *target = value)
    }
}

/// Trait for types that can be safely accessed in a mutex
pub trait SafeMutexOp: Sized {
    /// Get the inner value from a safe mutex
    fn from_safe_mutex(mutex: &SafeMutex<Self>) -> MutexResult<Self>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn safe_mutex_basic() {
        let mutex = SafeMutex::new(42);
        let result = mutex.read();
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    fn safe_mutex_write() {
        let mutex = SafeMutex::new(10);
        let _ = mutex.write(20);
        let result = mutex.read();
        assert_eq!(result.unwrap(), 20);
    }

    #[test]
    fn safe_mutex_with_lock() {
        let mutex = SafeMutex::new(vec![1, 2, 3]);
        let result = mutex.with_lock(|v| v.len());
        assert_eq!(result.unwrap(), 3);
    }
}
