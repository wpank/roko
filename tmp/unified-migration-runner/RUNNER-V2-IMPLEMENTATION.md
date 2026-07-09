# Plan Runner v2 — Implementation Prompt

> **What this is**: A self-contained implementation guide. Each task (R001–R045) can be
> executed by an independent agent with no prior context. Tasks are ordered — complete
> them in sequence. Each task has scope, steps, files, and verification.
>
> **Goal**: Replace `orchestrate.rs` (21,478 lines) with a clean `runner/` module
> (~2,500 lines) that streams agent output, flushes persistence per-task, and shows
> real-time progress in the TUI. Then wire it as the default `roko plan run` path.
>
> **Spec**: `tmp/unified/22-PLAN-RUNNER-V2.md`
> **Dogfood issues**: `tmp/dogfood/00-INDEX.md`
> **Mori reference**: `/Users/will/dev/uniswap/bardo/apps/mori/src/app/sequential.rs`

---

## Project Context

### What roko is
Rust toolkit for building agents that build themselves. 18 crates, ~177K LOC.
The plan runner executes task plans: load tasks.toml → dispatch agents → run gates → persist results.

### What's broken
`crates/roko-cli/src/orchestrate.rs` is a 21K-line god object. The plan runner works
but is fragile:
- Agent output is batch-only (TUI sees nothing during execution)
- All persistence is buffered in memory (crash = total loss)
- Enrichment pipeline overwrites user files
- Plan discovery confuses enrichment artifacts with plans
- 250 methods on PlanRunner across 15K lines

### What we're building
A new `crates/roko-cli/src/runner/` module that:
1. Streams agent output via `--stream-json` (like mori)
2. Flushes episodes/efficiency/executor.json after every task
3. Publishes DashboardEvents to StateHub for TUI
4. Loads tasks.toml directly (no discovery magic)
5. Uses existing crate APIs (ParallelExecutor, run_rung, LearningRuntime, etc.)

### Key files (read these first)
```
Spec & design:
  tmp/unified/22-PLAN-RUNNER-V2.md              — full architecture spec
  tmp/dogfood/05-mori-vs-roko-agent-wiring.md   — streaming gap analysis
  tmp/dogfood/07-orchestrate-analysis.md         — structural analysis

Reference implementation (mori):
  /Users/will/dev/uniswap/bardo/apps/mori/src/app/sequential.rs   — event loop
  /Users/will/dev/uniswap/bardo/apps/mori/src/agent/connection.rs — agent spawn + stream parsing
  /Users/will/dev/uniswap/bardo/apps/mori/src/agent/protocol.rs   — ClaudeStreamEvent types
  /Users/will/dev/uniswap/bardo/apps/mori/src/app/events.rs       — agent event handling
  /Users/will/dev/uniswap/bardo/apps/mori/src/state/persistence.rs — crash recovery

Existing APIs to use:
  crates/roko-orchestrator/src/executor/mod.rs   — ParallelExecutor::tick(), apply_event()
  crates/roko-agent/src/claude_cli_agent.rs      — build_command() already uses --stream-json
  crates/roko-gate/src/rung_dispatch.rs          — run_rung() signature
  crates/roko-learn/src/runtime_feedback.rs      — LearningRuntime
  crates/roko-core/src/dashboard_snapshot.rs     — DashboardEvent variants
  crates/roko-core/src/state_hub.rs              — StateHub::publish()
  crates/roko-cli/src/task_parser.rs             — TasksFile::parse(), TaskDef, TaskMeta

Current code to replace:
  crates/roko-cli/src/orchestrate.rs             — 21,478 lines, the god object
  crates/roko-cli/src/main.rs:5770-5830          — plan run CLI handler
```

### Build & verify commands
```bash
cargo check --workspace                                    # Must pass after every task
cargo clippy --workspace --no-deps -- -D warnings          # Must pass before committing
cargo test --workspace                                     # Must pass before committing
cargo +nightly fmt --all                                   # Format
```

### Git conventions
- Branch: `wp-runner-v2`
- Commits: `runner(RXXX): <description>` (e.g., `runner(R003): implement AgentEvent types`)
- Never push to main directly

---

## Phase 1: Foundation — Types & Plan Loading (R001–R007)

### R001 — Create runner module scaffold

**Objective**: Create the `runner/` module directory with empty files and register it.

**Files to create**:
```
crates/roko-cli/src/runner/mod.rs
crates/roko-cli/src/runner/types.rs
crates/roko-cli/src/runner/plan_loader.rs
crates/roko-cli/src/runner/state.rs
crates/roko-cli/src/runner/agent_stream.rs
crates/roko-cli/src/runner/agent_events.rs
crates/roko-cli/src/runner/gate_dispatch.rs
crates/roko-cli/src/runner/persist.rs
crates/roko-cli/src/runner/tui_bridge.rs
crates/roko-cli/src/runner/event_loop.rs
```

**Steps**:
1. Create directory `crates/roko-cli/src/runner/`
2. Create `mod.rs` with `pub mod` declarations for each submodule
3. Each file gets a doc comment explaining its purpose and an empty struct/function placeholder
4. In `crates/roko-cli/src/lib.rs`, add `pub mod runner;`
5. Ensure `cargo check -p roko-cli` passes

**Verification**:
```bash
cargo check -p roko-cli
test -f crates/roko-cli/src/runner/mod.rs
```

---

### R002 — Define AgentEvent and GateCompletion types

**Objective**: Define the event types that flow through the runner's channels.

**File**: `crates/roko-cli/src/runner/types.rs`

**Context**: Mori uses `AgentEvent` variants parsed from claude's `--stream-json` output.
See `/Users/will/dev/uniswap/bardo/apps/mori/src/agent/events.rs` for reference.
The claude CLI `--output-format stream-json` emits one JSON object per line:
- `{"type":"system", ...}` — session init with model info
- `{"type":"assistant", "message":{"content":[...], "usage":{...}}}` — text/tool blocks + tokens
- `{"type":"tool", "content":"..."}` — tool result
- `{"type":"result", "subtype":"success"|"error", "session_id":"...", "total_cost_usd":...}` — final

**Steps**:
1. Define `AgentEvent` enum:
   ```rust
   pub enum AgentEvent {
       SystemInit { model: String, session_id: Option<String> },
       MessageDelta { content: String },
       ToolCall { name: String, id: String, input: serde_json::Value },
       ToolOutput { content: String },
       TokenUsage { input_tokens: u64, output_tokens: u64, cost_usd: Option<f64> },
       TurnCompleted { session_id: String, total_cost_usd: Option<f64>, num_turns: u64 },
       Error { message: String },
       Exited { exit_code: Option<i32> },
   }
   ```
2. Define `GateCompletion` struct (plan_id, task_id, rung, verdicts, output, duration)
3. Define `RunConfig` struct (workdir, plan_dir, model, timeout, max_retries, etc.)
4. Define `ClaudeStreamEvent` serde types for parsing JSON lines (see mori's `protocol.rs:130-196`)
5. Add necessary imports (serde, serde_json, Duration, etc.)

**Verification**:
```bash
cargo check -p roko-cli
```

---

### R003 — Implement plan_loader.rs

**Objective**: Load a plan from a directory. No discovery magic. tasks.toml is the source of truth.

**File**: `crates/roko-cli/src/runner/plan_loader.rs`

**Context**: The old code uses `discover_plans()` which scans for .md files and confuses
enrichment artifacts with plans. The new loader just reads tasks.toml from the directory.
Use `crates/roko-cli/src/task_parser.rs` which already has `TasksFile::parse()`.

**Steps**:
1. Define `Plan` struct:
   ```rust
   pub struct Plan {
       pub id: String,           // directory name
       pub dir: PathBuf,         // absolute path to plan directory
       pub tasks: TasksFile,     // parsed tasks.toml
   }
   ```
2. Implement `pub fn load_plan(plan_dir: &Path) -> Result<Plan>`:
   - Verify `plan_dir` is a directory
   - Verify `plan_dir/tasks.toml` exists
   - Parse via `TasksFile::parse()`
   - Set `id` from directory name
   - Return `Plan`
3. Implement `pub fn load_plans(dir: &Path) -> Result<Vec<Plan>>`:
   - If `dir/tasks.toml` exists: return `vec![load_plan(dir)?]`
   - Otherwise: scan subdirectories for `tasks.toml`, return all found
   - Sort by directory name
4. **NEVER** scan for `.md` files
5. **NEVER** modify tasks.toml

**Verification**:
```bash
cargo check -p roko-cli
# Manual: confirm .roko/plans/unified-migration-phase0/ loads correctly
```

---

### R004 — Implement RunState (TUI data model)

**Objective**: Single struct holding all state the TUI needs to render.

**File**: `crates/roko-cli/src/runner/state.rs`

**Context**: Mori uses `RunState` — a flat struct that the event loop mutates and the
TUI renders each frame. See `/Users/will/dev/uniswap/bardo/apps/mori/src/state/mod.rs:325-395`.

**Steps**:
1. Define `RunState`:
   ```rust
   pub struct RunState {
       // Agent
       pub agent_active: bool,
       pub agent_id: String,
       pub agent_role: String,
       pub agent_model: String,
       pub agent_output: String,          // accumulated text
       pub agent_session_id: Option<String>,
       pub agent_pid: Option<u32>,

       // Tokens (real-time from streaming)
       pub tokens_in: u64,
       pub tokens_out: u64,
       pub cost_usd: f64,

       // Task progress
       pub plan_id: String,
       pub current_task_id: Option<String>,
       pub current_task_title: Option<String>,
       pub phase: String,
       pub tasks_total: usize,
       pub tasks_completed: Vec<String>,
       pub tasks_failed: Vec<String>,

       // Gate
       pub gate_output: String,
       pub gate_running: bool,

       // Iteration
       pub iteration: u32,

       // Cumulative
       pub total_cost_usd: f64,
       pub total_tokens_in: u64,
       pub total_tokens_out: u64,
       pub started_at: Instant,
   }
   ```
2. Implement `RunState::new(plan_id: &str, tasks_total: usize) -> Self`
3. Implement `RunState::reset_for_task(&mut self, task_id: &str, title: &str)` — clears agent output, sets current task
4. Implement `RunState::task_completed(&mut self, task_id: &str)` — moves to completed list
5. Implement `RunState::task_failed(&mut self, task_id: &str)` — moves to failed list

**Verification**:
```bash
cargo check -p roko-cli
```

---

### R005 — Implement persist.rs (atomic writes)

**Objective**: Flush executor state, episodes, and efficiency events to disk after each task.

**File**: `crates/roko-cli/src/runner/persist.rs`

**Context**: Current code buffers everything in memory. Mori writes every 2 seconds via
`append_task_event_bg()`. We write after each task completion (less frequent but more reliable).
Use atomic writes (write to .tmp, then rename) like mori's `persistence.rs:253`.

**Steps**:
1. Define `PersistPaths`:
   ```rust
   pub struct PersistPaths {
       pub executor_json: PathBuf,     // .roko/state/executor.json
       pub episodes_jsonl: PathBuf,    // .roko/episodes.jsonl
       pub efficiency_jsonl: PathBuf,  // .roko/learn/efficiency.jsonl
       pub routing_jsonl: PathBuf,     // .roko/learn/routing.jsonl
       pub agent_pids_json: PathBuf,   // .roko/runtime/agent-pids.json
   }
   ```
2. Implement `PersistPaths::from_workdir(workdir: &Path) -> Self` — construct all paths, create parent dirs
3. Implement `pub fn atomic_write(path: &Path, content: &[u8]) -> Result<()>` — write to `path.tmp`, then `fs::rename`
4. Implement `pub fn append_jsonl(path: &Path, value: &impl Serialize) -> Result<()>` — open append, write JSON line, flush
5. Implement `pub fn save_executor_snapshot(paths: &PersistPaths, snapshot: &ExecutorSnapshot) -> Result<()>`
6. Implement `pub fn save_agent_pids(paths: &PersistPaths, pids: &[u32]) -> Result<()>`
7. Implement `pub fn cleanup_orphaned_agents(paths: &PersistPaths)` — read agent-pids.json on startup, kill stale PIDs
   (Reference: mori's `connection.rs:163` `cleanup_orphaned_agents()`)

**Verification**:
```bash
cargo check -p roko-cli
cargo test -p roko-cli -- runner::persist  # after writing tests
```

---

### R006 — Implement tui_bridge.rs

**Objective**: Publish DashboardEvents to StateHub so the TUI shows real-time progress.

**File**: `crates/roko-cli/src/runner/tui_bridge.rs`

**Context**: `roko-core::StateHub` accepts `DashboardEvent` variants and materializes a
`DashboardSnapshot` that the TUI reads via `watch::Receiver`. See
`crates/roko-core/src/dashboard_snapshot.rs` for all event variants.
`crates/roko-core/src/state_hub.rs:169` for the `snapshot()` method.

**Steps**:
1. Define `TuiBridge`:
   ```rust
   pub struct TuiBridge {
       hub: roko_core::SharedStateHub,
   }
   ```
2. Implement convenience methods that publish specific events:
   - `pub fn plan_started(&self, plan_id: &str)`
   - `pub fn task_started(&self, plan_id: &str, task_id: &str, title: &str, phase: &str)`
   - `pub fn task_completed(&self, plan_id: &str, task_id: &str, outcome: &str)`
   - `pub fn agent_spawned(&self, agent_id: &str, role: &str)`
   - `pub fn agent_output(&self, agent_id: &str, content: &str)` — for streaming text
   - `pub fn agent_completed(&self, agent_id: &str)`
   - `pub fn gate_result(&self, plan_id: &str, task_id: &str, gate: &str, passed: bool)`
   - `pub fn phase_transition(&self, plan_id: &str, from: &str, to: &str)`
3. Each method creates the appropriate `DashboardEvent` variant and calls `self.hub.publish()`

**Verification**:
```bash
cargo check -p roko-cli
```

---

### R007 — Write tests for plan_loader and persist

**Objective**: Unit tests for plan loading and persistence.

**File**: `crates/roko-cli/src/runner/plan_loader.rs` (test module) and `persist.rs` (test module)

**Steps**:
1. Test `load_plan()` with a valid plan directory (create temp dir with tasks.toml)
2. Test `load_plan()` fails gracefully when tasks.toml missing
3. Test `load_plans()` with a directory containing multiple plan subdirectories
4. Test `load_plans()` with a single plan directory (tasks.toml at root)
5. Test `atomic_write()` produces correct file content
6. Test `append_jsonl()` appends correctly and file is valid JSONL
7. Test `save_executor_snapshot()` writes valid JSON
8. Test `cleanup_orphaned_agents()` with no stale PIDs

**Verification**:
```bash
cargo test -p roko-cli -- runner
```

---

## Phase 2: Agent Streaming (R008–R014)

### R008 — Implement claude stream-json parser

**Objective**: Parse the claude CLI's `--output-format stream-json` stdout line by line.

**File**: `crates/roko-cli/src/runner/agent_stream.rs`

**Context**: Claude CLI emits one JSON object per stdout line. Mori's parsing is at
`/Users/will/dev/uniswap/bardo/apps/mori/src/agent/protocol.rs:130-196` and
`/Users/will/dev/uniswap/bardo/apps/mori/src/agent/connection.rs:2659-2731`.

The current roko-agent crate already builds the command with `--output-format stream-json`
at `crates/roko-agent/src/claude_cli_agent.rs:308-309`.

**Steps**:
1. Implement `fn parse_stream_line(line: &str) -> Option<AgentEvent>`:
   - Parse JSON via `serde_json::from_str::<ClaudeStreamEvent>(line)`
   - Match on `event.type`:
     - `"system"` → `AgentEvent::SystemInit { model, session_id }`
     - `"assistant"` → iterate `message.content` blocks:
       - `{"type":"text","text":"..."}` → `AgentEvent::MessageDelta { content }`
       - `{"type":"tool_use","name":"...","id":"...","input":{}}` → `AgentEvent::ToolCall { name, id, input }`
     - If `message.usage` present → `AgentEvent::TokenUsage { input_tokens, output_tokens, cost_usd }`
     - `"tool"` → `AgentEvent::ToolOutput { content }`
     - `"result"` → `AgentEvent::TurnCompleted { session_id, total_cost_usd, num_turns }`
   - Return `None` for unparseable lines (log warning, don't crash)

2. Implement `pub async fn spawn_agent(config: &AgentSpawnConfig, event_tx: mpsc::Sender<AgentEvent>) -> Result<AgentHandle>`:
   - Build claude command (reuse logic from `crates/roko-agent/src/claude_cli_agent.rs:302-340`)
   - Set `--output-format stream-json`
   - Set `--model`, `--max-turns`, `--append-system-prompt`
   - Call `pre_exec` to `setpgid(0, 0)` for process group isolation
   - Spawn with `tokio::process::Command`
   - Write prompt to stdin, close stdin
   - Spawn tokio task to read stdout line-by-line via `BufReader::new(stdout).lines()`
   - For each line: `parse_stream_line(line)` → `event_tx.send(event)`
   - When stdout closes: `event_tx.send(AgentEvent::Exited { exit_code })`

3. Define `AgentSpawnConfig`:
   ```rust
   pub struct AgentSpawnConfig {
       pub model: String,
       pub prompt: String,
       pub system_prompt: Option<String>,
       pub working_dir: PathBuf,
       pub timeout: Duration,
       pub max_turns: Option<u32>,
       pub resume_session: Option<String>,
       pub mcp_config: Option<PathBuf>,
       pub allowed_tools: Option<Vec<String>>,
       pub denied_tools: Option<Vec<String>>,
   }
   ```

4. Define `AgentHandle`:
   ```rust
   pub struct AgentHandle {
       pub pid: u32,
       pub child: tokio::process::Child,
       pub task: tokio::task::JoinHandle<()>,
   }
   ```

5. Implement `AgentHandle::kill(&mut self)` — kill process group via `kill(-pgid, SIGTERM)`,
   wait 3 seconds, then `SIGKILL` if needed. Reference: mori's `connection.rs:45-117`.

**Verification**:
```bash
cargo check -p roko-cli
```

---

### R009 — Implement agent_events.rs (event handler)

**Objective**: Handle AgentEvent variants to update RunState and publish to TUI.

**File**: `crates/roko-cli/src/runner/agent_events.rs`

**Context**: Mori's event handler is at `/Users/will/dev/uniswap/bardo/apps/mori/src/app/events.rs:17-389`.

**Steps**:
1. Implement `pub fn handle_agent_event(event: AgentEvent, state: &mut RunState, tui: &TuiBridge)`:
   ```rust
   match event {
       AgentEvent::SystemInit { model, session_id } => {
           state.agent_model = model;
           state.agent_session_id = session_id;
       }
       AgentEvent::MessageDelta { content } => {
           state.agent_output.push_str(&content);
           tui.agent_output(&state.agent_id, &content);
       }
       AgentEvent::ToolCall { name, .. } => {
           state.agent_output.push_str(&format!("\n[tool: {name}]\n"));
           // Optionally publish DashboardEvent
       }
       AgentEvent::ToolOutput { content } => {
           // Truncate large outputs (>4KB) like mori does
           let truncated = if content.len() > 4096 {
               format!("{}...[truncated]", &content[..4096])
           } else {
               content
           };
           state.agent_output.push_str(&truncated);
       }
       AgentEvent::TokenUsage { input_tokens, output_tokens, cost_usd } => {
           state.tokens_in = input_tokens;
           state.tokens_out = output_tokens;
           if let Some(cost) = cost_usd {
               state.cost_usd = cost;
           }
           // Publish token update to TUI
       }
       AgentEvent::TurnCompleted { session_id, total_cost_usd, num_turns } => {
           state.agent_active = false;
           state.agent_session_id = Some(session_id);
           if let Some(cost) = total_cost_usd {
               state.total_cost_usd += cost;
           }
       }
       AgentEvent::Error { message } => {
           state.agent_output.push_str(&format!("\n[ERROR: {message}]\n"));
       }
       AgentEvent::Exited { exit_code } => {
           state.agent_active = false;
           state.agent_pid = None;
       }
   }
   ```

**Verification**:
```bash
cargo check -p roko-cli
```

---

### R010 — Implement gate_dispatch.rs

**Objective**: Run gates as background tokio tasks, send results through a channel.

**File**: `crates/roko-cli/src/runner/gate_dispatch.rs`

**Context**: Gates must not block the event loop. Spawn as tokio tasks. Results come back
through `gate_rx` channel. The gate API is at `crates/roko-gate/src/rung_dispatch.rs:83-89`.

**Steps**:
1. Implement `pub fn spawn_gate(plan_id: &str, task_id: &str, rung: u32, workdir: &Path, gate_tx: mpsc::Sender<GateCompletion>)`:
   - Create `Engram` from the changed files (or empty if none)
   - Create `Context::now()`
   - Create `RungExecutionInputs::default()` and `RungExecutionConfig` with source_roots
   - Spawn tokio task:
     ```rust
     tokio::spawn(async move {
         let verdicts = roko_gate::rung_dispatch::run_rung(&signal, &ctx, rung, &inputs, &config).await;
         let _ = gate_tx.send(GateCompletion { plan_id, task_id, rung, verdicts, .. }).await;
     });
     ```
2. Keep it simple — just compile gate (rung 0) and clippy (rung 1) for now.
   Higher rungs can be added later.

**Verification**:
```bash
cargo check -p roko-cli
```

---

### R011 — Write tests for stream parser

**Objective**: Test that `parse_stream_line` correctly parses all claude stream-json event types.

**File**: `crates/roko-cli/src/runner/agent_stream.rs` (test module)

**Steps**:
1. Test `system` event parsing → `AgentEvent::SystemInit`
2. Test `assistant` event with text content → `AgentEvent::MessageDelta`
3. Test `assistant` event with tool_use → `AgentEvent::ToolCall`
4. Test `assistant` event with usage → `AgentEvent::TokenUsage`
5. Test `tool` event → `AgentEvent::ToolOutput`
6. Test `result` event → `AgentEvent::TurnCompleted`
7. Test malformed JSON → `None` (no crash)
8. Test empty line → `None`
9. Use real claude output samples if available (check `.roko/` for any cached agent output)

**Verification**:
```bash
cargo test -p roko-cli -- runner::agent_stream
```

---

### R012 — Implement prompt building for task dispatch

**Objective**: Build the agent prompt from a TaskDef, including system prompt.

**File**: `crates/roko-cli/src/runner/agent_events.rs` (or new `prompt.rs`)

**Context**: The existing `crates/roko-compose/src/` has `RoleSystemPromptSpec` for
9-layer system prompts. For v2, start simple: role description + task description +
files list + acceptance criteria. Richer prompts can use the compose crate later.

**Steps**:
1. Implement `pub fn build_task_prompt(task: &TaskDef, plan_id: &str, workdir: &Path) -> String`:
   - Include task title, description, files, acceptance criteria
   - Include max_loc if set
   - Include verify steps as instructions
   - Keep it under 4K tokens (simple, focused)
2. Implement `pub fn build_system_prompt(task: &TaskDef, plan_id: &str) -> String`:
   - Role-based identity (from task.role)
   - Working directory info
   - Constraints (max_loc, allowed/denied tools)
   - Gate criteria preview

**Verification**:
```bash
cargo check -p roko-cli
```

---

### R013 — Implement process group management

**Objective**: Robust subprocess lifecycle — spawn in new process group, kill all descendants.

**File**: `crates/roko-cli/src/runner/agent_stream.rs` (extend from R008)

**Context**: Mori's process group handling is at `/Users/will/dev/uniswap/bardo/apps/mori/src/agent/connection.rs:45-157`.
Each agent runs as its own process group leader. Kill sends SIGTERM to the entire group.
PID registry tracks all spawned PIDs for orphan cleanup.

**Steps**:
1. In `spawn_agent()`, add `pre_exec` hook:
   ```rust
   unsafe {
       cmd.pre_exec(|| {
           libc::setpgid(0, 0);  // new process group
           Ok(())
       });
   }
   ```
2. In `AgentHandle::kill()`:
   - Get pgid from pid
   - `libc::kill(-(pgid as i32), libc::SIGTERM)` — kills entire group
   - Wait up to 3 seconds with timeout
   - If still alive: `libc::kill(-(pgid as i32), libc::SIGKILL)`
3. Track PIDs in PersistPaths.agent_pids_json via `save_agent_pids()`
4. On startup: `cleanup_orphaned_agents()` reads pids, kills any still alive

**Verification**:
```bash
cargo check -p roko-cli
```

---

### R014 — Integration test: spawn agent, parse stream, collect events

**Objective**: End-to-end test of agent spawn → stream parsing → event collection.

**File**: `crates/roko-cli/src/runner/agent_stream.rs` (test module, `#[tokio::test]`)

**Steps**:
1. Skip if `claude` binary not found (use `which claude`)
2. Spawn agent with a trivial prompt ("print hello world")
3. Collect events via `agent_rx`
4. Assert: received `SystemInit` with model name
5. Assert: received at least one `MessageDelta`
6. Assert: received `TurnCompleted` with session_id
7. Assert: received `Exited` with exit_code 0
8. Mark test as `#[ignore]` (requires live claude binary)

**Verification**:
```bash
cargo test -p roko-cli -- runner::agent_stream --ignored  # manual run
```

---

## Phase 3: Event Loop & Execution (R015–R022)

### R015 — Implement the core event loop

**Objective**: The main `tokio::select!` loop that drives plan execution.

**File**: `crates/roko-cli/src/runner/event_loop.rs`

**Context**: Mori's event loop is at `/Users/will/dev/uniswap/bardo/apps/mori/src/app/sequential.rs:451-730`.
Uses `tokio::select!` over agent_rx, gate_rx, and tick.

**Steps**:
1. Define the public entry point:
   ```rust
   pub async fn run(
       plan: Plan,
       workdir: PathBuf,
       config: RunConfig,
       state_hub: roko_core::SharedStateHub,
       cancel: CancellationToken,
   ) -> Result<RunReport>
   ```
2. Initialize:
   - `RunState::new(plan.id, plan.tasks.tasks.len())`
   - `PersistPaths::from_workdir(&workdir)`
   - `TuiBridge::new(state_hub)`
   - `ParallelExecutor::new(executor_config)` — add the plan
   - Agent channel: `mpsc::channel::<AgentEvent>(256)`
   - Gate channel: `mpsc::channel::<GateCompletion>(16)`
   - Tick interval: `tokio::time::interval(Duration::from_millis(100))`
   - Flush interval: `tokio::time::interval(Duration::from_secs(2))`
3. Publish `DashboardEvent::PlanStarted`
4. Main loop:
   ```rust
   loop {
       tokio::select! {
           Some(event) = agent_rx.recv() => {
               handle_agent_event(event, &mut run_state, &tui);
               if matches!(event, AgentEvent::TurnCompleted { .. } | AgentEvent::Exited { .. }) {
                   // Agent done — feed executor, maybe start gate
                   executor.apply_event(&plan_id, &ExecutorEvent::ImplementationDone);
               }
           }
           Some(completion) = gate_rx.recv() => {
               let passed = completion.verdicts.iter().all(|v| v.passed);
               executor.apply_event(&plan_id, if passed {
                   &ExecutorEvent::GatePassed
               } else {
                   &ExecutorEvent::GateFailed(...)
               });
               // Persist after gate
               persist::save_executor_snapshot(&paths, &executor.snapshot())?;
           }
           _ = tick.tick() => {
               // Drive executor state machine
               for action in executor.tick() {
                   dispatch_action(&action, &mut run_state, &config, &agent_tx, &gate_tx, &tui, &paths).await?;
               }
           }
           _ = flush.tick() => {
               persist::save_executor_snapshot(&paths, &executor.snapshot())?;
           }
           _ = cancel.cancelled() => {
               // Graceful shutdown
               if let Some(handle) = agent_handle.as_mut() {
                   handle.kill().await;
               }
               persist::save_executor_snapshot(&paths, &executor.snapshot())?;
               break;
           }
       }

       // Check completion
       if executor.all_plans_terminal() {
           break;
       }
   }
   ```

**Verification**:
```bash
cargo check -p roko-cli
```

---

### R016 — Implement action dispatcher

**Objective**: Dispatch `ExecutorAction` variants to real systems.

**File**: `crates/roko-cli/src/runner/event_loop.rs` (private function)

**Steps**:
1. Implement `async fn dispatch_action(action, state, config, agent_tx, gate_tx, tui, paths)`:
   ```rust
   match action {
       ExecutorAction::SpawnAgent { plan_id, role, task } => {
           // Look up TaskDef from plan
           // Build prompt from task
           // Build AgentSpawnConfig
           // Call spawn_agent() → AgentHandle
           // Store handle for lifecycle management
           // Update RunState
           // Publish AgentSpawned to TUI
           tui.agent_spawned(&format!("{plan_id}:{task}"), &format!("{role:?}"));
           tui.task_started(&plan_id, &task, &task_title, "implementing");
       }
       ExecutorAction::RunGate { plan_id, rung } => {
           gate_dispatch::spawn_gate(&plan_id, &current_task, rung, &workdir, gate_tx.clone());
           state.gate_running = true;
       }
       ExecutorAction::CompletePlan { plan_id } => {
           tui.plan_completed(&plan_id, true);
       }
       ExecutorAction::FailPlan { plan_id } => {
           tui.plan_completed(&plan_id, false);
       }
       // Handle other actions (MergeBranch, Reorder, etc.) as needed
       _ => {
           tracing::warn!("unhandled executor action: {action:?}");
       }
   }
   ```

**Verification**:
```bash
cargo check -p roko-cli
```

---

### R017 — Handle skip_enrichment in executor flow

**Objective**: When `skip_enrichment = true`, the executor should skip the enriching phase.

**File**: `crates/roko-cli/src/runner/event_loop.rs`

**Context**: The `ParallelExecutor` starts plans in `Enriching` phase by default. When
`skip_enrichment` is set, we need to immediately transition to `Implementing`.

**Steps**:
1. After adding a plan to the executor, check `plan.tasks.meta.skip_enrichment`
2. If true, immediately: `executor.apply_event(&plan_id, &ExecutorEvent::EnrichmentDone)`
3. This transitions the plan to `Implementing` without spawning any enrichment agents
4. The executor's next `tick()` will emit `SpawnAgent { role: Implementer, task: <first_ready> }`

**Verification**:
```bash
cargo check -p roko-cli
```

---

### R018 — Wire runner into CLI (replace orchestrate.rs path)

**Objective**: Make `roko plan run <dir>` use the new runner.

**File**: `crates/roko-cli/src/main.rs` (around line 5770-5830)

**Context**: Currently `main.rs` creates a `PlanRunner::from_plans_dir()` and calls `runner.run()`.
Replace this with the new `runner::event_loop::run()`.

**Steps**:
1. In the `plan run` handler (around line 5770), replace:
   ```rust
   // OLD:
   let mut runner = PlanRunner::from_plans_dir(&plans_dir, &wd, config, metrics, cli.no_replan).await?;
   runner.run(&plans_dir).await?

   // NEW:
   let plans = runner::plan_loader::load_plans(&plans_dir)?;
   let config = runner::types::RunConfig::from_cli(&cli, &wd)?;
   let report = runner::event_loop::run(plans[0].clone(), wd, config, state_hub, cancel).await?;
   ```
2. Keep the `--approval` TUI thread spawn (lines 5795-5820) — it shares StateHub
3. Keep `orchestrate.rs` in the crate but don't call it from `plan run`
4. The old path remains accessible for other commands that still use `PlanRunner`

**Verification**:
```bash
cargo check -p roko-cli
cargo run -p roko-cli -- plan run .roko/plans/unified-migration-phase0 --approval  # smoke test
```

---

### R019 — Handle Ctrl+C shutdown gracefully

**Objective**: Clean shutdown on SIGINT/SIGTERM — kill agents, flush state, exit.

**File**: `crates/roko-cli/src/runner/event_loop.rs`

**Context**: The event loop already has a `cancel.cancelled()` branch. Wire it to signal handlers.

**Steps**:
1. Before entering the event loop, register signal handlers:
   ```rust
   let cancel = CancellationToken::new();
   let cancel_for_signal = cancel.clone();
   tokio::spawn(async move {
       tokio::signal::ctrl_c().await.ok();
       cancel_for_signal.cancel();
   });
   ```
2. In the `cancel.cancelled()` branch:
   - Kill current agent (if any) via `AgentHandle::kill()`
   - Save executor snapshot (completed tasks preserved)
   - Flush any pending episodes/efficiency events
   - Break from loop
3. Return partial `RunReport` with `completed = false`

**Verification**:
```bash
cargo check -p roko-cli
# Manual: start a plan run, press Ctrl+C, verify executor.json exists
```

---

### R020 — Handle gate failure retry loop

**Objective**: When a gate fails, retry the task with error feedback.

**File**: `crates/roko-cli/src/runner/event_loop.rs`

**Context**: Mori's retry logic is in `/Users/will/dev/uniswap/bardo/apps/mori/src/app/gates.rs:194-201`.
On compile failure: increment iteration, re-spawn agent with error digest.

**Steps**:
1. When `gate_rx` receives a failure:
   - `executor.apply_event(&plan_id, &ExecutorEvent::GateFailed(error_msg))`
   - The executor's state machine handles retry logic internally
   - On next `tick()`, it emits `SpawnAgent` again with the same task
2. In `dispatch_action` for `SpawnAgent`:
   - Check if this is a retry (iteration > 0)
   - If retry: prepend gate error output to the prompt as context
   - Clear agent output in RunState
3. Limit retries via `config.max_retries` (default 2)

**Verification**:
```bash
cargo check -p roko-cli
```

---

### R021 — Implement RunReport (post-execution summary)

**Objective**: Return a structured report after plan execution.

**File**: `crates/roko-cli/src/runner/types.rs`

**Steps**:
1. Define `RunReport`:
   ```rust
   pub struct RunReport {
       pub plan_id: String,
       pub completed: bool,
       pub tasks_total: usize,
       pub tasks_completed: usize,
       pub tasks_failed: usize,
       pub total_cost_usd: f64,
       pub total_tokens_in: u64,
       pub total_tokens_out: u64,
       pub duration: Duration,
       pub iterations: u32,
   }
   ```
2. Build from `RunState` at end of event loop
3. Print summary to stderr after TUI exits

**Verification**:
```bash
cargo check -p roko-cli
```

---

### R022 — End-to-end test with unified-migration-phase0

**Objective**: Run the actual plan and verify everything works.

**Steps**:
1. Ensure `.roko/plans/unified-migration-phase0/tasks.toml` exists with `skip_enrichment = true`
2. Run: `env -u CLAUDECODE cargo run -p roko-cli -- plan run .roko/plans/unified-migration-phase0 --approval`
3. Verify checklist:
   - [ ] Only 1 plan discovered (not 6)
   - [ ] Enrichment skipped (no enrichment agents spawned)
   - [ ] First task dispatches (M001 or similar)
   - [ ] Agent output streams in TUI (text appears as agent types)
   - [ ] Token counters update in real time
   - [ ] Model name shown (from SystemInit event)
   - [ ] executor.json written after task completes
   - [ ] episodes.jsonl updated after task completes
   - [ ] Ctrl+C kills agent within 3 seconds
   - [ ] After Ctrl+C: executor.json has completed tasks

**Verification**: Manual — run the plan and observe.

---

## Phase 4: Cleanup & Dogfood Fixes (R023–R030)

### R023 — Move orchestrate.rs to orchestrate_legacy.rs

**Objective**: Rename the old code to clearly mark it as legacy.

**Steps**:
1. `mv crates/roko-cli/src/orchestrate.rs crates/roko-cli/src/orchestrate_legacy.rs`
2. Update `crates/roko-cli/src/lib.rs`: `mod orchestrate` → `mod orchestrate_legacy`
3. Update all imports across the crate that reference `orchestrate::`
4. Ensure `cargo check --workspace` passes
5. Other commands (like `roko run`, `roko agent`, etc.) that still use PlanRunner continue to work

**Verification**:
```bash
cargo check --workspace
cargo clippy --workspace --no-deps -- -D warnings
```

---

### R024 — Episode recording in event loop

**Objective**: Record an Episode after each task completion.

**File**: `crates/roko-cli/src/runner/event_loop.rs`

**Context**: Use `roko-learn::EpisodeLogger::record()`. Episodes need: agent_id, model,
task_id, success, cost, duration, gate verdicts.

**Steps**:
1. After agent completes + gate passes:
   - Build `Episode` from RunState (model, tokens, cost, duration)
   - Add gate verdicts
   - Call `episode_logger.record(&episode)`
   - Flush to `episodes.jsonl`
   - Publish `DashboardEvent::EpisodeRecorded`

**Verification**:
```bash
cargo check -p roko-cli
# Manual: after task completes, check .roko/episodes.jsonl has new entry
```

---

### R025 — Efficiency event recording

**Objective**: Record per-task efficiency events for cost/performance tracking.

**File**: `crates/roko-cli/src/runner/event_loop.rs`

**Steps**:
1. After each task completes, append to `.roko/learn/efficiency.jsonl`:
   ```json
   {
     "agent_id": "unified-migration-phase0:M001",
     "model": "claude-sonnet-4-6",
     "task_id": "M001",
     "role": "Implementer",
     "input_tokens": 12345,
     "output_tokens": 6789,
     "cost_usd": 0.045,
     "wall_ms": 45000,
     "gate_passed": true,
     "timestamp": "2026-04-26T..."
   }
   ```
2. Use `persist::append_jsonl()` with flush

**Verification**:
```bash
cargo check -p roko-cli
# Manual: after task, check .roko/learn/efficiency.jsonl
```

---

### R026 — Resume from executor.json

**Objective**: On restart, skip completed tasks by loading the executor snapshot.

**File**: `crates/roko-cli/src/runner/event_loop.rs`

**Steps**:
1. At startup, check if `executor.json` exists
2. If exists, load `ExecutorSnapshot` and create executor via `ParallelExecutor::from_snapshot()`
3. The executor's plan state tracks which tasks are completed
4. On next `tick()`, it only emits actions for remaining tasks
5. Print: "Resuming from checkpoint: N/M tasks completed"

**Verification**:
```bash
cargo check -p roko-cli
# Manual: run plan, Ctrl+C after 1 task, restart, verify it skips completed task
```

---

### R027 — Routing log recording

**Objective**: Record model routing decisions for observability.

**File**: `crates/roko-cli/src/runner/event_loop.rs`

**Steps**:
1. When dispatching an agent (in `dispatch_action` for `SpawnAgent`):
   - Record the model selection decision
   - Append to `.roko/learn/routing.jsonl`
   - Include: requested_model, selected_model, routing_reason, task_id, role

**Verification**:
```bash
cargo check -p roko-cli
```

---

### R028 — Fix TUI log bar (tracing vs raw terminal)

**Objective**: Route tracing output through the TUI instead of stderr.

**Context**: Currently tracing writes to stderr while the TUI has raw mode active,
causing garbled output. Mori routes all logs through a ring buffer in the TUI.
The existing TUI at `crates/roko-cli/src/tui/` has a Logs tab.

**Steps**:
1. Create a custom `tracing_subscriber::Layer` that captures log events into a bounded ring buffer
2. Publish log events as `DashboardEvent` variants (or directly to a `Vec<String>` shared with TUI)
3. In the Logs tab: render from the ring buffer instead of file
4. Suppress stderr output when TUI is active

This is a larger change — if time-constrained, skip and address post-v2.

**Verification**:
```bash
cargo check -p roko-cli
# Manual: TUI logs tab shows clean, non-garbled entries
```

---

### R029 — Add missing HTTP endpoints (from dogfood)

**Objective**: Wire endpoints identified in `tmp/dogfood/01-endpoint-audit.md`.

**Context**: These are in `crates/roko-serve/src/routes/`. Some were already added
(health, statehub/snapshot). Remaining:
- `GET /api/plans/:id` — individual plan state
- `GET /api/plans/:id/tasks` — plan task list
- `GET /api/knowledge` — neuro store entries
- `GET /api/learn/router` — cascade router state

**Steps**:
1. `plans/:id` — in `routes/plans.rs`, add handler that reads executor.json or state_hub
2. `plans/:id/tasks` — return task list from tasks.toml
3. `knowledge` — in `routes/neuro.rs`, query the knowledge store
4. `learn/router` — in `routes/learning.rs`, return cascade router snapshot

**Verification**:
```bash
cargo check -p roko-serve
curl http://localhost:6677/api/plans/unified-migration-phase0
```

---

### R030 — Final verification pass

**Objective**: Run the full dogfood checklist and verify everything works.

**Steps**:
1. Clean build: `cargo build --workspace`
2. Clippy: `cargo clippy --workspace --no-deps -- -D warnings`
3. Tests: `cargo test --workspace`
4. Format: `cargo +nightly fmt --all`
5. Run plan:
   ```bash
   env -u CLAUDECODE cargo run -p roko-cli -- plan run .roko/plans/unified-migration-phase0 --approval
   ```
6. Check each dogfood item from `tmp/dogfood/00-INDEX.md`:
   - [ ] 1 plan discovered
   - [ ] Enrichment skipped
   - [ ] Tasks execute with streaming output
   - [ ] Token counters update
   - [ ] Model shown
   - [ ] Task titles shown (not "plan plan")
   - [ ] executor.json written per-task
   - [ ] episodes.jsonl updated per-task
   - [ ] efficiency.jsonl updated per-task
   - [ ] Ctrl+C clean shutdown
   - [ ] Resume from checkpoint works
   - [ ] TUI log bar readable (or at least not garbled)
   - [ ] Memory reasonable (<500MB after 10 min)

---

## Phase 0 Prep Tasks (R031–R038)

These come from `tmp/unified-migration/01-PHASE-0-PREP.md`. They prepare the
codebase for the kernel rename but are independent of the runner rewrite.

### R031 — Wire ExtensionChain into agent dispatch
- Source: `tmp/unified-migration/01-PHASE-0-PREP.md` §0.1 item 1
- Code: `crates/roko-agent/src/extensions/` (539 LOC, built but never called)
- Wire pre/post-inference hooks into the runner's agent dispatch path
- **Verify**: add a no-op extension, confirm hooks fire during plan run

### R032 — Wire KnowledgeAdmissionController
- Source: §0.1 item 2
- Code: `crates/roko-neuro/src/admission.rs` (1,285 LOC)
- Wire into knowledge store's `put()` path
- **Verify**: post low-quality entry, confirm rejected

### R033 — Wire ContextualBanditPolicy for routing
- Source: §0.1 item 3
- Code: `crates/roko-learn/src/bandits/` (1,372 LOC)
- Wire into CascadeRouter's selection path
- **Verify**: bandit feedback updates after agent dispatch

### R034 — Audit ConnectorRegistry + FeedRegistry
- Source: §0.1 item 4
- Code: `crates/roko-runtime/src/` (493 LOC, empty registries)
- Wire or delete if unified Connect protocol supersedes
- **Verify**: `cargo clippy` clean

### R035 — Fix token accounting in gateway
- Source: §0.2 item 1
- Code: `crates/roko-serve/src/routes/gateway.rs:269-270`
- Use real token counts from LLM responses instead of `len/4` heuristic

### R036 — Parallelize batch requests in gateway
- Source: §0.2 item 2
- Code: `crates/roko-serve/src/routes/gateway.rs:437-487`
- Use `JoinSet` for parallelism

### R037 — Create module stubs for unified types
- Source: §0.3
- Create empty modules in `crates/roko-core/src/`:
  - `signal.rs` (will hold renamed Engram)
  - `pulse.rs` (new Pulse struct)
  - `cell.rs` (new Cell trait)
  - `bus.rs` (promoted from roko-runtime)
- **Verify**: `cargo check -p roko-core`

### R038 — Baseline verification snapshot
- Source: §0.4
- Run full workspace build + test + clippy
- Record baseline in `tmp/unified-migration-runner/baseline.json`
- This is the regression baseline for all subsequent work

---

## Dogfood-Specific Fixes (R039–R045)

These are remaining issues from `tmp/dogfood/00-INDEX.md` not covered by the runner rewrite.

### R039 — TOML parse: strip markdown fences (#8)
- Code: `crates/roko-compose/src/enrichment/` (wherever TOML is parsed from LLM output)
- LLM wraps TOML in ````toml` fences. Strip before parsing.
- Check if `strip_code_fences` already exists (search codebase)
- **Verify**: enrichment verify step parses LLM output correctly

### R040 — Enrichment timeouts configurable (#9)
- Code: `crates/roko-compose/src/enrichment/`
- Current hardcoded 120s timeout. Make configurable via `[executor]` config.
- Default to 300s (5 min) for enrichment agents.

### R041 — No codex backend (#4)
- Code: `crates/roko-agent/src/`
- Add codex as a provider backend (similar to claude_cli, ollama, etc.)
- Wire into provider adapter so `model = "gpt-5.4"` routes to codex

### R042 — Plan detail routes (#11)
- Already partially covered by R029
- `GET /api/plans/:id` — plan metadata + current phase
- `GET /api/plans/:id/tasks` — task list with status

### R043 — Knowledge endpoint (#12)
- Code: `crates/roko-serve/src/routes/neuro.rs`
- `GET /api/knowledge` — list entries
- `GET /api/knowledge?query=<topic>` — search by topic

### R044 — Executor state endpoint (#13)
- Code: `crates/roko-serve/src/routes/status.rs`
- `GET /api/executor/state` — return current executor.json contents

### R045 — Worktree isolation for plan runner (#16)
- In the new runner: optionally create a git worktree for each plan run
- Use `--use-worktrees` flag or `executor.use_worktrees = true` in config
- Prevents agent writes from polluting the main repo
- Reference: mori uses worktrees per-plan in parallel mode

---

## Task Dependency Graph

```
R001 (scaffold)
  ├── R002 (types) ──────────────────────────┐
  ├── R003 (plan_loader) ──── R007 (tests)   │
  ├── R004 (RunState)                         │
  ├── R005 (persist) ──────── R007 (tests)   │
  └── R006 (tui_bridge)                       │
                                              │
R008 (stream parser) ── R011 (tests) ── R014 (integration test)
R009 (agent events)                           │
R010 (gate dispatch)                          │
R012 (prompt building)                        │
R013 (process groups)                         │
                                              │
R015 (event loop) ◄───────────────────────────┘
  ├── R016 (action dispatcher)
  ├── R017 (skip_enrichment)
  ├── R019 (Ctrl+C)
  ├── R020 (gate retry)
  └── R021 (RunReport)

R018 (wire into CLI) ◄── R015
R022 (end-to-end test) ◄── R018

R023 (rename legacy) ◄── R022
R024 (episodes) ◄── R022
R025 (efficiency) ◄── R022
R026 (resume) ◄── R022
R027 (routing log) ◄── R022
R028 (TUI log bar) — independent
R029 (HTTP endpoints) — independent
R030 (final verification) ◄── all above

R031–R038 (Phase 0 prep) — independent, can run in parallel
R039–R045 (dogfood fixes) — independent, can run after R022
```

---

## Quick Reference: Crate API Signatures

```rust
// ParallelExecutor (roko-orchestrator)
pub fn tick(&self) -> Vec<ExecutorAction>
pub fn apply_event(&mut self, plan_id: &str, event: &ExecutorEvent) -> Result<PlanPhase, TransitionError>
pub fn snapshot(&self) -> ExecutorSnapshot
pub fn from_snapshot(config: ExecutorConfig, snapshot: ExecutorSnapshot) -> Self
pub fn add_plan(&mut self, plan_state: PlanState) -> bool

// Gate (roko-gate)
pub async fn run_rung(signal: &Engram, ctx: &Context, rung: u32, inputs: &RungExecutionInputs, config: &RungExecutionConfig) -> Vec<Verdict>

// StateHub (roko-core)
pub fn publish(&self, event: DashboardEvent)
pub fn current_snapshot(&self) -> DashboardSnapshot
pub fn snapshot(&self) -> watch::Receiver<DashboardSnapshot>

// TasksFile (roko-cli task_parser)
pub fn parse(path: &Path) -> Result<Self>
pub fn ready_tasks(&self, completed: &[String]) -> Vec<&TaskDef>
pub fn parallel_groups(&self) -> Vec<Vec<&TaskDef>>
```
