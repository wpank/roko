# Roko

Roko is a Rust toolkit for building agents that build themselves. 35 workspace members, ~800K LOC, 9,900+ tests.

**Goal**: roko develops itself — it reads PRDs, generates implementation plans, executes tasks
via Claude agents, validates with gates, and persists results. The core loop is wired. Your job
is to use it and improve it.

Programme rollup (2026-08-17): **48/48 epics accepted**, with no partial or greenfield
epics and **0 remaining epic-manifest tasks**. The raw E01-E48 executable-manifest
metadata count is 393/447; it is not a completion score because older accepted manifests
remain unreconciled. `.roko/GAPS.md` is the canonical runtime-acceptance source.

Executable queue: **124/124 tasks complete (100%)** across 30 plans; all 30 executable
plans are complete and `architecture-production-residuals` is 4/4 after R01-R04. The
broader master checklist census is **211 done / 11 partial / 20 unchecked** raw markers,
which also includes repeated controls, documentation/dogfood proof, and product gaps.

Bounded documentation integrity is live for the maintained operator corpus: exact
109/746/637 registry contracts, local paths/anchors, and deterministic `plans/INDEX.md` drift
are checked; DOC reconciliation is 3/71 done. Direct TUI/show/API/workspace Runner views now
share the verified canonical snapshot loader, while StateHub overlay/SSE cursor atomicity and
single-generation resume remain explicit partial work.

## Current state (2026-08-17)

The plan-execute-gate-persist loop **works end-to-end**, and so do the HTTP
control plane, per-agent sidecar, and interactive TUI:

| Component | Status | Where |
|---|---|---|
| Plan discovery + DAG executor | **Wired** | `crates/roko-cli/src/runner/event_loop.rs` (runner-v2; legacy `orchestrate.rs` removed) |
| Agent dispatch (CLI + HTTP providers) | **Wired** | Provider-neutral runner dispatch plus `roko-agent` adapters |
| Safety layer (role auth, pre/post checks) | **Complete (E34 8/8 strict)** | Trust-origin IFC, exact Cell × Graph × Space capabilities, five-head corrigibility, sandbox/process policy, and mandatory audited hooks are live. Canonical provider primary outputs and every host-visible tool result traverse the five-stage immune Graph; bounded workspace-rooted authority persists provider isolation, tool cooldown/isolation, evidence, and reciprocal incident links independently of attempt worktrees. Provider-owned internal calls/results, provider trace Signals, broad semantic/adaptive immune memory, and externally anchored whole-ledger authenticity remain product residuals |
| Gate pipeline (compile, test, clippy, diff) | **Wired** | `runner/gate_dispatch.rs` with enriched rung inputs |
| Release verification | **P34 4/4 historical checkpoint (2026-08-16)** | Formatting, workspace check, strict default-target/default-feature workspace clippy, the default package/integration/doctest suite, optimized release build, release CLI startup/doctor/plan validation, and focused strict plan validation passed at that checkpoint. Subsequent dirty-tree changes require fresh verification; the stricter all-target/all-feature final gate remains separate |
| Session persistence (snapshot + resume) | **Wired** | `.roko/state/state-snapshot.json` (authoritative runner-v2 snapshot) |
| PRD lifecycle (idea/draft/plan) | **Wired** | `roko prd` subcommands |
| Research agent | **Wired** | `roko research` subcommands |
| Plan generation from PRD | **Wired** | `roko prd plan <slug>` → agent generates tasks.toml |
| SystemPromptBuilder (9-layer prompts) | **Wired** | `RoleSystemPromptSpec` in runner/ |
| EpisodeLogger (agent turn recording) | **Wired** | `.roko/episodes.jsonl` via runner/ |
| Resource lifecycle + disk admission | **Wired** | Ordered runner-v2 cleanup, bounded JSONL generations, disk-aware worktree admission, `roko doctor disk` |
| Periodic telemetry sampling | **Wired** | `roko-serve` samples shared metrics every 30s through `PeriodicObserver` and writes rotation-bounded JSONL |
| Telemetry Lens runtime | **Complete (E33 9/9; 39/39 ingress)** | All 11 built-ins, bounded queued delivery, breaker controls, typed StateHub aggregation, REST/SSE, restart-durable history, resolution queries, configurable 7-day retention, and all 39 production variants are live. Registered agents commit the final six through a typed, durable, identity-bound observation boundary; direct native Agent publication remains separate product integration scope |
| Agent cognitive autonomy | **Complete (E23 10/10 manifest)** | Lifecycle type-state, behavioral vitality, CorticalState energy fields, energy accounting, adaptive timescales, energy/affect coupling, EFE routing, GoalTree, SlotManager, revisioned mode owners, and phase-aware runner dispatch are live. Native Agent-to-E33 observation publication remains broader integration scope |
| Declarative Trigger runtime | **Complete (E31 8/8)** | All seven sources, root-Cell payload Signals, Space/capability enforcement, shared Pulse delivery, live Graph execution, durable history, IANA/DST cron, watcher-to-raw-EVM ABI/finality/reorg handling, and CA-verified mTLS are live |
| Tool/plugin ecosystem | **Complete (E32 8/8 manifest)** | Signed dependency graphs, bounded typed WASM hooks, strict plugin admission, verified relay/install, and current CLI/MCP targets satisfy the manifest. WIT/Component hostcalls and OpenClaw/legacy adapter parity remain separate roadmap work |
| Named surfaces | **Complete (E37 9/9 contract/backend manifest)** | Typed Workbench/Inbox/Canvas/Minimap/Autonomy projections, five dedicated StateHub-backed routes, OpenAPI, events, object types, and legacy-tab mapping are live. Full named-surface TUI rendering and several native runtime sources remain product residuals |
| Cold substrate archival | **Wired** | Configurable server timer archives aged signals before pruning the hot substrate |
| Provider outcome feedback | **Wired** | Live workflow attempts plus runner-v2 CLI/bridge outcomes update one persisted health registry; unhealthy providers are filtered during learned routing |
| Worker callback authentication | **Wired** | Deployment-scoped opaque IDs and hashed token verifiers compose with the global serve-auth middleware |
| GitHub workflow integration | **Wired (E46 12/12)** | Draft plan PRs, terminal comments/issues, exact accepted-commit publication, local-regression + CI merge ordering, webhook trigger graduation, and `roko github status` |
| ProcessSupervisor (lifecycle mgmt) | **Wired** | `PlanRunner` tracks + shuts down agents |
| MCP config + HTTP execution | **Wired** | CLI passthrough plus retained live clients/resolvers for Anthropic/OpenAI/Gemini/Perplexity/Cerebras tool loops; definition-only MCP advertisements fail closed |
| Inline image input | **Wired for supported API paths (P28 5/5)** | Vision-capable Anthropic, OpenAI-compatible, and Gemini API paths preserve ordered image blocks; ACP advertises the resolved model capability. Subprocess/legacy transports and non-vision models fail closed; audio remains unsupported |
| ACP completion | **8/8 wired** | Mutation consent, experiments, Anthropic MCP parity, truthful capabilities, persisted/enforced USD budgets, shared health/rate-aware selection, configured sandboxing, and opaque-phase worktree inspection are live; 180 ACP tests pass |
| AgentContract tool policy | **Wired** | Role/task allowlists intersect, denials win, unknown roles deny all, and unsupported policy-bearing dispatches are rejected |
| Efficiency events (per-turn) | **Wired** | `.roko/learn/efficiency.jsonl` via runner/ |
| CascadeRouter (model routing) | **Wired** | Persists to `.roko/learn/cascade-router.json`, configurable models |
| Advanced learning loops | **Complete (E25 10/10)** | HDC consolidation, hindsight adjustments, c-factor governance, significance/early stopping, Variance Inequality, autocatalytic metrics, and when/then playbook enrichment are wired into runner dispatch/completion |
| Advanced memory | **Complete (E24 10/10)** | Balance demurrage/reinforcement, falsifiers, streaming HDC role/filler lookup, temporal query/GC, distillation and cross-domain transfer, configurable progression, Dream consolidation, and CLI lifecycle maintenance are wired |
| Inference gateway | **Complete (E26 12/12)** | The dedicated nine-stage gateway owns routing/fallback, exact and semantic caches, tool/output/thinking controls, convergence, cost accounting, key rotation, three-level backpressure, handles, batches, events, and authenticated serve routes |
| Continuous feeds and recipes | **Complete (E27 10/10)** | Cell-composed runtime feeds, discovery/lifecycle, Bus bridging, built-in source feeds, validated recipe DAG persistence/evaluation, HTTP routes, and CLI commands are live |
| Agent groups | **Complete (E28 8/8)** | Persisted invitations/membership, permissions, coordination, knowledge/pheromone/message/event flows, Bus publication, HTTP/CLI operations, and privacy-filtered group prompt context are wired |
| Connectivity and relay | **E29 + R01/R02 scoped runtime complete** | The async five-method contract has one supervised HTTP JSON adapter. `agent-relay`, the supervised client, and `roko-serve` provide bounded canonical-envelope delivery, atomic cursor restore, fail-closed reconciliation, and ACK-after-durable exact-room subscription terminalization. Additional transports, startup discovery, MCP/A2A/x402/finality execution, and dashboard integration remain product work |
| Artifact marketplace | **Complete contract/stub tranche (E38 9/9)** | Artifact/package/publish/economics/fork/capability contracts plus HTTP/CLI stubs are tested. Durable storage/search, executable publish/install pipelines, and ERC-8004 anchoring remain product work |
| Registries and identity | **Complete local tranche + critical-path runtime** | Transferable identity/delegation, challengeable knowledge, TraceRank, transport-neutral gossip, durable authenticated local passport/knowledge routes, and an optional bounded manual finality/reorg-aware indexer are tested. Deployed contracts, gossip transport, ABI decoding, background/WebSocket indexing, and a compatible write adapter remain product work |
| Arenas and evaluations | **E40 + R03 scoped local service complete** | The checked arena registry now backs authenticated owner-aware lifecycle, attempt submission, external-evidence settlement, atomic leaderboard/prize/reputation effects, and a durable event outbox. Eval orchestration, the seven-stage flywheel, token/on-chain settlement, and transfer detection remain product work |
| Meta-agent lineage and recursive safety | **R04 scoped lifecycle complete** | Owner-scoped durable proposal/activation/morph/rollback/deactivation enforces bounded lineage, non-widening authority, exact five-head evidence, and single-use artifact-bound R03 acceptance. Loop 4, ADAS/HGM, autonomous generated execution, and continuous wrapping of every Flow remain product work |
| DeFi products | **Complete local/stub tranche (E41 8/8)** | Checked instrument, bond, option, insurance, index, affect-sizing, and provider-neutral effect primitives plus authenticated structured 501 routes are tested. Durable/on-chain adapters, a risk engine, and venue execution remain product work |
| Prompt experiments (A/B) | **Runner wired; cross-runtime parity partial** | Runner attempts durably assign and replace canonical sections, bind exact prelaunch prompts, and idempotently settle from archived/live terminal facts. All production outcome writers transact under one sibling lock with scoped IDs. ACP/serve still inject context rather than using the runner receipt protocol |
| Adaptive gate thresholds | **Wired** | EMA per rung in `.roko/learn/gate-thresholds.json`; flush cadence is configurable under `[learning]` |
| Live knowledge tier progression | **Wired** | Successful gate-backed runner ingestion records confirmation/context evidence and evaluates Transient→Working progression |
| Interactive TUI (ratatui) | **Wired** | `crates/roko-cli/src/tui/`, F1–F10 tabs, `roko dashboard` |
| HTTP control plane (~376 canonical routes; ~421 total incl. aliases) | **Wired** | `crates/roko-serve/src/routes/`, `roko serve` on :6677 |
| Extension runtime status | **Wired** | Serve and plan execution share per-workspace chains; collection/detail routes expose metadata plus live circuit health |
| Per-agent sidecar (14 routes) | **Wired** | `crates/roko-agent-server/`, real LLM dispatch (T9) + integration tests (T19) |
| Code-intelligence MCP | **Wired** | `crates/roko-mcp-code/` |
| `roko chat` CLI | **Wired** | `crates/roko-cli/src/chat.rs` |
| Gate rung oracles (4-6) | **Wired** | `runner/gate_dispatch.rs` `build_rung_execution_inputs` |
| C-factor full metrics | **Wired** | runner/ `CFactorSummary` |
| Enrichment in dispatch | **Wired** | runner/ `dispatch_agent_with` |
| Gate failure replan | **Wired** | runner/ `build_gate_failure_plan_revision` |
| PRD auto-plan trigger | **Wired** | roko-serve `prd_publish_subscriber` |
| HDC fingerprint per-episode | **Wired** | Episode `hdc_fingerprint` field, computed + stored |
| Playbook store queries | **Wired (E25)** | Top when/then matches are queried at live dispatch and injected into the system prompt |
| Payments | **Complete (E36 8/8)** | x402 batching, MPP sessions, reputation pricing, paid-feed 402 enforcement, cost persistence, and dashboard events are wired |
| Config evolution | **Complete (E42 8/8)** | Priority/provenance, seven invariants, migrations, merge tracking, profiles, transactional reload, and freshness/doctor diagnostics are wired |
| Cross-cut functors | **Complete (E44 8/8)** | Memory/Daimon/Dreams/Safety functors, six transforms, conflict VCG, and the live non-blocking gate-failure cascade are wired; legacy generic composition can still choose its existing greedy strategy |
| Context bidders (Neuro/Task/Research) | **Wired** | `AttentionBidder` variants in runner/ |
| Safety contracts enforcement | **Complete (E34 8/8 strict)** | The trust-origin lattice/TaintTracker, five-layer immune Graph, five-head corrigibility ordering, five-level sandbox policy, exact capability wrappers, persistent quarantine, and mandatory audited production hooks meet their task contracts. The 2026-08-17 closure adds universal host-visible tool-result screening, canonical-workspace controls, verified output attestations, bounded evidence/checkpoints, provider isolation, tool cooldown/isolation, and linked incidents. Provider-owned internals, trace Signals, adaptive memory, and external ledger authentication remain broader product scope |
| TUI file watcher | **Wired** | `notify::RecommendedWatcher` in `tui/fs_watch.rs` |
| Engram-to-Signal rename (2026-08-12) | **Done** | `Signal` is the primary name across the workspace; `Engram` kept as underlying struct with `pub type Signal = Engram` |
| Code hygiene batch (2026-08-15) | **Done for E12** | E12 is 9/9: the obsolete orchestrator island is removed after live contracts/tests were ported, and the plugin consumer audit ratified `roko-plugin` as the canonical E30/E32 SDK. Broader `eprintln!`/`.expect()` cleanup remains separately tracked. |
| End-to-end dogfood (2026-08-13) | **Fixes landed; rerun pending** | The first run exposed config merge, stale-state, fsmonitor, and enrichment-transition defects; each now has a regression fix. See `tmp/dogfood-2026-08-13/DOGFOOD-DEBRIEF.md` |

### Remaining release/product work

1. **Executable residual queue**: closed at 124/124. New product work must be represented by a reviewed manifest rather than reopening the accepted R01-R04 scopes.
2. **Dogfood re-verification**: the four blockers found by the first 2026-08-13 run have regression fixes and the deterministic self-host coverage passes, but a clean live full self-hosting rerun is still required for that separate sign-off.
3. **Toolchain floor**: alloy dependencies require Rust 1.91 or newer; the green 2026-08-16 release checkpoint used rustc 1.96.1.

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

### 4. Log gaps when finishing work
After completing any implementation task, append unfinished items to `.roko/GAPS.md`.
Include: what's missing, why it wasn't done, and what subsystem it affects.
This file is the canonical gap tracker — check it before starting new work.

## Architecture

The primary protocol noun is `Signal`, backed by the `Engram` struct and its
`pub type Signal = Engram` alias in `roko-core/src/engram.rs`. The kernel exposes 12 traits
(Store, ColdStore, Score, Verify, Route, Compose, React, Bus, Observe, Connect, Trigger,
Substrate). Missing or unknown safety contracts fail closed: unsupported tool use is denied.
The conceptual workflow is query -> score -> route -> compose -> act -> verify -> write ->
react. Production ownership is explicit: `roko run` uses `WorkflowEngine`, plans use
Runner-v2 or Graph, and the core `select_compose_verify_persist` helper covers only the
non-ACT/non-BROADCAST signal-selection subset.

## Self-hosting workflow

This is how roko develops itself. Each step is a CLI command that exists today:

```bash
# 1. Capture a work item
cargo run -p roko-cli -- prd idea "Wire SystemPromptBuilder into runner"

# 2. Draft a PRD from the idea (agent-driven)
cargo run -p roko-cli -- prd draft new "system-prompt-wiring"

# 3. Research the topic for context
cargo run -p roko-cli -- research enhance-prd system-prompt-wiring

# 4. Generate implementation plan + tasks from the PRD
cargo run -p roko-cli -- prd plan system-prompt-wiring

# 5. Execute the plan (agents run tasks, gates validate, state persists)
cargo run -p roko-cli -- plan run plans/ --engine runner-v2

# 6. Resume if interrupted
cargo run -p roko-cli -- plan run plans/ --engine runner-v2 --resume-plan

# 7. Watch progress
cargo run -p roko-cli -- dashboard

# 8. Check status
cargo run -p roko-cli -- status
```

### Opt-in FAST self-development

For an eligible small/local plan with a prebuilt `target/debug/roko`, prefer the bounded wrapper:

```bash
./dev.sh fast plans/<plan-directory>
```

Each FAST task must define exactly one authored `verify` command. FAST tells the provider to hand
off after patching, keeps Cargo out of the provider session, skips critical-path warmup/cleanup,
and captures a private evidence bundle. It is not appropriate for safety, auth, persistence,
migration, payment, or other high-risk changes. FAST evidence does **not** replace the mandatory
pre-commit checks in the Building section.

## CLI commands reference

### Core workflow
| Command | What it does |
|---|---|
| `roko init` | Create `.roko/` directory and `roko.toml` |
| `roko run "<prompt>"` | Single prompt through `WorkflowEngine` (compose -> provider -> gate -> persist) |
| `roko do "<prompt>"` | Execute a task via agent dispatch (used internally by `roko run`) |
| `roko status` | Query signals, report counts and episodes |
| `roko doctor` | Diagnose workspace bootstrap state |
| `roko doctor disk` | Report free space, stale targets, worktrees, and oversized JSONL logs |
| `roko github status` | Inspect GitHub config, authentication, plan PR/CI state, and failure issues |

### Planning & PRDs
| Command | What it does |
|---|---|
| `roko plan list/show/create` | Manage plans |
| `roko plan run <dir> --engine runner-v2` | Execute plans through the live runner-v2 loop |
| `roko plan generate/regenerate` | Generate or regenerate plans from prompts/PRDs |
| `roko plan pause/resume/cancel` | Pause, resume, or cancel a running plan |
| `roko plan retry <dir>` | Retry failed tasks in a plan |
| `roko plan status <dir>` | Show execution status for a plan |
| `roko plan queue` | List queued plans awaiting execution |
| `roko plan validate <dir>` | Lint tasks.toml without executing |
| `roko prd idea "<text>"` | Capture a work item idea |
| `roko prd list/status` | List PRDs, coverage report |
| `roko prd draft new/edit/promote/list` | Draft lifecycle |
| `roko prd plan <slug>` | Generate implementation plan from PRD |
| `roko prd consolidate` | Scan PRDs for gaps and duplicates |

### Agents
| Command | What it does |
|---|---|
| `roko agent create --name X --domain Y` | Create agent from manifest |
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
| `roko research enhance-prd/plan/tasks` | Enhance documents with research |
| `roko research analyze` | Analyze execution data |

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
| `roko config validate/migrate` | Schema validation, legacy migration |
| `roko config set-secret/check-secrets` | Secret management |
| `roko config providers list/health/test` | LLM provider inspection |
| `roko config models list/route` | Model inspection and routing |
| `roko config subscriptions list/add/remove` | Event subscriptions |
| `roko config events` | Configured event sources |
| `roko config experiments` | Model A/B experiments |
| `roko config plugins list/install/remove/audit` | Plugin management |
| `roko config secrets set/get/list/rotate` | Profile-aware secrets |
| `roko config preset gates/routing/budget/model` | Apply validated config presets (with --dry-run, --yes) |

### Server & deployment
| Command | What it does |
|---|---|
| `roko serve` | Start HTTP control plane (~376 canonical routes on :6677) |
| `roko daemon start/stop/status/logs/install` | Daemon lifecycle |
| `roko deploy railway/fly/docker` | Cloud deployment |
| `roko worker` | Run as deployed worker |

### Utilities
| Command | What it does |
|---|---|
| `roko dashboard` | Interactive ratatui TUI (F1–F10 tabs) |
| `roko replay <hash>` | Walk signal DAG by hash |
| `roko inject <session> <payload>` | Signal injection |
| `roko index build/search/stats` | Code intelligence index |
| `roko new <type> <name>` | Scaffold boilerplate |
| `roko explain <topic>` | Concept explainer (3 depth levels) |
| `roko completions <shell>` | Shell completion scripts |

## Key crates

| Crate | Path | What | Status |
|---|---|---|---|
| roko-core | `crates/roko-core/` | Signal + 12 traits, types, config, tools, errors | Kernel, stable |
| roko-agent | `crates/roko-agent/` | 12 LLM provider kinds (AnthropicApi, ClaudeCli, CodexCli, OpenAiCompat, CursorAcp, CursorCli, PerplexityApi, GeminiApi, GeminiCli, CerebrasApi, Hermes, OpenClaw), pools, MCP, tool loop, safety | Dispatch wired, MCP passed |
| roko-agent-server | `crates/roko-agent-server/` | Per-agent HTTP sidecar: `/message` (real LLM dispatch), `/stream` WS, `/predictions`, `/research`, `/tasks` | Wired |
| roko-serve | `crates/roko-serve/` | HTTP control plane: ~376 canonical REST routes (~421 incl. aliases) + SSE + WebSocket on :6677 | Wired |
| roko-gate | `crates/roko-gate/` | 19 gates, 7-rung pipeline, adaptive thresholds | Wired, called per-task |
| roko-compose | `crates/roko-compose/` | Prompt assembly, 11 role templates, enrichment | Wired via RoleSystemPromptSpec |
| roko-conductor | `crates/roko-conductor/` | 12 watchers, circuit breaker, diagnosis | Used by executor internals |
| roko-learn | `crates/roko-learn/` | Episodes, playbooks, bandits, model routing, experiments, efficiency | ACP dispatch-time experiment assignment is live; generic runner assignment and other gaps are tracked in GAPS.md |
| roko-cli | `crates/roko-cli/` | CLI, plan DAG/runner, merge queue, worktree manager, ratatui TUI | Main execution entry point |
| roko-fs | `crates/roko-fs/` | FileSubstrate (JSONL), GC, layout | Stable |
| roko-std | `crates/roko-std/` | 35 definitions by default (16 executable local + 19 GitHub MCP); 52 with typed optional-chain placeholders; HTTP MCP clients/resolvers retained at runtime | Partial because optional-chain entries remain typed placeholders |
| roko-runtime | `crates/roko-runtime/` | ProcessSupervisor, event bus, cancellation | Wired into PlanRunner |
| roko-primitives | `crates/roko-primitives/` | HDC vectors, tier routing | Fully wired (tier routing + HDC fingerprint-per-episode) |
| roko-neuro | `crates/roko-neuro/` | Durable knowledge store, distillation, tier progression | Wired |
| roko-mcp-code | `crates/roko-mcp-code/` | Code-intelligence MCP server | Wired |
| roko-mcp-github / slack / scripts / stdio | `crates/roko-mcp-*/` | Additional MCP integrations | Partial; see `tmp/ux-followup/05-partially-wired-subsystems.md` |
| roko-index | `crates/roko-index/` | Parser + graph + HDC indexing | Built |
| roko-lang-rust / typescript / go | `crates/roko-lang-*/` | Language support | Built |
| roko-dreams | `crates/roko-dreams/` | Offline consolidation (hypnagogia, imagination, cycle) | Resident daemon scheduling is live for adaptive idle, cron, and episode-count triggers with idle queuing and checkpoint restore. Bus-reactive/intensive backlog controls remain partial |
| roko-daimon | `crates/roko-daimon/` | Affect engine, somatic markers, dispatch modulation | Wired (DaimonState loaded + used per-task in runner/) |
| roko-acp | `crates/roko-acp/` | ACP (Agent Client Protocol) server for Cursor/external agent integration | E17 8/8; 180 ACP tests pass |
| roko-plugin | `crates/roko-plugin/` | Plugin manifests, executable declarative tools, canonical tier/capability policy, semantic-version/dependency resolution | 8/8 manifest; signed dependency ranges, range-aware relay/CLI graph validation, strict admission, kernel confinement, verified registry install/publish, fail-fast startup, bounded all-23-hook WASM, Claude/Codex MCP, Cursor/Hermes ACP, and native authenticated Gemini CLI MCP are live. Component-model Store/Bus hostcalls plus OpenClaw/legacy one-shot parity are open |
| roko-graph | `crates/roko-graph/` | Graph data structures, DAG operations | Partial runtime: bounded parallel waves, conditional routing, live provider dispatch, actual paid-failure-aware provider-cost enforcement/reporting, atomic reservations, resume-durable schema-v2 cost state, exact graph-fingerprinted Activity resume, restart-durable Hot tick/output/budget checkpoints, seven cognitive Cells, five Verify Cells, and the immune decision Graph are wired. Runner-v2 gates/replan/approval/worktree/merge/full-persistence/cancellation parity remains; a single call can report more than its reservation because providers expose exact cost only after completion |
| roko-demo | `crates/roko-demo/` | Demo/example binary for showcasing features | Built |
| roko-chain | `crates/roko-chain/` | Optional chain client/runtime primitives plus tested local registry, marketplace, arena, and DeFi state machines. daeji owns node/BFT/precompiles in a separate repo. | Production transport, persistence, authorization, indexing, and execution adapters remain Phase 2+; legacy rate-oracle vertical removed |

## Absolute paths

| What | Path |
|---|---|
| **Workspace root** | `/Users/will/dev/nunchi/roko/roko/` |
| **All crates** | `/Users/will/dev/nunchi/roko/roko/crates/` |
| **CLI source** | `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/` |
| **Runner (event loop)** | `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/event_loop.rs` |
| **Agent dispatcher** | `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/dispatcher/mod.rs` |
| **Safety layer** | `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/safety/` |
| **System prompt builder** | `/Users/will/dev/nunchi/roko/roko/crates/roko-compose/src/system_prompt_builder.rs` |
| **Role templates** | `/Users/will/dev/nunchi/roko/roko/crates/roko-compose/src/templates/` |
| **Master task list** | `/Users/will/dev/nunchi/roko/roko/tmp/MASTER-TASKS.md` |
| **Roko data dir** | `/Users/will/dev/nunchi/roko/roko/.roko/` |
| **Executor snapshots** | `/Users/will/dev/nunchi/roko/roko/.roko/state/` |
| **PRD storage** | `/Users/will/dev/nunchi/roko/roko/.roko/prd/` |
| **Research artifacts** | `/Users/will/dev/nunchi/roko/roko/.roko/research/` |
| **Signal log** | `/Users/will/dev/nunchi/roko/roko/.roko/engrams.jsonl` |
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

Priority order for reaching full self-hosting:

1. ~~**Fix rustc**~~ → Done. Requires 1.91+ (`rustup default stable`).
2. ~~**Wire SystemPromptBuilder**~~ → Done. `RoleSystemPromptSpec` uses 9-layer builder + templates.
3. ~~**Wire EpisodeLogger**~~ → Done. Agent turns + gate results → `.roko/episodes.jsonl`.
4. ~~**Wire ProcessSupervisor**~~ → Done. `PlanRunner` tracks agents via `roko-runtime`.
5. ~~**Wire MCP**~~ → Done. CLI passthrough and executable HTTP-provider discovery/resolution are covered by a real stdio `tools/call` round trip.
6. **Learning & feedback core** → Efficiency, cascade persistence, configurable adaptive thresholds, playbooks, live knowledge tier progression, and crash/restart-durable runner prompt experiments are wired. ACP/serve experiment injection still needs canonical-section/receipt parity.
7. ~~**Interactive TUI**~~ → Done. ratatui wired; F1–F10 tabs, T1–T19 parity batches merged via PR #13.
8. ~~**Per-agent sidecar**~~ → Done. `roko-agent-server` real-dispatch path (T9) + integration tests (T19).
9. ~~**HTTP control plane**~~ → Done. `roko-serve` exposes ~376 canonical routes (~421 incl. aliases; generated 2026-09-03 via `python3 tools/http_route_inventory.py`, method+path counting, feature-gated/alias-duplicated routes included in the static scan) for dashboards / external callers.
10. ~~**Automatic plan generation**~~ → Done. `prd.auto_plan` config triggers `prd plan` on publish via `spawn_prd_publish_subscriber`.
11. ~~**Feedback loop**~~ → Done. `learning_config.replan_on_gate_failure` triggers `build_gate_failure_plan_revision`.
12. ~~**Follow-up catalog**~~ → Done. Most items verified/closed; see `tmp/ux-followup/00-INDEX.md`.

Roko can now fully self-host: read PRDs, generate plans, execute them, validate results,
learn from failures, and iterate. Remaining work:

13. ~~**Knowledge-informed agent routing**~~ → Done. Runner-v2 consults the neuro store and passes model-specific advice into cascade selection.
14. ~~**Cold substrate archival**~~ → Done. The server runs the configurable age-based archival policy and prunes hot data only after archive success.
15. ~~**Complete E33 telemetry producer integration**~~ → Done. All 11 built-in Lens executors, bounded delivery, breaker controls, restart-durable projection history, resolution queries, configurable 7-day time retention, central producer fanout, and all 39 production event variants are wired. E23 is also 10/10; direct native Agent publication into the accepted E33 observation ingress remains separate product integration work.
16. ~~**Complete strict E34 acceptance**~~ → Done at 8/8. Trust-origin taint/TaintTracker contracts, exact capability wrappers, persistent transitive incident handling, and mandatory audited production hooks now join the immune, corrigibility, and sandbox foundations. The 2026-08-17 closure screens canonical provider primary outputs and all host-visible tool results, keeps durable controls outside disposable attempt worktrees, and enacts provider isolation plus tool cooldown/isolation. Provider-owned internals, trace Signals, adaptive immune memory, and externally anchored whole-ledger authenticity remain broader product scope.
17. **UX34: force_backend override learning** → cascade router doesn't learn from manual overrides.
18. **Chain and economic runtime integration** → Local witness, x402, marketplace,
    registry, arena, and DeFi primitives exist, but production work spans several adapter
    classes: daeji-backed contracts/consensus, network transport and indexing, durable
    services, caller authorization, market data, risk admission, and venue execution. The
    deprecated rate-oracle vertical has been removed. See `.roko/GAPS.md` for boundaries.
19. **Fresh dogfood proof** → rerun the complete self-hosting workflow against the regression fixes for config layering, stale snapshots, fsmonitor, and authored-plan enrichment transitions.
20. ~~**Find `--bare` equivalent**~~ → Done. Claude CLI's supported `--system-prompt`
    replaces its built-in prompt when `bare_mode = true`; full mode continues to use
    `--append-system-prompt`. MCP and tool policy remain independent.

For detailed implementation status and product/release residuals, see `.roko/GAPS.md`.
