# Roko End-to-End Wiring Audit

> Date: 2026-08-19
> Scope: Trace every major CLI command from `main.rs` through dispatch to actual execution.
> Method: Source code inspection of dispatch paths, handler implementations, and data flow.
> Verdict: **Most core commands are genuinely wired.** A few have hidden prerequisites that
> would cause failures in practice. The system is far more real than the typical "built but
> not wired" pattern -- but several paths require external services or specific state to succeed.

---

## 1. `roko plan run plans/ --engine runner-v2`

**Verdict: GENUINELY WIRED -- the most thoroughly wired command in the system.**

### Trace

1. `main.rs` dispatches `Command::Plan { cmd }` to `commands::plan::cmd_plan()`
2. `PlanCmd::Run` branch validates plans, acquires workspace lock
3. For `--engine runner-v2` (default): loads config via `RokoBootstrap`, pre-flights provider
   and gate dependencies, initializes metrics, loads plans via `runner::plan_loader::load_plans()`
4. Scaffolds missing crates referenced by tasks
5. Calls `runner::event_loop::run()` with loaded plans, config, state hub, and cancellation token

### What actually happens in `event_loop::run()`

- Loads/validates `RokoConfig` with model normalization
- Sets up telemetry sink, GitHub workflow integration, conductor ring
- Initializes extensions, gate thresholds, knowledge store, run ledger
- Pre-run resource maintenance (JSONL rotation, worktree cleanup, disk check)
- Constructs `TaskDag` from loaded plans with dependency ordering
- Runs the main `tokio::select!` event loop:
  - Agent dispatch with full prompt assembly (9-layer SystemPromptBuilder)
  - Gate pipeline execution (compile, test, clippy, diff)
  - Snapshot persistence to `.roko/state/state-snapshot.json`
  - Episode logging to `.roko/episodes.jsonl`
  - Efficiency event recording
  - Cascade router learning updates
  - Gate threshold EMA updates
  - GitHub PR/comment workflow
  - Worktree management per task
  - Replan on gate failure (when configured)

### Prerequisites that must be met

- `.roko/` directory must exist (or `roko init` first)
- A valid `roko.toml` with a configured provider (or `--model` override)
- The `plans/` directory must contain valid `tasks.toml` files
- Git repo must exist (auto-initialized if missing)
- For gate execution: `cargo` must be on `$PATH`

### Gaps

- Resume from `state-snapshot.json` works but has had historical bugs with stale state
  (four blockers found in 2026-08-13 dogfood, regression fixes landed)
- Budget enforcement reports can exceed reservations because providers expose exact cost
  only after completion

### Rating: 9/10 -- deeply wired with real data flow

---

## 2. `roko dashboard`

**Verdict: GENUINELY WIRED -- both interactive and text modes work.**

### Trace

1. `main.rs` dispatches `Command::Dashboard` to `commands::dashboard::cmd_dashboard()`
2. If stdout is a TTY and not text mode: launches `App::new_with_page()` and calls `app.run()`
3. If text mode or not a TTY: calls `render_dashboard_text()` which loads
   `CommandDashboardSnapshot` and renders via `DashboardScaffold`

### Interactive TUI path (`App::run()`)

- Enters crossterm alternate screen with raw mode
- Sets up panic hook for terminal cleanup
- Runs main loop at 60fps with:
  - F1-F7 tab navigation (Mori-style)
  - `EventHandler` for keyboard/mouse/resize events
  - `FsWatchHandle` for filesystem change notifications
  - `GitWatchHandle` for git state changes
  - `TuiState` with agents, plans, navigation, modals, scroll
  - PostFX pipeline and atmosphere animations
  - WebSocket streaming client for live agent output

### Text mode path

- Loads snapshot from disk (`.roko/state/`, learn files, episodes)
- Renders pages as plain text tables
- Pages: Health, Trends, plus all PageId variants

### What it actually shows

- Plan status and progress from snapshot state
- Agent status from `.roko/runtime/agents.json`
- Episode history from `.roko/episodes.jsonl`
- Learning metrics from `.roko/learn/`
- System process information via `sysinfo`

### Prerequisites

- `.roko/` directory with some state files to display
- For interactive: a terminal that supports crossterm/ratatui

### Gaps

- Named-surface TUI rendering (E37) is documented as a product residual
- Some pages show "no data" when there are no episodes/plans yet (correct behavior, not a bug)

### Rating: 8/10 -- genuinely interactive, shows real data when available

---

## 3. `roko run "<prompt>"`

**Verdict: GENUINELY WIRED -- routes through WorkflowEngine or `cmd_do`.**

### Trace

1. `main.rs` dispatches `Command::Run` to either:
   - `commands::do_cmd::cmd_do()` (when no `--serve`/`--share`/`--max-retries`)
   - `commands::util::cmd_run()` (when those flags are set)
2. The `cmd_do` path classifies complexity via `ScopeResolver::resolve()`:
   - **Trivial/Simple**: direct single-agent `WorkflowEngine` run via `run_simple_path()`
   - **Standard**: generates a plan from the prompt, then executes it
   - **Complex**: creates PRD -> draft -> generates plan -> executes plan
3. `cmd_run` path: resolves config, optionally starts serve, runs `WorkflowEngine`

### What `run_simple_path` does

- Resolves model selection (cascade router, config, overrides)
- Builds `EffectServices` via `ServiceFactory`
- Creates `WorkflowEngine` with the resolved pipeline config
- Runs prompt through: compose -> agent dispatch -> gate validation -> persist
- Bridges events to `StateHub` for TUI/SSE consumers
- Records episode to `.roko/episodes.jsonl`

### Prerequisites

- A configured agent provider (not the default `cat` command)
- API keys for the selected provider (Anthropic, OpenAI, etc.)
- `.roko/` directory exists

### What would fail

- If no provider is configured: clear error "WorkflowEngine refused to run with the default
  `cat` agent"
- If API keys are missing: provider-specific auth error
- The scope classifier (`ScopeResolver::resolve()`) calls an LLM to classify complexity,
  so even classifying the prompt requires a working provider

### Rating: 8/10 -- genuinely wired, but requires working LLM provider

---

## 4. `roko status`

**Verdict: GENUINELY WIRED -- reads real data from disk.**

### Trace

1. `main.rs` dispatches `Command::Status` to `commands::status::cmd_status()` (alias for
   `commands::util::cmd_status()`)
2. Two modes:
   - `--quick`: 3-line summary (provider detection, learning data presence, workspace check)
   - Full: opens `FileSubstrate`, queries all signals, reads episodes, optionally computes
     C-Factor

### What it actually reports (full mode)

- Workdir path
- Session ID (if any)
- Daemon running status
- Signal count from `.roko/engrams.jsonl` (via `FileSubstrate::open()` + `query()`)
- Episode count from `.roko/episodes.jsonl`
- Last episode pass/fail
- Total cost and today's cost from the costs log
- Process session ledger summary (for restart/resume diagnosis)
- C-Factor metrics (with `--cfactor` flag)
- Named surface inventory (with `--surfaces` flag)

### What it actually reads

```
.roko/engrams.jsonl        -- signal store
.roko/episodes.jsonl       -- episode log
.roko/learn/costs.jsonl    -- cost tracking
.roko/learn/cascade-router.json -- learning state presence
roko.toml                  -- workspace config
```

### Prerequisites

- `.roko/` directory must exist (or you get "open substrate" error)
- For meaningful output: at least one signal or episode must have been recorded

### Rating: 9/10 -- straightforward file reads, always works when workspace exists

---

## 5. `roko doctor`

**Verdict: GENUINELY WIRED -- runs ~20 real diagnostic checks.**

### Trace

1. `main.rs` dispatches `Command::Doctor` to `commands::util::cmd_doctor()` which calls
   `doctor::run_doctor()`
2. Runs a sequential battery of checks, each returning a `DoctorCheck` with status
   (Ok/Warn/Fail/Skipped)

### Actual checks performed

| Check | What it does |
|---|---|
| `check_workdir` | Verifies working directory exists |
| `check_config_presence` | Looks for `roko.toml` in project/global paths |
| `check_layout_basics` | Verifies `.roko/` directory structure |
| `check_claude_cli` | Checks if `claude` CLI is on PATH |
| `check_configured_provider_keys` | Validates API keys for configured providers |
| `check_provider_usable` | Tests if at least one provider is functional |
| `check_available_providers` | Enumerates configured providers |
| `check_default_model_configured` | Verifies default model is set |
| `check_rust_version` | Checks `rustc` version (needs 1.91+) |
| `check_node_version` | Checks `node` availability |
| `check_serve_auth` | Validates serve auth configuration |
| `check_serve_health` | HTTP GET to `serve_url/api/health` (2s timeout) |
| `check_dead_conductor_config` | Warns about deprecated conductor settings |
| `check_v2_abstractions` | Checks for v2 architecture compatibility |
| `check_state_layout_audit` | Validates `.roko/state/` directory structure |
| `check_config_freshness` | Checks config staleness timestamps |
| `check_harness_providers` | Validates provider harness configuration |
| `check_mcp_allowlist` | Validates MCP tool allowlist |
| `check_orphaned_tmp_files` | Detects orphaned temporary files |
| `check_plans_dir_conflict` | Checks for conflicting plan directory layouts |
| `check_disk_health` | Reports disk space, stale targets, worktrees, oversized JSONL |
| `check_target_staleness` | Checks `target/` directory age |

### Subcommand: `roko doctor disk`

- Dedicated disk capacity report
- Checks free space, stale build targets, worktree storage, oversized log files
- Actionable fix suggestions for each finding

### Output formats

- Human-readable text with `[ok]`/`[warn]`/`[fail]` labels and fix suggestions
- JSON (with `--json`) for scripting

### Rating: 10/10 -- entirely self-contained, requires no external services

---

## 6. `roko prd idea/draft/plan`

**Verdict: GENUINELY WIRED -- all three stages work, but draft/plan require an LLM provider.**

### `roko prd idea "<text>"`

- Appends text to `.roko/prd/ideas.md`
- Pure file I/O, always succeeds
- **Rating: 10/10**

### `roko prd draft new "<title>"`

- Creates slug from title
- Resolves effective model (prefers "scribe" role)
- Pre-flights provider availability
- Writes scaffold to `.roko/prd/drafts/<slug>.md`
- Runs agent to fill the scaffold with real PRD content
- Includes repository grounding (loads `roko.toml`, `Cargo.toml`, crate summaries)
- Persists capture episode
- **Rating: 8/10** -- works but requires working LLM provider

### `roko prd plan <slug>`

- Locates published/draft PRD by slug
- Resolves effective model (prefers "strategist" role)
- Pre-flights provider
- Calls `generate_plan_from_prd_with_model()` which:
  - Reads PRD content, extracts metadata
  - Resolves template kind from PRD frontmatter
  - Builds generator system prompt with template guidance
  - Loads repository context (crate structure, imports, existing plans)
  - Queries knowledge store for relevant context
  - Runs agent to generate `tasks.toml`
  - Validates generated TOML structure
  - On validation failure: escalates to next-tier model (haiku -> sonnet -> opus)
  - Writes plan to `plans/<slug>/tasks.toml`
- **Rating: 8/10** -- solid wiring with escalation, but LLM-dependent

### `roko prd list` / `roko prd status` / `roko prd consolidate`

- `list`: scans `.roko/prd/` directories, pure file I/O
- `status`: reads PRDs and cross-references with plans
- `consolidate`: reads all PRDs + ideas, runs agent for gap analysis
- **Rating: 9/10 for list/status, 7/10 for consolidate (agent-dependent)**

---

## 7. `roko serve`

**Verdict: GENUINELY WIRED -- starts a real axum HTTP server with real route handlers.**

### Trace

1. `main.rs` dispatches `Command::Serve` to a block that:
   - Resolves workdir, acquires workspace lock
   - Loads config via `RokoBootstrap`
   - Creates `RokoCliRuntime` with `StateHub` and `MetricRegistry`
   - Prepares workspace extensions
   - Builds `RokoConfig` from bootstrap
   - Constructs `ServerBuildConfig` with runtime, config, state hub, metrics
   - Calls `ServerBuilder::new(config).start_background()` or foreground start

### What `build_router()` actually mounts

The router merges 40+ route modules plus WebSocket and relay handlers:

```
status, jobs, heartbeats, plans, prds, run, runs, research,
subscriptions, templates, aggregator, arenas, meta, agents,
learning, marketplace, defi, registries, config, deployments,
diagnosis, integrations, projections, neuro, dream, event_ingest,
extensions, gateway, chain, connectors, feeds, recipes, groups,
auth, secrets, vision_loop, team, bench, swe_bench, triggers,
workflows, workspaces, shared_runs, webhooks, providers, models,
routing, sse, rpc_proxy, ws
```

Plus unauthenticated routes:
- `/health` -- liveness probe
- `/ready` -- readiness probe
- `/metrics` -- Prometheus scrape endpoint
- Public webhook endpoints
- Public shared-run reader

### What the routes serve

Each route module contains real axum handlers that read from `AppState`:
- `AppState` holds: `StateHub`, config, `ArenaRegistry`, `PulseBus`, metrics,
  `SecretScrubber`, `SseAdapter`, and workspace paths
- Routes read from disk (`.roko/` state files, JSONL logs) and in-memory state
- SSE streaming delivers live dashboard events

### Authentication

- When `serve.auth.enabled = true`: API key middleware with RBAC permission checking
- Terminal routes always require auth when enabled
- Rate limiting: per-key and global backstop

### Gaps

- Many "stub" routes (marketplace, defi, registries, arenas) return 501 with structured
  JSON bodies -- they are wired but not backed by real services
- The ~317 route count is accurate for _registered_ routes, but not all serve production data

### Rating: 8/10 -- real HTTP server with real handlers, but many routes are stub/501

---

## 8. `roko agent start/stop/list`

**Verdict: GENUINELY WIRED -- process lifecycle management works.**

### `roko agent list`

- Scans `.roko/agents/` for manifest directories
- Reads `manifest.toml` from each agent directory
- Cross-references with runtime entries at `.roko/runtime/agents.json`
- Checks process liveness via `sysinfo` PID check
- Displays name, domain, status (running/stopped), PID, bind address
- **Rating: 9/10**

### `roko agent create --name X --domain Y`

- Validates manifest fields
- Resolves domain presets (coding, research, chain, general)
- Writes `AgentExtendedManifest` TOML to `.roko/agents/<name>/manifest.toml`
- Optional auto-registration with running `roko-serve`
- **Rating: 9/10**

### `roko agent start --name X`

- Checks manifest exists and agent is not deleted
- Checks if already running (stale entries cleaned up)
- Spawns `roko agent serve --agent-id <name> --bind <bind>` as a **detached child process**
- Registers PID in `.roko/runtime/agents.json` and process registry
- The spawned process runs `roko-agent-server` with real LLM dispatch
- **Rating: 8/10** -- works, but the spawned server needs a configured LLM provider

### `roko agent stop --name X`

- Looks up PID from runtime entries
- Sends SIGTERM (or SIGKILL with `--force`)
- Waits up to 5 seconds for exit
- Cleans up runtime entry and unregisters PID
- **Rating: 9/10**

### `roko agent chat --agent X`

- With `--provider`: direct API chat via `run_direct_provider_chat()`
- Without: connects to running agent serve instance or `roko-serve`
- Interactive REPL loop
- **Rating: 7/10** -- works but requires either running agent or serve instance

---

## 9. `roko knowledge query/dream`

**Verdict: GENUINELY WIRED -- reads/writes real data, but "knowledge" requires prior ingestion.**

### `roko knowledge query "<topic>"`

- Opens `KnowledgeStore::for_workdir()` (reads `.roko/neuro/knowledge.jsonl`)
- Calls `store.query(&topic, 10)` which does HDC vector similarity search
- Returns matching entries with topic, kind, confidence, tier, timestamp
- JSON output available with `--json`
- **Rating: 9/10** -- works, but returns empty if no knowledge has been ingested

### `roko knowledge stats`

- Opens `KnowledgeStore`, reports counts by tier and kind
- Pure file read
- **Rating: 10/10**

### `roko knowledge gc`

- Runs garbage collection on the knowledge store
- Removes low-confidence/expired entries
- **Rating: 10/10**

### `roko knowledge dream run`

- Builds `DreamRunner` with workspace config
- Calls `runner.consolidate_now()` which:
  - Reads recent episodes from `.roko/episodes.jsonl`
  - Clusters episode data
  - Generates knowledge entries, playbooks, and hypotheses
  - Updates daimon affect state on failure
  - Refreshes C-Factor snapshot
- **Rating: 7/10** -- wired but requires episodes to consolidate (empty input = empty output)

### `roko knowledge dream journal/archive`

- Journal: reads from `DreamJournal` JSONL file
- Archive: reads from dream archive entries
- Both are pure file reads
- **Rating: 9/10**

### `roko knowledge backup/restore`

- Backup: exports knowledge with genomic bottleneck (top-N filtering)
- Restore: imports with decay factor applied to confidence scores
- Both use the `KnowledgeStore` import/export API
- **Rating: 9/10**

### `roko knowledge sync <peer>`

- Calls knowledge store mesh sync (HTTP-based)
- **Rating: 5/10** -- wired but requires a reachable peer with compatible knowledge store

---

## 10. `roko learn all`

**Verdict: GENUINELY WIRED -- reads real learning state files from disk.**

### Trace

1. `main.rs` dispatches `Command::Learn` to `commands::learn::dispatch_learn()`
2. `cmd_learn(workdir, "all")` calls:
   - `print_learn_router()` -- reads `.roko/learn/cascade-router.json`
   - `print_learn_experiments()` -- reads `.roko/learn/experiments.json`
   - `print_learn_efficiency()` -- reads `.roko/learn/efficiency.jsonl`
   - `print_learn_episodes()` -- reads `.roko/episodes.jsonl`
   - `print_learn_gate_thresholds()` -- reads `.roko/learn/gate-thresholds.json`
   - `print_learn_knowledge()` -- reads `.roko/neuro/knowledge.jsonl`

### What it reports

| Subsystem | Source file | What it shows |
|---|---|---|
| Router | `cascade-router.json` | Model routing weights, observation counts |
| Experiments | `experiments.json` | Active A/B experiments, arm assignments |
| Efficiency | `efficiency.jsonl` | Token usage, cost per task, role profiles |
| Episodes | `episodes.jsonl` | Recent episodes, pass/fail, models used |
| Gate thresholds | `gate-thresholds.json` | Adaptive EMA thresholds per rung |
| Knowledge | `knowledge.jsonl` | Entry counts by tier and kind |

### `roko learn tune gates/routing/budget`

- Displays current adaptive thresholds or routing state
- `--dry-run` mode for preview without changes
- Reads the same files as `learn all` but with focused views

### Prerequisites

- `.roko/learn/` directory with at least one data file
- Returns "no data" messages when files don't exist (graceful, not an error)

### Rating: 9/10 -- pure file reads, always works, shows whatever data exists

---

## Summary: Wiring Status Matrix

| Command | Wired? | Works standalone? | External deps? | Rating |
|---|---|---|---|---|
| `roko plan run` | Yes | Yes (needs provider) | LLM API, git | 9/10 |
| `roko dashboard` | Yes | Yes | None (TUI) or serve (live) | 8/10 |
| `roko run "<prompt>"` | Yes | Yes (needs provider) | LLM API | 8/10 |
| `roko status` | Yes | Yes | None | 9/10 |
| `roko doctor` | Yes | Yes | None (serve health optional) | 10/10 |
| `roko prd idea` | Yes | Yes | None | 10/10 |
| `roko prd draft new` | Yes | Yes (needs provider) | LLM API | 8/10 |
| `roko prd plan` | Yes | Yes (needs provider) | LLM API | 8/10 |
| `roko serve` | Yes | Yes | None | 8/10 |
| `roko agent create` | Yes | Yes | None | 9/10 |
| `roko agent start` | Yes | Yes (needs provider) | LLM API | 8/10 |
| `roko agent stop` | Yes | Yes | None | 9/10 |
| `roko agent list` | Yes | Yes | None | 9/10 |
| `roko knowledge query` | Yes | Yes | None | 9/10 |
| `roko knowledge dream` | Yes | Yes (needs episodes) | None | 7/10 |
| `roko learn all` | Yes | Yes | None | 9/10 |

---

## Key Findings

### What is genuinely wired (and impressive)

1. **The plan-execute-gate-persist loop is real.** `event_loop::run()` is a ~2000-line async
   event loop with proper `tokio::select!` over agent events, gate completions, periodic
   flushes, cancellation, and deadline tracking. This is not a stub.

2. **The dispatch pipeline is deep.** `dispatch/mod.rs` + `dispatch/factory.rs` +
   `dispatch/prompt_builder.rs` + `dispatch/model_routing.rs` compose a real prompt assembly
   and model routing pipeline. The 9-layer SystemPromptBuilder, cascade router, playbook
   injection, error pattern injection, and knowledge-informed routing all feed into actual
   agent invocations.

3. **The TUI is a real ratatui application.** The `App::run()` method enters alternate screen,
   runs a 60fps event loop with keyboard/mouse handling, tab navigation, modals, and
   filesystem watchers. This is not a placeholder.

4. **Learning is read/write.** The cascade router, gate thresholds, efficiency log, and
   episode logger all write JSONL/JSON files during execution and read them back for routing
   decisions. The feedback loop is closed.

5. **The HTTP server is a real axum application.** `build_router()` mounts 40+ route modules
   with middleware for auth, RBAC, rate limiting, CORS, and secret scrubbing.

### What has hidden failure modes

1. **Provider configuration is the gating factor.** Any command that dispatches an agent
   (run, plan run, prd draft, prd plan, agent start, knowledge dream) requires a configured
   and authenticated LLM provider. The error messages are clear, but a new user running
   `roko run "hello"` without configuration will get a refusal.

2. **Scope classification requires an LLM.** `roko do` calls `ScopeResolver::resolve()` which
   itself makes an LLM call to classify prompt complexity. If the provider is misconfigured,
   even the classification step fails. This means `roko do` has two LLM dependencies: one for
   classification and one for execution.

3. **Dream consolidation requires prior episodes.** `roko knowledge dream run` runs but
   produces empty output if no episodes have been recorded. It needs a prior `plan run` or
   `run` to have generated episodes.

4. **Many serve routes return 501.** The marketplace, DeFi, registries, and arenas route
   modules are structurally wired but return 501 with "product work" messages. They are
   honest stubs, not fake success responses.

5. **Knowledge sync requires a peer.** `roko knowledge sync <peer>` is wired but requires a
   reachable peer instance with a compatible knowledge store. This is an integration boundary,
   not a wiring gap.

### "Built but not wired" patterns -- notably ABSENT

The classic antipattern in this codebase was "code exists but is never called from the
runtime." This audit found very few instances of that pattern in the 10 audited commands.
Every command traced from `main.rs` through `dispatch_subcommand()` reaches real handler
code that reads/writes real files or makes real HTTP/process calls.

The remaining "built but not wired" items are:
- **Named-surface TUI rendering** (E37 residual) -- backend data is served but the TUI
  does not render all surface types
- **Native Agent-to-E33 telemetry publication** -- the telemetry system is complete but
  native agent observation publication is separate scope
- **WIT/Component model hostcalls** (E32) -- plugin system is wired for WASM hooks but
  Component Model Store/Bus hostcalls are open

### Overall assessment

This is a genuinely functional system, not a documentation exercise. The core self-hosting
loop (prd -> plan -> execute -> gate -> learn -> iterate) works end-to-end when an LLM
provider is configured. The main risk is not "things are stubs" but rather "the dependency
graph for a successful execution is deep" (provider config + API keys + git + cargo + disk
space + valid plans).

The codebase has moved decisively past the "built but not wired" phase into "wired but
needs operational maturity" territory.

---

## Files examined

| File | Role |
|---|---|
| `crates/roko-cli/src/main.rs` | CLI entry point, clap dispatch |
| `crates/roko-cli/src/lib.rs` | Module declarations |
| `crates/roko-cli/src/run.rs` | `roko run` / WorkflowEngine path |
| `crates/roko-cli/src/runner/event_loop.rs` | Runner-v2 main event loop |
| `crates/roko-cli/src/runner/mod.rs` | Runner module structure |
| `crates/roko-cli/src/status.rs` | SessionStatus type |
| `crates/roko-cli/src/doctor.rs` | Doctor diagnostic checks |
| `crates/roko-cli/src/prd.rs` | PRD lifecycle implementation |
| `crates/roko-cli/src/dispatch/mod.rs` | Dispatch pipeline facade |
| `crates/roko-cli/src/tui/mod.rs` | TUI module structure |
| `crates/roko-cli/src/tui/app.rs` | Interactive TUI application |
| `crates/roko-cli/src/agent_serve.rs` | Agent lifecycle commands |
| `crates/roko-cli/src/commands/*.rs` | Command handler implementations |
| `crates/roko-serve/src/routes/mod.rs` | HTTP router assembly |
| `crates/roko-serve/src/lib.rs` | Server startup |
| `.roko/GAPS.md` | Known gaps tracker |
