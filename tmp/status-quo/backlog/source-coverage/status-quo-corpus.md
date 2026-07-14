# Status-Quo Corpus Source Coverage

Purpose: coverage ledger for direct markdown files under `tmp/status-quo/*.md`, excluding `tmp/status-quo/backlog/**`. Each row maps one source document to an executable reconciliation task in `tmp/status-quo/backlog/plans/DOC-status-quo-corpus/tasks.toml`.

Future agents running these tasks should compare the source document against the current E01-E18 executable backlog. If the source exposes an uncovered actionable finding, add or refine downstream tasks under `tmp/status-quo/backlog/plans/`. If the source is already covered, stale, intentionally deferred, or non-actionable, replace the placeholder with a concrete no-op mapping and evidence.

## Summary

- Source corpus: 108 direct markdown files.
- Executable reconciliation tasks: 12.
- Coverage status: all sources are queued for reconciliation.
- Backlog subtree excluded: `tmp/status-quo/backlog/**`.

## Ledger

### DOC-SQ-01 - Navigation And Roadmap

| Source | Downstream scan focus | Coverage task | Ledger state |
|---|---|---|---|
| `tmp/status-quo/00-INDEX.md` | E01-E18 global navigation/source priority | DOC-SQ-01 | queued for reconciliation |
| `tmp/status-quo/01-EXECUTIVE-SUMMARY.md` | E01-E18 global summary issues | DOC-SQ-01 | queued for reconciliation |
| `tmp/status-quo/12-ROADMAP.md` | E01-E18 roadmap and proof gates | DOC-SQ-01 | queued for reconciliation |
| `tmp/status-quo/13-CURRENT-STATE-MATRIX.md` | E01-E18 state matrix gaps | DOC-SQ-01 | queued for reconciliation |
| `tmp/status-quo/24-OPEN-ISSUE-LEDGER.md` | E01-E18 P0/P1/P2 issue coverage | DOC-SQ-01 | queued for reconciliation |
| `tmp/status-quo/25-PROOF-GATES.md` | E01-E18 verify/proof command coverage | DOC-SQ-01 | queued for reconciliation |
| `tmp/status-quo/26-CANONICAL-DECISIONS.md` | E01-E18 decision coverage | DOC-SQ-01 | queued for reconciliation |
| `tmp/status-quo/27-IMPLEMENTATION-BACKLOG.md` | E01-E18 backlog parity | DOC-SQ-01 | queued for reconciliation |
| `tmp/status-quo/28-DEFINITION-OF-DONE.md` | E01-E18 done criteria | DOC-SQ-01 | queued for reconciliation |
| `tmp/status-quo/29-RISK-REGISTER.md` | E01-E18 risk mitigation coverage | DOC-SQ-01 | queued for reconciliation |
| `tmp/status-quo/DOC-MANIFEST.md` | E01-E18 corpus taxonomy and consolidation | DOC-SQ-01 | queued for reconciliation |

### DOC-SQ-02 - Spec Coverage

| Source | Downstream scan focus | Coverage task | Ledger state |
|---|---|---|---|
| `tmp/status-quo/02-SPEC-EVOLUTION.md` | E13/E18 spec-history drift | DOC-SQ-02 | queued for reconciliation |
| `tmp/status-quo/14-V1-COVERAGE.md` | E13/E18 v1 coverage debt | DOC-SQ-02 | queued for reconciliation |
| `tmp/status-quo/15-V2-COVERAGE.md` | E13/E18 v2 summary coverage debt | DOC-SQ-02 | queued for reconciliation |
| `tmp/status-quo/18-V2-DEPTH-COVERAGE.md` | E13/E18 v2-depth coverage debt | DOC-SQ-02 | queued for reconciliation |
| `tmp/status-quo/72-SOURCE-DOC-COVERAGE-LEDGER.md` | E18 source-doc coverage gaps | DOC-SQ-02 | queued for reconciliation |
| `tmp/status-quo/78-V2-DEPTH-RESEARCH-PROMPT-LEDGER.md` | E13/E18 research-prompt fencing | DOC-SQ-02 | queued for reconciliation |
| `tmp/status-quo/79-REFERENCE-PROVENANCE-LEDGER.md` | E18 reference provenance rules | DOC-SQ-02 | queued for reconciliation |
| `tmp/status-quo/80-SOURCE-DOC-MANIFEST.md` | E18 source manifest ownership | DOC-SQ-02 | queued for reconciliation |
| `tmp/status-quo/85-V2-COVERAGE-KERNEL.md` | E03/E13 kernel spec coverage | DOC-SQ-02 | queued for reconciliation |
| `tmp/status-quo/86-V2-COVERAGE-PLATFORM.md` | E02/E09/E13 platform spec coverage | DOC-SQ-02 | queued for reconciliation |
| `tmp/status-quo/87-V2-COVERAGE-ECOSYSTEM.md` | E10/E11/E13 ecosystem spec coverage | DOC-SQ-02 | queued for reconciliation |
| `tmp/status-quo/102-SPEC-DEBT-LEDGER.md` | E13 concept-level spec debt | DOC-SQ-02 | queued for reconciliation |

### DOC-SQ-03 - Census And Type Cleanup

| Source | Downstream scan focus | Coverage task | Ledger state |
|---|---|---|---|
| `tmp/status-quo/03-CRATE-AUDIT.md` | E03/E12/E18 crate audit coverage | DOC-SQ-03 | queued for reconciliation |
| `tmp/status-quo/06-WIRING-STATUS.md` | E01-E18 built-but-unwired gaps | DOC-SQ-03 | queued for reconciliation |
| `tmp/status-quo/11-DEPENDENCY-GRAPH.md` | E12/E18 dependency and layering gaps | DOC-SQ-03 | queued for reconciliation |
| `tmp/status-quo/16-CODEBASE-INVENTORY.md` | E01-E18 inventory-derived gaps | DOC-SQ-03 | queued for reconciliation |
| `tmp/status-quo/47-FOUNDATION-TYPES-REDESIGN.md` | E03 foundation type coverage | DOC-SQ-03 | queued for reconciliation |
| `tmp/status-quo/54-PER-CRATE-MIGRATION-CHECKLIST.md` | E03/E12 per-crate migration coverage | DOC-SQ-03 | queued for reconciliation |
| `tmp/status-quo/103-DUPLICATE-TYPES-CENSUS.md` | E03 duplicate type coverage | DOC-SQ-03 | queued for reconciliation |
| `tmp/status-quo/104-DEAD-CODE-AND-FACADE-CENSUS.md` | E12 dead code and facade coverage | DOC-SQ-03 | queued for reconciliation |

### DOC-SQ-04 - Execution And Self-Hosting

| Source | Downstream scan focus | Coverage task | Ledger state |
|---|---|---|---|
| `tmp/status-quo/23-TASKRUNNER-MIGRATION-STATUS.md` | E01/E16 runner lineage gaps | DOC-SQ-04 | queued for reconciliation |
| `tmp/status-quo/36-ORCHESTRATION-RUNNERS.md` | E01/E05/E08 runner handoff gaps | DOC-SQ-04 | queued for reconciliation |
| `tmp/status-quo/37-RUNNER-V2-AND-GRAPH.md` | E01 engine-selection coverage | DOC-SQ-04 | queued for reconciliation |
| `tmp/status-quo/91-PRD-RESEARCH.md` | E16 PRD/research pipeline coverage | DOC-SQ-04 | queued for reconciliation |
| `tmp/status-quo/95-ENGINE-DRIFT.md` | E01/E18 engine drift coverage | DOC-SQ-04 | queued for reconciliation |
| `tmp/status-quo/96-TRACE-RUNNER-V2-EXECUTION.md` | E01 runner-v2 trace coverage | DOC-SQ-04 | queued for reconciliation |
| `tmp/status-quo/98-TRACE-SELF-HOSTING-LOOP.md` | E16 self-hosting loop coverage | DOC-SQ-04 | queued for reconciliation |
| `tmp/status-quo/101-TRACE-GATE-PIPELINE.md` | E05 gate pipeline coverage | DOC-SQ-04 | queued for reconciliation |

### DOC-SQ-05 - Core Substrate

| Source | Downstream scan focus | Coverage task | Ledger state |
|---|---|---|---|
| `tmp/status-quo/30-CORE-SIGNAL.md` | E03/E12 core noun and trait coverage | DOC-SQ-05 | queued for reconciliation |
| `tmp/status-quo/31-GRAPH-CELLS-ENGINE.md` | E01/E03 graph and cell coverage | DOC-SQ-05 | queued for reconciliation |
| `tmp/status-quo/32-EVENTS-BUS-STATEHUB.md` | E02/E08 event and StateHub coverage | DOC-SQ-05 | queued for reconciliation |
| `tmp/status-quo/49-INDEX-LANG.md` | E12/E13 index and parser coverage | DOC-SQ-05 | queued for reconciliation |
| `tmp/status-quo/52-PLUGIN-EXTENSIONS.md` | E14/E18 plugin and extension coverage | DOC-SQ-05 | queued for reconciliation |
| `tmp/status-quo/56-DAIMON.md` | E02/E08 daimon state coverage | DOC-SQ-05 | queued for reconciliation |
| `tmp/status-quo/88-CONDUCTOR.md` | E08 conductor coverage | DOC-SQ-05 | queued for reconciliation |
| `tmp/status-quo/89-PRIMITIVES-HDC.md` | E07/E12 HDC primitive coverage | DOC-SQ-05 | queued for reconciliation |
| `tmp/status-quo/90-RUNTIME-FS-STD.md` | E02/E14 runtime/fs/std coverage | DOC-SQ-05 | queued for reconciliation |
| `tmp/status-quo/92-RUNNER-V2-MODULE-FAMILY.md` | E01/E08 runner module coverage | DOC-SQ-05 | queued for reconciliation |

### DOC-SQ-06 - Agent, Safety, Provider, ACP, MCP

| Source | Downstream scan focus | Coverage task | Ledger state |
|---|---|---|---|
| `tmp/status-quo/33-AGENT-SAFETY.md` | E04 agent safety coverage | DOC-SQ-06 | queued for reconciliation |
| `tmp/status-quo/34-COMPOSE-PROMPTS.md` | E06 prompt composition coverage | DOC-SQ-06 | queued for reconciliation |
| `tmp/status-quo/35-GATES-VERIFICATION.md` | E05 gate verification coverage | DOC-SQ-06 | queued for reconciliation |
| `tmp/status-quo/38-AGENT-PROVIDERS-TOOLS.md` | E14 provider/tool coverage | DOC-SQ-06 | queued for reconciliation |
| `tmp/status-quo/44-AGENT-SERVER.md` | E10/E14 agent-server coverage | DOC-SQ-06 | queued for reconciliation |
| `tmp/status-quo/48-MCP-CRATES.md` | E15 MCP config/tool coverage | DOC-SQ-06 | queued for reconciliation |
| `tmp/status-quo/51-ACP.md` | E17 ACP completion coverage | DOC-SQ-06 | queued for reconciliation |
| `tmp/status-quo/75-SECURITY-AUTH-SCOPE-MATRIX.md` | E04/E17 trust-boundary coverage | DOC-SQ-06 | queued for reconciliation |
| `tmp/status-quo/99-TRACE-AGENT-TURN.md` | E04/E14 agent-turn trace coverage | DOC-SQ-06 | queued for reconciliation |
| `tmp/status-quo/100-TRACE-ACP-SESSION.md` | E17 ACP session trace coverage | DOC-SQ-06 | queued for reconciliation |

### DOC-SQ-07 - Learning And Observability

| Source | Downstream scan focus | Coverage task | Ledger state |
|---|---|---|---|
| `tmp/status-quo/39-NEURO-KNOWLEDGE.md` | E07 knowledge-store coverage | DOC-SQ-07 | queued for reconciliation |
| `tmp/status-quo/40-LEARNING-TELEMETRY.md` | E07/E09 learning and telemetry coverage | DOC-SQ-07 | queued for reconciliation |
| `tmp/status-quo/41-DREAMS.md` | E07/E13 dream/replay coverage | DOC-SQ-07 | queued for reconciliation |
| `tmp/status-quo/53-OBSERVABILITY.md` | E09/E13 observability coverage | DOC-SQ-07 | queued for reconciliation |
| `tmp/status-quo/64-PARITY-TEST-MATRIX.md` | E10/E16/E17 parity test coverage | DOC-SQ-07 | queued for reconciliation |
| `tmp/status-quo/74-TEST-AND-PROOF-INVENTORY.md` | E01-E18 proof inventory coverage | DOC-SQ-07 | queued for reconciliation |

### DOC-SQ-08 - Chain, ISFR, Apps, Demo

| Source | Downstream scan focus | Coverage task | Ledger state |
|---|---|---|---|
| `tmp/status-quo/42-CHAIN-REGISTRIES-ISFR.md` | E11/E14 chain and ISFR coverage | DOC-SQ-08 | queued for reconciliation |
| `tmp/status-quo/93-ROKO-DEMO.md` | E10/E11 demo coverage | DOC-SQ-08 | queued for reconciliation |
| `tmp/status-quo/94-FEED-AGENTS-FLEET.md` | E02/E09 feed-agent coverage | DOC-SQ-08 | queued for reconciliation |
| `tmp/status-quo/106-APPS-MIRAGE-RELAY-WATCHER.md` | E10/E11/E18 app coverage | DOC-SQ-08 | queued for reconciliation |

### DOC-SQ-09 - Surfaces, API, CLI, Data Contracts

| Source | Downstream scan focus | Coverage task | Ledger state |
|---|---|---|---|
| `tmp/status-quo/43-SURFACES-DEMO-UX.md` | E10 surface and UX coverage | DOC-SQ-09 | queued for reconciliation |
| `tmp/status-quo/45-CLI-SURFACE.md` | E01/E18 CLI surface coverage | DOC-SQ-09 | queued for reconciliation |
| `tmp/status-quo/46-SERVE-HTTP-REALTIME.md` | E04/E10 serve/realtime coverage | DOC-SQ-09 | queued for reconciliation |
| `tmp/status-quo/59-API-ROUTE-LEDGER.md` | E10 route parity coverage | DOC-SQ-09 | queued for reconciliation |
| `tmp/status-quo/62-CLI-COMMAND-LEDGER.md` | E01/E18 command coverage | DOC-SQ-09 | queued for reconciliation |
| `tmp/status-quo/66-FRONTEND-API-PARITY.md` | E10 frontend/API parity coverage | DOC-SQ-09 | queued for reconciliation |
| `tmp/status-quo/76-DATA-CONTRACTS-SCHEMAS.md` | E10/E17 data-contract coverage | DOC-SQ-09 | queued for reconciliation |
| `tmp/status-quo/97-TRACE-SERVE-LIFECYCLE.md` | E02/E09/E10 serve trace coverage | DOC-SQ-09 | queued for reconciliation |
| `tmp/status-quo/105-FRONTEND-DEMO-APP.md` | E10 frontend deep-dive coverage | DOC-SQ-09 | queued for reconciliation |

### DOC-SQ-10 - Persistence, Config, CI, Deploy, Ops

| Source | Downstream scan focus | Coverage task | Ledger state |
|---|---|---|---|
| `tmp/status-quo/09-DEPLOYMENT-STATUS.md` | E18 deployment coverage | DOC-SQ-10 | queued for reconciliation |
| `tmp/status-quo/50-QUALITY-CI-RELEASE.md` | E18 CI/release coverage | DOC-SQ-10 | queued for reconciliation |
| `tmp/status-quo/55-DATA-DIR.md` | E02 data-dir coverage | DOC-SQ-10 | queued for reconciliation |
| `tmp/status-quo/57-CONFIG.md` | E18 config coverage | DOC-SQ-10 | queued for reconciliation |
| `tmp/status-quo/58-JOBS-DEPLOY.md` | E18 jobs/deploy coverage | DOC-SQ-10 | queued for reconciliation |
| `tmp/status-quo/60-STATE-PERSISTENCE-LEDGER.md` | E02 persistence coverage | DOC-SQ-10 | queued for reconciliation |
| `tmp/status-quo/61-CONFIG-ENV-MATRIX.md` | E18 config/env coverage | DOC-SQ-10 | queued for reconciliation |
| `tmp/status-quo/71-CI-RELEASE-PROOF-GAPS.md` | E18 CI proof coverage | DOC-SQ-10 | queued for reconciliation |
| `tmp/status-quo/73-EXAMPLES-PLANS-GRAPHS.md` | E01/E16 example/plan/graph coverage | DOC-SQ-10 | queued for reconciliation |
| `tmp/status-quo/77-OPERATIONS-DEPLOY-RUNBOOK.md` | E18 ops/deploy coverage | DOC-SQ-10 | queued for reconciliation |
| `tmp/status-quo/83-ENV-VAR-MANIFEST.md` | E18 env-var coverage | DOC-SQ-10 | queued for reconciliation |

### DOC-SQ-11 - Docs, Migration, Debt

| Source | Downstream scan focus | Coverage task | Ledger state |
|---|---|---|---|
| `tmp/status-quo/04-NAMING-MIGRATION.md` | E03/E18 naming migration coverage | DOC-SQ-11 | queued for reconciliation |
| `tmp/status-quo/05-ARCHITECTURE-REALITY.md` | E01/E03 architecture-reality coverage | DOC-SQ-11 | queued for reconciliation |
| `tmp/status-quo/07-MIGRATION-CHECKLIST.md` | E12/E18 migration checklist coverage | DOC-SQ-11 | queued for reconciliation |
| `tmp/status-quo/08-TECH-DEBT.md` | E12/E18 tech-debt coverage | DOC-SQ-11 | queued for reconciliation |
| `tmp/status-quo/10-TESTING-STATUS.md` | E16/E18 testing-status coverage | DOC-SQ-11 | queued for reconciliation |
| `tmp/status-quo/19-DOC-DRIFT-REGISTER.md` | E18 doc-drift coverage | DOC-SQ-11 | queued for reconciliation |
| `tmp/status-quo/63-DELETE-ARCHIVE-PLAN.md` | E12 deletion/archive coverage | DOC-SQ-11 | queued for reconciliation |
| `tmp/status-quo/65-DOCS-CONVERGENCE-PLAN.md` | E18 docs convergence coverage | DOC-SQ-11 | queued for reconciliation |
| `tmp/status-quo/81-ROOT-DOCS-REWRITE-QUEUE.md` | E18 root docs rewrite coverage | DOC-SQ-11 | queued for reconciliation |
| `tmp/status-quo/82-COMMAND-EXAMPLE-DRIFT-LEDGER.md` | E18 command example coverage | DOC-SQ-11 | queued for reconciliation |
| `tmp/status-quo/84-STATUS-PACK-MAINTENANCE.md` | E18 status-pack maintenance coverage | DOC-SQ-11 | queued for reconciliation |

### DOC-SQ-12 - Tmp Archaeology

| Source | Downstream scan focus | Coverage task | Ledger state |
|---|---|---|---|
| `tmp/status-quo/17-TMP-SOURCE-RANKING.md` | E01-E18 tmp source-priority coverage | DOC-SQ-12 | queued for reconciliation |
| `tmp/status-quo/20-TMP-NEWEST.md` | E04/E10/E14 newest tmp gap coverage | DOC-SQ-12 | queued for reconciliation |
| `tmp/status-quo/21-TMP-MAY-BATCH.md` | E01-E18 May batch reconciliation | DOC-SQ-12 | queued for reconciliation |
| `tmp/status-quo/22-TMP-LEGACY.md` | E01-E18 legacy tmp reconciliation | DOC-SQ-12 | queued for reconciliation |
| `tmp/status-quo/67-TMP-FEEDBACK-2-CROSSWALK.md` | E04/E10/E14 dogfood feedback coverage | DOC-SQ-12 | queued for reconciliation |
| `tmp/status-quo/68-SELF-DEVELOPING-CROSSWALK.md` | E16/E17 self-developing UX coverage | DOC-SQ-12 | queued for reconciliation |
| `tmp/status-quo/69-RESIDUAL-AUDIT-TRACKER.md` | E01-E18 residual audit coverage | DOC-SQ-12 | queued for reconciliation |
| `tmp/status-quo/70-RELAY-PROTOCOL-FREEZE.md` | E10/E17 relay protocol coverage | DOC-SQ-12 | queued for reconciliation |
