//! **Reactive state system** — signals, computed values, effects, and batched updates.
//!
//! Inspired by Solid.js / Angular Signals / Preact signals — fine-grained reactivity without
//! a virtual DOM diffing pass. All state lives in a centralized [`ReactiveStore`] so Rust
//! ownership is clean (no Rc/RefCell spaghetti).
//!
//! ```text
//! Signal::set(v) ──► dirty set ──► topological propagation ──► effects fire
//! ```

use std::collections::{HashMap, HashSet, VecDeque};

// ── Identifiers ──────────────────────────────────────────────────────────────

pub type SignalId = u64;
pub type EffectId = u64;

// ── Stored value (UI-layer, independent of VM Value) ─────────────────────────

/// Lightweight value type for the reactive layer (maps 1:1 to VM `Value` in builtins).
#[derive(Debug, Clone, PartialEq)]
pub enum RxValue {
    Null,
    Bool(bool),
    Number(f64),
    Str(String),
    Array(Vec<RxValue>),
    Dict(HashMap<String, RxValue>),
}

impl Default for RxValue {
    fn default() -> Self {
        RxValue::Null
    }
}

impl std::fmt::Display for RxValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RxValue::Null => write!(f, "null"),
            RxValue::Bool(b) => write!(f, "{}", b),
            RxValue::Number(n) => write!(f, "{}", n),
            RxValue::Str(s) => write!(f, "{}", s),
            RxValue::Array(a) => {
                write!(f, "[")?;
                for (i, v) in a.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{}", v)?;
                }
                write!(f, "]")
            }
            RxValue::Dict(d) => {
                write!(f, "{{")?;
                for (i, (k, v)) in d.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{}: {}", k, v)?;
                }
                write!(f, "}}")
            }
        }
    }
}

// ── Signal slot ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
struct SignalSlot {
    value: RxValue,
    /// Computed signals that depend on this signal.
    dependents: HashSet<SignalId>,
    /// Effects subscribed to this signal.
    subscribed_effects: HashSet<EffectId>,
}

// ── Computed definition ──────────────────────────────────────────────────────

/// A computed signal: derives its value from other signals via `compute_fn`.
/// We store the dependency list and a function tag (used by builtin dispatch).
#[derive(Debug, Clone)]
struct ComputedSlot {
    signal_id: SignalId,
    /// IDs of upstream signals this computed reads from.
    deps: Vec<SignalId>,
    /// Expression tag — interpreted by the builtin layer (e.g. "add", "mul", "concat").
    expr: ComputedExpr,
}

/// Built-in computed expressions (extensible via `Custom`).
#[derive(Debug, Clone, PartialEq)]
pub enum ComputedExpr {
    /// Sum all deps: a + b + …
    Sum,
    /// Multiply all deps: a * b * …
    Product,
    /// Concatenate string representations.
    Concat,
    /// First dep > 0 → true, else false.
    ToBool,
    /// Return first dep unchanged (identity / alias).
    Identity,
    /// Count of deps.
    Count,
    /// Min of all numeric deps.
    Min,
    /// Max of all numeric deps.
    Max,
    /// Average of all numeric deps.
    Average,
    /// Negate first dep.
    Negate,
    /// Custom expression tag for user-defined computed logic.
    Custom(String),
}

// ── Effect definition ────────────────────────────────────────────────────────

/// Side-effect triggered when any of its deps change.
#[derive(Debug, Clone)]
struct EffectSlot {
    deps: Vec<SignalId>,
    /// Tag for builtin dispatch (e.g. "log", "render", "fetch").
    action: String,
    enabled: bool,
    /// Number of times this effect has fired (for devtools).
    fire_count: u64,
}

// ── Reactive Store ───────────────────────────────────────────────────────────

/// Central reactive state store — all signals, computed values, and effects live here.
///
/// # Design
/// - Signals are the atomic unit of state.
/// - Computed values are derived signals that auto-update when deps change.
/// - Effects are callbacks that fire when their deps change.
/// - Batch mode defers propagation until `batch_end()`.
///
/// # Example (via builtins)
/// ```text
/// let store = ui_reactive_create()
/// let count = ui_signal_create(store, 0)
/// let doubled = ui_computed_create(store, [count], "product")   // not yet: needs mul by 2
/// ui_signal_set(store, count, 5)
/// ui_signal_get(store, doubled)  // → 10
/// ```
#[derive(Debug)]
pub struct ReactiveStore {
    signals: HashMap<SignalId, SignalSlot>,
    computed: Vec<ComputedSlot>,
    effects: HashMap<EffectId, EffectSlot>,
    next_signal_id: SignalId,
    next_effect_id: EffectId,
    /// Batch mode: when > 0, propagation is deferred.
    batch_depth: u32,
    /// Signals dirtied during a batch.
    dirty_batch: HashSet<SignalId>,
    /// History of effect fires (for devtools): `(effect_id, signal_id_that_triggered)`.
    pub effect_log: Vec<(EffectId, SignalId)>,
}

impl ReactiveStore {
    pub fn new() -> Self {
        Self {
            signals: HashMap::new(),
            computed: Vec::new(),
            effects: HashMap::new(),
            next_signal_id: 1,
            next_effect_id: 1,
            batch_depth: 0,
            dirty_batch: HashSet::new(),
            effect_log: Vec::new(),
        }
    }

    // ── Signals ──────────────────────────────────────────────────────────

    /// Create a new signal with an initial value. Returns the signal ID.
    pub fn create_signal(&mut self, initial: RxValue) -> SignalId {
        let id = self.next_signal_id;
        self.next_signal_id += 1;
        self.signals.insert(id, SignalSlot {
            value: initial,
            dependents: HashSet::new(),
            subscribed_effects: HashSet::new(),
        });
        id
    }

    /// Read the current value of a signal (or computed signal).
    pub fn get(&self, id: SignalId) -> Option<&RxValue> {
        self.signals.get(&id).map(|s| &s.value)
    }

    /// Set a signal's value and propagate changes (or defer if batching).
    pub fn set(&mut self, id: SignalId, value: RxValue) -> Result<RxValue, ReactiveError> {
        let slot = self.signals.get(&id).ok_or(ReactiveError::UnknownSignal(id))?;
        let old = slot.value.clone();
        if old == value {
            return Ok(old); // no-op if unchanged
        }
        self.signals.get_mut(&id).unwrap().value = value;
        if self.batch_depth > 0 {
            self.dirty_batch.insert(id);
        } else {
            self.propagate(id);
        }
        Ok(old)
    }

    /// Bulk-read multiple signals. Returns `None` entries for unknown IDs.
    pub fn get_many(&self, ids: &[SignalId]) -> Vec<Option<RxValue>> {
        ids.iter().map(|id| self.get(*id).cloned()).collect()
    }

    /// Number of live signals (including computed).
    pub fn signal_count(&self) -> usize {
        self.signals.len()
    }

    // ── Computed ─────────────────────────────────────────────────────────

    /// Create a computed signal derived from `deps` using `expr`.
    pub fn create_computed(&mut self, deps: Vec<SignalId>, expr: ComputedExpr) -> Result<SignalId, ReactiveError> {
        // Validate all deps exist
        for &d in &deps {
            if !self.signals.contains_key(&d) {
                return Err(ReactiveError::UnknownSignal(d));
            }
        }
        let computed_id = self.create_signal(RxValue::Null);
        // Register as dependent of each upstream
        for &d in &deps {
            self.signals.get_mut(&d).unwrap().dependents.insert(computed_id);
        }
        let slot = ComputedSlot {
            signal_id: computed_id,
            deps: deps.clone(),
            expr,
        };
        self.computed.push(slot);
        // Compute initial value
        self.recompute(computed_id);
        Ok(computed_id)
    }

    /// Recompute a computed signal's value from its deps.
    fn recompute(&mut self, computed_id: SignalId) {
        let slot = self.computed.iter().find(|c| c.signal_id == computed_id);
        let slot = match slot {
            Some(s) => s.clone(),
            None => return,
        };
        let dep_values: Vec<RxValue> = slot.deps.iter()
            .filter_map(|d| self.signals.get(d).map(|s| s.value.clone()))
            .collect();
        let new_val = evaluate_computed(&slot.expr, &dep_values);
        if let Some(s) = self.signals.get_mut(&computed_id) {
            s.value = new_val;
        }
    }

    // ── Effects ──────────────────────────────────────────────────────────

    /// Create an effect that fires when any of `deps` change.
    pub fn create_effect(&mut self, deps: Vec<SignalId>, action: String) -> Result<EffectId, ReactiveError> {
        for &d in &deps {
            if !self.signals.contains_key(&d) {
                return Err(ReactiveError::UnknownSignal(d));
            }
        }
        let eid = self.next_effect_id;
        self.next_effect_id += 1;
        // Subscribe to each dep
        for &d in &deps {
            self.signals.get_mut(&d).unwrap().subscribed_effects.insert(eid);
        }
        self.effects.insert(eid, EffectSlot {
            deps,
            action,
            enabled: true,
            fire_count: 0,
        });
        Ok(eid)
    }

    /// Enable/disable an effect.
    pub fn set_effect_enabled(&mut self, eid: EffectId, enabled: bool) -> Result<(), ReactiveError> {
        let eff = self.effects.get_mut(&eid).ok_or(ReactiveError::UnknownEffect(eid))?;
        eff.enabled = enabled;
        Ok(())
    }

    /// Remove an effect and unsubscribe from all signals.
    pub fn remove_effect(&mut self, eid: EffectId) -> Result<(), ReactiveError> {
        let eff = self.effects.remove(&eid).ok_or(ReactiveError::UnknownEffect(eid))?;
        for &d in &eff.deps {
            if let Some(s) = self.signals.get_mut(&d) {
                s.subscribed_effects.remove(&eid);
            }
        }
        Ok(())
    }

    /// Get the list of fired effects (for devtools / testing).
    pub fn fired_effects(&self) -> &[(EffectId, SignalId)] {
        &self.effect_log
    }

    // ── Batching ─────────────────────────────────────────────────────────

    /// Begin a batch: signal writes are deferred until `batch_end`.
    pub fn batch_begin(&mut self) {
        self.batch_depth += 1;
    }

    /// End batch: propagate all deferred dirty signals.
    pub fn batch_end(&mut self) {
        if self.batch_depth == 0 { return; }
        self.batch_depth -= 1;
        if self.batch_depth == 0 {
            let dirty: Vec<SignalId> = self.dirty_batch.drain().collect();
            for id in dirty {
                self.propagate(id);
            }
        }
    }

    // ── Propagation (topological order) ──────────────────────────────────

    /// Propagate changes from `root` signal through computed + effects.
    fn propagate(&mut self, root: SignalId) {
        // BFS through dependents (computed signals)
        let mut queue = VecDeque::new();
        let mut visited = HashSet::new();
        queue.push_back(root);
        visited.insert(root);

        // Collect effects to fire
        let mut effects_to_fire: Vec<(EffectId, SignalId)> = Vec::new();

        // Collect direct effects from root
        if let Some(slot) = self.signals.get(&root) {
            for &eid in &slot.subscribed_effects {
                effects_to_fire.push((eid, root));
            }
        }

        while let Some(current) = queue.pop_front() {
            let dependents: Vec<SignalId> = self.signals
                .get(&current)
                .map(|s| s.dependents.iter().cloned().collect())
                .unwrap_or_default();

            for dep_id in dependents {
                self.recompute(dep_id);
                if visited.insert(dep_id) {
                    queue.push_back(dep_id);
                    // Collect effects from this computed signal
                    if let Some(slot) = self.signals.get(&dep_id) {
                        for &eid in &slot.subscribed_effects {
                            effects_to_fire.push((eid, dep_id));
                        }
                    }
                }
            }
        }

        // Fire effects
        for (eid, trigger_id) in effects_to_fire {
            if let Some(eff) = self.effects.get_mut(&eid) {
                if eff.enabled {
                    eff.fire_count += 1;
                    self.effect_log.push((eid, trigger_id));
                }
            }
        }
    }

    // ── Introspection (devtools) ─────────────────────────────────────────

    /// Dump the full dependency graph as a JSON string (for devtools).
    pub fn debug_json(&self) -> String {
        let mut s = String::from("{\n  \"signals\": {\n");
        let mut ids: Vec<SignalId> = self.signals.keys().cloned().collect();
        ids.sort();
        for (i, id) in ids.iter().enumerate() {
            let slot = &self.signals[id];
            if i > 0 { s.push_str(",\n"); }
            s.push_str(&format!(
                "    \"{}\": {{\"value\": \"{}\", \"dependents\": {:?}, \"effects\": {:?}}}",
                id, slot.value,
                slot.dependents.iter().collect::<Vec<_>>(),
                slot.subscribed_effects.iter().collect::<Vec<_>>()
            ));
        }
        s.push_str("\n  },\n  \"computed\": [\n");
        for (i, c) in self.computed.iter().enumerate() {
            if i > 0 { s.push_str(",\n"); }
            s.push_str(&format!(
                "    {{\"id\": {}, \"deps\": {:?}, \"expr\": \"{:?}\"}}",
                c.signal_id, c.deps, c.expr
            ));
        }
        s.push_str("\n  ],\n  \"effects\": {\n");
        let mut eids: Vec<EffectId> = self.effects.keys().cloned().collect();
        eids.sort();
        for (i, eid) in eids.iter().enumerate() {
            let eff = &self.effects[eid];
            if i > 0 { s.push_str(",\n"); }
            s.push_str(&format!(
                "    \"{}\": {{\"action\": \"{}\", \"deps\": {:?}, \"enabled\": {}, \"fire_count\": {}}}",
                eid, eff.action, eff.deps, eff.enabled, eff.fire_count
            ));
        }
        s.push_str("\n  }\n}\n");
        s
    }

    /// Remove a signal (and clean up dependents/computed). Returns error if signal has dependents.
    pub fn remove_signal(&mut self, id: SignalId) -> Result<RxValue, ReactiveError> {
        let slot = self.signals.get(&id).ok_or(ReactiveError::UnknownSignal(id))?;
        if !slot.dependents.is_empty() {
            return Err(ReactiveError::HasDependents(id));
        }
        let slot = self.signals.remove(&id).unwrap();
        // Remove from computed deps
        self.computed.retain(|c| c.signal_id != id);
        // Remove from other signals' dependent lists
        for (_, s) in &mut self.signals {
            s.dependents.remove(&id);
        }
        Ok(slot.value)
    }
}

impl Default for ReactiveStore {
    fn default() -> Self {
        Self::new()
    }
}

// ── Computed evaluator ───────────────────────────────────────────────────────

fn to_f64(v: &RxValue) -> f64 {
    match v {
        RxValue::Number(n) => *n,
        RxValue::Bool(b) => if *b { 1.0 } else { 0.0 },
        RxValue::Str(s) => s.parse().unwrap_or(0.0),
        _ => 0.0,
    }
}

fn evaluate_computed(expr: &ComputedExpr, deps: &[RxValue]) -> RxValue {
    match expr {
        ComputedExpr::Sum => {
            RxValue::Number(deps.iter().map(to_f64).sum())
        }
        ComputedExpr::Product => {
            RxValue::Number(deps.iter().map(to_f64).product())
        }
        ComputedExpr::Concat => {
            RxValue::Str(deps.iter().map(|v| format!("{}", v)).collect())
        }
        ComputedExpr::ToBool => {
            let v = deps.first().map(to_f64).unwrap_or(0.0);
            RxValue::Bool(v != 0.0)
        }
        ComputedExpr::Identity => {
            deps.first().cloned().unwrap_or(RxValue::Null)
        }
        ComputedExpr::Count => {
            RxValue::Number(deps.len() as f64)
        }
        ComputedExpr::Min => {
            let min = deps.iter().map(to_f64).fold(f64::INFINITY, f64::min);
            RxValue::Number(if min == f64::INFINITY { 0.0 } else { min })
        }
        ComputedExpr::Max => {
            let max = deps.iter().map(to_f64).fold(f64::NEG_INFINITY, f64::max);
            RxValue::Number(if max == f64::NEG_INFINITY { 0.0 } else { max })
        }
        ComputedExpr::Average => {
            if deps.is_empty() {
                return RxValue::Number(0.0);
            }
            let sum: f64 = deps.iter().map(to_f64).sum();
            RxValue::Number(sum / deps.len() as f64)
        }
        ComputedExpr::Negate => {
            let v = deps.first().map(to_f64).unwrap_or(0.0);
            RxValue::Number(-v)
        }
        ComputedExpr::Custom(_tag) => {
            // Custom expressions return first dep by default; override in builtins.
            deps.first().cloned().unwrap_or(RxValue::Null)
        }
    }
}

// ── Errors ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum ReactiveError {
    UnknownSignal(SignalId),
    UnknownEffect(EffectId),
    HasDependents(SignalId),
}

impl std::fmt::Display for ReactiveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReactiveError::UnknownSignal(id) => write!(f, "unknown signal {}", id),
            ReactiveError::UnknownEffect(id) => write!(f, "unknown effect {}", id),
            ReactiveError::HasDependents(id) => write!(f, "signal {} still has dependents", id),
        }
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signal_create_get_set() {
        let mut store = ReactiveStore::new();
        let id = store.create_signal(RxValue::Number(10.0));
        assert_eq!(store.get(id), Some(&RxValue::Number(10.0)));
        let old = store.set(id, RxValue::Number(20.0)).unwrap();
        assert_eq!(old, RxValue::Number(10.0));
        assert_eq!(store.get(id), Some(&RxValue::Number(20.0)));
    }

    #[test]
    fn computed_sum() {
        let mut store = ReactiveStore::new();
        let a = store.create_signal(RxValue::Number(3.0));
        let b = store.create_signal(RxValue::Number(7.0));
        let sum = store.create_computed(vec![a, b], ComputedExpr::Sum).unwrap();
        assert_eq!(store.get(sum), Some(&RxValue::Number(10.0)));
        store.set(a, RxValue::Number(5.0)).unwrap();
        assert_eq!(store.get(sum), Some(&RxValue::Number(12.0)));
    }

    #[test]
    fn computed_chain() {
        let mut store = ReactiveStore::new();
        let x = store.create_signal(RxValue::Number(2.0));
        let doubled = store.create_computed(vec![x, x], ComputedExpr::Sum).unwrap();
        let quad = store.create_computed(vec![doubled, doubled], ComputedExpr::Sum).unwrap();
        assert_eq!(store.get(quad), Some(&RxValue::Number(8.0)));
        store.set(x, RxValue::Number(3.0)).unwrap();
        assert_eq!(store.get(quad), Some(&RxValue::Number(12.0)));
    }

    #[test]
    fn effect_fires_on_change() {
        let mut store = ReactiveStore::new();
        let a = store.create_signal(RxValue::Number(0.0));
        let eid = store.create_effect(vec![a], "log".into()).unwrap();
        assert!(store.effect_log.is_empty());
        store.set(a, RxValue::Number(1.0)).unwrap();
        assert_eq!(store.effect_log.len(), 1);
        assert_eq!(store.effect_log[0], (eid, a));
    }

    #[test]
    fn effect_no_fire_when_disabled() {
        let mut store = ReactiveStore::new();
        let a = store.create_signal(RxValue::Number(0.0));
        let eid = store.create_effect(vec![a], "log".into()).unwrap();
        store.set_effect_enabled(eid, false).unwrap();
        store.set(a, RxValue::Number(1.0)).unwrap();
        assert!(store.effect_log.is_empty());
    }

    #[test]
    fn batch_defers_propagation() {
        let mut store = ReactiveStore::new();
        let a = store.create_signal(RxValue::Number(0.0));
        let b = store.create_signal(RxValue::Number(0.0));
        let sum = store.create_computed(vec![a, b], ComputedExpr::Sum).unwrap();
        store.batch_begin();
        store.set(a, RxValue::Number(5.0)).unwrap();
        store.set(b, RxValue::Number(10.0)).unwrap();
        // Not yet propagated
        assert_eq!(store.get(sum), Some(&RxValue::Number(0.0)));
        store.batch_end();
        assert_eq!(store.get(sum), Some(&RxValue::Number(15.0)));
    }

    #[test]
    fn noop_on_same_value() {
        let mut store = ReactiveStore::new();
        let a = store.create_signal(RxValue::Number(5.0));
        let _eid = store.create_effect(vec![a], "log".into()).unwrap();
        store.set(a, RxValue::Number(5.0)).unwrap(); // same value
        assert!(store.effect_log.is_empty()); // no fire
    }

    #[test]
    fn debug_json_not_empty() {
        let mut store = ReactiveStore::new();
        let _a = store.create_signal(RxValue::Number(1.0));
        let json = store.debug_json();
        assert!(json.contains("\"signals\""));
        assert!(json.contains("\"1\""));
    }

    #[test]
    fn computed_min_max_avg() {
        let mut store = ReactiveStore::new();
        let a = store.create_signal(RxValue::Number(10.0));
        let b = store.create_signal(RxValue::Number(20.0));
        let c = store.create_signal(RxValue::Number(30.0));
        let mn = store.create_computed(vec![a, b, c], ComputedExpr::Min).unwrap();
        let mx = store.create_computed(vec![a, b, c], ComputedExpr::Max).unwrap();
        let avg = store.create_computed(vec![a, b, c], ComputedExpr::Average).unwrap();
        assert_eq!(store.get(mn), Some(&RxValue::Number(10.0)));
        assert_eq!(store.get(mx), Some(&RxValue::Number(30.0)));
        assert_eq!(store.get(avg), Some(&RxValue::Number(20.0)));
    }
}
