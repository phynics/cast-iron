//! A tiny least-recently-used map for parsed shards.

#[derive(Debug)]
pub struct Lru<K, V> {
    capacity: usize,
    /// Least recently used first.
    items: Vec<(K, V)>,
}

impl<K: PartialEq, V: Clone> Lru<K, V> {
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity: capacity.max(1),
            items: Vec::new(),
        }
    }

    /// Returns a clone of the value and marks it most recently used.
    pub fn get(&mut self, key: &K) -> Option<V> {
        let position = self.items.iter().position(|(k, _)| k == key)?;
        let item = self.items.remove(position);
        let value = item.1.clone();
        self.items.push(item);
        Some(value)
    }

    pub fn insert(&mut self, key: K, value: V) {
        self.items.retain(|(k, _)| *k != key);
        if self.items.len() == self.capacity {
            self.items.remove(0);
        }
        self.items.push((key, value));
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn evicts_the_least_recently_used() {
        let mut lru = Lru::new(2);
        lru.insert("a", 1);
        lru.insert("b", 2);
        assert_eq!(lru.get(&"a"), Some(1)); // b is now the oldest
        lru.insert("c", 3);
        assert_eq!(lru.get(&"b"), None);
        assert_eq!(lru.get(&"a"), Some(1));
        assert_eq!(lru.get(&"c"), Some(3));
        assert_eq!(lru.len(), 2);
    }

    #[test]
    fn reinserting_replaces_without_growing() {
        let mut lru = Lru::new(2);
        lru.insert("a", 1);
        lru.insert("a", 2);
        assert_eq!(lru.len(), 1);
        assert_eq!(lru.get(&"a"), Some(2));
        assert!(!lru.is_empty());
    }
}
