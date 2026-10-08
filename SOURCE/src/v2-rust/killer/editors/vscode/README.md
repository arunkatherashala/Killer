# Killer for VS Code

Support for the Killer scripting language (`.killer` files).

* Syntax highlighting: keywords, strings with escapes and `k"..{x}.."` interpolation,
  `#`, `//` and `--` comments, numbers, operators, builtin names.
* Snippets for functions, classes, loops, `try/catch`, `match`, and `test "name" { }` blocks.
* Language configuration: brackets, auto-closing pairs, comment toggling, indentation rules.
* Language server (`killer_lsp`, built from the `killer-native` crate): diagnostics from the real
  compiler, completion, hover, go-to-definition and the outline.

## Install

1. Build the server: `cargo build --release --bin killer_lsp` in the crate root
   (`SOURCE/src/v2-rust/killer`).
2. Put `target/release/killer_lsp` on your `PATH`, or set `killer.server.path`.
3. Copy or symlink this folder into your extensions directory
   (`~/.vscode/extensions/killer-lang`) and reload VS Code, or run `npx vsce package` here to
   build a `.vsix`.

The extension has no npm dependencies; `client/extension.js` contains a small JSON-RPC client that
talks to the server over stdio.

## Settings

| Setting | Default | Meaning |
|---------|---------|---------|
| `killer.server.path` | `killer_lsp` | Path to the language server binary |
| `killer.server.args` | `[]` | Extra arguments |

## Validation

`python test/validate.py` loads every JSON file with `json.load`, compiles the grammar regexes and
checks that the grammar covers the language keywords and all three comment styles.
