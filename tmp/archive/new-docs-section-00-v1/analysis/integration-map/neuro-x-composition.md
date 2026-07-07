---
title: "Neuro × Composition"
section: analysis
subsection: integration-map
id: im-neuro-x-composition
source: 24-cross-section-integration-map.md (§6.2 M5, §3.2, §3.3)
missing-integration: M5
tier: 2
tags: [neuro, composition, knowledge-injection, warnings, heuristics, anti-knowledge, causal-links]
---

# Neuro × Composition

**Direction**: 06-Neuro → 03-Composition (knowledge injection); also 03-Composition → 06-Neuro (search queries, wired)  
**Status**: **Partially Wired** — `Kind::Insight` partially injected; `Kind::Warning`, `Kind::CausalLink`, `Kind::AntiKnowledge`, `Kind::StrategyFragment` all **Missing** (M5 gap)  
**Interface**: `roko-neuro::NeuroStore` → `roko-compose::SystemPromptBuilder`

## What Flows

| Signal | From | To | Status |
|---|---|---|---|
| `Kind::Insight` | `NeuroStore` | SystemPromptBuilder | **Partial** |
| `Kind::Heuristic` | `NeuroStore` | SystemPromptBuilder | **Missing** |
| `Kind::Warning` | `NeuroStore` | SystemPromptBuilder | **Missing** |
| `Kind::CausalLink` | `NeuroStore` | SystemPromptBuilder | **Missing** |
| `Kind::StrategyFragment` | `NeuroStore` | SystemPromptBuilder | **Missing** |
| `Kind::AntiKnowledge` | `NeuroStore` | SystemPromptBuilder | **Missing** (also M15) |
| HDC similarity query | SystemPromptBuilder | `NeuroStore` | **Wired** (partial) |

## The M5 Gap: Full Knowledge Injection

**Problem**: NeuroStore has six knowledge types but only Insights are partially injected. Warnings, CausalLinks, and AntiKnowledge are stored but never surface in agent context, meaning agents repeat known mistakes and ignore learned causal patterns.

### Wiring Recipe

```rust
// In orchestrate.rs, before building system prompt:
let task_context = TaskContext::from_task(&task);
let knowledge = neuro_store.query_relevant(&task_context, QueryOptions {
    max_results: 20,
    min_confidence: 0.3,
    include_types: vec![
        KnowledgeType::Insight,
        KnowledgeType::Heuristic,
        KnowledgeType::Warning,
        KnowledgeType::CausalLink,
        KnowledgeType::StrategyFragment,
        KnowledgeType::AntiKnowledge,
    ],
})?;

// Group by type and inject with different formatting:
builder.add_knowledge_section(KnowledgeSection {
    warnings: knowledge.iter().filter(|k| k.kind == Warning).collect(),
    heuristics: knowledge.iter().filter(|k| k.kind == Heuristic).collect(),
    anti_knowledge: knowledge.iter().filter(|k| k.kind == AntiKnowledge).collect(),
    causal_links: knowledge.iter().filter(|k| k.kind == CausalLink).collect(),
    // Warnings get highest priority (safety), AntiKnowledge second (avoid dead ends)
});
```

**Template format for Warnings:**
```
⚠️ KNOWN PITFALLS (from {n} validated observations):
- {warning.content} [confidence: {warning.confidence:.0%}]
```

**Template format for AntiKnowledge:**
```
❌ DISPROVEN APPROACHES (do NOT attempt):
- {anti.content} [falsified: {anti.falsified_date}]
```

Estimated LOC: ~90 for core injection; ~35 additional for AntiKnowledge formatting (M15).

## Invariants of the Interaction

1. Knowledge injection is bounded by token budget — the most urgent types (Warning > AntiKnowledge > Heuristic) get priority.
2. Only knowledge entries with `confidence ≥ min_confidence` are injected.
3. Knowledge entries with `Decay::Ebbinghaus` are excluded if their effective weight falls below threshold.
4. The knowledge section must not exceed the composition budget's reserved knowledge token allocation.
5. Composition reads NeuroStore; it never writes to it.

## Failure Modes

| Failure | Consequence | Detection |
|---|---|---|
| NeuroStore empty (early runs) | No knowledge injection; graceful degradation | Knowledge section omitted |
| HDC similarity query timeout | Stale or no knowledge injected | Timeout fallback; log miss |
| Knowledge entries expired | Outdated advice injected | Check decay model before injection |
| Too many Warning entries (token overrun) | Budget exceeded | Sort by confidence, take top N within budget |

## Observed Metrics

Expected after implementation:
- Knowledge injection rate per task type
- Warning injection frequency (proxy for known-pitfall density)
- AntiKnowledge injection rate (proxy for repeated-mistake prevention)
- Success rate comparison: tasks with vs without knowledge injection

## Open Questions

1. Should the ordering of knowledge types in the prompt be fixed (Warnings first) or dynamic (ranked by confidence × relevance)?
2. How does this interact with the 20-token role prompt gap (audit G4)? Knowledge injection may be partially wasted if the role prompt is too thin.
3. Should CausalLinks be rendered as natural language or as explicit IF→THEN rules?

## Cross-References

- AntiKnowledge subset: [anti-knowledge-x-composition.md](./anti-knowledge-x-composition.md) — M15
- Daimon modulation: [daimon-x-composition.md](./daimon-x-composition.md) — M2 (both inject; Daimon adjusts weights of knowledge sections)
- Skills complement: [learning-x-composition.md](./learning-x-composition.md) — M4 (both inject; recommended ordering: Knowledge > Skills > Code)
- Gate thresholds: [neuro-x-verification.md](./neuro-x-verification.md) — M14 (knowledge also informs gate thresholds)
- Readiness audit: [RA-06: Neuro](../readiness-audit/subsystem-neuro.md), [RA-03: Composition](../readiness-audit/subsystem-composition.md)
