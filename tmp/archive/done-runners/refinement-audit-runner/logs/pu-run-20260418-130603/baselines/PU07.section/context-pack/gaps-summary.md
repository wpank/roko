# Gap Inventory — 07 Conductor

Concise gap list for agents working on conductor parity batches.

## Focus Now

These are the gaps batch `07` should actively try to close:

### 1. StuckDetector + MetaCognitionHook Built But Not Wired — HIGH (D.11)

- `crates/roko-conductor/src/stuck_detection.rs` ships 1,085 LOC with 6 `StuckKind` heuristics + `MetaCognitionHook` at Theta frequency,
- `grep StuckDetector\|MetaCognitionHook` on `crates/roko-cli/` returns zero matches,
- only `StuckPatternWatcher` (one OutputLoop-ish heuristic) fires in production — 5 of 6 stuck checks are dark.

### 2. HealthMonitor Shipped But Not Constructed — HIGH (E.05)

- `HealthMonitor::new()` ships 4 checks (`terminal_liveness`, `golem_status`, `spec_drift`, `coverage_trend`),
- no CLI site constructs `SystemSnapshot` or calls `overall_status()`,
- the Doc 06 "10 s periodic snapshot" story is not honored by the runtime.

### 3. Circuit Breaker State Doesn't Persist Across Crashes — HIGH (C.09)

- `ExecutorSnapshot` at `crates/roko-orchestrator/src/executor/snapshot.rs:24-37` has no `failure_records` field,
- `PlanRunner::from_snapshot` constructs a fresh `Conductor::default()`,
- the two-failure budget can be bypassed via `kill -9` + relaunch, contradicting Doc 02 §Persistence.

### 4. ProcessSupervisor Accounting Is Structurally Wrong — HIGH (E.14)

- `PlanRunner` holds `supervisor: Arc<ProcessSupervisor>` but `supervisor.spawn(...)` is never called,
- agents spawn via the parallel `roko-agent/src/process/{group,kill,registry}.rs` stack that populates a separate registry,
- `supervisor.count()` returns 0 during real runs, so `active_agents` accounting at `orchestrate.rs:3945, 4157` is structurally wrong.

### 5. Post-Dissolution Name Drift (golem_status) — MEDIUM (A.08 / E.03 / F.05)

- `HealthMonitor::new()` still registers a `golem_status` check whose body reads `chain_connected`,
- the `roko-golem` dissolution (tracked in `tmp/docs-parity/06` F.05) left this holdover behind,
- cheap rename to `check_chain_status` closes the drift in both doc 00 and doc 06.

### 6. State-Machine And Timeout Contracts Are Only Half Active — MEDIUM (E.09 / E.10 / E.15)

- `PhaseTransition` is a real type, but orchestrator emits raw JSON instead,
- `adaptive_timeout_ms` is real, but no production timeout consumer reads it,
- attempt tracking for stale-exit races is still absent,
- later agents need one honest answer for whether these are runtime contracts or just nearby library surfaces.

## Defer From Batch 07

These are valid findings, but they should usually be documented and handed off:

- Yerkes-Dodson pressure dial / flow detection / `ModelPressureProfile` (Doc 12) -> later learning pass
- `CognitiveSignal` typed-interrupt enum (Doc 09) -> later signal-channel redesign
- Good Regulator Brier / Kalman / `ForwardPredictor` (Doc 08) -> later self-model pass
- Federated `ConductorLevel` / `SelfHealingConductor` / triple-loop learning (Doc 15) -> later governance pass
- CEP composite patterns / `OnlineIsolationForest` / CUSUM (Doc 01 advanced) -> later anomaly research pass
- Linux cgroup resource limits (Doc 13) -> later deployment-hardening pass

## Working Rule

If a conductor task requires:

- a new Yerkes-Dodson / pressure-theory primitive,
- a new Brier / Kalman / active-inference scorer,
- or a federated / self-healing conductor layer,

then batch `07` should normally wire the shipped infrastructure first (StuckDetector, HealthMonitor, breaker persistence, supervisor unification) and defer the rest.
