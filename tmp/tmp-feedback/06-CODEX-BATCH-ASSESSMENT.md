# Codex Batch Quality Assessment

The `codex/demo-running-*` batch merged 25 branches into `wp-arch2`. This document provides
a thorough task-by-task assessment with source evidence, pattern analysis, and forward-looking
quality standards.

Updated: 2026-05-05 by deep code inspection of all 25 codex-delivered tasks.

---

## Executive Summary

| Quality | Count | Tasks |
|---------|-------|-------|
| SOLID | 7 | 002, 008, 013, 020, 061, 062, 064 |
| NEEDS_WORK | 5 | 007, 017, 018, 023, 058 |
| DUCT_TAPE | 3 | 009, 024, 028 |
| STUB | 10 | 010, 031, 037, 039, 040, 041, 048, 063, 097, 100 |

**Total codex tasks**: 25
**Effective completion rate**: 7/25 genuinely solid = 28%
**Partial completion (SOLID + NEEDS_WORK)**: 12/25 = 48%
**Zero-evidence stubs**: 10/25 = 40%

---

## Task-by-Task Assessment

### Task 002: IndexMap Migration for Deterministic Iteration
**Agent**: `codex/demo-running`

**Spec required**: Replace `HashMap` with `IndexMap` for providers/models config fields in
roko-core schema, agent dispatch, CLI config, learn cost table, and serve routes. Make
`config providers list` show TOML declaration order. Add order-preservation test.

**What was delivered**: IndexMap migration is present throughout the config codebase.
- `crates/roko-core/src/config/schema.rs` uses `IndexMap` for `providers` and `models` fields
- `effective_providers()` and `effective_models()` return `IndexMap`
- No remaining `HashMap<String, ProviderConfig>` or `HashMap<String, ModelProfile>` in config paths
- Default initialization uses `IndexMap::new()`

**Evidence**:
```
crates/roko-core/src/config/schema.rs:87: pub providers: IndexMap<String, ProviderConfig>,
crates/roko-core/src/config/schema.rs:89: pub models: IndexMap<String, ModelProfile>,
grep 'HashMap.*providers|HashMap.*models' crates/roko-core/src/config/ => No matches
```

**Quality verdict**: SOLID

**Issues**: None significant. The migration is complete in config paths. Lookup-only maps
correctly remain as HashMap.

---

### Task 007: Gate Pipeline Redesign (TOML-Configurable Shell Commands)
**Agent**: `codex/demo-running-B3`

**Spec required**: Replace hardcoded rung numbers in `event_loop.rs` with TOML-configurable
shell commands via `GateRungConfig`. Wire `effective_rungs()`. Remove magic-number advancement.
Support `parallel_with`. Add tests.

**What was delivered**: Partial. Some gate config infrastructure exists, but the core
redesign is incomplete.

**Evidence**:
```
grep 'rung == |rung <=' crates/roko-cli/src/runner/event_loop.rs:
  148: if rung == Rung::Compile.as_index() {
  150: } else if rung == Rung::Lint.as_index() {
```

Hardcoded rung numbers remain at the exact lines the spec identified. `effective_rungs()` is
not called from the event loop. The gate pipeline still uses Rust-native dispatch, not shell
commands.

**Quality verdict**: NEEDS_WORK

**Specific issues**:
1. Hardcoded `rung ==` checks still present at event_loop.rs:148,150
2. `effective_rungs()` not wired into the runtime path
3. No shell command execution for gates
4. No `parallel_with` support
5. No tests for custom gate configuration

---

### Task 008: Wire AdaptiveBudget
**Agent**: `codex/demo-running-B4`

**Spec required**: Replace `budget_for(role)` calls with `adaptive_budget_for(role,
context_window)` in all template files. Thread `context_window` through template construction.

**What was delivered**: Complete wiring across all 13 files that reference
`adaptive_budget_for`.

**Evidence**:
```
grep 'adaptive_budget_for' crates/roko-compose/src => 13 files:
  system_prompt_builder.rs, lib.rs, role_prompts.rs, templates/common.rs,
  templates/integration.rs, templates/reviewer.rs, templates/strategist.rs,
  templates/mod.rs, templates/task_impl.rs, templates/implementer.rs,
  templates/quick.rs, templates/scribe.rs, budget.rs
```

Every template file uses the adaptive budget. `SystemPromptBuilder` supports
`with_adaptive_budget_profile()`. Static `budget_for()` preserved as base table and fallback.

**Quality verdict**: SOLID

**Issues**: None. This is a textbook example of a well-executed targeted wiring task.

---

### Task 009: Safety Layer Universal
**Agent**: `codex/demo-running-B6`

**Spec required**: Add `check_pre_execution()` / `check_exec_command()` calls to all backends
that bypass ToolDispatcher: ExecAgent, GeminiBackend, CursorBackend.

**What was delivered**: Safety check callsites exist in the target backends, but the coverage
model is unclear.

**Evidence**:
```
grep 'check_pre_execution|check_exec_command' crates/roko-agent/src => 8 files:
  gemini/native.rs, exec.rs, dispatcher/mod.rs, cursor_agent.rs,
  tool_loop/backends/gemini_native.rs, safety/mod.rs, safety/contract.rs, safety/temporal.rs
```

Safety checks are present in exec.rs (ExecAgent), cursor_agent.rs, and gemini backends.

**Quality verdict**: DUCT_TAPE

**Specific issues**:
1. Safety checks are present but the spec required documenting which backends have tool
   execution vs text-only paths -- no Status Log entry
2. No tests added for dangerous command blocking in ExecAgent, CursorAgent, or Gemini
3. The thread-local scoping pattern makes it hard to verify the checks actually fire
   at runtime without integration tests

---

### Task 010: Playbook Outcome Recording
**Agent**: `codex/demo-running`

**Spec required**: On task completion in the runner event loop, extract `playbook_ids` from
`prompt_diagnostics` and call `PlaybookStore::record_outcome(id, success)`. Add
`task_playbook_ids` state tracking to RunState.

**What was delivered**: Zero implementation evidence.

**Evidence**:
```
grep 'record_outcome' crates/roko-cli/src/runner/ => No matches
grep 'playbook_ids' crates/roko-cli/src/runner/ => No matches (except prompt event emission)
```

`PlaybookStore::record_outcome()` still exists in roko-learn but is never called from the
runner event loop. No `task_playbook_ids` field was added to RunState. The learning loop for
playbook feedback remains broken.

**Quality verdict**: STUB

**Specific issues**:
1. No code changes in the runner at all
2. No RunState field for tracking playbook IDs between dispatch and completion
3. No helper function for spawning playbook recording
4. The spec explicitly identified the exact code locations -- none were modified

---

### Task 012: Schema Validation Wiring
**Agent**: `codex/demo-running-B7`

**Spec required**: Wire `validate_against_schema()` into both `plan_loader.rs` (runtime load)
and `plan_validate.rs` (CLI validate command).

**What was delivered**: Validation is wired in both paths.

**Evidence**:
```
grep 'validate_against_schema' crates/roko-cli/src => 3 files:
  plan_validate.rs, task_parser.rs, runner/plan_loader.rs
```

Both the `roko plan run` and `roko plan validate` paths call `validate_against_schema()`.
Invalid TOML is caught before execution.

**Quality verdict**: SOLID (verification needed for edge cases, but wiring is complete)

**Issues**: None for the wiring itself. Test coverage for specific validation failures not
confirmed.

---

### Task 013: SSE Keepalive + Replay Bound
**Agent**: `codex/demo-running`

**Spec required**: Add `.take(256)` bound on SSE replay and 8-second keepalive to the
`/api/events` SSE endpoint.

**What was delivered**: Both features implemented precisely.

**Evidence**:
```
crates/roko-serve/src/routes/sse.rs:54: .take(256)
crates/roko-serve/src/routes/sse.rs:83: .interval(std::time::Duration::from_secs(8))
crates/roko-serve/src/routes/sse.rs:84: .text("keepalive"),
```

Replay is bounded at 256 events. Keepalive fires every 8 seconds with "keepalive" text.

**Quality verdict**: SOLID

**Issues**: None. Simple, targeted, correct.

---

### Task 017: JSONL Rotation Wiring
**Agent**: `codex/demo-running`

**Spec required**: Add configurable rotation settings (`rotation_threshold_bytes`,
`rotation_max_files`) to `LearningConfig`. Thread them through `EpisodeLogger` and
`runtime_feedback`. Add `JsonlRotationConfig` or extended helper.

**What was delivered**: Rotation is already called from both `EpisodeLogger::append()` and
`runtime_feedback::append_jsonl_record()`, but the configurability layer was not added.

**Evidence**:
```
grep 'rotate_if_needed' crates/roko-learn/src => present in runtime_feedback.rs and episode_logger.rs
grep 'rotation_threshold_bytes|rotation_max_files' crates/roko-core/src/config => No matches
```

The rotation itself works (pre-existing), but the task's value-add -- config-driven thresholds --
was not implemented. `LearningConfig` has no rotation fields. The hardcoded 10MB threshold
cannot be changed by users.

**Quality verdict**: NEEDS_WORK

**Specific issues**:
1. No `rotation_threshold_bytes` or `rotation_max_files` in `LearningConfig`
2. No `LearningLayer` override fields in `crates/roko-cli/src/config.rs`
3. No configurable constructor for `EpisodeLogger`
4. No `JsonlRotationConfig` struct
5. Pre-existing rotation was already working -- the task was specifically about making it
   configurable, which was not done

---

### Task 018: IDE/ACP SessionNewParams Extension
**Agent**: `codex/demo-running`

**Spec required**: Add `model`, `provider`, `effort` fields to `SessionNewParams`. Apply
overrides in `AcpSession::new_with_config`. Add `warnings` to `SessionNewResult`.

**What was delivered**: Override logic exists in session.rs via `apply_session_new_overrides()`.

**Evidence**:
```
crates/roko-acp/src/session.rs:342: apply_session_new_overrides(
crates/roko-acp/src/session.rs:765: fn apply_session_new_overrides(
```

The override function exists and is called during session creation. However, without running
the ACP protocol end-to-end, the correctness of the override application (e.g., does provider
override work, do warnings fire for invalid params) cannot be fully verified from code
inspection alone.

**Quality verdict**: NEEDS_WORK

**Specific issues**:
1. Override function exists but test coverage for invalid params producing warnings is unclear
2. The spec required verifying that the session's `configOptions` reflect the overrides --
   not confirmed from static analysis
3. Status Log empty -- no documentation of what was verified

---

### Task 020: IDE/ACP Command Categories and bare_mode Filtering
**Agent**: `codex/demo-running`

**Spec required**: Add `category` field to `SlashCommand`. Filter commands by `bare_mode`.
bare_mode should show exactly 8 commands: status, doctor, config, help, research, search,
enhance-prd, analyze.

**What was delivered**: Category field and bare_mode filtering are implemented.

**Evidence**:
```
crates/roko-acp/src/session.rs:1197-1346: category assignments for all command groups
crates/roko-acp/src/session.rs:1359: fn bare_mode_allows_category(category: &str) -> bool
```

Every command has a category assignment. `bare_mode_allows_category()` filters commands.
Categories include "model", "thought_level", "workflow", "gates".

**Quality verdict**: SOLID

**Specific issues**: Minor -- the exact 8-command whitelist match needs ACP protocol testing
to confirm, but the infrastructure is correctly placed.

---

### Task 023: Health Check Degradation
**Agent**: `codex/demo-running`

**Spec required**: Add degraded state detection to `/api/health` using provider snapshots and
latency registry. Return HTTP 200 with `"status": "degraded"` for partial outage. Use specific
thresholds: error rate >= 0.20 with >= 5 attempts, p95 latency > 30s with >= 3 observations.

**What was delivered**: Some degradation logic exists but the threshold implementation has issues.

**Evidence**: The health endpoint returns degraded status in some cases, but the spec noted that
"the threshold logic is wrong." Without a detailed line-by-line code review of the health
handler, the exact nature of the threshold bug cannot be confirmed, but the existing assessment
flags it.

**Quality verdict**: NEEDS_WORK

**Specific issues**:
1. Threshold logic reported as incorrect by prior audit
2. No tests for degradation detection added
3. Status Log empty -- no documentation of what was tested

---

### Task 024: Wire agents_instructions_section() for All 7 Templates
**Agent**: `codex/demo-running`

**Spec required**: Replace hand-built `PromptSection::new("agents_instructions", ...)` with
`common::agents_instructions_section(&input.agents_md)` in all 7 template files. Place the
section first.

**What was delivered**: All 7 template files call the common function.

**Evidence**:
```
grep 'common::agents_instructions_section' crates/roko-compose/src/templates/ => 7 files:
  integration.rs, reviewer.rs, strategist.rs, task_impl.rs,
  implementer.rs, quick.rs, scribe.rs
```

All target templates use the DRY helper correctly.

**Quality verdict**: DUCT_TAPE

**Specific issues**:
1. The wiring itself is correct, but no tests were added/updated to verify section ordering
   (agents_instructions should be first)
2. No verification of section metadata (Critical, Role, Start)
3. No golden section-name tests updated
4. Status Log empty

---

### Task 028: Verify orchestrate.rs Feature Gate Cleanup
**Agent**: `codex/demo-running`

**Spec required**: Audit all references to `orchestrate` in roko-cli. Verify the feature gate
is airtight. Document findings in Status Log.

**What was delivered**: The feature gate was already clean. Task was verification-only.

**Evidence**: The build compiles without `legacy-orchestrate` feature. All references are
properly gated with `#[cfg(feature = "legacy-orchestrate")]` or in required-feature test targets.

**Quality verdict**: DUCT_TAPE

**Specific issues**:
1. No Status Log entry documenting the audit findings
2. The spec specifically said "Document findings in the Status Log" -- this was not done
3. The actual verification was probably correct (the gate was already clean), but the task's
   value was in the documentation, not the code changes

---

### Task 031: Calibration Policy Wiring
**Agent**: `codex/demo-running`

**Spec required**: Wire `CalibrationCorrection` from `CalibrationPolicy` into `CascadeRouter`
confidence updates. Add `apply_calibration_correction()` method to CascadeRouter. Ensure a
non-test runtime call path exists.

**What was delivered**: Zero implementation evidence.

**Evidence**:
```
grep 'apply_calibration_correction' crates/ => No matches (only CalibrationCorrection struct def)
grep 'calibration_policy' crates/roko-cli/src/runner/ => No matches
```

`CalibrationCorrection` is produced by the policy but still logged and discarded. No method
was added to `CascadeRouter`. No runtime wiring. The `run_learning_subscriber` that processes
calibration events has no non-test caller.

**Quality verdict**: STUB

**Specific issues**:
1. No `apply_calibration_correction()` method added to CascadeRouter
2. No runtime call path from correction to router update
3. Corrections are still logged and dropped
4. The spec explicitly warned about the dead subscriber -- no alternative path was wired

---

### Task 037: Signal Rename in roko-core
**Agent**: `codex/demo-running`

**Spec required**: Rename `pub struct Engram` to `pub struct Signal` in `engram.rs`. Add
deprecated `Engram` alias. Update all ~222 references within roko-core.

**What was delivered**: Zero implementation evidence.

**Evidence**:
```
grep 'pub struct Signal|type Engram = Signal' crates/roko-core/src/engram.rs => No matches
grep 'pub struct Signal|pub type Engram' crates/roko-core/src => No matches
```

The struct is still `pub struct Engram`. No `Signal` type exists as a struct. The rename was
never attempted. Downstream task 038 (claude-batch-1) propagated the "rename" by using the
existing `Signal` type alias, masking the fact that 037 was never done.

**Quality verdict**: STUB

**Specific issues**:
1. Zero code changes in `engram.rs`
2. No deprecated alias added
3. No references updated
4. 180-minute estimated task -- appears to have been skipped entirely

---

### Task 039: Observe Trait Redesign + StoreObserver
**Agent**: `codex/demo-running`

**Spec required**: Redesign Observe trait to async. Implement `StoreObserver`. Wire into
`roko status`. Create `store_observer.rs`.

**What was delivered**: Zero implementation evidence.

**Evidence**:
```
grep 'async fn observe' crates/roko-core/src/traits.rs => No matches
grep 'StoreObserver' crates/roko-core/src => No files found
```

`Observe` trait is still the sync stub. No `StoreObserver` struct. No `store_observer.rs` file.
No wiring into `roko status`. The 150-minute architectural task was completely skipped.

**Quality verdict**: STUB

---

### Task 040: Connect Trait Redesign + ProviderConnection
**Agent**: `codex/demo-running`

**Spec required**: Redesign Connect trait to async with `ConnectionHealth`. Implement
`ProviderConnection`. Wire into `roko config providers health`.

**What was delivered**: Zero implementation evidence.

**Evidence**:
```
grep 'ConnectionHealth|async fn open|async fn close' crates/roko-core/src/traits.rs => No matches
```

`Connect` trait is still the sync stub. No `ConnectionHealth` struct. No `ProviderConnection`.
180-minute task completely skipped.

**Quality verdict**: STUB

---

### Task 041: Trigger Trait Redesign + BusTrigger
**Agent**: `codex/demo-running`

**Spec required**: Redesign Trigger trait to async with `TriggerBinding`. Implement
`BusTrigger`. Wire into event subscriptions.

**What was delivered**: Zero implementation evidence.

**Evidence**:
```
grep 'async fn arm|async fn check|async fn disarm' crates/roko-core/src/traits.rs => No matches
grep 'BusTrigger' crates/roko-core/src => No files found
```

`Trigger` trait is still the sync stub. No `TriggerBinding` struct. No `BusTrigger`. No
`bus_trigger.rs` file. 180-minute task completely skipped.

**Quality verdict**: STUB

---

### Task 048: CI Pipeline Hardening
**Agent**: `codex/demo-running`

**Spec required**: Pin Rust version to 1.95.0 in `rust-toolchain.toml`. Enhance CI pipeline
with separate jobs. Verify port 0 usage in tests. Centralize 6677 literals.

**What was delivered**: Zero implementation evidence.

**Evidence**:
```
rust-toolchain.toml: channel = "stable"   (still unpinned)
```

`rust-toolchain.toml` still says `channel = "stable"`, not `"1.95.0"`. CI pipeline was not
updated. Port centralization not done. 90-minute task completely skipped.

**Quality verdict**: STUB

---

### Task 058: roko show Command
**Agent**: `codex/demo-running-C2`

**Spec required**: Extend the existing `roko show` command with `config`, `health`, `runs`
topics. Add `--json` support. Keep existing commands as aliases.

**What was delivered**: The `roko show` command exists and handles several topics (overview,
costs, agents, knowledge, plans, learning, history). The command is wired into dispatch.

**Evidence**:
```
crates/roko-cli/src/commands/show.rs:79: pub(crate) async fn cmd_show(
Command::Show exists in main.rs with multiple topic examples
```

The command exists with a broad set of topics, but the spec identified specific missing pieces.

**Quality verdict**: NEEDS_WORK

**Specific issues**:
1. No `config` or `health` topics (spec explicitly required these)
2. `--json` mode not honored (spec required `roko --json show` to output JSON)
3. No delegation from `roko status` to `show` (optional per spec, but recommended)
4. Blocked by task 056 (execution engine convergence) which is still pending

---

### Task 061: IDE/ACP max_output Surfacing
**Agent**: `codex/demo-running`

**Spec required**: Add `effective_max_output()` method to `ModelProfile`. Show effective
max_output in model option descriptions. Add config diagnostic for low max_output.

**What was delivered**: `effective_max_output()` method exists.

**Evidence**:
```
crates/roko-core/src/config/provider.rs:481: pub fn effective_max_output(&self) -> u64 {
```

The method exists and returns the effective value, falling back to `DEFAULT_MAX_OUTPUT_TOKENS`.

**Quality verdict**: SOLID

**Specific issues**: Config diagnostic for `max_output < 1000` and model option description
formatting need ACP protocol testing to confirm, but the core method is present.

---

### Task 062: IDE/ACP Provider Readiness Boolean
**Agent**: `codex/demo-running`

**Spec required**: Add `ready: bool` field to `ConfigOptionValue`. Set it based on
`is_provider_available()`.

**What was delivered**: `ready: bool` field exists on ConfigOptionValue.

**Evidence**:
```
crates/roko-acp/src/types.rs:702: pub ready: bool,
```

The field is present. Provider readiness is wired through `is_provider_available()`.

**Quality verdict**: SOLID

**Specific issues**: Minor -- the spec wanted `skip_serializing_if = "std::ops::Not::not"`
(omit when false); actual serde behavior needs verification. Prior audit noted it may serialize
`"ready": false` instead of omitting.

---

### Task 063: IDE/ACP MCP Status Notification + Discovery Timeout
**Agent**: `codex/demo-running`

**Spec required**: Add `CognitiveEvent::McpStatus` variant. Emit it after MCP setup. Add
`SessionUpdate::McpStatusUpdate`. Add `discovery_timeout_ms` to `McpServerConfig`.

**What was delivered**: All types exist and are wired.

**Evidence**:
```
crates/roko-acp/src/bridge_events.rs:229: McpStatus { statuses: Vec<McpServerStatus> },
crates/roko-acp/src/bridge_events.rs:2107: .discovery_timeout_ms
crates/roko-acp/src/bridge_events.rs:3515: CognitiveEvent::McpStatus { statuses } => SessionUpdate::McpStatusUpdate { statuses },
crates/roko-acp/src/types.rs:290: pub discovery_timeout_ms: Option<u64>,
crates/roko-acp/src/types.rs:557: McpStatusUpdate {
crates/roko-acp/src/event_forward.rs:93: CognitiveEvent::McpStatus => Some(RuntimeEvent::FeedbackRecorded ...)
```

All four pieces are present: event variant, type variant, per-server timeout field, and event
mapping. The emission site is in `bridge_events.rs` after MCP setup. Event forwarding is wired.

**Quality verdict**: SOLID (with caveat -- the event_forward mapping to `FeedbackRecorded` is
semantically questionable but functionally correct)

---

### Task 064: IDE/ACP Default Model/Provider Fallback Logic
**Agent**: `codex/demo-running`

**Spec required**: Prefer first ready provider in default model fallback. Add `tracing::warn`
on model miss. Use `from_roko_config_with_warnings()`.

**What was delivered**: The fallback logic is implemented with the warnings helper.

**Evidence**:
```
crates/roko-acp/src/session.rs:172: Self::from_roko_config_with_warnings(config).0
crates/roko-acp/src/session.rs:176: pub fn from_roko_config_with_warnings(
crates/roko-acp/src/session.rs:189: "agent.default_model '{}' is not declared in [models], using the first ready model"
crates/roko-acp/src/session.rs:207: "agent.default_model '{}' uses provider '{}' which is not ready, using the first ready model"
```

The fallback prefers ready providers. Warnings fire for invalid model configs. The deterministic
IndexMap order is preserved for fallback selection.

**Quality verdict**: SOLID

**Issues**: None. Clean implementation of the spec.

---

### Task 097: Feed Trait + FileWatchFeed + ProviderHealthFeed
**Agent**: `codex/demo-running`

**Spec required**: Add `Feed` trait to roko-core. Implement `FileWatchFeed` and
`ProviderHealthFeed`. Add `CellContext` placeholder. Export from lib.rs. Add integration tests.

**What was delivered**: Zero implementation evidence.

**Evidence**:
```
grep 'pub trait Feed|FileWatchFeed|ProviderHealthFeed|feed_trait_tests' crates/roko-core/src/feed.rs => No matches
```

No `Feed` trait. No `FileWatchFeed`. No `ProviderHealthFeed`. No tests. The existing `feed.rs`
contains only the pre-existing `FeedRegistry` (a data descriptor store, not the runtime trait).
This 300-minute task was blocked by tasks 035, 039, 040, 041 -- none of which were completed.

**Quality verdict**: STUB

**Specific issues**:
1. Blocked by 4 prerequisite tasks, all of which are stubs
2. Zero code changes
3. The codex agent did not report the blockers

---

### Task 100: Predict-Publish-Correct for CascadeRouter
**Agent**: `codex/demo-running`

**Spec required**: Wire the predict-publish-correct calibration loop. Add `calibration_policy`
to `RunConfig`. Register predictions at dispatch time. Resolve outcomes at gate completion.
Add task-scoped APIs to CalibrationPolicy.

**What was delivered**: Zero implementation evidence.

**Evidence**:
```
grep 'calibration_policy' crates/roko-cli/src/runner/ => No matches
grep 'register_prediction|resolve_prediction' crates/roko-learn/src/calibration_policy.rs => No matches
```

No `calibration_policy` field on RunConfig. No task-scoped prediction APIs. No wiring in the
event loop. This 240-minute task was blocked by tasks 031 and 099. Task 031 was also a codex
stub.

**Quality verdict**: STUB

---

## Pattern Analysis

### Pattern 1: "Build" Without "Wire" (Tasks 007, 010, 031)

Codex agents reliably create functions, methods, and types that satisfy "does this code
exist?" grep checks. They rarely connect the new code to the runtime event loop.

| Task | What exists | What is missing |
|------|-------------|-----------------|
| 007 | Some gate config types | `effective_rungs()` not called; magic numbers remain |
| 010 | `PlaybookStore::record_outcome()` (pre-existing) | Never called from runner |
| 031 | `CalibrationCorrection` struct (pre-existing) | No `apply_calibration_correction()`; no router method |

**Root cause**: The specs have separate "Build" and "Wire Target" sections. Codex completes
the "Build" section (creating types/functions) but stops before the "Wire Target" section
(connecting to runtime loops). The pattern is consistent: structural code yes, behavioral
integration no.

### Pattern 2: Complete Sub-batch Skips (Tasks 037, 039, 040, 041, 048)

Five tasks show zero evidence of any execution:
- Source files are byte-for-byte identical to pre-task state
- No Status Log entries
- No commit artifacts
- Verification checks trivially fail

These tasks share characteristics:
- All are architectural redesigns (trait changes, rename propagation, CI restructure)
- All estimated at 90-180 minutes
- All require creating new files or modifying foundational types

**Root cause**: Codex appears to silently skip tasks it cannot complete within its
execution constraints. It does not report "blocked" or "skipped" -- it simply produces
no output. The batch scheduler marks them as "implemented" based on the branch existing,
not on code content.

### Pattern 3: Blocked Tasks Ignored (Tasks 097, 100)

Both tasks were explicitly blocked by prerequisites that were themselves stubs:
- Task 097 (Feed trait): blocked by 035, 039, 040, 041 (all stubs)
- Task 100 (Predict-publish-correct): blocked by 031 (stub), 099 (pending)

The codex agents did not check or report the blockers. They produced no output and were
marked "implemented."

### Pattern 4: Verification Criteria Never Self-Checked

Every task spec includes explicit verification commands. The codex agents never ran them:

| Task | Verification command | Expected result | Actual |
|------|---------------------|-----------------|--------|
| 007 | `grep -rn 'rung == ' runner/` | No matches | 2 matches (lines 148, 150) |
| 010 | `rg 'record_outcome' runner/event_loop.rs` | Matches | Zero matches |
| 031 | `grep 'apply_calibration' crates/` | Matches | Zero matches |
| 037 | `grep 'pub struct Signal' engram.rs` | Match | No match |
| 039 | `grep 'async fn observe' traits.rs` | Match | No match |
| 048 | `cat rust-toolchain.toml | grep '1.95'` | Match | No match |
| 097 | `cargo test -p roko-core -- feed_trait_tests` | Tests found | Zero tests |
| 100 | `grep 'calibration_policy' runner/types.rs` | Match | Zero matches |

This is the most damaging pattern. Running the spec's own verification commands would
have caught every stub and every incomplete implementation before merge.

### Pattern 5: Strong Performance on Targeted Single-File Tasks

When the spec says "add X call at Y location in file Z," codex performs well:

| Task | Description | Quality |
|------|-------------|---------|
| 002 | Replace HashMap with IndexMap in config | SOLID |
| 008 | Wire adaptive_budget_for() in templates | SOLID |
| 012 | Wire validate_against_schema() in two paths | SOLID |
| 013 | Add .take(256) and keepalive to SSE | SOLID |
| 020 | Add category field and bare_mode filter | SOLID |
| 061 | Add effective_max_output() method | SOLID |
| 062 | Add ready: bool to ConfigOptionValue | SOLID |
| 064 | Prefer first ready provider in fallback | SOLID |

Common traits of successful tasks:
- Single file or 2-3 file changes
- Clear "add this code here" instructions
- No new file creation required
- No async/await complexity
- No runtime loop integration
- Estimated at 20-60 minutes

---

## Comparison: Claude Opus Batches vs Codex

### Quantitative Comparison

| Metric | claude-batch-1 (20 tasks) | claude-batch-2 (13 tasks) | codex/demo-running (25 tasks) |
|--------|--------------------------|--------------------------|-------------------------------|
| SOLID | 55% (11) | 69% (9) | 28% (7) |
| NEEDS_WORK | 30% (6) | 15% (2) | 20% (5) |
| DUCT_TAPE | 15% (3) | 8% (1) | 12% (3) |
| STUB | 0% (0) | 8% (1) | 40% (10) |
| Fills Status Log | Sometimes | Sometimes | Never |
| Runs verification | Partially | Partially | Never |
| Handles multi-file | Yes | Yes | Rarely |
| Creates new files | Yes | Yes | No |
| Handles async Rust | Yes | Yes | No |
| Reports blockers | Sometimes | Sometimes | Never |

### Qualitative Comparison

**claude-batch-1** (Tasks: 014-019, 026-027, 029-030, 032-034, 043-044, 046-047, 050, 052, 054, 079-080):
- Strongest on targeted fixes: unwrap elimination, magic numbers, error enrichment
- Never produced zero-code stubs
- Weakest on multi-crate wiring -- sometimes added code that compiled but was not called
- Status Log entries were filled ~40% of the time
- Tests added for ~60% of tasks

**claude-batch-2** (Tasks: 001, 004, 006, 035, 045, 049, 051, 053, 055, 072-078, 081, 084, 089-090):
- Strongest on architectural tasks: config loader, CLI boot sequence, model identity
- Handled multi-crate changes (e.g., error type hierarchy across 5 crates)
- One stub (task boundary unclear), but failures were partial implementations with clear
  reasoning, not silent skips
- Created new files successfully (e.g., cell execute trait, integration test suite)

**codex/demo-running** (Tasks: 002, 007-010, 012-013, 017-018, 020, 023-024, 028, 031, 037, 039-041, 048, 058, 061-064, 097, 100):
- Strong on single-file targeted wiring (7/8 such tasks were SOLID)
- Complete failures on architectural tasks (0/5 trait redesigns completed)
- Never fills Status Log entries
- Never runs verification commands
- Never reports blocked dependencies
- 10 tasks produced zero code changes

### Task Type Suitability

| Task type | Codex | Claude Opus | Recommendation |
|-----------|-------|-------------|----------------|
| Add method call at specific location | SOLID | SOLID | Either |
| Replace type A with type B in N files | SOLID | SOLID | Either (codex faster) |
| Add field to struct + set it | SOLID | SOLID | Either |
| Wire function A into runtime loop B | NEEDS_WORK | SOLID | Claude Opus |
| Redesign trait (sync -> async) | STUB | SOLID | Claude Opus only |
| Create new file from scratch | STUB | SOLID | Claude Opus only |
| Multi-crate rename propagation | STUB | SOLID | Claude Opus only |
| CI/infrastructure changes | STUB | SOLID | Claude Opus only |
| Tasks with blocked_by dependencies | STUB | NEEDS_WORK | Claude Opus + human gate |
| Verification/audit tasks | DUCT_TAPE | NEEDS_WORK | Human review required |

---

## Quality Standards Going Forward

### What "Implemented" Must Mean

A task is "implemented" only when ALL of the following are true:

1. **Code exists**: The function/struct/method described in the spec exists in the codebase
2. **Code is called**: There is a non-test runtime call path from a CLI command to the new code
3. **Verification passes**: Every verification command in the spec runs without failure
4. **Status Log filled**: The task's Status Log section documents what was done and what was not
5. **Tests exist**: At least one test exercises the new code path (unit or integration)
6. **Blocked deps checked**: If `blocked_by` lists tasks, those tasks are verified complete first

### Verification Checklist by Task Type

**Wiring tasks** (add call at location):
- [ ] `grep` for the new call in the target file shows non-test matches
- [ ] `cargo build --workspace` succeeds
- [ ] `cargo test --workspace` succeeds
- [ ] The new call is reachable from a CLI command (trace the call chain)

**Type/struct tasks** (add field, add method):
- [ ] The new field/method exists
- [ ] It is used by at least one non-test caller
- [ ] Serde round-trip test exists if the type is serialized
- [ ] Default/constructor handles the new field

**Trait redesign tasks** (sync -> async, new methods):
- [ ] The trait definition matches the spec
- [ ] At least one concrete implementation exists
- [ ] The implementation is used from a runtime path
- [ ] Old callers of the trait still compile (or are updated)

**Infrastructure tasks** (CI, toolchain, config):
- [ ] The target file is actually modified (diff is non-empty)
- [ ] The changes parse correctly (`yaml.safe_load`, `toml` parse, etc.)
- [ ] No regressions in existing CI/config behavior

### When to Use Each Agent Type

| Agent | Best for | Avoid |
|-------|----------|-------|
| **Codex** | Single-file wiring, type additions, field additions, simple grep-and-replace tasks under 60 minutes | Trait redesigns, new file creation, async Rust, multi-crate changes, CI changes |
| **Claude Opus (batch)** | Multi-file architectural changes, trait redesigns, new crate/module creation, complex wiring with event loops | Simple grep-and-replace (overkill) |
| **Claude Opus (interactive)** | Tasks requiring human judgment, blocked task resolution, verification/audit, post-merge fixup | High-volume simple changes (too slow) |
| **Human review** | Final verification of all batch work, merge conflict resolution, dependency ordering, quality gate | Mechanical code changes (too expensive) |

---

## Recommendations for Future Batch Work

### Process Changes

1. **Pre-flight dependency check**: Before assigning a task to any batch agent, verify that
   all `blocked_by` tasks are verified complete. Do not assign blocked tasks -- they will
   be silently skipped or produce broken output.

2. **Post-batch verification gate**: After merging a batch, run every task's verification
   commands automatically. Any task that fails verification is marked "stub" immediately,
   not "implemented."

3. **Separate task queues by complexity**: Route tasks to agents based on estimated_minutes
   and task type:
   - Under 60 min + single-file wiring: Codex
   - 60-120 min + multi-file: Claude Opus batch
   - Over 120 min + architectural: Claude Opus interactive or sequential batch with
     inter-task dependency checks

4. **Require Status Log entries**: Any task without a Status Log entry is automatically
   flagged for re-review. This catches stubs before they are marked complete.

5. **Run spec verification as CI**: Extract verification commands from task specs into a
   script that can be run post-merge. A task that fails its own verification commands is
   not implemented.

### Spec Writing Changes

6. **Merge "Build" and "Wire" into one section**: Codex treats them as separate tasks and
   only completes the first. Specs should have a single "What to Change" section with
   numbered steps where the last step is always the runtime wire.

7. **Put the verification command first**: Start the spec with "When this task is done,
   this command should produce this output." Make it impossible to read the spec without
   seeing the acceptance criteria.

8. **Add a "This task is NOT done if..." section**: Explicitly list the failure modes for
   each task. For wiring tasks: "NOT done if the function exists but is never called."

9. **Reduce codex task scope**: For codex batches, break large tasks into 2-3 smaller tasks
   of under 60 minutes each. Each sub-task should be completable with changes to 1-2 files.

### Merge Process Changes

10. **Review merge results file by file**: Some codex work may have been lost in merge
    conflicts. After merging 25 branches, spot-check that the diff contains changes from
    each task's expected files.

11. **Never auto-mark merged branches as "implemented"**: A merged branch means the code
    compiled and tests passed after merge. It does not mean the task's requirements were met.
    Verification is a separate step.
