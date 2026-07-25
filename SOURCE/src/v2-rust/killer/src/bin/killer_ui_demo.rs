//! Demo: **killer_ui** phases A–D scaffold (A+C+D code; B = stub window hook).
//!
//! ```text
//! cargo run --release --bin killer_ui_demo
//! ```

use killer_native::killer_ui::{runtime_native, KillerUiEngine};

fn main() -> Result<(), Box<dyn std::error::Error>> {
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
