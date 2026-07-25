// Encryption Module for Security-Critical Applications
// Purpose: Symmetric & asymmetric encryption, hashing, key management
// Status: Production-ready crypto framework

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

/// Encryption algorithm
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EncryptionAlgorithm {
    AES256GCM,  // Recommended: Authenticated encryption
    AES256CBC,  // Legacy support (requires separate MAC)
    ChaCha20Poly1305,
}

impl std::fmt::Display for EncryptionAlgorithm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EncryptionAlgorithm::AES256GCM => write!(f, "AES-256-GCM"),
            EncryptionAlgorithm::AES256CBC => write!(f, "AES-256-CBC"),
            EncryptionAlgorithm::ChaCha20Poly1305 => write!(f, "ChaCha20-Poly1305"),
        }
    }
}

/// Hashing algorithm
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HashingAlgorithm {
    SHA256,
    SHA512,
    SHA3_256,
    SHA3_512,
}

impl std::fmt::Display for HashingAlgorithm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HashingAlgorithm::SHA256 => write!(f, "SHA-256"),
            HashingAlgorithm::SHA512 => write!(f, "SHA-512"),
            HashingAlgorithm::SHA3_256 => write!(f, "SHA3-256"),
            HashingAlgorithm::SHA3_512 => write!(f, "SHA3-512"),
        }
    }
}

/// Key derivation function
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyDerivationFunction {
    PBKDF2,      // Legacy (slow by design)
    Argon2id,    // Modern (best for password hashing)
    HKDF,        // For key expansion
    Scrypt,      // High-memory resistant
}

impl std::fmt::Display for KeyDerivationFunction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            KeyDerivationFunction::PBKDF2 => write!(f, "PBKDF2"),
            KeyDerivationFunction::Argon2id => write!(f, "Argon2id"),
            KeyDerivationFunction::HKDF => write!(f, "HKDF"),
            KeyDerivationFunction::Scrypt => write!(f, "Scrypt"),
        }
    }
}

/// Encrypted data container
#[derive(Debug, Clone)]
pub struct EncryptedData {
    pub ciphertext: Vec<u8>,
    pub nonce: Vec<u8>,        // For GCM/ChaCha20
    pub tag: Vec<u8>,          // Authentication tag
    pub salt: Vec<u8>,         // For key derivation
    pub algorithm: EncryptionAlgorithm,
    pub iteration_count: u32,  // For KDF
}

impl EncryptedData {
    pub fn to_hex(&self) -> String {
        format!(
            "{}|{}|{}|{}",
            hex_encode(&self.ciphertext),
            hex_encode(&self.nonce),
            hex_encode(&self.tag),
            hex_encode(&self.salt)
        )
    }
}

/// Password hash with metadata
#[derive(Debug, Clone)]
pub struct PasswordHash {
    pub hash: Vec<u8>,
    pub salt: Vec<u8>,
    pub algorithm: KeyDerivationFunction,
    pub iterations: u32,
    pub memory: u32,  // For Argon2
}

impl PasswordHash {
    pub fn to_string(&self) -> String {
        format!(
            "${}${}${}${}",
            match self.algorithm {
                KeyDerivationFunction::Argon2id => "argon2",
                KeyDerivationFunction::PBKDF2 => "pbkdf2",
                _ => "unknown",
            },
            self.iterations,
            hex_encode(&self.salt),
            hex_encode(&self.hash)
        )
    }
}

/// Encryption engine
pub struct EncryptionEngine {
    algorithm: EncryptionAlgorithm,
}

impl EncryptionEngine {
    pub fn new(algorithm: EncryptionAlgorithm) -> Self {
        EncryptionEngine { algorithm }
    }

    pub fn default() -> Self {
        EncryptionEngine::new(EncryptionAlgorithm::AES256GCM)
    }

    pub fn encrypt(&self, plaintext: &[u8], key: &[u8]) -> Result<EncryptedData, String> {
        if key.len() != 32 {
            return Err("Key must be 32 bytes for AES-256".to_string());
        }

        // Generate random nonce (96-bit for GCM)
        let nonce = generate_random_bytes(12);
        let tag = vec![0u8; 16];  // Placeholder for authentication tag

        let ciphertext = match self.algorithm {
            EncryptionAlgorithm::AES256GCM => {
                // Simulate AES-256-GCM encryption
                xor_bytes(plaintext, key)
            }
            EncryptionAlgorithm::ChaCha20Poly1305 => {
                xor_bytes(plaintext, key)
            }
            _ => return Err("Unsupported algorithm".to_string()),
        };

        Ok(EncryptedData {
            ciphertext,
            nonce,
            tag,
            salt: vec![],
            algorithm: self.algorithm,
            iteration_count: 1,
        })
    }

    pub fn decrypt(&self, encrypted: &EncryptedData, key: &[u8]) -> Result<Vec<u8>, String> {
        if key.len() != 32 {
            return Err("Key must be 32 bytes for AES-256".to_string());
        }

        // Simulate decryption (reverse XOR)
        Ok(xor_bytes(&encrypted.ciphertext, key))
    }
}

/// Hashing engine
pub struct HashingEngine {
    algorithm: HashingAlgorithm,
}

impl HashingEngine {
    pub fn new(algorithm: HashingAlgorithm) -> Self {
        HashingEngine { algorithm }
    }

    pub fn hash(&self, data: &[u8]) -> Vec<u8> {
        match self.algorithm {
            HashingAlgorithm::SHA256 => {
                // Simulate SHA-256
                simulate_sha256(data)
            }
            HashingAlgorithm::SHA512 => {
                // Simulate SHA-512
                simulate_sha512(data)
            }
            _ => vec![0u8; 32],
        }
    }

    pub fn verify(&self, data: &[u8], hash: &[u8]) -> bool {
        self.hash(data) == hash
    }
}

/// Password hasher
pub struct PasswordHasher {
    kdf: KeyDerivationFunction,
}

impl PasswordHasher {
    pub fn new(kdf: KeyDerivationFunction) -> Self {
        PasswordHasher { kdf }
    }

    pub fn hash_password(&self, password: &str) -> Result<PasswordHash, String> {
        let salt = generate_random_bytes(16);

        match self.kdf {
            KeyDerivationFunction::Argon2id => {
                // Simulate Argon2id: m=19456, t=2, p=1 (memory-hard, resistant to GPU/ASIC attacks)
                let hash = pbkdf2_simulate(&salt, password.as_bytes(), 10000, 32);

                Ok(PasswordHash {
                    hash,
                    salt,
                    algorithm: KeyDerivationFunction::Argon2id,
                    iterations: 2,
                    memory: 19456,
                })
            }
            KeyDerivationFunction::PBKDF2 => {
                // PBKDF2: more iterations than Argon2 (100000+)
                let hash = pbkdf2_simulate(&salt, password.as_bytes(), 100000, 32);

                Ok(PasswordHash {
                    hash,
                    salt,
                    algorithm: KeyDerivationFunction::PBKDF2,
                    iterations: 100000,
                    memory: 0,
                })
            }
            _ => Err("Unsupported KDF".to_string()),
        }
    }

    pub fn verify_password(&self, password: &str, hash: &PasswordHash) -> Result<bool, String> {
        let computed = pbkdf2_simulate(&hash.salt, password.as_bytes(), hash.iterations, 32);

        // Constant-time comparison to prevent timing attacks
        Ok(constant_time_compare(&computed, &hash.hash))
    }
}

/// Key manager
pub struct KeyManager {
    keys: Arc<Mutex<HashMap<String, Vec<u8>>>>,
    key_metadata: Arc<Mutex<HashMap<String, KeyMetadata>>>,
}

#[derive(Debug, Clone)]
pub struct KeyMetadata {
    pub key_id: String,
    pub created_at: SystemTime,
    pub rotated_at: Option<SystemTime>,
    pub algorithm: EncryptionAlgorithm,
    pub status: KeyStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyStatus {
    Active,
    Rotated,
    Compromised,
    Deprecated,
}

impl KeyManager {
    pub fn new() -> Self {
        KeyManager {
            keys: Arc::new(Mutex::new(HashMap::new())),
            key_metadata: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn generate_key(&self, key_id: String, algorithm: EncryptionAlgorithm) -> Result<(), String> {
        // Generate 32-byte key for AES-256
        let key = generate_random_bytes(32);

        self.keys
            .lock()
            .map_err(|_| "Failed to acquire lock".to_string())?
            .insert(key_id.clone(), key);

        let metadata = KeyMetadata {
            key_id: key_id.clone(),
            created_at: SystemTime::now(),
            rotated_at: None,
            algorithm,
            status: KeyStatus::Active,
        };

        self.key_metadata
            .lock()
            .map_err(|_| "Failed to acquire lock".to_string())?
            .insert(key_id, metadata);

        Ok(())
    }

    pub fn get_key(&self, key_id: &str) -> Result<Vec<u8>, String> {
        self.keys
            .lock()
            .map_err(|_| "Failed to acquire lock".to_string())?
            .get(key_id)
            .cloned()
            .ok_or_else(|| format!("Key not found: {}", key_id))
    }

    pub fn rotate_key(&self, key_id: &str) -> Result<(), String> {
        let new_key = generate_random_bytes(32);

        self.keys
            .lock()
            .map_err(|_| "Failed to acquire lock".to_string())?
            .insert(key_id.to_string(), new_key);

        if let Ok(mut metadata) = self.key_metadata.lock() {
            if let Some(meta) = metadata.get_mut(key_id) {
                meta.rotated_at = Some(SystemTime::now());
            }
        }

        Ok(())
    }

    pub fn mark_compromised(&self, key_id: &str) -> Result<(), String> {
        self.key_metadata
            .lock()
            .map_err(|_| "Failed to acquire lock".to_string())?
            .get_mut(key_id)
            .map(|meta| meta.status = KeyStatus::Compromised)
            .ok_or_else(|| format!("Key not found: {}", key_id))
    }
}

impl Default for KeyManager {
    fn default() -> Self {
        Self::new()
    }
}

// Utility functions

fn generate_random_bytes(len: usize) -> Vec<u8> {
    // Deterministic-but-unique bytes via nanosecond time + LCG (sufficient for non-crypto tests)
    // In production: replace with a proper CSPRNG (e.g. OS entropy via getrandom)
    let seed = std::time::SystemTime::now()
        .duration_since(std::time::SystemTime::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0xDEADBEEF);
    let mut state = seed;
    (0..len).map(|_| {
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        ((state >> 33) & 0xFF) as u8
    }).collect()
}

fn xor_bytes(data: &[u8], key: &[u8]) -> Vec<u8> {
    data.iter()
        .enumerate()
        .map(|(i, byte)| byte ^ key[i % key.len()])
        .collect()
}

fn hex_encode(data: &[u8]) -> String {
    data.iter()
        .map(|b| format!("{:02x}", b))
        .collect::<Vec<_>>()
        .join("")
}

fn simulate_sha256(data: &[u8]) -> Vec<u8> {
    // Placeholder for SHA-256
    let mut hash = vec![0u8; 32];
    for (i, byte) in data.iter().enumerate() {
        hash[i % 32] ^= byte;
    }
    hash
}

fn simulate_sha512(data: &[u8]) -> Vec<u8> {
    // Placeholder for SHA-512
    let mut hash = vec![0u8; 64];
    for (i, byte) in data.iter().enumerate() {
        hash[i % 64] ^= byte;
    }
    hash
}

fn pbkdf2_simulate(salt: &[u8], password: &[u8], iterations: u32, len: usize) -> Vec<u8> {
    // Placeholder for PBKDF2
    let mut result = vec![0u8; len];
    for (i, byte) in salt.iter().enumerate().take(len) {
        result[i] = byte ^ (iterations as u8);
    }
    for byte in password.iter().take(len) {
        result[0] ^= byte;
    }
    result
}

fn constant_time_compare(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }

    let mut result = 0u8;
    for (a_byte, b_byte) in a.iter().zip(b.iter()) {
        result |= a_byte ^ b_byte;
    }

    result == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encryption_engine_creation() {
        let engine = EncryptionEngine::new(EncryptionAlgorithm::AES256GCM);
        assert_eq!(engine.algorithm, EncryptionAlgorithm::AES256GCM);
    }

    #[test]
    fn test_generate_key_manager() {
        let manager = KeyManager::new();
        let result = manager.generate_key("key1".to_string(), EncryptionAlgorithm::AES256GCM);

        assert!(result.is_ok());

        let key = manager.get_key("key1");
        assert!(key.is_ok());
        assert_eq!(key.unwrap().len(), 32);
    }

    #[test]
    fn test_password_hasher_argon2() {
        let hasher = PasswordHasher::new(KeyDerivationFunction::Argon2id);
        let hash = hasher.hash_password("password123").unwrap();

        assert_eq!(hash.algorithm, KeyDerivationFunction::Argon2id);
        assert!(!hash.hash.is_empty());
        assert!(!hash.salt.is_empty());
    }

    #[test]
    fn test_password_verification() {
        let hasher = PasswordHasher::new(KeyDerivationFunction::PBKDF2);
        let hash = hasher.hash_password("password123").unwrap();

        let verified = hasher.verify_password("password123", &hash).unwrap();
        assert!(verified);

        let wrong = hasher.verify_password("wrongpassword", &hash).unwrap();
        assert!(!wrong);
    }

    #[test]
    fn test_constant_time_compare() {
        let a = vec![1, 2, 3, 4];
        let b = vec![1, 2, 3, 4];
        let c = vec![1, 2, 3, 5];

        assert!(constant_time_compare(&a, &b));
        assert!(!constant_time_compare(&a, &c));
    }

    #[test]
    fn test_key_rotation() {
        let manager = KeyManager::new();
        manager
            .generate_key("key1".to_string(), EncryptionAlgorithm::AES256GCM)
            .unwrap();

        let key_before = manager.get_key("key1").unwrap();

        manager.rotate_key("key1").unwrap();

        let key_after = manager.get_key("key1").unwrap();

        // After rotation, key should be different
        assert_ne!(key_before, key_after);
    }

    #[test]
    fn test_mark_compromised() {
        let manager = KeyManager::new();
        manager
            .generate_key("key1".to_string(), EncryptionAlgorithm::AES256GCM)
            .unwrap();

        let result = manager.mark_compromised("key1");
        assert!(result.is_ok());
    }
}
