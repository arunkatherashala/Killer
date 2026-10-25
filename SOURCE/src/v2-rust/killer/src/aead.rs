//! ChaCha20-Poly1305 authenticated encryption (RFC 8439), pure Rust, no external crates.
//!
//! `seal` encrypts and authenticates; `open` verifies the tag in constant time *before* returning any
//! plaintext. A nonce must never be reused with the same key (callers draw a fresh random 96-bit nonce
//! for every message). Checked against the RFC 8439 test vectors and against vectors produced by an
//! independent implementation (see the tests).

const SIGMA: [u32; 4] = [0x6170_7865, 0x3320_646e, 0x7962_2d32, 0x6b20_6574];

#[inline(always)]
fn quarter(s: &mut [u32; 16], a: usize, b: usize, c: usize, d: usize) {
    s[a] = s[a].wrapping_add(s[b]);
    s[d] = (s[d] ^ s[a]).rotate_left(16);
    s[c] = s[c].wrapping_add(s[d]);
    s[b] = (s[b] ^ s[c]).rotate_left(12);
    s[a] = s[a].wrapping_add(s[b]);
    s[d] = (s[d] ^ s[a]).rotate_left(8);
    s[c] = s[c].wrapping_add(s[d]);
    s[b] = (s[b] ^ s[c]).rotate_left(7);
}

/// One 64-byte ChaCha20 keystream block.
fn block(key: &[u8; 32], counter: u32, nonce: &[u8; 12]) -> [u8; 64] {
    let mut st = [0u32; 16];
    st[..4].copy_from_slice(&SIGMA);
    for i in 0..8 {
        st[4 + i] = u32::from_le_bytes([key[4 * i], key[4 * i + 1], key[4 * i + 2], key[4 * i + 3]]);
    }
    st[12] = counter;
    for i in 0..3 {
        st[13 + i] = u32::from_le_bytes([nonce[4 * i], nonce[4 * i + 1], nonce[4 * i + 2], nonce[4 * i + 3]]);
    }
    let mut w = st;
    for _ in 0..10 {
        quarter(&mut w, 0, 4, 8, 12);
        quarter(&mut w, 1, 5, 9, 13);
        quarter(&mut w, 2, 6, 10, 14);
        quarter(&mut w, 3, 7, 11, 15);
        quarter(&mut w, 0, 5, 10, 15);
        quarter(&mut w, 1, 6, 11, 12);
        quarter(&mut w, 2, 7, 8, 13);
        quarter(&mut w, 3, 4, 9, 14);
    }
    let mut out = [0u8; 64];
    for i in 0..16 {
        out[4 * i..4 * i + 4].copy_from_slice(&w[i].wrapping_add(st[i]).to_le_bytes());
    }
    out
}

fn xor_stream(key: &[u8; 32], first_counter: u32, nonce: &[u8; 12], data: &mut [u8]) {
    for (i, chunk) in data.chunks_mut(64).enumerate() {
        let ks = block(key, first_counter.wrapping_add(i as u32), nonce);
        for (b, k) in chunk.iter_mut().zip(ks.iter()) {
            *b ^= k;
        }
    }
}

/// Poly1305 one-time authenticator (26-bit limbs, 32-bit arithmetic with 64-bit products).
struct Poly1305 {
    r: [u32; 5],
    h: [u32; 5],
    pad: [u32; 4],
    buf: [u8; 16],
    used: usize,
}

fn le32(b: &[u8]) -> u32 {
    u32::from_le_bytes([b[0], b[1], b[2], b[3]])
}

impl Poly1305 {
    fn new(key: &[u8; 32]) -> Poly1305 {
        Poly1305 {
            r: [
                le32(&key[0..]) & 0x3ff_ffff,
                (le32(&key[3..]) >> 2) & 0x3ff_ff03,
                (le32(&key[6..]) >> 4) & 0x3ff_c0ff,
                (le32(&key[9..]) >> 6) & 0x3f0_3fff,
                (le32(&key[12..]) >> 8) & 0x00f_ffff,
            ],
            h: [0; 5],
            pad: [le32(&key[16..]), le32(&key[20..]), le32(&key[24..]), le32(&key[28..])],
            buf: [0; 16],
            used: 0,
        }
    }

    fn process(&mut self, m: &[u8; 16], hibit: u32) {
        let [r0, r1, r2, r3, r4] = self.r;
        let (s1, s2, s3, s4) = (r1 * 5, r2 * 5, r3 * 5, r4 * 5);
        let h = &mut self.h;
        h[0] += le32(&m[0..]) & 0x3ff_ffff;
        h[1] += (le32(&m[3..]) >> 2) & 0x3ff_ffff;
        h[2] += (le32(&m[6..]) >> 4) & 0x3ff_ffff;
        h[3] += (le32(&m[9..]) >> 6) & 0x3ff_ffff;
        h[4] += (le32(&m[12..]) >> 8) | hibit;
        let (h0, h1, h2, h3, h4) = (h[0] as u64, h[1] as u64, h[2] as u64, h[3] as u64, h[4] as u64);
        let (r0, r1, r2, r3, r4) = (r0 as u64, r1 as u64, r2 as u64, r3 as u64, r4 as u64);
        let (s1, s2, s3, s4) = (s1 as u64, s2 as u64, s3 as u64, s4 as u64);
        let d0 = h0 * r0 + h1 * s4 + h2 * s3 + h3 * s2 + h4 * s1;
        let mut d1 = h0 * r1 + h1 * r0 + h2 * s4 + h3 * s3 + h4 * s2;
        let mut d2 = h0 * r2 + h1 * r1 + h2 * r0 + h3 * s4 + h4 * s3;
        let mut d3 = h0 * r3 + h1 * r2 + h2 * r1 + h3 * r0 + h4 * s4;
        let mut d4 = h0 * r4 + h1 * r3 + h2 * r2 + h3 * r1 + h4 * r0;
        let mut c = d0 >> 26;
        h[0] = (d0 & 0x3ff_ffff) as u32;
        d1 += c;
        c = d1 >> 26;
        h[1] = (d1 & 0x3ff_ffff) as u32;
        d2 += c;
        c = d2 >> 26;
        h[2] = (d2 & 0x3ff_ffff) as u32;
        d3 += c;
        c = d3 >> 26;
        h[3] = (d3 & 0x3ff_ffff) as u32;
        d4 += c;
        c = d4 >> 26;
        h[4] = (d4 & 0x3ff_ffff) as u32;
        let d0 = (h[0] as u64) + c * 5;
        c = d0 >> 26;
        h[0] = (d0 & 0x3ff_ffff) as u32;
        h[1] += c as u32;
    }

    fn update(&mut self, mut data: &[u8]) {
        if self.used > 0 {
            let take = (16 - self.used).min(data.len());
            self.buf[self.used..self.used + take].copy_from_slice(&data[..take]);
            self.used += take;
            data = &data[take..];
            if self.used < 16 {
                return;
            }
            let block = self.buf;
            self.process(&block, 1 << 24);
            self.used = 0;
        }
        while data.len() >= 16 {
            let mut block = [0u8; 16];
            block.copy_from_slice(&data[..16]);
            self.process(&block, 1 << 24);
            data = &data[16..];
        }
        if !data.is_empty() {
            self.buf[..data.len()].copy_from_slice(data);
            self.used = data.len();
        }
    }

    fn finish(mut self) -> [u8; 16] {
        if self.used > 0 {
            let mut block = [0u8; 16];
            block[..self.used].copy_from_slice(&self.buf[..self.used]);
            block[self.used] = 1;
            self.process(&block, 0);
        }
        let h = &mut self.h;
        // fully carry
        let mut c = h[1] >> 26;
        h[1] &= 0x3ff_ffff;
        h[2] += c;
        c = h[2] >> 26;
        h[2] &= 0x3ff_ffff;
        h[3] += c;
        c = h[3] >> 26;
        h[3] &= 0x3ff_ffff;
        h[4] += c;
        c = h[4] >> 26;
        h[4] &= 0x3ff_ffff;
        h[0] += c * 5;
        c = h[0] >> 26;
        h[0] &= 0x3ff_ffff;
        h[1] += c;
        // g = h + 5 - 2^130; choose h or g in constant time
        let mut g = [0u32; 5];
        g[0] = h[0] + 5;
        c = g[0] >> 26;
        g[0] &= 0x3ff_ffff;
        for i in 1..4 {
            g[i] = h[i] + c;
            c = g[i] >> 26;
            g[i] &= 0x3ff_ffff;
        }
        g[4] = h[4].wrapping_add(c).wrapping_sub(1 << 26);
        let mask = (g[4] >> 31).wrapping_sub(1); // all ones when h >= p (g did not go negative)
        for i in 0..5 {
            h[i] = (h[i] & !mask) | (g[i] & mask);
        }
        // h mod 2^128, then add the pad
        let h0 = h[0] | (h[1] << 26);
        let h1 = (h[1] >> 6) | (h[2] << 20);
        let h2 = (h[2] >> 12) | (h[3] << 14);
        let h3 = (h[3] >> 18) | (h[4] << 8);
        let mut f = h0 as u64 + self.pad[0] as u64;
        let t0 = f as u32;
        f = h1 as u64 + self.pad[1] as u64 + (f >> 32);
        let t1 = f as u32;
        f = h2 as u64 + self.pad[2] as u64 + (f >> 32);
        let t2 = f as u32;
        f = h3 as u64 + self.pad[3] as u64 + (f >> 32);
        let t3 = f as u32;
        let mut tag = [0u8; 16];
        tag[0..4].copy_from_slice(&t0.to_le_bytes());
        tag[4..8].copy_from_slice(&t1.to_le_bytes());
        tag[8..12].copy_from_slice(&t2.to_le_bytes());
        tag[12..16].copy_from_slice(&t3.to_le_bytes());
        tag
    }
}

fn tag_for(key: &[u8; 32], nonce: &[u8; 12], aad: &[u8], ciphertext: &[u8]) -> [u8; 16] {
    let mut otk = [0u8; 32];
    otk.copy_from_slice(&block(key, 0, nonce)[..32]);
    let mut mac = Poly1305::new(&otk);
    let zeros = [0u8; 16];
    mac.update(aad);
    mac.update(&zeros[..(16 - aad.len() % 16) % 16]);
    mac.update(ciphertext);
    mac.update(&zeros[..(16 - ciphertext.len() % 16) % 16]);
    mac.update(&(aad.len() as u64).to_le_bytes());
    mac.update(&(ciphertext.len() as u64).to_le_bytes());
    mac.finish()
}

/// Encrypt `plaintext`; returns `ciphertext || 16-byte tag`.
pub fn seal(key: &[u8; 32], nonce: &[u8; 12], aad: &[u8], plaintext: &[u8]) -> Vec<u8> {
    let mut out = plaintext.to_vec();
    xor_stream(key, 1, nonce, &mut out);
    let tag = tag_for(key, nonce, aad, &out);
    out.extend_from_slice(&tag);
    out
}

/// Verify and decrypt `ciphertext || tag`. Returns `None` (and no plaintext) if the tag does not match.
pub fn open(key: &[u8; 32], nonce: &[u8; 12], aad: &[u8], sealed: &[u8]) -> Option<Vec<u8>> {
    if sealed.len() < 16 {
        return None;
    }
    let (ct, tag) = sealed.split_at(sealed.len() - 16);
    let expected = tag_for(key, nonce, aad, ct);
    let mut diff = 0u8;
    for (a, b) in expected.iter().zip(tag.iter()) {
        diff |= a ^ b;
    }
    if diff != 0 {
        return None;
    }
    let mut out = ct.to_vec();
    xor_stream(key, 1, nonce, &mut out);
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unhex(s: &str) -> Vec<u8> {
        (0..s.len() / 2).map(|i| u8::from_str_radix(&s[2 * i..2 * i + 2], 16).unwrap()).collect()
    }

    fn arr<const N: usize>(v: &[u8]) -> [u8; N] {
        let mut a = [0u8; N];
        a.copy_from_slice(v);
        a
    }

    #[test]
    fn rfc8439_poly1305_vector() {
        let key: [u8; 32] = arr(&unhex("85d6be7857556d337f4452fe42d506a80103808afb0db2fd4abff6af4149f51b"));
        let mut p = Poly1305::new(&key);
        p.update(b"Cryptographic Forum Research Group");
        assert_eq!(p.finish().to_vec(), unhex("a8061dc1305136c6c22b8baf0c0127a9"));
    }

    #[test]
    fn rfc8439_chacha20_block_vector() {
        let key: [u8; 32] = arr(&(0..32u8).collect::<Vec<_>>());
        let nonce: [u8; 12] = arr(&unhex("000000090000004a00000000"));
        let b = block(&key, 1, &nonce);
        assert_eq!(b[..32].to_vec(), unhex("10f1e7e4d13b5915500fdd1fa32071c4c7d1f4c733c068030422aa9ac3d46c4e"));
    }

    #[test]
    fn rfc8439_aead_vector() {
        let key: [u8; 32] = arr(&(0x80..0xa0u8).collect::<Vec<_>>());
        let nonce: [u8; 12] = arr(&unhex("070000004041424344454647"));
        let aad = unhex("50515253c0c1c2c3c4c5c6c7");
        let pt = b"Ladies and Gentlemen of the class of '99: If I could offer you only one tip for the future, sunscreen would be it.";
        let sealed = seal(&key, &nonce, &aad, pt);
        let (ct, tag) = sealed.split_at(sealed.len() - 16);
        assert_eq!(ct.to_vec(), unhex("d31a8d34648e60db7b86afbc53ef7ec2a4aded51296e08fea9e2b5a736ee62d63dbea45e8ca9671282fafb69da92728b1a71de0a9e060b2905d6a5b67ecd3b3692ddbd7f2d778b8c9803aee328091b58fab324e4fad675945585808b4831d7bc3ff4def08e4b7a9de576d26586cec64b6116"));
        assert_eq!(tag.to_vec(), unhex("1ae10b594f09e26a7e902ecbd0600691"));
        assert_eq!(open(&key, &nonce, &aad, &sealed).unwrap(), pt.to_vec());
    }

    /// (plaintext length, aad hex, ciphertext hex, tag hex) from an independent implementation; the plaintext is
    /// byte i = (13 i + 1) mod 256.
    const VECTORS: &[(usize, &str, &str, &str)] = &[
        (0, "00", "", "b8cf12da7b6a1b6808fa70e76bdc010b"),
        (1, "0003", "37", "23bb7fe8e67ff01397ee8d466a8b7657"),
        (15, "00", "37ad9e2d6e06c6bdf49dc407fdb81b", "0ac7138799e3363d4e5c97972ec8fb33"),
        (16, "0003", "37ad9e2d6e06c6bdf49dc407fdb81b74", "05d325cebc22e7de5d3873c39882a245"),
        (17, "000306", "37ad9e2d6e06c6bdf49dc407fdb81b748b", "d58e80bcdca202e392ae9d3f4ed0f159"),
        (63, "00030609", "37ad9e2d6e06c6bdf49dc407fdb81b748b41e6ac7382a3fc5ce2ef145989d2438b3bcbbc0154f07f811b271300507a8852516ec34b07a8a8d3a3caec07efdd", "e91fea9c831c85746d07398fcec74f13"),
        (64, "000306090c", "37ad9e2d6e06c6bdf49dc407fdb81b748b41e6ac7382a3fc5ce2ef145989d2438b3bcbbc0154f07f811b271300507a8852516ec34b07a8a8d3a3caec07efdda3", "d4c7755979bf979c403a38a1b1d0539a"),
        (65, "00", "37ad9e2d6e06c6bdf49dc407fdb81b748b41e6ac7382a3fc5ce2ef145989d2438b3bcbbc0154f07f811b271300507a8852516ec34b07a8a8d3a3caec07efdda3e1", "ef43ec5ca600217bdec12e1aaaa135d7"),
        (129, "000306090c", "37ad9e2d6e06c6bdf49dc407fdb81b748b41e6ac7382a3fc5ce2ef145989d2438b3bcbbc0154f07f811b271300507a8852516ec34b07a8a8d3a3caec07efdda3e155ab8d62fd8c33546563422d5c221234707692b7ba240cfb62f658e35a6c8d40cb2f230f8d3165038570793526dbf2ec0ac60d74d41494cbcf193e8a29d0ad77", "39df21f9e234c41de301c344c964958b"),
        (200, "00", "37ad9e2d6e06c6bdf49dc407fdb81b748b41e6ac7382a3fc5ce2ef145989d2438b3bcbbc0154f07f811b271300507a8852516ec34b07a8a8d3a3caec07efdda3e155ab8d62fd8c33546563422d5c221234707692b7ba240cfb62f658e35a6c8d40cb2f230f8d3165038570793526dbf2ec0ac60d74d41494cbcf193e8a29d0ad77df7cd354bc65a9591d1e6fdd64678669284a75950d27b02e438c451bead86120c600a3eb2a152dc9cd00c649cc663ed5c09a4234c96b2dc6966a3115e2aca0dde6e905173f28f5", "dcaa0fb7b8f1e8e774699d4cf6e76c9e"),
    ];

    #[test]
    fn matches_an_independent_implementation_at_every_block_boundary() {
        let key: [u8; 32] = arr(&(0..32).map(|i| ((i * 7 + 3) & 0xff) as u8).collect::<Vec<_>>());
        let nonce: [u8; 12] = arr(&(0..12).map(|i| ((i * 11 + 5) & 0xff) as u8).collect::<Vec<_>>());
        for (n, aad, ct, tag) in VECTORS {
            let pt: Vec<u8> = (0..*n).map(|i| ((i * 13 + 1) & 0xff) as u8).collect();
            let sealed = seal(&key, &nonce, &unhex(aad), &pt);
            let (got_ct, got_tag) = sealed.split_at(sealed.len() - 16);
            assert_eq!(got_ct.to_vec(), unhex(ct), "ciphertext, length {}", n);
            assert_eq!(got_tag.to_vec(), unhex(tag), "tag, length {}", n);
            assert_eq!(open(&key, &nonce, &unhex(aad), &sealed).unwrap(), pt, "round trip, length {}", n);
        }
    }

    #[test]
    fn any_change_is_rejected() {
        let key = [9u8; 32];
        let nonce = [4u8; 12];
        let sealed = seal(&key, &nonce, b"header", b"attack at dawn");
        assert!(open(&key, &nonce, b"header", &sealed).is_some());
        for i in 0..sealed.len() {
            let mut bad = sealed.clone();
            bad[i] ^= 1;
            assert!(open(&key, &nonce, b"header", &bad).is_none(), "flip at byte {}", i);
        }
        assert!(open(&key, &nonce, b"other", &sealed).is_none(), "wrong aad");
        assert!(open(&[8u8; 32], &nonce, b"header", &sealed).is_none(), "wrong key");
        assert!(open(&key, &[5u8; 12], b"header", &sealed).is_none(), "wrong nonce");
        assert!(open(&key, &nonce, b"header", &sealed[..sealed.len() - 1]).is_none(), "truncated");
        assert!(open(&key, &nonce, b"header", &[]).is_none(), "empty");
    }
}
