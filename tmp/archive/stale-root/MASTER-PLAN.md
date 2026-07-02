# Roko Master Implementation Plan

> **Created**: 2026-04-08
>
> **Single source of truth** for ALL remaining work. Supersedes:
> `MASTER-REMAINING-WORK.md`, `PROMPT-EXECUTOR-PARITY.md`, plans 07–10.
>
> **Structure**: 6 tiers ordered by dependency and impact. Tier 1 (Mori Parity)
> completes first. Each tier's sections are ordered by dependency within that tier.
>
> **Status key**: ✅ DONE — verified working. 🔌 WIRED — called from runtime.
> 🏗️ BUILT — code exists, not called. ❌ MISSING — not implemented.
>
> **THE RULE**: Do NOT simplify. Do NOT skip items. Do NOT stub with println.
> Every handler must do real work. Verify each item before marking done.
>
> **Detailed specs**: Phase files in `implementation-plans/11-sections/` contain
> full Rust code examples. This doc is the tracker; those are the blueprints.

---

## Status Overview

| Tier | Name | Sections | Items | Done | Remaining |
|------|------|----------|-------|------|-----------|
| 1 | Mori Parity | 1A–1J | 129 | ~15 | ~114 |
| 2 | Agent Platform Foundation | 2A–2D | 81 | 0 | 81 |
| 3 | Agent Templates & Events | 3A–3C | 28 | 0 | 28 |
| 4 | Daemon & Multi-Repo | 4A–4D | 40 | ~12 | ~28 |
| 5 | Cognitive Layer | 5A–5F | 92 | ~62 | ~30 |
| 6 | Chain Layer (deferred) | 6A–6G | 68 | 0 | 68 |
| CC | PRD Autonomous Workflow | — | 12 | 0 | 12 |
| | **TOTAL** | | **450** | **~89** | **~361** |

---

## Dependency Graph

```
Tier 1 (Mori Parity)
├── 1A Executor Integration ──────────┐
├── 1B Conductor Watchers ────────────┤
├── 1C MCP Tool Registry ────────────┤── all independent, do in order
├── 1D Observability ─────────────────┤
├── 1E Re-Planning + Regeneration ────┤ (depends on 1A)
├── 1F Auto Plan Generation ──────────┤ (depends on 1A)
├── 1G Remaining Gate/Learn/API ──────┤
├── 1H TUI Dashboard ────────────────┤ (independent, can run anytime)
├── 1I Skill Library + Playbook ──────┤ (independent, but benefits from 1A)
└── 1J LinUCB Bandit + Attribution ───┘ (independent, but benefits from 1A)

Tier 2 (Agent Platform) ← depends on Tier 1 complete
├── 2A Extract roko-serve ────────────┐
├── 2B Create roko-plugin SDK ────────┤ (depends on 2A)
├── 2C Webhook Endpoints + Dispatch ──┤ (depends on 2A, 2B)
└── 2D Integration MCP Servers ───────┘ (depends on 1C)

Tier 3 (Templates & Events) ← depends on 2A, 2B, 2C
├── 3A Agent Template Schema ─────────┐
├── 3B Subscription System ───────────┤ (depends on 3A)
└── 3C Cron + File Watcher ───────────┘ (depends on 2C)

Tier 4 (Daemon & Ops) ← depends on 2C, 3B
├── 4A Daemon Mode ───────────────────┐
├── 4B Multi-Repo Config ─────────────┤ (depends on 4A)
├── 4C Cloud Deployment ──────────────┤ (depends on 4A)
└── 4D Secret Management ────────────┘ (independent)

Tier 5 (Cognitive) ← depends on Tier 1, can parallel with 2-4
├── 5A roko-neuro ────────────────────┐
├── 5B Context Assembly ──────────────┤ (depends on 5A)
├── 5C Daimon ────────────────────────┤ (depends on 5A)
├── 5D Dreams ────────────────────────┤ (depends on 5A, 5C)
├── 5E Operating Frequencies ─────────┤ (depends on 5B)
└── 5F C-Factor ──────────────────────┘ (depends on 5A)

Tier 6 (Chain) ← depends on Tier 5, deferred
├── 6A Mirage Infrastructure ─────────┐
├── 6B Agent Identity ────────────────┤ (depends on 6A)
├── 6C Gossip Mesh ───────────────────┤ (depends on 6B)
├── 6D Job Market ────────────────────┤ (depends on 6B, 6C)
├── 6E Reputation + Economics ────────┤ (depends on 6D)
├── 6F ChainWitness ──────────────────┤ (depends on 6A)
└── 6G Advanced (ISFR/Clearing/TEE) ──┘ (depends on all above)
```

---

## Crate Creation Roadmap

| Tier | New Crate | Purpose | Dependencies |
|------|-----------|---------|--------------|
| 2 | `roko-serve` | HTTP server library (extract from roko-cli) | roko-core, roko-agent, roko-learn, roko-gate, roko-fs |
| 2 | `roko-plugin` | Plugin SDK (EventSource + FeedbackCollector traits) | roko-core |
| 2 | `roko-mcp-github` | GitHub API as MCP server | roko-core (optional) |
| 2 | `roko-mcp-slack` | Slack API as MCP server | roko-core (optional) |
| 2 | `roko-mcp-scripts` | Script runner as MCP server | roko-core (optional) |
| 5 | `roko-neuro` | Knowledge store, memory, HDC fingerprints | roko-core, roko-fs |
| 5 | `roko-daimon` | Affect engine, motivation, emotion model | roko-core, roko-neuro |
| 5 | `roko-dreams` | Offline learning, consolidation, simulation | roko-core, roko-neuro, roko-learn |
| 6 | *(roko-golem already exists)* | Chain-specific behaviors | roko-core, roko-neuro |

---

# Tier 1: Mori Parity

> **Priority**: 🔴 P0 — Complete these before starting any new platform work.
> **Goal**: Everything that's BUILT gets WIRED. The existing plan-execute-gate-persist
> loop works end-to-end with all subsystems actually connected.

## 1A: Executor Phase Integration — Complete Action Handlers

> **Absorbs**: MASTER-REMAINING §A (Executor Integration), §B (Phase Handler Stubs),
> §D (Parallel Limits), PROMPT-EXECUTOR-PARITY §1-§3, §10
>
> **What exists** (more is wired than previously thought):
> - `crates/roko-orchestrator/src/executor.rs` — `ParallelExecutor` with `tick()` → `Vec<ExecutorAction>` (🔌 WIRED)
> - `crates/roko-orchestrator/src/executor/action.rs` — `ExecutorAction` enum: `DispatchAgent`, `RunGates`, `MergeBranch`, `PausePlan`, `ResumePlan`, etc.
> - `crates/roko-cli/src/orchestrate.rs:1208` — `run_task_plans()` calls `executor.tick()` in a loop, dispatches actions (🔌 WIRED)
> - `crates/roko-orchestrator/src/merge_queue.rs` — `MergeQueue` with file lock tracking (🔌 WIRED at orchestrate.rs:1453)
> - `crates/roko-core/src/phase.rs:157` — `PlanPhase` enum: Queued, Enriching, Implementing, Gating, Verifying, Reviewing, DocRevision, AutoFixing, RegeneratingVerify, Merging, Done, Complete, Failed, Skipped
> - `ExecutorConfig` struct (line 44): `max_concurrent_plans` (4), `max_auto_fix_iterations` (5), `max_merge_attempts` (3)
>
> **What's partially wired**:
> - `dispatch_action()` handles `DispatchAgent`, `RunGates`, `MergeBranch`, `PausePlan`, `ResumePlan` (🔌)
> - Other `ExecutorAction` variants fall through with "unhandled action" log (🏗️ STUBS)
> - `attempt_replan()` exists at line 2099 — fires after 3 consecutive gate failures (🔌 PARTIAL)
>
> **Reference**:
> - Mori executor: `/Users/will/dev/uniswap/bardo/crates/bardo-orchestrator/src/executor.rs`
> - Component spec: `/Users/will/dev/nunchi/roko/bardo-backup/tmp/roko-progress/COMPONENTS/executor.md`

### Items

- [ ] Complete ALL `dispatch_action()` handlers — currently some `ExecutorAction` variants are unhandled stubs. In `orchestrate.rs`, find every match arm that logs "unhandled" and implement the real handler.
- [ ] Expand `ExecutorConfig` — add fields: `max_concurrent_tasks: usize` (parallel tasks within a plan, distinct from `max_concurrent_plans`), `task_timeout_secs: u64` (default 600), `budget_usd: Option<f64>` (cost cap), `auto_replan: bool` (default true)
- [ ] Wire `ExecutorConfig` from `roko.toml` `[executor]` section — currently hardcoded defaults
- [ ] Implement `Enriching` phase handler: before agent dispatch, run SystemPromptBuilder to assemble the 6-layer prompt with task context, read_files, role constraints. Currently the enrichment step is implicit.
- [ ] Implement `Verifying` phase handler: after gates pass, run task's `verify` command (from tasks.toml `verify` field). Currently gate pass = done, but tasks may have additional verification commands.
- [ ] Implement `Reviewing` phase handler: compare agent output against task spec, check for drift. Currently skipped.
- [ ] Implement `DocRevision` phase handler: if task touches public API, auto-generate doc update prompt. Currently skipped.
- [ ] Implement `AutoFixing` phase handler: on gate failure, extract error, build fix prompt, re-dispatch to same agent with error context. Currently `attempt_replan()` does a coarser version — wire the fine-grained per-phase autofix.
- [ ] Implement `RegeneratingVerify` phase handler: after autofix, re-run verification to confirm fix worked. Currently skipped.
- [ ] Wire `--resume` — load `ExecutorState` from `.roko/state/executor.json`, skip plans/tasks already in `Done`/`Complete` state. The state file exists but check that deserialization handles schema changes gracefully.
- [ ] Wire cross-plan dependency tracking: parse `depends_on_plan` field in tasks.toml, block task dispatch until referenced plan completes
- [ ] Ensure task TOML fields are parsed and used: `tier` (→ model selection), `model_hint` (→ override cascade router), `read_files` (→ inject into agent context via `--read`), `write_files` (→ verify after completion), `verify` (→ post-gate verification command), `depends_on` (→ intra-plan ordering), `timeout_secs` (→ per-task timeout)
- [ ] Add `[executor]` section to default `roko.toml` template generated by `roko init`

**Worktree Per Task + Parallel Execution** (from PROMPT-EXECUTOR-PARITY §6):
- [ ] Wire `WorktreeManager` (exists at `crates/roko-orchestrator/src/worktree.rs`): before dispatching a task, acquire a worktree. Agent runs in the worktree directory (not repo root). After task completes, release the worktree. Worktrees are ephemeral: created per task, deleted after use.
- [ ] Wire parallel task dispatch: tasks within a dependency level (from `parallel_groups()` in `task_parser.rs`) dispatch concurrently using `tokio::spawn` + `JoinSet`. Concurrency capped at `max_concurrent_tasks` from ExecutorConfig.
- [ ] Clean stale git locks: `.git/index.lock` files older than 60s cleaned before worktree operations.
- [ ] Set `CARGO_TARGET_DIR` to shared target dir (not per-worktree) to avoid duplicate compilation.

**Cost Budget Enforcement** (from PROMPT-EXECUTOR-PARITY §8):
- [ ] After each agent dispatch, record cost (`input_tokens`, `output_tokens`, `cost_usd`) to costs DB. `BudgetConfig` exists at `crates/roko-cli/src/config.rs` and is parsed.
- [ ] Before dispatching an agent, check `config.budget.max_task_usd` — abort task if exceeded.
- [ ] Track cumulative plan cost — abort plan execution if `config.budget.max_plan_usd` exceeded.
- [ ] Warn at `config.budget.warn_at_percent` threshold (default: 80%).
- [ ] Cost data recorded in episode log alongside `wall_ms`.

### Verification

```bash
# All ExecutorAction variants handled (no "unhandled" log entries)
cargo run -p roko-cli -- plan run plans/ 2>&1 | grep -c 'unhandled'  # = 0

# Phase transitions logged
cargo run -p roko-cli -- plan run plans/ 2>&1 | grep 'phase.*→'  # shows transitions

# Resume works — run, ctrl-c, resume
cargo run -p roko-cli -- plan run plans/ --resume .roko/state/executor.json

# ExecutorConfig loads from roko.toml
grep -A5 '\[executor\]' roko.toml  # section exists with max_concurrent_tasks, etc.

# Task TOML fields used
grep -c 'tier\|model_hint\|read_files\|verify' plans/*/tasks.toml  # > 0
# Run a task with read_files set, verify files appear in agent's --read args
```

---

## 1B: Conductor Watcher Wiring

> **Absorbs**: MASTER-REMAINING §C (Conductor Watchers)
>
> **What exists**:
> - `crates/roko-conductor/src/watchers/` — 10 watchers, all with `check()` method (🏗️ BUILT)
>   - `budget.rs`, `dependency.rs`, `drift.rs`, `memory.rs`, `quality.rs`,
>     `rate.rs`, `regression.rs`, `stall.rs`, `timeout.rs`, `mod.rs`
> - `crates/roko-conductor/src/circuit_breaker.rs` — circuit breaker pattern (🏗️ BUILT)
> - `crates/roko-conductor/src/diagnosis.rs` — error pattern matching (🏗️ BUILT)
>
> **Reference**:
> - Mori conductor: `/Users/will/dev/uniswap/bardo/crates/bardo-conductor/`
> - Component spec: `/Users/will/dev/nunchi/roko/bardo-backup/tmp/roko-progress/COMPONENTS/conductor.md`

### Items

**Note**: The actual watcher module names differ from earlier docs. The real files are:
`compile_fail_repeat`, `context_window_pressure`, `cost_overrun`, `ghost_turn`,
`iteration_loop`, `review_loop`, `spec_drift`, `stuck_pattern`, `test_failure_budget`, `time_overrun`.
Each implements the `Policy` trait from roko-core: `fn check(&self, signals: &[Signal]) -> Vec<Signal>`.

A `Conductor` struct already exists at `crates/roko-conductor/src/conductor.rs` that holds
`Vec<Box<dyn Policy>>`, a `CircuitBreaker`, and a `DiagnosisEngine` — but it's never instantiated
from orchestrate.rs.

- [ ] Instantiate `Conductor` in `PlanRunner::new()` — create with all 10 watchers registered. The `Conductor` struct exists but is never constructed in the runtime. Add `Conductor::new()` call in orchestrate.rs where `PlanRunner` is set up.
- [ ] Create `WatcherRunner` (or use `Conductor` directly): spawn a tokio task that calls `conductor.check_all(recent_signals)` every 30s during plan execution. Feed it the last N signals from `.roko/signals.jsonl`.
- [ ] Wire `WatcherRunner` lifecycle: start when `run_task_plans()` begins, cancel when it returns. Use a `CancellationToken` (from `tokio-util`) shared with the executor loop.
- [ ] Wire circuit breaker response: when `CircuitBreaker::is_broken(plan_id)` returns true, emit `ExecutorEvent::PausePlan` so the executor halts that plan. Currently `record_failure` and `is_broken` exist but nothing checks `is_broken()` during the dispatch loop.
- [ ] Wire diagnosis integration: when circuit breaker trips, call `DiagnosisEngine::diagnose(error_output)` to classify the failure (the engine has ~20 `ErrorCategory` variants like `TypeMismatch`, `BorrowChecker`, etc.). Include the `DiagnosisResult` in the emitted signal.
- [ ] Wire `cost_overrun` watcher: feed it actual cost data from efficiency events (`.roko/learn/efficiency.jsonl`). The watcher exists but needs a cost source — read the latest efficiency entries and compare against `ExecutorConfig::budget_usd`.
- [ ] Wire `stuck_pattern` watcher: feed it agent turn data. Detect when an agent produces the same output repeatedly (>3 identical turns). The watcher has the pattern matching logic but needs a data feed from the agent turn log.
- [ ] Wire `context_window_pressure` watcher: read agent token usage from efficiency events. Alert when usage exceeds 80% of model's context window (model context sizes: haiku 200K, sonnet 200K, opus 1M).
- [ ] Wire `time_overrun` watcher: compare elapsed time per task against `timeout_secs` from tasks.toml. Emit alert signal when task exceeds 80% of timeout.
- [ ] Wire `compile_fail_repeat` watcher: detect when same compilation error appears >2 times across consecutive agent turns. Signals the agent is stuck in a loop.
- [ ] Wire `ghost_turn` watcher: detect agent turns that produce no file changes and no meaningful output. Flag as wasted cost.
- [ ] Wire `spec_drift` watcher: compare agent's actual file changes against task's `write_files` list. Alert if agent modifies files not in scope.
- [ ] Wire `test_failure_budget` watcher: track test failure count. If failures increase after agent's changes (vs baseline), alert.
- [ ] Wire `iteration_loop` and `review_loop` watchers: detect repeated gate fail → retry cycles without progress.
- [ ] Emit all watcher alerts as Signals to `.roko/signals.jsonl` with kind `conductor:alert:{watcher_name}`

### Verification

```bash
# Conductor is instantiated and runs during plan execution
cargo run -p roko-cli -- plan run plans/ 2>&1 | grep -c 'conductor'  # > 0

# Cost overrun detection works
# Set budget_usd = 0.01 in roko.toml, run plan, verify alert signal
grep 'conductor:alert:cost_overrun' .roko/signals.jsonl

# Stuck pattern detection works
# (run task with agent that loops, verify stuck_pattern alert appears)
grep 'conductor:alert:stuck_pattern' .roko/signals.jsonl

# Circuit breaker trips after sustained failures
# Set MAX_PLAN_FAILURES = 2, run plan with always-failing tasks
grep 'circuit_breaker:tripped' .roko/signals.jsonl

# Diagnosis output included in signals
grep 'DiagnosisResult' .roko/signals.jsonl
```

---

## 1C: MCP Tool Registry + Server Lifecycle

> **Absorbs**: MASTER-REMAINING §E (MCP & Tool Registry), plan 07 (MCP & Tool Registry Wiring),
> PROMPT-EXECUTOR-PARITY §8
>
> **What exists**:
> - `crates/roko-agent/src/mcp/` — MCP client, JSON-RPC, tool converter, dedup, config walk-up (🔌 WIRED for passthrough)
> - `crates/roko-std/src/tool_definitions.rs` — 19 builtin tool definitions (🔌 WIRED)
> - MCP config passthrough via `--mcp-config` flag (🔌 WIRED)
>
> **Not yet wired**:
> - Tool restriction per role (e.g., researcher can't use `write_file`)
> - MCP server spawn/lifecycle management (start servers before agent, stop after)
> - Tool discovery from MCP servers (query available tools, merge into registry)
>
> **Reference**:
> - Mori tool system: `/Users/will/dev/nunchi/roko/bardo-backup/tmp/mori-agents/tool-system.md`
> - MCP spec: `crates/roko-agent/src/mcp/protocol.rs`
> - Plan 07 (superseded): `implementation-plans/07-mcp-tool-wiring.md`

### Items

**Note**: `DynamicToolRegistry` already exists at `crates/roko-agent/src/mcp/dynamic_registry.rs`.
It merges builtin tools (from `roko-std`) with MCP-discovered tools. What's MISSING is role-based
filtering — it returns all tools regardless of the agent's role.

The existing `ToolDispatcher` at `crates/roko-agent/src/dispatcher/mod.rs:80` has:
- `registry: Arc<dyn ToolRegistry>` — provides tool definitions
- `safety: Option<SafetyLayer>` — pre/post checks per call
- Permission check: `def.permission.satisfied_by(&role_perms)` (line 182) — checks role permissions
But NO `allowed_tools`/`denied_tools` filter per task.

- [ ] Add `allowed_tools: Option<Vec<String>>` and `denied_tools: Option<Vec<String>>` fields to the task struct in `crates/roko-orchestrator/src/task.rs`. Parse from tasks.toml.
- [ ] Wire tool filtering in `ToolDispatcher::dispatch()`: before executing, check if tool name is in `allowed_tools` (if set) and NOT in `denied_tools`. Reject with a clear error message explaining which tool was blocked and why.
- [ ] Wire `--allowedTools` CLI flag passthrough: when dispatching a ClaudeCliAgent, convert `allowed_tools` from task config into the `--allowedTools` flag format. This restricts what Claude can call.
- [ ] Define role-based tool profiles in `crates/roko-std/src/roles.rs` (or similar):
  - `Implementer`: all tools allowed
  - `Researcher`: read-only tools only (`Read`, `Grep`, `Glob`, `WebSearch`, `WebFetch`), deny `Write`, `Edit`, `Bash`
  - `Reviewer`: read + comment tools, deny `Write`, `Edit`
  - `Strategist`: read + plan tools, deny destructive ops
- [ ] Wire role profiles: when task specifies `role = "researcher"`, auto-populate `denied_tools` from the role profile. Task-level overrides take precedence.
- [ ] Wire MCP server lifecycle in `orchestrate.rs` `dispatch_action(DispatchAgent{..})`:
  1. Before agent spawn: start required MCP servers (from task's `mcp_servers` or global config)
  2. Health check: verify each server responds to `initialize` JSON-RPC method
  3. After agent completes: stop MCP server processes (or keep alive if shared across tasks)
  4. Timeout: if MCP server doesn't respond within 10s, skip it and log warning
- [ ] Wire tool discovery: after MCP server starts, call `tools/list` JSON-RPC method. Merge discovered tools into `DynamicToolRegistry`. The `mcp_to_tool_def()` converter already exists.
- [ ] Wire tool dedup in `DynamicToolRegistry`: if MCP tool has same name as builtin, prefer builtin (log a warning). Add config option `[tools] prefer_mcp = false` to flip priority.
- [ ] Add `[tools]` section to `roko.toml`: `prefer_mcp` (bool), `global_denied` (list of tool names blocked everywhere), `mcp_timeout_secs` (MCP server startup timeout)
- [ ] Add per-task `mcp_servers` field in tasks.toml — list of MCP server names this task needs

### Verification

```bash
# Role-based restrictions work
# Create tasks.toml with: role = "researcher", then run the plan
# Check agent logs — Write/Edit tool calls should be rejected
cargo run -p roko-cli -- plan run plans/test-role/ 2>&1 | grep 'tool.*denied'

# Allowed/denied tools work
# Create tasks.toml with: denied_tools = ["Bash"]
# Verify Bash calls are blocked
cargo run -p roko-cli -- plan run plans/test-tools/ 2>&1 | grep 'blocked.*Bash'

# MCP server lifecycle works
# Set mcp_config in roko.toml, run plan, check server starts
cargo run -p roko-cli -- plan run plans/ 2>&1 | grep -E 'mcp.*(start|stop|health)'

# Tool discovery works
# Start MCP server manually, verify tools/list response parsed
echo '{"jsonrpc":"2.0","method":"tools/list","id":1}' | cargo run -p roko-mcp-github 2>/dev/null | python3 -m json.tool
```

---

## 1D: Observability Infrastructure

> **Absorbs**: MASTER-REMAINING §F (Observability), plan 08 (Observability Wiring)
>
> **What exists**:
> - `tracing` crate in Cargo.toml (dependency exists in roko-core, roko-cli, roko-agent)
> - Basic `tracing::info!` / `tracing::error!` calls in orchestrate.rs, dispatcher, gates
> - `tracing-subscriber` is NOT in Cargo.toml yet — only `tracing` (the facade)
> - Efficiency events already track per-turn cost/tokens in `.roko/learn/efficiency.jsonl` (🔌 WIRED)
>
> **Not yet wired**:
> - Structured tracing subscriber (no JSON output, no span hierarchy)
> - Cost aggregation at plan/session level
> - Log level control
>
> **Reference**:
> - Plan 08 (superseded): `implementation-plans/08-observability-wiring.md`

### Items

- [ ] Add `tracing-subscriber` dependency to `roko-cli/Cargo.toml` with features `["json", "env-filter"]`
- [ ] Wire subscriber initialization in `main.rs` before any async runtime starts:
  ```
  // text mode (default)
  tracing_subscriber::fmt().with_env_filter("roko=info").init()
  // json mode (when --log-format json)
  tracing_subscriber::fmt().json().with_env_filter("roko=info").init()
  ```
- [ ] Add span hierarchy in `orchestrate.rs`:
  - `run_task_plans()`: `#[instrument(skip_all, fields(plan_dir = %path))]`
  - Per-task dispatch: `let _span = info_span!("task", task_id = %task.id, phase = %task.phase).entered();`
  - Agent dispatch: `let _span = info_span!("agent", model = %agent.model, role = %task.role).entered();`
  - Gate run: `let _span = info_span!("gate", gate = %gate.name, rung = %rung).entered();`
- [ ] Add span fields that propagate: `plan_id`, `task_id`, `agent_model`, `task_role`
- [ ] Wire `--log-format` CLI flag to `roko` command (add to `Cli` struct in main.rs): values `text` (default) or `json`
- [ ] Wire `ROKO_LOG` env var for log level (maps to `tracing-subscriber` `EnvFilter`)
- [ ] Add cost summary at end of `run_task_plans()`: read efficiency events, aggregate total tokens (input+output), total cost (USD), total duration. Print as structured log entry:
  ```
  info!(total_cost_usd = cost, total_input_tokens = in_tok, total_output_tokens = out_tok,
        duration_secs = dur, tasks_completed = done, tasks_failed = failed, "plan run complete");
  ```
- [ ] Wire timing: record `Instant::now()` at task start, log elapsed on task complete. Add `duration_ms` field to the efficiency event struct.
- [ ] Wire secret redaction: in agent dispatch logging, if log line contains patterns matching API keys (sk-*, xoxb-*, ghp_*), replace with `[REDACTED]`. Add a `tracing-subscriber` layer that filters these patterns.
- [ ] Wire structured error context: when a task fails, log the full error chain using `anyhow`'s `.context()` chain. Include task_id, phase, gate that failed, and last N lines of agent output.
- [ ] Ensure all `eprintln!` and `println!` in orchestrate.rs are replaced with appropriate `tracing::info!` / `tracing::error!` macros (currently mixed)

### Verification

```bash
# JSON logging works
ROKO_LOG=debug cargo run -p roko-cli -- --log-format json status 2>&1 | head -5 | python3 -c "import json,sys; [json.loads(l) for l in sys.stdin]"

# Span hierarchy visible in JSON output
ROKO_LOG=debug cargo run -p roko-cli -- --log-format json plan run plans/ 2>&1 | grep '"task_id"' | head -3

# Cost summary appears at end of plan run
cargo run -p roko-cli -- plan run plans/ 2>&1 | grep 'plan run complete.*total_cost'

# Env filter works
ROKO_LOG=warn cargo run -p roko-cli -- plan run plans/ 2>&1 | wc -l  # much fewer lines than default
```

---

## 1E: Re-Planning + Plan Regeneration

> **Absorbs**: MASTER-REMAINING §G (Re-planning), §K (Plan Regeneration)
> **Depends on**: 1A (executor must be wired first)
>
> **What exists**:
> - `roko prd plan <slug>` — generates plans from PRDs (🔌 WIRED)
> - `attempt_replan()` at `orchestrate.rs:2099` — fires after 3 consecutive gate failures on same task (🔌 PARTIAL)
>   - Current logic: if `gate_fail_count >= 3`, dispatches a new agent with the gate error output as context, asking it to produce a fix. The "replan" is really "retry with more context", not a structural re-decomposition.
> - Gate results persisted per task (🔌 WIRED)
> - Episode logger tracks success/failure (🔌 WIRED)
>
> **Not yet wired**:
> - Strategy selection (currently only retry-with-context)
> - Structural decomposition (splitting a failed task into subtasks)
> - Plan-level regeneration (full plan rebuild)
> - Feeding failure patterns to learning

### Items

- [ ] Extract `ReplanStrategy` enum: `RetrySame` (current behavior), `RetryWithEscalation` (upgrade model), `Decompose` (split task), `Skip` (mark task skipped, continue), `RegeneratePlan` (full re-plan). Add to `crates/roko-orchestrator/src/replan.rs` (new file).
- [ ] Add `replan_strategy` selection logic in `attempt_replan()`:
  - 1st failure: `RetrySame` with error context (current behavior)
  - 2nd failure: `RetryWithEscalation` — use next-tier model (haiku→sonnet→opus)
  - 3rd failure: `Decompose` — dispatch an agent to split the task into 2-3 subtasks
  - 4th+ failure: `Skip` — mark task skipped, log to episodes
  - If >50% of plan tasks are skipped/failed: `RegeneratePlan`
- [ ] Implement `Decompose` strategy:
  1. Build a prompt with: original task spec, all gate error outputs, file contents from `read_files`
  2. Dispatch an opus-tier agent asking it to produce 2-3 subtasks in tasks.toml format
  3. Parse the agent's output into `Task` structs
  4. Insert subtasks into the executor's plan DAG, replacing the failed task
  5. Mark original task as `Skipped` with `split_into: [subtask_ids]`
- [ ] Implement `RetryWithEscalation`: change task's `model_hint` to next tier before re-dispatch. Tier order from `CascadeRouter`: haiku → sonnet → opus. If already opus, fall through to `Decompose`.
- [ ] Implement `RegeneratePlan`: call `prd plan <slug>` to regenerate the entire plan from the PRD, preserving completed tasks. This requires:
  1. Collect completed task IDs from executor state
  2. Re-run plan generation
  3. Diff new plan vs old plan — only add genuinely new tasks
  4. Skip any task whose description matches a completed task (fuzzy match)
- [ ] Add `max_retries` field to task TOML schema (default: 3). Parse in task loader.
- [ ] Add `replan_strategy` field to task TOML schema (optional, overrides automatic selection)
- [ ] Feed failure patterns to learning: after each failed task, emit an `EfficiencyEvent` with `outcome: "failure"`, `gate_errors: [list]`, `model_used`, `strategy_attempted`. The cascade router can use this to learn which models fail on which task types.
- [ ] Wire `--no-replan` CLI flag: skip all re-planning, tasks that fail gate → immediately `Failed` state
- [ ] Log all re-plan events to `.roko/episodes.jsonl` with `kind: "replan"` and metadata about strategy chosen, attempt number, original task, resulting subtasks

### Verification

```bash
# Task failure triggers re-plan (retry with context)
# Create a tasks.toml with a task that has an intentional error. Run it.
cargo run -p roko-cli -- plan run plans/test-replan/ 2>&1 | grep 'replan.*attempt'

# Escalation works — verify model changes on retry
cargo run -p roko-cli -- plan run plans/test-replan/ 2>&1 | grep 'escalating.*model'

# Decompose works — verify subtask creation
cargo run -p roko-cli -- plan run plans/test-replan/ 2>&1 | grep 'decomposing.*subtask'

# Plan regeneration fires when >50% fail
cargo run -p roko-cli -- plan run plans/test-mass-fail/ 2>&1 | grep 'regenerating.*plan'

# --no-replan disables everything
cargo run -p roko-cli -- plan run plans/test-replan/ --no-replan 2>&1 | grep -c 'replan'  # = 0

# Learning receives failure events
grep '"outcome":"failure"' .roko/learn/efficiency.jsonl | head -3
```

---

## 1F: Automatic Plan Generation

> **Absorbs**: MASTER-REMAINING §I (Auto Plan)
> **Depends on**: 1A (executor)
>
> **What exists**:
> - `roko prd plan <slug>` — dispatches a Claude agent with the PRD content + plan generation prompt. Agent produces tasks.toml. (🔌 WIRED)
> - `roko prd draft promote` — moves PRD from draft/ to published/ directory (🔌 WIRED)
> - `roko prd list` / `roko prd status` — show PRD inventory (🔌 WIRED)
>
> **Not yet wired**:
> - Automatic trigger on promote (currently requires separate manual `prd plan` command)
> - Plan structural validation before execution
> - Plan quality check (are tasks small enough? do they have verify commands?)

### Items

- [ ] Wire `on_prd_promote` hook in `crates/roko-cli/src/prd.rs`: after the promote file move succeeds, check `auto_plan` config flag. If true, call the same logic as `prd plan <slug>` inline. Implementation: extract the plan generation logic from the `prd plan` subcommand handler into a shared function `generate_plan_from_prd(slug: &str, prd_path: &Path) -> Result<PathBuf>`.
- [ ] Add `auto_plan` field to the `[prd]` config section in `roko.toml` (default: false). Parse via `RokoConfig`.
- [ ] Wire plan structural validation — after plan generation, before execution, check:
  1. tasks.toml parses without errors (TOML syntax valid)
  2. All tasks have required fields: `id`, `title`, `description`
  3. No circular dependencies in `depends_on` references
  4. All `depends_on` references point to tasks that exist in the same plan
  5. At least one task has no dependencies (i.e., there's a start node)
  Emit validation errors as structured log entries. If validation fails, don't execute — log error and exit.
- [ ] Wire plan quality heuristics (warnings, not blockers):
  - Warn if any task description is >500 words (likely too coarse — DESIGN-TASK-GENERATION.md says ≤50 LOC changes)
  - Warn if task lacks `read_files` (agent won't have file context)
  - Warn if task lacks `verify` (no way to check completion)
  - Warn if >20 tasks in one plan (might need splitting)
- [ ] Add `--auto-execute` flag to `prd draft promote`: after plan generation, immediately call `run_task_plans()` on the generated plan directory
- [ ] Add `--dry-run` flag to `prd plan`: generate the plan, validate it, print summary, but don't write tasks.toml. Useful for previewing what would be generated.
- [ ] Wire signal emission: when auto-plan completes, emit a `prd:plan:generated` signal with plan path, task count, estimated complexity. When it fails, emit `prd:plan:failed` with error details.
- [ ] Wire plan template selection: add optional `plan_template` field to PRD TOML front matter. Templates could specify: default model tier, gate strictness level, max task count. If not set, use defaults.

**Old-Format Plan Regeneration** (absorbs MASTER-REMAINING §K):
- [ ] Wire `roko plan validate <dir>` command: check if tasks.toml has modern fields (`tier`, `model_hint`, `read_files`, `verify`, `depends_on`). Report which fields are missing.
- [ ] Wire `roko plan regenerate <dir>` command: re-run plan generation from the plan's source PRD, preserving completed task status. Output new tasks.toml with all modern fields populated.
- [ ] Wire `roko plan list` to flag old-format plans: show warning icon next to plans whose tasks.toml lacks modern fields.
- [ ] Regenerate existing old-format plans (P06-process-management, W01-wire-system-prompts, etc.)

### Verification

```bash
# Manual plan generation works (baseline)
cargo run -p roko-cli -- prd plan test-prd
test -f plans/test-prd/tasks.toml && echo "plan generated"

# Plan validation catches errors
echo 'invalid toml content {{{' > plans/bad/tasks.toml
cargo run -p roko-cli -- plan run plans/bad/ 2>&1 | grep 'validation.*failed'

# Auto-plan on promote
echo 'auto_plan = true' >> roko.toml  # or set in [prd] section
cargo run -p roko-cli -- prd draft promote --slug test-prd 2>&1 | grep 'auto.*generating'
test -f plans/test-prd/tasks.toml

# Quality warnings appear
cargo run -p roko-cli -- prd plan test-prd 2>&1 | grep 'warn.*read_files\|warn.*verify'

# --auto-execute triggers run
cargo run -p roko-cli -- prd draft promote --slug test-prd --auto-execute 2>&1 | grep 'executing.*plan'

# Old-format plan detection
cargo run -p roko-cli -- plan list  # flags old-format plans
cargo run -p roko-cli -- plan validate plans/P06-process-management/  # shows missing fields
```

---

## 1G: Remaining Gate, Learn, and API Items

> **Absorbs**: MASTER-REMAINING §J (API Execution), remaining gate/learn items
>
> **What exists**:
> - Gate pipeline: 11 gates in `crates/roko-gate/src/gates/`, 6-rung pipeline in `pipeline.rs` (🔌 WIRED)
>   - Gates: `compile`, `test`, `clippy`, `diff_review`, `format`, `coverage`, `doc_test`, `miri`, `semver`, `benchmark`, `custom`
> - Learning: efficiency events, cascade router, experiments, adaptive thresholds (🔌 WIRED)
> - HTTP API routes (🔌 WIRED):
>   - `POST /api/plans/{id}/execute` — triggers plan execution (ALREADY EXISTS)
>   - `GET /api/plans` — list plans
>   - `GET /api/agents` — list agents
>   - `GET /api/status` — system status
>   - `/ws` WebSocket endpoint with `EventBroadcaster` (ALREADY EXISTS)
> - Auth middleware: `X-Api-Key` header check exists in `serve/middleware.rs` but is optional (🏗️ BUILT)
>
> **What's actually missing** (after checking real code):
> - WebSocket doesn't broadcast execution progress events (only basic ping/status)
> - Gate result aggregation API doesn't exist
> - Learning state API endpoints don't exist
> - Auth is off by default and config to enable it isn't documented

### Items

- [ ] Wire execution progress → WebSocket: in `orchestrate.rs`, after each phase transition / task completion / gate result, call `EventBroadcaster::send(ExecutionEvent { plan_id, task_id, phase, status, timestamp })`. The broadcaster and WS endpoint exist — need to define `ExecutionEvent` struct and send it at the right points.
- [ ] Define `ExecutionEvent` enum variants: `PlanStarted`, `TaskStarted { task_id, phase }`, `TaskPhaseChanged { task_id, old_phase, new_phase }`, `GateResult { task_id, gate, passed, message }`, `TaskCompleted { task_id, outcome }`, `PlanCompleted { outcome, stats }`, `ReplanTriggered { task_id, strategy }`, `WatcherAlert { watcher, message }`. Serialize as JSON for WS clients.
- [ ] Wire `GET /api/gates/summary` endpoint: read all gate results from signals.jsonl (kind `gate:*`), aggregate by gate name → { total_runs, pass_rate, avg_duration_ms, last_run }. Return JSON.
- [ ] Wire `GET /api/gates/{gate_name}/history` endpoint: time series of pass/fail for a specific gate across all runs.
- [ ] Wire `GET /api/learn/cascade` endpoint: read `CascadeRouter` state from `.roko/learn/cascade-router.json`, return current model weights, routing stats, recommended model per task type.
- [ ] Wire `GET /api/learn/experiments` endpoint: read `ExperimentStore` from `.roko/learn/experiments.json`, return active experiments, variant performance, statistical significance.
- [ ] Wire `GET /api/learn/efficiency` endpoint: aggregate efficiency events from `.roko/learn/efficiency.jsonl` → { total_cost, cost_per_task, tokens_per_task, avg_task_duration, cost_trend }.
- [ ] Wire `GET /api/learn/adaptive-thresholds` endpoint: read adaptive gate thresholds from `.roko/learn/gate-thresholds.json`, return current thresholds per rung with EMA values.
- [ ] Ensure all API endpoints return proper HTTP error codes:
  - 404 for missing plans/gates/experiments (not panic)
  - 400 for invalid request parameters
  - 500 for internal errors (with error message, not stack trace)
  - Test each endpoint with missing/invalid inputs
- [ ] Wire API auth toggle: add `[serve.auth]` section to `roko.toml` with `enabled = false` (default), `api_key = ""`. When enabled, all `/api/*` routes require `X-Api-Key` header. Document in `roko init` template.
- [ ] Add `GET /api/health` endpoint: returns `{ status: "ok", version, uptime_secs, active_plans, active_agents }`. Useful for load balancer health checks and monitoring.

**Cybernetic Metrics Dashboard** (from phase-7-8.md §7.5):
- [ ] Add `GET /api/metrics/summary` endpoint — aggregate metrics over a time window (default: last 7 days):
  ```json
  {
    "period": "last_7_days",
    "agents_run": 142,
    "success_rate": 0.89,
    "feedback_engagement_rate": 0.73,
    "avg_cost_per_episode_cents": 12,
    "experiments_active": 4,
    "best_experiment_lift": { "name": "review-depth", "lift": 0.15, "winning": "concise" },
    "gate_pass_rate": 0.94,
    "self_improvement_velocity": 0.02,
    "top_templates": [
      { "name": "pr-review-agent", "runs": 38, "success_rate": 0.95 },
      { "name": "pm-board-agent", "runs": 42, "success_rate": 0.98 }
    ]
  }
  ```
- [ ] Add per-metric endpoints:
  - `GET /api/metrics/success_rate` — per template, per trigger kind
  - `GET /api/metrics/engagement` — `acknowledged_actions / total_actions` per template (from feedback collection)
  - `GET /api/metrics/model_efficiency` — cost per successful episode via CascadeRouter
  - `GET /api/metrics/gate_rate` — `passed_gates / total_gates` per gate type with trend
  - `GET /api/metrics/experiments` — metric difference between best and worst variant per experiment
  - `GET /api/metrics/feedback_latency` — median hours between action and first feedback signal
  - `GET /api/metrics/velocity` — rate of change of success rate over time (should be positive = self-improving)
  - `GET /api/metrics/coverage` — % of events with matching subscriptions vs unhandled

### Verification

```bash
# Start server
cargo run -p roko-cli -- serve &
sleep 2

# WebSocket receives execution events
(echo ""; sleep 5) | websocat ws://localhost:3000/ws &
cargo run -p roko-cli -- plan run plans/  # verify WS client receives events

# Gate summary endpoint works
curl -s http://localhost:3000/api/gates/summary | python3 -m json.tool

# Learning endpoints work
curl -s http://localhost:3000/api/learn/cascade | python3 -m json.tool
curl -s http://localhost:3000/api/learn/experiments | python3 -m json.tool
curl -s http://localhost:3000/api/learn/efficiency | python3 -m json.tool
curl -s http://localhost:3000/api/learn/adaptive-thresholds | python3 -m json.tool

# Auth works when enabled
curl -s http://localhost:3000/api/status -H "X-Api-Key: wrong" -w '%{http_code}'  # 401
curl -s http://localhost:3000/api/status -H "X-Api-Key: correct" -w '%{http_code}'  # 200

# Health endpoint
curl -s http://localhost:3000/api/health | python3 -m json.tool

# Error handling (404, not panic)
curl -s http://localhost:3000/api/plans/nonexistent/execute -w '%{http_code}'  # 404
```

---

## 1H: TUI Dashboard

> **Absorbs**: MASTER-REMAINING §H (TUI), plan 09 (TUI & Dashboard)
>
> **What exists**:
> - `crates/roko-cli/src/dashboard.rs` — text-mode rendering (🏗️ BUILT, no terminal UI)
> - Dashboard data collection from signals, episodes, gates (🔌 WIRED)
>
> **Not yet wired**:
> - ratatui terminal UI library
> - Interactive navigation between dashboard pages
> - Live-updating display during plan execution
>
> **Reference**:
> - Plan 09 (superseded): `implementation-plans/09-tui-dashboard.md`
> - Mori dashboard: `/Users/will/dev/uniswap/bardo/apps/mori/src/tui/`

### Items

- [ ] Add `ratatui = "0.29"` + `crossterm = "0.28"` dependencies to `roko-cli/Cargo.toml`
- [ ] Create `crates/roko-cli/src/tui/` module directory with:
  - `mod.rs` — `pub mod app, event, pages, widgets;`
  - `app.rs` — `App` struct and main render loop
  - `event.rs` — crossterm event polling (keyboard, resize, tick)
  - `pages/mod.rs` — page trait and registry
  - `widgets/mod.rs` — reusable widget components
- [ ] Define `App` struct:
  ```
  pub struct App {
      pub current_page: PageId,          // Overview, Execution, Agents, Gates, Learning, Signals
      pub data: DashboardData,           // shared data model, refreshed on tick
      pub running: bool,                 // false → exit event loop
      pub last_refresh: Instant,         // rate-limit data refresh
      pub scroll_offset: HashMap<PageId, u16>,  // per-page scroll state
  }
  ```
- [ ] Define `DashboardData` struct — shared data model loaded from `.roko/`:
  ```
  pub struct DashboardData {
      pub plans: Vec<PlanSummary>,           // from executor state
      pub active_tasks: Vec<TaskSummary>,    // currently executing
      pub agents: Vec<AgentSummary>,         // from ProcessSupervisor
      pub gate_results: Vec<GateResultSummary>,  // from signals.jsonl (kind gate:*)
      pub efficiency: EfficiencySummary,     // from .roko/learn/efficiency.jsonl
      pub cascade_router: CascadeRouterState,// from .roko/learn/cascade-router.json
      pub experiments: Vec<ExperimentSummary>,// from .roko/learn/experiments.json
      pub recent_signals: Vec<SignalSummary>,// last 100 from signals.jsonl
      pub conductor_alerts: Vec<AlertSummary>,// from signals.jsonl (kind conductor:alert:*)
      pub cfactor: Option<CFactor>,          // from .roko/learn/c-factor.jsonl (latest)
  }
  ```
- [ ] Implement event loop in `app.rs`:
  ```
  pub async fn run(terminal: &mut Terminal<CrosstermBackend<Stdout>>, app: &mut App) -> Result<()> {
      loop {
          terminal.draw(|f| render_page(f, app))?;
          if crossterm::event::poll(Duration::from_millis(250))? {
              match crossterm::event::read()? {
                  Event::Key(key) => handle_key(app, key),
                  Event::Resize(_, _) => {},  // ratatui handles this
                  _ => {}
              }
          }
          if app.last_refresh.elapsed() > Duration::from_secs(1) {
              app.data.refresh().await?;
              app.last_refresh = Instant::now();
          }
          if !app.running { break; }
      }
      Ok(())
  }
  ```
- [ ] Wire page 1 — **Overview**: 3-column layout using `ratatui::layout::Layout::horizontal()`:
  - Left column: plan list as `Table` widget — columns: plan name, status (colored), task progress bar (`Gauge`), elapsed time
  - Center column: health indicators as `Block` with `Sparkline` widgets — gate pass rate (7-day), cost trend, C-Factor score
  - Right column: conductor alerts as `List` widget — last 10 alerts with severity coloring (red=critical, yellow=warning)
  - Bottom bar: summary stats — "Plans: 3 active, 12 done | Tasks: 45/52 | Cost: $12.34 | C-Factor: 0.72 ↑"
- [ ] Wire page 2 — **Plan Execution**: focused view of currently executing plan:
  - Top: plan name + progress bar (`Gauge` widget, tasks_done/tasks_total)
  - Middle: task table (`Table`) — columns: task ID, title (truncated), phase (colored: Implementing=blue, Gating=yellow, Done=green, Failed=red), model, duration
  - Bottom: live agent output — last 20 lines from current agent's stderr, displayed in `Paragraph` widget with scroll. Source: tail `.roko/episodes.jsonl` for latest agent's turn output.
  - Right sidebar: current task detail — description, read_files, write_files, gate results
- [ ] Wire page 3 — **Agent Activity**:
  - Top: active agents table — columns: agent ID, model, task, role, turns, tokens used, cost, uptime
  - Middle: model distribution — horizontal `BarChart` showing haiku/sonnet/opus usage counts
  - Bottom: cost breakdown — `Table` with per-model cost (input tokens × rate + output tokens × rate), total session cost
  - Data source: `ProcessSupervisor::list()` for active agents, efficiency events for historical
- [ ] Wire page 4 — **Gate Results**:
  - Top: gate summary table — columns: gate name, total runs, pass rate (colored: >90%=green, 70-90%=yellow, <70%=red), avg duration, last run
  - Middle: adaptive thresholds — `Table` showing current threshold per rung with EMA value and trend arrow
  - Bottom: recent gate failures — `List` of last 10 failures with task ID, gate name, error excerpt (first line)
  - Data source: signals.jsonl (kind `gate:*`), `.roko/learn/gate-thresholds.json`
- [ ] Wire page 5 — **Learning**:
  - Top left: cascade router state — `Table` of models with weights, recommendation counts, UCB scores
  - Top right: active experiments — `Table` with experiment name, variants, sample sizes, current winner, statistical significance
  - Bottom: efficiency trends — `Sparkline` widgets for: cost per task (7-day), tokens per task, success rate, first-try rate
  - Data source: `.roko/learn/cascade-router.json`, `.roko/learn/experiments.json`, `.roko/learn/efficiency.jsonl`
- [ ] Wire page 6 — **Signals**:
  - Top: recent signals table — columns: timestamp (relative: "2m ago"), kind, plan/task ID (if present), payload preview (truncated to 60 chars)
  - Middle: signal kind distribution — `BarChart` showing counts per kind prefix (gate:*, conductor:*, prd:*, etc.)
  - Bottom: signal DAG explorer — for a selected signal, show its `parent_hash` chain as an indented tree. Navigate with arrow keys.
  - Data source: `.roko/signals.jsonl` (last 100)
- [ ] Wire keyboard navigation:
  - `1`-`6` or `Tab`/`Shift+Tab`: switch pages
  - `q` or `Esc`: quit
  - `↑`/`↓` or `j`/`k`: scroll within page
  - `Enter`: expand selected item (show full signal payload, full gate error, etc.)
  - `r`: force refresh data
  - `?`: show help overlay with all keybindings
- [ ] Wire live refresh: `DashboardData::refresh()` reads changed files only (check mtime before re-parsing). For signals.jsonl and episodes.jsonl, seek to last known offset and read new lines only (don't re-parse entire file each tick).
- [ ] Wire terminal setup/teardown: `crossterm::terminal::enable_raw_mode()` on entry, `disable_raw_mode()` + `crossterm::execute!(stdout(), LeaveAlternateScreen)` on exit (including panic handler via `std::panic::set_hook`).
- [ ] Wire `roko dashboard` to launch TUI (replace current text renderer). Keep existing `render_dashboard_text()` as fallback.
- [ ] Wire `roko dashboard --text` flag to use text-mode output (pipe-friendly, no terminal UI)
- [ ] Wire `roko dashboard --page <N>` flag to start on a specific page
- [ ] Wire color theming: use `ratatui::style::Color` constants. Define a `Theme` struct with configurable colors. Default: dark theme matching terminal. Respect `NO_COLOR` env var.

### Verification

```bash
# TUI launches and shows overview
cargo run -p roko-cli -- dashboard
# (verify: terminal clears, 6 pages accessible via number keys, q exits cleanly)

# Text mode still works (pipe-friendly)
cargo run -p roko-cli -- dashboard --text | head -20

# Start on specific page
cargo run -p roko-cli -- dashboard --page 4  # opens on Gates page

# Live updates during execution (two terminals):
# Terminal 1: cargo run -p roko-cli -- plan run plans/
# Terminal 2: cargo run -p roko-cli -- dashboard --page 2
# Verify: task phases update in real-time, agent output scrolls

# No panic on empty data (fresh install)
rm -rf .roko/signals.jsonl .roko/episodes.jsonl
cargo run -p roko-cli -- dashboard  # should show empty state, not crash

# NO_COLOR respected
NO_COLOR=1 cargo run -p roko-cli -- dashboard  # no color codes in output

# Resize handling
# (resize terminal window while dashboard is running — layout reflows)
```

---

## 1I: Skill Library + Playbook Wiring

> **Absorbs**: MASTER-REMAINING §B (Skill Library)
>
> **What exists**:
> - `roko-learn/src/skill_library.rs` — `SkillLibrary` with `extract_skill()` and `query()` (🏗️ BUILT, not called from orchestrate.rs)
> - `roko-learn/src/playbook.rs` — `PlaybookStore` with `record()` and `lookup()` (🏗️ BUILT, not called)
>
> **Goal**: When a task succeeds, record the context+prompt+model combo. When a similar
> task comes up, inject that successful pattern as guidance. This is the fastest-acting
> learning loop — works within a single session.

### Items

- [ ] Add `skill_library: SkillLibrary` field to `PlanRunner` in orchestrate.rs. Initialize from `.roko/learn/skills.json` (create if absent).
- [ ] On task success (gate pass + merge): call `skill_library.extract_skill()` with:
  - `task_files`: files the task touched (from agent output + `write_files`)
  - `task_tier`: complexity tier from tasks.toml
  - `symbols`: symbol names referenced (struct/function names from task description)
  - `model`: which model was used
  - `prompt_hash`: hash of the rendered system prompt
  - `gate_results`: which gates passed and their scores
- [ ] On task failure: call `skill_library.record_failure()` with same parameters, storing the failure pattern
- [ ] Before building context for a new task in dispatch: call `skill_library.query(task_files, task_tier, symbols)`. If a matching skill exists with success rate > 0.5, inject as a `Low`-priority context section in SystemPromptBuilder. Cap at 1024 tokens.
- [ ] Persist skill library to `.roko/learn/skills.json` after each extraction
- [ ] Add `playbook: PlaybookStore` field to `PlanRunner`. Initialize from `.roko/learn/playbooks.json`.
- [ ] On task success: call `playbook.record()` with task definition + outcome
- [ ] Before dispatch: call `playbook.lookup(task_type)` and inject as context if found

### Verification

```bash
grep -c 'skill_library\|SkillLibrary\|extract_skill' crates/roko-cli/src/orchestrate.rs  # >= 4
grep -c 'playbook\|PlaybookStore' crates/roko-cli/src/orchestrate.rs  # >= 2
# After a successful plan run:
test -f .roko/learn/skills.json && echo "skill file exists"
cargo test -p roko-cli --lib
```

---

## 1J: LinUCB Bandit + Context Attribution Wiring

> **Absorbs**: MASTER-REMAINING §C (LinUCB Bandit), §D (Context Attribution)
>
> **What exists**:
> - `roko-learn/src/model_router.rs` — LinUCB contextual bandit with 17-dim context vector, cold-start/confidence/UCB stages (🏗️ BUILT)
> - `roko-learn/src/cascade_router.rs` — 3-stage cascade (static → confidence → UCB), persists to `.roko/learn/cascade-router.json` (🔌 PARTIALLY WIRED — initialized, but `observe()` not called with real rewards)
> - Context attribution logs `was_referenced` per section to `context-attribution.jsonl` (🔌 WIRED)
> - Rolling averages + dynamic demotion NOT wired

### Items

**LinUCB Bandit**:
- [ ] Verify `CascadeRouter` is initialized from `.roko/learn/cascade-router.json` in `PlanRunner` — check it loads persisted state on startup
- [ ] After task success: call `cascade_router.observe(context_vec, model_idx, reward)` where:
  - `reward = pass_rate * 0.5 + (1.0 - normalized_cost) * 0.3 + (1.0 - normalized_duration) * 0.2`
  - `context_vec`: task tier (one-hot 4 dims), complexity scalar, iteration count, agent role hash, crate familiarity, prior failure flag — 17 dimensions total matching model_router.rs schema
  - `model_idx`: map model slug (haiku/sonnet/opus) to index in router's model list
- [ ] After task failure: call `cascade_router.observe(context_vec, model_idx, 0.0)` — zero reward
- [ ] Before model selection in `dispatch_agent_with()`:
  - Build context vector (same features as above)
  - Call `cascade_router.select(context_vec)` → returns model recommendation
  - If bandit has >50 observations for this context region, use its recommendation
  - Otherwise fall back to `task.effective_model()` (static tier mapping from tasks.toml)
- [ ] Track per-crate "familiarity score" = `success_count / total_count` for the specific crate being modified. Include in context vector dimension 15.
- [ ] Persist cascade router state after every observation — verify the existing persistence code actually triggers (currently `.save()` is called but verify the file updates)

**Context Attribution**:
- [ ] Maintain rolling average per `(task_tier, context_source_type)` in `.roko/learn/context-averages.json`. Use exponential moving average with alpha=0.1.
- [ ] When building context for a task: load rolling averages. For each context section type, check average reference rate for this task tier. If reference rate < 10%, demote priority from `Normal` to `Low` (droppable under token budget pressure in 5B).
- [ ] Log context attribution decisions: `[context] plan_brief: included (ref_rate=0.42)` / `[context] research: dropped (ref_rate=0.03)`
- [ ] Update rolling averages after each task completes (from attribution scan in agent output)

### Verification

```bash
# Bandit observations recorded
grep -c 'cascade_router\|observe.*reward' crates/roko-cli/src/orchestrate.rs  # >= 4

# Router state file updates after plan run
ls -la .roko/learn/cascade-router.json  # modification time changes after plan run

# Context averages tracked
cat .roko/learn/context-averages.json | python3 -m json.tool  # non-empty after plan run

# Bandit selects model (after enough observations)
cargo run -p roko-cli -- plan run plans/ 2>&1 | grep 'bandit.*selected\|cascade.*model'
```

---

# Tier 2: Agent Platform Foundation

> **Priority**: 🟠 P1 — Build the infrastructure for event-driven agents.
> **Depends on**: Tier 1 complete (especially 1A, 1C).
> **Detailed spec**: [`implementation-plans/11-sections/phase-0-1.md`](implementation-plans/11-sections/phase-0-1.md)

## 2A: Extract roko-serve

> **Spec**: phase-0-1.md § 0.1
>
> **What exists**:
> - `crates/roko-cli/src/serve/` — full HTTP server with 21 files (🔌 WIRED in roko-cli)
>   - Routes: learning, agents, config, plans, research, run, status, prds, ws
>   - Deployment: railway_api, railway_cli, manual
>   - State management, templates, events, error handling
>
> **Goal**: Move to `crates/roko-serve/` so other crates can embed the server.

### Items

The serve module currently has 21 files. Extraction must preserve all functionality.

- [ ] Create `crates/roko-serve/Cargo.toml` — copy serve-related dependencies from roko-cli's Cargo.toml: `axum`, `tower`, `tower-http`, `tokio`, `serde_json`, `askama` (if templates), `tokio-tungstenite` (for WS). Also depends on `roko-core`, `roko-agent`, `roko-learn`, `roko-gate`, `roko-fs`, `roko-compose`, `roko-orchestrator`, `roko-conductor`.
- [ ] Create `crates/roko-serve/src/lib.rs` with `pub mod` declarations for all submodules
- [ ] Move these files from `crates/roko-cli/src/serve/` → `crates/roko-serve/src/`:
  - `state.rs` (server state struct)
  - `events.rs` (event broadcaster, WS events)
  - `error.rs` (error types, HTTP error responses)
  - `templates.rs` (HTML templates, if any)
  - `middleware.rs` (auth, CORS, logging)
  - `routes/mod.rs` (router builder)
  - `routes/status.rs`, `routes/run.rs`, `routes/plans.rs`, `routes/agents.rs`
  - `routes/learning.rs`, `routes/config.rs`, `routes/research.rs`, `routes/prds.rs`
  - `routes/ws.rs` (WebSocket handler)
  - `deploy/` subdirectory (railway_api.rs, railway_cli.rs, manual.rs)
- [ ] Update roko-cli's `serve` subcommand handler to import from `roko-serve` and call `roko_serve::run_server(config)` instead of the inline serve module
- [ ] Remove `crates/roko-cli/src/serve/` directory (after verifying imports work)
- [ ] Add `roko-serve` to workspace `members` in root `Cargo.toml`
- [ ] Export a `ServerBuilder` in `roko-serve/src/lib.rs`:
  ```
  pub struct ServerBuilder { addr, config, state }
  impl ServerBuilder { pub fn new(config) -> Self; pub fn with_auth(key) -> Self; pub async fn run(self) -> Result<()>; }
  ```
  This allows `roko-cli` to construct the server, and future crates (like a test harness or daemon) to embed it.
- [ ] Extract `EventBus` from `events.rs` into its own type with `subscribe()` → `Receiver<Event>` and `publish(event)` methods. This becomes the shared event backbone that the webhook dispatch loop, executor, and WS broadcaster all use.
- [ ] Ensure all integration tests in roko-cli that test serve endpoints still pass after extraction

### Verification

```bash
cargo build -p roko-serve                    # new crate compiles
cargo build -p roko-cli                      # still compiles with roko-serve dependency
cargo run -p roko-cli -- serve &             # server starts
sleep 2
curl -s http://localhost:3000/api/status     # existing endpoints work
curl -s http://localhost:3000/api/plans      # plans endpoint works
kill %1

cargo test -p roko-serve                     # tests pass
cargo test -p roko-cli                       # CLI tests still pass

# No leftover serve module in roko-cli
test ! -d crates/roko-cli/src/serve && echo "cleaned up"
```

---

## 2B: Create roko-plugin SDK

> **Spec**: phase-0-1.md § 0.2
>
> **What exists**: Nothing — new crate.
>
> **Goal**: Define the `EventSource` and `FeedbackCollector` traits that all event
> sources (webhooks, cron, file watcher) implement. This is the extension point —
> anyone writing a new event source implements these traits.

### Items

- [ ] Create `crates/roko-plugin/Cargo.toml` — deps: `roko-core` (for Signal type), `async-trait`, `serde`, `tokio` (for `CancellationToken` and channels)
- [ ] Add `roko-plugin` to workspace `members` in root `Cargo.toml`
- [ ] Define `EventSource` trait:
  ```rust
  #[async_trait]
  pub trait EventSource: Send + Sync + 'static {
      fn name(&self) -> &str;
      fn kind(&self) -> EventSourceKind;  // Webhook, Cron, FileWatch, Custom
      async fn start(&self, sender: SignalSender, cancel: CancellationToken) -> Result<()>;
  }
  ```
  Must be object-safe (`Box<dyn EventSource>`). The `start` method runs until `cancel` fires, sending signals via `sender`.
- [ ] Define `FeedbackCollector` trait:
  ```rust
  #[async_trait]
  pub trait FeedbackCollector: Send + Sync + 'static {
      fn name(&self) -> &str;
      fn services(&self) -> Vec<String>;  // e.g., ["github", "slack"]
      fn interval(&self) -> Duration;      // how often to poll
      async fn collect(&self, since: DateTime<Utc>) -> Result<Vec<FeedbackSignal>>;
  }
  ```
  Used by the daemon to periodically check for outcomes of past agent actions (PR reviews, Slack reactions, etc.)
- [ ] Define `FeedbackSignal` struct: `original_episode_id: String`, `service: String`, `outcome: FeedbackOutcome` (Approved, Rejected, Commented, Ignored, Merged), `metadata: serde_json::Value`, `timestamp: DateTime<Utc>`
- [ ] Define `EventSourceKind` enum: `Webhook`, `Cron`, `FileWatch`, `Manual`, `Custom(String)`
- [ ] Define `SignalSender` as `tokio::sync::mpsc::Sender<Signal>` (re-export with a descriptive name)
- [ ] Define signal constants module:
  ```rust
  pub mod signal_kinds {
      pub const GITHUB_PUSH: &str = "github:push";
      pub const GITHUB_PR_OPENED: &str = "github:pull_request:opened";
      pub const GITHUB_PR_REVIEW: &str = "github:pull_request_review";
      pub const GITHUB_ISSUE_OPENED: &str = "github:issues:opened";
      pub const SLACK_MESSAGE: &str = "slack:message";
      pub const SLACK_REACTION: &str = "slack:reaction_added";
      pub const CRON_TICK: &str = "scheduler:cron";
      pub const FS_CHANGED: &str = "fswatcher:changed";
      pub const FS_CREATED: &str = "fswatcher:created";
      pub const MANUAL_TRIGGER: &str = "manual:trigger";
  }
  ```
- [ ] Define `PluginManifest` struct: `name: String`, `version: String`, `event_sources: Vec<Box<dyn EventSource>>`, `feedback_collectors: Vec<Box<dyn FeedbackCollector>>`
- [ ] Add `PluginBuilder` with fluent API: `PluginBuilder::new("my-plugin").event_source(src).feedback_collector(col).build()`
- [ ] Add integration tests:
  - Test with a `MockEventSource` that emits a signal after 100ms, verify it arrives on the receiver
  - Test that `CancellationToken::cancel()` stops the event source
  - Test that `Box<dyn EventSource>` is object-safe (compiles)
  - Test that `FeedbackCollector::collect()` returns empty vec when no feedback

### Verification

```bash
cargo build -p roko-plugin
cargo test -p roko-plugin

# Object safety check (compile-time)
# The test creates Box<dyn EventSource> and Box<dyn FeedbackCollector>
cargo test -p roko-plugin -- object_safe

# Mock event source test
cargo test -p roko-plugin -- mock_event_source
```

---

## 2C: Webhook Endpoints + Dispatch Loop

> **Spec**: phase-0-1.md § 1.1–1.4
>
> **What exists**:
> - HTTP routes in roko-serve (from 2A extraction)
> - `AgentDispatcher` in `crates/roko-agent/src/dispatcher/mod.rs` — dispatches agents via ClaudeCliAgent or ExecAgent (🔌 WIRED for plan execution)
> - `EventBroadcaster` in serve/events.rs (🔌 WIRED for WS, needs extension for dispatch)
>
> **Goal**: Accept webhooks → parse into Signals → route to agent templates → dispatch.
> This is the core event-driven agent loop — the central nervous system of the platform.

### Items

- [ ] Add `POST /webhooks/github` endpoint in roko-serve:
  1. Read raw body bytes + `X-Hub-Signature-256` header
  2. Compute HMAC-SHA256 of body with configured webhook secret (`roko.toml [webhooks.github] secret`)
  3. Compare computed vs received signature (constant-time comparison)
  4. Parse JSON body, extract event type from `X-GitHub-Event` header
  5. Map to `Signal` with kind from `signal_kinds::GITHUB_*`, payload = parsed JSON
  6. Publish signal to `EventBus`
  7. Return 200 immediately (processing is async)
- [ ] Add `POST /webhooks/slack` endpoint:
  1. Handle Slack URL verification challenge (return `challenge` field)
  2. Verify `X-Slack-Signature` header with signing secret
  3. Parse event payload, extract event type
  4. Map to `Signal` with kind `signal_kinds::SLACK_*`
  5. Return 200 immediately
- [ ] Add `POST /webhooks/generic` endpoint: accept any JSON, create signal with kind `webhook:generic`, payload = raw JSON. No signature verification (intended for internal use behind auth).
- [ ] Implement `DispatchLoop` — the central event routing engine:
  ```
  async fn dispatch_loop(event_bus: EventBus, subscriptions: SubscriptionRegistry, dispatcher: AgentDispatcher) {
      let mut rx = event_bus.subscribe();
      while let Some(signal) = rx.recv().await {
          let matched = subscriptions.find_matching(&signal);
          for sub in matched {
              if sub.check_concurrency_limit() && sub.check_cooldown() && sub.check_dedup(&signal) {
                  tokio::spawn(dispatch_agent(sub.template, signal.clone(), dispatcher.clone()));
              }
          }
      }
  }
  ```
- [ ] Wire `SubscriptionRegistry`: loads from `roko.toml` `[[subscriptions]]` array + `.roko/subscriptions/*.toml` files on startup. Each subscription specifies: `template` (agent template name), `trigger` (signal kind pattern, supports glob like `github:*`), `filter` (repo, branch, path globs), `concurrency_limit` (max N), `cooldown_secs` (min interval).
- [ ] Implement concurrency tracking: `SubscriptionRegistry` maintains `HashMap<subscription_id, AtomicUsize>` of active agent count. `check_concurrency_limit()` returns false if at limit. Decrement on agent completion.
- [ ] Implement cooldown: `HashMap<subscription_id, Instant>` of last dispatch time. `check_cooldown()` returns false if within cooldown window.
- [ ] Implement dedup: `HashMap<signal_content_hash, Instant>` with TTL window (default 60s). `check_dedup()` returns false if same signal content was seen within window.
- [ ] Wire `dispatch_agent()` function: takes template + signal → build system prompt (from template) → inject signal payload as context → call `AgentDispatcher::dispatch()` → log episode → collect output → run gates (if template specifies) → emit completion signal.
- [ ] Log all webhook events to `.roko/signals.jsonl` (via normal signal write path)
- [ ] Add `[webhooks]` config section to `roko.toml`:
  ```toml
  [webhooks.github]
  secret = "${GITHUB_WEBHOOK_SECRET}"
  enabled = true

  [webhooks.slack]
  signing_secret = "${SLACK_SIGNING_SECRET}"
  enabled = true
  ```

**Webhook Episode Logging** (from phase-7-8.md §7.1):
- [ ] Define `WebhookEpisodeMetadata` struct — extended metadata for event-driven agents (beyond plan-run episodes):
  ```rust
  pub struct WebhookEpisodeMetadata {
      pub trigger_kind: String,          // signal kind that triggered
      pub trigger_signal_hash: String,   // hash of trigger signal
      pub trigger_source: String,        // "github", "slack", "scheduler", "fswatcher"
      pub agent_template: String,        // template name used
      pub experiment_variant: Option<String>,  // A/B variant if experimenting
      pub external_actions: Vec<ExternalAction>,  // actions performed (PR reviews, Slack posts, etc.)
  }
  ```
- [ ] Wire episode logging in `dispatch_agent()`: wrap agent execution with episode start/end. Record `episode_id`, `agent_template`, `trigger_kind`, `trigger_signal_hash`, `started_at`, `completed_at`, `duration_secs`, `success`, `external_actions`, `model`, `turns`, `tokens_used`. Append to `.roko/episodes.jsonl`.
- [ ] Track `ExternalAction`s during execution — each tool call that mutates external state (GitHub review, Slack post, issue creation) emits an `ExternalAction { service, action_type, resource_id, metadata, performed_at }`. Tracked via `Arc<RwLock<Vec<ExternalAction>>>` shared with the agent.
- [ ] Feed episode into cascade router: `cascade_router.record_outcome(&template.model, result.success)` after each episode.
- [ ] Feed episode into efficiency tracker: `efficiency.record_event(&template_name, turns, tokens, success)`.

**Autonomous Feedback Collection** (from phase-7-8.md §7.3):
- [ ] Implement `start_feedback_loop()` in `roko-serve/src/feedback.rs` — background task running every 15 minutes. Loads recent episodes (last 24h) with external actions, polls external services for outcomes.
- [ ] Implement `collect_github_feedback()` — for each ExternalAction with `service = "github"`:
  - `review_pr`: check PR merged/closed/dismissed via `octocrab`. Compute sentiment: merged=+1.0, dismissed=-0.5, still open=skip.
  - `comment_issue`/`comment_pr`: fetch reactions on comment, count replies. Sentiment from positive/negative reactions.
  - `create_issue`: check if issue now closed, labels changed, assigned. Labels changed = acknowledged.
- [ ] Implement `collect_slack_feedback()` — for each ExternalAction with `service = "slack"`:
  - `post_message`/`reply_thread`: fetch reactions via `reactions.get` API, thread replies via `conversations.replies`. Positive reactions (👍,✅,🎉,❤️,🔥,🚀) vs negative (👎,❌,🚫). Count unique repliers.
- [ ] Implement polling schedule: first 24h → every 15min, days 2-7 → every 6h, after 7 days → stop polling.
- [ ] Wire feedback into experiment metrics: if episode had an experiment variant, convert feedback sentiment to metric value and record via `experiment_store.record_metric()`.
- [ ] Wire feedback into cascade router: positive feedback → quality signal for the model used.
- [ ] Record feedback signals in `.roko/signals.jsonl` with kind `feedback:{service}:{action_type}`.

### Verification

```bash
# Start server with webhooks enabled
cargo run -p roko-cli -- serve &
sleep 2

# GitHub webhook accepted (use test secret)
BODY='{"action":"opened","pull_request":{"number":1}}'
SIG=$(echo -n "$BODY" | openssl dgst -sha256 -hmac "test-secret" | awk '{print "sha256="$2}')
curl -s -w '%{http_code}' -X POST http://localhost:3000/webhooks/github \
  -H "Content-Type: application/json" \
  -H "X-GitHub-Event: pull_request" \
  -H "X-Hub-Signature-256: $SIG" \
  -d "$BODY"  # expect 200

# Signal appears in log
grep 'github:pull_request:opened' .roko/signals.jsonl

# Invalid signature rejected
curl -s -w '%{http_code}' -X POST http://localhost:3000/webhooks/github \
  -H "X-Hub-Signature-256: sha256=wrong" \
  -d '{}'  # expect 401

# Dispatch loop spawns agent (need matching subscription configured)
# Check agent process or episode log
grep 'webhook.*dispatch' .roko/episodes.jsonl

# Concurrency limit works (flood test)
for i in $(seq 1 10); do
  curl -s -X POST http://localhost:3000/webhooks/generic -d "{\"test\":$i}" &
done
wait
# Check that only N agents ran concurrently (from logs)

kill %1
```

---

## 2D: Integration MCP Servers

> **Spec**: [`implementation-plans/11-sections/phase-2.md`](implementation-plans/11-sections/phase-2.md)
> **Depends on**: 1C (MCP tool registry)
>
> **What exists**: Nothing — 3 new binary crates.
>
> **Goal**: Standalone MCP servers that agents connect to via `--mcp-config` for
> GitHub, Slack, and script execution.

### Items

All three are standalone binary crates that implement the MCP protocol over stdio (JSON-RPC 2.0).
Agents connect via `--mcp-config` which points to a JSON file listing server commands.
No changes to roko-agent needed — the existing MCP client handles the protocol.

**roko-mcp-github (GitHub API)**:
- [ ] Create `crates/roko-mcp-github/Cargo.toml` — deps: `serde`, `serde_json`, `reqwest`, `tokio`, `anyhow`. Binary crate.
- [ ] Implement stdio JSON-RPC transport: read line-delimited JSON from stdin, write responses to stdout
- [ ] Implement `initialize` handler: return server capabilities, tool list
- [ ] Implement `tools/list` handler: return all tool definitions
- [ ] Implement `tools/call` dispatcher: route to tool handlers by name
- [ ] Tool: `github_list_prs(owner, repo, state, per_page)` → list PRs with title, number, author, labels
- [ ] Tool: `github_get_pr(owner, repo, number)` → full PR details including diff stats, review state
- [ ] Tool: `github_create_pr(owner, repo, title, body, head, base)` → create PR, return URL
- [ ] Tool: `github_review_pr(owner, repo, number, body, event)` → submit review (APPROVE/REQUEST_CHANGES/COMMENT)
- [ ] Tool: `github_comment_pr(owner, repo, number, body)` → post comment on PR
- [ ] Tool: `github_merge_pr(owner, repo, number, merge_method)` → merge PR (merge/squash/rebase)
- [ ] Tool: `github_list_issues(owner, repo, state, labels, per_page)` → list issues
- [ ] Tool: `github_create_issue(owner, repo, title, body, labels, assignees)` → create issue
- [ ] Tool: `github_get_file(owner, repo, path, ref)` → get file contents (base64 decoded)
- [ ] Tool: `github_search_code(query, owner, repo)` → code search results
- [ ] Implement rate limiting: read `X-RateLimit-Remaining` header, sleep when <10 remaining. Exponential backoff on 429.
- [ ] Auth: read `GITHUB_TOKEN` from env. Return clear error if not set.

**roko-mcp-slack (Slack API)**:
- [ ] Create `crates/roko-mcp-slack/Cargo.toml` — same deps as github + Slack-specific types
- [ ] Same stdio JSON-RPC transport
- [ ] Tool: `slack_post_message(channel, text, thread_ts?)` → post message, return ts
- [ ] Tool: `slack_reply(channel, thread_ts, text)` → reply in thread
- [ ] Tool: `slack_react(channel, ts, emoji)` → add reaction
- [ ] Tool: `slack_list_channels(limit)` → list channels agent has access to
- [ ] Tool: `slack_get_thread(channel, thread_ts)` → get all messages in thread
- [ ] Tool: `slack_lookup_user(email_or_name)` → find user ID
- [ ] Tool: `slack_dm(user_id, text)` → send DM
- [ ] Rate limiting: respect `Retry-After` header on 429 responses
- [ ] Auth: read `SLACK_BOT_TOKEN` from env

**roko-mcp-scripts (Script Runner)**:
- [ ] Create `crates/roko-mcp-scripts/Cargo.toml`
- [ ] Tool: `run_script(name, args?)` → execute script from configured directory, return stdout/stderr
- [ ] Tool: `list_scripts()` → list available scripts with descriptions (read from `# description:` comment in first line)
- [ ] Sandboxing: set `timeout` (default 60s), working directory, env var allowlist. Use `tokio::process::Command` with `kill_on_drop(true)`.
- [ ] Script discovery: on startup, scan configured directories (from env `ROKO_SCRIPTS_DIR` or default `.roko/scripts/`). Index `.sh`, `.py`, `.js` files.
- [ ] Security: scripts must be in the configured directory (no path traversal). Reject `../` in script names.

**MCP lifecycle (in roko-serve/roko-cli)**:
- [ ] Wire auto-start: before dispatching an agent, check its template's `mcp_servers` list. For each, spawn the binary as a child process with stdin/stdout piped. Store process handle.
- [ ] Wire health check: after spawn, send `initialize` JSON-RPC call. If no response within 5s, log error and skip that MCP server.
- [ ] Wire auto-stop: after agent completes, kill MCP server processes. Or keep alive if another agent needs the same server (reference counting).
- [ ] Wire MCP config generation: dynamically generate the `mcp_config.json` file that gets passed to `--mcp-config` based on which servers are running.

### Verification

```bash
# GitHub MCP server starts and lists tools
echo '{"jsonrpc":"2.0","method":"tools/list","id":1}' | cargo run -p roko-mcp-github 2>/dev/null | python3 -m json.tool

# GitHub tool works (requires GITHUB_TOKEN)
echo '{"jsonrpc":"2.0","method":"tools/call","params":{"name":"github_list_prs","arguments":{"owner":"nunchi","repo":"roko","state":"open"}},"id":2}' \
  | cargo run -p roko-mcp-github 2>/dev/null | python3 -m json.tool

# Slack MCP server starts
echo '{"jsonrpc":"2.0","method":"tools/list","id":1}' | cargo run -p roko-mcp-slack 2>/dev/null | python3 -m json.tool

# Scripts MCP server starts
mkdir -p .roko/scripts && echo '#!/bin/bash\n# description: test script\necho hello' > .roko/scripts/test.sh && chmod +x .roko/scripts/test.sh
echo '{"jsonrpc":"2.0","method":"tools/list","id":1}' | ROKO_SCRIPTS_DIR=.roko/scripts cargo run -p roko-mcp-scripts 2>/dev/null | python3 -m json.tool

# Missing GITHUB_TOKEN returns clear error
unset GITHUB_TOKEN
echo '{"jsonrpc":"2.0","method":"tools/call","params":{"name":"github_list_prs","arguments":{}},"id":1}' \
  | cargo run -p roko-mcp-github 2>/dev/null | grep -i 'error.*token'
```

---

# Tier 3: Agent Templates & Events

> **Priority**: 🟡 P2 — Once platform foundation exists, define what agents do.
> **Depends on**: 2A, 2B, 2C.
> **Detailed spec**: [`implementation-plans/11-sections/phase-3-4.md`](implementation-plans/11-sections/phase-3-4.md)

## 3A: Agent Template Schema + 16 Templates

> **Spec**: phase-3-4.md § 3.0–3.16
>
> **What exists**: `SystemPromptBuilder` in roko-compose already builds 6-layer prompts.
> Templates need to feed into this builder, not replace it.

### Items

- [ ] Define `AgentTemplate` TOML schema in `crates/roko-serve/src/templates.rs` (or `roko-plugin`):
  ```toml
  name = "pr-reviewer"
  description = "Reviews pull requests for quality and correctness"
  model = "sonnet"                    # default model (cascade router can override)
  role = "reviewer"                   # maps to role-based tool restrictions (from 1C)
  system_prompt = """..."""           # template for system prompt (supports {{variables}})
  max_turns = 20                      # agent turn limit
  output_format = "markdown"          # expected output: markdown, json, toml, none
  mcp_servers = ["github"]            # which MCP servers this agent needs
  allowed_tools = ["Read", "Grep", "Glob", "github_*"]  # tool allowlist (glob patterns)
  denied_tools = ["Bash"]             # tool denylist

  [learning]
  track_efficiency = true             # emit efficiency events
  cascade_eligible = true             # cascade router can adjust model
  experiment_eligible = false         # don't A/B test this agent's prompts

  [feedback]
  collector = "github"                # which FeedbackCollector tracks outcomes
  success_signals = ["pr:merged"]     # what constitutes success
  failure_signals = ["pr:closed_without_merge"]

  [gates]
  run_gates = false                   # most webhook agents don't need gates
  # For code-modifying agents:
  # run_gates = true
  # gate_rungs = ["compile", "test", "clippy"]
  ```
- [ ] Implement template loader: on `roko serve` startup, scan `.roko/templates/` and `templates/` directories for `*.toml` files. Parse each into `AgentTemplate` struct. Log count and any validation errors.
- [ ] Wire template → dispatch: when dispatch loop matches a subscription to a template, build the agent invocation from template fields:
  - `model` → `--model` flag (or cascade router override)
  - `system_prompt` → inject into SystemPromptBuilder layer 1 (role prompt)
  - `max_turns` → `--max-turns` flag
  - `mcp_servers` → start required MCP servers, generate `--mcp-config`
  - `allowed_tools` / `denied_tools` → `--allowedTools` flag
  - `output_format` → include format instructions in system prompt
- [ ] Wire template variable interpolation: `{{signal.payload.pull_request.number}}`, `{{signal.payload.repository.full_name}}`, `{{env.GITHUB_TOKEN}}`, `{{timestamp}}`. Use a simple `{{key}}` → value replacement.
- [ ] Create 16 agent template TOML files in `.roko/templates/`:
  - [ ] `pr-reviewer.toml` — Trigger: `github:pull_request:opened|synchronize`. Reviews code quality, suggests improvements, posts review via `github_review_pr` tool. System prompt includes: review checklist, code style preferences, severity levels.
  - [ ] `issue-triager.toml` — Trigger: `github:issues:opened`. Reads issue, adds labels (bug/feature/docs/question), assigns to person based on file ownership, posts triage comment.
  - [ ] `code-implementer.toml` — Trigger: manual or `github:issues:labeled:implement`. Reads issue spec, generates code, runs gates, creates PR. Most complex template — role=implementer, gates=true.
  - [ ] `test-writer.toml` — Trigger: `github:push` (when src/ files changed). Identifies changed functions, generates unit tests, runs them, creates PR with tests.
  - [ ] `doc-updater.toml` — Trigger: `github:push` (when src/ public API changed). Updates README, API docs, inline docs. role=reviewer (read-only + doc edits).
  - [ ] `dependency-auditor.toml` — Trigger: `github:push` (when Cargo.toml/package.json changed). Checks for known vulnerabilities, license issues, breaking changes. Posts issue for findings.
  - [ ] `release-manager.toml` — Trigger: `manual` or cron (weekly). Collects merged PRs since last release, generates changelog, bumps version, creates release PR.
  - [ ] `security-scanner.toml` — Trigger: cron (daily) or `github:push`. Runs `cargo audit`, checks for hardcoded secrets, reviews permission patterns. Posts issues for findings.
  - [ ] `meeting-processor.toml` — Trigger: `fswatcher:created` (on call-notes/ directory). Reads transcript, extracts action items, creates GitHub issues per action.
  - [ ] `digest-agent.toml` — Trigger: cron (weekly). Aggregates: merged PRs, open issues, agent activity, cost summary. Posts to Slack.
  - [ ] `pm-board-agent.toml` — Trigger: cron (every 2h). Syncs GitHub issue state to TOML task files in the repo.
  - [ ] `research-agent.toml` — Trigger: manual. Deep research on a topic, produces structured report with citations.
  - [ ] `refactor-agent.toml` — Trigger: manual or cron (weekly). Scans codebase for code smells, duplication, complexity. Creates issues with refactoring suggestions.
  - [ ] `migration-agent.toml` — Trigger: `github:issues:labeled:migration`. Handles dependency version upgrades across the codebase.
  - [ ] `ci-fixer.toml` — Trigger: `github:check_run:completed:failure`. Reads CI failure logs, diagnoses issue, attempts fix, creates PR.
  - [ ] `onboarding-agent.toml` — Trigger: manual. Generates onboarding docs for new contributors based on codebase analysis.
- [ ] Wire template validation: on load, check required fields (`name`, `system_prompt`), valid model names, valid role names, MCP server names match configured servers. Log validation errors with file path.

**Prompt Experiments for Templates** (from phase-7-8.md §7.2):
- [ ] Wire `ExperimentStore` into dispatch loop — when template has `[experiment]` section, query `experiment_store.assign_variant(&experiment.name)` to get variant assignment.
- [ ] Implement variant-based prompt modification — modify system prompt based on variant:
  ```rust
  let prompt = match variant.as_str() {
      "concise" => format!("{}\n\n[STYLE: Be concise. Max 5 inline comments.]", template.system_prompt),
      "thorough" => format!("{}\n\n[STYLE: Be thorough. Review every file.]", template.system_prompt),
      _ => template.system_prompt.clone(),
  };
  ```
- [ ] Record experiment assignment in episode metadata — `experiment_variant` field in WebhookEpisodeMetadata.
- [ ] Wire feedback metrics into experiment outcomes — after feedback collection (2C), convert sentiment to metric and call `experiment_store.record_metric(&name, &variant, metric)`.
- [ ] Define concrete experiments for templates:
  | Template | Experiment | Variants | Metric | Measured By |
  |---|---|---|---|---|
  | `pr-reviewer` | review-depth | concise, thorough | `review_resolution_rate` | % of review comments resolved (feedback) |
  | `issue-triager` | triage-style | conservative, aggressive | `label_retention_rate` | % of labels not changed by humans |
  | `digest-agent` | digest-format | bullets, narrative | `thread_engagement` | Thread replies on Slack digest |

### Verification

```bash
# Templates load on server start
cargo run -p roko-cli -- serve 2>&1 | grep -E 'loaded.*template'

# List templates via CLI
cargo run -p roko-cli -- template list  # (add this subcommand)

# Template validation catches errors
echo 'name = "bad"' > .roko/templates/bad.toml  # missing system_prompt
cargo run -p roko-cli -- serve 2>&1 | grep 'validation.*bad.toml'

# Experiments: variants assigned
# Send webhook → check episode log for experiment_variant field
grep 'experiment_variant' .roko/episodes.jsonl | head -3

# Template variable interpolation works
# (send a webhook, verify the system prompt contains actual PR number)
```

---

## 3B: Subscription System

> **Spec**: phase-3-4.md § 3.17–3.21

### Items

- [ ] Define `Subscription` struct in `roko-serve` (or `roko-plugin`):
  ```rust
  pub struct Subscription {
      pub id: String,                       // unique ID (auto-generated or user-specified)
      pub template: String,                 // agent template name (e.g., "pr-reviewer")
      pub trigger: String,                  // signal kind pattern, supports glob: "github:pull_request:*"
      pub filter: SubscriptionFilter,       // additional filters
      pub concurrency_limit: usize,         // max concurrent agents for this subscription (default: 1)
      pub cooldown_secs: u64,               // min seconds between dispatches (default: 0)
      pub enabled: bool,                    // can be disabled without deleting
  }

  pub struct SubscriptionFilter {
      pub repos: Option<Vec<String>>,       // glob patterns: ["nunchi/*", "my-org/specific-repo"]
      pub branches: Option<Vec<String>>,    // regex patterns: ["main", "release/.*"]
      pub paths: Option<Vec<String>>,       // glob patterns: ["src/**/*.rs", "*.toml"]
      pub labels: Option<Vec<String>>,      // exact match: ["bug", "high-priority"]
      pub authors: Option<Vec<String>>,     // GitHub usernames
  }
  ```
- [ ] Wire subscription loading: read from two sources on startup:
  1. `roko.toml` `[[subscriptions]]` array (inline config)
  2. `.roko/subscriptions/*.toml` files (one per subscription, for dynamic management)
  Merge both into `SubscriptionRegistry`.
- [ ] Implement `SubscriptionRegistry::find_matching(signal: &Signal) -> Vec<&Subscription>`:
  1. Match `trigger` pattern against `signal.kind` (glob match with `*` and `**`)
  2. For each trigger match, evaluate `filter`:
     - `repos`: extract repo from signal payload (path varies by event type), glob match
     - `branches`: extract branch/ref, regex match
     - `paths`: extract changed file paths, glob match any
     - `labels`: extract labels, exact match any
     - `authors`: extract author login, exact match any
  3. Return all subscriptions where trigger + all filters pass
- [ ] Wire 21+ default subscriptions — these are auto-generated from the 16 templates. Each template gets at least one subscription. Some get multiple (e.g., `code-implementer` has both manual and label triggers). Store as TOML files in `.roko/subscriptions/`.
- [ ] Wire subscription CRUD API endpoints:
  - `GET /api/subscriptions` — list all with status
  - `POST /api/subscriptions` — create new
  - `PUT /api/subscriptions/:id` — update
  - `DELETE /api/subscriptions/:id` — remove
  - `POST /api/subscriptions/:id/enable` / `/disable` — toggle
- [ ] Add CLI commands:
  - `roko subscription list` — table of all subscriptions with template, trigger, enabled status
  - `roko subscription add --template pr-reviewer --trigger "github:pull_request:*"` — create
  - `roko subscription remove <id>` — delete
  - `roko subscription enable/disable <id>` — toggle

### Verification

```bash
# Subscriptions load from config
cargo run -p roko-cli -- subscription list  # shows all subscriptions

# Filter matching works
# Send a signal for repo "nunchi/roko" on branch "main"
# Subscription with repos=["nunchi/*"] should match
# Subscription with repos=["other-org/*"] should NOT match

# Concurrency limit enforced
# Configure subscription with concurrency_limit=1
# Send 2 rapid webhooks → only 1 agent starts, second queued

# CRUD via API
curl -X POST http://localhost:3000/api/subscriptions \
  -H "Content-Type: application/json" \
  -d '{"template":"pr-reviewer","trigger":"github:pull_request:*"}'
curl http://localhost:3000/api/subscriptions  # new subscription appears

# Enable/disable works
cargo run -p roko-cli -- subscription disable sub_abc123
cargo run -p roko-cli -- subscription list  # shows disabled
```

---

## 3C: Cron Scheduler + File Watcher

> **Spec**: phase-3-4.md § 4.1–4.3

### Items

**Cron Scheduler**:
- [ ] Implement `CronEventSource` struct that implements `EventSource` trait (from roko-plugin):
  ```rust
  pub struct CronEventSource {
      schedules: Vec<CronSchedule>,  // parsed from config
  }
  struct CronSchedule {
      name: String,           // "weekly-digest", "daily-security-scan"
      expression: String,     // standard cron: "0 9 * * MON" = 9am Monday
      signal_kind: String,    // "scheduler:cron:weekly-digest"
      metadata: Value,        // extra data to include in signal payload
  }
  ```
- [ ] Wire cron parsing: use the `cron` crate (or `tokio-cron-scheduler`) to parse cron expressions. Validate on startup — reject invalid expressions with clear error message including the schedule name.
- [ ] Wire scheduler loop: in `start()`, calculate next fire time for each schedule, sleep until earliest, emit signal, recalculate. Respects `CancellationToken` for clean shutdown.
- [ ] Add `[[scheduler.cron]]` config sections in `roko.toml`:
  ```toml
  [[scheduler.cron]]
  name = "weekly-digest"
  expression = "0 9 * * MON"
  signal_kind = "scheduler:cron:weekly-digest"

  [[scheduler.cron]]
  name = "daily-security"
  expression = "0 6 * * *"
  signal_kind = "scheduler:cron:daily-security"
  ```
- [ ] Wire signal emission: on cron fire, emit `Signal { kind: schedule.signal_kind, payload: { name, expression, fired_at } }` via `SignalSender`

**File Watcher**:
- [ ] Implement `FileWatchEventSource` struct implementing `EventSource`:
  - Use `notify` crate (cross-platform file system notifications)
  - Watch configured directories recursively
  - On file create/modify/delete: emit `Signal { kind: "fswatcher:created|modified|deleted", payload: { path, event_kind } }`
- [ ] Wire debounce: batch file events within a 500ms window. If multiple events for the same file occur within the window, emit only one signal with the latest event kind. Use `tokio::time::sleep` accumulator pattern.
- [ ] Wire path filtering: support include/exclude glob patterns in config so watchers don't fire for every `.git/` change or editor temp files.
- [ ] Add `[[watcher.paths]]` config sections in `roko.toml`:
  ```toml
  [[watcher.paths]]
  name = "call-notes"
  directory = "call-notes/"
  include = ["*.md", "*.txt"]
  exclude = ["*.tmp", ".DS_Store"]
  signal_kind = "fswatcher:created"  # only emit on creation, not modification
  recursive = true
  debounce_ms = 500
  ```

**Integration**:
- [ ] Wire both `CronEventSource` and `FileWatchEventSource` into the dispatch loop:
  - On `roko serve` startup, create instances from config
  - Call `event_source.start(sender, cancel)` for each
  - Signals flow into the same `EventBus` as webhook signals
  - Same subscription matching and dispatch applies
- [ ] Add `roko event-sources list` CLI command — shows configured cron schedules and file watchers with next fire time / watch status

### Verification

```bash
# Cron scheduler fires (use per-minute schedule for testing)
# Add to roko.toml: [[scheduler.cron]] name="test" expression="* * * * *" signal_kind="scheduler:cron:test"
cargo run -p roko-cli -- serve &
sleep 65  # wait for cron to fire
grep 'scheduler:cron:test' .roko/signals.jsonl  # signal emitted

# File watcher detects creation
# Add to roko.toml: [[watcher.paths]] name="test" directory="tmp/watch-test/" include=["*.md"]
mkdir -p tmp/watch-test
echo "test" > tmp/watch-test/new-file.md
sleep 1
grep 'fswatcher:created.*new-file.md' .roko/signals.jsonl

# Debounce works (rapid file saves don't flood)
for i in $(seq 1 10); do echo "edit $i" > tmp/watch-test/rapid.md; done
sleep 1
# Should see 1 signal, not 10
grep -c 'rapid.md' .roko/signals.jsonl  # = 1

# Event sources listed
cargo run -p roko-cli -- event-sources list

kill %1
```

---

# Tier 4: Daemon & Multi-Repo

> **Priority**: 🟡 P2 — Production deployment.
> **Depends on**: 2C, 3B.
> **Detailed spec**: [`implementation-plans/11-sections/phase-5-6.md`](implementation-plans/11-sections/phase-5-6.md)

## 4A: Daemon Mode

> **Spec**: phase-5-6.md § 5.1
>
> **What exists**:
> - `crates/roko-cli/src/daemon.rs` — scaffold with `DaemonState` enum (Starting/Running/Stopping/Stopped), `DaemonInfo` struct (pid, port, socket_path, started_at), socket path helpers. (🏗️ BUILT)
> - `roko serve` already runs the HTTP server foreground (🔌 WIRED)
>
> **What's missing**: The daemon.rs scaffold defines types but doesn't implement the actual daemonization (fork to background, PID file, log redirection, signal handling).

### Items

- [ ] Add `DaemonCmd` subcommand enum to `main.rs`:
  ```rust
  #[derive(Subcommand)]
  enum DaemonCmd {
      Start { #[arg(long)] foreground: bool, #[arg(long, default_value = "9090")] port: u16 },
      Stop,
      Status,
      Logs { #[arg(long, short = 'f')] follow: bool, #[arg(long, short = 'n', default_value = "50")] lines: usize },
      Reload,       // SIGHUP equivalent — re-scan subscriptions/templates without restart
      Restart { #[arg(long, default_value = "9090")] port: u16 },
      Install,      // macOS launchd plist generation
      Uninstall,    // remove launchd plist
  }
  ```
- [ ] Implement `daemon_start(foreground: bool, port: u16)` — full startup sequence:
  1. Check if already running: read `.roko/daemon.json`, check if PID is alive (`kill(pid, 0)`)
  2. If `foreground = false`: re-exec self with `--foreground` as a detached child process. Redirect stdout/stderr to `.roko/logs/daemon.log` / `.roko/logs/daemon.err`. Print PID and return.
     ```rust
     if !foreground {
         let exe = std::env::current_exe()?;
         let child = std::process::Command::new(exe)
             .args(["daemon", "start", "--foreground", "--port", &port.to_string()])
             .stdout(std::fs::File::create(log_path("daemon.log"))?)
             .stderr(std::fs::File::create(log_path("daemon.err"))?)
             .spawn()?;
         println!("daemon started (pid {})", child.id());
         return Ok(());
     }
     ```
  3. Write PID file and `DaemonInfo` to `.roko/daemon.json`: `{ pid, port, session_id, started_at, state: "running" }`
  4. Load config: `roko_core::config::load_config(&workdir)?`
  5. Build `AppState` (loads subscriptions, templates from all repos)
  6. Start HTTP server via `roko_serve::run_server_with_state(state, "0.0.0.0", port)`
  7. Start cron scheduler: `roko_serve::scheduler::start_scheduler(state)`
  8. Start file watchers: `roko_serve::fswatcher::start_watchers(state)`
  9. Start dispatch loop: `roko_serve::dispatch::start_dispatch_loop(state)`
  10. Start feedback collection loop: `roko_serve::feedback::start_feedback_loop(state)` (§7.3)
  11. Start Unix socket IPC server at `.roko/daemon.sock`
  12. Wait for shutdown signal (`tokio::signal::ctrl_c()` or SIGTERM)
- [ ] Implement IPC server on Unix socket (`.roko/daemon.sock`):
  ```rust
  async fn start_ipc_server(state: Arc<AppState>) -> Result<JoinHandle<()>> {
      let socket = socket_path();
      if socket.exists() { std::fs::remove_file(&socket)?; }
      let listener = tokio::net::UnixListener::bind(&socket)?;
      Ok(tokio::spawn(async move {
          loop {
              let (stream, _) = listener.accept().await?;
              let state = Arc::clone(&state);
              tokio::spawn(handle_ipc_command(stream, state));
          }
      }))
  }
  ```
  IPC commands: `"status"` → JSON with pid, active_agents, subscriptions, uptime_secs. `"reload"` → re-scan subscriptions and templates, return count. `"stop"` → trigger graceful shutdown.
- [ ] Implement `daemon_stop()`:
  1. Read PID from `.roko/daemon.json`
  2. Try IPC first: connect to `.roko/daemon.sock`, send `"stop"` command
  3. If IPC fails: send SIGTERM to PID
  4. Wait up to 30s for graceful shutdown (poll PID liveness)
  5. If still alive after 30s, send SIGKILL
  6. Remove `.roko/daemon.json` and `.roko/daemon.sock`
- [ ] Implement `daemon_status()`:
  1. Read `.roko/daemon.json`, check if PID alive
  2. Connect to IPC socket, send `"status"`, parse JSON response
  3. Print formatted table: state, PID, port, uptime, active agents, subscriptions, total signals processed
- [ ] Implement `daemon_logs(follow: bool, lines: usize)`:
  1. If `follow`: open `.roko/logs/daemon.log`, seek to end, poll for new lines with `tokio::fs::File` + `BufReader`
  2. If not follow: read last N lines (reverse-scan from end of file)
- [ ] Implement `daemon_reload()`: connect to IPC socket, send `"reload"`, print result. Equivalent to SIGHUP — re-reads `roko.toml`, reloads templates, reloads subscriptions, updates cron schedules. Does NOT restart active agents.
- [ ] Implement `daemon_restart()`: call `daemon_stop()` then `daemon_start()`
- [ ] Wire graceful shutdown sequence — on SIGTERM or IPC `"stop"`:
  1. Set `DaemonState::Stopping` in daemon.json
  2. Stop accepting new webhooks (drop HTTP listener)
  3. Cancel all `CancellationToken`s (stops cron scheduler, file watchers)
  4. Stop spawning new agents (drain dispatch queue)
  5. Wait for active agents to complete (up to 60s timeout per agent)
  6. Kill remaining agents via `ProcessSupervisor::kill_all()`
  7. Flush all logs, signals, episodes to disk
  8. Remove PID file and socket
  9. Set `DaemonState::Stopped` in daemon.json
  10. Exit 0
- [ ] Wire SIGHUP handler: install via `tokio::signal::unix::signal(SignalKind::hangup())`. On receive, execute same logic as IPC `"reload"`.

**Launchd Integration (macOS)**:
- [ ] Create `crates/roko-cli/src/daemon/launchd.rs` — generate macOS launchd plist:
  ```rust
  const LABEL: &str = "dev.nunchi.roko";
  fn plist_path() -> PathBuf {
      dirs::home_dir().unwrap().join("Library/LaunchAgents").join(format!("{LABEL}.plist"))
  }
  pub fn generate_plist(port: u16) -> String {
      // ProgramArguments: [exe, "daemon", "start", "--foreground", "--port", "{port}"]
      // KeepAlive: true, RunAtLoad: true
      // WorkingDirectory: $HOME
      // StandardOutPath: $HOME/.roko/logs/daemon.log
      // StandardErrorPath: $HOME/.roko/logs/daemon.err
      // EnvironmentVariables: PATH includes ~/.cargo/bin
  }
  ```
- [ ] Implement `daemon install` — write plist + `launchctl load -w <path>`. Ensure `~/.roko/logs/` exists.
- [ ] Implement `daemon uninstall` — `launchctl unload <path>` + remove plist file.

### Verification

```bash
# Start daemon in background
cargo run -p roko-cli -- daemon start --port 3001
cat .roko/daemon.json  # shows pid, port, session_id
ls .roko/daemon.sock   # IPC socket exists

# Status shows running (via IPC)
cargo run -p roko-cli -- daemon status
# Output: "Daemon running (pid XXXX, port 3001, uptime 5s, 0 active agents, 12 subscriptions)"

# HTTP endpoint works
curl -s http://localhost:3001/api/health  # responds

# Logs work
cargo run -p roko-cli -- daemon logs --lines 10
cargo run -p roko-cli -- daemon logs --follow  # streams live

# Reload without restart
cargo run -p roko-cli -- daemon reload
# Output: "Reloaded: 14 subscriptions, 16 templates"

# Stop is graceful
cargo run -p roko-cli -- daemon stop
test ! -f .roko/daemon.json && echo "cleaned up"
test ! -S .roko/daemon.sock && echo "socket cleaned"
curl -s http://localhost:3001/api/health  # connection refused

# Foreground mode works (for debugging)
cargo run -p roko-cli -- daemon start --foreground --port 3002  # blocks, ctrl-c to stop

# Restart works
cargo run -p roko-cli -- daemon start --port 3001
cargo run -p roko-cli -- daemon restart  # stop + start, new PID

# Launchd install (macOS)
cargo run -p roko-cli -- daemon install
launchctl list | grep roko  # shows loaded
cargo run -p roko-cli -- daemon status  # running

# Launchd uninstall
cargo run -p roko-cli -- daemon uninstall
launchctl list | grep roko  # not found
```

---

## 4B: Multi-Repo Configuration

> **Spec**: phase-5-6.md § 6.1–6.3

### Items

- [ ] Define `[[repos]]` config schema in roko.toml:
  ```toml
  [[repos]]
  name = "roko"
  path = "/Users/will/dev/nunchi/roko/roko"
  branch = "main"
  templates = ["pr-reviewer", "test-writer", "ci-fixer"]  # only these templates active
  # Inherits global subscriptions, can add repo-specific ones:
  [[repos.subscriptions]]
  template = "code-implementer"
  trigger = "github:issues:labeled:implement"

  [[repos]]
  name = "frontend"
  path = "/Users/will/dev/nunchi/frontend"
  templates = ["pr-reviewer", "doc-updater"]
  ```
- [ ] Implement `RepoRegistry` struct: loads `[[repos]]` on startup, validates paths exist, loads per-repo `.roko/` config
- [ ] Wire per-repo data isolation: each repo gets its own subdirectory under `.roko/repos/{repo_name}/`:
  - `.roko/repos/roko/signals.jsonl` (signals for this repo)
  - `.roko/repos/roko/episodes.jsonl` (episodes for this repo)
  - `.roko/repos/roko/state/` (executor state for this repo)
  - `.roko/repos/roko/learn/` (learning data for this repo)
  Global aggregation reads from all repo directories.
- [ ] Wire repo context in dispatch: when a webhook arrives with `repository.full_name`, match to configured repo. Set the agent's working directory to the repo's `path`. Pass repo-specific templates and subscriptions.
- [ ] Wire repo-local config override: if `{repo_path}/.roko/roko.toml` exists, merge it with global config (repo-local wins for per-repo settings, global wins for daemon settings)
- [ ] Wire cross-repo references: an agent in repo A can read files from repo B (if B is in `[[repos]]`) using absolute paths. The dispatch context includes all repo paths. Add a `repos` field to the agent's system prompt context listing available repos and their paths.

### Verification

```bash
# Multi-repo config loads
cargo run -p roko-cli -- daemon status  # shows configured repos

# Per-repo isolation
cargo run -p roko-cli -- plan run plans/ --repo roko
ls .roko/repos/roko/signals.jsonl  # repo-specific log

# Webhook routes to correct repo
curl -X POST http://localhost:3000/webhooks/github \
  -d '{"repository":{"full_name":"nunchi/roko"},...}'
# Verify agent starts with cwd = /Users/will/dev/nunchi/roko/roko
```

---

## 4C: Cloud Deployment

> **Spec**: phase-5-6.md § 6.4–6.5
>
> **What exists**:
> - `crates/roko-cli/src/serve/deploy/` — Railway deployment with:
>   - `railway_api.rs` — Railway API client (create project, deploy, get status) (🏗️ BUILT)
>   - `railway_cli.rs` — Railway CLI wrapper (🏗️ BUILT)
>   - `manual.rs` — manual deployment instructions (🏗️ BUILT)

### Items

**Container & Deployment**:
- [ ] Create `Dockerfile` at repo root:
  ```dockerfile
  FROM rust:1.91-bookworm AS builder
  WORKDIR /app
  COPY . .
  RUN cargo build --release -p roko-cli

  FROM debian:bookworm-slim
  RUN apt-get update && apt-get install -y ca-certificates git && rm -rf /var/lib/apt/lists/*
  COPY --from=builder /app/target/release/roko /usr/local/bin/
  RUN mkdir -p /data/.roko
  VOLUME /data/.roko
  EXPOSE 3000
  ENV ROKO_DATA_DIR=/data/.roko
  CMD ["roko", "daemon", "start", "--foreground", "--port", "3000"]
  ```
  Note: includes `git` (needed for cloud PlanRunner to clone repos) and `ca-certificates` (HTTPS).
- [ ] Create `fly.toml` template:
  ```toml
  app = "roko-agent"
  primary_region = "iad"
  [build]
  dockerfile = "Dockerfile"
  [http_service]
  internal_port = 3000
  force_https = true
  [[http_service.checks]]
  interval = "30s"
  timeout = "5s"
  path = "/api/health"
  method = "GET"
  [mounts]
  source = "roko_data"
  destination = "/data/.roko"
  ```
- [ ] Wire `roko deploy railway` subcommand: calls existing `railway_api.rs` to deploy:
  1. Build release binary: `cargo build --release -p roko-cli`
  2. Create/update Railway project via API
  3. Set environment variables (GITHUB_TOKEN, SLACK_TOKEN, etc.) via Railway API
  4. Configure volume mount for `.roko/` persistent data
  5. Deploy with health check endpoint `/api/health`
  6. Print deployed URL
- [ ] Wire `roko deploy fly` subcommand: generates `fly.toml`, runs `flyctl deploy`
- [ ] Wire `roko deploy docker` subcommand: `docker build -t roko . && docker tag roko:latest {registry}/roko:latest`
- [ ] Ensure health check endpoint (`GET /api/health`) returns 200 with `{"status":"ok","version":"...", "uptime_secs":...}`
- [ ] Add `--cloud` flag to `roko init` that generates cloud-optimized config:
  - `log_format = "json"` (structured logs to stdout for cloud log aggregation)
  - `bind = "0.0.0.0"` (not localhost)
  - `data_dir = "/data/.roko"` (volume-mounted path)

**Webhook Auto-Registration** (post-deploy):
- [ ] Implement `register_github_webhook()` — after deploy, auto-register the webhook URL on configured repos:
  ```rust
  async fn register_github_webhook(
      github: &octocrab::Octocrab, owner: &str, repo: &str,
      webhook_url: &str, secret: &str,
  ) -> Result<()> {
      github.repos(owner, repo).create_hook(
          "web",
          serde_json::json!({
              "url": format!("{webhook_url}/webhooks/github"),
              "content_type": "json",
              "secret": secret,
          }),
          vec!["push", "pull_request", "issues", "issue_comment",
               "pull_request_review", "check_run"],
      ).await?;
      Ok(())
  }
  ```
- [ ] Add `[serve.deploy]` config section:
  ```toml
  [serve.deploy]
  provider = "railway"  # or "fly"
  environment = ["GITHUB_TOKEN", "GITHUB_WEBHOOK_SECRET", "SLACK_BOT_TOKEN", "SLACK_SIGNING_SECRET"]
  # Auto-register webhooks after deploy
  [[serve.deploy.webhooks]]
  provider = "github"
  owner = "nunchi"
  repo = "roko"
  [[serve.deploy.webhooks]]
  provider = "github"
  owner = "nunchi"
  repo = "collaboration"
  ```
- [ ] Wire post-deploy hook: after successful deploy, iterate `[[serve.deploy.webhooks]]` and call `register_github_webhook()` for each. Skip if webhook already registered (check existing webhooks via API first).

**Remote Orchestrator Agents** (cloud-native plan execution):
- [ ] Define `CloudExecutionConfig` struct:
  ```rust
  pub struct CloudExecutionConfig {
      pub workspace_dir: PathBuf,     // default: /tmp/roko-workspace
      pub github_token: String,       // for cloning and pushing
      pub max_parallel: usize,        // default: 2
      pub cost_budget_cents: u64,     // default: 5000 ($50)
      pub timeout_secs: u64,          // default: 3600 (1 hour)
  }
  ```
- [ ] Implement cloud execution flow for `code-implementer` template:
  1. **Trigger**: `prd.plan_approved` signal (plan PR merged) or `github:issues:labeled:implement`
  2. **Clone**: `git clone --depth 1 https://x-access-token:{token}@github.com/{owner}/{repo}.git /tmp/roko-workspace/{repo}` — embed token in URL for ephemeral environments (no SSH keys)
  3. **Branch**: `git checkout -b impl/{plan-slug}`
  4. **Execute**: run `PlanRunner` with tasks from the plan
  5. **Gate**: after each task, run gates (compile, test, clippy)
  6. **Auto-fix**: if gate fails, `gate-fixer-agent` attempts repair (up to 3x, per 1E)
  7. **Commit**: `git add -A && git commit -m "task: {task_title}"` per task
  8. **Push**: `git push origin impl/{plan-slug}`
  9. **PR**: use `github_create_pr` MCP tool to open implementation PR
  10. **Cleanup**: remove workspace directory
- [ ] Implement git helper functions for cloud execution:
  ```rust
  async fn git_clone(url: &str, workspace: &Path, token: &str) -> Result<()>;   // rewrite URL with token
  async fn git_checkout_new_branch(workspace: &Path, branch: &str) -> Result<()>;
  async fn git_commit(workspace: &Path, message: &str) -> Result<()>;           // add -A, check diff, commit
  async fn git_push(workspace: &Path, branch: &str, token: &str) -> Result<()>; // push with token auth
  ```
  All helpers: use `tokio::process::Command`, scrub token from error output, set `GIT_TERMINAL_PROMPT=0`.
- [ ] Wire persistent storage: daemon stores all state in `.roko/`. Cloud deployments need a volume mount to persist between deploys. Required volume paths: `.roko/signals.jsonl`, `.roko/episodes.jsonl`, `.roko/learn/`, `.roko/state/`, `.roko/neuro/`, `.roko/dreams/`.

### Verification

```bash
# Docker build works
docker build -t roko .
docker run -p 3000:3000 -e GITHUB_TOKEN=test roko &
sleep 3
curl -s http://localhost:3000/api/health  # {"status":"ok","version":"..."}
docker stop $(docker ps -q --filter ancestor=roko)

# Volume persistence
docker run -v roko_data:/data/.roko -p 3000:3000 roko &
# (run some operations)
docker stop $(docker ps -q --filter ancestor=roko)
docker run -v roko_data:/data/.roko -p 3000:3000 roko &
# (verify .roko/ data persisted between runs)

# Railway deploy (dry run)
cargo run -p roko-cli -- deploy railway --dry-run  # shows what would be deployed

# Fly.io config generated
cargo run -p roko-cli -- deploy fly --init  # generates fly.toml
test -f fly.toml
flyctl deploy --remote-only  # deploys to Fly.io

# Webhook auto-registration
# After deploy, check GitHub repo settings:
curl -s -H "Authorization: Bearer $GITHUB_TOKEN" \
  https://api.github.com/repos/nunchi/roko/hooks | python3 -m json.tool
# Should show webhook pointing to deployed URL

# Cloud execution flow
# 1. Label a GitHub issue with "implement"
# 2. Webhook fires → cloud daemon receives
# 3. code-implementer template matches subscription
# 4. Agent clones repo, creates branch, implements, pushes, creates PR
# Verify: PR appears on GitHub with impl/{slug} branch

# Cloud init generates correct config
cargo run -p roko-cli -- init --cloud
grep 'log_format.*json' roko.toml
grep 'bind.*0.0.0.0' roko.toml
```

---

## 4D: Secret Management

> **Spec**: phase-5-6.md § 6.6

### Items

- [ ] Wire `.env` file loading: on startup (before config parse), load environment variables from:
  1. `~/.roko/.env` (global — user-wide tokens)
  2. `./.env` (local — repo-specific overrides)
  Use the `dotenvy` crate (Rust .env loader). Local overrides global.
- [ ] Wire `${VAR}` interpolation in `roko.toml` parser: before deserializing TOML, scan string values for `${VAR}` patterns and replace with `std::env::var("VAR")`. Return clear error if variable not set: "Config error: ${GITHUB_TOKEN} referenced but GITHUB_TOKEN not set. Set it in .env or environment."
- [ ] Wire secret masking in logs: add a `tracing-subscriber` layer that scans log output for patterns matching known secret formats:
  - `ghp_[A-Za-z0-9]{36}` (GitHub PATs)
  - `sk-[A-Za-z0-9-]+` (API keys)
  - `xoxb-[0-9]+-[A-Za-z0-9]+` (Slack bot tokens)
  - Any value loaded from `.env` files
  Replace matches with `[REDACTED:VAR_NAME]` in log output.
- [ ] Wire secret masking in API responses: the `/api/config` endpoint should never return raw secret values. Replace with `"***"` and a note about which env var to set.
- [ ] Document required env vars: add a `REQUIRED_ENV` section to the default `roko.toml` template:
  ```toml
  # Required environment variables (set in .env or shell):
  # GITHUB_TOKEN       — GitHub personal access token (for MCP GitHub server)
  # SLACK_BOT_TOKEN    — Slack bot token (for MCP Slack server)
  # SLACK_SIGNING_SECRET — Slack webhook signing secret
  # ANTHROPIC_API_KEY  — Claude API key (for direct API agents, not needed for CLI agents)
  ```
- [ ] Add `roko config check-secrets` command: verify all referenced `${VAR}` tokens in roko.toml are set, verify tokens are valid (test GitHub API, test Slack API), report which are missing/invalid.
- [ ] Add `roko config set-secret NAME VALUE` command: appends `NAME=VALUE` to `~/.roko/.env` (with appropriate file permissions: `chmod 600`). If NAME already exists, update it.

### Verification

```bash
# .env loading works
echo "TEST_VAR=hello" > .env
# Add to roko.toml: test_setting = "${TEST_VAR}"
cargo run -p roko-cli -- config show | grep 'test_setting.*hello'

# Missing env var gives clear error
# Add to roko.toml: missing_setting = "${DOES_NOT_EXIST}"
cargo run -p roko-cli -- config show 2>&1 | grep 'DOES_NOT_EXIST not set'

# Secret masking in logs
echo "GITHUB_TOKEN=ghp_test1234567890abcdefghijklmnop1234567" > .env
ROKO_LOG=debug cargo run -p roko-cli -- status 2>&1 | grep -c 'ghp_test'  # = 0
ROKO_LOG=debug cargo run -p roko-cli -- status 2>&1 | grep -c 'REDACTED'  # > 0

# Config check-secrets
cargo run -p roko-cli -- config check-secrets
# Output: "GITHUB_TOKEN: ✅ set, valid (scopes: repo, read:org)"
# Output: "SLACK_BOT_TOKEN: ❌ not set"

# Set secret
cargo run -p roko-cli -- config set-secret GITHUB_TOKEN ghp_xxxxx
stat -f '%Lp' ~/.roko/.env  # 600 (owner read/write only)
```

---

# Tier 5: Cognitive Layer

> **Priority**: 🟢 P3 — Agent intelligence and memory.
> **Depends on**: Tier 1 (can run in parallel with Tiers 2-4).
> **Detailed spec**: [`implementation-plans/12a-cognitive-layer.md`](implementation-plans/12a-cognitive-layer.md)
> **Detailed spec (learning)**: [`implementation-plans/11-sections/phase-7-8.md`](implementation-plans/11-sections/phase-7-8.md)
>
> **Source docs** (read before implementing any Tier 5 item):
> - `bardo-backup/prd/04-memory/` — grimoire (knowledge store), HDC fingerprints, memetic knowledge, emotional memory
> - `bardo-backup/prd/03-daimon/` — affect model (PAD vector), appraisal, behavior modulation, runtime integration
> - `bardo-backup/prd/05-dreams/` — offline learning, replay, imagination, consolidation
> - `bardo-backup/prd/12-inference/04-context-engineering.md` — context assembly, active inference scoring, attention curves
> - `bardo-backup/prd/12-inference/01-deployment-modes.md` — operating frequencies, tier routing
> - `bardo-backup/prd/12-inference/01a-routing.md` — model routing (cascade/UCB)
> - `bardo-backup/tmp/agent-chain/04-hdc.md` — HDC vector theory (10,240-bit BSC, Hamming similarity)
> - `bardo-backup/tmp/agent-chain/15-dynamic-context-assembly.md` — context assembly research
> - `bardo-backup/tmp/roko-progress/COMPONENTS/learn/` — per-component specs for episode logger, playbook, skill library, dream consolidation, pattern discovery, baseline computation
> - Existing code: `crates/bardo-primitives/src/` (HDC), `crates/roko-index/src/` (symbol graph), `crates/roko-learn/src/` (episodes, playbooks, bandits, patterns, baselines, clustering)
>
> **Academic foundations** (full bibliography in `bardo-backup/tmp/agent-chain/08-references.md`):
> - **HDC**: [Kanerva 2009] Hyperdimensional Computing; [Neubert 2022] VSA survey; [Kleyko 2022] HDC survey
> - **Cognitive arch**: [Sumers 2023] CoALA framework (perceive→retrieve→reason→act→learn) → Gamma/Theta/Delta
> - **PAD affect**: [Mehrabian 1996] Pleasure-Arousal-Dominance emotional model
> - **Context assembly**: [Liu 2023] Lost in the Middle (U-shaped attention curve); [Lewis 2020] RAG
> - **Memory/reflection**: [Park 2023] Generative Agents (memory + reflection + planning)
> - **Stigmergy**: [Grassé 1959] (original term); [Dorigo 1997] ant colony optimization (reinforcement/evaporation)
> - **Collective intelligence**: [Woolley 2010] c-factor (Science 330(6004)) — social sensitivity + turn-taking > max individual ability
> - **Self-improvement**: Meta-Harness (Stanford 2026) — 6× gap from harness changes alone
> - **Full reading order**: `agent-chain/14-academic-foundations.md` → `04-hdc.md` → `03-stigmergy.md` → `05-knowledge-layer.md` → `15-dynamic-context-assembly.md`

## 5A: roko-neuro (Knowledge + Memory)

> **Spec**: 12a-cognitive-layer.md § D
>
> **What exists**:
> - `crates/bardo-primitives/src/` — HDC (Hyperdimensional Computing) primitives (🏗️ BUILT, not called):
>   - `pad.rs` — `PadVector` (10,000-dim binary vector), encode/decode, bundling, binding
>   - `tier.rs` — tier classification using HDC similarity
>   - `fingerprint.rs` — content fingerprinting via HDC
> - `crates/roko-index/src/` — code indexing (🏗️ BUILT, not called):
>   - `graph.rs` — dependency graph builder
>   - `parser.rs` — Rust/TS/Go AST parsing
>   - `hdc.rs` — HDC-based code fingerprints
> - `crates/roko-learn/src/` — learning infrastructure (🔌 WIRED):
>   - `episodes.rs` — `EpisodeStore` with JSONL persistence
>   - `playbooks.rs` — `PlaybookStore` for reusable patterns
>   - `bandits.rs` — multi-armed bandit for exploration/exploitation
>
> **Reference**:
> - PRD grimoire: `/Users/will/dev/nunchi/roko/bardo-backup/prd/07-grimoire/`
> - Component spec: `/Users/will/dev/nunchi/roko/bardo-backup/tmp/roko-progress/COMPONENTS/grimoire.md`

### Items

- [ ] Create `crates/roko-neuro/Cargo.toml` — deps: `roko-core`, `roko-fs` (for persistence), `serde`, `serde_json`, `chrono`, `anyhow`. Optionally depend on `bardo-primitives` for HDC support (feature-gated: `features = ["hdc"]`).
- [ ] Add `roko-neuro` to workspace members in root `Cargo.toml`
- [ ] Define `KnowledgeEntry` struct:
  ```rust
  pub struct KnowledgeEntry {
      pub id: String,                    // unique ID
      pub kind: KnowledgeKind,           // Fact, Procedure, Heuristic, Constraint, AntiKnowledge
      pub content: String,               // the actual knowledge
      pub confidence: f64,               // 0.0-1.0, decays over time
      pub source_episodes: Vec<String>,  // episode IDs that contributed this knowledge
      pub tags: Vec<String>,             // topic tags for retrieval
      pub created_at: DateTime<Utc>,
      pub half_life_days: f64,           // knowledge decay rate (default: 30 days)
      pub hdc_vector: Option<Vec<u8>>,   // HDC fingerprint for similarity search (if hdc feature enabled)
  }
  ```
- [ ] Implement `KnowledgeStore`:
  - Storage: append-only JSONL at `.roko/neuro/knowledge.jsonl` (consistent with other stores)
  - `add(entry: KnowledgeEntry) -> Result<()>` — append to store
  - `query(topic: &str, limit: usize) -> Result<Vec<KnowledgeEntry>>` — retrieve relevant entries. Scoring: keyword match (tags + content) × confidence × recency. If HDC enabled, also HDC cosine similarity.
  - `decay()` — reduce confidence of old entries based on `half_life_days`. Called periodically (on dream cycle or startup).
  - `gc(min_confidence: f64)` — remove entries below confidence threshold. Default: 0.05.
- [ ] Implement `Distiller`:
  - Input: list of episodes (from `EpisodeStore`)
  - Output: list of `KnowledgeEntry` candidates
  - Logic: for each episode, extract:
    - **Facts**: "file X contains struct Y", "function Z takes 3 arguments"
    - **Procedures**: "to fix clippy warning W, change pattern P to Q"
    - **Heuristics**: "tasks with >200 LOC changes usually fail gates" (derived from episode patterns)
    - **Constraints**: "never modify file X without also updating Y" (derived from repeated failures)
  - Implementation: Build a distillation prompt, dispatch to a small model (haiku). Parse structured output. Cheaper than opus for knowledge extraction.
- [ ] Implement `MemoryIndex` (if HDC feature enabled):
  - Build HDC vectors for each knowledge entry's content using `bardo-primitives::fingerprint`
  - Similarity search: given a query string, fingerprint it, find top-K entries by HDC cosine similarity
  - Faster than keyword search for semantic queries
- [ ] Wire into agent dispatch in `orchestrate.rs`: before building system prompt, call `knowledge_store.query(task.description, 5)`. Format top entries as a "Relevant Knowledge" section in the system prompt (via SystemPromptBuilder's context layer).
- [ ] Wire into episode completion: after `EpisodeLogger` records a completed episode, call `distiller.distill(episode)`. Add resulting entries to `KnowledgeStore`. This can be async (doesn't block task completion).
- [ ] Add `roko neuro query "<topic>"` CLI command: search knowledge store, print matching entries with confidence scores
- [ ] Add `roko neuro stats` CLI command: total entries by kind, average confidence, oldest/newest entry
- [ ] Add `roko neuro gc` CLI command: run garbage collection, report removed entries

**4-Tier Distillation Pipeline** (from 12a-cognitive-layer.md §D.1):
- [ ] Implement tier progression: Raw Episodes → Insights → Heuristics → PLAYBOOK. Each tier compresses and validates:
  - **D1**: Episode → Insight extraction — "When X happened, Y consistently followed". Requires pattern detection across 3+ episodes.
  - **D2**: Insight → Heuristic promotion — 3+ independent confirmations → actionable rule. Confidence threshold: 0.7.
  - **D3**: Heuristic → PLAYBOOK compilation — top heuristics → human-readable `PLAYBOOK.md` with machine-parseable action rules.
- [ ] Implement temporal decay per knowledge type — configurable half-lives: Insights=30d, Heuristics=90d, Facts=365d.
- [ ] Implement confirmation boost — independent validation extends weight by 1.5×. Prevents premature decay of validated knowledge.
- [ ] Verification: 5-task plan produces ≥1 insight. Insights with <5 episodes stay at "insight" tier; ≥5 promotes to "heuristic". Entries older than 2× half-life have confidence <0.5.

**HDC Integration Points** (from phase-7-8.md §7.4):
- [ ] Wire signal fingerprinting — in webhook handler, after constructing Signal, compute HDC fingerprint via `bardo_primitives::hdc::fingerprint(&signal.body)`. Store in `signal.metadata["hdc_fingerprint"]`. Feature-gated: only when `hdc` feature enabled.
- [ ] Wire episode fingerprinting — after episode completion, fingerprint the episode text (`trigger_kind + agent_template + actions + outcome`) via `bardo_primitives::hdc::text_fingerprint()`. Store in episode metadata.
- [ ] Implement similarity-based template suggestion — when no exact subscription matches a signal, use HDC cosine similarity against recent episode fingerprints. If similarity >0.7, suggest the matching episode's template. Fallback only — exact subscription match always takes priority.
- [ ] Verification: signals have `hdc_fingerprint` in metadata. Similar events cluster (cosine >0.7). Template suggestion works for unmatched events.

### Verification

```bash
cargo build -p roko-neuro
cargo test -p roko-neuro

# Knowledge store round-trip
# Add entry programmatically, query it back
cargo test -p roko-neuro -- knowledge_store_roundtrip

# Distiller extracts knowledge from episodes
# Create test episode, run distiller, verify knowledge entries produced
cargo test -p roko-neuro -- distiller_extracts_knowledge

# CLI commands work
cargo run -p roko-cli -- plan run plans/  # generates episodes
cargo run -p roko-cli -- neuro query "tasks"  # finds relevant knowledge
cargo run -p roko-cli -- neuro stats  # shows counts

# Knowledge appears in system prompt
cargo run -p roko-cli -- plan run plans/ 2>&1 | grep 'Relevant Knowledge'  # section present
```

---

## 5B: Context Assembly (5-Stage Pipeline)

> **Spec**: 12a-cognitive-layer.md § E
>
> **What exists**: `SystemPromptBuilder` in roko-compose already builds 6-layer prompts:
> Layer 1 (role), Layer 2 (global rules), Layer 3 (task context), Layer 4 (tools),
> Layer 5 (output format), Layer 6 (meta-instructions). Currently Layer 3 is just the
> raw task description. This section enriches Layer 3 with assembled context.

### Items

- [x] Create `ContextAssembler` struct in `roko-compose/src/context_assembler.rs` (or `roko-neuro/src/context.rs`): ✅ (codex batch4, 5B.01)
  ```rust
  pub struct ContextAssembler {
      knowledge_store: Arc<KnowledgeStore>,
      episode_store: Arc<EpisodeStore>,
      max_context_tokens: usize,  // budget for assembled context (default: 4000 tokens)
  }
  ```
- [x] Implement Stage 1 — Gather: ✅ (codex batch4, 5B.02 — queries KnowledgeStore for matching entries)
- [x] Implement Stage 2 — Rank: ✅ (codex batch4, 5B.03 — active inference scoring from 12a §E2)
  - **Active inference scoring** (from 12a §E2): `score = track_record(entry) × belief_change(entry) / uncertainty`. Pragmatic value (how useful was this knowledge before) × epistemic value (how much would this change the agent's beliefs) balanced by uncertainty.
  - Fallback formula if insufficient data: `score = keyword_overlap * 0.3 + recency * 0.2 + confidence * 0.3 + source_priority * 0.2`
  - `source_priority`: knowledge entries > recent episodes > read_files > signals
  - Sort by score descending
  - If HDC enabled: use HDC Hamming similarity instead of keyword_overlap (better semantic matching)
- [x] Implement **attention-curve positioning** (from 12a §E3, Liu et al. U-shape): ✅ (codex batch4, 5B.04)
- [x] Implement **affect-modulated retrieval** (from 12a §E4): ✅ (codex batch4, 5B.05 — PAD state biases retrieval)
- [x] Implement Stage 3 — Compress: ✅ (codex batch4, 5B.06 — chunks below 50th percentile summarized)
- [x] Implement Stage 4 — Inject: ✅ (codex batch4, 5B.07)
  - Format assembled context as a structured section:
    ```
    ## Relevant Context
    ### Knowledge
    - [Heuristic] Tasks with >100 LOC usually need decomposition (confidence: 0.85)
    - [Procedure] To fix borrow checker errors in this crate, check lifetime annotations in... (confidence: 0.72)
    ### Recent Experience
    - Task "wire-gates" completed successfully: key insight was...
    ### Related Files
    - src/executor.rs (230 lines): defines ExecutorConfig, ExecutorState...
    ```
  - Pass this as the context_layer to `SystemPromptBuilder::with_context()`
- [x] Implement Stage 5 — Validate: ✅ (codex batch4, 5B.08 — total token count check)
- [x] Wire into `orchestrate.rs`: replace the current inline context building with `ContextAssembler::assemble(task) -> ContextSection`. Feed result into `SystemPromptBuilder`. ✅ (codex batch4, 5B.09)
  ⚠️ **Depth pass still needed**: verify active inference scoring formula matches 12a §E2 reference; verify U-shape attention curve implementation matches Liu et al. paper; verify affect-modulated retrieval integrates properly with DaimonState (5C).

### Verification

```bash
# Context assembler enriches prompts
# Run a plan, check that system prompt includes "Relevant Context" section
cargo run -p roko-cli -- plan run plans/ 2>&1 | grep -A3 'Relevant Context'

# Budget is respected
# Set max_context_tokens = 100, verify context is truncated
cargo test -p roko-compose -- context_budget_respected

# Ranking works — higher-confidence knowledge appears first
cargo test -p roko-compose -- context_ranking_order

# Token validation catches oversized prompts
cargo test -p roko-compose -- context_token_validation
```

---

## 5C: Daimon (Affect/Motivation)

> **Spec**: 12a-cognitive-layer.md § F
>
> **Reference**:
> - PRD: `/Users/will/dev/nunchi/roko/bardo-backup/prd/11-daimon/`
>
> **What exists**: Nothing yet. The cascade router already adjusts model selection
> based on task history, but it doesn't model agent "affect" or motivation.

### Items

- [x] Create `crates/roko-daimon/Cargo.toml` — deps: `roko-core`, `serde`, `serde_json`, `chrono`, `anyhow` ✅ (predates batch4)
- [x] Add `roko-daimon` to workspace members ✅ (codex batch4, 5C.02)
- [x] Define `AffectState` struct using full PAD model (from 12a §F1 — Pleasure, Arousal, Dominance): ✅ (codex batch4, 5C.03 — `AffectState` at roko-daimon/src/lib.rs:51-68, with `PadVector { pleasure, arousal, dominance }` all [-1,1], `confidence: f64` [0,1] default 0.5)
  ```rust
  pub struct AffectState {
      pub pleasure: f64,     // -1.0 to 1.0. Success → +, failure → -. (Maps to "valence")
      pub arousal: f64,      // -1.0 to 1.0. Time pressure/urgency → +, idle → -.
      pub dominance: f64,    // -1.0 to 1.0. Agency/control → +, blocked/stuck → -.
      pub updated_at: DateTime<Utc>,
  }
  ```
- [x] Implement 8 named affect states from PAD octants (from 12a §F2): ✅ (codex batch4, 5C.04 — `+P+A+D` = Exuberant, etc.)
- [x] Implement `AffectEngine` appraisal triggers (from 12a §F3): ✅ (codex batch4, 5C.05 — 8 event types defined: GateResult, TaskOutcome, Blocked, TimePressure, QueueWait, DreamFailure. See gap analysis §47.5 for exact deltas. Decay half-life=4h, decays toward neutral 0.5 not 0.0)
  ⚠️ **GAP**: Only `GateResult` event is fired from orchestrate.rs (line 3110). `TaskOutcome`, `Blocked`, `TimePressure`, `QueueWait`, `DreamFailure` are defined but NEVER called from the plan-runner. See gap 47m.
- [x] Implement affect → behavior modulation table (from 12a §F5): ✅ (codex batch4, 5C.06 — `modulate()` at lib.rs:393-415: Escalating/Exploratory/Conservative/Proactive/Balanced strategies, adjusts turn_limit and model tier)
- [x] Wire affect signatures on episodes (from 12a §F6): ✅ (codex batch4, 5C.07)
- [x] Wire affect → SystemPromptBuilder (from 12a §F7): ✅ (codex batch4, 5C.08)
- [x] Wire affect into task prioritization in executor: ✅ (codex batch4, 5C.09)
- [x] Wire motivation decay: ✅ (codex batch4, 5C.10)
- [x] Wire affect → cascade router integration: ✅ (codex batch4, 5C.11 — `confidence < 0.30 OR dominance < -0.25` → Escalating strategy + model promotion)
- [x] Persist affect state: store in `.roko/daimon/affect.json`. Load on startup, save after each state change. ✅ (codex batch4, 5C.12 — `DaimonState::load_or_new()`, autosave after every appraise via `.json.tmp` + `fs::rename`)
- [x] Emit affect signals: ✅ (codex batch4, 5C.13)
- [x] Wire into dashboard: ✅ (codex batch4, 5C.14)
  ⚠️ **Depth pass still needed (Pass 4)**: This is a shallow PAD model, NOT the full OCC/Scherer 8-step appraisal pipeline from the PRDs. Missing: somatic k-d tree landscape, ALMA three-layer EMA (emotion→mood→personality, Gebhard 2005), learned helplessness detection (D < -0.3 for 200+ ticks), mood-congruent memory retrieval (Bower 1981), cross-agent affect contagion, negativity bias 1.6x (Kahneman-Tversky).

### Verification

```bash
cargo build -p roko-daimon
cargo test -p roko-daimon

# Affect updates on success/failure
cargo test -p roko-daimon -- affect_on_success  # confidence increases
cargo test -p roko-daimon -- affect_on_failure  # valence decreases

# Affect persists
cargo test -p roko-daimon -- affect_persistence  # save + reload

# Low confidence triggers model escalation (integration test)
# Run task that fails twice → confidence drops → next retry uses stronger model
cargo run -p roko-cli -- plan run plans/test-affect/ 2>&1 | grep 'confidence.*escalating'

# Affect signals emitted
grep 'daimon:affect' .roko/signals.jsonl
```

---

## 5D: Dreams (Offline Learning)

> **Spec**: 12a-cognitive-layer.md § G
>
> **Reference**:
> - PRD: `/Users/will/dev/nunchi/roko/bardo-backup/prd/13-dreams/`
> - Research: `/Users/will/dev/nunchi/roko/bardo-backup/tmp/mori-refactor/`
>
> **What exists**: Episode data accumulates in `.roko/episodes.jsonl`. Learning modules
> in `roko-learn` track per-turn metrics. But there's no batch processing that reviews
> completed work to extract higher-level patterns. Dreams fill this gap.
>
> **Depends on**: 5A (roko-neuro for KnowledgeStore), 5C (roko-daimon for affect integration)

### Items

- [x] Create `crates/roko-dreams/Cargo.toml` ✅ (predates batch4)
- [x] Add `roko-dreams` to workspace members ✅ (codex batch4, 5D.02)
- [x] Implement `DreamCycle` — the main offline learning process: ✅ (codex batch4, 5D.03 — `DreamCycle` at roko-dreams/src/cycle.rs)
  ```rust
  pub struct DreamCycle {
      episode_store: Arc<EpisodeStore>,
      knowledge_store: Arc<KnowledgeStore>,
      playbook_store: Arc<PlaybookStore>,
      dispatcher: Arc<AgentDispatcher>,
      last_dream_at: Option<DateTime<Utc>>,
  }
  ```
- [x] Implement `DreamCycle::run()`: ✅ (codex batch4, 5D.04 — clusters by `(plan_id, task_type, model, outcome)`, distills via ClaudeCliAgent, writes `KnowledgeEntry` to KnowledgeStore, playbooks to PlaybookStore, reports to `.roko/dreams/dream-{timestamp_ms}.json`)
  ⚠️ **NOTE**: `DreamCycle::run()` dispatches a real agent for distillation. This is the ONLY code path that writes to `KnowledgeStore` during normal operation (via `ingest()`).
- [x] Wire automatic dream cycle in daemon mode: ✅ (codex batch4, 5D.05 — trigger when idle >30min, configurable via `DreamLoopConfig`: `auto_dream=true`, `idle_threshold_mins=15`, `min_episodes_for_dream=5`)
  ⚠️ **GAP**: `DreamRunner::schedule()` computes when next dream should fire, but plan-runner (orchestrate.rs) NEVER calls it. Auto-dream only works in daemon mode, not during `roko plan run`. See gap 47n.
- [x] Wire manual dream cycle: `roko dream` CLI command ✅ (codex batch4, 5D.06 — `cmd_dream()` at main.rs:2868)
- [x] Wire `roko dream --report` ✅ (codex batch4, 5D.07)
- [x] Wire dream-generated knowledge into context assembly (5B) ✅ (codex batch4, 5D.08)
- [x] Wire dream output into affect engine (5C) ✅ (codex batch4, 5D.09)

**Advanced Dream Capabilities** (from 12a §G2-G8):
- [x] **G2** Re-evaluate past episodes with current knowledge ✅ (codex batch4, 5D.10)
- [x] **G3** Mistake identification — for failed episodes, extract what specifically went wrong ✅ (codex batch4, 5D.11)
- [x] **G4** Heuristic strengthening/weakening — during replay, confirm or revise heuristic confidence scores ✅ (codex batch4, 5D.12)
- [x] **G6** Counterfactual simulation — use HDC vector permutation to explore "what if?" ✅ (codex batch4, 5D.13)
- [x] **G7** Cross-episode consolidation — discover meta-patterns across unrelated episodes ✅ (codex batch4, 5D.14)
- [x] **G8** Novel strategy generation — combine heuristics from different domains ✅ (codex batch4, 5D.15)
- [x] Implement regression detection in dream cycle ✅ (codex batch4, 5D.16)
- [x] Wire performance stall detection ✅ (codex batch4, 5D.17)
  ⚠️ **Depth pass still needed (Pass 4)**: These are shallow implementations without the PRD academic algorithms. Missing: Mattar-Daw utility-weighted replay (`utility = gain × need × (1 - 0.5×spacing_penalty)`), Pearl's SCM for counterfactual reasoning, Boden 2004 creative recombination (combinational/exploratory/transformational), Revonsuo 2000 threat simulation, Walker & van der Helm 2009 emotional depotentiation, Stickgold & Walker 2013 memory triage (preserve/abstract/forget), budget allocation by behavioral phase (thriving→terminal). G5 (threat simulation) was SKIPPED entirely.

### Verification

```bash
cargo build -p roko-dreams
cargo test -p roko-dreams

# Manual dream cycle
# First, generate some episodes by running plans
cargo run -p roko-cli -- plan run plans/test1/
cargo run -p roko-cli -- plan run plans/test2/

# Run dream
cargo run -p roko-cli -- dream
ls .roko/dreams/dream-*.json  # report generated

# Knowledge extracted
cargo run -p roko-cli -- neuro query "lessons from recent tasks"  # dream-generated entries appear

# Playbooks extracted
cargo run -p roko-cli -- dream --report | grep 'playbook'  # shows extracted playbooks

# Dream report
cat .roko/dreams/dream-*.json | python3 -m json.tool | head -20

# Auto-dream in daemon mode
# Start daemon, run some plans, wait 15 minutes idle → dream fires
cargo run -p roko-cli -- daemon start
# (run plans, then wait)
grep 'dream.*cycle.*starting' .roko/daemon.log
```

---

## 5E: Operating Frequencies (3-Speed Cognition)

> **Spec**: 12a-cognitive-layer.md § I
>
> **What exists**: Currently all agent interactions use the same pattern — dispatch an agent,
> wait for it to complete. There's no distinction between quick reactions and deep thinking.
> The cascade router selects models by cost/capability but not by speed/depth.

### Items

- [x] Define `OperatingFrequency` enum in roko-core (or roko-neuro). Named after EEG frequency bands (from 12a §I1-I3): ✅ (codex batch4, 5E.01 — Reactive/Deliberative/Extended mapped to Gamma/Theta/Delta)
  ```rust
  pub enum OperatingFrequency {
      Gamma,     // ~10s: reactive — perceive, retrieve, act. Tool calls, cache lookups, signal routing.
      Theta,     // ~2-5min: strategic — re-plan, update goals, evaluate progress. Periodic "step back".
      Delta,     // ~30min+: consolidation — dream replay, knowledge distillation, meta-cognition.
  }
  ```
  Maps to existing `bardo-primitives::tier::InferenceTier` (T0/T1/T2) and `roko-compose::context_provider::ContextTier` (Surgical/Focused/Full).
- [x] Implement frequency selection logic ✅ (codex batch4, 5E.02 — Task + AffectState → frequency)
- [x] Implement **frequency scheduler** (from 12a §I4) ✅ (codex batch4, 5E.03)
- [x] Implement **meta-cognition hook** (from 12a §I5) ✅ (codex batch4, 5E.04 — agent reflects on performance)
- [x] Wire frequency → model selection ✅ (codex batch4, 5E.05 — reactive=no model, deliberative=standard, extended=stronger)
- [x] Wire frequency → turn limits ✅ (codex batch4, 5E.06 — reactive=0, deliberative=standard, extended=more)
- [x] Wire frequency → context budget ✅ (codex batch4, 5E.07)
- [x] Wire frequency tagging in task TOML ✅ (codex batch4, 5E.08 — optional `frequency` field)
- [x] Wire frequency metrics to efficiency events ✅ (codex batch4, 5E.09 — `frequency` field on `EfficiencyEvent`)
- [x] Wire into dashboard ✅ (codex batch4, 5E.10)
  ⚠️ **Depth pass needed**: Verify frequency inference from task description works (quick_fix→Reactive, implement→Deliberative, design→Reflective). Verify periodic theta re-evaluation loop fires every 2-5 minutes during plan execution. Verify meta-cognition hook actually detects stuck agents and thrashing patterns.

### Verification

```bash
# Frequency tagging works
# Task with "quick fix" → Reactive
# Task with "implement" → Deliberative
# Task with "design architecture" → Reflective
cargo test -p roko-neuro -- frequency_inference

# Frequency affects model selection
# Reflective task always uses opus
cargo run -p roko-cli -- plan run plans/test-reflective/ 2>&1 | grep 'model.*opus'

# Frequency metrics tracked
grep '"frequency":"Deliberative"' .roko/learn/efficiency.jsonl | head -3
grep '"frequency":"Reflective"' .roko/learn/efficiency.jsonl | head -3
```

---

## 5F: C-Factor Metrics

> **Spec**: 12a-cognitive-layer.md § J
>
> **What exists**: Efficiency events track per-turn metrics (cost, tokens, duration).
> Cascade router tracks model performance. But there's no composite "capability" score
> that captures the overall cognitive performance of the system.
>
> C-Factor is inspired by psychometrics' g-factor — a single metric capturing general
> cognitive capability. For roko, it's: how effectively can the system translate plans
> into working code?

### Items

- [x] Define `CFactor` struct in roko-learn (or roko-neuro): ✅ (codex batch4, 5F.01)
  ```rust
  pub struct CFactor {
      pub overall: f64,           // 0.0-1.0 composite score
      pub components: CFactorComponents,
      pub computed_at: DateTime<Utc>,
      pub episode_count: usize,   // episodes used for calculation
  }
  pub struct CFactorComponents {
      pub gate_pass_rate: f64,    // % of tasks passing gates on first attempt
      pub cost_efficiency: f64,   // inverse of cost per successful task (normalized)
      pub speed: f64,             // inverse of time per successful task (normalized)
      pub first_try_rate: f64,    // % of tasks succeeding without re-plan
      pub knowledge_growth: f64,  // rate of new knowledge entries per episode
  }
  ```
- [x] Implement `compute_cfactor(episodes: &[Episode], window: Duration) -> CFactor`: ✅ (codex batch4, 5F.02 — ~1,300 lines, 11 sub-metrics)
  - Calculate each component from episodes within the time window (default: last 7 days)
  - `gate_pass_rate` = tasks_passed_gate / total_tasks_attempted
  - `cost_efficiency` = 1.0 / (avg_cost_per_successful_task / baseline_cost). Baseline from first 10 episodes.
  - `speed` = 1.0 / (avg_duration_per_successful_task / baseline_duration)
  - `first_try_rate` = tasks_without_replan / total_tasks
  - `knowledge_growth` = new_knowledge_entries / episode_count
  - `overall` = weighted average: gate_pass_rate * 0.3 + cost_efficiency * 0.2 + speed * 0.15 + first_try_rate * 0.25 + knowledge_growth * 0.1
- [x] Persist C-Factor time series: append to `.roko/learn/c-factor.jsonl` each time it's computed. Track trends. ✅ (codex batch4, 5F.03)
- [x] Wire C-Factor computation: compute after each plan run completes. Also compute on `roko status --cfactor` and `roko dream`. ✅ (codex batch4, 5F.04 — called from PlanRunner::finish())
- [x] Wire C-Factor into cascade router: when `cfactor.overall > 0.8` (high capability), prefer cheaper models (the system is performing well, don't need expensive models). When `cfactor.overall < 0.4` (low capability), prefer stronger models. This creates a natural cost optimization loop. ✅ (codex batch4, 5F.05)
- [x] Wire C-Factor trend into dashboard: show current C-Factor, 7-day trend (↑↓→), per-component breakdown. ✅ (codex batch4, 5F.06)
- [x] Add `roko status --cfactor` CLI command: compute current C-Factor, show components, trend. ✅ (codex batch4, 5F.07)
- [x] Wire C-Factor regression alert: if C-Factor drops >20% from 7-day average, emit `cfactor:regression` signal. Include in dream cycle analysis. ✅ (codex batch4, 5F.08)

**Collective Intelligence Sub-Metrics** (from 12a §J1-J12 — richer than the 5-component composite above):
- [x] **J1** Information flow rate — signals sent/received per unit time, processing latency. Measure signal throughput through the system. ✅ (codex batch4, 5F.09)
- [x] **J2** Turn-taking equality — Gini coefficient of agent contributions. Even participation = higher c-factor. Locally: are all agents in a plan productive, or is one doing all the work? ✅ (codex batch4, 5F.10)
- [ ] **J3** Social sensitivity proxy — response quality to other agents' outputs. How well does an agent incorporate context from upstream tasks? ⚠️ (codex batch4, 5F.11 — computation real but context-attribution.jsonl never written; always returns 0.0. See gap 47a.)
- [ ] **J4** Knowledge integration rate — how fast shared insights get confirmed. Track confirmation chains in Neuro distillation pipeline. ⚠️ (codex batch4, 5F.12 — logic real but KnowledgeConfirmationRecord data never produced by distiller. See gap 47b.)
- [x] **J5** Task diversity coverage — are agents specializing effectively? Capability utilization vs overlap. ✅ (codex batch4, 5F.13)
- [ ] **J6** Convergence velocity — time from divergent approaches to shared conclusion. Measure via knowledge agreement across agents. ⚠️ (codex batch4, 5F.14 — same data gap as J4. See gap 47c.)
- [x] **J7** Per-agent c-factor contribution score — how much does each individual agent improve collective intelligence? ✅ (codex batch4, 5F.15 — leave-one-out)
- [x] **J8** Per-fleet c-factor — computed across agents in a `roko plan run` session. Measurable today with multi-agent plan execution. ✅ (codex batch4, 5F.16)
- [x] **J9** C-factor → agent selection routing — prefer agents that fill capability gaps and improve collective c-factor. Feed into dispatch decisions. ✅ (codex batch4, 5F.17)
- [ ] **J10** C-factor metrics endpoint: `GET /api/metrics/c_factor` — composite + sub-metrics, per-agent, per-fleet. ❌ (codex batch4, 5F.18 — endpoint not created. See gap 47d.)

**Crate Extraction Public APIs** (from 12a §R1-R3):
- [x] `roko-neuro` public API — single entry point `NeuroStore`: ✅ (codex batch4, 5F.19 — full trait + KnowledgeStore JSONL impl, wired in orchestrate.rs:4282)
  ```rust
  pub trait NeuroStore {
      fn init(path: &Path) -> Result<Self>;
      fn query(&self, topic: &str, limit: usize) -> Result<Vec<KnowledgeEntry>>;
      fn ingest(&mut self, entries: Vec<KnowledgeEntry>) -> Result<()>;
      fn decay(&mut self) -> Result<usize>;  // returns entries decayed
      fn gc(&mut self, min_confidence: f64) -> Result<usize>;  // returns entries removed
  }
  ```
- [x] `roko-daimon` public API — single entry point `DaimonState`: ✅ (codex batch4, 5F.20 — PAD model with AffectEngine, 16+ tests, wired in orchestrate.rs)
  ```rust
  pub trait AffectEngine {
      fn appraise(&mut self, event: AffectEvent) -> PadVector;
      fn query(&self) -> AffectState;
      fn modulate(&self, params: &mut DispatchParams);  // adjust model, turn limit, strategy
      fn persist(&self, path: &Path) -> Result<()>;
  }
  ```
- [x] `roko-dreams` public API — single entry point `DreamRunner`: ✅ (codex batch4, 5F.21 — real consolidation cycle, wired in main.rs via build_dream_runner())
  ```rust
  pub trait DreamEngine {
      fn replay(&mut self, episodes: &[Episode]) -> Result<Vec<Insight>>;
      fn consolidate(&mut self) -> Result<DreamReport>;
      fn schedule(&self) -> Option<Duration>;  // when next dream should fire
  }
  ```

**AntiKnowledge** (from 12a §D10):
- [x] Implement `AntiKnowledge` entries — when an insight is proven wrong, create a counter-entry with kind `KnowledgeKind::AntiKnowledge`. Contains: the refuted insight ID, evidence of refutation, and a negative confidence weight. When context assembly retrieves knowledge matching a task, anti-knowledge entries appear as warnings: "Previous insight X was wrong because Y." ✅ (codex batch4, 5F.22)
- [x] AntiKnowledge reduces confidence of contradicted entries: when an AntiKnowledge entry is created, find the original entry and multiply its confidence by 0.5. ✅ (codex batch4, 5F.23 — tested, but auto-generation from gate failures not wired. See gap 47f.)

### Verification

```bash
# C-Factor computation
# After running some plans:
cargo run -p roko-cli -- status --cfactor
# Output:
#   C-Factor: 0.72 (↑ from 0.65 last week)
#   Components:
#     Gate pass rate:    0.85
#     Cost efficiency:   0.60
#     Speed:             0.75
#     First try rate:    0.80
#     Knowledge growth:  0.40

# C-Factor persists
cat .roko/learn/c-factor.jsonl | tail -3 | python3 -m json.tool

# C-Factor influences model selection
# When cfactor > 0.8, cascade router should prefer cheaper models
cargo test -p roko-learn -- cfactor_routing_high_capability

# C-Factor regression detected
# Manually corrupt some episodes to lower success rate
# Verify cfactor:regression signal emitted
grep 'cfactor:regression' .roko/signals.jsonl
```

---

# Tier 6: Chain Layer (Deferred)

> **Priority**: 🟢 P3 — Blockchain-specific features for roko-golem.
> **Depends on**: Tier 5 complete.
> **Detailed spec**: [`implementation-plans/12b-chain-layer.md`](implementation-plans/12b-chain-layer.md)
> **Detailed spec (golem)**: `implementation-plans/10-golem-integration.md` (superseded, content in 12b)
>
> **Note**: This tier is intentionally deferred. It contains 68 items across 7 sections
> covering mirage infrastructure (Q1-Q7), agent identity (A1-A7), gossip mesh (B1-B6),
> job market (C1-C9), reputation+economics+safety (K1-K8, L1-L5, M1-M5), ChainWitness
> (H1-H5), and advanced features: ISFR (N1-N3), clearing (O1-O5), privacy/TEE (P1-P4).
> Full specs with verification criteria are in `12b-chain-layer.md`.
>
> **Implementation order** (4 layers):
> - Layer 0: Mirage infra (Q) + Identity (A) + Gossip (B) + Crate arch (R4-R7)
> - Layer 1: Reputation (K) + Payments (L) — needs Layer 0
> - Layer 2: Job Market (C) + ChainWitness (H) + Safety (M) — needs Layer 0-1 + Tier 5
> - Layer 3: ISFR (N) + Clearing (O) + Privacy (P) — needs Layer 2

## 6A: Mirage Infrastructure

> **Spec**: 12b-chain-layer.md § Q
> **Depends on**: mirage-rs repo (separate), roko-golem existing scaffold
>
> **Key paths**:
> - Golem crate: `crates/roko-golem/`
> - Chain crate: `crates/roko-chain/`
> - mirage-rs: separate repo
> - Agent chain docs: `/Users/will/dev/nunchi/roko/bardo-backup/tmp/agent-chain/`
>
> **Source docs** (read before implementing any Tier 6 item):
> - `collaboration/docs/chain/korai/korai-full-spec.md` — **Korai chain spec** (10K lines): agent registration, tiers, events, RPC, state
> - `collaboration/docs/chain/daeji/daeji-chain-specification.md` — **DAEJI token spec** (3K lines): staking, escrow, fees, slashing
> - `collaboration/docs/chain/korai/korai-reputation-framework.md` — **reputation spec**: 7 domains, EMA, discipline, disputes
> - `collaboration/docs/marketplace/specs/architecture-spec.md` — **marketplace arch** (1.5K lines): jobs, matching, verification
> - `collaboration/docs/marketplace/specs/mechanism-design.md` — **mechanism design**: auctions, dispatch, clearing math
> - `collaboration/docs/gossip/gossip-architecture.md` — **gossip design**: topics, envelopes, peer scoring
> - `collaboration/docs/privacy/valhalla/valhalla-architecture.md` — **Valhalla privacy** (652 lines): TEE, privacy modes, PSI
> - `bardo-backup/prd/09-economy/` — identity, reputation, marketplace, coordination, economy PRDs
> - `bardo-backup/prd/10-safety/` — defense, policy, threat model, adaptive risk PRDs
> - `bardo-backup/prd/14-chain/` — witness, events, heartbeat, protocol state PRDs
> - `bardo-backup/tmp/agent-chain/` — 27 research files: stigmergy, HDC, tokenomics, adversarial defense
> - Full per-section source mapping in `12b-chain-layer.md` (Source Document Index table)

### Items

- [ ] **Q1** [mirage]: Implement in-process gossip mesh mock — `tokio::sync::broadcast::channel` per topic (8 topics: txs, capabilities, reputation, spore/jobs, spore/deltas, spore/status, sparrow, isfr). Fast, no libp2p. 100 concurrent subscribers, <10ms p99 delivery.
- [ ] **Q2** [mirage]: Wire auto block advancement — `mirage_stepBlock` RPC exists but needs auto-advance mode. Configurable interval (default 1s). Manual `mirage_stepBlock` still works alongside.
- [ ] **Q3** [mirage]: Implement persistent on-disk state — currently all in-memory. Knowledge, pheromones, agent registrations survive restart. State file <100MB for 1000 agents. Use `serde_json` + file-backed store.
- [ ] **Q4** [mirage]: Implement multi-agent simulation mode — register N agents with configurable profiles, run scenario scripts (post jobs, bid, clear, rate), produce c-factor report. Used for testing collective behaviors.
- [ ] **Q5** [mirage]: Wire event replay / time-travel debugging — full event log written to disk. Replay from any block number reproduces identical state. Snapshot/revert exists but needs event log integration.
- [ ] **Q6** [mirage]: Wire aggregated metrics endpoints — `GET /metrics` returns: `total_agents`, `active_jobs`, `clearing_volume`, `avg_reputation`, `network_c_factor`. Updated every block. HTTP API exists but needs metrics rollups.
- [ ] **Q7** [mirage]: Implement MCP server exposing chain operations — tools: `korai/knowledge/query`, `korai/marketplace/tasks`, `korai/agent/register`, `korai/reputation/query`. Agents interact via standard MCP protocol over stdio JSON-RPC.
- [ ] Wire deployment configs: mirage endpoint URL, chain ID, gas settings in roko.toml `[chain]` section
- [ ] Wire health check + connection retry logic between roko-golem and mirage-rs HTTP gateway

### Verification

```bash
# Q1: Gossip mock — 100 subscribers, message delivery <10ms
cargo test -p mirage-rs -- gossip_broadcast_latency

# Q2: Auto-advance blocks
curl -s http://localhost:8545 -d '{"method":"mirage_setAutoAdvance","params":[true, 1000]}' | jq .

# Q3: Restart persistence
# Register agent, restart mirage, verify agent still registered

# Q4: Multi-agent simulation
cargo run -p mirage-rs -- simulate --agents 20 --scenario jobs

# Q7: MCP server lists tools
echo '{"jsonrpc":"2.0","method":"tools/list","id":1}' | cargo run -p mirage-rs -- mcp 2>/dev/null | jq .
```

---

## 6B: Agent Identity + Passport

> **Spec**: 12b-chain-layer.md § A

### Items

- [ ] **A1** [golem]: Define `AgentPassport` struct — `address: Address`, `owner: Address`, `system_prompt_hash: [u8; 32]`, `stake: U256`, `tier: AgentTier` (Probation/Active/Elite/Master), `capabilities: u64` (bitmask: Trading=1, Security=2, Data=4, Knowledge=8, Strategy=16, Analytics=32). mirage has `AgentEntry`; needs full passport fields.
- [ ] **A2** [mirage]: Implement `chain_registerPassport` RPC — extends existing `chain_registerAgent` with stake, tier, capabilities. Passport queryable after registration.
- [ ] **A3** [roko]: Implement system prompt hash verification (Ventriloquist defense) — SHA-256 hash of system prompt at build time, stored on-chain at registration. Mismatch rejects the registration. Prevents prompt injection at the identity layer.
- [ ] **A4** [mirage]: Implement tier progression logic — thresholds: Probation→Active (10 jobs, reputation>0.5), Active→Elite (50 jobs, reputation>0.7), Elite→Master (200 jobs, reputation>0.9). Based on `jobs_completed + reputation_score`.
- [ ] **A5** [both]: Implement capability bitmask declaration and query — agent declares at registration. `chain_queryAgentsByCapability(bitmask)` returns matching agents. 6 capability domains.
- [ ] **A6** [golem]: Implement Ed25519 wallet/signing — keypair generated on first run, stored at `.roko/identity/key.json`. Used for signing gossip envelopes, transactions, attestations. Use `ed25519-dalek` or `ring` crate.
- [ ] **A7** [roko]: Implement local agent identity (non-chain) — `roko init` creates `.roko/identity.json` with UUID + optional display name. Agent has an ID even without chain. Persists across restarts.

### Verification

```bash
# A1: Passport round-trips through serde
cargo test -p roko-golem -- passport_serde_roundtrip

# A2: Registration against mirage mock
cargo test -p roko-golem -- register_passport_mock

# A3: Prompt hash mismatch rejects registration
cargo test -p roko-golem -- ventriloquist_defense

# A4: Tier auto-promotion after threshold
cargo test -p mirage-rs -- tier_progression

# A6: Ed25519 keypair generation + verification
cargo test -p roko-golem -- ed25519_sign_verify

# A7: Local identity persists
cargo run -p roko-cli -- init
cat .roko/identity.json | jq .id  # UUID present
```

---

## 6C: Gossip Mesh

> **Spec**: 12b-chain-layer.md § B
>
> **Architecture**: In-process mock mesh using tokio broadcast channels (NOT libp2p).
> mirage provides the mesh mock; golem agents subscribe to topics.

### Items

- [ ] **B1** [both]: Define `GossipEnvelope` message format — `version: u8`, `topic: String`, `sender: Address`, `timestamp: DateTime<Utc>`, `payload: Vec<u8>`, `signature: [u8; 64]` (Ed25519). Serializes to <64KB. Rejected if any field missing.
- [ ] **B2** [mirage]: Implement 8 topic subscriptions — `tokio::sync::broadcast::channel` per topic: `txs`, `capabilities`, `reputation`, `spore/jobs`, `spore/deltas`, `spore/status`, `sparrow`, `isfr`. Subscribing returns a `Receiver<GossipEnvelope>`.
- [ ] **B3** [golem]: Implement message signing and validation — sign outgoing envelopes with agent's Ed25519 key (A6). Verify incoming: reject unsigned or tampered envelopes.
- [ ] **B4** [golem]: Implement heartbeat publishing — every 30-60s, publish heartbeat on `capabilities` topic. `chain_agentHeartbeat` RPC exists but needs gossip envelope wrapping. mirage marks agent as offline after 3 missed heartbeats (90s).
- [ ] **B5** [mirage]: Implement 3-layer peer scoring — composite score: `behavioral * 0.4 + economic * 0.4 + tee * 0.2`. Score in [0, 1]. Behavioral = uptime + response rate. Economic = job completion + no slashing. TEE = attestation validity.
- [ ] **B6** [mirage]: Wire gossip → EventBus bridge — gossip-received signals enter the same dispatch loop as local signals. Topic `spore/jobs` → `signal_kind = "chain:job:posted"`, etc.

### Verification

```bash
# B1: Envelope serialization <64KB
cargo test -p roko-golem -- envelope_size_limit

# B2: 8 topics independent, 100 subscribers
cargo test -p mirage-rs -- topic_isolation_100_subscribers

# B3: Tampered envelope rejected
cargo test -p roko-golem -- reject_tampered_envelope

# B4: Heartbeat → offline after 90s
cargo test -p mirage-rs -- heartbeat_offline_detection

# B5: Peer score = 0.4*behavioral + 0.4*economic + 0.2*tee
cargo test -p mirage-rs -- peer_score_formula
```

---

## 6D: Job Market

> **Spec**: 12b-chain-layer.md § C
>
> **Protocols**: Spore (job posting/discovery), Sparrow (bidding/dispatch).

### Items

- [ ] **C1** [mirage]: Implement `SporeJob` posting and discovery — job spec: `reward: U256`, `deadline: DateTime<Utc>`, `quality_threshold: f64`, `required_capabilities: u64` (bitmask), `description: String`. Posted via RPC, discoverable by agents with matching capabilities within 1 block.
- [ ] **C2** [mirage]: Implement `BountySpec` with 3 hiring models — `FixedPrice` (set reward), `ReverseAuction` (lowest qualified bid wins), `DirectHire` (assign to specific agent). Full job definition struct.
- [ ] **C3** [golem]: Implement `SparrowBid` submission — `price: U256`, `eta_secs: u64`, `confidence: f64` (0-1). Rejected if agent lacks required capabilities from `SporeJob`.
- [ ] **C4** [mirage]: Implement power-of-two-choices dispatch — probe 2 random capable agents, assign to least loaded. Over 1000 dispatches, load variance across agents <20%.
- [ ] **C5** [both]: Implement job lifecycle state machine — `Created → Claimed → Running → Completed | Failed`. Timeout fallbacks at each state (e.g., Claimed → unclaim after 5min idle). Enforce: cannot skip states.
- [ ] **C6** [both]: Implement `JobReceipt` with proof-of-execution — `output_hash: [u8; 32]` (SHA-256 of output), `execution_ms: u64`, `gate_results: Vec<GateResult>`. Agent submits; chain verifies output_hash.
- [ ] **C7** [mirage]: Implement escrow and settlement — mock DAEJI token. Budget lock on job claim → release to agent on Completed → slash on Failed → refund to poster on timeout.
- [ ] **C8** [mirage]: Implement 6 mining types — `Genome` (code generation), `Verifier` (validation), `Repair` (bug fixing), `Mechanism` (protocol design), `Index` (code indexing), `Memory` (knowledge consolidation). Each type has distinct validation rules.
- [ ] **C9** [golem]: Implement `DeltaArtifact` submission — before/after metric snapshots + artifact hash. Mining solution format. mirage verifies hash matches stored artifact.

### Verification

```bash
# C1: Job posted, discoverable within 1 block
cargo test -p mirage-rs -- spore_job_discovery

# C2: Reverse auction selects lowest qualified bid
cargo test -p mirage-rs -- reverse_auction_selection

# C4: Power-of-two-choices load balancing
cargo test -p mirage-rs -- po2c_load_variance  # variance <20%

# C5: State machine enforced — no state skipping
cargo test -p mirage-rs -- job_state_machine_enforcement

# C7: Escrow lock/release/slash/refund all paths
cargo test -p mirage-rs -- escrow_lifecycle
```

---

## 6E: Reputation + Economics

> **Spec**: 12b-chain-layer.md § K (Reputation), § L (Payments), § M (Safety)

### Items

**Reputation (§K)**:
- [ ] **K1** [mirage]: Implement 7 domain reputation tracks — Trading, Predictions, Data, Security, Knowledge, Strategy, Analytics. Per-domain EMA scores, independently scored.
- [ ] **K2** [mirage]: Implement EMA scoring with 30-day half-life — `score_new = α * outcome + (1-α) * score_old`. After 10 successes, score converges >0.9. Decays measurably after 30 days of inactivity.
- [ ] **K3** [mirage]: Implement tier-based trust multipliers — Probation=0.5×, Active=1×, Elite=1.5×, Master=2×. Affects job eligibility and commission rates.
- [ ] **K4** [mirage]: Implement discipline protocol — escalation ladder: 3 penalty points → warning, 6 → probation, 10 → suspension. Penalty points decay over time.
- [ ] **K5** [mirage]: Implement slashing schedule — 6 offense types: abandoned job (2% stake), failed safety check (10%), dishonest reporting (15%), collusion (25%), data breach (50%), TEE violation (100%).
- [ ] **K6** [roko]: Implement local reputation tracking — agent polls own reputation from mirage, stores in `.roko/reputation.json`. Feeds Daimon appraisals on score changes.
- [ ] **K7** [mirage]: Wire reputation → gossip peer scoring bridge — reputation >0.8 adds +0.2 to peer score; <0.3 subtracts -0.3.
- [ ] **K8** [mirage]: Implement dispute resolution state machine — `Filed → Panel (3 random agents) → Commit-Reveal Vote → Majority Wins → Appeal Window (24h) → Finalize`. All transitions tested.

**Payments (§L)**:
- [ ] **L1** [mirage]: Implement mock DAEJI token — in-memory balance tracking. `chain_getBalance(agent)` returns correct balance after mint/transfer/slash.
- [ ] **L2** [mirage]: Wire job escrow lifecycle — lock reduces poster balance, release increases agent balance, slash reduces agent stake, refund restores poster. All paths tested.
- [ ] **L3** [mirage]: Implement fee structure — 0.5% posting fee, 5% validation fee, 2% protocol fee to treasury. Correctly computed for reward amounts 0.01–10000.
- [ ] **L4** [mirage]: Implement X402 micropayment protocol — HTTP 402 response with wallet-sig for knowledge API access. Valid signature grants access + deducts micropayment. Invalid returns 402.
- [ ] **L5** [roko]: Wire agent balance tracking + cost reporting — tracks cumulative cost in `.roko/learn/costs.jsonl`. Dashboard shows `avg_cost_per_episode_cents`.

**Chain Safety (§M)**:
- [ ] **M1** [mirage]: Implement watcher agent type — registered with `is_watcher: true`. Performs 5 check types: policy, behavioral, solvency, attestation, correlation. Configurable check interval.
- [ ] **M2** [mirage]: Implement escalation ladder — single watcher → Advisory (logged only). 2 independent watchers confirm → Throttle → Freeze → Slash depending on severity.
- [ ] **M3** [mirage]: Implement bounded safe actions — auto-triggered on threshold breach. E.g., solvency ratio <0.5 → auto-widen spreads. Actions bounded: cannot transfer funds.
- [ ] **M4** [mirage]: Implement `GuardianFreeze` — locks all agent actions. Only `governance_unfreeze` can restore. Frozen agent cannot submit bids, jobs, or gossip.
- [ ] **M5** [both]: Implement `PolicyManifest` per agent — declared at registration. Fields: `position_limits`, `allowed_assets`, `max_drawdown_pct`, `max_leverage`. Watcher validates every action against manifest.

### Verification

```bash
# K2: EMA convergence after 10 successes
cargo test -p mirage-rs -- ema_convergence

# K4: Discipline escalation ladder
cargo test -p mirage-rs -- discipline_ladder

# K5: All 6 slashing offense types
cargo test -p mirage-rs -- slashing_schedule

# K8: Dispute resolution full flow
cargo test -p mirage-rs -- dispute_full_flow

# L2: Escrow all paths (lock/release/slash/refund)
cargo test -p mirage-rs -- escrow_all_paths

# L3: Fee computation
cargo test -p mirage-rs -- fee_structure

# M2: Escalation: 2 watchers confirm → freeze
cargo test -p mirage-rs -- watcher_escalation

# M4: GuardianFreeze locks agent
cargo test -p mirage-rs -- guardian_freeze
```

---

## 6F: ChainWitness

> **Spec**: 12b-chain-layer.md § H
>
> Golem-only. Watches on-chain events and feeds them into the cognitive stack.
> Existing code: `crates/roko-golem/src/chain_witness.rs` (scaffold)

### Items

- [ ] **H1** [golem]: Wire `ChainWitnessEngine::subscribe()` — subscribes to relevant on-chain events via mirage RPC or gossip mesh. Reconnects on disconnect. Receives events within 1 block latency.
- [ ] **H2** [golem]: Implement event → Signal conversion — each chain event type (`tx`, `block`, `contract_call`) produces a valid `Signal` with correct `signal_type` and `body`. Bridge chain world into roko signal graph.
- [ ] **H3** [golem]: Wire ChainWitness → Daimon feed — profitable tx → Daimon appraisal with `pleasure > 0`; loss tx → `pleasure < 0, arousal > 0`. Feeds affect engine (5C).
- [ ] **H4** [golem]: Wire ChainWitness → Neuro feed — after witnessing 5+ similar market patterns, Neuro distillation produces an Insight entry. Witnessed patterns become knowledge entries.
- [ ] **H5** [golem]: Implement configurable event filters — `roko.toml` under `[golem.chain_witness]` specifies contract addresses and event types to watch. Unmatched events silently dropped.

### Verification

```bash
# H1: Subscribe receives events within 1 block
cargo test -p roko-golem -- chain_witness_subscribe

# H2: Event → Signal conversion
cargo test -p roko-golem -- chain_event_to_signal

# H3: Profit tx → positive Daimon appraisal
cargo test -p roko-golem -- chain_witness_daimon_feed

# H4: 5 similar patterns → knowledge entry
cargo test -p roko-golem -- chain_witness_neuro_distillation

# H5: Filter config — only matching events received
cargo test -p roko-golem -- chain_witness_filter
```

---

## 6G: Advanced (ISFR, Clearing, Privacy)

> **Spec**: 12b-chain-layer.md § N (ISFR), § O (Clearing), § P (Privacy)
>
> These are Layer 3 (Advanced Collective) — require all of 6A-6F.

### Items

**ISFR — Collective Price Discovery (§N)**:
- [ ] **N1** [golem]: Implement `IsfrSubmission` — agent submits `market_id`, `rate`, `confidence`, `agent_address`, `signature` after clearing round. Published on `korai/isfr` gossip topic.
- [ ] **N2** [mirage]: Implement `IsfrAggregate` — with <3 submissions, no aggregate produced. With 3+, compute median rate (outlier rejection: >3σ excluded). Broadcast result.
- [ ] **N3** [golem]: Wire agent ISFR consumption — receive `IsfrAggregate`, update local pricing model. Context assembly includes latest ISFR rates for relevant markets.

**Cooperative Clearing (§O)**:
- [ ] **O1** [mirage]: Implement clearing engine — QP (Quadratic Programming) solver: minimize inventory cost subject to constraints. Off-chain solve, on-chain verify.
- [ ] **O2** [mirage]: Implement soft-threshold analytical solution — bisection for λ* in O(80n) iterations. From collaboration spec. Result matches brute-force QP within PU18 precision.
- [ ] **O3** [mirage]: Implement `ClearingCertificate` — contains KKT optimality conditions. On-chain verification in O(n) confirms optimality. Invalid certificates rejected.
- [ ] **O4** [golem]: Implement sealed commitment — agent commits `hash(γ, c, I_min, I_max, nonce)` in round 1, reveals in round 2. Early reveal detected and penalized.
- [ ] **O5** [mirage]: Implement fallback ladder — QP fails → pruned solve (remove smallest positions) → external hedge (route to external venue) → safe mode (freeze + notify). Each fallback independently tested.

**Privacy & TEE (§P)**:
- [ ] **P1** [both]: Implement 4 privacy modes — `PUBLIC`, `OPERATOR_PRIVATE`, `HYBRID_CONFIDENTIAL`, `FULL_CONFIDENTIAL`. Per-knowledge-entry privacy level. `FULL_CONFIDENTIAL` encrypted at rest, only decryptable by owner.
- [ ] **P2** [mirage]: Implement TEE attestation stub — mock for dev (generates valid attestation document), real format for prod (AWS Nitro / Intel TDX). Verification function accepts mock and rejects tampered.
- [ ] **P3** [golem]: Implement Private Set Intersection — position matching without revealing positions. X25519 DH + HMAC-SHA256 protocol. Completes in 2 rounds.
- [ ] **P4** [golem]: Implement zero-knowledge range proofs — prove collateral > threshold without revealing actual amount. Bulletproofs over Ristretto255. Verifier accepts valid proofs, rejects invalid.

**Crate Architecture (§R4-R7)**:
- [ ] **R4** [golem]: Keep roko-golem as thin blockchain-variant assembly — imports roko-neuro + roko-daimon + roko-dreams + chain_witness. `lib.rs` re-exports their public APIs. No duplicated logic.
- [ ] **R7** [golem]: Remove death concepts — delete `crates/roko-golem/src/mortality.rs` and `crates/roko-golem/src/hypnagogia.rs`. No references to death concepts remain.

### Verification

```bash
# N2: ISFR aggregate with outlier rejection
cargo test -p mirage-rs -- isfr_aggregate_outlier_rejection

# O2: Bisection converges in 80 iterations
cargo test -p mirage-rs -- clearing_bisection_convergence

# O3: Invalid ClearingCertificate rejected
cargo test -p mirage-rs -- clearing_certificate_validation

# O5: Fallback ladder — each fallback independently
cargo test -p mirage-rs -- clearing_fallback_ladder

# P1: Privacy modes enforced in queries
cargo test -p roko-golem -- privacy_modes_enforcement

# P3: PSI completes in 2 rounds, no position leakage
cargo test -p roko-golem -- psi_no_leakage

# P4: ZK range proof valid/invalid
cargo test -p roko-golem -- zk_range_proof

# R7: Death concepts removed
test ! -f crates/roko-golem/src/mortality.rs
test ! -f crates/roko-golem/src/hypnagogia.rs
cargo build -p roko-golem  # compiles without them
```

---

# Cross-Cutting: PRD-Driven Autonomous Development Workflow

> **Source**: phase-7-8.md §8 (the capstone workflow)
> **Priority**: 🔴 P0 — This is the ultimate goal of the entire system.
> **Depends on**: Tiers 2-4 (webhook ingestion, templates, subscriptions, daemon, cloud deploy)
> **Detailed spec**: [`implementation-plans/11-sections/phase-7-8.md` §8](implementation-plans/11-sections/phase-7-8.md)
>
> The full cybernetic loop: humans write PRDs → agents autonomously implement them →
> gates validate → humans review → feedback loops improve the next cycle.

## End-to-End Signal Flow

```
webhook.github.push (canonical PRD in collaboration repo)
    │
    ▼
prd-ingestion-agent ──► prd.published signal
    │
    ▼
auto-plan-agent ──► prd.plan_generated (plan PR created)
    │
    ▼ (human reviews + merges plan PR)
prd.plan_approved ──► code-implementer-agent (cloud)
    │
    ▼
agent.completed (implementation PR created)
    │
    ▼ (human reviews PR)
webhook.github.pull_request_review ──► review-response-agent
    │
    ▼ (human merges)
feedback.github.pr_engagement (collected post-merge)
    │
    ▼
Learning ──► CascadeRouter, Experiments, Dreams ──► next cycle improves
```

## Items

**Additional Templates for the Workflow** (beyond the 16 in §3A):
- [ ] Create `prd-ingestion-agent.toml` — Trigger: `github:push` on collaboration repo (path: `docs/**/prd-*.md`, status: canonical). Reads canonical PRD, creates `.roko/prd/{slug}.md` in target repo, creates PR `prd/{slug}`, emits `prd.published` signal. Model: haiku. max_turns: 5. Role: operator.
- [ ] Create `auto-plan-agent.toml` — Trigger: `prd.published` signal. Reads PRD, generates tasks.toml, creates PR `plan/{slug}`, posts to Slack `#roko-plans`, emits `prd.plan_generated`. Model: sonnet. max_turns: 15.
- [ ] Create `review-response-agent.toml` — Trigger: `github:pull_request_review` on branches matching `impl/*`. Reads review comments, makes requested changes, pushes new commits (no force-push), replies to each comment with explanation + commit SHA. Re-runs gates after changes. Model: sonnet. max_turns: 15. Role: implementer.
- [ ] Create `gate-fixer-agent.toml` — Trigger: internal (called by code-implementer on gate failure). Reads gate error output, makes targeted fix, re-runs failed gate. Model: sonnet. max_turns: 10. max 3 attempts before escalating.

**PM System Integration** (knowledge-base sync):
- [ ] Wire PRD ingestion → PM task creation: when PRD is ingested, create PM task "Review PRD: {title}" in `pm/tasks/` via `pm_sync` script.
- [ ] Wire plan generation → PM workstream creation: when plan is generated, create PM workstream for the plan with one PM task per plan task.
- [ ] Wire execution progress → PM task status updates: `in-progress` when task starts, gate results attached, `done` when PR merged.
- [ ] Wire PM health agent additions: include autonomous work metrics — plans in progress, plans completed this week, avg time from PRD → merged implementation, gate failure rate, review cycle count.

**Safety Configuration for Autonomous Workflow**:
- [ ] Add `[safety]` section to `roko.toml`:
  ```toml
  [safety]
  level = "normal"           # strict|normal|autonomous
  max_cost_per_plan = 5000   # cents ($50)
  max_cost_per_day = 20000   # cents ($200)
  consecutive_failure_limit = 5  # circuit breaker threshold
  require_approval = ["github.merge_pr"]  # actions needing human approval
  ```
- [ ] Wire safety levels: `strict` = human approval for every PR. `normal` = human approval for code PRs, auto-merge for docs. `autonomous` = auto-merge if all gates pass (dangerous, opt-in only).
- [ ] Wire escape hatches: `roko daemon stop` (kill all agents), `roko agent stop <id>` (kill specific), circuit breaker auto-triggers, Slack alerts on any failure.

**GitHub App Setup** (for cloud orchestrator):
- [ ] Document required GitHub App permissions: `contents:write` (push), `pull_requests:write` (create/update PRs), `issues:write` (create issues, labels), `checks:read` (CI status), `metadata:read`.

## End-to-End Verification

```bash
# 1. Start daemon
roko daemon start --port 9090
roko daemon status  # state=running, subscriptions=21+, agents=0

# 2. Create a test PRD in collaboration repo
cd /Users/will/dev/nunchi/collaboration
cat > docs/roadmap/prd-test-feature.md << 'EOF'
---
title: "Test Feature PRD"
status: canonical
domain: roadmap
owner: wp
tags: [prd]
target_repo: roko
target_crates: [roko-core]
---
# Test Feature
Add a `pub fn hello() -> &'static str { "world" }` to roko-core/src/lib.rs.
EOF
git add . && git commit -m "add test PRD" && git push origin main

# 3. Wait 30s — prd-ingestion-agent should fire
# Verify: .roko/prd/test-feature.md exists in roko repo, PR created
grep 'prd-ingestion-agent' /Users/will/dev/nunchi/roko/roko/.roko/episodes.jsonl

# 4. Merge the PRD PR in roko repo

# 5. Wait for auto-plan-agent — creates plan PR: plan/test-feature
gh pr list --repo nunchi/roko | grep 'plan/test-feature'

# 6. Merge the plan PR

# 7. Wait for code-implementer-agent — clones, implements, pushes, creates impl PR
gh pr list --repo nunchi/roko | grep 'impl/test-feature'

# 8. Submit a review comment — review-response-agent handles it
# 9. Approve and merge the impl PR

# 10. Verify learning updated
cat .roko/learn/cascade-router.json | python3 -m json.tool
grep 'code-implementer' .roko/episodes.jsonl | tail -1 | python3 -m json.tool

# 11. Verify full cycle in metrics
curl -s http://localhost:9090/api/metrics/summary | python3 -m json.tool
```

---

# Reference Material Index

## Collaboration Repo (live specs — definitive for Tier 6)

| What | Path | Use For |
|------|------|---------|
| **Korai full spec** (10K lines) | `collaboration/docs/chain/korai/korai-full-spec.md` | Chain architecture, agent registration, tiers, events, RPC |
| **Korai reputation framework** (332 lines) | `collaboration/docs/chain/korai/korai-reputation-framework.md` | EMA formula, 7 domains, tiers, discipline, disputes |
| **DAEJI chain spec** (3K lines) | `collaboration/docs/chain/daeji/daeji-chain-specification.md` | Token, staking, escrow, kernel precompiles, block phases |
| **Marketplace architecture** (1.5K lines) | `collaboration/docs/marketplace/specs/architecture-spec.md` | Spore/Sparrow, job lifecycle, passport tiers, ventriloquist defense |
| **Mechanism design** (1K lines) | `collaboration/docs/marketplace/specs/mechanism-design.md` | Auctions (FPSB/Vickrey/Dutch), ISFR, clearing QP, slash rates, consortium |
| **On-chain/off-chain protocol** (900 lines) | `collaboration/docs/marketplace/specs/onchain-offchain-protocol.md` | GossipEnvelope, BountySpec, JobReceipt, Sparrow dispatch |
| **Output materialization** (900 lines) | `collaboration/docs/marketplace/specs/output-materialization.md` | CompletionProof, blob DAG, quality scoring, payment tiers |
| **Gossip architecture** (632 lines) | `collaboration/docs/gossip/gossip-architecture.md` | 4-tier comms, 8 topics, GossipSub config, FABRIC, MiroFish |
| **Valhalla privacy** (652 lines) | `collaboration/docs/privacy/valhalla/valhalla-architecture.md` | 4 privacy tiers, Tier 2.5 confidential preprocessing, TEE lifecycle |
| **Collaboration scripts** (18 Python scripts) | `collaboration/scripts/` | validate-frontmatter, gen-digest, extract-actions, detect-conflicts, etc. |
| **Knowledge-base scripts** (20 Python scripts) | `knowledge-base/scripts/` | pm-sync, pm-validate, pm-enrich, roko-status, etc. |

## Agent-Chain Research (theoretical foundations)

| What | Path | Use For |
|------|------|---------|
| **Academic bibliography** (50+ papers) | `bardo-backup/tmp/agent-chain/08-references.md` | Full citations with links and relevance notes |
| **15 research traditions** | `bardo-backup/tmp/agent-chain/14-academic-foundations.md` | How bacterial comms → gauge theory → sandpile physics converge on one architecture |
| **HDC from first principles** | `bardo-backup/tmp/agent-chain/04-hdc.md` | 10,240-bit BSC math, binding/bundling/permutation, capacity bounds |
| **Stigmergy theory** | `bardo-backup/tmp/agent-chain/03-stigmergy.md` | Indirect coordination, pheromone dynamics, Viable System Model mapping |
| **Dynamic context assembly** | `bardo-backup/tmp/agent-chain/15-dynamic-context-assembly.md` | From stigmergy to perfect prompts |
| **Knowledge layer** | `bardo-backup/tmp/agent-chain/05-knowledge-layer.md` | 6 knowledge types, context pack composition |
| **Tokenomics** | `bardo-backup/tmp/agent-chain/06-tokenomics.md` | GNOS demurrage, mining types, economic flywheels |
| **Exponential flywheels** | `bardo-backup/tmp/agent-chain/09-exponential-flywheels.md` | 10 compounding mechanisms |
| **Predictive foraging** | `bardo-backup/tmp/agent-chain/10-predictive-foraging.md` | Self-improving knowledge via falsifiable prediction |
| **Adversarial defense** | `bardo-backup/tmp/agent-chain/11-adversarial-defense-and-value.md` | Gaming resistance, value accrual |
| **Autonomous eval generation** | `bardo-backup/tmp/agent-chain/17-autonomous-eval-generation.md` | EVM as deterministic oracle, self-improvement loop |
| **Mirage-RS PoC** | `bardo-backup/tmp/agent-chain/16-mirage-rs-poc.md` | Simulation layer design |
| **Harness engineering** | `bardo-backup/tmp/agent-chain/harness-engineering.md` | Why scaffold > model (Meta-Harness 6× gap) |
| **Collective intelligence proof** | `bardo-backup/tmp/agent-chain/proving-collective-intelligence.md` | Correlation vs causation in knowledge sharing |

## Key Academic Papers (most-cited across the design)

| Paper | Where Cited | Key Insight |
|-------|------------|------------|
| [Kanerva 2009] Hyperdimensional Computing, Cognitive Computation 1(2) | 5A (HDC) | 10K-dim binary vectors, sub-μs similarity via Hamming |
| [Liu 2023] Lost in the Middle, TACL (arXiv:2307.03172) | 5B (Context) | U-shaped attention: high-value at start+end of prompt |
| [Mehrabian 1996] PAD Model, Current Psychology 14(4) | 5C (Daimon) | Pleasure-Arousal-Dominance emotional state space |
| [Park 2023] Generative Agents, UIST (arXiv:2304.03442) | 5A, 5D (Dreams) | Memory + reflection + planning → believable agents |
| [Sumers 2023] CoALA, arXiv:2309.02427 | 5E (Frequencies) | 9-step cognitive pipeline → Gamma/Theta/Delta |
| [Lewis 2020] RAG, NeurIPS (arXiv:2005.11401) | 5B (Context) | Retrieval-augmented generation foundation |
| [Grassé 1959] Stigmergy, Insectes Sociaux 6(1) | 6C (Gossip) | Indirect coordination via environment modification |
| [Dorigo 1997] Ant Colony, IEEE Trans. Evol. Comp. 1(1) | 6C (Gossip) | Pheromone reinforcement/evaporation → knowledge decay |
| [Beer 1972] Brain of the Firm, Allen Lane | Tier 6 (VSM) | Viable System Model: 5 nested subsystems |
| [Woolley 2010] c-factor, Science 330(6004) | 5F (C-Factor) | Group intelligence: social sensitivity > max individual ability |
| [Gesell 1916] Freigeld | 6E (Payments) | Demurrage: holding tax forces circulation, prevents hoarding |

## Original Reference Material (roko/mori codebase)

| What | Path | Use For |
|------|------|---------|
| Mori orchestrator (108K LOC) | `/Users/will/dev/uniswap/bardo/apps/mori/` | Reference implementation |
| Mori agent spawn | `/Users/will/dev/uniswap/bardo/apps/mori/src/agent/connection.rs:2444-2620` | Agent lifecycle |
| 36 original crates | `/Users/will/dev/uniswap/bardo/crates/` | Trait definitions, patterns |
| 171 mori plans | `/Users/will/dev/uniswap/bardo/.mori/plans/` | Plan/task format examples |
| 359 PRD documents | `/Users/will/dev/nunchi/roko/bardo-backup/prd/` | Feature specifications |
| 1,253-item parity checklist | `/Users/will/dev/nunchi/roko/bardo-backup/tmp/roko-progress/MORI-PARITY-CHECKLIST.md` | Completeness tracking |
| 30+ catalogued mistakes | `/Users/will/dev/nunchi/roko/bardo-backup/tmp/roko-progress/MISTAKES-LEARNED.md` | What NOT to do |
| 140+ component specs | `/Users/will/dev/nunchi/roko/bardo-backup/tmp/roko-progress/COMPONENTS/` | Per-component details |
| Agent architecture docs | `/Users/will/dev/nunchi/roko/bardo-backup/tmp/mori-agents/` | Backend design |
| Research docs | `/Users/will/dev/nunchi/roko/bardo-backup/tmp/mori-refactor/` | Design patterns |
| Task generation design | `tmp/DESIGN-TASK-GENERATION.md` | Task decomposition philosophy |
| Effectiveness comparison | `tmp/v2163-effectiveness.md` | Agent harness insights |

---

# Superseded Documents

These files are replaced by this master plan. They remain for historical reference only.

| File | Replaced By |
|------|-------------|
| `tmp/MASTER-REMAINING-WORK.md` | This file (all sections A-K absorbed) |
| `tmp/PROMPT-EXECUTOR-PARITY.md` | This file (all sections 1-11 absorbed) |
| `tmp/implementation-plans/07-mcp-tool-wiring.md` | Tier 1C + Tier 2D |
| `tmp/implementation-plans/08-observability-wiring.md` | Tier 1D |
| `tmp/implementation-plans/09-tui-dashboard.md` | Tier 1H |
| `tmp/implementation-plans/10-golem-integration.md` | Tier 6 (12b-chain-layer.md) |
| `tmp/implementation-plans/12-nunchi-integration.md` | 12a-cognitive-layer.md + 12b-chain-layer.md |
