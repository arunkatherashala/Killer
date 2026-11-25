//! PostgreSQL client builtins (`src/pg_client.rs`) against an in-test FAKE server that speaks the frontend/backend
//! protocol v3 on 127.0.0.1 (ports 20000-32000). No real PostgreSQL is needed. The optional live test at the end
//! runs only when the environment variable KILLER_PG_URL points at a real server.
use hmac::{Hmac, Mac};
use killer_native::builtin::BuiltinFunctions;
use killer_native::security::{CapabilityScopeGuard, CapabilitySet};
use killer_native::value::{SharedArray, SharedDict, Value};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicU32, Ordering};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

static NEXT_PORT: AtomicU32 = AtomicU32::new(0);

/// A listener on a free port in 20000-32000 (the range CI allows).
fn listen() -> (TcpListener, u16) {
    let span = 12000u32;
    let seed = (std::process::id().wrapping_mul(7919)) % span;
    for _ in 0..2000 {
        let k = NEXT_PORT.fetch_add(1, Ordering::SeqCst);
        let port = 20000 + ((seed + k * 37) % span) as u16;
        if let Ok(l) = TcpListener::bind(("127.0.0.1", port)) {
            return (l, port);
        }
    }
    panic!("no free port in 20000-32000");
}

fn s(x: &str) -> Value {
    Value::Str(x.to_string())
}

fn call(name: &str, args: &[Value]) -> Result<Value, String> {
    BuiltinFunctions::call(name, args).map_err(|e| format!("{e}"))
}

fn dict(pairs: &[(&str, Value)]) -> Value {
    Value::Dict(SharedDict::from_pairs(pairs.iter().map(|(k, v)| (k.to_string(), v.clone())).collect()))
}

fn list(items: Vec<Value>) -> Value {
    Value::Array(SharedArray::new(items))
}

fn text(v: &Value) -> String {
    match v {
        Value::Str(s) => s.clone(),
        Value::Null => "null".to_string(),
        other => format!("{other:?}"),
    }
}

fn field(v: &Value, key: &str) -> Value {
    match v {
        Value::Dict(d) => d.get(key).unwrap_or_else(|| panic!("no key {key} in {v:?}")),
        other => panic!("not a dict: {other:?}"),
    }
}

fn items(v: &Value) -> Vec<Value> {
    match v {
        Value::Array(a) => a.to_vec(),
        other => panic!("not a list: {other:?}"),
    }
}

fn num(v: &Value) -> f64 {
    match v {
        Value::Number(n) => *n,
        Value::Integer(i) => *i as f64,
        other => panic!("not a number: {other:?}"),
    }
}

// ---------------------------------------------------------------------------------------------------------------
// fake server

struct Fake {
    s: TcpStream,
}

fn cstr(b: &mut Vec<u8>, x: &str) {
    b.extend_from_slice(x.as_bytes());
    b.push(0);
}

fn take_cstr(b: &[u8], i: &mut usize) -> String {
    let end = b[*i..].iter().position(|&c| c == 0).expect("cstring") + *i;
    let out = String::from_utf8_lossy(&b[*i..end]).to_string();
    *i = end + 1;
    out
}

fn be32(b: &[u8], i: &mut usize) -> i32 {
    let v = i32::from_be_bytes([b[*i], b[*i + 1], b[*i + 2], b[*i + 3]]);
    *i += 4;
    v
}

fn be16(b: &[u8], i: &mut usize) -> i16 {
    let v = i16::from_be_bytes([b[*i], b[*i + 1]]);
    *i += 2;
    v
}

fn b64(data: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    for c in data.chunks(3) {
        let n = (c[0] as u32) << 16 | (*c.get(1).unwrap_or(&0) as u32) << 8 | *c.get(2).unwrap_or(&0) as u32;
        out.push(T[(n >> 18) as usize & 63] as char);
        out.push(T[(n >> 12) as usize & 63] as char);
        out.push(if c.len() > 1 { T[(n >> 6) as usize & 63] as char } else { '=' });
        out.push(if c.len() > 2 { T[n as usize & 63] as char } else { '=' });
    }
    out
}

fn unb64(s: &str) -> Vec<u8> {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = Vec::new();
    let mut acc = 0u32;
    let mut bits = 0;
    for c in s.bytes() {
        if c == b'=' {
            break;
        }
        let v = T.iter().position(|&t| t == c).expect("base64") as u32;
        acc = (acc << 6) | v;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
        }
    }
    out
}

fn hmac(key: &[u8], data: &[u8]) -> Vec<u8> {
    let mut m = <Hmac<Sha256> as Mac>::new_from_slice(key).unwrap();
    m.update(data);
    m.finalize().into_bytes().to_vec()
}

/// How the fake server ends the SCRAM exchange.
#[derive(Clone, Copy, PartialEq)]
enum ScramEnd {
    Normal,
    BadServerSignature,
    SkipFinal,
}

impl Fake {
    fn new(s: TcpStream) -> Fake {
        s.set_read_timeout(Some(Duration::from_secs(60))).unwrap();
        Fake { s }
    }

    /// Read the StartupMessage; returns its parameters.
    fn startup(&mut self) -> HashMap<String, String> {
        let mut len = [0u8; 4];
        self.s.read_exact(&mut len).unwrap();
        let n = u32::from_be_bytes(len) as usize;
        let mut body = vec![0u8; n - 4];
        self.s.read_exact(&mut body).unwrap();
        let mut i = 0;
        assert_eq!(be32(&body, &mut i), 196608, "protocol 3.0");
        let mut out = HashMap::new();
        while i < body.len() && body[i] != 0 {
            let k = take_cstr(&body, &mut i);
            let v = take_cstr(&body, &mut i);
            out.insert(k, v);
        }
        out
    }

    /// Read one frontend message; None when the client closed the connection.
    fn read(&mut self) -> Option<(u8, Vec<u8>)> {
        let mut head = [0u8; 5];
        if self.s.read_exact(&mut head).is_err() {
            return None;
        }
        let n = u32::from_be_bytes([head[1], head[2], head[3], head[4]]) as usize;
        let mut body = vec![0u8; n - 4];
        self.s.read_exact(&mut body).ok()?;
        Some((head[0], body))
    }

    fn expect(&mut self, t: u8) -> Vec<u8> {
        let (got, body) = self.read().unwrap_or_else(|| panic!("client closed, expected '{}'", t as char));
        assert_eq!(got as char, t as char, "message type");
        body
    }

    fn send(&mut self, t: u8, body: &[u8]) {
        let mut m = vec![t];
        m.extend_from_slice(&((body.len() + 4) as u32).to_be_bytes());
        m.extend_from_slice(body);
        self.s.write_all(&m).unwrap();
    }

    fn raw(&mut self, bytes: &[u8]) {
        let _ = self.s.write_all(bytes);
        let _ = self.s.flush();
    }

    fn auth(&mut self, code: i32, extra: &[u8]) {
        let mut b = code.to_be_bytes().to_vec();
        b.extend_from_slice(extra);
        self.send(b'R', &b);
    }

    /// AuthenticationOk, a ParameterStatus, BackendKeyData and ReadyForQuery.
    fn finish_login(&mut self) {
        self.auth(0, &[]);
        let mut p = Vec::new();
        cstr(&mut p, "server_version");
        cstr(&mut p, "16.0");
        self.send(b'S', &p);
        let mut k = 1234i32.to_be_bytes().to_vec();
        k.extend_from_slice(&5678i32.to_be_bytes());
        self.send(b'K', &k);
        self.ready(b'I');
    }

    fn ready(&mut self, status: u8) {
        self.send(b'Z', &[status]);
    }

    fn row_description(&mut self, names: &[&str]) {
        let mut b = (names.len() as i16).to_be_bytes().to_vec();
        for n in names {
            cstr(&mut b, n);
            b.extend_from_slice(&0i32.to_be_bytes()); // table oid
            b.extend_from_slice(&0i16.to_be_bytes()); // attribute number
            b.extend_from_slice(&25i32.to_be_bytes()); // type oid (text)
            b.extend_from_slice(&(-1i16).to_be_bytes()); // type size
            b.extend_from_slice(&(-1i32).to_be_bytes()); // type modifier
            b.extend_from_slice(&0i16.to_be_bytes()); // format: text
        }
        self.send(b'T', &b);
    }

    fn data_row(&mut self, cols: &[Option<&str>]) {
        let mut b = (cols.len() as i16).to_be_bytes().to_vec();
        for c in cols {
            match c {
                None => b.extend_from_slice(&(-1i32).to_be_bytes()),
                Some(v) => {
                    b.extend_from_slice(&(v.len() as i32).to_be_bytes());
                    b.extend_from_slice(v.as_bytes());
                }
            }
        }
        self.send(b'D', &b);
    }

    fn complete(&mut self, tag: &str) {
        let mut b = Vec::new();
        cstr(&mut b, tag);
        self.send(b'C', &b);
    }

    fn error(&mut self, sqlstate: &str, message: &str) {
        let mut b = Vec::new();
        for (code, v) in [(b'S', "ERROR"), (b'V', "ERROR"), (b'C', sqlstate), (b'M', message)] {
            b.push(code);
            cstr(&mut b, v);
        }
        b.push(0);
        self.send(b'E', &b);
    }

    /// Server side of SCRAM-SHA-256 (RFC 5802 / 7677), verifying the client proof against `password`.
    /// Returns Err when the proof is wrong (after sending an ErrorResponse, like PostgreSQL).
    fn scram(&mut self, password: &str, end: ScramEnd) -> Result<(), String> {
        let mut mech = Vec::new();
        cstr(&mut mech, "SCRAM-SHA-256-PLUS");
        cstr(&mut mech, "SCRAM-SHA-256");
        mech.push(0);
        self.auth(10, &mech);
        let body = self.expect(b'p');
        let mut i = 0;
        let mechanism = take_cstr(&body, &mut i);
        assert_eq!(mechanism, "SCRAM-SHA-256", "client must pick the non-PLUS mechanism");
        let n = be32(&body, &mut i) as usize;
        let client_first = String::from_utf8(body[i..i + n].to_vec()).unwrap();
        let bare = client_first.strip_prefix("n,,").expect("gs2 header n,,").to_string();
        let cnonce = bare.split(',').find_map(|a| a.strip_prefix("r=")).expect("client nonce").to_string();
        assert!(cnonce.len() >= 18, "client nonce too short: {cnonce}");
        let nonce = format!("{cnonce}fakeServerNonce123");
        let salt = b"killer-pg-salt!!".to_vec();
        let iters = 4096u32;
        let server_first = format!("r={},s={},i={}", nonce, b64(&salt), iters);
        self.auth(11, server_first.as_bytes());
        let body = self.expect(b'p');
        let client_final = String::from_utf8(body).unwrap();
        let (without_proof, proof) = client_final.rsplit_once(",p=").expect("proof");
        assert_eq!(without_proof, format!("c=biws,r={nonce}"));
        let mut salted = [0u8; 32];
        pbkdf2::pbkdf2_hmac::<Sha256>(password.as_bytes(), &salt, iters, &mut salted);
        let client_key = hmac(&salted, b"Client Key");
        let stored_key = Sha256::digest(&client_key).to_vec();
        let auth_message = format!("{bare},{server_first},{without_proof}");
        let client_sig = hmac(&stored_key, auth_message.as_bytes());
        let proof = unb64(proof);
        let recovered: Vec<u8> = proof.iter().zip(client_sig.iter()).map(|(a, b)| a ^ b).collect();
        if Sha256::digest(&recovered).to_vec() != stored_key {
            self.error("28P01", "password authentication failed for user \"alice\"");
            return Err("bad proof".into());
        }
        let server_key = hmac(&salted, b"Server Key");
        let mut server_sig = hmac(&server_key, auth_message.as_bytes());
        match end {
            ScramEnd::Normal => {}
            ScramEnd::BadServerSignature => server_sig[0] ^= 1,
            ScramEnd::SkipFinal => return Ok(()),
        }
        self.auth(12, format!("v={}", b64(&server_sig)).as_bytes());
        Ok(())
    }
}

/// Start a fake server running `f` for one connection; returns the URL to connect to and the thread.
fn serve(f: impl FnOnce(Fake) + Send + 'static) -> (String, JoinHandle<()>) {
    let (l, port) = listen();
    l.set_nonblocking(true).unwrap();
    let h = std::thread::spawn(move || {
        let deadline = Instant::now() + Duration::from_secs(60);
        loop {
            match l.accept() {
                Ok((sock, _)) => {
                    sock.set_nonblocking(false).unwrap();
                    f(Fake::new(sock));
                    return;
                }
                Err(_) if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(5)),
                Err(e) => panic!("fake server: nobody connected: {e}"),
            }
        }
    });
    (format!("postgres://alice:pencil@127.0.0.1:{port}/shop"), h)
}

/// A server that logs in with SCRAM and then runs `f`.
fn serve_logged_in(f: impl FnOnce(&mut Fake) + Send + 'static) -> (String, JoinHandle<()>) {
    serve(move |mut fk| {
        let p = fk.startup();
        assert_eq!(p.get("user").map(String::as_str), Some("alice"));
        assert_eq!(p.get("database").map(String::as_str), Some("shop"));
        fk.scram("pencil", ScramEnd::Normal).unwrap();
        fk.finish_login();
        f(&mut fk);
    })
}

fn connect(url: &str) -> Value {
    call("pg_connect", &[s(url)]).unwrap_or_else(|e| panic!("connect failed: {e}"))
}

// ---------------------------------------------------------------------------------------------------------------
// tests

#[test]
fn scram_login_simple_query_rows_and_close() {
    let (url, h) = serve_logged_in(|fk| {
        let q = fk.expect(b'Q');
        assert_eq!(&q[..q.len() - 1], b"SELECT id, name FROM items");
        fk.row_description(&["id", "name"]);
        fk.data_row(&[Some("1"), Some("pen")]);
        fk.data_row(&[Some("2"), None]);
        fk.complete("SELECT 2");
        fk.ready(b'I');
        assert_eq!(fk.read().map(|m| m.0), Some(b'X'), "pg_close sends Terminate");
    });
    let c = connect(&url);
    let r = call("pg_query", &[c.clone(), s("SELECT id, name FROM items")]).unwrap();
    let cols: Vec<String> = items(&field(&r, "columns")).iter().map(text).collect();
    assert_eq!(cols, vec!["id", "name"]);
    let rows = items(&field(&r, "rows"));
    assert_eq!(rows.len(), 2);
    assert_eq!(text(&field(&rows[0], "id")), "1");
    assert_eq!(text(&field(&rows[0], "name")), "pen");
    assert_eq!(field(&rows[1], "name"), Value::Null, "SQL NULL becomes null");
    assert_eq!(text(&field(&r, "command")), "SELECT 2");
    assert_eq!(num(&field(&r, "row_count")), 2.0);
    call("pg_close", &[c.clone()]).unwrap();
    h.join().unwrap();
    let e = call("pg_query", &[c, s("SELECT 1")]).unwrap_err();
    assert!(e.contains("not an open"), "{e}");
}

#[test]
fn scram_wrong_password_reports_sqlstate() {
    let (url, h) = serve(|mut fk| {
        fk.startup();
        assert!(fk.scram("the-real-password", ScramEnd::Normal).is_err());
    });
    let e = call("pg_connect", &[s(&url)]).unwrap_err();
    assert!(e.contains("28P01") && e.contains("password authentication failed"), "{e}");
    h.join().unwrap();
}

#[test]
fn scram_bad_server_signature_is_rejected() {
    let (url, h) = serve(|mut fk| {
        fk.startup();
        fk.scram("pencil", ScramEnd::BadServerSignature).unwrap();
        fk.finish_login();
    });
    let e = call("pg_connect", &[s(&url)]).unwrap_err();
    assert!(e.contains("signature"), "{e}");
    h.join().unwrap();
}

#[test]
fn auth_ok_before_scram_final_is_rejected() {
    // a server (or a man in the middle) that skips the SCRAM server proof must not be trusted
    let (url, h) = serve(|mut fk| {
        fk.startup();
        fk.scram("pencil", ScramEnd::SkipFinal).unwrap();
        fk.finish_login();
    });
    let e = call("pg_connect", &[s(&url)]).unwrap_err();
    assert!(e.contains("SCRAM"), "{e}");
    h.join().unwrap();
}

#[test]
fn md5_and_cleartext_are_refused_by_default_and_no_password_is_sent() {
    for code in [5, 3] {
        let (url, h) = serve(move |mut fk| {
            fk.startup();
            fk.auth(code, if code == 5 { &[1, 2, 3, 4] } else { &[] });
            // the client must hang up instead of sending the password
            match fk.read() {
                None | Some((b'X', _)) => {}
                Some((t, body)) => panic!("client sent '{}' {:?} after a refused auth method", t as char, body),
            }
        });
        let e = call("pg_connect", &[s(&url)]).unwrap_err();
        let want = if code == 5 { "allow_md5" } else { "allow_cleartext" };
        assert!(e.contains(want), "{e}");
        h.join().unwrap();
    }
}

#[test]
fn md5_login_with_allow_md5() {
    let (l, port) = listen();
    let h = std::thread::spawn(move || {
        let (sock, _) = l.accept().unwrap();
        let mut fk = Fake::new(sock);
        let p = fk.startup();
        assert_eq!(p.get("user").map(String::as_str), Some("alice"));
        fk.auth(5, &[1, 2, 3, 4]);
        let body = fk.expect(b'p');
        // md5(md5("secret" + "alice") hex + salt) computed with Python's hashlib
        assert_eq!(&body[..body.len() - 1], b"md598a0412b9c31436fc53776e863350083");
        fk.finish_login();
    });
    let cfg = dict(&[
        ("host", s("127.0.0.1")),
        ("port", Value::Number(port as f64)),
        ("user", s("alice")),
        ("password", s("secret")),
        ("database", s("shop")),
    ]);
    let c = call("pg_connect", &[cfg, dict(&[("allow_md5", Value::Bool(true))])]).unwrap();
    h.join().unwrap();
    call("pg_close", &[c]).unwrap();
}

#[test]
fn cleartext_login_with_allow_cleartext_and_percent_encoded_url() {
    let (l, port) = listen();
    let h = std::thread::spawn(move || {
        let (sock, _) = l.accept().unwrap();
        let mut fk = Fake::new(sock);
        let p = fk.startup();
        assert_eq!(p.get("user").map(String::as_str), Some("bob@corp"));
        assert_eq!(p.get("database").map(String::as_str), Some("my db"));
        fk.auth(3, &[]);
        let body = fk.expect(b'p');
        assert_eq!(&body[..body.len() - 1], "p@ss:wörd/?".as_bytes());
        fk.finish_login();
    });
    let url = format!("postgresql://bob%40corp:p%40ss%3Aw%C3%B6rd%2F%3F@127.0.0.1:{port}/my%20db?sslmode=disable");
    let c = call("pg_connect", &[s(&url), dict(&[("allow_cleartext", Value::Bool(true))])]).unwrap();
    h.join().unwrap();
    call("pg_close", &[c]).unwrap();
}

#[test]
fn trust_auth_without_password_is_accepted() {
    let (url, h) = serve(|mut fk| {
        fk.startup();
        fk.finish_login();
    });
    let c = connect(&url);
    h.join().unwrap();
    call("pg_close", &[c]).unwrap();
}

#[test]
fn extended_query_binds_parameters_as_text() {
    let (url, h) = serve_logged_in(|fk| {
        let p = fk.expect(b'P');
        let mut i = 0;
        assert_eq!(take_cstr(&p, &mut i), "", "unnamed statement");
        assert_eq!(take_cstr(&p, &mut i), "SELECT name FROM items WHERE name = $1 OR id = $2 OR $3 IS NULL OR $4");
        let b = fk.expect(b'B');
        let mut i = 0;
        take_cstr(&b, &mut i);
        take_cstr(&b, &mut i);
        assert_eq!(be16(&b, &mut i), 0, "all parameters in text format");
        assert_eq!(be16(&b, &mut i), 4, "four parameters");
        let mut vals = Vec::new();
        for _ in 0..4 {
            let n = be32(&b, &mut i);
            if n < 0 {
                vals.push(None);
            } else {
                vals.push(Some(String::from_utf8(b[i..i + n as usize].to_vec()).unwrap()));
                i += n as usize;
            }
        }
        assert_eq!(
            vals,
            vec![Some("x'; DROP TABLE items; --".to_string()), Some("7".to_string()), None, Some("true".to_string())]
        );
        fk.expect(b'D');
        fk.expect(b'E');
        fk.expect(b'S');
        fk.send(b'1', &[]);
        fk.send(b'2', &[]);
        fk.row_description(&["name"]);
        fk.data_row(&[Some("x'; DROP TABLE items; --")]);
        fk.complete("SELECT 1");
        fk.ready(b'I');
        fk.read();
    });
    let c = connect(&url);
    let params = list(vec![s("x'; DROP TABLE items; --"), Value::Number(7.0), Value::Null, Value::Bool(true)]);
    let r = call(
        "pg_query",
        &[c.clone(), s("SELECT name FROM items WHERE name = $1 OR id = $2 OR $3 IS NULL OR $4"), params],
    )
    .unwrap();
    let rows = items(&field(&r, "rows"));
    assert_eq!(text(&field(&rows[0], "name")), "x'; DROP TABLE items; --");
    call("pg_close", &[c]).unwrap();
    h.join().unwrap();
}

#[test]
fn pg_exec_returns_affected_rows_and_transactions_pass_through() {
    let (url, h) = serve_logged_in(|fk| {
        for (sql, tag, status) in [
            ("BEGIN", "BEGIN", b'T'),
            ("UPDATE items SET n = 1", "UPDATE 3", b'T'),
            ("INSERT INTO items VALUES (1)", "INSERT 0 5", b'T'),
            ("COMMIT", "COMMIT", b'I'),
        ] {
            let q = fk.expect(b'Q');
            assert_eq!(&q[..q.len() - 1], sql.as_bytes());
            fk.complete(tag);
            fk.ready(status);
        }
        // extended protocol for pg_exec with parameters
        fk.expect(b'P');
        fk.expect(b'B');
        fk.expect(b'D');
        fk.expect(b'E');
        fk.expect(b'S');
        fk.send(b'1', &[]);
        fk.send(b'2', &[]);
        fk.send(b'n', &[]);
        fk.complete("DELETE 2");
        fk.ready(b'I');
        fk.read();
    });
    let c = connect(&url);
    let n = |sql: &str| num(&call("pg_exec", &[c.clone(), s(sql)]).unwrap());
    assert_eq!(n("BEGIN"), 0.0);
    assert_eq!(n("UPDATE items SET n = 1"), 3.0);
    assert_eq!(n("INSERT INTO items VALUES (1)"), 5.0);
    assert_eq!(n("COMMIT"), 0.0);
    let d = call("pg_exec", &[c.clone(), s("DELETE FROM items WHERE id = $1"), list(vec![s("9")])]).unwrap();
    assert_eq!(num(&d), 2.0);
    call("pg_close", &[c]).unwrap();
    h.join().unwrap();
}

#[test]
fn error_response_maps_to_sqlstate_and_connection_stays_usable() {
    let (url, h) = serve_logged_in(|fk| {
        fk.expect(b'Q');
        fk.error("42P01", "relation \"nope\" does not exist");
        fk.ready(b'I');
        fk.expect(b'Q');
        fk.row_description(&["one"]);
        fk.data_row(&[Some("1")]);
        fk.complete("SELECT 1");
        fk.ready(b'I');
        fk.read();
    });
    let c = connect(&url);
    let e = call("pg_query", &[c.clone(), s("SELECT * FROM nope")]).unwrap_err();
    assert!(e.contains("42P01") && e.contains("relation \"nope\" does not exist"), "{e}");
    let r = call("pg_query", &[c.clone(), s("SELECT 1 AS one")]).unwrap();
    assert_eq!(text(&field(&items(&field(&r, "rows"))[0], "one")), "1");
    call("pg_close", &[c]).unwrap();
    h.join().unwrap();
}

/// Bytes of one backend message with an explicit (possibly lying) length field.
fn frame(t: u8, len: u32, body: &[u8]) -> Vec<u8> {
    let mut m = vec![t];
    m.extend_from_slice(&len.to_be_bytes());
    m.extend_from_slice(body);
    m
}

#[test]
fn malformed_or_oversized_server_messages_are_errors_not_panics() {
    let mut cases: Vec<(&str, Vec<u8>)> = Vec::new();
    cases.push(("length below 4", frame(b'C', 3, b"")));
    cases.push(("oversized length", frame(b'D', 0x7fff_fff0, b"")));
    cases.push(("just over the 64 MiB cap", frame(b'D', (64 << 20) + 5, b"")));
    // DataRow claiming a 1000-byte column inside a 10-byte body
    let mut d = 1i16.to_be_bytes().to_vec();
    d.extend_from_slice(&1000i32.to_be_bytes());
    d.extend_from_slice(b"abcd");
    cases.push(("column longer than message", frame(b'D', (d.len() + 4) as u32, &d)));
    // DataRow with a negative length other than -1
    let mut d = 1i16.to_be_bytes().to_vec();
    d.extend_from_slice(&(-5i32).to_be_bytes());
    cases.push(("negative column length", frame(b'D', (d.len() + 4) as u32, &d)));
    // DataRow with more columns than the RowDescription announced
    let mut d = 3i16.to_be_bytes().to_vec();
    for _ in 0..3 {
        d.extend_from_slice(&1i32.to_be_bytes());
        d.push(b'x');
    }
    cases.push(("column count mismatch", frame(b'D', (d.len() + 4) as u32, &d)));
    // RowDescription cut short in the middle of a field
    let mut t = 2i16.to_be_bytes().to_vec();
    t.extend_from_slice(b"id");
    t.push(0);
    t.extend_from_slice(&[0, 0]);
    cases.push(("truncated RowDescription", frame(b'T', (t.len() + 4) as u32, &t)));
    // RowDescription with a negative field count
    let t = (-3i16).to_be_bytes().to_vec();
    cases.push(("negative field count", frame(b'T', (t.len() + 4) as u32, &t)));
    // ErrorResponse whose field is not NUL-terminated
    cases.push(("unterminated ErrorResponse", frame(b'E', 4 + 5, b"Mboom")));
    // CommandComplete without its terminating NUL
    cases.push(("unterminated CommandComplete", frame(b'C', 4 + 8, b"SELECT 1")));
    // a message type the client never expects
    cases.push(("unknown message type", frame(b'@', 4, b"")));
    // the server closes the connection in the middle of a message (header says 100 bytes, 10 arrive)
    cases.push(("closed mid-message", frame(b'D', 104, b"0123456789")));
    // the server closes right after the header
    cases.push(("closed after header byte", vec![b'D', 0, 0]));
    for (name, bytes) in cases {
        let (url, h) = serve_logged_in(move |fk| {
            fk.expect(b'Q');
            fk.row_description(&["a"]);
            fk.raw(&bytes);
            let _ = fk.s.shutdown(std::net::Shutdown::Write);
            // drain until the client hangs up
            while fk.read().is_some() {}
        });
        let c = connect(&url);
        let r = std::panic::catch_unwind(|| call("pg_query", &[c.clone(), s("SELECT a")]));
        let r = r.unwrap_or_else(|_| panic!("{name}: pg_query panicked"));
        let e = r.expect_err(name);
        assert!(e.contains("pg_query"), "{name}: {e}");
        // a broken connection stays broken but is a clean error
        let again = call("pg_query", &[c.clone(), s("SELECT a")]).expect_err(name);
        assert!(!again.is_empty());
        call("pg_close", &[c]).unwrap();
        h.join().unwrap_or_else(|_| panic!("{name}: fake server failed"));
    }
}

#[test]
fn malformed_handshakes_are_errors_not_panics() {
    // (name, what the server sends after the StartupMessage)
    type Script = Box<dyn FnOnce(&mut Fake) + Send>;
    let cases: Vec<(&str, Script)> = vec![
        ("closed before any reply", Box::new(|_fk: &mut Fake| {})),
        ("truncated auth code", Box::new(|fk: &mut Fake| fk.raw(&frame(b'R', 6, &[0, 0])))),
        ("unsupported auth (GSS)", Box::new(|fk: &mut Fake| fk.auth(7, &[]))),
        ("md5 without salt", Box::new(|fk: &mut Fake| fk.auth(5, &[1]))),
        ("SASL without SCRAM-SHA-256", Box::new(|fk: &mut Fake| {
            let mut m = Vec::new();
            cstr(&mut m, "SCRAM-SHA-256-PLUS");
            m.push(0);
            fk.auth(10, &m);
        })),
        ("SASLContinue before SASL", Box::new(|fk: &mut Fake| fk.auth(11, b"r=x,s=AAAA,i=4096"))),
        ("ReadyForQuery before auth", Box::new(|fk: &mut Fake| fk.ready(b'I'))),
        ("server nonce does not extend client nonce", Box::new(|fk: &mut Fake| {
            let mut m = Vec::new();
            cstr(&mut m, "SCRAM-SHA-256");
            m.push(0);
            fk.auth(10, &m);
            fk.expect(b'p');
            fk.auth(11, b"r=someoneElsesNonce,s=AAAA,i=4096");
        })),
        ("absurd iteration count", Box::new(|fk: &mut Fake| {
            let mut m = Vec::new();
            cstr(&mut m, "SCRAM-SHA-256");
            m.push(0);
            fk.auth(10, &m);
            let body = fk.expect(b'p');
            let txt = String::from_utf8_lossy(&body).to_string();
            let nonce = txt.split("r=").nth(1).unwrap().to_string();
            fk.auth(11, format!("r={nonce}srv,s=AAAA,i=4000000000").as_bytes());
        })),
        ("garbage server-first", Box::new(|fk: &mut Fake| {
            let mut m = Vec::new();
            cstr(&mut m, "SCRAM-SHA-256");
            m.push(0);
            fk.auth(10, &m);
            fk.expect(b'p');
            fk.auth(11, &[0xff, 0xfe, b',', b'=']);
        })),
        ("startup ErrorResponse", Box::new(|fk: &mut Fake| fk.error("3D000", "database \"shop\" does not exist"))),
    ];
    for (name, script) in cases {
        let (url, h) = serve(move |mut fk| {
            fk.startup();
            script(&mut fk);
            let _ = fk.s.shutdown(std::net::Shutdown::Write);
            while fk.read().is_some() {}
        });
        let r = std::panic::catch_unwind(|| call("pg_connect", &[s(&url)]));
        let r = r.unwrap_or_else(|_| panic!("{name}: pg_connect panicked"));
        let e = r.expect_err(name);
        assert!(e.contains("pg_connect"), "{name}: {e}");
        if name == "startup ErrorResponse" {
            assert!(e.contains("3D000"), "{e}");
        }
        h.join().unwrap_or_else(|_| panic!("{name}: fake server failed"));
    }
}

#[test]
fn bad_configuration_is_a_clear_error() {
    for (cfg, want) in [
        ("mysql://a@127.0.0.1/x", "postgres://"),
        ("postgres://a@127.0.0.1:99999/x", "port"),
        ("postgres://a@127.0.0.1:20001/x?sslmode=require", "TLS"),
        ("postgres://a@127.0.0.1:20001/x?sslmode=verify-full", "TLS"),
        ("postgres://a@127.0.0.1:20001/x?bogus=1", "bogus"),
        ("postgres://127.0.0.1:20001/x", "user"),
        ("postgres://a%zz@127.0.0.1:20001/x", "percent"),
    ] {
        let e = call("pg_connect", &[s(cfg)]).unwrap_err();
        assert!(e.contains(want), "{cfg}: {e}");
    }
    let e = call("pg_connect", &[dict(&[("host", s("127.0.0.1"))])]).unwrap_err();
    assert!(e.contains("user"), "{e}");
    let e = call("pg_query", &[Value::Number(424242.0), s("SELECT 1")]).unwrap_err();
    assert!(e.contains("not an open"), "{e}");
    let e = call("pg_query", &[Value::Number(424242.0)]).unwrap_err();
    assert!(e.contains("pg_query"), "{e}");
}

#[test]
fn params_must_be_plain_values_and_sql_without_nul() {
    let (url, h) = serve_logged_in(|fk| {
        while fk.read().is_some() {}
    });
    let c = connect(&url);
    let e = call("pg_query", &[c.clone(), s("SELECT $1"), list(vec![list(vec![])])]).unwrap_err();
    assert!(e.contains("parameter 1"), "{e}");
    let mut bad = String::from("SELECT 1");
    bad.push(char::from(0u8));
    let e = call("pg_query", &[c.clone(), s(&bad)]).unwrap_err();
    assert!(e.contains("NUL"), "{e}");
    call("pg_close", &[c]).unwrap();
    h.join().unwrap();
}

#[test]
fn network_capability_is_required() {
    let _g = CapabilityScopeGuard::install(CapabilitySet::restricted());
    for (name, args) in [
        ("pg_connect", vec![s("postgres://a@127.0.0.1:20001/x")]),
        ("pg_query", vec![Value::Number(1.0), s("SELECT 1")]),
        ("pg_exec", vec![Value::Number(1.0), s("SELECT 1")]),
        ("pg_close", vec![Value::Number(1.0)]),
    ] {
        let e = call(name, &args).unwrap_err();
        assert!(e.contains("Capability denied"), "{name}: {e}");
    }
}

#[test]
fn killer_script_end_to_end_with_try_catch() {
    let (url, h) = serve_logged_in(|fk| {
        fk.expect(b'Q');
        fk.row_description(&["greeting"]);
        fk.data_row(&[Some("hello")]);
        fk.complete("SELECT 1");
        fk.ready(b'I');
        fk.expect(b'P');
        fk.expect(b'B');
        fk.expect(b'D');
        fk.expect(b'E');
        fk.expect(b'S');
        fk.error("23505", "duplicate key value violates unique constraint");
        fk.ready(b'I');
        fk.read();
    });
    let src = format!(
        r#"c = pg_connect("{url}")
r = pg_query(c, "SELECT 'hello' AS greeting")
println(r["rows"][0]["greeting"], r["columns"][0], r["row_count"])
try {{
  pg_exec(c, "INSERT INTO t VALUES ($1)", ["a"])
}} catch e {{
  println("caught", contains(str(e), "23505"))
}}
pg_close(c)
"#
    );
    let dir = std::env::temp_dir().join(format!("killer_pg_{}_{}", std::process::id(), NEXT_PORT.fetch_add(1, Ordering::SeqCst)));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("t.killer");
    std::fs::write(&file, src).unwrap();
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_killer")).arg("run").arg(&file).output().unwrap();
    let _ = std::fs::remove_dir_all(&dir);
    let stdout = String::from_utf8_lossy(&out.stdout).replace("\r\n", "\n");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(out.status.success(), "{stderr}");
    assert_eq!(stdout.trim_end(), "hello greeting 1\ncaught true");
    h.join().unwrap();
}

/// Optional: a real PostgreSQL server. Skipped unless KILLER_PG_URL is set, for example
/// `KILLER_PG_URL=postgres://user:password@127.0.0.1:5432/postgres`.
#[test]
fn live_server_when_killer_pg_url_is_set() {
    let Ok(url) = std::env::var("KILLER_PG_URL") else {
        eprintln!("skipped: set KILLER_PG_URL to run against a real PostgreSQL server");
        return;
    };
    let c = call("pg_connect", &[s(&url)]).unwrap_or_else(|e| panic!("{e}"));
    let r = call("pg_query", &[c.clone(), s("SELECT 1 + 1 AS two, NULL AS nothing")]).unwrap();
    let row = &items(&field(&r, "rows"))[0];
    assert_eq!(text(&field(row, "two")), "2");
    assert_eq!(field(row, "nothing"), Value::Null);
    let r = call("pg_query", &[c.clone(), s("SELECT $1::text || '!' AS x"), list(vec![s("a'b")])]).unwrap();
    assert_eq!(text(&field(&items(&field(&r, "rows"))[0], "x")), "a'b!");
    call("pg_exec", &[c.clone(), s("CREATE TEMP TABLE killer_pg_t (id int)")]).unwrap();
    call("pg_exec", &[c.clone(), s("BEGIN")]).unwrap();
    let n = call("pg_exec", &[c.clone(), s("INSERT INTO killer_pg_t SELECT generate_series(1, 3)")]).unwrap();
    assert_eq!(num(&n), 3.0);
    call("pg_exec", &[c.clone(), s("ROLLBACK")]).unwrap();
    let r = call("pg_query", &[c.clone(), s("SELECT count(*) AS n FROM killer_pg_t")]).unwrap();
    assert_eq!(text(&field(&items(&field(&r, "rows"))[0], "n")), "0");
    let e = call("pg_query", &[c.clone(), s("SELECT * FROM killer_pg_missing_table")]).unwrap_err();
    assert!(e.contains("42P01"), "{e}");
    call("pg_close", &[c]).unwrap();
}
