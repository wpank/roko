# ORCH_01: Add WorktreeManager and MergeQueue to EffectServices

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#orch-01`](../ISSUE-TRACKER.md#orch-01)
- Source: `tmp/solutions/roko/tasks/02-ORCHESTRATION.md` — Task 2.1
- Priority: **P0**
- Effort: 4 hours
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: ORCH_01 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

`EffectServices` (defined at `crates/roko-runtime/src/effect_driver.rs:37-50`) is the service injection point for the WorkflowEngine. It currently holds five services: `default_model`, `model_caller`, `prompt_assembler`, `feedback_sink`, `gate_runner`, and optional `affect_policy`. It has no reference to worktree or merge infrastructure.

`WorktreeManager` exists at `crates/roko-orchestrator/src/worktree.rs` (1,203 LOC) and is fully functional: `create_for_plan()`, `remove()`, `touch()`, `reclaim_idle()`, `health()`, `clear_stale_locks()`, `prune()`. It is re-exported from `crates/roko-orchestrator/src/lib.rs:105-108`.

`MergeQueue` exists at `crates/roko-orchestrator/src/merge_queue.rs` (924 LOC) with file-conflict-aware serialization: `enqueue()`, `dequeue()`, `complete()`, `fail()`. Re-exported from `crates/roko-orchestrator/src/lib.rs:79-82`.

`roko-runtime/Cargo.toml` currently depends on `roko-core` and `roko-primitives` but NOT on `roko-orchestrator`. Adding this dependency is required.

## Exact Changes

1. Add `roko-orchestrator = { path = "../roko-orchestrator" }` to `crates/roko-runtime/Cargo.toml` under `[dependencies]`.
2. Add two optional fields to `EffectServices` in `crates/roko-runtime/src/effect_driver.rs`:
   ```rust
   pub worktree_manager: Option<Arc<roko_orchestrator::WorktreeManager>>,
   pub merge_queue: Option<Arc<roko_orchestrator::MergeQueue>>,
   ```
3. Update the `EffectDriver::new()` constructor -- no behavior change, just pass-through.
4. Update all existing call sites that construct `EffectServices` (search for `EffectServices {` in `crates/roko-runtime/src/` and `crates/roko-cli/src/`) to add `worktree_manager: None, merge_queue: None`.
5. Update the test mock `EffectServices` in `crates/roko-runtime/src/effect_driver.rs:822-831` to include the new fields as `None`.

## Design Guidance

Use `Option<Arc<T>>` for the worktree/merge fields so all existing single-task workflows are unaffected. When `worktree_manager` is `None`, the EffectDriver uses `self.workdir` as-is. When `Some`, it allocates a worktree per task via `create_for_plan()`. This is the zero-regression pattern: existing callers pass `None` and behavior is identical.

## Write Scope

- `crates/roko-runtime/src/effect_driver.rs`
- `crates/roko-runtime/Cargo.toml`

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

- [ ] `EffectServices` has `worktree_manager` and `merge_queue` fields
- [ ] No breaking changes to any call site constructing `EffectServices`

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: ORCH_01 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- `EffectServices` has `worktree_manager` and `merge_queue` fields
- No breaking changes to any call site constructing `EffectServices`
- No files outside the Write Scope are modified.
- Commit message contains `tracker: ORCH_01 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
