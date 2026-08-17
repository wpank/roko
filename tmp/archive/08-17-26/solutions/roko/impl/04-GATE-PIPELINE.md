# Implementation Plan: Gate Pipeline Convergence and Intelligence

Converges 4 gate dispatch paths into GateService, fixes stub verdicts, wires the
full adaptive intelligence stack (SPC/CUSUM/Hotelling), failure classification,
LLM judge via CascadeRouter, anti-pattern pre-gates, process reward model,
custom gates from config, and gate events for UX.

**Scope**: `crates/roko-gate/`, `crates/roko-core/src/foundation.rs`,
`crates/roko-cli/src/run.rs`, `crates/roko-cli/src/orchestrate.rs`,
`crates/roko-acp/src/runner.rs`, `crates/roko-runtime/src/workflow_engine.rs`

**Source docs**: `20-GATE-AUDIT.md`, `20-GATE-GOALS.md`, `20-GATE-ISSUES.md`,
`20-GATE-PLAN.md`, `14-GATE-VIZ-00-System-Overview.md`, `14-GATE-VIZ-08-Migration-And-Orchestration.md`

**Estimated total effort**: 30-45 tasks across 8 phases, ~15-22 engineering days.

---

## Phase 1: Extend GateConfig and GateReport (Foundation)

These tasks extend the shared types in `roko-core` that all downstream
changes depend on. No behavioral changes -- purely additive struct fields.

---

### TASK-G01: Add complexity and prior_failures to GateConfig
**Priority**: P0
**Category**: rearchitecture
**Files**: `crates/roko-core/src/foundation.rs`
**What**: Add two optional fields to `GateConfig` so GateService can perform rung
selection internally instead of requiring each caller to implement it independently.
Currently each dispatch path either hardcodes its gate list or re-implements
rung selection logic from `rung_selector.rs`. Making complexity and
prior_failures part of GateConfig lets GateService drive rung selection for all
callers.
**Steps**:
1. Add `pub complexity: Option<PlanComplexity>` to `GateConfig` (import from `roko-gate::rung_selector`; or use a u8/string if cross-crate dep is unwanted and map in GateService)
2. Add `pub prior_failures: Option<u32>` to `GateConfig`
3. Default both to `None` with `#[serde(default)]`
4. Update all existing `GateConfig` construction sites to include the new fields as `None` (search: `GateConfig {` across workspace)
5. Run `cargo check --workspace`
**Acceptance**: `cargo check --workspace` passes. All existing GateConfig construction sites compile without changes (fields are Option with serde default). No behavioral change.
**Depends on**: none
**Effort**: S

---

### TASK-G02: Add feedback and failure_classification to GateReport
**Priority**: P0
**Category**: rearchitecture
**Files**: `crates/roko-core/src/foundation.rs`
**What**: Extend GateReport with structured feedback and failure classification
so callers get agent-ready feedback without calling `feedback_for_agent()`
themselves. Currently only `orchestrate.rs` calls `feedback_for_agent()`;
`roko run` and ACP get no structured feedback (I-4, AP-7).
**Steps**:
1. Add `pub feedback: Option<GateFeedback>` to `GateReport` (use `serde_json::Value` or a new simplified `GateReportFeedback` if cross-crate dep on `roko-gate` is undesirable)
2. Add `pub failure_classification: Option<GateFailureClassification>` (same consideration)
3. Alternative: define lightweight `GateReportFeedback` and `GateReportFailureClass` structs in `foundation.rs` that the gate crate populates
4. Add `#[serde(default, skip_serializing_if = "Option::is_none")]` to both fields
5. Update `GateReport::all_passed()` and `GateReport::first_failure()` -- no changes needed, they operate on verdicts
6. Run `cargo check --workspace`
**Acceptance**: `GateReport` carries optional feedback. All existing code compiles (fields are Option). No behavioral change.
**Depends on**: none
**Effort**: S

---

### TASK-G03: Add spc_alerts and joint_anomaly fields to GateReport
**Priority**: P1
**Category**: rearchitecture
**Files**: `crates/roko-core/src/foundation.rs`
**What**: Surface statistical process control alerts and Hotelling joint anomaly
status in the gate report so callers can react to distributional shifts (I-6, I-7).
Currently SPC alerts accumulate in memory and are never drained at runtime.
**Steps**:
1. Add `pub spc_alerts: Vec<SpcAlertSummary>` to `GateReport` (define `SpcAlertSummary { rung: u32, alert_type: String, detail: String }` in foundation.rs to avoid cross-crate dep on spc.rs types)
2. Add `pub joint_anomaly: Option<JointAnomalyReport>` where `JointAnomalyReport { t_squared: f64, threshold: f64, detected: bool }`
3. Default both: `spc_alerts` to empty vec, `joint_anomaly` to None
4. Run `cargo check --workspace`
**Acceptance**: GateReport compiles with new fields. Existing callers unaffected.
**Depends on**: none
**Effort**: S

---

## Phase 2: Converge Gate Dispatch Paths (I-1, I-5)

The core convergence: all 4 dispatch paths call GateService. This eliminates
duplicate implementations, fixes rung ordering (I-5), and gives all paths
adaptive thresholds, feedback, and rung selection for free.

---

### TASK-G04: Teach GateService rung selection from GateConfig
**Priority**: P0
**Category**: wiring
**Files**: `crates/roko-gate/src/gate_service.rs`
**What**: When `GateConfig.complexity` is set, GateService should use
`rung_selector::select_rungs()` to determine which gates to run, rather than
relying on the caller-provided `enabled_gates` list. This is the key change
that enables callers to delegate rung selection to GateService.
**Steps**:
1. Import `rung_selector::{select_rungs, PlanComplexity, RungCaps}` into gate_service.rs
2. In `run_gates()`, before the gate loop, check if `config.complexity` is Some
3. If so, call `select_rungs(complexity, &RungCaps::detect(&config.workdir), config.prior_failures.unwrap_or(0))`
4. Map the returned `Vec<Rung>` to gate names and use those instead of `config.enabled_gates`
5. If `config.complexity` is None, fall through to existing `ordered_gate_names()` behavior
6. Add unit test: `complexity_based_selection_matches_rung_selector`
7. Run `cargo test -p roko-gate`
**Acceptance**: When complexity is provided, GateService runs the same gates that `select_rungs()` would produce. When complexity is None, behavior is identical to before.
**Depends on**: TASK-G01
**Effort**: M

---

### TASK-G05: Generate feedback inside GateService
**Priority**: P0
**Category**: wiring
**Files**: `crates/roko-gate/src/gate_service.rs`
**What**: After running all gates, GateService generates structured feedback
and failure classification internally. This is the fix for I-4 / AP-7: gate
feedback is built at the service level, not per-caller.
**Steps**:
1. Import `feedback::feedback_for_agent` and `compile_errors::classify_gate_failure` into gate_service.rs
2. After the verdict loop completes, find the first non-skipped failing verdict
3. Call `feedback_for_agent()` on its output to produce `GateFeedback`
4. Call `classify_gate_failure()` on its output to produce `GateFailureClassification`
5. Populate `GateReport.feedback` and `GateReport.failure_classification`
6. Add unit test with a mock failing gate that verifies feedback is populated
7. Run `cargo test -p roko-gate`
**Acceptance**: `GateReport.feedback` is `Some(...)` whenever a gate fails. `GateReport.failure_classification` is `Some(...)` for compile-like failures. All existing tests pass.
**Depends on**: TASK-G02
**Effort**: M

---

### TASK-G06: Wire GateService into run.rs (replace inline gate dispatch)
**Priority**: P0
**Category**: rearchitecture
**Files**: `crates/roko-cli/src/run.rs`
**What**: Replace the `run_gate()` function in run.rs (behind `#[cfg(feature = "legacy-orchestrate")]`)
that matches on GateConfig variants with a call to `GateService::run_gates()`.
This gives `roko run` adaptive thresholds, rung ordering, and structured
feedback for free.
**Steps**:
1. Identify the `run_gate()` function (~line 2942) and its callers
2. Construct a `GateConfig` from the existing config values (workdir, enabled gates, shell gates)
3. Create a `GateService::new()` with `.with_adaptive_thresholds()` using loaded thresholds
4. Call `svc.run_gates(gate_config).await`
5. Convert `GateReport` back to whatever the caller expects (likely a `Verdict`)
6. Save updated thresholds after the gate run
7. Remove the old match-based dispatch code
8. Run `cargo test -p roko-cli` (specifically any test that exercises `roko run` gates)
**Acceptance**: `roko run` uses GateService. Gates run in rung order. Adaptive thresholds are loaded and saved. The old inline dispatch is removed.
**Depends on**: TASK-G04, TASK-G05
**Effort**: M

---

### TASK-G07: Wire GateService into ACP runner (fix I-5 rung ordering)
**Priority**: P0
**Category**: rearchitecture
**Files**: `crates/roko-acp/src/runner.rs`
**What**: Replace the `run_gates()` function (~line 1674) that hardcodes
compile -> test -> clippy order with a call to `GateService::run_gates()`.
This fixes I-5 (clippy runs after test) and gives ACP the full adaptive
threshold stack.
**Steps**:
1. Identify the `run_gates()` function and its callers in runner.rs
2. Construct a `GateConfig` with `enabled_gates: vec!["compile", "clippy", "test"]`
3. Create `GateService::new().with_adaptive_thresholds(loaded_thresholds)`
4. Call `svc.run_gates(gate_config).await`
5. Map `GateReport` back to whatever the ACP runner expects
6. Preserve the existing threshold save/load logic (path: `.roko/learn/gate-thresholds.json`)
7. Remove the old inline gate dispatch code
8. Run `cargo test -p roko-acp`
**Acceptance**: ACP uses GateService. Gates run in canonical rung order (compile -> clippy -> test). Adaptive thresholds work. Old inline dispatch is removed.
**Depends on**: TASK-G04, TASK-G05
**Effort**: M

---

### TASK-G08: Align orchestrate.rs to use GateService for rung dispatch
**Priority**: P0
**Category**: rearchitecture
**Files**: `crates/roko-cli/src/orchestrate.rs`
**What**: The orchestrate.rs path already uses the full 7-rung pipeline via
`rung_dispatch.rs`, but it constructs gates inline rather than going through
GateService. Align it to use GateService with a populated GateConfig (complexity,
prior_failures, oracle config) so all paths share one dispatch implementation.
This is the most complex migration because orchestrate.rs has the richest gate
integration (oracles, feedback injection, replan, pheromone deposition).
**Steps**:
1. In `run_gate_pipeline()`, construct a `GateConfig` with complexity, prior_failures, enabled_gates derived from the rung selector
2. Create `GateService` with adaptive thresholds
3. For oracle-dependent gates (judge, fact-check), pass oracle config through GateService (may require extending GateService or wrapping it)
4. Call `svc.run_gates(gate_config).await`
5. Use `GateReport.feedback` instead of calling `feedback_for_agent()` separately
6. Use `GateReport.failure_classification` for retry/replan decisions
7. Preserve pheromone deposition, learning feedback, episode logging
8. Run `cargo test -p roko-cli`
**Acceptance**: orchestrate.rs gates go through GateService. Feedback and classification come from GateReport. All existing orchestrate.rs gate tests pass.
**Depends on**: TASK-G04, TASK-G05, TASK-G10
**Effort**: L

---

### TASK-G09: Verify single dispatch path
**Priority**: P0
**Category**: fix
**Files**: workspace-wide
**What**: Verify that after TASK-G06, G07, G08, all gate dispatch goes through
GateService. This is the convergence checkpoint.
**Steps**:
1. `grep -rn 'fn run_gate\b' crates/roko-cli/src/run.rs --include='*.rs' | grep -v test` -- should return 0 hits or only the GateService-delegating function
2. `grep -rn 'fn run_gates\b' crates/roko-acp/src/runner.rs --include='*.rs' | grep -v test` -- should return 0 hits or only the GateService-delegating function
3. Verify GateService is the sole `GateRunner` impl: `grep -rn 'impl GateRunner' crates/ --include='*.rs' | grep -v test | grep -v target/`
4. Run `cargo test --workspace` -- all tests pass
5. Run `cargo clippy --workspace --no-deps -- -D warnings`
**Acceptance**: One GateRunner implementation. All 4 paths delegate to it. Full test suite passes.
**Depends on**: TASK-G06, TASK-G07, TASK-G08
**Effort**: S

---

## Phase 3: Fix Stub Verdicts and LLM Judge (I-2, I-3)

Fixes the two most impactful anti-patterns: stub gates that silently pass
(AP-1) and hardcoded LLM judge model (AP-5).

---

### TASK-G10: Add Verdict::skip constructor
**Priority**: P0
**Category**: fix
**Files**: `crates/roko-core/src/lib.rs` (wherever Verdict is defined)
**What**: Add a `Verdict::skip(gate, reason)` constructor that creates a verdict
with `passed: false` and a `skipped: true` marker. Currently there is no
first-class way to represent "this gate did not run" in a `Verdict` (only in
`GateVerdict`). The Verdict type needs this to fix stub verdicts in rung_dispatch.rs.
**Steps**:
1. Locate the `Verdict` struct definition (likely in `crates/roko-core/src/lib.rs` or a sub-module)
2. Add a `skipped: bool` field with `#[serde(default)]`
3. Add `pub fn skip(gate: impl Into<String>, reason: impl Into<String>) -> Self`
4. Ensure `Verdict::pass()` and `Verdict::fail()` set `skipped: false`
5. Update `to_gate_verdict()` in gate_service.rs to propagate the `skipped` field
6. Run `cargo check --workspace && cargo test --workspace`
**Acceptance**: `Verdict::skip()` exists. Skipped verdicts have `passed: false, skipped: true`. Existing code compiles.
**Depends on**: none
**Effort**: S

---

### TASK-G11: Replace stub pass with stub skip in rung_dispatch.rs
**Priority**: P0
**Category**: fix
**Files**: `crates/roko-gate/src/rung_dispatch.rs`
**What**: Change `stub_verdict()` (line 132) from `Verdict::pass()` to
`Verdict::skip()`. This is the fix for I-2 / AP-1: stub gates no longer
inflate the "all gates passed" count. Rungs 3-6 with missing inputs will
report as skipped rather than deceptively passed.
**Steps**:
1. Change `stub_verdict()` at line 132 from `Verdict::pass(gate.to_string())` to `Verdict::skip(gate, detail)`
2. Verify the 7 stub_verdict call sites still compile (they all pass a gate name and detail string)
3. Update any tests in `rung_dispatch.rs` or `crates/roko-gate/tests/rungs.rs` that assert stub verdicts pass
4. Verify `GateReport::all_passed()` in foundation.rs correctly handles skipped verdicts (it checks `v.passed && !v.skipped` -- skipped verdicts with `passed: false` will correctly not count as passed)
5. Run `cargo test -p roko-gate`
**Acceptance**: `stub_verdict()` returns a verdict with `passed: false, skipped: true`. `GateReport::all_passed()` returns false when only stub verdicts exist. Rung pipeline integration tests updated.
**Depends on**: TASK-G10
**Effort**: S

---

### TASK-G12: Route LLM judge through CascadeRouter
**Priority**: P0
**Category**: fix
**Files**: `crates/roko-cli/src/orchestrate.rs`
**What**: Replace the hardcoded `"claude-sonnet-4-20250514"` model fallback in
AgentJudgeOracle construction with a CascadeRouter lookup using a "gate-judge"
role. This fixes I-3 / AP-5: the judge model participates in routing
optimization and A/B experiments.
**Steps**:
1. Find the AgentJudgeOracle construction in orchestrate.rs (search for `claude-sonnet-4-20250514`)
2. Replace the hardcoded string with `self.cascade_router.select_model("gate-judge", task_complexity)`
3. Add a fallback chain: CascadeRouter -> config.agent.model -> "claude-sonnet-4-20250514" (last resort only)
4. Log when the last-resort fallback is used (warn level)
5. Run `cargo check -p roko-cli`
6. Verify: `grep -rn 'claude-sonnet-4-20250514' crates/ --include='*.rs' | grep -v test` returns fewer hits
**Acceptance**: LLM judge model selection goes through CascadeRouter. The hardcoded string is a last-resort fallback with a warning log.
**Depends on**: none
**Effort**: M

---

### TASK-G13: Record episode per LLM judge invocation
**Priority**: P1
**Category**: wiring
**Files**: `crates/roko-cli/src/orchestrate.rs`
**What**: Each LLM judge invocation should record an episode for cost tracking
and learning. Currently judge calls are invisible to the episode logger (I-9).
**Steps**:
1. After the AgentJudgeOracle returns a response, create an `Episode` with role "gate-judge"
2. Populate model, input/output tokens (from agent response metadata), estimated cost
3. Log via `self.episode_logger.record(episode)`
4. Add the episode caller field as "gate-judge" for filtering
5. Run `cargo check -p roko-cli`
**Acceptance**: Judge invocations appear in `.roko/episodes.jsonl` with role "gate-judge".
**Depends on**: TASK-G12
**Effort**: S

---

### TASK-G14: Add gate budget tracking
**Priority**: P1
**Category**: feature
**Files**: `crates/roko-gate/src/gate_service.rs`, `crates/roko-core/src/foundation.rs`
**What**: Track cumulative gate cost per task and cap LLM judge invocations.
Currently gate verification can consume unbounded LLM budget without visibility (I-9).
**Steps**:
1. Define `GateBudget { max_judge_invocations: u32, current_invocations: u32, max_cost_usd: f64, current_cost_usd: f64 }` in gate_service.rs or foundation.rs
2. Add `pub budget: Option<GateBudget>` to GateConfig
3. Add `GateService::with_budget(budget: GateBudget)` builder method
4. Before invoking the LLM judge gate, check `budget.can_invoke_judge()`
5. If budget exhausted, record a skipped verdict with reason "gate budget exceeded"
6. After judge invocation, update `current_invocations` and `current_cost_usd`
7. Add unit test: judge gate skipped when budget exhausted
**Acceptance**: Judge invocations respect a configurable budget cap. Budget exhaustion produces a skipped verdict, not a panic or silent pass.
**Depends on**: TASK-G02
**Effort**: M

---

## Phase 4: Wire Adaptive Intelligence (I-6, I-7, I-8)

Connects the built-but-unused statistical intelligence: SPC alert draining,
Hotelling joint anomaly detection, domain profiles, and temperament-aware
gate decisions.

---

### TASK-G15: Drain SPC alerts after pipeline run
**Priority**: P1
**Category**: wiring
**Files**: `crates/roko-gate/src/gate_service.rs`
**What**: After running all gates in `run_gates()`, drain SPC alerts from
`AdaptiveThresholds` and include them in the GateReport. Currently alerts
accumulate indefinitely in memory and are never consumed at runtime (I-6).
**Steps**:
1. After the verdict loop in `run_gates()`, acquire the adaptive threshold lock
2. Call `thresholds.drain_spc_alerts()` to get `Vec<(u32, SpcAlert)>`
3. Map each `(u32, SpcAlert)` to a `SpcAlertSummary` for GateReport
4. Populate `GateReport.spc_alerts`
5. Log each alert at `warn!` level
6. Add unit test: SPC alerts appear in GateReport after sustained shift
7. Run `cargo test -p roko-gate`
**Acceptance**: SPC alerts are drained and included in GateReport. `pending_spc_alerts` is empty after each pipeline run.
**Depends on**: TASK-G03
**Effort**: S

---

### TASK-G16: Call Hotelling observe_pipeline after full run
**Priority**: P1
**Category**: wiring
**Files**: `crates/roko-gate/src/gate_service.rs`
**What**: After all verdicts are collected, feed the pass-rate vector to
`observe_pipeline()` for joint anomaly detection. Currently Hotelling T-squared
has no runtime callers (I-7).
**Steps**:
1. After the verdict loop, compute pass rates: `verdicts.iter().filter(|v| !v.skipped).map(|v| if v.passed { 1.0 } else { 0.0 }).collect()`
2. Acquire the adaptive threshold lock
3. Call `thresholds.observe_pipeline(&pass_rates)`
4. If `thresholds.joint_anomaly_detected()`, populate `GateReport.joint_anomaly`
5. Log joint anomaly at `error!` level (this indicates systemic problems)
6. Add unit test: joint anomaly detected after simultaneous gate failures
7. Run `cargo test -p roko-gate`
**Acceptance**: `observe_pipeline()` is called after every full pipeline run. Joint anomaly detection appears in GateReport when triggered.
**Depends on**: TASK-G03
**Effort**: S

---

### TASK-G17: Instantiate domain profiles at plan start
**Priority**: P1
**Category**: wiring
**Files**: `crates/roko-gate/src/adaptive_threshold.rs`, `crates/roko-gate/src/gate_service.rs`
**What**: Initialize `AdaptiveThresholds` from the appropriate domain profile
based on agent role. Currently all agents start with neutral 0.5 priors
regardless of role (I-8). The profiles (coding, research, security) are built
but never instantiated.
**Steps**:
1. Add `AdaptiveThresholds::from_profile(profile: &ThresholdProfile) -> Self` constructor
2. Initialize EMA values from `profile.rung_priors`
3. Apply `profile.cusum_sensitivity_override` if present
4. Apply `profile.floor_multipliers` and `profile.retry_multipliers`
5. Add `GateService::with_domain_profile(profile_name: &str)` that selects the profile by name and initializes thresholds from it
6. Map common role names to profiles: "implementer"/"coder" -> coding, "researcher" -> research, "auditor"/"security" -> security
7. Add unit test: `from_profile(ThresholdProfile::coding())` starts with coding priors
8. Run `cargo test -p roko-gate`
**Acceptance**: `from_profile()` creates thresholds with domain-informed priors. GateService can be configured with a domain profile name.
**Depends on**: none
**Effort**: M

---

### TASK-G18: Wire temperament into GateService skip decisions
**Priority**: P1
**Category**: wiring
**Files**: `crates/roko-gate/src/gate_service.rs`
**What**: GateService should accept a `Temperament` parameter and use
temperament-aware skip and retry methods from AdaptiveThresholds. Currently
the temperament-aware methods exist (AGT-06) but are never called from
GateService.
**Steps**:
1. Add `temperament: Option<Temperament>` field to `GateService`
2. Add `GateService::with_temperament(t: Temperament)` builder
3. In `should_skip_rung_adaptively()`, use `thresholds.should_skip_rung_for_temperament(rung, self.temperament.unwrap_or(Temperament::Balanced))` instead of `thresholds.should_skip_rung(rung)`
4. Expose `suggested_max_retries_for_temperament()` via a method on GateService for callers that need retry count
5. Add unit test: Conservative temperament never skips, Aggressive skips earlier
6. Run `cargo test -p roko-gate`
**Acceptance**: GateService uses temperament-aware skip decisions. Conservative agents face stricter gates.
**Depends on**: none
**Effort**: S

---

### TASK-G19: Wire residual-based threshold tightening
**Priority**: P2
**Category**: wiring
**Files**: `crates/roko-gate/src/gate_service.rs`
**What**: After each gate run, call `observe_residual()` on AdaptiveThresholds
when an oracle prediction residual is available. Currently the
residual-based tightening logic (TA-15) exists but is never fed real data.
**Steps**:
1. Add optional `oracle_prediction: Option<f64>` to GateConfig or a new `GateRunContext`
2. After each gate verdict, if `oracle_prediction` is available, compute residual = prediction - actual
3. Call `thresholds.observe_residual(rung, residual)`
4. Log residual observations for debugging
5. Add unit test: threshold tightens after consistent over-prediction
**Acceptance**: Residual-based tightening is connected when oracle predictions are available.
**Depends on**: TASK-G04
**Effort**: S

---

## Phase 5: Failure Classification and Remediation (I-10)

Wires the existing failure classification system into the retry/replan
decision loop so different failure types get different remediation actions.

---

### TASK-G20: Route by GateFailureAction in orchestrate.rs
**Priority**: P1
**Category**: wiring
**Files**: `crates/roko-cli/src/orchestrate.rs`
**What**: After a gate failure, use `GateReport.failure_classification.recommended_action`
to determine the next step instead of always retrying. Currently the
orchestrator always retries with feedback regardless of whether the failure
classification says NeedsReplan or NeedsHuman (I-10).
**Steps**:
1. After receiving a GateReport with a failure, read `report.failure_classification`
2. Match on `recommended_action`:
   - `Retry` -> existing retry-with-feedback logic
   - `NeedsReplan` -> call `build_gate_failure_plan_revision()` immediately (skip retries)
   - `Blocked` -> pause the task, emit a blocking event, log at error level
   - `NeedsHuman` -> escalate to human (stop retrying, emit escalation event)
3. Add a fallback for when `failure_classification` is None: use existing retry behavior
4. Log the chosen action and reason at info level
5. Run `cargo test -p roko-cli`
**Acceptance**: Different failure types get different remediation actions. NeedsReplan skips retries and triggers replan. NeedsHuman stops retrying. Tests verify the routing.
**Depends on**: TASK-G05
**Effort**: M

---

### TASK-G21: Wire error_patterns into learning subsystem
**Priority**: P1
**Category**: wiring
**Files**: `crates/roko-cli/src/orchestrate.rs`, `crates/roko-learn/src/lib.rs`
**What**: After each gate failure, extract error patterns from the failure
classification and record them in the error pattern store. Currently
`error_patterns.rs` exists but is not fed runtime data.
**Steps**:
1. Import `roko_gate::error_patterns::records_from_classification` (or equivalent)
2. After each gate failure classification, extract failure pattern records
3. Write patterns to the error pattern store (`.roko/learn/error-patterns.jsonl` or equivalent)
4. Periodically query top-K frequent patterns for prompt enrichment
5. Wire frequent patterns into `PromptSpec.anti_patterns` for the next agent dispatch
6. Run `cargo test --workspace`
**Acceptance**: Error patterns are recorded after gate failures. Frequent patterns appear in agent prompts as anti-pattern guidance.
**Depends on**: TASK-G20
**Effort**: M

---

### TASK-G22: Wire failure classification feedback into `roko run` path
**Priority**: P1
**Category**: wiring
**Files**: `crates/roko-cli/src/run.rs`
**What**: The `roko run` path should also use failure classification from
GateReport to decide whether to retry. Currently `roko run` has no
structured retry logic at all.
**Steps**:
1. After `GateService::run_gates()` in run.rs, check `report.failure_classification`
2. If `NeedsReplan` or `NeedsHuman`, surface the classification in the CLI output
3. If `Retry`, inject `report.feedback` into the agent's next prompt context
4. If `Blocked`, print a clear error message and exit with non-zero
5. Add integration test: `roko run` with a failing gate shows structured feedback
**Acceptance**: `roko run` displays structured failure information and takes appropriate action based on classification.
**Depends on**: TASK-G06, TASK-G05
**Effort**: M

---

## Phase 6: Process Reward Model Integration (I-12)

Connects the PRM to the orchestrator loop for early termination and
strategy change decisions.

---

### TASK-G23: Record TurnSnapshot after each gate pipeline run
**Priority**: P2
**Category**: wiring
**Files**: `crates/roko-cli/src/orchestrate.rs`
**What**: After each gate pipeline run in the agent retry loop, record a
`TurnSnapshot` with the highest rung reached, verdicts, error count, and
diff lines. Currently the PRM is built but not instantiated (I-12).
**Steps**:
1. Add a `prm: ProcessRewardModel` field to PlanTracker (or the appropriate per-task state)
2. After each gate run, compute: highest_passing_rung from verdicts, error_count from feedback, diff_lines from agent output
3. Construct `TurnSnapshot { rung, verdicts, error_count, diff_lines }`
4. Push the snapshot to `prm.history`
5. Run `cargo check -p roko-cli`
**Acceptance**: TurnSnapshots accumulate during task execution. PRM history has one entry per gate pipeline run.
**Depends on**: none
**Effort**: M

---

### TASK-G24: Compute and log Promise/Progress signals
**Priority**: P2
**Category**: wiring
**Files**: `crates/roko-cli/src/orchestrate.rs`
**What**: After recording each TurnSnapshot, compute Promise (probability of
eventual success) and Progress (delta from previous turn). Log both to
efficiency events for learning and dashboard visibility.
**Steps**:
1. After pushing TurnSnapshot, call `prm.promise()` and `prm.progress()`
2. Log to efficiency events: `efficiency_logger.log_prm(task_id, turn, promise, progress)`
3. Include promise/progress in the episode metadata
4. Add unit test: promise and progress values are reasonable for known trajectories
**Acceptance**: Promise/Progress values appear in efficiency event logs after each gate run.
**Depends on**: TASK-G23
**Effort**: S

---

### TASK-G25: Act on PRM signals for early termination
**Priority**: P2
**Category**: feature
**Files**: `crates/roko-cli/src/orchestrate.rs`
**What**: Use Promise and Progress signals to drive early termination and
strategy changes. Low Promise triggers task abandonment with replan. Stalled
Progress triggers model/strategy change via CascadeRouter.
**Steps**:
1. Define configurable thresholds: `ABANDON_THRESHOLD` (default 0.1), `STALL_THRESHOLD` (default 0.01)
2. After computing promise: if `promise < ABANDON_THRESHOLD`, abandon the task and trigger replan
3. After computing progress: if `progress < STALL_THRESHOLD` for 3+ consecutive turns, trigger model switch via CascadeRouter
4. Log the decision at info level with promise/progress values
5. Make thresholds configurable via roko.toml `[learning]` section
6. Add integration test: task with declining promise is abandoned after threshold
**Acceptance**: Tasks with low promise are abandoned early. Stalled tasks trigger model switches. Thresholds are configurable.
**Depends on**: TASK-G24
**Effort**: M

---

## Phase 7: Anti-Pattern Pre-Gates and Acceptance Contracts

Wires the built-but-unused acceptance contract system and adds pre-gate
checks that catch common anti-patterns before running expensive gates.

---

### TASK-G26: Wire AcceptanceContract as pre-gate check
**Priority**: P2
**Category**: wiring
**Files**: `crates/roko-gate/src/gate_service.rs`, `crates/roko-gate/src/acceptance_contract.rs`
**What**: Before running the gate pipeline, validate agent output against
the task's AcceptanceContract if one exists. The NoStubRequirement catches
AP-1 at the structural level before gates even run. Currently
AcceptanceContract is fully built but not wired (I-13).
**Steps**:
1. Add `pub acceptance_contract: Option<AcceptanceContract>` to GateConfig
2. At the start of `run_gates()`, if contract is present, call `contract.validate_shape()`
3. If shape validation fails, return immediately with a failing GateReport
4. After collecting all verdicts, call `contract.evaluate(evidence)` to check requirements
5. If any required requirement fails, mark the overall GateReport as failed
6. Add unit test: NoStubRequirement detects stub output
**Acceptance**: AcceptanceContract is checked when provided in GateConfig. NoStubRequirement prevents stub pass from reaching callers.
**Depends on**: TASK-G04
**Effort**: M

---

### TASK-G27: Add anti-pattern pre-gate to detect common mistakes
**Priority**: P2
**Category**: feature
**Files**: `crates/roko-gate/src/gate_service.rs` (or new `pre_gate.rs`)
**What**: Add a lightweight pre-gate check that scans agent output for common
anti-patterns before running expensive gates. This catches issues like
`unimplemented!()`, `todo!()`, empty function bodies, and `#[allow(unused)]`
blanket suppression that would pass compile but indicate incomplete work.
**Steps**:
1. Define `AntiPatternPreGate` with a configurable list of patterns to check
2. Default patterns: `unimplemented!()`, `todo!()`, `panic!("not implemented")`, empty fn bodies, `#[allow(unused)]`
3. Run as the first step in `run_gates()` before any compilation
4. Pattern match against the diff (if available) or file content
5. If anti-patterns found, fail with a descriptive message (not a hard block -- configurable severity)
6. Add GateConfig option: `pre_gate_patterns: Vec<String>` for user-configurable patterns
7. Add unit test: pre-gate catches `todo!()` in diff
**Acceptance**: Anti-pattern pre-gate runs before compile. Common stub patterns are caught early with descriptive feedback.
**Depends on**: TASK-G04
**Effort**: M

---

### TASK-G28: Wire DiffGate as anti-stub verification
**Priority**: P2
**Category**: wiring
**Files**: `crates/roko-gate/src/gate_service.rs`
**What**: Wire the existing DiffGate into the standard gate pipeline. DiffGate
analyzes diffs for vacuous/stub changes. Currently it is built and tested but
only available as a standalone gate.
**Steps**:
1. Register DiffGate in `gate_for_name()` for the "diff" gate name (already partially done -- verify it works with real diffs)
2. Ensure DiffGate receives the actual git diff as input (not just `git diff --stat` from ShellGate)
3. Add diff content extraction: run `git diff HEAD` and pass the output to DiffGate
4. DiffGate should flag: empty diffs, comment-only changes, import-only changes
5. Add integration test: DiffGate detects vacuous changes
**Acceptance**: DiffGate runs as part of the standard pipeline. Vacuous/stub diffs are flagged.
**Depends on**: TASK-G04
**Effort**: S

---

## Phase 8: Custom Gates from Config

Enables users to define arbitrary verification gates in roko.toml.

---

### TASK-G29: Define custom gate config schema in roko.toml
**Priority**: P2
**Category**: feature
**Files**: `crates/roko-core/src/config/mod.rs` (or the roko.toml config module)
**What**: Define the TOML schema for user-defined custom gates: shell commands,
HTTP health checks, with configurable timeouts and rung placement.
**Steps**:
1. Define `CustomGateConfig { name: String, program: String, args: Vec<String>, timeout_ms: u64, rung: Option<u8>, fail_on_stderr: bool }`
2. Define `HttpGateConfig { name: String, url: String, method: String, expected_status: u16, timeout_ms: u64, rung: Option<u8> }`
3. Add `[[gate.custom]]` array to the roko.toml config schema
4. Parse custom gates during config loading
5. Validate: no duplicate names, no conflict with built-in gate names
6. Add unit test: parse sample custom gate config
**Acceptance**: Custom gates are parseable from roko.toml. Config validation catches duplicates and conflicts.
**Depends on**: none
**Effort**: M

---

### TASK-G30: Construct custom gates in GateService from config
**Priority**: P2
**Category**: feature
**Files**: `crates/roko-gate/src/gate_service.rs`
**What**: GateService constructs and runs custom gates defined in roko.toml
alongside built-in gates, respecting the rung ordering.
**Steps**:
1. Add `custom_gates: HashMap<String, CustomGateConfig>` field to GateService
2. Add `GateService::with_custom_gates(gates: Vec<CustomGateConfig>)` builder
3. In `gate_for_name()`, check `custom_gates` map before returning None for unknown names
4. Construct `ShellGate::new(config.program, config.args).with_timeout_ms(config.timeout_ms)` for each custom gate
5. In `rung_for_name()`, use `config.rung` or default to 5 (custom slot) for custom gates
6. Custom gates participate in rung ordering via their configured rung
7. Add integration test: custom gate runs at its configured rung position
**Acceptance**: Custom gates from roko.toml run in the pipeline at their configured rung. They appear in GateReport.
**Depends on**: TASK-G29, TASK-G04
**Effort**: M

---

### TASK-G31: Custom gates participate in adaptive thresholds
**Priority**: P2
**Category**: wiring
**Files**: `crates/roko-gate/src/gate_service.rs`
**What**: Custom gates should have their pass/fail outcomes recorded in
AdaptiveThresholds and benefit from the same skip decisions as built-in gates.
**Steps**:
1. When running a custom gate, use its configured rung (or assign a dynamic rung index starting at 7)
2. After custom gate execution, call `thresholds.observe(rung, passed)` with the assigned rung
3. Custom gates with high consecutive-pass streaks are skipped (unless rung 0)
4. Add unit test: custom gate with 20+ consecutive passes is skipped
**Acceptance**: Custom gates are tracked by AdaptiveThresholds and benefit from adaptive skip logic.
**Depends on**: TASK-G30
**Effort**: S

---

### TASK-G32: HTTP health check gate type
**Priority**: P2
**Category**: feature
**Files**: `crates/roko-gate/src/http_gate.rs` (new)
**What**: Implement an HTTP health check gate that verifies a running service
responds with an expected status code. This is a new gate type for custom
gate configs that use `url` instead of `program`.
**Steps**:
1. Create `crates/roko-gate/src/http_gate.rs`
2. Define `HttpGate { url: String, method: String, expected_status: u16, timeout: Duration }`
3. Implement `Verify` for HttpGate: send HTTP request, check status code
4. Implement `Cell` for HttpGate
5. Register in GateService: when custom gate config has `url` field, construct HttpGate instead of ShellGate
6. Add unit test with a local test server
7. Add to `lib.rs` module list
**Acceptance**: HTTP gates can be defined in roko.toml. They verify service health by checking status codes.
**Depends on**: TASK-G30
**Effort**: M

---

## Phase 9: Gate Events for UX

Adds rich event emission from GateService for real-time TUI and SSE visibility.

---

### TASK-G33: Extend RuntimeEvent with richer gate events
**Priority**: P1
**Category**: feature
**Files**: `crates/roko-core/src/runtime_event.rs`
**What**: Extend the existing gate events in RuntimeEvent with additional
variants for skipped gates, threshold updates, SPC alerts, and pipeline
completion. The existing events (GateStarted, GatePassed, GateFailed) are
minimal.
**Steps**:
1. Add `GateSkipped { run_id: String, gate_name: String, rung: u8, reason: String }` variant
2. Add `GateThresholdUpdated { run_id: String, rung: u8, old_threshold: f64, new_threshold: f64, trigger: String }` variant
3. Add `GateSpcAlert { run_id: String, rung: u8, alert_type: String, detail: String }` variant
4. Add `GatePipelineCompleted { run_id: String, passed: bool, duration_ms: u64, gates_run: u32, gates_skipped: u32 }` variant
5. Update `RuntimeEvent::run_id()` and `RuntimeEvent::kind()` for new variants
6. Run `cargo check --workspace` (all match arms must be exhaustive)
**Acceptance**: RuntimeEvent has the full set of gate event variants. All existing code compiles after adding new arms to match expressions.
**Depends on**: none
**Effort**: M

---

### TASK-G34: Add event sink to GateService
**Priority**: P1
**Category**: feature
**Files**: `crates/roko-gate/src/gate_service.rs`
**What**: GateService should accept an event sink (mpsc channel or EventBus
reference) and emit gate events during execution. This enables real-time
gate progress in TUI and SSE consumers.
**Steps**:
1. Add `event_sink: Option<tokio::sync::mpsc::Sender<RuntimeEvent>>` field to GateService
2. Add `GateService::with_event_sink(sink)` builder method
3. Before each gate: emit `GateStarted { gate_name, rung }`
4. After each gate pass: emit `GatePassed { gate_name, duration_ms }`
5. After each gate fail: emit `GateFailed { gate_name, output, duration_ms }`
6. After each gate skip: emit `GateSkipped { gate_name, rung, reason }`
7. After pipeline completion: emit `GatePipelineCompleted { passed, duration_ms, gates_run, gates_skipped }`
8. Use `try_send()` to avoid blocking on slow consumers
9. Add unit test: events emitted in correct order for a 3-gate pipeline
**Acceptance**: GateService emits RuntimeEvents during execution. Events arrive in correct order. Slow consumers do not block gate execution.
**Depends on**: TASK-G33
**Effort**: M

---

### TASK-G35: Emit SPC alert and threshold update events
**Priority**: P2
**Category**: feature
**Files**: `crates/roko-gate/src/gate_service.rs`
**What**: After draining SPC alerts (TASK-G15), emit RuntimeEvent variants for
each alert. After adaptive threshold changes, emit threshold update events.
These feed the dashboard threshold update notifications.
**Steps**:
1. After draining SPC alerts, for each alert emit `GateSpcAlert { rung, alert_type, detail }`
2. Before and after `thresholds.observe()`, capture the EMA value
3. If EMA changed significantly (delta > 0.05), emit `GateThresholdUpdated { rung, old, new, trigger }`
4. Run `cargo test -p roko-gate`
**Acceptance**: SPC alerts and threshold changes produce RuntimeEvents that TUI and SSE consumers can render.
**Depends on**: TASK-G15, TASK-G34
**Effort**: S

---

### TASK-G36: Wire gate events to workflow engine EventBus
**Priority**: P1
**Category**: wiring
**Files**: `crates/roko-runtime/src/workflow_engine.rs`, `crates/roko-runtime/src/effect_driver.rs`
**What**: The workflow engine's EffectDriver already uses GateService and emits
some RuntimeEvents. Wire the new event sink from GateService into the
workflow engine's EventBus so gate events flow to all consumers (TUI, SSE, JSONL logger).
**Steps**:
1. In EffectDriver, when constructing GateService, create an mpsc channel
2. Pass the sender to `GateService::with_event_sink()`
3. Spawn a forwarding task that reads from the receiver and publishes to EventBus
4. Verify the existing RuntimeEvent handlers in the TUI bridge handle the new variants (add match arms)
5. Run `cargo test -p roko-runtime`
**Acceptance**: Gate events from GateService flow through EventBus to all consumers. TUI shows real-time gate progress.
**Depends on**: TASK-G34
**Effort**: M

---

### TASK-G37: Wire gate events to orchestrate.rs EventBus
**Priority**: P1
**Category**: wiring
**Files**: `crates/roko-cli/src/orchestrate.rs`
**What**: Wire GateService's event sink into the orchestrate.rs event
infrastructure so `roko plan run` emits gate events to the TUI dashboard.
**Steps**:
1. When constructing GateService in orchestrate.rs (after TASK-G08), create an mpsc channel
2. Pass the sender to `GateService::with_event_sink()`
3. Forward received events to the orchestrate.rs EventBus (or DashboardEvent emitter)
4. Verify gate events appear in the TUI during `roko plan run`
5. Run `cargo test -p roko-cli`
**Acceptance**: `roko plan run` emits gate events visible in the TUI dashboard.
**Depends on**: TASK-G34, TASK-G08
**Effort**: M

---

## Phase 10: Gate Composition at Runtime

Wires the built-but-unused composition wrappers for runtime use.

---

### TASK-G38: Construct ComposedGatePipeline from roko.toml config
**Priority**: P2
**Category**: feature
**Files**: `crates/roko-gate/src/gate_service.rs`, `crates/roko-core/src/config/mod.rs`
**What**: Allow users to configure gate composition mode (sequential, parallel,
voting, fallback) in roko.toml. Currently ComposedGatePipeline exists with 4
modes but is only used in tests (I-11).
**Steps**:
1. Add `[gate.pipeline] mode = "sequential"` to roko.toml schema
2. Parse composition config: mode, voting threshold, fallback groups
3. In GateService, when composition mode is not "sequential", construct ComposedGatePipeline
4. For voting mode: construct VotingGate with configured threshold
5. For fallback mode: construct FallbackGate with configured primary/fallback groups
6. Run composed pipeline through the standard GateReport interface
7. Add integration test: voting gate with 2-of-3 judges
**Acceptance**: Gate composition mode is configurable. Non-sequential modes work at runtime.
**Depends on**: TASK-G04
**Effort**: M

---

### TASK-G39: Deprecate GatePipeline in favor of ComposedGatePipeline
**Priority**: P2
**Category**: rearchitecture
**Files**: `crates/roko-gate/src/gate_pipeline.rs`
**What**: ComposedGatePipeline in Sequential mode re-implements the loop from
GatePipeline rather than delegating (I-11, dead code at lines 408-435).
Clean up the duplication.
**Steps**:
1. Make ComposedGatePipeline::Sequential delegate to GatePipeline internally
2. OR deprecate GatePipeline and migrate all callers to ComposedGatePipeline
3. Remove the `let _ = pipeline;` dead code in the Sequential arm
4. Update all test imports
5. Run `cargo test -p roko-gate`
**Acceptance**: No duplicated sequential pipeline logic. One implementation for sequential gate execution.
**Depends on**: none
**Effort**: S

---

## Phase 11: Novel Gate Types

Adds new gate types that go beyond compile/test to semantic verification.

---

### TASK-G40: Incremental compile gate (package-scoped)
**Priority**: P2
**Category**: feature
**Files**: `crates/roko-gate/src/compile.rs`
**What**: Add package-scoped compilation to CompileGate so it can run
`cargo check -p <package>` instead of `cargo check --workspace`. This reduces
gate time from 3-8 minutes to 10-30 seconds for targeted changes (VS-2).
**Steps**:
1. Add `CompileGate::cargo_package(package: &str)` constructor that passes `--package <package>` as extra args
2. Add `CompileGate::cargo_packages(packages: &[String])` for multiple packages
3. Add a helper function `affected_crates_from_diff(workdir: &Path) -> Vec<String>` that parses `git diff --name-only` to identify affected crates
4. In GateService, when available, use affected crates for incremental compilation
5. Fall back to full workspace check if affected crate detection fails
6. Add unit test: `cargo_package()` passes `--package` flag
**Acceptance**: Incremental compilation runs only affected crates. Falls back to workspace check when needed.
**Depends on**: none
**Effort**: M

---

### TASK-G41: Flaky test detection and retry
**Priority**: P2
**Category**: feature
**Files**: `crates/roko-gate/src/test_gate.rs`
**What**: Add flaky test detection to TestGate. When a test fails, retry it
up to N times. If it passes on retry, mark it as flaky (pass with warning)
rather than failing the gate (VS-4).
**Steps**:
1. Add `TestGate::with_flaky_detection(retry_count: u32)` builder
2. In `verify()`, when tests fail and flaky detection is enabled, re-run only the failing tests
3. If any fail then pass on retry, mark as flaky in the verdict detail
4. Track flaky test names in `.roko/learn/flaky-tests.json` for persistent quarantine
5. After N flaky occurrences, auto-quarantine the test (exclude from future runs)
6. Make retry count configurable via roko.toml `[gate] flaky_retries = 2`
7. Add unit test: flaky test is detected and passes with warning
**Acceptance**: Flaky tests are detected and retried. Persistent quarantine prevents repeated false failures.
**Depends on**: none
**Effort**: M

---

### TASK-G42: Semantic correctness gate via EvalGenerator
**Priority**: P2
**Category**: feature
**Files**: `crates/roko-gate/src/semantic_gate.rs` (new)
**What**: Use the existing EvalGenerator to create task-specific verification
checks from acceptance criteria. This is a richer version of the LLM judge
that uses structured criteria rather than free-form prompting.
**Steps**:
1. Create `crates/roko-gate/src/semantic_gate.rs`
2. Define `SemanticCorrectnessGate { oracle: Arc<dyn JudgeOracle>, acceptance_criteria: Vec<String> }`
3. Implement `Verify`: for each acceptance criterion, generate a focused prompt asking if the diff satisfies it
4. Score each criterion 0.0-1.0, require average score >= configurable threshold
5. Implement `Cell` for SemanticCorrectnessGate
6. Register as an optional gate in GateService ("semantic" gate name, rung 6)
7. Add to `lib.rs` module list
8. Add unit test with mock oracle
**Acceptance**: Semantic gate scores diffs against acceptance criteria. Results appear in GateReport.
**Depends on**: none
**Effort**: M

---

### TASK-G43: Dependency analysis gate
**Priority**: P2
**Category**: feature
**Files**: `crates/roko-gate/src/dependency_gate.rs` (new)
**What**: Verify dependency hygiene after changes: check for new deps with
security advisories, circular dependencies, and vendored code duplication.
**Steps**:
1. Create `crates/roko-gate/src/dependency_gate.rs`
2. Define `DependencyGate { baseline_lockfile: Option<PathBuf> }`
3. Implement `Verify`: parse `Cargo.lock` diff, identify new/changed deps
4. Run `cargo audit` on new dependencies (if `cargo-audit` is installed)
5. Check for circular dependency introduction
6. Report findings as structured verdicts
7. Implement `Cell` for DependencyGate
8. Add to `lib.rs` module list
9. Register in GateService as optional gate ("dependency", rung 3)
**Acceptance**: Dependency gate detects new deps with advisories and circular deps.
**Depends on**: none
**Effort**: M

---

## Cross-Cutting Concerns

---

### TASK-G44: Comprehensive gate pipeline integration test
**Priority**: P0
**Category**: fix
**Files**: `crates/roko-gate/tests/gate_truth.rs`
**What**: Add an end-to-end integration test that exercises the converged
GateService with all features: adaptive thresholds, SPC alerts, Hotelling,
feedback generation, failure classification, event emission, and custom gates.
This is the acceptance test for the full pipeline convergence.
**Steps**:
1. Create a test that constructs GateService with: adaptive thresholds, event sink, custom gates, temperament
2. Run a GateConfig with compile + clippy + test + custom gate
3. Verify: gates run in rung order, feedback is populated, SPC alerts are drained, events are emitted
4. Verify: adaptive thresholds are updated, Hotelling is called
5. Verify: custom gates participate in adaptive tracking
6. Run with a failing gate: verify failure classification is populated
7. Run with all passes: verify `GateReport::all_passed()` is true
**Acceptance**: Integration test exercises all converged features in a single test run.
**Depends on**: TASK-G05, TASK-G15, TASK-G16, TASK-G34
**Effort**: M

---

### TASK-G45: Update GAPS.md with remaining gate work
**Priority**: P0
**Category**: fix
**Files**: `.roko/GAPS.md`
**What**: After implementing each phase, log remaining items to GAPS.md per
the project's gap tracking convention.
**Steps**:
1. After each phase completion, assess what was not finished
2. Append to `.roko/GAPS.md` with: what's missing, why, which subsystem
3. Include: eval framework migration (from 14-GATE-VIZ-08), wave-level aggregation (VS-1), cross-task regression (VS-3)
**Acceptance**: GAPS.md reflects the current state of gate pipeline work after each phase.
**Depends on**: all phases
**Effort**: S

---

## Dependency Graph

```
Phase 1 (G01-G03): Foundation types
  |
  +---> Phase 2 (G04-G09): Converge dispatch paths
  |       |
  |       +---> Phase 5 (G20-G22): Failure classification routing
  |       |
  |       +---> Phase 7 (G26-G28): Anti-pattern pre-gates
  |       |
  |       +---> Phase 8 (G29-G32): Custom gates from config
  |       |
  |       +---> Phase 10 (G38-G39): Gate composition
  |
  +---> Phase 3 (G10-G14): Fix stubs and LLM judge
  |
  +---> Phase 4 (G15-G19): Wire adaptive intelligence
  |       |
  |       +---> Phase 9 (G33-G37): Gate events for UX
  |
  +---> Phase 6 (G23-G25): Process reward model
  |
  +---> Phase 11 (G40-G43): Novel gate types (independent)
```

Phases 1-3 are P0 (critical path).
Phases 4-5 are P1 (high value).
Phases 6-11 are P2 (incremental value, can be done in any order).

---

## Priority Summary

| Priority | Tasks | Description |
|----------|-------|-------------|
| P0 | G01-G14, G44-G45 | Converge dispatch, fix stubs, fix judge, integration test |
| P1 | G15-G18, G20-G22, G33-G34, G36-G37 | Adaptive intelligence, failure routing, gate events |
| P2 | G19, G23-G32, G35, G38-G43 | PRM, pre-gates, custom gates, composition, novel gates |

---

## Verification Checkpoints

After each phase, run:

```bash
# Compile
cargo check --workspace

# Format
cargo +nightly fmt --all --check

# Lint
cargo clippy --workspace --no-deps -- -D warnings

# Tests
cargo test --workspace

# Anti-pattern checks (after Phase 2):
# Only one gate dispatch implementation
grep -rn 'fn run_gate\b\|fn run_gates\b' crates/ --include='*.rs' | \
  grep -v target/ | grep -v test | grep -v gate_service

# No hardcoded model strings (after Phase 3):
grep -rn 'claude-sonnet-4-20250514' crates/ --include='*.rs' | grep -v test

# No stub pass verdicts (after Phase 3):
grep -rn 'Verdict::pass.*stub' crates/ --include='*.rs' | grep -v test

# SPC alerts drained (after Phase 4):
grep -rn 'drain_spc_alerts' crates/ --include='*.rs' | grep -v test | grep -v target/
# Should show callers in gate_service.rs

# Hotelling called at runtime (after Phase 4):
grep -rn 'observe_pipeline' crates/ --include='*.rs' | grep -v test | grep -v target/
# Should show callers in gate_service.rs
```

---

## Sources

All tasks grounded in:

- `crates/roko-gate/src/gate_service.rs` (680 LOC) -- current GateService implementation
- `crates/roko-gate/src/rung_dispatch.rs` (249 LOC) -- stub_verdict, run_rung, run_canonical_rung
- `crates/roko-gate/src/adaptive_threshold.rs` (957 LOC) -- EMA, CUSUM, SPC, Hotelling, profiles, temperament
- `crates/roko-gate/src/spc.rs` (725 LOC) -- CUSUM, EWMA, BOCPD detectors
- `crates/roko-gate/src/hotelling.rs` (439 LOC) -- joint anomaly detection
- `crates/roko-gate/src/feedback.rs` (393 LOC) -- feedback_for_agent
- `crates/roko-gate/src/compile_errors.rs` (~400 LOC) -- error classification, GateFailureAction
- `crates/roko-gate/src/process_reward.rs` (~300 LOC) -- TurnSnapshot, Promise, Progress
- `crates/roko-gate/src/acceptance_contract.rs` (~400 LOC) -- NoStubRequirement
- `crates/roko-gate/src/composition.rs` (569 LOC) -- ParallelGate, VotingGate, FallbackGate
- `crates/roko-gate/src/gate_pipeline.rs` (1118 LOC) -- ComposedGatePipeline
- `crates/roko-core/src/foundation.rs` -- GateRunner, GateConfig, GateReport, GateVerdict
- `crates/roko-core/src/runtime_event.rs` -- RuntimeEvent gate variants
- `crates/roko-cli/src/orchestrate.rs` -- gate pipeline integration in plan runner
- `crates/roko-cli/src/run.rs` -- simple gate dispatch (run_gate function)
- `crates/roko-acp/src/runner.rs` -- ACP gate dispatch (run_gates function)
- `crates/roko-runtime/src/workflow_engine.rs` -- workflow engine gate dispatch
- `tmp/solutions/roko/20-GATE-AUDIT.md` -- comprehensive subsystem audit
- `tmp/solutions/roko/20-GATE-ISSUES.md` -- issues catalog (I-1 through I-15)
- `tmp/solutions/roko/20-GATE-GOALS.md` -- goals document
- `tmp/solutions/roko/20-GATE-PLAN.md` -- phased plan
- `tmp/solutions/roko/14-GATE-VIZ-00-System-Overview.md` -- eval architecture overview
- `tmp/solutions/roko/14-GATE-VIZ-08-Migration-And-Orchestration.md` -- migration strategy
