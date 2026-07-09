# C — Routing + Bandits (Docs 03, 04, 10, 11)

Parity analysis of `docs/05-learning/03-bandits-ucb-thompson-linucb.md`,
`04-cascade-router.md`, `10-pareto-frontier-pruning.md`, `11-thompson-sampling-drift.md`
vs the actual codebase.

---

## C.01 — `UcbBandit` core (UCB1 algorithm)

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 03 §UCB1 — `UcbBandit` implements UCB1 with `mean_a + C · √(ln(total_pulls) / pulls_a)`, default `C = √2`, unpulled arms get `f64::INFINITY`, `parking_lot::RwLock` for arms and `AtomicU64` for total pulls, optional persistence.
**Reality**: `crates/roko-learn/src/bandits.rs:284-291` — exact struct: `arms: RwLock<Vec<BanditArm>>`, `total_pulls: AtomicU64`, `exploration_c: f64` (set to `std::f64::consts::SQRT_2` in `new` at line 309), `persist_path: Option<PathBuf>`. `select()` at line 408, `update(arm, reward)` at line 435, `save()` at 376, `load()` at 336. `BanditArm` struct at `bandits.rs:76-83` with `name`, `pulls`, `total_reward`. 1727 LOC file with 25+ tests.

---

## C.02 — `BanditBank` keyed collection of UCB1 instances

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 03 §BanditBank — collection of independent `UcbBandit` instances keyed by context string; lazy creation on first `select(key, ...)`; persists as single JSON.
**Reality**: `bandits.rs:493-497` — `BanditBank { bandits: RwLock<HashMap<String, UcbBandit>>, arm_names, exploration_c }`. Lazy init in `select()` at line 511 (fast-path read lock, slow-path write + re-check). `update()` at 532. `save()` at 555 serializes as `BankSnapshot { bandits: HashMap<String, Vec<BanditArm>> }` (line 485). `load()` at 591.

---

## C.03 — `TrackAndStopBandit` (Garivier & Kaufmann 2016)

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 03 §Track-and-Stop — best-arm identification with anytime-valid GLR stopping; `β(t, δ) = ln((ln(t)+1)/δ)`; round-robin → D-tracking → stop; implements `FormatBandit` trait for `(model, role, tool_count, complexity)` keys; default `δ = 0.05`.
**Reality**: `bandits.rs:748-766` — `TrackAndStopBandit { profile_of, delta, state: RwLock<HashMap<BanditKey, TasState>> }`. `new()` sets `delta: 0.05` (line 764). `threshold(t)` at line 793 computes `β(t, δ) = ln(1/δ) + 3·ln(ln(max(t, e)))` — note the doc formula differs slightly (`ln((ln(t)+1)/δ)` vs `ln(1/δ) + 3·ln(ln(max(t,e)))`), both valid Track-and-Stop thresholds but not identical. `glr_statistic()` at line 802 implements the `Z(t) = min_{b ≠ a*} W_{a*,b}(t)` form documented in code comments line 740. `impl FormatBandit for TrackAndStopBandit` at line 922. `is_stopped()` at 1022, `stopped_winner()` at 1031.
**Notes**: Doc simplifies the threshold formula. Code uses the more standard 3·ln(ln(·)) correction term.

---

## C.04 — `EwcRegularizer` (Elastic Weight Consolidation)

**Status**: DONE
**Severity**: —
**Doc claim**: Not mentioned in doc 03 (or any target doc).
**Reality**: `bandits.rs:115-131` — `EwcRegularizer { lambda, fisher_diagonal, anchor_theta, preservation_events, last_penalty }`. `new(dim)` at line 148 sizes the regularizer; `penalty(theta)` at 200 computes the EWC quadratic loss; `regularize_reward(theta, reward)` at 222 dampens reward updates that deviate from the anchor. Used to prevent catastrophic forgetting in the contextual routing arms.
**Fix sketch**: Doc 03 should add a short section describing EWC as a catastrophic-forgetting guard, or move it to doc 05/07 if those docs better match.

---

## C.05 — `FormatBandit` trait + `TrackAndStopBandit` impl for tool-format selection

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 03 §Use Case — `FormatBandit` trait with `select_format(key) -> ToolFormat` and `update_format(key, format, outcome)`; used for per-`(model, role, tool_count, complexity)` tool-format selection.
**Reality**: Trait lives in **`roko-core`**, not `roko-learn`: `crates/roko-core/src/tool/bandit.rs:139-153`. Signature is `fn select(&self, key: &BanditKey) -> ToolFormat` / `fn feedback(&self, key, chosen, outcome)` / `fn arm_table(&self, key)` / `fn name()` — **not** `select_format`/`update_format` as the doc snippet shows. `impl FormatBandit for TrackAndStopBandit` at `bandits.rs:922`.
**Fix sketch**: Update doc 03 §Use Case to match actual method names (`select`/`feedback`/`arm_table`/`name`) and note that the trait is defined in `roko-core::tool::bandit`, not `roko-learn`.

---

## C.06 — `LinUCBRouter` with 18-dim `RoutingContext`

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 03 §LinUCB — `score(a) = θ_a^T · x + α · √(x^T · A_a^{-1} · x)`; 18-dim context; α decays from 1.0 to 0.05 via `α = 0.05 + 0.95 · exp(−observations / 60)`; `COLD_START_THRESHOLD = 50`; `CONTEXT_DIM = 18`.
**Reality**:
- Constants at `crates/roko-learn/src/model_router.rs:60-74`: `CONTEXT_DIM: usize = 18`, `COLD_START_THRESHOLD: u64 = 50`, `ALPHA_MIN: 0.05`, `ALPHA_MAX: 1.0`, `ALPHA_TAU: 60.0`.
- `RoutingContext` at `model_router.rs:129-159` — 14+ fields including `task_category`, `complexity`, `iteration`, `role`, `crate_familiarity`, `has_prior_failure`, `previous_model`, `plan_context_tokens`, and conductor-load fields (`conductor_load`, `active_agents`, `ready_queue_depth`, `max_queue_wait_hours`, `daimon_policy`).
- `to_features_for_model()` at line 170-214 encodes one-hot TaskCategory (8 dims), complexity scalar, iteration/10 capped, 4-dim role hash, crate familiarity, prior-failure binary, bias term, cache affinity — totalling 18 as doc specifies.
- `LinUCBRouter` struct at `model_router.rs:655-661`: `state: RwLock<RouterState>`, `persist_path`, `static_table: HashMap<ModelTier, String>`. `new()` at 676 enforces non-empty arms.
- `select_model()` at line 734 checks cold start at line 738 (`total_observations < COLD_START_THRESHOLD`).
- 2170 LOC file with 42 tests.

---

## C.07 — `ThompsonArm` single-arm Beta posterior with discount

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 11 §Implementation Design — `ThompsonArm { model, alpha, beta, total_observations }`; update: `α ← γ·α + reward`, `β ← γ·β + (1 − reward)`; Beta(1,1) prior; default γ ≈ 0.995 for effective window ~200.
**Reality**: `model_router.rs:449-464` — `ThompsonArm { slug, alpha, beta, sum_reward, sum_reward_sq, observations, discount }`. `new()` at 469 initializes `alpha=1.0, beta=1.0, discount=THOMPSON_DEFAULT_DISCOUNT` which is **`0.99`** (line 76, not the doc's recommended 0.995). Fields differ slightly: struct uses `slug` (not `model`), has extra `sum_reward`/`sum_reward_sq` for future continuous variants, `observations` (not `total_observations`).
- `update(reward, success)` at line 489: `self.alpha = 1.0 + self.discount * (self.alpha - 1.0)` which is equivalent to the doc's `γ·α + reward` only when the prior offset (1.0) is factored out — the code explicitly preserves the Beta(1,1) prior under discounting, which the doc does **not** describe.
- `sample()` at 483 uses `parking_lot` no-op + `rand::thread_rng()`; `sample_beta()` helper at line 524 implements the doc's fallback `max(0.01)` concept via `max(f64::MIN_POSITIVE)`.
**Fix sketch**: Doc 11 §Implementation Design should (a) update default discount to 0.99 matching `THOMPSON_DEFAULT_DISCOUNT`, (b) include the prior-preserving discount formula `α ← 1 + γ·(α − 1) + reward` rather than `α ← γ·α + reward`, (c) mention the `sum_reward`/`sum_reward_sq` accumulators for future continuous variants.

---

## C.08 — Contextual Thompson Sampling variant

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Doc 11 §Contextual Thompson Sampling — Bayesian analogue to LinUCB with per-arm posterior over weight vector θ_a ∼ N(μ_0, Σ_0); Bayesian linear regression updates; stochastic sampling instead of UCB bound.
**Reality**: `rg 'ContextualThompson' crates/` returns zero matches. The `ThompsonArm` is strictly single-arm Beta (no context vector input). No posterior covariance Σ, no μ posterior over a weight vector, no context-scored Thompson selection anywhere in `roko-learn`.
**Fix sketch**: Either (a) mark doc 11 §Contextual Thompson explicitly as "Design Only — not yet implemented", or (b) add a new struct `ContextualThompsonRouter` alongside `LinUCBRouter` that maintains per-arm `(mu: Vec<f64>, sigma: Vec<Vec<f64>>)` and samples weight vectors via Cholesky factorization.

---

## C.09 — NeuralUCB (Zhou et al. 2020)

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Doc 03 §Neural Contextual Bandits — `NeuralUCBRouter { network, gradient_covariance, nu, lambda, training_buffer, retrain_interval }` with `NeuralRewardNet { input_dim: 18, hidden_dims: [64, 32] }`; exploration bonus `ν × √(g^T · Z_a^{-1} · g)` where g = ∇_θ f(x; θ); retrain every 50 observations.
**Reality**: `rg 'NeuralUCB|NeuralRewardNet|gradient_covariance|retrain_interval' crates/` returns zero matches. No neural network training code, no autograd dependency, no `NeuralRewardNet` struct, no `nu` parameter. The only router implementation is linear (`LinUCBRouter`).
**Fix sketch**: Doc 03 §Neural Contextual Bandits is pure design-only prose. Add an explicit "**Status: Design Only**" header to the section, or move to `docs/05-learning/99-future-work.md` until prototyped. The recommendation line 311 ("Use LinUCB until 500+ observations accumulate") correctly positions it as future work but the surrounding section reads as shipped.

---

## C.10 — `BanditEnsemble` (meta-bandit strategy selection)

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Doc 03 §Bandit Ensembles — `BanditEnsemble { strategies, meta_bandit, strategy_stats, correlation_matrix, mode }`; modes `MetaSelect | WeightedVote | MajorityVote | AdaptiveSwitch`; tracks per-strategy regret.
**Reality**: `rg 'BanditEnsemble|EnsembleMode|BanditStrategy|StrategyStats|AdaptiveSwitch' crates/` returns zero matches. No meta-bandit over strategies, no correlation-matrix tracking, no strategy-switching logic exists. The router is a single fixed pipeline: Static → Confidence → LinUCB.
**Fix sketch**: Same treatment as C.09 — label doc 03 §Bandit Ensembles as design-only, or relocate to a future-work document. Implementation would require a strategy-enum wrapper around `UcbBandit`, `LinUCBRouter`, `TrackAndStopBandit`, `ThompsonArm` with a `BanditStrategy` trait.

---

## C.11 — Three-stage `CascadeRouter` (Static → Confidence → UCB)

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 04 §Three-Stage Cascade — stage 1 (static, <50 obs), stage 2 (confidence, 50–200 obs), stage 3 (LinUCB, >200 obs); `CONFIDENCE_TO_UCB_THRESHOLD = 200`; `CACHE_AFFINITY_BONUS = 0.15`; `PARETO_RECOMPUTE_INTERVAL = 50`.
**Reality**:
- `crates/roko-learn/src/cascade_router.rs:994-1009` — `CascadeRouter { linucb: LinUCBRouter, confidence_stats, pareto_frontier, role_table, model_slugs, stage_tracking, free_tier_shadow_runner }`. 4766 LOC file with 59 tests.
- `CascadeStage` enum at line 63: `Static | Confidence | Ucb` with `label()` returning `"static" | "confidence" | "ucb"`.
- Thresholds at lines 237-249: `CONFIDENCE_TO_UCB_THRESHOLD: 200`, `CACHE_AFFINITY_BONUS: 0.15`, `PARETO_RECOMPUTE_INTERVAL: 50`, plus additional constants not in doc: `LOW_AFFECT_CONFIDENCE_THRESHOLD: 0.3`, `HIGH_CFACTOR_THRESHOLD: 0.8`, `LOW_CFACTOR_THRESHOLD: 0.4`, `HYSTERESIS_THRESHOLD: 0.10`.
- Stage transition logic at line 3116 (`} else if obs < CONFIDENCE_TO_UCB_THRESHOLD {`).
- Pareto recompute bucketing at line 2740-2744 uses `total / PARETO_RECOMPUTE_INTERVAL` for cache keying.

---

## C.12 — `CascadeModel` primary + fallback routing output

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 04 §CascadeModel Output — `CascadeModel { primary: ModelSpec, fallback: Option<ModelSpec>, latency_sla_ms: u64, stage: CascadeStage }`; `fallback` is used by orchestrator on retry.
**Reality**: `cascade_router.rs:107-118` — struct has **four** fields not shown in the doc: `primary`, `fallback_chain: Vec<ModelSpec>` (not a single `Option<ModelSpec>`), `context_overflow_fallback: Option<ModelSpec>` (separate escalation path for context-overflow errors), `latency_sla_ms`, `stage`. `model_for_attempt(attempt)` at line 126 walks the fallback chain by index. `fallback_for_error(error)` at line 135 dispatches by `ProviderError` variant (ContextOverflow → `context_overflow_fallback`; RateLimit → backend-diverse fallback; else → first chain entry).
**Fix sketch**: Doc 04 §CascadeModel Output should (a) change `fallback: Option<ModelSpec>` to `fallback_chain: Vec<ModelSpec>`, (b) document the separate `context_overflow_fallback` field, (c) describe `model_for_attempt()` and `fallback_for_error()` helpers as the public API.

---

## C.13 — `StageTransition` history + `ShadowModelRunner` trait

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 04 §Three-Stage Cascade (transition recording implicit). Doc does not mention `ShadowModelRunner`.
**Reality**:
- `StageTransition` at `cascade_router.rs:92-101`: `{ from, to, observations, timestamp }` with `DateTime<Utc>`. Tracked in `StageTracking { current, transitions: Vec<StageTransition> }` at line 1019.
- `ShadowModelRunner` trait at line 54: `async fn run_shadow(&self, prompt: &str, model_slug: &str) -> AgentResult`. Wired via `CascadeRouter::free_tier_shadow_runner` (line 1008) + `with_free_tier_shadow_runner()` builder at line 1084. Stub used in tests at line 3322.
**Fix sketch**: Doc 04 should add a "Shadow Evaluation" subsection describing `ShadowModelRunner` as the free-tier Gemini shadow-routing hook.

---

## C.14 — Lookahead router (sequence-aware routing)

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Doc 04 §Cascade Router with Lookahead — `LookaheadRouter { inner, task_graph, horizon, gamma, cache_model }`; `CacheReuseModel { cache_hit_rates, avg_tokens_saved_per_hit, cache_read_discount }`; optimizes routing across upcoming task window.
**Reality**: `rg 'LookaheadRouter|CacheReuseModel' crates/` returns zero matches. No task-graph-aware routing, no cache-reuse cost model, no lookahead horizon. The router makes fully-myopic decisions per task.
**Fix sketch**: Label doc 04 §Cascade Router with Lookahead as "Design Only — not yet implemented". Reasonable implementation path: build `LookaheadRouter` as a decorator over `CascadeRouter` that queries the `TaskDag` for the next N tasks and scores each candidate against expected cache reuse savings.

---

## C.15 — Cost-spectrum router (CSCR)

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Doc 04 §Cost-Spectrum Routing — `CostSpectrumRouter { encoder: ContrastiveEncoder, model_descriptors, cost_band, band_adaptation }`; `CostBand { min_cost, max_cost, target_cost }`; `BandAdaptation { widen_threshold, narrow_threshold, step_size }`; `ModelDescriptor { features: [f64; 4], provider, supports_reasoning }`; continuous cost-quality tradeoff via contrastive similarity.
**Reality**: `rg 'CostSpectrumRouter|CostBand|ContrastiveEncoder|ModelDescriptor|BandAdaptation' crates/roko-learn` returns zero matches. No continuous cost band, no contrastive encoder, no model descriptor feature vectors. Cost adjustment lives only as a simple `cost_penalty` term in confidence-stage scoring.
**Fix sketch**: Label doc 04 §Cost-Spectrum Routing as "Design Only" or move to future-work. Sketch mentions research citation (CSCR, 2025) but no prototype exists.

---

## C.16 — Router calibration (Platt / Isotonic / Temperature)

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Doc 04 §Router Calibration — `RouterCalibration { calibrations, brier_score, recalibrate_interval }`; `ModelCalibration { predictions, bins: [CalibrationBin; 10], platt_a, platt_b, isotonic_map }`; auto-recalibrate every 100 decisions when ECE > 0.10.
**Reality**: `rg 'RouterCalibration|PlattScaling|IsotonicRegression|CalibrationBin|brier_score' crates/` returns zero matches. No calibration infrastructure attached to the router. The cascade router reports raw `score` values with no probabilistic interpretation applied.
**Notes**: A separate `CalibrationTracker` exists in the `roko-golem` / foraging subsystem (see doc 16), but that is not wired to router output scores.
**Fix sketch**: Label doc 04 §Router Calibration as "Design Only". If prioritized, implementation is small (~200 LOC) since Platt scaling is a 2-parameter logistic regression; could be added as a post-filter over `CascadeRouter::select()` output.

---

## C.17 — `compute_pareto_frontier` 2D (pass_rate × cost_per_success)

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 10 §Algorithm — `compute_pareto_frontier(stats: &HashMap<String, ModelObservation>) -> Vec<String>`; O(n²) with "at least as good on both and strictly better on one" dominance; sorted frontier output; `ModelObservation { pass_rate, cost_per_success, avg_latency_ms, observations }`.
**Reality**:
- `crates/roko-learn/src/pareto.rs:28-47` — function body matches the doc pseudocode **byte-for-byte** (modulo formatting): `obs_b.pass_rate >= obs_a.pass_rate && obs_b.cost_per_success <= obs_a.cost_per_success && (obs_b.pass_rate > obs_a.pass_rate || obs_b.cost_per_success < obs_a.cost_per_success)`. Final `frontier.sort()` present on line 45.
- `ModelObservation` at `pareto.rs:12-21` has exactly the 4 fields the doc shows: `pass_rate`, `cost_per_success`, `avg_latency_ms`, `observations`.
- 89 LOC file, 1 test (`pareto_frontier_keeps_non_dominated_models`) at line 55 covers the 3-model example from the doc.
**Notes**: The `avg_latency_ms` field is tracked but unused in dominance — doc 10 line 87 acknowledges this.

---

## C.18 — Multi-objective Pareto extension (4-dim)

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Doc 10 §Multi-Objective Extension — extend to 4 dims `(quality, cost, latency, reliability)` via scalarization with configurable weights; preserves O(n²) complexity.
**Reality**: Only 2 dimensions are used (pass_rate + cost_per_success). The `avg_latency_ms` field exists in `ModelObservation` but is **not** read in `compute_pareto_frontier` (`pareto.rs:32-38` — only `pass_rate` and `cost_per_success` are compared). No reliability/error-rate field on `ModelObservation`. No `ParetoWeights` struct, no scalarization helper.
**Fix sketch**: Either label doc 10 §Multi-Objective Extension as "Design Only", or wire the 4-dim extension by (a) adding `error_rate: f64` to `ModelObservation`, (b) introducing `ParetoWeights { quality, cost, latency, reliability }`, (c) generalizing the dominance check to the weighted/lexicographic case, (d) exposing via `compute_pareto_frontier_weighted()`.

---

## C.19 — `LearningRateSchedule` (phase-aware alpha modulation)

**Status**: DONE
**Severity**: —
**Doc claim**: Not mentioned in target docs.
**Reality**: `model_router.rs:80-91` — `LearningRateSchedule { cold_rate, warm_rate, mature_rate, cold_threshold, warm_threshold }`. Default at line 93: `(1.0, 0.85, 0.7, 25, 100)`. `multiplier_for_observations()` at line 108; `alpha_for_observations()` at 120 composes the phase multiplier with the exponential decay. Used to modulate LinUCB exploration across cold/warm/mature phases.
**Fix sketch**: Doc 03 §Alpha Decay mentions the exponential decay formula but not the phase-aware schedule. Add a short subsection describing `LearningRateSchedule` and its three-phase multiplier so users understand observed alpha values differ from the pure `0.05 + 0.95 · exp(−obs/60)` curve.

---

## C.20 — `BeliefState` + `select_tier` (active inference for tier routing)

**Status**: DONE
**Severity**: —
**Doc claim**: Not mentioned in target docs (03/04/10/11).
**Reality**: `crates/roko-learn/src/active_inference.rs:17-22` — `BeliefState { probabilities: Vec<f64>, updates: u64 }` over 90 flattened states (`STATE_COUNT = 90` at line 11, factorized as `SKILL_LEVELS (3) × difficulty (3) × CONFIDENCE_LEVELS (10)`). `BeliefState::observe()` at line 48 does Bayesian update with success likelihood + cost/latency penalties. `select_tier(belief, requirements) -> ModelTier` at line 83 minimizes expected free energy across `ModelTier::{Fast, Standard, Premium}`. 255 LOC, 3 tests.
**Notes**: This is a standalone tier-selection mechanism that parallels (but is not wired into) the cascade router's routing decision. Whether it should complement or replace `CascadeRouter` at any stage is unclear from the existing code.
**Fix sketch**: Add a new doc `docs/05-learning/XX-active-inference.md` describing `BeliefState` + expected-free-energy tier selection, or integrate into doc 04 as a proposed pre-cascade tier filter.

---

## Section Summary

| Status | Count |
|--------|-------|
| DONE | 13 (C.01, C.02, C.03, C.04, C.05, C.06, C.07, C.11, C.12, C.13, C.17, C.19, C.20) |
| PARTIAL | 0 |
| NOT DONE | 7 (C.08, C.09, C.10, C.14, C.15, C.16, C.18) |
| SCAFFOLD | 0 |

(Counts: DONE = 13, NOT DONE = 7, total = 20 items. Original plan was C.01–C.18; two extras (C.19 `LearningRateSchedule`, C.20 `BeliefState`) surfaced during code inspection.)

### Headline findings

1. **Core routing stack is fully shipped**: `UcbBandit`, `BanditBank`, `TrackAndStopBandit`, `LinUCBRouter`, `CascadeRouter`, `compute_pareto_frontier`, and `ThompsonArm` all exist with matching semantics, thresholds, and persistence. 9007 LOC total across the 5 source files with 130+ tests.
2. **All "advanced" sections in docs 03 and 04 are design-only**: `NeuralUCB`, `BanditEnsemble`, `LookaheadRouter`, `CostSpectrumRouter`, `RouterCalibration`, contextual Thompson, and 4-dim Pareto have zero code. Each needs either a `**Status: Design Only**` banner or relocation to a future-work document.
3. **`CascadeModel` shape drifted from docs**: Code has `fallback_chain: Vec<ModelSpec>` + separate `context_overflow_fallback: Option<ModelSpec>`; doc shows single `fallback: Option<ModelSpec>` (C.12). Non-trivial API surface mismatch.
4. **`FormatBandit` trait lives in `roko-core`**, not `roko-learn` (C.05). Method names in doc (`select_format`/`update_format`) do not match reality (`select`/`feedback`/`arm_table`/`name`).
5. **`ThompsonArm` discount formula drift** (C.07): code uses prior-preserving form `α ← 1 + γ·(α − 1) + reward`, doc shows plain `α ← γ·α + reward`. Default `γ = 0.99`, doc recommends 0.995.
6. **Two shipped pieces missing from target docs**: `EwcRegularizer` (C.04), `LearningRateSchedule` (C.19), `BeliefState`+`select_tier` (C.20). Each deserves a mention or its own section.
7. **Orchestrate.rs routing wiring confirmed**: `cascade_router.select_for_frequency_among` at `orchestrate.rs:9886`, observation feedback via `observe_cascade_router` at `:7017-7042`, routing log emission at `:10105-10109`.

## Agent Execution Notes

### C.01-C.07 / C.11-C.13 — Treat As The Shipped Routing Core

Most of the high-value `05` routing work should build on these surfaces, not replace them.

Recommended slice for routing-adjacent batches:

1. strengthen the current cascade / budget / calibration contract,
2. keep routing-log and stage behavior inspectable,
3. avoid widening into new routing algorithms.

Acceptance criteria:

- runtime behavior is clearer and more actionable,
- existing routers remain the source of truth,
- the batch does not turn into `NeuralUCB` or ensemble research.

### C.08-C.10 / C.14-C.18 — Defer By Default

Contextual Thompson, NeuralUCB, bandit ensembles, lookahead routing, cost-spectrum routing, calibration stacks, and 4D Pareto are all valid future work, but they are not current batch-`05` ownership unless a later pass explicitly re-scopes them.
