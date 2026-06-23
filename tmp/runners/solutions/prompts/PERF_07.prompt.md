# PERF_07: Batch Substrate Writes

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#perf-07`](../ISSUE-TRACKER.md#perf-07)
- Source: `tmp/solutions/roko/tasks/10-PERFORMANCE.md` — Task 10.7
- Priority: **??**
- Effort: ?
- Depends on: `PERF_01` (source 10.1)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: PERF_07 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

The `put_batch()` method already exists on `FileSubstrate` (line 137).
The task is to ensure the CLI `run` path uses it instead of sequential `put()`
calls, and to add crash-safety tests.

## Exact Changes

1. Verify `put_batch()` at `file_substrate.rs` line 137 does single-write I/O
   (it currently collects lines into a String and does one write)
2. In `run.rs`, identify the post-dispatch signal persistence (around line
   924-1050 per the bottleneck analysis) and replace individual `substrate.put()`
   calls with `substrate.put_batch(signals)`
3. Add a crash-safety test: write valid signals then a truncated last line,
   verify the reader returns only complete lines
4. If `put_batch` already coalesces I/O, confirm with tracing that a single
   `substrate_write` span covers all signals

## Write Scope

- `crates/roko-fs/src/file_substrate.rs`
- `crates/roko-cli/src/run.rs`

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

- [ ] Test: 10 signals via `put_batch()`, read back, all 10 present
- [ ] Test: partial last line is ignored by reader
- [ ] Tracing shows single `substrate_write` span for batch

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: PERF_07 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Test: 10 signals via `put_batch()`, read back, all 10 present
- Test: partial last line is ignored by reader
- Tracing shows single `substrate_write` span for batch
- No files outside the Write Scope are modified.
- Commit message contains `tracker: PERF_07 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
