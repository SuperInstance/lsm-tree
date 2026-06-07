# lsm-tree

A Log-Structured Merge Tree implementation in pure Rust with bloom filters.

## Features

- In-memory memtable (sorted skip-list style)
- Sorted string tables (SSTables) for persistent levels
- Level-based compaction
- Bloom filters for fast negative lookups
- Merge iteration across levels
- Zero external dependencies

## Usage

```rust
use lsm_tree::LsmTree;

let mut tree = LsmTree::new(100); // memtable capacity 100
tree.put(b"key", b"value");
assert_eq!(tree.get(b"key"), Some(b"value".to_vec()));
```

## Modules

- `memtable` — In-memory sorted key-value store
- `sstable` — Sorted string table representation
- `compaction` — Level compaction strategies
- `bloom` — Bloom filter for point lookups
- `merge` — Merge iteration across SSTables
