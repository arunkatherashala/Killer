mod registry;

use anyhow::Result;
use std::env;

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("kpm: usage: kpm install <package> [--dest <dir>]");
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
        _ => { eprintln!("unknown command"); }
    }
    Ok(())
}

