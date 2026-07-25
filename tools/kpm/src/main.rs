mod registry;

use anyhow::Result;
use std::env;

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("kpm: usage: kpm [install|publish|resolve] <args>");
        std::process::exit(1);
    }
    match args[1].as_str() {
        "install" => {
            let pkg = args.get(2).map(|s| s.as_str()).unwrap_or("");
            let mut dest = "./kpm_packages".to_string();
            if let Some(idx) = args.iter().position(|x| x == "--dest") {
                if let Some(d) = args.get(idx + 1) { dest = d.clone(); }
            }
            if pkg.is_empty() { eprintln!("package required"); return Ok(()); }
            registry::install_from_local(pkg, &dest)?;
        }
        "publish" => {
            let manifest = args.get(2).map(|s| s.as_str()).unwrap_or("");
            let out_dir = args.get(3).map(|s| s.as_str()).unwrap_or(".");
            if manifest.is_empty() { eprintln!("manifest required"); return Ok(()); }
            let (path, sha) = registry::publish_package(manifest, out_dir)?;
            println!("  path: {}", path);
            println!("  sha256: {}", sha);
        }
        "resolve" => {
            let pkg = args.get(2).map(|s| s.as_str()).unwrap_or("");
            let ver = args.get(3).map(|s| s.as_str()).unwrap_or("");
            if pkg.is_empty() || ver.is_empty() { eprintln!("package version required"); return Ok(()); }
            // Mock available packages for demo
            let available = vec![
                ("killer-stdlib".to_string(), "1.0.0".to_string()),
                ("killer-io".to_string(), "0.2.0".to_string()),
            ];
            match registry::resolve_deps(pkg, ver, &available) {
                Ok(resolved) => println!("resolved: {}@{}", pkg, resolved),
                Err(e) => eprintln!("resolve failed: {}", e),
            }
        }
        _ => { eprintln!("unknown command"); }
    }
    Ok(())
}

