# ORCH_02: Add Worktree Allocation to EffectDriver Agent Spawn

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#orch-02`](../ISSUE-TRACKER.md#orch-02)
- Source: `tmp/solutions/roko/tasks/02-ORCHESTRATION.md` — Task 2.2
- Priority: **P0**
- Effort: 6 hours
- Depends on: `ORCH_01` (source 2.1)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: ORCH_02 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

`EffectDriver::spawn_agent()` (at line 87-275) currently operates on `self.workdir` for all agent calls and git operations. When worktree isolation is active, each agent needs its own worktree directory. The `spawn_agent` method needs a `task_id` parameter (or an overloaded variant) so it can request a worktree from `WorktreeManager` and set the agent's working directory to the worktree path.

The current signature is:
```rust
pub async fn spawn_agent(&self, role: &str, user_prompt: &str, context: Option<&str>) -> PipelineInput
```

The `PromptAssembler::assemble()` call at line 121-132 passes `workdir: Some(self.workdir.clone())` in the `PromptSpec`. The git operations (diff at line 621-636, commit at line 340-421) use `self.workdir`. All of these must use the task-specific worktree path when isolation is active.

## Exact Changes

1. Add a new method `spawn_agent_in_worktree()` that accepts an additional `task_id: &str` and optional `worktree_path: Option<PathBuf>` parameter.
2. When `worktree_path` is `Some`, use it instead of `self.workdir` for:
   - `PromptSpec::workdir` in the prompt assembly call
   - `count_changed_files()` call after agent completion
   - Any future git operations
3. Keep the existing `spawn_agent()` method as a backward-compatible wrapper that calls `spawn_agent_in_worktree()` with `task_id: "default"` and `worktree_path: None`.
4. If `self.services.worktree_manager` is `Some` and `worktree_path` is `None`, allocate a new worktree via `worktree_manager.create_for_plan(task_id)`. Store the returned `WorktreeHandle::path` and use it.
5. After agent completion, touch the worktree handle via `worktree_manager.touch(task_id)` to prevent idle reclamation.
6. Add cleanup logic: if the worktree was created by this call and the agent failed, keep the worktree for debugging (do not auto-remove).

## Design Guidance

The worktree lifecycle should be managed by the caller (WorkflowEngine run loop), not by `spawn_agent` itself. `spawn_agent_in_worktree` should accept the path, not manage creation/deletion. This keeps the EffectDriver stateless with respect to worktree lifecycle.

## Write Scope

- `crates/roko-runtime/src/effect_driver.rs`

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

- [ ] `spawn_agent()` backward-compatible -- existing tests pass unchanged
- [ ] New `spawn_agent_in_worktree()` method accepts and uses a custom workdir
- [ ] `PromptSpec::workdir` reflects the worktree path, not the global workdir
- [ ] Unit test: `spawn_agent_in_worktree` with a custom tempdir as worktree path produces `AgentCompleted` with correct workdir context

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: ORCH_02 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- `spawn_agent()` backward-compatible -- existing tests pass unchanged
- New `spawn_agent_in_worktree()` method accepts and uses a custom workdir
- `PromptSpec::workdir` reflects the worktree path, not the global workdir
- Unit test: `spawn_agent_in_worktree` with a custom tempdir as worktree path produces `AgentCompleted` with correct workdir context
- No files outside the Write Scope are modified.
- Commit message contains `tracker: ORCH_02 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
