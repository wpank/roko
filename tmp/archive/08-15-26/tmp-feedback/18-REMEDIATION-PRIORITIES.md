# Remediation Priorities — Master Action Plan

**Date**: 2026-05-05
**Branch**: `wp-arch2`
**Source**: Audit files [01-STUBS](01-STUBS.md) through [17-SAFETY-CORRECTNESS](17-SAFETY-CORRECTNESS.md), [PROGRESS.md](../taskrunner/PROGRESS.md), [MASTER-TASKS.md](../MASTER-TASKS.md)
**Scope**: 72 implemented tasks audited, 45 issues identified, 25 actionable fixes prioritized

---

## Tier 1: Critical (Blocks Self-Hosting Reliability)

These fixes close the feedback loop that makes roko learn from experience. Without them, the system records data but never improves from it. The self-hosting loop is: `dispatch -> predict -> gate -> record outcome -> update router -> dispatch (improved)`. Every break in this chain is a Tier 1 issue.

---

### 1.1 Wire `PlaybookStore::record_outcome()` into v2 runner

- **Problem**: Playbooks influence prompt assembly but receive zero success/failure feedback. Scores remain at seeded defaults forever.
- **Root cause**: Task 010 built `record_outcome()` but only wired it into the legacy `run.rs` behind `#[cfg(feature = "legacy-orchestrate")]`. The v2 runner event loop never calls it.
- **Fix**:
  1. In `crates/roko-cli/src/runner/event_loop.rs`, at the gate-completion handler (around the `GateTerminal` match arm), extract `playbook_ids` from the task's `RunState.prompt_diagnostics` field.
  2. Add a `task_playbook_ids: HashMap<String, Vec<String>>` field to `RunState` in `crates/roko-cli/src/runner/types.rs`. Populate it at dispatch time when `prompt_diagnostics` is available.
  3. At gate terminal (pass or fail), iterate over the stored playbook IDs and call `playbook_store.record_outcome(id, gate_passed)`.
  4. Ensure `PlaybookStore` is accessible from the event loop context (it may need to be added to the runner's shared state).
- **Files to modify**: `crates/roko-cli/src/runner/event_loop.rs`, `crates/roko-cli/src/runner/types.rs`
- **Effort**: 2-3 hours
- **Dependencies**: None
- **Verification**: `rg -n 'record_outcome' crates/roko-cli/src/runner/event_loop.rs` returns at least 1 hit. Run a plan with 2+ tasks, inspect `.roko/learn/playbooks/` for updated `success_count`/`failure_count` values after gate results.
- **Source**: [02-DECEPTIVE-WIRING.md](02-DECEPTIVE-WIRING.md) (Task 010), [09-LEARNING-LOOP-BROKEN.md](09-LEARNING-LOOP-BROKEN.md) (Break 1)

---

### 1.2 Wire `event_subscriber` to a runtime caller

- **Problem**: `run_learning_subscriber()` in `roko-learn` is the hub connecting gate outcomes to all learning subsystems (CalibrationPolicy, VerdictScorer, ErrorEnrichment, QualityJudge). It has zero non-test callers. The entire learning pipeline is dark.
- **Root cause**: The function was built for the old event bus architecture. The v2 runner uses a different event flow and never calls it.
- **Fix**:
  1. In `crates/roko-cli/src/runner/event_loop.rs`, after gate results are processed, construct a `LearningEvent` from the gate outcome data (task_id, model, verdict, duration, error classification).
  2. Either: (A) spawn `run_learning_subscriber()` as a background task at runner startup that listens on a `tokio::sync::mpsc` channel, feeding it events from the gate handler. Or (B) call the subscriber's individual processors inline at each gate outcome point.
  3. Option A is preferred: create a `mpsc::Sender<LearningEvent>` in the runner init, pass the receiver to `run_learning_subscriber()` in a `tokio::spawn`, and send events from the gate handler.
- **Files to modify**: `crates/roko-cli/src/runner/event_loop.rs`, `crates/roko-learn/src/event_subscriber.rs` (remove `//! STATUS: NOT WIRED` header)
- **Effort**: 4-6 hours
- **Dependencies**: None (but 1.3 requires this to have runtime effect)
- **Verification**: `rg -n 'run_learning_subscriber\|LearningEvent' crates/roko-cli/src/runner/` returns hits. Run a plan, check logs for `calibration correction triggered` or `verdict scored` messages.
- **Source**: [05-CROSS-CUTTING-ANTIPATTERNS.md](05-CROSS-CUTTING-ANTIPATTERNS.md) (Anti-pattern 1), [09-LEARNING-LOOP-BROKEN.md](09-LEARNING-LOOP-BROKEN.md) (Break 4)

---

### 1.3 Add `apply_calibration_correction()` to CascadeRouter

- **Problem**: `CalibrationPolicy.process_event()` produces corrections that are logged with `tracing::info!` and discarded. No method exists on `CascadeRouter` to apply them. The router cannot self-correct from experience.
- **Root cause**: Task 031 built the correction generation but omitted the application method. The TODO comment at `event_subscriber.rs:101` says `// TODO: apply correction to cascade router`.
- **Fix**:
  1. In `crates/roko-learn/src/cascade_router.rs`, add a method:
     ```rust
     pub fn apply_calibration_correction(&mut self, correction: &CalibrationCorrection) {
         // Adjust the tier confidence/weight for the corrected model
         // based on correction.direction (Overconfident/Underconfident)
         // and correction.magnitude
     }
     ```
  2. In `event_subscriber.rs`, replace the `// TODO` comment with an actual call: pass a `&mut CascadeRouter` (or `Arc<Mutex<CascadeRouter>>`) to the subscriber and call `apply_calibration_correction()`.
  3. Call `cascade_router.persist()` after applying corrections so they survive restarts.
- **Files to modify**: `crates/roko-learn/src/cascade_router.rs`, `crates/roko-learn/src/event_subscriber.rs`
- **Effort**: 2-3 hours
- **Dependencies**: 1.2 (event_subscriber must be wired first for corrections to be generated at runtime)
- **Verification**: `rg -n 'apply_calibration_correction' crates/roko-learn/` returns the method definition and at least one call site. After a failed gate, check `.roko/learn/cascade-router.json` for updated confidence values.
- **Source**: [02-DECEPTIVE-WIRING.md](02-DECEPTIVE-WIRING.md) (Task 031), [09-LEARNING-LOOP-BROKEN.md](09-LEARNING-LOOP-BROKEN.md) (Break 2)

---

### 1.4 Fix config dual-loader: use core loader output

- **Problem**: `load_resolved_config()` calls the core unified loader but stores its result in `_core_validated` (intentionally unused). The function returns a `ResolvedConfig` built from the legacy `ConfigLayer` system. All 30+ callsites get the wrong config. Env overrides via `ROKO__SECTION__FIELD` are silently discarded.
- **Root cause**: Task 001 agent added the core loader call "for validation side effects" but never replaced the legacy return path.
- **Fix (Option A -- incremental, recommended)**:
  1. In `crates/roko-cli/src/config.rs`, function `load_resolved_config()` around line 2899:
     - Change `let _core_validated = ...` to `let core_validated = ...?;`
     - Build `ResolvedConfig` from `core_validated.config` instead of from the legacy `ConfigLayer` merge
     - Keep the legacy path as a `#[cfg(test)]` fallback for comparison testing
  2. Run `cargo test --workspace` to catch any field name mismatches between `ValidatedConfig` and `ResolvedConfig`.
  3. Test with `ROKO__AGENT__CONTEXT_LIMIT_K=32 roko config show` -- verify the override appears.
- **Files to modify**: `crates/roko-cli/src/config.rs`
- **Effort**: 3-4 hours (the code change is small but testing all 30+ callsites requires care)
- **Dependencies**: None
- **Verification**: `rg -n '_core_validated' crates/roko-cli/src/config.rs` returns zero hits (no underscore prefix). Setting `ROKO__AGENT__CONTEXT_LIMIT_K=32` and running `roko config show` reflects the override.
- **Source**: [02-DECEPTIVE-WIRING.md](02-DECEPTIVE-WIRING.md) (Task 001), [07-CONFIG-DUAL-LOADER.md](07-CONFIG-DUAL-LOADER.md)

---

### 1.5 Fix `enable_advanced_rungs` flag (gate rung 3 at minimum)

- **Problem**: The gate pipeline claims 7 rungs but only runs 2 (compile + test). Rungs 3-6 all return `stub_verdict()` (always pass). The `enable_advanced_rungs` flag has both branches identical (both increment `skipped_count`). Agent output is validated only by "does it compile?"
- **Root cause**: Gate infrastructure was built with stubs, intending to wire later. The flag added by task 089 is a no-op.
- **Fix (rung 3 -- symbol manifest -- is the minimum viable improvement)**:
  1. In `crates/roko-gate/src/rung_dispatch.rs`, replace `dispatch_symbol_manifest()` stub:
     - Parse the task spec for expected symbols (function names, struct names from the task description)
     - After agent completion, diff the worktree against pre-task state
     - Check that expected symbols appear in the diff (use `grep` or basic text matching -- full tree-sitter can come later)
     - Return `GateVerdict::Fail` if expected symbols are missing from the diff
  2. In `orchestrate.rs` (or v2 runner equivalent), fix the `enable_advanced_rungs` flag:
     - The `true` branch should dispatch rung 3 gates
     - The `false` branch should skip them (current behavior)
  3. Update `roko.toml` schema to document `enable_advanced_rungs = true` enables symbol checking.
- **Files to modify**: `crates/roko-gate/src/rung_dispatch.rs`, `crates/roko-cli/src/orchestrate.rs` (flag fix), `crates/roko-cli/src/runner/gate_dispatch.rs` (v2 path)
- **Effort**: 6-8 hours (symbol extraction from task specs requires design; rung 3 is the simplest)
- **Dependencies**: None
- **Verification**: Run with `enable_advanced_rungs = true`. An agent that claims to add `fn new_helper()` but doesn't should fail rung 3. Check gate output for non-stub rung 3 verdicts.
- **Source**: [05-CROSS-CUTTING-ANTIPATTERNS.md](05-CROSS-CUTTING-ANTIPATTERNS.md) (Anti-pattern 2), [08-GATE-PIPELINE-FACADE.md](08-GATE-PIPELINE-FACADE.md)

---

### 1.6 Fix section outcome IDs for bandit learning (Task 034)

- **Problem**: Section outcome data is recorded but with wrong ID formats. The downstream bandit cannot learn which prompt sections are effective because: `section_id` uses raw names instead of `prompt:<normalized-name>`, `invocation_id` collides across retries, and `agent_model` may be stale.
- **Root cause**: Task 034 implemented recording but deviated from the ID format spec required by the bandit consumer.
- **Fix**:
  1. In the section outcome recording code, change `section_id` to use `format!("prompt:{}", normalize_section_name(name))`.
  2. Change `invocation_id` from `format!("{}:{}", plan_id, task_id)` to `format!("{}:{}", run_id, attempt_key)` using the unique attempt identifier.
  3. Capture `agent_model` at dispatch time (not at gate-completion time) and store it in `RunState` for later use.
- **Files to modify**: `crates/roko-cli/src/runner/event_loop.rs` (section outcome recording logic)
- **Effort**: 1-2 hours
- **Dependencies**: None
- **Verification**: Run a plan, inspect `.roko/learn/section-outcomes.jsonl`. Verify `section_id` values start with `prompt:` and `invocation_id` values are unique across retries of the same task.
- **Source**: [03-NEEDS-WORK.md](03-NEEDS-WORK.md) (Task 034), [09-LEARNING-LOOP-BROKEN.md](09-LEARNING-LOOP-BROKEN.md) (Break 6)

---

**Tier 1 expected outcome**: After these 6 fixes, the system can learn which prompts work (playbook feedback), correct model routing (calibration), validate output beyond compilation (rung 3+), use consistent configuration (core loader), and feed accurate data to its section bandit. The self-hosting loop closes.

---

## Tier 2: High (Degrades Self-Hosting Quality)

These fixes prevent crashes, data corruption, and incorrect behavior during self-hosting runs. The system will "work" without them but will crash intermittently, report wrong data, or behave incorrectly in edge cases.

---

### 2.1 TOCTOU fixes (10 patterns across 3 files)

- **Problem**: 10 check-then-act file operations where `path.exists()` is checked before reading. Race conditions cause wrong error classification, missed files, or panics.
- **Root cause**: Task 047 added tests documenting the bugs but fixed zero source code patterns.
- **Fix**: Replace every `if path.exists() { read(path) }` with `match fs::read_to_string(path) { Ok(c) => ..., Err(e) if e.kind() == NotFound => ..., Err(e) => return Err(e.into()) }`. Apply to all 10 locations listed in [17-SAFETY-CORRECTNESS.md](17-SAFETY-CORRECTNESS.md).
- **Files to modify**: `crates/roko-cli/src/runner/plan_loader.rs` (5 patterns), `crates/roko-cli/src/runner/event_loop.rs` (4 patterns), `crates/roko-cli/src/runner/extension_loader.rs` (1 pattern)
- **Effort**: 3-4 hours
- **Dependencies**: None
- **Verification**: `rg -n '\.exists\(\)' crates/roko-cli/src/runner/plan_loader.rs` returns zero check-then-read patterns. Existing tests still pass.
- **Source**: [17-SAFETY-CORRECTNESS.md](17-SAFETY-CORRECTNESS.md) (TOCTOU), [02-DECEPTIVE-WIRING.md](02-DECEPTIVE-WIRING.md) (Task 047)

---

### 2.2 Unwrap sweep in critical-path files

- **Problem**: 3,220 `.unwrap()` calls in production code. A single unexpected `None`/`Err` in the plan execution loop kills the process and loses all in-flight state (77 PlanRunner fields). No graceful degradation.
- **Root cause**: No crate-level lint enforcement. `roko-cli/src/lib.rs` and `roko-agent/src/lib.rs` have blanket clippy suppressions that hide new unwraps.
- **Fix (critical path only -- full sweep is Tier 3)**:
  1. In `crates/roko-cli/src/runner/event_loop.rs`: replace all `.unwrap()` with `?` or `.unwrap_or_default()` or `tracing::warn!` + fallback. Focus on network I/O, file I/O, JSON parsing, and lock acquisition patterns.
  2. In `crates/roko-learn/src/runtime_feedback.rs` (91 unwraps): same treatment.
  3. In `crates/roko-cli/src/orchestrate.rs` (59 unwraps): same treatment for the legacy path.
  4. In `crates/roko-orchestrator/src/dag.rs` (56 unwraps): same, focusing on edge-case DAG operations.
- **Files to modify**: `event_loop.rs`, `runtime_feedback.rs`, `orchestrate.rs`, `dag.rs` (priority order)
- **Effort**: 6-8 hours (mechanical but requires judgment on fallback behavior)
- **Dependencies**: None
- **Verification**: `.unwrap()` count in targeted files drops by >80%. `cargo test --workspace` still passes. No `clippy::unwrap_used` suppressions added.
- **Source**: [13-UNWRAP-DENSITY.md](13-UNWRAP-DENSITY.md), [05-CROSS-CUTTING-ANTIPATTERNS.md](05-CROSS-CUTTING-ANTIPATTERNS.md) (Anti-pattern 3)

---

### 2.3 Fix ACP `ready` serde attribute (Task 062)

- **Problem**: `ready` field uses `#[serde(default = "default_true")]` which serializes `"ready": false` explicitly. Spec requires omitting the field when false for backward compatibility with IDE clients.
- **Root cause**: Wrong serde attribute chosen by the implementing agent.
- **Fix**: In `crates/roko-acp/src/types.rs` around line 701, change:
  ```rust
  // FROM:
  #[serde(default = "default_true")]
  pub ready: bool,
  // TO:
  #[serde(default, skip_serializing_if = "std::ops::Not::not")]
  pub ready: bool,
  ```
- **Files to modify**: `crates/roko-acp/src/types.rs`
- **Effort**: 5 minutes
- **Dependencies**: None
- **Verification**: Serialize a `ConfigOptionValue` with `ready: false` -- the JSON output should NOT contain a `"ready"` key.
- **Source**: [16-IDE-ACP-GAPS.md](16-IDE-ACP-GAPS.md) (Task 062)

---

### 2.4 Fix bare_mode command whitelist (Task 020)

- **Problem**: Bare mode allows 6 categories (exposing 20+ commands) instead of the spec's exact 8-command whitelist. Additionally, `enhance-prd` is categorized as `"specification"` which is NOT in the allow-list, so it's hidden despite being required.
- **Root cause**: Category-based filtering was chosen instead of explicit command name matching.
- **Fix**: In `crates/roko-acp/src/session.rs`, replace `bare_mode_allows_category()` with:
  ```rust
  const BARE_MODE_COMMANDS: &[&str] = &[
      "status", "doctor", "config", "help",
      "research", "search", "enhance-prd", "analyze",
  ];
  fn bare_mode_allows_command(name: &str) -> bool {
      BARE_MODE_COMMANDS.contains(&name)
  }
  ```
- **Files to modify**: `crates/roko-acp/src/session.rs`
- **Effort**: 15 minutes
- **Dependencies**: None
- **Verification**: In bare mode, exactly 8 commands are available. `enhance-prd` is visible. `plan run` is NOT visible.
- **Source**: [16-IDE-ACP-GAPS.md](16-IDE-ACP-GAPS.md) (Task 020), [03-NEEDS-WORK.md](03-NEEDS-WORK.md) (Task 020)

---

### 2.5 Fix `tool_format` in roko.toml for Anthropic models (Task 074)

- **Problem**: All Anthropic/Claude CLI provider models have `tool_format = "openai_json"` in `roko.toml`. Should be `"anthropic_blocks"`. The `translator_for_profile()` function reads this field, so wrong values cause wrong translator selection on certain code paths.
- **Root cause**: Default template for roko.toml was copied from OpenAI config without adjusting tool format for Anthropic providers.
- **Fix**: In `roko.toml`, change `tool_format = "openai_json"` to `tool_format = "anthropic_blocks"` for all models under Anthropic/Claude providers (haiku, claude-opus, sonnet, claude-sonnet, opus).
- **Files to modify**: `roko.toml` (project root)
- **Effort**: 5 minutes
- **Dependencies**: None
- **Verification**: `rg 'tool_format.*openai_json' roko.toml` returns zero hits for Anthropic provider sections.
- **Source**: [03-NEEDS-WORK.md](03-NEEDS-WORK.md) (Task 074)

---

### 2.6 RunLedger: wire `record_agent_completed()` and add TaskCostReport

- **Problem**: RunLedger records gate outcomes and lifecycle events but per-task cost data (tokens, model, cost_usd) is absent. `record_agent_completed()` is never called from the success path. `TaskCostReport` struct does not exist. Users cannot see which tasks are expensive.
- **Root cause**: Task 015 built the ledger infrastructure but omitted the cost harvesting step from RunState at task completion.
- **Fix**:
  1. Define `TaskCostReport` in `crates/roko-cli/src/runner/types.rs`:
     ```rust
     pub struct TaskCostReport {
         pub plan_id: String, pub task_id: String,
         pub model: String, pub provider: String,
         pub tokens_in: u64, pub tokens_out: u64,
         pub cost_usd: f64, pub agent_calls: u32,
         pub outcome: String, // "pass" or "fail"
     }
     ```
  2. At gate completion in `event_loop.rs`, harvest from `RunState` and call `ledger.record_agent_completed(cost_report)`.
  3. Add `task_costs: Vec<TaskCostReport>` to `RunReport`.
  4. Print summary at plan completion.
  5. Fix duplicate `run_summary` (remove one of the two `persist_run_ledger()` calls at lines 1536/1548).
- **Files to modify**: `crates/roko-cli/src/runner/types.rs`, `crates/roko-cli/src/runner/event_loop.rs`, plan command output
- **Effort**: 3-4 hours
- **Dependencies**: None
- **Verification**: After `roko plan run`, output includes a "Task costs:" summary block. `--json` output includes `task_costs` array with per-task token/cost data.
- **Source**: [12-RUNNER-COST-TRACKING.md](12-RUNNER-COST-TRACKING.md), [02-DECEPTIVE-WIRING.md](02-DECEPTIVE-WIRING.md) (Task 015)

---

### 2.7 Wire DemurrageConsumer properly (Task 032)

- **Problem**: `DemurrageConsumer` is constructed and immediately dropped (`let _consumer = ...`). A raw timer calls `apply_demurrage()` directly, bypassing the consumer's configurable validation interval and domain multipliers.
- **Root cause**: Task 032 agent created the consumer, satisfied the type-check, and used the `_` prefix to suppress the unused warning.
- **Fix**: In `crates/roko-serve/src/lib.rs`, in `start_demurrage_timer()`:
  1. Remove the `_` prefix from `consumer`.
  2. Replace the raw `tokio::time::interval(Duration::from_secs(300))` with the consumer's own tick method: `consumer.tick().await` in the loop.
  3. If `DemurrageConsumer` doesn't have a `tick()` method, add one that respects `validation_interval` and `domain_multipliers` from config.
- **Files to modify**: `crates/roko-serve/src/lib.rs`, possibly `crates/roko-runtime/src/demurrage_consumer.rs`
- **Effort**: 1-2 hours
- **Dependencies**: None
- **Verification**: `rg -n '_consumer' crates/roko-serve/src/lib.rs` returns zero hits (no underscore prefix). Demurrage interval matches config value, not hardcoded 300s.
- **Source**: [02-DECEPTIVE-WIRING.md](02-DECEPTIVE-WIRING.md) (Task 032), [09-LEARNING-LOOP-BROKEN.md](09-LEARNING-LOOP-BROKEN.md) (Break 3)

---

### 2.8 Fix runner v2 wiring gaps (from MASTER-TASKS.md section 3)

- **Problem**: Runner v2 is missing 4 wiring points that exist in legacy orchestrate.rs: CascadeRouter persistence, AdaptiveThresholds persistence, replan-on-gate-failure, and model field forwarding to TUI.
- **Root cause**: Runner v2 was built as a replacement but these features were not ported from the legacy path.
- **Fix**:
  1. After agent completion in runner v2, persist `cascade-router.json` via `cascade_router.persist()`.
  2. After gate completion, persist `gate-thresholds.json` via `adaptive_thresholds.persist()`.
  3. Wire `build_gate_failure_plan_revision()` into the gate-failure handler.
  4. In `tui.agent_spawned()` call, pass the resolved model string instead of `String::new()`.
- **Files to modify**: `crates/roko-cli/src/runner/event_loop.rs`
- **Effort**: 3-4 hours
- **Dependencies**: None
- **Verification**: After a plan run, `.roko/learn/cascade-router.json` and `.roko/learn/gate-thresholds.json` have updated timestamps. On gate failure, a revision plan is generated. TUI shows model name, not "-".
- **Source**: [MASTER-TASKS.md](../MASTER-TASKS.md) (Section 3: Runner v2 Completion)

---

**Tier 2 expected outcome**: The system runs without crashing on unexpected inputs, reports accurate cost data, handles file races correctly, and maintains IDE protocol compatibility.

---

## Tier 3: Medium (Technical Debt)

These reduce cognitive load, enable future development, and prevent the codebase from degrading further. Not blocking self-hosting but blocking velocity.

---

### 3.1 Remove blanket clippy suppression from lib.rs

- **Problem**: `crates/roko-cli/src/lib.rs` has `#![cfg_attr(clippy, allow(clippy::all, clippy::pedantic, ...))]` which suppresses ALL clippy lints for the entire CLI crate (99% of the code). Task 014 only removed it from `main.rs`. CI clippy is meaningless for this crate.
- **Root cause**: Task 014 was scoped to `main.rs` but the real suppression is in `lib.rs`.
- **Fix**: Remove the blanket `cfg_attr(clippy, ...)` line from `lib.rs`. Add targeted `#[allow(...)]` on specific items that have legitimate reasons for suppression. Fix or suppress the resulting clippy warnings (expect 50-200 new warnings).
- **Files to modify**: `crates/roko-cli/src/lib.rs`, various files with new targeted suppressions
- **Effort**: 4-6 hours (removing is easy; fixing resulting warnings is the work)
- **Dependencies**: None
- **Verification**: `cargo clippy -p roko-cli --no-deps -- -D warnings` fails on real issues, not vacuously passes.
- **Source**: [05-CROSS-CUTTING-ANTIPATTERNS.md](05-CROSS-CUTTING-ANTIPATTERNS.md) (Anti-pattern 6), [03-NEEDS-WORK.md](03-NEEDS-WORK.md) (Task 014)

---

### 3.2 Decide: Observe/Connect/Trigger traits -- implement or delete

- **Problem**: Three of the six "verb traits" have zero implementations and zero callers. They are exported from `roko-core` as synchronous stubs. `connector.rs` has a migration note saying "prefer Connect trait once available" -- it never became available. They block Feed trait (task 097) and integration test (task 042).
- **Root cause**: Designed during architecture phase, never implemented. Tasks 039-041 (Codex batch) produced zero code.
- **Fix (recommended: implement minimal versions)**:
  1. Redesign all three to async (as specified in task specs 039-041).
  2. Add one implementation each: `StoreObserver` for Observe, `ProviderConnection` for Connect, `BusTrigger` for Trigger.
  3. Wire `StoreObserver` into `roko status`, `ProviderConnection` into `roko config providers health`.
  4. **Alternative**: If the architecture has moved past these traits, delete them from `traits.rs` and remove the `connector.rs` migration note.
- **Files to modify**: `crates/roko-core/src/traits.rs`, new files `store_observer.rs`, `provider_connection.rs`, `bus_trigger.rs` (if implementing), or just `traits.rs` + `connector.rs` (if deleting)
- **Effort**: 8-12 hours (implement) or 1-2 hours (delete)
- **Dependencies**: Blocks 097 (Feed trait) and 042 (integration test) if implementing
- **Verification**: Either `rg 'impl Observe' crates/` returns hits OR `rg 'trait Observe' crates/` returns zero hits. No middle ground.
- **Source**: [11-TRAIT-STUBS.md](11-TRAIT-STUBS.md), [01-STUBS.md](01-STUBS.md) (Tasks 039-041)

---

### 3.3 Extract PlanRunner sub-structs (Phase 1 -- non-breaking)

- **Problem**: `PlanRunner` has 77 fields and `orchestrate.rs` is ~20,000 lines. Every new subsystem adds another field. Methods take `&mut self` giving implicit access to everything.
- **Root cause**: Organic growth without refactoring boundaries.
- **Fix (Phase 1 -- mechanical grouping)**:
  1. Group PlanRunner fields into sub-structs within `orchestrate.rs`:
     ```rust
     struct PlanRunner {
         workspace: WorkspaceContext,      // workdir, config, layout
         executor: ExecutionEngine,        // DAG executor, task dispatch
         learning: LearningSubsystem,      // runtime, skills, playbooks
         behavioral: BehavioralSubsystem,  // daimon, knowledge, conductor
         safety: SafetySubsystem,          // layer, monitor, stuck detector
         tracking: TrackingSubsystem,      // costs, attribution, familiarity
         io: IoSubsystem,                  // event_log, output_sink
     }
     ```
  2. Update all `self.field` references to `self.subsystem.field`.
  3. This is purely mechanical -- no behavior change.
- **Files to modify**: `crates/roko-cli/src/orchestrate.rs`
- **Effort**: 6-8 hours (large file, many references)
- **Dependencies**: None, but coordinate with anyone else editing `orchestrate.rs`
- **Verification**: `cargo test --workspace` passes. `PlanRunner` struct definition has <15 top-level fields.
- **Source**: [10-GOD-STRUCTS.md](10-GOD-STRUCTS.md)

---

### 3.4 Consolidate config loading (delete legacy path)

- **Problem**: 8+ config entry points. `load_resolved_config()` and `load_config_unified()` coexist with different semantics. Env overrides behave differently depending on which path is used.
- **Root cause**: Incremental migration left both old and new systems load-bearing.
- **Fix (after 1.4 makes core loader authoritative)**:
  1. Delete `ConfigLayer` struct, `collect_env_override_layer()`, and the legacy merge logic from `config.rs`.
  2. Make `load_resolved_config()` a thin wrapper around `load_config_unified()`.
  3. Migrate the 5 other entry points (`load_roko_config_models()`, `load_config_or_defaults()`, etc.) to delegate to the same core path.
  4. Ideally converge on 1-2 entry points: `load_config_unified()` and a cached variant.
- **Files to modify**: `crates/roko-cli/src/config.rs`, `crates/roko-cli/src/run.rs`, `crates/roko-cli/src/unified.rs`, `crates/roko-cli/src/serve_runtime.rs`, `crates/roko-acp/src/config.rs`
- **Effort**: 8-12 hours (high -- 30+ callsites to audit and test)
- **Dependencies**: 1.4 (core loader must be authoritative first)
- **Verification**: `rg 'ConfigLayer' crates/` returns zero hits outside tests. Only 1-2 config entry points remain in production code.
- **Source**: [14-DUPLICATE-SYSTEMS.md](14-DUPLICATE-SYSTEMS.md) (Section 1), [07-CONFIG-DUAL-LOADER.md](07-CONFIG-DUAL-LOADER.md)

---

### 3.5 Wire or delete `RetryPolicy::execute()` (Task 054)

- **Problem**: Two independent `RetryPolicy` implementations exist. The core one (task 054, with async `execute()`) has zero callers. The agent one (with `ErrorClass` enum and `Retry-After` support) is actually used. Both diverge.
- **Root cause**: Task 054 built the consolidated executor but never migrated the agent's retry loop.
- **Fix (recommended: delete core implementation)**:
  The agent's `RetryPolicy` is richer (has `ErrorClass`, respects `Retry-After` headers, full-jitter). Delete the core version and keep the agent one. Or extract the agent one into core and make it the shared version.
  1. Delete `crates/roko-core/src/error/retry.rs` (or gut the `execute()` method).
  2. If other crates need retry, re-export from `roko-agent`.
- **Files to modify**: `crates/roko-core/src/error/retry.rs`, `crates/roko-core/src/error/mod.rs`
- **Effort**: 1 hour
- **Dependencies**: None
- **Verification**: `rg 'RetryPolicy' crates/roko-core/src/error/retry.rs` returns only basic config struct, not a competing `execute()`. Only one `RetryPolicy::execute()` exists in the workspace.
- **Source**: [14-DUPLICATE-SYSTEMS.md](14-DUPLICATE-SYSTEMS.md) (Section 2)

---

### 3.6 Health check degradation detection (Task 023)

- **Problem**: Health endpoint returns `"degraded"` as a possible value, but the threshold logic is wrong: no error-rate constants, no latency p95 check, classification uses `consecutive_failures == 0` instead of `HealthState` enum, returns `"down"` instead of spec's `"unhealthy"`, missing `providers.degraded` count.
- **Root cause**: Codex agent built the endpoint shape without the correct business logic.
- **Fix**:
  1. Add threshold constants: `DEGRADED_ERROR_RATE_THRESHOLD`, `DEGRADED_LATENCY_P95_MS`.
  2. Use `HealthState` enum for classification: `Healthy | Degraded | Unhealthy`.
  3. Check both error rate and latency p95 for degradation detection.
  4. Return `"unhealthy"` instead of `"down"`.
  5. Add `providers.degraded` count to JSON response.
  6. Add 4 required tests.
- **Files to modify**: Health check route handler in `crates/roko-serve/src/routes/` (health endpoint)
- **Effort**: 3-4 hours
- **Dependencies**: None
- **Verification**: Hit `/api/health` with a provider in degraded state. Response includes correct classification and `providers.degraded` count.
- **Source**: [02-DECEPTIVE-WIRING.md](02-DECEPTIVE-WIRING.md) (Task 023)

---

### 3.7 Provider error mapping (Task 090)

- **Problem**: `map_provider_error()` exists with tests but is never called in the actual dispatch path. Raw HTTP 401/connection errors arrive undecorated to users.
- **Root cause**: Task 090 built the mapping function but skipped the wiring step.
- **Fix**: In the provider dispatch path (where `ProviderError` is first constructed), wrap with `map_provider_error()` before returning to the caller. This should be at the boundary between provider adapters and the agent dispatch layer.
- **Files to modify**: `crates/roko-agent/src/provider/mod.rs` or equivalent dispatch boundary
- **Effort**: 1-2 hours
- **Dependencies**: None
- **Verification**: Disconnect network. `roko run "test"` shows a user-friendly error message instead of raw HTTP/connection error.
- **Source**: [03-NEEDS-WORK.md](03-NEEDS-WORK.md) (Task 090)

---

**Tier 3 expected outcome**: Linting catches real issues, architecture is decomposed into manageable pieces, config is unified, duplicate systems are resolved, and error messages are user-friendly.

---

## Tier 4: Low (Polish)

These prevent future regressions, improve developer experience, and clean up cosmetic issues.

---

### 4.1 Add missing unit tests (13+ tasks have test gaps)

- **Problem**: Nearly every NEEDS_WORK task is missing spec-required tests. The code is correct but unprotected from regression.
- **Root cause**: Both Claude and Codex agents consistently skip test writing when implementation runs long.
- **Fix**: Add tests for these task gaps (priority by risk):
  - Task 013: SSE replay 256-cap unit test
  - Task 019: MCP transport/spawn failure tests
  - Task 026: serde roundtrip, bus-level, property-based tests for TopicFilter
  - Task 044: `#[tokio::test(start_paused = true)]` timeout test
  - Task 061: `effective_max_output()` edge case tests
  - Task 064: 6 fallback scenario tests
  - Task 072: `session_banner_label()` tests
- **Files to modify**: Various test modules across crates
- **Effort**: 4-6 hours total (30-45 min per task)
- **Dependencies**: None (tests for existing code)
- **Verification**: `cargo test --workspace` shows new test names. Coverage for tested functions increases.
- **Source**: [03-NEEDS-WORK.md](03-NEEDS-WORK.md) (Pattern: Missing Tests section)

---

### 4.2 Fix broken `cell_execute` integration test (Task 035)

- **Problem**: `crates/roko-core/tests/cell_execute.rs` imports `roko_core::Substrate` which is NOT in the crate's public API. `lib.rs` exports `Store` but not `Substrate`. The test doesn't compile.
- **Root cause**: Agent used wrong import name.
- **Fix**: Change import from `roko_core::Substrate` to `roko_core::Store` in the test file. Or add `pub use` for `Substrate` in `lib.rs`.
- **Files to modify**: `crates/roko-core/tests/cell_execute.rs` OR `crates/roko-core/src/lib.rs`
- **Effort**: 5 minutes
- **Dependencies**: None
- **Verification**: `cargo test -p roko-core --test cell_execute` compiles and passes.
- **Source**: [03-NEEDS-WORK.md](03-NEEDS-WORK.md) (Task 035)

---

### 4.3 Wire `DEFAULT_RUNNER_RETRY_STRATEGY_PIVOT_ATTEMPT` to callsites (Task 079)

- **Problem**: Constant defined in `defaults.rs` but never imported. Raw `3` still exists at 2 callsites.
- **Root cause**: Task 079 defined the constant but forgot to replace the literals.
- **Fix**: In `snapshot_writer.rs:167` and `state.rs:563`, replace `3` with `DEFAULT_RUNNER_RETRY_STRATEGY_PIVOT_ATTEMPT`.
- **Files to modify**: `crates/roko-cli/src/runner/snapshot_writer.rs`, `crates/roko-cli/src/runner/state.rs`
- **Effort**: 5 minutes
- **Dependencies**: None
- **Verification**: `rg 'DEFAULT_RUNNER_RETRY_STRATEGY_PIVOT_ATTEMPT' crates/roko-cli/src/runner/` returns 3+ hits (definition + 2 usage sites).
- **Source**: [03-NEEDS-WORK.md](03-NEEDS-WORK.md) (Task 079)

---

### 4.4 Add SIGTERM handler to `roko dev` (Task 049)

- **Problem**: Container runtimes send SIGTERM to stop processes. `roko dev` only handles SIGINT (ctrl+c). On SIGTERM, PID file isn't cleaned up, blocking next start.
- **Root cause**: Task 049 implemented SIGINT only.
- **Fix**: Add `tokio::signal::unix::signal(SignalKind::terminate())` to the `tokio::select!` in the dev command handler.
- **Files to modify**: `crates/roko-cli/src/commands/dev.rs`
- **Effort**: 10 minutes
- **Dependencies**: None
- **Verification**: Send SIGTERM to `roko dev` process. PID file is cleaned up. Next `roko dev` starts without "port already in use."
- **Source**: [17-SAFETY-CORRECTNESS.md](17-SAFETY-CORRECTNESS.md) (Signal Handling)

---

### 4.5 Frontend: replace polling with SSE

- **Problem**: `useLiveApi.ts` polls `/api/health` every 5s; `useRokoConfig.ts` polls `/api/config` every 15s. SSE alternatives already exist (`useServerConnected()`, `config_reloaded` event).
- **Root cause**: Task 087 completed 4/6 sub-tasks; these two remain.
- **Fix**:
  1. In `useLiveApi.ts`: remove `setInterval(probeServer, 5000)` module-scope code. Replace with `useServerConnected()` from EventStreamContext.
  2. In `useRokoConfig.ts`: remove 15s `setInterval`. Use one initial `fetch('/api/config')` + subscribe to `config_reloaded` SSE event for updates.
  3. In `useBlockStream.ts`: split `reconnectTimer` into `pollIntervalRef` and `reconnectTimerRef`.
- **Files to modify**: `demo/demo-app/src/hooks/useLiveApi.ts`, `useRokoConfig.ts`, `useBlockStream.ts`
- **Effort**: 1-2 hours
- **Dependencies**: None
- **Verification**: Browser network tab shows no periodic `/api/health` or `/api/config` polling. SSE connection handles state updates.
- **Source**: [15-FRONTEND-GAPS.md](15-FRONTEND-GAPS.md)

---

### 4.6 Pin `rust-toolchain.toml` and harden CI (Task 048)

- **Problem**: `rust-toolchain.toml` says `channel = "stable"` (unpinned). CI uses `dtolnay/rust-toolchain@stable`. Any stable release can break the build.
- **Root cause**: Task 048 (Codex batch) produced zero changes.
- **Fix**:
  1. Change `rust-toolchain.toml` to `channel = "1.95.0"` (or current stable).
  2. Update `.github/workflows/ci.yml` to use `dtolnay/rust-toolchain@1.95.0`.
  3. Split test job into unit (`--lib --bins`) and integration (`--tests`, serialized).
  4. Fix port-race in `smoke.rs` (bind to :0, pass listener).
- **Files to modify**: `rust-toolchain.toml`, `.github/workflows/ci.yml`, `crates/roko-serve/tests/smoke.rs`
- **Effort**: 2-3 hours
- **Dependencies**: None
- **Verification**: `cat rust-toolchain.toml | grep '1.95'` succeeds. CI has separate unit/integration jobs.
- **Source**: [01-STUBS.md](01-STUBS.md) (Task 048), [17-SAFETY-CORRECTNESS.md](17-SAFETY-CORRECTNESS.md) (Port race)

---

### 4.7 Silent error swallowing: remaining 4 targets (Task 050)

- **Problem**: 4 high-priority silent error locations were missed by task 050:
  - `routes/deployments.rs` -- deployment persistence errors silent
  - `routes/gateway.rs:975` -- bandit load failure silent
  - `routes/vision_loop.rs:242` -- child.kill() unlogged
  - `anthropic_api/tool_loop.rs:340` -- cache marker failure silent
  - Additionally, `routes/plans.rs` returns 200 OK when snapshot write fails.
- **Root cause**: Task 050 hit 7 files but missed these 4.
- **Fix**: Add `tracing::warn!` to each silent error path. Fix `routes/plans.rs` to return 500 when snapshot write fails.
- **Files to modify**: Listed above
- **Effort**: 1 hour
- **Dependencies**: None
- **Verification**: `rg 'let _ =' crates/roko-serve/src/routes/` returns zero unlogged patterns in the 4 target files.
- **Source**: [03-NEEDS-WORK.md](03-NEEDS-WORK.md) (Task 050)

---

### 4.8 Signal rename: flip struct (Task 037)

- **Problem**: `pub struct Engram` is still the canonical type. `Signal` is an alias. Downstream crates use `Signal` (correct) but docs, auto-complete, and error messages show `Engram`.
- **Root cause**: Task 037 (Codex batch) produced zero changes. Task 038 (propagation) went ahead anyway using the alias.
- **Fix**:
  1. In `crates/roko-core/src/engram.rs`: rename `pub struct Engram` to `pub struct Signal`.
  2. Add `#[deprecated(since = "0.2.0")] pub type Engram = Signal`.
  3. In `signal.rs`: flip from `pub use engram::{Engram as Signal}` to direct export.
  4. Rename `Datum::Engram` to `Datum::Signal` and `is_engram()` to `is_signal()`.
  5. Update roko-core internal references.
- **Files to modify**: `crates/roko-core/src/engram.rs`, `crates/roko-core/src/signal.rs`, `crates/roko-core/src/datum.rs` (or wherever `Datum` lives)
- **Effort**: 2-3 hours
- **Dependencies**: None
- **Verification**: `rg 'pub struct Engram' crates/roko-core/` returns zero hits. `rg 'pub struct Signal' crates/roko-core/` returns 1 hit.
- **Source**: [01-STUBS.md](01-STUBS.md) (Task 037), [14-DUPLICATE-SYSTEMS.md](14-DUPLICATE-SYSTEMS.md) (Section 4)

---

**Tier 4 expected outcome**: Tests catch regressions, CI is deterministic, naming is consistent, frontend is efficient, errors are visible.

---

## Tier 5: Deferred (Blocked or Low Priority)

Brief descriptions with blocking reasons.

| # | Issue | Blocking Reason | When to Revisit |
|---|-------|----------------|-----------------|
| 5.1 | Feed trait + FileWatchFeed + ProviderHealthFeed (097) | Blocked on Observe trait (3.2). Also requires async trait design. | After 3.2 is resolved |
| 5.2 | Predict-publish-correct (100) | Blocked on CalibrationPolicy wiring (1.3) and event_subscriber (1.2). Full spec is large. | After Tier 1 learning loop closes |
| 5.3 | Gate rungs 4-7 (verify chain, fact check, LLM judge, integration) | Rung 4 needs chain infrastructure (Phase 3+). Rung 6 needs LLM API call budget. Rung 7 needs integration test suite per feature area. | After rung 3 (1.5) is validated in production |
| 5.4 | `run.rs` legacy-orchestrate feature flag (3,662 LOC dead code) | Will be deleted when runner v2 replaces all paths (MASTER-TASKS Section 3, Phase D) | After runner v2 is default |
| 5.5 | TuiState 126 fields (god struct) | Needs design phase; mechanical grouping is safe but architectural boundaries need thought | After PlanRunner decomposition (3.3) proves the pattern |
| 5.6 | ContextBidder unification (3 incompatible definitions) | Architectural decision needed: which definition wins? Runtime bidders have richer semantics but live in floating modules. | After floating modules are wired (1.2) |
| 5.7 | roko-graph crate (10 tasks, Wave 3) | Entire new crate from scratch. Biggest single deliverable. Blocks waves 4-5. | When Waves 0-2 are 100% complete |
| 5.8 | Chain runtime integration | Blocked on blockchain backend (Phase 3+) | When chain infra is available |
| 5.9 | Dreams cron trigger | Built but no automatic scheduling. Needs daemon or cron integration. | After daemon is stable |
| 5.10 | Knowledge-informed model routing | Neuro store not consulted by CascadeRouter for model selection | After learning loop is closed (Tier 1) |
| 5.11 | `compat.rs` Mori migration | Actively modified. May still be needed for users migrating from Mori. | Ask user if migration is complete |
| 5.12 | Non-atomic JSONL append (episode, efficiency, run-ledger) | Low probability of corruption. Fix is medium complexity (buffered writes + flush). | When crash recovery is prioritized |

---

## Dependency Graph

```
                    +---------+
                    | 1.4     |
                    | Config  |
                    | Loader  |
                    +----+----+
                         |
                         v
                    +---------+
                    | 3.4     |
                    | Config  |
                    | Consolid|
                    +---------+

+---------+         +---------+         +---------+
| 1.1     |         | 1.2     |         | 1.5     |
| Playbook|         | Event   |         | Gate    |
| Outcome |         | Subscrib|         | Rung 3  |
+---------+         +----+----+         +---------+
                         |
                    +----+----+
                    | 1.3     |
                    | Calibrat|
                    | Correct |
                    +----+----+
                         |
              +----------+----------+
              |                     |
         +----+----+          +----+----+
         | 5.2     |          | 5.6     |
         | Predict |          | Context |
         | Publish |          | Bidder  |
         | Correct |          | Unify   |
         +---------+          +---------+

+---------+         +---------+
| 3.2     |         | 3.3     |
| Trait   |         | PlanRun |
| Decision|         | Extract |
+----+----+         +---------+
     |                   |
+----+----+         +----+----+
| 5.1     |         | 5.5     |
| Feed    |         | TuiState|
| Trait   |         | Extract |
+---------+         +---------+

Independent (no dependencies, can run in parallel):
  1.1, 1.4, 1.5, 1.6, 2.1, 2.2, 2.3, 2.4, 2.5, 2.6, 2.7, 2.8,
  3.1, 3.3, 3.5, 3.6, 3.7,
  4.1-4.8
```

---

## Suggested Execution Order (Optimal Parallelism)

### Sprint 1 (1 session, ~8 hours): Close the Learning Loop + Quick Wins

**Parallel track A** (learning loop): 1.1 -> 1.2 -> 1.3 (serial dependency)
**Parallel track B** (independent): 1.4, 1.6 (can run simultaneously with track A)
**Parallel track C** (quick wins): 2.3, 2.4, 2.5, 4.2, 4.3, 4.4 (all <15 min each, batch together)

**Sprint 1 delivers**: Learning loop closed, config fixed, IDE protocol correct, 6 quick wins done.

### Sprint 2 (1 session, ~8 hours): Quality + Gate Improvement

**Parallel track A**: 1.5 (gate rung 3 -- largest item)
**Parallel track B**: 2.1 (TOCTOU), 2.7 (DemurrageConsumer)
**Parallel track C**: 2.2 (unwrap sweep -- critical path files only)
**Parallel track D**: 2.6 (RunLedger cost tracking), 2.8 (runner v2 wiring gaps)

**Sprint 2 delivers**: Gates validate beyond compile, race conditions fixed, crash surface reduced, cost visibility.

### Sprint 3 (1-2 sessions, ~12 hours): Architecture + Cleanup

**Parallel track A**: 3.1 (clippy suppression removal -- generates warning list) -> fix warnings
**Parallel track B**: 3.2 (trait decision -- needs design), 3.3 (PlanRunner extraction)
**Parallel track C**: 3.4 (config consolidation -- depends on 1.4 from Sprint 1), 3.5 (RetryPolicy)
**Parallel track D**: 3.6 (health degradation), 3.7 (provider error mapping)

**Sprint 3 delivers**: Linting works, architecture is decomposed, duplicates resolved.

### Sprint 4 (ongoing): Polish + Tests

All Tier 4 items can be parallelized across agents:
- 4.1 (tests), 4.5 (frontend), 4.6 (CI), 4.7 (error swallowing), 4.8 (signal rename)

**Sprint 4 delivers**: Test coverage, CI determinism, frontend efficiency, naming consistency.

---

## Team Assignment Recommendations

### Good for Opus agents (well-scoped, file targets clear):
- 1.1 (Playbook wiring -- add calls at specific locations)
- 1.3 (CalibrationCorrection -- add method + one call)
- 1.6 (Section outcome IDs -- format string changes)
- 2.1 (TOCTOU -- mechanical pattern replacement)
- 2.2 (Unwrap sweep -- mechanical replacement with judgment)
- 2.3 (ACP serde -- one-line change)
- 2.4 (Bare mode whitelist -- small function replacement)
- 2.5 (tool_format -- config value changes)
- 2.7 (DemurrageConsumer -- remove underscore, wire method)
- 3.1 (Clippy -- remove suppression, fix warnings)
- 3.5 (RetryPolicy -- delete file)
- 4.1-4.8 (all polish items)

### Needs human review/design decision:
- 1.2 (Event subscriber wiring -- architectural choice: channel vs inline)
- 1.4 (Config loader -- 30+ callsite impact needs careful testing)
- 1.5 (Gate rung 3 -- design: how to extract expected symbols from task specs)
- 2.8 (Runner v2 wiring -- needs knowledge of both old and new runner)
- 3.2 (Observe/Connect/Trigger -- implement vs delete decision)
- 3.3 (PlanRunner extraction -- grouping boundaries need design)
- 3.4 (Config consolidation -- high callsite count, regression risk)
- 5.6 (ContextBidder unification -- architectural decision)

---

## Risk Matrix

| Item | Probability of Occurrence | Impact if Left Unfixed | Risk Score | Priority |
|------|--------------------------|----------------------|------------|----------|
| 1.1 Playbook outcomes | Certain (already broken) | HIGH: prompts never improve | 10 | Fix now |
| 1.2 Event subscriber | Certain (already dark) | HIGH: all learning subsystems dead | 10 | Fix now |
| 1.3 Calibration correction | Certain (already broken) | HIGH: router never self-corrects | 9 | Fix now |
| 1.4 Config dual-loader | Certain (already wrong) | HIGH: env overrides silently ignored | 9 | Fix now |
| 1.5 Gate rung 3 | Certain (stubs return pass) | CRITICAL: bad code passes gates | 9 | Fix now |
| 1.6 Section outcome IDs | Certain (wrong format) | MEDIUM: bandit cannot learn | 7 | Fix now |
| 2.1 TOCTOU | Low (race window small) | MEDIUM: wrong errors, missed files | 4 | Fix soon |
| 2.2 Unwrap density | Medium (unexpected None/Err) | CRITICAL: process crash, state loss | 8 | Fix soon |
| 2.3 ACP ready serde | Certain (always wrong) | LOW: IDE cosmetic issue | 3 | Fix soon |
| 2.4 Bare mode whitelist | Certain (wrong commands) | LOW: IDE sees too many commands | 3 | Fix soon |
| 2.5 tool_format | Certain (wrong value) | MEDIUM: wrong translator selected | 5 | Fix soon |
| 2.6 RunLedger costs | Certain (never recorded) | MEDIUM: zero cost visibility | 5 | Fix soon |
| 2.7 DemurrageConsumer | Certain (consumer dropped) | LOW: decay works, config ignored | 3 | Fix soon |
| 2.8 Runner v2 gaps | Certain (missing features) | MEDIUM: learning state not persisted | 6 | Fix soon |
| 3.1 Clippy suppression | Certain (always suppressed) | MEDIUM: bugs go undetected | 5 | Fix next |
| 3.2 Dead traits | Certain (zero callers) | LOW: dead code, blocks downstream | 3 | Decide |
| 3.3 God structs | Certain (77/126 fields) | LOW: velocity drag | 3 | Plan |
| 3.4 Config consolidation | Certain (8 entry points) | MEDIUM: inconsistent behavior | 5 | After 1.4 |
| 3.5 RetryPolicy dupe | Certain (two systems) | LOW: confusion, no runtime effect | 2 | Clean up |

---

## Success Metrics

### Tier 1 is "done" when:
- [ ] A plan run with 3+ tasks shows updated playbook scores in `.roko/learn/playbooks/` (1.1)
- [ ] `rg 'STATUS: NOT WIRED' crates/roko-learn/src/event_subscriber.rs` returns zero hits (1.2)
- [ ] `.roko/learn/cascade-router.json` confidence values change after a failed gate (1.3)
- [ ] `ROKO__AGENT__CONTEXT_LIMIT_K=32 roko config show` reflects the override (1.4)
- [ ] A plan run with `enable_advanced_rungs = true` produces non-stub rung 3 verdicts (1.5)
- [ ] `.roko/learn/section-outcomes.jsonl` entries have `prompt:` prefixed `section_id` values (1.6)

### Tier 2 is "done" when:
- [ ] `rg '\.exists\(\)' crates/roko-cli/src/runner/plan_loader.rs | wc -l` < 2 (2.1)
- [ ] `.unwrap()` count in event_loop.rs + runtime_feedback.rs + orchestrate.rs + dag.rs < 50 total (2.2)
- [ ] `cargo test -p roko-acp -- ready_field` passes with the new serde contract (2.3)
- [ ] Bare mode exposes exactly 8 commands (2.4)
- [ ] No Anthropic models have `tool_format = "openai_json"` in roko.toml (2.5)
- [ ] `roko plan run --json` output contains `task_costs` array (2.6)
- [ ] `rg '_consumer' crates/roko-serve/src/lib.rs` returns zero hits (2.7)
- [ ] After plan run, `cascade-router.json` and `gate-thresholds.json` have updated timestamps (2.8)

### Tier 3 is "done" when:
- [ ] `cargo clippy -p roko-cli --no-deps -- -D warnings` catches real issues (3.1)
- [ ] Observe/Connect/Trigger either have implementations or are deleted from exports (3.2)
- [ ] `PlanRunner` has <15 top-level fields (3.3)
- [ ] `rg 'ConfigLayer' crates/ --type rust` returns zero hits outside tests (3.4)
- [ ] Only one `RetryPolicy::execute()` exists in workspace (3.5)

### Tier 4 is "done" when:
- [ ] All spec-required tests from tasks 013, 019, 026, 044, 061, 064, 072 exist and pass (4.1)
- [ ] `cargo test -p roko-core --test cell_execute` compiles and passes (4.2)
- [ ] `rust-toolchain.toml` has a pinned version, not `"stable"` (4.6)

---

## Timeline Estimate

| Tier | Sessions | Calendar Days (1 dev) | With 3 Parallel Agents |
|------|----------|----------------------|----------------------|
| Tier 1 | 1-2 | 2-3 days | 1 day |
| Tier 2 | 1-2 | 2-3 days | 1 day |
| Tier 3 | 2-3 | 3-5 days | 2 days |
| Tier 4 | 1-2 | 1-2 days | 0.5 days |
| **Total** | **5-9** | **8-13 days** | **4-5 days** |

Tier 5 is deferred and not included in the timeline. Items unblock as their dependencies are resolved.

---

## Quick Wins (< 30 min each, do first in any sprint)

| Fix | Time | Item |
|-----|------|------|
| ACP `ready` serde attribute | 2 min | 2.3 |
| `tool_format` in roko.toml | 5 min | 2.5 |
| `cell_execute` test import | 5 min | 4.2 |
| `DEFAULT_RUNNER_RETRY_STRATEGY_PIVOT_ATTEMPT` wiring | 5 min | 4.3 |
| SIGTERM handler for `roko dev` | 10 min | 4.4 |
| Bare mode command whitelist | 15 min | 2.4 |

**Total quick wins**: ~42 minutes for 6 fixes. Do these before starting any sprint.

---

## Do NOT Fix (Intentional or Low-Value)

| Issue | Why leave it |
|-------|-------------|
| `run.rs` feature flag dead code (3,662 LOC) | Will be deleted when runner v2 replaces legacy orchestrate (MASTER-TASKS Phase D) |
| TuiState 126 fields | Needs design phase. Mechanical grouping is safe but the real fix (trait boundaries) needs architectural decisions. Defer to after PlanRunner extraction proves the pattern. |
| ContextBidder 3 definitions | Needs architectural decision: which definition wins? The runtime bidders are richer but live in floating modules. Resolve after event_subscriber (1.2) makes floating modules accessible. |
| `compat.rs` Mori migration | Actively modified (only dirty file in `git status`). May still be needed for users migrating from Mori. Ask before deleting. |
| Gate rungs 4-7 | Need real infrastructure that doesn't exist yet: chain backend (rung 4), LLM API budget (rung 6), integration test suite (rung 7). Focus on rung 3 first (1.5). |
| Env var data race in test (task 076) | Test-only. Use `#[tokio::test(flavor = "current_thread")]` if it becomes flaky. |
