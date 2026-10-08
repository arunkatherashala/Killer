//! Package registries: a directory on disk (`file:///...` or a plain path), optionally served over
//! HTTP, and clients for both.
//!
//! Layout of a registry directory:
//! ```text
//! registry/<name>/<version>/killer.toml     manifest of that release
//! registry/<name>/<version>/package.pack    archive of the package files (see pkg_files)
//! registry/<name>/<version>/sha256          content hash of the files in the archive
//! ```
//! Releases are immutable. HTTP API (all on std `TcpListener`, no dependencies):
//! ```text
//! GET  /health
//! GET  /api/packages                          JSON list of packages with their latest version
//! GET  /api/package/<name>                    JSON: name and versions
//! GET  /api/package/<name>/versions           one version per line
//! GET  /api/package/<name>/<version>          JSON: version, sha256
//! GET  /api/package/<name>/<version>/manifest raw killer.toml
//! GET  /download/<name>/<version>             raw archive; X-Killer-Sha256 header
//! POST /api/publish/<name>/<version>          body = archive; Authorization: Bearer <token>
//! ```

use crate::package_manager::Version;
use crate::pkg_files;
use crate::pkg_manifest::{valid_name, Manifest};
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

const MAX_BODY: usize = 64 * 1024 * 1024;

pub struct Release {
    pub manifest: String,
    pub pack: Vec<u8>,
    pub sha256: String,
}

// ---------------------------------------------------------------- directory registry

pub fn release_dir(root: &Path, name: &str, version: &Version) -> Result<PathBuf, String> {
    if !valid_name(name) {
        return Err(format!("invalid package name '{}'", name));
    }
    Ok(root.join(name).join(version.to_string()))
}

pub fn dir_versions(root: &Path, name: &str) -> Result<Vec<Version>, String> {
    if !valid_name(name) {
        return Err(format!("invalid package name '{}'", name));
    }
    let d = root.join(name);
    if !d.is_dir() {
        return Ok(Vec::new());
    }
    let mut v: Vec<Version> = std::fs::read_dir(&d)
        .map_err(|e| e.to_string())?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().join("package.pack").is_file())
        .filter_map(|e| Version::parse(&e.file_name().to_string_lossy()).ok())
        .collect();
    v.sort();
    Ok(v)
}

pub fn dir_names(root: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(root)
        .into_iter()
        .flatten()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir())
        .map(|e| e.file_name().to_string_lossy().to_string())
        .filter(|n| valid_name(n))
        .collect();
    names.sort();
    names
}

pub fn dir_release(root: &Path, name: &str, version: &Version) -> Result<Release, String> {
    let d = release_dir(root, name, version)?;
    let rd = |f: &str| std::fs::read(d.join(f)).map_err(|_| format!("package `{} {}` is not in the registry", name, version));
    Ok(Release {
        manifest: String::from_utf8_lossy(&rd("killer.toml")?).to_string(),
        pack: rd("package.pack")?,
        sha256: String::from_utf8_lossy(&rd("sha256")?).trim().to_string(),
    })
}

/// Validate an archive and store it as a new immutable release. Returns the content hash.
pub fn store_release(root: &Path, name: &str, version: &Version, pack: &[u8]) -> Result<String, String> {
    let files = pkg_files::unpack(pack)?;
    let manifest_bytes = files.iter().find(|(p, _)| p == "killer.toml").map(|(_, b)| b.clone()).ok_or("the package has no killer.toml")?;
    let manifest = Manifest::parse(&String::from_utf8_lossy(&manifest_bytes))?;
    if manifest.name != name || manifest.version != *version {
        return Err(format!("killer.toml says {} {} but {} {} is being published", manifest.name, manifest.version, name, version));
    }
    let d = release_dir(root, name, version)?;
    if d.join("package.pack").exists() {
        return Err(format!("{} {} is already published (releases are immutable; bump the version)", name, version));
    }
    let hash = pkg_files::content_hash(&files);
    std::fs::create_dir_all(&d).map_err(|e| format!("cannot create {}: {}", d.display(), e))?;
    let put = |f: &str, b: &[u8]| std::fs::write(d.join(f), b).map_err(|e| format!("cannot write {}: {}", d.join(f).display(), e));
    put("killer.toml", &manifest_bytes)?;
    put("package.pack", pack)?;
    put("sha256", hash.as_bytes())?;
    Ok(hash)
}

// ---------------------------------------------------------------- client

#[derive(Clone, Debug)]
pub enum Registry {
    Dir(PathBuf),
    Http { host: String, port: u16, url: String },
}

/// `file:///dir`, `file://C:/dir`, a plain path, or `http://host[:port]`.
pub fn open(url: &str) -> Result<Registry, String> {
    let u = url.trim();
    if let Some(rest) = u.strip_prefix("http://") {
        let hostport = rest.trim_end_matches('/');
        let (host, port) = match hostport.rsplit_once(':') {
            Some((h, p)) => (h.to_string(), p.parse::<u16>().map_err(|_| format!("bad registry port in '{}'", u))?),
            None => (hostport.to_string(), 80),
        };
        return Ok(Registry::Http { host, port, url: format!("http://{}", hostport) });
    }
    if u.starts_with("https://") {
        return Err("https registries are not supported (no TLS without external crates); use http://localhost or file://".to_string());
    }
    let path = if let Some(rest) = u.strip_prefix("file://") {
        // file:///C:/x -> C:/x on Windows; file:///srv/x -> /srv/x
        let r = rest.strip_prefix('/').filter(|r| r.as_bytes().get(1) == Some(&b':')).unwrap_or(rest);
        r.to_string()
    } else {
        u.to_string()
    };
    Ok(Registry::Dir(PathBuf::from(path)))
}

impl Registry {
    /// Canonical text used in killer.lock.
    pub fn url(&self) -> String {
        match self {
            Registry::Dir(p) => format!("file:///{}", p.to_string_lossy().replace('\\', "/").trim_start_matches('/')),
            Registry::Http { url, .. } => url.clone(),
        }
    }

    pub fn versions(&self, name: &str) -> Result<Vec<Version>, String> {
        match self {
            Registry::Dir(root) => dir_versions(root, name),
            Registry::Http { .. } => {
                if !valid_name(name) {
                    return Err(format!("invalid package name '{}'", name));
                }
                match self.get(&format!("/api/package/{}/versions", name))? {
                    (200, _, body) => Ok(String::from_utf8_lossy(&body).lines().filter_map(|l| Version::parse(l.trim()).ok()).collect()),
                    (404, _, _) => Ok(Vec::new()),
                    (s, _, b) => Err(format!("registry error {}: {}", s, String::from_utf8_lossy(&b).trim())),
                }
            }
        }
    }

    pub fn manifest(&self, name: &str, v: &Version) -> Result<String, String> {
        match self {
            Registry::Dir(root) => Ok(dir_release(root, name, v)?.manifest),
            Registry::Http { .. } => match self.get(&format!("/api/package/{}/{}/manifest", name, v))? {
                (200, _, body) => Ok(String::from_utf8_lossy(&body).to_string()),
                (s, _, b) => Err(format!("registry error {}: {}", s, String::from_utf8_lossy(&b).trim())),
            },
        }
    }

    /// Download a release: archive and the content hash the registry claims for it.
    pub fn fetch(&self, name: &str, v: &Version) -> Result<Release, String> {
        match self {
            Registry::Dir(root) => dir_release(root, name, v),
            Registry::Http { .. } => {
                let (status, headers, body) = self.get(&format!("/download/{}/{}", name, v))?;
                if status != 200 {
                    return Err(format!("registry error {}: {}", status, String::from_utf8_lossy(&body).trim()));
                }
                let sha = headers
                    .lines()
                    .find_map(|l| l.strip_prefix("x-killer-sha256:").map(|v| v.trim().to_string()))
                    .ok_or("registry did not send a checksum")?;
                let manifest = self.manifest(name, v)?;
                Ok(Release { manifest, pack: body, sha256: sha })
            }
        }
    }

    pub fn publish(&self, name: &str, v: &Version, pack: &[u8], token: Option<&str>) -> Result<String, String> {
        match self {
            Registry::Dir(root) => {
                std::fs::create_dir_all(root).map_err(|e| format!("cannot create registry {}: {}", root.display(), e))?;
                store_release(root, name, v, pack)
            }
            Registry::Http { host, port, .. } => {
                let token = token.ok_or("publishing to an HTTP registry needs a token (--token or KILLER_REGISTRY_TOKEN)")?;
                let (s, _, b) = http_request(host, *port, "POST", &format!("/api/publish/{}/{}", name, v), &[("Authorization", format!("Bearer {}", token))], pack)?;
                if s == 201 || s == 200 {
                    Ok(String::from_utf8_lossy(&b).trim().to_string())
                } else {
                    Err(format!("publish rejected ({}): {}", s, String::from_utf8_lossy(&b).trim()))
                }
            }
        }
    }

    pub fn names(&self) -> Result<Vec<String>, String> {
        match self {
            Registry::Dir(root) => Ok(dir_names(root)),
            Registry::Http { .. } => match self.get("/api/packages")? {
                (200, _, body) => Ok(extract_names(&String::from_utf8_lossy(&body))),
                (s, _, b) => Err(format!("registry error {}: {}", s, String::from_utf8_lossy(&b).trim())),
            },
        }
    }

    fn get(&self, path: &str) -> Result<(u16, String, Vec<u8>), String> {
        match self {
            Registry::Http { host, port, .. } => http_request(host, *port, "GET", path, &[], &[]),
            _ => Err("not an HTTP registry".to_string()),
        }
    }
}

fn extract_names(json: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = json;
    while let Some(i) = rest.find("\"name\":\"") {
        rest = &rest[i + 8..];
        if let Some(e) = rest.find('"') {
            out.push(rest[..e].to_string());
        }
    }
    out
}

/// Minimal HTTP/1.1 client. Returns (status, lower-cased header block, body).
pub fn http_request(host: &str, port: u16, method: &str, path: &str, headers: &[(&str, String)], body: &[u8]) -> Result<(u16, String, Vec<u8>), String> {
    let addr = format!("{}:{}", host, port);
    let mut s = TcpStream::connect(&addr).map_err(|e| format!("cannot reach registry {}: {}", addr, e))?;
    let _ = s.set_read_timeout(Some(Duration::from_secs(30)));
    let mut req = format!("{} {} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\nContent-Length: {}\r\n", method, path, host, body.len());
    for (k, v) in headers {
        req.push_str(&format!("{}: {}\r\n", k, v));
    }
    req.push_str("\r\n");
    s.write_all(req.as_bytes()).and_then(|_| s.write_all(body)).map_err(|e| format!("registry write failed: {}", e))?;
    let mut raw = Vec::new();
    s.read_to_end(&mut raw).map_err(|e| format!("registry read failed: {}", e))?;
    let split = raw.windows(4).position(|w| w == b"\r\n\r\n").ok_or("malformed registry response")?;
    let head = String::from_utf8_lossy(&raw[..split]).to_string();
    let status = head.lines().next().and_then(|l| l.split_whitespace().nth(1)).and_then(|c| c.parse::<u16>().ok()).ok_or("malformed registry status line")?;
    let headers_lc = head.lines().skip(1).map(|l| l.to_lowercase()).collect::<Vec<_>>().join("\n");
    Ok((status, headers_lc, raw[split + 4..].to_vec()))
}

// ---------------------------------------------------------------- server

pub struct Response {
    pub status: u16,
    pub content_type: &'static str,
    pub body: Vec<u8>,
    pub extra: Vec<(String, String)>,
}

fn text(status: u16, s: impl Into<String>) -> Response {
    Response { status, content_type: "text/plain; charset=utf-8", body: s.into().into_bytes(), extra: vec![] }
}

fn json_str(s: &str) -> String {
    format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', "\\n"))
}

/// Constant-time-ish token comparison.
fn token_ok(given: &str, tokens: &[String]) -> bool {
    tokens.iter().fold(false, |acc, t| {
        let same = t.len() == given.len() && t.bytes().zip(given.bytes()).fold(0u8, |d, (a, b)| d | (a ^ b)) == 0;
        acc | same
    })
}

/// Route one request against the registry directory `root`.
pub fn handle(root: &Path, tokens: &[String], method: &str, path: &str, auth: Option<&str>, body: &[u8]) -> Response {
    let path = path.split('?').next().unwrap_or(path);
    let parts: Vec<&str> = path.trim_matches('/').split('/').collect();
    let parse_v = |s: &str| Version::parse(s);
    match (method, parts.as_slice()) {
        ("GET", [""]) | ("GET", ["health"]) => text(200, "killer package registry ok\n"),
        ("GET", ["api", "packages"]) => {
            let items: Vec<String> = dir_names(root)
                .into_iter()
                .filter_map(|n| {
                    let v = dir_versions(root, &n).ok()?;
                    let latest = v.last()?.to_string();
                    Some(format!("{{\"name\":{},\"latest\":{},\"versions\":{}}}", json_str(&n), json_str(&latest), v.len()))
                })
                .collect();
            Response { status: 200, content_type: "application/json", body: format!("[{}]", items.join(",")).into_bytes(), extra: vec![] }
        }
        ("GET", ["api", "package", name]) => match dir_versions(root, name) {
            Ok(v) if !v.is_empty() => {
                let vs: Vec<String> = v.iter().map(|x| json_str(&x.to_string())).collect();
                Response { status: 200, content_type: "application/json", body: format!("{{\"name\":{},\"versions\":[{}]}}", json_str(name), vs.join(",")).into_bytes(), extra: vec![] }
            }
            Ok(_) => text(404, format!("no such package: {}\n", name)),
            Err(e) => text(400, e),
        },
        ("GET", ["api", "package", name, "versions"]) => match dir_versions(root, name) {
            Ok(v) if !v.is_empty() => text(200, v.iter().map(|x| format!("{}\n", x)).collect::<String>()),
            Ok(_) => text(404, format!("no such package: {}\n", name)),
            Err(e) => text(400, e),
        },
        ("GET", ["api", "package", name, ver]) => match parse_v(ver).and_then(|v| dir_release(root, name, &v)) {
            Ok(r) => Response { status: 200, content_type: "application/json", body: format!("{{\"name\":{},\"version\":{},\"sha256\":{}}}", json_str(name), json_str(ver), json_str(&r.sha256)).into_bytes(), extra: vec![] },
            Err(e) => text(404, e),
        },
        ("GET", ["api", "package", name, ver, "manifest"]) => match parse_v(ver).and_then(|v| dir_release(root, name, &v)) {
            Ok(r) => text(200, r.manifest),
            Err(e) => text(404, e),
        },
        ("GET", ["download", name, ver]) => match parse_v(ver).and_then(|v| dir_release(root, name, &v)) {
            Ok(r) => Response { status: 200, content_type: "application/octet-stream", body: r.pack, extra: vec![("X-Killer-Sha256".to_string(), r.sha256)] },
            Err(e) => text(404, e),
        },
        ("POST", ["api", "publish", name, ver]) => {
            if tokens.is_empty() {
                return text(403, "publishing is disabled: the registry was started without a token\n");
            }
            let given = auth.and_then(|a| a.strip_prefix("Bearer ")).unwrap_or("");
            if !token_ok(given, tokens) {
                return text(401, "missing or invalid token\n");
            }
            match parse_v(ver).and_then(|v| store_release(root, name, &v, body)) {
                Ok(h) => Response { status: 201, content_type: "text/plain; charset=utf-8", body: format!("published {} {} sha256={}\n", name, ver, h).into_bytes(), extra: vec![] },
                Err(e) if e.contains("already published") => text(409, e),
                Err(e) => text(400, e),
            }
        }
        _ => text(404, "not found\n"),
    }
}

fn serve_connection(mut s: TcpStream, root: &Path, tokens: &[String]) {
    let _ = s.set_read_timeout(Some(Duration::from_secs(30)));
    let mut buf = Vec::new();
    let mut chunk = [0u8; 8192];
    let header_end = loop {
        if let Some(i) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
            break i;
        }
        match s.read(&mut chunk) {
            Ok(0) | Err(_) => return,
            Ok(n) => buf.extend_from_slice(&chunk[..n]),
        }
        if buf.len() > 64 * 1024 {
            return;
        }
    };
    let head = String::from_utf8_lossy(&buf[..header_end]).to_string();
    let mut lines = head.lines();
    let first = lines.next().unwrap_or("");
    let mut it = first.split_whitespace();
    let (method, path) = (it.next().unwrap_or(""), it.next().unwrap_or("/"));
    let mut len = 0usize;
    let mut auth = None;
    for l in lines {
        if let Some((k, v)) = l.split_once(':') {
            match k.trim().to_ascii_lowercase().as_str() {
                "content-length" => len = v.trim().parse().unwrap_or(0),
                "authorization" => auth = Some(v.trim().to_string()),
                _ => {}
            }
        }
    }
    let resp = if len > MAX_BODY {
        text(413, "body too large\n")
    } else {
        let mut body = buf[header_end + 4..].to_vec();
        while body.len() < len {
            match s.read(&mut chunk) {
                Ok(0) | Err(_) => break,
                Ok(n) => body.extend_from_slice(&chunk[..n]),
            }
        }
        body.truncate(len);
        handle(root, tokens, method, path, auth.as_deref(), &body)
    };
    let reason = match resp.status {
        200 => "OK",
        201 => "Created",
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        409 => "Conflict",
        413 => "Payload Too Large",
        _ => "Error",
    };
    let mut head = format!("HTTP/1.1 {} {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n", resp.status, reason, resp.content_type, resp.body.len());
    for (k, v) in &resp.extra {
        head.push_str(&format!("{}: {}\r\n", k, v));
    }
    head.push_str("\r\n");
    let _ = s.write_all(head.as_bytes());
    let _ = s.write_all(&resp.body);
}

pub struct Server {
    pub addr: SocketAddr,
    stop: Arc<AtomicBool>,
    thread: Option<std::thread::JoinHandle<()>>,
}

impl Server {
    pub fn stop(mut self) {
        self.stop.store(true, Ordering::SeqCst);
        let _ = TcpStream::connect(self.addr); // wake the accept loop
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

/// Bind `bind` (e.g. `127.0.0.1:0`) and serve the registry in a background thread.
pub fn spawn_server(root: PathBuf, tokens: Vec<String>, bind: &str) -> Result<Server, String> {
    let listener = TcpListener::bind(bind).map_err(|e| format!("cannot bind {}: {}", bind, e))?;
    let addr = listener.local_addr().map_err(|e| e.to_string())?;
    let stop = Arc::new(AtomicBool::new(false));
    let flag = stop.clone();
    let root = Arc::new(root);
    let tokens = Arc::new(tokens);
    let thread = std::thread::spawn(move || {
        for conn in listener.incoming() {
            if flag.load(Ordering::SeqCst) {
                break;
            }
            if let Ok(s) = conn {
                let (r, t) = (root.clone(), tokens.clone());
                std::thread::spawn(move || serve_connection(s, &r, &t));
            }
        }
    });
    Ok(Server { addr, stop, thread: Some(thread) })
}

/// Tokens from `registry/tokens.txt` (one per line) plus extras.
pub fn load_tokens(root: &Path, extra: Option<String>) -> Vec<String> {
    let mut t: Vec<String> = std::fs::read_to_string(root.join("tokens.txt"))
        .unwrap_or_default()
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .collect();
    t.extend(extra.filter(|e| !e.is_empty()));
    t
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("killer_reg_{}_{}", std::process::id(), name));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn pack_of(name: &str, ver: &str) -> Vec<u8> {
        pkg_files::pack(&[
            ("killer.toml".to_string(), format!("[package]\nname = \"{}\"\nversion = \"{}\"\n", name, ver).into_bytes()),
            ("src/lib.killer".to_string(), b"fn f() {\n  return 1\n}\n".to_vec()),
        ])
    }

    #[test]
    fn directory_registry_publish_list_fetch_and_immutability() {
        let d = tmp("dir");
        let reg = open(&format!("file:///{}", d.to_string_lossy().replace('\\', "/").trim_start_matches('/'))).unwrap();
        let v1 = Version::parse("1.0.0").unwrap();
        let h = reg.publish("lib", &v1, &pack_of("lib", "1.0.0"), None).unwrap();
        reg.publish("lib", &Version::parse("1.10.0").unwrap(), &pack_of("lib", "1.10.0"), None).unwrap();
        reg.publish("lib", &Version::parse("1.2.0").unwrap(), &pack_of("lib", "1.2.0"), None).unwrap();
        assert_eq!(reg.versions("lib").unwrap().iter().map(|v| v.to_string()).collect::<Vec<_>>(), vec!["1.0.0", "1.2.0", "1.10.0"]);
        let rel = reg.fetch("lib", &v1).unwrap();
        assert_eq!(rel.sha256, h);
        assert_eq!(pkg_files::content_hash(&pkg_files::unpack(&rel.pack).unwrap()), h);
        assert!(reg.publish("lib", &v1, &pack_of("lib", "1.0.0"), None).unwrap_err().contains("already published"));
        // name / version in the archive must match
        assert!(reg.publish("other", &v1, &pack_of("lib", "1.0.0"), None).unwrap_err().contains("killer.toml says"));
        assert!(reg.versions("nothing").unwrap().is_empty());
        assert!(reg.versions("../x").is_err());
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn http_registry_requires_a_token_and_serves_downloads() {
        let d = tmp("http");
        let srv = spawn_server(d.clone(), vec!["s3cret".to_string()], "127.0.0.1:0").unwrap();
        let reg = open(&format!("http://127.0.0.1:{}", srv.addr.port())).unwrap();
        let v = Version::parse("2.0.0").unwrap();
        let pack = pack_of("net", "2.0.0");
        assert!(reg.publish("net", &v, &pack, None).is_err());
        assert!(reg.publish("net", &v, &pack, Some("wrong")).unwrap_err().contains("401"));
        let hash = reg.publish("net", &v, &pack, Some("s3cret")).unwrap();
        assert!(hash.contains("sha256="));
        assert!(reg.publish("net", &v, &pack, Some("s3cret")).unwrap_err().contains("409"));
        assert_eq!(reg.versions("net").unwrap(), vec![v.clone()]);
        assert!(reg.versions("absent").unwrap().is_empty());
        let rel = reg.fetch("net", &v).unwrap();
        assert_eq!(rel.pack, pack);
        assert!(rel.manifest.contains("name = \"net\""));
        assert_eq!(reg.names().unwrap(), vec!["net".to_string()]);
        srv.stop();
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn publishing_is_disabled_without_tokens() {
        let d = tmp("notoken");
        let r = handle(&d, &[], "POST", "/api/publish/a/1.0.0", Some("Bearer x"), b"");
        assert_eq!(r.status, 403);
        let _ = std::fs::remove_dir_all(&d);
    }
}
