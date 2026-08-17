# ACPM_02: Add TokenBudget Struct and Budget-Aware Knowledge Query

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#acpm-02`](../ISSUE-TRACKER.md#acpm-02)
- Source: `tmp/solutions/roko/tasks/09-ACP-MCP.md` — Task 9.2
- Priority: **P0**
- Effort: 4 hours
- Depends on: `ACPM_01` (source 9.1)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: ACPM_02 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

`query_dispatch_knowledge()` at `crates/roko-acp/src/knowledge.rs:65-82` returns unbounded results: hardcoded top-5 knowledge hits and top-3 playbooks regardless of the model's context window size. `render_context_body()` and `render_playbook_context()` render everything they receive with no truncation. The existing `ContextAssembler` in `crates/roko-neuro/src/context.rs:442` has a `max_context_tokens` field but is not used in the ACP path -- it is used by the orchestrate.rs path.

## Exact Changes

1. Add `TokenBudget` struct to `knowledge.rs`:
   ```rust
   #[derive(Debug, Clone)]
   pub(crate) struct TokenBudget {
       pub total: usize,
       pub system_prompt: usize,
       pub history: usize,
       pub knowledge: usize,
       pub file_context: usize,
       pub tool_results: usize,
   }

   impl TokenBudget {
       pub fn from_total(total: usize) -> Self {
           // Allocate: 15% system prompt, 30% history, 20% knowledge,
           // 25% file context, 10% tool results
           Self {
               total,
               system_prompt: total * 15 / 100,
               history: total * 30 / 100,
               knowledge: total * 20 / 100,
               file_context: total * 25 / 100,
               tool_results: total * 10 / 100,
           }
       }
   }
   ```
2. Add `budget: usize` parameter to `query_dispatch_knowledge()`:
   ```rust
   pub(crate) async fn query_dispatch_knowledge(
       workdir: &Path,
       prompt: &str,
       budget: usize,
   ) -> DispatchKnowledge
   ```
3. In `render_context_body()`, track cumulative token count. Stop adding knowledge hits when the running total exceeds the budget. Order by score (highest first, which is already the case).
4. In `render_playbook_context()`, allocate per-playbook budget as `budget / playbooks.len()`. Truncate step lists when budget exceeded.
5. Add `tokens_used: usize` field to `DispatchKnowledge` to report actual usage.
6. Update all callers of `query_dispatch_knowledge()` in `bridge_events.rs` (3 call sites at lines 741, 2003, 2045) to pass a default budget of `20_000` tokens (approximately 20% of a 100K context window).

## Write Scope

- `crates/roko-acp/src/knowledge.rs`

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

- [ ] Existing `card_and_context_include_results` test passes unchanged
- [ ] Existing `missing_stores_return_empty_results` test passes unchanged
- [ ] New unit test: `query_dispatch_knowledge` with `budget = 500` returns fewer items than with `budget = 20_000`
- [ ] New unit test: `render_context_body` with 10 large knowledge hits and budget 200 truncates to 2-3 hits

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: ACPM_02 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Existing `card_and_context_include_results` test passes unchanged
- Existing `missing_stores_return_empty_results` test passes unchanged
- New unit test: `query_dispatch_knowledge` with `budget = 500` returns fewer items than with `budget = 20_000`
- New unit test: `render_context_body` with 10 large knowledge hits and budget 200 truncates to 2-3 hits
- No files outside the Write Scope are modified.
- Commit message contains `tracker: ACPM_02 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
