//! Level-based compaction strategies.

use crate::memtable::MemTable;
use crate::sstable::SSTable;

/// Compaction strategy for merging SSTables.
#[derive(Debug, Clone, Copy)]
pub enum CompactionStrategy {
    /// Size-tiered compaction: merge SSTables of similar size.
    SizeTiered,
    /// Leveled compaction: each level has a max size.
    Leveled,
}

/// Compactor merges SSTables within a level.
pub struct Compactor {
    strategy: CompactionStrategy,
    level_size_ratio: usize,
    max_level: usize,
}

impl Compactor {
    /// Create a new compactor.
    pub fn new(strategy: CompactionStrategy, level_size_ratio: usize) -> Self {
        Compactor {
            strategy,
            level_size_ratio,
            max_level: 7,
        }
    }

    /// Check if compaction is needed for a given level.
    pub fn needs_compaction(&self, tables: &[SSTable], level: usize) -> bool {
        match self.strategy {
            CompactionStrategy::SizeTiered => {
                let level_max = self.level_size_ratio.pow(level as u32 + 1);
                let total_entries: usize = tables.iter().map(|t| t.len()).sum();
                total_entries > level_max
            }
            CompactionStrategy::Leveled => {
                let level_max = self.level_size_ratio.pow(level as u32 + 1);
                let total_entries: usize = tables.iter().map(|t| t.len()).sum();
                total_entries > level_max
            }
        }
    }

    /// Compact a set of SSTables into one SSTable at the next level.
    /// Returns the merged SSTable.
    pub fn compact(&self, tables: Vec<SSTable>, target_level: usize) -> SSTable {
        let merged = self.merge_tables(&tables);
        SSTable::new(target_level, merged)
    }

    /// Merge multiple SSTables into sorted entries, deduplicating by keeping the newest.
    pub fn merge_tables(&self, tables: &[SSTable]) -> Vec<(Vec<u8>, Vec<u8>)> {
        let mut all_entries: Vec<(Vec<u8>, Vec<u8>)> = Vec::new();
        // Later tables are newer, so add them last to win dedup
        for table in tables {
            for (k, v) in table.entries() {
                all_entries.push((k.clone(), v.clone()));
            }
        }
        // Sort by key
        all_entries.sort_by(|a, b| a.0.cmp(&b.0));
        // Deduplicate: keep the last (newest) entry for each key
        let mut result: Vec<(Vec<u8>, Vec<u8>)> = Vec::new();
        for entry in all_entries {
            if let Some(last) = result.last_mut() {
                if last.0 == entry.0 {
                    last.1 = entry.1;
                    continue;
                }
            }
            result.push(entry);
        }
        // Remove tombstones (empty values)
        result.retain(|(_, v)| !v.is_empty());
        result
    }

    /// Flush a memtable to an SSTable at level 0.
    pub fn flush_memtable(&self, memtable: &mut MemTable) -> Option<SSTable> {
        if memtable.is_empty() {
            return None;
        }
        let entries = memtable.drain();
        Some(SSTable::new(0, entries))
    }

    /// Get the maximum level.
    pub fn max_level(&self) -> usize {
        self.max_level
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_flush_memtable() {
        let compactor = Compactor::new(CompactionStrategy::Leveled, 4);
        let mut mt = MemTable::new(100);
        mt.put(b"a", b"1");
        mt.put(b"b", b"2");
        let sst = compactor.flush_memtable(&mut mt).unwrap();
        assert_eq!(sst.level(), 0);
        assert_eq!(sst.len(), 2);
        assert!(mt.is_empty());
    }

    #[test]
    fn test_compact_merge() {
        let compactor = Compactor::new(CompactionStrategy::Leveled, 4);
        let sst1 = SSTable::new(0, vec![
            (b"a".to_vec(), b"1".to_vec()),
            (b"c".to_vec(), b"3".to_vec()),
        ]);
        let sst2 = SSTable::new(0, vec![
            (b"b".to_vec(), b"2".to_vec()),
            (b"c".to_vec(), b"3_updated".to_vec()),
        ]);
        let merged = compactor.compact(vec![sst1, sst2], 1);
        assert_eq!(merged.level(), 1);
        assert_eq!(merged.len(), 3);
        assert_eq!(merged.get(b"c"), Some(b"3_updated".as_slice()));
    }

    #[test]
    fn test_needs_compaction() {
        let compactor = Compactor::new(CompactionStrategy::Leveled, 2);
        let tables: Vec<SSTable> = (0..5)
            .map(|_| SSTable::new(0, vec![(b"k".to_vec(), b"v".to_vec())]))
            .collect();
        assert!(compactor.needs_compaction(&tables, 0));
    }

    #[test]
    fn test_no_compaction_needed() {
        let compactor = Compactor::new(CompactionStrategy::Leveled, 100);
        let tables = vec![SSTable::new(0, vec![(b"k".to_vec(), b"v".to_vec())])];
        assert!(!compactor.needs_compaction(&tables, 0));
    }

    #[test]
    fn test_tombstone_removal() {
        let compactor = Compactor::new(CompactionStrategy::Leveled, 4);
        let sst = SSTable::new(0, vec![
            (b"a".to_vec(), b"1".to_vec()),
            (b"b".to_vec(), b"".to_vec()), // tombstone
        ]);
        let merged = compactor.compact(vec![sst], 1);
        assert_eq!(merged.len(), 1); // tombstone removed
        assert_eq!(merged.get(b"a"), Some(b"1".as_slice()));
    }

    #[test]
    fn test_flush_empty() {
        let compactor = Compactor::new(CompactionStrategy::Leveled, 4);
        let mut mt = MemTable::new(100);
        assert!(compactor.flush_memtable(&mut mt).is_none());
    }
}
