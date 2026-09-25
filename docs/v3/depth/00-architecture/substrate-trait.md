# The Substrate (Store) Trait

> **v3 depth file** -- `/docs/v3/depth/00-architecture/substrate-trait.md`
> Canonical source: v1 `docs/v1/00-architecture/07-substrate-trait.md`
> Implementation: `crates/roko-core/src/traits.rs` (Store + Substrate traits),
> `crates/roko-fs/src/file_substrate.rs` (FileSubstrate)
> Status: **Shipping** -- Store trait with `put`, `get`, `query`, `query_similar`, `prune`,
> `len`, `is_empty`. Substrate is a blanket-impl alias: `impl<T: Store> Substrate for T {}`.
> MemorySubstrate and FileSubstrate are implemented. ColdStore (archival) is also shipping.

---

## 1. Role in the Architecture

Store is the durable storage fabric at L0. It provides the ground for Signals:
content-addressed persistence, query-by-filter, native HDC similarity search, and pruning
by effective weight. Every subsystem that needs durable state depends on it.

Bus is the sibling fabric, not a replacement. Store persists Signals; Bus moves Pulses.
Together they are the complete kernel interface for Roko's runtime. The two fabrics are
separate because the system needs two different semantics:

- Store favors durability, idempotence, and retrieval of records.
- Bus favors fan-out, topic routing, and bounded replay of ephemeral Pulses.

### 1.1 Two-Fabric Summary

| Fabric | Medium | Core operations | Retention model | Implementations |
|---|---|---|---|---|
| Store | Signal | `put`, `get`, `query`, `query_similar`, `prune` | Long-lived storage with content identity, HDC fingerprints, and decay-aware pruning | MemorySubstrate, FileSubstrate |
| ColdStore | Signal | `archive`, `thaw`, `contains`, `purge_before` | Compressed archival for aged-out Signals | ArchiveColdSubstrate |
| Bus | Pulse | `publish`, `subscribe` | Bounded transport with topic routing | PulseBus |

---

## 2. Trait Surface

From `crates/roko-core/src/traits.rs`:

```rust
#[async_trait]
pub trait Store: Send + Sync {
    /// Store a signal. Returns its content hash. Idempotent on content.
    async fn put(&self, signal: Signal) -> Result<ContentHash>;

    /// Retrieve a signal by content hash. Does not apply decay.
    async fn get(&self, id: &ContentHash) -> Result<Option<Signal>>;

    /// Query for signals matching the given filter.
    async fn query(&self, q: &Query, ctx: &Context) -> Result<Vec<Signal>>;

    /// Query by HDC similarity against a fingerprint, returning ranked matches.
    async fn query_similar(
        &self,
        fp: &HdcVector,
        radius: f32,
        limit: usize,
        ctx: &Context,
    ) -> Result<Vec<(ContentHash, f32)>> {
        Ok(Vec::new())
    }

    /// Remove signals whose effective weight has fallen below threshold.
    async fn prune(&self, threshold: f32, ctx: &Context) -> Result<usize>;

    /// Optional: total count of stored signals.
    async fn len(&self) -> Result<usize> { Ok(0) }

    /// Optional: is the store empty?
    async fn is_empty(&self) -> Result<bool> { Ok(self.len().await? == 0) }

    /// Human-readable name for logging/debugging.
    fn name(&self) -> &'static str { "unnamed_store" }
}

/// Substrate -- legacy alias for the `Store` trait, used by `CellContext`.
pub trait Substrate: Store {}
impl<T: Store + ?Sized> Substrate for T {}
```

The shape is intentionally narrow. Store owns persistent state and retrieval by filter or
similarity; it does not absorb transport concerns, topic routing, or replay windows. Those
belong to the Bus fabric.

### 2.1 `put()` -- Persist

Stores a Signal and returns its `ContentHash`. The operation is idempotent: storing the
same Signal twice is a no-op because identity is content. When a fingerprint is not already
present, the Store populates the Signal's optional HDC fingerprint metadata at insert time
using the canonical encoder, so HDC similarity becomes a first-class property of the stored
record rather than a side-table concern.

### 2.2 `get()` -- Retrieve

Retrieves a single Signal by its `ContentHash`. Returns `None` if the Signal is not found
or has been pruned. `get()` returns the raw stored record, not a decay-adjusted view.

### 2.3 `query()` -- Filter and Retrieve

The primary read path. Queries combine all filters:

```rust
pub struct Query {
    pub kinds: Option<Vec<Kind>>,
    pub author: Option<String>,
    pub session: Option<String>,
    pub since_ms: Option<i64>,
    pub until_ms: Option<i64>,
    pub min_weight: Option<f32>,
    pub tags: Vec<(String, String)>,
    pub limit: Option<usize>,
}
```

Implementations may apply decay when evaluating `min_weight` and when ordering results.

### 2.4 `query_similar()` -- HDC Similarity Search

Native similarity search over durable records using 10,240-bit `HdcVector` fingerprints.
Returns the nearest matches within a Hamming radius and count limit.

```rust
async fn query_similar(
    &self,
    fp: &HdcVector,
    radius: f32,
    limit: usize,
    ctx: &Context,
) -> Result<Vec<(ContentHash, f32)>>
```

For file-backed and in-memory stores, a brute-force scan is viable because the comparison
is a tight Hamming/popcount operation over a fixed-size vector. The default implementation
returns empty results, keeping existing stores source-compatible until they add native
support.

### 2.5 `prune()` -- Garbage Collection

Removes Signals whose effective weight has fallen below the threshold:

```
weight = score.effective() x decay.apply(ctx.now_ms - created_at_ms)
```

Pruning is an explicit storage concern. It is not a transport concern and does not affect
Bus semantics.

---

## 3. ColdStore -- Archival Sibling

```rust
#[async_trait]
pub trait ColdStore: Send + Sync {
    async fn archive(&self, signal: Signal) -> Result<ContentHash>;
    async fn archive_batch(&self, signals: Vec<Signal>) -> Result<usize>;
    async fn thaw(&self, id: &ContentHash) -> Result<Option<Signal>>;
    async fn contains(&self, id: &ContentHash) -> Result<bool>;
    async fn archived_count(&self) -> Result<usize>;
    async fn storage_bytes(&self) -> Result<u64>;
    async fn purge_before(&self, epoch_ms: i64) -> Result<usize>;
    fn name(&self) -> &'static str;
}
```

ColdStore is the archival complement to Store. Signals migrate from hot to cold when they
age out, and can be thawed back on demand. The `roko-serve` cold-substrate archival timer
runs a configurable age-based policy.

---

## 4. Implementations

### 4.1 MemorySubstrate

In-memory `HashMap` backend for testing. Fast, ephemeral, single-process.

```rust
pub struct MemorySubstrate {
    engrams: RwLock<HashMap<ContentHash, Engram>>,
}
```

### 4.2 FileSubstrate (roko-fs)

JSONL file backend for default persistence. Uses append-only writes for crash safety
and periodic compaction via `prune()`. Computes or preserves each Signal's HDC fingerprint
at `put()` time so `query_similar()` can be answered directly against stored records.

Located in `crates/roko-fs/src/file_substrate.rs`. This is the default Store for all
Roko agents. Signal data is stored in `.roko/engrams.jsonl`.

### 4.3 ArchiveColdSubstrate (roko-fs)

Compressed JSONL archive files for cold storage. Implements ColdStore.

---

## 5. Concurrency

All Stores are `Send + Sync`. Implementations handle concurrent access internally:

- `MemorySubstrate` uses `RwLock<HashMap<...>>`
- `FileSubstrate` uses append-only writes with periodic compaction
- Future network Stores would use message passing or distributed locks

Multiple cognitive speeds can access the same Store concurrently. The `Send + Sync` bounds
ensure that callers can share a Store handle across async tasks without external locking.

---

## 6. The Store-Bus Bridge

Store and Bus cooperate but maintain separate responsibilities. The bridge pattern connects
them:

```rust
async fn put_and_broadcast<S: Store, B: Bus>(
    store: &S,
    bus: &B,
    signal: Signal,
) -> Result<ContentHash> {
    let hash = store.put(signal.clone()).await?;

    // Best-effort bridge: storage succeeds first, then a Pulse is emitted
    let pulse = signal.to_pulse(
        Topic::new("substrate.engram.stored"),
        0,
        PulseSource {
            component: "substrate:file".into(),
            agent_id: None,
        },
    );
    let _ = bus.publish(pulse);

    Ok(hash)
}
```

The bridge is the important architectural point: Store remains the durable store and
similarity surface, while Bus carries the notification stream that other operators can
observe without creating a layer violation.

---

## 7. Architectural Summary

The two-fabric kernel story:

- `Store` persists durable Signals and answers both filter queries and HDC similarity queries.
- `ColdStore` archives aged-out Signals with compressed storage and on-demand thaw.
- `Bus` transports ephemeral Pulses and carries coordination traffic without turning storage
  into transport.

---

## Academic Foundations

| Citation | Contribution |
|---|---|
| Beer 1972, *Brain of the Firm* | VSM System 1 (Operations): the operational storage fabric |
| Sumers et al. 2023 (arXiv:2309.02427) | CoALA: working memory and episodic memory components |
| Kanerva 2009, *Cognitive Computation* 1(2) | HDC: hyperdimensional computing for similarity search |

---

## Cross-References

- `bus-transport-fabric.md` -- The Bus sibling fabric
- `synapse-traits-12.md` -- All 12 kernel traits
- `score-7-axis-appraisal.md` -- How Score and Decay drive Store queries
- `decay-variants-and-tier-matrix.md` -- How pruning uses decay
- `naming-and-glossary.md` -- Canonical terminology
