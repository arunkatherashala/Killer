# LLM provider tests

`tests/llm_live.rs` has two groups of tests for the LLM code in `src/llm.rs` and `src/llm_stream.rs`.

## Offline tests (always run)

Tests named `offline_*` talk to a fake provider server that the test file starts on `127.0.0.1`. It speaks
the OpenAI-compatible, Anthropic and Ollama request and response shapes (JSON, SSE and NDJSON streaming,
chunked replies, 401 and 404 errors) and records each request. They check the request path, headers
(`Authorization: Bearer`, `x-api-key`, `anthropic-version`), the JSON body, response parsing (escapes, token
counts), streaming deltas, error replies, and that a key never appears in an error message. They need
`curl` (the HTTP client the library uses) but no network and no key:

```
cargo test --test llm_live
```

## Live tests (opt in)

Tests named `live_*` call the real providers. Each one runs only when its provider is configured, and
otherwise prints `skipped: <reason>` and passes. Add `-- --nocapture` to see the lines.

| Provider | Enabled by | Optional settings |
|---|---|---|
| OpenAI-compatible | `OPENAI_API_KEY` | `OPENAI_BASE_URL` (another compatible server), `OPENAI_TEST_MODEL` (default `gpt-4o-mini`), `OPENAI_EMBED_MODEL` (default `text-embedding-3-small`) |
| Anthropic | `ANTHROPIC_API_KEY` | `ANTHROPIC_TEST_MODEL` (default `claude-3-5-haiku-latest`) |
| Ollama and its embeddings | Ollama answering at `OLLAMA_HOST` (default `127.0.0.1:11434`) | `OLLAMA_TEST_MODEL` (default: first installed non-embedding model), `OLLAMA_EMBED_MODEL` (default `nomic-embed-text`, skipped if not installed) |

Each provider has a simple completion, streaming, and an error-path test (a bogus key, or a model that does
not exist, must give a clear error and not a panic). OpenAI and Ollama also have an embeddings test.
Anthropic has no embeddings endpoint, so it has none.

Commands (bash):

```
export OPENAI_API_KEY=...          # set it in your shell, never on the command line
export ANTHROPIC_API_KEY=...
cargo test --test llm_live live_ -- --nocapture --test-threads=1
cargo test --test llm_live live_openai -- --nocapture      # one provider
```

PowerShell: `$env:OPENAI_API_KEY = Read-Host` (or set it in your profile), then the same `cargo test` lines.

Local Ollama (no key, no cost):

```
ollama pull llama3.2 && ollama pull nomic-embed-text
cargo test --test llm_live live_ollama -- --nocapture
```

## Keys

Keys are read from the environment by the library (`LlmConfig::openai("", model)` with an empty key reads
`OPENAI_API_KEY` itself). The tests never copy a key into a variable, a command-line argument or an output
line, and the HTTP client sends secrets to curl on stdin, not in its arguments. The error-path tests send a
made-up key, and every live test checks that the real key is not in the text it prints.

## Cost

A full live run sends about ten short requests (about 16 output tokens each, one-word answers,
two to four embedding calls). With the default small models that is a fraction of a cent per run. Set
`OPENAI_TEST_MODEL` / `ANTHROPIC_TEST_MODEL` to change the models, and expect charges if you pick large ones.

## Status

The offline group passes. The live group has not been run against a real provider yet (no key was
available when it was written); see `tracker.csv` entry 175.
