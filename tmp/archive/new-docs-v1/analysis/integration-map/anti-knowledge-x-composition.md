---
title: "AntiKnowledge × Composition"
section: analysis
subsection: integration-map
id: im-anti-knowledge-x-composition
source: 24-cross-section-integration-map.md (§3.3, M15, §6.1)
missing-integration: M15
tier: 2
tags: [anti-knowledge, composition, neuro, disproven-approaches, context-injection]
---

# AntiKnowledge × Composition

**Direction**: 06-Neuro (`Kind::AntiKnowledge` entries) → 03-Composition  
**Status**: **Missing (M15)** — Tier 2, ~35 LOC (subset of M5)  
**Interface**: `NeuroStore::query_relevant()` filtering for `KnowledgeType::AntiKnowledge` → `SystemPromptBuilder`

## What Flows

`Kind::AntiKnowledge` Engrams store falsified approaches — things the system has tried and proven don't work, with evidence. These are the most actionable safety signal because they prevent agents from re-attempting known dead ends.

| Signal | From | To | Status |
|---|---|---|---|
| `Kind::AntiKnowledge` entries | `NeuroStore` | `SystemPromptBuilder` "disproven approaches" section | **Missing** |

## Note on Scope

M15 is a subset of M5 ([neuro-x-composition.md](./neuro-x-composition.md)). It is called out separately because:
1. AntiKnowledge has the highest safety/ROI ratio of all knowledge types — preventing a dead-end costs zero tokens beyond the warning
2. The template format is distinct (explicit prohibition rather than guidance)
3. It should be implemented first within M5 scope due to that ROI

## Wiring Recipe

```rust
// Within the M5 knowledge injection pipeline:
let anti_knowledge = neuro_store.query_relevant(&task_context, QueryOptions {
    include_types: vec![KnowledgeType::AntiKnowledge],
    min_confidence: 0.5,  // Higher threshold for prohibitions
    max_results: 5,
})?;

if !anti_knowledge.is_empty() {
    builder.add_section("disproven_approaches", Section {
        content: render_anti_knowledge(&anti_knowledge),
        priority: Priority::Critical,  // Second highest, after Warnings
        max_tokens: 300,
    });
}

fn render_anti_knowledge(entries: &[KnowledgeEntry]) -> String {
    let items: Vec<String> = entries.iter().map(|e| {
        format!("- {} [falsified: {}]", e.content, e.falsified_date.date())
    }).collect();
    format!("❌ DISPROVEN APPROACHES (do NOT attempt):\n{}", items.join("\n"))
}
```

Estimated LOC: ~35.

## Invariants of the Interaction

1. AntiKnowledge entries are only injected if their `confidence ≥ 0.5` (higher threshold than other knowledge types — prohibitions require more evidence).
2. AntiKnowledge renders as explicit prohibitions, not suggestions.
3. At most 5 AntiKnowledge entries per prompt (most relevant, by HDC similarity to task).
4. Falsified date must be displayed — a 2-year-old falsification may be invalid today.

## Failure Modes

| Failure | Consequence | Detection |
|---|---|---|
| No AntiKnowledge entries yet | Section omitted; graceful | Log knowledge store growth metrics |
| AntiKnowledge entry too generic | Prohibition blocks valid approaches | Require specific context in entry; review low-precision entries |
| Outdated falsification (domain changed) | Valid approach blocked | Decay model on AntiKnowledge; re-verify old entries |

## Open Questions

1. Should agents be able to flag an AntiKnowledge entry as "no longer valid" (a re-validation pathway)?
2. How does AntiKnowledge interact with the agent's tool-call safety layer — could it supplement `BashPolicy`?

## Cross-References

- Full knowledge injection: [neuro-x-composition.md](./neuro-x-composition.md) — M5 (parent integration)
- Safety constraints: [safety-x-composition.md](./safety-x-composition.md) — M13 (another "do not" injection pathway)
- Readiness audit: [RA-06: Neuro](../readiness-audit/subsystem-neuro.md)
