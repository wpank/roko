# STAB_47: Wire anomaly detector to live paths

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#stab-47`](../ISSUE-TRACKER.md#stab-47)
- Source: `tmp/solutions/roko/tasks/01-STABILITY-AND-FIXES.md` — Task 1.47
- Priority: **P2**
- Effort: 3 hours
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: STAB_47 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

Anomaly detector (prompt loops, cost spikes, quality degradation) is never instantiated.

## Exact Changes

1. Create `AnomalyDetector` at session start.
2. Check prompt hash before each dispatch (detect loops).
3. Check cost after each response (detect spikes).
4. On anomaly: log warning, optionally trigger abort.

## Write Scope

- `crates/roko-learn/src/anomaly.rs`
- `crates/roko-cli/src/run.rs`

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

- [ ] Prompt loop (3 identical prompts) triggers anomaly warning

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: STAB_47 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Prompt loop (3 identical prompts) triggers anomaly warning
- No files outside the Write Scope are modified.
- Commit message contains `tracker: STAB_47 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
