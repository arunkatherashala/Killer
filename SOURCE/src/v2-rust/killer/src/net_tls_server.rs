//! TLS server side, built the same way as the TLS client in `net_sock.rs`: no crates, the system `openssl` tool does
//! the cryptography.
//!
//! For every accepted TCP connection one `openssl s_server -naccept 1` child process is started on a private
//! loopback port. A bridge thread copies the raw (encrypted) bytes between the client socket and that port, and the
//! plain text travels through the child's stdin and stdout. The caller therefore gets three plain pipes
//! (`Terminated`) and never sees any TLS. Each connection has its own child, so connections are independent and a
//! failed handshake only affects that connection.
use crate::error::VmError;
use std::io::{Read, Write};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::process::{Child, ChildStderr, ChildStdin, ChildStdout, Command, Stdio};
use std::time::{Duration, Instant};

pub struct Terminated {
    pub child: Child,
    pub stdin: ChildStdin,
    pub stdout: ChildStdout,
    pub stderr: ChildStderr,
}

pub fn dbg_on() -> bool {
    std::env::var_os("KILLER_TLS_DEBUG").is_some()
}

fn t_ms() -> u128 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() % 1_000_000).unwrap_or(0)
}

macro_rules! tdbg {
    ($($a:tt)*) => { if dbg_on() { eprintln!("[tls {} pid{}] {}", t_ms(), std::process::id(), format!($($a)*)); } };
}

fn fail<T>(msg: String) -> Result<T, VmError> {
    Err(VmError::runtime_error(msg))
}

fn run_openssl(args: &[&str], who: &str) -> Result<std::process::Output, VmError> {
    match Command::new("openssl").args(args).stdin(Stdio::null()).output() {
        Ok(o) => Ok(o),
        Err(e) => fail(format!("{}(): TLS needs the `openssl` command-line tool on the PATH ({})", who, e)),
    }
}

/// Check up front that `openssl` exists and that the certificate and private key files are readable, valid and belong
/// together, so that mistakes show up as one clear error instead of connections that silently fail.
pub fn check_credentials(cert: &str, key: &str, who: &str) -> Result<(), VmError> {
    for (what, f) in [("certificate", cert), ("private key", key)] {
        if !std::path::Path::new(f).is_file() {
            return fail(format!("{}(): the {} file '{}' does not exist", who, what, f));
        }
    }
    let c = run_openssl(&["x509", "-in", cert, "-noout", "-pubkey"], who)?;
    if !c.status.success() {
        return fail(format!("{}(): '{}' is not a valid PEM certificate: {}", who, cert, last_line(&c.stderr)));
    }
    let k = run_openssl(&["pkey", "-in", key, "-pubout"], who)?;
    if !k.status.success() {
        return fail(format!("{}(): '{}' is not a valid unencrypted PEM private key: {}", who, key, last_line(&k.stderr)));
    }
    let norm = |b: &[u8]| String::from_utf8_lossy(b).replace("\r\n", "\n").trim().to_string();
    if norm(&c.stdout) != norm(&k.stdout) {
        return fail(format!("{}(): the private key '{}' does not match the certificate '{}'", who, key, cert));
    }
    Ok(())
}

fn last_line(b: &[u8]) -> String {
    String::from_utf8_lossy(b).lines().filter(|l| !l.trim().is_empty()).last().unwrap_or("").trim().to_string()
}

fn free_loopback_port() -> std::io::Result<u16> {
    let l = TcpListener::bind(("127.0.0.1", 0))?;
    Ok(l.local_addr()?.port())
}

fn pump(mut from: TcpStream, mut to: TcpStream, tag: &str) {
    let mut b = [0u8; 16384];
    let mut total = 0usize;
    loop {
        match from.read(&mut b) {
            Ok(0) => {
                tdbg!("{} EOF after {} bytes", tag, total);
                break;
            }
            Ok(n) => {
                total += n;
                if let Err(e) = to.write_all(&b[..n]) {
                    tdbg!("{} write error {:?} after {} bytes", tag, e, total);
                    break;
                }
            }
            Err(e) => {
                tdbg!("{} read error {:?} after {} bytes", tag, e, total);
                break;
            }
        }
    }
    let _ = to.shutdown(Shutdown::Write);
    let _ = from.shutdown(Shutdown::Read);
}

/// The command that terminates TLS for one connection: it listens on 127.0.0.1:`port`, takes one connection, speaks TLS
/// to it, and passes the plain text through its stdin (to the client) and stdout (from the client).
///
/// Everywhere but Windows this is `openssl s_server`. On Windows the `openssl s_server` build was found to block on its
/// piped standard input (it only handles the network while stdin delivers data), so a small PowerShell script using
/// .NET `SslStream` does the TLS work there; `openssl` converts the PEM files to a one-off PKCS#12 file, which the script
/// deletes as soon as it has loaded it.
#[cfg(not(windows))]
fn terminator_command(port: u16, cert: &str, key: &str, _who: &str) -> Result<Command, VmError> {
    let mut cmd = Command::new("openssl");
    cmd.arg("s_server").arg("-quiet").arg("-no_ign_eof").arg("-naccept").arg("1");
    cmd.arg("-accept").arg(format!("127.0.0.1:{}", port)).arg("-cert").arg(cert).arg("-key").arg(key);
    Ok(cmd)
}

#[cfg(windows)]
fn terminator_command(port: u16, cert: &str, key: &str, who: &str) -> Result<Command, VmError> {
    use std::sync::atomic::{AtomicU64, Ordering};
    static SEQ: AtomicU64 = AtomicU64::new(0);
    let pfx = std::env::temp_dir().join(format!("killer_tls_{}_{}.pfx", std::process::id(), SEQ.fetch_add(1, Ordering::SeqCst)));
    let pfx_s = pfx.to_string_lossy().to_string();
    let made = run_openssl(&["pkcs12", "-export", "-inkey", key, "-in", cert, "-out", &pfx_s, "-passout", "pass:killer"], who)?;
    if !made.status.success() {
        let _ = std::fs::remove_file(&pfx);
        return fail(format!("{}(): cannot prepare the certificate for TLS: {}", who, last_line(&made.stderr)));
    }
    let q = pfx_s.replace('\'', "''");
    let script = format!(
        "$ErrorActionPreference='Stop'
$ProgressPreference='SilentlyContinue'
$cert=New-Object System.Security.Cryptography.X509Certificates.X509Certificate2('{q}','killer')
Remove-Item -LiteralPath '{q}' -Force -ErrorAction SilentlyContinue
$l=New-Object System.Net.Sockets.TcpListener([System.Net.IPAddress]::Loopback,{port})
$l.Start()
$c=$l.AcceptTcpClient()
$l.Stop()
$ssl=New-Object System.Net.Security.SslStream($c.GetStream(),$false)
try {{
  $ssl.AuthenticateAsServer($cert)
}} catch {{
  $c.Client.Shutdown([System.Net.Sockets.SocketShutdown]::Send)
  $c.Client.ReceiveTimeout=1000
  $b=New-Object byte[] 4096
  try {{ while ($c.Client.Receive($b) -gt 0) {{}} }} catch {{}}
  $c.Close()
  exit 1
}}
$in=[Console]::OpenStandardInput()
$out=[Console]::OpenStandardOutput()
$up=$ssl.CopyToAsync($out)
$down=$in.CopyToAsync($ssl)
$first=[System.Threading.Tasks.Task]::WaitAny([System.Threading.Tasks.Task[]]@($up,$down))
if ($first -eq 1) {{
  $ssl.Flush()
  $c.Client.Shutdown([System.Net.Sockets.SocketShutdown]::Send)
  [void]$up.Wait(3000)
}}
$ssl.Dispose()
$c.Close()
"
    );
    let mut utf16 = Vec::with_capacity(script.len() * 2);
    for u in script.encode_utf16() {
        utf16.extend_from_slice(&u.to_le_bytes());
    }
    let mut cmd = Command::new("powershell");
    cmd.args(["-NoProfile", "-NonInteractive", "-EncodedCommand", &base64(&utf16)]);
    Ok(cmd)
}

#[cfg(windows)]
fn base64(data: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    for chunk in data.chunks(3) {
        let n = (chunk[0] as u32) << 16 | (*chunk.get(1).unwrap_or(&0) as u32) << 8 | *chunk.get(2).unwrap_or(&0) as u32;
        out.push(T[(n >> 18) as usize & 63] as char);
        out.push(T[(n >> 12) as usize & 63] as char);
        out.push(if chunk.len() > 1 { T[(n >> 6) as usize & 63] as char } else { '=' });
        out.push(if chunk.len() > 2 { T[n as usize & 63] as char } else { '=' });
    }
    out
}

/// Start TLS termination for an accepted connection. Returns at once; the handshake happens in the background and the
/// first read on `stdout` waits for it.
pub fn terminate(raw: TcpStream, cert: &str, key: &str, who: &str) -> Result<Terminated, VmError> {
    let port = match free_loopback_port() {
        Ok(p) => p,
        Err(e) => return fail(format!("{}(): cannot find a free local port for TLS: {}", who, e)),
    };
    let mut cmd = terminator_command(port, cert, key, who)?;
    cmd.stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) => return fail(format!("{}(): cannot start the TLS helper process ({})", who, e)),
    };
    let (stdin, stdout, stderr) = (child.stdin.take().unwrap(), child.stdout.take().unwrap(), child.stderr.take().unwrap());
    let _ = raw.set_nonblocking(false);
    let _ = raw.set_read_timeout(None);
    let (r1, r2) = match (raw.try_clone(), raw.try_clone()) {
        (Ok(a), Ok(b)) => (a, b),
        _ => {
            let _ = child.kill();
            let _ = child.wait();
            return fail(format!("{}(): cannot duplicate the client socket", who));
        }
    };
    drop(raw);
    std::thread::spawn(move || {
        // wait for s_server to start listening
        let deadline = Instant::now() + Duration::from_secs(15);
        let inner = loop {
            match TcpStream::connect(("127.0.0.1", port)) {
                Ok(s) => break s,
                Err(e) if Instant::now() < deadline => { let _ = e; std::thread::sleep(Duration::from_millis(15)) }
                Err(_) => {
                    let _ = r1.shutdown(Shutdown::Both);
                    return;
                }
            }
        };
        let _ = inner.set_nodelay(true);
        let inner2 = match inner.try_clone() {
            Ok(s) => s,
            Err(_) => return,
        };
        let r1b = r1.try_clone();
        let (done_tx, done_rx) = std::sync::mpsc::channel::<()>();
        std::thread::spawn(move || {
            pump(r2, inner2, "client->helper"); // client -> openssl
            let _ = done_tx.send(());
        });
        pump(inner, r1, "helper->client"); // openssl -> client; ends with a FIN (shutdown Write) towards the client
        // Closing a TCP socket that still has unread incoming data makes the stack send RST, and an RST discards the
        // reply the client has not read yet (curl exit 56). So only half-close towards the client and wait for the
        // client's own FIN (it sends it after reading the reply and its close_notify) before closing for good.
        let waited = done_rx.recv_timeout(Duration::from_secs(10));
        if let Ok(r) = r1b {
            tdbg!("bridge closing client socket (client finished: {})", waited.is_ok());
            let _ = r.shutdown(Shutdown::Both);
        }
    });
    Ok(Terminated { child, stdin, stdout, stderr })
}

/// Give a child that was told to finish (its stdin closed) a moment to flush and exit, then kill it.
pub fn reap(child: &mut Child, grace: Duration) {
    let end = Instant::now() + grace;
    while Instant::now() < end {
        match child.try_wait() {
            Ok(Some(_)) => return,
            Ok(None) => std::thread::sleep(Duration::from_millis(5)),
            Err(_) => break,
        }
    }
    let _ = child.kill();
    let _ = child.wait();
}

/// A TLS front for plain-text servers: listens on `front`, and for every connection terminates TLS and forwards the
/// plain text to `backend` (a loopback address). Used by `serve(..., {tls: {cert, key}})`. Stops when `stop` is set;
/// `active` counts the connections still being served, so the caller can let them finish before it exits.
pub fn spawn_front(front: TcpListener, backend: std::net::SocketAddr, cert: String, key: String, stop: std::sync::Arc<std::sync::atomic::AtomicBool>, active: std::sync::Arc<std::sync::atomic::AtomicUsize>) {
    use std::sync::atomic::Ordering;
    let _ = front.set_nonblocking(true);
    std::thread::spawn(move || {
        while !stop.load(Ordering::SeqCst) {
            match front.accept() {
                Ok((raw, _)) => {
                    let (cert, key) = (cert.clone(), key.clone());
                    struct Active(std::sync::Arc<std::sync::atomic::AtomicUsize>);
                    impl Drop for Active {
                        fn drop(&mut self) {
                            self.0.fetch_sub(1, Ordering::SeqCst);
                        }
                    }
                    active.fetch_add(1, Ordering::SeqCst);
                    let token = Active(active.clone());
                    std::thread::spawn(move || {
                        let _token = token;
                        let t = match terminate(raw, &cert, &key, "serve") {
                            Ok(t) => t,
                            Err(_) => return,
                        };
                        let Terminated { mut child, mut stdin, mut stdout, stderr } = t;
                        if dbg_on() {
                            std::thread::spawn(move || {
                                let mut e = stderr;
                                let mut t = String::new();
                                let _ = e.read_to_string(&mut t);
                                if !t.trim().is_empty() {
                                    tdbg!("helper stderr: {}", t.trim());
                                }
                            });
                        } else {
                            drop(stderr);
                        }
                        // Wait for the first plain-text bytes before touching the backend: the helper start-up and the
                        // handshake can take seconds under load, and a backend connection that sits idle meanwhile holds
                        // up the (single-threaded) HTTP loop and runs into its request timeout (408).
                        let mut first = [0u8; 16384];
                        let first_n = match stdout.read(&mut first) {
                            Ok(n) if n > 0 => n,
                            _ => {
                                tdbg!("no request arrived (handshake failed or client left)");
                                drop(stdin);
                                reap(&mut child, Duration::from_millis(200));
                                return;
                            }
                        };
                        let back = match TcpStream::connect(backend) {
                            Ok(b) => b,
                            Err(_) => {
                                drop(stdin);
                                reap(&mut child, Duration::from_millis(200));
                                return;
                            }
                        };
                        let mut back_r = match back.try_clone() {
                            Ok(b) => b,
                            Err(_) => return,
                        };
                        let mut back_w = back;
                        if back_w.write_all(&first[..first_n]).is_err() {
                            drop(stdin);
                            reap(&mut child, Duration::from_millis(200));
                            return;
                        }
                        // client -> backend
                        let up = std::thread::spawn(move || {
                            let mut b = [0u8; 16384];
                            while let Ok(n) = stdout.read(&mut b) {
                                if n == 0 || back_w.write_all(&b[..n]).is_err() {
                                    break;
                                }
                            }
                            let _ = back_w.shutdown(Shutdown::Write);
                        });
                        // backend -> client
                        let mut b = [0u8; 16384];
                        while let Ok(n) = back_r.read(&mut b) {
                            if n == 0 || stdin.write_all(&b[..n]).is_err() || stdin.flush().is_err() {
                                break;
                            }
                        }
                        drop(stdin); // EOF: s_server finishes the connection
                        reap(&mut child, Duration::from_secs(20));
                        let _ = up.join();
                    });
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => std::thread::sleep(Duration::from_millis(5)),
                Err(_) => break,
            }
        }
    });
}
