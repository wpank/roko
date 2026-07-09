# INNO_44: Wire RLAIF/RLSF pattern into learning loop

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#inno-44`](../ISSUE-TRACKER.md#inno-44)
- Source: `tmp/solutions/roko/tasks/11-INNOVATIONS.md` — Task 11.44
- Priority: **P2**
- Effort: 8 hours
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: INNO_44 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

Research: Absolute Zero Reasoner (NeurIPS 2025 Spotlight) -- trains from identity
seed with executor as only reward. Roko's Verify gate pipeline is a strict
superset. Dohmatob 2025: accumulate-only constraint prevents model collapse.

## Exact Changes

1. After gate passes, record the full trajectory as a positive training signal.
2. After gate fails, apply AgentHER (Hindsight Experience Replay): ask "what
   sub-goals did this trajectory actually achieve?" and record those as positive
   episodes for sub-goals.
3. Store trajectory quality scores alongside episodes.
4. Feed trajectory quality into CascadeRouter observations.
5. Implement accumulate-only constraint: synthetic/relabeled data always added
   to real data, never replaces it. Tag synthetic entries.

## Write Scope

- `crates/roko-learn/src/feedback_service.rs`
- `crates/roko-learn/src/runtime_feedback.rs`

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

- [ ] After a gate pass, a positive trajectory is recorded with quality score
- [ ] After a gate fail, at least one sub-goal is identified and recorded
- [ ] CascadeRouter observations include trajectory quality
- [ ] Synthetic entries are tagged and never replace real entries

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: INNO_44 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- After a gate pass, a positive trajectory is recorded with quality score
- After a gate fail, at least one sub-goal is identified and recorded
- CascadeRouter observations include trajectory quality
- Synthetic entries are tagged and never replace real entries
- No files outside the Write Scope are modified.
- Commit message contains `tracker: INNO_44 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
