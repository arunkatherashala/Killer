/// KPM Registry Server — Killer Package Manager public registry
///
/// Zero external dependencies — pure std::net TCP server.
/// Default port: 8080  (override with KPM_PORT env var)
///
/// API:
///   GET  /api/packages              — list all packages (JSON)
///   GET  /api/package/{name}        — package metadata (JSON)
///   GET  /api/package/{name}/{ver}  — specific version metadata (JSON)
///   POST /api/publish               — publish a new package (JSON body)
///   GET  /health                    — health check
///   GET  /                          — registry info page
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

// ── Package data structures ───────────────────────────────────────────────

#[derive(Clone, Debug)]
struct PackageVersion {
    version: String,
    description: String,
    published_at: u64,
    author: String,
    keywords: Vec<String>,
    source: String, // inline source code or download URL
}

#[derive(Clone, Debug)]
struct Package {
    name: String,
    versions: Vec<PackageVersion>,
}

type Registry = Arc<Mutex<HashMap<String, Package>>>;

fn now_ts() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs()
}

// ── Built-in starter packages ─────────────────────────────────────────────

fn seed_registry() -> HashMap<String, Package> {
    let mut reg = HashMap::new();

    let packages = vec![
        ("killer-math", "1.0.0", "Extended math functions: matrix ops, stats, FFT",
         "# killer-math\nfn matrix_mul(a, b) { # stub }\nfn fft(arr) { # stub }",
         vec!["math", "matrix", "fft"]),
        ("killer-http", "1.1.0", "HTTP client with retry, auth, JSON helpers",
         "# killer-http\nfn get_json(url) { json_parse(http_get(url)) }\nfn post_json(url, data) { http_post_json(url, data) }",
         vec!["http", "rest", "api"]),
        ("killer-fs", "1.0.0", "File system utilities: glob, walk, watch",
         "# killer-fs\nfn walk_dir(path) { listDir(path) }\nfn read_json(path) { json_parse(readFile(path)) }",
         vec!["fs", "files", "io"]),
        ("killer-date", "1.0.0", "Date/time helpers: parse, format, diff",
         "# killer-date\nfn today() { date_format(date_now(), \"%Y-%m-%d\") }",
         vec!["date", "time", "datetime"]),
        ("killer-test", "1.0.0", "Testing framework: assert, describe, it, expect",
         "# killer-test\nfn describe(name, body) { println(\"Suite: \" + name) }\nfn it(name, body) { println(\"  test: \" + name) }",
         vec!["test", "assert", "spec"]),
        ("killer-csv", "1.0.0", "CSV parse/stringify for data pipelines",
         "# killer-csv\nfn parse(s) { # stub - returns array of arrays }\nfn stringify(rows) { # stub }",
         vec!["csv", "data", "io"]),
        ("killer-crypto", "1.0.0", "SHA-256, AES-256-GCM, HMAC — zero deps",
         "# killer-crypto\nfn sha256(s) { # stub }\nfn hmac(key, msg) { # stub }",
         vec!["crypto", "hash", "security"]),
        ("killer-queue", "1.0.0", "Thread-safe queue, priority queue, ring buffer",
         "# killer-queue\nfn queue_new() { [] }\nfn queue_push(q, v) { q.push(v) }",
         vec!["queue", "async", "concurrency"]),
    ];

    for (name, version, desc, source, keywords) in packages {
        reg.insert(name.to_string(), Package {
            name: name.to_string(),
            versions: vec![PackageVersion {
                version: version.to_string(),
                description: desc.to_string(),
                published_at: now_ts(),
                author: "killer-lang".to_string(),
                keywords: keywords.iter().map(|s| s.to_string()).collect(),
                source: source.to_string(),
            }],
        });
    }
    reg
}

// ── HTTP helpers ──────────────────────────────────────────────────────────

fn respond(stream: &mut TcpStream, status: u16, body: &str, content_type: &str) {
    let status_text = match status {
        200 => "OK", 201 => "Created", 400 => "Bad Request",
        404 => "Not Found", 405 => "Method Not Allowed", _ => "Error",
    };
    let response = format!(
        "HTTP/1.1 {} {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nAccess-Control-Allow-Origin: *\r\nConnection: close\r\n\r\n{}",
        status, status_text, content_type, body.len(), body
    );
    let _ = stream.write_all(response.as_bytes());
}

fn json_str(s: &str) -> String {
    format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', "\\n"))
}

fn package_to_json(pkg: &Package) -> String {
    let latest = pkg.versions.last().unwrap();
    let kws: Vec<String> = latest.keywords.iter().map(|k| json_str(k)).collect();
    format!(
        r#"{{"name":{},"version":{},"description":{},"author":{},"keywords":[{}],"published_at":{}}}"#,
        json_str(&pkg.name),
        json_str(&latest.version),
        json_str(&latest.description),
        json_str(&latest.author),
        kws.join(","),
        latest.published_at,
    )
}

// ── Request handler ───────────────────────────────────────────────────────

fn handle(mut stream: TcpStream, registry: &Registry) {
    let mut buf = [0u8; 8192];
    let n = match stream.read(&mut buf) { Ok(n) => n, Err(_) => return };
    let request = String::from_utf8_lossy(&buf[..n]);

    let first_line = request.lines().next().unwrap_or("");
    let parts: Vec<&str> = first_line.split_whitespace().collect();
    if parts.len() < 2 { return; }
    let method = parts[0];
    let path = parts[1];

    // Parse body for POST
    let body = if let Some(idx) = request.find("\r\n\r\n") {
        request[idx + 4..].to_string()
    } else { String::new() };

    let reg = registry.lock().unwrap();

    match (method, path) {
        ("GET", "/") | ("GET", "/index") => {
            let html = format!(
                "<h1>KPM Registry</h1><p>{} packages available.</p><p>API: <code>GET /api/packages</code></p>",
                reg.len()
            );
            drop(reg);
            respond(&mut stream, 200, &html, "text/html");
        }

        ("GET", "/health") => {
            drop(reg);
            respond(&mut stream, 200, r#"{"status":"ok"}"#, "application/json");
        }

        ("GET", "/api/packages") => {
            let list: Vec<String> = reg.values().map(package_to_json).collect();
            let json = format!("[{}]", list.join(","));
            drop(reg);
            respond(&mut stream, 200, &json, "application/json");
        }

        ("GET", p) if p.starts_with("/api/package/") => {
            let rest = &p["/api/package/".len()..];
            let segments: Vec<&str> = rest.split('/').collect();
            let pkg_name = segments[0];

            match reg.get(pkg_name) {
                Some(pkg) => {
                    let json = if segments.len() >= 2 {
                        // Specific version
                        let ver = segments[1];
                        if let Some(v) = pkg.versions.iter().find(|v| v.version == ver) {
                            let kws: Vec<String> = v.keywords.iter().map(|k| json_str(k)).collect();
                            format!(
                                r#"{{"name":{},"version":{},"description":{},"source":{},"keywords":[{}]}}"#,
                                json_str(&pkg.name), json_str(&v.version),
                                json_str(&v.description), json_str(&v.source),
                                kws.join(",")
                            )
                        } else {
                            drop(reg);
                            respond(&mut stream, 404, r#"{"error":"version not found"}"#, "application/json");
                            return;
                        }
                    } else {
                        package_to_json(pkg)
                    };
                    drop(reg);
                    respond(&mut stream, 200, &json, "application/json");
                }
                None => {
                    drop(reg);
                    respond(&mut stream, 404,
                        &format!(r#"{{"error":"package {} not found"}}"#, json_str(pkg_name)),
                        "application/json");
                }
            }
        }

        ("POST", "/api/publish") => {
            drop(reg);
            // Simple publish: body must be JSON with name, version, description, source
            let name = extract_json_str(&body, "name").unwrap_or_default();
            let version = extract_json_str(&body, "version").unwrap_or_else(|| "1.0.0".to_string());
            let description = extract_json_str(&body, "description").unwrap_or_default();
            let source = extract_json_str(&body, "source").unwrap_or_default();
            let author = extract_json_str(&body, "author").unwrap_or_else(|| "anonymous".to_string());

            if name.is_empty() {
                respond(&mut stream, 400, r#"{"error":"name required"}"#, "application/json");
                return;
            }

            let mut reg = registry.lock().unwrap();
            let pkg = reg.entry(name.clone()).or_insert(Package {
                name: name.clone(), versions: vec![],
            });
            pkg.versions.push(PackageVersion {
                version: version.clone(), description, published_at: now_ts(),
                author, keywords: vec![], source,
            });
            drop(reg);
            println!("Published: {} v{}", name, version);
            respond(&mut stream, 201,
                &format!(r#"{{"ok":true,"name":{},"version":{}}}"#, json_str(&name), json_str(&version)),
                "application/json");
        }

        _ => {
            drop(reg);
            respond(&mut stream, 404, r#"{"error":"not found"}"#, "application/json");
        }
    }
}

fn extract_json_str(json: &str, key: &str) -> Option<String> {
    let search = format!("\"{}\"", key);
    let pos = json.find(&search)?;
    let after = json[pos + search.len()..].trim_start();
    let after = after.trim_start_matches(':').trim_start();
    if after.starts_with('"') {
        let content = &after[1..];
        let end = content.find('"')?;
        Some(content[..end].to_string())
    } else {
        None
    }
}

// ── Main ──────────────────────────────────────────────────────────────────

fn main() {
    let port: u16 = std::env::var("KPM_PORT")
        .ok().and_then(|p| p.parse().ok()).unwrap_or(8080);

    let addr = format!("127.0.0.1:{}", port);
    let listener = TcpListener::bind(&addr)
        .unwrap_or_else(|e| { eprintln!("Cannot bind {}: {}", addr, e); std::process::exit(1); });

    let registry: Registry = Arc::new(Mutex::new(seed_registry()));

    println!("KPM Registry running at http://{}", addr);
    println!("  GET  /api/packages          — list all packages");
    println!("  GET  /api/package/{{name}}    — package info");
    println!("  POST /api/publish           — publish package");
    println!("  GET  /health                — health check");
    println!();
    println!("Seeded with {} packages. Press Ctrl+C to stop.", registry.lock().unwrap().len());

    for stream in listener.incoming() {
        match stream {
            Ok(s) => {
                let reg = Arc::clone(&registry);
                std::thread::spawn(move || handle(s, &reg));
            }
            Err(e) => eprintln!("Connection error: {}", e),
        }
    }
}
