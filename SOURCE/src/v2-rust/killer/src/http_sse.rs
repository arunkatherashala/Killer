//! Server-Sent Events replies for `serve` handlers.
//!
//! A handler may return `{stream: generator_or_array, status?, headers?}` (or a bare generator).
//! The server writes `200 text/event-stream` headers without a Content-Length, then one SSE event
//! per item, flushing after each, and closes the connection when the stream ends. Items:
//! a string is sent as `data: <text>`; a dict with only the keys `event`, `data`, `id` becomes a
//! named event; any other dict/array is sent as JSON; other values as their text. The generator
//! runs lazily on the VM thread between writes, so a slow producer streams in real time.
//! An error raised by the generator is reported as an `event: error` and ends the stream; a
//! client that disconnects (write error) closes the generator; `stream_timeout` (serve option,
//! seconds, default 300) bounds the whole stream.

use std::io::Write;
use std::net::TcpStream;
use std::time::{Duration, Instant};

use crate::bytecode::Program;
use crate::value::Value;
use crate::vm::VirtualMachine;

pub struct StreamReply {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub source: Value,
}

/// Recognise a streaming reply value.
pub fn stream_reply(v: &Value) -> Option<StreamReply> {
    match v {
        Value::Generator(_) => Some(StreamReply { status: 200, headers: vec![], source: v.clone() }),
        Value::Dict(d) => {
            let src = d.get("stream")?;
            if !matches!(src, Value::Generator(_) | Value::Array(_)) || !d.keys().iter().all(|k| k == "stream" || k == "status" || k == "headers") {
                return None;
            }
            let status = match d.get("status") {
                Some(Value::Number(n)) if (200.0..600.0).contains(&n) => n as u16,
                Some(Value::Integer(n)) if (200..600).contains(&n) => n as u16,
                _ => 200,
            };
            let mut headers = Vec::new();
            if let Some(Value::Dict(h)) = d.get("headers") {
                for (k, val) in h.iter() {
                    headers.push((k, match val { Value::Str(s) => s, o => format!("{}", o) }));
                }
            }
            Some(StreamReply { status, headers, source: src })
        }
        _ => None,
    }
}

/// The response head for a stream.
pub fn head_bytes(r: &StreamReply) -> Vec<u8> {
    let mut out = format!("HTTP/1.1 {} {}\r\n", r.status, crate::http_serve::status_text(r.status));
    let mut has_ct = false;
    for (k, v) in &r.headers {
        let k: String = k.chars().filter(|c| *c != '\r' && *c != '\n' && *c != ':').collect();
        let v: String = v.chars().filter(|c| *c != '\r' && *c != '\n').collect();
        let lk = k.to_ascii_lowercase();
        if lk == "content-length" || lk == "transfer-encoding" || lk == "connection" {
            continue;
        }
        has_ct |= lk == "content-type";
        out.push_str(&format!("{}: {}\r\n", k, v));
    }
    if !has_ct {
        out.push_str("Content-Type: text/event-stream; charset=utf-8\r\n");
    }
    out.push_str("Cache-Control: no-cache\r\nX-Accel-Buffering: no\r\nConnection: close\r\n\r\n");
    out.into_bytes()
}

fn show(v: &Value) -> String {
    match v {
        Value::Str(s) => s.clone(),
        Value::Dict(_) | Value::Array(_) => crate::json_lang::stringify(v, 0).unwrap_or_else(|_| format!("{}", v)),
        o => format!("{}", o),
    }
}

/// Wire text of one stream item.
pub fn item_event(v: &Value) -> String {
    if let Value::Dict(d) = v {
        let keys = d.keys();
        if keys.iter().any(|k| k == "data") && keys.iter().all(|k| k == "event" || k == "data" || k == "id") {
            let ev = match d.get("event") {
                Some(Value::Str(e)) => Some(e),
                _ => None,
            };
            let mut s = String::new();
            if let Some(Value::Str(id)) = d.get("id") {
                let id: String = id.chars().filter(|c| *c != '\r' && *c != '\n').collect();
                s.push_str(&format!("id: {}\n", id));
            }
            s.push_str(&crate::sse::encode_event(ev.as_deref(), &show(&d.get("data").unwrap_or(Value::Null))));
            return s;
        }
    }
    crate::sse::encode_event(None, &show(v))
}

/// Write the whole stream to `conn`. Returns false when the client went away.
pub fn write_stream(conn: &mut TcpStream, vm: &mut VirtualMachine, program: &Program, reply: &StreamReply, limit: Duration) -> bool {
    if conn.write_all(&head_bytes(reply)).is_err() || conn.flush().is_err() {
        return false;
    }
    let started = Instant::now();
    let mut array_items: std::vec::IntoIter<Value> = match &reply.source {
        Value::Array(a) => a.to_vec().into_iter(),
        _ => Vec::new().into_iter(),
    };
    loop {
        if started.elapsed() > limit {
            let _ = conn.write_all(crate::sse::encode_event(Some("error"), "stream timeout").as_bytes());
            break;
        }
        let item = match &reply.source {
            Value::Generator(id) => match vm.gen_next(id, program) {
                Ok(Some(v)) => v,
                Ok(None) => break,
                Err(e) => {
                    let _ = conn.write_all(crate::sse::encode_event(Some("error"), &format!("{}", e)).as_bytes());
                    break;
                }
            },
            _ => match array_items.next() {
                Some(v) => v,
                None => break,
            },
        };
        if conn.write_all(item_event(&item).as_bytes()).is_err() || conn.flush().is_err() {
            if let Value::Generator(id) = &reply.source {
                vm.close_generator(id);
            }
            return false;
        }
    }
    let _ = conn.flush();
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::value::SharedDict;

    fn d(p: Vec<(&str, Value)>) -> Value {
        Value::Dict(SharedDict::from_pairs(p.into_iter().map(|(k, v)| (k.to_string(), v)).collect()))
    }

    #[test]
    fn recognises_stream_replies_only() {
        let arr = Value::Array(crate::value::SharedArray::new(vec![Value::Str("a".into())]));
        assert!(stream_reply(&d(vec![("stream", arr.clone())])).is_some());
        assert!(stream_reply(&d(vec![("stream", arr.clone()), ("body", Value::Null)])).is_none());
        assert!(stream_reply(&d(vec![("stream", Value::Str("x".into()))])).is_none());
        assert!(stream_reply(&Value::Str("x".into())).is_none());
    }

    #[test]
    fn items_become_events() {
        assert_eq!(item_event(&Value::Str("hi".into())), "data: hi\n\n");
        assert_eq!(item_event(&Value::Str("a\nb".into())), "data: a\ndata: b\n\n");
        assert_eq!(item_event(&d(vec![("event", Value::Str("tick".into())), ("data", Value::Number(5.0))])), "event: tick\ndata: 5\n\n");
        assert_eq!(item_event(&d(vec![("x", Value::Number(1.0))])), "data: {\"x\":1}\n\n");
    }

    #[test]
    fn head_has_event_stream_type_and_no_length() {
        let r = StreamReply { status: 200, headers: vec![("Content-Length".into(), "5".into()), ("X-A".into(), "b\r\nEvil: 1".into())], source: Value::Null };
        let h = String::from_utf8(head_bytes(&r)).unwrap();
        assert!(h.starts_with("HTTP/1.1 200 OK\r\n"));
        assert!(h.contains("Content-Type: text/event-stream"));
        assert!(!h.to_ascii_lowercase().contains("content-length"));
        assert!(h.contains("X-A: bEvil: 1\r\n"));
        assert!(h.ends_with("\r\n\r\n"));
    }
}
