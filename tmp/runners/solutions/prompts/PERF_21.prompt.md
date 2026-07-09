# PERF_21: Wire WarmDispatchPool Into EffectDriver

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#perf-21`](../ISSUE-TRACKER.md#perf-21)
- Source: `tmp/solutions/roko/tasks/10-PERFORMANCE.md` — Task 10.21
- Priority: **??**
- Effort: ?
- Depends on: `PERF_20` (source 10.20)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: PERF_21 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

Add `warm_pool: Option<Arc<WarmDispatchPool>>` to `EffectServices`.
`spawn_agent()` tries pool first, falls back to cold construction.

## Exact Changes

1. Add `pub warm_pool: Option<Arc<WarmDispatchPool>>` to `EffectServices` (after
   `affect_policy` at line 49)
2. In `spawn_agent()` (line 87), before constructing the model call:
   ```rust
   let caller = if let Some(ref pool) = self.services.warm_pool {
       if let Some(guard) = pool.acquire(&provider, &model).await {
           guard.caller
       } else {
           Arc::clone(&self.services.model_caller)
       }
   } else {
       Arc::clone(&self.services.model_caller)
   };
   ```
3. After dispatch completes, release slot back to pool
4. Default `warm_pool` to `None` -- update all `EffectServices` construction
   sites to include `warm_pool: None`

## Write Scope

- `crates/roko-runtime/src/effect_driver.rs`

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

- [ ] With `warm_pool = None`: identical to current behavior
- [ ] With `warm_pool = Some(pool)`: second dispatch reuses warm slot

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: PERF_21 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- With `warm_pool = None`: identical to current behavior
- With `warm_pool = Some(pool)`: second dispatch reuses warm slot
- No files outside the Write Scope are modified.
- Commit message contains `tracker: PERF_21 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
