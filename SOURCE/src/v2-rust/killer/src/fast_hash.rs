//! Small, fast, non-cryptographic hasher (FNV-1a) for short identifier keys.
//! HashMap's default SipHash is DoS-resistant but slow for the variable-name lookups the VM does
//! constantly; scope keys come from the program text, not from untrusted network input.

use std::collections::HashMap;
use std::hash::{BuildHasherDefault, Hasher};

#[derive(Default, Clone, Copy)]
pub struct FnvHasher(u64);

impl Hasher for FnvHasher {
    #[inline]
    fn finish(&self) -> u64 {
        self.0
    }

    #[inline]
    fn write(&mut self, bytes: &[u8]) {
        let mut h = if self.0 == 0 { 0xcbf29ce484222325 } else { self.0 };
        for b in bytes {
            h ^= *b as u64;
            h = h.wrapping_mul(0x100000001b3);
        }
        self.0 = h;
    }
}

pub type FastMap<K, V> = HashMap<K, V, BuildHasherDefault<FnvHasher>>;
pub type FastSet<K> = std::collections::HashSet<K, BuildHasherDefault<FnvHasher>>;
