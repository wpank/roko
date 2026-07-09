# Backend: In-Memory (`MemorySubstrate`)

> `MemorySubstrate` stores all `Engram`s in a heap `HashMap`. No I/O. Zero latency. Lost
> on process exit. The default backend for tests and short-lived agents.

**Status**: Shipping
**Crate**: `roko-runtime`
**Depends on**: [Backends Overview](./07-backends-overview.md), [Trait Surface](./01-trait-surface.md)
**Last reviewed**: 2026-04-19

---

## TL;DR

`MemorySubstrate` is a `HashMap<ContentHash, Engram>` with an HDC index array. All
operations are sub-microsecond. Data is not persisted; the store resets each run. Use for
tests and agents where persistence is handled externally.

---

## When to Use

- **Unit and integration tests** — fast, no file cleanup needed.
- **Benchmark harnesses** — removes I/O from the measured path.
- **Stateless agents** — agents whose state is fully reconstructed on each run.
- **Ephemeral pipelines** — process-and-discard workloads.

For production agents that need memory across restarts, use [FileSubstrate](./08-backend-file-jsonl.md).

---

## Structure

```rust
// source: crates/roko-runtime/src/memory_substrate.rs

pub struct MemorySubstrate {
    records: HashMap<ContentHash, Engram>,
    hdc_index: Vec<(ContentHash, HdcFingerprint)>,
    max_capacity: usize,
    decay_floor: f32,
}

impl MemorySubstrate {
    pub fn new() -> Self {
        Self {
            records: HashMap::new(),
            hdc_index: Vec::new(),
            max_capacity: usize::MAX,
            decay_floor: 0.0,
        }
    }

    pub fn with_capacity(max: usize) -> Self {
        Self { max_capacity: max, ..Self::new() }
    }
}
```
<!-- source: crates/roko-runtime/src/memory_substrate.rs -->

---

## Operation Costs

| Method | Time complexity | Notes |
|---|---|---|
| `put` | O(1) amortised | HashMap insert + Vec push |
| `get` | O(1) | HashMap lookup |
| `query` | O(n) | Linear scan of `records` |
| `query_similar` | O(n · D/64) | Linear scan of `hdc_index` |
| `prune` | O(n log n) | Decay sort + capacity trim |
| `len` | O(1) | `records.len()` |

---

## Concurrency

`MemorySubstrate` is not internally thread-safe — it does not include a lock. Wrap in
`Arc<RwLock<...>>` for multi-threaded access:

```rust
// source: crates/roko-runtime/src/agent.rs
let substrate: Arc<RwLock<Box<dyn Substrate>>> = Arc::new(RwLock::new(
    Box::new(MemorySubstrate::new())
));
```
<!-- source: crates/roko-runtime/src/agent.rs -->

---

## Failure Modes

<!-- ADDED -->

| Failure | Behaviour |
|---|---|
| OOM (HashMap grows beyond available RAM) | `put` panics with allocation error. Set `max_capacity` to prevent this. |
| `prune` on empty store | Returns `Ok(0)`. No-op. |
| `get` after process restart | Returns `Ok(None)` — all data is lost. This is expected. |

---

## See Also

- [Backend: JSONL File](./08-backend-file-jsonl.md) — durable alternative
- [Backends Overview](./07-backends-overview.md)

## Open Questions

- Should `MemorySubstrate` support optional snapshot/restore (serialize to bytes and back)
  to enable cheap checkpoint in tests?
