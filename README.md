# Roko

Roko is a Rust toolkit for building agents that build themselves.

Point it at a codebase, describe what you want, and roko handles the rest: it composes prompts, dispatches LLM agents, verifies output with compilation and test gates, persists results as content-addressed signals, and learns from outcomes to get better over time. The core loop is observe, plan, execute, verify, learn, repeat.

35 workspace members. ~800K lines of Rust. 9,900+ tests.

Current programme rollup (2026-08-17): all 48 epics are accepted in the canonical
roll-up, with no partial or greenfield epics and 0 remaining epic-manifest tasks. See
[`.roko/GAPS.md`](.roko/GAPS.md) for the human-readable source of truth and
[`tmp/status-quo/MASTER-EXECUTION-CHECKLIST.md`](tmp/status-quo/MASTER-EXECUTION-CHECKLIST.md)
for task-level evidence. The raw E01-E48 metadata count is 393/447; it is not a
completion score because older accepted manifests remain unreconciled.

The latest residual pass completed R01-R04: one supervised HTTP JSON connector; a bounded
canonical-envelope relay with supervised replay and restart-durable exact-room subscription
execution; an authorized, restart-safe local arena lifecycle with external scoring evidence;
and an owner-scoped meta-agent lifecycle with non-widening authority, bounded lineage, exact
five-head safety evidence, and single-use arena acceptance. The preceding closure pass
completed E29's portable contracts, E38's marketplace contracts/stubs, E39's local registry
state machines and critical-path routes/indexer, E40's local arena model, E41's local DeFi
primitives, and P28 inline image input. These closures do not claim additional connector
transports, startup discovery, MCP/A2A/x402/finality execution, a durable marketplace,
deployed registry contracts, the arena eval/flywheel/on-chain system, DeFi risk/venue
execution, or autonomous Loop 4/ADAS/HGM generation. Those boundaries remain explicit in
the gaps document.

The machine-derived executable queue is **124/124 tasks complete (100%)** across 30
plans; all 30 executable plans are complete. `architecture-production-residuals` is 4/4
after the R01-R04 closure above. P34 is 4/4: at its historical 2026-08-16 checkpoint,
formatting, workspace check, strict default-target/default-feature clippy, the default
workspace test/doctest run, optimized release build, and release CLI smoke/plan-validation
gates passed. Subsequent dirty-tree changes require fresh scoped and final verification. The master checklist's
broader raw census is 211 done / 11 partial / 20 unchecked markers; those markers include
repeated controls, docs/dogfood proof, and product work rather than only executable plans.

Documentation integrity is now bounded and enforced for the maintained operator corpus:
the exact 109/746/637 status, source-registry, and manifest contracts are checked, local
paths/anchors pass, and `plans/INDEX.md` has a deterministic non-mutating CI drift check. The
DOC reconciliation queue is 3/71 done. Direct TUI/show/API/workspace Runner projections share
the canonical verified snapshot loader; StateHub overlay/SSE cursor atomicity and a single
immutable resume generation remain open, so broad cross-surface projection closure is partial.

The preceding closure pass completed E24 advanced memory, E26's live inference gateway,
E27 continuous feeds and recipes, and E28 persisted agent-group coordination. The pass
before that completed E25 advanced learning loops, E36 paid-feed payments,
E42 configuration evolution, and E44 cross-cut functors with a live gate-failure cascade.
Earlier closure passes completed E31 watcher-to-raw-EVM finality/reorg ingress;
completed E32 through signed semantic-version dependency graphs, all 23 current WASM
hooks, and authenticated native Gemini CLI MCP dispatch; completed E33 with 39/39
production variants, including the final six through a durable registered-agent lifecycle
ingress; completed E23 with EFE/goal ownership plus live energy- and phase-aware dispatch;
completed strict E34 with monotonic trust-origin IFC, exact capability intersection,
persistent quarantine, and mandatory audited production hooks; completed E37's typed surface
contracts and five dedicated projection routes; added automatic provider final-output
screening plus host-visible tool-result screening, canonical-workspace durable quarantine,
provider isolation, tool cooldown/isolation, and incident linking; and made Hot Graphs restart-durable
with fingerprinted Activity, tick-state, and budget checkpoints. Graph plan execution
also enforces and reports actual per-plan provider cost, while daemon Dreams own adaptive
idle, cron, and episode-count scheduling. Native Agent-to-E33 telemetry publication,
provider-internal security visibility, surface rendering, Component hostcalls, Runner-v2
Graph parity, additional connector/protocol integrations, durable market services, deployed
chain adapters, arena eval/flywheel/on-chain execution, DeFi risk/venue integration, and
autonomous structural adaptation stay as separately identified product/roadmap residuals in
the gaps document.

## Quick start

```bash
cargo install --path crates/roko-cli
roko init my-project && cd my-project
roko run "add a health check endpoint to the API"
```

`roko init` detects your project type, sets up gates (cargo check for Rust, tsc for TypeScript, go build for Go), and writes a working `roko.toml`. `roko run` does everything else.

## How it works

### One-shot execution

The fastest path. One command, full pipeline.

```bash
roko run "refactor the auth module to use JWT"
```

Roko composes a prompt from your codebase context, calls the configured LLM agent, runs the output through compile/test/lint gates, and persists the result as a signal. If gates fail, it retries with escalating models.

### Full planning pipeline

For larger work that spans multiple tasks.

```bash
# 1. Capture what you want to build
roko prd idea "Add user authentication with OAuth2"

# 2. Research the topic (optional -- uses Perplexity for web-grounded citations)
roko research topic "OAuth2 best practices in Rust"

# 3. Generate a detailed PRD (agent-assisted)
roko prd draft new "oauth2-auth"

# 4. Create an implementation plan with tasks
roko prd plan oauth2-auth

# 5. Execute the plan (agents work in parallel, gates verify each task, state persists)
roko plan run plans/ --engine runner-v2

# 6. Resume if interrupted
roko plan run plans/ --engine runner-v2 --resume-plan

# 7. Watch progress
roko dashboard
```

Each task in the plan runs through its own agent loop with independent gate verification. Failed tasks feed back into the planner for re-decomposition.

Workspace-mutating commands use a single-writer lock, and plan resume is scoped by plan ID. A snapshot from an unrelated plan is ignored as a clean start; overlapping snapshots still receive strict task-fingerprint validation. Append-only runtime logs rotate at the configured `[resources].log_rotation_max_mb` threshold (100 MB by default), with archive retention and disk-health reporting handled by the resource lifecycle.

Long-running servers sample the shared metric registry every 30 seconds into
rotation-bounded telemetry JSONL. The configurable `[cold_storage]` timer moves aged
signals into deduplicated cold archives and removes them from hot storage only after a
successful archive write.

The telemetry contract also exposes seven typed Lens projections through
`/api/projections/{name}` and current materialized values through
`/api/statehub/{projection_id}`. StateHub retains bounded projection history in a
restart-durable companion JSONL log; `/api/statehub/{projection_id}/history` supports
version/time filters and checked `ms`/`s`/`m`/`h`/`d` resolution coalescing. Graph files
may attach any of the 11 built-in Lenses with top-level `[[lenses]]`;
`roko graph run` routes matching lifecycle evidence through raw stacks and ordered
derived chains, then prints resulting versioned projections. Lens state and cardinality
are bounded, configuration fails closed, and unavailable event metrics are not inferred.
Typed Workbench, Inbox, Canvas, Minimap, and Autonomy views are also available at
`/api/projections/workbench`, `/api/projections/inbox`, `/api/projections/canvas`,
`/api/projections/minimap`, and `/api/projections/autonomy`; their remaining TUI rendering
and runtime-source limitations are documented rather than filled with synthetic data.

Declarative triggers map event payloads into root-Cell Signals, enforce Space partition,
Graph visibility, and capability intersections, publish lifecycle evidence on the shared
Pulse Bus, and expose durable history through CLI and HTTP. Declarative plugin binaries
require kernel confinement (macOS Seatbelt or Linux firejail/seccomp); Claude/Codex CLI
providers reach canonical plugin handlers through an authenticated, contract-scoped
loopback MCP bridge, while unsupported adapters fail closed.

Use `roko doctor disk` for a read-only report of free space, stale Rust targets, orphaned
worktrees, oversized JSONL logs, and aggregate workspace storage.

### Implicit prompt mode

If no subcommand matches, roko treats the argument as a prompt:

```bash
roko "fix the bug in auth.rs"
```

This is equivalent to `roko run "fix the bug in auth.rs"`. The shortest path from thought to execution.

## Dashboard

`roko dashboard` launches an interactive terminal UI built on ratatui with the rosedust color theme. It has 10 TUI tabs, accessible via F1-F10 (or `0` for Learning):

| Key | Tab | What it shows |
|-----|-----|---------------|
| F1 | Dashboard | Health gauges, plan progress, cost tracking, system metrics |
| F2 | Plans | Plan tree, task progress bars, wave overview |
| F3 | Agents | Live agent output, diffs, token burn, parallel pool status |
| F4 | Git | Branch tree, commit graph, worktree list |
| F5 | Logs | Scrollable log viewer with level filtering |
| F6 | Config | Effective config view with source annotations |
| F7 | Inspect | Signal DAG inspector, episode replay |
| F8 | Marketplace | Job browser, creation, and assignment |
| F9 | Atelier | PRD workshop and plan progress |
| F10 / 0 | Learning | Cascade routing, model health, and efficiency |

Additional keybindings: `q` to quit, `?` for help, `Tab`/`Shift+Tab` to cycle panels, `Enter` to drill into a task, `i` to inject a signal into a running session.

When idle, the dashboard shows recent episodes, gate results, system health, and config summary rather than blank panels.

## Multi-provider support

Roko routes work across 11 LLM backends based on task complexity, cost, and latency. Supported backends:

| Backend | Kind | What it does |
|---------|------|-------------|
| AnthropicApi | HTTP API | Anthropic Messages API (Opus, Sonnet, Haiku) |
| ClaudeCli | CLI subprocess | `claude` CLI with stream-json protocol |
| GeminiApi | HTTP API | Google Gemini API (1M context, grounding, context caching) |
| GeminiCli | CLI subprocess | `gemini` CLI subprocess |
| PerplexityApi | HTTP API | Perplexity Sonar API (web-grounded research with citations) |
| CerebrasApi | HTTP API | Cerebras inference (ultra-fast) |
| OpenAiCompat | HTTP API | Any OpenAI chat completions-compatible API (GLM, Kimi, Groq, Together, etc.) |
| CursorAcp | ACP protocol | Cursor Agent Client Protocol |
| CursorCli | CLI subprocess | Cursor `agent` CLI (ACP JSON-RPC over stdio) |
| Hermes | HTTP / CLI / ACP | Hermes gateway |
| OpenClaw | CLI / ACP | OpenClaw inference runtime |

Tier-based model routing assigns the cheapest viable model to each task:

```toml
[agent.tier_models]
mechanical = "gemini-2-5-flash-lite"   # imports, renames, trivial edits
focused = "gemini-2-5-flash"           # single functions, tests
integrative = "claude-sonnet-4-6"      # multi-module wiring
architectural = "claude-opus-4-6"      # API design, architecture
```

On failure, roko escalates to the next tier's model automatically.

For editor-driven ACP sessions, mutation built-ins (`write_file`, `edit_file`,
and `bash`) request editor permission before execution. Rejection, cancellation,
disconnect, timeout, or a dropped reply denies the call without side effects;
workspace-scoped “always allow” decisions are persisted for the selected action.

See `examples/` for complete provider configurations:
- `roko-gemini.toml` -- Gemini-only with 8 model tiers
- `roko-multi-provider.toml` -- Claude + Gemini + Perplexity routing
- `roko-perplexity.toml` -- Research-focused with deep research

## Architecture

### One noun, core verbs, supporting protocols

Everything in roko is a **Signal** -- a content-addressed (BLAKE3), timestamped, scored record of something that happened. Signals form a DAG through parent pointers, so you can always trace why the agent made a decision by walking backwards through lineage.

Six core workflow traits define what you can do with signals; supporting contracts cover
storage substrates, buses, observation, connectivity, and triggers:

| Trait | Job |
|-------|-----|
| `Substrate` | Store and query signals (memory, disk, chain) |
| `Scorer` | Rate signal relevance (recency, novelty, priority) |
| `Gate` | Verify output against ground truth (compile, test, lint) |
| `Router` | Pick among options (top-K, Thompson bandit, cascade) |
| `Composer` | Pack signals into token-budgeted prompts |
| `Policy` | React to patterns over time (episodes, retries, escalation) |

### Universal loop

Every agent runs the same loop:

```
query -> score -> route -> compose -> act -> verify -> write -> react
```

Stop at any step and you still have something useful. A prompt composer without an agent is a retrieval pipeline. An agent without gates is a raw LLM wrapper. The pieces are independent.

### Crate map

| Crate | What it does |
|-------|-------------|
| `roko-core` | Signal type, core and supporting protocol contracts, config schema, tool system, errors |
| `roko-agent` | 11 LLM backends (AnthropicApi, ClaudeCli, OpenAiCompat, CursorAcp, CursorCli, PerplexityApi, GeminiApi, GeminiCli, CerebrasApi, Hermes, OpenClaw), pools, tool loop, MCP, safety |
| `roko-agent-server` | Per-agent HTTP sidecar: `/message`, `/stream` (WS), `/predictions`, `/research`, `/tasks` |
| `roko-serve` | HTTP control plane: ~317 REST routes + SSE + WebSocket on port 6677 |
| `roko-gate` | 14 gate types, 7-rung pipeline, adaptive thresholds, artifact store |
| `roko-compose` | Prompt assembly, 9 role templates, U-shape placement, token budgeting |
| `roko-conductor` | 10 watchers, circuit breaker, intervention policy, diagnosis |
| `roko-learn` | Episodes, playbooks, bandits, model routing, prompt experiments, efficiency tracking |
| `roko-neuro` | Durable knowledge store, distillation, tier progression, garbage collection |
| `roko-dreams` | Offline dream cycle: batch episodes, cluster, distill knowledge, promote playbooks |
| `roko-mcp-code` | Code-intelligence MCP server (symbol lookup, dependency graph) |
| `roko-mcp-github` / `slack` / `scripts` / `stdio` | Additional MCP integrations |
| `roko-cli` | CLI binary, interactive ratatui TUI, plan DAG/runner, merge queue, and worktree manager |
| `roko-fs` | Append-only JSONL substrate with compaction and GC |
| `roko-std` | Default trait impls (memory substrate, simple routers, no-op scorers) |
| `roko-plugin` | Plugin SDK, canonical tier/capability manifests, three-root semver/dependency resolution, and kernel-confined declarative local tools |
| `roko-runtime` | Process supervisor, typed event bus, cancellation |
| `roko-primitives` | 10,240-bit hyperdimensional vectors, Hamming similarity, tier routing |
| `roko-index` | Code parser, symbol graph, PageRank, HDC fingerprints |
| `roko-lang-*` | Language support for Rust, TypeScript, Go |

## Gate pipeline

Every agent output passes through a gate pipeline before it is accepted. Gates run sequentially and short-circuit on the first failure by default.

### Rungs

The pipeline uses a 7-rung system. Which rungs execute depends on task complexity -- trivial tasks skip expensive checks, complex tasks run all of them.

| Rung | Gate | What it checks |
|------|------|---------------|
| 0 | Compile | `cargo check`, `tsc`, `go build` -- does it build? |
| 1 | Lint | `cargo clippy`, `eslint` -- does it pass linting? |
| 2 | Test | `cargo test` -- do existing tests pass? |
| 3 | Symbol | Symbol manifest check -- did the change break any public API? |
| 4 | GeneratedTest | Agent-generated behavioral tests |
| 5 | PropertyTest | Property-based tests (proptest/quickcheck) |
| 6 | Integration | Full integration scenario |

Additional specialized gates: `DiffGate` (patch analysis), `LlmJudge` (subjective quality), `FactCheck` (search-backed verification), `CodeExec` (sandboxed execution).

### Adaptive thresholds

Gate thresholds adjust over time using exponential moving averages. If a gate consistently passes, its threshold tightens. If it consistently fails, the threshold relaxes. Thresholds persist to `.roko/learn/gate-thresholds.json`.

## Learning and self-improvement

Roko tracks its own performance and gets better with use.

### Cascade router

Three maturity stages govern model selection: Static for 0–49 observations,
Confidence for 50–199, then UCB/LinUCB from observation 200 onward. Provider health
is an additional candidate filter, not a maturity stage. Live workflow, bridge, and
CLI outcomes feed one persisted health registry; runner-v2 maps model slugs to provider
IDs and filters unhealthy providers before selection. ACP adaptive selection requires
exact opt-in with `ROKO_ACP_CASCADE_SELECT=1` and never overrides a valid explicit
session provider/model selection.

```bash
roko model route claude-sonnet-4-6 --explain --complexity focused
```

### Prompt experiments

The experiment store and CLI/TUI/HTTP inspection surfaces track prompt variants and
their results. Runner/plan-run now assigns variants per exact attempt, replaces the named
canonical section before composition, binds the final prompt before launch, and settles
the scoped outcome idempotently from durable terminal events (including rotated logs after
restart). Serve, LearningRuntime, and ACP outcome writers use the same locked transaction,
so concurrent updates do not overwrite one another. ACP and serve still inject their
experiment context rather than using the runner's canonical-section receipt protocol.

```bash
roko experiment list
roko experiment show <id>
```

### Efficiency tracking

Every agent turn records tokens in/out, latency, cost, and gate pass/fail. These events feed the cascade router, the dashboard, and the dream cycle.

### Knowledge distillation (neuro)

Completed episodes are distilled into durable knowledge entries: facts, insights, heuristics, procedures, constraints, and anti-knowledge. Successful gate-backed runner ingestion records confirmation/context evidence and evaluates tier progression immediately. Knowledge decays over time with configurable half-lives (365 days for facts, 30 days for insights, 90 days for heuristics).

```bash
roko knowledge query "authentication patterns"
roko knowledge stats
```

### Dream cycle

Offline consolidation that runs between work sessions. The dream engine batches completed episodes, clusters them by task shape, distills knowledge, and promotes reliable success patterns into playbooks. In daemon mode, `[dreams]` can enable adaptive idle scheduling, a fallback `scheduled_cron`, and an `episode_count_trigger`; automatic cycles queue until no managed agent is active and retain their checkpoint across restart.

```bash
roko dream run
roko dream report
roko dream schedule
```

## Deployment

### HTTP control plane (`roko serve`)

```bash
roko serve                           # default bind 127.0.0.1:6677
roko serve --bind 0.0.0.0 --port 9090
```

Starts an Axum-based HTTP server with ~317 routes grouped by subsystem:

Top-level `GET /health` and `GET /ready` are the stable liveness and readiness probes used
by Docker and Fly. The richer `/api/health` response remains available for operators.

| Prefix | What it covers |
|--------|----------------|
| `/api/health`, `/api/status`, `/api/metrics/*` | Readiness + metric rollups (C-factor, gate rate, cost, velocity, coverage) |
| `/api/plans/*` | List, create, execute, inspect plans, and report spent/projected cost plus budget status |
| `/api/prds/*` | PRD lifecycle: ideas → drafts → promote → plan |
| `/api/research/*` | Research topic, enhance-prd, enhance-plan, enhance-tasks, analyze |
| `/api/agents/*` | Per-agent discovery, registration, messaging (`POST /api/agents/{id}/message`), topology |
| `/api/predictions/*` | Session predictions, claims, calibration |
| `/api/knowledge/*` | Knowledge entries, edges, search |
| `/api/tasks/*` | Task list, stats, improve feedback |
| `/api/learn/*` | Efficiency, cascade router, cost tiers, experiments, adaptive thresholds |
| `/api/extensions`, `/api/extensions/{name}` | Loaded extension layer/tier/version plus live circuit-breaker health |
| `/api/subscriptions/*`, `/api/templates/*`, `/api/deployments/*` | Ops primitives |
| `/api/config/*`, `/api/providers/*`, `/api/models/*`, `/api/rate-limits` | Configuration, shared persisted provider health/circuit state, and rolling RPM/TPM utilization |
| `/ws`, `/api/events`, `/webhooks/*` | Real-time: SSE events, top-level WS, webhook ingestion |

Example responses:

```bash
curl http://localhost:6677/api/health
# {"status":"ok","version":"0.1.0","uptime_seconds":123}

curl http://localhost:6677/api/metrics/c_factor
# {"overall":0.73,"components":{...},"episode_count":120}

curl http://localhost:6677/api/learn/efficiency
# {"total_cost":12.45,"cost_per_task":0.83,"tokens_per_task":24500.0,...}

curl http://localhost:6677/api/agents?owner=will
# [{"agent_id":"nunchi-intelligence","owner":"will","endpoints":{...}}]
```

### Per-agent sidecar (`roko-agent-server`)

Each registered agent also runs its own small HTTP server (typically on a
private port, proxied by the control plane):

| Endpoint | What it does |
|----------|-------------|
| `GET /health`, `/capabilities`, `/stats` | Always-on introspection |
| `POST /message` | Single-turn prompt → real LLM dispatch via the agent's configured backend |
| `GET /stream` (WS) | Streaming turn with `content`, `reasoning`, `tool_call`, `usage`, `done` chunks |
| `GET/POST /predictions*` | Prediction records + calibration |
| `POST /research` | Sidecar-local research task |
| `GET/POST /tasks*` | Agent-owned task queue with typed `Artifact` on completion |

`POST /message` wire shape:

```bash
curl -X POST http://localhost:6677/api/agents/nunchi-intelligence/message \
  -H "Content-Type: application/json" \
  -d '{"prompt":"ping"}'
# {"response":"Hello, world","reasoning":null,"usage":{...},"session":{...},
#  "finish_reason":"stop","engram_id":"engram-...","context":{...}}
```

Missing dispatcher returns `503`. Backend failure returns `502`. See
`crates/roko-agent-server/README.md` for the full contract.

### Chat with a running agent

```bash
roko chat --agent nunchi-intelligence
roko chat --agent nunchi-intelligence --serve-url http://localhost:6677
```

Opens an interactive REPL that POSTs to the sidecar through the aggregator.
Useful for ad-hoc debugging, prompt iteration, and smoke-testing a deployed
agent from your terminal.

### Background daemon

```bash
roko daemon start --port 9090    # start in background
roko daemon status               # check if running
roko daemon logs -f              # tail logs
roko daemon stop                 # graceful shutdown
roko daemon install              # install as macOS launchd service
```

The daemon watches for file changes, processes cron-scheduled tasks, ingests webhooks, and dispatches agents from event subscriptions.

### Cloud worker

```bash
roko worker --port 8080
```

Reads a template from environment variables and serves tasks. Designed for Railway, Fly.io, and container platforms.

Control-plane-created workers receive an opaque callback ID and a scoped
`ROKO_WORKER_CALLBACK_TOKEN`. Callbacks send that token in `X-Roko-Worker-Token`;
the server persists only its SHA-256 verifier and accepts it through the same global
authentication stack used when API-key auth is enabled.

### Cloud deployment

```bash
roko deploy railway    # deploy via Railway GraphQL API
roko deploy fly        # generate fly.toml and deploy
roko deploy docker     # build and tag Docker image
```

## Configuration

Roko uses layered TOML configuration: global (`~/.config/roko/config.toml`) merged with project (`./roko.toml`), with environment variables as overrides.

### Minimal config

```toml
[agent]
command = "claude"
model = "claude-sonnet-4-6"

[[gate]]
kind = "compile"

[[gate]]
kind = "test"

[budget]
max_plan_usd = 10.0
max_task_usd = 1.0
```

### Full project config

```toml
[agent]
command = "claude"
args = ["--print", "--output-format", "stream-json"]
model = "claude-sonnet-4-6"
effort = "high"
bare_mode = true
fallback_model = "claude-haiku-4-5"
timeout_ms = 300000

[agent.tier_models]
mechanical = "claude-haiku-4-5"
focused = "claude-sonnet-4-6"
integrative = "claude-sonnet-4-6"
architectural = "claude-opus-4-6"

[agent.escalation]
max_retries = 3
escalate_model = true

[prompt]
token_budget = 50000
role = "You are a Roko agent working on the project."

[budget]
max_plan_usd = 10.0
max_task_usd = 1.0
warn_at_percent = 80

[budget.tier_multipliers]
mechanical = 0.2
standard = 1.0
complex = 3.0
expert = 5.0

[[gate]]
kind = "compile"

[[gate]]
kind = "test"
```

### Config management

```bash
roko config init                            # interactive wizard
roko config show                            # effective merged config
roko config set agent.model claude-opus-4-6 # set a value
roko config validate                        # check syntax and references
roko config migrate                         # upgrade legacy format
```

### GitHub workflow automation

Configure `[github]` in `roko.toml` and export `GITHUB_TOKEN` to let plan runs open draft
pull requests, report terminal task gates, track failures as issues, require GitHub CI, and
merge with the configured method. Inspect the effective setup without starting the server:

```bash
roko github status
roko --json github status
```

Inbound webhooks use a separate `GITHUB_WEBHOOK_SECRET`. See the
[GitHub integration guide](docs/v2/GITHUB-INTEGRATION.md) for least-privilege setup, MCP
configuration, branch naming, CI validation, and troubleshooting.

### Fast self-development lane (opt-in)

For a small, well-scoped local plan, use the existing debug binary through the bounded FAST
wrapper instead of `cargo run`:

```bash
./dev.sh fast plans/my-plan
```

Every FAST task must author exactly one `verify` command. The patching agent is instructed not to
build or test; the runner owns that one check, preserves the warm target, runs headlessly with a
bounded deadline, and writes a private evidence bundle under `.roko/runs/`. The gate fails closed
when the task has zero or multiple verification commands.

FAST is an interactive feedback lane, not release proof. Do not use it for migrations, auth,
safety, persistence, payment, or other high-risk changes, and still run the contribution checks
below before merging. See [Fast development](docs/v2/29-FAST-DEVELOPMENT.md) for the contract,
security boundaries, and deferred work.

## CLI quick reference

| Command | What it does |
|---------|-------------|
| `roko init [path]` | Create `.roko/` directory and `roko.toml` |
| `roko run "<prompt>"` | Execute prompt through the full loop |
| `roko plan run <dir> --engine runner-v2` | Execute a plan directory through runner-v2 |
| `roko prd idea "<text>"` | Capture a work item |
| `roko prd draft new "<title>"` | Generate a PRD (agent-assisted) |
| `roko prd plan <slug>` | Generate implementation plan from PRD |
| `roko research topic "<topic>"` | Deep research with citations |
| `roko status` | Signal counts, recent episodes, gate results |
| `roko github status` | GitHub config, auth, plan PR, CI, and failure-issue status |
| `roko dashboard` | Interactive terminal dashboard |
| `roko knowledge query "<topic>"` | Search durable knowledge |
| `roko dream run` | Run offline knowledge consolidation |
| `roko config init` | Interactive setup wizard |
| `roko serve` | Start HTTP API server |
| `roko daemon start` | Start background daemon |
| `roko deploy railway` | Deploy to Railway |

Full reference with all 85+ commands, flags, and examples: [docs/v2/CLI-REFERENCE.md](docs/v2/CLI-REFERENCE.md)

## Building and testing

```bash
rustup update stable          # 1.91+ required for alloy deps
cargo build --workspace
cargo test --workspace
cargo clippy --workspace --no-deps -- -D warnings
```

### Running a single crate

```bash
cargo test -p roko-core
cargo test -p roko-agent
cargo test -p roko-gate
```

## Contributing

Contributions are welcome. A few ground rules:

1. **Search before writing.** This codebase has 35 workspace members and ~800K lines. The thing you want to build might already exist. Run `rg 'StructName' crates/ --glob '*.rs'` first.
2. **Wire, don't build.** The most common pattern in this repo is "built but never connected." Before adding new code, check if existing code needs to be called from the runtime.
3. **Verify before marking done.** Run the actual CLI code path. Passing unit tests does not mean the feature works end-to-end.
4. **All tests must pass.** `cargo test --workspace` and `cargo clippy --workspace --no-deps -- -D warnings` must both be clean.

## License

MIT OR Apache-2.0 (dual-licensed).
