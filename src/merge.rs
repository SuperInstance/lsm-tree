//! Merge iteration across multiple SSTables.

use crate::sstable::SSTable;

/// Merge iterator that combines entries from multiple SSTables,
/// resolving duplicates by preferring newer entries (later in the list).
pub struct MergeIterator {
    /// All entries from all SSTables, sorted.
    entries: Vec<(Vec<u8>, Vec<u8>)>,
    index: usize,
}

impl MergeIterator {
    /// Create a new merge iterator over multiple SSTables.
    /// SSTables should be ordered from oldest to newest.
    pub fn new(tables: &[SSTable]) -> Self {
        let mut all_entries: Vec<(Vec<u8>, Vec<u8>)> = Vec::new();
        for table in tables {
            for entry in table.entries() {
                all_entries.push(entry.clone());
            }
        }
        all_entries.sort_by(|a, b| a.0.cmp(&b.0));
        // Deduplicate: keep the last (newest) entry for each key
        let mut deduped: Vec<(Vec<u8>, Vec<u8>)> = Vec::new();
        for entry in all_entries {
            if let Some(last) = deduped.last_mut() {
                if last.0 == entry.0 {
                    last.1 = entry.1;
                    continue;
                }
            }
            deduped.push(entry);
        }
        MergeIterator {
            entries: deduped,
            index: 0,
        }
    }

    /// Seek to a specific key.
    pub fn seek(&mut self, key: &[u8]) {
        self.index = self.entries.partition_point(|(k, _)| k.as_slice() < key);
    }

    /// Get the current entry.
    pub fn current(&self) -> Option<(&[u8], &[u8])> {
        self.entries.get(self.index).map(|(k, v)| (k.as_slice(), v.as_slice()))
    }
}

impl Iterator for MergeIterator {
    type Item = (Vec<u8>, Vec<u8>);

    fn next(&mut self) -> Option<Self::Item> {
        if self.index < self.entries.len() {
            let item = self.entries[self.index].clone();
            self.index += 1;
            Some(item)
        } else {
            None
        }
    }
}

/// Merge two SSTables into a single sorted entry list.
pub fn merge_two_tables(older: &SSTable, newer: &SSTable) -> Vec<(Vec<u8>, Vec<u8>)> {
    let mut result = Vec::new();
    let mut old_iter = older.entries().iter().peekable();
    let mut new_iter = newer.entries().iter().peekable();

    loop {
        match (old_iter.peek(), new_iter.peek()) {
            (Some((ok, ov)), Some((nk, nv))) => {
                match ok.cmp(nk) {
                    std::cmp::Ordering::Less => {
                        result.push((ok.clone(), ov.clone()));
                        old_iter.next();
                    }
                    std::cmp::Ordering::Equal => {
                        // Newer wins
                        result.push((nk.clone(), nv.clone()));
                        old_iter.next();
                        new_iter.next();
                    }
                    std::cmp::Ordering::Greater => {
                        result.push((nk.clone(), nv.clone()));
                        new_iter.next();
                    }
                }
            }
            (Some((k, v)), None) => {
                result.push((k.clone(), v.clone()));
                old_iter.next();
            }
            (None, Some((k, v))) => {
                result.push((k.clone(), v.clone()));
                new_iter.next();
            }
            (None, None) => break,
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_table(entries: Vec<(&[u8], &[u8])>) -> SSTable {
        let mut sorted: Vec<(Vec<u8>, Vec<u8>)> = entries
            .into_iter()
            .map(|(k, v)| (k.to_vec(), v.to_vec()))
            .collect();
        sorted.sort_by(|a, b| a.0.cmp(&b.0));
        SSTable::new(0, sorted)
    }

    #[test]
    fn test_merge_non_overlapping() {
        let t1 = make_table(vec![(b"a", b"1"), (b"c", b"3")]);
        let t2 = make_table(vec![(b"b", b"2"), (b"d", b"4")]);
        let merged: Vec<_> = MergeIterator::new(&[t1, t2]).collect();
        assert_eq!(merged.len(), 4);
        assert_eq!(merged[0].0, b"a");
        assert_eq!(merged[3].0, b"d");
    }

    #[test]
    fn test_merge_with_overlap() {
        let t1 = make_table(vec![(b"a", b"old"), (b"b", b"2")]);
        let t2 = make_table(vec![(b"a", b"new"), (b"c", b"3")]);
        let merged: Vec<_> = MergeIterator::new(&[t1, t2]).collect();
        assert_eq!(merged[0], (b"a".to_vec(), b"new".to_vec()));
    }

    #[test]
    fn test_merge_two_tables_func() {
        let t1 = make_table(vec![(b"a", b"1"), (b"b", b"2")]);
        let t2 = make_table(vec![(b"b", b"new2"), (b"c", b"3")]);
        let merged = merge_two_tables(&t1, &t2);
        assert_eq!(merged.len(), 3);
        assert_eq!(merged[1], (b"b".to_vec(), b"new2".to_vec()));
    }

    #[test]
    fn test_seek() {
        let t = make_table(vec![(b"a", b"1"), (b"b", b"2"), (b"c", b"3")]);
        let mut iter = MergeIterator::new(&[t]);
        iter.seek(b"b");
        assert_eq!(iter.current(), Some((b"b".as_slice(), b"2".as_slice())));
    }

    #[test]
    fn test_empty_merge() {
        let iter = MergeIterator::new(&[]);
        assert_eq!(iter.count(), 0);
    }

    #[test]
    fn test_single_table() {
        let t = make_table(vec![(b"x", b"1"), (b"y", b"2")]);
        let merged: Vec<_> = MergeIterator::new(&[t]).collect();
        assert_eq!(merged.len(), 2);
    }
}
