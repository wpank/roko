# F — Advanced Allocation (Docs 07, 09, 10, 11, 12)

Parity analysis of scoring, affect, mechanism design, foraging, and distributed context docs vs actual codebase. Most items expected NOT DONE or SCAFFOLD.

Note: the relevant `ContextAssembler` and `PadState` structs actually live in `crates/roko-neuro/src/context.rs` (re-exported from `crates/roko-compose/src/context_assembler.rs`). Doc 12 points to `crates/roko-compose/src/context_assembler.rs`; in practice that file is a 4-line re-export of the roko-neuro types. Cross-reference paths throughout the docs are stale.

---

## F.01 — SectionScorer (static priority × recency × size × trust) (Doc 07 §4 / Doc 13)

- **Status**: DONE
- **Priority**: --
- **Estimated LOC**: 0
- **Dependencies**: None
- **Files to modify**: None

### What the doc says
Doc 07 §4 ("Comparison with Static Priority") describes `SectionScorer` in `roko-compose/src/scorer.rs` as a 4-component static scorer:
- `confidence = priority_to_score(section.priority)` mapped to `[0.2, 0.4, 0.8, 1.0]`
- `novelty = recency_decay(section.created_at)` — 1h fresh, 24h stale
- `utility = inverse_content_size(section.content)` — shorter = higher utility
- `reputation = trust_level(section.source)` — source trust

Doc 13 lists it as "Implemented (6 tests)" wired into `PromptComposer`.

### What exists
`SectionScorer` at `crates/roko-compose/src/scorer.rs:21-90` with Default `recency_window_ms = 1h`, `staleness_window_ms = 24h`. Four-component score produced by `score()`:

| Component | Doc | Code (scorer.rs) | Match |
|---|---|---|---|
| confidence | priority → [0.2, 0.4, 0.8, 1.0] | `Critical=1.0, High=0.8, Normal=0.4, Low=0.2` (lines 52-57) | MATCH |
| novelty | 1h fresh → 24h stale | linear decay between `recency_window_ms` and `staleness_window_ms` (lines 61-70) | MATCH |
| utility | inverse length | `(1000.0 / len).min(10.0)` (lines 73-75) | MATCH |
| reputation | trust_level(source) | `signal.provenance.trust` or `0.1` if tainted (lines 78-82) | MATCH |

6 tests in the `tests` module at scorer.rs:301-402:
1. `critical_scores_higher_than_low` (321)
2. `recent_section_has_full_novelty` (332)
3. `stale_section_has_zero_novelty` (341)
4. `mid_age_section_has_partial_novelty` (350)
5. `short_content_has_high_utility` (360)
6. `non_section_signals_get_zero_score` (371)
Plus 1 additional test for ActiveInferenceScorer (`active_inference_prefers_goal_aligned_sections` at 382) — so the file now has **7** tests total, not 6 as doc 13 claims.

Wired into `PromptComposer` via `Box::new(SectionScorer::new())` at `orchestrate.rs:10449`, composed with `CatalystScorer` and conditionally `PredictiveScorer` through a `SumScorer` at 10461.

### Gaps
| ID | Gap | Where | Severity |
|----|-----|-------|----------|
| F.01.1 | Doc 13 says "6 tests"; the file has 6 SectionScorer tests plus 1 ActiveInferenceScorer test = 7 total | doc 13 §2.3 vs scorer.rs:301-402 | LOW (cosmetic doc drift) |

### Verify
```bash
grep -c '#\[test\]' crates/roko-compose/src/scorer.rs
grep -n 'pub struct SectionScorer\|impl Scorer for SectionScorer' crates/roko-compose/src/scorer.rs
```

---

## F.02 — ActiveInferenceScorer / EFE (Doc 07 §2–§5)

- **Status**: PARTIAL
- **Priority**: P1
- **Estimated LOC**: 350
- **Dependencies**: F.05 (episode history query surface)
- **Files to modify**: `crates/roko-compose/src/scorer.rs`, `crates/roko-cli/src/orchestrate.rs`

### What the doc says
Doc 07 specifies expected-free-energy scoring:
```
G(section) = pragmatic_value + epistemic_value − ambiguity
P(include section_i) = softmax(γ × G_i), γ = 8.0
```
Components:
- **pragmatic_value** = `E[task_success|included] − E[task_success|excluded]`, queried from episode history (Doc 07 §3.1)
- **epistemic_value** = `D_KL(posterior || prior)` ≈ 1 − HDC-fingerprint similarity (Doc 07 §3.2)
- **ambiguity** = `Var[task_success|included]`
- **uncertainty** = 1/(1+N/10) + recent prediction error (Doc 07 §3.3)

Doc 07 §10 plus Doc 13 row "Active inference scoring (EFE)" state: SCAFFOLD — PadState exists, scorer interface exists; needs episode-history query + belief change. Doc 13 §6.1 flags it as "the highest impact gap" (expected +10–15% gate pass rate).

### What exists
`ActiveInferenceScorer` at `crates/roko-compose/src/scorer.rs:97-229`. It is NOT an EFE implementation as the doc describes — it is a goal-similarity scorer with a belief-based uncertainty bonus:

- `pragmatic_value` at scorer.rs:167-185 = `0.65 × cosine(goal_emb, section_emb) + 0.35 × token_overlap(content, goal) + priority_bonus` (NO episode history, NO conditional probability).
- `epistemic_value` at scorer.rs:187-199 = `0.65 × (1 − belief) + 0.2 × score.novelty + 0.15 × 1/√len` (NO KL divergence, NO HDC similarity to existing context).
- `ambiguity` term: **not computed**.
- Softmax with γ=8.0 selection: **not present**. The scorer emits `Score::new_extended(...)` with components `pragmatic, epistemic, pragmatic+epistemic, belief, goal_focus, salience, coherence` (scorer.rs:215-223). Integration with `PromptComposer`'s greedy `bid_density` path is standard scoring; no softmax / EFE-mixture selection exists.
- Embedding is a 32-dim hashed bag-of-words (`embed_text` at scorer.rs:231-244), not HDC fingerprints from `bardo-primitives`.
- Prior beliefs are a `HashMap<String, f64>` keyed by section name / topic / cache_layer (scorer.rs:143-165). No episode-history integration; the prior must be supplied by the caller.

Critically, `ActiveInferenceScorer` is **never instantiated** in `orchestrate.rs`. The only scorers wired into the live `PromptComposer` path at orchestrate.rs:10448-10461 are `SectionScorer`, `CatalystScorer`, and conditionally `PredictiveScorer`. `ActiveInferenceScorer` lives only in the 1 in-file test (`active_inference_prefers_goal_aligned_sections`).

### Gaps
| ID | Gap | Where | Severity |
|----|-----|-------|----------|
| F.02.1 | EFE formula `G = pragmatic + epistemic − ambiguity` not implemented; ambiguity term absent | scorer.rs:187-229 | HIGH |
| F.02.2 | Pragmatic value does NOT use episode history (`E[success|included] − E[success|excluded]`); uses cosine goal-similarity | scorer.rs:167-185 | HIGH |
| F.02.3 | Epistemic value does NOT use KL divergence / Bayesian surprise; uses a simple `1 − belief` proxy | scorer.rs:187-199 | HIGH |
| F.02.4 | No softmax selection with γ=8.0 over EFE scores | scorer.rs | HIGH |
| F.02.5 | Embedding is 32-dim hashed BOW, not HDC fingerprints from `bardo-primitives` (doc 07 §3.2) | scorer.rs:231-244 | MEDIUM |
| F.02.6 | `ActiveInferenceScorer` is never instantiated in the live orchestrator path | orchestrate.rs:10448-10461 | HIGH |
| F.02.7 | No uncertainty estimate from `episode_count` or `recent_accuracy` (doc 07 §3.3) | scorer.rs | MEDIUM |
| F.02.8 | No PAD modulation of EFE weights (doc 07 §7); table of PAD effects unimplemented | scorer.rs | MEDIUM |
| F.02.9 | No cold-start fallback policy (doc 07 §4: "use static priorities for first 10 episodes") | scorer.rs / orchestrate.rs | LOW |

### Verify
```bash
grep -n 'ActiveInferenceScorer' crates/roko-compose/src/scorer.rs
grep -rn 'ActiveInferenceScorer' crates/roko-cli/src --include='*.rs'
# expected: only scorer.rs definitions + tests; no orchestrate.rs wire-up
```

---

## F.03 — PAD state modulation (8 octants, P/A/D effects) (Doc 12 §1–§2)

- **Status**: PARTIAL
- **Priority**: P2
- **Estimated LOC**: 80
- **Dependencies**: None
- **Files to modify**: `crates/roko-neuro/src/context.rs`, `crates/roko-compose/src/prompt.rs`

### What the doc says
Doc 12 §1 defines `PadState { pleasure, arousal, dominance }` in `[-1.0, 1.0]` and §1.1 enumerates 8 octants (Excited, Surprised, Confident, Calm, Angry, Anxious, Bored, Sad) with different context biases.

§2 defines modulation rules at thresholds ±0.35:
- Arousal ≥ 0.35 → boost recent/action-oriented by ×1.5; suppress exploratory
- Arousal ≤ −0.35 → boost novel/exploratory by ×1.5
- Pleasure ≤ −0.35 → boost anti-patterns/warnings by ×1.5
- Dominance: affects explanatory vs directive content (§2.3 reserved for future)

Doc 13 table: "Arousal modulation in scoring — Implemented", "Pleasure modulation in scoring — Implemented", "Dominance modulation — Not yet".

### What exists
`PadState` struct in `crates/roko-neuro/src/context.rs:147-159` — has the 3 PAD fields plus additional `somatic_valence, somatic_intensity` fields (not in doc 12). `PadState::new` at 164, `with_somatic_hint` at 176.

Modulation is NOT a clean octant-based table. Instead it is a weighted sum in `affect_bias` at context.rs:1224-1253:
- Arousal boost: `arousal * (0.30 * recency + 0.35 * action)` — no explicit ×1.5 multiplier.
- Low-pleasure boost: `low_pleasure * (1.00 * caution − 0.30 * action)` — approximates the doc's pleasure rule.
- Also includes `emotional_congruence` and `somatic_bias` terms not mentioned in the doc.

Arousal/pleasure thresholds in the doc (±0.35) are NOT strict gates here — modulation is continuous. The ±0.35 threshold only shows up in `roko-neuro/src/lib.rs:173-178` as discovery-emotion label mapping (`high_arousal` / `low_arousal` strings).

A second PAD path exists in the live composer at `crates/roko-compose/src/prompt.rs:467-501`: `AuctionAffectState` reads `roko.daimon.{pleasure, arousal, dominance}` from `Context` attrs, computes `urgency_multiplier = 1.0 + arousal.clamp(0..1) * 0.5`, `affect_weight_multiplier = 1.0 + 0.3 * |pleasure − 0.5|`, and applies a per-bidder modulation in `bidder_affect_multiplier` (prompt.rs:634-754). Note the `[0, 1]` clamp means negative PAD values are clamped to 0 in the composer path — a different convention from doc 12's `[-1, 1]` range.

Dominance **IS** used in the composer path (`low_dominance_pressure()` at prompt.rs:494-496, applied in bidder formulas at 731-749). This contradicts doc 13's "Dominance modulation — Not yet".

8-octant taxonomy: **no explicit octant enum or classification**. The code uses continuous PAD biasing; `BehavioralState::classify` in `roko-core` produces an enum but is not PAD-octant naming from doc 12.

### Gaps
| ID | Gap | Where | Severity |
|----|-----|-------|----------|
| F.03.1 | No explicit 8-octant enum (Excited / Surprised / Confident / Calm / Angry / Anxious / Bored / Sad) — doc 12 §1.1 | context.rs / prompt.rs | LOW (continuous modulation works functionally) |
| F.03.2 | Arousal ±0.35 threshold is not a hard gate in scoring (doc 12 §2.1); modulation is continuous | context.rs:1224-1253 | LOW |
| F.03.3 | No ×1.5 multiplier for arousal or pleasure — magnitudes differ from doc (0.30, 0.35, 0.24, 0.18 instead) | context.rs:1247-1248, prompt.rs:730-754 | LOW |
| F.03.4 | Dominance IS wired (prompt.rs:494-496, 731-749) contrary to doc 13's "Not yet" claim | doc 13 §3 row "Dominance modulation" | MEDIUM (doc 13 stale) |
| F.03.5 | Composer path clamps PAD to `[0, 1]` instead of `[-1, 1]` (doc 12 §1) | prompt.rs:480-482 | LOW |
| F.03.6 | `PadState` has undocumented `somatic_valence / somatic_intensity` fields (Doc 12 §1 specifies 3 fields) | context.rs:155-158 | LOW (enhancement) |

### Verify
```bash
grep -n 'pub struct PadState' crates/roko-neuro/src/context.rs
grep -n 'AuctionAffectState\|bidder_affect_multiplier' crates/roko-compose/src/prompt.rs
# expected: PadState in context.rs; AuctionAffectState in prompt.rs (two separate modulation paths)
```

---

## F.04 — Affect persistence to `.roko/daimon/` (Doc 12 §4.3)

- **Status**: DONE (doc 13 claim of "SCAFFOLD / Not yet" is wrong)
- **Priority**: --
- **Estimated LOC**: 0
- **Dependencies**: None
- **Files to modify**: None

### What the doc says
Doc 12 §4.3 specifies PAD persistence at `.roko/daimon/affect.json` with JSON `{ pleasure, arousal, dominance, updated_at }`, loaded on startup and decayed from `updated_at` to `now`.

Doc 12 §8 row "PAD persistence" and Doc 13 §3 row "Affect persistence + decay" both say **"Not yet"**. Doc 13 §6.1 calls it the #3 highest-impact gap.

### What exists
PAD persistence **is fully wired**:
- `crates/roko-daimon/src/lib.rs:1445-1635`: `DaimonState` struct with `persistence_path: Option<PathBuf>`, `with_persistence_path` builder (1503), `persist()` method (1631, 1757), `load_or_new` (inferred from test 2126). Full appraisal (`appraise` at 1635-1708) handles GateResult, TaskOutcome, Blocked, TimePressure, QueueWait, DreamFailure.
- Decay toward baseline: `AffectState::decay` at `roko-daimon/src/lib.rs:71-85` with `decay_by_factor` applied to PAD vector.
- Wired path: `crates/roko-cli/src/orchestrate.rs:157-159` defines `daimon_state_path(workdir) = .roko/daimon/affect.json`. The file is also referenced by `crates/roko-learn/src/runtime_feedback.rs:70` and `crates/roko-serve/src/state.rs:43`.
- Tests: `appraise_updates_state_and_persists` at lib.rs:2123, `DaimonState::load_or_new` roundtrip test at 2126-2144.
- Doc 09-daimon/02-alma-three-layer-temporal.md §104 explicitly references `.roko/daimon/affect.json` as the mood-layer persistence path.

### Gaps
| ID | Gap | Where | Severity |
|----|-----|-------|----------|
| F.04.1 | Doc 12 §8 and Doc 13 §3 both say "PAD persistence — Not yet"; this is stale/incorrect | doc 12, doc 13 | MEDIUM (doc drift — real feature is shipped) |
| F.04.2 | Persistence lives in `roko-daimon` crate, not `roko-compose` (doc 12 cross-reference points to `context_assembler.rs` which only has a 4-line re-export shim) | roko-daimon/src/lib.rs vs doc 12 cross-refs | LOW (refactor / docs drift) |

### Verify
```bash
grep -rn 'affect.json' crates/ --include='*.rs' | head
grep -n 'persistence_path\|load_or_new\|fn persist' crates/roko-daimon/src/lib.rs | head
```

---

## F.05 — Neuro injection: KnowledgeStore query from ContextAssembler (Doc 11 §2.3 / Doc 13)

- **Status**: DONE
- **Priority**: --
- **Estimated LOC**: 0
- **Dependencies**: None
- **Files to modify**: None

### What the doc says
Doc 11 §2.3 lists "Stigmergic knowledge accumulation" and "HDC-based retrieval" among Level 3 features. Doc 13 §3 table row "Neuro injection into context — SCAFFOLD: ContextAssembler queries KnowledgeStore. Blocker: E6 in 12a plan: bridge roko-neuro". Doc 13 §6.1 lists it as a medium-impact gap expected to yield +8–12% gate pass rate.

### What exists
Neuro injection is fully wired — not scaffold:
- `ContextAssembler::gather_knowledge` at `crates/roko-neuro/src/context.rs:522-533` calls `self.knowledge_store.query(task_text, query_limit)` and converts results into `ContextChunk`s. `query_limit = 20` when `affect_state` is present, else 10 (doubling on affect matches doc 12 §3).
- `KnowledgeStore::query` at `roko-neuro/src/knowledge_store.rs:255-269` executes the actual topic-based retrieval; the `NeuroStore` trait (lib.rs:349-354) provides the abstraction.
- Direct wire in orchestrator: `crates/roko-cli/src/orchestrate.rs:7191` calls `NeuroStore::query(&self.knowledge_store, task_text, query_limit)` to build knowledge context for the agent prompt. Strategy-fragment variant at orchestrate.rs:7135-7143 via `build_strategy_fragment_context`.
- `AttentionBidder::Neuro` is defined in `prompt.rs:79` and mapped from `ContextSource::KnowledgeEntry` in `context_provider.rs:240-242`. The bidder has its own affect-modulation branch in `prompt.rs:731-735`.
- `KnowledgeStore::init(.roko/neuro/knowledge.jsonl)` called in 3 places in orchestrate.rs (3221, 3346, 3469).

### Gaps
| ID | Gap | Where | Severity |
|----|-----|-------|----------|
| F.05.1 | Doc 13 says SCAFFOLD; actually DONE. Bridge between roko-compose and roko-neuro exists via re-export shim (`compose/src/context_assembler.rs` = 4-line `pub use roko_neuro::...`) | doc 13 §3 "Neuro injection" | MEDIUM (doc drift — real feature is shipped) |

### Verify
```bash
grep -n 'pub fn query' crates/roko-neuro/src/knowledge_store.rs
grep -n 'NeuroStore::query\|knowledge_store.query' crates/roko-cli/src/orchestrate.rs | head
cat crates/roko-compose/src/context_assembler.rs  # the shim file
```

---

## F.06 — VCG attention auction (8 bidders, truthful allocation) (Doc 10)

- **Status**: PARTIAL
- **Priority**: P2
- **Estimated LOC**: 500
- **Dependencies**: F.02 (EFE-style pragmatic value feeds bid formula)
- **Files to modify**: `crates/roko-compose/src/prompt.rs`

### What the doc says
Doc 10 specifies a Vickrey-Clarke-Groves attention auction with:
- **Bid formula** (§3): `bid(section) = expected_value × urgency × affect_weight`
  - `expected_value = track_record(section) × relevance(section)`
  - `urgency = 1.0 + max(0, (deadline − now) / total_time_budget)^(−1)`
  - `affect_weight = daimon_modulation(section.type, pad_state)`
- **8 bidders** (§4): Episodic Memory, Knowledge Store (Neuro), Task Context, File Context, Safety System, Enrichment, Daimon (Affect), Collective
- **VCG allocation** (§5.2): combinatorial knapsack — `maximize Σ v_i × x_i s.t. Σ tokens × x_i ≤ budget`, solved by greedy (N < 50)
- **VCG payments** (§5.3): `payment(i) = Σ_{j≠i} value_j(optimal without i) − Σ_{j≠i} value_j(optimal with i)` — "externality imposed"
- **Truthful bidding** guarantee (§7.1), `LearningBidder` with Thompson sampling (§8.2), welfare loss / Pareto / PoA diagnostics (§9), alpha-fairness + safety floor (§10).

Doc 13 row "VCG attention auction — Partially implemented in PromptComposer: shared bidder-aware auction, PAD-modulated bidding, diagnostic externality payments. Fuller bidder coverage + fairness/exact-settlement policy."

### What exists
A real (but partial) auction IS implemented in `crates/roko-compose/src/prompt.rs`:

Eight-bidder enum at prompt.rs:74-95:
```rust
pub enum AttentionBidder {
    Neuro, Daimon, IterationMemory, CodeIntelligence,
    PlaybookRules, Research, TaskContext, Oracles
}
```
The 8 match doc 10's 8 bidders by concept but not by name (e.g., doc says "Knowledge Store (Neuro)" → code `Neuro`; doc "Safety System" → no direct match, closest is `PlaybookRules`; doc "Collective" → no direct match; "Oracles" and "PlaybookRules" are code additions).

Auction allocation: `select_optional_candidates` at prompt.rs:529-585 runs a greedy bid-density loop with per-bidder diversity boost (`wins == 0 → ×1.18`) and diminishing returns (`0.82^wins`). Tokens consumed against `remaining_tokens` budget — matches doc §5.2 greedy knapsack concept.

VCG payment computation: `vcg_payment_summary` at prompt.rs:587-619 implements the exact doc §5.3 formula — recomputes allocation with each winner excluded and sums `(without_winner.total_bid − others_with_winner).max(0)`. Diagnostics emitted as tags `auction_total_bid`, `auction_total_payments`, `auction_urgency`, `auction_affect_weight`, `highest_payment_section`, `highest_payment_value` (prompt.rs:418-450) — matches the `AuctionDiagnostics` struct sketched in doc 10 §9.4.

Bid formula: doc's `expected_value × urgency × affect_weight` is implemented as `bid_density (= value/tokens) × diversity_boost × diminishing_returns × affect_multiplier` at `effective_candidate_bid` (prompt.rs:621-632). The `affect_multiplier` in `bidder_affect_multiplier` (prompt.rs:634-754) incorporates urgency, affect-weight, and per-bidder biases driven by keyword detection (warning, failure, exploratory, proven, deadline) — an ad-hoc approximation of doc's `daimon_modulation` table but NOT a function of `track_record` or an explicit `expected_value` term.

Tests: `composer_spreads_budget_across_bidders_when_scores_are_close` (prompt.rs:1007), `auction_affect_multiplier_boosts_urgent_task_context` (1036), `composer_emits_auction_payment_diagnostics` (1061).

Missing from doc 10:
- **Truthfulness proof / verification** (§7.1) — not present. The learning-path `LearningBidder` / Thompson sampling (§8.2) is absent.
- **Auction diagnostics** beyond the four tags: no welfare-loss vs. optimal (§9.1), no Pareto-check (§9.2), no PoA calculation (§9.3).
- **Alpha-fairness config** (§10.3) and **safety floor hybrid** (§10.4) — not present. The safety-floor policy (200-token guaranteed minimum for safety subsystem) is unimplemented.
- **Explicit track_record** as bid input — only keyword-heuristic `subsystem_bias` is used, not historical `E[success|section_included]` from episode store.
- **8-bidder name alignment**: Safety System / Collective / Episodic Memory names from doc 10 §4 don't map 1-to-1 to the code enum.

### Gaps
| ID | Gap | Where | Severity |
|----|-----|-------|----------|
| F.06.1 | Bid formula uses keyword-heuristic `subsystem_bias`, not `E[task_success|included]` track record (doc 10 §3.1) | prompt.rs:730-750 | HIGH |
| F.06.2 | No `LearningBidder` with Thompson-sampling Beta posteriors (doc 10 §8.2) | prompt.rs | MEDIUM |
| F.06.3 | No welfare-loss, Pareto-optimality, or PoA diagnostics (doc 10 §9.1–§9.3) | prompt.rs:587-619 | LOW |
| F.06.4 | No alpha-fairness config (α ∈ {0 = VCG, 1 = proportional, ∞ = max-min}) from doc 10 §10.3 | prompt.rs | MEDIUM |
| F.06.5 | No VCG + safety-floor hybrid (doc 10 §10.4: 200-token floor for safety subsystem) | prompt.rs | MEDIUM |
| F.06.6 | 8-bidder enum names don't match doc 10 §4 (Safety System, Collective, Episodic Memory absent; Oracles/PlaybookRules added) | prompt.rs:74-95 vs doc 10 §4 | LOW (semantic mapping) |
| F.06.7 | No collusion / bid-correlation detection (doc 10 §8.4) | prompt.rs | LOW |
| F.06.8 | Doc 13 "Partially implemented" + "Fuller bidder coverage + fairness/exact-settlement policy" is accurate — this item is correctly flagged in doc 13 | (doc 13 §3 "VCG attention auction") | — |

### Verify
```bash
grep -n 'pub enum AttentionBidder\|vcg_payment_summary\|select_optional_candidates\|bidder_affect_multiplier' crates/roko-compose/src/prompt.rs
grep -n 'auction_total_payments\|auction_urgency' crates/roko-compose/src/prompt.rs
```

---

## F.07 — MVT predictive foraging (Charnov stopping rule) (Doc 09)

- **Status**: PARTIAL
- **Priority**: P2
- **Estimated LOC**: 400
- **Dependencies**: F.05 (KnowledgeStore queries as foraging "patches")
- **Files to modify**: `crates/roko-neuro/src/context.rs`, `crates/roko-cli/src/orchestrate.rs`

### What the doc says
Doc 09 specifies Charnov's Marginal Value Theorem applied to context search:
- **Stopping rule** (§2.1): `relevance(last) / cost(last) ≤ total_gain / total_cost`
- **Exponential gain curve** (§2.2): `g(k) = G_max × (1 − exp(−λk))`, `g'(k) = G_max × λ × exp(−λk)`
- **Default parameters** (§3.3): `G_max=1.0`, `λ=0.3`, `max_iter=10`, `min_iter=2`
- **Per-category calibration** (§4.1) — fit exponential curve from episode history
- **Multi-patch foraging** (§7): `MultiPatchForager` struct with `source_params: HashMap<ContextSource, (G_max, λ)>`, `travel_costs`, `environment_rate`, plus `optimal_order()` / `should_visit()` / `optimal_iterations()`
- **Social foraging / stigmergic signals** (§8): `RetrievalSignal` struct + `social_foraging_boost` with 24h decay half-life, cap at +0.3
- **Sufficient-context integration** (§9.3): `estimate_context_sufficiency` + `should_stop_searching(mvt, sufficiency, threshold=0.85)`

Doc 13 row "Predictive foraging MVT — Not yet: Stopping rule + calibration spec. Requires search iteration tracking."

### What exists
Partial MVT — a single-patch stopping rule exists but is simpler than doc 09:

`MARGINAL_VALUE_STOP_RATIO: f64 = 0.5` constant at `roko-neuro/src/context.rs:235`. Used in the `compress()` auction loop at context.rs:486-491:
```rust
if winners.len() >= 3
    && best_choice.utility_density < average_density * MARGINAL_VALUE_STOP_RATIO
    && best_choice.bid_value < average_bid_value * MARGINAL_VALUE_STOP_RATIO
{
    break;
}
```
This matches Charnov stopping **in spirit** — stops when marginal utility falls below half the running average — but:
- Threshold is `0.5 × average`, not the doc's exact `marginal ≤ average` (doc 09 §2.1).
- Requires `winners.len() >= 3` warm-up (no `min_iter=2` per doc §3.3, no `max_iter=10` cap in this path — budget/tokens terminates instead).
- Operates on a pre-gathered list of `ContextCandidate`s during compression, not during retrieval iteration. The actual "patches" (knowledge_store, episode_store, etc.) are queried unconditionally in `gather_knowledge`/`gather_episodes`/`gather_read_files`/`gather_recent_signals` — there is no per-source iteration or per-source cost accounting.

Missing:
- **No exponential gain curve model** `g(k) = G_max × (1 − exp(−λk))`. No `G_max`, no `λ` per source, no `fit_exponential_curve`.
- **No per-category calibration** from episodes (doc §4). `.roko/learn/` has various files but no `foraging-calibration.json`.
- **No `MultiPatchForager`** struct. No `optimal_order`, no `should_visit`, no `optimal_iterations`. Gathering in `ContextAssembler::gather` at context.rs:267-288 is hardcoded order: knowledge → episodes → read_files → recent_signals.
- **No `RetrievalSignal`** / stigmergic deposits after successful tasks. No social-foraging boost. No 24h decay half-life.
- **No sufficient-context estimator** (`estimate_context_sufficiency`). The dual stopping rule `mvt_ratio <= 1.0 || sufficiency >= 0.85` from doc §9.3 is absent.
- **No search-iteration tracking** for cross-task calibration (doc §4.2 feedback loop).

`crates/roko-core/src/prediction.rs:1` comment says "Predictive-foraging primitives: calibration-aware scoring and policy hooks" — that is `CalibrationTracker` / `PredictiveScorer`, NOT MVT foraging. These are complementary (the `Kind::Prediction` variant at kind.rs:99 is described as "prediction claim for predictive foraging") but don't implement Charnov's stopping rule.

### Gaps
| ID | Gap | Where | Severity |
|----|-----|-------|----------|
| F.07.1 | Exact MVT stopping rule (`marginal ≤ average`) not implemented; approximation uses `< 0.5 × average` | context.rs:486-491 | MEDIUM |
| F.07.2 | No exponential gain curve `g(k) = G_max × (1 − exp(−λk))` with `G_max` / `λ` per-source | context.rs | HIGH |
| F.07.3 | No per-category calibration from episode data (doc §4.1 `fit_exponential_curve`) | context.rs / roko-learn | HIGH |
| F.07.4 | No `MultiPatchForager` or `optimal_order()` — source order is hardcoded | context.rs:267-288 | MEDIUM |
| F.07.5 | No `should_visit()` / source-skip based on initial gain vs. environment rate (doc §7.1) | context.rs | MEDIUM |
| F.07.6 | No stigmergic `RetrievalSignal` + `social_foraging_boost` (doc §8.2) | crates/ | MEDIUM |
| F.07.7 | No `estimate_context_sufficiency` / sufficient-context dual stopping rule (doc §9.3) | context.rs | LOW |
| F.07.8 | No `max_iterations=10` / `min_iterations=2` safety bounds per doc §3.3 | context.rs | LOW |
| F.07.9 | Doc 13 "Not yet" understates the partial implementation — stopping-rule constant exists | doc 13 §4 "Predictive foraging MVT" | LOW (doc slightly pessimistic) |

### Verify
```bash
grep -n 'MARGINAL_VALUE_STOP_RATIO\|marginal_ratio\|stopping_rule' crates/roko-neuro/src/context.rs
grep -rn 'MultiPatchForager\|RetrievalSignal\|G_max' crates/ --include='*.rs'
# expected: single MARGINAL_VALUE_STOP_RATIO constant; no MultiPatchForager / RetrievalSignal
```

---

## F.08 — Write / Select / Compress / Isolate (Karpathy's 4 strategies) (Doc 11 §1)

- **Status**: PARTIAL
- **Priority**: P3
- **Estimated LOC**: 150
- **Dependencies**: None
- **Files to modify**: `crates/roko-compose/src/compaction.rs`, `crates/roko-compose/src/prompt.rs`

### What the doc says
Doc 11 §1 enumerates Karpathy's four context-engineering strategies:
- **Write** (§1.1): generate new context via enrichment pipeline (13 artifact types), Strategist plans, knowledge distillation.
- **Select** (§1.2): Stage 2 scoring + ContextTier + role templates + MVT stopping.
- **Compress** (§1.3): `ContextAssembler.compress()`, history compaction, `hard_cap` truncation, PromptBudget tiering. Spectrum: lossless → near-lossless (LLMLingua) → lossy (Haiku) → extreme (gist tokens).
- **Isolate** (§1.4): per-agent session isolation, stable prefix / volatile suffix cache layers, role template isolation, git worktree isolation.

Doc 13 §9 table:
- Write strategy (enrichment pipeline) — **Implemented**
- Select strategy (priority dropping, tier budgets) — **Implemented**
- Compress strategy — **Partially implemented**
- Isolate strategy — **Implemented**

### What exists
- **Write** — enrichment pipeline at `crates/roko-compose/src/enrichment/pipeline.rs` (774 LOC) with 13 `EnrichStep` variants in `enrichment/step.rs`. Strategist role and knowledge-distillation via `roko-neuro/src/distiller.rs` (831+ LOC). Matches doc.
- **Select** — `PromptComposer::compose()` at prompt.rs:322-458 enforces budget, drops Low/Normal under pressure (critical always kept). `SectionPriority` enum with 4 tiers (prompt.rs:27-37). `ContextTier::Surgical/Focused/Full` + complexity-adaptive budgets (`budget.rs`, `templates/common.rs`). Role-specific budgets via `PromptBudget` / `budget_for()`. Matches doc.
- **Compress** — `crates/roko-compose/src/compaction.rs` implements conversation-history compaction (`compact_history` at compaction.rs:70-100+): anchored summarization with `CompactionPolicy { trigger_threshold, anchor_roles, preserve_last_n_turns, summary_budget_tokens }`. 3 tests (`compaction.rs:376, 428, 464`). `PromptSection::enforce_hard_cap` at prompt.rs:182-204 handles per-section truncation. `ContextAssembler::compress` at context.rs:365-520 runs the auction-based chunk selection with Full/Summary modes (`SelectionMode::Full | SelectionMode::Summary`). **Missing**: no LLMLingua / token-pruning integration (doc §1.3 "near-lossless"), no gist tokens (doc §1.3 "extreme"). Haiku summarization is used by compaction but is generic (not fidelity-tiered).
- **Isolate** — `CacheLayer` enum (prompt.rs:45-55) defines the stable-prefix ordering: `Role=0, Workspace=1, Plan=2, Volatile=3`. The composer sorts sections by cache layer to maximize KV-cache hit (prompt.rs:373-376). Per-agent session isolation is orchestrator-level (each agent spawn is a fresh subprocess). Git worktrees are referenced in orchestrate.rs / bardo-runtime. Matches doc.

### Gaps
| ID | Gap | Where | Severity |
|----|-----|-------|----------|
| F.08.1 | No LLMLingua / token-pruning integration for near-lossless compression (doc 11 §1.3) | compaction.rs | MEDIUM |
| F.08.2 | No gist-token / prompt-distillation path (doc 11 §1.3 extreme compression) | compaction.rs / prompt.rs | LOW |
| F.08.3 | Fidelity-tier spectrum (lossless → near-lossless → lossy → extreme) not made explicit in API — only `hard_cap` truncation and LLM summarization | compaction.rs | LOW |

### Verify
```bash
grep -n 'compact_history\|CompactionPolicy\|SelectionMode' crates/roko-compose/src/compaction.rs crates/roko-neuro/src/context.rs
grep -rn 'llmlingua\|gist_token' crates/ --include='*.rs'
# expected: compact_history + SelectionMode present; no llmlingua / gist_token matches
```

---

## F.09 — Level 1/2/3 context engineering (Doc 11 §2)

- **Status**: PARTIAL (Level 1 + 2 DONE, Level 3 NOT DONE)
- **Priority**: P3
- **Estimated LOC**: 800+
- **Dependencies**: F.06 (VCG auction for Level 3 §2.3), F.07 (social foraging for Level 3 stigmergic accumulation)
- **Files to modify**: new crate `roko-mesh` or additions to `roko-compose` / `roko-neuro`

### What the doc says
Three-level taxonomy (Doc 11 §2):
- **Level 1 (Local)** — single agent, single task: priority dropping, U-shape placement, cache-aligned prefix, complexity-adaptive budgets, affect modulation.
- **Level 2 (Allocation)** — multiple agents, same plan: shared plan context, role-specific budgets, cross-agent iteration memory, differential compression by role.
- **Level 3 (Network)** — agent collective sharing a knowledge mesh: stigmergic accumulation, collective calibration, VCG auction, HDC retrieval, knowledge distillation, agent-mesh sync.

Doc 13 §9:
- Level 1 context engineering — **Implemented**
- Level 2 context engineering — **Partially implemented**
- Level 3 context engineering — **Scaffold**

### What exists
- **Level 1** — fully implemented:
  - Priority dropping in `PromptComposer::compose` (prompt.rs:333-398).
  - U-shape via `Placement::{Start, Middle, End}` (prompt.rs:62-72, sort at 400-404).
  - Cache-aligned prefix via `CacheLayer` order (prompt.rs:45-55).
  - Complexity-adaptive budgets via `budget.rs` + `adjusted_budget_for`.
  - Affect modulation via `AuctionAffectState` (prompt.rs:467-501) + PadState path (context.rs).
- **Level 2** — partial:
  - Shared plan context exists — `SharedPlanContext` / `PlanArtifacts` referenced in orchestrate.rs imports at line 33.
  - Role-specific budgets via `PromptBudget::budget_for(role)` in `templates/common.rs`.
  - Cross-agent iteration memory: `AttentionBidder::IterationMemory` bidder exists (prompt.rs:83), with per-bidder affect modulation (prompt.rs:739-741) — but the cross-agent pipeline (Agent A's errors → Agent B's context) is not an explicit API surface.
  - Differential compression by role: each role has different `PromptBudget` but no role-specific compression fidelity.
- **Level 3** — mostly absent:
  - No agent-mesh infrastructure — no `roko-mesh` crate; Styx → Agent Mesh rename has "N/A" status per doc 13 §8.
  - No stigmergic knowledge accumulation across agent boundaries (F.07.6 RetrievalSignal missing).
  - No VCG multi-subsystem auction at the collective level (single composer handles a single agent; F.06 is still single-agent).
  - No agent-mesh sync / permissioned knowledge sharing.
  - HDC retrieval is designed in `bardo-primitives` but not called from the live path (CLAUDE.md states "bardo-primitives — Built, not called"). HDC fingerprints exist; retrieval via HDC doesn't.
  - Collective calibration fragments: `CFactorPolicy` / `CFactorSource` / `c-factor.jsonl` (orchestrate.rs:178-182) hints at cross-agent metrics, but the output is still per-agent prompt injection, not a collective scheduling policy.

### Gaps
| ID | Gap | Where | Severity |
|----|-----|-------|----------|
| F.09.1 | Cross-agent iteration memory not a public API (Level 2 §2.2) | orchestrate.rs / prompt.rs | MEDIUM |
| F.09.2 | Differential compression by role not wired (Architect gets full code, QuickReviewer gets summary) | templates/ | LOW |
| F.09.3 | No agent-mesh infrastructure — no `roko-mesh` crate | crates/ | HIGH (blocker for Level 3) |
| F.09.4 | No stigmergic knowledge accumulation across agent boundaries (see F.07.6) | crates/ | HIGH |
| F.09.5 | HDC retrieval exists but not used in live path | bardo-primitives/ + roko-neuro | MEDIUM |
| F.09.6 | No permissioned knowledge-sharing policy for agent mesh | crates/ | MEDIUM |
| F.09.7 | Doc 13 "Level 3 — Scaffold" is optimistic — effectively DESIGN-ONLY / NOT DONE | doc 13 §9 | LOW (doc drift toward optimistic) |

### Verify
```bash
ls crates/ | grep -i mesh  # expected: no mesh crate
grep -rn 'roko-mesh\|agent_mesh\|StigmergicSignal' crates/ --include='*.rs'
grep -rn 'SharedPlanContext\|PlanArtifacts' crates/roko-compose/src/
```

---

## F.10 — RAGAS / CLEAR / meta-harness evaluation (Doc 11 §3, §5, §6)

- **Status**: NOT DONE
- **Priority**: P3
- **Estimated LOC**: 1200
- **Dependencies**: Evaluation harness infrastructure (task replay, gate replay)
- **Files to modify**: new `crates/roko-eval/` with `ragas.rs`, `clear.rs`, `meta_harness.rs`

### What the doc says
Doc 11 specifies three evaluation frameworks:
- **Meta-Harness** (§3, Lee et al. 2026 arXiv:2603.28052): "6× performance gap from scaffold changes alone", "4× fewer input tokens in best scaffolds", scaffold diversity measurement.
- **CLEAR framework** (§5): 5-dimensional evaluation — Cost, Latency, Efficacy, Assurance, Reliability. Co-optimization is 4.4–10.8× cheaper than efficacy-only.
- **RAGAS triad** (§6): Faithfulness (hallucination), Answer Relevance (task completion), Context Relevance (selection quality). Shahul Es et al., EACL 2024.

Doc 13 §9:
- RAGAS-style evaluation — **Not yet**
- Meta-Harness benchmarking — **Not yet**

### What exists
Zero. Search:
```
grep -rn 'ragas\|RAGAS\|CLEAR\|meta_harness\|contextual_influence\|CIV' crates/
→ No matches found
```
No RAGAS metrics, no CLEAR-dimension measurement, no Meta-Harness harness. The closest existing eval infrastructure:
- `crates/roko-learn/` — episode / efficiency / cascade-router stats (already wired per CLAUDE.md). These track throughput and pass rates but not RAGAS faithfulness / context-relevance.
- `c-factor.jsonl` (orchestrate.rs:178-182) — collective-factor history. Components include `gate_pass_rate, turn_taking_equality, social_sensitivity, task_diversity_coverage` (orchestrate.rs:285-288). Not a RAGAS/CLEAR mapping.
- `section-effectiveness` registry (orchestrate.rs:197-224, `SectionEffectivenessRegistry`) — per-section, per-role pass/fail tracking. This is adjacent to "Context Relevance" but is input-effectiveness, not output-quality.
- Gate outputs (roko-gate) measure task outcome but not Faithfulness / Answer Relevance in the RAGAS sense.

### Gaps
| ID | Gap | Where | Severity |
|----|-----|-------|----------|
| F.10.1 | No RAGAS metric implementation (Faithfulness / Answer Relevance / Context Relevance) | — | MEDIUM |
| F.10.2 | No CLEAR 5-dim evaluation framework | — | LOW |
| F.10.3 | No Meta-Harness benchmarking harness | — | LOW |
| F.10.4 | Existing `c-factor` + `section-effectiveness` provide related signals but no RAGAS / CLEAR mapping | orchestrate.rs | LOW |

### Verify
```bash
grep -rn 'ragas\|RAGAS\|CLEAR' crates/ --include='*.rs'
grep -rn 'meta_harness\|Faithfulness\|answer_relevance' crates/ --include='*.rs'
# expected: no matches
```

---

## F.11 — Contextual Influence Value (leave-one-out per-section) (Doc 11 §7)

- **Status**: NOT DONE
- **Priority**: P3
- **Estimated LOC**: 400
- **Dependencies**: F.10 (eval harness needed for controlled replay)
- **Files to modify**: new `crates/roko-eval/src/civ.rs`

### What the doc says
Doc 11 §7: Contextual Influence Value [Shanghai Jiao Tong University 2025] = per-section leave-one-out impact:
```
For each section:
    remove the section → re-run the task → measure performance change
```
Three evaluation dimensions per section: **query-aware relevance**, **list-aware uniqueness**, **generator-aware utility**. Enables targeted pruning of redundant or harmful sections.

Doc 13 §4 row "Contextual influence value — Not yet".

### What exists
Zero. `grep -rn 'contextual_influence\|CIV\|leave_one_out\|section_influence' crates/` returns no matches.

Adjacent: `SectionEffectivenessRegistry` at orchestrate.rs:197 tracks `included_trials, excluded_trials, included_passes` per `(section_name, role)` — this is leave-one-out-like data but is correlational (from organic runs) rather than controlled (leave-one-out replay). It feeds `CatalystScorer` (orchestrate.rs:10450) and section-effectiveness policy injection, so the raw signal exists but the CIV framework (three dimensions: query-aware / list-aware / generator-aware) is not computed or surfaced.

### Gaps
| ID | Gap | Where | Severity |
|----|-----|-------|----------|
| F.11.1 | No leave-one-out controlled replay to measure section influence | — | LOW |
| F.11.2 | No query-aware / list-aware / generator-aware dimensional breakdown | — | LOW |
| F.11.3 | Existing `SectionEffectivenessRegistry` has related data but not the 3-dimensional CIV decomposition | orchestrate.rs:197 | LOW |

### Verify
```bash
grep -rn 'contextual_influence\|CIV\|leave_one_out' crates/ --include='*.rs'
grep -n 'SectionEffectivenessRegistry' crates/roko-cli/src/orchestrate.rs | head
# expected: no CIV matches; SectionEffectivenessRegistry exists but is correlational
```

---

## F.12 — Meta-Harness scaffold benchmarking (Doc 11 §3)

- **Status**: NOT DONE
- **Priority**: P3
- **Estimated LOC**: 1500
- **Dependencies**: F.10 (RAGAS+CLEAR eval harness), test-task suite, multi-scaffold configuration
- **Files to modify**: new `crates/roko-eval/src/meta_harness.rs`, new `eval/` or `benches/` task suite

### What the doc says
Doc 11 §3 summarizes Lee et al. (2026) arXiv:2603.28052:
- 6× performance gap from scaffold changes alone (same model, different scaffolds)
- 4× fewer input tokens in the best scaffolds
- Scaffold diversity: no single scaffold dominates all task types

Implication for Roko: run the same model across N scaffold configurations on a fixed task suite, measure pass rate × token usage × latency, identify Pareto frontier. Validates the "scaffold IS the product" claim.

Doc 13 §9 row "Meta-Harness benchmarking — Not yet".

### What exists
Zero. `grep -rn 'meta_harness\|MetaHarness\|scaffold_benchmark' crates/` returns no matches.

No multi-scaffold configuration surface (CLAUDE.md indicates 18 crates focused on a single-configuration orchestrator). No golden benchmark task set. No per-scaffold diff reports in `dashboard` output.

### Gaps
| ID | Gap | Where | Severity |
|----|-----|-------|----------|
| F.12.1 | No multi-scaffold configuration system — single PromptComposer config per run | crates/ | LOW |
| F.12.2 | No fixed benchmark task suite for replay | benches/ / eval/ | LOW |
| F.12.3 | No Pareto-frontier reporting over (pass_rate, tokens, latency) tuples | roko-dashboard / eval | LOW |
| F.12.4 | Without F.10 (RAGAS / CLEAR), Meta-Harness cannot compute the "4× fewer input tokens in best" metric | — | LOW |

### Verify
```bash
grep -rn 'meta_harness\|scaffold_benchmark\|pareto_frontier' crates/ --include='*.rs'
ls benches/ eval/ 2>/dev/null
# expected: no matches; no top-level benches/eval directory
```

---

## Section Summary

| Item | Title | Status | Parity |
|------|-------|--------|--------|
| F.01 | SectionScorer (static 4-component) | DONE | 100% — all 4 components, 6 tests (+1 active-inference test) |
| F.02 | ActiveInferenceScorer (EFE) | PARTIAL | 25% — scaffold exists but not EFE, not wired into orchestrator |
| F.03 | PAD state modulation (8 octants) | PARTIAL | 60% — continuous PAD modulation works; no octant enum, no ×1.5 multipliers |
| F.04 | Affect persistence to `.roko/daimon/affect.json` | DONE | 100% — persistence, decay, appraisal all wired (doc 13 incorrectly says "Not yet") |
| F.05 | Neuro KnowledgeStore injection | DONE | 100% — ContextAssembler + orchestrate.rs query NeuroStore directly (doc 13 incorrectly says "SCAFFOLD") |
| F.06 | VCG attention auction | PARTIAL | 45% — greedy allocation + VCG payments implemented; no LearningBidder, no fairness/safety-floor |
| F.07 | MVT predictive foraging | PARTIAL | 20% — single `MARGINAL_VALUE_STOP_RATIO=0.5` constant; no gain curve, no multi-patch, no social |
| F.08 | Write / Select / Compress / Isolate | PARTIAL | 75% — Write/Select/Isolate done; Compress lacks LLMLingua / gist tokens |
| F.09 | Level 1 / 2 / 3 context engineering | PARTIAL | 60% — Level 1 done, Level 2 partial, Level 3 effectively DESIGN-ONLY |
| F.10 | RAGAS / CLEAR / Meta-Harness eval | NOT DONE | 0% — no matches in codebase |
| F.11 | Contextual Influence Value (leave-one-out) | NOT DONE | 5% — related `SectionEffectivenessRegistry` exists but is correlational |
| F.12 | Meta-Harness scaffold benchmarking | NOT DONE | 0% — no multi-scaffold config, no golden task suite |

### Status counts
- **DONE**: 3 (F.01, F.04, F.05)
- **PARTIAL**: 6 (F.02, F.03, F.06, F.07, F.08, F.09)
- **NOT DONE**: 3 (F.10, F.11, F.12)

### Doc 13 SCAFFOLD claims that are actually wrong
- **F.04** Affect persistence + decay — Doc 13 §3 says "Not yet"; actually DONE (full `DaimonState::persist` + `load_or_new` + decay + appraisal in `roko-daimon`, wired in orchestrate.rs and roko-learn/runtime_feedback.rs).
- **F.05** Neuro injection — Doc 13 §3 says "SCAFFOLD: bridge roko-neuro"; actually DONE (`ContextAssembler::gather_knowledge` calls `KnowledgeStore::query`; `orchestrate.rs` calls `NeuroStore::query` directly).
- **F.03** Dominance modulation — Doc 13 §3 and Doc 12 §8 say "Not yet"; dominance IS used by `AuctionAffectState::low_dominance_pressure` and the per-bidder modulation (prompt.rs:494-496, 731-749).

### Doc 13 SCAFFOLD claims that are optimistic (real state worse than doc 13 suggests)
- **F.02** Active inference — Doc 13 says "SCAFFOLD: PadState exists, scorer interface exists"; actually the scorer is a goal-similarity heuristic (not EFE), has no ambiguity term, no softmax, no episode-history integration, and is not wired into the orchestrator at all.
- **F.09** Level 3 context engineering — Doc 13 §9 says "Scaffold"; actually effectively DESIGN-ONLY: no agent-mesh crate, no stigmergic accumulation, no HDC live retrieval.

### Priority actions
1. **P1** (F.02): Implement real EFE scoring — add ambiguity variance term, softmax with γ=8.0, episode-history-backed pragmatic value, HDC-based belief change — and wire into `orchestrate.rs` scorer chain.
2. **P2** (F.06): Extend VCG auction with `LearningBidder` + Thompson sampling, alpha-fairness config, and safety-floor hybrid policy.
3. **P2** (F.07): Implement exponential gain curve calibration per task category; add `MultiPatchForager` for source ordering; ship `RetrievalSignal` stigmergic boost.
4. **P3** (F.10–F.12): Build `roko-eval` crate with RAGAS metrics + CLEAR dimensions, then layer CIV and Meta-Harness on top.
5. **Doc fix**: Update `docs/03-composition/13-current-status-and-gaps.md` to correct the 3 stale SCAFFOLD / "Not yet" claims about F.03 (Dominance), F.04 (PAD persistence), and F.05 (Neuro injection) — these are shipped.

---

## Agent Execution Notes

### What Batch 03 Should Actually Own Here

Most of this file should remain bounded or deferred.

Good batch-`03` candidates:

- `F.02` truth-in-advertising around the scorer contract,
- maybe a small naming/API cleanup that keeps later agents from assuming real EFE is already shipped.

Usually defer:

- truthful VCG work,
- calibrated MVT,
- distributed context engineering,
- RAGAS / CLEAR / CIV / Meta-Harness.

### F.02 — Scorer Truth-In-Advertising

Preferred batch outcome:

1. rename or clarify the current scorer so it no longer over-claims active inference,
2. update tests and call sites,
3. leave a precise handoff for real EFE work in `05-learning`.

Acceptance criteria:

- the scorer name and comments match the implementation,
- later agents are not nudged toward the wrong mental model,
- no one can mistake the current heuristic for a live EFE system.
