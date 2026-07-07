# Engram — ContentHash Identity

> The ContentHash is the Engram's identity: a 32-byte BLAKE3 digest of the Engram's stable fields.

**Status**: Shipping  
**Crate**: `roko-core`  
**Depends on**: [ContentHash type](../10-types/content-hash/00-overview.md)  
**Used by**: [Lineage DAG](06-lineage-dag.md), Substrate, Gate  
**Last reviewed**: 2026-04-19

---

## TL;DR

An Engram's identity is computed by hashing its stable fields (kind, body, created_at_ms,
provenance.author, lineage, tags) with BLAKE3. Score, decay, and fingerprint are excluded.
The same content always produces the same hash. The Substrate uses ContentHash for exact-match
lookups; the HDC fingerprint handles similarity search.

---

## The Idea

Content addressing means: two records with identical content have identical addresses.
You never need an external id generator — the content IS the id. This has three immediate
consequences for Roko:

1. **Deduplication is free.** If the same agent output is produced twice (same content,
   same author, same tags), the substrate's `insert` is idempotent — the second insertion
   is a no-op because the ContentHash already exists.

2. **Lineage is cryptographically verifiable.** Because parent Engrams' ContentHashes are
   included in child Engrams' `lineage` field (which is itself hashed), the audit DAG
   cannot be silently tampered with. Changing a parent Engram changes its hash, which
   changes the child's lineage field, which changes the child's hash.

3. **Distributed consensus is natural.** Two agents holding identical Engrams have
   identical hashes — no synchronization protocol needed to verify identity.

---

## Specification

### The ContentHash Type

```rust
<!-- source: crates/roko-core/src/content_hash.rs -->

/// A 32-byte content-addressed identifier (BLAKE3 digest).
///
/// Two Engrams with identical canonical encoding share the same ContentHash.
/// ContentHash is the primary key in every Substrate implementation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ContentHash([u8; 32]);

impl ContentHash {
    /// Compute from a serialized canonical byte representation.
    pub fn from_bytes(canonical: &[u8]) -> Self {
        let digest = blake3::hash(canonical);
        ContentHash(*digest.as_bytes())
    }

    /// Display as lowercase hex (64 chars).
    pub fn to_hex(&self) -> String { /* ... */ }

    /// Parse from hex string.
    pub fn from_hex(s: &str) -> Result<Self, ContentHashError> { /* ... */ }

    /// Raw 32-byte array.
    pub fn as_bytes(&self) -> &[u8; 32] { &self.0 }
}
```

### What Is Hashed

The following fields are included in the canonical encoding:

| Field | Encoding rule |
|-------|--------------|
| `kind` | Discriminant tag as little-endian u32 |
| `body` | Canonical bytes of Body variant (see Body serialization spec) |
| `created_at_ms` | Little-endian i64 |
| `provenance.author` | UTF-8 bytes of the author string |
| `lineage` | Each ContentHash as 32-byte array; count prefixed as u32 LE |
| `tags` | Each key-value pair as length-prefixed UTF-8, in BTreeMap iteration order |

The following fields are **NOT** hashed:

| Field | Why excluded |
|-------|-------------|
| `score` | Scorers recompute without changing identity |
| `decay` | Substrates adjust decay model without changing identity |
| `fingerprint` | Encoder upgrades regenerate without changing identity |
| `id` itself | Recursive — would require circular hashing |

For the precise byte-level encoding rules, see
[`../10-types/content-hash/01-canonicalization.md`](../10-types/content-hash/01-canonicalization.md).

### Why BLAKE3

BLAKE3 was chosen for:

1. **Speed**: BLAKE3 runs at ~1 GB/s on a single core, well above the throughput of
   Engram emission. Hashing a typical Engram takes < 1 µs.

2. **Security**: 256-bit output; no known collision attacks; NIST finalist lineage
   (BLAKE2 predecessor). Suitable for content-addressing where collision resistance
   matters.

3. **Simplicity**: One algorithm, one output size, no parameter choices.

See [`../10-types/content-hash/02-collision-policy.md`](../10-types/content-hash/02-collision-policy.md)
for the collision probability analysis (spoiler: trust BLAKE3).

---

## Semantics

### Computing the ContentHash

`EngramBuilder::build()` computes the ContentHash automatically:

```rust
<!-- source: crates/roko-core/src/engram_builder.rs -->

fn compute_id(engram: &Engram) -> ContentHash {
    let canonical = canonical_encode(
        &engram.kind,
        &engram.body,
        engram.created_at_ms,
        &engram.provenance.author,
        &engram.lineage,
        &engram.tags,
    );
    ContentHash::from_bytes(&canonical)
}
```

### Verification

Any component can verify an Engram's id at any time by recomputing the hash:

```rust
<!-- source: crates/roko-core/src/engram.rs -->

impl Engram {
    pub fn verify_id(&self) -> bool {
        let recomputed = compute_id(self);
        recomputed == self.id
    }
}
```

The Substrate calls `verify_id()` on every ingested Engram.

---

## API Reference

See [`../10-types/content-hash/04-api-reference.md`](../10-types/content-hash/04-api-reference.md)
for the full ContentHash API.

---

## Invariants

1. `id == blake3(canonical_encode(kind, body, created_at_ms, provenance.author, lineage, tags))`
2. ContentHash is 32 bytes, always.
3. The canonical encoding is deterministic: same input → same bytes → same hash.

---

## Failure Modes

| Failure | Cause | Detection |
|---------|-------|-----------|
| Hash mismatch on ingest | Engram modified after construction | Substrate `verify_id()` on ingest |
| Non-deterministic hash | Non-canonical field ordering | Prevented by `BTreeMap<String, String>` for tags |
| Collision | Two distinct Engrams with same hash | Probability ≈ 2⁻¹²⁸ per pair; treated as impossible |

---

## See Also

- [`../10-types/content-hash/00-overview.md`](../10-types/content-hash/00-overview.md)
- [`../10-types/content-hash/01-canonicalization.md`](../10-types/content-hash/01-canonicalization.md)
- [`03-hdc-fingerprint.md`](03-hdc-fingerprint.md) — semantic similarity (vs. exact hash)
- [`06-lineage-dag.md`](06-lineage-dag.md) — how ContentHash enables the audit DAG
