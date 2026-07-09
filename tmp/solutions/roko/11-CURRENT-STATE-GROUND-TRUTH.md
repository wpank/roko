# 11 - Current State Ground Truth

Source-code-level analysis of what roko actually does vs. what it claims.
Produced by reading the live source files on branch `wp-arch2`, 2026-04-29.

---

## 1. CLI Commands: Reality vs. Claims

### 1.1 `roko run "<prompt>"`

**Claim**: "Seed a prompt and run the universal loop (compose -> agent -> gate -> persist)."

**Actual flow** (three paths, selected at runtime):

| Path | When chosen | What happens |
|---|---|---|
| **V2 WorkflowEngine** | `--engine v2` (default) | Calls `run_with_workflow_engine_with_hub` in `run.rs`. This creates `ServiceFactory::build` -> `EffectServices` -> `WorkflowEngine::run`. The engine drives a `PipelineStateV2` state machine in a loop: Start -> StrategyPhase (optional) -> ImplementPhase -> GatePhase -> ReviewPhase (optional) -> CommitPhase -> Done. Each step calls `EffectDriver` for side effects (model calls, gate execution). |
| **Inline rendering** | TTY, not `--json`/`--quiet`, engine=Legacy | Calls `run_once_inline` in `run_inline.rs`. Uses the legacy `run_once()` path with ratatui-based streaming viewport. |
| **Legacy batch** | `--json`, `--quiet`, or non-TTY with engine=Legacy | Calls `run_once()` directly. Assembles prompt sections, dispatches to `ClaudeCliAgent`, runs gates in sequence, persists episode. |

**What actually works**:
- The V2 path is the default and functional. It constructs a `ModelCallService`, assembles prompts via `PromptAssemblyService`, dispatches through `create_agent_for_model()`, runs configured gates, and records feedback via `FeedbackService`.
- The legacy path (`run_once`) is behind `#[cfg(feature = "legacy-orchestrate")]` guards in several places, suggesting it is being phased out.
- Gates configured in `roko.toml` (`[[gates]]`) are passed through as `enabled_gates` and `shell_gates`.

**Gaps**:
- `StateHub` sharing between serve and run is broken: the `external_hub` variable in `cmd_run` is always `None` (line 271 of `util.rs`: `let external_hub: Option<&roko_cli::state_hub::StateHub> = None;`). A TODO comment explains the two crates define distinct `StateHub` types via `#[path]` includes.
- The `--provider` flag uses `unsafe { std::env::set_var() }` to inject the override. This is fragile and unsafe in multi-threaded contexts.
- `--share` writes a JSON transcript to `.roko/shared/{token}.json` but the serve URL (`http://localhost:6677/runs/{token}`) is only accessible if `--serve` is also passed.

### 1.2 `roko plan run <dir>`

**Claim**: "Execute plans (the main orchestration loop)."

**Actual flow**:
- Validates plan TOML files via `validate_before_run`.
- Loads plans via `roko_cli::runner::plan_loader::load_plans`.
- Optionally initializes a git repo if none exists (lines 276-301 of `plan.rs`).
- Creates a `ParallelExecutor` from `roko-orchestrator` with max 4 concurrent plans, 1 concurrent task.
- Enters the event loop in `runner/event_loop.rs::run()`.
- The event loop drives the executor via `tokio::select!` over agent events, gate completions, executor ticks, periodic flushes, and cancellation.
- Each task dispatches an agent via Claude CLI subprocess, captures streaming output, runs gates, records episodes.

**What actually works**:
- Plan loading, TOML parsing, dependency ordering, resume validation, and agent dispatch are all functional.
- Streaming agent output with `--output-format stream-json` parsing is wired.
- Gate dispatch (compile, clippy, test, shell) runs after agent completion.
- State persistence to `.roko/state/executor.json` enables `--resume`.
- Episode recording, efficiency events, and feedback facade sinks are all wired.
- Budget guardrails (`max_plan_usd`, `max_turn_usd`) are configured from roko.toml.
- Approval mode launches a TUI for interactive task approval.

**Gaps**:
- `max_concurrent_tasks` is hardcoded to 1 (line 115 of `event_loop.rs`). The executor supports parallelism but the runner doesn't expose it.
- The plan runner always passes `dangerously_skip_permissions: true` (line 394 of `plan.rs`). Safety contracts are not enforced in plan execution.
- Extension chain initialization logs errors but proceeds regardless -- extensions are effectively advisory.

### 1.3 `roko chat`

**Claim**: Interactive chat REPL with agents.

**Actual flow**:
- `chat_inline.rs` provides a ratatui-based inline chat with streaming tokens, fuzzy slash-command completion, cost tracking, and multi-line input.
- `chat_session.rs` owns `ChatAgentSession` which delegates to `ClaudeCliAgent` for CLI providers or planned API providers.
- Slash commands (`/model`, `/effort`, `/status`, `/diff`, `/plan`, etc.) are handled locally without agent dispatch.

**What actually works**:
- Claude CLI streaming chat with `--output-format stream-json` parsing is fully wired.
- Session resume via `--resume` passes the session ID to Claude's `--resume` flag.
- Tool call tracking, usage statistics, and cost metering are functional.
- History management with FIFO trimming (40 turns, 64K chars) works.

**Gaps**:
- API provider chat is explicitly unimplemented. `send_turn_api()` in `chat_session.rs` contains `todo!()` markers for the HTTP dispatch step. The method constructs the request and manages history but never sends it.
- The `SessionError::ApiProviderNotImplemented` variant is returned when a non-CLI model is selected.
- The chat does not use `WorkflowEngine` or gates. It's pure back-and-forth with Claude CLI.

### 1.4 `roko prd` commands

**Claim**: Manage product requirements documents (idea, draft, publish, plan).

**Actual flow** (`prd draft new <title>`):
1. Generates slug from title.
2. Extracts feature keywords for repo context lookup.
3. Writes a scaffold markdown file to `.roko/prd/drafts/{slug}.md`.
4. Builds a system prompt via `prd_agent_prompt()`.
5. Builds repository context via `build_repo_context()` with feature keywords.
6. Dispatches to Claude agent via `run_agent_capture_silent()`.
7. Detects whether agent wrote the file directly (mtime check) or returned text.
8. Runs post-generation validation: `check_grounding_section()` and `validate_prd_grounding()`.
9. Persists context sidecar and validation sidecar as JSON files.

**What actually works**:
- The full lifecycle (idea -> draft -> promote -> plan) is wired end-to-end.
- Repository grounding validation catches false negatives (claims "no existing crates" when workspace has crates) and duplicate crate proposals.
- Context sidecar persistence provides audit trail for generation inputs.
- `prd plan <slug>` generates a `tasks.toml` from the PRD using an agent.

**Gaps**:
- `prd consolidate` is declared in the CLI but the handler is minimal.
- The auto-plan trigger (`prd.auto_plan`) is wired in `roko-serve` but not in CLI-only mode.

### 1.5 `roko serve`

**Claim**: Start the HTTP API server (~85 routes).

**Actual flow**:
- `ServerBuilder::start_background()` builds the full `AppState`, starts background tasks, and binds the Axum router.
- Background services started: dispatch loop, config watcher, PRD publish subscriber, feedback loop, state hub bridge, state snapshot saver, job runner, cold archival timer, workspace GC.
- JWKS cache primed if Privy auth is configured.
- Chain watcher spawned if `chain.rpc_url` is configured.

**What actually works**:
- The server starts, binds, and serves routes. The `routes/` directory contains handlers for agents, aggregator, bench, config, connectors, deployments, feeds, gateway, integrations, jobs, learning, middleware, plans, prds, providers, research, secrets, shared runs, status, templates, and workflows.
- SSE and WebSocket endpoints exist for real-time updates.
- Terminal PTY routes are gated behind `--enable-terminal`.
- Deployment persistence (Railway, Fly, Docker) loads from disk on startup.

**Gaps**:
- `roko_serve::state_hub_compat` includes `roko-core/src/state_hub.rs` via `#[path]` include. This creates two copies of the `StateHub` type in the process, preventing zero-cost sharing between serve and CLI.
- The relay registration fires but the relay infrastructure is partial.
- OpenAPI spec generation (`openapi.rs`) is present but its accuracy relative to actual routes is unverified.

### 1.6 `roko acp`

**Claim**: Start ACP (Agent Client Protocol) server for editor integration.

**Actual flow**:
- ACP sessions are managed via `AcpSession` in `session.rs`.
- Three modes: code, plan, research -- each with distinct system prompts.
- Pipeline runner in `runner.rs` bridges to `WorkflowEngine` or a direct agent dispatch.
- Conversation history is maintained with trimming (40 turns, 64K chars).
- Config options (model, effort, temperament, gates, workflow, review strictness) are exposed to clients.

**What actually works**:
- Session lifecycle (create, load, list, cancel) is implemented.
- The `run_with_workflow_engine` function in `runner.rs` creates `ServiceFactory::build`, constructs a `WorkflowEngine`, and runs the full pipeline with event bridging to ACP protocol.
- Gate forensics (`analyze_gate_failure`, `classify_gate_error`) provide structured failure analysis with episode similarity matching.
- File change detection via `git diff --name-status HEAD~1 HEAD` works.

**Gaps**:
- The ACP server communicates via stdout (protocol channel) and logs to a file. Integration with editors beyond the protocol layer is unclear.
- `review_strictness` is configurable but the review phase in WorkflowEngine may not have graduated beyond basic LLM review.

### 1.7 Other Commands

| Command | Status | Notes |
|---|---|---|
| `roko init` | **Works** | Creates `.roko/` layout, `roko.toml`, detects project domain, seeds demo data with `--demo`. |
| `roko status` | **Works** | Reads signals, episodes, C-Factor. `--surfaces` prints surface inventory. |
| `roko doctor` | **Works** | Checks workspace bootstrap state, probes serve URL. |
| `roko dashboard` | **Works** | ratatui TUI with F1-F7 tabs, connected to StateHub. |
| `roko up` | **Works** | Starts serve + configured agents from roko.toml. |
| `roko learn` | **Works** | Subcommands for router, experiments, efficiency, episodes, tune. Reads from `.roko/learn/`. |
| `roko knowledge` | **Works** | Wraps KnowledgeStore: query, stats, gc, backup/restore, sync, dream, custody, archive. |
| `roko research` | **Works** | Uses Perplexity API for search/topic research. `enhance-prd/plan/tasks` are agent-assisted. |
| `roko explain` | **Works** | Static concept explainer with 3 depth levels. |
| `roko config` | **Works** | Full config management: init wizard, show, edit, providers, models, subscriptions, plugins, secrets, MCP. |
| `roko job` | **Works** | Marketplace jobs: create, list, match, show, execute, cancel. |
| `roko bench` | **Works** | SWE-bench proxy and comparative benchmarks. |
| `roko deploy` | **Partial** | Railway, Fly, Docker scaffolds exist. Railway uses GraphQL API. |
| `roko replay` | **Works** | Walks signal DAG by content hash. |
| `roko history` | **Works** | Lists/shows past chat sessions. |
| `roko inject` | **Partial** | Signal injection into running sessions via daemon socket. |
| `roko vision-loop` | **Built** | Iterative vision-guided UI refinement. Needs vision model configured. |

---

## 2. Orchestration: End-to-End Execution Flow

### 2.1 WorkflowEngine (V2 -- Default Path)

The actual execution flow for `roko run`:

```
cmd_run()
  -> resolve_workflow_model_selection()  // reads roko.toml, merges global/project/env/CLI
  -> build_workflow_effect_services()    // creates ServiceFactory::build -> ServiceBundle
  -> WorkflowEngine::new(services)
  -> engine.run(config)
       -> PipelineStateV2::new(workflow, prompt)
       -> EffectDriver::new(services, run_id, workdir)
       -> loop:
            PipelineStateV2::step(input)
              -> match output:
                   PipelineOutput::SpawnAgent { role, prompt, context }
                     -> EffectDriver::spawn_agent(role, prompt, context)
                        -> PromptAssembler::assemble(spec)
                        -> ModelCaller::call(request)
                        -> feedback recording
                     -> feed PipelineInput::AgentCompleted/Failed back
                   PipelineOutput::RunGates { ... }
                     -> EffectDriver::run_gates(...)
                        -> GateRunner::run_gate(config)
                     -> feed PipelineInput::GatesCompleted/Failed back
                   PipelineOutput::Commit { ... }
                     -> EffectDriver::commit(...)
                     -> feed PipelineInput::CommitCompleted back
                   PipelineOutput::Done { outcome }
                     -> record feedback, persist affect
                     -> return WorkflowRunReport
```

### 2.2 Plan Runner V2 (for `plan run`)

```
runner::event_loop::run(plans, config, state_hub, cancel)
  -> ParallelExecutor::new(exec_config)
  -> resume validation (task fingerprint matching)
  -> load_executor() (resume from snapshot if valid)
  -> event loop:
       tokio::select! {
         executor tick => get next ExecutorAction
           -> ExecutorAction::DispatchTask { plan_id, task_id }
              -> build system prompt (with enrichment, playbooks, knowledge)
              -> spawn Claude CLI subprocess with streaming output
              -> parse stream-json lines for tool calls, usage, session ID
           -> ExecutorAction::RunGate { ... }
              -> compile/clippy/test/shell gates
           -> ExecutorAction::Complete { ... }
              -> persist episode, efficiency event, cost record
         agent event => handle_agent_event()
         gate completion => handle gate result
         flush interval => persist state to disk
         cancel signal => graceful shutdown
       }
```

### 2.3 Legacy `run_once()` Path

Behind `#[cfg(feature = "legacy-orchestrate")]`. The massive `orchestrate.rs` (~21K+ lines) contains the original implementation with:
- Full attention bidder system (Neuro/Task/Research/Oracles)
- VCG auction for context allocation
- C-Factor computation
- Conductor circuit breaker and stuck detection
- Daimon affect modulation
- Pheromone-based coordination
- HDC fingerprinting per episode
- Worktree management per task
- Acceptance gate pipeline (7 rungs)
- Chain witness integration
- Model experiment store
- Error pattern store with failure pattern matching

Most of this sophistication is NOT present in the V2 paths. The V2 WorkflowEngine and plan runner are drastically simpler.

---

## 3. Agent Dispatch: How Agents are Actually Spawned

### 3.1 ClaudeCliAgent

The primary agent implementation. Built in `claude_cli_agent.rs`:

```rust
fn build_command(&self) -> Command {
    cmd.arg("--print")
       .arg("--verbose")
       .arg("--output-format").arg("stream-json")
       .arg("--model").arg(&self.model)
       .arg("--effort").arg(&self.effort)
       .arg("--settings").arg(&self.settings_json);
    // + --dangerously-skip-permissions (always true in plan mode)
    // + --max-turns (from OperatingFrequency)
    // + --fallback-model (default: claude-haiku-4-5)
    // + --append-system-prompt (from PromptAssemblyService)
    // + --tools (tool allowlist)
    // + --mcp-config (auto-discovered or explicit)
    // + --resume (session ID for multi-turn)
}
```

**Safety hooks** built into `--settings`:
- Blocks `git checkout`, `git switch`, `git branch -m`, `git push`
- Blocks `rm -rf`, `rm -fr`, `rm -r`

**Environment variables set**:
- `CARGO_INCREMENTAL=0` (faster CI-style builds)
- `CARGO_BUILD_JOBS=2` (limit parallelism)
- `CLAUDECODE` removed (prevents nested session detection)

### 3.2 ModelCallService (V2 Path)

Used by `WorkflowEngine` via `ServiceFactory::build()`:

```
ServiceFactory::build(config)
  -> resolve model slug from key
  -> create CascadeRouter (load or new)
  -> create KnowledgeStore
  -> create FeedbackService (episodes, cascade router)
  -> create ModelCallService
       .with_config(workspace_config)
       .with_feedback_sink(feedback)
       .with_gateway_event_writer()
       .with_event_consumer(JsonlLogger)
       .with_knowledge_store(query function)
       .with_cascade_router()
       .with_model_router(routing function)
       .with_run_id()
  -> create PromptAssemblyService
       .with_knowledge_store()
       .with_episodes()
       .with_playbooks()
       .with_token_budget()
       .with_tool_instructions()
       .with_section_effectiveness()
  -> create GateService
  -> optionally create DaimonPolicy (affect)
```

The `ModelCallService` wraps `create_agent_for_model()` which resolves the provider kind and constructs the appropriate agent (Claude CLI, Claude API, OpenAI-compat, Ollama, Gemini, Perplexity, Cerebras, etc.).

### 3.3 Context Agents Receive

In the plan runner (V2), agents get:
1. **System prompt** from `PromptAssemblyService` -- includes role template, project conventions, tool instructions, knowledge context, episode history, playbook guidance, section effectiveness weights.
2. **User prompt** -- task description with dependencies, file paths, acceptance criteria from tasks.toml.
3. **Enrichment context** (optional) -- code context, anti-patterns, strategy fragments.
4. **MCP config** -- auto-discovered or explicit path.
5. **Tool allowlist** -- configured per role/task.

In the WorkflowEngine, agents get:
1. **System prompt** from `PromptAssemblyService::assemble()`.
2. **User prompt** -- the raw prompt string plus optional context.
3. **No enrichment** -- the V2 engine does not have the enrichment pipeline from orchestrate.rs.

---

## 4. Gates: What Actually Runs

### 4.1 Gate Types Available

| Gate | Implementation | What it does |
|---|---|---|
| `compile` | `CompileGate` in roko-gate | Runs `cargo check --workspace` (or configured build command) |
| `clippy` | `ClippyGate` in roko-gate | Runs `cargo clippy --workspace --no-deps -- -D warnings` |
| `test` | `TestGate` in roko-gate | Runs `cargo test --workspace` |
| `shell` | `ShellGate` in roko-gate | Runs arbitrary shell command from `[[gates]]` config |

### 4.2 What's Configured vs. What Runs

Gates are declared in `roko.toml`:
```toml
[[gates]]
type = "compile"

[[gates]]
type = "clippy"

[[gates]]
type = "test"
```

In the **WorkflowEngine** path, gates are passed as `enabled_gates: Vec<String>` and `shell_gates: Vec<ShellGateCommand>`. The `GateService` runs them in order.

In the **plan runner**, the `max_gate_rung` config controls how far the gate pipeline goes:
- Rung 0 = compile only
- Rung 1 = compile + clippy
- Rung 2 = compile + clippy + test

The `gates.skip_tests` and `gates.clippy_enabled` config knobs control this.

### 4.3 What's Skipped

The full 7-rung gate pipeline from `orchestrate.rs` is NOT used in V2 paths:
- Rung 3: Symbol manifest verification (not run)
- Rung 4: Generated test gate (not run)
- Rung 5: LLM judge gate (not run)
- Rung 6: Search oracle (not run)

These only existed in the legacy `orchestrate.rs` path. The V2 `GateService` is a simple sequential executor.

### 4.4 Gate Failure Handling

In the plan runner, gate failures trigger:
1. Error classification (`classify_gate_error` in `runner.rs`): compile error, test failure, clippy warning, runtime panic, unknown.
2. Root cause extraction.
3. Episode-based similarity search for past failures.
4. Feedback construction for the next agent retry attempt.

In the WorkflowEngine, gate failures feed `PipelineInput::GatesFailed` back to the state machine, which may trigger autofix iterations (up to `max_autofix_attempts`).

In the legacy path, gate failures could trigger `build_gate_failure_plan_revision` for full replanning. This is NOT available in V2.

---

## 5. Learning: What Actually Persists and What's Read Back

### 5.1 What Gets Written

| Artifact | Path | Written by | Format |
|---|---|---|---|
| Episodes | `.roko/episodes.jsonl` | EpisodeSink / EpisodeLogger | JSONL with task_id, model, success, usage, gate verdicts, reflection |
| Efficiency events | `.roko/learn/efficiency.jsonl` | FeedbackFacade | JSONL with agent metrics |
| Cascade router state | `.roko/learn/cascade-router.json` | RoutingObservationSink | JSON with model performance stats |
| Knowledge candidates | `.roko/learn/knowledge_candidates.jsonl` | KnowledgeIngestionSink | JSONL |
| Conductor observations | `.roko/conductor/observations.jsonl` | ConductorObservationSink | JSONL |
| Dream triggers | `.roko/learn/dream_triggers.jsonl` | DreamTriggerSink | JSONL |
| Routing log | `.roko/learn/routing.jsonl` | RoutingLogger | JSONL with routing decisions |
| Section effects | `.roko/learn/section-effects.json` | SectionEffectivenessRegistry | JSON |
| Playbooks | `.roko/learn/playbooks/` | PlaybookStore | Individual files |
| Cost records | `.roko/learn/costs.jsonl` | CostsLog | JSONL |
| Latency stats | `.roko/learn/latency-stats.json` | LatencyRegistry | JSON |
| C-Factor history | `.roko/learn/c-factor.jsonl` | cfactor | JSONL |
| Daimon state | `.roko/daimon/affect.json` | DaimonPolicy | JSON |
| Error patterns | `.roko/learn/discovered-patterns.json` | ErrorPatternStore | JSON |
| Model experiments | `.roko/learn/model-experiments.json` | ModelExperimentStore | JSON |
| Executor state | `.roko/state/executor.json` | persist module | JSON |
| Run state | `.roko/state/run-state.json` | persist module | JSON with task fingerprints |
| Gateway events | `.roko/events/gateway.jsonl` | GatewayEventWriter | JSONL |
| Runtime events | `.roko/events/runtime.jsonl` | JsonlLogger | JSONL |
| Custody log | `.roko/custody.jsonl` | CustodyLogger | JSONL |

### 5.2 What's Read Back

| Data | Read by | When | Effect |
|---|---|---|---|
| Cascade router | CascadeRouter | Plan start | Model selection uses historical success rates |
| Episodes | PromptAssemblyService, error similarity | Agent dispatch | Prior failures inform system prompt |
| Playbooks | PlaybookStore | Agent dispatch | Matching playbooks injected into prompt |
| Section effects | SectionEffectivenessRegistry | Prompt assembly | Sections weighted by historical lift |
| Knowledge store | KnowledgeStore | Prompt assembly, routing | Relevant knowledge entries injected |
| Daimon state | DaimonPolicy | Agent dispatch (V2 engine) | Affect modulates temperature, turn limits, exploration |
| C-Factor | CalibrationTracker | Predictive routing (orchestrate.rs) | Calibration and prediction accuracy |
| Error patterns | ErrorPatternStore | Plan runner retry | Pattern matching for failure recovery |
| Executor snapshot | ParallelExecutor | Resume | Task completion state restored |

### 5.3 What's Written But Never Read

| Data | Notes |
|---|---|
| Conductor observations | Written by `ConductorObservationSink` but no consumer reads them back. |
| Dream triggers | Written by `DreamTriggerSink` but the dream cycle has no cron/trigger in the runtime. |
| Knowledge candidates | Written but ingestion into `KnowledgeStore` is not automatic. |
| Gateway events | Written for audit but not consumed by any learning feedback loop. |

---

## 6. ACP/MCP: What's Wired vs. Stubbed

### 6.1 ACP (Agent Client Protocol)

**Wired**:
- Session management (create, load, list, cancel)
- Prompt dispatch through WorkflowEngine with event bridging
- Multi-turn conversation history
- Config options (model, effort, temperament, gates, workflow)
- Mode switching (code, plan, research) with distinct system prompts
- Slash commands for configuration and workflow control
- Cancellation via cooperative token

**Partially wired**:
- `run_workflow_pipeline` (non-WorkflowEngine path) exists alongside `run_with_workflow_engine`
- Provenance card emission
- The knowledge prepend context system

### 6.2 MCP (Model Context Protocol)

**Wired**:
- Auto-discovery of MCP config: checks `agent.mcp_config` in roko.toml, then scans for `.mcp.json` or `mcp.json` in project and home directories.
- `--mcp-config` and `--strict-mcp-config` flags passed to Claude CLI.
- `roko-mcp-code` crate provides code intelligence MCP server.
- `config mcp` subcommands (list, add, remove, test) manage MCP server configs.

**Stubbed or partial**:
- `roko-mcp-github`, `roko-mcp-slack`, `roko-mcp-scripts`, `roko-mcp-stdio` exist as crates but integration completeness varies.
- MCP config passthrough works for Claude CLI but non-CLI providers don't use MCP.

---

## 7. Pain Points: Where UX Breaks Down

### 7.1 Two Execution Engines, Unclear Which to Use

The codebase has three execution paths: V2 WorkflowEngine (default for `run`), plan runner V2 (for `plan run`), and legacy `orchestrate.rs` (behind feature flag). Each has different capabilities:
- WorkflowEngine: simple, clean, but lacks enrichment, worktrees, and advanced gates.
- Plan runner V2: streaming, resume, gates, but uses Claude CLI directly rather than ModelCallService.
- Legacy orchestrate.rs: 21K+ lines, has everything, but is being deprecated.

Users don't know which path they're on or what they lose.

### 7.2 StateHub Type Duplication

`roko-serve` and `roko-cli` both include `state_hub.rs` via `#[path]` includes, creating two incompatible `StateHub` types. When `--serve` is passed to `roko run`, the serve StateHub and the CLI StateHub cannot share state. DashboardEvents from the run don't flow to the HTTP SSE/WebSocket endpoints. The TODO at line 266-270 of `util.rs` documents this.

### 7.3 API Provider Chat Not Implemented

`ChatAgentSession::send_turn_api()` has `todo!()` at the HTTP dispatch step. Users who configure non-CLI models (Anthropic API, OpenAI, Ollama) and try `roko chat` get an explicit error. The workaround is to use `roko run` which does support API providers through `ModelCallService`.

### 7.4 `dangerously_skip_permissions` Always On

Every plan execution path sets `dangerously_skip_permissions: true`. The safety contracts (`AgentContract`) are loaded but fall back to permissive defaults when YAML is missing. In practice, agents always run with full permissions.

### 7.5 Single-Task Execution in Plan Runner

Despite having a `ParallelExecutor` that supports concurrent tasks, the plan runner hardcodes `max_concurrent_tasks: 1`. Plans execute tasks sequentially even when the DAG allows parallelism.

### 7.6 Enrichment Gap Between Legacy and V2

The legacy `orchestrate.rs` has sophisticated enrichment: code context extraction, anti-pattern querying, strategy fragments, knowledge routing, C-Factor scoring, attention bidding, VCG allocation. None of this runs in the V2 WorkflowEngine path. The PromptAssemblyService does inject knowledge and playbooks, but the depth is far less.

### 7.7 Dream Cycle Has No Trigger

The dream consolidation cycle is built (`roko-dreams` crate, `DreamRunner`, `DreamLoopConfig`) and can be run manually via `roko knowledge dream run`. But there is no cron/timer/trigger that runs it automatically. The `DreamTriggerSink` writes events that nothing reads.

### 7.8 Cold Archival Timer Exists But is Advisory

`start_cold_archival_timer` is called in serve startup but the archival depends on workspace signal volume. For small workspaces, it never triggers.

---

## 8. Recommendations: Top 20 Fixes by Impact

### Tier 1: Critical Path (do these first)

1. **Unify StateHub types**. Extract `StateHub` into `roko-core` as a first-class re-export. This unblocks serve+run integration, SSE streaming during `roko run --serve`, and TUI-serve zero-copy reads. Impact: enables the entire real-time observability story.

2. **Wire API provider chat**. Complete `send_turn_api()` in `chat_session.rs`. The request construction and history management are done -- only the HTTP POST call needs implementation. Impact: makes `roko chat` work with all configured providers, not just Claude CLI.

3. **Enable parallel task execution in plan runner**. Expose `max_concurrent_tasks` from config or auto-derive from DAG width. Impact: 2-4x speedup for plans with independent tasks.

4. **Port enrichment to V2 engine**. The most impactful enrichment steps from orchestrate.rs -- code context, anti-patterns, playbook injection -- should be available in `PromptAssemblyService`. Some of this is already there (playbooks, knowledge), but code context extraction and strategy fragments are not. Impact: V2 agent quality approaches legacy.

5. **Wire dream consolidation trigger**. Add a timer or post-run hook that calls `DreamRunner::run_cycle()` when enough episodes accumulate. Impact: knowledge consolidation actually happens.

### Tier 2: Quality of Life

6. **Remove `unsafe set_var` for --provider**. Use config struct propagation instead of environment variable mutation. Impact: correctness in multi-threaded scenarios.

7. **Make safety contracts non-permissive by default**. Generate default contract YAML during `roko init`. Remove the always-true `dangerously_skip_permissions` in plan mode. Impact: agents can't accidentally rm -rf outside the safety hooks.

8. **Add `--parallel` flag to `plan run`**. Let users opt into parallel execution with a CLI flag. Impact: power users get speedup without changing defaults.

9. **Wire knowledge candidate ingestion**. Currently candidates are written to JSONL but never ingested into the KnowledgeStore. Add a post-run ingestion step. Impact: knowledge store grows from execution experience.

10. **Bridge conductor observations to the feedback loop**. Conductor observations are written but never read. Wire them into the next run's stuck detection or meta-cognition. Impact: self-monitoring improves over time.

### Tier 3: Architecture Debt

11. **Consolidate execution paths**. The three paths (V2 engine, plan runner, legacy) should converge. The plan runner should use WorkflowEngine per-task rather than its own agent dispatch. Impact: reduces code surface, ensures consistent behavior.

12. **Extract `orchestrate.rs` modules**. The 21K+ line file should be broken into focused modules: enrichment, gate pipeline, worktree management, conductor integration, feedback recording. Impact: maintainability, testability.

13. **Standardize gate pipeline**. The V2 `GateService` and the legacy rung-based `GatePipeline` should share a common interface. Impact: advanced gates (LLM judge, symbol manifest) available everywhere.

14. **Wire section effectiveness feedback**. The `SectionEffectivenessRegistry` tracks lift per prompt section. This data should flow back into prompt assembly weights automatically. It is partially wired in the ServiceFactory path but not in plan runner.

15. **Deprecate `#[cfg(feature = "legacy-orchestrate")]`**. Mark the feature flag as deprecated and add a migration path. Impact: clarity about which code is live.

### Tier 4: Observability and Polish

16. **Fix `--share` without `--serve`**. Currently `--share` writes a local JSON file with a `localhost:6677` URL that's inaccessible without serve running. Either auto-start serve or generate a self-contained HTML artifact. Impact: sharing actually works.

17. **Add episode-based regression detection to V2**. The legacy path has `detect_cfactor_regression` and predictive calibration. Wire at least basic regression detection (gate pass rate trending down) into the V2 feedback path. Impact: early warning on quality degradation.

18. **Wire gateway events into cost dashboard**. Gateway events contain per-call cost and latency. Surface these in `roko learn efficiency` and the TUI cost tab. Impact: cost visibility.

19. **Add `roko run --dry-run`**. Show what would happen (model selection, gates, prompt assembly) without dispatching to an agent. Impact: debugging and verification.

20. **Persist WorkflowEngine checkpoint for resume**. The plan runner has resume via executor.json. The single-prompt WorkflowEngine path does not checkpoint -- if interrupted, work is lost. Add checkpoint at each phase transition. Impact: resilience for long-running single-prompt workflows.

---

## Appendix: Key Source File Index

| File | Lines | Role |
|---|---|---|
| `crates/roko-cli/src/main.rs` | ~3000 | CLI entry, all subcommand definitions |
| `crates/roko-cli/src/orchestrate.rs` | ~21000+ | Legacy orchestration loop (behind feature flag) |
| `crates/roko-cli/src/run.rs` | ~1100 | V2 WorkflowEngine entry point for `roko run` |
| `crates/roko-cli/src/chat_session.rs` | ~800 | Chat agent session, CLI and API turn dispatch |
| `crates/roko-cli/src/chat_inline.rs` | ~1500+ | ratatui inline chat UX |
| `crates/roko-cli/src/commands/plan.rs` | ~500+ | Plan list/show/create/run/validate handlers |
| `crates/roko-cli/src/commands/prd.rs` | ~500+ | PRD idea/draft/plan/consolidate handlers |
| `crates/roko-cli/src/commands/util.rs` | ~450 | cmd_run, cmd_init, cmd_oneshot routing |
| `crates/roko-cli/src/runner/event_loop.rs` | ~600+ | Plan runner V2 event loop |
| `crates/roko-cli/src/runner/mod.rs` | ~40 | Runner module index |
| `crates/roko-acp/src/runner.rs` | ~500+ | ACP pipeline runner (WorkflowEngine bridge) |
| `crates/roko-acp/src/session.rs` | ~500+ | ACP session state management |
| `crates/roko-agent/src/claude_cli_agent.rs` | ~700+ | Claude CLI subprocess wrapper |
| `crates/roko-serve/src/lib.rs` | ~600+ | HTTP server builder and startup |
| `crates/roko-runtime/src/workflow_engine.rs` | ~400+ | WorkflowEngine top-level facade |
| `crates/roko-runtime/src/effect_driver.rs` | ~400+ | EffectDriver for pipeline actions |
| `crates/roko-orchestrator/src/service_factory.rs` | ~250+ | ServiceFactory for shared service construction |

---

## Appendix: Data Flow Summary

```
User prompt
  |
  v
Model Selection (roko.toml + CLI + env + CascadeRouter)
  |
  v
Prompt Assembly (system prompt + knowledge + playbooks + tool instructions)
  |
  v
Agent Dispatch (Claude CLI subprocess / API provider via ModelCallService)
  |
  v
Streaming Output Capture (stream-json parsing)
  |
  v
Gate Execution (compile -> clippy -> test -> shell)
  |
  v
Feedback Recording (episodes + routing + efficiency + costs + knowledge)
  |
  v
State Persistence (executor snapshot for resume)
  |
  v
Learning Updates (cascade router, section effects, playbooks, daimon)
```
