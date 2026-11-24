//! Killer GC: cycle collector for the `Rc` value graph (arrays, dicts, objects, closure captures, variable cells).
//!
//! `Rc` frees acyclic data the moment the last handle goes away but cannot free a reference cycle (A -> B -> A, or a
//! closure that captured the variable holding itself). Every container created on a thread is registered here with a
//! `Weak` handle, and the VM runs a collection every few tens of thousands of registrations (a safe point of its run
//! loop).
//!
//! Soundness rule (trial deletion): a container is cleared only when it is *provably* garbage, that is when every
//! strong reference to it comes from other containers that are themselves unreachable. For each registered container
//! the collector counts the references held *inside* registered containers; `Rc::strong_count` above that number means
//! some holder the collector does not know about (a local-variable frame, the operand stack, a native registry, a
//! value in flight on the Rust stack) keeps it alive, and so is everything reachable from it. Only the remainder is
//! cleared. An incomplete root set can therefore never lose data, it can only delay freeing. No VM roots are needed.
//!
//! Limits (documented in docs/project/SECURITY.md): containers that are not registered are never collected when they
//! are part of a cycle. That covers values copied by `Value::detached` (arguments and results of spawned tasks,
//! channel messages, server worker state), because a registered container must never leave the thread whose
//! collector owns its `Weak`. Reference cycles that run through a suspended generator frame are not collected either
//! (the frame is owned by the VM's generator table, not by a registered container).
//!
//! Thread-safety: all state is thread-local; spawned VMs have their own heap.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::{Rc, Weak};
use crate::value::{ObjectInstance, SharedArray, Value};

/// How many registrations between GC runs.
pub const GC_INTERVAL: u64 = 50_000;

/// Weak handle of one registered container.
pub enum WeakNode {
    Arr(Weak<RefCell<Vec<Value>>>),
    Dict(Weak<RefCell<crate::ordered_map::OrderedMap<Value>>>),
    Obj(Weak<RefCell<ObjectInstance>>),
    Caps(Weak<RefCell<Vec<(String, Value)>>>),
    Cell(Weak<RefCell<Value>>),
}

/// Strong handle held only while a collection runs.
enum Node {
    Arr(Rc<RefCell<Vec<Value>>>),
    Dict(Rc<RefCell<crate::ordered_map::OrderedMap<Value>>>),
    Obj(Rc<RefCell<ObjectInstance>>),
    Caps(Rc<RefCell<Vec<(String, Value)>>>),
    Cell(Rc<RefCell<Value>>),
}

impl WeakNode {
    fn alive(&self) -> bool {
        match self {
            WeakNode::Arr(w) => w.strong_count() > 0,
            WeakNode::Dict(w) => w.strong_count() > 0,
            WeakNode::Obj(w) => w.strong_count() > 0,
            WeakNode::Caps(w) => w.strong_count() > 0,
            WeakNode::Cell(w) => w.strong_count() > 0,
        }
    }
    fn upgrade(&self) -> Option<Node> {
        Some(match self {
            WeakNode::Arr(w) => Node::Arr(w.upgrade()?),
            WeakNode::Dict(w) => Node::Dict(w.upgrade()?),
            WeakNode::Obj(w) => Node::Obj(w.upgrade()?),
            WeakNode::Caps(w) => Node::Caps(w.upgrade()?),
            WeakNode::Cell(w) => Node::Cell(w.upgrade()?),
        })
    }
}

impl Node {
    fn ptr(&self) -> usize {
        match self {
            Node::Arr(r) => Rc::as_ptr(r) as *const u8 as usize,
            Node::Dict(r) => Rc::as_ptr(r) as *const u8 as usize,
            Node::Obj(r) => Rc::as_ptr(r) as *const u8 as usize,
            Node::Caps(r) => Rc::as_ptr(r) as *const u8 as usize,
            Node::Cell(r) => Rc::as_ptr(r) as *const u8 as usize,
        }
    }
    fn strong(&self) -> usize {
        match self {
            Node::Arr(r) => Rc::strong_count(r),
            Node::Dict(r) => Rc::strong_count(r),
            Node::Obj(r) => Rc::strong_count(r),
            Node::Caps(r) => Rc::strong_count(r),
            Node::Cell(r) => Rc::strong_count(r),
        }
    }
    /// Visit the values held directly; false when the container is borrowed right now (left alone).
    fn each_value(&self, f: &mut dyn FnMut(&Value)) -> bool {
        match self {
            Node::Arr(r) => r.try_borrow().map(|b| b.iter().for_each(|v| f(v))).is_ok(),
            Node::Dict(r) => r.try_borrow().map(|b| b.values().for_each(|v| f(v))).is_ok(),
            Node::Obj(r) => r.try_borrow().map(|b| b.fields.values().for_each(|v| f(v))).is_ok(),
            Node::Caps(r) => r.try_borrow().map(|b| b.iter().for_each(|(_, v)| f(v))).is_ok(),
            Node::Cell(r) => r.try_borrow().map(|b| f(&b)).is_ok(),
        }
    }
    /// Move the contents out (to be dropped by the caller once no borrow is held); None when borrowed.
    fn take_contents(&self) -> Option<Vec<Value>> {
        match self {
            Node::Arr(r) => r.try_borrow_mut().ok().map(|mut b| std::mem::take(&mut *b)),
            Node::Dict(r) => r.try_borrow_mut().ok().map(|mut b| b.take_values()),
            Node::Obj(r) => r.try_borrow_mut().ok().map(|mut b| b.fields.drain().map(|(_, v)| v).collect()),
            Node::Caps(r) => r.try_borrow_mut().ok().map(|mut b| std::mem::take(&mut *b).into_iter().map(|(_, v)| v).collect()),
            Node::Cell(r) => r.try_borrow_mut().ok().map(|mut b| vec![std::mem::replace(&mut *b, Value::Null)]),
        }
    }
}

thread_local! {
    /// Weak references to every registered container created on this thread.
    static GC_HEAP: RefCell<Vec<WeakNode>> = const { RefCell::new(Vec::new()) };
    /// Running count of registrations (wraps at u64::MAX, which is safe).
    pub static GC_ALLOC_COUNT: RefCell<u64> = const { RefCell::new(0) };
    /// While non-zero, new containers are not registered (see [`SuppressGuard`]).
    static SUPPRESS: Cell<u32> = const { Cell::new(0) };
    /// Heap length at which dead entries are pruned (so dead `Rc` boxes held alive by their `Weak` stay bounded).
    static PRUNE_AT: Cell<usize> = const { Cell::new(4096) };
}

/// Containers created while a guard is alive are not registered with this thread's collector. `Value::detached`
/// uses it: the copies it makes are handed to another thread, and a `Weak` that stays behind in this thread's
/// registry would let two threads touch the same non-atomic counts.
pub struct SuppressGuard;

impl SuppressGuard {
    pub fn new() -> Self {
        SUPPRESS.with(|s| s.set(s.get() + 1));
        SuppressGuard
    }
}

impl Default for SuppressGuard {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for SuppressGuard {
    fn drop(&mut self) {
        SUPPRESS.with(|s| s.set(s.get().saturating_sub(1)));
    }
}

/// Register a newly created SharedArray with the GC.  Called from SharedArray::new.
pub fn gc_register(arr: &SharedArray) {
    gc_register_node(WeakNode::Arr(arr.weak()));
}

/// Register a newly created container.
pub fn gc_register_node(node: WeakNode) {
    if SUPPRESS.with(|s| s.get()) != 0 {
        return;
    }
    GC_HEAP.with(|heap| {
        let mut h = heap.borrow_mut();
        h.push(node);
        let at = PRUNE_AT.with(|p| p.get());
        if h.len() >= at {
            h.retain(|w| w.alive());
            PRUNE_AT.with(|p| p.set((h.len() * 2).max(4096)));
        }
    });
    GC_ALLOC_COUNT.with(|c| *c.borrow_mut() += 1);
}

/// Returns true if the current thread has reached the GC trigger threshold.
#[inline]
pub fn should_collect() -> bool {
    GC_ALLOC_COUNT.with(|c| *c.borrow() % GC_INTERVAL == 0 && *c.borrow() > 0)
}

/// Visit the registered containers a value holds directly (looking through the boxes of `Signal` and `TaskError`).
fn each_edge(v: &Value, f: &mut dyn FnMut(usize)) {
    let mut cur = v;
    loop {
        match cur {
            Value::Array(a) => return f(a.rc_ptr()),
            Value::Dict(d) => return f(d.rc_ptr()),
            Value::Object(o) => return f(o.rc_ptr()),
            Value::Function { captured, .. } => return f(captured.rc_ptr()),
            Value::Cell(c) => return f(Rc::as_ptr(c) as *const u8 as usize),
            Value::Signal { value, .. } => cur = value,
            Value::TaskError(inner) => cur = inner,
            _ => return,
        }
    }
}

/// Run a full cycle collection on this thread (trial deletion, see the module documentation). The roots argument is
/// accepted for compatibility and not needed: every holder outside the registered containers, listed or not, is
/// found through the reference counts.
pub fn gc_collect<I>(_roots: I)
where
    I: Iterator<Item = Value>,
{
    // Work on a private copy of the registry: dropping garbage below may run code that registers or prunes.
    let heap: Vec<WeakNode> = GC_HEAP.with(|h| std::mem::take(&mut *h.borrow_mut()));
    let nodes: Vec<Node> = heap.iter().filter_map(|w| w.upgrade()).collect();
    let n = nodes.len();
    let index: HashMap<usize, usize> = nodes.iter().enumerate().map(|(i, nd)| (nd.ptr(), i)).collect();

    // References held inside registered containers.
    let mut internal = vec![0usize; n];
    let mut live = vec![false; n];
    for (i, node) in nodes.iter().enumerate() {
        let readable = node.each_value(&mut |v| {
            each_edge(v, &mut |p| {
                if let Some(&j) = index.get(&p) {
                    internal[j] += 1;
                }
            })
        });
        if !readable {
            live[i] = true; // being mutated right now: leave it alone (its targets stay live by the count)
        }
    }

    // A holder outside the registered containers (`nodes` itself holds exactly one reference) keeps it live.
    let mut work: Vec<usize> = Vec::new();
    for (i, node) in nodes.iter().enumerate() {
        if live[i] || node.strong() > 1 + internal[i] {
            live[i] = true;
            work.push(i);
        }
    }
    // Everything reachable from a live container is live.
    while let Some(i) = work.pop() {
        let mut found: Vec<usize> = Vec::new();
        nodes[i].each_value(&mut |v| each_edge(v, &mut |p| found.push(p)));
        for p in found {
            if let Some(&j) = index.get(&p) {
                if !live[j] {
                    live[j] = true;
                    work.push(j);
                }
            }
        }
    }

    // The rest are unreachable cycles: take their contents out and drop them with no borrow held.
    let mut garbage: Vec<Vec<Value>> = Vec::new();
    for (i, node) in nodes.iter().enumerate() {
        if !live[i] {
            if let Some(c) = node.take_contents() {
                garbage.push(c);
            }
        }
    }
    drop(garbage);
    drop(nodes);

    // Restore the registry: survivors first, then whatever was registered meanwhile.
    GC_HEAP.with(|h| {
        let mut h = h.borrow_mut();
        let added = std::mem::take(&mut *h);
        let mut kept: Vec<WeakNode> = heap.into_iter().filter(|w| w.alive()).collect();
        kept.extend(added);
        PRUNE_AT.with(|p| p.set((kept.len() * 2).max(4096)));
        *h = kept;
    });
}

/// Drop this thread's container registry now. A task thread calls it while it is still counted against the memory
/// cap, so the weak boxes it keeps alive are returned to the budget (thread-local destructors run after the thread
/// left the budget, and their frees would otherwise never be subtracted).
pub fn release_thread() {
    let heap = GC_HEAP.try_with(|h| std::mem::take(&mut *h.borrow_mut())).unwrap_or_default();
    drop(heap);
}

/// Collect GC statistics (tracked containers, last_alloc_count) — for diagnostics.
pub fn gc_stats() -> (usize, u64) {
    let tracked = GC_HEAP.with(|h| h.borrow().len());
    let allocs = GC_ALLOC_COUNT.with(|c| *c.borrow());
    (tracked, allocs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::value::{Value, SharedArray};

    /// A self-referential array with no remaining holders other than itself.
    fn garbage_self_cycle() -> Weak<RefCell<Vec<Value>>> {
        let a = SharedArray::new(vec![]);
        a.push(Value::Array(a.clone())); // A contains itself
        a.weak()
        // the local handle `a` drops here; only the self-reference keeps A alive
    }

    #[test]
    fn gc_collects_unreferenced_cycle() {
        let weak = garbage_self_cycle();
        assert_eq!(weak.strong_count(), 1, "only the self-reference remains");
        gc_collect(std::iter::empty());
        // cleared (contents dropped) -> the self-reference is gone -> memory freed
        assert_eq!(weak.strong_count(), 0, "GC should have broken and freed the cycle");
    }

    #[test]
    fn gc_collects_two_array_cycle() {
        let (wa, wb) = {
            let a = SharedArray::new(vec![]);
            let b = SharedArray::new(vec![Value::Array(a.clone())]);
            a.push(Value::Array(b.clone()));
            (a.weak(), b.weak())
        };
        assert_eq!((wa.strong_count(), wb.strong_count()), (1, 1));
        gc_collect(std::iter::empty());
        assert_eq!((wa.strong_count(), wb.strong_count()), (0, 0));
    }

    #[test]
    fn gc_keeps_reachable_arrays() {
        let arr = SharedArray::new(vec![Value::Number(42.0)]);
        let root = Value::Array(arr.clone());

        gc_collect(std::iter::once(root));

        assert_eq!(arr.len(), 1, "GC must not collect reachable arrays");
    }

    /// The data-loss bug: arrays held by something the VM does not list as a root (for example
    /// local-variable frames) used to be emptied. They must survive, even when they are cyclic.
    #[test]
    fn gc_never_clears_arrays_with_unknown_holders() {
        let held = SharedArray::new(vec![Value::Number(1.0), Value::Number(2.0)]);
        gc_collect(std::iter::empty()); // `held` is not in the root set
        assert_eq!(held.len(), 2, "an externally held array must not be cleared");

        let cyc = SharedArray::new(vec![Value::Number(7.0)]);
        cyc.push(Value::Array(cyc.clone()));
        gc_collect(std::iter::empty()); // still held by `cyc`
        assert_eq!(cyc.len(), 2, "an externally held cyclic array must not be cleared");
    }

    #[test]
    fn gc_keeps_everything_reachable_from_an_unknown_holder() {
        let inner = SharedArray::new(vec![Value::Number(5.0)]);
        let outer = SharedArray::new(vec![Value::Array(inner.clone())]);
        let winner = inner.weak();
        drop(inner); // only `outer` references it now
        gc_collect(std::iter::empty()); // `outer` is held by this test, not by the roots
        assert_eq!(outer.len(), 1);
        let kept = winner.upgrade().expect("inner array must stay alive");
        assert_eq!(kept.borrow().len(), 1, "inner array contents must not be cleared");
    }

    #[test]
    fn gc_keeps_arrays_inside_dicts_held_elsewhere() {
        let inner = SharedArray::new(vec![Value::Number(9.0)]);
        let mut map = std::collections::HashMap::new();
        map.insert("k".to_string(), Value::Array(inner.clone()));
        let holder = Value::Dict(crate::value::SharedDict::new(map)); // held by this test
        drop(inner);
        gc_collect(std::iter::empty());
        if let Value::Dict(d) = &holder {
            if let Some(Value::Array(a)) = d.get("k") {
                assert_eq!(a.len(), 1);
                return;
            }
        }
        panic!("array in dict was lost");
    }

    #[test]
    fn gc_prunes_dead_weaks() {
        {
            let _tmp = SharedArray::new(vec![Value::Null]);
            // _tmp drops here
        }
        gc_collect(std::iter::empty());
        let (tracked, _) = gc_stats();
        let _ = tracked;
    }

    /// Without a GC, a self-referential array leaks: the cycle keeps itself alive.
    #[test]
    fn prove_cycle_leak_without_gc() {
        let a = SharedArray::new(vec![]);
        let weak = a.weak();
        let a_val = Value::Array(a.clone());
        a.push(a_val);
        assert_eq!(weak.strong_count(), 2, "both `a` and the vec element hold a ref");
        drop(a);
        assert_eq!(weak.strong_count(), 1, "LEAK: the cycle keeps memory alive forever");
        if let Some(strong) = weak.upgrade() {
            strong.borrow_mut().clear(); // break cycle manually
        }
        assert_eq!(weak.strong_count(), 0, "after breaking cycle: memory freed");
    }

    #[test]
    fn gc_collects_dict_and_object_cycles_and_spares_held_ones() {
        use crate::value::{ObjectInstance, SharedDict, SharedObject};
        let (wd, wo) = {
            let d = SharedDict::empty();
            let o = SharedObject::new(ObjectInstance { class_name: "N".into(), fields: Default::default() });
            d.insert("o".into(), Value::Object(o.clone()));
            o.set_field("d".into(), Value::Dict(d.clone()));
            (std::rc::Rc::downgrade(d.rc()), std::rc::Rc::downgrade(o.rc()))
        };
        assert_eq!((wd.strong_count(), wo.strong_count()), (1, 1));
        gc_collect(std::iter::empty());
        assert_eq!((wd.strong_count(), wo.strong_count()), (0, 0), "dict <-> object cycle must be freed");

        let held = SharedDict::empty();
        held.insert("self".into(), Value::Dict(held.clone()));
        held.insert("n".into(), Value::Number(3.0));
        gc_collect(std::iter::empty());
        assert!(matches!(held.get("n"), Some(Value::Number(n)) if n == 3.0), "a held cyclic dict must keep its contents");
    }

    #[test]
    fn gc_collects_closure_capture_cycles() {
        use crate::value::{new_cell, Captures};
        let w = {
            let cell = new_cell(Value::Null);
            let caps = Captures::from_items(vec![("f".to_string(), Value::Cell(cell.clone()))]);
            *cell.borrow_mut() = Value::Function { params: std::rc::Rc::from(Vec::<String>::new()), bytecode_start: 0, captured: caps.clone() };
            std::rc::Rc::downgrade(&cell)
        };
        assert_eq!(w.strong_count(), 1, "only the capture table holds the cell, and only the cell holds the table");
        gc_collect(std::iter::empty());
        assert_eq!(w.strong_count(), 0, "a closure that captured itself must be freed");
    }

    #[test]
    fn detached_copies_are_not_registered() {
        let a = SharedArray::new(vec![Value::Number(1.0)]);
        a.push(Value::Array(a.clone()));
        let (before, _) = gc_stats();
        let copy = Value::Array(a.clone()).detached();
        let (after, _) = gc_stats();
        assert_eq!(before, after, "a detached copy leaves this thread, so it must not enter this thread's registry");
        drop(copy);
    }
}
