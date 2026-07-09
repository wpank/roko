# Repo Map — Shared Conductor Context

Quick reference for agents working on `07` conductor parity.

## Workspace Root

`/Users/will/dev/nunchi/roko/roko/`

## High-Value Paths

| What | Path | Why It Matters In Batch 07 |
|------|------|----------------------------|
| Conductor crate root | `crates/roko-conductor/src/lib.rs` | 8 modules + `watchers/` — the composite `Policy` entry point |
| Conductor main | `crates/roko-conductor/src/conductor.rs` | `evaluate()` 5-step loop, `RoutingBias`, watcher wiring |
| Circuit breaker | `crates/roko-conductor/src/circuit_breaker.rs` | `MAX_PLAN_FAILURES=2`, `DashMap<String, FailureRecord>`, 2-state |
| Interventions | `crates/roko-conductor/src/interventions.rs` | `Severity`, `WatcherOutput`, `WorstSeverityPolicy`, `outputs_to_signals` |
| Diagnosis engine | `crates/roko-conductor/src/diagnosis.rs` | 34 patterns + 20 `ErrorCategory` + 9 `SuggestedIntervention` |
| Health monitor | `crates/roko-conductor/src/health.rs` | 4 checks + `HealthStatus` — built but not wired (E.05) |
| Stuck detection | `crates/roko-conductor/src/stuck_detection.rs` | 6 `StuckKind` + `MetaCognitionHook` — 1,085 LOC, unwired (D.11) |
| State machine | `crates/roko-conductor/src/state_machine.rs` | `phase_timeout()` + `PhaseTransition` audit struct (E.09) |
| 10 watchers | `crates/roko-conductor/src/watchers/` | one module per watcher, all `impl Policy` |
| Main orchestrator | `crates/roko-cli/src/orchestrate.rs` | conductor + breaker + diagnosis call sites; `PlanRunner` |
| Executor snapshot | `crates/roko-orchestrator/src/executor/snapshot.rs` | `ExecutorSnapshot` — no `failure_records` today (C.09) |
| Process supervisor | `crates/roko-runtime/src/process.rs` | `ProcessSupervisor` — built, never owns agent spawn (E.14) |
| Agent process stack | `crates/roko-agent/src/process/{group,kill,registry}.rs` | parallel PID stack that actually owns agent spawn |
| Adaptive gate thresholds | `crates/roko-gate/src/adaptive_threshold.rs` | EMA-per-rung — cross-ref F.19 |
| ConductorBandit | `crates/roko-learn/src/conductor.rs` | wired into retry path (F.21 counters stale Doc 15) |
| AnomalyDetector | `crates/roko-learn/src/anomaly.rs` | EWMA + prompt-hash window — wired pre-turn (F.15/F.16) |
| Conductor docs | `docs/07-conductor/` | 16 chapters — source material being checked |
| Parity batch | `tmp/docs-parity/07/` | execution contract, letter files A-F, findings |

## Important Corrections

Use these instead of older or misleading assumptions:

- `HealthMonitor::new(...)` exists and ships 4 checks, but no CLI site constructs `SystemSnapshot` or calls `overall_status()` — the Doc 06 "10 s periodic snapshot" story is currently unwired (E.05).
- `StuckDetector` is 1,085 LOC of production-shaped heuristics with zero call sites in `crates/roko-cli/`; only `StuckPatternWatcher` (one OutputLoop-like heuristic via `Policy`) fires in production (D.11).
- `ConductorBandit` IS wired into the orchestrator retry path at `orchestrate.rs:6039-6298` with persistence + `record_outcome` + `select_action` — Doc 15's "Scaffold" / "built, not wired" claim is stale (F.21).
- `supervisor.count()` returns 0 during real runs because `supervisor.spawn(SpawnConfig)` is never called; agents spawn via `roko-agent/src/process/registry.rs` and populate a separate static registry (E.14).
- `check_golem_status` is a real function name at `health.rs:159, 258` whose body inspects `chain_connected` — the `roko-golem` dissolution left this naming drift behind (A.08 / E.03 / F.05).
- `MAX_PLAN_FAILURES = 2`, not 3; the conductor `CircuitBreaker` is 2-state (tripped/not), not the 3-state `Closed/Open/HalfOpen` model that lives in `provider_health.rs` and `roko-core/src/error/retry.rs`.
- `RoutingBias` is a fourth `Conductor` field (`Mutex<RoutingBias>`) consumed by the cascade router at `orchestrate.rs:1787-1795` but is undocumented in Doc 00 (A.10).
- `ConductorDecision` is `#[non_exhaustive]` with exactly 3 variants (`Continue`, `Restart`, `Fail`) — no `Suggest` / `Nudge` variant anywhere (C.15).
- `PhaseTransition` is a real type in `state_machine.rs`, but the main orchestrator still emits raw `serde_json::json!` payloads for phase-change events (E.09).
- `LatencyStats::adaptive_timeout_ms()` exists, but there is no current production path that uses it to override provider timeout config (E.10).

## Search Priorities

Before editing, search these first:

```bash
rg -n "HealthMonitor|SystemSnapshot|check_terminal_liveness|check_golem_status|overall_status" crates/roko-cli crates/roko-conductor
rg -n "StuckDetector|MetaCognitionHook|ActivityEntry|check_stuck|assess\\(" crates/roko-cli crates/roko-conductor
rg -n "ConductorBandit|retry_conductor|select_action|record_outcome|conductor_policy_path" crates/roko-cli crates/roko-learn
rg -n "ProcessSupervisor|supervisor\\.spawn|supervisor\\.count|register_spawned_pid|cleanup_orphaned" crates/roko-cli crates/roko-runtime crates/roko-agent
rg -n "PhaseTransition|adaptive_timeout_ms|timeout_ms|attempt_id" crates/roko-cli crates/roko-conductor crates/roko-learn crates/roko-agent crates/roko-runtime
rg -n "DiagnosisEngine|ErrorCategory|SuggestedIntervention|built_in_patterns" crates/roko-cli crates/roko-conductor
rg -n "Yerkes|PressureDial|FlowDetector|ModelPressureProfile|PressureBandit|pressure_index|CognitiveSignal" crates/
```

## Build Commands

```bash
cargo build --workspace
cargo test --workspace
cargo clippy --workspace --no-deps -- -D warnings
```

## Practical Rules

1. Wire shipped conductor primitives before adding new control-theory ones.
2. Prefer one canonical owner of a signal or handle (supervisor vs agent registry, breaker vs iteration counter).
3. If a batch only proves one production path, make that path extremely explicit and testable via `cargo run -p roko-cli`.
4. If a task really belongs to Yerkes-Dodson / federation / triple-loop learning, record the handoff and stop.
