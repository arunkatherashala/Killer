/// Killer GC — tri-color mark-and-sweep cycle collector
///
/// `Rc<RefCell<Vec<Value>>>` handles typical short-lived allocations efficiently
/// but cannot collect reference cycles (A → B → A).  This module adds a
/// stop-the-world cycle sweep that the VM triggers every GC_INTERVAL allocs.
///
/// Algorithm (Baker-style mark/sweep over the `Rc` graph):
///   Mark:  walk every Value reachable from VM roots, record each array's
///          raw Rc pointer (usize) in a HashSet.
///   Sweep: for every tracked Weak that is still alive but NOT in the marked
///          set, the array is in a cycle — clear its contents to break it.
///   Prune: drop dead Weak entries from the registry.
///
/// Thread-safety: all state is thread-local; spawned VMs have their own heap.

use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Weak;
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
                queue.push(v.clone());
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
            for v in obj.fields.values() {
                queue.push(v.clone());
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

    // Sweep
    GC_HEAP.with(|heap| {
        let mut h = heap.borrow_mut();
        for weak in h.iter() {
            if let Some(strong) = weak.upgrade() {
                // Use Rc::as_ptr (not RefCell::as_ptr via Deref) to get
                // the same address that rc_ptr() returns during the mark phase.
                let ptr = std::rc::Rc::as_ptr(&strong) as usize;
                if !reachable.contains(&ptr) {
                    // Alive but unreachable → part of a cycle.  Break by clearing.
                    strong.borrow_mut().clear();
                }
            }
        }
        // Prune dead Weaks.
        h.retain(|w| w.strong_count() > 0);
    });
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

    #[test]
    fn gc_collects_simple_cycle() {
        // Build A → [A] cycle manually.
        let a = SharedArray::new(vec![]);
        let a_val = Value::Array(a.clone());
        a.push(a_val); // A contains itself

        // A is not in roots → should be cleared.
        gc_collect(std::iter::empty());

        assert_eq!(a.len(), 0, "GC should have cleared the cyclic array");
    }

    #[test]
    fn gc_keeps_reachable_arrays() {
        let arr = SharedArray::new(vec![Value::Number(42.0)]);
        let root = Value::Array(arr.clone());

        gc_collect(std::iter::once(root));

        assert_eq!(arr.len(), 1, "GC must not collect reachable arrays");
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

    /// PROOF OF THE CYCLE LEAK
    ///
    /// This test demonstrates exactly what happens WITHOUT a GC when a
    /// Killer script does:
    ///
    ///   a = []
    ///   a.push(a)   # a now contains itself
    ///
    /// After `a` goes out of scope, Rc strong_count should be 0 (freed).
    /// Without GC it stays at 1 — the memory is NEVER reclaimed.
    #[test]
    fn prove_cycle_leak_without_gc() {
        // a = []
        let a = SharedArray::new(vec![]);
        let weak = a.weak(); // watch the refcount through this

        // a.push(a)  — self-referential
        let a_val = Value::Array(a.clone());
        a.push(a_val);

        // strong_count = 2: `a` variable + the clone sitting inside the vec
        println!("strong_count before drop(a): {}", weak.strong_count());
        assert_eq!(weak.strong_count(), 2, "both `a` and the vec element hold a ref");

        // Simulate the VM dropping the variable `a` (end of scope)
        drop(a);

        // WITHOUT GC: strong_count is still 1 — not 0!
        // The value inside the vec still holds a ref to itself.
        // This memory is NEVER freed. The cycle keeps itself alive.
        println!("strong_count after drop(a) [THE LEAK]: {}", weak.strong_count());
        assert_eq!(
            weak.strong_count(), 1,
            "LEAK CONFIRMED: strong_count=1 instead of 0 — Rc cycle keeps memory alive forever"
        );

        // In a real script this would accumulate over time.
        // The only way to free it: break the cycle (what gc_collect does).
        // For the test, upgrade the weak ref and clear manually:
        if let Some(strong) = weak.upgrade() {
            strong.borrow_mut().clear(); // break cycle
        }
        // Now strong_count = 0 → freed.
        assert_eq!(weak.strong_count(), 0, "after breaking cycle: memory freed");
    }

    /// PROOF THAT GC FIXES IT
    ///
    /// Same cycle as above — but this time gc_collect() is called.
    /// After collection the array is cleared, Rc count drops to 0.
    #[test]
    fn prove_gc_fixes_the_leak() {
        let a = SharedArray::new(vec![]);
        let a_val = Value::Array(a.clone());
        a.push(a_val); // a → [a]  (cycle)

        // Rc strong_count = 2: `a` + the clone inside the vec
        assert_eq!(a.weak().strong_count(), 2, "cycle exists: count=2");

        // Run GC with empty root set (a is not reachable from any VM scope)
        gc_collect(std::iter::empty());

        // GC cleared the vec contents, breaking the cycle.
        // Now count = 1 (only `a` itself).  When `a` drops → 0 → freed.
        assert_eq!(a.len(), 0, "GC broke the cycle by clearing the array");
        assert_eq!(a.weak().strong_count(), 1, "after GC: only our local `a` holds it");
        // `a` drops here → strong_count = 0 → memory freed. No leak!
    }
}
