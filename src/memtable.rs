//! In-memory sorted key-value store (memtable).

/// A sorted in-memory table backed by a Vec of key-value pairs.
#[derive(Debug, Clone)]
pub struct MemTable {
    entries: Vec<(Vec<u8>, Vec<u8>)>,
    size_limit: usize,
}

impl MemTable {
    /// Create a new memtable with a size limit (number of entries).
    pub fn new(size_limit: usize) -> Self {
        MemTable {
            entries: Vec::new(),
            size_limit,
        }
    }

    /// Put a key-value pair into the memtable.
    pub fn put(&mut self, key: &[u8], value: &[u8]) {
        match self.entries.binary_search_by(|(k, _)| k.as_slice().cmp(key)) {
            Ok(idx) => {
                self.entries[idx].1 = value.to_vec();
            }
            Err(idx) => {
                self.entries.insert(idx, (key.to_vec(), value.to_vec()));
            }
        }
    }

    /// Delete a key by inserting a tombstone marker.
    pub fn delete(&mut self, key: &[u8]) {
        self.put(key, b"");
    }

    /// Get a value by key.
    pub fn get(&self, key: &[u8]) -> Option<&[u8]> {
        match self.entries.binary_search_by(|(k, _)| k.as_slice().cmp(key)) {
            Ok(idx) => Some(&self.entries[idx].1),
            Err(_) => None,
        }
    }

    /// Check if the memtable is full and should be flushed.
    pub fn is_full(&self) -> bool {
        self.entries.len() >= self.size_limit
    }

    /// Number of entries in the memtable.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Is the memtable empty?
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Drain all entries from the memtable, leaving it empty.
    pub fn drain(&mut self) -> Vec<(Vec<u8>, Vec<u8>)> {
        std::mem::take(&mut self.entries)
    }

    /// Get all entries in sorted order.
    pub fn entries(&self) -> &[(Vec<u8>, Vec<u8>)] {
        &self.entries
    }

    /// Check if a key exists in the memtable.
    pub fn contains(&self, key: &[u8]) -> bool {
        self.entries.binary_search_by(|(k, _)| k.as_slice().cmp(key)).is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_put_and_get() {
        let mut mt = MemTable::new(100);
        mt.put(b"key1", b"value1");
        mt.put(b"key2", b"value2");
        assert_eq!(mt.get(b"key1"), Some(b"value1".as_slice()));
        assert_eq!(mt.get(b"key2"), Some(b"value2".as_slice()));
        assert_eq!(mt.get(b"key3"), None);
    }

    #[test]
    fn test_put_overwrite() {
        let mut mt = MemTable::new(100);
        mt.put(b"key", b"v1");
        mt.put(b"key", b"v2");
        assert_eq!(mt.get(b"key"), Some(b"v2".as_slice()));
        assert_eq!(mt.len(), 1);
    }

    #[test]
    fn test_sorted_order() {
        let mut mt = MemTable::new(100);
        mt.put(b"c", b"3");
        mt.put(b"a", b"1");
        mt.put(b"b", b"2");
        let entries = mt.entries();
        assert_eq!(entries[0].0, b"a");
        assert_eq!(entries[1].0, b"b");
        assert_eq!(entries[2].0, b"c");
    }

    #[test]
    fn test_is_full() {
        let mut mt = MemTable::new(3);
        mt.put(b"a", b"1");
        mt.put(b"b", b"2");
        assert!(!mt.is_full());
        mt.put(b"c", b"3");
        assert!(mt.is_full());
    }

    #[test]
    fn test_drain() {
        let mut mt = MemTable::new(100);
        mt.put(b"a", b"1");
        mt.put(b"b", b"2");
        let drained = mt.drain();
        assert_eq!(drained.len(), 2);
        assert!(mt.is_empty());
    }

    #[test]
    fn test_contains() {
        let mut mt = MemTable::new(100);
        mt.put(b"key", b"value");
        assert!(mt.contains(b"key"));
        assert!(!mt.contains(b"nokey"));
    }
}
