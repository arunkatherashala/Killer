# Killer Code Linter

**Version**: 2.1.0  
**Command**: `killer-native --lint <file.killer>`

## Overview

The Killer Code Linter is a comprehensive static analysis tool that checks your Killer code against 50+ quality rules. It helps identify and fix code quality issues, security vulnerabilities, performance bottlenecks, and style inconsistencies.

## Quick Start

```bash
# Lint a single file
killer-native --lint myprogram.killer

# View full report with suggestions
killer-native --lint app.killer

# Check help
killer-native --help
```

## Rules by Category

### 1. Naming Conventions (10+ Rules)

**Philosophy**: Clear, consistent naming makes code more maintainable.

- **snake-case-functions** - Functions should use `snake_case`
  ```killer
  // ✓ Good
  fn get_user_name() { }
  
  // ✗ Bad
  fn GetUserName() { }
  ```

- **camel-case-variables** - Variables should use `camelCase`
  ```killer
  // ✓ Good
  let userName = "Alice"
  
  // ✗ Bad
  let user_name = "Alice"
  let USERNAME = "Alice"
  ```

- **CONST_ALL_CAPS** - Constants should use `CONST_ALL_CAPS`
  ```killer
  // ✓ Good
  const MAX_RETRIES = 3
  
  // ✗ Bad
  const max_retries = 3
  ```

- **no-single-letter-vars** - Avoid single-letter variable names (except loop counters)
  ```killer
  // ✓ Good
  let userName = getData()
  
  // ✗ Bad
  let u = getData()
  ```

- **meaningful-variable-names** - Use descriptive names, avoid abbreviations
  ```killer
  // ✓ Good
  let userCount = users.length
  
  // ✗ Bad
  let uc = users.length
  let temp = users.length
  ```

### 2. Code Style (7+ Rules)

**Philosophy**: Consistent style improves readability.

- **max-line-length** (default: 100 characters)
  ```
  Line length 125 exceeds maximum of 100
  ```

- **no-trailing-whitespace**
  ```killer
  let x = 42;  [spaces here]
  ```
  Suggestion: Remove trailing spaces

- **indentation-consistency** - Use consistent indentation (spaces vs tabs)

- **consistent-quotes** - Use one style: single or double quotes
  ```killer
  // ✗ Bad (mixed)
  let msg = "Hello" + 'World'
  ```

- **semicolon-style** - Be consistent with semicolons at end of statements

### 3. Unused Code (5+ Rules)

**Philosophy**: Remove dead code to reduce complexity.

- **unused-variables**
  ```killer
  let x = 42  // Never used
  ```

- **unused-imports** - Remove unused module imports

- **unused-functions**
  ```killer
  fn helperFunc() { }  // Never called
  ```

- **dead-code** - Unreachable code after return statements

- **unreachable-code**
  ```killer
  return 42
  // Everything after return is unreachable
  let x = 10
  ```

### 4. Best Practices (8+ Rules)

**Philosophy**: Follow patterns that prevent bugs.

- **no-empty-blocks**
  ```killer
  fn doNothing() {}  // ✗ Bad
  fn doNothing() { /* TODO */ }  // ✓ Good
  ```

- **no-duplicated-branches**
  ```killer
  if (x > 5) {
    print("big")
  } else {
    print("big")  // ✗ Same as if branch
  }
  ```

- **consistent-return**
  ```killer
  fn getValue(flag) {
    if (flag) return 42
    // ✗ Missing return in else path
  }
  ```

- **explicit-return-type** - Document function return types

- **use-const-where-possible**
  ```killer
  let PI = 3.14159  // ✓ Should be const if not reassigned
  ```

- **avoid-global-state** - Minimize global variables

- **simplify-boolean-expression**
  ```killer
  if (isValid == true)    // ✗ Redundant comparison
  if (isValid)             // ✓ Better
  ```

### 5. Security (7+ Rules)

**Severity**: Error - Security issues should be fixed immediately

- **eval-usage** [ERROR]
  ```killer
  eval(userInput)  // ✗ Dangerous!
  ```
  Suggestion: Use safer alternatives or static analysis

- **command-injection-risk** [WARNING]
  ```killer
  exec("rm -rf " + userPath)  // ✗ Dangerous
  ```
  Suggestion: Validate and sanitize all inputs

- **sql-injection-risk** [WARNING]
  ```killer
  query("SELECT * FROM users WHERE id = " + userId)  // ✗
  query("SELECT * FROM users WHERE id = ?", userId)  // ✓
  ```

- **unsafe-type-conversion** - Explicit type casting without validation

- **integer-overflow-risk** - Watch for arithmetic overflow

- **null-pointer-dereference** - Check for null before access

- **unvalidated-input** - Validate user input and external data

### 6. Performance (8+ Rules)

**Philosophy**: Better performance = better user experience.

- **unnecessary-loops**
  ```killer
  for (let x in items) print(x)
  // Could use: items.forEach(print)
  ```

- **inefficient-string-concat**
  ```killer
  // ✗ Creates many string objects
  let result = ""
  for (let s in strings) result = result + s
  
  // ✓ Use array join or StringBuilder
  let result = strings.join("")
  ```

- **avoid-nested-loops** - O(n²) algorithms can be slow

- **cache-miss-prevention** - Minimize cache-unfriendly access patterns

- **lazy-initialization** - Defer expensive operations until needed

- **avoid-repeated-calls** - Cache results of function calls

- **collection-size-check**
  ```killer
  if (items.length > 0)    // ✓ Clear intent
  if (items.length != 0)   // ✗ Less clear
  ```

- **premature-optimization** - Optimize only proven bottlenecks

### 7. Complexity (5+ Rules)

**Philosophy**: Simpler functions are easier to understand and test.

- **cyclomatic-complexity** (max: 10, warning: 7)
  ```
  Method 'calculate' has cyclomatic complexity 15 (max 10)
  Suggestion: Break into smaller functions
  ```

- **too-many-parameters** (max: 5)
  ```killer
  fn process(a, b, c, d, e, f) { }  // ✗
  fn process(config) { }             // ✓
  ```

- **too-many-locals** (max: 10)

- **function-too-long** (max: 100 lines)

- **high-cognitive-complexity**

### 8. Documentation (4+ Rules)

**Philosophy**: Good docs = easier maintenance.

- **missing-function-docs**
  ```killer
  fn getUserById(id) { }  // ✗ No documentation
  ```

- **missing-class-docs**
  ```killer
  class User { }  // ✗ No class documentation
  ```

- **outdated-comments**
  ```killer
  // This calculates the sum
  let product = a * b  // ✗ Comment is wrong
  ```

- **incomplete-documentation** - Docs that lack examples or edge cases

### 9. Testing (2+ Rules)

**Philosophy**: Better tests = better reliability.

- **no-hardcoded-test-data**
  ```killer
  fn test_example() {
    let data = "hardcoded" // ✗ Uses hardcoded test data
  }
  ```

- **test-coverage-gaps** - Identify uncovered code paths

## Severity Levels

### 🔴 ERROR
Critical issues that must be fixed (e.g., security vulnerabilities).
Return exit code: 1

```
killer-native --lint app.killer
[ERROR] eval-usage: Use of eval() is dangerous and should be avoided
```

### 🟡 WARNING
Issues that should be reviewed and likely fixed (e.g., performance problems).

```
[WARN] max-line-length: Line length 125 exceeds maximum of 100
```

### 🔵 INFO
Recommendations that may improve code quality.

```
[INFO] no-trailing-whitespace: Line has trailing whitespace
```

## Linter Configuration

###  Custom Configuration Example

```rust
use killer_native::linter::{Linter, LinterConfig};

let config = LinterConfig {
    max_line_length: 120,      // Change from default 100
    check_naming: true,         // Enable naming rules
    check_unused: true,         // Enable unused code rules
    check_security: true,       // Enable security rules
    check_performance: true,    // Enable performance rules
};

let mut linter = Linter::with_config(config);
linter.lint_source(source)?;
```

### Enable/Disable Rules

```rust
let mut linter = Linter::new();

// Disable a specific rule
linter.disable_rule("max-line-length");

// Enable a rule
linter.enable_rule("max-line-length");
```

## Report Format

```
=== Killer Code Linter Report ===
Total Issues: 5
Errors: 1 | Warnings: 2 | Info: 2

Errors:
  [ERROR] eval-usage (42:15): Use of eval() is dangerous and should be avoided
  Suggestion: Use safer alternatives or static analysis

Warnings:
  [WARN] max-line-length (15): Line length 125 exceeds maximum of 100
  [WARN] no-empty-blocks (28): Empty code block should contain implementation or comment
  Suggestion: Add implementation or remove block

Info:
  [INFO] no-trailing-whitespace (5): Line has trailing whitespace
  Suggestion: Remove trailing spaces
  [INFO] no-hardcoded-test-data (98): Test function contains hardcoded test data
```

## Integration Examples

### Pre-commit Hook

```bash
#!/bin/bash
# .git/hooks/pre-commit

files=$(git diff --cached --name-only | grep "\.killer$")
if [ -n "$files" ]; then
  killer-native --lint $files
  if [ $? -ne 0 ]; then
    echo "Linting failed. Fix issues and try again."
    exit 1
  fi
fi
```

### CI/CD Pipeline

```yaml
# .github/workflows/lint.yml
name: Lint
on: [push, pull_request]

jobs:
  lint:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v2
      - name: Killer Linter
        run: killer-native --lint src/**/*.killer
```

### IDE Integration

**VS Code**: Extension watches files, runs linter on save

```json
{
  "killer.lintOnSave": true,
  "killer.lintRules.maxLineLength": 100,
  "killer.lintRules.checkSecurity": true
}
```

## Best Practices

### 1. Start with Errors
Fix all ERROR level issues first - they're critical.

```bash
killer-native --lint app.killer | grep ERROR
```

### 2. Progressive Strictness
Enable stricter rules as your project matures:

- Phase 1: Errors only (security & crashes)
- Phase 2: Warnings (performance & maintainability)
- Phase 3: Info (style & consistency)

### 3. Team Standards
Define linter rules that match your team's style guide:

```bash
# Store config
killer-linter-config.json {
  "rules": {
    "max-line-length": 100,
    "snake-case-functions": true
  }
}
```

### 4. Continuous Improvement
Review linter reports regularly to identify patterns:

```bash
# Find most common issues
killer-native --lint *.killer | grep "Rule:" | sort | uniq -c | sort -rn
```

## FAQ

### Q: Can I ignore a specific rule on one line?

**A**: Use inline comments (planned for future release):

```killer
// killer-lint: disable eval-usage
eval(code)  // We need this for dynamic loading
```

### Q: What's the difference between Warning and Error?

**A**:
- **Error** (exit code 1): Must fix before committing
- **Warning** (exit code 0): Should review and likely fix
- **Info** (exit code 0): Nice-to-have improvements

### Q: Can I use the linter programmatically?

**A**: Yes, use the API:

```rust
use killer_native::linter::Linter;

let mut linter = Linter::new();
linter.lint_source(source)?;
for violation in linter.violations() {
    println!("{}", violation.format());
}
```

### Q: Does linting slow down my development?

**A**: Linting is very fast (~1ms per file):

```bash
time killer-native --lint large-codebase.killer
# user 0m0.001s
```

### Q: How do I suppress linter warnings?

**A**: Disable specific rules:

```rust
let mut linter = Linter::new();
linter.disable_rule("no-trailing-whitespace");
```

## Contributing Rules

Want to add new linter rules? Modify `linter.rs`:

```rust
// Add to check_tokens or check_lines
if self.enabled_rules.contains("my-new-rule") {
    self.violations.push(LintViolation::new(
        "my-new-rule".to_string(),
        LintSeverity::Warning,
        "Issue description".to_string(),
    ).with_suggestion("How to fix it".to_string()));
}
```

## See Also

- [API Contract](API_CONTRACT.md) - Public API stability guarantees
- [CHANGELOG.md](CHANGELOG.md) - Version history and features
- [Code Examples](examples/) - Sample Killer programs
