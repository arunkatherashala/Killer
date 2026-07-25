# Killer Hybrid Indentation Syntax - Specification

## Vision
**"Simple Use Indentation, Complex Use Braces When Needed"**

Transform Killer to make indentation the primary syntax with optional braces for complex nested structures. This gives us:
- ✅ Clean, simple code for everyday use
- ✅ Flexibility for complex logic
- ✅ Readability where it matters most
- ✅ Not dogmatic - pragmatic approach

---

## Design Philosophy

### Simple Code = Indentation ✨
```killer
# Simple function
kfn add(a, b)
  a + b

# Simple loop
for i in 1..5
  print(i)

# Simple conditional
if x > 0
  "positive"
else
  "negative"

# Simple actor
actor Worker
  handle process(msg: String)
    print("Processing: " + msg)
```

### Complex Code = Braces OK 🎯
```killer
# Complex pattern matching - braces help readability
process_request() =
  match request.method {
    "GET" -> handle_get()
    "POST" -> handle_post()
    "DELETE" -> handle_delete()
    _ -> error("405")
  }

# Complex nested structure
if check1() {
  if check2() {
    if check3() {
      do_something()
    }
  }
}

# One-liners OK
kfn add(a, b) = a + b
kfn double(x) = x * 2
```

---

## Rules

### RULE 1: Indentation is Primary
```killer
# ✅ PREFERRED - Clean indentation
kfn process(data)
  for item in data
    if item > 0
      print(item)
```

### RULE 2: Braces Optional for Clarity
```killer
# ✅ ALSO OK - Braces for complex match
kfn process(data)
  for item in data
    match item {
      positive -> print("pos")
      negative -> print("neg")
      _ -> print("zero")
    }
```

### RULE 3: One-Liners Allowed
```killer
# ✅ Simple one-liner
kfn add(a, b) = a + b

# ✅ Or with indentation
kfn add(a, b)
  a + b
```

### RULE 4: Mix & Match in Same File
```killer
# ✅ Function with indentation
kfn main()
  config = load_config()
  
  # ✅ Inline match with braces for clarity
  result = match config.mode {
    "dev" -> setup_dev()
    "prod" -> setup_prod()
    _ -> default_setup()
  }
  
  execute(result)
```

---

## Real-World Examples

### Example 1: HTTP Server (Indentation + Braces)
```killer
actor HTTPServer
  handle request(req)
    # Simple conditional - indentation
    if req.method == "GET"
      handle_get(req)
    else if req.method == "POST"
      handle_post(req)
    
    # Complex routing - braces help readability
    else
      match req.path {
        "/api/users" -> get_users()
        "/api/posts" -> get_posts()
        "/api/comments" -> get_comments()
        "/health" -> "OK"
        _ -> "404"
      }

  handle handle_get(req)
    # Simple for loop - indentation
    for header in req.headers
      process_header(header)
    
    return_response(200)
```

### Example 2: Data Processing (Mostly Indentation)
```killer
kfn process_pipeline(data)
  # Simple transformations - indentation
  filtered = []
  for item in data
    if validate(item)
      filtered.push(item)
  
  # Aggregate with braces for multi-case logic
  aggregated = match summary {
    count: filtered.length
    sum: aggregate_sum(filtered)
    average: aggregate_avg(filtered)
    groups: match filtered {
      positive: [x for x in filtered if x > 0]
      negative: [x for x in filtered if x < 0]
      zero: [x for x in filtered if x == 0]
    }
  }
  
  aggregated
```

### Example 3: Error Handling (Indentation)
```killer
kfn safe_operation(data)
  if not validate(data)
    return error("Invalid data")
  
  if not check_permissions(data)
    return error("No permission")
  
  if not check_quota(data)
    return error("Quota exceeded")
  
  # All checks passed
  execute(data)
```

---

## Syntax Rules by Context

### SIMPLE CONTEXTS → Indentation Required
```killer
# Functions
kfn name(params)
  body

# Loops
for item in list
  body

# While
while condition
  body

# Conditionals
if condition
  body
else if condition2
  body
else
  body

# Actors
actor Name
  handle method(params)
    body

# Structs
struct Name
  field1: Type
  field2: Type
```

### COMPLEX CONTEXTS → Indentation OR Braces
```killer
# Pattern matching - either works
# Option 1: Indentation
kfn classify(x)
  match x
    0 -> "zero"
    _ -> "many"

# Option 2: Braces (for complex logic)
kfn classify(x) =
  match x {
    0 -> handle_zero()
    1 -> handle_one()
    _ -> handle_many()
  }

# One-liners
kfn add(a, b) = a + b

# Inline expressions
result =
  if x > 0 {
    process_positive(x)
  } else {
    process_negative(x)
  }
```

---

## Migration Strategy

### Phase 1: Update Parser (Week 1)
- [ ] Add indentation tokenizer (same as before)
- [ ] Make braces optional (not required)
- [ ] Allow mixing indentation and braces
- [ ] Test all combinations

### Phase 2: Update Examples (Week 2)
- [ ] Update KILLER_VS_LANGUAGES_COMPARISON.md
  - Simple examples = indentation
  - Complex examples = can show braces
- [ ] Update all documentation
- [ ] Update teaching materials

### Phase 3: Validation (Week 3)
- [ ] Run all 1,903 tests
- [ ] Test mixed indentation/braces
- [ ] Performance validation

### Phase 4: Release (Week 4)
- [ ] Tag v4.2.0
- [ ] Publish release notes
- [ ] Community announcement

---

## Benefits of Hybrid Approach

| Aspect | Benefit |
|--------|---------|
| **Simplicity** | Most code is indentation-only (simple & clean) |
| **Flexibility** | Complex code can use braces when helpful |
| **Readability** | Writers choose best syntax for context |
| **Learning** | Easier entry (indentation), power users benefit (braces) |
| **Migration** | Gradual - keep braces in complex code, use indentation in new code |
| **Modern** | Pragmatic, not dogmatic (like well-designed languages) |

---

## Real Code Examples

### Current Killer (v4.1)
```killer
kfn fibonacci(n) {
  if n <= 1 {
    n
  } else {
    fibonacci(n - 1) + fibonacci(n - 2)
  }
}

actor Calculator {
  handle compute(expr: String) {
    result = parse(expr)
    match result {
      Ok(val) -> val
      Err(msg) -> "ERROR: " + msg
    }
  }
}
```

### Target Killer (v4.2 - Hybrid)
```killer
# Simple recursion - indentation
kfn fibonacci(n)
  if n <= 1
    n
  else
    fibonacci(n - 1) + fibonacci(n - 2)

# Actor with complex match - braces OK
actor Calculator
  handle compute(expr: String)
    result = parse(expr)
    match result {
      Ok(val) -> val
      Err(msg) -> "ERROR: " + msg
    }
```

**Result:** 
- Simple code (fibonacci) is 30% cleaner
- Complex code (match) readable with braces
- Best of both worlds! 🎯

---

## Comparison: Pure Indentation vs Hybrid

### Pure Indentation (Original Spec)
```killer
match x
  0 -> "zero"
  1 -> "one"
  2 -> "two"
  _ -> "many"
```
✅ Clean  
❌ Indentation scope ambiguous for complex cases

### Hybrid (This Spec)
```killer
match x {
  0 -> "zero"
  1 -> "one"
  2 -> "two"
  _ -> "many"
}
```
✅ Clean indentation for body  
✅ Braces clarify matching scope  
✅ Best readability for complex logic

---

## Implementation Details

### Tokenizer
- Same as pure indentation spec
- Emit INDENT/DEDENT tokens
- Also emit `{` and `}` tokens

### Parser Rules
```
# These MUST use indentation
function_def → "kfn" name NEWLINE INDENT body DEDENT
for_stmt → "for" pat "in" expr NEWLINE INDENT body DEDENT
while_stmt → "while" cond NEWLINE INDENT body DEDENT
if_stmt → "if" cond NEWLINE INDENT body DEDENT

# These CAN use indentation OR braces
match_expr → "match" expr (match_indent | match_braces)
match_indent → NEWLINE INDENT cases DEDENT
match_braces → "{" cases "}"
```

### No Changes Needed
- All other syntax remains same
- Performance unaffected
- Test suite adaptable

---

## Error Messages

```
# Missing indent after function declaration
✗ Error: Expected INDENT after "kfn add(a, b)"
  Line 1:   kfn add(a, b)
  Line 2:   result = 10    <- ✗ Same level, no indent

✓ Fix:
  kfn add(a, b)
    result = 10    <- ✓ Properly indented

# Mixed indentation
✗ Error: Mixed tabs and spaces (file uses spaces)
  Line 5:	  print("hi")  <- ✗ Tab detected

✓ Fix: Use consistent spacing (2 spaces recommended)
```

---

## Design Decisions Made

| Decision | Value | Rationale |
|----------|-------|-----------|
| **Indentation Primary?** | YES | Simple code should be clean |
| **Braces Optional?** | YES | Flexibility for complex code |
| **Both in Same File?** | YES | Pragmatic - writer chooses best syntax |
| **Tab Width?** | 2 spaces (default) | Python standard |
| **One-liners?** | YES | `kfn f(x) = x + 1` allowed |
| **Breaking Change?** | No braces required | Indentation works, braces optional |

---

## Success Metrics

✅ **Code Quality:**
- Indentation primary in 80%+ of code
- Braces used intentionally for complex logic
- Code readability improved

✅ **Developer Experience:**
- New developers: learn indentation first
- Advanced: use braces when beneficial
- No frustration with "must use braces"

✅ **Performance:**
- Zero performance impact
- Same parsing speed
- Same runtime speed

✅ **Testing:**
- All 1,903 tests pass
- Both indentation and braces tested
- Mixed usage validated

---

## Summary

**THIS IS THE BEST APPROACH** because:

1. **Simple by default** - Most everyday code uses clean indentation
2. **Powerful when needed** - Complex code can use braces for clarity
3. **Pragmatic** - Not dogmatic about indentation-only
4. **Gradual migration** - Can keep braces in existing code
5. **Familiar** - Combines best of Python (indentation) + Rust (optional braces)
6. **Beautiful code** - Naturally encourages simple functions

This is how **professional languages evolve** - flexible enough for real-world use! 🎯

