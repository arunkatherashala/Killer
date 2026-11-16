//! Insertion-ordered string-keyed map backing Killer dictionaries.
//!
//! Python, JavaScript and every JSON tool iterate and print a dictionary in the order its keys
//! were first inserted; a plain hash map would scramble that. Entries live in a vector (so
//! iteration is a straight walk) with a hash index for O(1) lookup. Removal leaves a tombstone
//! that is compacted away once tombstones outnumber live entries.

use crate::fast_hash::FastMap;

/// Up to this many entries the map is a plain vector scanned linearly: no hash table, no second
/// copy of every key (a 4-key dictionary literal used to cost a table plus four extra strings).
const SMALL: usize = 8;

#[derive(Clone)]
pub struct OrderedMap<V> {
    entries: Vec<Option<(String, V)>>,
    /// key -> position in `entries`; only maintained once `indexed` is set.
    index: FastMap<String, usize>,
    /// False while the map is small: then `entries` has no tombstones and `index` is empty.
    indexed: bool,
}

impl<V> Default for OrderedMap<V> {
    fn default() -> Self {
        OrderedMap { entries: Vec::new(), index: Default::default(), indexed: false }
    }
}

impl<V> OrderedMap<V> {
    pub fn new() -> Self {
        Self::default()
    }

    #[inline]
    fn find(&self, key: &str) -> Option<usize> {
        if self.indexed {
            self.index.get(key).copied()
        } else {
            self.entries.iter().position(|e| matches!(e, Some((k, _)) if k == key))
        }
    }

    #[inline]
    pub fn len(&self) -> usize {
        if self.indexed { self.index.len() } else { self.entries.len() }
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    #[inline]
    pub fn contains_key(&self, key: &str) -> bool {
        self.find(key).is_some()
    }

    #[inline]
    pub fn get(&self, key: &str) -> Option<&V> {
        let i = self.find(key)?;
        self.entries[i].as_ref().map(|(_, v)| v)
    }

    #[inline]
    pub fn get_mut(&mut self, key: &str) -> Option<&mut V> {
        let i = self.find(key)?;
        self.entries[i].as_mut().map(|(_, v)| v)
    }

    /// Insert or replace; a replaced key keeps its original position.
    pub fn insert(&mut self, key: String, value: V) -> Option<V> {
        if let Some(i) = self.find(&key) {
            if let Some((_, slot)) = self.entries[i].as_mut() {
                return Some(std::mem::replace(slot, value));
            }
        }
        if !self.indexed && self.entries.len() >= SMALL {
            self.index.reserve(self.entries.len() * 2);
            for (i, e) in self.entries.iter().enumerate() {
                if let Some((k, _)) = e {
                    self.index.insert(k.clone(), i);
                }
            }
            self.indexed = true;
        }
        if self.indexed {
            self.index.insert(key.clone(), self.entries.len());
        }
        self.entries.push(Some((key, value)));
        None
    }

    pub fn remove(&mut self, key: &str) -> Option<V> {
        if !self.indexed {
            let i = self.find(key)?;
            let (_, value) = self.entries.remove(i)?;
            return Some(value);
        }
        let i = self.index.remove(key)?;
        let (_, value) = self.entries[i].take()?;
        // compact once the dead entries dominate, so a queue-like use stays O(1) amortised
        let dead = self.entries.len() - self.index.len();
        if dead > 32 && dead > self.index.len() {
            self.compact();
        }
        Some(value)
    }

    fn compact(&mut self) {
        let live: Vec<(String, V)> = self.entries.drain(..).flatten().collect();
        self.index.clear();
        for (k, v) in live {
            self.index.insert(k.clone(), self.entries.len());
            self.entries.push(Some((k, v)));
        }
    }

    /// Move every live value out (in insertion order), leaving the map empty.
    pub fn take_values(&mut self) -> Vec<V> {
        self.index.clear();
        self.indexed = false;
        std::mem::take(&mut self.entries).into_iter().flatten().map(|(_, v)| v).collect()
    }

    pub fn clear(&mut self) {
        self.entries.clear();
        self.index.clear();
        self.indexed = false;
    }

    pub fn iter(&self) -> impl Iterator<Item = (&String, &V)> {
        self.entries.iter().flatten().map(|(k, v)| (k, v))
    }

    pub fn keys(&self) -> impl Iterator<Item = &String> {
        self.entries.iter().flatten().map(|(k, _)| k)
    }

    pub fn values(&self) -> impl Iterator<Item = &V> {
        self.entries.iter().flatten().map(|(_, v)| v)
    }

    pub fn extend<I: IntoIterator<Item = (String, V)>>(&mut self, iter: I) {
        for (k, v) in iter {
            self.insert(k, v);
        }
    }
}

impl<V: PartialEq> PartialEq for OrderedMap<V> {
    /// Dictionaries are equal when they hold the same pairs, whatever the insertion order.
    fn eq(&self, other: &Self) -> bool {
        self.len() == other.len() && self.iter().all(|(k, v)| other.get(k) == Some(v))
    }
}

impl<V: std::fmt::Debug> std::fmt::Debug for OrderedMap<V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_map().entries(self.iter()).finish()
    }
}

impl<V> FromIterator<(String, V)> for OrderedMap<V> {
    fn from_iter<I: IntoIterator<Item = (String, V)>>(iter: I) -> Self {
        let mut map = OrderedMap::new();
        map.extend(iter);
        map
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_insertion_order_and_replaces_in_place() {
        let mut m = OrderedMap::new();
        m.insert("z".to_string(), 1);
        m.insert("a".to_string(), 2);
        m.insert("m".to_string(), 3);
        assert_eq!(m.insert("a".to_string(), 9), Some(2));
        let keys: Vec<&str> = m.keys().map(|k| k.as_str()).collect();
        assert_eq!(keys, ["z", "a", "m"]);
        assert_eq!(m.get("a"), Some(&9));
        assert_eq!(m.len(), 3);
    }

    #[test]
    fn removal_keeps_order_and_compaction_keeps_lookups_valid() {
        let mut m = OrderedMap::new();
        for i in 0..200 {
            m.insert(format!("k{i}"), i);
        }
        for i in 0..150 {
            assert_eq!(m.remove(&format!("k{i}")), Some(i));
        }
        assert_eq!(m.len(), 50);
        assert_eq!(m.get("k199"), Some(&199));
        assert_eq!(m.get("k3"), None);
        let first: Vec<&str> = m.keys().take(2).map(|k| k.as_str()).collect();
        assert_eq!(first, ["k150", "k151"]);
        m.insert("k3".to_string(), 3);
        assert_eq!(m.keys().last().map(|k| k.as_str()), Some("k3"));
    }

    #[test]
    fn small_map_removes_in_place_and_grows_into_an_index() {
        let mut m = OrderedMap::new();
        for i in 0..6 {
            m.insert(format!("k{i}"), i);
        }
        assert_eq!(m.remove("k2"), Some(2));
        assert_eq!(m.remove("nope"), None);
        let keys: Vec<&str> = m.keys().map(|k| k.as_str()).collect();
        assert_eq!(keys, ["k0", "k1", "k3", "k4", "k5"]);
        assert_eq!(m.len(), 5);
        for i in 10..30 {
            m.insert(format!("k{i}"), i);
        }
        assert_eq!(m.len(), 25);
        assert_eq!(m.get("k3"), Some(&3));
        assert_eq!(m.get("k29"), Some(&29));
        assert_eq!(m.get("k2"), None);
        assert_eq!(m.remove("k0"), Some(0));
        assert_eq!(m.keys().next().map(|k| k.as_str()), Some("k1"));
        m.clear();
        m.insert("z".to_string(), 1);
        assert_eq!(m.get("z"), Some(&1));
    }
}
