# XCUT_20: Implement Temp File Cleanup on Startup

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#xcut-20`](../ISSUE-TRACKER.md#xcut-20)
- Source: `tmp/solutions/roko/tasks/19-CROSS-CUTTING.md` — Task 19.20
- Priority: **P2**
- Effort: 3 hours
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: XCUT_20 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

Failed runs leave temporary files: partial executor snapshots, lock files, MCP config fragments, and partial JSONL entries (truncated mid-line). These accumulate in `.roko/` and can cause issues on restart (e.g., stale lock files blocking new runs, corrupt JSONL causing parse failures on cascade router load).

## Exact Changes

1. Create `roko_runtime::cleanup::startup_cleanup(roko_dir: &Path)`:
   - Remove stale `.lock` files older than 1 hour.
   - Remove `.tmp` files (partial atomic writes that never completed rename).
   - Truncate corrupt JSONL files at the last valid line boundary.
   - Remove empty directories in `.roko/state/tasks/`.
2. Call `startup_cleanup()` at the beginning of `plan run`, `serve`, and `daemon start`.
3. Log each cleanup action: `tracing::info!("cleaned up stale lock: {}", path.display())`.
4. Add `--no-cleanup` flag to skip startup cleanup for debugging.

## Design Guidance

The JSONL truncation must be careful: read the file line by line, find the last line that parses as valid JSON, and truncate the file at that point. Do not delete the file. Use atomic write (write to `.tmp`, rename) for the truncated output to avoid data loss if the truncation itself is interrupted.

## Write Scope

- `crates/roko-runtime/src/lib.rs`
- `crates/roko-cli/src/commands/plan.rs`
- `crates/roko-serve/src/lib.rs`
- `crates/roko-cli/src/daemon.rs`

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

- [ ] Stale `.lock` files are removed on startup
- [ ] Corrupt JSONL files are truncated to valid state (not deleted)
- [ ] `--no-cleanup` skips all cleanup

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: XCUT_20 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Stale `.lock` files are removed on startup
- Corrupt JSONL files are truncated to valid state (not deleted)
- `--no-cleanup` skips all cleanup
- No files outside the Write Scope are modified.
- Commit message contains `tracker: XCUT_20 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
