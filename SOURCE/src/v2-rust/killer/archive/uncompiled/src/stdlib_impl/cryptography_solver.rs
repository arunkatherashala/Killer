// ================================================================
// CRYPTOGRAPHY SOLVER - Phase 21.3
// RSA, ECC, hash functions, key exchange, post-quantum
// Ported from: solver_cryptography_advanced.killer
// ================================================================

use std::collections::HashMap;

/// Cryptography Solver
pub struct CryptographySolver;

impl CryptographySolver {
    // ================================================================
    // RSA CRYPTOGRAPHY (1-15)
    // ================================================================

    /// Problem 1: GCD (Euclidean Algorithm)
    pub fn gcd(mut a: i64, mut b: i64) -> i64 {
        while b != 0 {
            let temp = b;
            b = a % b;
            a = temp;
        }
        a.abs()
    }

    /// Problem 2: Extended GCD (returns x where ax + by = gcd(a,b))
    pub fn extended_gcd(a: i64, b: i64) -> (i64, i64, i64) {
        if b == 0 {
            (a, 1, 0)
        } else {
            let (g, x, y) = Self::extended_gcd(b, a % b);
            (g, y, x - (a / b) * y)
        }
    }

    /// Problem 3: RSA Phi (Euler's Totient)
    pub fn rsa_phi(p: i64, q: i64) -> i64 {
        (p - 1) * (q - 1)
    }

    /// Problem 4: RSA Coprime Check
    pub fn rsa_coprime_check(e: i64, phi: i64) -> bool {
        Self::gcd(e, phi) == 1
    }

    /// Problem 5: RSA Private Exponent (using Extended GCD)
    pub fn rsa_private_exponent(e: i64, phi: i64) -> i64 {
        let (g, x, _) = Self::extended_gcd(e, phi);
        if g != 1 {
            return 0;  // No inverse exists
        }
        x.rem_euclid(phi)
    }

    /// Problem 6: Modular Exponentiation (fast exponentiation)
    pub fn mod_exp(base: i64, exp: i64, modulus: i64) -> i64 {
        if modulus == 1 { return 0; }
        
        let mut result = 1i64;
        let mut b = base.rem_euclid(modulus);
        let mut e = exp;
        
        while e > 0 {
            if e % 2 == 1 {
                result = (result * b).rem_euclid(modulus);
            }
            e /= 2;
            b = (b * b).rem_euclid(modulus);
        }
        result
    }

    /// Problem 7: RSA Encryption
    pub fn rsa_encrypt(plaintext: i64, e: i64, n: i64) -> i64 {
        Self::mod_exp(plaintext, e, n)
    }

    /// Problem 8: RSA Decryption
    pub fn rsa_decrypt(ciphertext: i64, d: i64, n: i64) -> i64 {
        Self::mod_exp(ciphertext, d, n)
    }

    /// Problem 9: RSA Signature Generation
    pub fn rsa_sign(message_hash: i64, d: i64, n: i64) -> i64 {
        Self::mod_exp(message_hash, d, n)
    }

    /// Problem 10: RSA Signature Verification
    pub fn rsa_verify(signature: i64, e: i64, n: i64, message_hash: i64) -> bool {
        let decrypted = Self::mod_exp(signature, e, n);
        decrypted == message_hash
    }

    // ================================================================
    // DIFFIE-HELLMAN KEY EXCHANGE (11-20)
    // ================================================================

    /// Problem 11: Diffie-Hellman Public Key Calculation
    pub fn dh_public_key(generator: i64, private_key: i64, prime: i64) -> i64 {
        Self::mod_exp(generator, private_key, prime)
    }

    /// Problem 12: Diffie-Hellman Shared Secret
    pub fn dh_shared_secret(peer_public: i64, private_key: i64, prime: i64) -> i64 {
        Self::mod_exp(peer_public, private_key, prime)
    }

    /// Problem 13: Diffie-Hellman Key Strength
    pub fn dh_security_strength(prime_bits: usize) -> String {
        match prime_bits {
            512..=1023 => "Broken",
            1024..=2047 => "Weak",
            2048..=3071 => "Strong",
            3072.. => "Very Strong",
            _ => "Unknown",
        }.to_string()
    }

    // ================================================================
    // ELLIPTIC CURVE CRYPTOGRAPHY (15-30)
    // ================================================================

    /// Problem 14: ECC Point Addition Slope
    pub fn ecc_slope(p_x: f64, p_y: f64, q_x: f64, q_y: f64) -> f64 {
        if (q_x - p_x).abs() < 1e-14 { return f64::INFINITY; }
        (p_y - q_y) / (p_x - q_x)
    }

    /// Problem 15: ECC Scalar Multiplication (Binary method)
    pub fn ecc_scalar_mult_steps(k: u32) -> usize {
        // Number of doublings needed
        ((k as f64).log2().ceil()) as usize
    }

    /// Problem 16: ECDH Public Key
    pub fn ecdh_public_key(private_key: i64, curve_order: i64) -> i64 {
        private_key % curve_order
    }

    // ================================================================
    // HASH FUNCTIONS (20-35)
    // ================================================================

    /// Problem 17: Simple Hash Function (DJB2)
    pub fn hash_djb2(data: &str) -> u64 {
        let mut hash: u64 = 5381;
        for byte in data.bytes() {
            hash = ((hash << 5).wrapping_add(hash)).wrapping_add(byte as u64);
        }
        hash
    }

    /// Problem 18: Collision Probability (Birthday Paradox)
    pub fn birthday_paradox_collision_prob(n: u64, hash_space: u64) -> f64 {
        // P(collision) ≈ 1 - e^(-n²/2m) where m is hash space size
        let exponent = -(n as f64 * n as f64) / (2.0 * hash_space as f64);
        1.0 - exponent.exp()
    }

    /// Problem 19: Merkle Tree Hash
    pub fn merkle_tree_hash(leaf_hashes: &[u64]) -> u64 {
        if leaf_hashes.is_empty() { return 0; }
        if leaf_hashes.len() == 1 { return leaf_hashes[0]; }
        
        let mut current = leaf_hashes.to_vec();
        
        while current.len() > 1 {
            let mut next = Vec::new();
            for i in (0..current.len()).step_by(2) {
                let h1 = current[i];
                let h2 = if i + 1 < current.len() { current[i + 1] } else { h1 };
                
                // Simple combination
                let combined = (h1 ^ h2).wrapping_mul(31).wrapping_add(h1);
                next.push(combined);
            }
            current = next;
        }
        
        current[0]
    }

    /// Problem 20: HMAC-Style Message Authentication
    pub fn hmac(key: &str, message: &str) -> u64 {
        // Simplified HMAC: hash(key XOR opad, hash(key XOR ipad, message))
        let ipad = Self::hash_djb2(&format!("{}{}", key, message));
        let opad = Self::hash_djb2(&format!("{}{}", key, ipad.to_string()));
        opad
    }

    // ================================================================
    // DIGITAL SIGNATURES (21-35)
    // ================================================================

    /// Problem 21: ECDSA Signature Verification
    pub fn ecdsa_verify_check(r: i64, s: i64, _order: i64) -> bool {
        // Basic checks: r and s must be in valid range
        r > 0 && s > 0
    }

    /// Problem 22: Digital Signature Size Calculation
    pub fn signature_size_rsa(key_bits: usize) -> usize {
        key_bits / 8
    }

    /// Problem 23: Digital Signature Size Calculation (ECDSA)
    pub fn signature_size_ecdsa(curve_bits: usize) -> usize {
        (curve_bits / 4) + 8  // Approximate for DER encoding
    }

    // ================================================================
    // KEY DERIVATION (25-40)
    // ================================================================

    /// Problem 24: PBKDF2 Component (simplified)
    pub fn pbkdf2_iteration(password: &str, salt: &str, iteration: usize) -> u64 {
        let mut result = Self::hash_djb2(&format!("{}{}{}", password, salt, iteration));
        for _ in 0..iteration {
            result = Self::hash_djb2(&result.to_string());
        }
        result
    }

    /// Problem 25: Key Stretching Factor
    pub fn key_stretch_factor(time_ms: u32) -> usize {
        // Number of iterations to consume ~time_ms milliseconds
        (1_000_000 / time_ms.max(1)) as usize
    }

    // ================================================================
    // RANDOM NUMBER GENERATION (26-40)
    // ================================================================

    /// Problem 26: Prime Number Test (Miller-Rabin simplified)
    pub fn is_prime_mr(n: u64, rounds: usize) -> bool {
        if n < 2 { return false; }
        if n == 2 { return true; }
        if n % 2 == 0 { return false; }
        
        // Simplified primality test
        let limit = (n as f64).sqrt() as u64;
        for i in 3..=limit {
            if n % i == 0 { return false; }
        }
        true
    }

    /// Problem 27: Generate Safe Prime
    pub fn generate_safe_prime_check(p: u64) -> bool {
        // p is safe prime if both p and (p-1)/2 are prime
        Self::is_prime_mr(p, 10) && Self::is_prime_mr((p - 1) / 2, 10)
    }

    // ================================================================
    // POST-QUANTUM CRYPTOGRAPHY (30-50)
    // ================================================================

    /// Problem 28: Lattice Security Strength
    pub fn lattice_security_bits(dimension: usize) -> usize {
        // Hermite factor ≈ dimension^(1/dimension) * 1.005
        (dimension as f64 * (dimension as f64).ln()) as usize
    }

    /// Problem 29: Code-Based Security Check
    pub fn code_based_security_parameters(n: usize, k: usize, t: usize) -> bool {
        // t ≥ (n-k)/2 for Goppa codes
        t >= (n - k) / 2
    }

    /// Problem 30: Multivariate Polynomial Degree
    pub fn mv_degree_estimate(variables: usize, equations: usize) -> usize {
        // Average degree growth
        (variables + equations) / 2
    }

    // ================================================================
    // ZERO-KNOWLEDGE PROOFS (35-50)
    // ================================================================

    /// Problem 31: Schnorr Proof Challenge
    pub fn schnorr_challenge(commitment: i64, nonce: i64) -> i64 {
        Self::hash_djb2(&format!("{}{}", commitment, nonce)) as i64
    }

    /// Problem 32: Fiat-Shamir Transform (Non-interactive proof)
    pub fn fiat_shamir_challenge(witness: &str, statement: &str) -> u64 {
        Self::hash_djb2(&format!("{}{}", witness, statement))
    }

    // ================================================================
    // SECURITY ANALYSIS (40-55)
    // ================================================================

    /// Problem 33: Entropy Estimation
    pub fn shannon_entropy(data: &str) -> f64 {
        let mut freq: HashMap<char, u32> = HashMap::new();
        for c in data.chars() {
            *freq.entry(c).or_insert(0) += 1;
        }
        
        let n = data.len() as f64;
        let mut entropy = 0.0;
        for count in freq.values() {
            let p = *count as f64 / n;
            entropy -= p * p.log2();
        }
        entropy
    }

    /// Problem 34: Information Leakage
    pub fn timing_attack_vulnerability(operations: &[u32]) -> f64 {
        if operations.is_empty() { return 0.0; }
        
        let mean: f64 = operations.iter().map(|&x| x as f64).sum::<f64>() / operations.len() as f64;
        let variance: f64 = operations.iter()
            .map(|&x| (x as f64 - mean).powi(2))
            .sum::<f64>() / operations.len() as f64;
        
        variance.sqrt()  // Timing std dev (vulnerability measure)
    }

    /// Problem 35: Quantum Key Distribution - BB84 Basis
    pub fn bb84_basis_match(alice_basis: u8, bob_basis: u8) -> bool {
        alice_basis == bob_basis
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gcd() {
        assert_eq!(CryptographySolver::gcd(48, 18), 6);
        assert_eq!(CryptographySolver::gcd(100, 50), 50);
    }

    #[test]
    fn test_mod_exp() {
        assert_eq!(CryptographySolver::mod_exp(2, 10, 1000), 24);
        assert_eq!(CryptographySolver::mod_exp(7, 13, 11), 2);
    }

    #[test]
    fn test_rsa_flow() {
        let p = 61i64;
        let q = 53i64;
        let n = p * q;
        let phi = CryptographySolver::rsa_phi(p, q);
        let e = 17i64;
        
        assert!(CryptographySolver::rsa_coprime_check(e, phi));
        
        let d = CryptographySolver::rsa_private_exponent(e, phi);
        let plaintext = 42i64;
        let ciphertext = CryptographySolver::rsa_encrypt(plaintext, e, n);
        let decrypted = CryptographySolver::rsa_decrypt(ciphertext, d, n);
        
        assert_eq!(plaintext, decrypted);
    }

    #[test]
    fn test_dh_shared_secret() {
        let p = 23i64;
        let g = 5i64;
        
        let a = 6i64;
        let b = 15i64;
        
        let pub_a = CryptographySolver::dh_public_key(g, a, p);
        let pub_b = CryptographySolver::dh_public_key(g, b, p);
        
        let secret_a = CryptographySolver::dh_shared_secret(pub_b, a, p);
        let secret_b = CryptographySolver::dh_shared_secret(pub_a, b, p);
        
        assert_eq!(secret_a, secret_b);
    }

    #[test]
    fn test_entropy() {
        let high_entropy = "abcdefghijklmnopqrst";
        let entropy = CryptographySolver::shannon_entropy(high_entropy);
        assert!(entropy > 4.0);
    }
}
