# Killer Language - System Architecture

**Version**: 4.2-SUPER  
**Date**: March 21, 2026  
**Status**: Production-Ready (v1.1), Beta Features (v1.2 Alpha)

---

## Table of Contents

1. [System Overview](#system-overview)
2. [Module Organization](#module-organization)
3. [Compilation Pipeline](#compilation-pipeline)
4. [Runtime Architecture](#runtime-architecture)
5. [Performance Optimizations](#performance-optimizations)
6. [Security Model](#security-model)
7. [Design Patterns](#design-patterns)
8. [Configuration & Tuning](#configuration--tuning)
9. [Concurrency Model](#concurrency-model)
10. [Error Handling](#error-handling)

---

## System Overview

Killer is a **high-performance systems programming language** built on Rust, featuring:

```
┌─────────────────────────────────────────────────────────┐
│            Killer Language Runtime (v4.2-SUPER)         │
├─────────────────────────────────────────────────────────┤
│                                                          │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐  │
│  │   Frontend   │→ │   Compiler   │→ │   Bytecode   │  │
│  │ (Lexer/Prsr) │  │  & Optimizer │  │   /LLVM      │  │
│  └──────────────┘  └──────────────┘  └──────────────┘  │
│         │                │                    │         │
│         └────────────────┼────────────────────┘         │
│                          ▼                               │
│             ┌────────────────────────┐                  │
│             │   Virtual Machine      │                  │
│             │  with Optimization     │                  │
│             │   Engine               │                  │
│             └────────────────────────┘                  │
│                    │         │         │                │
│            ┌───────┴─────┬───┴───┬────┴──────┐         │
│            ▼             ▼       ▼           ▼         │
│     Instr.  JIT    Hot    BaselJIT  Native  Call      │
│     Cache   Cmplr  Detect  JIT     GenCode  Cache     │
│                                                          │
└─────────────────────────────────────────────────────────┘
```

**Performance Characteristics**:
- v1.1: 72.78ms average latency (10 algorithms, 100K concurrent actors)
- v1.2 Alpha: 8-12ms Hash Map operations, 8-9ms Dijkstra on 100 vertices
- Optimization: 6-11.8x speedup potential with v1.2 optimizations

---

## Module Organization

### Core Modules Hierarchy

```
killer_native (lib.rs)
│
├── error.rs                    # Error types & handling
├── security.rs                 # Path validation, recursion limits (NEW)
│
├── lexer.rs                    # Tokenization with hybrid syntax
├── parser.rs                   # AST generation, type annotations
├── ast.rs                      # Abstract Syntax Tree definitions
│
├── compiler.rs                 # Semantic analysis, variable tracking
├── bytecode.rs                 # Instruction set, bytecode program
│
├── type_system.rs              # Type checking, type inference
├── type_specialization/        # Type-driven code generation
│
├── runtime_optimization.rs    # Optimization levels (O0-O3), GC strategies
├── optimization_engine.rs      # Consolidated optimization modules (NEW)
│   ├── instruction_cache.rs    # Cache instruction lookups
│   ├── jit_compiler.rs         # Simple JIT compilation
│   ├── hot_code_detector.rs    # Profile-guided optimization
│   ├── baseline_jit.rs         # Baseline JIT tier
│   ├── fast_path.rs            # Fast-path arithmetic loops
│   ├── native_codegen.rs       # x86-64 code generation
│   ├── variable_caching.rs     # O(1) variable access
│   ├── loop_pattern_detection.rs # Loop optimization hints
│   ├── call_site_cache.rs      # Method call caching
│   ├── allocation_pool.rs      # Memory pool reuse
│   └── scope_var_cache.rs      # Scope lookup cache
│
├── vm.rs                       # Virtual Machine interpreter
├── value.rs                    # Runtime value representation
├── builtin.rs                  # Built-in functions (220+)
│
├── exception.rs                # Try/catch/finally handling
├── generator.rs                # Yield/generator management
│
├── stdlib.rs                   # Standard Library interface
├── concurrency.rs              # Actor model, channels
├── generics.rs                 # Generic type system
│
├── formatter.rs                # Code formatting (30+ style rules)
├── linter.rs                   # Code quality analysis
├── debugger.rs                 # Interactive debugger
├── repl.rs                     # Interactive REPL shell
│
├── config.rs                   # Configuration management
├── version.rs                  # Version information
└── main.rs                     # CLI interface
```

### Module Dependencies (DAG)

```
lexer.rs      ──┐
                ├──→ parser.rs ──→ ast.rs ──→ compiler.rs
parser.rs     ──┘                                   │
                                                    ▼
error.rs  ────┐                              bytecode.rs
              ├──→ vm.rs ←──────┬──────────────────┼──→ value.rs ──→ builtin.rs
security.rs ──┤                 │                  │
              │            exception.rs      generator.rs
type_system.rs┼──→ type_specialization/
              │
runtime_optimization.rs ─→ optimization_engine.rs ──→ vm.rs
```

**Key Property**: No circular dependencies. All modules form a directed acyclic graph (DAG).

---

## Compilation Pipeline

### Phase 1: Lexing (lexer.rs)

**Input**: Source code (.killer file)  
**Output**: Token stream  
**Hybrid Syntax Support**: 
- Indentation-based (Python-like): automatic INDENT/DEDENT tokens
- Brace-based (C-like): `{}` delimiters

**Algorithm**:
```
1. Scan characters at O(n) linear time
2. Track indentation levels with stack
3. Emit INDENT/DEDENT tokens on level change
4. Handle mixed tabs/spaces: ERROR
5. Skip comments (// to EOL)
```

**Indentation Tracking**:
```
indent_stack = [0]
─────────────────────────────
Input:
  kfn foo()
    x = 1
    y = 2

Tokens:
  KFN, ID("foo"), LPAREN, RPAREN, NEWLINE
  INDENT(4), ID("x"), ...
  NEWLINE
  ID("y"), ...
  NEWLINE
  DEDENT(1)
```

### Phase 2: Parsing (parser.rs)

**Input**: Token stream  
**Output**: AST with type annotations  
**Features**:
- Recursive descent parser
- Type annotation extraction: `var x: Int`, `fn(Int) -> String`
- Pattern matching support
- Operator precedence tables
- Error recovery (collect multiple errors)

**Key Constructs**:
- Functions: `kfn name(params: Type) -> ReturnType { body }`
- Control Flow: `if/else`, `while`, `for`, `match`
- Literals: numbers, strings (with K-string interpolation), arrays, objects
- Operators: arithmetic, logical, comparison, assignment

### Phase 3: Compilation (compiler.rs + bytecode.rs)

**Input**: AST  
**Output**: Bytecode instructions (Program)

**Compilation Strategy**:
1. Semantic analysis (variable collection, scope checking)
2. Type checking (if type annotations present)
3. Bytecode generation (one instruction per source operation)

**Bytecode Examples**:
```rust
ConstNum(42)           // Load number literal
ConstStr("hello")      // Load string literal
Store("x")             // Store stack top to variable x
Load("x")              // Load variable x to stack
Add                    // Pop 2 values, push sum
Call { name, .. }      // Call function
JumpIfFalse(target)    // Conditional jump
Return(value)          // Return from function
```

### Phase 4: Optimization (optimization_engine.rs)

**Input**: Bytecode program  
**Output**: Optimized bytecode or native code

See [Performance Optimizations](#performance-optimizations) section.

### Phase 5: Execution (vm.rs)

**Input**: Bytecode program  
**Output**: Program results / stdout

---

## Runtime Architecture

### Virtual Machine (vm.rs)

**Design**: Stack-based interpreter with optimization layers

```rust
pub struct VirtualMachine {
    // Core state
    stack: Vec<Value>,                           // Value stack
    scopes: Vec<HashMap<String, Value>>,         // Scope stack
    call_stack: Vec<usize>,                      // Call stack (IP history)
    ip: usize,                                   // Instruction pointer
    
    // Optimization engine (NEW)
    optimization_engine: OptimizationEngine,     // All perf modules
    
    // Security (NEW)
    recursion_guard: RecursionGuard,             // Prevent stack overflow
    
    // Exception/control flow
    exception_manager: ExceptionManager,
    generator_manager: GeneratorManager,
}
```

**Execution Loop**:
```rust
while ip < program.instructions.len() {
    instruction = program.instructions[ip]
    
    match instruction {
        Instruction::ConstNum(n) => stack.push(Value::Number(n)),
        Instruction::Add => {
            rhs = stack.pop()
            lhs = stack.pop()
            stack.push(lhs + rhs)
        }
        Instruction::Call { function, args } => {
            result = call_function(function, args)
            stack.push(result)
        }
        // ... more instruction handlers
    }
    
    ip += 1
}
```

### Value Representation (value.rs)

```rust
pub enum Value {
    Number(f64),                        // 64-bit float
    String(String),                     // Heap string
    Boolean(bool),                      // True/False
    Array(Vec<Value>),                  // Dynamic array
    Map(HashMap<String, Value>),        // Hash map
    Function { params, bytecode, captured },  // Closure
    Object(ObjectInstance),             // Class instance
    Null,                               // Null value
}
```

**Memory Model**:
- Values stored on heap where needed (String, Array, Map, Object)
- Stack holds Value enum (small enum + pointer)
- Garbage collection: Mark-and-sweep (future: generational GC)

### Scope Management

```rust
// Variable scopes form a stack
scopes = [
    { "global_x": 1, "global_y": 2 },        // Global scope
    { "local_a": 10 },                       // Function scope
    { "block_b": 20 },                       // Block scope
]

// Load "local_a":
// Search from top of stack down, return first match

// Store "local_b": 99
// Store in topmost scope (block scope)
```

**Key Properties**:
- O(n) lookup if n scopes (optimized with ScopeVariableCache → O(1))
- Variables shadow outer scopes
- Exiting scope: pop from stack

---

## Performance Optimizations

### Optimization Engine Architecture

```
OptimizationEngine (NEW: Consolidated module)
├── Tier 0: Instruction Cache
│   └─ Cache frequently-accessed instructions → ~5x speedup
├── Tier 1: Hot Code Detection
│   └─ Track loop iteration counts → 1000 threshold
├── Tier 2: Baseline JIT
│   └─ Compile hot loops to native code → ~10x speedup
├── Tier 3: Fast Path Specialization
│   └─ Skip type checks in arithmetic loops → ~5x speedup
├── Tier 4: Native x86-64 Codegen
│   └─ Direct machine code with SIMD → ~20x speedup
├── Variable Caching
│   └─ O(1) hot variable access → ~3x for loop-heavy code
├── Call Site Caching
│   └─ Cache method resolution → ~3-5% speedup
└── Memory Pool Management
    └─ Reuse Value allocations → ~2-3% speedup
```

### Configuration (Cargo.toml)

```toml
[profile.release]
opt-level = 3              # Maximum optimization
lto = "thin"               # Link-time optimization
codegen-units = 16         # Parallel codegen
strip = true               # Strip debug symbols
panic = "abort"            # Faster panic handling
overflow-checks = false    # No bounds checking (production only)

[env]
RUSTFLAGS = "-C target-cpu=native"  # CPU-specific optimizations
```

**Expected Gains**:
- Database ops: +6x
- Agent consensus: +7x
- Variable-size ops: +11.8x
- CMI export: +7-8x

### Optimization Levels (O0-O3)

| Level | Focus | Modules Enabled | Use Case |
|-------|-------|-----------------|----------|
| **O0** | Fast compile | Instruction cache | Debug/testing |
| **O1** | Balanced | +Variable cache, loop detector | Development |
| **O2** | **(Default)** | +JIT, +Call cache | Production |
| **O3** | Maximum | All modules | Performance-critical |

### Hot Code Detection

**Algorithm** (HotCodeDetector):
1. Track loop iteration count
2. At 1000 iterations: Mark as "hot"
3. Transition to Baseline JIT
4. Compile to native code
5. Cache native function

**Example**:
```killer
for i in 0..100_000
  sum = sum + i          // Hot inner loop

// After 1000 iterations, this loop compiles to native
// Subsequent iterations run ~10x faster
```

### Loop Pattern Detection

Identifies optimization opportunities:
- Vectorizable loops (SIMD candidates)
- Allocations in loops (move outside)
- Cache-unfriendly patterns (prefetch hints)
- Parallel loops (actor spawn candidates)

---

## Security Model

### Path Safety (security.rs - NEW)

**Problem**: File path traversal attacks
```
killer file ../../../etc/passwd  // Could read sensitive files!
```

**Solution**: Path validation with whitelist
```rust
pub fn validate_file_path(path: &str) -> Result<PathBuf> {
    // Reject absolute paths
    // Reject .. (parent directory traversal)
    // Check against allowed directories
    // Canonicalize and verify result
}
```

**Default Allowed Directories**:
- `.` (current directory)
- `./examples`
- `./src`
- `./tests`

**Configuration**:
```rust
let mut config = SecurityConfig::default();
config.allowed_directories.push(PathBuf::from("/var/data/app"));
```

### Recursion Depth Limiting (security.rs - NEW)

**Problem**: Infinite recursion crashes with stack overflow
```killer
kfn recurse()
  recurse()

recurse()  // Stack overflow!
```

**Solution**: Recursion guard with depth tracking
```rust
pub struct RecursionGuard {
    current_depth: usize,
    max_depth: usize = 10_000,
}

// VM checks before each function call
guard.enter()?  // Err if depth exceeded
```

**Overhead**: ~0.1% per function call (negligible)

### File Size Limits

**Problem**: DOS attack with huge files
```
killer huge_file.killer  // 100MB file
```

**Solution**: Size checking before read
```
MAX_FILE_SIZE = 64 MB  // Configurable
MAX_PARSER_INPUT_SIZE = 100 MB
```

### Parser Nesting Depth

**Problem**: Deeply nested expressions exhaust parser stack
```
f(f(f(...f(x)...)))  // 10K nesting levels → stack overflow
```

**Solution**: Nesting depth limit
```
MAX_NESTING_DEPTH = 500  // Reasonable for any real code
```

### Input Validation

**Applied At**:
- Main CLI: Path validation for all file operations
- Lexer: File size check before tokenization
- Parser: Nesting depth tracking
- VM: Recursion depth checking

**Security Configuration Flow**:
```
main.rs
  └→ validate_file_path(path, config)
       └→ check!(no ..)
       └→ check!(canonicalize)
       └→ check!(in allowed_dirs)
       └→ check!(file_size <= MAX)
       └→ read_file_safe()
```

---

## Design Patterns

### 1. Builder Pattern (OptimizationEngine)

```rust
let engine = OptimizationEngine::with_level(OptimizationLevel::O3)
    .with_recursion_limit(5_000)
    .enable_jit(true);
```

**Benefits**:
- Flexible configuration
- Clear intent
- Defaults for common cases

### 2. Visitor Pattern (AST Traversal - compiler.rs)

```rust
fn collect_variables(stmt: &Stmt) -> HashSet<String> {
    match stmt {
        Stmt::Let { pattern, value } => { ... }
        Stmt::If { condition, then_branch, else_branch } => { ... }
        Stmt::While { condition, body } => { ... }
        // ...
    }
}
```

**Benefits**:
- Clean separation of concerns
- Easy to add new analyses
- Type-safe pattern matching

### 3. RAII Pattern (RecursionGuard)

```rust
let _guard = recursion_guard.enter()?;  // Increment
// ... do work ...
// Drop: automatically decrement (no panics mid-work)
```

**Benefits**:
- Exception-safe depth tracking
- Automatic cleanup
- No manual stack management

### 4. Module Facade (OptimizationEngine)

**Problem**: VirtualMachine exposes 10+ optimization modules directly
```rust
pub struct VirtualMachine {
    instruction_cache: ...,
    jit_compiler: ...,
    hot_detector: ...,
    // ... 7 more fields ...
}
// Clients: vm.instruction_cache.lookup()?
//          vm.jit_compiler.compile()?
//          vm.hot_detector.on_loop_iteration()?
```

**Solution**: Unified interface
```rust
pub struct VirtualMachine {
    optimization_engine: OptimizationEngine,  // Single field!
}
// Clients: vm.optimization_engine.cache(...)?
//          or vm.optimization_engine.compile(...)?
```

**Benefits**:
- Clean public API
- Internal flexibility
- Easy to test/profile

### 5. Strategy Pattern (GCStrategy, OptimizationLevel)

```rust
pub enum GCStrategy {
    MarkAndSweep,
    CopyingGC,
    GenerationalGC,
    IncrementalGC,
    ConcurrentGC,
}

// Runtime selects strategy based on workload
match gc_strategy {
    GCStrategy::Generational => run_generational_gc(),
    GCStrategy::Concurrent => run_concurrent_gc(),
    // ...
}
```

**Benefits**:
- Pluggable algorithms
- No runtime type checking
- Easy to add new strategies

---

## Configuration & Tuning

### Runtime Configuration Files

**`.killerrc` (YAML format)**:
```yaml
optimization:
  level: O2              # O0, O1, O2, O3
  max_recursion: 10000
  max_file_size: 67108864  # 64 MB

gc:
  strategy: GenerationalGC
  heap_size: 536870912   # 512 MB

parser:
  max_nesting_depth: 500
  max_input_size: 104857600

security:
  allow_unrestricted_file_access: false
  allowed_directories:
    - .
    - ./src
    - ./examples

formatter:
  indent_style: spaces
  indent_size: 4
  line_length: 100
```

### Environment Variables

```bash
KILLER_OPTIMIZATION_LEVEL=O3
KILLER_MAX_RECURSION=5000
KILLER_GC_STRATEGY=GenerationalGC
KILLER_DEBUG=1              # Enable debug output
```

### CLI Options

```bash
killer program.killer                    # Run with default config
killer --optimize O3 program.killer      # Explicit opt level
killer --gc=Concurrent program.killer    # Force GC strategy
killer --recurse-limit=5000 program      # Custom recursion limit
```

---

## Concurrency Model

### Actor Model Architecture

**Design**: Lightweight actors with message passing

```rust
actor Worker {
    handle request(msg: String) -> String {
        "Response: " + msg
    }
}

w = Worker::spawn()
result = w.request("Hello").await  // Async message passing
```

**Performance**:
- 100K+ concurrent actors on single thread
- <5ms message latency (p50)
- <50ms message latency (p99)

### Future Work: Async/Await (#1 AI Feature)

```killer
async kfn fetch_data(url: String) -> String {
  data = await fetch(url)
  return process(data)
}

// Usage:
result = await fetch_data("https://api.example.com")
```

**Expected Impact**: 100K+ concurrent operations

---

## Error Handling

### Error Hierarchy

```
VmError (toplevel)
├── SecurityError(message, suggestion)  // Path traversal, recursion
├── ParseError(message)                 // Lexer/parser errors
├── RuntimeError(message)               // Division by zero, null deref
└── IoError(message)                    // File read failures
```

### Error Messages with Context

```
Error at line 10, column 5:
  Parse error: unexpected token 'if'
  
  Expected: expression
  Found: keyword 'if'
  
  Suggestion: Did you mean to use a conditional expression?
```

### Recovery Strategies

**Parser Error Recovery**:
- Continue parsing after error
- Collect multiple errors
- Report all at once

**VM Runtime Recovery**:
- Try/catch blocks (user-level)
- Exception manager (internal)
- Graceful degradation on resource limits

---

## Future Roadmap

### Short Term (v1.2 - Q2 2026)
- [ ] Complete native code generation
- [ ] Generational garbage collection
- [ ] Type constraint support (generics)
- [ ] Performance profiling tools

### Medium Term (v2.0 - Q3 2026)
- [ ] Async/await support (100K+ concurrency)
- [ ] LLM integration (native types)
- [ ] Tool calling framework
- [ ] Vector/embedding support
- [ ] Multi-agent coordination

### Long Term (v3.0 - Q4 2026+)
- [ ] GPU support (CUDA, Metal, Vulkan)
- [ ] Foreign function interface (FFI)
- [ ] Distributed execution
- [ ] Advanced memory management
- [ ] Production serverless support

---

## Glossary

| Term | Definition |
|------|-----------|
| **Bytecode** | Intermediate representation between source and native code |
| **JIT** | Just-In-Time compilation: compile code while running |
| **Hot Path** | Code that executes frequently (candidate for optimization) |
| **Tier** | Level of JIT optimization (Tier 0 = lowest, Tier 4 = highest) |
| **RAII** | Resource Acquisition Is Initialization: automatic cleanup via destructors |
| **DAG** | Directed Acyclic Graph: module dependencies with no cycles |
| **Inlining** | Replace function call with function body to eliminate call overhead |
| **SIMD** | Single Instruction Multiple Data: vectorized operations |

---

## Contributing

To add new optimization modules:

1. Create module file: `src/new_optimizer.rs`
2. Implement statistics/configuration traits
3. Add to `OptimizationEngine` (not directly to VM!)
4. Update tuning guide and benchmarks
5. Document performance impact

Example: Adding a cache optimizer
```rust
// src/cache_optimizer.rs
pub struct CacheOptimizer {
    hits: u64,
    misses: u64,
}

impl CacheOptimizer {
    pub fn statistics(&self) -> CacheStats {
        CacheStats {
            hit_rate: self.hits as f64 / (self.hits + self.misses) as f64,
        }
    }
}
```

---

**Last Updated**: March 21, 2026  
**Maintainers**: Killer Language Team
