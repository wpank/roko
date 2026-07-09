# roko batch 3: orchestration loop, mirage dashboard, cloud deployment, learning feedback, mori parity

**Branch:** `roko-batch3-wiring` -> `main`
**432 files changed** | **+75,371** | **-4,206** | **250 commits**

---

## What this PR does

This PR wires the remaining infrastructure that makes roko self-hosting. Four major systems land:

1. **Orchestration loop** -- The 4,011-line `orchestrate.rs` connects the plan-execute-gate-persist cycle end-to-end. Plans discovered on disk get executed by Claude CLI agents, validated by gate pipelines, and persisted with session snapshots. The loop supports resume, parallel execution, cost budgets, conductor signals, and a learning feedback path.

2. **Mirage-RS dashboard and API** -- A full interactive single-page dashboard with 35+ REST endpoints, 8 JSON-RPC methods, WebSocket streaming, a force-directed pheromone particle system, knowledge graph visualization, agent topology network, task lifecycle tracking, and a 20-agent simulation harness.

3. **Cloud deployment** -- `roko serve` exposes the entire CLI surface as an HTTP API (25+ endpoints), with Railway deployment backends, a cloud worker system, and Docker packaging.

4. **Learning and feedback** -- Efficiency events, cascade router persistence, prompt A/B experiments, adaptive gate thresholds, and a runtime feedback system that feeds failed gates back into plan generation.

---

## Commits

| SHA | Summary |
|-----|---------|
| `393c513` | fix(mirage-rs): router ordering bug + heartbeat block number |
| `eeec158` | parity(3C.11): Add roko event-sources list CLI command |
| `296c9a4` | parity(3C.10): Wire CronEventSource and FileWatchEventSource into dispatch loop |
| `f00e2ba` | parity(3C.09): Add watcher.paths config sections |
| `662b65f` | parity(3C.08): Wire path filtering with include/exclude globs |
| `2bf5f9c` | parity(3C.07): Wire debounce for file events |
| `b639238` | parity(3C.06): Implement FileWatchEventSource |
| `413da91` | parity(3C.05): Wire signal emission on cron fire |
| `d012a06` | parity(3C.04): Add scheduler.cron config sections |
| `741394c` | parity(3C.01): Implement CronEventSource |
| `b81d1c1` | parity(3B.06): Add subscription CLI commands |
| `010a14b` | parity(3B.05): Wire subscription CRUD API endpoints |
| `f67207e` | parity(3B.03): Implement subscription matching |
| `49cc5e8` | parity(3B.01): Define Subscription struct |
| `10d4bd5` | parity(3A.11): Define concrete experiments for templates |
| `20c0166` | parity(3A.10): Wire feedback metrics into experiment outcomes |
| `a053692` | parity(3A.09): Record experiment assignment in episode metadata |
| `28ed4e9` | parity(3A.08): Implement variant-based prompt modification |
| `a229d7a` | parity(3A.07): Wire ExperimentStore into dispatch loop |
| `c70bbdd` | parity(3A.06): Wire template validation |
| `f8b9405` | parity(3A.04): Wire template variable interpolation |
| `02c9889` | parity(3A.03): Wire template -> dispatch |
| `d59b9f5` | parity(3A.02): Implement template loader |
| `b02453a` | parity(3A.01): Define AgentTemplate TOML schema |
| `f867f3b` | parity(2D.38): Wire MCP config generation |
| `c45932e` | parity(2D.37): Wire MCP auto-stop |
| `ce09554` | parity(2D.36): Wire MCP health check |
| `13de6c0` | parity(2D.35): Wire MCP auto-start |
| `e8507a5` | parity(2D.34): Script security (no path traversal) |
| `49a0172` | parity(2D.33): Script discovery on startup |
| `d7e8583` | parity(2D.32): Script sandboxing (timeout, workdir, env allowlist) |
| `0a83c1d` | parity(2D.31): Tool: list_scripts |
| `6637588` | parity(2D.30): Tool: run_script |
| `8ce55fc` | parity(2D.28): Slack auth (SLACK_BOT_TOKEN) |
| `68190be` | parity(2D.27): Slack rate limiting |
| `c0467c4` | parity(2D.26): Tool: slack_dm |
| `8490691` | parity(2D.25): Tool: slack_lookup_user |
| `1582fe3` | parity(2D.24): Tool: slack_get_thread |
| `6aaa768` | parity(2D.23): Tool: slack_list_channels |
| `926905c` | parity(2D.22): Tool: slack_react |
| `4ae1583` | parity(2D.21): Tool: slack_reply |
| `30f4f7d` | parity(2D.20): Tool: slack_post_message |
| `a3b3e29` | parity(2D.19): Slack MCP stdio transport |
| `e222999` | parity(2D.17): GitHub auth (GITHUB_TOKEN) |
| `1d4b18e` | parity(2D.16): GitHub rate limiting |
| `45695cb` | parity(2D.15): Tool: github_search_code |
| `f98b6d2` | parity(2D.14): Tool: github_get_file |
| `534aa6c` | parity(2D.13): Tool: github_create_issue |
| `1ea1bab` | parity(2D.12): Tool: github_list_issues |
| `3eeb39f` | parity(2D.11): Tool: github_merge_pr |
| `ba705ea` | parity(2D.10): Tool: github_comment_pr |
| `6ffc6b3` | parity(2D.09): Tool: github_review_pr |
| `788a3ab` | parity(2D.08): Tool: github_create_pr |
| `3604695` | parity(2D.07): Tool: github_get_pr |
| `45db0d8` | parity(2D.06): Tool: github_list_prs |
| `7b64369` | parity(2D.05): Implement tools/call dispatcher |
| `0e56daf` | parity(2D.04): Implement tools/list handler |
| `2a38f8a` | parity(2D.03): Implement initialize handler |
| `81fa974` | parity(2D.02): Implement stdio JSON-RPC transport |
| `72f7695` | parity(2D.01): Create roko-mcp-github crate |
| `b9d4f44` | parity(2C.23): Record feedback signals |
| `ee74f4b` | parity(2C.22): Wire feedback into cascade router |
| `ecd821b` | parity(2C.21): Wire feedback into experiment metrics |
| `bb706e7` | parity(2C.20): Implement feedback polling schedule |
| `47a0732` | parity(2C.19): Implement collect_slack_feedback |
| `ff742c4` | parity(2C.18): Implement collect_github_feedback |
| `71d9b66` | parity(2C.17): Implement start_feedback_loop |
| `658b83a` | parity(2C.16): Feed episode into efficiency tracker |
| `8e5ea0e` | parity(2C.15): Feed episode into cascade router |
| `c8050ad` | parity(2C.14): Track ExternalActions during execution |
| `ce0b2c2` | parity(2C.13): Wire episode logging in dispatch_agent |
| `c05f328` | parity(2C.12): Define WebhookEpisodeMetadata |
| `489f453` | parity(2C.11): Add webhooks config section |
| `fbcf191` | parity(2C.10): Log webhook events to signals |
| `e5b4a65` | parity(2C.09): Wire dispatch_agent function |
| `75c7050` | parity(2C.07): Implement cooldown tracking |
| `ccb8a63` | parity(2C.06): Implement concurrency tracking |
| `d7ffcb4` | parity(2C.05): Wire SubscriptionRegistry |
| `5364921` | parity(2C.04): Implement DispatchLoop |
| `bb30871` | parity(2C.03): Add POST /webhooks/generic endpoint |
| `5a679aa` | parity(2C.02): Add POST /webhooks/slack endpoint |
| `9faf71b` | parity(2C.01): Add POST /webhooks/github endpoint |
| `cced500` | parity(2B.10): Add PluginBuilder fluent API |
| `dcde052` | parity(2B.09): Define PluginManifest |
| `b2757dd` | parity(2B.08): Define signal constants module |
| `2b659d2` | parity(2B.07): Define SignalSender type |
| `98d3b42` | parity(2B.06): Define EventSourceKind enum |
| `0e55f94` | parity(2B.05): Define FeedbackSignal struct |
| `5ea6085` | parity(2B.04): Define FeedbackCollector trait |
| `60474f1` | parity(2B.03): Define EventSource trait |
| `224de6d` | parity(2B.02): Add roko-plugin to workspace |
| `5d27d4e` | parity(2A.09): Ensure serve integration tests pass |
| `4db57ff` | parity(2A.08): Extract EventBus into own type |
| `074aead` | parity(2A.07): Export ServerBuilder in roko-serve |
| `81bb461` | parity(2A.06): Add roko-serve to workspace |
| `f308ca4` | parity(2A.05): Remove old serve directory |
| `1d09913` | parity(2A.04): Update serve subcommand imports |
| `e75f56a` | parity(2A.03): Move serve files to roko-serve crate |
| `fb50d9a` | parity(4D.07): Add config set-secret command |
| `d7822fb` | parity(4D.06): Add config check-secrets command |
| `04ae08c` | parity(4D.05): Document required env vars |
| `69cea65` | parity(4D.04): Wire secret masking in API responses |
| `0c7d827` | parity(4D.03): Wire secret masking in logs |
| `e6f3e3a` | parity(4D.02): Wire ${VAR} interpolation |
| `4550eef` | parity(4D.01): Wire .env file loading |
| `28dcd48` | parity(1J.10): Update rolling averages after task |
| `ea472b0` | parity(1J.09): Log context attribution decisions |
| `0fa47b9` | parity(1J.08): Include/exclude context by reference rates |
| `d354f9e` | parity(1J.07): Maintain rolling averages per context source |
| `b5ed404` | parity(1J.06): Persist cascade router after observations |
| `828e06f` | parity(1J.05): Track per-crate familiarity scores |
| `bd40b52` | parity(1J.04): Build context vector for model selection |
| `ab29004` | parity(1J.03): Observe cascade router on failure |
| `8b88956` | parity(1J.02): Observe cascade router on success |
| `426411e` | parity(1J.01): Verify CascadeRouter initialization |
| `81ed6df` | parity(1I.08): Inject playbook lookup into dispatch context |
| `9fd37d9` | parity(1I.07): Record playbook on task success |
| `16635cf` | parity(1I.06): Add PlaybookStore to PlanRunner |
| `435bb15` | parity(1I.05): Persist skill library to JSON |
| `31f5d06` | parity(1I.04): Query skill library before task dispatch |
| `88870c5` | parity(1I.03): Record skill failure |
| `cba1655` | parity(1I.02): Extract skill on task success |
| `a7b6a95` | parity(1I.01): Add SkillLibrary to PlanRunner |
| `8a8bee3` | parity(1H.18): Wire color theming |
| `22e5837` | parity(1H.17): Wire --page flag |
| `54fcfb9` | parity(1H.16): Wire --text flag |
| `aa21d05` | parity(1H.15): Wire roko dashboard to launch TUI |
| `bbd4a02` | parity(1H.14): Wire terminal setup/teardown |
| `fcf03c1` | parity(1H.13): Wire live refresh |
| `48a0c46` | parity(1H.12): Wire keyboard navigation |
| `a30c9ac` | parity(1H.11): Wire page 6 -- Signals |
| `1420de5` | parity(1H.10): Wire page 5 -- Learning |
| `62942b8` | parity(1H.09): Wire page 4 -- Gate Results |
| `50a0a32` | parity(1H.08): Wire page 3 -- Agent Activity |
| `73cb0bd` | parity(1H.07): Wire page 2 -- Plan Execution |
| `1894f34` | parity(1H.06): Wire page 1 -- Overview |
| `419f57f` | parity(1H.05): Implement TUI event loop |
| `a69e08e` | parity(1H.04): Define DashboardData struct |
| `aceab74` | parity(1H.03): Define App struct |
| `55f2299` | parity(1H.02): Create tui/ module structure |
| `ea8f51c` | parity(1H.01): Add ratatui + crossterm deps |
| `a05e524` | parity(1G.13): Add per-metric endpoints |
| `2aaa727` | parity(1G.12): Add metrics/summary endpoint |
| `99f3339` | parity(1G.11): Add GET /api/health |
| `07a5d39` | parity(1G.10): Wire API auth toggle |
| `ade0d05` | parity(1G.09): Ensure proper HTTP error codes |
| `3795585` | parity(1G.08): Wire adaptive-thresholds endpoint |
| `70ffdae` | parity(1G.07): Wire efficiency endpoint |
| `78f92e9` | parity(1G.06): Wire experiments endpoint |
| `d00a875` | parity(1G.05): Wire cascade endpoint |
| `2440119` | parity(1G.04): Wire gate history endpoint |
| `88cbcc6` | parity(1G.03): Wire gate summary endpoint |
| `b78d457` | parity(1G.02): Define ExecutionEvent enum |
| `152bcb9` | parity(1G.01): Wire execution progress -> WebSocket |
| `9673e29` | parity(1D.11): Replace eprintln/println with tracing |
| `28794c9` | parity(1D.10): Wire structured error context |
| `2cb3566` | parity(1D.09): Wire secret redaction in logs |
| `eff8892` | parity(1D.08): Wire timing recording |
| `7cfb5df` | parity(1D.07): Add cost summary |
| `005d796` | parity(1D.06): Wire ROKO_LOG env var |
| `6961e21` | parity(1D.05): Wire --log-format CLI flag |
| `750ae05` | parity(1D.04): Add propagating span fields |
| `486442d` | parity(1D.03): Add span hierarchy |
| `52a4f28` | parity(1D.02): Wire subscriber initialization |
| `6cffd86` | parity(1D.01): Add tracing-subscriber dependency |
| `5c5724c` | parity(1C.10): Add per-task mcp_servers field |
| `92d8426` | parity(1C.09): Add [tools] section to roko.toml |
| `45debb4` | parity(1C.08): Wire tool dedup |
| `60b83dc` | parity(1C.07): Wire tool discovery |
| `acf1698` | parity(1C.06): Wire MCP server lifecycle |
| `e5a468c` | parity(1C.05): Wire role profiles |
| `e4ab286` | parity(1C.04): Define role-based tool profiles |
| `930623c` | parity(1C.03): Wire --allowedTools passthrough |
| `a24d6b9` | parity(1C.02): Wire tool filtering |
| `3316733` | parity(1C.01): Add allowed/denied tools to task schema |
| `6bfe947` | parity(1B.15): Emit watcher alerts as Signals |
| `6b31298` | parity(1B.14): Wire iteration_loop and review_loop watchers |
| `1df354e` | parity(1B.13): Wire test_failure_budget watcher |
| `0cfe0ee` | parity(1B.12): Wire spec_drift watcher |
| `5a15d3c` | parity(1B.11): Wire ghost_turn watcher |
| `0235114` | parity(1B.10): Wire compile_fail_repeat watcher |
| `ddc284a` | parity(1B.09): Wire time_overrun watcher |
| `ec4d2b7` | parity(1B.08): Wire context_window_pressure watcher |
| `4d2ea51` | parity(1B.07): Wire stuck_pattern watcher |
| `81a7c86` | parity(1B.06): Wire cost_overrun watcher |
| `37a419c` | parity(1B.05): Wire diagnosis integration |
| `392ad7d` | parity(1B.04): Wire circuit breaker response |
| `de8ec95` | parity(1B.03): Wire WatcherRunner lifecycle |
| `a976394` | parity(1B.02): Create WatcherRunner |
| `0beef0c` | parity(1A.22): Cost data in episode log |
| `d8a2ddf` | parity(1A.21): Warn at budget threshold |
| `2bf2ebb` | parity(1A.20): Track cumulative plan cost |
| `df92cd5` | parity(1A.19): Check max_task_usd before dispatch |
| `5b117c0` | parity(1A.18): Record cost after agent dispatch |
| `d300a9e` | parity(1A.17): Set shared CARGO_TARGET_DIR |
| `ff9e473` | parity(1A.16): Clean stale git locks |
| `4e01633` | parity(1A.15): Wire parallel task dispatch |
| `68ee8cb` | parity(1A.14): Wire WorktreeManager |
| `e8ca0ab` | parity(1A.13): Add [executor] section to roko.toml |
| `f96e203` | parity(1A.12): Parse and use task TOML fields |
| `45cbe47` | parity(1A.11): Wire cross-plan dependency tracking |
| `e4a9623` | parity(1A.10): Wire --resume |
| `f003777` | parity(1A.09): Implement RegeneratingVerify phase |
| `830008c` | parity(1A.08): Implement AutoFixing phase |
| `70791ce` | parity(1A.07): Implement DocRevision phase |
| `d6b1fb8` | parity(1A.06): Implement Reviewing phase |
| `f8b2071` | parity(1A.05): Implement Verifying phase |
| `3dcaccd` | parity(1A.04): Implement Enriching phase |
| `824c2e1` | parity(1A.03): Wire ExecutorConfig from roko.toml |
| `49f6d77` | parity(1E.10): Log replan events to episodes |
| `8ecdd0f` | parity(1E.09): Wire --no-replan CLI flag |
| `581a35b` | parity(1E.08): Feed failure patterns to learning |
| `c8b7f5d` | parity(1E.07): Add replan_strategy to task schema |
| `984b009` | parity(1E.06): Add max_retries to task schema |
| `cd2ee6d` | parity(1E.05): Implement RegeneratePlan |
| `95a3f6b` | parity(1E.04): Implement RetryWithEscalation |
| `e02d9cd` | parity(1E.03): Implement Decompose strategy |
| `b75b59f` | parity(1E.02): Add replan strategy selection logic |
| `6f90d1e` | parity(1E.01): Extract ReplanStrategy enum |
| `7fbf927` | parity(1F.12): Regenerate old-format plans |
| `d4bbebc` | parity(1F.11): Flag old-format plans in list |
| `6d20741` | parity(1F.10): Wire plan regenerate command |
| `545c66d` | parity(1F.09): Wire plan validate command |
| `54a5570` | parity(1F.08): Wire plan template selection |
| `45191b1` | parity(1F.07): Wire signal emission on auto-plan |
| `ecb0b26` | parity(1F.06): Add --dry-run to prd plan |
| `898e6c8` | parity(1F.05): Add --auto-execute to prd draft promote |
| `b3e3056` | parity(1F.04): Wire plan quality heuristics |
| `de5b6af` | parity(1F.03): Wire plan structural validation |
| `a7b52ed` | parity(1F.02): Add auto_plan config field |
| `ad22b7a` | parity(1F.01): Wire on_prd_promote hook |
| `61f7aac` | wip: codex partial 1A attempt + run-parity.sh |
| `064e5e5` | add comprehensive PR description |
| `b15d55f` | mirage-rs: real fork info, task stats, cognitive traces, dashboard polish |
| `40b4e32` | fix agent registration: use string pubkey, not array |
| `8cc9b04` | add task system, 20-agent simulation, tokenomics dashboard, auto-WS |
| `21c10e2` | dashboard: fix jitter -- incremental DOM updates, canvas size caching, CSS containment |
| `633674f` | add interactive dashboard UI with modular ES modules and static file serving |
| `2567918` | mirage-rs HTTP API: validation, write endpoints, caching, heartbeat, tests, docs |
| `b0f8864` | issue #5: agent registry, HTTP/RPC/WS endpoints, dashboard data rendering, block timestamp fix |
| `4c246d3` | add roko serve cloud deployment + worker subcommand |
| `7a184e4` | wire parity sections 6-11: cross-plan deps, parallel limits, conductor signals, cost budget, learning loop, plan regeneration |
| `8779ff0` | wire learning plan 05 remaining items: efficiency events, cascade persistence, prompt experiments, adaptive thresholds |
| `4ba8c3a` | wire learning, MCP tools, and observability into orchestration loop |
| `9635bd4` | wire EpisodeLogger, ProcessSupervisor, and MCP config into orchestration loop |
| `78d1046` | add research and PRD agent flows |
| `99b0989` | commit remaining repo changes |
| `0e87748` | repair Claude wiring and finish CLI runtime integration |
| `bfd13e5` | gitignore: exclude CLAUDE.md and scripts/ until refined |
| `be35513` | roko batch 3: fix failing test, wire safety dispatch, orchestration loop, session persistence |

---

## Crates touched

| Crate | Files | What changed |
|-------|-------|-------------|
| mirage-rs | 59 | HTTP API, agent registry, task system, WS, dashboard UI |
| roko-agent | 48 | Tool dispatch, MCP lifecycle, safety layer |
| roko-core | 45 | Config schema, tool traits, metric types, signal kinds |
| roko-cli | 37 | Orchestrate.rs, TUI, prd hooks, plan commands, event sources CLI |
| roko-compose | 30 | System prompt builder, templates, enrichment |
| roko-serve | 30 | **New crate** -- HTTP API server, webhooks, dispatch loop, templates, subscriptions, feedback |
| roko-std | 29 | Role profiles, builtin tools, mock dispatcher |
| roko-orchestrator | 23 | Worktree manager, parallel executor |
| roko-learn | 21 | Skill library, playbooks, runtime feedback, costs |
| roko-gate | 19 | Adaptive thresholds, gate payload |
| roko-conductor | 19 | 10 watchers, circuit breaker, diagnosis |
| roko-fs | 11 | Layout, observability sinks |
| roko-golem | 8 | Chain witness, daimon, dreams (phase 2+ scaffolding) |
| roko-demo | 7 | Demo scenarios and agent clade definitions |
| roko-chain | 6 | Chain primitives |
| bardo-runtime | 5 | ProcessSupervisor, event bus, cancellation |
| roko-plugin | 3 | **New crate** -- Plugin system: EventSource, FeedbackCollector, PluginBuilder traits |
| roko-mcp-github | 2 | **New crate** -- GitHub MCP server: PRs, issues, reviews, search, rate limiting |
| roko-mcp-slack | 2 | **New crate** -- Slack MCP server: messages, threads, reactions, DMs, rate limiting |
| roko-mcp-scripts | 2 | **New crate** -- Script runner MCP server: sandboxed execution, discovery, path security |
| roko-mcp-stdio | 2 | **New crate** -- Shared stdio JSON-RPC transport for MCP servers |

---

## 1. Orchestration loop

**File:** `crates/roko-cli/src/orchestrate.rs` (4,011 lines, new)

The core self-hosting runtime. Connects the CLI to `roko-orchestrator`'s pure state machine (`ParallelExecutor`), dispatching its `ExecutorAction`s to real agents, gates, and git, then feeding results back as `ExecutorEvent`s.

### PlanRunner

The top-level struct that manages a full orchestration run:

- Discovers plans via `roko_orchestrator::discover_plans()`
- Builds a `ParallelExecutor` with dependency-ordered DAG
- Dispatches up to `MAX_PARALLEL_TASKS` (4) agents concurrently
- Tracks running agent processes via `bardo_runtime::ProcessSupervisor`
- Auto-saves executor state every `AUTOSAVE_INTERVAL` (5) actions to `.roko/state/executor.json`
- Supports `--resume` from a saved snapshot

### Agent dispatch

Each task spawns an isolated agent subprocess via `AgentRunConfig`:

- **Claude CLI agents** (`ClaudeCliAgent`) -- spawns `claude` with model selection, system prompt, MCP config passthrough, bare mode, effort level, tool allowlists, fallback model, environment variables, resume session, and `--dangerously-skip-permissions` flag
- **Exec agents** (`ExecAgent`) -- spawns arbitrary commands with timeout and env vars
- Both wrapped in `run_prepared_agent()` which requires no `PlanRunner` borrow, enabling parallel dispatch

### System prompt assembly

`RoleSystemPromptSpec` drives 6-layer prompt construction via `roko-compose::SystemPromptBuilder`:

- Layer 1: Role identity (implementer, reviewer, strategist, researcher, etc.)
- Layer 2: Task context (plan metadata, task description, dependencies)
- Layer 3: Codebase context (relevant files, symbols, recent changes)
- Layer 4: Constraints (budget, time, quality gates)
- Layer 5: Learning context (past episodes, efficiency data, experiment variants)
- Layer 6: Output format (expected deliverables, gate requirements)

### Gate pipeline

Per-task validation after agent completion:

- `CompileGate` -- `cargo build` passes
- `TestGate` -- `cargo test` passes
- `ClippyGate` -- `cargo clippy` clean
- Diff gate -- changes stay within expected scope
- Adaptive thresholds -- EMA-adjusted pass/fail thresholds per rung via `roko_gate::AdaptiveThresholds`
- Gate results recorded as `GateVerdict` in episode log

### Conductor integration

`roko_conductor::Conductor` monitors execution health:

- Circuit breaker -- halts execution after repeated failures
- Stuck detection -- identifies tasks that exceed expected duration
- Cost overrun watcher -- enforces per-plan and per-task cost limits
- Returns `ConductorDecision` (proceed, pause, abort) each cycle

### Learning and feedback

Wired into the execution loop:

- **EpisodeLogger** -- records agent turns + gate results to `.roko/episodes.jsonl` as `Episode` entries with `Usage` and `GateVerdict`
- **Efficiency events** -- per-turn `AgentEfficiencyEvent` written to `.roko/learn/efficiency.jsonl`
- **CascadeRouter** -- persists model routing decisions to `.roko/learn/cascade-router.json` for replay
- **Prompt experiments** -- `ExperimentStore` runs A/B tests across prompt variants, persisted to `.roko/learn/experiments.json`
- **Adaptive gate thresholds** -- EMA per rung saved to `.roko/learn/gate-thresholds.json`
- **Cost tracking** -- `CostRecord` entries logged per task
- **Runtime feedback** -- `LearningRuntime` collects `CompletedRunInput` data and produces `LearningUpdate`s

### Context attribution

`ContextAttributionTracker` monitors which context tiers and source types agents actually reference:

- Loads historical data from `.roko/context-attribution.jsonl`
- Tracks per-(tier, source_type) reference rates
- Demotes context sources with <10% reference rate
- Records new attribution events after each agent run

### Cross-plan dependencies and parallel limits

- Tasks can declare dependencies on tasks in other plans
- Configurable `MAX_PARALLEL_TASKS` limits concurrent agent count
- DAG respects cross-plan edges during scheduling

### Plan regeneration

When tasks fail repeatedly, the learning loop feeds failure data back into plan generation for automatic re-planning.

### Report types

- `PlanRunReport` -- per-plan results (plan_id, succeeded, agent_calls, gate_results)
- `OrchestrationReport` -- aggregate across all plans (total_agent_calls, total_gate_runs, all_succeeded)

---

## 2. PRD system

**Files:** `crates/roko-cli/src/prd.rs` (~440 lines), `crates/roko-cli/src/prd_prompt.rs` (~225 lines)

Full PRD lifecycle management. PRDs live in `.roko/prd/` with this layout:

```
.roko/prd/
  ideas.md              # quick captures
  drafts/               # work-in-progress PRDs
    <slug>.md
  published/            # finalized PRDs
    <slug>.md
```

### CLI subcommands

| Command | What it does |
|---------|-------------|
| `roko prd idea "<text>"` | Append a timestamped idea to `.roko/prd/ideas.md` |
| `roko prd list` | List all PRDs (drafts + published) with status, title, creation date |
| `roko prd status` | Coverage report: plans generated per PRD, tasks per plan, completion ratio |
| `roko prd draft new "<title>"` | Create a new PRD draft with agent-driven refinement |
| `roko prd draft promote` | Promote a draft to published status (moves file, updates frontmatter) |
| `roko prd plan <slug>` | Generate implementation plan + `tasks.toml` from a published PRD |
| `roko prd consolidate` | Merge duplicate/overlapping PRDs into a single document |

### PRD frontmatter

`PrdMeta` struct parsed from markdown frontmatter:

- `id` -- stable identifier (e.g. `prd-golem-memory`)
- `title` -- human-readable title
- `status` -- lifecycle status (`draft` or `published`)

### Prompt generation

`prd_prompt.rs` builds the system prompt for PRD-related agent interactions, including context about existing PRDs, plan coverage, and the expected output format.

---

## 3. Research system

**File:** `crates/roko-cli/src/research.rs` (~286 lines)

Agent-driven research with academic rigor. Artifacts stored in `.roko/research/` as markdown files.

### CLI subcommands

| Command | What it does |
|---------|-------------|
| `roko research topic "<topic>"` | Deep research with citations (searches arXiv, ACL, NeurIPS, etc.) |
| `roko research enhance-prd <slug>` | Enhance PRD with research findings and supporting citations |
| `roko research enhance-plan <plan>` | Optimize plan with latest techniques from literature |
| `roko research enhance-tasks <plan>` | Split/optimize tasks based on research into decomposition |
| `roko research analyze` | Analyze execution data for self-learning insights |

### Research agent prompt

`RESEARCH_SYSTEM_PROMPT` enforces:

- Real citations with full author, title, venue, year in [AUTHOR-YEAR] format
- Practical relevance: every finding connects to a concrete recommendation
- Recency bias: prefer 2023-2026 papers
- Contrarian findings: actively seek papers that challenge the current approach
- Structured output: Finding / Source / Relevance / Recommendation / Confidence

### Sources checked

arXiv (cs.SE, cs.AI, cs.CL, cs.MA), ACL, EMNLP, NeurIPS, ICML, ICLR, ISSTA, ICSE, FSE, Anthropic/OpenAI/DeepMind research blogs, SWE-bench, HumanEval/MBPP benchmarks, and recent agent framework papers.

---

## 4. Cloud deployment (`roko serve`)

**Directory:** `crates/roko-cli/src/serve/` (~2,300+ lines across 21 files)

Complete HTTP API server and cloud deployment system.

### Server architecture

- `mod.rs` -- `run_server()` entry point: loads `roko.toml`, builds `AppState`, binds TCP listener with graceful shutdown
- `state.rs` -- `AppState` shared across all handlers (config, workdir, event bus)
- `events.rs` -- Server-sent event bus for real-time updates
- `error.rs` -- Typed API error responses
- `templates.rs` -- Template management utilities
- Respects `PORT` env var for Railway/cloud platform compatibility

### HTTP API endpoints

All routes nested under `/api`:

| Method | Path | Module | Description |
|--------|------|--------|-------------|
| GET | `/api/status` | `routes::status` | Server health, uptime, plan/PRD/agent counts |
| GET | `/api/plans` | `routes::plans` | List discovered plans with metadata |
| GET | `/api/plans/:id` | `routes::plans` | Plan details including task list |
| POST | `/api/plans/:id/run` | `routes::plans` | Execute a plan (kicks off orchestration loop) |
| POST | `/api/plans/:id/resume` | `routes::plans` | Resume a paused/failed plan from snapshot |
| DELETE | `/api/plans/:id` | `routes::plans` | Delete a plan |
| GET | `/api/prds` | `routes::prds` | List all PRDs |
| GET | `/api/prds/:slug` | `routes::prds` | Get PRD content and metadata |
| POST | `/api/prds` | `routes::prds` | Create new PRD |
| POST | `/api/prds/:slug/plan` | `routes::prds` | Generate implementation plan from PRD |
| POST | `/api/prds/:slug/promote` | `routes::prds` | Promote draft PRD to published |
| GET | `/api/agents` | `routes::agents` | List running/registered agents |
| GET | `/api/agents/:id` | `routes::agents` | Get agent details and stats |
| POST | `/api/agents/:id/stop` | `routes::agents` | Stop a running agent |
| GET | `/api/research` | `routes::research` | List research artifacts |
| POST | `/api/research` | `routes::research` | Start a research task |
| GET | `/api/learning` | `routes::learning` | Get learning metrics (episodes, efficiency, experiments) |
| GET | `/api/config` | `routes::config` | Get roko.toml configuration |
| PUT | `/api/config` | `routes::config` | Update configuration |
| POST | `/api/run` | `routes::run` | Single prompt -> universal loop (compose->agent->gate->persist) |
| GET | `/api/deployments` | `routes::deployments` | List cloud deployments |
| POST | `/api/deployments` | `routes::deployments` | Deploy to Railway/cloud |
| DELETE | `/api/deployments/:id` | `routes::deployments` | Tear down deployment |
| GET | `/api/templates` | `routes::templates` | List plan/PRD templates |
| POST | `/api/templates` | `routes::templates` | Create a template |
| WS | `/api/ws` | `routes::ws` | WebSocket event stream |

### Middleware

- CORS -- configurable allowed origins, permissive by default
- `tower_http::TraceLayer` -- request tracing
- Route grouping via `build_router()` that merges all submodule routers

### Deployment backends

**Railway CLI** (`deploy/railway_cli.rs`):
- `railway up` -- deploy to Railway
- `railway link` -- link to existing project
- Environment variable management

**Railway GraphQL API** (`deploy/railway_api.rs`):
- Create project and service via GraphQL
- Deploy service from Docker image
- Retrieve deployment logs

**Manual** (`deploy/manual.rs`):
- Generates Dockerfile + deployment instructions
- Produces `docker-compose.yml` for self-hosting

### Worker system

- `worker/mod.rs` + `worker/handler.rs` -- cloud worker that polls the server for pending tasks and executes them
- `roko worker` subcommand -- runs the polling loop
- `docker/worker.Dockerfile` -- container image for cloud workers
- `railway.toml` -- Railway platform configuration

---

## 5. Mirage-RS dashboard and API

**Directory:** `apps/mirage-rs/` (~8,000+ lines new/modified)

### HTTP REST API

All endpoints served under `/api` via `axum::Router`. Every list endpoint returns a `PaginatedResponse` envelope:

```json
{
  "items": [...],
  "total": 142,
  "offset": 0,
  "limit": 100,
  "has_more": true
}
```

#### Health and stats

| Method | Path | Description |
|--------|------|-------------|
| GET | `/api/health` | Server health: status, uptime_secs, chain toggles (hdc/knowledge/stigmergy), counts (insights/pheromones/agents/tasks) |
| GET | `/api/stats` | Combined dashboard stats: insight state breakdown (active/confirmed/challenged/decaying), pheromone kind breakdown (threat/opportunity/wisdom + total_intensity), task state breakdown (open/assigned/in_progress/completed/failed/cancelled + stake/reward totals), chain toggles |

#### Pheromone field

| Method | Path | Description |
|--------|------|-------------|
| GET | `/api/pheromones` | List active pheromones. Filters: `kind` (threat/opportunity/wisdom), `min_intensity`. Sort: `intensity` (default), `deposited_at`, `confirmations`. Pagination: `offset`, `limit` (max 1000). Each item includes `decay_projection` with `in_1h`, `in_4h`, `in_24h` intensities. |
| POST | `/api/pheromones` | Deposit a new pheromone. Body: `kind`, `content`, `intensity`, `half_life_seconds`. Returns the created pheromone with ID. |
| GET | `/api/pheromones/summary` | Aggregate stats per kind: count, total intensity, avg intensity, max intensity, avg half-life. |
| POST | `/api/pheromones/query` | Top-K by HDC similarity x intensity. Body: `query` (text), `k` (max 100), optional `kind` filter. Uses `ProjectionCache` (LRU, default 1024 entries) to avoid recomputing HDC projections. |
| GET | `/api/pheromones/heatmap` | Time-bucketed deposit activity. Params: `bucket_width` (min 60s), `buckets` (max 500). Returns array of `{start, end, count, kinds: {threat, opportunity, wisdom}}`. |
| GET | `/api/pheromones/{id}/projection` | Decay projection for a single pheromone at future timestamps. |

#### Knowledge graph

| Method | Path | Description |
|--------|------|-------------|
| GET | `/api/knowledge/entries` | List insight entries. Filters: `kind` (insight/heuristic/warning/causal_link/strategy_fragment/anti_knowledge), `state` (created/active/confirmed/decaying/challenged/pruned/stale), `min_weight`. Sort: `weight` (default), `created_at`, `confirmations`. Each entry includes: id, kind, weight, initial_weight, state, confirmations, challenges, created_at, content, author, enabled_by deps, half_life_seconds, effective_half_life_seconds, stake_wei (string for precision). |
| POST | `/api/knowledge/entries` | Post a new insight. Body: `content`, `author`, `kind`, `stake_wei`, optional `enabled_by` (dependency IDs). |
| POST | `/api/knowledge/entries/{id}/confirm` | Confirm an insight. Body: `confirmer` (agent ID). Updates confirmer's `confirmations_given` stat. |
| POST | `/api/knowledge/entries/{id}/challenge` | Challenge an insight. Body: `challenger` (agent ID). Updates challenger's `challenges_given` stat. |
| POST | `/api/knowledge/decay` | Trigger manual decay sweep across all entries. |
| GET | `/api/knowledge/edges` | Dependency edges (from `enabled_by`) + HDC similarity edges between entries. Returns `{dependency_edges, similarity_edges}` for force-directed graph layout. |
| GET | `/api/knowledge/search` | Semantic search over knowledge store. Params: `q` (query text), `k` (max 100). Uses HDC projection + cosine similarity for top-k ranking. |
| GET | `/api/knowledge/kinds` | Enumerate all knowledge kinds and pheromone kinds with descriptions. |

#### Agent registry

| Method | Path | Description |
|--------|------|-------------|
| GET | `/api/agents` | List all registered agents with summary stats: id, role, registered_at, last_heartbeat_block, last_heartbeat_ts, stats (confirmations_given, challenges_given, warnings_posted, insights_posted, tasks_completed, tasks_failed, delta_cycles, total_cost_usd, total_tokens). |
| POST | `/api/agents` | Register a new agent. Body: `id`, `pubkey` (string), `role`. |
| GET | `/api/agents/{id}/trace` | Cognitive loop history (paginated). Params: `limit` (default 10), `offset`. Each trace: cycle, phase (retrieve/reason/act/verify), reads, reasoning, action, action_id, timestamp. |
| POST | `/api/agents/{id}/trace` | Record a cognitive trace entry. Body: `cycle`, `phase`, `reads`, `reasoning`, `action`, `action_id`. |
| GET | `/api/agents/{id}/heartbeat` | Liveness status: alive (bool), last_block, last_timestamp, blocks_since, timeout_blocks (200). |
| POST | `/api/agents/{id}/heartbeat` | Send heartbeat. Body: optional `total_tokens`, `total_cost_usd`. Updates agent's last heartbeat and token/cost stats. |
| GET | `/api/agents/{id}/stats` | Aggregated stats for an agent: all fields from `AgentStats`. |

#### Agent topology

| Method | Path | Description |
|--------|------|-------------|
| GET | `/api/agents/topology` | Agent interaction graph derived from knowledge store. Returns `{nodes, edges, timestamp}`. Nodes: id, address, insights_posted, confirmations_given, challenges_given, total_weight. Edges: from, to, weight, type ("confirmed" or "challenged"). |

#### Task tracking

| Method | Path | Description |
|--------|------|-------------|
| GET | `/api/tasks` | List tasks with filters. Params: `state` (open/assigned/in_progress/completed/failed/cancelled), `kind` (research/validate/analyze/monitor/report/...), `assignee` (agent ID), `limit` (default 20, max 200), `offset`. Each task: id, title, description, kind, priority (low/medium/high/critical), state, creator, assignee, created_at, assigned_at, started_at, completed_at, stake_wei, reward_wei, result_insight_id, tags, attempts, max_attempts. |
| POST | `/api/tasks` | Create a new task. Body: `title`, `description`, `kind`, `priority`, `creator`, `tags`, `stake_wei`, `max_attempts`. |
| GET | `/api/tasks/stats` | Aggregate counts: open, assigned, in_progress, completed, failed, cancelled, total_stake_wei, total_reward_wei. |
| GET | `/api/tasks/{id}` | Get a single task by ID. |
| POST | `/api/tasks/{id}/assign` | Assign task to agent. Body: `assignee`. Transitions Open -> Assigned. |
| POST | `/api/tasks/{id}/start` | Mark task in-progress. Transitions Assigned -> InProgress. |
| POST | `/api/tasks/{id}/complete` | Complete task. Body: optional `result_insight_id`. Awards reward, increments agent's `tasks_completed`. Transitions InProgress -> Completed. |
| POST | `/api/tasks/{id}/fail` | Fail task. Increments agent's `tasks_failed` and task's `attempts`. Auto-cancels if `attempts >= max_attempts`. Transitions InProgress -> Failed (or Cancelled). |
| POST | `/api/tasks/{id}/cancel` | Cancel task. Transitions any non-terminal state -> Cancelled. |

#### WebSocket streaming

| Method | Path | Description |
|--------|------|-------------|
| WS | `/api/ws` | Live event stream. Params: `pheromones` (bool, default true), `insights` (bool, default true), `agents` (bool, default false), `agent_id` (optional filter). Wire format: `{"channel": "pheromone"|"insight"|"agent", "data": {...}}`. Server pings every 30s, closes if no pong within 90s. |

### JSON-RPC methods (chain_*)

New RPC methods added to the existing `chain_*` namespace:

| Method | Params | Returns |
|--------|--------|---------|
| `chain_registerAgent` | `{id, pubkey, role}` | `{id, registered_at}` |
| `chain_agentHeartbeat` | `{agent_id, block?, total_tokens?, total_cost_usd?}` | `{alive, last_block}` |
| `chain_agentTrace` | `{agent_id, cycle, phase, reads, reasoning, action, action_id}` | `{recorded: true}` |
| `chain_agentStats` | `{agent_id}` or `{agent_id, delta: {...}}` | `AgentStats` |
| `chain_listAgents` | none | `[AgentEntry]` |
| `chain_createTask` | `{title, description, kind, priority, creator, tags?, stake_wei?}` | `TaskEntry` |
| `chain_assignTask` | `{task_id, assignee}` | `TaskEntry` |
| `chain_completeTask` | `{task_id, result_insight_id?}` | `TaskEntry` |
| `chain_failTask` | `{task_id}` | `TaskEntry` |

### Infrastructure middleware

- **Request ID** -- `x-request-id` header injected via `AtomicU64` counter (`req-1`, `req-2`, ...), echoed on response
- **Cache-Control** -- `public, max-age=N` on read-only endpoints (2s for data, `no-cache` for static files)
- **Concurrency limit** -- `tower::limit::ConcurrencyLimitLayer::new(200)`
- **Tracing** -- `tower_http::TraceLayer` on all routes
- **Validation constants** -- `MAX_LIMIT=1000`, `MAX_K=100`, `MIN_BUCKET_WIDTH=60s`, `MAX_HEATMAP_BUCKETS=500`
- **HDC projection cache** -- thread-safe bounded LRU (`ProjectionCache`) backed by `lru::LruCache<String, HdcVector>` with configurable capacity (default 1024)

### Data models

**AgentEntry:**

| Field | Type | Description |
|-------|------|-------------|
| `id` | `String` | Unique agent identifier |
| `address` | `Vec<u8>` | On-chain address bytes |
| `role` | `String` | Agent role (researcher, coder, watcher, etc.) |
| `registered_at` | `u64` | Registration timestamp (Unix seconds) |
| `last_heartbeat_block` | `u64` | Block number of last heartbeat |
| `last_heartbeat_ts` | `u64` | Timestamp of last heartbeat |
| `stats` | `AgentStats` | Accumulated statistics |

**AgentStats:**

| Field | Type | Description |
|-------|------|-------------|
| `confirmations_given` | `u64` | Insight confirmations issued |
| `challenges_given` | `u64` | Insight challenges issued |
| `warnings_posted` | `u64` | Warnings posted |
| `insights_posted` | `u64` | Insights posted |
| `tasks_completed` | `u64` | Tasks completed successfully |
| `tasks_failed` | `u64` | Tasks that failed |
| `delta_cycles` | `u64` | Cognitive cycles completed |
| `total_cost_usd` | `f64` | Total cost in USD |
| `total_tokens` | `u64` | Total tokens consumed |

**AgentTrace:**

| Field | Type | Description |
|-------|------|-------------|
| `cycle` | `u64` | Cognitive cycle number |
| `phase` | `CognitivePhase` | retrieve, reason, act, or verify |
| `reads` | `Vec<String>` | Resources read during this phase |
| `reasoning` | `String` | Reasoning text |
| `action` | `String` | Action taken |
| `action_id` | `String` | Unique action identifier |
| `timestamp` | `u64` | Unix timestamp in seconds |

**AgentEvent** (WebSocket, tagged union):
- `Trace { agent_id, trace }`
- `Heartbeat { agent_id, block, timestamp }`
- `Stats { agent_id, delta }`
- `Registered { agent_id, role }`

**TaskEntry:**

| Field | Type | Description |
|-------|------|-------------|
| `id` | `u64` | Auto-incrementing task ID |
| `title` | `String` | Short human-readable title |
| `description` | `String` | Detailed work description |
| `kind` | `String` | research, validate, analyze, monitor, report, etc. |
| `priority` | `TaskPriority` | low, medium, high, critical |
| `state` | `TaskState` | open, assigned, in_progress, completed, failed, cancelled |
| `creator` | `String` | Agent ID that created the task |
| `assignee` | `Option<String>` | Agent ID assigned to the task |
| `created_at` | `u64` | Creation timestamp |
| `assigned_at` | `Option<u64>` | Assignment timestamp |
| `started_at` | `Option<u64>` | Work start timestamp |
| `completed_at` | `Option<u64>` | Terminal state timestamp |
| `stake_wei` | `u128` | Stake deposited for this task |
| `reward_wei` | `u128` | Reward paid on completion |
| `result_insight_id` | `Option<String>` | ID of produced insight |
| `tags` | `Vec<String>` | Topic tags for matching |
| `attempts` | `u32` | Times this task was attempted |
| `max_attempts` | `u32` | Auto-cancel threshold |

**TaskEvent** (streaming, tagged union):
- `Created { id, title, kind, creator }`
- `Assigned { id, assignee }`
- `Started { id }`
- `Completed { id }`
- `Failed { id }`
- `Cancelled { id }`

**Task state machine:**

```
Open -> Assigned -> InProgress -> Completed
                               -> Failed (retryable if attempts < max_attempts)
                               -> Cancelled
Any non-terminal -> Cancelled
```

### Dashboard frontend

**File:** `apps/mirage-rs/static/index.html` + `static/js/` + `static/style.css`

Single-page dashboard built with vanilla JS (no framework), ES modules, and Canvas 2D rendering.

#### Module architecture

| File | Responsibility |
|------|---------------|
| `js/main.js` | Init, `connect()`, `requestAnimationFrame` loop, event wiring, interval setup for all pollers |
| `js/state.js` | Single shared mutable state object imported by all modules: blocks, insights (Map), pheromones (particles), topology, heatmap, agent registry, sparkline series, RPC counters, poller handles |
| `js/api.js` | `rpc()` for JSON-RPC, `api()` for REST GET, `apiPost()` for REST POST, request logging, toast notifications, render callback registration |
| `js/polling.js` | All REST polling functions: `pollBlock`, `pollChain`, `pollEntries`, `pollEdges`, `pollKinds`, `pollPheroSummary`, `pollHeatmap`, `pollTopology`, `pollAgentRegistry`, `pollLeaderboard`, `pollTasks` |
| `js/pheromones.js` | Force-directed particle system: spatial grid for O(n) neighbor queries, shaped particles (diamond=threat, circle=opportunity, hexagon=wisdom), hover/click tooltips, entrance animations, death fade, decay projection overlay, kind filter pills, FPS counter |
| `js/graph.js` | Force-directed insight knowledge graph: HDC proximity edges, kind-colored nodes, click-to-detail sidebar, search highlighting, dynamic node addition |
| `js/topology.js` | Force-directed agent network graph: confirm/challenge edges, role-colored nodes, weight-scaled edges |
| `js/charts.js` | Sparkline renderer, knowledge growth timeline, heatmap visualization, block stream renderer, metric cards |
| `js/ws.js` | WebSocket live event stream: auto-reconnect with backoff, pheromone + insight event handlers, connection status chip |
| `style.css` | 680-line dark theme: glassmorphism panels, gradient accents, canvas animations, responsive grid, chip/badge system |

#### Dashboard sections (top to bottom)

1. **Header** -- RPC URL input, reconnect button, connection status chip, fork info chip (block number + upstream URL), agent count chip, WS toggle, reset button
2. **Hero stats** -- 6 cards with sparklines: chain tip, gas base fee (gwei), saturation (%), insights on-chain, live pheromones, registered agents. Each card shows current value, delta indicator, and a canvas sparkline
3. **Knowledge accumulation timeline** -- 60s rolling growth chart
4. **Block stream** -- Recent blocks with number, hash, gas, tx count, saturation bar
5. **Pheromone field** -- Canvas particle system: force-directed bubbles shaped by kind (diamond/circle/hexagon), colored by kind (red=threat, green=opportunity, gold=wisdom), sized by intensity, with spatial grid for collision detection. Hover shows tooltip with content, intensity, decay projection. Click pins the tooltip. Kind filter pills toggle visibility.
6. **Agent activity log** -- Real-time log of agent actions (posts, confirmations, challenges)
7. **Agent registry** -- Table of registered agents: ID, role, heartbeat status, stats. Expandable rows show cognitive traces (Retrieve -> Reason -> Act -> Verify per cycle)
8. **Pheromone summary** -- Per-kind aggregate cards: count, total intensity, avg intensity
9. **Task lifecycle** -- Task state distribution (open/assigned/in_progress/completed/failed), recent task list, stake/reward totals
10. **Tokenomics** -- Stake/reward economics, total value locked, completion rates
11. **Pheromone heatmap** -- Time-bucketed activity chart with kind breakdown
12. **Insight knowledge graph** -- Canvas force-directed graph: nodes colored by kind, sized by weight, connected by dependency + similarity edges. Click a node for detail sidebar. Search box highlights matching nodes.
13. **Agent topology** -- Canvas force-directed network: nodes per agent, edges per confirm/challenge interaction, weight-scaled
14. **Agent leaderboard** -- Ranked by total activity (insights + confirmations + challenges + tasks)
15. **Performance metrics** -- RPC call rate, cache hit rate, search latency
16. **Knowledge kinds reference** -- All registered kinds with descriptions
17. **Manual controls** -- Forms to post insight, deposit pheromone, run semantic search, register agent
18. **API trace log** -- Rolling log of all API requests with timing

#### Pheromone particle system details

- `P_COLORS`: threat (red #f87171), opportunity (green #4ade80), wisdom (gold #fbbf24)
- `P_HALFLIFE`: threat 60s, opportunity 90s, wisdom 180s
- Spatial grid (`GRID_CELL=60px`) for O(n) neighbor lookup via `gridBuild()` / `gridNeighbors()`
- Particles have: kind, content, intensity, position (x/y), anchor (anchorX/Y), velocity (vx/vy), age, deposited timestamp, halfLife, pulse animation, chainId, decayProjection
- Force simulation: repulsion between nearby particles, gentle attraction to anchor, velocity damping
- Entrance animation: scale from 0 to 1 with overshoot
- Death animation: fade out when intensity drops below threshold
- Filter pills: toggle threat/opportunity/wisdom visibility
- FPS tracking for performance monitoring

### Agent simulation

**File:** `apps/mirage-rs/examples/agent_simulation.rs`

20 concurrent agent personas across 6 roles, each running as a tokio task:

| Role | Count | Behavior | Pace |
|------|-------|----------|------|
| **Watcher** | 6 | Post insights about DeFi protocols, deposit pheromones, create tasks | 12-25s |
| **Security** | 4 | Post exploit alerts, rug pull detection, phishing warnings | 22-30s |
| **Strategy** | 4 | Yield farming analysis, momentum signals, correlation patterns | 20-28s |
| **Validator** | 4 | Confirm/challenge entries, claim and complete/fail tasks | 15-20s |
| **Synthesizer** | 2 | Cross-reference insights, post meta-analyses | 30-35s |
| **Infra** | 2 | Health monitoring, decay sweeps, task creation | 25-30s |

**Watcher agents** (6 unique DeFi focuses):
- `roko-alpha-amm` -- AMM pools: Uniswap, Curve, Balancer liquidity analysis
- `roko-beta-lending` -- Lending protocols: Aave, Compound, Morpho utilization monitoring
- `roko-gamma-mev` -- MEV: sandwiches, frontrunning, Flashbots block analysis
- `roko-delta-bridge` -- Cross-chain bridges and L2 monitoring
- `roko-epsilon-governance` -- DAO governance and voting pattern analysis
- `roko-zeta-derivatives` -- Perps, options, structured products tracking

**Security agents** (4):
- `roko-sec-exploit` -- Smart contract exploit detection
- `roko-sec-rugpull` -- Rug pull pattern matching
- `roko-sec-phishing` -- Phishing and social engineering alerts
- `roko-sec-audit` -- Code audit findings

**Cognitive traces:** Each agent cycle produces a 4-phase trace (Retrieve -> Reason -> Act -> Verify) with reads, reasoning text, action description, and unique action ID.

**Task templates:** 15 templates spanning: analyze pool rebalancing, monitor liquidation thresholds, research yield strategies, validate MEV claims, report bridge anomalies.

**Usage:**
```bash
cargo run -p mirage-rs --features chain,roko --example agent_simulation -- \
    --rpc-url http://127.0.0.1:8545
```

### Integration tests

**File:** `apps/mirage-rs/tests/http_api.rs` (1,061 lines)

Full coverage of all REST endpoints:

- Pheromone endpoints -- list with filters, deposit, summary, HDC query, heatmap, projection
- Knowledge endpoints -- list with filters, post insight, confirm, challenge, decay, edges, search, kinds
- Agent endpoints -- list, register, trace (get/post), heartbeat (get/post), stats
- Task endpoints -- list with filters, create, get, assign, start, complete, fail, cancel, stats
- Topology -- agent interaction graph
- Stats -- combined dashboard stats
- Health -- uptime, chain status, counts
- Pagination -- offset/limit, has_more flag
- Sorting -- ascending/descending on multiple fields
- Error handling -- 404 (not found), 409 (state conflict), 400 (validation)
- Input validation -- clamped limits, invalid kind strings, missing fields
- Cache-Control headers -- verified on read-only endpoints

---

## 6. Fork state improvements

- `ForkState` tracks `fork_block` (upstream head block number at fork time) and `fork_url` (upstream RPC URL)
- `mine_block()` advances timestamp via `now_secs()` instead of being frozen at init time
- `MirageStatus` includes `forkBlock` and `forkUrl` in `mirage_status` JSON-RPC responses
- `UpstreamRpc::http_url()` accessor added for fork URL display
- Cache-Control middleware on `/dashboard` static files: `no-cache, must-revalidate`

---

## 7. Agent system

**Directory:** `crates/roko-agent/`

### ClaudeCliAgent (new, 701 lines)

Spawns Claude Code CLI processes with full configuration:

- Model selection with fallback
- 6-layer system prompt injection
- MCP config passthrough (`--mcp-config` flag from `agent.mcp_config` in `roko.toml`)
- Bare mode (no interactive UI)
- Effort level control
- Tool allowlists (CSV)
- Settings JSON generation via `build_settings_json()`
- `--dangerously-skip-permissions` flag for CI/automation
- Session resume support
- Extra CLI args passthrough
- Environment variable injection
- Timeout enforcement

### Safety integration

Safety layer integrated into `ToolDispatcher`:
- Role-based authorization -- agents can only use tools permitted for their role
- Pre-execution checks -- validate tool arguments before execution
- Post-execution checks -- validate tool results after execution
- 342 lines of new safety integration tests

### MCP config passthrough

When `agent.mcp_config` is set in `roko.toml`, the path is forwarded to all spawned Claude CLI agents via `--mcp-config`. Auto-discovery fallback checks standard MCP config locations.

---

## 8. Learning and feedback

**Directory:** `crates/roko-learn/` (~1,800+ lines new)

### PromptExperiment

A/B testing framework for prompt variants:
- Define experiment variants with different prompt sections
- Track success/failure rates per variant
- Bayesian analysis for variant selection
- Persisted to `.roko/learn/experiments.json`

### RuntimeFeedback (937 lines)

`LearningRuntime` collects execution data and produces actionable updates:
- `CompletedRunInput` -- captures plan ID, task results, gate verdicts, agent metrics, cost data
- `LearningUpdate` -- recommendations for prompt changes, model routing, gate thresholds
- Integrates efficiency events, cost records, and episode data

### CostsLog

Per-task cost logging:
- Model, tokens, estimated cost per agent invocation
- Aggregated per plan and per session

### AdaptiveThreshold

EMA-based gate threshold adjustment:
- Tracks pass/fail rate per gate rung
- Adjusts thresholds based on recent performance
- Prevents threshold drift from noisy results
- Persisted to `.roko/learn/gate-thresholds.json`

### CascadeRouter persistence

Model routing decisions saved to `.roko/learn/cascade-router.json` for replay and analysis. Configurable model tiers.

### Integration tests (227 lines)

Coverage for experiment store, runtime feedback collection, threshold adjustment, and cost logging.

---

## 9. Compose system

**Directory:** `crates/roko-compose/` (~2,500+ lines new)

### ContextProvider (1,122 lines)

Assembles context for agent prompts:
- Gathers relevant source files based on task description
- Resolves code symbols referenced in the task
- Includes recent change history (git diff/log)
- Ranks context by relevance and fits within token budget
- Supports `Placement` (system/user) and `SectionPriority` (required/preferred/optional)

### SymbolResolver (616 lines)

Code symbol resolution for prompt context:
- Resolves function, struct, trait, and module references
- Extracts signatures and doc comments
- Supports cross-crate resolution

### TaskBrief (365 lines)

Generates structured task briefs for agents:
- Task description and acceptance criteria
- Dependency context (what prior tasks produced)
- File scope (which files the agent should modify)
- Gate expectations (what validation will run)

### RolePrompts (364 lines)

Role-specific prompt templates for the 6-layer system prompt builder:
- Implementer -- code generation focused
- Reviewer -- code review and quality
- Strategist -- architecture and planning
- Researcher -- literature search and analysis
- Debugger -- error diagnosis and fix
- Templates integrated via `RoleSystemPromptSpec`

### PromptComposer

Assembles `PromptSection`s into final prompts:
- Sections carry `Placement` (system vs user) and `SectionPriority`
- Budget-aware: trims optional sections when token limit approached
- Produces `PlanArtifacts` and `TaskContext` for executor consumption

---

## 10. Orchestrator

**Directory:** `crates/roko-orchestrator/` (~500+ lines modified)

- **Cross-plan dependency support** -- tasks can declare `depends_on` tasks in other plans; executor resolves cross-plan edges during DAG construction
- **Worktree improvements** (115 lines) -- `WorktreeConfig` and `WorktreeManager` for isolated git worktrees per plan execution
- **Lifecycle integration tests** (177 lines) -- end-to-end executor lifecycle: plan -> dispatch -> gate -> persist -> advance
- **Post-merge improvements** (145 lines) -- `PostMergeRunner` handles git operations after successful plan completion
- **DAG improvements** -- better parallel scheduling with dependency-aware task ordering
- **Event log** -- `EventLog`, `EventLogSnapshot`, `EventKind` for durable execution history

---

## 11. Gate system

**Directory:** `crates/roko-gate/` (~400+ lines new)

### AdaptiveThreshold (217 lines)

Adaptive gate threshold system:
- Tracks pass rates per gate rung using exponential moving average
- Adjusts thresholds up when pass rate is high (tighten quality), down when low (avoid blocking)
- Configurable alpha (learning rate) and min/max bounds
- Integrates with `roko-learn` persistence layer

### Gate improvements

- Symbol gate -- validates that expected symbols exist in modified files
- Verify chain gate -- validates chain of custody for modifications
- Integration gate -- validates cross-crate compatibility after changes

---

## 12. Additional changes

### roko-golem (new crate, phase 2+ scaffolding)

Modules for future chain witness and autonomous agent capabilities:
- `chain_witness` -- on-chain attestation of agent actions
- `daimon` -- autonomous agent daemon
- `dreams` -- long-term planning and goal setting
- `grimoire` -- knowledge base and memory
- `hypnagogia` -- sleep/wake cycle management

### roko-fs observability

- `observability.rs` (162 lines) -- `FsObservabilitySinks` for file system operation metrics
- Tool metrics sink (244 lines) -- tracks tool execution latency, success/failure rates

### CLI additions

- `tui/` -- text-mode dashboard with pages (efficiency view, operations view)
- `task_parser.rs` (611 lines) -- TOML task parser for `tasks.toml` files in plan directories
- `index.rs` (446 lines) -- codebase indexer for context assembly

### Plans

- P06: Process management plan with `tasks.toml`
- P07: Autofix retry plan with `tasks.toml`
- W01: Wire system prompts plan with `tasks.toml`

### Configuration

- `roko.toml` (70 lines) -- project configuration: server bind/port, agent settings (model, MCP config, timeout), gate thresholds, learning toggles

### Docker

- `docker/worker.Dockerfile` -- worker container for cloud execution
- Roko Dockerfile updates for `roko serve` deployment

---

## 13. Mori parity -- 228 checklist items

The bulk of new work in this PR. A `run-parity.sh` script systematically executed parity items from the mori-parity checklist. Each item was committed individually with a `parity(SECTION.ITEM)` prefix.

### Section 1A: Orchestration hardening (22 items)

Commits `824c2e1` through `0beef0c`.

- Wire `ExecutorConfig` from `roko.toml` `[executor]` section
- Implement `Enriching` phase handler: run SystemPromptBuilder before agent dispatch
- Implement `Verifying` phase handler: run task's verify commands after gates pass
- Implement `Reviewing` phase handler: compare agent output against task spec
- Implement `DocRevision` phase handler: auto-generate doc updates for public API changes
- Implement `AutoFixing` phase handler: on gate failure, extract error, build fix prompt, re-run agent
- Implement `RegeneratingVerify` phase handler: after autofix, re-run verification
- Wire `--resume`: load `ExecutorState` from `.roko/state/executor.json`, skip completed tasks
- Wire cross-plan dependency tracking: parse `depends_on_plan` field in tasks.toml
- Ensure task TOML fields are parsed: `tier` (model selection), `timeout`, `verify`, `role`
- Add `[executor]` section to default `roko.toml` template
- Wire `WorktreeManager` for isolated git worktrees per task execution
- Wire parallel task dispatch: tasks within a dependency level run concurrently up to limit
- Clean stale git locks: `.git/index.lock` files older than 60s cleaned before worktree ops
- Set `CARGO_TARGET_DIR` to shared target dir to avoid duplicate compilation across worktrees
- After each agent dispatch, record cost (input_tokens, output_tokens, model, estimated USD)
- Before dispatching, check `config.budget.max_task_usd` -- abort task if projected cost exceeds limit
- Track cumulative plan cost -- abort plan execution if `config.budget.max_plan_usd` exceeded
- Warn at `config.budget.warn_at_percent` threshold (default 80%)
- Cost data recorded in episode log alongside `wall_ms`

### Section 1B: Conductor wiring (15 items)

Commits `a976394` through `6bfe947`.

- Create `WatcherRunner`: spawn tokio task that feeds agent turn data to all watchers
- Wire `WatcherRunner` lifecycle: start when `run_task_plans()` begins, cancel on completion/abort
- Wire circuit breaker response: when `CircuitBreaker::is_broken(plan_id)` returns true, pause execution
- Wire diagnosis integration: when circuit breaker trips, call `DiagnosisEngine` to emit root cause analysis
- Wire `cost_overrun` watcher: feed actual cost data from efficiency events
- Wire `stuck_pattern` watcher: detect when agent produces same output 3+ consecutive turns
- Wire `context_window_pressure` watcher: read agent token usage, alert when >80% of context window
- Wire `time_overrun` watcher: compare elapsed time against `timeout_ms` from task config
- Wire `compile_fail_repeat` watcher: detect same compilation error appearing 3+ consecutive times
- Wire `ghost_turn` watcher: detect agent turns that produce no file changes
- Wire `spec_drift` watcher: compare agent's actual file changes against task's expected `files` list
- Wire `test_failure_budget` watcher: track test failure count, alert if failures exceed threshold
- Wire `iteration_loop` and `review_loop` watchers: detect repeated gate failures suggesting infinite loop
- Emit all watcher alerts as Signals to `.roko/signals.jsonl` with kind `conductor:alert`

### Section 1C: Tool system (10 items)

Commits `3316733` through `5c5724c`.

- Add `allowed_tools` and `denied_tools` fields to task TOML schema
- Wire tool filtering in `ToolDispatcher::dispatch()`: check allowed/denied before execution
- Wire `--allowedTools` CLI flag passthrough to ClaudeCliAgent
- Define role-based tool profiles in `crates/roko-std/src/roles.rs`
- Wire role profiles: when task specifies `role`, auto-populate tool allow/deny lists
- Wire MCP server lifecycle in `orchestrate.rs`: start before task, stop after
- Wire tool discovery: after MCP server starts, call `tools/list` JSON-RPC to register available tools
- Wire tool dedup in `DynamicToolRegistry`: if MCP tool has same name as builtin, prefer MCP version
- Add `[tools]` section to `roko.toml`: `prefer_mcp` (bool), `global_denied` (list)
- Add per-task `mcp_servers` field in tasks.toml for task-specific MCP servers

### Section 1D: Structured logging (11 items)

Commits `6cffd86` through `9673e29`.

- Add `tracing-subscriber` dependency with `env-filter`, `fmt`, `json` features
- Wire subscriber initialization in `main.rs` before async runtime starts
- Add span hierarchy in `orchestrate.rs`: `run_task_plans()`, `run_plan()`, `run_task()`, `run_phase()`
- Add span fields that propagate: `plan_id`, `task_id`, `agent_model`, `task_tier`
- Wire `--log-format` CLI flag (text/json/compact)
- Wire `ROKO_LOG` env var for log level filtering
- Add cost summary at end of `run_task_plans()`: aggregate efficiency events, print total cost/tokens
- Wire timing: record `Instant::now()` at task start, log elapsed on completion
- Wire secret redaction: tracing layer scans log lines for patterns matching `${VAR}` references
- Wire structured error context: on task failure, log full error chain with `anyhow` context
- Ensure all `eprintln!` and `println!` replaced with `tracing::info!`/`tracing::warn!`/`tracing::error!`

### Section 1E: Re-planning (10 items)

Commits `6f90d1e` through `49f6d77`.

- Extract `ReplanStrategy` enum: `RetrySame`, `RetryWithEscalation`, `Decompose`, `RegeneratePlan`
- Add `replan_strategy` selection logic in `attempt_replan()`: 1st failure -> RetrySame, 2nd -> RetryWithEscalation, 3rd -> Decompose, 4th -> RegeneratePlan
- Implement `Decompose` strategy: build prompt with original task spec + failure context, ask agent to split into subtasks, insert subtasks into executor DAG
- Implement `RetryWithEscalation`: change task's `model_hint` to next tier before retry
- Implement `RegeneratePlan`: call `prd plan <slug>` to regenerate entire plan from PRD
- Add `max_retries` field to task TOML schema (default 3)
- Add `replan_strategy` field to task TOML schema (optional, overrides automatic selection)
- Feed failure patterns to learning: after each failed task, emit an `EfficiencyEvent` with failure details
- Wire `--no-replan` CLI flag: skip all re-planning, tasks that fail gate -> immediate failure
- Log all re-plan events to `.roko/episodes.jsonl` with `kind: "replan"` and strategy used

### Section 1F: Auto plan generation (12 items)

Commits `ad22b7a` through `7fbf927`.

- Wire `on_prd_promote` hook in `prd.rs`: after promoting a draft, automatically call `prd plan <slug>`
- Add `auto_plan` field to `[prd]` config section in `roko.toml` (default true)
- Wire plan structural validation after generation: verify all tasks have required fields, no circular deps
- Wire plan quality heuristics (warnings): warn if any task has >50K token budget, if plan has >20 tasks, if all tasks target same file
- Add `--auto-execute` flag to `prd draft promote`: after plan generation, immediately run the plan
- Add `--dry-run` flag to `prd plan`: generate plan, validate, print summary, but don't write to disk
- Wire signal emission: on auto-plan completion, emit `prd:plan:generated` signal to `.roko/signals.jsonl`
- Wire plan template selection: optional `plan_template` field in PRD TOML
- Wire `roko plan validate <dir>` command: check if tasks.toml has modern fields
- Wire `roko plan regenerate <dir>` command: re-run plan generation from the PRD
- Wire `roko plan list` to flag old-format plans with warning icon
- Regenerate existing old-format plans (P06, W01, P07) to modern format

### Section 1G: Serve API (13 items)

Commits `152bcb9` through `a05e524`.

- Wire execution progress -> WebSocket: after each phase, send `ExecutionEvent` to all connected clients
- Define `ExecutionEvent` enum variants: `PlanStarted`, `TaskStarted`, `PhaseCompleted`, `TaskCompleted`, `PlanCompleted`, `Error`
- Wire `GET /api/gates/summary` endpoint
- Wire `GET /api/gates/{gate_name}/history` endpoint: time series of pass/fail per gate
- Wire `GET /api/learn/cascade` endpoint: CascadeRouter state
- Wire `GET /api/learn/experiments` endpoint: ExperimentStore data
- Wire `GET /api/learn/efficiency` endpoint: aggregate efficiency events
- Wire `GET /api/learn/adaptive-thresholds` endpoint
- Ensure all API endpoints return proper HTTP error codes (404, 409, 400, 500)
- Wire API auth toggle: `[serve.auth]` section in `roko.toml` with `enabled` and `api_key`
- Add `GET /api/health` endpoint: returns status, version, uptime, plan/PRD/agent counts
- Add `GET /api/metrics/summary` endpoint: aggregate metrics over time window
- Add per-metric endpoints: success_rate, engagement, model_efficiency, gate_rate, experiments, feedback_latency, velocity, coverage

### Section 1H: Interactive TUI (18 items)

Commits `ea8f51c` through `8a8bee3`.

- Add `ratatui = "0.29"` + `crossterm = "0.28"` dependencies
- Create `crates/roko-cli/src/tui/` module with `mod.rs`, `app.rs`, `pages/`, `widgets/`, `theme.rs`
- Define `App` struct with `current_page: Page`, `data: DashboardData`, `running: bool`
- Define `DashboardData` struct: loaded from `.roko/` files (signals, episodes, efficiency, plans, gates)
- Implement event loop in `app.rs` with crossterm key events + 1s data refresh tick
- Wire page 1 -- **Overview**: 3-column layout with health/plan/learning summary widgets
- Wire page 2 -- **Plan Execution**: focused view of currently executing plan, task progress bars
- Wire page 3 -- **Agent Activity**: active agents table with model, tokens, cost, elapsed time
- Wire page 4 -- **Gate Results**: gate summary table with pass/fail counts, adaptive thresholds
- Wire page 5 -- **Learning**: cascade router state table, experiment results, efficiency trends
- Wire page 6 -- **Signals**: recent signals table with timestamp, kind, source, message
- Wire keyboard navigation: `1-6` or `Tab/Shift+Tab` for pages, `q` to quit, `r` to refresh
- Wire live refresh: `DashboardData::refresh()` reads changed files only (checksum-based)
- Wire terminal setup/teardown: `enable_raw_mode()` on start, `disable_raw_mode()` on exit/panic
- Wire `roko dashboard` to launch TUI (replace text renderer), keep text renderer as fallback
- Wire `roko dashboard --text` flag to force text-mode output
- Wire `roko dashboard --page <N>` flag to start on a specific page
- Wire color theming: define `Theme` struct with configurable colors using `ratatui::style::Color`

### Section 1I: Skill library + playbooks (8 items)

Commits `a7b6a95` through `81ed6df`.

- Add `skill_library: SkillLibrary` field to `PlanRunner` in orchestrate.rs
- On task success (gate pass + merge): call `skill_library.extract_skill()` with task type, context, and outcome
- On task failure: call `skill_library.record_failure()` with same parameters
- Before building context for a task: call `skill_library.query()` with task description, inject matching skills as context
- Persist skill library to `.roko/learn/skills.json` after each extraction
- Add `playbook: PlaybookStore` field to `PlanRunner`, initialize from `.roko/learn/playbooks.json`
- On task success: call `playbook.record()` with task definition + outcome
- Before dispatch: call `playbook.lookup(task_type)` and inject as context in system prompt

### Section 1J: Context-aware model routing (10 items)

Commits `426411e` through `28dcd48`.

- Verify `CascadeRouter` is initialized from `.roko/learn/cascade-router.json` on startup
- After task success: call `cascade_router.observe(context_vec, model_idx, reward)`
- After task failure: call `cascade_router.observe(context_vec, model_idx, 0.0)`
- Before model selection: build context vector from task metadata, query `cascade_router.select()`
- Track per-crate "familiarity score" = `success_count / total_count` for the cascade router context
- Persist cascade router state after every observation
- Maintain rolling average per `(task_tier, context_source_type)` in `.roko/learn/context-attribution.jsonl`
- When building context for a task: load rolling averages, include/exclude context sources based on reference rates
- Log context attribution decisions: which sources were included/excluded and why
- Update rolling averages after each task completes (from attribution scan in episode data)

### Section 2A: roko-serve extraction (8 items)

Commits `e75f56a` through `4db57ff`.

- Move serve files from `crates/roko-cli/src/serve/` -> `crates/roko-serve/src/`
- Update roko-cli's `serve` subcommand handler to import from `roko-serve`
- Remove `crates/roko-cli/src/serve/` directory
- Add `roko-serve` to workspace `members` in root `Cargo.toml`
- Export a `ServerBuilder` in `roko-serve/src/lib.rs` for programmatic server construction
- Extract `EventBus` from `events.rs` into its own type with `subscribe()` -> `broadcast::Receiver`

### Section 2A continued: roko-serve integration tests (1 item)

Commit `5d27d4e`.

- Ensure all integration tests in roko-cli that test serve endpoints still pass after extraction to roko-serve

### Section 2B: Plugin system (9 items)

**New crate: `roko-plugin`**

Commits `224de6d` through `cced500`.

- Add `roko-plugin` to workspace members in root `Cargo.toml`
- Define `EventSource` trait: `async fn poll(&mut self) -> Option<Signal>`, `fn name(&self) -> &str`, `fn kind(&self) -> EventSourceKind`
- Define `FeedbackCollector` trait: `async fn collect(&self, episode_id: &str) -> Vec<FeedbackSignal>`, `fn service(&self) -> &str`
- Define `FeedbackSignal` struct: `original_episode_id: String`, `service: String`, `sentiment: Sentiment`, `raw_data: serde_json::Value`, `collected_at: u64`
- Define `EventSourceKind` enum: `Webhook`, `Cron`, `FileWatch`, `Manual`, `Plugin`
- Define `SignalSender` as `tokio::sync::mpsc::Sender<Signal>` (re-exported from roko-core)
- Define signal constants module: `signal_kinds::GITHUB_PUSH`, `GITHUB_PR_OPENED`, `GITHUB_PR_REVIEW`, `GITHUB_ISSUE_OPENED`, `SLACK_MESSAGE`, `SLACK_REACTION`, `CRON_TICK`, `FS_CHANGED`, `FS_CREATED`, `FS_MODIFIED`, `FS_DELETED`, `MANUAL_TRIGGER`
- Define `PluginManifest` struct: `name: String`, `version: String`, `event_sources: Vec<EventSourceKind>`, `feedback_services: Vec<String>`
- Add `PluginBuilder` with fluent API: `PluginBuilder::new("my-plugin").event_source(src).feedback_collector(coll).build()`

### Section 2C: Event-driven dispatch and webhooks (23 items)

Commits `9faf71b` through `b9d4f44`.

- Add `POST /webhooks/github` endpoint in roko-serve: read raw body bytes, verify `X-Hub-Signature-256` HMAC against `GITHUB_WEBHOOK_SECRET`, parse event type from `X-GitHub-Event` header, create `Signal` with kind from signal_kinds constants, enqueue to dispatch loop
- Add `POST /webhooks/slack` endpoint: handle Slack URL verification challenge, parse event type from `type` field, create Signal with `slack:*` kind
- Add `POST /webhooks/generic` endpoint: accept any JSON, create signal with `webhook:generic` kind
- Implement `DispatchLoop` -- central event routing engine: receives signals from webhook endpoints and event sources, matches against subscriptions, dispatches agents via templates
- Wire `SubscriptionRegistry`: loads from `roko.toml` `[[subscriptions]]` array, maintains active subscription state
- Implement concurrency tracking: `HashMap<subscription_id, AtomicUsize>` of active dispatches, respect `max_concurrent` limit
- Implement cooldown: `HashMap<subscription_id, Instant>` of last dispatch time, skip if within `cooldown_secs`
- Wire `dispatch_agent()` function: takes template + signal -> build system prompt from template, inject signal as context, run agent, record episode
- Add `[webhooks]` config section to `roko.toml`: `github.secret`, `github.enabled`, `slack.signing_secret`, `slack.enabled`
- Log all webhook events to `.roko/signals.jsonl` via normal signal write path
- Define `WebhookEpisodeMetadata` struct -- extended metadata for event-driven episodes: `trigger_signal_id`, `subscription_id`, `template_name`, `webhook_source`
- Wire episode logging in `dispatch_agent()`: wrap agent execution with episode start/end, record gate verdicts, cost, duration
- Track `ExternalAction`s during execution -- each tool call that mutates external state (GitHub PR, Slack message) is recorded for feedback collection
- Feed episode into cascade router: `cascade_router.record_outcome(&template_name, success, cost)`
- Feed episode into efficiency tracker: `efficiency.record_event(&template_name, tokens, cost, duration)`
- Implement `start_feedback_loop()` in `roko-serve/src/feedback.rs` -- background task that polls external services for reactions to agent actions
- Implement `collect_github_feedback()` -- for each ExternalAction with `service: "github"`, poll PR review status, comment reactions, merge status
- Implement `collect_slack_feedback()` -- for each ExternalAction with `service: "slack"`, poll message reactions, thread reply count, emoji sentiment
- Implement polling schedule: first 24h -> every 15min, days 2-7 -> every 6h, after 7 days -> stop polling
- Wire feedback into experiment metrics: if episode had an experiment variant, attribute feedback to that variant's success/failure counts
- Wire feedback into cascade router: positive feedback -> quality signal for model routing, negative -> penalty
- Record feedback signals in `.roko/signals.jsonl` with kind `feedback:{service}:{sentiment}`

### Section 2D: MCP servers (38 items)

**New crates: `roko-mcp-github`, `roko-mcp-slack`, `roko-mcp-scripts`, `roko-mcp-stdio`**

Commits `72f7695` through `f867f3b`.

**GitHub MCP server (`roko-mcp-github`):**
- Create crate with deps: `serde`, `serde_json`, `tokio`, `reqwest`
- Implement stdio JSON-RPC transport: read line-delimited JSON from stdin, write responses to stdout
- Implement `initialize` handler: return server capabilities, tool list
- Implement `tools/list` handler: return all tool definitions with JSON Schema for params
- Implement `tools/call` dispatcher: route to tool handlers by name
- Tool: `github_list_prs(owner, repo, state, per_page)` -- list PRs with title, number, state, author, updated_at
- Tool: `github_get_pr(owner, repo, number)` -- full PR details including diff stats, review status, labels
- Tool: `github_create_pr(owner, repo, title, body, head, base)` -- create PR, return URL + number
- Tool: `github_review_pr(owner, repo, number, body, event)` -- submit review (APPROVE/REQUEST_CHANGES/COMMENT)
- Tool: `github_comment_pr(owner, repo, number, body)` -- post comment on PR
- Tool: `github_merge_pr(owner, repo, number, merge_method)` -- merge PR (merge/squash/rebase)
- Tool: `github_list_issues(owner, repo, state, labels, per_page)` -- list issues with filters
- Tool: `github_create_issue(owner, repo, title, body, labels, assignees)` -- create issue
- Tool: `github_get_file(owner, repo, path, ref)` -- get file contents (base64 decoded)
- Tool: `github_search_code(query, owner, repo)` -- code search results
- Implement rate limiting: read `X-RateLimit-Remaining` header, sleep when < 10 remaining
- Auth: read `GITHUB_TOKEN` from env, return clear error if not set

**Slack MCP server (`roko-mcp-slack`):**
- Same stdio JSON-RPC transport (shared via `roko-mcp-stdio`)
- Tool: `slack_post_message(channel, text, thread_ts?)` -- post message, return ts for threading
- Tool: `slack_reply(channel, thread_ts, text)` -- reply in thread
- Tool: `slack_react(channel, ts, emoji)` -- add reaction
- Tool: `slack_list_channels(limit)` -- list channels agent has access to
- Tool: `slack_get_thread(channel, thread_ts)` -- get all messages in thread
- Tool: `slack_lookup_user(email_or_name)` -- find user ID
- Tool: `slack_dm(user_id, text)` -- send DM
- Rate limiting: respect `Retry-After` header on 429 responses
- Auth: read `SLACK_BOT_TOKEN` from env

**Script runner MCP server (`roko-mcp-scripts`):**
- Tool: `run_script(name, args?)` -- execute script from configured directory, stream stdout/stderr
- Tool: `list_scripts()` -- list available scripts with descriptions (read from `# description:` comment in script header)
- Sandboxing: set `timeout` (default 60s), working directory, env var allowlist
- Script discovery: on startup, scan configured directories (from env `ROKO_SCRIPTS_DIR` or `.roko/scripts/`)
- Security: scripts must be in the configured directory (no path traversal), executable bit required

**MCP lifecycle integration:**
- Wire auto-start: before dispatching an agent, check its template's `mcp_servers` list, start required servers
- Wire health check: after spawn, send `initialize` JSON-RPC call, retry 3x with 1s backoff
- Wire auto-stop: after agent completes, kill MCP server processes (or keep alive if shared)
- Wire MCP config generation: dynamically generate the `mcp_config.json` file from template + available servers

### Section 3A: Agent templates (11 items)

Commits `b02453a` through `10d4bd5`.

- Define `AgentTemplate` TOML schema in `crates/roko-serve/src/templates.rs`: `name`, `system_prompt`, `model`, `tools`, `mcp_servers`, `gate_pipeline`, `timeout_ms`, `max_tokens`, `experiment` (optional A/B config)
- Implement template loader: on `roko serve` startup, scan `.roko/templates/` for `*.toml` files, validate and register
- Wire template -> dispatch: when dispatch loop matches a subscription to a template, load template, build AgentRunConfig from its fields
- Wire template variable interpolation: `{{signal.payload.pull_request.number}}`, `{{signal.kind}}`, `{{now}}` variables expanded in system prompt and tool args
- Wire template validation: on load, check required fields (`name`, `system_prompt`), warn on missing optional fields, reject unknown fields
- Wire `ExperimentStore` into dispatch loop -- when template has `[experiment]` section, randomly assign variant, use variant's prompt/model overrides
- Implement variant-based prompt modification -- modify system prompt based on experiment variant (append variant suffix, swap model, adjust temperature)
- Record experiment assignment in episode metadata -- `experiment_variant` field in Episode for later analysis
- Wire feedback metrics into experiment outcomes -- after feedback collection, attribute positive/negative feedback to the variant
- Define concrete experiments for templates: table of template name, experiment name, variant A/B descriptions, metric to optimize

### Section 3B: Subscription system (4 items)

Commits `49cc5e8` through `b81d1c1`.

- Define `Subscription` struct in `roko-serve`: `id`, `name`, `signal_filter` (kind glob pattern), `template_name`, `max_concurrent`, `cooldown_secs`, `enabled`
- Implement `SubscriptionRegistry::find_matching(signal: &Signal) -> Vec<&Subscription>` -- glob matching on signal kind
- Wire subscription CRUD API endpoints: `GET /api/subscriptions` (list all), `POST /api/subscriptions` (create), `PUT /api/subscriptions/:id` (update), `DELETE /api/subscriptions/:id` (delete)
- Add CLI commands: `roko subscription list` (table of all subscriptions), `roko subscription add` (interactive), `roko subscription remove <id>`

### Section 3C: Event sources (11 items)

Commits `741394c` through `eeec158`.

- Implement `CronEventSource` struct implementing `EventSource` trait: parses cron expressions, fires signals at scheduled times, tracks next fire time
- Add `[[scheduler.cron]]` config sections in `roko.toml`: `name`, `expression` (cron syntax), `signal_kind` (signal to emit on fire)
- Wire signal emission: on cron fire, emit `Signal { kind: schedule.signal_kind }` to the dispatch loop via `SignalSender`
- Implement `FileWatchEventSource` struct implementing `EventSource`: uses `notify` crate for filesystem events, debounces within configurable window
- Wire debounce: batch file events within a 500ms window, emit single signal per batch
- Wire path filtering: support include/exclude glob patterns in config so watcher only fires for relevant paths
- Add `[[watcher.paths]]` config sections in `roko.toml`: `path`, `include` (glob list), `exclude` (glob list), `signal_kind`, `debounce_ms`
- Wire both `CronEventSource` and `FileWatchEventSource` into the dispatch loop: spawn as tokio tasks, feed signals to the central dispatcher
- Add `roko event-sources list` CLI command -- shows configured cron schedules + file watchers with status

### Section 4D: Secrets management (7 items)

Commits `4550eef` through `fb50d9a`.

- Wire `.env` file loading: on startup, load `.env` before config parse
- Wire `${VAR}` interpolation in `roko.toml` parser: substitute environment variables before TOML deserialization
- Wire secret masking in logs: tracing layer scans for patterns matching known secret env var values
- Wire secret masking in API responses: `/api/config` endpoint never returns raw secret values
- Document required env vars: add `REQUIRED_ENV` section to default `roko.toml`
- Add `roko config check-secrets` command: verify all referenced `${VAR}` tokens have values set
- Add `roko config set-secret NAME VALUE` command: appends to `.env` file

---

## Fixes

- **Agent registration** -- use string pubkey instead of byte array in JSON-RPC registration
- **Dashboard zeros** -- fork block, task stats, and agent stats were showing zeros because `ForkState` did not track the fork block and `mine_block()` used a frozen timestamp
- **Test failure** -- `agent_http_endpoints_via_full_server` had a response envelope key mismatch (`data` vs `items`)
- **Stale static files** -- added `no-cache, must-revalidate` Cache-Control for static dashboard files
- **Pheromone jitter** -- incremental DOM updates instead of full re-renders, canvas size caching, CSS containment (`contain: layout style paint`)
- **Knowledge graph too small** -- increased canvas dimensions, improved force simulation parameters
- **Agent topology clustering** -- increased repulsion force and link distance for readability
- **Router ordering bug** -- `.fallback_service(rpc_fallback)` was registered before `.nest("/api", api)` in `apps/mirage-rs/src/rpc.rs`, causing all `/api/*` GET requests to hit the JSON-RPC fallback and return "POST is required." Fixed by moving fallback registration after the `/api` nest.
- **Heartbeat block number always 0** -- All heartbeat handlers (REST GET, REST POST, JSON-RPC `chain_agentHeartbeat`) were passing hardcoded `block: 0` to the agent registry. This made `blocks_since` always 0 and `is_alive` always true. Fixed by adding a `BlockNumberFn` callback to `ApiState` that reads `MirageState.fork.local_block_number` from the live fork state. The JSON-RPC handler now reads `ctx.state.read().fork.local_block_number` and passes it through.
- **`ApiState` test construction** -- Added `ApiState::new()` constructor so tests don't need to manually construct all fields. Updated all 5 test construction sites in `tests/http_api.rs`.
- **Chain toggle defaults** -- `enable_hdc`, `enable_knowledge`, and `enable_stigmergy` CLI flags in `apps/mirage-rs/src/main.rs` defaulted to `false`. Running `cargo run -p mirage-rs --features roko` without explicit flags skipped `start_rpc_server_with_chain()` entirely -- the API router was never registered, making all `/api/*` endpoints return "POST is required." Changed defaults to `true`.

---

## Closes / addresses

- Issue #1: HTTP API for mirage-rs (router bug fixed, all endpoints now accessible)
- Issue #5: agent registry with HTTP/RPC/WS endpoints (heartbeat block number fixed)
- Issue #6: one-click deploy (partially -- deployment endpoints exist in roko serve, spawn endpoint pending)

---

## Test coverage

| Suite | Count | What |
|-------|-------|------|
| Library unit tests | 287 | Core crate tests across all 18 crates |
| Integration tests | 13 | Executor lifecycle, safety, learning, orchestrator |
| HTTP API tests | 37 | Full REST endpoint coverage (mirage-rs) |
| Roko bridge tests | 6 | Chain substrate, HDC substrate, simulation gate |
| End-to-end tests | 4 | CLI -> orchestration -> gate -> persist |
| **Total** | **347** | |

---

## How to test

```bash
# Build everything
cd /Users/will/dev/nunchi/roko/roko
rustup update stable  # need 1.91+
cargo build --workspace

# Run all tests
cargo test --workspace

# Run clippy
cargo clippy --workspace --no-deps -- -D warnings

# Start mirage-rs dashboard
cargo run -p mirage-rs --features chain,roko

# Run 20-agent simulation against the dashboard
cargo run -p mirage-rs --features chain,roko --example agent_simulation -- \
    --rpc-url http://127.0.0.1:8545

# Start roko serve API
cargo run -p roko-cli -- serve

# Execute the self-hosting loop
cargo run -p roko-cli -- prd idea "test idea"
cargo run -p roko-cli -- prd list
cargo run -p roko-cli -- plan run plans/
```

---

## What comes after this

With this PR merged, the self-hosting loop is wired end-to-end, the event-driven dispatch system is operational, and the MCP tool ecosystem is in place. Remaining work:

1. **End-to-end testing** -- run the full self-hosting loop (prd idea -> draft -> plan -> execute -> gate -> learn -> iterate) and the event-driven loop (webhook -> subscription -> template -> dispatch -> feedback) under real conditions
2. **TUI polish** -- the ratatui TUI is wired but needs visual refinement and live data testing
3. **Feedback loop closure** -- re-planning strategies and feedback collection are implemented but need end-to-end validation
4. **Agent spawn endpoint** -- add `POST /api/agents/spawn` to mirage-rs for in-process agent creation from templates (issue #6)
5. **MCP server packaging** -- build and publish Docker images for the GitHub, Slack, and Scripts MCP servers

After those items, roko can fully self-host: receive external signals (GitHub webhooks, Slack events, cron schedules, file changes), match them to subscriptions, dispatch agents with templates, execute plans, validate with gates, collect feedback from external services, learn from outcomes, and iterate without human intervention.
