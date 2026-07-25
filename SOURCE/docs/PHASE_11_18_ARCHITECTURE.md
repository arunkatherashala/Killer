# Phase 11-18 Architecture Plan: Operators, Ghost, & Assassin Layers

## Phase 11: Quality Operators (1 week)

### 11.1 Quality Arithmetic Operators
**Specification:**
```killer
quality q1 = 85
quality q2 = 90
quality q3 = q1 + q2  // Combines scores: weighted average
let score = q3.quality()  // Returns ~87.5

quality sum = q1 + q2  // Quality + Quality = Quality (avg scores)
let total = 90 + 50    // Number + Number = Number (normal math)
let mixed = q1 + 90    // Quality + Number = Number (auto-unwrap) or Quality (smart coercion)
```

**Implementation Strategy:**
1. **Binary Operations on Quality** (vm.rs)
   - Extend `BinaryOp` instruction to handle `Value::QualityWrapped`
   - Implement: `+`, `-`, `*`, `/` for quality values
   - Arithmetic strategy: combine scores via weighted average or rank arithmetic

2. **Type Coercion Rules**
   ```
   Quality + Quality      → Quality (weighted average of scores)
   Quality + Number       → Number (auto-unwrap quality score)
   Quality + String       → Error (type mismatch)
   Number + Quality       → Number (auto-unwrap to right)
   ```

3. **Quality Score Combination Algorithm**
   ```rust
   fn combine_quality_scores(q1: f64, q2: f64, op: BinaryOpType) -> f64 {
       match op {
           Add => (q1 + q2) / 2.0,           // Average
           Subtract => (q1 - q2).abs(),      // Difference magnitude
           Multiply => q1 * q2,              // Product  
           Divide => q1 / q2.max(0.001),     // Safe divide
       }
   }
   ```

### 11.2 Quality Comparison Operators

**Specification:**
```killer
quality email = "alice@example.com"
quality.validate_email()

quality threshold = 0.85
if email > threshold:     // Compare quality scores
    print "Email is good"

if email == threshold:    // Equality of scores
    print "Perfect match"
```

**Implementation Strategy:**
1. **Comparison Operations** (vm.rs)
   - Extend `Compare` instruction to handle Quality types
   - Operators: `<`, `>`, `<=`, `>=`, `==`, `!=`
   - Comparison is always done on quality scores (0.0-1.0)

2. **Comparison Algorithm**
   ```rust
   fn compare_quality(q1_score: f64, q2_value: &Value, op: CompareOp) -> Result<bool> {
       let q2_score = match q2_value {
           Value::Number(n) => *n,
           Value::QualityWrapped(q) => q.quality(),
           _ => return Err("Cannot compare quality with non-numeric type"),
       };
       
       match op {
           GreaterThan => Ok(q1_score > q2_score),
           LessThan => Ok(q1_score < q2_score),
           Equals => Ok((q1_score - q2_score).abs() < 0.0001),
           ...
       }
   }
   ```

### 11.3 Compiler Changes for Phase 11

**Parser Enhancement** (parser.rs)
- No new syntax needed; existing `BinaryOp` and `Compare` expressions already work
- Compiler will generate appropriate instructions for quality operands

**Compiler Changes** (compiler.rs)
- Detect `Value::QualityWrapped` operands in `compile_binary_op()`
- Generate appropriate type coercion instructions if needed
- For Quality+Number, generate `UNWRAP_QUALITY` then normal arithmetic

**New Instructions** (if needed)
```rust
enum Instruction {
    // Existing...
    UnwrapQuality,  // Stack: Quality → Number (converts Q.quality() value)
    // Or handle inline in existing instructions
}
```

### 11.4 Testing Plan for Phase 11
- Test files: `test_phase11_arithmetic.killer`, `test_phase11_compare.killer`
- Verify: `q1+q2`, `q1-q2`, `q1*q2`, `q1/q2`
- Verify: `q1>q2`, `q1==q2`, `q1<<q2`, etc.
- Type coercion: `quality + 5`, `10 - quality`
- Chain operations: `(q1 + q2) > 0.8`

---

## Phase 12-15: Multi-Method Dispatch & Advanced Features (4 weeks)

### Phase 12: Operator Overloading for User Types
- Classes can define `__add__`, `__sub__`, `__eq__` etc.
- Operators call these methods dynamically
- Method resolution: class hierarchy walk

### Phase 13: Generic Type Parameters  
- `Array<T>`, `Dict<K, V>`, `Quality<T>`
- Compile-time type checking
- Specialization for common types

### Phase 14: Pattern Matching & Destructuring
- `let {name, age} = person`
- `match value { pattern => ...}`
- Guard clauses: `if value > 10 =>`

### Phase 15: Async/Await & Generators
- `async fn fetch(url) { ... }`
- `yield value` in generators
- Promise/Future types

---

## Phase 16-18: Ghost Layer - Optimization & Specialization (6 weeks)

### Phase 16: Type Specialization Engine

**Goal:** JIT-compile hot code paths with specialized types

**Architecture:**
```
┌─────────────────────────────────────────┐
│       Killer Program (Interpreted)       │
└──────────────┬──────────────────────────┘
               │
        ┌──────▼──────┐
        │ Hot Path     │
        │ Detector     │
        │ (500 iters)  │
        └──────┬──────┘
               │
        ┌──────▼────────────────┐
        │ Type Analysis         │
        │ - Infer Value Types   │
        │ - Specialize Vars     │
        │ - Cache decisions     │
        └──────┬────────────────┘
               │
        ┌──────▼──────────────────┐
        │ Code Generation       │
        │ - LLVM IR or x86-64   │
        │ - Inline caches       │
        │ - Direct calls        │
        └──────┬──────────────────┘
               │
        ┌──────▼──────────────────────┐
        │ Native Code Execution       │
        │ (with fallback to VM)       │
        └─────────────────────────────┘
```

**Type Inference System:**
```rust
// For loop: `for i in 1..1000: x = x + i`
// After 500 iterations, mark as hot
// Infer: x is Number, i is Number
// Specialize: generate i64 add instruction
// Speedup: 10-50x for numeric loops

#[derive(Clone)]
struct TypeSpecialization {
    variable_types: HashMap<String, InferredType>,
    operation_types: HashMap<String, Vec<OccurringType>>,
    confidence: f64,  // 0.0-1.0
}

enum InferredType {
    AlwaysNumber,
    AlwaysString,
    AlwaysArray,
    AlwaysQuality,
    Polymorphic(Vec<(Type, Count)>),  // Tracks what types appear
}
```

**Implementation:**
1. Profile each variable assignment
2. Track inferred types (usually within first 10 iterations)
3. Generate LLVM IR for specialized hot loop
4. Swap in native code with guard checks
5. Fall back to VM if type assumptions violated

### Phase 17: Result Caching & Memoization

**Goal:** Cache computation results for deterministic functions

**Features:**
```killer
// @memoize marks function as cacheable
@memoize
fn fibonacci(n) {
    if n <= 1: return n
    return fibonacci(n-1) + fibonacci(n-2)
}

// First call: fib(40) takes 2 seconds (computed)
let a = fibonacci(40)
// Second call: fib(40) instant (cached)  
let b = fibonacci(40)

// Fibonacci with memoization: 2^N time → O(N) time!
```

**Implementation:**
```rust
struct MemoizationSystem {
    cache: HashMap<(String, Vec<Value>), Value>,
    hits: u64,
    misses: u64,
}

impl MemoizationSystem {
    fn call_memoized(&mut self, fn_name: &str, args: Vec<Value>) -> Value {
        let key = (fn_name.to_string(), args.clone());
        
        if let Some(cached) = self.cache.get(&key) {
            self.hits += 1;
            return cached.clone();
        }
        
        // Compute normally (execute function)
        let result = execute_function(fn_name, args.clone());
        self.cache.insert(key, result.clone());
        self.misses += 1;
        result
    }
}
```

### Phase 18: Adaptive Compilation Strategy

**Goal:** Pick best execution strategy per function

**Strategy Selection:**
```
1. First call: Quick VM execution + profiling
2. If called <100x: Interpret only
3. If called 100-10000x: Baseline JIT (simpler codegen)
4. If called 10000+x and hot: Full type specialization + optimization

// Example: 
// - `main()`: called 1x → interpret
// - `helper()`: called 100x → baseline JIT  
// - inner arithmetic loop: called 1M+ → full specialization
```

**Compilation Pipeline:**
```
┌────────────────────┐
│ Function Called    │
├────────────────────┤
│ Check call count   │
└─────────┬──────────┘
          │
     ┌────▼────┐
     │<100?    │
     └─┬──────┬┘
       │      │No
       │Yes   │
       │      │  ┌──────────────────┐
       │      │  │ Baseline JIT      │
       │      └─►│ Generate simpler  │
       │         │ native code       │
       │         └─────┬────────────┘
       │               │
      ┌┴───────────────┴──┐
      │   Call count>    │
      │   100,000?       │
      │ and is hot?      │
      └────┬────────────┬┘
           │            │
          Yes           │No
           │            │
    ┌──────▼────────┐   │
    │Full Type      │   │
    │Specialization │   │
    │+ Optimization │   │
    └──────┬────────┘   │
           │            │
      ┌────▼────────────┴──────┐
      │ Execute Native Code    │
      │ (with VM fallback)     │
      └───────────────────────┘
```

---

## Assassin Layer - Security & Isolation (8 weeks)

### Phase 19: Process Isolation (seccomp + chroot)

**Goal:** Run untrusted Killer code in empty sandbox

**Architecture:**
```
┌──────────────────────────────────┐
│    Host System                   │
│  · Filesystem (full access)      │
│  · Network (no restrictions)     │
│  · System calls (all allowed)    │
└─────────────┬────────────────────┘
              │
              │ execve() with seccomp + chroot
              │
    ┌─────────▼──────────────┐
    │ Sandbox Environment    │
    │ · Filesystem: /tmp     │
    │ · At most 1000 files   │
    │ · No network access    │
    │ · 64MB memory limit    │
    │ · 5 second timeout     │
    └────────┬───────────────┘
             │
    ┌────────▼──────────────┐
    │ Killer VM             │
    │ (untrusted code)      │
    └───────────────────────┘
```

**Implementation:**
1. **chroot / namespace switch**
   ```rust
   fn setup_sandbox(chroot_path: &str) -> Result<()> {
       unsafe {
           // Create temporary root filesystem in /tmp/killer_sandbox_XXXXX
           let sandbox_root = create_temp_dir("/tmp/killer_sandbox_")?;
           
           // Switch to new root - child process can't escape
           libc::chroot(sandbox_root.as_ptr() as *const i8)?;
           std::env::set_current_dir("/")?;
       }
       Ok(())
   }
   ```

2. **seccomp: Filter forbidden syscalls**
   ```rust
   fn setup_seccomp() -> Result<()> {
       let mut ctx = SeccompContext::new()?;
       
       // Allow only safe syscalls
       ctx.allow_syscall("read")?;
       ctx.allow_syscall("write")?;
       ctx.allow_syscall("mmap")?;
       
       // Block dangerous syscalls
       ctx.block_syscall("execve")?;     // No spawning processes
       ctx.block_syscall("open")?;       // No opening files (use fd only)
       ctx.block_syscall("socket")?;    // No network
       ctx.block_syscall("ptrace")?;    // No debugging
       
       ctx.load()?;  // Apply to current process
   }
   ```

### Phase 20: Resource Limits (cgroups)

**Goal:** Enforce memory, CPU, and I/O quotas

**Features:**
```bash
# Killer sandbox with limits
killer-run --memory=256MB --cpu=30% --timeout=5s untrusted.killer

# Specification:
# · Max 256MB memory (OOM killer stops process)
# · Max 30% CPU (throttled when exceeded)
# · Max 5 seconds runtime (SIGKILL if exceeded)
# · Max 1000 files (EMFILE error)
#  · Max 1MB/s write speed (disk throttle limit)
```

**Implementation:**
```rust
struct SandboxResourceLimits {
    max_memory: u64,          // Bytes
    max_cpu_percent: f64,     // 0-100%
    max_runtime_secs: u64,
    max_open_files: u32,
    max_write_bps: u64,       // Bytes/second
}

fn apply_cgroup_limits(cgroup_name: &str, limits: SandboxResourceLimits) -> Result<()> {
    // Write to cgroup v2 interface:
    // /sys/fs/cgroup/unified/killer_XXXXX/
    
    write_cgroup_file("memory.max", limits.max_memory.to_string())?;
    write_cgroup_file("cpu.max", format!("{} 100000", (limits.max_cpu_percent * 1000.0) as u64))?;
    
    // Timeout via signal-based approach
    set_timeout(limits.max_runtime_secs)?;
}
```

### Phase 21: Syscall Monitoring & Logged Execution

**Goal:** Audit all system calls made by untrusted code

**Syscall Logging:**
```json
{
  "execution_id": "sandbox_20260313_142530_abc123",
  "program": "untrusted.killer",
  "timestamp": "2026-03-13T14:25:30Z",
  "syscalls": [
    {
      "time_ns": 1000000,
      "syscall": "write",
      "args": { "fd":  1, "buf_len": 42 },
      "result": 42,
      "allowed": true
    },
    {
      "time_ns": 2000000,
      "syscall": "execve",
"args": { "path": "/bin/bash" },
      "result": -1,
      "error": "EPERM",
      "allowed": false,
      "blocked_by": "seccomp policy"
    }
  ],
  "summary": {
    "total_syscalls": 1024,
    "blocked_syscalls": 3,
    "exit_code": 0,
    "runtime_ms": 450
  }
}
```

**Implementation:**
```rust
struct SyscallAuditor {
    log_file: File,
    syscall_counts: HashMap<String, u64>,
}

impl SyscallAuditor {
    fn log_syscall(&mut self, syscall: &SyscallInfo) -> Result<()> {
        // Use ptrace() to intercept syscalls from child process
        // OR ebpf hooks for kernel-level syscall tracing
        
        writeln!(self.log_file, "{}", serde_json::to_string(syscall)?)?;
        *self.syscall_counts.entry(syscall.name.clone()).or_insert(0) += 1;
        Ok(())
    }
}
```

---

## Summary Timeline

| Phase | Duration | Focus | Key Tech |
|-------|----------|-------|----------|
| 10 | 1 week | Quality methods | Method dispatch |
| 11 | 1 week | Operators | BinaryOp, Compare |
| 12-15 | 4 weeks | Advanced features | Generics, pattern match |
| **Ghost Layer** ||||
| 16 | 2 weeks | Type specialization | JIT, LLVM IR |
| 17 | 2 weeks | Memoization | Function caching |
| 18 | 2 weeks | Adaptive compilation | Strategy selection |
| **Assassin Layer** ||||
| 19 | 3 weeks | Process isolation | seccomp, chroot |
| 20 | 2 weeks | Resource limits | cgroups |
| 21 | 3 weeks | Syscall monitoring | ptrace/eBPF |

**Total Remaining: 30 weeks (7-8 months) for full implementation**

---

## Architecture Principles

1. **Backwards Compatibility:** All new phases preserve Phase 1-10 functionality
2. **Opt-in Features:** Ghost and Assassin are optional; code runs without them
3. **Graceful Degradation:** If JIT fails, fall back to VM interpretation
4. **Security-First:** Assassin layer employs defense-in-depth (multiple isolation levels)
5. **Observable:** All layers expose metrics (call counts, cache hits, syscall logs)
