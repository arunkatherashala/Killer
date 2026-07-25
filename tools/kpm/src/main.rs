use anyhow::Result;
use reqwest::blocking::Client;
use std::env;

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("kpm: usage: kpm install <package>");
        std::process::exit(1);
    }
    match args[1].as_str() {
        "install" => install(&args[2..])?,
        _ => { eprintln!("unknown command"); }
    }
    Ok(())
}

fn install(args: &[String]) -> Result<()> {
    if args.is_empty() {
        eprintln!("kpm install <package>");
        return Ok(());
    }
    let pkg = &args[0];
    println!("Installing package '{}' (stub)...", pkg);
    // TODO: connect to registry, verify quality metadata, download, and install
    let client = Client::new();
    let _ = client.get("https://registry.killer.local/health").send();
    println!("Done (stub). Next: implement registry, verify signatures, and install logic.");
    Ok(())
}
