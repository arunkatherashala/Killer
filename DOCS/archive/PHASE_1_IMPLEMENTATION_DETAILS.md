# Phase 1: Parser Enhancement - Implementation Details

## Objective
Add indentation tokenization to Killer's lexer to support hybrid indentation-based syntax.

## Current State
- **File:** `_TOOLS/killer_rcore/src/parser.rs`
- **Lexer:** Basic tokenizer (skips whitespace)
- **Tokens:** No INDENT/DEDENT support
- **Parser:** Expects braces for all scopes

## Changes Required

### 1. Add Token Types (In TokenType enum)
```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenType {
    // ... existing tokens ...
    
    // NEW: Indentation tokens
    INDENT(usize),      // Indentation level
    DEDENT(usize),      // Number of dedent levels
    NEWLINE,            // Newline (significant in indentation mode)
    
    // ... rest of tokens ...
}
```

### 2. Modify Lexer Struct
```rust
pub struct Lexer {
    input: Vec<char>,
    position: usize,
    line: usize,
    column: usize,
    
    // NEW FIELDS:
    indent_stack: Vec<usize>,      // Stack of indentation levels
    pending_dedents: Vec<Token>,   // Dedent tokens to emit
    line_start: bool,              // True at line start
    indent_mode: bool,             // Enable indentation-based syntax
}
```

### 3. New Methods for Indentation

#### Track Indentation at Line Start
```rust
fn track_indentation(&mut self) -> Result<Vec<Token>> {
    let mut tokens = Vec::new();
    if !self.line_start {
        return Ok(tokens);
    }
    
    self.line_start = false;
    
    // Count leading spaces/tabs at line start
    let mut indent_count = 0;
    let mut uses_spaces = None;
    
    while let Some(ch) = self.current_char() {
        match ch {
            ' ' => {
                if uses_spaces == Some(false) {
                    return Err(KillerError::new(
                        format!("Mixed tabs and spaces at line {}", self.line)
                    ));
                }
                uses_spaces = Some(true);
                indent_count += 1;
                self.advance();
            }
            '\t' => {
                if uses_spaces == Some(true) {
                    return Err(KillerError::new(
                        format!("Mixed tabs and spaces at line {}", self.line)
                    ));
                }
                uses_spaces = Some(false);
                indent_count += 4;  // Tab = 4 spaces
                self.advance();
            }
            _ => break,
        }
    }
    
    // Skip blank lines
    if matches!(self.current_char(), Some('\n') | None) {
        return Ok(tokens);
    }
    
    let current_indent = self.indent_stack.last().copied().unwrap_or(0);
    
    if indent_count > current_indent {
        // INDENT
        self.indent_stack.push(indent_count);
        tokens.push(Token {
            token_type: TokenType::INDENT(indent_count),
            line: self.line,
            column: 1,
            value: " ".repeat(indent_count),
        });
    } else if indent_count < current_indent {
        // DEDENT(s)
        while let Some(&level) = self.indent_stack.last() {
            if level == indent_count {
                self.indent_stack.pop();
                break;
            } else if level > indent_count {
                self.indent_stack.pop();
                tokens.push(Token {
                    token_type: TokenType::DEDENT(1),
                    line: self.line,
                    column: 1,
                    value: String::new(),
                });
            } else {
                return Err(KillerError::new(
                    format!("Indentation error at line {}", self.line)
                ));
            }
        }
    }
    
    Ok(tokens)
}
```

#### Emit Newline When Significant
```rust
fn handle_newline(&mut self) -> Token {
    let token = Token {
        token_type: TokenType::NEWLINE,
        line: self.line,
        column: self.column,
        value: "\n".to_string(),
    };
    self.advance();
    self.line_start = true;
    token
}
```

### 4. Update Tokenize Method
```rust
pub fn tokenize(&mut self) -> Result<Vec<Token>> {
    let mut tokens = Vec::new();
    
    self.line_start = true;
    self.indent_stack.push(0);  // Base indentation = 0
    
    loop {
        // Track indentation at line start
        let indent_tokens = self.track_indentation()?;
        tokens.extend(indent_tokens);
        
        match self.current_char() {
            None => {
                // Emit remaining DEDENTs at EOF
                while self.indent_stack.len() > 1 {
                    self.indent_stack.pop();
                    tokens.push(Token {
                        token_type: TokenType::DEDENT(1),
                        line: self.line,
                        column: self.column,
                        value: String::new(),
                    });
                }
                tokens.push(Token {
                    token_type: TokenType::Eof,
                    line: self.line,
                    column: self.column,
                    value: String::new(),
                });
                break;
            }
            Some('\n') => {
                tokens.push(self.handle_newline());
            }
            Some(ch) if ch.is_whitespace() => {
                self.skip_whitespace();
            }
            // ... existing tokenization logic ...
        }
    }
    
    Ok(tokens)
}
```

### 5. Parser Changes

#### Update parse_function
**Before:**
```rust
// Expects: "kfn" IDENT "(" params ")" "{" body "}"
fn parse_function(&mut self) -> Result<AstNode> {
    self.expect(TokenType::Fn)?;
    let name = self.parse_identifier()?;
    self.expect(TokenType::LeftParen)?;
    let params = self.parse_parameters()?;
    self.expect(TokenType::RightParen)?;
    self.expect(TokenType::LeftBrace)?;  // REQUIRED
    let body = self.parse_block()?;
    self.expect(TokenType::RightBrace)?;  // REQUIRED
    Ok(AstNode::Function { name, params, body })
}
```

**After (Hybrid):**
```rust
// Accepts: "kfn" IDENT "(" params ")" (NEWLINE INDENT body DEDENT | "{" body "}")
fn parse_function(&mut self) -> Result<AstNode> {
    self.expect(TokenType::Fn)?;
    let name = self.parse_identifier()?;
    self.expect(TokenType::LeftParen)?;
    let params = self.parse_parameters()?;
    self.expect(TokenType::RightParen)?;
    
    let body = if self.current()?.token_type == TokenType::LeftBrace {
        // Brace style: {...}
        self.advance();
        let body = self.parse_block()?;
        self.expect(TokenType::RightBrace)?;
        body
    } else if self.current()?.token_type == TokenType::NEWLINE {
        // Indentation style: NEWLINE INDENT ... DEDENT
        self.expect(TokenType::NEWLINE)?;
        self.expect(TokenType::INDENT(_))?;
        let body = self.parse_block()?;
        self.expect(TokenType::DEDENT(_))?;
        body
    } else {
        return Err(KillerError::new(
            "Expected '{' or newline after function signature".to_string()
        ));
    };
    
    Ok(AstNode::Function { name, params, body })
}
```

#### Update parse_for_loop
**Before:**
```rust
fn parse_for_loop(&mut self) -> Result<AstNode> {
    self.expect(TokenType::For)?;
    let var = self.parse_identifier()?;
    self.expect(TokenType::In)?;
    let iterable = self.parse_expression()?;
    self.expect(TokenType::LeftBrace)?;
    let body = self.parse_block()?;
    self.expect(TokenType::RightBrace)?;
    Ok(AstNode::ForLoop { var, iterable, body })
}
```

**After (Hybrid):**
```rust
fn parse_for_loop(&mut self) -> Result<AstNode> {
    self.expect(TokenType::For)?;
    let var = self.parse_identifier()?;
    self.expect(TokenType::In)?;
    let iterable = self.parse_expression()?;
    
    let body = if self.current()?.token_type == TokenType::LeftBrace {
        self.advance();
        let body = self.parse_block()?;
        self.expect(TokenType::RightBrace)?;
        body
    } else if self.current()?.token_type == TokenType::NEWLINE {
        self.expect(TokenType::NEWLINE)?;
        self.expect(TokenType::INDENT(_))?;
        let body = self.parse_block()?;
        self.expect(TokenType::DEDENT(_))?;
        body
    } else {
        return Err(KillerError::new(
            "Expected '{' or newline after for statement".to_string()
        ));
    };
    
    Ok(AstNode::ForLoop { var, iterable, body })
}
```

#### Update parse_if_statement
Similar pattern - accept both indentation and braces

#### Update parse_while
Similar pattern - accept both indentation and braces

#### Update parse_match
Similar pattern - accept both indentation and braces

### 6. Error Messages

```rust
fn error_indentation(message: &str, line: usize) -> KillerError {
    KillerError::new(format!("IndentationError at line {}: {}", line, message))
}
```

Examples:
- "Expected indent after function declaration"
- "Mixed tabs and spaces"
- "Unexpected dedent"
- "Indentation must increase by at least 1"

## Testing Strategy

### Unit Tests to Add

```rust
#[cfg(test)]
mod indentation_tests {
    use super::*;

    #[test]
    fn test_simple_indent() {
        let mut lexer = Lexer::new("kfn test()\n  print(1)");
        let tokens = lexer.tokenize().unwrap();
        // Should emit: KFN, IDENTIFIER, LPAREN, RPAREN, NEWLINE, INDENT, ...
        assert!(tokens.iter().any(|t| t.token_type == TokenType::INDENT(_)));
    }
    
    #[test]
    fn test_nested_indents() {
        let code = "for i in 1..5\n  if i > 0\n    print(i)";
        let mut lexer = Lexer::new(code);
        let tokens = lexer.tokenize().unwrap();
        // Should have 2 INDENTs and 2 DEDENTs
    }
    
    #[test]
    fn test_mixed_tabs_spaces_error() {
        let code = "kfn test()\n  x = 1\n\tx = 2"; // Space then tab
        let mut lexer = Lexer::new(code);
        match lexer.tokenize() {
            Err(e) => {
                assert!(e.to_string().contains("Mixed"));
            }
            Ok(_) => panic!("Should reject mixed tabs/spaces"),
        }
    }
    
    #[test]
    fn test_hybrid_syntax_braces() {
        let code = "kfn test() { print(1) }";
        let mut lexer = Lexer::new(code);
        let tokens = lexer.tokenize().unwrap();
        // Should NOT have INDENT/DEDENT
        assert!(!tokens.iter().any(|t| matches!(t.token_type, TokenType::INDENT(_) | TokenType::DEDENT(_))));
    }
    
    #[test]
    fn test_blank_line_ignored() {
        let code = "kfn test()\n  x = 1\n\n  y = 2";
        let mut lexer = Lexer::new(code);
        let tokens = lexer.tokenize().unwrap();
        // Blank line should not emit tokens
    }
}
```

## Implementation Checklist

### Week 1: Parser Enhancement

- [ ] **Step 1:** Add INDENT/DEDENT tokens to TokenType enum
- [ ] **Step 2:** Add indent_stack and line_start to Lexer struct
- [ ] **Step 3:** Implement track_indentation() method
- [ ] **Step 4:** Implement handle_newline() method
- [ ] **Step 5:** Update tokenize() to use indentation tracking
- [ ] **Step 6:** Update parse_function() to accept both styles
- [ ] **Step 7:** Update parse_for_loop() to accept both styles
- [ ] **Step 8:** Update parse_if_statement() to accept both styles
- [ ] **Step 9:** Update parse_while() to accept both styles
- [ ] **Step 10:** Update parse_match() to accept both styles
- [ ] **Step 11:** Create comprehensive unit tests
- [ ] **Step 12:** Run all 1,903 existing tests - verify no regressions
- [ ] **Step 13:** Test error messages for clarity

## Code Review Points

1. **Indentation consistency:** Enforce 2 spaces or tabs, never mixed
2. **Blank line handling:** Ignore blank lines when tracking indentation
3. **EOF handling:** Emit DEDENTs at end of file
4. **Comment handling:** Comments don't affect indentation
5. **One-liner functions:** Should still work
6. **Backward compatibility:** Braces should still work (hybrid mode)

## Performance Considerations

- Indentation tracking adds O(n) pass through input
- INDENT/DEDENT tokens minimal overhead
- No impact on runtime performance (only parsing time)
- Estimated parsing time increase: <5%

## Deliverables (Week 1)

✅ Modified parser.rs with indentation support  
✅ Updated TokenType enum  
✅ Hybrid parser that accepts both indentation and braces  
✅ Comprehensive error messages  
✅ 30+ unit tests covering edge cases  
✅ All 1,903 existing tests still passing  
✅ Documentation of changes  

## Next Steps (After Phase 1)

Phase 2: Update all documentation and examples  
Phase 3: Performance validation and edge case testing  
Phase 4: Release v4.2  

---

## Files to Modify

- `_TOOLS/killer_rcore/src/parser.rs` - Main changes
- `_TOOLS/killer_rcore/src/lib.rs` - Export new token types
- Tests: Create `_TOOLS/killer_rcore/tests/indentation_tests.rs`

## Estimated Effort

- Implementation: 20-30 hours
- Testing: 10-15 hours
- Documentation: 5 hours
- **Total Phase 1: 35-50 hours**

---

## Success Criteria

✅ Lexer emits INDENT/DEDENT tokens correctly  
✅ Parser accepts both indentation and braces  
✅ All 1,903 tests pass without modification  
✅ Error messages are clear and helpful  
✅ Mixed indentation properly rejected  
✅ Performance unchanged or improved  
✅ Code review passed  

