//! Cryptography helpers. Only SHA-256 is real here (it calls the verified implementation); AES is NOT implemented and
//! returns an error rather than handing back the plaintext. See docs/CRYPTO.md.

pub fn sha256(data: &[u8]) -> Vec<u8> {
    crate::builtin::sha256_digest(data).to_vec()
}

pub fn aes_encrypt(_data: &[u8], _key: &[u8]) -> Result<Vec<u8>, String> {
    Err("AES is not implemented; use ChaCha20-Poly1305 (killer_native::aead)".to_string())
}

pub fn aes_decrypt(_data: &[u8], _key: &[u8]) -> Result<Vec<u8>, String> {
    Err("AES is not implemented; use ChaCha20-Poly1305 (killer_native::aead)".to_string())
}
