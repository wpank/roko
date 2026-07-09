# PROM_24: Add Attention Curve Learning from Gate Outcomes

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#prom-24`](../ISSUE-TRACKER.md#prom-24)
- Source: `tmp/solutions/roko/tasks/06-PROMPT-ASSEMBLY.md` — Task 6.24
- Priority: **??**
- Effort: ?
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: PROM_24 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

: After each dispatch, if the task's critical information was at a
known position and the gate outcome is known, update model curve parameters.

## Exact Changes

1. Add `pub fn record_placement_outcome(&mut self, model: &str, position: f64, success: bool)` to `ModelAttentionCurves`
2. Track per-model, per-position-bin (5 bins: 0.0-0.2, 0.2-0.4, ...) success rates
3. After 20+ observations per bin, refit curve parameters to match observed patterns
4. Persist updated curves to `.roko/learn/attention-curves.json`

## Write Scope

_None — this is a documentation/verification-only batch._

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/06-PROMPT-ASSEMBLY.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] After 20+ tasks with position-tracked critical info, per-model curves are updated
- [ ] Updated curves reflect observed position-success patterns
- [ ] Curves persist across runs

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: PROM_24 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- After 20+ tasks with position-tracked critical info, per-model curves are updated
- Updated curves reflect observed position-success patterns
- Curves persist across runs
- No files outside the Write Scope are modified.
- Commit message contains `tracker: PROM_24 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
