# Lineage Acyclicity

> An Engram's parent chain is always a DAG — never a cycle. An Engram cannot be its own ancestor.

**Crate**: `roko-core`
**Test type**: Property-based (proptest)
**Enforcement**: `Engram::new`, `Lineage::add_parent`
**Last reviewed**: 2026-04-19

---

## Statement

For all valid Engram graphs G: G contains no directed cycle in the parent-of relation.

Equivalently: the set of all Engrams and their parent links forms a DAG (directed acyclic graph). An Engram E cannot appear in its own ancestor chain.

---

## Why It Matters

Lineage is used for:
- **Provenance**: tracing where a piece of knowledge came from.
- **Deduplication**: detecting when two agents derived the same Engram from the same source.
- **Decay propagation**: a child Engram's decay can be linked to its parent's decay.
- **Causal replay**: the forensic replay system follows parent chains backward.

A cycle in the lineage would make provenance tracing non-terminating, decay propagation infinite, and causal replay enter an infinite loop.

---

## Enforcement

The `Engram::new` constructor accepts a `parent: Option<ContentHash>`. The parent is looked up in the substrate at construction time. The substrate's `lineage_check` method verifies that adding the proposed parent does not create a cycle by walking the existing lineage chain.

```rust
impl Engram {
    pub fn new(
        body: Body,
        parent: Option<ContentHash>,
        substrate: &impl Substrate,
    ) -> Result<Self, LineageError> {
        if let Some(ref parent_hash) = parent {
            // Walk the parent's ancestors; if we find our own hash, it's a cycle
            substrate.check_lineage_acyclic(parent_hash, &our_hash)?;
        }
        // …construction…
    }
}
```

<!-- source: crates/roko-core/src/engram.rs -->

---

## Property Test

```rust
use proptest::prelude::*;
use roko_test::strategies::arb_lineage_dag;

proptest! {
    #[test]
    fn lineage_is_acyclic(dag in arb_lineage_dag(10)) {
        // arb_lineage_dag generates valid DAGs by construction (topological order insertion)
        for engram in &dag {
            let ancestors: Vec<_> = dag.ancestors_of(engram.id()).collect();
            prop_assert!(
                !ancestors.contains(&engram.id()),
                "Engram {:?} must not appear in its own ancestor chain",
                engram.id()
            );
        }
    }
}
```

**File**: `crates/roko-core/src/lineage.rs` (test module)

---

## Failure Mode

If an Engram construction with a cyclic parent is attempted:
- `LineageError::CycleDetected { engram_id, cycle_path }` is returned.
- The Engram is not written to the substrate.
- The cycle path in the error identifies the full cycle for debugging.

A cycle in a stored Engram (substrate corruption) is detected during lineage queries and reported as a `SubstrateError::CorruptLineage`.

---

## Related Properties

- [plan-dag-acyclicity.md](plan-dag-acyclicity.md) — same structural invariant in the plan layer
- [provenance-chain-integrity.md](provenance-chain-integrity.md) — provenance uses the lineage structure

## See also

- [../by-subsystem/subsystem-core.md](../by-subsystem/subsystem-core.md)
