//! KPM registry server: a persistent, file-backed package registry served over HTTP.
//!
//! Zero external dependencies (std `TcpListener`). Packages live in a directory
//! (`<dir>/<name>/<version>/{killer.toml, package.pack, sha256}`), so the same directory can also
//! be used directly as `registry = "file:///<dir>"` without running this server.
//!
//! USAGE: kpm-registry [--dir DIR] [--port 8080] [--bind 127.0.0.1] [--token SECRET]
//!   env: KPM_DIR, KPM_PORT, KPM_BIND, KPM_TOKEN
//! Publishing requires `Authorization: Bearer <token>` matching --token / KPM_TOKEN or a line in
//! `<dir>/tokens.txt`; with no token configured, publishing is disabled (downloads still work).
//!
//! Endpoints: see `killer_native::pkg_registry` (GET /health, /api/packages, /api/package/<name>,
//! /api/package/<name>/versions, /api/package/<name>/<version>[/manifest], /download/<name>/<version>;
//! POST /api/publish/<name>/<version>).

use killer_native::pkg_registry;
use std::path::PathBuf;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut dir = std::env::var("KPM_DIR").unwrap_or_else(|_| "registry".to_string());
    let mut port = std::env::var("KPM_PORT").unwrap_or_else(|_| "8080".to_string());
    let mut bind = std::env::var("KPM_BIND").unwrap_or_else(|_| "127.0.0.1".to_string());
    let mut token = std::env::var("KPM_TOKEN").ok();
    let mut i = 0;
    while i < args.len() {
        let val = args.get(i + 1).cloned();
        match (args[i].as_str(), val) {
            ("--dir", Some(v)) => dir = v,
            ("--port", Some(v)) => port = v,
            ("--bind", Some(v)) => bind = v,
            ("--token", Some(v)) => token = Some(v),
            ("--help", _) | ("-h", _) => {
                println!("usage: kpm-registry [--dir DIR] [--port 8080] [--bind 127.0.0.1] [--token SECRET]");
                return;
            }
            (other, _) => {
                eprintln!("unknown or incomplete option '{}'", other);
                std::process::exit(2);
            }
        }
        i += 2;
    }
    let root = PathBuf::from(&dir);
    if let Err(e) = std::fs::create_dir_all(&root) {
        eprintln!("cannot create registry directory {}: {}", dir, e);
        std::process::exit(1);
    }
    let tokens = pkg_registry::load_tokens(&root, token);
    let server = match pkg_registry::spawn_server(root.clone(), tokens.clone(), &format!("{}:{}", bind, port)) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("{}", e);
            std::process::exit(1);
        }
    };
    println!("kpm-registry serving {} on http://{}", root.display(), server.addr);
    if tokens.is_empty() {
        println!("no publish token configured: publishing is disabled");
    }
    // run until the process is killed
    loop {
        std::thread::sleep(std::time::Duration::from_secs(3600));
    }
}
