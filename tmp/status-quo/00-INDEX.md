# Status Quo: Index

> **Disposition: CURRENT NAVIGATION (2026-08-17).** `.roko/GAPS.md` is the canonical
> human runtime-acceptance roll-up and `MASTER-EXECUTION-CHECKLIST.md` is the execution
> control plane. Numbered audit/census documents and the historical epic table below are
> dated evidence: treat them as historical unless their own leading status banner explicitly
> says otherwise.

> **What is this?** This directory contains 107 numbered audit documents, an executable
> backlog of 48 epics (447 implementation tasks), and supporting evidence produced during systematic
> codebase audits of the roko Rust workspace. Together they track the gap between roko's
> aspirational specifications (`docs/v2/`) and the actual implementation across 35
> workspace members (~800K LOC of Rust).
>
> **Why does it exist?** Roko went through rapid parallel development where many features
> were "built but never wired" -- code compiles and may even have tests, but is not
> reachable from the CLI or runtime. These documents provide ground-truth evidence of
> what actually works, what is partial, and what is facade. They drive the remediation
> backlog that roko's own plan runner executes.
>
> **Terminology:**
> - **Epic (E01-E48):** A large feature area spanning one or more crates. E01-E18 fix
>   audit findings. E19-E48 build toward the v2 specification.
> - **Wave (0-13):** Dependency-ordered development phases. Wave 0 bootstraps the control
>   plane. Waves 1-6 are self-heal plans (SH01-SH06) that fix the runner. Waves 7-13
>   work through the epic backlog.
> - **Self-heal (SH01-SH06):** Six targeted bug-fix plans covering runner lifecycle,
>   isolation, persistence, telemetry, config, and regression harness. All 57/57 done.
> - **Plan (P01-P31):** Implementation plans with tracked tasks, reconciled against
>   audit findings.
> - **Batch:** A multi-agent execution run. Batches 1-10 (2026-07 through 2026-08-13)
>   ran ~230+ agents producing ~130+ confirmed-done tasks.
>
> **Last updated:** 2026-08-17

---

## How to Read This

**If you are new to this directory**, follow this path:

1. **Read [`.roko/GAPS.md`](../../.roko/GAPS.md)** for the current human-readable
   runtime/product boundary. Code and tests remain the ultimate behavioral evidence.
2. **Read [MASTER-EXECUTION-CHECKLIST.md](MASTER-EXECUTION-CHECKLIST.md)** for the
   canonical execution control document: wave-by-wave progress, agent protocols,
   evidence contracts, and what "done" means.
3. **Read [95-ENGINE-DRIFT.md](95-ENGINE-DRIFT.md)** for the dated engine-drift audit
   and its explicit current superseding banner. It explains why the default-runtime
   correction was necessary; it is not the current product roll-up.
4. **Use [01-EXECUTIVE-SUMMARY.md](01-EXECUTIVE-SUMMARY.md)** and
   **[13-CURRENT-STATE-MATRIX.md](13-CURRENT-STATE-MATRIX.md)** only as dated
   2026-08-13 audit baselines. Their unbannered implementation claims are historical,
   not current truth.
5. **Go to [backlog/00-INDEX.md](backlog/00-INDEX.md)** for the executable work plan:
   48 epics, 447 implementation tasks, milestone gates.

**For depth on a specific subsystem**, use Sections 5-8 below to find the relevant
evidence ledger. Each ledger contains `file:line` citations from the codebase.

**For spec coverage**, start at [102-SPEC-DEBT-LEDGER.md](102-SPEC-DEBT-LEDGER.md)
which maps ~129 v2 specification concepts to their implementation status.

**When sources disagree**, use this priority order:

1. Current code behavior, tests, and git history.
2. `.roko/GAPS.md` for the current human runtime/product boundary.
3. The MASTER-EXECUTION-CHECKLIST and per-task evidence on the integration branch.
4. Task records and acceptance requirements in `tasks.toml` files.
5. Dated audit ledgers, older status-quo prose, roadmaps, and historical claims.

## Top-Level Disposition Registry

The 109 direct Markdown files under `tmp/status-quo/` have one deterministic disposition
rule. This prevents an old audit from becoming current merely because its filename appears
in this index:

| Scope | Count | Disposition |
|---|---:|---|
| `00-INDEX.md` | 1 | `CURRENT-NAVIGATION`; this file routes readers to current owners but does not replace runtime evidence. |
| `MASTER-EXECUTION-CHECKLIST.md` | 1 | `CURRENT-CONTROL`; raw checklist markers and execution protocol. |
| `DOC-MANIFEST.md` | 1 | `GENERATED-HISTORICAL`; dated taxonomy, never current implementation proof. |
| Numbered `01-*.md` through `106-*.md` | 106 | `HISTORICAL-AUDIT` by default. A document may override that default only with an explicit leading `CURRENT`, `AUTHORITATIVE-PARTIAL`, `SUPERSEDED`, or `HISTORICAL` disposition banner. |
| **Total** | **109** | Every direct top-level status document is classified. |

The classification is about authority, not deletion: historical evidence remains useful for
provenance. Current behavior comes from code/tests, `.roko/GAPS.md`, the master checklist,
and any explicitly current document banner, in that order.

---

## Overall Programme Status

As of 2026-08-17 (use `.roko/GAPS.md` as the canonical runtime-acceptance rollup):

- **Self-heal plans (SH01-SH06):** 57/57 done. Runner lifecycle, isolation, persistence,
  telemetry, config, and regression harness are all remediated.
- **Accepted epics (48/48, canonical roll-up):** E01-E48. Acceptance boundaries and
  broader product residuals remain explicit in `.roko/GAPS.md`.
- **Partial epics:** none.
- **Greenfield epic-manifest work:** zero tasks.
- **Broader master census:** 209 done, 8 partial, and 25 unchecked raw markers before
  the current documentation/projection tranche; these include proof and product residuals.
- **Executable plan queue:** 30/30 plans and 124/124 tasks complete.
- **Roadmap residuals outside completed manifests:** E32 still has aspirational
  WIT/Component hostcalls and OpenClaw/legacy adapter parity; E37 still has named-surface
  rendering/command ingress and native autonomy/graph/HDC source work.
- **Executable task manifests:** `backlog/plans/00-INDEX.md` mirrors each `[meta].done`
  value; those historical metadata counts are intentionally distinct from runtime acceptance.
- **Signal rename:** Done (2026-08-12) -- Engram-to-Signal flip landed across the
  codebase.
- **docs/v2 annotated:** Done (2026-08-12) -- every v2 document has an explicit
  status marker; accuracy still requires ongoing reconciliation as wiring changes.
- **Production error handling:** Partial -- `.expect()` replaced with proper errors
  in batch 10, but coverage is incomplete.
- **Documentation plans:** 71 corpus-reconciliation tasks remain in Wave 13; source
  ownership coverage is complete and drift enforcement is tracked separately.
- **Deprecated rate-oracle removal (2026-08-13):** The ISFR vertical has been removed
  from active Rust, demo, and v2 documentation surfaces. Historical audit records remain.

---

## 1. Top-Level Control Documents

These are the documents you use to drive and track the remediation programme.

| File | Size | What it is |
|------|------|-----------|
| [MASTER-EXECUTION-CHECKLIST.md](MASTER-EXECUTION-CHECKLIST.md) | 82K | **The canonical control document.** Wave-by-wave task status, multi-agent scheduling protocol, git/worktree/commit/merge rules, evidence contracts, milestone gates. The only starting document an agent needs. |
| [DOC-MANIFEST.md](DOC-MANIFEST.md) | 19K | Taxonomy of all 107 numbered documents: category, subsystem, size, purpose, and consolidation recommendations. |

## 2. Navigation Layer (10 documents)

Decision and strategy documents. Read these to understand the project's state and direction.

| # | File | Size | What it covers |
|---|------|------|---------------|
| 95 | [ENGINE-DRIFT](95-ENGINE-DRIFT.md) | 12K | Historical default-engine/dry-run audit with a current superseding banner: Runner v2 is default and the Graph plan path dispatches live providers but lacks lifecycle parity. |
| 01 | [EXECUTIVE-SUMMARY](01-EXECUTIVE-SUMMARY.md) | 11K | Dated 2026-08-13 audit baseline. Its P0 findings and inventory are retained as provenance, not current implementation truth. |
| 13 | [CURRENT-STATE-MATRIX](13-CURRENT-STATE-MATRIX.md) | 12K | Dated 2026-08-13 per-subsystem audit baseline. Its Live / Partial / Stub / Legacy rows require current-code verification before use. |
| 12 | [ROADMAP](12-ROADMAP.md) | 14K | Risk-ordered implementation sequence (Phases 0-3). Each item paired with a proof gate. |
| 24 | [OPEN-ISSUE-LEDGER](24-OPEN-ISSUE-LEDGER.md) | 21K | P0/P1/P2 debt register with codebase evidence and done-when criteria. |
| 25 | [PROOF-GATES](25-PROOF-GATES.md) | 12K | Runnable commands + pass criteria that prove a fix is actually done. |
| 26 | [CANONICAL-DECISIONS](26-CANONICAL-DECISIONS.md) | 11K | Architecture decisions ratified to stop drift (engine choice, noun naming, persistence paths). |
| 27 | [IMPLEMENTATION-BACKLOG](27-IMPLEMENTATION-BACKLOG.md) | 10K | Risk-sliced actionable backlog. Every item closes with a proof gate. |
| 28 | [DEFINITION-OF-DONE](28-DEFINITION-OF-DONE.md) | 6K | Strict criteria for "fully migrated": default path + durable state + test + docs. |
| 29 | [RISK-REGISTER](29-RISK-REGISTER.md) | 9K | Risks that create false confidence, data loss, or security exposure, with mitigations. |

## 3. Execution Backlog

The [`backlog/`](backlog/00-INDEX.md) subfolder turns audit findings into roko-executable
plans: 48 epics (E01-E48), 447 implementation tasks, 71 DOC reconciliation tasks, grouped
into 4 milestones (M0 bootstrap through M3+).

### Backlog documents

| File | What it covers |
|------|---------------|
| [backlog/00-INDEX.md](backlog/00-INDEX.md) | Backlog navigation, full 48-epic table, how to execute plans. |
| [backlog/01-TASK-EXECUTION-SCHEMA.md](backlog/01-TASK-EXECUTION-SCHEMA.md) | Canonical `tasks.toml` schema for authoring executable tasks. |
| [backlog/02-PLANS-RECONCILIATION.md](backlog/02-PLANS-RECONCILIATION.md) | Maps existing `plans/` (P08-P34) to audit findings. |
| [backlog/03-WORK-BREAKDOWN-EPICS.md](backlog/03-WORK-BREAKDOWN-EPICS.md) | Master roadmap: epic dependency DAG, milestones, critical path, parallel tracks. |
| [backlog/04-EXECUTION-READINESS.md](backlog/04-EXECUTION-READINESS.md) | M0 bootstrap gate -- prerequisites before any epic can run. |
| [backlog/05-MASTER-CHECKLIST.md](backlog/05-MASTER-CHECKLIST.md) | Flat tickable checklist of all tasks by milestone. |
| [backlog/06-EXECUTABLE-TASK-FILE-COVERAGE.md](backlog/06-EXECUTABLE-TASK-FILE-COVERAGE.md) | Coverage ledger for materialized task files (149 implementation tasks, 0 gaps). |
| [backlog/07-SUBAGENT-TASK-AUTHORING-NOTES.md](backlog/07-SUBAGENT-TASK-AUTHORING-NOTES.md) | Corrections for task blocks with stale paths, deps, or scopes. |
| [backlog/08-SOURCE-CORPUS-PLAN-COVERAGE.md](backlog/08-SOURCE-CORPUS-PLAN-COVERAGE.md) | 746 source docs mapped into DOC plans. |
| [backlog/09-UNIFIED-ROADMAP.md](backlog/09-UNIFIED-ROADMAP.md) | Unified roadmap combining all tracks. |
| [backlog/10-EPIC-DEPENDENCY-MATRIX.md](backlog/10-EPIC-DEPENDENCY-MATRIX.md) | Which epics block which -- dependency analysis. |
| [backlog/11-EXECUTION-PLAYBOOK.md](backlog/11-EXECUTION-PLAYBOOK.md) | Operations playbook for running multi-agent execution. |
| [backlog/12-MILESTONE-DEFINITIONS.md](backlog/12-MILESTONE-DEFINITIONS.md) | M0-M3+ milestone definitions and acceptance criteria. |
| [backlog/13-PLAN-AUDIT-E19-E30.md](backlog/13-PLAN-AUDIT-E19-E30.md) | Plan audit for v2 epics E19-E30. |
| [backlog/14-PLAN-AUDIT-E31-E42.md](backlog/14-PLAN-AUDIT-E31-E42.md) | Plan audit for v2 epics E31-E42. |
| [backlog/15-PLAN-AUDIT-E43-E48.md](backlog/15-PLAN-AUDIT-E43-E48.md) | Plan audit for operational epics E43-E48. |
| [backlog/16-FINAL-GAP-ANALYSIS.md](backlog/16-FINAL-GAP-ANALYSIS.md) | Final gap analysis across all plans. |
| [backlog/17-OPERATIONAL-OWNERSHIP.md](backlog/17-OPERATIONAL-OWNERSHIP.md) | Ownership assignments for operational concerns. |
| [backlog/GAP-REPORT-V3.md](backlog/GAP-REPORT-V3.md) | Coverage gaps feeding E13 spec-debt. |

### Backlog subdirectories

| Path | What it contains |
|------|-----------------|
| [backlog/plans/](backlog/plans/) | 48 per-epic `tasks.toml` plan directories (E01-E48) + 6 DOC plans. |
| [backlog/epics/](backlog/epics/) | 18 epic markdown specs (E01-E18) with granular task requirements. |
| [backlog/exemplars/](backlog/exemplars/) | 3 drop-in `plan validate`-clean `tasks.toml` exemplars. |
| [backlog/source-coverage/](backlog/source-coverage/) | 6 per-corpus coverage ledgers. |
| [backlog/references/](backlog/references/) | Planning methodology docs and naming decisions. |
| [backlog/decisions/](backlog/decisions/) | Architecture decision records. |

## 4. Execution Traces

Hop-by-hop traces through real runtime code paths, with `file:line` evidence. These are the
deepest evidence layer -- they correct earlier documents where those were wrong.

| # | File | Size | What it traces |
|---|------|------|---------------|
| 96 | [TRACE-RUNNER-V2-EXECUTION](96-TRACE-RUNNER-V2-EXECUTION.md) | 29K | Full `plan run --engine runner-v2` path: flat task-index scheduling (no live DAG), per-plan FSM, agent dispatch, gate dispatch, snapshots. |
| 97 | [TRACE-SERVE-LIFECYCLE](97-TRACE-SERVE-LIFECYCLE.md) | 23K | `roko serve` startup through request handling, realtime (SSE/WS), and state writes. Key finding: events.jsonl is a 44 MB write-only firehose that no panel reads. |
| 98 | [TRACE-SELF-HOSTING-LOOP](98-TRACE-SELF-HOSTING-LOOP.md) | 23K | The full idea-to-execution self-hosting loop: idea, draft, research, plan, execute. Honest verdict on what works at each step. |
| 99 | [TRACE-AGENT-TURN](99-TRACE-AGENT-TURN.md) | 28K | One agent turn end-to-end. Key finding: Claude CLI (the default provider) bypasses per-tool safety entirely. |
| 100 | [TRACE-ACP-SESSION](100-TRACE-ACP-SESSION.md) | 22K | ACP (Agent Client Protocol) session lifecycle. Key finding: permission gate is architecturally unwireable as-is (wrong side of a task boundary). |
| 101 | [TRACE-GATE-PIPELINE](101-TRACE-GATE-PIPELINE.md) | 25K | Gate execution pipeline. Key finding: adaptive thresholds and oracles were live only on the dead orchestrate.rs path, not on Runner v2. |

## 5. Censuses and Inventories

Whole-tree enumerations and generated manifests.

| # | File | Size | What it inventories |
|---|------|------|-------------------|
| 03 | [CRATE-AUDIT](03-CRATE-AUDIT.md) | 39K | All 35 workspace members: LOC, status, purpose per crate. ~800K LOC. |
| 06 | [WIRING-STATUS](06-WIRING-STATUS.md) | 14K | Built-but-unwired census. Every symbol checked for callers, with WIRED/PARTIAL/DEAD/ORPHAN tags. |
| 11 | [DEPENDENCY-GRAPH](11-DEPENDENCY-GRAPH.md) | 17K | Workspace crate layering (L0-L4), dependency edges, and violation enumeration. |
| 16 | [CODEBASE-INVENTORY](16-CODEBASE-INVENTORY.md) | 13K | File count, LOC, API surface, test attribute, and artifact inventory. |
| 80 | [SOURCE-DOC-MANIFEST](80-SOURCE-DOC-MANIFEST.md) | 92K | Generated manifest of 637 docs files with status tags and owners. Largest file in the pack. |
| 83 | [ENV-VAR-MANIFEST](83-ENV-VAR-MANIFEST.md) | 16K | 99 fixed environment variables + dynamic families. |
| 102 | [SPEC-DEBT-LEDGER](102-SPEC-DEBT-LEDGER.md) | 30K | ~129 named v2 specification concepts mapped to code status (~24 built, ~52 partial, ~48 zero-code). Synthesizes 15/18/85-87. |
| 103 | [DUPLICATE-TYPES-CENSUS](103-DUPLICATE-TYPES-CENSUS.md) | 19K | 19 cross-crate duplicated type families with no conversions (e.g., GateVerdict 4x). |
| 104 | [DEAD-CODE-AND-FACADE-CENSUS](104-DEAD-CODE-AND-FACADE-CENSUS.md) | 9K | Feature facades, orphan files, 71 `allow(dead_code)` sites. |

## 6. Spec Coverage

How much of the v1/v2 specifications is implemented in code. Start at 102 for the
concept-level roll-up, then drill into per-layer detail.

| # | File | Size | What it covers |
|---|------|------|---------------|
| 02 | [SPEC-EVOLUTION](02-SPEC-EVOLUTION.md) | 7K | How the specification evolved v1 to v2 to v2-depth, and where code sits relative to each. |
| 14 | [V1-COVERAGE](14-V1-COVERAGE.md) | 47K | v1 design corpus mapped to implementation status. |
| 15 | [V2-COVERAGE](15-V2-COVERAGE.md) | 11K | v2 coverage summary (detail in 85-87). |
| 18 | [V2-DEPTH-COVERAGE](18-V2-DEPTH-COVERAGE.md) | 34K | v2-depth (185 files) mapped to code coverage. |
| 72 | [SOURCE-DOC-COVERAGE-LEDGER](72-SOURCE-DOC-COVERAGE-LEDGER.md) | 5K | How much of v1/v2/v2-depth has been converted into status documents. |
| 78 | [V2-DEPTH-RESEARCH-PROMPT-LEDGER](78-V2-DEPTH-RESEARCH-PROMPT-LEDGER.md) | 7K | Separates 14 aspirational RESEARCH-PROMPT pitch docs from implementation truth. |
| 79 | [REFERENCE-PROVENANCE-LEDGER](79-REFERENCE-PROVENANCE-LEDGER.md) | 8K | v1 bibliography provenance and usage rules. |
| 85 | [V2-COVERAGE-KERNEL](85-V2-COVERAGE-KERNEL.md) | 56K | Exhaustive kernel-spec (chapters 01-07, 27) coverage matrix. |
| 86 | [V2-COVERAGE-PLATFORM](86-V2-COVERAGE-PLATFORM.md) | 25K | Exhaustive platform-spec (chapters 08-19) coverage matrix. |
| 87 | [V2-COVERAGE-ECOSYSTEM](87-V2-COVERAGE-ECOSYSTEM.md) | 31K | Exhaustive ecosystem-spec (chapters 20-28) coverage matrix. |

## 7. Subsystem Evidence Ledgers

Deep per-subsystem audits with `file:line` evidence. Each covers one crate family or
cross-cutting concern. Organized by domain.

### Core kernel

| # | File | Size | What it audits |
|---|------|------|---------------|
| 30 | [CORE-SIGNAL](30-CORE-SIGNAL.md) | 30K | roko-core: the Signal noun (formerly Engram), 6 verb traits, storage split-brain. |
| 31 | [GRAPH-CELLS-ENGINE](31-GRAPH-CELLS-ENGINE.md) | 29K | Historical Graph/Cell audit; current plan path has live TaskExecutor dispatch and typed cognitive Cells, with incomplete Runner-v2 lifecycle parity. |
| 32 | [EVENTS-BUS-STATEHUB](32-EVENTS-BUS-STATEHUB.md) | 33K | Event bus, PulseBus, StateHub push-based dashboard, relay topics, feed agents. |
| 47 | [FOUNDATION-TYPES-REDESIGN](47-FOUNDATION-TYPES-REDESIGN.md) | 10K | DispatchPlan / RunLedger / GateStatus / CommitOutcome type consolidation. |
| 89 | [PRIMITIVES-HDC](89-PRIMITIVES-HDC.md) | 30K | roko-primitives: HDC (hyperdimensional computing) vectors and tier routing. |

### Orchestration and execution

| # | File | Size | What it audits |
|---|------|------|---------------|
| 36 | [ORCHESTRATION-RUNNERS](36-ORCHESTRATION-RUNNERS.md) | 39K | Historical three-generation orchestration audit; current Graph live-dispatch status is superseded by the master checklist. |
| 37 | [RUNNER-V2-AND-GRAPH](37-RUNNER-V2-AND-GRAPH.md) | 7K | Historical operator decision doc with a current banner: Runner v2 owns the full lifecycle; Graph has live dispatch but incomplete parity. |
| 92 | [RUNNER-V2-MODULE-FAMILY](92-RUNNER-V2-MODULE-FAMILY.md) | 14K | Runner v2 per-file ledger (19 files, the live engine). |
| 88 | [CONDUCTOR](88-CONDUCTOR.md) | 41K | roko-conductor: supervision, 10 watchers, circuit breaker, diagnosis. |
| 90 | [RUNTIME-FS-STD](90-RUNTIME-FS-STD.md) | 25K | roko-runtime (event bus, process supervisor), roko-fs (JSONL substrate), roko-std (builtins). |

### Agent and dispatch

| # | File | Size | What it audits |
|---|------|------|---------------|
| 33 | [AGENT-SAFETY](33-AGENT-SAFETY.md) | 21K | Safety contracts, taint propagation, capabilities, custody chain. |
| 34 | [COMPOSE-PROMPTS](34-COMPOSE-PROMPTS.md) | 35K | Prompt composition, VCG auction, 9-layer system prompt builder, context assembly. |
| 38 | [AGENT-PROVIDERS-TOOLS](38-AGENT-PROVIDERS-TOOLS.md) | 36K | 10 LLM providers (Claude CLI/API, Codex, Cursor, OpenAI, Ollama, Gemini, Perplexity...), pools, tool loop. |
| 44 | [AGENT-SERVER](44-AGENT-SERVER.md) | 35K | Per-agent HTTP sidecar: 13 routes, live vs mirage tally, dead task queue. |

### Gates and verification

| # | File | Size | What it audits |
|---|------|------|---------------|
| 35 | [GATES-VERIFICATION](35-GATES-VERIFICATION.md) | 28K | 11 gates, 7-rung pipeline, verdict signals, adaptive threshold status. |

### Knowledge, learning, and affect

| # | File | Size | What it audits |
|---|------|------|---------------|
| 39 | [NEURO-KNOWLEDGE](39-NEURO-KNOWLEDGE.md) | 28K | Durable knowledge store, admission, tier progression, decay, HDC retrieval. |
| 40 | [LEARNING-TELEMETRY](40-LEARNING-TELEMETRY.md) | 39K | Episodes, cascade router, prompt experiments, efficiency events. LinUCB weights not persisted. |
| 41 | [DREAMS](41-DREAMS.md) | 24K | Dream consolidation across three engines (hypnagogia, imagination, cycle). Phase 2 shells. |
| 56 | [DAIMON](56-DAIMON.md) | 31K | Affect engine + somatic markers + dispatch modulation. Live on Runner v2, dark on Graph. |

### Integrations

| # | File | Size | What it audits |
|---|------|------|---------------|
| 42 | [CHAIN-REGISTRIES-ISFR](42-CHAIN-REGISTRIES-ISFR.md) | 44K | Chain client, contracts, registries, ISFR vertical, relay, deploy. Most modules have 0 callers. |
| 48 | [MCP-CRATES](48-MCP-CRATES.md) | 32K | MCP server crates (code/github/slack/scripts/stdio) + config passthrough drift. |
| 49 | [INDEX-LANG](49-INDEX-LANG.md) | 30K | Code intelligence index, language parsers (Rust/TypeScript/Go), duplicate HDC struct. |
| 52 | [PLUGIN-EXTENSIONS](52-PLUGIN-EXTENSIONS.md) | 37K | Plugin SDK, extension hooks, manifest gaps. |
| 91 | [PRD-RESEARCH](91-PRD-RESEARCH.md) | 35K | idea-to-draft-to-research-to-plan pipeline status. |
| 94 | [FEED-AGENTS-FLEET](94-FEED-AGENTS-FLEET.md) | 10K | 29 serve-side feed agents driving events.jsonl firehose. |

### CLI, TUI, and surfaces

| # | File | Size | What it audits |
|---|------|------|---------------|
| 45 | [CLI-SURFACE](45-CLI-SURFACE.md) | 24K | 45 top-level CLI commands, ~155 leaves. |
| 62 | [CLI-COMMAND-LEDGER](62-CLI-COMMAND-LEDGER.md) | 16K | Command-by-command status ledger (companion to 45). |
| 43 | [SURFACES-DEMO-UX](43-SURFACES-DEMO-UX.md) | 43K | TUI dashboard, roko-demo, demo-app, apps overview, UX migration. |
| 93 | [ROKO-DEMO](93-ROKO-DEMO.md) | 9K | Compiled chain-scenario orchestrator with its own TUI (4th surface). |

### HTTP API, serve, and frontend

| # | File | Size | What it audits |
|---|------|------|---------------|
| 46 | [SERVE-HTTP-REALTIME](46-SERVE-HTTP-REALTIME.md) | 30K | Routes, auth middleware, StateHub, SSE/WS, persistence. |
| 59 | [API-ROUTE-LEDGER](59-API-ROUTE-LEDGER.md) | 20K | Route-by-route maturity table (companion to 46). |
| 66 | [FRONTEND-API-PARITY](66-FRONTEND-API-PARITY.md) | 11K | React DataHub / API route parity + route ownership. |
| 105 | [FRONTEND-DEMO-APP](105-FRONTEND-DEMO-APP.md) | 22K | React SPA deep-dive: embedded-served via rust-embed, dual SSE managers. |
| 106 | [APPS-MIRAGE-RELAY-WATCHER](106-APPS-MIRAGE-RELAY-WATCHER.md) | 18K | 3 `apps/` crates: mirage (schema of record for serve routes), relay, watcher. |

### ACP (editor integration)

| # | File | Size | What it audits |
|---|------|------|---------------|
| 51 | [ACP](51-ACP.md) | 22K | ACP editor integration. Permission gate unwireable as-is (see trace 100). |

### Persistence and state

| # | File | Size | What it audits |
|---|------|------|---------------|
| 55 | [DATA-DIR](55-DATA-DIR.md) | 29K | `.roko/` filesystem layout. |
| 60 | [STATE-PERSISTENCE-LEDGER](60-STATE-PERSISTENCE-LEDGER.md) | 22K | Deep writer-to-reader matrix. Which files are written, which are read, which are orphaned. |
| 76 | [DATA-CONTRACTS-SCHEMAS](76-DATA-CONTRACTS-SCHEMAS.md) | 10K | Runtime / server / frontend / schema ownership and drift. |

### Config and environment

| # | File | Size | What it audits |
|---|------|------|---------------|
| 57 | [CONFIG](57-CONFIG.md) | 31K | Config schema (roko.toml), secrets, providers, models. |
| 61 | [CONFIG-ENV-MATRIX](61-CONFIG-ENV-MATRIX.md) | 7K | Config source / env var / secret / provenance matrix. |

### Observability

| # | File | Size | What it audits |
|---|------|------|---------------|
| 53 | [OBSERVABILITY](53-OBSERVABILITY.md) | 31K | Tracing, metrics, logs. Lens subsystem at 0. events.jsonl firehose (97% feed_tick). |

### Security and auth

| # | File | Size | What it audits |
|---|------|------|---------------|
| 75 | [SECURITY-AUTH-SCOPE-MATRIX](75-SECURITY-AUTH-SCOPE-MATRIX.md) | 17K | Trust boundary map across relay, auth, ACP, MCP, and tool safety. |

### Testing, CI, and quality

| # | File | Size | What it audits |
|---|------|------|---------------|
| 50 | [QUALITY-CI-RELEASE](50-QUALITY-CI-RELEASE.md) | 7K | Testing, CI, release, doc-proof policy overview. |
| 64 | [PARITY-TEST-MATRIX](64-PARITY-TEST-MATRIX.md) | 11K | Cross-surface parity tests + false-green census. |
| 71 | [CI-RELEASE-PROOF-GAPS](71-CI-RELEASE-PROOF-GAPS.md) | 7K | Workflow-by-workflow CI gap table. |
| 73 | [EXAMPLES-PLANS-GRAPHS](73-EXAMPLES-PLANS-GRAPHS.md) | 11K | Graph, plan, and PRD example assets + which fail to load. |
| 74 | [TEST-AND-PROOF-INVENTORY](74-TEST-AND-PROOF-INVENTORY.md) | 15K | Test volume vs proof quality + mocked/false-green census. |

### Jobs, deploy, and operations

| # | File | Size | What it audits |
|---|------|------|---------------|
| 58 | [JOBS-DEPLOY](58-JOBS-DEPLOY.md) | 22K | Jobs marketplace, worker, daemon, deploy shapes + blockers. |
| 77 | [OPERATIONS-DEPLOY-RUNBOOK](77-OPERATIONS-DEPLOY-RUNBOOK.md) | 9K | Docker / Railway / Fly / health / ops runbook gaps. |
| 82 | [COMMAND-EXAMPLE-DRIFT-LEDGER](82-COMMAND-EXAMPLE-DRIFT-LEDGER.md) | 6K | Stale command snippets in docs + current replacements. |

## 8. Migration, Debt, and Convergence Plans

Documents tracking migration state, debt reduction, and doc convergence. Some are superseded
by later documents (noted inline) but kept for historical context.

| # | File | Size | What it covers | Current? |
|---|------|------|---------------|----------|
| 04 | [NAMING-MIGRATION](04-NAMING-MIGRATION.md) | 31K | Signal/Engram naming state, grep census of the noun-flip blast radius. Engram-to-Signal rename landed 2026-08-12. | Historical (rename done) |
| 05 | [ARCHITECTURE-REALITY](05-ARCHITECTURE-REALITY.md) | 11K | Architecture reality notes. | Superseded by 13, 36 |
| 07 | [MIGRATION-CHECKLIST](07-MIGRATION-CHECKLIST.md) | 8K | Phase-by-phase V1-to-V2 migration checklist. | Superseded by 12, 24 |
| 08 | [TECH-DEBT](08-TECH-DEBT.md) | 7K | First-pass tech debt list. | Superseded by 24 |
| 09 | [DEPLOYMENT-STATUS](09-DEPLOYMENT-STATUS.md) | 4K | First-pass deploy readiness. | Superseded by 77, 58 |
| 10 | [TESTING-STATUS](10-TESTING-STATUS.md) | 18K | Test coverage summary (~9,968 test attributes, per-crate census). | Superseded by 16, 74 |
| 19 | [DOC-DRIFT-REGISTER](19-DOC-DRIFT-REGISTER.md) | 10K | Claims in docs (CLAUDE.md, README, v2) that no longer match code. | Current |
| 54 | [PER-CRATE-MIGRATION-CHECKLIST](54-PER-CRATE-MIGRATION-CHECKLIST.md) | 18K | Crate keep / merge / quarantine guidance. | Current |
| 63 | [DELETE-ARCHIVE-PLAN](63-DELETE-ARCHIVE-PLAN.md) | 4K | Removal and archive candidates with required proofs. | Current |
| 65 | [DOCS-CONVERGENCE-PLAN](65-DOCS-CONVERGENCE-PLAN.md) | 8K | How to converge root / v1 / v2 / v2-depth / tmp / code docs. | Current |
| 70 | [RELAY-PROTOCOL-FREEZE](70-RELAY-PROTOCOL-FREEZE.md) | 10K | Relay and API response-shape freeze checklist. | Current |
| 81 | [ROOT-DOCS-REWRITE-QUEUE](81-ROOT-DOCS-REWRITE-QUEUE.md) | 6K | README / CLAUDE.md / v2 / demo docs rewrite queue. | Current |
| 84 | [STATUS-PACK-MAINTENANCE](84-STATUS-PACK-MAINTENANCE.md) | 3K | How to regenerate and validate the status-quo pack itself. | Current |

## 9. Tmp Archaeology

Reconciliation of the `tmp/` design material accumulated during development. The `tmp/`
directory contains ~6,700 markdown/TOML/shell files from various design phases.

| # | File | Size | What it covers |
|---|------|------|---------------|
| 17 | [TMP-SOURCE-RANKING](17-TMP-SOURCE-RANKING.md) | 7K | Meta-ranking: which tmp/ folders are authoritative, current, stale, or scratch. |
| 20 | [TMP-NEWEST](20-TMP-NEWEST.md) | 27K | Newest tmp material (May 8 - Jun 23). Still-open P0 dogfood evidence. |
| 21 | [TMP-MAY-BATCH](21-TMP-MAY-BATCH.md) | 22K | May batch reconciliation. |
| 22 | [TMP-LEGACY](22-TMP-LEGACY.md) | 22K | Older tmp folder archaeology. |
| 23 | [TASKRUNNER-MIGRATION-STATUS](23-TASKRUNNER-MIGRATION-STATUS.md) | 15K | Runner lineage: how the plan engine evolved through three generations. |
| 67 | [TMP-FEEDBACK-2-CROSSWALK](67-TMP-FEEDBACK-2-CROSSWALK.md) | 10K | May-8 35-issue defect map to current disposition. |
| 68 | [SELF-DEVELOPING-CROSSWALK](68-SELF-DEVELOPING-CROSSWALK.md) | 9K | Self-developing architecture spec to implementation status. |
| 69 | [RESIDUAL-AUDIT-TRACKER](69-RESIDUAL-AUDIT-TRACKER.md) | 7K | Remaining audit work and residual unknowns. |

## 10. Subdirectories

| Path | Contents | File count |
|------|----------|-----------|
| [audit-2026-07-14/](audit-2026-07-14/README.md) | July 14 re-audit: top-level document audit, backlog/roadmap audit, issues/self-heal audit. The baseline that spawned the MASTER-EXECUTION-CHECKLIST. | 4 files |
| [backlog/](backlog/00-INDEX.md) | Executable work plan: 48 epic plan directories, 18 epic specs, coverage ledgers, gap reports, exemplars. See Section 3 above. | ~25 docs + subdirectories |
| [execution-evidence/](execution-evidence/) | Per-task implementation evidence and independent review records. Created by worker agents during remediation. | 95 files |
| [issues/](issues/) | 75 issue documents from the original audit. Each describes a specific defect or gap with codebase evidence. These feed the self-heal plans. | 75 files |
| [self-heal/](self-heal/README.md) | Self-heal plans (SH01-SH06): 57 tasks converting issue findings into runner fixes. All 57/57 done. Contains plans, changelog, and coverage mapping. | plans + changelog |

---

## Epic Status Summary

### Audit Epics (E01-E18) -- fixing what exists

| Epic | Name | Status | Tasks | Key crate(s) |
|------|------|--------|-------|-------------|
| E01 | Execution Engine | Done | 16/16 | roko-cli/runner, roko-graph |
| E02 | Storage Convergence | Done | 12/12 | roko-fs, roko-core |
| E03 | Type Consolidation | Done | 7/7 | roko-core, cross-crate |
| E04 | Security Perimeter | Done | 19/19 | roko-serve, roko-agent/safety |
| E05 | Gate Adaptivity (Live) | Done | 8/8 | roko-gate, runner/gate_dispatch |
| E06 | Compose / Prompt Unification | Done | 9/9 | roko-compose |
| E07 | Learning & Knowledge Loops | Done | 10/10 | roko-learn, roko-neuro |
| E08 | Conductor Supervision | Done | 9/9 | roko-conductor |
| E09 | Observability | Done | 11/11 | roko-conductor, roko-fs |
| E10 | Frontend / API Contract | Done | 7/7 | roko-serve, demo-app |
| E11 | Chain / ISFR | Done | 5/5 | roko-chain |
| E12 | Dead-Code & Legacy Cleanup | Done | 9/9 | cross-crate |
| E13 | v2 Spec-Debt (long-horizon) | Done | 3/3 | roko-core |
| E14 | Providers & Tools | Done | 12/12 | roko-agent |
| E15 | MCP Config & Passthrough | Done | 7/7 | roko-agent, roko-mcp-* |
| E16 | PRD & Self-Hosting Pipeline | Done | 2/2 | roko-cli |
| E17 | ACP Completion | Done | 8/8 | roko-acp |
| E18 | Docs, Config, CI & Ops | Done | 15/15 | roko-cli, cross-crate |

### Foundation Epics (E19-E22) -- v2 kernel

| Epic | Name | Status | Tasks | Key crate(s) |
|------|------|--------|-------|-------------|
| E19 | Signal Protocol | Done | 10/10 | roko-core |
| E20 | Cell Unification | Done | 10/10 | roko-core, cross-crate |
| E21 | Graph Engine | Done | 10/10 | roko-graph |
| E22 | Execution Runtime | Done | 10/10 | roko-graph, roko-runtime |

### Platform Epics (E23-E35) -- v2 platform capabilities

| Epic | Name | Status | Tasks |
|------|------|--------|-------|
| E23 | Agent Cognitive Autonomy | Done | 10/10; EFE/GoalTree ownership and phase/energy-aware runner dispatch close the manifest. Native E33 observation publication remains separate product scope |
| E24 | Memory Advanced | Not started | 0/10 |
| E25 | Learning Loops Advanced | Done | 10/10; HDC consolidation, hindsight adjustments, governed c-factor recommendations, significance/early stopping, Variance Inequality, autocatalytic metrics, and live when/then playbook enrichment |
| E26 | Inference Gateway | Not started | 0/12 |
| E27 | Feeds System | Not started | 0/10 |
| E28 | Groups & Coordination | Not started | 0/8 |
| E29 | Connectivity & Relay | Not started | 0/9 |
| E30 | Extension System | Done | 9/9 |
| E31 | Trigger System | Done | 8/8; bundled watcher-to-raw-EVM ABI/finality/reorg path is live and tested |
| E32 | Tool & Plugin Ecosystem | Done | 8/8 manifest; Component-hostcall/OpenClaw/one-shot work remains separately tracked roadmap scope |
| E33 | Telemetry & Lens | Done | 9/9 manifest and runtime ingress complete; all 39 variants have production evidence, including a durable typed registered-agent lifecycle observation boundary. Direct native Agent publication remains separate product scope |
| E34 | Security IFC | Done | 8/8 strict; trust-origin propagation, exact capability intersection, persistent transitive quarantine, and mandatory audited hooks join the immune/corrigibility/sandbox foundations. Provider-internal semantic/effect coverage remains separate product scope |
| E35 | Auth Protocol | Done | 8/8 |

### Ecosystem Epics (E36-E45) -- v2 ecosystem

| Epic | Name | Status | Tasks |
|------|------|--------|-------|
| E36 | Payments | Done | 8/8; x402 batching, MPP sessions, reputation pricing, paid-feed 402 enforcement, cost persistence, and dashboard events |
| E37 | Surfaces | Done | 9/9 shared contract/backend manifest; full named-surface rendering and native source/command wiring remain roadmap residuals |
| E38 | Marketplace | Not started | 0/9 |
| E39 | Registries & Identity | Not started | 0/8 |
| E40 | Arenas & Evals | Not started | 0/8 |
| E41 | DeFi Products | Not started | 0/8 |
| E42 | Config Evolution | Done | 8/8; priority/provenance, seven invariants, migrations, profiles, transactional reload, and freshness diagnostics |
| E43 | Deployment & Portability | Done | 8/8 |
| E44 | Cross-Cut Functors | Done | 8/8; Memory/Daimon/Dreams/Safety functors, six transforms, conflict VCG, and live gate-failure cascade |
| E45 | Orchestrator Mori Parity | Done | 10/10 |

### Operational Epics (E46-E48) -- infrastructure

| Epic | Name | Status | Tasks |
|------|------|--------|-------|
| E46 | GitHub Workflow Integration | Complete | 12/12 |
| E47 | Resource & Disk Management | Complete | 11/11 |
| E48 | Rate Limit & Budgeting | Complete | 12/12 |

---

## Audit Method

- Codebase inspected with `rg`, `cargo metadata`, `cargo check`, and targeted file reads.
- Parallel explorer passes audited all major subsystems across multiple waves.
- The 2026-07-08 re-audit ran ~37 parallel subagents across three waves.
- The 2026-07-09/10 backlog expansion materialized 48 epics with executable task files.
- Historical batches 1-10 (2026-07 through 2026-08-13) ran ~230+ agents producing
  ~130+ confirmed-done tasks; later reconciliation is reflected in the canonical ledger.
- Every one of the 35 workspace members has coverage in this pack.
