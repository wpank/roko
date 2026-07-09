# C — Knowledge Query + Cross-Domain Transfer + ContextAssembler (Docs 08, 10)

Parity analysis of `docs/06-neuro/08-cross-domain-hdc-transfer.md` and
`docs/06-neuro/10-knowledge-query-api.md` vs the actual codebase.

---

## C.01 — `NeuroStore` trait (5 methods: init, query, ingest, decay, gc)

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 10 §"The NeuroStore Trait" (lines 25-46) — single entry point trait with exactly five methods: `init(path) -> Result<Self>`, `query(topic, limit) -> Result<Vec<KnowledgeEntry>>`, `ingest(entries) -> Result<()>`, `decay(&mut self) -> Result<usize>`, `gc(&mut self, min_confidence) -> Result<usize>`.
**Reality**: `crates/roko-neuro/src/lib.rs:349-364` defines the trait with exactly those five methods, matching the doc byte-for-byte (including `Sized` bound at `:349`). `KnowledgeStore` impls `NeuroStore` at `crates/roko-neuro/src/knowledge_store.rs:590-610` by delegating to its own inherent methods. The trait is re-exported at `crates/roko-neuro/src/lib.rs:383-385` alongside `KnowledgeConfirmationRecord`, `KnowledgeStats`, and `KnowledgeStore`; the `hdc`-gated `MemoryHit`/`MemoryIndex` re-exports sit at `:386-387`.

---

## C.02 — `KnowledgeStore` JSONL backend + append-only ingest

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 10 §"KnowledgeStore Implementation" — append-only JSONL at `.roko/neuro/knowledge.jsonl`, ingest appends one JSON per line, existing entries are never modified.
**Reality**: `crates/roko-neuro/src/knowledge_store.rs:70-75` defines `KnowledgeStore { path, confirmations_path, write_gate: Arc<Mutex<()>> }`. `for_roko_dir()` at `:115-117` builds `{root}/neuro/knowledge.jsonl` — matches doc's claimed path. `ingest()` at `:167-243` uses `OpenOptions::new().create(true).append(true)` at `:223-226`, writes one serialized entry + `\n` per line at `:228-233`, then `flush()` + `sync_all()` at `:234-235`. AntiKnowledge ingestion at `:194-217` does NOT append — it rewrites the whole file after halving the refuted entry's confidence, which the doc's "append-only" claim glosses over.
**Notes**: `add()` at `:157-159` is a single-entry wrapper over `ingest(vec![entry])`.

---

## C.03 — `query` + `query_kind` with composite scoring

**Status**: PARTIAL
**Severity**: LOW
**Doc claim**: Doc 10 §"Method Details" — query returns up to `limit` entries ranked by `confidence × decay_weight × (1 + hdc_similarity_bonus)`. Claim (line 57): "The current implementation uses simple keyword matching; the refactoring-prd design specifies a more sophisticated scoring function."
**Reality**: `crates/roko-neuro/src/knowledge_store.rs:255-257` exposes `query()`. The real scoring is richer than the doc admits: `query_filtered()` at `:279-322` scores each entry as `keyword_score * confidence * recency * emotional` (`:307`), then adds `hdc_similarity` as additive (not multiplicative `1 + bonus`) when the `hdc` feature is enabled (`:309-310`). `effective_confidence()` at `:866-868` multiplies confidence by `confirmation_boost` (1.5× when `source_episodes.len() >= 2`) and `emotional_consolidation_boost()`. `recency_factor()` at `:852-860` is `0.5^(age_days / effective_half_life_days)`. `query_kind()` at `:270-277` filters by `KnowledgeKind` using the same pipeline.
**Fix sketch**: Update doc §"Method Details" to show the real composite formula: `keyword × confidence × confirmation_boost × emotional_consolidation × recency (+ hdc_similarity)`. Doc's `confidence × decay_weight × (1 + hdc_similarity_bonus)` does not match the code.

---

## C.04 — `KnowledgeStats` struct shape

**Status**: PARTIAL
**Severity**: LOW
**Doc claim**: Doc 10 §"KnowledgeStats" — `{ total_entries: usize, entries_by_kind: HashMap<KnowledgeKind, usize>, mean_confidence: f64, entries_above_threshold: usize }`.
**Reality**: `crates/roko-neuro/src/knowledge_store.rs:78-90` defines `KnowledgeStats { total_entries, kind_counts: BTreeMap<String, usize>, average_confidence: Option<f64>, oldest_entry: Option<KnowledgeEntry>, newest_entry: Option<KnowledgeEntry> }`. Doc and code agree on the first field, but: (a) doc says `HashMap<KnowledgeKind, usize>`, code uses `BTreeMap<String, usize>` keyed by snake-case label from `knowledge_kind_label()` at `:907-909`; (b) doc says `mean_confidence: f64`, code uses `average_confidence: Option<f64>` (None when empty); (c) doc's `entries_above_threshold` field does not exist — code carries `oldest_entry`/`newest_entry` instead. `stats()` at `:333-374` is the sole constructor.
**Fix sketch**: Rewrite doc's `KnowledgeStats` block to reflect actual fields (`kind_counts` not `entries_by_kind`, `average_confidence: Option<f64>`, `oldest_entry`/`newest_entry` instead of `entries_above_threshold`).

---

## C.05 — `KnowledgeConfirmationRecord` shape

**Status**: PARTIAL
**Severity**: LOW
**Doc claim**: Doc 10 §"KnowledgeConfirmationRecord" — `{ entry_id: String, confirmed: bool, episode_id: String, timestamp: DateTime<Utc> }`. Used to trigger tier promotion/demotion.
**Reality**: `crates/roko-neuro/src/knowledge_store.rs:42-52` defines `KnowledgeConfirmationRecord { created_at: DateTime<Utc>, source_episodes: Vec<String>, confirmed_entry_id: String, confirming_entry_id: String }`. The doc's `confirmed: bool` (pos/neg outcome), `entry_id`, and `episode_id` fields do **not** exist. Instead, records are emitted when a newly ingested entry overlaps with an existing entry (detected via `entries_are_similar()` at `:927-950` and `detect_confirmations()` at `:957-993`), capturing both the `confirmed_entry_id` (the existing entry) and `confirming_entry_id` (the new one). They feed C-Factor metrics (`knowledge_integration_rate`, `convergence_velocity`) in `roko-learn`, not tier promotion.
**Fix sketch**: Rewrite doc §"KnowledgeConfirmationRecord" — field names and purpose differ from the shipping struct. Drop the "positive/negative outcome" framing and describe the overlap-detection mechanism instead.

---

## C.06 — `MemoryIndex` feature-gate (`#[cfg(feature = "hdc")]`)

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 10 §"Optional HDC MemoryIndex" — gated by `#[cfg(feature = "hdc")]`, struct holds `vectors: Vec<HdcVector>` + `entry_ids: Vec<String>`, plus `MemoryHit { entry_id, similarity }`.
**Reality**: `crates/roko-neuro/src/knowledge_store.rs:612-640` defines `MemoryIndex { entries: Vec<IndexedKnowledgeEntry> }` under `#[cfg(feature = "hdc")]`, where `IndexedKnowledgeEntry { entry: KnowledgeEntry, fingerprint: HdcVector }` at `:625-630`. `MemoryHit { entry: KnowledgeEntry, similarity: f64 }` at `:632-640`. Feature flag in `crates/roko-neuro/Cargo.toml:15-17` — `default = []`, `hdc = ["dep:roko-primitives"]`. **The `hdc` feature is NOT enabled by default** — it must be opted into explicitly. Build helper `KnowledgeStore::memory_index()` at `:576-587` is also cfg-gated. Re-exports at `crates/roko-neuro/src/lib.rs:386-387` are feature-gated.
**Notes**: Doc's claimed struct layout (`vectors: Vec<HdcVector>`, `entry_ids: Vec<String>`) is wrong — the shipping layout uses a single `Vec<IndexedKnowledgeEntry>` that holds both. `MemoryHit.entry_id` in doc is actually `MemoryHit.entry: KnowledgeEntry` in code. Both are feature-gated correctly, though.

---

## C.07 — `ContextAssembler` struct + `gather`/`rank`/`compress` pipeline

**Status**: PARTIAL
**Severity**: HIGH
**Doc claim**: Doc 10 §"ContextAssembler" (lines 151-178) — `ContextAssembler { knowledge_store: Arc<KnowledgeStore>, episode_store: Arc<EpisodeStore>, max_context_tokens: usize }`. Five-stage pipeline: query knowledge → query episodes → rank → budget auction → format. Doc 10 §"Integration Points" (lines 184-192) step 1 says the orchestrator calls Neuro "Before task execution: Query knowledge store for relevant context → inject into agent prompt".
**Reality**: `crates/roko-neuro/src/context.rs:221-227` defines `ContextAssembler { knowledge_store: Arc<KnowledgeStore>, episode_store: Arc<EpisodeStore>, affect_state: Option<PadState>, max_context_tokens: usize }` — doc missed the `affect_state` field. `gather()` at `:267-288` runs the full pipeline: `gather_knowledge` (`:522-533`), `gather_episodes` (`:535-566`), `gather_read_files` (`:568-609`), `gather_recent_signals` (`:611-643`), then `apply_somatic_bias` (`:283`), `rank` (`:291-315`), `compress` (`:365-520`), and `log_context_tuning` (`:317-363`).

**HOWEVER**: `rg 'ContextAssembler::new|\.gather\(' crates/` confirms the assembler is constructed and `.gather()`-ed **only** from inside `crates/roko-neuro/src/context.rs` tests (15 hits at lines 1768, 1771, 1839, 1842, 1952, 1954, 1957, 1958, 2162, 2181, 2257, 2276, 2289, 2309, 2334). `rg 'ContextAssembler' crates/roko-cli/` returns **zero matches**. `crates/roko-cli/src/orchestrate.rs` never instantiates `ContextAssembler` and never calls `gather()`. Instead it reaches the knowledge store directly: `KnowledgeStore::init(...)` at `:3221, :3346, :3469`; `knowledge_store.query_kind(...)` for StrategyFragments at `:1842-1848`; `NeuroStore::query(&self.knowledge_store, task_text, query_limit)` at `:7191`; `self.knowledge_store.add(anti_entry)` at `:5456`; `self.knowledge_store.ingest(vec![success_entry])` at `:7399`; and `self.knowledge_store.update_entries(...)` at `:12927`. The only `ContextAssembler` consumer outside `roko-neuro` is `crates/roko-compose/src/context_assembler.rs:1-4`, a four-line re-export shim (`pub use roko_neuro::{ContextAssembler, ContextChunk, PadState};`) that is itself not consumed by the CLI. Doc 10 §"Integration Points" step 1 ("inject into agent prompt") is aspirational — the wiring to orchestrate is absent, and the assembler's auction/PAD biasing never runs at inference time.
**Fix sketch**: Either (a) wire `ContextAssembler::new(Arc::new(knowledge_store.clone()), Arc::new(episode_store.clone())).with_affect_state(...).gather(workdir, &task, plan_id, signals_path)` into `orchestrate.rs` where raw `knowledge_store.query_kind` / `NeuroStore::query` are currently called (at `:1842-1848` and `:7191`), or (b) relabel Doc 10 §"Integration Points" step 1 as "library primitive — not yet called by orchestrator" and add a gap item to the doc's §"Current Status and Gaps".
**Notes**: The doc's field-order listing omits `affect_state: Option<PadState>`; see `:224`. `max_context_tokens` defaults to `4_000` at `:247`. Builder methods `with_affect_state` (`:253-256`) and `with_max_context_tokens` (`:260-263`) are the only runtime configuration knobs.

---

## C.08 — `ContextAssemblerConfig` struct

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: No explicit `ContextAssemblerConfig` struct is named in doc 10 (verified by reading §"ContextAssembler"). The doc describes configuration via the `max_context_tokens` field only. This audit item was included as a higher-order check for parity with a possible `*Config` sidecar.
**Reality**: `rg 'ContextAssemblerConfig' crates/` returns **zero matches**. Configuration is done via two builder methods on `ContextAssembler` itself: `with_affect_state()` at `crates/roko-neuro/src/context.rs:253-256` and `with_max_context_tokens()` at `:260-263`. Default budget is hard-coded `4_000` at `:247`. Constants like `BASE_ATTENTION_RESERVE = 0.18`, `MAX_CHUNK_BUDGET_FRACTION = 0.35`, `MIN_CHUNK_BUDGET_TOKENS = 32`, `SUMMARY_UTILITY_DISCOUNT = 0.86`, `SAME_SOURCE_DIMINISHING_RETURNS = 0.82`, `NOVELTY_PENALTY_WEIGHT = 0.35`, `MARGINAL_VALUE_STOP_RATIO = 0.5`, `CONTRARIAN_RETRIEVAL_RATIO = 0.15`, `CONTRARIAN_NEUTRAL_BAND = 0.1` at `:229-237` are module-level `const`s, not runtime-configurable.
**Fix sketch**: No doc change needed. If future work needs runtime-tunable thresholds (contrarian ratio, chunk budget fraction), expose a `ContextAssemblerConfig` and document it — currently those constants are compile-time.

---

## C.09 — Token budget enforcement (auction-style allocator)

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 10 §"ContextAssembler" step 4 — "Budget — run an auction-style allocator that weighs retrieval value against token cost, dampens repeated source families, and stops when marginal gain falls below the running average."
**Reality**: `crates/roko-neuro/src/context.rs:365-520` implements `compress()` exactly as described. Budget cap at `:370`, per-chunk cap `MAX_CHUNK_BUDGET_FRACTION = 0.35` at `:230` produces `max_chunk_tokens` at `:371-374`. Contrarian slice (`CONTRARIAN_RETRIEVAL_RATIO = 0.15` at `:236`) reserves a slot for affect-dissonant candidates at `:387-438` before the main auction. The main auction loop at `:440-499` iterates remaining candidates, scores via `ContextCandidate::best_choice()`, applies a reserve-price check at `:458-461`, and breaks when `best_choice.utility_density < average_density * MARGINAL_VALUE_STOP_RATIO && best_choice.bid_value < average_bid_value * MARGINAL_VALUE_STOP_RATIO` with at least 3 winners at `:486-491` (`MARGINAL_VALUE_STOP_RATIO = 0.5` at `:235`). Winners are re-sorted at `:501-506` and rendered full vs. summary based on `SelectionMode` at `:512-515`. `SAME_SOURCE_DIMINISHING_RETURNS = 0.82` at `:233` dampens repeated families via `selected_families` tracking. `estimate_chunk_tokens()` at `:1419-1421` uses the classic `len/4` char-to-token heuristic.

---

## C.10 — `apply_somatic_bias` + `PadState`

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 10 §"ContextAssembler" — "PAD-based affect biasing is also wired into ranking via `PadState`, and the allocator now enforces a small contrarian slice for affect-heavy retrieval so one mood does not monopolize the recalled knowledge set."
**Reality**: `crates/roko-neuro/src/context.rs:148-159` defines `PadState { pleasure, arousal, dominance, somatic_valence, somatic_intensity }` with `new()` (`:164-172`) and `with_somatic_hint()` (`:176-180`) builders plus `From<PadVector>` at `:183-187`. `apply_somatic_bias()` at `:1260-1292` (module-level `pub fn`, not a method) adjusts each chunk's relevance based on `action_orientation`/`caution_orientation` (`:1294-1359`) modulated by somatic pressure and emotional congruence. `affect_bias()` at `:1224-1258` applies a second PAD-aware bias inside `score_chunk`. Contrarian reservation is wired via `reserved_contrarian_slots` + `is_contrarian_candidate` in `compress()` at `:387-438`. Tests at `:1839-1842, :1952-1958` exercise different PAD states and verify retrieval changes; the test at `:1975-1976` asserts `StrategyFragment` stays at the top under both high-arousal and low-pleasure states.

---

## C.11 — `search_hdc` / `search_by_tag` / `search_by_tier` variants

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: No doc explicitly names `search_hdc`, `search_by_tag`, or `search_by_tier` on `KnowledgeStore`/`NeuroStore` (verified by reading doc 10 end-to-end). Audit item was a grep-target to check whether these variant entry points exist as sibling retrieval helpers alongside `query`/`query_kind`.
**Reality**: `rg 'search_hdc|search_by_tag|search_by_tier' crates/roko-neuro/` returns **zero matches**. The only `search_by_tag` in the workspace is on `SkillLibrary` at `crates/roko-learn/src/skill_library.rs:1104` (called from `crates/roko-cli/src/orchestrate.rs:7085` to find skills by role tag — unrelated to the knowledge store). `MemoryIndex::search()` at `crates/roko-neuro/src/knowledge_store.rs:678-696` is the sole HDC search path; it is generic over "query" strings and is not tag-scoped or tier-scoped. `query_kind()` at `:270-277` is the only axis-filtered variant (filters by `KnowledgeKind`, not tags or tier).
**Fix sketch**: No doc change needed. If tag- or tier-filtered retrieval becomes a first-class need (e.g., "only Persistent heuristics"), add `query_by_tier` / `query_by_tag` thin wrappers over the existing `query_filtered` closure (`:279-322`).

---

## C.12 — Doc 08 `Resonance` struct + `ResonanceConfig` + detection loop

**Status**: NOT DONE
**Severity**: HIGH
**Doc claim**: Doc 08 §"Implementation Details: Cross-Domain Resonance Detection Loop" (lines 198-298) — defines `Resonance { source_entry_id, target_entry_id, similarity, source_domain, target_domain, abstract_pattern }`, `ResonanceConfig { cross_domain_threshold: 0.526, max_resonances_per_entry: 5, generate_descriptions: true }`, and a `detect_resonances(new_entry, store, config)` algorithm that fires on every `NeuroStore::ingest()` call.
**Reality**: `rg 'Resonance|ResonanceConfig|detect_resonance|cross_domain_threshold' crates/` returns **zero matches**. No `Resonance` type, no `ResonanceConfig`, no detection hook in `KnowledgeStore::ingest()` (`crates/roko-neuro/src/knowledge_store.rs:167-243`). The ingest path only runs `detect_confirmations()` at `:219-221` (same-domain overlap detection, not cross-domain HDC resonance). Doc 08's own §"Current Status and Gaps" at lines 782-786 already admits this: "Missing: ... Cross-domain resonance detection loop (designed above)."
**Fix sketch**: Implementation is absent. Either ship `Resonance`/`ResonanceConfig`/`detect_resonances()` in `crates/roko-neuro/src/` (new `resonance.rs` module) and wire it into `KnowledgeStore::ingest()`, or keep doc 08's current "Design — not yet implemented" framing and move the Rust code blocks into a clearly-labeled design appendix.

---

## C.13 — Doc 08 `TransferRisk` + `TransferRecommendation` + `assess_transfer_risk`

**Status**: NOT DONE
**Severity**: MEDIUM
**Doc claim**: Doc 08 §"Transfer Risk Assessment" (lines 516-622) — defines `TransferRisk { structural_similarity, domain_distance, causal_alignment, historical_success_rate, risk_score, recommendation }`, `TransferRecommendation` enum with `Accept/AcceptWithCaution/Reject/NeedsReview`, and a `assess_transfer_risk(resonance, domain_stats) -> TransferRisk` function. Configurable weights: domain_distance=0.4, historical_success=0.3, similarity_risk=0.2, causal_alignment=0.1.
**Reality**: `rg 'TransferRisk|TransferRecommendation|assess_transfer_risk|DomainPairStats' crates/` returns **zero matches**. No transfer-risk type, no recommendation enum, no weighting logic, no H-divergence computation referenced in the doc. None of the negative-transfer safeguards described at lines 524-531 (Ben-David et al.) exist in code.
**Fix sketch**: Doc 08 §"Transfer Risk Assessment" must be labeled design-only until shipped. If this lands, it requires both `DomainPairStats` (not in code either — see C.14) and the `Resonance` type from C.12 as prerequisites.

---

## C.14 — Doc 08 `DomainProfile` + `DomainDistance` + `compute_domain_distance`

**Status**: NOT DONE
**Severity**: MEDIUM
**Doc claim**: Doc 08 §"Domain Distance Metric" (lines 626-723) — defines `DomainProfile { name, codebook: HashMap<String, HdcVector>, entry_vectors, gate_pass_rates, entry_count, mean_confidence, last_updated }` and `DomainDistance { vocabulary_divergence, structural_divergence, outcome_correlation, combined }` with a `compute_domain_distance(domain_a, domain_b) -> DomainDistance` function implementing three-component weighted geometric mean.
**Reality**: `rg 'DomainProfile|DomainDistance|compute_domain_distance|vocabulary_divergence|structural_divergence' crates/` returns **zero matches**. No domain-profile aggregation, no per-domain codebook, no gate-pass-rate tracking, no H-divergence approximation. Cross-domain strategy hypothesis generation in `crates/roko-dreams/src/cycle.rs:1622-1852` (`generate_cross_domain_strategy_hypotheses`, `render_cross_domain_strategy_content`) is the closest related work, but it runs inside Dreams cycle clustering and does not compute or persist a distance metric.
**Fix sketch**: Same as C.13 — design-only in doc today; requires new `crates/roko-neuro/src/domain.rs` module if shipped. The initial distance table (doc lines 729-738) is hand-waved estimates, not derived from running code.

---

## C.15 — Doc 08 `AnalogyResult` + `analogy` / `analogy_top_k` query API

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Doc 08 §"Analogical reasoning query API" (lines 422-512) — defines `AnalogyResult { concept: String, similarity: f32, significant: bool }` and two methods on `NeuroStore`: `analogy(a, b, c, codebook) -> Option<AnalogyResult>` and `analogy_top_k(a, b, c, codebook, k) -> Vec<AnalogyResult>`, implementing HDC analogy via `BIND(BIND(A,B), C)` followed by codebook nearest-neighbor lookup.
**Reality**: `rg 'AnalogyResult|fn analogy|ItemMemory' crates/` returns **zero matches**. The `NeuroStore` trait at `crates/roko-neuro/src/lib.rs:349-364` has exactly five methods (`init`, `query`, `ingest`, `decay`, `gc`) — no `analogy()`, no `analogy_top_k()`. No `ItemMemory` codebook type is defined anywhere. Doc 08's own §"Current Status and Gaps" lists this at line 786 as missing.
**Fix sketch**: Analogical-reasoning API is design-only. Shipping it requires (a) the `ItemMemory` codebook type referenced by the signatures, (b) a binding implementation on `HdcVector` (exists in `roko-primitives::hdc`), and (c) tier-progression hooks so analogies feed back into the knowledge store as new entries.

---

## C.16 — Doc 08 `ConfirmationRequest` / `ConfirmationResponse` / `ConfirmationTracker`

**Status**: NOT DONE
**Severity**: MEDIUM
**Doc claim**: Doc 08 §"Confirmation protocol (2+ agents)" (lines 335-419) — defines `ConfirmationRequest { request_id, resonance, deadline, required_confirmations }`, `ConfirmationResponse { request_id, agent_id, confirmed, reasoning, independent_similarity }`, and `ConfirmationTracker { request, responses, created_at }` with `is_confirmed()`, `is_rejected()`, `is_expired()` methods. 3-agent consensus algorithm for validating cross-domain resonances.
**Reality**: `rg 'ConfirmationRequest|ConfirmationResponse|ConfirmationTracker' crates/` returns **zero matches**. The only `KnowledgeConfirmationRecord` that exists is the overlap-detection record at `crates/roko-neuro/src/knowledge_store.rs:42-52` (see C.05), which is a **completely different** mechanism — it is emitted automatically when two entries in the same domain share enough tags/keywords, with no multi-agent quorum, no deadlines, and no reasoning fields. The 3-agent consensus protocol from doc lines 399-415 is entirely absent.
**Fix sketch**: Confirmation protocol is design-only. Needs the `Resonance` type from C.12 as a prerequisite plus new agent-messaging infrastructure (likely via `bardo-runtime` or `roko-orchestrator`) to send `ConfirmationRequest`s and collect responses with deadlines.

---

## C.17 — Doc 10 "Key Constants" `pub const CONFIRMATION_BOOST` visibility

**Status**: PARTIAL
**Severity**: LOW
**Doc claim**: Doc 10 §"Key Constants" (lines 93-99) shows `pub const DEFAULT_GC_MIN_CONFIDENCE: f64 = 0.05;` and `pub const CONFIRMATION_BOOST: f64 = 1.5;` as two public constants in `roko-neuro/src/knowledge_store.rs`.
**Reality**: `crates/roko-neuro/src/knowledge_store.rs:24` declares `pub const DEFAULT_GC_MIN_CONFIDENCE: f64 = 0.05;` (doc correct) but `:28` declares `const CONFIRMATION_BOOST: f64 = 1.5;` — **private**, not `pub`. It is referenced only inside `confirmation_boost()` at `:881`. The lib.rs re-export block (`crates/roko-neuro/src/lib.rs:383-385`) exports `DEFAULT_GC_MIN_CONFIDENCE` but cannot export `CONFIRMATION_BOOST` because it has no public visibility. Also missing from doc: companion private constants `ANTI_KNOWLEDGE_CONFIDENCE_FLOOR = 0.3` (`:26`), `MIN_TAG_OVERLAP = 1` (`:30`), `MIN_KEYWORD_OVERLAP = 2` (`:33`).
**Fix sketch**: Doc should drop the `pub` from `CONFIRMATION_BOOST` (or the code should expose it publicly). Optionally list the anti-knowledge/overlap thresholds alongside.

---

## C.18 — Doc 10 "With the Distiller" `DistillationBackend` trait signature

**Status**: PARTIAL
**Severity**: MED
**Doc claim**: Doc 10 §"Integration Points" > "With the Distiller" (lines 194-204) quotes a trait: `pub trait DistillationBackend: Send + Sync { async fn distill(&self, episode: &Episode) -> Result<Vec<KnowledgeEntry>>; }`, implying distillation goes per-episode through this trait.
**Reality**: `crates/roko-neuro/src/distiller.rs:30-38` defines `pub trait DistillationBackend: Send + Sync + std::fmt::Debug { async fn complete(&self, prompt: &str) -> Result<String>; fn model(&self) -> &str; }` — **completely different shape**. The trait only exposes raw LLM completion; knowledge extraction lives on `Distiller::distill(&self, episodes: &[Episode]) -> Result<Vec<KnowledgeEntry>>` at `:81` as an inherent method on the `Distiller` struct, and it takes a slice of episodes (batch), not a single episode. The actual prompt assembly/schema parsing happen in `distillation_system_prompt()` (`:342`) and `distillation_schema()` (`:362`), hidden behind `Distiller::distill`, not behind the backend trait.
**Fix sketch**: Rewrite doc §"With the Distiller" to show the real two-level split: the `DistillationBackend` trait models LLM completion (`complete`, `model`); the `Distiller` struct owns prompt construction + response parsing and exposes `distill(&[Episode])`. Adjust "Distilled entries are ingested into the NeuroStore at Transient tier" to be explicit that the Distiller returns entries and the caller (e.g., `spawn_episode_distillation` at `crates/roko-neuro/src/episode_completion.rs`) is what hands them to the store.

---

## C.19 — Doc 10 "Tier Progression Pipeline" default thresholds

**Status**: PARTIAL
**Severity**: LOW
**Doc claim**: Doc 10 §"Integration Points" > "With the Tier Progression Pipeline" (lines 206-214) claims D2 uses `min_support=5, min_confidence=0.7` when clustering Insights → HeuristicRules.
**Reality**: `crates/roko-neuro/src/tier_progression.rs:24-26` declares `const DEFAULT_MIN_SUPPORT: usize = 3;` and `const DEFAULT_MIN_CONFIDENCE: f64 = 0.7;`. `Default for TierProgression` at `:174-182` plumbs both into the struct. `min_confidence=0.7` matches the doc but `min_support=5` is wrong — **the default is 3**, not 5. `TierProgression::new(min_support, min_confidence, playbook_limit)` at `:188-200` accepts runtime overrides but the default pipeline sees 3.
**Fix sketch**: Update doc 10 line 211 to say `min_support=3, min_confidence=0.7` (or explicitly reference `DEFAULT_MIN_SUPPORT` / `DEFAULT_MIN_CONFIDENCE` so the doc auto-tracks future changes).

---

## C.20 — Doc 08 abstract role vector hierarchy / `KnowledgeHdcEncoder` role registry

**Status**: NOT DONE
**Severity**: MED
**Doc claim**: Doc 08 §"Abstract role vector hierarchy" (lines 300-333) specifies a two-tier role registry: abstract cross-domain roles (`role:risk_factor`, `role:response`, `role:pattern`, `role:severity`, `role:temporal`, `role:confidence`) plus domain-specific roles (coding: `role:crate/function/module`, chain: `role:protocol/asset/pool`, research: `role:source/citation/method`). Abstract roles are the only ones that contribute to cross-domain similarity. Adding a new abstract role requires three steps listed at lines 330-333: add the seed to `KnowledgeHdcEncoder::new()` role registry, update the encoding logic, and re-encode existing entries.
**Reality**: `rg 'role:risk_factor|role:response|role:pattern|role:severity|role:temporal|role:confidence' crates/` returns **zero matches**. `crates/roko-neuro/src/hdc.rs:9` declares `pub(crate) struct KnowledgeHdcEncoder;` as a **unit struct with no `new()` method and no role registry field**. The encoder uses only five hard-coded role strings via `role_hv("cause")` / `role_hv("effect")` / `role_hv("kind")` / `role_hv("source")` / `role_hv("causal_edge")` / `role_hv("strength")` inside `encode_generic_entry` (`:36-59`) and `encode_causal_link` (`:61-80`). There is no abstract/domain split, no registry, no per-entry enumeration of applicable roles. The unrelated `crates/roko-index/src/hdc.rs:130` has its own `role_vector(&SymbolKind)` for AST symbols, not knowledge-entry abstract roles.
**Fix sketch**: Either (a) ship a real role registry on `KnowledgeHdcEncoder` (likely `HashMap<&'static str, HdcVector>` populated in `new()`) and route abstract roles through `encode_generic_entry` based on entry-level tags/metadata, then re-encode the existing JSONL to populate `hdc_vector` with the new structure; or (b) label doc 08 §"Abstract role vector hierarchy" as a design appendix until shipped. Without the abstract roles, cross-domain resonance (C.12) has no structural signal to detect.

---

## C.21 — Doc 08 Strategy-Regime Bidirectional Lookup

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Doc 08 §"Applications Beyond Knowledge Retrieval" > "Strategy-Regime Bidirectional Lookup" (lines 147-161) describes forward/reverse HDC lookups: "Given this regime, what strategies worked?" and "Given this strategy, when has it been used?" Both paths are expressed as `BIND(role_volatility, hv_high) ⊕ BIND(role_liquidity, hv_low)` pattern queries against `StrategyFragment` or episode-summary entries.
**Reality**: `rg 'strategy_regime|regime_lookup|regime_bidirectional' crates/` returns **zero matches**. No API on `KnowledgeStore`, `NeuroStore`, or `MemoryIndex` takes a multi-role bound vector as a query. `KnowledgeStore::query_kind(topic, KnowledgeKind::StrategyFragment, limit)` at `crates/roko-neuro/src/knowledge_store.rs:270-277` (called from `crates/roko-cli/src/orchestrate.rs:1842-1848`) is the closest path, but it uses string-keyword matching with an additive HDC topic similarity (`:303-310`) — it does not accept a role-bound pattern vector and does not support "reverse" lookup (strategy → regimes). Episode-summary retrieval through the same pattern is absent. Doc 08's own §"Current Status and Gaps" at line 786 includes "Strategy-regime bidirectional lookup" in the Missing list.
**Fix sketch**: Needs C.20 (abstract role registry) as a prerequisite, plus a `query_by_pattern(pattern_hv, kind, limit)` on `MemoryIndex` (or a new method on `KnowledgeStore`) that bypasses keyword scoring and ranks purely on HDC similarity. Reverse lookup requires indexing episode summaries into the same HDC space.

---

## Section Summary

| Status | Count | IDs |
|--------|-------|-----|
| DONE | 5 | C.01, C.02, C.06, C.09, C.10 |
| PARTIAL | 7 | C.03 (scoring formula drift), C.04 (stats shape drift), C.05 (confirmation record shape drift), C.07 (assembler not wired into orchestrate), C.17 (`CONFIRMATION_BOOST` visibility), C.18 (`DistillationBackend` shape drift), C.19 (`min_support` default drift) |
| NOT DONE | 9 | C.08 (config), C.11 (search variants), C.12 (Resonance), C.13 (TransferRisk), C.14 (DomainProfile/Distance), C.15 (Analogy), C.16 (ConfirmationTracker), C.20 (abstract role registry), C.21 (strategy-regime bidirectional lookup) |
| SCAFFOLD | 0 | — |

*Note: C.07 is counted under PARTIAL because the assembler struct and its internal pipeline are fully implemented, but the wiring from `roko-cli/src/orchestrate.rs` into `ContextAssembler::gather()` is missing — orchestrate calls `KnowledgeStore::query_kind` (`:1842-1848`) and `NeuroStore::query` (`:7191`) directly instead, bypassing the auction and PAD biasing at inference time.*

**Doc 10 (Knowledge Query API)** ships the core trait and backend but has
six distinct drifts from code that callers will notice:
1. `KnowledgeStats` field names (`entries_by_kind`, `mean_confidence`,
   `entries_above_threshold`) do not match shipping struct (`kind_counts`,
   `average_confidence: Option<f64>`, `oldest_entry`/`newest_entry`) —
   see C.04.
2. `KnowledgeConfirmationRecord` is an overlap-detection event, not the
   `{entry_id, confirmed: bool, episode_id, timestamp}` tier-progression
   signal the doc describes — see C.05.
3. The composite score formula `confidence × decay_weight × (1 + hdc_similarity_bonus)`
   is a design note; real code is `keyword × confidence × confirmation_boost × emotional_consolidation × recency (+ hdc_similarity)`
   — see C.03.
4. `CONFIRMATION_BOOST` is private, not `pub const` — see C.17.
5. `DistillationBackend` trait has `complete`/`model`, not `distill(episode)` — see C.18.
6. Tier progression default `min_support` is 3, not 5 — see C.19.

Additionally, the feature-gated `MemoryIndex` layout in the doc
(`vectors: Vec<HdcVector>` + `entry_ids: Vec<String>`) does not match
shipping `entries: Vec<IndexedKnowledgeEntry>` — see C.06 notes — and the
`ContextAssembler` is a library primitive that orchestrate never
constructs (see C.07).

**Doc 08 (Cross-Domain HDC Transfer)** is the opposite: its §"Abstract"
and §"How Cross-Domain Transfer Works" sections describe a capability
that **does not exist in code**. Every struct between lines 198-622
(`Resonance`, `ResonanceConfig`, `TransferRisk`, `TransferRecommendation`,
`DomainProfile`, `DomainDistance`, `AnalogyResult`, `ConfirmationRequest`,
`ConfirmationResponse`, `ConfirmationTracker`) returns zero grep hits in
`crates/`. The abstract role vector hierarchy (C.20) and strategy-regime
bidirectional lookup (C.21) are likewise missing. Doc 08's own §"Current
Status and Gaps" at lines 782-786 already admits this, but the doc still
opens with "> **Implementation**: Built" at line 6 — that banner is
wrong and should read "Design — not yet implemented" until the resonance
detection loop, confirmation protocol, analogy API, and role registry
land. The closest real code is `roko-dreams/src/cycle.rs:1622-1852` for
cross-domain strategy hypotheses (`generate_cross_domain_strategy_hypotheses`
at `:1622`, `render_cross_domain_strategy_content` at `:1852`), which is
a different mechanism running on Dreams-cycle clusters rather than
per-ingest HDC resonance.

## Agent Execution Notes

### C.07 / C.09 / C.10 — This Is The Main Runtime Batch

This section owns the strongest “already built, not really live” seam in batch `06`.

Good execution sequence:

1. activate `ContextAssembler` on one real orchestrator path,
2. prove budget / PAD / contrarian logic can fire in production,
3. only then harden thresholds and surrounding query semantics.

### C.03-C.05 / C.08 / C.17-C.19 — Make The Contract Explicit

Prefer one clear runtime story for:

- score composition,
- stats shape,
- confirmation records,
- config and threshold semantics.

Do not invent a large new API just to mirror the docs.

### C.12-C.16 / C.20-C.21 — Truth-In-Advertising First

For doc `08`, the default action should be:

- make the header and implementation claims honest,
- distinguish Dreams-side transfer-adjacent work from the absent resonance system,
- only add code if the seam is genuinely tiny and useful.

Acceptance criteria for this section:

- `ContextAssembler` is no longer purely library-side,
- later agents can name the canonical query contract,
- doc `08` no longer overclaims implementation status.
