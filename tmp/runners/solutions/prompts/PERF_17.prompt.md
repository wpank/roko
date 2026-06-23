# PERF_17: Source Hash Gate Guard

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#perf-17`](../ISSUE-TRACKER.md#perf-17)
- Source: `tmp/solutions/roko/tasks/10-PERFORMANCE.md` — Task 10.17
- Priority: **??**
- Effort: ?
- Depends on: `PERF_16` (source 10.16)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: PERF_17 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

Skip the compile gate if modified source files have not changed since
the last successful compile (hash-based guard).

## Exact Changes

1. Add `fn hash_modified_sources(modified_files: &[String]) -> u64` that hashes
   concatenated mtimes + sizes of all `.rs` files using `DefaultHasher`
2. Store last successful compile hash in `.roko/state/last-compile-hash` (single
   line file with hex-encoded u64)
3. In `run_gates()`, before running the compile rung:
   - Compute current hash from modified files list
   - If matches stored hash: skip compile, log "compile skipped (unchanged)"
   - If differs: run compile, update stored hash on pass
4. Never skip if last compile failed (always re-check after failure)

## Write Scope

- `crates/roko-gate/src/gate_service.rs`

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

- [ ] Two runs on unchanged codebase: second skips compile gate
- [ ] Modify a `.rs` file: compile gate runs again

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: PERF_17 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Two runs on unchanged codebase: second skips compile gate
- Modify a `.rs` file: compile gate runs again
- No files outside the Write Scope are modified.
- Commit message contains `tracker: PERF_17 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
