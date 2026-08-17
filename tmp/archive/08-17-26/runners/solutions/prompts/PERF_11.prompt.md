# PERF_11: Routing Decision Cache

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#perf-11`](../ISSUE-TRACKER.md#perf-11)
- Source: `tmp/solutions/roko/tasks/10-PERFORMANCE.md` — Task 10.11
- Priority: **??**
- Effort: ?
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: PERF_11 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

The `EfficiencyCache` at line 2445 already caches raw efficiency
signals with 10s TTL. Extend with routing decision memoization so sequential
plan tasks skip re-scoring all model candidates.

## Exact Changes

1. Add a `routing_decisions: HashMap<u64, (String, Instant)>` field to
   `PlanRunner` (or adjacent to `EfficiencyCache`)
2. Compute routing cache key as `hash(task_type, complexity_tier, (recent_quality * 100) as u32)`
3. Before the cascade routing logic (around line 5971 and 14663 where
   `efficiency_cache.get()` is called): check if a routing decision exists for
   the computed key within TTL
4. On cache hit: use cached model, skip scoring loop
5. On cache miss: run normal cascade routing, store decision
6. Invalidate on new efficiency event write (set cached_at to the past)
7. TTL: 30 seconds (shorter than efficiency cache because routing decisions
   incorporate external state like neuro store queries)

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

- [ ] 5-task plan: `efficiency.jsonl` read at most twice (initial + one TTL expiry)
- [ ] Routing decisions for identical task profiles are consistent within TTL window

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: PERF_11 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- 5-task plan: `efficiency.jsonl` read at most twice (initial + one TTL expiry)
- Routing decisions for identical task profiles are consistent within TTL window
- No files outside the Write Scope are modified.
- Commit message contains `tracker: PERF_11 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
