# RNNR_27: Implement model escalation on repeated failure

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#rnnr-27`](../ISSUE-TRACKER.md#rnnr-27)
- Source: `tmp/solutions/roko/tasks/14-RUNNER-PATTERNS.md` — Task 14.27
- Priority: **??**
- Effort: ?
- Depends on: `RNNR_13` (source 14.13)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: RNNR_27 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

: When a cheap model fails the same gate repeatedly, escalate to a
stronger model. The `TaskRunnerError::ModelEscalation` variant already exists
but the runner event loop does not act on it to switch models.

## Exact Changes

1. Add `ModelEscalation` config to `RunConfig`:
   ```rust
   pub struct ModelEscalationConfig {
       pub enabled: bool,
       pub escalation_after: u32,      // attempts before escalating (default 2)
       pub strong_model: Option<String>,  // override; otherwise use CascadeRouter
   }
   ```
2. In the event loop, track attempt count per task
3. When attempt count exceeds `escalation_after`, override the model for the
   next dispatch with `strong_model` (or ask CascadeRouter for a stronger option)
4. Log: "Task {id}: escalating from {cheap} to {strong} after {n} failures"
5. Record escalation as a CascadeRouter observation for future routing

## Write Scope

- `crates/roko-cli/src/runner/event_loop.rs`

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/14-RUNNER-PATTERNS.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] First 2 attempts use the configured/default model
- [ ] Third attempt uses the stronger model
- [ ] Escalation logged and trackable
- [ ] CascadeRouter receives the observation for future routing

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: RNNR_27 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- First 2 attempts use the configured/default model
- Third attempt uses the stronger model
- Escalation logged and trackable
- CascadeRouter receives the observation for future routing
- No files outside the Write Scope are modified.
- Commit message contains `tracker: RNNR_27 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
