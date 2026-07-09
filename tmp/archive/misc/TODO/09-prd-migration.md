# prd-migration/ — Documentation Migration

**Directory**: `tmp/prd-migration/`
**Status**: DONE — 22/22 topics generated, 422 files, 190,309 lines
**Executed**: 2026-04-11/12

## What This Was

A full documentation regeneration pipeline that consolidated ~600 legacy source documents (bardo-backup PRDs, research, implementation plans, crate source) into a clean Roko documentation set at `docs/`.

## Results

| Metric | Value |
|--------|-------|
| Topics generated | 22 / 22 |
| Result codes | 18 success + 4 success_warnings |
| Output files | 422 markdown files |
| Total lines | 190,309 |
| Naming applied | 100% (Bardo->Roko, Golem->Agent, Grimoire->Neuro, etc.) |
| Citations preserved | All academic citations from legacy sources |

### Topics Covered

00-architecture through 21-references (complete coverage of all subsystems)

### Warning Topics (Minor, Non-Blocking)

- 02-agents — possibly sparse sub-doc count
- 07-conductor — newer subsystem, sparse legacy sources
- 18-tools — more reference-like, citation density
- 19-deployment — some sections marked future refinements

## Migration Infrastructure

Reusable pipeline at `tmp/prd-migration/`:
- `run-migration.sh` — main orchestrator
- `lib/common.sh`, `lib/spawn.sh`, `lib/verify.sh` — utilities
- `context-pack/` — 7-file context injected into every agent
- `prompts/` — 22 per-topic prompts
- `verify/` — quality check scripts
- `logs/run-*` — timestamped execution logs

Can re-run individual topics: `./run-migration.sh --only 02-agents --force`

## What's NOT Wired (Separate from Migration)

These are roadmap items T10 and T11 from CLAUDE.md, not migration gaps:

- [ ] Auto-generate plans when PRD is published (`roko prd plan`)
- [ ] Failed task gates feed back into plan regeneration

## No Remaining Action on Migration Itself

The migration is complete. All 422 files live at `docs/`. The pipeline is preserved for future re-runs.

## Source Files

- **Runner**: `tmp/prd-migration/run-migration.sh`
- **Context pack**: `tmp/prd-migration/context-pack/`
- **Per-topic prompts**: `tmp/prd-migration/prompts/`
- **Execution logs**: `tmp/prd-migration/logs/`
- **Output**: `docs/` (422 files)
