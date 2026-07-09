# PERF_12: Parallelize Enrichment Pipeline

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#perf-12`](../ISSUE-TRACKER.md#perf-12)
- Source: `tmp/solutions/roko/tasks/10-PERFORMANCE.md` — Task 10.12
- Priority: **??**
- Effort: ?
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: PERF_12 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

Run independent enrichment steps concurrently using `tokio::join!`
instead of the sequential `ALL_ORDERED` iteration.

## Exact Changes

1. Locate the `EnrichmentPipeline::new()` usage at orchestrate.rs line 8888 and
   the `selected_enrichment_steps()` call at line 1829
2. Identify which steps are independent (file intel, knowledge, wave, research
   are all independent; tasks step may depend on file intel)
3. Group independent steps and run them with `tokio::join!` or
   `futures::future::join_all()`
4. Steps that are CPU-bound (not I/O-bound): wrap in
   `tokio::task::spawn_blocking()`
5. Handle errors individually -- each enrichment step's failure should not abort
   the others; collect results and report which steps failed
6. Preserve the existing `StepSelector` complexity-based filtering

## Write Scope

- `crates/roko-cli/src/orchestrate.rs`

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

- [ ] Tracing shows enrichment time = `max(step_times)` not `sum(step_times)`
- [ ] Enriched prompt content is unchanged (diff before/after parallelization)

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: PERF_12 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Tracing shows enrichment time = `max(step_times)` not `sum(step_times)`
- Enriched prompt content is unchanged (diff before/after parallelization)
- No files outside the Write Scope are modified.
- Commit message contains `tracker: PERF_12 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
