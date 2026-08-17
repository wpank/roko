# XCUT_19: Implement Worktree Cleanup Policy

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#xcut-19`](../ISSUE-TRACKER.md#xcut-19)
- Source: `tmp/solutions/roko/tasks/19-CROSS-CUTTING.md` — Task 19.19
- Priority: **P4**
- Effort: 4 hours
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: XCUT_19 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

Per MEMORY.md and project rules, worktrees are never deleted automatically. The glob shows 20+ worktrees under `.claude/worktrees/` and `.roko/worktrees/`. There is no visibility into worktree age, size, or relationship to completed plans. `WorktreeManager` at `crates/roko-orchestrator/src/worktree.rs` has `create_for_plan()`, `remove()`, `touch()`, `reclaim_idle()`, `health()`, `clear_stale_locks()`, `prune()` but no status reporting.

**CRITICAL**: Never auto-delete worktrees. This is a hard project rule. The GC command must always prompt for confirmation.

## Exact Changes

1. Add `WorktreeManager::status() -> Vec<WorktreeInfo>` that reports: path, branch, age, disk size, plan association, clean/dirty.
2. Add `roko util worktrees list` subcommand that displays the status table.
3. Add `roko util worktrees gc --older-than 30d --dry-run` that identifies candidates for cleanup.
4. Without `--dry-run`, prompt for confirmation (never auto-delete).
5. Add `roko util worktrees archive <path>` that creates a tarball of the worktree before removal.
6. Track worktree creation time in `.roko/state/worktrees.json`.

## Write Scope

- `crates/roko-orchestrator/src/worktree.rs`
- `crates/roko-cli/src/main.rs`

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/19-CROSS-CUTTING.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] `roko util worktrees list` shows all worktrees with age and size
- [ ] `roko util worktrees gc --older-than 30d --dry-run` lists candidates without deleting
- [ ] Actual deletion requires explicit user confirmation (never auto-delete)
- [ ] Worktree registry persists across restarts

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: XCUT_19 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- `roko util worktrees list` shows all worktrees with age and size
- `roko util worktrees gc --older-than 30d --dry-run` lists candidates without deleting
- Actual deletion requires explicit user confirmation (never auto-delete)
- Worktree registry persists across restarts
- No files outside the Write Scope are modified.
- Commit message contains `tracker: XCUT_19 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
