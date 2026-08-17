# CONF_04: Wire `learning.replan_on_gate_failure` Into Runner V2

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#conf-04`](../ISSUE-TRACKER.md#conf-04)
- Source: `tmp/solutions/roko/tasks/16-CONFIG-AND-WIRING.md` — Task 16.4
- Priority: **P2**
- Effort: Medium
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: CONF_04 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

`learning.replan_on_gate_failure` is parsed from roko.toml into
`LearningConfig.replan_on_gate_failure` (`roko-core/src/config/learning.rs:39`,
defaults to `true`). The flag is consumed by orchestrate.rs (`orchestrate.rs:5101`)
but runner v2 never reads it. Gate failures exhaust the autofix budget and mark the
task as failed without triggering replanning.

## Exact Changes

1. Add `replan_on_gate_failure: bool` to `RunnerConfig`, populated from
   `roko_config.learning.replan_on_gate_failure`.
2. In the event loop, after autofix budget is exhausted, if `replan_on_gate_failure`
   is true, extract or call `build_gate_failure_plan_revision()` from orchestrate.rs.
3. The revision spawns a strategist agent with gate error context to produce a revised
   approach, which is then retried.
4. Cap replan attempts at 1 per task to prevent infinite loops.

## Write Scope

- `crates/roko-cli/src/runner/types.rs`
- `crates/roko-cli/src/runner/event_loop.rs`
- `crates/roko-cli/src/orchestrate.rs`

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

- [ ] With `replan_on_gate_failure = true`, a task that fails all autofix attempts spawns a
- [ ] The strategist's output is visible in the episode log.
- [ ] With `replan_on_gate_failure = false`, behavior is unchanged from current.

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: CONF_04 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- With `replan_on_gate_failure = true`, a task that fails all autofix attempts spawns a
- The strategist's output is visible in the episode log.
- With `replan_on_gate_failure = false`, behavior is unchanged from current.
- No files outside the Write Scope are modified.
- Commit message contains `tracker: CONF_04 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
