/// Native regex engine for Killer — no external crates.
/// Supports: . * + ? [] [^] ^ $ \d \w \s \n | (groups captured as strings)

#[derive(Debug, Clone)]
enum Token {
    Literal(char),
    AnyChar,          // .
    Digit,            // \d
    NonDigit,         // \D
    Word,             // \w
    NonWord,          // \W
    Space,            // \s
    NonSpace,         // \S
    Class(Vec<ClassItem>, bool), // [abc], negated=true for [^abc]
    Group(Vec<Token>),
    Alternation(Vec<Vec<Token>>),
    AnchorStart,      // ^
    AnchorEnd,        // $
}

#[derive(Debug, Clone)]
enum ClassItem {
    Single(char),
    Range(char, char),
    Digit,
    Word,
    Space,
}

#[derive(Debug, Clone)]
enum Quantifier {
    One,
    ZeroOrOne,     // ?
    ZeroOrMore,    // *
    OneOrMore,     // +
    Exact(usize),  // {n}
    AtLeast(usize),// {n,}
    Between(usize, usize), // {n,m}
}

#[derive(Debug, Clone)]
struct Node {
    token: Token,
    quantifier: Quantifier,
    greedy: bool,
}

fn parse_class(chars: &[char], i: &mut usize) -> (Vec<ClassItem>, bool) {
    let negated = *i < chars.len() && chars[*i] == '^';
    if negated { *i += 1; }
    let mut items = Vec::new();
    while *i < chars.len() && chars[*i] != ']' {
        match chars[*i] {
            '\\' if *i + 1 < chars.len() => {
                *i += 1;
                match chars[*i] {
                    'd' => items.push(ClassItem::Digit),
                    'w' => items.push(ClassItem::Word),
                    's' => items.push(ClassItem::Space),
                    c   => items.push(ClassItem::Single(c)),
                }
                *i += 1;
            }
            c => {
                if *i + 2 < chars.len() && chars[*i + 1] == '-' && chars[*i + 2] != ']' {
                    items.push(ClassItem::Range(c, chars[*i + 2]));
                    *i += 3;
                } else {
                    items.push(ClassItem::Single(c));
                    *i += 1;
                }
            }
        }
    }
    if *i < chars.len() { *i += 1; } // consume ']'
    (items, negated)
}

fn parse_tokens(chars: &[char], i: &mut usize, stop_at_pipe: bool) -> Vec<Token> {
    let mut tokens = Vec::new();
    while *i < chars.len() {
        match chars[*i] {
            ')' => break,
            '|' if stop_at_pipe => break,
            '^' => { tokens.push(Token::AnchorStart); *i += 1; }
            '$' => { tokens.push(Token::AnchorEnd); *i += 1; }
            '.' => { tokens.push(Token::AnyChar); *i += 1; }
            '[' => {
                *i += 1;
                let (items, negated) = parse_class(chars, i);
                tokens.push(Token::Class(items, negated));
            }
            '(' => {
                *i += 1;
                // Parse alternation inside group
                let mut branches: Vec<Vec<Token>> = Vec::new();
                let branch = parse_tokens(chars, i, true);
                branches.push(branch);
                while *i < chars.len() && chars[*i] == '|' {
                    *i += 1;
                    let branch = parse_tokens(chars, i, true);
                    branches.push(branch);
                }
                if *i < chars.len() && chars[*i] == ')' { *i += 1; }
                if branches.len() == 1 {
                    tokens.push(Token::Group(branches.remove(0)));
                } else {
                    tokens.push(Token::Alternation(branches));
                }
            }
            '|' => {
                // Top-level alternation
                *i += 1;
                let right = parse_tokens(chars, i, false);
                let left = std::mem::take(&mut tokens);
                tokens.push(Token::Alternation(vec![left, right]));
                break;
            }
            '\\' if *i + 1 < chars.len() => {
                *i += 1;
                let tok = match chars[*i] {
                    'd' => Token::Digit,
                    'D' => Token::NonDigit,
                    'w' => Token::Word,
                    'W' => Token::NonWord,
                    's' => Token::Space,
                    'S' => Token::NonSpace,
                    'n' => Token::Literal('\n'),
                    't' => Token::Literal('\t'),
                    'r' => Token::Literal('\r'),
                    c   => Token::Literal(c),
                };
                tokens.push(tok);
                *i += 1;
            }
            c => { tokens.push(Token::Literal(c)); *i += 1; }
        }
    }
    tokens
}

fn parse_nodes(pattern: &str) -> Vec<Node> {
    let chars: Vec<char> = pattern.chars().collect();
    let mut i = 0;
    let tokens = parse_tokens(&chars, &mut i, false);
    let mut nodes = Vec::new();
    let mut ti = 0;
    while ti < tokens.len() {
        let token = tokens[ti].clone();
        ti += 1;
        // Look ahead for quantifier
        let (quantifier, greedy) = if ti < tokens.len() {
            match &tokens[ti] {
                Token::Literal('?') => { ti += 1; (Quantifier::ZeroOrOne, true) }
                Token::Literal('*') => { ti += 1; (Quantifier::ZeroOrMore, true) }
                Token::Literal('+') => { ti += 1; (Quantifier::OneOrMore, true) }
                _ => (Quantifier::One, true),
            }
        } else {
            (Quantifier::One, true)
        };
        // Check for non-greedy modifier
        let greedy = if ti < tokens.len() {
            if let Token::Literal('?') = &tokens[ti] {
                ti += 1;
                false
            } else { greedy }
        } else { greedy };
        nodes.push(Node { token, quantifier, greedy });
    }
    nodes
}

fn match_class_item(item: &ClassItem, c: char) -> bool {
    match item {
        ClassItem::Single(x) => *x == c,
        ClassItem::Range(a, b) => c >= *a && c <= *b,
        ClassItem::Digit => c.is_ascii_digit(),
        ClassItem::Word => c.is_alphanumeric() || c == '_',
        ClassItem::Space => c.is_whitespace(),
    }
}

fn match_token(token: &Token, text: &[char], pos: usize) -> Option<usize> {
    if pos >= text.len() {
        return match token {
            Token::AnchorEnd => Some(pos),
            _ => None,
        };
    }
    match token {
        Token::Literal(c) => if text[pos] == *c { Some(pos + 1) } else { None }
        Token::AnyChar => if text[pos] != '\n' { Some(pos + 1) } else { None }
        Token::Digit => if text[pos].is_ascii_digit() { Some(pos + 1) } else { None }
        Token::NonDigit => if !text[pos].is_ascii_digit() { Some(pos + 1) } else { None }
        Token::Word => if text[pos].is_alphanumeric() || text[pos] == '_' { Some(pos + 1) } else { None }
        Token::NonWord => if !(text[pos].is_alphanumeric() || text[pos] == '_') { Some(pos + 1) } else { None }
        Token::Space => if text[pos].is_whitespace() { Some(pos + 1) } else { None }
        Token::NonSpace => if !text[pos].is_whitespace() { Some(pos + 1) } else { None }
        Token::AnchorStart => None, // handled at outer level
        Token::AnchorEnd => if pos == text.len() { Some(pos) } else { None }
        Token::Class(items, negated) => {
            let matched = items.iter().any(|item| match_class_item(item, text[pos]));
            if matched != *negated { Some(pos + 1) } else { None }
        }
        Token::Group(inner_tokens) => {
            let inner_nodes = tokens_to_nodes(inner_tokens);
            match_nodes(&inner_nodes, text, pos, 0)
        }
        Token::Alternation(branches) => {
            for branch in branches {
                let nodes = tokens_to_nodes(branch);
                if let Some(end) = match_nodes(&nodes, text, pos, 0) {
                    return Some(end);
                }
            }
            None
        }
    }
}

fn tokens_to_nodes(tokens: &[Token]) -> Vec<Node> {
    tokens.iter().map(|t| Node {
        token: t.clone(),
        quantifier: Quantifier::One,
        greedy: true,
    }).collect()
}

fn match_nodes(nodes: &[Node], text: &[char], pos: usize, node_idx: usize) -> Option<usize> {
    if node_idx >= nodes.len() {
        return Some(pos);
    }
    let node = &nodes[node_idx];

    match &node.token {
        Token::AnchorStart => {
            if pos == 0 { match_nodes(nodes, text, pos, node_idx + 1) } else { None }
        }
        Token::AnchorEnd => {
            if pos == text.len() { match_nodes(nodes, text, pos, node_idx + 1) } else { None }
        }
        _ => {
            match &node.quantifier {
                Quantifier::One => {
                    let end = match_token(&node.token, text, pos)?;
                    match_nodes(nodes, text, end, node_idx + 1)
                }
                Quantifier::ZeroOrOne => {
                    // Try with one match first (greedy)
                    if let Some(end) = match_token(&node.token, text, pos) {
                        if let Some(result) = match_nodes(nodes, text, end, node_idx + 1) {
                            return Some(result);
                        }
                    }
                    match_nodes(nodes, text, pos, node_idx + 1)
                }
                Quantifier::ZeroOrMore => {
                    // Greedy: collect all matches, then try rest from each end
                    let mut positions = vec![pos];
                    let mut cur = pos;
                    while let Some(end) = match_token(&node.token, text, cur) {
                        if end == cur { break; } // prevent infinite loop on zero-width
                        positions.push(end);
                        cur = end;
                    }
                    for &p in positions.iter().rev() {
                        if let Some(result) = match_nodes(nodes, text, p, node_idx + 1) {
                            return Some(result);
                        }
                    }
                    None
                }
                Quantifier::OneOrMore => {
                    let first = match_token(&node.token, text, pos)?;
                    let mut positions = vec![first];
                    let mut cur = first;
                    while let Some(end) = match_token(&node.token, text, cur) {
                        if end == cur { break; }
                        positions.push(end);
                        cur = end;
                    }
                    for &p in positions.iter().rev() {
                        if let Some(result) = match_nodes(nodes, text, p, node_idx + 1) {
                            return Some(result);
                        }
                    }
                    None
                }
                Quantifier::Exact(n) => {
                    let mut cur = pos;
                    for _ in 0..*n {
                        cur = match_token(&node.token, text, cur)?;
                    }
                    match_nodes(nodes, text, cur, node_idx + 1)
                }
                Quantifier::AtLeast(n) => {
                    let mut cur = pos;
                    for _ in 0..*n {
                        cur = match_token(&node.token, text, cur)?;
                    }
                    // Then zero or more
                    let mut positions = vec![cur];
                    while let Some(end) = match_token(&node.token, text, cur) {
                        if end == cur { break; }
                        positions.push(end);
                        cur = end;
                    }
                    for &p in positions.iter().rev() {
                        if let Some(result) = match_nodes(nodes, text, p, node_idx + 1) {
                            return Some(result);
                        }
                    }
                    None
                }
                Quantifier::Between(n, m) => {
                    let mut cur = pos;
                    for _ in 0..*n {
                        cur = match_token(&node.token, text, cur)?;
                    }
                    let mut positions = vec![cur];
                    for _ in *n..*m {
                        if let Some(end) = match_token(&node.token, text, cur) {
                            if end == cur { break; }
                            positions.push(end);
                            cur = end;
                        } else { break; }
                    }
                    for &p in positions.iter().rev() {
                        if let Some(result) = match_nodes(nodes, text, p, node_idx + 1) {
                            return Some(result);
                        }
                    }
                    None
                }
            }
        }
    }
}

/// Find first match of pattern in text. Returns (start, end) byte offsets or None.
pub fn find(pattern: &str, text: &str) -> Option<(usize, usize)> {
    let nodes = parse_nodes(pattern);
    let chars: Vec<char> = text.chars().collect();
    let anchor_start = nodes.first().map(|n| matches!(n.token, Token::AnchorStart)).unwrap_or(false);

    let search_range = if anchor_start { 0..1 } else { 0..chars.len() + 1 };
    for start in search_range {
        if start > chars.len() { break; }
        if let Some(end) = match_nodes(&nodes, &chars, start, 0) {
            // Convert char indices to byte indices
            let byte_start: usize = chars[..start].iter().map(|c| c.len_utf8()).sum();
            let byte_end: usize = chars[..end].iter().map(|c| c.len_utf8()).sum();
            return Some((byte_start, byte_end));
        }
    }
    None
}

/// Returns true if pattern matches anywhere in text.
pub fn is_match(pattern: &str, text: &str) -> bool {
    find(pattern, text).is_some()
}

/// Returns all non-overlapping matches as Vec of (matched_string, start, end).
pub fn find_all(pattern: &str, text: &str) -> Vec<String> {
    let nodes = parse_nodes(pattern);
    let chars: Vec<char> = text.chars().collect();
    let mut results = Vec::new();
    let mut pos = 0;

    while pos <= chars.len() {
        if let Some(end) = match_nodes(&nodes, &chars, pos, 0) {
            if end == pos {
                pos += 1;
                continue;
            }
            results.push(chars[pos..end].iter().collect());
            pos = end;
        } else {
            pos += 1;
        }
    }
    results
}

/// Replace all matches of pattern in text with replacement.
/// Supports \0 or \& for full match in replacement string.
pub fn replace_all(pattern: &str, text: &str, replacement: &str) -> String {
    let nodes = parse_nodes(pattern);
    let chars: Vec<char> = text.chars().collect();
    let mut result = String::new();
    let mut pos = 0;

    while pos <= chars.len() {
        if let Some(end) = match_nodes(&nodes, &chars, pos, 0) {
            if end == pos {
                if pos < chars.len() { result.push(chars[pos]); }
                pos += 1;
                continue;
            }
            let matched: String = chars[pos..end].iter().collect();
            let repl = replacement.replace("\\0", &matched).replace("\\&", &matched);
            result.push_str(&repl);
            pos = end;
        } else {
            if pos < chars.len() { result.push(chars[pos]); }
            pos += 1;
        }
    }
    result
}

/// Split text by pattern matches.
pub fn split(pattern: &str, text: &str) -> Vec<String> {
    let nodes = parse_nodes(pattern);
    let chars: Vec<char> = text.chars().collect();
    let mut parts = Vec::new();
    let mut last = 0;
    let mut pos = 0;

    while pos <= chars.len() {
        if let Some(end) = match_nodes(&nodes, &chars, pos, 0) {
            if end == pos { pos += 1; continue; }
            parts.push(chars[last..pos].iter().collect());
            last = end;
            pos = end;
        } else {
            pos += 1;
        }
    }
    parts.push(chars[last..].iter().collect());
    parts
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test] fn test_literal() { assert!(is_match("hello", "say hello world")); }
    #[test] fn test_dot() { assert!(is_match("h.llo", "hello")); }
    #[test] fn test_star() { assert!(is_match("ab*c", "ac")); assert!(is_match("ab*c", "abbc")); }
    #[test] fn test_plus() { assert!(is_match("ab+c", "abc")); assert!(!is_match("ab+c", "ac")); }
    #[test] fn test_digit() { assert!(is_match("\\d+", "42")); assert!(!is_match("\\d+", "abc")); }
    #[test] fn test_word() { assert!(is_match("\\w+", "hello_123")); }
    #[test] fn test_class() { assert!(is_match("[aeiou]", "hello")); assert!(!is_match("[^aeiou]", "a")); }
    #[test] fn test_anchor_start() { assert!(is_match("^hello", "hello world")); assert!(!is_match("^world", "hello world")); }
    #[test] fn test_anchor_end() { assert!(is_match("world$", "hello world")); }
    #[test] fn test_find_all() {
        let m = find_all("\\d+", "abc 123 def 456");
        assert_eq!(m, vec!["123", "456"]);
    }
    #[test] fn test_replace_all() {
        let r = replace_all("\\d+", "a1b2c3", "N");
        assert_eq!(r, "aNbNcN");
    }
    #[test] fn test_split() {
        let parts = split(",", "a,b,c");
        assert_eq!(parts, vec!["a", "b", "c"]);
    }
    #[test] fn test_alternation() {
        assert!(is_match("cat|dog", "I have a cat"));
        assert!(is_match("cat|dog", "I have a dog"));
        assert!(!is_match("cat|dog", "I have a fish"));
    }
    #[test] fn test_optional() {
        assert!(is_match("colou?r", "color"));
        assert!(is_match("colou?r", "colour"));
    }
}
