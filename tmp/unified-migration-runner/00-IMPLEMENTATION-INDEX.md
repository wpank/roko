# Implementation Plans — Index

> Each plan is a self-contained prompt that can be given to a fresh Claude session.
> Plans are independent unless noted. Execute in suggested order for best results.

## Plans

| # | Plan | File | Tasks | Est. | Dependencies |
|---|------|------|-------|------|-------------|
| 1 | **Plan Runner v2** | [RUNNER-V2-IMPLEMENTATION.md](RUNNER-V2-IMPLEMENTATION.md) | R001–R045 | 2-4d | None |
| 2 | **main.rs Decomposition** | [MAIN-RS-DECOMPOSITION.md](MAIN-RS-DECOMPOSITION.md) | M-D001–M-D012 | 1d | None |
| 3 | **Cascade Router Refactor** | [CASCADE-ROUTER-REFACTOR.md](CASCADE-ROUTER-REFACTOR.md) | CR001–CR007 | 1-2d | None |
| 4 | **Config Schema Decomposition** | [CONFIG-SCHEMA-DECOMPOSITION.md](CONFIG-SCHEMA-DECOMPOSITION.md) | CS001–CS009 | 1d | None |
| 5 | **Cell Trait + Protocol Renames** | [CELL-TRAIT-AND-RENAMES.md](CELL-TRAIT-AND-RENAMES.md) | CT001–CT012 | 2-3d | None (but do after #2,#3,#4 to minimize merge conflicts) |
| 6 | **Serve Routes Consolidation** | [SERVE-ROUTES-CONSOLIDATION.md](SERVE-ROUTES-CONSOLIDATION.md) | SR001–SR005 | 1d | None |
| 7 | **Demurrage + Tier Progression** | [DEMURRAGE-AND-TIERS.md](DEMURRAGE-AND-TIERS.md) | DT001–DT006 | 2d | #5 (Cell trait for protocol alignment) |

## Suggested Execution Order

```
Parallel Track A (critical path):     Parallel Track B (cleanup):
  1. Runner v2 (R001-R045)              2. main.rs decomposition
                                        3. cascade_router refactor
                                        4. config/schema decomposition
                                        6. serve routes consolidation

After both tracks:
  5. Cell trait + protocol renames (touches many files, do when codebase is clean)
  7. Demurrage + tiers (builds on Cell trait)
```

Track A and Track B can run simultaneously in separate sessions/branches.
Plans #5 and #7 should wait for #1-4 and #6 to merge first.

## Existing Migration Plans (from unified-migration/)

These are the broader Phase 0-3 migration tasks. They overlap with and are informed
by the plans above:

| Phase | File | Status |
|-------|------|--------|
| Phase 0 | `../unified-migration/01-PHASE-0-PREP.md` | Partially covered by plans #1, #5 |
| Phase 1 | `../unified-migration/02-PHASE-1-KERNEL.md` | Covered by plan #5 (renames) + #7 (demurrage) |
| Phase 2 | `../unified-migration/03-PHASE-2-ENGINE.md` | Future — depends on Graph implementation |
| Phase 3 | `../unified-migration/04-PHASE-3-ECONOMY.md` | Future — depends on Phase 1-2 |

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
