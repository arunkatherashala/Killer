//! **Tier 5 — `killer_ui` CLI:** serve HTTP, print scaffold, or invoke native window hook (Tier 2 is stub until optional `eframe` is added — see `KILLER_UI_ENGINE.md`).
//!
//! ```text
//! cargo run --bin killer_ui -- serve 8787
//! cargo run --bin killer_ui -- demo
//! cargo run --bin killer_ui -- window
//! ```

use killer_native::killer_ui::{runtime_native, KillerUiEngine};

fn main() {
    let mut it = std::env::args().skip(1);
    let cmd = it.next().unwrap_or_else(|| "help".to_string());
    let rest: Vec<String> = it.collect();

    match cmd.as_str() {
        "serve" => {
            let (host, port) = match rest.len() {
                0 => ("127.0.0.1".to_string(), 8787u16),
                1 => {
                    let port: u16 = rest[0].parse().unwrap_or_else(|_| {
                        eprintln!("serve: PORT must be a number (e.g. 8787)");
                        std::process::exit(2);
                    });
                    ("127.0.0.1".to_string(), port)
                }
                _ => {
                    let port: u16 = rest[1].parse().unwrap_or_else(|_| {
                        eprintln!("serve: PORT must be a number");
                        std::process::exit(2);
                    });
                    (rest[0].clone(), port)
                }
            };
            if let Err(e) = killer_native::killer_ui::run_headless_panel_server(&host, port) {
                eprintln!("{}", e);
                std::process::exit(1);
            }
        }
        "demo" => {
            if let Err(e) = run_demo_text() {
                eprintln!("{}", e);
                std::process::exit(1);
            }
        }
        "window" => {
            if let Err(e) = run_window() {
                eprintln!("{}", e);
                std::process::exit(1);
            }
        }
        "help" | "--help" | "-h" => print_help(),
        _ => {
            eprintln!("unknown command: {}\n", cmd);
            print_help();
            std::process::exit(2);
        }
    }
}

fn print_help() {
    eprintln!(
        r#"killer_ui — control plane (Tiers 2–5)

  serve [HOST] PORT | serve [PORT]
                   HTTP JSON panel. One arg = port on 127.0.0.1; two args = bind host + port (default 8787).
  demo             Print parallel scaffold (headless floats + cook line).
  window           Invoke native window path (stderr stub until optional eframe is wired).

Examples:
  cargo run --bin killer_ui -- serve 8787
  cargo run --bin killer_ui -- serve 0.0.0.0 8787
  cargo run --bin killer_ui -- demo
  cargo run --bin killer_ui -- window
"#
    );
}

fn run_demo_text() -> Result<(), Box<dyn std::error::Error>> {
    let engine = KillerUiEngine::example_parallel();
    let summary = runtime_native::cook_summary(&engine.graph);
    println!("killer_ui parallel scaffold");
    println!("  cluster: {}", engine.workspace.cluster_id);
    println!("  panels: {}", engine.workspace.panels.len());
    println!("  cook: {}", summary);
    let frame = engine.tick_headless();
    println!("  headless frame floats: {:?}", frame.cooked_floats);
    let line = format!(
        "{} | workspace panels: {}",
        summary,
        engine.workspace.panels.len()
    );
    runtime_native::run_demo_window(line)?;
    Ok(())
}

fn run_window() -> Result<(), Box<dyn std::error::Error>> {
    let engine = KillerUiEngine::example_parallel();
    let summary = runtime_native::cook_summary(&engine.graph);
    let line = format!(
        "{} | workspace panels: {}",
        summary,
        engine.workspace.panels.len()
    );
    runtime_native::run_demo_window(line)?;
    Ok(())
}
