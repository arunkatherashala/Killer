// String Module for Killer Language
// Comprehensive string manipulation functions
// Version: 2.1.0

/// String module providing 25+ string manipulation functions
/// Includes: case conversion, searching, splitting, replacing, trimming, padding, and more
pub struct StringModule;

impl StringModule {
    // ==================== Case Conversion ====================
    
    /// Convert to uppercase
    /// uppercase("hello") => "HELLO"
    pub fn uppercase(s: &str) -> String {
        s.to_uppercase()
    }
    
    /// Convert to lowercase
    /// lowercase("HELLO") => "hello"
    pub fn lowercase(s: &str) -> String {
        s.to_lowercase()
    }
    
    /// Convert to title case (capitalize first letter)
    /// capitalize("hello world") => "Hello world"
    pub fn capitalize(s: &str) -> String {
        if s.is_empty() {
            String::new()
        } else {
            let mut chars = s.chars();
            chars.next().unwrap().to_uppercase().collect::<String>() + &chars.as_str().to_lowercase()
        }
    }
    
    /// Convert to title case (capitalize each word)
    /// title_case("hello world") => "Hello World"
    pub fn title_case(s: &str) -> String {
        s.split_whitespace()
            .map(|word| {
                if word.is_empty() {
                    word.to_string()
                } else {
                    let mut chars = word.chars();
                    chars.next().unwrap().to_uppercase().collect::<String>() + &chars.as_str().to_lowercase()
                }
            })
            .collect::<Vec<_>>()
            .join(" ")
    }
    
    /// Convert to camelCase
    /// camel_case("hello world test") => "helloWorldTest"
    pub fn camel_case(s: &str) -> String {
        let words: Vec<&str> = s.split_whitespace().collect();
        if words.is_empty() {
            return String::new();
        }
        
        let mut result = words[0].to_lowercase();
        for word in &words[1..] {
            let mut chars = word.chars();
            result.push_str(&chars.next().unwrap().to_uppercase().collect::<String>());
            result.push_str(&chars.as_str().to_lowercase());
        }
        result
    }
    
    /// Convert to snake_case
    /// snake_case("HelloWorld") => "hello_world"
    pub fn snake_case(s: &str) -> String {
        let mut result = String::new();
        for (i, ch) in s.chars().enumerate() {
            if ch.is_uppercase() && i > 0 {
                result.push('_');
                result.push_str(&ch.to_lowercase().to_string());
            } else {
                result.push(ch);
            }
        }
        result.replace(' ', "_")
    }
    
    /// Convert to kebab-case
    /// kebab_case("Hello World") => "hello-world"
    pub fn kebab_case(s: &str) -> String {
        Self::snake_case(s).replace('_', "-")
    }
    
    // ==================== Searching & Finding ====================
    
    /// Find index of substring (first occurrence)
    /// index_of("hello world", "world") => Some(6)
    pub fn index_of(s: &str, search: &str) -> Option<usize> {
        s.find(search)
    }
    
    /// Find index of substring (last occurrence)
    /// last_index_of("hello world hello", "hello") => Some(12)
    pub fn last_index_of(s: &str, search: &str) -> Option<usize> {
        s.rfind(search)
    }
    
    /// Check if string contains substring
    /// contains("hello world", "world") => true
    pub fn contains(s: &str, search: &str) -> bool {
        s.contains(search)
    }
    
    /// Check if string starts with substring
    /// starts_with("hello", "he") => true
    pub fn starts_with(s: &str, prefix: &str) -> bool {
        s.starts_with(prefix)
    }
    
    /// Check if string ends with substring
    /// ends_with("hello", "lo") => true
    pub fn ends_with(s: &str, suffix: &str) -> bool {
        s.ends_with(suffix)
    }
    
    /// Count occurrences of substring
    /// count("hello hello hello", "hello") => 3
    pub fn count(s: &str, search: &str) -> usize {
        if search.is_empty() {
            return 0;
        }
        s.matches(search).count()
    }
    
    // ==================== Splitting & Joining ====================
    
    /// Split string by delimiter
    /// split("a,b,c", ",") => ["a", "b", "c"]
    pub fn split(s: &str, delimiter: &str) -> Vec<String> {
        s.split(delimiter)
            .map(|s| s.to_string())
            .collect()
    }
    
    /// Split on whitespace
    /// split_whitespace("hello world test") => ["hello", "world", "test"]
    pub fn split_whitespace(s: &str) -> Vec<String> {
        s.split_whitespace()
            .map(|s| s.to_string())
            .collect()
    }
    
    /// Join array of strings with delimiter
    /// join(["a", "b", "c"], ",") => "a,b,c"
    pub fn join(parts: &[&str], delimiter: &str) -> String {
        parts.join(delimiter)
    }
    
    // ==================== Trimming & Padding ====================
    
    /// Trim whitespace from both ends
    /// trim("  hello  ") => "hello"
    pub fn trim(s: &str) -> String {
        s.trim().to_string()
    }
    
    /// Trim whitespace from start
    /// trim_start("  hello") => "hello"
    pub fn trim_start(s: &str) -> String {
        s.trim_start().to_string()
    }
    
    /// Trim whitespace from end
    /// trim_end("hello  ") => "hello"
    pub fn trim_end(s: &str) -> String {
        s.trim_end().to_string()
    }
    
    /// Trim specific character from both ends
    /// trim_char("##hello##", "#") => "hello"
    pub fn trim_char(s: &str, char_to_trim: &str) -> String {
        if char_to_trim.is_empty() {
            return s.to_string();
        }
        let ch = char_to_trim.chars().next().unwrap();
        s.trim_matches(ch).to_string()
    }
    
    /// Pad string to length with character on left
    /// pad_start("5", 3, "0") => "005"
    pub fn pad_start(s: &str, length: usize, pad_char: &str) -> String {
        if s.len() >= length || pad_char.is_empty() {
            return s.to_string();
        }
        let pad_str = pad_char.chars().next().unwrap().to_string();
        let padding = pad_str.repeat(length - s.len());
        format!("{}{}", padding, s)
    }
    
    /// Pad string to length with character on right
    /// pad_end("5", 3, "0") => "500"
    pub fn pad_end(s: &str, length: usize, pad_char: &str) -> String {
        if s.len() >= length || pad_char.is_empty() {
            return s.to_string();
        }
        let pad_str = pad_char.chars().next().unwrap().to_string();
        let padding = pad_str.repeat(length - s.len());
        format!("{}{}", s, padding)
    }
    
    // ==================== Replacing & Substitution ====================
    
    /// Replace first occurrence of search with replacement
    /// replace_first("hello hello", "hello", "hi") => "hi hello"
    pub fn replace_first(s: &str, search: &str, replacement: &str) -> String {
        if let Some(pos) = s.find(search) {
            let mut result = String::new();
            result.push_str(&s[..pos]);
            result.push_str(replacement);
            result.push_str(&s[pos + search.len()..]);
            result
        } else {
            s.to_string()
        }
    }
    
    /// Replace all occurrences of search with replacement
    /// replace_all("hello hello", "hello", "hi") => "hi hi"
    pub fn replace_all(s: &str, search: &str, replacement: &str) -> String {
        s.replace(search, replacement)
    }
    
    // ==================== Extraction & Slicing ====================
    
    /// Get substring starting at index with specified length
    /// substring("hello", 1, 3) => "ell"
    pub fn substring(s: &str, start: usize, length: usize) -> String {
        s.chars()
            .skip(start)
            .take(length)
            .collect()
    }
    
    /// Get substring from start index to end
    /// substring_from("hello", 1) => "ello"
    pub fn substring_from(s: &str, start: usize) -> String {
        s.chars()
            .skip(start)
            .collect()
    }
    
    /// Get substring up to index
    /// substring_to("hello", 3) => "hel"
    pub fn substring_to(s: &str, end: usize) -> String {
        s.chars()
            .take(end)
            .collect()
    }
    
    /// Get first N characters
    /// first("hello", 3) => "hel"
    pub fn first(s: &str, n: usize) -> String {
        s.chars().take(n).collect()
    }
    
    /// Get last N characters
    /// last("hello", 3) => "llo"
    pub fn last(s: &str, n: usize) -> String {
        let len = s.len();
        if n >= len {
            s.to_string()
        } else {
            s.chars().skip(len - n).collect()
        }
    }
    
    /// Reverse string
    /// reverse("hello") => "olleh"
    pub fn reverse(s: &str) -> String {
        s.chars().rev().collect()
    }
    
    // ==================== String Queries ====================
    
    /// Get string length in characters
    /// length("hello") => 5
    pub fn length(s: &str) -> usize {
        s.chars().count()
    }
    
    /// Get string length in bytes
    /// byte_length("hello") => 5
    /// byte_length("café") => 5 (é is 2 bytes)
    pub fn byte_length(s: &str) -> usize {
        s.len()
    }
    
    /// Check if string is empty
    /// is_empty("") => true
    pub fn is_empty(s: &str) -> bool {
        s.is_empty()
    }
    
    /// Check if string is all uppercase
    /// is_uppercase("HELLO") => true
    pub fn is_uppercase(s: &str) -> bool {
        !s.is_empty() && s.chars().all(|c| !c.is_lowercase())
    }
    
    /// Check if string is all lowercase
    /// is_lowercase("hello") => true
    pub fn is_lowercase(s: &str) -> bool {
        !s.is_empty() && s.chars().all(|c| !c.is_uppercase())
    }
    
    /// Check if string is all digits
    /// is_numeric("12345") => true
    pub fn is_numeric(s: &str) -> bool {
        !s.is_empty() && s.chars().all(|c| c.is_numeric())
    }
    
    /// Check if string is all alphabetic
    /// is_alpha("hello") => true
    pub fn is_alpha(s: &str) -> bool {
        !s.is_empty() && s.chars().all(|c| c.is_alphabetic())
    }
    
    /// Check if string is alphanumeric
    /// is_alphanumeric("hello123") => true
    pub fn is_alphanumeric(s: &str) -> bool {
        !s.is_empty() && s.chars().all(|c| c.is_alphanumeric())
    }
    
    // ==================== Repeating & Multiplying ====================
    
    /// Repeat string N times
    /// repeat("ab", 3) => "ababab"
    pub fn repeat(s: &str, times: usize) -> String {
        s.repeat(times)
    }
    
    // ==================== Conversion & Format ====================
    
    /// Convert number to string
    /// to_string(42) => "42"
    pub fn to_string(n: f64) -> String {
        if n.fract() == 0.0 {
            format!("{:.0}", n)
        } else {
            n.to_string()
        }
    }
    
    /// Convert value to string with custom formatting
    /// format_number(3.14159, 2) => "3.14"
    pub fn format_number(n: f64, decimal_places: u32) -> String {
        format!("{:.p$}", n, p = decimal_places as usize)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_case_conversion() {
        assert_eq!(StringModule::uppercase("hello"), "HELLO");
        assert_eq!(StringModule::lowercase("HELLO"), "hello");
        assert_eq!(StringModule::capitalize("hello world"), "Hello world");
        assert_eq!(StringModule::title_case("hello world"), "Hello World");
    }

    #[test]
    fn test_searching() {
        assert_eq!(StringModule::index_of("hello world", "world"), Some(6));
        assert_eq!(StringModule::last_index_of("hello hello", "hello"), Some(6));
        assert!(StringModule::contains("hello world", "world"));
        assert!(StringModule::starts_with("hello", "he"));
        assert!(StringModule::ends_with("hello", "lo"));
    }

    #[test]
    fn test_splitting_joining() {
        let split = StringModule::split("a,b,c", ",");
        assert_eq!(split, vec!["a", "b", "c"]);
        
        let joined = StringModule::join(&["a", "b", "c"], ",");
        assert_eq!(joined, "a,b,c");
    }

    #[test]
    fn test_trimming_padding() {
        assert_eq!(StringModule::trim("  hello  "), "hello");
        assert_eq!(StringModule::pad_start("5", 3, "0"), "005");
        assert_eq!(StringModule::pad_end("5", 3, "0"), "500");
    }

    #[test]
    fn test_replacing() {
        assert_eq!(StringModule::replace_all("hello hello", "hello", "hi"), "hi hi");
        assert_eq!(StringModule::replace_first("hello hello", "hello", "hi"), "hi hello");
    }

    #[test]
    fn test_extraction() {
        assert_eq!(StringModule::substring("hello", 1, 3), "ell");
        assert_eq!(StringModule::reverse("hello"), "olleh");
    }

    #[test]
    fn test_queries() {
        assert_eq!(StringModule::length("hello"), 5);
        assert!(StringModule::is_numeric("12345"));
        assert!(StringModule::is_alpha("hello"));
    }
}
