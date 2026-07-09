# E — Somatic, Library of Babel, Backup/Restore (Docs 13, 14, 15)

Parity analysis of `docs/06-neuro/13-somatic-integration.md`,
`docs/06-neuro/14-library-of-babel.md`, and
`docs/06-neuro/15-knowledge-backup-restore.md` vs the actual codebase.

---

## E.01 — `SomaticLandscape` k-d tree over 8D strategy space

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 13 §"The SomaticLandscape Struct" — `SomaticLandscape { tree: KdTree<f64, SomaticMarker, 8> }` storing situation-specific emotional markers indexed in an 8-dimensional strategy space, queried in < 1 ms to aggregate valence from nearby neighbors.
**Reality**: `crates/roko-daimon/src/lib.rs:1063-1071` defines `SomaticLandscape { markers: Vec<SomaticMarker>, tree: SomaticTree }` where `SomaticTree = KdTree<f64, STRATEGY_DIMENSIONS>` (`:30`) and `STRATEGY_DIMENSIONS = 8` (`:20`). Backed by the `kiddo = "5.3.0"` crate (`roko-daimon/Cargo.toml:21`). `SomaticLandscape::query()` at `:1155-1223` performs `SquaredEuclidean` k-nearest over the in-memory tree and returns a `SomaticSignal`. `record_marker()` at `:1124-1151` inserts with a 0.25-sq-Euclidean merge threshold (`SOMATIC_MERGE_DISTANCE_SQUARED` at `:23`). `rebuild_index()` at `:1095-1102` reconstructs the tree from persisted `markers` after deserialization.
**Notes**: The doc's signature `KdTree<f64, SomaticMarker, 8>` places markers inside the tree; the code uses the kiddo idiom of storing a `u64` index into the separate `markers: Vec<SomaticMarker>` slab, with the tree holding coordinates only. Behaviorally equivalent.

---

## E.02 — `SomaticMarker` struct (strategy_coords, valence, intensity, episodes)

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 13 §"The SomaticLandscape Struct" — `SomaticMarker { strategy_coords: [f64; 8], valence: f64, intensity: f64, episodes: Vec<ContentHash> }`.
**Reality**: `crates/roko-daimon/src/lib.rs:963-975` defines `SomaticMarker` with five fields: `strategy_coords: StrategyCoordinates` (a typed wrapper around `[f64; 8]`), `valence: f64` (clamped to `[-1.0, 1.0]`), `intensity: f64` (clamped to `[0.0, 1.0]`), `episodes: Vec<ContentHash>`, and an additional `updated_at: DateTime<Utc>` for emotional decay. `SomaticMarker::clamped()` at `:977-984` enforces the ranges (`valence.clamp(-1.0, 1.0)` at `:980`, `clamp_unit(self.intensity)` at `:981`). Valence / intensity behavior is exercised in `somatic_landscape_merges_nearby_markers` (`:2313-2325`), `somatic_summary_reflects_landscape_balance` (`:2327-2353`), and `somatic_query_blends_contrarian_markers` (`:2355-2386`).
**Notes**: Schema drift only: `[f64; 8]` in doc is spelled `StrategyCoordinates` in code (see E.03); the marker carries an extra `updated_at` timestamp the doc does not mention.

---

## E.03 — `StrategyCoordinates` (8D named wrapper)

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 13 §"The 8-Dimensional Strategy Space" — 8 named axes for the coding domain: `Complexity`, `Risk`, `Novelty`, `Confidence`, `Time pressure`, `Scope`, `Reversibility`, `Dependency depth`. Each axis is a continuous `[0, 1]` dimension.
**Reality**: `crates/roko-daimon/src/lib.rs:128-146` defines `pub struct StrategyCoordinates` with named fields `complexity`, `risk`, `novelty`, `confidence`, `time_pressure`, `scope`, `reversibility`, `dependency_depth` — exact doc match. `as_array()` at `:197` projects to `[f64; STRATEGY_DIMENSIONS]` for k-d tree queries. `clamped()` at `:212-222` forces each axis into `[0.0, 1.0]`. `neutral()` at `:182-193` returns the mid-space point with every axis at `0.5`. `StrategySpaceDefinition::default_dimensions()` at `:241-252` exposes the same 8 labels as `[String; 8]` for UI/serialization.

---

## E.04 — `AffectState` dimensions (PAD + confidence + behavioral_state + updated_at)

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 13 §"PAD Vector Integration" — the Daimon maintains a Pleasure-Arousal-Dominance state that drives retrieval bias. Doc does not explicitly claim a 9-dimensional `AffectState`.
**Reality**: `crates/roko-daimon/src/lib.rs:37-48` defines `AffectState { pad: PadVector, confidence: f64, behavioral_state: BehavioralState, updated_at: DateTime<Utc> }` — a **PAD-3** vector plus scalar confidence, discrete behavioral state, and timestamp. `PadVector` at `crates/roko-core/src/affect.rs:6-14` carries exactly three fields: `pleasure`, `arousal`, `dominance`, each in `[-1.0, 1.0]`. `AffectState::decay()` at `:71-85` applies half-life decay to both PAD and confidence. `refresh_behavioral_state()` at `:101-103` classifies the discrete state from PAD + confidence via `BehavioralState::classify` (`affect.rs:106-131`).
**Notes**: The actual model is PAD-3 plus auxiliary scalars, not 9-dimensional Plutchik-style affect. The PAD model is the one cited (Mehrabian 1996 in the doc's Academic Foundations).

---

## E.05 — `PadState` (PAD vector consumed by retrieval)

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 13 §"PAD Vector Integration" — a PAD vector from the Daimon drives retrieval bias in Neuro; low Pleasure biases toward Warnings / AntiKnowledge, high Arousal narrows retrieval scope, Dominance biases execution vs. exploration.
**Reality**: `crates/roko-neuro/src/context.rs:148-159` defines `pub struct PadState { pleasure, arousal, dominance, somatic_valence, somatic_intensity }` (all `f64`). `PadState::with_somatic_hint()` at `:175-180` attaches a somatic signal from the Daimon's strategy-space query. `From<PadVector> for PadState` at `:183-187` converts the Daimon's PAD into the retrieval-facing state. `ContextAssembler::with_affect_state()` at `:252-256` wires it into the assembler. Pleasure / arousal / dominance bias is implemented in `affect_bias()` at `:1224-1253`: `arousal_bias = arousal * (0.30*recency + 0.35*action)` (`:1247`), `pleasure_bias = low_pleasure * (1.00*caution - 0.30*action)` (`:1248`), `somatic_bias = negative_somatic * (0.90*caution - 0.20*action) + positive_somatic * (0.75*action - 0.08*caution)` (`:1249-1250`). The live PAD vector is sourced from `PlanRunner::current_pad_state` at `crates/roko-cli/src/orchestrate.rs:12871-12873`: `PadState::from(self.daimon.query().pad)`.

---

## E.06 — 15% contrarian retrieval constant

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 13 §"Mandatory 15% Contrarian Retrieval" — for each knowledge retrieval batch of ~20 entries, 85% selected by standard score and 15% (at least 3) selected from the OPPOSITE valence to prevent emotional echo chambers. Also surfaced in the somatic landscape's neighbor blend at 85/15.
**Reality**: Two independent 15% implementations are present.
- `crates/roko-daimon/src/lib.rs:22` — `const CONTRARIAN_FRACTION: f64 = 0.15;` used at `:1174` (`contrarian_target = ceil(k * 0.15)`) and at `:1214-1215` to blend congruent vs. contrarian valence as `0.85 * congruent + 0.15 * contrarian`.
- `crates/roko-neuro/src/context.rs:236` — `const CONTRARIAN_RETRIEVAL_RATIO: f64 = 0.15;` drives `reserved_contrarian_slots()` at `:1134-1156`, which picks `ceil(knowledge_candidates * 0.15)` with a floor of `3` when there are >= 20 knowledge candidates and `1` otherwise. `is_contrarian_candidate()` at `:1158-1176` enforces opposite-valence selection using `chunk_valence()` at `:1178-1180` and the `affect.pleasure < 0.5` threshold. Reservation wired into `ContextAssembler::compress()` at `:387-438` (the `if reserved_contrarian > 0` branch). The other `0.15` literals in `context.rs` are unrelated: `:798` is a same-task episode scoring bonus, `:1219` is the `dream_source_bonus` for KnowledgeEntries tagged `source == "dream"`, and `:1340` is the default `caution_orientation` fallback kind-score.
**Notes**: Both the somatic landscape (Daimon side) and the context assembler (Neuro side) enforce 15% / 85% blending — doc's "each knowledge retrieval batch" clause is satisfied on the Neuro side; the Daimon blend covers the somatic-signal side.

---

## E.07 — `DispatchStrategy` + `DispatchParams` + `CodingStrategySpace`

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 13 §"How It Works" — after querying the landscape, the agent routes on the aggregate valence (strong negative → stronger model / System 2; strong positive → cheaper model / System 1). Doc §"Coding domain (default)" — the 8 axes are specific to coding.
**Reality**: `crates/roko-daimon/src/lib.rs:1333-1347` defines `pub enum DispatchStrategy { Conservative, Balanced, Exploratory, Escalating, Proactive }`. `DispatchParams { model, turn_limit, strategy, effort }` at `:1363-1374`. `CodingStrategySpace` at `:728-740` owns the coding `StrategySpaceDefinition` and implements `StrategySpaceComputer<TaskStrategyObservation>` at `:863-870` and `StrategySpaceComputer<EpisodeStrategyObservation>` at `:873-881`. The coding extractors live at `:743-860`: `complexity_from_tier`, `scope`, `novelty`, `risk`, `reversibility`, `dependency_depth`, `task_time_pressure`, `episode_time_pressure` — one extractor per axis.
**Notes**: The `Proactive` strategy does not appear in the doc's System 1 / System 2 framing and is orthogonal to valence-driven routing (it is a background-maintenance mode, see `:1345-1346`).

---

## E.08 — Mood-congruence weight 0.15

**Status**: PARTIAL
**Severity**: LOW
**Doc claim**: Doc 13 §"Mood-Congruent Retrieval" — `retrieval_weight = base_weight × (1 + 0.15 × mood_congruence)`, where `mood_congruence` is the dot product between entry emotional valence and the Daimon's PAD vector. The 0.15 coefficient caps mood congruence at 15% of the retrieval weight.
**Reality**: `crates/roko-neuro/src/context.rs:1237-1245` implements mood congruence via `affect_pad.cosine_similarity(tag.mood_snapshot)` (not the raw dot product the doc describes) and scales it by `(congruence - 0.5) * (0.20 + intensity * 0.20)` — a weight of `0.20 + 0.20 * intensity`, **not** `0.15`. The separate `apply_somatic_bias()` at `:1260-1292` scales emotional match by `* 0.06` (`:1289`) as an additive retrieval prior, also not 0.15. There is no literal `0.15` multiplier on mood congruence anywhere in `crates/roko-neuro/`; the 0.15 constants in `context.rs:236, 798, 1219, 1340` are for contrarian ratio (E.06), a dream-bonus source priority, and unrelated source weights.
**Fix sketch**: Either replace the `× 0.15` coefficient in the doc's retrieval-weight formula with the actual `0.20 + 0.20 * intensity` scaling (or the `0.06` somatic prior), or change `context.rs:1243` to a bare `0.15` multiplier. The doc also frames `mood_congruence` as a dot product; the code uses PAD cosine similarity mapped to `[0, 1]` via `(dot / mags + 1.0) / 2.0` (`affect.rs:71-81`).

---

## E.09 — 8-axis strategy space (coding domain defaults + non-coding extensibility)

**Status**: PARTIAL
**Severity**: MEDIUM
**Doc claim**: Doc 13 §"The 8-Dimensional Strategy Space" — lists two concrete domains with 8 axes each: **coding** (`Complexity, Risk, Novelty, Confidence, Time pressure, Scope, Reversibility, Dependency depth`) and **chain** (`Volatility, Exposure, Liquidity, Correlation, Leverage, Time horizon, Slippage risk, Counterparty risk`). Doc claims the 8 dimensions are domain-configurable.
**Reality**: `crates/roko-daimon/src/lib.rs:226-305` defines `StrategySpaceDefinition { domain: String, dimensions: [String; 8] }` with `default_dimensions()` at `:241-252` listing exactly the 8 coding axes — match. `StrategySpaceDefinition::coding()` at `:256-258` returns the default. `validate()` at `:266-291` enforces non-empty unique labels. `classify_dimension_role()` at `:623-707` accepts keyword families including `volatility`, `exposure`, `leverage`, `slippage`, `counterparty`, `horizon`, `correlation` — enough to map the doc's chain axes onto the internal `DimensionRole` canonical profile — but there is **no dedicated `ChainStrategySpace` extractor**. The Daimon "Still missing" gap note in doc 13 §"Current Status and Gaps" explicitly calls out: "Dedicated non-coding strategy-space extractors beyond the centralized coding/default projection".
**Fix sketch**: Implement `ChainStrategySpace` (mirroring `CodingStrategySpace` at `:730-860`) for the chain-domain observation type, or keep the doc's "still missing" note in sync with the fact that non-coding strategy spaces fall back to label-aware projection via `RegisteredStrategySpaceComputer::task_coords` at `:910-919`.

---

## E.10 — `apply_somatic_bias` wired into `ContextAssembler`

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 13 §"Abstract" — "`roko-neuro::ContextAssembler` applies PAD-biased retrieval, direct somatic re-ranking, arousal-shaped scope, and a mandatory contrarian slice when selecting knowledge."
**Reality**: `crates/roko-neuro/src/context.rs:283` calls `apply_somatic_bias(&mut chunks, self.affect_state)` inside `ContextAssembler::gather()`, immediately after candidate collection and before scoring. The function at `:1260-1292` computes `somatic_pressure = somatic_valence * somatic_intensity`, and adjusts each chunk's relevance by `somatic_match * somatic_pressure.abs() * 0.08 + emotional_match * 0.06`. `DaimonState` → `PadState` wiring lives at `crates/roko-cli/src/orchestrate.rs:10029-10037` and `:10251-10253`: `query_somatic(task_strategy)` produces the signal which is then `.with_somatic_hint(valence, intensity)` into the task's affect state that feeds the assembler.

---

## E.11 — Five inflow channels (self / mesh / Korai / restore / Lethe)

**Status**: PARTIAL
**Severity**: HIGH
**Doc claim**: Doc 14 §"Five Inflow Channels" — knowledge enters through five named channels with source-specific discount factors: (1) self-distillation 1.00×, (2) collective mesh sync 0.80×, (3) public Korai marketplace 0.60×, (4) user restore 0.85×, (5) cross-collective Lethe 0.50×.
**Reality**: Only channel (1) is implemented. `crates/roko-neuro/src/knowledge_store.rs` has `NeuroStore.ingest()` for bulk admission, and `KnowledgeEntry` carries a `source` field for provenance tracking (noted in doc 14 §"Current Status and Gaps"). **Zero matches** for source-specific discount factor application — `rg '0\.80|0\.60|0\.50|discount_factor|source_discount|confidence_discount' crates/roko-neuro/` returns only unrelated float literals (source priorities, fixture content). No structured channel enum or ingestion path that differentiates self vs. mesh vs. marketplace vs. restore vs. Lethe.
**Fix sketch**: Add a `KnowledgeSource { SelfDistilled, CollectiveMesh, KoraiChain, UserRestore, Lethe }` enum with per-variant `discount_factor()`, thread it through `NeuroStore.ingest()`, and persist it on `KnowledgeEntry` alongside the existing freeform `source` string. Doc 14 §"Current Status and Gaps" already lists these as missing.

---

## E.12 — `MeshSync` primitive

**Status**: NOT DONE
**Severity**: MEDIUM
**Doc claim**: Doc 14 §"Agent Mesh (Peer / Private)" — mesh layer connects agents in the same permissioned collective via WebSocket / Iroh P2P. Doc 15 §"Mesh-Based Knowledge Sharing" — `[neuro.mesh_sync]` config block with `sync_types`, `sync_interval`, `min_confidence`.
**Reality**: `rg 'MeshSync|mesh_sync' crates/` returns **zero matches**. No mesh-sync primitive, no config loader, no periodic collective-sync task. The `roko.toml` schema does not parse `[neuro.mesh_sync]` anywhere in `crates/roko-core/src/config.rs` or `crates/roko-cli/src/config*.rs`.
**Fix sketch**: Mark doc 14 §"Agent Mesh" and doc 15 §"Mesh-Based Knowledge Sharing" as Phase 2+ vision. Until mesh transport and subnet protocol exist, the `[neuro.mesh_sync]` TOML block in doc 15 is aspirational.

---

## E.13 — `KoraiChannel` and `LetheChannel` primitives

**Status**: NOT DONE
**Severity**: MEDIUM
**Doc claim**: Doc 14 §"Public Korai Chain (Marketplace)" — agents publish/discover knowledge through the public Korai chain with KORAI-token economics. Doc 14 §"Cross-Collective Exchange (Lethe)" — cross-collective channel with 0.50× confidence discount.
**Reality**: `rg 'KoraiChannel|LetheChannel|korai_channel|lethe_channel' crates/` returns **zero matches**. `rg 'Lethe|LibraryOfBabel' crates/` also returns zero. The only `Korai`/`korai` references are the chain crate name and `attestation.rs` signature primitives. `roko-chain` implements attestation witnessing (see E.14) but exposes no knowledge-publish / knowledge-query API. The entire inflow layer is unimplemented.
**Fix sketch**: Keep doc 14 §"KORAI Token Economics and Knowledge" as design-only. No shipping code publishes knowledge to or queries knowledge from the Korai chain. The `Cross-Collective Exchange (Lethe)` section is equally aspirational.

---

## E.14 — `ChainWitnessEngine` (attestation anchoring, not knowledge publish)

**Status**: DONE for attestation witnessing; NOT DONE for knowledge publishing
**Severity**: LOW
**Doc claim**: Doc 14 §"Three-Level Knowledge Architecture" — the Korai chain anchors "on-chain HDC vectors, KORAI tokenomics, collective knowledge, reputation, ERC-8004". Doc 15 §"Publishing to Korai" — `roko neuro publish` CLI pushes validated HDC vectors to the chain.
**Reality**: `crates/roko-chain/src/witness.rs:18` defines `pub struct ChainWitnessEngine` (199 LOC total). `witness_on_chain()` at `:40-66` submits a small transaction whose calldata is `b"roko.attestation.witness:" ++ witness_hash` (built at `:114-128`) and attaches `ChainAttestation { chain_id, tx_hash, block_number }` on mined receipt. `verify_on_chain()` at `:70-90` checks that at least one log carries the topic `"roko.attestation.witness"` (`WITNESS_TOPIC` at `:12`) with data equal to `attestation.witness_hash()`. Tests at `:154-199` cover the happy path (`witness_roundtrip_records_chain_attestation` at `:168-190`) and the missing-witness case (`verify_on_chain_rejects_missing_witness` at `:192-198`). **However**: the engine only anchors attestation hashes, not HDC knowledge vectors. Doc 14's claim of "on-chain HDC vectors" and doc 15's `roko neuro publish` is not satisfied by `ChainWitnessEngine`. `rg 'witness_on_chain' crates/roko-cli/` returns zero matches — the engine is not wired into the CLI either.
**Fix sketch**: Keep the witness pipeline but note doc 14 §"Three-Level Knowledge Architecture" and doc 15 §"Publishing to Korai" as design — the current chain layer only witnesses attestation hashes, not knowledge bodies or HDC vectors. The CLI never calls `witness_on_chain` today.

---

## E.15 — 4-step backup/restore flow (BACKUP → DELETE → CREATE → RESTORE)

**Status**: NOT DONE
**Severity**: HIGH
**Doc claim**: Doc 15 §"The Four-Step Process" — users drive knowledge lifecycle through BACKUP (`roko neuro backup`), optional DELETE (`roko agent delete`), CREATE (`roko agent create`), and RESTORE (`roko neuro restore`) with filters (`--types`, `--min-confidence`, `--tags`, `--exclude-tags`, `--max-entries`, `--all`). Restored entries enter at Transient tier with `0.85×` confidence discount and updated provenance.
**Reality**: `crates/roko-cli/src/main.rs:555-576` defines `enum NeuroCmd { Query, Stats, Gc }` — **only three subcommands**. There is no `Backup`, no `Restore`, no `Publish` variant. `rg 'neuro backup|neuro restore|neuro publish' crates/` returns **zero matches** across the entire crate tree (only hits are in `docs/`). There is no `agent delete` or `agent create` subcommand either (no `AgentCmd { Delete, Create }`). The four-step flow is pure design.
**Fix sketch**: Mark doc 15 §"The Four-Step Process" entirely as "Design — not implemented". Add `Backup`, `Restore`, `Publish` variants to `NeuroCmd` and implement the export/import pipeline when prioritized. Doc 15 §"Current Status and Gaps" already calls this out; the audit just confirms it.

---

## E.16 — `BackupManifest` schema (manifest.json + knowledge.jsonl + hdc_vectors.bin)

**Status**: NOT DONE
**Severity**: MEDIUM
**Doc claim**: Doc 15 §"Backup format" — backup directory layout: `manifest.json` (agent ID, date, entry count, stats), `knowledge.jsonl`, `hdc_vectors.bin` (1,280 bytes × N), `tier_metadata.jsonl`, `provenance.jsonl`, `somatic_markers.jsonl`. BLAKE3 checksums for integrity verification.
**Reality**: `rg 'BackupManifest|RestoreEngine|RestoreOptions|manifest\.json|hdc_vectors\.bin' crates/` returns **zero matches**. No schema struct, no serializer, no file-layout writer, no checksum integrity check. The `KnowledgeEntry` type does support serialization (via serde, as doc 15 §"Current Status and Gaps" mentions), but there is no bulk export format.
**Fix sketch**: Keep doc 15 §"Backup format" as a proposed specification. Once `roko neuro backup` lands (E.15), implement `BackupManifest`, a writer for the six-file layout, and BLAKE3 integrity verification. Nothing in the current tree writes any of the listed files.

---

## E.17 — Arousal-driven retrieval scope (Yerkes-Dodson effective_limit)

**Status**: PARTIAL
**Severity**: LOW
**Doc claim**: Doc 13 §"Arousal Encoding" — arousal modulates **retrieval scope** with the formula `effective_limit = base_limit × (1 + 0.5 × (1 - |arousal - 0.5| × 2))`: moderate arousal → broad retrieval (limit × 1.5), extreme arousal → narrow retrieval (limit × 1.0), citing Yerkes-Dodson (1908).
**Reality**: Arousal is consumed **only as an additive score bias**, never as a limit modulator. `crates/roko-neuro/src/context.rs:1247` computes `arousal_bias = arousal * (0.30 * recency + 0.35 * action)` inside `affect_bias()` — this boosts recency and action-oriented chunks' scores but does not change the number of chunks retrieved. The token budget comes from `max_context_tokens` (default `4_000` at `:247`) and is not arousal-sensitive. `rg 'arousal.*limit|effective_limit.*arousal|Yerkes' crates/roko-neuro/` returns zero matches for the doc's formula. `max_context_tokens` is only set via `ContextAssembler::with_max_context_tokens()` at `:260-263` and never derived from PAD.
**Fix sketch**: Either implement arousal-sensitive budget scaling (wrap `compress()`'s `budget = self.max_context_tokens.max(1)` with the doc's inverted-U formula), or reframe doc 13 §"Arousal Encoding" around the additive-bias-only behavior that ships.

---

## E.18 — Emotional decay half-life (3-day PAD half-life claim)

**Status**: PARTIAL
**Severity**: LOW
**Doc claim**: Doc 13 §"Emotional Decay" — emotional half-life is **3 days** (72 h), distinct from knowledge half-life (type-dependent, 7–365 days). "Emotions fade faster than knowledge."
**Reality**: `crates/roko-daimon/src/lib.rs:1769-1771` defines `fn default_half_life_hours() -> f64 { 4.0 }` — the live PAD state decays with a **4-hour** half-life, not 3 days. `AffectState::decay()` at `:71-85` applies `0.5^(elapsed_hours / half_life_hours)` via `decay_factor()` at `:1773-1781`. Somatic marker intensity does not decay continuously at all; it is only reduced during Dreams cycles via `SomaticLandscape::apply_dream_depotentiation()` (`:1262-1281`) by a delta of `DEPOTENTIATION_DELTA_MIN = 0.30` to `DEPOTENTIATION_DELTA_MAX = 0.50` per pass (`:26-28`). Doc 13 §"Current Status and Gaps" already lists "Emotional decay (3-day half-life) separate from knowledge decay" as Still missing.
**Fix sketch**: Reconcile by either raising `default_half_life_hours()` toward `72.0` for PAD state, or revising the doc to describe the ship-level decay: 4-hour PAD half-life plus dream-driven marker depotentiation of 0.30–0.50 per cycle.

---

## E.19 — Publishing policies in `roko.toml` ([neuro.publishing])

**Status**: NOT DONE
**Severity**: MEDIUM
**Doc claim**: Doc 14 §"Publishing Policies" — agents declare `[neuro.publishing]` with `auto_publish`, `publish_to` (`"mesh" | "korai" | "both"`), `publish_types`, `exclude_tags`, `min_confidence`, `min_tier`. The policy classifies knowledge automatically into safe-to-share vs. private buckets.
**Reality**: `rg '\[neuro\.publishing\]|neuro\.publishing|publishing_policy|PublishingPolicy' crates/` returns **zero matches**. The publishing-related hits under `crates/roko-agent-server/` and `crates/roko-plugin/` are about server event publishing, not knowledge publishing. No TOML loader parses `[neuro.publishing]`, no `PublishingPolicy` struct, no auto-classification, and no enforcement layer.
**Fix sketch**: Mark doc 14 §"Publishing Policies" as design. Introduce `PublishingPolicy` in `roko-core::config` with a serde-compatible schema mirroring the doc, plumb it through `NeuroStore`, and guard a future `roko neuro publish` behind the policy check.

---

## E.20 — 4-stage ingestion safety pipeline (QUARANTINE → CONSENSUS → SANDBOX → ADOPT)

**Status**: NOT DONE
**Severity**: MEDIUM
**Doc claim**: Doc 14 §"Ingestion Safety" — externally ingested knowledge passes through a 4-stage pipeline: (1) QUARANTINE (isolate + HDC similarity check against known-bad + source-based confidence discount), (2) CONSENSUS (verify multi-agent agreement inside a collective), (3) SKILL SANDBOX (test StrategyFragment / Heuristic entries in a sandbox), (4) ADOPT (admit to `NeuroStore` at Transient tier). An "immune memory" LSH Bloom filter of previously rejected entries is maintained.
**Reality**: `rg 'Quarantine|QUARANTINE|SkillSandbox|skill_sandbox|ImmuneMemory|immune_memory|LshBloom|bloom_filter' crates/` returns **zero matches**. `NeuroStore::ingest()` at `crates/roko-neuro/src/knowledge_store.rs:167` accepts `Vec<KnowledgeEntry>` unconditionally and merges by `id`. There is no quarantine state, no consensus check, no sandbox execution path, and no Bloom filter of rejections.
**Fix sketch**: Keep doc 14 §"Ingestion Safety" as design-only. Until mesh/Korai transports exist (E.12, E.13), the 4-stage pipeline has no external inputs to gate, so the present grep-negative is aligned with the runtime reality.

---

## E.21 — KORAI token economics (posting cost, query cost, demurrage, challenge staking)

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Doc 14 §"KORAI Token Economics and Knowledge" — posting knowledge costs KORAI, querying costs KORAI per query, validated knowledge earns KORAI on confirmations, challenges require staking KORAI (refunded on success, forfeit on failure), and on-chain knowledge decays at **1% / year** demurrage (0.5% for AntiKnowledge).
**Reality**: `rg 'demurrage|DEMURRAGE|KORAI_token|token_economics' crates/` returns **zero matches**. `roko-chain` only implements the attestation witness path (E.14); there is no KORAI token contract interface, no demurrage schedule, no posting-cost deduction, and no query-billing hook. `roko-neuro` has no chain client dependency at all.
**Fix sketch**: Mark doc 14 §"KORAI Token Economics and Knowledge" as Phase 2+ vision. None of the economic machinery (staking, demurrage, earnings) exists in shipping code.

---

## E.22 — Inheritance / lineage discount (`0.85^N` geometric compounding)

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Doc 14 §"Inheritance Discounting" — when knowledge is restored across a lineage (A → B → C), confidence compounds geometrically: `confidence_after_N_transfers = original_confidence × 0.85^N`. After 5 transfers confidence drops to `0.85^5 ≈ 0.444`. Doc 15 §"Current Status and Gaps" lists "Lineage tracking with geometric confidence discount (0.85^N per transfer)" as Missing.
**Reality**: `rg '0\.85.*\^|powi.*0\.85|transfer_discount|lineage_discount|inheritance_discount' crates/` returns **zero matches**. `KnowledgeEntry` at `crates/roko-neuro/src/lib.rs:186-244` has a freeform `source: Option<String>` but no lineage-depth counter and no transfer-discount application. There is no restore pipeline to apply the discount, so the compounding logic has nowhere to live yet.
**Fix sketch**: Defer until E.15 (backup/restore flow) lands. At that point, extend `KnowledgeEntry` with a `lineage_depth: u32` counter, apply `confidence *= 0.85_f64.powi(depth as i32)` at restore time, and bump the counter on each subsequent re-restore.

---

## Section Summary

| Status | Count |
|--------|-------|
| DONE | 9 (E.01, E.02, E.03, E.04, E.05, E.06, E.07, E.10, E.14 attestation-only) |
| PARTIAL | 5 (E.08 mood-congruence weight, E.09 chain domain extractor, E.11 self-only channel, E.17 arousal scope, E.18 emotional decay half-life) |
| NOT DONE | 8 (E.12 MeshSync, E.13 Korai/Lethe, E.15 backup/restore flow, E.16 BackupManifest, E.19 publishing policies, E.20 ingestion safety pipeline, E.21 KORAI economics, E.22 inheritance discount) |
| SCAFFOLD | 0 |

The somatic layer (Doc 13) is the strongest: `roko-daimon` ships a real k-d-tree
`SomaticLandscape` over an 8-D coding strategy space with `SomaticMarker`,
`StrategyCoordinates`, `DispatchStrategy`, and an explicit 15% contrarian blend
(`CONTRARIAN_FRACTION`). `ContextAssembler` mirrors the contrarian ratio with
`CONTRARIAN_RETRIEVAL_RATIO = 0.15` and wires `apply_somatic_bias` plus
PAD-driven affect biasing through `orchestrate.rs`. The main somatic gaps are a
dedicated chain-domain extractor, the mood-congruence multiplier (doc claims
`× 0.15`, code uses `0.20 + 0.20 × intensity`), an arousal-driven
`effective_limit` scope formula that is only an additive score bias today
(E.17), and an "emotional half-life" that the doc states as 3 days but that
`default_half_life_hours` pins at 4 h (E.18). Doc 14 (Library of Babel) is
mostly vision: `MeshSync`, `KoraiChannel`, `LetheChannel`, `LibraryOfBabel`,
the five-channel discount table, publishing policies (`[neuro.publishing]`),
the 4-stage ingestion safety pipeline, immune-memory Bloom filter, KORAI
tokenomics, and lineage `0.85^N` compounding are all grep-negatives; only
self-distillation is implemented via `NeuroStore.ingest()`.
`ChainWitnessEngine` exists in `roko-chain` but only anchors attestation hashes
— not HDC knowledge vectors, and not wired into the CLI. Doc 15 (backup /
restore) is design-only: `NeuroCmd` has just `Query`, `Stats`, `Gc` — no
`Backup`, `Restore`, or `Publish` variant, no `BackupManifest`, and no backup
file-layout writer.

## Agent Execution Notes

### E.11 / E.20 — Narrow The Ingest Story

Best outcome for this section is not a full distributed Library of Babel implementation.

It is:

1. make the real ingest/source ownership story explicit,
2. decide whether any small source or staging contract belongs in runtime now,
3. leave mesh, Korai, Lethe, and token-governed ingest as explicit handoffs.

### E.15-E.16 / E.19 / E.22 — Backup MVP Or Honest Demotion

For unattended batch work, this section should converge on one of two clear results:

- local backup/restore has a real CLI and manifest contract,
- or docs `14` and `15` are made explicitly design-only for now.

Do not let publish, tokenomics, or lineage policy expand ahead of basic local backup semantics.

Acceptance criteria for this section:

- later agents can say exactly which inflow channels are real,
- backup/restore is either obviously live or obviously not,
- network and token layers are not accidentally dragged into a local runtime batch.
