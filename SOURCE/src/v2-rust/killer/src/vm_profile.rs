//! VM execution profiler: per-function call counts with inclusive and self wall time, per-line
//! hit counts and time, a call tree for flame graphs, and the text / HTML reports.
//! (Not to be confused with `profiler`, the older source-level analysis tool.)
//!
//! The VM keeps a `profiling: bool` and an `Option<Box<Profiler>>`. When profiling is off the
//! call, return and per-instruction paths pay one predictable branch on that bool and nothing
//! else. When it is on the VM calls [`Profiler::enter`] on every call, [`Profiler::exit`] on every
//! return and [`Profiler::step`] before every instruction (cheap: one table lookup, timing only
//! at statement starts). Native JIT paths are disabled while profiling so every call and loop
//! iteration is seen.
//!
//! Accounting: a frame's *self* time is its wall time minus the wall time of the calls it made;
//! *total* (inclusive) time counts a recursive function once (outermost activation only). Line
//! time is the time between one statement start and the next, so a line that calls a function
//! is charged only for its own work, the callee's lines are charged to the callee.

use std::collections::HashMap;
use std::time::Instant;

use crate::bytecode::Program;
use crate::error::VmError;
use crate::value::{SharedArray, SharedDict, Value};
use crate::vm::VirtualMachine;

const MAX_TREE_DEPTH: usize = 128;
pub const MAIN: &str = "<main>";

#[derive(Clone, Default, Debug)]
struct FnStat {
    calls: u64,
    self_ns: u64,
    total_ns: u64,
    active: u32,
}

#[derive(Clone, Debug)]
struct Node {
    fn_id: u32,
    calls: u64,
    self_ns: u64,
    total_ns: u64,
}

struct Frame {
    depth: usize,
    fn_id: u32,
    node: u32,
    /// false when the tree depth cap folded this call into its parent's node
    own_node: bool,
    recursive: bool,
    start: Instant,
    child_ns: u64,
    /// the line entry that was current when the call happened (restored on return)
    caller_line: Option<u32>,
}

pub struct Profiler {
    names: Vec<String>,
    by_start: HashMap<usize, u32>,
    fn_names: HashMap<usize, String>,
    lines: crate::bytecode::LineTable,
    flat: Vec<FnStat>,
    nodes: Vec<Node>,
    children: HashMap<(u32, u32), u32>,
    frames: Vec<Frame>,
    /// instruction index -> line entry index + 1, nonzero only at a statement's first instruction
    entry_at: Vec<u32>,
    line_hits: Vec<u64>,
    line_ns: Vec<u64>,
    cur_line: Option<(u32, Instant)>,
    started: Instant,
    /// `call_stack.len()` seen at the previous step: a change means a call or a return
    last_depth: usize,
    stopped: Option<Instant>,
    /// instruction index -> true at a function's first instruction
    is_fn_start: Vec<bool>,
}

/// One row of the function table.
#[derive(Clone, Debug, PartialEq)]
pub struct FnRow {
    pub name: String,
    pub calls: u64,
    pub self_ms: f64,
    pub total_ms: f64,
    pub self_pct: f64,
}

/// One row of the line table.
#[derive(Clone, Debug, PartialEq)]
pub struct LineRow {
    pub file: String,
    pub line: u32,
    pub hits: u64,
    pub ms: f64,
}

/// A node of the call tree (for flame graphs).
#[derive(Clone, Debug)]
pub struct TreeNode {
    pub name: String,
    pub calls: u64,
    pub total_ms: f64,
    pub self_ms: f64,
    pub children: Vec<TreeNode>,
}

fn ms(ns: u64) -> f64 {
    ns as f64 / 1_000_000.0
}

impl Profiler {
    /// `depth` is the current `call_stack.len()` (0 when starting before the program runs).
    pub fn new(program: &Program, depth: usize) -> Self {
        let mut is_fn_start = vec![false; program.instructions.len() + 1];
        for k in program.function_names.keys() {
            if let Some(s) = is_fn_start.get_mut(*k) {
                *s = true;
            }
        }
        let mut entry_at = vec![0u32; program.instructions.len() + 1];
        for (i, e) in program.line_table.entries.iter().enumerate() {
            if let Some(slot) = entry_at.get_mut(e.0 as usize) {
                *slot = i as u32 + 1;
            }
        }
        let n = program.line_table.entries.len();
        let now = Instant::now();
        let mut p = Profiler {
            names: vec![MAIN.to_string()],
            by_start: HashMap::new(),
            fn_names: program.function_names.clone(),
            lines: program.line_table.clone(),
            flat: vec![FnStat::default()],
            nodes: vec![Node { fn_id: 0, calls: 1, self_ns: 0, total_ns: 0 }],
            children: HashMap::new(),
            frames: Vec::new(),
            entry_at,
            line_hits: vec![0; n],
            line_ns: vec![0; n],
            cur_line: None,
            started: now,
            last_depth: depth,
            stopped: None,
            is_fn_start,
        };
        p.flat[0].calls = 1;
        p.flat[0].active = 1;
        p.frames.push(Frame { depth: 0, fn_id: 0, node: 0, own_node: true, recursive: false, start: now, child_ns: 0, caller_line: None });
        p
    }

    fn fn_id_for(&mut self, start: usize) -> u32 {
        if let Some(id) = self.by_start.get(&start) {
            return *id;
        }
        let name = match self.fn_names.get(&start) {
            Some(n) => n.clone(),
            None => match self.lines.lookup(start) {
                Some((f, l)) => format!("<anonymous {}:{}>", f.rsplit(['/', '\\']).next().unwrap_or(f), l),
                None => format!("<anonymous @{}>", start),
            },
        };
        let id = match self.names.iter().position(|n| *n == name) {
            Some(i) => i as u32,
            None => {
                self.names.push(name);
                self.flat.push(FnStat::default());
                (self.names.len() - 1) as u32
            }
        };
        self.by_start.insert(start, id);
        id
    }

    /// A call to the function starting at instruction `start`; `depth` is `call_stack.len()`
    /// after the VM pushed the return address.
    pub fn enter(&mut self, start: usize, depth: usize) {
        let now = Instant::now();
        let id = self.fn_id_for(start);
        let parent = self.frames.last().map(|f| f.node).unwrap_or(0);
        let (node, own) = if self.frames.len() >= MAX_TREE_DEPTH {
            (parent, false)
        } else {
            let key = (parent, id);
            let n = match self.children.get(&key) {
                Some(n) => *n,
                None => {
                    self.nodes.push(Node { fn_id: id, calls: 0, self_ns: 0, total_ns: 0 });
                    let n = (self.nodes.len() - 1) as u32;
                    self.children.insert(key, n);
                    n
                }
            };
            self.nodes[n as usize].calls += 1;
            (n, true)
        };
        let st = &mut self.flat[id as usize];
        st.calls += 1;
        let recursive = st.active > 0;
        st.active += 1;
        let caller_line = self.cur_line.map(|l| l.0);
        self.frames.push(Frame { depth, fn_id: id, node, own_node: own, recursive, start: now, child_ns: 0, caller_line });
    }

    /// A return: close every frame entered at call depth `depth` or deeper (`depth` is
    /// `call_stack.len()` before the VM pops the return address). Also used after an error
    /// unwound the call stack.
    pub fn exit(&mut self, depth: usize) {
        if self.frames.len() <= 1 || self.frames.last().map_or(true, |f| f.depth < depth) {
            return;
        }
        let now = Instant::now();
        self.close_line(now);
        let mut caller_line = None;
        while self.frames.len() > 1 && self.frames.last().map_or(false, |f| f.depth >= depth) {
            let f = self.frames.pop().expect("checked");
            Self::account(&mut self.flat, &mut self.nodes, &f, now);
            let elapsed = now.duration_since(f.start).as_nanos() as u64;
            if let Some(p) = self.frames.last_mut() {
                p.child_ns += elapsed;
            }
            caller_line = f.caller_line;
        }
        // the rest of the caller's statement is charged to the caller's line
        self.cur_line = caller_line.map(|l| (l, now));
    }

    fn account(flat: &mut [FnStat], nodes: &mut [Node], f: &Frame, now: Instant) {
        let elapsed = now.duration_since(f.start).as_nanos() as u64;
        let own = elapsed.saturating_sub(f.child_ns);
        let st = &mut flat[f.fn_id as usize];
        st.self_ns += own;
        st.active = st.active.saturating_sub(1);
        if !f.recursive {
            st.total_ns += elapsed;
        }
        let n = &mut nodes[f.node as usize];
        n.self_ns += own;
        if f.own_node {
            n.total_ns += elapsed;
        }
    }

    fn close_line(&mut self, now: Instant) {
        if let Some((e, t)) = self.cur_line.take() {
            self.line_ns[e as usize] += now.duration_since(t).as_nanos() as u64;
        }
    }

    /// Called before every instruction while profiling. Calls and returns are detected from the
    /// VM's call-stack depth, so every call path (plain, dynamic, methods, callbacks from Rust,
    /// errors unwinding frames) is covered without a hook at each site; a call's first
    /// instruction is its function's entry, and a function entry at an unchanged depth is a tail
    /// call.
    #[inline]
    pub fn step(&mut self, ip: usize, depth: usize) {
        if depth != self.last_depth {
            if depth > self.last_depth {
                self.enter(ip, depth);
            } else {
                self.exit(depth + 1);
            }
            self.last_depth = depth;
        } else if self.is_fn_start.get(ip).copied().unwrap_or(false) {
            self.exit(depth);
            self.enter(ip, depth);
        }
        let e = match self.entry_at.get(ip) {
            Some(e) if *e != 0 => *e - 1,
            _ => return,
        };
        let now = Instant::now();
        self.close_line(now);
        self.line_hits[e as usize] += 1;
        self.cur_line = Some((e, now));
    }

    // --- reports -----------------------------------------------------------------

    /// Freeze the clock: later reports measure up to this moment.
    pub fn stop(&mut self) {
        if self.stopped.is_none() {
            let now = Instant::now();
            self.close_line(now);
            self.stopped = Some(now);
        }
    }

    /// Show `name` instead of the compiler's `<script>` placeholder in line reports.
    pub fn rename_file(&mut self, name: &str) {
        for f in self.lines.files.iter_mut() {
            if f == "<script>" {
                *f = name.to_string();
            }
        }
    }

    /// Wall time since profiling started, in ms.
    pub fn elapsed_ms(&self) -> f64 {
        ms(self.stopped.unwrap_or_else(Instant::now).duration_since(self.started).as_nanos() as u64)
    }

    /// Snapshot with the still-open frames closed at "now" (the profiler stays usable).
    fn snapshot(&self) -> (Vec<FnStat>, Vec<Node>, Vec<u64>) {
        let mut flat = self.flat.clone();
        let mut nodes = self.nodes.clone();
        let mut line_ns = self.line_ns.clone();
        let now = self.stopped.unwrap_or_else(Instant::now);
        if let Some((e, t)) = self.cur_line {
            line_ns[e as usize] += now.duration_since(t).as_nanos() as u64;
        }
        let mut child_of_above = 0u64;
        for f in self.frames.iter().rev() {
            let g = Frame {
                depth: f.depth,
                fn_id: f.fn_id,
                node: f.node,
                own_node: f.own_node,
                recursive: f.recursive,
                start: f.start,
                child_ns: f.child_ns + child_of_above,
                caller_line: None,
            };
            Self::account(&mut flat, &mut nodes, &g, now);
            child_of_above = now.duration_since(f.start).as_nanos() as u64;
        }
        (flat, nodes, line_ns)
    }

    /// Function rows sorted by self time (descending), ties by name.
    pub fn functions(&self) -> Vec<FnRow> {
        let (flat, _, _) = self.snapshot();
        let total_self: u64 = flat.iter().map(|s| s.self_ns).sum();
        let mut rows: Vec<FnRow> = flat
            .iter()
            .enumerate()
            .filter(|(_, s)| s.calls > 0)
            .map(|(i, s)| FnRow {
                name: self.names[i].clone(),
                calls: s.calls,
                self_ms: ms(s.self_ns),
                total_ms: ms(s.total_ns),
                self_pct: if total_self == 0 { 0.0 } else { s.self_ns as f64 * 100.0 / total_self as f64 },
            })
            .collect();
        rows.sort_by(|a, b| b.self_ms.partial_cmp(&a.self_ms).unwrap_or(std::cmp::Ordering::Equal).then_with(|| a.name.cmp(&b.name)));
        rows
    }

    /// Line rows (only lines that ran), sorted by time then hits.
    pub fn lines(&self) -> Vec<LineRow> {
        let (_, _, line_ns) = self.snapshot();
        let mut acc: HashMap<(u32, u32), (u64, u64)> = HashMap::new();
        for (i, e) in self.lines.entries.iter().enumerate() {
            if self.line_hits[i] > 0 {
                let a = acc.entry((e.1, e.2)).or_insert((0, 0));
                a.0 += self.line_hits[i];
                a.1 += line_ns[i];
            }
        }
        let mut rows: Vec<LineRow> = acc
            .into_iter()
            .map(|((f, l), (h, ns))| LineRow { file: self.lines.files.get(f as usize).cloned().unwrap_or_default(), line: l, hits: h, ms: ms(ns) })
            .collect();
        rows.sort_by(|a, b| b.ms.partial_cmp(&a.ms).unwrap_or(std::cmp::Ordering::Equal).then(b.hits.cmp(&a.hits)).then(a.line.cmp(&b.line)));
        rows
    }

    /// The call tree rooted at `<main>`, children sorted by name.
    pub fn tree(&self) -> TreeNode {
        let (_, nodes, _) = self.snapshot();
        let mut kids: HashMap<u32, Vec<u32>> = HashMap::new();
        for ((p, _), c) in &self.children {
            kids.entry(*p).or_default().push(*c);
        }
        fn build(i: u32, nodes: &[Node], names: &[String], kids: &HashMap<u32, Vec<u32>>) -> TreeNode {
            let n = &nodes[i as usize];
            let mut ch: Vec<TreeNode> = kids.get(&i).map(|v| v.iter().map(|c| build(*c, nodes, names, kids)).collect()).unwrap_or_default();
            ch.sort_by(|a, b| a.name.cmp(&b.name));
            TreeNode { name: names[n.fn_id as usize].clone(), calls: n.calls, total_ms: ms(n.total_ns), self_ms: ms(n.self_ns), children: ch }
        }
        build(0, &nodes, &self.names, &kids)
    }

    pub fn render_text(&self, top_fns: usize, top_lines: usize) -> String {
        let rows = self.functions();
        let total_calls: u64 = rows.iter().map(|r| r.calls).sum();
        let mut out = format!("Profile: {:.2} ms wall, {} function calls\n\n", self.elapsed_ms(), total_calls.saturating_sub(1));
        let w = rows.iter().take(top_fns).map(|r| r.name.chars().count()).max().unwrap_or(8).clamp(8, 48);
        out.push_str(&format!("{:<w$}  {:>10}  {:>10}  {:>10}  {:>6}\n", "function", "calls", "self ms", "total ms", "self %", w = w));
        for r in rows.iter().take(top_fns) {
            let name: String = r.name.chars().take(48).collect();
            out.push_str(&format!("{:<w$}  {:>10}  {:>10.3}  {:>10.3}  {:>5.1}%\n", name, r.calls, r.self_ms, r.total_ms, r.self_pct, w = w));
        }
        if rows.len() > top_fns {
            out.push_str(&format!("... {} more functions\n", rows.len() - top_fns));
        }
        let lines = self.lines();
        if top_lines > 0 && !lines.is_empty() {
            out.push_str("\nhot lines\n");
            out.push_str(&format!("{:<28}  {:>10}  {:>10}\n", "line", "hits", "ms"));
            for l in lines.iter().take(top_lines) {
                let file = l.file.rsplit(['/', '\\']).next().unwrap_or(&l.file);
                out.push_str(&format!("{:<28}  {:>10}  {:>10.3}\n", format!("{}:{}", file, l.line), l.hits, l.ms));
            }
        }
        out
    }

    /// A self-contained HTML page: flame graph (click a box to zoom, click the root to reset),
    /// function table and hot lines. No external resources.
    pub fn render_html(&self, title: &str) -> String {
        fn json_str(s: &str) -> String {
            let mut o = String::from("\"");
            for c in s.chars() {
                match c {
                    '"' => o.push_str("\\\""),
                    '\\' => o.push_str("\\\\"),
                    '\n' => o.push_str("\\n"),
                    '<' => o.push_str("\\u003c"),
                    '>' => o.push_str("\\u003e"),
                    '&' => o.push_str("\\u0026"),
                    c if (c as u32) < 0x20 => o.push_str(&format!("\\u{:04x}", c as u32)),
                    c => o.push(c),
                }
            }
            o.push('"');
            o
        }
        fn node_json(n: &TreeNode, out: &mut String) {
            out.push_str(&format!("{{\"n\":{},\"c\":{},\"t\":{:.4},\"s\":{:.4},\"k\":[", json_str(&n.name), n.calls, n.total_ms, n.self_ms));
            for (i, c) in n.children.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                node_json(c, out);
            }
            out.push_str("]}");
        }
        fn esc(s: &str) -> String {
            s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
        }
        let mut tree = String::new();
        node_json(&self.tree(), &mut tree);
        let mut table = String::new();
        for r in self.functions() {
            table.push_str(&format!(
                "<tr><td>{}</td><td>{}</td><td>{:.3}</td><td>{:.3}</td><td>{:.1}%</td></tr>\n",
                esc(&r.name), r.calls, r.self_ms, r.total_ms, r.self_pct
            ));
        }
        let mut lines = String::new();
        for l in self.lines().iter().take(50) {
            let file = l.file.rsplit(['/', '\\']).next().unwrap_or(&l.file);
            lines.push_str(&format!("<tr><td>{}:{}</td><td>{}</td><td>{:.3}</td></tr>\n", esc(file), l.line, l.hits, l.ms));
        }
        format!(
            r##"<!DOCTYPE html>
<html lang="en"><head><meta charset="utf-8"><title>{title}</title>
<style>
body{{font:13px/1.4 system-ui,sans-serif;margin:16px;color:#222}}
h1{{font-size:18px}} h2{{font-size:15px;margin-top:24px}}
#flame{{position:relative;width:100%;border:1px solid #ccc;background:#fafafa}}
.b{{position:absolute;height:18px;box-sizing:border-box;border:1px solid #fff;overflow:hidden;white-space:nowrap;font-size:11px;line-height:16px;padding:0 2px;cursor:pointer;color:#111}}
.b:hover{{outline:1px solid #000}}
table{{border-collapse:collapse}} td,th{{border:1px solid #ddd;padding:2px 8px;text-align:right}} td:first-child,th:first-child{{text-align:left}}
#info{{margin:6px 0;min-height:18px;color:#555}}
</style></head><body>
<h1>{title}</h1>
<div id="info">Flame graph: width is inclusive time. Click a box to zoom.</div>
<div id="flame"></div>
<h2>Functions (by self time)</h2>
<table><tr><th>function</th><th>calls</th><th>self ms</th><th>total ms</th><th>self %</th></tr>
{table}</table>
<h2>Hot lines</h2>
<table><tr><th>line</th><th>hits</th><th>ms</th></tr>
{lines}</table>
<script>
var T={tree};
var flame=document.getElementById("flame"),info=document.getElementById("info"),ROW=18;
function hue(s){{var h=0;for(var i=0;i<s.length;i++)h=(h*31+s.charCodeAt(i))>>>0;return 20+h%40;}}
function depthOf(n){{var d=0;for(var i=0;i<n.k.length;i++)d=Math.max(d,depthOf(n.k[i]));return d+1;}}
function draw(root){{
  flame.innerHTML="";flame.style.height=(depthOf(root)*ROW+2)+"px";
  var total=root.t>0?root.t:1;
  (function walk(n,x,d){{
    var w=n.t/total*100; if(w<0.05)return;
    var b=document.createElement("div");b.className="b";
    b.style.left=x+"%";b.style.width=w+"%";b.style.top=(d*ROW)+"px";
    b.style.background="hsl("+hue(n.n)+",75%,65%)";
    b.textContent=n.n+" ("+n.t.toFixed(2)+" ms)";
    b.title=n.n+"\ncalls: "+n.c+"\ntotal: "+n.t.toFixed(3)+" ms\nself: "+n.s.toFixed(3)+" ms";
    b.onclick=function(e){{e.stopPropagation();draw(n===root?T:n);}};
    b.onmouseover=function(){{info.textContent=b.title.replace(/\n/g,"   ");}};
    flame.appendChild(b);
    var cx=x;for(var i=0;i<n.k.length;i++){{walk(n.k[i],cx,d+1);cx+=n.k[i].t/total*100;}}
  }})(root,0,0);
}}
draw(T);
</script></body></html>
"##,
            title = esc(title),
            tree = tree,
            table = table,
            lines = lines
        )
    }
}

impl VirtualMachine {
    /// Start collecting a profile for `program` (replaces any earlier one).
    pub fn start_profiling(&mut self, program: &Program) {
        self.profiler = Some(Box::new(Profiler::new(program, self.call_stack_depth())));
        self.profiling = true;
    }

    /// Stop collecting and hand back the profiler (for reports).
    pub fn stop_profiling(&mut self) -> Option<Box<Profiler>> {
        self.profiling = false;
        let mut p = self.profiler.take();
        if let Some(p) = p.as_mut() {
            p.stop();
        }
        p
    }
}

pub const NAMES: &[&str] = &["profile_start", "profile_stop", "profile_report"];

fn row_dict(pairs: Vec<(&str, Value)>) -> Value {
    Value::Dict(SharedDict::from_pairs(pairs.into_iter().map(|(k, v)| (k.to_string(), v)).collect()))
}

/// VM-level builtins: `profile_start()`, `profile_stop()`, `profile_report([format])` where format is
/// "text" (default; a table sorted by self time plus hot lines), "html" (self-contained flame
/// graph page), "functions" (array of `{name, calls, self_ms, total_ms, self_pct}`) or "lines"
/// (array of `{file, line, hits, ms}`). `profile_report` works while profiling or after
/// `profile_stop()`; `profile_start()` begins a fresh profile.
pub fn vm_call(vm: &mut VirtualMachine, name: &str, args: &[Value], program: &Program) -> Option<Result<Value, VmError>> {
    match name {
        "profile_start" => {
            vm.start_profiling(program);
            Some(Ok(Value::Null))
        }
        "profile_stop" => {
            vm.profiling = false;
            if let Some(p) = vm.profiler.as_mut() {
                p.stop();
            }
            Some(Ok(Value::Null))
        }
        "profile_report" => Some((|| {
            let p = vm.profiler.as_ref().ok_or_else(|| VmError::runtime_error("profile_report(): no profile; call profile_start() first".to_string()))?;
            let fmt = match args.first() {
                None | Some(Value::Null) => "text".to_string(),
                Some(Value::Str(s)) => s.to_lowercase(),
                Some(_) => return Err(VmError::runtime_error("profile_report(format) expects a string".to_string())),
            };
            match fmt.as_str() {
                "text" => Ok(Value::Str(p.render_text(30, 15))),
                "html" => Ok(Value::Str(p.render_html("Killer profile"))),
                "functions" => Ok(Value::Array(SharedArray::new(
                    p.functions()
                        .into_iter()
                        .map(|r| {
                            row_dict(vec![
                                ("name", Value::Str(r.name)),
                                ("calls", Value::Number(r.calls as f64)),
                                ("self_ms", Value::Number(r.self_ms)),
                                ("total_ms", Value::Number(r.total_ms)),
                                ("self_pct", Value::Number(r.self_pct)),
                            ])
                        })
                        .collect(),
                ))),
                "lines" => Ok(Value::Array(SharedArray::new(
                    p.lines()
                        .into_iter()
                        .map(|r| {
                            row_dict(vec![
                                ("file", Value::Str(r.file)),
                                ("line", Value::Number(r.line as f64)),
                                ("hits", Value::Number(r.hits as f64)),
                                ("ms", Value::Number(r.ms)),
                            ])
                        })
                        .collect(),
                ))),
                other => Err(VmError::runtime_error(format!("profile_report: unknown format {} (text, html, functions, lines)", other))),
            }
        })()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn prog() -> Program {
        let mut p = Program {
            instructions: vec![],
            function_arities: HashMap::new(),
            function_names: HashMap::new(),
            method_bytecode: HashMap::new(),
            classes: HashMap::new(),
            live_vars: HashMap::new(),
            line_table: Default::default(),
        };
        p.function_names.insert(10, "alpha".to_string());
        p.function_names.insert(20, "beta".to_string());
        p
    }

    #[test]
    fn calls_self_and_total_time_nest_correctly() {
        let p = prog();
        let mut pr = Profiler::new(&p, 0);
        // The sleeps are far apart on purpose: a loaded CI machine (macOS runners especially) can
        // oversleep by tens of milliseconds, and the ordering below must survive that.
        pr.enter(10, 1); // alpha
        std::thread::sleep(std::time::Duration::from_millis(2));
        pr.enter(20, 2); // beta inside alpha
        std::thread::sleep(std::time::Duration::from_millis(150));
        pr.exit(2);
        pr.exit(1);
        pr.enter(20, 1); // beta from main
        pr.exit(1);
        let rows = pr.functions();
        let get = |n: &str| rows.iter().find(|r| r.name == n).unwrap().clone();
        assert_eq!(get("alpha").calls, 1);
        assert_eq!(get("beta").calls, 2);
        assert!(get("beta").self_ms >= 100.0, "{:?}", rows);
        assert!(get("alpha").total_ms >= get("alpha").self_ms + 100.0, "{:?}", rows);
        assert_eq!(rows[0].name, "beta");
        let t = pr.tree();
        assert_eq!(t.name, MAIN);
        let alpha = t.children.iter().find(|c| c.name == "alpha").unwrap();
        assert_eq!(alpha.children[0].name, "beta");
        assert_eq!(t.children.iter().find(|c| c.name == "beta").unwrap().calls, 1);
    }

    #[test]
    fn recursion_counts_inclusive_time_once() {
        let p = prog();
        let mut pr = Profiler::new(&p, 0);
        pr.enter(10, 1);
        pr.enter(10, 2);
        std::thread::sleep(std::time::Duration::from_millis(10));
        pr.exit(2);
        pr.exit(1);
        let a = pr.functions().into_iter().find(|r| r.name == "alpha").unwrap();
        assert_eq!(a.calls, 2);
        assert!(a.total_ms < a.self_ms + 5.0, "{:?}", a);
    }

    #[test]
    fn exit_after_an_unwind_closes_all_deeper_frames() {
        let p = prog();
        let mut pr = Profiler::new(&p, 0);
        pr.enter(10, 1);
        pr.enter(20, 2);
        pr.exit(1); // an exception unwound both
        assert_eq!(pr.frames.len(), 1);
    }

    #[test]
    fn html_is_self_contained_and_escaped() {
        let mut p = prog();
        p.function_names.insert(30, "<b>&evil\"".to_string());
        let mut pr = Profiler::new(&p, 0);
        pr.enter(30, 1);
        pr.exit(1);
        let h = pr.render_html("t <x>");
        assert!(h.starts_with("<!DOCTYPE html>"));
        assert!(!h.contains("http://") && !h.contains("https://") && !h.contains("src="));
        assert!(!h.contains("<b>&evil"));
        assert!(h.contains("&lt;b&gt;&amp;evil"));
        assert!(h.contains("\\u003cb\\u003e"));
    }
}
