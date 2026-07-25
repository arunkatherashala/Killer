# Phase 21: Standard Library Implementation Guide
## Killer Standard Library - 220+ Functions (4 weeks)

**Status:** Framework Complete | Implementation Pending  
**File:** `_TOOLS/killer_rcore/src/stdlib_builder.rs` (1600+ lines)  
**Tests:** `_TOOLS/killer_rcore/tests/test_phase21_stdlib.rs` (400+ lines)  
**Timeline:** Weeks 21-24 (4 weeks total)  

---

## 1. Overview

### 1.1 Standard Library Architecture

Phase 21 introduces a comprehensive standard library with **220+ functions** organized into **7 categories**:

| Category | Count | Complexity | Timeline |
|----------|-------|-----------|----------|
| Math | 80 | O(1) to O(n) | Week 21 |
| String | 60 | O(n) to O(n*m) | Week 21 |
| Collections | 50 | O(1) to O(n log n) | Week 22 |
| I/O | 10 | O(n) | Week 22 |
| Time | 4 | O(1) to O(ms) | Week 23 |
| Type | 4 | O(1) | Week 23 |
| Concurrency | 3 | Var | Week 24 |
| **Total** | **211** | - | **4 weeks** |

### 1.2 Integration with FFI (Phase 20)

Many stdlib functions leverage FFI for C library integration:

```rust
// Example: sqrt using C FFI
fn sqrt(x: Float) -> Float {
    ffi::call_math_function("sqrt", vec![ffi::CValue::Float(x)])
}

// Example: strlen using C FFI
fn strlen(s: String) -> Int {
    ffi::call_string_function("strlen", vec![ffi::CValue::CStr(s)])
}
```

### 1.3 Performance Impact

- **Expected speedup:** 10-50x on critical paths (math, string operations)
- **Latency:** < 1ms for most operations
- **Memory overhead:** Minimal (functions are stateless)

---

## 2. Math Library (80 functions)

### 2.1 Trigonometric Functions (10)

**Core Functions:**
- `sin(x: Float) -> Float` - Sine using C::sin
- `cos(x: Float) -> Float` - Cosine using C::cos
- `tan(x: Float) -> Float` - Tangent using C::tan
- `asin(x: Float) -> Float` - Inverse sine using C::asin
- `acos(x: Float) -> Float` - Inverse cosine using C::acos
- `atan(x: Float) -> Float` - Inverse tangent using C::atan
- `sinh(x: Float) -> Float` - Hyperbolic sine using C::sinh
- `cosh(x: Float) -> Float` - Hyperbolic cosine using C::cosh
- `tanh(x: Float) -> Float` - Hyperbolic tangent using C::tanh
- `atan2(y: Float, x: Float) -> Float` - Two-argument arctangent using C::atan2

**Implementation Strategy:**
```rust
// 1. FFI binding to C math library
// 2. Parameter validation (-π to π for angles)
// 3. Error handling (domain errors for asin/acos)
// 4. Unit tests: verify against known values (sin(π/2) = 1.0)
```

**Tests Required:**
- Boundary values (0, π/2, π, ±Infinity)
- Domain error handling (asin(2.0) should error)
- Accuracy verification (sin(π/6) ≈ 0.5)
- Performance benchmark (1M calls/sec)

### 2.2 Exponential/Logarithmic Functions (10)

**Core Functions:**
- `exp(x: Float) -> Float` - Exponential (e^x)
- `log(x: Float) -> Float` - Natural logarithm
- `log10(x: Float) -> Float` - Base-10 logarithm
- `log2(x: Float) -> Float` - Base-2 logarithm
- `pow(base: Float, exp: Float) -> Float` - Power function
- `sqrt(x: Float) -> Float` - Square root
- `cbrt(x: Float) -> Float` - Cube root
- `hypot(x: Float, y: Float) -> Float` - Hypotenuse
- `expm1(x: Float) -> Float` - e^x - 1 (accurate for small x)
- `log1p(x: Float) -> Float` - log(1 + x) (accurate for small x)

**Implementation Challenge:** Handle edge cases (negative domain, infinity)

### 2.3 Rounding Functions (10)

**Core Functions:**
- `abs(x: Int) -> Int` / `fabs(x: Float) -> Float` - Absolute value
- `ceil(x: Float) -> Float` - Round up
- `floor(x: Float) -> Float` - Round down
- `round(x: Float) -> Float` - Round to nearest
- `trunc(x: Float) -> Float` - Truncate decimal
- `fmod(x: Float, y: Float) -> Float` - Floating-point modulo
- `remainder(x: Float, y: Float) -> Float` - IEEE remainder
- `sign(x: Float) -> Int` - Return -1, 0, or 1
- `copysign(x: Float, y: Float) -> Float` - Copy sign from y to x

**Tests:** Verify rounding behavior matches IEEE 754 standard

### 2.4 Min/Max/Number Operations (10)

**Core Functions:**
- `min(a: Float, b: Float) -> Float` - Minimum
- `max(a: Float, b: Float) -> Float` - Maximum
- `clamp(x: Float, min: Float, max: Float) -> Float` - Constrain range
- `gcd(a: Int, b: Int) -> Int` - Greatest common divisor
- `lcm(a: Int, b: Int) -> Int` - Least common multiple
- `mod(x: Int, m: Int) -> Int` - Integer modulo
- `rem(x: Int, m: Int) -> Int` - Integer remainder
- `saturating_add(a: Int, b: Int) -> Int` - Add without overflow
- `saturating_sub(a: Int, b: Int) -> Int` - Subtract without underflow
- `saturating_mul(a: Int, b: Int) -> Int` - Multiply without overflow

**Tests:** Verify overflow handling (saturating_add(INT_MAX, 1) = INT_MAX)

### 2.5 Random/Statistical Functions (15)

**Core Functions:**
- `random() -> Float` - Random [0, 1) using MT19937
- `random_int(max: Int) -> Int` - Random [0, max)
- `random_range(min: Int, max: Int) -> Int` - Random in range
- `random_float(min: Float, max: Float) -> Float` - Random float in range
- `randn() -> Float` - Normal distribution (Box-Muller)
- `seed(s: Int) -> Void` - Set random seed
- `mean(list: List<Float>) -> Float` - Calculate mean
- `median(list: List<Float>) -> Float` - Calculate median
- `stddev(list: List<Float>) -> Float` - Standard deviation
- `variance(list: List<Float>) -> Float` - Calculate variance
- `sum(list: List<Float>) -> Float` - Sum all elements
- `product(list: List<Float>) -> Float` - Product all elements
- `min_of(list: List<Float>) -> Float` - Minimum in list
- `max_of(list: List<Float>) -> Float` - Maximum in list
- `percentile(list: List<Float>, p: Float) -> Float` - Calculate percentile

**Implementation Notes:**

**MT19937 (Mersenne Twister) Implementation:**
```rust
using std::cell::RefCell;

thread_local! {
    static MT19937_STATE: RefCell<MT19937> = RefCell::new(MT19937::new(42));
}

fn random() -> Float {
    MT19937_STATE.with(|state| {
        state.borrow_mut().next_float()
    })
}

fn seed(s: Int) {
    MT19937_STATE.with(|state| {
        *state.borrow_mut() = MT19937::new(s as u32);
    })
}
```

**Box-Muller Algorithm for Normal Distribution:**
```rust
fn randn() -> Float {
    let u1 = random();
    let u2 = random();
    let r = (-2.0 * u1.ln()).sqrt();
    let theta = 2.0 * 3.14159265358979 * u2;
    r * theta.cos()
}
```

### 2.6 Special Functions (15)

**Core Functions:**
- `erf(x: Float) -> Float` - Error function
- `erfc(x: Float) -> Float` - Complementary error function
- `tgamma(x: Float) -> Float` - Gamma function
- `lgamma(x: Float) -> Float` - Log gamma function
- `j0(x: Float) -> Float` - Bessel J0
- `j1(x: Float) -> Float` - Bessel J1
- `y0(x: Float) -> Float` - Bessel Y0
- `y1(x: Float) -> Float` - Bessel Y1
- `factorial(n: Int) -> Long` - Factorial
- `combinations(n: Int, k: Int) -> Long` - Combinations nCk
- `permutations(n: Int, k: Int) -> Long` - Permutations nPk
- `is_prime(n: Int) -> Bool` - Primality test
- `gcd_extended(a: Int, b: Int) -> (Int, Int, Int)` - Extended GCD
- `modular_pow(base: Int, exp: Int, mod: Int) -> Int` - Modular exponentiation
- `modular_inverse(a: Int, m: Int) -> Int` - Modular multiplicative inverse

**Implementation Challenge:** Some (Bessel, gamma) require advanced numeric algorithms

---

## 3. String Library (60 functions)

### 3.1 Basic Operations (20)

**Core Functions:**
- `length(s: String) -> Int` - String length (O(1) with caching)
- `concat(a: String, b: String) -> String` - Concatenate (O(n+m))
- `substring(s: String, start: Int, end: Int) -> String` - Extract substring
- `index_of(s: String, sub: String) -> Int` - Find substring
- `last_index_of(s: String, sub: String) -> Int` - Find last position
- `starts_with(s: String, prefix: String) -> Bool` - Check prefix
- `ends_with(s: String, suffix: String) -> Bool` - Check suffix
- `contains(s: String, sub: String) -> Bool` - Check contains
- `replace(s: String, old: String, new: String) -> String` - Replace all
- `replace_first(s: String, old: String, new: String) -> String` - Replace first
- `split(s: String, delim: String) -> List<String>` - Split by delimiter
- `split_limit(s: String, delim: String, limit: Int) -> List<String>` - Split with limit
- `join(list: List<String>, sep: String) -> String` - Join with separator
- `trim(s: String) -> String` - Remove whitespace
- `trim_left(s: String) -> String` - Remove left whitespace
- `trim_right(s: String) -> String` - Remove right whitespace
- `to_upper(s: String) -> String` - Convert to uppercase
- `to_lower(s: String) -> String` - Convert to lowercase
- `reverse(s: String) -> String` - Reverse string
- `repeat(s: String, count: Int) -> String` - Repeat N times

**Implementation Strategy:**

```rust
// 1. Use Rust String for underlying storage
// 2. Implement search using KMP algorithm for O(n+m) substring search
// 3. Use UTF-8 aware operations for Unicode support
// 4. Cache string length when possible

fn index_of(s: String, sub: String) -> Int {
    // KMP search for O(n+m) complexity
    match s.find(&sub) {
        Some(pos) => pos as Int,
        None => -1
    }
}

fn split(s: String, delim: String) -> List<String> {
    s.split(&delim)
        .map(|part| part.to_string())
        .collect()
}
```

### 3.2 Case Operations (5)

- `to_title_case()` - Convert "hello world" → "Hello World"
- `capitalize()` - Convert "hello" → "Hello"
- `decapitalize()` - Convert "Hello" → "hello"
- `camel_case()` - Convert "hello_world" → "helloWorld"
- `snake_case()` - Convert "helloWorld" → "hello_world"

### 3.3 Testing Functions (10)

- `is_empty()` - Check if empty
- `is_blank()` - Check if whitespace only
- `is_numeric()` - Check if all digits
- `is_alpha()` - Check if all letters
- `is_alphanumeric()` - Check if alphanumeric
- `equals_ignore_case()` - Case-insensitive compare
- `compare_to()` - Lexicographic compare
- `compare_to_ignore_case()` - Case-insensitive lex compare
- `starts_with_any()` - Check any prefix
- `ends_with_any()` - Check any suffix

### 3.4 Parsing/Formatting (15)

- `parse_int(s: String) -> Int` - Parse integer (using strtol via FFI)
- `parse_float(s: String) -> Float` - Parse floating point (using strtod via FFI)
- `parse_bool(s: String) -> Bool` - Parse boolean
- `to_string(x: Any) -> String` - Convert to string
- `format(template: String, args: List<String>) -> String` - String formatting
- `pad_left()` / `pad_right()` / `center()` - Padding operations
- `ljust()` / `rjust()` - Justification
- `escape()` / `unescape()` - Escape special characters
- `codes(s: String) -> List<Int>` - Get character codes
- `from_codes(codes: List<Int>) -> String` - Create from char codes
- `grapheme_count()` - Count graphemes (unicode)

### 3.5 Pattern/Regex (10)

- `match(s: String, pattern: String) -> Bool` - Simple pattern match
- `matches(s: String, pattern: String) -> List<String>` - Find all matches
- `match_groups()` - Extract capture groups
- `split_pattern()` - Split by pattern
- `replace_pattern()` - Replace by pattern
- `is_match()` - Check pattern match
- `find()` - Find pattern position
- `find_all()` - Find all positions
- `count_pattern()` - Count occurrences
- `has_whitespace()` - Check for whitespace

**Implementation Strategy:**

Option A: Simple string search (Week 21)
```rust
fn match(s: String, pattern: String) -> Bool {
    s.contains(&pattern)
}
```

Option B: Full Regex Support (Week 24)
```rust
use regex::Regex;

fn match(s: String, pattern: String) -> Bool {
    Regex::new(&pattern).unwrap().is_match(&s)
}
```

---

## 4. Collections Library (50 functions)

### 4.1 List Operations (25)

**Core Functions:**
- `list_new<T>() -> List<T>` - Create empty list
- `list_push<T>(list, item)` - Add to end (O(1) amortized)
- `list_pop<T>(list) -> T` - Remove from end (O(1))
- `list_shift<T>(list) -> T` - Remove from start (O(n))
- `list_unshift<T>(list, item)` - Add to start (O(n))
- `list_insert<T>(list, index, item)` - Insert at index (O(n))
- `list_remove<T>(list, index) -> T` - Remove at index (O(n))
- `list_get<T>(list, index) -> T` - Get at index (O(1))
- `list_set<T>(list, index, item)` - Set at index (O(1))
- `list_clear<T>(list)` - Remove all items
- `list_contains<T>(list, item) -> Bool` - Check membership
- `list_index_of<T>(list, item) -> Int` - Find item index
- `list_reverse<T>(list)` - Reverse in place
- `list_sort<T>(list)` - Sort in place (quicksort)
- `list_sorted<T>(list) -> List<T>` - Return sorted copy
- `list_shuffle<T>(list)` - Shuffle in place (Fisher-Yates)
- `list_map<T,U>(list, fn) -> List<U>` - Apply function
- `list_filter<T>(list, fn) -> List<T>` - Filter items
- `list_reduce<T>(list, fn) -> T` - Fold/reduce
- `list_unique<T>(list) -> List<T>` - Remove duplicates
- `list_flatten<T>(list) -> List<T>` - Flatten nested
- `list_zip<T,U>(a, b) -> List<(T,U)>` - Zip two lists
- `list_chunk<T>(list, size) -> List<List<T>>` - Split into chunks
- `list_take<T>(list, count) -> List<T>` - Take first N
- `list_drop<T>(list, count) -> List<T>` - Drop first N

**Implementation:**

```rust
// Using Rust Vec<T> internally
impl<T> List<T> {
    pub fn new() -> Self { List(Vec::new()) }
    
    pub fn push(&mut self, item: T) {
        self.0.push(item);  // O(1) amortized
    }
    
    pub fn sort(&mut self) where T: Ord {
        self.0.sort();  // Rust's introsort: O(n log n)
    }
    
    pub fn map<U, F>(&self, f: F) -> List<U> 
        where F: Fn(&T) -> U 
    {
        List(self.0.iter().map(f).collect())
    }
}
```

### 4.2 Map Operations (15)

- `map_new<K,V>() -> Map<K,V>` - Create empty map
- `map_put<K,V>(map, key, value)` - Set key-value
- `map_get<K,V>(map, key) -> V` - Get by key
- `map_remove<K,V>(map, key) -> V` - Remove by key
- `map_contains<K,V>(map, key) -> Bool` - Check key exists
- `map_keys<K,V>(map) -> List<K>` - Get all keys
- `map_values<K,V>(map) -> List<V>` - Get all values
- `map_clear<K,V>(map)` - Remove all items
- `map_size<K,V>(map) -> Int` - Get item count
- `map_is_empty<K,V>(map) -> Bool` - Check if empty
- `map_merge<K,V>(a, b) -> Map<K,V>` - Merge two maps
- `map_map<K,V,U>(map, fn) -> List<U>` - Transform values
- `map_filter<K,V>(map, fn) -> Map<K,V>` - Filter entries
- `map_from_pairs<K,V>(pairs) -> Map<K,V>` - Create from list
- `map_to_pairs<K,V>(map) -> List<(K,V)>` - Convert to list

### 4.3 Set Operations (10)

- `set_new<T>() -> Set<T>` - Create empty set
- `set_add<T>(set, item)` - Add item
- `set_remove<T>(set, item)` - Remove item
- `set_contains<T>(set, item) -> Bool` - Check membership
- `set_union<T>(a, b) -> Set<T>` - Set union
- `set_intersection<T>(a, b) -> Set<T>` - Set intersection
- `set_difference<T>(a, b) -> Set<T>` - Set difference
- `set_symmetric_diff<T>(a, b) -> Set<T>` - Symmetric difference
- `set_size<T>(set) -> Int` - Get item count
- `set_to_list<T>(set) -> List<T>` - Convert to list

**Implementation:**

```rust
// Using HashMap/HashSet from Rust std
pub struct Set<T: Hash + Eq>(HashSet<T>);

impl<T: Hash + Eq> Set<T> {
    pub fn new() -> Self { Set(HashSet::new()) }
    
    pub fn union(a: &Set<T>, b: &Set<T>) -> Set<T> {
        let mut result = a.0.clone();
        for item in &b.0 {
            result.insert(item.clone());
        }
        Set(result)
    }
}
```

---

## 5. I/O Library (10 functions)

- `print(msg: String) -> Void` - Print to stdout
- `println(msg: String) -> Void` - Print with newline
- `read_line() -> String` - Read from stdin
- `read_file(path: String) -> String` - Read entire file
- `write_file(path: String, content: String) -> Void` - Write entire file
- `append_file(path: String, content: String) -> Void` - Append to file
- `file_exists(path: String) -> Bool` - Check file exists
- `list_files(dir: String) -> List<String>` - List directory
- `mkdir(path: String) -> Bool` - Create directory
- `delete_file(path: String) -> Bool` - Delete file

---

## 6. Time Library (4 functions)

- `now_ms() -> Long` - Milliseconds since epoch
- `now_s() -> Long` - Seconds since epoch
- `sleep(ms: Int) -> Void` - Sleep milliseconds
- `sleep_seconds(s: Int) -> Void` - Sleep seconds

---

## 7. Type Library (4 functions)

- `type_of(x: Any) -> String` - Get type name
- `is_int(x: Any) -> Bool` - Check if int
- `is_float(x: Any) -> Bool` - Check if float
- `is_string(x: Any) -> Bool` - Check if string

---

## 8. Concurrency Library (3 functions)

- `spawn_actor<T>(fn: Fn()->T) -> Actor` - Spawn actor
- `send_message(actor: Actor, msg: String) -> Void` - Send message
- `receive_message(actor: Actor) -> String` - Receive message

---

## 9. Implementation Timeline

### Week 21: Math & String (Day 1-5)
- [ ] Math library (80 functions)
- [ ] String library (60 functions)
- [ ] Integrated tests (100+ tests)
- [ ] Performance benchmarks

### Week 22: Collections & I/O (Day 6-10)
- [ ] Collections library (50 functions)
- [ ] I/O library (10 functions)
- [ ] Generic type support
- [ ] Iterator protocols

### Week 23: Time & Type (Day 11-15)
- [ ] Time library (4 functions)
- [ ] Type library (4 functions)
- [ ] Advanced type reflection
- [ ] Type conversion utilities

### Week 24: Concurrency & Polish (Day 16-20)
- [ ] Concurrency library (3 functions)
- [ ] Full stdlib integration testing
- [ ] Documentation generation
- [ ] Performance optimization pass

---

## 10. Testing Strategy

### 10.1 Unit Tests (350+ tests)
```
- Math:        80 functions × 2-3 tests = 200+ tests
- String:      60 functions × 1-2 tests = 100+ tests
- Collections: 50 functions × 1 test = 50 tests
- I/O:         10 functions × 1 test = 10 tests
- Time/Type:   8 functions × 1 test = 8 tests
```

### 10.2 Integration Tests (30+ tests)
- Cross-category operations (map + string conversion)
- Large collection handling (1M items)
- Concurrent access (multiple actors)
- Error recovery

### 10.3 Performance Tests (15+ benchmarks)
- Math: 1M operations/second target
- String: 100K operations/second target
- Collections: Depends on size
- Overall: < 10% Phase 7 overhead

---

## 11. Killer Syntax Examples

### 11.1 Math Operations
```killer
use std::math

fn calculate_distance(x1: Float, y1: Float, x2: Float, y2: Float) -> Float {
    let dx = x2 - x1
    let dy = y2 - y1
    std::math::hypot(dx, dy)
}

fn is_perfect_square(n: Int) -> Bool {
    let sqrt_n = std::math::sqrt(n as Float)
    std::math::round(sqrt_n) * std::math::round(sqrt_n) == n
}
```

### 11.2 String Operations
```killer
use std::string

fn format_name(first: String, last: String) -> String {
    let parts = [std::string::to_upper(first), std::string::to_upper(last)]
    std::string::join(parts, " ")
}
```

### 11.3 Collections
```killer
use std::collections

fn unique_sorted(list: List<Int>) -> List<Int> {
    std::collections::list_unique(
        std::collections::list_sorted(list)
    )
}
```

---

## 12. Success Criteria

✅ **Completeness:**
- 220+ functions implemented
- 350+ unit tests passing
- 100% code coverage for core functions

✅ **Performance:**
- Math: 1M+ ops/sec
- String: 100K+ ops/sec
- Collections: O(n log n) for sort

✅ **Integration:**
- All functions discoverable via module system
- Seamless FFI integration
- Clear error messages

✅ **Documentation:**
- Function signatures documented
- Examples for all categories
- Type annotations complete

---

## 13. Risk Mitigation

**Risk 1: Implementation falls behind schedule**
- Mitigation: Prioritize high-impact functions (sqrt, split, sort)
- Fallback: Use Rust stdlib functions directly

**Risk 2: Performance doesn't meet targets**
- Mitigation: Profile early and optimize hot paths
- Fallback: Acceptable if within 2x target

**Risk 3: Type system complexity for generics**
- Mitigation: Use trait bounds (Fn, Ord, Hash)
- Fallback: Monomorphize at compile time

---

## 14. Deliverables Checklist

- [x] stdlib_builder.rs (1600 lines)
- [x] test_phase21_stdlib.rs (400 lines)
- [ ] stdlib implementation (2000+ lines)
- [ ] Comprehensive tests (350+ tests)
- [ ] Integration guide
- [ ] Performance benchmarks
- [ ] User documentation
- [ ] Examples for each category

**Total Lines of Code:** ~4000 lines (stdlib + tests + docs)  
**Est. Implementation Time:** 20-25 developer-days  
**Complexity:** Medium-High (generics, FFI integration)  

---

## Next Steps After Phase 21

**Phase 22 (Weeks 25-26):** Observability & Monitoring
- Performance metrics collection
- Request tracing
- Error tracking
- Health checks

**Phase 23 (Weeks 27-30):** Advanced Type System
- Generics with constraints
- Trait/interface system
- Type annotations
- Pattern matching enhancements

**Phase 24+ (Month 6+):** Production Features
- JIT compilation
- Package manager
- Database integration
- Web framework
