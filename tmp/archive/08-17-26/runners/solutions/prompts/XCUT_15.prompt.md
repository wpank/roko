# XCUT_15: Wire GracefulShutdown into roko serve

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#xcut-15`](../ISSUE-TRACKER.md#xcut-15)
- Source: `tmp/solutions/roko/tasks/19-CROSS-CUTTING.md` — Task 19.15
- Priority: **P1**
- Effort: 4 hours
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: XCUT_15 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

`GracefulShutdown` in `crates/roko-core/src/shutdown.rs` is fully implemented: hook registration, concurrent drain with `join_all`, hard deadline, `ShutdownReport` with drained/timed-out counts. But it is imported by only 1 file in the codebase. `roko-serve` in `crates/roko-serve/src/lib.rs` uses ad-hoc `force_shutdown` signaling (found in `daemon.rs`, `agent_serve.rs`). WebSocket connections in `crates/roko-serve/src/routes/` are dropped without drain. SSE streams cut off mid-event.

## Exact Changes

1. Add `GracefulShutdown` to `AppState` in `crates/roko-serve/src/state.rs` alongside the existing `CancelToken`.
2. Create `GracefulShutdown::with_deadline(Duration::from_secs(5))` during server startup.
3. Register shutdown hooks for each subsystem:
   - `"ws-drain"`: send `subscription_ended` to all WebSocket clients, wait up to 1s.
   - `"sse-drain"`: send final SSE event to all clients, close connections.
   - `"state-flush"`: flush StateHub snapshot to disk.
   - `"metrics-flush"`: flush any buffered OTel/metrics data.
4. On SIGTERM/SIGINT, call `shutdown.drain().await` instead of the current `force_shutdown` flag.
5. Log the `ShutdownReport` (drained_hooks, timed_out_hooks, elapsed_ms).

## Write Scope

- `crates/roko-serve/src/lib.rs`
- `crates/roko-serve/src/state.rs`

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

- [ ] `kill -TERM <roko-serve-pid>` produces a `ShutdownReport` in the log
- [ ] WebSocket clients receive `subscription_ended` before disconnection
- [ ] SSE streams receive a final event before closing
- [ ] Shutdown completes within the 5-second deadline
- [ ] No orphan background tasks after shutdown

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: XCUT_15 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- `kill -TERM <roko-serve-pid>` produces a `ShutdownReport` in the log
- WebSocket clients receive `subscription_ended` before disconnection
- SSE streams receive a final event before closing
- Shutdown completes within the 5-second deadline
- No orphan background tasks after shutdown
- No files outside the Write Scope are modified.
- Commit message contains `tracker: XCUT_15 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
