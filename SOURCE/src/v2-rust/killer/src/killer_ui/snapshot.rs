//! Dependency-free **JSON** snapshots of headless UI state (web lane, HTTP panels, tests).
//! No `serde` — keeps default `killer-native` lean.

use super::runtime_headless::HeadlessFrame;
use super::patch::UiEvent;

fn json_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 8);
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c < ' ' => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

fn ui_event_json(e: &UiEvent) -> String {
    match e {
        UiEvent::ButtonClicked(id) => {
            format!(r#"{{"ButtonClicked":"{}"}}"#, json_escape(id))
        }
        UiEvent::SliderChanged { id, value } => {
            format!(
                r#"{{"SliderChanged":{{"id":"{}","value":{}}}}}"#,
                json_escape(id),
                value
            )
        }
        UiEvent::ToggleChanged { id, on } => {
            format!(
                r#"{{"ToggleChanged":{{"id":"{}","on":{}}}}}"#,
                json_escape(id),
                if *on { "true" } else { "false" }
            )
        }        other => {
            format!(r#"{{"UiEvent":"{:?}"}}"#, other)
        }    }
}

/// JSON object: `killer_ui_engine_version`, `cooked`, `events`, `events_pending`.
pub fn headless_frame_json(engine_version: u32, frame: &HeadlessFrame) -> String {
    let mut keys: Vec<&String> = frame.cooked_floats.keys().collect();
    keys.sort();
    let cooked_inner = keys
        .into_iter()
        .map(|k| {
            let v = frame.cooked_floats[k];
            format!(r#""{}":{}"#, json_escape(k), v)
        })
        .collect::<Vec<_>>()
        .join(",");

    let events_inner = frame
        .pending_events
        .iter()
        .map(ui_event_json)
        .collect::<Vec<_>>()
        .join(",");

    format!(
        r#"{{"killer_ui_engine_version":{},"cooked":{{{}}},"events":[{}],"events_pending":{}}}"#,
        engine_version,
        cooked_inner,
        events_inner,
        frame.pending_events.len()
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn headless_json_has_cooked_and_version() {
        let frame = HeadlessFrame {
            cooked_floats: HashMap::from([("sum".into(), 3.0)]),
            pending_events: vec![UiEvent::ButtonClicked("btn".into())],
        };
        let j = headless_frame_json(1, &frame);
        assert!(j.contains("\"killer_ui_engine_version\":1"));
        assert!(j.contains("\"sum\":3"));
        assert!(j.contains("ButtonClicked"));
        assert!(j.contains("\"events_pending\":1"));
    }
}
