# Conductor Summary - Parity Refresh 07

Concise runtime picture for the docs-only refresh in `tmp/docs-parity/07/`.

## Live And In Path

- `Conductor::new()` wires 10 default watchers and `WorstSeverityPolicy`.
- `Conductor::evaluate()` checks the plan-level `CircuitBreaker`, runs the
  watcher set, updates `RoutingBias`, and records breaker failures.
- `orchestrate.rs` refuses dispatch for tripped plans and calls
  `run_conductor_check(...)`, so the core watcher loop is live.
- `DiagnosisEngine::default().diagnose(...)` is used from live
  orchestrator paths for circuit-breaker diagnosis and retry
  classification.
- `ConductorBandit::load_or_new(...)` is live in the retry path; docs
  should not describe it as scaffold-only or unwired.

## What This Refresh Needs To Fix

- Make the top-line status honest: the conductor core is already wired.
- Describe `RoutingBias` as a live conductor surface, not an omitted
  detail.
- Retag theory-heavy material so docs do not blur "live runtime",
  "library surface", and "planned design".
- Update repo counts and operator guidance to match the current tree.

## Adjacent Surfaces To Describe Carefully

These may still matter in the broader audit, but they are not the work
item for this docs-only refresh:

- `HealthMonitor`
- `StuckDetector` and `MetaCognitionHook`
- `PhaseTransition`
- `adaptive_timeout_ms`
- `ProcessSupervisor` ownership vs `roko-agent` process registry

The job here is to document their current posture honestly, not to wire
them.

## Explicit Deferrals

Defer these from the refresh posture:

- Yerkes-Dodson pressure / flow-control theory
- Good Regulator self-model work
- typed `CognitiveSignal` redesign
- conductor federation / self-healing / triple-loop learning

If a task needs Rust changes outside the owned docs files, it belongs to a
separate implementation pass.
