#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    Let,
    Fn,
    Class,
    Extends,
    New,
    This,
    For,
    In,
    Of,
    Return,
    If,
    Else,
    While,
    Break,
    Continue,
    Try,
    Catch,
    Finally,
    Throw,
    Yield,
    Switch,
    Case,
    Default,
    Match,
    Do,
    Typeof,
    True,
    False,
    Null,
    Print,
    Quality,
    Identifier(String),
    Number(f64),
    String(String),
    Template(String),
    KString(String),
    Plus,
    PlusPlus,
    PlusEqual,
    Minus,
    MinusMinus,
    MinusEqual,
    Star,
    DoubleStar,
    StarEqual,
    Slash,
    /// Python-style floor division `//` (line comments use `#` only).
    DoubleSlash,
    SlashEqual,
    Percent,
    PercentEqual,
    Equal,
    EqualEqual,
    BangEqual,
    Greater,
    GreaterEqual,
    Less,
    LessEqual,
    AndAnd,
    OrOr,
    Arrow,
    LParen,
    RParen,
    LBrace,
    RBrace,
    LBracket,
    RBracket,
    Comma,
    Semicolon,
    Colon,
    At,
    /// `@python`, `@java`, `@bash` etc — Nova Galaxy Engine polyglot annotation
    LangAnnotation(String),
    Question,
    Dot,
    DotDot,
    DotDotDot,
    Newline,
    Indent,
    Dedent,
    Eof,
    // Async / concurrency / packages (v2.2)
    Async,
    Await,
    Spawn,
    Import,
}

#[derive(Debug, Clone)]
pub struct Token {
    pub kind: TokenKind,
    pub col: usize,
}

#[allow(dead_code)]
pub struct Lexer {
    input: Vec<char>,
    position: usize,
    indent_stack: Vec<usize>,
    pending_dedents: usize,
    at_line_start: bool,
}

impl Lexer {
    fn new(source: &str) -> Self {
        let mut input: Vec<char> = source.chars().collect();
        
        // Skip UTF-8 BOM if present
        if input.len() >= 1 && input[0] == '\u{FEFF}' {
            input.remove(0);
        }
        
        Lexer {
            input,
            position: 0,
            indent_stack: vec![0],
            pending_dedents: 0,
            at_line_start: true,
        }
    }

    fn is_at_end(&self) -> bool {
        self.position >= self.input.len()
    }

    fn current_char(&self) -> char {
        if self.is_at_end() {
            '\0'
        } else {
            self.input[self.position]
        }
    }

    #[allow(dead_code)]
    fn peek_char(&self, offset: usize) -> char {
        let pos = self.position + offset;
        if pos >= self.input.len() {
            '\0'
        } else {
            self.input[pos]
        }
    }

    fn advance(&mut self) -> char {
        if self.is_at_end() {
            return '\0';
        }
        let ch = self.input[self.position];
        self.position += 1;
        ch
    }

    fn skip_whitespace(&mut self) {
        while !self.is_at_end() && self.current_char() != '\n' && self.current_char().is_whitespace() {
            self.advance();
        }
    }

    fn skip_whitespace_inline(&mut self) {
        while !self.is_at_end() && (self.current_char() == ' ' || self.current_char() == '\t') {
            self.advance();
        }
    }

    fn get_line_indent(&mut self) -> usize {
        let mut indent = 0;
        while !self.is_at_end() && (self.current_char() == ' ' || self.current_char() == '\t') {
            if self.current_char() == ' ' {
                indent += 1;
            } else {
                indent += 4; // Tab = 4 spaces
            }
            self.advance();
        }
        indent
    }

    fn skip_comment(&mut self) {
        if self.current_char() == '#' {
            while !self.is_at_end() && self.current_char() != '\n' {
                self.advance();
            }
        }
    }

    fn read_string(&mut self) -> Result<String, String> {
        self.advance();
        let mut result = String::new();
        while !self.is_at_end() && self.current_char() != '"' {
            if self.current_char() == '\\' {
                self.advance();
                match self.current_char() {
                    'n' => result.push('\n'),
                    't' => result.push('\t'),
                    'r' => result.push('\r'),
                    '\\' => result.push('\\'),
                    '"' => result.push('"'),
                    _ => result.push(self.current_char()),
                }
                self.advance();
            } else {
                result.push(self.current_char());
                self.advance();
            }
        }
        if self.is_at_end() {
            return Err("unterminated string".to_string());
        }
        self.advance();
        Ok(result)
    }

    fn read_template_string(&mut self) -> Result<String, String> {
        // Current char is `
        self.advance();
        let mut result = String::new();
        while !self.is_at_end() && self.current_char() != '`' {
            if self.current_char() == '\\' {
                self.advance();
                match self.current_char() {
                    'n' => result.push('\n'),
                    't' => result.push('\t'),
                    'r' => result.push('\r'),
                    '\\' => result.push('\\'),
                    '`' => result.push('`'),
                    _ => result.push(self.current_char()),
                }
                self.advance();
            } else {
                result.push(self.current_char());
                self.advance();
            }
        }
        if self.is_at_end() {
            return Err("unterminated template string".to_string());
        }
        self.advance();
        Ok(result)
    }

    fn read_kstring(&mut self) -> Result<String, String> {
        // Current char is k, next char is "
        self.advance(); // skip 'k'
        self.advance(); // skip '"'
        let mut result = String::new();
        while !self.is_at_end() && self.current_char() != '"' {
            if self.current_char() == '\\' {
                self.advance();
                match self.current_char() {
                    'n' => result.push('\n'),
                    't' => result.push('\t'),
                    'r' => result.push('\r'),
                    '\\' => result.push('\\'),
                    '"' => result.push('"'),
                    _ => result.push(self.current_char()),
                }
                self.advance();
            } else {
                result.push(self.current_char());
                self.advance();
            }
        }
        if self.is_at_end() {
            return Err("unterminated k-string".to_string());
        }
        self.advance(); // skip closing '"'
        Ok(result)
    }

    fn read_number(&mut self) -> f64 {
        let mut result = String::new();
        while !self.is_at_end() && (self.current_char().is_numeric() || self.current_char() == '.') {
            result.push(self.current_char());
            self.advance();
        }
        result.parse().unwrap_or(0.0)
    }

    fn read_identifier(&mut self) -> String {
        let mut result = String::new();
        while !self.is_at_end()
            && (self.current_char().is_alphanumeric() || self.current_char() == '_')
        {
            result.push(self.current_char());
            self.advance();
        }
        result
    }

    fn keyword_or_identifier(&self, word: &str) -> TokenKind {
        match word {
            "let" => TokenKind::Let,
            "fn" => TokenKind::Fn,
            "kfn" => TokenKind::Fn,
            "class" => TokenKind::Class,
            "extends" => TokenKind::Extends,
            "new" => TokenKind::New,
            "this" => TokenKind::This,
            "for" => TokenKind::For,
            "in" => TokenKind::In,
            "of" => TokenKind::Of,
            "return" => TokenKind::Return,
            "if" => TokenKind::If,
            "else" => TokenKind::Else,
            "while" => TokenKind::While,
            "break" => TokenKind::Break,
            "continue" => TokenKind::Continue,
            "try" => TokenKind::Try,
            "catch" => TokenKind::Catch,
            "finally" => TokenKind::Finally,
            "throw" => TokenKind::Throw,
            "yield" => TokenKind::Yield,
            "switch" => TokenKind::Switch,
            "case" => TokenKind::Case,
            "default" => TokenKind::Default,
            "match" => TokenKind::Match,
            "do" => TokenKind::Do,
            "typeof" => TokenKind::Typeof,
            "true" => TokenKind::True,
            "false" => TokenKind::False,
            "null" => TokenKind::Null,
            "print" => TokenKind::Print,
            "quality"  => TokenKind::Quality,
            "async"   => TokenKind::Async,
            "await"   => TokenKind::Await,
            "spawn"   => TokenKind::Spawn,
            "import"  => TokenKind::Import,
            _ => TokenKind::Identifier(word.to_string()),
        }
    }

    fn next_token(&mut self) -> Result<Token, String> {
        loop {
            self.skip_whitespace();
            if self.current_char() == '#' {
                self.skip_comment();
            } else {
                break;
            }
        }

        if self.is_at_end() {
            return Ok(Token {
                col: 0,
                kind: TokenKind::Eof,
            });
        }

        // Handle newline characters
        if self.current_char() == '\n' {
            self.advance();
            return Ok(Token {
                col: 0,
                kind: TokenKind::Newline,
            });
        }

        match self.current_char() {
            '(' => {
                self.advance();
                Ok(Token {
                    col: 0,
                    kind: TokenKind::LParen,
                })
            }
            ')' => {
                self.advance();
                Ok(Token {
                    col: 0,
                    kind: TokenKind::RParen,
                })
            }
            '{' => {
                self.advance();
                Ok(Token {
                    col: 0,
                    kind: TokenKind::LBrace,
                })
            }
            '}' => {
                self.advance();
                Ok(Token {
                    col: 0,
                    kind: TokenKind::RBrace,
                })
            }
            '[' => {
                self.advance();
                Ok(Token {
                    col: 0,
                    kind: TokenKind::LBracket,
                })
            }
            ']' => {
                self.advance();
                Ok(Token {
                    col: 0,
                    kind: TokenKind::RBracket,
                })
            }
            ':' => {
                self.advance();
                Ok(Token {
                    col: 0,
                    kind: TokenKind::Colon,
                })
            }
            '@' => {
                self.advance();
                // Nova Galaxy Engine: @lang — consume the identifier after @
                if !self.is_at_end() && (self.current_char().is_alphabetic() || self.current_char() == '_') {
                    let mut lang = String::new();
                    while !self.is_at_end() && (self.current_char().is_alphanumeric() || self.current_char() == '_') {
                        lang.push(self.current_char());
                        self.advance();
                    }
                    Ok(Token {
                        col: 0,
                        kind: TokenKind::LangAnnotation(lang),
                    })
                } else {
                    Ok(Token {
                        col: 0,
                        kind: TokenKind::At,
                    })
                }
            }
            '?' => {
                self.advance();
                Ok(Token {
                    col: 0,
                    kind: TokenKind::Question,
                })
            }
            ',' => {
                self.advance();
                Ok(Token {
                    col: 0,
                    kind: TokenKind::Comma,
                })
            }
            '.' => {
                self.advance();
                // Check for spread operator (...) and range operator (..)
                if self.position < self.input.len() && self.input[self.position] == '.' {
                    self.advance();
                    // Check for three dots (...)
                    if self.position < self.input.len() && self.input[self.position] == '.' {
                        self.advance();
                        Ok(Token {
                            col: 0,
                            kind: TokenKind::DotDotDot,
                        })
                    } else {
                        // Two dots (..)
                        Ok(Token {
                            col: 0,
                            kind: TokenKind::DotDot,
                        })
                    }
                } else {
                    Ok(Token {
                        col: 0,
                        kind: TokenKind::Dot,
                    })
                }
            }
            ';' => {
                self.advance();
                Ok(Token {
                    col: 0,
                    kind: TokenKind::Semicolon,
                })
            }
            '+' => {
                self.advance();
                if self.current_char() == '+' {
                    self.advance();
                    Ok(Token {
                        col: 0,
                        kind: TokenKind::PlusPlus,
                    })
                } else if self.current_char() == '=' {
                    self.advance();
                    Ok(Token {
                        col: 0,
                        kind: TokenKind::PlusEqual,
                    })
                } else {
                    Ok(Token {
                        col: 0,
                        kind: TokenKind::Plus,
                    })
                }
            }
            '-' => {
                self.advance();
                if self.current_char() == '-' {
                    self.advance();
                    Ok(Token {
                        col: 0,
                        kind: TokenKind::MinusMinus,
                    })
                } else if self.current_char() == '=' {
                    self.advance();
                    Ok(Token {
                        col: 0,
                        kind: TokenKind::MinusEqual,
                    })
                } else {
                    Ok(Token {
                        col: 0,
                        kind: TokenKind::Minus,
                    })
                }
            }
            '*' => {
                self.advance();
                if self.current_char() == '*' {
                    self.advance();
                    Ok(Token {
                        col: 0,
                        kind: TokenKind::DoubleStar,
                    })
                } else if self.current_char() == '=' {
                    self.advance();
                    Ok(Token {
                        col: 0,
                        kind: TokenKind::StarEqual,
                    })
                } else {
                    Ok(Token {
                        col: 0,
                        kind: TokenKind::Star,
                    })
                }
            }
            '/' => {
                self.advance();
                if self.current_char() == '/' {
                    self.advance();
                    Ok(Token {
                        col: 0,
                        kind: TokenKind::DoubleSlash,
                    })
                } else if self.current_char() == '=' {
                    self.advance();
                    Ok(Token {
                        col: 0,
                        kind: TokenKind::SlashEqual,
                    })
                } else {
                    Ok(Token {
                        col: 0,
                        kind: TokenKind::Slash,
                    })
                }
            }
            '%' => {
                self.advance();
                if self.current_char() == '=' {
                    self.advance();
                    Ok(Token {
                        col: 0,
                        kind: TokenKind::PercentEqual,
                    })
                } else {
                    Ok(Token {
                        col: 0,
                        kind: TokenKind::Percent,
                    })
                }
            }
            '=' => {
                self.advance();
                if self.current_char() == '=' {
                    self.advance();
                    Ok(Token {
                        col: 0,
                        kind: TokenKind::EqualEqual,
                    })
                } else if self.current_char() == '>' {
                    self.advance();
                    Ok(Token {
                        col: 0,
                        kind: TokenKind::Arrow,
                    })
                } else {
                    Ok(Token {
                        col: 0,
                        kind: TokenKind::Equal,
                    })
                }
            }
            '!' => {
                self.advance();
                if self.current_char() == '=' {
                    self.advance();
                    Ok(Token {
                        col: 0,
                        kind: TokenKind::BangEqual,
                    })
                } else {
                    Err("unexpected character '!'".to_string())
                }
            }
            '>' => {
                self.advance();
                if self.current_char() == '=' {
                    self.advance();
                    Ok(Token {
                        col: 0,
                        kind: TokenKind::GreaterEqual,
                    })
                } else {
                    Ok(Token {
                        col: 0,
                        kind: TokenKind::Greater,
                    })
                }
            }
            '<' => {
                self.advance();
                if self.current_char() == '=' {
                    self.advance();
                    Ok(Token {
                        col: 0,
                        kind: TokenKind::LessEqual,
                    })
                } else {
                    Ok(Token {
                        col: 0,
                        kind: TokenKind::Less,
                    })
                }
            }
            '&' => {
                self.advance();
                if self.current_char() == '&' {
                    self.advance();
                    Ok(Token {
                        col: 0,
                        kind: TokenKind::AndAnd,
                    })
                } else {
                    Err("expected '&&'".to_string())
                }
            }
            '|' => {
                self.advance();
                if self.current_char() == '|' {
                    self.advance();
                    Ok(Token {
                        col: 0,
                        kind: TokenKind::OrOr,
                    })
                } else {
                    Err("expected '||'".to_string())
                }
            }
            '"' => {
                let s = self.read_string()?;
                Ok(Token {
                    col: 0,
                    kind: TokenKind::String(s),
                })
            }
            '`' => {
                let s = self.read_template_string()?;
                Ok(Token {
                    col: 0,
                    kind: TokenKind::Template(s),
                })
            }
            ch if ch.is_numeric() => {
                let num = self.read_number();
                Ok(Token {
                    col: 0,
                    kind: TokenKind::Number(num),
                })
            }
            ch if ch.is_alphabetic() || ch == '_' => {
                let word = self.read_identifier();
                
                // Check for K-string (k"...")
                if word == "k" && !self.is_at_end() && self.current_char() == '"' {
                    let s = self.read_kstring()?;
                    Ok(Token {
                        col: 0,
                        kind: TokenKind::KString(s),
                    })
                } else {
                    let kind = self.keyword_or_identifier(&word);
                    Ok(Token { col: 0, kind })
                }
            }
            ch => Err(format!("unexpected character: '{}'", ch)),
        }
    }
}

/// Skip one line-start prefix: inline whitespace, full-line comment, or restore position (same as former single-iteration loop).
fn skip_line_prefix_for_indent(lexer: &mut Lexer) {
    let saved_pos = lexer.position;
    lexer.skip_whitespace_inline(); // Space/tab only, not newlines
    if lexer.current_char() == '\n' {
        lexer.advance();
    } else if lexer.current_char() == '#' {
        lexer.skip_comment();
        if lexer.current_char() == '\n' {
            lexer.advance();
        }
    } else {
        lexer.position = saved_pos;
    }
}

pub fn lex(source: &str) -> Result<Vec<Token>, String> {
    let mut lexer = Lexer::new(source);
    let mut tokens = Vec::new();
    let mut indent_stack = vec![0usize];
    let mut at_line_start = true;
    let mut current_line_col: usize = 0; // column of first token on this line

    loop {
        // Check indentation at line start
        if at_line_start && !lexer.is_at_end() && lexer.current_char() != '\n' {
            // Skip empty lines and comments (single pass; was `loop` with only `break` paths — clippy never_loop)
            skip_line_prefix_for_indent(&mut lexer);

            if !lexer.is_at_end() && lexer.current_char() != '\n' {
                let current_indent = lexer.get_line_indent();
                current_line_col = current_indent;
                let previous_indent = *indent_stack.last().unwrap_or(&0);

                if current_indent > previous_indent {
                    indent_stack.push(current_indent);
                } else if current_indent < previous_indent {
                    while indent_stack.len() > 1 {
                        if let Some(&last_indent) = indent_stack.last() {
                            if last_indent <= current_indent {
                                break;
                            }
                            indent_stack.pop();
                        } else {
                            break;
                        }
                    }
                }
            }

            at_line_start = false;
        }

        let token = lexer.next_token()?;
        let is_eof = matches!(token.kind, TokenKind::Eof);

        // Handle newlines and reset indent tracking
        if matches!(token.kind, TokenKind::Newline | TokenKind::Indent | TokenKind::Dedent) {
            at_line_start = true;
        } else {
            tokens.push(Token { kind: token.kind, col: current_line_col });
        }

        if is_eof {
            break;
        }
    }

    Ok(tokens)
}

/// Like [`lex`], but inserts [`TokenKind::Newline`] into the stream so statement boundaries are visible.
pub fn lex_with_newlines(source: &str) -> Result<Vec<Token>, String> {
    let mut lexer = Lexer::new(source);
    let mut tokens = Vec::new();
    let mut indent_stack = vec![0usize];
    let mut at_line_start = true;
    let mut current_line_col: usize = 0;

    loop {
        if at_line_start && !lexer.is_at_end() && lexer.current_char() != '\n' {
            skip_line_prefix_for_indent(&mut lexer);

            if !lexer.is_at_end() && lexer.current_char() != '\n' {
                let current_indent = lexer.get_line_indent();
                current_line_col = current_indent;
                let previous_indent = *indent_stack.last().unwrap_or(&0);

                if current_indent > previous_indent {
                    indent_stack.push(current_indent);
                } else if current_indent < previous_indent {
                    while indent_stack.len() > 1 {
                        if let Some(&last_indent) = indent_stack.last() {
                            if last_indent <= current_indent {
                                break;
                            }
                            indent_stack.pop();
                        } else {
                            break;
                        }
                    }
                }
            }

            at_line_start = false;
        }

        let token = lexer.next_token()?;
        let is_eof = matches!(token.kind, TokenKind::Eof);

        if matches!(token.kind, TokenKind::Newline | TokenKind::Indent | TokenKind::Dedent) {
            if matches!(token.kind, TokenKind::Newline) {
                tokens.push(Token {
                    kind: TokenKind::Newline,
                    col: current_line_col,
                });
            }
            at_line_start = true;
        } else {
            tokens.push(Token {
                kind: token.kind,
                col: current_line_col,
            });
        }

        if is_eof {
            break;
        }
    }

    Ok(tokens)
}
