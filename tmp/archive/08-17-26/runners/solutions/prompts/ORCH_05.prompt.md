# ORCH_05: Parallel Execution Configuration in WorkflowConfig

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#orch-05`](../ISSUE-TRACKER.md#orch-05)
- Source: `tmp/solutions/roko/tasks/02-ORCHESTRATION.md` — Task 2.5
- Priority: **P1**
- Effort: 3 hours
- Depends on: `ORCH_03` (source 2.3)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: ORCH_05 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

`WorkflowConfig` (at `crates/roko-runtime/src/pipeline_state.rs:37-46`) currently has four fields: `has_strategy`, `has_review`, `max_iterations`, `max_autofix_attempts`. It needs `max_parallel_tasks` and `worktree_isolation` to configure parallel execution.

The TOML parsing in `parse_workflow_config_toml()` (lines 151-236) already handles the `[workflow]` table and `[[workflow.steps]]` array. New keys need to be added to this parser.

The CLI config at `crates/roko-cli/src/config.rs` has `ExecutorConfig` (referenced via `config.executor.max_concurrent_tasks`) which already supports `max_concurrent_tasks`. This needs to flow into `WorkflowConfig` when WorkflowEngine is used for plan execution.

## Exact Changes

1. Add to `WorkflowConfig`:
   ```rust
   pub max_parallel_tasks: usize,      // default: 1
   pub worktree_isolation: bool,       // default: false
   ```
2. Update `WorkflowConfig::express/standard/full()` presets to include the new fields (all default to serial, no isolation).
3. Add TOML parsing for `max_parallel_tasks` and `worktree_isolation` in `parse_workflow_config_toml()` following the existing pattern (lines 216-227).
4. Add `Default` impl update to set `max_parallel_tasks: 1, worktree_isolation: false`.
5. Wire `ExecutorConfig::max_concurrent_tasks` into `WorkflowConfig::max_parallel_tasks` at the CLI entry points.

## Design Guidance

Keep `worktree_isolation` as a boolean rather than a full `WorktreeConfig` at this level. The detailed worktree config (max_live, idle_ttl, worktrees_root) should come from the global roko.toml config, not from the per-workflow TOML. WorkflowEngine should construct WorktreeManager from the global config when `worktree_isolation` is true.

## Write Scope

- `crates/roko-runtime/src/pipeline_state.rs`
- `crates/roko-runtime/src/workflow_engine.rs`
- `crates/roko-cli/src/config.rs`

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/02-ORCHESTRATION.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] `WorkflowConfig::from_toml_str("max_parallel_tasks = 4\nworktree_isolation = true")` parses correctly
- [ ] Default `WorkflowConfig` has `max_parallel_tasks: 1, worktree_isolation: false`
- [ ] Existing TOML parsing tests continue to pass
- [ ] New test: round-trip checkpoint preserves `max_parallel_tasks` and `worktree_isolation`

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: ORCH_05 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- `WorkflowConfig::from_toml_str("max_parallel_tasks = 4\nworktree_isolation = true")` parses correctly
- Default `WorkflowConfig` has `max_parallel_tasks: 1, worktree_isolation: false`
- Existing TOML parsing tests continue to pass
- New test: round-trip checkpoint preserves `max_parallel_tasks` and `worktree_isolation`
- No files outside the Write Scope are modified.
- Commit message contains `tracker: ORCH_05 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
