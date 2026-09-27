# Killer Language Reference

Killer is a dynamically-typed, bytecode-compiled scripting language with a focus on
data engineering and AI workloads. It compiles to an internal bytecode that runs on
a register-and-stack virtual machine written in Rust.

---

## Section 1: Quick Start

Five minimal programs that compile and run today.

### 1.1 Hello world

```killer
println("hello world")
```

### 1.2 Variables and arithmetic

```killer
let x = 10
let y = 3
println(x + y)
println(x * y)
println(x / y)
```

### 1.3 Function call

```killer
fn greet(name) {
    return "Hello, " + name
}
println(greet("Killer"))
```

### 1.4 Array iteration

```killer
let nums = [1, 2, 3, 4, 5]
for n in nums {
    println(n * n)
}
```

### 1.5 Dictionary

```killer
let person = {"name": "Alice", "age": 30}
println(person["name"])
println(person["age"])
```

---

## Section 2: Variables and Types

### 2.1 Declaration

```killer
let x = 42            -- Number (f64 internally)
let s = "hello"       -- String
let b = true          -- Bool
let n = null          -- Null
let arr = [1, 2, 3]   -- Array
let d = {"k": "v"}    -- Dict
let s2 = set_new(1, 2, 3)  -- Set
```

Assignment without `let` rebinds an existing name:

```killer
let count = 0
count = count + 1
```

Compound assignment:

```killer
let x = 10
x += 5
x -= 2
x *= 3
x /= 2
```

Increment / decrement:

```killer
let i = 0
i++
i--
++i
--i
```

### 2.2 Number

All numeric literals are 64-bit float internally. When the fractional part is
zero, the display drops the decimal point.

```killer
let a = 3.14
let b = 1e6
println(a)   -- 3.14
println(b)   -- 1000000
```

Floor division uses `//`:

```killer
println(7 // 2)   -- 3
```

Modulo uses `%`:

```killer
println(7 % 3)    -- 1
```

### 2.3 String

Double-quoted strings. Escape sequences: `\"`, `\\`, `\n`, `\t`.

K-strings support interpolation:

```killer
let name = "world"
let msg = K"Hello {name}!"
println(msg)    -- Hello world!
```

String concatenation uses `+`:

```killer
println("foo" + "bar")   -- foobar
```

### 2.4 Bool

`true` and `false`. Logical operators: `&&` (and), `||` (or), `!` (not).

### 2.5 Null

The singleton value `null`. Truthy rules: `false`, `0`, `""`, `[]`, and `null`
are falsy; everything else is truthy.

### 2.6 Array

```killer
let a = [10, 20, 30]
println(a[0])      -- 10
push(a, 40)        -- mutates a in place
println(len(a))    -- 4
a[0] = 99
println(a)         -- [99, 20, 30, 40]
```

### 2.7 Dict

```killer
let d = {"x": 1, "y": 2}
d["z"] = 3
println(keys(d))   -- [x, y, z]
println(d["z"])    -- 3
```

### 2.8 Set

Unordered collection of unique values. Members must be numbers, strings, or booleans.

```killer
let s = set_new(1, 2, 3)
set_add(s, 4)
println(set_has(s, 2))    -- true
println(set_size(s))       -- 4
```

### 2.9 Bytes (OS-level primitive)

Raw byte buffer for memory regions and binary data.

```killer
let buf = bytes_new(16)       -- 16-byte zeroed buffer
bytes_set(buf, 0, 255)
println(bytes_get(buf, 0))    -- 255
```

### 2.10 Integer (OS-level primitive)

64-bit signed integer for addresses and bitwise operations.

```killer
let reg = to_integer(42)
println(bit_and(reg, 0xFF))
```

---

## Section 3: Control Flow

### 3.1 if / else

```killer
let x = 10
if x > 5 {
    println("big")
} else {
    println("small")
}
```

Else-if chain:

```killer
if x < 0 {
    println("negative")
} else if x == 0 {
    println("zero")
} else {
    println("positive")
}
```

Ternary expression:

```killer
let label = x > 0 ? "positive" : "non-positive"
```

### 3.2 while

```killer
let i = 0
while i < 5 {
    println(i)
    i++
}
```

### 3.3 do / while

```killer
let i = 0
do {
    println(i)
    i++
} while i < 3
```

### 3.4 for-in

Iterates over arrays and ranges.

```killer
let fruits = ["apple", "banana", "cherry"]
for fruit in fruits {
    println(fruit)
}
```

Range syntax `start..end`:

```killer
for i in 0..5 {
    println(i)    -- 0 1 2 3 4
}
```

Range with step `start..end..step`:

```killer
for i in 0..10..2 {    -- 0, 2, 4, 6, 8
    println(i)
}
```

### 3.5 C-style for

```killer
for (let i = 0; i < 5; i++) {
    println(i)
}
```

### 3.6 break and continue

```killer
let i = 0
while true {
    if i >= 3 { break }
    println(i)
    i++
}
```

### 3.7 switch

```killer
let color = "red"
switch color {
    case "red":
        println("stop")
    case "green":
        println("go")
    default:
        println("wait")
}
```

### 3.8 match

Structural pattern matching with optional guard:

```killer
let val = 42
match val {
    0 => println("zero")
    n if n < 0 => println("negative")
    _ => println("positive")
}
```

Array destructure in match:

```killer
let point = [1, 2]
match point {
    [0, 0] => println("origin")
    [x, y] => println(str(x) + "," + str(y))
}
```

### 3.9 try / catch / finally

```killer
try {
    let data = readFile("missing.txt")
    println(data)
} catch e {
    println("error: " + e)
} finally {
    println("done")
}
```

### 3.10 throw

```killer
fn divide(a, b) {
    if b == 0 { throw "division by zero" }
    return a / b
}
```

---

## Section 4: Functions

### 4.1 Regular function

Both `fn` and `kfn` keywords compile identically.

```killer
fn add(a, b) {
    return a + b
}
println(add(3, 4))    -- 7
```

### 4.2 Implicit return

The last expression in a function body is the return value when no explicit
`return` is reached first.

```killer
fn square(n) {
    n * n
}
println(square(5))    -- 25
```

### 4.3 Closures (function expressions)

```killer
let double = |x| { x * 2 }
println(double(7))     -- 14

let add_n = |n| { |x| { x + n } }
let add5 = add_n(5)
println(add5(10))      -- 15
```

### 4.4 Async functions

Async functions run in a background OS thread. The call returns a Future handle
immediately. `await` blocks until the result is ready.

```killer
async fn fetch_data(url) {
    return http_get(url)
}

let f = fetch_data("https://example.com/api")
let result = await f
println(result)
```

`async kfn` is also accepted (same as `async fn`).

### 4.5 Decorators

A decorator is any callable that accepts a function and returns a replacement.
Place `@name` on the line before the `fn` definition.

```killer
fn memoize(f) {
    let cache = {}
    return |x| {
        let key = str(x)
        if cache[key] == null {
            cache[key] = f(x)
        }
        return cache[key]
    }
}

@memoize
fn fib(n) {
    if n <= 1 { return n }
    return fib(n - 1) + fib(n - 2)
}
println(fib(30))
```

Multiple decorators are applied bottom-up (innermost first).

### 4.6 Self tail-call optimization

Recursive calls in tail position are automatically converted to iteration.

```killer
fn sum_to(n, acc) {
    if n == 0 { return acc }
    return sum_to(n - 1, acc + n)
}
println(sum_to(100000, 0))
```

### 4.7 spawn (fire-and-forget)

```killer
async fn background_task() {
    thread_sleep_ms(100)
    println("background done")
}

spawn background_task()
println("main continues immediately")
```

---

## Section 5: Killer Unique Features

### 5.1 believe -- uncertain values

`believe name = value +/- margin` declares a variable whose nominal value carries
a known uncertainty range. The result is an `Uncertain` typed value.

The `+-` symbol is the Unicode plus-minus character U+00B1 (±).

```killer
believe temp = 98.6 +/- 0.5
println(temp)
```

Single-argument form stores a confidence level (no margin):

```killer
believe raining = 0.7
```

### 5.2 live -- reactive variables

`live name = expr` declares a variable that re-evaluates automatically whenever
any variable it depends on changes.

```killer
let a = 10
let b = 20
live total = a + b
println(total)    -- 30
a = 15
println(total)    -- 35
```

Dependencies are inferred from the expression at declaration time. The reactive
update executes synchronously on every assignment to a dependency.

### 5.3 x@-1 -- time travel

`name@offset` loads a historical snapshot of a variable. `@-1` returns the value
from the previous assignment.

```killer
let x = 10
x = 20
println(x@-1)    -- 10
x = 30
println(x@-1)    -- 20
```

### 5.4 ?? -- null coalescing

Returns the left operand when it is not null; otherwise evaluates and returns
the right operand.

```killer
let cfg = null
let port = cfg ?? 8080
println(port)    -- 8080
```

### 5.5 ?. -- optional chaining

Evaluates a method call or field access only when the object is not null.
Returns null rather than raising an error when the object is null.

```killer
let obj = null
let result = obj?.toString()
println(result)    -- null
```

### 5.6 List comprehensions

```killer
let nums = [1, 2, 3, 4, 5]
let squares = [n * n for n in nums]
println(squares)    -- [1, 4, 9, 16, 25]
```

With a filter condition:

```killer
let evens = [n for n in nums if n % 2 == 0]
println(evens)    -- [2, 4]
```

### 5.7 Trit values (balanced ternary)

Three-state logic: `T_NEG` (-1), `T_ZERO` (0), `T_POS` (+1).

```killer
let t = T_POS
let u = T_NEG
println(trit_and(t, u))     -- T_NEG  (min)
println(trit_or(t, u))      -- T_POS  (max)
println(trit_not(t))        -- T_NEG
println(trit_to_int(T_POS)) -- 1
```

### 5.8 Classes

```killer
class Animal {
    fn init(name) {
        this.name = name
    }
    fn speak() {
        return this.name + " makes a sound"
    }
}

class Dog extends Animal {
    fn speak() {
        return this.name + " barks"
    }
}

let d = new Dog("Rex")
println(d.speak())    -- Rex barks
```

### 5.9 Destructuring assignment

Array:

```killer
let [a, b, c] = [1, 2, 3]
println(a)    -- 1
```

Object:

```killer
let {name, age} = {"name": "Bob", "age": 25}
println(name)
```

### 5.10 kala -- natural language as code (experimental)

```killer
kala "add 5 and 3 and print the result"
kala "print the square root of 144" with x
```

---

## Section 6: Kore Data Engine

Kore (Killer Optimized Record Exchange) is a binary columnar file format built
into the Killer runtime. Features: adaptive compression (8 codecs), bloom
filters, per-column XOR encryption, ACID versioned writes, and predicate
pushdown.

### 6.1 Basic Kore (kore.rs)

**kore_write(path, schema, data)**

Writes data in Kore v1 format.
- `schema` -- comma-separated `name:type` pairs; types: `int`, `float`, `str`,
  `bool`, `bytes`
- `data` -- newline-separated rows, fields separated by commas

```killer
kore_write("people.kore", "name:str,age:int", "Alice,30\nBob,25")
```

**kore_read(path)**

Returns all rows as a formatted string.

```killer
let rows = kore_read("people.kore")
println(rows)
```

**kore_read_col(path, col)**

Returns the values of a single named column as a string.

```killer
let ages = kore_read_col("people.kore", "age")
println(ages)
```

**kore_info(path)**

Returns metadata (row count, column count, file size).

```killer
let info = kore_info("people.kore")
println(info)
```

### 6.2 Kore SQL queries (kore_query.rs)

**kore_query(sql)**

Executes a SQL-style SELECT and returns an array of row arrays.

Supported clauses: `SELECT`, `FROM`, `WHERE`, `GROUP BY`, `ORDER BY`, `LIMIT`.
Supported aggregates: `COUNT`, `SUM`, `AVG`, `MIN`, `MAX`.
Supported WHERE operators: `=`, `!=`, `>`, `<`, `>=`, `<=`, `CONTAINS`.

```killer
let rows = kore_query("SELECT name, age FROM people.kore WHERE age > 25")
for row in rows {
    println(row)
}
```

**kore_query_table(sql)**

Same query, returns a pretty-printed string table.

```killer
let table = kore_query_table(
    "SELECT region, SUM(amount) as total FROM sales.kore GROUP BY region ORDER BY total DESC LIMIT 5"
)
println(table)
```

**kore_query_csv(sql)**

Returns the result as a CSV string.

```killer
let csv = kore_query_csv("SELECT * FROM people.kore")
writeFile("out.csv", csv)
```

### 6.3 Kore transactions (kore_txn.rs)

Every commit produces an immutable snapshot. Old versions are preserved for
time-travel queries. Version history is stored in `<file>.kore.versions`.

**kore_txn_begin(path)**

Opens a transaction. Returns a numeric handle.

```killer
let handle = kore_txn_begin("inventory.kore")
```

**kore_txn_commit(handle, message)**

Commits the transaction. Returns the new version number.

```killer
let version = kore_txn_commit(handle, "restock update")
println("saved as version " + str(version))
```

**kore_txn_abort(handle)**

Rolls back the transaction.

```killer
kore_txn_abort(handle)
```

**kore_versions(path)**

Returns an array of version metadata objects.

```killer
let versions = kore_versions("inventory.kore")
for v in versions {
    println(v)
}
```

**kore_checkout(path, version)**

Reads the file at a specific version number.

```killer
let old_data = kore_checkout("inventory.kore", 2)
println(old_data)
```

**kore_as_of(path, timestamp_ms)**

Returns the data as it existed at a specific Unix timestamp (milliseconds).

```killer
let ts = 1700000000000
let old_data = kore_as_of("inventory.kore", ts)
```

**kore_diff(path, v1, v2)**

Returns a textual diff between two version numbers.

```killer
let diff = kore_diff("inventory.kore", 1, 2)
println(diff)
```

### 6.4 Kore v2 -- advanced format (kore_v2.rs)

Kore v2 uses an adaptive codec stack (Raw, RLE, Delta, DictRLE, Bitpack,
BDict, CDelta, FOR, HuffDict), Huffman+LZ77, bloom filters, and a
footer-indexed layout for O(1) column seeks.

**kore2_from_csv(csv_path, kore_path)**

Converts a CSV file to Kore v2 format.

```killer
kore2_from_csv("sales.csv", "sales.kore")
```

**kore2_to_csv(kore_path, csv_path)**

Exports a Kore v2 file to CSV.

```killer
kore2_to_csv("sales.kore", "sales_export.csv")
```

**kore2_filter(path, col, op, val)**

Returns rows matching a column predicate.
Supported operators: `"="`, `"!="`, `">"`, `"<"`, `">="`, `"<="`.

```killer
let big_orders = kore2_filter("sales.kore", "amount", ">", "1000")
println(big_orders)
```

**kore2_stats(path, col)**

Returns a dict with `null_count`, `min`, and `max` for one column.

```killer
let stats = kore2_stats("sales.kore", "amount")
println(stats)
```

**kore2_read_row(path, idx)**

Returns a single row by zero-based index. O(1) access via the footer.

```killer
let row = kore2_read_row("sales.kore", 0)
println(row)
```

**kore2_read_range(path, start, end)**

Returns rows from index `start` (inclusive) to `end` (exclusive).

```killer
let rows = kore2_read_range("sales.kore", 0, 100)
```

**kore2_info(path)**

Returns file metadata (version, columns, row count, codec details).

```killer
let info = kore2_info("sales.kore")
println(info)
```

**kore2_read(path)**

Reads all rows from a Kore v2 file.

```killer
let all_rows = kore2_read("sales.kore")
```

**kore2_read_col(path, col)**

Reads a single column from a Kore v2 file.

```killer
let amounts = kore2_read_col("sales.kore", "amount")
```

---

## Section 7: Standard Library Reference

### 7.1 Print and I/O

| Function | Description |
|---|---|
| `println(v, ...)` | Print values with newline |
| `print(v, ...)` | Alias for println |
| `readline()` | Read a line from stdin |
| `readline_prompt(prompt)` | Print prompt then read a line |
| `readFile(path)` | Read entire file as string |
| `writeFile(path, content)` | Write string to file |

### 7.2 Type functions

| Function | Description |
|---|---|
| `type(val)` | Returns type name as string |
| `str(val)` | Convert to string |
| `int(val)` | Convert to integer (truncates float) |
| `parseInt(s)` | Parse string to int |
| `parseFloat(s)` | Parse string to float |
| `String(val)` | Convert to string |
| `Number(val)` | Convert to number |
| `Boolean(val)` | Convert to bool |
| `isNaN(val)` | True if val is NaN |
| `isFinite(val)` | True if val is finite |

### 7.3 String functions

| Function | Description |
|---|---|
| `len(s)` | String length |
| `upper(s)` | Uppercase |
| `lower(s)` | Lowercase |
| `trim(s)` | Strip leading and trailing whitespace |
| `split(s, sep)` | Split into array |
| `starts_with(s, prefix)` | Bool |
| `ends_with(s, suffix)` | Bool |
| `contains(s, sub)` | Bool |
| `replace(s, old, new)` | Replace first occurrence |
| `substring(s, start, end)` | Extract substring |
| `indexOf(s, sub)` | First index (-1 if not found) |
| `charAt(s, i)` | Character at index |
| `charCodeAt(s, i)` | Code point at index |
| `repeat(s, n)` | Repeat string n times |

### 7.4 Array functions

| Function | Description |
|---|---|
| `len(arr)` | Array length |
| `push(arr, val)` | Append (mutates) |
| `pop(arr)` | Remove and return last element (mutates) |
| `reverse(arr)` | Reverse in place (mutates) |
| `reversed(arr)` | Return reversed copy |
| `sorted(arr)` | Return sorted copy |
| `sort(arr)` | Alias for sorted |
| `join(arr, sep)` | Join elements with separator |
| `slice(arr, start, end)` | Extract sub-array |
| `concat(arr1, arr2)` | Concatenate two arrays |
| `index_of(arr, val)` | First index (-1 if not found) |
| `includes(arr, val)` | Bool |
| `copy(arr)` | Shallow copy |
| `sum(arr)` | Sum of numeric elements |
| `enumerate(arr)` | Array of [index, value] pairs |
| `all(arr, fn)` | True if fn(elem) is true for every element |
| `any(arr, fn)` | True if fn(elem) is true for any element |
| `zip(arr1, arr2)` | Array of [a, b] pairs |
| `map(arr, fn)` | Transform each element |
| `filter(arr, fn)` | Keep elements where fn returns true |
| `reduce(arr, fn, init)` | Fold array to single value |

### 7.5 Math functions

| Function | Description |
|---|---|
| `sqrt(n)` | Square root |
| `pow(base, exp)` | Exponentiation |
| `abs(n)` | Absolute value |
| `floor(n)` | Round down |
| `ceil(n)` | Round up |
| `round(n)` | Round to nearest integer |
| `min(a, b)` | Minimum |
| `max(a, b)` | Maximum |
| `sin(n)` | Sine (radians) |
| `cos(n)` | Cosine (radians) |
| `tan(n)` | Tangent (radians) |
| `random()` | Random float in [0, 1) |

Math constants:

```killer
println(Math.PI)    -- 3.141592653589793
println(Math.E)     -- 2.718281828459045
```

Bitwise operations (on Number or Integer values):

| Function | Description |
|---|---|
| `bit_and(a, b)` | Bitwise AND |
| `bit_or(a, b)` | Bitwise OR |
| `bit_xor(a, b)` | Bitwise XOR |
| `bit_not(a)` | Bitwise NOT |
| `bit_shl(a, n)` | Left shift |
| `bit_shr(a, n)` | Right shift |
| `bit_rotl(a, n)` | Rotate left |
| `bit_rotr(a, n)` | Rotate right |

### 7.6 Dict functions

| Function | Description |
|---|---|
| `keys(d)` | Array of keys |
| `values(d)` | Array of values |
| `entries(d)` | Array of [key, value] pairs |
| `get(d, key)` | Value or null |
| `get(d, key, default)` | Value or default if key missing |
| `setdefault(d, key, default)` | Returns [updated_dict, value]; inserts default if key missing |

### 7.7 Set functions

| Function | Description |
|---|---|
| `set_new(v1, v2, ...)` | Create set with initial values |
| `set_from_array(arr)` | Create set from array |
| `set_add(s, val)` | Add value (mutates) |
| `set_remove(s, val)` | Remove value (mutates) |
| `set_has(s, val)` | Bool |
| `set_size(s)` | Count of elements |
| `set_to_array(s)` | Convert to sorted array |
| `set_union(s1, s2)` | Union |
| `set_intersection(s1, s2)` | Intersection |
| `set_difference(s1, s2)` | s1 minus s2 |
| `set_clear(s)` | Remove all elements |

### 7.8 Channel functions

Channels are backed by `std::sync::mpsc`. Safe to pass between spawned threads.

| Function | Description |
|---|---|
| `chan_new()` | Create unbounded channel; returns handle |
| `chan_send(ch, val)` | Send value |
| `chan_recv(ch)` | Receive (blocks until value available) |
| `chan_try_recv(ch)` | Receive without blocking; returns null if empty |
| `chan_close(ch)` | Close the channel |

```killer
let ch = chan_new()
async fn sender(ch) { chan_send(ch, 42) }
spawn sender(ch)
let msg = chan_recv(ch)
println(msg)    -- 42
```

### 7.9 HTTP

| Function | Description |
|---|---|
| `http_get(url)` | GET request; returns response body as string |
| `http_post(url, body)` | POST with plain body string |
| `http_post_json(url, json_str)` | POST with JSON content-type |
| `http_head(url)` | HEAD request; returns headers |
| `http_status(url)` | Returns HTTP status code as number |
| `http_download(url, path)` | Download URL to local file |

### 7.10 JSON

| Function | Description |
|---|---|
| `parse_json(s)` | Parse JSON string to Killer value |
| `json_stringify(val)` | Serialize to JSON string |
| `json_pretty(val)` | Serialize with indentation |

### 7.11 Crypto

| Function | Description |
|---|---|
| `sha256(s)` | SHA-256 of string; returns hex string |
| `sha256_bytes(s)` | SHA-256 of string; returns Bytes value |

Pure Rust implementation with no external crates.

### 7.12 Compression and encoding

| Function | Description |
|---|---|
| `compress(text, algo)` | Compress text; algo: "nova", "rle", or "lz77" |
| `decompress(data, algo)` | Decompress |
| `b64_encode(text)` | Base64 encode |
| `b64_decode(b64)` | Base64 decode |
| `hex_encode(text)` | Hex encode |
| `hex_decode(hex)` | Hex decode |
| `compress_ratio(orig, comp)` | orig_len / comp_len |
| `compress_info(text)` | Dict with sizes and ratios for all codecs |

### 7.13 System and process

| Function | Description |
|---|---|
| `system_time_ms()` | Current Unix time in milliseconds |
| `thread_sleep_ms(ms)` | Sleep for ms milliseconds |
| `cli_args()` | Array of command-line arguments |
| `env_get(name)` | Read environment variable |
| `env_set(name, val)` | Set environment variable |
| `process_exit(code)` | Exit the process with code |

### 7.14 Assertion (for tests)

| Function | Description |
|---|---|
| `assert_eq(a, b)` | Panic if a != b |
| `assert_ne(a, b)` | Panic if a == b |
| `assert_true(v)` | Panic if v is falsy |
| `assert_false(v)` | Panic if v is truthy |
| `assert_contains(arr, v)` | Panic if arr does not contain v |
| `assert_nil(v)` | Panic if v is not null |

### 7.15 Trit (balanced ternary)

| Constant or Function | Description |
|---|---|
| `T_NEG` | Trit value -1 |
| `T_ZERO` | Trit value 0 |
| `T_POS` | Trit value +1 |
| `trit_and(a, b)` | min(a, b) |
| `trit_or(a, b)` | max(a, b) |
| `trit_not(a)` | negation (-a) |
| `trit_add(a, b)` | clamp(a+b, -1, 1) |
| `trit_mul(a, b)` | a*b as trit |
| `trit_to_int(t)` | -1, 0, or 1 as Number |
| `trit_from_int(n)` | Number to Trit (sign) |
| `trit_to_str(t)` | "-", "0", or "+" |

---

## Section 8: Full Program Examples

### Example 1: CSV to Kore to SQL result

```killer
-- Ingest a CSV file into the Kore columnar format,
-- then run an aggregate query and print a table.

kore2_from_csv("sales.csv", "sales.kore")

let table = kore_query_table(
    "SELECT region, SUM(amount) as total FROM sales.kore GROUP BY region ORDER BY total DESC LIMIT 5"
)
println(table)
```

### Example 2: ACID transaction with time travel

```killer
-- Write two versions of a file, then read back version 1
-- and display a diff between versions.

kore_write("inventory.kore", "item:str,qty:int", "widget,100\ngadget,50")
let h1 = kore_txn_begin("inventory.kore")
let v1 = kore_txn_commit(h1, "initial stock")

kore_write("inventory.kore", "item:str,qty:int", "widget,120\ngadget,60\ndoohickey,30")
let h2 = kore_txn_begin("inventory.kore")
let v2 = kore_txn_commit(h2, "restock")

println("version 1 data:")
println(kore_checkout("inventory.kore", v1))

println("diff v1->v2:")
println(kore_diff("inventory.kore", v1, v2))
```

### Example 3: Uncertain values with set membership check

```killer
-- Declare a temperature reading with uncertainty.
-- Check if the rounded reading is in the acceptable range.

believe temp = 98.6 +/- 0.5
let allowed = set_new(97, 98, 99, 100)
let reading = int(temp)

if set_has(allowed, reading) {
    println("Normal temperature")
} else {
    println("Out of range: " + str(reading))
}
```

---

## Appendix A: Import system

```killer
import "math"                       -- import all from a module
import math { sqrt, pow }           -- selective import
export add, subtract                -- mark names as public from this module
```

## Appendix B: Range operator

`start..end` creates an iterable range. `start..end..step` adds a step.

```killer
for i in 1..6 { println(i) }        -- 1 2 3 4 5
for i in 10..0..-1 { println(i) }   -- 10 9 8 ... 1
```

## Appendix C: Comments

Killer accepts both `//` and `--` as line comment prefixes.

```killer
// this is a comment
-- so is this
```

## Appendix D: Spread operator

```killer
let a = [1, 2, 3]
let b = [...a, 4, 5]    -- [1, 2, 3, 4, 5]
```
