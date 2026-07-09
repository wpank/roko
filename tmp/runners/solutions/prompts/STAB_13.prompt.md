# STAB_13: Wire feedback recording to ACP pipeline

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#stab-13`](../ISSUE-TRACKER.md#stab-13)
- Source: `tmp/solutions/roko/tasks/01-STABILITY-AND-FIXES.md` — Task 1.13
- Priority: **P1**
- Effort: 3 hours
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: STAB_13 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

ACP records only adaptive gate thresholds for rungs 0/1/2. No episodes, no routing
observations, no cost tracking. Editor-integrated usage (VS Code, etc.) is likely the
highest-frequency interaction but produces zero learning signal.

## Exact Changes

1. In ACP pipeline initialization, create `FeedbackService`:
   ```rust
   let feedback = FeedbackService::from_roko_dir_with_episodes(&roko_dir)?;
   ```
2. Thread `feedback` through the ACP runner to all model dispatch points.
3. After each model call in `runner.rs`, emit `FeedbackEvent::ModelCall`.
4. After gate execution, emit `FeedbackEvent::GateResult` (not just threshold updates).
5. On session completion, emit `FeedbackEvent::SessionComplete`.
6. Ensure the feedback service is flushed on session end (not just buffered).

## Design Guidance

ACP sessions can be long-lived (hours in an editor). Use periodic flush (every 5 turns or
30 seconds) rather than waiting for session end. The feedback service instance should be
shared across the session lifetime.

## Write Scope

- `crates/roko-acp/src/runner.rs`
- `crates/roko-acp/src/pipeline.rs`

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

- [ ] Run an ACP session, dispatch a model call, run gates
- [ ] `.roko/episodes.jsonl` has a new entry with source="acp"
- [ ] `.roko/learn/cascade-router.json` observation count increases

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: STAB_13 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Run an ACP session, dispatch a model call, run gates
- `.roko/episodes.jsonl` has a new entry with source="acp"
- `.roko/learn/cascade-router.json` observation count increases
- No files outside the Write Scope are modified.
- Commit message contains `tracker: STAB_13 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
