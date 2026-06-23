# LERN_18: Wire StagingBuffer Promotion Without Full Dream Cycle

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#lern-18`](../ISSUE-TRACKER.md#lern-18)
- Source: `tmp/solutions/roko/tasks/07-LEARNING-FEEDBACK.md` — Task 7.18
- Priority: **P2**
- Effort: 3 hours
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: LERN_18 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

`StagingBuffer` (at `roko-dreams/src/staging.rs:93`) holds dream-generated knowledge candidates that progress through Raw -> Replayed -> Validated stages. Promotion to the durable `KnowledgeStore` only happens during a full `DreamCycle::run()`. Without a running dream cycle, the staging buffer grows without bound.

`LearningRuntime::record_completed_run()` (at `runtime_feedback.rs:2075`) has a knowledge seed append step. This is the natural place to add a lightweight promotion check.

## Exact Changes

1. Add `staging_buffer: Option<StagingBuffer>` to `LearningRuntime` (or load on demand from `.roko/learn/staging-buffer.json`).
2. At the end of `record_completed_run()`, after knowledge seed append:
   - Load `StagingBuffer` from disk if not in memory.
   - Check for entries at `StagingStage::Validated`.
   - For each validated entry, promote to `KnowledgeStore` via the store's `append()` method.
   - Remove promoted entries from the staging buffer.
   - Save the buffer back.
3. This promotion is lightweight (no LLM calls, no clustering) -- it just moves already-validated candidates into the durable store.
4. Log promotions at INFO level.

## Write Scope

- `crates/roko-learn/src/runtime_feedback.rs`

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/07-LEARNING-FEEDBACK.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] Generate knowledge candidates that reach Validated stage
- [ ] After `record_completed_run()`, validated entries appear in the durable store
- [ ] No full dream cycle required for promotion

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: LERN_18 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Generate knowledge candidates that reach Validated stage
- After `record_completed_run()`, validated entries appear in the durable store
- No full dream cycle required for promotion
- No files outside the Write Scope are modified.
- Commit message contains `tracker: LERN_18 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
