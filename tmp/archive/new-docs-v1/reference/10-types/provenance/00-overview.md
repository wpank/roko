# Provenance — Overview

> The struct that records an Engram's origin: who created it, how much it is trusted, and any flags indicating unreliability.

**Status**: Shipping  
**Crate**: `roko-core`  
**Depends on**: [Engram](../../01-engram/00-overview.md)  
**Used by**: [ContentHash](../content-hash/00-overview.md), [Score](../score/00-overview.md)  
**Last reviewed**: 2026-04-19

---

## TL;DR

`Provenance` is a three-field struct attached to every Engram: `author` (a string identifier
for the creator), `trust` (a `TrustLevel` enum representing verification depth), and `taint`
(an optional set of flags indicating that the Engram's content may be unreliable). Only
`author` participates in the Engram's identity hash — `trust` and `taint` can be upgraded
after creation without changing the Engram's identity.

---

## The Idea

When an Engram arrives in the Substrate, the system needs to answer three questions:

1. **Who made this?** — The `author` field.
2. **Can it be trusted?** — The `trust` field.
3. **Are there any red flags?** — The `taint` field.

These questions are separable. An Engram's authorship is fixed at creation (it is part of
its identity). Its trust level can rise as it is verified by peers or committed to a chain.
Its taint status can change as new evidence emerges (e.g., its author is later found to be
compromised).

The design deliberately includes only `author` in the content hash. This means:
- Verifying a piece of knowledge (`trust` upgrade) does not create a new identity.
- Flagging knowledge as tainted (`taint` update) does not create a new identity.
- The Engram's hash is stable for its entire lifetime.

---

## Specification

```rust
<!-- source: crates/roko-core/src/provenance.rs -->

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Provenance {
    /// Stable identifier for the creating agent or system.
    /// Included in the Engram's ContentHash.
    pub author: String,

    /// Verification level at the time of last trust update.
    /// NOT included in the ContentHash.
    pub trust: TrustLevel,

    /// Optional flags indicating known unreliability.
    /// NOT included in the ContentHash.
    pub taint: Option<BTreeSet<TaintFlag>>,
}
```

---

## Field Summary

| Field | Type | In hash? | Mutable after creation? | Purpose |
|---|---|---|---|---|
| `author` | `String` | ✓ | No | Identity anchor — who produced this Engram |
| `trust` | `TrustLevel` | ✗ | Yes (escalation only) | Verification depth |
| `taint` | `Option<BTreeSet<TaintFlag>>` | ✗ | Yes | Reliability flags |

---

## Quick Examples

**Local agent creating a KnowledgeEntry:**
```rust
let prov = Provenance::local_agent("agent-001");
// author = "agent-001", trust = LocalAgent(0.25), taint = None
```

**Peer-verified claim:**
```rust
let prov = Provenance {
    author: "agent-001".into(),
    trust: TrustLevel::PeerVerified,
    taint: None,
};
```

**Tainted Engram from an unverified external source:**
```rust
let prov = Provenance {
    author: "external-ingest-42".into(),
    trust: TrustLevel::LocalAgent,
    taint: Some(BTreeSet::from([TaintFlag::UnverifiedSource])),
};
```

---

## Relationship to Score

`TrustLevel` is one of the inputs to `Score::reputation`. A higher trust level contributes
a higher reputation score. Taint flags can reduce the reputation component or cause a Gate
to reject the Engram entirely. See [Score overview](../score/00-overview.md) for details.

---

## Open Questions

- Should the `author` field be a structured type (agent URI, DID, public key) rather
  than a free-form string? Currently a string for simplicity; a structured type is
  planned for the chain-integration phase.
- Should `taint` be a single optional flag or a set? Currently a set to allow multiple
  independent flags. A single flag would be simpler but less expressive.

## See Also

- [`01-author.md`](01-author.md) — author format and conventions
- [`02-trust-level.md`](02-trust-level.md) — TrustLevel variants
- [`03-taint-flags.md`](03-taint-flags.md) — TaintFlag variants
- [`04-hash-inclusion-rules.md`](04-hash-inclusion-rules.md) — which fields enter the hash
