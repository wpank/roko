# Provenance — Hash Inclusion Rules

> A definitive audit of which provenance fields enter the ContentHash and the rationale for each decision.

**Status**: Shipping  
**Crate**: `roko-core`  
**Depends on**: [Overview](00-overview.md), [ContentHash](../content-hash/00-overview.md)  
**Used by**: [Engram Invariants](../../01-engram/12-invariants.md)  
**Last reviewed**: 2026-04-19

---

## TL;DR

Only `provenance.author` is included in the Engram's `ContentHash`. The `trust` and `taint`
fields are excluded. This allows trust escalation and taint flagging to happen after creation
without producing a new Engram identity. The cost is that two Engrams from the same author
with the same content are always the same identity, regardless of their current trust level.

---

## The Rule Table

| Field | Included in hash? | Reason |
|---|---|---|
| `provenance.author` | **Yes** | Author is part of identity — same content from different authors = different claims |
| `provenance.trust` | **No** | Trust can be upgraded after creation; the identity should not change |
| `provenance.taint` | **No** | Taint can be added after creation; the identity should not change |

This table can be verified by reading `canonical_encode()`:

```rust
<!-- source: crates/roko-core/src/content_hash.rs -->

fn canonical_encode(engram: &Engram) -> Vec<u8> {
    let mut buf = Vec::new();
    buf.extend_from_slice(engram.kind.as_bytes());
    buf.extend_from_slice(&engram.body.canonical_bytes());
    buf.extend_from_slice(&engram.created_at_ms.to_le_bytes());
    buf.extend_from_slice(engram.provenance.author.as_bytes());
    // NB: trust and taint are NOT included
    for parent_hash in &engram.lineage {
        buf.extend_from_slice(parent_hash.as_bytes());
    }
    for (k, v) in &engram.tags {
        buf.extend_from_slice(k.as_bytes());
        buf.extend_from_slice(v.as_bytes());
    }
    buf
}
```

---

## Rationale for Including Author

**Why author is in the hash:**

Two agents independently producing the text "Paris is the capital of France" are making the
same claim, but they may have reached it via different reasoning paths, with different
evidence, and with different reliability histories. In a multi-agent system, these are
distinct knowledge nodes. Collapsing them would conflate their independent epistemics.

If you want to deduplicate semantically identical claims across authors, use the
[HDC Fingerprint](../hdc-fingerprint/00-overview.md) for near-duplicate detection rather
than collapsing the identity hash.

---

## Rationale for Excluding Trust

**Why trust is NOT in the hash:**

The lifecycle of a KnowledgeEntry typically looks like:
1. Agent creates it → `trust = LocalAgent`
2. Two peers review it → `trust = PeerVerified`
3. Chain commits it → `trust = ChainWitness`

All three states represent the **same piece of knowledge**. If trust were in the hash,
steps 2 and 3 would create new Engrams, breaking all lineage references pointing to the
original. The Substrate would accumulate three separate Engrams where one is sufficient.

By excluding trust from the hash, the identity is stable and trust escalation is a
metadata update, not an identity change.

---

## Rationale for Excluding Taint

**Why taint is NOT in the hash:**

Taint is a post-hoc signal. The typical flow is:
1. Engram is created (no taint).
2. A week later, the author is identified as compromised.
3. An operator adds `TaintFlag::CompromisedAuthor`.

If taint were in the hash, step 3 would create a new Engram, leaving the original
(without taint) in the Substrate, visible to any consumer that had cached the original
hash. The whole point of taint is to retroactively mark existing Engrams — this only
works if the hash is stable.

---

## Verification via Test

```rust
<!-- source: crates/roko-core/tests/provenance_hash.rs -->

#[test]
fn trust_does_not_affect_hash() {
    let base = Provenance::local_agent("agent-001");
    let escalated = Provenance {
        author: "agent-001".into(),
        trust: TrustLevel::ChainWitness,
        taint: None,
    };
    let e1 = make_engram(base);
    let e2 = make_engram(escalated);
    assert_eq!(e1.id, e2.id);
}

#[test]
fn taint_does_not_affect_hash() {
    let clean = Provenance::local_agent("agent-001");
    let tainted = Provenance {
        author: "agent-001".into(),
        trust: TrustLevel::LocalAgent,
        taint: Some(BTreeSet::from([TaintFlag::PossibleHallucination])),
    };
    let e1 = make_engram(clean);
    let e2 = make_engram(tainted);
    assert_eq!(e1.id, e2.id);
}

#[test]
fn different_authors_produce_different_hashes() {
    let p1 = Provenance::local_agent("agent-001");
    let p2 = Provenance::local_agent("agent-002");
    // Same content, different authors
    let e1 = make_engram_with_text("Paris", p1);
    let e2 = make_engram_with_text("Paris", p2);
    assert_ne!(e1.id, e2.id);
}
```

---

## Invariants

1. `canonical_encode()` must not read `provenance.trust` or `provenance.taint`.
2. Any change to `author` must produce a different hash.
3. Any change to `trust` or `taint` must produce the **same** hash.
4. This contract is verified by the three tests above in CI.

---

## Open Questions

- Should the hash include a version marker so that future changes to `canonical_encode()`
  can be detected? Not currently included; all hashes are v1 implicitly.
- If `author` is eventually replaced by a structured `AuthorId`, the hash input must be
  the canonical serialization of `AuthorId`, not its debug string.

## See Also

- [`00-overview.md`](00-overview.md) — full Provenance struct
- [`../content-hash/00-overview.md`](../content-hash/00-overview.md) — how the hash is computed
- [`../../01-engram/12-invariants.md`](../../01-engram/12-invariants.md) — Engram invariants
