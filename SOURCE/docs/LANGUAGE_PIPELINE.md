# Killer language: compilation pipelines

**Product story (one line):** embeddable bytecode VM (+ growing native/JIT path), clear compile pipelines, optional AI/tooling, and **first-class trits** for three-valued logic — see [`POSITIONING.md`](./POSITIONING.md).

## What you ship: `.killer` (super-language surface)

**Application logic is written in Killer and saved as `*.killer`.** Rust in this repo is the **implementation** (parser, bytecode, VM, `killer_super`, MCP, tests)—not a parallel public language for demos or product code. From now on, new features, tutorials, and “language ready” proof should land as **`.killer`** under `SOURCE/src/v2-rust/killer/examples/` (or your own tree), executed via `run_killer_source` / `compile_killer_default` / `killer_super` as below. See `examples/killer_language_ready.killer` and `examples/README.md` in that crate.

This document describes how Killer **text** becomes bytecode in **killer-native** (`SOURCE/src/v2-rust/killer`). It reflects the implementation as of the “unified documentation + API” update.

## Three ways from source to bytecode

### 0. Text → `ast::Stmt` → bytecode (new)

- **`parse_killer_program(source)`** — `preprocess_killer_source` → **`lex_with_newlines`** → recursive-descent parser → `Vec<Stmt>`.
- **`run_killer_parsed(source)`** — parse + **`compile_killer_ast`** + VM.
- **Covers (initial):** `let`, assignment, `print`, `if` / `else`, `while`, `for`/`in` and `for`/`of`, `return`, `break`/`continue`, `fn` / `kfn`, `async`+`fn`/`kfn`, `import`, `spawn`, `await` (as expression statement), expressions with calls/index/methods, arrays, dicts, ternary, binary ops. **Python-style builtins:** `sorted`, `sum`, `enumerate`, `all`, `any`, `zip`, `reversed`, `copy`, `get`, `setdefault` (see `examples/stdlib_pythonic.killer`). **MCP:** `killer_run` executes `.killer` in a subprocess and returns captured output.
- **Default line compiler `for`:** both `for x in arr` and `for x of arr` are accepted; both compile to index iteration over `len`/`IndexRead` (value iteration for arrays). For key iteration on dicts use the **AST** path (`for k in d` uses `iterKeys` there).
- **Arithmetic (default / line pipeline):** chained `+` / `-` and `*` / `/` / `%` use **left-associative** splitting at the same precedence (e.g. `a - b - c` → `(a - b) - c`; `a / b * c` → `(a / b) * c`). Exponentiation `**` splits at the **first** top-level `**`, which chains as `pow(a, pow(b, c))` (same as Python’s right-associative `**`).
- **Default line compiler (recent / high-signal):** non-empty `{ }` dict literals; `**` and `pow`-compatible multiply precedence (chained `**` is right-associative like Python); `bit_and` / `bit_or` / `bit_xor` / `bit_shl` / `bit_shr` builtins; general `expr[i]` indexing (not only a bare name); `name[i] = rhs`, `name["k"] = rhs`, and chained `name[i][j]` (and deeper) assignment with correct copy-on-read write-back; multi-line `kfn`/`fn` signatures (balanced `(`/`[` across lines); Python-style top-level `if`/`while`/`for` bodies via the offside preprocessor when no brace-style `fn`/`kfn` forces the fast path; single-line dict/array grouping in `normalize_lines` for `{`/`}` on one source line.
- **Explicitly rejected today (text → `Stmt` parser):** `try`/`catch`, `class` — `stmt_parser.rs` returns a clear error. The **default line compiler** does not parse `try`/`class` from text either; use **AST** construction or extend the parser / line compiler when you need those from source.
- **Not yet (no dedicated arm):** `switch`, `do`/`while`, `yield`, `quality`, and some edge forms — extend `stmt_parser.rs` or use the default line compiler.

### 1. Default (line-oriented) — `compile_killer_default` / `compile_killer_subset`

- **Entry points:** `run_killer_source`, VM package `import`, `supernova`, `killer-mcp` (`killer_compile` tool).
- **Stages:** Indentation / polyglot preprocess → normalized lines → **line-oriented** codegen → optional bytecode optimize.
- **Program shape:** `Program.classes` is **empty**. Class-like syntax on this path is not turned into the same `Program` layout as the AST pipeline.
- **Strengths:** What users run today; stable for scripts with `kfn`, control flow, builtins, K-strings, etc.
- **Limitations (line pipeline):** no `expr[i] = v` when `expr` is not a simple identifier (nested `a[i][j] = v` **is** supported when `a` is a simple name); leading unary mix on full expressions (e.g. `-5 + 3`) can differ from typical infix rules; multiline dict literals split across source lines are not merged in `normalize_lines` (use one line or AST).

### 2. Full AST (hand-built or parsed) — `compile_killer_ast` / `compile_statements`

- **Entry point:** `run_killer_ast(&[Stmt])` or `compile_killer_ast` when you have a `Vec<ast::Stmt>`.
- **Stages:** No text lexer here — you supply [`ast::Stmt`](../../src/v2-rust/killer/src/ast.rs) trees → bytecode with **functions**, **methods**, **`DefineClass`**, and `Program.classes` populated.
- **Strengths:** Matches the rich `Stmt` / `Expr` enum (classes, async/spawn/import nodes, switch, try/catch, …) as implemented in `compile_stmt`.
- **Text → `Stmt`:** Implemented in **`stmt_parser.rs`** (see §0). The older `parser.rs` module still uses a **different** AST (`AstNode` / type annotations), not `ast::Stmt`.

Related source: `ast.rs`, `stmt_parser.rs`, `lexer.rs` (`lex_with_newlines`), `compiler.rs`.

## CI “green” set

The default **killer-native** check (see `SOURCE/src/v2-rust/killer/QUICKSTART.md` and `.github/workflows/killer-native.yml`) runs:

- `cargo build`
- `cargo test --lib`
- `cargo test --test pipeline_conformance`
- `cargo test --test trit_three_valued`
- `cargo test --test ai_integration_tests --test ai_annotations_tests`
- `cargo test --test builtin_pythonic --test parser_tests`

Local parity: `scripts/revalidate_killer.ps1` (Windows) or `scripts/revalidate_killer.sh` (Unix); add `-Full` / `--full` for the full `cargo test` sweep.

Legacy integration tests that still import the separate **`killer_rcore`** crate are behind Cargo feature **`legacy-killer-rcore-tests`**. CLI subprocess tests use **`cli-e2e-tests`**.

## API summary

| Function | Pipeline |
|----------|-----------|
| `run_killer_source(&str)` | Default (text) |
| `compile_killer_default(&str)` | Default (text) |
| `compile_killer_subset(&str)` | Same as `compile_killer_default` (historical name) |
| `parse_killer_program(&str)` | Text → `Vec<Stmt>` |
| `run_killer_parsed(&str)` | Text → Stmt → AST compile → VM |
| `run_killer_ast(&[Stmt])` | AST |
| `compile_killer_ast(&[Stmt])` | AST |

## Recommendations

1. **New embedding / tools:** Prefer `compile_killer_default` in code for clarity; keep `compile_killer_subset` for backward compatibility.
2. **Classes from source:** Until a text→`Stmt` parser exists, use **AST** construction in Rust or extend the line compiler deliberately.
3. **Specs and tests:** Add conformance tests for **both** paths (`tests/pipeline_conformance.rs`).
