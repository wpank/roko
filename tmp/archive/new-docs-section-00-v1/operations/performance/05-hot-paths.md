# Hot Paths

> The operations that run on every Engram, every turn, and every event — the code that
> must be fast and must not allocate unexpectedly in steady state.

**Status**: Shipping
**Crate**: `roko-core`, `roko-fs`, `roko-runtime`
**Depends on**: [03-memory-model.md](03-memory-model.md), [04-numerical-stability.md](04-numerical-stability.md)
**Last reviewed**: 2026-04-19

---

## TL;DR

Five operations are on the hot path and must not allocate in steady state:

1. `Engram::new()` and field mutation.
2. `Score::compute()` and decay step.
3. `HdcVector` XOR bind and Hamming distance.
4. `Substrate::append()` (buffered write path).
5. `EventBus::publish()`.

If you are adding code that runs in any of these five operations, do not allocate on the
heap unless there is no alternative.

---

## Hot Path 1: Engram Construction

Every piece of data in Roko — agent output, gate verdict, tool result, learning episode — is
stored as an Engram. `Engram::new()` runs thousands of times per minute in a busy system.

**What runs on this path:**
- BLAKE3 content hash computation (content-addressed ID).
- Score initialization (7 × `f32`, zeroed or caller-supplied).
- HDC fingerprint computation (XOR bind of content chunks).
- Provenance stamping (author kind + trust level).
- Arena allocation for the Engram's field buffers.

**Allocation rule**: All field allocations are from the tick arena. No `Box<T>` or
`Vec<T>` on this path. Strings are arena-allocated slices. BLAKE3 operates on the input
bytes without intermediate heap allocation.

**What not to add here**:
- LLM calls.
- Substrate reads (writing is fine; reads require index traversal).
- Regex compilation (compile regexes at startup, cache them).
- Any `String::from`, `Vec::new`, or `HashMap::new` — use arena-allocated variants.

---

## Hot Path 2: Score Arithmetic and Decay

Score computation runs when an Engram is scored (immediately after construction), when an
agent's output score is re-evaluated, and on every GC pass (decay step for all live
Engrams).

**What runs on this path:**
- 7 × `f32` multiply/add/clamp operations.
- Optional `f64` intermediate for decay (see [04-numerical-stability.md](04-numerical-stability.md)).
- EMA update for adaptive gate thresholds (on gate events, not every Engram).

**Allocation rule**: Zero allocations. All Score values are inline `f32` fields in the
`Engram` struct. No boxing, no `Vec<f32>`.

**GC decay pass**: runs on all live Engrams in the Substrate. For a 100,000-Engram
store, this is `100,000 × 7 × ~22 ns ≈ 15 ms`. The GC decay pass runs on a background
Tokio task; it does not block the main agent loop.

**Vectorisation**: The decay pass on `f32[7]` arrays is a vectorisation candidate
(AVX2: 8 floats per instruction). The compiler auto-vectorises this in practice — verify
with `cargo asm` if performance is unexpectedly poor.

---

## Hot Path 3: HDC Vector Operations

HDC vectors are 10,240-bit (1,280-byte) binary arrays. The critical operations:

| Operation | Implementation | Alloc? |
|-----------|---------------|--------|
| `bind()` (XOR) | 20 × `u64::bitxor` (loop) | None |
| `bundle()` (majority vote) | 20 × `u64` popcount + threshold | None |
| `permute()` (circular shift) | Bitwise shift across 20 `u64` words | None |
| `hamming_distance()` | 20 × `u64::count_ones()` | None |
| Construction from bytes | `memcpy` into a fixed `[u8; 1280]` | None |

All HDC operations work on fixed-size arrays on the stack or in pre-allocated pool
slots. No heap allocation.

**SIMD note**: `u64::count_ones()` compiles to `POPCNT` on x86-64. The full Hamming
distance across 20 `u64` words is 20 `POPCNT` instructions — very fast.

**Search path**: HDC similarity search (find nearest neighbours) is a linear scan. At
100K entries, this is 100K × 20 `POPCNT` ≈ 2M instructions — < 1 ms. At 1M entries,
it is < 10 ms (see [01-latency-budgets.md](01-latency-budgets.md) for measured numbers).
The search is inherently parallelisable; multi-threaded search is planned.

---

## Hot Path 4: Substrate Append

Every Engram persist call goes through the buffered JSONL append path:

```
Engram → serde_json::to_writer (arena-allocated output buffer) → write to 64 KB buffer
 → if buffer full: flush buffer → write(2) syscall
```

**Allocation rule**: The serde output writes into a pre-allocated `Vec<u8>` buffer (not
a fresh `Vec` per write). The buffer is reused across appends. Flushing to disk is the
only system call; no `malloc`.

**fsync policy**: The default policy does NOT fsync on every append. The 64 KB buffer
is flushed to the OS page cache. Data survives process crashes (written to disk by OS
on shutdown / page eviction) but not hardware power loss. If durability against power
loss is required, set `substrate.sync_writes = true` (not yet in the schema — planned).

---

## Hot Path 5: EventBus Publish

`EventBus<E>::publish(event)` is called:
- When a task starts or completes.
- When a gate verdict is emitted.
- When an agent Pulse is emitted.
- On every learning episode flush.

**Implementation**: tokio broadcast channel. `publish()` is a `try_send` on an MPMC
channel — O(N subscribers) with one clone per subscriber.

**Allocation rule**: The event type `E` must be `Clone + Send`. If `E` is a large
struct, use `Arc<E>` to make cloning O(1). For small events (< 256 bytes), direct
cloning is preferred to avoid an `Arc` allocation per event.

**Back-pressure**: If a subscriber's receive buffer is full (capacity: 1,024 events),
`publish()` returns a `Lagged` error — the subscriber is notified it has missed events.
This is by design: publishers are never blocked by slow subscribers.

---

## The "Do Not Do This" List

For contributors and integrators — operations that must not appear on any hot path:

| Forbidden | Why |
|-----------|-----|
| `String::new()` or `String::from(...)` | Heap allocation per call |
| `Vec::new()` or `Vec::with_capacity(...)` | Heap allocation per call |
| `HashMap::new()` | Heap allocation + rehash risk |
| `Box::new(...)` | Heap allocation per call |
| `format!("...")` with non-trivial arguments | Heap allocation |
| `regex::Regex::new(...)` | Compilation + heap allocation |
| Tokio `spawn` (per event/Engram) | Task allocation + scheduler overhead |
| Disk `read()` syscall | Blocks for O(10 µs) minimum |
| Logging at `debug` or lower | Allocates a `String` for the formatted message |

All of these are fine outside the hot path. The rule is specifically about code that
runs in the five paths listed above.

---

## See Also

- [03-memory-model.md](03-memory-model.md) — arena allocator details
- [04-numerical-stability.md](04-numerical-stability.md) — Score and HDC arithmetic
- [07-benchmarks-reference.md](07-benchmarks-reference.md) — verifying hot path behaviour

## Open Questions

- The arena allocator API is not yet public — third-party crates cannot access the tick arena.
- SIMD-accelerated HDC search (AVX2/NEON) is planned but not yet implemented.
