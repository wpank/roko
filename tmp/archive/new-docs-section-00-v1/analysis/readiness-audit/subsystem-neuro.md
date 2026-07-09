---
title: "Readiness Audit: Neuro (§06)"
section: analysis
subsection: readiness-audit
id: ra-subsystem-06
source: 31-implementation-readiness-audit.md (§06)
score: 22/30
tags: [neuro, knowledge-store, HDC, bardo-primitives, weakest-section]
---

# Readiness Audit: Neuro (§06)

**Score**: 22/30 (weakest scored section) | **Crate**: bardo-primitives (Stable, HDC only), knowledge types spread across roko-core and roko-golem without consolidation

## Criterion Scores

| Criterion | Score | Notes |
|---|---|---|
| rust_structs | 4 | HDC implementation rigorously specified |
| pseudocode | 4 | Ebbinghaus decay with 4 worked examples |
| config_params | 5 | Best config spec in codebase |
| error_handling | 3 | Implicit failure modes |
| integration_wiring | 3 | bardo-primitives wired; NeuroStore not consolidated |
| test_criteria | 3 | HDC benchmarks exceptional; higher-order retrieval untested |

## Strengths

- HDC implementation: XOR bind ~5ns, Hamming similarity ~13ns — rigorously benchmarked
- False positive threshold derivation: Z-score 5.26, Bonferroni for 100K vocabulary, threshold 0.526
- Ebbinghaus decay with tier multiplier: 4 worked examples
- `HdcVector` (3 files, ~500 LOC, 18 tests) in bardo-primitives

## Critical Gaps

- **G10**: Tier field missing from `KnowledgeEntry` (Low effort)
- **G11**: Half-life constants default to 30d; spec values are CausalLink=60d, StrategyFragment=14d (Low effort)
- Cross-domain HDC transfer entirely unimplemented
- Knowledge store types scattered across roko-core and roko-golem without consolidation

## Integration Gaps

- M5: Neuro→Composition (partial)
- M7: Dreams→Neuro (missing — no consolidation pipeline)
- M14: Neuro→Gate Thresholds (missing)
- M15: AntiKnowledge→Composition (missing)

## Cross-References

- [../integration-map/neuro-x-composition.md](../integration-map/neuro-x-composition.md) — M5
- [../integration-map/dreams-x-neuro.md](../integration-map/dreams-x-neuro.md) — M7
- [../integration-map/neuro-x-learning.md](../integration-map/neuro-x-learning.md) — Partial
- [../architectural-analysis/05-finding-engram-universality.md](../architectural-analysis/05-finding-engram-universality.md) — HDC extension proposal
