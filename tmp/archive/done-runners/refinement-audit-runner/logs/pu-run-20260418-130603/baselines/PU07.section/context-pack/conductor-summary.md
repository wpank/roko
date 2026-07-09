# Conductor Summary — Batch 07

Concise runtime picture for agents entering `07` without prior context.

## What Is Already Real

- `Conductor::evaluate()` ships the documented 5-step loop (breaker check -> 10 watchers -> `WorstSeverityPolicy` -> record -> return) and is called from `orchestrate.rs`.
- 10 watchers with the exact `Info/Warning/Critical` severity ladder, `Policy` trait + `WorstSeverityPolicy` composite, `ConductorDecision` 3-variant `#[non_exhaustive]` enum (Continue/Restart/Fail) — no nudges.
- `DashMap`-backed `CircuitBreaker` with `MAX_PLAN_FAILURES=2`, `is_tripped`/`is_broken` both used from `orchestrate.rs` to refuse dispatch pre-launch.
- `DiagnosisEngine` with 34 patterns + 20 `ErrorCategory` variants + 9 `SuggestedIntervention` variants; wired into `orchestrate.rs` at the circuit-breaker failure and retry-classification call sites.
- `AnomalyDetector` (EWMA + prompt-hash window + quality history + budget accumulator) wired pre-turn, and `ConductorBandit` wired into the per-task retry path with 7 actions + 19-dim state + persistence.
- `AdaptiveThresholds` per gate rung (EMA) and `ProviderHealthTracker` (3-state) both persist and feed routing / dispatch.

## What Is Misleading Today

- `StuckDetector` + `MetaCognitionHook` are 1,085 LOC with full tests but zero runtime callers — 5 of 6 stuck heuristics are dark (D.11).
- `HealthMonitor` is built with 4 checks but no CLI constructs `SystemSnapshot` or calls `overall_status()` on a 10 s tick (E.05).
- The plan-level `CircuitBreaker` is not in `ExecutorSnapshot`; a `kill -9` + relaunch resets every failure count (C.09).
- `ProcessSupervisor` is a field on `PlanRunner` but `supervisor.spawn(...)` is never called — agent spawn is handled by the parallel `roko-agent/src/process/{group,kill,registry}.rs` stack, so `supervisor.count()` always returns 0 (E.14).
- `PhaseTransition` is a real type in `state_machine.rs`, but orchestrator emission still uses raw JSON payloads, and `adaptive_timeout_ms` is computed but not applied on a production timeout path (E.09, E.10).
- `check_golem_status` is a post-dissolution naming holdover; the second health check is really chain-status (A.08 / E.03 / F.05).
- `RoutingBias` is a live fourth `Conductor` field and cascade-router consumer but the architecture doc never mentions it (A.10).
- Doc 15 still says `ConductorBandit` is "Scaffold" / "built, not wired" — it IS wired into the retry path; the gap is only the `LearnedConductorPolicy` wrapper (F.21).
- Doc 12 (Yerkes-Dodson) carries "Implementation: Built" but `PressureBandit` / `FlowDetector` / `ModelPressureProfile` / `pressure_index` are entirely grep-negative.

## What Batch 07 Should Usually Do

1. Wire the shipped-but-dark primitives (`StuckDetector`, `MetaCognitionHook`, `HealthMonitor`) into the `orchestrate.rs` tick loop.
2. Persist the plan-level breaker across restarts by extending `ExecutorSnapshot` with `failure_records`.
3. Resolve the `ProcessSupervisor` vs `roko-agent/process/registry.rs` split — pick one canonical owner of agent PIDs.
4. Make `PhaseTransition` / adaptive-timeout / attempt-tracking semantics explicit instead of leaving them half-runtime and half-doc.
5. Fix the post-dissolution drift (`golem_status` -> `chain_status`) and document `RoutingBias` in Doc 00.
6. Explicitly defer Yerkes-Dodson, federated conductors, triple-loop learning, and self-healing work.
