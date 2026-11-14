//! Published test vectors for every cryptographic primitive in the crate, checked against both the Rust functions
//! and the Killer builtins (run through the `killer` binary), plus a cross-check against an independent Python
//! implementation (tests/data/crypto_reference.py; skipped, with a message, when Python is not installed).
//!
//! Standards covered: FIPS 180-4 (SHA-1, SHA-256, SHA-512), RFC 4231 (HMAC-SHA256), RFC 8018 / RFC 7914 section 11 /
//! the usual PBKDF2-HMAC-SHA256 vectors, RFC 8439 (ChaCha20, Poly1305, AEAD, appendix A), RFC 4648 (base64),
//! RFC 6455 (WebSocket accept key). See docs/CRYPTO.md for the full list and the limits.
use killer_native::{aead, android_security, cryptography_solver, encryption, ghost_vm, net_sock, pkg_files};
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

// ---------------------------------------------------------------------------------------------------------------
// helpers

fn unhex(s: &str) -> Vec<u8> {
    assert!(s.len() % 2 == 0, "odd hex");
    (0..s.len() / 2).map(|i| u8::from_str_radix(&s[2 * i..2 * i + 2], 16).unwrap()).collect()
}

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{:02x}", x)).collect()
}

static NEXT: AtomicUsize = AtomicUsize::new(0);

/// Run a Killer program; returns (success, stdout lines, stderr).
fn killer(src: &str) -> (bool, Vec<String>, String) {
    let dir: PathBuf = std::env::temp_dir().join(format!("killer_crypto_{}_{}", std::process::id(), NEXT.fetch_add(1, Ordering::SeqCst)));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("t.killer");
    std::fs::write(&file, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_killer")).arg("run").arg(&file).current_dir(&dir).output().unwrap();
    let _ = std::fs::remove_dir_all(&dir);
    let s = |b: &[u8]| String::from_utf8_lossy(b).replace("\r\n", "\n");
    let lines = s(&out.stdout).trim_end_matches('\n').split('\n').map(|l| l.to_string()).collect();
    (out.status.success(), lines, s(&out.stderr))
}

fn killer_ok(src: &str) -> Vec<String> {
    let (ok, lines, err) = killer(src);
    assert!(ok, "killer script failed: {}\n{}", err, src);
    lines
}

fn python() -> Option<String> {
    for name in ["python", "python3", "py"] {
        if let Ok(o) = Command::new(name).arg("--version").output() {
            if o.status.success() {
                return Some(name.to_string());
            }
        }
    }
    None
}

fn reference(args: &[&str]) -> Option<String> {
    let py = python()?;
    let script = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/data/crypto_reference.py");
    let out = Command::new(py).arg(script).args(args).output().unwrap();
    assert!(out.status.success(), "python reference failed: {}", String::from_utf8_lossy(&out.stderr));
    Some(String::from_utf8_lossy(&out.stdout).replace("\r\n", "\n"))
}

// ---------------------------------------------------------------------------------------------------------------
// SHA-256, SHA-512, SHA-1

const ABC: &[u8] = b"abc";
const M448: &[u8] = b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq";
const M896: &[u8] = b"abcdefghbcdefghicdefghijdefghijkefghijklfghijklmghijklmnhijklmnoijklmnopjklmnopqklmnopqrlmnopqrsmnopqrstnopqrstu";

const SHA256_ABC: &str = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";
const SHA256_EMPTY: &str = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";
const SHA256_448: &str = "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1";
const SHA256_896: &str = "cf5b16a778af8380036ce59e7b0492370b249b11e8f07a51afac45037afee9d1";
const SHA256_1M: &str = "cdc76e5c9914fb9281a1c7e284d73e67f1809a48a497200e046d39ccc7112cd0";

/// (number of 'a' bytes, SHA-256, SHA-512): every padding boundary of a 64-byte and a 128-byte block.
const A_RUNS: &[(usize, &str, &str)] = &[
    (55, "9f4390f8d30c2dd92ec9f095b65e2b9ae9b0a925a5258e241c9f1e910f734318", "b0220c772cbf6c1822e2cb38a437d0e1d58772417a4bbb21c961364f8b6143e05aa6316dca8d1d7b19e16448419076395f6086cb55101fbd6d5497b148e1745f"),
    (56, "b35439a4ac6f0948b6d6f9e3c6af0f5f590ce20f1bde7090ef7970686ec6738a", "962b64aae357d2a4fee3ded8b539bdc9d325081822b0bfc55583133aab44f18bafe11d72a7ae16c79ce2ba620ae2242d5144809161945f1367f41b3972e26e04"),
    (63, "7d3e74a05d7db15bce4ad9ec0658ea98e3f06eeecf16b4c6fff2da457ddc2f34", "c1b0f5c6d3b03dfe4a2602e67242f54e344090b66e01100a469b129f583f016c7e27dddeaa438393dcc7ec54b0b57c9ba7af007f9b56db5f6fb677d972a31362"),
    (64, "ffe054fe7ae0cb6dc65c3af9b61d5209f439851db43d0ba5997337df154668eb", "01d35c10c6c38c2dcf48f7eebb3235fb5ad74a65ec4cd016e2354c637a8fb49b695ef3c1d6f7ae4cd74d78cc9c9bcac9d4f23a73019998a7f73038a5c9b2dbde"),
    (65, "635361c48bb9eab14198e76ea8ab7f1a41685d6ad62aa9146d301d4f17eb0ae0", "b83086cd8494e55708ad7ecd82dfb4bca1bda61ecbb7caf0c68967902e709345e5d8305eb7ac0d588afc6cbb75161aa9c8c7e0ea986bd833dafe5e1ccd37345a"),
    (111, "6374f73208854473827f6f6a3f43b1f53eaa3b82c21c1a6d69a2110b2a79baad", "fa9121c7b32b9e01733d034cfc78cbf67f926c7ed83e82200ef86818196921760b4beff48404df811b953828274461673c68d04e297b0eb7b2b4d60fc6b566a2"),
    (112, "f54353008a2553262ecdc4a34749563ba0950e8b0fc8652780b0a614b99683c1", "c01d080efd492776a1c43bd23dd99d0a2e626d481e16782e75d54c2503b5dc32bd05f0f1ba33e568b88fd2d970929b719ecbb152f58f130a407c8830604b70ca"),
    (119, "31eba51c313a5c08226adf18d4a359cfdfd8d2e816b13f4af952f7ea6584dcfb", "130396a75cb483f2eee8c56d8a668bb3d2641f5243212c0bee2bd33da096ad9eb8179fe18f9eaacf76e09fae9de4c3f14ba13341e345be05bf76c182cc3468cb"),
    (120, "2f3d335432c70b580af0e8e1b3674a7c020d683aa5f73aaaedfdc55af904c21c", "f241de612b01aa2fa3cf01531d2a8e5e17fc761dfd48a704a834a47f57d6eade7804ecc39be42fdef16ec6adeaf7c01c2fd0c4cc97d3860907cfa4a3b36d0c05"),
    (127, "c57e9278af78fa3cab38667bef4ce29d783787a2f731d4e12200270f0c32320a", "828613968b501dc00a97e08c73b118aa8876c26b8aac93df128502ab360f91bab50a51e088769a5c1eff4782ace147dce3642554199876374291f5d921629502"),
    (128, "6836cf13bac400e9105071cd6af47084dfacad4e5e302c94bfed24e013afb73e", "b73d1929aa615934e61a871596b3f3b33359f42b8175602e89f7e06e5f658a243667807ed300314b95cacdd579f3e33abdfbe351909519a846d465c59582f321"),
    (129, "c12cb024a2e5551cca0e08fce8f1c5e314555cc3fef6329ee994a3db752166ae", "4f681e0bd53cda4b5a2041cc8a06f2eabde44fb16c951fbd5b87702f07aeab611565b19c47fde30587177ebb852e3971bbd8d3fd30da18d71037dfbd98420429"),
];

const SHA512_ABC: &str = "ddaf35a193617abacc417349ae20413112e6fa4e89a97ea20a9eeee64b55d39a2192992a274fc1a836ba3c23a3feebbd454d4423643ce80e2a9ac94fa54ca49f";
const SHA512_EMPTY: &str = "cf83e1357eefb8bdf1542850d66d8007d620e4050b5715dc83f4a921d36ce9ce47d0d13c5d85f2b0ff8318d2877eec2f63b931bd47417a81a538327af927da3e";
const SHA512_896: &str = "8e959b75dae313da8cf4f72814fc143f8f7779c6eb9f7fa17299aeadb6889018501d289e4900f7e4331b99dec4b5433ac7d329eeb6dd26545e96e55b874be909";
const SHA512_1M: &str = "e718483d0ce769644e2e42c7bc15b4638e1f98b13b2044285632a803afa973ebde0ff244877ea60a4cb0432ce577c31beb009c5c2c49aa2e4eadb217ad8cc09b";

#[test]
fn sha256_fips_180_in_every_implementation() {
    let million = vec![b'a'; 1_000_000];
    let mut cases: Vec<(Vec<u8>, String)> = vec![
        (ABC.to_vec(), SHA256_ABC.into()),
        (vec![], SHA256_EMPTY.into()),
        (M448.to_vec(), SHA256_448.into()),
        (M896.to_vec(), SHA256_896.into()),
        (million, SHA256_1M.into()),
    ];
    for (n, h256, _) in A_RUNS {
        cases.push((vec![b'a'; *n], h256.to_string()));
    }
    for (msg, want) in &cases {
        let label = format!("length {}", msg.len());
        assert_eq!(hex(&encryption::sha256(msg)), *want, "encryption::sha256, {}", label);
        assert_eq!(hex(&android_security::sha256(msg)), *want, "android_security::sha256, {}", label);
        assert_eq!(hex(&ghost_vm::sha256_digest(msg)), *want, "ghost_vm::sha256_digest, {}", label);
        assert_eq!(pkg_files::sha256_hex(msg), *want, "pkg_files::sha256_hex, {}", label);
        assert_eq!(hex(&cryptography_solver::sha256(msg)), *want, "cryptography_solver::sha256, {}", label);
        assert_eq!(
            hex(&encryption::HashingEngine::new(encryption::HashingAlgorithm::SHA256).hash(msg).unwrap()),
            *want,
            "HashingEngine, {}",
            label
        );
    }
}

#[test]
fn sha512_fips_180() {
    let mut cases: Vec<(Vec<u8>, String)> = vec![
        (ABC.to_vec(), SHA512_ABC.into()),
        (vec![], SHA512_EMPTY.into()),
        (M896.to_vec(), SHA512_896.into()),
        (vec![b'a'; 1_000_000], SHA512_1M.into()),
    ];
    for (n, _, h512) in A_RUNS {
        cases.push((vec![b'a'; *n], h512.to_string()));
    }
    for (msg, want) in &cases {
        assert_eq!(hex(&encryption::sha512(msg)), *want, "sha512, length {}", msg.len());
        let via_engine = encryption::HashingEngine::new(encryption::HashingAlgorithm::SHA512).hash(msg).unwrap();
        assert_eq!(hex(&via_engine), *want);
        assert!(encryption::HashingEngine::new(encryption::HashingAlgorithm::SHA512).verify(msg, &via_engine).unwrap());
    }
    // SHA-3 is not implemented and says so
    assert!(encryption::HashingEngine::new(encryption::HashingAlgorithm::SHA3_256).hash(b"x").is_err());
}

#[test]
fn sha1_fips_180_and_websocket_accept_key() {
    let h = |d: &[u8]| hex(&net_sock::sha1(d));
    assert_eq!(h(b"abc"), "a9993e364706816aba3e25717850c26c9cd0d89d");
    assert_eq!(h(b""), "da39a3ee5e6b4b0d3255bfef95601890afd80709");
    assert_eq!(h(M448), "84983e441c3bd26ebaae4aa1f95129e5e54670f1");
    assert_eq!(h(M896), "a49b2446a02c645bf419f995b67091253a04a259");
    assert_eq!(h(&vec![b'a'; 1_000_000]), "34aa973cd4c4daa4f61eeb2bdbad27316534016f");
    // RFC 6455 section 1.3
    assert_eq!(net_sock::accept_key("dGhlIHNhbXBsZSBub25jZQ=="), "s3pPLMBiTxaQ9kYGzzhZRbK+xOo=");
}

#[test]
fn killer_sha256_builtins() {
    // sha256 and secure_hash accept strings (and bytes for sha256); sha256_bytes returns raw bytes
    let out = killer_ok(&format!(
        r#"fn rep(b, n) {{
  return bytes_fill(bytes_new(n), b)
}}
println(sha256("abc"))
println(sha256(""))
println(sha256("{m448}"))
println(sha256("{m896}"))
println(sha256(rep(97, 1000000)))
println(sha256("a" * 1000000))
println(secure_hash("abc"))
println(secure_hash("{m448}"))
println(secure_hash("a" * 1000000))
println(sha256(rep(97, 55)))
println(sha256(rep(97, 56)))
println(sha256(rep(97, 64)))
println(sha256(rep(97, 65)))
println(sha256(rep(97, 128)))
println(base64_encode(sha256_bytes("abc")))
println(len(sha256_bytes("abc")))
"#,
        m448 = std::str::from_utf8(M448).unwrap(),
        m896 = std::str::from_utf8(M896).unwrap(),
    ));
    let a = |n: usize| A_RUNS.iter().find(|r| r.0 == n).unwrap().1;
    let want = vec![
        SHA256_ABC, SHA256_EMPTY, SHA256_448, SHA256_896, SHA256_1M, SHA256_1M, SHA256_ABC, SHA256_448, SHA256_1M,
        a(55), a(56), a(64), a(65), a(128),
    ];
    assert_eq!(out[..want.len()], want.iter().map(|s| s.to_string()).collect::<Vec<_>>()[..]);
    // base64 of the 32 raw digest bytes of "abc"
    assert_eq!(out[want.len()], "ungWv48Bz+pBQUDeXa4iI7ADYaOWF3qctBD/YfIAFa0=");
    assert_eq!(out[want.len() + 1], "32");
}

// ---------------------------------------------------------------------------------------------------------------
// HMAC-SHA256 (RFC 4231)

/// (key, data, expected HMAC-SHA256 as hex); case 5 is the 128-bit truncation the RFC lists (compared as a prefix).
fn rfc4231() -> Vec<(Vec<u8>, Vec<u8>, &'static str)> {
    vec![
        (vec![0x0b; 20], b"Hi There".to_vec(), "b0344c61d8db38535ca8afceaf0bf12b881dc200c9833da726e9376c2e32cff7"),
        (b"Jefe".to_vec(), b"what do ya want for nothing?".to_vec(), "5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843"),
        (vec![0xaa; 20], vec![0xdd; 50], "773ea91e36800e46854db8ebd09181a72959098b3ef8c122d9635514ced565fe"),
        ((1..=25).collect(), vec![0xcd; 50], "82558a389a443c0ea4cc819899f2083a85f0faa3e578f8077a2e3ff46729665b"),
        (vec![0x0c; 20], b"Test With Truncation".to_vec(), "a3b6167473100ee06e0c796c2955552b"),
        (vec![0xaa; 131], b"Test Using Larger Than Block-Size Key - Hash Key First".to_vec(), "60e431591ee0b67f0d8a26aacbf5b77f8e0bc6213728c5140546040f0ee37f54"),
        (
            vec![0xaa; 131],
            b"This is a test using a larger than block-size key and a larger than block-size data. The key needs to be hashed before being used by the HMAC algorithm.".to_vec(),
            "9b09ffa71b942fcb27635fbcd5b0e944bfdc63644f0713938a7f51535c3a35e2",
        ),
    ]
}

#[test]
fn hmac_sha256_rfc_4231_cases_1_to_7() {
    for (i, (key, data, want)) in rfc4231().iter().enumerate() {
        let got = hex(&encryption::hmac_sha256(key, data));
        assert!(got.starts_with(want), "RFC 4231 case {}: got {}", i + 1, got);
    }
    // key exactly 64 bytes (block size), 65 bytes (hashed first), and empty key/data, against Python's hmac module
    assert_eq!(hex(&encryption::hmac_sha256(b"", b"")), "b613679a0814d9ec772f95d778c35fc5ff1697c493715653c6c712144292c5ad");
}

#[test]
fn killer_hmac_sha256_rfc_4231() {
    let out = killer_ok(
        r#"fn rep(b, n) {
  return bytes_fill(bytes_new(n), b)
}
k4 = bytes_new(25)
i = 0
while i < 25 {
  k4 = bytes_set(k4, i, i + 1)
  i += 1
}
println(hmac_sha256(rep(11, 20), "Hi There"))
println(hmac_sha256("Jefe", "what do ya want for nothing?"))
println(hmac_sha256(rep(170, 20), rep(221, 50)))
println(hmac_sha256(k4, rep(205, 50)))
println(hmac_sha256(rep(12, 20), "Test With Truncation"))
println(hmac_sha256(rep(170, 131), "Test Using Larger Than Block-Size Key - Hash Key First"))
println(hmac_sha256(rep(170, 131), "This is a test using a larger than block-size key and a larger than block-size data. The key needs to be hashed before being used by the HMAC algorithm."))
println(hmac_sha256("", ""))
"#,
    );
    for (i, (_, _, want)) in rfc4231().iter().enumerate() {
        assert!(out[i].starts_with(want), "RFC 4231 case {} via killer: got {}", i + 1, out[i]);
    }
    assert_eq!(out[7], "b613679a0814d9ec772f95d778c35fc5ff1697c493715653c6c712144292c5ad");
}

// ---------------------------------------------------------------------------------------------------------------
// PBKDF2-HMAC-SHA256

/// (password, salt, iterations, derived key length, expected hex)
fn pbkdf2_vectors() -> Vec<(&'static [u8], &'static [u8], u32, usize, &'static str)> {
    vec![
        (b"password", b"salt", 1, 32, "120fb6cffcf8b32c43e7225256c4f837a86548c92ccc35480805987cb70be17b"),
        (b"password", b"salt", 2, 32, "ae4d0c95af6b46d32d0adff928f06dd02a303f8ef3c251dfd6e2d85a95474c43"),
        (b"password", b"salt", 4096, 32, "c5e478d59288c841aa530db6845c4c8d962893a001ce4e11a4963873aa98134a"),
        (b"passwordPASSWORDpassword", b"saltSALTsaltSALTsaltSALTsaltSALTsalt", 4096, 40, "348c89dbcbd32b2f32d814b8116e84cf2b17347ebc1800181c4e2a1fb8dd53e1c635518c7dac47e9"),
        (b"pass\0word", b"sa\0lt", 4096, 16, "89b69d0516f829893c696226650a8687"),
        // RFC 7914 section 11
        (b"passwd", b"salt", 1, 64, "55ac046e56e3089fec1691c22544b605f94185216dde0465e68b9d57c20dacbc49ca9cccf179b645991664b39d77ef317c71b845b1e30bd509112041d3a19783"),
        (b"Password", b"NaCl", 80000, 64, "4ddcd8f60b98be21830cee5ef22701f9641a4418d04c0414aeff08876b34ab56a1d425a1225833549adb841b51c9b3176a272bdebba1d078478f62b397f33c8d"),
        // empty password / empty salt / a 100-byte password (longer than the HMAC block) with a 70-byte output (3 blocks)
        (b"", b"salt", 3, 32, "5ddf839afa2d5fb4be56e1a0f48917617559bef61ec122bfca1c7f75ac8f401d"),
        (b"password", b"", 3, 32, "02048ce01cff51053aa4002a47f6ce3d1e123a76a6b178a8790d8b7c09063606"),
    ]
}

#[test]
fn pbkdf2_hmac_sha256_known_answers() {
    for (pw, salt, iters, len, want) in pbkdf2_vectors() {
        let got = encryption::pbkdf2_hmac_sha256(pw, salt, iters, len);
        assert_eq!(hex(&got), want, "pbkdf2_hmac_sha256 {:?}/{:?}/{}/{}", String::from_utf8_lossy(pw), String::from_utf8_lossy(salt), iters, len);
        // the single-block function used by secure_encrypt must agree on the first 32 bytes
        if iters <= 4096 {
            let first = android_security::pbkdf2_sha256(pw, salt, iters);
            assert!(hex(&first).starts_with(&want[..want.len().min(64)]), "android pbkdf2_sha256, {} iterations", iters);
        }
    }
    let long_pw = vec![b'k'; 100];
    assert_eq!(
        hex(&encryption::pbkdf2_hmac_sha256(&long_pw, b"salt", 5, 70)),
        "0c1fe31f745bff40c1749c4e65d512fa5620b8f7aa2b199d19f09d00bb03c041fdabf95ea1c720f4eedfed335ce760818304f678f2e3f95eca4553f0d8bc737ab99d6a83eae7"
    );
}

#[test]
fn pbkdf2_output_depends_on_every_input() {
    let base = encryption::pbkdf2_hmac_sha256(b"password", b"salt", 10, 32);
    assert_eq!(base, encryption::pbkdf2_hmac_sha256(b"password", b"salt", 10, 32), "deterministic");
    assert_ne!(base, encryption::pbkdf2_hmac_sha256(b"password", b"salt2", 10, 32), "a different salt gives a different key");
    assert_ne!(base, encryption::pbkdf2_hmac_sha256(b"password", b"Salt", 10, 32), "salt is case sensitive");
    assert_ne!(base, encryption::pbkdf2_hmac_sha256(b"passwore", b"salt", 10, 32), "a different password gives a different key");
    assert_ne!(base, encryption::pbkdf2_hmac_sha256(b"password", b"salt", 11, 32), "a different iteration count gives a different key");
    // a shorter output is a prefix of a longer one
    assert_eq!(base[..20], encryption::pbkdf2_hmac_sha256(b"password", b"salt", 10, 20)[..]);
    // zero iterations is treated as one (it does not skip the hash, and does not panic)
    assert_eq!(encryption::pbkdf2_hmac_sha256(b"p", b"s", 0, 32), encryption::pbkdf2_hmac_sha256(b"p", b"s", 1, 32));
    assert!(encryption::pbkdf2_hmac_sha256(b"p", b"s", 1, 0).is_empty());
    // password hashes use a fresh random salt each time and verify only the right password
    let hasher = encryption::PasswordHasher::new(encryption::KeyDerivationFunction::PBKDF2);
    let (h1, h2) = (hasher.hash_password("hunter2").unwrap(), hasher.hash_password("hunter2").unwrap());
    assert_ne!(h1.salt, h2.salt);
    assert_ne!(h1.hash, h2.hash);
    assert_eq!(h1.salt.len(), 16);
    assert!(hasher.verify_password("hunter2", &h1).unwrap());
    assert!(hasher.verify_password("hunter2", &h2).unwrap());
    assert!(!hasher.verify_password("hunter3", &h1).unwrap());
    assert!(!hasher.verify_password("", &h1).unwrap());
    let mut tampered = h1.clone();
    tampered.hash[31] ^= 1;
    assert!(!hasher.verify_password("hunter2", &tampered).unwrap());
    // and the key-derivation functions that are not implemented refuse instead of returning something weak
    for kdf in [encryption::KeyDerivationFunction::Argon2id, encryption::KeyDerivationFunction::HKDF, encryption::KeyDerivationFunction::Scrypt] {
        assert!(encryption::PasswordHasher::new(kdf).hash_password("x").is_err(), "{} must not pretend to work", kdf);
    }
}

// ---------------------------------------------------------------------------------------------------------------
// ChaCha20-Poly1305 (RFC 8439)

const RFC_PT: &[u8] = b"Ladies and Gentlemen of the class of '99: If I could offer you only one tip for the future, sunscreen would be it.";

fn key_80() -> [u8; 32] {
    let mut k = [0u8; 32];
    for (i, b) in k.iter_mut().enumerate() {
        *b = 0x80 + i as u8;
    }
    k
}

#[test]
fn rfc8439_section_2_8_2_aead() {
    let nonce: [u8; 12] = unhex("070000004041424344454647").try_into().unwrap();
    let aad = unhex("50515253c0c1c2c3c4c5c6c7");
    let sealed = aead::seal(&key_80(), &nonce, &aad, RFC_PT);
    assert_eq!(
        hex(&sealed[..RFC_PT.len()]),
        "d31a8d34648e60db7b86afbc53ef7ec2a4aded51296e08fea9e2b5a736ee62d63dbea45e8ca9671282fafb69da92728b1a71de0a9e060b2905d6a5b67ecd3b3692ddbd7f2d778b8c9803aee328091b58fab324e4fad675945585808b4831d7bc3ff4def08e4b7a9de576d26586cec64b6116"
    );
    assert_eq!(hex(&sealed[RFC_PT.len()..]), "1ae10b594f09e26a7e902ecbd0600691");
    assert_eq!(aead::open(&key_80(), &nonce, &aad, &sealed).unwrap(), RFC_PT);
}

#[test]
fn rfc8439_section_2_4_2_keystream_encryption_through_the_aead() {
    // seal() encrypts with the keystream starting at block counter 1, exactly the section 2.4.2 setting
    let key: Vec<u8> = (0..32u8).collect();
    let nonce: [u8; 12] = unhex("000000000000004a00000000").try_into().unwrap();
    let sealed = aead::seal(&key.try_into().unwrap(), &nonce, &[], RFC_PT);
    assert_eq!(
        hex(&sealed[..RFC_PT.len()]),
        "6e2e359a2568f98041ba0728dd0d6981e97e7aec1d4360c20a27afccfd9fae0bf91b65c5524733ab8f593dabcd62b3571639d624e65152ab8f530c359f0861d807ca0dbf500d6a6156a38e088a22b65e52bc514d16ccf806818ce91ab77937365af90bbf74a35be6b40b8eedf2785e42874d"
    );
}

/// RFC 8439 appendix A.5: (key, nonce, aad, plain text, ciphertext || tag). The plain text is
/// Internet-Draft boilerplate with curly quotes (UTF-8 e2 80 9c / e2 80 9d) and slashes, 265 bytes.
fn a5() -> ([u8; 32], [u8; 12], Vec<u8>, Vec<u8>) {
    let key: [u8; 32] = unhex("1c9240a5eb55d38af333888604f6b5f0473917c1402b80099dca5cbc207075c0").try_into().unwrap();
    let nonce: [u8; 12] = unhex("000000000102030405060708").try_into().unwrap();
    let aad = unhex("f33388860000000000004e91");
    let pt = "Internet-Drafts are draft documents valid for a maximum of six months and may be updated, replaced, or obsoleted by other documents at any time. It is inappropriate to use Internet-Drafts as reference material or to cite them other than as /\u{201c}work in progress./\u{201d}";
    (key, nonce, aad, pt.as_bytes().to_vec())
}

#[test]
fn rfc8439_appendix_a5() {
    let (key, nonce, aad, pt) = a5();
    assert_eq!(pt.len(), 265);
    let sealed = aead::seal(&key, &nonce, &aad, &pt);
    assert_eq!(sealed.len(), 265 + 16);
    assert_eq!(hex(&sealed[..32]), "64a0861575861af460f062c79be643bd5e805cfd345cf389f108670ac76c8cb2");
    assert_eq!(hex(&sealed[265..]), "eead9d67890cbb22392336fea1851f38", "tag from RFC 8439 A.5");
    assert_eq!(aead::open(&key, &nonce, &aad, &sealed).expect("must decrypt"), pt);
}

#[test]
fn aead_rejects_every_kind_of_tampering() {
    let (key, nonce, aad, pt) = a5();
    let sealed = aead::seal(&key, &nonce, &aad, &pt);
    // every single-bit flip, in every byte of ciphertext and tag
    for i in 0..sealed.len() {
        for bit in [0x01u8, 0x80] {
            let mut bad = sealed.clone();
            bad[i] ^= bit;
            assert!(aead::open(&key, &nonce, &aad, &bad).is_none(), "flipped bit {:#x} of byte {} accepted", bit, i);
        }
    }
    // wrong tag (all zero, all ones), wrong key, wrong nonce, wrong or missing aad
    let n = sealed.len();
    for fake in [[0u8; 16], [0xff; 16]] {
        let mut bad = sealed.clone();
        bad[n - 16..].copy_from_slice(&fake);
        assert!(aead::open(&key, &nonce, &aad, &bad).is_none());
    }
    let mut k2 = key;
    k2[0] ^= 1;
    assert!(aead::open(&k2, &nonce, &aad, &sealed).is_none(), "wrong key");
    let mut n2 = nonce;
    n2[11] ^= 1;
    assert!(aead::open(&key, &n2, &aad, &sealed).is_none(), "wrong nonce");
    assert!(aead::open(&key, &nonce, &aad[..11], &sealed).is_none(), "truncated aad");
    assert!(aead::open(&key, &nonce, &[], &sealed).is_none(), "missing aad");
    // truncated, extended, shorter than a tag, empty
    assert!(aead::open(&key, &nonce, &aad, &sealed[..n - 1]).is_none());
    let mut longer = sealed.clone();
    longer.push(0);
    assert!(aead::open(&key, &nonce, &aad, &longer).is_none());
    assert!(aead::open(&key, &nonce, &aad, &sealed[..15]).is_none());
    assert!(aead::open(&key, &nonce, &aad, &[]).is_none());
    // swapping ciphertext blocks is detected too
    let mut swapped = sealed.clone();
    let (a, b) = (swapped[0..16].to_vec(), swapped[16..32].to_vec());
    swapped[0..16].copy_from_slice(&b);
    swapped[16..32].copy_from_slice(&a);
    assert!(aead::open(&key, &nonce, &aad, &swapped).is_none());
    // an empty message still authenticates: 16-byte output, any change rejected
    let empty = aead::seal(&key, &nonce, &aad, b"");
    assert_eq!(empty.len(), 16);
    assert_eq!(aead::open(&key, &nonce, &aad, &empty).unwrap(), Vec::<u8>::new());
    let mut bad = empty.clone();
    bad[0] ^= 1;
    assert!(aead::open(&key, &nonce, &aad, &bad).is_none());
}

#[test]
fn nonce_reuse_is_the_callers_problem_and_the_library_never_reuses() {
    // The AEAD primitive is deterministic: the same (key, nonce) twice leaks the XOR of the plaintexts. That is
    // inherent to ChaCha20-Poly1305, so the callers must (and do) draw a fresh random nonce per message.
    let (key, nonce) = ([3u8; 32], [4u8; 12]);
    let (c1, c2) = (aead::seal(&key, &nonce, &[], b"attack at dawn!!"), aead::seal(&key, &nonce, &[], b"retreat at dusk!"));
    let x: Vec<u8> = c1[..16].iter().zip(&c2[..16]).map(|(a, b)| a ^ b).collect();
    let px: Vec<u8> = b"attack at dawn!!".iter().zip(b"retreat at dusk!").map(|(a, b)| a ^ b).collect();
    assert_eq!(x, px, "demonstrates why a nonce must never be reused with one key");
    // the engine draws 12 fresh bytes from the OS generator for every message: 2000 messages, 2000 distinct nonces
    let engine = encryption::EncryptionEngine::default();
    let mut seen = std::collections::HashSet::new();
    for _ in 0..2000 {
        let e = engine.encrypt(b"same message", &[1u8; 32]).unwrap();
        assert_eq!(e.nonce.len(), 12);
        assert!(seen.insert(e.nonce), "nonce repeated");
    }
}

#[test]
fn encryption_engine_validates_key_length_and_refuses_unimplemented_ciphers() {
    let engine = encryption::EncryptionEngine::default();
    let good = engine.encrypt(b"x", &[0u8; 32]).unwrap();
    for len in [0usize, 1, 16, 24, 31, 33, 64] {
        assert!(engine.encrypt(b"x", &vec![0u8; len]).is_err(), "a {}-byte key must be refused", len);
        assert!(engine.decrypt(&good, &vec![0u8; len]).is_err(), "decrypt with a {}-byte key must be refused", len);
    }
    let e = engine.encrypt(b"hello", &[9u8; 32]).unwrap();
    assert_eq!(e.tag.len(), 16);
    assert_ne!(e.ciphertext, b"hello".to_vec(), "ciphertext is not the plain text");
    assert_eq!(engine.decrypt(&e, &[9u8; 32]).unwrap(), b"hello".to_vec());
    // AES is not implemented: the old code returned a XOR-with-the-key fake that "worked"; now it errors
    for algo in [encryption::EncryptionAlgorithm::AES256GCM, encryption::EncryptionAlgorithm::AES256CBC] {
        assert!(encryption::EncryptionEngine::new(algo).encrypt(b"x", &[0u8; 32]).is_err(), "{} must not pretend to work", algo);
    }
    assert!(cryptography_solver::aes_encrypt(b"plain", &[0u8; 32]).is_err(), "aes_encrypt used to return the plaintext unchanged");
    assert!(cryptography_solver::aes_decrypt(b"x", &[0u8; 32]).is_err());
}

#[test]
fn constant_time_compare_is_correct_on_every_position() {
    let a: Vec<u8> = (0..64).collect();
    assert!(encryption::constant_time_compare(&a, &a.clone()));
    for i in 0..a.len() {
        let mut b = a.clone();
        b[i] ^= 0x80;
        assert!(!encryption::constant_time_compare(&a, &b), "difference at {}", i);
    }
    assert!(!encryption::constant_time_compare(&a, &a[..63]));
    assert!(encryption::constant_time_compare(b"", b""));
    // The tag check in aead::open and the compare above both OR the XOR of every byte pair before deciding, so the
    // time does not depend on where the first difference is. That is by inspection of the code; it is not measured.
}

// ---------------------------------------------------------------------------------------------------------------
// base64 and hex

const B64_VECTORS: &[(&str, &str)] = &[("", ""), ("f", "Zg=="), ("fo", "Zm8="), ("foo", "Zm9v"), ("foob", "Zm9vYg=="), ("fooba", "Zm9vYmE="), ("foobar", "Zm9vYmFy")];

#[test]
fn killer_base64_rfc_4648_section_10() {
    let mut src = String::new();
    for (p, e) in B64_VECTORS {
        src.push_str(&format!("println(base64_encode(\"{p}\"))\nprintln(base64_decode(\"{e}\") == \"{p}\")\nprintln(b64_encode(\"{p}\"))\nprintln(b64_decode(\"{e}\") == \"{p}\")\n"));
    }
    // binary data: all 256 byte values round trip, and the high-bit patterns that exercise + and /
    src.push_str(
        r#"all = bytes_new(256)
i = 0
while i < 256 {
  all = bytes_set(all, i, i)
  i += 1
}
enc = base64_encode(all)
println(len(enc))
back = base64_decode(enc)
println(bytes_len(back))
ok = true
i = 0
while i < 256 {
  if bytes_get(back, i) != i {
    ok = false
  }
  i += 1
}
println(ok)
println(base64_encode(bytes_fill(bytes_new(3), 255)))
println(base64_encode(bytes_fill(bytes_new(4), 255)))
println(base64_encode(bytes_fill(bytes_new(5), 251)))
"#,
    );
    let out = killer_ok(&src);
    for (i, (_, e)) in B64_VECTORS.iter().enumerate() {
        assert_eq!(out[i * 4], *e, "base64_encode");
        assert_eq!(out[i * 4 + 1], "true", "base64_decode of {}", e);
        assert_eq!(out[i * 4 + 2], *e, "b64_encode");
        assert_eq!(out[i * 4 + 3], "true", "b64_decode of {}", e);
    }
    let t = B64_VECTORS.len() * 4;
    assert_eq!(out[t], "344", "256 bytes encode to 4 * ceil(256/3) characters");
    assert_eq!(&out[t + 1..t + 3], ["256", "true"]);
    assert_eq!(out[t + 3], "////");
    assert_eq!(out[t + 4], "/////w==");
    assert_eq!(out[t + 5], "+/v7+/s=");
}

#[test]
fn killer_base64_invalid_input_is_an_error_not_garbage() {
    // base64_decode(): lenient about whitespace, missing padding and the URL-safe alphabet, strict about everything
    // that would make the decoded bytes ambiguous. b64_decode() is the strict RFC 4648 decoder (padding required).
    let bad = ["@@@@", "Zg=", "Zg=a", "Z", "Zm9vY", "Zh==", "Zg===", "=Zg=", "Zg==Zg==", "Zm9v!", "Zm9=", "====", "Zm9vYg=a"];
    let strict_extra = ["Zg", "Zm9vYg", "Zm9vYQ", "Zg== ="];
    let list = |xs: &[&str]| xs.iter().map(|s| format!("\"{}\"", s)).collect::<Vec<_>>().join(", ");
    let strict: Vec<&str> = bad.iter().chain(strict_extra.iter()).copied().collect();
    let src = format!(
        r#"for s in [{bad}] {{
  r = "ok"
  try {{
    base64_decode(s)
  }} catch e {{
    r = "err"
  }}
  println(r)
}}
for s in [{strict}] {{
  r = "ok"
  try {{
    b64_decode(s)
  }} catch e {{
    r = "err"
  }}
  println(r)
}}
println(base64_decode("Zg"))
println(base64_decode("Zm9vYg"))
println(base64_decode("Zm 9v Yg=="))
"#,
        bad = list(&bad),
        strict = list(&strict),
    );
    let out = killer_ok(&src);
    for (i, s) in bad.iter().enumerate() {
        assert_eq!(out[i], "err", "base64_decode({:?}) must be rejected", s);
    }
    for (i, s) in strict.iter().enumerate() {
        assert_eq!(out[bad.len() + i], "err", "b64_decode({:?}) must be rejected", s);
    }
    let m = bad.len() + strict.len();
    assert_eq!(&out[m..m + 3], ["f", "foob", "foob"]);
}

#[test]
fn compression_module_base64_is_standard_and_strict() {
    use killer_native::compression_module::CompressionModule as C;
    for (p, e) in B64_VECTORS {
        assert_eq!(C::base64_encode(p), *e);
        assert_eq!(C::base64_decode(e).as_deref(), Some(*p));
    }
    for bad in ["Zg", "Zg=", "@@@@", "Zh==", "=Zg=", "Zg==Zg==", "Z", "Zm9vYg=a", "===="] {
        assert_eq!(C::base64_decode(bad), None, "{:?}", bad);
    }
}

#[test]
fn killer_hex_builtins() {
    let out = killer_ok(
        r#"println(hex_encode("Hello, world"))
println(hex_encode(""))
println(hex_decode("48656c6c6f2c20776f726c64"))
println(hex_decode("48656C6C6F"))
println(hex_decode("") == "")
for s in ["abc", "zz", "4", "é1", "éé", "0x41", "41 42", "ff"] {
  r = "ok"
  try {
    hex_decode(s)
  } catch e {
    r = "err"
  }
  println(r)
}
"#,
    );
    assert_eq!(out[0], "48656c6c6f2c20776f726c64");
    assert_eq!(out[1], "");
    assert_eq!(out[2], "Hello, world");
    assert_eq!(out[3], "Hello");
    assert_eq!(out[4], "true");
    // odd length, non-hex digits, multi-byte characters (this used to panic on a string slice), and bytes that are
    // not UTF-8 ("ff") are all clean errors
    assert_eq!(out[5..], ["err"; 8]);
}

// ---------------------------------------------------------------------------------------------------------------
// secure_encrypt / secure_decrypt (PBKDF2 + ChaCha20-Poly1305 with random salt and nonce)

#[test]
fn killer_secure_encrypt_round_trips_and_is_fresh_every_time() {
    let msg = "Attack at dawn, naïve café ✓ 日本語";
    let out = killer_ok(&format!(
        r#"pw = "correct horse battery staple"
msg = "{msg}"
c = secure_encrypt(msg, pw)
println(secure_decrypt(c, pw) == msg)
println(secure_decrypt(secure_encrypt("", pw), pw) == "")
big = "x" * 100000
println(secure_decrypt(secure_encrypt(big, pw), pw) == big)
println(c != msg)
cts = []
heads = []
for i in range(0, 6) {{
  e = secure_encrypt(msg, pw)
  push(cts, e)
  raw = base64_decode(e)
  push(heads, base64_encode(bytes_slice(raw, 4, 32)))
}}
println(len(set(cts)))
println(len(set(heads)))
raw = base64_decode(c)
println(base64_encode(bytes_slice(raw, 0, 4)))
println(bytes_len(raw))
"#
    ));
    assert_eq!(&out[..4], ["true", "true", "true", "true"]);
    assert_eq!(out[4], "6", "six encryptions of the same text under the same password must all differ");
    assert_eq!(out[5], "6", "salt and nonce (bytes 4..32) must be fresh every time");
    assert_eq!(out[6], "S1NFMg==", "format 2 prefix KSE2");
    // 4 magic + 16 salt + 12 nonce + ciphertext + 16 tag
    assert_eq!(out[7], (4 + 16 + 12 + msg.len() + 16).to_string());
}

#[test]
fn killer_secure_decrypt_rejects_wrong_password_tampering_and_garbage() {
    let out = killer_ok(
        r#"pw = "right"
msg = "wire the money to account 12345"
c = secure_encrypt(msg, pw)
raw = base64_decode(c)
n = bytes_len(raw)
fn attempt(data, p) {
  r = "decrypted"
  try {
    secure_decrypt(data, p)
  } catch e {
    r = "rejected"
  }
  return r
}
println(attempt(c, pw))
println(attempt(c, "wrong"))
println(attempt(c, ""))
for i in [0, 4, 19, 20, 31, 32, n - 17, n - 16, n - 1] {
  bad = bytes_set(raw, i, (bytes_get(raw, i) + 1) % 256)
  println(attempt(base64_encode(bad), pw))
}
println(attempt(base64_encode(bytes_slice(raw, 0, n - 1)), pw))
println(attempt(base64_encode(bytes_concat(raw, bytes_new(1))), pw))
println(attempt("", pw))
println(attempt("!!!!", pw))
println(attempt("AAAA", pw))
println(attempt(base64_encode(bytes_slice(raw, 0, 20)), pw))
"#,
    );
    assert_eq!(out[0], "decrypted");
    assert_eq!(out[1], "rejected", "wrong password");
    assert_eq!(out[2], "rejected", "empty password is not a wildcard");
    for (i, o) in out[3..12].iter().enumerate() {
        assert_eq!(o, "rejected", "tamper case {}", i);
    }
    assert_eq!(out.len(), 18);
    for (i, o) in out[12..].iter().enumerate() {
        assert_eq!(o, "rejected", "truncated/extended/garbage case {}", i);
    }
}

#[test]
fn killer_secure_random_bytes_and_uuid() {
    let out = killer_ok(
        r#"println(bytes_len(base64_decode(secure_random_bytes(32))))
println(bytes_len(base64_decode(secure_random_bytes())))
println(bytes_len(base64_decode(secure_random_bytes(1))))
println(bytes_len(base64_decode(secure_random_bytes(1024))))
println(bytes_len(base64_decode(secure_random_bytes(5000))))
println(secure_random_bytes(0) == "")
a = []
for i in range(0, 50) {
  push(a, secure_random_bytes(32))
}
println(len(set(a)))
u = []
for i in range(0, 200) {
  push(u, uuid())
}
println(len(set(u)))
println(len(u[0]))
println(slice(u[0], 14, 15))
"#,
    );
    assert_eq!(&out[..5], ["32", "32", "1", "1024", "1024"], "default 32 bytes, capped at 1024");
    assert_eq!(out[5], "true");
    assert_eq!(out[6], "50");
    assert_eq!(out[7], "200");
    assert_eq!(out[8], "36");
    assert_eq!(out[9], "4", "UUID version 4");
}

#[test]
fn killer_pin_hash_is_salted_and_verifies() {
    let out = killer_ok(
        r#"println(secure_pin_is_set())
println(secure_pin_set("4821"))
println(secure_pin_is_set())
println(secure_pin_verify("4821"))
println(secure_pin_verify("0000"))
println(secure_pin_verify("4821"))
"#,
    );
    assert_eq!(out, ["false", "true", "true", "true", "false", "true"]);
}

// ---------------------------------------------------------------------------------------------------------------
// cross-checks against the independent Python implementation

#[test]
fn python_sweep_matches_every_primitive() {
    let Some(expected) = reference(&["sweep"]) else {
        eprintln!("python not installed: skipping the Python cross-check");
        return;
    };
    let mut got = std::collections::HashMap::new();
    for line in expected.lines() {
        let mut it = line.split(' ');
        let (kind, n, h) = (it.next().unwrap(), it.next().unwrap(), it.next().unwrap());
        got.insert((kind.to_string(), n.parse::<usize>().unwrap()), h.to_string());
    }
    let mut checked = 0;
    for n in (0..300).step_by(7) {
        let data: Vec<u8> = (0..n).map(|i| ((i * 31 + n) % 256) as u8).collect();
        let key = &data[..n % 97];
        let (pw, salt) = (&data[..(n % 70).min(n)], &data[..(n % 40).min(n)]);
        let nonce: Vec<u8> = (0..12).map(|i| ((n + i) % 256) as u8).collect();
        let k32: Vec<u8> = (0..32).map(|i| ((n * 3 + i * 5) % 256) as u8).collect();
        let aad = &data[..(n % 29).min(n)];
        let mut check = |kind: &str, have: String| {
            assert_eq!(&have, got.get(&(kind.to_string(), n)).unwrap(), "{} mismatch at input size {}", kind, n);
            checked += 1;
        };
        check("sha256", hex(&encryption::sha256(&data)));
        check("sha512", hex(&encryption::sha512(&data)));
        check("sha1", hex(&net_sock::sha1(&data)));
        check("hmac", hex(&encryption::hmac_sha256(key, &data)));
        check("pbkdf2", hex(&encryption::pbkdf2_hmac_sha256(pw, salt, 1 + (n % 5) as u32, 1 + n % 80)));
        check("aead", hex(&aead::seal(&k32.try_into().unwrap(), &nonce.try_into().unwrap(), aad, &data)));
    }
    assert_eq!(checked, 43 * 6);
}

#[test]
fn python_and_killer_decrypt_each_others_secure_encrypt_output() {
    if python().is_none() {
        eprintln!("python not installed: skipping the Python interoperability check");
        return;
    }
    // Python encrypts (fixed salt and nonce), Killer decrypts
    let from_python = reference(&["kse2-encrypt", "pässword", "héllo wörld 123"]).unwrap();
    let out = killer_ok(&format!("println(secure_decrypt(\"{}\", \"pässword\"))\nprintln(secure_encrypt(\"héllo wörld 123\", \"pässword\"))\n", from_python.trim()));
    assert_eq!(out[0], "héllo wörld 123");
    // Killer encrypts (random salt and nonce), Python decrypts
    let plain = reference(&["kse2-decrypt", out[1].trim(), "pässword"]).unwrap();
    assert_eq!(plain, "héllo wörld 123");
    // and a wrong password makes the Python reference fail too (so the check above is not vacuous)
    let py = python().unwrap();
    let script = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/data/crypto_reference.py");
    let bad = Command::new(py).arg(script).args(["kse2-decrypt", out[1].trim(), "wrong"]).output().unwrap();
    assert!(!bad.status.success());
}
