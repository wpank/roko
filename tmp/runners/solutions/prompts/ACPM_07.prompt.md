# ACPM_07: Per-Turn Context Usage Tracking

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#acpm-07`](../ISSUE-TRACKER.md#acpm-07)
- Source: `tmp/solutions/roko/tasks/09-ACP-MCP.md` — Task 9.7
- Priority: **P2**
- Effort: 3 hours
- Depends on: `ACPM_04` (source 9.4)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: ACPM_07 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

The cascade router is updated with bandit observations after each ACP dispatch (in `bridge_events.rs`), but there is no record of what context was provided to the agent. Without this data, the learning system cannot optimize context budgets.

## Exact Changes

1. Define `ContextUsageRecord`:
   ```rust
   struct ContextUsageRecord {
       turn_id: String,
       total_budget: usize,
       knowledge_tokens: usize,
       playbook_tokens: usize,
       file_tokens: usize,
       history_tokens: usize,
       items_evicted: usize,
       success: bool,
       timestamp: DateTime<Utc>,
   }
   ```
2. After each prompt completes, construct the record from `ContextManager::stats()`.
3. Append to `.roko/learn/context-usage.jsonl` via JSONL file append.
4. In the cascade router observation, include `context_budget` as a feature in the routing context vector (extend the `RoutingContext` struct).

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

- [ ] Run 5 ACP prompts -- `.roko/learn/context-usage.jsonl` has 5 entries
- [ ] Each entry has non-zero `total_budget` and accurate source breakdowns
- [ ] Cascade router context vector includes budget feature

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: ACPM_07 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Run 5 ACP prompts -- `.roko/learn/context-usage.jsonl` has 5 entries
- Each entry has non-zero `total_budget` and accurate source breakdowns
- Cascade router context vector includes budget feature
- No files outside the Write Scope are modified.
- Commit message contains `tracker: ACPM_07 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
