# PERF_03: Shared Config Cache: Load Once, Arc Through

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#perf-03`](../ISSUE-TRACKER.md#perf-03)
- Source: `tmp/solutions/roko/tasks/10-PERFORMANCE.md` — Task 10.3
- Priority: **??**
- Effort: ?
- Depends on: `PERF_01` (source 10.1)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: PERF_03 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

Eliminate 4+ redundant `roko.toml` loads per `roko run`. Load once at
CLI entry, wrap in `Arc`, pass through the call chain.

## Exact Changes

1. In `resolve_workflow_model_selection()` (line 420), this already loads both
   `load_layered()` and `load_config()`. Refactor so the caller loads once and
   passes the result in.
2. Change `dispatch_agent()` (line 1846) to accept config as a parameter instead
   of calling `load_config(workdir)` internally at line 1855
3. Change `append_episode_log()` (line 2545) to accept config -- remove its
   internal `load_roko_config_models(workdir)` call at line 2656
4. Remove standalone `load_config()` / `load_roko_config_models()` calls from
   functions that now receive config as parameter
5. Add `debug!("config loaded once, {} providers", ...)` at the single load site

## Write Scope

- `crates/roko-cli/src/run.rs`
- `crates/roko-cli/src/model_selection.rs`

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

- [ ] `RUST_LOG=roko_cli=debug roko run "echo hello"` shows exactly ONE "config
- [ ] `roko run --model <override> "test"` still correctly uses the override

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: PERF_03 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- `RUST_LOG=roko_cli=debug roko run "echo hello"` shows exactly ONE "config
- `roko run --model <override> "test"` still correctly uses the override
- No files outside the Write Scope are modified.
- Commit message contains `tracker: PERF_03 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
