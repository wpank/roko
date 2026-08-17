# TUI Push-Mode Panel Data

**Priority**: P3
**Size**: L (3-5 days)

---

## Problem

The TUI has two operating modes:

- **Pull mode** (standalone `roko dashboard`): reads state from disk — episode JSONL,
  executor snapshot JSON, task-output files. Data is refreshed on a polling tick.
- **Push mode** (embedded in `roko plan run` or connected via HTTP SSE): receives
  `DashboardEvent` values published through the `StateHub` `watch::Sender` and applies
  them to a `DashboardSnapshot` via `DashboardSnapshot::apply`.

Push mode is the intended live mode. Several panels are empty or show stale data when
the TUI runs in push mode because the event pipeline between the runner and the
snapshot is incomplete in four places.

### Gap 1: Output panel shows no live agent text in push mode

`render_output_panel` in `tui/views/dashboard_view.rs:277` collects lines from three
sources in priority order:

1. `current_plan_execution.agent_output_tail` (populated by pull-mode episode reads)
2. `tui_state.agents[selected].output_lines` (populated by pull-mode episode reads)
3. `tui_state.task_output_tails` (populated by pull-mode task-output file cursor)

All three sources are populated by `update_from_snapshot` in
`state.rs:update_from_snapshot` and `update_from_dashboard_data`, both of which read
from JSONL and file cursors. In push mode, `update_from_dashboard_snapshot` is called
instead, and it does not populate `agent.output_lines` — it only updates
`agent.output_bytes` and `agent.last_event_at_ms` from `DashboardEvent::AgentOutput`.

The runner does emit `DashboardEvent::AgentOutput` (via `TuiBridge::emit_agent_output`
in `runner/tui_bridge.rs:102`) with the agent's streamed text. The `DashboardSnapshot`
records this in `task_outputs` (a ring-buffered `HashMap<String, VecDeque<String>>`
keyed by `task_id`). But `update_from_dashboard_snapshot` in `state.rs:2052` reads
`snap.task_outputs` into `self.task_output_tails` — which is source #3 above,
the lowest priority. If sources #1 and #2 are non-empty (as they would be after any
prior pull-mode tick), source #3 is never reached.

The result: live text from the runner is available in the snapshot but the Output panel
shows stale episode text instead of the live stream.

### Gap 2: Diagnosis panel is empty in push mode

`render_diagnosis_panel` in `dashboard_view.rs:1034` renders from
`tui_state.diagnoses`. `update_from_dashboard_snapshot` at `state.rs:2355` does
populate `self.diagnoses` from `snap.diagnoses`. The snapshot is populated by
`DashboardEvent::Diagnosis { summary }`. This wiring appears complete.

However, the runner emits `Diagnosis` events only from the conductor's circuit-breaker
path, which requires a sustained failure pattern to trigger. In practice the panel
is empty during normal plan execution because no `Diagnosis` event fires. This is
correct behavior, but the Output panel gap (Gap 1) causes confusion because it makes
the Diagnosis panel look broken when it is actually working as designed.

The real gap here is documentation: `render_diagnosis_panel` shows "no diagnoses" with
no indication of when diagnoses are generated or how to interpret an empty panel.

### Gap 3: Agent token counts are stale in push mode

`update_from_dashboard_snapshot` at `state.rs:2244` reads `agent.input_tokens` and
`agent.output_tokens` from the snapshot's `AgentState` map. The snapshot accumulates
these through `DashboardEvent::EfficiencyEvent` when `metric == "input_tokens"` or
`metric == "output_tokens"` (see `dashboard_snapshot.rs:1308`).

The runner emits efficiency events after each gate completion, not after each
streaming token. This means token counts in the push-mode agent rows update once
per task (at gate time) rather than per turn. The token sparkline and header bar
token counter therefore update in steps rather than smoothly, even though the runner
knows the per-turn token usage immediately after each LLM call.

### Gap 4: `agent_topology` field is never populated by any subsystem

`DashboardSnapshot.agent_topology` (an `AgentTopology { nodes, edges, timestamp }`)
is consumed by `update_from_dashboard_snapshot` at `state.rs:2359` and rendered by
the agents view topology overlay. No code in the runner, the server, or the ACP path
ever sets `snap.agent_topology.nodes` to a non-empty list. The overlay always shows
"no topology nodes reported". There is no `DashboardEvent` variant that carries
topology data.

The TUI does have a pull-mode path that HTTP-fetches topology from `roko-serve`, but
that endpoint is not wired to any real data source either — it returns the default
empty `AgentTopology`.

### What already exists

| Component | Location | Status |
|---|---|---|
| `DashboardEvent::AgentOutput` | `crates/roko-core/src/dashboard_snapshot.rs:120` | EXISTS — emitted by runner |
| `DashboardSnapshot.task_outputs` | `crates/roko-core/src/dashboard_snapshot.rs:955` | EXISTS — populated by AgentOutput events |
| `update_from_dashboard_snapshot` | `crates/roko-cli/src/tui/state.rs:2052` | EXISTS — reads task_outputs into task_output_tails |
| `render_output_panel` | `crates/roko-cli/src/tui/views/dashboard_view.rs:277` | EXISTS — reads from 3 priority sources |
| `DashboardEvent::EfficiencyEvent` | `crates/roko-core/src/dashboard_snapshot.rs` | EXISTS — updates input/output_tokens per-task |
| `TuiBridge::emit_agent_output` | `crates/roko-cli/src/runner/tui_bridge.rs:102` | EXISTS — called by runner per streaming chunk |
| `AgentTopology` struct | `crates/roko-core/src/dashboard_snapshot.rs:558` | EXISTS — never populated |
| `DashboardEvent::Diagnosis` | `crates/roko-core/src/dashboard_snapshot.rs` | EXISTS — wired end-to-end |

### What is missing

1. **Priority fix in `render_output_panel`** — When `snap.task_outputs` contains live
   data for the currently active task, it should take priority over stale episode
   reads. The priority order inside `render_output_panel` should prefer push-mode live
   data (`task_output_tails` keyed by current task id) over the pull-mode episode
   tail when an active plan is running.

2. **Per-turn token events from the runner** — The runner should emit a lightweight
   `DashboardEvent::EfficiencyEvent` (or a new `AgentTokenUsage` variant) immediately
   after each LLM call completes, carrying `input_tokens` and `output_tokens` from the
   provider's usage response. Currently only gate-level efficiency events are emitted.

3. **`AgentTopology` population from live agent state** — After `AgentSpawned` events
   accumulate in the snapshot, the runner (or a periodic `PeriodicObserver` tick)
   should build an `AgentTopology` from the `agents` map and publish it as a new
   `DashboardEvent::AgentTopologyUpdated` variant. The topology itself is derivable
   from the snapshot's existing agent/plan relationships — no new data source is needed.

4. **Diagnosis panel placeholder text** — The empty-state message in
   `render_diagnosis_panel` should explain that diagnoses are emitted by the conductor
   circuit breaker and will appear here if a sustained gate-failure pattern is detected.
   This is a UI copy change, not a data wiring change.

---

## Proposed changes

### Change A: flip output panel priority for active plans
In `render_output_panel`, when `tui_state.current_plan_execution` is active (non-None
and `active == true`), check `task_output_tails` for the current task id *before*
checking `current_plan_execution.agent_output_tail`. This ensures live push-mode data
wins over the pull-mode episode snapshot when both are present.

Estimated: ~20 lines changed. Risk: low.

### Change B: per-turn token events in the runner
In `runner/event_loop.rs` (or the dispatch helper), after the provider call returns a
`TokenUsage`, emit `DashboardEvent::EfficiencyEvent` with `metric = "input_tokens"`
and `metric = "output_tokens"` for the current agent id. These events are already
handled by the snapshot's `apply` method.

Estimated: ~30 lines. Risk: low.

### Change C: topology derivation from snapshot agents
Add a `DashboardEvent::AgentTopologyUpdated { topology: AgentTopology }` variant.
In `DashboardSnapshot::apply`, handle this variant by replacing `self.agent_topology`.
In `update_from_dashboard_snapshot`, when `snap.agent_topology.is_empty()` but
`snap.agents` is non-empty, derive a minimal topology (each agent is one node; plan
edges connect agents that share a plan) and store it. Alternatively, emit the event
from a `PeriodicObserver` tick.

Estimated: ~80 lines. Risk: medium (new event variant, schema change, derive logic).

### Change D: diagnosis panel placeholder text
Replace the empty-state string in `render_diagnosis_panel` with a two-line message
that explains when diagnoses appear.

Estimated: ~5 lines. Risk: none.

---

## Acceptance criteria

1. Run `roko plan run` with an active plan. The Output panel shows live agent text
   that updates during streaming (not just after gate completion).
2. The agent token counts in the agents tab update at least once per LLM turn (not only
   at gate time). Confirmed by watching the header bar token counter during a run.
3. `grep 'AgentTopologyUpdated' crates/roko-core/src/dashboard_snapshot.rs` returns at
   least one match (new variant exists).
4. After two `AgentSpawned` events, `tui_state.agent_topology.nodes` is non-empty.
5. `cargo test --workspace` passes with zero failures after each change.
6. `cargo clippy --workspace --no-deps -- -D warnings` is clean.

---

## References

- `crates/roko-core/src/dashboard_snapshot.rs` — `DashboardSnapshot`, `DashboardEvent`, `AgentTopology`
- `crates/roko-cli/src/tui/state.rs` — `update_from_dashboard_snapshot` (~line 2052), `update_from_snapshot` (~line 1685)
- `crates/roko-cli/src/tui/views/dashboard_view.rs` — `render_output_panel` (~line 277), `render_diagnosis_panel` (~line 1034)
- `crates/roko-cli/src/runner/tui_bridge.rs` — `TuiBridge::emit_agent_output`
- `crates/roko-cli/src/runner/event_loop.rs` — runner dispatch and token usage path
- `crates/roko-cli/src/tui/dashboard.rs` — pull-mode data loading (`load_current_plan_execution`, `backfill_agent_output_tail`)
