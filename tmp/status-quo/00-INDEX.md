# Status Quo Index

Generated 2026-07-07; re-audited 2026-07-08; backlog expanded 2026-07-09/10 for
`main` branch at `5852c93c05`.

This folder is the current-state pack for Roko: what exists, what is live, what is
partial, what is stale, and what to do next.

**Summary statistics**: 108 numbered analysis docs (00-106 + DOC-MANIFEST),
48 epics (E01-E48) with executable `tasks.toml` plan directories,
6 DOC reconciliation plans, 389+ authored implementation tasks,
71 DOC reconciliation tasks, 18 epic markdown specs, 6 source-coverage ledgers,
3 exemplar plans.

---

## Reading Guide

**If you are new to this pack**, follow this path:

1. **Read [95-ENGINE-DRIFT.md](95-ENGINE-DRIFT.md) first.** The single most
   load-bearing cross-cutting fact: `roko plan run` defaults to the Graph Engine,
   a dry-run stub (prints SUCCESS, spawns no agent, $0, no code change). Real
   execution is Runner v2 (`--engine runner-v2`, implicit for `do`/`serve`/`prd`/`worker`).
2. **Read [01-EXECUTIVE-SUMMARY.md](01-EXECUTIVE-SUMMARY.md)** for the current
   truth in one pass: P0 problems, what is real, what is not.
3. **Read [13-CURRENT-STATE-MATRIX.md](13-CURRENT-STATE-MATRIX.md)** for the
   system-by-system shipped/partial/stub matrix.
4. **Read [12-ROADMAP.md](12-ROADMAP.md)** for the risk-ordered implementation
   sequence with proof gates.
5. **Go to [backlog/00-INDEX.md](backlog/00-INDEX.md)** for the executable work
   plan: 48 epics, 389+ tasks, milestones M0-M3+.

**For depth on a specific subsystem**, use the topic-organized sections below
to find the relevant evidence ledger (30-106). Each ledger contains `file:line`
evidence from the current codebase.

**For spec coverage**, start at [102-SPEC-DEBT-LEDGER.md](102-SPEC-DEBT-LEDGER.md)
which synthesizes 85/86/87 into one concept-to-status register, then drill into
the per-layer matrices (14, 15, 18, 85-87).

**For the full doc taxonomy** (category, subsystem, size, consolidation
recommendations), see [DOC-MANIFEST.md](DOC-MANIFEST.md).

---

## 1. Start Here -- Navigation Layer

These 11 documents form the decision and navigation layer. Read them in this order.

| # | File | Purpose | Epic cross-ref |
|---|---|---|---|
| 95 | [ENGINE-DRIFT](95-ENGINE-DRIFT.md) | **Cross-cut**: 3 plan engines, hollow default, projection seam | E01 |
| 01 | [EXECUTIVE-SUMMARY](01-EXECUTIVE-SUMMARY.md) | Current truth in one pass; P0 problems, what is real | All |
| 13 | [CURRENT-STATE-MATRIX](13-CURRENT-STATE-MATRIX.md) | System-by-system Live/Partial/Stub/Legacy/Stale matrix | All |
| 12 | [ROADMAP](12-ROADMAP.md) | Risk-ordered implementation roadmap with proof gates | E01-E18 |
| 24 | [OPEN-ISSUE-LEDGER](24-OPEN-ISSUE-LEDGER.md) | P0/P1/P2 debt register with evidence + done-when criteria | E01-E18 |
| 25 | [PROOF-GATES](25-PROOF-GATES.md) | Runnable acceptance commands that prove a fix is done | E01-E18 |
| 26 | [CANONICAL-DECISIONS](26-CANONICAL-DECISIONS.md) | Architecture decisions to stop name/path drift | E03, E06, E12 |
| 27 | [IMPLEMENTATION-BACKLOG](27-IMPLEMENTATION-BACKLOG.md) | Risk-sliced actionable backlog; every item has a proof gate | E01-E18 |
| 28 | [DEFINITION-OF-DONE](28-DEFINITION-OF-DONE.md) | Done criteria for migration: default path + state + test + docs | E01-E18 |
| 29 | [RISK-REGISTER](29-RISK-REGISTER.md) | Operational risks with mitigations and triggers | E01, E04 |

## 2. Execution Backlog

The [`backlog/`](backlog/00-INDEX.md) subfolder turns this pack's findings into a
roko-executable work plan: **48 epics (E01-E48)**, **389+ implementation tasks** +
**71 DOC reconciliation tasks** in roko's native `tasks.toml` schema, grouped into
4 milestones (M0 bootstrap through M3+).

> **BOOTSTRAP WARNING**: Roko cannot self-execute this backlog until E01 lands.
> Use `--engine runner-v2` explicitly until then. See
> [backlog/04-EXECUTION-READINESS.md](backlog/04-EXECUTION-READINESS.md).

### Core backlog documents

| File | Purpose |
|---|---|
| [backlog/00-INDEX.md](backlog/00-INDEX.md) | Backlog navigation + full 48-epic table + how-to-execute |
| [backlog/03-WORK-BREAKDOWN-EPICS.md](backlog/03-WORK-BREAKDOWN-EPICS.md) | Master roadmap: epic dependency DAG, milestones, critical path, parallel tracks |
| [backlog/05-MASTER-CHECKLIST.md](backlog/05-MASTER-CHECKLIST.md) | Flat, tickable checklist of all tasks by milestone |
| [backlog/04-EXECUTION-READINESS.md](backlog/04-EXECUTION-READINESS.md) | M0 bootstrap -- the gate before every epic |
| [backlog/01-TASK-EXECUTION-SCHEMA.md](backlog/01-TASK-EXECUTION-SCHEMA.md) | Canonical `tasks.toml` schema for authoring executable tasks |
| [backlog/02-PLANS-RECONCILIATION.md](backlog/02-PLANS-RECONCILIATION.md) | Maps existing `plans/` (P08-P34) to findings; currency/coverage |

### Coverage and quality documents

| File | Purpose |
|---|---|
| [backlog/06-EXECUTABLE-TASK-FILE-COVERAGE.md](backlog/06-EXECUTABLE-TASK-FILE-COVERAGE.md) | Coverage ledger for materialized task files (149 implementation tasks, 0 gaps) |
| [backlog/07-SUBAGENT-TASK-AUTHORING-NOTES.md](backlog/07-SUBAGENT-TASK-AUTHORING-NOTES.md) | Corrections for missing task blocks: stale paths, deps, scopes |
| [backlog/08-SOURCE-CORPUS-PLAN-COVERAGE.md](backlog/08-SOURCE-CORPUS-PLAN-COVERAGE.md) | Source-corpus coverage: 744 docs mapped into DOC plans |
| [backlog/GAP-REPORT-V3.md](backlog/GAP-REPORT-V3.md) | Coverage gaps feeding E13 spec-debt |

### Executable plans

| Path | What |
|---|---|
| [backlog/plans/](backlog/plans/00-INDEX.md) | 48 per-epic `tasks.toml` plan directories (E01-E48) + 6 DOC plans + authoring-gaps plan |
| [backlog/epics/](backlog/epics/) | 18 epic markdown files (E01-E18) with granular task specs |
| [backlog/exemplars/](backlog/exemplars/) | 3 drop-in `plan validate`-clean `tasks.toml` exemplars |
| [backlog/source-coverage/](backlog/source-coverage/) | 6 per-corpus coverage ledgers |
| [backlog/references/PLANNING-METHODOLOGY.md](backlog/references/PLANNING-METHODOLOGY.md) | Cited best-practice for sizing/gating agent-executable work |

### Epic summary (E01-E48)

**Status-quo audit epics (E01-E18, 149 tasks)**:

| Epic | Title | Milestone | Tasks | Addresses findings in |
|---|---|---|---|---|
| [E01](backlog/epics/E01-EXECUTION-ENGINE.md) | Execution Engine | M0 bootstrap | 10 | 95, 96, 36, 37, 92, 98 |
| [E02](backlog/epics/E02-STORAGE-CONVERGENCE.md) | Storage Convergence | M1 | 12 | 55, 60, 32, 97, 76 |
| [E03](backlog/epics/E03-TYPE-CONSOLIDATION.md) | Type Consolidation | M1 | 7 | 103, 47, 30 |
| [E04](backlog/epics/E04-SECURITY-PERIMETER.md) | Security Perimeter | M0/M1 | 19 | 75, 99, 33 |
| [E05](backlog/epics/E05-GATE-ADAPTIVITY-LIVE.md) | Gate Adaptivity (Live) | M1 | 8 | 101, 35 |
| [E06](backlog/epics/E06-COMPOSE-UNIFY.md) | Compose / Prompt Unification | M1 | 9 | 34 |
| [E07](backlog/epics/E07-LEARNING-KNOWLEDGE.md) | Learning & Knowledge Loops | M1 | 10 | 40, 39 |
| [E08](backlog/epics/E08-CONDUCTOR-SUPERVISION.md) | Conductor Supervision | M1 | 7 | 88 |
| [E09](backlog/epics/E09-OBSERVABILITY.md) | Observability | M1 | 9 | 53, 94 |
| [E10](backlog/epics/E10-FRONTEND-CONTRACT.md) | Frontend / API Contract | M2 | 7 | 66, 105, 59, 76 |
| [E11](backlog/epics/E11-CHAIN-ISFR.md) | Chain / ISFR | Phase 2+ | 5 | 42 |
| [E12](backlog/epics/E12-DEAD-CODE-CLEANUP.md) | Dead-Code & Legacy Cleanup | M2 | 9 | 104, 63, 06 |
| [E13](backlog/epics/E13-SPEC-DEBT-V2.md) | v2 Spec-Debt (long-horizon) | M3+ | 3 | 102, 15, 18, 85-87 |
| [E14](backlog/epics/E14-PROVIDERS-TOOLS.md) | Providers & Tools | M1 | 7 | 38, 99 |
| [E15](backlog/epics/E15-MCP-CONFIG.md) | MCP Config & Passthrough | M1 | 6 | 48 |
| [E16](backlog/epics/E16-PRD-SELF-HOSTING.md) | PRD & Self-Hosting Pipeline | M1 | 2 | 91, 98 |
| [E17](backlog/epics/E17-ACP-COMPLETION.md) | ACP Completion | M2 | 6 | 51, 100 |
| [E18](backlog/epics/E18-DOCS-CONFIG-OPS.md) | Docs, Config, CI & Ops | M1 | 13 | 19, 57, 71, 77, 81, 82, 84 |

**v2 specification epics (E19-E48, 240+ tasks)**:

| Epic | Title | Phase | Tasks |
|---|---|---|---|
| E19 | Signal Protocol | Phase 1 | 10 |
| E20 | Cell Unification | Phase 1 | 10 |
| E21 | Graph Engine | Phase 1 | 10 |
| E22 | Execution Runtime | Phase 1 | 10 |
| E23 | Agent Cognitive Autonomy | Phase 2 | 10 |
| E24 | Memory Advanced | Phase 2 | 10 |
| E25 | Learning Loops Advanced | Phase 2 | 10 |
| E26 | Inference Gateway | Phase 2 | 12 |
| E27 | Feeds System | Phase 2 | 8 |
| E28 | Groups & Coordination | Phase 2 | 8 |
| E29 | Connectivity & Relay | Phase 2 | 9 |
| E30 | Extension System | Phase 2 | 8 |
| E31 | Trigger System | Phase 2 | 8 |
| E32 | Tool & Plugin Ecosystem | Phase 2 | 8 |
| E33 | Telemetry & Lens | Phase 2 | 9 |
| E34 | Security IFC | Phase 2 | 8 |
| E35 | Auth Protocol | Phase 2 | 8 |
| E36 | Payments | Phase 3 | 8 |
| E37 | Surfaces | Phase 2+ | 9 |
| E38 | Marketplace | Phase 3 | 9 |
| E39 | Registries & Identity | Phase 3 | 8 |
| E40 | Arenas & Evals | Phase 3 | 8 |
| E41 | DeFi Products | Phase 3 | 8 |
| E42 | Config Evolution | Phase 2 | 8 |
| E43 | Deployment & Portability | Phase 2+ | 8 |
| E44 | Cross-Cut Functors | Phase 2 | 8 |
| E45 | Orchestrator Mori Parity | Phase 2 | 10 |
| E46 | GitHub Workflow Integration | Phase 1 | -- |
| E47 | Resource & Disk Management | Phase 1 | -- |
| E48 | Rate Limit & Budgeting | Phase 1 | -- |

## 3. Execution Traces

Second-pass docs that trace real runtime code paths hop-by-hop with `file:line`.
Several correct earlier docs (noted inline). These are the deepest evidence layer.

| # | File | Purpose | Epic cross-ref |
|---|---|---|---|
| 96 | [TRACE-RUNNER-V2-EXECUTION](96-TRACE-RUNNER-V2-EXECUTION.md) | Full `plan run --engine runner-v2` trace; no live DAG, flat task-index + per-plan FSM | E01 |
| 97 | [TRACE-SERVE-LIFECYCLE](97-TRACE-SERVE-LIFECYCLE.md) | `roko serve` startup->request->realtime->write; events.jsonl write-only firehose | E02, E09 |
| 98 | [TRACE-SELF-HOSTING-LOOP](98-TRACE-SELF-HOSTING-LOOP.md) | idea->draft->research->plan->execute traced; honest self-host verdict | E01, E16 |
| 99 | [TRACE-AGENT-TURN](99-TRACE-AGENT-TURN.md) | One agent turn; Claude-CLI bypasses per-tool safety entirely | E04, E14 |
| 100 | [TRACE-ACP-SESSION](100-TRACE-ACP-SESSION.md) | ACP session lifecycle; permission gate architecturally unwireable | E17 |
| 101 | [TRACE-GATE-PIPELINE](101-TRACE-GATE-PIPELINE.md) | Gate execution; adaptive thresholds/oracles live only on dead path | E05 |

## 4. Censuses and Inventories

Whole-tree enumerations and generated manifests.

| # | File | Purpose | Epic cross-ref |
|---|---|---|---|
| 03 | [CRATE-AUDIT](03-CRATE-AUDIT.md) | Per-crate status (35 members); LOC/status per crate | E03, E12 |
| 06 | [WIRING-STATUS](06-WIRING-STATUS.md) | Built-but-unwired census; every symbol caller-checked | E12 |
| 11 | [DEPENDENCY-GRAPH](11-DEPENDENCY-GRAPH.md) | Workspace layering and dependency + violation enumeration | E03 |
| 16 | [CODEBASE-INVENTORY](16-CODEBASE-INVENTORY.md) | File/LOC/API/test/artifact inventory | -- |
| 80 | [SOURCE-DOC-MANIFEST](80-SOURCE-DOC-MANIFEST.md) | Generated 636-file docs manifest with status tags and owners | E18 |
| 83 | [ENV-VAR-MANIFEST](83-ENV-VAR-MANIFEST.md) | Generated env-var inventory (99 fixed + dynamic families) | E18, E42 |
| 102 | [SPEC-DEBT-LEDGER](102-SPEC-DEBT-LEDGER.md) | ~129 named v2 concepts -> code status; synthesizes 15/18/85-87 | E13 |
| 103 | [DUPLICATE-TYPES-CENSUS](103-DUPLICATE-TYPES-CENSUS.md) | 19 cross-crate duplicated type families with no conversions | E03 |
| 104 | [DEAD-CODE-AND-FACADE-CENSUS](104-DEAD-CODE-AND-FACADE-CENSUS.md) | Feature facades, orphan files, 71 `allow(dead_code)` sites | E12 |

## 5. Spec Coverage

v1/v2/v2-depth specification-to-code coverage audits. Start at 102 (concept-level
roll-up), then drill into the per-layer detail matrices.

| # | File | Purpose | Epic cross-ref |
|---|---|---|---|
| 02 | [SPEC-EVOLUTION](02-SPEC-EVOLUTION.md) | v1->v2->v2-depth evolution and where code sits | E13 |
| 14 | [V1-COVERAGE](14-V1-COVERAGE.md) | v1 design corpus -> implementation coverage audit | E13 |
| 15 | [V2-COVERAGE](15-V2-COVERAGE.md) | Navigable v2 coverage summary (detail in 85) | E13 |
| 18 | [V2-DEPTH-COVERAGE](18-V2-DEPTH-COVERAGE.md) | v2-depth (185 files) -> code coverage audit | E13 |
| 72 | [SOURCE-DOC-COVERAGE-LEDGER](72-SOURCE-DOC-COVERAGE-LEDGER.md) | How much of v1/v2/v2-depth is converted to status docs (-> 80) | E18 |
| 78 | [V2-DEPTH-RESEARCH-PROMPT-LEDGER](78-V2-DEPTH-RESEARCH-PROMPT-LEDGER.md) | Fences 14 RESEARCH-PROMPT pitch docs from implementation truth | E13 |
| 79 | [REFERENCE-PROVENANCE-LEDGER](79-REFERENCE-PROVENANCE-LEDGER.md) | v1 bibliography provenance and status-use rules | -- |
| 85 | [V2-COVERAGE-KERNEL](85-V2-COVERAGE-KERNEL.md) | Exhaustive kernel-spec (01-07,27) coverage matrix | E13, E19-E22 |
| 86 | [V2-COVERAGE-PLATFORM](86-V2-COVERAGE-PLATFORM.md) | Exhaustive platform-spec (08-19) coverage matrix | E13, E23-E35 |
| 87 | [V2-COVERAGE-ECOSYSTEM](87-V2-COVERAGE-ECOSYSTEM.md) | Exhaustive ecosystem-spec (20-28) coverage matrix | E13, E36-E45 |

## 6. Subsystem Evidence Ledgers -- Kernel and Runtime

Deep per-subsystem audits with `file:line` evidence. Each covers one crate family
or cross-cutting concern.

### Core kernel

| # | File | Subsystem | Epic cross-ref |
|---|---|---|---|
| 30 | [CORE-SIGNAL](30-CORE-SIGNAL.md) | roko-core: noun + 6 verb traits + storage split-brain | E03, E19 |
| 31 | [GRAPH-CELLS-ENGINE](31-GRAPH-CELLS-ENGINE.md) | roko-graph: Graph engine, Cell, stub cognitive cells, dry-run task | E01, E20, E21 |
| 32 | [EVENTS-BUS-STATEHUB](32-EVENTS-BUS-STATEHUB.md) | Event bus, PulseBus, StateHub, relay topics, feed agents | E02, E09, E27, E31 |
| 47 | [FOUNDATION-TYPES-REDESIGN](47-FOUNDATION-TYPES-REDESIGN.md) | DispatchPlan/RunLedger/GateStatus/CommitOutcome consolidation | E03 |
| 89 | [PRIMITIVES-HDC](89-PRIMITIVES-HDC.md) | roko-primitives: HDC core + tier routing; parallel-impl P0 | E19, E25 |

### Orchestration and execution

| # | File | Subsystem | Epic cross-ref |
|---|---|---|---|
| 36 | [ORCHESTRATION-RUNNERS](36-ORCHESTRATION-RUNNERS.md) | Three-generation orchestrate.rs / Runner v2 / Graph audit | E01, E12 |
| 37 | [RUNNER-V2-AND-GRAPH](37-RUNNER-V2-AND-GRAPH.md) | Operator decision doc: which plan engine is live and why | E01 |
| 92 | [RUNNER-V2-MODULE-FAMILY](92-RUNNER-V2-MODULE-FAMILY.md) | Runner v2 per-file ledger (19 files, the live engine) | E01 |
| 88 | [CONDUCTOR](88-CONDUCTOR.md) | Supervision/watchers/circuit-breaker/diagnosis | E08 |
| 90 | [RUNTIME-FS-STD](90-RUNTIME-FS-STD.md) | roko-runtime/-fs/-std: bus/builtins, 6 unwired consumers | E02, E14 |

### Agent and dispatch

| # | File | Subsystem | Epic cross-ref |
|---|---|---|---|
| 33 | [AGENT-SAFETY](33-AGENT-SAFETY.md) | Safety contracts, taint, capabilities, custody | E04, E34 |
| 34 | [COMPOSE-PROMPTS](34-COMPOSE-PROMPTS.md) | Prompt composition, VCG, context assembly | E06, E44 |
| 38 | [AGENT-PROVIDERS-TOOLS](38-AGENT-PROVIDERS-TOOLS.md) | 10 LLM providers, pools, tool loop, provider bugs | E14 |
| 44 | [AGENT-SERVER](44-AGENT-SERVER.md) | Agent sidecar: 13 routes, live-vs-mirage, dead task queue | E14 |

### Gates and verification

| # | File | Subsystem | Epic cross-ref |
|---|---|---|---|
| 35 | [GATES-VERIFICATION](35-GATES-VERIFICATION.md) | 11 gates, 7 rungs, verdict signals, gate-service drift | E05 |

### Knowledge, learning, and affect

| # | File | Subsystem | Epic cross-ref |
|---|---|---|---|
| 39 | [NEURO-KNOWLEDGE](39-NEURO-KNOWLEDGE.md) | Knowledge store, demurrage, HDC, retrieval | E07, E24 |
| 40 | [LEARNING-TELEMETRY](40-LEARNING-TELEMETRY.md) | Episodes/router/experiments; LinUCB weights not persisted | E07, E25 |
| 41 | [DREAMS](41-DREAMS.md) | Dream consolidation across three engines; phase2 shells | E07, E24 |
| 56 | [DAIMON](56-DAIMON.md) | Affect state + dispatch modulation; live on runner-v2, dark on Graph | E44 |

### Integrations

| # | File | Subsystem | Epic cross-ref |
|---|---|---|---|
| 42 | [CHAIN-REGISTRIES-ISFR](42-CHAIN-REGISTRIES-ISFR.md) | Chain, contracts, registries, ISFR, relay, deploy | E11, E39 |
| 48 | [MCP-CRATES](48-MCP-CRATES.md) | MCP code/github/slack/scripts/stdio + config drift | E15, E32 |
| 49 | [INDEX-LANG](49-INDEX-LANG.md) | Code index, language parsers, duplicate HDC struct | E19 |
| 52 | [PLUGIN-EXTENSIONS](52-PLUGIN-EXTENSIONS.md) | Plugin SDK, extension hooks, manifest gaps | E30, E32 |
| 91 | [PRD-RESEARCH](91-PRD-RESEARCH.md) | idea->draft->research->plan pipeline status | E16 |
| 94 | [FEED-AGENTS-FLEET](94-FEED-AGENTS-FLEET.md) | 29 serve-side feed agents driving events.jsonl firehose | E09, E27 |

## 7. Subsystem Evidence Ledgers -- Surfaces and Operations

### CLI and TUI

| # | File | Subsystem | Epic cross-ref |
|---|---|---|---|
| 45 | [CLI-SURFACE](45-CLI-SURFACE.md) | 45 top-level commands, ~155 leaves; resume broken | E01, E18 |
| 62 | [CLI-COMMAND-LEDGER](62-CLI-COMMAND-LEDGER.md) | Command-by-command status ledger (companion to 45) | E18 |
| 43 | [SURFACES-DEMO-UX](43-SURFACES-DEMO-UX.md) | TUI, roko-demo, demo-app, apps overview + UX migration | E10, E37 |
| 93 | [ROKO-DEMO](93-ROKO-DEMO.md) | Compiled chain-scenario orchestrator + own TUI (4th surface) | E37 |

### HTTP API, serve, and frontend

| # | File | Subsystem | Epic cross-ref |
|---|---|---|---|
| 46 | [SERVE-HTTP-REALTIME](46-SERVE-HTTP-REALTIME.md) | Routes, auth, StateHub, SSE/WS, persistence | E04, E10, E35 |
| 59 | [API-ROUTE-LEDGER](59-API-ROUTE-LEDGER.md) | Route-by-route maturity table (companion to 46) | E10, E35 |
| 66 | [FRONTEND-API-PARITY](66-FRONTEND-API-PARITY.md) | React DataHub/API route parity + route-ownership | E10 |
| 105 | [FRONTEND-DEMO-APP](105-FRONTEND-DEMO-APP.md) | React SPA deep-dive; embedded-served, dual SSE managers | E10, E37 |
| 106 | [APPS-MIRAGE-RELAY-WATCHER](106-APPS-MIRAGE-RELAY-WATCHER.md) | 3 `apps/` deep-dive; mirage = schema-of-record for serve routes | E11, E29 |

### ACP and editor integration

| # | File | Subsystem | Epic cross-ref |
|---|---|---|---|
| 51 | [ACP](51-ACP.md) | ACP editor integration; permission gate unwireable as-is | E17 |

### Persistence and state

| # | File | Subsystem | Epic cross-ref |
|---|---|---|---|
| 55 | [DATA-DIR](55-DATA-DIR.md) | `.roko/` filesystem layout (full matrix in 60) | E02 |
| 60 | [STATE-PERSISTENCE-LEDGER](60-STATE-PERSISTENCE-LEDGER.md) | Deep writer->reader matrix; executor.json never-written | E02 |
| 76 | [DATA-CONTRACTS-SCHEMAS](76-DATA-CONTRACTS-SCHEMAS.md) | Runtime/server/frontend/schema ownership and drift | E03, E10 |

### Config and environment

| # | File | Subsystem | Epic cross-ref |
|---|---|---|---|
| 57 | [CONFIG](57-CONFIG.md) | Config schema, secrets, providers, models; preflight gap | E18, E42 |
| 61 | [CONFIG-ENV-MATRIX](61-CONFIG-ENV-MATRIX.md) | Config source/env/secret/provenance matrix (-> 83) | E18, E42 |

### Observability and telemetry

| # | File | Subsystem | Epic cross-ref |
|---|---|---|---|
| 53 | [OBSERVABILITY](53-OBSERVABILITY.md) | Tracing/metrics/traces/logs; Lens=0, firehose events.jsonl | E09, E33 |

### Security and auth

| # | File | Subsystem | Epic cross-ref |
|---|---|---|---|
| 75 | [SECURITY-AUTH-SCOPE-MATRIX](75-SECURITY-AUTH-SCOPE-MATRIX.md) | Trust-boundary map (relay/auth/ACP/MCP/tool-safety) | E04, E34, E35 |

### Testing, CI, and quality

| # | File | Subsystem | Epic cross-ref |
|---|---|---|---|
| 50 | [QUALITY-CI-RELEASE](50-QUALITY-CI-RELEASE.md) | Testing/CI/release/doc-proof policy overview | E18 |
| 64 | [PARITY-TEST-MATRIX](64-PARITY-TEST-MATRIX.md) | Cross-surface parity tests + false-green census | E18 |
| 71 | [CI-RELEASE-PROOF-GAPS](71-CI-RELEASE-PROOF-GAPS.md) | Workflow-by-workflow CI gap table (expands 50) | E18 |
| 73 | [EXAMPLES-PLANS-GRAPHS](73-EXAMPLES-PLANS-GRAPHS.md) | Graph/plan/PRD assets + which fail to load | E01, E18 |
| 74 | [TEST-AND-PROOF-INVENTORY](74-TEST-AND-PROOF-INVENTORY.md) | Test volume vs proof + mocked/false-green census | E18 |

### Jobs, deploy, and operations

| # | File | Subsystem | Epic cross-ref |
|---|---|---|---|
| 58 | [JOBS-DEPLOY](58-JOBS-DEPLOY.md) | Jobs, worker, daemon, deploy shapes + blockers | E11, E43 |
| 77 | [OPERATIONS-DEPLOY-RUNBOOK](77-OPERATIONS-DEPLOY-RUNBOOK.md) | Docker/Railway/Fly/health/ops runbook gaps | E43 |
| 82 | [COMMAND-EXAMPLE-DRIFT-LEDGER](82-COMMAND-EXAMPLE-DRIFT-LEDGER.md) | Stale command snippets + current replacements | E18 |

## 8. Migration, Debt, and Convergence Plans

Documents that track migration state, debt reduction, and doc convergence.
Some are superseded by later docs (noted inline) but kept for history.

| # | File | Purpose | Status | Epic cross-ref |
|---|---|---|---|---|
| 04 | [NAMING-MIGRATION](04-NAMING-MIGRATION.md) | Signal/Engram and other naming state | Current | E03, E19 |
| 05 | [ARCHITECTURE-REALITY](05-ARCHITECTURE-REALITY.md) | Architecture reality notes | **Superseded by 13, 36** | -- |
| 07 | [MIGRATION-CHECKLIST](07-MIGRATION-CHECKLIST.md) | Older migration checklist | **Superseded by 12, 24** | -- |
| 08 | [TECH-DEBT](08-TECH-DEBT.md) | Older tech debt list | **Superseded by 24** | -- |
| 09 | [DEPLOYMENT-STATUS](09-DEPLOYMENT-STATUS.md) | First-pass deploy readiness | **Superseded by 77, 58** | -- |
| 10 | [TESTING-STATUS](10-TESTING-STATUS.md) | Test coverage summary | **Superseded by 16, 74** | -- |
| 19 | [DOC-DRIFT-REGISTER](19-DOC-DRIFT-REGISTER.md) | Claims in docs that no longer match code | Current | E18 |
| 54 | [PER-CRATE-MIGRATION-CHECKLIST](54-PER-CRATE-MIGRATION-CHECKLIST.md) | Crate keep/merge/quarantine guidance | Current | E03, E12 |
| 63 | [DELETE-ARCHIVE-PLAN](63-DELETE-ARCHIVE-PLAN.md) | Removal/archive candidates + required proofs | Current | E12 |
| 65 | [DOCS-CONVERGENCE-PLAN](65-DOCS-CONVERGENCE-PLAN.md) | How to converge root/v1/v2/v2-depth/tmp/code | Current | E18 |
| 70 | [RELAY-PROTOCOL-FREEZE](70-RELAY-PROTOCOL-FREEZE.md) | Relay/API response-shape freeze checklist | Current | E29 |
| 81 | [ROOT-DOCS-REWRITE-QUEUE](81-ROOT-DOCS-REWRITE-QUEUE.md) | README/CLAUDE/v2/demo docs rewrite queue | Current | E18 |
| 84 | [STATUS-PACK-MAINTENANCE](84-STATUS-PACK-MAINTENANCE.md) | How to regenerate/validate the pack | Current | -- |

## 9. Tmp Archaeology

Reconciliation of the `tmp/` design material. Start at 17 for the meta-ranking,
then 20 for newest material with still-open dogfood evidence.

| # | File | Purpose | Epic cross-ref |
|---|---|---|---|
| 17 | [TMP-SOURCE-RANKING](17-TMP-SOURCE-RANKING.md) | Authoritative/current/stale/scratch tiering of tmp folders | -- |
| 20 | [TMP-NEWEST](20-TMP-NEWEST.md) | Newest tmp inventory (May 8-Jun 23); still-open P0 dogfood evidence | -- |
| 21 | [TMP-MAY-BATCH](21-TMP-MAY-BATCH.md) | May batch reconciliation | -- |
| 22 | [TMP-LEGACY](22-TMP-LEGACY.md) | Older tmp folder archaeology | -- |
| 23 | [TASKRUNNER-MIGRATION-STATUS](23-TASKRUNNER-MIGRATION-STATUS.md) | Runner lineage and v2 migration | E01 |
| 67 | [TMP-FEEDBACK-2-CROSSWALK](67-TMP-FEEDBACK-2-CROSSWALK.md) | May-8 35-issue defect map -> current disposition | -- |
| 68 | [SELF-DEVELOPING-CROSSWALK](68-SELF-DEVELOPING-CROSSWALK.md) | Self-developing architecture -> implementation status | -- |
| 69 | [RESIDUAL-AUDIT-TRACKER](69-RESIDUAL-AUDIT-TRACKER.md) | Remaining audit work and residual unknowns | -- |

## 10. Pack Meta

| File | Purpose |
|---|---|
| [DOC-MANIFEST.md](DOC-MANIFEST.md) | Full doc taxonomy (category/subsystem/purpose/size) + consolidation recommendations |

---

## Source Priority

When sources disagree, use this priority order:

1. Current code at `5852c93c05`.
2. `.roko/` live state and generated artifacts, when the path is in this workspace.
3. This status-quo pack, especially `01`, `12`, `13`, `24`, `25`, and `26`.
4. Newest tmp design material, especially `tmp/tmp-feedback/2`, `tmp/relay-bus`, `tmp/subsystem-audits/05-01`, `tmp/solutions/self-developing`, and `tmp/doc-convergence`.
5. `docs/v2-depth`, then `docs/v2`, then `docs/v1`.
6. Root narrative files such as `README.md` and `CLAUDE.md`, which are useful but currently stale in several high-impact areas.

## Method

- Codebase inspected with `rg`, `cargo metadata`, `cargo check`, route/test/file counts, and targeted file reads.
- Parallel explorer passes audited orchestration/runtime, learning/telemetry, neuro/dreams/daimon, server/realtime, frontend/TUI/demo, chain/contracts/ISFR, route/API, CLI, state/config, CI/release, crate boundaries, and tmp/docs archaeology.
- Follow-up explorer passes audited source-doc coverage, examples/plans/graphs, tests/proof, security/auth/scope, data contracts/schemas, operations/deploy, root docs, env/config, command examples, and source-manifest gaps.
- The 2026-07-08 re-audit ran ~37 parallel subagents across three waves. Their findings were reconciled into this navigation layer, and four new docs (92-95) were created.
- The 2026-07-09/10 backlog expansion materialized 48 epics (E01-E48) with executable task files, 6 DOC plans, source-corpus coverage ledgers, and gap reports.
- Every one of the 31 `crates/` packages now has coverage in this pack.
