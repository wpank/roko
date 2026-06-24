# XCUT_11: Add Event Bus Backpressure and Overflow Handling

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#xcut-11`](../ISSUE-TRACKER.md#xcut-11)
- Source: `tmp/solutions/roko/tasks/19-CROSS-CUTTING.md` — Task 19.11
- Priority: **P6**
- Effort: 3 hours
- Depends on: `XCUT_10` (source 19.10)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: XCUT_11 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

`EventBus` in `crates/roko-runtime/src/event_bus.rs` uses `tokio::sync::broadcast` which silently drops events when consumers lag. The bus has a bounded `VecDeque` ring (replay buffer) but the broadcast channel capacity is not configurable. During fast multi-agent runs, TUI or SSE clients can miss events with no indication. The `Envelope<E>` wrapper at line 63 includes `seq` for gap detection but no overflow tracking.

## Exact Changes

1. In `EventBus`, add an overflow counter: `Arc<AtomicU64>` tracking total dropped events.
2. Add `EventBus::overflow_count() -> u64` public method.
3. In `StateHub`, log a warning when overflow count increases: `tracing::warn!(overflow = count, "event bus overflow: {count} events dropped")`.
4. Add a `DashboardEvent::Overflow { dropped_count }` variant so the TUI can display a "missed N events" indicator.
5. Increase the default broadcast channel capacity to 2048 for multi-agent runs.
6. Add `[dashboard] event_buffer_size = 2048` config option.

## Write Scope

- `crates/roko-runtime/src/event_bus.rs`
- `crates/roko-core/src/state_hub.rs`
- `crates/roko-core/src/dashboard_snapshot.rs`

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

- [ ] Overflow events are tracked and logged
- [ ] TUI displays "N events missed" when overflow occurs
- [ ] Default capacity handles 10 concurrent agents at 10 events/second without overflow

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: XCUT_11 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Overflow events are tracked and logged
- TUI displays "N events missed" when overflow occurs
- Default capacity handles 10 concurrent agents at 10 events/second without overflow
- No files outside the Write Scope are modified.
- Commit message contains `tracker: XCUT_11 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
