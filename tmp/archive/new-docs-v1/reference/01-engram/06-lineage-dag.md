# Engram — Lineage DAG

> Every Engram records the ContentHashes of the Engrams it was derived from. These parent links form a directed acyclic graph (DAG) that enables full audit trails.

**Status**: Shipping  
**Crate**: `roko-core`  
**Depends on**: [ContentHash](02-content-hash.md), [Struct reference](01-struct-reference.md)  
**Used by**: Substrate DAG queries, audit tools, autocatalytic metrics  
**Last reviewed**: 2026-04-19

---

## TL;DR

The `lineage: Vec<ContentHash>` field lists the direct parents of an Engram. The Substrate
maintains the full DAG — ancestors can be traversed arbitrarily deep. The DAG is
append-only and cycle-free. Any decision in the system can be explained by following the
DAG backward: "Why this output? Because of these context Engrams. Where did those come
from? Follow their lineage."

---

## The Idea

Agent outputs are not created from nothing. A GateVerdict derives from the AgentOutput it
judged. An AgentOutput derives from the ContextAssembly that built its prompt. A
ContextAssembly derives from the KnowledgeEntry and Observation Engrams that were
retrieved. Following these parent links gives you the complete causal chain for any
decision.

This is fundamentally different from log-based audit trails:
- Logs are text; lineage DAGs are typed Engrams with their own scores, provenance, and
  decay.
- Logs are append-only files; the DAG is a queryable graph.
- Logs capture what happened; the DAG captures why it happened (causal provenance).

The DAG also powers the **autocatalytic metrics** system: when a KnowledgeEntry is
frequently in the lineage of successful GateVerdicts, its score is reinforced. The system
learns which knowledge is actually useful.

---

## Specification

### The lineage Field

```rust
<!-- source: crates/roko-core/src/engram.rs -->

/// ContentHashes of Engrams this derived from.
/// 
/// Empty for root Engrams (produced from external input, not from other Engrams).
/// Maximum depth is bounded by the substrate's max_lineage_depth configuration.
pub lineage: Vec<ContentHash>,
```

### Root vs. Derived Engrams

- **Root Engram**: `lineage.is_empty()`. Produced from external input (tool response,
  user message, environment observation).
- **Derived Engram**: `lineage.len() > 0`. Produced by processing other Engrams.

### DAG Invariants

1. No cycles: `id` must not appear anywhere in `lineage` transitively (enforced by
   Substrate on ingest).
2. No self-reference: `lineage` must not contain `id`.
3. Parent ids should resolve: each ContentHash in `lineage` should exist in the Substrate
   at ingest time. Dangling references are allowed but flagged (parents may have decayed
   to zero and been GC'd).
4. Lineage entries are distinct: no duplicate ContentHashes in `lineage`.

---

## Semantics

### Building a Derived Engram

```rust
<!-- source: crates/roko-core/src/engram_builder.rs -->

// Build a GateVerdict derived from an AgentOutput
let verdict = EngramBuilder::new()
    .kind(Kind::GateVerdict)
    .body(Body::GateVerdict(GateVerdictBody {
        passed: true,
        gate_name: "syntax_check".to_string(),
        confidence: 0.97,
        rationale: "Code compiles without errors".to_string(),
        rung: 1,
    }))
    .lineage(vec![agent_output.id])  // reference the parent
    .build()?;
```

### DAG Queries in the Substrate

```rust
<!-- source: crates/roko-core/src/substrate.rs -->

trait Substrate {
    /// Return the direct parents of an Engram.
    fn parents(&self, id: &ContentHash) -> Vec<Engram>;

    /// Return the direct children of an Engram.
    fn children(&self, id: &ContentHash) -> Vec<Engram>;

    /// Return all ancestors up to `max_depth`.
    fn ancestors(&self, id: &ContentHash, max_depth: usize) -> Vec<Engram>;

    /// Return all descendants up to `max_depth`.
    fn descendants(&self, id: &ContentHash, max_depth: usize) -> Vec<Engram>;

    /// Return the complete causal chain as an ordered path.
    /// Returns `Err` if the chain has cycles (should not happen; defensive).
    fn causal_chain(
        &self,
        from: &ContentHash,
        to: &ContentHash,
    ) -> Result<Vec<Engram>, SubstrateError>;
}
```

### Audit Trail Example

Given an agent output that was rejected:

```
Observation(id=A)
  └─ KnowledgeEntry(id=B) [derived from A]
       └─ ContextAssembly(id=C) [derived from B + ...]
            └─ AgentOutput(id=D) [derived from C]
                 └─ GateVerdict(id=E, passed=false) [derived from D]
```

To understand why the output was rejected: follow E → D → C → B → A.

---

## Cycle Prevention

The Substrate's ingest pipeline checks for cycles before accepting an Engram:

```rust
<!-- source: crates/roko-core/src/substrate.rs -->

fn detect_cycle(
    substrate: &impl Substrate,
    new_id: ContentHash,
    lineage: &[ContentHash],
) -> bool {
    // BFS: check if new_id appears in the transitive ancestors of any lineage entry
    let mut visited = HashSet::new();
    let mut queue: VecDeque<ContentHash> = lineage.to_vec().into();
    while let Some(id) = queue.pop_front() {
        if id == new_id { return true; }
        if visited.insert(id) {
            for parent in substrate.parents(&id) {
                queue.push_back(parent.id);
            }
        }
    }
    false
}
```

Cycle detection is O(ancestors), bounded by `max_lineage_depth`.

---

## Performance

- **Direct parent lookup**: O(1) from Substrate index
- **Ancestor traversal**: O(depth × branching_factor), typically < 10 µs for depth ≤ 10
- **Cycle detection on ingest**: O(lineage_size × ancestor_depth)
- **Deep audit chains**: for debugging, not for hot paths; acceptable to be O(100 ms)

---

## Invariants

1. `id ∉ lineage` (no self-reference)
2. No transitive cycles (enforced by Substrate ingest)
3. All `lineage` entries are distinct ContentHashes
4. Lineage is included in the identity hash (cannot change without changing `id`)

---

## Failure Modes

| Failure | Cause | Recovery |
|---------|-------|----------|
| Dangling lineage reference | Parent Engram was GC'd after decay to zero | Flagged in Substrate; audit query returns partial chain |
| Cycle detected on ingest | Bug in builder or adversarial input | Substrate rejects the Engram; error logged with full lineage |
| Deep lineage causing slow audit | DAG depth > 1000 | `ancestors()` has `max_depth` parameter; audit tools cap at configurable depth |

---

## See Also

- [`02-content-hash.md`](02-content-hash.md) — how ContentHash identifies parents
- [`10-provenance-fields.md`](10-provenance-fields.md) — author attribution alongside lineage
- [`reference/10-types/provenance/04-custody.md`](../10-types/provenance/04-custody.md) — chain-of-custody for auditable actions
