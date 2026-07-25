# Killer Dependent Types Design
## Phase 1 Weeks 1-6

### Overview
Dependent types are types that depend on values, enabling compile-time verification of:
- **Array bounds**: `Vector[10]` proves length at compile time
- **Matrix dimensions**: `Matrix[3][4]` proves shape in type system
- **Type invariants**: Church encoding of constraints
- **Program correctness**: Impossible programs are unrepresentable

**Power Example:**
```killer
// Traditional: length checked at runtime, can fail
fn safe_access(v: [i32], i: i32) -> i32 {
    if i >= 0 && i < v.len() {
        v[i]
    } else {
        panic("out of bounds")
    }
}

// With dependent types: impossible to write unsafe code
fn safe_access(v: Vector[n], i: Idx[n]) -> i32 {
    v[i]  // Compiler proves i < n always holds
}
```

---

## Core Concepts

### Dependent Type Syntax

```killer
// Size parameter (nat = natural number)
type Vector[n: nat] {
    data: i32[],
    len: n,  // proof that len == n
}

// Multiple parameters
type Matrix[m: nat, n: nat] {
    data: f64[],
    rows: m,
    cols: n,
}

// Refinement: property about value
type Sorted[n: nat] = Vector[n] with (
    forall i. forall j. (i < j) implies (data[i] <= data[j])
)

// Index into n-element array
type Idx[n: nat] = i32 with (0 <= value && value < n)
```

### Type-Level Arithmetic

```killer
// Vector concatenation increases size
fn concat[n: nat, m: nat](
    v1: Vector[n], 
    v2: Vector[m]
) -> Vector[n + m] {
    // Implementation proves result has size n + m
}

// Matrix transpose swaps dimensions
fn transpose[m: nat, n: nat](
    a: Matrix[m][n]
) -> Matrix[n][m] {
    // Compiler tracks dimension swap
}

// Known-size loop
fn fill[n: nat](value: i32) -> Vector[n] pure {
    let mut result = Vector::new();
    for i in 0..n {
        result.push(value);  // i is Idx[n]
    }
    result  // Proven to have size n
}
```

### Refinement Types

```killer
// Positive integer
type Positive = i32 with (value > 0)

// Even number
type Even = i32 with (value % 2 == 0)

// Non-empty list
type NonEmpty[T] = [T] with (len(self) > 0)

// Function on positive integers only
fn factorial(n: Positive) -> Positive {
    if n == 1 { 1 } else { n * factorial(n - 1) }
}
```

---

## Implementation Roadmap

### Week 1-2: Parser & AST
**Goal**: Parse dependent type syntax

```killer
// Examples to parse:
type Vector[n: nat] { ... }
fn foo(v: Vector[10]) -> i32 { ... }
fn bar[T](v: Vector[T]) -> Vector[T] { ... }
```

Tasks:
- [ ] Extend parser to handle `Type[param: constraint]` syntax
- [ ] Add `DependentParam` to AST: name + constraint
- [ ] Store dependent params in type definitions
- [ ] Parse type-level arithmetic: `Vector[n + 1]`, `Matrix[m][n]`

### Week 3-4: Kind Checking
**Goal**: Validate type-level expressions

```
Check that:
- n and m are nats
- n + m is nat
- constraints are boolean expressions
```

Tasks:
- [ ] Implement kind system (similar to Haskell types)
- [ ] Kind inference for type parameters
- [ ] Constraint validation (only nat, Positive, etc. allowed)
- [ ] Type checking of size expressions

### Week 5-6: Constraint Solving
**Goal**: Prove dependent type constraints at compile time

```killer
fn safe_access[n: nat](v: Vector[n], i: Idx[n]) -> i32 {
    v[i]  // Constraint solver verifies: i < n
}

fn concat[n: nat, m: nat](v1: Vector[n], v2: Vector[m]) -> Vector[n + m] {
    // Solver must verify: result.len() == n + m
}
```

Tasks:
- [ ] SMT solver integration (Z3 or similar) for constraint solving
- [ ] Prove equality: `n + m == m + n`
- [ ] Prove ordering: `n < n + 1`
- [ ] Function contract enforcement
- [ ] Error messages when constraints can't be satisfied

---

## Design Decisions

### Why Dependent Types?
1. **Correctness by design**: Impossible programs unrepresentable
2. **Zero-cost abstraction**: Erased at runtime
3. **Type-safe indexing**: Bounds checking in type system, not runtime
4. **Verification**: Formal properties proven automatically
5. **Developer experience**: Compiler provides much better error messages

### Levels of Sophistication (Progressive)

**Level 1 (Week 1-2): Size-aware types**
```killer
type Vector[n: nat] { data: i32[], len: n }
fn access(v: Vector[n], i: Idx[n]) -> i32 { v[i] }
```

**Level 2 (Week 3-4): Arithmetic in types**
```killer
type Matrix[m: nat, n: nat] { ... }
fn matmul[m][n][p](a: Matrix[m][n], b: Matrix[n][p]) -> Matrix[m][p]
```

**Level 3 (Week 5-6): Refinements + constraints**
```killer
type Sorted[n: nat] = Vector[n] with (is_sorted)
fn merge[n][m](v: Sorted[n], w: Sorted[m]) -> Sorted[n + m]
```

### Alternative Approaches Rejected
- **Gradual typing** (TypeScript): Too lenient for Killer's goals
- **Flow-sensitive types** (Kotlin): Runtime, not compile-time verification
- **Linear types** (Rust borrow checking): Orthogonal feature, can combine
- **Full dependent types** (Idris): Too slow for Killer's compilation speed goals

---

## Examples

### Example 1: Safe Vector Access
```killer
// Type-level proof that index is valid
type Idx[n: nat] = i32 with (0 <= value && value < n)

fn safe_get[n: nat](v: Vector[n], i: Idx[n]) -> i32 pure {
    v[i]  // No bounds check needed - proven at compile time
}

// Compiler error: Idx[10] doesn't include 10
fn bad_access() {
    let v = Vector[10] { ... };
    safe_get(v, 10)  // ERROR: 10 is not Idx[10]
}

// OK: 5 is proven Idx[10]
fn good_access() {
    let v = Vector[10] { ... };
    safe_get(v, 5)  // OK: 5 < 10 proven
}
```

### Example 2: Type-Safe Matrix Operations
```killer
type Matrix[m: nat, n: nat] {
    data: f64[],
    rows: m,
    cols: n,
}

// Matrix multiplication requires compatible dimensions
fn matmul[m: nat, n: nat, p: nat](
    a: Matrix[m][n],
    b: Matrix[n][p]
) -> Matrix[m][p] pure {
    // Result guaranteed to be m × p
    // Type system prevents m×n * p×q for n != p
}

// Compiler error: dimensions don't match
fn bad_matmul() {
    let a = Matrix[3][4] { ... };
    let b = Matrix[5][6] { ... };
    matmul(a, b)  // ERROR: n=4 doesn't equal 5
}

// OK: dimensions align
fn good_matmul() {
    let a = Matrix[3][4] { ... };
    let b = Matrix[4][5] { ... };
    matmul(a, b)  // Result is Matrix[3][5]
}
```

### Example 3: Non-Empty List Operations
```killer
type NonEmpty[T, n: nat] = [T] with (n > 0)

fn head[T, n: nat](list: NonEmpty[T, n]) -> T pure {
    list[0]  // Safe - proven non-empty
}

fn tail[T, n: nat](list: NonEmpty[T, n]) -> [T] pure {
    if n == 1 { [] } else { list[1 ..] }
}

fn maximum[n: nat](list: NonEmpty[i32, n]) -> i32 pure {
    // Cannot write: reduce((a, b) => max(a, b))
    // Proven non-empty, reduction always valid
}
```

### Example 4: Dependent Types in Functions
```killer
// Dependent type in parameter
fn repeat[T, n: nat](x: T, count: Positive) -> Vector[count] pure {
    let mut result = Vector::new();
    for i in 0..count {
        result.push(x);
    }
    result  // Proven size = count
}

// Verify length at compile time
fn triple[T](x: T) -> Vector[3] pure {
    repeat(x, 3)  // Compiler proves result size = 3
}

// Type-safe concatenation
fn concat[T, m: nat, n: nat](
    v1: Vector[m],
    v2: Vector[n]
) -> Vector[m + n] pure {
    // Result proven to have size m + n
}
```

---

## Testing Strategy

### Compile-time Tests (Goal: pass 30+ cases)
```
✓ Vector[n] with valid size
✗ Vector[n] with invalid index access
✓ Matrix[m][n] multiplication with valid dimensions
✗ Matrix[m][n] multiplication with invalid dimensions
✓ Type-level arithmetic: Vector[3] + Vector[5] = Vector[8]
✗ Type mismatch: Vector[3] + Vector[5] = Vector[8] should fail if proof wrong
```

### Runtime Tests
```
✓ Vector operations perform as expected
✓ Index access doesn't use runtime checks
✓ Performance identical to non-dependent version
```

---

## Files to Create

```
docs/phase1/dependent_types/
  ├── DESIGN.md (this file)
  ├── IMPLEMENTATION.md
  ├── CONSTRAINT_SOLVER.md
  └── examples/
      ├── vectors_basic.killer
      ├── matrices.killer
      ├── refined_types.killer
      ├── arithmetic.killer
      └── error_cases.killer

src/v2-rust/killer_vm/src/
  ├── dependent_types.rs
  ├── constraint_solver.rs
  └── kind_checker.rs
```

---

## Success Criteria

✓ Parser handles dependent type syntax
✓ Kind system validates type-level expressions
✓ Constraint solver proves most common cases
✓ Bounds-checked access compiles without runtime checks
✓ Type-level arithmetic works (Vector[n+m], Matrix[m][n])
✓ Error messages explain type mismatches clearly
✓ Tests pass: 30+ dependent type examples

---

## Integration with Other Features

### With Effect System
```killer
fn read_n_items[n: nat](file: String) -> Vector[n] uses io {
    // Returns vector of exactly n items
}
```

### With Contracts
```killer
fn sort[n: nat](arr: Vector[n]) -> Sorted[n] pure
    requires true
    ensures is_sorted(result)
```

### With Async/Await
```killer
async fn fetch_n[n: nat](urls: Vector[n]) -> Vector[String] uses io {
    // Returns exactly n responses
}
```

---

## Performance Impact

| Aspect | Impact | Notes |
|--------|--------|-------|
| Compile time | +20-30% | Constraint solving overhead |
| Binary size | 0% | Dependent types erased |
| Runtime | 0% | Just syntax for compiler |
| Optimization | +5-10% | Bounds checks can be eliminated |

---

## Next Steps After Week 6

Phase 2 starts (Week 7): Effect System
- Dependent types + effects together
- Type-safe I/O patterns
