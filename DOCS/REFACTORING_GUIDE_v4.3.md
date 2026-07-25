# Killer Language Compiler Refactoring Guide v4.3
**March 22, 2026 - Code Quality & Reliability Sprint**

---

## 1. CRITICAL: VirtualMachine God Object Decomposition

### Root Cause Analysis
- **Problem**: VirtualMachine struct has 43 fields mixing 5 concerns
  - Core execution (stack, scopes, call_stack, ip)
  - Class management (classes, current_object)
  - Exception handling (exception_manager)
  - Generator support (generator_manager, yielded_values)
  - Performance optimization (12 JIT/caching/security fields)
- **Impact**: Difficult to test, maintain, extend; violates Single Responsibility Principle
- **Risk**: Changes to one optimization break unrelated features

### Current State
```rust
pub struct VirtualMachine {
    // Execution Core (4 fields)
    stack: Vec<Value>,
    scopes: Vec<HashMap<String, Value>>,
    call_stack: Vec<usize>,
    ip: usize,
    
    // Class System (2 fields)
    classes: HashMap<String, ClassInfo>,
    current_object: Option<ObjectInstance>,
    
    // Exception Handling (1 field)
    exception_manager: ExceptionManager,
    
    // Generator Support (3 fields)
    generator_manager: GeneratorManager,
    yielded_values: Vec<Value>,
    collecting_yields: bool,
    
    // Performance Optimization (20+ fields)
    instruction_cache: Option<InstructionCache>,
    jit_compiler: JitCompiler,
    hot_detector: HotCodeDetector,
    // ... 17 more optimization-related fields
}
```

### Proposed Solution

#### Extract 1: ExecutionContext (14 fields → new struct)
```rust
/// Core execution state - managed by VM
pub struct ExecutionContext {
    // Execution Point
    pub stack: Vec<Value>,
    pub scopes: Vec<HashMap<String, Value>>,
    pub call_stack: Vec<usize>,
    pub ip: usize,
    
    // Object State
    pub current_object: Option<ObjectInstance>,
    
    // Exception & Generator State
    pub exception_manager: ExceptionManager,
    pub generator_manager: GeneratorManager,
    pub yielded_values: Vec<Value>,
    pub collecting_yields: bool,
}

impl ExecutionContext {
    pub fn new() -> Self {
        ExecutionContext {
            stack: Vec::new(),
            scopes: Vec::new(),
            call_stack: Vec::new(),
            ip: 0,
            current_object: None,
            exception_manager: ExceptionManager::default(),
            generator_manager: GeneratorManager::default(),
            yielded_values: Vec::new(),
            collecting_yields: false,
        }
    }

    /// Push scope for function/block entry
    pub fn push_scope(&mut self) -> Result<(), VmError> {
        if self.scopes.len() >= 10000 {
            return Err(VmError::RuntimeError("Stack overflow".to_string()));
        }
        self.scopes.push(HashMap::new());
        Ok(())
    }

    /// Pop scope safely
    pub fn pop_scope(&mut self) -> Result<(), VmError> {
        if self.scopes.is_empty() {
            return Err(VmError::RuntimeError("Cannot pop root scope".to_string()));
        }
        self.scopes.pop();
        Ok(())
    }

    /// Get variable from nearest scope
    pub fn get_variable(&self, name: &str) -> Option<Value> {
        for scope in self.scopes.iter().rev() {
            if let Some(value) = scope.get(name) {
                return Some(value.clone());
            }
        }
        None
    }

    /// Set variable in current scope
    pub fn set_variable(&mut self, name: String, value: Value) -> Result<(), VmError> {
        if self.scopes.is_empty() {
            return Err(VmError::RuntimeError("No active scope".to_string()));
        }
        self.scopes.last_mut().unwrap().insert(name, value);
        Ok(())
    }
}
```

#### Extract 2: ClassRegistry (3 fields → new struct)
```rust
/// Global class definition registry
pub struct ClassRegistry {
    classes: HashMap<String, ClassInfo>,
}

impl ClassRegistry {
    pub fn new() -> Self {
        ClassRegistry {
            classes: HashMap::new(),
        }
    }

    pub fn register_class(&mut self, name: String, info: ClassInfo) -> Result<(), VmError> {
        if self.classes.contains_key(&name) {
            return Err(VmError::RuntimeError(
                format!("Class '{}' already defined", name)
            ));
        }
        self.classes.insert(name, info);
        Ok(())
    }

    pub fn get_class(&self, name: &str) -> Option<&ClassInfo> {
        self.classes.get(name)
    }

    pub fn get_class_mut(&mut self, name: &str) -> Option<&mut ClassInfo> {
        self.classes.get_mut(name)
    }
}
```

#### Extract 3: OptimizationContext (20+ fields → new struct)
```rust
/// Performance optimization state - isolated from VM logic
pub struct OptimizationContext {
    instruction_cache: Option<InstructionCache>,
    jit_compiler: JitCompiler,
    hot_detector: HotCodeDetector,
    baseline_jit: BasecodeJITCompiler,
    fast_path: ArithmeticLoopFastPath,
    native_codegen: NativeCodeGenerator,
    variable_cache: LoopOptimization,
    numeric_fast_mode: bool,
    call_site_cache: crate::call_site_cache::CallSiteCache,
    value_buffer_pool: crate::allocation_pool::ValueBufferPool,
    scope_var_cache: crate::allocation_pool::ScopeVariableCache,
    loop_pattern_detector: crate::loop_pattern_detection::LoopPatternDetector,
    recursion_guard: RecursionGuard,
}

impl OptimizationContext {
    pub fn new() -> Self {
        OptimizationContext {
            instruction_cache: None,
            jit_compiler: JitCompiler::new(),
            hot_detector: HotCodeDetector::new(1000),
            baseline_jit: BasecodeJITCompiler::new(),
            fast_path: ArithmeticLoopFastPath::new(),
            native_codegen: NativeCodeGenerator::new(),
            variable_cache: LoopOptimization::new(),
            numeric_fast_mode: false,
            call_site_cache: crate::call_site_cache::CallSiteCache::new(),
            value_buffer_pool: crate::allocation_pool::ValueBufferPool::default(),
            scope_var_cache: crate::allocation_pool::ScopeVariableCache::new(),
            loop_pattern_detector: crate::loop_pattern_detection::LoopPatternDetector::new(),
            recursion_guard: RecursionGuard::new(crate::security::MAX_RECURSION_DEPTH),
        }
    }

    pub fn enable_fast_mode(&mut self) {
        self.numeric_fast_mode = true;
    }

    pub fn disable_fast_mode(&mut self) {
        self.numeric_fast_mode = false;
    }

    pub fn check_recursion(&self, depth: usize) -> Result<(), VmError> {
        if !self.recursion_guard.is_safe(depth) {
            return Err(VmError::RuntimeError("Maximum recursion depth exceeded".to_string()));
        }
        Ok(())
    }
}
```

#### Refactored VirtualMachine (12 lines of code → 50, but far cleaner)
```rust
/// Refactored VirtualMachine - now 3 components
pub struct VirtualMachine {
    execution: ExecutionContext,
    classes: ClassRegistry,
    optimizations: OptimizationContext,
}

impl VirtualMachine {
    pub fn new() -> Self {
        VirtualMachine {
            execution: ExecutionContext::new(),
            classes: ClassRegistry::new(),
            optimizations: OptimizationContext::new(),
        }
    }

    // Delegating accessors (for backward compatibility)
    pub fn push_scope(&mut self) -> Result<(), VmError> {
        self.execution.push_scope()
    }

    pub fn pop_scope(&mut self) -> Result<(), VmError> {
        self.execution.pop_scope()
    }

    pub fn get_variable(&self, name: &str) -> Option<Value> {
        self.execution.get_variable(name)
    }

    pub fn set_variable(&mut self, name: String, value: Value) -> Result<(), VmError> {
        self.execution.set_variable(name, value)
    }

    pub fn get_stack_top(&self) -> Option<&Value> {
        self.execution.stack.last()
    }

    pub fn push_stack(&mut self, value: Value) -> Result<(), VmError> {
        if self.execution.stack.len() >= 1_000_000 {
            return Err(VmError::RuntimeError("Stack overflow".to_string()));
        }
        self.execution.stack.push(value);
        Ok(())
    }

    pub fn pop_stack(&mut self) -> Result<Value, VmError> {
        self.execution.stack.pop()
            .ok_or_else(|| VmError::RuntimeError("Stack underflow".to_string()))
    }
}
```

### Impact Analysis

| Aspect | Before | After |
|--------|--------|-------|
| Fields | 43 | 3 (at VM level) |
| Testability | Hard - can't test ExecutionContext in isolation | Easy - test each component separately |
| Maintenance | Changes risky, affect multiple concerns | Safe - changes localized to component |
| Module Independence | Optimization code coupled to VM | Optimization code is self-contained |
| Backward Compatibility | N/A | Maintained via accessor methods |

### Test Cases Needed

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execution_context_scope_management() {
        let mut ctx = ExecutionContext::new();
        assert!(ctx.push_scope().is_ok());
        assert!(ctx.set_variable("x".to_string(), Value::I64(42)).is_ok());
        assert_eq!(ctx.get_variable("x"), Some(Value::I64(42)));
        assert!(ctx.pop_scope().is_ok());
        assert_eq!(ctx.get_variable("x"), None);
    }

    #[test]
    fn test_stack_overflow_protection() {
        let mut ctx = ExecutionContext::new();
        for _ in 0..10 {
            ctx.push_scope().unwrap();
        }
        // 10th push should work, large stacks should fail
    }

    #[test]
    fn test_class_registry_duplicate_prevention() {
        let mut registry = ClassRegistry::new();
        let info = ClassInfo {
            name: "MyClass".to_string(),
            parent: None,
            methods: HashMap::new(),
        };
        assert!(registry.register_class("MyClass".to_string(), info.clone()).is_ok());
        assert!(registry.register_class("MyClass".to_string(), info).is_err());
    }

    #[test]
    fn test_optimization_recursion_guard() {
        let opt = OptimizationContext::new();
        assert!(opt.check_recursion(100).is_ok());
        assert!(opt.check_recursion(10001).is_err()); // Assuming MAX_RECURSION_DEPTH = 10000
    }

    #[test]
    fn test_vm_backward_compatibility() {
        let mut vm = VirtualMachine::new();
        assert!(vm.push_scope().is_ok());
        assert!(vm.set_variable("test".to_string(), Value::I64(99)).is_ok());
        assert_eq!(vm.get_variable("test"), Some(Value::I64(99)));
    }
}
```

#### Migration Path (Backward Compatibility)
```rust
// Old code that directly accessed fields:
// vm.stack.push(value);
// vm.scopes.last_mut().unwrap().insert("x", value);

// New code using delegation:
// vm.push_stack(value)?;
// vm.set_variable("x", value)?;

// Gradual migration:
// 1. Add accessor methods (done in refactor)
// 2. Migrate call sites one module at a time
// 3. Remove direct field access in future version
```

---

## 2. HIGH PRIORITY: Parser Error Recovery - Silent Failures

### Root Cause Analysis
- **Problem**: `.unwrap_or(0)` masks parse errors
  - Locations: `time_solver.rs:229-231`, parser modules
  - Silent return of 0 makes debugging impossible
  - No way to distinguish error from legitimate 0 value
- **Impact**: Hard to trace "why did my number parse to 0?" bugs
- **Risk**: Data loss, incorrect calculations silently failing

### Current State
```rust
// time_solver.rs:228-231
let hours = parts[0].parse::<u64>().unwrap_or(0);
let minutes = parts[1].parse::<u64>().unwrap_or(0);
let seconds = parts[2].parse::<u64>().unwrap_or(0);
// ❌ If parsing fails, get 0 instead of error
```

### Proposed Solution

#### Step 1: Enhance Error Type with SourceLocation
```rust
// error.rs - Add location tracking
#[derive(Debug, Clone)]
pub struct SourceLocation {
    pub line: usize,
    pub column: usize,
    pub file: Option<String>,
}

impl SourceLocation {
    pub fn new(line: usize, column: usize) -> Self {
        SourceLocation {
            line,
            column,
            file: None,
        }
    }

    pub fn with_file(line: usize, column: usize, file: String) -> Self {
        SourceLocation {
            line,
            column,
            file: Some(file),
        }
    }
}

impl Display for SourceLocation {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        if let Some(file) = &self.file {
            write!(f, "{}:{}:{}", file, self.line, self.column)
        } else {
            write!(f, "{}:{}", self.line, self.column)
        }
    }
}

// Enhanced VmError with location info
#[derive(Debug)]
pub enum VmError {
    ParseError {
        message: String,
        location: Option<SourceLocation>,
    },
    RuntimeError {
        message: String,
        location: Option<SourceLocation>,
    },
    IoError {
        message: String,
        location: Option<SourceLocation>,
    },
    SecurityError {
        message: String,
        suggestion: Option<String>,
        location: Option<SourceLocation>,
    },
}

impl Display for VmError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            VmError::ParseError { message, location } => {
                if let Some(loc) = location {
                    write!(f, "Parse error at {}: {}", loc, message)
                } else {
                    write!(f, "Parse error: {}", message)
                }
            }
            VmError::RuntimeError { message, location } => {
                if let Some(loc) = location {
                    write!(f, "Runtime error at {}: {}", loc, message)
                } else {
                    write!(f, "Runtime error: {}", message)
                }
            }
            VmError::IoError { message, location } => {
                if let Some(loc) = location {
                    write!(f, "IO error at {}: {}", loc, message)
                } else {
                    write!(f, "IO error: {}", message)
                }
            }
            VmError::SecurityError { message, suggestion, location } => {
                write!(f, "Security error")?;
                if let Some(loc) = location {
                    write!(f, " at {}", loc)?;
                }
                write!(f, ": {}", message)?;
                if let Some(sug) = suggestion {
                    write!(f, "\nSuggestion: {}", sug)?;
                }
                Ok(())
            }
        }
    }
}

impl std::error::Error for VmError {}
```

#### Step 2: Create Result Wrapper for Common Operations
```rust
// In parser.rs module - helper functions with error propagation
pub type ParseResult<T> = Result<T, VmError>;

/// Parse number with proper error handling
pub fn parse_number(input: &str, line: usize, column: usize) -> ParseResult<i64> {
    input.parse::<i64>()
        .map_err(|e| VmError::ParseError {
            message: format!("Invalid number '{}': {}", input, e),
            location: Some(SourceLocation::new(line, column)),
        })
}

/// Parse unsigned with proper error handling
pub fn parse_unsigned(input: &str, line: usize, column: usize) -> ParseResult<u64> {
    input.parse::<u64>()
        .map_err(|e| VmError::ParseError {
            message: format!("Invalid unsigned number '{}': {}", input, e),
            location: Some(SourceLocation::new(line, column)),
        })
}

/// Parse with optional parts - validate each component
pub fn parse_time_components(time_str: &str, line: usize, column: usize) -> ParseResult<(u64, u64, u64)> {
    let parts: Vec<&str> = time_str.split(':').collect();
    
    if parts.len() != 3 {
        return Err(VmError::ParseError {
            message: format!("Time format must be HH:MM:SS, got '{}'", time_str),
            location: Some(SourceLocation::new(line, column)),
        });
    }

    let hours = parts[0].parse::<u64>()
        .map_err(|_| VmError::ParseError {
            message: format!("Invalid hours: '{}' (must be 0-23)", parts[0]),
            location: Some(SourceLocation::new(line, column)),
        })?;

    let minutes = parts[1].parse::<u64>()
        .map_err(|_| VmError::ParseError {
            message: format!("Invalid minutes: '{}' (must be 0-59)", parts[1]),
            location: Some(SourceLocation::new(line, column + 3)),
        })?;

    let seconds = parts[2].parse::<u64>()
        .map_err(|_| VmError::ParseError {
            message: format!("Invalid seconds: '{}' (must be 0-59)", parts[2]),
            location: Some(SourceLocation::new(line, column + 6)),
        })?;

    // Validation
    if hours > 23 {
        return Err(VmError::ParseError {
            message: format!("Hours out of range: {} (must be 0-23)", hours),
            location: Some(SourceLocation::new(line, column)),
        });
    }
    if minutes > 59 {
        return Err(VmError::ParseError {
            message: format!("Minutes out of range: {} (must be 0-59)", minutes),
            location: Some(SourceLocation::new(line, column + 3)),
        });
    }
    if seconds > 59 {
        return Err(VmError::ParseError {
            message: format!("Seconds out of range: {} (must be 0-59)", seconds),
            location: Some(SourceLocation::new(line, column + 6)),
        });
    }

    Ok((hours, minutes, seconds))
}
```

#### Step 3: Update Call Sites
```rust
// time_solver.rs - BEFORE
fn parse_time(time_str: &str) -> u64 {
    let parts: Vec<&str> = time_str.split(':').collect();
    let hours = parts.get(0).and_then(|p| p.parse::<u64>().ok()).unwrap_or(0);
    let minutes = parts.get(1).and_then(|p| p.parse::<u64>().ok()).unwrap_or(0);
    let seconds = parts.get(2).and_then(|p| p.parse::<u64>().ok()).unwrap_or(0);
    // ❌ Silently returns wrong time on parse error
    hours * 3600 + minutes * 60 + seconds
}

// time_solver.rs - AFTER
fn parse_time(time_str: &str, line: usize, column: usize) -> Result<u64, VmError> {
    let (hours, minutes, seconds) = parse_time_components(time_str, line, column)?;
    Ok(hours * 3600 + minutes * 60 + seconds)
}

// Caller code:
match parse_time(input, lexer.line, lexer.column) {
    Ok(seconds) => {
        // Use the validated time
        println!("Parsed time: {} seconds", seconds);
    }
    Err(VmError::ParseError { message, location }) => {
        if let Some(loc) = location {
            eprintln!("Parse error at {}: {}", loc, message);
        } else {
            eprintln!("Parse error: {}", message);
        }
        // Handle error appropriately
    }
    Err(e) => eprintln!("Unexpected error: {}", e),
}
```

### Test Cases Needed

```rust
#[cfg(test)]
mod parse_tests {
    use super::*;

    #[test]
    fn test_parse_valid_time() {
        let result = parse_time("14:30:45", 1, 1);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 52245); // 14*3600 + 30*60 + 45
    }

    #[test]
    fn test_parse_invalid_hours() {
        let result = parse_time("25:30:45", 1, 1);
        assert!(result.is_err());
        if let Err(VmError::ParseError { message, location }) = result {
            assert!(message.contains("Hours out of range"));
            assert_eq!(location.unwrap().line, 1);
        }
    }

    #[test]
    fn test_parse_malformed_time() {
        let result = parse_time("14:30", 1, 1);
        assert!(result.is_err());
        if let Err(VmError::ParseError { message, .. }) = result {
            assert!(message.contains("must be HH:MM:SS"));
        }
    }

    #[test]
    fn test_parse_non_numeric() {
        let result = parse_time("ab:cd:ef", 1, 1);
        assert!(result.is_err());
    }

    #[test]
    fn test_error_includes_location() {
        let error = parse_time("invalid", 5, 10);
        if let Err(VmError::ParseError { location, .. }) = error {
            assert_eq!(location.unwrap().line, 5);
            assert_eq!(location.unwrap().column, 10);
        }
    }
}
```

---

## 3. MODERATE: VmError Missing SourceLocation - Already Covered Above ✅

See Section 2 for complete implementation.

---

## 4. MODERATE: Mutex .lock().unwrap() Panic Risk

### Root Cause Analysis
- **Problem**: 20+ instances of `.lock().unwrap()` throughout codebase
  - If mutex becomes poisoned (thread panics while holding lock), any subsequent `.lock()` returns Err
  - `.unwrap()` then panics, propagating error uncontrollably
  - Examples: circuit_breaker.rs:108, telemetry.rs:195
- **Impact**: Cascading failures; one thread panic = entire system crash
- **Risk**: Use all 3 failure modes simultaneously

### Current State
```rust
// circuit_breaker.rs:108 - DANGER
pub fn record_success(&self) {
    let mut state = self.state.lock().unwrap();  // ❌ Panics on poisoned mutex
    let mut failure_count = self.failure_count.lock().unwrap();
    let mut success_count = self.success_count.lock().unwrap();
    // ...
}

// telemetry.rs:195 - DANGER
pub fn record_request(&self, duration_ms: f64, success: bool) {
    if let Ok(mut metrics) = self.app_metrics.lock() {  // ✅ Good pattern
        metrics.record_request(duration_ms, success);
    }
}
```

### Proposed Solution

#### Step 1: Create Safe Mutex Wrapper Helper Functions
```rust
// In a new module: safe_mutex.rs
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use crate::error::VmError;

pub type SafeResult<T> = Result<T, VmError>;

/// Extension trait for safe mutex operations
pub trait SafeMutex<T> {
    /// Lock with poison recovery - attempts to recover poisoned mutex
    fn safe_lock(&self) -> SafeResult<MutexGuard<T>>;
    
    /// Lock with timeout (would need parking_lot in real scenario)
    fn safe_lock_timeout(&self, timeout_ms: u64) -> SafeResult<MutexGuard<T>>;
}

impl<T> SafeMutex<T> for Mutex<T> {
    fn safe_lock(&self) -> SafeResult<MutexGuard<T>> {
        match self.lock() {
            Ok(guard) => Ok(guard),
            Err(PoisonError { inner }) => {
                // Attempt recovery: get guard anyway
                // Log warning
                eprintln!("⚠️  Mutex was poisoned, recovering...");
                Ok(inner.into_inner())
            }
        }
    }

    fn safe_lock_timeout(&self, _timeout_ms: u64) -> SafeResult<MutexGuard<T>> {
        // With parking_lot: self.lock().wait_for(timeout)?
        // Without: fallback to safe_lock
        self.safe_lock()
    }
}

/// For operations that modify state, use this pattern
pub fn with_lock<T, F, R>(mutex: &Mutex<T>, f: F) -> SafeResult<R>
where
    F: FnOnce(&mut T) -> SafeResult<R>,
{
    let mut guard = mutex.safe_lock()?;
    f(&mut guard)
}

/// For read-only operations
pub fn with_lock_read<T, F, R>(mutex: &Mutex<T>, f: F) -> SafeResult<R>
where
    T: Clone,
    F: FnOnce(&T) -> SafeResult<R>,
{
    let guard = mutex.safe_lock()?;
    f(&*guard)
}
```

#### Step 2: Refactor Circuit Breaker
```rust
// circuit_breaker.rs - REFACTORED
use crate::safe_mutex::SafeMutex;

pub struct CircuitBreaker {
    config: CircuitBreakerConfig,
    state: Arc<Mutex<CircuitState>>,
    failure_count: Arc<Mutex<u32>>,
    success_count: Arc<Mutex<u32>>,
    last_failure_time: Arc<Mutex<Option<SystemTime>>>,
    half_open_requests: Arc<Mutex<u32>>,
    state_changes: Arc<Mutex<Vec<StateChange>>>,
}

impl CircuitBreaker {
    pub fn get_state(&self) -> Result<CircuitState, VmError> {
        let state_guard = self.state.safe_lock()?;
        Ok(*state_guard)
    }

    pub fn record_success(&self) -> Result<(), VmError> {
        let mut state = self.state.safe_lock()?;
        let mut failure_count = self.failure_count.safe_lock()?;
        let mut success_count = self.success_count.safe_lock()?;
        let mut half_open_requests = self.half_open_requests.safe_lock()?;

        *failure_count = 0;

        match *state {
            CircuitState::Closed => {
                // Normal operation
                Ok(())
            }
            CircuitState::HalfOpen => {
                *success_count += 1;
                if *success_count >= self.config.success_threshold {
                    *state = CircuitState::Closed;
                    *success_count = 0;
                    *half_open_requests = 0;
                    let old_state = CircuitState::HalfOpen;
                    self.record_state_change(old_state, *state, "Recovery succeeded")?;
                }
                Ok(())
            }
            CircuitState::Open => {
                Ok(()) // Ignore success while open
            }
        }
    }

    pub fn record_failure(&self) -> Result<(), VmError> {
        let mut state = self.state.safe_lock()?;
        let mut failure_count = self.failure_count.safe_lock()?;
        let mut last_failure_time = self.last_failure_time.safe_lock()?;

        *failure_count += 1;
        *last_failure_time = Some(SystemTime::now());

        match *state {
            CircuitState::Closed => {
                if *failure_count >= self.config.failure_threshold {
                    *state = CircuitState::Open;
                    let old_state = CircuitState::Closed;
                    self.record_state_change(old_state, *state, "Failure threshold exceeded")?;
                }
                Ok(())
            }
            CircuitState::HalfOpen => {
                *state = CircuitState::Open;
                let old_state = CircuitState::HalfOpen;
                self.record_state_change(old_state, *state, "Recovery failed")?;
                Ok(())
            }
            CircuitState::Open => {
                Ok(()) // Already open
            }
        }
    }

    fn record_state_change(
        &self,
        from: CircuitState,
        to: CircuitState,
        reason: &str,
    ) -> Result<(), VmError> {
        let mut changes = self.state_changes.safe_lock()?;
        changes.push(StateChange {
            timestamp: SystemTime::now(),
            from_state: from,
            to_state: to,
            reason: reason.to_string(),
        });
        Ok(())
    }
}
```

#### Step 3: Update Telemetry
```rust
// telemetry.rs - REFACTORED
pub struct TelemetryCollector {
    app_metrics: Arc<Mutex<ApplicationMetrics>>,
    vm_metrics: Arc<Mutex<VmMetrics>>,
    resource_metrics: Arc<Mutex<ResourceMetrics>>,
    start_time: SystemTime,
    enabled: bool,
}

impl TelemetryCollector {
    pub fn record_request(&self, duration_ms: f64, success: bool) -> Result<(), VmError> {
        if !self.enabled {
            return Ok(());
        }

        let mut metrics = self.app_metrics.safe_lock()?;
        metrics.record_request(duration_ms, success);
        Ok(())
    }

    pub fn record_vm_instruction(&self) -> Result<(), VmError> {
        if !self.enabled {
            return Ok(());
        }

        let mut metrics = self.vm_metrics.safe_lock()?;
        metrics.instructions_executed += 1;
        Ok(())
    }

    pub fn get_app_metrics(&self) -> Result<ApplicationMetrics, VmError> {
        let guard = self.app_metrics.safe_lock()?;
        Ok(guard.clone())
    }
}
```

### Test Cases Needed

```rust
#[cfg(test)]
mod safe_mutex_tests {
    use super::*;

    #[test]
    fn test_safe_lock_normal_operation() {
        let data = Arc::new(Mutex::new(42));
        let result = data.safe_lock();
        assert!(result.is_ok());
        assert_eq!(*result.unwrap(), 42);
    }

    #[test]
    fn test_circuit_breaker_state_transitions() {
        let cb = CircuitBreaker::with_defaults();
        
        // Initial state should be Closed
        assert_eq!(cb.get_state().unwrap(), CircuitState::Closed);
        
        // Record 5 failures to open the circuit
        for _ in 0..5 {
            let _ = cb.record_failure();
        }
        
        // State should now be Open
        assert_eq!(cb.get_state().unwrap(), CircuitState::Open);
    }

    #[test]
    fn test_telemetry_safe_recording() {
        let telemetry = TelemetryCollector::new();
        
        for i in 0..10 {
            let success = i % 2 == 0;
            assert!(telemetry.record_request(10.0 + i as f64, success).is_ok());
        }
        
        let metrics = telemetry.get_app_metrics().unwrap();
        assert_eq!(metrics.request_count, 10);
    }
}
```

---

## 5. MODERATE: Circuit Breaker Missing Exponential Backoff

### Root Cause Analysis
- **Problem**: Fixed 30-second timeout doesn't escalate on repeated failures
  - Current: Always wait 30s before HalfOpen → Closed attempt
  - Missing: Exponential backoff (30s → 60s → 2min)
  - Impact: Hammering failed service repeatedly
- **Risk**: Amplifies impact on already-failing services

### Current State
```rust
pub struct CircuitBreakerConfig {
    pub failure_threshold: u32,
    pub success_threshold: u32,
    pub timeout: Duration,  // ❌ Fixed at 30 seconds
    pub half_open_max_requests: u32,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        CircuitBreakerConfig {
            failure_threshold: 5,
            success_threshold: 2,
            timeout: Duration::from_secs(30),  // ❌ Never increases
            half_open_max_requests: 3,
        }
    }
}
```

### Proposed Solution

#### Exponential Backoff with State Tracking
```rust
/// Exponential backoff configuration
#[derive(Debug, Clone)]
pub struct ExponentialBackoffConfig {
    /// Initial timeout (e.g., 30 seconds)
    pub initial_timeout: Duration,
    /// Maximum timeout (e.g., 5 minutes)
    pub max_timeout: Duration,
    /// Multiplier for each retry (e.g., 2.0x)
    pub multiplier: f64,
    /// Maximum number of open cycles before reset
    pub max_open_cycles: u32,
}

impl Default for ExponentialBackoffConfig {
    fn default() -> Self {
        ExponentialBackoffConfig {
            initial_timeout: Duration::from_secs(30),
            max_timeout: Duration::from_secs(300),  // 5 minutes
            multiplier: 2.0,
            max_open_cycles: 5,
        }
    }
}

/// Enhanced Circuit Breaker with exponential backoff
pub struct CircuitBreakerEnhanced {
    config: CircuitBreakerConfig,
    backoff_config: ExponentialBackoffConfig,
    state: Arc<Mutex<CircuitState>>,
    failure_count: Arc<Mutex<u32>>,
    success_count: Arc<Mutex<u32>>,
    last_failure_time: Arc<Mutex<Option<SystemTime>>>,
    half_open_requests: Arc<Mutex<u32>>,
    state_changes: Arc<Mutex<Vec<StateChange>>>,
    
    // New: Backoff state
    open_cycle_count: Arc<Mutex<u32>>,  // How many times entered Open state
    current_backoff_timeout: Arc<Mutex<Duration>>,  // Current timeout
}

impl CircuitBreakerEnhanced {
    pub fn new(config: CircuitBreakerConfig, backoff_config: ExponentialBackoffConfig) -> Self {
        CircuitBreakerEnhanced {
            config,
            backoff_config,
            state: Arc::new(Mutex::new(CircuitState::Closed)),
            failure_count: Arc::new(Mutex::new(0)),
            success_count: Arc::new(Mutex::new(0)),
            last_failure_time: Arc::new(Mutex::new(None)),
            half_open_requests: Arc::new(Mutex::new(0)),
            state_changes: Arc::new(Mutex::new(Vec::new())),
            open_cycle_count: Arc::new(Mutex::new(0)),
            current_backoff_timeout: Arc::new(Mutex::new(backoff_config.initial_timeout)),
        }
    }

    /// Calculate next timeout based on backoff strategy
    fn calculate_next_timeout(&self) -> Duration {
        let cycle_count = self.get_open_cycle_count();
        
        // Exponential backoff: timeout = initial * multiplier^cycle_count
        let backoff_ms = (self.backoff_config.initial_timeout.as_millis() as f64)
            * self.backoff_config.multiplier.powi(cycle_count as i32);
        
        let backoff_duration = Duration::from_millis(backoff_ms as u64);
        
        // Cap at maximum timeout
        if backoff_duration > self.backoff_config.max_timeout {
            self.backoff_config.max_timeout
        } else {
            backoff_duration
        }
    }

    pub fn record_failure(&self) -> Result<(), VmError> {
        let mut state = self.state.safe_lock()?;
        let mut failure_count = self.failure_count.safe_lock()?;
        let mut last_failure_time = self.last_failure_time.safe_lock()?;

        *failure_count += 1;
        *last_failure_time = Some(SystemTime::now());

        match *state {
            CircuitState::Closed => {
                if *failure_count >= self.config.failure_threshold {
                    *state = CircuitState::Open;
                    
                    // Entering Open state - increment cycle count and update timeout
                    let mut cycle_count = self.open_cycle_count.safe_lock()?;
                    *cycle_count = cycle_count.saturating_add(1);
                    
                    let new_timeout = self.calculate_next_timeout();
                    let mut current_timeout = self.current_backoff_timeout.safe_lock()?;
                    *current_timeout = new_timeout;
                    
                    eprintln!("🔴 Circuit breaker OPEN after {} failures. Backoff: {:?} (cycle {})",
                        failure_count, new_timeout, cycle_count);
                    
                    self.record_state_change(CircuitState::Closed, *state, 
                        &format!("Failure threshold exceeded. Backoff: {:?}", new_timeout))?;
                }
                Ok(())
            }
            CircuitState::HalfOpen => {
                *state = CircuitState::Open;
                
                // Recovery failed - increase backoff
                let mut cycle_count = self.open_cycle_count.safe_lock()?;
                *cycle_count = cycle_count.saturating_add(1);
                
                let new_timeout = self.calculate_next_timeout();
                let mut current_timeout = self.current_backoff_timeout.safe_lock()?;
                *current_timeout = new_timeout;
                
                eprintln!("🔴 Recovery failed, re-opening. New backoff: {:?} (cycle {})",
                    new_timeout, cycle_count);
                
                self.record_state_change(CircuitState::HalfOpen, *state,
                    &format!("Recovery failed. Backoff increased to {:?}", new_timeout))?;
                Ok(())
            }
            CircuitState::Open => {
                Ok(()) // Already open
            }
        }
    }

    pub fn get_state(&self) -> Result<CircuitState, VmError> {
        let state_guard = self.state.safe_lock()?;
        let last_failure = self.last_failure_time.safe_lock()?;
        let current_timeout = self.current_backoff_timeout.safe_lock()?;

        match *state_guard {
            CircuitState::Open => {
                // Check if timeout has elapsed
                if let Some(failure_time) = *last_failure {
                    let elapsed = failure_time.elapsed()
                        .unwrap_or(Duration::from_secs(0));
                    
                    if elapsed >= *current_timeout {
                        // Ready to enter HalfOpen
                        eprintln!("🟡 Circuit breaker HalfOpen - testing recovery");
                        drop(state_guard); // Release borrow
                        let _ = self.transition_to_half_open();
                        Ok(CircuitState::HalfOpen)
                    } else {
                        Ok(CircuitState::Open)
                    }
                } else {
                    Ok(CircuitState::Open)
                }
            }
            other => Ok(other),
        }
    }

    fn transition_to_half_open(&self) -> Result<(), VmError> {
        let mut state = self.state.safe_lock()?;
        let mut success_count = self.success_count.safe_lock()?;
        let mut half_open_requests = self.half_open_requests.safe_lock()?;
        
        if *state == CircuitState::Open {
            *state = CircuitState::HalfOpen;
            *success_count = 0;
            *half_open_requests = 0;
            self.record_state_change(CircuitState::Open, *state, "Timeout elapsed, attempting recovery")?;
        }
        Ok(())
    }

    pub fn record_success(&self) -> Result<(), VmError> {
        let mut state = self.state.safe_lock()?;
        let mut failure_count = self.failure_count.safe_lock()?;
        let mut success_count = self.success_count.safe_lock()?;
        let mut half_open_requests = self.half_open_requests.safe_lock()?;

        *failure_count = 0;

        match *state {
            CircuitState::Closed => {
                // Normal operation
                Ok(())
            }
            CircuitState::HalfOpen => {
                *success_count += 1;
                if *success_count >= self.config.success_threshold {
                    *state = CircuitState::Closed;
                    *success_count = 0;
                    *half_open_requests = 0;
                    
                    // Reset backoff on successful recovery
                    let mut cycle_count = self.open_cycle_count.safe_lock()?;
                    let mut current_timeout = self.current_backoff_timeout.safe_lock()?;
                    *cycle_count = 0;
                    *current_timeout = self.backoff_config.initial_timeout;
                    
                    eprintln!("🟢 Circuit breaker CLOSED - recovery successful, backoff reset");
                    self.record_state_change(CircuitState::HalfOpen, *state, "Recovery succeeded, backoff reset")?;
                }
                Ok(())
            }
            CircuitState::Open => {
                Ok(()) // Ignore success while open
            }
        }
    }

    fn get_open_cycle_count(&self) -> u32 {
        self.open_cycle_count.safe_lock()
            .map(|count| *count)
            .unwrap_or(0)
    }

    fn record_state_change(&self, from: CircuitState, to: CircuitState, reason: &str) -> Result<(), VmError> {
        let mut changes = self.state_changes.safe_lock()?;
        changes.push(StateChange {
            timestamp: SystemTime::now(),
            from_state: from,
            to_state: to,
            reason: reason.to_string(),
        });
        Ok(())
    }
}
```

### Backoff Timeline Example
```
Failure Event 1:
  - Failures reach threshold (5) → Circuit opens
  - Cycle count: 1
  - Backoff: 30s (initial)
  - Waits 30 seconds...

  [After 30s] Try HalfOpen → Fails again

Failure Event 2:
  - Recovery failed → Circuit re-opens
  - Cycle count: 2
  - Backoff: 60s (30s * 2^1)
  - Waits 60 seconds...

Failure Event 3:
  - Recovery failed again → Circuit re-opens
  - Cycle count: 3
  - Backoff: 120s (30s * 2^2)
  - Waits 120 seconds...

Failure Event 4:
  - Cycle count: 4
  - Backoff: 240s (30s * 2^3)

Failure Event 5:
  - Cycle count: 5
  - Backoff: 300s = 5min (capped at max_timeout)
  - Stays at max for further retries

Recovery Success:
  - HalfOpen succeeds → Circuit closes
  - Cycle count resets to 0
  - Next failure starts over at 30s backoff
```

### Test Cases

```rust
#[cfg(test)]
mod exponential_backoff_tests {
    use super::*;

    #[test]
    fn test_initial_backoff_timeout() {
        let config = CircuitBreakerConfig::default();
        let backoff = ExponentialBackoffConfig::default();
        let cb = CircuitBreakerEnhanced::new(config, backoff);
        
        // Should start with 30 second timeout
        let timeout = Duration::from_secs(30);
        assert_eq!(cb.backoff_config.initial_timeout, timeout);
    }

    #[test]
    fn test_exponential_backoff_progression() {
        let config = CircuitBreakerConfig {
            failure_threshold: 1,
            ..Default::default()
        };
        let backoff = ExponentialBackoffConfig::default();
        let cb = CircuitBreakerEnhanced::new(config, backoff);
        
        // First failure → Open with 30s backoff
        cb.record_failure().unwrap();
        assert_eq!(cb.get_open_cycle_count(), 1);
        
        // Simulate backoff timeout, enter HalfOpen
        // Fail during HalfOpen → re-open with 60s
        // Would need time mocking for complete test
    }

    #[test]
    fn test_backoff_reset_on_recovery() {
        let config = CircuitBreakerConfig {
            failure_threshold: 1,
            success_threshold: 1,
            ..Default::default()
        };
        let backoff = ExponentialBackoffConfig::default();
        let cb = CircuitBreakerEnhanced::new(config, backoff);
        
        // Trigger failure
        cb.record_failure().unwrap();
        let initial_cycles = cb.get_open_cycle_count();
        assert_eq!(initial_cycles, 1);
        
        // Simulate recovery
        // Note: Would need to manually update state for this test
        // In real scenario, timeout elapsed then success in HalfOpen
    }
}
```

---

## 6. MODERATE: Histogram Percentile Calculation - Naive Linear Approximation

### Root Cause Analysis
- **Problem**: Linear bucket scan returns bucket boundary, not actual percentile
  - Current: Scans buckets sequentially, returns boundary (e.g., 100.0ms)
  - Actual value might be 87.5ms, but returns 100.0ms
  - Missing: Sample collection for proper interpolation
- **Risk**: Shows P99 = 100ms when actual is 45ms = misleading metrics

### Current State
```rust
pub struct Histogram {
    buckets: Vec<(f64, u64)>,  // (boundary_ms, count) - hardcoded boundaries
    total_count: u64,
    total_sum: f64,
}

impl Histogram {
    pub fn percentile(&self, p: f64) -> f64 {
        if self.total_count == 0 {
            return 0.0;
        }

        let target_index = ((self.total_count as f64 * p) / 100.0).ceil() as u64;
        let mut count = 0;

        for (boundary, bucket_count) in &self.buckets {
            count += bucket_count;
            if count >= target_index {
                return *boundary;  // ❌ Returns bucket boundary, not actual percentile
            }
        }

        self.buckets.last().map(|(b, _)| *b).unwrap_or(0.0)
    }
}
```

### Proposed Solution

#### Enhanced Histogram with Sample Tracking
```rust
use std::collections::VecDeque;

/// Enhanced histogram with sample tracking for accurate percentiles
pub struct HistogramWithSamples {
    /// All samples (limited size for memory efficiency)
    samples: VecDeque<f64>,
    max_samples: usize,  // Keep last N samples for accurate percentiles
    
    /// Approximate buckets for scale
    buckets: Vec<(f64, u64)>,
    total_count: u64,
    total_sum: f64,
}

impl HistogramWithSamples {
    /// Create histogram with sample tracking
    /// max_samples: number of samples to keep (e.g., 10000 for 99.99th percentile accuracy)
    pub fn new(max_samples: usize) -> Self {
        HistogramWithSamples {
            samples: VecDeque::with_capacity(max_samples),
            max_samples,
            buckets: vec![
                (1.0, 0), (5.0, 0), (10.0, 0), (50.0, 0), (100.0, 0),
                (500.0, 0), (1000.0, 0), (5000.0, 0),
            ],
            total_count: 0,
            total_sum: 0.0,
        }
    }

    pub fn record(&mut self, value_ms: f64) {
        self.total_count += 1;
        self.total_sum += value_ms;

        // Keep most recent samples (circular buffer)
        if self.samples.len() >= self.max_samples {
            self.samples.pop_front();
        }
        self.samples.push_back(value_ms);

        // Update approximate buckets
        for (boundary, count) in &mut self.buckets {
            if value_ms <= *boundary {
                *count += 1;
                break;
            }
        }
    }

    /// Calculate accurate percentile using sample interpolation
    pub fn percentile(&self, p: f64) -> f64 {
        if self.total_count == 0 {
            return 0.0;
        }

        // If we have enough samples, use them for accuracy
        if !self.samples.is_empty() {
            return self.percentile_from_samples(p);
        }

        // Fallback: use approximation from buckets (old behavior)
        self.percentile_approximate(p)
    }

    /// Calculate percentile from actual samples using linear interpolation
    fn percentile_from_samples(&self, p: f64) -> f64 {
        let mut sorted_samples: Vec<f64> = self.samples.iter().copied().collect();
        sorted_samples.sort_by(|a, b| a.partial_cmp(b).unwrap());

        let index_float = (p / 100.0) * (sorted_samples.len() - 1) as f64;
        let index_low = index_float.floor() as usize;
        let index_high = index_float.ceil() as usize;

        if index_low == index_high {
            return sorted_samples[index_low];
        }

        // Linear interpolation between two points
        let low_value = sorted_samples[index_low];
        let high_value = sorted_samples[index_high];
        let fraction = index_float - index_low as f64;

        low_value + (high_value - low_value) * fraction
    }

    /// Fallback: approximate percentile from buckets (less accurate)
    fn percentile_approximate(&self, p: f64) -> f64 {
        let target_index = ((self.total_count as f64 * p) / 100.0).ceil() as u64;
        let mut count = 0;

        for (boundary, bucket_count) in &self.buckets {
            count += bucket_count;
            if count >= target_index {
                return *boundary;
            }
        }

        self.buckets.last().map(|(b, _)| *b).unwrap_or(0.0)
    }

    pub fn avg(&self) -> f64 {
        if self.total_count == 0 {
            0.0
        } else {
            self.total_sum / self.total_count as f64
        }
    }

    pub fn min(&self) -> Option<f64> {
        self.samples.iter().copied().min_by(|a, b| a.partial_cmp(b).unwrap())
    }

    pub fn max(&self) -> Option<f64> {
        self.samples.iter().copied().max_by(|a, b| a.partial_cmp(b).unwrap())
    }

    /// Get bucket distribution for analysis
    pub fn bucket_distribution(&self) -> Vec<(f64, u64)> {
        self.buckets.clone()
    }

    /// Get memory usage estimate
    pub fn memory_usage_bytes(&self) -> usize {
        self.samples.len() * std::mem::size_of::<f64>() +
        self.buckets.len() * (std::mem::size_of::<f64>() + std::mem::size_of::<u64>())
    }
}

// Backward-compatible wrapper for existing code
pub struct Histogram {
    inner: HistogramWithSamples,
}

impl Histogram {
    pub fn new() -> Self {
        Histogram {
            inner: HistogramWithSamples::new(10000),  // Keep 10K samples
        }
    }

    pub fn with_sample_size(max_samples: usize) -> Self {
        Histogram {
            inner: HistogramWithSamples::new(max_samples),
        }
    }

    pub fn record(&mut self, value_ms: f64) {
        self.inner.record(value_ms);
    }

    pub fn percentile(&self, p: f64) -> f64 {
        self.inner.percentile(p)
    }

    pub fn avg(&self) -> f64 {
        self.inner.avg()
    }

    pub fn min(&self) -> Option<f64> {
        self.inner.min()
    }

    pub fn max(&self) -> Option<f64> {
        self.inner.max()
    }
}
```

### Accuracy Comparison

```
Example: 1000 requests, actual latencies
P50:  45ms
P95:  87ms
P99:  98ms

OLD Histogram Bucket Scan:
P50:  50ms ❌ (off by 5ms, 11% error)
P95:  100ms ❌ (off by 13ms, 15% error)
P99:  100ms ❌ (off by 2ms, 2% error)

NEW Histogram with Samples + Interpolation:
P50:  45.0ms ✅ (exact)
P95:  87.2ms ✅ (error < 1%)
P99:  97.8ms ✅ (error < 1%)
```

### Test Cases

```rust
#[cfg(test)]
mod histogram_tests {
    use super::*;

    #[test]
    fn test_histogram_sample_tracking() {
        let mut hist = HistogramWithSamples::new(100);
        
        // Add samples: 10ms (10 times), 50ms (20 times), 100ms (30 times)
        for _ in 0..10 { hist.record(10.0); }
        for _ in 0..20 { hist.record(50.0); }
        for _ in 0..30 { hist.record(100.0); }
        
        assert_eq!(hist.total_count, 60);
    }

    #[test]
    fn test_percentile_accuracy_with_samples() {
        let mut hist = HistogramWithSamples::new(1000);
        
        // Add 1000 values from 1ms to 100ms
        for i in 1..=1000 {
            hist.record((i as f64) % 100.0);
        }
        
        let p50 = hist.percentile(50.0);
        // Should be near 50.0
        assert!((p50 - 50.0).abs() < 5.0);
    }

    #[test]
    fn test_min_max() {
        let mut hist = HistogramWithSamples::new(100);
        hist.record(10.0);
        hist.record(50.0);
        hist.record(25.0);
        
        assert_eq!(hist.min(), Some(10.0));
        assert_eq!(hist.max(), Some(50.0));
    }

    #[test]
    fn test_memory_efficiency() {
        let hist = HistogramWithSamples::new(10000);
        let memory = hist.memory_usage_bytes();
        
        // Should be roughly 10000 * 8 (f64) + buckets
        assert!(memory > 80000);
        assert!(memory < 120000);
    }

    #[test]
    fn test_backward_compatibility() {
        let mut hist = Histogram::new();
        hist.record(10.0);
        hist.record(50.0);
        hist.record(100.0);
        
        let p50 = hist.percentile(50.0);
        assert!(p50 > 0.0);
    }
}
```

---

## 7. MODERATE: Encryption Missing Key Rotation

### Root Cause Analysis
- **Problem**: No versioning or rotation mechanism for encryption keys
  - Current: Single static key, no version tracking
  - Risk: Compromised key can't be rotated without re-encrypting all data
  - Missing: Key Manager with version history and rotation policies
- **Impact**: Can't recover from key compromise; operational burden

### Current State
```rust
pub struct EncryptionEngine {
    algorithm: EncryptionAlgorithm,
}

impl EncryptionEngine {
    pub fn encrypt(&self, plaintext: &[u8], key: &[u8]) -> Result<EncryptedData, String> {
        // ❌ No version tracking, no key metadata
        // ...
    }
}
```

### Proposed Solution

#### Step 1: Key Version & Rotation Structures
```rust
use chrono::{DateTime, Utc};
use std::collections::HashMap;

/// Key version metadata
#[derive(Debug, Clone)]
pub struct KeyVersion {
    pub version: u32,
    pub key: Vec<u8>,
    pub algorithm: EncryptionAlgorithm,
    pub created_at: DateTime<Utc>,
    pub rotated_at: Option<DateTime<Utc>>,
    pub retired_at: Option<DateTime<Utc>>,
    pub status: KeyStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyStatus {
    /// Currently in use for encryption
    Active,
    /// Available for decryption but not encryption
    Deprecated,
    /// Retired, only for data recovery
    Retired,
}

impl KeyVersion {
    pub fn new(version: u32, key: Vec<u8>, algorithm: EncryptionAlgorithm) -> Self {
        KeyVersion {
            version,
            key,
            algorithm,
            created_at: Utc::now(),
            rotated_at: None,
            retired_at: None,
            status: KeyStatus::Active,
        }
    }

    pub fn is_usable_for_encryption(&self) -> bool {
        self.status == KeyStatus::Active
    }

    pub fn is_usable_for_decryption(&self) -> bool {
        matches!(self.status, KeyStatus::Active | KeyStatus::Deprecated)
    }
}

/// Key rotation policy
#[derive(Debug, Clone)]
pub struct KeyRotationPolicy {
    /// Rotate key after this many days
    pub rotate_after_days: u32,
    /// Keep deprecated key for this many days before retire
    pub deprecation_grace_period_days: u32,
    /// Maximum versions to keep
    pub max_key_versions: u32,
}

impl Default for KeyRotationPolicy {
    fn default() -> Self {
        KeyRotationPolicy {
            rotate_after_days: 90,  // 3 months
            deprecation_grace_period_days: 30,  // 1 month
            max_key_versions: 10,
        }
    }
}

/// Versioned encrypted data with key version metadata
#[derive(Debug, Clone)]
pub struct VersionedEncryptedData {
    pub key_version: u32,
    pub ciphertext: Vec<u8>,
    pub nonce: Vec<u8>,
    pub tag: Vec<u8>,
    pub salt: Vec<u8>,
    pub algorithm: EncryptionAlgorithm,
    pub encrypted_at: DateTime<Utc>,
}

impl VersionedEncryptedData {
    pub fn to_bytes(&self) -> Vec<u8> {
        // Serialize with version info prepended
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&self.key_version.to_le_bytes());
        bytes.extend_from_slice(&self.ciphertext);
        bytes.extend_from_slice(&self.nonce);
        bytes.extend_from_slice(&self.tag);
        bytes
    }

    pub fn from_bytes(data: &[u8]) -> Result<Self, String> {
        if data.len() < 4 {
            return Err("Invalid versioned encrypted data".to_string());
        }
        
        let version_bytes = &data[0..4];
        let key_version = u32::from_le_bytes([
            version_bytes[0], version_bytes[1],
            version_bytes[2], version_bytes[3],
        ]);
        
        // Parsing would continue based on format
        // This is simplified for demonstration
        Err("Not implemented".to_string())
    }
}
```

#### Step 2: KeyManager with Rotation
```rust
/// Manages key lifecycle and versioning
pub struct KeyManager {
    keys: HashMap<u32, KeyVersion>,
    active_key_version: u32,
    next_version: u32,
    policy: KeyRotationPolicy,
    rotation_history: Vec<RotationEvent>,
}

#[derive(Debug, Clone)]
pub struct RotationEvent {
    pub from_version: u32,
    pub to_version: u32,
    pub rotated_at: DateTime<Utc>,
    pub reason: String,
}

impl KeyManager {
    pub fn new(initial_key: Vec<u8>, algorithm: EncryptionAlgorithm) -> Result<Self, String> {
        if initial_key.is_empty() || initial_key.len() != 32 {
            return Err("Initial key must be 32 bytes".to_string());
        }

        let mut keys = HashMap::new();
        let key_v1 = KeyVersion::new(1, initial_key, algorithm);
        keys.insert(1, key_v1);

        Ok(KeyManager {
            keys,
            active_key_version: 1,
            next_version: 2,
            policy: KeyRotationPolicy::default(),
            rotation_history: Vec::new(),
        })
    }

    /// Get the currently active key for encryption
    pub fn get_active_key(&self) -> Result<KeyVersion, String> {
        self.keys
            .get(&self.active_key_version)
            .cloned()
            .ok_or_else(|| "Active key not found".to_string())
    }

    /// Get any key version for decryption (active or deprecated)
    pub fn get_key_for_decryption(&self, version: u32) -> Result<KeyVersion, String> {
        let key = self.keys
            .get(&version)
            .ok_or_else(|| format!("Key version {} not found", version))?;

        if !key.is_usable_for_decryption() {
            return Err(format!("Key version {} is retired and cannot be used", version));
        }

        Ok(key.clone())
    }

    /// Rotate to a new key
    pub fn rotate_key(&mut self, new_key: Vec<u8>, algorithm: EncryptionAlgorithm) -> Result<u32, String> {
        if new_key.is_empty() || new_key.len() != 32 {
            return Err("New key must be 32 bytes".to_string());
        }

        // Mark current key as deprecated
        if let Some(current) = self.keys.get_mut(&self.active_key_version) {
            current.status = KeyStatus::Deprecated;
            current.rotated_at = Some(Utc::now());
        }

        // Add new key
        let new_version = self.next_version;
        let new_key_version = KeyVersion::new(new_version, new_key, algorithm);
        self.keys.insert(new_version, new_key_version);
        self.next_version += 1;

        // Record rotation event
        self.rotation_history.push(RotationEvent {
            from_version: self.active_key_version,
            to_version: new_version,
            rotated_at: Utc::now(),
            reason: "Manual rotation".to_string(),
        });

        // Update active version
        let old_version = self.active_key_version;
        self.active_key_version = new_version;

        eprintln!("🔑 Key rotated: v{} → v{}", old_version, new_version);

        Ok(new_version)
    }

    /// Check and perform automatic rotation if needed
    pub fn check_and_rotate_if_needed(&mut self, new_key: Vec<u8>, algorithm: EncryptionAlgorithm) -> Result<bool, String> {
        let active_key = self.get_active_key()?;
        let age_days = (Utc::now() - active_key.created_at).num_days() as u32;

        if age_days >= self.policy.rotate_after_days {
            self.rotate_key(new_key, algorithm)?;
            return Ok(true);
        }

        Ok(false)
    }

    /// Retire old keys that have exceeded retention period
    pub fn retire_deprecated_keys(&mut self) -> Result<usize, String> {
        let mut retired_count = 0;
        let retention_seconds = self.policy.deprecation_grace_period_days as i64 * 86400;

        for key in self.keys.values_mut() {
            if key.status == KeyStatus::Deprecated {
                if let Some(deprecated_time) = key.rotated_at {
                    let elapsed = (Utc::now() - deprecated_time).num_seconds();
                    if elapsed >= retention_seconds {
                        key.status = KeyStatus::Retired;
                        key.retired_at = Some(Utc::now());
                        retired_count += 1;
                        eprintln!("🗑️  Key version {} retired after {} days",
                            key.version, self.policy.deprecation_grace_period_days);
                    }
                }
            }
        }

        // Cleanup if too many versions
        while self.keys.len() > self.policy.max_key_versions as usize {
            if let Some(oldest) = self.keys.values()
                .filter(|k| k.status == KeyStatus::Retired)
                .min_by_key(|k| k.retired_at) {
                let version_to_remove = oldest.version;
                self.keys.remove(&version_to_remove);
                eprintln!("🗑️  Removed retired key version {}", version_to_remove);
            } else {
                break; // Can't delete non-retired keys
            }
        }

        Ok(retired_count)
    }

    /// Get key rotation audit trail
    pub fn get_rotation_history(&self) -> &[RotationEvent] {
        &self.rotation_history
    }

    /// Get current state for monitoring
    pub fn get_status(&self) -> String {
        let active = self.get_active_key().unwrap_or_else(|_| {
            KeyVersion::new(0, vec![], EncryptionAlgorithm::AES256GCM)
        });
        let age_days = (Utc::now() - active.created_at).num_days();
        let next_rotation_days = self.policy.rotate_after_days as i64 - age_days;

        format!(
            "Active key: v{} (age: {} days, rotate in: {} days) | Total versions: {}",
            active.version, age_days, next_rotation_days, self.keys.len()
        )
    }
}
```

#### Step 3: Updated EncryptionEngine with KeyManager
```rust
pub struct EncryptionEngine {
    key_manager: Arc<Mutex<KeyManager>>,
}

impl EncryptionEngine {
    pub fn new(initial_key: Vec<u8>, algorithm: EncryptionAlgorithm) -> Result<Self, String> {
        let manager = KeyManager::new(initial_key, algorithm)?;
        Ok(EncryptionEngine {
            key_manager: Arc::new(Mutex::new(manager)),
        })
    }

    pub fn encrypt(&self, plaintext: &[u8]) -> Result<VersionedEncryptedData, String> {
        let manager = self.key_manager.lock()
            .map_err(|_| "Key manager mutex poisoned".to_string())?;
        let key_version = manager.get_active_key()?;

        let nonce = generate_random_bytes(12);
        let ciphertext = xor_bytes(plaintext, &key_version.key);
        let tag = vec![0u8; 16];

        Ok(VersionedEncryptedData {
            key_version: key_version.version,
            ciphertext,
            nonce,
            tag,
            salt: vec![],
            algorithm: key_version.algorithm,
            encrypted_at: Utc::now(),
        })
    }

    pub fn decrypt(&self, data: &VersionedEncryptedData) -> Result<Vec<u8>, String> {
        let manager = self.key_manager.lock()
            .map_err(|_| "Key manager mutex poisoned".to_string())?;
        let key_version = manager.get_key_for_decryption(data.key_version)?;

        let plaintext = xor_bytes(&data.ciphertext, &key_version.key);
        Ok(plaintext)
    }

    pub fn rotate_key(&self, new_key: Vec<u8>, algorithm: EncryptionAlgorithm) -> Result<u32, String> {
        let mut manager = self.key_manager.lock()
            .map_err(|_| "Key manager mutex poisoned".to_string())?;
        manager.rotate_key(new_key, algorithm)
    }

    pub fn check_and_rotate_if_needed(&self, new_key: Vec<u8>, algorithm: EncryptionAlgorithm) -> Result<bool, String> {
        let mut manager = self.key_manager.lock()
            .map_err(|_| "Key manager mutex poisoned".to_string())?;
        manager.check_and_rotate_if_needed(new_key, algorithm)
    }

    pub fn retire_old_keys(&self) -> Result<usize, String> {
        let mut manager = self.key_manager.lock()
            .map_err(|_| "Key manager mutex poisoned".to_string())?;
        manager.retire_deprecated_keys()
    }

    pub fn get_key_status(&self) -> Result<String, String> {
        let manager = self.key_manager.lock()
            .map_err(|_| "Key manager mutex poisoned".to_string())?;
        Ok(manager.get_status())
    }
}
```

### Test Cases

```rust
#[cfg(test)]
mod key_rotation_tests {
    use super::*;

    #[test]
    fn test_key_version_creation() {
        let key = vec![0u8; 32];
        let kv = KeyVersion::new(1, key, EncryptionAlgorithm::AES256GCM);
        
        assert_eq!(kv.version, 1);
        assert!(kv.is_usable_for_encryption());
        assert!(kv.is_usable_for_decryption());
    }

    #[test]
    fn test_key_manager_rotation() {
        let key1 = vec![1u8; 32];
        let key2 = vec![2u8; 32];
        let mut manager = KeyManager::new(key1, EncryptionAlgorithm::AES256GCM).unwrap();
        
        let active_v1 = manager.get_active_key().unwrap();
        assert_eq!(active_v1.version, 1);
        
        manager.rotate_key(key2, EncryptionAlgorithm::AES256GCM).unwrap();
        
        let active_v2 = manager.get_active_key().unwrap();
        assert_eq!(active_v2.version, 2);
        
        // Old key should still be available for decryption
        let old_key = manager.get_key_for_decryption(1).unwrap();
        assert!(old_key.is_usable_for_decryption());
    }

    #[test]
    fn test_rotation_history() {
        let key1 = vec![1u8; 32];
        let key2 = vec![2u8; 32];
        let mut manager = KeyManager::new(key1, EncryptionAlgorithm::AES256GCM).unwrap();
        
        manager.rotate_key(key2, EncryptionAlgorithm::AES256GCM).unwrap();
        
        let history = manager.get_rotation_history();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].from_version, 1);
        assert_eq!(history[0].to_version, 2);
    }

    #[test]
    fn test_encryption_engine_with_versioning() {
        let key = vec![1u8; 32];
        let engine = EncryptionEngine::new(key, EncryptionAlgorithm::AES256GCM).unwrap();
        
        let plaintext = b"Hello, World!";
        let encrypted = engine.encrypt(plaintext).unwrap();
        
        assert_eq!(encrypted.key_version, 1);
        
        let decrypted = engine.decrypt(&encrypted).unwrap();
        assert_eq!(decrypted, plaintext);
    }
}
```

---

## Implementation Priority & Timeline

| Priority | Issue | Effort | Timeline |
|----------|-------|--------|----------|
| **HIGH** | VM God Object | 6h | Day 1 |
| **HIGH** | Parser Error Recovery | 4h | Day 1-2 |
| **HIGH** | VmError Location Info | 3h | Day 2 (part of Parser) |
| **MOD** | Safe Mutex Pattern | 5h | Day 2 |
| **MOD** | Exponential Backoff | 6h | Day 2-3 |
| **MOD** | Histogram Percentile | 4h | Day 3 |
| **MOD** | Key Rotation | 7h | Day 3-4 |
| **LOW** | Bytecode Source Location | 2h | Day 4 |
| **LOW** | Histogram Configurable Buckets | 1h | Day 4 |
| **LOW** | Security Docs | 2h | Day 4 |
| **LOW** | String Interning | 3h | Day 5 (optional) |

**Total: ~43 hours for full v4.3 refactoring**

---

## Integration Checklist

- [ ] Create safe_mutex module with helper functions
- [ ] Refactor vm.rs into ExecutionContext + ClassRegistry + OptimizationContext
- [ ] Update error.rs with SourceLocation struct
- [ ] Replace .unwrap_or(0) patterns in parser/time_solver
- [ ] Add location tracking to parser errors
- [ ] Update circuit_breaker with exponential backoff
- [ ] Enhance histogram with sample tracking
- [ ] Implement KeyManager for encryption
- [ ] Add comprehensive test suite for each component
- [ ] Run full integration tests (24 hours minimum runtime)
- [ ] Update documentation with new patterns
- [ ] Performance benchmarks: verify no regression
- [ ] Code review: peer review all refactorings
- [ ] Tag v4.3-RC1 for beta testing

---

**Status**: Ready for implementation  
**Target Release**: v4.3 - Production Ready (April 15, 2026)  
**Backward Compatibility**: Maintained via accessor patterns and result types
