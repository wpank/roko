# STAB_49: Add end-of-run summary to plan runner

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#stab-49`](../ISSUE-TRACKER.md#stab-49)
- Source: `tmp/solutions/roko/tasks/01-STABILITY-AND-FIXES.md` — Task 1.49
- Priority: **P2**
- Effort: 3 hours
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: STAB_49 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

After `roko plan run` completes, no aggregate outcome summary is printed. Users must read
log files to determine results.

## Exact Changes

1. After all tasks complete, collect results from executor state.
2. Print summary:
   ```
   Run complete: {plan_name}
     Passed: 8/10 tasks
     Failed: T6 (gate: clippy), T9 (gate: test)
     Skipped: 0
     Cost: $8.47 | Duration: 34min
     Resume: roko plan run plans/ --resume .roko/state/executor.json
   ```
3. Save to `.roko/state/last-run-summary.json`.

## Write Scope

- `crates/roko-cli/src/runner/event_loop.rs`
- `crates/roko-cli/src/commands/plan.rs`

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

- [ ] `roko plan run` on 3-task plan prints summary with pass/fail counts
- [ ] Summary includes cost and duration

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: STAB_49 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- `roko plan run` on 3-task plan prints summary with pass/fail counts
- Summary includes cost and duration
- No files outside the Write Scope are modified.
- Commit message contains `tracker: STAB_49 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
