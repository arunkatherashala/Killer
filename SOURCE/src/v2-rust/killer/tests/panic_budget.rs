//! Panic budget: the number of `.unwrap()`, `.expect(`, `panic!`, `unreachable!`, `todo!`
//! and `unimplemented!` sites in NON-test code may never go UP.  With `panic = "abort"` in
//! release, each site is a potential process kill; new code must use `?` / `Result`.
//!
//! Counting mirrors `scripts/count_panics.py` (comments, strings, `#[cfg(test)]` / `#[test]`
//! items, tests/, benches/ and archive/ are ignored; `self.expect(..)` parser calls are not
//! counted).  Budgets live in `tests/panic_budget.txt` ("path count" per line, plus a
//! `TOTAL` line).  When you REMOVE sites, lower the numbers (regenerate with
//! `python scripts/count_panics.py --json`); raising a number needs a justification.
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

fn blank(chars: &[char], from: usize, to: usize, out: &mut String) {
    for c in &chars[from..to] {
        out.push(if *c == '\n' || *c == '\r' { *c } else { ' ' });
    }
}

/// Blank out comments and string / char literals, keeping the layout.
fn sanitize(src: &str) -> String {
    let c: Vec<char> = src.chars().collect();
    let n = c.len();
    let mut out = String::with_capacity(src.len());
    let mut i = 0;
    while i < n {
        let ch = c[i];
        let nx = if i + 1 < n { c[i + 1] } else { '\0' };
        if ch == '/' && nx == '/' {
            let mut j = i;
            while j < n && c[j] != '\n' {
                j += 1;
            }
            blank(&c, i, j, &mut out);
            i = j;
        } else if ch == '/' && nx == '*' {
            let mut depth = 1;
            let mut j = i + 2;
            while j < n && depth > 0 {
                if c[j] == '/' && j + 1 < n && c[j + 1] == '*' {
                    depth += 1;
                    j += 2;
                } else if c[j] == '*' && j + 1 < n && c[j + 1] == '/' {
                    depth -= 1;
                    j += 2;
                } else {
                    j += 1;
                }
            }
            blank(&c, i, j, &mut out);
            i = j;
        } else if ch == 'r'
            && (i == 0 || !(c[i - 1].is_alphanumeric() || c[i - 1] == '_'))
            && {
                let mut k = i + 1;
                while k < n && c[k] == '#' {
                    k += 1;
                }
                k < n && c[k] == '"'
            }
        {
            let mut k = i + 1;
            let mut hashes = 0;
            while c[k] == '#' {
                hashes += 1;
                k += 1;
            }
            let mut j = k + 1;
            loop {
                if j >= n {
                    break;
                }
                if c[j] == '"' && (0..hashes).all(|h| j + 1 + h < n && c[j + 1 + h] == '#') {
                    j += 1 + hashes;
                    break;
                }
                j += 1;
            }
            let j = j.min(n);
            blank(&c, i, j, &mut out);
            i = j;
        } else if ch == '"' {
            let mut j = i + 1;
            while j < n && c[j] != '"' {
                j += if c[j] == '\\' { 2 } else { 1 };
            }
            let j = (j + 1).min(n);
            blank(&c, i, j, &mut out);
            i = j;
        } else if ch == '\'' {
            // char literal ('x', '\n', '\u{..}') or a lifetime
            let mut j = None;
            if i + 2 < n && c[i + 1] != '\\' && c[i + 1] != '\'' && c[i + 2] == '\'' {
                j = Some(i + 3);
            } else if i + 1 < n && c[i + 1] == '\\' {
                let mut k = i + 2;
                while k < n && k < i + 12 && c[k] != '\'' {
                    k += 1;
                }
                if k < n && c[k] == '\'' {
                    j = Some(k + 1);
                }
            }
            match j {
                Some(j) => {
                    blank(&c, i, j, &mut out);
                    i = j;
                }
                None => {
                    out.push(ch);
                    i += 1;
                }
            }
        } else {
            out.push(ch);
            i += 1;
        }
    }
    out
}

/// Blank out items annotated `#[cfg(test)]` or `#[test]`.
fn strip_test_items(text: &str) -> String {
    let mut c: Vec<char> = text.chars().collect();
    let n = c.len();
    let mut i = 0;
    while i < n {
        if c[i] == '#' && i + 1 < n && c[i + 1] == '[' {
            // read the attribute
            let mut j = i + 2;
            let mut depth = 1;
            while j < n && depth > 0 {
                if c[j] == '[' {
                    depth += 1;
                } else if c[j] == ']' {
                    depth -= 1;
                }
                j += 1;
            }
            let attr: String = c[i + 2..j.saturating_sub(1)].iter().filter(|x| !x.is_whitespace()).collect();
            if attr == "cfg(test)" || attr == "test" {
                // the item that follows: up to `;` or a balanced `{ .. }`
                let mut k = j;
                let mut paren = 0i32;
                let end;
                loop {
                    if k >= n {
                        end = n;
                        break;
                    }
                    match c[k] {
                        '(' | '[' => paren += 1,
                        ')' | ']' => paren -= 1,
                        ';' if paren <= 0 => {
                            end = k + 1;
                            break;
                        }
                        '{' if paren <= 0 => {
                            let mut d = 1;
                            k += 1;
                            while k < n && d > 0 {
                                if c[k] == '{' {
                                    d += 1;
                                } else if c[k] == '}' {
                                    d -= 1;
                                }
                                k += 1;
                            }
                            end = k;
                            break;
                        }
                        _ => {}
                    }
                    k += 1;
                }
                for x in c.iter_mut().take(end).skip(i) {
                    if *x != '\n' && *x != '\r' {
                        *x = ' ';
                    }
                }
                i = end;
                continue;
            }
            i = j;
            continue;
        }
        i += 1;
    }
    c.into_iter().collect()
}

fn count_sites(text: &str) -> usize {
    let mut total = 0;
    let bytes = text.as_bytes();
    let boundary_ok = |pos: usize| pos == 0 || !(bytes[pos - 1].is_ascii_alphanumeric() || bytes[pos - 1] == b'_');
    for pat in [".unwrap()", ".expect("] {
        let mut from = 0;
        while let Some(p) = text[from..].find(pat) {
            let at = from + p;
            let is_self_expect = pat == ".expect(" && text[..at].ends_with("self");
            if !is_self_expect {
                total += 1;
            }
            from = at + pat.len();
        }
    }
    for pat in ["panic!", "unreachable!", "todo!", "unimplemented!"] {
        let mut from = 0;
        while let Some(p) = text[from..].find(pat) {
            let at = from + p;
            if boundary_ok(at) {
                total += 1;
            }
            from = at + pat.len();
        }
    }
    total
}

fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let rd = match std::fs::read_dir(dir) {
        Ok(r) => r,
        Err(_) => return,
    };
    for e in rd.flatten() {
        let p = e.path();
        let name = e.file_name().to_string_lossy().into_owned();
        if p.is_dir() {
            if !["tests", "benches", "archive", "target"].contains(&name.as_str()) {
                rust_files(&p, out);
            }
        } else if name.ends_with(".rs") {
            out.push(p);
        }
    }
}

#[test]
fn panic_sites_in_production_code_never_increase() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let budget_text = std::fs::read_to_string(root.join("tests").join("panic_budget.txt")).expect("tests/panic_budget.txt");
    let mut budget: BTreeMap<String, usize> = BTreeMap::new();
    let mut total_budget = usize::MAX;
    for line in budget_text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (name, num) = line.rsplit_once(' ').expect("'path count'");
        let num: usize = num.trim().parse().expect("count");
        if name == "TOTAL" {
            total_budget = num;
        } else {
            budget.insert(name.to_string(), num);
        }
    }

    let mut files = Vec::new();
    rust_files(&root.join("src"), &mut files);
    let mut over = Vec::new();
    let mut total = 0;
    for f in files {
        let rel = f.strip_prefix(root).unwrap().to_string_lossy().replace('\\', "/");
        let raw = std::fs::read_to_string(&f).unwrap_or_default();
        let n = count_sites(&strip_test_items(&sanitize(&raw)));
        total += n;
        let allowed = budget.get(&rel).copied().unwrap_or(0);
        if n > allowed {
            over.push(format!("{rel}: {n} sites, budget {allowed}"));
        }
    }
    assert!(
        over.is_empty(),
        "new unwrap/expect/panic sites in non-test code (use Result and `?`):\n{}",
        over.join("\n")
    );
    assert!(total <= total_budget, "total {total} exceeds budget {total_budget}");
    // guard against a broken counter silently passing: it must agree with the recorded budget
    assert!(
        total * 100 >= total_budget * 95,
        "counter found only {total} sites but the budget records {total_budget}: counter or budget file is stale"
    );
}

#[test]
fn no_lock_unwraps_in_production_code() {
    // a poisoned Mutex/RwLock must not cascade a panic into every other thread
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut files = Vec::new();
    rust_files(&root.join("src"), &mut files);
    let mut bad = Vec::new();
    for f in files {
        let raw = std::fs::read_to_string(&f).unwrap_or_default();
        let text = strip_test_items(&sanitize(&raw));
        for (i, line) in text.lines().enumerate() {
            let l: String = line.chars().filter(|c| !c.is_whitespace()).collect();
            if l.contains(".lock().unwrap()") {
                bad.push(format!("{}:{}", f.strip_prefix(root).unwrap().display(), i + 1));
            }
        }
    }
    assert!(bad.is_empty(), "lock().unwrap() in non-test code:\n{}", bad.join("\n"));
}
