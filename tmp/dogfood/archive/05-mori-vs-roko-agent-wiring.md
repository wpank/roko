# Dogfood: Mori vs Roko — Agent Output & TUI Wiring

Deep comparison of how mori handles agent output end-to-end vs roko's current state.
Root cause analysis for: no agent text in TUI, model shows "-", tokens show "0k/200k",
tasks show "plan plan".

## TL;DR

Mori has a **streaming pipeline**: claude CLI → JSON parse per line → AgentEvent channel
→ RunState update → TUI render at 60ms ticks. Every text delta, token count, and tool
call flows in real time.

Roko has a **batch pipeline**: claude CLI → wait for exit → AgentResult struct → post-
dispatch processing. The TUI event types exist (`AgentOutput`, `AgentSpawned`) but the
output event is **never emitted**. The wiring is dead code.

---

## 1. Agent Output Capture: Streaming vs Batch

### Mori: Real-time streaming
**File**: `apps/mori/src/agent/connection.rs:2444-2731`

Mori spawns claude with `--output-format stream-json` and reads stdout line by line:

```
claude CLI stdout → each line is a ClaudeStreamEvent JSON →
  parse_claude_event() extracts:
    - Text content → AgentEvent::MessageDelta
    - Tool calls → AgentEvent::ToolCall
    - Token usage → AgentEvent::TokenUsage (per-message, mid-turn)
    - Tool output → AgentEvent::CommandOutput
    - Session end → AgentEvent::TurnCompleted (with session_id for --resume)
```

The protocol types (`apps/mori/src/agent/protocol.rs:130-196`):
```rust
enum ClaudeStreamEvent {
    System(Value),              // Init
    Assistant(ClaudeAssistantEvent), // Text + tools + usage
    Tool(Value),                // Tool result
    Result(ClaudeResultEvent),  // Final summary + cost
}
```

Key: **every content block and every token count arrives as a separate event** while
the agent is still running. The TUI sees text appear character-by-character.

### Roko: Batch after exit
**File**: `crates/roko-agent/src/dispatcher/mod.rs`, `crates/roko-agent/src/task_runner/`

Roko spawns claude CLI and waits for it to exit. The entire output is collected into
a single `AgentResult`:

```rust
pub struct AgentResult {
    pub output: Engram,      // Final text blob
    pub trace: Vec<Engram>,  // Intermediate signals (if any)
    pub usage: Usage,        // Total tokens + cost
    pub success: bool,
}
```

**No streaming events**. No per-line parsing. No mid-turn token counts. The TUI gets
nothing until the agent process exits and `dispatch_agent_with()` returns.

### Impact
- TUI shows "no agent output yet" for the entire duration of agent execution
- Token counters stay at 0 until the agent finishes
- No way to see what the agent is doing (reading files? writing code? stuck?)

---

## 2. Event Flow: Rich Pipeline vs Dead Code

### Mori: 9-variant AgentEvent → RunState → TUI
**File**: `apps/mori/src/agent/events.rs`

```rust
enum AgentEvent {
    MessageDelta { role, instance, content },     // Streaming text
    TurnCompleted { role, instance, thread_id },  // Turn done + session_id
    DiffUpdated { role, instance, diff },          // Git diff
    ApprovalRequested { role, instance, command }, // Permission gate
    TokenUsage { role, instance, input_tokens, output_tokens, cost_usd },
    ToolCall { role, instance, name },             // Tool invoked
    CommandOutput { role, instance, content },     // Tool result
    Error { role, instance, error },
    Exited { role, instance, exit_code },
}
```

All events flow through `handle_agent_event()` (`apps/mori/src/app/events.rs:17-389`)
into `RunState`:

```rust
MessageDelta → state.agent_state_mut(role).output.push_str(&content)
TokenUsage   → agent.input_tokens = N; state.cumulative_cost_usd += delta
ToolCall     → state.mcp.record_tool_call(backend, role, &name)
CommandOutput → state.append_command_output(&content)
TurnCompleted → agent.active = false; agent.thread_id = session_id
```

The main event loop (`apps/mori/src/app/sequential.rs:451-730`) does:
```rust
tokio::select! {
    event = agent_rx.recv() => handle_agent_event(..., event),
    event = orch_rx.recv() => handle_orchestrator_event(...),
    input = term_events.next() => handle_tui_action(...),
}
// then: terminal.draw(|f| tui::layout::render(f, &state));
```

Every ~60ms tick, the TUI renders the current `RunState` with accumulated text, tokens,
and cost.

### Roko: Event types exist, emission is dead code
**File**: `crates/roko-serve/src/events.rs:84-300`

```rust
pub enum ServerEvent {
    AgentSpawned { agent_id, role },           // ← EMITTED ✓
    AgentOutput { agent_id, content, done, metadata },  // ← NEVER EMITTED ✗
    GateResult { ... },                        // ← EMITTED ✓
    EfficiencyEvent { ... },                   // ← EMITTED ✓ (after dispatch)
    // ... 200+ more variants
}
```

The `DashboardEvent::AgentOutput` type exists (`dashboard_snapshot.rs:52`):
```rust
pub enum DashboardEvent {
    AgentOutput { agent_id: String, content: String },
    // ...
}
```

The converter exists (`orchestrate.rs:17995-18000`):
```rust
ServerEvent::AgentOutput { agent_id, content, .. } =>
    Some(DashboardEvent::AgentOutput { agent_id, content })
```

**But `emit_server_event(ServerEvent::AgentOutput { ... })` is never called anywhere.**
The entire pipeline is wired except for the single line that would activate it.

---

## 3. Token & Model Display: Why "-" and "0k"

### Mori: Real-time per-message
**File**: `apps/mori/src/app/events.rs:314-358`

```rust
AgentEvent::TokenUsage { input_tokens, output_tokens, cost_usd, .. } => {
    agent.input_tokens = input_tokens;         // Updated per-message
    agent.output_tokens = output_tokens;
    state.cumulative_input_tokens += input_tokens;
    state.cumulative_cost_usd += cost_delta;
    *state.cost_per_plan.entry(plan).or_default() += cost_delta;
}
```

Mori gets token updates **mid-turn** from `ClaudeStreamEvent::Assistant` usage fields
and **final totals** from `ClaudeStreamEvent::Result`. The TUI always has current numbers.

### Roko: Batch from efficiency.jsonl, loaded from disk
**File**: `crates/roko-cli/src/tui/dashboard.rs:1314-1402`

```rust
fn build_agent_activity_snapshot(
    active_agents: &[AgentSummary],
    efficiency_events: &[AgentEfficiencyEvent],
) -> Option<AgentActivitySnapshot> {
    for event in efficiency_events {
        entry.turns += 1;
        entry.tokens_used += event.input_tokens + event.output_tokens;
        entry.cost_usd += event.cost_usd;
    }
}
```

The model and tokens come from `AgentEfficiencyEvent` records in `.roko/learn/efficiency.jsonl`.
But:
1. `efficiency.jsonl` is only written after `record_turn_learning_feedback()` completes
2. That only runs after `dispatch_agent_with()` returns
3. The file doesn't even exist during this run (see dogfood issue S3)
4. Even if it did, the TUI loads it from disk — there's no event-bus path

**Result**: Model column shows "-", tokens show "0k", cost shows "-" for the entire run.

---

## 4. Task Display: Why "plan plan" Repeats

### The bug
The Plans tab in the TUI shows tasks as "plan plan" repeated instead of actual task
names like "M001 Baseline verification snapshot".

### Root cause
**File**: `crates/roko-core/src/dashboard_snapshot.rs`

`TaskState` in the snapshot lacks a title/description field:
```rust
pub struct TaskState {
    pub task_id: String,    // e.g., "M001"
    pub plan_id: String,    // e.g., "unified-migration-phase0"
    pub phase: String,
    pub outcome: Option<String>,
}
```

**File**: `crates/roko-cli/src/tui/state.rs:2052-2058`

```rust
TaskRow {
    id: task.task_id.clone(),
    title: task.task_id.clone(),  // ← Falls back to task_id, no title!
    status,
}
```

The plan view renders both `plan_id` and `task_id` in columns, but when no meaningful
title is available, it shows generic "plan" text.

### Mori comparison
Mori's `RunState` carries full task definitions with titles, descriptions, and role
assignments. The TUI renders `task.title` directly.

---

## 5. Persistence During Run: Flush vs Buffer

### Mori: Writes immediately, background thread
**File**: `apps/mori/src/state/persistence.rs:524-571`

```rust
pub fn append_task_event_bg(&self, event: &TaskEvent) {
    let path = self.events_file.clone();
    tokio::task::spawn_blocking(move || {
        let mut file = OpenOptions::new().create(true).append(true).open(&path)?;
        writeln!(file, "{json}")?;
    });
}
```

Events are flushed to `events.jsonl` in real-time via background tasks. Cost summaries
and task state are also persisted after each transition. If mori crashes, you lose at
most the current turn.

### Roko: Buffers in memory, writes at shutdown
The EpisodeLogger, efficiency writer, and signal log all buffer in memory. Disk writes
happen:
- At graceful shutdown (`flush_logs()`)
- At state checkpoint (which never triggers — `executor.json` is never created)

If roko crashes (like the TUI panic in run 1), **everything is lost**.

Current run evidence:
- `episodes.jsonl`: 2 lines (from April 24-25, not this run)
- `efficiency.jsonl`: does not exist
- `signals.jsonl`: 0 lines
- `executor.json`: does not exist
- `learn/` files: all timestamps from April 25

---

## 6. Process Management: Tracked vs Fire-and-Forget

### Mori: Full subprocess lifecycle
Mori tracks the claude subprocess PID, reads its stdout/stderr via async readers,
catches exit codes, and records session_ids for `--resume`. The `AgentPool` manages
concurrent agent limits and handles graceful shutdown of all agents.

### Roko: PID tracked but output ignored
Roko tracks PIDs in `agent-pids.json` and the `ProcessSupervisor` can `kill_all()`.
But there's no stdout/stderr reading during execution — the entire output is collected
after exit. If the process hangs, the only signal is a timeout.

---

## 7. Summary: What's Missing in Roko

| Feature | Mori | Roko | Gap |
|---------|------|------|-----|
| Streaming text output | `--stream-json` parsed per-line | Batch after exit | **No real-time output** |
| Per-message token counts | `AgentEvent::TokenUsage` mid-turn | `AgentResult.usage` at end | **No live token tracking** |
| Agent output → TUI | `MessageDelta` → `RunState.output` → render | `AgentOutput` event defined but never emitted | **Dead code** |
| Model name in TUI | From agent config, available immediately | From `efficiency.jsonl`, never written | **Always "-"** |
| Task titles in TUI | `task.title` from full definitions | `task_id` string only, no title | **"plan plan"** |
| Cost tracking | Per-turn, per-plan, per-task, cumulative | Efficiency events, never flushed to disk | **$0.00 always** |
| Tool call visibility | `ToolCall` events → MCP stats panel | Not published to TUI | **Invisible** |
| Command/gate output | `CommandOutput` → scrollable panel | Gate results as `DashboardEvent::GateResult` only | **No streaming gate output** |
| Persistence during run | `append_task_event_bg()` after every event | Buffer in memory, flush at shutdown | **Crash = total loss** |
| Executor snapshot | Task state saved after every transition | `executor.json` never created | **No resume possible** |
| Session resumption | `--resume` with thread_id per-agent | `--resume` with executor.json (never written) | **Can't resume** |

## 8. Fix Priority

### P0 — Emit AgentOutput after dispatch
Single missing `emit_server_event(ServerEvent::AgentOutput {...})` call in
`dispatch_agent_with()` post-return path. This unblocks the entire TUI output pipeline.

### P0 — Write executor.json during run
Persist after each phase transition. Without this, crashes lose all progress.

### P1 — Flush episodes/efficiency during run
Call `flush_logs()` or write events immediately (like mori's `append_task_event_bg`).

### P1 — Add task title to DashboardSnapshot::TaskState
Carry `title: String` from `TaskDef` into `TaskState` so the TUI can display it.

### P1 — Embed model in AgentSpawned event
So the TUI knows the model name from the start, not from a file that doesn't exist.

### P2 — Stream output during execution
Replace batch collection with line-by-line `--stream-json` parsing (like mori).
This is a larger refactor but is what makes the TUI actually useful.

### P2 — Per-turn cost tracking
Emit `DashboardEvent::EfficiencyUpdate` with tokens/cost after each turn, not just
at the end.
