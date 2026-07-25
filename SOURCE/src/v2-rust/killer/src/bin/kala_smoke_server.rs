//! Start the Kala web UI for local smoke tests (`KALA_SMOKE.md`).
//! Native `killer_super` codegen does not yet emit `kala_serve(...)`; this binary calls the builtin directly.
//!
//! Usage: `cargo run --bin kala_smoke_server`  (default port 8080)
//!        `cargo run --bin kala_smoke_server -- 8090`

use killer_native::kala_ui::builtin_kala_serve;
use killer_native::value::Value;

fn main() {
    let port: u16 = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(8080);
    if let Err(e) = builtin_kala_serve(&[Value::Number(f64::from(port))]) {
        eprintln!("{}", e);
        std::process::exit(1);
    }
}
