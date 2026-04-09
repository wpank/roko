# Path 6: Observable Execution -- TUI, HTTP Events, Progress

> Completed for persisted runner artifacts and HTTP projection/query surfaces in implementation pass 2026-04-26. Live TUI polish remains tracked outside this completed HTTP/runtime slice.

## Current State (What's Broken)

The observability pipeline has solid foundations -- `StateHub` provides a pub/sub event bus with
materialized snapshots, and `TuiBridge` wraps it with convenience methods. But the runner only
publishes a narrow subset of events, leaving the TUI and HTTP consumers with incomplete data.

### Specific gaps (6/10 wired)

| # | Problem | Where |
|---|---------|-------|
| O1 | **Only a subset of agent events published** -- `handle_agent_event` in `agent_events.rs` publishes `AgentOutput` (MessageDelta) and `AgentCompleted` to TUI, but not `ToolCall`, `ToolOutput`, or `TokenUsage`. TUI/HTTP consumers can't show tool activity or token burn rate. | `runner/agent_events.rs:11-95` |
| O2 | **No token usage events** -- `TokenUsage` is accumulated silently in `RunState` fields (`tokens_in`, `tokens_out`, etc.) but never published as a `DashboardEvent`. The TUI has no real-time token count. | `runner/agent_events.rs:42-54` |
| O3 | **No cost tracking events** -- `TurnCompleted.total_cost_usd` is stored in `state.cost_usd` but never emitted as a dashboard event. The TUI cost display only updates after gate completion. | `runner/agent_events.rs:56-82` |
| O4 | **Agent model name incomplete in spawn event** -- `TuiBridge::agent_spawned` is called with the model string in `event_loop.rs:507`, but `AgentState.model` in the snapshot is not populated until the `SystemInit` event reports the actual model (which may differ from the requested model). No event updates the snapshot model. | `runner/event_loop.rs:507` vs `agent_events.rs:13-17` |
| O5 | **Gate output not streamed** -- Gate results arrive as a batch in `GateCompletion`, but the gate subprocess output is not streamed live. The `TaskOutputAppended` event exists in `DashboardEvent` but is never emitted during gate execution. | `runner/event_loop.rs:197-262` |
| O6 | **No phase transition logging as events** -- `TuiBridge::phase_transition` is called, publishing `PhaseTransition` events, but there is no structured `EventLogEntry` emitted at each transition for the event log tab. | `runner/event_loop.rs:177,248` |
| O7 | **Non-TUI mode gets no output** -- When running without `--tui`, the runner produces only `tracing` log lines. There is no structured CLI output showing task progress, agent activity, or gate results. | `runner/event_loop.rs` |
| O8 | **TUI task list shows phases but not individual task status icons** -- `TaskStarted` and `TaskCompleted` are published, but there is no per-task status tracking in the snapshot that shows pending/running/passed/failed with status icons. | `dashboard_snapshot.rs:170-184` |

### What works (6/10)

| # | Working | Where |
|---|---------|-------|
| W1 | `StateHub` pub/sub with watch + broadcast + ring buffer | `state_hub.rs` |
| W2 | `DashboardSnapshot` materialized from events | `dashboard_snapshot.rs` |
| W3 | `TuiBridge` convenience methods for common events | `tui_bridge.rs` |
| W4 | `AgentOutput` (MessageDelta) streamed to TUI | `agent_events.rs:20-24` |
| W5 | `PlanStarted/Completed`, `TaskStarted/Completed` events | `event_loop.rs` |
| W6 | `GateResult` events per gate verdict | `event_loop.rs:200-207` |


## Design Goals

1. **All agent events published to StateHub** -- AgentSpawned (with model), ToolUse, ToolOutput, TurnTokenUsage, TurnCompleted, AgentFinished -- not just MessageDelta.
2. **Per-task progress in TUI** -- task checklist with status icons: pending, running, passed, failed.
3. **Token usage per-turn** as dashboard events for real-time burn rate display.
4. **Cost tracking** as authoritative event on TurnCompleted.
5. **Agent model name** confirmed in a dedicated event after SystemInit reports actual model.
6. **Gate output streamed** to `TaskOutputAppended` events during gate execution.
7. **Event log entries** at every phase transition for the structured event log.
8. **Live agent output in non-TUI mode** via structured `tracing` subscriber or plain-text progress.


## Architecture

### Event Catalog

Every `DashboardEvent` variant and when it fires:

| Variant | Fires when | Published by | Status |
|---------|-----------|-------------|--------|
| `PlanStarted` | Plan dispatched | `dispatch_action:DispatchPlan` | Existing |
| `PlanCompleted` | Plan terminal | `dispatch_action:CompletePlan/FailPlan` | Existing |
| `TaskStarted` | Agent about to spawn | `dispatch_action:SpawnAgent` | Existing |
| `TaskCompleted` | Gate pass/fail | `gate_rx.recv` branch | Existing |
| `TaskPhaseChanged` | Task changes phase | `dispatch_action` | Existing |
| `AgentSpawned` | Agent process started | `dispatch_action:SpawnAgent` | Existing |
| `AgentOutput` | MessageDelta from agent | `handle_agent_event` | Existing |
| `AgentCompleted` | Agent process done | `handle_agent_event:TurnCompleted` | Existing |
| `GateResult` | Per-gate verdict | `gate_rx.recv` branch | Existing |
| `PhaseTransition` | Plan phase change | `event_loop.rs` | Existing |
| `EfficiencyEvent` | Metric recorded | `emit_episode` | Existing |
| `TaskOutputAppended` | Gate subprocess output | Not wired | **NEW wiring** |
| `EventLogEntry` | Phase transitions, errors, milestones | Not wired | **NEW wiring** |
| **`AgentToolUse`** | Agent calls a tool | Not wired | **NEW variant** |
| **`AgentToolResult`** | Tool returns result | Not wired | **NEW variant** |
| **`TurnTokenUsage`** | Per-turn token counts | Not wired | **NEW variant** |
| **`TurnCostUpdate`** | Authoritative cost from TurnCompleted | Not wired | **NEW variant** |
| **`AgentModelConfirmed`** | SystemInit reports actual model | Not wired | **NEW variant** |

### New Types

```rust
// ── dashboard_snapshot.rs additions ─────────────────────────────────

/// Events that mutate the dashboard snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DashboardEvent {
    // ... existing variants unchanged ...

    /// Agent invoked a tool (new).
    AgentToolUse {
        agent_id: String,
        tool_name: String,
        tool_call_id: String,
    },

    /// Tool returned a result (new).
    AgentToolResult {
        agent_id: String,
        tool_call_id: String,
        /// Whether the tool call succeeded.
        success: bool,
        /// Truncated output preview (max 256 chars for snapshot).
        preview: String,
    },

    /// Per-turn token usage update (new).
    TurnTokenUsage {
        agent_id: String,
        plan_id: String,
        task_id: String,
        turn_index: u32,
        input_tokens: u64,
        output_tokens: u64,
        cache_read_tokens: u64,
        cache_write_tokens: u64,
    },

    /// Authoritative cost update from a completed turn (new).
    TurnCostUpdate {
        agent_id: String,
        plan_id: String,
        task_id: String,
        total_cost_usd: f64,
    },

    /// Agent model confirmed by SystemInit (new).
    /// Fires when the agent's actual model (from the LLM provider) is known,
    /// which may differ from the requested model.
    AgentModelConfirmed {
        agent_id: String,
        model: String,
    },
}
```

```rust
// ── dashboard_snapshot.rs: AgentState additions ─────────────────────

/// A single agent's live state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentState {
    // ... existing fields ...

    /// Tools invoked during the current session (ring buffer, last 20).
    #[serde(default)]
    pub recent_tools: VecDeque<ToolUseSummary>,

    /// Per-turn token snapshots for the current task.
    #[serde(default)]
    pub turn_tokens: Vec<TurnTokenSummary>,
}

/// Summary of a single tool invocation for the TUI.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolUseSummary {
    pub tool_name: String,
    pub tool_call_id: String,
    pub timestamp_ms: u64,
    pub completed: bool,
    pub success: Option<bool>,
}

/// Per-turn token summary for the TUI token burn display.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TurnTokenSummary {
    pub turn_index: u32,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_read_tokens: u64,
    pub cache_write_tokens: u64,
}
```

```rust
// ── dashboard_snapshot.rs: TaskState additions ──────────────────────

/// Task execution status for per-task progress display.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    /// Task is waiting for dependencies.
    Pending,
    /// Task is currently being executed by an agent.
    Running,
    /// Task passed all gates.
    Passed,
    /// Task failed gate validation.
    Failed,
    /// Task was skipped (dependency failed).
    Skipped,
}

/// A single task's live state (extended).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskState {
    // ... existing fields ...

    /// Structured status for TUI display.
    #[serde(default = "default_task_status")]
    pub status: TaskStatus,
}

fn default_task_status() -> TaskStatus {
    TaskStatus::Pending
}
```

```rust
// ── tui_bridge.rs additions ─────────────────────────────────────────

impl TuiBridge {
    // ... existing methods ...

    /// Agent invoked a tool.
    pub fn agent_tool_use(&self, agent_id: &str, tool_name: &str, tool_call_id: &str) {
        self.sender.publish(DashboardEvent::AgentToolUse {
            agent_id: agent_id.to_string(),
            tool_name: tool_name.to_string(),
            tool_call_id: tool_call_id.to_string(),
        });
    }

    /// Tool returned a result.
    pub fn agent_tool_result(
        &self,
        agent_id: &str,
        tool_call_id: &str,
        success: bool,
        preview: &str,
    ) {
        self.sender.publish(DashboardEvent::AgentToolResult {
            agent_id: agent_id.to_string(),
            tool_call_id: tool_call_id.to_string(),
            success,
            preview: preview.chars().take(256).collect(),
        });
    }

    /// Per-turn token usage.
    pub fn turn_token_usage(
        &self,
        agent_id: &str,
        plan_id: &str,
        task_id: &str,
        turn_index: u32,
        input_tokens: u64,
        output_tokens: u64,
        cache_read_tokens: u64,
        cache_write_tokens: u64,
    ) {
        self.sender.publish(DashboardEvent::TurnTokenUsage {
            agent_id: agent_id.to_string(),
            plan_id: plan_id.to_string(),
            task_id: task_id.to_string(),
            turn_index,
            input_tokens,
            output_tokens,
            cache_read_tokens,
            cache_write_tokens,
        });
    }

    /// Authoritative cost update after a completed turn.
    pub fn turn_cost_update(
        &self,
        agent_id: &str,
        plan_id: &str,
        task_id: &str,
        total_cost_usd: f64,
    ) {
        self.sender.publish(DashboardEvent::TurnCostUpdate {
            agent_id: agent_id.to_string(),
            plan_id: plan_id.to_string(),
            task_id: task_id.to_string(),
            total_cost_usd,
        });
    }

    /// Agent model confirmed by the LLM provider.
    pub fn agent_model_confirmed(&self, agent_id: &str, model: &str) {
        self.sender.publish(DashboardEvent::AgentModelConfirmed {
            agent_id: agent_id.to_string(),
            model: model.to_string(),
        });
    }

    /// Structured event log entry for the event log tab.
    pub fn event_log(
        &self,
        event_type: &str,
        plan_id: &str,
        task_id: &str,
        message: &str,
    ) {
        let timestamp_ms = chrono::Utc::now().timestamp_millis() as u64;
        self.sender.publish(DashboardEvent::EventLogEntry {
            timestamp_ms,
            event_type: event_type.to_string(),
            plan_id: plan_id.to_string(),
            task_id: task_id.to_string(),
            message: message.to_string(),
        });
    }

    /// Live gate output lines.
    pub fn gate_output(&self, task_id: &str, lines: Vec<String>) {
        self.sender.publish(DashboardEvent::TaskOutputAppended {
            task_id: task_id.to_string(),
            lines,
        });
    }
}
```

```rust
// ── runner/cli_output.rs (NEW) ──────────────────────────────────────

use roko_core::dashboard_snapshot::DashboardEvent;
use roko_core::state_hub::StateHub;
use tokio::sync::broadcast;

/// Subscriber that prints structured progress to stderr when TUI is disabled.
///
/// Listens to the StateHub broadcast channel and formats events as
/// human-readable lines. This ensures `roko plan run` without `--tui`
/// still shows meaningful progress.
pub struct CliProgressSubscriber {
    rx: broadcast::Receiver<roko_runtime::event_bus::Envelope<DashboardEvent>>,
}

impl CliProgressSubscriber {
    pub fn new(hub: &StateHub) -> Self {
        Self {
            rx: hub.subscribe_events(),
        }
    }

    /// Run the subscriber loop (spawn as a tokio task).
    pub async fn run(mut self) {
        while let Ok(envelope) = self.rx.recv().await {
            let line = format_event(&envelope.payload);
            if !line.is_empty() {
                eprintln!("{line}");
            }
        }
    }
}

fn format_event(event: &DashboardEvent) -> String {
    match event {
        DashboardEvent::PlanStarted { plan_id } =>
            format!("[plan] started: {plan_id}"),
        DashboardEvent::PlanCompleted { plan_id, success } =>
            format!("[plan] {}: {plan_id}", if *success { "completed" } else { "FAILED" }),
        DashboardEvent::TaskStarted { plan_id, task_id, title, .. } =>
            format!("[task] started: {plan_id}/{task_id} -- {title}"),
        DashboardEvent::TaskCompleted { plan_id, task_id, outcome } =>
            format!("[task] {outcome}: {plan_id}/{task_id}"),
        DashboardEvent::AgentSpawned { agent_id, model, .. } =>
            format!("[agent] spawned: {agent_id} (model: {model})"),
        DashboardEvent::AgentCompleted { agent_id } =>
            format!("[agent] completed: {agent_id}"),
        DashboardEvent::GateResult { task_id, gate, passed, .. } =>
            format!("[gate] {}: {task_id}/{gate}",
                if *passed { "PASS" } else { "FAIL" }),
        DashboardEvent::Error { message } =>
            format!("[error] {message}"),
        DashboardEvent::TurnCostUpdate { total_cost_usd, .. } =>
            format!("[cost] ${total_cost_usd:.4}"),
        _ => String::new(),
    }
}
```

### Module Layout

```
crates/roko-core/src/
  dashboard_snapshot.rs  # MODIFIED: new event variants, TaskStatus, ToolUseSummary, TurnTokenSummary

crates/roko-cli/src/runner/
  tui_bridge.rs          # MODIFIED: new convenience methods
  agent_events.rs        # MODIFIED: publish all agent events
  event_loop.rs          # MODIFIED: event log entries, gate streaming, cli subscriber
  cli_output.rs          # NEW: CliProgressSubscriber for non-TUI mode
  mod.rs                 # MODIFIED: add cli_output module
```

### Integration Points

#### 1. All agent events published

```rust
// agent_events.rs -- MODIFIED handle_agent_event

pub fn handle_agent_event(event: &AgentEvent, state: &mut RunState, tui: &TuiBridge) {
    let agent_id = agent_id_for_state(state);

    match event {
        AgentEvent::SystemInit { session_id, model } => {
            state.agent_active = true;
            state.agent_model = model.clone();
            state.session_id = Some(session_id.clone());
            // NEW: confirm actual model from provider
            tui.agent_model_confirmed(&agent_id, model);
            debug!(model = %model, session_id = %session_id, "agent initialized");
        }

        AgentEvent::MessageDelta { text } => {
            state.agent_output.push_str(text);
            tui.agent_output(&agent_id, text);
        }

        AgentEvent::ToolCall { id, name } => {
            let marker = format!("\n[tool: {name}]\n");
            state.agent_output.push_str(&marker);
            // NEW: publish tool use event
            tui.agent_tool_use(&agent_id, name, id);
        }

        AgentEvent::ToolOutput { id, output } => {
            let truncated = if output.len() > 4096 { &output[..4096] } else { output.as_str() };
            state.agent_output.push_str(truncated);
            state.agent_output.push('\n');
            // NEW: publish tool result event
            let preview: String = output.chars().take(256).collect();
            tui.agent_tool_result(&agent_id, id, true, &preview);
        }

        AgentEvent::TokenUsage {
            input_tokens, output_tokens, cache_read_tokens, cache_write_tokens,
        } => {
            state.tokens_in += input_tokens;
            state.tokens_out += output_tokens;
            state.cache_read_tokens += cache_read_tokens;
            state.cache_write_tokens += cache_write_tokens;
            // NEW: publish per-turn token usage
            tui.turn_token_usage(
                &agent_id,
                &state.plan_id,
                &state.current_task,
                state.task_agent_calls,
                *input_tokens,
                *output_tokens,
                *cache_read_tokens,
                *cache_write_tokens,
            );
        }

        AgentEvent::TurnCompleted {
            session_id, total_cost_usd, num_turns: _, is_error,
        } => {
            state.agent_active = false;
            if let Some(sid) = session_id {
                state.session_id = Some(sid.clone());
            }
            if let Some(cost) = total_cost_usd {
                state.cost_usd = *cost;
                // NEW: publish authoritative cost update
                tui.turn_cost_update(&agent_id, &state.plan_id, &state.current_task, *cost);
            }
            if *is_error {
                state.agent_output.push_str("\n[agent error]\n");
            }
            tui.agent_completed(&agent_id);
        }

        AgentEvent::Error { message } => {
            state.agent_output.push_str(&format!("\n[error: {message}]\n"));
            tui.error(message);
        }

        AgentEvent::Exited { exit_code } => {
            state.agent_active = false;
            state.agent_pid = None;
            debug!(exit_code = ?exit_code, task = %state.current_task, "agent process exited");
        }
    }
}
```

#### 2. Event log entries at phase transitions

```rust
// event_loop.rs -- in dispatch_action, SpawnAgent branch
// After agent spawned successfully:
ctx.tui.event_log(
    "agent_dispatched",
    plan_id,
    &task_id,
    &format!("Agent spawned with model {} for task {}", model_display, task_id),
);

// In gate_rx.recv branch, after gate result processing:
ctx.tui.event_log(
    if completion.passed { "gate_passed" } else { "gate_failed" },
    &completion.plan_id,
    &completion.task_id,
    &format!("Gate rung {} {}", completion.rung,
        if completion.passed { "passed" } else { "failed" }),
);

// In dispatch_action CompletePlan:
ctx.tui.event_log("plan_completed", plan_id, "", "Plan completed successfully");

// In dispatch_action FailPlan:
ctx.tui.event_log("plan_failed", plan_id, "", &format!("Plan failed: {reason}"));
```

#### 3. Gate output streaming

```rust
// gate_dispatch.rs -- MODIFIED spawn_gate to stream output

/// Spawn a gate subprocess that streams output lines to TUI.
pub fn spawn_gate_streaming(
    plan_id: String,
    task_id: String,
    rung: u32,
    workdir: PathBuf,
    gate_tx: mpsc::Sender<GateCompletion>,
    tui: TuiBridge,  // NEW parameter
) {
    tokio::spawn(async move {
        // ... existing gate subprocess spawn ...

        // NEW: stream stdout lines to TUI as they arrive
        let stdout = child.stdout.take().expect("gate stdout");
        let reader = BufReader::new(stdout);
        let mut lines = reader.lines();
        let mut output_lines = Vec::new();

        while let Ok(Some(line)) = lines.next_line().await {
            output_lines.push(line.clone());
            // Batch: send every 10 lines or on completion
            if output_lines.len() >= 10 {
                tui.gate_output(&task_id, output_lines.drain(..).collect());
            }
        }
        // Flush remaining
        if !output_lines.is_empty() {
            tui.gate_output(&task_id, output_lines);
        }

        // ... existing gate completion logic ...
    });
}
```

#### 4. Non-TUI CLI progress output

```rust
// event_loop.rs -- in run(), after creating state_hub

// NEW: spawn CLI progress subscriber when TUI is disabled
if !config.tui_enabled {
    let subscriber = CliProgressSubscriber::new(state_hub);
    tokio::spawn(subscriber.run());
}
```

### Sequence Diagram: Full Task Lifecycle

```
Time  Event Loop          AgentStream         TUI/HTTP          EventLog
 |
 |--- DispatchPlan ------> PlanStarted -------> [plan] started
 |                                               EventLogEntry("plan_started")
 |
 |--- SpawnAgent ---------> AgentSpawned ------> [agent] spawned (model: X)
 |                          TaskStarted -------> task status: Running
 |                                               EventLogEntry("agent_dispatched")
 |
 |    (agent stdout)
 |    <-- SystemInit ------- AgentModelConfirmed -> update model in snapshot
 |    <-- MessageDelta ----- AgentOutput -------> live text
 |    <-- ToolCall --------- AgentToolUse ------> tool activity indicator
 |    <-- ToolOutput ------- AgentToolResult ----> tool result preview
 |    <-- TokenUsage ------- TurnTokenUsage ----> token burn rate
 |    <-- MessageDelta ----- AgentOutput -------> live text
 |    <-- TurnCompleted ---- TurnCostUpdate ----> cost display
 |                           AgentCompleted ----> agent done
 |                                                EventLogEntry("agent_done")
 |
 |--- ImplementationDone --> PhaseTransition ----> phase: gating
 |                                                 EventLogEntry("phase_transition")
 |
 |--- RunGate ------------> (gate subprocess)
 |    <-- gate stdout ------ TaskOutputAppended -> live gate output
 |    <-- gate result ------ GateResult ---------> per-gate verdict
 |                                                 EventLogEntry("gate_passed/failed")
 |
 |--- (if passed) --------> TaskCompleted -------> task status: Passed
 |--- (if all done) ------> PlanCompleted -------> [plan] completed
 |                                                  EventLogEntry("plan_completed")
```


## Detailed Specification

### 6.1 Agent Event Publishing

**Current**: `handle_agent_event` publishes `AgentOutput` and `AgentCompleted` only.

**New**: Every `AgentEvent` variant produces at least one `DashboardEvent`:

| AgentEvent | DashboardEvent(s) |
|------------|------------------|
| `SystemInit` | `AgentModelConfirmed` |
| `MessageDelta` | `AgentOutput` (existing) |
| `ToolCall` | `AgentToolUse` |
| `ToolOutput` | `AgentToolResult` |
| `TokenUsage` | `TurnTokenUsage` |
| `TurnCompleted` | `TurnCostUpdate` + `AgentCompleted` |
| `Error` | `Error` (existing) |
| `Exited` | (no event -- internal state only) |

### 6.2 DashboardSnapshot Apply Rules

The `DashboardSnapshot::apply()` method must handle each new event variant:

```rust
DashboardEvent::AgentToolUse { agent_id, tool_name, tool_call_id } => {
    if let Some(agent) = self.agents.get_mut(agent_id) {
        agent.recent_tools.push_back(ToolUseSummary {
            tool_name: tool_name.clone(),
            tool_call_id: tool_call_id.clone(),
            timestamp_ms: now_ms(),
            completed: false,
            success: None,
        });
        // Ring buffer: keep last 20 tool calls
        while agent.recent_tools.len() > 20 {
            agent.recent_tools.pop_front();
        }
    }
}

DashboardEvent::AgentToolResult { agent_id, tool_call_id, success, .. } => {
    if let Some(agent) = self.agents.get_mut(agent_id) {
        // Find the matching pending tool call and mark it complete
        for tool in agent.recent_tools.iter_mut() {
            if tool.tool_call_id == *tool_call_id && !tool.completed {
                tool.completed = true;
                tool.success = Some(*success);
                break;
            }
        }
    }
}

DashboardEvent::TurnTokenUsage {
    agent_id, turn_index, input_tokens, output_tokens,
    cache_read_tokens, cache_write_tokens, ..
} => {
    if let Some(agent) = self.agents.get_mut(agent_id) {
        agent.input_tokens += input_tokens;
        agent.output_tokens += output_tokens;
        agent.turn_tokens.push(TurnTokenSummary {
            turn_index: *turn_index,
            input_tokens: *input_tokens,
            output_tokens: *output_tokens,
            cache_read_tokens: *cache_read_tokens,
            cache_write_tokens: *cache_write_tokens,
        });
    }
}

DashboardEvent::TurnCostUpdate { agent_id, total_cost_usd, .. } => {
    if let Some(agent) = self.agents.get_mut(agent_id) {
        agent.cost_usd = *total_cost_usd;
    }
}

DashboardEvent::AgentModelConfirmed { agent_id, model } => {
    if let Some(agent) = self.agents.get_mut(agent_id) {
        agent.model = model.clone();
    }
}
```

### 6.3 Per-Task Status Tracking

The `TaskState` gains a `status: TaskStatus` field. The snapshot applies these rules:

| Event | Status transition |
|-------|------------------|
| `TaskStarted` | `Pending -> Running` |
| `TaskCompleted { outcome: "passed" }` | `Running -> Passed` |
| `TaskCompleted { outcome: "failed" }` | `Running -> Failed` |
| Parent task failed | `Pending -> Skipped` (computed, not event-driven) |

The TUI renders status icons:
- Pending: `[ ]`
- Running: `[~]` (with spinner in TUI)
- Passed: `[+]`
- Failed: `[x]`
- Skipped: `[-]`

### 6.4 Gate Output Streaming

Currently `spawn_gate` runs the gate subprocess and collects all output at the end. The new
design streams output lines incrementally:

1. Gate subprocess spawns with `stdout = Stdio::piped()`.
2. A reader task iterates over lines and batches them (every 10 lines or 500ms).
3. Each batch is published as `TaskOutputAppended { task_id, lines }`.
4. The snapshot appends lines to the task's output buffer (capped at 500 lines).
5. The final `GateCompletion` still carries the full output for episode logging.

### 6.5 Non-TUI CLI Output

When `--tui` is not passed (or the terminal doesn't support it):

1. A `CliProgressSubscriber` is spawned as a tokio task.
2. It subscribes to the `StateHub` broadcast channel.
3. It formats selected events as single-line human-readable output to stderr.
4. Format: `[category] action: details`
5. Events printed: `PlanStarted`, `PlanCompleted`, `TaskStarted`, `TaskCompleted`, `AgentSpawned`, `AgentCompleted`, `GateResult`, `Error`, `TurnCostUpdate`.
6. Events suppressed: `AgentOutput` (too noisy), `AgentToolUse/Result` (too frequent), `TurnTokenUsage` (internal metric).

### 6.6 Event Log Entries

Every significant state transition emits an `EventLogEntry`:

| Transition | `event_type` | `message` |
|-----------|-------------|-----------|
| Plan dispatched | `plan_started` | `"Plan {plan_id} started with {n} tasks"` |
| Agent spawned | `agent_dispatched` | `"Agent spawned with model {model} for task {task_id}"` |
| Agent completed | `agent_done` | `"Agent completed (cost: ${cost:.4}, tokens: {in}/{out})"` |
| Phase transition | `phase_transition` | `"Plan {plan_id}: {from} -> {to}"` |
| Gate pass | `gate_passed` | `"Gate rung {rung} passed for {task_id}"` |
| Gate fail | `gate_failed` | `"Gate rung {rung} failed for {task_id}: {summary}"` |
| Plan completed | `plan_completed` | `"Plan {plan_id} completed successfully"` |
| Plan failed | `plan_failed` | `"Plan {plan_id} failed: {reason}"` |
| Budget exceeded | `budget_exceeded` | `"Budget exceeded: ${spent:.2} >= ${limit:.2}"` |
| Error | `error` | The error message |

These are persisted to the on-disk event log (`.roko/events.jsonl`) via the existing
`StateHub` persistence mechanism.


## Error Handling

| Scenario | Handling |
|----------|----------|
| Broadcast channel full (slow consumer) | `broadcast::Receiver::recv` returns `Lagged(n)`. CLI subscriber logs the lag count and continues. No data loss in snapshot (watch channel). |
| Gate subprocess output too large | Cap the `TaskOutputAppended` buffer at 500 lines per task. Older lines dropped. |
| Event log disk full | Existing behavior: `EventLogWriter::append` is best-effort (no propagation). |
| TUI not available (no terminal) | `CliProgressSubscriber` takes over. If stderr is not a TTY, output still works (for log capture). |
| New event variants in old snapshot files | Serde `#[serde(default)]` on all new fields ensures forward compatibility. Old snapshots load without error. |


## Testing Strategy

### Unit tests

1. **Snapshot apply for new events**: Create a `DashboardSnapshot`, apply each new event variant, verify the materialized state is correct (tool use ring buffer, token counts, cost, model confirmed).
2. **TaskStatus transitions**: Apply `TaskStarted` then `TaskCompleted(passed)`, verify status goes `Pending -> Running -> Passed`. Same for failed path.
3. **Tool use ring buffer**: Apply 25 `AgentToolUse` events, verify only the last 20 are retained.
4. **Event formatting**: Unit test `format_event` for each event variant in `CliProgressSubscriber`.

### Integration tests

5. **Full event stream**: Run a mock agent through the event loop, capture all `DashboardEvent`s from the broadcast channel, verify the complete sequence matches the expected lifecycle.
6. **Gate streaming**: Run a gate subprocess, verify `TaskOutputAppended` events arrive before `GateCompletion`.
7. **Non-TUI output**: Run the event loop without TUI, capture stderr, verify progress lines are printed.

### Snapshot tests

8. **Event log replay**: Write events to a JSONL file, replay them into a fresh `StateHub`, verify the materialized snapshot matches the expected state. This tests forward/backward compatibility.

### Performance tests

9. **Broadcast throughput**: Publish 10,000 events/second to StateHub, verify the watch channel updates are < 1ms latency (TUI 60fps requirement).


## Open Questions

1. **Tool output in AgentToolResult**: Should we include the full tool output or just a preview? The current spec uses 256-char preview. Full output would bloat the snapshot for tools like `Bash` that can return megabytes.
   - **Recommendation**: 256-char preview in the snapshot event. Full output stays in the accumulated `state.agent_output` string for episode logging.

2. **Event log entry deduplication**: Phase transitions can fire multiple times for the same plan/task. Should `EventLogEntry` deduplicate?
   - **Recommendation**: No deduplication. The event log is append-only. Consumers can deduplicate by `(plan_id, task_id, event_type)` if needed.

3. **CliProgressSubscriber verbosity levels**: Should there be a `--verbose` flag that prints `AgentToolUse` events in non-TUI mode?
   - **Recommendation**: Yes, but defer to a follow-up. The initial implementation prints the core lifecycle events only.

## No-Mock Observability Findings (2026-04-26)

This section captures what was actually visible during real end-to-end runs in `/tmp/roko-real-e2e-nrUD05/work`.

### Concrete findings from persisted events

1. Codex run persisted only sparse events in `.roko/events.jsonl`:
   - `agent.error` (Codex internal session logging issue)
   - `agent.exited`
   - `gate.completed`
   - `run.completed`
2. Claude run persisted rich stream events:
   - `agent.system_init`
   - `agent.message_delta`
   - `agent.tool_call`
   - `agent.token_usage`
   - `agent.turn_completed`
   - plus gate and run completion events
3. Gate verdict visibility is now good:
   - `gate.completed` includes per-gate verdict list and summaries
4. Terminal state visibility is now good:
   - `.roko/state/executor.json` reflects `current_phase.kind = "complete"` for passing plan

### Missing observability capabilities proven by the run

| Gap | Impact | Evidence |
|---|---|---|
| No normalized cross-provider event contract at projection layer | Codex and Claude produce materially different event richness | Codex vs Claude event sets in `.roko/events.jsonl` |
| No explicit run id in emitted event records | Hard to query and correlate multi-run history in one workspace | Events currently keyed by plan/task/timestamp only |
| No first-class endpoint/query for gate history by run | Debugging gate regressions requires raw jsonl parsing | Gate data exists in events file but not query indexed |
| No first-class endpoint/query for agent stderr categories | Provider CLI internal warnings look like errors with no severity taxonomy | Codex session warning captured only as raw message |

### Additions to implementation checklist

- [ ] Add normalized provider-agnostic event category mapping before projection output.
- [ ] Add `run_id` to every persisted runtime event payload.
- [ ] Add lightweight event query index file per run (`.roko/state/events-index.json`).
- [ ] Add endpoint support for `GET /api/events?run_id=...&plan_id=...`.
- [ ] Add endpoint support for `GET /api/gates?run_id=...`.
- [ ] Add agent stderr classification (`warning`, `error`, `infra`) before persistence.
- [ ] Add projection-level counters for "events dropped" and "events coerced".
- [ ] Add no-mock parity test asserting minimum event set across Codex and Claude.

## Implementation Packet

This work makes runtime state observable through one projection stream.

### Required Context

- `crates/roko-core/src/dashboard_snapshot.rs`
- `crates/roko-core/src/state_hub.rs`
- `crates/roko-cli/src/runner/tui_bridge.rs`
- `crates/roko-cli/src/runner/agent_events.rs`
- `crates/roko-cli/src/runner/gate_dispatch.rs`
- `crates/roko-cli/src/tui/`
- `docs/12-interfaces/06-websocket-streaming.md`
- `docs/12-interfaces/22-statehub-projection-layer.md`
- `tmp/unified/15-TELEMETRY.md`

### Target Files

- [ ] Extend `DashboardEvent` in `roko-core`.
- [ ] Extend `DashboardSnapshot` state mutation logic.
- [ ] Add methods to `runner/tui_bridge.rs` or replace it with a projection facade.
- [ ] Add `crates/roko-cli/src/projection/mod.rs`.
- [ ] Add `crates/roko-cli/src/projection/cli_progress.rs`.
- [ ] Update `runner/agent_events.rs` to publish tool, token, and cost events.

### Checklist

- [ ] Add events for agent model confirmation, tool call start, tool call finish, token usage, and cost update.
- [ ] Add task status enum values: pending, running, passed, failed, skipped.
- [ ] Stream gate output chunks as `TaskOutputAppended` or equivalent.
- [ ] Emit structured event-log entries on phase transitions.
- [ ] Add non-TUI progress subscriber that prints concise task lifecycle output.
- [ ] Ensure dashboard snapshot stores bounded ring buffers for tools and output previews.
- [ ] Avoid storing full megabyte-scale tool output in `DashboardSnapshot`.
- [ ] Ensure every runtime terminal event creates a visible projection event.

### Acceptance Criteria

- [ ] TUI can show current task, active agent, tool activity, tokens, and cost before task completion.
- [ ] Non-TUI plan run prints useful progress without requiring debug logs.
- [ ] HTTP/SSE subscribers can consume the same event categories as TUI.
- [ ] Snapshot mutation tests cover every new `DashboardEvent` variant.
