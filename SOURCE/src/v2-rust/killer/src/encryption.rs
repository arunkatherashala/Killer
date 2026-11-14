// Encryption module: authenticated encryption, hashing, password hashing and key management.
//
// Everything here is real and checked against published test vectors (tests/crypto_vectors.rs):
//   * encryption: ChaCha20-Poly1305 (RFC 8439) through `crate::aead`, with a fresh random 96-bit nonce per message
//   * hashing: SHA-256 (FIPS 180-4) and SHA-512 (FIPS 180-4)
//   * password hashing: PBKDF2-HMAC-SHA256 (RFC 8018) with a random salt
//   * randomness: the operating system generator through `crate::os_random`, never a time-seeded one
//
// What is NOT implemented, and says so with an error instead of pretending: AES-256-GCM, AES-256-CBC, SHA-3,
// Argon2id, HKDF and scrypt. This code has not been audited and makes no side-channel guarantees beyond a
// constant-time tag and password-hash comparison.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

/// Encryption algorithm. Only `ChaCha20Poly1305` is implemented; the AES variants return an error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EncryptionAlgorithm {
    AES256GCM,  // not implemented
    AES256CBC,  // not implemented
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

/// Hashing algorithm. SHA-256 and SHA-512 are implemented; the SHA-3 variants return an error.
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

/// Key derivation function. Only `PBKDF2` is implemented.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyDerivationFunction {
    PBKDF2,
    Argon2id,    // not implemented
    HKDF,        // not implemented
    Scrypt,      // not implemented
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
    pub nonce: Vec<u8>,        // 12 random bytes, new for every message
    pub tag: Vec<u8>,          // 16-byte Poly1305 authentication tag
    pub salt: Vec<u8>,         // for key derivation (empty when the caller supplies a raw key)
    pub algorithm: EncryptionAlgorithm,
    pub iteration_count: u32,  // for KDF
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
    pub memory: u32,  // for Argon2 (unused)
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

fn not_implemented_cipher(algorithm: EncryptionAlgorithm) -> String {
    format!("{} is not implemented; use ChaCha20-Poly1305", algorithm)
}

impl EncryptionEngine {
    pub fn new(algorithm: EncryptionAlgorithm) -> Self {
        EncryptionEngine { algorithm }
    }

    pub fn default() -> Self {
        EncryptionEngine::new(EncryptionAlgorithm::ChaCha20Poly1305)
    }

    /// Encrypt and authenticate `plaintext` under a 32-byte `key` with a fresh random nonce.
    pub fn encrypt(&self, plaintext: &[u8], key: &[u8]) -> Result<EncryptedData, String> {
        if self.algorithm != EncryptionAlgorithm::ChaCha20Poly1305 {
            return Err(not_implemented_cipher(self.algorithm));
        }
        let key: [u8; 32] = key.try_into().map_err(|_| "Key must be exactly 32 bytes".to_string())?;
        let mut nonce = [0u8; 12];
        crate::os_random::fill(&mut nonce)?;
        let mut sealed = crate::aead::seal(&key, &nonce, &[], plaintext);
        let tag = sealed.split_off(sealed.len() - 16);
        Ok(EncryptedData {
            ciphertext: sealed,
            nonce: nonce.to_vec(),
            tag,
            salt: vec![],
            algorithm: self.algorithm,
            iteration_count: 1,
        })
    }

    /// Verify the tag, then decrypt. Any change to the ciphertext, nonce, tag or key is an error.
    pub fn decrypt(&self, encrypted: &EncryptedData, key: &[u8]) -> Result<Vec<u8>, String> {
        if self.algorithm != EncryptionAlgorithm::ChaCha20Poly1305 || encrypted.algorithm != self.algorithm {
            return Err(not_implemented_cipher(if encrypted.algorithm != self.algorithm { encrypted.algorithm } else { self.algorithm }));
        }
        let key: [u8; 32] = key.try_into().map_err(|_| "Key must be exactly 32 bytes".to_string())?;
        let nonce: [u8; 12] = encrypted.nonce.as_slice().try_into().map_err(|_| "Nonce must be 12 bytes".to_string())?;
        if encrypted.tag.len() != 16 {
            return Err("Tag must be 16 bytes".to_string());
        }
        let mut sealed = encrypted.ciphertext.clone();
        sealed.extend_from_slice(&encrypted.tag);
        crate::aead::open(&key, &nonce, &[], &sealed).ok_or_else(|| "Authentication failed: wrong key or the data was changed".to_string())
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

    pub fn hash(&self, data: &[u8]) -> Result<Vec<u8>, String> {
        match self.algorithm {
            HashingAlgorithm::SHA256 => Ok(sha256(data).to_vec()),
            HashingAlgorithm::SHA512 => Ok(sha512(data).to_vec()),
            other => Err(format!("{} is not implemented", other)),
        }
    }

    pub fn verify(&self, data: &[u8], hash: &[u8]) -> Result<bool, String> {
        Ok(constant_time_compare(&self.hash(data)?, hash))
    }
}

/// Password hasher
pub struct PasswordHasher {
    kdf: KeyDerivationFunction,
}

/// PBKDF2 rounds for new password hashes. OWASP suggests 600,000 for HMAC-SHA256; this pure-Rust hash is slow in
/// unoptimised builds, so the value is kept equal to what `secure_encrypt` uses. Existing hashes store their own count.
pub const PBKDF2_ITERATIONS: u32 = 100_000;

impl PasswordHasher {
    pub fn new(kdf: KeyDerivationFunction) -> Self {
        PasswordHasher { kdf }
    }

    pub fn hash_password(&self, password: &str) -> Result<PasswordHash, String> {
        match self.kdf {
            KeyDerivationFunction::PBKDF2 => {
                let mut salt = vec![0u8; 16];
                crate::os_random::fill(&mut salt)?;
                let hash = pbkdf2_hmac_sha256(password.as_bytes(), &salt, PBKDF2_ITERATIONS, 32);
                Ok(PasswordHash { hash, salt, algorithm: KeyDerivationFunction::PBKDF2, iterations: PBKDF2_ITERATIONS, memory: 0 })
            }
            other => Err(format!("{} is not implemented; use PBKDF2", other)),
        }
    }

    pub fn verify_password(&self, password: &str, hash: &PasswordHash) -> Result<bool, String> {
        if hash.algorithm != KeyDerivationFunction::PBKDF2 || self.kdf != KeyDerivationFunction::PBKDF2 {
            return Err("only PBKDF2 hashes can be verified".to_string());
        }
        let computed = pbkdf2_hmac_sha256(password.as_bytes(), &hash.salt, hash.iterations, hash.hash.len());
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
        // 32 random bytes from the operating system generator
        let key = generate_random_bytes(32)?;

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
        let new_key = generate_random_bytes(32)?;

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

fn generate_random_bytes(len: usize) -> Result<Vec<u8>, String> {
    let mut buf = vec![0u8; len];
    crate::os_random::fill(&mut buf)?;
    Ok(buf)
}

fn hex_encode(data: &[u8]) -> String {
    data.iter()
        .map(|b| format!("{:02x}", b))
        .collect::<Vec<_>>()
        .join("")
}

/// SHA-256 (FIPS 180-4), via the RustCrypto `sha2` crate.
pub fn sha256(data: &[u8]) -> [u8; 32] {
    use sha2::Digest;
    sha2::Sha256::digest(data).into()
}

/// SHA-512 (FIPS 180-4), via the RustCrypto `sha2` crate.
pub fn sha512(data: &[u8]) -> [u8; 64] {
    use sha2::Digest;
    sha2::Sha512::digest(data).into()
}

/// HMAC-SHA256 (RFC 2104 / RFC 4231), via the RustCrypto `hmac` crate.
pub fn hmac_sha256(key: &[u8], data: &[u8]) -> [u8; 32] {
    use hmac::{Hmac, Mac};
    let mut mac = <Hmac<sha2::Sha256> as Mac>::new_from_slice(key).expect("HMAC accepts keys of any length");
    mac.update(data);
    mac.finalize().into_bytes().into()
}

/// PBKDF2-HMAC-SHA256 (RFC 8018) for any output length, via the RustCrypto `pbkdf2` crate.
pub fn pbkdf2_hmac_sha256(password: &[u8], salt: &[u8], iterations: u32, dk_len: usize) -> Vec<u8> {
    let mut out = vec![0u8; dk_len];
    pbkdf2::pbkdf2_hmac::<sha2::Sha256>(password, salt, iterations.max(1), &mut out);
    out
}

/// Compare two byte strings without stopping at the first difference (length is not secret).
pub fn constant_time_compare(a: &[u8], b: &[u8]) -> bool {
    use subtle::ConstantTimeEq;
    a.len() == b.len() && bool::from(a.ct_eq(b))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encryption_engine_creation() {
        let engine = EncryptionEngine::new(EncryptionAlgorithm::ChaCha20Poly1305);
        assert_eq!(engine.algorithm, EncryptionAlgorithm::ChaCha20Poly1305);
    }

    #[test]
    fn engine_round_trips_and_rejects_tampering() {
        let engine = EncryptionEngine::default();
        let key = [7u8; 32];
        let enc = engine.encrypt(b"attack at dawn", &key).unwrap();
        assert_ne!(enc.ciphertext, b"attack at dawn".to_vec(), "not stored in the clear");
        assert_eq!(engine.decrypt(&enc, &key).unwrap(), b"attack at dawn".to_vec());
        let enc2 = engine.encrypt(b"attack at dawn", &key).unwrap();
        assert_ne!(enc.nonce, enc2.nonce, "fresh nonce every message");
        assert_ne!(enc.ciphertext, enc2.ciphertext);
        let mut bad = enc.clone();
        bad.ciphertext[0] ^= 1;
        assert!(engine.decrypt(&bad, &key).is_err());
        let mut bad = enc.clone();
        bad.tag[0] ^= 1;
        assert!(engine.decrypt(&bad, &key).is_err());
        let mut bad = enc.clone();
        bad.nonce[0] ^= 1;
        assert!(engine.decrypt(&bad, &key).is_err());
        assert!(engine.decrypt(&enc, &[8u8; 32]).is_err());
        assert!(engine.encrypt(b"x", &[0u8; 31]).is_err(), "key length is checked");
        assert!(engine.encrypt(b"x", &[0u8; 33]).is_err());
    }

    #[test]
    fn unimplemented_algorithms_fail_loudly() {
        assert!(EncryptionEngine::new(EncryptionAlgorithm::AES256GCM).encrypt(b"x", &[0u8; 32]).is_err());
        assert!(EncryptionEngine::new(EncryptionAlgorithm::AES256CBC).encrypt(b"x", &[0u8; 32]).is_err());
        assert!(HashingEngine::new(HashingAlgorithm::SHA3_256).hash(b"x").is_err());
        assert!(PasswordHasher::new(KeyDerivationFunction::Argon2id).hash_password("x").is_err());
    }

    #[test]
    fn test_generate_key_manager() {
        let manager = KeyManager::new();
        let result = manager.generate_key("key1".to_string(), EncryptionAlgorithm::ChaCha20Poly1305);

        assert!(result.is_ok());

        let key = manager.get_key("key1");
        assert!(key.is_ok());
        assert_eq!(key.unwrap().len(), 32);
    }

    #[test]
    fn test_password_verification() {
        let hasher = PasswordHasher::new(KeyDerivationFunction::PBKDF2);
        let hash = hasher.hash_password("password123").unwrap();
        let other = hasher.hash_password("password123").unwrap();
        assert_ne!(hash.salt, other.salt, "random salt");
        assert_ne!(hash.hash, other.hash, "same password, different salt, different hash");

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
        assert!(!constant_time_compare(&a, &a[..3]));
    }

    #[test]
    fn test_key_rotation() {
        let manager = KeyManager::new();
        manager
            .generate_key("key1".to_string(), EncryptionAlgorithm::ChaCha20Poly1305)
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
            .generate_key("key1".to_string(), EncryptionAlgorithm::ChaCha20Poly1305)
            .unwrap();

        let result = manager.mark_compromised("key1");
        assert!(result.is_ok());
    }
}
