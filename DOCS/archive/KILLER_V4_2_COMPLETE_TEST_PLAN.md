# Killer v4.2 Hybrid Indentation - Complete 4-Phase Test Plan

## Executive Summary
This document outlines comprehensive testing strategy for all 4 phases of the Killer v4.2 indentation implementation. Success criteria: 100% backward compatibility (1,903 existing tests pass) + 40+ new indentation tests pass.

**Total Test Coverage: 1,943+ tests**
- Phase 1: 40 unit tests
- Phase 2: 50 documentation tests
- Phase 3: 100+ regression tests
- Phase 4: Integration & performance tests

---

## PHASE 1: PARSER ENHANCEMENT (Weeks 1-2)
**Owner:** Parser Development Team
**Duration:** 40-60 hours
**Success Criteria:** All 40 new unit tests passing + 1,903 existing tests still pass

### Phase 1.1: Lexer Unit Tests (20 tests)

#### Category 1: Basic Indentation (4 tests)
- **L1.1.1: Simple Indent Token**
  - Input: `"kfn test()\n  x = 1"`
  - Expected: Token stream contains `INDENT(2)`
  - Pass Criteria: Token type matches, indentation level is 2

- **L1.1.2: Simple Dedent Token**
  - Input: `"kfn test()\n  x = 1\ny = 2"`
  - Expected: Token stream contains `DEDENT(1)`
  - Pass Criteria: DEDENT emitted after statement

- **L1.1.3: Multiple Indent Levels**
  - Input: 3-level nested indentation
  - Expected: 3 × INDENT tokens, 3 × DEDENT tokens
  - Pass Criteria: Balanced indent/dedent count

- **L1.1.4: Consistent Indentation**
  - Input: Multiple statements at same indentation
  - Expected: One INDENT token, one final DEDENT
  - Pass Criteria: No extra INDENT/DEDENT tokens

#### Category 2: Mixed Syntax (4 tests)
- **L1.2.1: Brace Syntax Unchanged**
  - Input: `"kfn test() { x = 1 }"`
  - Expected: LEFTBRACE, RIGHTBRACE tokens (no INDENT/DEDENT)
  - Pass Criteria: Brace mode requires no indentation tokens

- **L1.2.2: Hybrid Mixed (Indent then Brace)**
  - Input: Mix of function with indent and function with braces in same file
  - Expected: Both styles tokenize correctly
  - Pass Criteria: No cross-contamination

- **L1.2.3: Indent Mode Disabled**
  - Input: Code with indentation + indent_mode = false
  - Expected: No INDENT/DEDENT tokens emitted
  - Pass Criteria: Lexer respects mode flag

- **L1.2.4: Tab vs Space Detection**
  - Input: `"x = 1\n\ty = 2"` (tab-indented)
  - Expected: INDENT(4) token (tab → 4 spaces)
  - Pass Criteria: Correct tab-to-space conversion

#### Category 3: Error Conditions (4 tests)
- **L1.3.1: Mixed Tabs and Spaces Error**
  - Input: `"kfn test()\n  x = 1\n\ty = 2"` (space then tab)
  - Expected: KillerError with "mixed tabs and spaces"
  - Pass Criteria: Error thrown, error message clear

- **L1.3.2: Unexpected Dedent Error**
  - Input: Invalid dedent level not matching any previous level
  - Expected: KillerError with "unexpected dedent"
  - Pass Criteria: Parser rejects invalid indentation

- **L1.3.3: Blank Line Handling**
  - Input: Blank line between statements
  - Expected: No INDENT/DEDENT tokens on blank line
  - Pass Criteria: Blank lines are transparent

- **L1.3.4: Comment Line Handling**
  - Input: Comment line at start of logical block
  - Expected: NEWLINE, no INDENT
  - Pass Criteria: Comments don't affect indentation

#### Category 4: Edge Cases (4 tests)
- **L1.4.1: EOF Multiple Dedents**
  - Input: 3-level nested code reaching EOF
  - Expected: All remaining DEDENTs emitted at EOF
  - Pass Criteria: indent_stack properly unwound

- **L1.4.2: Empty Function Body**
  - Input: `"kfn empty()\n  # empty"`
  - Expected: INDENT then DEDENT with no statements
  - Pass Criteria: Empty bodies handled gracefully

- **L1.4.3: Single Space Indent**
  - Input: 1-space indentation
  - Expected: INDENT(1) token accepted
  - Pass Criteria: Any indentation increase accepted

- **L1.4.4: Very Deep Nesting**
  - Input: 10-level nested structure
  - Expected: 10 INDENTs and 10 DEDENTs
  - Pass Criteria: No stack overflow, correct tracking

### Phase 1.2: Parser Unit Tests (10 tests)

#### Category 1: Hybrid Block Parsing (5 tests)
- **P1.1.1: Parse Indent-Based Function**
  - Input: `"kfn add(a: i64, b: i64)\n  a + b"`
  - Expected: AstNode::FuncDecl with correct body
  - Pass Criteria: Parses successfully, body contains expression

- **P1.1.2: Parse Brace-Based Function**
  - Input: `"kfn add(a: i64, b: i64) { a + b }"`
  - Expected: AstNode::FuncDecl with correct body
  - Pass Criteria: Backward compatibility maintained

- **P1.1.3: Parse Indented If Statement**
  - Input: `"if (x > 0)\n  print(x)\nelse\n  print(0)"`
  - Expected: AstNode::If with both branches
  - Pass Criteria: Conditional properly parsed

- **P1.1.4: Parse Indented While Loop**
  - Input: `"while (i < 10)\n  i = i + 1"`
  - Expected: AstNode::While with body
  - Pass Criteria: Loop condition and body parsed

- **P1.1.5: Parse Indented For Loop**
  - Input: `"for i in 1..10\n  print(i)"`
  - Expected: AstNode::ForLoop with body
  - Pass Criteria: Iteration properly structured

#### Category 2: Hybrid Error Recovery (5 tests)
- **P1.2.1: Missing Block Error**
  - Input: `"kfn test() print(1)"`  (no brace or newline)
  - Expected: Parse error with helpful message
  - Pass Criteria: Error caught, message clear

- **P1.2.2: Mismatched Braces Error**
  - Input: `"kfn test() { x = 1"`  (missing })
  - Expected: Parse error
  - Pass Criteria: Error detected

- **P1.2.3: Indent/Dedent Mismatch Error**
  - Input: `"kfn test()\n  if true\n    x = 1"` (only 1 dedent expected, but 2)
  - Expected: Parse error with indentation hint
  - Pass Criteria: Error message helpful

- **P1.2.4: Return in Indented Block**
  - Input: `"kfn test()\n  return 42"`
  - Expected: Parsed successfully with RETURN statement
  - Pass Criteria: Return correctly positioned

- **P1.2.5: Newline Handling**
  - Input: Multiple newlines in block
  - Expected: Extra newlines skipped gracefully
  - Pass Criteria: Parser robust to whitespace variation

### Phase 1.3: Regression Tests (10 tests)
All existing v4.1 code must still parse/tokenize correctly

- **R1.1: Simple Variable Declaration**
  - Ensure: `let x = 42;` still works exactly as before
  
- **R1.2: Brace-Based Functions**
  - Ensure: All existing function definitions with braces work
  
- **R1.3: Complex Expressions**
  - Ensure: Operator precedence, parentheses, etc. unchanged
  
- **R1.4: Type Annotations**
  - Ensure: All type annotation parsing unchanged
  
- **R1.5: Generic Types**
  - Ensure: `List<T>`, `Map<K,V>` still parse correctly
  
- **R1.6: Pattern Matching**
  - Ensure: Match statements with braces still work
  
- **R1.7: Actor Definitions**
  - Ensure: Actor syntax unchanged
  
- **R1.8: Struct/Enum/Trait**
  - Ensure: All composite type definitions work
  
- **R1.9: Error Handling**
  - Ensure: Try/catch (or equivalent) unchanged
  
- **R1.10: Comments**
  - Ensure: Line comments `//` and block comments `/* */` still work

---

## PHASE 2: DOCUMENTATION & EXAMPLES (Weeks 2-3)
**Owner:** Documentation Team
**Duration:** 30-40 hours
**Success Criteria:** All documentation reviewed, 50+ validated examples

### Phase 2.1: Documentation Tests (20 docs)

1. **Indentation Guide** - Clarity on tab vs space, indentation amounts
2. **Hybrid Syntax Guide** - When to use indentation vs braces
3. **Migration Guide** - How to convert existing code
4. **Error Messages Guide** - Understanding indentation error messages
5. **Best Practices** - Style guide for new indentation-based code
6. **Examples Repository** - 50+ real-world examples
7. **FAQ** - Common questions about indentation
8. **Comparison Charts** - Indentation vs brace code side-by-side
9. **Performance Notes** - Any parsing performance implications
10. **API Documentation** - Updated parser.rs documentation

### Phase 2.2: Example Validation (30 examples)

#### Category 1: Function Examples (6)
- Simple function with indentation
- Function with multiple parameters
- Nested function calls
- Function with type annotations
- Function returning value
- Function with comments

#### Category 2: Control Flow Examples (8)
- If/else with indentation
- Nested if statements
- While loop with indentation
- For loop with indentation
- Break/continue in loops
- Match statement with indentation
- Complex conditionals
- Early return statements

#### Category 3: Complex Structures (8)
- Struct definition with methods
- Enum definition with patterns
- Actor with message handler
- Trait implementation
- Generic types with indentation
- Pattern matching examples
- List comprehensions (if supported)
- Error handling with indentation

#### Category 4: Real-World Patterns (8)
- HTTP server pattern
- Data processing pipeline
- Microservice examples
- Concurrent actor patterns
- State management
- Configuration parsing
- Logging patterns
- Testing patterns

---

## PHASE 3: INTEGRATION & REGRESSION TESTING (Weeks 3-4)
**Owner:** QA Team
**Duration:** 50-70 hours
**Success Criteria:** 1,903/1,903 existing tests passing + 100+ new tests passing

### Phase 3.1: Build System Integration

- **Compile Check**: Modified parser.rs compiles without warnings
- **Link Check**: No linker errors when including enhanced parser
- **Type Check**: All type annotations still correct
- **Clippy Lint**: No new clippy warnings

### Phase 3.2: Full Regression Test Suite (50 tests)

Run complete existing test suite and verify:
- All 1,903 existing tests still pass
- No performance degradation >5%
- No memory usage increase >10%
- No security vulnerabilities introduced

### Phase 3.3: New Feature Tests (50+ tests)

#### Lexer New Features (20 tests)
- Indentation tracking accuracy
- Tab handling (4-space conversion)
- Mixed syntax detection
- Error recovery with clear messages

#### Parser New Features (20 tests)
- Hybrid block parsing
- Multiple dedent emission
- Error recovery with suggestions
- Type annotation preservation

#### End-to-End (15+ tests)
- Complete programs in indentation style
- Mixed-style programs
- Large codebases (10K+ lines)
- Deep nesting (100+ levels)

### Phase 3.4: Performance Testing

- **Compile Time**: Measure parser time on v4.1 codebase
  - Criteria: < 5% increase from baseline
  
- **Runtime Performance**: Execute 1,903 existing tests
  - Criteria: No test slower than v4.1 by >10%
  
- **Memory Usage**: Profile lexer/parser memory
  - Criteria: < 10% increase from baseline
  
- **Indentation Overhead**: Time parsing indentation vs braces
  - Criteria: Indentation < 15% slower than braces

### Phase 3.5: Edge Case Testing (20+ tests)

- Very large indent stack (100+ levels)
- Very long lines (10K+ chars)
- Very large files (100K+ lines)
- Mixed encodings (UTF-8)
- Binary-like characters in strings
- Pathological indentation patterns

---

## PHASE 4: INTEGRATION, PERFORMANCE & RELEASE (Weeks 4+)
**Owner:** DevOps/Release Team
**Duration:** 20-30 hours
**Success Criteria:** Ready for production release, full CI/CD integration

### Phase 4.1: CI/CD Integration

#### GitHub Actions Integration
```yaml
name: Test Killer v4.2
on: [push, pull_request]
jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v2
      - name: Build Parser v4.2
        run: cargo build --release
      - name: Run Unit Tests (Phase 1)
        run: cargo test --lib parser::tests
      - name: Run Integration Tests (Phase 3)
        run: cargo test --integration_tests
      - name: Run Regression Suite
        run: killer test --regression
      - name: Performance Benchmark
        run: killer bench --baseline v4.1
      - name: Coverage Report
        run: cargo tarpaulin --out Html
```

#### Test Coverage Requirements
- Phase 1 Unit Tests: 100% code coverage in parser.rs
- Phase 2 Examples: 50+ validated examples, 0 errors
- Phase 3 Regression: 1,943+ tests passing
- Phase 4 Integration: E2E tests passing

### Phase 4.2: Performance Benchmarking

#### Lexer Benchmarks
```
Benchmark: Tokenize 1,903 test files (avg 50KB each)
- v4.1 (baseline): 250ms
- v4.2 (indentation): <262ms (< +5%)

Benchmark: Parse same files
- v4.1 (baseline): 500ms
- v4.2 (indentation): <525ms (< +5%)
```

#### Memory Benchmarks
```
Benchmark: Peak memory usage parsing all test files
- v4.1 (baseline): 150MB
- v4.2 (indentation): <165MB (< +10%)
```

### Phase 4.3: Mercury (Mercuri) System Integration

**STATUS: Awaiting user clarification on "mercuri" system**

Once Mercury system is clarified (CI/CD platform, testing framework, or monitoring tool), integrate:

#### Option A: Mercury CI/CD (Jenkins/GitLab/GitHub Actions)
```
- Trigger test on every commit
- Report results to Mercury dashboard
- Block merge if tests fail
- Track performance trends
```

#### Option B: Mercury Testing Framework
```
- Use Mercury test runner for Phase 3/4 tests
- Generate Mercury-compatible test reports
- Integrate with code coverage tools
```

#### Option C: Mercury Monitoring Tool
```
- Monitor performance during integration tests
- Alert if performance degrades >10%
- Track historical trends
```

### Phase 4.4: Documentation & Release

#### Release Checklist
- [ ] All 1,943 tests passing
- [ ] Performance <5% regression
- [ ] Memory <10% increase
- [ ] Documentation complete (10 docs)
- [ ] Examples validated (50+ examples)
- [ ] CI/CD integration verified
- [ ] Backward compatibility confirmed
- [ ] Security audit passed
- [ ] Performance benchmarks recorded
- [ ] Release notes prepared

#### Release Notes Template
```
# Killer v4.2.0 Release Notes

## New Features
- Hybrid indentation-based syntax
- INDENT/DEDENT/NEWLINE tokens
- Backward compatible with all v4.1 code

## Performance
- Parser speed: +<5% vs v4.1
- Memory: +<10% vs v4.1
- Compilation: unchanged

## Testing
- 1,943 total tests passing
- 40 new indentation tests
- 100+ new integration tests
- Regression suite: 100% pass

## Migration Guide
See KILLER_V4_2_MIGRATION_GUIDE.md for detailed instructions
```

---

## TEST EXECUTION TIMELINE

### Week 1-2: Phase 1 (40 tests)
```
Day 1-2: Lexer unit tests (20 tests)
Day 3-4: Parser unit tests (10 tests)
Day 5: Regression tests (10 tests)
Goal: All 40 passing before moving to Phase 2
```

### Week 2-3: Phase 2 (50 tests)
```
Day 1-3: Documentation validation
Day 4-5: Example validation (50 examples)
Goal: All examples parse/execute correctly
```

### Week 3-4: Phase 3 (100+ tests)
```
Day 1: Build system + compile checks
Day 2-3: Full regression suite (1,903 existing tests)
Day 4-5: New feature tests + edge cases
Goal: 100% regression pass + all new tests passing
```

### Week 4: Phase 4 (Integration & Release)
```
Day 1-2: CI/CD setup + Mercury integration
Day 3: Performance benchmarking
Day 4: Release preparation + documentation
Day 5: Release and monitoring
Goal: Production release, monitoring active
```

---

## SUCCESS METRICS

| Metric | Target | Phase |
|--------|--------|-------|
| Unit Test Pass Rate | 100% (40/40) | 1 |
| Example Validation | 100% (50/50) | 2 |
| Regression Pass Rate | 100% (1,903/1,903) | 3 |
| New Feature Tests | 100% (100+/100+) | 3 |
| Parser Compile Time | <5% regression | 4 |
| Memory Usage | <10% increase | 4 |
| Performance | <5% regression | 4 |
| Code Coverage | >95% for parser.rs | 4 |
| Documentation | 10/10 complete | 4 |
| CI/CD Integration | 100% | 4 |

---

## BLOCKERS & RISKS

### Known Risks
1. **Parser complexity**: Hybrid mode could introduce subtle bugs
   - Mitigation: Extensive regression testing, fuzzing

2. **Performance**: Indentation tracking adds overhead
   - Mitigation: Benchmark-driven optimization, lazy evaluation

3. **Error messages**: Indentation errors can be confusing
   - Mitigation: Clear error messages with visual indicators

4. **Mixed codebases**: Mixing indent/brace styles could confuse developers
   - Mitigation: linter rules, style guide enforcement

### Dependency: Mercury System Clarification
**CRITICAL BLOCKER**: Phase 4.3 cannot begin until user specifies what "mercuri" system means.

Awaiting confirmation:
- [ ] Mercury CI/CD platform? (Which one?)
- [ ] Mercury testing framework? (Custom or standard?)
- [ ] Mercury monitoring tool? (Prometheus, DataDog, etc.?)
- [ ] Other/Custom integration?

---

## APPROVAL & SIGN-OFF

This test plan is ready for Phase 1 implementation upon Mercury system clarification.

**Next Action**: User provides Mercury system details → Finalize Phase 4.3 specs → Begin Phase 1
