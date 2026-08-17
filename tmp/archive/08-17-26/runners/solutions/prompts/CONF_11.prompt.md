# CONF_11: Wire CascadeRouter Observations Into Runner V2

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#conf-11`](../ISSUE-TRACKER.md#conf-11)
- Source: `tmp/solutions/roko/tasks/16-CONFIG-AND-WIRING.md` — Task 16.11
- Priority: **P2**
- Effort: Medium
- Depends on: `CONF_10` (source 16.10)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: CONF_11 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

`CascadeRouter` is loaded at `runner/types.rs:1306` and stored in `RunnerConfig`.
A comment at `event_loop.rs:2347` says "CascadeRouter observation: record gate outcome
for learned model selection" but no `cascade_router.observe()` call exists in the
runner event loop. The router accumulates zero observations from plan runs.

`resolve_effective_model()` at `model_selection.rs:140` accepts
`Option<&CascadeRouter>` but callers frequently pass `None`.

## Exact Changes

1. After task completion in the event loop, call `cascade_router.observe()` with the
   model used, success/failure, and response quality metrics.
2. Persist the router to disk after each observation batch (or on flush interval).
3. In `resolve_effective_model_key()` at `model_selection.rs:184`, load the
   CascadeRouter from disk if a `.roko` directory exists instead of hardcoding `None`.
4. Add a startup log line showing CascadeRouter state: observation count, stage, model count.

## Write Scope

- `crates/roko-cli/src/runner/event_loop.rs`
- `crates/roko-cli/src/runner/types.rs`
- `crates/roko-cli/src/model_selection.rs`

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

- [ ] After running a 5-task plan, `.roko/learn/cascade-router.json` has `observations > 0`.
- [ ] `roko learn router` shows observation count increasing after each run.

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: CONF_11 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- After running a 5-task plan, `.roko/learn/cascade-router.json` has `observations > 0`.
- `roko learn router` shows observation count increasing after each run.
- No files outside the Write Scope are modified.
- Commit message contains `tracker: CONF_11 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
