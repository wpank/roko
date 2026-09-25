# Content-Addressed Artifact Storage

> Depth file for [07-GATES.md](../../07-GATES.md) section 6.
> Source: `crates/roko-gate/src/artifact_store.rs`

---

## 1. Overview

The `ArtifactStore` is a content-addressed, append-only store for gate
artifacts. Every artifact is identified by its BLAKE3 hash. The store
deduplicates automatically: storing the same content twice returns the same
hash without writing a second copy.

Content addressing is a cornerstone of the verification architecture.

---

## 2. Structure

```rust
pub type ContentHash = [u8; 32];

pub struct ArtifactStore {
    items: HashMap<ContentHash, Vec<u8>>,
}
```

The store is an in-memory `HashMap` from 32-byte BLAKE3 hashes to byte
vectors. Intentionally simple -- no filesystem, no database, no network.

### Why BLAKE3

BLAKE3 is chosen over SHA-256 for three reasons:

1. **Speed.** 5-15x faster than SHA-256 on modern hardware, critical when
   hashing megabytes of test output.
2. **Streaming.** Supports incremental hashing without buffering the full
   input.
3. **Keyed mode.** Supports keyed hashing, enabling future per-session
   namespacing without a separate HMAC construction.

---

## 3. Operations

### 3.1 Store

```rust
pub fn store(&mut self, data: &[u8]) -> ContentHash {
    let hash = blake3::hash(data).into();
    self.items.entry(hash).or_insert_with(|| data.to_vec());
    hash
}
```

Computes BLAKE3 hash, inserts if not already present, returns hash.
This is the only write operation.

### 3.2 Retrieve

```rust
pub fn get(&self, hash: &ContentHash) -> Option<&[u8]> {
    self.items.get(hash).map(Vec::as_slice)
}
```

Returns artifact bytes for a given hash, or `None` if unknown.

### 3.3 Contains

```rust
pub fn contains(&self, hash: &ContentHash) -> bool {
    self.items.contains_key(hash)
}
```

Check existence without retrieving data.

### 3.4 Count

```rust
pub fn len(&self) -> usize {
    self.items.len()
}
```

Number of unique artifacts stored.

---

## 4. Immutability and Append-Only Semantics

The store has **no** `delete`, `update`, or `clear` operations in its
public API. Once stored, an artifact exists for the lifetime of the store.

This is a deliberate design constraint:

- **No accidental loss.** A gate artifact used to produce a verdict cannot
  disappear.
- **Audit trail.** The chain from verdict -> artifact hash -> artifact
  content is always intact.
- **Concurrency safety.** Append-only structures have simpler concurrency
  properties than mutable ones.

---

## 5. What Content Addressing Enables

### 5.1 Immutable Artifacts

Hash is identity. No "update" operation exists. An artifact retrieved by
its hash tomorrow contains the same bytes as today.

### 5.2 Deduplication

When an agent retries a task and produces identical output, the store does
not allocate new memory. The BLAKE3 hash matches the existing entry, and
`or_insert_with` short-circuits.

This matters because:

- Gate outputs can be large (megabytes of test runner output)
- Retries are common (3-5 attempts per task is typical)
- Many retries produce identical output on unchanged portions

Deduplication is automatic and zero-cost at the application level.

### 5.3 Reproducibility

Given a hash, retrieve the exact artifact. Same bytes, every time.

### 5.4 Forensic Replay

Any verdict traces to its exact inputs and outputs. Content addressing
makes replay exact: the same hash guarantees the same content, so replayed
inputs are byte-identical to the originals.

---

## 6. Relationship to Gate Verdicts

The artifact store sits alongside the gate pipeline, not inside it:

```
Gate produces verdict with detail (full output)
    |
Orchestrator stores detail in ArtifactStore
    |
ArtifactStore returns ContentHash
    |
ContentHash can be attached to the verdict or signal for later retrieval
```

Gates do not need to know about the artifact store. They produce verdicts
with `detail` strings, and the orchestrator decides what to persist.

---

## 7. Content Addressing Across the System

BLAKE3 content addressing is consistent across the codebase:

| Component | What It Hashes | Hash Algorithm |
|-----------|---------------|----------------|
| `ArtifactStore` | Gate output bytes | BLAKE3 |
| `Signal` (Engram) | Signal content | BLAKE3 |
| `FileSubstrate` | Signal bodies (JSONL) | BLAKE3 |
| `ForensicReplayBuilder` | Causal chain elements | BLAKE3 |
| Worktree fingerprinting | Deterministic gate inputs | BLAKE3 |

Any artifact or signal can be cross-referenced by hash. A verdict's detail
text, stored in the artifact store, hashes to the same value whether you
compute it from the store or from the verdict's detail field.

---

## 8. Runner Gate Adapter

The `RunnerProductionGateAdapter` in `gate_adapter.rs` integrates the
artifact store with the runner event loop. The `FsGeneratedArtifactStore`
provides filesystem-backed storage for generated test artifacts and
evidence bundles, complementing the in-memory `ArtifactStore` for
production use.

---

## 9. Future: Persistent Artifact Store

The current in-memory store is ephemeral. The design anticipates a
persistent version:

### 9.1 Filesystem Layout

```
.roko/artifacts/
+-- ab/
|   +-- ab3f8c1d2e...  (BLAKE3 hash as filename)
|   +-- abd9e4f720...
+-- cd/
|   +-- cd1a2b3c4d...
+-- manifest.jsonl      (hash -> metadata mapping)
```

Two-character prefix directories prevent single-directory performance
issues.

### 9.2 Manifest

A JSONL file mapping hashes to metadata:

```json
{"hash": "ab3f8c1d2e...", "gate": "compile:cargo", "plan": "plan-42",
 "rung": 0, "timestamp": "2026-04-10T12:00:00Z", "size_bytes": 4096}
```

### 9.3 Garbage Collection

Artifacts older than a configurable threshold (e.g., 30 days) with no
references from active plans can be pruned. The JSONL manifest enables
efficient reference counting.

---

## 10. Relationship to Forensic AI

The artifact store is a building block for forensic causal replay. To
replay an agent's verification history:

1. Retrieve the signal by hash from the Substrate.
2. Retrieve the gate artifacts by hash from the ArtifactStore.
3. Replay the gate pipeline with the original signal and compare verdicts.

Content addressing makes this replay exact.

---

## 11. Testing

| Test | What It Verifies |
|------|------------------|
| `store_and_retrieve` | Basic store/get roundtrip |
| `deduplication` | Same content -> same hash, no duplicate storage |
| `missing_hash` | `get()` returns `None` for unstored hashes |
| `contains_check` | `contains()` matches `get().is_some()` |
| `empty_data` | Empty byte slices are valid artifacts |
| `large_data` | Large inputs (megabytes) work correctly |

---

## Verification

```bash
cargo test -p roko-gate -- artifact
```
