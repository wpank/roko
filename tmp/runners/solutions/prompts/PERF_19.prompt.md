# PERF_19: Create WarmDispatchPool

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#perf-19`](../ISSUE-TRACKER.md#perf-19)
- Source: `tmp/solutions/roko/tasks/10-PERFORMANCE.md` — Task 10.19
- Priority: **??**
- Effort: ?
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: PERF_19 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

Three-tier warm dispatch pool: hot (in-flight), warm (pre-built idle),
cold (on-demand construct). RAII slot guards. Pool metrics.

## Exact Changes

1. Create `warm_dispatch_pool.rs` with:
   - `WarmPoolConfig`: `max_warm_slots`, `max_active`, `idle_timeout`,
     `pre_warm`, `pre_warm_targets`
   - `WarmSlot`: `provider`, `model`, `caller: Arc<dyn ModelCaller>`,
     `created_at`, `last_used`, `dispatches_served`, `state: SlotState`
   - `SlotState`: `Idle`, `Active { run_id, since }`, `Draining`
   - `WarmPoolMetrics`: `total_dispatches`, `warm_hits`, `cold_misses`,
     `evictions`, `peak_active`, `avg_acquire_us`
   - `WarmDispatchPool`: `config`, `slots: Mutex<Vec<WarmSlot>>`,
     `metrics: Mutex<WarmPoolMetrics>`, `factory`
   - `WarmSlotGuard<'a>`: `pool`, `slot_idx`, `caller`
2. `acquire()`: tier 1 (exact match) -> tier 2 (same provider) -> tier 3 (cold)
3. `pre_warm()`: create idle slots for configured targets
4. `evict_idle()`: remove slots past `idle_timeout`
5. `release()`: return slot to idle state
6. Document: `Drop` for `WarmSlotGuard` cannot be async; callers must call
   `pool.release(idx)` explicitly

## Write Scope

- `crates/roko-runtime/src/warm_dispatch_pool.rs`

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

- [ ] Unit test: acquire from empty -> cold miss, slot created
- [ ] Unit test: acquire, release, acquire again -> warm hit
- [ ] Unit test: same provider different model -> warm hit (provider reuse)
- [ ] Unit test: evict_idle removes expired slots
- [ ] Unit test: metrics track hits/misses/evictions accurately

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: PERF_19 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Unit test: acquire from empty -> cold miss, slot created
- Unit test: acquire, release, acquire again -> warm hit
- Unit test: same provider different model -> warm hit (provider reuse)
- Unit test: evict_idle removes expired slots
- Unit test: metrics track hits/misses/evictions accurately
- No files outside the Write Scope are modified.
- Commit message contains `tracker: PERF_19 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
