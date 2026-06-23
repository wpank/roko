# PERF_40: Parallel Inference for Independent Plan Tasks

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#perf-40`](../ISSUE-TRACKER.md#perf-40)
- Source: `tmp/solutions/roko/tasks/10-PERFORMANCE.md` — Task 10.40
- Priority: **??**
- Effort: ?
- Depends on: `PERF_23` (source 10.23)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: PERF_40 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

Ensure DAG executor dispatches independent tasks concurrently, using
the warm pool for connection reuse.

## Exact Changes

1. Verify existing DAG executor identifies independent tasks (no unmet deps)
2. If independent tasks dispatched sequentially: change to `tokio::spawn` per
   ready task + `futures::future::join_all()`
3. Limit concurrency to `config.conductor.max_concurrent_tasks` (default 3)
4. Each concurrent task acquires from warm pool independently
5. Verify concurrent substrate writes don't corrupt data (mutex in FileSubstrate)

## Write Scope

- `crates/roko-cli/src/orchestrate.rs`
- `crates/roko-orchestrator/src/dag.rs`

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

- [ ] 3 independent tasks dispatch concurrently (trace log)
- [ ] Wall clock < sum of individual task times
- [ ] No data corruption from concurrent substrate writes

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: PERF_40 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- 3 independent tasks dispatch concurrently (trace log)
- Wall clock < sum of individual task times
- No data corruption from concurrent substrate writes
- No files outside the Write Scope are modified.
- Commit message contains `tracker: PERF_40 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
