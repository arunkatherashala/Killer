#![allow(unsafe_code)]

use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::{Debug, Display, Formatter, Result as FmtResult};
use std::rc::Rc;

/// Reference-counted, interior-mutable array. `Value::clone()` for arrays shares storage so
/// `kfn` updates see the caller's array (Python-style list semantics).
#[derive(Clone)]
pub struct SharedArray(Rc<RefCell<Vec<Value>>>);

impl Debug for SharedArray {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{:?}", *self.0.borrow())
    }
}

impl PartialEq for SharedArray {
    fn eq(&self, other: &Self) -> bool {
        *self.0.borrow() == *other.0.borrow()
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
        self.0.borrow().contains(x)
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

    pub fn to_vec(&self) -> Vec<Value> {
        self.0.borrow().clone()
    }

    pub fn fmt_bracketed(&self, f: &mut Formatter<'_>) -> FmtResult {
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
        match Rc::try_unwrap(self.0) {
            Ok(cell) => cell.into_inner().into_iter(),
            Err(rc) => rc.borrow().clone().into_iter(),
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
    pub fields: HashMap<String, Value>,
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
pub struct SharedDict(Rc<RefCell<crate::fast_hash::FastMap<String, Value>>>);

impl SharedDict {
    pub fn new(map: HashMap<String, Value>) -> Self {
        SharedDict(Rc::new(RefCell::new(map.into_iter().collect())))
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
    /// New, independent dict with the same entries.
    pub fn copy(&self) -> SharedDict {
        SharedDict::new(self.to_map())
    }
    pub fn extend<I: IntoIterator<Item = (String, Value)>>(&self, iter: I) {
        self.0.borrow_mut().extend(iter);
    }
    pub fn borrow(&self) -> std::cell::Ref<'_, crate::fast_hash::FastMap<String, Value>> {
        self.0.borrow()
    }
    pub fn borrow_mut(&self) -> std::cell::RefMut<'_, crate::fast_hash::FastMap<String, Value>> {
        self.0.borrow_mut()
    }
}

impl Debug for SharedDict {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
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
        SharedDict::new(iter.into_iter().collect())
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
    pub fn fields(&self) -> HashMap<String, Value> {
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

#[derive(Debug, Clone, PartialEq)]
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
    Set(Box<std::collections::BTreeSet<SetKey>>),
    Null,
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

impl Display for Value {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            Value::Number(n) => {
                if n.fract() == 0.0 {
                    write!(f, "{}", *n as i64)
                } else {
                    write!(f, "{n}")
                }
            }
            Value::Bool(b) => write!(f, "{b}"),
            Value::Str(s) => write!(f, "{s}"),
            Value::Array(arr) => arr.fmt_bracketed(f),
            Value::Dict(dict) => {
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
            Value::QualityWrapped(quality) => write!(f, "<quality score={:.2}>", quality.get_trim_score()),
            Value::Trit(t) => match t {
                -1 => write!(f, "T_NEG"),
                0  => write!(f, "T_ZERO"),
                1  => write!(f, "T_POS"),
                _  => write!(f, "T_UNKNOWN({})", t),
            },
            Value::Signal { value, confidence, reason } => {
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
            Value::Integer(n) => write!(f, "{}", n),
            Value::Bytes(b) => write!(f, "<bytes[{}]>", b.len()),
            Value::Pointer(p) => write!(f, "0x{:016x}", p),
            Value::Uncertain { value, margin } => write!(f, "{} ± {}", value, margin),
            Value::Gauss { mean, sigma } => write!(f, "{} ± {}σ", mean, sigma),
            Value::Set(s) => {
                let items: Vec<String> = s.iter().map(|k| format!("{}", k.to_value())).collect();
                write!(f, "{{{}}}", items.join(", "))
            }
            Value::Null => write!(f, "null"),
        }
    }
}

impl Value {
    /// Human-readable type name for error messages.
    pub fn type_name(&self) -> &'static str {
        match self {
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
