//! Incremental Server-Sent Events (SSE) and NDJSON parsers, plus the SSE writer helpers used by
//! the HTTP server.
//!
//! [`SseParser`] accepts bytes in arbitrary chunks (a chunk boundary may fall anywhere: inside a
//! line, inside a multi-byte UTF-8 character, between the `\r` and `\n` of a CRLF) and returns
//! each event as soon as its terminating blank line has arrived. Rules (WHATWG SSE):
//! lines end with `\n`, `\r\n` or a lone `\r`; a line starting with `:` is a comment (keep-alive);
//! `field: value` strips a single leading space; several `data:` lines are joined with `\n`;
//! `event:` names the event; `id:` and `retry:` are recorded; unknown fields are ignored; a
//! leading UTF-8 BOM is skipped; an event without any `data:` line is not dispatched.
//! [`SseParser::finish`] flushes a final event whose blank line never arrived (lenient: the spec
//! would drop it, but a provider closing the socket early should not lose the last delta).

/// One dispatched event.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SseEvent {
    /// The `event:` name, `None` for the default ("message") event.
    pub event: Option<String>,
    /// The `data:` lines joined with `\n`.
    pub data: String,
    pub id: Option<String>,
    pub retry: Option<u64>,
}

impl SseEvent {
    pub fn new(data: &str) -> Self {
        Self { data: data.to_string(), ..Default::default() }
    }

    /// Wire format of this event.
    pub fn format(&self) -> String {
        encode_event(self.event.as_deref(), &self.data)
    }
}

#[derive(Default)]
pub struct SseParser {
    buf: Vec<u8>,
    /// the previous chunk ended in `\r`: a leading `\n` of the next chunk belongs to it
    skip_lf: bool,
    started: bool,
    event: Option<String>,
    data: Vec<String>,
    id: Option<String>,
    retry: Option<u64>,
}

impl SseParser {
    pub fn new() -> Self {
        Self::default()
    }

    /// Feed more bytes; returns the events completed by them.
    pub fn feed(&mut self, bytes: &[u8]) -> Vec<SseEvent> {
        let mut out = Vec::new();
        let mut bytes = bytes;
        if self.skip_lf && !bytes.is_empty() {
            if bytes[0] == b'\n' {
                bytes = &bytes[1..];
            }
            self.skip_lf = false;
        }
        self.buf.extend_from_slice(bytes);
        let mut start = 0usize;
        let mut i = 0usize;
        while i < self.buf.len() {
            let b = self.buf[i];
            if b == b'\n' || b == b'\r' {
                let line = self.buf[start..i].to_vec();
                if b == b'\r' {
                    if i + 1 < self.buf.len() {
                        if self.buf[i + 1] == b'\n' {
                            i += 1;
                        }
                    } else {
                        self.skip_lf = true;
                    }
                }
                i += 1;
                start = i;
                self.line(&line, &mut out);
            } else {
                i += 1;
            }
        }
        self.buf.drain(..start);
        out
    }

    /// End of stream: dispatch a pending, unterminated event (and a final unterminated line).
    pub fn finish(&mut self) -> Vec<SseEvent> {
        let mut out = Vec::new();
        if !self.buf.is_empty() {
            let line = std::mem::take(&mut self.buf);
            self.line(&line, &mut out);
        }
        self.dispatch(&mut out);
        out
    }

    fn line(&mut self, raw: &[u8], out: &mut Vec<SseEvent>) {
        let mut raw = raw;
        if !self.started {
            self.started = true;
            if raw.starts_with(&[0xEF, 0xBB, 0xBF]) {
                raw = &raw[3..];
            }
        }
        if raw.is_empty() {
            self.dispatch(out);
            return;
        }
        if raw[0] == b':' {
            return; // comment / keep-alive
        }
        let text = String::from_utf8_lossy(raw);
        let (field, value) = match text.find(':') {
            Some(p) => {
                let v = &text[p + 1..];
                (&text[..p], v.strip_prefix(' ').unwrap_or(v))
            }
            None => (&text[..], ""),
        };
        match field {
            "data" => self.data.push(value.to_string()),
            "event" => self.event = Some(value.to_string()),
            "id" => {
                if !value.contains('\0') {
                    self.id = Some(value.to_string());
                }
            }
            "retry" => {
                if let Ok(n) = value.parse::<u64>() {
                    self.retry = Some(n);
                }
            }
            _ => {}
        }
    }

    fn dispatch(&mut self, out: &mut Vec<SseEvent>) {
        let event = self.event.take();
        let retry = self.retry.take();
        if self.data.is_empty() {
            return;
        }
        let data = self.data.join("\n");
        self.data.clear();
        out.push(SseEvent { event, data, id: self.id.clone(), retry });
    }
}

/// Splits a byte stream into non-empty lines (NDJSON), tolerant of chunk boundaries and CRLF.
#[derive(Default)]
pub struct LineParser {
    buf: Vec<u8>,
}

impl LineParser {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn feed(&mut self, bytes: &[u8]) -> Vec<String> {
        self.buf.extend_from_slice(bytes);
        let mut out = Vec::new();
        while let Some(p) = self.buf.iter().position(|b| *b == b'\n') {
            let line: Vec<u8> = self.buf.drain(..=p).collect();
            let s = String::from_utf8_lossy(&line).trim().to_string();
            if !s.is_empty() {
                out.push(s);
            }
        }
        out
    }

    pub fn finish(&mut self) -> Vec<String> {
        let s = String::from_utf8_lossy(&std::mem::take(&mut self.buf)).trim().to_string();
        if s.is_empty() { vec![] } else { vec![s] }
    }
}

/// Encode one event for the wire. Newlines in `data` become several `data:` lines; `\r` is
/// normalised so a payload can never inject fields.
pub fn encode_event(event: Option<&str>, data: &str) -> String {
    let mut s = String::new();
    if let Some(e) = event {
        let e: String = e.chars().filter(|c| *c != '\r' && *c != '\n').collect();
        s.push_str("event: ");
        s.push_str(&e);
        s.push('\n');
    }
    let norm = data.replace("\r\n", "\n").replace('\r', "\n");
    for l in norm.split('\n') {
        s.push_str("data: ");
        s.push_str(l);
        s.push('\n');
    }
    s.push('\n');
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    fn all(chunks: &[&[u8]]) -> Vec<SseEvent> {
        let mut p = SseParser::new();
        let mut v = Vec::new();
        for c in chunks {
            v.extend(p.feed(c));
        }
        v.extend(p.finish());
        v
    }

    #[test]
    fn basic_event() {
        let v = all(&[b"data: hello\n\n"]);
        assert_eq!(v, vec![SseEvent { data: "hello".into(), ..Default::default() }]);
    }

    #[test]
    fn multi_line_data_is_joined() {
        let v = all(&[b"data: a\ndata: b\ndata:\ndata: c\n\n"]);
        assert_eq!(v[0].data, "a\nb\n\nc");
    }

    #[test]
    fn event_name_id_retry_and_reset() {
        let v = all(&[b"event: x\nid: 7\nretry: 500\ndata: 1\n\ndata: 2\n\n"]);
        assert_eq!(v[0].event.as_deref(), Some("x"));
        assert_eq!(v[0].id.as_deref(), Some("7"));
        assert_eq!(v[0].retry, Some(500));
        assert_eq!(v[1].event, None);
        assert_eq!(v[1].id.as_deref(), Some("7")); // id persists
        assert_eq!(v[1].retry, None);
    }

    #[test]
    fn comments_and_keepalives_are_ignored() {
        let v = all(&[b": keep-alive\n\n:ping\ndata: x\n: mid\n\n"]);
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].data, "x");
    }

    #[test]
    fn crlf_cr_and_lf_terminators() {
        let v = all(&[b"data: a\r\n\r\ndata: b\r\rdata: c\n\n"]);
        let d: Vec<&str> = v.iter().map(|e| e.data.as_str()).collect();
        assert_eq!(d, vec!["a", "b", "c"]);
    }

    #[test]
    fn crlf_split_between_chunks() {
        let v = all(&[b"data: a\r", b"\n\r", b"\ndata: b\n\n"]);
        let d: Vec<&str> = v.iter().map(|e| e.data.as_str()).collect();
        assert_eq!(d, vec!["a", "b"]);
    }

    #[test]
    fn every_split_point_gives_the_same_events() {
        let src = "event: m\r\ndata: caf\u{e9} \u{1F600}\r\ndata: line2\r\n\r\n: ka\r\n\r\ndata: [DONE]\r\n\r\n".as_bytes();
        let whole = all(&[src]);
        assert_eq!(whole.len(), 2);
        for i in 0..=src.len() {
            assert_eq!(all(&[&src[..i], &src[i..]]), whole, "split at {}", i);
        }
        // byte at a time
        let bytes: Vec<&[u8]> = src.chunks(1).collect();
        assert_eq!(all(&bytes), whole);
    }

    #[test]
    fn field_without_colon_and_no_space() {
        let v = all(&[b"data\n\ndata:x\n\ndata:  y\n\n"]);
        assert_eq!(v[0].data, "");
        assert_eq!(v[1].data, "x");
        assert_eq!(v[2].data, " y");
    }

    #[test]
    fn bom_is_skipped_and_unknown_fields_ignored() {
        let v = all(&[b"\xEF\xBB\xBFfoo: bar\ndata: ok\n\n"]);
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].data, "ok");
    }

    #[test]
    fn event_without_data_is_not_dispatched() {
        assert!(all(&[b"event: x\n\n"]).is_empty());
    }

    #[test]
    fn unterminated_final_event_is_flushed_by_finish() {
        let v = all(&[b"data: tail"]);
        assert_eq!(v[0].data, "tail");
    }

    #[test]
    fn partial_event_waits_for_the_blank_line() {
        let mut p = SseParser::new();
        assert!(p.feed(b"data: par").is_empty());
        assert!(p.feed(b"tial\n").is_empty());
        let v = p.feed(b"\n");
        assert_eq!(v[0].data, "partial");
    }

    #[test]
    fn line_parser_handles_boundaries_and_crlf() {
        let mut p = LineParser::new();
        assert_eq!(p.feed(b"{\"a\":1}\r\n{\"b\""), vec!["{\"a\":1}".to_string()]);
        assert_eq!(p.feed(b":2}\n\n\n{\"c\":3}"), vec!["{\"b\":2}".to_string()]);
        assert_eq!(p.finish(), vec!["{\"c\":3}".to_string()]);
    }

    #[test]
    fn encode_event_round_trips_through_the_parser() {
        let wire = format!("{}{}", encode_event(None, "a\nb"), encode_event(Some("done"), "x\r\ny"));
        let v = all(&[wire.as_bytes()]);
        assert_eq!(v[0].data, "a\nb");
        assert_eq!(v[1].event.as_deref(), Some("done"));
        assert_eq!(v[1].data, "x\ny");
    }
}
