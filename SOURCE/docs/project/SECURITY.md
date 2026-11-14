# Killer Security Features & Guidelines

## Current Security Implementation

### 1. **Safe Installation**
- ✅ No admin privileges required for user installation
- ✅ Installation to user directory only (no system-wide access)
- ✅ File associations configured safely
- ✅ PATH environment variable isolation

### 2. **Script Execution Safety**
- ✅ Scripts run in isolated Python interpreter context
- ✅ Standard I/O isolation per execution
- ✅ Error handling and exception catching
- ✅ Stack trace prevention in release mode

### 3. **File System Protection**
- ✅ Relative path resolution prevents directory traversal
- ✅ File access limited to user's accessible directories
- ✅ No automatic file deletion or modification
- ✅ Explicit file handling in scripts only

## Planned Security Features (v3.1+)

### Permission System
```killer
// Explicit permission declarations
@permission("file:read")
@permission("file:write")
@permission("network:http")
fn main() {
    // Code that requires these permissions
}
```

### Sandboxed Execution Mode
```bash
killer --sandbox script.killer    # Run in restricted environment
killer --strict script.killer      # No external calls allowed
```

### Code Signing & Verification
```bash
killer --sign script.killer        # Sign a script
killer --verify script.killer      # Verify signature before execution
```

### Resource Limits
- Memory limits per script
- Execution time limits
- File size restrictions
- Network bandwidth restrictions

## Best Practices for Users

### ✅ DO:
- ✅ Review scripts before executing untrusted code
- ✅ Use `--sandbox` mode for untrusted scripts
- ✅ Keep Killer updated for security patches
- ✅ Report security vulnerabilities responsibly

### ❌ DON'T:
- ❌ Run `.killer` files from untrusted sources without review
- ❌ Give `.killer` files admin/elevated privileges
- ❌ Store sensitive data in script files
- ❌ Share sensitive scripts unencrypted

## Security Policies

### Error Handling
- Errors display minimal information (no system paths in release build)
- Stack traces hidden by default
- Permission denied messages clear without details

### Network Security (Future)
- No network access by default
- Explicit permission declarations required
- HTTPS enforced for network requests
- No certificate validation bypass

### File Security (Future)
- No access to system directories
- User directory access only
- Explicit file permissions required
- Audit trail for file operations

## Vulnerability Reporting

If you discover a security vulnerability in Killer:

1. **Do NOT** open a public GitHub issue
2. **Email**: security@killerlang.dev (when available)
3. **Include**: 
   - Description of vulnerability
   - Steps to reproduce
   - Potential impact
   - Suggested fix (if available)

We will:
- Acknowledge within 48 hours
- Investigate and develop fix
- Release security patch
- Credit discoverer (if desired)

## Security Version History

| Version | Security Features |
|---------|------------------|
| v3.0 | Safe installation, script isolation |
| v3.1 | Permission system, sandboxed execution |
| v3.2 | Code signing, resource limits |
| v4.0 | Full security framework |

## Questions?

For security concerns or to report issues:
- GitHub Issues: (Non-security bugs only)
- Documentation: See SECURITY.md in repository

---

**Remember**: Killer prioritizes transparency and security. All source code is visible for audit. We believe in "security through openness" - anyone can review the code!


## Trust model for external helper programs and `eval`

Killer has no TLS or HTTP stack of its own. TLS (`tcp_connect_tls`, `ws_connect("wss://...")`, `tls_listen`, `serve`
with TLS) runs the `openssl` command-line tool, HTTP (`http_*`, LLM and vision calls) runs `curl`, and on Windows the
TLS server also uses `powershell.exe`.

**How the program is found.** It is resolved once to an absolute path and never started by bare name.
- `KILLER_OPENSSL` / `KILLER_CURL`, when set, must be an absolute path to an existing file. A relative value is an error.
- Otherwise the directories of PATH are searched in order. Empty entries, relative entries (`.`, `bin`, `../x`) and the
  current directory (even when written as an absolute entry) are skipped. On Windows only `name.exe` is accepted.
- PowerShell is taken from `%SystemRoot%/System32/WindowsPowerShell/v1.0` first.
- PATH and the two variables are read from a snapshot taken on first use, or just before the script's first `env_set`.
  A script that edits PATH and then calls a TLS builtin therefore cannot redirect the helper.
- A missing tool gives an error that names the tool, says where it looked and how to set the override.

**What is trusted.** The operating system, the directories on the startup PATH (an attacker who can write there already
owns the user), the program named by `KILLER_OPENSSL` / `KILLER_CURL` (set by whoever starts Killer, not by scripts),
and the certificate verification done by `openssl s_client` and `curl`. Killer passes the host name to `-verify_hostname`
and never turns verification off for `tcp_connect_tls`; `http_request` verifies unless `insecure` is set.

**What is not trusted.** The current directory, relative PATH entries, PATH changes made by a script, and the content
a peer sends (it is handled as data and never executed). The helper's stderr text is shown to the script as an error
message only.

**Secrets.** HTTP requests, LLM calls and vision calls give curl their URL, headers (API keys) and body as a config on
standard input, so they do not appear in the process list. The temporary PKCS#12 password used by the Windows TLS
server is generated per connection and passed through an environment variable of the child, not argv. Certificate and
key file paths are on the command line (they are not secret; the files are).

**Capabilities.** TLS and HTTP builtins need the network capability. Scripts can only change the environment with
`env_set`, which needs the process capability. The helper is started by the interpreter, not by the script, so a
network-only sandbox can still run `openssl` and `curl` (and nothing else) through these builtins. A script that can
write files and has network access cannot plant a helper anywhere the lookup searches, unless the startup PATH
already contains a directory that script can write to.

**Known remaining weak spots.** `image_gen` and the web-search fetchers in `llm.rs` call curl with `-k` (certificate
errors ignored) for proxy compatibility; do not send secrets through them.

**`eval` / `exec`.** The text runs with all the caller's capabilities (a fresh VM is given the caller's set, so a
restricted script cannot use `eval` to escape), cannot see or change the caller's variables beyond the `vars` dict,
is limited to 8 nested levels, 50 million instructions, 30 seconds and a memory cap, and its errors carry no host
paths. It is still arbitrary code: never evaluate untrusted text.

*Memory cap.* Each evaluation is capped at 256 MB by default (`eval(code, vars, {"max_memory_mb": N})` or the
environment variable `KILLER_EVAL_MAX_MEMORY`; 1 to 2048 MB, anything else is an error). The Killer CLI binaries install a
thin counting wrapper around the system allocator (`killer_native::memcap::CountingAlloc`) that, only while an evaluation runs, keeps a per-thread count of live
bytes; the VM compares it with the limit every 256 instructions and the big allocation sites (string and list repeat
and concatenation, `range`, padding) check before allocating. Exceeding it ends the evaluation with a catchable error
(`memory limit exceeded (N MB)`) that code inside the evaluation cannot swallow, and the evaluation's VM is dropped,
which frees its memory. A nested evaluation is bounded by what is left of its parent's cap. Honest limits: the count
is requested bytes, not resident memory (no allocator overhead or fragmentation); an allocation that is not at a
checked site is only noticed at the next periodic check, and a growing list or string can briefly hold its old and new
buffer, so the real peak can be up to about twice the cap; threads or processes the evaluated code starts are not
counted; builtins that allocate outside the VM (file reads, network bodies, native libraries) are only caught by the
periodic check afterwards; the caller's own memory is not counted. The cap protects against accidents and sloppy
generated code, not against a determined attacker, who should be kept out by not evaluating untrusted text at all.

*Embedders.* The library does not install an allocator itself, so it cannot conflict with an embedder's own
`#[global_allocator]`. The live-bytes check is on in the Killer CLI binaries. An embedder that wants it either
declares `killer_native::memcap::CountingAlloc` as its global allocator or enables the optional `mem-cap` Cargo
feature (the library then installs it; the CLI binaries skip their own declaration in that case). Otherwise the
live-memory check does not exist: there is no periodic live-bytes check at all, and only the explicit pre-checks
(string and list repeat and concatenation, `range`, padding), the step budget, the time limit and the capability
inheritance apply; nothing panics and no false "limit exceeded" is reported. `memcap::is_installed()` reports which
case applies. `tests/eval_memory_embedded.rs` covers the no-allocator case.
