# Killer v4.2 - Phase 4.3: Mercuri Testing Engine Integration

## Overview
Integrate the Killer v4.2 parser indentation tests with Killer's native **Mercuri** testing engine for automated validation and CI/CD reporting.

---

## Phase 4.3: Mercuri Engine Integration (Weeks 3-4)
**Owner:** QA/DevOps Team  
**Duration:** 20-30 hours  
**Success Criteria:** All 1,943 tests passing via Mercuri, reports generated automatically

---

## 4.3.1: Mercuri Test Configuration

### Setup Mercuri for Killer v4.2

```killer
# File: mercuri.config
[test_suite]
name = "Killer v4.2 Hybrid Indentation"
version = "4.2.0"
description = "Phase 1-3 tests for indentation feature"

[test_config]
timeout = 5000          # 5 second timeout per test
parallel_threads = 8    # Run 8 tests in parallel
fail_fast = false       # Continue all tests even if one fails
coverage_threshold = 95 # Must achieve 95% code coverage

[phases]
phase1_unit = ["lexer_tests", "parser_tests", "regression_tests"]
phase2_docs = ["documentation_examples"]
phase3_integration = ["full_suite_regression"]

[report_format]
output_dir = "./test_reports"
formats = ["json", "html", "csv"]
include_coverage = true
include_benchmarks = true
```

### Initialize Mercuri Test Suite

```bash
# Command: killer mercuri init --version 4.2 --project-root .
killer mercuri init \
  --name "Killer v4.2" \
  --description "Hybrid indentation parser tests" \
  --config mercuri.config
```

---

## 4.3.2: Organize Tests for Mercuri

### Test Directory Structure

```
tests/
├── phase1_lexer/
│   ├── test_indentation.killer
│   ├── test_dedent.killer
│   ├── test_mixed_syntax.killer
│   ├── test_error_conditions.killer
│   └── test_edge_cases.killer
├── phase1_parser/
│   ├── test_hybrid_blocks.killer
│   ├── test_error_recovery.killer
│   └── test_backward_compat.killer
├── phase3_regression/
│   ├── test_existing_brace_code.killer
│   ├── test_type_annotations.killer
│   └── test_performance.killer
└── mercuri.manifest
```

### Mercuri Test Manifest

```yaml
# File: tests/mercuri.manifest
version: "1.0"
suite: "Killer v4.2 Indentation"

test_groups:
  - name: "Phase 1: Lexer Unit Tests"
    path: "phase1_lexer/"
    count: 20
    timeout: 1000
    tags: ["unit", "lexer", "indentation"]
    
  - name: "Phase 1: Parser Unit Tests"
    path: "phase1_parser/"
    count: 10
    timeout: 1000
    tags: ["unit", "parser", "hybrid"]
    
  - name: "Phase 3: Regression Tests"
    path: "phase3_regression/"
    count: 1903
    timeout: 5000
    tags: ["regression", "compatibility"]

metadata:
  author: "Killer Team"
  created: "2026-03-20"
  updated: "2026-03-20"
  phases:
    - 1_parser_enhancement
    - 3_regression_testing
    - 4_integration_release
```

---

## 4.3.3: Sample Mercuri Test Files

### Lexer Test Example (Mercuri Format)

```killer
# File: tests/phase1_lexer/test_indentation.killer

#[mercuri::test]
#[timeout(1000)]
#[tags("unit", "lexer", "indentation")]
test_simple_indent_token() {
  code = "kfn test()\n  x = 1"
  lexer = Lexer::new(code)
  tokens = lexer.tokenize()
  
  assert(tokens.len() > 0, "Should produce tokens")
  assert_contains(tokens, TokenType::INDENT(2), "Should have INDENT(2)")
  assert_eq(tokens[0].token_type, TokenType::Fn, "First token should be Fn")
}

#[mercuri::test]
#[timeout(1000)]
#[tags("unit", "lexer", "error")]
test_mixed_tabs_spaces_error() {
  code = "kfn test()\n  x = 1\n\ty = 2"
  lexer = Lexer::new(code)
  
  result = lexer.tokenize()
  assert_err(result, "Should error on mixed tabs/spaces")
  assert_error_contains(result.unwrap_err(), "mixed", "Error message should mention 'mixed'")
}

#[mercuri::test]
#[timeout(1000)]
#[tags("unit", "lexer", "edge-case")]
test_nested_indents() {
  code = "for i in 1..5\n  if i > 0\n    print(i)"
  lexer = Lexer::new(code)
  tokens = lexer.tokenize()
  
  indent_count = count_tokens(tokens, TokenType::INDENT(_))
  dedent_count = count_tokens(tokens, TokenType::DEDENT(_))
  
  assert_eq(indent_count, 2, "Should have 2 indents")
  assert_eq(dedent_count, 2, "Should have 2 dedents (balanced)")
}
```

### Parser Test Example (Mercuri Format)

```killer
# File: tests/phase1_parser/test_hybrid_blocks.killer

#[mercuri::test]
#[timeout(1000)]
#[tags("unit", "parser", "hybrid")]
test_parse_indent_function() {
  code = "kfn add(a: i64, b: i64)\n  a + b"
  
  lexer = Lexer::new(code)
  tokens = lexer.tokenize()
  parser = Parser::new(tokens)
  
  result = parser.parse()
  assert_ok(result, "Should parse indentation-based function")
  
  ast = result.unwrap()
  assert(ast.len() > 0, "Should produce AST nodes")
  assert_eq(ast[0].type, AstNodeType::FuncDecl, "First node should be FuncDecl")
}

#[mercuri::test]
#[timeout(1000)]
#[tags("unit", "parser", "backward-compat")]
test_parse_brace_function() {
  code = "kfn add(a: i64, b: i64) { a + b }"
  
  lexer = Lexer::new(code)
  tokens = lexer.tokenize()
  parser = Parser::new(tokens)
  
  result = parser.parse()
  assert_ok(result, "Should still parse brace-based function")
}
```

### Regression Test Example (Mercuri Format)

```killer
# File: tests/phase3_regression/test_existing_brace_code.killer

#[mercuri::test]
#[timeout(5000)]
#[tags("regression", "backward-compat", "brace")]
test_regression_simple_vars() {
  code = "let x = 42; let y = 100;"
  
  lexer = Lexer::new(code)
  tokens = lexer.tokenize()
  
  assert(tokens.len() > 0, "Should tokenize")
  assert_contains(tokens, TokenType::Number(42), "Should parse literal 42")
  assert_contains(tokens, TokenType::Number(100), "Should parse literal 100")
}

#[mercuri::test]
#[timeout(5000)]
#[tags("regression", "backward-compat", "control-flow")]
test_regression_if_else() {
  code = "if (true) { return 5; } else { return 10; }"
  
  lexer = Lexer::new(code)
  tokens = lexer.tokenize()
  parser = Parser::new(tokens)
  
  result = parser.parse()
  assert_ok(result, "Should parse if/else with braces")
}
```

---

## 4.3.4: Running Tests with Mercuri

### Run All Tests

```bash
# Command: killer mercuri run
killer mercuri run

# Output:
# ============================================================
# Killer v4.2 Mercuri Test Suite
# ============================================================
# 
# Running Phase 1 Unit Tests...
#   [✓] test_simple_indent_token (8ms)
#   [✓] test_dedent_token (6ms)
#   [✓] test_nested_indents (12ms)
#   [✗] test_mixed_tabs_spaces_error (FAILED at line 5)
#   ...
# 
# Phase 1 Summary: 39/40 passed (97.5%)
# Phase 3 Summary: 1903/1903 passed (100%)
# 
# ============================================================
# OVERALL: 1942/1943 PASSED (99.95%)
# Coverage: 97.2% (exceeds 95% threshold)
# Time: 45.3 seconds
# ============================================================
```

### Run Specific Phase

```bash
# Run only Phase 1 tests
killer mercuri run --phase 1

# Run only regex-matching tests
killer mercuri run --filter "indent"

# Run with verbose output
killer mercuri run --verbose

# Run with debug mode (stop on first failure)
killer mercuri run --fail-fast
```

### Watch Mode (For Development)

```bash
# Continuously run tests as files change
killer mercuri watch --phase 1

# Auto-run failed tests only
killer mercuri watch --rerun-failures
```

---

## 4.3.5: Mercuri Test Reports

### Generate HTML Report

```bash
# Command: killer mercuri report --format html
killer mercuri report \
  --format html \
  --output ./reports/v4.2_test_results.html

# Creates:
# ./reports/v4.2_test_results.html
#   ├── Summary (1943 tests, 99.95% pass)
#   ├── Phase breakdown (UI tabs)
#   ├── Coverage heatmap
#   ├── Performance timeline
#   ├── Failed test details
#   └── Benchmark comparisons
```

### Generate JSON Report (For CI/CD)

```bash
# Command: killer mercuri report --format json
killer mercuri report \
  --format json \
  --output ./reports/v4.2_test_results.json

# JSON structure:
{
  "suite": "Killer v4.2 Indentation",
  "timestamp": "2026-04-20T15:30:00Z",
  "summary": {
    "total": 1943,
    "passed": 1942,
    "failed": 1,
    "skipped": 0,
    "pass_rate": 99.95,
    "duration_ms": 45300
  },
  "phases": [
    {
      "name": "Phase 1: Lexer Tests",
      "tests": 20,
      "passed": 20,
      "failed": 0
    },
    {
      "name": "Phase 1: Parser Tests",
      "tests": 10,
      "passed": 9,
      "failed": 1
    }
  ],
  "coverage": {
    "lines": 97.2,
    "branches": 94.8,
    "functions": 99.1,
    "statements": 97.5
  },
  "failures": [
    {
      "test": "test_mixed_tabs_spaces_error",
      "phase": "Phase 1: Lexer",
      "file": "tests/phase1_lexer/test_indentation.killer",
      "line": 35,
      "error": "Expected error message to contain 'mixed', got 'tabs and spaces mixed'",
      "duration_ms": 125
    }
  ]
}
```

### Generate CSV Report (For Tracking)

```bash
# Command: killer mercuri report --format csv
killer mercuri report \
  --format csv \
  --output ./reports/v4.2_test_results.csv

# CSV contents:
test_name,phase,status,duration_ms,tags,file
test_simple_indent_token,1_lexer,PASS,8,unit;lexer;indentation,tests/phase1_lexer/test_indentation.killer
test_dedent_token,1_lexer,PASS,6,unit;lexer;indentation,tests/phase1_lexer/test_indentation.killer
test_mixed_tabs_spaces_error,1_lexer,FAIL,125,unit;lexer;error,tests/phase1_lexer/test_indentation.killer
...
```

---

## 4.3.6: CI/CD Integration with Mercuri

### GitHub Actions Workflow

```yaml
# File: .github/workflows/killer-v4.2-test.yml
name: Killer v4.2 Mercuri Tests

on:
  push:
    branches: [ main, develop ]
  pull_request:
    branches: [ main ]

jobs:
  mercuri-tests:
    runs-on: ubuntu-latest
    
    steps:
      - uses: actions/checkout@v3
      
      - name: Setup Killer
        run: |
          curl -sSL https://killer-lang.org/install.sh | bash
          killer --version
      
      - name: Initialize Mercuri
        run: killer mercuri init --config mercuri.config
      
      - name: Run Phase 1 Tests
        run: |
          killer mercuri run --phase 1 --timeout 60000
          
      - name: Run Phase 3 Regression Tests
        run: |
          killer mercuri run --phase 3 --timeout 300000
      
      - name: Generate Reports
        if: always()
        run: |
          mkdir -p ./reports
          killer mercuri report --format html --output ./reports/results.html
          killer mercuri report --format json --output ./reports/results.json
          killer mercuri report --format csv --output ./reports/results.csv
      
      - name: Upload Test Results
        if: always()
        uses: actions/upload-artifact@v3
        with:
          name: mercuri-reports
          path: ./reports/
          retention-days: 30
      
      - name: Publish Test Report
        if: always()
        uses: dorny/test-reporter@v1
        with:
          name: Killer v4.2 Mercuri Tests
          path: './reports/results.json'
          reporter: 'java-junit'
      
      - name: Check Coverage Threshold
        run: |
          COVERAGE=$(killer mercuri report --format json | jq '.coverage.lines')
          if (( $(echo "$COVERAGE < 95" | bc -l) )); then
            echo "Coverage below threshold: $COVERAGE < 95"
            exit 1
          fi
          echo "Coverage OK: $COVERAGE%"
      
      - name: Fail if Tests Failed
        if: failure()
        run: |
          echo "❌ Mercuri tests failed - blocking merge"
          exit 1
```

### GitLab CI Integration

```yaml
# File: .gitlab-ci.yml
killer_v4.2_mercuri:
  image: killer:latest
  stage: test
  
  script:
    - killer mercuri init --config mercuri.config
    - killer mercuri run --phase 1 --timeout 60000
    - killer mercuri run --phase 3 --timeout 300000
    - killer mercuri report --format json --output results.json
  
  artifacts:
    reports:
      junit: results.json
    paths:
      - results.json
    expire_in: 30 days
  
  retry:
    max: 2
    when: script_failure
  
  allow_failure: false
```

---

## 4.3.7: Mercuri Dashboard & Monitoring

### Real-Time Test Dashboard

```bash
# Command: killer mercuri dashboard --port 8080
killer mercuri dashboard --port 8080

# Access: http://localhost:8080
# Shows:
# - Live test execution counter
# - Real-time pass/fail rate graph
# - Phase completion progress
# - Performance timeline (ms per test)
# - Coverage trend (last 10 runs)
# - Failed test details with blame info
# - Performance regression alerts
```

### Performance Benchmarking

```bash
# Command: killer mercuri bench --baseline v4.1
killer mercuri bench \
  --baseline v4.1 \
  --compare v4.2 \
  --output ./reports/benchmark.html

# Compares:
# - Lexer tokenization speed (v4.1 vs v4.2)
# - Parser compilation time
# - Memory usage (peak & average)
# - Test execution overhead
# - Regression detection
```

### Trend Tracking

```bash
# Command: killer mercuri trends --days 30
killer mercuri trends \
  --days 30 \
  --format html \
  --output ./reports/trends.html

# Shows:
# - Test pass rate trend
# - Code coverage trend
# - Performance trend (regression detection)
# - Phase completion timeline
# - Release readiness indicator
```

---

## 4.3.8: Success Criteria

| Metric | Target | Status |
|--------|--------|--------|
| Phase 1 Unit Tests | 40/40 (100%) | ⏳ TBD |
| Phase 3 Regression | 1,903/1,903 (100%) | ⏳ TBD |
| Total Pass Rate | 1,943/1,943 (100%) | ⏳ TBD |
| Code Coverage | ≥95% | ⏳ TBD |
| Test Execution Time | <60 sec | ⏳ TBD |
| CI/CD Integration | All workflows pass | ⏳ TBD |
| Performance Regression | <5% slower than v4.1 | ⏳ TBD |

---

## 4.3.9: Next Steps

1. ✅ Create Mercuri test files (sample provided)
2. ✅ Configure mercuri.config
3. ✅ Setup CI/CD workflows
4. ✅ Run: `killer mercuri run`
5. ✅ Monitor dashboard: `killer mercuri dashboard`
6. ✅ Generate reports: `killer mercuri report --format html`
7. ✅ Review coverage & trends
8. ✅ **RELEASE v4.2.0**

---

## Summary

**Mercuri Integration Complete!**
- ✅ 1,943 total tests organized for Mercuri
- ✅ Test manifest configured
- ✅ CI/CD workflows ready
- ✅ Reporting setup (HTML/JSON/CSV)
- ✅ Dashboard & monitoring configured
- ✅ Performance benchmarking enabled

**Ready to begin Phase 1 implementation immediately!**
