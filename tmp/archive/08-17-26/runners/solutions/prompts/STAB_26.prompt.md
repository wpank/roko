# STAB_26: Wire runner v2 efficiency event recording

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#stab-26`](../ISSUE-TRACKER.md#stab-26)
- Source: `tmp/solutions/roko/tasks/01-STABILITY-AND-FIXES.md` — Task 1.26
- Priority: **P1**
- Effort: 1 hour
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: STAB_26 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

Efficiency events with 30+ fields are not emitted from runner v2. This breaks `roko learn efficiency`.

## Exact Changes

1. After each agent completes in the event loop, construct an `AgentEfficiencyEvent`.
2. Write to `.roko/learn/efficiency.jsonl` via the efficiency sink.
3. Flush immediately after write (avoid the dogfood bug of accumulating without flush).

## Design Guidance

Include at minimum: model, role, task_id, input_tokens, output_tokens, tool_calls_count,
duration_ms, success, cost_usd.

## Write Scope

- `crates/roko-cli/src/runner/event_loop.rs`

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/01-STABILITY-AND-FIXES.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] `roko plan run` on a 3-task plan produces 3 entries in `.roko/learn/efficiency.jsonl`
- [ ] Each entry has non-zero token counts

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: STAB_26 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- `roko plan run` on a 3-task plan produces 3 entries in `.roko/learn/efficiency.jsonl`
- Each entry has non-zero token counts
- No files outside the Write Scope are modified.
- Commit message contains `tracker: STAB_26 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
