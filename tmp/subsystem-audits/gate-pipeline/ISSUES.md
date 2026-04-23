# Gate Pipeline: Issues

Concrete problems in the gate pipeline, grounded in source code analysis and
operational experience from mega-parity batch runs (~195 parallel batches).

---

## Critical Issues

### I-1: Three Separate Gate Dispatch Paths (AP-6 Copy Between Runtimes)

**Location**: `crates/roko-cli/src/run.rs`, `crates/roko-acp/src/runner.rs`,
`crates/roko-cli/src/orchestrate.rs`

**Problem**: Gate dispatch is implemented three times with different feature sets.
The `roko run` path has 4 hardcoded gate types. The ACP path has 3 hardcoded
gates with adaptive skip. The orchestrate.rs path has the full 7-rung pipeline.

**Evidence**:
```bash
# Returns 4 separate implementations:
grep -rn 'fn run_gate\|fn run_gates\|fn run_gate_rung\|fn run_gate_pipeline' \
  crates/ --include='*.rs' | grep -v target/ | grep -v test
```

**Impact**: Feature gaps compound silently. When adaptive thresholds were added
to ACP, they were not added to `roko run`. When feedback injection was added to
orchestrate.rs, it was not added to ACP. Users get different verification behavior
depending on which entry point they use, with no indication of the difference.

**Root Cause**: Each path was built incrementally. run.rs was the first gate path.
ACP was added for the sidecar runner. orchestrate.rs was built for the full plan
executor. Nobody went back to converge them because each path had enough
functionality for its immediate use case.

**Fix**: Migrate all callers to GateService. See PLAN.md Phase 1.

---

### I-2: Stub Verdicts Give False Confidence (AP-1 Silent Pass)

**Location**: `crates/roko-gate/src/rung_dispatch.rs` lines 132-138

**Problem**: When rung inputs are missing, gates return stub verdicts that PASS:

```rust
fn stub_verdict(gate: &str, detail: impl Into<String>) -> Verdict {
    let message = format!("stub gate; {}", detail.into());
    let mut verdict = Verdict::pass(gate.to_string());  // <-- PASS
    verdict.reason.clone_from(&message);
    verdict.detail = Some(message);
    verdict
}
```

This means a plan running at Complex complexity with no SymbolManifest, no
FactCheckOracle, and no JudgeOracle will report that rungs 3-6 all passed,
when in reality they were never executed.

**Impact**: The AP-1 anti-pattern from mega-parity runs (stub gates that return
pass) is baked into the core gate dispatch. This was the single most common
source of false confidence in batch runs -- agents appeared to pass all gates
when higher rungs were simply not wired.

**Evidence**: From `tmp/solutions/runner/LESSONS.md`:
> AP-1: Stub gates that return pass (silent-pass)

**Fix**: Stub verdicts should be marked as skipped, not passed. The `GateVerdict`
struct already has `skipped: bool` and `skip_reason: Option<String>`. Stubs
should use these fields:

```rust
fn stub_verdict(gate: &str, detail: impl Into<String>) -> Verdict {
    let message = format!("stub gate; {}", detail.into());
    Verdict::skip(gate, message)  // New: Verdict::skip constructor
}
```

---

### I-3: Hardcoded LLM Judge Model Fallback (AP-5)

**Location**: `crates/roko-cli/src/orchestrate.rs` (AgentJudgeOracle construction)

**Problem**: The LLM judge oracle falls back to `"claude-sonnet-4-20250514"` when
no model is configured:

```rust
AgentJudgeOracle {
    command: self.config.agent.command.clone(),
    model: self.config.agent.model.as_deref()
               .unwrap_or("claude-sonnet-4-20250514"),
    timeout_ms: 120_000,
    skip_permissions: true,
}
```

This is the AP-5 anti-pattern from mega-parity runs: hardcoded model strings
bypass the routing system. The model string will silently break when the model
is deprecated, and it doesn't participate in CascadeRouter learning.

**Impact**:
- Judge model selection doesn't benefit from CascadeRouter optimization
- No episode recorded for the judge call (invisible cost)
- Model string hardcoded to a specific dated version
- Cannot A/B test judge models through the experiment system

**Fix**: Route through CascadeRouter with a "gate-judge" role. Record an
episode per invocation. See PLAN.md Phase 2.

---

### I-4: Gate Feedback Not Available Outside orchestrate.rs (AP-7)

**Location**: `crates/roko-gate/src/feedback.rs` (the module),
`crates/roko-cli/src/orchestrate.rs` (only caller)

**Problem**: `feedback_for_agent()` is called only from orchestrate.rs.
The `roko run` and ACP paths run gates and get verdicts but never parse the
output into structured feedback for agent retry.

**Evidence**:
```bash
grep -rn 'feedback_for_agent' crates/ --include='*.rs' | grep -v target/ | grep -v test
# Returns only orchestrate.rs
```

**Impact**: When a gate fails in `roko run` or ACP, the agent gets either:
- Raw stderr dumped into its context (noisy, wastes tokens)
- No feedback at all (agent retries blind)

The structured feedback system (errors/warnings/suggestions classification,
noise filtering) exists and works but only benefits one of three paths.

**Fix**: GateService should call `feedback_for_agent()` internally and include
the feedback in `GateReport`. See PLAN.md Phase 1.

---

## Significant Issues

### I-5: ACP Runs Clippy After Test (Rung Order Violation)

**Location**: `crates/roko-acp/src/runner.rs` (`run_gates()` function)

**Problem**: ACP runs gates in the order: compile -> test -> clippy. The canonical
rung order is: compile (0) -> clippy (1) -> test (2). Running test before clippy
means:

1. Tests may take 5-15 minutes before discovering a trivial lint failure
2. Test output pollutes the failure context when the real issue was a lint warning
3. Inconsistent behavior between ACP and other paths

**Impact**: Time waste and confusing error output. If clippy would catch the
issue in 30 seconds, there is no reason to run the full test suite first.

**Fix**: Use GateService which orders by rung index. GateService.ordered_gate_names()
already sorts by rung.

---

### I-6: SPC Alerts Accumulated But Never Drained (AP-8 Built But Unused)

**Location**: `crates/roko-gate/src/adaptive_threshold.rs` lines 446-454

**Problem**: SPC alerts from the CUSUM/EWMA/BOCPD ensemble are collected in
`pending_spc_alerts` but:
- `drain_spc_alerts()` exists but is never called from runtime code
- Alerts accumulate indefinitely in memory
- No runtime consumer reacts to statistical shifts

**Evidence**:
```bash
grep -rn 'drain_spc_alerts' crates/ --include='*.rs' | grep -v target/ | grep -v test
# Returns only the method definition in adaptive_threshold.rs
```

**Impact**: The SPC detectors fire correctly (verified by tests) but their
signals are discarded. A gradual gate degradation that CUSUM detects in 5
observations will be invisible until the EMA catches up after 20+ observations.

**Fix**: After each gate pipeline run, drain SPC alerts and:
1. Log them to efficiency events
2. Surface them in the TUI as threshold update events
3. If OutOfControl alert: tighten adaptive thresholds immediately
4. If ChangePoint detected: reset EMA to adapt faster

---

### I-7: Hotelling T-Squared Has No Runtime Callers

**Location**: `crates/roko-gate/src/hotelling.rs` (439 LOC),
`crates/roko-gate/src/adaptive_threshold.rs` `observe_pipeline()`

**Problem**: The Hotelling T-squared joint anomaly detector is fully implemented
and tested but `observe_pipeline()` is never called from runtime code. Only
unit tests in `adaptive_threshold.rs` exercise it.

**Evidence**:
```bash
grep -rn 'observe_pipeline' crates/ --include='*.rs' | grep -v target/ | grep -v test
# Returns only the method definition
```

**Impact**: Joint anomalies (multiple gates degrading simultaneously) go
undetected. This is the exact scenario where individual gate monitoring fails:
compile and test each drop 5% but the joint shift indicates a systemic problem
(e.g., bad model, corrupted environment) that neither gate alone would catch.

**Fix**: After each full pipeline run, call `observe_pipeline()` with the
pass-rate vector. If `joint_anomaly_detected()`, emit a high-priority alert.

---

### I-8: Domain Profiles Never Instantiated

**Location**: `crates/roko-gate/src/adaptive_threshold.rs` lines 67-163

**Problem**: Three domain profiles are implemented (coding, research, security)
with per-rung prior pass rates, floor multipliers, retry multipliers, and CUSUM
sensitivity overrides. None of these are ever constructed or applied at runtime.

**Evidence**:
```bash
grep -rn 'ThresholdProfile' crates/ --include='*.rs' | grep -v target/ | grep -v test
# Returns only the struct definition and methods
```

**Impact**: The adaptive threshold system starts from neutral priors (0.5) for
every rung regardless of the agent's role. A security auditor agent and a
research agent both start with the same gate expectations, when their actual
pass rate distributions are known to differ significantly.

**Fix**: At plan start, select a domain profile from the agent role config.
Apply rung priors as the initial EMA values for fresh AdaptiveThresholds.

---

### I-9: No Gate Budget Tracking

**Location**: N/A (not implemented)

**Problem**: LLM judge gate invocations have no cost tracking. Each judge
call involves a full LLM API call but:
- No episode is recorded for the judge call
- No cost is attributed to the gate verification budget
- No limit prevents runaway judge invocations during replan loops

**Impact**: Gate verification can consume unbounded LLM budget without visibility.
A task that fails and replans repeatedly will invoke the LLM judge on each
iteration with no cost accounting.

**Fix**: Record an episode per judge invocation. Track cumulative gate cost.
Cap judge invocations per task at a configurable maximum.

---

### I-10: Compile Error Classification Not Used for Agent Remediation

**Location**: `crates/roko-gate/src/compile_errors.rs` (full classification),
`crates/roko-cli/src/orchestrate.rs` (partial use)

**Problem**: The compile error classification system is sophisticated:
- 11 error categories (Syntax, UnresolvedImport, TypeMismatch, etc.)
- 12 failure classes (SyntaxError, ImportError, TypeError, etc.)
- 4 failure actions (Retry, NeedsReplan, Blocked, NeedsHuman)

But the failure action is computed and rendered but not used to drive different
remediation strategies. The orchestrator always retries with gate feedback
regardless of whether the failure classification says "NeedsReplan" or
"NeedsHuman".

**Impact**: Agents waste retries on failures that cannot be fixed by retrying.
An `ArchitecturalConflictRequiresReplan` failure should not be retried -- it
should trigger a replan event. A `NeedsHuman` failure should not be retried
at all -- it should pause and alert.

**Fix**: Wire failure classification into the retry/replan decision:

```rust
match classify_gate_failure(&output).recommended_action {
    GateFailureAction::Retry => retry_with_feedback(),
    GateFailureAction::NeedsReplan => emit_replan_event(),
    GateFailureAction::Blocked => pause_with_alert(),
    GateFailureAction::NeedsHuman => escalate_to_human(),
}
```

---

## Moderate Issues

### I-11: GatePipeline and ComposedGatePipeline Are Parallel Implementations

**Location**: `crates/roko-gate/src/gate_pipeline.rs`

**Problem**: `GatePipeline` (sequential only, with short-circuit toggle) and
`ComposedGatePipeline` (4 composition modes) are separate structs that partially
duplicate logic. The ComposedGatePipeline in Sequential mode re-implements the
loop from GatePipeline rather than delegating.

**Evidence**: Lines 408-435 of gate_pipeline.rs re-implement sequential gate
execution inline:

```rust
GateComposition::Sequential => {
    let pipeline = GatePipeline::new(&*self.name);  // Created then unused
    // ... re-implements loop ...
    let _ = pipeline;  // Dead code
}
```

**Impact**: Maintenance burden. Changes to sequential pipeline behavior must be
made in two places. The `let _ = pipeline` dead code is a tell.

**Fix**: ComposedGatePipeline should delegate Sequential mode to GatePipeline,
or GatePipeline should be deprecated in favor of ComposedGatePipeline.

---

### I-12: ProcessRewardModel Not Connected to Orchestrator

**Location**: `crates/roko-gate/src/process_reward.rs`

**Problem**: The ProcessRewardModel tracks per-turn gate snapshots and derives
Promise (probability of eventual success) and Progress (trajectory delta)
signals. These signals could drive early termination and model switching.
But the PRM is not instantiated or updated during orchestration.

**Impact**: Tasks that are clearly failing (Promise declining, Progress stalled)
continue to consume budget until they exhaust retries. The PRM would enable
earlier abandonment and strategy changes.

---

### I-13: Acceptance Contract Not Enforced

**Location**: `crates/roko-gate/src/acceptance_contract.rs`

**Problem**: The AcceptanceContract system defines formal requirements
(NoStubRequirement, ParityLedgerRequirement, etc.) with evidence collection.
This is the formal verification layer that could prevent AP-1 (stub pass)
at the architectural level. But it is not wired into the gate pipeline.

**Impact**: No formal acceptance criteria are checked. The gate pipeline
relies on individual gate implementations to be honest about pass/fail,
rather than having a meta-verification layer.

---

### I-14: Eval Generator Not Used at Runtime

**Location**: `crates/roko-gate/src/eval_generator.rs`

**Problem**: EvalGenerator can dynamically create verification checks from
templates and strategies. This could enable task-specific verification
(generate checks from acceptance criteria). Not used at runtime.

---

### I-15: Verdict Publisher Optional and Rarely Configured

**Location**: `crates/roko-gate/src/verdict_publisher.rs`

**Problem**: VerdictPublisher is optional in RungExecutionConfig and rarely
provided by callers. Gate verdicts are therefore not broadcast to interested
consumers (TUI, SSE, WebSocket).

**Impact**: Real-time gate progress is not visible in the TUI or dashboard.
Gates run silently and results appear only after the full pipeline completes.

---

## Anti-Pattern Catalog (From Mega-Parity Runs)

The mega-parity batch runs (~195 parallel batches via Codex) identified 10
anti-patterns. Several directly relate to the gate pipeline:

| AP# | Name | Gate Pipeline Relevance |
|-----|------|----------------------|
| AP-1 | Stub gates that silently pass | **Direct**: I-2 (stub verdicts pass in rung_dispatch.rs) |
| AP-2 | `block_on` in async code | Indirect: gates are async, using block_on would deadlock |
| AP-3 | Duplicate trait definitions | Indirect: Verify trait is defined once in roko-core |
| AP-5 | Raw `Command::new("claude")` | **Direct**: I-3 (LLM judge model hardcoded) |
| AP-6 | Inline prompt strings | Indirect: judge prompts should use templates |
| AP-7 | std::sync::Mutex across .await | Moderate: GateService uses Arc<Mutex<AdaptiveThresholds>> |
| AP-8 | Empty function bodies | Indirect: could produce silent-pass gates |
| AP-9 | unimplemented!/unreachable! | Indirect: would cause gate panics |
| AP-10 | Hardcoded localhost/port | Indirect: integration gate URLs |

### AP-7 Mutex Concern

GateService holds `Arc<Mutex<AdaptiveThresholds>>`. The mutex is locked briefly
for `should_skip_rung()` checks and `observe()` calls, both of which are
non-async operations. The lock is never held across an `.await` point.

However, if gate execution were truly parallelized (not just sequential as today),
multiple gates could contend on the mutex. The current implementation avoids this
because gates run sequentially within GateService.run_gates().

---

## Verification Strategy Gaps

From the mega-parity experience, several verification strategies are identified
as needed but not implemented:

### VS-1: Wave-Level Gate Aggregation

Individual batch gates pass but the merged result fails. Need a "wave gate"
that verifies the aggregate after merging all batch outputs.

**Current state**: orchestrate.rs gates each task individually. No post-merge
verification step.

### VS-2: Incremental Gate Verification

Running full `cargo check --workspace` after each task is expensive (3-8 minutes
in a 177K LOC workspace). Need incremental verification that only checks
affected crates.

**Current state**: CompileGate always runs `cargo check --workspace`. The
`--package` flag exists on cargo but is not used by the gate.

### VS-3: Cross-Task Regression Detection

Task B passes its gates but breaks something Task A fixed. Need cross-task
regression detection that re-runs Task A's verification after Task B merges.

**Current state**: No cross-task regression detection. Each task's gates are
independent.

### VS-4: Flaky Test Handling

Tests that intermittently fail cause gate failures that are not the agent's
fault. Need flaky test detection and quarantine.

**Current state**: No flaky test detection. TestGate treats all failures equally.
The adaptive threshold system partially addresses this (high EMA skip) but
doesn't distinguish flaky from genuinely failing.

---

## Sources

Issues identified from analysis of:

- `crates/roko-gate/src/` -- all 40 source files
- `crates/roko-gate/tests/` -- 4 integration test files
- `crates/roko-cli/src/orchestrate.rs` -- gate integration
- `crates/roko-cli/src/run.rs` -- simple gate dispatch
- `crates/roko-acp/src/runner.rs` -- ACP gate dispatch
- `crates/roko-runtime/src/workflow_engine.rs` -- workflow gate dispatch
- `tmp/solutions/runner/LESSONS.md` -- operational lessons from 195-batch run
