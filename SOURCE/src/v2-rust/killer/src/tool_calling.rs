// ===============================================================================
// NOVA GALAXY ENGINE v1 — KhLM Tool Calling Engine
// AI can autonomously call Killer builtins as tools during reasoning
//
// Architecture:
//   tool_register(name, desc, params)    → register a tool the AI can call
//   tool_list()                          → list all registered tools (shown to AI)
//   tool_call(name, arg1, arg2, ...)     → manually invoke a registered tool
//   khlm_with_tools(prompt)             → let KhLM AI call tools to answer prompt
//   khlm_tool_status()                   → show tool registry state
//   khlm_tool_clear()                    → reset tool registry
//
// Tool Call Protocol:
//   LLM is given a TOOL_LIST in system prompt.
//   LLM emits: TOOL_CALL: {"name":"http_get","args":["https://..."]}
//   Engine parses this, dispatches to registered function, injects result.
//   LLM continues reasoning with: TOOL_RESULT: {"name":"http_get","result":"..."}
//   Final answer stripped of all TOOL_CALL/TOOL_RESULT lines.
//
// Built-in tools auto-registered on init:
//   http_get, http_post_json, http_status, http_head, http_download,
//   vmem_*, polyglot_list, polyglot_check, readFile, parse_json
//
// Zero external crates — pure std
// ===============================================================================

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use crate::builtin_dispatch::call_builtin;
use crate::value::Value;

// --- Tool Definition ----------------------------------------------------------

/// A tool that the AI can call.
#[derive(Debug, Clone)]
pub struct AiTool {
    /// Name the AI uses to call this tool (e.g. "http_get")
    pub name: String,
    /// One-line description shown in the AI's system prompt
    pub description: String,
    /// Parameter names + descriptions (for the AI to understand inputs)
    pub params: Vec<(String, String)>,
    /// Number of times this tool was called
    pub call_count: u64,
}

/// Tool call registry.
pub struct ToolRegistry {
    pub tools: HashMap<String, AiTool>,
}

// --- Global Singleton ---------------------------------------------------------

static REGISTRY: OnceLock<Mutex<ToolRegistry>> = OnceLock::new();

pub fn registry() -> &'static Mutex<ToolRegistry> {
    REGISTRY.get_or_init(|| {
        let mut reg = ToolRegistry { tools: HashMap::new() };
        // Auto-register built-in tools
        reg.register_builtin_tools();
        Mutex::new(reg)
    })
}

// --- Registry Implementation --------------------------------------------------

impl ToolRegistry {
    fn register_builtin_tools(&mut self) {
        self.add("http_get",
            "Fetch the content of a URL (HTTPS only)",
            &[("url", "HTTPS URL to fetch")]);

        self.add("http_post_json",
            "POST JSON to a URL (HTTPS only) and return response",
            &[("url", "HTTPS URL"), ("json", "JSON string body")]);

        self.add("http_status",
            "Get HTTP status code for a URL (200=ok, 404=not found, etc.)",
            &[("url", "HTTPS URL to check")]);

        self.add("vmem_search",
            "Search vector memory for relevant past answers",
            &[("query", "Search query text"), ("top_k", "Max results to return (default 3)")]);

        self.add("vmem_store",
            "Store information in vector memory for later retrieval",
            &[("key", "Unique identifier"), ("text", "Content to remember")]);

        self.add("vmem_recall",
            "Recall an exact entry from vector memory by key",
            &[("key", "Key to look up")]);

        self.add("polyglot_list",
            "List all programming language runtimes available on this machine",
            &[]);

        self.add("polyglot_check",
            "Check if a specific language runtime is available (e.g. python, go, node)",
            &[("lang", "Language name to check")]);

        self.add("readFile",
            "Read a UTF-8 text file (relative path). Returns contents or empty on failure.",
            &[("path", "Path to the file")]);

        self.add("parse_json",
            "Parse a JSON object string into Killer dict-compatible data (string keys).",
            &[("json", "JSON object as a single string")]);
    }

    pub fn add(&mut self, name: &str, desc: &str, params: &[(&str, &str)]) {
        self.tools.insert(name.to_string(), AiTool {
            name: name.to_string(),
            description: desc.to_string(),
            params: params.iter().map(|(p, d)| (p.to_string(), d.to_string())).collect(),
            call_count: 0,
        });
    }

    /// Remove a tool from registry.
    pub fn remove(&mut self, name: &str) -> bool {
        self.tools.remove(name).is_some()
    }

    /// Build the tools listing to inject into the AI system prompt.
    pub fn tools_prompt(&self) -> String {
        if self.tools.is_empty() {
            return String::new();
        }
        let mut out = String::from(
            "You have access to the following tools. To call a tool, output EXACTLY:\n\
             TOOL_CALL: {\"name\":\"tool_name\",\"args\":[\"arg1\",\"arg2\"]}\n\
             After seeing TOOL_RESULT, continue your reasoning.\n\
             Available tools:\n"
        );
        let mut tool_list: Vec<&AiTool> = self.tools.values().collect();
        tool_list.sort_by(|a, b| a.name.cmp(&b.name));
        for tool in tool_list {
            let param_str = tool.params.iter()
                .map(|(p, d)| format!("{}: {}", p, d))
                .collect::<Vec<_>>()
                .join(", ");
            out.push_str(&format!("  • {} — {} ({})\n",
                tool.name, tool.description,
                if param_str.is_empty() { "no args".to_string() } else { param_str }
            ));
        }
        out
    }

    /// Status display.
    pub fn status(&self) -> String {
        let total_calls: u64 = self.tools.values().map(|t| t.call_count).sum();
        let mut tool_list: Vec<&AiTool> = self.tools.values().collect();
        tool_list.sort_by(|a, b| a.name.cmp(&b.name));
        let mut out = format!("KhLM Tool Registry\n  Tools: {}   Total calls: {}\n",
            self.tools.len(), total_calls);
        for t in tool_list {
            out.push_str(&format!("  {}  calls={}  {}\n",
                t.name, t.call_count, t.description));
        }
        out
    }
}

// --- Tool Call Parser ---------------------------------------------------------

/// Parsed tool invocation from LLM output.
#[derive(Debug)]
struct ToolCallRequest {
    name: String,
    args: Vec<String>,
}

/// Parse `TOOL_CALL: {"name":"...","args":["a","b"]}` from a line.
fn parse_tool_call(line: &str) -> Option<ToolCallRequest> {
    let prefix = "TOOL_CALL:";
    let trimmed = line.trim();
    if !trimmed.starts_with(prefix) { return None; }
    let json = trimmed[prefix.len()..].trim();

    // Extract "name" field
    let name = extract_json_str(json, "name")?;

    // Extract "args" array
    let args = extract_json_str_array(json, "args").unwrap_or_default();

    Some(ToolCallRequest { name, args })
}

/// Minimal JSON string extractor (no serde needed).
fn extract_json_str(json: &str, key: &str) -> Option<String> {
    let needle = format!("\"{}\":", key);
    let pos = json.find(&needle)?;
    let rest = json[pos + needle.len()..].trim_start();
    if !rest.starts_with('"') { return None; }
    let inner = &rest[1..];
    let end = inner.find('"')?;
    Some(inner[..end].to_string())
}

/// Minimal JSON string-array extractor.
fn extract_json_str_array(json: &str, key: &str) -> Option<Vec<String>> {
    let needle = format!("\"{}\":", key);
    let pos = json.find(&needle)?;
    let rest = json[pos + needle.len()..].trim_start();
    if !rest.starts_with('[') { return None; }
    let end = rest.find(']')?;
    let inner = &rest[1..end];
    // Parse quoted strings
    let mut result = Vec::new();
    let mut s = inner;
    while let Some(start) = s.find('"') {
        s = &s[start + 1..];
        if let Some(end_quote) = s.find('"') {
            result.push(s[..end_quote].to_string());
            s = &s[end_quote + 1..];
        } else {
            break;
        }
    }
    Some(result)
}

// --- Tool Dispatcher ----------------------------------------------------------

/// Dispatch a tool call by name+args to the corresponding Killer builtin.
/// Returns the tool result as a String.
pub fn dispatch_tool(name: &str, args: &[String]) -> String {
    let val_args: Vec<Value> = args.iter()
        .map(|a| {
            // Try to parse as number, otherwise string
            if let Ok(n) = a.parse::<f64>() {
                Value::Number(n)
            } else {
                Value::Str(a.clone())
            }
        })
        .collect();

    // All builtins now return Result<Value, VmError> — unwrap to Value
    let to_val = |r: Result<Value, crate::error::VmError>| {
        r.unwrap_or_else(|e| Value::Str(format!("Tool error: {:?}", e)))
    };

    let result: Value = match name {
        "http_get"          => to_val(crate::http_client::builtin_http_get(&val_args)),
        "http_post_json"    => to_val(crate::http_client::builtin_http_post_json(&val_args)),
        "http_status"       => to_val(crate::http_client::builtin_http_status(&val_args)),
        "http_head"         => to_val(crate::http_client::builtin_http_head(&val_args)),
        "http_download"     => to_val(crate::http_client::builtin_http_download(&val_args)),
        "vmem_search"       => to_val(crate::vector_memory::builtin_vmem_search(&val_args)),
        "vmem_store"        => to_val(crate::vector_memory::builtin_vmem_store(&val_args)),
        "vmem_recall"       => to_val(crate::vector_memory::builtin_vmem_recall(&val_args)),
        "polyglot_list"     => to_val(crate::polyglot::builtin_polyglot_list(&val_args)),
        "polyglot_check"    => to_val(crate::polyglot::builtin_polyglot_check(&val_args)),
        "readFile"          => to_val(call_builtin("readFile", &val_args)),
        "parse_json"        => to_val(call_builtin("parse_json", &val_args)),
        _ => Value::Str(format!("Error: unknown tool '{}'", name)),
    };

    // Increment call count
    if let Ok(mut reg) = registry().lock() {
        if let Some(tool) = reg.tools.get_mut(name) {
            tool.call_count += 1;
        }
    }

    match result {
        Value::Str(s)    => s,
        Value::Number(n) => n.to_string(),
        Value::Bool(b)   => b.to_string(),
        other            => format!("{:?}", other),
    }
}

// --- KhLM + Tool Calling Integration -----------------------------------------

/// Ask KhLM a question, giving it access to tools.
/// The AI can emit TOOL_CALL lines → engine executes → result injected back.
/// Max 5 tool call rounds to prevent infinite loops.
pub fn khlm_with_tools(prompt: &str) -> String {
    let tools_prompt = {
        if let Ok(reg) = registry().lock() {
            reg.tools_prompt()
        } else {
            String::new()
        }
    };

    // Build system prompt injecting tool descriptions
    let system_msg = if tools_prompt.is_empty() {
        "You are a helpful assistant for the Killer programming language.".to_string()
    } else {
        format!(
            "You are a helpful assistant for the Killer programming language.\n\n{}",
            tools_prompt
        )
    };

    // Use KhLM to answer, loop up to 5 tool call rounds
    let current_prompt = prompt.to_string();
    let mut tool_log: Vec<String> = Vec::new();
    let max_rounds = 5;

    for round in 0..max_rounds {
        let full_prompt = if tool_log.is_empty() {
            format!("{}\n\nQuestion: {}", system_msg, current_prompt)
        } else {
            format!("{}\n\nQuestion: {}\n\n{}",
                system_msg, prompt, tool_log.join("\n"))
        };

        // Get LLM response (use khlm_ask as base — falls back gracefully offline)
        let response = crate::llm::khlm_ask(&full_prompt);

        // Scan response for TOOL_CALL lines
        let mut tool_calls_found = false;
        let mut result_lines: Vec<String> = Vec::new();

        for line in response.lines() {
            if let Some(tc) = parse_tool_call(line) {
                tool_calls_found = true;
                let tool_result = dispatch_tool(&tc.name, &tc.args);
                tool_log.push(format!("TOOL_CALL: {{\"name\":\"{}\",\"args\":[{}]}}",
                    tc.name,
                    tc.args.iter().map(|a| format!("\"{}\"", a)).collect::<Vec<_>>().join(",")
                ));
                tool_log.push(format!("TOOL_RESULT: {{\"name\":\"{}\",\"result\":{}}}",
                    tc.name,
                    serde_json_str(&tool_result)
                ));
                result_lines.push(format!("[Tool {} → {}]", tc.name, &tool_result[..tool_result.len().min(100)]));
            } else {
                result_lines.push(line.to_string());
            }
        }

        if !tool_calls_found || round == max_rounds - 1 {
            // No more tools to call — this is the final answer
            // Strip internal TOOL_CALL/TOOL_RESULT from output
            let final_answer: Vec<&str> = response.lines()
                .filter(|l| !l.trim_start().starts_with("TOOL_CALL:")
                         && !l.trim_start().starts_with("TOOL_RESULT:"))
                .collect();
            let answer = final_answer.join("\n");
            if !tool_log.is_empty() {
                return format!(
                    "{}\n\n[Tools used: {}]",
                    answer,
                    tool_log.iter()
                        .filter(|l| l.starts_with("TOOL_CALL:"))
                        .count()
                );
            }
            return answer;
        }
        // Continue with tool results injected
        let _ = result_lines; // tool_log carries state
    }

    crate::llm::khlm_ask(prompt)
}

/// Minimal JSON string escaping for tool results.
fn serde_json_str(s: &str) -> String {
    let escaped = s
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t");
    format!("\"{}\"", &escaped[..escaped.len().min(500)])
}

// --- Builtin dispatch functions -----------------------------------------------

use crate::error::VmError;

/// tool_register(name, description) → "Registered: name"
pub fn builtin_tool_register(args: &[Value]) -> Result<Value, VmError> {
    crate::security::require_llm()?;
    let name = match args.first() {
        Some(Value::Str(s)) => s.clone(),
        _ => return Ok(Value::Str("Error: tool_register(name, desc) requires a tool name".to_string())),
    };
    let desc = match args.get(1) {
        Some(Value::Str(s)) => s.clone(),
        _ => String::from("(no description)"),
    };
    if let Ok(mut reg) = registry().lock() {
        reg.add(&name, &desc, &[]);
        Ok(Value::Str(format!("Tool registered: '{}'", name)))
    } else {
        Ok(Value::Str("Error: tool registry lock failed".to_string()))
    }
}

/// tool_call(name, arg1, arg2, ...) → String (tool result)
pub fn builtin_tool_call(args: &[Value]) -> Result<Value, VmError> {
    crate::security::require_llm()?;
    let name = match args.first() {
        Some(Value::Str(s)) => s.clone(),
        _ => return Ok(Value::Str("Error: tool_call(name, args...) requires a tool name".to_string())),
    };
    let call_args: Vec<String> = args[1..].iter().map(|v| match v {
        Value::Str(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => b.to_string(),
        other => format!("{:?}", other),
    }).collect();
    Ok(Value::Str(dispatch_tool(&name, &call_args)))
}

/// tool_list() → String (formatted tool registry)
pub fn builtin_tool_list(args: &[Value]) -> Result<Value, VmError> {
    crate::security::require_llm()?;
    let _ = args;
    if let Ok(reg) = registry().lock() {
        Ok(Value::Str(reg.status()))
    } else {
        Ok(Value::Str("Error: tool registry lock failed".to_string()))
    }
}

/// khlm_with_tools(prompt) → String (AI answer using tools)
pub fn builtin_khlm_with_tools(args: &[Value]) -> Result<Value, VmError> {
    crate::security::require_llm()?;
    let prompt = match args.first() {
        Some(Value::Str(s)) => s.clone(),
        _ => return Ok(Value::Str("Error: khlm_with_tools(prompt) requires a question string".to_string())),
    };
    Ok(Value::Str(khlm_with_tools(&prompt)))
}

/// khlm_tool_status() → String
pub fn builtin_khlm_tool_status(args: &[Value]) -> Result<Value, VmError> {
    crate::security::require_llm()?;
    let _ = args;
    if let Ok(reg) = registry().lock() {
        Ok(Value::Str(reg.status()))
    } else {
        Ok(Value::Str("Error: tool registry lock failed".to_string()))
    }
}

/// khlm_tool_clear() → "cleared"
pub fn builtin_khlm_tool_clear(args: &[Value]) -> Result<Value, VmError> {
    crate::security::require_llm()?;
    let _ = args;
    if let Ok(mut reg) = registry().lock() {
        reg.tools.clear();
        Ok(Value::Str("Tool registry cleared".to_string()))
    } else {
        Ok(Value::Str("Error: tool registry lock failed".to_string()))
    }
}
