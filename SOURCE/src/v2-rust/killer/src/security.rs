/// Killer Security Module - Input validation, path safety, recursion limits
/// Features:
/// - Path traversal prevention
/// - Recursion depth limiting
/// - Input size validation
/// - Resource quotas
/// - Safe file operations

use std::cell::RefCell;
use std::path::{Path, PathBuf};
use crate::error::VmError;

/// Maximum recursion depth (prevents stack overflow)
pub const MAX_RECURSION_DEPTH: usize = 10_000;

/// Maximum file size to load (64 MB limit)
pub const MAX_FILE_SIZE: u64 = 64 * 1024 * 1024;

/// Maximum parser input size (prevent DOS)
pub const MAX_PARSER_INPUT_SIZE: usize = 100 * 1024 * 1024;

/// Maximum nesting depth in parser (prevent stack exhaustion)
pub const MAX_NESTING_DEPTH: usize = 500;

/// Maximum call stack depth (prevent infinite recursion)
pub const MAX_CALL_STACK_DEPTH: usize = 10_000;

/// Security configuration for the runtime
#[derive(Debug, Clone)]
pub struct SecurityConfig {
    /// Hard wall-clock limit for hosted script runs (ms). `0` = no limit from this field.
    /// Prefer wiring [`ExecutionBudget::max_wall_ms`] into the VM for enforcement.
    pub max_execution_ms: u64,
    /// Allow reading arbitrary files
    pub allow_unrestricted_file_access: bool,
    /// Allowed directory prefixes (whitelist)
    pub allowed_directories: Vec<PathBuf>,
    /// Maximum recursion depth
    pub max_recursion_depth: usize,
    /// Maximum file size
    pub max_file_size: u64,
    /// Enable path canonicalization checking
    pub enforce_path_canonicalization: bool,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        SecurityConfig {
            max_execution_ms: 0,
            allow_unrestricted_file_access: false,
            allowed_directories: vec![
                PathBuf::from("."),
                PathBuf::from("./examples"),
                PathBuf::from("./src"),
            ],
            max_recursion_depth: MAX_RECURSION_DEPTH,
            max_file_size: MAX_FILE_SIZE,
            enforce_path_canonicalization: true,
        }
    }
}

/// Validates a file path against security policy
/// 
/// Returns Ok(normalized_path) if valid, Err if traversal attempt detected
pub fn validate_file_path(
    path: &str,
    config: &SecurityConfig,
) -> Result<PathBuf, VmError> {
    // Convert to PathBuf
    let path_buf = PathBuf::from(path);

    // Reject absolute paths that leave allowed directories
    if path_buf.is_absolute() {
        return Err(VmError::SecurityError {
            message: "Absolute paths not allowed".to_string(),
            location: None,
            suggestion: Some("Use relative paths within the project directory".to_string()),
        });
    }

    // Reject paths with `..` (parent directory traversal)
    if path_buf.components().any(|c| {
        if let std::path::Component::ParentDir = c {
            true
        } else {
            false
        }
    }) {
        return Err(VmError::SecurityError {
            message: format!("Path traversal attempt detected: {}", path),
            location: None,
            suggestion: Some("Use relative paths without '..'.".to_string()),
        });
    }

    // If canonicalization is enabled, verify the result is in allowed directory
    if config.enforce_path_canonicalization {
        let canonical = std::fs::canonicalize(&path_buf).map_err(|e| {
            VmError::SecurityError {
                message: format!("Could not canonicalize path '{}': {}", path, e),
                location: None,
                suggestion: None,
            }
        })?;

        // Check if canonical path is within allowed directories
        let mut in_allowed_dir = false;
        for allowed_dir in &config.allowed_directories {
            let allowed_canonical = std::fs::canonicalize(allowed_dir)
                .unwrap_or_else(|_| allowed_dir.clone());
            
            if canonical.starts_with(&allowed_canonical) {
                in_allowed_dir = true;
                break;
            }
        }

        if !in_allowed_dir && !config.allow_unrestricted_file_access {
            return Err(VmError::SecurityError {
                message: format!("Access denied: {} is outside allowed directories", path),
                location: None,
                suggestion: Some("Configure allowed_directories in security config".to_string()),
            });
        }
    }

    Ok(path_buf)
}

/// Check file size safety before reading
pub fn check_file_size(path: &Path, config: &SecurityConfig) -> Result<(), VmError> {
    match std::fs::metadata(path) {
        Ok(metadata) => {
            if metadata.len() > config.max_file_size {
                return Err(VmError::SecurityError {
                    message: format!(
                        "File too large: {} bytes (max: {} bytes)",
                        metadata.len(),
                        config.max_file_size
                    ),
                    location: None,
                    suggestion: Some("Split large files or increase MAX_FILE_SIZE".to_string()),
                });
            }
            Ok(())
        }
        Err(e) => Err(VmError::IoError {
            message: format!(
                "Could not read file metadata '{}': {}",
                path.display(),
                e
            ),
            location: None,
        }),
    }
}

/// Safe file read with security checks
pub fn read_file_safe(path: &str, config: &SecurityConfig) -> Result<String, VmError> {
    // Validate path
    let path_buf = validate_file_path(path, config)?;

    // Check file size
    check_file_size(&path_buf, config)?;

    // Read file and strip UTF-8 BOM if present
    let content = std::fs::read_to_string(&path_buf).map_err(|e| {
        VmError::IoError {
            message: format!("Could not read '{}': {}", path, e),
            location: None,
        }
    })?;
    Ok(content.strip_prefix('\u{FEFF}').unwrap_or(&content).to_string())
}

/// Recursion depth validator
///
/// Uses `Cell<usize>` for interior mutability so multiple `RecursionGuardToken`s can
/// coexist without requiring `&mut self` on `enter()`.
pub struct RecursionGuard {
    current_depth: std::cell::Cell<usize>,
    max_depth: usize,
}

impl RecursionGuard {
    pub fn new(max_depth: usize) -> Self {
        RecursionGuard {
            current_depth: std::cell::Cell::new(0),
            max_depth,
        }
    }

    /// Enter a recursive call — returns `Err` if max depth is reached.
    /// Token is RAII: dropping it decrements the depth counter automatically.
    pub fn enter(&self) -> Result<RecursionGuardToken<'_>, VmError> {
        let depth = self.current_depth.get();
        if depth >= self.max_depth {
            return Err(VmError::SecurityError {
                message: format!(
                    "Recursion limit exceeded: {} calls (max: {})",
                    depth, self.max_depth
                ),
                location: None,
                suggestion: Some("Increase recursion limit or refactor to use iteration".to_string()),
            });
        }
        self.current_depth.set(depth + 1);
        Ok(RecursionGuardToken { parent: self })
    }

    pub fn current(&self) -> usize {
        self.current_depth.get()
    }

    pub fn max(&self) -> usize {
        self.max_depth
    }
}

/// RAII guard for recursion depth tracking
pub struct RecursionGuardToken<'a> {
    parent: &'a RecursionGuard,
}

impl<'a> Drop for RecursionGuardToken<'a> {
    fn drop(&mut self) {
        let d = self.parent.current_depth.get();
        if d > 0 {
            self.parent.current_depth.set(d - 1);
        }
    }
}

// ── Execution budget & capabilities (wire into VM / hosted runtimes) ────────

/// Optional **instruction-step budget** for `VirtualMachine::run` (not yet enforced by default).
/// Hosts can decrement per opcode and abort with [`VmError::SecurityError`].
#[derive(Debug, Clone)]
pub struct ExecutionBudget {
    /// Max bytecode steps per `run` (0 = unlimited).
    pub max_instruction_steps: u64,
    /// Wall-clock limit in milliseconds (0 = unlimited).
    pub max_wall_ms: u64,
    /// Soft heap ceiling in bytes for hosted scripts (0 = track only / no cap).
    pub max_heap_bytes: u64,
}

impl Default for ExecutionBudget {
    fn default() -> Self {
        ExecutionBudget {
            max_instruction_steps: 50_000_000,
            max_wall_ms: 120_000,
            max_heap_bytes: 256 * 1024 * 1024,
        }
    }
}

/// **Capability** flags for sandboxed `.killer` execution (wired via thread-local during `VM::run`).
#[derive(Debug, Clone)]
pub struct CapabilitySet {
    pub allow_file_read: bool,
    pub allow_file_write: bool,
    pub allow_network: bool,
    pub allow_process_spawn: bool,
    pub allow_native_jit: bool,
    /// LLM / KhLM / vision / tool-calling and other model-backed builtins.
    pub allow_llm: bool,
}

impl Default for CapabilitySet {
    fn default() -> Self {
        Self::restricted()
    }
}

impl CapabilitySet {
    /// Restricted sandbox — builtins that touch I/O should check this.
    pub fn restricted() -> Self {
        Self {
            allow_file_read: false,
            allow_file_write: false,
            allow_network: false,
            allow_process_spawn: false,
            allow_native_jit: false,
            allow_llm: false,
        }
    }

    /// Local dev default (current behaviour): everything on.
    pub fn trusted_local() -> Self {
        Self {
            allow_file_read: true,
            allow_file_write: true,
            allow_network: true,
            allow_process_spawn: true,
            allow_native_jit: true,
            allow_llm: true,
        }
    }
}

thread_local! {
    static THREAD_CAPABILITIES: RefCell<CapabilitySet> =
        RefCell::new(CapabilitySet::trusted_local());
}

/// Current effective capabilities for this OS thread (defaults to [`CapabilitySet::trusted_local`]).
pub fn current_capabilities() -> CapabilitySet {
    THREAD_CAPABILITIES.with(|c| c.borrow().clone())
}

/// RAII: install capabilities for the current thread; previous set is restored on drop (supports nesting).
pub struct CapabilityScopeGuard {
    previous: CapabilitySet,
}

impl CapabilityScopeGuard {
    pub fn install(caps: CapabilitySet) -> Self {
        let previous = THREAD_CAPABILITIES.with(|cell| {
            let mut b = cell.borrow_mut();
            std::mem::replace(&mut *b, caps)
        });
        Self { previous }
    }
}

impl Drop for CapabilityScopeGuard {
    fn drop(&mut self) {
        let prev = self.previous.clone();
        THREAD_CAPABILITIES.with(|cell| {
            *cell.borrow_mut() = prev;
        });
    }
}

fn capability_denied(what: &str) -> VmError {
    VmError::SecurityError {
        message: format!(
            "Capability denied: {} is disabled in this execution context",
            what
        ),
        location: None,
        suggestion: Some(
            "Enable the corresponding flag on CapabilitySet (e.g. trusted_local()) or configure the host runtime."
                .to_string(),
        ),
    }
}

pub fn require_file_read() -> Result<(), VmError> {
    if !current_capabilities().allow_file_read {
        return Err(capability_denied("file read"));
    }
    Ok(())
}

pub fn require_file_write() -> Result<(), VmError> {
    if !current_capabilities().allow_file_write {
        return Err(capability_denied("file write"));
    }
    Ok(())
}

pub fn require_network() -> Result<(), VmError> {
    if !current_capabilities().allow_network {
        return Err(capability_denied("network"));
    }
    Ok(())
}

pub fn require_process_spawn() -> Result<(), VmError> {
    if !current_capabilities().allow_process_spawn {
        return Err(capability_denied("process / thread spawn"));
    }
    Ok(())
}

pub fn require_llm() -> Result<(), VmError> {
    if !current_capabilities().allow_llm {
        return Err(capability_denied("LLM / model-backed builtins"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_path_traversal_detection() {
        let config = SecurityConfig::default();
        assert!(validate_file_path("../../../etc/passwd", &config).is_err());
        assert!(validate_file_path("src/main.rs", &config).is_ok());
    }

    #[test]
    fn test_recursion_guard() {
        let guard = RecursionGuard::new(3);
        let _t1 = guard.enter().expect("depth 1 ok");
        let _t2 = guard.enter().expect("depth 2 ok");
        let _t3 = guard.enter().expect("depth 3 ok");
        assert!(guard.enter().is_err()); // depth=3 >= max=3 → should fail
    }

    #[test]
    fn test_recursion_guard_cleanup() {
        let guard = RecursionGuard::new(10);
        {
            let _token = guard.enter().unwrap();
            // Cannot borrow guard immutably while _token holds mutable borrow;
            // verify depth after the token is dropped instead.
        } // _token dropped here — depth decrements
        assert_eq!(guard.current(), 0); // Depth should be back to 0
    }

    #[test]
    fn capability_scope_denies_file_read_when_restricted() {
        let _outer = CapabilityScopeGuard::install(CapabilitySet::trusted_local());
        {
            let _inner = CapabilityScopeGuard::install(CapabilitySet::restricted());
            assert!(require_file_read().is_err());
        }
        assert!(require_file_read().is_ok());
    }

    #[test]
    fn capability_scope_restores_after_drop() {
        let _g = CapabilityScopeGuard::install(CapabilitySet::restricted());
        drop(_g);
        assert!(require_file_read().is_ok());
    }
}
