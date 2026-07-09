# C — Conductor Decision Space (Docs 07/02 + 07/03)

Docs-vs-code parity for the circuit breaker (Doc 02) and graduated
interventions (Doc 03). Focus: per-plan failure budget, trip semantics,
`DashMap` concurrency, three-state pattern, severity → decision mapping,
and the "decide, don't nudge" contract.

Generated: 2026-04-16

---

## C.01 — `MAX_PLAN_FAILURES = 2` constant and per-plan budget (Doc 02 §Implementation, §Why Two Failures)

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 02:2-5 states "A plan can fail a maximum of two times. After that, it requires human attention. This is not configurable. This is law." Doc 02:43 shows `pub const MAX_PLAN_FAILURES: u32 = 2;`. Doc 02:203 ties the constant to production data.
**Reality**: Confirmed. `crates/roko-conductor/src/circuit_breaker.rs:11` declares `pub const MAX_PLAN_FAILURES: u32 = 2;` exactly as the doc shows. The `Default` impl at `circuit_breaker.rs:35-39` constructs `CircuitBreaker::new(MAX_PLAN_FAILURES)`, so instantiation via `CircuitBreaker::default()` — which is how `Conductor::new` at `conductor.rs:99` wires it — uses the 2-failure budget. Unit test `trips_at_max_failures` at `circuit_breaker.rs:146-153` exercises both record calls and asserts trip on the second. Contrary to the doc's "not configurable" line, the struct also exposes `CircuitBreaker::new(max_failures: u32)` at `:44-49`, used in tests (`:147`, `:157`, `:180`, `:190`, `:238`), so the threshold *is* overridable per-instance; in practice nothing in production code instantiates it with a value other than `MAX_PLAN_FAILURES`.

---

## C.02 — `DashMap<String, FailureRecord>` for concurrent sharded storage (Doc 02 §Thread Safety)

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 02:41-53 specifies `failures: DashMap<String, FailureRecord>` with a `FailureRecord { count: u32, ... }` struct, and §Thread Safety (02:55-65) argues for `DashMap` over `Mutex<HashMap>` because plans execute in parallel and should not block on each other's failure records.
**Reality**: Matches. `use dashmap::DashMap;` at `circuit_breaker.rs:7`, field `records: DashMap<String, FailureRecord>` at `:32`, and `FailureRecord` at `:14-22` with `count: u32`, `last_failure_ms: Option<i64>`, and `reasons: Vec<String>`. The doc's struct sketch omits `last_failure_ms` and `reasons` but calls out "additional metadata: timestamps, failure reasons, etc." at `:50-51`, so the drift is minor. Test `concurrent_access_is_safe` at `:206-222` spawns 10 threads × 10 failures into a shared breaker and asserts the total count; this validates the sharded-lock behaviour described at Doc 02:62-65.

---

## C.03 — `PlanCircuitBreaker` name vs. real struct name (Doc 02 §Implementation)

**Status**: PARTIAL
**Severity**: LOW
**Doc claim**: The Doc 02 code snippets (`:45`, `:70`) name the struct `CircuitBreaker`, not `PlanCircuitBreaker`. The parity-check tag in the audit brief mentions a `PlanCircuitBreaker` name — that naming is not in Doc 02 and not in the codebase.
**Reality**: The plan-level struct is plain `CircuitBreaker` at `crates/roko-conductor/src/circuit_breaker.rs:28`. A second, unrelated `CircuitBreaker` struct exists in `crates/roko-core/src/error/retry.rs:139` (a three-state `Closed/Open/HalfOpen` breaker for generic backend calls), and a third, provider-scoped `ProviderHealth` three-state breaker lives at `crates/roko-learn/src/provider_health.rs:82-99`. Three distinct "CircuitBreaker" surfaces coexist in the tree under two different names (`CircuitBreaker`, `ProviderHealth`). Doc 02:330-333 correctly points to both `crates/roko-conductor/src/circuit_breaker.rs` and `crates/roko-learn/src/provider_health.rs` and labels the latter the "Extended 3-state model for provider health", so the doc does not fabricate `PlanCircuitBreaker` — it uses `CircuitBreaker` throughout. The parity concern is the naming collision, not a doc drift.
**Fix sketch**: Optionally rename `roko-conductor::CircuitBreaker` to `PlanCircuitBreaker` to eliminate the collision with `roko-core::error::retry::CircuitBreaker`; update Doc 02 file reference at `:330` accordingly. If kept as-is, consider adding a "Note on naming" callout in Doc 02 explaining that three `CircuitBreaker` types coexist for distinct scopes (plan, backend call, provider).

---

## C.04 — `is_tripped`, `record_failure`, `reset` API (Doc 02 §API)

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 02:69-97 specifies `fn new() -> Self`, `record_failure(&self, plan_id: &str) -> bool` returning "true if now tripped", `is_tripped(&self, plan_id: &str) -> bool`, and `reset(&self, plan_id: &str)`.
**Reality**: All four methods exist at `circuit_breaker.rs:44-95`. `new(max_failures: u32)` at `:44-49`, `record_failure(&self, plan_id: &str, reason: impl Into<String>, now_ms: i64) -> bool` at `:55-63` with the claimed `count >= self.max_failures` return semantics, `is_tripped` at `:67-71`, and `reset` at `:89-91`. Real signatures are richer than the doc snippet: `record_failure` takes a `reason` and `now_ms` so the struct can populate `FailureRecord.last_failure_ms` and `FailureRecord.reasons`, and the code adds `is_broken` (`:78-80`, alias for `is_tripped`), `failure_count` (`:84-86`), `reset_all` (`:94-96`), `get_record` (`:100-102`), `tracked_plans` (`:106-108`), and `max_failures` (`:112-114`). None of these extensions contradicts Doc 02 — they extend it.

---

## C.05 — Two-state model in conductor, three-state in provider-health (Doc 02 §Three-State Model)

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 02:101-141 explicitly flags that the conductor's `CircuitBreaker` is a simplified two-state model (tripped / not tripped), while the full `Closed / Open / HalfOpen` three-state model lives in `roko-learn/src/provider_health.rs`. Rationale at 02:137-141: "a plan that failed twice needs a different approach, not another attempt at the same approach." HalfOpen therefore does not apply to plans.
**Reality**: The conductor breaker is two-state in practice — there is no `CircuitState` enum, `recovery_at` timer, or `HalfOpen` variant anywhere in `crates/roko-conductor/src/circuit_breaker.rs`; `is_tripped` returns `bool` and the struct never transitions "backward" from tripped without a manual `reset()`. The three-state model exists in `crates/roko-learn/src/provider_health.rs:42-50` as `enum CircuitState { Closed, Open, HalfOpen }` with the transition semantics described by Doc 02:111-125 (cooldown expiry → `HalfOpen`, probe success → `Closed`, probe failure → `Open` with reset cooldown). See `ProviderHealth::record_success` at `provider_health.rs:109-116`, `record_failure` at `:119-137`, and `is_available` at `:143-157`. Separately, `crates/roko-core/src/error/retry.rs:139-145` defines *another* three-state `CircuitBreaker` with the same pattern (`BreakerState::Closed/Open/HalfOpen` at `:117-126`, state transitions at `:164-186`). The doc's claim that the three-state model lives "in the provider health tracker" is narrowly true; it just omits the retry-layer breaker in `roko-core`.

---

## C.06 — Error-type-specific cooldowns per Doc 02:148-160

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 02:148-154 table: RateLimit=5s, Timeout=10s, ServerError=30s, AuthFailure=5min, ContentPolicy=5min, ContextOverflow=N/A.
**Reality**: `ProviderHealth::cooldown_ms` at `crates/roko-learn/src/provider_health.rs:160-168` returns 5_000 ms for RateLimit, 10_000 ms for Timeout, 30_000 ms for ServerError, 300_000 ms (5 min) for AuthFailure, and 5_000 ms fallback for all other variants (`ContentPolicy`, `ContextOverflow`, `Unknown`). The doc table correctly states RateLimit=5s, Timeout=10s, ServerError=30s, AuthFailure=5min. The doc's `ContentPolicy=5min` claim is not reflected in code (5 s fallback via `_ =>` branch). `ContextOverflow` is documented as "N/A — not retryable" which is consistent with the 5 s fallback never being reached because `is_available` would be evaluated only after a caller decided to retry; but code does NOT special-case ContextOverflow to permanently open the circuit.
**Fix sketch**: Either (a) add match arms for `ContentPolicy => 300_000` and `ContextOverflow => non-retryable` in `cooldown_ms`, or (b) update Doc 02:153-155 to note that the code applies a 5 s fallback for ContentPolicy and has no explicit "not retryable" treatment of ContextOverflow in the cooldown path.

---

## C.07 — Breaker checked first in `Conductor::evaluate` (Doc 02 §Integration with the Conductor)

**Status**: PARTIAL
**Severity**: LOW
**Doc claim**: Doc 02:169-192 shows `Conductor::evaluate` (a) checks the breaker first and returns `ConductorDecision::Fail { reason: String }`, (b) runs watchers, (c) applies the intervention policy, (d) records failures in the breaker on a `Fail` decision.
**Reality**: The ordering matches exactly. `crates/roko-conductor/src/conductor.rs:156-187` — breaker check at `:158-166`, watchers at `:169`, policy at `:173`, failure record on `Fail` at `:176-184`. Two drifts between doc snippet and real code: (1) the real `ConductorDecision::Fail` variant takes `{ watcher: String, reason: FailureKind }` (see `crates/roko-core/src/conductor.rs:35-41`), so the doc's `Fail { reason: format!(...) }` with a plain `String` is wrong — the implementation passes `roko_core::FailureKind::MaxIterations` at `conductor.rs:162-164`; (2) the doc omits the routing-bias side effect (`self.update_routing_bias(stream, &[])` at `:160` on trip, and `:170` on normal path). Also, the doc calls the call `is_tripped(plan_id)` but on the breaker-tripped trip path real code calls `circuit_breaker.is_tripped(&plan_id)` with `plan_id` extracted from the signal stream via `extract_plan_id` at `:191-198` — the plan-id source is not shown in the doc snippet.
**Fix sketch**: Update Doc 02:173-176 snippet to show the real `Fail { watcher, reason: FailureKind::MaxIterations }` shape and the routing-bias side effect. Clarify in Doc 02:166 that the plan-id comes from the `PlanPhase`-tagged signal at the tail of the stream, not from a direct `plan_id` function argument.

---

## C.08 — `is_broken` alias used from orchestrate.rs call sites (code-only, not in doc)

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 02 uses `is_tripped` throughout. The orchestrator call sites at `crates/roko-cli/src/orchestrate.rs:3885` and `:3897` instead call `is_broken`.
**Reality**: `is_broken` at `crates/roko-conductor/src/circuit_breaker.rs:78-80` is a documented alias for `is_tripped`: `#[must_use] pub fn is_broken(&self, plan_id: &str) -> bool { self.is_tripped(plan_id) }`. Unit test `trips_at_max_failures` at `:146-153` asserts both names return the same bool after trip. The `handle_tripped_circuit_breaker` helper at `orchestrate.rs:3844-3881` calls `circuit_breaker().get_record(plan_id)` at `:3850` to pull the reasons list into a diagnosis payload, emits `EventKind::InterventionFired`, emits a `conductor.circuit_breaker` signal, and emits a `WatcherAlert { watcher: "circuit-breaker", ... }` execution event. `ensure_dispatch_allowed` at `:3884-3892` calls `is_broken(plan_id)` before every agent dispatch (invoked from `dispatch_agent_with` at `:9636`), and `run_conductor_check` at `:3897-3900` also checks `is_broken` and short-circuits to `ConductorDecision::Continue` after pausing the plan. Dispatch-refusal test at `:14098-14148` asserts that after two recorded failures, the next `dispatch_agent_with` call returns `"circuit breaker tripped"` error and marks the plan paused. The two-name surface is intentional and works end-to-end.

---

## C.09 — Persistence in executor snapshot (Doc 02 §Persistence) — DRIFTED

**Status**: NOT DONE
**Severity**: HIGH
**Doc claim**: Doc 02:294-304 says "The circuit breaker state is part of the executor snapshot. When the orchestrator checkpoints to `.roko/state/executor.json`, failure records are included. On resume, the circuit breaker is restored from the snapshot, preserving failure counts across restarts. … This prevents a circumvention where restarting the orchestrator would reset all breakers. … The breaker survives crashes."
**Reality**: False. `ExecutorSnapshot` at `crates/roko-orchestrator/src/executor/snapshot.rs:24-37` contains only `plan_states: HashMap<String, PlanState>`, `queue_order: Vec<String>`, `speculative_executions: HashMap<String, SpeculativeExecution>`, and `timestamp_ms: u64`. There is **no** `circuit_breaker` or `failure_records` field. Grep confirms: `Grep 'CircuitBreaker|circuit_breaker|FailureRecord' crates/roko-orchestrator` returns zero matches. `save_snapshot_atomic` at `crates/roko-cli/src/orchestrate.rs:646-657` serializes `ExecutorSnapshot`, not the breaker. `PlanRunner::from_snapshot` at `orchestrate.rs:3318-3430` rehydrates the executor from the snapshot JSON, but constructs a fresh `Conductor::default()` (and therefore a fresh `CircuitBreaker::default()` at `crates/roko-conductor/src/conductor.rs:99`) — meaning failure counts are wiped on every restart. Although `FailureRecord` derives `Serialize, Deserialize` at `circuit_breaker.rs:14-22` (and has a roundtrip test at `:247-257`), the `CircuitBreaker` struct itself does *not* — `DashMap` is not serde-serializable out of the box, and there is no `to_snapshot` / `from_snapshot` method on `CircuitBreaker`. The doc's "breaker survives crashes" claim is not honored by the implementation.
**Fix sketch**: Either (a) add `failure_records: HashMap<String, FailureRecord>` to `ExecutorSnapshot`, wire `CircuitBreaker::to_snapshot() -> HashMap<String, FailureRecord>` and `CircuitBreaker::from_snapshot(HashMap<...>)` helpers, and teach `PlanRunner::from_snapshot` to rehydrate the breaker from the executor JSON; or (b) strike Doc 02:294-304 and replace with a short "Persistence: not yet wired — breaker state is reset on process restart" paragraph with a follow-up pointer. Track under P1 because a restart can currently bypass the failure budget.

---

## C.10 — `ConductorDecision` enum has exactly Continue / Restart / Fail (Doc 03 §The ConductorDecision Enum)

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 03:15-21 declares `pub enum ConductorDecision { Continue, Restart { reason: String }, Fail { reason: String } }`. Doc 03:1-5 promises "Three actions. No nudges. Continue, Restart, or Fail. The Conductor decides; it does not suggest." This is Hard Guarantee 6 per Doc 03:61-63.
**Reality**: The enum has exactly three variants and no "Suggest" or "Nudge" variant. `crates/roko-core/src/conductor.rs:22-42` defines `#[non_exhaustive] pub enum ConductorDecision { Continue, Restart { watcher, reason }, Fail { watcher, reason: FailureKind } }`. The doc comment at `:19-21` explicitly reiterates: "Simplified from Mori's `InterventionTier { Nudge, Restart, Abort }` — this is §11.2 of the parity checklist: **no nudges**." Grep `pub enum ConductorDecision` returns exactly one hit (`roko-core/src/conductor.rs:25`). Grep `Suggest|Nudge` across `crates/` for this enum finds only the historical note at `roko-core/src/conductor.rs:19` ("Simplified from Mori's … Nudge, Restart, Abort") and no live variant. Contract upheld. Two drifts between doc snippet and real code: (1) real `Restart` and `Fail` variants each carry both `watcher: String` and a reason (not just `reason: String`); (2) real `Fail.reason` is typed `FailureKind`, not `String`. Neither drift changes the number or semantics of variants; they only refine the payload.

---

## C.11 — `Severity` enum (Info / Warning / Critical) with `PartialOrd` (Doc 03 §Severity System)

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 03:70-77 declares `#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)] pub enum Severity { Info = 0, Warning = 1, Critical = 2 }`. Doc 03:79-82 says the `PartialOrd` derive "enables severity comparison: Critical > Warning > Info".
**Reality**: Matches. `crates/roko-conductor/src/interventions.rs:22-31` declares `#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)] #[serde(rename_all = "snake_case")] pub enum Severity { Info = 0, Warning = 1, Critical = 2 }`. Real derive list is a superset: adds `Hash`, `Serialize`, `Deserialize`, and `#[serde(rename_all = "snake_case")]` (for the "info"/"warning"/"critical" wire tags that watchers use at `crates/roko-conductor/src/watchers/*.rs:*`). Unit test `severity_ordering` at `interventions.rs:151-154` asserts `Info < Warning < Critical`. Mapping helper `Severity::to_decision(self, watcher, reason)` at `:36-44` converts each variant to the corresponding `ConductorDecision`: Info → `ConductorDecision::cont()`, Warning → `ConductorDecision::restart(watcher, reason)`, Critical → `ConductorDecision::fail(watcher, FailureKind::Other(reason))`. Tests at `:157-171` exercise each branch. Note the Critical-branch choice of `FailureKind::Other(reason)` at `:41` is a pragmatic default; the circuit-breaker auto-trip path in `Conductor::evaluate` uses `FailureKind::MaxIterations` at `conductor.rs:163` instead.

---

## C.12 — `WatcherOutput` + `InterventionPolicy` + `WorstSeverityPolicy` (Doc 03 §WatcherOutput, §InterventionPolicy, §WorstSeverityPolicy)

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 03:117-124 `WatcherOutput { watcher, severity, description, metric: Option<f64> }`. Doc 03:135-142 `trait InterventionPolicy { fn evaluate(&self, &[WatcherOutput], &Context) -> ConductorDecision; }`. Doc 03:152-175 `WorstSeverityPolicy` that takes `outputs.iter().map(|o| o.severity).max()` and maps Info→Continue, Warning→Restart, Critical→Fail, with empty-outputs → Continue.
**Reality**: `WatcherOutput` at `crates/roko-conductor/src/interventions.rs:50-60` with the exact fields `watcher: String`, `severity: Severity`, `description: String`, `metric: Option<f64>`. Builder helpers `new` (`:65-76`) and `with_metric` (`:80-83`), plus `to_decision` (`:87-89`) per-output. `InterventionPolicy` trait at `:99-105` takes `&[WatcherOutput]` and `&Context` and returns `ConductorDecision`, with an extra `name(&self) -> &str` method not shown in the doc but used for logging. `WorstSeverityPolicy` at `:109-121` implements the trait using `outputs.iter().max_by_key(|o| o.severity)` (equivalent to the doc's `.map(|o| o.severity).max()` because `WatcherOutput::to_decision` just delegates to `Severity::to_decision`). Empty-slice → `ConductorDecision::cont()` via `map_or_else(ConductorDecision::cont, WatcherOutput::to_decision)` at `:114`. Tests: `worst_severity_policy_empty_is_continue` at `:182-186`, `worst_severity_policy_picks_worst` at `:188-198`, `worst_severity_policy_critical_wins` at `:200-209`. The policy is the default wired into `Conductor::new` at `conductor.rs:98`. Matches the doc in every load-bearing detail; the only drift is the doc using `.map(...).max()` vs real `.max_by_key(...)`, which is semantically identical.

---

## C.13 — Decision flow and failure-recording side effects (Doc 03 §Decision Flow, §Escalation Semantics)

**Status**: PARTIAL
**Severity**: MEDIUM
**Doc claim**: Doc 03:206-245 illustrates signal-stream → watchers → `WatcherOutputs` → `WorstSeverityPolicy` → `ConductorDecision`; Doc 03:238-245 promises the orchestrator responds to `Restart` by (1) killing the current agent, (2) recording a failure in the circuit breaker, (3) spawning a new agent with the error brief. Doc 03:281-293 promises `Fail` (1) cancels in-flight tasks, (2) transitions plan phase, (3) records the failure in the breaker ("If this is the second failure, the plan is tripped"), (4) moves on to other plans, (5) surfaces the failure in the dashboard.
**Reality**: The conductor's `evaluate` records a failure on `ConductorDecision::Fail` only — not on `Restart` (`crates/roko-conductor/src/conductor.rs:176-184`). Doc 03:241-242 "Record failure in circuit breaker" shows Restart incrementing the counter; the real pattern is that the *iteration counter* increments per restart (Doc 03:267-270 talks about `MAX_ITERATION_LOOP`, which is the watcher-level threshold enforced by `IterationLoopWatcher` at `crates/roko-conductor/src/watchers/iteration_loop.rs:9`), but the plan-level `CircuitBreaker::failure_count` only advances on `Fail`. In the orchestrator, `run_conductor_check` at `crates/roko-cli/src/orchestrate.rs:3896-3910` calls `Conductor::evaluate` which internally records the failure; separately the dashboard surfaces the tripped state via the `WatcherAlert` execution event emitted by `handle_tripped_circuit_breaker` at `:3874-3880`. The doc correctly describes Fail-path behaviour at 03:281-293; the Restart-path description at 03:240-242 conflates iteration-counter increments with `CircuitBreaker::record_failure` calls.
**Fix sketch**: Update Doc 03:240-242 to read "Record the restart as an iteration counter increment (tracked by `IterationLoopWatcher`); the plan-level `CircuitBreaker` is only incremented on Fail decisions, not on Restart decisions." Add a note at 03:255 clarifying the two counters (iteration-loop per-plan at `MAX_IMPLEMENTER_ATTEMPTS=3` at `iteration_loop.rs:9`, plan-level breaker at `MAX_PLAN_FAILURES=2`) and the invariant `MAX_PLAN_FAILURES * (MAX_IMPLEMENTER_ATTEMPTS + 1) ≈ Doc 02:256`'s `2 × 3 = 6 max cycles` claim.

---

## C.14 — Watcher severity defaults (Doc 03 §Watcher Severity Defaults)

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 03:93-104 lists default severities for the ten watchers: ghost-turn=Warning, compile-fail-repeat=Warning, cost-overrun=Warning, iteration-loop=**Critical**, review-loop=Warning, spec-drift=Warning, stuck-pattern=Warning, test-failure-budget=Warning, time-overrun=Warning, context-window-pressure=Warning. Doc 03:106-108: "Only `iteration-loop` defaults to Critical."
**Reality**: Exact match by grep on each watcher file. `ghost_turn.rs:124` → `.tag("severity", "warning")`, `compile_fail_repeat.rs:94` → `"warning"`, `cost_overrun.rs:79` → `"warning"`, `iteration_loop.rs:104` → `"critical"` (exclusive Critical), `review_loop.rs:104` → `"warning"`, `spec_drift.rs:145` → `"warning"`, `stuck_pattern.rs:110` → `"warning"`, `test_failure_budget.rs:105` → `"warning"`, `time_overrun.rs:91` → `"warning"`, `context_window_pressure.rs:79` → `"warning"`. Ten-for-ten match. `Conductor::new` at `crates/roko-conductor/src/conductor.rs:82-94` constructs exactly these ten watchers. `watcher_count` test at `:491-495` asserts `self.watchers.len() == 10`. Severity parsing at `collect_watcher_outputs` `:210-214` converts `"critical"` → `Severity::Critical`, `"warning"` → `Severity::Warning`, everything else → `Severity::Info`. The `WorstSeverityPolicy` then maps the max severity to a `ConductorDecision`. Integration test `multiple_watchers_worst_wins` at `conductor.rs:459-489` constructs a stream that fires both ghost-turn (Warning) and iteration-loop (Critical) and asserts the final decision is terminal (Fail), confirming the mapping Doc 03:86-89 describes.

---

## C.15 — "Decide, don't nudge" — no Suggest/Nudge variant anywhere (Doc 03 §Why No Nudge, §The ConductorDecision Enum)

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 03:1-5 masthead and 03:39-63 "Why No Nudge" and 03:55-63 "Hard Guarantee 6" all restate that the Conductor produces exactly one of Continue / Restart / Fail and does not emit Suggest / Nudge / Advisory messages.
**Reality**: The invariant holds. `ConductorDecision` at `crates/roko-core/src/conductor.rs:25-42` has three variants — `Continue`, `Restart { watcher, reason }`, `Fail { watcher, reason: FailureKind }` — and the enum is `#[non_exhaustive]` so future variants cannot sneak in without a crate bump. `Severity::to_decision` at `crates/roko-conductor/src/interventions.rs:36-44` is the only site that maps severity → decision, and it has exactly three arms (Info/Warning/Critical). `WorstSeverityPolicy::evaluate` at `interventions.rs:111-121` produces `ConductorDecision` only through `WatcherOutput::to_decision` (which in turn delegates to `Severity::to_decision`). Grep confirms: no occurrences of `Nudge` or `Suggest` as a variant anywhere in `crates/` for `ConductorDecision` or any watcher policy. The only hit is the historical comment at `crates/roko-core/src/conductor.rs:19` explicitly contrasting against Mori's prior `InterventionTier { Nudge, Restart, Abort }`. Hard Guarantee 6 is intact at the type level; no code can construct a "nudge" through this API.

---

## C.16 — Cooldown per-watcher per-plan (Doc 03 §Cooldown Periods)

**Status**: NOT DONE
**Severity**: MEDIUM
**Doc claim**: Doc 03:297-313 says "Each watcher intervention has a built-in cooldown to prevent the conductor from firing the same intervention on consecutive evaluation cycles. … The production default is 120 seconds per plan per watcher."
**Reality**: No such cooldown exists in the conductor. `collect_watcher_outputs` at `crates/roko-conductor/src/conductor.rs:201-224` iterates every watcher on every `evaluate` call with no rate-limiting or debouncing. Grep of `crates/roko-conductor` for `cooldown|debounce|last_fired|120|rate_limit` returns no matches that relate to watcher firing (only the provider-health cooldown at `crates/roko-learn/src/provider_health.rs:160-168`, which is about per-provider circuit breaker cooldown, a different concept). Each watcher makes its own decision whether to fire based on signal-stream state, but there is no per-plan-per-watcher timer gating repeat firings within the 120s window the doc promises. The doc also motivates this with a concrete production story ("the conductor would detect a stuck agent, emit a restart signal, and then on the next tick … detect the same stuck signal again and emit another restart"), so this is a real gap, not just documentation overreach.
**Fix sketch**: Either (a) add a `DashMap<(String, String), i64>` on `Conductor` keyed by `(plan_id, watcher_name)` → last-fire-ms; wrap `collect_watcher_outputs` to filter out outputs whose key has fired within `COOLDOWN_MS = 120_000`; or (b) remove Doc 03:297-313 and replace with a "Planned: per-plan-per-watcher cooldown" callout. Without this, the restart storm described in the doc can occur today. Track under P1.

---

## C.17 — Intervention signals are emitted (Doc 03 §Intervention Signals)

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 03:317-340 shows non-Continue conductor decisions emit a `Signal::builder(Kind::Custom("conductor.intervention".into())).body(Body::text(...)).tag("watcher", ...).tag("severity", ...).tag("plan_id", ...)...` for observability and learning.
**Reality**: Two related emission paths match the doc. (1) `outputs_to_signals(&[WatcherOutput]) -> Vec<Engram>` at `crates/roko-conductor/src/interventions.rs:124-144` emits one signal per non-Info output with `Kind::Custom(format!("conductor:alert:{watcher}"))` (note colon, not dot, and not exactly `conductor.intervention`), JSON body containing the full `WatcherOutput`, `.tag("watcher", watcher)`, `.tag("severity", format!("{:?}", severity))`. Test at `interventions.rs:211-224` exercises this. (2) `Conductor::decide` (the `Policy` impl) at `crates/roko-conductor/src/conductor.rs:226-249` additionally emits a `Kind::Custom("conductor.decision")` signal carrying the `ConductorDecision` body and a `"decision"` tag (at `:240-243`). (3) The orchestrator-side `handle_tripped_circuit_breaker` at `crates/roko-cli/src/orchestrate.rs:3873` emits `Kind::Custom("conductor.circuit_breaker")` with the diagnosis payload. Doc's exact literal `"conductor.intervention"` does not appear in code (`Grep conductor.intervention` in `crates/` returns zero hits). The three actual kinds — `conductor:alert:<watcher>`, `conductor.decision`, `conductor.circuit_breaker` — cover the same observability + learning use cases but use different naming conventions than the doc shows.
**Fix sketch**: Update Doc 03:321 to use `Kind::Custom(format!("conductor:alert:{watcher_name}"))` to match the real wire format, and note the additional `conductor.decision` kind at 03:319-340. Alternatively, rename the real kinds to `conductor.intervention.alert` and `conductor.intervention.decision` if consistency with the doc's `conductor.intervention` prefix is desired — the former is cheaper.

---

## Section Summary

| Status | Count | Items |
|--------|-------|-------|
| DONE | 10 | C.01 MAX_PLAN_FAILURES=2, C.02 DashMap storage, C.04 breaker API surface, C.05 2-state plan breaker + 3-state provider breaker, C.06 error cooldowns (narrow), C.08 is_broken alias wiring, C.10 3-variant ConductorDecision, C.11 Severity enum, C.12 WatcherOutput/Policy/WorstSeverityPolicy, C.14 watcher severity defaults, C.15 no-nudge invariant |
| PARTIAL | 3 | C.03 naming (PlanCircuitBreaker collision with 3 `CircuitBreaker` types), C.07 evaluate() snippet drift (Fail signature + routing-bias side effect), C.13 decision-flow description conflates iteration counter with breaker counter |
| NOT DONE | 2 | C.09 breaker persistence in executor snapshot (HIGH — restart bypasses budget), C.16 per-plan-per-watcher 120s cooldown (MEDIUM — restart storm possible) |

**HIGH items**:
- C.09 — Doc 02:294-304 claims the breaker survives crashes via `ExecutorSnapshot`; in fact `ExecutorSnapshot` at `crates/roko-orchestrator/src/executor/snapshot.rs:24-37` has no `failure_records` field and `PlanRunner::from_snapshot` at `crates/roko-cli/src/orchestrate.rs:3318-3430` constructs a fresh `Conductor::default()`, so every restart resets all per-plan failure counts to zero. The two-failure budget can be circumvented by `kill -9` + relaunch.

**MEDIUM items**:
- C.13 — Doc 03:240-242 says Restart "records a failure in the circuit breaker"; in fact only `ConductorDecision::Fail` triggers `record_failure` at `crates/roko-conductor/src/conductor.rs:176-184`. Restarts bump an iteration counter (inside `IterationLoopWatcher` at `crates/roko-conductor/src/watchers/iteration_loop.rs:9` with `MAX_IMPLEMENTER_ATTEMPTS=3`), but they do *not* tick the plan-level breaker.
- C.16 — Doc 03:297-313 promises a 120 s per-plan-per-watcher cooldown; no such timer exists in `crates/roko-conductor/`. The restart-storm scenario the doc describes can happen today.

**LOW items**:
- C.03 — `PlanCircuitBreaker` is not the actual struct name; the real name is plain `CircuitBreaker` and collides with two other `CircuitBreaker` structs at `crates/roko-core/src/error/retry.rs:139` and the `ProviderHealth`-wrapping type at `crates/roko-learn/src/provider_health.rs:82`.
- C.07 — The `Conductor::evaluate` snippet in Doc 02 uses `Fail { reason: format!(...) }` (string), but the real variant is `Fail { watcher: String, reason: FailureKind }` with `FailureKind::MaxIterations` on the breaker-trip path.
- C.17 — Doc 03's literal `conductor.intervention` Kind is not in code; actual kinds are `conductor:alert:<watcher>`, `conductor.decision`, and `conductor.circuit_breaker`.

Overall: Doc 02 and Doc 03 hold up on the core contracts that matter — the three-variant `ConductorDecision`, the three-level `Severity`, the `WorstSeverityPolicy` default mapping, the ten watcher severity defaults, the `DashMap<String, FailureRecord>` concurrency model, and the `MAX_PLAN_FAILURES = 2` budget are all implemented as described. The "decide, don't nudge" invariant is enforced at the type level by a `#[non_exhaustive]` enum with exactly three variants and no `Suggest` / `Nudge` variant anywhere in the crates tree. Where the docs drift, they do so on peripheral details: (a) Doc 02's persistence section overstates (C.09 — breaker does not survive crashes); (b) Doc 03's cooldown section overstates (C.16 — no 120 s per-watcher debounce exists); (c) Doc 03 conflates iteration-counter increments with breaker-counter increments on Restart (C.13); (d) struct naming and signal-kind literals have modest divergences (C.03, C.07, C.17). The two NOT DONE items (C.09 persistence, C.16 cooldown) represent real operational gaps: a restart can today reset the per-plan failure count, and nothing prevents a restart storm within an evaluation cycle — both are load-bearing promises in the docs that code does not honor.

## Agent Execution Notes

### C.09 / C.16 — Real Runtime Guardrails

This section owns two of the most operationally meaningful non-theory gaps:

1. breaker persistence across restart,
2. per-plan-per-watcher cooldown.

Treat them as runtime guardrails, not doc polish.

### C.13 / C.17 — Make The Decision Contract Explicit

Prefer one clear runtime story for:

- what counts toward the plan-level breaker,
- what a `Restart` actually means,
- which intervention kinds are emitted.

Do not conflate plan breaker, provider-health breaker, and iteration-loop counters.

Acceptance criteria for this section:

- later agents can explain the conductor’s decision flow without cross-referencing multiple drifted docs,
- snapshot and cooldown semantics are either real or explicitly deferred,
- emitted decision/intervention kinds are no longer ambiguous.
