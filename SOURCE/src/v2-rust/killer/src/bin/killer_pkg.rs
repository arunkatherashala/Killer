//! `killer_pkg` - the Killer package manager. All logic lives in `killer_native::pkg_cli`.

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    std::process::exit(killer_native::pkg_cli::run(&args));
}
