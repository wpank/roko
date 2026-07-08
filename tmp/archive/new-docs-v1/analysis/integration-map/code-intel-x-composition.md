---
title: "Code Intelligence × Composition"
section: analysis
subsection: integration-map
id: im-code-intel-x-composition
source: 24-cross-section-integration-map.md (§6.2 M8, §3.3)
missing-integration: M8
tier: 2
tags: [code-intelligence, composition, roko-index, symbol-graph, context-assembly, pagerank]
---

# Code Intelligence × Composition

**Direction**: 15-Code Intelligence → 03-Composition (code-aware context injection)  
**Status**: **Missing (M8)** — Tier 2, ~120 LOC. `roko-index` is built/tested but has no consumer.  
**Interface**: `roko-index::SymbolGraph` + `roko-index::HdcIndex` → `roko-compose::PromptComposer`

## What Flows

`roko-index` builds symbol graphs (functions, types, call chains) and HDC fingerprints for source code. Currently this structural understanding is not injected into agent prompts — agents receive raw file content rather than semantically ranked code context.

| Signal | From | To | Status |
|---|---|---|---|
| `Kind::Symbol` Engrams | `roko-index` | `SystemPromptBuilder` | **Missing** |
| PageRank-ranked symbol list | `roko-index::HdcIndex` | Composition context assembler | **Missing** |
| Caller/callee chains | `roko-index::SymbolGraph` | Composition context | **Missing** |
| HDC similarity-ranked symbols | `roko-index::HdcIndex` | Context budget allocator | **Missing** |

## Wiring Recipe

```rust
// In the context assembly pipeline (before prompt composition):
let task_files = task.affected_files();
let symbol_context = code_index.context_for_files(&task_files, ContextOptions {
    max_tokens: 4000,
    strategy: SearchStrategy::PageRankBudget,  // Rank by importance, fit to budget
    include_callers: true,
    include_callees: true,
    depth: 2,
})?;

// Inject as a dedicated section in the system prompt:
builder.add_section("code_context", Section {
    content: symbol_context.render(),
    priority: Priority::High,
    max_tokens: 4000,
});
```

**Impact estimate** (from source file 24): Reduces agent context waste by 30-50%. Instead of including entire files, the agent receives only the semantically relevant symbols, ranked by PageRank importance and budget-fitted to the token limit.

**Required prerequisite**: A `CodeIndex` trait and search API must be defined to abstract over `roko-index` (see Readiness Audit G2, G20).

## Invariants of the Interaction

1. Code context injection is task-scoped — only symbols from `task.affected_files()` and their depth-2 callers/callees are included.
2. Code context is budget-bounded — `max_tokens: 4000` is a hard cap.
3. The PageRank strategy ensures the most structurally important symbols appear first within the budget.
4. If `task.affected_files()` is empty or files don't exist in the index, the section is omitted gracefully.
5. Composition reads the code index; it never writes to it.

## Failure Modes

| Failure | Consequence | Detection |
|---|---|---|
| `roko-index` not populated | No code context injected | Log cache-miss rate; trigger background indexing |
| Index stale (file changed since indexing) | Outdated symbols injected | File modification time check; invalidate on change |
| Symbol graph too large (huge crate) | Token budget overrun | PageRank budget-fitting must strictly honor `max_tokens` |
| No `affected_files` metadata on task | Can't scope query | Fall back to task-description HDC similarity query |

## Observed Metrics

Expected after implementation:
- Code context injection rate (% of tasks where code context was available)
- Token savings vs raw-file injection
- Correlation between code context injection and gate pass rate (quality proxy)

## Open Questions

1. Should code context be indexed at the function level or file level? Function level is richer but requires more index maintenance.
2. Should `roko-index` be re-indexed on every task, or only on `git commit`? Background indexing is cheaper but may be stale.
3. How does code context interact with the existing `workspace_map` section in the system prompt? They may overlap; deduplication logic is needed.

## Cross-References

- Verification extension: [code-intel-x-verification.md](./code-intel-x-verification.md) — M16 (code index also enables semantic diff verification)
- Knowledge complement: [neuro-x-composition.md](./neuro-x-composition.md) — M5 (both inject context; recommended ordering: Knowledge > Code > Skills)
- Readiness audit: [RA-15: Code Intelligence](../readiness-audit/subsystem-code-intelligence.md), [RA-03: Composition](../readiness-audit/subsystem-composition.md)
- Audit gap: G2 (wire roko-index into ContextProvider) and G3 (register lang providers) must precede M8
