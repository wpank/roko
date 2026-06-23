# PERF_15: Add `--gates` CLI Flag

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#perf-15`](../ISSUE-TRACKER.md#perf-15)
- Source: `tmp/solutions/roko/tasks/10-PERFORMANCE.md` — Task 10.15
- Priority: **??**
- Effort: ?
- Depends on: `PERF_14` (source 10.14)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: PERF_15 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

Expose `GateMode` as `--gates <MODE>` on `roko run`.

## Exact Changes

1. Add `--gates <MODE>` argument to the `run` subcommand, using `GateMode`'s
   `ValueEnum` derive
2. Default to `auto` for `roko run` (interactive runs benefit from detection)
3. Default to `full` for `roko plan run` (plan execution is thorough)
4. Pass resolved `GateMode` through to `WorkflowConfig::with_gate_mode()`
5. Log resolved gate mode at run start

## Write Scope

- `crates/roko-cli/src/main.rs`
- `crates/roko-cli/src/run.rs`

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/10-PERFORMANCE.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] `roko run --gates none "echo hello"` skips all gates
- [ ] `roko run --gates express "echo hello"` runs only diff + fmt
- [ ] `roko run "echo hello"` defaults to auto-detection
- [ ] `roko --help` shows `--gates` with value options

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: PERF_15 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- `roko run --gates none "echo hello"` skips all gates
- `roko run --gates express "echo hello"` runs only diff + fmt
- `roko run "echo hello"` defaults to auto-detection
- `roko --help` shows `--gates` with value options
- No files outside the Write Scope are modified.
- Commit message contains `tracker: PERF_15 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
