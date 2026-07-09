# INNO_46: Enhance HDC fingerprinting for routing

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#inno-46`](../ISSUE-TRACKER.md#inno-46)
- Source: `tmp/solutions/roko/tasks/11-INNOVATIONS.md` — Task 11.46
- Priority: **P2**
- Effort: 8 hours
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: INNO_46 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

HdcVector at `crates/roko-primitives/src/hdc.rs` provides `fingerprint()` and
`hamming_similarity()`. CascadeRouter at `crates/roko-learn/src/cascade_router.rs`
does not use HDC for task-to-model matching.

Research: IBM NorthPole projects >100M HDC similarity searches/s on a single
chip. HRR-VSA (arxiv 2502.01657): 82.86% lower cross-entropy loss.

## Exact Changes

1. Compute capability fingerprints per model from historical episode data:
   aggregate HDC fingerprints of tasks where the model succeeded.
2. Compute requirement fingerprints per task from task context.
3. In CascadeRouter, add a routing stage: compute Hamming distance between
   task requirement fingerprint and each model's capability fingerprint.
4. Use HDC distance as a feature in the LinUCB context vector.
5. Track HDC routing accuracy.

## Write Scope

- `crates/roko-primitives/src/hdc.rs`
- `crates/roko-learn/src/cascade_router.rs`

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

- [ ] After 50+ episodes, models have non-trivial capability fingerprints
- [ ] HDC routing selects the model whose capability profile best matches the task
- [ ] HDC distance is included in the LinUCB context vector

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: INNO_46 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- After 50+ episodes, models have non-trivial capability fingerprints
- HDC routing selects the model whose capability profile best matches the task
- HDC distance is included in the LinUCB context vector
- No files outside the Write Scope are modified.
- Commit message contains `tracker: INNO_46 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
