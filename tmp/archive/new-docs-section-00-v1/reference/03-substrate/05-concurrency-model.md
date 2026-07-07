# Substrate Concurrency Model

> How `Substrate` handles concurrent access: threading requirements, `Send + Sync` bounds,
> the shared-state rules callers must obey, and the async path that is coming.

**Status**: Shipping
**Crate**: `roko-core`
**Depends on**: [Trait Surface](./01-trait-surface.md)
**Last reviewed**: 2026-04-19

---

## TL;DR

The `Substrate` trait requires `Send + Sync`. Today the trait is synchronous — callers that
need concurrent access must wrap the implementation in `Arc<Mutex<dyn Substrate>>` or use a
backend that handles internal locking (like `MemorySubstrate`). An async-compatible trait
shape is planned.

---

## Trait Bounds

```rust
// source: crates/roko-core/src/substrate.rs
pub trait Substrate: Send + Sync { ... }
```
<!-- source: crates/roko-core/src/substrate.rs -->

`Send` means a `Box<dyn Substrate>` can be moved to another thread.
`Sync` means a `&dyn Substrate` can be shared between threads.

Together, these allow the runtime to hold a single substrate instance in a `Arc<RwLock<...>>`
or similar and share it across multiple agent threads.

---

## Today: Synchronous Trait

The current `Substrate` trait is **synchronous** — all methods block the calling thread until
completion. This is straightforward but has two consequences:

1. **Blocking I/O on the calling thread.** If the backend does disk I/O, the calling thread
   is blocked for the duration. In the cognitive loop, this contributes to loop-tick latency.
   See [Performance](./13-performance.md) for measured costs.

2. **External locking for mutable access.** `put` and `prune` take `&mut self`, so only one
   writer at a time. The runtime wraps the substrate in a lock:

   ```rust
   // source: crates/roko-runtime/src/agent.rs
   pub struct Agent {
       substrate: Arc<Mutex<Box<dyn Substrate>>>,
       // ...
   }
   ```
   <!-- source: crates/roko-runtime/src/agent.rs -->

   Reads (`get`, `query`, `query_similar`, `len`) take `&self` and can be called concurrently
   if the lock is an `RwLock`.

---

## Lock Patterns

### Single-agent (most common)

```rust
// source: crates/roko-runtime/src/agent.rs
// Single agent, single-threaded cognitive loop.
// No locking needed — the loop owns the substrate exclusively.
let mut substrate = FileSubstrate::open(path)?;
substrate.put(engram)?;
```
<!-- source: crates/roko-runtime/src/agent.rs -->

### Multi-reader / single-writer

```rust
// source: crates/roko-runtime/src/agent.rs
// Multiple loop speeds reading concurrently, one writer at a time.
let substrate: Arc<RwLock<Box<dyn Substrate>>> = Arc::new(RwLock::new(
    Box::new(MemorySubstrate::new())
));

// Reader (Gamma/Alpha speed):
let results = substrate.read().unwrap().query_similar(&fp, 16)?;

// Writer (Delta consolidation):
substrate.write().unwrap().put(consolidated_engram)?;
```
<!-- source: crates/roko-runtime/src/agent.rs -->

---

## Shared State Rules

1. **One active writer.** `put` and `prune` are exclusive. Never call them from two threads
   simultaneously without an external lock.
2. **Read-after-write is consistent.** A `get(hash)` called after `put(engram)` in the same
   thread (or after a lock release) must return the stored record. Implementations must not
   defer writes beyond the `put` return.
3. **Prune visibility.** A record that has been pruned must not appear in subsequent `get` or
   `query` calls. Implementations must not prune records while a `query` is in progress
   (readers hold the read lock during the query).

---

## Today vs. Planned: Async

> Target state: `Specified`

The planned async trait shape uses `async_trait` (or native Rust async traits, once stable):

```rust
// source: crates/roko-core/src/substrate.rs  [target-state, not shipped]
#[async_trait]
pub trait AsyncSubstrate: Send + Sync {
    async fn put(&mut self, engram: Engram) -> Result<(), SubstrateError>;
    async fn get(&self, id: &ContentHash) -> Result<Option<Engram>, SubstrateError>;
    async fn query(&self, q: &SubstrateQuery) -> Result<Vec<Engram>, SubstrateError>;
    async fn query_similar(
        &self,
        fingerprint: &HdcFingerprint,
        k: usize,
    ) -> Result<Vec<Engram>, SubstrateError>;
    async fn prune(&mut self) -> Result<usize, SubstrateError>;
    fn len(&self) -> usize;
}
```
<!-- source: crates/roko-core/src/substrate.rs -->

Migration from sync to async is a breaking change. The plan is to introduce `AsyncSubstrate`
as a parallel trait and let backends implement both during a transition window.

---

## See Also

- [Trait Surface](./01-trait-surface.md)
- [Performance](./13-performance.md) — lock-contention benchmarks
- [Failure Modes](./12-failure-modes.md) — what happens if a write panics mid-operation

## Open Questions

- Should `Substrate` adopt `async_trait` now (accepting the boxing overhead) or wait for
  native async traits in stable Rust?
- Is `Arc<RwLock<Box<dyn Substrate>>>` the right runtime shape, or should the runtime use a
  dedicated substrate-worker thread with a channel?
