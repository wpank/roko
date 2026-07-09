# roko-learn — Episodes, Routing, Experiments, Feedback Loops

> Status-quo audit · verified 2026-07-08 (re-verified against HEAD `5852c93c05`, `main`) · supersedes 2026-07-07 rev · sources: 66 src modules + 5 integration-test files in `crates/roko-learn/`, ~30 call-site files read across roko-cli/serve/compose/agent/orchestrator/acp/dreams/neuro/conductor, git commits, live `.roko/learn/` data (34 dir entries), docs/v1/05-learning (21 docs), docs/v2/07-LEARNING.md, docs/v2-depth/10-learning-loops (11 docs), tmp/tmp-feedback/2 (19/34 ACP gap docs), sibling audits 18/22/35/36/37/39.

> **2026-07-08 deep second pass (HEAD `5852c93c05`) — new mechanics surfaced:** added the CascadeRouter internals deep-dive (§ "CascadeRouter internals"), episode schema table (§ "Episode schema"), and one **material new defect**: **LinUCB A/b matrices are NOT persisted.** `CascadeRouter::snapshot()` hardcodes `linucb_state: None` (`cascade_router.rs:1795`, comment "LinUCB export methods don't exist yet") and `load_from` destructures `linucb_state: _linucb_state` — **discarded** (`:1832`). Only `confidence_stats` + `total_observations` round-trip. Consequence: after restart a router with `total_observations > 200` **stays in UCB stage** (`set_total_observations`, `:1878`) but every arm resets to identity-A / zero-b → the contextual bandit is effectively **untrained and exploration-dominated** until it re-accumulates observations. The `LinUCBSnapshot` type (`cascade/persistence.rs:14-24`) is dead schema. **New P1.** This subsumes prior notes that the router "persists at shutdown" — it persists *stage + confidence stats*, not the LinUCB weights.

> **2026-07-08 re-verify delta (what changed this pass):** (1) confirmed `gate-thresholds.json` still **absent on disk** and the reserved `gate_thresholds_every_n` cadence (`runtime_feedback.rs:234`) has **zero consumers workspace-wide** — it is dead config, not merely "reserved" [P0 unchanged]. (2) Resolved Open-Q #1: Runner v2 **does** consult knowledge routing, but via a **divergent, thinner mechanism** than legacy orchestrate.rs — a manual score-nudge over `dispatch_plan.model` (`runner/event_loop.rs:4231-4340`), NOT the router's `select_for_frequency_among_with_knowledge`. Two knowledge-routing implementations now coexist [new P1 drift]. (3) **Newly documented third dispatch path: roko-acp.** ACP records cascade-router observations (`bridge_events.rs:700-720`) but never selects from the router and hardcodes `DaimonPolicy::default()` (`:634`) — learning is **write-only** in ACP; no prompt experiments, no `record_completed_run` [new P1 drift, cross-cutting]. (4) 11 stale `cascade-router.json.tmp.*` files still present on disk (10 zero-byte). (5) `roko learn tune` re-confirmed display-only (`commands/learn.rs:64-110`; `--dry-run` prints a no-op line at :106-108).

Legend: ✅ wired (runtime path) · 🔌 built-not-wired · 🟡 partial · ❌ missing · 🕰️ legacy/deleted.

## Summary

`roko-learn` is the most genuinely-wired learning stack in the workspace — not a stub, and materially better than both CLAUDE.md and the concise draft describe. The central hook is `LearningRuntime::record_completed_run` (`crates/roko-learn/src/runtime_feedback.rs:2321`), called from the legacy orchestrator (`orchestrate.rs:12134,14896`), `roko run` (`run.rs:2837`), `agent_exec.rs:316`, `commands/util.rs:1825`, and `bench.rs:698`. It fans one completed episode into ~20 subsystems (episodes, costs, latency, cascade-router observations, prompt experiments, skill mining, pattern discovery, section effectiveness, post-gate reflections, local rewards, provider health/outcomes, regression detection, WAL) on cadences defined by `UpdateFrequency` (`runtime_feedback.rs:230-243`). Runner v2 has a parallel, thinner path: `FeedbackFacade` + sinks (`roko-cli/src/runtime_feedback/mod.rs:157`; wired at `commands/plan.rs:472-484`, `commands/do_cmd.rs:506-514`) plus a learning `EventBus` with `run_learning_subscriber` (`runner/event_loop.rs:742,766`).

Two CLAUDE.md "remaining work" items are stale: **knowledge-informed routing (item 13) is wired** (`knowledge_helpers.rs:531` → `orchestrate.rs:15658` → `cascade_router.rs:404`), and **force_backend override learning UX34 (item 15) is wired** (`cascade_router.rs:1262` ← `roko-agent/src/model_call_service.rs:529,1886,1928,1986`, e2e-tested). All 8 of v1's "missing feedback loops" now close in code (with caveats below). The main open wounds: adaptive gate thresholds persist **only at graceful shutdown** (`orchestrate.rs:5947-5959`; `gate-thresholds.json` absent on disk — confirms 35-GATES), episodes are written to three roots, `roko learn tune` doesn't actually tune, and a cluster of v2-vocabulary modules (oracles, quality_judge, bayesian_confidence, error_enrichment, heuristics scoring) are shells or orphans. 12 modules (~6.9K LOC) were deliberately deleted in May 2026 (T2-16/T2-17); a further 4-module deletion (T2-17b) sits unmerged on wp-arch2 and would remove live code if merged.

## Current state table

| Component | Design source | Code | Status | Evidence |
|---|---|---|---|---|
| EpisodeLogger (bespoke JSONL, not Signals) | v1 00-episode-logger | `episode_logger.rs` | ✅ | Append-only mutex-serialized JSONL; `Episode` is its own struct — `Signal` used only for template-suggestion fingerprints (`episode_logger.rs:598,733`) |
| Episode HDC fingerprint | v1 00 / CLAUDE table | `hdc_fingerprint.rs:16` | ✅ | Set in both paths: `orchestrate.rs:3185-3186`, Runner v2 `roko-cli/src/runtime_feedback/episodes.rs:121-122`; historical on-disk rows are `null` (pre-wiring data) |
| LearningRuntime (per-episode fan-out) | v2 07-LEARNING "LearningRuntime" | `runtime_feedback.rs:2321` | ✅ | 6 external call sites; `LearningPaths::under` (`runtime_feedback.rs:176-206`) = canonical 22-file layout under `.roko/learn/` |
| Runner v2 feedback | v2 04-EXECUTION | `roko-cli/src/runtime_feedback/mod.rs:157` | ✅ | `FeedbackFacade` + EpisodeSink/RoutingObservationSink/KnowledgeIngestionSink/NeuroKnowledgeIngestor (`commands/plan.rs:464-484`); learning EventBus (`runner/event_loop.rs:742,766,1139,2557-2583`) |
| CascadeRouter (3-stage + LinUCB) | v1 04-cascade-router | `cascade_router.rs`, `cascade/`, `model_router.rs` | ✅ | Persisted `.roko/learn/cascade-router.json` (on disk); saved at shutdown `orchestrate.rs:5964` and immediately in `record_completed_run` (test `runtime_feedback.rs:4748`) |
| Knowledge-informed routing (legacy orchestrate) | v2-depth bandit-routing | `cascade_router.rs:404-432` | ✅ | `build_knowledge_routing_advice` (`knowledge_helpers.rs:531`) → `select_for_frequency_among_with_knowledge` at `orchestrate.rs:15658` → `route_with_knowledge_among` (`cascade_router.rs:977`). Knowledge biases scores **inside** the router. **CLAUDE.md item 13 stale (done)** |
| Knowledge-informed routing (Runner v2) | — | `runner/event_loop.rs:4231-4340` | 🟡 divergent | Runner v2 builds `knowledge_advice` from `KnowledgeStore` then **manually nudges** `dispatch_plan.model` (best hint if `score + bias_weight > baseline_score`, `:4311-4340`); gated by `task.model_hint.is_none() && !dispatch_plan.forced`. Does **NOT** call `select_for_frequency_among_with_knowledge` — a second, thinner knowledge-routing mechanism. Resolves prior Open-Q #1 |
| Knowledge-informed routing (ACP) | — | `crates/roko-acp/src/bridge_events.rs` | ❌ | No knowledge routing; ACP never selects from router (see ACP row) |
| **ACP learning path (3rd dispatch surface)** | tmp-feedback/2 docs 19/34 | `roko-acp/src/bridge_events.rs` | 🟡 write-only | CascadeRouter loaded per-observation (`:700 load_or_new`) → `router.observe(...)` (`:712`) + `save` (`:714`); **selection never invoked** (no `select_*`/`route_*`). `DaimonPolicy::default()` hardcoded (`:634`). No prompt experiments, no `record_completed_run`/LearningRuntime fan-out. ACP writes learning it never reads back |
| force_backend override learning (UX34) | CLAUDE item 15 | `cascade_router.rs:1255-1287` | ✅ | `record_override_outcome`, dampened `OVERRIDE_LEARNING_RATE=0.5` (`cascade/types.rs:260`); caller `model_call_service.rs:529` at 3 outcome points; e2e `roko-cli/tests/dispatch_feedback_projection_e2e.rs:223`. **CLAUDE.md item 15 stale** |
| Provider health circuit breaker | v1 09 | `provider_health.rs` | ✅ | Filters candidates inside router (`cascade_router.rs:743`); `provider-health.json` on disk; also fed from direct model calls via `model_call_feedback.rs` (chat.rs, dispatch_v2.rs, serve dispatch.rs, vision_loop/evaluator.rs) |
| Latency registry → reward | v1 03/08 | `latency.rs`, `compute_routing_reward_v2` (`model_router.rs`) | ✅ | `latency-stats.json` on disk; reward v2 consumed in `runtime_feedback.rs:34` |
| Bandits (UCB/Thompson/LinUCB) | v1 03-bandits | `bandits.rs`, `model_router.rs` | 🟡 | LinUCB live via model_router/cascade; `UcbBandit` used by serve gateway; `contextual_bandit.rs` test-only (`roko-cli/tests/phase0_wiring.rs:261`) |
| Prompt experiments (A/B) | v1 / CLAUDE | `prompt_experiment.rs` | ✅ | `experiments.json` + `experiment-winners.json` on disk (`runtime_feedback.rs:196-197`); `DashboardEvent::ExperimentWinnersUpdated` (`orchestrate.rs:12172`); serve metrics (`routes/status/metrics.rs:23,651,1141`) |
| Model experiments CLI | v1 | `model_experiment.rs` | ✅ | `roko experiment` (`commands/experiment.rs:84,141,213`) |
| Efficiency events + summaries | CLAUDE | `efficiency.rs`, `aggregate.rs` | ✅ | `efficiency.jsonl`/`efficiency-summaries.jsonl` on disk; trends in TUI (`tui/dashboard.rs`, `tui/state.rs:1200-1202`) |
| Adaptive gate thresholds persistence | v1 07 / CLAUDE | roko-gate `AdaptiveThresholds` (`orchestrate.rs:92`) | 🟡 P0 | **Sole writer = `PlanRunner::shutdown`** (`orchestrate.rs:5947-5959`, confirmed 07-08: only `adaptive_thresholds.save` call in file). In-memory EMA updates every task (`observe_pipeline` `:17801`, `override_for_role` `:18437`, `threshold_for` `:18445`) but never flushed until graceful teardown → crash/kill loses all. `LearningPaths.gate_thresholds_json` + `gate_thresholds_every_n=5` cadence (`runtime_feedback.rs:198,234,276`) declared but have **zero consumers workspace-wide** (verified `rg gate_thresholds_every_n` → only defaults/tests) = dead config. **File absent on live disk** while serve exposes read routes (`roko-serve/src/routes/learning/mod.rs:47-48`). Confirms 35-GATES |
| Section effectiveness → prompt weights | v1 loop 3 | `section_effect.rs` | ✅ | `SectionEffectivenessRegistry` in `system_prompt_builder.rs:47,347`, `context_provider.rs:29,448-468` (LearningAttention), `role_prompts.rs:403+`; `section-effects.json` on disk |
| Skill library (Voyager) | v1 02 | `skill_library.rs` (2,754 LOC) | ✅ | orchestrate.rs:137 + `system_prompt_builder.rs:48`; `skills.json` on disk; mining driven by LearningRuntime cadence |
| Playbooks + rules | v1 01 | `playbook.rs`, `playbook_rules.rs` | ✅ | orchestrate.rs:126; `.roko/learn/playbooks/` on disk; queried at dispatch (CLAUDE table) |
| Pattern discovery (trigram) | v1 05 | `pattern_discovery.rs` | ✅ | Driven internally by LearningRuntime (`runtime_feedback.rs:35-37`, `pattern_discovery_every_n`); also 5 external imports |
| Regression detection | v1 07 | `regression.rs` | ✅ | `detect_regressions` inside LearningRuntime (`runtime_feedback.rs:49,211-216`) + 2 external refs |
| Cost normalization / DB / table | v1 08 | `costs_db.rs`, `costs_log.rs`, `cost_table.rs` | ✅ | `costs.jsonl` on disk; `CostTable` in main.rs/chat_inline.rs |
| Budget guardrail (Cost→Routing) | v1 loop 6 | `budget.rs` | ✅ | `use roko_learn::budget::{BudgetAction, BudgetGuardrail}` in orchestrate.rs (pre-dispatch enforcement) |
| Conductor intervention bandit | v1 loop 2 adj. | `conductor.rs` | ✅ | orchestrate.rs + `roko-conductor/src/interventions.rs` |
| Curriculum scheduler | v1 16-predictive-foraging adj. | `curriculum.rs` | ✅ | `CurriculumMode, CurriculumScheduler` imported by orchestrate.rs |
| Prediction / calibration tracker | v1 15-collective-calibration | `prediction.rs` | 🟡 | `CalibrationTracker` + `PredictionRecord::register` (`orchestrate.rs:23286`); single-process only — no *collective* (cross-agent) calibration |
| Calibration policy (bus predict-publish-correct, LEARN-09) | v2 07 §2 | `calibration_policy.rs` | 🟡 | Internal-only (2 refs, via `event_subscriber`); v2 pattern exists in miniature, not per-Cell |
| Routing log / explainability | v2-depth | `routing_log.rs` | ✅ | Imported by orchestrate.rs (append-only audit log) |
| Routing extras (router calibration) | v1 04 | `routing_extras.rs` | ✅ | `RouterCalibration` fields in PlanRunner (`orchestrate.rs:2748,4687,4922,5150`) |
| Anomaly detection | v1 07 adj. | `anomaly.rs` | ✅ | `dispatch.rs:36`, `learning_helpers.rs:15` |
| Pareto frontier | v1 10 | `pareto.rs` | 🟡 | Internal-only (1 ref, router internals); no external consumer |
| WAL crash-safety | v1 14-stability | `wal.rs` | ✅ | Internal to LearningRuntime (`runtime_feedback.rs:52`); `wal.jsonl` on disk |
| FeedbackService (knowledge outcomes) | v2 06-MEMORY | `feedback_service.rs` | ✅ | Writes `knowledge-feedback.jsonl` + `knowledge-scores.json` (`feedback_service.rs:20-21`, both on disk); callers: `service_factory.rs:145`, serve `routes/gateway.rs:1351`, `dispatch_v2.rs:110`, `chat_session.rs:86`, orchestrate.rs:4549/4790/5018 |
| Error pattern store / enrichment | v1 07 | `error_pattern_store.rs` / `error_enrichment.rs` | 🟡 | Store ✅ (orchestrate + `roko-compose/context_provider.rs`); enrichment ❌ 0 refs |
| HDC clustering | v1 05 | `hdc_clustering.rs` | ✅ | `roko-neuro/src/tier_progression.rs:951-955` (k-medoids) |
| Heuristics / worldviews / falsifiers | v1 19 | `heuristics.rs` | 🔌 | Self-described "shells… runtime scoring and calibration left to future work" (`heuristics.rs:1-6`); `falsifier: Option<Predicate>` + `Falsifier` struct (`:252,:313`) exist; only external use is `type Hypothesis = String` alias (`roko-dreams/src/phase2/shared.rs:19`) |
| MAP-Elites | v2-depth 10 (per 18-V2-DEPTH "real") | roko-dreams `phase2/evolution.rs:73,215` | 🔌 | `MapElitesArchive` + evolution pass are real algorithms w/ tests, but live in the dreams phase2 cycle which has **no runtime trigger/cron** (CLAUDE roko-dreams row); not in roko-learn at all |
| Oracles (Chain/Coding/Research) | v2 vocabulary | `oracles/` | ❌ orphan | 0 internal + 0 external refs. The wired "gate rung oracles" are roko-gate's `JudgeOracle`/`SearchOracle` (`orchestrate.rs:90-96,18492`) — different types |
| Quality judge | v1 12 | `quality_judge.rs` | ❌ orphan | 0 refs |
| Bayesian confidence (AS-07) | v2-depth | `bayesian_confidence.rs` | ❌ orphan | 0 refs |
| Active inference tier select | v2 02-CELL EFE | `active_inference.rs` | 🔌 | Shell delegation `select_tier_with_active_inference` (`cascade_router.rs:436-443`, `let _ = self`) |
| `roko learn` CLI | CLAUDE | `commands/learn.rs:30-61` | 🟡 | all/route/experiments/efficiency/episodes work; **`learn tune` is display-only** — `cmd_tune` (`:64-110`) prints JSON, never writes; `--dry-run` is a no-op distinction |
| Serve learning routes | CLAUDE (~85 routes) | `roko-serve/src/routes/learning/mod.rs` | ✅ | `/learning/gate-thresholds`, `/learn/adaptive-thresholds`, cascade-router, cost tiers + `truth_map.rs:208` experiment_winners projection |
| Dream routing advice | 41-DREAMS territory | `.roko/learn/dream-routing-advice.json` | 🟡 | Artifact exists on disk; producer (dreams) has no runtime trigger |
| Tests | — | src + `tests/` | ✅ | 918 `#[test]`/`#[tokio::test]`; integration: `learning_loop.rs`, `cascade_router_integration.rs`, `model_router_integration.rs`, `cost_comparison.rs`, `agent_event_types.rs` |

## Feedback-loop census (closed vs open)

v1's "8 missing loops" (`docs/v1/05-learning/13-8-missing-feedback-loops.md`) — all 8 now **closed in code**, verified independently of the doc's own claims:

| # | Loop | Verdict | Evidence |
|---|---|---|---|
| 1 | Health → Routing | ✅ closed | `health.is_available(provider_id)` in candidate scoring, `cascade_router.rs:743` |
| 2 | Conductor → Routing | ✅ closed (heuristic) | `RoutingContext.conductor_load/active_agents/ready_queue_depth/max_queue_wait_hours` (`model_router.rs:143-151`); no real CPU/mem telemetry |
| 3 | Section → Scaffold | ✅ closed (orchestrator path) | `SectionEffectivenessRegistry` → builder/attention (`system_prompt_builder.rs:347`, `context_provider.rs:448-468`); `section-effects.json` + `section-outcomes.jsonl` live |
| 4 | Failure → Replanning | ✅ closed | `build_gate_failure_plan_revision` + `learning_config.replan_on_gate_failure` (CLAUDE item 11, orchestrate.rs) |
| 5 | Skills → Prompts | ✅ closed | skill sections injected pre-composition (orchestrate.rs:137, `system_prompt_builder.rs:48`) |
| 6 | Cost → Routing | ✅ closed (pre-dispatch) | `BudgetGuardrail`/`BudgetAction` in orchestrate.rs; not yet in-router candidate scoring |
| 7 | Latency → Reward | ✅ closed | latency registries + `compute_routing_reward_v2` in `record_completed_run` |
| 8 | Experiments → Static | ✅ closed (runtime defaults) | winners persisted (`experiment-winners.json`) + router static-table sync; ❌ no materialization back into `roko.toml`/prompt source |

Additional loops beyond the 8:

| Loop | Verdict | Evidence |
|---|---|---|
| Gate verdicts → adaptive thresholds → gate config | 🟡 half-open | EMA updates in-memory per task; **persistence only on graceful shutdown** (`orchestrate.rs:5947-5959`) — crash/kill loses everything; `.roko/learn/gate-thresholds.json` never observed on disk (467 verdicts recorded per 35-GATES) |
| Override → bandit (UX34) | ✅ closed | dampened `observe_multi_objective` (`cascade_router.rs:1278-1285`) |
| Knowledge → routing | ✅ closed | hints bias scores during selection (`cascade_router.rs:398-432`) |
| Knowledge outcome → scores → neuro admission | ✅ closed | `FeedbackService` → `knowledge-scores.json`/`knowledge-feedback.jsonl`; Runner v2 `KnowledgeIngestionSink` → `knowledge-candidates.jsonl` → NeuroKnowledgeIngestor (`commands/plan.rs:482-484`) |
| Shadow evaluation (free-tier Gemini) → router | ✅ closed (opt-in) | `shadow_evaluate` (`cascade_router.rs:1290-1319`) |
| Predict → publish → correct (v2 §2) | 🟡 partial | `PredictionRecord::register` (`orchestrate.rs:23286`) + `CalibrationTracker`; `calibration_policy.rs` bus-joined via `event_subscriber`; not per-Cell as v2 designs |
| Dreams → routing advice | 🔌 open | artifact consumed if present; producer untriggered |
| Collective calibration 31× (v1 doc 15) | ❌ open | single-process calibration only; no cross-agent aggregation |
| Heuristic falsification loop (v1 doc 19) | ❌ open | types only, no runtime trials/Brier updates |
| Efficiency → per-turn enrichment | ✅ closed | efficiency events per turn (orchestrate) + `read_efficiency_events` consumed by `event_subscriber.rs:322` |
| ACP → learning fan-out | 🟡 half-open (write-only) | ACP `router.observe`+`save` (`bridge_events.rs:712-714`) but no selection consumption, no LearningRuntime, no experiments, `DaimonPolicy::default()` (`:634`). Learning written but never read on the ACP surface — a one-directional loop. Also: ACP router observation uses `compute_acp_reward` (`:711`) with an independent reward function, and silently drops observations when `model_index_for_slug` misses (`:702-708`) — possible slug/arm mismatch (tmp-feedback/2 doc 19 §A) |

## CascadeRouter internals (deep dive)

The router (`cascade_router.rs`, wrapping `LinUCBRouter` in `model_router.rs`) is a **3-stage cascade keyed on `total_observations`**, auto-transitioning as data accrues. Stage is a pure function of the observation count (`stage_for_observations`; thresholds `COLD_START_THRESHOLD=50`, `CONFIDENCE_TO_UCB_THRESHOLD=200`, `cascade/types.rs:247`):

| Stage | Obs | Selection math | Entry point |
|---|---|---|---|
| **1 Static** | `< 50` | Role→model lookup in `role_table` (`default_role_model_table`); no learning input — pure hardcoded table filtered to available slugs | `route_static` (`cascade_router.rs:1951`) |
| **2 Confidence** | `50–200` | Per-slug score = `trials==0 ? 0.5 : upper_bound()`, where `upper_bound = min(1.0, pass_rate + 1.96·√(p(1-p)/n))` — a **Wilson-style upper confidence bound** on empirical gate-pass rate (`ModelStats::upper_bound`, `cascade/types.rs:324-327`). Optimistic-init at 0.5 forces exploration of untried arms | `route_confidence`→`confidence_scores` (`:2110,2281`) |
| **3 UCB (LinUCB)** | `> 200` | Full contextual bandit (below) | `route_ucb`→`select_ucb_model`→`ucb_scores` (`:2170,2310,2399`) |

**LinUCB math (stage 3).** Per arm (model), maintain `A` (18×18, init identity) and `b` (18-vec, init 0). Context is an **18-dim feature vector** (`CONTEXT_DIM=18`, `model_router.rs:61`). Score = **exploitation + exploration**:
- `θ = A⁻¹·b` (ridge-regressed reward weights), `exploitation = θᵀx` (`linucb_score_components`, `model_router.rs:1368-1384`)
- `exploration = α · √(xᵀ·A⁻¹·x)` — the uncertainty bonus; `A⁻¹` via `cholesky_inverse` (singular A → `(0.0, 0.0)` fallback, no exploration)
- **α exploration schedule**: `α = 0.05 + 0.95·exp(-n / 60)` (`alpha_for_observations`, `:1395-1402`; `ALPHA_MIN=0.05`, `ALPHA_MAX=1.0`, `ALPHA_TAU=60`). Decays 1.0→~0.05 over ~200 obs — cold-start explores, converges to near-greedy. **Cold start inside LinUCB**: if `total_observations < 50`, `select_features` ignores the bandit and returns the static-table slug for the context tier (`:797-827`).

**Update rule** (`update_features_internal`, `:1028-1086`): `A += x·xᵀ`; `b += reward·x`; `arm.observations += 1`; `total_observations += 1`. Reward is `clamp(0,1)` unless an **EWC regularizer** dampens it to protect consolidated weights (`arm.ewc.regularize_reward`, logs `knowledge_preservation`). Multi-objective path (`update_features_multi_objective`, `:1005`) scalarizes via `compute_routing_reward_with_weights(quality, normalized_cost, normalized_latency, weights)` before the same update; the raw (q,c,l) vector is retained in `arm.reward_stats`.

**Exploration modulation.** Base α is scaled by `temperament_exploration_multiplier(ctx)` (daimon affect) before scoring (`ucb_scores`, `:2405`), and **Pareto-dominated** arms get α×0.1 via `pareto_adjusted_alpha` (`cascade/helpers.rs:552-557`) — frontier recomputed every 50 obs (`PARETO_RECOMPUTE_INTERVAL`). Final pick uses `select_with_hysteresis` — incumbent (`ctx.previous_model`) only displaced if a rival beats it by `HYSTERESIS_THRESHOLD=0.10`; plus a `CACHE_AFFINITY_BONUS=0.15` for reusing the previous model (`apply_cache_affinity`).

**Knowledge overlay (item 13 mechanism).** `route_with_knowledge_among` computes the base route first, then `apply_knowledge_to_route` (`:623-683`) re-scores candidates as `ucb_score(slug) + KnowledgeHint.score(slug)` and **swaps the primary only if `best - current > 0.1`**. `KnowledgeHint.score` is signed (positive=past success, negative=anti-knowledge; `cascade/types.rs:204-215`), built by orchestrate from the neuro store. So knowledge is a **post-hoc additive nudge over UCB scores gated at 0.1**, not a feature fed into LinUCB's context vector — this is the drift from Runner v2's manual `dispatch_plan.model` nudge (both approximate the same intent differently).

**Decision trace (fresh workspace, Theta frequency, ~10 obs).** `select_for_frequency_among_with_knowledge(Theta, ctx, cfactor, agent, candidates, knowledge)` → `route_with_knowledge_among` → `route_with_cfactor_among` → `current_stage()` returns **Static** (obs<50) → `route_static_among` returns the role-table model (e.g. implementer→sonnet) → C-Factor bias (`bias_model_for_cfactor_among`: if `cfactor.overall > 0.8` pick cheapest, `< 0.4` pick strongest, `:2253`) → knowledge overlay checks `has_signal`; with no data it's a no-op → returns sonnet. Same call at ~120 obs would instead take the **Confidence** branch and pick the highest Wilson-UCB pass-rate slug; at ~250 obs, the **LinUCB** branch.

**Persistence schema** (`.roko/learn/cascade-router.json`, `CascadeSnapshot`, `cascade/persistence.rs:26-46`):

| Field | Persisted? | Note |
|---|---|---|
| `model_slugs` | ✅ | arm ordering |
| `role_table` | ✅ | static-stage map |
| `confidence_stats: HashMap<slug, PersistedModelStats>` | ✅ | trials/successes + Perplexity/Gemini counters (`:48-77`); `weighted_half()` halves on version change |
| `total_observations` | ✅ | restores **stage** (recomputed from Σtrials if absent) |
| `stage_transitions` | ✅ | audit history |
| `linucb_state: Option<LinUCBSnapshot>` | ❌ **always `None`** | **A/b matrices lost on restart** — see header delta. UCB stage resumes untrained |

Saved immediately in `record_completed_run` (test `runtime_feedback.rs:4748`) and at `PlanRunner::shutdown` (`orchestrate.rs:5964`); atomic tmp-rename leaks the `cascade-router.json.tmp.*` files noted in Debt.

## Episode schema

The `Episode` struct (`episode_logger.rs:169-269`) is a **bespoke append-only JSONL record** (not a `Signal`). Every field is `#[serde(default)]` (forward/backward tolerant). Key fields:

| Field | Type | Meaning |
|---|---|---|
| `kind` | String | `"agent_turn"` \| `"gate"` \| `"replan"` … |
| `id` / `episode_id` | String | hash-derived (`derive_id(agent_id, task_id, completed_at)`); `episode_id` **deprecated**, mirrors `id` |
| `agent_id`, `task_id`, `agent_template` | String | dispatch identity |
| `model`, `backend`, `trigger_kind` | String | routing outcome + cause |
| `input_signal_hash`, `output_signal_hash`, `trigger_signal_hash` | String | DAG linkage (hashes only, never raw payloads) |
| `started_at`/`completed_at`/`duration_secs` | ts/f64 | timing |
| `gate_verdicts: Vec<GateVerdict>` | — | per-rung verdicts feeding adaptive thresholds |
| `usage: Usage`, `tokens_used`, `turns` | — | cost/latency/turn accounting |
| `success`, `failure_reason: Option` | bool/String | outcome (failure reason **hashed**) |
| `reflection`, `reasoning_summary` | Option | post-gate reflection → playbook/retry learning |
| `hdc_fingerprint: Option<String>` | — | HDC episode fingerprint (set in both write paths; historical rows `null`) |
| `emotional_tag: Option<EmotionalTag>` | — | daimon affect signature |
| `headline: bool` | — | exempt from `compact()` pruning |
| `prompt_composition: Option<Value>` | — | which prompt sections/tokens/truncations produced the system prompt |
| `extra: HashMap` | — | forward-compat bag, ≤ `MAX_EXTRA_BYTES` |

**Three episode roots** (unchanged, the canonical-root defect): `.roko/episodes.jsonl` (Runner v2 `layout.root_episodes_path()` + orchestrate's direct logger `orchestrate.rs:12316`), `.roko/learn/episodes.jsonl` (LearningRuntime fan-out), `.roko/memory/episodes.jsonl` (🕰️ stale since 2026-05-02, nothing current writes). Same logical turn can land in ≥2 roots depending on dispatch surface.

## V2-aligned

- **L1 (parameter tuning, per-tick)**: AdaptiveThresholds EMA + `UpdateFrequency` cadences match v2's bounded-param loop — modulo the persistence bug.
- **L2 (strategy routing, per-task)**: cascade/model_router LinUCB with multi-objective reward (quality/cost/latency weights) is v2's EFE routing in bandit clothing; `active_inference.rs` reserves the EFE vocabulary.
- **L3 (knowledge consolidation, per-session)**: FeedbackService knowledge outcomes + neuro ingestion sinks + (dormant) dreams pipeline map onto v2's consolidation loop.
- **Predict-publish-correct fabric**: `events.rs` AgentEvent bus + `run_learning_subscriber` spawned in Runner v2 (`runner/event_loop.rs:742,766`) and orchestrate — the Bus-as-learning-fabric direction v2 §2.3 wants, at module rather than Cell granularity.
- **HDC everywhere**: episode fingerprints (both write paths) + k-medoids clustering feeding neuro tier progression.
- **WAL + jsonl_rotation**: crash-safety and log-rotation primitives match v2 15-TELEMETRY durability expectations.

## Old paradigm & tech debt

**Deleted-modules story (verified in git; refines 22-TMP-LEGACY):**
- **T2-16** (`63034aa99`/`9a55d5b48`, 2026-05-01, **merged to main**): deleted 4 orphan files that were never in `lib.rs` and never compiled — `resonant_patterns.rs`, `signal_metabolism.rs`, `shapley.rs`, `kalman.rs` (1,554 LOC). Harmless.
- **T2-17** (`d9d2bbf07` on main; `657257990` duplicate on wp-arch2 branches, 2026-05-01): deleted 8 zero-caller modules — `adversarial`, `adas`, `causal`, `reinforce_kind`, `research_pipeline`, `bandit_research`, `forensic_replay`, `drift` (3,797 deletions). These were the code homes of v1 docs 17 (ADAS/autocatalytic), 11 (Thompson drift), 12 (self-improvement frameworks) — those designs are now **doc-only**. Recoverable via `git show d9d2bbf07^:crates/roko-learn/src/<mod>.rs`. 22-TMP-LEGACY's "4,808 LOC" figure is close to but doesn't exactly match the verified 1,554+3,797=5,351 total deletions.
- **T2-17b** (`9557f68f7`, `eac3407d9`, `ea1fc7249`, `e43dc2721` — **NOT merged**, only on `wp-arch2-t5-38-t2-17b-config-learn`): would delete `local_reward`, `regression`, `verdict_scorer`, `calibration_policy` as "dead". **On main these are alive**: regression runs inside `record_completed_run`, local-rewards.json is written, calibration_policy is bus-joined. Merging that branch as-is would remove live loops — treat as do-not-merge without re-audit.

**Three dispatch surfaces, three learning-wiring tiers (cross-cutting drift for navigation):**
Learning is wired to different depths depending on which of roko's **three dispatch paths** runs a task:
1. **Legacy orchestrate.rs** — full LearningRuntime fan-out (~20 subsystems), in-router knowledge selection, DaimonState, experiments, adaptive thresholds (in-memory). The richest path.
2. **Runner v2** (`runner/event_loop.rs`) — FeedbackFacade sinks + learning EventBus + a **thinner, manual** knowledge-nudge; parallel not identical to #1.
3. **roko-acp** (`bridge_events.rs`) — **write-only**: records cascade observations, but never selects from the router, hardcodes `DaimonPolicy::default()`, no experiments, no LearningRuntime.
Same episode on different surfaces gets different learning treatment. Any navigation-layer/self-hosting doc that says "learning is wired" must qualify *which path*. This is the single largest learning-drift theme and should be surfaced at the index level.

**Debt items:**
- 11 stale `cascade-router.json.tmp.<pid>.<seq>` files still in `.roko/learn/` on 07-08 (10 zero-byte, 1×3224B, 1×5236B partial) — atomic-rename temp files leak on interrupted saves; no cleanup pass (see checklist).
- **Triple episode roots** (draft claim confirmed): `.roko/episodes.jsonl` (Runner v2 `layout.root_episodes_path()`, `roko-fs/src/layout.rs:229-231`; plus orchestrate's direct logger `orchestrate.rs:12316`), `.roko/learn/episodes.jsonl` (LearningRuntime), `.roko/memory/episodes.jsonl` (stale since 2026-05-02 — a 🕰️ third root nothing current writes).
- `roko learn tune gates|routing|budget` is a read-only viewer masquerading as a tuner (`commands/learn.rs:64-110`); CLAUDE.md advertises "Tune adaptive thresholds".
- `#[deprecated] record_outcome` shim (`cascade_router.rs:1247-1253`) still awaiting caller migration.
- 55-line crate-wide clippy allow block (`lib.rs:25-76`).
- Split ownership of gate thresholds: type in roko-gate, path in roko-learn's `LearningPaths` ("Reserved… batching" comment), writer in roko-cli shutdown — three crates, one file, zero durable writes in practice.
- Runner v2 routing observations flow through `RoutingObservationSink` with fallback/zero-reward conversions in places (draft's fidelity concern stands — e.g. `ModelChoiceSource::Default` conversions).

## Not implemented

- **v2 L4 structural adaptation** (proposal + human approval + observation window) — nothing in code.
- **Demurrage-based insight economics / AntiKnowledge / Resonator networks** (v2 06/07) — absent from roko-learn.
- **Heuristic runtime**: scoring, falsifier evaluation, Brier updates, worldview revision (v1 19) — types only.
- **Collective calibration (31×)** — no cross-agent calibration aggregation.
- **Experiments → static config materialization** — winners never rewrite `roko.toml`/templates (v1 loop-8 recipe's `config apply-experiments` subcommand doesn't exist).
- **roko-learn `oracles/`, `quality_judge`, `bayesian_confidence`, `error_enrichment`** — compiled, tested, never referenced.
- **ADAS, drift detection, causal/adversarial/forensic-replay learners** — deleted (T2-17), design-only.
- **Dedicated telemetry crate** — telemetry remains distributed (core/runtime/serve/fs/CLI); OTLP still config-recognition only.

## Migration checklist

- [ ] **[P0]** Persist adaptive gate thresholds incrementally — wire the **already-declared-but-dead** `gate_thresholds_every_n` cadence (`runtime_feedback.rs:234`, zero consumers today) into `record_completed_run`, or add a per-task flush in orchestrate.rs after `observe_pipeline`. Currently sole writer is `shutdown()` (`orchestrate.rs:5953`) → crash loses everything. Verify: `cargo run -p roko-cli -- plan run plans/ && kill -9 <pid>; test -f .roko/learn/gate-thresholds.json`
- [ ] **[P1]** Persist LinUCB A/b matrices: implement `LinUCBRouter` A/b export and populate `CascadeSnapshot.linucb_state` in `snapshot()` (`cascade_router.rs:1795`) + consume it in `load_from` (`:1832`, currently `_linucb_state`). Today the stage-3 contextual bandit resets to identity-A/zero-b on every restart while `total_observations` keeps it in UCB stage → untrained-but-greedy router. Verify: train >200 obs, save, reload, assert `arm.a_matrix != identity` (or a routing-parity test across restart)
- [ ] **[P1]** Wire cascade-router **selection** (not just observation) into the ACP path (`roko-acp/src/bridge_events.rs`) + replace `DaimonPolicy::default()` (`:634`) with real DaimonState; add prompt-experiment selection. ACP currently writes learning it never consumes (tmp-feedback/2 docs 19/34). Verify: `rg 'select_for_frequency|route_with' crates/roko-acp/src/` non-empty
- [ ] **[P1]** Unify the two knowledge-routing mechanisms: legacy orchestrate uses `select_for_frequency_among_with_knowledge` (in-router bias); Runner v2 manually nudges `dispatch_plan.model` (`event_loop.rs:4311-4340`). Pick one so fidelity/reward semantics don't drift. Verify: both paths call the same selection entrypoint
- [ ] **[P1]** Fix/instrument ACP router observation slug mismatch — observations silently dropped when `model_index_for_slug` misses (`bridge_events.rs:702-708`); count drops in a metric
- [ ] **[P0]** Mark wp-arch2 T2-17b (`9557f68f7..e43dc2721`) do-not-merge or re-audit: it deletes `regression`/`local_reward`/`calibration_policy`/`verdict_scorer` which are live on main — verify: `git log main --oneline | grep -c T2-17b` (expect 0) and `grep -rn 'detect_regressions' crates/roko-learn/src/runtime_feedback.rs`
- [ ] **[P1]** Pick one canonical episode root; make `.roko/learn/episodes.jsonl` + `.roko/memory/episodes.jsonl` derived or delete-after-migrate — verify: `roko learn episodes` shows a single documented source; `ls .roko/memory/episodes.jsonl` (expect gone)
- [ ] **[P1]** Update CLAUDE.md: item 13 (knowledge routing) and item 15 (UX34) are done — verify: `grep -n 'force_backend\|Knowledge-informed' CLAUDE.md`
- [ ] **[P1]** Make `roko learn tune` actually tune (write adjusted thresholds/weights) or rename to `roko learn show` — verify: `cargo run -p roko-cli -- learn tune gates` mutates or renamed command exists
- [ ] **[P1]** Clean up `cascade-router.json.tmp.*` leaks: add best-effort unlink of stale tmps on router load — verify: `ls .roko/learn/*.tmp.* | wc -l` → 0 after a run
- [ ] **[P2]** Delete or wire the 4 orphan modules (`oracles/`, `quality_judge`, `bayesian_confidence`, `error_enrichment`) — same treatment T2-17 gave the last batch — verify: `grep -rn 'roko_learn::(oracles|quality_judge|bayesian_confidence|error_enrichment)' crates/ | grep -v roko-learn` non-empty, else removed
- [ ] **[P2]** Loop 8 completion: `roko config apply-experiments` materializing `experiment-winners.json` into config (v1 recipe, ~90 LOC) — verify: command exists and edits `roko.toml`
- [ ] **[P2]** Preserve real model source/cost/latency in Runner v2 `RoutingObservationSink` conversions (no `ModelChoiceSource::Default` zero-reward fallbacks) — verify: `roko-cli/tests/dispatch_feedback_projection_e2e.rs` asserts non-default source
- [ ] **[P3]** Give dreams (and thus MAP-Elites + dream-routing-advice) a runtime trigger/cron — verify: `.roko/learn/dream-routing-advice.json` mtime advances after scheduled run
- [ ] **[P3]** Implement heuristic runtime scoring + falsifier trials over `heuristics.rs` shells, or move the module to docs — verify: `grep -rn 'Calibration' crates/roko-learn/src/heuristics.rs` has a runtime caller
- [ ] **[P3]** Decide telemetry consolidation (shared crate vs distributed) and OTLP layer installation in serve — verify: 53-OBSERVABILITY updated with the decision

## Open questions

1. ~~Does the Runner v2 dispatch path consult knowledge routing advice?~~ **RESOLVED (07-08): yes, but via a divergent mechanism** — Runner v2 builds `knowledge_advice` and manually nudges `dispatch_plan.model` (`runner/event_loop.rs:4231-4340`); it does **not** call `select_for_frequency_among_with_knowledge`. See new P1 unify-mechanisms checklist item. Follow-up: do the two paths produce equivalent selections under the same knowledge state, or does the manual-nudge path skip LinUCB context that the in-router path uses?
2. `gateway.jsonl` in `.roko/learn/` — writer appears to be serve's gateway/FeedbackService path (`routes/gateway.rs:1351`); confirm schema owner and whether it belongs in `LearningPaths`.
3. The live `.roko/learn/episodes.jsonl` rows have `hdc_fingerprint: null` while both write paths now set it — is that purely pre-wiring data (May 6-8) or is a code path still bypassing `fingerprint_episode`? Re-run and inspect a fresh episode.
4. 22-TMP-LEGACY's 4,808-LOC figure vs verified 5,351 deletions — which files did it count? (Cosmetic, but the recoverable-inventory list should cite the commit hashes above.)
5. `UpdateFrequency.distiller_every_n` — which distiller does LearningRuntime call today (roko-neuro distillation vs internal)? Confirm cross-crate consolidation actually fires per cadence.
