# Implementation Plans — Index

> **Last updated: 2026-08-13**

## What is this?

This directory contains concrete implementation plans for the mori-to-roko migration.
Each plan is a self-contained prompt that can be given to a fresh Claude session. The
companion directory `tmp/unified-migration/` has the high-level phase checklists
(Phase 0-3) that these plans execute against.

The `run.sh` script in this directory is a parallel-agent runner that can execute
multiple plans concurrently across 4 agents with crate-partitioned ownership.

> Each plan is a self-contained prompt that can be given to a fresh Claude session.
> Plans are independent unless noted. Execute in suggested order for best results.

## Plans

| # | Plan | File | Tasks | Est. | Status |
|---|------|------|-------|------|--------|
| 1 | **Plan Runner v2** | [RUNNER-V2-IMPLEMENTATION.md](RUNNER-V2-IMPLEMENTATION.md) | R001-R045 | 2-4d | **DONE** -- `crates/roko-cli/src/runner/event_loop.rs` is the production engine. `orchestrate.rs` superseded. |
| 2 | **main.rs Decomposition** | [MAIN-RS-DECOMPOSITION.md](MAIN-RS-DECOMPOSITION.md) | M-D001-M-D012 | 1d | **DONE** -- main.rs reduced from 12,690 to ~5,600 lines. `commands/` module directory created with plan, prd, agent, config, knowledge, learn, job, server submodules. |
| 3 | **Cascade Router Refactor** | [CASCADE-ROUTER-REFACTOR.md](CASCADE-ROUTER-REFACTOR.md) | CR001-CR007 | 1-2d | Pending |
| 4 | **Config Schema Decomposition** | [CONFIG-SCHEMA-DECOMPOSITION.md](CONFIG-SCHEMA-DECOMPOSITION.md) | CS001-CS009 | 1d | Pending |
| 5 | **Cell Trait + Protocol Renames** | [CELL-TRAIT-AND-RENAMES.md](CELL-TRAIT-AND-RENAMES.md) | CT001-CT012 | 2-3d | **Partial** -- `Engram`->`Signal` rename DONE (2026-08-12). Remaining 6 trait renames + Cell trait pending. |
| 6 | **Serve Routes Consolidation** | [SERVE-ROUTES-CONSOLIDATION.md](SERVE-ROUTES-CONSOLIDATION.md) | SR001-SR005 | 1d | **DONE** -- `status/` split into 8 subfiles (health, metrics, episodes, gates, dashboard, disk, helpers, mod). `learning/` split into 4 subfiles (router_state, experiments, helpers, mod). |
| 7 | **Demurrage + Tier Progression** | [DEMURRAGE-AND-TIERS.md](DEMURRAGE-AND-TIERS.md) | DT001-DT006 | 2d | Pending -- depends on #5 (Cell trait) |

## Execution Status

```
Parallel Track A (critical path):     Parallel Track B (cleanup):
  1. Runner v2 (R001-R045) ........... DONE
                                        2. main.rs decomposition .......... DONE
                                        3. cascade_router refactor ........ pending
                                        4. config/schema decomposition .... pending
                                        6. serve routes consolidation ..... DONE

After both tracks:
  5. Cell trait + protocol renames .... PARTIAL (Engram->Signal done, 6 trait renames pending)
  7. Demurrage + tiers ............... pending (blocked on #5)
```

Track A and Track B ran simultaneously. Plans #1, #2, and #6 are complete.
Plans #3 and #4 can proceed independently. Plans #5 and #7 should wait for #3-4.

## Existing Migration Plans (from unified-migration/)

These are the broader Phase 0-3 migration tasks. They overlap with and are informed
by the plans above:

| Phase | File | Status |
|-------|------|--------|
| Phase 0 | `../unified-migration/01-PHASE-0-PREP.md` | **Mostly DONE** -- runner v2, main.rs decomposition, serve routes all complete. Dead code wiring items remain. |
| Phase 1 | `../unified-migration/02-PHASE-1-KERNEL.md` | **Partial** -- Engram->Signal DONE (2026-08-12). Covered by plan #5 (remaining renames) + #7 (demurrage). |
| Phase 2 | `../unified-migration/03-PHASE-2-ENGINE.md` | Future -- depends on Phase 1 completion (Graph engine, agent runtime) |
| Phase 3 | `../unified-migration/04-PHASE-3-ECONOMY.md` | Future -- depends on Phases 1-2 (L4 self-evolution, CaMeL IFC, on-chain) |

## Context Files (for agent prompts)

These files are in `context-pack/` and should be loaded by any agent working on these plans:

| File | Purpose |
|------|---------|
| `context-pack/01-orientation.md` | Project overview |
| `context-pack/02-vocabulary.md` | Naming conventions |
| `context-pack/03-migration-rules.md` | What to change, what not to change |
| `context-pack/04-coding-conventions.md` | Rust style, commit messages |
| `context-pack/05-verification-gates.md` | cargo check/test/clippy/fmt |

## Dogfood Reference

All known issues tracked in `../dogfood/00-INDEX.md`. Plans above resolve:
- Runner v2 (#1): fixes streaming, persistence, enrichment, TUI, model display
- Serve routes (#6): fixes missing endpoints
- Cell + renames (#5): aligns with unified spec
- Demurrage (#7): enables knowledge management
