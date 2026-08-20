# 41 — TUI Push-Mode Panel Data

**Priority**: P3 — Live panel data gaps degrade observability during active plan runs
**Size**: L (3-5 days)
**Crates**: `roko-cli` (`crates/roko-cli/`), `roko-core` (`crates/roko-core/`)
**Depends on**: None

---

## Background

The TUI (`roko dashboard` or embedded in `roko plan run`) has two distinct operating modes. In **pull mode**, which is used by standalone `roko dashboard` invocations, data is read from disk: episode JSONL files, executor snapshot JSON, and task-output files. Data refreshes on a polling tick. In **push mode**, which activates when the TUI is embedded inside a live `roko plan run` or connected to a running server via HTTP SSE, data arrives as `DashboardEvent` values that are applied to a `DashboardSnapshot` in memory via `DashboardSnapshot::apply`.

Push mode is the correct live mode — the TUI should show real-time data during a plan run. However, several panels show stale or empty data in push mode because the event pipeline between the runner and the TUI state is incomplete in specific places. The problems are not in event emission (the runner does emit the right events) but in how the TUI state consumes them.

The `DashboardSnapshot` struct in `roko-core` is the canonical push-mode state. The runner populates it by publishing `DashboardEvent` variants through a `StateHub` `watch::Sender`. The TUI periodically receives the latest snapshot and calls `TuiState::update_from_dashboard_snapshot` in `crates/roko-cli/src/tui/state.rs`. The pull-mode equivalent is `TuiState::update_from_snapshot`, which reads from a `DashboardData` struct assembled from disk.

## Current State

1. **`render_output_panel` in `crates/roko-cli/src/tui/views/dashboard_view.rs:277`** collects output lines from three priority sources, in order: (1) `tui_state.current_plan_execution.agent_output_tail`, (2) `tui_state.agents[selected].output_lines`, (3) `tui_state.task_output_tails` keyed by the current task id. Sources 1 and 2 are populated by `update_from_snapshot` (pull mode). Source 3 is populated by `update_from_dashboard_snapshot` at line 2482. Because source 1 takes priority and pull-mode episode reads can produce non-empty output, live push-mode data from source 3 is never reached during an active run.

2. **`TuiBridge::agent_output` in `crates/roko-cli/src/runner/tui_bridge.rs:94`** (the method, not a separate `emit_agent_output` name) publishes `DashboardEvent::AgentOutput`. The snapshot's `apply` method at `crates/roko-core/src/dashboard_snapshot.rs:1239` records incoming chunks in `task_outputs`, a `HashMap<String, VecDeque<String>>` keyed by `task_id`. `update_from_dashboard_snapshot` at `state.rs:2482` copies `snap.task_outputs` into `self.task_output_tails`. The data is available; the output panel just never reaches it due to priority ordering.

3. **`TuiBridge::token_usage` exists at `crates/roko-cli/src/runner/tui_bridge.rs:155`** and publishes all four token counters as `EfficiencyEvent` variants. However, searching `event_loop.rs` shows this method is not called from the `AgentEvent::TokenUsage` handler (line 6760). Token counts update at gate completion time via indirect paths, not per-turn.

4. **`DashboardSnapshot.agent_topology` in `crates/roko-core/src/dashboard_snapshot.rs:932`** is an `AgentTopology { nodes, edges, timestamp }` struct. `update_from_dashboard_snapshot` at `state.rs:2359` copies it into `self.agent_topology` only when `snap.agent_topology.is_empty()` is false. No code in the runner, server, or ACP path ever populates `snap.agent_topology.nodes` with real data; it is always empty. There is no `DashboardEvent` variant that carries topology data.

5. **`render_diagnosis_panel` in `crates/roko-cli/src/tui/views/dashboard_view.rs:1034`** shows "no conductor diagnoses yet" (line 1065) when the diagnoses list is empty. The wiring is complete (`DashboardEvent::Diagnosis` → snapshot → `update_from_dashboard_snapshot` → `self.diagnoses`), but the empty state message gives no context about when diagnoses appear. Diagnoses are only emitted by the conductor's circuit-breaker path, which requires a sustained failure pattern; the panel is correctly empty during normal runs.

6. **`PlanExecutionSnapshot.agent_output_tail` is defined in `crates/roko-cli/src/tui/dashboard.rs:1293`** and populated during pull-mode disk reads via `backfill_agent_output_tail`. It is not populated by `update_from_dashboard_snapshot`, so once pull-mode data exists in this field, it prevents push-mode data from being shown.

## Implementation Plan

### Step 1: Fix output panel priority for active plans

In `crates/roko-cli/src/tui/views/dashboard_view.rs`, modify `render_output_panel` (starting at line 277). The current priority order puts `current_plan_execution.agent_output_tail` (pull-mode) first. When a plan is actively running (i.e., `tui_state.current_plan_execution` is Some and the active task is present in `tui_state.task_output_tails`), the live push-mode tail from `task_output_tails` should take priority.

The change: before falling through to source 1, check if `tui_state.task_output_tails` contains an entry for the currently active task id. If it does and it is non-empty, return those lines. The active task id is available from `tui_state.current_plan_execution.as_ref().map(|e| &e.current_task_id)` or from the active plan's task list.

The same priority fix should be applied to the agents view output panel in `crates/roko-cli/src/tui/views/agents_view.rs:972` which has identical logic.

Estimated diff: ~25 lines changed in two files.

### Step 2: Emit per-turn token events from the runner

In `crates/roko-cli/src/runner/event_loop.rs`, find the `AgentEvent::TokenUsage` handler at line 6760. After updating the JSON debug event, call `tui.token_usage(plan_id, task_id, input_tokens, output_tokens, cache_read_tokens, cache_write_tokens)`. The `tui` binding is the `TuiBridge` instance available as a local in the enclosing function.

The `tui.token_usage` method in `tui_bridge.rs:155` already publishes four `EfficiencyEvent` variants. These are handled by `DashboardSnapshot::apply` at `dashboard_snapshot.rs:1298`, which accumulates them into `AgentState.input_tokens` and `AgentState.output_tokens`. The TUI reads these from `update_from_dashboard_snapshot` at `state.rs:2244`.

Estimated diff: ~5 lines in `event_loop.rs`.

### Step 3: Add `AgentTopologyUpdated` event and topology derivation

In `crates/roko-core/src/dashboard_snapshot.rs`, add a new variant to `DashboardEvent`:

```rust
AgentTopologyUpdated { topology: AgentTopology },
```

In `DashboardSnapshot::apply` (line 1073), handle this variant by replacing `self.agent_topology`:

```rust
DashboardEvent::AgentTopologyUpdated { topology } => {
    self.agent_topology = topology.clone();
}
```

In `TuiState::update_from_dashboard_snapshot` (state.rs line 2359), when `snap.agent_topology.is_empty()` is true but `snap.agents` is non-empty, derive a minimal topology: each agent becomes one `AgentTopologyNode`, and agents sharing a `plan_id` get edges between them. Emit this derived topology into `self.agent_topology`.

Alternatively (simpler), add a periodic tick in the runner after `AgentSpawned` events that calls a helper to build the `AgentTopology` from `snap.agents` and publishes `AgentTopologyUpdated`.

Estimated diff: ~80 lines across two files.

### Step 4: Update diagnosis panel empty-state text

In `crates/roko-cli/src/tui/views/dashboard_view.rs` at line 1065, replace the empty-state string:

```rust
// Before:
Paragraph::new("no conductor diagnoses yet").style(theme.muted()),

// After:
Paragraph::new(
    "No diagnoses — the conductor circuit breaker fires only when \
     a sustained gate-failure pattern is detected across multiple tasks.",
)
.style(theme.muted()),
```

Estimated diff: ~5 lines.

## Acceptance Criteria

1. Run `roko plan run <plandir> --engine runner-v2` with a multi-task plan. While an agent is streaming, the Output panel on the Dashboard tab shows live text that updates during streaming, not after gate completion.
2. The agent token counts in the Agents tab header and sparkline update at least once per LLM turn (after each `AgentEvent::TokenUsage`), not only at gate completion time.
3. `grep 'AgentTopologyUpdated' crates/roko-core/src/dashboard_snapshot.rs` returns at least one match.
4. After two `AgentSpawned` events are applied to a `DashboardSnapshot`, `snap.agent_topology.nodes` is non-empty (verifiable by unit test).
5. `cargo test --workspace` passes with zero failures after each change.
6. `cargo clippy --workspace --no-deps -- -D warnings` is clean.

## Verification Checklist

- [ ] Run `roko plan run` and open the dashboard; confirm Output panel shows streaming text before gate fires
- [ ] Watch the Agents tab token counter during a run; confirm it updates per turn, not just at task completion
- [ ] Add a unit test for `DashboardSnapshot::apply` with an `AgentTopologyUpdated` event
- [ ] Add a unit test that verifies topology derivation from `snap.agents` when `agent_topology` is empty
- [ ] Verify diagnosis panel shows the updated placeholder text when no diagnoses exist
- [ ] `cargo test -p roko-cli -p roko-core` passes
- [ ] `cargo clippy --workspace --no-deps -- -D warnings` is clean

## Files to Modify

| File | Change |
|---|---|
| `crates/roko-cli/src/tui/views/dashboard_view.rs` | Flip output panel priority at line 277 to prefer `task_output_tails` when active task is present; update empty-state text in `render_diagnosis_panel` at line 1065 |
| `crates/roko-cli/src/tui/views/agents_view.rs` | Same priority flip for output lines at line 972 |
| `crates/roko-cli/src/runner/event_loop.rs` | Call `tui.token_usage(...)` from the `AgentEvent::TokenUsage` handler at line 6760 |
| `crates/roko-core/src/dashboard_snapshot.rs` | Add `AgentTopologyUpdated` variant to `DashboardEvent`; handle in `DashboardSnapshot::apply` |
| `crates/roko-cli/src/tui/state.rs` | Derive topology in `update_from_dashboard_snapshot` when `snap.agent_topology.is_empty()` and `snap.agents` is non-empty |
