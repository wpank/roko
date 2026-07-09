---
title: "Neuro × Verification"
section: analysis
subsection: integration-map
id: im-neuro-x-verification
source: 24-cross-section-integration-map.md (§6.1 M14, §3.2, §3.3)
missing-integration: M14
tier: 3
tags: [neuro, verification, gate-thresholds, adaptive-thresholds, knowledge-informed-verification]
---

# Neuro × Verification

**Direction**: 06-Neuro → 04-Verification (knowledge-informed gate thresholds)  
**Status**: **Missing (M14)** — Tier 3, ~60 LOC. Gate thresholds adapt via EWMA/CUSUM from their own performance history but do not use NeuroStore knowledge.  
**Interface**: `NeuroStore` domain-specific knowledge → `roko-gate::AdaptiveThresholds`

## What Flows

NeuroStore accumulates knowledge about which gate configurations succeed in which contexts. This domain-specific pattern knowledge should inform gate threshold adaptation — context-aware thresholds rather than purely statistical adaptation.

| Signal | From | To | Status |
|---|---|---|---|
| Domain-specific gate performance patterns | `NeuroStore` (Insights, CausalLinks) | `AdaptiveThresholds` | **Missing** (M14) |
| Current context similarity | Task context HDC fingerprint | Gate threshold lookup | **Missing** |
| `Kind::Warning` about gate bypasses | `NeuroStore` | Gate pipeline configuration | **Missing** |

## Wiring Recipe

```rust
// In gate pipeline setup (before verification run):
let task_fingerprint = hdc_index.fingerprint(&task.description);
let relevant_patterns = neuro_store.query_relevant_for_verification(
    &task_fingerprint,
    QueryOptions {
        include_types: vec![KnowledgeType::Insight, KnowledgeType::CausalLink],
        min_confidence: 0.4,
        max_results: 10,
    }
)?;

// Adjust gate thresholds based on known patterns for this context
if let Some(patterns) = relevant_patterns {
    adaptive_thresholds.apply_knowledge_bias(&patterns);
}
```

**Key insight**: The existing `AdaptiveThresholds` system (EWMA/CUSUM) adapts based on statistical performance. Neuro provides a semantic layer: "in contexts similar to this task, the LLM judge gate tends to over-reject, so lower its threshold."

Estimated LOC: ~60.

## Invariants of the Interaction

1. Knowledge-based threshold adjustments are bounded: at most ±20% relative to the statistical baseline.
2. NeuroStore knowledge is read-only from the gate pipeline's perspective.
3. Threshold adjustments are logged as `Kind::Metric` Engrams for auditability.
4. If NeuroStore has no relevant patterns, the gate uses its standard adaptive thresholds unchanged.

## Failure Modes

| Failure | Consequence | Detection |
|---|---|---|
| NeuroStore lookup fails | Gate uses standard thresholds (graceful fallback) | Log lookup failure rate |
| Knowledge biases gate toward too-permissive | Quality drops | Gate pass rate monitoring; alert on >5% week-over-week drop |
| Conflicting patterns (multiple contradictory insights) | Ambiguous threshold bias | Weighted average by confidence × recency |

## Observed Metrics

Expected after implementation:
- Threshold adjustment frequency (how often knowledge influences gate settings)
- Quality delta for tasks where knowledge-informed thresholds applied vs baseline
- Pattern match rate (% of tasks where relevant patterns were found)

## Open Questions

1. Should M14 be part of the gradient gate feedback proposal (AA-08 Proposal 2)? The two complement each other.
2. Should CausalLinks (e.g., "long latency → higher rejection rate") trigger threshold adjustments or just inform post-hoc analysis?

## Cross-References

- Input to gate: [neuro-x-composition.md](./neuro-x-composition.md) — M5 (knowledge enriches composition; M14 enriches verification)
- Gradient feedback: [../architectural-analysis/08-novel-proposals.md](../architectural-analysis/08-novel-proposals.md) — Proposal 2 (continuous gate feedback creates the knowledge M14 consumes)
- Code intel verification: [code-intel-x-verification.md](./code-intel-x-verification.md) — M16 (another form of semantic verification enrichment)
- Readiness audit: [RA-06: Neuro](../readiness-audit/subsystem-neuro.md), [RA-04: Verification](../readiness-audit/subsystem-verification.md)
