# ContentHash — Overview

> A 32-byte BLAKE3 digest of an Engram's canonical fields, serving as its unique, stable identity key.

**Status**: Shipping  
**Crate**: `roko-core`  
**Depends on**: [Engram](../../01-engram/00-overview.md)  
**Used by**: [Substrate](../../../subsystems/substrate/), [Lineage DAG](../../01-engram/06-lineage-dag.md)  
**Last reviewed**: 2026-04-19

---

## TL;DR

Every Engram has a `ContentHash` stored in its `id` field. It is a 32-byte BLAKE3 digest
computed over a canonical encoding of the Engram's stable fields: `kind`, `body`,
`created_at_ms`, `provenance.author`, `lineage`, and `tags`. Fields that can change after
creation (`decay`, `score`, `provenance.trust`, `provenance.taint`, `fingerprint`) are
excluded. The same content from the same author at the same time always produces the same
hash; different content always produces a different hash with overwhelming probability.

---

## The Idea

Roko uses content-addressing rather than assigned IDs. There is no auto-incrementing integer
or random UUID. An Engram's identity is derived entirely from its content and provenance.

This has several consequences:

**Deduplication is automatic**: If two agents independently produce an Engram with the same
kind, body, author, and creation time, they get the same `ContentHash` — the Substrate
treats them as the same Engram.

**Lineage references are stable**: Parent Engrams are referenced by `ContentHash` in the
`lineage` field. Because the hash is stable, a lineage reference from 6 months ago still
points to the correct Engram. No foreign key maintenance is needed.

**Mutations require a new Engram**: If you want to update an Engram's body, you create a
new Engram with the updated content and add the old Engram to its `lineage`. The old Engram
is not deleted — it is a permanent record.

---

## What Goes Into the Hash

| Field | Included? | Notes |
|---|---|---|
| `kind` | Yes | The Kind variant's canonical bytes |
| `body` | Yes | The Body's canonical byte encoding |
| `created_at_ms` | Yes | Little-endian i64 |
| `provenance.author` | Yes | UTF-8 bytes |
| `lineage` | Yes | Ordered list of parent ContentHash bytes |
| `tags` | Yes | Sorted BTreeMap key-value bytes |
| `score` | **No** | Can be recomputed; excluded for mutability |
| `decay` | **No** | Can be upgraded; excluded for mutability |
| `provenance.trust` | **No** | Can be escalated; excluded for mutability |
| `provenance.taint` | **No** | Can be added; excluded for mutability |
| `fingerprint` | **No** | Encoder upgrades must be transparent |

---

## Type Definition

```rust
<!-- source: crates/roko-core/src/content_hash.rs -->

/// A 32-byte BLAKE3 digest of an Engram's canonical fields.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ContentHash([u8; 32]);
```

---

## Security Properties

BLAKE3 provides:
- **Collision resistance**: Probability of two distinct canonical encodings producing the
  same hash is approximately `2^-128`. Practically impossible.
- **Preimage resistance**: Given a hash, it is computationally infeasible to find the
  original canonical encoding.
- **Performance**: BLAKE3 processes ~3 GB/s on modern hardware; hashing a typical Engram
  takes well under 1 µs.

---

## Open Questions

- Should `ContentHash` include a version byte to allow future hash algorithm changes?
  Not currently included; all hashes are implicitly BLAKE3 v1.
- Should the hash be truncated to 16 bytes for storage compactness? Not planned;
  collision risk is too important to compromise.

## See Also

- [`01-canonical-encoding.md`](01-canonical-encoding.md) — exact byte layout
- [`02-api-reference.md`](02-api-reference.md) — methods
- [`../provenance/04-hash-inclusion-rules.md`](../provenance/04-hash-inclusion-rules.md) — field-by-field audit
