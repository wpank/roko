# 12 - Code Debt Elimination Plan

> 37 tasks targeting dead code, duplication, god objects, hardcoded strings,
> type duplication, stale feature gates, suppressed lints, config inconsistencies,
> and test coverage gaps. Every task includes exact file paths, acceptance criteria,
> and dependency ordering.
>
> Evidence sourced from: `11-CURRENT-STATE-GROUND-TRUTH.md`, `05-CURRENT-STATE-AND-GAPS.md`,
> `01-LESSONS-AND-APPROACHES.md`, `03-PROVIDER-AND-AGENT-AUDIT.md`,
> `04-ORCHESTRATION-AND-GATES-AUDIT.md`, and direct source inspection on `wp-arch2`.

---

## Phase 1: Suppress Nothing -- Remove Lint Blankets (3 tasks)

These blanket `allow` attributes hide real problems. Removing them surfaces dead
code and unused imports that the compiler already knows about.

### Task 1.1: Remove blanket `#![allow(dead_code, unused_imports, unused_variables)]` from roko-cli

**File**: `crates/roko-cli/src/lib.rs` line 6
**What**: The crate-level `#![allow(dead_code, unused_imports, unused_variables)]` suppresses
all unused-code warnings for the entire CLI crate -- the largest crate in the workspace.
This hides hundreds of real dead-code instances.

**Steps**:
1. Remove line 6: `#![allow(dead_code, unused_imports, unused_variables)]`
2. Run `cargo clippy -p roko-cli --no-deps -- -D warnings 2>&1 | head -200`
3. Catalog every warning into three categories:
   - Dead code that can be deleted immediately (unused private functions/structs)
   - Dead code behind `#[cfg(feature = "legacy-orchestrate")]` (leave for Phase 2)
   - Unused imports that should be removed
4. Fix all unused-import warnings by removing the imports
5. For dead private functions/structs not behind feature gates, delete them
6. For items that are pub but never used externally, downgrade to `pub(crate)` or delete

**Acceptance criteria**:
- `cargo clippy -p roko-cli --no-deps -- -D warnings` passes clean
- No blanket `#![allow(dead_code, ...)]` remains in `lib.rs`
- Line 6 is gone or replaced with targeted `#[allow(...)]` on specific items with justification

### Task 1.2: Remove blanket clippy suppression from roko-cli

**File**: `crates/roko-cli/src/lib.rs` lines 14-23
**What**: The `#![cfg_attr(clippy, allow(clippy::all, clippy::pedantic, clippy::nursery, ...))]`
block suppresses ALL clippy lints for the entire crate. This is the nuclear option and hides
real quality issues.

**Steps**:
1. Remove the entire `#![cfg_attr(clippy, allow(...))]` block (lines 14-23)
2. Run `cargo clippy -p roko-cli --no-deps 2>&1 | wc -l` to gauge scope
3. If > 500 warnings, add targeted per-module `#![allow(clippy::module_name_repetitions)]`
   only for the specific lints that are genuinely noisy (not quality-relevant)
4. Fix all clippy::correctness and clippy::suspicious warnings (these are bugs)
5. For remaining clippy::pedantic/nursery, add `#[allow(...)]` at the item level with
   a comment explaining why the lint is wrong for that specific case

**Acceptance criteria**:
- No crate-level clippy blanket suppression remains
- `cargo clippy -p roko-cli --no-deps` produces zero warnings from `clippy::correctness`
  and `clippy::suspicious` categories
- Any remaining `#[allow(clippy::...)]` annotations are item-level with justification

**Dependencies**: Task 1.1 (dead code removal reduces clippy surface)

### Task 1.3: Audit all crates for blanket lint suppression

**Files**: All `crates/*/src/lib.rs` files
**What**: Scan every crate's lib.rs for `#![allow(dead_code)]` or `#![allow(clippy::all)]`.
Repeat the Task 1.1/1.2 pattern for any that have them.

**Steps**:
1. `grep -rn '#!\[allow(dead_code' crates/*/src/lib.rs`
2. `grep -rn '#!\[allow(clippy::all' crates/*/src/lib.rs`
3. For each match, evaluate whether the blanket is justified (e.g., generated code)
4. Remove unjustified blankets, fix resulting warnings

**Acceptance criteria**:
- No crate in the workspace has blanket `#![allow(dead_code)]` or `#![allow(clippy::all)]`
  without an explicit justification comment
- `cargo clippy --workspace --no-deps -- -D warnings` passes clean

**Dependencies**: Tasks 1.1, 1.2

---

## Phase 2: Kill the God Object -- orchestrate.rs Decomposition (8 tasks)

orchestrate.rs is 22,522 lines with an 80+ field `PlanRunner` struct. It is gated
behind `#[cfg(feature = "legacy-orchestrate")]` and has zero live callers. The
valuable code inside it needs to be extracted into proper modules before the file
can be deleted. Every dogfood fix touched this file.

### Task 2.1: Extract gate pipeline functions into roko-gate

**Source**: `crates/roko-cli/src/orchestrate.rs` (functions around `run_gate_pipeline`,
`run_rung`, `enrich_rung_config`, gate dispatch logic)
**Target**: `crates/roko-gate/src/gate_executor.rs` (new file)

**Steps**:
1. Identify all gate-related functions in orchestrate.rs:
   - `run_gate_pipeline()`, `run_rung()` dispatch, rung-specific handlers
   - `enrich_rung_config()` (oracle enrichment per rung)
   - Gate artifact persistence
2. Extract into `crates/roko-gate/src/gate_executor.rs` as `GateExecutor` struct
3. `GateExecutor` takes `GateService`, `AdaptiveThresholds`, `ArtifactStore` as deps
4. Expose `pub async fn run_pipeline(&self, payload: GatePayload) -> Vec<GateVerdict>`
5. Wire `GateExecutor` into runner v2's `gate_dispatch.rs` as the gate execution path
6. Wire into `WorkflowEngine`'s `EffectDriver` as the `GateRunner` impl

**Acceptance criteria**:
- `GateExecutor` in roko-gate compiles independently
- Runner v2 uses `GateExecutor` for gate dispatch
- orchestrate.rs gate functions are no longer needed by any live path
- `cargo test -p roko-gate` passes

### Task 2.2: Extract enrichment pipeline into roko-compose

**Source**: `crates/roko-cli/src/orchestrate.rs` (`EnrichmentPipeline::run()`,
code context extraction, anti-pattern queries, strategy fragments)
**Target**: `crates/roko-compose/src/enrichment/` (already partially exists)

**Steps**:
1. The enrichment module at `crates/roko-compose/src/enrichment/` already has the
   types (`EnrichStep`, `StepSelector`, `PlanInfo`, etc.)
2. Identify orchestrate.rs code that calls the enrichment pipeline
3. Extract the orchestrate.rs-specific wiring (knowledge queries, pheromone signals,
   prior output injection) into `PromptAssemblyService` extensions
4. Add an `enrich()` method to `PromptAssemblyService` that runs the enrichment steps
5. Wire into runner v2's dispatch path and WorkflowEngine's `EffectDriver`

**Acceptance criteria**:
- `PromptAssemblyService::enrich()` is callable from both runner v2 and WorkflowEngine
- Code context extraction runs on the live dispatch path
- No enrichment logic remains only in orchestrate.rs

**Dependencies**: None (enrichment module already exists)

### Task 2.3: Extract worktree management into roko-orchestrator

**Source**: `crates/roko-cli/src/orchestrate.rs` (worktree creation, branch naming,
post-merge, health checks)
**Target**: `crates/roko-orchestrator/src/worktree.rs` (already partially exists)

**Steps**:
1. `WorktreeManager` already exists at `crates/roko-orchestrator/src/worktree.rs`
   with `format_branch_name()` and `WorktreeHealth`
2. Identify orchestrate.rs worktree code not already in the manager:
   - Worktree cleanup on task failure
   - Branch protection during merge
   - Post-merge validation
3. Move missing functionality into `WorktreeManager`
4. Verify runner v2's `merge.rs` uses `WorktreeManager` for all operations

**Acceptance criteria**:
- All worktree operations go through `WorktreeManager`
- No raw git worktree commands remain in orchestrate.rs that are needed by live paths
- `cargo test -p roko-orchestrator` passes

### Task 2.4: Extract learning persistence into roko-learn

**Source**: `crates/roko-cli/src/orchestrate.rs` (`record_task_success()`,
`record_task_failure()`, CascadeRouter/threshold/playbook/familiarity/section-effectiveness
persistence)
**Target**: `crates/roko-learn/src/runtime_feedback.rs` (already exists as `LearningRuntime`)

**Steps**:
1. `LearningRuntime` at `crates/roko-learn/src/runtime_feedback.rs` already has the
   types and `LearningPaths`
2. Compare `LearningRuntime`'s methods with orchestrate.rs's `record_task_success()`
   and `record_task_failure()` to identify missing recording calls
3. Add missing methods to `LearningRuntime`:
   - `record_playbook_from_tool_calls()`
   - `update_crate_familiarity()`
   - `update_section_effectiveness()`
   - `compute_cfactor()`
4. Ensure `CompletedRunInput` captures all data orchestrate.rs records

**Acceptance criteria**:
- `LearningRuntime` has methods for every learning persistence call in orchestrate.rs
- Every learning artifact written by orchestrate.rs can be written by `LearningRuntime`
- `cargo test -p roko-learn` passes

### Task 2.5: Extract conductor/anomaly detection into roko-conductor

**Source**: `crates/roko-cli/src/orchestrate.rs` (Conductor, StuckDetector, HealthMonitor,
AnomalyDetector, DiagnosisEngine wiring)
**Target**: `crates/roko-conductor/` (crate already exists)

**Steps**:
1. roko-conductor already has `Conductor`, `StuckDetector`, `HealthMonitor`, `DiagnosisEngine`
2. Identify orchestrate.rs code that wires these into the plan execution loop
3. Create `ConductorRuntime` in roko-conductor that wraps the monitoring lifecycle:
   - Activity observation per task
   - Stuck detection checks on periodic tick
   - Health snapshot emission
   - Circuit breaker state management
4. Expose `ConductorRuntime` for runner v2 and WorkflowEngine integration

**Acceptance criteria**:
- `ConductorRuntime` is a self-contained monitoring lifecycle manager
- No conductor/stuck/health wiring logic remains only in orchestrate.rs
- `cargo test -p roko-conductor` passes

### Task 2.6: Extract Daimon affect wiring into roko-daimon

**Source**: `crates/roko-cli/src/orchestrate.rs` (DaimonState loading, affect modulation
per dispatch, somatic signal emission, strategy observation)
**Target**: `crates/roko-daimon/` (crate already exists)

**Steps**:
1. roko-daimon already has `DaimonState`, `AffectEngine`, `DispatchParams`, `SomaticSignal`
2. Identify orchestrate.rs wiring:
   - Loading DaimonState from `.roko/daimon/affect.json`
   - Calling `daimon.modulate()` before dispatch to adjust temperature/turns/exploration
   - Recording `TaskStrategyObservation` after completion
   - Emitting `SomaticSignal` on failures
3. Create `DaimonRuntime` struct that encapsulates this lifecycle
4. Wire into `EffectDriver` as the `AffectPolicy` implementation (already partially done)

**Acceptance criteria**:
- `DaimonRuntime` handles load/modulate/observe/persist lifecycle
- `EffectDriver`'s `AffectPolicy` uses `DaimonRuntime`
- `cargo test -p roko-daimon` passes

### Task 2.7: Extract replan-on-gate-failure into roko-orchestrator

**Source**: `crates/roko-cli/src/orchestrate.rs` (`build_gate_failure_plan_revision()`,
`maybe_emit_gate_failure_plan_revision()`, `ReplanLedger`)
**Target**: `crates/roko-orchestrator/src/replan.rs` (new file)

**Steps**:
1. Extract the replan logic:
   - `GateFailureAction` enum (NeedsReplan, Retry, Fail)
   - `ReplanLedger` (deduplication of replan attempts)
   - `build_gate_failure_plan_revision()` (constructs revision prompt from gate errors)
2. Create `ReplanService` in roko-orchestrator:
   - `should_replan(failures: &[GateFailure], config: &LearningConfig) -> GateFailureAction`
   - `build_revision(failures: &[GateFailure], task: &Task) -> ReplanRequest`
3. Wire into runner v2's event loop on gate failure path

**Acceptance criteria**:
- `ReplanService` compiles in roko-orchestrator
- Runner v2 calls `should_replan()` when gate failures exhaust autofix budget
- `cargo test -p roko-orchestrator` passes

### Task 2.8: Delete orchestrate.rs and remove legacy-orchestrate feature flag

**File**: `crates/roko-cli/src/orchestrate.rs` (22,522 lines)
**Also**: All `#[cfg(feature = "legacy-orchestrate")]` guards across the codebase

**Steps**:
1. Verify all valuable code from Tasks 2.1-2.7 has been extracted
2. Run `grep -rn 'cfg.*legacy-orchestrate' crates/` to find all guard sites
3. For each guard site:
   - If the guarded code calls orchestrate.rs types, verify the replacement is wired
   - If the guarded code is a fallback path, verify the primary path works
   - Remove the `#[cfg]` guard and either keep the code (if still needed) or delete it
4. Delete `crates/roko-cli/src/orchestrate.rs`
5. Remove `legacy-orchestrate` from `Cargo.toml` features
6. Remove `pub mod orchestrate;` and `pub use orchestrate::*` from `lib.rs`

**Acceptance criteria**:
- `find crates/ -name 'orchestrate*.rs' | wc -l` returns 0
- `grep -rn 'legacy-orchestrate' crates/ Cargo.toml` returns 0
- `cargo build --workspace` passes
- `cargo test --workspace` passes
- No runtime behavior changes (all extracted code is wired through runner v2/WorkflowEngine)

**Dependencies**: Tasks 2.1-2.7 (all extractions must complete first)

---

## Phase 3: Eliminate Duplicate Dispatch Paths (5 tasks)

Nine dispatch paths with inconsistent model selection, cost tracking, and event
emission. Three files (`dispatch_direct.rs`, `dispatch_v2.rs`, `dispatch/mod.rs`)
overlap significantly.

### Task 3.1: Consolidate dispatch_direct.rs into dispatch_v2.rs

**Files**:
- `crates/roko-cli/src/dispatch_direct.rs` (403 lines)
- `crates/roko-cli/src/dispatch_v2.rs` (1,049 lines)

**What**: `dispatch_direct.rs` is the legacy dispatch path with hardcoded model strings
("claude-sonnet-4-6-20250514", "gpt-4o"). `dispatch_v2.rs` uses `ModelCallService`.
All callers of `dispatch_direct` should use `dispatch_v2` instead.

**Steps**:
1. Find all callers of `dispatch_direct::dispatch_prompt()`:
   - `chat_inline.rs` line 1778 (behind `#[cfg(feature = "legacy-orchestrate")]`)
   - `unified.rs` line 102 (fallback path)
2. Replace each caller with `dispatch_v2::dispatch_via_model_call_service()`
3. Delete `dispatch_direct.rs`
4. Remove `pub mod dispatch_direct;` from `lib.rs`

**Acceptance criteria**:
- `dispatch_direct.rs` deleted
- `grep -rn 'dispatch_direct' crates/` returns 0
- `cargo build -p roko-cli` passes
- Chat inline and unified dispatch use `dispatch_v2`

### Task 3.2: Merge dispatch/mod.rs wrapper into dispatch_v2.rs

**Files**:
- `crates/roko-cli/src/dispatch/mod.rs` (405 lines)
- `crates/roko-cli/src/dispatch_v2.rs` (1,049 lines)

**What**: `dispatch/mod.rs` is a thin wrapper that re-exports from `dispatch_v2` and adds
`AgentDispatchManager`. The two files share types and have overlapping responsibility.

**Steps**:
1. Move any unique functionality from `dispatch/mod.rs` into `dispatch_v2.rs`
2. Update all imports from `crate::dispatch::*` to `crate::dispatch_v2::*`
3. If `dispatch/mod.rs` becomes empty or just re-exports, delete the module
4. Rename `dispatch_v2.rs` to `dispatch.rs` (it is now the canonical dispatch module)

**Acceptance criteria**:
- Single dispatch module: `crates/roko-cli/src/dispatch.rs`
- No `dispatch_v2` or `dispatch/mod.rs` remains
- All callers updated
- `cargo build -p roko-cli` passes

**Dependencies**: Task 3.1

### Task 3.3: Remove hardcoded model strings from CLI dispatch

**Files** (8 instances found in audit):
- `crates/roko-cli/src/run.rs` line ~530: `"claude-sonnet-4-6"`
- `crates/roko-cli/src/run.rs` line ~657: `"llama3.1:8b"`
- `crates/roko-cli/src/auth_detect.rs` line ~42: `"claude-sonnet-4-6"`
- `crates/roko-cli/src/plan_generate.rs` lines 130-131: `"claude-haiku-4-5"`, `"claude-sonnet-4-6"`

**Steps**:
1. For each hardcoded model string, trace where the value should come from:
   - `run.rs`: Should read `config.agent.default_model` or fall back to `ServiceFactory::resolve_model()`
   - `auth_detect.rs`: Should not have a model string at all -- model resolution belongs in ServiceFactory
   - `plan_generate.rs`: Task tier model hints are documentation/prompt content (acceptable)
2. Replace runtime model strings with config reads: `config.agent.default_model.as_deref().unwrap_or("claude-sonnet-4-6")`
3. For `auth_detect.rs`, remove model selection entirely -- it should detect auth credentials
   only, not resolve models
4. Add a `const DEFAULT_FALLBACK_MODEL: &str = "claude-sonnet-4-6"` in `roko-core/src/config/`
   as the single source of truth for the fallback

**Acceptance criteria**:
- `grep -rn '"claude-sonnet-4-6"' crates/roko-cli/src/{run,auth_detect,dispatch}*.rs` returns 0
  (excluding test code and prompt template strings)
- Setting `default_model = "cerebras-70b"` in roko.toml causes `roko run` to use Cerebras
- A single `DEFAULT_FALLBACK_MODEL` constant exists in roko-core

**Dependencies**: Task 3.1 (dispatch_direct removal)

### Task 3.4: Unify model selection through ServiceFactory

**Files**:
- `crates/roko-cli/src/auth_detect.rs` (159 lines)
- `crates/roko-orchestrator/src/service_factory.rs`
- `crates/roko-cli/src/model_selection.rs`

**What**: `auth_detect.rs` scans environment variables in fixed priority to determine model
and provider, ignoring `roko.toml` configuration. This path disagrees with `ServiceFactory::resolve_model()`.
Nine entry points use different model resolution, causing the same user config to select
different models depending on the command.

**Steps**:
1. Change `auth_detect.rs` to detect ONLY provider credentials (API keys present, CLI binary
   available) without resolving a model
2. Model resolution should always go through `ServiceFactory::resolve_model()` which reads
   roko.toml `default_model`, merges with CLI `--model` flag, and applies CascadeRouter
3. Update all entry points (`run`, `chat`, `plan run`, `prd draft`, `prd plan`, `research`,
   `agent chat`) to use ServiceFactory for model resolution
4. Delete `EffectiveModelSelection` if it duplicates ServiceFactory logic

**Acceptance criteria**:
- `auth_detect.rs` has zero model string constants
- All 9 entry points use the same model resolution path
- `roko config show` model matches the model actually used by `roko run`
- `cargo test -p roko-cli` passes

**Dependencies**: Task 3.3

### Task 3.5: Remove unsafe env::set_var calls

**Files**:
- `crates/roko-cli/src/commands/util.rs` line 236: `unsafe { std::env::set_var("ROKO_PROVIDER", p) }`
- `crates/roko-cli/src/main.rs` line 2225: `unsafe { std::env::set_var("ROKO_HIGH_CONTRAST", "1") }`
- `crates/roko-cli/src/main.rs` line 2229: `unsafe { std::env::set_var("ROKO_REDUCED_MOTION", "1") }`

**What**: `set_var` is unsafe in multi-threaded contexts (UB if another thread reads env
concurrently). The `--provider` flag uses it to inject a provider override through the
env var `ROKO_PROVIDER`. This should use config struct propagation instead.

**Steps**:
1. For `ROKO_PROVIDER`: Add a `provider_override: Option<String>` field to the config
   struct passed into dispatch. Remove the `set_var` call. Pass the override via config.
2. For `ROKO_HIGH_CONTRAST` and `ROKO_REDUCED_MOTION`: These are read by the TUI theme.
   Set them before spawning the tokio runtime (while still single-threaded) or pass them
   as config fields to the TUI.
3. Search for all `std::env::set_var` calls in the workspace and audit each one

**Acceptance criteria**:
- `grep -rn 'set_var' crates/roko-cli/src/ | grep -v '// safe:' | grep -v test` returns 0
- `--provider anthropic` flag works without env var mutation
- `cargo test -p roko-cli` passes

---

## Phase 4: Fix StateHub Type Duplication (3 tasks)

Both `roko-cli` and `roko-serve` include `state_hub.rs` via `#[path]` includes from
`roko-core`, creating two incompatible `StateHub` types. DashboardEvents from CLI
runs cannot flow to HTTP SSE/WebSocket endpoints.

### Task 4.1: Move StateHub into roko-runtime as first-class module

**Files**:
- `crates/roko-core/src/state_hub.rs` (source file)
- `crates/roko-cli/src/lib.rs` line 36-37: `#[path = "../../roko-core/src/state_hub.rs"] pub mod state_hub;`
- `crates/roko-serve/src/lib.rs` line 68-69: `#[path = "../../roko-core/src/state_hub.rs"] pub mod state_hub_compat;`

**What**: `state_hub.rs` lives in roko-core but depends on roko_runtime types, creating
a circular dependency. The `#[path]` hack compiles it twice -- once in roko-cli's module
tree, once in roko-serve's. The two copies produce distinct Rust types that cannot be
shared across crate boundaries.

**Steps**:
1. Move `state_hub.rs` into `crates/roko-runtime/src/state_hub.rs`
2. Add `pub mod state_hub;` to `crates/roko-runtime/src/lib.rs`
3. Re-export from roko-runtime: `pub use state_hub::{StateHub, SharedStateHub, DashboardEvent}`
4. Update roko-cli to import from roko-runtime: `use roko_runtime::state_hub::StateHub`
5. Update roko-serve to import from roko-runtime: `use roko_runtime::state_hub::StateHub`
6. Remove both `#[path]` includes
7. If roko-core needs StateHub types, add roko-runtime as a dependency or use traits

**Acceptance criteria**:
- `grep -rn '#\[path.*state_hub' crates/` returns 0
- `StateHub` is defined in exactly one place: `crates/roko-runtime/src/state_hub.rs`
- Both roko-cli and roko-serve import the SAME type
- `cargo build --workspace` passes

### Task 4.2: Remove remaining #[path] includes

**Files** (from grep):
- `crates/roko-cli/src/prd.rs` line 17: `#[path = "plan_validate.rs"]`
- `crates/roko-cli/src/lib.rs` line 81: `#[path = "../../../scripts/layer_check.rs"]`
- `crates/roko-core/src/lib.rs` lines 109-117: `#[path = "../obs/*.rs"]` (4 includes)

**What**: `#[path]` includes are a code smell indicating module structure problems.
Each one creates a compilation unit that could be a proper module or dependency.

**Steps**:
1. For `plan_validate.rs` in prd.rs: Rename or restructure so it is a normal `mod` include
2. For `layer_check.rs` from scripts: Move the check into roko-cli as a proper module
   or add scripts as a build dependency
3. For roko-core obs includes: These import from `../obs/` which is outside the src tree.
   Move the obs files into `crates/roko-core/src/obs/` as a proper submodule
4. For each, verify the module structure makes sense architecturally

**Acceptance criteria**:
- `grep -rn '#\[path' crates/*/src/lib.rs crates/*/src/prd.rs` returns 0 (excluding test files)
- `cargo build --workspace` passes
- Module structure is navigable without path tricks

**Dependencies**: Task 4.1

### Task 4.3: Wire StateHub sharing between serve and CLI

**File**: `crates/roko-cli/src/commands/util.rs` line ~271
**What**: `let external_hub: Option<&roko_cli::state_hub::StateHub> = None;` -- the TODO
explains the two crates define distinct StateHub types. After Task 4.1 unifies the type,
this can be wired.

**Steps**:
1. After Task 4.1, both crates use the same `StateHub` type from roko-runtime
2. When `--serve` is passed to `roko run`, create a shared `Arc<StateHub>`
3. Pass the same `Arc<StateHub>` to both the serve `AppState` and the run context
4. DashboardEvents from the run flow to the serve SSE/WebSocket endpoints
5. Verify in the TUI that events from `roko run --serve` appear in real-time

**Acceptance criteria**:
- `roko run --serve "hello"` produces SSE events on `http://localhost:6677/events`
- DashboardEvents from the run appear in the TUI when connected to the serve endpoint
- The TODO comment at util.rs ~271 is removed

**Dependencies**: Task 4.1

---

## Phase 5: Remove Dead and Duplicate Code (6 tasks)

### Task 5.1: Delete dispatch_direct.rs after consolidation

**File**: `crates/roko-cli/src/dispatch_direct.rs` (403 lines)

This is a duplicate of Task 3.1 -- included here for completeness in the dead code
tracking. If Phase 3 is done first, this task is already complete.

**Acceptance criteria**: File deleted, no references remain.

**Dependencies**: Task 3.1

### Task 5.2: Remove duplicate response parsers

**What**: Agent response parsing is implemented in multiple locations:
- `crates/roko-agent/src/claude_cli_agent.rs` -- stream-json parser
- `crates/roko-acp/src/bridge_events.rs` -- ClaudeStreamEvent parser
- `crates/roko-cli/src/runner/agent_stream.rs` -- line-by-line JSON stream parser
- `crates/roko-cli/src/chat_session.rs` -- output parsing for chat

**Steps**:
1. Identify the canonical parser (likely `bridge_events.rs` as it handles all event types)
2. Compare each parser's event handling to find unique functionality
3. Consolidate into a single `StreamParser` in roko-agent that all consumers use
4. Update runner, chat, and ACP to import from the canonical location
5. Delete duplicate implementations

**Acceptance criteria**:
- Stream-json parsing exists in one location
- All consumers (runner, chat, ACP) use the same parser
- `cargo test --workspace` passes

### Task 5.3: Remove stale module declarations

**What**: `lib.rs` declares modules that may have no live callers after orchestrate.rs
removal. Candidates from the module list:

- `pub mod snapshot_migrate` -- used only by orchestrate.rs?
- `pub mod snapshot_reconcile` -- used only by orchestrate.rs?
- `pub mod surface_inventory` -- status command or dead?
- `pub mod pipe` -- stdin piping, may be dead
- `pub mod inline` -- vs `chat_inline`, overlap?

**Steps**:
1. For each candidate module, search for callers: `grep -rn 'module_name::' crates/`
2. If zero callers outside tests and the module itself, mark as dead
3. Check if the module is reachable from any CLI subcommand
4. Delete confirmed dead modules
5. For modules with partial usage, evaluate whether they should be merged into related modules

**Acceptance criteria**:
- Every `pub mod` in `lib.rs` has at least one live caller from a CLI subcommand
- Dead modules deleted
- `cargo build -p roko-cli` passes

**Dependencies**: Task 2.8 (orchestrate.rs deletion may reveal newly dead modules)

### Task 5.4: Remove duplicate Usage types

**Files**:
- `crates/roko-core/src/...` -- `Usage` struct with `u32` fields and `f32` cost
- `crates/roko-agent/src/usage.rs` -- `UsageObservation` with `Option<u64>` fields and `f64` cost

**What**: Two usage types exist with different precision and nullability semantics.
`From` impls exist between them but the clamping (u64 -> u32) loses data.

**Steps**:
1. Audit all usage of the legacy `Usage` type across the workspace
2. Migrate all consumers to `UsageObservation` (the canonical type with better semantics)
3. If roko-core needs a usage type for trait signatures, define it as a re-export of
   roko-agent's type or as a trait
4. Remove the legacy `Usage` struct from roko-core
5. Remove the `From` conversion impls

**Acceptance criteria**:
- Single `UsageObservation` type used everywhere
- No `u32` usage fields remain (all `Option<u64>`)
- No lossy clamping in conversions
- `cargo test --workspace` passes

### Task 5.5: Remove VCG auction dead code or wire it

**File**: `crates/roko-compose/src/auction.rs`
**What**: `vcg_allocate()` is built and exported but the greedy path dominates at runtime.
The auction has zero live callers.

**Steps**:
1. `grep -rn 'vcg_allocate' crates/` to confirm zero callers
2. Decision: either wire it into `PromptAssemblyService` as the allocation strategy
   (replacing the greedy path) or delete it
3. If wiring: add a config flag `prompt.allocation_strategy = "vcg" | "greedy"` and
   call `vcg_allocate()` from `PromptAssemblyService::assemble()` when selected
4. If deleting: remove `auction.rs` and its types from the module tree

**Acceptance criteria**:
- `vcg_allocate()` either has a live caller or is deleted
- No "built but never called" code remains in roko-compose
- `cargo build -p roko-compose` passes

### Task 5.6: Remove or wire write-only learning artifacts

**What**: Four learning artifacts are written but never read (from Ground Truth doc):
- Conductor observations (`.roko/conductor/observations.jsonl`)
- Dream triggers (`.roko/learn/dream_triggers.jsonl`)
- Knowledge candidates (`.roko/learn/knowledge_candidates.jsonl`)
- Gateway events (`.roko/events/gateway.jsonl`)

**Steps**:
1. For each artifact, decide: wire a consumer or stop writing
2. Conductor observations: wire into stuck detection or meta-cognition feedback
3. Dream triggers: wire into `DreamRunner::run_cycle()` trigger logic
4. Knowledge candidates: wire into `KnowledgeStore::ingest()` post-run hook
5. Gateway events: wire into cost dashboard in TUI and `roko learn efficiency`
6. For any that remain write-only after analysis, remove the writer to reduce I/O

**Acceptance criteria**:
- Every JSONL artifact written has at least one consumer that reads it
- OR the writer is removed if no consumer is justified
- `cargo test --workspace` passes

---

## Phase 6: Config Inconsistencies (4 tasks)

### Task 6.1: Unify gate config format (`[[gate]]` vs `[gates]`)

**Files**:
- `crates/roko-cli/src/commands/init.rs` -- writes `[[gate]]` arrays
- `crates/roko-core/src/config/schema.rs` -- `RokoConfig::from_toml()` reads `[gates]` table

**What**: `roko init` generates `[[gate]]` TOML arrays. The runtime parser silently
discards them because it expects `[gates]` tables. Gates generated by `roko init` are
invisible to `roko plan run`.

**Steps**:
1. In `RokoConfig::from_toml()`, add support for parsing `[[gate]]` arrays
2. Normalize both formats to the internal `GatesConfig` struct
3. OR change `roko init` to emit the `[gates]` format the runtime reads
4. Add a deprecation warning if `[[gate]]` format is detected, suggesting migration
5. Add a test that creates a config with `[[gate]]` and verifies gates are loaded

**Acceptance criteria**:
- `roko init` generates gates config that `roko plan run` actually uses
- Running `roko init && roko plan run` uses the initialized gate configuration
- Test covers both formats
- `cargo test -p roko-core` passes

### Task 6.2: Normalize model aliases in config loading

**Files**:
- `crates/roko-core/src/config/schema.rs`
- `crates/roko-orchestrator/src/service_factory.rs`

**What**: `glm-5-1` on provider "zai" and `glm51` on provider "zhipu" both resolve to
the same model. Multiple Claude aliases resolve to the same model ID. No normalization
happens at config load time.

**Steps**:
1. Create a `normalize_model_slug(slug: &str) -> String` function in roko-core config
2. Maintain a mapping of known aliases:
   - `"glm-5-1"` / `"glm51"` -> `"glm-5.1"`
   - `"sonnet"` / `"claude-sonnet"` -> `"claude-sonnet-4-6"`
   - etc.
3. Call `normalize_model_slug()` at config load time for all model references
4. Call it in `ServiceFactory::resolve_model()` before lookup
5. Add tests for alias resolution

**Acceptance criteria**:
- `roko config show` displays normalized model slugs
- `default_model = "glm51"` and `default_model = "glm-5-1"` resolve to the same model
- `cargo test -p roko-core` passes

### Task 6.3: Fix config fields that are loaded but not used

**What**: From the config analysis, these fields are loaded but ignored by most paths:

| Field | Loaded | Used |
|---|---|---|
| `agent.tier_models` | Into CascadeRouter | Never queried at dispatch |
| `workflow.template` | By WorkflowEngine | Not by runner v2 |
| `learning.replan_on_gate_failure` | By orchestrate.rs | Not by runner v2 |
| `budget.max_cost_per_run` | By orchestrate.rs | Not by runner v2 |
| `tools.profiles` | Into instructions | Never enforced at gate level |

**Steps**:
1. For each field, trace the config load path and identify where it should be read
2. Wire `tier_models` into `CascadeRouter::route()` at dispatch time
3. Wire `workflow.template` into runner v2's dispatch config
4. Wire `learning.replan_on_gate_failure` into runner v2's event loop (connects to Task 2.7)
5. Wire `budget.max_cost_per_run` into runner v2's budget check
6. Either wire `tools.profiles` enforcement or document it as advisory-only

**Acceptance criteria**:
- Each config field either affects runtime behavior or is documented as informational
- Setting `workflow.template = "full"` in config changes behavior in both `roko run` and `roko plan run`
- `budget.max_cost_per_run = 0.01` stops execution when cost exceeds $0.01

**Dependencies**: Tasks 2.7 (replan), 3.4 (model selection)

### Task 6.4: Fix inconsistent max_tokens across dispatch paths

**What**: Same model gets different max_tokens depending on entry point:
- `dispatch_direct.rs`: 8192
- Anthropic adapter: 4096
- Gateway routes: 1024
- Demo mode: 512

**Steps**:
1. Define max_tokens resolution order: model profile -> provider config -> global default
2. Add `default_max_tokens` to `ModelProfile` in roko-core config schema
3. Remove hardcoded max_tokens from each dispatch path
4. All paths read from the resolved model profile
5. Add a `MAX_TOKENS_DEFAULT: u32 = 4096` constant as the last-resort fallback

**Acceptance criteria**:
- Same model produces same max_tokens regardless of entry point
- `cargo test --workspace` passes
- Setting `max_tokens = 16384` in model profile config is respected by all paths

**Dependencies**: Task 3.1 (dispatch_direct removal)

---

## Phase 7: Test Coverage for Critical Paths (5 tasks)

### Task 7.1: Add integration test for runner v2 learning persistence

**File**: `crates/roko-cli/tests/` (new test file)
**What**: No test verifies that runner v2 writes learning artifacts. This is the most
critical gap because it means the default execution path has no verified feedback loop.

**Steps**:
1. Create `crates/roko-cli/tests/runner_learning.rs`
2. Set up a minimal plan with 2 tasks (one succeeds, one fails a gate)
3. Use `ROKO_MOCK_AGENT_REPLY` to avoid real API calls
4. Run through runner v2's event loop
5. Assert learning artifacts exist and contain expected data:
   - `.roko/episodes.jsonl` has entries
   - `.roko/learn/cascade-router.json` has observations
   - `.roko/learn/gate-thresholds.json` has rung stats
   - `.roko/learn/efficiency.jsonl` has entries

**Acceptance criteria**:
- Test runs in CI without API keys
- Test verifies learning persistence end-to-end
- `cargo test -p roko-cli --test runner_learning` passes

**Dependencies**: PLAN 1 from `06-IMPLEMENTATION-PLANS.md` (learning must be wired first)

### Task 7.2: Add test for config-respected model selection

**File**: `crates/roko-cli/tests/` (new test file)
**What**: No test verifies that `default_model` in roko.toml is actually used by dispatch.

**Steps**:
1. Create `crates/roko-cli/tests/model_selection.rs`
2. Create a temp roko.toml with `default_model = "test-model-slug"`
3. Trace through `ServiceFactory::resolve_model()` and assert it returns "test-model-slug"
4. Test with `--model` CLI override and verify it takes precedence
5. Test with no config and verify fallback is used

**Acceptance criteria**:
- Test covers: config-only, CLI-override, no-config-fallback scenarios
- `cargo test -p roko-cli --test model_selection` passes

### Task 7.3: Add test for gate config loading from both formats

**File**: `crates/roko-core/tests/` (new test file or extend existing)
**What**: No test verifies that both `[[gate]]` and `[gates]` formats are loaded correctly.

**Steps**:
1. Create test with `[[gate]]` TOML: `[[gate]]\ntype = "compile"\n[[gate]]\ntype = "test"`
2. Create test with `[gates]` TOML: `[gates]\nenabled = ["compile", "test"]`
3. Parse both and assert they produce identical `GatesConfig`
4. Test edge cases: empty gates, shell gates in both formats, unknown gate types

**Acceptance criteria**:
- Both gate config formats produce identical internal representation
- `cargo test -p roko-core` passes

**Dependencies**: Task 6.1

### Task 7.4: Add test for StateHub event flow between serve and CLI

**File**: `crates/roko-serve/tests/` or `crates/roko-runtime/tests/`
**What**: No test verifies that DashboardEvents published from a run reach the serve SSE endpoint.

**Steps**:
1. Create a shared `StateHub` instance
2. Publish a `DashboardEvent` from one side
3. Subscribe from the other side and verify receipt
4. Test with the SSE endpoint if feasible (may need an in-process server)

**Acceptance criteria**:
- Events published to StateHub are received by subscribers
- Same StateHub instance works across serve and CLI contexts
- `cargo test` passes

**Dependencies**: Task 4.1

### Task 7.5: Add regression tests for HOLLOW items

**What**: Three HOLLOW items from the audit should have regression tests preventing recurrence:
1. `roko config mcp` -- should not panic
2. API provider chat -- should return clear error, not `todo!()`
3. Plan regenerate diagnostics -- validation errors should be injected

**Steps**:
1. For MCP config: test that `ConfigCmd::Mcp` handler returns Ok, not unreachable!()
2. For API provider: test that `send_turn_api()` returns a descriptive error, not panic
3. For plan regenerate: test that validation failure triggers a retry with diagnostics

**Acceptance criteria**:
- Each HOLLOW item has a test that would catch the original bug
- Tests run in CI
- `cargo test --workspace` passes

---

## Phase 8: Structural Cleanup (3 tasks)

### Task 8.1: Break up run.rs (3,624 lines)

**File**: `crates/roko-cli/src/run.rs`
**What**: run.rs is the second-largest file after orchestrate.rs. It contains the V2
WorkflowEngine entry point, legacy `run_once()`, inline rendering, prompt assembly,
and 45+ `#[cfg(feature = "legacy-orchestrate")]` blocks.

**Steps**:
1. After orchestrate.rs deletion (Task 2.8), remove all `#[cfg(feature = "legacy-orchestrate")]`
   blocks from run.rs (this alone may remove 1,000+ lines)
2. Extract WorkflowEngine setup into `run_workflow.rs`
3. Extract prompt assembly helpers into `run_prompt.rs`
4. Extract report formatting into `run_report.rs`
5. Keep `run.rs` as the entry point with `pub fn run_once()` that delegates

**Acceptance criteria**:
- `run.rs` is under 800 lines
- Each extracted module has a clear single responsibility
- `cargo build -p roko-cli` passes

**Dependencies**: Task 2.8

### Task 8.2: Break up main.rs (4,423 lines)

**File**: `crates/roko-cli/src/main.rs`
**What**: main.rs contains all CLI argument parsing (clap derives) plus handler dispatch.
The clap structs and the handler functions should be separated.

**Steps**:
1. Extract clap struct definitions into `crates/roko-cli/src/cli.rs`
2. Keep `main()` in main.rs as a thin dispatcher: parse args, call handler
3. Each command group's handler is already in `commands/` -- main.rs should just route

**Acceptance criteria**:
- `main.rs` is under 500 lines
- All clap struct definitions in `cli.rs`
- `cargo build -p roko-cli` passes

### Task 8.3: Consolidate event types (DashboardEvent vs ServerEvent vs RuntimeEvent)

**What**: Three overlapping event types with lossy conversion between them:
- `DashboardEvent` (TUI) in state_hub.rs
- `ServerEvent` (HTTP SSE) in roko-serve
- `RuntimeEvent` (internal) in roko-runtime

**Steps**:
1. Map all three event types and identify overlapping variants
2. Define a canonical `RuntimeEvent` in roko-runtime that covers all variants
3. Make `DashboardEvent` and `ServerEvent` thin wrappers or `From` conversions
   from `RuntimeEvent` with zero information loss
4. Remove duplicate event variants
5. Ensure TUI and SSE consumers get all events without lossy conversion

**Acceptance criteria**:
- One canonical event type with lossless conversion to presentation types
- TUI shows all events that SSE shows (and vice versa)
- `cargo test --workspace` passes

**Dependencies**: Task 4.1 (StateHub must be unified first)

---

## Dependency Graph

```
Phase 1 (lint blankets)
  1.1 -> 1.2 -> 1.3

Phase 2 (orchestrate.rs decomposition)
  2.1, 2.2, 2.3, 2.4, 2.5, 2.6, 2.7  (all independent)
  -> 2.8 (delete orchestrate.rs, depends on all 2.x)

Phase 3 (dispatch consolidation)
  3.1 -> 3.2 -> 3.3 -> 3.4
  3.5 (independent)

Phase 4 (StateHub)
  4.1 -> 4.2
  4.1 -> 4.3

Phase 5 (dead code)
  5.1 depends on 3.1
  5.2, 5.4, 5.5, 5.6 (independent)
  5.3 depends on 2.8

Phase 6 (config)
  6.1, 6.2 (independent)
  6.3 depends on 2.7, 3.4
  6.4 depends on 3.1

Phase 7 (tests)
  7.1 depends on learning wiring (PLAN 1 from 06-IMPLEMENTATION-PLANS.md)
  7.2, 7.5 (independent)
  7.3 depends on 6.1
  7.4 depends on 4.1

Phase 8 (structural)
  8.1 depends on 2.8
  8.2 (independent)
  8.3 depends on 4.1
```

## Critical Path

The longest dependency chain:

```
1.1 -> 1.2 -> 1.3 -> 2.1-2.7 (parallel) -> 2.8 -> 5.3 -> 8.1
```

Estimated: ~12-15 days for the critical path, ~20-25 days total with parallelism.

## Execution Order (Recommended)

| Wave | Tasks | Rationale |
|---|---|---|
| 1 | 1.1, 1.2, 3.5, 6.1, 6.2 | Quick wins, no dependencies, surfaces problems |
| 2 | 1.3, 2.1-2.7 (parallel), 3.1, 4.1 | Major extraction work, dispatch cleanup starts |
| 3 | 2.8, 3.2, 3.3, 4.2, 4.3, 5.2, 5.4 | Delete orchestrate.rs, continue consolidation |
| 4 | 3.4, 5.3, 5.5, 5.6, 6.3, 6.4 | Wire config, remove dead code |
| 5 | 7.1-7.5, 8.1-8.3 | Tests and structural cleanup |

---

## Summary

| Phase | Tasks | Lines Removed (est.) | Lines Added (est.) | Net |
|---|---|---|---|---|
| 1. Lint blankets | 3 | 200 | 50 | -150 |
| 2. orchestrate.rs | 8 | 22,522 | 2,000 | -20,522 |
| 3. Dispatch paths | 5 | 1,500 | 200 | -1,300 |
| 4. StateHub | 3 | 100 | 150 | +50 |
| 5. Dead code | 6 | 1,500 | 100 | -1,400 |
| 6. Config | 4 | 100 | 400 | +300 |
| 7. Tests | 5 | 0 | 800 | +800 |
| 8. Structural | 3 | 2,000 | 2,200 | +200 |
| **Total** | **37** | **~27,922** | **~5,900** | **~-22,022** |

Net effect: ~22K lines removed from the codebase while preserving all runtime functionality,
adding test coverage for critical paths, and eliminating the primary sources of confusion
(three execution engines, nine dispatch paths, two StateHub types, two gate config formats).
