# XCUT_12: Add Event Bus Filtering and Subscription Topics

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#xcut-12`](../ISSUE-TRACKER.md#xcut-12)
- Source: `tmp/solutions/roko/tasks/19-CROSS-CUTTING.md` — Task 19.12
- Priority: **P6**
- Effort: 4 hours
- Depends on: `XCUT_10` (source 19.10)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: XCUT_12 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

All consumers receive all events. The TUI does not need gate compliance events. The SSE learning endpoint does not need agent output chunks. Broadcasting everything wastes CPU on serialization/deserialization for events consumers discard.

## Exact Changes

1. Add `EventTopic` enum: `Agent`, `Gate`, `Learning`, `System`, `Compliance`, `All`.
2. Add `RuntimeEvent::topic() -> EventTopic` method that classifies each variant.
3. Add `EventBus::subscribe_filtered(topics: &[EventTopic]) -> FilteredReceiver` that only delivers matching events.
4. Keep `EventBus::subscribe()` as the unfiltered path for backward compatibility.
5. Migrate SSE route to use filtered subscription (exclude `Agent` output chunks).
6. Migrate TUI bridge to use filtered subscription (exclude `Compliance` events).

## Write Scope

- `crates/roko-runtime/src/event_bus.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `crates/roko-cli/src/runner/tui_bridge.rs`

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

- [ ] Filtered subscribers only receive events matching their topic set
- [ ] Unfiltered subscribers still receive everything
- [ ] No performance regression for the unfiltered path

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: XCUT_12 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Filtered subscribers only receive events matching their topic set
- Unfiltered subscribers still receive everything
- No performance regression for the unfiltered path
- No files outside the Write Scope are modified.
- Commit message contains `tracker: XCUT_12 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
