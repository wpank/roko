# 22 — Plan Runner v2

> Status: **SPEC DRAFT**
> Replaces: `crates/roko-cli/src/orchestrate.rs` (21,478 lines)
> Target: `crates/roko-cli/src/runner/` (~2,500 lines across 10 files)

## 1. Why Rewrite

`orchestrate.rs` has correct crate boundaries but broken wiring. Every fix reveals
another layer of tangled assumptions:

- `plans_dir()` resolves to the wrong directory (3 bugs from this alone)
- Enrichment pipeline overwrites user's `tasks.toml`
- `discover_plans` treats enrichment artifacts as plans
- Agent output captured batch-only (no streaming)
- All persistence buffered in memory (crash = total loss)
- 250 methods on PlanRunner across 15K lines in 3 impl blocks
- TUI gets nothing during agent execution

Mori solved all of these in ~2,500 lines. The existing roko crate APIs are solid.
The problem is entirely the glue layer.

## 2. Design Principles

Taken from mori's architecture + unified spec:

### P1: Single-threaded event loop, async I/O
No race conditions on state. TUI rendering atomic per frame. All mutations in one
place. Mori's `sequential.rs` uses `tokio::select!` over 4 channels — proven pattern.

### P2: Channels as event bus
Four independent channels: agent events, executor actions, gate results, TUI input.
Decouples subsystems. Non-blocking recv. Easy to extend.

### P3: Stream, don't batch
Parse `--stream-json` line by line (like mori's `parse_claude_event`). Emit
`AgentEvent::MessageDelta` / `TokenUsage` / `ToolCall` in real time. TUI sees text
as it arrives.

### P4: Flush after every task
Write episode + efficiency + executor snapshot after each task completes. Atomic
writes (tmp + rename). Crash loses at most 1 task. Mori writes every 2 seconds.

### P5: The plan dir is the plan
When user passes a directory with `tasks.toml`, that's the plan. No scanning for
`.md` files. No enrichment artifact confusion. `tasks.toml` is the source of truth
and is never overwritten by the system.

### P6: Executor is pure, runner does I/O
`ParallelExecutor::tick()` returns actions. Runner dispatches them to real systems.
Results fed back as events. This is already the pattern — just not cleanly implemented.

### P7: Align with unified spec
Use naming conventions from the unified spec where practical. Structure the runner
as a precursor to the unified Engine. Activity recording per-node (not monolithic
snapshots). Lifecycle Pulses on StateHub.

## 3. Architecture

```
                    ┌─────────────────────────────┐
                    │         PlanRunner           │
                    │  (event loop: select!)       │
                    │                              │
  ┌──agent_rx──►    │  match event {               │
  │                 │    AgentEvent::* => update    │    ──► StateHub (TUI)
  │  ┌─gate_rx──►   │    GateResult => feed executor│
  │  │              │    TuiInput => handle keys   │    ──► EpisodeLogger
  │  │  ┌─tick──►   │    Tick => executor.tick()   │
  │  │  │           │      => dispatch actions     │    ──► executor.json
  │  │  │           │  }                           │
  │  │  │           └─────────────────────────────┘
  │  │  │                    │
  │  │  │           ┌────────┴────────┐
  │  │  │           │  ParallelExecutor │  (pure state machine)
  │  │  │           └────────┬────────┘
  │  │  │                    │ ExecutorAction
  │  │  │           ┌────────┴────────┐
  │  │  │           │ Action Dispatcher │
  │  │  │           ├─────────────────┤
  │  │  │           │ SpawnAgent      │──► AgentProcess (--stream-json)
  │  │  │           │ RunGate         │──► gate pipeline (background task)
  │  │  │           │ MergeBranch     │──► git operations
  │  │  │           │ CompletePlan    │──► cleanup
  │  │  │           └─────────────────┘
  │  │  │
  │  │  └── tokio interval (33ms for TUI, 2s for flush)
  │  └───── gate completion channel
  └──────── agent stdout line-by-line parsing
```

## 4. Module Layout

```
crates/roko-cli/src/runner/
  mod.rs            — PlanRunner struct, public run() API              (~200 lines)
  event_loop.rs     — tokio::select! main loop                        (~400 lines)
  agent_stream.rs   — spawn claude, parse --stream-json per line      (~300 lines)
  agent_events.rs   — handle AgentEvent variants, update RunState     (~250 lines)
  gate_dispatch.rs  — spawn gate as background task, collect results  (~200 lines)
  persist.rs        — atomic writes: executor.json, episodes, efficiency (~200 lines)
  plan_loader.rs    — load tasks.toml, validate, no discovery magic   (~150 lines)
  state.rs          — RunState: agent output, tokens, costs, progress (~300 lines)
  tui_bridge.rs     — publish DashboardEvents to StateHub             (~200 lines)
  types.rs          — AgentEvent, GateCompletion, RunConfig           (~200 lines)
```

**Total: ~2,400 lines.** Average ~240 lines/file. Largest: event_loop.rs at ~400.

## 5. Data Flow

### 5.1 Agent Output (Streaming)

Mori pattern adapted for roko:

```
claude --stream-json stdout
  │
  ├── {"type":"system", ...}           → AgentEvent::SystemInit
  ├── {"type":"assistant", "message":  → for each content block:
  │     {"content": [                      Text → AgentEvent::MessageDelta
  │       {"type":"text", "text":"..."     ToolUse → AgentEvent::ToolCall
  │       {"type":"tool_use", ...}
  │     ], "usage": {...}}}            → AgentEvent::TokenUsage
  ├── {"type":"tool", ...}             → AgentEvent::ToolOutput
  └── {"type":"result", ...}           → AgentEvent::TurnCompleted
                                          (session_id, total_cost, num_turns)
```

Each event flows through `agent_rx` to the event loop:

```rust
AgentEvent::MessageDelta { content, .. } => {
    run_state.agent_output.push_str(&content);
    state_hub.publish(DashboardEvent::AgentOutput {
        agent_id, content,
    });
}

AgentEvent::TokenUsage { input, output, cost, .. } => {
    run_state.tokens_in = input;
    run_state.tokens_out = output;
    run_state.cost_usd += cost_delta;
    state_hub.publish(DashboardEvent::TokenUpdate {
        agent_id, input, output, cost,
    });
}

AgentEvent::TurnCompleted { session_id, .. } => {
    run_state.agent_active = false;
    run_state.session_id = session_id;  // for --resume
    executor.apply_event(plan_id, &ExecutorEvent::AgentDone);
}
```

### 5.2 Gate Execution (Background)

```rust
// Spawn gate as tokio task, don't block event loop
let gate_tx = gate_tx.clone();
tokio::spawn(async move {
    let verdicts = roko_gate::run_rung(&signal, &ctx, rung, &inputs, &config).await;
    let _ = gate_tx.send(GateCompletion { plan_id, rung, verdicts });
});
```

Event loop receives:
```rust
Some(completion) = gate_rx.recv() => {
    let passed = completion.verdicts.iter().all(|v| v.passed);
    executor.apply_event(&completion.plan_id, if passed {
        &ExecutorEvent::GatePassed
    } else {
        &ExecutorEvent::GateFailed(format_failures(&completion.verdicts))
    });
    persist.record_gate_result(&completion);
    state_hub.publish(DashboardEvent::GateResult { ... });
}
```

### 5.3 Persistence (After Every Task)

```rust
// Called after each task completion or gate result
fn flush_task_checkpoint(&self) {
    // 1. Executor snapshot (atomic write)
    let snap = self.executor.snapshot();
    atomic_write(&self.paths.executor_json, &serde_json::to_string_pretty(&snap)?)?;

    // 2. Episode (append)
    self.episode_logger.record(&episode)?;  // already flushes per-write

    // 3. Efficiency event (append + flush)
    append_jsonl(&self.paths.efficiency_jsonl, &efficiency_event)?;

    // 4. Routing decision (append)
    append_jsonl(&self.paths.routing_jsonl, &routing_log)?;
}
```

### 5.4 Plan Loading (Simple, Correct)

```rust
pub fn load_plan(plan_dir: &Path) -> Result<Plan> {
    let tasks_path = plan_dir.join("tasks.toml");
    anyhow::ensure!(tasks_path.exists(), "no tasks.toml in {}", plan_dir.display());

    let tasks_file = TasksFile::parse(&tasks_path)?;

    // NEVER overwrite tasks.toml. NEVER scan for .md files.
    // The plan is exactly what the user authored.

    Ok(Plan {
        id: plan_dir.file_name().unwrap().to_string_lossy().to_string(),
        dir: plan_dir.to_path_buf(),
        tasks: tasks_file,
        skip_enrichment: tasks_file.meta.skip_enrichment,
    })
}
```

## 6. RunState (TUI Data Model)

Mori's `RunState` pattern — single struct, all TUI data:

```rust
pub struct RunState {
    // Agent state
    pub agent_active: bool,
    pub agent_role: Option<AgentRole>,
    pub agent_model: String,
    pub agent_output: String,           // accumulated MessageDelta text
    pub agent_session_id: Option<String>,

    // Token tracking (real-time from streaming)
    pub tokens_in: u64,
    pub tokens_out: u64,
    pub cost_usd: f64,
    pub cost_per_plan: HashMap<String, f64>,
    pub cost_per_task: HashMap<String, f64>,

    // Task progress
    pub current_plan: String,
    pub current_task: Option<String>,
    pub current_phase: String,
    pub tasks_total: usize,
    pub tasks_completed: usize,
    pub tasks_failed: usize,
    pub task_checklist: Vec<TaskRow>,

    // Gate output (streamed from background task)
    pub gate_output: String,
    pub gate_running: bool,

    // Iteration tracking
    pub iteration: u32,
    pub max_iterations: u32,

    // Cumulative
    pub total_cost_usd: f64,
    pub total_tokens_in: u64,
    pub total_tokens_out: u64,
    pub episodes: Vec<EpisodeSummary>,
}
```

Every field is updated inside the event loop. TUI renders `&RunState` each frame.

## 7. Event Types

```rust
/// Events from the agent subprocess (parsed from --stream-json).
pub enum AgentEvent {
    SystemInit { model: String, session_id: Option<String> },
    MessageDelta { content: String },
    ToolCall { name: String, id: String, input: Value },
    ToolOutput { content: String },
    TokenUsage { input: u64, output: u64, cost: Option<f64> },
    TurnCompleted { session_id: String, total_cost: Option<f64>, num_turns: u64 },
    Error { message: String },
    Exited { exit_code: Option<i32> },
}

/// Results from background gate tasks.
pub struct GateCompletion {
    pub plan_id: String,
    pub task_id: String,
    pub rung: u32,
    pub verdicts: Vec<Verdict>,
    pub output: String,         // gate stdout/stderr for TUI
    pub duration: Duration,
}
```

## 8. What Gets Rewritten vs Kept

### Rewrite (new `runner/` module)
- **Event loop** — new, based on mori's `sequential.rs` pattern
- **Agent streaming** — new, parse `--stream-json` line by line
- **Plan loading** — new, simple `tasks.toml` reader (no discovery)
- **Persistence** — new, flush after every task
- **TUI bridge** — new, publish DashboardEvents with real data
- **RunState** — new, single struct for all TUI data

### Keep (existing crate APIs, called from runner)
- `ParallelExecutor` from roko-orchestrator (pure state machine)
- `run_rung()` from roko-gate (gate execution)
- `LearningRuntime` from roko-learn (episodes, routing, efficiency)
- `ProcessSupervisor` from roko-runtime (PID tracking, kill)
- `StateHub` from roko-core (TUI event publishing)
- `SafetyLayer` from roko-agent (tool authorization)
- `Conductor` from roko-conductor (health monitoring)
- `RoleSystemPromptSpec` from roko-compose (prompt building)
- `CascadeRouter` from roko-learn (model selection)

### Deprecate (move to `orchestrate_legacy.rs`)
- `PlanRunner` god object (250 methods)
- `ReviewDriftReport` (2,833 lines — not needed for plan execution)
- `StaticCFactorSource` (644 lines — can be a separate file)
- `WatcherRunner` (564 lines — conductor handles this)
- `CrateFamiliarityTracker` (302 lines — enrichment-only)
- All enrichment pipeline code (not needed when `skip_enrichment = true`)
- All path resolution hacks (`plans_dir()` + fallbacks)

## 9. Related Rewrites

### 9.1 Agent Spawn Layer (`runner/agent_stream.rs`)

Current: `roko-agent` spawns claude and waits for exit. Batch result.

New: Spawn claude with `--output-format stream-json`, read stdout line by line,
parse `ClaudeStreamEvent` (mori's protocol.rs has the serde types), emit
`AgentEvent` variants through channel.

This replaces both the spawn code in `orchestrate.rs` AND the batch collection
in `roko-agent`. The agent crate's `Agent` trait and `TaskRunner` are bypassed
in favor of direct subprocess management (like mori does).

Process group isolation from mori:
- `setpgid(0, 0)` on spawn (new process group)
- Kill via `kill(-pgid, SIGTERM)` (kills all descendants)
- PID registry in `.roko/runtime/agent-pids.json`
- Orphan cleanup on startup

### 9.2 TUI Agent View

Current: TUI gets nothing during agent execution. Shows "no agent output yet."

New: TUI receives `DashboardEvent::AgentOutput` in real time via StateHub.
The Agents tab renders `run_state.agent_output` (accumulated text). Token
counters update per-message. Model name available from `SystemInit` event.

### 9.3 Enrichment (Optional, Separate)

Current: Enrichment is interleaved with execution, overwrites `tasks.toml`.

New: Enrichment is a **separate command** (`roko plan enrich <dir>`), not part
of `plan run`. If the user wants enrichment, they run it first. `plan run`
NEVER modifies `tasks.toml`. The `skip_enrichment` flag becomes unnecessary
because `plan run` doesn't enrich by default.

Enrichment artifacts go to a subdirectory (`<plan>/enrichment/`) not alongside
`tasks.toml`.

### 9.4 Plan Discovery

Current: `discover_plans()` scans for `.md` files, confuses artifacts with plans.

New: `plan run <dir>` takes a directory. If it has `tasks.toml`, it's a plan.
If it has subdirectories with `tasks.toml`, those are plans. No `.md` scanning.
Simple `find_plan_dirs()` (which already exists and works correctly).

## 10. Migration Strategy

### Phase A: Build runner alongside orchestrate.rs (1-2 days)
1. Create `crates/roko-cli/src/runner/` module
2. Implement plan loading, event loop, agent streaming, persistence
3. Wire to existing crate APIs
4. Test with `unified-migration-phase0` plan

### Phase B: Wire into CLI (hours)
1. Add `--runner v2` flag to `roko plan run`
2. Default to v2, `--runner legacy` falls back to orchestrate.rs
3. Verify TUI works with new runner

### Phase C: Deprecate orchestrate.rs (after validation)
1. Rename to `orchestrate_legacy.rs`
2. Extract useful pieces (CFactorSource, etc.) to separate files
3. Remove `--runner legacy` after confidence period

### Phase D: Align with unified spec (future)
1. Rename types (Engram→Signal, etc.) per Phase 1 kernel
2. Add Activity recording per-node (unified resumability)
3. Add Pulse-based lifecycle events
4. Replace event loop with Engine interpretation of TOML Graphs

## 11. Relationship to Unified Spec

This plan runner v2 is a **stepping stone** toward the unified Engine (spec §05):

| Unified Engine concept | Plan runner v2 equivalent |
|---|---|
| Engine interprets Graphs | Event loop dispatches ExecutorActions |
| Flow lifecycle Pulses | DashboardEvents on StateHub |
| Activity recording per-node | Episode + efficiency flush per-task |
| Workflow/Activity split | Executor (pure) / Runner (I/O) split |
| Failure strategies | Gate retry loop (iteration++) |
| Budget enforcement | Cost tracking per-plan per-task |
| Verify protocol | Gate pipeline (run_rung) |
| Agent type-state lifecycle | Agent spawn → stream → exit tracking |

The v2 runner won't implement the full Cell/Graph/Protocol stack, but it's
structured so the unified Engine can subsume it incrementally.

## 12. Success Criteria

A plan run of `unified-migration-phase0` with `--approval`:

1. **One plan discovered** (not 6 phantom plans)
2. **Enrichment skipped** (no artifacts written, no agents spawned for enrichment)
3. **Tasks execute** (M001, M002, ... dispatched to claude-sonnet-4-6)
4. **Streaming output** in TUI (text appears as agent types, not after exit)
5. **Token counters update** in real time (not "-" or "0k")
6. **Model name shown** (from SystemInit event, not efficiency.jsonl)
7. **Task titles shown** (from tasks.toml, not "plan plan")
8. **executor.json written** after each task (crash-safe)
9. **Episodes written** after each task (visible in `.roko/episodes.jsonl`)
10. **Efficiency events** flushed (visible in `.roko/learn/efficiency.jsonl`)
11. **Ctrl+C** kills agent + all descendants within 3 seconds
12. **Resume** from executor.json after crash (skip completed tasks)
13. **Gate output** visible in TUI (compile/test results stream)
14. **Cost tracking** accurate per-plan and per-task
