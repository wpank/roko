# E — Feedback + Calibration (Docs 13, 14, 15, 16)

Parity analysis of `docs/05-learning/13-8-missing-feedback-loops.md`,
`14-stability-mechanisms.md`, `15-collective-calibration-31x.md`,
`16-predictive-foraging.md` vs the actual codebase.

---

## E.01 — `LearningRuntime::record_completed_run` as the per-episode integration hub

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 13 §Cybernetic Theory + doc 14 §Frequency Separation imply a single orchestration point that fans out to all learning subsystems per episode with frequency separation applied.
**Reality**: `crates/roko-learn/src/runtime_feedback.rs:764-927` — `pub async fn record_completed_run(&self, mut input: CompletedRunInput) -> Result<LearningUpdate, LearningRuntimeError>`. The body is a ~165-line sequential pipeline that dispatches (in order): episode log append, affect signature, cost record, provider health, playbook outcomes, playbook rule outcomes, matched-skill outcomes, skill extraction, task metric, C-Factor snapshot, pattern miner ingest, cascade router observation+save, prompt experiment outcome + winner promotion, and local reward observations for `router`/`skill`/`playbook_rule`. Hub is invoked from `crates/roko-cli/src/orchestrate.rs:7470-7475` via wrapper `record_and_check_learning`; that wrapper has 12+ call sites in orchestrate.rs (e.g. lines 5323, 5760, 7389, 8331, 8749, 8989, 9045, 9202, 9280, 9384).
**Notes**: `LearningRuntime` struct declared at `runtime_feedback.rs:323-345` holds 15 subsystem handles. The 165-line body is where the "13 subsystem updates per episode" pattern actually happens. This is the single most important integration point in `roko-learn`.

---

## E.02 — Loop 1 · Health → Routing

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 13 §Loop 1 — `ProviderHealthRegistry::is_available()` filters the cascade router candidate set; 15 lines in `cascade_router.rs`.
**Reality**: `ProviderHealthTracker` handle is stored on `LearningRuntime` (`runtime_feedback.rs:331`) and updated in `record_completed_run` at `runtime_feedback.rs:805-812` (`record_success`/`record_failure` per provider). Cascade router references provider-health 11 times in `crates/roko-learn/src/cascade_router.rs` (filter calls during candidate scoring). Doc's self-report: "Wired."

---

## E.03 — Loop 2 · Conductor → Routing

**Status**: PARTIAL (LOW severity)
**Doc claim**: Doc 13 §Loop 2 — `SystemLoadSnapshot` feeds `RoutingContext` and biases `CascadeRouter::select()` toward cheaper tiers under pressure. Doc self-reports "Wired (pressure heuristic)" with gap around richer resource telemetry.
**Reality**: Conductor pressure is wired through `routing_ctx.conductor_load`, `active_agents`, and `ready_queue_depth` at `crates/roko-cli/src/orchestrate.rs:9753-9756` (populated from `routing_load_snapshot()` at `orchestrate.rs:3944`). Cascade router applies it via `apply_cost_pressure` (`cascade_router.rs:1547`) and the "Prefer cheaper tiers when live load or budget pressure is high" bias comment at line 228. No `SystemLoadSnapshot` struct literally exists; the doc's theoretical source type is not the actual wire type.
**Fix sketch**: Update doc to reference `RoutingLoadSnapshot` (the real type) and the `conductor_load`/`active_agents`/`ready_queue_depth` triad instead of the fictional `SystemLoadSnapshot`.

---

## E.04 — Loop 3 · Section → Scaffold

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 13 §Loop 3 — `PromptSectionMeta` from efficiency events + `SectionEffectivenessTracker` → section weights that re-prioritize the prompt composer. Doc self-reports "Wired (live orchestration path)" with a gap around richer weighting.
**Reality**: `SectionEffectivenessRegistry` lives on `LearningRuntime` (`runtime_feedback.rs:343`, loaded via `load_or_new` at line 385-386). Section effect types at `crates/roko-learn/src/section_effect.rs:30` (`SectionEffect`), `:114` (`SectionEffectivenessRegistry`), `:17` (`PriorityChange` enum), `:96` (`recommend_priority_change`), `:181` (registry lookup by `section_name` + `role`). Consumed from orchestrate via `apply_section_effectiveness_to_prompt_section` with 8 call sites at `orchestrate.rs:10291-10432` and definition at `orchestrate.rs:12788`.
**Notes**: 386 LOC total in `section_effect.rs`, 5 tests inline.

---

## E.05 — Loop 4 · Failure → Replanning

**Status**: PARTIAL (MEDIUM severity)
**Doc claim**: Doc 13 §Loop 4 — consecutive gate failures trigger replanning strategies (retry/escalate/decompose). Doc self-reports "Wired (orchestrator path)" with a gap that the standalone `roko prd plan` generator is not tightly coupled.
**Reality**: `orchestrate.rs` contains 119 matches for `replan|auto_replan|MaxIterations|ReplanStrategy` tokens — replan strategies are implemented in the live orchestrator loop. However, no sink call into the standalone `roko prd plan` subcommand: the replanning happens in-process via subtask insertion, not via re-invoking the PRD-driven plan generator. The sophisticated `FailureAnalysis` + `FailureRecommendation` enum shown in doc 13 does not exist in code (grep confirms no such types).
**Fix sketch**: Doc 13 §Loop 4 recipe (80 LOC estimate for `FailureAnalysis`/`FailureRecommendation`) describes an aspirational design; code uses a simpler per-plan failure counter + strategy dispatch.

---

## E.06 — Loop 5 · Skills → Prompts

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 13 §Loop 5 — `SkillLibrary::search_by_task` → system prompt builder section. Doc self-reports "Wired (orchestration-layer injection)" with a gap around moving the section to be native inside `SystemPromptBuilder`.
**Reality**: Skill section built in `orchestrate.rs:7089-7093` (`"## Relevant Skills from Past Successes"` header, then per-skill `- **{name}**: {summary}` lines, pushed to `parts`). `SkillLibrary` imported from `roko-learn` into `roko-compose` at `crates/roko-compose/src/system_prompt_builder.rs:38` and `role_prompts.rs:26`. The orchestrator constructs a `skill-library`-flavored section before composition runs rather than the prompt builder owning the section type.
**Notes**: Doc 13 wiring-recipe's proposed `add_skill_section(skills)` method does not exist; the actual code injects by string-format concatenation.

---

## E.07 — Loop 6 · Cost → Routing

**Status**: PARTIAL (MEDIUM severity)
**Doc claim**: Doc 13 §Loop 6 — `BudgetGuardrail` checks cumulative cost and returns `Allow|Warn|Block`; cascade router should treat this as first-class candidate scoring input. Doc self-reports "Wired (pre-dispatch guardrail)" with the gap that it's a pre-dispatch override rather than scoring input.
**Reality**: `pub struct BudgetGuardrail` at `crates/roko-learn/src/budget.rs:8`, `pub enum BudgetAction` at `:24` with `Block` and `BlockNewSessions` variants. Used in `crates/roko-cli/src/orchestrate.rs` and `crates/roko-agent/src/task_runner.rs` as a pre-dispatch gate. No evidence in `cascade_router.rs` of BudgetGuardrail integration — the candidate-scoring stage does not read the guardrail directly. The router has `apply_cost_pressure` (line 1547) which applies a boolean "spike" flag, not the multi-level `Continue/Downgrade/Block/HardStop` enum the doc describes.
**Fix sketch**: Either move `BudgetGuardrail::check()` into `CascadeRouter::select()` scoring, or update doc to acknowledge the override-style wiring.

---

## E.08 — Loop 7 · Latency → Reward

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 13 §Loop 7 — latency enters the bandit reward via `LatencyRegistry` ewma; `compose composite_reward = quality + cost + latency`. Doc self-reports "Wired."
**Reality**: `latency_sla_ms` field threaded through `cascade_router.rs` (18+ references, lines 115, 203, 1320, 1426, and every `default_latency_sla(tier)` construction). Reward scalarization at `crates/roko-learn/src/model_router.rs:316-347`: `compute_routing_reward_v2(pass_rate, normalized_cost, observed_latency_ms, latency_sla_ms)` normalizes duration against SLA and calls the composite reward function. Also `latency_weight: 0.3` default at `crates/roko-core/src/tool/metrics.rs:207,224`.
**Notes**: Reward computation uses `observed_latency_ms / latency_sla_ms` (clamped to 1.0) rather than the piecewise function doc 13 describes at §Loop 7 code sample, but the shape is equivalent.

---

## E.09 — Loop 8 · Experiments → Static

**Status**: PARTIAL (LOW severity)
**Doc claim**: Doc 13 §Loop 8 — concluded experiments materialize winners back into static config/routing table. Doc self-reports "Wired (persisted winner + router sync)" with gap around materializing into human-edited `roko.toml`.
**Reality**: `on_experiment_concluded` method at `runtime_feedback.rs:1038-1061` updates the cascade router's static routing table when a prompt experiment concludes: looks up `experiment.winner_id` + `experiment.role`, calls `update static routing table: experiment={} winner={} role={}` log line. Persisted experiment state lives in `ExperimentStore::load_or_new(&paths.experiments_json)` at `runtime_feedback.rs:383`. Neither `roko config apply-experiments` subcommand nor any TOML-mutation logic exists — winners stay in JSON state only.
**Fix sketch**: Implement the `roko config apply-experiments` subcommand from doc 13's §Loop 8 recipe, or soften doc wording to "static routing table sync — no TOML materialization."

---

## E.10 — Hysteresis + frequency separation (stability)

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 14 §Hysteresis — 10% score delta for model switching; §Frequency Separation — every-1 / every-5 / every-20 / every-50 cadence.
**Reality**:
- Hysteresis: `const HYSTERESIS_THRESHOLD: f64 = 0.10` at `crates/roko-learn/src/cascade_router.rs:247`. `fn select_with_hysteresis` at line 884. Three production call sites at `cascade_router.rs:2550, 2573, 2734`. Two tests at lines 3693-3719 (`routing_hysteresis_keeps_incumbent_below_threshold`, `routing_hysteresis_switches_at_threshold`).
- Frequency separation: `pub struct UpdateFrequency` at `runtime_feedback.rs:163` with `router_every_n_episodes`, `gate_thresholds_every_n`, `experiments_every_n`, `skill_mining_every_n`, `pattern_discovery_every_n`, `distiller_every_n` fields. `Default` at line 205: router=1, gate_thresholds=5, experiments=1, skill_mining=10, pattern_discovery=20, distiller=50. Due-checks at lines 184-202.

**Notes**: Doc 14 table says "Every 1 / Every 5 / Every 20 / Every 50" for routing / gate-thresholds / pattern-discovery / Pareto+CFactor — code matches exactly (distiller_every_n=50 covers the C-Factor snapshot at `runtime_feedback.rs:862-864`). Doc places experiments at "every 50 episodes" but code ships `experiments_every_n: 1`. Skill mining at `every 10` is not mentioned in doc 14's table (doc implies every 20).
**Fix sketch**: Doc 14 §Frequency Separation table: add skill mining row (every 10) and correct the experiments cadence or explain why it is per-episode.

---

## E.11 — EMA damping + anti-pattern prevention

**Status**: PARTIAL (LOW severity)
**Doc claim**: Doc 14 §Damping table — Gate thresholds α=0.1, Cost EWMA α=0.2, Latency EMA α=0.1, LinUCB alpha decay `exp(-obs/60)`. §Anti-Pattern table lists model lock-in, playbook explosion, cost death spiral, threshold collapse — each with "specific stability mechanism."
**Reality**: EMA usage: `crates/roko-gate/src/adaptive_threshold.rs` (9 matches), `crates/roko-gate/src/gate_pipeline.rs` (1), `crates/roko-gate/src/integration_gate.rs` (16), `verify_chain_gate.rs` (2), `env_builder.rs` (1). Latency and cost EWMA live on `LatencyRegistry`/`CostsLog`. `LinUCB` alpha decay and `UCB1` exploration live in `bandits.rs` (not re-verified here — scaffolded per prior parity docs). Playbook `min_confidence` pruning: 5 matches in `playbook_rules.rs`.
No centralized "anti-pattern detector" module exists — the four anti-patterns are prevented distributedly by hysteresis + EMA + cadence, as doc 14 §Compound Stability describes. No `HedgeBudget` or `StabilityBudget` struct (grep-negative confirmed: 0 matches for either token anywhere in `crates/`).
**Fix sketch**: Doc 14 §Stability Budget describes an abstract "budget" concept but there is no `StabilityBudget` struct — soften language to "notional budget" or implement.

---

## E.12 — C-Factor 11-component composite

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 15 §Components lists 11 fields (gate_pass_rate, cost_efficiency, speed, information_flow_rate, first_try_rate, knowledge_growth, knowledge_integration_rate, task_diversity_coverage, convergence_velocity, turn_taking_equality, social_sensitivity).
**Reality**: `pub struct CFactorComponents` at `crates/roko-learn/src/cfactor.rs:95-123` contains exactly these 11 fields in the same order. `pub struct CFactor` at `cfactor.rs:16-31` holds `overall`, `components`, `agent_contributions`, `pathologies`, `computed_at`, `episode_count`. Persisted via `append_cfactor_snapshot` at `runtime_feedback.rs:1086-1088, 1330, 1415`, called every 50 episodes (`distiller_due`) from the hub. 1847 LOC total in `cfactor.rs`, 17 inline tests.

---

## E.13 — Leave-one-out `AgentCFactorContribution` + `AgentDispatchBias`

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 15 §Leave-One-Out Contributions — per-agent contribution score = full − leave-one-out overall. Bias enum `PreferStronger|PreferCheaper|Neutral` drives cascade router bias.
**Reality**: `pub struct AgentCFactorContribution` at `cfactor.rs:39-48` with `agent_id`, `episode_count`, `without_agent_overall`, `contribution_score`. `pub enum AgentDispatchBias` at `cfactor.rs:52-59` with all three variants. `dispatch_bias_for_agent` method at `cfactor.rs:209-221` uses `-0.05` / `+0.05` thresholds plus `self.overall >= 0.65` gate before returning `PreferCheaper`. Bias consumed in `cascade_router.rs:2665, 2666, 2693, 2694` — `PreferStronger → strongest_model()`, `PreferCheaper → cheapest_model()`.

---

## E.14 — `CollectivePathology` detection

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 15 does not describe pathology detection directly — it lives as part of the C-Factor snapshot that doc 15 §C-Factor Regression references.
**Reality**: `pub enum CollectivePathology` at `cfactor.rs:63-91` with 5 variants: `Cascade { trigger_agent, affected_count }`, `Groupthink { diversity_score }`, `EchoChamber { repeated_knowledge_pct }`, `Deadlock { blocked_agents }`, `Hallucination { ungrounded_claims }`. `fn detect_pathologies` at `cfactor.rs:555`, fan-out to 5 private detectors at lines 587, 623, 647, 673, 718. `CFactor.pathologies: Vec<CollectivePathology>` field at `cfactor.rs:26`. Test coverage at lines 1802-1844.
**Notes**: `CollectivePathology` is richer than doc 15 advertises; doc 15 should add a subsection documenting the 5 pathology categories.
**Fix sketch**: Add §Collective Pathologies subsection to doc 15 listing the 5 variants.

---

## E.15 — 31.6× heuristic framing

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 15 §31.6× Heuristic — `accuracy(t) = 1 − 1/√(N × t)`, explicit caveats (independence, stationarity, aggregation, finite-sample, heterogeneity), guidance "expect 3-10× in practice."
**Reality**: No code implements the 31.6× formula literally. C-Factor is the practical measurement surrogate; doc 15 itself flags this is a "heuristic upper bound with explicit caveats, not a proven theorem." The `CFactor.overall` score is the measurement; the 31.6× is a framing device.
**Notes**: This is correctly represented — code does not claim more than doc does.

---

## E.16 — `PredictionRecord` struct + `CalibrationTracker`

**Status**: PARTIAL (MEDIUM severity)
**Doc claim**: Doc 16 §CalibrationTracker — `CalibrationTracker { predictions: Vec<PredictionRecord> }`, records predictions + actuals, computes calibration.
**Reality**: `pub struct PredictionRecord` at `crates/roko-learn/src/prediction.rs:14-47` with `task_id`, `model_slug`, `task_category`, `complexity`, `predicted_success_prob`, `predicted_cost_usd`, `predicted_duration_ms`, `actual_success`, `actual_cost_usd`, `actual_duration_ms`, `residual_success`, `residual_cost`, `residual_duration`, `timestamp`. `pub struct CalibrationTracker` at `prediction.rs:125-128` stores residuals keyed by `(model, category)`. `PredictionRecord::register`, `resolve`, `from_routing_log` methods all exist. `CalibrationTracker::load_from_routing_log(...)` is used by `orchestrate.rs:237-244`, and the loaded tracker already feeds `PredictionPolicy` sections plus `PredictiveScorer` at `orchestrate.rs:10415-10459`.
**Critical gap**: the **direct** `PredictionRecord::register/resolve` path is unused outside `prediction.rs`, while the real runtime path reconstructs calibration from the routing log after the fact. That means doc 16 currently mixes two different stories: a direct prediction-record pipeline that is not live, and a routing-log replay path that is live but narrower than the doc implies. `CalibrationTracker::record_prediction` also has no non-test caller.
**Fix sketch**: Make the routing-log replay path the explicit source of truth unless there is a strong reason to add a second live path. If the direct `register/resolve` contract is kept, wire it deliberately; otherwise demote it from “primary runtime story” in the docs.

---

## E.17 — Brier score, reliability diagrams, arithmetic corrector

**Status**: NOT DONE
**Severity**: HIGH
**Doc claim**: Doc 16 §Calibration Metric — Brier score formula, reliability diagram binning, arithmetic corrector with `correction_factor = actual_mean / predicted_mean` applied in ~50 ns per decision.
**Reality**: Grep-negatives all confirmed across `crates/`:
- `BrierScore|brier_score` → 0 hits
- `ReliabilityDiagram|reliability_diagram` → 0 hits
- `ArithmeticCorrector|arithmetic_corrector|calibrate_prediction|corrected_prediction` → 0 hits
- `PredictiveForager|PredictiveForagingEngine` → 0 hits

The `CalibrationTracker` already supports residual summaries and adjustment helpers, and the tracker is loaded into predictive prompt/scoring consumers via routing-log replay. What is still absent is the richer calibration surface doc 16 describes: no Brier computation, no reliability-diagram binning, no explicit arithmetic-corrector artifact, and no routing path that consumes a calibrated probability as a first-class decision input. Doc 16 §Integration with Routing ("calibrated 0.55 routes to opus") still has no code path.
**Fix sketch**: Doc 16 should be downgraded to "design intent" until the Brier/reliability/corrector pipeline ships. Alternatively, implement the arithmetic corrector as described — it is claimed to be "~50 ns per decision" so the implementation cost is low.

---

## E.18 — Dead modules: `DriftDetector`, `run_learning_subscriber`

**Status**: NOT DONE (dead code)
**Severity**: LOW (no doc claims either module as shipping)
**Doc claim**: Neither doc 13-16 mentions `DriftDetector` or `run_learning_subscriber` by name; doc 14 §Anti-Pattern lists "threshold collapse" prevention without pointing to the drift module.
**Reality**:
- `pub struct DriftDetector` at `crates/roko-learn/src/drift.rs:89`, 450 LOC, 4 tests. Exactly **0 external callers**: grep for `DriftDetector::new|DriftDetector::|DriftDetector\{` outside `drift.rs` finds only the in-module test at `drift.rs:340`. The `spec_drift::SpecDriftWatcher` hit in `roko-conductor` is an unrelated module.
- `pub async fn run_learning_subscriber` at `crates/roko-learn/src/event_subscriber.rs:48` (394 LOC fan-out), **0 external callers** — grep for `run_learning_subscriber(` only finds 2 references, both inside the same file at lines 266 and 342, both in its own tests.
- `pub enum AgentEvent` + `pub struct EventBus` at `crates/roko-learn/src/events.rs:15, 80` (160 LOC) are referenced from `orchestrate.rs` (7), `roko-agent/src/task_runner.rs` (8), `roko-serve/src/dispatch.rs` (3) — the bus itself is live, but the learn-side subscriber fan-out is unused.

**Fix sketch**: Either wire the subscriber (the event bus is emitting events that nothing consumes on the learn side) or flag both modules as "scaffold / planned" in the module docs.

---

## E.19 — `LocalRewardFunction` (Optimas-style)

**Status**: DONE
**Severity**: —
**Doc claim**: Not directly in docs 13-16; implied by doc 13 §Cybernetic Theory's "each subsystem learns from global outcomes."
**Reality**: `pub struct LocalRewardFunction` at `crates/roko-learn/src/local_reward.rs:18`, 126 LOC, 3 tests. Recorded per-episode in `record_completed_run` at `runtime_feedback.rs:911-925` for three subsystems: `router` (keyed by model), `skill` (keyed by skill id), `playbook_rule` (keyed by rule id). Persisted via `save_local_rewards()`.
**Notes**: This is the concrete mechanism by which "each local decision learns from global task success" — doc 13 should cite it.
**Fix sketch**: Add §Local Reward Functions subsection to doc 13 referencing `LocalRewardFunction`.

---

## Section Summary

| Status | Count |
|--------|-------|
| DONE | 11 |
| PARTIAL | 6 (E.03, E.05, E.07, E.09, E.11, E.16) |
| NOT DONE | 2 (E.17, E.18) |
| SCAFFOLD | 0 |

Total items: 19 (E.01–E.19).

### 8 Feedback Loops Status Matrix (doc 13 self-reported vs verified code)

| # | Loop | Doc 13 Self-Report | Verified Status | Evidence |
|---|------|--------------------|-----------------| --------|
| 1 | Health → Routing | Wired | **DONE** | E.02: `is_available` called 11× in `cascade_router.rs` |
| 2 | Conductor → Routing | Wired (pressure heuristic) | **PARTIAL** | E.03: pressure via `routing_ctx.conductor_load`, no `SystemLoadSnapshot` type |
| 3 | Section → Scaffold | Wired (live orch path) | **DONE** | E.04: 8 call sites of `apply_section_effectiveness_to_prompt_section` |
| 4 | Failure → Replanning | Wired (orch path) | **PARTIAL** | E.05: 119 replan-token matches in orchestrate.rs; no standalone generator sink |
| 5 | Skills → Prompts | Wired (orch-layer injection) | **DONE** | E.06: `orchestrate.rs:7089-7093` skill section composition |
| 6 | Cost → Routing | Wired (pre-dispatch guardrail) | **PARTIAL** | E.07: `BudgetGuardrail` is pre-dispatch override, not router scoring input |
| 7 | Latency → Reward | Wired | **DONE** | E.08: `compute_routing_reward_v2` + `latency_sla_ms` threading |
| 8 | Experiments → Static | Wired (winner + router sync) | **PARTIAL** | E.09: router sync works, no TOML/config materialization |

**Aggregate**: 4 of 8 loops fully wired, 4 partial. Doc 13's own self-report of "all 8 wired with explicit gaps" is accurate — the gaps it names are the same gaps this audit finds.

### Major gaps vs docs

- **E.16 + E.17 (doc 16)**: predictive calibration is only partially real. Routing-log replay plus predictive prompt/scoring consumers ship; the richer Brier / reliability / arithmetic-corrector pipeline does not, and the direct `PredictionRecord::register/resolve` path is still unused.
- **E.18**: `DriftDetector` (450 LOC) and `run_learning_subscriber` (394 LOC) are dead modules with 0 external callers. These are not doc claims but represent ~850 LOC of unwired scaffolding.
- **E.03, E.07, E.09**: Each Loop uses a different real type than doc 13's recipes claim (`RoutingLoadSnapshot` vs `SystemLoadSnapshot`; `BudgetAction::Block` vs `BudgetRoutingAction::HardStop`; cascade router table vs `roko.toml` mutation). The substance is present; the nomenclature is drifted.

The feedback+calibration layer is the most complete part of the learning stack by LOC, and the 11 DONE items confirm that — but doc 16's predictive foraging claims are the largest single drift in `docs/05-learning/`: an entire pipeline documented as shipping that has no integration point.

## Agent Execution Notes

### E.16 / E.17 — Predictive Calibration Canonicalization

This batch should start by choosing one canonical calibration path.

Recommended slice:

1. prefer the routing-log path if it already contains the needed fields,
2. add at least one real calibration metric or summary on that path,
3. keep prompt/scorer consumers aligned with the same source of truth.

Acceptance criteria:

- later agents can point to one canonical calibration data source,
- a real calibration metric exists beyond raw residual storage,
- docs stop implying a richer predictive-foraging pipeline than runtime actually ships.

### E.07 / E.09 — Partial Loop Hardening

Good outcome:

- cost pressure becomes more than a narrow override,
- experiment winners leave a clearer durable artifact,
- the current loops become easier to operate and debug.

### E.18 — Dead Module Resolution

If `run_learning_subscriber` and `DriftDetector` stay out of path, say so explicitly. Do not leave them as ambiguous “maybe-live” subsystems.
