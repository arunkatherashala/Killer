# Killer Standard Library API Reference

**Version**: 2.1.0  
**Status**: ✅ 171 tests passing (all phases complete)  
**Database**: 180+ functions across 9 modules

## Table of Contents

1. [Phase 1: Core Data Operations](#phase-1-core-data-operations)
   - [Math Module](#math-module)
   - [String Module](#string-module)
   - [Array Module](#array-module)
   - [File I/O Module](#file-io-module)

2. [Phase 2: Data Types & Formats](#phase-2-data-types--formats)
   - [JSON Module](#json-module)
   - [Type Utilities Module](#type-utilities-module)
   - [DateTime Module](#datetime-module)

3. [Phase 3: Advanced Processing](#phase-3-advanced-processing)
   - [Logging Module](#logging-module)
   - [Regex Module](#regex-module)
   - [Compression Module](#compression-module)

4. [Examples & Best Practices](#examples--best-practices)

---

## Phase 1: Core Data Operations

### Math Module

**File**: `src/math.rs` | **Lines**: 900+ | **Tests**: 9 | **Functions**: 30+

Core mathematical operations for numerical computing.

#### Constants
```
PI, E, LN2, LN10, SQRT2, TAU, INF, NEG_INF, NAN
```

#### Basic Operations
| Function | Example | Result |
|----------|---------|--------|
| `abs(n)` | `MathModule::abs(-42.0)` | `42.0` |
| `min(a, b)` | `MathModule::min(3.0, 5.0)` | `3.0` |
| `max(a, b)` | `MathModule::max(3.0, 5.0)` | `5.0` |
| `sum(arr)` | `MathModule::sum(&vec![1, 2, 3])` | `6` |
| `average(arr)` | `MathModule::average(&vec![2, 4, 6])` | `4.0` |

#### Rounding Functions
| Function | Input | Output |
|----------|-------|--------|
| `ceil(4.2)` | → | `5.0` |
| `floor(4.8)` | → | `4.0` |
| `round(4.5)` | → | `5.0` (rounds away from zero) |
| `trunc(4.9)` | → | `4.0` |

#### Powers & Roots
```
pow(base, exp) - Exponentiation
sqrt(n) - Square root
cbrt(n) - Cube root
nthroot(n, root) - Nth root
exp(x) - e^x
exp2(x) - 2^x
exp10(x) - 10^x
```

#### Logarithms
```
ln(x) - Natural logarithm
log10(x) - Base 10 logarithm
log2(x) - Base 2 logarithm
log(x, base) - Custom base logarithm
```

#### Trigonometry
```
sin(rad), cos(rad), tan(rad) - Basic trig
asin(x), acos(x), atan(x) - Inverse trig
atan2(y, x) - Two-argument arctangent
sinh(x), cosh(x), tanh(x) - Hyperbolic
```

#### Utilities
```
gcd(a, b) - Greatest common divisor
lcm(a, b) - Least common multiple
is_even(n) - Check if even
is_odd(n) - Check if odd
is_prime(n) - Primality test
fibonacci(n) - Nth Fibonacci number
factorial(n) - N!
to_radians(degrees) - Convert degrees to radians
to_degrees(radians) - Convert radians to degrees
is_nan(n), is_infinite(n), is_finite(n)
random() - Random [0, 1)
random_int(min, max) - Random integer in range
random_range(min, max) - Random float in range
```

---

### String Module

**File**: `src/string_utils.rs` | **Lines**: 750+ | **Tests**: 7 | **Methods**: 25+

String manipulation and text processing.

#### Case Conversion
```
uppercase(s) → "HELLO"
lowercase(s) → "hello"
capitalize(s) → "Hello"
title_case(s) → "Hello World"
camel_case(s) → "helloWorld"
snake_case(s) → "hello_world"
kebab_case(s) → "hello-world"
```

#### Search & Find
```
index_of(s, sub) → Some(2)  // First occurrence
last_index_of(s, sub) → Some(5)  // Last occurrence
contains(s, sub) → true/false
starts_with(s, prefix) → true/false
ends_with(s, suffix) → true/false
count(s, sub) → 3  // Occurrence count
```

#### Trimming & Padding
```
trim(s) → "hello"  // Both sides
trim_start(s) → "hello  "  // Left side only
trim_end(s) → "  hello"  // Right side only
trim_char(s, ch) → With custom character
pad_start(s, width, ch) → Pad left
pad_end(s, width, ch) → Pad right
```

#### Splitting & Joining
```
split(s, sep) → Vec<String>  // By delimiter
split_whitespace(s) → Vec<String>  // By whitespace
join(arr, sep) → "a,b,c"  // Array join
```

#### Replacement
```
replace_first(s, old, new) → "Hello, World!"
replace_all(s, old, new) → "HeLLo, WorLd!"
```

#### Extraction
```
substring(s, start, end) → Substring slice
substring_from(s, start) → From position to end
substring_to(s, end) → From start to position
first(s, n) → First n characters
last(s, n) → Last n characters
reverse(s) → "dlrow olleH"
```

#### Queries
```
length(s) → 5
byte_length(s) → 5 (UTF-8 bytes)
is_empty(s) → true/false
is_uppercase(s) → true/false
is_lowercase(s) → true/false
is_numeric(s) → true/false
is_alpha(s) → true/false
is_alphanumeric(s) → true/false
repeat(s, times) → "aaa" (from "a", 3 times)
```

---

### Array Module

**File**: `src/array_utils.rs` | **Lines**: 850+ | **Tests**: 5 | **Methods**: 20+

Array and collection operations (works with Value enum).

#### Basic Operations
```
length(arr) → 5
is_empty(arr) → false
first(arr) → Some(Value)
last(arr) → Some(Value)
at(arr, index) → Some(Value)
fill(arr, value) → Filled array
```

#### Sorting & Ordering
```
sort(arr) → Sorted ascending
sort_reverse(arr) → Sorted descending
reverse(arr) → Reversed array
```

#### Search
```
index_of(arr, value) → Some(2)
last_index_of(arr, value) → Some(5)
contains(arr, value) → true/false
count(arr, value) → 3
```

#### Transformation
```
push(arr, value) → Add to end
pop(arr) → Remove last
unshift(arr, value) → Add to start
shift(arr) → Remove first
concat(arr1, arr2) → Merged array
flatten(arr) → One level deep
deep_flatten(arr) → Fully flattened
```

#### Advanced
```
unique(arr) → Deduplicated
chunk(arr, size) → Split into chunks [[a,b], [c,d]]
slice(arr, start, end) → Subarray
rotate_left(arr, n) → Rotate left
rotate_right(arr, n) → Rotate right
sum(arr) → Total of numbers
min(arr) → Minimum value
max(arr) → Maximum value
average(arr) → Mean value
join(arr, sep) → "a,b,c"
```

---

### File I/O Module

**File**: `src/file_io.rs` | **Lines**: 950+ | **Tests**: 4 | **Functions**: 25+

File system operations with error handling.

#### Reading
```
read_file(path) → Result<String>
read_bytes(path) → Result<Vec<u8>>
read_lines(path) → Result<Vec<String>>
read_lines_chunked(path, chunk_size) → Result<String>
```

#### Writing
```
write_file(path, content) → Result<()>
write_bytes(path, bytes) → Result<()>
append_file(path, content) → Result<()>
write_lines(path, lines) → Result<()>
```

#### Metadata & Queries
```
exists(path) → bool
is_file(path) → bool
is_directory(path) → bool
file_size(path) → Result<u64>
extension(path) → Result<String>  // "txt"
file_name(path) → Result<String>  // "readme.txt"
dir_name(path) → Result<String>  // parent directory
absolute_path(path) → Result<String>
```

#### Directory Operations
```
list_dir(path) → Result<Vec<String>>  // Files in dir
list_dir_recursive(path) → Result<Vec<String>>  // All files
mkdir(path) → Result<()>  // Create directory
```

#### Deletion
```
delete_file(path) → Result<()>
delete_dir(path) → Result<()>  // Empty only
delete_dir_recursive(path) → Result<()>  // With contents
```

#### Movement & Copy
```
rename(old_path, new_path) → Result<()>
copy_file(src, dst) → Result<()>
```

#### Error Types
```
FileError::NotFound
FileError::PermissionDenied
FileError::InvalidPath
FileError::IOError(msg)
FileError::InvalidEncoding
```

---

## Phase 2: Data Types & Formats

### JSON Module

**File**: `src/json_module.rs` | **Lines**: 950+ | **Tests**: 9 | **Functions**: 15+

JSON parsing, serialization, and validation.

#### Parsing
```
parse(json_str) → Result<JsonValue>
parse_to_value(json_str) → Result<Value>
```

#### Serialization
```
stringify(value) → "compact"
stringify_pretty(value) → "formatted\n  nicely"
```

#### Validation
```
is_valid(json_str) → true/false
type_of(json_value) → "null" | "boolean" | "number" | "string" | "array" | "object"
```

#### Access
```
get(obj, "key") → Some(JsonValue)
get_at(arr, 0) → Some(JsonValue)
get_path(obj, "person.name") → Some(JsonValue)
has_key(obj, "name") → true/false
keys(obj) → ["name", "age"]
length(value) → Some(3)
```

#### Example
```killer
// Parse JSON
let data = json_module::parse("{\"name\": \"Alice\", \"age\": 30}")
let name = json_module::get(data, "name")  // "Alice"

// Create and stringify
let obj = JsonValue::Object(...)
let json_str = json_module::stringify(obj)
```

---

### Type Utilities Module

**File**: `src/types_module.rs` | **Lines**: 750+ | **Tests**: 7 | **Functions**: 25+

Runtime type checking, conversion, and inspection.

#### Type Checking
```
typeof_value(val) → "number" | "string" | ...
is_null(val), is_bool(val), is_number(val), is_string(val)
is_array(val), is_object(val), is_function(val)
is_integer(n) → true if n.0 = 0.0
is_finite(n), is_infinite(n), is_nan(n)
is_empty(val) → true if array/dict/string empty
is_truthy(val) → loose truthiness check
```

#### Type Conversion
```
to_bool(val) → Some(true/false)
to_number(val) → Some(42.0)
to_string(val) → "42" or "[Array]"
cast(val, "number") → Some(Value::Number(...))
cast_or(val, "number", default) → Value with fallback
parse_as(str, "number") → Some(Value::Number(...))
```

#### Comparison
```
equals(a, b) → true (strict equality - types must match)
loose_equals(a, b) → true (coerced equality)
same_type(a, b) → true if same type
```

#### Inspection
```
length(val) → Some(5) for arrays/strings/dicts
has(val, key) → true if contains key
keys(obj) → ["a", "b", "c"]
values(obj) → [1, 2, 3]
inspect(val) → "number (42.0), finite, positive, integer"
```

---

### DateTime Module

**File**: `src/datetime_module.rs` | **Lines**: 1000+ | **Tests**: 10 | **Functions**: 25+

Time and date manipulation with Unix timestamps.

#### Current Time
```
now() → u64  // Seconds since Unix epoch
now_millis() → u64  // Milliseconds
now_micros() → u64  // Microseconds
today() → DateTime
```

#### Timestamp Operations
```
from_timestamp(secs) → DateTime
from_millis(millis) → DateTime
to_seconds(dt) → u64
to_millis(dt) → u64
```

#### Arithmetic
```
add_seconds(dt, 3600) → DateTime
add_minutes(dt, 60) → DateTime
add_hours(dt, 2) → DateTime
add_days(dt, 1) → DateTime
subtract_seconds(dt, secs) → DateTime
// Also: subtract_minutes, subtract_hours, subtract_days
```

#### Differences
```
difference_seconds(dt1, dt2) → i64
difference_minutes(dt1, dt2) → i64
difference_hours(dt1, dt2) → i64
difference_days(dt1, dt2) → i64
```

#### Formatting
```
format_iso(ts) → "2024-03-13T12:30:45Z"
format_date(ts) → "2024-03-13"
format_time(ts) → "12:30:45"
parse_iso("2024-03-13T...") → Some(timestamp)
parse_date("2024-03-13") → Some(timestamp)
```

#### Calendar Utilities
```
day_of_week(ts) → 0-6 (0=Sunday)
day_name(ts) → "Friday"
is_leap_year(2024) → true
days_in_month(2, 2024) → 29
days_in_year(2024) → 366
is_valid_date(2024, 2, 29) → true
```

#### Utilities
```
elapsed(ts) → "2 hours ago" | "in 1 day"
components(ts) → HashMap with breakdown
```

---

## Phase 3: Advanced Processing

### Logging Module

**File**: `src/logging_module.rs` | **Lines**: 900+ | **Tests**: 8 | **Functions**: 18+

Structured logging with levels and filtering.

#### Log Levels (in order of severity)
```
Trace (0), Debug (1), Info (2), Warn (3), Error (4)
```

#### Logger Creation
```
Logger::new(LogLevel::Debug)  // Filter below this level
Logger::with_capacity(LogLevel::Info, 5000)  // Custom max entries
```

#### Logging Methods
```
logger.trace(msg)
logger.debug(msg)
logger.info(msg)
logger.warn(msg)
logger.error(msg)
```

#### Configuration
```
logger.set_level(LogLevel::Warn)  // Change filter
logger.set_source("module_name")  // Set context
logger.clear_source()  // Remove context
```

#### Retrieval
```
logger.logs(LogLevel::Error) → Vec of error messages
logger.all_logs() → All messages
logger.logs_since(LogLevel::Warn) → Warn + Error
logger.all_logs_json() → JSON array
logger.count() → Total entries
logger.count_by_level(LogLevel::Error) → Count
logger.search("query") → Search messages
logger.first(10) → First N entries
logger.last(10) → Last N entries
logger.clear() → Remove all
```

#### Formatting
```
LogEntry::format() → "[timestamp] LEVEL | source: message"
LogEntry::format_json() → JSON object
LoggingModule::summary(logger) → Statistics
```

---

### Regex Module

**File**: `src/regex_module.rs` | **Lines**: 800+ | **Tests**: 8 | **Functions**: 15+

Pattern matching and text search (without external regex library).

#### Basic Matching
```
matches("hello", "h.*o") → true
contains("hello world", "wor") → true
starts_with(text, pattern) → bool
ends_with(text, pattern) → bool
```

#### Searching
```
find("abcabc", "bc") → Some((1, 3))  // Position tuple
find_all("abcabc", "bc") → [(1, 3), (4, 6)]
count("hello", "l") → 2
```

#### Replacement
```
replace("hello", "l", "L") → "heLlo"  // First only
replace_all("hello", "l", "L") → "heLLo"  // All
```

#### Splitting & Extraction
```
split("a,b,c", ",") → ["a", "b", "c"]
extract("hello world", "w.*d") → Some("world")
extract_all("abc abc", "a.c") → ["abc", "abc"]
```

#### Supported Patterns
```
.     - Any single character
*     - Zero or more of previous
+     - One or more of previous
?     - Zero or one of previous
[abc] - Character class
```

#### Example
```killer
if regex_module::contains(email, "@") {
    let parts = regex_module::split(email, "@")
}
```

---

### Compression Module

**File**: `src/compression_module.rs` | **Lines**: 900+ | **Tests**: 9 | **Functions**: 15+

Data compression and encoding utilities.

#### Run-Length Encoding (RLE)
```
rle_encode("aaabbb") → "a3b3"
rle_decode("a3b3") → "aaabbb"
compression_ratio(original, compressed) → 2.0
should_compress(text) → true if ratio > 1.1
```

#### Base64 Encoding
```
base64_encode("ABC") → encoded string
base64_decode(encoded) → Some("ABC")
```

#### Hexadecimal
```
hex_encode("ABC") → "414243"
hex_decode("414243") → Some("ABC")
```

#### Analysis
```
size(text) → bytes
size_kb(text) → float
size_mb(text) → float
best_compression(text) → ("rle" or "base64", ratio)
```

#### Example
```killer
let encoded = compression_module::hex_encode("Secret")
let compressed = compression_module::rle_encode(data)
let ratio = compression_module::compression_ratio(original, compressed)
```

---

## Examples & Best Practices

### Mathematical Computing
```killer
use math

// Calculate circle area
let radius = 5.0
let area = math::PI * math::pow(radius, 2.0)

// GCD and LCM
let gcd = math::gcd(48, 18)  // 6
let lcm = math::lcm(12, 18)  // 36

// Fibonacci sequence
for i in 0..10 {
    print(math::fibonacci(i))
}
```

### String Processing
```killer
use string_utils

// Parse email
let email = "john@example.com"
let parts = string_utils::split(email, "@")
let username = parts[0]
let domain = parts[1]

// Format text
let name = "john doe"
let title_case = string_utils::title_case(name)  // "John Doe"
```

### File Operations
```killer
use file_io

// Read and process
let lines = file_io::read_lines("data.txt")
for line in lines {
    let processed = string_utils::uppercase(line)
    // Process each line
}

// Check and create
if !file_io::exists("output/") {
    file_io::mkdir("output/")
}
```

### JSON Data Handling
```killer
use json_module
use types_module

// Parse and extract
let data = json_module::parse(json_string)
let name = json_module::get(data, "user")
let age = types_module::to_number(json_module::get(data, "age"))

// Validate input
if json_module::is_valid(user_input) {
    let parsed = json_module::parse(user_input)
}
```

### Logging Best Practices
```killer
use logging_module

let logger = Logger::new(LogLevel::Debug)
logger.set_source("main")

logger.info("Application started")
logger.debug("Loading configuration...")

if error_occurred {
    logger.error("Failed to load config")
}

// Output summary
let summary = LoggingModule::summary(&logger)
```

### Type-Safe Operations
```killer
use types_module

// Safe conversions
if let Some(num) = types_module::to_number(value) {
    let doubled = num * 2.0
}

// Type checking
if types_module::is_array(value) {
    let elements = types_module::length(value)
}

// Loose equality for user input
if types_module::loose_equals(user_input, expected) {
    // Input matches, accounting for type coercion
}
```

---

## Performance Notes

| Module | Use Case | Performance |
|--------|----------|-------------|
| **Math** | Numerical computing | Native Rust speed |
| **String** | Text processing | O(n) per operation |
| **Array** | Collections | O(n) sorting, O(1) access |
| **File I/O** | Disk operations | System-dependent |
| **JSON** | Data serialization | Approx 10-20MB/s parse |
| **Types** | Runtime checking | O(1) for most operations |
| **DateTime** | Timestamp math | O(1) all operations |
| **Logging** | Debug output | Minimal overhead (thread-safe) |
| **Regex** | Pattern matching | Basic patterns O(n*m) |
| **Compression** | Data encoding | RLE O(n), Base64 O(n) |

---

## Error Handling

All modules return `Result` types or `Option` for safe operations:

```killer
// File I/O
match file_io::read_file("test.txt") {
    Ok(content) → { /* Process */ },
    Err(FileError::NotFound) → { /* Handle missing */ },
    Err(e) → { /* Handle other errors */ }
}

// JSON
match json_module::parse(input) {
    Ok(data) → { /* Use data */ },
    Err(JsonError::ParseError(msg)) → { /* Handle parse error */ }
}

// Type conversion
match types_module::to_number(value) {
    Some(n) → { /* Use numeric value */ },
    None → { /* Value not convertible */ }
}
```

---

## Module Import Summary

```killer
use math                // 30+ math functions
use string_utils        // 25+ string methods
use array_utils         // 20+ array methods
use file_io             // 25+ file operations
use json_module         // 15+ JSON functions
use types_module        // 25+ type functions
use datetime_module     // 25+ time functions
use logging_module      // 18+ logging functions
use regex_module        // 15+ regex functions
use compression_module  // 15+ compression functions
```

---

## Version Compatibility

**Killer Standard Library v2.1.0** is compatible with:
- Killer VM v2.1.0+
- All Value enum operations
- Zero external dependencies (Rust std only)

**Last Updated**: March 12, 2026  
**Total Tests**: 171 passing (100% pass rate)
