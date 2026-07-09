# SOURCE-INDEX — Code Anchors for 06-Neuro Parity

Verified code references for batch `06`, organized around the runtime seams an agent is most likely to touch.

Generated: 2026-04-16

---

## Important Corrections First

Use these before trusting the docs literally:

- `ContextAssembler` is fully implemented in `crates/roko-neuro/src/context.rs`, but `rg -n "ContextAssembler::new|\\.gather\\(" crates/` only finds internal and test-side uses. The main orchestrator still bypasses it.
- `KnowledgeStore::query(...)` and `query_kind(...)` are real production retrieval paths today; the docs overstate the assembler’s live integration.
- doc `08` opens as if cross-domain transfer is implemented, but `Resonance`, `TransferRisk`, `DomainProfile`, `AnalogyResult`, and `ConfirmationTracker` are grep-negative in `crates/`.
- `DreamCycle` does generate cross-domain strategy hypotheses, but that is not the same system as doc-08 resonance transfer.
- `NeuroCmd` currently exposes only `Query`, `Stats`, and `Gc`; there is no backup/restore/publish CLI.
- `roko-golem` is gone from the workspace, but `CLAUDE.md` and some older docs still mention it.

---

## crates/roko-neuro/src/

### Core knowledge model

| File | What | Section |
|------|------|---------|
| `lib.rs:57-126` | `KnowledgeKind`, legacy serde aliases, and tier-related types | A.02-A.06 |
| `lib.rs:186-244` | `KnowledgeEntry` current field surface | A.03, A.16 |
| `lib.rs:246-290` | `refutation_warning`, tier helpers, half-life composition | A.05-A.08 |
| `lib.rs:292-344` | `NeuroStore` trait and query/ingest contract | C.01 |

### Store and retrieval path

| File | What | Section |
|------|------|---------|
| `knowledge_store.rs:28-44` | confirmation boost, defaults, and core constants | A.11-A.13, C.17 |
| `knowledge_store.rs:78-122` | `KnowledgeStats`, `KnowledgeConfirmationRecord`, and related struct shapes | C.04-C.05 |
| `knowledge_store.rs:264-340` | append-only load/save and store initialization | C.02 |
| `knowledge_store.rs:498-747` | ingest path, inferred tiering, and source-field handling | A.22, E.11 |
| `knowledge_store.rs:852-946` | decay and GC behavior including AntiKnowledge exemptions | A.10-A.13 |
| `knowledge_store.rs:948-1127` | query and `query_kind` scoring path | C.03, B.15 |

### HDC encoder and advanced hooks

| File | What | Section |
|------|------|---------|
| `hdc.rs:12-113` | `CausalLinkParts` directional HDC encoding | A.21, F.12 |
| `hdc.rs:115-255` | `KnowledgeHdcEncoder` core encode/query helpers | B.12, C.20 |

### Distillation and progression

| File | What | Section |
|------|------|---------|
| `distiller.rs:23-94` | `DistillationBackend` trait and concrete constructor surface | D.04-D.05, C.18 |
| `distiller.rs:96-286` | episode-to-insight distillation path | D.01, D.09 |
| `tier_progression.rs:24-31` | default thresholds including support/confidence | D.02, C.19 |
| `tier_progression.rs:33-122` | `TierProgression` struct and helpers | D.06 |
| `tier_progression.rs:124-146` | `TierProgressionDecision` variants | D.07 |
| `tier_progression.rs:166-410` | heuristic promotion, playbook writing, and analysis path | D.02-D.03, D.13 |

### Context assembler

| File | What | Section |
|------|------|---------|
| `context.rs:28-73` | assembler constants and budget policy | C.08-C.10 |
| `context.rs:111-303` | `ContextAssembler` type and gather pipeline | C.07 |
| `context.rs:305-699` | ranking, marginal-value pruning, and compression | C.09 |
| `context.rs:701-862` | somatic/PAD bias integration helpers | C.10, E.10 |

---

## crates/roko-primitives/src/

### HDC core

| File | What | Section |
|------|------|---------|
| `hdc.rs:25-31` | `HdcVector { bits: [u64; 160] }` dimension | B.01, B.23 |
| `hdc.rs:52-140` | zero/random/seed construction | B.02 |
| `hdc.rs:142-193` | `bind` / `bundle` / `permute` | B.03-B.05 |
| `hdc.rs:195-233` | similarity and archived similarity | B.06-B.07 |
| `hdc.rs:235-320` | byte conversion, helpers, and tests | B.16 |

---

## crates/roko-index/src/

### Symbol fingerprinting

| File | What | Section |
|------|------|---------|
| `hdc.rs:1-191` | code-symbol HDC fingerprints and naming drift from docs | B.17-B.18 |

---

## crates/roko-daimon/src/

### Somatic and strategy-space surfaces

| File | What | Section |
|------|------|---------|
| `lib.rs:269-412` | `StrategyCoordinates`, `DispatchStrategy`, and coding-space defaults | E.01-E.09 |
| `lib.rs:414-621` | `SomaticMarker` and marker lookup surface | E.02-E.04 |
| `lib.rs:832-1076` | `SomaticLandscape` and contrarian-related constants | E.01, E.06 |

---

## crates/roko-dreams/src/

### Dream-cycle and cross-domain-adjacent behavior

| File | What | Section |
|------|------|---------|
| `cycle.rs:398-430` | `DreamCycle::run_budgeted` calling `TierProgression::analyze + replay_heuristics` | D.10, F.13 |
| `cycle.rs:1622-1852` | cross-domain strategy hypothesis generation | F.14, N7 boundary note |

---

## crates/roko-chain/src/

### Attestation only

| File | What | Section |
|------|------|---------|
| `witness.rs:1-199` | witness engine surface; no neuro publish / token / demurrage path | E.14, E.21 |

---

## crates/roko-core/src/

### Related shared types

| File | What | Section |
|------|------|---------|
| `kind.rs:92,138` | `Kind::Pheromone` and string label | F.15 |
| `attestation.rs:46` | chain-id comment stub only, not Korai integration | F.08 |

---

## crates/roko-compose/src/

### Context consumers adjacent to neuro

| File | What | Section |
|------|------|---------|
| `context_provider.rs:453-490` | pheromone signal ingestion in compose layer | F.15 |
| `context_provider.rs:772-917` | pheromone-context extraction and prompt chunk injection | F.15 |
| `lib.rs` | re-exports and compose-side context surface | N1, F.15 |

---

## crates/roko-cli/src/

### Main CLI and orchestrator hot spots

| File | What | Section |
|------|------|---------|
| `main.rs:555-576` | `NeuroCmd` currently has only `Query`, `Stats`, `Gc` | E.15-E.16 |
| `orchestrate.rs:693-698` | `spawn_episode_distillation` hook on completion path | D.09, N4 |
| `orchestrate.rs:1842-1848` | direct `query_kind(...)` caller | C.07, N1 |
| `orchestrate.rs:3000-3255` | orchestrator neuro/runtime initialization path | C.07, N1 |
| `orchestrate.rs:7191` | direct `NeuroStore::query(...)` caller | C.07, N1 |
| `orchestrate.rs:7360-7445` | success-entry ingest path | E.11, N5 |
| `orchestrate.rs:10029-10037,10251-10253` | somatic bias path adjacent to neuro context assembly | E.10 |
| `orchestrate.rs:12890-12945` | `apply_knowledge_tier_feedback(...)` progression updates | A.14, D.10 |

---

## crates/roko-learn/src/

### HDC-related adjacent code

| File | What | Section |
|------|------|---------|
| `hdc_clustering.rs:38-170` | k-medoids HDC clustering used by learning paths | B.22, D.12 boundary |

---

## Missing / Absent (code-search negatives)

These doc features have no matching production code in `crates/`:

### Cross-domain transfer and analogy

| Absent Feature | Search | Section |
|----------------|--------|---------|
| `Resonance`, `ResonanceConfig`, `detect_resonances` | `rg -n "Resonance|ResonanceConfig|detect_resonances" crates/` | C.12 |
| `TransferRisk`, `TransferRecommendation`, `assess_transfer_risk` | `rg -n "TransferRisk|TransferRecommendation|assess_transfer_risk" crates/` | C.13 |
| `DomainProfile`, `DomainDistance`, `compute_domain_distance` | `rg -n "DomainProfile|DomainDistance|compute_domain_distance" crates/` | C.14 |
| `AnalogyResult`, `analogy_top_k` | `rg -n "AnalogyResult|analogy_top_k|analogy\\(" crates/` | C.15 |
| `ConfirmationRequest`, `ConfirmationResponse`, `ConfirmationTracker` | `rg -n "ConfirmationRequest|ConfirmationResponse|ConfirmationTracker" crates/` | C.16 |

### Advanced HDC helpers

| Absent Feature | Search | Section |
|----------------|--------|---------|
| `BundleAccumulator` | `rg -n "BundleAccumulator" crates/` | B.08 |
| `ItemMemory` | `rg -n "ItemMemory" crates/` | B.09 |
| `ResonatorNetwork` | `rg -n "ResonatorNetwork" crates/` | B.10, F.07 |
| `DecayingBundleAccumulator`, `OnlineBundler`, `fractional_bind` | `rg -n "DecayingBundleAccumulator|OnlineBundler|fractional_bind" crates/` | B.11 |
| `query_by_role`, `unbind_role`, `query_multi` | `rg -n "query_by_role|unbind_role|query_multi" crates/` | B.13 |
| `KnowledgeOntology`, `TypeSchema`, `ProvenanceChain` | `rg -n "KnowledgeOntology|TypeSchema|ProvenanceChain" crates/` | B.20 |
| `compress_episode`, `BundleDiversity` | `rg -n "compress_episode|BundleDiversity" crates/` | B.21 |

### Distillation extras

| Absent Feature | Search | Section |
|----------------|--------|---------|
| `extract_warnings`, `WarningCategory` | `rg -n "extract_warnings|WarningCategory" crates/` | D.11 |
| `cross_validation_check`, `anti_knowledge_check` | `rg -n "cross_validation_check|anti_knowledge_check" crates/` | D.13 |
| `DistillationScheduler`, `DistillationQualityReport` | `rg -n "DistillationScheduler|DistillationQualityReport" crates/` | D.14 |

### Exchange, backup, and frontier

| Absent Feature | Search | Section |
|----------------|--------|---------|
| `KnowledgeSource`, `MeshSync`, `KoraiChannel`, `LetheChannel` | `rg -n "KnowledgeSource|MeshSync|KoraiChannel|LetheChannel" crates/` | E.11-E.13 |
| `BackupManifest`, `Backup`, `Restore`, `Publish` on `NeuroCmd` | `rg -n "BackupManifest|enum NeuroCmd|Backup|Restore|Publish" crates/roko-cli crates/roko-neuro crates/roko-fs` | E.15-E.16 |
| `quarantine`, `consensus`, `sandbox`, `adopt` staging | `rg -n "quarantine|consensus|sandbox|adopt" crates/roko-neuro crates/roko-cli` | E.20 |
| `KORAI`, `demurrage`, `inheritance_discount`, `lineage_depth` | `rg -n "KORAI|demurrage|inheritance_discount|lineage_depth" crates/` | E.21-E.22 |
| `KnowledgeCrystal`, `CrystalStore`, `MetabolismMetrics`, `NeurosymbolicStore` | `rg -n "KnowledgeCrystal|CrystalStore|MetabolismMetrics|NeurosymbolicStore" crates/` | F.10 |

---

## Runtime Negatives That Matter For Batch 06

These matter because the code exists, but production or docs are still thinner or less honest than they should be:

| Runtime-negative | Evidence | Section |
|------------------|----------|---------|
| assembler built but no production caller | `ContextAssembler::new` / `.gather()` hits are confined to `context.rs` | C.07 |
| query semantics are more implicit than explicit | score formula and threshold contract live in code/comments rather than one canonical API | B.15, C.03 |
| D3 output path is unclear | `write_playbook` exists but is uncalled | D.03 |
| progression guards are thinner than the docs imply | no cross-validation or AntiKnowledge gate helpers | D.13 |
| inflow channels are much narrower than docs | no `KnowledgeSource` or multi-channel ingest surface | E.11 |
| backup/restore story is doc-only | `NeuroCmd` lacks matching subcommands | E.15 |
| some frontier self-reporting is stale | Dreams, transfer-adjacent hypotheses, and pheromone context already ship in narrower forms | F.13-F.15 |

---

## Practical Search Priorities

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

## Working Rule

If a neuro task requires:

- a new network protocol,
- a token or staking model,
- or a research-heavy HDC / neurosymbolic architecture,

then batch `06` should normally implement the smallest honest runtime contract and defer the rest.
