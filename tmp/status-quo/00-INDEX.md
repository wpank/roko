# Status Quo Index

Generated 2026-07-07; re-audited and expanded 2026-07-08 for the current `main` branch at `5852c93c05`.

This folder is the current-state pack for Roko: what exists, what is live, what is partial, what is stale, and what to do next. Treat this index plus `01`, `12`, `13`, `24`, `25`, `26`, `27`, `28`, `29`, and `95` as the navigation layer. The longer subsystem files are evidence ledgers.

**Read `95-ENGINE-DRIFT.md` before trusting any "roko plan run works" claim.** The single most load-bearing cross-cutting fact in this codebase: `roko plan run` defaults to the Graph Engine, which is a **dry-run stub** (prints SUCCESS, spawns no agent, $0, no code change). Real execution is Runner v2 (`--engine runner-v2`, and the implicit engine for `do`/`serve`/`prd`/`worker`). CLAUDE.md's orchestrate.rs-centric component table is stale — that module is dead-by-default behind `legacy-orchestrate`.

## Start Here

| File | Purpose |
|---|---|
| [01-EXECUTIVE-SUMMARY.md](01-EXECUTIVE-SUMMARY.md) | The current truth in one pass. |
| [12-ROADMAP.md](12-ROADMAP.md) | Ordered implementation roadmap with proof gates. |
| [13-CURRENT-STATE-MATRIX.md](13-CURRENT-STATE-MATRIX.md) | System-by-system shipped/partial/stub matrix. |
| [24-OPEN-ISSUE-LEDGER.md](24-OPEN-ISSUE-LEDGER.md) | P0/P1/P2 debt register. |
| [25-PROOF-GATES.md](25-PROOF-GATES.md) | Commands and acceptance criteria that prove migration work is done. |
| [26-CANONICAL-DECISIONS.md](26-CANONICAL-DECISIONS.md) | Architecture decisions that should stop name/path drift. |
| [27-IMPLEMENTATION-BACKLOG.md](27-IMPLEMENTATION-BACKLOG.md) | Actionable backlog sliced by risk and dependency order. |
| [28-DEFINITION-OF-DONE.md](28-DEFINITION-OF-DONE.md) | Done criteria for migration work, proofs, docs, and deletion. |
| [29-RISK-REGISTER.md](29-RISK-REGISTER.md) | Operational risk register with mitigations and triggers. |

## Execution Backlog (2026-07-09 third pass)

The [`backlog/`](backlog/00-INDEX.md) subfolder turns this pack's findings into a roko-**executable** work plan: 149 granular tasks in roko's native `tasks.toml` schema (tier, `depends_on`, `[task.context]`, `[[task.verify]]`, acceptance), grouped into 18 epics (E01–E18) and 4 milestones (M0 bootstrap → M3+), reconciled against the ~180 tasks already in `plans/`. Start with [`backlog/00-INDEX.md`](backlog/00-INDEX.md).

| File | Purpose |
|---|---|
| [backlog/00-INDEX.md](backlog/00-INDEX.md) | Backlog navigation + epics table + how-to-execute. |
| [backlog/03-WORK-BREAKDOWN-EPICS.md](backlog/03-WORK-BREAKDOWN-EPICS.md) | Master roadmap: epic dependency DAG, milestones, critical path, parallel tracks. |
| [backlog/05-MASTER-CHECKLIST.md](backlog/05-MASTER-CHECKLIST.md) | The flat, tickable checklist of all 149 tasks by milestone. |
| [backlog/04-EXECUTION-READINESS.md](backlog/04-EXECUTION-READINESS.md) | **M0 bootstrap** — what must be true before roko can self-execute the backlog (the single unblocking fix). |
| [backlog/01-TASK-EXECUTION-SCHEMA.md](backlog/01-TASK-EXECUTION-SCHEMA.md) | Canonical `tasks.toml` schema — how a finding becomes an executable task with gates/acceptance. |
| [backlog/02-PLANS-RECONCILIATION.md](backlog/02-PLANS-RECONCILIATION.md) | Maps existing `plans/` (P08–P34) to findings; which are current/stale/gaps. |
| [backlog/epics/](backlog/epics/) | E01–E18 epic files, each with granular tasks (tier/deps/acceptance/verify). |
| [backlog/exemplars/](backlog/exemplars/) | 3 drop-in `plan validate`-clean `tasks.toml` proving the format. |
| [backlog/references/PLANNING-METHODOLOGY.md](backlog/references/PLANNING-METHODOLOGY.md) | Cited best-practice for sizing/gating agent-executable work. |
| [DOC-MANIFEST.md](DOC-MANIFEST.md) | Full doc taxonomy (category/subsystem/purpose) + consolidation recommendations. |

## Deep Traces And Census (2026-07-08 second pass)

Second-pass docs that trace real runtime code paths hop-by-hop with `file:line`, and consolidate cross-cutting evidence. These are the deepest layer of the pack; several *correct* earlier docs (noted inline in each).

| File | Purpose |
|---|---|
| [96-TRACE-RUNNER-V2-EXECUTION.md](96-TRACE-RUNNER-V2-EXECUTION.md) | Full `plan run --engine runner-v2` trace. **Key correction:** Runner v2 has no live DAG — it's a flat task-index + per-plan FSM capped at one agent per plan (`max_concurrent_plans=4`); `task_dag.rs` is dead. |
| [97-TRACE-SERVE-LIFECYCLE.md](97-TRACE-SERVE-LIFECYCLE.md) | `roko serve` startup → request → realtime → write path. **Key finding:** `events.jsonl` (44 MB) is a write-only firehose nothing reads (FeedTick is a no-op in snapshot apply). |
| [98-TRACE-SELF-HOSTING-LOOP.md](98-TRACE-SELF-HOSTING-LOOP.md) | idea→draft→research→plan→execute traced; the honest "can Roko self-host today?" verdict + the exact command sequence that works. |
| [99-TRACE-AGENT-TURN.md](99-TRACE-AGENT-TURN.md) | One agent turn. **Key finding:** roko's safety/tool funnel runs only on the OpenAI-compat ToolLoop; the default Claude-CLI provider bypasses per-tool safety entirely. Only 16 of 37 builtin tools have executable handlers. |
| [100-TRACE-ACP-SESSION.md](100-TRACE-ACP-SESSION.md) | ACP session lifecycle; the permission gate is architecturally *unwireable* as-is (needs a reply channel), not merely uncalled. |
| [101-TRACE-GATE-PIPELINE.md](101-TRACE-GATE-PIPELINE.md) | Gate execution. **Key correction:** the live runner path never calls `enrich_rung_config`; adaptive thresholds/oracles/replan live only on the dead PlanRunner, so rungs 3-6 stub-pass and EMA only updates rung 2. |
| [102-SPEC-DEBT-LEDGER.md](102-SPEC-DEBT-LEDGER.md) | Concept-level ledger of ~129 named v2/v2-depth concepts → code status (~55 zero-code, ~52 partial, ~24 built). Lens is the top load-bearing zero-code concept. |
| [103-DUPLICATE-TYPES-CENSUS.md](103-DUPLICATE-TYPES-CENSUS.md) | 19 cross-crate duplicated type families with no conversions; `DashboardSnapshot ×3` is the highest blast-radius (root of dashboard emptiness). |
| [104-DEAD-CODE-AND-FACADE-CENSUS.md](104-DEAD-CODE-AND-FACADE-CENSUS.md) | Feature façades, orphan files, 71 `allow(dead_code)` sites; `legacy-runner-v2` is a default-on feature that gates only tests. |
| [105-FRONTEND-DEMO-APP.md](105-FRONTEND-DEMO-APP.md) | `demo/demo-app` React deep-dive. **Correction:** the SPA IS embedded-served by `roko serve` (rust-embed fallback), not standalone; two parallel SSE managers with divergent casing. |
| [106-APPS-MIRAGE-RELAY-WATCHER.md](106-APPS-MIRAGE-RELAY-WATCHER.md) | The three `apps/`. Mirage is an anvil-in-Rust fork simulator that is the schema-of-record behind roko-serve's "mirage-shape" routes; none of the 3 apps are in `default-members`. |

Companion deepened census docs live in their home sections: `06` (built-but-unwired), `08` (tech debt), `74`/`64` (false-green tests), `55`/`60` (persistence writer/reader map), `16`/`11` (inventory + layering violations), `03`/`54` (per-crate).

## Current State And Migration

| File | Purpose |
|---|---|
| [02-SPEC-EVOLUTION.md](02-SPEC-EVOLUTION.md) | v1 to v2 to v2-depth evolution, with correction note. |
| [03-CRATE-AUDIT.md](03-CRATE-AUDIT.md) | Per-crate status from the first audit pass. |
| [04-NAMING-MIGRATION.md](04-NAMING-MIGRATION.md) | Signal/Engram and other naming state. |
| [05-ARCHITECTURE-REALITY.md](05-ARCHITECTURE-REALITY.md) | Architecture reality notes; use with current corrections in `13` and `24`. |
| [06-WIRING-STATUS.md](06-WIRING-STATUS.md) | Built-but-not-wired inventory. |
| [07-MIGRATION-CHECKLIST.md](07-MIGRATION-CHECKLIST.md) | Older migration checklist; superseded for priority by `12` and `24`. |
| [08-TECH-DEBT.md](08-TECH-DEBT.md) | Older tech debt list; superseded for priority by `24`. |
| [11-DEPENDENCY-GRAPH.md](11-DEPENDENCY-GRAPH.md) | Workspace layering and dependency notes. |
| [15-V2-COVERAGE.md](15-V2-COVERAGE.md) | v2 migration coverage across Graph, Cell, Bus, Store, Lens, Group, and Extension concepts. |
| [16-CODEBASE-INVENTORY.md](16-CODEBASE-INVENTORY.md) | Package, file, route, doc, and test inventory. |
| [19-DOC-DRIFT-REGISTER.md](19-DOC-DRIFT-REGISTER.md) | Claims in docs/tmp/CLAUDE that no longer match code. |
| [47-FOUNDATION-TYPES-REDESIGN.md](47-FOUNDATION-TYPES-REDESIGN.md) | DispatchPlan, RunLedger, GateStatus, CommitOutcome, RoutingContext consolidation status. |
| [54-PER-CRATE-MIGRATION-CHECKLIST.md](54-PER-CRATE-MIGRATION-CHECKLIST.md) | Crate ownership, boundary problems, and keep/merge/quarantine guidance. |
| [60-STATE-PERSISTENCE-LEDGER.md](60-STATE-PERSISTENCE-LEDGER.md) | Canonical `.roko` state/path map and migration checklist. |
| [61-CONFIG-ENV-MATRIX.md](61-CONFIG-ENV-MATRIX.md) | Config source, env var, secret, and provenance matrix. |
| [63-DELETE-ARCHIVE-PLAN.md](63-DELETE-ARCHIVE-PLAN.md) | Removal/archive candidates and required proofs. |
| [65-DOCS-CONVERGENCE-PLAN.md](65-DOCS-CONVERGENCE-PLAN.md) | How to converge root docs, v1/v2/v2-depth, tmp designs, and code. |
| [72-SOURCE-DOC-COVERAGE-LEDGER.md](72-SOURCE-DOC-COVERAGE-LEDGER.md) | Source-doc manifest gaps across docs/v1, docs/v2, docs/v2-depth, research prompts, and references. |
| [80-SOURCE-DOC-MANIFEST.md](80-SOURCE-DOC-MANIFEST.md) | Generated 636-file source-doc manifest with status tags and suggested owners. |
| [84-STATUS-PACK-MAINTENANCE.md](84-STATUS-PACK-MAINTENANCE.md) | Regeneration and validation commands for the status pack. |

## Spec Coverage

| File | Purpose |
|---|---|
| [14-V1-COVERAGE.md](14-V1-COVERAGE.md) | v1 coverage audit; has current correction note. |
| [18-V2-DEPTH-COVERAGE.md](18-V2-DEPTH-COVERAGE.md) | v2-depth coverage audit. |
| [78-V2-DEPTH-RESEARCH-PROMPT-LEDGER.md](78-V2-DEPTH-RESEARCH-PROMPT-LEDGER.md) | Strategy/research prompt ledger; fences pitch/demo claims from implementation truth. |
| [79-REFERENCE-PROVENANCE-LEDGER.md](79-REFERENCE-PROVENANCE-LEDGER.md) | v1 reference bibliography provenance and status-use rules. |
| [30-CORE-SIGNAL.md](30-CORE-SIGNAL.md) | Core noun and trait migration. |
| [31-GRAPH-CELLS-ENGINE.md](31-GRAPH-CELLS-ENGINE.md) | Graph Engine and Cell audit. |
| [32-EVENTS-BUS-STATEHUB.md](32-EVENTS-BUS-STATEHUB.md) | Event buses, PulseBus, StateHub, and projection split. |
| [36-ORCHESTRATION-RUNNERS.md](36-ORCHESTRATION-RUNNERS.md) | Runner v2, Graph Engine, legacy orchestrate.rs handoff. |
| [37-RUNNER-V2-AND-GRAPH.md](37-RUNNER-V2-AND-GRAPH.md) | Short operational guide for the two plan engines. |
| [92-RUNNER-V2-MODULE-FAMILY.md](92-RUNNER-V2-MODULE-FAMILY.md) | Runner v2 module-family evidence ledger (`runner/`, 19 files, the live engine). |
| [95-ENGINE-DRIFT.md](95-ENGINE-DRIFT.md) | **Cross-cutting**: three plan engines, the hollow default, projection seam — navigation reference. |

## Runtime Subsystems

| File | Purpose |
|---|---|
| [33-AGENT-SAFETY.md](33-AGENT-SAFETY.md) | Safety contracts, taint, capabilities, custody, audit. |
| [34-COMPOSE-PROMPTS.md](34-COMPOSE-PROMPTS.md) | Prompt composition, VCG, context, prompt assembly services. |
| [35-GATES-VERIFICATION.md](35-GATES-VERIFICATION.md) | Gate pipeline, rungs, verdict signals, gate service drift. |
| [38-AGENT-PROVIDERS-TOOLS.md](38-AGENT-PROVIDERS-TOOLS.md) | Provider and tool execution surface. |
| [39-NEURO-KNOWLEDGE.md](39-NEURO-KNOWLEDGE.md) | Knowledge store, demurrage, HDC, retrieval. |
| [40-LEARNING-TELEMETRY.md](40-LEARNING-TELEMETRY.md) | Learning, feedback, episode, router, and telemetry reality. |
| [41-DREAMS.md](41-DREAMS.md) | Dream consolidation and replay. |
| [42-CHAIN-REGISTRIES-ISFR.md](42-CHAIN-REGISTRIES-ISFR.md) | Chain, contracts, registries, ISFR, relay, deploy. |
| [44-AGENT-SERVER.md](44-AGENT-SERVER.md) | Agent sidecar server and aggregation. |
| [48-MCP-CRATES.md](48-MCP-CRATES.md) | MCP crates and config drift. |
| [49-INDEX-LANG.md](49-INDEX-LANG.md) | Code index and language parsers. |
| [52-PLUGIN-EXTENSIONS.md](52-PLUGIN-EXTENSIONS.md) | Plugin SDK, extension hooks, and manifest gaps. |
| [56-DAIMON.md](56-DAIMON.md) | Daimon affect state, dispatch modulation, duplicate persistence paths. |

## Surfaces And Operations

| File | Purpose |
|---|---|
| [09-DEPLOYMENT-STATUS.md](09-DEPLOYMENT-STATUS.md) | Deployment status from the first pass. |
| [10-TESTING-STATUS.md](10-TESTING-STATUS.md) | Test coverage summary; use current inventory in `16` for counts. |
| [43-SURFACES-DEMO-UX.md](43-SURFACES-DEMO-UX.md) | React demo, TUI, static demos, UX migration. |
| [93-ROKO-DEMO.md](93-ROKO-DEMO.md) | `roko-demo` crate: compiled chain-scenario orchestrator, own ratatui TUI + WS:9090 (fourth TUI-class surface). |
| [94-FEED-AGENTS-FLEET.md](94-FEED-AGENTS-FLEET.md) | 29 serve-side feed agents producing the `.roko/events.jsonl` firehose. |
| [45-CLI-SURFACE.md](45-CLI-SURFACE.md) | CLI command surface and TUI notes. |
| [46-SERVE-HTTP-REALTIME.md](46-SERVE-HTTP-REALTIME.md) | HTTP API, auth, StateHub, SSE/WS, persistence. |
| [50-QUALITY-CI-RELEASE.md](50-QUALITY-CI-RELEASE.md) | Testing, CI, release, doc-proof policy. |
| [51-ACP.md](51-ACP.md) | ACP/editor integration. |
| [53-OBSERVABILITY.md](53-OBSERVABILITY.md) | Metrics, events, traces, logs, and gaps. |
| [55-DATA-DIR.md](55-DATA-DIR.md) | `.roko/` filesystem state and artifact paths. |
| [57-CONFIG.md](57-CONFIG.md) | Config state and drift. |
| [58-JOBS-DEPLOY.md](58-JOBS-DEPLOY.md) | Jobs, worker, deploy, marketplace status. |
| [59-API-ROUTE-LEDGER.md](59-API-ROUTE-LEDGER.md) | Route counts, namespaces, frontend mismatches, and route maturity. |
| [62-CLI-COMMAND-LEDGER.md](62-CLI-COMMAND-LEDGER.md) | CLI command status, stale docs, and proof checklist. |
| [64-PARITY-TEST-MATRIX.md](64-PARITY-TEST-MATRIX.md) | Cross-surface parity tests needed before migration completion. |
| [66-FRONTEND-API-PARITY.md](66-FRONTEND-API-PARITY.md) | React DataHub/API route parity and route-ownership checklist. |
| [71-CI-RELEASE-PROOF-GAPS.md](71-CI-RELEASE-PROOF-GAPS.md) | CI/release workflow gaps, missing gates, and command matrix. |
| [73-EXAMPLES-PLANS-GRAPHS.md](73-EXAMPLES-PLANS-GRAPHS.md) | Graph examples, executable plans, PRD plans, and demo-resource proof status. |
| [74-TEST-AND-PROOF-INVENTORY.md](74-TEST-AND-PROOF-INVENTORY.md) | Test/CI/proof surface inventory with explicit non-default gates. |
| [75-SECURITY-AUTH-SCOPE-MATRIX.md](75-SECURITY-AUTH-SCOPE-MATRIX.md) | Route auth, terminal, ACP, MCP, workspace, and tool-safety trust boundaries. |
| [76-DATA-CONTRACTS-SCHEMAS.md](76-DATA-CONTRACTS-SCHEMAS.md) | Runtime/server/frontend/schema ownership and drift ledger. |
| [77-OPERATIONS-DEPLOY-RUNBOOK.md](77-OPERATIONS-DEPLOY-RUNBOOK.md) | Docker, Railway, Fly, release, health, and ops runbook gaps. |
| [81-ROOT-DOCS-REWRITE-QUEUE.md](81-ROOT-DOCS-REWRITE-QUEUE.md) | Maintained README/CLAUDE/v2/demo/deploy docs rewrite queue. |
| [82-COMMAND-EXAMPLE-DRIFT-LEDGER.md](82-COMMAND-EXAMPLE-DRIFT-LEDGER.md) | Stale command snippets and current replacement rules. |
| [83-ENV-VAR-MANIFEST.md](83-ENV-VAR-MANIFEST.md) | Generated direct env-var manifest with categories and code owners. |

## Tmp Archaeology

| File | Purpose |
|---|---|
| [17-TMP-SOURCE-RANKING.md](17-TMP-SOURCE-RANKING.md) | Which tmp folders are authoritative, current, stale, or scratch. |
| [20-TMP-NEWEST.md](20-TMP-NEWEST.md) | Detailed newest tmp inventory, with correction note. |
| [21-TMP-MAY-BATCH.md](21-TMP-MAY-BATCH.md) | May batch reconciliation. |
| [22-TMP-LEGACY.md](22-TMP-LEGACY.md) | Older tmp folder archaeology. |
| [23-TASKRUNNER-MIGRATION-STATUS.md](23-TASKRUNNER-MIGRATION-STATUS.md) | Taskrunner and unified-migration-runner status. |
| [67-TMP-FEEDBACK-2-CROSSWALK.md](67-TMP-FEEDBACK-2-CROSSWALK.md) | May 8 dogfooding feedback mapped to current code/docs. |
| [68-SELF-DEVELOPING-CROSSWALK.md](68-SELF-DEVELOPING-CROSSWALK.md) | Self-developing architecture tmp designs mapped to implementation status. |
| [69-RESIDUAL-AUDIT-TRACKER.md](69-RESIDUAL-AUDIT-TRACKER.md) | Remaining audit work and residual unknowns. |
| [70-RELAY-PROTOCOL-FREEZE.md](70-RELAY-PROTOCOL-FREEZE.md) | Relay/API response-shape freeze checklist. |

## Source Priority

When sources disagree, use this priority order:

1. Current code at `5852c93c05`.
2. `.roko/` live state and generated artifacts, when the path is in this workspace.
3. This status-quo pack, especially `01`, `12`, `13`, `24`, `25`, and `26`.
4. Newest tmp design material, especially `tmp/tmp-feedback/2`, `tmp/relay-bus`, `tmp/subsystem-audits/05-01`, `tmp/solutions/self-developing`, and `tmp/doc-convergence`.
5. `docs/v2-depth`, then `docs/v2`, then `docs/v1`.
6. Root narrative files such as `README.md` and `CLAUDE.md`, which are useful but currently stale in several high-impact areas.

## Method

- Codebase inspected with `rg`, `cargo metadata`, `cargo check -q -p roko-learn -p roko-cli`, route/test/file counts, and targeted file reads.
- Parallel explorer passes audited orchestration/runtime, learning/telemetry, neuro/dreams/daimon, server/realtime, frontend/TUI/demo, chain/contracts/ISFR, route/API, CLI, state/config, CI/release, crate boundaries, and tmp/docs archaeology.
- Follow-up explorer passes audited source-doc coverage, examples/plans/graphs, tests/proof, security/auth/scope, data contracts/schemas, operations/deploy, root docs, env/config, command examples, and source-manifest gaps.
- The 2026-07-08 re-audit ran a fleet of ~37 parallel subagents across three waves (kernel/core, runtime subsystems, surfaces, spec-coverage, tmp archaeology, and a coverage-gap sweep). Each agent re-verified its owned docs against current code at `5852c93c05` with `rg`/file reads before editing, added `file:line` evidence, and reported cross-cutting drift. Their findings were reconciled into this navigation layer, and four new docs (`92`–`95`) were created for live subsystems the original pass had not documented (Runner v2 module family, `roko-demo`, feed-agents fleet, and the engine-drift cross-cut).
- Every one of the 31 `crates/` packages now has coverage in this pack; the only known residual is `roko-cli/scripts/swebench_*.py` (SWE-bench harness, testing-infra only).
