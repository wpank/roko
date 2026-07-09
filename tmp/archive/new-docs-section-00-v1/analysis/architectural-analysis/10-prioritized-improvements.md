---
title: "Prioritized Improvements"
section: analysis
subsection: architectural-analysis
id: aa-10
source: 23-architectural-analysis-improvements.md (§10)
tags: [improvements, prioritization, high-priority, medium-priority, low-priority, roadmap]
---

# Prioritized Improvements

> **Summary**: 11 improvements in three tiers derived from the full architectural analysis. High-priority items address architectural integrity; medium-priority items enhance the architecture; low-priority items are architectural innovations.

## Source

From source file 23 (§10), the full analysis distilled to 11 prioritized improvements. Each item has: finding origin, estimated effort, and cross-references to the relevant pair files.

---

## Tier 1: High Priority — Architectural Integrity

These four improvements are quick wins that directly repair known violations and documentation errors. They should be executed before any new architectural work.

### I1: Fix roko-conductor → roko-learn Dependency Violation

**Finding origin**: [AA-03: Layer Taxonomy](./03-finding-layer-taxonomy.md)

**Problem**: `roko-conductor` (L3/L4) imports `roko-learn` (L2/Cross-cut), creating an upward dependency. This violates the five-layer taxonomy rule that higher layers must not be imported by lower/equal layers for concerns that belong lower.

**Fix**:
```rust
// In roko-core/src/traits.rs — extract the interface down to L0
pub trait HealthMetrics: Send + Sync {
    fn failure_rate(&self, gate: &str, window: Duration) -> f32;
    fn avg_latency(&self, gate: &str, window: Duration) -> Duration;
}
```

`roko-conductor` then depends on `&dyn HealthMetrics` (L0 trait). `roko-learn` implements it. Dependency flows downward.

**Effort**: Small (1-2 days)  
**Impact**: Restores layer integrity; enables independent compilation of `roko-conductor`

**Cross-references**:
- [conductor-x-orchestration](../integration-map/conductor-x-orchestration.md)
- [learning-x-verification](../integration-map/learning-x-verification.md)
- Missing integration M9 in integration-map

---

### I2: Classify 6 Unclassified Crates in Taxonomy

**Finding origin**: [AA-03: Layer Taxonomy](./03-finding-layer-taxonomy.md)

**Problem**: Six crates lack formal layer assignment, making the architectural map incomplete.

**Recommended assignments**:

| Crate | Assignment | Rationale |
|---|---|---|
| `roko-neuro` | Cross-cut | Bridges L0-L2 for knowledge; inject via `&dyn Substrate` |
| `roko-daimon` | Cross-cut | No upward deps (only roko-core); inject via PAD trait object |
| `roko-dreams` | Cross-cut | Bridges Neuro + Daimon at Delta frequency |
| `roko-golem` | Phase 2+ umbrella | Contains Daimon and Dreams code pending dissolution |
| `roko-chain` | L1 Domain Plugin | Analogous to roko-agent for chain domain |
| `roko-plugin` | L1 Framework | Plugin SDK extending the tool/agent system |

**Effort**: Small (documentation update, 1 day)  
**Impact**: Completes the architectural map; enables reasoning about which crates are safe to import where

---

### I3: Fix roko-fs Layer Assignment (L3 → L0) in Docs

**Finding origin**: [AA-09: Inconsistencies](./09-finding-inconsistencies.md), item DI-2

**Problem**: `roko-fs` is listed under L3 Harness in `12-five-layer-taxonomy.md` but is functionally an L0 Runtime crate (implements `FileSubstrate`).

**Fix**: Update `12-five-layer-taxonomy.md` to move `roko-fs` to L0 Runtime. One-line change.

**Effort**: Trivial (minutes)  
**Impact**: Corrects a key documentation error in the canonical architectural reference

---

### I4: Align Score Documentation (7-axis → 4-axis current, 7-axis planned)

**Finding origin**: [AA-09: Inconsistencies](./09-finding-inconsistencies.md), items DI-5 and CM-2

**Problem**: `02-engram-data-type.md` references a "7-axis appraisal" but the `Score` struct in `roko-core` has only 4 axes.

**Fix**: Update documentation to distinguish:
- **Stable (current)**: confidence, novelty, utility, reputation
- **Planned (Phase 2+)**: specificity, urgency, empathy

**Effort**: Small (documentation update)  
**Impact**: Prevents confusion on a core type; the Score struct is used in every trait interaction

---

## Tier 2: Medium Priority — Architectural Enhancement

These four improvements add significant capability while preserving existing interfaces.

### I5: Implement Gradient Gate Feedback (Active Inference)

**Finding origin**: [AA-08: Novel Proposals](./08-novel-proposals.md), Proposal 2

**Problem**: Gate returns binary pass/fail; learning uses only the boolean. The continuous confidence score is discarded.

**Fix**: Use `verdict.score ∈ [0,1]` as continuous reward signal to `router.feedback()`. When `verdict.score < 0.3`, generate a knowledge `Insight` and persist to `NeuroStore`.

**Theoretical basis**: Active inference / Free Energy Principle (Friston 2010). Prediction error should be proportional, not binary.

**Effort**: Medium (1 week)  
**Impact**: **Highest-leverage near-term improvement** per the full analysis conclusion. Continuous learning from every verification attempt rather than binary pass/fail signals.

**Cross-references**:
- [learning-x-verification](../integration-map/learning-x-verification.md)
- [neuro-x-verification](../integration-map/neuro-x-verification.md)
- Missing integration M14

---

### I6: Define AffectModel Trait in roko-core for Daimon Injection

**Finding origin**: [AA-06: Cross-Cut Isolation](./06-finding-crosscut-isolation.md), gap #1

**Problem**: Daimon is not injected via trait object. L0/L1 code must import `roko-daimon` types directly.

**Fix**:
```rust
// In roko-core/src/traits.rs
pub trait AffectModel: Send + Sync {
    fn pad(&self) -> PadVector;
    fn behavioral_state(&self) -> BehavioralState;
}
```

`roko-daimon` implements `AffectModel`. All consumers (orchestrator, composer, router) receive `&dyn AffectModel`.

**Effort**: Small (2-3 days)  
**Impact**: Proper cross-cut isolation; enables mocking in tests; enables alternative affect implementations

**Cross-references**:
- [daimon-x-orchestration](../integration-map/daimon-x-orchestration.md)
- [daimon-x-composition](../integration-map/daimon-x-composition.md)

---

### I7: Formalize Pipeline as Composable Unit (Hierarchical Speeds)

**Finding origin**: [AA-08: Novel Proposals](./08-novel-proposals.md), Proposal 3

**Problem**: The relationship between cognitive speeds (how Theta folds Gamma outcomes) is implicit in scheduling logic. There is no formal `fold_outcomes` operation.

**Fix**: Introduce `Pipeline` struct and `fold_outcomes() → Query` as a monoid homomorphism connecting speed levels.

**Effort**: Medium (1 week)  
**Impact**: Makes the three-speed architecture formally composable; enables testing speed interactions in isolation

**Cross-references**:
- [AA-04: Cognitive Speeds](./04-finding-cognitive-speeds.md)
- [AA-07: Category Theory](./07-finding-category-theory.md)
- [orchestration-x-learning](../integration-map/orchestration-x-learning.md)

---

### I8: Implement Cross-Cut Arbitration Protocol

**Finding origin**: [AA-06: Cross-Cut Isolation](./06-finding-crosscut-isolation.md), gap #3

**Problem**: Cross-cut conflicts (Neuro vs Daimon vs Dreams all wanting to modify the same pipeline step) are resolved ad hoc. The VCG arbitration described in `13-cognitive-cross-cuts.md` Section 6 is not implemented.

**Fix**: Implement the priority hierarchy (Daimon > Neuro > Dreams) with VCG tiebreaker. This establishes the adjunction unit/counit described in [AA-07](./07-finding-category-theory.md) §7.5.

**Effort**: Medium (1-2 weeks)  
**Impact**: Resolves Daimon↔Neuro↔Dreams conflicts deterministically; enables the categorical commutative diagram to hold formally

**Cross-references**:
- [AA-07: Category Theory](./07-finding-category-theory.md) §7.5
- [dreams-x-neuro](../integration-map/dreams-x-neuro.md)
- [dreams-x-daimon](../integration-map/dreams-x-daimon.md)
- [neuro-x-learning](../integration-map/neuro-x-learning.md)

---

## Tier 3: Low Priority — Architectural Innovation

These three improvements are significant research and engineering investments. They extend the architecture beyond its current theoretical grounding into novel territory.

### I9: CompetitiveRouter (LIDA-Inspired)

**Finding origin**: [AA-08: Novel Proposals](./08-novel-proposals.md), Proposal 1

**Problem**: Single-scorer routing is brittle against scorer blind spots.

**Fix**: `CompetitiveRouter` implementing the `Router` trait — multiple scorers form coalitions, strongest coalition wins.

**Effort**: Large (2-3 weeks plus evaluation)  
**Impact**: More robust attention/selection; implements Global Workspace Theory's consciousness spotlight

---

### I10: VSA/HDC Operations on Signal Struct

**Finding origin**: [AA-05: Engram Universality](./05-finding-engram-universality.md), §5.4

**Problem**: `bardo-primitives` provides 10,240-bit HDC vectors but they are not exposed on the `Signal` struct as algebraic operations.

**Fix**: Add `bind()`, `bundle()`, `permute()` methods to `Signal` that delegate to `bardo-primitives`, making `Signal` a proper Vector Symbolic Architecture element.

**Effort**: Large (2-3 weeks plus correctness verification)  
**Impact**: Compositional knowledge representation at the type level; enables HDC-based semantic queries on Engrams directly

**Cross-references**:
- [AA-05: Engram Universality](./05-finding-engram-universality.md)
- Synergy S1 in [synergy-map](../synergy-map/) — HDC is a key node in the synergy graph

---

### I11: Formal Category Theory Verification of Pipeline Laws

**Finding origin**: [AA-07: Category Theory](./07-finding-category-theory.md)

**Problem**: The categorical claims (Score is a monoid, pipeline is Kleisli composition, cross-cuts are endofunctors) are asserted but not formally verified.

**Fix**: Implement property-based tests that verify the monoid laws, Kleisli laws, and functor laws for all trait implementations in `roko-core`. Use `proptest` or `quickcheck`.

**Effort**: Large (3-4 weeks)  
**Impact**: Mathematical guarantees of composability; catches implementations that break categorical structure before they ship

**Cross-references**:
- [AA-07: Category Theory](./07-finding-category-theory.md) §7.6 (design rules)

---

## Master Table

| # | Improvement | Finding | Tier | Effort | Impact |
|---|---|---|---|---|---|
| I1 | Fix conductor→learn violation | AA-03 | High | Small | Layer integrity |
| I2 | Classify 6 unclassified crates | AA-03 | High | Small | Complete arch map |
| I3 | Fix roko-fs label (L3→L0) | AA-09 | High | Trivial | Correct docs |
| I4 | Align Score docs (7→4 axis) | AA-09 | High | Small | Prevent confusion |
| I5 | Gradient Gate feedback | AA-08 P2 | Medium | Medium | **Highest-leverage** |
| I6 | AffectModel trait for Daimon | AA-06 | Medium | Small | Cross-cut isolation |
| I7 | Hierarchical Pipeline struct | AA-08 P3 | Medium | Medium | Formal speed composition |
| I8 | Cross-cut arbitration protocol | AA-06 | Medium | Medium | Deterministic conflict resolution |
| I9 | CompetitiveRouter | AA-08 P1 | Low | Large | Robust attention |
| I10 | HDC ops on Signal | AA-05 | Low | Large | Compositional knowledge |
| I11 | Formal CT verification | AA-07 | Low | Large | Mathematical guarantees |

---

## Academic References

- Friston, K. (2010). "The free-energy principle: a unified brain theory?" Nature Reviews Neuroscience 11(2).
- Franklin, S. et al. (2016). "LIDA: A Systems-level Architecture." IEEE Trans. AMD 6(1).
- Beer, S. (1972). "Brain of the Firm." Allen Lane. (VSM framework for layer assignments)
- Kleyko, D. et al. (2022). "A Survey on Hyperdimensional Computing." Artificial Intelligence Review 56.
- Milewski, B. (2014). "Category Theory for Programmers." bartoszmilewski.com.
