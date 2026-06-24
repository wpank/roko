# XCUT_10: Unify DashboardEvent and RuntimeEvent Types

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#xcut-10`](../ISSUE-TRACKER.md#xcut-10)
- Source: `tmp/solutions/roko/tasks/19-CROSS-CUTTING.md` — Task 19.10
- Priority: **P2**
- Effort: 8 hours
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: XCUT_10 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

Two parallel event systems exist. `DashboardEvent` (23 files, defined in `crates/roko-core/src/dashboard_snapshot.rs` line 23) has ~15 variants driving the TUI via StateHub. `RuntimeEvent` (34 files, defined in `crates/roko-core/src/runtime_event.rs` line 56) has 12 variants driving the workflow engine's observers. They overlap on agent start/complete, gate results, and phase transitions but use different types with different field sets.

For example, agent completion is `DashboardEvent::AgentSpawned { agent_id, role, model }` vs `RuntimeEvent::AgentSpawned { run_id, agent_id, role, model }`. Gate results are `DashboardEvent::GateResult { plan_id, task_id, gate, passed }` vs `RuntimeEvent::GatePassed { run_id, gate_name, duration_ms }`. The same occurrence is emitted twice through different channels.

## Exact Changes

1. Add `impl From<RuntimeEvent> for Option<DashboardEvent>` that maps runtime events to their dashboard equivalents. Not all runtime events have dashboard counterparts (e.g., `FeedbackRecorded` has no TUI equivalent), so the conversion returns `Option`.
2. Add missing variants to `RuntimeEvent` for events that only exist in `DashboardEvent`: `EfficiencyMetric`, `Diagnosis`, `ExperimentWinnersUpdated`, `CFactorTrendUpdated`, `EpisodeRecorded`.
3. Modify `StateHub::publish()` to accept `RuntimeEvent` and auto-convert via the `From` impl.
4. Keep `DashboardEvent` as the TUI-facing type but derive it from `RuntimeEvent`.
5. Remove duplicate event emission sites where both event types are emitted for the same occurrence. Search for patterns where `state_hub.push_dashboard_event(...)` and `event_bus.emit(RuntimeEvent::...)` appear near each other for the same logical event.

## Design Guidance

This is a wide-reaching change that touches 23+ files. Implement the `From` conversion first, then gradually migrate emission sites. Use a two-phase approach: Phase 1 adds the conversion and keeps both emission paths. Phase 2 removes the `DashboardEvent` emission from sites that now emit `RuntimeEvent`. This allows incremental verification.

## Write Scope

- `crates/roko-core/src/runtime_event.rs`
- `crates/roko-core/src/dashboard_snapshot.rs`
- `crates/roko-core/src/state_hub.rs`
- `crates/roko-cli/src/runner/tui_bridge.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `crates/roko-serve/src/routes/run.rs`

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

- [ ] Single event emission point per occurrence (not two parallel emits)
- [ ] TUI still receives `DashboardEvent` via `watch` channel
- [ ] SSE/WebSocket still receive events via broadcast channel
- [ ] Event emission count does not increase (no regression in event volume)

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: XCUT_10 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Single event emission point per occurrence (not two parallel emits)
- TUI still receives `DashboardEvent` via `watch` channel
- SSE/WebSocket still receive events via broadcast channel
- Event emission count does not increase (no regression in event volume)
- No files outside the Write Scope are modified.
- Commit message contains `tracker: XCUT_10 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
