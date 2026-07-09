# RNNR_22: Harden JSONL recovery for partial writes

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#rnnr-22`](../ISSUE-TRACKER.md#rnnr-22)
- Source: `tmp/solutions/roko/tasks/14-RUNNER-PATTERNS.md` — Task 14.22
- Priority: **??**
- Effort: ?
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: RNNR_22 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

: The `recover_jsonl()` function already exists in `persist.rs`. Verify
it handles all edge cases from crash scenarios and add fsync to `append_jsonl()`.

## Exact Changes

1. Verify `recover_jsonl()` skips malformed lines with warnings (it does)
2. Add `fsync` call to `append_jsonl()` after write to minimize data loss window:
   ```rust
   file.sync_data()?;  // fsync to ensure durability
   ```
3. Add a recovery mode that detects truncated final entries (incomplete JSON)
   and logs them as warnings rather than errors
4. Ensure `atomic_write()` (already exists) uses write-to-tmp-then-rename
   pattern (verify it does -- it does at line 141)
5. Add a test: simulate crash (truncated write) and verify recovery

## Write Scope

- `crates/roko-cli/src/runner/persist.rs`

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/14-RUNNER-PATTERNS.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] Truncated JSONL lines skipped with warning, not crash
- [ ] Complete lines before truncation point successfully recovered
- [ ] `append_jsonl()` uses fsync for durability
- [ ] Test covers simulated crash recovery

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: RNNR_22 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Truncated JSONL lines skipped with warning, not crash
- Complete lines before truncation point successfully recovered
- `append_jsonl()` uses fsync for durability
- Test covers simulated crash recovery
- No files outside the Write Scope are modified.
- Commit message contains `tracker: RNNR_22 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
