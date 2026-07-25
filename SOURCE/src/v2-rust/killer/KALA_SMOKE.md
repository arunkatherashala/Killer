# Kala — evaluation smoke tests

Use this for quick regression checks after UI or API changes. It does **not** score answer quality; it proves the **server, HTML shell, and JSON API** work end to end.

## 1. Automated (HTTP)

**Terminal A** — from this crate directory (`SOURCE/src/v2-rust/killer`):

```powershell
cargo run --bin kala_smoke_server
```

Optional port: `cargo run --bin kala_smoke_server -- 8090` — then set `KALA_PORT=8090` for the scripts below.

**Why not `killer_super` + `.killer`?** The `kala_serve(port)` builtin is not yet supported by the `killer_super` native codegen path; `kala_smoke_server` calls the same runtime builtin the VM uses.

**Terminal B**:

- **Windows (PowerShell):**

  ```powershell
  $env:KALA_PORT = '8080'   # optional; default is 8080
  .\scripts\kala-smoke.ps1
  ```

- **macOS / Linux:**

  ```bash
  export KALA_PORT=8080   # optional
  chmod +x scripts/kala-smoke.sh
  ./scripts/kala-smoke.sh
  ```

If the server uses another port (e.g. repo `tests/kala_serve.killer` uses **8088**), set `KALA_PORT` to match before running the script.

## 2. Manual checklist (browser)

With the same server running, open `http://127.0.0.1:8080/` (or your port).

| Step | Check |
|------|--------|
| Load | Page title and sidebar modes render. |
| Chat | Send a short question; assistant bubble appears; no console JSON errors. |
| New chat | Clears thread (or confirm dialog then clears). |
| Voice | **Voice** opens studio; mic permission; one listen → reply → speak cycle (Chrome/Edge). |
| Failure | Stop the server; send a message — error bubble and **Retry question** appear; restore server and retry succeeds. |

## 3. Rust tests (no browser)

From this directory:

```powershell
cargo test --test knowledge_base_tests
cargo test --test code_generation_tests
```

## 4. Growing past “smoke”

To move toward a numeric score (e.g. accuracy on your tasks), keep a **fixed list of prompts** (facts, code, safety) and record pass/fail or rubric grades per release. The scripts above stay your **gate**; your prompt list becomes the **scorecard**.
