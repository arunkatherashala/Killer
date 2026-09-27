/// Killer Language Web Playground
///
/// HTTP server that executes Killer code in a sandboxed VM and returns output.
/// This powers the killer-lang.org/play online playground.
///
/// API:
///   POST /run          — run Killer code, return output/error as JSON
///   GET  /             — serve the playground HTML page
///   GET  /health       — health check
///
/// Usage:  killer-playground [port]   (default: 3000)
/// Env:    PLAYGROUND_PORT=3000
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::time::{Duration, Instant};

fn main() {
    let port: u16 = std::env::args().nth(1)
        .and_then(|p| p.parse().ok())
        .or_else(|| std::env::var("PLAYGROUND_PORT").ok().and_then(|p| p.parse().ok()))
        .unwrap_or(3000);

    let addr = format!("127.0.0.1:{}", port);
    let listener = TcpListener::bind(&addr)
        .unwrap_or_else(|e| { eprintln!("Cannot bind {}: {}", addr, e); std::process::exit(1); });

    println!("Killer Playground running at http://{}", addr);
    println!("  POST /run    — execute Killer code");
    println!("  GET  /       — playground UI");
    println!("Press Ctrl+C to stop.");

    for stream in listener.incoming().flatten() {
        std::thread::spawn(|| handle(stream));
    }
}

fn handle(mut stream: TcpStream) {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
    let mut buf = [0u8; 65536];
    let n = match stream.read(&mut buf) { Ok(n) => n, Err(_) => return };
    let req = String::from_utf8_lossy(&buf[..n]);

    let first_line = req.lines().next().unwrap_or("");
    let parts: Vec<&str> = first_line.split_whitespace().collect();
    if parts.len() < 2 { return; }
    let (method, path) = (parts[0], parts[1]);

    match (method, path) {
        ("GET", "/") | ("GET", "/index.html") => {
            respond(&mut stream, 200, PLAYGROUND_HTML, "text/html; charset=utf-8");
        }
        ("GET", "/health") => {
            respond(&mut stream, 200, r#"{"status":"ok","runtime":"killer-v2.1"}"#, "application/json");
        }
        ("POST", "/run") => {
            let body = extract_body(&req);
            let code = extract_json_str(&body, "code").unwrap_or_default();
            let (output, error, elapsed_ms) = run_code(&code);
            let resp = format!(
                r#"{{"output":{},"error":{},"elapsed_ms":{}}}"#,
                json_str(&output), json_str(&error), elapsed_ms
            );
            respond(&mut stream, 200, &resp, "application/json");
        }
        ("OPTIONS", _) => {
            // CORS preflight
            let r = "HTTP/1.1 204 No Content\r\nAccess-Control-Allow-Origin: *\r\nAccess-Control-Allow-Methods: POST, GET, OPTIONS\r\nAccess-Control-Allow-Headers: Content-Type\r\nContent-Length: 0\r\n\r\n";
            let _ = stream.write_all(r.as_bytes());
        }
        _ => respond(&mut stream, 404, r#"{"error":"not found"}"#, "application/json"),
    }
}

fn run_code(code: &str) -> (String, String, u64) {
    use std::process::Command;

    let start = Instant::now();

    // Write code to a temp file
    let tmp_path = std::env::temp_dir().join(format!("killer_play_{}.killer", std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH).unwrap_or_default().subsec_nanos()));
    if std::fs::write(&tmp_path, code).is_err() {
        return (String::new(), "Failed to write temp file".to_string(), 0);
    }

    // Find the killer_super binary next to our own executable
    let killer_bin = std::env::current_exe().ok()
        .and_then(|p| p.parent().map(|d| d.join("killer_super.exe")))
        .filter(|p| p.exists())
        .or_else(|| std::env::current_exe().ok()
            .and_then(|p| p.parent().map(|d| d.join("killer_super")))
            .filter(|p| p.exists()))
        .unwrap_or_else(|| std::path::PathBuf::from("killer_super"));

    let result = Command::new(&killer_bin)
        .arg("--run")
        .arg(&tmp_path)
        .output();

    let _ = std::fs::remove_file(&tmp_path);
    let elapsed = start.elapsed().as_millis() as u64;

    match result {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
            let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
            if out.status.success() {
                (stdout, String::new(), elapsed)
            } else if !stderr.is_empty() {
                (stdout, stderr, elapsed)
            } else {
                (String::new(), format!("Process exited with code {:?}", out.status.code()), elapsed)
            }
        }
        Err(e) => (String::new(), format!("Could not run killer_super: {}", e), elapsed),
    }
}

fn extract_body(req: &str) -> String {
    if let Some(idx) = req.find("\r\n\r\n") { req[idx + 4..].to_string() }
    else if let Some(idx) = req.find("\n\n") { req[idx + 2..].to_string() }
    else { String::new() }
}

fn extract_json_str(json: &str, key: &str) -> Option<String> {
    let search = format!("\"{}\"", key);
    let pos = json.find(&search)?;
    let after = json[pos + search.len()..].trim_start().trim_start_matches(':').trim_start();
    if after.starts_with('"') {
        let content = &after[1..];
        let mut result = String::new();
        let mut escaped = false;
        for ch in content.chars() {
            if escaped {
                match ch { 'n' => result.push('\n'), 't' => result.push('\t'), _ => result.push(ch) }
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                break;
            } else {
                result.push(ch);
            }
        }
        Some(result)
    } else { None }
}

fn json_str(s: &str) -> String {
    format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\"")
        .replace('\n', "\\n").replace('\r', "\\r").replace('\t', "\\t"))
}

fn respond(stream: &mut TcpStream, status: u16, body: &str, ct: &str) {
    let status_text = match status { 200 => "OK", 404 => "Not Found", _ => "Error" };
    let resp = format!(
        "HTTP/1.1 {} {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nAccess-Control-Allow-Origin: *\r\nConnection: close\r\n\r\n{}",
        status, status_text, ct, body.len(), body
    );
    let _ = stream.write_all(resp.as_bytes());
}

// ── Embedded playground HTML ──────────────────────────────────────────────

const PLAYGROUND_HTML: &str = r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>Killer Language Playground</title>
<style>
  * { box-sizing: border-box; margin: 0; padding: 0; }
  body { font-family: 'Segoe UI', system-ui, sans-serif; background: #0d1117; color: #e6edf3; height: 100vh; display: flex; flex-direction: column; }
  header { background: #161b22; border-bottom: 1px solid #30363d; padding: 12px 24px; display: flex; align-items: center; gap: 16px; }
  header h1 { font-size: 1.2rem; font-weight: 700; color: #58a6ff; }
  header span { color: #8b949e; font-size: 0.85rem; }
  .badge { background: #1f6feb; color: #fff; padding: 2px 8px; border-radius: 12px; font-size: 0.75rem; font-weight: 600; }
  main { display: flex; flex: 1; overflow: hidden; }
  .editor-pane { flex: 1; display: flex; flex-direction: column; border-right: 1px solid #30363d; }
  .output-pane { width: 420px; display: flex; flex-direction: column; }
  .pane-header { background: #161b22; padding: 8px 16px; font-size: 0.8rem; color: #8b949e; border-bottom: 1px solid #30363d; display: flex; justify-content: space-between; align-items: center; }
  textarea { flex: 1; background: #0d1117; color: #e6edf3; border: none; padding: 16px; font-family: 'Consolas', 'Monaco', monospace; font-size: 14px; line-height: 1.6; resize: none; outline: none; }
  #output { flex: 1; padding: 16px; font-family: 'Consolas', monospace; font-size: 13px; line-height: 1.6; overflow-y: auto; white-space: pre-wrap; }
  .output-ok { color: #3fb950; }
  .output-err { color: #f85149; }
  .output-info { color: #8b949e; }
  .toolbar { background: #161b22; padding: 8px 16px; border-top: 1px solid #30363d; display: flex; gap: 8px; }
  button { padding: 6px 16px; border-radius: 6px; border: none; cursor: pointer; font-size: 0.85rem; font-weight: 600; }
  #runBtn { background: #238636; color: #fff; }
  #runBtn:hover { background: #2ea043; }
  #clearBtn { background: #21262d; color: #8b949e; border: 1px solid #30363d; }
  #clearBtn:hover { background: #30363d; color: #e6edf3; }
  select { background: #21262d; color: #8b949e; border: 1px solid #30363d; border-radius: 6px; padding: 6px 10px; font-size: 0.8rem; cursor: pointer; }
  .elapsed { font-size: 0.75rem; color: #8b949e; }
</style>
</head>
<body>
<header>
  <h1>⚡ Killer</h1>
  <span>Language Playground</span>
  <span class="badge">v2.1</span>
</header>
<main>
  <div class="editor-pane">
    <div class="pane-header">
      <span>Code</span>
      <select id="examples" onchange="loadExample()">
        <option value="">— Examples —</option>
        <option value="hello">Hello World</option>
        <option value="fib">Fibonacci</option>
        <option value="arr">Arrays &amp; Loops</option>
        <option value="dict">Dicts</option>
        <option value="trit">Ternary (Trit)</option>
        <option value="uncertain">Uncertain Values</option>
        <option value="async">Async</option>
        <option value="json">JSON</option>
      </select>
    </div>
    <textarea id="code" spellcheck="false">println("Hello from Killer!")
x = 2 + 3
println("2 + 3 = " + str(x))
</textarea>
    <div class="toolbar">
      <button id="runBtn" onclick="runCode()">▶ Run</button>
      <button id="clearBtn" onclick="clearOutput()">Clear</button>
    </div>
  </div>
  <div class="output-pane">
    <div class="pane-header">
      <span>Output</span>
      <span class="elapsed" id="elapsed"></span>
    </div>
    <div id="output"><span class="output-info">Click ▶ Run to execute your code.</span></div>
  </div>
</main>
<script>
const EXAMPLES = {
  hello: `println("Hello from Killer!")
name = "World"
println("Hello, " + name + "!")`,
  fib: `fn fib(n) {
  if n <= 1 { n }
  else { fib(n-1) + fib(n-2) }
}
for i in range(10) {
  println("fib(" + str(i) + ") = " + str(fib(i)))
}`,
  arr: `arr = [10, 20, 30, 40, 50]
total = 0
for x in arr { total = total + x }
println("Sum: " + str(total))
doubled = [x * 2 for x in arr]
println("Doubled: " + str(doubled))`,
  dict: `person = {name: "Sai", age: 25, city: "Hyderabad"}
println(person["name"] + " is " + str(person["age"]))`,
  trit: `a = T_POS
b = T_NEG
c = T_ZERO
println("Pos: " + str(a))
println("Neg: " + str(b))
println("Zero: " + str(c))`,
  uncertain: `x = believe 72 ± 3
y = believe 50 ± 5
println("x = " + str(x))
println("y = " + str(y))`,
  async: `fn worker(id) {
  println("Worker " + str(id) + " done")
  id * 10
}
result = async_await(async_spawn(fn() { worker(1) }))
println("Result: " + str(result))`,
  json: `data = json_parse('{"name":"Killer","version":"2.1","score":10}')
println("Language: " + data["name"])
println("Version: " + str(data["version"]))
encoded = json_stringify(data)
println("JSON: " + encoded)`
};

function loadExample() {
  const key = document.getElementById('examples').value;
  if (key && EXAMPLES[key]) {
    document.getElementById('code').value = EXAMPLES[key];
  }
}

async function runCode() {
  const code = document.getElementById('code').value;
  const btn = document.getElementById('runBtn');
  const out = document.getElementById('output');
  const elapsed = document.getElementById('elapsed');
  btn.textContent = '⏳ Running...';
  btn.disabled = true;
  out.innerHTML = '<span class="output-info">Running...</span>';
  try {
    const resp = await fetch('/run', {
      method: 'POST',
      headers: {'Content-Type': 'application/json'},
      body: JSON.stringify({code})
    });
    const data = await resp.json();
    elapsed.textContent = data.elapsed_ms + ' ms';
    if (data.error) {
      out.innerHTML = '<span class="output-err">Error:\n' + escHtml(data.error) + '</span>';
    } else if (data.output) {
      out.innerHTML = '<span class="output-ok">' + escHtml(data.output) + '</span>';
    } else {
      out.innerHTML = '<span class="output-info">(no output)</span>';
    }
  } catch(e) {
    out.innerHTML = '<span class="output-err">Connection error: ' + e.message + '</span>';
  }
  btn.textContent = '▶ Run';
  btn.disabled = false;
}

function clearOutput() {
  document.getElementById('output').innerHTML = '<span class="output-info">Output cleared.</span>';
  document.getElementById('elapsed').textContent = '';
}

function escHtml(s) {
  return s.replace(/&/g,'&amp;').replace(/</g,'&lt;').replace(/>/g,'&gt;').replace(/\n/g,'<br>');
}

document.getElementById('code').addEventListener('keydown', e => {
  if ((e.ctrlKey || e.metaKey) && e.key === 'Enter') { e.preventDefault(); runCode(); }
});
</script>
</body>
</html>"#;
