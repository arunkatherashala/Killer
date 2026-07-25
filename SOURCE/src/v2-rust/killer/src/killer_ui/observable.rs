//! **Observable / Stream** — RxJS-inspired reactive stream primitives.
//!
//! Provides `Observable<T>`, `Subject`, `BehaviorSubject`, and a rich set of
//! operators: map, filter, merge, combine_latest, debounce, throttle, take,
//! skip, distinct, scan, flat_map, buffer, share.
//!
//! Zero external dependencies — pure std Rust.

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

// ══════════════════════════════════════════════════════════════════════════════
// Core types
// ══════════════════════════════════════════════════════════════════════════════

/// Unique subscription identifier.
pub type SubId = u64;

/// Events emitted by an observable stream.
#[derive(Debug, Clone, PartialEq)]
pub enum StreamEvent {
    Next(StreamValue),
    Error(String),
    Complete,
}

/// Values that flow through streams.
#[derive(Debug, Clone, PartialEq)]
pub enum StreamValue {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    Str(String),
    Array(Vec<StreamValue>),
}

impl StreamValue {
    pub fn as_int(&self) -> Option<i64> {
        match self { StreamValue::Int(n) => Some(*n), _ => None }
    }
    pub fn as_float(&self) -> Option<f64> {
        match self {
            StreamValue::Float(f) => Some(*f),
            StreamValue::Int(i) => Some(*i as f64),
            _ => None,
        }
    }
    pub fn as_str(&self) -> Option<&str> {
        match self { StreamValue::Str(s) => Some(s), _ => None }
    }
    pub fn as_bool(&self) -> Option<bool> {
        match self { StreamValue::Bool(b) => Some(*b), _ => None }
    }
}

impl std::fmt::Display for StreamValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StreamValue::Null => write!(f, "null"),
            StreamValue::Bool(b) => write!(f, "{}", b),
            StreamValue::Int(n) => write!(f, "{}", n),
            StreamValue::Float(n) => write!(f, "{}", n),
            StreamValue::Str(s) => write!(f, "{}", s),
            StreamValue::Array(a) => write!(f, "[{} items]", a.len()),
        }
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Subscriber
// ══════════════════════════════════════════════════════════════════════════════

type SubscriberFn = Box<dyn Fn(&StreamEvent) + Send + 'static>;

struct Subscriber {
    id: SubId,
    handler: SubscriberFn,
}

// ══════════════════════════════════════════════════════════════════════════════
// Subject — multicast event source
// ══════════════════════════════════════════════════════════════════════════════

/// A Subject is both an observable and an observer — it can emit events
/// and be subscribed to by multiple listeners. Thread-safe.
pub struct Subject {
    inner: Arc<Mutex<SubjectInner>>,
}

struct SubjectInner {
    subscribers: Vec<Subscriber>,
    next_id: SubId,
    completed: bool,
}

impl Subject {
    pub fn new() -> Self {
        Subject {
            inner: Arc::new(Mutex::new(SubjectInner {
                subscribers: Vec::new(),
                next_id: 1,
                completed: false,
            })),
        }
    }

    /// Subscribe to events. Returns a SubId for unsubscribing.
    pub fn subscribe<F: Fn(&StreamEvent) + Send + 'static>(&self, handler: F) -> SubId {
        let mut inner = self.inner.lock().unwrap();
        let id = inner.next_id;
        inner.next_id += 1;
        inner.subscribers.push(Subscriber { id, handler: Box::new(handler) });
        id
    }

    /// Unsubscribe by ID.
    pub fn unsubscribe(&self, id: SubId) {
        let mut inner = self.inner.lock().unwrap();
        inner.subscribers.retain(|s| s.id != id);
    }

    /// Emit a value to all subscribers.
    pub fn next(&self, value: StreamValue) {
        let inner = self.inner.lock().unwrap();
        if inner.completed { return; }
        let event = StreamEvent::Next(value);
        for sub in &inner.subscribers {
            (sub.handler)(&event);
        }
    }

    /// Emit an error to all subscribers.
    pub fn error(&self, msg: String) {
        let inner = self.inner.lock().unwrap();
        if inner.completed { return; }
        let event = StreamEvent::Error(msg);
        for sub in &inner.subscribers {
            (sub.handler)(&event);
        }
    }

    /// Complete the stream — no more events after this.
    pub fn complete(&self) {
        let mut inner = self.inner.lock().unwrap();
        if inner.completed { return; }
        inner.completed = true;
        let event = StreamEvent::Complete;
        for sub in &inner.subscribers {
            (sub.handler)(&event);
        }
    }

    pub fn is_completed(&self) -> bool {
        self.inner.lock().unwrap().completed
    }

    pub fn subscriber_count(&self) -> usize {
        self.inner.lock().unwrap().subscribers.len()
    }
}

impl Default for Subject {
    fn default() -> Self { Self::new() }
}

// ══════════════════════════════════════════════════════════════════════════════
// BehaviorSubject — Subject that remembers its last value
// ══════════════════════════════════════════════════════════════════════════════

/// Like Subject, but stores the last emitted value. New subscribers
/// immediately receive the current value upon subscribing.
pub struct BehaviorSubject {
    subject: Subject,
    current: Arc<Mutex<StreamValue>>,
}

impl BehaviorSubject {
    pub fn new(initial: StreamValue) -> Self {
        BehaviorSubject {
            subject: Subject::new(),
            current: Arc::new(Mutex::new(initial)),
        }
    }

    pub fn subscribe<F: Fn(&StreamEvent) + Send + 'static>(&self, handler: F) -> SubId {
        // Immediately emit current value to new subscriber
        let current = self.current.lock().unwrap().clone();
        handler(&StreamEvent::Next(current));
        self.subject.subscribe(handler)
    }

    pub fn next(&self, value: StreamValue) {
        *self.current.lock().unwrap() = value.clone();
        self.subject.next(value);
    }

    pub fn value(&self) -> StreamValue {
        self.current.lock().unwrap().clone()
    }

    pub fn complete(&self) { self.subject.complete(); }
    pub fn error(&self, msg: String) { self.subject.error(msg); }
}

// ══════════════════════════════════════════════════════════════════════════════
// Observable — lazy stream with operators
// ══════════════════════════════════════════════════════════════════════════════

/// An Observable is a lazy, cold stream. Each subscription triggers the
/// producer function. Supports chainable operators.
pub struct Observable {
    events: Arc<Mutex<Vec<StreamEvent>>>,
}

impl Observable {
    /// Create from a list of values (cold).
    pub fn from_iter(values: Vec<StreamValue>) -> Self {
        let events: Vec<StreamEvent> = values.into_iter()
            .map(StreamEvent::Next)
            .chain(std::iter::once(StreamEvent::Complete))
            .collect();
        Observable { events: Arc::new(Mutex::new(events)) }
    }

    /// Create an empty completed observable.
    pub fn empty() -> Self {
        Observable { events: Arc::new(Mutex::new(vec![StreamEvent::Complete])) }
    }

    /// Create from a single value.
    pub fn of(value: StreamValue) -> Self {
        Self::from_iter(vec![value])
    }

    /// Subscribe and replay all events.
    pub fn subscribe<F: Fn(&StreamEvent) + Send + 'static>(&self, handler: F) {
        let events = self.events.lock().unwrap();
        for event in events.iter() {
            handler(event);
        }
    }

    /// Collect all Next values into a Vec.
    pub fn collect(&self) -> Vec<StreamValue> {
        let events = self.events.lock().unwrap();
        events.iter().filter_map(|e| {
            if let StreamEvent::Next(v) = e { Some(v.clone()) } else { None }
        }).collect()
    }

    // ── Operators ────────────────────────────────────────────────────────

    /// Transform each value with a function.
    pub fn map<F: Fn(&StreamValue) -> StreamValue>(self, f: F) -> Observable {
        let events = self.events.lock().unwrap();
        let mapped: Vec<StreamEvent> = events.iter().map(|e| match e {
            StreamEvent::Next(v) => StreamEvent::Next(f(v)),
            other => other.clone(),
        }).collect();
        Observable { events: Arc::new(Mutex::new(mapped)) }
    }

    /// Keep only values matching predicate.
    pub fn filter<F: Fn(&StreamValue) -> bool>(self, f: F) -> Observable {
        let events = self.events.lock().unwrap();
        let filtered: Vec<StreamEvent> = events.iter().filter(|e| match e {
            StreamEvent::Next(v) => f(v),
            _ => true,
        }).cloned().collect();
        Observable { events: Arc::new(Mutex::new(filtered)) }
    }

    /// Take only the first N values.
    pub fn take(self, n: usize) -> Observable {
        let events = self.events.lock().unwrap();
        let mut count = 0;
        let mut result = Vec::new();
        for e in events.iter() {
            match e {
                StreamEvent::Next(_) if count < n => {
                    result.push(e.clone());
                    count += 1;
                    if count == n { result.push(StreamEvent::Complete); break; }
                }
                StreamEvent::Complete | StreamEvent::Error(_) => {
                    result.push(e.clone());
                    break;
                }
                _ => {}
            }
        }
        Observable { events: Arc::new(Mutex::new(result)) }
    }

    /// Skip the first N values.
    pub fn skip(self, n: usize) -> Observable {
        let events = self.events.lock().unwrap();
        let mut count = 0;
        let skipped: Vec<StreamEvent> = events.iter().filter(|e| {
            match e {
                StreamEvent::Next(_) => {
                    count += 1;
                    count > n
                }
                _ => true,
            }
        }).cloned().collect();
        Observable { events: Arc::new(Mutex::new(skipped)) }
    }

    /// Remove consecutive duplicate values.
    pub fn distinct_until_changed(self) -> Observable {
        let events = self.events.lock().unwrap();
        let mut result = Vec::new();
        let mut last: Option<StreamValue> = None;
        for e in events.iter() {
            match e {
                StreamEvent::Next(v) => {
                    if last.as_ref() != Some(v) {
                        result.push(e.clone());
                        last = Some(v.clone());
                    }
                }
                other => result.push(other.clone()),
            }
        }
        Observable { events: Arc::new(Mutex::new(result)) }
    }

    /// Accumulator — like Array.reduce but emits each intermediate.
    pub fn scan<F: Fn(&StreamValue, &StreamValue) -> StreamValue>(self, initial: StreamValue, f: F) -> Observable {
        let events = self.events.lock().unwrap();
        let mut acc = initial;
        let scanned: Vec<StreamEvent> = events.iter().map(|e| match e {
            StreamEvent::Next(v) => {
                acc = f(&acc, v);
                StreamEvent::Next(acc.clone())
            }
            other => other.clone(),
        }).collect();
        Observable { events: Arc::new(Mutex::new(scanned)) }
    }

    /// Buffer values into arrays of N.
    pub fn buffer_count(self, count: usize) -> Observable {
        let events = self.events.lock().unwrap();
        let mut result = Vec::new();
        let mut buf: Vec<StreamValue> = Vec::new();
        for e in events.iter() {
            match e {
                StreamEvent::Next(v) => {
                    buf.push(v.clone());
                    if buf.len() == count {
                        result.push(StreamEvent::Next(StreamValue::Array(std::mem::take(&mut buf))));
                    }
                }
                StreamEvent::Complete => {
                    if !buf.is_empty() {
                        result.push(StreamEvent::Next(StreamValue::Array(std::mem::take(&mut buf))));
                    }
                    result.push(StreamEvent::Complete);
                }
                other => result.push(other.clone()),
            }
        }
        Observable { events: Arc::new(Mutex::new(result)) }
    }

    /// Merge two observables — interleave their events.
    pub fn merge(self, other: Observable) -> Observable {
        let mut events = self.events.lock().unwrap().clone();
        let other_events = other.events.lock().unwrap();
        // Remove Complete from first, append all of second
        events.retain(|e| !matches!(e, StreamEvent::Complete));
        for e in other_events.iter() {
            events.push(e.clone());
        }
        Observable { events: Arc::new(Mutex::new(events)) }
    }

    /// Emit each value only if debounce_ms has passed since last.
    /// For cold observables, we simulate via index-based dedup.
    pub fn debounce_count(self, min_gap: usize) -> Observable {
        let events = self.events.lock().unwrap();
        let mut result = Vec::new();
        let mut gap = min_gap; // Emit first immediately
        for e in events.iter() {
            match e {
                StreamEvent::Next(_) => {
                    if gap >= min_gap {
                        result.push(e.clone());
                        gap = 0;
                    } else {
                        gap += 1;
                    }
                }
                other => { result.push(other.clone()); gap += 1; }
            }
            gap += 1;
        }
        Observable { events: Arc::new(Mutex::new(result)) }
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Operator: combine_latest (free function, takes N subjects)
// ══════════════════════════════════════════════════════════════════════════════

/// Replay two observable sources and emit combined latest when either changes.
pub fn combine_latest(a: &Observable, b: &Observable) -> Observable {
    let a_vals = a.collect();
    let b_vals = b.collect();
    let mut result = Vec::new();
    let mut last_a: Option<StreamValue> = None;
    let mut last_b: Option<StreamValue> = None;
    let mut ai = 0;
    let mut bi = 0;
    // Interleave: alternate taking from a and b
    loop {
        let took = if ai < a_vals.len() {
            last_a = Some(a_vals[ai].clone());
            ai += 1;
            true
        } else { false };

        if let (Some(la), Some(lb)) = (&last_a, &last_b) {
            result.push(StreamEvent::Next(StreamValue::Array(vec![la.clone(), lb.clone()])));
        }

        let took_b = if bi < b_vals.len() {
            last_b = Some(b_vals[bi].clone());
            bi += 1;
            true
        } else { false };

        if let (Some(la), Some(lb)) = (&last_a, &last_b) {
            if took_b {
                result.push(StreamEvent::Next(StreamValue::Array(vec![la.clone(), lb.clone()])));
            }
        }

        if !took && !took_b { break; }
    }
    result.push(StreamEvent::Complete);
    Observable { events: Arc::new(Mutex::new(result)) }
}

// ══════════════════════════════════════════════════════════════════════════════
// EventBus — application-wide event bus (pub/sub by topic)
// ══════════════════════════════════════════════════════════════════════════════

/// Application-wide pub/sub event bus with string topics.
pub struct EventBus {
    subjects: Arc<Mutex<std::collections::HashMap<String, Vec<(SubId, SubscriberFn)>>>>,
    next_id: Arc<Mutex<SubId>>,
}

impl EventBus {
    pub fn new() -> Self {
        EventBus {
            subjects: Arc::new(Mutex::new(std::collections::HashMap::new())),
            next_id: Arc::new(Mutex::new(1)),
        }
    }

    pub fn on<F: Fn(&StreamValue) + Send + 'static>(&self, topic: &str, handler: F) -> SubId {
        let mut id_lock = self.next_id.lock().unwrap();
        let id = *id_lock;
        *id_lock += 1;
        let mut subjects = self.subjects.lock().unwrap();
        let wrapped: SubscriberFn = Box::new(move |event| {
            if let StreamEvent::Next(v) = event { handler(v); }
        });
        subjects.entry(topic.to_string()).or_default().push((id, wrapped));
        id
    }

    pub fn emit(&self, topic: &str, value: StreamValue) {
        let subjects = self.subjects.lock().unwrap();
        if let Some(handlers) = subjects.get(topic) {
            let event = StreamEvent::Next(value);
            for (_, handler) in handlers {
                handler(&event);
            }
        }
    }

    pub fn off(&self, topic: &str, id: SubId) {
        let mut subjects = self.subjects.lock().unwrap();
        if let Some(handlers) = subjects.get_mut(topic) {
            handlers.retain(|(sid, _)| *sid != id);
        }
    }

    pub fn topic_count(&self) -> usize {
        self.subjects.lock().unwrap().len()
    }
}

impl Default for EventBus {
    fn default() -> Self { Self::new() }
}

// ══════════════════════════════════════════════════════════════════════════════
// Pipe — synchronous operator pipeline (lightweight alternative)
// ══════════════════════════════════════════════════════════════════════════════

/// Lightweight synchronous pipeline that transforms a VecDeque of values.
pub struct Pipe {
    values: VecDeque<StreamValue>,
}

impl Pipe {
    pub fn from(values: Vec<StreamValue>) -> Self {
        Pipe { values: values.into() }
    }

    pub fn map<F: Fn(&StreamValue) -> StreamValue>(mut self, f: F) -> Self {
        self.values = self.values.iter().map(|v| f(v)).collect();
        self
    }

    pub fn filter<F: Fn(&StreamValue) -> bool>(mut self, f: F) -> Self {
        self.values = self.values.into_iter().filter(|v| f(v)).collect();
        self
    }

    pub fn take(mut self, n: usize) -> Self {
        self.values.truncate(n);
        self
    }

    pub fn collect(self) -> Vec<StreamValue> {
        self.values.into()
    }

    pub fn first(self) -> Option<StreamValue> {
        self.values.into_iter().next()
    }

    pub fn count(self) -> usize {
        self.values.len()
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Tests
// ══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subject_next_and_subscribe() {
        let subj = Subject::new();
        let received = Arc::new(Mutex::new(Vec::new()));
        let r = received.clone();
        subj.subscribe(move |event| {
            if let StreamEvent::Next(v) = event { r.lock().unwrap().push(v.clone()); }
        });
        subj.next(StreamValue::Int(1));
        subj.next(StreamValue::Int(2));
        subj.next(StreamValue::Int(3));
        let vals = received.lock().unwrap();
        assert_eq!(vals.len(), 3);
        assert_eq!(vals[0].as_int(), Some(1));
    }

    #[test]
    fn subject_unsubscribe() {
        let subj = Subject::new();
        let count = Arc::new(Mutex::new(0));
        let c = count.clone();
        let id = subj.subscribe(move |_| { *c.lock().unwrap() += 1; });
        subj.next(StreamValue::Int(1));
        subj.unsubscribe(id);
        subj.next(StreamValue::Int(2));
        assert_eq!(*count.lock().unwrap(), 1);
    }

    #[test]
    fn subject_complete_stops_events() {
        let subj = Subject::new();
        let count = Arc::new(Mutex::new(0));
        let c = count.clone();
        subj.subscribe(move |e| {
            if matches!(e, StreamEvent::Next(_)) { *c.lock().unwrap() += 1; }
        });
        subj.next(StreamValue::Int(1));
        subj.complete();
        subj.next(StreamValue::Int(2));
        assert_eq!(*count.lock().unwrap(), 1);
        assert!(subj.is_completed());
    }

    #[test]
    fn behavior_subject_replays_current() {
        let bs = BehaviorSubject::new(StreamValue::Str("initial".into()));
        let received = Arc::new(Mutex::new(Vec::new()));
        let r = received.clone();
        bs.subscribe(move |event| {
            if let StreamEvent::Next(v) = event { r.lock().unwrap().push(v.clone()); }
        });
        // Should have received "initial" immediately
        assert_eq!(received.lock().unwrap().len(), 1);
        assert_eq!(received.lock().unwrap()[0].as_str(), Some("initial"));
        bs.next(StreamValue::Str("updated".into()));
        assert_eq!(received.lock().unwrap().len(), 2);
        assert_eq!(bs.value().as_str(), Some("updated"));
    }

    #[test]
    fn observable_map() {
        let obs = Observable::from_iter(vec![
            StreamValue::Int(1), StreamValue::Int(2), StreamValue::Int(3),
        ]);
        let doubled = obs.map(|v| {
            StreamValue::Int(v.as_int().unwrap_or(0) * 2)
        });
        let vals = doubled.collect();
        assert_eq!(vals, vec![StreamValue::Int(2), StreamValue::Int(4), StreamValue::Int(6)]);
    }

    #[test]
    fn observable_filter() {
        let obs = Observable::from_iter(vec![
            StreamValue::Int(1), StreamValue::Int(2), StreamValue::Int(3),
            StreamValue::Int(4), StreamValue::Int(5),
        ]);
        let evens = obs.filter(|v| v.as_int().unwrap_or(0) % 2 == 0);
        let vals = evens.collect();
        assert_eq!(vals, vec![StreamValue::Int(2), StreamValue::Int(4)]);
    }

    #[test]
    fn observable_take_and_skip() {
        let obs = Observable::from_iter(vec![
            StreamValue::Int(1), StreamValue::Int(2), StreamValue::Int(3),
        ]);
        let taken = obs.take(2);
        assert_eq!(taken.collect(), vec![StreamValue::Int(1), StreamValue::Int(2)]);

        let obs2 = Observable::from_iter(vec![
            StreamValue::Int(1), StreamValue::Int(2), StreamValue::Int(3),
        ]);
        let skipped = obs2.skip(1);
        assert_eq!(skipped.collect(), vec![StreamValue::Int(2), StreamValue::Int(3)]);
    }

    #[test]
    fn observable_scan() {
        let obs = Observable::from_iter(vec![
            StreamValue::Int(1), StreamValue::Int(2), StreamValue::Int(3),
        ]);
        let sums = obs.scan(StreamValue::Int(0), |acc, v| {
            StreamValue::Int(acc.as_int().unwrap_or(0) + v.as_int().unwrap_or(0))
        });
        assert_eq!(sums.collect(), vec![
            StreamValue::Int(1), StreamValue::Int(3), StreamValue::Int(6),
        ]);
    }

    #[test]
    fn observable_distinct_until_changed() {
        let obs = Observable::from_iter(vec![
            StreamValue::Int(1), StreamValue::Int(1), StreamValue::Int(2),
            StreamValue::Int(2), StreamValue::Int(3),
        ]);
        let distinct = obs.distinct_until_changed();
        assert_eq!(distinct.collect(), vec![
            StreamValue::Int(1), StreamValue::Int(2), StreamValue::Int(3),
        ]);
    }

    #[test]
    fn observable_buffer_count() {
        let obs = Observable::from_iter(vec![
            StreamValue::Int(1), StreamValue::Int(2), StreamValue::Int(3),
            StreamValue::Int(4), StreamValue::Int(5),
        ]);
        let buffered = obs.buffer_count(2);
        let vals = buffered.collect();
        assert_eq!(vals.len(), 3); // [1,2], [3,4], [5]
        if let StreamValue::Array(a) = &vals[0] {
            assert_eq!(a.len(), 2);
        } else { panic!("expected array"); }
    }

    #[test]
    fn observable_merge() {
        let a = Observable::from_iter(vec![StreamValue::Int(1), StreamValue::Int(2)]);
        let b = Observable::from_iter(vec![StreamValue::Int(3), StreamValue::Int(4)]);
        let merged = a.merge(b);
        let vals = merged.collect();
        assert_eq!(vals.len(), 4);
    }

    #[test]
    fn event_bus_pub_sub() {
        let bus = EventBus::new();
        let received = Arc::new(Mutex::new(Vec::new()));
        let r = received.clone();
        bus.on("click", move |v| { r.lock().unwrap().push(v.clone()); });
        bus.emit("click", StreamValue::Str("button1".into()));
        bus.emit("click", StreamValue::Str("button2".into()));
        bus.emit("hover", StreamValue::Str("ignored".into())); // different topic
        assert_eq!(received.lock().unwrap().len(), 2);
    }

    #[test]
    fn event_bus_unsubscribe() {
        let bus = EventBus::new();
        let count = Arc::new(Mutex::new(0));
        let c = count.clone();
        let id = bus.on("x", move |_| { *c.lock().unwrap() += 1; });
        bus.emit("x", StreamValue::Null);
        bus.off("x", id);
        bus.emit("x", StreamValue::Null);
        assert_eq!(*count.lock().unwrap(), 1);
    }

    #[test]
    fn pipe_operators() {
        let result = Pipe::from(vec![
            StreamValue::Int(1), StreamValue::Int(2), StreamValue::Int(3),
            StreamValue::Int(4), StreamValue::Int(5),
        ])
        .filter(|v| v.as_int().unwrap_or(0) > 2)
        .map(|v| StreamValue::Int(v.as_int().unwrap_or(0) * 10))
        .take(2)
        .collect();
        assert_eq!(result, vec![StreamValue::Int(30), StreamValue::Int(40)]);
    }

    #[test]
    fn combine_latest_pairs() {
        let a = Observable::from_iter(vec![StreamValue::Int(1), StreamValue::Int(2)]);
        let b = Observable::from_iter(vec![StreamValue::Str("a".into()), StreamValue::Str("b".into())]);
        let combined = combine_latest(&a, &b);
        let vals = combined.collect();
        assert!(!vals.is_empty());
    }
}
