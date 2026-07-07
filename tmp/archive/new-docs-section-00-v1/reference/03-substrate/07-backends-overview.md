# Backends Overview

> `Substrate` is a trait — the backing store is pluggable. This page enumerates the backend
> families, their trade-offs, and how to choose among them.

**Status**: Shipping
**Crate**: `roko-core` (trait), `roko-fs` (file backend), `roko-runtime` (wiring)
**Depends on**: [Trait Surface](./01-trait-surface.md)
**Last reviewed**: 2026-04-19

---

## TL;DR

Three backend families exist today or are specified:

| Backend | File | Status | Use case |
|---|---|---|---|
| JSONL File | `roko-fs` | Shipping | Single-process, laptop/server, durable across restarts |
| In-Memory | `roko-runtime` | Shipping | Tests, short-lived agents, zero-latency benchmarks |
| Chain | — | Specified | Cross-agent verifiable memory (future, chain-dependent) |

---

## Backend Comparison

| Property | JSONL File | In-Memory | Chain |
|---|---|---|---|
| Durability | Survives restarts | Lost on exit | Permanent on-chain |
| Write latency | ~1–5 ms (fsync) | ~1 µs | Seconds (block time) |
| Read latency | ~0.1 ms (index) | ~1 µs | Seconds (RPC) |
| Max scale | ~1M records (tested) | RAM-limited | Unlimited (sharded) |
| HDC index | In-memory linear scan | In-memory linear scan | TBD |
| Concurrent writes | File lock | `Mutex<HashMap>` | On-chain tx ordering |
| Config complexity | Low | Zero | High |

---

## How to Choose

**Use JSONL File** when:
- The agent needs memory that survives process restarts.
- You are running on a single machine (laptop, server, container).
- You want simplicity and inspect-ability (the JSONL log is human-readable).

**Use In-Memory** when:
- You are writing tests and want zero I/O overhead.
- The agent is stateless across runs (or state is managed externally).
- You are benchmarking other parts of the loop without I/O noise.

**Use Chain** (once available) when:
- Multiple agents need a shared, verifiable memory store.
- You need provenance immutability — records that cannot be deleted or altered.
- Deployment is part of the Roko chain-layer stack (see roadmap).

---

## Adding a New Backend

Implement `Substrate`:

```rust
// source: crates/roko-core/src/substrate.rs
use roko_core::{Substrate, SubstrateError, SubstrateQuery, Engram, ContentHash, HdcFingerprint};

pub struct MyBackend { /* ... */ }

impl Substrate for MyBackend {
    fn put(&mut self, engram: Engram) -> Result<(), SubstrateError> { todo!() }
    fn get(&self, id: &ContentHash) -> Result<Option<Engram>, SubstrateError> { todo!() }
    fn query(&self, q: &SubstrateQuery) -> Result<Vec<Engram>, SubstrateError> { todo!() }
    fn query_similar(
        &self,
        fingerprint: &HdcFingerprint,
        k: usize,
    ) -> Result<Vec<Engram>, SubstrateError> { todo!() }
    fn prune(&mut self) -> Result<usize, SubstrateError> { todo!() }
    fn len(&self) -> usize { todo!() }
}
```
<!-- source: crates/roko-core/src/substrate.rs -->

Run the shared backend test suite from `roko-core/tests/substrate_suite.rs` against your
implementation to verify the invariants.

---

## See Also

- [Backend: JSONL File](./08-backend-file-jsonl.md)
- [Backend: In-Memory](./09-backend-in-memory.md)
- [Backend: Chain](./10-backend-chain.md)
- [Invariants](./11-invariants.md) — the test suite every backend must pass

## Open Questions

- Should there be a `CompositiveSubstrate` that reads from one backend and writes to two
  (e.g., in-memory + file for write-through caching)?
- Is a SQLite backend worth shipping between JSONL and chain?
