# F — Theory & Learning Frontier (Docs 07 + 08 + 11 + 15)

Docs 07 (OODA / cybernetic loop), 08 (Good Regulator / self-model),
11 (anomaly detection / learning integration), and 15 (conductor
learning / federation / self-healing) are the conductor-chapter's
theory + learning frontier. Most of each doc is deliberately
aspirational — control-theory framing that names primitives the code
may or may not ship. This file audits every load-bearing technical
claim against the current tree. Most items are grep-negative and
low-severity (design-only frontier); a few are DONE because the
underlying primitives do exist (anomaly detector, ConductorBandit,
adaptive gate thresholds, efficiency events). Two doc claims are
materially drifted — both LOW/MEDIUM severity — and are called out
as such.

Generated: 2026-04-16

---

## F.01 — OODA framing: conductor evaluate() is the loop body (Doc 07 §OODA Framework)

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 07 lines 13-79 frame each conductor tick as one OODA iteration: Observe (signal stream read), Orient (watcher outputs), Decide (`WorstSeverityPolicy`), Act (`ConductorDecision` returned to orchestrator). Banner at line 8: "Implementation: Built."
**Reality**: The body of `Conductor::evaluate()` at `crates/roko-conductor/src/conductor.rs:156-187` maps 1:1 to the four OODA stages: circuit-breaker lookup (early-fail path at `:158-166`), `collect_watcher_outputs` at `:169` (Observe + Orient), `self.policy.evaluate(...)` at `:173` via `WorstSeverityPolicy` at `:98` (Decide), and return of a `ConductorDecision` at `:186` (Act — the orchestrator translates decisions at `crates/roko-cli/src/orchestrate.rs:3910` where `self.conductor.evaluate(&signals, &ctx)` is called). The doc never names an explicit `Observe`/`Orient`/`Decide`/`Act` type — `Grep 'struct Observe|struct Orient|struct Decide|struct Act|enum OodaPhase'` of `crates/` returns **zero matches** — but doc 07 does not claim such types exist; it uses OODA purely as a conceptual frame. The conductor plumbing holds.

---

## F.02 — Signal kinds: TokenUsage, GateVerdict, AgentOutput, PlanPhase, Metric, Custom("conductor.agent_output") (Doc 07 §Observe)

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 07 lines 24-32 list exactly six signal kinds consumed by the conductor: `TokenUsage`, `GateVerdict`, `AgentOutput`, `PlanPhase`, `Metric`, `Custom("conductor.agent_output")`.
**Reality**: All six kinds exist and are consumed. `Kind::TokenUsage`, `Kind::GateVerdict`, `Kind::AgentOutput`, `Kind::PlanPhase`, `Kind::Metric` are canonical variants in `roko_core::Kind`. `Custom("conductor.agent_output")` is emitted from `Conductor` tests at `crates/roko-conductor/src/conductor.rs:509` and read by the time-overrun watcher at `crates/roko-conductor/src/watchers/time_overrun.rs:13` (`pub const TASK_OUTPUT_KIND: &str = "conductor.agent_output";`). The orchestrator pushes it in the conductor-signal enrichment path at `crates/roko-cli/src/orchestrate.rs:11028`. All six kinds land in the signal stream that `evaluate()` reads.

---

## F.03 — WorstSeverityPolicy + 3-decision output (Doc 07 §Decide / §Act)

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 07 lines 54-65 claim `WorstSeverityPolicy` resolves watcher assessments to a single `ConductorDecision::Continue | Restart | Fail`. Lines 71-75 claim a fixed 3-decision enum mapped to concrete orchestrator actions.
**Reality**: `WorstSeverityPolicy` is the concrete impl of `InterventionPolicy` at `crates/roko-conductor/src/interventions.rs:109-111`, selected as the default in `Conductor::new()` at `crates/roko-conductor/src/conductor.rs:98` and `:118`. Severity maps 1:1 to `ConductorDecision` via `Severity::to_decision` at `:36-44` (`Info → cont()`, `Warning → restart(...)`, `Critical → fail(...)`). `ConductorDecision` is the canonical 3-variant enum in `roko_core` as used across the conductor and orchestrator.

---

## F.04 — 10 watchers + circuit breaker ensemble (Doc 07 §Observability / §OODA Loop Speed table)

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 07 lines 197-199 claim "All 10 watchers < 1 ms (stream scan, no I/O)"; line 253 claims "10 watcher types × configurable thresholds"; line 495 test asserts `assert_eq!(c.watchers.len(), 10)`.
**Reality**: `Conductor::new()` instantiates exactly 10 watchers at `crates/roko-conductor/src/conductor.rs:83-94` (`GhostTurnWatcher`, `ReviewLoopWatcher`, `IterationLoopWatcher`, `TestFailureBudgetWatcher`, `CompileFailRepeatWatcher`, `ContextWindowPressureWatcher`, `SpecDriftWatcher`, `CostOverrunWatcher`, `TimeOverrunWatcher`, `StuckPatternWatcher`). Unit test `watcher_count` at `:491-495` pins the count at 10. Every watcher has a `pub const MAX_*` threshold file-local (`MAX_GHOST_TURNS=3`, `MAX_REVIEW_CYCLES=3`, `MAX_CONTEXT_USAGE_RATIO=0.80`, `MAX_SPEC_DRIFT_RATIO=0.25`, `ALERT_THRESHOLD=0.80`, `MAX_PLAN_FAILURES=2` — see grep above) — matching the Doc 08 §Behavioral Norms table row-for-row.

---

## F.05 — LivenessMonitor struct (Doc 07 §Observation latency)

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Doc 07 lines 337-350 propose a `LivenessMonitor` with `expected_interval`, `last_heartbeat: DashMap<String, Instant>`, `warning_multiplier`, `critical_multiplier`, as a dedicated heartbeat-based liveness monitor to close the "blind window" gap between events.
**Reality**: `Grep 'LivenessMonitor'` on `crates/` returns **zero matches**. The doc text is self-aware — it frames the monitor as a design proposal ("would close this gap"), not a shipping component. The existing health monitor at `crates/roko-conductor/src/health.rs` has checks but no heartbeat liveness primitive. The 10-second health tick mentioned in the comparison table at doc 07 line 158 is referenced conceptually but no per-agent heartbeat DashMap exists.
**Fix sketch**: This is labeled as frontier in the doc itself ("A dedicated liveness monitor would close this gap"). No banner-drift — doc 07 banner says "Built" for the existing loop, not for `LivenessMonitor`. Leave as designed-only; build when silent-hang detection becomes a blocker.

---

## F.06 — ImplicitGuidance / IG&C pre-compiled rules (Doc 07 §Implicit Guidance and Control)

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Doc 07 lines 372-402 describe Boyd's "Implicit Guidance and Control" shortcut: a pre-compiled `ImplicitGuidance` struct with `rules: Vec<ImplicitRule>` where each rule has `name`, `matcher: Box<dyn Fn(&[Signal]) -> bool>`, `action: ConductorDecision`, `min_confidence: f64`. Claims IG&C rules should be "extracted from the `ConductorBandit`'s converged actions" when a bandit arm converges to >95%.
**Reality**: `Grep 'ImplicitGuidance|ImplicitRule|IGaC'` on `crates/` returns **zero matches**. The bandit exists (`ConductorBandit` — see F.12) but no convergence-extraction pipeline ships. Again, doc 07 frames this as a proposal, not as a shipping component — "should not be hand-written" (line 398).
**Fix sketch**: Implement alongside F.05 when hand-written watcher patterns become a maintenance burden. Not urgent.

---

## F.07 — Nested OODA: ParameterCascade + Delta/Theta/Gamma typed structs (Doc 07 §Nested OODA loops)

**Status**: PARTIAL
**Severity**: LOW
**Doc claim**: Doc 07 lines 478-506 specify a typed `ParameterCascade { delta, theta, gamma }` containing `DeltaParameters { default_model_tier, base_cost_budget_usd, gate_threshold_adjustments }`, `ThetaParameters { adjusted_stuck_threshold, adjusted_ghost_turn_max, current_pressure_level }`, and `GammaParameters { watcher_thresholds, intervention_cooldown }`. Lines 456-461 claim a three-level timescale decomposition (Gamma per turn, Theta per task, Delta per batch). Line 520: Delta runs per batch (~hours), Theta per task (~75s), Gamma per turn (~5s).
**Reality**: The typed struct does not exist. `Grep 'ParameterCascade|DeltaParameters|ThetaParameters|GammaParameters|delta_loop|theta_loop|gamma_loop'` returns **zero matches**. What does exist: `OperatingFrequency::Theta` is used by the meta-cognition hook at `crates/roko-conductor/src/stuck_detection.rs:264, 504, 582` (`MetaCognitionHook::frequency() -> OperatingFrequency::Theta`), confirming one of the three timescales has a frequency tag. Gate-adaptive thresholds (`crates/roko-gate/src/adaptive_threshold.rs`) and the cascade router (`crates/roko-learn/src/cascade_router.rs`) provide Delta-like cross-batch learning surfaces. However, the parameter-cascade contract that the doc promises — slower loops writing typed parameters consumed by faster loops — is not a named type; it is implicit across separate subsystems. No explicit Gamma/Theta/Delta separation-of-concerns enforcement.
**Fix sketch**: The three-tier OODA framing is a design narrative, not a load-bearing API. Tag doc 07 §"Nested OODA loops" as "Designed — emergent via frequency tags on separate subsystems; no typed `ParameterCascade` ships yet." If a typed cascade becomes useful, place it in `roko-conductor` next to `InterventionPolicy`.

---

## F.08 — Algedonic signals / priority interrupts (Doc 07 §Algedonic signals)

**Status**: PARTIAL
**Severity**: LOW
**Doc claim**: Doc 07 lines 538-598 specify four algedonic trigger conditions (runaway cost, safety violation, total infrastructure failure, operator interrupt) with escalating time windows (5s→30s→emergency shutdown) that bypass the normal Gamma/Theta/Delta hierarchy.
**Reality**: `Grep 'algedonic|Algedonic'` on `crates/` returns **zero matches** for a named primitive. However, three of the four trigger mechanisms do ship under different names: runaway-cost/budget-exhausted detection exists in `AnomalyDetector::check_budget` at `crates/roko-learn/src/anomaly.rs:120-131` and via `BudgetGuardrail` (`crates/roko-learn/src/budget.rs`) referenced from `orchestrate.rs:26`; safety violations are emitted by `crates/roko-agent/src/safety/` (path/bash guards); operator interrupts via `tokio::signal::ctrl_c` handling are plumbed through the CLI entrypoints at `crates/roko-cli/src/main.rs`, `crates/roko-cli/src/daemon.rs`, `crates/roko-cli/src/worker/mod.rs`. What is absent: an explicit time-window escalation ladder (5s→30s→emergency shutdown) and any single `AlgedonicSignal` / priority-interrupt type the conductor emits to bypass normal watcher evaluation.
**Fix sketch**: The building blocks ship; the aggregated escalation protocol does not. Add a note to doc 07 §"Algedonic signals" that each trigger is implemented as a separate subsystem (budget guardrail, path safety, tokio Ctrl+C handler) but that no unified `AlgedonicChannel` type consolidates them.

---

## F.09 — Good Regulator theorem framing + self-model as watchers (Doc 08 §Components of the Self-Model)

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 08 lines 16-22 invoke Conant-Ashby and frame the conductor's watchers/heuristics/error-categories/health-checks as the concrete self-model. Lines 30-96 enumerate four model components: (1) watcher thresholds, (2) 20 error categories, (3) 6 stuck kinds, (4) health checks.
**Reality**: All four components ship. (1) Watcher thresholds: each of the 10 watchers owns a `pub const MAX_*` matching the Doc 08 §Behavioral Norms table — see F.04. (2) `ErrorCategory` at `crates/roko-conductor/src/diagnosis.rs:26-67` has exactly 20 variants (`CompileError`, `TestFailure`, `ClippyWarning`, `GitConflict`, `DependencyError`, `TypeMismatch`, `BorrowCheckerError`, `LifetimeError`, `ImportError`, `MissingFile`, `PermissionDenied`, `NetworkError`, `TimeoutError`, `OomError`, `DiskFull`, `LlmRateLimit`, `LlmContextOverflow`, `LlmRefusal`, `ProcessCrash`, `LoopDetected`). (3) `StuckKind` at `crates/roko-conductor/src/stuck_detection.rs:34-47` has 6 variants (`OutputLoop`, `NoProgress`, `GateLoop`, `CompileLoop`, `EmptyOutput`, `ExcessiveRetries`). (4) `HealthStatus` + `HealthCheckResult` + `HealthMonitor` at `crates/roko-conductor/src/health.rs:26-80`. The Conant-Ashby framing is a framing, not a type — no `GoodRegulator` struct exists (`Grep 'GoodRegulator|ConantAshby'` returns zero matches), and the doc does not claim one.

---

## F.10 — Static thresholds are compile-time constants (Doc 08 §Current: Static Model)

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 08 lines 147-150 show `pub const MAX_CONTEXT_USAGE_RATIO: f64 = 0.80; pub const MAX_GHOST_TURNS: usize = 3; pub const MAX_COMPILE_FAIL_REPEAT: usize = 3;` as concrete compile-time constants.
**Reality**: All three constants ship with the claimed values. `MAX_CONTEXT_USAGE_RATIO: f64 = 0.80` at `crates/roko-conductor/src/watchers/context_window_pressure.rs:10`, `MAX_GHOST_TURNS: usize = 3` at `crates/roko-conductor/src/watchers/ghost_turn.rs:11`, `MAX_COMPILE_FAIL_REPEAT` in `crates/roko-conductor/src/watchers/compile_fail_repeat.rs`. The doc snippet is lifted directly from shipping code.

---

## F.11 — SelfModelAccuracy + BrierScoreTracker + CalibrationBin (Doc 08 §Self-Model Accuracy Metrics)

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Doc 08 lines 312-407 specify typed structs `SelfModelAccuracy { intervention_effectiveness, stuck_detection_precision, diagnosis_accuracy, completion_time_rmse_ms, gate_pass_brier_score, composite_accuracy }` and `BrierScoreTracker { sum_squared_error, count, calibration_bins }` and `CalibrationBin { range_low, range_high, actual_passes, total }` with an `impl BrierScoreTracker::record` + `brier_score` method pair.
**Reality**: `Grep 'SelfModelAccuracy|BrierScore|CalibrationBin|intervention_effectiveness|diagnosis_accuracy|gate_pass_brier_score'` on `crates/` returns **zero matches**. These are design-only types — the doc positions them as future metrics ("this section defines formal accuracy measurements," line 304) and the code does not ship them. There is no running calibration surface and no Brier-score computation anywhere in the learn/conductor crates.
**Fix sketch**: Good candidates for future `roko-learn` modules once intervention-outcome data accumulates. The doc doesn't falsely claim these ship — the banner at line 10 says "Built" referring to the static self-model, not these measurement types. LOW severity.

---

## F.12 — Bayesian ThresholdLearner + ThresholdPosterior (Doc 08 §Bayesian threshold adaptation)

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Doc 08 lines 439-499 specify `ThresholdLearner { watchers: HashMap<String, ThresholdPosterior> }` and `ThresholdPosterior { threshold, alpha, beta, discount, min_samples }` with an online Bayesian update rule (alpha += 1 on success, beta += 1 on failure, discount factor 0.995).
**Reality**: `Grep 'ThresholdLearner|ThresholdPosterior|ThresholdDirection'` returns **zero matches**. The `AdaptiveThresholds` type that does exist at `crates/roko-gate/src/adaptive_threshold.rs:44-47` uses EMA (not Beta posterior) over per-rung pass rates, with `RungStats { ema_pass_rate, total_observations, consecutive_passes }` at `:23-30`, alpha=0.1, min-retries floor=1, max-retries ceiling=5. That is a per-gate-rung adaptive threshold, not a per-watcher Bayesian conductor threshold. The conductor's watcher thresholds remain compile-time constants (F.10). The gate-rung EMA is the closest existing analogue but operates in a different subsystem with a different update rule.
**Fix sketch**: LOW severity — the doc explicitly positions the Bayesian learner under "Future: Adaptive Model" (line 156). When the ConductorBandit integration (F.15) graduates to adjust watcher thresholds (not just select actions), use `ThresholdLearner` as the shape of that extension.

---

## F.13 — ScalarKalman filter for parameter drift (Doc 08 §Kalman filter for state estimation)

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Doc 08 lines 515-544 specify a `ScalarKalman { estimate, uncertainty, process_noise, measurement_noise }` with standard Kalman update (`predict` adds process_noise; `update` applies Kalman gain) and a `prediction_error` getter.
**Reality**: `Grep 'ScalarKalman|kalman|Kalman|KalmanFilter|process_noise|measurement_noise'` on `crates/` returns **zero matches**. No Kalman-filter code ships anywhere in the tree. The closest online-estimator in the learn crate is the EWMA in `EwmaState` (`crates/roko-learn/src/anomaly.rs:152-188`), which is a different filter family (exponential smoothing, not Kalman gain). Doc 08 is explicit that this is a design proposal, not a shipping component.
**Fix sketch**: Keep as design. Once per-turn cost and completion-time drift become observable targets, add a small Kalman module in `roko-learn`.

---

## F.14 — PrecisionWeightedUpdater (active inference) + ForwardPredictor (Doc 08 §Active inference integration / §Forward prediction)

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Doc 08 lines 566-594 specify `PrecisionWeightedUpdater { context_precision, min_precision, max_precision }` with a `weighted_update(context, prediction_error, base_learning_rate)` method. Lines 647-683 specify `ForwardPredictor { weights, bias, feature_dim }` with `predict_pass_probability` (sigmoid) and SGD `update` methods. Doc 08 also leans on the Internal Model Principle (Francis-Wonham 1976) at lines 608-631 to require the conductor's forward model mirror the gate pipeline's dynamics.
**Reality**: `Grep 'PrecisionWeightedUpdater|ForwardPredictor|predict_pass_probability|InternalModelPrinciple|active_inference'` on `crates/` returns the following matches, all unrelated to the conductor self-model: `ActiveInferenceScorer` at `crates/roko-compose/src/scorer.rs:98-105` is a **prompt section scorer** (goal-directed context selection), not a forward predictor for gate outcomes. The `crates/roko-learn/src/active_inference.rs` module also exists and is re-exported from `crates/roko-learn/src/lib.rs` (see filename in the Glob), but it is active-inference for learning, not a `ForwardPredictor` for conductor self-modeling. No Brier-score forward model for gate-pass probability ships. The IMP framing is narrative only.
**Fix sketch**: Two separate design gaps. (a) `ForwardPredictor` belongs near `AdaptiveThresholds` in `roko-gate` or as a new `roko-conductor` module once enough efficiency events accumulate to train a logistic. (b) `PrecisionWeightedUpdater` would live in `roko-learn` alongside the active-inference primitives already there — confirm shapes before adding.

---

## F.15 — AnomalyDetector with EWMA, prompt-hash window, quality history, budget accumulator (Doc 11 §AnomalyDetector)

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 11 lines 12-33 describe `AnomalyDetector { prompt_hash_window: VecDeque<u64>, cost_ewma: EwmaState, quality_history: VecDeque<f64>, session_cost_usd: f64, session_start_ms: i64 }` with 4 `Anomaly` variants (PromptLoop, CostSpike, QualityDegradation, BudgetExhausted). Lines 41-56 show `check_prompt` triggering at 5+ identical hashes in a 20-window; lines 71-84 show EWMA update logic; lines 96-105 show quality-degradation dual condition (drop > 0.15 AND recent < 0.5).
**Reality**: Ships essentially verbatim. `AnomalyDetector` at `crates/roko-learn/src/anomaly.rs:18-26` has all five fields. `Anomaly` enum at `:205-229` has exactly the four variants claimed. `check_prompt` at `:52-69` implements `PROMPT_LOOP_WINDOW=20` and `PROMPT_LOOP_THRESHOLD=5` at `:9-10`. `EwmaState::update` at `:172-176` uses the exact update rule from doc 11 line 72-76 (`mean += alpha * diff; variance = (1.0 - alpha) * (variance + alpha * diff * diff)`). `z_score` at `:180-187` matches. `check_quality` at `:95-118` applies the dual condition `recent_avg < earlier_avg - 0.15 && recent_avg < 0.5` (line 113). `check_budget` at `:122-131` matches the `BudgetExhausted` body. Unit tests at `:236-310` cover all four anomalies. The doc is accurate and the code ships.

---

## F.16 — AnomalyDetector runs BEFORE each turn in dispatch pipeline (Doc 11 §Anomaly Detection in the Dispatch Pipeline)

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 11 lines 286-326 claim the anomaly detector integrates into the dispatch pipeline before each agent turn: `check_prompt(prompt_hash)` → possibly abort with `DispatchError::PromptLoop`; `check_cost(turn_cost_usd)` → log cost-spike warning; `check_budget(budget_limit_usd)` → possibly abort with `DispatchError::BudgetExhausted`.
**Reality**: The "anticipate, don't react" wiring ships. `drain_turn_learning_events` at `crates/roko-cli/src/orchestrate.rs:578-636` calls `anomaly_detector.check_prompt(feedback.prompt_hash)` on the `AgentEvent::TurnStarted` event (at `:583-584`, before the turn produces output) and `anomaly_detector.check_cost(feedback.cost_usd)` on `AgentEvent::CostRecorded` (at `:610-611`). The same `AnomalyDetector` instance is also threaded through `PlanRunner` at `:2198` and `TaskRunner` at `crates/roko-agent/src/task_runner.rs:24`. Two detectors cooperate: the orchestrator-level one (long-lived, session-wide — `:3279, 3398, 3521`) and the per-task-runner `RunnerAnomalyDetector` re-exported at `:26` and constructed at `:10590, 10701`. The integration point matches the doc exactly.

---

## F.17 — AgentEfficiencyEvent (20+ fields) shared across subsystems (Doc 11 §Efficiency Events)

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 11 lines 142-175 show `AgentEfficiencyEvent` with 20+ fields including `agent_id`, `role`, `backend`, `model`, `plan_id`, `task_id`, `input_tokens`, `output_tokens`, `reasoning_tokens`, `cache_read_tokens`, `cache_write_tokens`, `cost_usd`, `cost_usd_without_cache`, `prompt_sections`, `total_prompt_tokens`, `system_prompt_tokens`, `tools_available`, `tools_used`, `tool_calls`, `wall_time_ms`, `duration_ms`, `time_to_first_token_ms`, `was_warm_start`, `iteration`, `gate_passed`, `outcome`, `gate_errors`, `model_used`, `frequency: OperatingFrequency`, `strategy_attempted`, `timestamp`. Lines 178-186 show context-window-pressure watcher reading events directly to derive token usage.
**Reality**: `AgentEfficiencyEvent` ships at `crates/roko-learn/src/efficiency.rs:79-80` (struct opening; full struct is in that file after the type documentation header). `PromptSectionMeta` at `:34-46` and `ToolCallMeta` at `:51-70` are the nested field types. Context-window-pressure reads events at `crates/roko-conductor/src/watchers/context_window_pressure.rs:95` (`if let Some(total) = context_window_tokens(&event.model)`). Orchestrator wires `AgentEfficiencyEvent` emission at `crates/roko-cli/src/orchestrate.rs:7409` (`self.emit_efficiency_event(...)`) and `:7692` (`emit_failure_efficiency_event`), reads at `:1674` (`latest_efficiency_event`), and retains a `Vec<AgentEfficiencyEvent>` in runner state at `:2222, 3300, 3419, 3542`. Records land in `.roko/learn/efficiency.jsonl` (see CLAUDE.md).

---

## F.18 — Cascade router consumes conductor interventions as negative routing feedback (Doc 11 §Cascade Router Feedback)

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 11 lines 192-218 claim every conductor intervention produces a negative observation for the cascade router, routing future similar tasks away from bad model-context combinations. Table at lines 196-202 maps `Continue → positive`, `Restart (compile-fail-repeat) → negative`, etc.
**Reality**: The feedback path is wired end-to-end. `record_conductor_intervention` at `crates/roko-learn/src/runtime_feedback.rs:727-743` records an intervention as a cascade-router observation and calls `cascade_router.save` (with error-logging at `:743`: `eprintln!("[learn] cascade router save failed after conductor intervention: {err}")`). The orchestrator calls this at `crates/roko-cli/src/orchestrate.rs:4237-4249`: `self.learning.record_conductor_intervention(&routing_context, &model_slug, intervention)` followed by `tracing::info!(... "recorded conductor intervention as negative routing feedback")`. Test at `runtime_feedback.rs:2536` confirms the code path. CLAUDE.md already marks CascadeRouter as "Wired".

---

## F.19 — Adaptive gate thresholds — EMA-per-rung ships with configurable bounds (Doc 11 §Adaptive Gate Thresholds, Doc 08 reference path)

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 11 lines 220-231 claim `crates/roko-gate/src/adaptive_threshold.rs` uses EMA per gate rung with historical pass-rate adjustment. Doc 08 line 295 cross-references this as an "Adaptive model precedent (EMA thresholds)".
**Reality**: `AdaptiveThresholds` at `crates/roko-gate/src/adaptive_threshold.rs:44-47` holds `rungs: HashMap<u32, RungStats>`. `RungStats { ema_pass_rate, total_observations, consecutive_passes }` at `:23-30` starts neutral at `ema_pass_rate: 0.5` (`:35`). `EMA_ALPHA: f64 = 0.1` at `:11` (recent observations weigh more heavily). `MIN_RETRIES: u32 = 1` and `MAX_RETRIES: u32 = 5` at `:13-16` match the "floor/ceiling bounds" doc language. `SKIP_STREAK_THRESHOLD: u32 = 20` at `:19` gates auto-skip suggestions. `load_or_new` at `:58-60` persists to disk. CLAUDE.md confirms: "Adaptive gate thresholds | **Wired** | EMA per rung in `.roko/learn/gate-thresholds.json`".

---

## F.20 — ProviderHealthTracker + Registry for infrastructure-level breaker (Doc 11 §Provider Health Integration)

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 11 lines 330-352 describe a `ProviderHealthTracker` as a separate API-level circuit breaker (3 consecutive failures → open → cooldown → probe → close), independent of the Conductor's plan-level breaker.
**Reality**: `ProviderHealthTracker` at `crates/roko-learn/src/provider_health.rs:519-528` implements the three-state machine documented at `:6-13` (ASCII diagram: `Healthy → Unhealthy → Probing → Healthy/Unhealthy`). `ProviderHealthRegistry` at `:182-196` wraps multiple trackers with persistence (snapshot types at `:40-50`, `CircuitState { Closed, Open, HalfOpen }`). Uses `parking_lot::RwLock` for thread-safe concurrent access (`:16-18, :29`). Tests at `:733-752+` exercise tracker state transitions. The "separate from conductor breaker" claim is accurate — the conductor's breaker at `crates/roko-conductor/src/circuit_breaker.rs` (`MAX_PLAN_FAILURES: u32 = 2`) is distinct code.

---

## F.21 — ConductorBandit is fully wired, not "built, not wired" (Doc 15 File reference + §Contextual bandit)

**Status**: DONE (doc 15 is stale — banner says "Scaffold" and file reference says "built, not wired", but the bandit IS wired into `orchestrate.rs`)
**Severity**: MEDIUM
**Doc claim**: Doc 15 line 8 banner: "Implementation: Scaffold". Lines 21-29 claim "`ConductorBandit` in `roko-learn/src/conductor.rs` implements a contextual bandit … The conductor's decision path does not use any of this." File reference line 553: `crates/roko-learn/src/conductor.rs | ConductorBandit (built, not wired)`. Doc goes on to propose a `LearnedConductorPolicy` wrapper (lines 66-98) that the current code "does not" use.
**Reality**: `ConductorBandit` is wired into the orchestrator's retry path. Full decision flow at `crates/roko-cli/src/orchestrate.rs:6039-6298` threads a `ConductorBandit` through every task retry: `retry_conductor: ConductorBandit` field at `:2186`, loaded with persistence at `:3261, 3380, 3503` via `ConductorBandit::load_or_new(&conductor_policy_path(workdir))`, imported at `:27, 74`. Per-retry invocation: `self.retry_conductor.select_action(&state)` at `:6210`, then action-specific branches at `:6236 (Continue)`, `:6250 (InjectHint)`, `:6262 (SwitchModel)`, `:6275 (Restart)`, `:6282 (Abort)`. Outcome recording: `self.retry_conductor.record_outcome(&state, action, true/false)` at `:6089-6090, 6193-6194, 6229-6230`. Persistence: `self.persist_retry_conductor()` helper at `:6919-6921+` writes the bandit snapshot (`ConductorBandit::save` method exists at `crates/roko-learn/src/conductor.rs:175-200`). The 7-action enum in `ConductorBandit` (`Continue`, `InjectHint × 3 hint types`, `SwitchModel`, `Restart`, `Abort` at `:28-36`) is the full action set, and the 19-dimensional state encoding at `:249-291` matches the doc's description (iteration, failure_count, elapsed_ms, cost, model_tier, complexity, error_pattern one-hots, interaction terms at `x[16-18]`). Tests at `:553-596` confirm the learning behavior (abort dominates after repeated mechanical failures). The doc 15 "Scaffold" banner + "built, not wired" file-reference are materially drifted.
**Fix sketch**: Update doc 15 line 8 banner to "Partial" (since the learned-policy _conductor_ proper is not the same as the retry-path bandit) or "Built (retry path)". Update line 553 of the file reference table to: `ConductorBandit (wired into orchestrator retry path; not yet wired as InterventionPolicy replacement)`. The `LearnedConductorPolicy` wrapper that wraps `WorstSeverityPolicy` with a fallback (lines 69-98) is genuinely NOT implemented (see F.22), but the bandit itself is live.

---

## F.22 — LearnedConductorPolicy wrapper + warmup + min_confidence fallback (Doc 15 §Contextual bandit)

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Doc 15 lines 66-98 specify a `LearnedConductorPolicy { bandit: ConductorBandit, min_confidence: f64, warmup_observations: usize }` that implements `InterventionPolicy` and falls back to `WorstSeverityPolicy` when (a) observation count < warmup (default 50) or (b) confidence < min (default 0.6). `ConductorBandit::select_with_confidence` and `total_observations()` are referenced.
**Reality**: `Grep 'LearnedConductorPolicy|select_with_confidence|total_observations'` on `crates/` returns **zero matches** on the wrapper type. The `ConductorBandit` exists (F.21) but has no `total_observations()` or `select_with_confidence()` method; only `select_action(&state) -> ConductorAction` at `crates/roko-learn/src/conductor.rs:204-218` and `record_outcome` at `:221-247`. There is no typed fallback-to-static-policy wrapper, and the bandit is wired to the retry-path directly (not as an `InterventionPolicy` substitute for `WorstSeverityPolicy` inside the conductor).
**Fix sketch**: Add the `LearnedConductorPolicy` wrapper in `roko-conductor` when there's appetite to replace `WorstSeverityPolicy`. Add `ConductorBandit::total_observations()` (sum of `arm.observations`) + `select_with_confidence(&state) -> (ConductorAction, f64)` in `roko-learn`. Warmup + confidence threshold are both cheap additions.

---

## F.23 — Reward shaping table (Continue=0.9, Restart-succeed=0.8, Fail-correct=0.7, Fail-premature=0.1) (Doc 15 §Reward shaping)

**Status**: PARTIAL (different numbers than the doc, but similar spirit)
**Severity**: LOW
**Doc claim**: Doc 15 lines 116-124 specify a reward table: `Continue/pass=0.9`, `Continue/fail=0.1`, `Restart/succeed=0.8`, `Restart/fail=0.2`, `Fail/correct=0.7`, `Fail/premature=0.1`. Narrative at `:125-148` explains why: reserve 1.0 headroom, reward attempted recovery, lightly penalize premature fail.
**Reality**: The actual shape in `ConductorBandit::reward_for_outcome` at `crates/roko-learn/src/conductor.rs:305-337` uses different numbers and a different structure. Success branch at `:311-318`: `Continue=1.0, InjectHint=0.92, SwitchModel=0.88, Restart=0.82, Abort=0.0`. Failure branch at `:320-336`: uses a `futility_score` heuristic (`:432-462`) that blends consecutive_failures, iteration, time, cost, and error-pattern pressure, then scales: `Continue = 0.15 * (1.0 - futility)`, `Restart = 0.10 + 0.60 * futility * restart_bias(error_pattern)`, `Abort = 0.05 + 0.75 * futility * abort_bias(complexity)`. The directional claim — attempted recovery > passive continue, fail-fast has value when futility is high — holds qualitatively, but the concrete numbers in the doc are off by a few percentage points everywhere and the shape is quite different (multi-factor futility vs. fixed table).
**Fix sketch**: Either update doc 15 lines 116-148 to show the actual reward function (futility-weighted shape, with `Continue=1.0 on success` rather than `0.9`), or document the discrepancy. The doc's rationale (reserve headroom, reward recovery) is defensible but the specific numbers are out of sync.

---

## F.24 — Four-level conductor federation (L1 Turn / L2 Task / L3 Plan / L4 Fleet) (Doc 15 §Conductor federation)

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Doc 15 lines 174-267 specify a four-level conductor hierarchy (L1 Turn via `AnomalyDetector`, L2 Task via existing conductor, L3 Plan, L4 Fleet) with a shared `ConductorLevel` trait and `ConductorScope { Turn, Task, Plan, Fleet }`. Communication is via signal-stream tags (`conductor.anomaly.*`, `conductor.intervention.*`, `conductor.plan.*`, `conductor.fleet.*`).
**Reality**: Only L1 (anomaly) and L2 (per-task conductor) ship. `Grep 'ConductorLevel|ConductorScope|conductor.plan.|conductor.fleet.'` on `crates/` returns **zero matches**. Signal tags that do exist are `conductor.agent_output` (time-overrun watcher) and `conductor.intervention` (all 10 watchers, see F.18). There is no `ConductorLevel` trait, no plan-level L3 conductor, no fleet-level L4 conductor, and no typed `ConductorScope` enum. The cascade router performs some L4-flavored behavior (cross-plan observations) but is not packaged as a conductor at that level.
**Fix sketch**: Doc 15 banner already says "Scaffold", and this federation section is explicitly the frontier. Keep as designed-only. If/when a plan-level conductor becomes necessary, adding `ConductorLevel` trait + `ConductorScope` enum is straightforward given the existing signal-stream plumbing.

---

## F.25 — Self-healing: SelfHealingConductor + SelfRepairAction (Doc 15 §Self-healing conductor)

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Doc 15 lines 422-462 specify `SelfHealingConductor { inner, accuracy, threshold_learner, min_accuracy, self_check_interval }` + `SelfRepairAction { RecalibrateThresholds, ExpandStuckHeuristics, AddNewWatcher, ResetCircuitBreakers, RetrainBandit }`, with a `self_assess` method running on a 5-minute timer.
**Reality**: `Grep 'SelfHealingConductor|SelfRepairAction|RecoveryOriented|MicroReboot|self_assess'` on `crates/` returns **zero matches**. The doc-banner says "Scaffold" so this is expected — no drift, but also no code. The four failure modes in the doc 15 table (lines 359-362: threshold drift, model staleness, watcher blindness, circuit-breaker stuck) have no corresponding detectors. Persistence for bandit state does ship (see F.21 — `ConductorBandit::save/load_or_new`), which the doc's "Survivor functions" claim (line 416-420) depends on; so that piece is in place.
**Fix sketch**: Frontier. Good next step once (a) `SelfModelAccuracy` metrics ship (F.11) and (b) threshold adaptation moves from gate-rungs to conductor watchers (F.12).

---

## F.26 — Triple-loop learning (single/double/triple) framing (Doc 15 §Triple-loop learning)

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Doc 15 lines 491-545 map Argyris & Schon's single/double/triple-loop learning onto the conductor: single = restart-agent-on-failure, double = ThresholdLearner adjusts thresholds, triple = adjust the learning rate/discount/min-sample-size on the ThresholdLearner itself.
**Reality**: Single-loop ships (F.15-F.18 — conductor interventions and retry-bandit). Double-loop requires `ThresholdLearner` (F.12), which does not ship. Triple-loop requires meta-parameters over `ThresholdLearner`, which cannot exist before the double loop does. The doc framing is aspirational narrative; no anti-pattern in the code, just absent. `Grep 'single_loop|double_loop|triple_loop|learning_rate_meta'` returns **zero matches**.
**Fix sketch**: Sequence: (a) complete F.12 (Bayesian watcher-threshold adaptation), (b) instrument whether adaptations converge or oscillate, (c) only add meta-parameter tuning if oscillation shows up in practice. Premature triple-loop would be over-engineering.

---

## F.27 — Pattern-library / learning integration: auto-fix loop (Doc 11 §Loop 3 Error Classification)

**Status**: PARTIAL
**Severity**: LOW
**Doc claim**: Doc 11 lines 260-269 describe "Loop 3: Error Classification → Auto-Fix → Pattern Library": `DiagnosisEngine` classifies errors, auto-fix is attempted, and "If auto-fix succeeds → Pattern stored in diagnosis engine with higher confidence → Future similar errors auto-fixed faster."
**Reality**: Classification ships (`DiagnosisEngine::diagnose(&self, err: &str) -> Vec<DiagnosisResult>` — implied from the doc-test at `crates/roko-conductor/src/diagnosis.rs:10-16`, with the 20-variant `ErrorCategory` at `:26-67`). Pattern storage and confidence-updating do **not** ship: `Grep 'pattern_library|pattern_store'` on `crates/` returns only `pattern_discovery` (a separate miner, see below). The `DiagnosisEngine` loads a static pattern set at `:284+` but has no learned-confidence persistence. `PatternMiner` at `crates/roko-learn/src/pattern_discovery.rs:99+` mines trigrams from episodes but feeds a different pipeline (task-level pattern discovery, not diagnosis-engine confidence updates).
**Fix sketch**: LOW severity. Add a persistence surface to `DiagnosisEngine` that tracks `pattern_id → (hits, auto_fix_successes)` and multiplies stored confidence by the observed ratio. Or wire `PatternMiner`-discovered patterns back into the diagnosis engine.

---

## F.28 — Loop 4: Quality Degradation → Model Escalation (Doc 11 §Loop 4)

**Status**: PARTIAL
**Severity**: LOW
**Doc claim**: Doc 11 lines 273-282 describe "Loop 4: Quality Degradation → Model Escalation → Quality Data": anomaly detector fires `QualityDegradation`, escalates to higher-tier model, quality data feeds router, eventually router learns tier requirements.
**Reality**: `Anomaly::QualityDegradation` ships (`check_quality` at `crates/roko-learn/src/anomaly.rs:95-118`, tested at `:270-289`). Model escalation on cost-spike is logged at `crates/roko-cli/src/orchestrate.rs:615-616` (`tracing::warn!(... "learning anomaly detected from cost")`), and the `ConductorBandit::SwitchModel` action is wired (F.21). But the specific pipeline "QualityDegradation triggers escalation → router learns from the escalation outcome" is not one single wired loop — it's constructed by combining the quality detector (in anomaly.rs) with the retry-path bandit (in orchestrate.rs), which does observe `SwitchModel` outcomes via `record_outcome`. Doc 11's narrative is a conceptual summary, and the pieces exist, but there is no direct `QualityDegradation → SwitchModel` shortcut.
**Fix sketch**: Document the current split (quality detector is one subsystem; cascade router is another) or add an explicit handler on `Anomaly::QualityDegradation` that nudges the retry bandit. LOW severity because the anomaly is logged and the retry path can still pick up SwitchModel on its own.

---

## Section Summary

| Status | Count |
|--------|-------|
| DONE | 11 (F.01 OODA evaluate() body, F.02 6 signal kinds, F.03 WorstSeverityPolicy + 3-decision enum, F.04 10 watchers + thresholds, F.09 Good Regulator components, F.10 static constants, F.15 AnomalyDetector, F.16 pre-turn wiring, F.17 AgentEfficiencyEvent, F.18 cascade-router feedback, F.19 adaptive gate thresholds, F.20 ProviderHealthTracker; F.21 ConductorBandit-is-wired-counter-to-doc) |
| PARTIAL | 5 (F.07 nested OODA — implicit across subsystems, no typed cascade; F.08 algedonic — triggers exist, no unified channel; F.23 reward table — different numbers but same spirit; F.27 pattern library — static only; F.28 quality-degradation-to-escalation loop — pieces exist, no direct hop) |
| NOT DONE | 11 (F.05 LivenessMonitor, F.06 ImplicitGuidance, F.11 SelfModelAccuracy/BrierScoreTracker, F.12 ThresholdLearner/Posterior, F.13 ScalarKalman, F.14 PrecisionWeightedUpdater/ForwardPredictor, F.22 LearnedConductorPolicy wrapper, F.24 ConductorLevel/ConductorScope federation, F.25 SelfHealingConductor/SelfRepairAction, F.26 triple-loop learning meta-params) |
| SCAFFOLD | 0 |

Total: 28 items (F.01–F.28). HIGH-severity items: none. MEDIUM-severity items: one — **F.21** (doc 15 banner "Scaffold" + file reference "ConductorBandit (built, not wired)" is drifted; the bandit is wired into the orchestrator's retry path at `crates/roko-cli/src/orchestrate.rs:6039-6298`, with persistence, 7 actions, 19-dim state encoding, and full test coverage).

**Overall posture.** Docs 07, 08, 11 are accurate "Built" reports on the current loop. Their theoretical framing (OODA, Conant-Ashby, Wiener feedback, negative-feedback stability, observability) is a narrative layer over plumbing that really ships — watchers, circuit breaker, diagnosis engine, health monitor, anomaly detector, efficiency events, adaptive gate thresholds, provider health tracker, and cascade-router feedback are all present and match their doc-level descriptions. The "future work" sections in each doc (LivenessMonitor, ImplicitGuidance, SelfModelAccuracy, BrierScoreTracker, ThresholdLearner, ScalarKalman, ForwardPredictor) are self-labeled as proposals and the code correctly does not ship them — no drift, just frontier.

Doc 15 is mostly labeled "Scaffold" and its frontier pieces (ConductorLevel federation, SelfHealingConductor, triple-loop learning) are all grep-negative as expected. The one point of material drift is the claim that `ConductorBandit` is "built, not wired" — it IS wired into the per-task retry path with persistence, 7 actions, 19-dim state encoding, and `record_outcome` + `select_action` on every retry cycle. This is tracked as F.21 (MEDIUM). The doc's `LearnedConductorPolicy` wrapper that would replace `WorstSeverityPolicy` inside the conductor itself is genuinely absent (F.22), so the doc is correct about the **intervention-policy replacement** gap even if it is wrong about the bandit's **wiring status** more broadly.

Recommendations: (a) update doc 15 line 8 banner and line 553 file-reference to reflect the ConductorBandit's actual wiring in the retry path (F.21); (b) either update doc 15's reward-shaping table (F.23) to match the `futility_score`-based implementation in `crates/roko-learn/src/conductor.rs:305-462` or note the discrepancy; (c) leave docs 07 and 08 as-is — their "Built" banners refer to the current loop, not to the future-proposal subsections, and the `>` block-quotes plus future-tense narrative make the distinction clear to a careful reader; (d) continue to treat doc 15's federation/self-healing/triple-loop sections as designated frontier.

## Agent Execution Notes

### F.21 / F.23 — Keep The Learning Story Honest

The useful work here is mostly status correction:

1. `ConductorBandit` is real on the retry path,
2. the learned-policy wrapper is still absent,
3. reward-shaping descriptions should match the current implementation or be labeled as conceptual.

### F.05-F.14 / F.22 / F.24-F.26 — Frontier Demotion, Not Frontier Build

Do not let these sections pull batch `07` into self-model, federation, or triple-loop implementation work unless a later dedicated pass explicitly owns that.

Acceptance criteria for this section:

- later agents can tell which conductor-learning pieces are operational,
- theoretical sections are clearly marked as frontier,
- docs stop understating or overstating retry-path learning.
