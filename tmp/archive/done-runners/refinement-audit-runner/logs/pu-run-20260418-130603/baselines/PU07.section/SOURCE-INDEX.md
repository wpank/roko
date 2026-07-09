# SOURCE-INDEX — Code Anchors for 07-Conductor Parity

Code file references extracted from sections A–F, organized by crate and module.

Generated: 2026-04-16

---

## crates/roko-conductor/src/

### Conductor struct + orchestration (doc 00, 01, 02, 03, 07)

| File | What | Section |
|------|------|---------|
| `lib.rs:24-31` | 8 module declarations (`circuit_breaker`, `conductor`, `diagnosis`, `health`, `interventions`, `state_machine`, `stuck_detection`, `watchers`) | A.13 |
| `lib.rs:34-40` | Re-exports (`ConductorDecision`, `CircuitBreaker`, `Conductor`, `RoutingBias`, `InterventionPolicy`, `Severity`, `WatcherOutput`, `WorstSeverityPolicy`, `PhaseTransition`, `phase_timeout`) | A.13 |
| `Cargo.toml:13-21` | Crate deps (`roko-core`, `roko-learn`, `serde`, `parking_lot`, `dashmap`, `tracing`, `chrono`) | A.13 |
| `Cargo.toml:19` | `dashmap = { workspace = true }` dep | A.04 |
| `conductor.rs:9` | `use crate::circuit_breaker::CircuitBreaker` import | B.24 |
| `conductor.rs:26-35` | `RoutingBias { deprioritize, prefer_cheaper, reason }` struct | A.10, B.25 |
| `conductor.rs:53-62` | `Conductor` struct with `watchers`, `policy`, `circuit_breaker`, `routing_bias` | A.02 |
| `conductor.rs:59` | `circuit_breaker: CircuitBreaker` field | B.24 |
| `conductor.rs:60-61` | `routing_bias: Mutex<RoutingBias>` field | A.10, A.02, B.25 |
| `conductor.rs:82-102` | `Conductor::new()` boxes 10 watchers | A.03, B.15, F.04 |
| `conductor.rs:83-94` | Watchers constructor order (GhostTurn, ReviewLoop, IterationLoop, TestFailureBudget, CompileFailRepeat, ContextWindowPressure, SpecDrift, CostOverrun, TimeOverrun, StuckPattern) | B.15, C.14, D.14, D.16, D.18, D.20, F.04 |
| `conductor.rs:84` | GhostTurnWatcher registered | D.18 |
| `conductor.rs:85` | ReviewLoopWatcher registered | D.16, D.20 |
| `conductor.rs:86` | IterationLoopWatcher registered | D.16, D.20 |
| `conductor.rs:88` | CompileFailRepeatWatcher registered | D.20 |
| `conductor.rs:89` | ContextWindowPressureWatcher registered | D.18, D.19 |
| `conductor.rs:90` | SpecDriftWatcher registered | D.16, D.19, D.20 |
| `conductor.rs:91` | CostOverrunWatcher registered | D.18 |
| `conductor.rs:92` | TimeOverrunWatcher registered | D.18 |
| `conductor.rs:93` | StuckPatternWatcher registered | D.11, D.18, D.20 |
| `conductor.rs:98` | `policy: Box::new(WorstSeverityPolicy)` wired as default | A.05, B.04, C.12, F.03 |
| `conductor.rs:99` | `CircuitBreaker::default()` wired | C.01, C.09, D.16 |
| `conductor.rs:108-111` | `Conductor::check_all(&stream)` periodic-tick helper | B.15 |
| `conductor.rs:114-122` | `Conductor::with_watchers()` custom set | B.15 |
| `conductor.rs:118` | `WorstSeverityPolicy` default fallback | F.03 |
| `conductor.rs:119` | `CircuitBreaker` held by `Conductor` | D.16 |
| `conductor.rs:125-128` | `with_policy()` swap-in | B.04 |
| `conductor.rs:132-136` | `with_circuit_breaker()` injector | B.24 |
| `conductor.rs:145-148` | `routing_bias()` snapshot getter | A.10, B.25 |
| `conductor.rs:156-187` | `Conductor::evaluate()` body (OODA loop) | A.12, B.16, B.24, C.07, F.01 |
| `conductor.rs:158-166` | Circuit-breaker early-return check | A.12, B.16, C.07, F.01 |
| `conductor.rs:159` | `is_tripped(&plan_id)` call | B.24, C.07 |
| `conductor.rs:160` | `update_routing_bias(stream, &[])` on trip | C.07 |
| `conductor.rs:161-164` | `ConductorDecision::fail("circuit-breaker", FailureKind::MaxIterations)` | A.12, B.24, C.07, C.11 |
| `conductor.rs:169` | `collect_watcher_outputs` call | A.12, B.16, C.07, F.01 |
| `conductor.rs:170` | `update_routing_bias` on normal path | A.10, B.25, C.07 |
| `conductor.rs:173` | `self.policy.evaluate(&watcher_outputs, ctx)` | A.12, B.16, C.07, F.01 |
| `conductor.rs:176-184` | `record_failure` on `ConductorDecision::Fail` | A.12, B.16, C.07, C.13, C.15 |
| `conductor.rs:186` | Decision returned | A.12, B.16, F.01 |
| `conductor.rs:191-198` | `extract_plan_id` helper | A.11, C.07 |
| `conductor.rs:201-224` | `collect_watcher_outputs` helper (sequential loop) | B.16, C.16 |
| `conductor.rs:207-222` | Sequential watcher iteration | B.16 |
| `conductor.rs:210-214` | Severity string parsing (warning/critical/Info) | B.02, C.14 |
| `conductor.rs:226-255` | `impl Policy for Conductor` | A.02, B.16, C.17 |
| `conductor.rs:227-249` | `decide()` emits signals | C.17 |
| `conductor.rs:230` | `update_routing_bias` inside `decide()` | A.10, B.25 |
| `conductor.rs:240-245` | Emits `Kind::Custom("conductor.decision")` | A.11, C.17 |
| `conductor.rs:258-269` | `update_routing_bias` implementation | A.10, B.25 |
| `conductor.rs:272-315` | `derive_routing_bias` helper (load-pressure + recent-failure groups) | A.10, B.17, B.25 |
| `conductor.rs:277-285` | Binary "load pressure" cost/context/time grouping | E.26 |
| `conductor.rs:278-280` | Resource group (`cost-overrun | context-window-pressure | time-overrun`) | B.17 |
| `conductor.rs:290-298` | Behavioral/coordination group (ghost-turn, review-loop, iteration-loop, test-failure-budget, compile-fail-repeat, stuck-pattern, spec-drift) | B.17 |
| `conductor.rs:396-489` | Decision-flow integration tests | B.16 |
| `conductor.rs:417-428` | `circuit_breaker_aborts_tripped_plan` test | B.16, B.24 |
| `conductor.rs:439-449` | `conductor_policy_emits_on_anomaly` test | B.23 |
| `conductor.rs:459-489` | `multiple_watchers_worst_wins` test | A.05, C.14 |
| `conductor.rs:491-495` | `watcher_count` test asserts `.len() == 10` | A.03, B.15, C.14, F.04 |
| `conductor.rs:494` | `assert_eq!(c.watchers.len(), 10)` | A.03 |
| `conductor.rs:506-538` | `routing_bias_tracks_recent_failures_and_load_pressure` test | B.25 |
| `conductor.rs:509` | `Custom("conductor.agent_output")` emitted in test | F.02 |
| `conductor.rs:540-554` | `routing_bias_deprioritizes_recent_model_failures` test | B.25 |

### Circuit breaker (doc 02)

| File | What | Section |
|------|------|---------|
| `circuit_breaker.rs:7` | `use dashmap::DashMap;` | C.02 |
| `circuit_breaker.rs:11` | `pub const MAX_PLAN_FAILURES: u32 = 2` | A.04, C.01, F.20 |
| `circuit_breaker.rs:14-22` | `FailureRecord { count, last_failure_ms, reasons }` struct + Serialize/Deserialize | A.04, C.02, C.09 |
| `circuit_breaker.rs:28-33` | `CircuitBreaker` struct with `max_failures: u32` + `records: DashMap<String, FailureRecord>` | A.04, C.02, C.03 |
| `circuit_breaker.rs:32` | `records: DashMap<String, FailureRecord>` field | C.02 |
| `circuit_breaker.rs:35-39` | `Default` impl constructs via `MAX_PLAN_FAILURES` | C.01 |
| `circuit_breaker.rs:44-49` | `new(max_failures: u32)` constructor | C.01, C.04 |
| `circuit_breaker.rs:55-63` | `record_failure(&self, plan_id, reason, now_ms) -> bool` | A.04, C.04 |
| `circuit_breaker.rs:67-71` | `is_tripped(&self, plan_id) -> bool` | A.04, C.04, C.07 |
| `circuit_breaker.rs:78-80` | `is_broken` alias | A.04, C.04, C.08 |
| `circuit_breaker.rs:84-86` | `failure_count` method | C.04 |
| `circuit_breaker.rs:89-91` | `reset(&self, plan_id)` | C.04 |
| `circuit_breaker.rs:94-96` | `reset_all` extension | C.04 |
| `circuit_breaker.rs:100-102` | `get_record` accessor | C.04 |
| `circuit_breaker.rs:106-108` | `tracked_plans` extension | C.04 |
| `circuit_breaker.rs:112-114` | `max_failures` accessor | C.04 |
| `circuit_breaker.rs:146-153` | `trips_at_max_failures` test | C.01, C.08 |
| `circuit_breaker.rs:147,157,180,190,238` | Tests using `CircuitBreaker::new(n)` overrides | C.01 |
| `circuit_breaker.rs:206-222` | `concurrent_access_is_safe` test (10 threads × 10 failures) | C.02 |
| `circuit_breaker.rs:225-234` | `tripped_stays_tripped_on_more_failures` test | A.04 |
| `circuit_breaker.rs:247-257` | `FailureRecord` serde roundtrip test | C.09 |

### Interventions + severity (doc 00, 01, 03)

| File | What | Section |
|------|------|---------|
| `interventions.rs:22-31` | `Severity` enum (`Info=0, Warning=1, Critical=2`) with `PartialOrd, Ord` | A.05, B.02, C.11 |
| `interventions.rs:33-45` | `Severity::to_decision()` mapping | A.05 |
| `interventions.rs:36-44` | `Info→cont, Warning→restart, Critical→fail` | B.02, C.11, C.15, F.03 |
| `interventions.rs:41` | `Critical→FailureKind::Other(reason)` | C.11 |
| `interventions.rs:50-60` | `WatcherOutput { watcher, severity, description, metric }` struct | B.03, C.12 |
| `interventions.rs:65-76` | `WatcherOutput::new()` constructor | B.03, C.12 |
| `interventions.rs:80-83` | `.with_metric(v)` chainable | B.03, C.12 |
| `interventions.rs:86-89` | `to_decision()` per-output adaptor | B.03, C.12 |
| `interventions.rs:99-105` | `InterventionPolicy` trait | A.05, B.04, C.12 |
| `interventions.rs:108-121` | `WorstSeverityPolicy` struct + impl | A.05, B.04, C.12, F.03 |
| `interventions.rs:109-111` | `WorstSeverityPolicy` impl of `InterventionPolicy` | F.03 |
| `interventions.rs:113` | `.max_by_key(|o| o.severity)` picker | B.02 |
| `interventions.rs:114` | `map_or_else(ConductorDecision::cont, WatcherOutput::to_decision)` | B.04, C.12 |
| `interventions.rs:123-144` | `outputs_to_signals()` bridge | A.11, B.03, B.23, C.17 |
| `interventions.rs:128` | Filters `Severity::Info` | B.23 |
| `interventions.rs:133` | `Kind::Custom(format!("conductor:alert:{watcher}"))` | A.11, B.23, C.17 |
| `interventions.rs:138-139` | Tags `watcher`, `severity` on engram | B.03, B.23 |
| `interventions.rs:150-154` | `severity_ordering` unit test | B.02, C.11 |
| `interventions.rs:157-171` | Severity→decision branch tests | C.11 |
| `interventions.rs:181-186` | `worst_severity_policy_empty_is_continue` test | B.04, C.12 |
| `interventions.rs:188-198` | `worst_severity_policy_picks_worst` test | B.04, C.12 |
| `interventions.rs:200-209` | `worst_severity_policy_critical_wins` test | A.05, B.04, C.12 |
| `interventions.rs:211-224` | `outputs_to_signals` serialization test | C.17 |
| `interventions.rs:227-232` | `watcher_output_serde_roundtrip` test | B.03 |

### Diagnosis engine (doc 00, 04, 14)

| File | What | Section |
|------|------|---------|
| `diagnosis.rs:23-67` | `ErrorCategory` enum with 20 variants | A.06, D.01, D.17, F.09 |
| `diagnosis.rs:26-67` | `ErrorCategory` span | F.09 |
| `diagnosis.rs:28-66` | 20 category variants | D.01 |
| `diagnosis.rs:44` | `ImportError` variant (not `ImportNotFound`) | D.17 |
| `diagnosis.rs:72-94` | `SuggestedIntervention` enum with 9 variants | A.06, D.02, D.17 |
| `diagnosis.rs:75-93` | 9 intervention variants | A.06, D.02 |
| `diagnosis.rs:100-111` | `ErrorPattern` struct (`name, needle, category, suggested_action, case_insensitive`) | D.07 |
| `diagnosis.rs:110` | `case_insensitive: bool` flag | D.06 |
| `diagnosis.rs:148` | Rustdoc "20+ error patterns" stale string | D.04 |
| `diagnosis.rs:181-186` | Confidence sort step | D.06 |
| `diagnosis.rs:201-225` | `match_pattern` fn (substring search) | D.06 |
| `diagnosis.rs:208` | `haystack.find(&needle)` call | D.06 |
| `diagnosis.rs:213-216` | Excerpt extraction | D.06 |
| `diagnosis.rs:234-261` | `compute_confidence` dynamic scoring | D.07 |
| `diagnosis.rs:276` | Inline comment "20+ error patterns" stale | A.06, D.04 |
| `diagnosis.rs:277-531` | `built_in_patterns()` with 34 patterns | A.06, D.03, D.09 |
| `diagnosis.rs:281,288,295,302,309,316,323,330,337,345,352,360,368,375,383,390,398,405,413,420,428,436,443,450,458,465,472,479,486,493,501,508,515,523` | 34 `ErrorPattern {` openings | D.03 |
| `diagnosis.rs:290` | E0308 pattern | D.05 |
| `diagnosis.rs:300,307,314` | BorrowCheckerError suggested actions | D.09 |
| `diagnosis.rs:304` | E0382 pattern | D.05 |
| `diagnosis.rs:318` | E0106 pattern | D.05 |
| `diagnosis.rs:321,328` | LifetimeError suggested actions | D.09 |
| `diagnosis.rs:331-336` | E0432 pattern (only) | D.05, D.17 |
| `diagnosis.rs:332` | E0432 in patterns | D.17 |
| `diagnosis.rs:335,342` | ImportError actions (`RetryWithContext`) | D.09 |
| `diagnosis.rs:339` | "cannot find" pattern | D.05 |
| `diagnosis.rs:347` | "test result: FAILED" pattern | D.05 |
| `diagnosis.rs:350,357` | TestFailure actions (`AutoFix`) | D.09 |
| `diagnosis.rs:362` | Generic `warning:` pattern (not `clippy::`) | D.05 |
| `diagnosis.rs:365` | ClippyWarning action (`AutoFix`) | D.09 |
| `diagnosis.rs:377` | `CONFLICT (content)` pattern | D.05 |
| `diagnosis.rs:392` | `failed to select a version for` DependencyError | D.17 |
| `diagnosis.rs:415` | `Connection refused` pattern | D.05 |
| `diagnosis.rs:452` | `No space left on device` pattern | D.05 |
| `diagnosis.rs:460` | "rate limit" pattern | D.05 |
| `diagnosis.rs:474` | `context_length_exceeded` pattern | D.05 |
| `diagnosis.rs:517` | `thread 'main' panicked` pattern | D.05 |
| `diagnosis.rs:544-546` | `has_at_least_20_patterns` test (stale name) | D.04 |

### Health monitor (doc 00, 06)

| File | What | Section |
|------|------|---------|
| `health.rs:1-18` | File-level doc module | E.07 |
| `health.rs:26-33` | `HealthStatus { Healthy=0, Degraded=1, Critical=2 }` | A.08, E.02 |
| `health.rs:80` | `HealthCheckResult` + monitor boundaries | F.09 |
| `health.rs:92-114` | `SystemSnapshot` struct | A.08, E.01 |
| `health.rs:95` | `active_agents: u32` | E.01 |
| `health.rs:97` | `expected_agents: u32` | E.01 |
| `health.rs:99` | `last_agent_heartbeat_ms: i64` | E.01 |
| `health.rs:101` | `chain_connected: bool` | E.01 |
| `health.rs:103` | `chain_expected: bool` | E.01 |
| `health.rs:105` | `spec_hash_at_start: String` | E.01 |
| `health.rs:107` | `spec_hash_current: String` | E.01 |
| `health.rs:109` | `coverage_history: Vec<f64>` | E.01 |
| `health.rs:111` | `now_ms: i64` extra field | E.01 |
| `health.rs:113` | `heartbeat_timeout_ms: i64` extra field | E.01 |
| `health.rs:148-172` | `HealthMonitor::new()` registers 4 checks | A.08 |
| `health.rs:152-171` | Four `NamedCheck` registrations | E.03 |
| `health.rs:155` | `terminal_liveness` check | E.03 |
| `health.rs:159` | `golem_status` check (drift name) | A.08, E.03, E.05 |
| `health.rs:163` | `spec_drift` check | E.03, E.05 |
| `health.rs:167` | `coverage_trend` check | E.03, E.05 |
| `health.rs:176-178` | `check_count()` accessor | E.04 |
| `health.rs:182-184` | `check_all()` per-check results | D.19, E.04 |
| `health.rs:188-194` | `overall_status()` aggregation (worst wins) | E.02, E.04 |
| `health.rs:201-255` | `check_terminal_liveness()` | D.19, E.03 |
| `health.rs:218-227` | Active-agents-match-expected logic | E.03 |
| `health.rs:258-270` | `check_golem_status` (chain-status) | A.08, D.19, E.03, E.05 |
| `health.rs:273` | `check_spec_drift` | D.19 |
| `health.rs:292-327` | `check_coverage_trend` | D.19, E.06 |
| `health.rs:308` | Critical threshold `-5.0` | E.06 |
| `health.rs:314` | Degraded threshold `-1.0` | E.06 |
| `health.rs:465-473` | `coverage_drop_is_critical` test | E.06 |
| `health.rs:476-485` | `coverage_slight_decline_is_degraded` test | E.06 |
| `health.rs:525-540` | `overall_status_picks_worst` test | E.02 |
| `health.rs:545-548` | `health_status_ordering` test | E.02 |

### State machine + phase timeouts (doc 00, 10)

| File | What | Section |
|------|------|---------|
| `state_machine.rs:13-31` | `TIMEOUT_*` constants | E.08 |
| `state_machine.rs:25` | `TIMEOUT_ENRICHING = 120` | A.09, E.08 |
| `state_machine.rs:27` | `TIMEOUT_VERIFYING = 300` | A.09, E.08 |
| `state_machine.rs:29` | `TIMEOUT_AUTO_FIXING = 300` | A.09, E.08 |
| `state_machine.rs:31` | `TIMEOUT_DOC_REVISION = 120` | A.09, E.08 |
| `state_machine.rs:37-55` | `phase_timeout()` const fn | A.09, E.08 |
| `state_machine.rs:39-44` | Implementing complexity branch (600/120/300) | A.09 |
| `state_machine.rs:41` | `TaskComplexityBand::Fast → TIMEOUT_IMPLEMENTING_SIMPLE` | E.08 |
| `state_machine.rs:45` | `Gating → 300` | A.09 |
| `state_machine.rs:46` | Verifying timeout | A.09 |
| `state_machine.rs:47` | `Reviewing → 300` | A.09 |
| `state_machine.rs:48` | `Merging → 60` | A.09 |
| `state_machine.rs:49` | Enriching timeout | A.09 |
| `state_machine.rs:50` | AutoFixing timeout | A.09 |
| `state_machine.rs:51` | DocRevision timeout | A.09 |
| `state_machine.rs:53` | Terminal phases return `None` | E.08 |
| `state_machine.rs:61-73` | `PhaseTransition` struct (`plan_id`, `from`, `to`, `at_ms`, `reason`) | A.09, E.09 |
| `state_machine.rs:89-92` | `with_reason()` helper | E.09 |
| `state_machine.rs:96-99` | `elapsed_ms()` helper | E.09 |
| `state_machine.rs:103-108` | `elapsed_secs()` helper | E.09 |
| `state_machine.rs:115-129` | `implementing_timeout_varies_by_complexity` test | E.08 |
| `state_machine.rs:131-142` | `non_implementing_phases_ignore_complexity` test | E.08 |
| `state_machine.rs:168-186` | `all_active_phases_have_timeouts` test | E.08 |
| `state_machine.rs:190,197,205,212` | `PhaseTransition::new` test-only uses | E.09 |

### Stuck detection (doc 00, 05)

| File | What | Section |
|------|------|---------|
| `stuck_detection.rs:25` | `OperatingFrequency` re-export | D.12 |
| `stuck_detection.rs:34-47` | `StuckKind` enum (6 variants) | A.07, D.10, F.09 |
| `stuck_detection.rs:107-132` | `StuckThresholds` struct | A.07 |
| `stuck_detection.rs:125` | `output_loop_count: 4` default | D.10 |
| `stuck_detection.rs:126` | `no_progress_ms: 300_000` default | D.10 |
| `stuck_detection.rs:127` | `gate_loop_count: 3` default | D.10 |
| `stuck_detection.rs:128` | `compile_loop_count: 3` default | D.10 |
| `stuck_detection.rs:129` | `empty_output_count: 3` default | D.10 |
| `stuck_detection.rs:130` | `excessive_retry_count: 6` default | D.10 |
| `stuck_detection.rs:178-204` | `StuckDetector::check_stuck` dispatcher | A.07, D.10, D.11 |
| `stuck_detection.rs:208-233` | `check_all` runs six checks | D.10 |
| `stuck_detection.rs:264` | `MetaCognitionAssessment::frequency = Theta` | D.12, F.07 |
| `stuck_detection.rs:278-311` | `check_output_loop` | D.10 |
| `stuck_detection.rs:315-341` | `check_no_progress` | D.10 |
| `stuck_detection.rs:344-381` | `check_gate_loop` | D.10 |
| `stuck_detection.rs:384-417` | `check_compile_loop` | D.10 |
| `stuck_detection.rs:420-447` | `check_empty_output` | D.10 |
| `stuck_detection.rs:450-473` | `check_excessive_retries` | D.10 |
| `stuck_detection.rs:504` | `OperatingFrequency::Theta` reference | F.07 |
| `stuck_detection.rs:524-539` | `MetaCognitionAssessment::to_signal()` | D.11 |
| `stuck_detection.rs:544-591` | `MetaCognitionHook` struct + impl | A.07 |
| `stuck_detection.rs:582-584` | `frequency() -> OperatingFrequency::Theta` | A.07, D.12, F.07 |
| `stuck_detection.rs:587-590` | `assess()` delegates | A.07 |
| `stuck_detection.rs:593-627` | `classify_meta_cognition_action` | D.13 |
| `stuck_detection.rs:600-606` | Escalate branch (GateLoop | CompileLoop | ExcessiveRetries) | D.13 |
| `stuck_detection.rs:602` | `ExcessiveRetries → Escalate` | D.13 |
| `stuck_detection.rs:608-624` | AdjustStrategy branch | D.13 |
| `stuck_detection.rs:1040-1042` | `meta_cognition_is_theta_frequency` test | D.12 |
| `stuck_detection.rs:1060-1072` | `meta_cognition_escalates_for_gate_failure_patterns` test | D.13 |

### Watchers (doc 01, 03)

| File | What | Section |
|------|------|---------|
| `watchers/mod.rs:8-17` | 10 watcher module declarations | A.03 |
| `watchers/mod.rs:19-28` | Watcher re-exports | A.03, B.15 |
| `watchers/ghost_turn.rs:11` | `MAX_GHOST_TURNS: usize = 3` | B.05, D.18, F.10 |
| `watchers/ghost_turn.rs:14` | `WATCHER_NAME = "ghost-turn"` | B.05 |
| `watchers/ghost_turn.rs:17` | `TURN_SIGNAL_KIND = "conductor.ghost_turn"` | A.11, B.05 |
| `watchers/ghost_turn.rs:19-32` | `GhostTurnEvent` (11 fields) | B.05 |
| `watchers/ghost_turn.rs:40` | `GhostTurnWatcher` struct | D.18 |
| `watchers/ghost_turn.rs:62` | Reads `Kind::Custom("conductor.ghost_turn")` | B.21 |
| `watchers/ghost_turn.rs:65-81` | `extract_ghost_turn_event` | B.05 |
| `watchers/ghost_turn.rs:84-140` | Decide loop (reverse scan, counts consecutive) | B.05 |
| `watchers/ghost_turn.rs:124` | `severity=warning` tag | B.05, C.14 |
| `watchers/ghost_turn.rs:219-273` | At-threshold / below-threshold tests | B.05 |
| `watchers/compile_fail_repeat.rs:9` | `MAX_IDENTICAL_COMPILE_FAILURES: usize = 3` | B.06 |
| `watchers/compile_fail_repeat.rs:12` | Watcher name `compile-fail-repeat` | B.06 |
| `watchers/compile_fail_repeat.rs:42-61` | `diagnostic_key()` fingerprint | B.06 |
| `watchers/compile_fail_repeat.rs:68` | Reads `Kind::CompileDiagnostic` | B.06, B.21 |
| `watchers/compile_fail_repeat.rs:85-97` | Fires `severity=warning` | B.06 |
| `watchers/compile_fail_repeat.rs:94` | `severity=warning` tag | C.14 |
| `watchers/compile_fail_repeat.rs:146-179` | Identical-errors / different-errors tests | B.06 |
| `watchers/cost_overrun.rs:10` | Watcher name `cost-overrun` | B.07 |
| `watchers/cost_overrun.rs:15` | `PLAN_COST_METRIC = "plan_cost"` | B.07 |
| `watchers/cost_overrun.rs:17` | `PLAN_BUDGET_METRIC = "plan_budget"` | B.07 |
| `watchers/cost_overrun.rs:22` | `DEFAULT_BUDGET: f64 = 10.0` (no `_USD`) | B.07 |
| `watchers/cost_overrun.rs:51-58` | `latest_metric()` helper | B.07 |
| `watchers/cost_overrun.rs:55` | Reads `Kind::Metric` (plan_cost / plan_budget) | B.21 |
| `watchers/cost_overrun.rs:60-87` | Decide path (severity=warning always) | B.07 |
| `watchers/cost_overrun.rs:79` | `severity=warning` tag | C.14 |
| `watchers/cost_overrun.rs:134-167` | `above_budget_fires` / `below_budget_no_fire` / `uses_most_recent_cost` tests | B.07 |
| `watchers/iteration_loop.rs:9` | `MAX_IMPLEMENTER_ATTEMPTS: usize = 3` | B.08, C.13 |
| `watchers/iteration_loop.rs:12` | Watcher name `iteration-loop` | B.08 |
| `watchers/iteration_loop.rs:41,62,90` | Reads `Kind::PlanPhase` | B.21 |
| `watchers/iteration_loop.rs:94-96` | Filters `plan_event == "GateFailed"` | B.08 |
| `watchers/iteration_loop.rs:104` | `severity=critical` tag (only watcher) | B.08, C.14 |
| `watchers/iteration_loop.rs:111-117` | Counter resets on GatePassed/etc | B.08 |
| `watchers/iteration_loop.rs:145-207` | Tests | B.08 |
| `watchers/iteration_loop.rs:172` | Test assertion verifying critical severity | B.08 |
| `watchers/review_loop.rs:10` | `MAX_REVIEW_CYCLES: usize = 3` | B.09 |
| `watchers/review_loop.rs:13` | Watcher name `review-loop` | B.09 |
| `watchers/review_loop.rs:42,63,92` | Reads `Kind::PlanPhase` | B.21 |
| `watchers/review_loop.rs:58-60` | `latest_plan_id()` | B.09 |
| `watchers/review_loop.rs:78-121` | Decide loop (ReviewRejected / reset verbs) | B.09 |
| `watchers/review_loop.rs:93-109` | Counter increments at threshold | B.09 |
| `watchers/review_loop.rs:104` | `severity=warning` tag | C.14 |
| `watchers/review_loop.rs:111-113` | Reset on ReviewApproved/DocRevisionDone/MergeSucceeded | B.09 |
| `watchers/review_loop.rs:169-227` | Tests | B.09 |
| `watchers/spec_drift.rs:12` | `MAX_SPEC_DRIFT_RATIO: f64 = 0.25` | B.10 |
| `watchers/spec_drift.rs:15` | Watcher name `spec-drift` | B.10 |
| `watchers/spec_drift.rs:24-38` | `SpecDriftEvent` struct | B.10 |
| `watchers/spec_drift.rs:40-46` | `path_is_allowed()` | B.10 |
| `watchers/spec_drift.rs:61-72` | `drift_ratio()` fallback | B.10 |
| `watchers/spec_drift.rs:101-157` | Decide path (strictly greater) | B.10 |
| `watchers/spec_drift.rs:107` | Reads `Kind::Metric` (spec_drift) | B.21 |
| `watchers/spec_drift.rs:118` | Tag-based fallback `METRIC_VALUE_TAG` | B.10 |
| `watchers/spec_drift.rs:145` | `severity=warning` tag | C.14 |
| `watchers/spec_drift.rs:184-262` | Threshold boundary / JSON payload tests | B.10 |
| `watchers/stuck_pattern.rs:10` | `MAX_IDENTICAL_ACTIONS: usize = 4` | B.11, D.11, D.18 |
| `watchers/stuck_pattern.rs:13` | Watcher name `stuck-pattern` | B.11 |
| `watchers/stuck_pattern.rs:16` | `ACTION_KINDS = &[AgentOutput, AgentMessage]` | B.11, B.21 |
| `watchers/stuck_pattern.rs:51-72` | `body_fingerprint()` | B.11 |
| `watchers/stuck_pattern.rs:74-122` | Decide loop (reverse walk, breaks on mismatch) | B.11, D.11 |
| `watchers/stuck_pattern.rs:110` | `severity=warning` tag | C.14 |
| `watchers/stuck_pattern.rs:186-223` | Tests | B.11 |
| `watchers/test_failure_budget.rs:13` | `MIN_FAILURE_INCREASE: u32 = 1` | B.12 |
| `watchers/test_failure_budget.rs:16` | Watcher name `test-failure-budget` | B.12 |
| `watchers/test_failure_budget.rs:62` | Reads `Kind::GateVerdict` | B.21 |
| `watchers/test_failure_budget.rs:84` | `baselines.entry(plan_id).or_insert(failed)` | B.12 |
| `watchers/test_failure_budget.rs:85` | `latest.insert(plan_id, failed)` | B.12 |
| `watchers/test_failure_budget.rs:99-111` | Fires with `severity=warning` + tags | B.12 |
| `watchers/test_failure_budget.rs:105` | `severity=warning` tag | C.14 |
| `watchers/test_failure_budget.rs:161-200` | Tests (per-plan independence) | B.12 |
| `watchers/time_overrun.rs:10` | Watcher name `time-overrun` | B.13 |
| `watchers/time_overrun.rs:13` | `TASK_OUTPUT_KIND = "conductor.agent_output"` | A.11, B.13, F.02 |
| `watchers/time_overrun.rs:16` | `ALERT_THRESHOLD: f64 = 0.80` | B.13 |
| `watchers/time_overrun.rs:22-28` | `TaskTimingEvent` struct | B.13 |
| `watchers/time_overrun.rs:39` | Reads `Kind::Custom("conductor.agent_output")` | B.21 |
| `watchers/time_overrun.rs:50-57` | Integer arithmetic `dur*5 > timeout*4` | B.13 |
| `watchers/time_overrun.rs:51-53` | Zero-timeout guard | B.13 |
| `watchers/time_overrun.rs:59-100` | Decide path (most recent) | B.13 |
| `watchers/time_overrun.rs:91` | `severity=warning` tag | C.14 |
| `watchers/time_overrun.rs:124-169` | Above/at-threshold / zero-timeout tests | B.13 |
| `watchers/context_window_pressure.rs:7` | `use roko_learn::efficiency::AgentEfficiencyEvent` | A.13, B.14 |
| `watchers/context_window_pressure.rs:10` | `MAX_CONTEXT_USAGE_RATIO: f64 = 0.80` | B.14, E.28, F.10 |
| `watchers/context_window_pressure.rs:13` | Watcher name `context-window-pressure` | B.14 |
| `watchers/context_window_pressure.rs:22-23` | Context window constants (1M opus / 200k haiku-sonnet) | B.14 |
| `watchers/context_window_pressure.rs:52` | `impl Policy for ContextWindowPressureWatcher` | A.03 |
| `watchers/context_window_pressure.rs:55` | Reads `Kind::TokenUsage` | A.11, B.21 |
| `watchers/context_window_pressure.rs:71-86` | Firing at `ratio > self.max_ratio` | B.14 |
| `watchers/context_window_pressure.rs:79` | `severity=warning` tag | C.14 |
| `watchers/context_window_pressure.rs:93-114` | `extract_usage()` (efficiency event first, tag fallbacks) | B.14 |
| `watchers/context_window_pressure.rs:95` | `context_window_tokens(&event.model)` lookup | F.17 |
| `watchers/context_window_pressure.rs:116-125` | `context_window_tokens()` substring check | B.14 |
| `watchers/context_window_pressure.rs:173-232` | All three formats + at-threshold tests | B.14 |

---

## crates/roko-core/src/

### Six verb traits + Policy (doc 00, 01, 07)

| File | What | Section |
|------|------|---------|
| `traits.rs:15,65,88,110,135,158` | Six verb traits (`Substrate, Scorer, Gate, Router, Composer, Policy`) | A.01 |
| `traits.rs:166-168` | `Policy` trait signature | A.02 |
| `traits.rs:166-172` | `pub trait Policy: Send + Sync { decide, name }` | B.01 |

### ConductorDecision type (doc 00, 03, 09)

| File | What | Section |
|------|------|---------|
| `conductor.rs:5-8` | Doc comment "Simplified from Mori's InterventionTier" | B.20 |
| `conductor.rs:19` | Historical comment contrasting Mori's Nudge/Restart/Abort | C.10, C.15 |
| `conductor.rs:22-42` | `ConductorDecision` enum `#[non_exhaustive]` 3 variants | B.20, C.07, C.10, C.15 |
| `conductor.rs:25` | `pub enum ConductorDecision` (single hit) | C.10 |
| `conductor.rs:25-42` | Continue, Restart{watcher, reason}, Fail{watcher, reason: FailureKind} | B.20 |
| `conductor.rs:35-41` | `Fail { watcher, reason: FailureKind }` variant | C.07 |
| `conductor.rs:44-67` | `cont()`, `restart()`, `fail()` constructors | B.20 |
| `conductor.rs:71-89` | `is_terminal`, `is_continue`, `label` helpers | B.20 |
| `conductor.rs:127-149` | Serde roundtrip tests (`#[serde(rename_all = "snake_case")]`) | B.20 |

### Kind enum + Context (doc 00, 07, 09)

| File | What | Section |
|------|------|---------|
| `context.rs` | `Context::at(_)` / `Context::now()` tick helpers | B.01 |
| `kind.rs:25-106` | `Kind` enum (28 named variants + `Custom`) | B.21 |
| `kind.rs:66-68` | `RouterChoice`, `RouterFeedback` variants | B.22 |
| `kind.rs:71-77` | `Episode`, `PlaybookRule`, `Skill` variants | B.22 |
| `kind.rs:80-87` | `ExperimentResult`, `ToolInvocation`, `ToolHealthDegraded` variants | B.22 |
| `kind.rs:89-100` | Chain-participation variants (`Insight, Pheromone, Bounty, Transaction, Service, Prediction`) | B.22 |
| `kind.rs:91` | `Kind::Pheromone` with stigmergic doc comment | E.30 |

### Config schema (doc 10)

| File | What | Section |
|------|------|---------|
| `config/schema.rs:452-496` | `effective_providers()` | E.11 |
| `config/schema.rs:981-1020` | `ProviderConfig { timeout_ms, ttft_timeout_ms, connect_timeout_ms }` | E.10, E.11 |
| `config/schema.rs:1022-1032` | Default fn helpers (120s / 15s / 5s) | E.11 |

### Error retry breaker (doc 02 — naming collision)

| File | What | Section |
|------|------|---------|
| `error/retry.rs:117-126` | `BreakerState { Closed, Open, HalfOpen }` | C.05 |
| `error/retry.rs:139` | Second `CircuitBreaker` struct (generic backend) | C.03, C.05 |
| `error/retry.rs:139-145` | Three-state retry breaker | C.05 |
| `error/retry.rs:164-186` | State transitions | C.05 |

### Affect / operating frequency (doc 12)

| File | What | Section |
|------|------|---------|
| `affect.rs:10` | `Arousal` (PAD dimension, not Yerkes-Dodson) | E.22 |
| `operating_frequency.rs:97,232` | Arousal in PAD | E.22 |

---

## crates/roko-learn/src/

### Anomaly detector (doc 11)

| File | What | Section |
|------|------|---------|
| `anomaly.rs:9-10` | `PROMPT_LOOP_WINDOW=20`, `PROMPT_LOOP_THRESHOLD=5` | D.18, F.15 |
| `anomaly.rs:11` | `COST_SPIKE_Z_THRESHOLD: f64 = 3.0` | D.18 |
| `anomaly.rs:18-26` | `AnomalyDetector` struct (5 fields) | F.15 |
| `anomaly.rs:52-69` | `check_prompt` (5+ identical in 20) | D.18, F.15 |
| `anomaly.rs:52-80` | Prompt loop check range | D.18 |
| `anomaly.rs:95-118` | `check_quality` (dual condition) | F.15, F.28 |
| `anomaly.rs:113` | `recent < earlier - 0.15 && recent < 0.5` | F.15 |
| `anomaly.rs:120-131` | `check_budget` | F.08, F.15 |
| `anomaly.rs:152-188` | `EwmaState` struct + update | F.13, F.15 |
| `anomaly.rs:172-176` | EWMA update (`mean += alpha * diff; variance = ...`) | F.15 |
| `anomaly.rs:180-187` | `z_score` helper | F.15 |
| `anomaly.rs:205-229` | `Anomaly` enum (4 variants) | F.15 |
| `anomaly.rs:236-310` | Unit tests for all 4 anomalies | F.15 |
| `anomaly.rs:270-289` | `QualityDegradation` test | F.28 |
| `anomaly.rs` | `AnomalyDetector::check_quality()` (spec-drift quality) | D.16 |

### ConductorBandit (doc 15)

| File | What | Section |
|------|------|---------|
| `conductor.rs:28-36` | 7-action enum (`Continue, InjectHint×3, SwitchModel, Restart, Abort`) | F.21 |
| `conductor.rs:175-200` | `ConductorBandit::save` method | F.21 |
| `conductor.rs:204-218` | `select_action(&state) -> ConductorAction` | F.22 |
| `conductor.rs:221-247` | `record_outcome` | F.22 |
| `conductor.rs:249-291` | 19-dim state encoding (iter, failures, elapsed, cost, tier, complexity, error_patterns, interactions) | F.21 |
| `conductor.rs:305-337` | `reward_for_outcome` (futility-weighted) | F.23 |
| `conductor.rs:305-462` | Full reward function | F.23 |
| `conductor.rs:311-318` | Success branch (`Continue=1.0, InjectHint=0.92, SwitchModel=0.88, Restart=0.82, Abort=0.0`) | F.23 |
| `conductor.rs:320-336` | Failure branch (futility-weighted) | F.23 |
| `conductor.rs:432-462` | `futility_score` helper | F.23 |
| `conductor.rs:553-596` | Tests (abort dominates after mechanical failures) | F.21 |

### Efficiency events (doc 11)

| File | What | Section |
|------|------|---------|
| `efficiency.rs:34-46` | `PromptSectionMeta` | F.17 |
| `efficiency.rs:51-70` | `ToolCallMeta` | F.17 |
| `efficiency.rs:79-80` | `AgentEfficiencyEvent` struct (20+ fields) | F.17 |

### Cascade router feedback (doc 11)

| File | What | Section |
|------|------|---------|
| `runtime_feedback.rs:727-743` | `record_conductor_intervention` | F.18 |
| `runtime_feedback.rs:743` | `cascade_router.save` error-log | F.18 |
| `runtime_feedback.rs:2536` | Test confirms code path | F.18 |

### Latency / adaptive timeout (doc 10)

| File | What | Section |
|------|------|---------|
| `latency.rs:70-77` | `LatencyStats::adaptive_timeout_ms` (10 obs warmup, `[5s, 300s]` clamp) | E.10 |
| `latency.rs:481-514` | Cold-start + clamp tests | E.10 |

### Provider health (doc 02, 11)

| File | What | Section |
|------|------|---------|
| `provider_health.rs:6-13` | ASCII state-machine diagram | F.20 |
| `provider_health.rs:16-18,29` | `parking_lot::RwLock` concurrency | F.20 |
| `provider_health.rs:40-50` | Snapshot types | F.20 |
| `provider_health.rs:42-50` | `enum CircuitState { Closed, Open, HalfOpen }` | C.05 |
| `provider_health.rs:82` | Third `CircuitBreaker` — `ProviderHealth` wrapper | C.03, C.05 |
| `provider_health.rs:82-99` | `ProviderHealth` breaker | C.03 |
| `provider_health.rs:109-116` | `record_success` | C.05 |
| `provider_health.rs:119-137` | `record_failure` | C.05 |
| `provider_health.rs:143-157` | `is_available` | C.05 |
| `provider_health.rs:160-168` | `cooldown_ms` per error type (5s/10s/30s/5min) | C.05, C.06, C.16 |
| `provider_health.rs:182-196` | `ProviderHealthRegistry` | F.20 |
| `provider_health.rs:519-528` | `ProviderHealthTracker` | F.20 |
| `provider_health.rs:733-752` | Tracker state-transition tests | F.20 |

### Active inference + gate-rung EMA (doc 08)

| File | What | Section |
|------|------|---------|
| `active_inference.rs` | Active-inference module (for learning, not conductor forward predictor) | F.14 |
| `pattern_discovery.rs:99+` | `PatternMiner` trigram miner | F.27 |

---

## crates/roko-gate/src/

### Adaptive thresholds (doc 08, 11)

| File | What | Section |
|------|------|---------|
| `adaptive_threshold.rs:11` | `EMA_ALPHA: f64 = 0.1` | F.19 |
| `adaptive_threshold.rs:13-16` | `MIN_RETRIES: u32 = 1`, `MAX_RETRIES: u32 = 5` | F.19 |
| `adaptive_threshold.rs:19` | `SKIP_STREAK_THRESHOLD: u32 = 20` | F.19 |
| `adaptive_threshold.rs:23-30` | `RungStats { ema_pass_rate, total_observations, consecutive_passes }` | F.12, F.19 |
| `adaptive_threshold.rs:35` | `ema_pass_rate: 0.5` neutral start | F.19 |
| `adaptive_threshold.rs:44-47` | `AdaptiveThresholds { rungs: HashMap<u32, RungStats> }` | F.12, F.19 |
| `adaptive_threshold.rs:58-60` | `load_or_new` persistence | F.19 |

---

## crates/roko-runtime/src/

### ProcessSupervisor (doc 13)

| File | What | Section |
|------|------|---------|
| `process.rs:139` | `SpawnConfig::default` grace = 5s | E.17 |
| `process.rs:147-156` | `ProcessOutcome` struct | E.15 |
| `process.rs:159-170` | `ProcessHandle` (`id`, `label`, `child`, `os_pid`, `grace_period`, `cancel`, `spawn_config`, `started_at`) | E.15 |
| `process.rs:182-197` | `wait_for_graceful_exit()` | E.17 |
| `process.rs:199-204` | `force_kill()` | E.17 |
| `process.rs:244-251` | `ProcessHandle::shutdown()` | E.17 |
| `process.rs:280-591` | `ProcessSupervisor` implementation | E.14 |
| `process.rs:312,603,628,651` | `supervisor.spawn` only in tests | E.14 |
| `process.rs:451-480` | `reap_exited()` (one-shot) | E.16 |
| `process.rs:483-485` | `supervisor.count()` returns zero during real runs | E.14 |
| `process.rs:510-542` | `restart_process` | E.15 |

### Resource accounting (doc 13)

| File | What | Section |
|------|------|---------|
| `resource.rs:11-148` | `ResourceAccount` struct (budget tiers) | E.20, E.21 |
| `resource.rs:86-88` | `any_exceeded()` | E.20 |
| `resource.rs:92-114` | `token_utilisation / cost_utilisation / time_utilisation` | E.21, E.26 |
| `resource.rs:129-132` | `trivial(label)` constructor | E.21 |
| `resource.rs:134-137` | `simple(label)` constructor | E.21 |
| `resource.rs:139-142` | `standard(label)` constructor | E.21 |
| `resource.rs:144-147` | `complex(label)` constructor | E.21 |

---

## crates/roko-agent/src/

### Agent-side process management (doc 13 — parallel to runtime)

| File | What | Section |
|------|------|---------|
| `exec.rs:178` | Agent subprocess spawn + `register_spawned_pid` | E.14 |
| `claude_cli_agent.rs:420` | Claude CLI spawn + `register_spawned_pid` | E.14 |
| `process/group.rs:15-26` | `set_process_group` — `libc::setpgid(0, 0)` | E.18 |
| `process/group.rs:38-65` | `collect_descendants()` via `pgrep -P` (depth cap 8) | E.18, E.19 |
| `process/group.rs:41,43` | Depth cap at 8 | E.18 |
| `process/group.rs:87-118` | `kill_process_group()` | E.18 |
| `process/kill.rs:21` | `GRACE_SIGTERM_MS` = 800ms | E.17 |
| `process/kill.rs:33-71` | `kill_tree()` SIGTERM→SIGKILL escalation | E.17 |
| `process/kill.rs:36` | `drop(child.stdin.take())` EOF | E.17 |
| `process/kill.rs:39` | Wait `grace` for natural exit | E.17 |
| `process/kill.rs:49` | `kill_process_group(child, SIGTERM)` | E.17 |
| `process/kill.rs:53` | 800ms grace | E.17 |
| `process/kill.rs:62` | `kill_process_group(child, SIGKILL)` | E.17 |
| `process/kill.rs:91-116` | `kill_tree_escalates_to_sigkill` test (bash trap TERM) | E.17 |
| `process/registry.rs:89-148` | `cleanup_orphaned_agents()` (cross-restart cleanup) | E.16 |
| `process/registry.rs:166-229` | `reap_orphaned_children()` (parent-PID-1 detection) | E.16 |
| `safety/` | Path / bash safety guards | F.08 |
| `tool_loop/max_iter.rs:7` | `DEFAULT_MAX_ITERATIONS=25` | E.23 |
| `task_runner.rs:24` | `AnomalyDetector` thread | F.16 |
| `task_runner.rs:26` | `RunnerAnomalyDetector` re-export | F.16 |

---

## crates/roko-cli/src/

### Orchestrator integration (doc 00, 02, 03, 04, 06, 10, 11, 13, 15)

| File | What | Section |
|------|------|---------|
| `orchestrate.rs:26` | `BudgetGuardrail` import | F.08 |
| `orchestrate.rs:27,74` | `ConductorBandit` imports | F.21 |
| `orchestrate.rs:36` | `use roko_conductor::diagnosis::{DiagnosisEngine, ErrorCategory}` | A.12, D.08 |
| `orchestrate.rs:37` | `use roko_conductor::{Conductor, ConductorDecision}` | A.12 |
| `orchestrate.rs:147` | `GHOST_TURN_SIGNAL_KIND` constant | D.18 |
| `orchestrate.rs:578-636` | `drain_turn_learning_events` (anticipate-don't-react) | F.16 |
| `orchestrate.rs:583-584` | `anomaly_detector.check_prompt(feedback.prompt_hash)` on TurnStarted | F.16 |
| `orchestrate.rs:610-611` | `anomaly_detector.check_cost(feedback.cost_usd)` on CostRecorded | F.16 |
| `orchestrate.rs:615-616` | `tracing::warn!(... "learning anomaly detected from cost")` | F.28 |
| `orchestrate.rs:646-658` | `save_snapshot_atomic` (temp-then-rename) | C.09, E.13 |
| `orchestrate.rs:647-649` | `create_dir_all(parent)` | E.13 |
| `orchestrate.rs:651` | `tmp_path = path.with_extension("json.tmp")` | E.13 |
| `orchestrate.rs:653-654` | `std::fs::write` tmp | E.13 |
| `orchestrate.rs:655-656` | `std::fs::rename(tmp, path)` | E.13 |
| `orchestrate.rs:660-681` | `wait_for_shutdown_signal` (SIGINT/SIGTERM Unix, ctrl_c Windows) | E.12 |
| `orchestrate.rs:1474` | `self.conductor.check_all(&signals)` tick | A.12, E.05 |
| `orchestrate.rs:1674` | `latest_efficiency_event` reader | F.17 |
| `orchestrate.rs:1787-1795` | `cascade_routing_bias_from_conductor` consumer | A.10 |
| `orchestrate.rs:2135-2172` | `PlanRunner` struct | E.05, E.14 |
| `orchestrate.rs:2172` | `supervisor: Arc<ProcessSupervisor>` field | E.14 |
| `orchestrate.rs:2186` | `retry_conductor: ConductorBandit` field | F.21 |
| `orchestrate.rs:2198` | `AnomalyDetector` threaded through PlanRunner | F.16 |
| `orchestrate.rs:2222,3300,3419,3542` | Retained `Vec<AgentEfficiencyEvent>` in runner state | F.17 |
| `orchestrate.rs:3254,3373,3496` | `ProcessSupervisor::new(cancel.clone())` per PlanRunner build | E.14 |
| `orchestrate.rs:3258,3377,3500` | `Arc::new(Conductor::new())` construction sites | A.12 |
| `orchestrate.rs:3261,3380,3503` | `ConductorBandit::load_or_new(&conductor_policy_path(workdir))` | F.21 |
| `orchestrate.rs:3279,3398,3521` | Session-wide `AnomalyDetector` constructed | F.16 |
| `orchestrate.rs:3318-3430` | `PlanRunner::from_snapshot` | C.09 |
| `orchestrate.rs:3721` | `supervisor.shutdown_all()` | E.14 |
| `orchestrate.rs:3754-3761` | `supervisor.kill_all()` | E.12 |
| `orchestrate.rs:3766` | `self.flush_logs().await` (shutdown phase 4) | E.12 |
| `orchestrate.rs:3844-3881` | `handle_tripped_circuit_breaker` helper | A.12, C.08, C.17, D.16 |
| `orchestrate.rs:3844-3892` | Tripped breaker path + `ensure_dispatch_allowed` | D.16 |
| `orchestrate.rs:3850` | `circuit_breaker().get_record(plan_id)` | C.08 |
| `orchestrate.rs:3859-3869` | `DiagnosisEngine::default().diagnose(&error_output)` | D.08, D.20 |
| `orchestrate.rs:3873` | Emits `Kind::Custom("conductor.circuit_breaker")` | C.17 |
| `orchestrate.rs:3874-3880` | Emits `WatcherAlert` execution event | C.13 |
| `orchestrate.rs:3884-3892` | `ensure_dispatch_allowed` guard | A.12, C.08 |
| `orchestrate.rs:3885,3897` | `is_broken` call sites | C.08 |
| `orchestrate.rs:3897-3910` | `run_conductor_check` | C.13 |
| `orchestrate.rs:3897-3900` | `is_broken` short-circuit to `ConductorDecision::Continue` | C.08 |
| `orchestrate.rs:3910` | `self.conductor.evaluate(&signals, &ctx)` | A.12, F.01 |
| `orchestrate.rs:3945,4157` | `supervisor.count()` accounting (broken) | E.14 |
| `orchestrate.rs:4237-4249` | `record_conductor_intervention` call | F.18 |
| `orchestrate.rs:4749,4841` | `is_cancelled()` observation points | E.12 |
| `orchestrate.rs:5084-5148` | Shutdown path (4 phases) | E.12 |
| `orchestrate.rs:5104` | `self.cancel.cancel()` phase 1 | E.12 |
| `orchestrate.rs:5106-5110` | `tokio::time::timeout(SHUTDOWN_DRAIN_GRACE_SECS, run)` phase 2 | E.12 |
| `orchestrate.rs:5129,5140` | `self.save_state_to(&snapshot_path)` phase 3 | E.12 |
| `orchestrate.rs:5134,5145` | `self.flush_logs()` phase 4 | E.12 |
| `orchestrate.rs:5137-5138` | `RunExit::SignalTimedOut → self.force_shutdown()` | E.12 |
| `orchestrate.rs:5682-5707` | Phase-change emission (uses `serde_json::json!`, not `PhaseTransition::new`) | E.09 |
| `orchestrate.rs:5682-5684` | Ad-hoc JSON payload for `EventKind::PhaseTransition` | E.09 |
| `orchestrate.rs:5704-5707` | Same JSON-payload pattern | E.09 |
| `orchestrate.rs:6039-6298` | Per-task retry decision flow (ConductorBandit threaded) | F.21 |
| `orchestrate.rs:6089-6090,6193-6194,6229-6230` | `retry_conductor.record_outcome` calls | F.21 |
| `orchestrate.rs:6210` | `retry_conductor.select_action(&state)` | F.21 |
| `orchestrate.rs:6236` | Continue branch | F.21 |
| `orchestrate.rs:6250` | InjectHint branch | F.21 |
| `orchestrate.rs:6262` | SwitchModel branch | F.21 |
| `orchestrate.rs:6275` | Restart branch | F.21 |
| `orchestrate.rs:6282` | Abort branch | F.21 |
| `orchestrate.rs:6816-6842` | `DiagnosisEngine::default().diagnose(&chain)` retry-error classification | D.08 |
| `orchestrate.rs:6919-6921` | `persist_retry_conductor()` helper | F.21 |
| `orchestrate.rs:7409` | `self.emit_efficiency_event(...)` | F.17 |
| `orchestrate.rs:7692` | `emit_failure_efficiency_event` | F.17 |
| `orchestrate.rs:9424-9430` | Phase-change JSON emit | E.09 |
| `orchestrate.rs:9636` | `dispatch_agent_with` invocation | C.08 |
| `orchestrate.rs:9766-9767` | `conductor.decide(&signals, &Context::now()); conductor.routing_bias()` | A.10, A.12 |
| `orchestrate.rs:10590,10701` | `RunnerAnomalyDetector` construction | F.16 |
| `orchestrate.rs:10775` | `conductor.ghost_turn` signal emission | D.18 |
| `orchestrate.rs:11028` | `Custom("conductor.agent_output")` pushed to signal stream | F.02 |
| `orchestrate.rs:14097-14141` | `dispatch_refuses_tripped_circuit_breaker_before_launch` integration test | A.12 |
| `orchestrate.rs:14098-14148` | Same test (alternate line range cited) | C.08 |
| `orchestrate.rs:15042,15049` | `save_snapshot_atomic` + rename-failure tests | E.13 |
| `main.rs:3635-3636,3872-3873` | Static `ProviderConfig` defaults wired | E.10 |
| `main.rs` | CLI entrypoint with `tokio::signal::ctrl_c` | F.08 |
| `daemon.rs` | Daemon entrypoint with signal handling | F.08 |
| `worker/mod.rs` | Worker entrypoint with signal handling | F.08 |

---

## crates/roko-orchestrator/src/

### Executor snapshot (doc 02 persistence)

| File | What | Section |
|------|------|---------|
| `executor/snapshot.rs:24-37` | `ExecutorSnapshot` — no `failure_records` field | C.09 |

---

## crates/roko-compose/src/

### Prompt composition (doc 12)

| File | What | Section |
|------|------|---------|
| `prompt.rs:77-260` | `ContextAssembler` auction surface (VCG) | E.28 |
| `scorer.rs:98-105` | `ActiveInferenceScorer` (prompt-section scorer, not forward predictor) | F.14 |
| `system_prompt_builder.rs:11` | Pheromone / stigmergic guidance comment | E.30 |
| `system_prompt_builder.rs` | `SystemPromptBuilder` (does not implement three-way partition) | E.28 |

---

## Missing / Absent (grep negatives)

Every identifier below returns **zero matches** via `rg '...' crates/`.
Each is a doc claim with no backing Rust code.

### Cognitive signals (doc 09)

| Identifier | Doc claim | Section |
|------------|-----------|---------|
| `CognitiveSignal` | Doc 09 §Definition enum | B.19 |
| `cognitive_signal` | Doc 09 signal stream | B.19 |
| `cognitive.pause` | Doc 09 signal kind | B.19 |
| `cognitive.escalate` | Doc 09 signal kind | B.19 |
| `Reprioritize`, `InjectContext`, `Cooldown`, `Explore` (as cognitive variants) | Doc 09 variants | B.19 |

### Scheduler trait (doc 00)

| Identifier | Doc claim | Section |
|------------|-----------|---------|
| `pub trait Scheduler` | Doc 00 L4 row "Router, Scheduler" | A.01 |

### Conductor output signal drift (doc 00)

| Identifier | Doc claim | Section |
|------------|-----------|---------|
| `conductor.intervention` (exact literal) | Doc 00 §Signal Flow | A.11, C.17 |

### Watcher composition + isolation forest (doc 01)

| Identifier | Doc claim | Section |
|------------|-----------|---------|
| `CompositePattern` / `PatternStage` | Doc 01 advanced composition (NFA) | B.17 |
| `Contiguity { Strict, Relaxed, NonDeterministic }` | Doc 01 CEP | B.17 |
| `Quantifier { Exactly, AtLeast, Between }` | Doc 01 CEP | B.17 |
| `WatcherFamily` / `WATCHER_FAMILIES` | Doc 01 multi-watcher correlation | B.17 |
| `BayesianFusionPolicy` / `DempsterShafer` | Doc 01 fusion | B.17 |
| `OnlineIsolationForest` / `IsolationTree` / `IsolationNode` | Doc 01 streaming anomaly | B.18 |
| `CusumDetector` | Doc 01 change-point detection | B.18 |
| `TraceAegis` | Doc 01 behavioral rules | B.18 |

### ConductorDecision persistence (doc 02)

| Identifier | Doc claim | Section |
|------------|-----------|---------|
| `CircuitBreaker` field on `ExecutorSnapshot` | Doc 02:294-304 "breaker survives crashes" | C.09 |
| `failure_records` on `ExecutorSnapshot` | Doc 02 persistence | C.09 |
| `CircuitBreaker::to_snapshot` / `from_snapshot` | Doc 02 persistence helpers | C.09 |

### Cooldown / debounce (doc 03)

| Identifier | Doc claim | Section |
|------------|-----------|---------|
| `cooldown` / `debounce` / `last_fired` (in conductor) | Doc 03:297-313 120 s per-plan-per-watcher | C.16 |

### Doc 14 catalog drifts

| Identifier | Doc claim | Section |
|------------|-----------|---------|
| `TomlParsing` (as `ErrorCategory` variant) | Doc 14 issue #5, #13 | D.17 |
| `RetryWithFix` (as `SuggestedIntervention` variant) | Doc 14 issues #5, #13, #14 | D.17 |
| `ImportNotFound` (as `ErrorCategory` variant) | Doc 14 issue #14 | D.17 |
| `error[E0433]` / `error[E0063]` in `built_in_patterns` | Doc 14 issue #14, Doc 4 examples | D.05, D.17 |
| `clippy::` exact needle | Doc 4 pattern example | D.05 |
| `panicked at` exact needle | Doc 4 pattern example | D.05 |

### Health monitor wiring (doc 06)

| Identifier | Doc claim | Section |
|------------|-----------|---------|
| `HealthMonitor` caller in `crates/roko-cli/` | Doc 06 "orchestrator constructs snapshot every 10s" | E.05 |
| `SystemSnapshot` constructor in orchestrator | Doc 06 §Snapshot Collection | E.05 |
| `check_terminal_liveness`/`check_golem_status`/`check_spec_drift`/`check_coverage_trend` callers outside health module | Doc 06 | E.05 |
| `DiskPressure` / `check_disk_pressure` | Doc 14 #10 resource management | D.19 |
| `DiskBudget` | Doc 14 #10 | D.19 |
| `VSM` / `System 3` / `Beer` | Doc 06 VSM mapping | E.07 |

### StuckDetector / MetaCognitionHook wiring (doc 05)

| Identifier | Doc claim | Section |
|------------|-----------|---------|
| `StuckDetector` / `MetaCognitionHook` / `check_stuck` / `meta_cognition` in `roko-cli` | Doc 5 theta-frequency periodic self-assessment | D.11 |
| `ActivityEntry` construction outside the stuck module | Doc 5 input type | D.11 |

### Adaptive timeout consumer (doc 10)

| Identifier | Doc claim | Section |
|------------|-----------|---------|
| `adaptive_timeout_ms` caller in dispatcher | Doc 10 §Per-Phase Adaptive Timeouts | E.10 |

### ProcessSupervisor attempt tracking (doc 13)

| Identifier | Doc claim | Section |
|------------|-----------|---------|
| `attempt_id` on `ProcessHandle` | Doc 13 §Attempt Tracking | E.15 |
| `spawn_backoff` table | Doc 13 exponential backoff | E.15 |
| `ProcessEntry { pid, parent_pid, plan_id, task_id, attempt_id, ... }` | Doc 13 lines 102-118 | E.15 |

### Resource limits / cgroups (doc 13)

| Identifier | Doc claim | Section |
|------------|-----------|---------|
| `cgroup` / `cpu.max` / `memory.max` | Doc 13 resource limits | E.20 |
| `SIGSTOP` / `SIGCONT` throttle | Doc 13 macOS fallback | E.20 |
| Linux `/proc/{pid}/task/*/children` fallback | Doc 13 discovery | E.19 |

### Yerkes-Dodson / pressure (doc 12)

| Identifier | Doc claim | Section |
|------------|-----------|---------|
| `Yerkes` / `YerkesDodson` | Doc 12 curve theory | E.22 |
| `YerkesDodsonEstimator` | Doc 12 binned estimator | E.24 |
| `PressureDial` | Doc 12 control surface | E.23 |
| `PressureEnvelope` | Doc 12 §The Pressure Envelope | E.23 |
| `pressure_dial` | Doc 12 | E.23 |
| `ModelPressureProfile` | Doc 12 per-agent calibration | E.24 |
| `PressureBandit` / `PressureArm` / `PressureConfig` / `PRESSURE_CONFIGS` | Doc 12 Thompson sampling | E.25 |
| `pressure_index` helper | Doc 12 multi-dim scalar | E.26 |
| `FlowDetector` / `FlowState` / `TurnMetrics` / `is_productive` | Doc 12 flow detection | E.27 |
| Cognitive load `Intrinsic/Extraneous/Germane` tags on `PromptSection` | Doc 12 cognitive load mapping | E.28 |
| `CooperationMetric` / `cooperation_metrics` | Doc 12 cooperation signals | E.29 |

### Theory frontier (doc 07, 08)

| Identifier | Doc claim | Section |
|------------|-----------|---------|
| `struct Observe` / `struct Orient` / `struct Decide` / `struct Act` / `enum OodaPhase` | Doc 07 OODA (narrative only) | F.01 |
| `LivenessMonitor` | Doc 07 heartbeat proposal | F.05 |
| `ImplicitGuidance` / `ImplicitRule` / `IGaC` | Doc 07 pre-compiled rules | F.06 |
| `ParameterCascade` | Doc 07 nested OODA | F.07 |
| `DeltaParameters` / `ThetaParameters` / `GammaParameters` | Doc 07 timescale structs | F.07 |
| `delta_loop` / `theta_loop` / `gamma_loop` | Doc 07 timescale loops | F.07 |
| `algedonic` / `AlgedonicSignal` / `AlgedonicChannel` | Doc 07 priority interrupts | F.08 |
| `GoodRegulator` / `ConantAshby` | Doc 08 theorem framing | F.09 |

### Good Regulator / self-model (doc 08)

| Identifier | Doc claim | Section |
|------------|-----------|---------|
| `SelfModelAccuracy` | Doc 08 §Self-Model Accuracy Metrics | F.11 |
| `BrierScore` / `BrierScoreTracker` | Doc 08 Brier tracker | F.11 |
| `CalibrationBin` | Doc 08 calibration bins | F.11 |
| `intervention_effectiveness` / `diagnosis_accuracy` / `gate_pass_brier_score` | Doc 08 metric fields | F.11 |
| `ThresholdLearner` / `ThresholdPosterior` / `ThresholdDirection` | Doc 08 Bayesian update | F.12 |
| `ScalarKalman` / `kalman` / `KalmanFilter` / `process_noise` / `measurement_noise` | Doc 08 filter | F.13 |
| `PrecisionWeightedUpdater` | Doc 08 active inference | F.14 |
| `ForwardPredictor` / `predict_pass_probability` | Doc 08 forward model | F.14 |
| `InternalModelPrinciple` / `active_inference` (conductor-side) | Doc 08 IMP | F.14 |

### Conductor federation + self-healing + triple-loop (doc 15)

| Identifier | Doc claim | Section |
|------------|-----------|---------|
| `LearnedConductorPolicy` | Doc 15 wrapper | F.22 |
| `select_with_confidence` / `total_observations` | Doc 15 bandit methods | F.22 |
| `ConductorLevel` / `ConductorScope` | Doc 15 federation trait + enum | F.24 |
| `conductor.plan.` / `conductor.fleet.` signal tags | Doc 15 federation signals | F.24 |
| `SelfHealingConductor` | Doc 15 | F.25 |
| `SelfRepairAction` | Doc 15 repair enum | F.25 |
| `MicroReboot` / `RecoveryOriented` / `self_assess` | Doc 15 | F.25 |
| `single_loop` / `double_loop` / `triple_loop` / `learning_rate_meta` | Doc 15 triple-loop | F.26 |
| `pattern_library` / `pattern_store` | Doc 11 Loop 3 pattern persistence | F.27 |
