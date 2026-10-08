//! `killer_dap`: Debug Adapter Protocol server for Killer (stdio). See `killer_native::dap`.

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    std::process::exit(killer_native::dap::main_with_args(&args));
}
