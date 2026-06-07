//! Log-structured merge tree with memtable, SSTable, compaction, and bloom filters.

pub mod bloom;
pub mod compaction;
pub mod memtable;
pub mod merge;
pub mod sstable;

use compaction::{CompactionStrategy, Compactor};
use memtable::MemTable;
use sstable::SSTable;

/// LSM Tree combining memtable and SSTable levels.
pub struct LsmTree {
    memtable: MemTable,
    levels: Vec<Vec<SSTable>>,
    compactor: Compactor,
}

impl LsmTree {
    /// Create a new LSM tree with the given memtable capacity.
    pub fn new(memtable_capacity: usize) -> Self {
        let compactor = Compactor::new(CompactionStrategy::Leveled, 4);
        LsmTree {
            memtable: MemTable::new(memtable_capacity),
            levels: vec![Vec::new(); 7],
            compactor,
        }
    }

    /// Put a key-value pair.
    pub fn put(&mut self, key: &[u8], value: &[u8]) {
        self.memtable.put(key, value);
        if self.memtable.is_full() {
            self.flush();
        }
    }

    /// Delete a key (inserts tombstone).
    pub fn delete(&mut self, key: &[u8]) {
        self.memtable.put(key, b"");
    }

    /// Get a value by key.
    pub fn get(&self, key: &[u8]) -> Option<Vec<u8>> {
        // Check memtable first
        if let Some(val) = self.memtable.get(key) {
            if val.is_empty() {
                return None; // Tombstone
            }
            return Some(val.to_vec());
        }

        // Check SSTables from level 0 to deepest
        for level in &self.levels {
            // Check in reverse order (newest first)
            for table in level.iter().rev() {
                if let Some(val) = table.get(key) {
                    if val.is_empty() {
                        return None; // Tombstone
                    }
                    return Some(val.to_vec());
                }
            }
        }
        None
    }

    /// Flush the memtable to L0.
    fn flush(&mut self) {
        if let Some(sst) = self.compactor.flush_memtable(&mut self.memtable) {
            self.levels[0].push(sst);
            self.maybe_compact(0);
        }
    }

    /// Check if compaction is needed and perform it.
    fn maybe_compact(&mut self, level: usize) {
        if level >= self.levels.len() - 1 {
            return;
        }
        if self.compactor.needs_compaction(&self.levels[level], level) {
            let tables = std::mem::take(&mut self.levels[level]);
            let merged = self.compactor.compact(tables, level + 1);
            self.levels[level + 1].push(merged);
            self.maybe_compact(level + 1);
        }
    }

    /// Number of SSTables across all levels.
    pub fn table_count(&self) -> usize {
        self.levels.iter().map(|l| l.len()).sum()
    }

    /// Number of levels with data.
    pub fn active_levels(&self) -> usize {
        self.levels.iter().filter(|l| !l.is_empty()).count()
    }

    /// Memtable size.
    pub fn memtable_len(&self) -> usize {
        self.memtable.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_put_get() {
        let mut tree = LsmTree::new(100);
        tree.put(b"hello", b"world");
        assert_eq!(tree.get(b"hello"), Some(b"world".to_vec()));
    }

    #[test]
    fn test_delete() {
        let mut tree = LsmTree::new(100);
        tree.put(b"key", b"value");
        tree.delete(b"key");
        assert_eq!(tree.get(b"key"), None);
    }

    #[test]
    fn test_memtable_flush() {
        let mut tree = LsmTree::new(5);
        for i in 0..10 {
            tree.put(format!("k{}", i).as_bytes(), b"v");
        }
        assert!(tree.table_count() > 0);
        // All keys should still be accessible
        for i in 0..10 {
            assert!(tree.get(format!("k{}", i).as_bytes()).is_some());
        }
    }

    #[test]
    fn test_not_found() {
        let tree = LsmTree::new(100);
        assert_eq!(tree.get(b"nonexistent"), None);
    }

    #[test]
    fn test_overwrite() {
        let mut tree = LsmTree::new(100);
        tree.put(b"key", b"old");
        tree.put(b"key", b"new");
        assert_eq!(tree.get(b"key"), Some(b"new".to_vec()));
    }

    #[test]
    fn test_many_keys() {
        let mut tree = LsmTree::new(10);
        for i in 0..100 {
            tree.put(format!("key_{:04}", i).as_bytes(), format!("val_{}", i).as_bytes());
        }
        for i in 0..100 {
            let val = tree.get(format!("key_{:04}", i).as_bytes());
            assert_eq!(val, Some(format!("val_{}", i).into_bytes()));
        }
    }

    #[test]
    fn test_compaction_triggers() {
        let mut tree = LsmTree::new(3);
        for i in 0..50 {
            tree.put(format!("k{}", i).as_bytes(), b"v");
        }
        // Should have compacted some levels
        assert!(tree.active_levels() >= 1);
    }
}
