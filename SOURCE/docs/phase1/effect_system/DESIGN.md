# Killer Effect System Design
## Phase 1 Week 7-11

### Overview
An effect system explicitly tracks which side effects a function can perform, enabling:
- **Type-safe I/O**: Distinguish pure functions from functions that perform I/O
- **Parallelization safety**: Automatically parallelize functions with no shared effects
- **Resource control**: Track memory allocation, file operations, network access
- **Determinism verification**: Prove functions are deterministic (no random effects)

---

## Core Concepts

### Effect Definition
Effects represent side effect capabilities:

| Effect | Meaning | Examples |
|--------|---------|----------|
| `io` | File/console I/O | read, write, print |
| `allocate` | Memory allocation | array creation, object new |
| `random` | Non-deterministic | random numbers, timestamps |
| `panic` | Can throw/error | division by zero, unwrap |
| `network` | Network access | HTTP, sockets |

### Function Signatures

**Pure function (no effects):**
```killer
fn add(x: i32, y: i32) -> i32 pure {
    x + y  // Can run anywhere, anytime
}
```

**With I/O effect:**
```killer
fn read_file(path: String) -> String uses io {
    // Performs file I/O
}
```

**With multiple effects:**
```killer
fn process() -> Result uses (io, random, allocate) {
    // Can do I/O, generate random numbers, allocate memory
}
```

**Effect combining (caller gets union of effects):**
```killer
fn pipeline(data: String) -> i32 uses io {
    // Even though read_file uses io, caller must declare it
    let content = read_file(data);
    
    // If this also used random, this function would need: uses (io, random)
    len(content)
}
```

---

## Implementation Plan

### Phase 1 (Weeks 7-8): Parser Support
- [ ] Add `pure` keyword to function signatures
- [ ] Add `uses` keyword with effect list `uses (io, random, allocate)`
- [ ] Parse effect annotations in function definitions
- [ ] Store effects in function AST node

**Example syntax to parse:**
```killer
fn foo() -> i32 pure { ... }
fn bar() -> String uses io { ... }
fn baz() -> Result uses (io, allocate) { ... }
```

### Phase 2 (Weeks 9-10): Type Checking
- [ ] Track effects in function context
- [ ] Check effect consistency when calling functions
- [ ] Enforce: calling a `uses io` function requires caller to also `uses io`
- [ ] Error on effect leaks (pure function calling impure)

**Type checking rules:**
```
Rule 1: pure -> pure ✓ (allowed)
Rule 2: pure -> !pure ✗ (error)
Rule 3: uses io -> uses io ✓
Rule 4: uses io -> pure ✗ (error)
Rule 5: uses (io, random) -> uses io ✗ (need both effects)
```

### Phase 3 (Week 11): Effect Inference + Optimization
- [ ] Auto-infer effects for simple functions
- [ ] Optimize: deduplicate multiple instances of same effect
- [ ] Generate error messages for effect violations
- [ ] Add `#[must_check_effects]` lint

---

## Design Decisions

### Why Explicit Effects?
1. **Readability**: Function type signature shows exactly what it does
2. **Parallelization**: Functions with disjoint effects can run in parallel safely
3. **Testing**: Mock pure functions; test effects separately
4. **Performance**: Compiler can optimize knowing what effects occur

### Why Not Algebraic Effects?
- Simpler mental model for most users
- Faster compilation
- Works with existing Rust/LLVM infrastructure
- Can add algebraic effects in future Phase

### Effect Subtyping
Pure is a subtype of all effects (can use pure wherever any effect is expected):
```
pure ⊆ {io} ⊆ {io, random} ⊆ ...
```

---

## Examples

### Example 1: Pure Computation
```killer
fn factorial(n: i32) -> i32 pure {
    if n <= 1 { 1 } else { n * factorial(n - 1) }
}

// OK - pure can call pure
fn factorial_safe(n: i32) -> i32 pure {
    factorial(n)
}

// ERROR - factorial is pure, but trying to use in random context
fn random_factorial() -> i32 uses random {
    factorial(5)  // Compiler error: pure ⊄ random
}
```

### Example 2: Effect Propagation
```killer
fn log_message(msg: String) -> void uses io {
    println(msg)
}

// Caller must declare io effect
fn process_data(data: String) -> void uses io {
    log_message("Processing: " + data)
}

// ERROR - didn't declare io
fn bad_process(data: String) -> void {
    log_message("Processing: " + data)  // Compiler error
}
```

### Example 3: Multiple Effects
```killer
fn fetch_and_shuffle(url: String) -> [i32] uses (io, random) {
    let response = fetch(url);  // uses io
    shuffle(parse(response))     // uses random
}

// OK - has both io and random
fn pipeline() -> [i32] uses (io, random) {
    fetch_and_shuffle("https://example.com")
}

// ERROR - only has io, but needs random too
fn bad_pipeline() -> [i32] uses io {
    fetch_and_shuffle("https://example.com")  // Compiler error
}

// OK - pure can call anything with explicit effects
fn safe_sort(arr: [i32]) -> [i32] pure {
    // Can't call fetch_and_shuffle since it has effects
    bubble_sort(arr)  // OK - bubble_sort is pure
}
```

### Example 4: Effect Driven Optimization
```killer
// Compiler optimizes: parallel safe (no shared effects)
async fn process_batch(items: [String]) -> [i32] uses io {
    items.map(fn(item) -> i32 uses io {
        // Each call is independent io
        fetch(item).len()
    })
}

// Compiler error: can't parallelize (shared random state)
async fn randomize_batch(items: [i32]) -> [i32] uses random {
    items.map(fn(item) -> i32 uses random {
        item + random()  // Shared randomness breaks parallelization
    })
}
```

---

## Performance Implications

| Scenario | Optimization |
|----------|------------|
| Pure function | No I/O overhead, can inline aggressively |
| Effect separated | Lazy I/O, can defer operations |
| Known effects | Specialize code path (e.g., buffered I/O) |
| Effect mismatch | Compile error (no runtime cost) |

---

## Integration with Other Phase 1 Features

### With Dependent Types
```killer
fn safe_read(path: String, max_size: nat) -> String uses io
    requires max_size > 0
    ensures result.len() <= max_size
{
    // ...
}
```

### With Contracts
```killer
fn allocate(size: nat) -> Array uses allocate
    requires size > 0
    ensures result.len() == size
{
    // ...
}
```

### With Async/Await
```killer
async fn download(url: String) -> String uses io {
    // Can be awaited from async context
    fetch(url).await
}
```

---

## Files to Create This Week

```
docs/phase1/effect_system/
  ├── DESIGN.md (this file)
  ├── IMPLEMENTATION.md (roadmap details)
  └── examples/
      ├── pure_functions.killer
      ├── io_effects.killer
      ├── multi_effects.killer
      └── error_cases.killer
```

---

## Success Criteria

✓ Parser recognizes `pure` and `uses` keywords
✓ Effect annotations stored in AST
✓ Type checker validates effect consistency
✓ Error messages guide users on effect violations
✓ Tests pass: 20+ effect test cases
✓ Documentation: Examples + design rationale

---

## Next Phase After Week 11

- Phase 2: Async/Await (Weeks 12-15)
- Effect system will integrate with async tasks
- Each async task tracks its effects
