# Roko

Roko is a Rust toolkit for building agents that build themselves. 36 workspace members, ~1.2M LOC, 11,000+ tests.

**Goal**: roko develops itself — it turns requests into plans, executes their tasks
via Claude agents, validates with gates, and persists results. The core loop is wired. Your job
is to use it and improve it.

**Status is not tracked in this file.** What to work on next: `work/NOW.md` (generated from the goals in
`work/goals.toml`). All open work: `work/STATUS.md`; p0/p1 only: `work/CLAUDE-OPEN.md`; history: `work/history/`. Item format and rules are in
`work/README.md`. The old status table, programme rollup and priority list are frozen, along with
a list of their known-stale claims, in `work/history/claude-md-status-2026-09-28.md`.

## Components

What each subsystem is and where it lives. The table makes no maturity or status claims.

| Component | What it is | Where |
|---|---|---|
| Kernel | `Signal` (backed by the `Engram` struct), the 12 kernel traits, the signal-selection loop helper | `crates/roko-core/` (`src/engram.rs`, `src/loop_tick.rs`) |
| Graph engine | The only plan executor: a DAG of Cells with topology, cost state and checkpoints | `crates/roko-graph/` (`src/engine.rs`) |
| Graph plan execution (host side) | Plan runner, control, delivery, feedback, workspaces and per-task dispatch for Graph runs | `crates/roko-cli/src/graph_execution/`, `crates/roko-cli/src/graph_task_dispatch.rs` |
| Runner support modules | Gate dispatch, persistence, output sinks, preflight, merge, resume, plan loading/DAG, queue manifests. The Runner-v2 event loop was deleted on 2026-09-06, and the `runner::run()` stub was removed in `725f21e05` | `crates/roko-cli/src/runner/` |
| Agent dispatch | Provider-neutral dispatch, prompt building, model routing; provider adapters | `crates/roko-cli/src/dispatch/`, `crates/roko-agent/` (`src/dispatcher/mod.rs`) |
| Gates | Gate implementations and the rung pipeline; the per-task entry point is `run_gate_once` | `crates/roko-gate/`, `crates/roko-cli/src/runner/gate_dispatch.rs` |
| Prompt assembly | Layered system prompt builder and role templates | `crates/roko-compose/` (`src/system_prompt_builder.rs`, `src/templates/`), `crates/roko-cli/src/dispatch/prompt_builder.rs` |
| Safety | Role authorization, pre/post checks, tool policy | `crates/roko-agent/src/safety/` |
| Runtime services | Shared `RuntimeServices` builder (the service facade for CLI, serve and ACP) | `crates/roko-execution/` |
| Process supervision | ProcessSupervisor, event bus, cancellation | `crates/roko-runtime/` |
| Execution state | Per-plan Graph checkpoint, activity log and cost state | `.roko/state/graph/<plan>/` (`checkpoint.json`, `activities.jsonl`, `costs.json`) |
| Episodes | Per-turn episode records. `hdc_fingerprint` is `HdcVector::from_seed` of the serialized prompt and outcome, i.e. one 64-bit FNV-1a hash expanded into a vector. It identifies exact inputs; it does not measure semantic similarity | `crates/roko-cli/src/runtime_feedback/episodes.rs`, `crates/roko-primitives/src/hdc.rs`, `.roko/episodes.jsonl` |
| Learning | Model routing (CascadeRouter), bandits, playbooks, prompt experiments, efficiency events | `crates/roko-learn/`; state in `.roko/learn/` (`cascade-router.json`, `gate-thresholds.json`, `efficiency.jsonl`) |
| Knowledge and dreams | Durable knowledge store, distillation, tiers; offline Dream consolidation | `crates/roko-neuro/`, `crates/roko-dreams/` |
| Affect | Daimon affect engine and dispatch modulation | `crates/roko-daimon/` |
| Plan authoring and research | `roko run --plan` / `roko plan generate` (prompt -> plan; the one plan generator) and `roko research` | `crates/roko-cli/src/plan_generate/`, `crates/roko-cli/src/plan_authoring.rs`, `crates/roko-cli/src/commands/run_cmd.rs`, `crates/roko-cli/src/research.rs`; data in `plans/`, `.roko/research/` |
| HTTP control plane | REST, SSE and WebSocket API on :6677 | `crates/roko-serve/` (`src/routes/`) |
| Per-agent sidecar | HTTP sidecar for a single agent | `crates/roko-agent-server/` |
| ACP server | Agent Client Protocol server for editors | `crates/roko-acp/` |
| Inference gateway | Provider routing and fallback, caching, cost accounting | `crates/roko-gateway/` |
| MCP | MCP client plus roko's own MCP servers | `crates/roko-agent/src/mcp/`, `crates/roko-mcp-*/` |
| TUI and chat | ratatui dashboard (`roko dashboard`) with a file watcher; `roko chat` REPL | `crates/roko-cli/src/tui/` (`fs_watch.rs`), `crates/roko-cli/src/chat.rs` |
| GitHub integration | `roko github status`; `GitHubOps` trait with no-op and live adapters; `[github]` config | `crates/roko-cli/src/commands/github.rs`, `crates/roko-cli/src/github_ops.rs`, `crates/roko-cli/src/github_ops_impl.rs`, `roko.toml` |
| Plugins | Plugin manifests, declarative tools, capability policy, dependency resolution | `crates/roko-plugin/` |
| Chain primitives | Optional chain client plus local registry, marketplace, arena and DeFi state machines | `crates/roko-chain/` |
| Signal log | Canonical signal log (a legacy `engrams.jsonl` is read only as a fallback) | `.roko/signals.jsonl` (path logic in `crates/roko-fs/src/layout.rs`) |

## Critical rules

### 1. NEVER reimplement what already exists
Search before writing: `grep -rn 'FunctionName\|StructName' crates/ --include='*.rs' | grep -v target/`
This codebase has duplicate implementations from parallel development. CHECK FIRST.

### 2. WIRE, don't build
The pattern in this codebase is "built but never connected." Before building anything new,
check if existing code just needs to be called from the runtime. If your change isn't visible
via `cargo run -p roko-cli -- <subcommand>`, it's probably wrong.

### 3. Verify before marking done
Run the actual code path. "Code exists" != "feature works". Test via CLI, not just unit tests.

### 4. Track work in the work graph, and close what you finish
Open work lives in `work/items/`, one file per item; the rules are in `work/README.md`. A new gap or bug
becomes a new item (anchors, a body that follows the template, a verify command). When your change
finishes an item, close it in the same flow: end the commit message with `Closes: <id>` and run
`python3 tools/work.py close <id> --commit HEAD --evidence "…"`. Items that nobody closes go stale, which
is what happened to the old backlog. `.roko/GAPS.md` and `tmp/backlog/` are frozen.

## Architecture

The primary protocol noun is `Signal`, backed by the `Engram` struct and its
`pub type Signal = Engram` alias in `roko-core/src/engram.rs`. The kernel exposes 12 traits
(Store, ColdStore, Score, Verify, Route, Compose, React, Bus, Observe, Connect, Trigger,
Substrate). Missing or unknown safety contracts fail closed: unsupported tool use is denied.
The conceptual workflow is query -> score -> route -> compose -> act -> verify -> write ->
react. Production ownership is explicit: `roko run` sizes a prompt and runs it as a one-task plan,
or as a plan it writes first, with `run_graph_plan` (backlog #276 retired `WorkflowEngine`); plans
use the Graph engine,
which is the only plan executor, and the core `select_compose_verify_persist` helper covers only
the non-ACT/non-BROADCAST signal-selection subset. Backlog #260 made Graph the default. The
Runner-v2 event loop was deleted on 2026-09-06 (`6b5da8616`); `--engine legacy` and
`--engine runner-v2` are still parsed (`PlanEngine` in `crates/roko-cli/src/main.rs`) but rejected with an error
(`crates/roko-cli/src/commands/plan.rs`).
`#NNN` numbers in this file are backlog items, not GitHub PRs.

## Self-hosting workflow

This is how roko develops itself. Plans are the unit of work: a request (a prompt, or a written
spec passed with `--from-file`) becomes a plan, a person can review and edit it, and then it runs.
Each step is a CLI command that exists today:

```bash
# 1. Write a plan from a request: plans/<slug>/tasks.toml + plan.md (nothing runs yet)
cargo run -p roko-cli -- run --plan --dry-run "Wire SystemPromptBuilder into runner"
#    (the same as: cargo run -p roko-cli -- plan generate "Wire SystemPromptBuilder into runner")

# 2. Optional: improve the plan with research-backed decomposition
cargo run -p roko-cli -- research enhance-plan wire-systempromptbuilder-into-runner

# 3. Review or edit plans/<slug>/, then lint it
cargo run -p roko-cli -- plan validate plans/wire-systempromptbuilder-into-runner

# 4. Execute the plan (agents run tasks, gates validate, state persists)
cargo run -p roko-cli -- run plans/wire-systempromptbuilder-into-runner

# 5. Resume if interrupted
cargo run -p roko-cli -- run plans/wire-systempromptbuilder-into-runner --resume-plan

# 6. Watch progress
cargo run -p roko-cli -- dashboard

# 7. Check status
cargo run -p roko-cli -- status
```

`roko run --plan "<prompt>"` does steps 1 and 4 in one go: it writes the plan, shows it, and asks
before running it (`--yes` skips the question). A small change needs no plan:
`roko run "<prompt>"` sizes the prompt and runs a small one as one checked task. The PRD pipeline
(`roko prd`) and `roko do` were removed on 2026-10-02 (`tmp/workflow-audit/`); for one release
they still parse and exit 1 with their replacement. `roko develop` (an error since 2026-09-04) is
gone.

### Opt-in FAST self-development

For an eligible small/local plan with a prebuilt `target/debug/roko`, prefer the bounded wrapper:

```bash
./dev.sh fast plans/<plan-directory>
```

Each FAST task must define exactly one authored `verify` command. On the Graph engine FAST bounds
prompt context, stops the run at `ROKO_FAST_PLAN_DEADLINE_SECS` (exit 143, reported as
`deadline`), caps each task attempt at `ROKO_FAST_MAX_AGENT_TURNS` turns (default 6) and 90 s,
tells the agent to patch and hand off without running Cargo, builds simple cargo verify commands
in the `dev-fast` profile, never runs `cargo fix` after a failed verify, and captures a private
evidence bundle (`crates/roko-cli/src/graph_execution/fast_lane.rs`). It is not appropriate for
safety, auth, persistence, migration, payment, or other high-risk changes. FAST evidence does
**not** replace the mandatory pre-commit checks in the Building section.

## CLI commands reference

### Core workflow
| Command | What it does |
|---|---|
| `roko init` | Create `.roko/` directory and `roko.toml` |
| `roko setup` | Interactive setup wizard: detect providers, init workspace, verify |
| `roko run "<prompt>"` | Size the prompt: one checked task, or a plan written first and then run (`commands/run_cmd.rs`) |
| `roko run --plan "<prompt>"` | Always write a plan first (`--dry-run`: write it and stop; on a terminal it asks before running) |
| `roko run plans/<dir>` | Run an existing plan directory, as `roko plan run` does (`--fresh`, `--resume-plan`) |
| `roko show [subject]` | Inspect workspace state: costs, agents, knowledge, plans, learning, history |
| `roko status` | Query signals, report counts and episodes |
| `roko doctor` | Diagnose workspace bootstrap state |
| `roko doctor disk/network` | Report free space, stale targets, worktrees, or network reachability |
| `roko diagnose <plan-id>` | Diagnose why a plan failed: a readable report (`--json` for JSON, `--verbose` adds the tasks that completed) |
| `roko resume [run-id]` | Resume a plan execution from its last checkpoint |
| `roko github status` | Inspect GitHub config, authentication, plan PR/CI state, and failure issues |
| `roko think "<question>"` | Research a question without executing agents or changing files |
| `roko note "<text>"` | Capture a quick note (no LLM, instant, with optional tags) |
| `roko login [url]` | Authenticate with a roko-serve instance (browser or API key) |
| `roko logout` | Remove stored credentials |
| `roko whoami` | Show current authentication status |
| `roko vision-loop <file>` | Iterative vision-guided UI refinement loop |
| `roko history [id]` | List or show past chat session summaries |
| `roko cache status/prune` | Inspect and safely prune workspace-local build/evidence caches |

### Planning
| Command | What it does |
|---|---|
| `roko plan list/show/create` | Manage plans |
| `roko plan run <dir>` | Execute plans through the Graph engine (the only engine; `--engine legacy`/`runner-v2` exits with an error) |
| `roko plan generate/regenerate` | Generate a plan from a prompt, file, notes or backlog spec (runs nothing), or regenerate one in place |
| `roko plan index` | Rebuild or verify the deterministic plans index |
| `roko plan pause/resume/cancel` | Pause, resume, or cancel a running plan |
| `roko plan retry <dir>` | Retry failed tasks in a plan |
| `roko plan status <dir>` | Show execution status for a plan |
| `roko plan queue show/validate/init` | Queue manifest operations |
| `roko plan validate <dir>` | Lint tasks.toml without executing |
| `roko backlog list/audit/mark-done` | List `tmp/backlog` items, reconcile plan status against Graph runs, record closure evidence (`roko plan generate --from-backlog <ids>` turns specs into plans) |

### Agents
| Command | What it does |
|---|---|
| `roko agent create --name X --domain Y` | Create agent from manifest |
| `roko agent delete --name X` | Delete an agent and clean up its state (ordered 8-step shutdown) |
| `roko agent start --name X` | Start a long-running agent |
| `roko agent stop --name X` | Stop a running agent |
| `roko agent list` | List agents with status |
| `roko agent status --name X` | Detailed agent health |
| `roko agent serve` | Start per-agent HTTP sidecar |
| `roko agent chat --agent X` | Interactive chat REPL with an agent |

### Research
| Command | What it does |
|---|---|
| `roko research topic "<topic>"` | Deep research with citations |
| `roko research search "<query>"` | Direct web search (Perplexity) |
| `roko research enhance-plan/tasks` | Improve a plan (or its tasks) with research |
| `roko research analyze` | Analyze execution data |
| `roko research list` | List all research artifacts |

### Knowledge (neuro + dreams + custody + archive)
| Command | What it does |
|---|---|
| `roko knowledge query "<topic>"` | Search durable knowledge store |
| `roko knowledge stats/gc` | Store statistics, garbage collection |
| `roko knowledge backup/restore` | Backup with genomic bottleneck, restore with decay |
| `roko knowledge sync <peer>` | Mesh knowledge sync |
| `roko knowledge dream run/report/schedule` | Dream consolidation cycle |
| `roko knowledge dream journal/archive` | Dream journal and archive entries |
| `roko knowledge export/import` | Export or import knowledge entries |
| `roko knowledge backfill-hdc` | Backfill HDC fingerprints for existing entries |
| `roko knowledge custody list/show/verify` | Custody audit chain |
| `roko knowledge archive` | Cold storage archival |

### Learning & feedback
| Command | What it does |
|---|---|
| `roko learn all/router/experiments/efficiency/episodes` | Inspect learning state |
| `roko learn reflexes` | Show T0 reflex rules (count, top five by hits, recent demotions) |
| `roko learn gates` | Show adaptive gate threshold state |
| `roko learn knowledge-stats` | Show durable knowledge entry counts |
| `roko learn inspect gates/routing/budget` | Read-only subsystem inspection (thresholds, routing, budget) |
| `roko learn tune gates/routing/budget` | (deprecated) Alias for `learn inspect` |

### Jobs
| Command | What it does |
|---|---|
| `roko job list/create/show/execute/cancel` | Manage marketplace jobs |
| `roko job match` | Find matching jobs for an agent's capabilities |

### Configuration
| Command | What it does |
|---|---|
| `roko config init/show/path/edit/set` | Core config management |
| `roko config doctor` | Print basic config health without modifying files |
| `roko config validate/migrate` | Schema validation, legacy migration |
| `roko config set-secret/check-secrets` | Secret management |
| `roko config export` | Export config as environment variables for a deployment target |
| `roko config env` | List all recognized environment variables with descriptions |
| `roko config providers list/health/test` | LLM provider inspection |
| `roko config providers available/discover/add/catalog/validate` | Provider discovery and setup |
| `roko config models list/route` | Model inspection and routing |
| `roko config subscriptions list/add/remove` | Event subscriptions |
| `roko config events` | Configured event sources |
| `roko config experiments` | Model A/B experiments |
| `roko config plugins list/install/remove/audit/publish` | Plugin management |
| `roko config secrets set/get/list/rotate` | Profile-aware secrets |
| `roko config mcp list/test/add` | MCP server configuration |
| `roko config preset gates/routing/budget/model` | Apply validated config presets (with --dry-run, --yes) |

### Server & deployment
| Command | What it does |
|---|---|
| `roko serve` | Start HTTP control plane on :6677 (count routes with `python3 tools/http_route_inventory.py`) |
| `roko acp` | Start ACP (Agent Client Protocol) server for editor integration |
| `roko daemon start/stop/status/logs/install` | Daemon lifecycle |
| `roko deploy railway/fly/docker` | Cloud deployment |
| `roko worker` | Run as deployed worker |

### Graph, feeds, recipes, and triggers
| Command | What it does |
|---|---|
| `roko graph run/validate/show` | Execute, validate, and inspect graph definitions (DAGs of cells) |
| `roko feed list/status/start/stop` | Inspect and manage runtime data feeds |
| `roko recipe list/show/validate/run` | Manage and evaluate pure-data feed recipes |
| `roko trigger list/show/create/fire` | Manage trigger bindings |

### Utilities
| Command | What it does |
|---|---|
| `roko dashboard` | Interactive ratatui TUI (F1–F10 tabs) |
| `roko replay <hash>` | Walk signal DAG by hash |
| `roko inject <session> <payload>` | Signal injection |
| `roko index build/rebuild/search/stats` | Code intelligence index |
| `roko run-index repair` | Inspect or rebuild derived per-run event indexes |
| `roko bench demo/swe` | Run benchmark evaluations and write learning telemetry |
| `roko new <type> <name>` | Scaffold boilerplate |
| `roko explain <topic>` | Concept explainer (3 depth levels) |
| `roko completions <shell>` | Shell completion scripts |

## Key crates

| Crate | Path | What |
|---|---|---|
| roko-core | `crates/roko-core/` | Signal + 12 traits, types, config, tools, errors |
| roko-agent | `crates/roko-agent/` | 12 LLM provider kinds (AnthropicApi, ClaudeCli, CodexCli, OpenAiCompat, CursorAcp, CursorCli, PerplexityApi, GeminiApi, GeminiCli, CerebrasApi, Hermes, OpenClaw), pools, MCP, tool loop, safety |
| roko-agent-server | `crates/roko-agent-server/` | Per-agent HTTP sidecar: `/message` (real LLM dispatch), `/stream` WS, `/predictions`, `/research`, `/tasks` |
| roko-serve | `crates/roko-serve/` | HTTP control plane: REST routes + SSE + WebSocket on :6677 |
| roko-gate | `crates/roko-gate/` | 19 gates, 7-rung pipeline, adaptive thresholds |
| roko-compose | `crates/roko-compose/` | Prompt assembly, 11 role templates, enrichment |
| roko-conductor | `crates/roko-conductor/` | 12 watchers, circuit breaker, diagnosis |
| roko-learn | `crates/roko-learn/` | Episodes, playbooks, bandits, model routing, experiments, efficiency |
| roko-cli | `crates/roko-cli/` | CLI, plan DAG/runner, merge queue, worktree manager, ratatui TUI |
| roko-fs | `crates/roko-fs/` | FileSubstrate (JSONL), GC, layout |
| roko-std | `crates/roko-std/` | 35 definitions by default (16 executable local + 19 GitHub MCP); 52 with typed optional-chain placeholders; HTTP MCP clients/resolvers retained at runtime |
| roko-execution | `crates/roko-execution/` | RuntimeServices builder, diagnostic service, execution control, feedback settlement |
| roko-runtime | `crates/roko-runtime/` | ProcessSupervisor, event bus, cancellation, workflow contract |
| roko-primitives | `crates/roko-primitives/` | HDC vectors, tier routing |
| roko-neuro | `crates/roko-neuro/` | Durable knowledge store, distillation, tier progression |
| roko-mcp-code | `crates/roko-mcp-code/` | Code-intelligence MCP server |
| roko-mcp-github / stdio | `crates/roko-mcp-*/` | Additional MCP integrations |
| roko-index | `crates/roko-index/` | Parser + graph + HDC indexing |
| roko-lang-rust / typescript / go | `crates/roko-lang-*/` | Language support |
| roko-dreams | `crates/roko-dreams/` | Offline consolidation (hypnagogia, imagination, cycle) |
| roko-daimon | `crates/roko-daimon/` | Affect engine, somatic markers, dispatch modulation |
| roko-acp | `crates/roko-acp/` | ACP (Agent Client Protocol) server for Cursor/external agent integration |
| roko-plugin | `crates/roko-plugin/` | Plugin manifests, executable declarative tools, canonical tier/capability policy, semantic-version/dependency resolution |
| roko-graph | `crates/roko-graph/` | Graph engine, DAG cells, topology, cost state |
| roko-demo | `crates/roko-demo/` | Demo/example binary for showcasing features |
| roko-chain | `crates/roko-chain/` | Optional chain client/runtime primitives plus tested local registry, marketplace, arena, and DeFi state machines. daeji owns node/BFT/precompiles in a separate repo. |

## Absolute paths

| What | Path |
|---|---|
| **Workspace root** | `/Users/will/dev/nunchi/roko/roko/` |
| **All crates** | `/Users/will/dev/nunchi/roko/roko/crates/` |
| **CLI source** | `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/` |
| **Graph engine** | `/Users/will/dev/nunchi/roko/roko/crates/roko-graph/src/engine.rs` |
| **Graph plan execution (host side)** | `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/graph_execution/` |
| **Agent dispatcher** | `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/dispatcher/mod.rs` |
| **Safety layer** | `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/safety/` |
| **System prompt builder** | `/Users/will/dev/nunchi/roko/roko/crates/roko-compose/src/system_prompt_builder.rs` |
| **Role templates** | `/Users/will/dev/nunchi/roko/roko/crates/roko-compose/src/templates/` |
| **Work graph (status, open work)** | `/Users/will/dev/nunchi/roko/roko/work/` |
| **Frozen gap log** | `/Users/will/dev/nunchi/roko/roko/.roko/GAPS.md` |
| **Roko data dir** | `/Users/will/dev/nunchi/roko/roko/.roko/` |
| **Graph checkpoints** | `/Users/will/dev/nunchi/roko/roko/.roko/state/graph/` |
| **Plans** | `/Users/will/dev/nunchi/roko/roko/plans/` |
| **Research artifacts** | `/Users/will/dev/nunchi/roko/roko/.roko/research/` |
| **Signal log** | `/Users/will/dev/nunchi/roko/roko/.roko/signals.jsonl` |
| **Episode log** | `/Users/will/dev/nunchi/roko/roko/.roko/episodes.jsonl` |

## Reference material (read-only, do not modify)

| What | Path | Notes |
|---|---|---|
| Mori (original orchestrator) | `/Users/will/dev/uniswap/bardo/apps/mori/` | 108K LOC, the reference for what roko replaces |
| Mori agent connection | `/Users/will/dev/uniswap/bardo/apps/mori/src/agent/connection.rs` | Lines 2444-2620 = reference agent spawn |
| Original 36 crates | `/Users/will/dev/uniswap/bardo/crates/` | 137K LOC |
| Mori plans | `/Users/will/dev/uniswap/bardo/.mori/plans/` | 171 plans with TOML tasks |
| PRD documents | `/Users/will/dev/nunchi/roko/bardo-backup/prd/` | 359 files, 26 sections |
| Roko progress docs | `/Users/will/dev/nunchi/roko/bardo-backup/tmp/roko-progress/` | 140+ files, parity checklist (stale paths) |
| Mori parity checklist | `/Users/will/dev/nunchi/roko/bardo-backup/tmp/roko-progress/MORI-PARITY-CHECKLIST.md` | 1,253 items, ~33% done |
| Mistakes learned | `/Users/will/dev/nunchi/roko/bardo-backup/tmp/roko-progress/MISTAKES-LEARNED.md` | 30+ catalogued mistakes |
| Component specs | `/Users/will/dev/nunchi/roko/bardo-backup/tmp/roko-progress/COMPONENTS/` | 140+ per-component specs |
| Mori agent docs | `/Users/will/dev/nunchi/roko/bardo-backup/tmp/mori-agents/` | Backend arch, tool system |
| Research docs | `/Users/will/dev/nunchi/roko/bardo-backup/tmp/mori-refactor/` | Layer theory, design patterns |
| Agent chain docs | `/Users/will/dev/nunchi/roko/bardo-backup/tmp/agent-chain/` | Phase 2+ chain architecture |

## Building

```bash
cd /Users/will/dev/nunchi/roko/roko
rustup update stable          # Need 1.91+ for alloy deps
cargo build --workspace
cargo test --workspace
cargo clippy --workspace --no-deps -- -D warnings
```

### Pre-commit checks (MANDATORY before any commit)

**Always run these before committing. CI will reject code that fails any of these.**

```bash
cargo +nightly fmt --all                              # Format (nightly, matches CI)
cargo clippy --workspace --no-deps -- -D warnings     # Lint (must pass clean)
cargo test --workspace                                # Tests (must pass)
```

Do NOT push without running all three. The CI uses the latest stable rustc which may have
stricter lints than your local toolchain.

## What to work on

Open work lives in the work graph, not in this file. Start with `work/NOW.md` (generated: the top
items of each goal in `work/goals.toml`); the full list, grouped by subsystem, is in `work/STATUS.md`. The old numbered priority list is
frozen in `work/history/claude-md-status-2026-09-28.md`.

To pick up a task, alone or in parallel with other agents, follow `work/README.md`, "For agents":
`python3 tools/work.py next` picks the top unclaimed item that doesn't touch files others are working on,
`claim` reserves it, and you work in your own worktree on branch `work/<id>`. The `/work-next`,
`/work-batch` and `/work-sweep` skills in `.claude/skills/` run that procedure.

Long-term priorities that still hold:

1. **Fresh self-hosting proof**: rerun the complete self-hosting workflow above, live and end to
   end. The blockers from the first dogfood run have regression fixes, but no live rerun has been
   recorded.
2. **Learning loops on the Graph path**: several feedback paths were attached to the deleted
   Runner-v2 event loop and have not been re-attached to Graph runs. For example, Graph task
   dispatch passes `prompt_experiment: None` (`crates/roko-cli/src/graph_task_dispatch.rs:1791`).
3. **roko tracks its own work**: the `roko work` CLI (it extends `roko backlog`, per
   `work/README.md`) plus plan-task `closes = [...]` links, so that roko itself maintains the work
   graph.
4. **Chain and economic runtime integration**: local witness, x402, marketplace, registry, arena
   and DeFi primitives exist. Production work still spans several adapter classes:
   daeji-backed contracts/consensus, network transport and indexing, durable services, caller
   authorization, market data, risk admission, and venue execution.
