# A — Architecture (Doc 07/00)

Parity of `docs/07-conductor/00-conductor-architecture.md` against the shipping
`roko-conductor` crate and its orchestrator wiring. Doc 00 positions the
Conductor as the Harness (L3) layer, frames it as a composite `Policy` that
aggregates ten watchers through an intervention policy plus a circuit
breaker, and enumerates seven subsystems (watchers, circuit breaker,
intervention policy, diagnosis, stuck detection, health, state machine).
This section walks that positioning top-to-bottom and verifies the module
layout, type names, constants, and cross-crate integration points.

Generated 2026-04-16.

---

## A.01 — Five-layer stack placement at L3 (Doc 00 §"Position in the Five-Layer Architecture")

**Status**: PARTIAL
**Severity**: LOW
**Doc claim**: The Conductor sits at Layer 3 ("Harness") alongside gates, with `Gate` and `Policy` as the key traits for that layer; the doc also tables out L0 Runtime/`Substrate`, L1 Framework, L2 Scaffold/`Composer`, L3 Harness/`Gate,Policy`, L4 Orchestration/`Router,Scheduler`.
**Reality**: The six canonical verb traits in `roko-core` are `Substrate`, `Scorer`, `Gate`, `Router`, `Composer`, `Policy` — enumerated at `traits.rs:15, 65, 88, 110, 135, 158`. There is no `Scheduler` trait (`Grep 'pub trait Scheduler'` on `crates/` returns zero matches), and `Scorer` is absent from the L3 row in the doc even though `Gate` and `Router` both depend on scoring for their canonical composition. The Conductor's placement at L3 via `impl Policy for Conductor` at `conductor.rs:226-255` is real, but the table's "Router, Scheduler" label for L4 is doc-only terminology and the L1 "Framework" row has no trait at all. The placement claim holds; the table column "Key Traits" is partially fictional.
**Fix sketch**: Either (a) promote the L4 row to `Router` only (drop Scheduler) and add `Scorer` to L3, or (b) annotate the traits column as "primary trait surface" rather than exhaustive. `docs/00-architecture/` already uses the six-trait framing; mirror that.

---

## A.02 — Synapse model: Conductor is a composite `Policy` (Doc 00 §"Synapse Architecture Placement")

**Status**: DONE
**Severity**: —
**Doc claim**: Roko's kernel defines one noun (`Signal`) and six verb traits (`Substrate`, `Scorer`, `Gate`, `Router`, `Composer`, `Policy`); every watcher implements `Policy`, the Conductor also implements `Policy`, delegates to inner watchers, and aggregates output through an intervention policy.
**Reality**: Confirmed. `Policy` trait at `crates/roko-core/src/traits.rs:166-168` defines `fn decide(&self, stream: &[Engram], ctx: &Context) -> Vec<Engram>`. The Conductor struct at `conductor.rs:53-62` holds `watchers: Vec<Box<dyn Policy>>`, `policy: Box<dyn InterventionPolicy>`, `circuit_breaker: CircuitBreaker`, and `routing_bias: Mutex<RoutingBias>`. The `impl Policy for Conductor` at `:226-255` calls `collect_watcher_outputs` over the ten boxed watchers, then feeds results through `self.policy.evaluate(...)`. The doc's Rust snippet at Doc 00 `:59-76` is directionally correct — the shipping struct simply adds a `routing_bias` field the doc does not show (see A.10). The composite-`Policy` story is accurate.

---

## A.03 — Ten watchers wired into `Conductor::new()` (Doc 00 §"1. Watcher Ensemble")

**Status**: DONE
**Severity**: —
**Doc claim**: The Conductor comprises a "watcher ensemble" of ten watchers — Ghost Turn, Compile Fail Repeat, Cost Overrun, Iteration Loop, Review Loop, Spec Drift, Stuck Pattern, Test Failure Budget, Time Overrun, Context Window Pressure — each implementing `Policy` and each in its own module under `watchers/`.
**Reality**: All ten ship. `crates/roko-conductor/src/watchers/mod.rs:8-17` declares modules `compile_fail_repeat`, `context_window_pressure`, `cost_overrun`, `ghost_turn`, `iteration_loop`, `review_loop`, `spec_drift`, `stuck_pattern`, `test_failure_budget`, `time_overrun` — the exact ten listed. Re-exports at `:19-28` expose their named types. `Conductor::new()` at `conductor.rs:82-102` constructs `Vec<Box<dyn Policy>>` containing `GhostTurnWatcher::default()`, `ReviewLoopWatcher::default()`, `IterationLoopWatcher::default()`, `TestFailureBudgetWatcher::default()`, `CompileFailRepeatWatcher::default()`, `ContextWindowPressureWatcher::default()`, `SpecDriftWatcher::default()`, `CostOverrunWatcher::default()`, `TimeOverrunWatcher::new()`, `StuckPatternWatcher::default()` — ten entries. Assertion `assert_eq!(c.watchers.len(), 10)` at `:494` pins the count. Each watcher file implements `Policy` (e.g. `context_window_pressure.rs:52` `impl Policy for ContextWindowPressureWatcher`).

---

## A.04 — Circuit breaker uses `DashMap` + `MAX_PLAN_FAILURES=2` (Doc 00 §"2. Circuit Breaker")

**Status**: DONE
**Severity**: —
**Doc claim**: Per-plan failure budget, `DashMap`-backed, default `MAX_PLAN_FAILURES = 2`; struct shape `CircuitBreaker { failures: DashMap<String, FailureRecord> }`; after two failures the plan is "permanently tripped — no further retries".
**Reality**: `pub const MAX_PLAN_FAILURES: u32 = 2` at `circuit_breaker.rs:11`. `CircuitBreaker` at `:28-33` carries `max_failures: u32` and `records: DashMap<String, FailureRecord>` — the doc's field name is `failures` but the shipping field is `records` (minor drift). `FailureRecord` at `:14-22` stores `count`, `last_failure_ms`, `reasons`. `record_failure` at `:55-63` increments count, writes reason + timestamp, returns `count >= self.max_failures`. `is_tripped` at `:67-71` and the alias `is_broken` at `:78-80` gate downstream dispatch. The "permanently tripped" property is exercised in test `tripped_stays_tripped_on_more_failures` at `:225-234`. `Cargo.toml:19` pulls in `dashmap = { workspace = true }`. Doc text is accurate; only the field name is stylistic drift.

---

## A.05 — Intervention policy maps severity to decision (Doc 00 §"3. Intervention Policy")

**Status**: DONE
**Severity**: —
**Doc claim**: Severity maps `Info → Continue`, `Warning → Restart`, `Critical → Fail`; the default is `WorstSeverityPolicy` where the highest severity among all watcher outputs determines the decision.
**Reality**: `Severity` enum at `interventions.rs:22-31` has exactly `Info = 0`, `Warning = 1`, `Critical = 2` with `#[derive(...PartialOrd, Ord...)]`. `Severity::to_decision` at `:33-45` maps `Info → ConductorDecision::cont()`, `Warning → ConductorDecision::restart(...)`, `Critical → ConductorDecision::fail(...)`. The `InterventionPolicy` trait at `:99-105` has `evaluate(&self, outputs: &[WatcherOutput], ctx: &Context) -> ConductorDecision`. `WorstSeverityPolicy` at `:108-121` uses `max_by_key(|o| o.severity)` to pick the highest. `Conductor::new()` wires `policy: Box::new(WorstSeverityPolicy)` at `conductor.rs:98`. Tests `worst_severity_policy_critical_wins` at `interventions.rs:200-209` and `multiple_watchers_worst_wins` at `conductor.rs:460-489` pin the behavior. The doc matches the code exactly.

---

## A.06 — Diagnosis engine has thirty-four patterns and twenty categories (Doc 00 §"4. Diagnosis Engine")

**Status**: PARTIAL
**Severity**: LOW
**Doc claim**: "Thirty-four built-in error patterns covering twenty error categories"; the `SuggestedIntervention` enum has nine variants (`RetryWithContext`, `AutoFix`, `RestartAgent`, `AbortPlan`, `BackoffRetry`, `MergeResolution`, `ReduceContext`, `SwitchModel`, `WarnAndContinue`).
**Reality**: The `SuggestedIntervention` enum at `diagnosis.rs:75-94` lists exactly the nine variants the doc enumerates. `ErrorCategory` at `:26-67` has **twenty** variants (`CompileError, TestFailure, ClippyWarning, GitConflict, DependencyError, TypeMismatch, BorrowCheckerError, LifetimeError, ImportError, MissingFile, PermissionDenied, NetworkError, TimeoutError, OomError, DiskFull, LlmRateLimit, LlmContextOverflow, LlmRefusal, ProcessCrash, LoopDetected`), matching the doc. The pattern count is **thirty-four** per `built_in_patterns()` at `:278-531` — verified by `awk '/^fn built_in_patterns/,/^}/' ... | grep -c "ErrorPattern {"` returning 34. The doc number is correct; the minor drift is only that the doc comment inside the function reads "The default set of 20+ error patterns" at `:276`, which understates.
**Fix sketch**: Update the in-code comment at `diagnosis.rs:276` from "20+" to "34" so the docstring tracks the actual count.

---

## A.07 — Stuck detection: six heuristics plus `MetaCognitionHook` (Doc 00 §"5. Stuck Detection")

**Status**: DONE
**Severity**: —
**Doc claim**: Six stuck heuristics (`OutputLoop`, `NoProgress`, `GateLoop`, `CompileLoop`, `EmptyOutput`, `ExcessiveRetries`); `StuckDetector` operates at configurable thresholds; `MetaCognitionHook` wraps it for periodic self-assessment at Theta frequency.
**Reality**: `StuckKind` enum at `stuck_detection.rs:34-47` has exactly the six variants the doc names. `StuckThresholds` at `:107-132` defaults to `output_loop_count=4, no_progress_ms=300_000, gate_loop_count=3, compile_loop_count=3, empty_output_count=3, excessive_retry_count=6` — six configurable knobs, one per heuristic. `StuckDetector::check_stuck` at `:178-204` runs the six checks in priority order (`check_excessive_retries`, `check_output_loop`, `check_gate_loop`, `check_compile_loop`, `check_empty_output`, `check_no_progress`). `MetaCognitionHook` at `:544-591` wraps a `StuckDetector`, exposes `frequency() -> OperatingFrequency::Theta` at `:582-584`, and `assess()` at `:587-590` delegates to `detector.meta_cognition()`. The doc lines up with the code.

---

## A.08 — Health monitor has four checks, but one is named `golem_status` (Doc 00 §"6. Health Monitor")

**Status**: PARTIAL
**Severity**: MEDIUM
**Doc claim**: Four system-level health checks — `terminal_liveness`, `agent_status`, `spec_drift`, `coverage_trend` — producing a `HealthStatus` of Healthy / Degraded / Critical; operates on `SystemSnapshot`.
**Reality**: Four built-in checks ship but the second is still named `golem_status`, not `agent_status`. `HealthMonitor::new()` at `health.rs:148-172` registers `NamedCheck { name: "terminal_liveness", ... }`, `NamedCheck { name: "golem_status", ... }` (`:159`), `NamedCheck { name: "spec_drift", ... }` (`:163`), `NamedCheck { name: "coverage_trend", ... }` (`:167`). The `check_golem_status` function at `:258-270` monitors chain-connection state (`snapshot.chain_expected`, `snapshot.chain_connected`), not agent-process liveness as the doc "agent_status" label implies. `HealthStatus` enum at `:26-33` lists `Healthy = 0`, `Degraded = 1`, `Critical = 2` — matches. `SystemSnapshot` struct at `:92-114` is present. This is post-rename drift: the `golem` naming is a holdover the project has otherwise stamped out (`tmp/docs-parity/06/F-status-frontier.md` F.05 documents the `roko-golem` dissolution). `Grep 'HealthMonitor'` on `crates/roko-cli/` returns zero matches — the health monitor is built but not consumed by the orchestrator runtime, only tested inside the crate.
**Fix sketch**: Rename `check_golem_status` → `check_chain_status` (or `check_agent_status`, keeping the doc's label but clarifying the snapshot field it actually reads) at `health.rs:159, 258` and update the `NamedCheck` registration string. Separately, decide whether `HealthMonitor` should be consumed from `orchestrate.rs` alongside `conductor.check_all`; if not, delete the stub or annotate doc 00 to flag health monitor as not-yet-wired.

---

## A.09 — Phase timeout matrix by complexity band (Doc 00 §"7. State Machine")

**Status**: PARTIAL
**Severity**: LOW
**Doc claim**: Phase timeout table covers `Implementing` (Complex 600s / Standard 300s / Fast 120s), `Gating 300s`, `Reviewing 300s`, `Merging 60s`. `PhaseTransition` records carry `plan_id`, source/target phase, timestamp, reason.
**Reality**: All four rows verified and then some. `phase_timeout()` at `state_machine.rs:37-55` returns `Some(600/120/300)` for `Implementing` by `TaskComplexityBand` (`:39-44`), `Some(300)` for `Gating` (`:45`), `Some(300)` for `Reviewing` (`:47`), `Some(60)` for `Merging` (`:48`). Additional phases the doc table omits: `Verifying 300s` (`:46, TIMEOUT_VERIFYING = 300`), `Enriching 120s` (`:49, TIMEOUT_ENRICHING = 120`), `AutoFixing 300s` (`:50, TIMEOUT_AUTO_FIXING = 300`), `DocRevision 120s` (`:51, TIMEOUT_DOC_REVISION = 120`). `PhaseTransition` at `:61-73` has `plan_id: String`, `from: PhaseKind`, `to: PhaseKind`, `at_ms: i64`, `reason: Option<String>` — all four listed fields present. The shipping code has **eight** timed phases to the doc's four; doc is a subset, not wrong but incomplete.
**Fix sketch**: Extend the Doc 00 §"7" table to include `Verifying`, `Enriching`, `AutoFixing`, `DocRevision` rows, since all four have defined timeouts and the doc's table already reads like a canonical matrix.

---

## A.10 — Undocumented `RoutingBias` surface on Conductor (Doc 00 §"Synapse Architecture Placement", §"Core Components")

**Status**: PARTIAL
**Severity**: MEDIUM
**Doc claim**: The doc's struct snippet for `Conductor` at Doc 00 `:61-65` lists only `watchers`, `policy`, `circuit_breaker` and lists seven subsystems total (watchers, circuit breaker, intervention policy, diagnosis, stuck detection, health, state machine). The doc does not mention routing bias or any integration with the cascade router.
**Reality**: The shipping `Conductor` struct has a fourth field `routing_bias: Mutex<RoutingBias>` at `conductor.rs:60-61`. `RoutingBias` struct at `:26-35` carries `deprioritize: Vec<String>`, `prefer_cheaper: bool`, `reason: String`. The public API `routing_bias()` at `:145-148` returns a snapshot. `update_routing_bias` at `:258-269` is called inside every `evaluate()` (`:170`) and every `decide()` (`:230`), with `derive_routing_bias` at `:272-315` keying off "load pressure" (cost/context/time overrun) and "recent failure" (ghost turn, review/iteration loop, test/compile fail, stuck-pattern, spec-drift) watcher outputs. This bias is consumed at `orchestrate.rs:1787-1795` by `cascade_routing_bias_from_conductor` and `:9766-9767` (`self.conductor.decide(&signals, &Context::now()); self.conductor.routing_bias()`). Routing bias is a real eighth subsystem that the architecture doc silently omits.
**Fix sketch**: Add an "8. Routing Bias" subsection to Doc 00 §"Core Components" describing the `RoutingBias { deprioritize, prefer_cheaper, reason }` contract, its watcher inputs, and its cascade-router consumer at `orchestrate.rs:1787-1795`. Also add `routing_bias: Mutex<RoutingBias>` to the struct snippet at Doc 00 `:61-65`.

---

## A.11 — Signal flow kinds (Doc 00 §"Signal Flow")

**Status**: PARTIAL
**Severity**: LOW
**Doc claim**: Input signals consumed: `TokenUsage`, `GateVerdict`, `AgentOutput`, `PlanPhase`, `Metric (name=spec_drift)`, `Custom("conductor.agent_output")`. Output signal: `Custom("conductor.intervention")`.
**Reality**: The input-kind list matches what the watchers actually read — `TokenUsage` is used by `ContextWindowPressureWatcher` (`context_window_pressure.rs:55`), `GateVerdict` by the iteration-loop watcher, `AgentOutput` by ghost-turn and stuck-pattern, `PlanPhase` by review-loop + `extract_plan_id` at `conductor.rs:191-198`, the spec-drift `Metric` by `SpecDriftWatcher`, and `Custom("conductor.agent_output")` by `TimeOverrunWatcher` (`TASK_OUTPUT_KIND` const at `watchers/time_overrun.rs:13`). But the output-kind claim is wrong: the conductor actually emits two kinds — `Custom("conductor:alert:<watcher>")` via `outputs_to_signals` at `interventions.rs:123-144, 133-135` (note the colon-separated suffix, not "conductor.intervention"), and `Custom("conductor.decision")` via the `Policy` impl at `conductor.rs:240-245`. Additionally the ghost-turn watcher emits `Custom("conductor.ghost_turn")` as a tick marker (`TURN_SIGNAL_KIND = "conductor.ghost_turn"` at `ghost_turn.rs:17`). `Grep 'conductor.intervention'` on `crates/` returns zero matches — the doc's exact string does not exist anywhere.
**Fix sketch**: Update Doc 00 §"Signal Flow" output table to: `Custom("conductor:alert:<watcher>")` — any watcher fires with Warning+; `Custom("conductor.decision")` — terminal decision emitted by the `Policy` impl; `Custom("conductor.ghost_turn")` — per-turn tick.

---

## A.12 — Orchestrator integration points (Doc 00 §"Evaluation Flow")

**Status**: DONE
**Severity**: —
**Doc claim**: "When the orchestrator calls `conductor.evaluate()`, the following sequence executes: circuit breaker check → run all 10 watchers → apply intervention policy → record failure if Restart/Fail → return decision." Evaluation is stateless from the Conductor's perspective; state lives in the `CircuitBreaker`.
**Reality**: All five evaluation steps present at `conductor.rs:156-187`: (1) circuit-breaker early-return at `:158-166` returns `ConductorDecision::fail("circuit-breaker", FailureKind::MaxIterations)` if `is_tripped(plan_id)`; (2) `collect_watcher_outputs` at `:169` walks all ten watchers; (3) `self.policy.evaluate(&watcher_outputs, ctx)` at `:173`; (4) `circuit_breaker.record_failure` at `:178-182` on `ConductorDecision::Fail`; (5) `decision` returned at `:186`. Orchestrator wiring: `use roko_conductor::{Conductor, ConductorDecision}` at `orchestrate.rs:37`, diagnostic engine import at `:36`, three `Arc::new(Conductor::new())` construction sites at `:3258, 3377, 3500` (per PlanRunner build path), `self.conductor.check_all(&signals)` tick at `:1474` (bus-scan path), `self.conductor.evaluate(&signals, &ctx)` at `:3910` inside `run_conductor_check`, and `self.conductor.decide(...)` at `:9766` for routing-bias refresh. `handle_tripped_circuit_breaker` at `:3844-3881` pauses the plan and emits both a diagnosis-tagged payload and a `Custom("conductor.circuit_breaker")` signal. The refuse-before-dispatch guard `ensure_dispatch_allowed` at `:3884-3892` is exercised by the integration test `dispatch_refuses_tripped_circuit_breaker_before_launch` at `:14097-14141`. The flow is stateless from the Conductor's own perspective; mutable state lives only in `CircuitBreaker` (`DashMap`) and `routing_bias: Mutex<RoutingBias>` (see A.10). Doc matches the shipping evaluation flow.

---

## A.13 — Module organization and re-exports (Doc 00 §"File Reference")

**Status**: DONE
**Severity**: —
**Doc claim**: File reference table lists `lib.rs`, `conductor.rs`, `circuit_breaker.rs`, `interventions.rs`, `diagnosis.rs`, `health.rs`, `state_machine.rs`, `stuck_detection.rs`, and `watchers/` — eight top-level modules plus the watchers directory. `lib.rs` re-exports core types for convenience.
**Reality**: `crates/roko-conductor/src/lib.rs:24-31` declares exactly those eight modules: `circuit_breaker, conductor, diagnosis, health, interventions, state_machine, stuck_detection, watchers`. Re-exports at `lib.rs:34-40` expose `roko_core::{ConductorDecision, PhaseKind, PlanPhase}` plus local `CircuitBreaker`, `{Conductor, RoutingBias}`, `{InterventionPolicy, Severity, WatcherOutput, WorstSeverityPolicy}`, `{PhaseTransition, phase_timeout}`. Cargo deps at `Cargo.toml:13-21` pull `roko-core`, `roko-learn`, `serde`, `serde_json`, `parking_lot`, `dashmap`, `tracing`, `chrono`. The `roko-learn` dep (undocumented in the architecture doc) exists because `ContextWindowPressureWatcher` consumes `roko_learn::efficiency::AgentEfficiencyEvent` at `watchers/context_window_pressure.rs:7` — a minor doc omission but not a drift. File counts: `conductor.rs` is 555 LOC (matches the doc's positioning), `diagnosis.rs` 899 LOC, `health.rs` 568 LOC, `stuck_detection.rs` 1,085 LOC, `interventions.rs` 233 LOC, `state_machine.rs` 218 LOC, `circuit_breaker.rs` 258 LOC, `lib.rs` 40 LOC, and 10 watcher files plus `watchers/mod.rs` totaling ~2,254 LOC — all shipped. The module organization in the doc is accurate.

---

## Section Summary

| Status | Count |
|--------|-------|
| DONE | 7 (A.02 composite Policy, A.03 ten watchers wired, A.04 DashMap circuit breaker, A.05 severity→decision mapping, A.07 six stuck heuristics + Theta hook, A.12 evaluation flow end-to-end, A.13 module layout) |
| PARTIAL | 6 (A.01 five-layer trait table drift, A.06 "20+" docstring understates 34 patterns, A.08 `golem_status` naming holdover, A.09 doc table omits 4 additional timed phases, A.10 undocumented `RoutingBias` subsystem, A.11 wrong output signal kind name) |
| NOT DONE | 0 |

Doc 00 holds up on every load-bearing architectural claim: the Conductor
really is a composite `Policy` with exactly ten `Box<dyn Policy>` watchers
(A.03), the circuit breaker really uses a `DashMap` with a default budget of
two failures (A.04), the severity→decision mapping is exact (A.05), the six
stuck heuristics plus `MetaCognitionHook` at Theta frequency ship (A.07),
and the orchestrator really calls `conductor.evaluate` with the documented
five-step flow (A.12). The module layout and re-exports in `lib.rs` match
the doc's File Reference table (A.13). Six items are PARTIAL but
none are HIGH severity. The MEDIUM item worth fixing first is A.08 — the
`golem_status` check name is a rename holdover that contradicts the
project's finished `roko-golem` dissolution (cross-ref tmp/docs-parity/06
F.05) and its snapshot field actually reads `chain_connected`, so the name
is misleading on both axes. The second MEDIUM is A.10 — `RoutingBias` is a
real public field on `Conductor` consumed by `orchestrate.rs` via
`cascade_routing_bias_from_conductor`, but the architecture doc never
mentions it, which leaves readers of Doc 00 without a map to the actual
routing-feedback surface. The remaining LOW-severity items (A.01 trait
table drift, A.06 docstring understating pattern count, A.09 timeout-table
subset, A.11 wrong emitted-signal kind name) are documentation-only
polishing. No NOT-DONE items: every subsystem the doc describes ships in
`crates/roko-conductor/`.

## Agent Execution Notes

### A.08 / A.10 / A.11 — Architecture Contract Cleanup

Best use of this section in batch `07`:

1. complete the post-dissolution `golem_status` -> `chain_status` cleanup when the health code is touched,
2. document `RoutingBias` as a real conductor surface,
3. make the emitted signal-kind story match the actual runtime kinds.

Do not widen this section into a new architecture redesign. The core conductor architecture already ships.

### A.01 / A.06 / A.09 — Light Drift, Not Runtime Blockers

These are useful cleanup targets, but they should usually follow the runtime batches rather than block them.

Acceptance criteria for this section:

- the architecture docs map cleanly to the live conductor fields and emitted kinds,
- post-dissolution naming drift is reduced,
- later agents do not need to infer hidden conductor surfaces from code alone.
