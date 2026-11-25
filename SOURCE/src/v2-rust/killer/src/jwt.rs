//! JSON Web Tokens (RFC 7519) signed with HMAC (RFC 7515 / RFC 7518 section 3.2): HS256, HS384, HS512.
//!
//! `jwt_sign(claims, secret[, opts])` returns the compact token `header.payload.signature`.
//!   opts: `alg` ("HS256" default, "HS384", "HS512"), `kid` (string, copied into the header),
//!   `expires_in` (seconds: sets `iat` to now and `exp` to now + expires_in).
//! `jwt_verify(token, secret[, opts])` returns the claims dict, or fails with a clear error.
//!   opts: `algorithms` (list of allowed algs, default ["HS256"]), `leeway` (seconds, default 60),
//!   `require_exp` (default true), `aud` (string or list), `iss` (string or list), `now` (unix seconds, for tests
//!   and replays; default the system clock).
//!
//! Security rules (see docs/LANGUAGE.md, "JSON Web Tokens"):
//! - `alg: none` (any letter case) is always refused, even when listed in `algorithms`; any alg not in the allow
//!   list is refused before the signature is looked at (no algorithm confusion).
//! - RS*/ES*/PS*/EdDSA need public-key crates that are not built in: they fail with
//!   "unsupported algorithm RS256: needs a public-key crate (not built)".
//! - The signature is compared in constant time (`subtle`).
//! - base64url is strict: alphabet `A-Za-z0-9-_`, no padding, no whitespace, unused low bits must be zero.
//! - Tokens (and signed output) are limited to 16 KiB.
//! - The secret must be at least as long as the hash output (RFC 7518 section 3.2): 32 bytes for HS256,
//!   48 for HS384, 64 for HS512.
//! - A header with `crit` is refused (no extensions are understood, RFC 7515 section 4.1.11).
//! - `exp` / `nbf` / `iat` must be numbers; a token with `aud` must be checked against an expected `aud`.
//!
//! Pure computation: no capability is needed (no file, network or process access).

use crate::error::VmError;
use crate::value::{SharedDict, Value};

/// Largest token `jwt_verify` accepts and `jwt_sign` produces.
pub const MAX_TOKEN_BYTES: usize = 16 * 1024;
const DEFAULT_LEEWAY: f64 = 60.0;

pub fn call(name: &str, args: &[Value]) -> Option<Result<Value, VmError>> {
    match name {
        "jwt_sign" => Some(sign_builtin(args).map(Value::Str).map_err(VmError::runtime_error)),
        "jwt_verify" => Some(verify_builtin(args).map_err(VmError::runtime_error)),
        _ => None,
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Alg {
    Hs256,
    Hs384,
    Hs512,
}

impl Alg {
    fn name(self) -> &'static str {
        match self {
            Alg::Hs256 => "HS256",
            Alg::Hs384 => "HS384",
            Alg::Hs512 => "HS512",
        }
    }

    fn min_key(self) -> usize {
        match self {
            Alg::Hs256 => 32,
            Alg::Hs384 => 48,
            Alg::Hs512 => 64,
        }
    }

    fn mac(self, key: &[u8], data: &[u8]) -> Vec<u8> {
        use hmac::{Hmac, Mac};
        fn run<M: Mac + hmac::digest::KeyInit>(key: &[u8], data: &[u8]) -> Vec<u8> {
            // HMAC accepts keys of any length; the fallback is never taken.
            match <M as Mac>::new_from_slice(key) {
                Ok(mut m) => {
                    m.update(data);
                    m.finalize().into_bytes().to_vec()
                }
                Err(_) => Vec::new(),
            }
        }
        match self {
            Alg::Hs256 => run::<Hmac<sha2::Sha256>>(key, data),
            Alg::Hs384 => run::<Hmac<sha2::Sha384>>(key, data),
            Alg::Hs512 => run::<Hmac<sha2::Sha512>>(key, data),
        }
    }
}

/// Map an algorithm name to a supported one, or say why it is refused.
fn parse_alg(name: &str) -> Result<Alg, String> {
    match name {
        "HS256" => Ok(Alg::Hs256),
        "HS384" => Ok(Alg::Hs384),
        "HS512" => Ok(Alg::Hs512),
        n if n.eq_ignore_ascii_case("none") => Err("algorithm 'none' is never accepted (unsigned tokens are refused)".to_string()),
        n if ["RS", "PS", "ES"].iter().any(|p| n.starts_with(p)) || n == "EdDSA" => {
            Err(format!("unsupported algorithm {}: needs a public-key crate (not built)", n))
        }
        n => Err(format!("unsupported algorithm {}", short(n))),
    }
}

/// Clip untrusted text before it goes into an error message.
fn short(s: &str) -> String {
    let clipped: String = s.chars().take(32).collect();
    if clipped.len() < s.len() {
        format!("{:?}...", clipped)
    } else {
        format!("{:?}", clipped).trim_matches('"').to_string()
    }
}

const B64URL: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";

/// base64url without padding (RFC 7515 section 2).
pub fn b64url_encode(data: &[u8]) -> String {
    let mut out = String::with_capacity(data.len() * 4 / 3 + 3);
    for c in data.chunks(3) {
        let b1 = c.get(1).copied().unwrap_or(0);
        let b2 = c.get(2).copied().unwrap_or(0);
        let v = ((c[0] as u32) << 16) | ((b1 as u32) << 8) | b2 as u32;
        for i in 0..=c.len() {
            out.push(B64URL[((v >> (18 - 6 * i)) & 63) as usize] as char);
        }
    }
    out
}

/// Strict base64url decoding: only `A-Za-z0-9-_`, no padding or whitespace, and the unused low bits of the last
/// character must be zero (so every byte string has exactly one accepted encoding).
pub fn b64url_decode(s: &str) -> Result<Vec<u8>, String> {
    fn val(c: u8) -> Option<u32> {
        match c {
            b'A'..=b'Z' => Some((c - b'A') as u32),
            b'a'..=b'z' => Some((c - b'a') as u32 + 26),
            b'0'..=b'9' => Some((c - b'0') as u32 + 52),
            b'-' => Some(62),
            b'_' => Some(63),
            _ => None,
        }
    }
    let bytes = s.as_bytes();
    if bytes.len() % 4 == 1 {
        return Err("invalid base64url: impossible length".to_string());
    }
    let mut out = Vec::with_capacity(bytes.len() * 3 / 4);
    for chunk in bytes.chunks(4) {
        let mut v: u32 = 0;
        for (i, &c) in chunk.iter().enumerate() {
            let d = val(c).ok_or_else(|| {
                if c == b'=' {
                    "invalid base64url: padding '=' is not allowed".to_string()
                } else {
                    "invalid base64url: character outside A-Z a-z 0-9 - _".to_string()
                }
            })?;
            v |= d << (18 - 6 * i);
        }
        let n = chunk.len() - 1; // bytes carried by this chunk
        let unused_mask: u32 = match chunk.len() {
            2 => 0xFFFF,
            3 => 0xFF,
            _ => 0,
        };
        if v & unused_mask != 0 {
            return Err("invalid base64url: non-canonical encoding (unused bits set)".to_string());
        }
        for i in 0..n {
            out.push((v >> (16 - 8 * i)) as u8);
        }
    }
    Ok(out)
}

fn secret_bytes(v: &Value, who: &str) -> Result<Vec<u8>, String> {
    match v {
        Value::Str(s) => Ok(s.as_bytes().to_vec()),
        Value::Bytes(b) => Ok(b.clone()),
        _ => Err(format!("{}(): the secret must be a string or bytes", who)),
    }
}

fn check_key(alg: Alg, key: &[u8], who: &str) -> Result<(), String> {
    if key.len() < alg.min_key() {
        return Err(format!(
            "{}(): the secret for {} must be at least {} bytes (got {}); use a random key such as secure_random_bytes({})",
            who,
            alg.name(),
            alg.min_key(),
            key.len(),
            alg.min_key()
        ));
    }
    Ok(())
}

fn opts_dict(v: Option<&Value>, who: &str) -> Result<Option<SharedDict>, String> {
    match v {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Dict(d)) => Ok(Some(d.clone())),
        Some(_) => Err(format!("{}(): opts must be a dict", who)),
    }
}

fn opt(d: &Option<SharedDict>, key: &str) -> Option<Value> {
    d.as_ref().and_then(|d| d.get(key)).filter(|v| !matches!(v, Value::Null))
}

fn num(v: &Value) -> Option<f64> {
    match v {
        Value::Number(n) if n.is_finite() => Some(*n),
        Value::Integer(i) => Some(*i as f64),
        _ => None,
    }
}

fn now_secs() -> f64 {
    match std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
        Ok(d) => d.as_secs_f64(),
        Err(_) => 0.0,
    }
}

fn sign_builtin(args: &[Value]) -> Result<String, String> {
    const W: &str = "jwt_sign";
    if args.len() < 2 || args.len() > 3 {
        return Err("jwt_sign() expects (claims, secret[, opts])".to_string());
    }
    let claims = match &args[0] {
        Value::Dict(d) => d.clone(),
        _ => return Err("jwt_sign(): claims must be a dict".to_string()),
    };
    let key = secret_bytes(&args[1], W)?;
    let opts = opts_dict(args.get(2), W)?;
    let alg = match opt(&opts, "alg") {
        None => Alg::Hs256,
        Some(Value::Str(s)) => parse_alg(&s).map_err(|e| format!("jwt_sign(): {}", e))?,
        Some(_) => return Err("jwt_sign(): opts.alg must be a string".to_string()),
    };
    check_key(alg, &key, W)?;

    let mut header: Vec<(String, Value)> = vec![("alg".to_string(), Value::Str(alg.name().into())), ("typ".to_string(), Value::Str("JWT".into()))];
    match opt(&opts, "kid") {
        None => {}
        Some(Value::Str(k)) => header.push(("kid".to_string(), Value::Str(k))),
        Some(_) => return Err("jwt_sign(): opts.kid must be a string".to_string()),
    }

    // copy so expires_in never changes the caller's dict
    let mut pairs: Vec<(String, Value)> = {
        let m = claims.borrow();
        m.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
    };
    match opt(&opts, "expires_in") {
        None => {}
        Some(v) => {
            let secs = num(&v).filter(|s| *s > 0.0).ok_or("jwt_sign(): opts.expires_in must be a positive number of seconds")?;
            let now = now_secs().floor();
            pairs.retain(|(k, _)| k != "iat" && k != "exp");
            pairs.push(("iat".to_string(), Value::Number(now)));
            pairs.push(("exp".to_string(), Value::Number(now + secs.ceil())));
        }
    }

    let header_json = crate::json_lang::stringify(&Value::Dict(SharedDict::from_pairs(header)), 0).map_err(|e| format!("jwt_sign(): {}", e))?;
    let claims_json = crate::json_lang::stringify(&Value::Dict(SharedDict::from_pairs(pairs)), 0).map_err(|e| format!("jwt_sign(): {}", e))?;
    if claims_json.len() > MAX_TOKEN_BYTES {
        return Err(format!("jwt_sign(): token too large (limit {} bytes)", MAX_TOKEN_BYTES));
    }
    let mut token = b64url_encode(header_json.as_bytes());
    token.push('.');
    token.push_str(&b64url_encode(claims_json.as_bytes()));
    let sig = alg.mac(&key, token.as_bytes());
    token.push('.');
    token.push_str(&b64url_encode(&sig));
    if token.len() > MAX_TOKEN_BYTES {
        return Err(format!("jwt_sign(): token too large ({} bytes, limit {})", token.len(), MAX_TOKEN_BYTES));
    }
    Ok(token)
}

fn string_set(v: &Value, what: &str) -> Result<Vec<String>, String> {
    match v {
        Value::Str(s) => Ok(vec![s.to_string()]),
        Value::Array(a) => a
            .to_vec()
            .into_iter()
            .map(|x| match x {
                Value::Str(s) => Ok(s.to_string()),
                _ => Err(format!("jwt_verify(): {} must be a string or a list of strings", what)),
            })
            .collect(),
        _ => Err(format!("jwt_verify(): {} must be a string or a list of strings", what)),
    }
}

fn decode_json_segment(seg: &str, what: &str) -> Result<SharedDict, String> {
    let raw = b64url_decode(seg).map_err(|e| format!("jwt_verify(): {}: {}", what, e))?;
    let text = std::str::from_utf8(&raw).map_err(|_| format!("jwt_verify(): {} is not UTF-8", what))?;
    match crate::json_lang::parse(text) {
        Ok(Value::Dict(d)) => Ok(d),
        Ok(_) => Err(format!("jwt_verify(): {} must be a JSON object", what)),
        Err(e) => Err(format!("jwt_verify(): {} is not valid JSON ({})", what, e)),
    }
}

fn verify_builtin(args: &[Value]) -> Result<Value, String> {
    const W: &str = "jwt_verify";
    if args.len() < 2 || args.len() > 3 {
        return Err("jwt_verify() expects (token, secret[, opts])".to_string());
    }
    let token = match &args[0] {
        Value::Str(s) => s.to_string(),
        _ => return Err("jwt_verify(): the token must be a string".to_string()),
    };
    let key = secret_bytes(&args[1], W)?;
    let opts = opts_dict(args.get(2), W)?;

    // ---- options (validated before the token is looked at) ----
    let allowed: Vec<Alg> = match opt(&opts, "algorithms") {
        None => vec![Alg::Hs256],
        Some(Value::Array(a)) => {
            let mut out = Vec::new();
            for x in a.to_vec() {
                match x {
                    Value::Str(s) if s.eq_ignore_ascii_case("none") => {} // never allowed; the header check reports it
                    Value::Str(s) => out.push(parse_alg(&s).map_err(|e| format!("jwt_verify(): opts.algorithms: {}", e))?),
                    _ => return Err("jwt_verify(): opts.algorithms must be a list of strings".to_string()),
                }
            }
            out
        }
        Some(_) => return Err("jwt_verify(): opts.algorithms must be a list of strings such as [\"HS256\"]".to_string()),
    };
    let leeway = match opt(&opts, "leeway") {
        None => DEFAULT_LEEWAY,
        Some(v) => num(&v).filter(|l| *l >= 0.0).ok_or("jwt_verify(): opts.leeway must be a non-negative number of seconds")?,
    };
    let require_exp = match opt(&opts, "require_exp") {
        None => true,
        Some(Value::Bool(b)) => b,
        Some(_) => return Err("jwt_verify(): opts.require_exp must be true or false".to_string()),
    };
    let now = match opt(&opts, "now") {
        None => now_secs(),
        Some(v) => num(&v).ok_or("jwt_verify(): opts.now must be a number of unix seconds")?,
    };
    let want_aud = opt(&opts, "aud").map(|v| string_set(&v, "opts.aud")).transpose()?;
    let want_iss = opt(&opts, "iss").map(|v| string_set(&v, "opts.iss")).transpose()?;

    // ---- shape ----
    if token.len() > MAX_TOKEN_BYTES {
        return Err(format!("jwt_verify(): token too large ({} bytes, limit {})", token.len(), MAX_TOKEN_BYTES));
    }
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 3 {
        return Err("jwt_verify(): malformed token (expected header.payload.signature)".to_string());
    }
    if parts[0].is_empty() || parts[1].is_empty() {
        return Err("jwt_verify(): malformed token (empty header or payload)".to_string());
    }

    // ---- header: algorithm decided by the allow list, never by the token alone ----
    let header = decode_json_segment(parts[0], "header")?;
    let alg_name = match header.get("alg") {
        Some(Value::Str(s)) => s.to_string(),
        Some(_) => return Err("jwt_verify(): header alg must be a string".to_string()),
        None => return Err("jwt_verify(): header has no alg".to_string()),
    };
    let alg = parse_alg(&alg_name).map_err(|e| format!("jwt_verify(): {}", e))?;
    if !allowed.contains(&alg) {
        let names: Vec<&str> = allowed.iter().map(|a| a.name()).collect();
        return Err(format!("jwt_verify(): algorithm {} is not allowed (allowed: {})", alg.name(), names.join(", ")));
    }
    if header.get("crit").is_some() {
        return Err("jwt_verify(): header has crit extensions, which are not supported".to_string());
    }
    check_key(alg, &key, W)?;

    // ---- signature (constant time) ----
    let sig = b64url_decode(parts[2]).map_err(|e| format!("jwt_verify(): signature: {}", e))?;
    let signing_input_len = parts[0].len() + 1 + parts[1].len();
    let expected = alg.mac(&key, &token.as_bytes()[..signing_input_len]);
    let same = {
        use subtle::ConstantTimeEq;
        sig.len() == expected.len() && bool::from(sig.ct_eq(&expected))
    };
    if !same {
        return Err("jwt_verify(): invalid signature".to_string());
    }

    // ---- claims (only trusted after the signature checks out) ----
    let claims = decode_json_segment(parts[1], "payload")?;
    let time_claim = |k: &str| -> Result<Option<f64>, String> {
        match claims.get(k) {
            None | Some(Value::Null) => Ok(None),
            Some(v) => num(&v).map(Some).ok_or(format!("jwt_verify(): claim {} must be a number of unix seconds", k)),
        }
    };
    match time_claim("exp")? {
        Some(exp) => {
            if now >= exp + leeway {
                return Err("jwt_verify(): token expired (exp is in the past)".to_string());
            }
        }
        None if require_exp => return Err("jwt_verify(): token has no exp claim (set opts.require_exp to false to allow that)".to_string()),
        None => {}
    }
    if let Some(nbf) = time_claim("nbf")? {
        if now < nbf - leeway {
            return Err("jwt_verify(): token not yet valid (nbf is in the future)".to_string());
        }
    }
    if let Some(iat) = time_claim("iat")? {
        if iat > now + leeway {
            return Err("jwt_verify(): token iat (issued at) is in the future".to_string());
        }
    }
    let token_aud = match claims.get("aud") {
        None | Some(Value::Null) => None,
        Some(v) => Some(string_set(&v, "claim aud").map_err(|_| "jwt_verify(): claim aud must be a string or a list of strings".to_string())?),
    };
    match (&want_aud, &token_aud) {
        (Some(want), Some(have)) => {
            if !have.iter().any(|a| want.contains(a)) {
                return Err("jwt_verify(): wrong audience (aud does not match opts.aud)".to_string());
            }
        }
        (Some(_), None) => return Err("jwt_verify(): token has no audience (aud) but opts.aud was given".to_string()),
        (None, Some(_)) => return Err("jwt_verify(): token has an audience (aud); pass the expected one as opts.aud".to_string()),
        (None, None) => {}
    }
    if let Some(want) = &want_iss {
        match claims.get("iss") {
            Some(Value::Str(s)) if want.iter().any(|w| w.as_str() == &*s) => {}
            Some(_) => return Err("jwt_verify(): wrong issuer (iss does not match opts.iss)".to_string()),
            None => return Err("jwt_verify(): token has no issuer (iss) but opts.iss was given".to_string()),
        }
    }
    Ok(Value::Dict(claims))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hex(b: &[u8]) -> String {
        b.iter().map(|x| format!("{:02x}", x)).collect()
    }

    #[test]
    fn hmac_sha2_rfc_4231_case_2() {
        let (k, d) = (b"Jefe", b"what do ya want for nothing?");
        assert_eq!(hex(&Alg::Hs256.mac(k, d)), "5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843");
        assert_eq!(
            hex(&Alg::Hs384.mac(k, d)),
            "af45d2e376484031617f78d2b58a6b1b9c7ef464f5a01b47e42ec3736322445e8e2240ca5e69e2c78b3239ecfab21649"
        );
        assert_eq!(
            hex(&Alg::Hs512.mac(k, d)),
            "164b7a7bfcf819e2e395fbe73b56e0a387bd64222e831fd610270cd7ea2505549758bf75c05a994a6d034f65f8f0e6fdcaeab1a34d4a6b4b636e070a38bce737"
        );
    }

    #[test]
    fn rfc7515_a1_signature_is_reproduced() {
        let key = b64url_decode("AyM1SysPpbyDfgZld3umj1qzKObwVMkoqQ-EstJQLr_T-1qS0gZH75aKtMN3Yj0iPS4hcgUuTwjAzZr1Z9CAow").unwrap();
        let input = "eyJ0eXAiOiJKV1QiLA0KICJhbGciOiJIUzI1NiJ9.eyJpc3MiOiJqb2UiLA0KICJleHAiOjEzMDA4MTkzODAsDQogImh0dHA6Ly9leGFtcGxlLmNvbS9pc19yb290Ijp0cnVlfQ";
        assert_eq!(b64url_encode(&Alg::Hs256.mac(&key, input.as_bytes())), "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk");
    }

    #[test]
    fn base64url_round_trip_and_strictness() {
        for len in 0..40usize {
            let data: Vec<u8> = (0..len as u8).map(|b| b.wrapping_mul(37).wrapping_add(11)).collect();
            let e = b64url_encode(&data);
            assert!(!e.contains('='));
            assert_eq!(b64url_decode(&e).unwrap(), data);
        }
        // RFC 4648 section 10 vectors, unpadded
        for (plain, enc) in [("", ""), ("f", "Zg"), ("fo", "Zm8"), ("foo", "Zm9v"), ("foob", "Zm9vYg"), ("fooba", "Zm9vYmE"), ("foobar", "Zm9vYmFy")] {
            assert_eq!(b64url_encode(plain.as_bytes()), enc);
            assert_eq!(b64url_decode(enc).unwrap(), plain.as_bytes());
        }
        for bad in ["Zg==", "Zm8=", "Z", "Zm9v Yg", "Zm+v", "Zm/v", "Zh", "Zm9"] {
            assert!(b64url_decode(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn crit_header_is_refused() {
        let key = vec![7u8; 32];
        let h = b64url_encode(br#"{"alg":"HS256","crit":["exp"],"exp":1}"#);
        let p = b64url_encode(br#"{"exp":99999999999}"#);
        let input = format!("{}.{}", h, p);
        let t = format!("{}.{}", input, b64url_encode(&Alg::Hs256.mac(&key, input.as_bytes())));
        let r = verify_builtin(&[Value::Str(t.into()), Value::Bytes(key)]);
        assert!(matches!(&r, Err(m) if m.contains("crit")), "{r:?}");
    }

    #[test]
    fn errors_do_not_echo_long_untrusted_text() {
        let h = b64url_encode(format!(r#"{{"alg":"{}"}}"#, "X".repeat(500)).as_bytes());
        let t = format!("{}.e30.c2ln", h);
        let r = verify_builtin(&[Value::Str(t.into()), Value::Str("k".repeat(32).into())]);
        match r {
            Err(m) => assert!(m.len() < 200, "{m}"),
            Ok(v) => panic!("{v:?}"),
        }
    }
}
