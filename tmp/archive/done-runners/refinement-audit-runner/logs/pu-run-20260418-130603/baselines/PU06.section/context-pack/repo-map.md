# Repo Map — Shared Neuro Context

Quick reference for agents working on `06` neuro parity.

## Workspace Root

`/Users/will/dev/nunchi/roko/roko/`

## High-Value Paths

| What | Path | Why It Matters In Batch 06 |
|------|------|----------------------------|
| Neuro types and store trait | `crates/roko-neuro/src/lib.rs` | canonical knowledge model, tiers, store contract |
| Neuro persistence and query path | `crates/roko-neuro/src/knowledge_store.rs` | ingest, decay, GC, query scoring, stats, confirmations |
| Neuro HDC encoder | `crates/roko-neuro/src/hdc.rs` | typed HDC encoding and future cross-domain prerequisites |
| Distillation and tier progression | `crates/roko-neuro/src/distiller.rs`, `tier_progression.rs` | real D1/D2/D3 contract and missing guards |
| Context assembler | `crates/roko-neuro/src/context.rs` | strongest dormant runtime seam |
| HDC primitives | `crates/roko-primitives/src/hdc.rs` | core vector math and any threshold/helpers work |
| Code-symbol HDC fingerprints | `crates/roko-index/src/hdc.rs` | adjacent HDC contract and drift from docs |
| Somatic layer | `crates/roko-daimon/src/lib.rs` | PAD / strategy-space inputs already used by neuro context |
| Dream-cycle integration | `crates/roko-dreams/src/cycle.rs` | real distillation and cross-domain-adjacent runtime |
| Chain witness | `crates/roko-chain/src/witness.rs` | attestation-only reality vs publish/token docs |
| Main orchestrator | `crates/roko-cli/src/orchestrate.rs` | current direct query callers and tier-feedback paths |
| CLI surface | `crates/roko-cli/src/main.rs` | `NeuroCmd` gap for backup/restore/publish |
| Compose-side pheromone surface | `crates/roko-compose/src/context_provider.rs` | doc-16 truth-in-advertising correction |
| Neuro docs | `docs/06-neuro/` | source material being checked |
| Parity batch | `tmp/docs-parity/06/` | execution contract and findings |

## Important Corrections

Use these instead of older or misleading assumptions:

- `ContextAssembler` is implemented but still not on the main production path.
- Dreams-side cross-domain strategy hypotheses do not mean doc-08 resonance transfer is implemented.
- the current neuro CLI is query/stats/gc only.
- `roko-golem` is code-gone but still present in some docs.
- `Kind::Pheromone` and compose-side pheromone context are real, even though some status docs still talk as if they are only designed.

## Search Priorities

Before editing, search these first:

```bash
rg -n "ContextAssembler::new|\\.gather\\(|query_kind\\(|query\\(" crates/roko-cli crates/roko-neuro
rg -n "KnowledgeStats|KnowledgeConfirmationRecord|CONFIRMATION_BOOST|min_similarity|DEFAULT_MIN_SUPPORT" crates/roko-neuro crates/roko-cli
rg -n "spawn_episode_distillation|write_playbook|extract_warnings|cross_validation|anti_knowledge|DistillationScheduler" crates/roko-neuro crates/roko-dreams crates/roko-cli
rg -n "KnowledgeSource|BackupManifest|enum NeuroCmd|MeshSync|KoraiChannel|LetheChannel|quarantine|sandbox|publish" crates docs/06-neuro tmp/docs-parity/06
rg -n "roko-golem|Fact|FACT_HALF_LIFE_DAYS|KnowledgeCrystal|Pheromone|Dreams cycle|cross-domain transfer" docs CLAUDE.md tmp/docs-parity/06
```

## Build Commands

```bash
cargo build --workspace
cargo test --workspace
cargo clippy --workspace --no-deps -- -D warnings
```

## Practical Rules

1. Activate current neuro runtime seams before adding new neuro systems.
2. Prefer one canonical query/distillation contract over several partial stories.
3. If a batch only proves one production path, make that path explicit and testable.
4. If a task really belongs to network, token, or frontier work, record the handoff and stop.
