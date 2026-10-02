# Token Budget Management

> **Depth file for [06-COMPOSITION.md](../../06-COMPOSITION.md)**
> Source: `crates/roko-compose/src/budget.rs`, `crates/roko-compose/src/templates/common.rs`
> v1 source: `docs/v1/03-composition/05-token-budget-management.md`

---

## Overview

Token budget management determines how much of the LLM's context window is
allocated to each prompt section. Roko implements a three-tier budget system:
static per-role budgets (`budget_for`), complexity-adaptive budgets
(`adaptive_budget_for`), and dynamic context-tier budgets
(Surgical/Focused/Full). The system ensures the most valuable context sections
receive the most tokens while low-value sections are dropped or truncated
before they consume budget that higher-value sections need.

---

## 1. Three-Tier Budget Architecture

### Tier 1: Static Per-Role Budgets

The foundation. Each role receives a fixed allocation across 8 section
categories via `budget_for(role)`. These represent the baseline assumption
about what each role needs.

### Tier 2: Complexity-Adaptive Budgets

Overlaid on static budgets. `adaptive_budget_for(role, complexity)` scales
allocations up or down based on task complexity:

| Complexity | Effect on Budget |
|-----------|-----------------|
| **Trivial** | Drop context and skills. Halve workspace_map and brief. |
| **Standard** | No change. Base budget applies. |
| **Complex** | +50% workspace_map, +100% context, +50% file_context. ~40% increase. |

### Tier 3: Context-Tier Budgets

The outermost constraint. The context tier sets the absolute maximum:

| Context Tier | Max Tokens | Model Class |
|-------------|-----------|-------------|
| **Surgical** | 4,000 | Haiku, Ollama, local models |
| **Focused** | 12,000 | Sonnet |
| **Full** | 24,000 | Opus |

The tightest constraint wins.

---

## 2. Token Counting

The `TokenCounter` struct provides configurable token estimation:

```rust
pub fn estimate_tokens(text: &str) -> usize {
    text.len() / 4  // ~4 bytes per token heuristic
}
```

Heuristic characteristics:
- English prose: ~4.5 bytes/token (slightly overestimates)
- Source code: ~3.5 bytes/token (slightly underestimates)
- Overall accuracy: +/-15%
- Performance: <1 microsecond vs ~2ms for exact tokenization

The `TokenCounter` also supports per-model estimation ratios when higher
accuracy is needed for specific model families.

---

## 3. The Differential Budget Principle

Different content types have different information density and compression
tolerance (inspired by LLMLingua's Budget Controller):

| Content Type | Compress? | Priority | Rationale |
|-------------|-----------|----------|-----------|
| Task description | Never | Highest | Agent must know what to do |
| Role identity | Never | Highest | Agent must know what it is |
| Safety constraints | Never | Highest | Agent must know what not to do |
| Gate errors | 5% | High | Recent failures guide corrections |
| File context | 10-20% | High | Source code needs fidelity |
| Task brief | 10% | High | What/Why/How summary |
| Workspace map | 30-50% | Medium | Project structure overview |
| Cross-plan context | 50%+ | Low | Often irrelevant |
| Learning pack | 50%+ | Low | High noise ratio (49% tokens, 61% pass rate) |

---

## 4. Budget Allocation Algorithm

### Phase 1: Section Collection

All available sections are gathered with content and metadata.

### Phase 2: Priority-Ordered Allocation

```
1. Sort sections by priority (Critical first)
2. For each section in priority order:
   a. Look up its allocation in the PromptBudget
   b. If remaining budget < section's min_tokens, skip
   c. Allocate min(actual_tokens, max_tokens, remaining_budget)
   d. If actual > max, truncate section
   e. Deduct allocated tokens
3. Return allocated sections with final content
```

### The Min-Tokens Guard

Each section has a `min_tokens` threshold. If remaining budget cannot
accommodate at least `min_tokens`, the section is skipped entirely rather than
being included in a uselessly truncated form. A workspace map truncated to 100
tokens is worse than none -- it provides structure without substance.

---

## 5. Prompt Prefix Stability for Caching

```
+-----------------------------------------------------+
| System Prompt (role-specific, identical per role)    | <- ALWAYS cached
| Token cost: ~800                                     |
+-----------------------------------------------------+
| Workspace Map (changes only when files change)       | <- Cached within wave
| Token cost: ~334                                     |
+-----------------------------------------------------+
| Learning Pack (changes only on playbook refresh)     | <- Cached within batch
| Token cost: ~2,000 (after cap)                       |
+-----------------------------------------------------+
| Task Description (unique per task)                   | <- CACHE MISS boundary
+-----------------------------------------------------+
| Iteration Context (unique per attempt)               | <- Always miss
+-----------------------------------------------------+
```

Rules for budget-aware prefix stability:
1. Never randomize section ordering
2. Freeze workspace map within a plan execution
3. Cap learning pack within a batch
4. Normalize whitespace via `normalize_for_caching()`
5. Sort tool definitions alphabetically via `canonical_tool_order()`

---

## 6. Budget Conflict Resolution

### Priority Ordering

| Priority | Category | Drop policy |
|---|---|---|
| 0 (Critical) | Identity and safety | Never drop, never truncate |
| 1 (High) | Recent failures | Truncate to last N errors |
| 2 (High) | Source code | Truncate from bottom (keep imports + signatures) |
| 3 (Medium) | Task context | Truncate from bottom (keep requirements) |
| 4 (Medium) | Structure | Truncate deep nodes (keep top-level tree) |
| 5 (Low) | History | Drop entirely before truncating higher sections |

### Truncation Strategies

```
Gate errors:    Keep N most recent (LIFO). Latest is most relevant.
File context:   Keep imports, struct/enum defs, function signatures.
                Drop function bodies (largest consumer).
Workspace map:  Keep top 2 levels. Drop deeper levels.
Task brief:     Keep What/How. Drop Why/Context.
Learning pack:  Drop entire section if budget < min_tokens (2,000).
                Partially truncated learning content is actively harmful.
```

---

## 7. History Compaction

When conversation history exceeds the budget:

```
Non-system messages split into (older, recent) at split point.
Split point = total_messages - (recent_verbatim_turns x 2).
If older messages exceed summary budget:
    Summarize older messages using Haiku -> <conversation_summary>
    Prepend summary, then append recent messages verbatim.
```

Two strategies:
1. **In-place compaction:** Haiku summarizes older messages. Quality degrades
   after 2-3 compactions.
2. **Handoff:** Sonnet produces structured briefing from full thread, new
   session starts with that briefing.

---

## 8. The "Context Anxiety" Mitigation

Claude proactively summarizes when it perceives it is near context limits, even
when it is not. The agent's own compaction interferes with managed compaction.

Mitigation: always request the maximum context window from the provider (1M
tokens) regardless of actual usage. This keeps context management entirely in
the scaffold's control.

---

## 9. Impact Numbers

| Metric | Without management | With management |
|--------|-------------------|-----------------|
| Input tokens per task | ~12K average | ~2.4K average |
| Inference cost per task | ~$2.50 | ~$0.42 |
| Gate pass rate (1st attempt) | 71% | 94% |
| Average iterations per plan | 3.4 | 1.8 |
| 20-plan run cost | ~$200 | ~$34 |

The 83% cost reduction comes from every layer stacking: extraction eliminates
LLM calls, compression reduces token count, caching reduces per-token cost,
better context reduces iteration count.

---

## 10. Academic Foundations

**LLMLingua Budget Controller** [Jiang et al., EMNLP 2023]. Differential
budget principle: different content types have different compression tolerance.
Up to 20x compression with minimal quality loss.

**Selective Context** [Li et al., EMNLP 2023]. Information-theoretic pruning.
50% reduction, 0.023 BERTscore drop.

**Sufficient Context** [Joren et al., ICLR 2025]. Gemma went from 10.2%
incorrect with no context to 66.1% incorrect with insufficient context. Bad
context makes models 6x worse. Motivates the min_tokens guard.

**Context Rot** [Chroma 2025]. Performance degrades as context grows, even
within capacity. Semantically close distractors are far more harmful than
obviously irrelevant content. Motivates aggressive pruning.

**CLEAR** [Mehta 2025, "Beyond Accuracy: A Multi-Dimensional Framework for
Evaluating Enterprise Agentic AI Systems", arXiv:2511.14136]. Accuracy-only
optimization yields agents 4.4-10.8x more expensive than cost-aware alternatives
with comparable performance.

---

## 11. Implementation Status

| Aspect | Status |
|--------|--------|
| PromptBudget per role | **Shipped** |
| Complexity-adaptive budgets | **Shipped** |
| Context tier (Surgical/Focused/Full) | **Shipped** |
| Min-tokens guard | **Shipped** |
| Cache-aware allocation ordering | **Shipped** |
| History compaction | **Shipped** |
| TokenCounter integration | **Shipped** |
| Section effectiveness learning | **Shipped** |
| Budget prediction (learned) | **Designed** |
| Leave-one-out section influence | **Designed** |
| Information-theoretic density scoring | **Designed** |

---

## Cross-References

- [composer-trait.md](composer-trait.md) -- Budget struct in Compose trait
- [prompt-composer.md](prompt-composer.md) -- Budget enforcement in assembly
- [role-templates-11.md](role-templates-11.md) -- Per-role allocation table
- [lost-in-the-middle-u-shape.md](lost-in-the-middle-u-shape.md) -- Attention-aware placement
- `crates/roko-compose/src/budget.rs` -- Complexity-adaptive budgets
- `crates/roko-compose/src/templates/common.rs` -- budget_for() table
