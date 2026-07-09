# INNO_58: Implement Bayesian Model Reduction in dream cycle

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#inno-58`](../ISSUE-TRACKER.md#inno-58)
- Source: `tmp/solutions/roko/tasks/11-INNOVATIONS.md` — Task 11.58
- Priority: **P3**
- Effort: 8 hours
- Depends on: `INNO_45` (source 11.45)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: INNO_58 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

Research: AXIOM (arxiv 2505.24784) -- BMR with 7.6x sample efficiency, 39x
faster wall-clock.

DreamCycle at `crates/roko-dreams/src/cycle.rs` has `pub fn run_dream`.

## Exact Changes

1. During consolidation, compute evidence for each knowledge entry: how many
   episodes support vs contradict it.
2. Apply BMR: score candidate knowledge models from accumulated posteriors.
3. Prune low-evidence entries (evidence < threshold).
4. Merge near-duplicate entries (HDC similarity > 0.95).
5. Log pruning decisions.

## Write Scope

- `crates/roko-dreams/src/cycle.rs`

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

- [ ] After dream consolidation, knowledge store has fewer entries but higher average confidence
- [ ] Entries with zero supporting episodes are pruned
- [ ] Near-duplicate entries are merged

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: INNO_58 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- After dream consolidation, knowledge store has fewer entries but higher average confidence
- Entries with zero supporting episodes are pruned
- Near-duplicate entries are merged
- No files outside the Write Scope are modified.
- Commit message contains `tracker: INNO_58 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
