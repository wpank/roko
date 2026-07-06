# Conductor — supervision, watchers, circuit breaker, diagnosis

> Status-quo audit · verified 2026-07-08 · **deep second pass** re-verified against git HEAD 5852c93c05 · sources: all 24 files of `crates/roko-conductor/` (10,101 LOC src, read exhaustively), `crates/roko-cli/src/orchestrate.rs` (WatcherRunner :2257-2350, run_conductor_check :6320-6387, ensure_dispatch_allowed :6308-6316) + `runner/event_loop.rs` (select loop :1743-1865, conductor_load :4258), `roko-core` config/dashboard types, `docs/v1/07-conductor/` (16 files), `docs/v2-depth/07-agent-runtime/14-17`, git history, sibling audits 36/41/53
>
> **Second-pass corrections to pass-1 counts** (verified by grep against `diagnosis.rs`): the engine ships **37** patterns, not 39 (`awk '/fn built_in_patterns/,/^}/' | grep -c 'name:'` → 37); **20** `ErrorCategory` variants, not 21; **9** `SuggestedIntervention` variants (unchanged). The only `roko_conductor` importer in `crates/**/*.rs` outside the crate itself is `roko-cli/src/orchestrate.rs` (grep confirmed: property_tests.rs + 4 intra-crate files are the rest) — runner-v2 (`event_loop.rs`) imports **zero** conductor symbols.

Status vocab: ✅ wired | 🔌 built-not-wired | 🟡 partial | ❌ missing | 🕰️ legacy-v1-shape (wired only behind non-default `legacy-orchestrate`)

## Summary

roko-conductor is one of the best-built, best-tested crates in the workspace — 10,101 LOC, ~302 in-module tests plus 14 property tests (`tests/property_tests.rs`) — and **effectively none of it runs in the shipping binary**. Its only runtime consumer is `orchestrate.rs` (imports at `crates/roko-cli/src/orchestrate.rs:46-51`), which is compiled only under the non-default `legacy-orchestrate` feature (`crates/roko-cli/src/lib.rs:94-95`). Two other crates declare the dependency but never import it: `roko-serve/Cargo.toml:37` and `roko-orchestrator/Cargo.toml:17` are **dead deps** (grep `roko_conductor` in both srcs: zero hits; only `AgentRole::Conductor` from roko-core appears).

Inside the gated legacy path the integration is genuinely deep: a background `WatcherRunner` tails `.roko/engrams.jsonl` every 30s and can cancel the orchestrator on critical alerts (orchestrate.rs:2257-2350, interval const :226); `run_conductor_check` runs after gates and merges and **enforces** decisions (Restart → `ExecutorEvent::Start`, Fail → `ExecutorEvent::Fatal`, orchestrate.rs:9092-9113, 9177-9198, 9857); the circuit breaker refuses dispatch (`ensure_dispatch_allowed` :6308-6316), pauses tripped plans, and runs the diagnosis engine on the failure reasons (:6255-6305); breaker state persists across resume (`PersistedCircuitBreakerState`, :790-825, snapshots at :7208, 7243, 7290); routing bias flows into cascade routing (:15425-15605, `cascade_routing_bias_from_conductor` :2478); health monitor + stuck detection run on a daimon-modulated theta/delta heartbeat (:6996-7075); and compound patterns trigger INT-19 coordination dreams (:8316-8359).

**Runner v2 — the engine every live surface actually uses (per `tmp/status-quo/36-ORCHESTRATION-RUNNERS.md`) — imports zero `roko_conductor` symbols** (`grep -rln roko_conductor crates/**/*.rs` → **only** `orchestrate.rs`, the gated legacy module). Its supervision is static: `TimeoutConfig`-derived wall clocks (agent dispatch / plan total / LLM call / gate rung / HTTP / health-check, `runner/event_loop.rs:132-191`), a plan-timeout select branch (:1835-1868), per-turn and per-plan budget checks (:825-838, 4033-4041), retry budgets with failure-kind classification (:1124, 2064-2067), and process-group `kill_tree`. Its lone recovery hook — `RecoveryEngine` (event_loop.rs:29, used at :3681-3697 for snapshot recovery at resume) — is **`roko_orchestrator`'s, not the conductor's** (`use roko_orchestrator::{… RecoveryEngine …}`, event_loop.rs:26-31); it is unrelated to the conductor's `RecoveryEngine`-less anomaly loop. So runner v2 has no watchers, no per-plan failure breaker, no stuck detection, no diagnosis. CLAUDE.md's row "roko-conductor … 10 watchers … Used by executor internals" is **stale**: the executor internals that used it are no longer compiled by default.

The v2 redesign docs (`docs/v2-depth/07-agent-runtime/14-17`) rethink every conductor component as kernel Cells (watchers as Verify Cells, breaker as a state-machine Cell, diagnosis as a Route Cell, stuck detection and self-model accuracy as Lenses, OODA as a Loop Graph). **Zero Cell-shaped conductor code exists** — `roko-graph` registers no conductor/watcher cells, and no `Lens` type exists anywhere in the workspace (grep `struct|trait|enum .*Lens` → 0 hits).

## Current state table

| Component | Design source | Code | Status | Evidence |
|---|---|---|---|---|
| Watcher ensemble (10 watchers) | v1 `07-conductor/01-watcher-ensemble.md`; v2-depth 14 §2 "watchers as Verify Cells" | `crates/roko-conductor/src/watchers/` (10 files) | 🕰️ built + tested; runs only in gated `WatcherRunner`/`run_conductor_check` | default set of 10 at `conductor.rs:95-108`; loop sites orchestrate.rs:2300, 6343 |
| Conductor composite (React impl, `evaluate_full`) | v1 `00-conductor-architecture.md` | `conductor.rs:60-453` | 🕰️ | constructed `Conductor::new()` orchestrate.rs:4637, restored from breaker state :4745-4750, 4972-4977 |
| Circuit breaker (count-based, per-plan) | v1 `02-circuit-breaker.md`; v2-depth 15 §2 | `circuit_breaker.rs:147-180+` (`MAX_PLAN_FAILURES = 2` :19) | 🕰️ enforced pre-dispatch + pause + persisted | orchestrate.rs:6255-6316 (pause+diagnose), :7208/7243/7290 (snapshot), conductor.rs:315-326 (trip → Shutdown signal) |
| Predictive breaking (Holt, COND-08) | v2-depth 15 §3 "predict-publish-correct" | `HoltForecaster` `circuit_breaker.rs:44-104`, `check_proactive` :276 | 🕰️ inside `evaluate_full` (conductor.rs:370-396); `predictive: false` by default (circuit_breaker.rs:178) | warning/proactive-trip → Cooldown/Shutdown signals |
| Graduated interventions (Continue/Restart/Fail) | v1 `03-graduated-interventions.md` | `interventions.rs:31-128` (`Severity`, `WorstSeverityPolicy`) | 🕰️ enforced: Restart→`ExecutorEvent::Start`, Fail→`ExecutorEvent::Fatal` | orchestrate.rs:9092-9113, 9177-9198 |
| Bandit intervention policy (Thompson blend 65/35) | v1 `11-anomaly-detection-learning.md` | `BanditPolicy` `interventions.rs:150-200+` | 🔌 never installed — Conductor always uses `WorstSeverityPolicy` (`conductor.rs:222`); no `with_policy` caller | grep `BanditPolicy` outside crate → 0 |
| Retry bandit (separate, roko-learn) | — | `roko_learn::conductor::ConductorBandit` | 🕰️ used directly for retries, not via policy | orchestrate.rs:107-109, 2708, 4645; persists `.roko/learn/conductor.json` (:650-651) |
| Diagnosis engine (**37** patterns, **20** categories, 9 interventions) | v1 `04-diagnosis-engine.md`; v2-depth 16 §3-4 (claims "34 patterns", Route Cell) | `diagnosis.rs` (20 `ErrorCategory` :26-67, 9 `SuggestedIntervention` :75-94; 37 `name:` entries in `built_in_patterns` :294-568) | 🕰️ invoked on breaker trip (orchestrate.rs:6275-6293) and failure chains (:10933) | results → `publish_conductor_diagnosis` :6389, event log `InterventionFired` :6295 |
| Diagnosis dashboard surface | — | `roko-core/src/dashboard_snapshot.rs:342` `DiagnosisSummary`, ring of 50 :768-770, projection :2174-2293 | 🟡 type + projection live in roko-core/serve; **no producer** in default binary | only producer is gated orchestrate.rs:6389/22049-22073 |
| Stuck detection (12 checks + cooldown + metacognition) | v1 `05-stuck-detection.md`; v2-depth 16 §6 "as a Lens" | `stuck_detection.rs` (checks :479-888; `MetaCognitionHook` :907+; `CooldownFilter`) | 🕰️ theta-heartbeat driven per plan tracker | orchestrate.rs:6920-6994 (`run_stuck_detection`), gated by `frequency == Theta` :7041-7043 |
| Health monitors (5 checks) | v1 `06-health-monitors.md` | `health.rs:122-360+` (`check_terminal_liveness/agent_status/chain_status/spec_drift/coverage_trend` :237-344) | 🕰️ every heartbeat; alerts → stderr + pheromones (INT-13) + `conductor:alert:health_monitor` signals | orchestrate.rs:6841-6918, snapshot built :6841-6857 |
| OODA / adaptive supervision loop | v1 `07-ooda-cybernetic-loop.md`, `10-adaptive-timeouts-state-machine.md`; v2-depth 17 | `HeartbeatClock` `crates/roko-cli/src/heartbeat.rs:150+`, cadence from daimon affect via `OperatingFrequencyScheduleContext::from_affect` (roko-core) | 🕰️ only caller is `maybe_run_heartbeat` orchestrate.rs:6996-7075; persists snapshots under `.roko/learn/` (heartbeat.rs:1-6) | theta→stuck detection, delta→dreams :7041-7074 |
| Good-regulator self-model | v1 `08-good-regulator-self-model.md`; v2-depth 17 §2 (self-model accuracy Lens, Brier) | — | ❌ no conductor/runner code (Brier scoring exists but only in `roko-learn` `heuristics.rs:66-76`/`prediction.rs:293+` for heuristic calibration — not a conductor self-model) | grep `Brier|self_model` in roko-conductor → 0; nearest is roko-learn's unrelated Brier |
| Cognitive signals | v1 `09-cognitive-signals.md` | `roko_core::CognitiveSignal`, derived in `conductor.rs:482-544` | 🕰️ emitted by `evaluate_full`; consumed?— decision `.with_signals()` returned to gated caller only | orchestrate.rs:6343 uses `evaluate` (drops signals; `evaluate_full` only in WatcherRunner path via `check_all`→`decide`) |
| Adaptive timeouts / phase state machine | v1 `10-adaptive-timeouts-state-machine.md` | `state_machine.rs` (`phase_timeout`, `PhaseTransition`) | 🔌 zero external callers; roko-runtime/orchestrator define their own unrelated `PhaseTransition` types | grep `phase_timeout` outside crate → 0 |
| Threshold learner (COND-03) | v1 `11-anomaly-detection-learning.md`; v2-depth 17 §3 (Bayesian Beta) | `threshold_learner.rs` | 🔌 instantiated inside Conductor (`conductor.rs:72,226`) but **never fed**: `record_intervention_outcome`/`with_learner` have no callers | grep → 0 outside crate |
| Yerkes-Dodson pressure (COND-04) | v1 `12-yerkes-dodson-pressure.md`; v2-depth 17 §4 (Score Cell) | `yerkes_dodson.rs` (10 tests + property tests) | 🔌 dormant — zero callers anywhere | grep `YerkesDodson` outside crate → property_tests.rs only |
| Federation L1→L4 (COND-05) | v1 `15-conductor-learning-federation.md` | `federation.rs` | 🔌 dormant | grep → 0 outside crate |
| Self-healing (COND-06) | v1 `14-production-failure-catalog.md` adjacent | `self_healing.rs` (`HealingAction` :54) | 🔌 dormant | grep → 0 outside crate |
| Pattern detector (COND-07, compound patterns) | v2-depth 14 §3 "pattern detector as Verify Cell" | `pattern_detector.rs` (`WatcherFamily` :20, `CompoundPattern`) | 🕰️ live inside `evaluate_full` (conductor.rs:336-368): resource_exhaustion / quality_degradation / progress_stall etc. escalate signals + override decision :414-423 | |
| INT-19 coordination-pattern dream trigger | dream audit `41-DREAMS.md:29` | `take_compound_patterns` conductor.rs:447-452; orchestrate.rs:2696-2698, 6345-6350, 7068-7073, 8316-8359 | 🕰️ + 🟡 — fires on Delta heartbeat, but `DreamTrigger::CoordinationPattern` is built then **discarded** (`let _trigger =` orchestrate.rs:8351) and it delegates to plain `maybe_auto_dream` | matches 41-DREAMS.md:61 |
| Provider-health escalation (COND-09) | — | `with_provider_health` conductor.rs:258-263, check :398-409 | 🔌 `with_provider_health` never called → branch dead | grep → 0 callers |
| Routing bias → model routing | v1 `00-conductor-architecture.md` §routing | `RoutingBias` conductor.rs:34-42, derive :618-661 | 🕰️ consumed by cascade routing (orchestrate.rs:15425-15605) | prefer_cheaper + deprioritize honored |
| `ConductorConfig` (thresholds, TOML) | — | `roko-core/src/config/schema.rs:1257-1280` (`watchers`, `context_pressure_enabled`, hot-reload diff :818-819, env overrides :514-538) | 🔌 **`Conductor::from_config` never called** — orchestrate uses `Conductor::new()` (:4637), so `[conductor.watchers.*]` thresholds are dead config | `configured_watchers` conductor.rs:110-192 unreachable |
| Conductor-as-verify-pipeline (Cells) | v2-depth 14 (Pipeline Graph, FanOut, Route Cell selector, OODA Loop Graph) | — | ❌ no ConductorCell / watcher cells in `roko-graph`; no `Lens` types workspace-wide | grep `Lens` structs → 0; roko-graph cells = task_executor + 3 shell gates (36-ORCHESTRATION:25) |
| Runner v2 supervision (what exists instead) | `docs/v2/27-ORCHESTRATOR.md` | timeouts `event_loop.rs:132-191`, plan-timeout branch :1835-1868, budgets :825-838/4033-4041, retry classification :1124/2064-2067, `RecoveryEngine` resume-only :3681 | 🟡 static-only; 27-ORCHESTRATOR Gap 7 (conductor wiring) open | `conductor_load: 0.0` hardcoded event_loop.rs:4258 |
| Tests | — | ~302 `#[test]` across src (stuck 50, diagnosis 33, breaker 26, health 20, conductor 18, interventions 16, yerkes 10, …) + `tests/property_tests.rs` (14: breaker + yerkes) | ✅ crate-level; ❌ zero integration tests exercising conductor from any runner | counts via grep `#[test]` per file |

## Watcher census (deep second pass)

All 10 default watchers are pure `React` impls (`decide(&[Engram], &Context) -> Vec<Engram>`, watchers/mod.rs) emitting `Kind::Custom("conductor.intervention")` signals tagged `watcher=<name>` + `severity=<info|warning|critical>`. The `Conductor` runs them all each tick (`collect_watcher_outputs` conductor.rs:547-570), reads each output's `severity` tag back into a `Severity`, and `WorstSeverityPolicy` picks the max (interventions.rs:118-128). Default thresholds are compile-time consts; `from_config` overrides exist but are unreachable (see `ConductorConfig` row). The construction order in `default_watchers()` is fixed (conductor.rs:95-108).

| # | Watcher (`name()`) | Anomaly detected | Input signal(s) it reads | Fire condition (threshold) | Severity → decision | Emit file:line |
|---|---|---|---|---|---|---|
| 1 | `ghost-turn` | Agent burns tokens with zero net progress | `Kind::Custom("conductor.ghost_turn")`; JSON `GhostTurnEvent{output_meaningful,net_new_changes,cost_usd,…}` (ghost_turn.rs:19-32,61-81) | ≥ `MAX_GHOST_TURNS=3` **consecutive** events (counted from stream tail) where `!output_meaningful && net_new_changes==0` (ghost_turn.rs:11,84-99) | warning → **Restart** | ghost_turn.rs:120-135 |
| 2 | `review-loop` | Same plan rejected in review repeatedly, no phase advance | `Kind::PlanPhase` bodies, field `event` (review_loop.rs:62-76) | ≥ `MAX_REVIEW_CYCLES=3` `event="ReviewRejected"` for the latest plan_id; reset by `ReviewApproved`/`DocRevisionDone`/`MergeSucceeded` (review_loop.rs:10,92-114) | warning → **Restart** | review_loop.rs:96-107 |
| 3 | `iteration-loop` | Gate-fail retry cycle with no forward progress | `Kind::PlanPhase` bodies, field `event="GateFailed"` (iteration_loop.rs:63-77,96-99) | ≥ `MAX_IMPLEMENTER_ATTEMPTS=3` `GateFailed` for latest plan_id; reset by `GatePassed`/`ImplementationDone`/`ReviewApproved`/`VerifyPassed`/… (iteration_loop.rs:9,113-119) | **critical → Fail** | iteration_loop.rs:100-110 |
| 4 | `test-failure-budget` | Regression: test failures rose above the run's earliest baseline | `Kind::GateVerdict` bodies, JSON `{plan_id, test_count:{failed}}` (test_failure_budget.rs:61-86) | latest `failed − baseline_failed ≥ MIN_FAILURE_INCREASE=1` per plan (baseline = first seen) (test_failure_budget.rs:13,84-98) | warning → **Restart** | test_failure_budget.rs:99-111 |
| 5 | `compile-fail-repeat` | Agent stuck on the identical compile error | `Kind::CompileDiagnostic` bodies (`message` field or text) (compile_fail_repeat.rs:44-70) | same normalized diagnostic key ≥ `MAX_IDENTICAL_COMPILE_FAILURES=3` **consecutive** from tail (compile_fail_repeat.rs:9) | warning → **Restart** | compile_fail_repeat.rs:89 |
| 6 | `context-window-pressure` | Prompt nearing the model context window | `Kind::TokenUsage` (tags `tokens_used`/`tokens_total`/`model`; also `AgentEfficiencyEvent`) (context_window_pressure.rs:23-38,108) | max utilization over last `PRESSURE_LOOKBACK=3` `TokenUsage` signals > `MAX_CONTEXT_USAGE_RATIO=0.80` (context_window_pressure.rs:28,45) | warning → **Restart** | context_window_pressure.rs:141 |
| 7 | `spec-drift` | Task edits files outside its declared scope | `Kind::Metric` with tag `name=spec_drift`, JSON `SpecDriftEvent{write_files,changed_files,drift_ratio}` (spec_drift.rs:16-38,107) | `drift_ratio > MAX_SPEC_DRIFT_RATIO=0.25` (spec_drift.rs:12) | warning → **Restart** | spec_drift.rs:139 |
| 8 | `cost-overrun` | Accumulated plan cost past budget | `Kind::Metric` tags `name=plan_cost` / `name=plan_budget`, `value=<f64>` (cost_overrun.rs:11-19,54-60) | latest `plan_cost ≥ plan_budget` (fallback `DEFAULT_BUDGET=$10`) (cost_overrun.rs:21-24) | warning/critical → Restart/**Fail** | cost_overrun.rs:76 |
| 9 | `time-overrun` | Task approaching its wall-clock timeout | `Kind::Custom("conductor.agent_output")`, JSON `TaskTimingEvent{duration_ms,timeout_secs}` (time_overrun.rs:13,33-38) | `duration_ms > ALERT_THRESHOLD=0.80 × timeout_secs` (time_overrun.rs:16) | warning → **Restart** | time_overrun.rs:103 |
| 10 | `stuck-pattern` | Agent repeating the exact same action | `Kind::AgentOutput` / `Kind::AgentMessage` body fingerprints (stuck_pattern.rs:16,50-60) | `MAX_IDENTICAL_ACTIONS=4` consecutive identical fingerprints (stuck_pattern.rs:10) | warning → **Restart** | stuck_pattern.rs:104 |

**Signal-source reality**: watchers 1, 9 read bespoke `conductor.*` custom kinds that **only** the gated orchestrate.rs emits; watcher 6's `TokenUsage`, watcher 5's `CompileDiagnostic`, watchers 7/8's tagged `Metric`s, and watchers 2/3's `PlanPhase{event}` bodies are likewise all produced only inside orchestrate.rs. Runner-v2 emits `RuntimeEvent`/`RunnerEvent` (event_loop.rs:2796-2963) — a **different vocabulary** — so even if a `Conductor` were instantiated in runner-v2, every watcher would see an empty stream and never fire without an adapter (see Wiring gap + Adapter design below).

**Family map** (pattern_detector.rs:60-77) groups them for compound-pattern escalation: Resource = {cost-overrun, time-overrun, context-window-pressure}; Quality = {compile-fail-repeat, test-failure-budget, spec-drift}; Progress = {ghost-turn, iteration-loop, stuck-pattern, review-loop}.

Separate from the ensemble: `StuckDetector` has its own 12 checks over `ActivityEntry` history (output_loop, no_progress, gate_loop, compile_loop, empty_output, excessive_retries, review_loop, iteration_loop, silence_timeout, compile_fail_threshold, task_stall, context_pressure — stuck_detection.rs:479-888), duplicating several watcher concerns at task granularity.

## Circuit breaker internals (circuit_breaker.rs)

Per-plan failure budget on a lock-free `DashMap<plan_id, FailureRecord{count,last_failure_ms,reasons}>` (circuit_breaker.rs:22-30,147-161). Two tripping mechanisms, count-based always active + Holt predictive opt-in:

- **Count-based (default)**: `record_failure` increments `count`; trips when `count ≥ max_failures`, `MAX_PLAN_FAILURES=2` (circuit_breaker.rs:19,214-243). `is_tripped`/`is_broken` re-check the same threshold (:314-343). Once tripped it stays tripped — further failures don't un-trip (test :505-514).
- **Predictive (COND-08, off by default `predictive=false` :178)**: `with_predictive(threshold)` enables per-plan `HoltForecaster` (double exponential smoothing: `level(t)=α·obs+(1−α)(level+trend)`, `trend(t)=β·Δlevel+(1−β)·trend`, α=0.3 β=0.1 :49-52,81-91). `check_proactive` (:276-304) forecasts error rate: `forecast(1) ≥ forecast_trip_threshold(0.5)` → **ProactiveTrip** (Shutdown signal); `forecast(3) ≥ threshold` → **Warning** (Cooldown 1.5). Needs ≥2 observations for a trend. Successes feed `record_success`→`update(0.0)` improving the forecast (:249-253).
- **Evaluation ordering** (conductor.rs:311-436): `evaluate_full` (1) short-circuits with `Fail(circuit-breaker, MaxIterations)` + Shutdown signal if already tripped (:315-326); (2) runs watchers; (3) feeds outputs to `PatternDetector`; (4) checks proactive trip; (5) provider-health; (6) applies `WorstSeverityPolicy`, overriding Continue→Restart if a compound pattern hit Critical (:414-423); (7) records any `Fail` decision back into the breaker (:426-433).
- **Persistence**: `snapshot_state`/`from_state` round-trip `max_failures` + records (:200-208,373-382); orchestrate.rs snapshots at :7208/7243/7290 and boots via `from_circuit_breaker_state` (conductor.rs:254-256). Forecasters/eval-counts are **not** persisted (only `records`), so predictive trend resets on resume.
- **Live enforcement (legacy only)**: `ensure_dispatch_allowed` refuses dispatch pre-flight if `is_broken` (orchestrate.rs:6308-6316, called :15194); `run_conductor_check` short-circuits + pauses (:6320-6324). Neither exists in runner-v2.

## Diagnosis-pattern taxonomy (diagnosis.rs, 37 patterns)

`DiagnosisEngine::diagnose(&str)` is a pure substring matcher (no regex despite `ErrorPattern.needle` naming): each pattern carries `{name, needle, category, suggested_action, case_insensitive}` (diagnosis.rs:99-111). Matches are ranked by a confidence score (coverage ratio + specificity bonus + hyphenated-name bonus, :240-277), so specific codes (`error[E0308]`) outrank generic ones (`error[E`). **20 `ErrorCategory` → 9 `SuggestedIntervention`.** The 37 patterns by category and the intervention they map to:

| Family | Categories | Patterns (needle) → intervention |
|---|---|---|
| **Compile (11)** | CompileError, TypeMismatch, BorrowCheckerError, LifetimeError, ImportError | `rust-compile-error`(`error[E`)→RetryWithContext; `rust-type-mismatch`(`E0308`)→RetryWithContext; `rust-borrow-conflict`(`E0502`)/`rust-use-after-move`(`E0382`)/`rust-moved-value`(`E0505`)→**RestartAgent**; `rust-lifetime-missing`(`E0106`)/`rust-lifetime-mismatch`(`E0621`)→RestartAgent; `rust-unresolved-import`(`E0432`)/`rust-unresolved-path`(`E0433`)/`rust-missing-field`(`E0063`)→**AutoFix**; `rust-cannot-find`→RetryWithContext (diagnosis.rs:297-373) |
| **Test (2)** | TestFailure | `rust-test-failure`(`test result: FAILED`), `assertion-failed`(ci)→RetryWithContext (:375-388) |
| **Clippy (2)** | ClippyWarning | `clippy-warning`(`warning: `), `clippy-lint`(`clippy::`)→**WarnAndContinue** (:390-403) |
| **Git (2)** | GitConflict | `git-conflict-markers`(`<<<<<<<`), `git-merge-conflict`(`CONFLICT (content)`)→**MergeResolution** (:405-418) |
| **Deps (2)** | DependencyError | `cargo-dependency-missing`, `cargo-version-mismatch`→RetryWithContext (:420-433) |
| **Filesystem (2)** | MissingFile, PermissionDenied | `file-not-found`→RetryWithContext; `permission-denied`→**AbortPlan** (:435-448) |
| **Network (2)** | NetworkError | `connection-refused`, `dns-failure`(ci)→**BackoffRetry** (:450-463) |
| **Timeout (1)** | TimeoutError | `command-timeout`(`timed out`, ci)→BackoffRetry (:465-471) |
| **Resource (3)** | OomError, DiskFull | `out-of-memory`(ci), `oom-killed`(`SIGKILL`), `disk-full`(`No space left`)→**AbortPlan** (:473-493) |
| **LLM (6)** | LlmRateLimit, LlmContextOverflow, LlmRefusal | `llm-rate-limit`/`llm-429`→BackoffRetry; `llm-context-overflow`(`context_length_exceeded`)/`llm-max-tokens`→**ReduceContext**; `llm-content-filter`/`llm-safety-refusal`(`I cannot`)→**SwitchModel** (:495-536) |
| **Process (3)** | ProcessCrash | `segfault`, `process-abort`(`SIGABRT`), `process-panic`(`thread 'main' panicked`)→RestartAgent (:538-558) |
| **Loop (1)** | LoopDetected | `loop-detected-marker`(`LOOP DETECTED`, ci)→RestartAgent (:560-566) |

Intervention distribution: RetryWithContext ×7, AutoFix ×3, RestartAgent ×9, AbortPlan ×4, BackoffRetry ×5, MergeResolution ×2, ReduceContext ×2, SwitchModel ×2, WarnAndContinue ×2. Note `SuggestedIntervention` (9-way, diagnosis-time) is a **richer** vocabulary than the 3-way `ConductorDecision` (Continue/Restart/Fail) the policy actually enforces — the extra granularity (AutoFix, BackoffRetry, ReduceContext, SwitchModel, MergeResolution) is advisory metadata surfaced to the dashboard, never mechanically actioned by runner-v2.

## Wiring gap: orchestrate.rs (legacy) vs runner-v2 (live)

```
LEGACY orchestrate.rs (feature "legacy-orchestrate", NOT default)          RUNNER-V2 event_loop.rs (DEFAULT, every live surface)
────────────────────────────────────────────────────────                  ───────────────────────────────────────────────────
 agent turn / gate / merge                                                   agent turn / gate / merge
        │ emit_conductor_signal → self.conductor_signals (Vec<Engram>)              │ sink.emit(RuntimeEvent::{GateFailed,AgentFailed,…})
        │ + append .roko/engrams.jsonl                                              │ + append events.jsonl / run-ledger.jsonl
        ▼                                                                           ▼
 ┌──────────────────────────────┐   30s ┌─────────────────────┐            ┌───────────────────────────────┐
 │ run_conductor_check(plan_id) │◀──────│ WatcherRunner (tail  │            │ tokio::select! branches:      │
 │  :6320  conductor.evaluate() │       │  engrams.jsonl,      │            │  1 agent_rx  2 gate_rx        │
 └──────────────┬───────────────┘       │  check_all :2300)    │            │  3 tick_interval(100ms) :1743 │  ← NO conductor
   Continue / Restart / Fail             │  critical→cancel()   │            │  4 flush_interval(2s)   :1825 │  ← NO conductor
        │                                └─────────────────────┘            │  5 plan_timeout         :1836 │  (static wall clock only)
        ▼ enforced:                                                         │  6 cancel               :1854 │
  Restart → ExecutorEvent::Start  (:9092-9113)                              └───────────────┬───────────────┘
  Fail    → ExecutorEvent::Fatal  (:9177-9198, :9857)                                       ▼
  breaker → ensure_dispatch_allowed refuses (:6308)                          RoutingContext{ conductor_load: 0.0 }  ← hardcoded :4258
                                                                             (no Conductor, no CircuitBreaker, no watchers,
                                                                              no diagnosis, no stuck detection, no health monitor)
```

What orchestrate.rs has that runner-v2 lacks: (a) a `conductor_signals: Vec<Engram>` accumulator fed by `emit_conductor_signal`; (b) two call sites for `Conductor::check_all`/`evaluate` — the 30s `WatcherRunner` background tail (:2300) and the synchronous per-phase `run_conductor_check` (:6343); (c) **enforcement** — Restart/Fail decisions become `ExecutorEvent::Start`/`Fatal` (:9092-9198) and a tripped breaker refuses dispatch (:6308-6316); (d) breaker-state snapshot/resume (:7208-7290). Runner-v2 has **none** — its only supervision is `TimeoutConfig` wall clocks + per-task retry budgets, and `conductor_load` is a literal `0.0`.

## Minimal adapter design: give runner-v2 anomaly supervision

The watchers are pure functions over `&[Engram]`; the only missing piece is (i) an `Engram` feed and (ii) a tick that runs `check_all` and enforces the decision. Minimal, three parts:

**1. `RuntimeEvent → Engram` adapter** (new `runner/conductor_adapter.rs`). Map the events runner-v2 already emits (event_loop.rs:2796-2963) into the exact `Kind`s the watchers read:

| RuntimeEvent (source) | → Engram the watcher needs |
|---|---|
| `GateFailed{plan_id,gate,rung}` | `Kind::PlanPhase` body `{plan_id,event:"GateFailed"}` (feeds **iteration-loop**, review-loop) |
| `GatePassed{…}` / `TaskCompleted{…}` | `Kind::PlanPhase` body `{plan_id,event:"GatePassed"}` (resets iteration-loop) |
| gate test verdict counts | `Kind::GateVerdict` body `{plan_id,test_count:{failed}}` (**test-failure-budget**) |
| compile gate stderr | `Kind::CompileDiagnostic` text (**compile-fail-repeat**) |
| agent turn w/ token usage | `Kind::TokenUsage` tags `tokens_used/tokens_total/model` (**context-window-pressure**) |
| agent turn cost + budget | `Kind::Metric` `name=plan_cost` / `plan_budget` (**cost-overrun**; already derivable from efficiency.jsonl, cf. `load_efficiency_cost_signals`) |
| agent turn timing | `Kind::Custom("conductor.agent_output")` `{duration_ms,timeout_secs}` (**time-overrun**) |
| agent stdout | `Kind::AgentOutput` / `AgentMessage` (**stuck-pattern**) |
| ghost-turn heuristic (net-0 change turn) | `Kind::Custom("conductor.ghost_turn")` `GhostTurnEvent` (**ghost-turn**) |

Maintain a bounded `VecDeque<Engram>` (tail ~256, mirroring `WATCHER_SIGNAL_TAIL`) on the `EventLoop` state, pushed to at each `sink.emit` site (or via a wrapping `EventSink` decorator so no emit site changes).

**2. A conductor tick.** Instantiate `Conductor::from_config(&config.conductor)` (finally exercising the dead `[conductor.watchers.*]` TOML) once at loop start, held in `state`. Add a `conductor_interval = interval(Duration::from_secs(30))` and a `select!` branch (slot beside tick/flush, event_loop.rs:1743-1833):

```rust
_ = conductor_interval.tick() => {
    let decision = conductor.evaluate_full(&state.conductor_ring.make_slice(), &Context::now());
    match decision.decision {
        ConductorDecision::Restart{watcher,reason} => { /* re-queue task, emit WatcherAlert */ }
        ConductorDecision::Fail{watcher,reason} => {
            emit_runner_event(WatcherAlert{..}); cancel.cancel();   // mirror WatcherRunner critical path
        }
        ConductorDecision::Continue => { /* feed decision.signals into RoutingContext.conductor_load */ }
    }
    // record breaker into snapshot; feed routing_bias() to next RoutingContext
}
```

**3. Enforcement + feedback.** (a) On `Fail`, cancel the plan the way the legacy WatcherRunner does (`cancel.cancel()`, orchestrate.rs:2315). (b) Replace `conductor_load: 0.0` (event_loop.rs:4258) with a real load derived from `conductor.routing_bias()` / recent warning density. (c) Persist `conductor.circuit_breaker().snapshot_state()` into the runner snapshot next to the executor state so resume restores the budget.

### Ordered wiring checklist (adapter-first)

- [ ] **[P0]** Add `conductor_ring: VecDeque<Engram>` to runner-v2 `EventLoop` state + push at emit sites (or wrap `EventSink`). Verify: ring populated after one gate failure in `roko plan run` (log the len).
- [ ] **[P0]** Write `runner/conductor_adapter.rs` mapping the 9 `RuntimeEvent` classes above to watcher `Kind`s. Verify: unit test — feed 3 synthetic `GateFailed` → `iteration-loop` fires critical.
- [ ] **[P0]** Instantiate `Conductor::from_config(&config.conductor)` + 30s `conductor_interval` select branch; enforce Restart/Fail (Fail→`cancel.cancel()`). Verify: forced 3× identical compile-fail in a live run aborts via conductor and `events.jsonl` gains a `WatcherAlert`.
- [ ] **[P1]** Replace hardcoded `conductor_load: 0.0` (event_loop.rs:4258) with `conductor.routing_bias()`-derived load. Verify: `RoutingContext.conductor_load > 0` under sustained warnings.
- [ ] **[P1]** Snapshot/restore `circuit_breaker().snapshot_state()` in the runner snapshot. Verify: plan tripped, resumed, stays refused.
- [ ] **[P1]** Now that `from_config` has a caller, either honor `context_pressure_enabled` (watcher 6) or delete the dead `[conductor.watchers.*]` schema (schema.rs:1257-1280). Verify: setting `watchers.ghost_turn.max_consecutive=1` changes behavior in a test.
- [ ] **[P2]** Feed `decision.signals` (Escalate/Cooldown/Explore) into runner-v2 budget/routing knobs instead of dropping them (legacy uses `evaluate` and drops signals too). Verify: cost-pressure warning extends the turn budget.
- [ ] **[P2]** Thread `take_compound_patterns()` → dream trigger (INT-19) properly instead of `let _trigger` (orchestrate.rs:8351). Verify: dream journal records `trigger: coordination_pattern`.

## V2-aligned

- The 3-level decision model (Continue/Restart/Fail) matches v2-depth 15's simplified intervention semantics, and severity→decision mapping is a pure function (`interventions.rs:43-51`).
- Watchers are already side-effect-free pure functions over signal slices — exactly the property v2-depth 14 §2 wants for Verify Cells; the port is mechanical.
- Holt predictive breaking (COND-08) implements v2-depth 15 §3's predict-publish-correct math (`circuit_breaker.rs:34-104`).
- Pattern detector + compound-pattern escalation (COND-07) matches v2-depth 14 §3; INT-19 plumbing to dreams exists (however lossy).
- Diagnosis engine's category→intervention mapping (39 patterns) is a superset of v2-depth 16's 34-pattern catalog and is pure (`diagnosis.rs:1-5`).
- `ConductorEvaluation` returning decision + `CognitiveSignal`s (conductor.rs:301-437) anticipates the v2 "signals as modulations" idea.

## Old paradigm & tech debt

- **Everything runtime-facing lives in a feature-gated 23.7K-line legacy module.** The conductor's entire wiring story (WatcherRunner, heartbeat, health, stuck, breaker persistence, routing bias, INT-19) exists only under `legacy-orchestrate` (`lib.rs:94-95`). In the default binary the conductor is a library with no process.
- **Dead Cargo deps**: `roko-serve/Cargo.toml:37` and `roko-orchestrator/Cargo.toml:17` pull roko-conductor for nothing.
- **Dead config**: `[conductor.watchers.*]` thresholds + `context_pressure_enabled` (schema.rs:1257-1280) are parsed, hot-reload-diffed (:818-819), env-overridable (:514-538) — and never consulted (`Conductor::from_config` has zero callers). The schema comment even instructs "Enable only after wiring a subscriber in orchestrate.rs" (schema.rs:1272-1274), pointing at the module that's compiled out.
- **Dormant modules** (~zero external callers): `yerkes_dodson.rs`, `federation.rs`, `self_healing.rs`, `state_machine.rs`, `threshold_learner` feeding, `BanditPolicy`, `with_provider_health`. COND-03/04/05/06/09 are shelf-ware.
- **CLAUDE.md drift**: "10 watchers, circuit breaker, diagnosis — Used by executor internals" implies live supervision; reality is 🕰️. Also the v1 doc set (`docs/v1/07-conductor/13-process-supervision-wiring.md`) describes wiring that matched orchestrate.rs, not runner v2.
- **Signal-kind coupling**: ghost-turn/time-overrun watch bespoke `conductor.*` custom kinds that only orchestrate.rs ever emitted; runner v2 emits `RunnerEvent`s/`RuntimeEvent`s instead — the watchers would be blind even if instantiated there without an adapter.
- **`evaluate` vs `evaluate_full` split-brain**: the periodic check uses `evaluate` (orchestrate.rs:6343) which discards cognitive signals; only the WatcherRunner's `check_all→decide` path persists intervention signals. Signals like `InjectContext`/`Escalate` are computed and mostly dropped.
- **INT-19 metadata loss**: `DreamTrigger::CoordinationPattern` constructed then discarded (orchestrate.rs:8351); dream journal records `trigger: Manual` (41-DREAMS.md:61).
- Git note: `ba0cd4005` "fix: surface suppressed errors in budget, circuit breaker, and channels" (2026-05-23) hardened breaker-trip surfacing in orchestrate.rs (+190 lines) + serve aggregator — i.e. recent investment went into the path that was subsequently feature-gated off.

## Not implemented

- Conductor as a Pipeline/Loop Graph of Cells (v2-depth 14 §3-4): no ConductorCell, no watcher Verify Cells, no FanOut evaluation, no Route-Cell intervention selector.
- OODA loop as kernel pattern (v2-depth 14 §4) — the closest artifact, `HeartbeatClock`, is legacy-gated and not graph-shaped.
- Self-model accuracy Lens + Brier calibration (v2-depth 17 §2) — no code at all.
- Feature-level breakers ("same Cell, different instances", v2-depth 15 §4) — breaker is plan-scoped only; no gate-rung or provider breakers.
- Stuck detection as a Lens (v2-depth 16 §6), diagnosis as a Route Cell (16 §4), auto-fix Compose Cell (16 §5 — legacy `try_pre_agent_cargo_remediation` was orchestrate.rs-only per 36-ORCHESTRATION:113).
- Any conductor presence in runner v2: no watcher loop, no per-plan failure budget across tasks, no health monitors, no stuck detection, no routing bias (`conductor_load: 0.0` hardcoded, event_loop.rs:4258).
- Integration tests: nothing proves a watcher can stop a real run in any engine.

## Migration checklist

- [ ] **[P0]** Decide the conductor's v2 home and wire a minimal loop into runner v2: periodic `Conductor::check_all` over a `RunnerEvent`→`Engram` adapter (or StateHub events), enforcing Restart/Fail like orchestrate.rs:9092-9113 did — today the default binary has zero anomaly supervision beyond static timeouts — verify: forced 3× identical compile-fail in a live `roko plan run --engine runner-v2` aborts via conductor, and `.roko/events.jsonl` contains a `conductor` intervention event
- [ ] **[P0]** Fix CLAUDE.md roko-conductor row ("Used by executor internals" → "dormant; legacy-orchestrate only") — verify: `grep -n "conductor" CLAUDE.md` matches reality
- [ ] **[P1]** Add a per-plan failure budget (circuit breaker) to runner v2 retries — retry_budget (event_loop.rs:1124) is per-task; nothing stops a plan burning N tasks × M retries — verify: plan with 3 failing tasks stops after budget, snapshot records breaker state
- [ ] **[P1]** Remove dead `roko-conductor` deps from roko-serve and roko-orchestrator (or start using them) — verify: `cargo build -p roko-serve -p roko-orchestrator` after removal; `grep -rn roko_conductor crates/roko-serve crates/roko-orchestrator` = 0
- [ ] **[P1]** Make `ConductorConfig` real: call `Conductor::from_config` at the (new) instantiation site, or delete `[conductor.watchers.*]`/`context_pressure_enabled` from schema.rs:1257-1280 and docs — verify: setting `watchers.ghost_turn.max_consecutive = 1` in roko.toml changes behavior in a test
- [ ] **[P2]** Close the learning loops or cut the claims: feed `ThresholdLearner` (`record_intervention_outcome`), install `BanditPolicy` after warmup, call `with_provider_health` — or move COND-03/09 code behind a `future` module — verify: `.roko/learn/` gains threshold-learner state after interventions
- [ ] **[P2]** INT-19: thread `DreamTrigger::CoordinationPattern` into the dream runner instead of `let _trigger` (orchestrate.rs:8351 today; port with the v2 wiring) — verify: dream journal entry records `trigger: coordination_pattern`
- [ ] **[P2]** Emit the signals watchers need from runner v2 (`TokenUsage` for context pressure, `Metric plan_cost/plan_budget`, `CompileDiagnostic`, ghost-turn events) or re-target watchers at `RuntimeEvent` kinds — verify: each watcher has ≥1 integration test with v2-emitted events
- [ ] **[P3]** Quarantine or implement dormant modules (`yerkes_dodson`, `federation`, `self_healing`, `state_machine`) per v2-depth 14-17 Cell designs — verify: `cargo build` after moving to feature `conductor-experimental`, or Cells registered in roko-graph
- [ ] **[P3]** Conductor-as-verify-pipeline: implement watcher Verify Cells + Route-Cell intervention selector in roko-graph once TaskExecutorCell live dispatch lands (36-ORCHESTRATION P0s first) — verify: `roko plan run --engine graph` with an injected failure routes through a conductor cell
- [ ] **[P3]** Keep-or-kill `DashboardSnapshot.diagnoses` (roko-core dashboard_snapshot.rs:768-770): give it a v2 producer or drop the ring + serve plumbing — verify: `GET /api/...` diagnosis surface is either populated by a live run or gone

## Open questions

1. **Re-wire or redesign?** Is the plan to port the existing `Conductor` into runner v2's event loop as-is, or to skip straight to the v2-depth Cell/Lens decomposition (in which case the runner wiring is throwaway)? The answer changes every P1/P2 above.
2. **Which event stream should watchers consume?** Legacy watchers tail `.roko/engrams.jsonl`; runner v2 writes `events.jsonl`/`run-ledger.jsonl` and StateHub `DashboardEvent`s. Signal-kind vocabulary differs (Engram `Kind` vs `RuntimeEvent`).
3. **Severity semantics**: `WorstSeverityPolicy` maps any single Warning to a full phase Restart (interventions.rs:43-51). Is one warning → restart intentional, or should Warning be advisory (cognitive signal only) with Restart reserved for compound patterns?
4. **`MAX_PLAN_FAILURES = 2`** (circuit_breaker.rs:19) is aggressive and not config-exposed at the live call site — intended default for self-hosting runs?
5. **Duplication between `StuckDetector` (12 checks) and the watcher ensemble** (ghost-turn/iteration-loop/stuck-pattern overlap) — converge on one detector family in v2?
6. Does anything still emit `conductor:alert:*`-tagged signals that the WatcherRunner critical-shutdown path (orchestrate.rs:2306-2316) expects, given `emit_tagged_conductor_signal` is also legacy-gated? (Self-contained loop — fine — but worth noting for the port.)
