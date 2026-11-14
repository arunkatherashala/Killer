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

    // ---- RFC 8439 vectors (section 2.3.2, 2.4.2, 2.5.2, appendix A.1, A.3, A.5) ----

    #[test]
    fn rfc8439_2_3_2_full_block() {
        let key: [u8; 32] = arr(&(0..32u8).collect::<Vec<_>>());
        let nonce: [u8; 12] = arr(&unhex("000000090000004a00000000"));
        assert_eq!(block(&key, 1, &nonce).to_vec(), unhex("10f1e7e4d13b5915500fdd1fa32071c4c7d1f4c733c068030422aa9ac3d46c4ed2826446079faa0914c2d705d98b02a2b5129cd1de164eb9cbd083e8a2503c4e"));
    }

    #[test]
    fn rfc8439_a1_zero_key_blocks() {
        let (key, nonce) = ([0u8; 32], [0u8; 12]);
        assert_eq!(block(&key, 0, &nonce).to_vec(), unhex("76b8e0ada0f13d90405d6ae55386bd28bdd219b8a08ded1aa836efcc8b770dc7da41597c5157488d7724e03fb8d84a376a43b8f41518a11cc387b669b2ee6586"));
        assert_eq!(block(&key, 1, &nonce).to_vec(), unhex("9f07e7be5551387a98ba977c732d080dcb0f29a048e3656912c6533e32ee7aed29b721769ce64e43d57133b074d839d531ed1f28510afb45ace10a1f4b794d6f"));
    }

    #[test]
    fn rfc8439_2_4_2_stream_encryption_and_decryption() {
        let key: [u8; 32] = arr(&(0..32u8).collect::<Vec<_>>());
        let nonce: [u8; 12] = arr(&unhex("000000000000004a00000000"));
        let pt = b"Ladies and Gentlemen of the class of '99: If I could offer you only one tip for the future, sunscreen would be it.".to_vec();
        let want = unhex("6e2e359a2568f98041ba0728dd0d6981e97e7aec1d4360c20a27afccfd9fae0bf91b65c5524733ab8f593dabcd62b3571639d624e65152ab8f530c359f0861d807ca0dbf500d6a6156a38e088a22b65e52bc514d16ccf806818ce91ab77937365af90bbf74a35be6b40b8eedf2785e42874d");
        let mut buf = pt.clone();
        xor_stream(&key, 1, &nonce, &mut buf);
        assert_eq!(buf, want);
        xor_stream(&key, 1, &nonce, &mut buf);
        assert_eq!(buf, pt, "XOR with the same stream decrypts");
    }

    fn poly(key: &str, msg: &[u8]) -> Vec<u8> {
        let mut p = Poly1305::new(&arr(&unhex(key)));
        p.update(msg);
        p.finish().to_vec()
    }

    #[test]
    fn rfc8439_a3_poly1305_vectors() {
        let z = "00000000000000000000000000000000";
        // 1: all zero key and message
        assert_eq!(poly(&format!("{z}{z}"), &[0u8; 64]), vec![0u8; 16]);
        let ietf = br#"Any submission to the IETF intended by the Contributor for publication as all or part of an IETF Internet-Draft or RFC and any statement made within the context of an IETF activity is considered an "IETF Contribution". Such statements include oral statements in IETF sessions, as well as written and electronic communications made at any time or place, which are addressed to"#;
        // 2: r = 0, so the tag is s; 3: s = 0
        assert_eq!(poly(&format!("{z}36e5f6b5c5e06070f0efca96227a863e"), ietf), unhex("36e5f6b5c5e06070f0efca96227a863e"));
        assert_eq!(poly(&format!("36e5f6b5c5e06070f0efca96227a863e{z}"), ietf), unhex("f3477e7cd95417af89a6b8794c310cf0"));
        // 4: the Jabberwocky text, 127 bytes (not a multiple of 16)
        let jab = b"'Twas brillig, and the slithy toves\nDid gyre and gimble in the wabe:\nAll mimsy were the borogoves,\nAnd the mome raths outgrabe.";
        assert_eq!(jab.len(), 127);
        assert_eq!(poly("1c9240a5eb55d38af333888604f6b5f0473917c1402b80099dca5cbc207075c0", jab), unhex("4541669a7eaaee61e708dc7cbcc5eb62"));
        // 5, 6, 9: carries and the wrap at 2^130 - 5
        assert_eq!(poly(&format!("02{}{z}", "00".repeat(15)), &[0xff; 16]), unhex("03000000000000000000000000000000"));
        assert_eq!(poly(&format!("02{}{}", "00".repeat(15), "ff".repeat(16)), &unhex("02000000000000000000000000000000")), unhex("03000000000000000000000000000000"));
        assert_eq!(poly(&format!("02{}{z}", "00".repeat(15)), &unhex("fdffffffffffffffffffffffffffffff")), unhex("faffffffffffffffffffffffffffffff"));
        // 7 and 8: sums that land exactly on, and just below, a multiple of 2^130 - 5
        let k78 = format!("01{}{z}", "00".repeat(15));
        let m7 = [vec![0xffu8; 16], unhex("f0ffffffffffffffffffffffffffffff"), unhex("11000000000000000000000000000000")].concat();
        assert_eq!(poly(&k78, &m7), unhex("05000000000000000000000000000000"));
        let m8 = [vec![0xffu8; 16], unhex("fbfefefefefefefefefefefefefefefe"), unhex("01010101010101010101010101010101")].concat();
        assert_eq!(poly(&k78, &m8), vec![0u8; 16]);
    }

    #[test]
    fn poly1305_split_updates_equal_one_update() {
        let key = "1c9240a5eb55d38af333888604f6b5f0473917c1402b80099dca5cbc207075c0";
        let msg: Vec<u8> = (0..100u32).map(|i| (i * 7 + 1) as u8).collect();
        let whole = poly(key, &msg);
        for cut in [0, 1, 15, 16, 17, 31, 32, 33, 99, 100] {
            let mut p = Poly1305::new(&arr(&unhex(key)));
            p.update(&msg[..cut]);
            p.update(&msg[cut..]);
            assert_eq!(p.finish().to_vec(), whole, "cut at {}", cut);
        }
    }

    #[test]
    fn rfc8439_a5_decryption_vector() {
        let key: [u8; 32] = arr(&unhex("1c9240a5eb55d38af333888604f6b5f0473917c1402b80099dca5cbc207075c0"));
        let nonce: [u8; 12] = arr(&unhex("000000000102030405060708"));
        let aad = unhex("f33388860000000000004e91");
        let pt = "Internet-Drafts are draft documents valid for a maximum of six months and may be updated, replaced, or obsoleted by other documents at any time. It is inappropriate to use Internet-Drafts as reference material or to cite them other than as /\u{201c}work in progress./\u{201d}";
        let sealed = seal(&key, &nonce, &aad, pt.as_bytes());
        assert_eq!(sealed.len(), 265 + 16);
        assert_eq!(sealed[..32].to_vec(), unhex("64a0861575861af460f062c79be643bd5e805cfd345cf389f108670ac76c8cb2"));
        assert_eq!(sealed[265..].to_vec(), unhex("eead9d67890cbb22392336fea1851f38"));
        assert_eq!(open(&key, &nonce, &aad, &sealed).unwrap(), pt.as_bytes().to_vec());
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
