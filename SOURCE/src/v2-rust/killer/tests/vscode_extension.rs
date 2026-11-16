//! The VS Code extension files under editors/vscode must parse and keep the scopes the grammar promises.

use killer_native::json_lang;
use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("editors").join("vscode")
}

fn read(rel: &str) -> String {
    std::fs::read_to_string(root().join(rel)).unwrap_or_else(|e| panic!("{rel}: {e}"))
}

#[test]
fn every_json_file_parses() {
    for rel in ["package.json", "language-configuration.json", "syntaxes/killer.tmLanguage.json", "snippets/killer.json"] {
        let text = read(rel);
        if let Err(e) = json_lang::parse(&text) {
            panic!("{rel} is not valid JSON: {e}");
        }
    }
}

#[test]
fn package_json_wires_language_grammar_snippets_and_entry() {
    let pkg = read("package.json");
    for needle in [
        "\"id\": \"killer\"",
        "\".killer\"",
        "\"scopeName\": \"source.killer\"",
        "\"onLanguage:killer\"",
        "./syntaxes/killer.tmLanguage.json",
        "./snippets/killer.json",
        "./language-configuration.json",
        "./client/extension.js",
        "killer.server.path",
    ] {
        assert!(pkg.contains(needle), "package.json lacks {needle}");
    }
    for rel in ["syntaxes/killer.tmLanguage.json", "snippets/killer.json", "language-configuration.json", "client/extension.js"] {
        assert!(root().join(rel).exists(), "{rel} missing");
    }
}

#[test]
fn grammar_scopes_cover_comments_strings_keywords_numbers() {
    let g = read("syntaxes/killer.tmLanguage.json");
    for scope in [
        "comment.line.number-sign.killer",
        "comment.line.double-slash.killer",
        "comment.line.double-dash.killer",
        "string.quoted.double.killer",
        "string.quoted.single.killer",
        "constant.numeric.integer.killer",
        "constant.numeric.float.killer",
        "constant.numeric.hex.killer",
        "keyword.control.conditional.killer",
        "keyword.control.loop.killer",
        "storage.type.killer",
        "entity.name.function.killer",
        "entity.name.type.class.killer",
        "support.function.builtin.killer",
        "constant.language.killer",
    ] {
        assert!(g.contains(&format!("\"{scope}\"")), "grammar lacks scope {scope}");
    }
    for kw in ["fn", "class", "if", "else", "while", "for", "return", "import", "try", "catch", "match", "async", "await"] {
        assert!(g.contains(&format!("|{kw}|")) || g.contains(&format!("({kw}|")) || g.contains(&format!("|{kw})")) || g.contains(&format!("({kw})")),"grammar lacks keyword {kw}");
    }
    // the comment markers themselves
    assert!(g.contains("\"match\": \"#.*$\""));
    assert!(g.contains("\"match\": \"//.*$\""));
    assert!(g.contains("--(?!-).*$"));
}

#[test]
fn extension_launches_the_language_server_and_snippets_have_prefixes() {
    let js = read("client/extension.js");
    assert!(js.contains("killer_lsp") && js.contains("spawn") && js.contains("Content-Length"), "extension must spawn the server and frame LSP");
    let snippets = read("snippets/killer.json");
    assert!(snippets.contains("\"prefix\"") && snippets.contains("\"body\""));
    let cfg = read("language-configuration.json");
    assert!(cfg.contains("brackets") && cfg.contains("lineComment"));
}
