# DISP_33: Persist CostMeter Data to Durable Log

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#disp-33`](../ISSUE-TRACKER.md#disp-33)
- Source: `tmp/solutions/roko/tasks/03-INFERENCE-DISPATCH.md` — Task 3.33
- Priority: **P2**
- Effort: 2 hours
- Depends on: `DISP_32` (source 3.32)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: DISP_33 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

`CostMeter` in `chat_inline.rs` tracks in-memory cost data during a chat session. This data is displayed in the TUI but lost on exit. It should be persisted to `.roko/learn/costs.jsonl` so cumulative cost tracking works across sessions.

## Exact Changes

1. At chat session exit, serialize `CostMeter` summary to a cost record
2. Append to `.roko/learn/costs.jsonl`
3. Include: session_id, total_cost, model breakdown, turn count, duration
4. The `roko learn efficiency` command should include cost data in its report

## Design Guidance

Cost records are append-only JSONL. Each session produces one summary record. Per-turn cost is already captured in episodes (Task 3.32). The session summary provides aggregate data for trend analysis.

## Write Scope

- `crates/roko-cli/src/chat_inline.rs`

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/03-INFERENCE-DISPATCH.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] After a `roko chat` session, `.roko/learn/costs.jsonl` has a new entry
- [ ] The entry includes total cost and model breakdown
- [ ] `roko learn efficiency` can read and report on cost data

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: DISP_33 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- After a `roko chat` session, `.roko/learn/costs.jsonl` has a new entry
- The entry includes total cost and model breakdown
- `roko learn efficiency` can read and report on cost data
- No files outside the Write Scope are modified.
- Commit message contains `tracker: DISP_33 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
