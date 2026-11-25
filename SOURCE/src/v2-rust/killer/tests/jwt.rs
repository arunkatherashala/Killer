//! `jwt_sign(claims, secret[, opts])` / `jwt_verify(token, secret[, opts])`: HS256 / HS384 / HS512 JSON Web Tokens.
//!
//! Covers the RFC 7515 Appendix A.1 example token (verified with the RFC's key), tampering (payload, header,
//! `alg: none`, algorithm confusion), the time claims with leeway, audience / issuer, the size limit, the minimum
//! secret length, strict base64url, round trips, and that no capability is needed (pure computation).
use killer_native::builtin::BuiltinFunctions;
use killer_native::security::{CapabilityScopeGuard, CapabilitySet};
use killer_native::value::{SharedDict, Value};

fn s(x: &str) -> Value {
    Value::Str(x.to_string())
}

fn n(x: f64) -> Value {
    Value::Number(x)
}

fn dict(pairs: &[(&str, Value)]) -> Value {
    Value::Dict(SharedDict::from_pairs(pairs.iter().map(|(k, v)| (k.to_string(), v.clone())).collect()))
}

fn list(items: &[Value]) -> Value {
    Value::from(items.to_vec())
}

fn now() -> f64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() as f64
}

fn sign(claims: Value, secret: &Value, opts: Option<Value>) -> Result<String, String> {
    let mut args = vec![claims, secret.clone()];
    if let Some(o) = opts {
        args.push(o);
    }
    match BuiltinFunctions::call("jwt_sign", &args).map_err(|e| format!("{e}"))? {
        Value::Str(t) => Ok(t.to_string()),
        other => Err(format!("jwt_sign returned {other:?}")),
    }
}

fn verify(token: &str, secret: &Value, opts: Option<Value>) -> Result<Value, String> {
    let mut args = vec![s(token), secret.clone()];
    if let Some(o) = opts {
        args.push(o);
    }
    BuiltinFunctions::call("jwt_verify", &args).map_err(|e| format!("{e}"))
}

fn field(v: &Value, k: &str) -> Option<Value> {
    match v {
        Value::Dict(d) => d.get(k),
        _ => None,
    }
}

fn expect_err(r: Result<Value, String>, needle: &str) {
    match r {
        Err(m) => assert!(m.to_lowercase().contains(&needle.to_lowercase()), "error {m:?} should mention {needle:?}"),
        Ok(v) => panic!("expected an error mentioning {needle:?}, got {v:?}"),
    }
}

/// base64url without padding (RFC 7515 section 2), for building hand-made tokens.
fn b64url(data: &[u8]) -> String {
    const A: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut out = String::new();
    for c in data.chunks(3) {
        let b = [c[0], *c.get(1).unwrap_or(&0), *c.get(2).unwrap_or(&0)];
        let v = ((b[0] as u32) << 16) | ((b[1] as u32) << 8) | b[2] as u32;
        let chars = c.len() + 1;
        for i in 0..chars {
            out.push(A[((v >> (18 - 6 * i)) & 63) as usize] as char);
        }
    }
    out
}

// RFC 7515 Appendix A.1 (also RFC 7519 section 3.1): the JWS compact serialization and its HMAC key.
const A1_TOKEN: &str = "eyJ0eXAiOiJKV1QiLA0KICJhbGciOiJIUzI1NiJ9.eyJpc3MiOiJqb2UiLA0KICJleHAiOjEzMDA4MTkzODAsDQogImh0dHA6Ly9leGFtcGxlLmNvbS9pc19yb290Ijp0cnVlfQ.dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";
const A1_KEY: [u8; 64] = [
    3, 35, 53, 75, 43, 15, 165, 188, 131, 126, 6, 101, 119, 123, 166, 143, 90, 179, 40, 230, 240, 84, 201, 40, 169, 15,
    132, 178, 210, 80, 46, 191, 211, 251, 90, 146, 210, 6, 71, 239, 150, 138, 180, 195, 119, 98, 61, 34, 61, 46, 33,
    114, 5, 46, 79, 8, 192, 205, 154, 245, 103, 208, 128, 163,
];
const A1_EXP: f64 = 1300819380.0;

fn a1_key() -> Value {
    Value::Bytes(A1_KEY.to_vec())
}

fn secret32() -> Value {
    s("0123456789abcdef0123456789abcdef")
}

#[test]
fn rfc7515_appendix_a1_token_verifies_with_the_rfc_key() {
    let opts = dict(&[("now", n(A1_EXP - 100.0)), ("iss", s("joe"))]);
    let claims = verify(A1_TOKEN, &a1_key(), Some(opts)).expect("A.1 token must verify");
    assert_eq!(field(&claims, "iss"), Some(s("joe")));
    assert_eq!(field(&claims, "http://example.com/is_root"), Some(Value::Bool(true)));
    let exp = match field(&claims, "exp") {
        Some(Value::Number(x)) => x,
        Some(Value::Integer(i)) => i as f64,
        other => panic!("exp: {other:?}"),
    };
    assert_eq!(exp, A1_EXP);
}

#[test]
fn rfc7515_appendix_a1_token_is_expired_on_the_real_clock() {
    expect_err(verify(A1_TOKEN, &a1_key(), None), "expired");
}

#[test]
fn wrong_key_is_rejected() {
    let mut k = A1_KEY.to_vec();
    k[0] ^= 1;
    let opts = dict(&[("now", n(A1_EXP - 100.0))]);
    expect_err(verify(A1_TOKEN, &Value::Bytes(k), Some(opts)), "signature");
}

#[test]
fn tampered_payload_is_rejected() {
    let parts: Vec<&str> = A1_TOKEN.split('.').collect();
    let forged = b64url(b"{\"iss\":\"joe\",\r\n \"exp\":1300819380,\r\n \"http://example.com/is_admin\":true}");
    let t = format!("{}.{}.{}", parts[0], forged, parts[2]);
    let opts = dict(&[("now", n(A1_EXP - 100.0))]);
    expect_err(verify(&t, &a1_key(), Some(opts)), "signature");
    // the signature itself changed by one character
    let mut sig = parts[2].to_string();
    sig.replace_range(0..1, if sig.starts_with('A') { "B" } else { "A" });
    let t = format!("{}.{}.{}", parts[0], parts[1], sig);
    expect_err(verify(&t, &a1_key(), Some(dict(&[("now", n(A1_EXP - 100.0))]))), "signature");
}

#[test]
fn alg_none_is_always_rejected() {
    let payload = b64url(format!("{{\"sub\":\"x\",\"exp\":{}}}", now() + 600.0).as_bytes());
    for alg in ["none", "None", "NONE", "nOnE"] {
        let header = b64url(format!("{{\"alg\":\"{alg}\",\"typ\":\"JWT\"}}").as_bytes());
        for t in [format!("{header}.{payload}."), format!("{header}.{payload}.c2ln")] {
            expect_err(verify(&t, &secret32(), None), "none");
            // even when the caller lists it as allowed
            let opts = dict(&[("algorithms", list(&[s(alg), s("HS256")]))]);
            expect_err(verify(&t, &secret32(), Some(opts)), "none");
        }
    }
    // signing with none is refused too
    let r = sign(dict(&[("sub", s("x"))]), &secret32(), Some(dict(&[("alg", s("none"))])));
    assert!(matches!(&r, Err(m) if m.contains("none")), "{r:?}");
}

#[test]
fn algorithm_confusion_is_rejected() {
    let key64 = s(&"k".repeat(64));
    let claims = dict(&[("sub", s("x")), ("exp", n(now() + 600.0))]);
    let t512 = sign(claims.clone(), &key64, Some(dict(&[("alg", s("HS512"))]))).unwrap();
    // default allow list is only HS256
    expect_err(verify(&t512, &key64, None), "not allowed");
    // explicitly allowed: accepted
    let ok = verify(&t512, &key64, Some(dict(&[("algorithms", list(&[s("HS256"), s("HS512")]))])));
    assert!(ok.is_ok(), "{ok:?}");
    // an HS256 token whose header was rewritten to HS384 is refused (not allowed, or bad signature)
    let t256 = sign(claims, &key64, None).unwrap();
    let parts: Vec<&str> = t256.split('.').collect();
    let h = b64url(b"{\"alg\":\"HS384\",\"typ\":\"JWT\"}");
    let forged = format!("{}.{}.{}", h, parts[1], parts[2]);
    expect_err(verify(&forged, &key64, None), "not allowed");
    let all = dict(&[("algorithms", list(&[s("HS256"), s("HS384")]))]);
    expect_err(verify(&forged, &key64, Some(all)), "signature");
    // a header without alg, or with a non-string alg
    for hdr in [&b"{\"typ\":\"JWT\"}"[..], b"{\"alg\":256}", b"[\"HS256\"]", b"not json"] {
        let t = format!("{}.{}.{}", b64url(hdr), parts[1], parts[2]);
        assert!(verify(&t, &key64, None).is_err(), "header {:?}", String::from_utf8_lossy(hdr));
    }
}

#[test]
fn public_key_algorithms_are_reported_as_not_built() {
    let want = "unsupported algorithm RS256: needs a public-key crate (not built)";
    let r = sign(dict(&[("sub", s("x"))]), &secret32(), Some(dict(&[("alg", s("RS256"))])));
    assert!(matches!(&r, Err(m) if m.contains(want)), "{r:?}");
    let h = b64url(b"{\"alg\":\"RS256\"}");
    let p = b64url(format!("{{\"exp\":{}}}", now() + 600.0).as_bytes());
    let t = format!("{h}.{p}.c2ln");
    expect_err(verify(&t, &secret32(), Some(dict(&[("algorithms", list(&[s("RS256")]))]))), want);
    for alg in ["ES256", "PS256", "EdDSA"] {
        let r = sign(dict(&[("sub", s("x"))]), &secret32(), Some(dict(&[("alg", s(alg))])));
        assert!(matches!(&r, Err(m) if m.contains(&format!("unsupported algorithm {alg}"))), "{alg}: {r:?}");
    }
}

#[test]
fn expired_tokens_are_rejected_with_leeway() {
    let k = secret32();
    let t = sign(dict(&[("exp", n(now() - 3600.0))]), &k, None).unwrap();
    expect_err(verify(&t, &k, None), "expired");
    // 30 s past exp is inside the default 60 s leeway
    let t = sign(dict(&[("exp", n(now() - 30.0))]), &k, None).unwrap();
    assert!(verify(&t, &k, None).is_ok());
    expect_err(verify(&t, &k, Some(dict(&[("leeway", n(0.0))]))), "expired");
    // exp of the wrong type
    let t = sign(dict(&[("exp", s("tomorrow"))]), &k, None).unwrap();
    expect_err(verify(&t, &k, None), "exp");
}

#[test]
fn not_yet_valid_and_future_iat_are_rejected() {
    let k = secret32();
    let exp = n(now() + 7200.0);
    let t = sign(dict(&[("exp", exp.clone()), ("nbf", n(now() + 3600.0))]), &k, None).unwrap();
    expect_err(verify(&t, &k, None), "not yet valid");
    let t = sign(dict(&[("exp", exp.clone()), ("nbf", n(now() + 30.0))]), &k, None).unwrap();
    assert!(verify(&t, &k, None).is_ok(), "nbf inside leeway");
    let t = sign(dict(&[("exp", exp), ("iat", n(now() + 3600.0))]), &k, None).unwrap();
    expect_err(verify(&t, &k, None), "iat");
}

#[test]
fn exp_is_required_by_default() {
    let k = secret32();
    let t = sign(dict(&[("sub", s("x"))]), &k, None).unwrap();
    expect_err(verify(&t, &k, None), "exp");
    let c = verify(&t, &k, Some(dict(&[("require_exp", Value::Bool(false))]))).unwrap();
    assert_eq!(field(&c, "sub"), Some(s("x")));
    // expires_in adds iat and exp
    let t = sign(dict(&[("sub", s("x"))]), &k, Some(dict(&[("expires_in", n(600.0))]))).unwrap();
    let c = verify(&t, &k, None).unwrap();
    assert!(field(&c, "exp").is_some() && field(&c, "iat").is_some(), "{c:?}");
}

#[test]
fn audience_and_issuer_are_checked() {
    let k = secret32();
    let exp = n(now() + 600.0);
    let t = sign(dict(&[("exp", exp.clone()), ("aud", s("api")), ("iss", s("auth.example"))]), &k, None).unwrap();
    expect_err(verify(&t, &k, Some(dict(&[("aud", s("other"))]))), "audience");
    assert!(verify(&t, &k, Some(dict(&[("aud", s("api"))]))).is_ok());
    assert!(verify(&t, &k, Some(dict(&[("aud", list(&[s("x"), s("api")]))]))).is_ok());
    // a token with an audience must be checked against one
    expect_err(verify(&t, &k, None), "audience");
    expect_err(verify(&t, &k, Some(dict(&[("aud", s("api")), ("iss", s("evil"))]))), "issuer");
    assert!(verify(&t, &k, Some(dict(&[("aud", s("api")), ("iss", s("auth.example"))]))).is_ok());
    // aud as a list in the token
    let t = sign(dict(&[("exp", exp.clone()), ("aud", list(&[s("a"), s("b")]))]), &k, None).unwrap();
    assert!(verify(&t, &k, Some(dict(&[("aud", s("b"))]))).is_ok());
    expect_err(verify(&t, &k, Some(dict(&[("aud", s("c"))]))), "audience");
    // expected audience / issuer but the token has none
    let t = sign(dict(&[("exp", exp)]), &k, None).unwrap();
    expect_err(verify(&t, &k, Some(dict(&[("aud", s("api"))]))), "audience");
    expect_err(verify(&t, &k, Some(dict(&[("iss", s("auth.example"))]))), "issuer");
}

#[test]
fn oversize_tokens_are_rejected() {
    let k = secret32();
    let big = "a".repeat(20 * 1024);
    expect_err(verify(&format!("{big}.{big}.{big}"), &k, None), "too large");
    let r = sign(dict(&[("blob", s(&big)), ("exp", n(now() + 600.0))]), &k, None);
    assert!(matches!(&r, Err(m) if m.contains("too large")), "{r:?}");
    // just under the limit works
    let fits = "a".repeat(10 * 1024);
    let t = sign(dict(&[("blob", s(&fits)), ("exp", n(now() + 600.0))]), &k, None).unwrap();
    assert!(t.len() <= 16 * 1024);
    assert!(verify(&t, &k, None).is_ok());
}

#[test]
fn short_secrets_are_rejected() {
    let claims = dict(&[("exp", n(now() + 600.0))]);
    let short = s(&"k".repeat(31));
    let r = sign(claims.clone(), &short, None);
    assert!(matches!(&r, Err(m) if m.contains("at least 32 bytes")), "{r:?}");
    let t = sign(claims.clone(), &secret32(), None).unwrap();
    expect_err(verify(&t, &short, None), "at least 32 bytes");
    expect_err(verify(&t, &s(""), None), "at least 32 bytes");
    let r = sign(claims.clone(), &s(&"k".repeat(47)), Some(dict(&[("alg", s("HS384"))])));
    assert!(matches!(&r, Err(m) if m.contains("at least 48 bytes")), "{r:?}");
    let r = sign(claims, &s(&"k".repeat(63)), Some(dict(&[("alg", s("HS512"))])));
    assert!(matches!(&r, Err(m) if m.contains("at least 64 bytes")), "{r:?}");
}

#[test]
fn round_trip_all_hmac_algorithms() {
    let key = Value::Bytes((0u8..64).collect());
    let claims = dict(&[
        ("sub", s("user-42")),
        ("name", s("Arun \"quoted\" \u{e9}")),
        ("roles", list(&[s("admin"), s("dev")])),
        ("n", n(3.5)),
        ("nested", dict(&[("ok", Value::Bool(true)), ("none", Value::Null)])),
        ("exp", n(now() + 600.0)),
    ]);
    for alg in ["HS256", "HS384", "HS512"] {
        let t = sign(claims.clone(), &key, Some(dict(&[("alg", s(alg)), ("kid", s("key-1"))]))).unwrap();
        assert_eq!(t.split('.').count(), 3);
        assert!(!t.contains('=') && !t.contains('+') && !t.contains('/'), "{t}");
        let opts = dict(&[("algorithms", list(&[s(alg)]))]);
        let back = verify(&t, &key, Some(opts)).unwrap();
        for k in ["sub", "name", "roles", "n", "exp"] {
            assert_eq!(field(&back, k), field(&claims, k), "{alg} {k}");
        }
        let nested = field(&back, "nested").unwrap();
        assert_eq!(field(&nested, "ok"), Some(Value::Bool(true)));
        assert_eq!(field(&nested, "none"), Some(Value::Null));
    }
}

#[test]
fn base64url_is_strict() {
    let k = secret32();
    let t = sign(dict(&[("exp", n(now() + 600.0))]), &k, None).unwrap();
    assert!(verify(&t, &k, None).is_ok());
    let parts: Vec<&str> = t.split('.').collect();
    let bad = [
        format!("{}=.{}.{}", parts[0], parts[1], parts[2]),
        format!("{}.{}.{}=", parts[0], parts[1], parts[2]),
        format!("{}.{}.{}", parts[0].replace('J', "+"), parts[1], parts[2]),
        format!("{}.{}/.{}", parts[0], parts[1], parts[2]),
        format!("{} .{}.{}", parts[0], parts[1], parts[2]),
        format!("{}.{}", parts[0], parts[1]),
        format!("{}.{}.{}.{}", parts[0], parts[1], parts[2], parts[2]),
        format!("{}..{}", parts[0], parts[2]),
        String::new(),
    ];
    for b in &bad {
        assert!(verify(b, &k, None).is_err(), "{b:?} should be rejected");
    }
    // a non-canonical last character (unused low bits set) decodes to the same bytes in lax decoders
    let sig = parts[2];
    let last = sig.chars().last().unwrap();
    // 32-byte HS256 signature -> 43 chars, the last carries 2 unused bits
    let alphabet = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let idx = alphabet.find(last).unwrap();
    let alt = alphabet.chars().nth(idx | 1).unwrap();
    if alt != last {
        let t2 = format!("{}.{}.{}{}", parts[0], parts[1], &sig[..sig.len() - 1], alt);
        expect_err(verify(&t2, &k, None), "base64url");
    }
}

#[test]
fn no_capability_is_needed() {
    let _g = CapabilityScopeGuard::install(CapabilitySet::restricted());
    let k = secret32();
    let t = sign(dict(&[("sub", s("x")), ("exp", n(now() + 600.0))]), &k, None).unwrap();
    let c = verify(&t, &k, None).unwrap();
    assert_eq!(field(&c, "sub"), Some(s("x")));
}

#[test]
fn bad_arguments_are_clear_errors() {
    let k = secret32();
    assert!(sign(s("not a dict"), &k, None).is_err());
    assert!(sign(dict(&[]), &n(5.0), None).is_err());
    assert!(BuiltinFunctions::call("jwt_verify", &[n(1.0), k.clone()]).is_err());
    assert!(BuiltinFunctions::call("jwt_verify", &[]).is_err());
    let t = sign(dict(&[("exp", n(now() + 600.0))]), &k, None).unwrap();
    expect_err(verify(&t, &k, Some(dict(&[("algorithms", s("HS256"))]))), "algorithms");
    expect_err(verify(&t, &k, Some(dict(&[("leeway", n(-1.0))]))), "leeway");
    // a payload that is valid JSON but not an object
    let h = t.split('.').next().unwrap();
    let p = b64url(b"[1,2,3]");
    assert!(verify(&format!("{h}.{p}.c2ln"), &k, None).is_err());
}

#[test]
fn scripts_can_call_jwt_builtins() {
    let dir = std::env::temp_dir().join(format!("killer_jwt_script_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("t.killer");
    std::fs::write(
        &file,
        r#"secret = "0123456789abcdef0123456789abcdef"
t = jwt_sign({"sub": "arun"}, secret, {"expires_in": 600})
c = jwt_verify(t, secret)
println(c["sub"])
try {
  jwt_verify(t + "x", secret)
  println("accepted a tampered token")
} catch e {
  println("rejected")
}
"#,
    )
    .unwrap();
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_killer")).arg("run").arg(&file).current_dir(&dir).output().unwrap();
    let _ = std::fs::remove_dir_all(&dir);
    let stdout = String::from_utf8_lossy(&out.stdout).replace("\r\n", "\n");
    assert!(out.status.success(), "stderr: {}", String::from_utf8_lossy(&out.stderr));
    assert_eq!(stdout.trim_end(), "arun\nrejected");
}
