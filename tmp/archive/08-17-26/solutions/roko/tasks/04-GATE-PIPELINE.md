# Gate Pipeline Convergence and Intelligence: Task Breakdown

> Converge 4 gate dispatch paths into GateService, fix stub verdicts, wire the
> full adaptive intelligence stack (SPC/CUSUM/Hotelling), failure classification,
> LLM judge via CascadeRouter, domain profiles, process reward model, custom
> gates from config, and gate events for UX. 35 tasks across 8 phases.
>
> Sources: `impl/04-GATE-PIPELINE.md`, `20-GATE-{AUDIT,ISSUES,PLAN}.md`,
> `04-ORCHESTRATION-AND-GATES-AUDIT.md`, codebase analysis

---

## Overview

The gate pipeline has four separate dispatch paths, each with different feature sets:

| Path | Location | Gates | Adaptive | Feedback | Replan |
|---|---|---|---|---|---|
| `roko run` | `crates/roko-cli/src/run.rs` L2942 | 4 hardcoded | No | No | No |
| ACP Runner | `crates/roko-acp/src/runner.rs` L1674 | 3 hardcoded (wrong order) | EMA skip | No | No |
| orchestrate.rs | `crates/roko-cli/src/orchestrate.rs` L16576 | 7-rung pipeline | Full SPC | Yes | Yes |
| Runner v2 | `crates/roko-cli/src/runner/gate_dispatch.rs` | `run_rung()` via spawn | No | Classification only | No |

**Target state**: `GateService` (`crates/roko-gate/src/gate_service.rs`) is the single gate runner. All callers use GateConfig + GateRunner trait. GateService handles rung selection, adaptive thresholds, feedback generation, failure classification, SPC alert draining, Hotelling observation, domain profiles, temperament, and event emission internally. Callers get a `GateReport` with all intelligence included.

**Key types (verified from source)**:
- `GateConfig` at `crates/roko-core/src/foundation.rs:271` (4 fields, needs 2 more)
- `GateReport` at `crates/roko-core/src/foundation.rs:300` (1 field, needs 3 more)
- `GateVerdict` at `crates/roko-core/src/foundation.rs:284` (6 fields including `skipped` and `skip_reason`)
- `GateRunner` trait at `crates/roko-core/src/foundation.rs:331` (single method: `run_gates(&self, config: GateConfig) -> Result<GateReport>`)
- `GateService` at `crates/roko-gate/src/gate_service.rs:26` (1 field: `adaptive: Option<Arc<Mutex<AdaptiveThresholds>>>`)
- `Verdict` at `crates/roko-core/src/verdict.rs:51` (has `pass()` and `fail()`, no `skip()`)

---

## Anti-Patterns to Remove

| ID | Anti-Pattern | Where | Severity |
|---|---|---|---|
| AP-1 | Stub gates that silently pass | `crates/roko-gate/src/rung_dispatch.rs:132` `stub_verdict()` returns `Verdict::pass()` | Critical |
| AP-5 | Hardcoded LLM judge model | `crates/roko-cli/src/orchestrate.rs` `AgentJudgeOracle` falls back to `"claude-sonnet-4-20250514"` | High |
| AP-6 | Four separate gate dispatch paths | `run.rs`, `runner.rs`, `orchestrate.rs`, `runner/gate_dispatch.rs` each implement gate dispatch differently | Critical |
| AP-7 | Feedback only in orchestrate.rs | `feedback_for_agent()` in `crates/roko-gate/src/feedback.rs:202` only called from orchestrate.rs | High |
| AP-8 | Built but unused features | Domain profiles (`ThresholdProfile` L75), Hotelling `observe_pipeline()` L468, SPC `drain_spc_alerts()` L446 -- all test-only | Medium |
| AP-9 | ACP runs clippy after test | `crates/roko-acp/src/runner.rs` L1674 runs compile->test->clippy (rung order 0->2->1) | Medium |
| AP-10 | No cost tracking for LLM judge | `AgentJudgeOracle` records no episode, no gate budget | Medium |
| AP-DUP | GatePipeline / ComposedGatePipeline duplication | `crates/roko-gate/src/gate_pipeline.rs` sequential mode re-implements loop, dead code `let _ = pipeline` | Low |

---

## Phase 1: Extend Foundation Types (GateConfig, GateReport)

Additive struct changes only. No behavioral changes. All downstream phases depend on these fields existing.

### Task 4.1: Add complexity and prior_failures to GateConfig
**Priority**: P0
**Estimated Effort**: 1 hour
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/foundation.rs`
**Depends On**: none

#### Context
`GateConfig` at `crates/roko-core/src/foundation.rs:271` currently has 4 fields: `workdir`, `enabled_gates`, `shell_gates`, `max_rung`. Each caller implements rung selection independently. Making complexity and prior_failures part of GateConfig lets GateService perform rung selection internally via `rung_selector::select_rungs()`.

`PlanComplexity` is defined at `crates/roko-gate/src/rung_selector.rs:25` with variants: Trivial, Simple, Standard, Complex. `roko-core` already depends on `roko-gate` types (via re-exports) or can reference the enum by path. If circular dependency is an issue, the fields should be typed as `Option<u8>` (0-3 mapping to complexity) and `Option<u32>` for prior_failures, avoiding direct dependency on roko-gate from roko-core.

#### Implementation Steps
1. Open `crates/roko-core/src/foundation.rs` and locate `pub struct GateConfig` at line 271.
2. Add two optional fields after `max_rung`:
   ```rust
   /// Plan complexity for rung selection (0=Trivial, 1=Simple, 2=Standard, 3=Complex).
   /// When Some, GateService uses rung_selector::select_rungs() to determine which gates to run.
   /// When None, GateService runs all enabled_gates as-is.
   #[serde(default, skip_serializing_if = "Option::is_none")]
   pub complexity: Option<u8>,
   /// Number of prior gate failures for this task. Used for escalation ladder
   /// (each failure promotes complexity one tier, saturating at Complex).
   #[serde(default, skip_serializing_if = "Option::is_none")]
   pub prior_failures: Option<u32>,
   ```
3. Find all sites constructing `GateConfig` (search `GateConfig {` in `crates/`). Add `complexity: None, prior_failures: None` to each. Known sites:
   - `crates/roko-runtime/src/effect_driver.rs:285` in `run_gates()`
   - `crates/roko-runtime/src/effect_driver.rs` test mocks
   - `crates/roko-runtime/src/workflow_engine.rs` test mocks
   - `crates/roko-gate/src/gate_service.rs` tests
   - `crates/roko-gate/tests/gate_truth.rs`

#### Design Guidance
Use `Option<u8>` for complexity rather than importing `PlanComplexity` from roko-gate into roko-core. roko-core is the kernel crate and must not depend on roko-gate (circular). GateService will convert `u8 -> PlanComplexity` internally. Document the 0-3 mapping in the field doc-comment.

#### Verification Criteria
- [ ] `cargo check --workspace` compiles without errors
- [ ] `cargo test -p roko-core` passes
- [ ] All construction sites updated (grep `GateConfig {` returns no compile errors)
- [ ] New fields are `Option` with serde defaults (backward-compatible deserialization)

---

### Task 4.2: Add feedback, failure_classification, and spc_alerts to GateReport
**Priority**: P0
**Estimated Effort**: 1 hour
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/foundation.rs`
**Depends On**: none

#### Context
`GateReport` at `crates/roko-core/src/foundation.rs:300` has only `verdicts: Vec<GateVerdict>`. Callers that need feedback must call `feedback_for_agent()` separately (only orchestrate.rs does). By including feedback and failure classification in the report, all callers get structured feedback for free.

`GateFeedback` from `crates/roko-gate/src/feedback.rs:53` has fields: `rung`, `passed`, `errors`, `warnings`, `suggestions`. `GateFailureClassification` from `crates/roko-gate/src/compile_errors.rs:180` has fields: `gate`, `primary`, `failure_kind`, `retry_policy`, `summary`, `classes`, `recommended_action`, `cargo_fix_candidate`.

Since roko-core cannot depend on roko-gate, these fields must use serialized representations or a core-level type.

#### Implementation Steps
1. Add a new struct to `crates/roko-core/src/foundation.rs`:
   ```rust
   /// Structured feedback extracted from gate output. Gate-layer detail types
   /// serialize into these core-level types so callers don't need roko-gate.
   #[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
   pub struct GateReportFeedback {
       /// Error-level items (must fix).
       #[serde(default)]
       pub errors: Vec<String>,
       /// Warning-level items (should fix).
       #[serde(default)]
       pub warnings: Vec<String>,
       /// Actionable suggestions.
       #[serde(default)]
       pub suggestions: Vec<String>,
   }

   /// Coarse classification of a gate failure for retry/replan decisions.
   #[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
   pub struct GateReportClassification {
       /// Primary failure class (e.g., "SyntaxError", "ImportError").
       #[serde(default)]
       pub primary_class: String,
       /// Recommended action: "retry", "replan", "blocked", "needs_human".
       #[serde(default)]
       pub recommended_action: String,
       /// Concise failure summary.
       #[serde(default)]
       pub summary: String,
       /// Whether cargo fix could resolve the issue.
       #[serde(default)]
       pub cargo_fix_candidate: bool,
   }

   /// Statistical process control alert from the adaptive threshold system.
   #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
   pub struct GateReportSpcAlert {
       /// Rung number where the alert fired.
       pub rung: u32,
       /// Alert kind: "cusum_shift", "ewma_out_of_control", "ewma_warning", "change_point".
       pub kind: String,
       /// Alert detail (e.g., EWMA value, change probability).
       #[serde(default)]
       pub detail: String,
   }
   ```
2. Add fields to `GateReport`:
   ```rust
   pub struct GateReport {
       pub verdicts: Vec<GateVerdict>,
       /// Structured feedback from the first failing gate. None if all passed.
       #[serde(default, skip_serializing_if = "Option::is_none")]
       pub feedback: Option<GateReportFeedback>,
       /// Failure classification for retry/replan routing. None if all passed.
       #[serde(default, skip_serializing_if = "Option::is_none")]
       pub failure_classification: Option<GateReportClassification>,
       /// SPC alerts drained after this pipeline run.
       #[serde(default, skip_serializing_if = "Vec::is_empty")]
       pub spc_alerts: Vec<GateReportSpcAlert>,
       /// Whether Hotelling's T-squared detected a joint anomaly across gates.
       #[serde(default)]
       pub joint_anomaly: bool,
   }
   ```
3. Derive `serde::Serialize, serde::Deserialize` on `GateReport` (currently only `Debug, Clone`).
4. Update `GateReport::all_passed()` -- logic unchanged (already checks `passed && !skipped`).
5. Update all sites constructing `GateReport { verdicts }` to include the new fields as defaults. Known sites:
   - `crates/roko-gate/src/gate_service.rs:365` in `run_gates()`
   - `crates/roko-runtime/src/effect_driver.rs` and `workflow_engine.rs` test mocks
   - `crates/roko-cli/src/run.rs` test mocks

#### Design Guidance
Use `String`-typed fields in the core-level structs rather than importing roko-gate enums. GateService will convert from `GateFeedback -> GateReportFeedback` and `GateFailureClassification -> GateReportClassification` using simple `From` impls in roko-gate. This keeps roko-core free of roko-gate dependency.

#### Verification Criteria
- [ ] `cargo check --workspace` compiles
- [ ] `cargo test -p roko-core` passes
- [ ] `GateReport` has `feedback`, `failure_classification`, `spc_alerts`, `joint_anomaly` fields
- [ ] All construction sites updated
- [ ] Serde derives present on `GateReport`

---

## Phase 2: Wire GateService as Single Gate Runner

Converge all callers to GateService. Eliminates AP-6 (four dispatch paths) and AP-9 (wrong rung order). After this phase, GateService is the only gate execution implementation.

### Task 4.3: Generate feedback and classification inside GateService
**Priority**: P0
**Estimated Effort**: 3 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-gate/src/gate_service.rs`
**Depends On**: Task 4.1, Task 4.2

#### Context
`GateService::run_gates()` at `crates/roko-gate/src/gate_service.rs:235` currently returns `Ok(GateReport { verdicts })` with no feedback or classification. The feedback module (`crates/roko-gate/src/feedback.rs:202` `feedback_for_agent()`) and classification module (`crates/roko-gate/src/compile_errors.rs:491` `classify_gate_failure()`) exist and work but are only called from orchestrate.rs.

By moving feedback generation into GateService, all callers get structured feedback automatically. This eliminates AP-7 (feedback as afterthought).

#### Implementation Steps
1. Add `use crate::feedback::feedback_for_agent;` and `use crate::compile_errors::classify_gate_failure;` to gate_service.rs imports.
2. After the verdict collection loop (line 363), before the `Ok(GateReport { verdicts })` return (line 365), add feedback generation:
   ```rust
   // Generate feedback from first failing gate
   let (feedback, failure_classification) = if let Some(failing) = verdicts.iter().find(|v| !v.passed && !v.skipped) {
       let rung = Self::rung_for_name(&failing.gate_name).unwrap_or(0);
       let fb = feedback_for_agent(&failing.output, rung);
       let core_fb = GateReportFeedback {
           errors: fb.errors,
           warnings: fb.warnings,
           suggestions: fb.suggestions,
       };
       let classification = classify_gate_failure(&failing.gate_name, &failing.output);
       let core_class = GateReportClassification {
           primary_class: format!("{:?}", classification.primary),
           recommended_action: format!("{:?}", classification.recommended_action),
           summary: classification.summary.clone(),
           cargo_fix_candidate: classification.cargo_fix_candidate,
       };
       (Some(core_fb), Some(core_class))
   } else {
       (None, None)
   };
   ```
3. Return the enriched report:
   ```rust
   Ok(GateReport {
       verdicts,
       feedback,
       failure_classification,
       spc_alerts: Vec::new(),  // Wired in Task 4.15
       joint_anomaly: false,     // Wired in Task 4.16
   })
   ```
4. Update existing tests in `gate_service.rs` to assert on the new fields where relevant (at minimum: verify `feedback` is `None` when all gates pass, `Some` when a gate fails).

#### Design Guidance
Keep feedback generation synchronous and cheap -- it is pure string parsing, no I/O. The `feedback_for_agent()` function filters noise and classifies severity in ~0.1ms. `classify_gate_failure()` parses cargo JSON diagnostics in ~0.5ms. Both are safe to call on every gate run.

#### Verification Criteria
- [ ] `cargo test -p roko-gate` passes
- [ ] GateService returns `feedback: Some(...)` when a gate fails
- [ ] GateService returns `feedback: None` when all gates pass
- [ ] `failure_classification.recommended_action` is populated on failure

---

### Task 4.4: Add rung selection to GateService
**Priority**: P0
**Estimated Effort**: 3 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-gate/src/gate_service.rs`
**Depends On**: Task 4.1

#### Context
Currently GateService runs all `enabled_gates` from GateConfig in rung order. With `complexity` and `prior_failures` now available on GateConfig (Task 4.1), GateService can perform rung selection internally using `rung_selector::select_rungs()` (at `crates/roko-gate/src/rung_selector.rs`).

When `GateConfig.complexity` is `Some`, GateService should compute the effective rung set and filter `enabled_gates` to only those in the selected set. When `None`, behavior is unchanged (all enabled_gates run).

#### Implementation Steps
1. Import `crate::rung_selector::{PlanComplexity, RungCaps, select_rungs, Rung}` in gate_service.rs.
2. Add a helper to convert `u8 -> PlanComplexity`:
   ```rust
   fn complexity_from_u8(v: u8) -> PlanComplexity {
       match v {
           0 => PlanComplexity::Trivial,
           1 => PlanComplexity::Simple,
           2 => PlanComplexity::Standard,
           _ => PlanComplexity::Complex,
       }
   }
   ```
3. At the beginning of `run_gates()`, after `ordered_gate_names()`, filter the gate list when complexity is provided:
   ```rust
   let gate_names = Self::ordered_gate_names(&config);
   let gate_names = if let Some(complexity_u8) = config.complexity {
       let complexity = complexity_from_u8(complexity_u8);
       let prior_failures = config.prior_failures.unwrap_or(0);
       let caps = RungCaps::all(); // TODO: detect from environment
       let selected_rungs = select_rungs(complexity, &caps, prior_failures);
       let selected_indices: HashSet<u8> = selected_rungs.iter().map(|r| r.index()).collect();
       gate_names.into_iter().filter(|name| {
           Self::rung_for_name(name).map_or(true, |rung| selected_indices.contains(&rung))
       }).collect()
   } else {
       gate_names
   };
   ```
4. Add tests: verify that `complexity: Some(0)` (Trivial) only runs compile, `complexity: Some(3)` (Complex) runs all rungs, and `prior_failures: Some(2)` escalates Trivial -> Standard.

#### Design Guidance
Rung selection is pure computation with no I/O. `select_rungs()` returns a `Vec<Rung>` in ~0.01ms. The escalation ladder is: each prior failure promotes complexity one tier, saturating at Complex. This means a Trivial task that fails twice gets Standard-level gates automatically.

#### Verification Criteria
- [ ] `cargo test -p roko-gate` passes
- [ ] `complexity: Some(0)` with `enabled_gates: ["compile", "clippy", "test"]` runs only compile
- [ ] `complexity: Some(3)` runs all enabled gates
- [ ] `prior_failures: Some(2)` escalates Trivial to Standard

---

### Task 4.5: Migrate ACP runner to GateService
**Priority**: P0
**Estimated Effort**: 4 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-acp/src/runner.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-acp/Cargo.toml`
**Depends On**: Task 4.3

#### Context
`run_gates()` at `crates/roko-acp/src/runner.rs:1674` is a ~100 line function that hardcodes three gates: CompileGate, TestGate, ClippyGate. It runs them in the wrong order (compile -> test -> clippy, should be compile -> clippy -> test). It implements its own adaptive threshold loading/saving and skip logic. Replacing it with GateService eliminates AP-6 (duplicate path) and AP-9 (wrong order) simultaneously.

The ACP runner already loads `AdaptiveThresholds` from disk for its skip logic. GateService accepts `with_adaptive_thresholds()` and handles skip/observe internally.

#### Implementation Steps
1. Add `roko-gate = { path = "../roko-gate" }` to `crates/roko-acp/Cargo.toml` if not already present.
2. Replace the entire `run_gates()` function body:
   ```rust
   async fn run_gates(
       _session_id: &str,
       workdir: &Path,
       clippy_enabled: bool,
       tests_enabled: bool,
       cancel_token: &CancelToken,
   ) -> Result<(bool, String)> {
       let mut enabled = vec!["compile".to_string()];
       if clippy_enabled { enabled.push("clippy".into()); }
       if tests_enabled { enabled.push("test".into()); }

       let gate_config = GateConfig {
           workdir: workdir.to_path_buf(),
           enabled_gates: enabled,
           shell_gates: vec![],
           max_rung: None,
           complexity: None,
           prior_failures: None,
       };

       let thresholds_path = workdir.join(".roko/learn/gate-thresholds.json");
       let thresholds = AdaptiveThresholds::load_or_new(&thresholds_path);
       let svc = GateService::new().with_adaptive_thresholds(thresholds.clone());
       let report = svc.run_gates(gate_config).await?;

       // Save updated thresholds
       if let Ok(t) = thresholds.lock() {
           let _ = t.save(&thresholds_path);
       }

       let passed = report.all_passed();
       let output = report.verdicts.iter()
           .map(|v| format!("{}: {}", v.gate_name, if v.passed { "pass" } else { &v.output }))
           .collect::<Vec<_>>()
           .join("\n");

       Ok((passed, output))
   }
   ```
3. Remove the old inline CompileGate/TestGate/ClippyGate construction and invocation code.
4. Update imports: remove direct gate imports, add `use roko_gate::{GateService, AdaptiveThresholds}; use roko_core::foundation::GateConfig;`.
5. Verify the caller at line 969 (`PipelineAction::RunGates`) still receives `(bool, String)` -- adjust return type if GateReport is more appropriate.

#### Design Guidance
Keep the `(bool, String)` return type for minimal caller disruption. The ACP runner's caller only needs pass/fail and a summary string. Future work can expose the full `GateReport` if needed.

#### Verification Criteria
- [ ] `cargo check -p roko-acp` compiles
- [ ] `cargo test -p roko-acp` passes
- [ ] Gates now run in canonical order: compile -> clippy -> test (rung 0 -> 1 -> 2)
- [ ] Adaptive thresholds are loaded and saved via GateService
- [ ] `grep -rn 'CompileGate\|ClippyGate\|TestGate' crates/roko-acp/` returns no runtime usage (only import lines removed)

---

### Task 4.6: Migrate `roko run` gate dispatch to GateService
**Priority**: P0
**Estimated Effort**: 3 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/run.rs`
**Depends On**: Task 4.3

#### Context
`run_gate()` at `crates/roko-cli/src/run.rs:2942` (behind `#[cfg(feature = "legacy-orchestrate")]`) matches on a `GateConfig` enum with 4 variants: Shell, Compile, Clippy, Test. This is the simplest dispatch path with no adaptive thresholds, no feedback, no rung selection. Replacing it with GateService adds all those features for free.

Note this function is behind a feature flag `legacy-orchestrate`. The primary `roko run` path now goes through WorkflowEngine's EffectDriver, which already uses `GateRunner` trait. Verify which code path is active before modifying.

#### Implementation Steps
1. Search for the active gate dispatch path in `roko run` (not behind feature flags):
   - The WorkflowEngine path (`crates/roko-runtime/src/effect_driver.rs:280` `run_gates()`) already constructs `GateConfig` and calls `self.services.gate_runner.run_gates(config)`. This path already uses GateService when ServiceFactory wires it.
2. For the legacy path behind `#[cfg(feature = "legacy-orchestrate")]`:
   - Replace the match-based dispatch with a GateService call similar to Task 4.5.
   - Or mark the legacy path as deprecated and schedule removal.
3. Verify ServiceFactory at `crates/roko-orchestrator/src/service_factory.rs` constructs `GateService` as the `gate_runner`. If it does, `roko run` already uses GateService transitively. Document this finding.
4. If `roko run` still has a non-legacy gate path outside the WorkflowEngine, migrate it.

#### Design Guidance
The WorkflowEngine + EffectDriver + ServiceFactory path is the canonical one for `roko run`. If ServiceFactory already wires GateService, this task is primarily verification + cleanup of the legacy path.

#### Verification Criteria
- [ ] `cargo run -p roko-cli -- run "add a comment"` uses GateService for gates
- [ ] No inline gate construction outside GateService in the active `roko run` code path
- [ ] Legacy `run_gate()` behind feature flag is either migrated or marked deprecated

---

### Task 4.7: Align Runner v2 gate dispatch with GateService
**Priority**: P1
**Estimated Effort**: 6 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/gate_dispatch.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/event_loop.rs`
**Depends On**: Task 4.3, Task 4.4

#### Context
Runner v2's `gate_dispatch.rs` calls `rung_dispatch::run_rung()` directly, bypassing GateService entirely. It spawns gates as background tokio tasks with a semaphore for serialization (`GATE_SEMAPHORE`). The `spawn_gate()` function at line 29 constructs `RungExecutionInputs::default()` (all oracles missing -- triggering stub verdicts) and `RungExecutionConfig` with only `source_roots`.

The runner v2 event loop receives `GateCompletion` from the gate channel and uses `classify_failure_kind()` (line 287) which already calls `classify_gate_failure()` from roko-gate.

Migrating to GateService requires replacing the per-rung dispatch with a single GateService call that runs the full gate pipeline. The background task pattern can remain -- just wrap GateService::run_gates() instead of run_rung().

#### Implementation Steps
1. In `gate_dispatch.rs`, replace `spawn_gate()` internals:
   ```rust
   pub fn spawn_gate(
       plan_id: String,
       task_id: String,
       rung: u32,
       workdir: PathBuf,
       verify_steps: Vec<VerifyStep>,
       timeout_secs: u64,
       gate_tx: mpsc::Sender<GateCompletion>,
   ) {
       tokio::spawn(async move {
           let Ok(_permit) = gate_semaphore().acquire_owned().await else { return; };
           let start = Instant::now();

           // Use GateService instead of direct run_rung()
           let enabled = rung_gates_for_level(rung);
           let config = GateConfig {
               workdir: workdir.clone(),
               enabled_gates: enabled,
               shell_gates: vec![],
               max_rung: Some(rung as u8),
               complexity: None,
               prior_failures: None,
           };

           let svc = GateService::new();
           let limit = Duration::from_secs(timeout_secs.max(1));
           let report = match timeout(limit, svc.run_gates(config)).await {
               Ok(Ok(report)) => report,
               Ok(Err(e)) => { /* construct error GateCompletion */ },
               Err(_) => { /* timeout GateCompletion */ },
           };

           // Also run verify steps
           // ... (keep existing verify_steps logic)

           // Convert GateReport -> GateCompletion
           let completion = report_to_completion(plan_id, task_id, rung, report, start.elapsed());
           let _ = gate_tx.send(completion).await;
       });
   }
   ```
2. Add helper `rung_gates_for_level(rung: u32) -> Vec<String>` that maps rung level to gate names:
   ```rust
   fn rung_gates_for_level(rung: u32) -> Vec<String> {
       match rung {
           0 => vec!["compile".into()],
           1 => vec!["compile".into(), "clippy".into()],
           2 => vec!["compile".into(), "clippy".into(), "test".into()],
           _ => vec!["compile".into(), "clippy".into(), "test".into()],
       }
   }
   ```
3. Add helper `report_to_completion()` that converts `GateReport` fields to `GateCompletion` fields.
4. Update `event_loop.rs` gate completion handler to use `report.failure_classification.recommended_action` for retry routing (instead of re-classifying from raw output).
5. Keep the verify_steps logic (ShellGate for task-specific verify commands) -- this is runner-v2-specific and not part of GateService.

#### Design Guidance
The semaphore pattern (`GATE_SEMAPHORE`) serializes gate execution to avoid concurrent cargo processes fighting over the target directory. This is correct and should remain even with GateService -- just wrap the GateService call inside the semaphore. The verify_steps pattern (ShellGate per task) complements GateService and should run after GateService's gates.

#### Verification Criteria
- [ ] `cargo test -p roko-cli` passes
- [ ] Runner v2 gate dispatch uses GateService internally
- [ ] `run_rung()` is no longer called directly from gate_dispatch.rs
- [ ] Verify steps still execute after GateService gates
- [ ] Gate completion includes feedback and classification from GateReport

---

## Phase 3: Fix Stub Verdicts and LLM Judge (AP-1, AP-5)

### Task 4.8: Add Verdict::skip constructor and convert stub_verdict
**Priority**: P0
**Estimated Effort**: 2 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/verdict.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-gate/src/rung_dispatch.rs`
**Depends On**: none

#### Context
`Verdict` at `crates/roko-core/src/verdict.rs:51` has `pass()` and `fail()` constructors but no `skip()`. The `stub_verdict()` function at `crates/roko-gate/src/rung_dispatch.rs:132` uses `Verdict::pass()` for stubs, meaning missing inputs produce passing verdicts. This is the AP-1 anti-pattern: false confidence from silent passes.

`GateVerdict` (roko-core) already has `skipped: bool` and `skip_reason: Option<String>` fields, but `Verdict` (roko-core) does not have corresponding fields. The `to_gate_verdict()` function in gate_service.rs hardcodes `skipped: false`.

#### Implementation Steps
1. Add a `skip()` constructor to `Verdict` in `crates/roko-core/src/verdict.rs`:
   ```rust
   /// A skipped verdict -- the gate did not run but this is not a failure.
   /// Skipped verdicts have `passed: true` (they don't block the pipeline)
   /// but carry a distinct marker so they are not counted as real passes.
   #[must_use]
   pub fn skip(gate: impl Into<String>, reason: impl Into<String>) -> Self {
       Self {
           passed: true,       // Does not block pipeline
           reason: reason.into(),
           gate: gate.into(),
           score: 0.0,         // No quality signal from a skip
           detail: None,
           test_count: None,
           error_digest: None,
           duration_ms: 0,
       }
   }

   /// Whether this verdict represents a skip (gate did not execute).
   #[must_use]
   pub fn is_skip(&self) -> bool {
       self.score == 0.0 && self.passed && self.reason.starts_with("stub gate;")
   }
   ```
2. Actually -- better approach: add a `skipped: bool` field to `Verdict` directly:
   ```rust
   pub struct Verdict {
       pub passed: bool,
       /// Whether the gate was skipped rather than executed.
       #[serde(default)]
       pub skipped: bool,
       // ... existing fields ...
   }
   ```
   Update `pass()` and `fail()` to set `skipped: false`. Add `skip()` that sets `skipped: true, passed: true`.
3. Change `stub_verdict()` in `crates/roko-gate/src/rung_dispatch.rs:132`:
   ```rust
   fn stub_verdict(gate: &str, detail: impl Into<String>) -> Verdict {
       let message = format!("stub gate; {}", detail.into());
       Verdict::skip(gate, message)
   }
   ```
4. Update `to_gate_verdict()` in `crates/roko-gate/src/gate_service.rs:369` to propagate the skipped flag:
   ```rust
   fn to_gate_verdict(gate_name: String, verdict: Verdict) -> GateVerdict {
       GateVerdict {
           gate_name,
           passed: verdict.passed,
           skipped: verdict.skipped,
           skip_reason: if verdict.skipped { Some(verdict.reason.clone()) } else { None },
           output: /* ... existing logic ... */,
           duration_ms: verdict.duration_ms,
       }
   }
   ```
5. Update `GateReport::all_passed()` at `crates/roko-core/src/foundation.rs:308` -- it already checks `v.passed && !v.skipped`, so skipped verdicts will correctly not count as "passed". Verify this logic.
6. Update `AdaptiveThresholds::observe()` callers -- skipped verdicts should NOT be observed (no real data). The gate_service.rs loop at line 353 already checks `!was_skipped`, so this is already correct.

#### Design Guidance
Stubs should be `passed: true, skipped: true` so they don't block the pipeline (backward-compatible) but are clearly distinguishable from real passes. This means `all_passed()` returns false when only stubs ran (since it checks `!skipped`), which is the desired behavior: a pipeline of only stubs is not a real pass.

#### Verification Criteria
- [ ] `cargo test --workspace` passes
- [ ] `Verdict::skip("gate", "reason").skipped == true`
- [ ] `Verdict::skip("gate", "reason").passed == true` (does not block pipeline)
- [ ] `stub_verdict()` returns a skipped verdict, not a passing verdict
- [ ] `GateReport::all_passed()` returns false when all verdicts are skipped
- [ ] `AdaptiveThresholds::observe()` is NOT called for skipped verdicts

---

### Task 4.9: Route LLM judge through CascadeRouter
**Priority**: P1
**Estimated Effort**: 4 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/orchestrate.rs`
**Depends On**: none

#### Context
The LLM judge oracle in orchestrate.rs (`AgentJudgeOracle` construction) falls back to `"claude-sonnet-4-20250514"` when no model is configured. This is the AP-5 anti-pattern: hardcoded model strings bypass the routing system. The CascadeRouter already exists and is wired in orchestrate.rs for agent dispatch. It should also be used for judge model selection.

#### Implementation Steps
1. Locate the `AgentJudgeOracle` construction in orchestrate.rs (search for `AgentJudgeOracle`).
2. Replace the hardcoded model fallback:
   ```rust
   // Before:
   let model = self.config.agent.model.as_deref().unwrap_or("claude-sonnet-4-20250514");

   // After:
   let model = if let Some(m) = self.config.agent.model.as_deref() {
       m.to_string()
   } else if let Ok(router) = self.cascade_router.lock() {
       router.select_model_for_role("gate-judge")
           .unwrap_or_else(|| "claude-sonnet-4-20250514".into())
   } else {
       "claude-sonnet-4-20250514".into()
   };
   ```
3. After the judge call completes, record a CascadeRouter observation:
   ```rust
   if let Ok(mut router) = self.cascade_router.lock() {
       router.observe("gate-judge", &model, judge_score, cost, duration_ms);
   }
   ```
4. Record an episode for each judge invocation:
   ```rust
   let episode = Episode {
       agent_id: "gate-judge".into(),
       task_id: plan_id.to_string(),
       kind: "gate-judge".into(),
       model: model.clone(),
       // ... fill usage from response
   };
   self.episode_logger.record(&episode);
   ```
5. Search for all occurrences of `"claude-sonnet-4-20250514"` in crates/ (excluding tests) and ensure none remain as runtime fallbacks.

#### Verification Criteria
- [ ] `grep -rn 'claude-sonnet-4-20250514' crates/ --include='*.rs' | grep -v test | grep -v '\/\/'` returns no runtime usages
- [ ] Judge model selection goes through CascadeRouter when available
- [ ] Episode recorded per judge invocation with model slug and cost

---

### Task 4.10: Add gate budget tracking
**Priority**: P1
**Estimated Effort**: 2 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-gate/src/gate_service.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/foundation.rs`
**Depends On**: Task 4.2

#### Context
LLM judge invocations have no cost tracking (AP-10). Each judge call is a full LLM API call but no episode is recorded, no cost is attributed, and no limit prevents runaway invocations during replan loops.

#### Implementation Steps
1. Add a `GateBudget` struct to `crates/roko-core/src/foundation.rs`:
   ```rust
   #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
   pub struct GateBudget {
       pub max_judge_invocations: u32,
       pub current_judge_invocations: u32,
       pub max_cost_usd: f64,
       pub current_cost_usd: f64,
   }

   impl Default for GateBudget {
       fn default() -> Self {
           Self {
               max_judge_invocations: 10,
               current_judge_invocations: 0,
               max_cost_usd: 5.0,
               current_cost_usd: 0.0,
           }
       }
   }

   impl GateBudget {
       pub fn can_invoke_judge(&self) -> bool {
           self.current_judge_invocations < self.max_judge_invocations
               && self.current_cost_usd < self.max_cost_usd
       }
   }
   ```
2. Add optional `budget: Option<GateBudget>` field to `GateConfig`.
3. In GateService, before invoking the judge gate, check `budget.can_invoke_judge()`. If exhausted, return a skipped verdict with reason "gate budget exhausted".
4. After judge invocation, increment `budget.current_judge_invocations` and add estimated cost to `budget.current_cost_usd`.

#### Verification Criteria
- [ ] `cargo test -p roko-gate` passes
- [ ] Judge gate is skipped when budget is exhausted
- [ ] Budget tracking increments on each judge invocation

---

## Phase 4: Wire Adaptive Intelligence (SPC, Hotelling, Profiles)

### Task 4.11: Add domain profile initialization to GateService
**Priority**: P1
**Estimated Effort**: 3 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-gate/src/adaptive_threshold.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-gate/src/gate_service.rs`
**Depends On**: Task 4.3

#### Context
`ThresholdProfile` at `crates/roko-gate/src/adaptive_threshold.rs:75` defines three domain profiles (coding, research, security) with per-rung priors and sensitivity overrides. `ThresholdProfile::coding()` at line 93, `research()` at line 110, `security()` at line 127. None of these are ever instantiated at runtime (AP-8).

`AdaptiveThresholds::new()` starts all rungs at neutral priors (EMA 0.5). A security auditor and a code implementer both start with identical expectations, when their pass rates differ significantly.

#### Implementation Steps
1. Add `from_profile()` constructor to `AdaptiveThresholds`:
   ```rust
   pub fn from_profile(profile: &ThresholdProfile) -> Self {
       let mut at = Self::new();
       for (&rung, &prior) in &profile.rung_priors {
           let stats = at.rungs.entry(rung).or_default();
           stats.ema_pass_rate = prior;
           stats.total_observations = 5; // small but non-zero to weight priors
       }
       if let Some(sensitivity) = profile.cusum_sensitivity_override {
           at.cusum_sensitivity = sensitivity;
       }
       at
   }
   ```
2. Add `with_profile()` builder method to `GateService`:
   ```rust
   pub fn with_profile(self, profile: &ThresholdProfile) -> Self {
       let thresholds = AdaptiveThresholds::from_profile(profile);
       self.with_adaptive_thresholds(Arc::new(Mutex::new(thresholds)))
   }
   ```
3. Add `profile` field to `GateConfig`:
   ```rust
   /// Domain profile name for threshold initialization ("coding", "research", "security").
   #[serde(default, skip_serializing_if = "Option::is_none")]
   pub profile: Option<String>,
   ```
4. In `GateService::run_gates()`, when adaptive thresholds are not already set and `config.profile` is provided, initialize from the named profile:
   ```rust
   if self.adaptive.is_none() && let Some(ref profile_name) = config.profile {
       let profile = ThresholdProfile::by_name(profile_name)
           .unwrap_or_else(ThresholdProfile::coding);
       // Initialize on first use
   }
   ```
5. Add `ThresholdProfile::by_name()` helper:
   ```rust
   pub fn by_name(name: &str) -> Option<Self> {
       match name {
           "coding" => Some(Self::coding()),
           "research" => Some(Self::research()),
           "security" => Some(Self::security()),
           _ => None,
       }
   }
   ```
6. Add tests verifying profile initialization sets correct EMA priors.

#### Verification Criteria
- [ ] `cargo test -p roko-gate` passes
- [ ] `AdaptiveThresholds::from_profile(&ThresholdProfile::security())` starts rung 0 at 0.95 EMA
- [ ] GateService with profile "security" has stricter initial thresholds than default

---

### Task 4.12: Add temperament to GateService
**Priority**: P1
**Estimated Effort**: 2 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-gate/src/gate_service.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/foundation.rs`
**Depends On**: Task 4.1

#### Context
`Temperament` at `crates/roko-core/src/temperament.rs:14` has variants: Conservative, Balanced, Aggressive, Exploratory. `AdaptiveThresholds::should_skip_rung_for_temperament()` at `crates/roko-gate/src/adaptive_threshold.rs:602` implements temperament-aware skip logic (Conservative never skips, Aggressive skips at half the streak threshold). But GateService ignores temperament -- it always calls `should_skip_rung()` (the temperament-unaware version).

#### Implementation Steps
1. Add `temperament: Option<String>` field to `GateConfig` (string to avoid roko-core importing Temperament directly -- or use the `Temperament` type since it is already in roko-core).
2. Actually, since `Temperament` is in roko-core (`crates/roko-core/src/temperament.rs`), use it directly:
   ```rust
   pub temperament: Option<Temperament>,
   ```
3. Add `temperament` field to `GateService`:
   ```rust
   pub struct GateService {
       adaptive: Option<Arc<Mutex<AdaptiveThresholds>>>,
       temperament: Temperament,
   }
   ```
4. Add builder method:
   ```rust
   pub fn with_temperament(mut self, t: Temperament) -> Self {
       self.temperament = t;
       self
   }
   ```
5. Update `should_skip_rung_adaptively()` to use temperament-aware method:
   ```rust
   fn should_skip_rung_adaptively(&self, rung: Option<u8>) -> Result<bool> {
       // ... existing None check ...
       thresholds.should_skip_rung_for_temperament(u32::from(r), self.temperament)
       // ... rest unchanged ...
   }
   ```
6. Alternatively, read temperament from GateConfig at the start of run_gates() and use it throughout.

#### Verification Criteria
- [ ] `cargo test -p roko-gate` passes
- [ ] Conservative temperament never skips any rung
- [ ] Aggressive temperament skips at half the normal streak threshold

---

### Task 4.13: Drain SPC alerts after pipeline run
**Priority**: P1
**Estimated Effort**: 2 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-gate/src/gate_service.rs`
**Depends On**: Task 4.2, Task 4.3

#### Context
`drain_spc_alerts()` at `crates/roko-gate/src/adaptive_threshold.rs:446` returns and clears accumulated SPC alerts. These alerts are generated by the CUSUM/EWMA/BOCPD ensemble during `observe()` calls. But `drain_spc_alerts()` is never called from runtime code (AP-8). Alerts accumulate in memory indefinitely.

With Task 4.2, `GateReport` now has an `spc_alerts` field. GateService should drain alerts after each pipeline run and include them in the report.

#### Implementation Steps
1. After the verdict loop and feedback generation in `run_gates()`, drain SPC alerts:
   ```rust
   let spc_alerts = if let Some(adaptive) = &self.adaptive {
       if let Ok(mut thresholds) = adaptive.lock() {
           thresholds.drain_spc_alerts()
               .into_iter()
               .map(|(rung, alert)| GateReportSpcAlert {
                   rung,
                   kind: match &alert {
                       SpcAlert::CusumShift(_) => "cusum_shift".into(),
                       SpcAlert::EwmaOutOfControl { .. } => "ewma_out_of_control".into(),
                       SpcAlert::EwmaWarning { .. } => "ewma_warning".into(),
                       SpcAlert::ChangePoint(_) => "change_point".into(),
                   },
                   detail: format!("{alert:?}"),
               })
               .collect()
       } else {
           vec![]
       }
   } else {
       vec![]
   };
   ```
2. Include `spc_alerts` in the returned `GateReport`.
3. Add a test that generates 50 failures on rung 0, then runs a pipeline and verifies SPC alerts appear in the report.

#### Verification Criteria
- [ ] `cargo test -p roko-gate` passes
- [ ] After sustained failures, `GateReport.spc_alerts` is non-empty
- [ ] Alerts are drained (subsequent report has empty alerts if no new alerts fired)

---

### Task 4.14: Call Hotelling observe_pipeline after full run
**Priority**: P1
**Estimated Effort**: 2 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-gate/src/gate_service.rs`
**Depends On**: Task 4.2, Task 4.3

#### Context
`observe_pipeline()` at `crates/roko-gate/src/adaptive_threshold.rs:468` feeds the full pass-rate vector to Hotelling's T-squared detector for joint anomaly detection. `joint_anomaly_detected()` at line 488 returns whether the last observation triggered an anomaly. Neither is called from runtime code (AP-8).

#### Implementation Steps
1. After the verdict loop in `run_gates()`, build the pass-rate vector and call `observe_pipeline()`:
   ```rust
   let joint_anomaly = if let Some(adaptive) = &self.adaptive {
       let pass_rates: Vec<f64> = verdicts.iter()
           .filter(|v| !v.skipped)
           .map(|v| if v.passed { 1.0 } else { 0.0 })
           .collect();
       if pass_rates.len() >= 2 {
           if let Ok(mut thresholds) = adaptive.lock() {
               thresholds.observe_pipeline(&pass_rates);
               thresholds.joint_anomaly_detected()
           } else {
               false
           }
       } else {
           false
       }
   } else {
       false
   };
   ```
2. Include `joint_anomaly` in the returned `GateReport`.
3. Add a test: 50 normal observations then a joint drop to [0.0, 0.0] should set `joint_anomaly: true`.

#### Verification Criteria
- [ ] `cargo test -p roko-gate` passes
- [ ] After a sudden multi-gate drop, `GateReport.joint_anomaly` is true
- [ ] Normal runs have `joint_anomaly: false`

---

## Phase 5: Wire Failure Classification into Retry/Replan (I-10)

### Task 4.15: Route by failure action in Runner v2 event loop
**Priority**: P1
**Estimated Effort**: 4 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/event_loop.rs`
**Depends On**: Task 4.7

#### Context
Runner v2's event loop at `crates/roko-cli/src/runner/event_loop.rs` receives `GateCompletion` from the gate channel and currently always retries on failure (up to max iterations). It does not differentiate between retryable failures and structural/blocked/human-needed failures.

`GateFailureAction` at `crates/roko-gate/src/compile_errors.rs:69` has four variants: Retry, NeedsReplan, Blocked, NeedsHuman. The runner's `gate_dispatch.rs:309` already maps these to `RunnerFailureKind` variants but the event loop doesn't fully utilize them.

After Task 4.7, `GateCompletion` will carry `failure_classification` from `GateReport`. The event loop should use this to decide whether to retry, replan, pause, or escalate.

#### Implementation Steps
1. In the gate completion handler (`gate_rx.recv()` branch around line 494), extract the failure classification:
   ```rust
   if !completion.passed {
       match completion.failure_kind {
           Some(RunnerFailureKind::Structural) => {
               // Don't retry -- mark task as needing replan
               // Log reason, skip retry, mark failed with replan flag
           }
           Some(RunnerFailureKind::Resource) => {
               // External blocker -- pause and alert
               // Don't consume retry budget
           }
           Some(RunnerFailureKind::Permanent) => {
               // Needs human -- stop immediately
               // Mark task as permanently failed
           }
           _ => {
               // Retry with feedback (existing behavior)
           }
       }
   }
   ```
2. Ensure `RunnerFailureKind` variants map correctly from `GateReport.failure_classification.recommended_action` string.
3. When `NeedsReplan` is detected, emit a `RunnerEvent` that the caller can use to trigger replanning.
4. When `NeedsHuman` is detected, log a prominent warning and mark the task as failed without consuming retries.

#### Design Guidance
The existing `RunnerFailureKind` mapping in `gate_dispatch.rs:309-322` is a good starting point. The event loop should respect it rather than always defaulting to retry. The key change is behavioral: `Structural` failures should not retry, `Resource` failures should pause, and `Permanent` failures should stop.

#### Verification Criteria
- [ ] `cargo test -p roko-cli` passes
- [ ] Structural failures (NeedsReplan) do not consume retry budget
- [ ] Resource failures (Blocked) pause the task
- [ ] Permanent failures (NeedsHuman) immediately fail the task

---

### Task 4.16: Track failure patterns in learning system
**Priority**: P2
**Estimated Effort**: 3 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/event_loop.rs`
**Depends On**: Task 4.15

#### Context
`error_patterns.rs` at `crates/roko-gate/src/error_patterns.rs` extracts `ErrorPatternRecord` from gate classifications. `records_from_classification()` at line 64 converts a `GateFailureClassification` to pattern records with keys like `"E0425::src/main.rs"`. These records enable detection of recurring failure modes.

Runner v2 constructs a `LearningRuntime` but does not record error patterns from gate failures.

#### Implementation Steps
1. After each gate failure in the event loop, extract error patterns:
   ```rust
   use roko_gate::error_patterns::records_from_classification;

   if let Some(ref classification) = completion.failure_classification {
       let patterns = records_from_classification(classification);
       for pattern in patterns {
           learning_runtime.record_error_pattern(pattern);
       }
   }
   ```
2. If `LearningRuntime` does not have `record_error_pattern()`, add it -- or append directly to the error patterns file.
3. At plan completion, query top-K error patterns and log them for diagnostic purposes.

#### Verification Criteria
- [ ] Error patterns are recorded after gate failures
- [ ] Patterns persist across runs (written to disk)
- [ ] Duplicate patterns increment count rather than creating new entries

---

## Phase 6: Process Reward Model (I-12)

### Task 4.17: Instantiate ProcessRewardModel per task in Runner v2
**Priority**: P2
**Estimated Effort**: 3 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/event_loop.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/state.rs`
**Depends On**: Task 4.7

#### Context
`ProcessRewardModel` at `crates/roko-gate/src/process_reward.rs:51` tracks per-turn `TurnSnapshot` (rung, verdicts, error_count, diff_lines) and derives `promise()` (probability of eventual success) and `progress()` (trajectory delta). It is fully implemented and tested but never instantiated at runtime.

#### Implementation Steps
1. Add `HashMap<String, ProcessRewardModel>` to `RunState` (per-task PRM):
   ```rust
   pub prm_per_task: HashMap<String, ProcessRewardModel>,
   ```
2. After each gate pipeline run, record a `TurnSnapshot`:
   ```rust
   let snapshot = TurnSnapshot {
       rung: completion.rung,
       verdicts: completion.verdicts.iter().map(|v| /* convert */).collect(),
       error_count: completion.verdicts.iter().filter(|v| !v.passed).count() as u32,
       diff_lines: 0, // Filled from agent output if available
   };
   let prm = run_state.prm_per_task.entry(task_id.clone()).or_insert_with(ProcessRewardModel::new);
   prm.history.push(snapshot);
   ```
3. Compute and log promise/progress:
   ```rust
   let promise = prm.promise();
   let progress = prm.progress();
   tracing::info!(task_id = %task_id, promise, progress, "PRM signals");
   ```

#### Verification Criteria
- [ ] PRM is created per task and updated on each gate completion
- [ ] Promise and progress values are logged
- [ ] History grows with each attempt

---

### Task 4.18: Act on PRM signals (early termination, model switch)
**Priority**: P2
**Estimated Effort**: 3 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/event_loop.rs`
**Depends On**: Task 4.17

#### Context
With PRM tracking per-task trajectories, the runner can make informed decisions about whether to continue, abandon, or change strategy.

#### Implementation Steps
1. Define threshold constants:
   ```rust
   const PRM_ABANDON_THRESHOLD: f64 = 0.15;
   const PRM_STALL_THRESHOLD: f64 = -0.05;
   const PRM_STALL_MIN_TURNS: usize = 3;
   ```
2. After computing PRM signals, act on them:
   ```rust
   if promise < PRM_ABANDON_THRESHOLD && prm.history.len() >= 3 {
       tracing::warn!(task_id, promise, "PRM: low promise, abandoning task");
       // Mark task as failed with reason "PRM: low promise"
   } else if progress < PRM_STALL_THRESHOLD && prm.history.len() >= PRM_STALL_MIN_TURNS {
       tracing::warn!(task_id, progress, "PRM: stalled progress, consider model switch");
       // Emit RunnerEvent::ModelSwitchSuggested
   }
   ```
3. Log PRM signals to efficiency events for post-run analysis.

#### Verification Criteria
- [ ] Tasks with consistently declining promise are abandoned early
- [ ] Stalled tasks emit model switch suggestions
- [ ] PRM signals appear in efficiency events

---

## Phase 7: Custom Gates from Config

### Task 4.19: Define custom gate config schema
**Priority**: P2
**Estimated Effort**: 2 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/foundation.rs`
**Depends On**: Task 4.1

#### Context
Currently, custom gates are configured via `ShellGateCommand` in `GateConfig.shell_gates`. This supports arbitrary shell commands but requires callers to construct the vector programmatically. A config-driven approach (from `roko.toml`) enables user extensibility without code changes.

#### Implementation Steps
1. Add `CustomGateSpec` to `crates/roko-core/src/foundation.rs`:
   ```rust
   #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
   pub struct CustomGateSpec {
       /// Gate name (used in enabled_gates list).
       pub name: String,
       /// Program to invoke.
       pub program: String,
       /// Arguments.
       #[serde(default)]
       pub args: Vec<String>,
       /// Timeout in milliseconds (default: 60000).
       #[serde(default = "default_custom_gate_timeout")]
       pub timeout_ms: u64,
       /// Which rung to assign (for ordering). Default: 5 (custom).
       #[serde(default = "default_custom_rung")]
       pub rung: u8,
       /// Whether non-empty stderr should fail the gate.
       #[serde(default)]
       pub fail_on_stderr: bool,
   }
   ```
2. Add `custom_gates: Vec<CustomGateSpec>` to `GateConfig`.
3. Update all construction sites with `custom_gates: vec![]`.

#### Verification Criteria
- [ ] `cargo check --workspace` compiles
- [ ] `CustomGateSpec` is serializable/deserializable
- [ ] Default timeout and rung values are sensible

---

### Task 4.20: Wire custom gates into GateService
**Priority**: P2
**Estimated Effort**: 3 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-gate/src/gate_service.rs`
**Depends On**: Task 4.19

#### Context
GateService currently resolves gate names via `gate_for_name()` which returns concrete gate implementations for known names (compile, clippy, test, diff, fmt) and returns `None` for unknown names. Custom gates from config should be resolvable by name.

#### Implementation Steps
1. Add a `custom_gates: HashMap<String, CustomGateSpec>` field to `GateService`.
2. Add builder method:
   ```rust
   pub fn with_custom_gates(mut self, specs: Vec<CustomGateSpec>) -> Self {
       for spec in specs {
           self.custom_gates.insert(spec.name.clone(), spec);
       }
       self
   }
   ```
3. In `run_gates()`, when processing gate names, check custom_gates before the default `gate_for_name()`:
   ```rust
   if let Some(custom) = self.custom_gates.get(&gate_name) {
       let gate = ShellGate::new(&custom.program, custom.args.clone())
           .with_name(&custom.name)
           .with_timeout_ms(custom.timeout_ms);
       // Run gate...
   }
   ```
4. Override `rung_for_name()` to check custom gates:
   ```rust
   fn rung_for_name_with_custom(&self, name: &str) -> Option<u8> {
       if let Some(custom) = self.custom_gates.get(name) {
           Some(custom.rung)
       } else {
           Self::rung_for_name(name)
       }
   }
   ```
5. In the ServiceFactory or WorkflowEngine config builder, read `[[gate.custom]]` sections from `roko.toml` and pass them as `CustomGateSpec` to GateService.

#### Verification Criteria
- [ ] `cargo test -p roko-gate` passes
- [ ] Custom gate "my-check" with program "make" and args ["lint"] executes via GateService
- [ ] Custom gates are ordered by their configured rung value
- [ ] Unknown custom gate names produce skipped verdicts, not panics

---

## Phase 8: Gate Events for UX

### Task 4.21: Define GateEvent enum
**Priority**: P2
**Estimated Effort**: 1 hour
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/runtime_event.rs`
**Depends On**: none

#### Context
`RuntimeEvent` at `crates/roko-core/src/runtime_event.rs:56` already has `GateStarted`, `GatePassed`, `GateFailed` variants with `run_id`, `gate_name`, `rung`, `duration_ms`, `output`. These exist but are only emitted from the WorkflowEngine's EffectDriver path. GateService does not emit them.

Rather than adding new event types, extend the existing `RuntimeEvent` variants with additional fields for SPC alerts and threshold updates, or add new variants for pipeline-level events.

#### Implementation Steps
1. Add new variants to `RuntimeEvent`:
   ```rust
   GateSkipped {
       run_id: String,
       gate_name: String,
       reason: String,
   },
   GatePipelineCompleted {
       run_id: String,
       passed: bool,
       duration_ms: u64,
       gates_run: usize,
       gates_skipped: usize,
       joint_anomaly: bool,
   },
   GateSpcAlert {
       run_id: String,
       rung: u8,
       alert_kind: String,
       detail: String,
   },
   GateThresholdUpdated {
       run_id: String,
       rung: u8,
       old_ema: f64,
       new_ema: f64,
   },
   ```
2. Update `run_id()` and `kind()` match arms for the new variants.
3. Update `Display` impl for new variants.

#### Verification Criteria
- [ ] `cargo check --workspace` compiles
- [ ] New RuntimeEvent variants have consistent structure
- [ ] `kind()` returns correct labels for new variants

---

### Task 4.22: Emit RuntimeEvents from GateService
**Priority**: P2
**Estimated Effort**: 3 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-gate/src/gate_service.rs`
**Depends On**: Task 4.21

#### Context
GateService runs gates silently -- no events are emitted during execution. The TUI and dashboard only see results after the full pipeline completes. Emitting per-gate events enables real-time progress display.

#### Implementation Steps
1. Add an event sink to GateService:
   ```rust
   pub struct GateService {
       adaptive: Option<Arc<Mutex<AdaptiveThresholds>>>,
       temperament: Temperament,
       event_sink: Option<tokio::sync::mpsc::Sender<RuntimeEvent>>,
       run_id: String,
   }
   ```
2. Add builder method:
   ```rust
   pub fn with_event_sink(mut self, sink: mpsc::Sender<RuntimeEvent>, run_id: String) -> Self {
       self.event_sink = Some(sink);
       self.run_id = run_id;
       self
   }
   ```
3. Emit events in the `run_gates()` loop:
   ```rust
   // Before gate execution:
   self.emit(RuntimeEvent::GateStarted {
       run_id: self.run_id.clone(),
       gate_name: gate_name.clone(),
       rung,
   });

   // After gate execution:
   if verdict.passed {
       self.emit(RuntimeEvent::GatePassed { ... });
   } else {
       self.emit(RuntimeEvent::GateFailed { ... });
   }

   // On skip:
   self.emit(RuntimeEvent::GateSkipped { ... });
   ```
4. After the full pipeline:
   ```rust
   self.emit(RuntimeEvent::GatePipelineCompleted { ... });
   ```
5. Add a non-blocking emit helper:
   ```rust
   fn emit(&self, event: RuntimeEvent) {
       if let Some(sink) = &self.event_sink {
           let _ = sink.try_send(event);
       }
   }
   ```

#### Verification Criteria
- [ ] `cargo test -p roko-gate` passes
- [ ] Events are emitted for each gate start/pass/fail/skip
- [ ] Pipeline completion event includes aggregate statistics
- [ ] When no event sink is configured, no events are emitted (no-op)

---

### Task 4.23: Consume gate events in TUI bridge
**Priority**: P2
**Estimated Effort**: 3 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/tui_bridge.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/verdicts.rs`
**Depends On**: Task 4.22

#### Context
The TUI's verdicts tab (`crates/roko-cli/src/tui/verdicts.rs:85` `VerdictsAggregator`) currently reads from the substrate after facts. With RuntimeEvent gate events, the TUI can show real-time gate progress.

The TUI bridge (`crates/roko-cli/src/runner/tui_bridge.rs`) translates between runtime events and StateHub dashboard events.

#### Implementation Steps
1. In `tui_bridge.rs`, handle the new gate RuntimeEvent variants:
   ```rust
   RuntimeEvent::GateStarted { gate_name, .. } => {
       state_hub.update_gate_status(&gate_name, "running");
   }
   RuntimeEvent::GatePassed { gate_name, duration_ms, .. } => {
       state_hub.update_gate_status(&gate_name, "passed");
       state_hub.update_gate_duration(&gate_name, duration_ms);
   }
   RuntimeEvent::GateFailed { gate_name, output, duration_ms, .. } => {
       state_hub.update_gate_status(&gate_name, "failed");
       state_hub.update_gate_detail(&gate_name, &output);
   }
   RuntimeEvent::GateSkipped { gate_name, reason, .. } => {
       state_hub.update_gate_status(&gate_name, "skipped");
       state_hub.update_gate_detail(&gate_name, &reason);
   }
   ```
2. In the verdicts tab, render real-time gate status from StateHub events.
3. Show SPC alerts and joint anomaly warnings prominently in the TUI.

#### Verification Criteria
- [ ] Gate progress is visible in TUI during execution (not just after completion)
- [ ] Skipped gates show reason
- [ ] SPC alerts are surfaced in the TUI

---

## Phase 9: Cleanup and Consolidation

### Task 4.24: Remove dead gate dispatch code
**Priority**: P1
**Estimated Effort**: 2 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-acp/src/runner.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/run.rs`
**Depends On**: Task 4.5, Task 4.6

#### Context
After Tasks 4.5 and 4.6, the old inline gate dispatch code in ACP runner and `roko run` is replaced by GateService calls. The old code should be removed to prevent drift.

#### Implementation Steps
1. Remove the old `run_gates()` function body in `crates/roko-acp/src/runner.rs` (the old ~100 line implementation that directly constructs CompileGate/TestGate/ClippyGate).
2. Remove the `#[cfg(feature = "legacy-orchestrate")] async fn run_gate()` in `crates/roko-cli/src/run.rs` if the legacy feature flag is no longer needed.
3. Remove unused imports of `CompileGate`, `TestGate`, `ClippyGate` from migrated files.
4. Run `cargo clippy --workspace --no-deps -- -D warnings` to catch dead code.

#### Verification Criteria
- [ ] `cargo clippy --workspace --no-deps -- -D warnings` passes clean
- [ ] No direct CompileGate/TestGate/ClippyGate construction outside of roko-gate crate
- [ ] `grep -rn 'CompileGate::new\|TestGate::new\|ClippyGate::new' crates/ --include='*.rs' | grep -v roko-gate | grep -v test` returns empty

---

### Task 4.25: Consolidate GatePipeline and ComposedGatePipeline
**Priority**: P2
**Estimated Effort**: 3 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-gate/src/gate_pipeline.rs`
**Depends On**: none

#### Context
`GatePipeline` and `ComposedGatePipeline` in `crates/roko-gate/src/gate_pipeline.rs` are parallel implementations (AP-DUP). `ComposedGatePipeline::Sequential` mode re-implements the loop from `GatePipeline` rather than delegating. There is dead code: `let _ = pipeline;`.

#### Implementation Steps
1. Make `ComposedGatePipeline::Sequential` delegate to `GatePipeline::verify()`:
   ```rust
   GateComposition::Sequential => {
       let pipeline = GatePipeline::new(&self.name);
       // Actually use the pipeline
       for gate in &self.gates {
           pipeline.push(gate.clone());
       }
       return pipeline.verify(signal, ctx).await;
   }
   ```
2. Or deprecate `GatePipeline` in favor of `ComposedGatePipeline` with a default Sequential mode.
3. Remove the dead `let _ = pipeline;` code.
4. Add `#[deprecated]` annotation if keeping both types.

#### Verification Criteria
- [ ] `cargo test -p roko-gate` passes (all 19 pipeline tests)
- [ ] No dead code warnings from `GatePipeline` usage
- [ ] Sequential mode behavior is identical (short-circuit semantics preserved)

---

### Task 4.26: Add GateService integration tests
**Priority**: P1
**Estimated Effort**: 4 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-gate/tests/gate_service_integration.rs` (new file)
**Depends On**: Task 4.3, Task 4.4, Task 4.8

#### Context
Existing integration tests at `crates/roko-gate/tests/gate_truth.rs` (6 tests) and `crates/roko-gate/tests/rungs.rs` (9 tests) test GateService and rung dispatch separately. After the convergence work, we need integration tests that verify the full pipeline including feedback, classification, SPC alerts, rung selection, and temperament.

#### Implementation Steps
1. Create `crates/roko-gate/tests/gate_service_integration.rs`.
2. Add tests:
   - `test_feedback_generated_on_failure`: GateService returns `feedback: Some(...)` with classified errors when compile fails.
   - `test_rung_selection_trivial`: `complexity: Some(0)` runs only compile.
   - `test_rung_selection_escalation`: `prior_failures: Some(3)` escalates Trivial to Complex.
   - `test_stub_verdicts_are_skipped`: Missing inputs produce skipped (not passed) verdicts.
   - `test_adaptive_skip_respected`: After 20 consecutive passes, rung is skipped (except compile).
   - `test_temperament_conservative_no_skip`: Conservative temperament never skips.
   - `test_spc_alerts_in_report`: After regime change, SPC alerts appear in report.
   - `test_custom_gates`: Custom gate spec executes via ShellGate.
3. Use real cargo scaffolds (tempdir with Cargo.toml + src/lib.rs) for compile/clippy/test gates.

#### Verification Criteria
- [ ] All new integration tests pass
- [ ] Tests cover the full task scope (feedback, rung selection, stubs, adaptive, SPC, custom)
- [ ] Tests do not depend on external services (all local)

---

### Task 4.27: Update GateService documentation
**Priority**: P2
**Estimated Effort**: 1 hour
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-gate/src/gate_service.rs`
**Depends On**: Task 4.3, Task 4.4, Task 4.8, Task 4.11, Task 4.12, Task 4.13, Task 4.14

#### Context
After all the convergence work, GateService's module-level documentation and struct-level documentation should reflect its new capabilities: rung selection, feedback generation, failure classification, SPC draining, Hotelling observation, domain profiles, temperament, custom gates, and event emission.

#### Implementation Steps
1. Update the module-level doc comment in `gate_service.rs` to describe the full feature set.
2. Update the `GateService` struct doc comment to list all builder methods and their purpose.
3. Update the `run_gates()` method doc comment to describe the full pipeline: rung selection -> ordering -> adaptive skip -> execution -> feedback -> classification -> SPC drain -> Hotelling -> report.
4. Add a "Usage" section showing the builder pattern:
   ```rust
   /// let svc = GateService::new()
   ///     .with_adaptive_thresholds(thresholds)
   ///     .with_temperament(Temperament::Conservative)
   ///     .with_custom_gates(custom_specs)
   ///     .with_event_sink(tx, run_id);
   ```

#### Verification Criteria
- [ ] `cargo doc -p roko-gate --no-deps` generates clean documentation
- [ ] GateService documentation describes all builder methods
- [ ] run_gates() documentation describes the full pipeline

---

## Dependency Graph

```
Phase 1 (Foundation):
  4.1  GateConfig fields        ──┐
  4.2  GateReport fields        ──┤
                                  │
Phase 2 (Convergence):            │
  4.3  GateService feedback     ←─┤ depends on 4.1, 4.2
  4.4  GateService rung select  ←─┘ depends on 4.1
  4.5  ACP migration            ←── depends on 4.3
  4.6  roko-run migration       ←── depends on 4.3
  4.7  Runner v2 alignment      ←── depends on 4.3, 4.4

Phase 3 (Stubs + Judge):
  4.8  Verdict::skip + stubs    ←── independent
  4.9  CascadeRouter judge      ←── independent
  4.10 Gate budget              ←── depends on 4.2

Phase 4 (Intelligence):
  4.11 Domain profiles          ←── depends on 4.3
  4.12 Temperament              ←── depends on 4.1
  4.13 SPC alert draining       ←── depends on 4.2, 4.3
  4.14 Hotelling observation    ←── depends on 4.2, 4.3

Phase 5 (Failure Routing):
  4.15 Failure action routing   ←── depends on 4.7
  4.16 Error pattern tracking   ←── depends on 4.15

Phase 6 (PRM):
  4.17 PRM instantiation        ←── depends on 4.7
  4.18 PRM action signals       ←── depends on 4.17

Phase 7 (Custom Gates):
  4.19 Custom gate schema       ←── depends on 4.1
  4.20 Custom gate wiring       ←── depends on 4.19

Phase 8 (Events):
  4.21 GateEvent variants       ←── independent
  4.22 GateService event emit   ←── depends on 4.21
  4.23 TUI gate event display   ←── depends on 4.22

Phase 9 (Cleanup):
  4.24 Remove dead code         ←── depends on 4.5, 4.6
  4.25 Pipeline consolidation   ←── independent
  4.26 Integration tests        ←── depends on 4.3, 4.4, 4.8
  4.27 Documentation            ←── depends on 4.3, 4.4, 4.8, 4.11-4.14
```

---

## Implementation Priority

| Phase | Tasks | Effort | Impact | Priority |
|-------|-------|--------|--------|----------|
| Phase 1: Foundation types | 4.1, 4.2 | 2 hours | Unblocks all downstream | P0 |
| Phase 2: Converge dispatch | 4.3-4.7 | 19 hours | Eliminates 4 paths, adds feedback | P0 |
| Phase 3: Fix stubs + judge | 4.8-4.10 | 8 hours | Fixes AP-1, AP-5, AP-10 | P0 |
| Phase 4: Wire intelligence | 4.11-4.14 | 9 hours | Enables SPC, Hotelling, profiles | P1 |
| Phase 5: Failure routing | 4.15-4.16 | 7 hours | Smarter retry/replan | P1 |
| Phase 6: PRM | 4.17-4.18 | 6 hours | Early termination, model switch | P2 |
| Phase 7: Custom gates | 4.19-4.20 | 5 hours | User extensibility | P2 |
| Phase 8: Gate events | 4.21-4.23 | 7 hours | Real-time visibility | P2 |
| Phase 9: Cleanup | 4.24-4.27 | 10 hours | Maintenance, quality | P1-P2 |

**Total estimated effort**: ~73 hours (~9-10 engineering days)

**Critical path**: 4.1 + 4.2 -> 4.3 -> 4.5/4.6/4.7 (parallel) -> 4.24

Phases 1-3 are the critical path. They eliminate duplicate dispatch paths, fix the most impactful anti-patterns, and give all callers feedback + classification. Everything after Phase 3 builds on the unified GateService foundation.

---

## Verification Checkpoints

After each phase, verify with:

```bash
# Compile
cargo check --workspace

# Lint
cargo clippy --workspace --no-deps -- -D warnings

# Tests
cargo test --workspace

# Anti-pattern checks (after Phase 2):
grep -rn 'CompileGate::new\|TestGate::new\|ClippyGate::new' crates/ --include='*.rs' | grep -v roko-gate | grep -v test
# Should return 0 hits

# Stub check (after Phase 3):
grep -rn 'Verdict::pass.*stub\|stub.*Verdict::pass' crates/ --include='*.rs' | grep -v test
# Should return 0 hits

# Hardcoded model check (after Phase 3):
grep -rn 'claude-sonnet-4-20250514' crates/ --include='*.rs' | grep -v test | grep -v '//'
# Should return 0 hits (only in tests and comments)

# Unused feature check (after Phase 4):
grep -rn 'drain_spc_alerts\|observe_pipeline\|ThresholdProfile::' crates/ --include='*.rs' | grep -v test
# Should return runtime callers, not just definitions
```

---

## Sources

Task breakdown derived from:

- `tmp/solutions/roko/impl/04-GATE-PIPELINE.md` -- primary implementation plan
- `tmp/solutions/roko/20-GATE-AUDIT.md` -- gate pipeline subsystem audit
- `tmp/solutions/roko/20-GATE-ISSUES.md` -- concrete issues catalog (I-1 through I-15)
- `tmp/solutions/roko/20-GATE-PLAN.md` -- phased implementation plan
- `tmp/solutions/roko/04-ORCHESTRATION-AND-GATES-AUDIT.md` -- cross-cutting orchestration audit
- `crates/roko-gate/src/gate_service.rs` -- GateService implementation (680 LOC)
- `crates/roko-gate/src/adaptive_threshold.rs` -- AdaptiveThresholds (957 LOC)
- `crates/roko-gate/src/rung_dispatch.rs` -- stub_verdict, run_rung
- `crates/roko-gate/src/rung_selector.rs` -- PlanComplexity, select_rungs
- `crates/roko-gate/src/feedback.rs` -- feedback_for_agent
- `crates/roko-gate/src/compile_errors.rs` -- classify_gate_failure, GateFailureAction
- `crates/roko-gate/src/process_reward.rs` -- ProcessRewardModel
- `crates/roko-gate/src/spc.rs` -- SpcAlert variants
- `crates/roko-gate/src/hotelling.rs` -- HotellingDetector
- `crates/roko-core/src/foundation.rs` -- GateConfig, GateReport, GateVerdict, GateRunner
- `crates/roko-core/src/verdict.rs` -- Verdict struct
- `crates/roko-core/src/runtime_event.rs` -- RuntimeEvent (existing Gate* variants)
- `crates/roko-cli/src/runner/gate_dispatch.rs` -- Runner v2 gate dispatch
- `crates/roko-cli/src/runner/event_loop.rs` -- Runner v2 event loop
- `crates/roko-acp/src/runner.rs` -- ACP gate dispatch
- `crates/roko-cli/src/run.rs` -- roko run gate dispatch
