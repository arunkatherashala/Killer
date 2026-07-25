// killer_rcore/src/jit/signature.rs
// Loop signature and hashing for cache management
// Week 3 part 1

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// Minimal loop profile for benchmarking (needed for signature generation)
/// In production, this would be crate::optimizer::LoopProfile
#[derive(Clone, Debug, Hash)]
pub struct LoopProfile {
    pub loop_id: String,
    pub estimated_iterations: i64,
    pub loop_var: String,
    pub source_line: usize,
}

/// Unique signature for a loop
/// Used as cache key to avoid recompilation
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct LoopSignature {
    /// Hex hash of loop code
    pub hash: String,
    
    /// Original loop ID
    pub loop_id: String,
    
    /// Hash of source code (for invalidation if code changes)
    pub source_hash: String,
    
    /// File and line where loop appears
    pub location: String,
}

impl LoopSignature {
    /// Create signature from loop profile
    pub fn from_profile(profile: &LoopProfile) -> Self {
        // Hash the loop profile
        let mut hasher = DefaultHasher::new();
        profile.loop_id.hash(&mut hasher);
        profile.estimated_iterations.hash(&mut hasher);
        profile.loop_var.hash(&mut hasher);
        
        let hash = format!("{:016x}", hasher.finish());
        
        LoopSignature {
            hash: hash.clone(),
            loop_id: profile.loop_id.clone(),
            source_hash: hash, // Simplified: same as main hash
            location: format!("line_{}", profile.source_line),
        }
    }
    
    /// Create signature from loop ID and bounds
    pub fn from_id_and_bounds(loop_id: &str, iterations: i64) -> Self {
        let mut hasher = DefaultHasher::new();
        loop_id.hash(&mut hasher);
        iterations.hash(&mut hasher);
        
        let hash = format!("{:016x}", hasher.finish());
        
        LoopSignature {
            hash: hash.clone(),
            loop_id: loop_id.to_string(),
            source_hash: hash,
            location: "unknown".to_string(),
        }
    }
    
    /// Get cache filename for this signature
    pub fn cache_filename(&self) -> String {
        format!("killer_jit_{}.so", self.hash)
    }
    
    /// Get cache filename with extension
    pub fn cache_filename_with_ext(&self, ext: &str) -> String {
        format!("killer_jit_{}.{}", self.hash, ext)
    }
    
    /// Check if hash is valid (non-empty, valid hex)
    pub fn is_valid(&self) -> bool {
        !self.hash.is_empty() && self.hash.len() == 16
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    fn create_test_profile() -> LoopProfile {
        LoopProfile {
            loop_id: "test_loop".to_string(),
            estimated_iterations: 1_000_000,
            loop_var: "i".to_string(),
            source_line: 42,
        }
    }
    
    #[test]
    fn test_signature_from_profile() {
        let profile = create_test_profile();
        let sig = LoopSignature::from_profile(&profile);
        
        assert!(!sig.hash.is_empty());
        assert_eq!(sig.loop_id, "test_loop");
        assert!(sig.is_valid());
    }
    
    #[test]
    fn test_signature_deterministic() {
        let profile1 = create_test_profile();
        let profile2 = create_test_profile();
        
        let sig1 = LoopSignature::from_profile(&profile1);
        let sig2 = LoopSignature::from_profile(&profile2);
        
        // Same profile should produce same hash
        assert_eq!(sig1.hash, sig2.hash);
    }
    
    #[test]
    fn test_signature_different_for_different_loops() {
        let profile1 = create_test_profile();
        let mut profile2 = create_test_profile();
        
        profile2.estimated_iterations = 2_000_000; // Different
        
        let sig1 = LoopSignature::from_profile(&profile1);
        let sig2 = LoopSignature::from_profile(&profile2);
        
        assert_ne!(sig1.hash, sig2.hash);
    }
    
    #[test]
    fn test_cache_filename_generation() {
        let profile = create_test_profile();
        let sig = LoopSignature::from_profile(&profile);
        
        let filename = sig.cache_filename();
        assert!(filename.starts_with("killer_jit_"));
        assert!(filename.ends_with(".so"));
    }
    
    #[test]
    fn test_cache_filename_with_extension() {
        let profile = create_test_profile();
        let sig = LoopSignature::from_profile(&profile);
        
        let win_filename = sig.cache_filename_with_ext("dll");
        assert!(win_filename.ends_with(".dll"));
        
        let unix_filename = sig.cache_filename_with_ext("so");
        assert!(unix_filename.ends_with(".so"));
    }
    
    #[test]
    fn test_signature_validation() {
        let valid_sig = LoopSignature::from_profile(&create_test_profile());
        assert!(valid_sig.is_valid());
        
        let invalid_sig = LoopSignature {
            hash: String::new(),
            loop_id: "test".to_string(),
            source_hash: String::new(),
            location: "line_1".to_string(),
        };
        assert!(!invalid_sig.is_valid());
    }
    
    #[test]
    fn test_signature_from_id_and_bounds() {
        let sig = LoopSignature::from_id_and_bounds("my_loop", 500_000);
        
        assert!(!sig.hash.is_empty());
        assert_eq!(sig.loop_id, "my_loop");
        assert!(sig.is_valid());
    }
}
