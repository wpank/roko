# 126 — Error Digest Widget (Cross-Source Error Aggregation Panel)

**Priority**: P2 — Gate failures, preflight warnings, and runtime errors currently appear in multiple places across different tabs; there is no single location to see all active errors without switching tabs.
**Size**: S (1 day)
**Crates**: `crates/roko-cli/src/tui/tabs.rs`, `crates/roko-cli/src/tui/`
**Depends on**: #121 (TUI data model unification — errors need a single source)
**Sources**: `tmp/backlog/_checklist-gaps.md` §2.6, `tmp/backlog/_mori-old-gaps.md` MO-11

---

## Background

In Mori's TUI, the F2:plans right panel included an error digest: a scrollable list of all current errors from any source — gate failures, compile errors, agent stalls, preflight warnings, runtime panics — aggregated into one panel. The panel's border turned red when any error was active, providing a visual signal visible from any tab (since the border color was part of the panel's style, not the tab-specific rendering).

Roko's TUI shows errors per-plan (on the F2 plan detail view) and per-gate (on a dedicated gate view), but no aggregated digest. An operator must switch to F2, select the failing plan, and navigate to the gate output to see compile errors. This adds friction during debugging.

The underlying data already exists: gate failure outputs, preflight check results, and agent error events are all tracked in `TuiModel` (or will be after #121). The widget is a rendering concern, not a data concern.

## Current State

- Gate failure output: available in plan/task runner events and stored per-task.
- Preflight check results: available from `PlanRunPreflight` (new in #120).
- Agent stall detection: available from conductor watcher events (if wired, per future work).
- No aggregated error list widget exists anywhere in the TUI.
- Error classification (from #114 `diagnose.rs`): `ClassifiedError` structs provide structured error data.

## Implementation Plan

1. **`ErrorDigestWidget` struct** in `crates/roko-cli/src/tui/error_digest.rs`:
   ```rust
   pub struct ErrorEntry {
       pub source: ErrorSource,  // GateFailure, PlanPreflight, AgentStall, RuntimeError
       pub plan_id: Option<String>,
       pub task_id: Option<String>,
       pub error_class: ErrorClass,
       pub summary: String,  // one-line human-readable
       pub timestamp: DateTime<Utc>,
       pub resolved: bool,   // true if the plan/task has since succeeded
   }
   ```

2. **Populate `TuiModel.error_digest: Vec<ErrorEntry>`**:
   - On each `RunnerEvent::GateFailed`, push a `GateFailure` entry.
   - On each `PreflightCheck { status: Fail|Warn }`, push a `PlanPreflight` entry.
   - On plan completion (success), mark all entries for that plan as `resolved = true`.
   - Keep the last 50 entries (ring buffer or VecDeque with max size).

3. **Widget rendering**: Show in the bottom section of the F2:plans right panel (or as a sub-panel in F1:dashboard). Each entry: `[source] [plan-id] error_summary (timestamp)`. Resolved entries shown in muted color; active errors in EMBER red.

4. **Panel border color**: When `error_digest.iter().any(|e| !e.resolved)`, set the F2 right panel border to `theme::EMBER`. When all errors are resolved, use the default `theme::BORDER`.

5. **Error count in tab badge**: Feed unresolved error count to the tab badge system (#130): `e:Errors(N)` on the F2 tab label.

6. **Scroll support**: The error digest panel should be scrollable with `j`/`k` when focused. The most recent error is shown at the top.

## Acceptance Criteria

1. After a gate failure, at least one entry appears in the error digest panel on F2.
2. The F2 panel border turns red (EMBER) when there is at least one unresolved error.
3. After the failing plan succeeds or is reset, the entry is marked resolved and the border returns to default color.
4. Preflight warnings from #120 appear in the digest.
5. The last 50 errors are retained; older entries are dropped.
6. Errors are scrollable with `j`/`k` keys.

## Verification Checklist

- [ ] Trigger a compile gate failure; verify the error appears in the digest panel.
- [ ] Verify the panel border turns EMBER red on gate failure.
- [ ] After the task succeeds on retry, verify the entry is marked resolved and border returns to default.
- [ ] With 60 errors accumulated, verify only the 50 most recent are shown.
- [ ] Verify preflight failure (from #120) appears in the digest on run startup.

## Files to Modify

| File | Change |
|---|---|
| `crates/roko-cli/src/tui/error_digest.rs` | New file: `ErrorEntry`, `ErrorDigestWidget` |
| `crates/roko-cli/src/tui/mod.rs` | Export `error_digest` module |
| `crates/roko-cli/src/tui/app.rs` | Add `error_digest: VecDeque<ErrorEntry>` to `TuiModel` |
| `crates/roko-cli/src/runner/event_loop.rs` | Push error entries on gate failure events |
| `crates/roko-cli/src/tui/tabs.rs` | Embed `ErrorDigestWidget` in F2 right panel |
