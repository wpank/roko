# Gate Composition — 11-Gate / 7-Rung Pipelines

> How to compose Gate implementations into layered pipelines, including the reference
> 7-rung gauntlet and how to build a full 11-gate pipeline.

**Status**: Shipping
**Crate**: `roko-gate`
**Depends on**: [Semantics](./02-semantics.md)
**Last reviewed**: 2026-04-19

---

## The 7-Rung Reference Pipeline

The reference production pipeline uses 7 gate rungs that check progressively finer-grained
criteria. Each rung is one `Gate` implementation:

| Rung | Gate | Rejects when |
|---|---|---|
| 1 | `ConfidenceGate { min: 0.3 }` | score.confidence < 0.3 (coarse filter) |
| 2 | `SafetyGate` | body contains prohibited content |
| 3 | `CoherenceGate { min: 0.2 }` | incoherent or contradictory |
| 4 | `FreshnessGate { max_age_days: 90 }` | engram is too old |
| 5 | `AuthorityGate` | reputation < 0.3 |
| 6 | `RelevanceGate` | utility < 0.2 |
| 7 | `ConfidenceGate { min: 0.7 }` | final confidence check (high bar) |

Rungs 1 and 7 both use `ConfidenceGate` with different thresholds: rung 1 is a coarse
pre-filter to save cost on later gates; rung 7 is the final high-confidence bar.

---

## 11-Gate Pipeline

An extended 11-gate pipeline adds: domain gate, novelty gate, precision gate, and a
custom business-logic gate at rungs 8–11. The domain gate uses `Abstain` for out-of-domain
input; the novelty gate rejects if `score.novelty < 0.1` (reject stale re-submissions).

---

## Ordering Principle

Order gates by:
1. **Cheapest first** — fast gates (confidence float comparison) before expensive gates (regex, NLP).
2. **Most-likely-to-reject first** — in profiling, if 80% of rejections are confidence-based,
   put `ConfidenceGate` first.
3. **Safety before quality** — safety gates always come before quality/relevance gates.

---

## Building a Pipeline Programmatically

```rust
// source: crates/roko-gate/src/lib.rs
let pipeline: Vec<Box<dyn Gate>> = GatePipelineBuilder::new()
    .add(ConfidenceGate { min_confidence: 0.3 })
    .add(SafetyGate::default())
    .add(CoherenceGate { min_coherence: 0.2 })
    .add(ConfidenceGate { min_confidence: 0.7 })
    .build();
```
<!-- source: crates/roko-gate/src/lib.rs -->

---

## See Also

- [Trait Composition Model](../01-trait-composition-model.md)
- [Performance](./07-performance.md)
