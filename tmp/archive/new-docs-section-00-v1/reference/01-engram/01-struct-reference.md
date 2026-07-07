# Engram — Struct Reference

> Complete field-by-field specification of the Engram struct, every type, every invariant.

**Status**: Shipping  
**Crate**: `roko-core`  
**Depends on**: [ContentHash](../10-types/content-hash/00-overview.md), [Score](../10-types/score/00-overview.md), [Decay](../10-types/decay/00-overview.md), [Provenance](../10-types/provenance/00-overview.md), [Kind](../10-types/kind/00-overview.md), [Body](../10-types/body/00-overview.md), [HdcFingerprint](../10-types/hdc-fingerprint/00-overview.md)  
**Used by**: every subsystem  
**Last reviewed**: 2026-04-19

---

## TL;DR

The Engram struct has ten fields. Three establish identity and content (`id`, `kind`, `body`).
Two handle the time dimension (`created_at_ms`, `decay`). Four attach quality and provenance
metadata (`score`, `provenance`, `fingerprint`, `lineage`). One carries freeform metadata
(`tags`). Every field is serializable; three are excluded from the identity hash
(`score`, `decay`, `fingerprint`).

---

## The Idea

The struct is designed so that identity (`id`) is stable across mutations that do not change
the content — only the quality or operational parameters. A Scorer can recompute a `Score`
without producing a new Engram. A decay model can be changed without breaking content
addressing. The fingerprint can be regenerated with a newer encoder without changing identity.

---

## Specification

### Full Struct

```rust
<!-- source: crates/roko-core/src/engram.rs -->

/// The universal datum of the Roko system.
///
/// # Identity
/// An Engram's identity is its ContentHash, computed from:
///   kind + body + provenance.author + tags (stable fields)
/// The following are NOT included in the hash:
///   score, decay, fingerprint (mutable metadata)
///
/// # Shipping note
/// The current codebase uses the identifier `Signal` for this type.
/// `Engram` is the canonical architectural name. Migration is tracked in
/// reference/01-engram/15-rationale-and-history.md.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Engram {
    /// Content-addressed identity. Computed by EngramBuilder or explicitly set.
    /// Invariant: id == blake3(canonical_encode(kind, body, provenance.author, tags))
    pub id: ContentHash,

    /// Semantic fingerprint for HDC similarity search.
    /// None only when the HDC encoder is explicitly disabled (e.g., test fixtures
    /// that do not exercise search paths).
    pub fingerprint: Option<HdcFingerprint>,

    /// What category of information this record represents.
    /// Determines how Body is interpreted.
    pub kind: Kind,

    /// The payload — typed content for this Kind.
    pub body: Body,

    /// Unix timestamp in milliseconds at the instant this Engram was emitted.
    /// Set once by the builder; never mutated.
    pub created_at_ms: i64,

    /// Decay model governing how this Engram's weight decreases over time.
    /// Not included in identity hash — can be adjusted without changing id.
    pub decay: Decay,

    /// Author attribution and trust classification.
    pub provenance: Provenance,

    /// Quality score at emission time.
    /// Not included in identity hash — Scorers can recompute without new id.
    pub score: Score,

    /// Content hashes of Engrams this derived from.
    /// Forms the edges of the audit DAG. Empty for root Engrams.
    pub lineage: Vec<ContentHash>,

    /// Arbitrary key-value metadata.
    /// Included in identity hash; must be ordered (BTreeMap) for stability.
    pub tags: BTreeMap<String, String>,
}
```

### HdcFingerprint Sub-struct

```rust
<!-- source: crates/roko-core/src/engram.rs -->

/// Semantic fingerprint for an Engram.
/// Used by Substrate for similarity search alongside exact ContentHash lookup.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct HdcFingerprint {
    /// The 10,240-bit binary spatter code (BSC) vector.
    pub vector: HdcVector,

    /// Registry version of the encoder that produced this vector.
    /// Required for deterministic cross-version comparison.
    /// Two fingerprints are only comparable if encoder_version matches.
    pub encoder_version: u32,
}
```

---

## Field Reference Table

| Field | Type | In hash? | Mutable? | Notes |
|-------|------|----------|----------|-------|
| `id` | `ContentHash` | — (is the hash) | No | Computed from stable fields |
| `fingerprint` | `Option<HdcFingerprint>` | No | Yes | Regeneratable from body+kind |
| `kind` | `Kind` | Yes | No | Determines Body interpretation |
| `body` | `Body` | Yes | No | The payload |
| `created_at_ms` | `i64` | Yes | No | Unix ms, set at emission |
| `decay` | `Decay` | No | Yes | Model can be changed by substrate |
| `provenance` | `Provenance` | Yes (author only) | Partial | See provenance spec |
| `score` | `Score` | No | Yes | Scorers recompute freely |
| `lineage` | `Vec<ContentHash>` | Yes | No | Audit DAG edges |
| `tags` | `BTreeMap<String, String>` | Yes | No | Ordered for hash stability |

**Hash-included fields**: `kind`, `body`, `created_at_ms`, `provenance.author`, `lineage`, `tags`  
**Hash-excluded fields**: `score`, `decay`, `fingerprint`

For the precise canonicalization algorithm, see
[`../10-types/content-hash/01-canonicalization.md`](../10-types/content-hash/01-canonicalization.md).

---

## Semantics

### Field: `id: ContentHash`

The `id` is the Engram's identity. It is a 32-byte BLAKE3 digest computed once — either
by `EngramBuilder::build()` or by direct construction when deserializing from a verified
store. The id is **immutable** after the Engram is built.

Two Engrams with the same `kind`, `body`, `provenance.author`, `created_at_ms`, `lineage`,
and `tags` will have the same `id`. This is the content-addressing guarantee.

### Field: `fingerprint: Option<HdcFingerprint>`

The fingerprint is a semantic summary of the Engram's content encoded as a 10,240-bit BSC
vector. It enables approximate similarity search: "find me Engrams about topic X" without
knowing the exact `id`. The `encoder_version` field ensures that fingerprints generated by
different encoder versions are not compared incorrectly.

`fingerprint` is `None` only when:
1. The HDC encoder was explicitly disabled in the substrate configuration.
2. The Engram was constructed manually in a test without an encoder.

In production, `fingerprint` is always `Some`.

### Field: `kind: Kind`

The Kind is a discriminant telling operators how to interpret the Body. See
[`04-kind-enum.md`](04-kind-enum.md) for all variants. The Kind is part of the identity
hash — changing Kind produces a different Engram.

### Field: `body: Body`

The Body carries the actual payload. Its type varies by Kind — the Body enum has one
variant per Kind. See [`05-body-enum.md`](05-body-enum.md). The Body is part of the
identity hash.

### Field: `created_at_ms: i64`

Unix timestamp in milliseconds at the moment of emission. Set once by the builder; never
changed. Negative values are invalid. Future timestamps (clock skew > 60 s) trigger a
warning from the substrate but are not rejected.

### Field: `decay: Decay`

The decay model governs how this Engram's effective weight changes over time. See
[`09-decay-fields.md`](09-decay-fields.md) and [`../10-types/decay/`](../10-types/decay/README.md).
Excluded from the identity hash so that substrates can upgrade decay models for existing
Engrams (e.g., promoting a cold-tier Engram to Demurrage) without changing its id.

### Field: `provenance: Provenance`

Provenance records who produced this Engram, at what trust level, and what taint it
inherited. Only `provenance.author` is included in the identity hash — taint and trust
level can change as an Engram moves through the system. See
[`10-provenance-fields.md`](10-provenance-fields.md) and
[`../10-types/provenance/`](../10-types/provenance/README.md).

### Field: `score: Score`

A 7-axis quality score. Not part of identity — Scorers are designed to recompute scores
on existing Engrams. For example, after an outcome becomes known, a Scorer can update the
`utility` axis without creating a new Engram. See [`08-scoring-fields.md`](08-scoring-fields.md).

### Field: `lineage: Vec<ContentHash>`

The content hashes of the Engrams this was derived from. A root Engram (produced from
external input, not from other Engrams) has an empty lineage. Derived Engrams list their
parent Engrams' ids here. The substrate uses lineage to maintain the audit DAG. See
[`06-lineage-dag.md`](06-lineage-dag.md).

### Field: `tags: BTreeMap<String, String>`

Ordered string key-value metadata. Included in the identity hash — adding a tag changes
the Engram's id. Common uses: agent session ids, task ids, model names, tool names,
run-specific metadata. The BTreeMap ordering is essential for hash stability (see
[`../10-types/content-hash/01-canonicalization.md`](../10-types/content-hash/01-canonicalization.md)).

---

## Invariants

1. `id == blake3(canonical_encode(kind, body, created_at_ms, provenance.author, lineage, tags))`
2. `created_at_ms > 0`
3. `tags` keys and values are valid UTF-8 (enforced by BTreeMap<String, String>)
4. `lineage` entries are distinct (no duplicate parent hashes)
5. `lineage` entries do not contain `id` (no self-reference)
6. If `fingerprint.is_some()`, `fingerprint.encoder_version` is a registered version
7. `body` is compatible with `kind` (the Body variant matches the Kind variant)

For enforcement points, see [`12-invariants.md`](12-invariants.md).

---

## Failure Modes

| Failure | Trigger | Recovery |
|---------|---------|----------|
| Hash mismatch | `id` does not match recomputed hash | Substrate rejects on ingest; log provenance for audit |
| Future timestamp | `created_at_ms` far ahead of wall clock | Substrate warns; accepts with skew flag |
| Unknown encoder version | `fingerprint.encoder_version` not in registry | Similarity search skipped; hash lookup still works |
| Body-Kind mismatch | Body variant inconsistent with Kind | Rejected at build time by EngramBuilder |
| Lineage cycle | An ancestor references this Engram | Detected and rejected by substrate DAG checker |

---

## API Reference

See [`14-api-reference.md`](14-api-reference.md) for the full API surface.

---

## See Also

- [`02-content-hash.md`](02-content-hash.md) — how `id` is computed
- [`03-hdc-fingerprint.md`](03-hdc-fingerprint.md) — how `fingerprint` is computed
- [`06-lineage-dag.md`](06-lineage-dag.md) — how `lineage` forms the audit DAG
- [`12-invariants.md`](12-invariants.md) — where each invariant is enforced
