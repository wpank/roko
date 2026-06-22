# ACPM_03: Build ContextManager with Priority Queue and Eviction

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#acpm-03`](../ISSUE-TRACKER.md#acpm-03)
- Source: `tmp/solutions/roko/tasks/09-ACP-MCP.md` — Task 9.3
- Priority: **P0**
- Effort: 6 hours
- Depends on: `ACPM_02` (source 9.2)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: ACPM_03 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

The existing `ContextAssembler` in `crates/roko-neuro/src/context.rs:442` is tightly coupled to the orchestrate.rs path and operates on `KnowledgeStore` + `EpisodeStore` directly. It uses attention scoring, diminishing returns, novelty penalties, and contrarian retrieval. It has the right ideas but is not suitable for direct use in the ACP path because: (a) it depends on `roko-neuro` internals not exported for ACP, (b) it uses a 4K default budget when ACP needs dynamic budgets, (c) it lacks MCP tool results and file context as source types.

The ACP path needs a simpler, trait-based context manager that can accept context items from any source (knowledge, playbooks, files, history, MCP tools) and render them within a token budget.

## Exact Changes

1. Define the `ContextSource` trait in `context_manager.rs`:
   ```rust
   pub(crate) trait ContextSource: Send + Sync {
       fn source_name(&self) -> &str;
       fn priority(&self) -> u8;  // 0 = lowest, 255 = highest; higher = evicted last
   }
   ```
2. Define `ContextItem`:
   ```rust
   pub(crate) struct ContextItem {
       pub source_name: String,
       pub priority: u8,
       pub score: f64,         // Relevance score, higher = more relevant
       pub content: String,
       pub token_count: usize,
       pub evictable: bool,    // User-requested context (@-mentions) is not evictable
   }
   ```
3. Define `ContextManager`:
   ```rust
   pub(crate) struct ContextManager {
       budget: TokenBudget,
       items: Vec<ContextItem>,
   }
   ```
4. Implement methods:
   - `new(budget: TokenBudget) -> Self`
   - `add(&mut self, item: ContextItem)` -- inserts into the priority queue
   - `render(&self) -> String` -- renders items within budget, highest-scored first, evicting lowest-scored evictable items when budget exceeded
   - `stats(&self) -> ContextUsageStats` -- returns per-source token counts and eviction counts
5. `ContextUsageStats`:
   ```rust
   pub(crate) struct ContextUsageStats {
       pub total_budget: usize,
       pub total_used: usize,
       pub per_source: Vec<(String, usize)>,
       pub items_evicted: usize,
   }
   ```
6. Add `pub(crate) mod context_manager;` to `lib.rs`.

## Design Guidance

Do NOT reuse `ContextAssembler` from `roko-neuro`. The ACP context manager is simpler: it is a budget-aware priority queue, not an attention-scored retrieval system. Keep it under 200 lines. The fancy scoring (diminishing returns, novelty penalties, contrarian retrieval) lives in the neuro layer and feeds items into this manager; the manager only does budget fitting and eviction.

## Write Scope

- `crates/roko-acp/src/lib.rs`

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/09-ACP-MCP.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] Unit test: add 5 items totaling 1000 tokens to a manager with budget 500 -- render returns ~500 tokens, lowest-scored evictable items are dropped
- [ ] Unit test: non-evictable items are always included even when budget is tight
- [ ] Unit test: stats reports correct per-source breakdown
- [ ] Unit test: empty manager renders empty string

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: ACPM_03 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Unit test: add 5 items totaling 1000 tokens to a manager with budget 500 -- render returns ~500 tokens, lowest-scored evictable items are dropped
- Unit test: non-evictable items are always included even when budget is tight
- Unit test: stats reports correct per-source breakdown
- Unit test: empty manager renders empty string
- No files outside the Write Scope are modified.
- Commit message contains `tracker: ACPM_03 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
