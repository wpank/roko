# 00 — Backlog Index

> Navigation layer for the roko **executable backlog**.
> Repo HEAD `5852c93c05` (branch `main`) · authored 2026-07-09 ·
> root `/Users/will/dev/nunchi/roko/roko`.
> Parent pack index: [`../00-INDEX.md`](../00-INDEX.md).

## What this is

This backlog turns the findings of the 107-doc **status-quo pack**
(`tmp/status-quo/00–106`) and the full **v2 specification** (`docs/v2/`, `docs/v2-depth/`)
into roko-native, **agent-executable tasks**. Every task is
authored to the canonical `tasks.toml` schema that `crates/roko-cli/src/task_parser.rs`
deserializes and `plan_validate.rs` enforces — each carries verify commands, gates, and
acceptance criteria, grouped into **48 epics (E01–E48)**, 447 implementation tasks total, and
71 DOC tasks.

- **E01–E18** (169 tasks): Status-quo audit findings — path to self-hosting (M0–M3+).
- **E19–E45** (243 tasks): v2 specification implementation — full feature coverage. Where the
status-quo audit says an issue is open, the task that targets it is still current: the
audit and this backlog share the **same HEAD** (`5852c93c05`).

> ### ⚠ BOOTSTRAP — read this first
> **Roko cannot self-execute this backlog until E01 lands.** `roko plan run <dir>` with no
> flags currently defaults to the **Graph engine**, a dry-run stub: it prints `SUCCESS` in
> ~2 s, spawns 0 agents, spends $0, and changes no files. An autonomous agent following the
> self-hosting workflow will "run" every epic, see green, and do nothing. **E01 (flip the
> engine default to Runner v2)** is the gate on everything else. Until it lands, run plans
> explicitly with `--engine runner-v2`. Details: [`04-EXECUTION-READINESS.md`](04-EXECUTION-READINESS.md).

## Start here

| Doc | What it gives you |
|---|---|
| [`03-WORK-BREAKDOWN-EPICS.md`](03-WORK-BREAKDOWN-EPICS.md) | Roadmap, epic DAG, milestone sequencing (M0→M3+), critical path, parallel tracks |
| [`05-MASTER-CHECKLIST.md`](05-MASTER-CHECKLIST.md) | Historical 149-task E01-E18 seed checklist; the expanded executable layer contains 169 E01-E18 tasks |
| [`06-EXECUTABLE-TASK-FILE-COVERAGE.md`](06-EXECUTABLE-TASK-FILE-COVERAGE.md) | Canonical coverage ledger for all 48 epic manifests: 447 implementation tasks (169 in E01-E18), 0 definition gaps |
| [`07-SUBAGENT-TASK-AUTHORING-NOTES.md`](07-SUBAGENT-TASK-AUTHORING-NOTES.md) | Subagent-derived corrections for missing task blocks: stale paths, deps, scopes, and verify hints |
| [`08-SOURCE-CORPUS-PLAN-COVERAGE.md`](08-SOURCE-CORPUS-PLAN-COVERAGE.md) | Source-corpus coverage ledger: all 745 docs from `tmp/status-quo`, `docs/v1`, `docs/v2`, and `docs/v2-depth` mapped into DOC plans |
| [`plans/00-INDEX.md`](plans/00-INDEX.md) | Runnable per-epic `tasks.toml` layer plus the `status-quo-authoring-gaps` plan |
| [`04-EXECUTION-READINESS.md`](04-EXECUTION-READINESS.md) | **M0 bootstrap** — the gate before every epic; the one fix that unblocks self-execution |
| [`01-TASK-EXECUTION-SCHEMA.md`](01-TASK-EXECUTION-SCHEMA.md) | Canonical `tasks.toml` schema — how to author a roko-executable task |
| [`02-PLANS-RECONCILIATION.md`](02-PLANS-RECONCILIATION.md) | Bridge from the authored `plans/` backlog to the status-quo findings (currency + coverage map) |
| [`12-MILESTONE-DEFINITIONS.md`](12-MILESTONE-DEFINITIONS.md) | Detailed milestone specs (M0-M2, Phase 1-3, Phase 3+): entry/exit criteria, effort estimates, risks, verification commands |
| [`09-UNIFIED-ROADMAP.md`](09-UNIFIED-ROADMAP.md) | Unified roadmap across all 48 epics with milestone dependency diagram, critical path, effort estimates |
| [`10-EPIC-DEPENDENCY-MATRIX.md`](10-EPIC-DEPENDENCY-MATRIX.md) | Cross-epic dependency DAG, adjacency matrix, topological sort, parallel opportunity analysis |
| [`11-EXECUTION-PLAYBOOK.md`](11-EXECUTION-PLAYBOOK.md) | Step-by-step operations manual: M0 bootstrap, parallel execution, resource/rate-limit management, failure recovery |
| [`13-PLAN-AUDIT-E19-E30.md`](13-PLAN-AUDIT-E19-E30.md) | Schema compliance and quality audit for v2 kernel + infrastructure plans |
| [`14-PLAN-AUDIT-E31-E42.md`](14-PLAN-AUDIT-E31-E42.md) | Schema compliance and quality audit for v2 operations + economy plans |
| [`15-PLAN-AUDIT-E43-E48.md`](15-PLAN-AUDIT-E43-E48.md) | Schema compliance and quality audit for meta + operational capability plans |
| [`16-FINAL-GAP-ANALYSIS.md`](16-FINAL-GAP-ANALYSIS.md) | Final gap analysis: v2 coverage proof + 3 identified gaps (GitHub/resources/rate-limits) remediated by E46-E48 |

## Epics

Task count = distinct authored `EXX-Tnn` IDs in the epic file. Milestone/gate is stated
where the epic declares it; otherwise the epic's own **Depends on** is shown (most gate on
E01). The full M0→M3+ sequencing, DAG, and critical path live in
[`03-WORK-BREAKDOWN-EPICS.md`](03-WORK-BREAKDOWN-EPICS.md).

| Epic | Title | Milestone / gate | Tasks | Goal |
|---|---|---|---|---|
| [E01](epics/E01-EXECUTION-ENGINE.md) | Execution Engine | **M0 — bootstrap** | 16 | Make bare `plan run` spawn real agents, run gates, persist episodes/snapshots, resume — flip the engine default off the dry-run Graph |
| [E02](epics/E02-STORAGE-CONVERGENCE.md) | Storage Convergence | dep E01 | 12 | One canonical writer per durable `.roko/` concern so dashboards read what gates actually write |
| [E03](epics/E03-TYPE-CONSOLIDATION.md) | Type Consolidation | unblocks E02/E10 | 7 | Collapse 19 cross-crate duplicate type families to single definitions with real conversions |
| [E04](epics/E04-SECURITY-PERIMETER.md) | Security Perimeter | self-exec prereq | 19 | Close three exploitable P0s and enforce the safety funnel + audit chain before unattended self-execution |
| [E05](epics/E05-GATE-ADAPTIVITY-LIVE.md) | Gate Adaptivity on the Live Path | dep E01 | 8 | Make the live gate path honest: real rung inputs, non-passing stubs, per-rung stats that persist |
| [E06](epics/E06-COMPOSE-UNIFY.md) | Compose / Prompt Unification | dep E01 | 9 | Route the default Runner-v2 prompt path through the canonical roko-compose stack; retire 4 parallel assemblers |
| [E07](epics/E07-LEARNING-KNOWLEDGE.md) | Learning & Knowledge Loops | dep E01 | 10 | Make write-only learning loops durable & closed (persist LinUCB, credit the knowledge economy, wire HDC) |
| [E08](epics/E08-CONDUCTOR-SUPERVISION.md) | Conductor Supervision | dep E01 | 9 | Wire reactive anomaly supervision (ghost-turn, compile-loop, cost-blowout) into the live event loop |
| [E09](epics/E09-OBSERVABILITY.md) | Observability | dep E01 | 11 | Thread the built `MetricRegistry` into `RunConfig`, rotate runaway logs, give operators a trustworthy window |
| [E10](epics/E10-FRONTEND-CONTRACT.md) | Frontend / API Contract | dep E03 | 7 | Fix the web dashboard's wire contract with `roko serve` (404s, camel/snake, double SSE, replay) |
| [E11](epics/E11-CHAIN-ISFR.md) | Chain / ISFR | Phase 2+ (subset now) | 5 | Recover the core queue, implement `get_logs`, reach deploy parity for the DeFi critical-path subset (client side only) |
| [E12](epics/E12-DEAD-CODE-CLEANUP.md) | Dead-Code & Legacy Cleanup | dep E05/E06/E08 | 9 | Delete the ~52K-LOC legacy `orchestrate.rs` island after its live value is ported out |
| [E13](epics/E13-SPEC-DEBT-V2.md) | v2 Spec-Debt (long-horizon) | **M3+** | 3 | Triage ~55 zero-code v2 concepts; author tasks only for load-bearing survivors (e.g. `Lens`) — must not gate M0–M2 |
| [E14](epics/E14-PROVIDERS-TOOLS.md) | Providers & Tools | dep E01 | 12 | Harden the dispatch path: retries retry, tools survive per provider, every advertised builtin is executable |
| [E15](epics/E15-MCP-CONFIG.md) | MCP Config & Passthrough | dep E01 | 7 | Fix the MCP seams so tools actually reach the agent (config-shape normalizer first) |
| [E16](epics/E16-PRD-SELF-HOSTING.md) | PRD & Self-Hosting Pipeline | dep E01/E14 | 2 | Close the generative front-half (idea→draft→research→plan); 2 gap tasks atop plans P08/P09/P23 |
| [E17](epics/E17-ACP-COMPLETION.md) | ACP Completion | dep E04/E07/E15 | 8 | Make an editor-driven ACP turn behave like a `plan run` turn: consent-gated, learning-informed, MCP-equipped, honest |
| [E18](epics/E18-DOCS-CONFIG-OPS.md) | Docs, Config, CI & Ops Hygiene | dep E01 | 15 | Stop the repo lying to its readers and make the release pipeline prove what it claims |

### Phase 1 — Kernel Upgrade (E19–E22, 40 tasks)

| Epic | Title | Milestone / gate | Tasks | Goal |
|---|---|---|---|---|
| E19 | Signal Protocol | Phase 1 (dep E01) | 10 | Graduation, Pulse bridges, demurrage economics, HDC fingerprints, IFC taint, Kind registry |
| E20 | Cell Unification | Phase 1 (dep E01) | 10 | Cell supertrait with 9 protocols, TypeSchema, predict-publish-correct, CellContext, CellRegistry |
| E21 | Graph Engine | Phase 1 (dep E20) | 10 | Typed edge validation, Hot Graphs, Workflow/Activity split, parallel waves, snapshot/resume, merge queue |
| E22 | Execution Runtime | Phase 1 (dep E20, E21) | 10 | 7 cognitive loop Cells, nested gamma/theta/delta loops, T0 short-circuit, error taxonomy, budget, replay |

### Phase 2 — Agent Cognition (E23–E26, 42 tasks)

| Epic | Title | Milestone / gate | Tasks | Goal |
|---|---|---|---|---|
| E23 | Agent Cognitive Autonomy | Phase 2 (dep E19, E20) | 10 | Type-state machine, behavioral phases, CorticalState, EFE routing, emergent goals, energy budget |
| E24 | Memory Advanced | Phase 2 (dep E07) | 10 | Heuristics with falsifiers, Allen intervals, resonator networks, income policy, dream triggers, ODE tuning |
| E25 | Learning Loops Advanced | Phase 2 (dep E07) | 10 | L3 HDC defragmentation, L4 c-factor governance, experiment lifecycle, playbooks, variance inequality |
| E26 | Inference Gateway | Phase 2 (dep E14) | 12 | 9-stage pipeline (loop detect→cache→prune→budget→think→converge→call→store→track), Batch API |

### Phase 2 — Infrastructure (E27–E32, 49 tasks)

| Epic | Title | Milestone / gate | Tasks | Goal |
|---|---|---|---|---|
| E27 | Feeds System | Phase 2 (dep E19, E20) | 8 | Feed trait, registry, raw/derived/composite taxonomy, recipes, marketplace, config |
| E28 | Groups & Coordination | Phase 2 (dep E20) | 8 | Group as Space, 4 coordination modes, membership, pheromone fields, shared knowledge |
| E29 | Connectivity & Relay | Phase 2 (dep E04) | 9 | Connect protocol, relay wire protocol, A2A cards, reconnection FSM, backpressure, exoskeleton adapters |
| E30 | Extension System | Phase 2 (dep E20) | 8 | Extension trait, 22 hooks, CaMeL IFC, discovery/resolution, circuit breaking, lifecycle |
| E31 | Trigger System | Phase 2 (dep E08) | 8 | Trigger as Cell, event source registry, bindings, debounce/filter, Bus topics |
| E32 | Tool & Plugin Ecosystem | Phase 2 (dep E14, E15) | 8 | Plugin SDK (5-tier SPI), dynamic loading, capability binding, sandboxing, catalog validation |

### Phase 2 — Operations & Security (E33–E35, 25 tasks)

| Epic | Title | Milestone / gate | Tasks | Goal |
|---|---|---|---|---|
| E33 | Telemetry & Lens | Phase 2 (dep E09) | 9 | 7 StateHub projections, Lens stacking, Observe protocol, c-factor computation, circuit breaker |
| E34 | Security IFC | Phase 2 (dep E04) | 8 | Taint lattice, immune system 5-layer pipeline, 5-head corrigibility, sandbox levels, quarantine |
| E35 | Auth Protocol | Phase 2 (dep E04) | 8 | API key rotation, agent tokens, JWKS caching, team RBAC, relay tokens, invitations, audit trail |

### Phase 3 — Economy (E36–E41, 50 tasks)

| Epic | Title | Milestone / gate | Tasks | Goal |
|---|---|---|---|---|
| E36 | Payments | Phase 3 (dep E11, E29) | 8 | x402 per-request, MPP session-based, reputation pricing, settlement batching |
| E37 | Surfaces | Phase 2+ (dep E09, E33) | 9 | 5 named surfaces (Workbench, Inbox, Canvas, Minimap, Autonomy Slider), 12 object types |
| E38 | Marketplace | Phase 3 (dep E36, E39) | 9 | Agent passport, TraceRank reputation, publish/discover/fork, Package SPI, DAW composability |
| E39 | Registries & Identity | Phase 3 (dep E11) | 8 | ERC-8004 transferable identity, ZK-HDC, on-chain InsightStore, gossip, job market |
| E40 | Arenas & Evals | Phase 3 (dep E25, E39) | 8 | 7-step flywheel, scoring functions, leaderboards, bounty escrow, arena-to-learning pipeline |
| E41 | DeFi Products | Phase 3 (dep E11, E39) | 8 | VCG clearing Cell, yield perpetuals, VenueAdapter, DeFiRiskEngine, affect-modulated sizing |

### Phase 2+ — Meta (E42–E45, 34 tasks)

| Epic | Title | Milestone / gate | Tasks | Goal |
|---|---|---|---|---|
| E42 | Config Evolution | Phase 2 (dep E19) | 8 | Config-as-Signal, schema versioning, 7 invariants, hot-reload trigger, priority merge |
| E43 | Deployment & Portability | Phase 2+ (dep E18) | 8 | Brain export/import (Merkle-CRDT), daemon lifecycle, secrets rotation, tier advisor |
| E44 | Cross-Cut Functors | Phase 2 (dep E19, E20) | 8 | Endofunctor algebra (Memory/Daimon/Dreams), natural transformations, VCG arbitration, safety wrapper |
| E45 | Orchestrator Mori Parity | Phase 2 (dep E01, E12) | 10 | Structured review, auto-fix, error sharing, reflection loop, context scoping, warm spawn |

### Operational Capabilities (E46–E48, 35 tasks)

| Epic | Title | Milestone / gate | Tasks | Goal |
|---|---|---|---|---|
| E46 | GitHub Workflow Integration | Phase 1 (dep E01, E04, E15) | 12 | GitHub-native self-development: branch/PR/issue/review/merge automation, CI awareness, webhook triggers |
| E47 | Resource & Disk Management | Phase 1 (dep E01, E02) | 11 | Disk monitoring, worktree lifecycle, artifact cleanup, GC wiring, log rotation, parallel execution awareness |
| E48 | Rate Limit & Token Budgeting | Phase 1 (dep E01, E14) | 12 | Per-provider rate tracking, retry with backoff, token budget enforcement, cost ceiling, graceful degradation |

Total: **447 authored implementation tasks** across 48 epics + 71 DOC reconciliation tasks.

## Exemplars & references

**Exemplars** — drop-in `tasks.toml` plans, each `roko plan validate`-clean and checked
against HEAD `5852c93c05`. Copy their shape when authoring real backlog plans.

| File | Demonstrates |
|---|---|
| [`exemplars/EX01-flip-engine-default.toml`](exemplars/EX01-flip-engine-default.toml) | The E01/M0 bootstrap task — flip the engine default |
| [`exemplars/EX02-unify-signal-store.toml`](exemplars/EX02-unify-signal-store.toml) | The E02 flagship — unify the signal store |
| [`exemplars/EX03-delete-orphan-statehub.toml`](exemplars/EX03-delete-orphan-statehub.toml) | An E12-style deletion task — delete an orphan StateHub |

**Executable task-file layer** — generated under [`plans/`](plans/) so Roko can start working
from task files instead of prose-only epic markdown. The layer contains 48 per-epic plan
directories with all **447** implementation tasks from the master checklist. The old
[`plans/status-quo-authoring-gaps/tasks.toml`](plans/status-quo-authoring-gaps/tasks.toml) plan is
kept only as skipped/superseded provenance after its 96 authoring tasks were consumed.
See [`06-EXECUTABLE-TASK-FILE-COVERAGE.md`](06-EXECUTABLE-TASK-FILE-COVERAGE.md) for exact
coverage and validation notes.

**Source-corpus reconciliation layer** — generated under `plans/DOC-*` with six DOC plan
directories and 71 grouped reconciliation tasks. This layer covers all **745** source documents
from `tmp/status-quo/*.md`, `docs/v1/**`, `docs/v2/**`, and `docs/v2-depth/**`. The per-corpus
ledgers live in [`source-coverage/`](source-coverage/) and the aggregate proof lives in
[`08-SOURCE-CORPUS-PLAN-COVERAGE.md`](08-SOURCE-CORPUS-PLAN-COVERAGE.md).

**References**

| File | What |
|---|---|
| [`references/PLANNING-METHODOLOGY.md`](references/PLANNING-METHODOLOGY.md) | Cited (2024–2026) best practice for decomposing/sizing/gating agent-executable work, mapped to roko's schema |
| [`GAP-REPORT-V3.md`](GAP-REPORT-V3.md) | Coverage gaps in the status-quo pack (real-code-undocumented vs spec-only), feeding E13 spec-debt |

## How to execute

A human or a roko agent runs an epic like this:

1. Pick an epic (start with **E01** — nothing else is trustworthy until it lands).
2. Prefer the generated plan directory under `tmp/status-quo/backlog/plans/<epic>/tasks.toml`.
   Every checklist task is now represented directly in its per-epic plan file.
3. Lint without executing:
   `cargo run -p roko-cli --bin roko -- plan validate tmp/status-quo/backlog/plans/<name>`.
4. Execute on the live engine:
   `cargo run -p roko-cli --bin roko -- plan run tmp/status-quo/backlog/plans/<name> --engine runner-v2`.

> The explicit `--engine runner-v2` is **mandatory until E01 lands** — the bare default is
> the dry-run Graph stub. Once E01 flips the default, `roko plan run plans/<name>` alone
> does real work. See [`04-EXECUTION-READINESS.md`](04-EXECUTION-READINESS.md) and
> [`01-TASK-EXECUTION-SCHEMA.md`](01-TASK-EXECUTION-SCHEMA.md).

---

_Back to the full status-quo pack: [`../00-INDEX.md`](../00-INDEX.md)._
