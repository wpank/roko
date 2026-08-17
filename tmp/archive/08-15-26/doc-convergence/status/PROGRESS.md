# Doc Convergence Progress

Last updated: 2026-08-12

## What is this?

This file tracks progress of the 5-phase convergence pipeline that reads all of roko's
scattered spec documents (docs/v1/, docs/v2/, docs/v2-depth/, tmp/prds/) and the actual
Rust code, then produces a single unified spec (docs/v3/). See `../README.md` for the
full explanation.

## Pre-convergence items now resolved

Before running the pipeline, several cross-cutting issues needed to be fixed. These are
now done:

| Item | Status | Notes |
|---|---|---|
| Engram -> Signal rename | **RESOLVED** | `pub type Signal = Engram` alias in `roko-core/src/engram.rs:226`, re-export in `roko-core/src/signal.rs`. All new code uses `Signal`. |
| docs/v2/ status markers | **RESOLVED** | All 29 chapters (01-SIGNAL through 28-ROADMAP) annotated with implementation status (2026-08-12). |
| docs/v1/ deprecation headers | **RESOLVED** | All 417 files have deprecation headers pointing to the corresponding docs/v2/ chapter (2026-08-12). |
| v1/v2 vocabulary mapping | **RESOLVED in docs** | The mapping (Engram->Signal, Substrate->Store, Gate->Verify, etc.) is documented in v2. Rust trait names still use v1 vocabulary -- this is intentional (renaming 18 crates is out of scope). |

## Pipeline Status

| Phase | Status | Output | Notes |
|---|---|---|---|
| 1. Build Matrix | PENDING | `status/MATRIX.md` | Single agent, ~10 min. Scans all doc sets + code to produce topic-by-topic inventory. |
| 2. Converge Topics | PENDING | `output/{NN}-{TOPIC}.md` | 28 parallel agents, ~30-60 min. One converged doc per topic. |
| 3. Synthesize | PENDING | `output/00-SYNTHESIS.md` | Single agent, ~15 min. Cross-topic analysis. |
| 4. Dogfood | PENDING | `.roko/prd/` + `plans/` | Single agent, ~10 min. Converts results into roko's own PRD/task system. |
| 5. Redesign | PENDING | `output/00-REDESIGN.md` | Single agent, ~20 min. Fresh architecture review informed by full convergence. |

## Topic Convergence Status (Phase 2)

These 28 topics map 1:1 to the docs/v2/ chapters. Phase 2 produces one converged doc
per topic by reading all four doc layers plus the actual Rust code.

| Topic | Status | Output File | Agent Log |
|---|---|---|---|
| 01-SIGNAL | PENDING | | |
| 02-CELL | PENDING | | |
| 03-GRAPH | PENDING | | |
| 04-EXECUTION | PENDING | | |
| 05-AGENT | PENDING | | |
| 06-MEMORY | PENDING | | |
| 07-LEARNING | PENDING | | |
| 08-GATEWAY | PENDING | | |
| 09-FEEDS | PENDING | | |
| 10-GROUPS | PENDING | | |
| 11-CONNECTIVITY | PENDING | | |
| 12-EXTENSIONS | PENDING | | |
| 13-TRIGGERS | PENDING | | |
| 14-TOOLS | PENDING | | |
| 15-TELEMETRY | PENDING | | |
| 16-SECURITY | PENDING | | |
| 17-AUTH | PENDING | | |
| 18-PAYMENTS | PENDING | | |
| 19-CONFIG | PENDING | | |
| 20-SURFACES | PENDING | | |
| 21-MARKETPLACE | PENDING | | |
| 22-REGISTRIES | PENDING | | |
| 23-ARENAS | PENDING | | |
| 24-DEFI | PENDING | | |
| 25-DEPLOYMENT | PENDING | | |
| 26-CROSS-CUTS | PENDING | | |
| 27-ORCHESTRATOR | PENDING | | |
| 28-ROADMAP | PENDING | | |

## Estimated Effort

- Phase 1: ~1 agent, 10 min
- Phase 2: ~28 agents (5 parallel batches of ~6), 30-60 min
- Phase 3: ~1 agent, 15 min
- Phase 4: ~1 agent, 10 min (writes files)
- Phase 5: ~1 agent, 20 min
- **Total: ~32 agent runs, ~90-120 min wall time**
