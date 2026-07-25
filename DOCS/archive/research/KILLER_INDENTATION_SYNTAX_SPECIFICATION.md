# Killer Indentation-Only Syntax Implementation Specification

## Vision
Transform Killer from brace-based syntax to **pure indentation-based syntax**, making it even simpler and more consistent with how modern languages work (Python, YAML, etc.).

---

## Current State vs Target

### BEFORE (Current - Brace-based)
```killer
kfn add(a, b) {
  a + b
}

for i in 1..5 {
  print(i)
}

if x > 0 {
  "positive"
}
```

### AFTER (Target - Indentation-only)
```killer
kfn add(a, b)
  a + b

for i in 1..5
  print(i)

if x > 0
  "positive"
```

---

## Parser Changes Required

### 1. Function Declarations
**Current:**
```killer
kfn function_name(params) { body }
```

**Target:**
```killer
kfn function_name(params)
  body
```

**Implementation:**
- Remove requirement for `{` after function signature
- Use indentation to detect start of function body
- Indentation level determines scope (standard: 2 spaces or 1 tab)

### 2. Actor Declarations
**Current:**
```killer
actor Worker {
  handle process(msg: String) { ... }
}
```

**Target:**
```killer
actor Worker
  handle process(msg: String)
    print("Processing")
```

**Implementation:**
- Actor keyword followed by name, then indented methods
- Each method indentation level 1
- Method bodies indentation level 2

### 3. Struct Declarations
**Current:**
```killer
struct Person {
  name: String
  age: Int
}
```

**Target:**
```killer
struct Person
  name: String
  age: Int
```

**Implementation:**
- Struct name followed by newline
- Fields indented (level 1)
- No braces required

### 4. Control Flow (if/else)
**Current:**
```killer
if condition {
  statement1
} else if condition2 {
  statement2
} else {
  statement3
}
```

**Target:**
```killer
if condition
  statement1
else if condition2
  statement2
else
  statement3
```

**Implementation:**
- `if/else if/else` at same indentation level
- Condition followed by newline
- Body indented one level deeper
- Maintain same indent for next `else if` or `else`

### 5. Loops (for/while)
**Current:**
```killer
for item in list {
  process(item)
}

while x > 0 {
  x = x - 1
}
```

**Target:**
```killer
for item in list
  process(item)

while x > 0
  x = x - 1
```

**Implementation:**
- Loop header followed by newline
- Body indented one level deeper
- Auto-detect scope end by dedent

### 6. Pattern Matching
**Current:**
```killer
match value {
  pattern1 -> result1
  pattern2 -> result2
}
```

**Target:**
```killer
match value
  pattern1 -> result1
  pattern2 -> result2
```

**Implementation:**
- `match` keyword followed by value, then newline
- Cases indented (level 1)
- Each case kept on one line with `->` arrow
- Auto-detect end by dedent or EOF

### 7. Block Expressions
**Current:**
```killer
let result = {
  a = 10
  b = 20
  a + b
}
```

**Target:**
```killer
result =
  a = 10
  b = 20
  a + b
```

**Implementation:**
- Assignment followed by newline
- Multi-statement blocks use indentation
- Last value is implicit return

---

## Indentation Rules

### Standard Unit
```
2 spaces = 1 indentation level (RECOMMENDED)
OR
1 tab = 1 indentation level (if tab-mode enabled)
```

### Rules
1. **Consistency**: All indentation in a file must use same style (2 spaces OR tabs)
2. **Scope**: Each deeper indentation level represents a nested scope
3. **Dedent = End**: When indentation decreases, all nested scopes end
4. **Blank lines**: Ignored (don't count as scope changes)
5. **Comments**: Ignored in indentation tracking
6. **One-liners**: Single statement functions allowed without indentation:
   ```killer
   kfn add(a, b) = a + b
   ```

### Ambiguity Prevention
1. **Mixed tabs/spaces**: ERROR - parser rejects
2. **Inconsistent dedent**: ERROR - parser rejects
3. **No indent after colon**: ERROR - parser expects newline + indent

---

## Parser Algorithm

### Tokenization Phase
```
1. Track indentation level at start of each line
2. Detect indent increase: emit INDENT token
3. Detect indent decrease: emit DEDENT tokens (1 per level)
4. Ignore blank lines and comments
```

### Example Tokenization
```killer
kfn add(a, b)      # INDENT_LEVEL: 0
  result = 10      # INDENT_LEVEL: 1 → emit INDENT
  x = 20           # INDENT_LEVEL: 1 (same)
  result           # INDENT_LEVEL: 1 (same)
                   # INDENT_LEVEL: 0 → emit DEDENT

for i in 1..5      # INDENT_LEVEL: 0
  print(i)         # INDENT_LEVEL: 1 → emit INDENT
                   # INDENT_LEVEL: 0 → emit DEDENT
```

### Parsing Phase
```
When INDENT is encountered:
  - Parser expects nested statements at deeper level
  - All statements at this level belong to current scope

When DEDENT is encountered:
  - Current scope ends
  - Parser returns to previous scope
  - DEDENT count indicates how many levels to pop
```

---

## Error Handling

### Parser Errors
```killer
# ERROR: Missing indent
kfn add(a, b)
result = 10  # ❌ No indent - parser error

# FIX:
kfn add(a, b)
  result = 10  # ✅ Properly indented
```

### ERROR: Inconsistent indentation
```killer
kfn test()
  x = 10
    y = 20  # ❌ ERROR: Unexpected extra indent (not 1 level deeper)
```

### ERROR: Mixed tabs and spaces
```killer
kfn test()      # Uses spaces
	print("hi")  # Uses tab - ERROR: Mixed indentation
```

### Clear Error Messages
```
Line 5, Col 1: IndentationError: Expected indent after function declaration "test"
Line 5, Col 1: IndentationError: Mixed tabs and spaces
Line 5, Col 1: IndentationError: Unexpected dedent (current=1, expected=0)
```

---

## Migration Strategy

### Phase 1: Parser Enhancement (Week 1)
- Add indentation tokenizer to lexer
- Add INDENT/DEDENT token types
- Modify parser to handle indentation
- Test with new syntax

### Phase 2: Update Examples (Week 2)
- Update all documentation (100+ examples)
- Update KILLER_VS_LANGUAGES_COMPARISON.md
- Update killer_rcore library examples
- Update teaching materials

### Phase 3: Backward Compatibility (Optional, Week 3)
- Maintain brace support as deprecated (warn on use)
- Or: Complete cleanup (remove braces entirely)
- Update migration guide

### Phase 4: Validation
- Test all 1,903 test cases with new syntax
- Verify performance unchanged
- Update parser error messages
- Publish language update (v4.1.1 or v4.2)

---

## Benefits of This Change

| Aspect | Improvement |
|--------|-------------|
| **Simplicity** | Remove visual clutter (`{}`) entirely |
| **Learning** | More intuitive for Python developers |
| **Readability** | Code flows naturally with indentation |
| **Consistency** | One way to structure code (indentation) |
| **UX** | Fewer syntax errors (no missing braces) |
| **Modern** | Aligns with Python, Rust (in some contexts), YAML |

---

## Syntax Examples - All 13 Features

### 1. Simple Functions
```killer
kfn add(a, b)
  a + b

result = add(5, 3)
print(result)
```

### 2. Loops
```killer
kfn loop_test()
  for i in 1..5
    print(i)

loop_test()
```

### 3. Conditionals
```killer
kfn check(x)
  if x > 0
    "positive"
  else if x < 0
    "negative"
  else
    "zero"

print(check(10))
```

### 4. Lists/Arrays
```killer
kfn list_test()
  list = [1, 2, 3, 4, 5]
  for item in list
    print(item)

list_test()
```

### 5. Maps/Dictionaries
```killer
kfn map_test()
  users = {"alice": 25, "bob": 30}
  age = users["alice"]
  print(age)

map_test()
```

### 6. Filter/Map
```killer
kfn filter_even()
  nums = [1, 2, 3, 4, 5, 6]
  evens = []
  for n in nums
    if n % 2 == 0
      evens.push(n)
  evens

print(filter_even())
```

### 7. Concurrency (Actors)
```killer
actor Worker
  handle process(msg: String)
    print("Processing: " + msg)

kfn main()
  w = Worker::spawn()
  w.process("task1")
  w.process("task2")

main()
```

### 8. Error Handling
```killer
kfn safe_divide(a, b)
  if b == 0
    "error"
  else
    a / b

print(safe_divide(10, 2))
print(safe_divide(10, 0))
```

### 9. Pattern Matching
```killer
kfn classify(x)
  match x
    0 -> "zero"
    1 -> "one"
    2 -> "two"
    _ -> "many"

print(classify(1))
```

### 10. Structs/Classes
```killer
struct Person
  name: String
  age: Int

kfn greet(p: Person)
  "Hello, " + p.name

p = Person(name: "Alice", age: 30)
print(greet(p))
```

### 11. Higher-Order Functions
```killer
kfn apply(f, x)
  f(x)

double = |x| x * 2
result = apply(double, 5)
print(result)
```

### 12. String Operations
```killer
kfn process_string(s)
  upper = s.to_upper()
  reversed = s.reverse()
  upper + " " + reversed

print(process_string("hello"))
```

### 13. Heavy Computation
```killer
kfn heavy_compute(n)
  sum = 0
  for i in 1..n
    sum = sum + (i * i)
  sum

result = heavy_compute(1000000)
print(result)
```

---

## Special Cases

### Multiple Statements in One Block
```killer
kfn multi_step()
  x = 10
  y = 20
  z = x + y
  print(z)
  z

result = multi_step()
```

### Nested Structures
```killer
for i in 1..3
  for j in 1..3
    if i == j
      print("matched")
    else
      print("no match")
```

### Conditional Returns
```killer
kfn check_range(x)
  if x < 0
    "negative"
  else if x > 100
    "too large"
  else
    "valid"
```

---

## Testing Checklist

- [ ] Lexer produces correct INDENT/DEDENT tokens
- [ ] Parser handles single-level indent
- [ ] Parser handles nested indents (3+ levels)
- [ ] Parser handles mixed structures (if/for/while)
- [ ] Dedent properly closes scopes
- [ ] Error messages clear and helpful
- [ ] All 1,903 existing tests pass
- [ ] Performance unaffected
- [ ] Actor syntax works
- [ ] Struct syntax works
- [ ] Pattern matching scopes correct
- [ ] Comments ignored in indentation
- [ ] Blank lines ignored
- [ ] Mixed tabs/spaces properly rejected

---

## Real World Example

### Before (Braces)
```killer
actor HTTPServer {
  handle request(req: Request) {
    if req.method == "GET" {
      handle_get(req)
    } else if req.method == "POST" {
      handle_post(req)
    } else {
      "405 Method Not Allowed"
    }
  }
  
  handle handle_get(req: Request) {
    match req.path {
      "/health" -> "OK"
      "/api/users" -> get_users()
      _ -> "404"
    }
  }
}
```

### After (Indentation-only)
```killer
actor HTTPServer
  handle request(req: Request)
    if req.method == "GET"
      handle_get(req)
    else if req.method == "POST"
      handle_post(req)
    else
      "405 Method Not Allowed"
  
  handle handle_get(req: Request)
    match req.path
      "/health" -> "OK"
      "/api/users" -> get_users()
      _ -> "404"
```

**Result:** 30% less visual noise, 100% more readable!

---

## Implementation Roadmap

```
Week 1: Parser Enhancement
├─ Lexer: Add indentation tracking
├─ Tokenizer: Emit INDENT/DEDENT
├─ Parser: Handle indentation in all contexts
└─ Testing: Unit tests for tokenizer

Week 2: Documentation & Examples
├─ Update KILLER_VS_LANGUAGES_COMPARISON.md
├─ Update all library examples
├─ Update teaching materials
└─ Create migration guide

Week 3: Validation & Release
├─ Run full test suite (1,903 tests)
├─ Performance verification
├─ Error message refinement
└─ Release v4.2 with indentation syntax
```

---

## Decision: Keep or Remove Braces?

### Option A: Full Migration (Recommended)
- Remove braces completely
- Updated parser only supports indentation
- Cleaner language, no ambiguity
- Deprecation path: 1 version cycle

### Option B: Support Both (Backward Compatible)
- Parser accepts both indentation and braces
- Issue warning when braces detected
- Allows gradual migration
- More complexity in parser

**RECOMMENDATION:** Option A - Full migration
- Simpler codebase
- Clear language semantics
- Aligns with modern language design
- One migration cycle (one version bump)

---

## Success Metrics

✅ **Language Design:**
- Pure indentation-based syntax (no braces)
- Consistent with Python/modern standards
- Clear, unambiguous grammar

✅ **Quality:**
- All 1,903 tests passing
- Zero performance impact
- Clear error messages for indentation issues

✅ **User Experience:**
- Zero `SyntaxError: unexpected {}`
- Self-explanatory code structure
- 20-30% less visual clutter

✅ **Documentation:**
- 100+ examples updated
- Migration guide published
- Teaching materials aligned

---

## Questions & Decisions

1. **Tab width**: 2 spaces (Python standard) or 1 tab?
   - **DECISION:** Support both, auto-detect, enforce consistency per file

2. **One-liners**: Allow `kfn add(a, b) = a + b`?
   - **DECISION:** Yes, allow for brevity on simple functions

3. **Comments**: How does indentation interact with comments?
   - **DECISION:** Comments are ignored, don't affect indentation tracking

4. **Implicit line continuation**: `x = 1 +\n  2` supported?
   - **DECISION:** Yes, backslash allows line continuation

5. **Brace support during transition**: Keep deprecated support?
   - **DECISION:** No - clean break. One version cycle, then remove.

