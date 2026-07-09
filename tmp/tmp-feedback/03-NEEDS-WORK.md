# NEEDS_WORK Tasks — Partial Implementations

These tasks have real code changes but are missing significant spec requirements, have incorrect logic, or lack required tests.

---

## Task 004: Workspace/RokoLayout Boundary Migration

**Priority**: 3/10 (foundational but large; downstream tasks don't block on it)
**Effort**: L (multi-session; 128 RokoLayout callsites across 30 files)

### Current State

- `crates/roko-core/src/workspace.rs` (278 lines): `Workspace` struct with `open()`, `create()`, `open_or_create()` and 28 path accessors covering `.roko/` subdirectories.
- Migration warning in `Workspace::open()` (lines 43-53): emits `tracing::warn!` when `.roko/memory` exists without `.roko/learn`.
- The accessors required by the spec are all present: `events_jsonl_path()`, `run_state_path()`, `task_trackers_path()`, `playbooks_dir()`, `archive_dir()`, `mcp_config_path()`, `runner_stderr_log()`, `learn_episodes_path()`, `engrams_path()`, `serve_pid_file()`.
- `Workspace` is imported in only 2 runtime files: `crates/roko-cli/src/commands/util.rs` and `crates/roko-cli/src/main.rs`.

### Gap Analysis

The spec required 5 work items; only item 1 (expand accessors) and item 2 (migration warning) are done.

| Spec Requirement | Status |
|---|---|
| Expand Workspace accessors | DONE -- all listed accessors exist |
| Migration warning in `open()` | DONE -- lines 43-53 |
| Fix episode path ordering | NOT DONE |
| Migrate high-risk runtime RokoLayout usage | NOT DONE -- 128 callsites across 30 files remain |
| Document exceptions in Status Log | NOT DONE |

Specific gaps:
1. `project_episode_paths()` in `crates/roko-learn/src/runtime_feedback.rs:3209` -- ordering is now correct (root -> learn -> memory), but still uses raw `.join(".roko")` instead of Workspace accessors.
2. `crates/roko-serve/src/routes/workspaces.rs` -- still imports `RokoLayout` (3 callsites), not migrated to Workspace.
3. `crates/roko-core/src/dashboard_snapshot.rs` -- not migrated.
4. `crates/roko-cli/src/dispatch/prompt_builder.rs` -- not migrated.
5. `crates/roko-fs/src/archive.rs` -- still uses RokoLayout (documented exception, acceptable).
6. `AppState.layout` in serve dispatch is still `RokoLayout` (15 callsites in `crates/roko-serve/src/dispatch.rs`).
7. Runner subsystem (`crates/roko-cli/src/runner/`) has 14 RokoLayout callsites across 6 files.

### Completion Design

**Step 1: Migrate the 4 spec-named files** (items 3-4 of the spec)

- `crates/roko-learn/src/runtime_feedback.rs`:
  - At `project_episode_paths()` (line 3209): accept `&Workspace` or use Workspace accessors instead of raw `.join(".roko")`.
  - Pattern: `ws.episodes_path()`, `ws.learn_episodes_path()`, `ws.memory_dir().join("episodes.jsonl")`.

- `crates/roko-serve/src/routes/workspaces.rs`:
  - Replace `RokoLayout::for_project(workdir)` with `Workspace::open(workdir)?` in `get_workspace_state()`.
  - Episode reading: try `ws.episodes_path()`, then `ws.learn_episodes_path()`, then `ws.memory_dir().join("episodes.jsonl")`.
  - Remove `use roko_fs::layout::RokoLayout` if no remaining callsites.

- `crates/roko-core/src/dashboard_snapshot.rs`:
  - In `load_from_workdir()` and `bootstrap_episodes()`, derive `Workspace` from resolved root.
  - Use `ws.state_dir()`, `ws.learn_dir()`, `ws.episodes_path()` etc.
  - Keep memory fallback only in `bootstrap_episodes()`.

- `crates/roko-cli/src/dispatch/prompt_builder.rs`:
  - Use `Workspace` accessors for playbooks and episode candidate paths.

**Step 2: Document remaining exceptions** (item 5 of the spec)

Run `rg 'RokoLayout' crates/ -g '*.rs'` and categorize each hit as:
- `roko-fs internal` (layout.rs, gc.rs, archive.rs) -- documented exception
- `test fixture` -- acceptable
- `runner subsystem` -- future migration wave
- `serve dispatch` -- future migration wave (AppState.layout)

Paste categorized grep output into the Status Log.

**Tests**:
- `cargo test -p roko-core workspace`
- `cargo test -p roko-learn project_episode_paths`
- `cargo test -p roko-cli learn_paths`

---

## Task 007: Gate Pipeline TOML-Configurable Shell Commands

**Priority**: 2/10 (blocks correct gate behavior for all plan runs)
**Effort**: M (concentrated in 3 files, clear mechanical steps)

### Current State

- `crates/roko-core/src/config/gates.rs`: `GateRungConfig` struct with `name`, `command`, `timeout_secs`, `required`, `parallel_with` fields. `GatesConfig::effective_rungs()` (lines 85-112) returns shell command configs -- but with wrong timeouts.
- `crates/roko-cli/src/runner/gate_dispatch.rs`: `spawn_gate()` (line 28) still takes a `rung: u32` parameter and dispatches via `GatePipelineBuilder::from_config()` / `from_config_with_execution()` -- the Rust-native path, not shell commands.
- `crates/roko-cli/src/runner/event_loop.rs`: `gate_timeout()` (line 143) still does `rung == Rung::Compile.as_index()` / `rung == Rung::Lint.as_index()` magic-number matching.
- `effective_rungs()` is never called from any non-test runtime path (confirmed: `rg 'effective_rungs' crates/roko-cli/ -g '*.rs'` returns no matches).

### Gap Analysis

| Spec Requirement | Status |
|---|---|
| Wire `effective_rungs()` into gate pipeline startup | NOT DONE -- zero runtime callers |
| Replace `run_rung()` with shell command execution | NOT DONE -- still uses GatePipelineBuilder Rust path |
| Convert built-in gates to shell commands with correct timeouts | PARTIAL -- defaults exist but compile=120s (spec: 300s), test=300s (spec: 600s) |
| Support `parallel_with` | NOT DONE -- field parsed, never used |
| Remove hardcoded rung numbers from event_loop.rs | NOT DONE -- `rung == Rung::Compile.as_index()` still active at line 148 |
| `effective_rungs()` respects `clippy_enabled`/`skip_tests` | NOT DONE -- always returns all 3 rungs |

### Completion Design

**File: `crates/roko-core/src/config/gates.rs`**

1. Fix `effective_rungs()` default timeouts:
   - compile: `timeout_secs: 300` (was 120)
   - test: `timeout_secs: 600` (was 300)
   - lint: `timeout_secs: 120` (correct)

2. Make `effective_rungs()` respect flags:
   ```rust
   pub fn effective_rungs(&self) -> Vec<GateRungConfig> {
       if self.has_custom_rungs() {
           return self.custom_rungs.clone();
       }
       let mut rungs = vec![
           GateRungConfig { name: "compile".into(), command: "cargo build --workspace".into(),
               timeout_secs: 300, required: true, parallel_with: vec![] },
       ];
       if self.clippy_enabled {
           rungs.push(GateRungConfig { name: "lint".into(),
               command: "cargo clippy --workspace --no-deps -- -D warnings".into(),
               timeout_secs: 120, required: true, parallel_with: vec![] });
       }
       if !self.skip_tests {
           rungs.push(GateRungConfig { name: "test".into(),
               command: "cargo test --workspace".into(),
               timeout_secs: 600, required: true, parallel_with: vec![] });
       }
       rungs
   }
   ```

3. Add unit tests: `skip_tests_omits_test_rung`, `clippy_disabled_omits_lint_rung`, `custom_rungs_replace_defaults`.

**File: `crates/roko-cli/src/runner/gate_dispatch.rs`**

4. Add a new function `spawn_configured_gate_pipeline()` that:
   - Takes `Vec<GateRungConfig>` instead of `rung: u32`
   - Groups rungs by `parallel_with` into execution groups
   - For each group, runs rungs concurrently via `ShellGate::new("sh", vec!["-c", &rung.command])`
   - Uses each rung's `timeout_secs` for per-rung timeout
   - Aggregates verdicts: `required=true` failures fail the gate; `required=false` failures become passing verdicts with detail
   - Appends task-level `verify` step verdicts after all configured rungs

5. Keep existing `spawn_gate()` as fallback for sentinel rungs (RUNG_PLAN_VERIFY=1000, RUNG_MERGE=1001).

**File: `crates/roko-cli/src/runner/event_loop.rs`**

6. In the `ExecutorAction::RunGate` handler:
   - Call `gates_config.effective_rungs()` to get the configured pipeline
   - Call `spawn_configured_gate_pipeline()` instead of `spawn_gate()` for regular gate rungs
   - Remove the `completion.rung < config.max_gate_rung` advancement loop -- the pipeline runs all rungs in one pass

7. Replace `gate_timeout()` magic-number matching:
   - For configured pipeline: use rung-specific `timeout_secs` from config
   - Keep fallback timeout only for RUNG_PLAN_VERIFY and RUNG_MERGE sentinel values

**Tests to add/update**:
- `crates/roko-core/src/config/gates.rs`: test `effective_rungs()` with `skip_tests=true`, `clippy_enabled=false`, custom rungs
- `crates/roko-cli/src/runner/gate_dispatch.rs`: test shell gate pass/fail, optional rung failure, `parallel_with` pair
- Update event-loop tests that assumed multi-rung numeric advancement

---

## Task 014: Clippy Suppression Removal

**Priority**: 7/10 (cleanup task; lib.rs suppression is worse than main.rs)
**Effort**: S-M (removing the attribute is trivial; fixing resulting warnings may cascade)

### Current State

- `crates/roko-cli/src/main.rs` lines 11-12: only `#![allow(clippy::too_many_lines)]` and `#![allow(missing_docs)]` remain -- the blanket suppression was successfully removed from main.rs.
- `crates/roko-cli/src/lib.rs` lines 6-16: still has the blanket suppression:
  ```rust
  #![allow(dead_code, unused_imports, unused_variables)]
  #![allow(clippy::module_name_repetitions)]
  #![allow(missing_docs)]
  #![cfg_attr(clippy, allow(clippy::all, clippy::pedantic, clippy::nursery, clippy::restriction, missing_docs))]
  ```
- Since `main.rs` is a thin binary that delegates to `roko_cli` (the lib crate), the blanket suppression in lib.rs governs virtually all CLI code.

### Gap Analysis

| Spec Requirement | Status |
|---|---|
| Remove blanket `#![cfg_attr(clippy, allow(...))]` from main.rs | DONE |
| `cargo clippy -p roko-cli --no-deps -- -D warnings` passes clean | NOT MET -- lib.rs suppression hides all warnings |

The spec's `touches` list only included `main.rs`, but the intent was to make clippy actionable for the roko-cli crate. The lib.rs suppression defeats that.

### Completion Design

**File: `crates/roko-cli/src/lib.rs`**

1. Remove the blanket `#![cfg_attr(clippy, allow(clippy::all, clippy::pedantic, clippy::nursery, clippy::restriction, missing_docs))]` attribute (lines ~13-16).

2. Keep the targeted allows that are legitimate:
   - `#![allow(dead_code, unused_imports, unused_variables)]` -- these may need to stay during migration; evaluate after removal
   - `#![allow(clippy::module_name_repetitions)]` -- acceptable crate-level exception
   - `#![allow(missing_docs)]` -- acceptable per the W8-A source spec

3. Run `cargo clippy -p roko-cli --no-deps -- -D warnings` and fix warnings iteratively:
   - Start with mechanical fixes (redundant clones, unnecessary borrows, etc.)
   - For intentional patterns, add `#[allow(clippy::specific_lint)]` with a reason comment on the specific item
   - Do NOT add new blanket suppressions

4. Warning: removing `clippy::all` will likely surface hundreds of warnings across all lib.rs modules. The agent should batch-fix by lint category, not file-by-file.

**Note**: The `#![allow(dead_code, unused_imports, unused_variables)]` suppression also hides real issues. Once the blanket clippy suppression is removed, evaluate whether these can be narrowed or removed. They are separate from the clippy suppression removal.

**Tests**: `cargo clippy -p roko-cli --no-deps -- -D warnings` must pass clean.

---

## Task 017: JSONL Rotation Configuration

**Priority**: 5/10 (correctness -- rotation works but is not configurable as spec requires)
**Effort**: S (concentrated in 4 files, well-scoped)

### Current State

- `crates/roko-learn/src/jsonl_rotation.rs`: rotation logic exists with hardcoded constants:
  - `DEFAULT_ROTATION_THRESHOLD_BYTES = 10 * 1024 * 1024` (10 MB)
  - `MAX_ROTATED_FILES = 5`
  - `rotate_if_needed(path, threshold_bytes)` -- takes threshold but hardcodes max files to 5.
- `crates/roko-learn/src/episode_logger.rs`: calls `rotate_if_needed()` before append.
- `crates/roko-learn/src/runtime_feedback.rs`: `append_jsonl_record()` calls `rotate_if_needed()`.
- `crates/roko-core/src/config/learning.rs`: `LearningConfig` has NO `rotation_threshold_bytes` or `rotation_max_files` fields (confirmed by grep).
- `crates/roko-cli/src/config.rs`: `LearningLayer` (lines 1009-1022) has NO rotation config fields.

### Gap Analysis

| Spec Requirement | Status |
|---|---|
| Add `rotation_threshold_bytes` to `LearningConfig` | NOT DONE |
| Add `rotation_max_files` to `LearningConfig` | NOT DONE |
| Thread settings through `EpisodeLogger` | NOT DONE |
| Thread settings through `runtime_feedback::append_jsonl_record()` | NOT DONE |
| Add `rotate_if_needed_with_config()` or equivalent | NOT DONE |
| Add matching fields to `LearningLayer` with merge | NOT DONE |
| Add config parsing test | NOT DONE |
| Add small-threshold rotation test | NOT DONE |

The entire spec deliverable (configurability) is unimplemented. Rotation works, but only with hardcoded defaults.

### Completion Design

**File: `crates/roko-learn/src/jsonl_rotation.rs`**

1. Add a config struct and a new rotation function:
   ```rust
   pub struct JsonlRotationConfig {
       pub threshold_bytes: u64,
       pub max_files: usize,
   }

   impl Default for JsonlRotationConfig {
       fn default() -> Self {
           Self {
               threshold_bytes: DEFAULT_ROTATION_THRESHOLD_BYTES,
               max_files: MAX_ROTATED_FILES,
           }
       }
   }

   pub fn rotate_if_needed_with_config(path: &Path, config: &JsonlRotationConfig) -> io::Result<bool>
   ```
   Keep existing `rotate_if_needed(path, threshold_bytes)` as a wrapper calling the new function with `MAX_ROTATED_FILES`.

**File: `crates/roko-core/src/config/learning.rs`**

2. Add fields to `LearningConfig`:
   ```rust
   #[serde(default = "default_rotation_threshold")]
   pub rotation_threshold_bytes: u64,
   #[serde(default = "default_rotation_max_files")]
   pub rotation_max_files: usize,
   ```
   With defaults matching current constants (10MB, 5).

**File: `crates/roko-cli/src/config.rs`**

3. Add to `LearningLayer`:
   ```rust
   #[serde(default, skip_serializing_if = "Option::is_none")]
   pub rotation_threshold_bytes: Option<u64>,
   #[serde(default, skip_serializing_if = "Option::is_none")]
   pub rotation_max_files: Option<usize>,
   ```
   Update `LearningLayer::merge()` to include these fields.

**File: `crates/roko-learn/src/episode_logger.rs`**

4. Add a constructor that accepts rotation config:
   ```rust
   pub fn with_rotation(path: PathBuf, config: JsonlRotationConfig) -> Self
   ```
   Use `config.threshold_bytes` and `config.max_files` in the `append()` call.

**File: `crates/roko-learn/src/runtime_feedback.rs`**

5. Thread `JsonlRotationConfig` through `append_jsonl_record()` or accept threshold+max_files parameters.

**Tests to add**:
- In `jsonl_rotation.rs`: test `rotate_if_needed_with_config` with `max_files=2` to verify capping.
- In `episode_logger.rs`: test `with_rotation()` using tiny threshold, verify `.jsonl.1` created.
- In `learning.rs`: test parsing `[learning] rotation_threshold_bytes = 1024` and `rotation_max_files = 3`.
- In `config.rs`: test `LearningLayer` merge propagates rotation fields.

---

## Task 020: IDE/ACP Command Categories + bare_mode Filtering

**Priority**: 4/10 (user-facing bug -- bare mode exposes too many commands)
**Effort**: S (single function change + 1 test)

### Current State

- `crates/roko-acp/src/types.rs:749-751`: `SlashCommand` has `pub category: Option<String>` field.
- `crates/roko-acp/src/session.rs:1343-1357`: `slash_command()` helper correctly assigns category to every command.
- `crates/roko-acp/src/session.rs:1359-1364`: `bare_mode_allows_category()` allows 6 categories:
  ```rust
  matches!(category, "system" | "research" | "implementation" | "verification" | "workflow" | "help")
  ```
- `crates/roko-acp/src/session.rs:1370-1651`: `build_slash_commands(bare_mode)` builds ~47 commands, then filters in bare mode.
- `crates/roko-acp/src/handler.rs:316-318,422-424`: `bare_mode` is correctly threaded from config to `send_slash_commands_notification()`.

### Gap Analysis

| Spec Requirement | Status |
|---|---|
| `SlashCommand` has `category` field | DONE |
| Every command has a category | DONE (via `slash_command()` helper) |
| `bare_mode` filtering in `build_slash_commands()` | DONE -- but filter is too broad |
| Bare mode returns exactly 8 commands | NOT MET -- returns ~25+ commands |

The filter allows `"implementation"`, `"verification"`, and `"workflow"` categories in bare mode. These expose commands like `run`, `plan-run`, `review-this`, `pipeline`, `gate-*`, `build`, `test`, `lint`, etc. The spec requires exactly 8 commands: `status`, `doctor`, `config`, `help`, `research`, `search`, `enhance-prd`, `analyze`.

Additionally, `enhance-prd` is categorized as `"specification"` (session.rs line 1404), which is NOT in the allow-list, so it would be hidden despite being one of the required 8.

### Completion Design

**File: `crates/roko-acp/src/session.rs`**

1. Change `bare_mode_allows_category()` to use a name-based whitelist instead of category-based:
   ```rust
   fn bare_mode_allowed_commands() -> &'static [&'static str] {
       &["status", "doctor", "config", "help", "research", "search", "enhance-prd", "analyze"]
   }
   ```

2. Update the filter in `build_slash_commands()` (line 1638-1647):
   ```rust
   if bare_mode {
       let allowed = bare_mode_allowed_commands();
       commands.into_iter()
           .filter(|cmd| allowed.contains(&cmd.name.as_str()))
           .collect()
   } else {
       commands
   }
   ```
   Or keep the category-based approach but restrict to only `"system"` and `"research"` and `"help"`, AND fix `enhance-prd`'s category from `"specification"` to `"research"`.

3. Either approach requires fixing `enhance-prd`'s category to be included. Simplest: change it from `"specification"` to `"research"` (line 1404), since it IS a research-adjacent command.

**Tests to add/update in `crates/roko-acp/src/session.rs`**:
- `bare_mode_returns_exact_8_commands`: assert `build_slash_commands(true).len() == 8` and check all names.
- `bare_mode_includes_enhance_prd`: verify `enhance-prd` is in the bare mode set.
- `full_mode_returns_all_commands`: verify `build_slash_commands(false).len() > 8`.

---

## Task 021: Demo Scenario Redesign

**Priority**: 8/10 (frontend demo -- lower priority than core runtime)
**Effort**: M (TypeScript changes across ~8 files)

### Current State

- `demo/demo-app/src/lib/scenario-runners/index.ts`: exports 5 scenarios (cost, pipeline, memory, isfr, oracle). Old runners archived under `scenario-runners/archive/`.
- Individual runners exist: `cost.ts`, `pipeline.ts`, `memory.ts`, `isfr.ts`, `oracle.ts`.
- `demo/demo-app/src/pages/Demo/ScenarioSlot.tsx`: scenario layout component (needs verification for stale references).

### Gap Analysis

| Spec Requirement | Status |
|---|---|
| 5 scenarios exported (Cost, Pipeline, Memory, ISFR, Oracle) | DONE |
| Old scenarios archived | DONE |
| Cost runner: naive vs cascade split | DONE |
| Pipeline runner: single `roko do` | DONE |
| Memory runner: cold vs warm with workspace transfer | NOT DONE -- no explicit transfer step |
| ISFR runner: chain-health diagnostic step | NOT DONE -- no diagnostic step |
| ScenarioSlot.tsx: scenario-specific panels | PARTIALLY -- may still route to generic sidebar |
| ScenarioSlot.tsx: no stale prd-pipeline references | NOT VERIFIED |
| e2e tests updated | NOT VERIFIED |

### Completion Design

**File: `demo/demo-app/src/lib/scenario-runners/memory.ts`**

1. Add an explicit workspace transfer step between cold and warm runs:
   - After cold run completes, add a step that calls `roko knowledge sync` or equivalent knowledge transfer command
   - Label the step clearly as "Knowledge Transfer: cold -> warm"
   - The warm run should then demonstrate improved efficiency from transferred knowledge

**File: `demo/demo-app/src/lib/scenario-runners/isfr.ts`**

2. Add a chain-health diagnostic step:
   - Before or alongside agent collaboration panes, run `roko doctor` or `roko status --chain` to show chain health
   - This should be pane 0 or a pre-step before the 4 collaboration panes

**File: `demo/demo-app/src/pages/Demo/ScenarioSlot.tsx`**

3. Remove any stale `prd-pipeline` references in routing code:
   - Search for `prd-pipeline`, `knowledge-transfer`, `chain-intelligence`, `isfr-agents` string literals
   - Replace with scenario-id-based routing: `switch(scenario.id) { case 'cost': ...; case 'pipeline': ... }`

4. Add scenario-specific sidebar panels:
   - Cost: `CostComparisonPanel` (or equivalent)
   - Pipeline: existing `PipelineStagesPanel`
   - Memory: `MemoryTransferPanel` showing transfer efficiency
   - ISFR: chain health panel
   - Oracle: `OracleFlowPanel` with data/synthesis panes

**File: `demo/demo-app/e2e/demo-all-scenarios.spec.ts`**

5. Update e2e test to expect exactly 5 scenarios with correct names and pane counts.

**Tests**: `cd demo/demo-app && npx tsc --noEmit && npm run build`

---

## Task 026: TopicFilter And/Or/Not Combinators

**Priority**: 9/10 (code is correct, only missing tests)
**Effort**: S (test-only additions)

### Current State

- `crates/roko-core/src/pulse.rs`: `TopicFilter` enum has all 6 variants: `Exact`, `Prefix`, `All`, `And(Vec<TopicFilter>)`, `Or(Vec<TopicFilter>)`, `Not(Box<TopicFilter>)` (lines 197-209).
- `matches()` method correctly handles all variants (confirmed by existing unit tests).
- Unit tests in `pulse.rs` (lines 442-510): `filter_and_with_mixed_filters`, `filter_or_with_disjoint_filters`, `filter_not_inverts_exact_match`, `filter_nested_combinators`, `filter_and_empty_is_vacuous_truth`, `filter_or_empty_is_false` -- all pass.
- `filter_serde_roundtrip` test (line 430) only covers `Exact`, `Prefix`, `All` -- NOT the new combinator variants.
- `crates/roko-core/tests/property_tests.rs` (lines 90-104): only has `filter_all_matches_everything` and `filter_exact_matches_same_topic` -- no combinator property tests.
- `TopicFilter::And/Or/Not` is only used in test code within `pulse.rs` -- no bus-level integration tests.

### Gap Analysis

| Spec Requirement | Status |
|---|---|
| Add And/Or/Not variants to TopicFilter | DONE |
| Extend matches() method | DONE |
| Unit tests for combinators | DONE |
| Serde roundtrip test for new variants | NOT DONE |
| Bus-level integration test with combinators | NOT DONE |
| Property-based algebraic law tests | NOT DONE |

### Completion Design

**File: `crates/roko-core/src/pulse.rs`** -- tests section

1. Extend `filter_serde_roundtrip` (line 430) to include combinator variants:
   ```rust
   TopicFilter::And(vec![TopicFilter::Prefix("gate.".into()), TopicFilter::Exact(Topic::new("gate.verdict"))]),
   TopicFilter::Or(vec![TopicFilter::Exact(Topic::new("a")), TopicFilter::Exact(Topic::new("b"))]),
   TopicFilter::Not(Box::new(TopicFilter::Exact(Topic::new("x")))),
   ```

**File: `crates/roko-core/src/pulse_bus.rs`** or `crates/roko-core/src/bus_backends.rs`** -- tests section

2. Add bus-level integration test:
   ```rust
   #[tokio::test]
   async fn replay_from_with_and_combinator() {
       // Publish gate.compile, gate.heartbeat, episode.logged
       // Replay with And([Prefix("gate."), Not(Exact("gate.heartbeat"))])
       // Assert only gate.compile is returned
   }
   ```

3. Add bus-level test with Or combinator:
   ```rust
   #[tokio::test]
   async fn publish_with_or_combinator() {
       // Subscribe with Or([Exact("gate.compile"), Exact("gate.test")])
       // Publish gate.compile, gate.lint, gate.test
       // Assert only gate.compile and gate.test are received
   }
   ```

**File: `crates/roko-core/tests/property_tests.rs`**

4. Add algebraic property tests:
   ```rust
   proptest! {
       #[test]
       fn not_exact_never_matches_same_topic(s in "[a-z]{1,16}") {
           let topic = Topic::new(&s);
           let filter = TopicFilter::Not(Box::new(TopicFilter::Exact(topic.clone())));
           prop_assert!(!filter.matches(&topic));
       }

       #[test]
       fn and_all_with_exact_matches_topic(s in "[a-z]{1,16}") {
           let topic = Topic::new(&s);
           let filter = TopicFilter::And(vec![TopicFilter::All, TopicFilter::Exact(topic.clone())]);
           prop_assert!(filter.matches(&topic));
       }

       #[test]
       fn or_exact_with_impossible_matches_topic(s in "[a-z]{1,16}") {
           let topic = Topic::new(&s);
           let filter = TopicFilter::Or(vec![
               TopicFilter::Exact(topic.clone()),
               TopicFilter::Prefix("impossible_prefix_zzz.".into()),
           ]);
           prop_assert!(filter.matches(&topic));
       }
   }
   ```

---

## Task 030: Tag Floating Code

**Priority**: 10/10 (documentation-only, lowest impact -- but has an accuracy bug)
**Effort**: S (single file fix + re-audit)

### Current State

- All 15 expected modules have `//! STATUS: NOT WIRED` tags (confirmed by grep):
  - `roko-runtime`: theta_consumer, delta_consumer, demurrage_consumer, energy, heartbeat_attention, heartbeat_probes, task_scheduler (7 files)
  - `roko-learn`: calibration_policy, bayesian_confidence, quality_judge, error_enrichment, event_subscriber, verdict_scorer, active_inference, oracles/mod (8 files)
- Wording matches spec: "built but no non-test runtime caller" or "called internally by floating code but no runtime entrypoint".

### Gap Analysis

| Spec Requirement | Status |
|---|---|
| Re-audit each module before tagging | DONE |
| Add STATUS comment to floating modules | DONE |
| Use correct floating-code characterization | PARTIALLY -- active_inference.rs is inaccurate |
| Do not tag wired modules (baseline, jsonl_rotation, etc.) | DONE |
| Do not tag lang-* crates | DONE |

The issue: `crates/roko-learn/src/active_inference.rs` line 1 says:
```
//! STATUS: NOT WIRED -- called internally by floating code but no runtime entrypoint.
```

But `active_inference` IS imported by `cascade_router.rs` (line 37: `use crate::active_inference::{BeliefState, select_tier as select_tier_with_belief}`), and `CascadeRouter` is wired code -- it's used in `orchestrate.rs`. The import is via `select_tier_with_active_inference()` method on CascadeRouter (line 393), which is a dead method on a live struct. This is a "dead method on wired struct" problem, not "called internally by floating code."

### Completion Design

**File: `crates/roko-learn/src/active_inference.rs`**

1. Change the status tag from:
   ```rust
   //! STATUS: NOT WIRED -- called internally by floating code but no runtime entrypoint.
   ```
   to:
   ```rust
   //! STATUS: NOT WIRED -- imported by CascadeRouter (wired) but only via dead method
   //! `select_tier_with_active_inference()`. No runtime caller invokes that method.
   ```

2. Alternatively, if `select_tier_with_active_inference()` on `CascadeRouter` is never called, it could be tagged `#[allow(dead_code)]` with a comment pointing to this task, or the method itself could get a doc comment explaining its status.

**Verification**: Re-run `rg 'select_tier_with_active_inference' crates/ -g '*.rs'` to confirm no runtime callers exist beyond the definition and the import.

---

## Task 034: SectionOutcome Recording

**Priority**: 1/10 (critical for learning loop correctness -- bandit IDs must be stable)
**Effort**: M (3 specific fixes in event_loop.rs, already wired)

### Current State

- `crates/roko-cli/src/runner/event_loop.rs`: SectionOutcome recording IS wired (lines 980-1008).
  - Imports: `SectionOutcomeStore`, `SectionOutcomeRecord`, `SectionOutcomeStatus`, `SECTION_OUTCOME_SCHEMA_VERSION` (lines 46-47).
  - `section_diagnostics` HashMap captures prompt diagnostics at dispatch time (line 3144-3150), keyed by `format!("{plan_id}:{task_id}:{attempt_num}")`.
  - `build_section_outcome_records()` (lines 3761-3844) constructs records.
  - `append_section_outcomes()` (lines 3848-3856) persists to JSONL.
- `crates/roko-cli/src/runner/persist.rs:587`: `section_outcomes_path()` helper exists with test (line 697).

### Gap Analysis

| Spec Requirement | Status |
|---|---|
| Identify task completion point | DONE -- lines 980-1008 |
| Record section outcomes after completion | DONE |
| Thread prompt section IDs via diagnostics | DONE -- `section_diagnostics` HashMap |
| Persistence helper in persist.rs | DONE |
| `section_id` uses `prompt:<normalized-name>` format | NOT DONE -- uses raw section name (line 3798) |
| `invocation_id` uses `run_id:attempt_key` | NOT DONE -- uses `plan_id:task_id` (line 3792), collides across retries |
| Model captured at dispatch time | PARTIAL -- reads `state.agent_model` at gate completion (line 996), may be stale |
| Stable IDs for bandit consumption | NOT MET -- wrong ID formats |

### Completion Design

**File: `crates/roko-cli/src/runner/event_loop.rs`**

1. **Fix `section_id` format** in `build_section_outcome_records()` (lines 3798, 3826):
   - Change: `section_id: section_name.clone(),`
   - To: `section_id: format!("prompt:{}", normalize_section_name(section_name)),`
   - Add helper:
     ```rust
     fn normalize_section_name(name: &str) -> String {
         name.to_ascii_lowercase()
            .replace(|c: char| !c.is_ascii_alphanumeric(), "-")
            .trim_matches('-')
            .to_string()
            // collapse consecutive dashes
     }
     ```

2. **Fix `invocation_id` format** (lines 3792, 3820):
   - Change: `invocation_id: format!("{plan_id}:{task_id}"),`
   - To: `invocation_id: format!("{}:{}", run_id, attempt_key),`
   - Pass `run_id: &str` and `attempt_key: &str` into `build_section_outcome_records()`.
   - At the call site (line 993-1001), pass `state.run_id()` and `completion_attempt.key()`.

3. **Fix model/provider capture at dispatch time**:
   - Extend the struct stored in `section_diagnostics` (currently just `PromptDiagnostics`) to include model and provider strings.
   - At the dispatch site (line 3144-3150), also store the selected model slug and provider label.
   - At gate completion (line 996-998), read model/provider from the stashed context instead of `state.agent_model` / `state.agent_provider`.
   - This may require changing `section_diagnostics` from `HashMap<String, PromptDiagnostics>` to `HashMap<String, PromptOutcomeContext>` where:
     ```rust
     struct PromptOutcomeContext {
         diagnostics: PromptDiagnostics,
         model: String,
         provider: String,
     }
     ```

4. **Update `build_section_outcome_records()` signature** to accept `run_id` and `attempt_key`:
   ```rust
   fn build_section_outcome_records(
       plan_id: &str,
       task_id: &str,
       run_id: &str,
       attempt_key: &str,
       model: &str,
       provider: &str,
       status: SectionOutcomeStatus,
       diag: &PromptDiagnostics,
       verdicts: &[GateVerdictSummary],
   ) -> Vec<SectionOutcomeRecord>
   ```

**Tests to add**:
- Unit test for `normalize_section_name()`: "System Prompt" -> "system-prompt", "knowledge_context" -> "knowledge-context".
- Unit test for `build_section_outcome_records()`: verify `section_id` starts with `prompt:`, `invocation_id` contains run_id and attempt_key, and different attempt_nums produce different invocation_ids.

---

## Task 035: Cell execute() Trait

**Priority**: 6/10 (test compilation bug, but trait itself is correct and all 10 tests pass)
**Effort**: S (the test bug described in the original gap was WRONG -- tests pass clean)

### Current State

- `crates/roko-core/src/cell.rs` (133 lines): `CellContext`, `TypeSchema`, and `Cell` trait with `#[async_trait]` and `async fn execute()` default method -- all correctly implemented per spec.
- `crates/roko-core/src/lib.rs:178`: `pub use cell::*;` exports everything.
- `crates/roko-core/src/lib.rs:303`: `Substrate` IS exported: `pub use traits::{..., Substrate, ...};`
- `crates/roko-core/src/traits.rs:428`: `pub trait Substrate: Store {}` -- it's a supertrait alias.
- `crates/roko-core/tests/cell_execute.rs` (236 lines): 10 tests covering:
  - Echo cell execute (custom impl)
  - Default cell execute (error return)
  - Empty input execute
  - TypeSchema compatibility (5 tests)
  - Cell metadata accessors (2 tests)
- **All 10 tests pass**: `cargo test -p roko-core --test cell_execute` -- confirmed: `10 passed; 0 failed`.

### Gap Analysis

The original NEEDS_WORK entry stated: "imports `roko_core::Substrate` which is NOT in the crate's public API." This was **incorrect**. `Substrate` IS exported at `crates/roko-core/src/lib.rs:303` and the test compiles and passes.

| Spec Requirement | Status |
|---|---|
| Add CellContext struct | DONE -- lines 25-57 |
| Add TypeSchema enum | DONE -- lines 62-83 |
| Add execute() default method on Cell | DONE -- lines 126-132 |
| #[async_trait] on Cell trait | DONE -- line 90 |
| Export from lib.rs | DONE -- line 178 (`pub use cell::*`) |
| Integration test that compiles and passes | DONE -- 10 tests, all pass |
| Existing Cell impls still compile | DONE -- workspace builds |

### Re-assessment

This task appears to be COMPLETE. The original gap report was based on stale information. The `Substrate` type IS publicly exported, the test compiles, and all 10 tests pass. Recommend re-categorizing as DONE.

If there is any remaining concern, verify with:
```bash
cargo test -p roko-core --test cell_execute
rg "pub use.*Substrate" crates/roko-core/src/lib.rs
```

---

## Task 038: Signal Rename Propagation

**Priority**: 10/10 (functionally complete for its scope; remaining work is cosmetic)
**Effort**: L (822 occurrences across 107 files for the full rename, but task scope is done)

### Current State

- The 5 target crates (roko-agent, roko-gate, roko-learn, roko-compose, roko-orchestrator) have **zero** remaining `Engram` references. `grep -rn 'Engram' crates/roko-agent/src/` returns nothing. The rename IS complete in the target scope.
- 822 `Engram` occurrences persist across 107 other crate files, primarily in:
  - `roko-core` (55 in engram.rs, 27 in traits.rs, 21 in datum.rs, 16 in prediction.rs, 16 in attestation.rs, 13 in pulse.rs)
  - `roko-cli` (49 in orchestrate.rs, 37 in scaffold.rs, 17 in run.rs, 15 in tui/verdicts.rs, 13 in episode.rs)
  - `roko-serve` (43 in dispatch.rs, 13 in webhooks.rs)
  - `roko-conductor` (29 in conductor.rs + watchers)
  - `roko-std`, `roko-fs`, `roko-chain`, `roko-dreams`, `roko-plugin`, `roko-acp`
- Core alias: `crates/roko-core/src/engram.rs` still has `pub type Engram = Signal` and `pub type EngramBuilder = SignalBuilder`.

### Gap Analysis

Spec said: "Propagate Engram -> Signal rename to 5 core crates." Those 5 crates are clean -- task is technically done for its literal scope. The dependency order concern (propagation before core rename) is moot since the alias exists and the workspace compiles.

### Completion Design

This task is arguably **DONE** for its scope. What remains is a follow-up task for the other 107 files:
1. `roko-core/src/` (~170 refs) -- traits.rs, datum.rs, pulse.rs, prediction.rs, attestation.rs, etc.
2. `roko-cli/src/` (~180 refs) -- orchestrate.rs alone has 49
3. `roko-serve/src/` (~65 refs) -- dispatch.rs, routes/webhooks.rs
4. `roko-conductor/src/` (~55 refs) -- conductor.rs + all 10 watchers
5. `roko-std/src/` (~45 refs) -- noop.rs, router.rs, scorer.rs, memory.rs
6. `roko-fs/src/` (~30 refs) -- file_substrate.rs, cold_substrate.rs
7. `roko-chain/src/` (~40 refs) -- identity_economy, gate modules
8. `roko-dreams/src/` (~12 refs) -- cycle.rs, runner.rs
9. Tests/benches (~30 refs) -- benches/engram_bench.rs, tests/property_tests.rs, tests/cell_execute.rs

### Test Coverage Gaps

No tests needed for rename. Verification: `grep -rn 'Engram' crates/ | grep -v 'type Engram = Signal' | wc -l` reaching 0 and `cargo test --workspace` passing.

### Effort: 4-6 hours (follow-up task for remaining 107 files). Priority: Low.

---

## Task 050: Silent Error Swallowing

**Priority**: 2/10 (silent failures hide production issues)
**Effort**: S-M (targeted fixes in 5 files, clear patterns)

### Current State

- 7 serve route files and 2 provider files received `warn!` logging additions. The work done is correct.
- **`crates/roko-serve/src/routes/gateway.rs:975`**: `if let Ok(bandit) = UcbBandit::load(&bandit_path, candidate_slugs)` -- still silent. No `else` branch, no log. Bandit state corruption is invisible to operators.
- **`crates/roko-serve/src/routes/vision_loop.rs:242`**: `let _ = child.kill().await;` -- still unlogged in the `cancel_vision_loop()` handler. If the kill fails, the subprocess leaks without trace.
- **`crates/roko-agent/src/provider/anthropic_api/tool_loop.rs:340`**: `let _ = crate::translate::claude::inject_cache_markers_into_content(&mut system);` -- still silent. Cache marker injection failure means users lose prompt caching without knowing why.
- **`crates/roko-serve/src/routes/deployments.rs`**: `if let Ok(key) = std::env::var("ANTHROPIC_API_KEY")` at line 115 is an env-var lookup (intentional absence OK), but the actual deployment persistence write path (create_dir, serialize, write, rename) needs audit for silent failures.
- **`crates/roko-serve/src/routes/plans.rs`**: Pause snapshot write returning 200 OK on failure -- needs to return error HTTP status.

### Gap Analysis

| Spec Target | Status |
|---|---|
| `routes/deployments.rs` persistence errors | **Not done** |
| `routes/gateway.rs:975` bandit load fallback | **Not done** -- `if let Ok` without else |
| `routes/vision_loop.rs:242` child kill | **Not done** -- `let _ =` without log |
| `anthropic_api/tool_loop.rs:340` cache markers | **Not done** -- `let _ =` without log |
| `routes/plans.rs` pause snapshot returns 200 on failure | **Not done** |

### Completion Design

**File: `crates/roko-serve/src/routes/gateway.rs` (line 975)**
```rust
// BEFORE:
if let Ok(bandit) = UcbBandit::load(&bandit_path, candidate_slugs) {
// AFTER:
match UcbBandit::load(&bandit_path, candidate_slugs) {
    Ok(bandit) => { /* existing usage block */ }
    Err(e) => {
        tracing::warn!(path = %bandit_path.display(), error = %e,
            "failed to load bandit state, using defaults");
    }
}
```

**File: `crates/roko-serve/src/routes/vision_loop.rs` (line 242)**
```rust
// BEFORE:
let _ = child.kill().await;
// AFTER:
if let Err(e) = child.kill().await {
    tracing::warn!(run_id = %run_id, error = %e, "failed to kill vision loop child process");
}
```

**File: `crates/roko-agent/src/provider/anthropic_api/tool_loop.rs` (line 340)**
```rust
// BEFORE:
let _ = crate::translate::claude::inject_cache_markers_into_content(&mut system);
// AFTER:
if let Err(e) = crate::translate::claude::inject_cache_markers_into_content(&mut system) {
    tracing::warn!(error = %e, "failed to inject cache markers into system prompt");
}
```

**File: `crates/roko-serve/src/routes/plans.rs`** -- find the pause-snapshot write path. If snapshot write fails, return `ApiError::internal(format!("snapshot write failed: {e}"))` instead of `Ok(Json(...))`.

**File: `crates/roko-serve/src/routes/deployments.rs`** -- audit `create_dir_all`, `serde_json::to_string_pretty`, `fs::write`, `fs::rename` calls in deployment persistence. Each failure needs `warn!` with deployment ID and path context.

### Integration Wiring

No new wiring needed. These are in-place error handling upgrades.

### Test Coverage Gaps

- `routes/plans.rs`: Test that pause request with a read-only state directory returns non-200 status
- `routes/gateway.rs`: Test that a corrupt bandit JSON file does not crash the gateway route (returns default behavior instead)
- No log-assertion tests needed (spec explicitly says don't assert on tracing output)

### Effort: 2-3 hours. Priority: High.

---

## Task 057: roko do Command

**Priority**: 4/10 (user-facing flag semantics, but no current workflows prompt for approval)
**Effort**: S (Option A) or M (Option B, adding actual approval prompts)

### Current State

- **File**: `crates/roko-cli/src/commands/do_cmd.rs` (1058 lines including 184 lines of tests)
- **`cmd_do()`** (line 14): Accepts `yes: bool` parameter. The `yes` value is used at line 61 in `print_do_preview()` to display `"approval: auto"` vs `"workflow"` in dry-run output.
- **The `yes` value is never forwarded** to `run_simple_path()` (line 74), `run_standard_path()` (line 77), or `run_complex_path()` (line 80). None of these function signatures accept `yes`.
- **`run_simple_path()`** (line 87): Routes to `run_workflow_engine_report_with_hub()` -- no approval parameter exists in this API.
- **`run_standard_path()`** (line 139): Generates plan and executes it. Pre-flight provider check exists (line 163-171) but no interactive approval prompt.
- **`run_complex_path()`** (line 240): Creates PRD idea, drafts PRD, generates plan, executes plan. No interactive approval prompts at any step.
- **Existing tests** (lines 874-1058): 22 unit tests covering complexity labels, workflow templates, pipeline descriptions, cost ranges, promote logic, and truncation. No test for `--yes` behavior.

### Gap Analysis

| Spec Requirement | Status |
|---|---|
| `--yes` accepted on CLI | DONE |
| `--yes` shown in dry-run preview | DONE -- line 764 prints "approval: auto" |
| `--yes` bypasses approval prompts | **Not done** -- never passed to execution paths |
| Execution paths have approval prompts | **Not done** -- none of the 3 paths prompt |

This is a double gap: (1) the flag is inert, and (2) no workflows prompt for confirmation. The fix requires both adding prompts AND threading the bypass.

### Completion Design

**Option A (minimal -- document the no-op and suppress the warning):**
```rust
// In cmd_do(), at line 71, add:
let _ = yes; // NOTE: unused -- no execution path currently prompts for approval.
             // When approval gates are added, thread `yes` to bypass them.
```
Add a test: `assert_eq!(print_preview_with_yes, "approval    : auto")`.

**Option B (recommended -- add approval prompts to planned/complex paths):**

1. Add `yes: bool` to `run_standard_path()` and `run_complex_path()` signatures.

2. In `cmd_do()` (line 72-82), forward `yes`:
```rust
PlanComplexity::Standard => {
    run_standard_path(cli, &workdir, &prompt, no_cascade, provider, yes).await
}
PlanComplexity::Complex => {
    run_complex_path(cli, &workdir, &prompt, no_cascade, provider, yes).await
}
```

3. In `run_standard_path()`, after plan loading (after line 231):
```rust
if !yes && atty::is(atty::Stream::Stdin) {
    eprintln!("\u{25b8} Generated plan with {total_tasks} tasks. Execute? [y/N]");
    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;
    if !input.trim().eq_ignore_ascii_case("y") {
        eprintln!("\u{25b8} Aborted by user.");
        return Ok(EXIT_SUCCESS);
    }
}
```

4. In `run_complex_path()`, add similar prompt before plan execution (after line 400, before step 4).

5. If stdin is not a TTY (piped/CI), skip the prompt entirely (behave as `--yes`).

### Test Coverage Gaps

- Unit test: `--yes` flag parsed correctly from CLI (add `cli_parses_do_yes` test in main.rs)
- Dry-run test: `--yes` appears in preview output as "approval: auto" (add test)
- If Option B: test that `yes=true` skips the prompt (mock stdin or test the condition)

### Effort: 1 hour (Option A) or 3-4 hours (Option B). Priority: Medium.

---

## Task 058: roko show Command

**Priority**: 3/10 (core inspection command, user-facing)
**Effort**: M (concentrated in single file, clear requirements)

### Current State

- **File**: `crates/roko-cli/src/commands/show.rs`
- **`ShowSubject` enum** (lines 13-22): Has 7 variants: `Overview`, `Costs`, `Agents`, `Knowledge`, `Plans`, `Learning`, `History`. Missing: `Config`, `Health`, `Runs`.
- **`ShowTarget::parse()`** (lines 31-47): Maps strings to subjects. `"config"`, `"health"`, and `"runs"` fall through to `Self::WorkId(subject)`, meaning `roko show config` tries to look up a work item called "config" instead of showing configuration.
- **`cmd_show()`**: No branch checks for `cli.json`. All output is `println!`/`eprintln!` text. No JSON serialization path exists.
- **Existing topics work correctly**: `overview`, `costs`, `agents`, `knowledge`, `plans`, `learning`, `history` all render readable text from `DashboardData::load_best_effort()` and `.roko/` state files.
- **Data loading** (`ShowState`, line 49): loads from `RokoLayout`, `DashboardData`, `.roko/state/`, `.roko/learn/`, `.roko/neuro/knowledge.jsonl`.

### Gap Analysis

| Spec Requirement | Status |
|---|---|
| `roko show` (default overview) | Done |
| `roko show plans` | Done |
| `roko show learning` | Done (summarized, not raw JSONL) |
| `roko show runs` | **Missing** -- no `Runs` variant in enum |
| `roko show config` | **Missing** -- falls through to WorkId("config") |
| `roko show health` | **Missing** -- falls through to WorkId("health") |
| `--json` for every topic | **Missing** -- no JSON code path at all |
| Topic aliases | Done (`overview`=`summary`, `learning`=`router`, etc.) |

### Completion Design

**1. Extend `ShowSubject` enum** (line 13-22):
```rust
enum ShowSubject {
    Overview,
    Costs,
    Agents,
    Knowledge,
    Plans,
    Learning,
    History,
    Config,   // NEW
    Health,   // NEW
    Runs,     // NEW
}
```

**2. Extend `ShowTarget::parse()`** (add before the `_ =>` fallthrough at line 44):
```rust
"config" | "configuration" | "conf" => Self::Subject(ShowSubject::Config),
"health" | "providers" => Self::Subject(ShowSubject::Health),
"runs" | "executions" => Self::Subject(ShowSubject::Runs),
```
Note: `"history"` already maps to `History` (event log). Keep `runs` separate.

**3. Implement `render_config(state: &ShowState, workdir: &Path)`**:
- Load `RokoConfig` via `roko_core::config::loader::load_config_unified(workdir)`
- Print active providers with credential status: name, kind, `api_key_env` present/missing (via `std::env::var`)
- Print configured models with provider and role assignments
- Print gate config: `clippy_enabled`, `skip_tests`, max_gate_rung
- Print learning settings: `replan_on_gate_failure`, adaptive thresholds
- **Redact secrets**: show env var name + "present"/"missing", never the actual value

**4. Implement `render_health(state: &ShowState)`**:
- Per provider: read provider health from `DashboardData` if available
- Gate thresholds: read `.roko/learn/gate-thresholds.json`, print current EMA values per rung
- If data absent, print "unknown" instead of failing

**5. Implement `render_runs(state: &ShowState)`**:
- List `.roko/state/*.json` snapshot files sorted by mtime (most recent first)
- For each (limit 10): parse enough JSON to extract timestamp, plan count, task counts, pass/fail
- Print as a formatted table

**6. Add `--json` support** in `cmd_show()`:
```rust
if cli.json {
    let json_output = serde_json::json!({
        "topic": topic_name,
        "workspace": workdir.display().to_string(),
        "state_root": state.layout.root().display().to_string(),
        "generated_at": chrono::Utc::now().to_rfc3339(),
        "data": topic_specific_data,
    });
    println!("{}", serde_json::to_string_pretty(&json_output)?);
    return Ok(());
}
```
Branch on `cli.json` before each text render function. Reuse the same `ShowState` data for both text and JSON paths.

### Files to Change

- `crates/roko-cli/src/commands/show.rs` -- all changes concentrated here (enum, parse, 3 new render functions, JSON branch)

### Test Coverage Gaps

- Parse test: `ShowTarget::parse(Some("config".into()))` == `Subject(Config)` (and same for health, runs, conf, executions)
- Parse test: `ShowTarget::parse(Some("health".into()))` == `Subject(Health)`, not `WorkId("health")`
- JSON render test: verify output is valid JSON with `topic` and `data` fields
- Config render test: secrets are redacted (no actual env var values appear in output string)

### Effort: 4-5 hours. Priority: High.

---

## Task 062: IDE/ACP Provider Readiness Boolean

**Priority**: 7/10 (serde contract mismatch -- backward compatibility concern)
**Effort**: XS (single attribute change + remove unused helper)

### Current State

- **File**: `crates/roko-acp/src/types.rs`
- **Line 675**: `const fn default_true() -> bool { true }` -- helper function
- **Lines 700-702**: Current serde attribute on `ready` field:
  ```rust
  /// Whether this option can be used right now.
  #[serde(default = "default_true")]
  pub ready: bool,
  ```

### Gap Analysis

| Behavior | Current | Spec Target |
|---|---|---|
| `ready: true` serialization | `"ready": true` in JSON | `"ready": true` in JSON |
| `ready: false` serialization | `"ready": false` in JSON | **Omitted from JSON** |
| Missing `ready` deserialization | Defaults to `true` | Defaults to `false` |

The `default_true` also means an absent `ready` field deserializes as `true`. The spec target changes deserialization default to `false`. This is acceptable because:
1. ACP clients never send `ConfigOptionValue` back -- it is server-produced
2. All construction sites in `build_config_options()` set `ready` explicitly

### Completion Design

**File: `crates/roko-acp/src/types.rs`**

Change lines 700-702:
```rust
// BEFORE:
#[serde(default = "default_true")]
pub ready: bool,

// AFTER:
#[serde(default, skip_serializing_if = "std::ops::Not::not")]
pub ready: bool,
```

Check if `default_true()` (line 675) is still used elsewhere:
```bash
grep -n 'default_true' crates/roko-acp/src/ -r --include='*.rs'
```
If only used by `ready`, remove the `default_true` function.

Verify all `ConfigOptionValue { ... }` construction sites in `crates/roko-acp/src/session.rs` set `ready` explicitly:
```bash
grep -n 'ConfigOptionValue {' crates/roko-acp/src/session.rs
```
Each site must have `ready: true` (for always-available options like effort, temperament) or `ready: config.is_provider_available(...)` (for provider/model options).

### Test Coverage Gaps

Add to `crates/roko-acp/src/types.rs` tests:
- `serde_ready_false_omitted`: serialize `ConfigOptionValue { ready: false, .. }`, assert JSON string does NOT contain `"ready"`
- `serde_ready_true_present`: serialize `ConfigOptionValue { ready: true, .. }`, assert JSON contains `"ready": true`
- `serde_ready_absent_defaults_false`: deserialize `{"value":"x","name":"Y"}` (no ready field), assert `ready == false`

### Effort: 30 minutes. Priority: Medium.

---

## Task 074: Claude CLI Provider Completeness

**Priority**: 2/10 (wrong tool_format affects model routing on certain code paths)
**Effort**: S (config file edits + verification)

### Current State

- **File**: `roko.toml` -- ALL 35 model entries use `tool_format = "openai_json"`, including Claude/Anthropic models
- **Claude-specific models confirmed with wrong format**:
  - `haiku` (line 123, provider=anthropic)
  - `claude-opus` (line 145, provider=anthropic)
  - `sonnet` (line ~167, provider=anthropic)
  - `claude-sonnet` (line ~191, provider=anthropic)
  - `opus` (line ~213, provider=anthropic or claude_cli)
- **Test helper confirms correct value**: `crates/roko-agent/src/provider/claude_cli.rs` -- `claude_model()` uses `tool_format: "anthropic_blocks"`
- **Impact**: `translator_for_profile()` in `crates/roko-agent/src/translate/mod.rs` matches on `tool_format` to select translator. With `"openai_json"`, it selects the OpenAI translator for Claude models -- wrong format for tool use blocks.

### Gap Analysis

The task spec had 3 sub-tasks. Only sub-task 3 (config file fixes) was clearly not done. Sub-tasks 1-2 (StreamJson usage/finish_reason extraction) need verification:

```bash
grep -A 5 'Self::StreamJson' crates/roko-agent/src/translate/mod.rs | head -20
```

If `extract_usage()` for `StreamJson` still returns `Usage::default()` and `extract_finish_reason_raw()` still returns `None`, those sub-tasks are also incomplete.

### Completion Design

**File: `roko.toml`** -- Identify Claude/Anthropic models and fix their `tool_format`:

First, identify which providers are Claude-based:
```bash
grep -B 1 'kind = "claude_cli"\|kind = "anthropic_api"' roko.toml
```

For every `[models.*]` section whose `provider` references a Claude/Anthropic provider, change:
```toml
# BEFORE:
tool_format = "openai_json"
# AFTER:
tool_format = "anthropic_blocks"
```

**Models to change** (provider=anthropic or provider=claude_cli):
haiku, claude-opus, sonnet, claude-sonnet, opus

**Models to leave as `"openai_json"`** (non-Claude):
o3-mini, gpt54 (openai), gemini-* (gemini), sonar-* (perplexity), llama32/gemma4 (ollama), cerebras-* (cerebras), glm-*/kimi-* (zai/zhipu/moonshot)

**File: `docker/railway.roko.toml`** -- apply same fix if file exists. Check first:
```bash
test -f docker/railway.roko.toml && echo exists || echo missing
```

**Sub-tasks 1-2 verification**: If `extract_usage()` and `extract_finish_reason_raw()` for `StreamJson` are still stubs, implement per the spec's code blocks in sections 1-2 of the task spec at `/Users/will/dev/nunchi/roko/roko/tmp/taskrunner/tasks/074-claude-cli-provider-completeness.md`.

### Test Coverage Gaps

- Config audit: script or test that parses `roko.toml`, finds models with `provider = "anthropic"`, asserts `tool_format = "anthropic_blocks"`
- If StreamJson extraction was completed: verify tests `stream_json_extract_usage_from_result_event`, `stream_json_extract_finish_reason_end_turn`, etc. exist
- If StreamJson extraction NOT completed: add all 8 tests from spec section 5

### Effort: 1 hour (config only) or 3-4 hours (if StreamJson extraction also needed). Priority: High.

---

## Task 079: Magic Number Centralization

**Priority**: 9/10 (code quality -- constant defined but never used)
**Effort**: XS-S (wire existing constant to its intended replacement site)

### Current State

- **File**: `crates/roko-core/src/defaults.rs` (line 325): `pub const DEFAULT_RUNNER_RETRY_STRATEGY_PIVOT_ATTEMPT: u32 = 3;` -- defined, part of the 105 constants in this file.
- **Usage**: `grep -rn 'RETRY_STRATEGY_PIVOT' crates/ --include='*.rs'` returns ONLY the definition. Zero imports. Zero references anywhere in the workspace.
- **The raw literal**: The `3` this constant was meant to replace may have been removed, moved, or restructured by another task. The spec cites `snapshot_writer.rs:167` and `state.rs:563` but those line numbers may be stale.

### Gap Analysis

| Spec Requirement | Status |
|---|---|
| Audit remaining magic numbers | Partial -- candidates identified in spec |
| Add constants to defaults.rs | Partial -- at least one constant added but unused |
| Replace literals with constants | **Not done** for `RETRY_STRATEGY_PIVOT` |
| Sanity tests for constants | Unknown -- need to check |

### Completion Design

**Step 1: Find the raw literal this constant should replace**:
```bash
grep -rn '>= 3\|== 3\|< 3' crates/roko-cli/src/runner/ --include='*.rs' | grep -iE 'attempt|retry|pivot|iteration'
```
If the literal no longer exists (moved by another task), the constant is dead code and should either be removed or documented as "reserved for future use."

**Step 2: If the literal exists**, import and replace:
```rust
use roko_core::defaults::DEFAULT_RUNNER_RETRY_STRATEGY_PIVOT_ATTEMPT;
// Replace: if state.iteration_for(...) >= 3
// With:    if state.iteration_for(...) >= DEFAULT_RUNNER_RETRY_STRATEGY_PIVOT_ATTEMPT
```

**Step 3: Check for other orphaned constants** -- run:
```bash
for const in $(grep -oP 'pub const (DEFAULT_\w+)' crates/roko-core/src/defaults.rs | awk '{print $3}'); do
  count=$(grep -rn "$const" crates/ --include='*.rs' | grep -v 'defaults.rs' | wc -l)
  if [ "$count" -eq 0 ]; then echo "UNUSED: $const"; fi
done
```
Fix or document each orphaned constant.

**Step 4: Add sanity test** if not already present:
```rust
#[test]
fn runner_retry_pivot_is_positive() {
    assert!(DEFAULT_RUNNER_RETRY_STRATEGY_PIVOT_ATTEMPT >= 1);
    assert!(DEFAULT_RUNNER_RETRY_STRATEGY_PIVOT_ATTEMPT <= 10);
}
```

### Files to Change

- `crates/roko-core/src/defaults.rs` -- sanity test, possibly remove dead constants
- `crates/roko-cli/src/runner/state.rs` or equivalent -- import and use the constant (if literal exists)

### Test Coverage Gaps

- Sanity test for each constant (value constraints)
- No behavioral tests needed -- values are unchanged

### Effort: 1 hour. Priority: Low.

---

## Task 081: Error Type Hierarchy

**Priority**: 5/10 (library architecture -- types exist but serve no purpose without API migration)
**Effort**: M-L (mechanical signature updates require updating all callers)

### Current State

- **`crates/roko-agent/src/error.rs`** (exists): `pub enum AgentError { ... }` with variants for Creation, Backend, Provider, ToolDispatch, SafetyViolation, Other.
- **`crates/roko-agent/src/lib.rs` line 111**: `pub use error::AgentError;` -- exported.
- **External callers**: `grep -rn 'roko_agent::AgentError' crates/ --include='*.rs'` returns **zero results**. No crate imports or uses `AgentError`.
- **`crates/roko-gate/src/error.rs`**: needs verification -- check if GateError exists and whether the `GateError = RokoError` alias in `generated.rs` was removed.
- **`crates/roko-learn/src/error.rs`**: needs verification.
- **`crates/roko-compose/src/error.rs`**: needs verification.

### Gap Analysis

| Spec Requirement | Status |
|---|---|
| Create `roko-agent/src/error.rs` with `AgentError` | Done (exists, exported) |
| Create `roko-gate/src/error.rs` with `GateError` | **Needs verification** |
| Create `roko-learn/src/error.rs` with `LearnError` | **Needs verification** |
| Create `roko-compose/src/error.rs` with `ComposeError` | **Needs verification** |
| Remove `GateError = roko_core::RokoError` alias | **Needs verification** |
| Update public API signatures to return typed errors | **Not done** -- no function returns these types |
| External callers use typed errors | **Not done** -- zero callers |

The core problem: error types were created as shells with no function signatures updated to use them.

### Completion Design

**Phase 1: Verify all 4 error.rs files**:
```bash
for crate in roko-agent roko-gate roko-learn roko-compose; do
  test -f crates/$crate/src/error.rs && echo "$crate: exists" || echo "$crate: MISSING"
done
grep -n 'GateError = roko_core' crates/roko-gate/src/generated.rs  # should be gone
```

**Phase 2: Wire `GateError` into gate APIs** (if error.rs exists):
- Remove `pub type GateError = roko_core::RokoError` from `generated.rs`
- Update `GateGenerator::generate()` to return `Result<_, crate::error::GateError>`
- Update `lib.rs` re-export: `pub use error::GateError;` instead of `pub use generated::GateError;`

**Phase 3: Wire `LearnError` into learn APIs** (if error.rs exists):
- `CascadeRouter::save()` -- return `LearnError` instead of `std::io::Error`
- `CascadeRouter::from_snapshot_json()` -- return `LearnError`
- `EpisodeLogger::append()` -- keep existing `LoggerError` OR add `impl From<LoggerError> for LearnError`
- Update all callers of these functions (primarily in `roko-cli` and `roko-serve`)

**Phase 4: Wire `AgentError` into agent dispatch** (lowest priority since `AgentCreationError` and `LlmError` already have typed variants):
- The most impactful change: `ToolDispatcher::dispatch()` could return `AgentError::ToolDispatch` instead of `anyhow::Error`
- Low priority because the existing typed errors (`AgentCreationError`, `LlmError`, `ProviderError`) are already good enough for most callers

**Phase 5: Wire `ComposeError`**:
- Spec notes that `conventions.rs` uses anyhow internally and has no public functions. ComposeError is primarily a compile-only validation that the type exists.

### Files to Change

- `crates/roko-gate/src/generated.rs` -- remove type alias
- `crates/roko-gate/src/lib.rs` -- re-export from error.rs
- `crates/roko-learn/src/cascade_router.rs` -- change return types to `LearnError`
- `crates/roko-learn/src/episode_logger.rs` -- add `From<LoggerError> for LearnError` or change return types
- Callers in `crates/roko-cli/src/` and `crates/roko-serve/src/` that call CascadeRouter/EpisodeLogger public methods

### Test Coverage Gaps

- `roko-agent`: test `AgentCreationError` converts to `AgentError` via `From`
- `roko-gate`: test that `roko_gate::GateError` is the `error.rs` type, not the `RokoError` alias
- `roko-learn`: test `LearnError::Io { path, .. }` construction and `LearnError::Parse` from serde error
- `roko-compose`: compile-only test importing `roko_compose::ComposeError`

### Effort: 4-6 hours. Priority: Medium.

---

## Task 087: Frontend Architecture Redesign

**Priority**: 8/10 (demo UI technical debt)
**Effort**: S-M (2 files to fix, clear replacement patterns)

### Current State

**Completed (4/6 sub-tasks)**:
- `serve-url.ts`: `TIMEOUTS` and `RECONNECT_BACKOFF` exported
- `EventStreamContext.tsx`: uses `mgr.onConnectedChange` callback (no polling interval)
- `useServerHealth.ts`: uses SSE subscription + 10s fallback timeout (no polling interval)
- `prd-pipeline-types.ts`: uses `startsWith` tier map (no `model.includes`)
- `palette.ts`: exports `modelColor()` helper function
- ErrorBoundary: applied to route-level, dashboard, terminal boundaries

**Remaining (3 items)**:

1. **`demo/demo-app/src/hooks/useLiveApi.ts`** (117 lines):
   - Lines 14-16: Module-level singleton variables `_serverLive`, `_healthProbeInFlight`, `_healthListeners`
   - Lines 24-43: `probeServer()` module-level function with fetch + listeners
   - Lines 66-68: `setInterval(() => { void probeServer(); }, 5_000)` -- the 5-second polling loop
   - Lines 47-50: Already marked `@deprecated` with clear migration path to `useServerConnected()` from `src/data/selectors.ts`
   - Cleanup IS correct (line 70-73: `clearInterval` + listener removal)

2. **`demo/demo-app/src/hooks/useRokoConfig.ts`** (194 lines):
   - Lines 102-106: Function marked `@deprecated` pointing to `useConfigSlice()` from `src/data/selectors.ts`
   - Lines 126-130: `useEffect` with `fetchConfig()` + `setInterval(fetchConfig, 15_000)` -- 15-second config poll
   - Line 129: Cleanup correctly clears interval
   - Config reload SSE event (`config_reloaded`) is already handled by `DataHub.ts`

3. **`demo/demo-app/src/hooks/useBlockStream.ts`**: Timer type confusion -- `setTimeout` ref used where `setInterval` ref should be, or vice versa. Needs split into `pollIntervalRef` and `reconnectTimerRef`.

### Gap Analysis

| Spec Sub-task | Status |
|---|---|
| Central timeout/reconnect config | Done |
| Replace setInterval in EventStreamContext | Done |
| Replace setInterval in useServerHealth | Done |
| Fix stale closures in useWorkspace | Done |
| Fix model tier inference | Done |
| Apply ErrorBoundary | Done |
| Replace setInterval in useLiveApi | **Not done** -- 5s poll active at line 66 |
| Replace setInterval in useRokoConfig | **Not done** -- 15s poll active at line 128 |
| Split useBlockStream timer refs | **Not done** |

### Completion Design

**1. `useLiveApi.ts` -- Remove module-level polling**

Since the hook is deprecated and all callers should migrate to `useServerConnected()`, the simplest fix replaces the internal state with that selector:

```typescript
import { useServerConnected } from '../data/selectors';

export function useLiveApi() {
  const api = useApi();
  const isLive = useServerConnected();

  const get = useCallback(async <T = unknown>(path: string): Promise<T> => {
    return api.get<T>(path);
  }, [api]);

  const post = useCallback(async <T = unknown>(path: string, body?: unknown): Promise<T> => {
    return api.post<T>(path, body);
  }, [api]);

  const put = useCallback(async <T = unknown>(path: string, body?: unknown): Promise<T> => {
    return api.put<T>(path, body);
  }, [api]);

  return useMemo(
    () => ({ get, post, put, baseUrl: api.baseUrl, isLive }),
    [get, post, put, api.baseUrl, isLive],
  );
}
```

Remove ALL module-level variables and functions: `_serverLive`, `_healthProbeInFlight`, `_healthListeners`, `notifyHealthListeners()`, `probeServer()`.

Verify `useServerConnected` import exists -- check:
```bash
grep -n 'useServerConnected' demo/demo-app/src/data/selectors.ts
```

**2. `useRokoConfig.ts` -- Replace 15s poll with one-shot**

Replace lines 126-130:
```typescript
// BEFORE:
useEffect(() => {
  fetchConfig();
  intervalRef.current = setInterval(fetchConfig, 15_000);
  return () => clearInterval(intervalRef.current);
}, [fetchConfig]);

// AFTER:
useEffect(() => {
  fetchConfig();
  // No polling -- config reloads are handled by DataHub via config_reloaded SSE event.
}, [fetchConfig]);
```

Remove `intervalRef` (line 114) if it becomes unused.

**3. `useBlockStream.ts` -- Split timer refs**

Audit the file for `setTimeout`/`setInterval` usage. If a `ReturnType<typeof setTimeout>` ref is used with `clearInterval()`, or vice versa, split into two refs with correct types:
```typescript
const pollIntervalRef = useRef<ReturnType<typeof setInterval>>();
const reconnectTimerRef = useRef<ReturnType<typeof setTimeout>>();
```

### Files to Change

- `demo/demo-app/src/hooks/useLiveApi.ts` -- remove polling singleton, use `useServerConnected()`
- `demo/demo-app/src/hooks/useRokoConfig.ts` -- remove `setInterval`, keep initial fetch
- `demo/demo-app/src/hooks/useBlockStream.ts` -- split timer refs (if file exists and has confusion)

### Test Coverage Gaps

- `npx tsc --noEmit` must pass (TypeScript compilation)
- `npm run build` must succeed
- Verification greps:
  - `grep -rn 'setInterval' demo/demo-app/src/hooks/useLiveApi.ts` -- 0 matches
  - `grep -rn 'setInterval' demo/demo-app/src/hooks/useRokoConfig.ts` -- 0 matches
  - `grep -rn '_serverLive\|_healthListeners\|probeServer' demo/demo-app/src/hooks/useLiveApi.ts` -- 0 matches

### Effort: 2-3 hours. Priority: Medium.

---

## Task 089: Orchestration Cleanup

**Priority**: 6/10 (legacy path -- but config flag is misleading users)
**Effort**: S-M (concentrated in orchestrate.rs, one critical fix)

### Current State

- **File**: `crates/roko-cli/src/orchestrate.rs` (behind `#[cfg(feature = "legacy-orchestrate")]`)
- **Lines 17879-17894**: The `enable_advanced_rungs` match arm:
  ```rust
  Rung::Symbol | Rung::PropertyTest | Rung::Integration => {
      if !gate_config.enable_advanced_rungs {
          tracing::debug!(?rung, "advanced rung skipped (gates.enable_advanced_rungs not set)");
          skipped_count = skipped_count.saturating_add(1);  // SKIP
      } else {
          tracing::debug!(?rung, "advanced rung enabled via config");
          skipped_count = skipped_count.saturating_add(1);  // ALSO SKIP!
      }
  }
  ```
  Both branches do `skipped_count += 1`. The `enable_advanced_rungs = true` flag logs a different message but has **identical behavior** to `false`.

- **The `enable_advanced_rungs` field** exists on `gate_config` (confirmed by the access at line 17883). It is settable via `[gates]` in config.
- **The active runner** (`crates/roko-cli/src/runner/`) does NOT have this issue -- it uses `GatePipelineBuilder` with `RungCaps` selection.

### Gap Analysis

The spec had 5 sub-tasks:
| Sub-task | Status |
|---|---|
| Replace raw rung integers with Rung enum | Partial -- T1-11 refs removed, Rung used in match |
| Fix worktree config loading | **Needs verification** |
| Fix `enable_advanced_rungs` else branch | **Not done** -- enabled branch is a no-op |
| Delete `resolve_enrichment_backend()` | **Needs verification** |
| Remove model strings from active runner | **Needs verification** |

### Completion Design

**Critical fix -- `enable_advanced_rungs` else branch** (lines 17889-17894):

Replace the no-op else branch with actual gate dispatch:
```rust
} else {
    // Advanced rungs dispatched through run_gate_rung; stub_verdict
    // fallback handles missing capability inputs.
    tracing::debug!(?rung, "dispatching advanced rung (enabled via config)");
    match self.run_gate_rung(Some(plan_id), &payload_sig, rung.as_index()).await {
        Ok(verdicts) => {
            for v in verdicts {
                gate_results.push(v);
            }
        }
        Err(e) => {
            tracing::warn!(?rung, error = %e, "advanced rung failed, treating as skip");
            skipped_count = skipped_count.saturating_add(1);
        }
    }
}
```

If `run_gate_rung()` integration is complex (returns don't match `gate_results` type), the minimum fix is:
```rust
} else {
    // TODO: wire through run_gate_rung when return types are compatible
    tracing::warn!(?rung, "advanced rung enabled but dispatch not yet wired");
    skipped_count = skipped_count.saturating_add(1);
}
```
This at least makes the behavior difference visible (warn vs debug) and documents the limitation.

**Verification for remaining sub-tasks**:
```bash
# Check if resolve_enrichment_backend was deleted:
grep -n 'resolve_enrichment_backend' crates/roko-cli/src/orchestrate.rs
# Check worktree config loading:
grep -n 'load_roko_config.*exec_dir' crates/roko-cli/src/orchestrate.rs
# Check model strings in active runner:
grep -rn '"claude-\|"sonnet\|"haiku\|"opus' crates/roko-cli/src/runner/ --include='*.rs' | grep -v 'cfg(test'
# Check rung integer literals:
grep -n 'rung == [0-9]' crates/roko-cli/src/orchestrate.rs
```

### Files to Change

- `crates/roko-cli/src/orchestrate.rs` (lines 17889-17894) -- fix the else branch

### Test Coverage Gaps

- No test currently exercises the `enable_advanced_rungs = true` path
- Add test: with `enable_advanced_rungs = true`, assert advanced rungs are NOT unconditionally skipped (verify they enter the dispatch path or at least log at warn level)

### Effort: 1-2 hours (critical fix only) or 4-6 hours (all 5 sub-tasks). Priority: Medium.

---

## Task 090: Provider UX Redesign

**Priority**: 3/10 (user-facing error quality, first-run experience)
**Effort**: L (4 sub-tasks across 7 files)

### Current State

- **`map_provider_error()`** in `crates/roko-agent/src/provider/mod.rs` (lines 617-683): Well-implemented with 5 pattern matches (401, 429, 404, connection refused, ENOENT). Returns human-readable error strings.
- **6 unit tests** (lines 1535-1620): Cover all 5 patterns plus unknown-pattern fallback. All pass.
- **Zero production callers**: `grep -rn 'map_provider_error(' crates/ --include='*.rs'` shows ONLY the definition and test calls. The function is never called from `create_agent_for_model()`, `ModelCallService::execute()`, or any other dispatch path.
- **No `check_provider_readiness()` function exists** -- no pre-flight check at startup.
- **No `Available` variant** in `ConfigProviderCmd` -- no provider discovery command.
- **Post-merge validation**: `collect_diagnostics()` in `loader.rs` warns about dangling provider references in lenient mode, but no strict mode (`ValidationConfig`) exists.

### Gap Analysis

| Spec Sub-task | Status |
|---|---|
| 1. `map_provider_error()` implementation | Done -- function exists with tests |
| 1b. Wire into dispatch path | **Not done** -- zero production callers |
| 2. `check_provider_readiness()` pre-flight | **Not done** -- function doesn't exist |
| 3. `roko config providers available` command | **Not done** -- no `Available` variant |
| 4. Post-merge strict validation | **Partial** -- lenient warnings exist, no strict mode |

### Completion Design

**Sub-task 1b: Wire `map_provider_error()` into the dispatch path**

In `crates/roko-agent/src/provider/mod.rs`, at the `create_agent_for_model()` error path:
```rust
// Wrap provider adapter errors with human-readable context:
let agent = adapter.create_agent(profile, config).map_err(|e| {
    let human_msg = map_provider_error(
        provider_config.kind,
        provider_name,
        provider_config.api_key_env.as_deref(),
        provider_config.base_url.as_deref(),
        &e,
    );
    e.context(human_msg)  // or create a new AgentCreationError variant
})?;
```

Also wire into streaming execution paths where HTTP errors surface raw. Check `openai_compat_backend.rs::send_turn_streaming()` and `anthropic_api/tool_loop.rs` for error propagation points.

**Sub-task 2: Implement `check_provider_readiness()`**

Add to `crates/roko-agent/src/provider/mod.rs` (or new `pre_flight.rs`):
```rust
pub struct ProviderReadinessIssue {
    pub provider_name: String,
    pub message: String,
}

pub fn check_provider_readiness(config: &RokoConfig) -> Vec<ProviderReadinessIssue> {
    let mut issues = Vec::new();
    // Build set of providers referenced by at least one model
    let referenced: HashSet<&str> = config.models.values()
        .map(|m| m.provider.as_str()).collect();

    for name in &referenced {
        let Some(provider) = config.providers.get(*name) else { continue };
        match provider.kind {
            ProviderKind::ClaudeCli => {
                let cmd = provider.command.as_deref().unwrap_or("claude");
                if std::process::Command::new(cmd).arg("--version").output().is_err() {
                    issues.push(ProviderReadinessIssue {
                        provider_name: name.to_string(),
                        message: format!("{cmd} not found on PATH"),
                    });
                }
            }
            _ => {
                if let Some(env_var) = &provider.api_key_env {
                    if std::env::var(env_var).unwrap_or_default().is_empty() {
                        issues.push(ProviderReadinessIssue {
                            provider_name: name.to_string(),
                            message: format!("${env_var} not set for provider '{name}'"),
                        });
                    }
                }
            }
        }
    }
    issues
}
```

Wire into CLI boot in:
- `crates/roko-cli/src/commands/do_cmd.rs` -- before `run_standard_path`/`run_complex_path`
- `crates/roko-cli/src/commands/plan.rs` -- in `PlanCmd::Run` branch
- `crates/roko-cli/src/unified.rs` -- before `cmd_unified_chat()`

Print each issue as `warning: ...` to stderr. Exit nonzero only when ALL referenced providers fail.

**Sub-task 3: `roko config providers available` command**

In `crates/roko-cli/src/main.rs`, add to `ConfigProviderCmd`:
```rust
/// List all supported provider kinds with setup instructions.
Available,
```

In `crates/roko-cli/src/commands/config_cmd.rs`:
- Add dispatch arm for `ConfigProviderCmd::Available => cmd_provider_available().await`
- Implement `cmd_provider_available()`: iterate all 7 `ProviderKind` variants, print kind name, description, required env var, base URL, and setup instructions
- This must work with NO `roko.toml` present (no config loading)

**Sub-task 4: Strict post-merge validation**

In `crates/roko-core/src/config/schema.rs`:
```rust
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ValidationConfig {
    #[serde(default)]
    pub strict_validation: bool,
}
```
Add `#[serde(default)] pub validation: ValidationConfig` to `RokoConfig`.

In `crates/roko-core/src/config/loader.rs`, after merge:
```rust
for (model_key, profile) in &config.models {
    if !config.providers.contains_key(&profile.provider) {
        if config.validation.strict_validation {
            return Err(anyhow::anyhow!(
                "model '{}' references provider '{}' which is not defined",
                model_key, profile.provider
            ));
        }
        // Lenient mode: existing collect_diagnostics() already warns
    }
}
```

### Files to Change

- `crates/roko-agent/src/provider/mod.rs` -- wire `map_provider_error`, add `check_provider_readiness`
- `crates/roko-cli/src/main.rs` -- add `Available` to `ConfigProviderCmd`
- `crates/roko-cli/src/commands/config_cmd.rs` -- add `cmd_provider_available()`
- `crates/roko-core/src/config/schema.rs` -- add `ValidationConfig`
- `crates/roko-core/src/config/loader.rs` -- add strict validation after merge
- `crates/roko-cli/src/commands/do_cmd.rs` -- call `check_provider_readiness`
- `crates/roko-cli/src/commands/plan.rs` -- call `check_provider_readiness` in Run branch

### Test Coverage Gaps

- `map_provider_error()` unit tests exist -- add integration test proving it is called from `create_agent_for_model()` error path
- `check_provider_readiness()`: test with temp config, missing env var, nonexistent claude command path
- `cmd_provider_available()`: CLI parse test + output contains all 7 provider kind labels
- Config validation: lenient mode warns but succeeds; strict mode returns error with model key and provider key in message

### Effort: 6-8 hours (4 sub-tasks). Priority: High.

---

## Pattern: Missing Tests

Nearly every NEEDS_WORK task shares a common gap: the spec required specific unit/integration tests that weren't written. This is consistent across both Claude and Codex agents -- implementation code is written, test assertions are skipped.

Tasks where missing tests are the PRIMARY gap (code is otherwise correct):
- 013 (SSE replay bound test)
- 019 (MCP status path tests)
- 026 (serde/bus/property tests)
- 044 (time-paused timeout test)
- 061 (effective_max_output tests)
- 064 (fallback logic tests)
- 072 (banner helper tests)
