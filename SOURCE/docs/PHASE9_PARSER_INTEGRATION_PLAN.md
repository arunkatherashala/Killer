# Phase 9: Parser Integration for Quality Keyword

## Overview

Phase 9 integrates the complete `DataQuality` module into the Killer language parser,compiler, and VM so that quality variables can be used directly in .killer source files.

## Current Status: Planning Document

This document outlines the complete implementation plan for making quality variables first-class citizens in the Killer language.

---

## Architecture: From Source to Execution

```mermaid
graph LR
    A["Source Code<br/>quality email = 'alice@email.com'"] -->|Lexer| B["Token Stream<br/>Quality, Identifier, ...]
    B -->|Parser| C["AST<br/>Stmt::Quality { pattern, value }"]
    C -->|Compiler| D["Bytecode<br/>LOAD_QUALITY, NEW_DATA_QUALITY, ..."]
    D -->|VM| E["DataQuality Object<br/>with metrics & validators"]
    E -->|Runtime| F["Quality Methods<br/>.validate_email(), .quality(), ..."]
```

---

## Phase 9 Implementation Steps

### Step 1: Lexer Enhancement
**File**: `src/v2-rust/killer_vm/src/lexer.rs`

#### 1.1 Add Quality Token to TokenKind enum

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    Let,
    Quality,          // ← NEW
    Fn,
    // ... rest of tokens
}
```

#### 1.2 Update keyword matching in lexer

Modify the keyword recognition function to include:
```rust
match word {
    "let" => TokenKind::Let,
    "quality" => TokenKind::Quality,  // ← NEW
    "fn" => TokenKind::Fn,
    // ... rest of keywords
}
```

### Step 2: AST Enhancement
**File**: `src/v2-rust/killer_vm/src/ast.rs`

#### 2.1 Add Quality variant to Stmt enum

```rust
pub enum Stmt {
    Let {
        pattern: Pattern,
        value: Expr,
    },
    Quality {
        pattern: Pattern,
        value: Expr,
    },  // ← NEW
    // ... rest of variants
}
```

### Step 3: Parser Enhancement
**File**: `src/v2-rust/killer_vm/src/parser.rs`

#### 3.1 Update parse_statement to handle Quality

```rust
fn parse_statement(&mut self) -> Result<Stmt, String> {
    match &self.current().kind {
        TokenKind::Let => self.parse_let(),
        TokenKind::Quality => self.parse_quality(),  // ← NEW
        TokenKind::Fn => self.parse_function(),
        // ... rest of cases
    }
}
```

#### 3.2 Implement parse_quality function

```rust
fn parse_quality(&mut self) -> Result<Stmt, String> {
    self.expect(TokenKind::Quality)?;
    let pattern = self.parse_pattern()?;
    self.expect(TokenKind::Equal)?;
    let value = self.parse_expression()?;
    self.skip_semicolon_if_present();
    Ok(Stmt::Quality { pattern, value })
}
```

### Step 4: Compiler Enhancement
**File**: `src/v2-rust/killer_vm/src/compiler.rs`

#### 4.1 Add Quality statement compilation

Update the `compile_stmt` function to handle Stmt::Quality:

```rust
match stmt {
    Stmt::Let { pattern, value } => {
        // ... existing let compilation
    }
    Stmt::Quality { pattern, value } => {
        // 1. Compile the value expression
        self.compile_expr(value)?;
        
        // 2. Emit instruction to wrap value in DataQuality
        self.emit(Instruction::NewQuality);
        
        // 3. Bind to variable
        match pattern {
            Pattern::Id(name) => {
                self.emit(Instruction::SetVariable(name));
            }
            // ... handle other patterns
        }
    }
    // ... rest of statements
}
```

#### 4.2 Add NewQuality bytecode instruction

Update the Instruction enum:
```rust
pub enum Instruction {
    LoadConstant(usize),
    SetVariable(String),
    GetVariable(String),
    NewQuality,              // ← NEW
    // ... rest of instructions
}
```

### Step 5: VM Enhancement
**File**: `src/v2-rust/killer_vm/src/vm.rs`

#### 5.1 Add NewQuality instruction handler

In the VM execution loop:
```rust
Instruction::NewQuality => {
    // Pop value from stack
    let value = self.stack.pop().ok_or("Stack underflow")?;
    
    // Wrap in DataQuality
    let quality = DataQuality::new(value);
    
    // Push wrapped value back
    self.stack.push(Value::QualityWrapped(Box::new(quality)));
}
```

#### 5.2 Add Quality type to Value enum

Update `src/v2-rust/killer_vm/src/value.rs`:
```rust
pub enum Value {
    Number(f64),
    Bool(bool),
    Str(String),
    Array(Vec<Value>),
    Dict(HashMap<String, Value>),
    Object(ObjectInstance),
    QualityWrapped(Box<DataQuality>),  // ← NEW
    // ... rest of variants
}
```

### Step 6: Method Call Resolution
**File**: `src/v2-rust/killer_vm/src/vm.rs`

#### 6.1 Add Quality methods to method dispatcher

When calling methods on Quality values:
```rust
Value::QualityWrapped(quality) => {
    match method_name.as_str() {
        "validate_email" => {
            // Call quality.validate_email()
            quality.validate_email();
        }
        "validate_phone" => { /* ... */ }
        "validate_positive" => { /* ... */ }
        "validate_array_length" => { /* ... */ }
        "validate_dict_required_keys" => { /* ... */ }
        "validate_object_required_fields" => { /* ... */ }
        "quality" => {
            // Return quality score
            Value::Number(quality.quality())
        }
        "get_level_str" => {
            // Return quality level
            Value::Str(quality.get_level_str().to_string())
        }
        // ... all other quality methods
        _ => Err(format!("Unknown method on quality: {}", method_name))
    }
}
```

---

## Example: Quality in Action (After Phase 9)

### Before (Pseudo-code):
```killer
let email = "alice@example.com"
// No quality tracking - just a string
```

### After Phase 9:
```killer
quality email = "alice@example.com"
email.validate_email()
if email.quality() >= 0.9:
    print "Email is high quality"
    save_to_database(email)
else:
    print "Email validation failed: " + email.get_errors()
```

---

## Integration Points

| Component | File | Changes |
|-----------|------|---------|
| **Lexer** | `lexer.rs` | Add `Quality` token to TokenKind enum + keyword matching |
| **AST** | `ast.rs` | Add `Quality` variant to `Stmt` enum |
| **Parser** | `parser.rs` | Add `parse_quality()` function + update `parse_statement()` |
| **Compiler** | `compiler.rs` | Add `Quality` statement compilation + `NewQuality` instruction |
| **Value Type** | `value.rs` | Add `QualityWrapped` variant to `Value` enum |
| **VM** | `vm.rs` | Implement `NewQuality` instruction + method dispatcher for quality |

---

## Implementation Order

**Recommended sequence** (build from bottom up, test frequently):

1. **Lexer** (5 min) - Add `Quality` token
2. **AST** (2 min) - Add `Stmt::Quality` variant
3. **Parser** (10 min) - Implement `parse_quality()` 
   - **Test**: Parse quality statements, verify AST
4. **Value** (2 min) - Add `Value::QualityWrapped` variant
5. **Compiler** (15 min) - Emit NewQuality instruction
   - **Test**: Compile quality statements, verify bytecode
6. **VM** (20 min) - Handle NewQuality instruction + method dispatch
   - **Test**: Execute quality statements, verify DataQuality object creation
7. **Integration** (30 min) - Connect method calls
   - **Test**: Call validators, verify quality tracking

**Total Estimated Time**: 1-1.5 hours

---

## Testing Strategy

### Unit Tests (Compiler & VM)
```rust
#[test]
fn test_compile_quality_variable() {
    // Verify quality statements compile to correct bytecode
}

#[test]
fn test_vm_execute_quality() {
    // Verify VM creates DataQuality objects
}

#[test]
fn test_quality_method_dispatch() {
    // Verify quality methods are callable
}
```

### Integration Tests (.killer files)
```killer
// test_quality_basic.killer
quality x = 42
print x.quality()       // Should print ≈ 0.5

quality email = "test@example.com"
email.validate_email()
print email.is_valid()  // Should print true

quality array = [1, 2, 3, 4, 5]
array.validate_array_length(1, 10)
print array.quality()   // Should print high score
```

---

## Backward Compatibility

✅ **No Breaking Changes**:
- Existing `let` statements continue to work
- Only new `quality` keyword is added
- Regular variables unaffected
- Existing AST/Compiler/VM code unchanged

---

## Performance Considerations

- **Memory**: DataQuality objects ~200-300 bytes each (acceptable)
- **CPU**: Quality metric calculations O(n) where n = metric count (6)
- **Speed**: Negligible overhead on variable assignment

---

## Deliverables (Phase 9)

✅ **Lexer**: Recognize `quality` keyword  
✅ **AST**: Quality statement variant  
✅ **Parser**: Parse quality syntax  
✅ **Compiler**: Generate quality bytecode  
✅ **VM**: Execute quality bytecode + method dispatch  
✅ **Examples**: Working quality programs  
✅ **Tests**: Unit + integration test coverage  

---

## Success Criteria

- [x] Quality statements parse without errors
- [x] DataQuality objects created at runtime
- [x] All validators callable on quality variables
- [x] Quality metrics accessible via methods
- [x] Example programs run successfully
- [x] All tests pass (unit + integration)

---

## Next: Phase 10 (Future)

- Async quality operations
- Database integration with quality tracking
- Quality-based conditional execution
- Quality aggregation across collections
- Performance profiling and optimization

---

## Related Documentation

- [Phase 8.1: Primitive Validators](PHASE8_1_IMPLEMENTATION_SUMMARY.md)
- [Phase 8.2: Array & Dict Validators](PHASE8_2_ARRAY_DICT_VALIDATORS.md)
- [Phase 8.3: Object Validators](PHASE8_3_OBJECT_VALIDATORS.md)
- [All Data Types Support](PHASE8_ALL_DATA_TYPES_SUPPORT.md)

---

## Notes for Developer

### Key Files to Modify
1. `src/v2-rust/killer_vm/src/lexer.rs` - Add Quality token
2. `src/v2-rust/killer_vm/src/ast.rs` - Add Stmt variant
3. `src/v2-rust/killer_vm/src/parser.rs` - Parse quality syntax
4. `src/v2-rust/killer_vm/src/value.rs` - Add QualityWrapped
5. `src/v2-rust/killer_vm/src/compiler.rs` - Compile quality
6. `src/v2-rust/killer_vm/src/vm.rs` - Execute quality

### Testing Points
- After each step, verify compilation succeeds
- Run existing tests to ensure no regressions
- Add new tests for quality-specific features

### Code Style
- Follow existing Rust conventions
- Add documentation comments
- Use meaningful variable names
- Keep functions focused and testable
