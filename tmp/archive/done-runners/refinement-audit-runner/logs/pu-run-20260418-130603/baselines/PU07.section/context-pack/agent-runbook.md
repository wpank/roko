# Agent Runbook — Batch 07

Use this when executing any batch from `tmp/docs-parity/07`.

## Mission

Wire the conductor infrastructure that already ships before adding new control theory. The 10-watcher ensemble, circuit breaker, diagnosis engine, anomaly detector, retry-path bandit, and adaptive gate thresholds are all live; the gaps worth closing first are the dark `StuckDetector` / `MetaCognitionHook`, the unwired `HealthMonitor`, the non-persisted plan-level breaker, and the `ProcessSupervisor` that never actually owns agent spawns.

## Workflow

1. Read [00-INDEX.md](../00-INDEX.md), [BATCHES.md](../BATCHES.md), and the owning letter file (A-architecture, B-watchers-signals, C-decision-space, D-diagnosis-stuck, E-health-adaptive, F-theory-learning).
2. Read [SOURCE-INDEX.md](../SOURCE-INDEX.md) and search the actual code before planning changes.
3. Prefer wiring already-shipped conductor surfaces over building new control-theory primitives.
4. Keep the patch inside the batch scope unless the code makes that impossible.
5. Run the verify commands.
6. If blocked, leave a concrete blocker note with exact file paths and missing dependencies.

## Default Decision Rules

- If the conductor crate already ships a primitive (StuckDetector, HealthMonitor, PhaseTransition, ProcessSupervisor), activate it from `orchestrate.rs` before inventing a new subsystem.
- If two parallel stacks exist (e.g. `ProcessSupervisor` vs `roko-agent/src/process/registry.rs`), pick one canonical path and reduce ambiguity.
- If a batch touches `orchestrate.rs`, choose the smallest production path that proves the contract (one call site, one test).
- If a task starts requiring new Yerkes-Dodson theory, Good Regulator Brier/Kalman primitives, federated multi-level conductors, or self-healing machinery, record the handoff and stop.

## Required Completion Evidence

Every batch completion note should include:

- files changed,
- commands run,
- whether tests passed,
- what was intentionally deferred,
- and which later batch now has a cleaner seam because of this work.

## Failure Modes To Avoid

- leaving `StuckDetector` / `MetaCognitionHook` unwired while claiming meta-cognition works (D.11),
- building fresh Brier / self-model / Kalman primitives when the current loop has dark `HealthMonitor` wiring to do first (E.05, F.11-F.14),
- letting the two parallel process stacks (`roko-runtime::ProcessSupervisor` vs `roko-agent::process::registry`) drift further without picking one (E.14),
- shipping a "breaker persists across crashes" story without adding `failure_records` to `ExecutorSnapshot` (C.09),
- widening docs-honesty work into a Yerkes-Dodson / federation / triple-loop speculative architecture build.
