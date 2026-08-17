# EVAL_26: Sampling strategy and adaptive budget

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#eval-26`](../ISSUE-TRACKER.md#eval-26)
- Source: `tmp/solutions/roko/tasks/05-GATE-EVOLUTION.md` — Task 5.26
- Priority: **P2**
- Effort: 3 hours
- Depends on: `EVAL_18` (source 5.18)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: EVAL_26 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

N=3 at T=0 per position (base). Adaptive increase when panel disagrees.

## Exact Changes

1. Implement `adaptive_sample_count(initial_agreement: f64, base_samples: u32, max_samples: u32) -> u32`:
   - agreement >= 0.8 -> base_samples
   - agreement >= 0.5 -> 2 * base_samples
   - agreement < 0.5 -> max_samples

## Write Scope

- `crates/roko-eval-judge/src/lib.rs`

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/05-GATE-EVOLUTION.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] Unit test for each agreement tier

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: EVAL_26 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Unit test for each agreement tier
- No files outside the Write Scope are modified.
- Commit message contains `tracker: EVAL_26 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
