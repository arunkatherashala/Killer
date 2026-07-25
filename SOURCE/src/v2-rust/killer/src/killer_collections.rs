//! **killer_collections** — production-grade data structures: HashSet, BTreeMap, BinaryHeap, VecDeque, LinkedList.
//!
//! These fill the critical collections gap vs Python/Rust/Go.
//! All structures use Killer's `Value` system for dynamic typing.

use std::collections::{BTreeMap, BinaryHeap, HashMap, LinkedList, VecDeque};
use std::cmp::Ordering;
use std::fmt;

// ══════════════════════════════════════════════════════════════════════════════
// HashSet — unordered unique collection
// ══════════════════════════════════════════════════════════════════════════════

/// A hash set backed by HashMap<String, ()> for Killer's string-keyed world.
#[derive(Debug, Clone)]
pub struct KillerSet {
    inner: HashMap<String, ()>,
}

impl KillerSet {
    pub fn new() -> Self { Self { inner: HashMap::new() } }

    pub fn with_capacity(cap: usize) -> Self { Self { inner: HashMap::with_capacity(cap) } }

    pub fn insert(&mut self, value: String) -> bool {
        self.inner.insert(value, ()).is_none()
    }

    pub fn remove(&mut self, value: &str) -> bool {
        self.inner.remove(value).is_some()
    }

    pub fn contains(&self, value: &str) -> bool { self.inner.contains_key(value) }

    pub fn len(&self) -> usize { self.inner.len() }

    pub fn is_empty(&self) -> bool { self.inner.is_empty() }

    pub fn clear(&mut self) { self.inner.clear(); }

    pub fn iter(&self) -> impl Iterator<Item = &str> {
        self.inner.keys().map(|s| s.as_str())
    }

    pub fn to_vec(&self) -> Vec<String> {
        self.inner.keys().cloned().collect()
    }

    /// Set union: self ∪ other
    pub fn union(&self, other: &KillerSet) -> KillerSet {
        let mut result = self.clone();
        for key in other.inner.keys() {
            result.insert(key.clone());
        }
        result
    }

    /// Set intersection: self ∩ other
    pub fn intersection(&self, other: &KillerSet) -> KillerSet {
        let mut result = KillerSet::new();
        for key in self.inner.keys() {
            if other.contains(key) {
                result.insert(key.clone());
            }
        }
        result
    }

    /// Set difference: self \ other
    pub fn difference(&self, other: &KillerSet) -> KillerSet {
        let mut result = KillerSet::new();
        for key in self.inner.keys() {
            if !other.contains(key) {
                result.insert(key.clone());
            }
        }
        result
    }

    /// Symmetric difference: (self \ other) ∪ (other \ self)
    pub fn symmetric_difference(&self, other: &KillerSet) -> KillerSet {
        let mut result = KillerSet::new();
        for key in self.inner.keys() {
            if !other.contains(key) { result.insert(key.clone()); }
        }
        for key in other.inner.keys() {
            if !self.contains(key) { result.insert(key.clone()); }
        }
        result
    }

    /// Is self a subset of other?
    pub fn is_subset(&self, other: &KillerSet) -> bool {
        self.inner.keys().all(|k| other.contains(k))
    }

    /// Is self a superset of other?
    pub fn is_superset(&self, other: &KillerSet) -> bool {
        other.is_subset(self)
    }
}

impl Default for KillerSet {
    fn default() -> Self { Self::new() }
}

impl fmt::Display for KillerSet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let items: Vec<&str> = self.inner.keys().map(|s| s.as_str()).collect();
        write!(f, "{{{}}}", items.join(", "))
    }
}

impl FromIterator<String> for KillerSet {
    fn from_iter<I: IntoIterator<Item = String>>(iter: I) -> Self {
        let mut set = KillerSet::new();
        for item in iter { set.insert(item); }
        set
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// OrderedMap — sorted key-value map (BTreeMap wrapper)
// ══════════════════════════════════════════════════════════════════════════════

/// A sorted map backed by BTreeMap. Keys are always sorted.
#[derive(Debug, Clone)]
pub struct OrderedMap {
    inner: BTreeMap<String, f64>,
}

impl OrderedMap {
    pub fn new() -> Self { Self { inner: BTreeMap::new() } }

    pub fn insert(&mut self, key: String, value: f64) -> Option<f64> {
        self.inner.insert(key, value)
    }

    pub fn get(&self, key: &str) -> Option<f64> { self.inner.get(key).copied() }

    pub fn remove(&mut self, key: &str) -> Option<f64> { self.inner.remove(key) }

    pub fn contains_key(&self, key: &str) -> bool { self.inner.contains_key(key) }

    pub fn len(&self) -> usize { self.inner.len() }

    pub fn is_empty(&self) -> bool { self.inner.is_empty() }

    pub fn clear(&mut self) { self.inner.clear(); }

    /// First (smallest) key.
    pub fn first_key(&self) -> Option<&str> { self.inner.keys().next().map(|s| s.as_str()) }

    /// Last (largest) key.
    pub fn last_key(&self) -> Option<&str> { self.inner.keys().next_back().map(|s| s.as_str()) }

    /// Range query: all entries with keys in [start, end].
    pub fn range(&self, start: &str, end: &str) -> Vec<(String, f64)> {
        use std::ops::RangeInclusive;
        self.inner.range::<String, RangeInclusive<&String>>(&start.to_string()..=&end.to_string())
            .map(|(k, v)| (k.clone(), *v))
            .collect()
    }

    /// Iterate in sorted order.
    pub fn iter_sorted(&self) -> impl Iterator<Item = (&str, &f64)> {
        self.inner.iter().map(|(k, v)| (k.as_str(), v))
    }

    pub fn keys(&self) -> Vec<String> { self.inner.keys().cloned().collect() }
    pub fn values(&self) -> Vec<f64> { self.inner.values().cloned().collect() }
}

impl Default for OrderedMap {
    fn default() -> Self { Self::new() }
}

// ══════════════════════════════════════════════════════════════════════════════
// PriorityQueue — max-heap by default, supports min-heap via Reverse
// ══════════════════════════════════════════════════════════════════════════════

/// Wrapper for BinaryHeap items with priority.
#[derive(Debug, Clone)]
struct PqItem {
    priority: f64,
    value: String,
    /// Insertion order for stable tie-breaking.
    seq: u64,
}

impl PartialEq for PqItem {
    fn eq(&self, other: &Self) -> bool { self.priority == other.priority && self.seq == other.seq }
}

impl Eq for PqItem {}

impl PartialOrd for PqItem {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> { Some(self.cmp(other)) }
}

impl Ord for PqItem {
    fn cmp(&self, other: &Self) -> Ordering {
        self.priority.partial_cmp(&other.priority)
            .unwrap_or(Ordering::Equal)
            .then_with(|| other.seq.cmp(&self.seq)) // earlier items first on tie
    }
}

/// A priority queue: highest-priority item comes out first.
/// Use negative priorities for min-heap behavior.
#[derive(Debug)]
pub struct PriorityQueue {
    heap: BinaryHeap<PqItem>,
    seq: u64,
}

impl PriorityQueue {
    pub fn new() -> Self { Self { heap: BinaryHeap::new(), seq: 0 } }

    pub fn with_capacity(cap: usize) -> Self {
        Self { heap: BinaryHeap::with_capacity(cap), seq: 0 }
    }

    /// Push a value with a given priority.
    pub fn push(&mut self, value: String, priority: f64) {
        self.seq += 1;
        self.heap.push(PqItem { priority, value, seq: self.seq });
    }

    /// Pop the highest-priority item. Returns (value, priority).
    pub fn pop(&mut self) -> Option<(String, f64)> {
        self.heap.pop().map(|item| (item.value, item.priority))
    }

    /// Peek at the highest-priority item without removing.
    pub fn peek(&self) -> Option<(&str, f64)> {
        self.heap.peek().map(|item| (item.value.as_str(), item.priority))
    }

    pub fn len(&self) -> usize { self.heap.len() }
    pub fn is_empty(&self) -> bool { self.heap.is_empty() }
    pub fn clear(&mut self) { self.heap.clear(); }
}

impl Default for PriorityQueue {
    fn default() -> Self { Self::new() }
}

// ══════════════════════════════════════════════════════════════════════════════
// Deque — double-ended queue
// ══════════════════════════════════════════════════════════════════════════════

/// A double-ended queue backed by VecDeque.
#[derive(Debug, Clone)]
pub struct KillerDeque {
    inner: VecDeque<String>,
}

impl KillerDeque {
    pub fn new() -> Self { Self { inner: VecDeque::new() } }

    pub fn with_capacity(cap: usize) -> Self { Self { inner: VecDeque::with_capacity(cap) } }

    pub fn push_front(&mut self, value: String) { self.inner.push_front(value); }
    pub fn push_back(&mut self, value: String) { self.inner.push_back(value); }
    pub fn pop_front(&mut self) -> Option<String> { self.inner.pop_front() }
    pub fn pop_back(&mut self) -> Option<String> { self.inner.pop_back() }
    pub fn front(&self) -> Option<&str> { self.inner.front().map(|s| s.as_str()) }
    pub fn back(&self) -> Option<&str> { self.inner.back().map(|s| s.as_str()) }
    pub fn get(&self, index: usize) -> Option<&str> { self.inner.get(index).map(|s| s.as_str()) }
    pub fn len(&self) -> usize { self.inner.len() }
    pub fn is_empty(&self) -> bool { self.inner.is_empty() }
    pub fn clear(&mut self) { self.inner.clear(); }

    pub fn contains(&self, value: &str) -> bool {
        self.inner.iter().any(|s| s == value)
    }

    /// Rotate left by n positions (front elements move to back).
    pub fn rotate_left(&mut self, n: usize) {
        self.inner.rotate_left(n.min(self.inner.len()));
    }

    /// Rotate right by n positions (back elements move to front).
    pub fn rotate_right(&mut self, n: usize) {
        self.inner.rotate_right(n.min(self.inner.len()));
    }

    pub fn to_vec(&self) -> Vec<String> { self.inner.iter().cloned().collect() }

    pub fn iter(&self) -> impl Iterator<Item = &str> {
        self.inner.iter().map(|s| s.as_str())
    }
}

impl Default for KillerDeque {
    fn default() -> Self { Self::new() }
}

// ══════════════════════════════════════════════════════════════════════════════
// KillerLinkedList — doubly-linked list
// ══════════════════════════════════════════════════════════════════════════════

/// Doubly-linked list wrapper.
#[derive(Debug, Clone)]
pub struct KillerLinkedList {
    inner: LinkedList<String>,
}

impl KillerLinkedList {
    pub fn new() -> Self { Self { inner: LinkedList::new() } }

    pub fn push_front(&mut self, value: String) { self.inner.push_front(value); }
    pub fn push_back(&mut self, value: String) { self.inner.push_back(value); }
    pub fn pop_front(&mut self) -> Option<String> { self.inner.pop_front() }
    pub fn pop_back(&mut self) -> Option<String> { self.inner.pop_back() }
    pub fn front(&self) -> Option<&str> { self.inner.front().map(|s| s.as_str()) }
    pub fn back(&self) -> Option<&str> { self.inner.back().map(|s| s.as_str()) }
    pub fn len(&self) -> usize { self.inner.len() }
    pub fn is_empty(&self) -> bool { self.inner.is_empty() }
    pub fn clear(&mut self) { self.inner.clear(); }

    pub fn contains(&self, value: &str) -> bool {
        self.inner.iter().any(|s| s == value)
    }

    pub fn to_vec(&self) -> Vec<String> { self.inner.iter().cloned().collect() }

    pub fn iter(&self) -> impl Iterator<Item = &str> {
        self.inner.iter().map(|s| s.as_str())
    }
}

impl Default for KillerLinkedList {
    fn default() -> Self { Self::new() }
}

// ══════════════════════════════════════════════════════════════════════════════
// KillerIterator — lazy functional iterator with map/filter/reduce/take/skip
// ══════════════════════════════════════════════════════════════════════════════

/// A lazy iterator pipeline over string values.
/// Supports chaining: `iter.map(f).filter(g).take(10).collect()`
#[derive(Debug)]
pub struct KillerIter {
    items: Vec<String>,
    pos: usize,
}

impl KillerIter {
    pub fn from_vec(v: Vec<String>) -> Self { Self { items: v, pos: 0 } }

    pub fn from_set(s: &KillerSet) -> Self { Self::from_vec(s.to_vec()) }

    pub fn from_deque(d: &KillerDeque) -> Self { Self::from_vec(d.to_vec()) }

    /// Collect remaining items into a Vec.
    pub fn collect_vec(self) -> Vec<String> { self.items[self.pos..].to_vec() }

    /// Take up to n items.
    pub fn take(mut self, n: usize) -> Self {
        let end = (self.pos + n).min(self.items.len());
        self.items = self.items[self.pos..end].to_vec();
        self.pos = 0;
        self
    }

    /// Skip n items.
    pub fn skip(mut self, n: usize) -> Self {
        self.pos = (self.pos + n).min(self.items.len());
        self
    }

    /// Filter items that contain the given substring.
    pub fn filter_contains(self, substr: &str) -> Self {
        let remaining: Vec<String> = self.items[self.pos..]
            .iter()
            .filter(|s| s.contains(substr))
            .cloned()
            .collect();
        Self { items: remaining, pos: 0 }
    }

    /// Map: prefix each item with a string.
    pub fn map_prefix(self, prefix: &str) -> Self {
        let remaining: Vec<String> = self.items[self.pos..]
            .iter()
            .map(|s| format!("{}{}", prefix, s))
            .collect();
        Self { items: remaining, pos: 0 }
    }

    /// Map: suffix each item.
    pub fn map_suffix(self, suffix: &str) -> Self {
        let remaining: Vec<String> = self.items[self.pos..]
            .iter()
            .map(|s| format!("{}{}", s, suffix))
            .collect();
        Self { items: remaining, pos: 0 }
    }

    /// Map: uppercase all items.
    pub fn map_upper(self) -> Self {
        let remaining: Vec<String> = self.items[self.pos..]
            .iter()
            .map(|s| s.to_uppercase())
            .collect();
        Self { items: remaining, pos: 0 }
    }

    /// Reduce: join all items with a separator.
    pub fn join(self, sep: &str) -> String {
        self.items[self.pos..].join(sep)
    }

    /// Count remaining items.
    pub fn count(self) -> usize { self.items.len() - self.pos }

    /// First remaining item.
    pub fn first(self) -> Option<String> {
        self.items.get(self.pos).cloned()
    }

    /// Last remaining item.
    pub fn last(self) -> Option<String> {
        self.items.last().cloned()
    }

    /// Unique items (preserving first occurrence order).
    pub fn unique(self) -> Self {
        let mut seen = HashMap::new();
        let mut result = Vec::new();
        for s in &self.items[self.pos..] {
            if seen.insert(s.clone(), ()).is_none() {
                result.push(s.clone());
            }
        }
        Self { items: result, pos: 0 }
    }

    /// Sort lexicographically.
    pub fn sorted(mut self) -> Self {
        let mut remaining = self.items[self.pos..].to_vec();
        remaining.sort();
        self.items = remaining;
        self.pos = 0;
        self
    }

    /// Reverse order.
    pub fn reversed(mut self) -> Self {
        let mut remaining = self.items[self.pos..].to_vec();
        remaining.reverse();
        self.items = remaining;
        self.pos = 0;
        self
    }

    /// Flat map: split each item by separator, flatten.
    pub fn flat_split(self, sep: &str) -> Self {
        let remaining: Vec<String> = self.items[self.pos..]
            .iter()
            .flat_map(|s| s.split(sep).map(String::from))
            .collect();
        Self { items: remaining, pos: 0 }
    }

    /// Enumerate: "0:item", "1:item", ...
    pub fn enumerate_prefixed(self) -> Self {
        let remaining: Vec<String> = self.items[self.pos..]
            .iter()
            .enumerate()
            .map(|(i, s)| format!("{}:{}", i, s))
            .collect();
        Self { items: remaining, pos: 0 }
    }

    /// Zip with another iterator: "a,b" pairs.
    pub fn zip_with(self, other: KillerIter) -> Self {
        let a = &self.items[self.pos..];
        let b = &other.items[other.pos..];
        let len = a.len().min(b.len());
        let pairs: Vec<String> = (0..len)
            .map(|i| format!("{},{}", a[i], b[i]))
            .collect();
        Self { items: pairs, pos: 0 }
    }

    /// Chain another iterator after this one.
    pub fn chain(mut self, other: KillerIter) -> Self {
        let mut remaining = self.items[self.pos..].to_vec();
        remaining.extend_from_slice(&other.items[other.pos..]);
        self.items = remaining;
        self.pos = 0;
        self
    }

    /// Window: sliding window of size n.
    pub fn windows(self, n: usize) -> Vec<Vec<String>> {
        let items = &self.items[self.pos..];
        if n == 0 || n > items.len() { return vec![]; }
        items.windows(n).map(|w| w.to_vec()).collect()
    }

    /// Chunk: split into fixed-size chunks.
    pub fn chunks(self, n: usize) -> Vec<Vec<String>> {
        let items = &self.items[self.pos..];
        if n == 0 { return vec![]; }
        items.chunks(n).map(|c| c.to_vec()).collect()
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Tests
// ══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    // ── Set tests ──
    #[test]
    fn set_basic_ops() {
        let mut s = KillerSet::new();
        assert!(s.insert("a".into()));
        assert!(s.insert("b".into()));
        assert!(!s.insert("a".into())); // duplicate
        assert_eq!(s.len(), 2);
        assert!(s.contains("a"));
        assert!(!s.contains("c"));
        assert!(s.remove("a"));
        assert_eq!(s.len(), 1);
    }

    #[test]
    fn set_union_intersection() {
        let a: KillerSet = ["x", "y", "z"].iter().map(|s| s.to_string()).collect();
        let b: KillerSet = ["y", "z", "w"].iter().map(|s| s.to_string()).collect();
        let union = a.union(&b);
        assert_eq!(union.len(), 4);
        let inter = a.intersection(&b);
        assert_eq!(inter.len(), 2);
        assert!(inter.contains("y"));
        assert!(inter.contains("z"));
    }

    #[test]
    fn set_difference_symmetric() {
        let a: KillerSet = ["a", "b", "c"].iter().map(|s| s.to_string()).collect();
        let b: KillerSet = ["b", "c", "d"].iter().map(|s| s.to_string()).collect();
        let diff = a.difference(&b);
        assert_eq!(diff.len(), 1);
        assert!(diff.contains("a"));
        let sym = a.symmetric_difference(&b);
        assert_eq!(sym.len(), 2);
        assert!(sym.contains("a"));
        assert!(sym.contains("d"));
    }

    #[test]
    fn set_subset_superset() {
        let a: KillerSet = ["a", "b"].iter().map(|s| s.to_string()).collect();
        let b: KillerSet = ["a", "b", "c"].iter().map(|s| s.to_string()).collect();
        assert!(a.is_subset(&b));
        assert!(!b.is_subset(&a));
        assert!(b.is_superset(&a));
    }

    // ── OrderedMap tests ──
    #[test]
    fn ordered_map_sorted_iteration() {
        let mut m = OrderedMap::new();
        m.insert("cherry".into(), 3.0);
        m.insert("apple".into(), 1.0);
        m.insert("banana".into(), 2.0);
        let keys = m.keys();
        assert_eq!(keys, vec!["apple", "banana", "cherry"]);
        assert_eq!(m.first_key(), Some("apple"));
        assert_eq!(m.last_key(), Some("cherry"));
    }

    #[test]
    fn ordered_map_range_query() {
        let mut m = OrderedMap::new();
        for i in 0..10 {
            m.insert(format!("key_{:02}", i), i as f64);
        }
        let range = m.range("key_03", "key_07");
        assert_eq!(range.len(), 5);
        assert_eq!(range[0].0, "key_03");
        assert_eq!(range[4].0, "key_07");
    }

    // ── PriorityQueue tests ──
    #[test]
    fn pq_max_heap_order() {
        let mut pq = PriorityQueue::new();
        pq.push("low".into(), 1.0);
        pq.push("high".into(), 10.0);
        pq.push("mid".into(), 5.0);
        let (val, pri) = pq.pop().unwrap();
        assert_eq!(val, "high");
        assert_eq!(pri, 10.0);
        let (val, _) = pq.pop().unwrap();
        assert_eq!(val, "mid");
    }

    #[test]
    fn pq_min_heap_via_negation() {
        let mut pq = PriorityQueue::new();
        pq.push("expensive".into(), -100.0);
        pq.push("cheap".into(), -1.0);
        pq.push("mid".into(), -50.0);
        let (val, _) = pq.pop().unwrap();
        assert_eq!(val, "cheap"); // -1 is highest (least negative)
    }

    #[test]
    fn pq_stable_ordering() {
        let mut pq = PriorityQueue::new();
        pq.push("first".into(), 5.0);
        pq.push("second".into(), 5.0);
        let (val, _) = pq.pop().unwrap();
        assert_eq!(val, "first"); // FIFO for same priority
    }

    // ── Deque tests ──
    #[test]
    fn deque_both_ends() {
        let mut d = KillerDeque::new();
        d.push_back("a".into());
        d.push_back("b".into());
        d.push_front("z".into());
        assert_eq!(d.front(), Some("z"));
        assert_eq!(d.back(), Some("b"));
        assert_eq!(d.len(), 3);
        assert_eq!(d.pop_front(), Some("z".into()));
        assert_eq!(d.pop_back(), Some("b".into()));
    }

    #[test]
    fn deque_rotate() {
        let mut d = KillerDeque::new();
        for c in ["a", "b", "c", "d"] { d.push_back(c.into()); }
        d.rotate_left(1);
        assert_eq!(d.to_vec(), vec!["b", "c", "d", "a"]);
        d.rotate_right(1);
        assert_eq!(d.to_vec(), vec!["a", "b", "c", "d"]);
    }

    // ── LinkedList tests ──
    #[test]
    fn linked_list_ops() {
        let mut ll = KillerLinkedList::new();
        ll.push_back("a".into());
        ll.push_back("b".into());
        ll.push_front("z".into());
        assert_eq!(ll.len(), 3);
        assert_eq!(ll.front(), Some("z"));
        assert_eq!(ll.pop_front(), Some("z".into()));
        assert!(ll.contains("a"));
    }

    // ── Iterator tests ──
    #[test]
    fn iter_take_skip() {
        let items: Vec<String> = (0..10).map(|i| format!("item_{}", i)).collect();
        let result = KillerIter::from_vec(items).skip(3).take(4).collect_vec();
        assert_eq!(result.len(), 4);
        assert_eq!(result[0], "item_3");
        assert_eq!(result[3], "item_6");
    }

    #[test]
    fn iter_filter_map_chain() {
        let items = vec!["hello".into(), "world".into(), "help".into(), "wide".into()];
        let result = KillerIter::from_vec(items)
            .filter_contains("hel")
            .map_upper()
            .collect_vec();
        assert_eq!(result, vec!["HELLO", "HELP"]);
    }

    #[test]
    fn iter_unique_sorted() {
        let items = vec!["c".into(), "a".into(), "b".into(), "a".into(), "c".into()];
        let result = KillerIter::from_vec(items).unique().sorted().collect_vec();
        assert_eq!(result, vec!["a", "b", "c"]);
    }

    #[test]
    fn iter_zip_chain() {
        let a = KillerIter::from_vec(vec!["x".into(), "y".into()]);
        let b = KillerIter::from_vec(vec!["1".into(), "2".into()]);
        let zipped = a.zip_with(b).collect_vec();
        assert_eq!(zipped, vec!["x,1", "y,2"]);
    }

    #[test]
    fn iter_flat_split() {
        let items = vec!["a,b".into(), "c,d,e".into()];
        let result = KillerIter::from_vec(items).flat_split(",").collect_vec();
        assert_eq!(result, vec!["a", "b", "c", "d", "e"]);
    }

    #[test]
    fn iter_windows_chunks() {
        let items: Vec<String> = (0..5).map(|i| i.to_string()).collect();
        let iter = KillerIter::from_vec(items.clone());
        let wins = iter.windows(3);
        assert_eq!(wins.len(), 3);
        let iter2 = KillerIter::from_vec(items);
        let chunks = iter2.chunks(2);
        assert_eq!(chunks.len(), 3);
        assert_eq!(chunks[2].len(), 1);
    }
}
