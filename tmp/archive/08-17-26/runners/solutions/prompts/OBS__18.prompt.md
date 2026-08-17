# OBS__18: Integrate new widgets into TUI App and view dispatch

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#obs--18`](../ISSUE-TRACKER.md#obs--18)
- Source: `tmp/solutions/roko/tasks/18-OBSERVABILITY.md` — Task 18.18
- Priority: **??**
- Effort: ?
- Depends on: `OBS__14` (source 18.14), `OBS__15` (source 18.15), `OBS__16` (source 18.16), `OBS__17` (source 18.17)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: OBS__18 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

_(no context section in source)_

## Exact Changes

_(no implementation section in source — read source task)_

## Write Scope

- `crates/roko-cli/src/tui/app.rs`
- `crates/roko-cli/src/tui/views/dashboard_view.rs`
- `crates/roko-cli/src/tui/state.rs`
- `crates/roko-cli/src/tui/tabs.rs`

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/18-OBSERVABILITY.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] `rg 'router_decisions\|cost_history\|task_gates\|swarm_state' crates/roko-cli/src/tui/state.rs` matches >= 4.
- [ ] `rg 'ObservabilityEvent' crates/roko-cli/src/tui/app.rs` matches >= 1.

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: OBS__18 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- `rg 'router_decisions\|cost_history\|task_gates\|swarm_state' crates/roko-cli/src/tui/state.rs` matches >= 4.
- `rg 'ObservabilityEvent' crates/roko-cli/src/tui/app.rs` matches >= 1.
- No files outside the Write Scope are modified.
- Commit message contains `tracker: OBS__18 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
