# Plan Execution: Graph Engine is a No-Op

## Symptom

```
$ roko plan run plans/ --approval --fresh
▸ Running plan via Graph Engine (8 tasks): hdc-core-crate
  Plan 'hdc-core-crate' completed: 8 nodes, 8 output signals, SUCCESS
```

8 tasks complete in ~2 seconds. No agents spawned. No code changed. "SUCCESS."

## Root Cause

### TaskExecutorCell is a stub

`crates/roko-graph/src/cells/task_executor.rs`:

```rust
impl Cell for TaskExecutorCell {
    async fn execute(&self, input: Vec<Engram>, _ctx: &CellContext) -> Result<Vec<Engram>> {
        if self.dry_run {
            // Returns: "task-output:dry-run:<label>"
        } else {
            tracing::warn!("TaskExecutorCell live dispatch not yet implemented; using dry-run fallback");
            // Returns: "task-output:stub:<label>"
        }
    }
}
```

- `TaskExecutorCell::default()` sets `dry_run = true`
- Even when `dry_run = false`, falls back to stub output with a warning
- **No agent is ever spawned. No code is ever generated. No gate is ever run.**

### Graph Engine is the default, Runner v2 is feature-gated

`crates/roko-cli/src/commands/plan.rs:264-298`:

```rust
if matches!(engine, PlanEngine::Graph) {
    return cmd_plan_run_engine(...).await;  // ← DEFAULT, goes here
}
// Runner v2 only if `legacy-runner-v2` feature enabled
```

The working runner (v2) is behind a Cargo feature flag. The default path (Graph Engine)
has no real agent dispatch.

### The "8 output signals" are synthetic

Each task produces one `Engram` with text like `"task-output:stub:Build parser"`.
These are not real agent outputs. They're hardcoded strings.

## What Runner v2 Does (the working path)

Runner v2 (`crates/roko-cli/src/runner/`) has real agent dispatch:

| Component | File | What it does |
|-----------|------|-------------|
| Event loop | `runner/event_loop.rs` | Maintains ParallelExecutor, drives task lifecycle |
| Agent spawn | `runner/agent_stream.rs` | Spawns `claude <prompt> --model <model> --max-turns 15` |
| Agent events | `runner/agent_events.rs` | Receives real LLM responses, tool calls |
| Gate dispatch | `runner/gate_dispatch.rs` | Runs `cargo check`, `cargo test`, `cargo clippy` |
| Persistence | `runner/persist.rs` | Writes `.roko/episodes.jsonl`, task results |
| Streaming | `runner/streaming.rs` | Real-time output to TUI/SSE |

Runner v2 is ~2,400 lines with full agent lifecycle management.

### How Runner v2 spawns agents

```rust
// runner/agent_stream.rs
pub async fn spawn_agent(
    config: &AgentSpawnConfig,
    event_tx: mpsc::Sender<AgentEvent>,
) -> Result<AgentHandle> {
    // Spawns: `claude <prompt> --model <model> --max-turns 15 ...`
    // Parses stream-json output line-by-line
    // Sends AgentEvent::MessageDelta, TokenUsage through channel
}
```

### How Runner v2 validates with gates

```rust
// runner/gate_dispatch.rs
pub async fn run_gate_pipeline(task: &TaskDef, ...) -> GateResult {
    // 1. cargo check (compile gate)
    // 2. cargo test (test gate)
    // 3. cargo clippy (lint gate)
    // 4. diff validation (scope gate)
}
```

## What the Graph Engine Is Missing

### All 7 cognitive loop cells are stubs

From `.roko/GAPS.md` (Task 103):

| Cell | Current | Needed |
|------|---------|--------|
| `signal-reader` | PassthroughCell | Read signals from substrate |
| `relevance-scorer` | PassthroughCell | Score relevance of context |
| `system-prompt-builder` | PassthroughCell | Build 9-layer system prompt |
| `claude-agent` | PassthroughCell | Spawn LLM agent, run tool loop |
| `gate-pipeline` | PassthroughCell | Run compile/test/clippy/diff gates |
| `store-writer` | PassthroughCell | Persist results to substrate |
| `event-publisher` | PassthroughCell | Emit events to bus |

### TaskExecutorCell missing from Task 101

- `TaskExecutorCell.execute()` dry_run path not wired to real dispatch
- Graph Engine `--resume-plan` not supported
- No state persistence between runs

### Task 102: Engine not parallel

- `max_parallel` metadata is stored but **not used** for concurrent node dispatch
- All nodes execute sequentially despite the plan supporting parallelism

## Fix Options

### Option A: Enable Runner v2 as default (Quick fix, ~30 min)

Flip the default engine from Graph to Runner v2:

```rust
// plan.rs:264
if matches!(engine, PlanEngine::RunnerV2) || !cfg!(feature = "legacy-runner-v2") {
    // Use Runner v2 by default, Graph Engine as opt-in
}
```

Or just enable the `legacy-runner-v2` feature in the default Cargo build:

```toml
# crates/roko-cli/Cargo.toml
[features]
default = ["legacy-runner-v2"]
```

**Pros**: Working agent dispatch today. No new code needed.
**Cons**: Graph Engine remains unused. Doesn't advance the architecture.

### Option B: Wire TaskExecutorCell to Runner v2 dispatch (Medium, ~4 hr)

Have the Graph Engine delegate to Runner v2's agent spawn:

```rust
impl Cell for TaskExecutorCell {
    async fn execute(&self, input: Vec<Engram>, ctx: &CellContext) -> Result<Vec<Engram>> {
        // 1. Extract TaskDef from node config
        let task = TaskDef::from_toml(&self.config)?;

        // 2. Build agent spawn config (model, prompt, tools)
        let spawn_config = build_spawn_config(&task, ctx)?;

        // 3. Spawn agent via Runner v2's agent_stream
        let (tx, rx) = mpsc::channel(100);
        let handle = runner::agent_stream::spawn_agent(&spawn_config, tx).await?;

        // 4. Collect output
        let output = collect_agent_output(rx).await?;

        // 5. Run gate pipeline
        let gate_result = runner::gate_dispatch::run_gate_pipeline(&task, ...).await?;

        // 6. Return as Engram
        Ok(vec![Engram::from_agent_output(output, gate_result)])
    }
}
```

**Requires**: Passing `RunConfig`, model routing, event channel into `CellContext`.
**Pros**: Graph Engine becomes real. Preserves graph structure for parallelism.
**Cons**: Significant plumbing work.

### Option C: Hybrid — Graph for structure, Runner v2 for execution (Recommended, ~2 hr)

1. Graph Engine loads the plan and validates the DAG
2. Converts the graph back to a task list with dependency order
3. Hands off to Runner v2's event loop for actual execution
4. Graph Engine reports results from Runner v2

```rust
async fn cmd_plan_run_engine(...) {
    // 1. Load and validate graph (existing code)
    let graph = plan_to_graph(...);
    engine.validate(&graph)?;

    // 2. Extract execution order from graph
    let ordered_tasks = graph.topological_sort();

    // 3. Hand off to Runner v2
    runner::event_loop::run(ordered_tasks, config).await?;
}
```

**Pros**: Uses graph validation without duplicating agent dispatch.
**Cons**: Doesn't fully use graph execution model.

## Immediate Action Needed

**The current default (`roko plan run`) does nothing.** This needs to be fixed before
any self-hosting can happen. The simplest fix:

1. Enable `legacy-runner-v2` as default feature
2. OR change the default engine from Graph to RunnerV2
3. OR implement Option C (hybrid)

## Files to Modify

| File | Change |
|------|--------|
| `crates/roko-cli/Cargo.toml` | Add `legacy-runner-v2` to default features |
| `crates/roko-cli/src/commands/plan.rs:264` | Change default engine |
| `crates/roko-graph/src/cells/task_executor.rs` | Wire real dispatch (Option B) |
| `crates/roko-graph/src/engine.rs` | Add parallel execution (Task 102) |
| `.roko/GAPS.md` | Update task 101-103 status |

## BUG #06: Preflight Verify Skips Agent Dispatch

**Even when Runner v2 is enabled, agents may never spawn.**

### The Preflight Verify Logic

`crates/roko-cli/src/runner/event_loop.rs:3790-3998`:

```rust
fn task_should_preflight_verify(task_def: &TaskDef, attempt_num: u32) -> bool {
    attempt_num == 1 && !task_def.verify.is_empty() && !task_role_is_read_only(Some(task_def))
}
```

On first attempt, if the task has verify steps and isn't read-only, the runner runs the gate
pipeline **before** spawning any agent. If the gates pass:

```rust
if preflight.passed {
    info!(
        plan_id = %plan_id,
        task = %task_id,
        duration_ms = preflight.duration_ms,
        "task verification already passes -- skipping agent"
    );
    // → marks task complete, never spawns agent
}
```

### Why This Is a Problem

For **new crate scaffolding tasks**, the verify steps are typically:
1. `cargo check -p <crate-name>` (compile gate)
2. `cargo test -p <crate-name>` (test gate)

If stub files already exist (e.g., `lib.rs` with `pub mod core;`), `cargo check` passes.
The runner concludes "verification already passes" and skips the agent entirely.

**Result**: Tasks that need real implementation are marked complete because their
skeleton code compiles. No agent ever runs. No real code is generated.

### When Preflight Skip Is Correct vs Wrong

| Scenario | Preflight Skip | Correct? |
|----------|---------------|----------|
| Task: "fix bug X", tests already pass | Skip agent | **Yes** — nothing to do |
| Task: "implement feature Y", stub compiles | Skip agent | **No** — stub isn't the feature |
| Task: "add tests for Z", no test file exists | Don't skip | **Yes** — verify fails |
| Task: "refactor module W", cargo check passes | Skip agent | **No** — no refactoring done |

The fundamental issue: **compile gates don't verify that work was done, only that code compiles.**
A stub `pub struct Foo;` passes `cargo check` but isn't a real implementation.

### Fix Options

1. **Add `--force` flag**: Skip preflight verify, always dispatch agents
2. **Content-aware verify**: Check that files actually changed, not just that they compile
3. **Task-type awareness**: Only preflight-skip for `fix` tasks, not `implement`/`scaffold` tasks
4. **Disable by default**: Only enable preflight verify with `--preflight-verify` flag

### Files to Modify

| File | Change |
|------|--------|
| `crates/roko-cli/src/runner/event_loop.rs:3790` | Add `--force` check or task-type filter |
| `crates/roko-cli/src/runner/event_loop.rs:3967` | Condition preflight on task role/type |
| `crates/roko-cli/src/commands/plan.rs` | Add `--force` or `--no-preflight` CLI flag |

## BUG #07: Gate Crate Name Extraction Fails on Nested Paths

### The Bug

`crates/roko-cli/src/task_helpers.rs:49-68`:

```rust
pub(crate) fn crate_name_for_path(path: &str) -> Option<String> {
    let parts: Vec<&str> = normalized.split('/').filter(|part| !part.is_empty()).collect();
    match parts.as_slice() {
        [first, second, ..] if *first == "crates" || *first == "apps" => {
            Some((*second).to_string())  // ← takes 2nd path segment
        }
        // ...
    }
}
```

For `crates/hdc/core/src/lib.rs`, this extracts `hdc`. But the Cargo package is `kora-hdc`
(with a different name than the directory). `cargo check -p hdc` fails:

```
error: package ID specification `hdc` did not match any packages
```

### Why It Breaks

The function assumes `crates/<dirname>` == Cargo package name. This is wrong when:
- Package name differs from directory name (`kora-hdc` in `crates/hdc/core/`)
- Crate is nested under a group directory (`crates/hdc/core/` vs `crates/kora-hdc/`)
- Workspace uses path dependencies with custom names

### Fix Options

1. **Parse Cargo.toml**: Read `[package] name` from the nearest Cargo.toml
2. **Use `cargo metadata`**: Query the workspace for actual package names
3. **Normalize at plan time**: Store Cargo package names in task files, not directory names

## Evidence: Daeji Runs (2026-05-08)

### Run 1: Graph Engine (no-op)

```
$ roko plan run plans/ --approval --fresh
▸ Running plan via Graph Engine (8 tasks): hdc-core-crate
  Plan 'hdc-core-crate' completed: 8 nodes, 8 output signals, SUCCESS
```

8 tasks in ~2 seconds. TaskExecutorCell stub. No agents, no code.

### Run with Runner v2 (after rebuilding with `legacy-runner-v2`)

From `.roko/state/state-snapshot.json`:
- **Plan status**: `failed` (after 3 iterations)
- **Error**: `gate failed and retries exhausted: compile:cargo — error: package ID specification 'hdc' did not match any packages`
- **Agent calls**: 3 (agents DID spawn with Runner v2)
- **Cost**: $0.17 (86 tokens in, 1,645 tokens out)
- **Model**: `claude-sonnet-4-6`
- **Tasks completed**: 0/8
- **Tasks failed**: 1 (T01 — gate failure on crate name)

### Run 3: Runner v2, hdc-core-crate (SUCCESS after gate fixes)

From log: `run complete — Succeeded total_tasks=8 completed=8 failed=0 cost_usd=2.7468 agent_calls=8 duration_secs=621`

All 8 tasks completed. Structural verify steps drove agent dispatch correctly:
- T01: preflight pass (scaffold already done)
- T02-T05: structural check failed → agent dispatched → implemented → gates pass
- T06: failed first attempt (structural), auto-fixed on retry
- T07-T08: passed after agent implementation

### Run 4: Runner v2, hdc-core-fixes (SUCCESS)

From log: `run complete — Succeeded total_tasks=6 completed=6 failed=0 cost_usd=1.0593 agent_calls=5 duration_secs=284`

Audit fix tasks. F01 failed on first run attempt (haiku model routing), passed after
model_hint was fixed. F02-F03 preflight-skipped (already done). F04 needed 2 retries
due to overly-broad structural verify pattern.

### Run Cost Summary

| Run | Plan | Tasks | Agents | Cost | Duration | Outcome |
|-----|------|-------|--------|------|----------|---------|
| 1 | hdc-core-crate | 8 | 0 | $0.00 | 2s | No-op (Graph Engine) |
| 2 | hdc-core-crate | 8 | 3 | $0.17 | 445s | Failed (crate name bug) |
| 3 | hdc-core-crate | 8 | 8 | $2.75 | 621s | **Success** |
| 4 | hdc-core-fixes | 6 | 5 | $1.06 | 284s | **Success** |
| **Total** | | **30** | **16** | **$3.98** | **~22 min** | |

### The Cascade of Issues

```
Problem 1: Graph Engine is default → TaskExecutorCell is a stub → no agents
    ↓ fix: enable legacy-runner-v2 feature flag
Problem 2: Runner v2 runs, but crate_name_for_path extracts wrong name
    ↓ fix: move crates/hdc/core/ → crates/kora-hdc/ (or fix name extraction)
Problem 3: Gates pass on stub code → preflight verify skips agents
    ↓ fix: add --force flag or content-aware verify
Problem 4: Gate pipeline runs global compile, not per-crate
    ↓ fix: disable global gates when per-task verify exists
```

Each fix reveals the next bug. The full cascade must be resolved for plan execution to work.

### Gate Threshold Learning

The daeji run's gate threshold data shows 0% pass rate at rung 2:
```json
{"rungs": {"2": {"pass_count": 0, "total_count": 3, "ema_pass_rate": 0.0}}}
```

This is the adaptive gate system correctly learning that rung 2 gates are failing.
But the learning doesn't help because the root cause is a crate name bug, not a threshold issue.

## Evidence: Previous Run Was Also a No-Op

From the self-dev-ux execution state:
- **55 tasks completed**, 0 agent outcomes, 0 tokens used, $0.00 cost
- Gates ran 110 times, all passed with no file changes
- Duration: 115 minutes (spent on gate verification of unchanged code)
- `agent_outcomes: 0` in run-ledger.jsonl

The entire self-dev-ux batch was executed without any real agent dispatch.
The commits (L01-L10, M01-M15, H07-H09, V01) were likely made manually or via
a different code path, not through `roko plan run`.

## BUG #08: TUI Shows "0 Active Agents" While Agents Are Running

### Symptom

The TUI dashboard shows `0 active agents` and `Live Stream connecting...` while the runner
is actively dispatching CLI subprocess agents that are implementing tasks and passing gates.

### Root Cause: Two Disconnected Systems

The runner publishes agent events correctly:

```rust
// event_loop.rs:4435
ctx.tui.agent_spawned(&agent_id, role, &model_display);
// → publishes DashboardEvent::AgentSpawned to StateHub
```

StateHub receives and stores the event in its in-memory snapshot:

```rust
// state_hub.rs:293-299 → dashboard_snapshot.rs:996-1039
snap.apply(&DashboardEvent::AgentSpawned { .. });
// → Inserts AgentState into snapshot.agents HashMap ✓
```

**But the TUI ignores the StateHub snapshot.** Instead it reads from static JSON files:

```rust
// tui/dashboard.rs:700-710
self.agents = load_agents(&self.executor_state);      // ← reads executor.json
merge_runtime_agents(&mut self.agents, &self.root);   // ← reads runtime/agents.json
// Neither file is populated by the runner's CLI agent dispatch!
```

### The Disconnect

```
Runner spawns agent → TuiBridge → StateHub (in-memory) ✓
                                      ↓
                              TUI DashboardData
                              reads executor.json ✗ (not populated)
                              reads runtime/agents.json ✗ (not created)
                                      ↓
                              Shows: "0 active agents"
```

| Data Source | Updated by Runner? | Read by TUI? |
|-------------|-------------------|-------------|
| StateHub in-memory snapshot | Yes (via DashboardEvent) | **No** |
| `.roko/state/executor.json` → `assigned_agents` | **No** | Yes |
| `.roko/runtime/agents.json` | **No** | Yes |

### Fix Options

1. **Populate executor.json** — Update `plan_states[plan_id].assigned_agents` when agent spawns.
   Minimal change, fits existing TUI architecture.

2. **Populate runtime/agents.json** — Write agent PIDs when runner spawns CLI agents.
   The runtime merge logic already checks `kill(pid, 0)` for liveness.

3. **Read from StateHub** — Make `DashboardData` consume the live StateHub snapshot.
   Correct fix but bigger refactor.

### Files to Modify

| File | Change |
|------|--------|
| `crates/roko-cli/src/runner/event_loop.rs:4435` | Also write to executor state or runtime file |
| `crates/roko-cli/src/tui/dashboard.rs:700-710` | Read from StateHub snapshot (Option 3) |
| `crates/roko-cli/src/tui/dashboard.rs:2179-2210` | Fix `load_agents` to include CLI agents |
| `crates/roko-cli/src/tui/dashboard.rs:2236-2260` | Fix `merge_runtime_agents` to find CLI agents |

## BUG #09: Model Hint Routes to Wrong Provider (No Fallback)

### Symptom

Task F01 had `model_hint = "claude-haiku-4-5"`. The CascadeRouter routed this to
`provider=anthropic_api` (needs ANTHROPIC_API_KEY). Agent creation failed instantly:

```
dispatch: model selected model=claude-haiku-4-5 provider=anthropic_api
agent failed task — continuing with remaining independent tasks task_id=F01
```

The task failed with no retry on a different provider. No error message shown to user.

### Root Cause

`claude-haiku-4-5` is configured in roko.toml with `provider = "anthropic_api"` but
no ANTHROPIC_API_KEY is set. The model dispatcher has **no fallback** — it doesn't try
`provider=claude_cli` when `anthropic_api` fails.

### What Should Happen

1. Try configured provider (anthropic_api)
2. On auth failure, fall back to an alternative provider for the same model family
3. OR warn at plan load time that a model_hint references an unconfigured provider
4. OR validate provider health before dispatch (doctor-style check)

### Impact

Any task with a model_hint pointing to a provider without credentials silently fails.
The user sees no error — the task just appears "failed" with no explanation in the TUI.

## BUG #10: Config Validation Warnings Spam Logs

During every run, these warnings repeat on every model selection:

```
WARN config reference validation: agent.default_model references missing model 'claude-sonnet-4-6'
WARN config warning: duplicate model slug 'claude-sonnet-4-6' defined by keys: claude-sonnet-4-6, claude-sonnet
```

These fire on **every** model selection event (dozens of times per run). They indicate:
1. `agent.default_model` references a model key that doesn't exist as-named
2. Two model config entries resolve to the same slug

Neither is fatal, but they clutter logs and make it hard to find real errors.

### Fix

- Validate config once at startup, not per-selection
- Deduplicate model slugs at config load time
- Make `agent.default_model` reference validation case-insensitive or alias-aware

## BUG #11: Structural Verify Steps Can Be Too Broad

### Symptom

F04's verify step: `grep -q 'pub fn encode(text:' ... && ! grep 'fn encode(&self' ...`

This checks that NO `&self` encode method exists in the file. But `ProjectionEncoder::encode(&self, ...)`
is a different struct's method that legitimately takes `&self`. The verify step matched it and
marked the task as failing even after the agent correctly converted `TrigramEncoder::encode` to static.

### Root Cause

Structural verify grep patterns are file-scoped, not struct-scoped. When a file has multiple
types with similar method names, broad patterns cause false negatives.

### Impact

F04 went through 2 extra retry iterations ($0.51 wasted) before the agent eventually
restructured the code enough to not match the pattern. The task eventually passed, but only
because the agent happened to produce code that satisfied the overly-broad grep.

### Recommendations

1. **Scope patterns**: Use `grep -A5 'impl TrigramEncoder'` to scope to the right struct
2. **Use AST-aware checks**: `cargo expand` or tree-sitter queries instead of grep
3. **Warn on broad patterns**: Detect when a verify step matches in a location the task
   didn't modify (different struct/function than intended)

## Summary: All Plan Execution Bugs

| Bug | Where | Impact | Status |
|-----|-------|--------|--------|
| Graph Engine stub | `task_executor.rs` | Default engine does nothing | Known, documented |
| Runner v2 feature-gated | `Cargo.toml`, `plan.rs` | Working runner hidden behind flag | Fixed in daeji build |
| Preflight verify skip | `event_loop.rs:3992` | Agents skipped when stubs compile | NEW — needs fix |
| Crate name extraction | `task_helpers.rs:49` | Gates fail on nested crate paths | NEW — needs fix |
| TUI agent display | `dashboard.rs:700` | Shows 0 agents while agents run | NEW — needs fix |
| Model hint no fallback | `provider/mod.rs` | Wrong provider = silent task failure | NEW — needs fix |
| Config warning spam | `config/schema.rs` | Validation warns on every model selection | NEW — annoying |
| Broad verify patterns | `tasks.toml` | Grep matches wrong struct's methods | NEW — design issue |
| Global gate scope | `gate_dispatch.rs` | Compiles whole workspace, not task crates | Partially addressed |
| No `--force` flag | `plan.rs` | Can't bypass preflight verify | Missing feature |
