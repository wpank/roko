# F — Status Self-Report + Frontier (Doc 16)

Meta-parity of `docs/06-neuro/16-current-status-and-gaps.md`. Doc 16 is itself a
self-report, so each item here either confirms or challenges a doc-16 claim
against the current state of the tree. Frontier / Phase-2+ items are recorded
as grep-negatives where appropriate.

---

## F.01 — Self-claim: `KnowledgeKind`, `KnowledgeEntry`, `NeuroStore`, `KnowledgeStore` all implemented

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 16 §"Implemented Components › roko-neuro (Knowledge Store)" lists `KnowledgeKind` enum (6 canonical variants with legacy aliases), `KnowledgeEntry` struct (tier, refuted_insight_id, refutation_evidence, emotional_tag, emotional_provenance, hdc_vector fields), `NeuroStore` trait (init/query/ingest/decay/gc), and `KnowledgeStore` JSONL impl as implemented.
**Reality**: Confirmed on all four. `KnowledgeKind` at `crates/roko-neuro/src/lib.rs:42-59` defines exactly the six canonical variants (`Insight`, `Heuristic`, `AntiKnowledge`, `Warning`, `CausalLink`, `StrategyFragment`) with `#[serde(alias = ...)]` for legacy names (`Fact`, `Procedure`, `Playbook`, `Constraint`). `KnowledgeEntry` at `:186-243` has all claimed fields including `tier: KnowledgeTier` (`:233`), `refuted_insight_id` (`:207`), `refutation_evidence` (`:210`), `emotional_tag` (`:236`), `emotional_provenance` (`:239`), and `hdc_vector: Option<Vec<u8>>` (`:242`). `NeuroStore` trait at `:349-364` exposes the five methods (`init`, `query`, `ingest`, `decay`, `gc`) exactly as claimed. `KnowledgeStore` is the dominant file at `crates/roko-neuro/src/knowledge_store.rs` (2,006 LOC) with `impl NeuroStore for KnowledgeStore` at `:590-610`. Default half-life constants (`INSIGHT_HALF_LIFE_DAYS=30`, `HEURISTIC=90`, `WARNING=7`, `CAUSAL_LINK=60`, `STRATEGY_FRAGMENT=14`) match the PRD table at `lib.rs:29-37`. The doc self-report is accurate.

---

## F.02 — Self-claim: HDC operations (bind, bundle, permute, similarity, seeds, serde) implemented

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 16 §"roko-primitives (HDC Vectors)" claims `HdcVector` is `[u64; 160]` = 10,240 bits = 1,280 bytes, with XOR `bind()`, majority `bundle()`, cyclic `permute()`, Hamming `similarity()`, deterministic `from_seed()`, `to_bytes()`/`from_bytes()` (1,280 LE), serde, rkyv zero-copy, `fingerprint()`, `text_fingerprint()`.
**Reality**: All twelve entry points verified in `crates/roko-primitives/src/hdc.rs`. Storage is `bits: [u64; 160]` at `:24-26`. Operations: `zeros` at `:80`, `random` at `:86`, `bind` at `:107`, `bundle` at `:117`, `permute` at `:142`, `to_bytes` at `:168`, `from_bytes` at `:178`, `from_seed` at `:193`, `similarity` at `:211`, `similarity_archived` at `:226` (feature-gated `rkyv`), and the free functions `fingerprint` at `:242` and `text_fingerprint` at `:253`. Serde via custom `Serialize`/`Deserialize` at `:28-75`; rkyv feature-gated via `#[cfg_attr(feature = "rkyv", derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize))]` at `:20-23`. `from_seed` uses FNV-1a + splitmix64 (splitmix at `:6-12`). The doc self-report is fully accurate.

---

## F.03 — Self-claim: `Distiller`, `TierProgression`, `ContextAssembler` implemented

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 16 §"roko-neuro (Knowledge Store)" claims LLM-backed `Distiller` (Haiku default), `TierProgression` with D1/D2/D3 stages (`analyze`, `extract_insights`, `promote_heuristics`, `compile_playbook`, `replay_heuristics`), and `ContextAssembler` (canonical gather/rank/compress pipeline with PAD biasing, contrarian affect retention, auction-style token allocation) re-exported by `roko-compose`.
**Reality**: `Distiller` at `crates/roko-neuro/src/distiller.rs:42-44` with `DEFAULT_MODEL = "claude-haiku-3-5"` at `:25`, `with_claude`/`with_claude_model`/`with_backend` constructors at `:52-67`, async `distill()` at `:81-94`, `DistillationBackend` trait at `:31-38`. `TierProgression` at `crates/roko-neuro/src/tier_progression.rs:167-172` with default thresholds at `:174-183`, `analyze` at `:207-217` driving `discover_patterns → extract_insights → promote_heuristics → compile_playbook`. `TierProgressionReport { insights, heuristics, playbook }` at `:126-133` matches the D1/D2/D3 shape. `ContextAssembler` at `crates/roko-neuro/src/context.rs:221` (file is 2,362 LOC), with `apply_somatic_bias()` at `:1260` as the PAD-biasing hook. `roko-compose` re-exports via `crates/roko-compose/src/context_assembler.rs:4` (`pub use roko_neuro::{ContextAssembler, ContextChunk, PadState};`) and through `crates/roko-compose/src/lib.rs:39`. The doc self-report is fully accurate.

---

## F.04 — Self-claim "missing: full tier multiplier enforcement" is stale; tier multipliers ARE applied

**Status**: DONE (doc 16 self-claim is stale — tier multipliers are enforced in retrieval scoring)
**Severity**: LOW
**Doc claim**: Doc 16 §"Gaps" and §"Implementation Plan Mapping" hedge: "the tier multiplier system is now implemented on `KnowledgeEntry`" (abstract, line 22) but the D category summary at line 151 says only ~40% of D1-D18 is implemented. The doc calls the tier system "incomplete" without identifying the remaining gap precisely.
**Reality**: Tier multipliers are fully enforced end-to-end in retrieval scoring. `KnowledgeTier::multiplier()` at `crates/roko-neuro/src/lib.rs:113-120` returns `Transient=0.1, Working=0.5, Consolidated=1.0, Persistent=5.0`. `KnowledgeEntry::effective_half_life_days()` at `:281-288` composes `base_half_life * tier.multiplier()`. That method feeds directly into `recency_factor()` at `crates/roko-neuro/src/knowledge_store.rs:852-860` (`0.5_f64.powf(age / effective_half_life_days(entry))`), which feeds into the query score at `:303-310` (`score = keyword_score * confidence * recency * emotional [+ hdc_similarity]`). Unit test `effective_half_life_applies_tier_multiplier` at `lib.rs:402-425` confirms `base=20d, tier=Persistent` yields `100.0` days. The doc's hedge on "full tier multiplier enforcement" is stale — the only outstanding piece is the fuller PRD-specified `confidence × decay × similarity` product for cross-subsystem retrieval, which F.05 in doc 16 itself tracks as "moderate gap" at line 94.

---

## F.05 — Dissolution of `roko-golem` is COMPLETE; CLAUDE.md is stale

**Status**: DONE (dissolution complete; project CLAUDE.md is stale)
**Severity**: MEDIUM
**Doc claim**: Doc 16 §"Dissolved Components" and §"Crate Rename Status" claim `roko-golem` has been dissolved: `GrimoireEngine`, `GolemScaffold`, `ScaffoldEngine`, mortality engine all gone; crate rename marked **Completed**.
**Reality**: Directory `crates/roko-golem/` does not exist (`Glob 'crates/roko-golem/**/*'` returns **no files**). `Cargo.toml` workspace at `Cargo.toml:3-77` has **no** `roko-golem` member — only `roko-primitives`, `roko-runtime`, `roko-core`, `roko-std`, `roko-gate`, `roko-fs`, `roko-compose`, `roko-plugin`, `roko-agent`, `roko-orchestrator`, `roko-chain`, `roko-mcp-*`, `roko-cli`, `roko-serve`, `roko-agent-server`, `roko-conductor`, `roko-learn`, `roko-neuro`, `roko-dreams`, `roko-daimon`, `roko-demo`, `roko-lang-*`, `roko-index` (no `roko-golem` anywhere). `Grep 'roko-golem|roko_golem' Cargo.toml` returns **zero matches**. `Grep 'GrimoireEngine'` on `crates/` returns **no files** — matches remain only in docs. `Grep 'golem::' crates/roko-cli/` returns **zero matches**. The doc 16 self-claim is verified against the tree. However, the project-level `CLAUDE.md` still lists `roko-golem` as "Phase 2+" in the crate table at line 134 (`| roko-golem | crates/roko-golem/ | Chain witness, daimon, dreams, grimoire | Phase 2+ |`) — this instruction file is stale. Other docs that still reference `roko-golem/src/...` paths (`docs/09-daimon/*`, `docs/10-dreams/*`, `docs/00-architecture/15-crate-map.md`) are also stale for the same reason.
**Fix sketch**: Remove the `roko-golem` row at `CLAUDE.md:134` and update references in `docs/00-architecture/15-crate-map.md`, `docs/09-daimon/*`, and `docs/10-dreams/*` to the dissolved successor crates (`roko-daimon`, `roko-dreams`, `roko-neuro`, `roko-chain`).

---

## F.06 — VCG attention auction frontier (Tier 3 future)

**Status**: PARTIAL
**Severity**: MEDIUM
**Doc claim**: Doc 16 §"Priority 3 — Frontier Innovation Gaps" says "VCG attention auction" is "Partially implemented — Neuro does auction-style chunk allocation internally, and `PromptComposer` now runs a shared bidder-aware cross-subsystem auction over composed prompt sections with PAD-derived urgency / affect multipliers plus diagnostic externality payments; the remaining gap is exact welfare maximization, fairness policy, and fuller bidder coverage (Tier 2, P2)".
**Reality**: Confirmed — there is no explicit `VcgAttentionAuction` type. `Grep 'VcgAttentionAuction|VCG'` of `crates/` returns **zero matches** on the `VcgAttentionAuction` name and only finds `vcg_payment_summary` in `crates/roko-compose/src/prompt.rs:385, 587`. The auction machinery that *does* exist lives in the `PromptComposer`: `AttentionBidder` enum at `crates/roko-compose/src/prompt.rs:77` (8 bidders: `Neuro`, `Daimon`, `IterationMemory`, `CodeIntelligence`, `PlaybookRules`, `Research`, `TaskContext`, `Oracles` — see `bidder_tag` at `:251-260`), `AuctionAffectState` at `:468`, `bidder_affect_multiplier()` at `:634`, `effective_candidate_bid()` at `:621-630` that applies PAD-derived urgency / affect multipliers per bidder, and `vcg_payment_summary()` at `:587` that emits externality-style payment tags. The selected allocation is emitted through `Engram` tags at `prompt.rs:411-451` including `distinct_bidders`, `auction_total_bid` (`:418`), `auction_total_payments` (`:420-422`), `auction_urgency` (`:424-430`), `auction_affect_weight` (`:432-439`), `highest_payment_section` (`:442-446`), and `highest_payment_value` (`:448-450`). What is missing vs the full PRD spec (doc 16 line 112): exact welfare-maximizing allocation (current implementation is greedy-by-effective-bid, not optimal-subset), fairness policy (no per-bidder fairness floor), and full bidder coverage for the Neuro / Daimon cross-subsystem surface. The doc 16 self-report is accurate.
**Fix sketch**: Keep the "partially implemented" label. If/when the exact welfare-maximization step lands, update doc 16 line 112.

---

## F.07 — `ResonatorNetwork` frontier (HDC factor decomposition)

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Doc 16 §"Priority 2 — HDC Enhancement Gaps" row "ResonatorNetwork (factor decomposition)" → "No — advanced feature" (blocking = No).
**Reality**: `Grep 'ResonatorNetwork|resonator_network'` on `crates/` returns **zero matches**. There is no resonator implementation in `roko-primitives`, `roko-neuro`, or anywhere else in the tree. `crates/roko-primitives/src/hdc.rs` exposes 12 `pub fn` entry points (`zeros`, `random`, `bind`, `bundle`, `permute`, `to_bytes`, `from_bytes`, `from_seed`, `similarity`, `similarity_archived`, plus module-level `fingerprint` and `text_fingerprint`) — no factor-decomposition primitive, and no `BundleAccumulator`, `ItemMemory`, or `DecayingBundleAccumulator` either (all four listed in doc 16 §"Priority 2 — HDC Enhancement Gaps" as non-blocking). The doc self-classification as "frontier, not blocking" is accurate. Matches the Tier 3 frontier designation.
**Fix sketch**: Doc is fine. Add the ResonatorNetwork in `roko-primitives` when factor-decomposition becomes load-bearing; until then leave as declared frontier.

---

## F.08 — Korai chain integration frontier (Phase 2+)

**Status**: PARTIAL (only the `Korai` chain-id comment stub exists)
**Severity**: LOW
**Doc claim**: Doc 16 §"Priority 3 — Frontier Innovation Gaps" row "Korai chain integration" → "Not implemented" (references `04-knowledge-and-mesh.md` §2).
**Reality**: `Grep 'Korai|korai'` of `crates/` returns exactly **one** match: `crates/roko-core/src/attestation.rs:46` — a doc comment `/// Chain identifier (for example, Korai mainnet).` on the `ChainAttestation::chain_id: u64` field. No integration logic exists. The `roko-chain` crate is backend-agnostic: `crates/roko-chain/src/lib.rs:10-17` declares modules `alloy_impl` (feature-gated), `client`, `gate`, `mock`, `types`, `wallet`, `witness` with no `korai` backend. `ChainWitnessEngine` at `crates/roko-chain/src/witness.rs:18` (empty unit struct, file is 199 LOC of helpers — `witness_on_chain` at `:40-66`, `verify_on_chain` at `:70-89`) handles attestation anchoring on any `ChainClient`/`ChainWallet` but does not wire Korai specifically. The doc self-report is accurate; Korai is a **designed chain id** with no backend behind it.
**Fix sketch**: When Korai lands, add `korai_impl.rs` alongside `alloy_impl.rs` in `roko-chain`. Doc 16 line 120 correctly tracks this.

---

## F.09 — Mesh sync frontier (Phase 2+)

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Doc 16 §"Priority 3 — Frontier Innovation Gaps" row "Mesh sync" → "Not implemented" (references `04-knowledge-and-mesh.md` §3).
**Reality**: `Grep 'mesh_sync|MeshSync|mesh::'` of `crates/` returns **zero matches**. No mesh-sync primitive in `roko-neuro`, `roko-chain`, or anywhere else. The only "mesh" mentions in the tree are in docs (Korai mesh, agent mesh for contagion in `docs/09-daimon/12-collective-emotional-contagion.md`). Doc 16's self-report is accurate.
**Fix sketch**: Doc is fine. Phase-2+ frontier item, nothing to do today.

---

## F.10 — `KnowledgeCrystal` frontier (doc 16 §"Frontier Concepts")

**Status**: NOT DONE (design-only, not yet implemented)
**Severity**: LOW
**Doc claim**: Doc 16 §"Frontier Concepts: Knowledge Crystals and Knowledge Metabolism" (lines 194-306) specifies `KnowledgeCrystal` struct (8 fields: `id`, `principle`, `hdc_vector: HdcVector`, `confidence`, `confirmation_count`, `validated_domains`, `source_heuristics`, `crystallized_at`, `provenance`) and `CrystalStore` (`try_crystallize` with 5 gating criteria). Doc describes crystals as "the final stage of knowledge evolution — beyond Playbooks".
**Reality**: `Grep 'KnowledgeCrystal|CrystalStore|crystallize'` of the full repo returns matches only in **docs** (`docs/06-neuro/16-current-status-and-gaps.md`, `docs/10-dreams/11-inner-worlds-and-rendering.md`, `docs/12-interfaces/10-spectre-creature-visualization.md`) — **no source code**. The D-tier progression stops at `PlaybookCompilation` (`crates/roko-neuro/src/tier_progression.rs:117-122`); there is no fifth stage. The doc 16 Frontier Concepts section is explicitly labeled as design narrative, not implementation — so the absence is not a drift, but the self-report does not explicitly tag the crystal types as "design-only" at the section level.
**Fix sketch**: Add a one-line "Design — not implemented" tag to doc 16 line 198 so readers don't mistake the Rust snippet at `:209-307` for shipping code. Same note belongs on the `MetabolismMetrics` (lines 336-360) and `NeurosymbolicStore` (lines 457-464) blocks — none of those types exist in `crates/`.

---

## F.11 — Self-claim: AntiKnowledge 0.3 confidence floor + automatic HDC encoding on ingest

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 16 §"Priority 1 — Core Knowledge System Gaps" row 1 marks the AntiKnowledge 0.3 confidence floor as "Implemented — decay clamps at 0.3 and GC preserves AntiKnowledge entries" (line 92). Doc 16 §"Priority 2 — HDC Enhancement Gaps" row "Automatic HDC encoding on ingest" marks it "Implemented — ingest populates `hdc_vector` when the `hdc` feature is enabled" (line 105).
**Reality**: Both claims verified. `ANTI_KNOWLEDGE_CONFIDENCE_FLOOR: f64 = 0.3` at `crates/roko-neuro/src/knowledge_store.rs:26`. The decay path at `:381-399` explicitly clamps AntiKnowledge entries up to the floor: `entry.confidence = if entry.kind == KnowledgeKind::AntiKnowledge { decayed_confidence.max(ANTI_KNOWLEDGE_CONFIDENCE_FLOOR) } else { decayed_confidence };` (`:390-394`). The effective-confidence path for scoring re-applies the floor at `:873`. Unit test `decay_preserves_antiknowledge_confidence_floor` at `:1216` confirms behavior. For automatic HDC encoding, `normalize_entry_for_ingest` at `:749-759` is guarded by `#[cfg(feature = "hdc")]` and calls `ensure_hdc_vector` at `:738-747`, which invokes `fingerprint_entry` (→ `KnowledgeHdcEncoder.encode_entry`) at `:712-719` when the entry has no valid 1,280-byte vector. Ingest populates `hdc_vector` for every new entry when the `hdc` feature is enabled. The doc self-report is accurate on both.

---

## F.12 — Self-claim: typed HDC encoder for directional CausalLink encoding

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 16 §"Priority 2 — HDC Enhancement Gaps" row "Role vector registry / typed HDC encoder" marks it "Implemented at a lightweight level — directional CausalLink encoding and query probing now live behind a dedicated encoder module" (line 106).
**Reality**: The encoder module lives at `crates/roko-neuro/src/hdc.rs` (feature-gated via `#[cfg(feature = "hdc")]` at `lib.rs:371-372`). `KnowledgeHdcEncoder` struct at `:9`, with `encode_entry` at `:21-27` branching on `KnowledgeKind::CausalLink` → `encode_causal_link` at `:61-110`. Causal encoding uses directional `permute(CAUSE_SHIFT=1)` and `permute(EFFECT_SHIFT=2)` constants at `:5-6` and role vectors `role_hv("cause")`, `role_hv("effect")`, `role_hv("causal_edge")`, `role_hv("strength")`, `role_hv("domain")`, `role_hv("condition")` bundled together at `:66-107`. Query probing uses the same role bindings: `encode_query` at `:29-34` builds `bundle([topic_hv, cause_probe, effect_probe])` so causal-link lookups match both directions. Parsing supports structured tags (`cause:...`, `effect:...`, `strength:...`, `domain:...`, `condition:...`) and natural-language separators (`->`, `=>`, `→`, `causes`, `leads to`, etc.) at `parse_causal_content`, `:140-170`. The doc self-report is accurate.

---

## F.13 — Doc-16 claim "Dreams cycle is not [wired]" is stale

**Status**: DONE (doc 16 §117 is drifted — Dreams cycle IS implemented and wired into the CLI)
**Severity**: MEDIUM
**Doc claim**: Doc 16 §"Priority 3 — Frontier Innovation Gaps" row "Dream engine integration" says: "Distillation is wired; Dreams cycle is not" (line 117).
**Reality**: The Dreams cycle is implemented in `crates/roko-dreams/` and surfaces through the CLI. `DreamCycle` struct at `crates/roko-dreams/src/cycle.rs:333`, `DreamCycleReport` at `:67`, `DreamClusterKey` at `:259`, `DreamClusterReport` at `:300`. Additional modules `roko-dreams/src/runner.rs`, `lib.rs`, `hypnagogia.rs` all present. The CLI exposes `roko dream run/report/schedule` via `DreamCmd` at `crates/roko-cli/src/main.rs:372-391`, dispatched by `Command::Dream { cmd } => cmd_dream(cli, cmd).await` at `:952`. The CLI imports `use roko_dreams::{DreamAgentConfig, DreamEngine, DreamLoopConfig, DreamRunner};` at `main.rs:46`. Dream-time depotentiation is wired into `SomaticLandscape::apply_dream_depotentiation` (`crates/roko-daimon/src/lib.rs:1598`). The doc 16 self-claim is stale.
**Fix sketch**: Update doc 16 line 117 to "Distillation wired; Dreams cycle also wired (see `roko dream run/report/schedule` CLI surface)". The PRD should separately track any remaining Dreams gaps (e.g., fully autonomous scheduling, cross-subsystem replay) rather than claiming the whole cycle is unimplemented.

---

## F.14 — Doc-16 claim "Cross-domain resonance detection: designed, not implemented" is stale

**Status**: DONE (doc 16 §115 is drifted — cross-domain strategy transfer IS implemented in Dreams)
**Severity**: LOW
**Doc claim**: Doc 16 §"Priority 3 — Frontier Innovation Gaps" row "Cross-domain resonance detection" says: "Designed, not implemented" (line 115).
**Reality**: Cross-domain strategy transfer is implemented in the Dreams cycle. `generate_cross_domain_strategy_hypotheses` at `crates/roko-dreams/src/cycle.rs:1622-1691` takes `DreamCluster`s, computes HDC `cluster_structure_vector` per cluster, and for every failure-dominated target cluster scores all source clusters with `structural_transfer_score` (`:1647-1654`). `render_cross_domain_strategy_content` at `:1852-1875` synthesizes natural-language transfer hypotheses ("blend the X approach with the Y approach; structural match scores: …"). Hypotheses land as `KnowledgeEntry` values via the cycle. The function is called from the main dreams pipeline at `:487`. The doc 16 self-claim is stale — cross-domain transfer has real HDC-backed plumbing.
**Fix sketch**: Update doc 16 line 115 to "Partially implemented — Dreams cycle emits cross-domain strategy hypotheses via HDC cluster similarity (`roko-dreams/src/cycle.rs:1622-1875`). Remaining gap: no dedicated `ResonanceDetector` primitive in `roko-primitives` for ad-hoc cross-domain queries outside of Dreams."

---

## F.15 — Doc-16 claim "Pheromone system: types designed, not implemented" understates what ships

**Status**: DONE (doc 16 §116 understates — `Kind::Pheromone` and `pheromone_context` chunker are implemented and wired)
**Severity**: LOW
**Doc claim**: Doc 16 §"Priority 3 — Frontier Innovation Gaps" row "Pheromone system" says: "Types designed, not implemented (Tier 5E, P2)" (line 116).
**Reality**: A working pheromone context surface ships today. `Kind::Pheromone` enum variant at `crates/roko-core/src/kind.rs:92` with `as_str` label at `:138`. `ContextProvider` in `crates/roko-compose/src/context_provider.rs` tracks recent pheromone signals via `pheromone_signals: Vec<Engram>` at `:453-475`, with builder `with_pheromone_signals` at `:486-490`. Public `pheromone_context(field: &[Engram], scope: &str) -> Vec<ContextChunk>` at `:772` filters signals by `Kind::Pheromone` (`:776`) and current scope (`:777-778`). `add_pheromone_context` at `:899-917` injects chunks into the composed prompt surface with priority ranking via `pheromone_priority`. `pheromone_context` is re-exported from `roko-compose::lib.rs`. The doc 16 self-claim is too conservative — pheromones are more than "types designed".
**Fix sketch**: Update doc 16 line 116 to "Partially implemented — `Kind::Pheromone` Engram kind and `pheromone_context` chunker in `roko-compose` are wired; full mesh-level propagation and decay (Tier 5E, P2) still open."

---

## Section Summary

| Status | Count |
|--------|-------|
| DONE | 10 (F.01 core types, F.02 HDC ops, F.03 Distiller/Tier/Context, F.04 tier multipliers, F.05 golem dissolution, F.11 AntiKnowledge floor + auto HDC ingest, F.12 typed HDC encoder, F.13 Dreams cycle wired, F.14 cross-domain transfer, F.15 pheromone surface) |
| PARTIAL | 2 (F.06 VCG auction, F.08 Korai chain-id stub) |
| NOT DONE | 3 (F.07 ResonatorNetwork, F.09 Mesh sync, F.10 KnowledgeCrystal) |
| SCAFFOLD | 0 |

Doc 16's self-report holds up well on the core Tier-1 claims. `KnowledgeEntry`,
`KnowledgeKind`, `NeuroStore`, `KnowledgeStore`, all HDC operations, `Distiller`,
`TierProgression`, and the canonical `ContextAssembler` are in the tree exactly
as described. The tier multiplier hedge in the abstract is outdated — multipliers
flow through `effective_half_life_days` into `recency_factor` into the query
score today (F.04). The AntiKnowledge 0.3 floor, automatic HDC encoding on
ingest, and typed directional encoder for CausalLinks all ship as the PRD
claims (F.11, F.12). The `roko-golem` dissolution is complete in the workspace
(F.05), but the project-level `CLAUDE.md:134` and several `docs/09-daimon/` and
`docs/10-dreams/` files still reference `roko-golem/src/...` paths and mark the
crate as "Phase 2+" — meta-parity calls this out. Three doc-16 claims about
missing frontier work are themselves stale: the Dreams cycle is wired into the
CLI (F.13), cross-domain strategy transfer is implemented in Dreams via HDC
cluster similarity (F.14), and the pheromone system has a real `Kind::Pheromone`
Engram variant plus a `pheromone_context` chunker in `roko-compose` (F.15).
Frontier items that remain grep-negative (F.07 ResonatorNetwork, F.09 Mesh sync,
F.10 KnowledgeCrystal) and partially-done work (F.06 VCG auction without exact
welfare maximization, F.08 Korai chain-id comment-only stub) match their
self-reported status. Recommend: (a) strike `roko-golem` from `CLAUDE.md:134`,
(b) tag doc 16 §"Frontier Concepts" sub-sections with explicit "Design — not
implemented" markers so the `KnowledgeCrystal`, `MetabolismMetrics`, and
`NeurosymbolicStore` code snippets can't be misread as shipping types, and
(c) update doc 16 lines 115-117 to reflect that Dreams cycle, cross-domain
transfer, and pheromone context injection have all moved from "designed" to
at least "partially implemented".

## Agent Execution Notes

### F.05 / F.11-F.15 — Clean Up Contradictory Status Claims

This section is mostly docs honesty and should usually run late in the batch sequence.

Good targets:

1. remove stale `roko-golem` references from meta-docs,
2. correct “not implemented” claims for Dreams, pheromone, and transfer-adjacent surfaces that already exist,
3. keep partial implementations labeled as partial rather than upgraded to “complete”.

### F.06-F.10 — Mark Frontier Work Explicitly

Do not let frontier concepts read like current engineering commitments unless a batch actually implements them.

Acceptance criteria for this section:

- meta-docs stop contradicting crate reality,
- frontier sections are labeled as design-only where appropriate,
- later agents can trust doc `16` as a status summary instead of a source of ambiguity.
