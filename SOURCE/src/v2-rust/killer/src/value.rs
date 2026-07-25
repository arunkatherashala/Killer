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
        Self(Rc::new(RefCell::new(elements)))
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
unsafe impl Send for FutureHandle {}
unsafe impl Sync for FutureHandle {}

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Number(f64),
    Bool(bool),
    Str(String),
    Array(SharedArray),
    Dict(Box<HashMap<String, Value>>),
    Object(Box<ObjectInstance>),
    Class(Box<ClassDef>),
    Function {
        params: Vec<String>,
        bytecode_start: usize,  // Index in VM's function bytecode
        captured: Box<HashMap<String, Value>>,  // Variables captured from outer scope (closures)
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
    Null,
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
            Value::Object(obj) => write!(f, "<{} instance>", obj.class_name),
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
            Value::Null => write!(f, "null"),
        }
    }
}
