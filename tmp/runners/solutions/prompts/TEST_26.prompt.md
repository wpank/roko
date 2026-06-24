# TEST_26: Learning feedback loop integration test

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#test-26`](../ISSUE-TRACKER.md#test-26)
- Source: `tmp/solutions/roko/tasks/15-TESTING-VERIFICATION.md` — Task 15.26
- Priority: **P1**
- Effort: 4 hours
- Depends on: `TEST_01` (source 15.1), `TEST_05` (source 15.5)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: TEST_26 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

End-to-end test of the learning cycle: agent dispatch -> gate verdict -> episode recording -> routing update -> next dispatch uses updated routing. Uses `LearningRuntime` at `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/runtime_feedback.rs` (line 1243).

## Exact Changes

1. Create a `TestWorkspace` with clean learning state
2. Simulate 5 agent dispatches with varying outcomes:
   - Dispatch 1: model A, compile fail -> episode recorded, router observes failure
   - Dispatch 2: model B, all gates pass -> episode recorded, router observes success
   - Dispatch 3: router should now prefer model B (higher success rate)
   - Dispatch 4: model B again, test fail -> episode recorded
   - Dispatch 5: verify router adjusts (B's success rate declined)
3. After all dispatches verify:
   - `episodes.jsonl` has 5 entries
   - `cascade-router.json` has observations for both models
   - Router's recommended model reflects observed outcomes
   - `gate-thresholds.json` has per-rung observations

## Write Scope

_None — this is a documentation/verification-only batch._

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/15-TESTING-VERIFICATION.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] Full loop verified: dispatch -> gate -> episode -> router -> next dispatch
- [ ] Router recommendations change based on observed outcomes
- [ ] All learning artifacts are consistent at end of loop

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: TEST_26 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Full loop verified: dispatch -> gate -> episode -> router -> next dispatch
- Router recommendations change based on observed outcomes
- All learning artifacts are consistent at end of loop
- No files outside the Write Scope are modified.
- Commit message contains `tracker: TEST_26 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
