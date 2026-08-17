# TUI Agent Data Gaps: Output Discarded, Tasks Never Populated

## Problem

During `roko plan run`, the TUI dashboard shows agent cards with empty fields:
- `AgentOutput` text is discarded at the snapshot boundary
- `agents[].current_task` is never populated
- No `Diagnosis` events from runner v2
- Efficiency events are pull-mode only (TUI never receives them in push mode)

## Root Cause

### A. AgentOutput discarded at snapshot boundary

**File:** `crates/roko-cli/src/tui/state_hub.rs`

`DashboardEvent::AgentOutput { agent_id, text }` is received by StateHub and stored in
`DashboardData.agent_outputs`. But when StateHub builds `DashboardSnapshot` for the TUI,
the `agent_outputs` field is not copied:

```rust
impl StateHub {
    fn build_snapshot(&self) -> DashboardSnapshot {
        DashboardSnapshot {
            agents: self.data.agents.clone(),
            tasks: self.data.tasks.clone(),
            // agent_outputs: NOT included
            ..Default::default()
        }
    }
}
```

The TUI `AgentTab` reads `snapshot.agent_outputs` which is always empty.

### B. `agents[].current_task` never set

**File:** `crates/roko-cli/src/orchestrate.rs`

When dispatching an agent for a task, the orchestrator publishes:
```rust
event_bus.publish(DashboardEvent::AgentStarted { agent_id, model, role });
```

But it never publishes:
```rust
event_bus.publish(DashboardEvent::AgentTaskAssigned { agent_id, task_id, task_title });
// ← this event type doesn't exist
```

So the TUI shows agents with a model and role but no indication of what they're working on.

### C. No Diagnosis events from runner v2

**File:** `crates/roko-orchestrator/src/runner.rs`

The runner v2 (`PlanRunner`) doesn't publish `DashboardEvent::Diagnosis` events. The
`DiagnosisTab` in the TUI is completely empty during plan execution. The diagnosis system
in `roko-conductor` generates insights but they're not forwarded to the event bus.

### D. Efficiency events pull-mode only

**File:** `crates/roko-cli/src/orchestrate.rs`

Efficiency events are written to `.roko/learn/efficiency.jsonl` (pull mode) but not
published to the event bus (push mode). The TUI `EfficiencyTab` reads from the file on
a timer, which works but has latency. The event bus path would be real-time.

## Fix

### Fix 1: Include agent_outputs in snapshot (~5 min)

**File:** `crates/roko-cli/src/tui/state_hub.rs`

Add `agent_outputs` to the snapshot builder:
```rust
fn build_snapshot(&self) -> DashboardSnapshot {
    DashboardSnapshot {
        agents: self.data.agents.clone(),
        tasks: self.data.tasks.clone(),
        agent_outputs: self.data.agent_outputs.clone(),  // ← add this
        ..Default::default()
    }
}
```

### Fix 2: Publish task assignment events (~10 min)

**File:** `crates/roko-cli/src/orchestrate.rs`

After agent dispatch, publish the task context:
```rust
event_bus.publish(DashboardEvent::AgentUpdate {
    agent_id,
    current_task: Some(task.title.clone()),
});
```

### Fix 3: Forward diagnosis events from runner v2 (~15 min)

**File:** `crates/roko-orchestrator/src/runner.rs`

After each task completion, publish diagnosis:
```rust
if let Some(diagnosis) = conductor.diagnose(&task_result) {
    event_bus.publish(DashboardEvent::Diagnosis(diagnosis));
}
```

### Fix 4: Publish efficiency events to bus (~5 min)

**File:** `crates/roko-cli/src/orchestrate.rs`

After writing to the JSONL file, also publish:
```rust
event_bus.publish(DashboardEvent::Efficiency(efficiency_event.clone()));
```

## Files to Modify

| File | Change |
|------|--------|
| `crates/roko-cli/src/tui/state_hub.rs` | Include agent_outputs in snapshot |
| `crates/roko-cli/src/orchestrate.rs` | Publish task assignment + efficiency events |
| `crates/roko-orchestrator/src/runner.rs` | Forward diagnosis events |

## Priority

**P1** — The TUI is the primary monitoring interface during plan execution. Empty panels
make it useless for understanding what agents are doing.
