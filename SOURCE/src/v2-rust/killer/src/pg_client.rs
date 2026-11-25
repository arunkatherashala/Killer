//! A minimal PostgreSQL client (frontend/backend protocol version 3) on `std::net`, with no new crates.
//!
//! ```killer
//! c = pg_connect("postgres://alice:secret@127.0.0.1:5432/shop")      # or a dict, see below
//! r = pg_query(c, "SELECT id, name FROM items WHERE id = $1", [7])  # parameters are bound, never pasted
//! println(r["columns"], r["rows"][0]["name"], r["command"], r["row_count"])
//! n = pg_exec(c, "UPDATE items SET n = n + 1")                       # affected rows
//! pg_exec(c, "BEGIN") ... pg_exec(c, "COMMIT")                       # transactions are plain statements
//! pg_close(c)
//! ```
//!
//! * `pg_connect(url_or_dict[, opts])`: the URL form is `postgres://user:password@host:port/database?sslmode=disable`
//!   (`postgresql://` too; user, password and database are percent-decoded). The dict form has `host` (default
//!   `localhost`), `port` (5432), `user` (required), `password`, `database` (default: the user name). Options, in
//!   the dict or in `opts`: `allow_md5`, `allow_cleartext` (both false by default), `connect_timeout_ms` (10000),
//!   `timeout_ms` (30000 per read; 0 waits forever), `application_name`, `sslmode`.
//! * Authentication: SCRAM-SHA-256 (RFC 5802 / RFC 7677) with the server signature checked; a server that reports
//!   success without proving it knows the password is refused. MD5 and cleartext passwords are refused unless
//!   `allow_md5` / `allow_cleartext` is set (MD5 is weak; cleartext sends the password itself). The password is used
//!   as its UTF-8 bytes (no SASLprep normalization; ASCII passwords are unaffected). `trust` auth needs no password.
//! * No TLS: the connection is plaintext. `sslmode=disable|allow|prefer` (or none) connect without TLS;
//!   `require`, `verify-ca` and `verify-full` are an error. Use a local TLS proxy (stunnel, pgbouncer, an SSH
//!   tunnel) for servers across a network, or plaintext only on trusted networks.
//! * `pg_query(conn, sql[, params])` returns `{"columns": [...], "rows": [{col: text or null}], "command": tag,
//!   "row_count": n}`. All values come back as text (`null` for SQL NULL). Without `params` the simple query
//!   protocol is used (several statements separated by `;` are allowed; the result of the last one is returned).
//!   With `params` (a list) the extended protocol binds `$1`, `$2`, ... as text, so values are never parsed as SQL.
//!   Parameters may be strings, numbers, booleans or null.
//! * `pg_exec(conn, sql[, params])` returns the affected row count from the command tag (0 for `BEGIN`, DDL, ...).
//! * A server error becomes a Killer error with the SQLSTATE: `pg_query(): ERROR: relation "x" does not exist
//!   (SQLSTATE 42P01)`; the connection stays usable. A protocol error (malformed or oversized message, the server
//!   hanging up, a timeout) breaks the connection: later calls fail until `pg_close`.
//! * Every length field from the server is bounds-checked; one message may be at most 64 MiB and one result at
//!   most 256 MiB. Every call needs the network capability. Handles are per thread.

use crate::error::VmError;
use crate::value::{SharedArray, SharedDict, Value};
use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256};
use std::cell::RefCell;
use std::collections::HashMap;
use std::io::{ErrorKind, Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::time::Duration;

pub const NAMES: &[&str] = &["pg_connect", "pg_query", "pg_exec", "pg_close"];

/// Largest single backend message accepted.
pub const MAX_MESSAGE: usize = 64 << 20;
/// Largest total size of the rows of one query result.
pub const MAX_RESULT: usize = 256 << 20;
/// Largest SCRAM iteration count accepted from a server (a hostile server could otherwise stall the client).
const MAX_SCRAM_ITERATIONS: u32 = 10_000_000;
const PROTOCOL_3: i32 = 196_608;

pub fn call(name: &str, args: &[Value]) -> Option<Result<Value, VmError>> {
    if !NAMES.contains(&name) {
        return None;
    }
    if let Err(e) = crate::security::require_network() {
        return Some(Err(e));
    }
    Some(dispatch(name, args))
}

fn err<T>(msg: impl Into<String>) -> Result<T, VmError> {
    Err(VmError::runtime_error(msg.into()))
}

/// Why an operation failed. `Server` errors leave the connection usable; `Broken` ones do not.
#[derive(Debug)]
enum PgErr {
    Server(String),
    Broken(String),
}

impl PgErr {
    fn text(&self) -> &str {
        match self {
            PgErr::Server(s) | PgErr::Broken(s) => s,
        }
    }
}

fn broken<T>(msg: impl Into<String>) -> Result<T, PgErr> {
    Err(PgErr::Broken(msg.into()))
}

struct PgConn {
    stream: Option<TcpStream>,
    broken: Option<String>,
}

thread_local! {
    static CONNS: RefCell<HashMap<u64, PgConn>> = RefCell::new(HashMap::new());
    static NEXT: RefCell<u64> = const { RefCell::new(1) };
}

fn dispatch(name: &str, args: &[Value]) -> Result<Value, VmError> {
    match name {
        "pg_connect" => {
            let cfg = config(args)?;
            let stream = connect(&cfg).map_err(|e| VmError::runtime_error(format!("pg_connect(): {}", e.text())))?;
            let id = NEXT.with(|n| {
                let mut n = n.borrow_mut();
                let id = *n;
                *n += 1;
                id
            });
            CONNS.with(|c| c.borrow_mut().insert(id, PgConn { stream: Some(stream), broken: None }));
            Ok(Value::Number(id as f64))
        }
        "pg_query" | "pg_exec" => {
            let id = handle(args, name)?;
            let sql = match args.get(1) {
                Some(Value::Str(s)) => s.to_string(),
                Some(other) => return err(format!("{}(): the SQL must be a string, got {}", name, other.type_name())),
                None => return err(format!("{}(conn, sql[, params]) is missing the SQL", name)),
            };
            if sql.as_bytes().contains(&0) {
                return err(format!("{}(): the SQL contains a NUL character", name));
            }
            let params = match args.get(2) {
                None | Some(Value::Null) => None,
                Some(Value::Array(a)) => Some(params_text(&a.to_vec(), name)?),
                Some(other) => return err(format!("{}(): params must be a list, got {}", name, other.type_name())),
            };
            let result = with_conn(id, name, |s| match &params {
                None => simple_query(s, &sql),
                Some(p) => extended_query(s, &sql, p),
            })?;
            if name == "pg_exec" {
                Ok(Value::Number(affected(&result.tag) as f64))
            } else {
                Ok(result.to_value())
            }
        }
        "pg_close" => {
            let id = handle(args, name)?;
            let conn = CONNS.with(|c| c.borrow_mut().remove(&id));
            match conn {
                Some(PgConn { stream: Some(mut s), .. }) => {
                    let _ = s.write_all(&[b'X', 0, 0, 0, 4]);
                    let _ = s.shutdown(std::net::Shutdown::Both);
                    Ok(Value::Bool(true))
                }
                Some(_) => Ok(Value::Bool(true)),
                None => Ok(Value::Bool(false)),
            }
        }
        _ => err(format!("{}(): unknown PostgreSQL builtin", name)),
    }
}

fn handle(args: &[Value], who: &str) -> Result<u64, VmError> {
    match args.first() {
        Some(Value::Number(n)) if *n >= 1.0 && n.fract() == 0.0 && *n < 9.0e15 => Ok(*n as u64),
        Some(Value::Integer(n)) if *n >= 1 => Ok(*n as u64),
        _ => err(format!("{}(): the first argument must be a connection from pg_connect()", who)),
    }
}

/// Run `f` on connection `id`. The connection is taken out of the table meanwhile and put back afterwards; a
/// protocol failure marks it broken (it is shut down and later calls fail with the original reason).
fn with_conn(id: u64, who: &str, f: impl FnOnce(&mut TcpStream) -> Result<QueryResult, PgErr>) -> Result<QueryResult, VmError> {
    let conn = CONNS.with(|c| c.borrow_mut().remove(&id));
    let mut conn = match conn {
        Some(c) => c,
        None => return err(format!("{}(): {} is not an open PostgreSQL connection", who, id)),
    };
    let outcome = match (&conn.broken, conn.stream.as_mut()) {
        (Some(why), _) => Err(PgErr::Broken(format!("the connection is broken ({}); pg_close it and connect again", why))),
        (None, Some(s)) => f(s),
        (None, None) => Err(PgErr::Broken("the connection is closed".to_string())),
    };
    if let Err(PgErr::Broken(why)) = &outcome {
        if conn.broken.is_none() {
            if let Some(s) = conn.stream.take() {
                let _ = s.shutdown(std::net::Shutdown::Both);
            }
            conn.broken = Some(why.clone());
        }
    }
    CONNS.with(|c| c.borrow_mut().insert(id, conn));
    outcome.map_err(|e| VmError::runtime_error(format!("{}(): {}", who, e.text())))
}

// ---------------------------------------------------------------------------------------------------------------
// configuration

#[derive(Debug, Clone, PartialEq)]
struct Config {
    host: String,
    port: u16,
    user: String,
    password: String,
    database: String,
    application_name: Option<String>,
    allow_md5: bool,
    allow_cleartext: bool,
    connect_timeout_ms: u64,
    timeout_ms: u64,
}

fn config(args: &[Value]) -> Result<Config, VmError> {
    let mut cfg = Config {
        host: "localhost".to_string(),
        port: 5432,
        user: String::new(),
        password: String::new(),
        database: String::new(),
        application_name: None,
        allow_md5: false,
        allow_cleartext: false,
        connect_timeout_ms: 10_000,
        timeout_ms: 30_000,
    };
    match args.first() {
        Some(Value::Str(url)) => parse_url(url, &mut cfg).map_err(|e| VmError::runtime_error(format!("pg_connect(): {}", e)))?,
        Some(Value::Dict(d)) => apply_dict(d, &mut cfg, true)?,
        Some(other) => return err(format!("pg_connect(): expected a postgres:// URL or a dict, got {}", other.type_name())),
        None => return err("pg_connect(url_or_dict[, opts]) is missing its argument"),
    }
    match args.get(1) {
        None | Some(Value::Null) => {}
        Some(Value::Dict(d)) => apply_dict(d, &mut cfg, false)?,
        Some(other) => return err(format!("pg_connect(): opts must be a dict, got {}", other.type_name())),
    }
    if cfg.user.is_empty() {
        return err("pg_connect(): a user name is required");
    }
    if cfg.database.is_empty() {
        cfg.database = cfg.user.clone();
    }
    for (what, v) in [("user", &cfg.user), ("database", &cfg.database), ("password", &cfg.password), ("host", &cfg.host)] {
        if v.as_bytes().contains(&0) {
            return err(format!("pg_connect(): the {} contains a NUL character", what));
        }
    }
    if let Some(a) = &cfg.application_name {
        if a.as_bytes().contains(&0) {
            return err("pg_connect(): the application_name contains a NUL character");
        }
    }
    Ok(cfg)
}

fn check_sslmode(mode: &str) -> Result<(), String> {
    match mode {
        "disable" | "allow" | "prefer" => Ok(()),
        "require" | "verify-ca" | "verify-full" => Err(format!(
            "sslmode={} needs TLS, which this client does not support; connect through a local TLS proxy (stunnel, pgbouncer, an SSH tunnel) or use sslmode=disable on a trusted network",
            mode
        )),
        other => Err(format!("unknown sslmode '{}'", other)),
    }
}

fn apply_dict(d: &SharedDict, cfg: &mut Config, connection: bool) -> Result<(), VmError> {
    let bad = |k: &str, want: &str| VmError::runtime_error(format!("pg_connect(): '{}' must be {}", k, want));
    for key in d.keys() {
        let v = d.get(&key).unwrap_or(Value::Null);
        let as_str = || match &v {
            Value::Str(s) => Ok(s.to_string()),
            _ => Err(bad(&key, "a string")),
        };
        let as_num = |max: u64| match &v {
            Value::Number(n) if n.is_finite() && *n >= 0.0 && n.fract() == 0.0 && *n <= max as f64 => Ok(*n as u64),
            Value::Integer(i) if *i >= 0 && (*i as u64) <= max => Ok(*i as u64),
            _ => Err(bad(&key, &format!("a whole number from 0 to {}", max))),
        };
        let as_bool = || match &v {
            Value::Bool(b) => Ok(*b),
            _ => Err(bad(&key, "true or false")),
        };
        match key.as_str() {
            "host" if connection => cfg.host = as_str()?,
            "port" if connection => {
                let p = as_num(65535)?;
                if p == 0 {
                    return Err(bad(&key, "a port from 1 to 65535"));
                }
                cfg.port = p as u16;
            }
            "user" if connection => cfg.user = as_str()?,
            "password" if connection => cfg.password = as_str()?,
            "database" | "dbname" if connection => cfg.database = as_str()?,
            "application_name" => cfg.application_name = Some(as_str()?),
            "sslmode" => check_sslmode(&as_str()?).map_err(|e| VmError::runtime_error(format!("pg_connect(): {}", e)))?,
            "allow_md5" => cfg.allow_md5 = as_bool()?,
            "allow_cleartext" => cfg.allow_cleartext = as_bool()?,
            "connect_timeout_ms" => cfg.connect_timeout_ms = as_num(3_600_000)?,
            "timeout_ms" => cfg.timeout_ms = as_num(86_400_000)?,
            other => return err(format!("pg_connect(): unknown option '{}'", other)),
        }
    }
    Ok(())
}

fn percent_decode(s: &str) -> Result<String, String> {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' {
            let hex = |c: u8| (c as char).to_digit(16);
            match (b.get(i + 1).and_then(|c| hex(*c)), b.get(i + 2).and_then(|c| hex(*c))) {
                (Some(h), Some(l)) => out.push((h * 16 + l) as u8),
                _ => return Err(format!("bad percent escape in '{}'", s)),
            }
            i += 3;
        } else {
            out.push(b[i]);
            i += 1;
        }
    }
    String::from_utf8(out).map_err(|_| format!("percent-decoded '{}' is not UTF-8", s))
}

fn parse_url(url: &str, cfg: &mut Config) -> Result<(), String> {
    let rest = url
        .strip_prefix("postgres://")
        .or_else(|| url.strip_prefix("postgresql://"))
        .ok_or_else(|| "the URL must start with postgres:// or postgresql://".to_string())?;
    let (main, query) = match rest.split_once('?') {
        Some((m, q)) => (m, Some(q)),
        None => (rest, None),
    };
    let (authority, path) = match main.split_once('/') {
        Some((a, p)) => (a, p),
        None => (main, ""),
    };
    let (userinfo, hostport) = match authority.rsplit_once('@') {
        Some((u, h)) => (Some(u), h),
        None => (None, authority),
    };
    if let Some(u) = userinfo {
        match u.split_once(':') {
            Some((name, pass)) => {
                cfg.user = percent_decode(name)?;
                cfg.password = percent_decode(pass)?;
            }
            None => cfg.user = percent_decode(u)?,
        }
    }
    let (host, port) = if let Some(after) = hostport.strip_prefix('[') {
        match after.split_once(']') {
            Some((h, p)) => (h.to_string(), p.strip_prefix(':')),
            None => return Err("unclosed '[' in the host".to_string()),
        }
    } else {
        match hostport.rsplit_once(':') {
            Some((h, p)) => (h.to_string(), Some(p)),
            None => (hostport.to_string(), None),
        }
    };
    if !host.is_empty() {
        cfg.host = percent_decode(&host)?;
    }
    if let Some(p) = port {
        match p.parse::<u16>() {
            Ok(n) if n > 0 => cfg.port = n,
            _ => return Err(format!("bad port '{}'", p)),
        }
    }
    if !path.is_empty() {
        cfg.database = percent_decode(path)?;
    }
    for pair in query.unwrap_or("").split('&').filter(|p| !p.is_empty()) {
        let (k, v) = pair.split_once('=').unwrap_or((pair, ""));
        let v = percent_decode(v)?;
        match k {
            "sslmode" => check_sslmode(&v)?,
            "application_name" => cfg.application_name = Some(v),
            "connect_timeout" => {
                let secs: u64 = v.parse().map_err(|_| format!("bad connect_timeout '{}'", v))?;
                cfg.connect_timeout_ms = secs.min(3600) * 1000;
            }
            other => return Err(format!("unknown URL parameter '{}'", other)),
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------------------------------------------
// wire format

/// A bounds-checked reader over one message body.
struct Cur<'a> {
    b: &'a [u8],
    i: usize,
    what: &'static str,
}

impl<'a> Cur<'a> {
    fn new(b: &'a [u8], what: &'static str) -> Self {
        Cur { b, i: 0, what }
    }
    fn short<T>(&self) -> Result<T, PgErr> {
        broken(format!("malformed {} message from the server (truncated)", self.what))
    }
    fn bytes(&mut self, n: usize) -> Result<&'a [u8], PgErr> {
        match self.i.checked_add(n) {
            Some(end) if end <= self.b.len() => {
                let out = &self.b[self.i..end];
                self.i = end;
                Ok(out)
            }
            _ => self.short(),
        }
    }
    fn u8(&mut self) -> Result<u8, PgErr> {
        Ok(self.bytes(1)?[0])
    }
    fn i16(&mut self) -> Result<i16, PgErr> {
        let b = self.bytes(2)?;
        Ok(i16::from_be_bytes([b[0], b[1]]))
    }
    fn i32(&mut self) -> Result<i32, PgErr> {
        let b = self.bytes(4)?;
        Ok(i32::from_be_bytes([b[0], b[1], b[2], b[3]]))
    }
    fn cstr(&mut self) -> Result<String, PgErr> {
        let rest = &self.b[self.i.min(self.b.len())..];
        match rest.iter().position(|&c| c == 0) {
            Some(n) => {
                let s = String::from_utf8_lossy(&rest[..n]).to_string();
                self.i += n + 1;
                Ok(s)
            }
            None => broken(format!("malformed {} message from the server (missing string terminator)", self.what)),
        }
    }
    fn rest(&mut self) -> &'a [u8] {
        let out = &self.b[self.i.min(self.b.len())..];
        self.i = self.b.len();
        out
    }
}

fn io_err(e: std::io::Error) -> PgErr {
    match e.kind() {
        ErrorKind::UnexpectedEof => PgErr::Broken("the server closed the connection".to_string()),
        ErrorKind::WouldBlock | ErrorKind::TimedOut => PgErr::Broken("timed out waiting for the server".to_string()),
        _ => PgErr::Broken(format!("network error: {}", e)),
    }
}

/// Read one backend message: (type byte, body). Never allocates more than the bytes that actually arrive.
fn read_msg(s: &mut TcpStream) -> Result<(u8, Vec<u8>), PgErr> {
    let mut head = [0u8; 5];
    s.read_exact(&mut head).map_err(io_err)?;
    let len = u32::from_be_bytes([head[1], head[2], head[3], head[4]]) as usize;
    if len < 4 {
        return broken(format!("malformed message from the server (length {} is below 4)", len));
    }
    let body_len = len - 4;
    if body_len > MAX_MESSAGE {
        return broken(format!("the server sent a {} byte message; the limit is {} bytes", body_len, MAX_MESSAGE));
    }
    let mut body = Vec::with_capacity(body_len.min(64 * 1024));
    let got = Read::by_ref(s).take(body_len as u64).read_to_end(&mut body).map_err(io_err)?;
    if got != body_len {
        return broken("the server closed the connection in the middle of a message");
    }
    Ok((head[0], body))
}

/// A frontend message with type byte `t`.
fn msg(t: u8, body: &[u8]) -> Result<Vec<u8>, PgErr> {
    let len = u32::try_from(body.len() + 4).map_err(|_| PgErr::Server("the message is too large to send".to_string()))?;
    if body.len() > i32::MAX as usize - 4 {
        return Err(PgErr::Server("the message is too large to send".to_string()));
    }
    let mut m = Vec::with_capacity(body.len() + 5);
    m.push(t);
    m.extend_from_slice(&len.to_be_bytes());
    m.extend_from_slice(body);
    Ok(m)
}

fn put_cstr(b: &mut Vec<u8>, s: &str) {
    b.extend_from_slice(s.as_bytes());
    b.push(0);
}

fn send(s: &mut TcpStream, bytes: &[u8]) -> Result<(), PgErr> {
    s.write_all(bytes).and_then(|_| s.flush()).map_err(io_err)
}

/// The text of an ErrorResponse: `ERROR: message (SQLSTATE xxxxx) detail / hint`.
fn server_error(body: &[u8]) -> Result<String, PgErr> {
    let mut c = Cur::new(body, "ErrorResponse");
    let mut fields: HashMap<u8, String> = HashMap::new();
    loop {
        let code = c.u8()?;
        if code == 0 {
            break;
        }
        let v = c.cstr()?;
        fields.insert(code, v);
    }
    let get = |k: u8| fields.get(&k).cloned().unwrap_or_default();
    let severity = fields.get(&b'V').or_else(|| fields.get(&b'S')).cloned().unwrap_or_else(|| "ERROR".to_string());
    let mut out = format!("{}: {}", severity, get(b'M'));
    let state = get(b'C');
    if !state.is_empty() {
        out.push_str(&format!(" (SQLSTATE {})", state));
    }
    if let Some(d) = fields.get(&b'D') {
        out.push_str(&format!("; detail: {}", d));
    }
    if let Some(h) = fields.get(&b'H') {
        out.push_str(&format!("; hint: {}", h));
    }
    Ok(out)
}

// ---------------------------------------------------------------------------------------------------------------
// connection and authentication

fn connect(cfg: &Config) -> Result<TcpStream, PgErr> {
    let addrs: Vec<_> = (cfg.host.as_str(), cfg.port)
        .to_socket_addrs()
        .map_err(|e| PgErr::Broken(format!("cannot resolve '{}': {}", cfg.host, e)))?
        .collect();
    if addrs.is_empty() {
        return broken(format!("cannot resolve '{}'", cfg.host));
    }
    let mut last = String::new();
    let mut stream = None;
    for a in &addrs {
        let r = if cfg.connect_timeout_ms == 0 {
            TcpStream::connect(a)
        } else {
            TcpStream::connect_timeout(a, Duration::from_millis(cfg.connect_timeout_ms))
        };
        match r {
            Ok(s) => {
                stream = Some(s);
                break;
            }
            Err(e) => last = format!("cannot connect to {}: {}", a, e),
        }
    }
    let mut s = match stream {
        Some(s) => s,
        None => return broken(last),
    };
    let _ = s.set_nodelay(true);
    let t = if cfg.timeout_ms == 0 { None } else { Some(Duration::from_millis(cfg.timeout_ms)) };
    s.set_read_timeout(t).map_err(io_err)?;
    s.set_write_timeout(t).map_err(io_err)?;
    let r = login(&mut s, cfg);
    if r.is_err() {
        let _ = s.write_all(&[b'X', 0, 0, 0, 4]);
        let _ = s.shutdown(std::net::Shutdown::Both);
    }
    r.map(|_| s)
}

fn startup_message(cfg: &Config) -> Vec<u8> {
    let mut body = PROTOCOL_3.to_be_bytes().to_vec();
    let mut pairs = vec![("user", cfg.user.as_str()), ("database", cfg.database.as_str()), ("client_encoding", "UTF8")];
    if let Some(a) = &cfg.application_name {
        pairs.push(("application_name", a.as_str()));
    }
    for (k, v) in pairs {
        put_cstr(&mut body, k);
        put_cstr(&mut body, v);
    }
    body.push(0);
    let mut m = ((body.len() + 4) as u32).to_be_bytes().to_vec();
    m.extend_from_slice(&body);
    m
}

fn login(s: &mut TcpStream, cfg: &Config) -> Result<(), PgErr> {
    send(s, &startup_message(cfg))?;
    let mut scram: Option<Scram> = None;
    let mut scram_verified = false;
    let mut authenticated = false;
    loop {
        let (t, body) = read_msg(s)?;
        match t {
            b'R' => {
                let mut c = Cur::new(&body, "Authentication");
                let code = c.i32()?;
                match code {
                    0 => {
                        if scram.is_some() && !scram_verified {
                            return broken("the server ended SCRAM authentication without proving it knows the password");
                        }
                        authenticated = true;
                    }
                    3 => {
                        if !cfg.allow_cleartext {
                            return broken("the server asks for the password in cleartext; refused (set allow_cleartext: true to permit it, only over a trusted network)");
                        }
                        need_password(cfg)?;
                        let mut b = Vec::new();
                        put_cstr(&mut b, &cfg.password);
                        send(s, &msg(b'p', &b)?)?;
                    }
                    5 => {
                        let salt = c.bytes(4)?;
                        if !cfg.allow_md5 {
                            return broken("the server asks for MD5 password authentication, which is weak; refused (set allow_md5: true to permit it, or switch the server to scram-sha-256)");
                        }
                        need_password(cfg)?;
                        let mut b = Vec::new();
                        put_cstr(&mut b, &md5_password(&cfg.user, &cfg.password, salt));
                        send(s, &msg(b'p', &b)?)?;
                    }
                    10 => {
                        if scram.is_some() {
                            return broken("the server restarted SASL authentication");
                        }
                        let mut mechs = Vec::new();
                        loop {
                            let m = c.cstr()?;
                            if m.is_empty() {
                                break;
                            }
                            mechs.push(m);
                        }
                        if !mechs.iter().any(|m| m == "SCRAM-SHA-256") {
                            return broken(format!("the server offers only unsupported SASL mechanisms: {}", mechs.join(", ")));
                        }
                        need_password(cfg)?;
                        let st = Scram::new().map_err(PgErr::Broken)?;
                        let first = st.client_first();
                        let mut b = Vec::new();
                        put_cstr(&mut b, "SCRAM-SHA-256");
                        b.extend_from_slice(&(first.len() as i32).to_be_bytes());
                        b.extend_from_slice(first.as_bytes());
                        send(s, &msg(b'p', &b)?)?;
                        scram = Some(st);
                    }
                    11 => {
                        let st = match scram.as_mut() {
                            Some(st) if st.server_first.is_none() => st,
                            _ => return broken("unexpected SASLContinue message from the server"),
                        };
                        let fin = st.client_final(c.rest(), cfg.password.as_bytes()).map_err(PgErr::Broken)?;
                        send(s, &msg(b'p', fin.as_bytes())?)?;
                    }
                    12 => {
                        let st = match scram.as_ref() {
                            Some(st) if st.server_first.is_some() && !scram_verified => st,
                            _ => return broken("unexpected SASLFinal message from the server"),
                        };
                        st.verify_server_final(c.rest()).map_err(PgErr::Broken)?;
                        scram_verified = true;
                    }
                    2 | 6 | 7 | 8 | 9 => return broken(format!("the server asks for an unsupported authentication method (code {}: Kerberos/GSSAPI/SSPI)", code)),
                    other => return broken(format!("the server asks for an unknown authentication method (code {})", other)),
                }
            }
            b'E' => return broken(server_error(&body)?),
            b'N' | b'S' | b'K' | b'v' => {}
            b'Z' => {
                if !authenticated {
                    return broken("the server reported ready before authentication finished");
                }
                return Ok(());
            }
            other => return broken(format!("unexpected message '{}' from the server during login", printable(other))),
        }
    }
}

fn need_password(cfg: &Config) -> Result<(), PgErr> {
    if cfg.password.is_empty() {
        return broken("the server asks for a password but none was given");
    }
    Ok(())
}

fn printable(t: u8) -> String {
    if t.is_ascii_graphic() {
        (t as char).to_string()
    } else {
        format!("0x{:02x}", t)
    }
}

// ---------------------------------------------------------------------------------------------------------------
// SCRAM-SHA-256 (RFC 5802, RFC 7677)

struct Scram {
    client_nonce: String,
    /// user name sent in client-first (PostgreSQL ignores it and uses the startup user; libpq sends it empty)
    user: String,
    server_first: Option<String>,
    auth_message: String,
    salted: [u8; 32],
}

fn hmac256(key: &[u8], data: &[u8]) -> Result<[u8; 32], String> {
    let mut m = <Hmac<Sha256> as Mac>::new_from_slice(key).map_err(|_| "HMAC key error".to_string())?;
    m.update(data);
    Ok(m.finalize().into_bytes().into())
}

impl Scram {
    fn new() -> Result<Scram, String> {
        let mut raw = [0u8; 18];
        crate::os_random::fill(&mut raw)?;
        Ok(Scram::with_nonce(&b64encode(&raw), ""))
    }

    fn with_nonce(nonce: &str, user: &str) -> Scram {
        Scram { client_nonce: nonce.to_string(), user: user.to_string(), server_first: None, auth_message: String::new(), salted: [0; 32] }
    }

    fn client_first_bare(&self) -> String {
        format!("n={},r={}", self.user, self.client_nonce)
    }

    fn client_first(&self) -> String {
        format!("n,,{}", self.client_first_bare())
    }

    /// Handle server-first; return client-final (with the proof).
    fn client_final(&mut self, server_first: &[u8], password: &[u8]) -> Result<String, String> {
        let sf = std::str::from_utf8(server_first).map_err(|_| "malformed SCRAM server-first message (not UTF-8)".to_string())?;
        let mut nonce = None;
        let mut salt = None;
        let mut iters = None;
        for attr in sf.split(',') {
            if let Some(v) = attr.strip_prefix("r=") {
                nonce = Some(v);
            } else if let Some(v) = attr.strip_prefix("s=") {
                salt = Some(b64decode(v).ok_or_else(|| "malformed SCRAM salt".to_string())?);
            } else if let Some(v) = attr.strip_prefix("i=") {
                iters = Some(v.parse::<u32>().map_err(|_| format!("malformed SCRAM iteration count '{}'", v))?);
            } else if attr.starts_with("m=") {
                return Err("the server requires an unsupported SCRAM extension".to_string());
            }
        }
        let (nonce, salt, iters) = match (nonce, salt, iters) {
            (Some(n), Some(s), Some(i)) => (n, s, i),
            _ => return Err("malformed SCRAM server-first message (missing r=, s= or i=)".to_string()),
        };
        if !nonce.starts_with(&self.client_nonce) || nonce.len() <= self.client_nonce.len() {
            return Err("the SCRAM server nonce does not extend the client nonce".to_string());
        }
        if nonce.bytes().any(|c| !(0x21..=0x7e).contains(&c) || c == b',') {
            return Err("malformed SCRAM server nonce".to_string());
        }
        if iters == 0 || iters > MAX_SCRAM_ITERATIONS {
            return Err(format!("the SCRAM iteration count {} is outside 1..={}", iters, MAX_SCRAM_ITERATIONS));
        }
        pbkdf2::pbkdf2_hmac::<Sha256>(password, &salt, iters, &mut self.salted);
        let client_key = hmac256(&self.salted, b"Client Key")?;
        let stored_key: [u8; 32] = Sha256::digest(client_key).into();
        let without_proof = format!("c=biws,r={}", nonce);
        self.auth_message = format!("{},{},{}", self.client_first_bare(), sf, without_proof);
        let sig = hmac256(&stored_key, self.auth_message.as_bytes())?;
        let mut proof = [0u8; 32];
        for i in 0..32 {
            proof[i] = client_key[i] ^ sig[i];
        }
        self.server_first = Some(sf.to_string());
        Ok(format!("{},p={}", without_proof, b64encode(&proof)))
    }

    /// Check server-final (`v=<server signature>`), in constant time.
    fn verify_server_final(&self, data: &[u8]) -> Result<(), String> {
        use subtle::ConstantTimeEq;
        let text = std::str::from_utf8(data).map_err(|_| "malformed SCRAM server-final message".to_string())?;
        if let Some(e) = text.strip_prefix("e=") {
            return Err(format!("SCRAM authentication failed on the server: {}", e));
        }
        let v = text.split(',').find_map(|a| a.strip_prefix("v=")).ok_or_else(|| "malformed SCRAM server-final message".to_string())?;
        let got = b64decode(v).ok_or_else(|| "malformed SCRAM server signature".to_string())?;
        let server_key = hmac256(&self.salted, b"Server Key")?;
        let want = hmac256(&server_key, self.auth_message.as_bytes())?;
        if got.len() != 32 || !bool::from(got.as_slice().ct_eq(&want)) {
            return Err("the SCRAM server signature is wrong (the server does not know the password, or the connection was tampered with)".to_string());
        }
        Ok(())
    }
}

const B64: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

fn b64encode(data: &[u8]) -> String {
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for c in data.chunks(3) {
        let b1 = c.get(1).copied().unwrap_or(0) as u32;
        let b2 = c.get(2).copied().unwrap_or(0) as u32;
        let n = (c[0] as u32) << 16 | b1 << 8 | b2;
        out.push(B64[(n >> 18) as usize & 63] as char);
        out.push(B64[(n >> 12) as usize & 63] as char);
        out.push(if c.len() > 1 { B64[(n >> 6) as usize & 63] as char } else { '=' });
        out.push(if c.len() > 2 { B64[n as usize & 63] as char } else { '=' });
    }
    out
}

fn b64decode(s: &str) -> Option<Vec<u8>> {
    let body = s.trim_end_matches('=');
    if s.len() - body.len() > 2 {
        return None;
    }
    let mut out = Vec::with_capacity(body.len() * 3 / 4);
    let mut acc = 0u32;
    let mut bits = 0;
    for c in body.bytes() {
        let v = B64.iter().position(|&t| t == c)? as u32;
        acc = (acc << 6) | v;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
        }
    }
    Some(out)
}

// ---------------------------------------------------------------------------------------------------------------
// MD5 (RFC 1321), only for the legacy `md5` auth method behind allow_md5

fn md5(data: &[u8]) -> [u8; 16] {
    const S: [u32; 64] = [
        7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 5, 9, 14, 20, 5, 9, 14, 20, 5, 9, 14, 20, 5, 9, 14, 20,
        4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21,
    ];
    let k: Vec<u32> = (0..64).map(|i| ((i as f64 + 1.0).sin().abs() * 4_294_967_296.0) as u32).collect();
    let mut msg = data.to_vec();
    let bit_len = (data.len() as u64).wrapping_mul(8);
    msg.push(0x80);
    while msg.len() % 64 != 56 {
        msg.push(0);
    }
    msg.extend_from_slice(&bit_len.to_le_bytes());
    let (mut a0, mut b0, mut c0, mut d0) = (0x6745_2301u32, 0xefcd_ab89u32, 0x98ba_dcfeu32, 0x1032_5476u32);
    for chunk in msg.chunks(64) {
        let mut m = [0u32; 16];
        for (i, w) in m.iter_mut().enumerate() {
            *w = u32::from_le_bytes([chunk[i * 4], chunk[i * 4 + 1], chunk[i * 4 + 2], chunk[i * 4 + 3]]);
        }
        let (mut a, mut b, mut c, mut d) = (a0, b0, c0, d0);
        for i in 0..64 {
            let (f, g) = match i / 16 {
                0 => ((b & c) | (!b & d), i),
                1 => ((d & b) | (!d & c), (5 * i + 1) % 16),
                2 => (b ^ c ^ d, (3 * i + 5) % 16),
                _ => (c ^ (b | !d), (7 * i) % 16),
            };
            let f2 = f.wrapping_add(a).wrapping_add(k[i]).wrapping_add(m[g]);
            a = d;
            d = c;
            c = b;
            b = b.wrapping_add(f2.rotate_left(S[i]));
        }
        a0 = a0.wrapping_add(a);
        b0 = b0.wrapping_add(b);
        c0 = c0.wrapping_add(c);
        d0 = d0.wrapping_add(d);
    }
    let mut out = [0u8; 16];
    out[..4].copy_from_slice(&a0.to_le_bytes());
    out[4..8].copy_from_slice(&b0.to_le_bytes());
    out[8..12].copy_from_slice(&c0.to_le_bytes());
    out[12..].copy_from_slice(&d0.to_le_bytes());
    out
}

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{:02x}", x)).collect()
}

/// `"md5" + md5hex(md5hex(password + user) + salt)`, as PostgreSQL expects.
fn md5_password(user: &str, password: &str, salt: &[u8]) -> String {
    let inner = hex(&md5(format!("{}{}", password, user).as_bytes()));
    let mut outer = inner.into_bytes();
    outer.extend_from_slice(salt);
    format!("md5{}", hex(&md5(&outer)))
}

// ---------------------------------------------------------------------------------------------------------------
// queries

#[derive(Debug, Default)]
struct QueryResult {
    columns: Vec<String>,
    rows: Vec<Vec<Option<String>>>,
    tag: String,
}

impl QueryResult {
    fn to_value(&self) -> Value {
        let columns: Vec<Value> = self.columns.iter().map(|c| Value::Str(c.clone().into())).collect();
        let rows: Vec<Value> = self
            .rows
            .iter()
            .map(|r| {
                let pairs = self
                    .columns
                    .iter()
                    .zip(r.iter())
                    .map(|(c, v)| (c.clone(), v.as_ref().map_or(Value::Null, |s| Value::Str(s.clone().into()))))
                    .collect();
                Value::Dict(SharedDict::from_pairs(pairs))
            })
            .collect();
        let count = if self.columns.is_empty() { affected(&self.tag) } else { self.rows.len() as u64 };
        Value::Dict(SharedDict::from_pairs(vec![
            ("columns".to_string(), Value::Array(SharedArray::new(columns))),
            ("rows".to_string(), Value::Array(SharedArray::new(rows))),
            ("command".to_string(), Value::Str(self.tag.clone().into())),
            ("row_count".to_string(), Value::Number(count as f64)),
        ]))
    }
}

/// The row count at the end of a command tag (`UPDATE 3`, `INSERT 0 5`, `SELECT 2`); 0 when there is none.
fn affected(tag: &str) -> u64 {
    let mut words = tag.split_whitespace();
    match words.next() {
        Some("INSERT" | "UPDATE" | "DELETE" | "SELECT" | "MERGE" | "MOVE" | "FETCH" | "COPY") => {
            tag.split_whitespace().last().and_then(|w| w.parse().ok()).unwrap_or(0)
        }
        _ => 0,
    }
}

fn params_text(params: &[Value], who: &str) -> Result<Vec<Option<String>>, VmError> {
    if params.len() > i16::MAX as usize {
        return err(format!("{}(): at most {} parameters", who, i16::MAX));
    }
    params
        .iter()
        .enumerate()
        .map(|(i, v)| match v {
            Value::Null => Ok(None),
            Value::Str(s) => Ok(Some(s.to_string())),
            Value::Bool(b) => Ok(Some(b.to_string())),
            Value::Integer(n) => Ok(Some(n.to_string())),
            Value::Number(n) if n.is_nan() => Ok(Some("NaN".to_string())),
            Value::Number(n) if n.is_infinite() => Ok(Some(if *n > 0.0 { "Infinity" } else { "-Infinity" }.to_string())),
            Value::Number(n) if n.fract() == 0.0 && n.abs() < 9.0e15 => Ok(Some(format!("{}", *n as i64))),
            Value::Number(n) => Ok(Some(format!("{}", n))),
            other => err(format!("{}(): parameter {} must be a string, number, boolean or null, got {}", who, i + 1, other.type_name())),
        })
        .collect()
}

/// Collects the responses of one query cycle until ReadyForQuery.
struct Collector {
    current: QueryResult,
    last: Option<QueryResult>,
    have_description: bool,
    bytes: usize,
    error: Option<String>,
}

impl Collector {
    fn new() -> Self {
        Collector { current: QueryResult::default(), last: None, have_description: false, bytes: 0, error: None }
    }

    /// Handle one message; Ok(true) once ReadyForQuery arrived.
    fn take(&mut self, s: &mut TcpStream, t: u8, body: &[u8]) -> Result<bool, PgErr> {
        match t {
            b'T' => {
                let mut c = Cur::new(body, "RowDescription");
                let n = c.i16()?;
                if n < 0 {
                    return broken("malformed RowDescription message from the server (negative field count)");
                }
                let mut cols = Vec::with_capacity(n as usize);
                for _ in 0..n {
                    cols.push(c.cstr()?);
                    c.bytes(18)?; // table oid, column number, type oid, type size, type modifier, format code
                }
                self.current = QueryResult { columns: cols, rows: Vec::new(), tag: String::new() };
                self.have_description = true;
            }
            b'D' => {
                if !self.have_description {
                    return broken("the server sent a DataRow without a RowDescription");
                }
                self.bytes += body.len();
                if self.bytes > MAX_RESULT {
                    return broken(format!("the result is larger than {} bytes", MAX_RESULT));
                }
                let mut c = Cur::new(body, "DataRow");
                let n = c.i16()?;
                if n < 0 || n as usize != self.current.columns.len() {
                    return broken(format!("malformed DataRow message from the server ({} columns, {} expected)", n, self.current.columns.len()));
                }
                let mut row = Vec::with_capacity(n as usize);
                for _ in 0..n {
                    let len = c.i32()?;
                    if len == -1 {
                        row.push(None);
                    } else if len < 0 {
                        return broken(format!("malformed DataRow message from the server (column length {})", len));
                    } else {
                        row.push(Some(String::from_utf8_lossy(c.bytes(len as usize)?).to_string()));
                    }
                }
                self.current.rows.push(row);
            }
            b'C' => {
                let mut c = Cur::new(body, "CommandComplete");
                self.current.tag = c.cstr()?;
                self.last = Some(std::mem::take(&mut self.current));
                self.have_description = false;
            }
            b'I' => {
                self.last = Some(std::mem::take(&mut self.current));
                self.have_description = false;
            }
            b'E' => {
                let e = server_error(body)?;
                if self.error.is_none() {
                    self.error = Some(e);
                }
            }
            b'Z' => {
                Cur::new(body, "ReadyForQuery").u8()?;
                return Ok(true);
            }
            b'1' | b'2' | b'3' | b'n' | b's' | b't' | b'N' | b'S' | b'A' | b'K' => {}
            b'G' => {
                // COPY FROM STDIN is not supported: refuse it, the server then reports an error and gets ready
                let mut b = Vec::new();
                put_cstr(&mut b, "COPY FROM STDIN is not supported by the Killer PostgreSQL client");
                send(s, &msg(b'f', &b)?)?;
            }
            b'H' => {
                if self.error.is_none() {
                    self.error = Some("COPY TO STDOUT is not supported by the Killer PostgreSQL client".to_string());
                }
            }
            b'd' | b'c' => {}
            b'W' => return broken("COPY BOTH (replication) is not supported"),
            other => return broken(format!("unexpected message '{}' from the server", printable(other))),
        }
        Ok(false)
    }

    fn finish(self) -> Result<QueryResult, PgErr> {
        if let Some(e) = self.error {
            return Err(PgErr::Server(e));
        }
        Ok(self.last.unwrap_or(self.current))
    }
}

fn run_cycle(s: &mut TcpStream) -> Result<QueryResult, PgErr> {
    let mut col = Collector::new();
    loop {
        let (t, body) = read_msg(s)?;
        if col.take(s, t, &body)? {
            return col.finish();
        }
    }
}

fn simple_query(s: &mut TcpStream, sql: &str) -> Result<QueryResult, PgErr> {
    let mut b = Vec::with_capacity(sql.len() + 1);
    put_cstr(&mut b, sql);
    send(s, &msg(b'Q', &b)?)?;
    run_cycle(s)
}

fn extended_query(s: &mut TcpStream, sql: &str, params: &[Option<String>]) -> Result<QueryResult, PgErr> {
    let mut out = Vec::new();
    // Parse: unnamed statement, parameter types left to the server
    let mut b = Vec::new();
    put_cstr(&mut b, "");
    put_cstr(&mut b, sql);
    b.extend_from_slice(&0i16.to_be_bytes());
    out.extend_from_slice(&msg(b'P', &b)?);
    // Bind: unnamed portal, all parameters and results in text format
    let mut b = Vec::new();
    put_cstr(&mut b, "");
    put_cstr(&mut b, "");
    b.extend_from_slice(&0i16.to_be_bytes());
    b.extend_from_slice(&(params.len() as i16).to_be_bytes());
    for p in params {
        match p {
            None => b.extend_from_slice(&(-1i32).to_be_bytes()),
            Some(v) => {
                let n = i32::try_from(v.len()).map_err(|_| PgErr::Server("a parameter is too large".to_string()))?;
                b.extend_from_slice(&n.to_be_bytes());
                b.extend_from_slice(v.as_bytes());
            }
        }
    }
    b.extend_from_slice(&0i16.to_be_bytes());
    out.extend_from_slice(&msg(b'B', &b)?);
    // Describe the portal, Execute it (no row limit), Sync
    out.extend_from_slice(&msg(b'D', &[b'P', 0])?);
    out.extend_from_slice(&msg(b'E', &[0, 0, 0, 0, 0])?);
    out.extend_from_slice(&msg(b'S', &[])?);
    send(s, &out)?;
    run_cycle(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scram_rfc7677_test_vector() {
        // RFC 7677 section 3: user "user", password "pencil"
        let mut st = Scram::with_nonce("rOprNGfwEbeRWgbNEkqO", "user");
        assert_eq!(st.client_first(), "n,,n=user,r=rOprNGfwEbeRWgbNEkqO");
        let sf = b"r=rOprNGfwEbeRWgbNEkqO%hvYDpWUa2RaTCAfuxFIlj)hNlF$k0,s=W22ZaJ0SNY7soEsUEjb6gQ==,i=4096";
        let fin = st.client_final(sf, b"pencil").unwrap();
        assert_eq!(fin, "c=biws,r=rOprNGfwEbeRWgbNEkqO%hvYDpWUa2RaTCAfuxFIlj)hNlF$k0,p=dHzbZapWIk4jUhN+Ute9ytag9zjfMHgsqmmiz7AndVQ=");
        st.verify_server_final(b"v=6rriTRBi23WpRR/wtup+mMhUZUn/dB5nLTJRsjl95G4=").unwrap();
        assert!(st.verify_server_final(b"v=7rriTRBi23WpRR/wtup+mMhUZUn/dB5nLTJRsjl95G4=").is_err());
        assert!(st.verify_server_final(b"e=invalid-proof").is_err());
    }

    #[test]
    fn md5_known_values() {
        assert_eq!(hex(&md5(b"")), "d41d8cd98f00b204e9800998ecf8427e");
        assert_eq!(hex(&md5(b"abc")), "900150983cd24fb0d6963f7d28e17f72");
        assert_eq!(hex(&md5(b"The quick brown fox jumps over the lazy dog")), "9e107d9d372bb6826bd81d3542a419d6");
        assert_eq!(hex(&md5(&[b'a'; 64])), "014842d480b571495a4a0363793f7367");
        assert_eq!(md5_password("alice", "secret", &[1, 2, 3, 4]), "md598a0412b9c31436fc53776e863350083");
    }

    #[test]
    fn base64_round_trip() {
        for n in 0..40usize {
            let data: Vec<u8> = (0..n).map(|i| (i * 37 + 11) as u8).collect();
            assert_eq!(b64decode(&b64encode(&data)).unwrap(), data);
        }
        assert!(b64decode("a*b=").is_none());
        assert!(b64decode("AAAA===").is_none());
    }

    #[test]
    fn command_tag_counts() {
        assert_eq!(affected("INSERT 0 5"), 5);
        assert_eq!(affected("UPDATE 3"), 3);
        assert_eq!(affected("SELECT 12"), 12);
        assert_eq!(affected("CREATE TABLE"), 0);
        assert_eq!(affected("BEGIN"), 0);
        assert_eq!(affected(""), 0);
    }

    #[test]
    fn url_parsing() {
        let base = || Config {
            host: "localhost".into(),
            port: 5432,
            user: String::new(),
            password: String::new(),
            database: String::new(),
            application_name: None,
            allow_md5: false,
            allow_cleartext: false,
            connect_timeout_ms: 10_000,
            timeout_ms: 30_000,
        };
        let mut c = base();
        parse_url("postgres://u:p%2Fw@[::1]:6543/db?application_name=app&connect_timeout=3", &mut c).unwrap();
        assert_eq!((c.host.as_str(), c.port, c.user.as_str(), c.password.as_str(), c.database.as_str()), ("::1", 6543, "u", "p/w", "db"));
        assert_eq!(c.application_name.as_deref(), Some("app"));
        assert_eq!(c.connect_timeout_ms, 3000);
        let mut c = base();
        parse_url("postgresql://u@db.example", &mut c).unwrap();
        assert_eq!((c.host.as_str(), c.port), ("db.example", 5432));
        assert!(parse_url("postgres://u@h:0/x", &mut base()).is_err());
        assert!(parse_url("postgres://u@[::1/x", &mut base()).is_err());
    }

    #[test]
    fn cursor_never_reads_past_the_end() {
        let mut c = Cur::new(&[0, 1], "x");
        assert!(c.i32().is_err());
        let mut c = Cur::new(&[0, 0, 0, 1], "x");
        c.i32().unwrap();
        assert!(c.u8().is_err());
        assert!(c.cstr().is_err());
        assert!(c.bytes(usize::MAX).is_err());
        assert!(server_error(b"Mboom").is_err());
        assert_eq!(server_error(b"SERROR\0C42P01\0Mno\0\0").unwrap(), "ERROR: no (SQLSTATE 42P01)");
    }
}
