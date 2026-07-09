# SOURCE-INDEX — Code Anchors For 05-Learning

Current code anchors for the learning parity refresh.

Generated: 2026-04-18

---

## First Corrections To Trust

- `roko-learn` is already broad and wired; parity work here is mostly contract cleanup, not subsystem invention.
- `EpisodeLogger::compact(...)` already exists.
- `PatternMiner` is already instantiated and used by `LearningRuntime`.
- `build_learned_context(...)` still under-populates `MatchContext`.
- predictive calibration already has consumers, but the live path is routing-log replay rather than the direct `PredictionRecord` lifecycle.
- `TierProgression` already exists in `roko-neuro`.

## Learning Runtime

| Anchor | Why it matters |
|--------|----------------|
| `crates/roko-learn/src/runtime_feedback.rs:163` | `UpdateFrequency` |
| `crates/roko-learn/src/runtime_feedback.rs:323` | `LearningRuntime` |
| `crates/roko-learn/src/runtime_feedback.rs:364` | `EpisodeLogger::new(...)` wiring |
| `crates/roko-learn/src/runtime_feedback.rs:376` | `PatternMiner::new(...)` wiring |
| `crates/roko-learn/src/runtime_feedback.rs:782` | `record_completed_run(...)` |
| `crates/roko-learn/src/runtime_feedback.rs:888` | pattern ingestion in runtime flow |
| `crates/roko-learn/src/runtime_feedback.rs:1057` | experiment winner sync |

## Episodes And Patterns

| Anchor | Why it matters |
|--------|----------------|
| `crates/roko-learn/src/episode_logger.rs:90` | `GateVerdict` |
| `crates/roko-learn/src/episode_logger.rs:169` | `Episode` |
| `crates/roko-learn/src/episode_logger.rs:807` | `EpisodeLogger` |
| `crates/roko-learn/src/episode_logger.rs:860` | append path |
| `crates/roko-learn/src/episode_logger.rs:982` | compaction path |
| `crates/roko-learn/src/pattern_discovery.rs:99` | `PatternMiner` |
| `crates/roko-learn/src/hdc_clustering.rs` | k-medoids implementation |

## Knowledge Tiers

| Anchor | Why it matters |
|--------|----------------|
| `crates/roko-learn/src/playbook.rs:77` | `Playbook` |
| `crates/roko-learn/src/playbook_rules.rs:66` | `Rule` |
| `crates/roko-learn/src/playbook_rules.rs:116` | `MatchContext` |
| `crates/roko-learn/src/playbook_rules.rs:235` | rule selection |
| `crates/roko-learn/src/skill_library.rs:395` | `SkillQuery` |
| `crates/roko-learn/src/skill_library.rs:1119` | `search_by_tag(...)` |
| `crates/roko-learn/src/skill_library.rs:1543` | `select(...)` |
| `crates/roko-cli/src/orchestrate.rs:8208` | `build_learned_context(...)` |
| `crates/roko-neuro/src/tier_progression.rs:167` | `TierProgression` |
| `crates/roko-neuro/src/tier_progression.rs:207` | `analyze(...)` |

## Routing, Bandits, And Calibration

| Anchor | Why it matters |
|--------|----------------|
| `crates/roko-learn/src/active_inference.rs:17` | `BeliefState` |
| `crates/roko-learn/src/active_inference.rs:83` | `select_tier(...)` |
| `crates/roko-learn/src/bandits.rs:284` | `UcbBandit` |
| `crates/roko-learn/src/cascade_router.rs:994` | `CascadeRouter` |
| `crates/roko-learn/src/cascade_router.rs:1213` | active-inference tier hook |
| `crates/roko-learn/src/prompt_experiment.rs:135` | `PromptExperiment` |
| `crates/roko-learn/src/prompt_experiment.rs:395` | `ExperimentStore` |
| `crates/roko-learn/src/prediction.rs:14` | `PredictionRecord` |
| `crates/roko-learn/src/prediction.rs:125` | `CalibrationTracker` |
| `crates/roko-learn/src/prediction.rs:274` | `adjust_prediction(...)` |
| `crates/roko-learn/src/routing_log.rs:17` | `RoutingDecisionLog` |
| `crates/roko-cli/src/orchestrate.rs:253` | load calibration from workdir |
| `crates/roko-cli/src/orchestrate.rs:312` | predictive policy sections |

## Metrics, Cost, And Health

| Anchor | Why it matters |
|--------|----------------|
| `crates/roko-learn/src/efficiency.rs:80` | `AgentEfficiencyEvent` |
| `crates/roko-learn/src/regression.rs:69` | `RegressionAlert` |
| `crates/roko-learn/src/regression.rs:140` | `detect_regressions(...)` |
| `crates/roko-learn/src/budget.rs:8` | `BudgetGuardrail` |
| `crates/roko-learn/src/budget.rs:24` | `BudgetAction` |
| `crates/roko-learn/src/provider_health.rs` | provider health registry |
| `crates/roko-learn/src/latency.rs` | latency registry |

## Dormant Or Ambiguous

| Anchor | Why it matters |
|--------|----------------|
| `crates/roko-learn/src/drift.rs:89` | `DriftDetector` exists but still needs a live/dormant decision |
| `crates/roko-learn/src/event_subscriber.rs:48` | `run_learning_subscriber(...)` exists but still needs a live/dormant decision |

## Core Consumers Outside `roko-learn`

| Anchor | Why it matters |
|--------|----------------|
| `crates/roko-core/src/prediction.rs:51` | `PredictiveScorer` |
| `crates/roko-core/src/prediction.rs:174` | `PredictionPolicy` |
| `crates/roko-core/src/cfactor.rs:8` | `CFactorSummary` |
| `crates/roko-core/src/cfactor.rs:38` | `CFactorPolicy` |
| `crates/roko-neuro/src/lib.rs:128` | `KnowledgeTier` |

## Explicitly Absent And Therefore Deferred

These should not be described as present-tense runtime behavior:

- `EpisodeStorageConfig`
- `EpisodeCluster` / DBSCAN episode clustering
- `ToolUsageProfile`
- contextual Thompson / NeuralUCB / router ensembles
- Brier / reliability / arithmetic-corrector artifacts
- worldview / replication-ledger / constitutional layers

## Working Rule

If a doc section cannot be tied back to one of the anchors above, it should probably be written as `planned`, `target-state`, or `future work`.
