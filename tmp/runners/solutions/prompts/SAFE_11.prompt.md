# SAFE_11: Worktree Path Isolation Enforcement

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#safe-11`](../ISSUE-TRACKER.md#safe-11)
- Source: `tmp/solutions/roko/tasks/17-SAFETY-SECURITY.md` — Task 17.11
- Priority: **P2**
- Effort: ?
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: SAFE_11 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

: `PathPolicy` in `path.rs` has complete worktree escape detection
including symlink traversal blocking. Wire it into the Claude CLI settings
hooks to prevent agents from accessing files outside their assigned worktree.

## Exact Changes

1. When spawning a Claude CLI agent in a worktree, set `current_dir` to the
   worktree root (this may already happen)
2. Add `PreToolUse` hooks to `build_settings_json()` that block:
   - `Bash(cd /*)` targeting paths outside the worktree
   - `Bash(cat /etc/*)`, `Bash(cat ~/.*)` and similar escape patterns
   - `Bash(ln -s /*)` creating symlinks to outside paths
3. The `--allowed-directory` flag on Claude CLI (if available) should be set
   to the worktree root
4. Log path violations at `tracing::warn!` level

## Write Scope

- `crates/roko-agent/src/claude_cli_agent.rs`

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/17-SAFETY-SECURITY.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] An agent in a worktree cannot `cat /etc/passwd` via bash
- [ ] An agent cannot create a symlink pointing outside the worktree
- [ ] Legitimate file operations within the worktree are unaffected
- [ ] Path blocks appear in the settings JSON hooks

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: SAFE_11 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- An agent in a worktree cannot `cat /etc/passwd` via bash
- An agent cannot create a symlink pointing outside the worktree
- Legitimate file operations within the worktree are unaffected
- Path blocks appear in the settings JSON hooks
- No files outside the Write Scope are modified.
- Commit message contains `tracker: SAFE_11 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
