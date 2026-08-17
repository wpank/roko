# Learning & Feedback Subsystem Audit

## Executive Summary

The learning subsystem spans 70+ source files across three crates (`roko-learn`, `roko-neuro`,
`roko-dreams`) and implements a multi-timescale feedback architecture. Ten primary learning
components form a closed loop: episode logger, CascadeRouter (LinUCB bandit), efficiency
events, prompt experiments (A/B), model experiments, playbook store, conductor bandit,
budget guardrails, adaptive gate thresholds, and cost tracking. All are fully built, persisted,
and wired. The `FeedbackService` (Phase 1.3) provides a unified sink that bridges the
`WorkflowEngine` effect driver to all learning subsystems.

The critical finding: `roko run` feeds partial learning signals via
`LearningRuntime::record_completed_run()` -- episodes, cost records, routing observations
(frequency-gated), playbook updates, experiment outcomes, pattern discovery, skill extraction,
regression detection, provider health, and efficiency summaries. The full closed loop with
all 10 components runs from `orchestrate.rs` (dead code). `roko chat` and ACP paths record
almost nothing (ACP: adaptive gate thresholds for rungs 0/1/2 only).

---

## 1. Crate Architecture

### roko-learn (70+ modules, ~15K LOC)

The learning crate is the largest subsystem by module count. Key module families:

| Family | Modules | Purpose |
|---|---|---|
| **Routing** | `cascade_router.rs`, `model_router.rs`, `cascade/` (mod, types, helpers, persistence, tests) | LinUCB bandit, 3-stage cascade, context encoding, persistence |
| **Episodes** | `episode_logger.rs`, `efficiency.rs`, `cfactor.rs` | Append-only JSONL records, per-turn cost/quality snapshots, composite C-Factor |
| **Experiments** | `prompt_experiment.rs`, `model_experiment.rs` | UCB1 A/B testing for prompt sections and model selection |
| **Playbooks** | `playbook.rs`, `playbook_rules.rs`, `skill_library.rs` | Proven action sequences, rule confidence tracking, reusable skill registry |
| **Conductor** | `conductor.rs` | Thompson bandit over 7 retry interventions with 19-dim context |
| **Feedback** | `feedback_service.rs`, `runtime_feedback.rs`, `event_subscriber.rs` | Unified FeedbackSink, LearningRuntime integration, event fan-out |
| **Detection** | `anomaly.rs`, `regression.rs`, `pattern_discovery.rs`, `drift.rs` | Prompt loops, cost spikes, quality degradation, metric regressions |
| **Advanced** | `active_inference.rs`, `bandits.rs`, `bayesian_confidence.rs`, `contextual_bandit.rs`, `kalman.rs`, `shapley.rs`, `causal.rs` | Belief-state tier routing, EWC regularization, Beta-Binomial confidence, Kalman filtering, Shapley attribution, causal discovery |
| **Telemetry** | `costs_db.rs`, `costs_log.rs`, `cost_table.rs`, `latency.rs`, `provider_health.rs`, `routing_log.rs`, `task_metric.rs`, `provider_model_outcome.rs` | Cost persistence, latency EMAs, provider circuit breakers, audit logs |
| **Knowledge** | `section_effect.rs`, `section_outcome.rs`, `post_gate_reflection.rs`, `error_enrichment.rs`, `error_pattern_store.rs`, `hdc_fingerprint.rs`, `hdc_clustering.rs` | Section effectiveness tracking, gate reflections, error pattern persistence |
| **Research** | `research_pipeline.rs`, `bandit_research.rs`, `calibration_policy.rs`, `adas.rs` | Paper-to-trial pipeline, calibration predict-publish-correct loop |
| **Misc** | `budget.rs`, `pareto.rs`, `aggregate.rs`, `baseline.rs`, `local_reward.rs`, `forensic_replay.rs`, `verdict_scorer.rs`, `reinforce_kind.rs`, `quality_judge.rs`, `curriculum.rs` | Budget guardrails, Pareto frontiers, baseline computation |

### roko-neuro (10 modules, ~5K LOC)

The durable knowledge layer. Entries progress through tiers (Transient -> Consolidated ->
Canonical) based on gate-backed evidence.

| Module | Purpose |
|---|---|
| `knowledge_store.rs` | Append-only JSONL store with HDC similarity, anti-knowledge gating, confirmation tracking, decay/GC |
| `distiller.rs` | Episode batch -> Claude Haiku -> structured KnowledgeEntry candidates |
| `episode_completion.rs` | Background distillation spawn for completed episodes |
| `tier_progression.rs` | D1 (episodes -> insights), D2 (insights -> heuristics), D3 (heuristics -> PLAYBOOK.md) |
| `admission.rs` | Evidence-based admission control: LightAdmissionGate fast path + full KnowledgeAdmissionStore |
| `lifecycle.rs` | Entry lifecycle management (creation, confirmation, decay, resurrection) |
| `temporal.rs` | Time-weighted retrieval and recency scoring |
| `context.rs` | Context assembly weights (HDC 40%, keyword 30%, PF utility 20%, freshness 10%) |
| `hdc.rs` | HDC vector encoding for knowledge entries |
| `lib.rs` | KnowledgeEntry, KnowledgeKind (Insight, Warning, CausalLink, AntiKnowledge, ...), KnowledgeTier, NeuroStore trait |

### roko-dreams (26 modules, ~8K LOC)

Offline consolidation system modeled on sleep neuroscience.

| Module | Purpose |
|---|---|
| `cycle.rs` | DreamCycle orchestrator: batch episodes, cluster, distill, promote playbooks, write report |
| `runner.rs` | DreamRunner facade: scheduling (cron, plan-completion, heartbeat), budget, agent config |
| `hypnagogia.rs` | Pre-NREM liminal phase: ThalamicGate, HomuncularObserver, ExecutiveLoosener, DaliInterrupt |
| `imagination.rs` | Counterfactual episode generation, cross-domain hypothesis synthesis |
| `replay.rs` | Prioritized experience replay (Mattar-Daw utility, affect-weighted selection) |
| `rehearsal.rs` | Threat scenario rehearsal for robustness |
| `staging.rs` | StagingBuffer with Raw -> Replayed -> Validated confidence stages |
| `routing_advice.rs` | DreamRoutingAdvice: pattern summaries -> routing recommendations for CascadeRouter |
| `threat.rs` | Threat scenario enumeration and warning entry generation |
| `phase2/` | Advanced features: divergence, evolution, hauntology, oneirography, rendering, sleep_time budget |

---

## 2. Component Deep Dive

### 2.1 CascadeRouter (LinUCB Bandit)

**File:** `crates/roko-learn/src/cascade_router.rs` + `cascade/` submodules

**Architecture:**
- Arms: all configured model slugs (e.g., `["sonnet", "opus", "haiku"]`)
- Three stages with automatic transition:
  - Stage 1 (Static, < 50 obs): hardcoded AgentRole -> model table
  - Stage 2 (Confidence, 50-200 obs): empirical pass rates + Wald CI (`1.96 * sqrt(p*(1-p)/n)`)
  - Stage 3 (UCB, > 200 obs): full LinUCB with alpha decay (`0.05 + 0.95 * exp(-obs/60)`)

**Context vector (18 dimensions, `CONTEXT_DIM = 18` in `model_router.rs`):**
- Task category: one-hot, 8 dims for TaskCategory variants
- Complexity band: scalar (0.0 / 0.5 / 1.0 for Fast / Standard / Complex)
- Iteration: normalized (iteration / 10, capped at 1.0)
- Agent role: hashed to 4-dim float vector via `role_hash()`
- Crate familiarity: 0.0-1.0 (success_count / total_count)
- Has prior failure: 0.0 or 1.0
- Bias term: always 1.0
- Cache affinity: 1.0 when candidate matches previous model

**Reward:** `compute_routing_reward_v2(pass_rate, normalized_cost, observed_latency_ms, latency_sla_ms)`
- Multi-objective: success dominates, with cost and latency as secondary factors
- No c-factor in reward signal

**Additional features:**
- `ForceBackendOverrideRecorder` trait: records manual model overrides with `OVERRIDE_LEARNING_RATE`
- Pareto frontier recomputation every `PARETO_RECOMPUTE_INTERVAL` observations
- Free-tier Gemini shadow evaluation support
- Affect-adjusted tier thresholds via `daimon_policy` and `tier_thresholds`
- Hysteresis via `select_with_hysteresis()` to prevent oscillation
- Thompson sampling discount (`THOMPSON_DEFAULT_DISCOUNT = 0.99`) for non-stationary environments
- EWC regularization (`EwcRegularizer`) to prevent catastrophic forgetting
- Learning rate schedule: cold (1.0), warm (0.85), mature (0.7)

**Persistence:** `.roko/learn/cascade-router.json` via `CascadeSnapshot`

**Live callers:** FeedbackService.observe_model_call() on every model call (when router attached). `LearningRuntime::record_completed_run()` updates with derived RoutingContext (frequency-gated). orchestrate.rs has full routing context.

### 2.2 Episode Logger

**File:** `crates/roko-learn/src/episode_logger.rs`

**Per-episode record:**
- Agent identity: agent_id, role, template, kind
- Task context: plan_id, task_id, domain, episode_id
- Token usage: input, output, cached (read/write), cost_usd, cost_usd_without_cache, wall_ms
- Gate verdicts: ordered Vec<GateVerdict> with gate name, passed, signature
- Prompt composition snapshot: section names, token counts, truncation flags
- Model, provider, backend identification
- HDC fingerprint (text + metadata)
- Emotional tags (daimon state)
- Success flag + failure_reason
- Extra map (capped at 16KB serialized via `MAX_EXTRA_BYTES`)

**Template suggestions:** Episodes with similar HDC fingerprints (> 0.7 similarity) within
30 days are surfaced as template candidates for new tasks.

**Write serialization:** process-wide `parking_lot::Mutex` ensures concurrent appenders
never interleave JSONL lines.

**Fault tolerance:** Reader is line-tolerant -- parse failures on individual lines surface
as `LoggerError::Parse` with line number, not a stream corruption.

### 2.3 FeedbackService

**File:** `crates/roko-learn/src/feedback_service.rs`

The canonical unified feedback sink implementing `FeedbackSink` trait from `roko-core::foundation`.

**Event types handled:**
1. `ModelCall` -- logs to efficiency.jsonl, observes CascadeRouter (when attached), tracks
   provenance (prompt_section_ids + knowledge_ids) for deferred outcome attribution
2. `GateResult` -- resolves pending provenance for run_id, updates knowledge scores (+1/-1)
   and section effectiveness
3. `WorkflowComplete` -- resolves provenance, generates Episode for logger

**Knowledge feedback loop:**
- On ModelCall: remembers which knowledge entries and prompt sections were used (provenance)
- On GateResult: retrieves provenance for that run, applies KnowledgeOutcome (Success/Failure/Partial)
- Knowledge scores persisted to `knowledge-scores.json` (cumulative +1/-1 per entry)
- Section effectiveness persisted to `section-effects.json` (inclusion/exclusion pass rates)
- Scores are loadable on restart -- `load_knowledge_scores()` reads snapshot

**Integration test coverage:** `test_knowledge_loop_scoring` verifies the full cycle:
KnowledgeStore -> PromptAssemblyService -> FeedbackService -> knowledge score update -> reload

### 2.4 LearningRuntime

**File:** `crates/roko-learn/src/runtime_feedback.rs` (~1400 lines)

Single integration point for CLI/orchestrator code. `record_completed_run(input)` triggers:

| Step | Component | Cadence |
|---|---|---|
| 1 | EpisodeLogger.append() | Every run |
| 2 | CostsLog / CostsDb | Every run |
| 3 | ProviderHealthTracker.record() | Every run |
| 4 | CascadeRouter.observe() | Every `router_every_n_episodes` (default: 1) |
| 5 | PlaybookStore.record_outcome() | When `playbook_id` set |
| 6 | PlaybookRules.record_outcome() | When `playbook_rule_id` set |
| 7 | SkillLibrary.record_use() | When `matched_skill_id` set |
| 8 | SkillLibrary.extract_from_episode() | Every `skill_mining_every_n` (default: 10) |
| 9 | PromptExperiment.record_outcome() | When `experiment_variant_id` set, cadence-gated |
| 10 | PatternMiner.ingest_episode() | Every `pattern_discovery_every_n` (default: 20) |
| 11 | TaskMetric append + regression detection | When `task_metric` set |
| 12 | PostGateReflectionStore | Every run with gate verdicts |
| 13 | ProviderModelOutcomeStore | Every run |
| 14 | NormalizedEfficiencySummary | Every run |
| 15 | GateOutcome records | Every run with gate verdicts |
| 16 | Retry outcome records | When retry data present |
| 17 | Knowledge seed append | When distiller cadence reached |
| 18 | DaimonState affect update | Every run |

**Persistence paths (LearningPaths):**
21 distinct files under `.roko/learn/`:
- `episodes.jsonl`, `costs.jsonl`, `skills.json`, `playbooks/`, `playbook-rules.toml`
- `task-metrics.jsonl`, `efficiency.jsonl`, `efficiency-summaries.jsonl`
- `gate-outcomes.jsonl`, `retry-outcomes.jsonl`, `knowledge-seeds.jsonl`
- `latency-stats.json`, `c-factor.jsonl`, `cascade-router.json`
- `experiments.json`, `experiment-winners.json`, `gate-thresholds.json`
- `local-rewards.json`, `section-effects.json`, `post-gate-reflections.json`
- `provider-model-outcomes.jsonl`

### 2.5 Conductor Bandit

**File:** `crates/roko-learn/src/conductor.rs`

**7 actions:** Continue, InjectHint(ErrorDigest), InjectHint(SkillSuggestion),
InjectHint(SimplifyApproach), SwitchModel, Restart, Abort

**19-dimension context:** iteration (normalized), consecutive_failures, error_pattern (one-hot
for 10 ErrorPattern variants), elapsed_ms, cost_so_far_usd, model_tier (hash), task_complexity (hash)

**Algorithm:** Blended Thompson + linear context scoring:
- `ACTION_BLEND_THOMPSON = 0.65` (Thompson posterior sampling weight)
- `ACTION_BLEND_CONTEXT = 0.35` (linear context model weight)
- Weight updates via online SGD with `WEIGHT_LEARNING_RATE = 0.35`, clamped to [-4, 4]

**Persistence:** JSON via `roko_fs::atomic_write_json()`

### 2.6 Prompt Experiments (A/B Testing)

**File:** `crates/roko-learn/src/prompt_experiment.rs`

Two distinct systems:

1. **Prompt variant experiments (`ExperimentStore`):**
   - Multiple `PromptVariant` per experiment with UCB1 arm selection
   - Per-variant stats: trials, successes
   - Wilson 95% CI for convergence detection: `(p + z^2/2n) / (1 + z^2/n)`
   - Lifecycle: Running -> Concluded (winner applied as static override)
   - Metric tracking: per-variant `VariantMetricStats` (samples, sum, last)
   - Winner export to `experiment-winners.json`

2. **Model A/B experiments (`ModelExperimentStore` in `model_experiment.rs`):**
   - Same UCB1 framework applied to model selection
   - Additional per-variant: cost_usd, tokens, duration tracking
   - Persistence at `model-experiments.json`

### 2.7 Playbook Store

**File:** `crates/roko-learn/src/playbook.rs`

Named sequences of proven action steps. Each `Playbook` contains:
- Ordered `PlaybookStep`s (index, description, action_kind, expected_signals)
- Success/failure counters for confidence tracking
- Merge threshold: `PLAYBOOK_MERGE_THRESHOLD = 0.80` -- similar playbooks are merged

**Extraction:** `extract_playbook_from_episode()` converts successful task tool call
sequences into playbook steps. Goals truncated to 200 chars.

**Query:** `PlaybookStore::query()` returns matching playbooks for a given goal/context.
Matched playbooks are injected into system prompt Layer 6.

### 2.8 Anomaly Detection

**File:** `crates/roko-learn/src/anomaly.rs`

Three detection channels:
1. **Prompt loops:** sliding window of 20 prompt hashes, alert at 5+ repeats
2. **Cost spikes:** EWMA baseline with z-score > 3.0 threshold
3. **Quality degradation:** compares recent 5 scores against prior 10,
   flags when drop > 0.15 and recent average < 0.5

### 2.9 Section Effectiveness

**File:** `crates/roko-learn/src/section_effect.rs`

Tracks whether including a prompt section (e.g., "workspace_map", "playbook_hits")
correlates with higher gate pass rates.

- Per-section: `included_trials`, `included_passes`, `excluded_trials`, `excluded_passes`
- Lift calculation: `included_rate - excluded_rate`
- Budget weight: `(1.0 + lift).clamp(0.5, 1.5)` -- sections proven harmful get deprioritized
- Priority change recommendation at 20+ included trials and 5+ excluded trials

### 2.10 Knowledge Store (roko-neuro)

**File:** `crates/roko-neuro/src/knowledge_store.rs`

Append-only JSONL store with sophisticated retrieval scoring:

**Entry lifecycle:**
- Creation: Transient tier, initial confidence
- Confirmation: when new entries overlap existing (tag + keyword similarity), confidence boosted by `CONFIRMATION_BOOST = 1.5`
- Decay: `weight = initial * 0.5^(age/halfLife) * (1 + confirmations * 0.1)`
- Death: when recency factor < `DEATH_THRESHOLD = 0.01` (1% of initial weight)
- Resurrection: dead entries re-confirmed get `RESURRECTION_CONFIDENCE = 0.6`

**Anti-knowledge gating (HDC feature):**
- Warn at similarity > 0.5
- Discount confidence at > 0.7 (`ANTI_KNOWLEDGE_DISCOUNT_FACTOR = 0.5`)
- Reject entirely at > 0.9

**Query scoring (ContextAssemblyWeights):**
- HDC similarity: 40%
- Keyword/pheromone relevance: 30%
- Predictive foraging utility: 20%
- Freshness/recency: 10%
- Cross-domain diversity bonus: 15%

**Admission control (`admission.rs`):**
- `LightAdmissionGate`: fast path for trusted, novel, confident observations
  - min_confidence: 0.5, min_novelty: 0.3, min_source_trust: 0.65
- `KnowledgeAdmissionStore`: full evidence pipeline for ambiguous candidates
  - Default admission confidence: 0.72 for positive, 0.65 for anti-knowledge

### 2.11 Dream Cycle (roko-dreams)

**File:** `crates/roko-dreams/src/cycle.rs`

Offline consolidation modeled on sleep phases:

1. **Hypnagogia** (pre-NREM): ThalamicGate filters, HomuncularObserver tags,
   ExecutiveLoosener generates variant encodings, DaliInterrupt for creative associations
2. **NREM**: Episode clustering by plan/task shape, structural consolidation via
   `CrossEpisodeConsolidator`, distillation into knowledge candidates
3. **REM**: Counterfactual episode generation (`imagination.rs`), cross-domain hypothesis
   synthesis, threat rehearsal
4. **Integration**: Tier progression (D1 -> D2 -> D3), playbook promotion, routing advice
   generation, StagingBuffer management

**Trigger policies:**
- Cron schedule (configurable)
- Plan completion trigger
- Bus pulse trigger (event-driven)
- Heartbeat interval

**Budget tracking:** Per-phase USD budget via `DreamComputeBudget`

**Outputs:**
- `DreamCycleReport` with cluster summaries, C-Factor regression, routing recommendations
- Knowledge entries written to durable store
- Playbooks created from successful clusters
- Dream routing advice saved for CascadeRouter consumption

---

## 3. Feedback Loop Topology

### 3.1 The Full Closed Loop (orchestrate.rs)

```
dispatch_agent_with()
  |-- Consult CascadeRouter -> select model
  |-- Check budget -> routing pressure
  |-- Load experiment -> variant override
  |-- Query playbooks -> prompt injection
  |-- Build 9-layer prompt -> PromptComposer
  |-- Dispatch agent
  |-- Record episode -> EpisodeLogger
  |-- Record efficiency -> efficiency.jsonl
  |-- Observe routing -> CascadeRouter update
  |-- Record outcome -> playbook update
  |-- Record experiment -> variant stats
  |-- Update conductor -> retry policy
  |-- Update thresholds -> gate learning
  |-- Trigger distillation -> knowledge store
  |-- Check replan -> gate failure response
```

### 3.2 What `roko run` Records

| Signal | Source | Coverage |
|---|---|---|
| Episode | `append_episode_log()` + `record_completed_run()` | Full |
| Cost record | `derive_cost_record()` via `record_completed_run()` | Full |
| Routing observation | `update_cascade_router()` via `record_completed_run()` | Frequency-gated, simple context |
| Playbook update | `record_completed_run()` | Only if `playbook_id` set |
| Experiment update | `record_completed_run()` | Only if `experiment_variant_id` set |
| Pattern mining | `PatternMiner.ingest_episode()` | Every 20 episodes |
| Skill extraction | `SkillLibrary.extract_from_episode()` | Every 10 episodes |
| Task metric | `append_task_metric()` | When metric provided |
| Regression detection | `detect_regressions()` | When sufficient data |
| Provider health | `ProviderHealthTracker.record()` | Every run |
| Post-gate reflection | `PostGateReflectionStore` | Every run with gates |
| Provider/model outcome | `ProviderModelOutcomeStore` | Every run |
| Efficiency summary | Normalized summary append | Every run |
| Gate outcomes | Per-verdict JSONL records | Every run with gates |
| Knowledge seed | Seed for neuro ingestion | Every `distiller_every_n` |
| Daimon affect | `DaimonState` update | Every run |

**Not recorded by `roko run`:**
- Raw efficiency events (the rich `AgentEfficiencyEvent` with prompt sections, tool call meta)
- Budget checks / routing pressure
- Conductor intervention policy
- Full routing context (18 features; only derived simple context)
- CascadeRouter selection (pre-dispatch)

### 3.3 What `roko chat` Records

Almost nothing. No episodes, no routing, no cost tracking, no learning signals.

### 3.4 What ACP Records

Adaptive gate thresholds only (rungs 0/1/2 via `THRESHOLDS_PATH`).

---

## 4. Anti-Patterns

| # | Anti-Pattern | Where | Severity |
|---|---|---|---|
| 1 | Feedback as afterthought | `roko chat` and ACP record almost nothing | High |
| 2 | Full loop in dead code | orchestrate.rs (21K+ lines) has the only complete loop | High |
| 3 | God file | All 10+ learning components called from one file | Medium |
| 4 | Dual episode writes | `roko run` writes episodes twice (once direct, once via runtime) | Low |
| 5 | Missing budget enforcement | No live path checks budget before dispatch | Medium |
| 6 | No conductor in live paths | Retry decisions are hardcoded, not learned | Medium |

---

## 5. Sources

| File | What it covers |
|---|---|
| `crates/roko-learn/src/lib.rs` | 70+ module declarations, crate overview |
| `crates/roko-learn/src/feedback_service.rs` | FeedbackService, KnowledgeOutcome, provenance tracking |
| `crates/roko-learn/src/runtime_feedback.rs` | LearningRuntime, LearningPaths (21 files), CompletedRunInput, UpdateFrequency |
| `crates/roko-learn/src/cascade_router.rs` | CascadeRouter, 3 stages, stage tracking, override learning |
| `crates/roko-learn/src/model_router.rs` | CONTEXT_DIM=18, RoutingContext, LinUCBRouter, alpha decay |
| `crates/roko-learn/src/conductor.rs` | ConductorBandit, 7 actions, 19-dim context, blended Thompson+context |
| `crates/roko-learn/src/episode_logger.rs` | Episode struct, GateVerdict, Usage, HDC fingerprinting |
| `crates/roko-learn/src/prompt_experiment.rs` | ExperimentStore, UCB1, Wilson CI, VariantStats |
| `crates/roko-learn/src/model_experiment.rs` | ModelExperimentStore, model A/B |
| `crates/roko-learn/src/playbook.rs` | PlaybookStore, PlaybookStep, extraction, merge |
| `crates/roko-learn/src/efficiency.rs` | AgentEfficiencyEvent (20+ fields), PromptSectionMeta, ToolCallMeta |
| `crates/roko-learn/src/section_effect.rs` | SectionEffectivenessRegistry, lift weights, priority recommendations |
| `crates/roko-learn/src/anomaly.rs` | AnomalyDetector, prompt loops, cost spikes, quality degradation |
| `crates/roko-learn/src/pattern_discovery.rs` | PatternMiner, trigram mining, EpisodeView trait |
| `crates/roko-learn/src/skill_library.rs` | SkillLibrary, Skill, extraction from episodes |
| `crates/roko-learn/src/budget.rs` | BudgetGuardrail, 3 scopes, 5 actions |
| `crates/roko-learn/src/regression.rs` | RegressionThresholds, detect_regressions() |
| `crates/roko-learn/src/cfactor.rs` | CFactor composite metrics, leave-one-out contributions, pathologies |
| `crates/roko-learn/src/costs_db.rs` | CostRecord, CostsDb |
| `crates/roko-learn/src/latency.rs` | LatencyRegistry, rolling EMAs |
| `crates/roko-learn/src/provider_health.rs` | ProviderHealthTracker, circuit breaker |
| `crates/roko-neuro/src/lib.rs` | KnowledgeEntry, KnowledgeKind, KnowledgeTier, NeuroStore |
| `crates/roko-neuro/src/knowledge_store.rs` | KnowledgeStore, query scoring, anti-knowledge, confirmation tracking |
| `crates/roko-neuro/src/distiller.rs` | Distiller, Claude Haiku backend, episode-to-knowledge extraction |
| `crates/roko-neuro/src/episode_completion.rs` | spawn_episode_distillation(), background knowledge extraction |
| `crates/roko-neuro/src/tier_progression.rs` | D1/D2/D3 progression, InsightRecord, heuristic calibration |
| `crates/roko-neuro/src/admission.rs` | LightAdmissionGate, KnowledgeAdmissionStore, evidence sources |
| `crates/roko-dreams/src/cycle.rs` | DreamCycle, DreamCycleReport, 4-phase consolidation |
| `crates/roko-dreams/src/runner.rs` | DreamRunner, DreamConfig, trigger policies, scheduling |
| `crates/roko-dreams/src/lib.rs` | Public re-exports, DreamsEngine facade |
| `crates/roko-dreams/src/routing_advice.rs` | DreamRoutingAdvice, pattern-to-recommendation |
| `crates/roko-dreams/src/imagination.rs` | Counterfactual episodes, hypothesis synthesis |
| `crates/roko-dreams/src/replay.rs` | Prioritized experience replay, Mattar-Daw utility |
| `crates/roko-dreams/src/staging.rs` | StagingBuffer, Raw/Replayed/Validated stages |
