# new-docs/ and new-docs-section-00/ — Documentation Trees

**Status**: Both DONE — comprehensive, accurate, current (2026-04-19)

## new-docs/ — Full Documentation Tree

**Directory**: `tmp/new-docs/`
**Files**: 554 markdown files, 63,135 lines, 3.5 MB
**Accuracy**: Current and accurate. All Shipping-tier claims verified against code.

### Structure

| Section | Files | Scope |
|---------|-------|-------|
| `reference/01-engram/` | 17 | Universal data type: struct, hashing, scoring, decay, lineage |
| `reference/02-pulse/` | 9 | Ephemeral event medium (planned; EventBus is live) |
| `reference/03-substrate/` | 16 | Storage trait: CRUD, similarity, backends |
| `reference/04-bus/` | 15 | Transport fabric (planned) |
| `reference/05-operators/` | ~60 | Six Synapse traits: Scorer, Gate, Router, Composer, Policy |
| `reference/06-loop/` | 16 | 8-step cognitive loop |
| `reference/07-speeds/` | 8 | Gamma/Theta/Delta cognitive speeds |
| `reference/08-layers/` | 10 | L0-L4 five-layer taxonomy |
| `reference/09-cross-cuts/` | 7 | Neuro, Daimon, Dreams |
| `reference/10-types/` | ~42 | Body, ContentHash, Decay, HDC, Kind, Provenance, Score |
| `00-architecture/` | ~50 | Vision, foundations, perspectives, innovations |
| `strategy/` | ~15 | Refactor phases, roadmap, refinements |
| `operations/` | ~35 | Configuration, error handling, performance |
| `testing/` | ~80 | Philosophy, tiers, 36 property tests, quality gates |
| `analysis/` | ~100 | Readiness audit, integration map, synergy map |
| `research/` | ~120 | Foundations, 8 innovations, 6 perspectives |

### Verified Against Codebase

- Engram struct in `crates/roko-core/src/engram.rs`
- Six Synapse traits in `crates/roko-core/src/traits.rs`
- FileSubstrate in `crates/roko-fs/src/file_substrate.rs`
- CascadeRouter in `crates/roko-learn/src/cascade_router.rs`
- SystemPromptBuilder in `crates/roko-compose/src/system_prompt_builder.rs`
- GatePipeline in `crates/roko-gate/src/gate_pipeline.rs`
- 35+ gate implementations in `crates/roko-gate/src/`

### Caveats

- Pulse/Bus described as target-state, not yet shipping (correctly labeled `[planned]`)
- Innovations are speculative (correctly labeled `[Speculative]`)
- Status snapshot dated 2026-04-19

---

## new-docs-section-00/ — Architecture Reference

**Directory**: `tmp/new-docs-section-00/`
**Files**: 549 files across ~40 directories
**Purpose**: Specification-grade architectural reference

Same structure and quality as `new-docs/` with additional:
- `readiness-audit/` — 21-section scorecard (6 criteria per section, 0-30 scale)
- `ALIASES.md` — Public vs. internal names
- `GLOSSARY.md` — 150+ terms with status tags
- `_migration/` — Audit trail from old docs

### Known Discrepancies

- Crate count: docs say 36, actual is 30 — needs reconciliation
- LOC count: docs say 322K, actual is ~440K — docs likely exclude generated code
- Some `[Built]` items may be `[Wired]` now; some `[Wired]` may be `[Partial]`

No remaining action items — these are reference artifacts.
