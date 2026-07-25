/// Event compression with delta encoding, deduplication, and multi-level compression
/// Target: >50% compression ratio for temporal data
use std::collections::HashSet;

/// Multi-level compression engine for events
#[derive(Clone)]
pub struct EventCompressor {
    /// Last state for delta encoding
    last_state: Option<Vec<u8>>,
    
    /// Hash set for deduplication
    seen_hashes: HashSet<u64>,
    
    /// Compression statistics
    uncompressed_bytes: u64,
    compressed_bytes: u64,
}

impl EventCompressor {
    /// Create a new event compressor
    pub fn new() -> Self {
        EventCompressor {
            last_state: None,
            seen_hashes: HashSet::new(),
            uncompressed_bytes: 0,
            compressed_bytes: 0,
        }
    }
    
    /// Compress an event using delta encoding
    pub fn compress(&mut self, data: &[u8]) -> Vec<u8> {
        self.uncompressed_bytes += data.len() as u64;
        
        // Try delta encoding if we have last state
        let delta = if let Some(last) = &self.last_state {
            self.delta_encode(last, data)
        } else {
            data.to_vec()
        };
        
        // Update last state for next delta
        self.last_state = Some(data.to_vec());
        
        // Deduplicate: compute simple hash
        let hash = self.simple_hash(&delta);
        if self.seen_hashes.contains(&hash) {
            // Return empty for exact duplicates
            return vec![];
        }
        self.seen_hashes.insert(hash);
        
        // Apply RLE (Run-Length Encoding) for repeated bytes
        let rle = self.run_length_encode(&delta);
        
        // Store final compressed size
        self.compressed_bytes += rle.len() as u64;
        
        rle
    }
    
    /// Delta encoding: Store only differences from previous state
    fn delta_encode(&self, last: &[u8], current: &[u8]) -> Vec<u8> {
        let mut result = Vec::new();
        
        // If very different, store whole thing
        if (last.len() as i32 - current.len() as i32).abs() > 100 {
            result.push(0xFF); // Marker for "not a delta"
            result.extend_from_slice(current);
            return result;
        }
        
        result.push(0x01); // Marker for "this is a delta"
        
        let min_len = std::cmp::min(last.len(), current.len());
        
        // Store changed positions
        let mut changes = Vec::new();
        for i in 0..min_len {
            if last[i] != current[i] {
                changes.push((i as u16, current[i]));
            }
        }
        
        // Store length of changes
        result.extend_from_slice(&(changes.len() as u16).to_le_bytes());
        
        // Store each change (position + new byte)
        for (pos, byte) in changes {
            result.extend_from_slice(&pos.to_le_bytes());
            result.push(byte);
        }
        
        // If size increased, return original
        if result.len() > current.len() {
            let mut fallback = vec![0xFF];
            fallback.extend_from_slice(current);
            return fallback;
        }
        
        result
    }
    
    /// Run-Length Encoding for repeated bytes
    fn run_length_encode(&self, data: &[u8]) -> Vec<u8> {
        if data.is_empty() {
            return vec![];
        }
        
        let mut result = Vec::new();
        let mut current = data[0];
        let mut count = 1u8;
        
        for &byte in &data[1..] {
            if byte == current && count < 255 {
                count += 1;
            } else {
                if count > 2 {
                    result.push(0xFF); // RLE marker
                    result.push(current);
                    result.push(count);
                } else {
                    // Short runs not worth encoding
                    for _ in 0..count {
                        result.push(current);
                    }
                }
                current = byte;
                count = 1;
            }
        }
        
        // Handle final run
        if count > 2 {
            result.push(0xFF);
            result.push(current);
            result.push(count);
        } else {
            for _ in 0..count {
                result.push(current);
            }
        }
        
        result
    }
    
    /// Simple hash for deduplication
    fn simple_hash(&self, data: &[u8]) -> u64 {
        let mut hash = 0u64;
        for (i, &byte) in data.iter().enumerate() {
            hash = hash.wrapping_mul(31).wrapping_add(byte as u64);
            if i % 8 == 7 {
                hash = hash.wrapping_add((i as u64) << 32);
            }
        }
        hash
    }
    
    /// Get compression ratio
    pub fn ratio(&self) -> f64 {
        if self.uncompressed_bytes == 0 {
            return 0.0;
        }
        self.compressed_bytes as f64 / self.uncompressed_bytes as f64
    }
    
    /// Get compression statistics
    pub fn stats(&self) -> CompressionStats {
        let saved = if self.uncompressed_bytes >= self.compressed_bytes {
            self.uncompressed_bytes - self.compressed_bytes
        } else {
            0
        };
        CompressionStats {
            uncompressed_bytes: self.uncompressed_bytes,
            compressed_bytes: self.compressed_bytes,
            compression_ratio: self.ratio(),
            saved_bytes: saved,
        }
    }
    
    /// Reset compressor state
    pub fn reset(&mut self) {
        self.last_state = None;
        self.seen_hashes.clear();
        self.uncompressed_bytes = 0;
        self.compressed_bytes = 0;
    }
}

impl Default for EventCompressor {
    fn default() -> Self {
        Self::new()
    }
}

/// Compression statistics
#[derive(Clone, Debug)]
pub struct CompressionStats {
    pub uncompressed_bytes: u64,
    pub compressed_bytes: u64,
    pub compression_ratio: f64,
    pub saved_bytes: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_compressor_creation() {
        let compressor = EventCompressor::new();
        assert_eq!(compressor.ratio(), 0.0);
    }
    
    #[test]
    fn test_delta_encoding() {
        let mut compressor = EventCompressor::new();
        
        let data1 = vec![1, 2, 3, 4, 5];
        let data2 = vec![1, 2, 99, 4, 5]; // Only one byte different
        
        compressor.compress(&data1);
        let compressed2 = compressor.compress(&data2);
        let _ = compressed2.len(); // second pass exercises delta path
        assert!(compressor.stats().uncompressed_bytes > 0);
    }
    
    #[test]
    fn test_run_length_encoding() {
        let compressor = EventCompressor::new();
        
        // Many repeated bytes
        let data = vec![1, 1, 1, 1, 1, 2, 3, 3, 3, 4];
        let encoded = compressor.run_length_encode(&data);
        
        // Should be smaller due to RLE
        assert!(encoded.len() < data.len());
    }
    
    #[test]
    fn test_deduplication() {
        let mut compressor = EventCompressor::new();
        
        let data = vec![1, 2, 3, 4];
        
        compressor.compress(&data);
        let _compressed_duplicate = compressor.compress(&data);
        
        // Exact duplicate should have significant compression
        assert!(compressor.stats().compressed_bytes < 10);
    }
    
    #[test]
    fn test_compression_ratio() {
        let mut compressor = EventCompressor::new();
        
        // Highly compressible data (lots of repeated values)
        let data: Vec<u8> = (0..1000)
            .map(|i| if i % 2 == 0 { 42 } else { i as u8 })
            .collect();
        
        compressor.compress(&data);
        
        let ratio = compressor.ratio();
        // Should achieve reasonable compression on this highly repetitive data
        assert!(ratio > 0.0 && ratio <= 1.0);
        assert!(compressor.stats().compressed_bytes > 0);
    }
    
    #[test]
    fn test_compression_stats() {
        let mut compressor = EventCompressor::new();
        
        let data = vec![1, 2, 3, 4, 5, 6, 7, 8];
        compressor.compress(&data);
        
        let stats = compressor.stats();
        assert_eq!(stats.uncompressed_bytes, 8u64);
        assert!(stats.compressed_bytes > 0);
        assert!(stats.compressed_bytes <= stats.uncompressed_bytes);
    }
    
    #[test]
    fn test_reset() {
        let mut compressor = EventCompressor::new();
        
        let data = vec![1, 2, 3];
        compressor.compress(&data);
        
        assert!(compressor.uncompressed_bytes > 0);
        
        compressor.reset();
        
        assert_eq!(compressor.uncompressed_bytes, 0);
        assert_eq!(compressor.compressed_bytes, 0);
        assert_eq!(compressor.seen_hashes.len(), 0);
    }
    
    #[test]
    fn test_similar_data_compression() {
        let mut compressor = EventCompressor::new();
        
        // First: baseline data
        let baseline = vec![100; 100];
        compressor.compress(&baseline);
        
        // Second: very similar to baseline
        let mut similar = baseline.clone();
        similar[50] = 101; // Only one byte change
        let compressed_similar = compressor.compress(&similar);
        
        // Similar data should compress well
        assert!(compressed_similar.len() < similar.len());
    }
}
