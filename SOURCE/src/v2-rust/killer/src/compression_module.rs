// Compression Module for Killer Language
// Basic compression utilities and data compression operations
// Version: 2.1.0

/// Simple run-length encoding compression
pub struct CompressionModule;

impl CompressionModule {
    // ==================== Run-Length Encoding ====================
    
    /// Compress string simple RLE: "aaabbb" => "a3b3"
    /// Simple format without regex dependencies
    pub fn rle_encode(text: &str) -> String {
        if text.is_empty() {
            return String::new();
        }
        
        let chars: Vec<char> = text.chars().collect();
        let mut result = String::new();
        let mut count = 1;
        let mut current = chars[0];
        
        for i in 1..chars.len() {
            if chars[i] == current {
                count += 1;
            } else {
                result.push(current);
                if count > 1 {
                    result.push_str(&count.to_string());
                }
                current = chars[i];
                count = 1;
            }
        }
        
        // Flush last group
        result.push(current);
        if count > 1 {
            result.push_str(&count.to_string());
        }
        
        result
    }
    
    /// Decompress RLE: "a3b3" => "aaabbb"
    pub fn rle_decode(encoded: &str) -> String {
        let chars: Vec<char> = encoded.chars().collect();
        let mut result = String::new();
        let mut i = 0;
        
        while i < chars.len() {
            if chars[i].is_alphabetic() || chars[i] == ' ' {
                let ch = chars[i];
                let mut count = 1;
                
                // Check if next character(s) are digit
                let mut j = i + 1;
                while j < chars.len() && chars[j].is_numeric() {
                    j += 1;
                }
                
                if j > i + 1 {
                    let count_str: String = chars[(i + 1)..j].iter().collect();
                    count = count_str.parse().unwrap_or(1);
                    i = j;
                } else {
                    i += 1;
                }
                
                for _ in 0..count {
                    result.push(ch);
                }
            } else {
                i += 1;
            }
        }
        
        result
    }
    
    // ==================== Compression Ratio ====================
    
    /// Calculate compression ratio: original / compressed
    /// compression_ratio("aaabbb", compressed) => ratio
    pub fn compression_ratio(original: &str, compressed: &str) -> f64 {
        if compressed.is_empty() {
            return f64::INFINITY;
        }
        original.len() as f64 / compressed.len() as f64
    }
    
    /// Check if data is worth compressing
    pub fn should_compress(original: &str) -> bool {
        let compressed = Self::rle_encode(original);
        Self::compression_ratio(original, &compressed) > 1.1
    }
    
    // ==================== Simple Codec ====================
    
    /// Encode text as base64 (simplified)
    /// Uses basic ASCII mapping without padding
    pub fn base64_encode(data: &str) -> String {
        const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        
        let bytes = data.as_bytes();
        let mut result = String::new();
        
        for chunk in bytes.chunks(3) {
            let b1 = chunk[0];
            let b2 = chunk.get(1).copied().unwrap_or(0);
            let b3 = chunk.get(2).copied().unwrap_or(0);
            
            let n = ((b1 as u32) << 16) | ((b2 as u32) << 8) | (b3 as u32);
            
            result.push(ALPHABET[((n >> 18) & 0x3f) as usize] as char);
            result.push(ALPHABET[((n >> 12) & 0x3f) as usize] as char);
            
            if chunk.len() > 1 {
                result.push(ALPHABET[((n >> 6) & 0x3f) as usize] as char);
            }
            if chunk.len() > 2 {
                result.push(ALPHABET[(n & 0x3f) as usize] as char);
            }
        }
        
        result
    }
    
    /// Decode base64 string
    pub fn base64_decode(encoded: &str) -> Option<String> {
        const ALPHABET: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        
        let chars: Vec<char> = encoded.chars().collect();
        let mut result = Vec::new();
        
        for chunk in chars.chunks(4) {
            let mut indices = Vec::new();
            
            for &ch in chunk {
                if let Some(idx) = ALPHABET.find(ch) {
                    indices.push(idx as u32);
                } else {
                    return None;
                }
            }
            
            if indices.is_empty() {
                continue;
            }
            
            let n = (indices[0] << 18) 
                | (indices.get(1).unwrap_or(&0) << 12)
                | (indices.get(2).unwrap_or(&0) << 6)
                | indices.get(3).unwrap_or(&0);
            
            result.push(((n >> 16) & 0xff) as u8);
            
            if chunk.len() > 2 {
                result.push(((n >> 8) & 0xff) as u8);
            }
            if chunk.len() > 3 {
                result.push((n & 0xff) as u8);
            }
        }
        
        String::from_utf8(result).ok()
    }
    
    // ==================== Hex Encoding ====================
    
    /// Encode as hexadecimal
    /// hex_encode("ABC") => "414243"
    pub fn hex_encode(text: &str) -> String {
        text.as_bytes()
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect()
    }
    
    /// Decode from hexadecimal
    /// hex_decode("414243") => Some("ABC")
    pub fn hex_decode(hex: &str) -> Option<String> {
        if hex.len() % 2 != 0 {
            return None;
        }
        
        let mut result = Vec::new();
        
        for i in (0..hex.len()).step_by(2) {
            let byte_str = &hex[i..i + 2];
            let byte = u8::from_str_radix(byte_str, 16).ok()?;
            result.push(byte);
        }
        
        String::from_utf8(result).ok()
    }
    
    // ==================== Utilities ====================
    
    /// Get size in bytes
    pub fn size(text: &str) -> usize {
        text.len()
    }
    
    /// Get size in kilobytes
    pub fn size_kb(text: &str) -> f64 {
        text.len() as f64 / 1024.0
    }
    
    /// Get size in megabytes
    pub fn size_mb(text: &str) -> f64 {
        text.len() as f64 / (1024.0 * 1024.0)
    }
    
    /// Compare compression methods and return best
    pub fn best_compression(text: &str) -> (String, f64) {
        let rle = Self::rle_encode(text);
        let rle_ratio = Self::compression_ratio(text, &rle);
        
        let b64 = Self::base64_encode(text);
        let b64_ratio = Self::compression_ratio(text, &b64);
        
        if rle_ratio > b64_ratio {
            (String::from("rle"), rle_ratio)
        } else {
            (String::from("base64"), b64_ratio)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_rle_encode() {
        assert_eq!(CompressionModule::rle_encode("aaabbb"), "a3b3");
        assert_eq!(CompressionModule::rle_encode("abc"), "abc");
    }
    
    #[test]
    fn test_rle_decode() {
        assert_eq!(CompressionModule::rle_decode("a3b3"), "aaabbb");
        assert_eq!(CompressionModule::rle_decode("abc"), "abc");
    }
    
    #[test]
    fn test_rle_roundtrip() {
        let original = "aaabbbcccc";
        let encoded = CompressionModule::rle_encode(original);
        let decoded = CompressionModule::rle_decode(&encoded);
        assert_eq!(original, decoded);
    }
    
    #[test]
    fn test_compression_ratio() {
        let ratio = CompressionModule::compression_ratio("aaabbb", "a3b3");
        assert!(ratio > 1.0);
    }
    
    #[test]
    fn test_base64_encode() {
        let encoded = CompressionModule::base64_encode("ABC");
        assert!(!encoded.is_empty());
    }
    
    #[test]
    fn test_base64_roundtrip() {
        let original = "Hello, World!";
        let encoded = CompressionModule::base64_encode(original);
        let decoded = CompressionModule::base64_decode(&encoded).unwrap();
        assert_eq!(original, decoded);
    }
    
    #[test]
    fn test_hex_encode() {
        assert_eq!(CompressionModule::hex_encode("ABC"), "414243");
    }
    
    #[test]
    fn test_hex_decode() {
        assert_eq!(CompressionModule::hex_decode("414243"), Some("ABC".to_string()));
    }
    
    #[test]
    fn test_size_functions() {
        assert_eq!(CompressionModule::size("hello"), 5);
        assert!(CompressionModule::size_kb("hello") > 0.0);
    }
}
