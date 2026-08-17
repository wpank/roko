# STAB_28: Wire gate failure classification to retry/replan routing

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#stab-28`](../ISSUE-TRACKER.md#stab-28)
- Source: `tmp/solutions/roko/tasks/01-STABILITY-AND-FIXES.md` — Task 1.28
- Priority: **P1**
- Effort: 3 hours
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: STAB_28 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

`classify_gate_error` in `compile_errors.rs` computes failure actions (Retry, NeedsReplan,
Blocked, NeedsHuman) but the action is rendered as a string and discarded. The orchestrator
always retries regardless of classification.

## Exact Changes

1. After gate failure, call `classify_gate_failure(&output)` to get the recommended action.
2. Route based on the action:
   - `Retry`: continue with existing retry logic (feedback to agent)
   - `NeedsReplan`: emit replan event, construct a strategist prompt with the errors
   - `Blocked`: pause the task, mark as blocked, log the reason
   - `NeedsHuman`: pause the task, emit notification, set status to "needs-human"
3. Expose the classification in the episode record.
4. For runner v2: implement at least `Retry` and `NeedsReplan` actions.

## Design Guidance

The classification should be transparent -- log the classification result so users understand
why the system chose to retry vs. replan vs. block.

## Write Scope

- `crates/roko-gate/src/compile_errors.rs`
- `crates/roko-cli/src/runner/gate_dispatch.rs`
- `crates/roko-cli/src/runner/event_loop.rs`

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

- [ ] Gate failure classified as `Retry` triggers normal retry with feedback
- [ ] Gate failure classified as `NeedsReplan` triggers a strategist agent call
- [ ] Classification appears in episode record

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: STAB_28 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Gate failure classified as `Retry` triggers normal retry with feedback
- Gate failure classified as `NeedsReplan` triggers a strategist agent call
- Classification appears in episode record
- No files outside the Write Scope are modified.
- Commit message contains `tracker: STAB_28 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
