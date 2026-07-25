# Fuzzing the Killer lexer / parser

## Goal

Catch panics on arbitrary bytes/UTF-8 for `lexer::lex_with_newlines` and `stmt_parser::parse_killer_program`.

## Setup (cargo-fuzz)

```bash
cd SOURCE/src/v2-rust/killer
cargo install cargo-fuzz
cargo fuzz init   # once, if fuzz/ does not exist
```

Add a target `fuzz_targets/lexer_parser.rs` that:

1. Takes `data: &[u8]`.
2. Converts lossy to `String` (or rejects invalid UTF-8).
3. Calls `lex_with_newlines` then `Parser::parse_program` inside `catch_unwind` or returns `Ok(())` on `Err`.

Run:

```bash
cargo fuzz run lexer_parser -- -runs=100000
```

## CI

Keep fuzz as **manual** or **nightly** job until deterministic seeds are checked in — full runs are slow.
