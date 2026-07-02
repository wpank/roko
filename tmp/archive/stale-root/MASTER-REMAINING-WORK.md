# ⚠️ SUPERSEDED — See [`MASTER-PLAN.md`](MASTER-PLAN.md)
>
> This document has been replaced by `MASTER-PLAN.md` which consolidates all remaining
> work into a single tier/section structure. Sections A-K below are absorbed into
> MASTER-PLAN.md Tiers 1-5. This file is retained for historical reference only.

---

# Roko: Master Remaining Work

> **Updated**: 2026-04-08 (SUPERSEDED)
>
> ~~This is the single source of truth for ALL remaining work to reach mori parity.~~
> Cross-references: `MORI-PARITY-CHECKLIST.md` (1,232 items, 36% done),
> `PROMPT-EXECUTOR-PARITY.md` (11 sections), implementation plans 05-10.
>
> **Status key**: DONE = checked in and verified. WIRED = code exists and is called.
> BUILT = code exists but is NOT called from runtime. MISSING = not implemented.
>
> **THE RULE**: Do NOT simplify. Do NOT skip items. Do NOT stub with println.
> Every handler must do real work. Verify each item before marking done.
> If verification fails, fix it before moving on.

---

## How to Use This File

1. Work through sections in order (A → B → C → ...)
2. Within each section, items are ordered by dependency
3. Every item has a verification command — run it
4. After each section: `cargo test --workspace --exclude roko-demo` must pass
5. After each section: `cargo clippy -p roko-cli --no-deps` must have zero new warnings
6. Mark items [x] only after verification passes

---

## Current State Summary

### What Works End-to-End (DONE)

These are verified, wired, and tested:

| Component | Evidence | Where |
|---|---|---|
| Plan discovery + DAG executor | `PlanRunner::run_task_plans()` calls `executor.tick()` in loop | `orchestrate.rs:1066-1107` |
| Agent dispatch (Claude CLI + ExecAgent) | `dispatch_agent_with()` builds prompt, spawns agent, collects output | `orchestrate.rs:2550-2780` |
| Per-task worktree isolation | `task_exec_dir()` calls `worktrees.create()`, parallel tasks get separate dirs | `orchestrate.rs:3315-3333` |
| Parallel task execution | `handle_implementing_parallel()` with `tokio::spawn` + `futures::join_all` | `orchestrate.rs:1695-1750` |
| Parallel concurrency cap | `max_parallel` from `tasks.toml` meta, falls back to `MAX_PARALLEL_TASKS` | `orchestrate.rs:1604-1612` |
| Single-task retry (2 retries) | `handle_implementing_single()` with retry loop | `orchestrate.rs:1648-1693` |
| AutoFix on gate failure | `handle_autofixing()` spawns AutoFixer, re-runs gates, up to 5 iterations | `orchestrate.rs` AutoFix handler |
| Review phase | `handle_reviewing()` spawns Auditor with ReviewerTemplate, read-only tools | `orchestrate.rs` Review handler |
| DocRevision + Merge | `handle_doc_revision()` spawns Scribe, `handle_merging()` uses WorktreeManager | `orchestrate.rs` Merge handler |
| Gate pipeline (compile, test, clippy, diff) | `run_gate_pipeline()` runs 6-rung pipeline per task | `orchestrate.rs:2380-2520` |
| Adaptive gate thresholds | EMA per rung, persisted to `.roko/learn/gate-thresholds.json` | `orchestrate.rs` |
| 6-layer system prompts | `RoleSystemPromptSpec` + `SystemPromptBuilder` + templates | `orchestrate.rs:2650-2710` |
| Episode logging | Every agent turn + gate → `.roko/episodes.jsonl` | `orchestrate.rs` |
| Efficiency events | Per-turn section tokens, tool usage, cost → `.roko/learn/efficiency.jsonl` | `orchestrate.rs` |
| Cascade model routing | Persists to `.roko/learn/cascade-router.json` | `orchestrate.rs` |
| Prompt experiments (A/B) | `ExperimentStore` in `.roko/learn/experiments.json` | `orchestrate.rs` |
| Cost budget enforcement | Per-task and per-plan USD caps checked before dispatch | `orchestrate.rs:2572-2600` |
| State persistence + resume | Auto-saves to `.roko/state/executor.json`, `--resume` flag | `orchestrate.rs` |
| Process supervisor | `PlanRunner` tracks agent processes via `bardo-runtime` | `orchestrate.rs` |
| MCP config passthrough | `agent.mcp_config` in `roko.toml` → `--mcp-config` | `orchestrate.rs` |
| Cross-plan dependency ordering | `UnifiedTaskDag` respects `depends_on` frontmatter | `orchestrate.rs` |
| Context attribution | Scans output for references to injected sections, logs to `context-attribution.jsonl` | `orchestrate.rs` |
| POST-merge regression runner | After merge, re-runs gates on merged branch | `orchestrate.rs` |
| HTTP API (`roko serve`) | Full CRUD: templates, plans, PRDs, research, config, deployments | `serve/` module |
| Cloud deploy backends | Railway API, Railway CLI, Manual bundle generation | `serve/deploy/` |
| Worker subcommand | `roko worker` reads template from env, serves `/task` | `worker/` module |

### What's BUILT but NOT WIRED

These have implementations in crates but are NOT called from `orchestrate.rs` or the CLI:

| Component | Crate/Path | What's Missing |
|---|---|---|
| Conductor watchers (10) | `roko-conductor/src/watchers/` | Not called after phase transitions |
| Skill library | `roko-learn/src/skill_library.rs` | Not called on success; not queried before dispatch |
| Playbook store | `roko-learn/src/playbook.rs` | Not called |
| LinUCB bandit (model routing) | `roko-learn/src/model_router.rs` | `observe()` not called; `select_model()` not called |
| TraceSink | `roko-core/src/trace.rs` | Not initialized |
| MetricsSink | `roko-core/src/metrics.rs` | Not initialized |
| ToolTrace | `roko-core/src/tool/trace.rs` | Not emitted |
| FailureTrace | `roko-core/src/failure.rs` | Not emitted |
| MetricRegistry + Prometheus | `roko-agent/src/metrics/` | Not initialized, no endpoint |
| Per-role tool matrix | `roko-std/src/tool_registry.rs` | `--tools` not passed per role |
| MCP server lifecycle | `roko-agent/src/mcp/` | Server not spawned/killed with agent |
| Progressive tool discovery | `roko-std/` | Bandit-based subset not wired |

### What's MISSING (not implemented at all)

| Component | Scope | Priority |
|---|---|---|
| Interactive TUI (ratatui) | 26 widgets, 13 modals, live agent status | P1 — can't watch roko work without it |
| Failure-driven re-planning | Failed tasks feed back into plan generator | P1 — self-healing |
| Automatic plan generation | PRD publish triggers `prd plan` automatically | P2 |
| Web frontend for `roko serve` | React/whatever UI hitting API + WebSocket | P3 |

---

## Section A: Wire Conductor Checks Between Phases

**Why**: The conductor (10 watchers: cost, loop, latency, token, error-rate, stall, memory, regression, dependency, quality) exists and is tested but never called. Without it, roko can't detect and intervene when things go wrong (infinite loops, budget blowout, stalled agents).

**What exists**:
- `roko-conductor/src/watchers/` — 10 watchers, all with `check()` method
- `roko-conductor/src/diagnosis.rs` — error pattern matching
- `roko-conductor/src/lib.rs` — `ConductorDecision` enum (Continue, Restart, Fail, Pause)

**Read first**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-conductor/src/lib.rs` — `Conductor` struct and `evaluate()` method
- `/Users/will/dev/nunchi/roko/roko/crates/roko-conductor/src/watchers/mod.rs` — watcher registry
- `/Users/will/dev/uniswap/bardo/apps/mori/src/conductor/watchers.rs:32` — mori's `check()` call sites

**Mori reference**: After every phase transition, mori calls `conductor.evaluate()` with current metrics. If any watcher fires, mori either restarts the task, fails it, or pauses execution.

### Checklist

- [ ] **A.1** Import `roko_conductor::{Conductor, ConductorDecision}` in `orchestrate.rs`
- [ ] **A.2** Add `conductor: Conductor` field to `PlanRunner`
- [ ] **A.3** Initialize conductor with all 10 watchers in `PlanRunner::new()`
- [ ] **A.4** After every `self.executor.apply_event()` call, call `self.conductor.evaluate()` with context:
  - `gate_results`: last gate pass/fail counts
  - `iteration_count`: current task retry count
  - `cost_so_far`: cumulative plan cost
  - `elapsed`: wall time since plan start
  - `context_tokens`: tokens in last prompt
  - `error_pattern`: last error message (if any)
- [ ] **A.5** Handle `ConductorDecision::Restart` → log reason, reset task to Implementing, increment iteration
- [ ] **A.6** Handle `ConductorDecision::Fail` → log reason, emit `Fatal` event for the plan
- [ ] **A.7** Handle `ConductorDecision::Pause` → log reason, set plan to Paused state (if executor supports it, otherwise treat as Continue with warning log)
- [ ] **A.8** Log every conductor decision to the event log: `[conductor] watcher={name} decision={decision} reason={reason}`

### Verify

```bash
grep -c 'conductor\|Conductor\|ConductorDecision\|evaluate' crates/roko-cli/src/orchestrate.rs  # >= 5
grep -c 'Watcher\|watcher' crates/roko-cli/src/orchestrate.rs  # >= 2
cargo test -p roko-cli --lib
cargo clippy -p roko-cli --no-deps 2>&1 | grep "^error" | wc -l  # 0
```

---

## Section B: Wire Skill Library (Extract on Success, Inject on Dispatch)

**Why**: Without skill learning, roko repeats the same mistakes. When a task succeeds, the context+prompt+model combination should be recorded. When a similar task comes up, that successful pattern should be injected as guidance.

**What exists**:
- `roko-learn/src/skill_library.rs` — `SkillLibrary` with `extract_skill()` and `query()` methods
- `roko-learn/src/playbook.rs` — `PlaybookStore` with `record()` and `lookup()` methods

**Read first**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/skill_library.rs` — full API
- `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/playbook.rs` — full API

### Checklist

- [ ] **B.1** Add `skill_library: SkillLibrary` field to `PlanRunner`
- [ ] **B.2** Initialize from `.roko/learn/skills.json` (create if absent, load if exists)
- [ ] **B.3** On task success (gate pass + merge): call `skill_library.extract_skill()` with:
  - `task_files`: files the task touched
  - `task_tier`: the task's complexity tier
  - `symbols`: symbol names referenced in the task
  - `model`: which model was used
  - `prompt_hash`: hash of the rendered prompt
  - `gate_results`: which gates passed and their scores
- [ ] **B.4** On task failure: record failure pattern with `skill_library.record_failure()` (if method exists) or store in separate failure log
- [ ] **B.5** Before building context for a new task in `dispatch_agent_with()`:
  - Call `skill_library.query(task_files, task_tier, symbols)`
  - If a matching skill is found with success rate > 0.5, inject its patterns as a `Low`-priority context section
  - Cap injected skill context at 1,024 tokens
- [ ] **B.6** Persist skill library to `.roko/learn/skills.json` after each successful extraction
- [ ] **B.7** Add `playbook: PlaybookStore` field, initialize from `.roko/learn/playbooks.json`
- [ ] **B.8** On task success: call `playbook.record()` with task definition + outcome
- [ ] **B.9** Before dispatch: call `playbook.lookup(task_type)` and inject as context if found

### Verify

```bash
grep -c 'skill_library\|SkillLibrary\|extract_skill\|query.*skill' crates/roko-cli/src/orchestrate.rs  # >= 4
grep -c 'playbook\|PlaybookStore' crates/roko-cli/src/orchestrate.rs  # >= 2
test -f .roko/learn/skills.json && echo "skill file exists"  # after a successful run
cargo test -p roko-cli --lib
```

---

## Section C: Wire LinUCB Bandit for Model Routing

**Why**: The cascade router currently uses static tier → model mapping. The LinUCB bandit can learn which models work best for which task types, reducing cost and improving pass rates.

**What exists**:
- `roko-learn/src/model_router.rs` — LinUCB contextual bandit with 17-dim context vector, cold-start/confidence/UCB stages
- `roko-learn/src/cascade_router.rs` — 3-stage cascade (static → confidence → UCB), already wired for persistence

**Read first**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/model_router.rs` — `observe()` and `select_model()` API
- `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/cascade_router.rs` — `CascadeRouter::select()`

### Checklist

- [ ] **C.1** The `CascadeRouter` is already in `PlanRunner`. Verify it's initialized from `.roko/learn/cascade-router.json`
- [ ] **C.2** After task success: call `cascade_router.observe(context_vec, model_idx, reward)` where:
  - `reward = pass_rate * 0.5 + (1.0 - normalized_cost) * 0.3 + (1.0 - normalized_duration) * 0.2`
  - `context_vec` is built from: task tier (one-hot 4 dims), complexity scalar, iteration count, agent role hash, crate familiarity, prior failure flag
  - `model_idx` maps the model slug to an index in the router's model list
- [ ] **C.3** After task failure: call `cascade_router.observe(context_vec, model_idx, 0.0)`
- [ ] **C.4** Before model selection in `dispatch_agent_with()`:
  - Build context vector (same features as C.2)
  - Call `cascade_router.select(context_vec)`
  - If bandit has > 50 observations for this context region, use its recommendation
  - Otherwise fall back to `TaskDef.effective_model()` (static tier mapping)
- [ ] **C.5** Persist cascade router state after every observation (already partially wired — verify it actually writes)
- [ ] **C.6** Track a per-crate "familiarity score" = `success_count / total_count` and include it in the context vector

### Verify

```bash
grep -c 'cascade_router\|select_model\|observe.*reward\|context_vec' crates/roko-cli/src/orchestrate.rs  # >= 4
grep -c 'familiarity\|crate_score' crates/roko-cli/src/orchestrate.rs  # >= 1
cargo test -p roko-cli --lib
```

---

## Section D: Wire Context Attribution Feedback (Rolling Averages)

**Why**: Context attribution scanning is wired (Section 2 of PROMPT-EXECUTOR-PARITY, partially done) but rolling averages and dynamic demotion are not. Without this, roko keeps injecting context sections that agents never use.

**What exists**:
- Context attribution already logs `was_referenced` per section to `context-attribution.jsonl`
- `ContextProvider.resolve()` already runs with priority levels

### Checklist

- [ ] **D.1** Maintain a rolling average per `(task_tier, context_source_type)` — e.g. "focused tasks referencing PlanBrief 15% of the time"
  - Store in `.roko/learn/context-averages.json`
  - Use exponential moving average with alpha=0.1
- [ ] **D.2** When `ContextProvider.resolve()` runs for a task:
  - Load rolling averages
  - For each context source type, check average reference rate for this task tier
  - If reference rate < 10%, demote priority from `Normal` to `Low` (droppable under budget pressure)
- [ ] **D.3** Log context attribution decisions: `[context] plan_brief: included (ref_rate=0.42)` / `[context] research: dropped (ref_rate=0.03)`
- [ ] **D.4** Update rolling averages after each task completes (from the attribution scan results)

### Verify

```bash
grep -c 'rolling_average\|ref_rate\|context.*demote\|context.*drop' crates/roko-cli/src/orchestrate.rs  # >= 3
grep -c 'context-averages' crates/roko-cli/src/orchestrate.rs  # >= 1
cargo test -p roko-cli --lib
```

---

## Section E: Wire MCP Server Lifecycle + Per-Role Tool Matrix

**Why**: MCP config is passed through to Claude CLI, but the MCP server isn't spawned/killed as part of agent lifecycle. Also, different roles (Implementer, Reviewer, AutoFixer) should get different tool allowlists.

**What exists**:
- MCP client: `roko-agent/src/mcp/` with JSON-RPC, tool converter, dedup
- MCP config walk-up: auto-discovers `.mcp.json` in project root
- Tool registry: `roko-std/src/tool_registry.rs` with `ToolRegistry::for_call(role, task_ctx, limit)`

**Mori reference**: `connection.rs:2497-2510` — Reviewer gets `Read,Glob,Grep,Bash` only. AutoFixer gets `Read,Write,Edit,Bash`. Implementer gets all tools.

### Checklist

- [ ] **E.1** Before spawning a Claude CLI agent, check for `.mcp.json` in the worktree directory
- [ ] **E.2** If found, pass `--mcp-config <path>` to the agent command
- [ ] **E.3** Pass `--strict-mcp-config` when MCP config is active (prevents the agent from using tools not declared in config)
- [ ] **E.4** Skip MCP for AutoFixer and Conductor roles (they don't need code intelligence)
- [ ] **E.5** Define per-role tool allowlists:
  - `Implementer`: all tools
  - `Reviewer/Auditor`: `Read,Glob,Grep,Bash` (read-only)
  - `AutoFixer`: `Read,Write,Edit,Bash,Glob,Grep`
  - `Scribe`: `Read,Write,Edit,Glob`
  - `Conductor`: no external tools
- [ ] **E.6** Pass `--allowedTools` to Claude CLI based on role (already supported by Claude CLI)
- [ ] **E.7** If MCP server needs to be spawned (not just config passthrough), track its PID in ProcessSupervisor and kill on agent end
- [ ] **E.8** Tool result compression for outputs > 10K tokens (summarize before injecting into context)

### Verify

```bash
grep -c 'mcp-config\|mcp_config\|strict-mcp\|allowedTools\|allowed_tools' crates/roko-cli/src/orchestrate.rs  # >= 3
grep -c 'Implementer\|Reviewer\|AutoFixer.*tool\|tool.*role' crates/roko-cli/src/orchestrate.rs  # >= 3
cargo test -p roko-cli --lib
```

---

## Section F: Wire Observability (TraceSink, MetricsSink, Prometheus)

**Why**: Without observability, roko is a black box. Can't debug failures, measure performance, or export metrics.

**What exists (all built, none wired)**:
- `TraceSink` trait: `roko-core/src/trace.rs`
- `MetricsSink` trait: `roko-core/src/metrics.rs`
- `ToolTrace` struct: `roko-core/src/tool/trace.rs`
- `FailureTrace` struct: `roko-core/src/failure.rs`
- `MetricRegistry`: `roko-agent/src/metrics/`
- Prometheus exporter: `roko-agent/src/metrics/prometheus.rs`

### Checklist

- [ ] **F.1** Initialize `TraceSink` at CLI startup (in `PlanRunner::new()`)
  - Read sink config from `roko.toml` — file sink to `.roko/traces.jsonl`
  - Register the sink so all trace events flow to it
- [ ] **F.2** Initialize `MetricsSink` at CLI startup
  - File sink to `.roko/metrics.jsonl`
- [ ] **F.3** Agent dispatch emits `ToolTrace` for each tool invocation
  - After parsing agent output, extract tool calls and emit traces
- [ ] **F.4** Failed operations emit `FailureTrace`
  - Gate failures, agent crashes, merge conflicts, budget exhaustion
- [ ] **F.5** `MetricRegistry` tracks these counters:
  - `agent_runs_total` (labels: role, model, plan_id)
  - `agent_duration_seconds` (histogram)
  - `gate_pass_rate` (labels: gate_name)
  - `tokens_used_total` (labels: model, direction=input/output)
  - `cost_usd_total` (labels: model)
- [ ] **F.6** Add `/metrics` endpoint to `roko serve` (Prometheus format)
  - Only when `roko serve` is running, expose scrape-able endpoint
- [ ] **F.7** Structured JSON logs via `tracing-subscriber` JSON formatter
  - Enable with `--json-logs` flag or `ROKO_JSON_LOGS=1` env var
- [ ] **F.8** Cost attribution per agent/role/task in trace spans
  - Every agent dispatch wrapped in a tracing span with cost fields
- [ ] **F.9** `ToolTrace` and `FailureTrace` serialized to episode artifacts alongside episodes.jsonl

### Verify

```bash
grep -c 'TraceSink\|MetricsSink\|ToolTrace\|FailureTrace' crates/roko-cli/src/orchestrate.rs  # >= 4
grep -c 'MetricRegistry\|agent_runs_total\|gate_pass_rate' crates/roko-cli/src/orchestrate.rs  # >= 2
curl -s localhost:3000/metrics | head -5  # when roko serve is running
cargo test -p roko-cli --lib
```

---

## Section G: Failure-Driven Re-Planning

**Why**: When a task fails after all retries + auto-fix, roko currently just marks it Failed and moves on. Mori feeds failure patterns back into the plan generator so it produces different task decomposition next time.

**What exists**: Nothing — this is new code.

**Mori reference**: When a plan accumulates too many failures, mori pauses execution, re-generates the plan with failure context, and resumes.

### Checklist

- [ ] **G.1** When a task fails after max retries AND max auto-fix iterations:
  - Record failure pattern to `.roko/learn/failure-patterns.jsonl`:
    ```json
    {"task_id": "...", "plan_id": "...", "error_type": "compile|test|clippy|timeout|budget",
     "files": ["src/foo.rs"], "model": "claude-sonnet-...", "iterations": 7,
     "last_error": "...", "timestamp": "..."}
    ```
- [ ] **G.2** Add `failure_patterns: Vec<FailurePattern>` to `PlanRunner` (loaded from file at startup)
- [ ] **G.3** When `roko prd plan <slug>` generates a new plan, query failure patterns:
  - Group by files and crate
  - If any file/crate has > 2 failures, inject into generation prompt:
    ```
    WARNING: Previous attempts to modify {file} have failed {n} times.
    Common error: {last_error}
    Recommendation: decompose changes to {file} into smaller tasks (< 30 LOC each)
    or use a different approach than {previous_model}.
    ```
- [ ] **G.4** Track per-crate familiarity score: `success_count / (success_count + failure_count)`
  - Persist to `.roko/learn/crate-familiarity.json`
  - Inject as context: "crate `roko-agent` familiarity: 0.73 (11/15 tasks succeeded)"
- [ ] **G.5** When a plan has > 50% failed tasks:
  - Emit a `PlanRegenerationNeeded` event
  - If automatic regeneration is enabled (config flag `learn.auto_regenerate = true`):
    - Pause the plan
    - Call `prd plan` with failure context injected
    - Replace remaining tasks with regenerated ones
    - Resume execution
  - If not enabled: log warning and continue

### Verify

```bash
test -f .roko/learn/failure-patterns.jsonl && echo "failure patterns file exists"
grep -c 'failure_pattern\|FailurePattern\|familiarity' crates/roko-cli/src/orchestrate.rs  # >= 3
grep -c 'auto_regenerate\|PlanRegenerationNeeded' crates/roko-cli/src/orchestrate.rs  # >= 1
cargo test -p roko-cli --lib
```

---

## Section H: Interactive TUI (ratatui)

**Why**: This is the biggest UX gap. Without it, you can't watch roko work. Mori has 26 widgets, 13 modals, 6 views. The current `roko dashboard` just dumps text to stdout.

**What exists**:
- `roko-cli/src/tui.rs` — `DashboardScaffold` with 6 pages (Health, Trends, Correlations, Parameters, Experiments, Optimizer)
- Each page has widgets but they render to text strings, not terminal UI
- `roko serve` has WebSocket event streaming

**Read first**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui.rs` — current scaffold
- `/Users/will/dev/nunchi/roko/roko/tmp/implementation-plans/09-tui-dashboard.md` — plan
- `/Users/will/dev/nunchi/roko/bardo-backup/tmp/roko-progress/COMPONENTS/tui/` — 5 spec files
- `/Users/will/dev/uniswap/bardo/apps/mori/src/tui/` — mori TUI (reference)

**Dependency**: Add `ratatui = "0.29"` and `crossterm = "0.28"` to workspace + roko-cli Cargo.toml.

### Phase H.1: Core TUI Framework

- [ ] **H.1.1** Add `ratatui` and `crossterm` dependencies
- [ ] **H.1.2** Create `crates/roko-cli/src/tui/mod.rs` — TUI app struct with:
  - `App` struct holding `terminal: Terminal<CrosstermBackend<Stdout>>`
  - `run()` method with event loop: crossterm events + tick timer (250ms)
  - Keyboard: `q` quit, `Tab`/`Shift-Tab` switch pages, `j/k` scroll, `?` help
  - Graceful shutdown on ctrl-c
- [ ] **H.1.3** Create `crates/roko-cli/src/tui/layout.rs` — screen layout:
  - Header bar: roko version + current page name + time + key hints
  - Main area: current page content
  - Footer: status bar with plan progress, cost, agent count
- [ ] **H.1.4** Create `crates/roko-cli/src/tui/event.rs` — event channel:
  - Receive crossterm key/mouse events
  - Receive `ServerEvent` from EventBus (plan updates, agent status, gate results)
  - Tick events for periodic refresh
- [ ] **H.1.5** Wire into `roko dashboard` command:
  - `roko dashboard` enters TUI mode (full terminal takeover)
  - `roko dashboard --text` keeps current text dump behavior
  - Pass EventBus to TUI for live updates

### Phase H.2: Plan Execution View (Primary View)

This is the "watch roko work" view — equivalent to mori's main screen.

- [ ] **H.2.1** Plan list panel (left sidebar):
  - Each plan: name, phase, progress bar (tasks done / total), cost
  - Color coding: green=complete, yellow=running, red=failed, gray=queued
  - Highlight currently selected plan
- [ ] **H.2.2** Task detail panel (right main area):
  - For selected plan: task list with status icons
  - Currently running task: agent name, model, tokens used, elapsed time
  - Completed tasks: pass/fail, duration, cost
  - Failed tasks: error summary, retry count
- [ ] **H.2.3** Live agent output panel (bottom):
  - Stream agent stdout/stderr in real-time
  - Scroll buffer (last 500 lines)
  - Filter by plan/task
- [ ] **H.2.4** Gate results panel:
  - After gates run: show compile/test/clippy results inline
  - Color: green=pass, red=fail, yellow=warning

### Phase H.3: Dashboard Pages (from existing scaffold)

- [ ] **H.3.1** Health page: 6 gauges rendered as ratatui `Gauge` widgets
  - Pass rate, cost/task, iterations/task, haiku use %, prompt size, cache hit %
- [ ] **H.3.2** Trends page: sparkline widgets for time-series data
  - Learning velocity, regression detection, cost trend
- [ ] **H.3.3** Correlations page: table of learning-pack vs pass-rate correlations
- [ ] **H.3.4** Parameters page: tunable knobs with current values + impact ratings
- [ ] **H.3.5** Experiments page: A/B test results with z-test verdicts
- [ ] **H.3.6** Optimizer page: learning loop status with confidence bars

### Phase H.4: Interactive Controls

- [ ] **H.4.1** Pause/Resume: `p` pauses current plan execution, `r` resumes
  - Pausing sets `CancelToken` on the plan's agent
  - Resuming re-enters the executor loop
- [ ] **H.4.2** Cancel: `x` cancels selected plan (with confirmation modal)
- [ ] **H.4.3** Retry: `R` retries a failed task (resets to Implementing)
- [ ] **H.4.4** Model override: `m` opens modal to change model for next dispatch
- [ ] **H.4.5** Log filter: `/` opens search/filter for log panel

### Verify

```bash
# Framework
cargo run -p roko-cli -- dashboard 2>/dev/null  # should enter TUI mode
cargo run -p roko-cli -- dashboard --text  # should print text dump

# During plan execution (in separate terminal)
cargo run -p roko-cli -- plan run plans/ &
cargo run -p roko-cli -- dashboard  # should show live progress

# Key verification
grep -c 'ratatui\|crossterm' crates/roko-cli/Cargo.toml  # 2
grep -c 'Terminal\|CrosstermBackend\|draw\|render' crates/roko-cli/src/tui/mod.rs  # >= 5
cargo test -p roko-cli --lib
cargo clippy -p roko-cli --no-deps 2>&1 | grep "^error" | wc -l  # 0
```

---

## Section I: Automatic Plan Generation on PRD Publish

**Why**: Currently `roko prd plan <slug>` is manual. For full self-hosting, publishing a PRD should automatically trigger plan generation.

### Checklist

- [ ] **I.1** In `roko prd draft promote` (the command that publishes a PRD):
  - After successful promotion, check config flag `prd.auto_plan = true`
  - If enabled, call `roko prd plan <slug>` automatically
  - Log: `[prd] auto-generating plan for published PRD: {slug}`
- [ ] **I.2** In `roko serve`, when a PRD is promoted via API:
  - Emit `PrdPublished { slug }` event
  - If auto_plan is enabled, spawn background task to generate plan
  - Emit `PlanGenerated { slug, plan_dir }` when complete
- [ ] **I.3** Add `auto_plan: bool` to PRD config section in `roko.toml` (default: false)
- [ ] **I.4** Add `auto_execute: bool` to config (default: false) — if both auto_plan and auto_execute are true, also start `plan run` after generation

### Verify

```bash
# Manual trigger
cargo run -p roko-cli -- prd draft promote --slug test-prd
ls plans/test-prd/tasks.toml  # should exist if auto_plan = true

# Config
grep 'auto_plan' crates/roko-core/src/config/schema.rs  # >= 1
cargo test -p roko-cli --lib
```

---

## Section J: Plan Execution via `roko serve` API

**Why**: For cloud deployment, the orchestrator needs to be triggerable via HTTP, not just CLI.

### Checklist

- [ ] **J.1** Add `POST /api/plans/:dir/run` endpoint:
  - Accepts `{ resume_path?: string }` body
  - Creates `PlanRunner` from the plan directory
  - Spawns execution in background task
  - Returns `{ id: "...", status: "running" }`
  - Stores run handle in `AppState`
- [ ] **J.2** Add `POST /api/plans/:id/pause` endpoint:
  - Calls `cancel.cancel()` on the plan's CancelToken
  - Returns `{ status: "paused" }`
- [ ] **J.3** Add `POST /api/plans/:id/resume` endpoint:
  - Re-creates PlanRunner from saved state
  - Resumes execution
- [ ] **J.4** Add `DELETE /api/plans/:id/run` endpoint:
  - Cancels execution and tears down agents
- [ ] **J.5** Stream plan execution events via existing WebSocket
  - `PlanRunner` emits events through `EventBus`
  - WebSocket clients receive live updates
- [ ] **J.6** Add `GET /api/plans/:id/run/status` endpoint:
  - Returns current phase, task progress, cost, duration

### Verify

```bash
# Start serve
cargo run -p roko-cli -- serve &
sleep 2

# Trigger plan execution
curl -X POST localhost:3000/api/plans/test-plan/run | jq .
curl localhost:3000/api/plans/test-plan/run/status | jq .

# WebSocket should show events
websocat ws://localhost:3000/ws

cargo test -p roko-cli --lib
```

---

## Section K: Regenerate Old-Format Plans

**Why**: Some existing plans have old-format `tasks.toml` (no tier, context, verify). They need regeneration.

### Checklist

- [ ] **K.1** Run `roko plan generate --from-file plans/P06-process-management/plan.md` to regenerate P06
- [ ] **K.2** Verify P06/tasks.toml has: `tier`, `model_hint`, `context.read_files`, `verify` steps
- [ ] **K.3** Run `roko plan generate --from-file plans/W01-wire-system-prompts/plan.md` to regenerate W01
- [ ] **K.4** Verify W01/tasks.toml has the same fields
- [ ] **K.5** Run `roko plan run plans/W01-wire-system-prompts/` and verify it executes through all phases
- [ ] **K.6** Any plan directory without `tier` in its tasks.toml should be flagged by `roko plan list`

### Verify

```bash
grep -c 'tier\|model_hint\|read_files\|verify' plans/P06-process-management/tasks.toml  # >= 10
grep -c 'tier\|model_hint\|read_files\|verify' plans/W01-wire-system-prompts/tasks.toml  # >= 10
cargo run -p roko-cli -- plan list  # should flag old-format plans
```

---

## Priority Order

For reaching "runs like mori" (can watch roko execute plans with live feedback):

| Priority | Section | What | Effort |
|---|---|---|---|
| **P0** | H.1-H.2 | TUI framework + plan execution view | Large — this is the big one |
| **P0** | J | Plan execution via serve API | Medium |
| **P1** | A | Wire conductor checks | Small |
| **P1** | G | Failure-driven re-planning | Medium |
| **P1** | E | MCP lifecycle + tool matrix | Small |
| **P2** | B | Wire skill library | Small |
| **P2** | C | Wire LinUCB bandit | Small |
| **P2** | D | Context attribution feedback | Small |
| **P2** | F | Observability wiring | Medium |
| **P3** | H.3-H.5 | Dashboard pages + interactive controls | Medium |
| **P3** | I | Auto plan generation | Small |
| **P3** | K | Regenerate old plans | Small |

**The minimum to "run like mori"**: Sections H.1-H.2 (TUI) + J (API execution) + A (conductor).
Everything else makes it better but isn't required to watch roko chug through a queue.

---

## Cross-Reference to Existing Plans

| This Section | PROMPT-EXECUTOR-PARITY.md | Implementation Plans | MORI-PARITY-CHECKLIST |
|---|---|---|---|
| A (Conductor) | Section 7 | — | §35 |
| B (Skills) | Section 9b | 05-learning-wiring.md | §27, I.3 |
| C (Bandit) | Section 9a | 05-learning-wiring.md | §16 |
| D (Context) | Section 2 | — | §12 |
| E (MCP/Tools) | — | 07-mcp-tool-wiring.md | §22, §36 |
| F (Observability) | — | 08-observability-wiring.md | §40, I.4 |
| G (Re-planning) | Section 9c | — | new |
| H (TUI) | — | 09-tui-dashboard.md | §18, §19 |
| I (Auto plan) | — | — | new |
| J (API execution) | — | — | new |
| K (Regen plans) | Section 11 | — | — |
