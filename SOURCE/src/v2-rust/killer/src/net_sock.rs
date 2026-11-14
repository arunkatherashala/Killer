//! Real sockets for Killer scripts: TCP, UDP and WebSocket (ws://), built on `std::net` only.
//!
//! A socket is a whole number (a handle) returned by the opening call and passed to the others. Handles live in a
//! per-thread table, so a handle made on one thread is not valid on another. Every call needs the network
//! capability (`security::require_network`). Timeouts are in milliseconds; 0 means wait forever.
//!
//! TCP:   tcp_listen(addr) tcp_accept(l[, ms]) tcp_connect(addr[, ms]) sock_send(s, text) sock_recv(s[, max, ms])
//!        sock_recv_line(s[, ms]) sock_addr(s) sock_peer(s) sock_close(s)
//! UDP:   udp_bind(addr) udp_send(s, addr, text) udp_recv(s[, max, ms]) -> {"data", "from"}
//! WS:    ws_connect(url[, opts]) ws_accept(listener[, ms]) ws_send(s, text) ws_recv(s[, ms]) ws_close(s)
//! TLS:   tcp_connect_tls(addr[, opts]) and ws_connect("wss://...") connect through the system `openssl s_client`,
//!        which verifies the certificate chain and the host name; opts {"ca_file": path} trusts an extra CA file.
//!        tls_error(s) returns what openssl reported (for example a certificate failure) once such a connection closed.
//!
//! A receive that times out returns `null`; `sock_recv` returns `""` once the peer has closed.
//! `call` returns `None` for names this module does not own.

use crate::error::VmError;
use crate::value::{SharedDict, Value};
use std::cell::RefCell;
use std::collections::HashMap;
use std::io::{ErrorKind, Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream, ToSocketAddrs, UdpSocket};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{channel, Receiver, RecvTimeoutError};
use std::sync::{Arc, Mutex};
use std::time::Duration;

enum Sock {
    Listener(TcpListener),
    Tcp { stream: Conn, buf: Vec<u8> },
    Udp(UdpSocket),
    Ws { stream: Conn, buf: Vec<u8>, client: bool, closed: bool },
}

/// A byte stream: a plain TCP socket, or a TLS connection made through the system `openssl s_client` (which checks the
/// server certificate and host name). The TLS side is a child process whose output is read on a helper thread.
enum Conn {
    Tcp(TcpStream),
    Tls(Box<TlsPipe>),
}

struct TlsPipe {
    child: Child,
    stdin: Option<ChildStdin>,
    rx: Receiver<Vec<u8>>,
    pending: Vec<u8>,
    timeout: Option<Duration>,
    errors: Arc<Mutex<String>>,
    addr: String,
    /// server side: on close, let `openssl` flush and finish before it is killed
    graceful: bool,
}

impl Drop for TlsPipe {
    fn drop(&mut self) {
        self.stdin.take();
        if self.graceful {
            crate::net_tls_server::reap(&mut self.child, Duration::from_secs(20));
            return;
        }
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl Conn {
    fn set_read_timeout(&mut self, d: Option<Duration>) -> std::io::Result<()> {
        match self {
            Conn::Tcp(s) => s.set_read_timeout(d),
            Conn::Tls(t) => {
                t.timeout = d;
                Ok(())
            }
        }
    }
    fn local(&self) -> std::io::Result<String> {
        match self {
            Conn::Tcp(s) => s.local_addr().map(|a| a.to_string()),
            Conn::Tls(_) => Ok("(tls)".to_string()),
        }
    }
    fn peer(&self) -> std::io::Result<String> {
        match self {
            Conn::Tcp(s) => s.peer_addr().map(|a| a.to_string()),
            Conn::Tls(t) => Ok(t.addr.clone()),
        }
    }
    /// What `openssl` said (certificate or network errors), for messages after a TLS connection closed early.
    fn tls_hint(&self) -> String {
        match self {
            Conn::Tls(t) => {
                let e = t.errors.lock().map(|g| g.trim().to_string()).unwrap_or_default();
                if e.is_empty() {
                    String::new()
                } else {
                    format!(" (TLS: {})", e.lines().rev().take(3).collect::<Vec<_>>().into_iter().rev().collect::<Vec<_>>().join("; "))
                }
            }
            Conn::Tcp(_) => String::new(),
        }
    }
}

impl Read for Conn {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match self {
            Conn::Tcp(s) => s.read(buf),
            Conn::Tls(t) => {
                if t.pending.is_empty() {
                    let got = match t.timeout {
                        Some(d) => t.rx.recv_timeout(d),
                        None => t.rx.recv().map_err(|_| RecvTimeoutError::Disconnected),
                    };
                    match got {
                        Ok(v) => t.pending = v,
                        Err(RecvTimeoutError::Timeout) => return Err(std::io::Error::new(ErrorKind::TimedOut, "timed out")),
                        Err(RecvTimeoutError::Disconnected) => return Ok(0),
                    }
                }
                let n = buf.len().min(t.pending.len());
                buf[..n].copy_from_slice(&t.pending[..n]);
                t.pending.drain(..n);
                Ok(n)
            }
        }
    }
}

impl Write for Conn {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        match self {
            Conn::Tcp(s) => s.write(buf),
            Conn::Tls(t) => match t.stdin.as_mut() {
                Some(w) => w.write(buf),
                None => Err(std::io::Error::new(ErrorKind::BrokenPipe, "the TLS connection is closed")),
            },
        }
    }
    fn flush(&mut self) -> std::io::Result<()> {
        match self {
            Conn::Tcp(s) => s.flush(),
            Conn::Tls(t) => t.stdin.as_mut().map_or(Ok(()), |w| w.flush()),
        }
    }
}

/// Open a verified TLS connection to `host:port` with `openssl s_client`. `ca_file` adds a trusted certificate
/// authority file (for a private or test CA). The system trust store is used otherwise.
fn tls_connect(addr: &str, ca_file: Option<&str>, who: &str) -> Result<Conn, VmError> {
    let (host, port) = match addr.rsplit_once(':') {
        Some((h, p)) if !h.is_empty() && p.parse::<u16>().is_ok() => (h.trim_matches(|c| c == '[' || c == ']').to_string(), p.to_string()),
        _ => return err(format!("{}(): the address must look like host:port, got '{}'", who, addr)),
    };
    if host.starts_with('-') || host.chars().any(|c| c.is_whitespace() || c.is_control()) {
        return err(format!("{}(): bad host name", who));
    }
    let mut cmd = Command::new("openssl");
    cmd.arg("s_client").arg("-quiet").arg("-connect").arg(format!("{}:{}", host, port)).arg("-servername").arg(&host);
    cmd.arg("-verify_return_error").arg("-verify_hostname").arg(&host);
    if let Some(f) = ca_file {
        if !std::path::Path::new(f).is_file() {
            return err(format!("{}(): the ca_file '{}' does not exist", who, f));
        }
        cmd.arg("-CAfile").arg(f);
    }
    cmd.stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) => return err(format!("{}(): TLS needs the `openssl` command-line tool on the PATH ({})", who, e)),
    };
    let out = child.stdout.take().unwrap();
    let errp = child.stderr.take().unwrap();
    let stdin = child.stdin.take();
    Ok(pipe_conn(child, stdin, out, errp, addr.to_string(), false))
}

/// Wrap a TLS child process (plain text read from its stdout on a helper thread, stderr collected) as a connection.
fn pipe_conn(child: Child, stdin: Option<ChildStdin>, mut out: impl Read + Send + 'static, mut errp: impl Read + Send + 'static, addr: String, graceful: bool) -> Conn {
    let (tx, rx) = channel::<Vec<u8>>();
    std::thread::spawn(move || {
        let mut b = [0u8; 8192];
        while let Ok(n) = out.read(&mut b) {
            if n == 0 || tx.send(b[..n].to_vec()).is_err() {
                break;
            }
        }
    });
    let errors = Arc::new(Mutex::new(String::new()));
    let e2 = errors.clone();
    std::thread::spawn(move || {
        let mut b = [0u8; 2048];
        while let Ok(n) = errp.read(&mut b) {
            if n == 0 {
                break;
            }
            if let Ok(mut g) = e2.lock() {
                if g.len() < 8192 {
                    g.push_str(&String::from_utf8_lossy(&b[..n]));
                }
            }
        }
    });
    Conn::Tls(Box::new(TlsPipe { child, stdin, rx, pending: Vec::new(), timeout: None, errors, addr, graceful }))
}

/// A connection accepted on listener `id`: plain, or (for a `tls_listen` listener) with TLS terminated by `openssl`.
fn wrap_accepted(id: u64, raw: TcpStream, who: &str) -> Result<Conn, VmError> {
    match TLS_CFG.with(|c| c.borrow().get(&id).cloned()) {
        None => Ok(Conn::Tcp(raw)),
        Some((cert, key)) => {
            let peer = raw.peer_addr().map(|a| a.to_string()).unwrap_or_default();
            let t = crate::net_tls_server::terminate(raw, &cert, &key, who)?;
            Ok(pipe_conn(t.child, Some(t.stdin), t.stdout, t.stderr, peer, true))
        }
    }
}

fn opt_str(args: &[Value], i: usize, key: &str) -> Option<String> {
    match args.get(i) {
        Some(Value::Dict(d)) => match d.get(key) {
            Some(Value::Str(s)) => Some(s),
            _ => None,
        },
        _ => None,
    }
}

thread_local! {
    static SOCKS: RefCell<HashMap<u64, Sock>> = RefCell::new(HashMap::new());
    /// listener handle -> (certificate file, key file) for listeners made by `tls_listen`
    static TLS_CFG: RefCell<HashMap<u64, (String, String)>> = RefCell::new(HashMap::new());
    static NEXT: RefCell<u64> = const { RefCell::new(1) };
}

const NAMES: &[&str] = &[
    "tcp_listen", "tcp_accept", "tcp_connect", "sock_send", "sock_recv", "sock_recv_line", "sock_addr", "sock_peer",
    "sock_close", "tcp_connect_tls", "tls_error", "tls_listen", "tls_accept", "udp_bind", "udp_send", "udp_recv", "ws_connect", "ws_accept", "ws_send", "ws_recv", "ws_close",
];

/// True when `name` is one of this module's builtins (the compiler needs the list too).
pub fn owns(name: &str) -> bool {
    NAMES.contains(&name)
}

pub fn names() -> &'static [&'static str] {
    NAMES
}

fn err<T>(msg: impl Into<String>) -> Result<T, VmError> {
    Err(VmError::runtime_error(msg.into()))
}

fn string<'a>(args: &'a [Value], i: usize, who: &str) -> Result<&'a str, VmError> {
    match args.get(i) {
        Some(Value::Str(s)) => Ok(s.as_str()),
        Some(other) => err(format!("{}(): argument {} must be a string, got {}", who, i + 1, other.type_name())),
        None => err(format!("{}() is missing argument {}", who, i + 1)),
    }
}

fn whole(args: &[Value], i: usize, who: &str, default: Option<u64>) -> Result<u64, VmError> {
    match args.get(i) {
        Some(Value::Number(n)) if n.is_finite() && *n >= 0.0 && n.fract() == 0.0 => Ok(*n as u64),
        Some(Value::Integer(n)) if *n >= 0 => Ok(*n as u64),
        Some(other) => err(format!("{}(): argument {} must be a whole number >= 0, got {}", who, i + 1, other.type_name())),
        None => default.ok_or_else(|| VmError::runtime_error(format!("{}() is missing argument {}", who, i + 1))),
    }
}

fn handle(args: &[Value], who: &str) -> Result<u64, VmError> {
    whole(args, 0, who, None)
}

fn timeout(ms: u64) -> Option<Duration> {
    if ms == 0 {
        None
    } else {
        Some(Duration::from_millis(ms))
    }
}

fn is_timeout(e: &std::io::Error) -> bool {
    matches!(e.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut)
}

fn io<T>(who: &str, r: std::io::Result<T>) -> Result<T, VmError> {
    r.map_err(|e| VmError::runtime_error(format!("{}(): {}", who, e)))
}

fn insert(sock: Sock) -> Value {
    let id = NEXT.with(|n| {
        let mut n = n.borrow_mut();
        let id = *n;
        *n += 1;
        id
    });
    SOCKS.with(|s| s.borrow_mut().insert(id, sock));
    Value::Number(id as f64)
}

fn with_sock<T>(id: u64, who: &str, f: impl FnOnce(&mut Sock) -> Result<T, VmError>) -> Result<T, VmError> {
    SOCKS.with(|s| match s.borrow_mut().get_mut(&id) {
        Some(sock) => f(sock),
        None => err(format!("{}(): {} is not an open socket", who, id)),
    })
}

fn resolve(addr: &str, who: &str) -> Result<SocketAddr, VmError> {
    match addr.to_socket_addrs() {
        Ok(mut it) => it.next().ok_or_else(|| VmError::runtime_error(format!("{}(): cannot resolve \"{}\"", who, addr))),
        Err(e) => err(format!("{}(): bad address \"{}\": {}", who, addr, e)),
    }
}

fn connect(addr: &str, ms: u64, who: &str) -> Result<TcpStream, VmError> {
    let sa = resolve(addr, who)?;
    let r = match timeout(ms) {
        Some(d) => TcpStream::connect_timeout(&sa, d),
        None => TcpStream::connect(sa),
    };
    let s = io(who, r)?;
    let _ = s.set_nodelay(true);
    Ok(s)
}

fn accept(listener: &TcpListener, ms: u64, who: &str) -> Result<Option<TcpStream>, VmError> {
    if ms == 0 {
        let (s, _) = io(who, listener.accept())?;
        let _ = s.set_nodelay(true);
        return Ok(Some(s));
    }
    io(who, listener.set_nonblocking(true))?;
    let deadline = std::time::Instant::now() + Duration::from_millis(ms);
    let result = loop {
        match listener.accept() {
            Ok((s, _)) => break Ok(Some(s)),
            Err(e) if e.kind() == ErrorKind::WouldBlock => {
                if std::time::Instant::now() >= deadline {
                    break Ok(None);
                }
                std::thread::sleep(Duration::from_millis(2));
            }
            Err(e) => break err(format!("{}(): {}", who, e)),
        }
    };
    let _ = listener.set_nonblocking(false);
    if let Ok(Some(s)) = &result {
        let _ = s.set_nonblocking(false);
        let _ = s.set_nodelay(true);
    }
    result
}

fn text(bytes: &[u8]) -> Value {
    Value::Str(String::from_utf8_lossy(bytes).into_owned())
}

/// Read what is available (up to `max` bytes) into the connection's own buffer logic. `Ok(None)` is a timeout.
fn read_some(stream: &mut Conn, buf: &mut Vec<u8>, max: usize, ms: u64, who: &str) -> Result<Option<Vec<u8>>, VmError> {
    if !buf.is_empty() {
        let n = max.min(buf.len());
        return Ok(Some(buf.drain(..n).collect()));
    }
    io(who, stream.set_read_timeout(timeout(ms)))?;
    let mut chunk = vec![0u8; max.clamp(1, 1 << 20)];
    match stream.read(&mut chunk) {
        Ok(n) => {
            chunk.truncate(n);
            Ok(Some(chunk))
        }
        Err(e) if is_timeout(&e) => Ok(None),
        Err(e) => err(format!("{}(): {}", who, e)),
    }
}

fn read_line(stream: &mut Conn, buf: &mut Vec<u8>, ms: u64, who: &str) -> Result<Option<Vec<u8>>, VmError> {
    io(who, stream.set_read_timeout(timeout(ms)))?;
    loop {
        if let Some(p) = buf.iter().position(|&b| b == b'\n') {
            let mut line: Vec<u8> = buf.drain(..=p).collect();
            line.pop();
            if line.last() == Some(&b'\r') {
                line.pop();
            }
            return Ok(Some(line));
        }
        let mut chunk = [0u8; 4096];
        match stream.read(&mut chunk) {
            Ok(0) => {
                if buf.is_empty() {
                    return Ok(None);
                }
                return Ok(Some(std::mem::take(buf)));
            }
            Ok(n) => buf.extend_from_slice(&chunk[..n]),
            Err(e) if is_timeout(&e) => return Ok(None),
            Err(e) => return err(format!("{}(): {}", who, e)),
        }
    }
}

pub fn call(name: &str, args: &[Value]) -> Option<Result<Value, VmError>> {
    if !owns(name) {
        return None;
    }
    if let Err(e) = crate::security::require_network() {
        return Some(Err(e));
    }
    Some(dispatch(name, args))
}

fn dispatch(name: &str, args: &[Value]) -> Result<Value, VmError> {
    match name {
        "tcp_listen" => {
            let addr = string(args, 0, name)?;
            let l = io(name, TcpListener::bind(resolve(addr, name)?))?;
            Ok(insert(Sock::Listener(l)))
        }
        "tls_listen" => {
            let addr = string(args, 0, name)?;
            let (cert, key) = (string(args, 1, name)?, string(args, 2, name)?);
            crate::net_tls_server::check_credentials(cert, key, name)?;
            let l = io(name, TcpListener::bind(resolve(addr, name)?))?;
            let v = insert(Sock::Listener(l));
            if let Value::Number(n) = &v {
                TLS_CFG.with(|c| c.borrow_mut().insert(*n as u64, (cert.to_string(), key.to_string())));
            }
            Ok(v)
        }
        "tcp_accept" | "tls_accept" => {
            let id = handle(args, name)?;
            let ms = whole(args, 1, name, Some(0))?;
            if name == "tls_accept" && !TLS_CFG.with(|c| c.borrow().contains_key(&id)) {
                return err("tls_accept(): not a TLS listener (make one with tls_listen)");
            }
            let got = with_sock(id, name, |s| match s {
                Sock::Listener(l) => accept(l, ms, name),
                _ => err(format!("{}(): not a listener", name)),
            })?;
            Ok(match got {
                Some(stream) => insert(Sock::Tcp { stream: wrap_accepted(id, stream, name)?, buf: Vec::new() }),
                None => Value::Null,
            })
        }
        "tcp_connect" => {
            let addr = string(args, 0, name)?;
            let ms = whole(args, 1, name, Some(10_000))?;
            Ok(insert(Sock::Tcp { stream: Conn::Tcp(connect(addr, ms, name)?), buf: Vec::new() }))
        }
        "tls_error" => {
            let id = handle(args, name)?;
            with_sock(id, name, |s| match s {
                Sock::Tcp { stream, .. } | Sock::Ws { stream, .. } => match stream {
                    Conn::Tls(t) => Ok(Value::Str(t.errors.lock().map(|g| g.trim().to_string()).unwrap_or_default())),
                    Conn::Tcp(_) => Ok(Value::Str(String::new())),
                },
                _ => err("tls_error(): not a TCP or WebSocket connection"),
            })
        }
        "tcp_connect_tls" => {
            let addr = string(args, 0, name)?;
            let ca = opt_str(args, 1, "ca_file");
            Ok(insert(Sock::Tcp { stream: tls_connect(addr, ca.as_deref(), name)?, buf: Vec::new() }))
        }
        "sock_send" => {
            let id = handle(args, name)?;
            let data = string(args, 1, name)?;
            with_sock(id, name, |s| match s {
                Sock::Tcp { stream, .. } => {
                    io(name, stream.write_all(data.as_bytes()))?;
                    Ok(Value::Number(data.len() as f64))
                }
                _ => err("sock_send(): not a TCP connection (use ws_send for WebSockets, udp_send for UDP)"),
            })
        }
        "sock_recv" => {
            let id = handle(args, name)?;
            let max = whole(args, 1, name, Some(4096))? as usize;
            let ms = whole(args, 2, name, Some(0))?;
            with_sock(id, name, |s| match s {
                Sock::Tcp { stream, buf } => Ok(match read_some(stream, buf, max, ms, name)? {
                    Some(b) => text(&b),
                    None => Value::Null,
                }),
                _ => err("sock_recv(): not a TCP connection"),
            })
        }
        "sock_recv_line" => {
            let id = handle(args, name)?;
            let ms = whole(args, 1, name, Some(0))?;
            with_sock(id, name, |s| match s {
                Sock::Tcp { stream, buf } => Ok(match read_line(stream, buf, ms, name)? {
                    Some(b) => text(&b),
                    None => Value::Null,
                }),
                _ => err("sock_recv_line(): not a TCP connection"),
            })
        }
        "sock_addr" | "sock_peer" => {
            let id = handle(args, name)?;
            with_sock(id, name, |s| {
                let r = match s {
                    Sock::Listener(l) => l.local_addr().map(|a| a.to_string()),
                    Sock::Tcp { stream, .. } | Sock::Ws { stream, .. } => {
                        if name == "sock_peer" {
                            stream.peer()
                        } else {
                            stream.local()
                        }
                    }
                    Sock::Udp(u) => u.local_addr().map(|a| a.to_string()),
                };
                Ok(Value::Str(io(name, r)?))
            })
        }
        "sock_close" | "ws_close" => {
            let id = handle(args, name)?;
            let removed = SOCKS.with(|s| s.borrow_mut().remove(&id));
            TLS_CFG.with(|c| c.borrow_mut().remove(&id));
            if let Some(Sock::Ws { mut stream, mut buf, client, closed: false }) = removed {
                let _ = stream.write_all(&frame(8, &[], client));
                if matches!(stream, Conn::Tls(_)) {
                    // finish the closing handshake so the TLS child has sent the close frame before it is stopped
                    let mut done = false;
                    let _ = ws_read(&mut stream, &mut buf, client, &mut done, 1500);
                }
            }
            Ok(Value::Null)
        }
        "udp_bind" => {
            let addr = string(args, 0, name)?;
            let u = io(name, UdpSocket::bind(resolve(addr, name)?))?;
            Ok(insert(Sock::Udp(u)))
        }
        "udp_send" => {
            let id = handle(args, name)?;
            let to = resolve(string(args, 1, name)?, name)?;
            let data = string(args, 2, name)?;
            with_sock(id, name, |s| match s {
                Sock::Udp(u) => Ok(Value::Number(io(name, u.send_to(data.as_bytes(), to))? as f64)),
                _ => err("udp_send(): not a UDP socket"),
            })
        }
        "udp_recv" => {
            let id = handle(args, name)?;
            let max = whole(args, 1, name, Some(65_535))? as usize;
            let ms = whole(args, 2, name, Some(0))?;
            with_sock(id, name, |s| match s {
                Sock::Udp(u) => {
                    io(name, u.set_read_timeout(timeout(ms)))?;
                    let mut b = vec![0u8; max.clamp(1, 65_535)];
                    match u.recv_from(&mut b) {
                        Ok((n, from)) => {
                            let mut d = HashMap::new();
                            d.insert("data".to_string(), text(&b[..n]));
                            d.insert("from".to_string(), Value::Str(from.to_string()));
                            Ok(Value::Dict(SharedDict::new(d)))
                        }
                        Err(e) if is_timeout(&e) => Ok(Value::Null),
                        Err(e) => err(format!("{}(): {}", name, e)),
                    }
                }
                _ => err("udp_recv(): not a UDP socket"),
            })
        }
        "ws_connect" => ws_connect(string(args, 0, name)?, opt_str(args, 1, "ca_file").as_deref()),
        "ws_accept" => {
            let id = handle(args, name)?;
            let ms = whole(args, 1, name, Some(0))?;
            let got = with_sock(id, name, |s| match s {
                Sock::Listener(l) => accept(l, ms, name),
                _ => err("ws_accept(): not a listener (make one with tcp_listen)"),
            })?;
            match got {
                Some(raw) => {
                    let mut stream = wrap_accepted(id, raw, name)?;
                    let mut buf = Vec::new();
                    server_handshake(&mut stream, &mut buf)?;
                    Ok(insert(Sock::Ws { stream, buf, client: false, closed: false }))
                }
                None => Ok(Value::Null),
            }
        }
        "ws_send" => {
            let id = handle(args, name)?;
            let data = string(args, 1, name)?;
            with_sock(id, name, |s| match s {
                Sock::Ws { stream, client, closed, .. } => {
                    if *closed {
                        return err("ws_send(): the connection is closed");
                    }
                    io(name, stream.write_all(&frame(1, data.as_bytes(), *client)))?;
                    Ok(Value::Number(data.len() as f64))
                }
                _ => err("ws_send(): not a WebSocket"),
            })
        }
        "ws_recv" => {
            let id = handle(args, name)?;
            let ms = whole(args, 1, name, Some(0))?;
            with_sock(id, name, |s| match s {
                Sock::Ws { stream, buf, client, closed } => ws_read(stream, buf, *client, closed, ms),
                _ => err("ws_recv(): not a WebSocket"),
            })
        }
        _ => err(format!("unknown socket builtin {}", name)),
    }
}

// ---------------------------------------------------------------------------------------------
// WebSocket (RFC 6455): text frames, ping/pong, close; no extensions or fragmentation on send.

const GUID: &str = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11";

fn accept_key(key: &str) -> String {
    base64(&sha1(format!("{}{}", key.trim(), GUID).as_bytes()))
}

fn ws_connect(url: &str, ca_file: Option<&str>) -> Result<Value, VmError> {
    let (tls, rest) = if let Some(r) = url.strip_prefix("ws://") {
        (false, r)
    } else if let Some(r) = url.strip_prefix("wss://") {
        (true, r)
    } else {
        return err("ws_connect(): the url must start with ws:// or wss://");
    };
    let (hostport, path) = match rest.find('/') {
        Some(i) => (&rest[..i], &rest[i..]),
        None => (rest, "/"),
    };
    if hostport.is_empty() || hostport.chars().any(|c| c.is_whitespace() || c.is_control()) || path.chars().any(|c| c.is_control() || c == ' ') {
        return err("ws_connect(): bad url");
    }
    let addr = if hostport.contains(':') { hostport.to_string() } else { format!("{}:{}", hostport, if tls { 443 } else { 80 }) };
    let mut stream = if tls { tls_connect(&addr, ca_file, "ws_connect")? } else { Conn::Tcp(connect(&addr, 10_000, "ws_connect")?) };
    let key = base64(&nonce16());
    let req = format!(
        "GET {path} HTTP/1.1\r\nHost: {hostport}\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Key: {key}\r\nSec-WebSocket-Version: 13\r\n\r\n"
    );
    io("ws_connect", stream.write_all(req.as_bytes()))?;
    let mut buf = Vec::new();
    let head = read_head(&mut stream, &mut buf, "ws_connect")?;
    let first = head.lines().next().unwrap_or("");
    if !first.contains(" 101") {
        return err(format!("ws_connect(): the server did not upgrade the connection ({})", first.trim()));
    }
    let want = accept_key(&key);
    let got = head
        .lines()
        .find_map(|l| l.split_once(':').filter(|(k, _)| k.trim().eq_ignore_ascii_case("sec-websocket-accept")).map(|(_, v)| v.trim().to_string()));
    if got.as_deref() != Some(want.as_str()) {
        return err("ws_connect(): the server's Sec-WebSocket-Accept value is wrong");
    }
    Ok(insert(Sock::Ws { stream, buf, client: true, closed: false }))
}

fn server_handshake(stream: &mut Conn, buf: &mut Vec<u8>) -> Result<(), VmError> {
    let head = read_head(stream, buf, "ws_accept")?;
    let key = head
        .lines()
        .find_map(|l| l.split_once(':').filter(|(k, _)| k.trim().eq_ignore_ascii_case("sec-websocket-key")).map(|(_, v)| v.trim().to_string()));
    let key = match key {
        Some(k) => k,
        None => {
            let _ = stream.write_all(b"HTTP/1.1 400 Bad Request\r\nContent-Length: 0\r\n\r\n");
            return err("ws_accept(): the client sent no Sec-WebSocket-Key (not a WebSocket request)");
        }
    };
    let resp = format!(
        "HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Accept: {}\r\n\r\n",
        accept_key(&key)
    );
    io("ws_accept", stream.write_all(resp.as_bytes()))
}

/// Read an HTTP head (up to the blank line); anything read past it stays in `buf`.
fn read_head(stream: &mut Conn, buf: &mut Vec<u8>, who: &str) -> Result<String, VmError> {
    io(who, stream.set_read_timeout(Some(Duration::from_secs(30))))?;
    loop {
        if let Some(p) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
            let head: Vec<u8> = buf.drain(..p + 4).collect();
            io(who, stream.set_read_timeout(None))?;
            return Ok(String::from_utf8_lossy(&head).into_owned());
        }
        if buf.len() > 65_536 {
            return err(format!("{}(): the HTTP header is too large", who));
        }
        let mut chunk = [0u8; 2048];
        match stream.read(&mut chunk) {
            Ok(0) => return err(format!("{}(): the connection closed during the handshake{}", who, stream.tls_hint())),
            Ok(n) => buf.extend_from_slice(&chunk[..n]),
            Err(e) => return err(format!("{}(): {}", who, e)),
        }
    }
}

/// One frame: FIN set, `opcode`, masked when sent by a client.
fn frame(opcode: u8, payload: &[u8], mask: bool) -> Vec<u8> {
    let mut out = vec![0x80 | opcode];
    let m = if mask { 0x80 } else { 0 };
    let n = payload.len();
    if n < 126 {
        out.push(m | n as u8);
    } else if n <= 0xFFFF {
        out.push(m | 126);
        out.extend_from_slice(&(n as u16).to_be_bytes());
    } else {
        out.push(m | 127);
        out.extend_from_slice(&(n as u64).to_be_bytes());
    }
    if mask {
        let key: [u8; 4] = nonce16()[..4].try_into().unwrap();
        out.extend_from_slice(&key);
        out.extend(payload.iter().enumerate().map(|(i, b)| b ^ key[i % 4]));
    } else {
        out.extend_from_slice(payload);
    }
    out
}

fn fill(stream: &mut Conn, buf: &mut Vec<u8>, need: usize, ms: u64) -> Result<bool, VmError> {
    while buf.len() < need {
        io("ws_recv", stream.set_read_timeout(timeout(ms)))?;
        let mut chunk = [0u8; 4096];
        match stream.read(&mut chunk) {
            Ok(0) => return err(format!("ws_recv(): the connection closed in the middle of a message{}", stream.tls_hint())),
            Ok(n) => buf.extend_from_slice(&chunk[..n]),
            Err(e) if is_timeout(&e) => return Ok(false),
            Err(e) => return err(format!("ws_recv(): {}", e)),
        }
    }
    Ok(true)
}

/// Next text/binary message: `null` on timeout or once the peer has sent close. Pings are answered.
fn ws_read(stream: &mut Conn, buf: &mut Vec<u8>, client: bool, closed: &mut bool, ms: u64) -> Result<Value, VmError> {
    if *closed {
        return Ok(Value::Null);
    }
    let mut message: Vec<u8> = Vec::new();
    loop {
        // a frame is consumed only when all of it has arrived, so a timeout never loses part of one
        if !fill(stream, buf, 2, ms)? {
            return Ok(Value::Null);
        }
        let (b0, b1) = (buf[0], buf[1]);
        let masked = b1 & 0x80 != 0;
        let (len, head) = match b1 & 0x7F {
            126 => {
                if !fill(stream, buf, 4, ms)? {
                    return Ok(Value::Null);
                }
                (u16::from_be_bytes([buf[2], buf[3]]) as usize, 4)
            }
            127 => {
                if !fill(stream, buf, 10, ms)? {
                    return Ok(Value::Null);
                }
                let n = u64::from_be_bytes(buf[2..10].try_into().unwrap());
                if n > 64 * 1024 * 1024 {
                    return err("ws_recv(): a message frame larger than 64 MB");
                }
                (n as usize, 10)
            }
            n => (n as usize, 2),
        };
        let total = head + if masked { 4 } else { 0 } + len;
        if !fill(stream, buf, total, ms)? {
            return Ok(Value::Null);
        }
        let all: Vec<u8> = buf.drain(..total).collect();
        let mut payload = all[total - len..].to_vec();
        if masked {
            let key = &all[head..head + 4];
            for (i, b) in payload.iter_mut().enumerate() {
                *b ^= key[i % 4];
            }
        }
        let fin = b0 & 0x80 != 0;
        match b0 & 0x0F {
            0 | 1 | 2 => {
                message.extend_from_slice(&payload);
                if fin {
                    return Ok(text(&message));
                }
            }
            8 => {
                *closed = true;
                let _ = stream.write_all(&frame(8, &payload, client));
                return Ok(Value::Null);
            }
            9 => {
                let _ = stream.write_all(&frame(10, &payload, client));
            }
            _ => {} // pong and unknown control frames are ignored
        }
    }
}

// ---------------------------------------------------------------------------------------------
// SHA-1 and base64 (only for the WebSocket handshake)

fn sha1(data: &[u8]) -> [u8; 20] {
    let mut h: [u32; 5] = [0x67452301, 0xEFCDAB89, 0x98BADCFE, 0x10325476, 0xC3D2E1F0];
    let mut msg = data.to_vec();
    msg.push(0x80);
    while msg.len() % 64 != 56 {
        msg.push(0);
    }
    msg.extend_from_slice(&((data.len() as u64) * 8).to_be_bytes());
    for block in msg.chunks(64) {
        let mut w = [0u32; 80];
        for i in 0..16 {
            w[i] = u32::from_be_bytes(block[i * 4..i * 4 + 4].try_into().unwrap());
        }
        for i in 16..80 {
            w[i] = (w[i - 3] ^ w[i - 8] ^ w[i - 14] ^ w[i - 16]).rotate_left(1);
        }
        let [mut a, mut b, mut c, mut d, mut e] = h;
        for (i, wi) in w.iter().enumerate() {
            let (f, k) = match i {
                0..=19 => ((b & c) | (!b & d), 0x5A827999),
                20..=39 => (b ^ c ^ d, 0x6ED9EBA1),
                40..=59 => ((b & c) | (b & d) | (c & d), 0x8F1BBCDC),
                _ => (b ^ c ^ d, 0xCA62C1D6u32),
            };
            let t = a.rotate_left(5).wrapping_add(f).wrapping_add(e).wrapping_add(k).wrapping_add(*wi);
            e = d;
            d = c;
            c = b.rotate_left(30);
            b = a;
            a = t;
        }
        h[0] = h[0].wrapping_add(a);
        h[1] = h[1].wrapping_add(b);
        h[2] = h[2].wrapping_add(c);
        h[3] = h[3].wrapping_add(d);
        h[4] = h[4].wrapping_add(e);
    }
    let mut out = [0u8; 20];
    for (i, v) in h.iter().enumerate() {
        out[i * 4..i * 4 + 4].copy_from_slice(&v.to_be_bytes());
    }
    out
}

fn base64(data: &[u8]) -> String {
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

/// 16 random bytes from the operating system (the handshake key and frame masks).
fn nonce16() -> [u8; 16] {
    let mut b = [0u8; 16];
    let _ = crate::os_random::fill(&mut b);
    b
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha1_known_vectors() {
        let hex = |d: [u8; 20]| d.iter().map(|b| format!("{:02x}", b)).collect::<String>();
        assert_eq!(hex(sha1(b"abc")), "a9993e364706816aba3e25717850c26c9cd0d89d");
        assert_eq!(hex(sha1(b"")), "da39a3ee5e6b4b0d3255bfef95601890afd80709");
        assert_eq!(
            hex(sha1(b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq")),
            "84983e441c3bd26ebaae4aa1f95129e5e54670f1"
        );
    }

    #[test]
    fn sha1_fips_180_long_and_block_boundary_messages() {
        let hex = |d: [u8; 20]| d.iter().map(|b| format!("{:02x}", b)).collect::<String>();
        // FIPS 180 / NIST: 896-bit and one-million-'a' messages
        assert_eq!(
            hex(sha1(b"abcdefghbcdefghicdefghijdefghijkefghijklfghijklmghijklmnhijklmnoijklmnopjklmnopqklmnopqrlmnopqrsmnopqrstnopqrstu")),
            "a49b2446a02c645bf419f995b67091253a04a259"
        );
        assert_eq!(hex(sha1(&vec![b'a'; 1_000_000])), "34aa973cd4c4daa4f61eeb2bdbad27316534016f");
        // padding edges: 55, 56, 63, 64, 65 bytes (checked against Python hashlib)
        assert_eq!(hex(sha1(&[b'a'; 55])), "c1c8bbdc22796e28c0e15163d20899b65621d65a");
        assert_eq!(hex(sha1(&[b'a'; 56])), "c2db330f6083854c99d4b5bfb6e8f29f201be699");
        assert_eq!(hex(sha1(&[b'a'; 63])), "03f09f5b158a7a8cdad920bddc29b81c18a551f5");
        assert_eq!(hex(sha1(&[b'a'; 64])), "0098ba824b5c16427bd7a1122a5a442a25ec644d");
        assert_eq!(hex(sha1(&[b'a'; 65])), "11655326c708d70319be2610e8a57d9a5b959d3b");
    }

    #[test]
    fn base64_rfc_4648_section_10() {
        for (plain, enc) in [("", ""), ("f", "Zg=="), ("fo", "Zm8="), ("foo", "Zm9v"), ("foob", "Zm9vYg=="), ("fooba", "Zm9vYmE="), ("foobar", "Zm9vYmFy")] {
            assert_eq!(base64(plain.as_bytes()), enc);
        }
    }

    #[test]
    fn websocket_accept_key_matches_rfc_6455() {
        assert_eq!(accept_key("dGhlIHNhbXBsZSBub25jZQ=="), "s3pPLMBiTxaQ9kYGzzhZRbK+xOo=");
    }

    #[test]
    fn base64_padding() {
        assert_eq!(base64(b"f"), "Zg==");
        assert_eq!(base64(b"fo"), "Zm8=");
        assert_eq!(base64(b"foo"), "Zm9v");
    }

    #[test]
    fn frames_round_trip_through_a_real_socket() {
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = l.local_addr().unwrap();
        let t = std::thread::spawn(move || {
            let (mut s, _) = l.accept().unwrap();
            let mut buf = Vec::new();
            let mut closed = false;
            let mut s = Conn::Tcp(s);
            let v = ws_read(&mut s, &mut buf, false, &mut closed, 5000).unwrap();
            let big = "x".repeat(70_000);
            s.write_all(&frame(1, big.as_bytes(), false)).unwrap();
            v
        });
        let mut c = Conn::Tcp(TcpStream::connect(addr).unwrap());
        c.write_all(&frame(1, b"hello", true)).unwrap();
        let (mut buf, mut closed) = (Vec::new(), false);
        let reply = ws_read(&mut c, &mut buf, true, &mut closed, 5000).unwrap();
        assert!(matches!(&reply, Value::Str(s) if s.len() == 70_000));
        assert!(matches!(t.join().unwrap(), Value::Str(s) if s == "hello"));
    }
}
