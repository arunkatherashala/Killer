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

/// SHA-256 (FIPS 180-4).
pub fn sha256(data: &[u8]) -> [u8; 32] {
    crate::builtin::sha256_digest(data)
}

const SHA512_K: [u64; 80] = [
    0x428a2f98d728ae22, 0x7137449123ef65cd, 0xb5c0fbcfec4d3b2f, 0xe9b5dba58189dbbc, 0x3956c25bf348b538,
    0x59f111f1b605d019, 0x923f82a4af194f9b, 0xab1c5ed5da6d8118, 0xd807aa98a3030242, 0x12835b0145706fbe,
    0x243185be4ee4b28c, 0x550c7dc3d5ffb4e2, 0x72be5d74f27b896f, 0x80deb1fe3b1696b1, 0x9bdc06a725c71235,
    0xc19bf174cf692694, 0xe49b69c19ef14ad2, 0xefbe4786384f25e3, 0x0fc19dc68b8cd5b5, 0x240ca1cc77ac9c65,
    0x2de92c6f592b0275, 0x4a7484aa6ea6e483, 0x5cb0a9dcbd41fbd4, 0x76f988da831153b5, 0x983e5152ee66dfab,
    0xa831c66d2db43210, 0xb00327c898fb213f, 0xbf597fc7beef0ee4, 0xc6e00bf33da88fc2, 0xd5a79147930aa725,
    0x06ca6351e003826f, 0x142929670a0e6e70, 0x27b70a8546d22ffc, 0x2e1b21385c26c926, 0x4d2c6dfc5ac42aed,
    0x53380d139d95b3df, 0x650a73548baf63de, 0x766a0abb3c77b2a8, 0x81c2c92e47edaee6, 0x92722c851482353b,
    0xa2bfe8a14cf10364, 0xa81a664bbc423001, 0xc24b8b70d0f89791, 0xc76c51a30654be30, 0xd192e819d6ef5218,
    0xd69906245565a910, 0xf40e35855771202a, 0x106aa07032bbd1b8, 0x19a4c116b8d2d0c8, 0x1e376c085141ab53,
    0x2748774cdf8eeb99, 0x34b0bcb5e19b48a8, 0x391c0cb3c5c95a63, 0x4ed8aa4ae3418acb, 0x5b9cca4f7763e373,
    0x682e6ff3d6b2b8a3, 0x748f82ee5defb2fc, 0x78a5636f43172f60, 0x84c87814a1f0ab72, 0x8cc702081a6439ec,
    0x90befffa23631e28, 0xa4506cebde82bde9, 0xbef9a3f7b2c67915, 0xc67178f2e372532b, 0xca273eceea26619c,
    0xd186b8c721c0c207, 0xeada7dd6cde0eb1e, 0xf57d4f7fee6ed178, 0x06f067aa72176fba, 0x0a637dc5a2c898a6,
    0x113f9804bef90dae, 0x1b710b35131c471b, 0x28db77f523047d84, 0x32caab7b40c72493, 0x3c9ebe0a15c9bebc,
    0x431d67c49c100d4c, 0x4cc5d4becb3e42b6, 0x597f299cfc657e2a, 0x5fcb6fab3ad6faec, 0x6c44198c4a475817,
];

/// SHA-512 (FIPS 180-4).
pub fn sha512(data: &[u8]) -> [u8; 64] {
    let mut h: [u64; 8] = [
        0x6a09e667f3bcc908, 0xbb67ae8584caa73b, 0x3c6ef372fe94f82b, 0xa54ff53a5f1d36f1,
        0x510e527fade682d1, 0x9b05688c2b3e6c1f, 0x1f83d9abfb41bd6b, 0x5be0cd19137e2179,
    ];
    let mut msg = data.to_vec();
    msg.push(0x80);
    while msg.len() % 128 != 112 {
        msg.push(0);
    }
    msg.extend_from_slice(&((data.len() as u128) * 8).to_be_bytes());
    for block in msg.chunks_exact(128) {
        let mut w = [0u64; 80];
        for i in 0..16 {
            w[i] = u64::from_be_bytes(block[i * 8..i * 8 + 8].try_into().unwrap());
        }
        for i in 16..80 {
            let s0 = w[i - 15].rotate_right(1) ^ w[i - 15].rotate_right(8) ^ (w[i - 15] >> 7);
            let s1 = w[i - 2].rotate_right(19) ^ w[i - 2].rotate_right(61) ^ (w[i - 2] >> 6);
            w[i] = w[i - 16].wrapping_add(s0).wrapping_add(w[i - 7]).wrapping_add(s1);
        }
        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut hh] = h;
        for i in 0..80 {
            let s1 = e.rotate_right(14) ^ e.rotate_right(18) ^ e.rotate_right(41);
            let ch = (e & f) ^ (!e & g);
            let t1 = hh.wrapping_add(s1).wrapping_add(ch).wrapping_add(SHA512_K[i]).wrapping_add(w[i]);
            let s0 = a.rotate_right(28) ^ a.rotate_right(34) ^ a.rotate_right(39);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let t2 = s0.wrapping_add(maj);
            hh = g;
            g = f;
            f = e;
            e = d.wrapping_add(t1);
            d = c;
            c = b;
            b = a;
            a = t1.wrapping_add(t2);
        }
        for (x, y) in h.iter_mut().zip([a, b, c, d, e, f, g, hh]) {
            *x = x.wrapping_add(y);
        }
    }
    let mut out = [0u8; 64];
    for (i, v) in h.iter().enumerate() {
        out[i * 8..i * 8 + 8].copy_from_slice(&v.to_be_bytes());
    }
    out
}

/// HMAC-SHA256 (RFC 2104 / RFC 4231).
pub fn hmac_sha256(key: &[u8], data: &[u8]) -> [u8; 32] {
    let mut k = if key.len() > 64 { sha256(key).to_vec() } else { key.to_vec() };
    k.resize(64, 0);
    let mut inner: Vec<u8> = k.iter().map(|b| b ^ 0x36).collect();
    inner.extend_from_slice(data);
    let inner_hash = sha256(&inner);
    let mut outer: Vec<u8> = k.iter().map(|b| b ^ 0x5c).collect();
    outer.extend_from_slice(&inner_hash);
    sha256(&outer)
}

/// PBKDF2-HMAC-SHA256 (RFC 8018) for any output length.
pub fn pbkdf2_hmac_sha256(password: &[u8], salt: &[u8], iterations: u32, dk_len: usize) -> Vec<u8> {
    let iterations = iterations.max(1);
    let mut out = Vec::with_capacity(dk_len + 32);
    let mut block_index: u32 = 1;
    while out.len() < dk_len {
        let mut msg = salt.to_vec();
        msg.extend_from_slice(&block_index.to_be_bytes());
        let mut u = hmac_sha256(password, &msg);
        let mut t = u;
        for _ in 1..iterations {
            u = hmac_sha256(password, &u);
            for (a, b) in t.iter_mut().zip(u.iter()) {
                *a ^= b;
            }
        }
        out.extend_from_slice(&t);
        block_index += 1;
    }
    out.truncate(dk_len);
    out
}

/// Compare two byte strings without stopping at the first difference (length is not secret).
pub fn constant_time_compare(a: &[u8], b: &[u8]) -> bool {
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
