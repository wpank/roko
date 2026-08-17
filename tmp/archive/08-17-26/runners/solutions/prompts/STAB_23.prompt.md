# STAB_23: Wire runner v2 AdaptiveThreshold observations

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#stab-23`](../ISSUE-TRACKER.md#stab-23)
- Source: `tmp/solutions/roko/tasks/01-STABILITY-AND-FIXES.md` — Task 1.23
- Priority: **P1**
- Effort: 1 hour
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: STAB_23 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

Runner v2 does not call `AdaptiveThresholds::observe()` after gate execution. The adaptive
threshold system (`adaptive_threshold.rs`) includes SPC monitoring (CUSUM, EWMA, BOCPD) but
receives no observations from the default execution path.

## Exact Changes

1. After each gate verdict in `event_loop.rs`:
   ```rust
   thresholds.observe(rung, verdict.passed);
   ```
2. Before each gate dispatch, check adaptive skip:
   ```rust
   if thresholds.should_skip_rung(rung) {
       tracing::info!(rung, "adaptive skip: rung has high pass rate");
       continue; // skip this gate
   }
   ```
3. Record skip decisions in the episode for debugging.
4. Persist thresholds during periodic flush.

## Design Guidance

Adaptive skipping should be conservative -- only skip when pass rate is > 0.99 over 50+
observations. This prevents skipping gates that occasionally catch issues.

## Write Scope

- `crates/roko-cli/src/runner/event_loop.rs`
- `crates/roko-gate/src/adaptive_threshold.rs`

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

- [ ] `roko plan run` with gates produces `.roko/learn/gate-thresholds.json` with `total_observations > 0`

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: STAB_23 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- `roko plan run` with gates produces `.roko/learn/gate-thresholds.json` with `total_observations > 0`
- No files outside the Write Scope are modified.
- Commit message contains `tracker: STAB_23 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
