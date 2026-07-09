# RNNR_20: Implement per-task result file tracking for manual intervention

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#rnnr-20`](../ISSUE-TRACKER.md#rnnr-20)
- Source: `tmp/solutions/roko/tasks/14-RUNNER-PATTERNS.md` — Task 14.20
- Priority: **??**
- Effort: ?
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: RNNR_20 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

: Write per-task `.result` files to disk as a coordination mechanism.
The mega-parity runner proved that simple files on disk enable manual
intervention: mark a task as success, skip it, or force a retry.

Note: `persist.rs` already has `atomic_write()`, `append_jsonl()`, and
`recover_jsonl()`. This task extends that with per-task result files.

## Exact Changes

1. Define result file location: `.roko/state/runs/{run_id}/{task_id}.result`
2. Write result file on each task status transition:
   ```json
   {"status": "success", "elapsed_ms": 12345, "commit": "abc123", "files_changed": 3}
   ```
3. Valid statuses: `in_progress`, `success`, `failed`, `blocked`, `skipped`
4. On `--resume-plan`, read all `.result` files and reconstruct task states
5. Support manual override: if a human writes `success` to a `.result` file,
   the scheduler treats that task as completed and unblocks dependents
6. Write result files atomically (use existing `atomic_write()`)

## Write Scope

- `crates/roko-cli/src/runner/persist.rs`
- `crates/roko-cli/src/runner/event_loop.rs`

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

- [ ] Each task produces a `.result` file at its designated path
- [ ] `--resume-plan` reads result files and skips completed tasks
- [ ] Manually writing `success` to a result file unblocks dependents on resume
- [ ] Result files written atomically

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: RNNR_20 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Each task produces a `.result` file at its designated path
- `--resume-plan` reads result files and skips completed tasks
- Manually writing `success` to a result file unblocks dependents on resume
- Result files written atomically
- No files outside the Write Scope are modified.
- Commit message contains `tracker: RNNR_20 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
