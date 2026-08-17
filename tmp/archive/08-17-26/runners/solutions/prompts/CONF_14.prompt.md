# CONF_14: Wire ConductorBandit Into Plan Runner Retry Loop

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#conf-14`](../ISSUE-TRACKER.md#conf-14)
- Source: `tmp/solutions/roko/tasks/16-CONFIG-AND-WIRING.md` — Task 16.14
- Priority: **P3**
- Effort: Medium
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: CONF_14 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

`ConductorBandit` at `crates/roko-learn/src/conductor.rs` (also in
`roko-agent/src/task_runner.rs`, `roko-conductor/src/interventions.rs`) decides
whether a failing task should continue, receive a hint, escalate, restart, or abort.
It is never invoked in runner v2. All retry decisions use hardcoded logic.

## Exact Changes

1. Load `ConductorBandit` state from `.roko/learn/conductor.json` at plan runner start.
2. On task failure, call `bandit.select_action(context)` instead of hardcoded retry.
3. Map actions: Continue -> retry same model, Hint -> inject failure context,
   Escalate -> switch to stronger model, Restart -> clear state and retry,
   Abort -> mark task failed.
4. Feed reward after retry outcome. Save state after each observation.

## Write Scope

- `crates/roko-learn/src/conductor.rs`
- `crates/roko-cli/src/runner/event_loop.rs`

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/16-CONFIG-AND-WIRING.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] After 20+ task completions with mixed success, conductor's action distribution is

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: CONF_14 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- After 20+ task completions with mixed success, conductor's action distribution is
- No files outside the Write Scope are modified.
- Commit message contains `tracker: CONF_14 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
