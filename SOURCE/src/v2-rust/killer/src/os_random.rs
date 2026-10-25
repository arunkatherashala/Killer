#![allow(unsafe_code)]
//! Random bytes from the operating system's cryptographic generator, with no external crates.
//!
//! Windows: `BCryptGenRandom` with the system-preferred RNG. Unix (Linux, macOS, BSD): `/dev/urandom`.
//! There is deliberately no fallback to a weaker source: if the system generator fails, the error is
//! returned and the caller must fail (a key or salt from a guessable generator is worse than no key).

/// Fill `buf` with cryptographically secure random bytes.
pub fn fill(buf: &mut [u8]) -> Result<(), String> {
    if buf.is_empty() {
        return Ok(());
    }
    fill_os(buf)
}

#[cfg(windows)]
fn fill_os(buf: &mut [u8]) -> Result<(), String> {
    #[link(name = "bcrypt")]
    extern "system" {
        fn BCryptGenRandom(algorithm: *mut core::ffi::c_void, buffer: *mut u8, count: u32, flags: u32) -> i32;
    }
    const BCRYPT_USE_SYSTEM_PREFERRED_RNG: u32 = 0x0000_0002;
    // at most u32::MAX bytes per call; larger buffers are filled in pieces
    for chunk in buf.chunks_mut(1 << 20) {
        // SAFETY: `chunk` is a valid, writable buffer of `chunk.len()` bytes for the whole call, and a
        // null algorithm handle is what BCRYPT_USE_SYSTEM_PREFERRED_RNG requires.
        let status = unsafe { BCryptGenRandom(core::ptr::null_mut(), chunk.as_mut_ptr(), chunk.len() as u32, BCRYPT_USE_SYSTEM_PREFERRED_RNG) };
        if status != 0 {
            return Err(format!("the operating system random generator failed (BCryptGenRandom status {:#x})", status));
        }
    }
    Ok(())
}

#[cfg(not(windows))]
fn fill_os(buf: &mut [u8]) -> Result<(), String> {
    use std::io::Read;
    let mut f = std::fs::File::open("/dev/urandom").map_err(|e| format!("cannot open /dev/urandom: {}", e))?;
    f.read_exact(buf).map_err(|e| format!("cannot read /dev/urandom: {}", e))
}

#[cfg(test)]
mod tests {
    use super::fill;

    #[test]
    fn fills_with_different_bytes_each_time() {
        let (mut a, mut b) = ([0u8; 32], [0u8; 32]);
        fill(&mut a).unwrap();
        fill(&mut b).unwrap();
        assert_ne!(a, [0u8; 32], "all zero");
        assert_ne!(a, b, "two draws were identical");
    }

    #[test]
    fn spreads_across_the_byte_range() {
        let mut big = vec![0u8; 8192];
        fill(&mut big).unwrap();
        let mut seen = [false; 256];
        for b in &big {
            seen[*b as usize] = true;
        }
        // 8192 uniform bytes miss a given value with probability (255/256)^8192, about 1e-14
        assert!(seen.iter().all(|s| *s), "some byte values never appeared");
    }

    #[test]
    fn empty_and_large_buffers_work() {
        fill(&mut []).unwrap();
        let mut huge = vec![0u8; (1 << 20) + 17];
        fill(&mut huge).unwrap();
        assert!(huge.iter().any(|b| *b != 0));
    }
}
