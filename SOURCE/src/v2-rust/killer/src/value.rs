#![allow(unsafe_code)]

use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::{Debug, Display, Formatter, Result as FmtResult};
use std::rc::Rc;

thread_local! {
    /// Nesting depth of the `Debug` call in progress on this thread.
    static DEBUG_DEPTH: std::cell::Cell<u32> = const { std::cell::Cell::new(0) };
}

/// Keeps `{:?}` of deeply nested or self-referencing containers finite and off the native stack.
struct DebugGuard;

impl DebugGuard {
    fn enter() -> Option<DebugGuard> {
        DEBUG_DEPTH.with(|c| {
            if c.get() >= 64 {
                return None;
            }
            c.set(c.get() + 1);
            Some(DebugGuard)
        })
    }
}

impl Drop for DebugGuard {
    fn drop(&mut self) {
        DEBUG_DEPTH.with(|c| c.set(c.get().saturating_sub(1)));
    }
}

/// Reference-counted, interior-mutable array. `Value::clone()` for arrays shares storage so
/// `kfn` updates see the caller's array (Python-style list semantics).
#[derive(Clone)]
pub struct SharedArray(Rc<RefCell<Vec<Value>>>);

impl Debug for SharedArray {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        let Some(_guard) = DebugGuard::enter() else {
            return write!(f, "[..]");
        };
        match self.0.try_borrow() {
            Ok(v) => write!(f, "{:?}", *v),
            Err(_) => write!(f, "[..]"),
        }
    }
}

impl PartialEq for SharedArray {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0) || *self.0.borrow() == *other.0.borrow()
    }
}

impl SharedArray {
    pub fn new(elements: Vec<Value>) -> Self {
        let arr = Self(Rc::new(RefCell::new(elements)));
        crate::gc::gc_register(&arr);
        arr
    }

    /// Raw pointer of the inner Rc allocation — used as a stable identity by the GC.
    #[inline]
    pub fn rc_ptr(&self) -> usize {
        Rc::as_ptr(&self.0) as usize
    }

    /// Cloned snapshot of elements — used by the GC mark phase without holding a borrow.
    #[inline]
    pub fn clone_elements(&self) -> Vec<Value> {
        self.0.borrow().clone()
    }

    /// Downgraded Weak reference — registered in the GC heap.
    #[inline]
    pub fn weak(&self) -> std::rc::Weak<RefCell<Vec<Value>>> {
        Rc::downgrade(&self.0)
    }

    /// Full structural copy (new buffer). Use when an API must return an independent array.
    pub fn deep_copy(&self) -> Self {
        Self(Rc::new(RefCell::new(self.0.borrow().clone())))
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.0.borrow().len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.0.borrow().is_empty()
    }

    pub fn get(&self, i: usize) -> Option<Value> {
        self.0.borrow().get(i).cloned()
    }

    pub fn set(&self, i: usize, v: Value) {
        if let Some(slot) = self.0.borrow_mut().get_mut(i) {
            *slot = v;
        }
    }

    pub fn push(&self, v: Value) {
        self.0.borrow_mut().push(v);
    }

    pub fn pop(&self) -> Option<Value> {
        self.0.borrow_mut().pop()
    }

    pub fn extend<I: IntoIterator<Item = Value>>(&self, iter: I) {
        self.0.borrow_mut().extend(iter);
    }

    pub fn reverse(&self) {
        self.0.borrow_mut().reverse();
    }

    pub fn sort_by<F>(&self, compare: F)
    where
        F: FnMut(&Value, &Value) -> std::cmp::Ordering,
    {
        self.0.borrow_mut().sort_by(compare);
    }

    pub fn insert(&self, index: usize, element: Value) {
        let len = self.len();
        let i = index.min(len);
        self.0.borrow_mut().insert(i, element);
    }

    /// Remove `count` elements starting at `start`, returning removed values.
    pub fn drain_range(&self, start: usize, end: usize) -> Vec<Value> {
        let mut b = self.0.borrow_mut();
        let len = b.len();
        let s = start.min(len);
        let e = end.min(len);
        if s >= e {
            return Vec::new();
        }
        let drained: Vec<Value> = b.drain(s..e).collect();
        drained
    }

    pub fn contains(&self, x: &Value) -> bool {
        self.0.borrow().iter().any(|e| e.loose_eq(x))
    }

    pub fn join_strings(&self, sep: &str) -> String {
        self.0
            .borrow()
            .iter()
            .map(|v| v.to_string())
            .collect::<Vec<_>>()
            .join(sep)
    }

    pub fn iter_cloned(&self) -> std::vec::IntoIter<Value> {
        self.0.borrow().clone().into_iter()
    }

    /// Snapshot iterator (cloned [`Value`]s), same as [`Self::iter_cloned`].
    pub fn iter(&self) -> std::vec::IntoIter<Value> {
        self.iter_cloned()
    }

    /// Borrow the elements in place (no copy); do not mutate the array while the guard lives.
    pub fn borrow(&self) -> std::cell::Ref<'_, Vec<Value>> {
        self.0.borrow()
    }

    pub fn to_vec(&self) -> Vec<Value> {
        self.0.borrow().clone()
    }

    pub fn fmt_bracketed(&self, f: &mut Formatter<'_>) -> FmtResult {
        let Some(_guard) = DisplayGuard::enter() else {
            return write!(f, "[...]");
        };
        write!(f, "[")?;
        for (i, v) in self.0.borrow().iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }
            write!(f, "{v}")?;
        }
        write!(f, "]")
    }

    /// In-place range extract (for slices). Returns empty if out of range.
    pub fn slice_to_vec(&self, start: usize, end: usize) -> Vec<Value> {
        let b = self.0.borrow();
        if start >= b.len() {
            return Vec::new();
        }
        let e = end.min(b.len());
        b[start..e].to_vec()
    }

    #[inline]
    pub(crate) fn replace_all(&self, new_inner: Vec<Value>) {
        *self.0.borrow_mut() = new_inner;
    }
}

impl From<Vec<Value>> for SharedArray {
    fn from(elements: Vec<Value>) -> Self {
        Self::new(elements)
    }
}

impl FromIterator<Value> for SharedArray {
    fn from_iter<I: IntoIterator<Item = Value>>(iter: I) -> Self {
        Self::new(iter.into_iter().collect())
    }
}

impl<'a> IntoIterator for &'a SharedArray {
    type Item = Value;
    type IntoIter = std::vec::IntoIter<Value>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter_cloned()
    }
}

impl IntoIterator for SharedArray {
    type Item = Value;
    type IntoIter = std::vec::IntoIter<Value>;
    fn into_iter(self) -> Self::IntoIter {
        // (SharedArray has a Drop impl, so the vector is cloned or taken rather than moved out)
        if Rc::strong_count(&self.0) == 1 {
            std::mem::take(&mut *self.0.borrow_mut()).into_iter()
        } else {
            self.0.borrow().clone().into_iter()
        }
    }
}

impl From<Vec<Value>> for Value {
    fn from(elements: Vec<Value>) -> Self {
        Value::Array(SharedArray::new(elements))
    }
}

#[derive(Debug, Clone)]
pub struct Method {
    pub name: String,
    pub params: Vec<String>,
    pub bytecode_start: usize,  // Index in VM's method bytecode array
}

#[derive(Debug, Clone)]
pub struct ClassDef {
    pub name: String,
    pub parent: Option<String>,
    pub methods: HashMap<String, Method>,  // method_name -> Method descriptor
}

#[derive(Debug, Clone)]
pub struct ObjectInstance {
    pub class_name: String,
    pub fields: crate::fast_hash::FastMap<String, Value>,
}

impl PartialEq for Method {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name && self.params == other.params
    }
}

impl PartialEq for ClassDef {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

impl PartialEq for ObjectInstance {
    fn eq(&self, other: &Self) -> bool {
        self.class_name == other.class_name && self.fields == other.fields
    }
}


/// Reference-counted, interior-mutable dictionary. Cloning a `Value::Dict` shares storage, so a
/// dict passed to a function or stored in two variables is the same dict (like arrays, Python and
/// JS), and reading a dict variable is O(1) instead of a deep copy.
#[derive(Clone)]
pub struct SharedDict(Rc<RefCell<crate::ordered_map::OrderedMap<Value>>>);

impl SharedDict {
    /// From an unordered map: entries are laid out in sorted key order so the result is
    /// deterministic. Use [`SharedDict::from_pairs`] when insertion order matters.
    pub fn new(map: HashMap<String, Value>) -> Self {
        let mut pairs: Vec<(String, Value)> = map.into_iter().collect();
        pairs.sort_by(|a, b| a.0.cmp(&b.0));
        SharedDict::from_pairs(pairs)
    }
    /// From pairs in order; a repeated key keeps its first position and its last value.
    pub fn from_pairs(pairs: Vec<(String, Value)>) -> Self {
        SharedDict(Rc::new(RefCell::new(pairs.into_iter().collect())))
    }
    pub fn empty() -> Self {
        SharedDict(Rc::new(RefCell::new(Default::default())))
    }
    #[inline]
    pub fn rc_ptr(&self) -> usize {
        Rc::as_ptr(&self.0) as usize
    }
    #[inline]
    pub fn get(&self, key: &str) -> Option<Value> {
        self.0.borrow().get(key).cloned()
    }
    #[inline]
    pub fn contains_key(&self, key: &str) -> bool {
        self.0.borrow().contains_key(key)
    }
    #[inline]
    pub fn insert(&self, key: String, value: Value) -> Option<Value> {
        self.0.borrow_mut().insert(key, value)
    }
    /// Set `key`, reusing the existing key allocation when it is already present.
    #[inline]
    pub fn set(&self, key: &str, value: Value) {
        let mut m = self.0.borrow_mut();
        match m.get_mut(key) {
            Some(slot) => *slot = value,
            None => {
                m.insert(key.to_string(), value);
            }
        }
    }
    pub fn remove(&self, key: &str) -> Option<Value> {
        self.0.borrow_mut().remove(key)
    }
    pub fn clear(&self) {
        self.0.borrow_mut().clear();
    }
    #[inline]
    pub fn len(&self) -> usize {
        self.0.borrow().len()
    }
    pub fn is_empty(&self) -> bool {
        self.0.borrow().is_empty()
    }
    /// Snapshot of the keys.
    pub fn keys(&self) -> Vec<String> {
        self.0.borrow().keys().cloned().collect()
    }
    /// Snapshot of the values.
    pub fn values(&self) -> Vec<Value> {
        self.0.borrow().values().cloned().collect()
    }
    /// Snapshot of the entries (safe to hold while the dict is modified).
    pub fn iter(&self) -> std::vec::IntoIter<(String, Value)> {
        self.0.borrow().iter().map(|(k, v)| (k.clone(), v.clone())).collect::<Vec<_>>().into_iter()
    }
    /// Copy of the underlying map (values themselves are shared handles where they are arrays/dicts).
    pub fn to_map(&self) -> HashMap<String, Value> {
        self.0.borrow().iter().map(|(k, v)| (k.clone(), v.clone())).collect()
    }
    /// New, independent dict with the same entries in the same order.
    pub fn copy(&self) -> SharedDict {
        SharedDict::from_pairs(self.iter().collect())
    }
    pub fn extend<I: IntoIterator<Item = (String, Value)>>(&self, iter: I) {
        self.0.borrow_mut().extend(iter);
    }
    pub fn borrow(&self) -> std::cell::Ref<'_, crate::ordered_map::OrderedMap<Value>> {
        self.0.borrow()
    }
    pub fn borrow_mut(&self) -> std::cell::RefMut<'_, crate::ordered_map::OrderedMap<Value>> {
        self.0.borrow_mut()
    }
}

impl Debug for SharedDict {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        let Some(_guard) = DebugGuard::enter() else {
            return write!(f, "{{..}}");
        };
        match self.0.try_borrow() {
            Ok(m) => write!(f, "{:?}", *m),
            Err(_) => write!(f, "{{..}}"),
        }
    }
}

impl PartialEq for SharedDict {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0) || *self.0.borrow() == *other.0.borrow()
    }
}

impl From<HashMap<String, Value>> for SharedDict {
    fn from(map: HashMap<String, Value>) -> Self {
        SharedDict::new(map)
    }
}

impl FromIterator<(String, Value)> for SharedDict {
    fn from_iter<I: IntoIterator<Item = (String, Value)>>(iter: I) -> Self {
        SharedDict::from_pairs(iter.into_iter().collect())
    }
}

/// Reference-counted, interior-mutable set. Like arrays and dicts, a set passed around is the same
/// set, so `seen.add(x)` / `set_add(seen, x)` inside a function updates the caller's set.
#[derive(Clone)]
pub struct SharedSet(Rc<RefCell<std::collections::BTreeSet<SetKey>>>);

impl SharedSet {
    pub fn new(set: std::collections::BTreeSet<SetKey>) -> Self {
        SharedSet(Rc::new(RefCell::new(set)))
    }
    pub fn len(&self) -> usize {
        self.0.borrow().len()
    }
    pub fn is_empty(&self) -> bool {
        self.0.borrow().is_empty()
    }
    pub fn contains(&self, key: &SetKey) -> bool {
        self.0.borrow().contains(key)
    }
    pub fn insert(&self, key: SetKey) -> bool {
        self.0.borrow_mut().insert(key)
    }
    pub fn remove(&self, key: &SetKey) -> bool {
        self.0.borrow_mut().remove(key)
    }
    pub fn clear(&self) {
        self.0.borrow_mut().clear();
    }
    /// Snapshot of the members in order.
    pub fn iter(&self) -> std::vec::IntoIter<SetKey> {
        self.0.borrow().iter().cloned().collect::<Vec<_>>().into_iter()
    }
    /// Plain copy of the members.
    pub fn to_btree(&self) -> std::collections::BTreeSet<SetKey> {
        self.0.borrow().clone()
    }
    /// New, independent set with the same members.
    pub fn copy(&self) -> SharedSet {
        SharedSet::new(self.to_btree())
    }
}

impl Debug for SharedSet {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self.0.try_borrow() {
            Ok(s) => write!(f, "{:?}", *s),
            Err(_) => write!(f, "{{..}}"),
        }
    }
}

impl PartialEq for SharedSet {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0) || *self.0.borrow() == *other.0.borrow()
    }
}

/// Reference-counted object instance: methods mutate the same object the caller holds.
#[derive(Clone)]
pub struct SharedObject(Rc<RefCell<ObjectInstance>>);

impl SharedObject {
    pub fn new(inst: ObjectInstance) -> Self {
        SharedObject(Rc::new(RefCell::new(inst)))
    }
    #[inline]
    pub fn rc_ptr(&self) -> usize {
        Rc::as_ptr(&self.0) as usize
    }
    pub fn class_name(&self) -> String {
        self.0.borrow().class_name.clone()
    }
    pub fn get_field(&self, name: &str) -> Option<Value> {
        self.0.borrow().fields.get(name).cloned()
    }
    pub fn has_field(&self, name: &str) -> bool {
        self.0.borrow().fields.contains_key(name)
    }
    pub fn set_field(&self, name: String, value: Value) {
        self.0.borrow_mut().fields.insert(name, value);
    }
    /// Copy of the fields map.
    pub fn fields(&self) -> crate::fast_hash::FastMap<String, Value> {
        self.0.borrow().fields.clone()
    }
    /// Independent copy of the whole instance.
    pub fn snapshot(&self) -> ObjectInstance {
        self.0.borrow().clone()
    }
    pub fn borrow(&self) -> std::cell::Ref<'_, ObjectInstance> {
        self.0.borrow()
    }
    pub fn borrow_mut(&self) -> std::cell::RefMut<'_, ObjectInstance> {
        self.0.borrow_mut()
    }
}

impl Debug for SharedObject {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        let Some(_guard) = DebugGuard::enter() else {
            return write!(f, "<object>");
        };
        match self.0.try_borrow() {
            Ok(o) => write!(f, "{:?}", *o),
            Err(_) => write!(f, "<object>"),
        }
    }
}

impl PartialEq for SharedObject {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0) || *self.0.borrow() == *other.0.borrow()
    }
}

// v2.2: Async future handle -----------------------------------------------
/// Newtype wrapper so Value can still derive PartialEq (futures are never equal)
#[derive(Debug, Clone)]
pub struct FutureHandle(pub std::sync::Arc<std::sync::Mutex<Option<Box<Value>>>>);
impl PartialEq for FutureHandle { fn eq(&self, _: &Self) -> bool { false } }
// Safety: Arc<Mutex<_>> provides interior synchronisation; Value contains no
// thread-local pointers.  All primitive fields (f64/bool/String/Vec/HashMap)
// are Send.  Raw-pointer JIT types live only in VirtualMachine, not in Value.
unsafe impl Send for Value {}
unsafe impl Sync for Value {}
// A spawned task receives its own detached copy of the captures (see SpawnCall), like Value itself.
unsafe impl Send for Captures {}
unsafe impl Send for FutureHandle {}
unsafe impl Sync for FutureHandle {}

/// Variables a closure captured when it was created. Shared between clones of the function value
/// and written back after every call, so a closure keeps its own state (`n = n + 1` persists).
#[derive(Clone, Default)]
pub struct Captures(Rc<RefCell<HashMap<String, Value>>>);

impl Captures {
    pub fn new(map: HashMap<String, Value>) -> Self {
        Captures(Rc::new(RefCell::new(map)))
    }
    pub fn is_empty(&self) -> bool {
        self.0.borrow().is_empty()
    }
    /// Address of the shared capture map (identity for the GC).
    #[inline]
    pub fn rc_ptr(&self) -> usize {
        Rc::as_ptr(&self.0) as *const u8 as usize
    }
    pub fn snapshot(&self) -> Vec<(String, Value)> {
        self.0.borrow().iter().map(|(k, v)| (k.clone(), v.clone())).collect()
    }
    pub fn names(&self) -> Vec<String> {
        self.0.borrow().keys().cloned().collect()
    }
    /// Visit every captured variable without copying the map.
    pub fn each(&self, mut f: impl FnMut(&str, &Value)) {
        for (k, v) in self.0.borrow().iter() {
            f(k, v);
        }
    }
    /// Overwrite each captured value in place with `f(name)` when it returns one (no key allocation).
    pub fn refresh_with(&self, mut f: impl FnMut(&str) -> Option<Value>) {
        for (k, slot) in self.0.borrow_mut().iter_mut() {
            // a shared cell is the variable itself: it is updated in place, never copied back
            if matches!(slot, Value::Cell(_)) {
                continue;
            }
            if let Some(v) = f(k) {
                *slot = v;
            }
        }
    }
    pub fn values(&self) -> Vec<Value> {
        self.0.borrow().values().cloned().collect()
    }
    pub fn set(&self, name: &str, value: Value) {
        self.0.borrow_mut().insert(name.to_string(), value);
    }
}

impl std::fmt::Debug for Captures {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Captures({} vars)", self.0.borrow().len())
    }
}

impl PartialEq for Captures {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

#[derive(Debug, Clone)]
pub enum Value {
    Number(f64),
    Bool(bool),
    Str(String),
    Array(SharedArray),
    Dict(SharedDict),
    Object(SharedObject),
    Class(Box<ClassDef>),
    Function {
        params: Vec<String>,
        bytecode_start: usize,  // Index in VM's function bytecode
        captured: Captures,  // Variables captured from the enclosing function (closures); shared, persists across calls
    },
    Generator(String),  // Generator ID string to track state in VM
    QualityWrapped(Box<crate::data_quality::DataQuality>),  // Wrapped data quality object
    // -- Phase 1: Ternary (Trit) -----------------------------------------------
    // Balanced ternary: -1 = T_NEG (no/false), 0 = T_ZERO (unknown), +1 = T_POS (yes/true)
    Trit(i8),
    // -- Phase 3: Cognitive Signal ---------------------------------------------
    // value + confidence [0.0-1.0] + human-readable reason
    Signal {
        value: Box<Value>,
        confidence: f64,
        reason: String,
    },
    // -- Phase 4: Qubit (quantum simulation) ----------------------------------
    // |ψ⟩ = alpha|0⟩ + beta|1⟩  where |alpha|²+|beta|²=1
    Qubit { alpha: f64, beta: f64 },
    // -- Phase 5: Tryte (6-trit balanced ternary word) ------------------------
    // 6 trits, each -1/0/+1 — 729 states — range -364..+364 — 9.51 bits
    Tryte([i8; 6]),
    // -- v2.2: Async future handle (OS-thread task result) --------------------
    Future(FutureHandle),
    // -- v2.3: OS-level primitive types ----------------------------------------
    /// Fixed-width 64-bit signed integer — for addresses, registers, bitwise ops
    Integer(i64),
    /// Raw byte buffer — for memory regions, disk blocks, network packets
    Bytes(Vec<u8>),
    /// Raw memory pointer (usize) — for hardware MMIO, page tables, DMA
    Pointer(usize),
    /// `believe x = 72 ± 3` — value with known uncertainty margin
    Uncertain { value: f64, margin: f64 },
    /// Statistical uncertainty: independent normally distributed error (`gauss(mean, sigma)`)
    Gauss { mean: f64, sigma: f64 },
    /// Unordered unique-value collection (set semantics)
    Set(SharedSet),
    /// Result slot of a spawned task that failed; `await` turns it back into an error in the
    /// awaiting thread. Never visible to scripts as a value.
    TaskError(Box<Value>),
    /// A shared variable cell ("boxed" variable): a local that a nested function captures and
    /// assigns lives in one of these so every closure and the enclosing call see the same value.
    /// Only ever stored in a scope or in `Captures`; variable loads/stores see through it.
    Cell(Rc<RefCell<Value>>),
    /// Arbitrary-precision integer (`123n`, `big(x)`, `factorial(n)`); see `bigint_ops` for the rules.
    BigInt(Rc<crate::bigint::BigInt>),
    /// A local variable that has not been assigned yet. Never visible to scripts: reading it is an error.
    Unbound,
    Null,
}

// ---------------------------------------------------------------------------------------------
// Structural equality without native recursion
//
// `==` on nested lists, dicts and objects used to be the derived, recursive `PartialEq`: two
// distinct 100,000-deep lists overflowed the native stack, and two cyclic lists never finished.
// Containers are now compared with an explicit work list. Once a comparison has visited many
// container pairs, each pair of container identities is compared only once, so cycles end (a pair
// met again is assumed equal, the usual co-inductive rule) and heavily shared structures
// (`x = [x, x]` forty times) stay linear instead of exponential.
// ---------------------------------------------------------------------------------------------

impl PartialEq for Value {
    #[inline]
    fn eq(&self, other: &Value) -> bool {
        match (self, other) {
            (Value::Number(a), Value::Number(b)) => a == b,
            (Value::Str(a), Value::Str(b)) => a == b,
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::Integer(a), Value::Integer(b)) => a == b,
            (Value::Null, Value::Null) => true,
            _ => deep_eq(self, other),
        }
    }
}

fn is_deep(v: &Value) -> bool {
    matches!(
        v,
        Value::Array(_) | Value::Dict(_) | Value::Object(_) | Value::Signal { .. } | Value::TaskError(_) | Value::Cell(_)
    )
}

/// Equality of two values that are not both plain scalars (see the section comment above).
fn deep_eq(a: &Value, b: &Value) -> bool {
    if !is_deep(a) || !is_deep(b) {
        return leaf_eq(a, b);
    }
    let mut stack: Vec<(Value, Value)> = vec![(a.clone(), b.clone())];
    let mut visited = 0usize;
    let mut seen: std::collections::HashSet<(usize, usize)> = std::collections::HashSet::new();
    while let Some((x, y)) = stack.pop() {
        // false when this pair of container identities was already compared (after a warm-up)
        let mut fresh = |p: usize, q: usize| -> bool {
            visited += 1;
            visited <= 4096 || seen.insert((p, q))
        };
        match (&x, &y) {
            (Value::Array(p), Value::Array(q)) => {
                if Rc::ptr_eq(&p.0, &q.0) || !fresh(p.rc_ptr(), q.rc_ptr()) {
                    continue;
                }
                let (pb, qb) = (p.0.borrow(), q.0.borrow());
                if pb.len() != qb.len() {
                    return false;
                }
                for (i, j) in pb.iter().zip(qb.iter()) {
                    if is_deep(i) && is_deep(j) {
                        stack.push((i.clone(), j.clone()));
                    } else if !i.eq(j) {
                        return false;
                    }
                }
            }
            (Value::Dict(p), Value::Dict(q)) => {
                if Rc::ptr_eq(&p.0, &q.0) || !fresh(p.rc_ptr(), q.rc_ptr()) {
                    continue;
                }
                let (pb, qb) = (p.0.borrow(), q.0.borrow());
                if pb.len() != qb.len() {
                    return false;
                }
                for (k, i) in pb.iter() {
                    match qb.get(k) {
                        None => return false,
                        Some(j) => {
                            if is_deep(i) && is_deep(j) {
                                stack.push((i.clone(), j.clone()));
                            } else if !i.eq(j) {
                                return false;
                            }
                        }
                    }
                }
            }
            (Value::Object(p), Value::Object(q)) => {
                if Rc::ptr_eq(&p.0, &q.0) || !fresh(p.rc_ptr(), q.rc_ptr()) {
                    continue;
                }
                let (pb, qb) = (p.0.borrow(), q.0.borrow());
                if pb.class_name != qb.class_name || pb.fields.len() != qb.fields.len() {
                    return false;
                }
                for (k, i) in pb.fields.iter() {
                    match qb.fields.get(k) {
                        None => return false,
                        Some(j) => {
                            if is_deep(i) && is_deep(j) {
                                stack.push((i.clone(), j.clone()));
                            } else if !i.eq(j) {
                                return false;
                            }
                        }
                    }
                }
            }
            (Value::Signal { value: v1, confidence: c1, reason: r1 }, Value::Signal { value: v2, confidence: c2, reason: r2 }) => {
                if c1 != c2 || r1 != r2 {
                    return false;
                }
                stack.push(((**v1).clone(), (**v2).clone()));
            }
            (Value::TaskError(p), Value::TaskError(q)) => stack.push(((**p).clone(), (**q).clone())),
            (Value::Cell(p), Value::Cell(q)) => {
                if Rc::ptr_eq(p, q) {
                    continue;
                }
                match (p.try_borrow(), q.try_borrow()) {
                    (Ok(i), Ok(j)) => stack.push((i.clone(), j.clone())),
                    _ => return false,
                }
            }
            // a container paired with a different kind of value
            _ => {
                if !leaf_eq(&x, &y) {
                    return false;
                }
            }
        }
    }
    true
}

/// Equality of values that hold no nested container to descend into (a wrapper or container
/// compared with a different kind of value is simply unequal).
fn leaf_eq(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => x == y,
        (Value::Bool(x), Value::Bool(y)) => x == y,
        (Value::Str(x), Value::Str(y)) => x == y,
        (Value::Class(x), Value::Class(y)) => x == y,
        (
            Value::Function { params: p1, bytecode_start: s1, captured: c1 },
            Value::Function { params: p2, bytecode_start: s2, captured: c2 },
        ) => p1 == p2 && s1 == s2 && c1 == c2,
        (Value::Generator(x), Value::Generator(y)) => x == y,
        (Value::QualityWrapped(x), Value::QualityWrapped(y)) => x == y,
        (Value::Trit(x), Value::Trit(y)) => x == y,
        (Value::Qubit { alpha: a1, beta: b1 }, Value::Qubit { alpha: a2, beta: b2 }) => a1 == a2 && b1 == b2,
        (Value::Tryte(x), Value::Tryte(y)) => x == y,
        (Value::Future(x), Value::Future(y)) => x == y,
        (Value::Integer(x), Value::Integer(y)) => x == y,
        (Value::Bytes(x), Value::Bytes(y)) => x == y,
        (Value::Pointer(x), Value::Pointer(y)) => x == y,
        (Value::Uncertain { value: v1, margin: m1 }, Value::Uncertain { value: v2, margin: m2 }) => v1 == v2 && m1 == m2,
        (Value::Gauss { mean: m1, sigma: s1 }, Value::Gauss { mean: m2, sigma: s2 }) => m1 == m2 && s1 == s2,
        (Value::Set(x), Value::Set(y)) => x == y,
        (Value::BigInt(x), Value::BigInt(y)) => x == y,
        (Value::Null, Value::Null) => true,
        _ => false,
    }
}

/// Ordered key for Set — numbers stored as bits (NaN excluded), then strings, then bools
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum SetKey {
    Num(u64),   // f64::to_bits() — NaN not allowed as set member
    Str(String),
    Bool(bool),
}

impl SetKey {
    pub fn from_value(v: &Value) -> Option<SetKey> {
        match v {
            Value::Number(n) if !n.is_nan() => Some(SetKey::Num(n.to_bits())),
            Value::Str(s) => Some(SetKey::Str(s.clone())),
            Value::Bool(b) => Some(SetKey::Bool(*b)),
            Value::Integer(i) => Some(SetKey::Num((*i as f64).to_bits())),
            _ => None,
        }
    }
    pub fn to_value(&self) -> Value {
        match self {
            SetKey::Num(bits) => Value::Number(f64::from_bits(*bits)),
            SetKey::Str(s) => Value::Str(s.clone()),
            SetKey::Bool(b) => Value::Bool(*b),
        }
    }
}

thread_local! {
    /// (nesting depth, remaining container budget) of the `Display` call in progress on this thread.
    static DISPLAY_STATE: std::cell::Cell<(u32, u32)> = const { std::cell::Cell::new((0, 0)) };
}

/// Keeps `Display` of self-referencing or enormously shared structures finite: containers nested
/// deeper than 64 levels, or more than 2 million containers in one print, are shown as `...`.
struct DisplayGuard;

impl DisplayGuard {
    fn enter() -> Option<DisplayGuard> {
        DISPLAY_STATE.with(|c| {
            let (depth, budget) = c.get();
            let budget = if depth == 0 { 2_000_000 } else { budget };
            if depth >= 64 || budget == 0 {
                return None;
            }
            c.set((depth + 1, budget - 1));
            Some(DisplayGuard)
        })
    }
}

impl Drop for DisplayGuard {
    fn drop(&mut self) {
        DISPLAY_STATE.with(|c| {
            let (depth, budget) = c.get();
            c.set((depth.saturating_sub(1), budget));
        });
    }
}

impl Display for Value {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            Value::Unbound => write!(f, "<unassigned>"),
            Value::Number(n) => {
                if n.fract() == 0.0 && n.abs() < 9.0e18 {
                    write!(f, "{}", *n as i64)
                } else {
                    write!(f, "{n}")
                }
            }
            Value::Bool(b) => write!(f, "{b}"),
            Value::Str(s) => write!(f, "{s}"),
            Value::Array(arr) => arr.fmt_bracketed(f),
            Value::Dict(dict) => {
                let Some(_guard) = DisplayGuard::enter() else {
                    return write!(f, "{{...}}");
                };
                write!(f, "{{")?;
                let mut first = true;
                for (k, v) in dict.iter() {
                    if !first {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}: {}", k, v)?;
                    first = false;
                }
                write!(f, "}}")
            }
            Value::Object(obj) => write!(f, "<{} instance>", obj.class_name()),
            Value::Class(class) => write!(f, "<class {}>", class.name),
            Value::Function { params, .. } => write!(f, "<function({})>", params.join(", ")),
            Value::Generator(_) => write!(f, "<generator>"),
            Value::Cell(c) => {
                let Some(_guard) = DisplayGuard::enter() else {
                    return write!(f, "...");
                };
                match c.try_borrow() {
                    Ok(v) => write!(f, "{}", v),
                    Err(_) => write!(f, "..."),
                }
            }
            Value::BigInt(b) => write!(f, "{}", b),
            Value::QualityWrapped(quality) => write!(f, "<quality score={:.2}>", quality.get_trim_score()),
            Value::Trit(t) => match t {
                -1 => write!(f, "T_NEG"),
                0  => write!(f, "T_ZERO"),
                1  => write!(f, "T_POS"),
                _  => write!(f, "T_UNKNOWN({})", t),
            },
            Value::Signal { value, confidence, reason } => {
                let Some(_guard) = DisplayGuard::enter() else {
                    return write!(f, "Signal(...)");
                };
                write!(f, "Signal({}, {:.2}, {})", value, confidence, reason)
            }
            Value::Qubit { alpha, beta } => {
                write!(f, "Qubit({:.4}|0⟩ + {:.4}|1⟩)", alpha, beta)
            }
            Value::Tryte(ts) => {
                let parts: Vec<String> = ts.iter().map(|t| match t {
                    -1 => "-".to_string(),
                     0 => "0".to_string(),
                     1 => "+".to_string(),
                     _ => "?".to_string(),
                }).collect();
                write!(f, "Tryte[{}]", parts.join(""))
            }
            Value::Future(_) => write!(f, "<future>"),
            Value::TaskError(msg) => {
                let Some(_guard) = DisplayGuard::enter() else {
                    return write!(f, "<task error>");
                };
                write!(f, "<task error: {}>", msg)
            }
            Value::Integer(n) => write!(f, "{}", n),
            Value::Bytes(b) => write!(f, "<bytes[{}]>", b.len()),
            Value::Pointer(p) => write!(f, "0x{:016x}", p),
            Value::Uncertain { value, margin } => write!(f, "{} ± {}", value, margin),
            Value::Gauss { mean, sigma } => write!(f, "{} ± {}σ", mean, sigma),
            Value::Set(s) => {
                let Some(_guard) = DisplayGuard::enter() else {
                    return write!(f, "{{...}}");
                };
                let items: Vec<String> = s.iter().map(|k| format!("{}", k.to_value())).collect();
                write!(f, "{{{}}}", items.join(", "))
            }
            Value::Null => write!(f, "null"),
        }
    }
}

/// Pending work of [`Value::detached`]: copy the contents of `.0` into the (empty) copy `.1`.
enum DetachJob {
    Array(SharedArray, SharedArray),
    Dict(SharedDict, SharedDict),
    Object(SharedObject, SharedObject),
    Captures(Captures, Captures),
    Cell(Rc<RefCell<Value>>, Rc<RefCell<Value>>),
}

impl Value {
    /// `==` as the language sees it: structural equality, except that a BigInt equals a
    /// number (or integer) of the same exact value (`5n == 5`, `5n in [5]`).
    pub fn loose_eq(&self, other: &Value) -> bool {
        match (self, other) {
            (Value::BigInt(_), Value::Number(_) | Value::Integer(_))
            | (Value::Number(_) | Value::Integer(_), Value::BigInt(_)) => crate::bigint_ops::equals(self, other),
            _ => self == other,
        }
    }

    /// THE truthiness rule of the language (if/while/not/&&/||/ternary/bool()/filter/all/any).
    /// Every other truthiness site delegates here.
    pub fn truthy(&self) -> bool {
        match self {
            Value::Bool(v) => *v,
            Value::Null | Value::Unbound => false,
            Value::Number(v) => *v != 0.0,
            Value::Str(v) => !v.is_empty(),
            Value::Array(a) => !a.is_empty(),
            Value::Dict(d) => !d.is_empty(),
            Value::Object(_) => true,  // Objects are always truthy
            Value::Class(_) => true,   // Classes are always truthy
            Value::Function { .. } => true,  // Functions are always truthy
            Value::Generator(_) => true,  // Generators are always truthy
            Value::QualityWrapped(_) => true,  // Quality objects are always truthy
            Value::Trit(t) => *t > 0,  // T_POS is truthy
            Value::Signal { value, .. } => value.truthy(),  // Delegate to inner value
            Value::Qubit { alpha, .. } => alpha * alpha >= 0.5,  // P(|0⟩) >= 50%
            Value::Tryte(ts) => ts.iter().any(|&t| t > 0),  // truthy if any T_POS
            Value::Future(_) => true,  // a future handle is truthy
            Value::Integer(n) => *n != 0,
            Value::Bytes(b) => !b.is_empty(),
            Value::Pointer(p) => *p != 0,
            Value::Uncertain { value, margin } => *value > *margin,
            Value::Gauss { mean, sigma } => *mean > crate::uncertain::Z95 * sigma.abs(),
            Value::Set(s) => !s.is_empty(),
            Value::TaskError(_) => true,
            Value::Cell(c) => !matches!(&*c.borrow(), Value::Null | Value::Bool(false)),
            Value::BigInt(b) => !b.is_zero(),
        }
    }

    /// A structurally independent copy that shares no `Rc` with the original. Values handed to
    /// another thread (spawn arguments, captured variables, task results) must be detached,
    /// otherwise two threads would update the same non-atomic reference counts.
    ///
    /// Preserves sharing and cycles *inside* the copy (an array that contains itself copies to an
    /// array that contains its copy), so it terminates on any structure. The walk uses an explicit
    /// work list, so a value nested a million levels deep is copied without touching the native
    /// stack.
    pub fn detached(&self) -> Value {
        let mut memo: HashMap<usize, Value> = HashMap::new();
        let mut work: Vec<DetachJob> = Vec::new();
        let root = self.detach_shell(&mut memo, &mut work, 0);
        while let Some(job) = work.pop() {
            match job {
                DetachJob::Array(src, dst) => {
                    let items: Vec<Value> = src.to_vec().iter().map(|v| v.detach_shell(&mut memo, &mut work, 0)).collect();
                    dst.replace_all(items);
                }
                DetachJob::Dict(src, dst) => {
                    for (k, v) in src.iter() {
                        let c = v.detach_shell(&mut memo, &mut work, 0);
                        dst.insert(k, c);
                    }
                }
                DetachJob::Object(src, dst) => {
                    let fields: Vec<(String, Value)> =
                        src.0.borrow().fields.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
                    let fields: crate::fast_hash::FastMap<String, Value> = fields
                        .into_iter()
                        .map(|(k, v)| (k, v.detach_shell(&mut memo, &mut work, 0)))
                        .collect();
                    dst.0.borrow_mut().fields = fields;
                }
                DetachJob::Captures(src, dst) => {
                    for (k, v) in src.snapshot() {
                        let c = v.detach_shell(&mut memo, &mut work, 0);
                        dst.set(&k, c);
                    }
                }
                DetachJob::Cell(src, dst) => {
                    let inner = src.borrow().clone();
                    let c = inner.detach_shell(&mut memo, &mut work, 0);
                    *dst.borrow_mut() = c;
                }
            }
        }
        root
    }

    /// The copy of `self` with containers left empty: each new container is queued in `work` to be
    /// filled in later. Wrapper values (`Signal`, `TaskError`) nest at most 1000 deep (deeper is
    /// cut with `null`), since they are copied on the way in.
    fn detach_shell(&self, memo: &mut HashMap<usize, Value>, work: &mut Vec<DetachJob>, depth: usize) -> Value {
        match self {
            Value::Array(a) => {
                let key = Rc::as_ptr(&a.0) as *const u8 as usize;
                if let Some(done) = memo.get(&key) {
                    return done.clone();
                }
                let copy = SharedArray::new(Vec::new());
                memo.insert(key, Value::Array(copy.clone()));
                work.push(DetachJob::Array(a.clone(), copy.clone()));
                Value::Array(copy)
            }
            Value::Dict(d) => {
                let key = d.rc_ptr();
                if let Some(done) = memo.get(&key) {
                    return done.clone();
                }
                let copy = SharedDict::empty();
                memo.insert(key, Value::Dict(copy.clone()));
                work.push(DetachJob::Dict(d.clone(), copy.clone()));
                Value::Dict(copy)
            }
            Value::Object(o) => {
                let key = Rc::as_ptr(&o.0) as *const u8 as usize;
                if let Some(done) = memo.get(&key) {
                    return done.clone();
                }
                let class_name = o.0.borrow().class_name.clone();
                let copy = SharedObject::new(ObjectInstance { class_name, fields: Default::default() });
                memo.insert(key, Value::Object(copy.clone()));
                work.push(DetachJob::Object(o.clone(), copy.clone()));
                Value::Object(copy)
            }
            Value::Function { params, bytecode_start, captured } => {
                let key = Rc::as_ptr(&captured.0) as *const u8 as usize;
                let copy = if let Some(Value::Function { captured: done, .. }) = memo.get(&key) {
                    done.clone()
                } else {
                    let copy = Captures::new(HashMap::new());
                    memo.insert(
                        key,
                        Value::Function { params: Vec::new(), bytecode_start: 0, captured: copy.clone() },
                    );
                    work.push(DetachJob::Captures(captured.clone(), copy.clone()));
                    copy
                };
                Value::Function { params: params.clone(), bytecode_start: *bytecode_start, captured: copy }
            }
            Value::Signal { value, confidence, reason } => {
                if depth >= 1000 {
                    return Value::Null;
                }
                Value::Signal {
                    value: Box::new(value.detach_shell(memo, work, depth + 1)),
                    confidence: *confidence,
                    reason: reason.clone(),
                }
            }
            Value::TaskError(inner) => {
                if depth >= 1000 {
                    return Value::Null;
                }
                Value::TaskError(Box::new(inner.detach_shell(memo, work, depth + 1)))
            }
            Value::Set(s) => Value::Set(s.copy()),
            Value::Cell(c) => {
                let key = Rc::as_ptr(c) as *const u8 as usize;
                if let Some(done) = memo.get(&key) {
                    return done.clone();
                }
                let copy = Rc::new(RefCell::new(Value::Null));
                memo.insert(key, Value::Cell(copy.clone()));
                work.push(DetachJob::Cell(c.clone(), copy.clone()));
                Value::Cell(copy)
            }
            other => other.clone(),
        }
    }

    /// Human-readable type name for error messages.
    pub fn type_name(&self) -> &'static str {
        match self {
            Value::Unbound => "unassigned",
            Value::Number(_) => "number",
            Value::Bool(_) => "bool",
            Value::Str(_) => "string",
            Value::Array(_) => "array",
            Value::Dict(_) => "dict",
            Value::Object(_) => "object",
            Value::Class(_) => "class",
            Value::Function { .. } => "function",
            Value::Generator(_) => "generator",
            Value::Trit(_) => "trit",
            Value::Signal { .. } => "signal",
            Value::Qubit { .. } => "qubit",
            Value::Tryte(_) => "tryte",
            Value::Future(_) => "future",
            Value::TaskError(_) => "error",
            Value::Cell(_) => "cell",
            Value::BigInt(_) => "bigint",
            Value::Integer(_) => "integer",
            Value::Bytes(_) => "bytes",
            Value::Pointer(_) => "pointer",
            Value::Uncertain { .. } => "uncertain",
            Value::Gauss { .. } => "gauss",
            Value::Set(_) => "set",
            Value::QualityWrapped(_) => "quality",
            Value::Null => "null",
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Iterative drop of nested containers
//
// Dropping `Rc<RefCell<Vec<Value>>>` recursively frees children from inside the parent's `Drop`,
// so a list nested 2000 levels deep (a linked list, a deep AST, `x = [x]` in a loop) overflowed
// the native stack when it was freed and killed the process. When the last owner of a container
// goes away its children are moved onto an explicit work list instead.
// ---------------------------------------------------------------------------------------------

fn is_container(v: &Value) -> bool {
    matches!(v, Value::Array(_) | Value::Dict(_) | Value::Object(_))
}

fn unwind_children(mut stack: Vec<Value>) {
    while let Some(v) = stack.pop() {
        match v {
            Value::Array(a) => {
                if Rc::strong_count(&a.0) == 1 {
                    if let Ok(mut b) = a.0.try_borrow_mut() {
                        stack.extend(std::mem::take(&mut *b));
                    }
                }
            }
            Value::Dict(d) => {
                if Rc::strong_count(&d.0) == 1 {
                    if let Ok(mut b) = d.0.try_borrow_mut() {
                        stack.extend(b.take_values());
                    }
                }
            }
            Value::Object(o) => {
                if Rc::strong_count(&o.0) == 1 {
                    if let Ok(mut b) = o.0.try_borrow_mut() {
                        stack.extend(b.fields.drain().map(|(_, v)| v));
                    }
                }
            }
            _ => {}
        }
    }
}

impl Drop for SharedArray {
    fn drop(&mut self) {
        if Rc::strong_count(&self.0) == 1 {
            if let Ok(mut b) = self.0.try_borrow_mut() {
                if b.iter().any(is_container) {
                    unwind_children(std::mem::take(&mut *b));
                }
            }
        }
    }
}

impl Drop for SharedDict {
    fn drop(&mut self) {
        if Rc::strong_count(&self.0) == 1 {
            if let Ok(mut b) = self.0.try_borrow_mut() {
                if b.values().any(is_container) {
                    unwind_children(b.take_values());
                }
            }
        }
    }
}

impl Drop for SharedObject {
    fn drop(&mut self) {
        if Rc::strong_count(&self.0) == 1 {
            if let Ok(mut b) = self.0.try_borrow_mut() {
                if b.fields.values().any(is_container) {
                    unwind_children(b.fields.drain().map(|(_, v)| v).collect());
                }
            }
        }
    }
}
