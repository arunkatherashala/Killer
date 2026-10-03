//! Source maps: which original file and line produced each line of the text being compiled.
//!
//! Killer rewrites its source several times before the real compiler sees it (imports are
//! inlined, `match`/`switch`/`do-while`/C-style `for` are lowered, lambdas are lifted, sugar is
//! expanded, offside blocks get braces). Those passes insert and remove lines, so a line number
//! in the compiler's input says nothing about the user's file.
//!
//! Every pass therefore also returns a *line map*: for each output line, the index (0-based) of
//! the input line it came from (`Vec<usize>`). [`SourceMap::compose`] chains those onto the map
//! built by the import pass, which knows the real `(file, line)` of every line. In the end the
//! compiler looks up `(file, line)` for the line number it reports in an error or records in the
//! program's line table.
//!
//! Line-map convention: entry `j` describes the `j`-th piece of `text.split('\n')`. Lookups are
//! clamped, so a stray off-by-one at the very end of a text can only blur the last line.

/// An original position. `line` is 1-based, `0` means unknown.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Loc {
    pub file: u32,
    pub line: u32,
}

/// Per-line origin of a (rewritten) text.
#[derive(Clone, Debug, Default)]
pub struct SourceMap {
    pub files: Vec<String>,
    pub locs: Vec<Loc>,
    /// Original text of each file (parallel to `files`; may be empty), for diagnostics.
    pub sources: std::rc::Rc<Vec<String>>,
}

/// Number of lines of `text` in the convention above.
pub fn line_count(text: &str) -> usize {
    text.bytes().filter(|&b| b == b'\n').count() + 1
}

impl SourceMap {
    /// A map for a text that is exactly the file `name`: line `n` came from `name:n`.
    pub fn identity(name: &str, text: &str) -> SourceMap {
        SourceMap {
            files: vec![name.to_string()],
            locs: (0..line_count(text)).map(|i| Loc { file: 0, line: i as u32 + 1 }).collect(),
            sources: std::rc::Rc::new(vec![text.to_string()]),
        }
    }

    /// The original text of the line `loc` points at.
    pub fn source_line(&self, loc: Loc) -> Option<&str> {
        self.sources.get(loc.file as usize)?.split('\n').nth((loc.line as usize).checked_sub(1)?)
    }

    /// Map for the output of a pass, given the pass's own line map (output line -> input line
    /// index) and the map of the pass's input.
    pub fn compose(&self, pass: &[usize]) -> SourceMap {
        let last = self.locs.len().saturating_sub(1);
        SourceMap {
            files: self.files.clone(),
            sources: self.sources.clone(),
            locs: pass
                .iter()
                .map(|&i| self.locs.get(i.min(last)).copied().unwrap_or(Loc { file: 0, line: 0 }))
                .collect(),
        }
    }

    /// Origin of the 1-based line `line` of the current text.
    pub fn loc(&self, line: usize) -> Option<Loc> {
        if line == 0 || self.locs.is_empty() {
            return None;
        }
        let l = self.locs[(line - 1).min(self.locs.len() - 1)];
        if l.line == 0 {
            None
        } else {
            Some(l)
        }
    }

    pub fn file_name(&self, file: u32) -> &str {
        self.files.get(file as usize).map(String::as_str).unwrap_or("<unknown>")
    }

    /// `(file name, original line)` of the 1-based line `line` of the current text.
    pub fn resolve(&self, line: usize) -> Option<(&str, u32)> {
        self.loc(line).map(|l| (self.file_name(l.file), l.line))
    }
}

/// Rewrite the line numbers in a compiler message (`Line 12: ...`, `(line 12)`, `on line 12`)
/// from the compiler's text to the original files. Lines in other files get ` of <file>` added.
/// Text inside backticks (echoed source) is left alone.
pub fn remap_message(msg: &str, map: &SourceMap) -> String {
    if map.locs.is_empty() {
        return msg.to_string();
    }
    let bytes = msg.as_bytes();
    let mut out = String::with_capacity(msg.len() + 16);
    let mut i = 0;
    let mut in_tick = false;
    while i < bytes.len() {
        let b = bytes[i];
        if b == b'`' {
            in_tick = !in_tick;
        }
        let is_kw = !in_tick
            && i + 5 <= bytes.len()
            && msg.is_char_boundary(i)
            && msg.is_char_boundary(i + 5)
            && msg[i..i + 5].eq_ignore_ascii_case("line ")
            && (i == 0 || !bytes[i - 1].is_ascii_alphanumeric());
        if is_kw {
            let mut j = i + 5;
            while j < bytes.len() && bytes[j].is_ascii_digit() {
                j += 1;
            }
            if j > i + 5 {
                if let Ok(n) = msg[i + 5..j].parse::<usize>() {
                    if let Some(l) = map.loc(n) {
                        out.push_str(&msg[i..i + 5]);
                        out.push_str(&l.line.to_string());
                        if l.file != 0 {
                            out.push_str(" of ");
                            out.push_str(map.file_name(l.file));
                        }
                        i = j;
                        continue;
                    }
                }
            }
        }
        let ch_len = msg[i..].chars().next().map_or(1, char::len_utf8);
        out.push_str(&msg[i..i + ch_len]);
        i += ch_len;
    }
    out
}

/// Builds the output of a text rewrite together with its line map.
///
/// Used by passes that replace *regions* of the source (a `match` statement becomes an `if`
/// chain): text outside replaced regions keeps its own lines; lines of a replacement are mapped
/// by looking for them in the replaced region and otherwise fall back to the region's first line.
pub struct Rewriter<'a> {
    src: &'a str,
    line_starts: Vec<usize>,
    pub out: String,
    pub map: Vec<usize>,
}

impl<'a> Rewriter<'a> {
    pub fn new(src: &'a str) -> Self {
        let mut line_starts = vec![0usize];
        for (i, b) in src.bytes().enumerate() {
            if b == b'\n' {
                line_starts.push(i + 1);
            }
        }
        Rewriter { src, line_starts, out: String::with_capacity(src.len() + 64), map: vec![0] }
    }

    /// 0-based line index of the byte offset `pos` of the source.
    pub fn line_of(&self, pos: usize) -> usize {
        match self.line_starts.binary_search(&pos) {
            Ok(i) => i,
            Err(i) => i - 1,
        }
    }

    /// Append `src[from..to]` unchanged.
    pub fn copy(&mut self, from: usize, to: usize) {
        if from >= to {
            return;
        }
        let first = self.line_of(from);
        let seg = &self.src[from..to];
        self.out.push_str(seg);
        let mut line = first;
        for b in seg.bytes() {
            if b == b'\n' {
                line += 1;
                self.map.push(line);
            }
        }
    }

    /// Append `replacement` in place of `src[from..to]`.
    pub fn replace(&mut self, from: usize, to: usize, replacement: &str) {
        let first = self.line_of(from);
        let last = self.line_of(to.saturating_sub(1).max(from));
        let region: Vec<&str> = self.src[from..to].split('\n').collect();
        let mut cursor = 0usize;
        let mut current = first;
        self.out.push_str(replacement);
        for (k, piece) in replacement.split('\n').enumerate() {
            if k == 0 {
                continue; // continues the line already being built
            }
            let t = piece.trim();
            if t.len() >= 3 && t.bytes().any(|b| b.is_ascii_alphanumeric()) {
                if let Some(off) = region[cursor..].iter().position(|r| r.contains(t)) {
                    cursor += off;
                    current = first + cursor;
                }
            }
            self.map.push(current.min(last));
        }
    }

    pub fn finish(self) -> (String, Vec<usize>) {
        (self.out, self.map)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_map_counts_lines() {
        let m = SourceMap::identity("a.killer", "x\ny\n");
        assert_eq!(m.locs.len(), 3);
        assert_eq!(m.resolve(2), Some(("a.killer", 2)));
        assert_eq!(m.resolve(0), None);
    }

    #[test]
    fn compose_chains_passes() {
        let base = SourceMap::identity("a.killer", "l1\nl2\nl3\nl4");
        // pass 1 inserts a line before l3: out = l1 l2 NEW l3 l4
        let p1 = base.compose(&[0, 1, 2, 2, 3]);
        assert_eq!(p1.resolve(3), Some(("a.killer", 3)));
        assert_eq!(p1.resolve(5), Some(("a.killer", 4)));
        // pass 2 drops the first line: out = l2 NEW l3 l4 (indices into p1's output)
        let p2 = p1.compose(&[1, 2, 3, 4]);
        assert_eq!(p2.resolve(1), Some(("a.killer", 2)));
        assert_eq!(p2.resolve(4), Some(("a.killer", 4)));
        // out-of-range lookups clamp to the last line
        assert_eq!(p2.resolve(99), Some(("a.killer", 4)));
    }

    #[test]
    fn imported_lines_carry_their_file() {
        let m = SourceMap {
            sources: Default::default(),
            files: vec!["main.killer".into(), "lib.killer".into()],
            locs: vec![Loc { file: 1, line: 1 }, Loc { file: 1, line: 2 }, Loc { file: 0, line: 2 }],
        };
        let c = m.compose(&[2, 1]);
        assert_eq!(c.resolve(1), Some(("main.killer", 2)));
        assert_eq!(c.resolve(2), Some(("lib.killer", 2)));
    }

    #[test]
    fn remap_message_rewrites_line_numbers_outside_backticks() {
        let m = SourceMap {
            sources: Default::default(),
            files: vec!["main.killer".into(), "lib.killer".into()],
            locs: vec![Loc { file: 0, line: 1 }, Loc { file: 1, line: 7 }, Loc { file: 0, line: 2 }],
        };
        assert_eq!(remap_message("Line 2: bad", &m), "Line 7 of lib.killer: bad");
        assert_eq!(remap_message("Line 3: x", &m), "Line 2: x");
        assert_eq!(remap_message("type error (line 2): y", &m), "type error (line 7 of lib.killer): y");
        assert_eq!(remap_message("Line 1: unsupported `see line 2`", &m), "Line 1: unsupported `see line 2`");
        assert_eq!(remap_message("found `x` on line 3", &m), "found `x` on line 2");
    }

    #[test]
    fn rewriter_maps_copied_and_replaced_lines() {
        let src = "a\nmatch x {\n  1 => foo()\n  2 => bar()\n}\nz\n";
        let start = src.find("match").unwrap();
        let end = src.find("}\nz").unwrap() + 1;
        let mut rw = Rewriter::new(src);
        rw.copy(0, start);
        rw.replace(start, end, "if x == 1 {\n  foo()\n} else {\n  bar()\n}");
        rw.copy(end, src.len());
        let (out, map) = rw.finish();
        assert_eq!(out, "a\nif x == 1 {\n  foo()\n} else {\n  bar()\n}\nz\n");
        assert_eq!(map.len(), line_count(&out));
        assert_eq!(map[0], 0);
        assert_eq!(map[1], 1); // if
        assert_eq!(map[2], 2); // foo() is on source line 3 (index 2)
        assert_eq!(map[4], 3); // bar() is on source line 4 (index 3)
        assert_eq!(map[6], 5); // z
    }
}
