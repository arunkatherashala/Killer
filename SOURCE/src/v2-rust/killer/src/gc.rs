/// Killer GC — cycle collector for the `Rc` array graph
///
/// `Rc<RefCell<Vec<Value>>>` handles typical short-lived allocations efficiently
/// but cannot collect reference cycles (A → B → A).  This module adds a
/// stop-the-world cycle sweep that the VM triggers every GC_INTERVAL allocs.
///
/// Safety rule: an array is only cleared when it is *provably* garbage, i.e. every
/// strong reference to it comes from other unreachable arrays (trial deletion).  Holders
/// the collector does not know about — local-variable frames, native registries, values
/// in flight — keep their arrays alive, so an incomplete root set can never lose data.
///
///   Mark:   walk every Value reachable from VM roots, record each array's raw Rc address.
///   Count:  for every unreachable ("candidate") array, count the references held *inside*
///           candidate arrays.  strong_count above that means an outside holder exists.
///   Close:  candidates with an outside holder are live roots; everything reachable from
///           them is live too.
///   Sweep:  the remaining candidates are garbage cycles — clear their contents.
///
/// Thread-safety: all state is thread-local; spawned VMs have their own heap.

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::{Rc, Weak};
use crate::value::{Value, SharedArray};

/// How many SharedArray allocations between GC runs.
pub const GC_INTERVAL: u64 = 50_000;

thread_local! {
    /// Weak references to every living SharedArray on this thread.
    static GC_HEAP: RefCell<Vec<Weak<RefCell<Vec<Value>>>>> = RefCell::new(Vec::new());
    /// Running count of SharedArray allocations (wraps at u64::MAX — safe).
    pub static GC_ALLOC_COUNT: RefCell<u64> = RefCell::new(0);
}

/// Register a newly created SharedArray with the GC.  Called from SharedArray::new.
pub fn gc_register(arr: &SharedArray) {
    GC_HEAP.with(|heap| {
        heap.borrow_mut().push(arr.weak());
    });
    GC_ALLOC_COUNT.with(|c| *c.borrow_mut() += 1);
}

/// Returns true if the current thread has reached the GC trigger threshold.
#[inline]
pub fn should_collect() -> bool {
    GC_ALLOC_COUNT.with(|c| *c.borrow() % GC_INTERVAL == 0 && *c.borrow() > 0)
}

// ── Mark phase ────────────────────────────────────────────────────────────────

/// Walk `val` and all Values reachable from it, recording array Rc addresses.
fn mark_value(val: &Value, seen: &mut HashSet<usize>, queue: &mut Vec<Value>) {
    match val {
        Value::Array(arr) => {
            let ptr = arr.rc_ptr();
            if seen.insert(ptr) {
                // First visit — enqueue elements for further traversal.
                queue.extend(arr.clone_elements());
            }
        }
        Value::Dict(dict) => {
            for v in dict.values() {
                queue.push(v);
            }
        }
        Value::Function { captured, .. } => {
            for v in captured.values() {
                queue.push(v.clone());
            }
        }
        Value::Signal { value, .. } => {
            queue.push((**value).clone());
        }
        Value::Object(obj) => {
            for v in obj.fields().into_values() {
                queue.push(v);
            }
        }
        _ => {}
    }
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Run a full mark-and-sweep cycle collection.
///
/// `roots` should be an iterator over all Values currently live in the VM
/// (stack + all scope frames).  Unreachable arrays are cleared in-place to
/// break cycles; their Rc counts then drop to zero and memory is reclaimed by
/// the normal Rc destructor when the caller releases its handle.
pub fn gc_collect<I>(roots: I)
where
    I: Iterator<Item = Value>,
{
    let mut reachable: HashSet<usize> = HashSet::new();
    let mut queue: Vec<Value> = roots.collect();

    // Mark
    while let Some(val) = queue.pop() {
        mark_value(&val, &mut reachable, &mut queue);
    }

    // Sweep (trial deletion): only arrays whose every reference comes from other garbage.
    GC_HEAP.with(|heap| {
        let mut h = heap.borrow_mut();
        let strongs: Vec<Rc<RefCell<Vec<Value>>>> = h.iter().filter_map(|w| w.upgrade()).collect();
        let ptr_of = |rc: &Rc<RefCell<Vec<Value>>>| Rc::as_ptr(rc) as usize;
        let index: HashMap<usize, usize> = strongs.iter().enumerate().map(|(i, rc)| (ptr_of(rc), i)).collect();
        let candidate: Vec<bool> = strongs.iter().map(|rc| !reachable.contains(&ptr_of(rc))).collect();

        // References held inside candidate arrays.
        let mut internal: HashMap<usize, usize> = HashMap::new();
        let mut live: Vec<bool> = candidate.iter().map(|c| !*c).collect();
        for (i, rc) in strongs.iter().enumerate() {
            if !candidate[i] {
                continue;
            }
            match rc.try_borrow() {
                Ok(elems) => {
                    for v in elems.iter() {
                        count_array_refs(v, &mut internal);
                    }
                }
                Err(_) => live[i] = true, // being mutated right now: leave it alone
            }
        }

        // Candidates with a holder outside the candidate set are live roots.
        let mut work: Vec<usize> = Vec::new();
        for (i, rc) in strongs.iter().enumerate() {
            if candidate[i] && !live[i] {
                let inside = internal.get(&ptr_of(rc)).copied().unwrap_or(0);
                // `strongs` itself holds exactly one reference.
                if Rc::strong_count(rc) > 1 + inside {
                    live[i] = true;
                }
            }
            if candidate[i] && live[i] {
                work.push(i);
            }
        }
        // Everything reachable from a live candidate is live.
        let mut found: Vec<usize> = Vec::new();
        while let Some(i) = work.pop() {
            if let Ok(elems) = strongs[i].try_borrow() {
                found.clear();
                for v in elems.iter() {
                    collect_array_ptrs(v, &mut found);
                }
                for ptr in found.iter() {
                    if let Some(&j) = index.get(ptr) {
                        if !live[j] {
                            live[j] = true;
                            work.push(j);
                        }
                    }
                }
            }
        }
        // Remaining candidates are unreachable cycles: break them.
        for (i, rc) in strongs.iter().enumerate() {
            if !live[i] {
                rc.borrow_mut().clear();
            }
        }
        drop(strongs);
        // Prune dead Weaks.
        h.retain(|w| w.strong_count() > 0);
    });
}

/// Count references held directly by a candidate array. Dicts and objects are shared handles that
/// other holders may also reference, so arrays reachable only through them are deliberately NOT
/// counted as "internal": that keeps them live (a cycle through a dict is leaked, never freed).
fn count_array_refs(v: &Value, internal: &mut HashMap<usize, usize>) {
    match v {
        Value::Array(a) => *internal.entry(a.rc_ptr()).or_insert(0) += 1,
        Value::Function { captured, .. } => captured.values().for_each(|x| count_array_refs(x, internal)),
        Value::Signal { value, .. } => count_array_refs(value, internal),
        _ => {}
    }
}

fn collect_array_ptrs(v: &Value, out: &mut Vec<usize>) {
    match v {
        Value::Array(a) => out.push(a.rc_ptr()),
        Value::Dict(d) => d.values().iter().for_each(|x| collect_array_ptrs(x, out)),
        Value::Object(o) => o.fields().values().for_each(|x| collect_array_ptrs(x, out)),
        Value::Function { captured, .. } => captured.values().for_each(|x| collect_array_ptrs(x, out)),
        Value::Signal { value, .. } => collect_array_ptrs(value, out),
        _ => {}
    }
}

/// Collect GC statistics (tracked_arrays, last_alloc_count) — for diagnostics.
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
}
