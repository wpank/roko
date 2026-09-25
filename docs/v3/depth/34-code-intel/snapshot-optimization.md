# Snapshot Optimization

> **Parent:** [34-CODE-INTELLIGENCE](../../34-CODE-INTELLIGENCE.md) Section 14

---

## The Cold Start Problem

### Current situation

Without persistent storage, every `roko-index` session starts from scratch:

```
Session start
     |
  Enumerate files (~50ms for 300 files)
     |
  Parse all files (~300ms for ~1M lines)
     |
  Build graph (~1ms for 5K+ symbols)
     |
  Compute fingerprints (~25ms for 5K symbols)
     |
  Compute PageRank (~1ms)
     |
  Ready (~377ms total)
```

This is acceptable for modest workspaces but scales linearly:

| Workspace | Symbol count | Full build | Incremental (1 file) |
|-----------|-------------|-----------|---------------------|
| Small (5K symbols) | 5,000 | ~377ms | ~5ms |
| Medium (50K) | 50,000 | ~2s | ~5ms |
| Large (500K) | 500,000 | ~20s | ~5ms |

Agents waiting 20 seconds before querying code intelligence is
unacceptable.

### With SQLite only

SQLite eliminates re-parsing but adds deserialization overhead:

```
Session start
     |
  Open SQLite database (~5ms)
     |
  Load symbols into memory (~50ms for 50K symbols)
     |
  Load fingerprints (~100ms for 50K)
     |
  Build in-memory graph (~20ms for 200K edges)
     |
  Ready (~175ms total for 50K symbols)
```

Better than re-parsing, but deserialization cost remains substantial.

### With rkyv snapshots (planned)

Zero-copy snapshots eliminate deserialization entirely:

```
Session start
     |
  Memory-map snapshot file (~1ms, regardless of size)
     |
  Validate snapshot header (~0.1ms)
     |
  Ready (~1.1ms total, regardless of workspace size)
```

---

## rkyv Serialization

### What rkyv provides

`rkyv` (Rust archiving) is a zero-copy deserialization framework. It
serializes Rust structs into a binary format where the serialized bytes
ARE the in-memory representation:

```rust
use rkyv::{Archive, Serialize, Deserialize};

#[derive(Archive, Serialize, Deserialize)]
pub struct ArchivedFingerprints {
    pub symbols: Vec<ArchivedSymbolEntry>,
}

#[derive(Archive, Serialize, Deserialize)]
pub struct ArchivedSymbolEntry {
    pub id: ArchivedSymbolId,
    pub fingerprint: [u64; 160],  // Directly accessible
    pub pagerank: f64,
}
```

When deserialized with `rkyv::from_bytes()`, the returned reference points
directly into the byte buffer -- no allocation, no copying.

### Snapshot format

```
+-------------------------------------------------+
| Header (64 bytes)                                |
|   Magic: "ROKO_IDX"  (8 bytes)                  |
|   Version: u32        (4 bytes)                  |
|   Flags: u32          (4 bytes)                  |
|   Symbol count: u64   (8 bytes)                  |
|   Edge count: u64     (8 bytes)                  |
|   File count: u64     (8 bytes)                  |
|   Workspace hash: [u8; 32]  (BLAKE3)            |
+-------------------------------------------------+
| Fingerprint section                              |
|   Contiguous array of [u64; 160] x symbol_count  |
|   (1,280 bytes per fingerprint)                  |
+-------------------------------------------------+
| Symbol metadata section (rkyv serialized)        |
+-------------------------------------------------+
| Graph section (rkyv serialized)                  |
|   Forward adjacency + Reverse adjacency          |
+-------------------------------------------------+
| PageRank section                                 |
|   Contiguous array of f64 x symbol_count         |
+-------------------------------------------------+
| String table (rkyv serialized, deduped)          |
+-------------------------------------------------+
```

### Why fingerprints are separate

The fingerprint section is a contiguous array rather than embedded in
metadata:

1. **Bulk comparison** -- Scanning for nearest-neighbor reads a contiguous
   memory region, maximizing cache line utilization.
2. **SIMD operations** -- Aligned u64 arrays can use AVX2/AVX-512 for
   parallel XOR + popcount.
3. **Partial loading** -- Fingerprints can be memory-mapped independently.

---

## Memory Mapping with memmap2

```rust
use memmap2::Mmap;

pub fn load_snapshot(path: &Path) -> Result<SnapshotIndex> {
    let file = std::fs::File::open(path)?;
    let mmap = unsafe { Mmap::map(&file)? };
    let header = SnapshotHeader::from_bytes(&mmap[0..64])?;
    if header.magic != *b"ROKO_IDX" {
        return Err(Error::InvalidSnapshot);
    }
    Ok(SnapshotIndex { mmap, header })
}
```

### Fingerprint access

With memory mapping, accessing a fingerprint is pointer arithmetic:

```rust
impl SnapshotIndex {
    pub fn fingerprint(&self, index: usize) -> &[u64; 160] {
        let offset = HEADER_SIZE + (index * FINGERPRINT_BYTES);
        let bytes = &self.mmap[offset..offset + FINGERPRINT_BYTES];
        unsafe { &*(bytes.as_ptr() as *const [u64; 160]) }
    }

    pub fn similarity(&self, a: usize, b: usize) -> f64 {
        let fp_a = self.fingerprint(a);
        let fp_b = self.fingerprint(b);
        let mut diff = 0u32;
        for (left, right) in fp_a.iter().zip(fp_b.iter()) {
            diff += (left ^ right).count_ones();
        }
        1.0 - (f64::from(diff) / 10_240.0)
    }
}
```

No deserialization. No allocation. Direct XOR + popcount on memory-mapped
bytes.

### OS-level optimizations

- **Demand paging** -- Only accessed pages load from disk
- **Shared mapping** -- Multiple processes share physical pages
- **Eviction under pressure** -- OS evicts pages when memory is tight
- **Read-ahead** -- Sequential access triggers OS prefetching

---

## Snapshot Size Estimates

| Component | Bytes per symbol |
|-----------|-----------------|
| HDC fingerprint | 1,280 |
| PageRank score | 8 |
| Symbol name (avg) | 20 |
| File path (avg) | 60 |
| Kind + visibility + line | 12 |
| Graph edges (avg 3) | 24 |
| **Total per symbol** | **~1,404** |

| Workspace | Symbols | Snapshot size | mmap load time |
|-----------|---------|-------------|---------------|
| Small (Roko) | ~5,000 | ~7 MB | < 1ms |
| Medium | ~50,000 | ~68 MB | < 1ms |
| Large | ~122,000 | ~166 MB | < 1ms |
| Enterprise | ~500,000 | ~680 MB | < 1ms |

### Performance comparison

| Operation | SQLite (cold) | SQLite (warm) | Snapshot (mmap) |
|-----------|-------------|-------------|----------------|
| Load 5K fingerprints | 50ms | 10ms | < 1ms |
| Compare 2 fingerprints | 5us | 1us | 50ns |
| Full scan 5K | 25ms | 5ms | 0.25ms |
| Full scan 50K | 250ms | 50ms | 2.5ms |

Snapshots are 20--100x faster for fingerprint operations.

---

## Differential Updates

### Overlay + compaction

Snapshots are immutable once written. The planned approach uses overlays:

```
Base snapshot (written during full index build)
    |
    +-- Overlay 1 (delta: 3 symbols changed)
    +-- Overlay 2 (delta: 1 symbol added)
    +-- Overlay 3 (delta: 2 symbols removed)
    |
    +-- Compacted snapshot (merges base + overlays)
```

Each overlay contains only changed symbols and fingerprints. When overlays
grow large (> 10% of base), a compaction pass merges everything into a new
base snapshot.

### Graph dirty flag

When files change, the graph may have new or removed edges. A dirty flag
triggers incremental graph update:

```rust
pub struct IncrementalGraph {
    base_graph: SymbolGraph,
    dirty: bool,
    pending_additions: Vec<SymbolEdge>,
    pending_removals: Vec<SymbolEdge>,
}
```

---

## Salsa Memoization (Planned)

Salsa (inspired by Adapton, used by rust-analyzer) is an incremental
computation framework that memoizes function results and re-computes only
when inputs change:

```rust
#[salsa::query_group(IndexDatabaseStorage)]
trait IndexDatabase {
    #[salsa::input]
    fn file_content(&self, path: String) -> Arc<String>;

    fn parsed_file(&self, path: String) -> Arc<SourceFile>;
    fn symbol_fingerprint(&self, id: SymbolId) -> HdcFingerprint;
    fn file_graph_edges(&self, path: String) -> Vec<SymbolEdge>;
    fn full_graph(&self) -> Arc<SymbolGraph>;
    fn pagerank_scores(&self) -> Arc<HashMap<SymbolId, f64>>;
}
```

When `file_content("graph.rs")` changes, Salsa re-executes only dependent
computations. `parsed_file("symbol.rs")` is NOT re-executed because its
input did not change.

### Salsa vs. snapshot overlays

| Property | Salsa | Snapshot overlays |
|----------|-------|-------------------|
| Granularity | Per-function memoization | Per-file deltas |
| Best for | Continuous IDE-like use | Batch agent sessions |
| Startup cost | Rebuild from snapshot | mmap + apply overlays |
| Memory | In-memory (all cached) | On-disk (demand paged) |

---

## Current Status

### Built

- In-memory `SymbolGraph`, `HdcFingerprint`, and `SymbolId` types
- `Serialize`/`Deserialize` on `SymbolId` and `SymbolRef`
- Deterministic fingerprint generation
- SQLite persistence with BLAKE3 incremental updates

### Missing

- rkyv `Archive`/`Serialize`/`Deserialize` derives on index types
- Snapshot file format and writer
- Memory-mapped snapshot reader
- Overlay mechanism for differential updates
- Compaction pass
- Salsa integration
- Benchmark suite comparing SQLite vs. snapshot performance

---

## Academic Foundations

- **rkyv**: Rust archiving framework. Zero-copy deserialization.
- **memmap2**: Safe memory-mapped file access for Rust.
- **Salsa**: rust-analyzer team (2019). Incremental computation inspired by
  Adapton.
- **Adapton**: Hammer, Khoo, Hicks, and Foster (2014). *PLDI*.
  Demand-driven incremental computation with automatic change propagation.
- **BLAKE3**: O'Connor et al. (2020). Content hashing for change detection.

---

## Cross-References

- See [index-db-scaling.md](./index-db-scaling.md) for SQLite storage
  (complementary to snapshots)
- See [hdc-fingerprints.md](./hdc-fingerprints.md) for the fingerprint data
  that snapshots optimize
- See [pagerank-symbol-importance.md](./pagerank-symbol-importance.md) for
  PageRank caching in snapshots
