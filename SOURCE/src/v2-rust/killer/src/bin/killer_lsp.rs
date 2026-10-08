//! `killer_lsp`: the Killer language server (LSP over stdio). See `killer_native::lsp_core`.

fn main() {
    std::process::exit(killer_native::lsp_core::serve_stdio());
}
