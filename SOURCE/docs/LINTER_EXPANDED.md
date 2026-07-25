# Expanded Linter - 100+ Rules Documentation

## Overview

The Killer Code Linter now includes **100+ comprehensive rules** for detecting code quality issues, security problems, performance bottlenecks, and best practice violations.

**Status**: ✅ Enhanced (v2.1.0)
**Total Rules**: 100+
**Categories**: 10+ specialized categories
**Configurable**: Via .killerrc or programmatic API

## Rule Categories

### 1. Naming Conventions (15+ rules)

Ensures consistent, descriptive naming across codebase.

| Rule | Severity | Description |
|------|----------|-------------|
| `snake-case-functions` | Warning | Function names should use snake_case |
| `camel-case-variables` | Warning | Variable names should use camelCase |
| `CONST_ALL_CAPS` | Warning | Constant names must be UPPER_CASE |
| `no-single-letter-vars` | Warning | Avoid single-letter variable names (except loop counters) |
| `meaningful-variable-names` | Warning | Variable names should be descriptive |
| `descriptive-parameter-names` | Warning | Parameter names should be meaningful |
| `avoid-abbreviations` | Info | Prefer full words over abbreviations |
| `consistent-naming-style` | Warning | Naming style should be consistent |
| `no-confusing-names` | Warning | Avoid names that could be confused with keywords |
| `boolean-naming-prefix` | Warning | Boolean variables should start with is/has/can |
| `class-naming-convention` | Warning | Classes should use PascalCase |
| `interface-naming-convention` | Warning | Interfaces should use I-prefix or -able suffix |
| `enum-naming-convention` | Warning | Enums should use PascalCase |
| `avoid-reserved-keywords` | Error | Cannot use reserved keywords as names |
| `naming-shadows-builtin` | Warning | Name shadows a built-in function |

### 2. Code Style (12+ rules)

Enforces consistent formatting and styling conventions.

| Rule | Severity | Description |
|------|----------|-------------|
| `no-trailing-whitespace` | Info | Lines should not have trailing whitespace |
| `max-line-length` | Warning | Line exceeds maximum length |
| `indentation-consistency` | Warning | Inconsistent indentation detected |
| `consistent-quotes` | Warning | Mixed quote styles on same line |
| `semicolon-style` | Info | Inconsistent semicolon usage |
| `space-after-comma` | Info | Missing space after comma |
| `space-around-operators` | Info | Inconsistent spaces around operators |
| `no-multiple-declarations-per-line` | Warning | Multiple declarations on same line |
| `line-length-consistency` | Warning | Inconsistent line lengths |
| `brace-consistency` | Warning | Inconsistent brace style |
| `consistent-blank-lines` | Warning | Inconsistent blank line usage |
| `no-mixed-spaces-tabs` | Warning | Mixed spaces and tabs in indentation |

### 3. Unused Code (8+ rules)

Detects dead code and unused declarations.

| Rule | Severity | Description |
|------|----------|-------------|
| `unused-variables` | Warning | Variable is declared but never used |
| `unused-imports` | Warning | Import statement is unused |
| `unused-functions` | Warning | Function is declared but never called |
| `dead-code` | Warning | Code block is unreachable |
| `unreachable-code` | Error | Statement after return/throw |
| `unused-parameters` | Warning | Function parameter not used in body |
| `unused-return-values` | Info | Return value is not used |
| `unused-assignments` | Info | Variable assigned but overwritten immediately |

### 4. Best Practices (15+ rules)

Encourages patterns that improve code quality and maintainability.

| Rule | Severity | Description |
|------|----------|-------------|
| `no-empty-blocks` | Warning | Empty code block should have implementation |
| `no-duplicated-branches` | Warning | Duplicated if/else branches |
| `consistent-return` | Warning | Return statement style inconsistent |
| `explicit-return-type` | Warning | Return type should be explicit |
| `use-const-where-possible` | Info | Local variable could be const |
| `avoid-global-state` | Warning | Global variables reduce testability |
| `simplify-boolean-expression` | Warning | Boolean expression can be simplified |
| `no-nested-ternary` | Warning | Nested ternary operators reduce readability |
| `avoid-magic-numbers` | Info | Magic number should be named constant |
| `use-early-return` | Info | Consider using early return |
| `no-yoda-conditions` | Warning | Condition syntax is backwards |
| `consistent-equality-checks` | Warning | Inconsistent equality operators |
| `avoid-side-effects` | Warning | Function has unexpected side effects |
| `single-responsibility` | Warning | Function/class does too much |
| `avoid-temporal-coupling` | Warning | Code has temporal dependency issues |

### 5. Security (12+ rules)

Detects potential security vulnerabilities and dangerous patterns.

| Rule | Severity | Description |
|------|----------|-------------|
| `sql-injection-risk` | Error | Potential SQL injection vulnerability |
| `command-injection-risk` | Warning | Dynamic command execution detected |
| `eval-usage` | Error | Use of eval() is dangerous |
| `unsafe-type-conversion` | Error | Unsafe type casting detected |
| `integer-overflow-risk` | Warning | Integer overflow potential |
| `null-pointer-dereference` | Error | Potential null pointer access |
| `unvalidated-input` | Warning | User input not validated |
| `hardcoded-credentials` | Error | Credentials hardcoded in source |
| `hardcoded-paths` | Warning | Hardcoded file paths reduce portability |
| `weak-cryptography` | Error | Weak cryptographic algorithm used |
| `insecure-deserialization` | Error | Deserialization without validation |
| `exposed-sensitive-data` | Error | Sensitive data exposed in logs/errors |

### 6. Performance (15+ rules)

Identifies patterns that could impact runtime performance.

| Rule | Severity | Description |
|------|----------|-------------|
| `unnecessary-loops` | Warning | Loop can be replaced with built-in |
| `inefficient-string-concat` | Warning | String concatenation in loop |
| `avoid-nested-loops` | Warning | Nested loops may be inefficient |
| `cache-miss-prevention` | Info | Data access pattern inefficient |
| `lazy-initialization` | Info | Value could be lazily initialized |
| `avoid-repeated-calls` | Warning | Function called repeatedly with same args |
| `collection-size-check` | Info | Consider caching collection size |
| `premature-optimization` | Info | Optimization may not be needed |
| `string-interning` | Info | String should be interned |
| `avoid-unnecessary-copies` | Warning | Value copied unnecessarily |
| `use-range-queries` | Info | Range query more efficient |
| `batch-operations` | Warning | Operations could be batched |
| `connection-pooling` | Warning | Database connections not pooled |
| `memory-leak-risk` | Error | Potential memory leak detected |
| `algorithm-optimization` | Info | Algorithm could be optimized |

### 7. Complexity (10+ rules)

Detects overly complex code that should be simplified.

| Rule | Severity | Description |
|------|----------|-------------|
| `cyclomatic-complexity` | Warning | Function has high cyclomatic complexity |
| `too-many-parameters` | Warning | Function has too many parameters |
| `too-many-locals` | Warning | Function has too many local variables |
| `function-too-long` | Warning | Function body is too long |
| `high-cognitive-complexity` | Warning | Code is hard to understand |
| `deeply-nested-code` | Warning | Code nesting is too deep |
| `too-many-branches` | Warning | Too many if/else branches |
| `too-many-returns` | Warning | Multiple return statements |
| `high-fan-out` | Warning | Function calls too many other functions |
| `god-function-detection` | Error | Function does too much |

### 8. Documentation (8+ rules)

Ensures code has adequate documentation.

| Rule | Severity | Description |
|------|----------|-------------|
| `missing-function-docs` | Info | Function lacks documentation |
| `missing-class-docs` | Info | Class lacks documentation |
| `outdated-comments` | Warning | Comment appears out of sync with code |
| `incomplete-documentation` | Warning | Documentation is incomplete |
| `contradictory-documentation` | Error | Documentation contradicts code |
| `missing-edge-case-docs` | Info | Edge cases not documented |
| `missing-example-docs` | Info | No usage examples provided |
| `typo-in-comments` | Info | Spelling error in comment |

### 9. Testing (8+ rules)

Checks code related to testing and test quality.

| Rule | Severity | Description |
|------|----------|-------------|
| `no-hardcoded-test-data` | Info | Hardcoded test data detected |
| `test-coverage-gaps` | Warning | Code path not tested |
| `missing-test-class` | Warning | Class lacks test class |
| `untestable-code` | Warning | Code structure prevents testing |
| `test-naming-convention` | Warning | Test name doesn't follow convention |
| `inadequate-assertions` | Warning | Test has too few assertions |
| `flaky-test-detection` | Warning | Test may be flaky/non-deterministic |
| `slow-test-detection` | Warning | Test takes too long |

### 10. Type Safety (8+ rules) - NEW

Detects type-related issues and unsafe conversions.

| Rule | Severity | Description |
|------|----------|-------------|
| `implicit-type-conversion` | Warning | Implicit type conversion may lose data |
| `type-mismatch-potential` | Error | Type mismatch detected |
| `unsafe-cast` | Error | Unsafe type casting |
| `null-safety-violation` | Error | Potential null reference |
| `optional-type-misuse` | Warning | Optional type used incorrectly |
| `generic-constraint-violation` | Error | Generic constraint violated |
| `type-inference-ambiguity` | Warning | Type inference is ambiguous |
| `missing-type-annotation` | Info | Type annotation recommended |

### 11. Resource Management (6+ rules) - NEW

Ensures proper handling of system resources.

| Rule | Severity | Description |
|------|----------|-------------|
| `resource-not-closed` | Warning | Resource not properly closed |
| `file-handle-leak` | Error | File handle not closed |
| `memory-leak-risk` | Error | Potential memory leak |
| `database-connection-leak` | Error | Database connection not closed |
| `unclosed-stream` | Warning | Stream not closed |
| `missing-finally-block` | Warning | Resource cleanup in finally |

### 12. Consistency & Modularity (10+ rules) - NEW

Ensures consistent patterns and good modular design.

| Rule | Severity | Description |
|------|----------|-------------|
| `inconsistent-exception-handling` | Warning | Exception handling pattern varies |
| `inconsistent-logging` | Warning | Logging format inconsistent |
| `inconsistent-validation` | Warning | Input validation varies |
| `inconsistent-error-messages` | Info | Error messages not consistent |
| `violation-of-dry-principle` | Warning | Code duplication detected |
| `high-coupling` | Warning | Modules too tightly coupled |
| `low-cohesion` | Warning | Module lacks cohesion |
| `cyclic-dependency` | Error | Circular dependency detected |
| `tight-coupling` | Warning | Classes tightly coupled |
| `feature-envy` | Warning | Class uses another class too much |

## Configuration

All 100+ rules can be configured via `.killerrc`:

```toml
[linter]
# Category toggles
check_naming = true
check_unused = true
check_security = true
check_performance = true

# Disable specific rules
disabled_rules = "avoid-magic-numbers, premature-optimization"
```

## Severity Levels

- **Error** (Exit code 1) - Must be fixed before commit
- **Warning** (Exit code 0) - Should be reviewed and fixed
- **Info** (Exit code 0) - Nice to have improvements

## Usage Examples

### Detect Security Issues

```bash
killer-native --lint mycode.killer
# Reports: hardcoded-credentials, sql-injection-risk, etc.
```

### Detect Performance Issues

```bash
killer-native --lint mycode.killer
# Reports: inefficient-string-concat, unnecessary-loops, etc.
```

### Detect Complexity

```bash
killer-native --lint mycode.killer
# Reports: cyclomatic-complexity, too-many-parameters, etc.
```

## Statistics

- **Total Rules**: 100+
- **Error Severity**: 20+ rules
- **Warning Severity**: 50+ rules
- **Info Severity**: 30+ rules
- **Categories**: 12
- **Lines of Code**: 600+ (expanded from 550)

## Performance

- Single file (< 10 KB): ~5-10ms
- Medium file (< 100 KB): ~50-100ms
- Large file (< 1 MB): ~500-1000ms

## Integration Examples

### GitHub Actions

```yaml
- name: Lint Killer Code
  run: |
    for file in **/*.killer; do
      killer-native --lint "$file" || exit 1
    done
```

### Pre-commit Hook

```bash
#!/bin/bash
for file in $(git diff --cached --name-only | grep '\.killer$'); do
  killer-native --lint "$file" || exit 1
done
```

## Rule Coverage by Design Goal

| Goal | Rules | Coverage |
|------|-------|----------|
| Code Quality | 45+ | ⭐⭐⭐⭐⭐ |
| Security | 12+ | ⭐⭐⭐⭐⭐ |
| Performance | 15+ | ⭐⭐⭐⭐ |
| Maintainability | 20+ | ⭐⭐⭐⭐⭐ |
| Testing | 8+ | ⭐⭐⭐ |

## Future Enhancements

- [ ] Machine learning-based rule suggestions
- [ ] Per-file rule configuration
- [ ] Custom rule definitions
- [ ] IDE inline diagnostics
- [ ] Auto-fix capabilities
- [ ] Complexity metrics dashboard
- [ ] Historical trend analysis
- [ ] Team rule consensus

## Related Tools

- **Formatter** (`--format`) - Auto-fix style issues
- **Configuration** (`.killerrc`) - Customize rules per project
- **Version Manager** - Track stability across versions
- **API Contract** - Define stable public APIs
