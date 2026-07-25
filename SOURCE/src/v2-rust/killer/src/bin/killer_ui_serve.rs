//! **Tier 3 (web lane):** minimal HTTP surface for `killer_ui` headless JSON — no browser engine in-process.
//!
//! ```text
//! cargo run --bin killer_ui_serve -- 8787
//! cargo run --bin killer_ui_serve -- 0.0.0.0 8787
//! curl -s http://127.0.0.1:8787/health
//! ```

fn main() -> Result<(), String> {
    let mut args = std::env::args().skip(1).collect::<Vec<_>>();
    let (host, port): (String, u16) = match args.len() {
        0 => ("127.0.0.1".to_string(), 8787),
        1 => {
            let port: u16 = args[0]
                .parse()
                .map_err(|_| "usage: killer_ui_serve [HOST] PORT  (one arg = port on 127.0.0.1)".to_string())?;
            ("127.0.0.1".to_string(), port)
        }
        _ => {
            let host = std::mem::take(&mut args[0]);
            let port: u16 = args[1]
                .parse()
                .map_err(|_| "usage: killer_ui_serve [HOST] PORT".to_string())?;
            (host, port)
        }
    };

    killer_native::killer_ui::run_headless_panel_server(&host, port)
}
