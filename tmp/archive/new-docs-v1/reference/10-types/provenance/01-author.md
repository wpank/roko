# Provenance — Author Field

> The string that identifies who produced an Engram. The only provenance field included in the ContentHash.

**Status**: Shipping  
**Crate**: `roko-core`  
**Depends on**: [Overview](00-overview.md)  
**Used by**: [ContentHash](../content-hash/00-overview.md)  
**Last reviewed**: 2026-04-19

---

## TL;DR

`author` is a free-form UTF-8 string that identifies the Engram's creator. It is the only
provenance field that participates in the Engram's identity hash, which means two Engrams
with identical content but different authors are considered **distinct Engrams** — different
identities, different hashes. The format is currently a string convention; a structured type
(DID or agent URI) is planned.

---

## The Idea

Identity in Roko is not just "what does this Engram contain" but also "who says so". Two
agents can independently produce Engrams with identical text bodies. They should have
different identities, because their reliability and lineage are different. The `author`
field carries this identity anchor.

The author is fixed at creation. It cannot be changed after the Engram is hashed. Changing
the author would produce a new hash — a new identity — which is the correct behaviour for
derivative works rather than mutating an existing one.

---

## Format Conventions

Author strings follow the convention `"<type>-<id>"`:

| Type | Example | Meaning |
|---|---|---|
| `agent` | `"agent-001"` | A Roko agent instance, identified by its numeric ID |
| `tool` | `"tool-rust-analyzer"` | A tool invocation |
| `external` | `"external-ingest-42"` | Ingest from an external data source |
| `system` | `"system-boot"` | System-level Engrams created at startup |
| `human` | `"human-will"` | Human-authored input (for human-in-the-loop workflows) |

These are conventions, not enforced by the type system. Any non-empty string is accepted.

<!-- ADDED: format conventions — not explicit in source docs; inferred from codebase usage
patterns. The structured-type roadmap item is from the source docs. -->

---

## Specification

```rust
<!-- source: crates/roko-core/src/provenance.rs -->

/// Author identifier. Included in the Engram's ContentHash.
/// Must be non-empty.
pub author: String,
```

Validation at construction:

```rust
<!-- source: crates/roko-core/src/provenance.rs -->

impl Provenance {
    pub fn new(
        author: impl Into<String>,
        trust: TrustLevel,
        taint: Option<BTreeSet<TaintFlag>>,
    ) -> Result<Self, ProvenanceError> {
        let author = author.into();
        if author.is_empty() {
            return Err(ProvenanceError::EmptyAuthor);
        }
        Ok(Provenance { author, trust, taint })
    }

    /// Convenience constructor for a local agent with default trust.
    pub fn local_agent(id: impl Into<String>) -> Self {
        Provenance {
            author: format!("agent-{}", id.into()),
            trust: TrustLevel::LocalAgent,
            taint: None,
        }
    }
}
```

---

## Author in the ContentHash

The `canonical_encode()` function includes `author` directly as a UTF-8 byte sequence:

```rust
<!-- source: crates/roko-core/src/content_hash.rs -->

// Inside canonical_encode():
hasher.update(provenance.author.as_bytes());
// trust and taint are NOT passed to the hasher
```

This means:
- Same author + same content = same hash (no duplicates for the same author).
- Different author + same content = different hash (provenance is baked in).

---

## Invariants

1. `author` is non-empty — enforced at construction with `ProvenanceError::EmptyAuthor`.
2. `author` is valid UTF-8 — enforced by `String` type.
3. `author` is immutable after construction — the `Provenance` struct does not expose
   a setter for `author`.
4. `author` changes produce a new `ContentHash` — a logical consequence of inclusion in
   `canonical_encode()`.

---

## Failure Modes

| Failure | Cause | Recovery |
|---|---|---|
| Duplicate Engrams from different agents | Author field ensures separate hashes | No action needed; this is correct behaviour |
| Empty author accepted | Pre-validation bypass in legacy code | Add `ProvenanceError::EmptyAuthor` check to all construction paths |
| Author contains newlines/control chars | Free-form string with no sanitization | Recommend sanitization in `local_agent()` constructor |

---

## Future: Structured Author Type

The long-term plan is to replace `String` with a structured type:

```rust
<!-- ADDED: target-state spec — no code exists yet -->

pub enum AuthorId {
    Agent { id: AgentId },
    Did(String),           // W3C DID for chain-integrated agents
    External { tag: String },
    System,
    Human { name: String },
}
```

This would enable stronger typing, DID verification, and cross-chain identity resolution.
It is deferred until the chain integration phase.

---

## Open Questions

- Should the author field be length-capped (e.g., 256 bytes) to prevent abuse?
  Not currently capped.
- Should the author be a cryptographic public key for tamper-evident attribution?
  Deferred to chain integration phase.

## See Also

- [`00-overview.md`](00-overview.md) — full Provenance struct
- [`04-hash-inclusion-rules.md`](04-hash-inclusion-rules.md) — full hash field audit
- [`../content-hash/00-overview.md`](../content-hash/00-overview.md) — ContentHash details
