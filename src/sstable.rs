//! Sorted string table (SSTable) representation.

use crate::bloom::BloomFilter;

/// An SSTable is an immutable sorted list of key-value pairs.
#[derive(Debug, Clone)]
pub struct SSTable {
    /// Level this SSTable belongs to.
    level: usize,
    /// Sorted entries.
    entries: Vec<(Vec<u8>, Vec<u8>)>,
    /// Bloom filter for fast negative lookups.
    bloom: BloomFilter,
}

impl SSTable {
    /// Create a new SSTable from sorted entries at the given level.
    pub fn new(level: usize, entries: Vec<(Vec<u8>, Vec<u8>)>) -> Self {
        let mut bloom = BloomFilter::new(entries.len().max(100), 0.01);
        for (key, _) in &entries {
            bloom.insert(key);
        }
        SSTable {
            level,
            entries,
            bloom,
        }
    }

    /// Get the level of this SSTable.
    pub fn level(&self) -> usize {
        self.level
    }

    /// Look up a key in the SSTable.
    pub fn get(&self, key: &[u8]) -> Option<&[u8]> {
        if !self.bloom.might_contain(key) {
            return None;
        }
        match self.entries.binary_search_by(|(k, _)| k.as_slice().cmp(key)) {
            Ok(idx) => Some(&self.entries[idx].1),
            Err(_) => None,
        }
    }

    /// Check if the SSTable might contain a key (via bloom filter).
    pub fn might_contain(&self, key: &[u8]) -> bool {
        self.bloom.might_contain(key)
    }

    /// Get all entries.
    pub fn entries(&self) -> &[(Vec<u8>, Vec<u8>)] {
        &self.entries
    }

    /// Number of entries in this SSTable.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Is this SSTable empty?
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Get the smallest key in the SSTable.
    pub fn min_key(&self) -> Option<&[u8]> {
        self.entries.first().map(|(k, _)| k.as_slice())
    }

    /// Get the largest key in the SSTable.
    pub fn max_key(&self) -> Option<&[u8]> {
        self.entries.last().map(|(k, _)| k.as_slice())
    }

    /// Check if a key range overlaps with this SSTable.
    pub fn overlaps(&self, min_key: &[u8], max_key: &[u8]) -> bool {
        if let (Some(my_min), Some(my_max)) = (self.min_key(), self.max_key()) {
            my_min <= max_key && my_max >= min_key
        } else {
            false
        }
    }

    /// Iterate over entries within a key range.
    pub fn range(&self, start: &[u8], end: &[u8]) -> Vec<(&[u8], &[u8])> {
        self.entries
            .iter()
            .filter(|(k, _)| k.as_slice() >= start && k.as_slice() <= end)
            .map(|(k, v)| (k.as_slice(), v.as_slice()))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_sstable() -> SSTable {
        let entries = vec![
            (b"a".to_vec(), b"1".to_vec()),
            (b"b".to_vec(), b"2".to_vec()),
            (b"c".to_vec(), b"3".to_vec()),
            (b"d".to_vec(), b"4".to_vec()),
            (b"e".to_vec(), b"5".to_vec()),
        ];
        SSTable::new(0, entries)
    }

    #[test]
    fn test_get_found() {
        let sst = make_sstable();
        assert_eq!(sst.get(b"c"), Some(b"3".as_slice()));
    }

    #[test]
    fn test_get_not_found() {
        let sst = make_sstable();
        assert_eq!(sst.get(b"z"), None);
    }

    #[test]
    fn test_min_max_key() {
        let sst = make_sstable();
        assert_eq!(sst.min_key(), Some(b"a".as_slice()));
        assert_eq!(sst.max_key(), Some(b"e".as_slice()));
    }

    #[test]
    fn test_overlaps() {
        let sst = make_sstable();
        assert!(sst.overlaps(b"a", b"c"));
        assert!(sst.overlaps(b"d", b"f"));
        assert!(!sst.overlaps(b"f", b"z"));
    }

    #[test]
    fn test_range() {
        let sst = make_sstable();
        let range = sst.range(b"b", b"d");
        assert_eq!(range.len(), 3);
        assert_eq!(range[0].0, b"b");
        assert_eq!(range[2].0, b"d");
    }

    #[test]
    fn test_level() {
        let sst = make_sstable();
        assert_eq!(sst.level(), 0);
    }

    #[test]
    fn test_len() {
        let sst = make_sstable();
        assert_eq!(sst.len(), 5);
    }
}
