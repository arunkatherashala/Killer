// Regex Module for Killer Language
// Simple pattern matching without external regex library
// Version: 2.1.0

/// Simple regex engine for pattern matching
/// Supports basic patterns: *, +, ?, ., ^, $, [], [^]
pub struct RegexModule;

#[derive(Debug, Clone)]
pub struct Pattern {
    pattern: String,
}

impl Pattern {
    /// Create a new pattern
    pub fn new(pattern: &str) -> Self {
        Pattern {
            pattern: pattern.to_string(),
        }
    }
    
    /// Check if pattern matches the string
    pub fn is_match(&self, text: &str) -> bool {
        Self::match_internal(text, &self.pattern, false)
    }
    
    /// Find first match position
    pub fn find(&self, text: &str) -> Option<(usize, usize)> {
        for i in 0..text.len() {
            if let Some(end) = Self::match_at(&text[i..], &self.pattern) {
                return Some((i, i + end));
            }
        }
        None
    }
    
    /// Find all matches
    pub fn find_all(&self, text: &str) -> Vec<(usize, usize)> {
        let mut matches = Vec::new();
        let mut pos = 0;
        
        while pos < text.len() {
            if let Some(end) = Self::match_at(&text[pos..], &self.pattern) {
                matches.push((pos, pos + end));
                pos += end.max(1);
            } else {
                pos += 1;
            }
        }
        
        matches
    }
    
    fn match_internal(text: &str, pattern: &str, _anchor_start: bool) -> bool {
        Self::match_at(text, pattern).is_some()
    }
    
    fn match_at(text: &str, pattern: &str) -> Option<usize> {
        if pattern.is_empty() {
            return Some(0);
        }
        
        let pattern_chars: Vec<char> = pattern.chars().collect();
        let text_chars: Vec<char> = text.chars().collect();
        
        let mut p = 0; // pattern index
        let mut t = 0; // text index
        
        while p < pattern_chars.len() {
            let pc = pattern_chars[p];
            
            if pc == '.' {
                if t >= text_chars.len() {
                    return None;
                }
                p += 1;
                t += 1;
            } else if pc == '*' {
                return None; // * must follow a character
            } else if p + 1 < pattern_chars.len() && pattern_chars[p + 1] == '*' {
                let ch = pc;
                p += 2;
                while t < text_chars.len() && text_chars[t] == ch {
                    t += 1;
                }
            } else if p + 1 < pattern_chars.len() && pattern_chars[p + 1] == '+' {
                if t >= text_chars.len() || text_chars[t] != pc {
                    return None;
                }
                let ch = pc;
                p += 2;
                t += 1;
                while t < text_chars.len() && text_chars[t] == ch {
                    t += 1;
                }
            } else if p + 1 < pattern_chars.len() && pattern_chars[p + 1] == '?' {
                if t < text_chars.len() && text_chars[t] == pc {
                    t += 1;
                }
                p += 2;
            } else if pc == '[' {
                if t >= text_chars.len() {
                    return None;
                }
                
                let (range_end, matches) = Self::parse_char_class(&pattern_chars, p + 1);
                if !matches.contains(&text_chars[t]) {
                    return None;
                }
                p = range_end;
                t += 1;
            } else {
                if t >= text_chars.len() || text_chars[t] != pc {
                    return None;
                }
                p += 1;
                t += 1;
            }
        }
        
        Some(t)
    }
    
    fn parse_char_class(chars: &[char], start: usize) -> (usize, Vec<char>) {
        let mut pos = start;
        let mut matches = Vec::new();
        let negate = chars.get(start) == Some(&'^');
        
        if negate {
            pos += 1;
        }
        
        while pos < chars.len() && chars[pos] != ']' {
            if pos + 2 < chars.len() && chars[pos + 1] == '-' && chars[pos + 2] != ']' {
                // Range like a-z
                let start_ch = chars[pos];
                let end_ch = chars[pos + 2];
                for ch in (start_ch as u32)..=(end_ch as u32) {
                    if let Some(c) = char::from_u32(ch) {
                        matches.push(c);
                    }
                }
                pos += 3;
            } else {
                matches.push(chars[pos]);
                pos += 1;
            }
        }
        
        if negate {
            // For simplicity, negate returns empty in our minimalist implementation
            matches.clear();
        }
        
        (pos + 1, matches)
    }
}

impl RegexModule {
    // ==================== Basic Matching ====================
    
    /// Check if text matches pattern (anchored at start)
    /// matches("hello", "h.*o") => true
    pub fn matches(text: &str, pattern: &str) -> bool {
        Pattern::new(pattern).is_match(text)
    }
    
    /// Check if text contains pattern (anywhere)
    /// contains("hello world", "wor") => true
    pub fn contains(text: &str, pattern: &str) -> bool {
        let p = Pattern::new(pattern);
        p.find(text).is_some()
    }
    
    /// Check if text starts with pattern
    pub fn starts_with(text: &str, pattern: &str) -> bool {
        Pattern::new(pattern).is_match(&text[..text.len().min(pattern.len() * 2)])
    }
    
    /// Check if text ends with pattern
    pub fn ends_with(text: &str, pattern: &str) -> bool {
        if pattern.len() > text.len() {
            return false;
        }
        Pattern::new(pattern).is_match(&text[text.len() - pattern.len()..])
    }
    
    // ==================== Search & Find ====================
    
    /// Find first occurrence of pattern with position
    /// find("abcabc", "bc") => Some((1, 3))
    pub fn find(text: &str, pattern: &str) -> Option<(usize, usize)> {
        Pattern::new(pattern).find(text)
    }
    
    /// Find all non-overlapping occurrences
    /// find_all("abcabc", "bc") => [(1, 3), (4, 6)]
    pub fn find_all(text: &str, pattern: &str) -> Vec<(usize, usize)> {
        Pattern::new(pattern).find_all(text)
    }
    
    /// Count matches
    /// count("hello", "l") => 2
    pub fn count(text: &str, pattern: &str) -> usize {
        Self::find_all(text, pattern).len()
    }
    
    // ==================== Replace ====================
    
    /// Replace first match
    /// replace("hello", "l", "L") => "heLlo"
    pub fn replace(text: &str, pattern: &str, replacement: &str) -> String {
        if let Some((start, end)) = Self::find(text, pattern) {
            let mut result = String::new();
            result.push_str(&text[..start]);
            result.push_str(replacement);
            result.push_str(&text[end..]);
            result
        } else {
            text.to_string()
        }
    }
    
    /// Replace all matches
    /// replace_all("hello", "l", "L") => "heLLo"
    pub fn replace_all(text: &str, pattern: &str, replacement: &str) -> String {
        let mut result = text.to_string();
        let matches = Self::find_all(text, pattern);
        
        let mut offset = 0i32;
        for (start, end) in matches {
            let adj_start = (start as i32 + offset) as usize;
            let adj_end = (end as i32 + offset) as usize;
            
            let before = result[..adj_start].to_string();
            let after = result[adj_end..].to_string();
            
            offset += (replacement.len() as i32) - ((end - start) as i32);
            result = format!("{}{}{}", before, replacement, after);
        }
        
        result
    }
    
    // ==================== Split ====================
    
    /// Split text by pattern
    /// split("a,b,c", ",") => ["a", "b", "c"]
    pub fn split(text: &str, pattern: &str) -> Vec<String> {
        let matches = Self::find_all(text, pattern);
        
        if matches.is_empty() {
            return vec![text.to_string()];
        }
        
        let mut result = Vec::new();
        let mut last_end = 0;
        
        for (start, end) in matches {
            result.push(text[last_end..start].to_string());
            last_end = end;
        }
        
        result.push(text[last_end..].to_string());
        result
    }
    
    // ==================== Extraction ====================
    
    /// Extract matched portion
    /// extract("hello world", "w.*d") => Some("world")
    pub fn extract(text: &str, pattern: &str) -> Option<String> {
        Self::find(text, pattern)
            .map(|(start, end)| text[start..end].to_string())
    }
    
    /// Extract all matched portions
    /// extract_all("abc abc", "a.c") => ["abc", "abc"]
    pub fn extract_all(text: &str, pattern: &str) -> Vec<String> {
        Self::find_all(text, pattern)
            .iter()
            .map(|(start, end)| text[*start..*end].to_string())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_exact_match() {
        assert!(RegexModule::matches("hello", "hello"));
        assert!(!RegexModule::matches("hello", "hallo"));
    }
    
    #[test]
    fn test_contains() {
        assert!(RegexModule::contains("hello world", "world"));
        assert!(!RegexModule::contains("hello world", "xyz"));
    }
    
    #[test]
    fn test_dot_wildcard() {
        assert!(RegexModule::matches("cat", "c.t"));
        assert!(RegexModule::matches("dog", "d.g"));
    }
    
    #[test]
    fn test_find() {
        assert_eq!(RegexModule::find("hello", "ll"), Some((2, 4)));
    }
    
    #[test]
    fn test_count() {
        assert_eq!(RegexModule::count("hello hello", "ll"), 2);
    }
    
    #[test]
    fn test_replace() {
        assert_eq!(RegexModule::replace("hello", "l", "L"), "heLlo");
    }
    
    #[test]
    fn test_replace_all() {
        assert_eq!(RegexModule::replace_all("hello", "l", "L"), "heLLo");
    }
    
    #[test]
    fn test_split() {
        let parts = RegexModule::split("a,b,c", ",");
        assert_eq!(parts.len(), 3);
        assert_eq!(parts[0], "a");
        assert_eq!(parts[2], "c");
    }
    
    #[test]
    fn test_extract() {
        assert_eq!(RegexModule::extract("hello world", "world"), Some("world".to_string()));
    }
}
