# XCUT_22: Periodic Learning File Compaction

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#xcut-22`](../ISSUE-TRACKER.md#xcut-22)
- Source: `tmp/solutions/roko/tasks/19-CROSS-CUTTING.md` — Task 19.22
- Priority: **P6**
- Effort: 4 hours
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: XCUT_22 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

Append-only JSONL files grow without bound: `episodes.jsonl`, `efficiency.jsonl`, `costs.jsonl`, `routing.jsonl`. After weeks of use, these files can reach hundreds of MB. The cascade router loads and parses the full `cascade-router.json` on startup, slowing initialization. No compaction or retention policy exists.

## Exact Changes

1. Add `compact_episodes(path: &Path, retention_days: u32)` that:
   - Reads the JSONL file line by line.
   - Removes entries older than `retention_days`.
   - Computes aggregate statistics for removed entries (total tokens, total cost, pass rate by model).
   - Writes a summary entry and the retained entries to a `.tmp` file.
   - Atomic rename to replace the original.
2. Add `compact_cascade_router(path: &Path)` that prunes observations older than the confidence window.
3. Add `roko learn compact --retention-days 30` subcommand.
4. Add optional auto-compaction on startup when file exceeds 50MB (configurable via `[learning] max_file_mb = 50`).
5. Always preserve the aggregate summary so historical trends are not lost.

## Write Scope

- `crates/roko-learn/src/lib.rs`
- `crates/roko-cli/src/commands/learn.rs`

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

- [ ] `roko learn compact --retention-days 30` reduces file size by removing old entries
- [ ] Aggregate statistics are preserved in a summary entry
- [ ] CascadeRouter observations are pruned without losing learned routing weights

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: XCUT_22 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- `roko learn compact --retention-days 30` reduces file size by removing old entries
- Aggregate statistics are preserved in a summary entry
- CascadeRouter observations are pruned without losing learned routing weights
- No files outside the Write Scope are modified.
- Commit message contains `tracker: XCUT_22 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
