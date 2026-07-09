# Roko PRD Integration & Refactoring Plan

## Goal

Three-phase approach to aligning the roko codebase with the PRD docs at `/docs/`:

1. **Phase A — Mechanical Refactor**: Rename crates, dissolve roko-golem, update metadata. No semantic changes — just alignment of names and structure.
2. **Phase B — Structural Refactor**: Extend the remaining core types (attestation, emotional tags), wire dormant subsystems (safety, conductor, neuro, daimon), and fill in missing algorithms and integrations that the docs specify for existing crates. The 7-axis score and knowledge-tier work are already in the branch.
3. **Phase C — New Features**: Build entirely new subsystems (pheromone field, agent mesh, heartbeat theta/delta, T0 probes, dreams consolidation, agent composition) using roko's self-development pipeline.

## Scale

The docs describe **~42 distinct implementation gaps beyond naming changes**:
- 5 naming/mechanical changes
- 2 core type extensions still open (attestation, emotional tags)
- 12 missing algorithms (CPM, task fusion, NREM replay, EWC, curriculum learning, etc.)
- 6 missing subsystems (pheromone field, agent mesh, morphogenetics, T0 probes, heartbeat, adaptive clock)
- 8 missing integrations (safety→orchestrator, daimon→router, neuro→compose, etc.)
- 5 missing agent patterns (composition, introspection, metamorphosis, OCaps, supervision)
- 4 missing infrastructure (HTTP LlmBackends, daemon mode, WASM, lifecycle mgmt)

## Why this order matters

Phase A must come first because the docs reference `roko-runtime`, `roko-primitives`, `Engram` — names that don't exist in code yet. Fresh agents would be confused by the mismatch.

Phase B must come before Phase C because the new subsystems (pheromone field, dreams, mesh) depend on the remaining type extensions and integrations from Phase B (attestation, emotional tags, affect integration, lineage tracking).

Phase C items are largely independent of each other and can be parallelized.

## Document Index

| Doc | What |
|-----|------|
| [01-CURRENT-STATE.md](01-CURRENT-STATE.md) | Inventory of docs vs code gaps |
| [02-NAMING-CHANGES.md](02-NAMING-CHANGES.md) | Complete naming migration map |
| [03-REFACTOR-SEQUENCE.md](03-REFACTOR-SEQUENCE.md) | Ordered refactoring steps (Phase A) |
| [04-PLAN-GENERATION-STRATEGY.md](04-PLAN-GENERATION-STRATEGY.md) | How to generate plans with sufficient context for fresh agents |
| [05-SELF-DEV-PIPELINE-STATUS.md](05-SELF-DEV-PIPELINE-STATUS.md) | Bugs found, fixes applied, what works now |
| [06-BUILD-SEQUENCE.md](06-BUILD-SEQUENCE.md) | Feature build order from PRDs (Phase C) |
| [07-TASK-FORMAT-GUIDE.md](07-TASK-FORMAT-GUIDE.md) | How to write tasks that fresh agents can implement |
| [08-DEEP-ARCHITECTURAL-GAPS.md](08-DEEP-ARCHITECTURAL-GAPS.md) | **Full inventory of 44 structural gaps** — type extensions, missing algorithms, dormant integrations, new subsystems |
| [09-REFACTORING-PRD-ADDITIONS.md](09-REFACTORING-PRD-ADDITIONS.md) | Additions from `/refactoring-prd/` — implementation ordering, 8 feedback loops, frontier innovations, trait signatures |
| [10-MISSING-DOC-SECTIONS.md](10-MISSING-DOC-SECTIONS.md) | 11 doc sections not in original survey — composition, verification, conductor, chain, interfaces, identity-economy, code-intelligence, tools, deployment, technical-analysis, references |
| [11-CODEBASE-TODOS-AND-STUBS.md](11-CODEBASE-TODOS-AND-STUBS.md) | ~12,000 lines of dormant code — safety guards, conductor watchers, HDC features, executor config fields, serve stubs, learning feedback |
