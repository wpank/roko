# SOURCE-INDEX — Code Anchors for 05-Learning Parity

Verified code references for batch `05`, organized by crate and focused on the runtime seams an agent is likely to touch.

Generated: 2026-04-16

---

## Important Corrections First

Use these before trusting the docs literally:

- `EpisodeLogger::compact(...)` exists at `crates/roko-learn/src/episode_logger.rs:964-1042`; the runtime retention story is not “append forever with no compaction”.
- `build_learned_context(...)` at `crates/roko-cli/src/orchestrate.rs:7069-7135` currently builds `MatchContext` with `role` only and uses `search_by_tag(role)` for skills.
- `detect_regressions(...)` at `crates/roko-learn/src/regression.rs:140-276` does not iterate `baseline.slices`, and every emitted alert still sets `slice: None`.
- `RegressionThresholds.iterations_increase` exists, but `detect_regressions(...)` never checks it.
- `CalibrationTracker` is already used indirectly through routing-log replay and predictive prompt/scoring consumers. What remains unused is the direct `PredictionRecord::register/resolve` path.
- `run_learning_subscriber(...)` and `DriftDetector` still have no production caller.

---

## crates/roko-learn/src/

### Runtime hub

| File | What | Section |
|------|------|---------|
| `runtime_feedback.rs:163-215` | `UpdateFrequency` and due checks | A.14, E.10 |
| `runtime_feedback.rs:323-345` | `LearningRuntime` struct and subsystem handles | E.01 |
| `runtime_feedback.rs:376-386` | runtime loading of `PatternMiner`, `LatencyRegistry`, `CascadeRouter`, experiments, section-effectiveness | A.10, E.01 |
| `runtime_feedback.rs:764-927` | `record_completed_run(...)` main integration sequence | E.01 |
| `runtime_feedback.rs:805-812` | provider health update path | E.02 |
| `runtime_feedback.rs:829-831` | playbook-rule confidence update path | B.02, E.01 |
| `runtime_feedback.rs:862-870` | C-Factor and pattern-discovery periodic updates | A.14, E.12 |
| `runtime_feedback.rs:911-925` | local reward observations | E.19 |
| `runtime_feedback.rs:1038-1061` | experiment winner sync into cascade router | E.09 |
| `runtime_feedback.rs:1356-1375` | regression-report generation and return | D.18 |

### Episodes + patterns

| File | What | Section |
|------|------|---------|
| `episode_logger.rs:89-100` | `GateVerdict` embedded in `Episode` | A.02 |
| `episode_logger.rs:169-250` | `Episode` struct — current schema | A.01 |
| `episode_logger.rs:317-343` | fingerprint helpers storing into `extra` | A.06 |
| `episode_logger.rs:384-523` | importance components, score, free helpers | A.08 |
| `episode_logger.rs:842-868` | `EpisodeLogger::append` crash-safe append path | A.03 |
| `episode_logger.rs:964-1042` | `EpisodeLogger::compact` retention / pruning path | A.07 |
| `pattern_discovery.rs:53-81` | `EpisodeView` and `Pattern` | A.11 |
| `pattern_discovery.rs:99-181` | `PatternMiner` core and `ingest_episode` | A.10 |
| `pattern_discovery.rs:291-407` | `CrossEpisodeConsolidator::discover()` | A.12 |
| `hdc_clustering.rs:38-81` | `KMedoidsConfig` and `k_medoids()` | A.13 |

### Knowledge tiers

| File | What | Section |
|------|------|---------|
| `playbook.rs:44-98` | `PlaybookStep` and `Playbook` structs | B.01 |
| `playbook.rs:147-151` | `PlaybookStore` shape | B.02 |
| `playbook.rs:196,235,329,369,390,400` | `save`, `load`, `list`, `record_outcome`, `record`, `delete` | B.02-B.03 |
| `playbook_rules.rs:35-46` | `Triggers` | B.04 |
| `playbook_rules.rs:66-89` | `Rule` | B.04 |
| `playbook_rules.rs:116-127` | `MatchContext` | B.04, B.08 |
| `playbook_rules.rs:173-200` | `PlaybookRules` load/open path | B.07 |
| `playbook_rules.rs:341-352` | confidence update math | B.05, F.01 |
| `playbook_rules.rs:357-362` | `prune(min_confidence)` | B.06 |
| `playbook_rules.rs:595-644` | rule matching logic | B.08 |
| `skill_library.rs:63-154` | `Skill` struct | B.09 |
| `skill_library.rs:395` | `SkillQuery` | B.12 |
| `skill_library.rs:1045` | `SkillLibrary::register` | B.09, F.06 |
| `skill_library.rs:1104` | `search_by_tag(tag)` | B.12 |
| `skill_library.rs:1128-1158` | helper mapping into `SkillQuery` | B.12 |
| `skill_library.rs:1246` | `extract_skill(request)` | F.02 |
| `skill_library.rs:1528` | `select(query, limit)` | B.12 |
| `skill_library.rs:1632-1671` | `prune_stale(days)` | B.10 |

### Routing + learning policies

| File | What | Section |
|------|------|---------|
| `bandits.rs:115-131` | `EwcRegularizer` | C.04 |
| `bandits.rs:284-435` | `UcbBandit` core | C.01 |
| `bandits.rs:493-591` | `BanditBank` | C.02 |
| `bandits.rs:748-1031` | `TrackAndStopBandit` | C.03, C.05 |
| `model_router.rs:60-74` | core constants including `CONTEXT_DIM=18` | C.06, C.07 |
| `model_router.rs:80-120` | `LearningRateSchedule` and alpha modulation | C.19 |
| `model_router.rs:129-214` | `RoutingContext` and feature construction | C.06, E.03 |
| `model_router.rs:316-347` | `compute_routing_reward_v2` | E.08 |
| `model_router.rs:449-524` | `ThompsonArm` | C.07 |
| `model_router.rs:655-734` | `LinUCBRouter` and selection path | C.06 |
| `cascade_router.rs:63-70` | `CascadeStage` | C.11, F.04 |
| `cascade_router.rs:92-118` | `StageTransition`, `CascadeModel` | C.12-C.13, F.05 |
| `cascade_router.rs:237-249` | thresholds including `HYSTERESIS_THRESHOLD` | C.11, E.10 |
| `cascade_router.rs:994-1009` | `CascadeRouter` struct | C.11 |
| `cascade_router.rs:1547` | `apply_cost_pressure` current boolean contract | E.07 |
| `cascade_router.rs:2144+` | `append_routing_log(...)` | E.16 |
| `cascade_router.rs:2665-2694` | `AgentDispatchBias` consumption | E.13 |
| `pareto.rs:12-47` | `ModelObservation` and 2D Pareto frontier | C.17-C.18 |
| `active_inference.rs:17-83` | `BeliefState` and `select_tier` | C.20 |

### Metrics + regression + cost + health

| File | What | Section |
|------|------|---------|
| `efficiency.rs:80` | `AgentEfficiencyEvent` | D.01 |
| `efficiency.rs:251-349` | `Grade` and composite prompt-efficiency scoring | D.02 |
| `task_metric.rs:35-58` | `MetricFilter` | D.03 |
| `baseline.rs:23-61` | `SliceBaseline` and `Baseline` | D.05 |
| `baseline.rs:128-190` | `compute_baseline()` groups by `(role, complexity)` | D.05 |
| `regression.rs:29-52` | `RegressionThresholds` | D.06-D.07 |
| `regression.rs:69-89` | `RegressionAlert` / `RegressionReport` | D.07-D.08 |
| `regression.rs:140-276` | `detect_regressions(...)` overall-only path | D.07-D.08 |
| `cost_table.rs:11-70` | `ModelPricing` and blended-cost formula | D.10 |
| `costs_log.rs` | append-only cost log | D.11 |
| `costs_db.rs:61-78,472-602` | `CostSummary`, `CostsDb`, aggregation methods | D.12 |
| `budget.rs:8-40` | `BudgetGuardrail` and `BudgetAction` | D.13, E.07 |
| `budget.rs:82-102` | budget threshold mapping | D.13 |
| `provider_health.rs:43-69` | `CircuitState` and `ErrorClass` | D.14-D.15 |
| `provider_health.rs:162-165` | cooldown mapping | D.15 |
| `latency.rs:20-40,123-127` | `LatencyStats` and `LatencyRegistry` | D.16 |
| `anomaly.rs:20-44,206+` | `AnomalyDetector`, `session_start_ms`, anomaly variants | D.17 |

### Prediction + dead scaffolding

| File | What | Section |
|------|------|---------|
| `prediction.rs:14-47` | `PredictionRecord` | E.16 |
| `prediction.rs:52-117` | `register`, `resolve`, `from_routing_log` | E.16 |
| `prediction.rs:125-299` | `CalibrationTracker` and `PredictionCalibrationSource` impl | E.16 |
| `prediction.rs:230-299` | `adjust_prediction` and `summary()` | E.16-E.17 |
| `routing_log.rs:13-51` | `RoutingDecisionLog` | E.16 |
| `routing_log.rs:68-88` | `RoutingDecisionMeta` | E.16 |
| `routing_log.rs:93-197` | `RoutingDecisionLogStore` | E.16 |
| `event_subscriber.rs:48-182` | `run_learning_subscriber(...)` | E.18 |
| `drift.rs:89+` | `DriftDetector` | E.18 |
| `local_reward.rs:18+` | `LocalRewardFunction` | E.19 |
| `cfactor.rs:16-31` | `CFactor` summary struct | E.12 |
| `cfactor.rs:39-59` | contribution and bias types | E.13 |
| `cfactor.rs:63-91` | `CollectivePathology` | E.14 |
| `cfactor.rs:95-123` | `CFactorComponents` | E.12 |
| `cfactor.rs:209-221` | `dispatch_bias_for_agent(...)` | E.13 |
| `cfactor.rs:465+` | trend and regression helpers | F.10 |

---

## crates/roko-core/src/

### Predictive and collective-calibration consumers

| File | What | Section |
|------|------|---------|
| `prediction.rs:8-39` | `PredictionCalibrationSummary` and source trait | E.16-E.17 |
| `prediction.rs:51-172` | `PredictiveScorer` | E.16 |
| `prediction.rs:174-215` | `PredictionPolicy` | E.16 |
| `cfactor.rs:8-34` | `CFactorSummary` and `CFactorSource` | E.12-E.13 |
| `cfactor.rs:39+` | `CFactorPolicy` | E.12-E.13 |

---

## crates/roko-cli/src/

### Orchestrator hot spots

| File | What | Section |
|------|------|---------|
| `orchestrate.rs:237-244` | `load_predictive_calibration(workdir)` from routing logs | E.16 |
| `orchestrate.rs:293-326` | `predictive_policy_sections(...)` | E.16 |
| `orchestrate.rs:7017-7042` | `observe_cascade_router(...)` | C.11, E.01 |
| `orchestrate.rs:7069-7135` | `build_learned_context(...)` | B.08, B.12 |
| `orchestrate.rs:7479-7495` | regression-report consumption / logging | D.18 |
| `orchestrate.rs:9739-9740` | routing-log store / record setup | E.16 |
| `orchestrate.rs:10105-10113` | initial routing-log append | E.16 |
| `orchestrate.rs:10336-10409` | learned-context prompt section injection | B.08, E.06 |
| `orchestrate.rs:10415-10459` | predictive-calibration sections and `PredictiveScorer` integration | E.16 |
| `orchestrate.rs:10740-10747` | completed routing-log append with outcome | E.16 |
| `orchestrate.rs:1753+` | `SkillQuery` construction on another production path | B.12 |

### Other CLI surfaces

| File | What | Section |
|------|------|---------|
| `commands/experiment.rs` | experiment command surface | E.09 |
| `main.rs` | possible startup wiring target if subscriber activation is chosen | E.18 |

---

## crates/roko-compose/src/

| File | What | Section |
|------|------|---------|
| `role_prompts.rs:282-283` | skills forwarded into prompt builder | B.12, E.06 |
| `system_prompt_builder.rs:179+` | skills section builder path | B.12 |

---

## Missing / Absent (code-search negatives)

These doc features have no matching production code in `crates/`:

### Storage / clustering and typed usage profiles

| Absent Feature | Search | Section |
|----------------|--------|---------|
| `EpisodeStorageConfig`, `CompressedEpisodeSummary` | `rg -n "EpisodeStorageConfig|CompressedEpisodeSummary" crates/` | A.07 |
| `EpisodeCluster`, `incremental_dbscan`, `eps_similarity` | `rg -n "EpisodeCluster|incremental_dbscan|eps_similarity" crates/` | A.09 |
| `ToolUsageProfile`, `ToolSequencePattern`, `ToolWarning` | `rg -n "ToolUsageProfile|ToolSequencePattern|ToolWarning" crates/` | B.13 |

### Advanced routing / analytics research

| Absent Feature | Search | Section |
|----------------|--------|---------|
| `ContextualThompson` | `rg -n "ContextualThompson" crates/` | C.08 |
| `NeuralUCB` | `rg -n "NeuralUCB" crates/` | C.09 |
| `BanditEnsemble` | `rg -n "BanditEnsemble" crates/` | C.10 |
| `LookaheadRouter` | `rg -n "LookaheadRouter" crates/` | C.14 |
| `CostSpectrumRouter` | `rg -n "CostSpectrumRouter" crates/` | C.15 |
| `PlattScaling`, `IsotonicRegression` | `rg -n "PlattScaling|IsotonicRegression" crates/` | C.16 |
| `PageHinkleyDetector`, `AdwinDetector`, `CusumDetector`, `BOCPD` | `rg -n "PageHinkleyDetector|AdwinDetector|CusumDetector|BOCPD" crates/` | D.09 |

### Predictive-foraging and governance prescriptive types

| Absent Feature | Search | Section |
|----------------|--------|---------|
| `BrierScore`, `ReliabilityDiagram`, `ArithmeticCorrector` | `rg -n "BrierScore|ReliabilityDiagram|ArithmeticCorrector|brier_score|reliability_diagram" crates/` | E.17 |
| `PredictiveForager`, `PredictiveForagingEngine` | `rg -n "PredictiveForager|PredictiveForagingEngine" crates/` | E.17 |
| `ImprovementScoreCard`, `PeriodMetrics`, `SignificanceTests` | `rg -n "ImprovementScoreCard|PeriodMetrics|SignificanceTests" crates/` | F.07 |
| `SafetyInvariants`, `GateGamingDetector`, `ConstitutionalConstraints` | `rg -n "SafetyInvariants|GateGamingDetector|ConstitutionalConstraints" crates/` | F.08 |

---

## Runtime Negatives That Matter For Batch 05

These matter because the code exists, but production is still thinner than it should be:

| Runtime-negative | Evidence | Section |
|------------------|----------|---------|
| `MatchContext` richer fields never populated in main learned-context path | `build_learned_context(...)` fills only `role` | B.08 |
| `SkillQuery::select(...)` exists but main learned-context path uses `search_by_tag(role)` | orchestrator learned-context path | B.12 |
| `RegressionAlert::slice` always `None` in production | `detect_regressions(...)` emits `slice: None` only | D.08 |
| `iterations_increase` is dead | no detection block reads it | D.07 |
| direct `PredictionRecord::register/resolve` path is unused | no non-test callers | E.16 |
| `run_learning_subscriber(...)` and `DriftDetector` have no production caller | search negatives outside their own files/tests | E.18 |

---

## File Size Reality Check

Batch `05` is mostly about **runtime activation and contract cleanup inside an already large learning system**:

- `runtime_feedback.rs` is large and central,
- `cascade_router.rs` is very large and already heavily used,
- `prediction.rs` is modest but already connected indirectly through routing-log replay,
- `playbook_rules.rs` and `skill_library.rs` are richer than the main production path currently uses,
- and the main gaps are often “library real, production thinner” rather than “no subsystem exists”.

That is why batch `05` should default to tighter production contracts and clearer docs, not to building new routing or governance theory.
