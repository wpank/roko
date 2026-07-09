---
title: "Code Intelligence × Verification"
section: analysis
subsection: integration-map
id: im-code-intel-x-verification
source: 24-cross-section-integration-map.md (§6.1 M16)
missing-integration: M16
tier: 3
tags: [code-intelligence, verification, semantic-diff, symbol-graph, gate]
---

# Code Intelligence × Verification

**Direction**: 15-Code Intelligence → 04-Verification (semantic diff as gate input)  
**Status**: **Missing (M16)** — Tier 3, ~100 LOC. Depends on roko-index consumer API (Readiness Audit G2, G20).  
**Interface**: `roko-index::SymbolGraph` diff → `roko-gate` semantic verification gate

## What Flows

Code intelligence can compute semantic diffs — not just line-level changes but changes in the call graph, public API, and symbol relationships. This richer diff provides better input to gates that verify code correctness.

| Signal | From | To | Status |
|---|---|---|---|
| Semantic diff (symbol-level changes) | `roko-index::SymbolGraph` | `Gate::verify()` input context | **Missing** (M16) |
| API surface change detection | `roko-index` | `LlmJudgeGate` prompt enrichment | **Missing** |
| Call graph regression detection | `roko-index` | `PropertyTestGate` selector | **Missing** |

## Wiring Recipe

```rust
// In gate pipeline, after agent output is produced:
let semantic_diff = code_index.semantic_diff(
    &original_files,
    &modified_files,
    DiffOptions { include_callers: true, include_api_surface: true }
)?;

// Inject semantic diff into gate context
let enriched_context = ctx.with_extension(
    "semantic_diff", &semantic_diff
);

// LlmJudgeGate uses semantic_diff to evaluate structural correctness
let verdict = gate.verify(&composed_output, &enriched_context).await?;
```

Estimated LOC: ~100. Requires CodeIndex trait and search API (G2, G20) as prerequisites.

## Invariants of the Interaction

1. Semantic diff is optional — gates operate correctly without it (graceful degradation).
2. Semantic diff does not increase gate latency beyond the configured gate timeout.
3. Code index state is captured at task start, not re-indexed mid-task (snapshot semantics).

## Failure Modes

| Failure | Consequence | Detection |
|---|---|---|
| roko-index not populated | No semantic diff; gates use text diff only | Log code-intel miss rate |
| Diff computation timeout | Gate uses text-only context | Timeout with fallback to standard gate context |
| Symbol graph inaccurate | Incorrect API-surface detection | Index validation tests |

## Open Questions

1. Should semantic diff detection trigger specific gate types (e.g., API surface changes always trigger LlmJudgeGate)?
2. How does M16 interact with M8 (Code Intel → Composition)? Both require roko-index; should they share an index client?

## Cross-References

- Composition complement: [code-intel-x-composition.md](./code-intel-x-composition.md) — M8 (roko-index also enriches composition; M8 should be implemented first)
- Knowledge thresholds: [neuro-x-verification.md](./neuro-x-verification.md) — M14 (both enrich verification; complementary)
- Readiness audit: [RA-15: Code Intelligence](../readiness-audit/subsystem-code-intelligence.md), [RA-04: Verification](../readiness-audit/subsystem-verification.md)
