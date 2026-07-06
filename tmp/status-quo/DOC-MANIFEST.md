# DOC-MANIFEST — status-quo pack taxonomy & consolidation map

> Generated 2026-07-09 @ HEAD `5852c93c05` on `main`. Enumerates all 107 numbered docs
> (`00`–`106`) plus the new `backlog/` subtree, assigns each a **category**, subsystem,
> size, and one-line purpose, then recommends consolidations and a clean `00-INDEX`
> taxonomy. This file does **not** edit `00-INDEX.md` and does **not** move/delete any
> file — it is a recommendation + navigation aid only.

**Category legend** (7 buckets):
`Navigation` = decision/roadmap/summary layer · `Evidence-Ledger` = subsystem deep audit
or operational ledger with `file:line` evidence · `Trace` = hop-by-hop runtime execution
trace · `Census` = whole-tree enumeration/inventory/manifest · `Spec-Coverage` = v1/v2/v2-depth
spec→code coverage · `Tmp-Archaeology` = `tmp/` source reconciliation · `Backlog-Execution` =
work-breakdown/epics/exemplars for building the fixes.

**Totals by category:** Navigation 11 · Evidence-Ledger 62 · Census 9 · Spec-Coverage 10 ·
Trace 6 · Tmp-Archaeology 9 · Backlog-Execution 0 files (2 scaffolded dirs). **= 107 numbered docs.**

---

## Navigation (11)

| # | Title | Subsystem | Size | Purpose |
|---|---|---|---|---|
| 00 | Status Quo Index | pack meta | 16K | Master index / reading order for the whole pack (reconciled separately). |
| 01 | Executive Summary | whole system | 10K | Current truth in one pass; leads with engine-drift + security. |
| 12 | Roadmap | whole system | 14K | Risk-ordered implementation roadmap, each item paired to a proof gate. |
| 13 | Current State Matrix | whole system | 12K | System-by-system Live/Partial/Stub/Legacy/Stale-doc matrix. |
| 24 | Open Issue Ledger | whole system | 21K | P0/P1/P2 debt register with evidence + done-when criteria. |
| 25 | Proof Gates | whole system | 12K | Runnable command + pass-criteria checks that prove a fix is done. |
| 26 | Canonical Decisions | architecture | 11K | Decisions to ratify (engine, noun, persistence) to stop drift. |
| 27 | Implementation Backlog | whole system | 10K | Risk-sliced actionable backlog; every item closes with a proof gate. |
| 28 | Definition Of Done | process | 6K | Strict "fully migrated" criteria (default path + durable state + test + docs). |
| 29 | Risk Register | whole system | 9K | Risks that create false confidence/loss/exposure, with mitigations. |
| 95 | Engine Drift | plan engines | 12K | **Cross-cut nav reference**: 3 plan engines, hollow default, projection seam. |

## Evidence-Ledger (62)

| # | Title | Subsystem | Size | Purpose |
|---|---|---|---|---|
| 04 | Naming & Convention Migration | core naming | 31K | Signal/Engram rename state; grep census of the noun-flip blast radius. |
| 05 | Architecture: Spec vs Reality | architecture | 11K | Spec-vs-running overview (stale orchestrate.rs framing; see corrections). |
| 07 | V1→V2 Migration Checklist | migration | 7K | Older phase-by-phase migration checklist. |
| 08 | Tech Debt Inventory | debt | 7K | Older TODO/stub/structural-debt list. |
| 09 | Deployment Status | deploy | 4K | First-pass deploy readiness; confirmed clean-checkout blockers. |
| 10 | Testing Status | tests | 18K | Test-attribute census (~9,968) + P0 stub-run caveat. |
| 19 | Doc Drift Register | docs | 10K | Maintained-doc claims (CLAUDE/README/v2) that mislead vs code. |
| 30 | roko-core — Signal & Kernel Traits | roko-core | 30K | Core noun + 6 verb traits + storage split-brain audit. |
| 31 | roko-graph — Cells, Graphs, Engine | roko-graph | 28K | Graph engine/Cell audit; stub cognitive cells + dry-run task cell. |
| 32 | Events, Bus, Relay, Feeds, StateHub | events/bus | 33K | Event bus, StateHub, relay topics, feed agents, orphan pulse_bus. |
| 33 | Safety Layer | roko-agent/safety | 20K | Contracts, capabilities, taint, dual-LLM; fail-closed fallback (CLAUDE.md stale). |
| 34 | roko-compose — Prompt Assembly | roko-compose | 34K | Prompt layers/VCG; Runner-v2 bypasses canonical Compose stack. |
| 35 | roko-gate — Gates, Rungs, Pipeline | roko-gate | 28K | 11 gates, 7 rungs, verdict signals, gate-service dual-dialect drift. |
| 36 | Orchestration — Runners handoff | orchestration | 39K | Three-generation orchestrate.rs / Runner v2 / Graph audit. |
| 37 | Runner V2 And Graph | plan engines | 7K | Operator decision doc: which plan engine is live and why. |
| 38 | roko-agent — Backends, Dispatch, Tools | roko-agent | 35K | 10 providers, pools, tool loop; live tmp-feedback provider bugs. |
| 39 | roko-neuro — Knowledge Store | roko-neuro | 27K | Durable store, admission/tiers/decay, doubly-inert balance ledger. |
| 40 | roko-learn — Learning Loops | roko-learn | 38K | Episodes/router/experiments; LinUCB weights not persisted (P1). |
| 41 | roko-dreams — Consolidation | roko-dreams | 23K | Dream cycle across three engines; phase2 shells. |
| 42 | roko-chain — Witness/Registries/ISFR | roko-chain | 44K | Contracts, ISFR, x402; most modules have 0 external callers. |
| 43 | Surfaces — TUI/Demo/Chat/Web | surfaces | 43K | TUI, roko-demo, demo-app, apps overview + UX migration. |
| 44 | roko-agent-server — Sidecar | agent-server | 35K | 13 routes; live-vs-mirage tally, dead task queue, blob /stream. |
| 45 | roko-cli — Command Surface | roko-cli | 24K | 45 top-level cmds, ~155 leaves; resume broken, engine-default stub. |
| 46 | roko-serve — HTTP Control Plane | roko-serve | 29K | Routes, auth, StateHub, SSE/WS, persistence deep audit. |
| 47 | Foundation Types Redesign | core types | 10K | DispatchPlan/RunLedger/GateStatus/CommitOutcome consolidation status. |
| 48 | MCP Crates | roko-mcp-* | 32K | code/github/slack/scripts/stdio MCP crates + config passthrough. |
| 49 | roko-index + lang crates | code-intel | 29K | Index/graph/HDC + lang parsers; duplicate incompatible HDC struct. |
| 50 | Quality, CI, Release | CI/release | 7K | CI/release/doc-proof policy overview. |
| 51 | roko-acp — Agent Client Protocol | roko-acp | 21K | ACP editor integration; permission gate unwireable as-is. |
| 52 | roko-plugin — Extension System | roko-plugin | 37K | Plugin SDK/manifest/extension hooks + wiring gaps. |
| 53 | Observability | telemetry | 30K | Tracing/metrics/traces/logs; Lens=0, events.jsonl firehose. |
| 54 | Per-Crate Migration Checklist | migration | 17K | Zero-debt roadmap per package; companion to 03. |
| 55 | .roko/ Runtime State Layout | data dir | 28K | `.roko/` artifact layout (full writer/reader matrix now in 60). |
| 56 | roko-daimon — Affect Engine | roko-daimon | 31K | Affect state + dispatch modulation; live on runner-v2, dark on Graph. |
| 57 | Configuration — roko.toml/Secrets | config | 31K | Config schema, secrets, providers, models; preflight gap. |
| 58 | Jobs Marketplace + Deployment | jobs/deploy | 22K | Jobs, worker, daemon, deploy shapes + blockers. |
| 59 | API Route Ledger | roko-serve | 19K | Route-by-route mounted/data-source/maturity table (companion to 46). |
| 60 | State And Persistence Ledger | persistence | 22K | Deep writer→reader matrix; executor.json never-written-but-read. |
| 61 | Config And Environment Matrix | config/env | 7K | Config-source/env/secret/provenance matrix (points to 83). |
| 62 | CLI Command Ledger | roko-cli | 16K | Condensed command-by-command status ledger (companion to 45). |
| 63 | Delete And Archive Plan | cleanup | 4K | Removal/archive/quarantine candidates gated on proofs. |
| 64 | Parity Test Matrix | tests | 10K | Cross-surface (CLI/HTTP/ACP) parity tests to add + false-green census. |
| 65 | Docs Convergence Plan | docs | 8K | How to converge root/v1/v2/v2-depth/tmp/code without losing history. |
| 66 | Frontend/API Parity | demo-app | 11K | React DataHub/API route parity + route-ownership checklist. |
| 71 | CI And Release Proof Gaps | CI/release | 6K | Workflow-by-workflow CI gap table (expands 50). |
| 73 | Examples, Plans, And Graphs | assets | 10K | Runnable-looking graph/plan/PRD assets + which fail to load. |
| 74 | Test And Proof Inventory | tests | 15K | Test-volume-vs-proof ledger + mocked/false-green census. |
| 75 | Security, Auth, Scope Matrix | security | 17K | Canonical trust-boundary map (relay/auth/ACP/MCP/tool-safety). |
| 76 | Data Contracts And Schemas | schemas | 10K | runtime→server→frontend→persistence→docs schema ownership/drift. |
| 77 | Operations, Deploy, Runbook | ops | 9K | Docker/Railway/Fly/health/ops runbook + confirmed blockers. |
| 81 | Root Docs Rewrite Queue | docs | 6K | Maintained README/CLAUDE/v2/demo docs rewrite priority queue. |
| 82 | Command Example Drift Ledger | docs | 5K | Stale/unsafe command snippets + current replacements. |
| 84 | Status Pack Maintenance | pack meta | 3K | How to regenerate/validate the pack cheaply. |
| 88 | Conductor | roko-conductor | 41K | Supervision/watchers/circuit-breaker/diagnosis; imported only by orchestrate.rs. |
| 89 | roko-primitives — HDC & Tier Routing | roko-primitives | 29K | HDC core + tier routing; parallel-impl P0, feature activated by 0 crates. |
| 90 | roko-runtime/-fs/-std | substrate | 25K | Substrate/bus/builtins; 6 NOT-WIRED consumers, orphan state modules. |
| 91 | PRD Lifecycle + Research | self-hosting | 34K | idea→draft→research→plan pipeline status. |
| 92 | Runner v2 — Module Family | roko-cli/runner | 13K | Per-file ledger of the live runner crate (19 files). |
| 93 | roko-demo | roko-demo | 9K | Compiled chain-scenario orchestrator + own TUI (4th TUI surface). |
| 94 | Feed-agents Fleet | roko-serve/feed | 10K | 29 serve-side feed agents driving the events.jsonl firehose. |
| 105 | Frontend: demo/demo-app | demo-app | 21K | React SPA deep-dive; embedded-served, dual SSE managers. |
| 106 | Apps: mirage/relay/watcher | apps/ | 18K | The 3 `apps/` deep-dive; mirage = schema-of-record for serve mirage routes. |

## Census (9)

| # | Title | Subsystem | Size | Purpose |
|---|---|---|---|---|
| 03 | Crate Audit | all packages | 39K | Exhaustive per-package reference (34 members); LOC/status per crate. |
| 06 | Wiring Status | whole system | 14K | Built-but-unwired census; every symbol caller-checked. |
| 11 | Dependency Graph | workspace | 17K | Manifest-parsed dep graph + layering-violation enumeration. |
| 16 | Codebase Inventory | whole system | 13K | Shell-computed file/LOC/API/test/artifact inventory. |
| 80 | Source Doc Manifest | docs corpus | 91K | Generated 636-file docs/v1+v2+v2-depth manifest with owners. |
| 83 | Env Var Manifest | config/env | 16K | Generated direct env-var inventory (99 fixed + dynamic families). |
| 102 | Spec-Debt Ledger (Concept-Level) | spec vs code | 29K | ~129 named v2 concepts → code status; synthesizes 15/18/85/86/87. |
| 103 | Duplicate Types Census | type dup | 19K | 19 cross-crate duplicated type families with no conversions. |
| 104 | Dead Code & Façade Census | dead code | 9K | Feature façades, orphan files, 71 allow(dead_code) sites + verdicts. |

## Spec-Coverage (10)

| # | Title | Subsystem | Size | Purpose |
|---|---|---|---|---|
| 02 | Spec Evolution | spec history | 7K | V1→V2→V2-depth spec changes and where code sits. |
| 14 | V1 Coverage | docs/v1 | 46K | V1 design corpus → implementation coverage audit. |
| 15 | V2 Coverage | docs/v2 | 11K | Navigable v2 coverage summary (detail in 85). |
| 18 | V2-Depth Coverage | docs/v2-depth | 33K | v2-depth (185 files) → code coverage audit. |
| 72 | Source Doc Coverage Ledger | docs corpus | 5K | How much of v1/v2/v2-depth is converted to status docs (points to 80). |
| 78 | V2-Depth Research Prompt Ledger | docs/v2-depth | 7K | Fences the 14 RESEARCH-PROMPT pitch docs from implementation truth. |
| 79 | Reference Provenance Ledger | docs/v1 refs | 7K | v1 bibliography provenance + status-use rules. |
| 85 | V2 Coverage — Kernel (01-07,27) | docs/v2 kernel | 56K | Exhaustive per-concept kernel-spec coverage matrix. |
| 86 | V2 Coverage — Platform (08-19) | docs/v2 platform | 25K | Exhaustive platform-spec coverage matrix. |
| 87 | V2 Coverage — Ecosystem (20-28) | docs/v2 ecosystem | 31K | Exhaustive ecosystem-spec + guides coverage matrix. |

## Trace (6)

| # | Title | Subsystem | Size | Purpose |
|---|---|---|---|---|
| 96 | Trace: Runner v2 Execution | plan exec | 29K | `plan run --engine runner-v2` hop-by-hop; no live DAG, per-plan FSM. |
| 97 | Trace: roko serve Lifecycle | serve | 23K | Startup→request→realtime→write; events.jsonl write-only firehose. |
| 98 | Trace: Self-Hosting Loop | self-hosting | 23K | idea→…→execute traced; honest "can Roko self-host today?" verdict. |
| 99 | Trace: One Agent Turn | agent turn | 27K | End-to-end turn; default Claude-CLI provider bypasses per-tool safety. |
| 100 | Trace: ACP Session | roko-acp | 21K | ACP session lifecycle; permission gate architecturally unwireable. |
| 101 | Trace: Gate Pipeline | gates | 24K | Runner-v2 gate exec; adaptive thresholds/oracles live only on dead path. |

## Tmp-Archaeology (9)

| # | Title | Subsystem | Size | Purpose |
|---|---|---|---|---|
| 17 | Tmp Source Ranking | tmp/ | 7K | Authoritative/current/stale/scratch tiering of tmp folders. |
| 20 | tmp/ Newest (May 8→Jun 23) | tmp/ | 27K | Newest design clusters; still-open P0 dogfood evidence. |
| 21 | tmp/ May 4-6 Batch | tmp/ | 21K | v2-refactor-era batch reconciliation. |
| 22 | tmp/ Legacy (May 1) | tmp/ | 21K | Pre-v2-refactor legacy archive reconciliation. |
| 23 | Runner Lineage & v2 Migration | runners | 14K | How Runner v2 arrived; disambiguates overloaded "runner". |
| 67 | tmp-feedback/2 Crosswalk | dogfood | 10K | May-8 35-issue defect map → current code/docs disposition. |
| 68 | Self-Developing Crosswalk | self-dev UX | 9K | tmp/solutions self-developing UX issues → implementation status. |
| 69 | Residual Audit Tracker | audit | 7K | Continuation tracker from 05-01 subsystem-audit worktree. |
| 70 | Relay Protocol Freeze | relay | 10K | Relay/topic-bus freeze checklist vs shipped wire protocol. |

## Backlog-Execution (0 files; scaffolded)

| Path | Status | Purpose (intended) |
|---|---|---|
| `backlog/epics/` | empty (being built) | Per-epic work-breakdown files (native task DAGs for the fixes). |
| `backlog/exemplars/` | empty (being built) | Worked exemplar tasks/plans agents can imitate. |

---

## Consolidation & Redundancy

Recommendations only — no file is deleted or moved. "Superseded" docs stay on disk as
history but should be demoted in the index and carry a superseded banner.

### Superseded / archive candidates

| Doc(s) | Overlaps with | Recommendation | Rationale |
|---|---|---|---|
| 07-MIGRATION-CHECKLIST | 24, 12, 27 | **MARK-SUPERSEDED-BY-24** (then ARCHIVE) | Self-declares superseded for priority; its checklist is a stale duplicate of the 24/12/27 ordering. |
| 08-TECH-DEBT | 24 | **MARK-SUPERSEDED-BY-24** (then ARCHIVE) | Self-declares superseded; "orchestrate.rs monolith" P0 is stale framing vs current engine-drift P0. |
| 05-ARCHITECTURE-REALITY | 13, 36 | **MARK-SUPERSEDED-BY-13/36** | Header admits orchestrate.rs-centric framing is outdated; 13 (matrix) + 36 (runners) carry current truth. |
| 10-TESTING-STATUS | 16, 74 | **MERGE-INTO-74** (counts → 16) | Test census duplicated; 16 owns the count, 74 owns volume-vs-proof. Keep only the P0 stub caveat if merged. |
| 55-DATA-DIR | 60 | **MARK-SUPERSEDED-BY-60** (keep as light layout index) | 55's own header says the full writer→reader matrix now lives in 60; keep 55 only as a path index. |
| 09-DEPLOYMENT-STATUS | 77, 58 | **MERGE-INTO-77** | First-pass deploy status; 77 (runbook) + 58 (deploy shapes) hold the current blocker set. |

### Coverage-ledger sprawl (Spec-Coverage)

| Doc(s) | Overlaps with | Recommendation | Rationale |
|---|---|---|---|
| 102-SPEC-DEBT-LEDGER | 15/18/85/86/87 | **KEEP-AS-IS (promote to canonical roll-up)** | It explicitly synthesizes the coverage ledgers into one concept→status register; make it the entry point. |
| 15-V2-COVERAGE | 85 | **KEEP as summary-of-85** (cross-link only) | Already the "navigable summary"; keep thin, point detail at 85 + roll-up at 102. |
| 85 / 86 / 87 | 102, 18 | **KEEP-AS-IS (detail tier)** | Exhaustive per-concept matrices are the evidence 102 rolls up; needed for depth, not redundant. |
| 14-V1-COVERAGE | — | **KEEP-AS-IS** | Distinct corpus (v1), not covered by the v2 roll-up. |

### Tmp-archaeology overlap

| Doc(s) | Overlaps with | Recommendation | Rationale |
|---|---|---|---|
| 21-TMP-MAY-BATCH + 22-TMP-LEGACY | each other | **MERGE 21→22** (single "older tmp inventory") | Both are historical time-slice inventories with low live-signal; one archive doc suffices. |
| 20-TMP-NEWEST | 17, 67 | **KEEP-AS-IS** | Newest material carries still-open P0 dogfood evidence; highest live value of the four. |
| 17-TMP-SOURCE-RANKING | 20/21/22 | **KEEP-AS-IS (as the tmp index)** | It's the meta-ranking that orients the three inventories; keep as the section's front door. |

### Engine/runner cluster (heavy but intentional depth)

| Doc(s) | Recommendation | Rationale |
|---|---|---|
| 95 / 37 / 36 / 92 / 96 | **KEEP 95 canonical; MERGE-37-INTO-95** | 95 (nav cross-cut) and 37 (operator decision) say the same thing at the same altitude. 36 (three-gen audit), 92 (module ledger), 96 (trace) are genuinely different depth layers — keep. |

### Companion pairs — KEEP BOTH (not redundant)

03↔54 (crate audit / migration checklist) · 45↔62 (CLI surface / command ledger) ·
46↔59 (serve audit / route ledger) · 43↔105↔106↔93↔94 (surfaces summary / demo-app /
apps / roko-demo / feed-agents) · 72↔80 (coverage ledger / generated manifest) ·
61↔83↔57 (config matrix / env manifest / config audit) · 50↔71 (CI overview / gap table).

---

## Proposed 00-INDEX Section Taxonomy

Nine top-level sections (replaces the current 7). Docs listed in each; a new **Execution
Backlog** section points at `backlog/`.

1. **Start Here (Navigation)** — 01, 12, 13, 24, 25, 26, 27, 28, 29, 95
   *(read-order + decision layer; 95 kept front-and-center as the load-bearing cross-cut)*
2. **Execution Traces** — 96, 97, 98, 99, 100, 101
3. **Censuses & Inventories** — 03, 06, 11, 16, 80, 83, 102, 103, 104
4. **Spec Coverage** — 02, 14, 15, 18, 72, 78, 79, 85, 86, 87
5. **Subsystem Evidence Ledgers — Kernel & Runtime** — 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 44, 47, 48, 49, 52, 56, 88, 89, 90, 91, 92, 94
6. **Subsystem Evidence Ledgers — Surfaces & Ops** — 43, 45, 46, 51, 53, 55, 57, 58, 59, 60, 62, 64, 66, 73, 74, 75, 76, 77, 82, 93, 105, 106
7. **Migration, Debt & Convergence Plans** — 04, 05, 07, 08, 09, 10, 19, 47*, 54, 61, 63, 65, 81, 84, 50, 71
   *(demote superseded 05/07/08/09/10/55 here with banners; 47 cross-listed with §5)*
8. **Tmp Archaeology** — 17, 20, 21, 22, 23, 67, 68, 69, 70
9. **Execution Backlog (NEW)** — `backlog/epics/*`, `backlog/exemplars/*`
   *(the forward-looking work-breakdown layer that turns 24/27 into buildable tasks)*
