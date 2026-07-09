# Finding: Engram/Signal Universality

> Edge cases that stress the universal Engram type, comparison to the Agent Data Protocol,
> and the VSA/HDC algebraic extension opportunity.

**Status**: Analysis
**Crate**: `roko-core`
**Depends on**: [Engram Data Type](../../reference/01-engram/README.md)
**Last reviewed**: 2026-04-13

---

## TL;DR

The Engram/Signal type is genuinely universal. All data categories fit without awkward
workarounds. Engram is strictly richer than the Agent Data Protocol (ADP), validating the
core design choice. The HDC algebraic extension (bind, bundle, permute) is a natural next
step that would make Signal a proper Vector Symbolic Architecture element.

---

## What the Universal Type Handles Well

| Data Category | Engram Representation | Fit |
|---|---|---|
| LLM output | `Kind::AgentOutput`, `Body::Text(response)` | Excellent |
| Gate verdict | `Kind::GateVerdict`, `Body::Json(verdict_data)` | Excellent |
| Code file | `Kind::PromptSection`, `Body::Text(code)` | Good |
| Binary artifact | `Kind::Custom("artifact")`, `Body::Bytes(data)` | Good |
| Prediction | `Kind::Prediction`, `Body::Json(claim)` | Excellent |
| Metric | `Kind::Metric`, `Body::Json(metric_data)` | Excellent |
| Pheromone | `Kind::Pheromone`, `Body::Json(pheromone)` + `Decay::HalfLife` | Excellent |

---

## Edge Cases That Stress the Type

| Edge Case | Problem | Current Handling | Adequacy |
|---|---|---|---|
| **Large binary blobs** (e.g., model weights) | Signal struct held in memory | `Body::Bytes` exists but no streaming | Adequate for current use; streaming needed at scale |
| **Structured multi-part data** (e.g., PR with title + body + files) | Single Body can't hold structured parts | `Body::Json` with nested structure | Adequate but verbose |
| **Cross-signal relationships** (e.g., gate verdict about an agent output) | Lineage is a Vec of parent hashes | Lineage + tags (`"target_id": hash`) | Adequate |
| **Real-time streaming data** (e.g., live price feed) | Engram is a snapshot, not a stream | Create new Engrams per tick with Decay::TTL | Adequate; TTL handles ephemerality |
| **Confidential data** (e.g., API keys in context) | Provenance.tainted exists but no encryption | Taint flag + scrub policy | Adequate for current threat model |

No edge case requires a new type or a structural change to Engram. All are handled by existing
extension mechanisms (`Kind::Custom`, `Body::Json`, `tags`, `Decay::TTL`, `Provenance.tainted`).

---

## Comparison to Agent Data Protocol (ADP)

The Agent Data Protocol (arXiv:2510.24702) addresses the same problem: universal data
representation for agent systems. ADP unifies all agent data into Trajectory objects composed
of Actions (API, code, message) and Observations (text, web).

| Dimension | ADP | Roko Engram |
|---|---|---|
| **Universal type** | Trajectory | Signal/Engram |
| **Identity** | Sequential index | Content-addressed (BLAKE3 hash) |
| **Quality assessment** | None | 4-axis Score (confidence, novelty, utility, reputation) |
| **Temporal dynamics** | None | Four Decay variants (None, HalfLife, TTL, Ebbinghaus) |
| **Trust tracking** | None | Provenance (author, trust, tainted, session) |
| **Composition** | Concatenation | Composer trait with budget constraints |
| **Complexity reduction** | O(D+A) vs O(D×A) | Same: universal type enables O(D+A) integration |

Roko's Engram is strictly richer than ADP's Trajectory: it adds scoring, decay, provenance,
content-addressing, and lineage tracking. The ADP paper validates the core insight that a
universal type reduces integration complexity from multiplicative to additive.

---

## VSA/HDC Algebraic Extension Opportunity

The existing `bardo-primitives` crate provides 10,240-bit Hyperdimensional Computing vectors.
These could extend the Engram with algebraic operations:

```rust
// Potential extension: Engram algebraic operations
impl Signal {
    /// Bind two Engrams: creates an association (XOR in HDC space)
    pub fn bind(&self, other: &Signal) -> Signal { /* ... */ }

    /// Bundle Engrams: creates a superposition (majority vote in HDC space)
    pub fn bundle(engrams: &[Signal]) -> Signal { /* ... */ }

    /// Permute: creates a sequential ordering (cyclic shift in HDC space)
    pub fn permute(&self, position: usize) -> Signal { /* ... */ }
}
```

This would make Signal a proper Vector Symbolic Architecture element, enabling compositional
knowledge representation directly at the type level. Currently, HDC operations exist in
`bardo-primitives` but are not exposed on the Signal struct.

This enhancement corresponds to gap [G — Code Intelligence × Neuro cross-domain transfer]
identified in the readiness audit. See also [08-novel-proposals.md](08-novel-proposals.md).

---

## Related Findings

- [F2 — Trait Sufficiency](02-finding-trait-sufficiency.md): The six traits operate on Engrams
  as their primary data type.
- [07 — Category Theory](07-finding-category-theory.md): Engrams are the objects of the
  Engram category.
- [Integration Map: neuro×composition](../integration-map/neuro-x-composition.md): The
  missing full knowledge injection depends on Engram Kind richness.

## References

- Phan-Ba, R. et al. (2025). "Agent Data Protocol (ADP)." arXiv:2510.24702
- Kanerva, P. (2009). "Hyperdimensional Computing." Cognitive Computation 1(2).
- Kleyko, D. et al. (2022). "A Survey on Hyperdimensional Computing." AI Review 56.

## Open Questions

- Should HDC bind/bundle/permute be added to `roko-core::Signal` before or after the
  Signal→Engram rename?
- Does the streaming extension for large binary blobs belong in Phase 1 or Phase 2?
