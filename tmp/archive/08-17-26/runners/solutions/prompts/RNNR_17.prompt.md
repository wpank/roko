# RNNR_17: Wire AntiPatternChecker as pre-gate in the runner pipeline

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#rnnr-17`](../ISSUE-TRACKER.md#rnnr-17)
- Source: `tmp/solutions/roko/tasks/14-RUNNER-PATTERNS.md` — Task 14.17
- Priority: **??**
- Effort: ?
- Depends on: `RNNR_16` (source 14.16)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: RNNR_17 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

: Run anti-pattern checks after agent completion but before compilation
gates. Catches structural mistakes without waiting for `cargo check`.

## Exact Changes

1. After `AgentCompleted`, before transitioning to gate phase, run
   `AntiPatternChecker::check()` on files changed by the agent
2. If any `Severity::Error` violations found, treat as gate failure:
   inject violation details into retry context and re-dispatch
3. If only `Severity::Warning` violations, log but continue to gates
4. In wave-gate mode, still run AP checks per-task (fast enough at <100ms)
   even when compilation is deferred
5. Track AP check duration as a runner event

## Write Scope

- `crates/roko-cli/src/runner/event_loop.rs`
- `crates/roko-cli/src/runner/gate_dispatch.rs`

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

- [ ] Anti-pattern checks run on every task completion, regardless of gate mode
- [ ] Error-severity violations trigger immediate retry (no wasted compilation)
- [ ] Warning-severity violations logged but do not block
- [ ] AP checks complete in < 100ms per task

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: RNNR_17 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Anti-pattern checks run on every task completion, regardless of gate mode
- Error-severity violations trigger immediate retry (no wasted compilation)
- Warning-severity violations logged but do not block
- AP checks complete in < 100ms per task
- No files outside the Write Scope are modified.
- Commit message contains `tracker: RNNR_17 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
