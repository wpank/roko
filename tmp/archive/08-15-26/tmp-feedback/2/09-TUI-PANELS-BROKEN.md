# TUI Dashboard Panels: Mostly Disconnected

## Symptom

During a live `roko plan run` in daeji (Runner v2, 8 tasks, agents actively running),
the TUI dashboard shows partial data. Many panels display zeros or placeholder text
despite real work happening.

### What Works

| Panel | Status | Notes |
|-------|--------|-------|
| Top bar | Working | Wave 1/1, 7/9 tasks, 77%, ETA:2m22s |
| Plans | Working | Shows plan progress, phase (Gating) |
| Tasks | Working | T01-T07 checkmarks, T08 running |
| Agents | **Partial** | Shows 8 agents, active/idle status — but task column is "-", progress is "0k/200k" |
| Routes | **Partial** | Shows 4 entries, all "balanced" |
| System | Working | CPU 65.4%, MEM, NET, DSK |

### What's Broken

| Panel | Shows | Should Show |
|-------|-------|-------------|
| Output | "no agent output yet" | Streaming text from active agent |
| Efficiency | "tokens 0 cost $0.00 avg/task 0" | Cumulative token usage and cost |
| Diagnosis | "no conductor diagnoses yet" | Gate failure analysis, health warnings |
| Agent task column | "-" | Current task name (e.g., "T08") |
| Agent progress | "0k/200k" | Actual context window usage |

## Root Cause: Push vs Pull Mode Mismatch

The TUI has **two data loading paths** that were never unified:

### Pull Mode (standalone `roko dashboard`)

`TuiState::from_dashboard_data()` reads from disk:
- `.roko/learn/efficiency.jsonl` → efficiency events, token counts
- `.roko/task-outputs/` → agent output text
- `.roko/state/executor.json` → plan/task state
- `.roko/conductor/` → diagnosis alerts

This path populates all fields correctly. **It works when the TUI reads static files.**

### Push Mode (embedded in `roko plan run`)

`TuiState::update_from_dashboard_snapshot()` receives `DashboardSnapshot` via StateHub:

```rust
// state.rs — update_from_dashboard_snapshot()
// These fields ARE updated:
self.plans = snap.plans...          ✓
self.tasks = snap.tasks...          ✓
self.agents = snap.active_agents... ✓ (partial)

// These fields are NOT updated:
self.agent_output = ???             ✗ Not in snapshot
self.efficiency_events = ???         ✗ Not in snapshot
self.efficiency_summary = default!() ✗ Hardcoded zeros
self.diagnoses = ???                 ✗ Not propagated
self.agents[i].current_task = ???    ✗ Missing from active_agents
self.agents[i].input_tokens = ???    ✗ Missing from active_agents
```

**The `DashboardSnapshot` struct doesn't carry the data these panels need.**

## Panel-by-Panel Diagnosis

### 1. Output Panel — "no agent output yet"

**Data path**: `tui_state.current_plan_execution.agent_output_tail`

In pull mode: read from `.roko/task-outputs/{task_id}.txt`
In push mode: **never populated** — `DashboardSnapshot` has no `agent_output_tail` field

**File**: `crates/roko-cli/src/tui/views/dashboard_view.rs:277-384`

**Fix**: Add `agent_output_tail: Vec<String>` to `DashboardSnapshot`, populate it from
the runner's stdout capture channel, or forward `AgentEvent::MessageDelta` text to TUI.

### 2. Efficiency Panel — All zeros

**Data path**: `tui_state.efficiency_events` + `tui_state.efficiency_summary`

In pull mode: read from `.roko/learn/efficiency.jsonl` (populated by orchestrate.rs)
In push mode: `update_from_dashboard_snapshot()` creates a default `EfficiencySummary`
with all zeros — never reads from snapshot

**File**: `crates/roko-cli/src/tui/state.rs:2393`

**Fix**: Add `efficiency_events: Vec<EfficiencyEvent>` to `DashboardSnapshot`. The runner
already emits `RunnerEvent::task_attempt_started` with cost data — forward it to the snapshot.

### 3. Diagnosis Panel — "no conductor diagnoses yet"

**Data path**: `tui_state.diagnoses`

In pull mode: read from `data.conductor_alerts` — but then NOT propagated to `tui_state.diagnoses`
In push mode: `snap.diagnoses.iter().cloned().collect()` — works IF snap has data, but runner
doesn't publish conductor diagnoses to StateHub

**Files**: `crates/roko-cli/src/tui/state.rs:2264`, `dashboard.rs:477-480`

**Fix**: Wire `conductor_alerts` → `diagnoses` in pull mode. In push mode, have the runner
emit `DashboardEvent::DiagnosisAdded` when gate failures occur.

### 4. Agent Task Column — Shows "-"

**Data path**: `AgentRow.current_task`

In pull mode: populated from `efficiency_events` — last event per agent gives task_id
In push mode: reconstructed from `snap.active_agents` which doesn't include task assignments

**File**: `crates/roko-cli/src/tui/widgets/parallel_pool.rs:62-68` — falls back to `"-"`
when `agent.current_task.is_empty()`

**Fix**: Include `task_id` in `DashboardSnapshot::active_agents` entries. The runner already
has this mapping in `ctx.active_agent_tasks: HashMap<String, String>`.

### 5. Agent Progress — "0k/200k"

**Data path**: `AgentRow.input_tokens` + `AgentRow.context_limit`

In pull mode: from `efficiency_events` last entry per agent → `event.input_tokens`
In push mode: never updated — stays at 0

**File**: `crates/roko-cli/src/tui/widgets/parallel_pool.rs:70-71` — computes
`ctx_ratio = agent.input_tokens / agent.context_limit`

**Fix**: Forward token usage from `AgentEvent::TokenUsage` through StateHub to snapshot.
The runner receives these events from `agent_events.rs`.

## The Systemic Pattern

Every broken panel follows the same pattern:

1. **Pull mode works** — reads from disk, has all data
2. **Push mode is incomplete** — `DashboardSnapshot` doesn't carry the data
3. **Runner publishes to StateHub** — but only plan/task/agent lifecycle events
4. **StateHub → TUI gap** — no efficiency, output, or diagnosis events flow through

### What the Runner DOES publish (via TuiBridge)

| Event | Published? | Received by TUI? |
|-------|-----------|-----------------|
| `plan_started` | Yes | Yes |
| `task_started` | Yes | Yes |
| `agent_spawned` | Yes | Partially (no task, no tokens) |
| `agent_finished` | Yes | Yes |
| `gate_result` | Yes | Yes |
| `phase_transition` | Yes | Yes |
| `message_delta` (streaming text) | **No** | No |
| `token_usage` | **No** | No |
| `efficiency_event` | **No** | No |
| `conductor_diagnosis` | **No** | No |

### What Needs to Flow Through

```
Runner → TuiBridge → StateHub → DashboardSnapshot → TuiState
                                    ↑ MISSING:
                                    - message_delta (output text)
                                    - token_usage (context progress)
                                    - efficiency_event (cost tracking)
                                    - conductor diagnosis (health alerts)
                                    - agent task assignment (task column)
```

## Fix Plan

### Phase 1: Make existing events carry more data (~1 hr)

Add fields to existing `DashboardEvent` variants:
- `AgentSpawned` → add `task_id: String`
- `AgentUpdate` → add `input_tokens: u64, output_tokens: u64`
- `GateCompleted` → forward as `DiagnosisAdded` when gate fails

### Phase 2: Add missing events (~2 hr)

New `DashboardEvent` variants:
- `AgentOutputDelta { agent_id, text }` — for output panel
- `EfficiencyUpdate { tokens_in, tokens_out, cost_usd, task_id }` — for efficiency panel
- `DiagnosisAdded { severity, message, source }` — for diagnosis panel

### Phase 3: Update TuiState::update_from_dashboard_snapshot (~1 hr)

Make the push-mode handler consume the new fields and populate all `TuiState` fields
that currently only work in pull mode.

## Files to Modify

| File | Change |
|------|--------|
| `crates/roko-core/src/dashboard_snapshot.rs` | Add missing fields to snapshot struct |
| `crates/roko-core/src/dashboard_event.rs` | Add new event variants |
| `crates/roko-cli/src/tui/state.rs` | Fix `update_from_dashboard_snapshot()` |
| `crates/roko-cli/src/tui/dashboard.rs` | Wire `conductor_alerts` → `diagnoses` in pull mode |
| `crates/roko-cli/src/runner/event_loop.rs` | Publish token_usage + output events to TuiBridge |
| `crates/roko-cli/src/runner/streaming.rs` | Forward agent message deltas |
| `crates/roko-runtime/src/state_hub.rs` | Handle new event variants in snapshot apply |
