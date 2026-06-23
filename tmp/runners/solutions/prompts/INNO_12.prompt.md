# INNO_12: Add `roko learn costs` CLI command

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#inno-12`](../ISSUE-TRACKER.md#inno-12)
- Source: `tmp/solutions/roko/tasks/11-INNOVATIONS.md` — Task 11.12
- Priority: **P1**
- Effort: 4 hours
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: INNO_12 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

Cost data is recorded in `.roko/learn/efficiency.jsonl` via the efficiency
events system. No CLI command aggregates and displays it.

## Exact Changes

1. Read `.roko/learn/efficiency.jsonl` for cost data.
2. Aggregate: total cost, per-task cost, per-model cost distribution.
3. Compute cost-per-gate-pass: total cost / number of gate passes.
4. Display as a formatted table: Task, Model, Cost, Gate Pass, Cost/Pass.
5. Show model distribution as a simple bar chart (Unicode blocks).
6. Wire into the `learn` subcommand as `roko learn costs`.

## Write Scope

- `crates/roko-cli/src/commands/learn.rs`

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/11-INNOVATIONS.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] `roko learn costs` displays a table after at least one run with cost data
- [ ] Per-model breakdown sums to total cost (within rounding)
- [ ] Cost-per-gate-pass is computed correctly

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: INNO_12 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- `roko learn costs` displays a table after at least one run with cost data
- Per-model breakdown sums to total cost (within rounding)
- Cost-per-gate-pass is computed correctly
- No files outside the Write Scope are modified.
- Commit message contains `tracker: INNO_12 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
