# ACPM_04: Wire ContextManager into ACP Bridge Events

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#acpm-04`](../ISSUE-TRACKER.md#acpm-04)
- Source: `tmp/solutions/roko/tasks/09-ACP-MCP.md` — Task 9.4
- Priority: **P0**
- Effort: 4 hours
- Depends on: `ACPM_03` (source 9.3)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: ACPM_04 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

In `bridge_events.rs`, the `handle_session_prompt()` function (line ~679) assembles context via inline `append_context()` calls:
1. Queries `query_dispatch_knowledge()` for knowledge + playbooks (line ~741)
2. Gets the mode-specific system prompt (lines ~746-770)
3. Calls `append_context()` to merge knowledge into system prompt (lines ~771-780)
4. Builds conversation history (lines ~785-800)

This needs to be replaced with the `ContextManager` flow.

## Exact Changes

1. At the start of `handle_session_prompt()`, construct a `ContextManager` with budget from session config or model's `max_context_tokens` (default 100,000):
   ```rust
   let budget = TokenBudget::from_total(session.context_budget.unwrap_or(100_000));
   let mut ctx = ContextManager::new(budget);
   ```
2. Add knowledge hits as `ContextItem { source_name: "knowledge", priority: 128, evictable: true }`
3. Add playbook context as `ContextItem { source_name: "playbook", priority: 120, evictable: true }`
4. Add file context from @-mentions as `ContextItem { source_name: "file_context", priority: 200, evictable: false }` (user-requested context is never evicted)
5. Add conversation history as `ContextItem { source_name: "history", priority: 100, evictable: true }` with recency-based scoring (recent turns scored higher)
6. Call `ctx.render()` to produce the final context string
7. Replace the existing `append_context()` chain with the manager output
8. Log `ctx.stats()` for debugging via `tracing::debug!`

## Write Scope

- `crates/roko-acp/src/bridge_events.rs`

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

- [ ] Run ACP session with a prompt that @-mentions 5 files -- verify file context appears in agent prompt
- [ ] Verify knowledge context is truncated when file context consumes most of the budget
- [ ] Existing ACP unit tests pass
- [ ] `tracing::debug` output shows context stats with per-source breakdown

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: ACPM_04 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Run ACP session with a prompt that @-mentions 5 files -- verify file context appears in agent prompt
- Verify knowledge context is truncated when file context consumes most of the budget
- Existing ACP unit tests pass
- `tracing::debug` output shows context stats with per-source breakdown
- No files outside the Write Scope are modified.
- Commit message contains `tracker: ACPM_04 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
