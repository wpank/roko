# Provenance — Propagation

> How provenance flows when one Engram is derived from another via lineage links.

**Status**: Shipping  
**Crate**: `roko-core`  
**Depends on**: [Overview](00-overview.md), [Taint Flags](03-taint-flags.md)  
**Last reviewed**: 2026-04-19

---

## TL;DR

When Agent A creates a derived Engram from parent Engrams B and C, the derived Engram gets
its own `author` (Agent A), its own `trust` level (starts at `LocalAgent`), and inherits
selected `taint` flags from parents. `author` is never inherited — each Engram has exactly
one author. `trust` is never inherited — each Engram earns trust independently.

---

## The Idea

Derivation is common in Roko: an agent summarizes two KnowledgeEntries into a third, or
combines a Plan and a ToolTrace into a Reflection. The derived Engram carries `lineage`
links to its parents, but its authorship is the deriving agent, not the original authors.

This design means that trust must be independently established for every new Engram. A chain
of `ChainWitness` parents does not make a derived Engram `ChainWitness` — it must earn that
designation on its own.

---

## What Is Inherited

| Field | Inherited? | Rule |
|---|---|---|
| `author` | No | Derived Engram's author is the deriving agent |
| `trust` | No | Starts at `LocalAgent`; must be escalated independently |
| `taint` | Partially | See propagation rule below |

### Taint Propagation Rule

Only these two flags propagate from parent to child:
- `CompromisedAuthor` — if a parent was produced by a compromised agent, the derived
  work is also suspect.
- `PossibleHallucination` — if a parent contained hallucinated content, the child may
  inherit the hallucination.

All other flags (`UnverifiedSource`, `OutdatedAt`, `SuspiciousContext`, `Custom`) are
specific to the Engram where they were set and do **not** propagate.

```rust
<!-- source: crates/roko-core/src/provenance.rs -->

impl Provenance {
    /// Compute the taint set for a derived Engram given its parents.
    pub fn inherit_taint(parents: &[&Provenance]) -> Option<BTreeSet<TaintFlag>> {
        let inherited: BTreeSet<TaintFlag> = parents.iter()
            .filter_map(|p| p.taint.as_ref())
            .flat_map(|flags| flags.iter())
            .filter(|flag| matches!(flag,
                TaintFlag::CompromisedAuthor | TaintFlag::PossibleHallucination
            ))
            .cloned()
            .collect();
        if inherited.is_empty() { None } else { Some(inherited) }
    }
}
```

---

## Builder Usage

```rust
<!-- source: crates/roko-core/src/builder.rs -->

let derived = Engram::builder()
    .kind(Kind::Reflection)
    .body(Body::Text("Summary of planning session".into()))
    .provenance(Provenance {
        author: "agent-003".into(),
        trust: TrustLevel::LocalAgent,
        // Inherit taint from parents:
        taint: Provenance::inherit_taint(&[
            &parent_a.provenance,
            &parent_b.provenance,
        ]),
    })
    .lineage(vec![parent_a.id, parent_b.id])
    .build()?;
```

---

## Worked Example

```
parent_A:
  author = "agent-001"
  trust  = PeerVerified
  taint  = Some({CompromisedAuthor})

parent_B:
  author = "external-ingest-99"
  trust  = LocalAgent
  taint  = Some({UnverifiedSource})

derived:
  author = "agent-003"           ← deriving agent
  trust  = LocalAgent            ← starts fresh
  taint  = Some({CompromisedAuthor})  ← inherited from parent_A
                                      ← UnverifiedSource NOT inherited (non-propagating flag)
```

---

## Invariants

1. Derived Engram's `author` is always the deriving agent — never a parent's author.
2. Derived Engram's `trust` starts at `LocalAgent` — never inherited.
3. Only `CompromisedAuthor` and `PossibleHallucination` propagate from parents.
4. If no propagating flags exist in any parent, derived `taint = None`.
5. Lineage links (`engram.lineage`) record parent `ContentHash` values — they are
   the structural parent pointers, separate from provenance inheritance.

---

## Failure Modes

| Failure | Cause | Recovery |
|---|---|---|
| Author incorrectly inherited | Builder copies parent provenance | Builder always requires explicit `author` field |
| `CompromisedAuthor` not inherited | Taint propagation skipped | Substrate validates derived Engrams have correct taint inheritance |
| Over-propagation of `UnverifiedSource` | Incorrect propagation logic | Unit tests verify `UnverifiedSource` does not appear in derived taint |

---

## Open Questions

- Should trust propagation be allowed as an optional override? E.g., an operator could
  choose to inherit `PeerVerified` from all-verified parents. Not currently supported.
- Should there be a maximum propagation depth (e.g., only propagate from direct parents,
  not grandparents)? Currently depth-unlimited via lineage traversal; may be capped.

## See Also

- [`03-taint-flags.md`](03-taint-flags.md) — full TaintFlag reference
- [`06-trust-escalation.md`](06-trust-escalation.md) — how derived Engrams earn trust
- [`../../01-engram/06-lineage-dag.md`](../../01-engram/06-lineage-dag.md) — lineage DAG structure
