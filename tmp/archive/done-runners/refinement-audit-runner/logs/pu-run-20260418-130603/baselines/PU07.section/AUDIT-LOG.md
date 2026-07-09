# AUDIT-LOG — Conductor Parity Initial Generation

Initial generation of `tmp/docs-parity/07/` against current source.

**Generation date**: 2026-04-16
**Scope**: 16 PRD files in `docs/07-conductor/` (~7,500 lines) audited against
`crates/roko-conductor/` (~4,700 LOC, 19 files), with cross-references to
`crates/roko-cli/src/orchestrate.rs`, `crates/roko-runtime/`,
`crates/roko-learn/`, `crates/roko-gate/`, and `crates/roko-core/`.
**Method**: Three-wave parallel agent generation:

- **Wave 1 (6 agents in parallel)** — one per letter file (A–F). Each agent
  read its owning PRD docs in full, audited each claim against code with Grep
  + file reads, wrote item-by-item parity entries with `file.rs:line`
  anchors.
- **Wave 2 (2 agents in parallel)** — `SOURCE-INDEX.md` (aggregated ~530
  per-crate anchors + ~75 grep-negative identifiers from the six letter
  files) and `context-pack/` (five operator-facing summary files).
- **Wave 3 (1 agent)** — this file + `00-INDEX.md` + `BATCHES.md`
  (aggregation and follow-up batching).

---

## Generation totals

| File | Items | DONE | PARTIAL | NOT DONE |
|------|-------|------|---------|----------|
| A | 13 | 7 | 6 | 0 |
| B | 25 | 17 | 5 | 3 |
| C | 17 | 12 | 3 | 2 |
| D | 20 | 10 | 9 | 1 |
| E | 30 | 8 | 9 | 13 |
| F | 28 | 13 | 5 | 10 |
| **Total** | **133** | **67** | **37** | **29** |

HIGH-severity items (4):

- **C.09** — `CircuitBreaker` state does not survive crashes
- **D.11** — `StuckDetector` + `MetaCognitionHook` unwired
- **E.05** — `HealthMonitor` unconstructed
- **E.14** — `ProcessSupervisor` dark-spawn path

---

## Systematic issues observed

1. **Conductor foundations are shipped.** Ten watchers
   (`GhostTurnWatcher` through `StuckPatternWatcher`), `Policy` trait,
   `ConductorDecision` 3-state `#[non_exhaustive]` enum, `DashMap`-backed
   `CircuitBreaker`, `DiagnosisEngine` with 34 patterns over 20 categories +
   9 interventions, `AnomalyDetector` with pre-turn wiring, `ConductorBandit`
   with orchestrator-retry-path wiring, `AdaptiveThresholds` EMA-per-rung
   persistence, and `ProviderHealthTracker` three-state API-level breaker
   all present and correctly named.

2. **"Built but not wired" pattern recurs in the conductor subsystem.**
   `HealthMonitor` (E.05, 568 LOC with tests, zero CLI call sites),
   `StuckDetector` + `MetaCognitionHook` (D.11, 1,085 LOC, zero CLI call
   sites), `ProcessSupervisor` spawn path (E.14, field on `PlanRunner` but
   `supervisor.spawn()` never called). Each is a fully-tested module with
   zero runtime call sites from `crates/roko-cli/`.

3. **Doc 12 Yerkes-Dodson is 919 lines of design-only theory under a
   "Built" banner.** `rg 'Yerkes|PressureDial|FlowDetector|CooperationMetric'
   crates/` returns zero hits. Ten items (E.22–E.30, plus the doc's entire
   §"Implementation" sections) are grep-negative while the top-of-file
   banner claims implementation.

4. **Doc 13 ProcessSupervisor claim is misleading.** `supervisor.count()`
   is called from `orchestrate.rs:3945, 4157` but returns 0 because agents
   dispatch through a parallel `roko-agent/src/process/*` stack that uses
   a different static registry at `roko-agent/src/process/registry.rs`.
   `supervisor.spawn(SpawnConfig)` appears only in `roko-runtime`'s own
   tests; real agent spawn is at `roko-agent/src/exec.rs:178` and
   `claude_cli_agent.rs:420`.

5. **`CircuitBreaker` state does not survive `kill -9`.** `ExecutorSnapshot`
   at `crates/roko-orchestrator/src/executor/snapshot.rs:24-37` has
   `plan_states`, `queue_order`, `speculative_executions`, `timestamp_ms`
   but no `failure_records`. A restart constructs `Conductor::default()`
   fresh, resetting every plan's failure count to zero. The
   `MAX_PLAN_FAILURES=2` budget can be bypassed via `kill -9` + relaunch,
   contradicting Doc 02:294-304.

6. **Post-dissolution naming drift.** `check_golem_status` still registered
   in `HealthMonitor::new()` at `health.rs:159` with body at `:258-270`.
   The `roko-golem` dissolution is complete (tracked in section 06 F.05)
   but this identifier — which actually reads `chain_connected` — remains.

7. **`RoutingBias` is an undocumented fourth `Conductor` field.** The arch
   doc (00) describes `Conductor` with three components; the real struct
   has `routing_bias: Mutex<RoutingBias>` at `conductor.rs:60-61` driving
   cascade-router decisions via `orchestrate.rs:1787-1795`. Eight
   subsystems ship; Doc 00 describes seven.

8. **Doc 15 `ConductorBandit` "not wired" claim is stale.** The bandit is
   fully wired into the orchestrator retry path at
   `orchestrate.rs:6039-6298` with persistence via
   `conductor_policy_path(workdir)`, 7 actions (`Continue`, `InjectHint ×
   3`, `SwitchModel`, `Restart`, `Abort`), and outcome recording. Doc 15
   banner + file-reference both need a 5-minute edit.

9. **Constant-name drifts.** Doc 01 lists `MAX_COMPILE_FAIL_REPEAT`,
   `MAX_ITERATION_LOOP`, `MAX_STUCK_REPEATS`, `DEFAULT_BUDGET_USD`; code
   uses `MAX_IDENTICAL_COMPILE_FAILURES`, `MAX_IMPLEMENTER_ATTEMPTS`,
   `MAX_IDENTICAL_ACTIONS`, `DEFAULT_BUDGET`. Two watcher-kind
   descriptions also drift: compile-fail-repeat scans `CompileDiagnostic`
   (not `GateVerdict`), iteration-loop scans `PlanPhase` (not
   `GateVerdict`).

10. **Category → intervention table inversion (D.09).** Doc 04:142-164
    table claims `ImportError → AutoFix` and `TestFailure →
    RetryWithContext`; real code at `diagnosis.rs:335, 350` does the
    opposite. Doc's claimed $11.94/task savings on `ImportError → AutoFix`
    is not captured because every `ImportError` pattern has
    `suggested_action: SuggestedIntervention::RetryWithContext` in the
    shipped `built_in_patterns()`.

11. **`PhaseTransition` typed struct shipped but unused.**
    `state_machine.rs:61-73` defines the record (`plan_id`, `from`, `to`,
    `at_ms`, `reason`), but `orchestrate.rs:5682-5684, 5704-5707,
    9424-9430` emits raw `serde_json::json!` payloads instead. Four
    downstream consumers Doc 10 names (post-mortem, optimization, anomaly,
    learning) are unreachable as-is.

12. **Doc 09 `CognitiveSignal` enum entirely design-only** despite a
    "Built" banner on the doc's own line 8 — the doc's own §"Implementation
    Status" at lines 247-271 admits this. The banner is wrong; the
    admission is correct.

13. **Doc 01 §Watcher Composition + §Streaming Anomaly Detection are
    design-only.** `CompositePattern`, `PatternStage`, `WatcherFamily`,
    `OnlineIsolationForest`, `BayesianFusionPolicy`, `CusumDetector` all
    grep-negative. The "Implementation: Built" banner should apply only
    to the 10-watcher catalog + `WorstSeverityPolicy`.

14. **Doc 08 Good Regulator machinery is entirely design-only.**
    `SelfModelAccuracy`, `BrierScoreTracker`, `ThresholdLearner`,
    `Posterior`, `ScalarKalman`, `ForwardPredictor`,
    `PrecisionWeightedUpdater` all grep-negative. Doc is self-aware
    frontier, LOW severity.

15. **Doc 14 catalog references absent enum variants.** `TomlParsing` /
    `RetryWithFix` / `ImportNotFound` do not exist as `ErrorCategory` or
    `SuggestedIntervention` variants; closest real names are
    `DependencyError` / `RetryWithContext` / `ImportError`. Catalog was
    written against aspirational API (D.17).

---

## Verification smoke-tests (self-check)

1. **Structural parity with section 06** — `ls tmp/docs-parity/07/`
   mirrors `tmp/docs-parity/06/` 1:1 with the addition of `BATCHES.md` and
   `context-pack/`.
2. **Item-count consistency** — sum of per-file DONE/PARTIAL/NOT DONE
   (67 / 37 / 29 = 133) equals the 00-INDEX overall totals.
3. **Anchor integrity** — every `crates/roko-conductor/src/*.rs:N`
   reference in letter files and SOURCE-INDEX points to an existing line.
4. **Grep-negative spot check** — every grep-negative identifier in
   SOURCE-INDEX re-runs as zero hits today (`rg 'Yerkes'` = 0,
   `rg 'LearnedConductorPolicy'` = 0, `rg 'ParameterCascade'` = 0,
   `rg 'ConductorLevel'` = 0, etc.).
5. **HIGH-item count** — four HIGH items (C.09, D.11, E.05, E.14) cross-
   reference across at least two audit paths (letter files + context-pack
   gaps-summary).

---

## Deferred to future re-verification

- Re-running every file:line anchor with ±2 line drift check
- Cross-checking test-count claims (counted symbolically; not all tests
  run)
- Confirming every HIGH-severity gap from a fresh
  `cargo test -p roko-conductor` run
- Reconciling the C-file section-summary table (lists 11 items under
  "DONE" count of 10; actual status tags show 12 DONE / 3 PARTIAL / 2 NOT
  DONE — status tags treated as authoritative)

Those passes belong to the re-verification audit, not the initial
generation.

---

*End of initial generation log.*
