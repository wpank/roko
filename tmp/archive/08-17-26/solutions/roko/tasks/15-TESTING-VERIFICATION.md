# Testing and Verification: Task Breakdown

> Build comprehensive test infrastructure across 10 phases, 38 tasks. Shared
> harness, per-subsystem integration tests, end-to-end CLI smoke tests, gate
> parity verification, benchmark regression suite, learning persistence tests,
> performance regression baselines, chaos/fault injection, CLI snapshot tests,
> and ACP protocol conformance.
>
> Sources: `impl/15-TESTING-VERIFICATION.md`, `13-PERF-HAL-AND-AGENT-BENCHMARKS.md`,
> `13-PERF-HAL-BENCHMARK-INTEGRATION.md`, codebase analysis

---

## Overview

### Current Test Inventory

| Crate | Integration Tests | Inline `#[cfg(test)]` | Notable Gaps |
|---|---|---|---|
| roko-cli | 21 files | ~15 modules | No plan lifecycle, no learn CLI, no error handling coverage |
| roko-agent | 23 files | ~10 modules | Good provider coverage, no dispatch integration |
| roko-gate | 4 files (`gate_truth.rs`, `rungs.rs`, `compile_real_project.rs`, `adaptive_threshold.rs`) | 38 modules | No composition tests, no parity tests, no feedback tests |
| roko-learn | 4 files (`learning_loop.rs`, `cascade_router_integration.rs`, `model_router_integration.rs`, `agent_event_types.rs`) | ~8 modules | No concurrent access, no full feedback loop, no artifact roundtrip |
| roko-serve | 5 files (`api_integration.rs`, `security_bind.rs`, `prd_publish.rs`, `job_runner_integration.rs`, `job_lifecycle.rs`) | ~5 modules | Limited route coverage |
| roko-compose | 2 files (`cache_stability.rs`, `system_prompt_snapshot.rs`) | ~3 modules | No prompt assembly integration |
| roko-acp | 3 files (`protocol_conformance.rs`, `telemetry_integration.rs`, `helpers.rs`) | ~3 modules | Limited session lifecycle |
| roko-runtime | 1 file (`process_supervisor.rs`) | ~4 modules | No workflow engine, no pipeline state, no event bus |
| roko-orchestrator | 1 file (`lifecycle.rs`) | ~3 modules | No DAG, no parallel executor |
| roko-fs | 0 files | ~3 modules | No integration tests at all |
| roko-dreams | 0 files | ~2 modules | No integration tests at all |
| roko-neuro | 0 files | ~3 modules | No integration tests at all |
| roko-index | 0 files | ~2 modules | No integration tests at all |

### Existing Test Infrastructure

Shared test helpers live at `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/tests/common/mod.rs` and provide:
- `init_workspace()` -- runs `roko init` in a tempdir
- `seed_minimal_rust_project()` -- writes `Cargo.toml` + `src/main.rs`
- `seed_git_repo()` -- initializes git with an initial commit
- `setup_sample_plan_workspace()` -- full workspace with mock claude, sample plan, roko.toml
- `run_roko()` / `run_roko_isolated()` -- run CLI with assert_cmd
- `spawn_roko_serve_on_random_port()` -- HTTP server for API tests
- `pick_unused_port()` -- random port allocation

Workspace-level dev-dependencies available: `assert_cmd`, `tempfile`, `proptest`, `criterion`, `reqwest`.

### Benchmark Infrastructure

Existing benchmark harness at `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/bench.rs`:
- `SweBenchOptions` with `SweAgentMode` (Gold, Empty, PredictionFile, Command)
- Built-in 2-task smoke dataset
- Learning integration (episodes, efficiency events, C-factor)
- Knowledge store integration

Existing benchmark comparison at `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/bench_demo.rs`:
- `BenchTask` / `BenchResult` structs for side-by-side comparison
- Cost waterfall decomposition

Serve-side benchmark types at `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/bench.rs`:
- `BenchSuite` / `BenchTask` / `BenchTaskResult` / `BenchStrategy`

### Target State

Every crate has at least one integration test. Gate parity between `GateService` and legacy paths is verified. All CLI subcommands have crash-free tests. Learning artifacts survive restart cycles. Performance baselines detect 20%+ regressions. Agent crashes and state corruption produce graceful recovery.

---

## Anti-Patterns to Remove

| ID | Anti-Pattern | Where | Severity |
|---|---|---|---|
| AP-BENCH-STUB | `BenchmarkRegressionGate` always passes (stub) | `/Users/will/dev/nunchi/roko/roko/crates/roko-gate/src/benchmark_gate.rs` | High |
| AP-COST-ZERO | `BenchResult.cost_usd` always 0.0 in bench harness | `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/bench.rs` | Medium |
| AP-UNREACHABLE | `unreachable!()` in config MCP/experiments/plugins/secrets dispatch | `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/commands/config_cmd.rs:197-209` | Medium |
| AP-NO-HARNESS | Test helpers are CLI-only; no shared crate for gate/learn/runtime test scaffolding | `crates/roko-cli/tests/common/mod.rs` only | Medium |
| AP-NO-CONCURRENT | Learning artifacts (episodes, router, thresholds) never tested under concurrent access | `crates/roko-learn/tests/` | High |
| AP-NO-ROUNDTRIP | Learning artifacts never tested for write-restart-read persistence | `crates/roko-learn/tests/` | High |
| AP-SINGLE-RUN | Benchmark harness runs each task once; no multi-trial consistency measurement | `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/bench.rs` | Medium |
| AP-NO-PARITY | Two gate execution paths (GateService vs orchestrate.rs run_rung) never compared for verdict equivalence | `crates/roko-gate/` | High |

---

## Phase 1: Test Harness and Infrastructure (Tasks 15.1-15.3)

All subsequent phases depend on Phase 1. No other phase should start until these three tasks are complete.

### Task 15.1: Create roko-test-harness crate with TestWorkspace
**Priority**: P0
**Estimated Effort**: 4 hours
**Files to Create**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-test-harness/Cargo.toml`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-test-harness/src/lib.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-test-harness/src/workspace.rs`
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/Cargo.toml` (add member)
**Depends On**: none

#### Context
Test helpers are currently embedded in `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/tests/common/mod.rs` and only available to roko-cli tests. The `TestWorkspace` needs to be shared across roko-gate, roko-learn, roko-runtime, roko-compose, roko-orchestrator, and roko-acp integration tests.

The existing `common/mod.rs` (470 LOC) provides `init_workspace()`, `seed_minimal_rust_project()`, `seed_git_repo()`, `setup_sample_plan_workspace()`, `run_roko()`, `run_roko_isolated()`, `spawn_roko_serve_on_random_port()`, `pick_unused_port()`, and a mock claude script. These patterns should be extracted and generalized.

#### Implementation Steps
1. Create `crates/roko-test-harness/Cargo.toml` with dependencies:
   - `tempfile = { workspace = true }`
   - `serde = { workspace = true }`
   - `serde_json = { workspace = true }`
   - `roko-core = { path = "../roko-core" }`
   - `tokio = { workspace = true, features = ["full", "test-util"] }`
2. Add `"crates/roko-test-harness"` to workspace members in the root `Cargo.toml`.
3. Create `TestWorkspace` struct wrapping `TempDir` with methods:
   - `new() -> Self` -- creates tempdir, initializes `.roko/` directory structure, writes minimal `roko.toml`
   - `path() -> &Path` -- returns tempdir path
   - `roko_dir() -> PathBuf` -- returns `.roko/` path
   - `write_config(toml: &str)` -- writes `roko.toml`
   - `write_file(relative: &str, content: &str)` -- writes arbitrary file
   - `read_file(relative: &str) -> String` -- reads file content
   - `file_exists(relative: &str) -> bool`
   - `seed_learning_state()` -- creates `.roko/learn/` with empty artifacts (episodes.jsonl, cascade-router.json, efficiency.jsonl, gate-thresholds.json, section-effects.json)
   - `seed_episodes(episodes: &[Episode])` -- writes pre-built episodes to `.roko/episodes.jsonl`
4. `TestWorkspace` implements `Drop` by delegating to `TempDir::drop` (automatic cleanup).
5. Re-export `TestWorkspace` from `lib.rs`.

#### Design Guidance
`TestWorkspace` should NOT depend on the `roko` binary (no `assert_cmd`). It creates the filesystem layout that roko expects, but tests that need the binary use `CliRunner` (Task 15.2). This separation lets gate, learn, and runtime tests use `TestWorkspace` without building the full CLI binary.

The `.roko/` directory layout must match what `roko init` creates. Inspect the actual `roko init` output path: `.roko/`, `.roko/learn/`, `.roko/state/`, `.roko/prd/`, `.roko/research/`, `.roko/bench/`, `roko.toml`. The `seed_learning_state()` method should create files that match the schemas expected by `EpisodeLogger`, `CascadeRouter`, `AdaptiveThresholds`, etc.

#### Verification Criteria
- [ ] `cargo check -p roko-test-harness` compiles
- [ ] `TestWorkspace::new()` creates a valid directory layout
- [ ] `seed_learning_state()` creates all expected files
- [ ] `TestWorkspace` can be used from any crate via `[dev-dependencies]`

---

### Task 15.2: Build CliRunner wrapper
**Priority**: P0
**Estimated Effort**: 3 hours
**Files to Create**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-test-harness/src/cli_runner.rs`
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-test-harness/Cargo.toml` (add `assert_cmd`, `predicates`)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-test-harness/src/lib.rs` (add module)
**Depends On**: Task 15.1

#### Context
The existing `run_roko()` and `run_roko_isolated()` functions in `crates/roko-cli/tests/common/mod.rs` wrap `assert_cmd::Command::cargo_bin("roko")` with workspace isolation. These need to be promoted to the shared harness with a richer API for structured output capture.

#### Implementation Steps
1. Add `assert_cmd = { workspace = true }` and `predicates = { workspace = true }` to `roko-test-harness/Cargo.toml`.
2. Create `CliRunner` struct with fields: `workdir: PathBuf`, `env_overrides: HashMap<String, String>`, `env_removals: Vec<String>`.
3. Methods:
   - `new(workspace: &TestWorkspace) -> Self` -- binds to workspace, removes `ANTHROPIC_API_KEY`, `OPENAI_API_KEY`, `XDG_CONFIG_HOME` by default
   - `with_env(key: &str, value: &str) -> Self` -- add environment variable
   - `without_env(key: &str) -> Self` -- remove environment variable
   - `run_init() -> CapturedOutput` -- executes `roko init <workdir>` and returns output
   - `run_cmd(args: &[&str]) -> CapturedOutput` -- executes arbitrary subcommand
   - `assert_success(args: &[&str])` -- runs and asserts exit 0
   - `assert_failure(args: &[&str])` -- runs and asserts non-zero exit
   - `assert_output_contains(args: &[&str], pattern: &str)` -- runs, asserts exit 0, asserts stdout or stderr contains pattern
   - `assert_json_output(args: &[&str]) -> serde_json::Value` -- runs, asserts exit 0, parses stdout as JSON
4. `CapturedOutput` struct: `stdout: String`, `stderr: String`, `exit_code: i32`, `duration: Duration`.
5. All commands set `HOME` to the workspace dir (mimicking `run_roko_isolated()`).
6. All commands set `ROKO_LOG=error` to suppress noise.

#### Verification Criteria
- [ ] `CliRunner::new(ws).assert_output_contains(&["--help"], "Usage")` passes
- [ ] `CliRunner::new(ws).run_init()` succeeds with exit 0
- [ ] Environment variables are isolated between test runs
- [ ] `CapturedOutput.duration` is populated

---

### Task 15.3: Build gate test scaffold
**Priority**: P0
**Estimated Effort**: 3 hours
**Files to Create**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-test-harness/src/gate_scaffold.rs`
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-test-harness/src/lib.rs` (add module)
**Depends On**: Task 15.1

#### Context
Gate tests need a minimal Cargo project to run `CompileGate`, `ClippyGate`, `TestGate`, and friends. The existing `scaffold_cargo_project()` in `crates/roko-gate/tests/gate_truth.rs` creates a minimal project but only for compile tests. The `seed_minimal_rust_project()` in `crates/roko-cli/tests/common/mod.rs` creates another variant. Both should be replaced by a shared scaffold with mutation methods.

#### Implementation Steps
1. Create `GateTestProject` struct wrapping a `TempDir` with a valid Cargo project.
2. `GateTestProject::new()` creates:
   - `Cargo.toml` with `[package]` and `edition = "2021"` and `[lib]`
   - `src/lib.rs` with `pub fn answer() -> u32 { 42 }` and a passing `#[test]`
3. Mutation methods:
   - `break_compile()` -- inserts `let x: i32 = "not a number";` into `src/lib.rs`
   - `break_clippy()` -- inserts `let _ = vec![1,2,3].len() > 0;` (triggers `clippy::len_zero`)
   - `break_test()` -- changes test assertion to `assert_eq!(answer(), 999)`
   - `add_unused_import()` -- adds `use std::collections::HashMap;` (clippy warning)
   - `add_borrow_error()` -- inserts code with borrow checker violation
   - `add_type_mismatch()` -- inserts `let _: String = 42u32;`
   - `restore()` -- resets `src/lib.rs` to the original passing state
4. Query methods:
   - `path() -> &Path` -- root of the Cargo project
   - `lib_path() -> PathBuf` -- path to `src/lib.rs`
5. Helper: `run_gate(project: &GateTestProject, gates: &[&str]) -> GateReport` -- runs `GateService::run_gates()` with the project as workdir.

#### Design Guidance
The gate scaffold must produce deterministic Cargo projects. Use `edition = "2021"` (not `edition = "2024"` which the existing `seed_minimal_rust_project` uses and which is only available on newer rustc). The test method in `src/lib.rs` should be `#[test] fn it_works() { assert_eq!(answer(), 42); }`.

#### Verification Criteria
- [ ] `GateTestProject::new()` passes compile + clippy + test gates
- [ ] `break_compile()` causes `CompileGate` to fail
- [ ] `break_clippy()` causes `ClippyGate` to fail
- [ ] `break_test()` causes `TestGate` to fail
- [ ] `restore()` returns project to passing state

---

## Phase 2: Per-Subsystem Integration Tests (Tasks 15.4-15.11)

All tasks in this phase can run in parallel. Each targets a single crate.

**Dependencies**: Phase 1 (Tasks 15.1-15.3)

### Task 15.4: Gate subsystem integration tests
**Priority**: P0
**Estimated Effort**: 6 hours
**Files to Create**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-gate/tests/gate_subsystem.rs`
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-gate/Cargo.toml` (add `roko-test-harness` dev-dep)
**Depends On**: Task 15.3

#### Context
roko-gate has 38 source files with inline `#[cfg(test)]` modules but only 4 integration test files. Key types that need integration-level testing:

- `GateService` at `/Users/will/dev/nunchi/roko/roko/crates/roko-gate/src/gate_service.rs` (line 26) -- the main entry point, method `run_gates(config) -> GateReport`
- `ComposedGatePipeline` at `/Users/will/dev/nunchi/roko/roko/crates/roko-gate/src/gate_pipeline.rs` (line 328) -- sequential/parallel/voting/fallback composition
- `ParallelGate` / `VotingGate` / `FallbackGate` at `/Users/will/dev/nunchi/roko/roko/crates/roko-gate/src/composition.rs` (lines 22, 152, 284)
- `BenchmarkRegressionGate` at `/Users/will/dev/nunchi/roko/roko/crates/roko-gate/src/benchmark_gate.rs` (line 30) -- currently a stub that always passes
- `GateFeedback` at `/Users/will/dev/nunchi/roko/roko/crates/roko-gate/src/feedback.rs` (line 53) -- structured error/warning/suggestion extraction
- `feedback_for_agent()` at `/Users/will/dev/nunchi/roko/roko/crates/roko-gate/src/feedback.rs` (line 202)

#### Implementation Steps
1. Test `GateService` with a passing `GateTestProject`:
   - Enable `["compile"]` -- verify 1 verdict, passed=true
   - Enable `["compile", "clippy"]` -- verify 2 verdicts, both passed
   - Enable `["compile", "clippy", "test"]` -- verify 3 verdicts, all passed
2. Test `GateService` with a broken project:
   - `break_compile()` + enable `["compile"]` -- verify passed=false, output contains "error"
   - `break_clippy()` + enable `["compile", "clippy"]` -- compile passes, clippy fails
   - `break_test()` + enable `["compile", "clippy", "test"]` -- compile+clippy pass, test fails
3. Test `GateService` with `["shell"]` and custom `ShellGateCommand`:
   - `program: "echo"`, `args: ["ok"]` -- passes
   - `program: "false"` -- fails with exit code 1
4. Test `GateService` with `["diff"]` -- verify DiffGate runs (needs git repo in project)
5. Test `GateService` ordering: rungs 0, 1, 2 execute in order; failure on rung 0 prevents rung 1
6. Test `GateReport::all_passed()` is true only when all verdicts pass
7. Test `GateReport::all_passed()` is false when any verdict fails
8. Test `ParallelGate` with 3 mock shell gates (all `true`) -- runs concurrently, minimum score
9. Test `VotingGate` with 3 mock gates: 2 pass, 1 fail -- passes at threshold 2/3
10. Test `VotingGate` with 3 mock gates: 1 pass, 2 fail -- fails at threshold 2/3
11. Test `FallbackGate` with primary=`false`, fallback=`true` -- tries primary, falls back
12. Test `ComposedGatePipeline` in Sequential mode with 2 gates
13. Test `ComposedGatePipeline` in Parallel mode with 2 gates
14. Test `ComposedGatePipeline` in Voting mode with 3 gates
15. Test `ComposedGatePipeline` in Fallback mode with 2 gates
16. Test `BenchmarkRegressionGate` stub behavior -- currently always passes, verify
17. Test `feedback_for_agent()` with compile error output -- verify structured `GateFeedback`

#### Verification Criteria
- [ ] 17+ new tests, all passing
- [ ] Every gate type in `crates/roko-gate/src/` has at least one test
- [ ] Composition modes all tested
- [ ] `cargo test -p roko-gate -- gate_subsystem` runs in < 60 seconds

---

### Task 15.5: Learning subsystem integration tests
**Priority**: P0
**Estimated Effort**: 5 hours
**Files to Create**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/tests/learning_subsystem.rs`
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/Cargo.toml` (add `roko-test-harness` dev-dep)
**Depends On**: Task 15.1

#### Context
Key types to test:
- `EpisodeLogger` at `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/episode_logger.rs` (line 911) -- append-only JSONL
- `CascadeRouter` at `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/cascade_router.rs` (line 82) -- bandit-based model routing
- `SectionEffectivenessRegistry` at `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/section_effect.rs` (line 114)
- `PlaybookStore` at `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/playbook.rs` (line 652)
- `AgentEfficiencyEvent` at `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/efficiency.rs` (line 80)
- `DriftDetector` at `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/drift.rs` (line 89)

Existing tests in `learning_loop.rs` (4 tests) and `cascade_router_integration.rs` cover basic paths but not concurrent access, field integrity under volume, or persistence roundtrips.

#### Implementation Steps
1. Test `EpisodeLogger`: write 100 episodes, read back, verify ordering and field integrity (all fields non-default)
2. Test `EpisodeLogger`: concurrent appends from 5 tokio tasks (20 episodes each), verify total count = 100
3. Test `CascadeRouter`: `load_or_new`, observe 50 outcomes across 3 models, save to disk, reload from same file, verify observation counts match
4. Test `CascadeRouter`: routing decisions shift after observing 20 successes for model A and 20 failures for model B
5. Test `SectionEffectivenessRegistry`: record positive/negative signals for 10 sections, verify weights shift in expected direction
6. Test `SectionEffectivenessRegistry`: persist and reload, verify weights and counts match
7. Test `PlaybookStore`: write 5 playbooks with different roles/categories, query by role, verify correct subset returned
8. Test `PlaybookStore`: query by category, verify filtering works
9. Test `AgentEfficiencyEvent`: write 50 events, verify JSONL line count matches
10. Test `DriftDetector`: feed 100 increasing values, verify drift detected
11. Test `DriftDetector`: feed 100 stable values, verify no drift detected

#### Verification Criteria
- [ ] 11+ new tests, all passing
- [ ] Every learning artifact type has at least one integration test
- [ ] Concurrent access test uses real tokio tasks (not serial simulation)
- [ ] `cargo test -p roko-learn -- learning_subsystem` runs in < 30 seconds

---

### Task 15.6: Agent dispatcher integration tests
**Priority**: P1
**Estimated Effort**: 4 hours
**Files to Create**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/tests/dispatch_subsystem.rs`
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/Cargo.toml` (add `roko-test-harness` dev-dep if needed)
**Depends On**: Task 15.1

#### Context
roko-agent has 23 integration test files focused on individual provider parity (openai, codex, cursor, gemini, kimi, ollama, etc.) and safety. Missing: integration tests for the dispatcher layer itself, tool loop, and MCP passthrough.

Key types:
- Dispatcher at `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/dispatcher/mod.rs`
- Mock provider test at `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/tests/mock_provider.rs`
- Safety integration at `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/tests/safety_integration.rs`
- Tool loop at `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/tests/tool_loop_integration.rs`

#### Implementation Steps
1. Test mock provider dispatches correctly: configure mock, send prompt, verify response contains expected text
2. Test provider selection: configure 2 providers, verify dispatch routes to requested provider
3. Test tool loop: mock provider returns a tool call, verify tool is executed and result fed back
4. Test tool loop max iterations: configure `max_iterations=3`, verify loop terminates after 3 rounds
5. Test safety contract enforcement: configure deny-list, attempt denied tool, verify rejection
6. Test MCP config passthrough: provide `mcp_config` path, verify it is passed to the agent process
7. Test stream accumulation: mock provider returns 5 stream chunks, verify accumulator produces complete message
8. Test error handling: mock provider returns error, verify dispatch returns structured error (not panic)

#### Verification Criteria
- [ ] 8+ new tests, all passing
- [ ] Mock provider tests do not require API keys
- [ ] Tool loop tested with at least one tool call round-trip
- [ ] `cargo test -p roko-agent -- dispatch_subsystem` runs in < 30 seconds

---

### Task 15.7: Runtime and workflow engine integration tests
**Priority**: P1
**Estimated Effort**: 4 hours
**Files to Create**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-runtime/tests/workflow_subsystem.rs`
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-runtime/Cargo.toml` (add `roko-test-harness` dev-dep)
**Depends On**: Task 15.1

#### Context
Key types:
- `PipelineStateV2` at `/Users/will/dev/nunchi/roko/roko/crates/roko-runtime/src/pipeline_state.rs` (line 530) -- 10-state state machine
- `WorkflowEngine` at `/Users/will/dev/nunchi/roko/roko/crates/roko-runtime/src/workflow_engine.rs` (line 105)
- `EventBus` at `/Users/will/dev/nunchi/roko/roko/crates/roko-runtime/src/event_bus.rs` (line 233)
- `JsonlLogger` at `/Users/will/dev/nunchi/roko/roko/crates/roko-runtime/src/jsonl_logger.rs` (line 15)
- `ProcessSupervisor` at `/Users/will/dev/nunchi/roko/roko/crates/roko-runtime/src/process.rs` (line 839)

Only 1 existing integration test file (`process_supervisor.rs`). The state machine, event bus, and workflow engine have zero integration coverage.

#### Implementation Steps
1. Test `PipelineStateV2` state machine transitions: Start -> StrategyPhase -> ImplementPhase -> GatePhase -> ReviewPhase -> CommitPhase -> Done
2. Test `PipelineStateV2` error paths: agent failure at ImplementPhase triggers retry up to `max_autofix_attempts`
3. Test `PipelineStateV2` gate failure: GatesFailed input triggers autofix iteration or terminal failure
4. Test `EventBus` publish-subscribe: register consumer, publish event, verify consumer receives it
5. Test `EventBus` fan-out: register 3 consumers, publish 1 event, verify all 3 receive it
6. Test `JsonlLogger` writes events to disk in JSONL format, verify each line is valid JSON
7. Test `ProcessSupervisor` tracks spawned processes and reports them via status queries
8. Test `WorkflowEngine` lifecycle: construct with mock services, verify state transitions

#### Verification Criteria
- [ ] 8+ new tests, all passing
- [ ] Every `PipelineStateV2` major transition path is covered
- [ ] EventBus fan-out works with 3+ consumers
- [ ] `cargo test -p roko-runtime -- workflow_subsystem` runs in < 15 seconds

---

### Task 15.8: ACP protocol conformance tests
**Priority**: P1
**Estimated Effort**: 5 hours
**Files to Create**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-acp/tests/acp_conformance.rs`
**Depends On**: Task 15.1

#### Context
Existing test infrastructure in `/Users/will/dev/nunchi/roko/roko/crates/roko-acp/tests/protocol_conformance.rs` provides a `TestHarness` with `TestClient` using `tokio::io::DuplexStream` for in-process JSON-RPC communication. This pattern should be reused.

Key types:
- `AcpSession` at `/Users/will/dev/nunchi/roko/roko/crates/roko-acp/src/session.rs` (line 237)
- Error codes: `SESSION_NOT_FOUND`, `METHOD_NOT_FOUND`, `PARSE_ERROR` from `roko_acp::types`

The existing conformance file covers basic JSON-RPC; missing: session lifecycle, mode switching, config updates, slash commands, conversation history, cancellation.

#### Implementation Steps
1. Test session create -> load -> list -> cancel lifecycle via JSON-RPC
2. Test mode switching: code/plan/research modes produce different system prompts
3. Test config updates: model, effort, temperament, gates, workflow -- all modifiable mid-session
4. Test slash command handling: `/model`, `/effort`, `/status` produce local responses (no agent dispatch)
5. Test conversation history: multi-turn history accumulates correctly
6. Test cancellation: start a pipeline run, cancel mid-execution, verify cooperative shutdown
7. Test error codes: `SESSION_NOT_FOUND` for invalid session IDs, `METHOD_NOT_FOUND` for unknown methods, `PARSE_ERROR` for malformed JSON
8. Test notification delivery: server sends notifications for gate progress, phase transitions
9. Test protocol version negotiation: client sends protocolVersion, server responds
10. Test empty/missing fields: omit optional fields, verify defaults are applied

#### Verification Criteria
- [ ] 10+ new tests, all passing
- [ ] Every JSON-RPC error code exercised
- [ ] Session lifecycle fully covered (create, use, resume, cancel, cleanup)
- [ ] `cargo test -p roko-acp -- acp_conformance` runs in < 20 seconds

---

### Task 15.9: Serve HTTP API integration tests
**Priority**: P1
**Estimated Effort**: 5 hours
**Files to Create**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/tests/serve_subsystem.rs`
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/Cargo.toml` (add `roko-test-harness` dev-dep)
**Depends On**: Task 15.1

#### Context
roko-serve has ~85 routes and 5 existing integration test files. The existing `api_integration.rs` uses `tower::ServiceExt::oneshot` for in-process HTTP testing (no real server). This pattern should be extended to cover more route groups.

#### Implementation Steps
1. Test status routes: `GET /api/status` returns valid JSON with expected fields
2. Test plan routes: `POST /api/plans`, `GET /api/plans`, `GET /api/plans/:id`
3. Test PRD routes: `GET /api/prds`, `POST /api/prds`, `GET /api/prds/:slug`
4. Test agent routes: `GET /api/agents`, `POST /api/agents`, agent lifecycle
5. Test job routes: `GET /api/jobs`, `POST /api/jobs`, `GET /api/jobs/:id`
6. Test config routes: `GET /api/config`, `GET /api/config/providers`, `GET /api/config/models`
7. Test learning routes: `GET /api/learning/episodes`, `GET /api/learning/router`, `GET /api/learning/efficiency`
8. Test auth middleware: requests without API key return 401 when auth enabled
9. Test auth bypass: requests without auth header succeed when auth disabled
10. Test SSE endpoint: `GET /api/events` returns SSE stream with `event:` prefix
11. Test OpenAPI spec: `GET /api/openapi.json` returns valid JSON
12. Test 404: unknown route returns 404 with JSON body

#### Verification Criteria
- [ ] 12+ new tests, all passing
- [ ] Every major route group has at least one test
- [ ] Auth middleware tested in both enabled and disabled modes
- [ ] `cargo test -p roko-serve -- serve_subsystem` runs in < 30 seconds

---

### Task 15.10: Compose and prompt assembly tests
**Priority**: P1
**Estimated Effort**: 3 hours
**Files to Create**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-compose/tests/compose_subsystem.rs`
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-compose/Cargo.toml` (add `roko-test-harness` dev-dep)
**Depends On**: Task 15.1

#### Context
Key types:
- `SystemPromptBuilder` at `/Users/will/dev/nunchi/roko/roko/crates/roko-compose/src/system_prompt_builder.rs` (line 62) -- 9-layer prompt assembly
- `RoleSystemPromptSpec` at `/Users/will/dev/nunchi/roko/roko/crates/roko-compose/src/role_prompts.rs` (line 228)
- `PromptAssemblyService` at `/Users/will/dev/nunchi/roko/roko/crates/roko-compose/src/prompt_assembly_service.rs` (line 47)

Only 2 existing integration test files (`cache_stability.rs`, `system_prompt_snapshot.rs`). Missing: multi-layer assembly, knowledge injection, episode injection, playbook injection, token budget enforcement, role-specific content.

#### Implementation Steps
1. Test `SystemPromptBuilder` produces a prompt with all 9 layers present (verify layer markers or section headers)
2. Test `PromptAssemblyService::assemble()` with mock knowledge entries -- verify knowledge appears in assembled prompt
3. Test episode injection: provide mock episodes with prior failures, verify failure context appears
4. Test playbook injection: provide matching playbook, verify guidance appears
5. Test tool instructions: configure tool profiles, verify instructions in output
6. Test `SectionEffectivenessRegistry` weighting: high-lift sections get more token budget
7. Test token budget enforcement: set budget to 1000 tokens, verify assembled prompt is within budget
8. Test template rendering for each role: implementer, reviewer, strategist, researcher, tester -- verify role-specific content differs

#### Verification Criteria
- [ ] 8+ new tests, all passing
- [ ] All 9 prompt layers verified
- [ ] Token budget enforcement tested
- [ ] `cargo test -p roko-compose -- compose_subsystem` runs in < 10 seconds

---

### Task 15.11: Orchestrator DAG and executor tests
**Priority**: P1
**Estimated Effort**: 3 hours
**Files to Create**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-orchestrator/tests/orchestrator_subsystem.rs`
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-orchestrator/Cargo.toml` (add `roko-test-harness` dev-dep)
**Depends On**: Task 15.1

#### Context
Key types:
- `ParallelExecutor` at `/Users/will/dev/nunchi/roko/roko/crates/roko-orchestrator/src/executor/mod.rs` (line 241)
- DAG module at `/Users/will/dev/nunchi/roko/roko/crates/roko-orchestrator/src/dag.rs` (2,557 LOC)
- TOML fence stripping at `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/task_parser.rs` (line 964, `extract_toml_payload`)

Only 1 existing integration test file (`lifecycle.rs`). Missing: DAG construction, topological ordering, cycle detection, parallel readiness, plan validation, state persistence.

#### Implementation Steps
1. Test DAG construction from a 5-task TOML with dependencies
2. Test topological ordering: tasks with no dependencies come first
3. Test parallel readiness: tasks A and B (no deps) are both ready; task C (depends on A) is not ready until A completes
4. Test cycle detection: circular dependencies (A->B->C->A) produce error
5. Test plan validation: missing required fields (`id`, `title`) produce specific errors
6. Test TOML fence stripping via `extract_toml_payload()`: input with markdown code fences (` ```toml ... ``` `) strips fences correctly
7. Test state persistence: executor saves progress to JSON, reloads, resumes from correct task
8. Test resume fingerprint: modified tasks.toml after snapshot produces fingerprint mismatch

#### Verification Criteria
- [ ] 8+ new tests, all passing
- [ ] DAG ordering, parallelism, cycle detection all covered
- [ ] State persistence roundtrip verified
- [ ] `cargo test -p roko-orchestrator -- orchestrator_subsystem` runs in < 10 seconds

---

## Phase 3: End-to-End CLI Smoke Tests (Tasks 15.12-15.16)

Full CLI binary tests using `CliRunner`. Every test spawns `roko` as a subprocess. No API keys required.

**Dependencies**: Phase 1 (Tasks 15.1-15.2)

### Task 15.12: CLI init and config smoke tests
**Priority**: P0
**Estimated Effort**: 3 hours
**Files to Create**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/tests/init_config_smoke.rs`
**Depends On**: Tasks 15.1, 15.2

#### Implementation Steps
1. Test `roko init <tmpdir>`: creates `.roko/`, `roko.toml`, `.roko/engrams.jsonl`, `.roko/learn/`
2. Test `roko init --demo <tmpdir>`: seeds demo data (additional files/directories)
3. Test `roko config show` outputs valid TOML (contains `[agent]` section)
4. Test `roko config path` outputs the config file path (non-empty, ends with `roko.toml`)
5. Test `roko config providers list` shows configured providers
6. Test `roko config models list` shows configured models
7. Test `roko config validate` on a valid config returns success (exit 0)
8. Test `roko config validate` on an invalid config returns error with validation message
9. Test `roko doctor` runs without panic and reports status
10. Test `roko --version` outputs version string matching `roko X.Y.Z`

#### Verification Criteria
- [ ] 10 tests, all passing
- [ ] Every test uses isolated tempdir
- [ ] Tests pass without API keys
- [ ] Tests complete in < 30 seconds total

---

### Task 15.13: CLI plan lifecycle smoke tests
**Priority**: P0
**Estimated Effort**: 3 hours
**Files to Create**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/tests/plan_lifecycle_smoke.rs`
**Depends On**: Tasks 15.1, 15.2

#### Implementation Steps
1. Test `roko plan create <name>` creates plan directory with plan.md
2. Test `roko plan list` shows created plans
3. Test `roko plan show <name>` displays plan details
4. Test `roko plan validate <dir>` on valid tasks.toml returns success
5. Test `roko plan validate <dir>` on invalid TOML returns parse error
6. Test `roko plan validate <dir>` on TOML with missing required fields returns specific error
7. Test `roko plan validate <dir>` on TOML with circular dependencies returns cycle error
8. Test `roko plan validate <dir>` on TOML wrapped in markdown fences strips fences and validates

#### Verification Criteria
- [ ] 8 tests, all passing
- [ ] Plan validation catches all known error classes
- [ ] Markdown fence stripping works
- [ ] Tests complete in < 20 seconds total

---

### Task 15.14: CLI knowledge and learn smoke tests
**Priority**: P1
**Estimated Effort**: 3 hours
**Files to Create**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/tests/knowledge_learn_smoke.rs`
**Depends On**: Tasks 15.1, 15.2

#### Implementation Steps
1. Test `roko knowledge stats` on empty store returns zero counts
2. Test `roko knowledge query "test"` on empty store returns no results (not crash)
3. Test `roko learn all` on empty learn directory returns empty state
4. Test `roko learn router` on empty cascade-router.json returns defaults
5. Test `roko learn experiments` on empty experiments.json returns empty
6. Test `roko learn efficiency` on empty efficiency.jsonl returns empty
7. Test `roko learn episodes` on empty episodes.jsonl returns empty
8. Pre-seed `.roko/learn/efficiency.jsonl` with 3 events, verify `roko learn efficiency` parses them
9. Pre-seed `.roko/episodes.jsonl` with 3 episodes, verify `roko learn episodes` shows counts

#### Verification Criteria
- [ ] 9 tests, all passing
- [ ] All learn subcommands tested with both empty and seeded data
- [ ] No panics on missing/empty files
- [ ] Tests complete in < 15 seconds total

---

### Task 15.15: CLI explain, status, and utility smoke tests
**Priority**: P1
**Estimated Effort**: 2 hours
**Files to Create**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/tests/misc_commands_smoke.rs`
**Depends On**: Tasks 15.1, 15.2

#### Implementation Steps
1. Test `roko explain agent` outputs concept explanation (non-empty stdout)
2. Test `roko explain gate` outputs gate concept explanation
3. Test `roko explain agent --depth deep` produces longer output than default
4. Test `roko status` on initialized workspace shows signal/episode counts
5. Test `roko status --surfaces` shows surface inventory
6. Test `roko history list` on workspace with no sessions returns empty (not crash)
7. Test `roko completions bash` outputs bash completion script (contains `_roko` or `roko`)
8. Test `roko completions zsh` outputs zsh completion script

#### Verification Criteria
- [ ] 8 tests, all passing
- [ ] Explain depth levels (brief, standard, deep) tested
- [ ] Completion scripts are non-empty
- [ ] Tests complete in < 10 seconds total

---

### Task 15.16: CLI error handling smoke tests
**Priority**: P0
**Estimated Effort**: 3 hours
**Files to Create**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/tests/error_handling_smoke.rs`
**Depends On**: Tasks 15.1, 15.2

#### Context
Addresses AP-UNREACHABLE: the `unreachable!()` in `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/commands/config_cmd.rs:197-209` for config MCP/experiments/plugins/secrets dispatch.

#### Implementation Steps
1. Test `roko run "hello"` without init returns meaningful error (not panic, not raw Rust error)
2. Test `roko plan run nonexistent/` returns "directory not found" or similar error
3. Test `roko config show` without roko.toml returns "not initialized" error
4. Test `roko prd list` without `.roko/prd/` returns empty list (not crash)
5. Test `roko knowledge stats` without `.roko/` returns error or empty
6. Test invalid subcommand returns usage help (exit code 2)
7. Test `roko run` without prompt argument returns argument error
8. Test `roko config mcp list` does NOT panic with `unreachable!()` (AP-UNREACHABLE)

#### Verification Criteria
- [ ] 8 tests, all passing
- [ ] No test produces a Rust panic backtrace
- [ ] Every error has a human-readable message
- [ ] Tests complete in < 15 seconds total

---

## Phase 4: Gate Parity Tests (Tasks 15.17-15.19)

Verify that `GateService` (new) and legacy `run_rung()` dispatch produce equivalent verdicts.

**Dependencies**: Phase 1 (Tasks 15.1, 15.3), Phase 2 (Task 15.4)

### Task 15.17: Gate verdict equivalence tests
**Priority**: P0
**Estimated Effort**: 4 hours
**Files to Create**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-gate/tests/gate_parity.rs`
**Depends On**: Tasks 15.3, 15.4

#### Implementation Steps
1. Create a `GateTestProject` that passes all gates
2. Run through `GateService::run_gates()` -- capture verdicts
3. Run through legacy `run_rung()` / `run_canonical_rung()` path if accessible -- capture verdicts
4. Assert: same pass/fail result per gate for compile, clippy, test
5. Assert: test counts match for TestGate
6. Create a `GateTestProject` that fails compile -- verify both paths report failure
7. Create a `GateTestProject` that fails clippy -- verify both paths report clippy failure
8. Assert: gate ordering is identical (rung 0 before rung 1 before rung 2)
9. Assert: duration is within reasonable bounds (not pathologically different)

#### Verification Criteria
- [ ] Verdicts match for rungs 0-2 across both passing and failing scenarios
- [ ] Gate ordering is identical
- [ ] Duration measurements are within 2x of each other

---

### Task 15.18: Adaptive threshold parity tests
**Priority**: P1
**Estimated Effort**: 3 hours
**Files to Create**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-gate/tests/threshold_parity.rs`
**Depends On**: Task 15.4

#### Context
`AdaptiveThresholds` at `/Users/will/dev/nunchi/roko/roko/crates/roko-gate/src/adaptive_threshold.rs` (line 168, 957 LOC). Uses EMA, CUSUM, and SPC detectors to decide whether to skip gates that consistently pass.

#### Implementation Steps
1. Create `AdaptiveThresholds::new()`
2. Feed 25 consecutive passes on rung 1 (clippy), verify `should_skip_rung(1)` returns true (or the skip threshold is documented)
3. Feed 1 failure on rung 1, verify skip decision resets (consecutive streak broken)
4. Save to JSON, reload, verify state is identical (EMA, CUSUM, streak all preserved)
5. Test temperament adjustments: Conservative never skips, Aggressive skips earlier
6. Test role-based overrides: high `gate_pass_rate_floor` prevents skipping
7. Test threshold updates: observe 100 outcomes, verify EMA converges
8. Test SPC detector integration: `SpcDetector` at `/Users/will/dev/nunchi/roko/roko/crates/roko-gate/src/spc.rs` (line 473) feeds observations, detects change points

#### Verification Criteria
- [ ] Skip decision matches expected behavior at documented streak thresholds
- [ ] JSON roundtrip preserves all state
- [ ] Temperament and role overrides work

---

### Task 15.19: Gate feedback parity tests
**Priority**: P1
**Estimated Effort**: 3 hours
**Files to Create**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-gate/tests/feedback_parity.rs`
**Depends On**: Task 15.4

#### Context
`feedback_for_agent()` at `/Users/will/dev/nunchi/roko/roko/crates/roko-gate/src/feedback.rs` (line 202) produces `GateFeedback` (line 53) with `errors`, `warnings`, `suggestions` fields.

`classify_gate_failure()` at `/Users/will/dev/nunchi/roko/roko/crates/roko-gate/src/compile_errors.rs` (line 491) maps output to `FailureClass` variants (line 39).

#### Implementation Steps
1. Feed compile error output through `feedback_for_agent()`, verify `errors` contains error-classified lines
2. Verify `warnings` contains warning-classified lines
3. Verify `suggestions` contains help/note lines
4. Test noise filtering: cargo progress lines (Downloading, Compiling, Checking, Fresh, Running) are stripped
5. Test fallback: non-empty output with no classified lines produces at least one error entry
6. Test empty output: produces empty feedback (not crash)
7. Test `classify_gate_failure()` maps to correct `FailureClass`:
   - Syntax error -> `SyntaxError`
   - Missing import -> `ImportError`
   - Type mismatch -> `TypeError`
   - Borrow error -> `BorrowOrLifetime`
8. Test every `FailureClass` variant has at least one exercised input

#### Verification Criteria
- [ ] Every `FailureClass` variant has at least one test input
- [ ] Noise filtering strips all known cargo progress patterns
- [ ] Feedback structure is correct for each error category

---

## Phase 5: Benchmark Regression Suite (Tasks 15.20-15.23)

Build and wire benchmark infrastructure for performance tracking and agent evaluation.

**Dependencies**: Phase 1 (Tasks 15.1-15.3), Phase 2 (Tasks 15.5, 15.6)

### Task 15.20: SWE-bench proxy smoke tests
**Priority**: P0
**Estimated Effort**: 4 hours
**Files to Create**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/tests/bench_smoke.rs`
**Depends On**: Tasks 15.1, 15.2

#### Context
Existing SWE-bench proxy harness at `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/bench.rs` defines `SweBenchOptions`, `SweAgentMode`, `SweBenchReport`. The built-in smoke dataset has 2 tiny tasks. The `SweAgentMode::Gold` path applies gold patches and validates plumbing.

#### Implementation Steps
1. Test `SweAgentMode::Gold` (plumbing validation): apply gold patch, run tests, verify pass
2. Test `SweAgentMode::Empty` (negative control): apply empty patch, verify fail
3. Test `SweAgentMode::PredictionFile`: write a JSONL predictions file with a valid patch, verify parsing and patch application
4. Test scoring: pass/fail/error counts in `SweBenchReport` are correct
5. Test learning integration: verify episodes are written after bench run (check `.roko/episodes.jsonl` or learn dir)
6. Test batch execution: run 2 tasks, verify both produce `BenchInstanceResult`
7. Test cost tracking: verify `BenchInstanceResult.cost_usd` field is present (even if 0.0 for mock)

#### Verification Criteria
- [ ] 7 tests, all passing
- [ ] Gold mode passes, Empty mode fails (validates test integrity)
- [ ] JSONL prediction file parsing handles well-formed input
- [ ] `cargo test -p roko-cli -- bench_smoke` runs in < 60 seconds

---

### Task 15.21: Performance baseline capture
**Priority**: P1
**Estimated Effort**: 4 hours
**Files to Create**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/tests/perf_baselines.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/tests/fixtures/perf_baselines.json`
**Depends On**: Tasks 15.1, 15.3

#### Context
This task establishes measurable performance baselines for critical paths. All measurements use `std::time::Instant` for wall-clock timing. Thresholds are generous (2-3x expected) to avoid CI flakiness while still catching genuine regressions.

#### Implementation Steps
1. Test gate pipeline latency: run compile+clippy+test on a minimal `GateTestProject`, assert total time < 30 seconds
2. Test prompt assembly latency: assemble a 9-layer system prompt via `SystemPromptBuilder` with mock data, assert time < 100ms
3. Test episode logger throughput: write 1000 episodes via `EpisodeLogger`, assert time < 1 second
4. Test TOML parsing throughput: parse a 50-task plan TOML (construct programmatically), assert time < 50ms
5. Test config loading time: load a full `roko.toml` with providers and models, assert time < 50ms
6. Test state persistence roundtrip: save/load a 100-task executor state, assert < 100ms
7. Write baseline values to `perf_baselines.json` fixture file; tests print actual measurements

#### Design Guidance
Baselines should be stored as a JSON map: `{ "gate_pipeline_ms": 30000, "prompt_assembly_ms": 100, ... }`. Each test reads the baseline, measures, prints actual vs baseline, and asserts within threshold. The `#[ignore]` attribute should NOT be used -- these tests must run in CI to catch regressions early. Thresholds should be generous enough for CI runners.

#### Verification Criteria
- [ ] 6+ tests, all passing with generous initial baselines
- [ ] `perf_baselines.json` committed as test fixture
- [ ] Each test prints actual measurement for CI visibility
- [ ] Tests use `std::time::Instant` (not `SystemTime`)

---

### Task 15.22: HAL harness integration stub
**Priority**: P2
**Estimated Effort**: 5 hours
**Files to Create**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/tests/hal_integration.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/hal_adapter.rs`
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/lib.rs` or `mod.rs` (add `hal_adapter` module)
**Depends On**: Tasks 15.1, 15.5, 15.6

#### Context
HAL (Holistic Agent Leaderboard) at `hal.cs.princeton.edu` evaluates agents on accuracy, cost, and reliability across 9+ benchmarks. Roko's integration wraps the Rust CLI binary in a format that HAL's Python harness can invoke. This task builds the Rust-side adapter and tests it without requiring the Python harness.

Research from `13-PERF-HAL-AND-AGENT-BENCHMARKS.md` shows:
- HAL evaluates across 4 dimensions: accuracy, cost, reliability, safety
- Key benchmarks: SWE-bench Verified/Pro, USACO, CORE-Bench, GAIA
- HAL's reliability dashboard decomposes into consistency, robustness, predictability, safety
- Internal task replay is the most predictive benchmark for deployment

Research from `13-PERF-HAL-BENCHMARK-INTEGRATION.md` details the Python wrapper pattern: `hal/roko_agent/main.py` with `run(task, **kwargs)` signature, calling `roko run` as a subprocess.

#### Implementation Steps
1. Define `HalAgentAdapter` trait in `hal_adapter.rs`:
   ```rust
   pub trait HalAgentAdapter {
       fn initialize(&mut self, config: HalConfig) -> Result<()>;
       fn step(&mut self, task: HalTask) -> Result<HalResult>;
       fn cleanup(&mut self) -> Result<HalMetrics>;
   }
   ```
2. Define data types:
   - `HalConfig`: model, workflow, gates, timeout
   - `HalTask`: instance_id, prompt, repo (optional), base_commit (optional)
   - `HalResult`: model_patch (diff), cost_usd, tokens, duration_s, exit_code
   - `HalMetrics`: total_cost, total_tokens, total_duration, tasks_passed, tasks_failed
3. Implement `RokoHalAdapter` struct that wraps the CLI binary:
   - `initialize()` -- verifies `roko` binary exists, sets up workspace
   - `step()` -- runs `roko run` with the task prompt, captures diff and metrics
   - `cleanup()` -- aggregates metrics from all steps
4. Test `HalAgentAdapter::initialize()` creates workspace and config
5. Test `HalAgentAdapter::step(task)` dispatches to mock agent and returns structured result
6. Test `HalAgentAdapter::cleanup()` collects cost/usage metrics
7. Test HAL-compatible result format: all required fields present
8. Test multi-dimensional scoring structure: correctness, cost, latency fields populated

#### Design Guidance
The `HalAgentAdapter` trait is the extensibility point. Different adapter implementations can wrap:
- The CLI binary (for external HAL harness integration)
- The Rust API directly (for in-process benchmarking)
- A mock (for testing)

The Python wrapper (`hal/roko_agent/main.py` from the research doc) will call the CLI adapter. This task only builds the Rust types and mock-backed tests.

#### Verification Criteria
- [ ] `HalAgentAdapter` trait and types compile
- [ ] `RokoHalAdapter` passes mock-backed tests
- [ ] Result format matches HAL's expected schema (instance_id, model_patch, cost)
- [ ] Cost and latency are populated even in mock mode

---

### Task 15.23: Benchmark regression detection
**Priority**: P1
**Estimated Effort**: 3 hours
**Files to Create**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/tests/bench_regression.rs`
**Depends On**: Task 15.21

#### Context
Uses baselines from Task 15.21's `perf_baselines.json` to detect regressions. Addresses AP-BENCH-STUB by providing the baseline infrastructure that `BenchmarkRegressionGate` needs.

#### Implementation Steps
1. Load baselines from `perf_baselines.json`
2. Run each performance measurement (same as Task 15.21)
3. Compare against baseline with configurable threshold (default: 20% regression)
4. Report per-measurement: name, current value, baseline, delta percentage, pass/fail
5. Test detection: artificially inflate a baseline by 50%, verify the test catches the "regression"
6. Test threshold configuration: 0% threshold catches any increase, 100% threshold catches nothing
7. Output regression report as structured JSON for CI consumption
8. Provide a `RegressionChecker` utility struct that can be reused by `BenchmarkRegressionGate`

#### Design Guidance
The `RegressionChecker` struct encapsulates:
```rust
pub struct RegressionChecker {
    baselines: HashMap<String, f64>,
    threshold_pct: f64,
}
impl RegressionChecker {
    pub fn load(path: &Path) -> Result<Self>;
    pub fn check(&self, name: &str, current: f64) -> RegressionResult;
}
pub struct RegressionResult {
    pub name: String,
    pub baseline: f64,
    pub current: f64,
    pub delta_pct: f64,
    pub passed: bool,
}
```
This struct should live in the test harness crate so `BenchmarkRegressionGate` can eventually use it at runtime.

#### Verification Criteria
- [ ] Regression detection catches 20%+ degradation
- [ ] Threshold is configurable per measurement
- [ ] Report is machine-parseable (JSON)
- [ ] Artificial regression test verifies detection works

---

## Phase 6: Learning Persistence Tests (Tasks 15.24-15.26)

Verify learning artifacts survive write-restart-read cycles.

**Dependencies**: Phase 1 (Task 15.1), Phase 2 (Task 15.5)

### Task 15.24: Learning artifact roundtrip tests
**Priority**: P0
**Estimated Effort**: 4 hours
**Files to Create**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/tests/artifact_roundtrip.rs`
**Depends On**: Tasks 15.1, 15.5

#### Context
Addresses AP-NO-ROUNDTRIP. Every learning artifact type must survive write -> drop -> reload -> verify cycle.

#### Implementation Steps
1. Test `cascade-router.json`: write with observations across 3 models, drop the `CascadeRouter`, create new one from same file, verify observation counts preserved and routing decisions consistent
2. Test `gate-thresholds.json`: write with per-rung EMA values, reload, verify EMA and CUSUM state preserved
3. Test `section-effects.json`: write section weights, reload, verify weights and counts match
4. Test `episodes.jsonl`: append 10 episodes in one logger instance, create new logger instance on same file, append 10 more, verify all 20 present and ordered
5. Test `efficiency.jsonl`: same append-across-restarts pattern
6. Test `playbooks/`: write 3 playbook files, create new `PlaybookStore` on same directory, verify all 3 queryable
7. Test `costs.jsonl` (if exists): append cost records, reload, verify totals
8. Test affect state (if applicable): write DaimonState, reload, verify fields

#### Verification Criteria
- [ ] All artifact types survive the roundtrip
- [ ] No data loss across "process restarts" (new instances from same files)
- [ ] JSONL files support append without corrupting previous entries
- [ ] JSON files are well-formed after write

---

### Task 15.25: Learning under concurrent access
**Priority**: P1
**Estimated Effort**: 4 hours
**Files to Create**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/tests/concurrent_learning.rs`
**Depends On**: Tasks 15.1, 15.5

#### Context
Addresses AP-NO-CONCURRENT. The plan runner dispatches agents in parallel, each producing episodes and efficiency events. Concurrent writes to JSONL files and JSON state files must not corrupt data.

#### Implementation Steps
1. Spawn 10 tokio tasks, each appending 100 episodes to the same `EpisodeLogger`. Verify total count is 1000 with no corruption (every line is valid JSON)
2. Spawn 5 tokio tasks, each observing 20 routing outcomes to the same `CascadeRouter`. Verify total observations = 100
3. Test file contention: two `AdaptiveThresholds` instances observing the same file concurrently. Verify no partial writes or corrupted JSON
4. Test JSONL append atomicity: write partial data to simulate an interrupted write, verify reader skips malformed lines (not fatal error)

#### Verification Criteria
- [ ] No data corruption under concurrent access
- [ ] Total counts match expected values (no lost writes)
- [ ] Malformed JSONL lines are skipped, not fatal
- [ ] All tests use real tokio tasks (not serial simulation)

---

### Task 15.26: Learning feedback loop integration test
**Priority**: P1
**Estimated Effort**: 4 hours
**Files to Create**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/tests/feedback_loop.rs`
**Depends On**: Tasks 15.1, 15.5

#### Context
End-to-end test of the learning cycle: agent dispatch -> gate verdict -> episode recording -> routing update -> next dispatch uses updated routing. Uses `LearningRuntime` at `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/runtime_feedback.rs` (line 1243).

#### Implementation Steps
1. Create a `TestWorkspace` with clean learning state
2. Simulate 5 agent dispatches with varying outcomes:
   - Dispatch 1: model A, compile fail -> episode recorded, router observes failure
   - Dispatch 2: model B, all gates pass -> episode recorded, router observes success
   - Dispatch 3: router should now prefer model B (higher success rate)
   - Dispatch 4: model B again, test fail -> episode recorded
   - Dispatch 5: verify router adjusts (B's success rate declined)
3. After all dispatches verify:
   - `episodes.jsonl` has 5 entries
   - `cascade-router.json` has observations for both models
   - Router's recommended model reflects observed outcomes
   - `gate-thresholds.json` has per-rung observations

#### Verification Criteria
- [ ] Full loop verified: dispatch -> gate -> episode -> router -> next dispatch
- [ ] Router recommendations change based on observed outcomes
- [ ] All learning artifacts are consistent at end of loop

---

## Phase 7: Performance Regression Tests (Tasks 15.27-15.29)

**Dependencies**: Phase 1 (Tasks 15.1-15.3), Phase 5 (Task 15.21)

### Task 15.27: Memory usage regression tests
**Priority**: P1
**Estimated Effort**: 4 hours
**Files to Create**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/tests/memory_regression.rs`
**Depends On**: Tasks 15.1, 15.21

#### Context
Addresses the 9.5-11.5GB RSS leak from dogfood sessions. Unbounded vectors in efficiency events and enrichment artifacts were identified as contributors.

#### Implementation Steps
1. Test efficiency events vector is bounded: append 10000 events, verify the in-memory vector does not exceed a configured cap (e.g., 1000 entries). After flush, verify vector is cleared.
2. Test episode logger does not accumulate in memory: write 10000 episodes, verify memory stays flat (each write flushes to disk).
3. Test enrichment context is dropped after use: build enrichment for a task, verify the string is not held after dispatch context goes out of scope.
4. Test executor state serialization does not grow unboundedly: 100-task plan with all tasks completed, verify serialized state JSON is < 1MB.

#### Design Guidance
Memory measurement on macOS: use `mach_task_info` via the `mach2` crate or parse `ps -o rss` output. On Linux: parse `/proc/self/status` for VmRSS. For cross-platform: use `std::alloc::GlobalAlloc` wrapper that tracks high-water mark.

#### Verification Criteria
- [ ] No unbounded vector growth (verified by size checks, not just compilation)
- [ ] Baselines are documented in test comments
- [ ] Tests pass on both macOS and Linux

---

### Task 15.28: Startup latency regression tests
**Priority**: P1
**Estimated Effort**: 2 hours
**Files to Create**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/tests/startup_latency.rs`
**Depends On**: Tasks 15.1, 15.2

#### Implementation Steps
1. Test `roko --version` completes in < 500ms (cold start)
2. Test `roko --help` completes in < 500ms
3. Test `roko status --workdir <tmpdir>` completes in < 2s (includes config loading)
4. Test `roko config show --workdir <tmpdir>` completes in < 1s
5. Run each measurement 3 times, take median to reduce variance

#### Verification Criteria
- [ ] All measurements within baseline thresholds
- [ ] Median of 3 runs used
- [ ] Thresholds are generous for CI runners (2x local dev machine)

---

### Task 15.29: Gate pipeline throughput tests
**Priority**: P1
**Estimated Effort**: 3 hours
**Files to Create**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-gate/tests/gate_throughput.rs`
**Depends On**: Tasks 15.1, 15.3

#### Context
Tests the overhead of gate infrastructure itself, separate from the cost of running cargo commands.

#### Implementation Steps
1. Test `GateService` with 3 mock shell gates (`true`): 100 iterations, assert total time < 1s (pipeline overhead < 10ms per run)
2. Test `AdaptiveThresholds` update speed: 10000 observations, assert total time < 1s
3. Test `SpcDetector` update speed: 10000 observations, assert < 2s (BOCPD is more expensive)
4. Test `ComposedGatePipeline` with `ParallelGate(3 mock gates)`: verify parallel execution is faster than sequential
5. Test `feedback_for_agent()` parsing speed: 1000 lines of mixed compile output, assert < 100ms

#### Verification Criteria
- [ ] Pipeline overhead is measurable and bounded
- [ ] Parallel gate execution provides speedup over sequential
- [ ] Statistical detectors scale linearly with observations

---

## Phase 8: Chaos and Fault Injection Tests (Tasks 15.30-15.32)

**Dependencies**: Phase 1 (Tasks 15.1-15.3), Phase 2 (Tasks 15.4, 15.7)

### Task 15.30: Agent crash recovery tests
**Priority**: P1
**Estimated Effort**: 4 hours
**Files to Create**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/tests/agent_crash_recovery.rs`
**Depends On**: Tasks 15.1, 15.2

#### Implementation Steps
1. Test agent SIGKILL: spawn a mock agent (shell script that sleeps), send SIGKILL, verify the runner detects crash and records a failure episode
2. Test agent timeout: configure 5-second timeout, spawn agent that sleeps 10 seconds, verify timeout fires and process is killed
3. Test agent stderr output: mock agent writes to stderr, verify error output is captured in the failure context
4. Test agent invalid output: mock agent produces malformed JSON (not valid stream-json), verify parser handles it without panic
5. Test agent exit code: mock agent exits with code 1, verify appropriate error message in output
6. Test partial output recovery: agent produces valid output then crashes mid-stream, verify partial output is preserved where possible

#### Design Guidance
Use shell scripts as mock agents: `#!/bin/sh\nsleep 100` for timeout tests, `#!/bin/sh\nexit 1` for failure tests, `#!/bin/sh\necho 'not json'` for malformed output tests. Configure the mock agent via `roko.toml` `[agent] command = "/path/to/mock.sh"`.

#### Verification Criteria
- [ ] No panic in any crash scenario
- [ ] All crashes produce failure context (not silent swallow)
- [ ] Timeout is enforced (process killed, not hung)
- [ ] Partial output preserved when possible

---

### Task 15.31: Gate failure edge cases
**Priority**: P1
**Estimated Effort**: 3 hours
**Files to Create**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-gate/tests/gate_edge_cases.rs`
**Depends On**: Tasks 15.1, 15.3

#### Implementation Steps
1. Test missing cargo binary: set PATH to empty, verify `CompileGate` returns clear "cargo not found" or "command not found" error (not raw OS error)
2. Test empty project: run gates on a directory with no `Cargo.toml`, verify clear error message
3. Test very large output: mock shell gate that produces 1MB of stderr, verify output is truncated and not OOM
4. Test shell gate with non-UTF-8 output: verify output is handled (lossy conversion, not panic)
5. Test concurrent gate execution: run 3 `CompileGate` instances on the same `GateTestProject` simultaneously, verify no interference
6. Test gate timeout: configure a shell gate with 1-second timeout and a command that sleeps 5 seconds, verify timeout

#### Verification Criteria
- [ ] Every edge case produces a clear error message
- [ ] No panics, no OOM, no hangs
- [ ] Concurrent execution is safe

---

### Task 15.32: State persistence under failure
**Priority**: P1
**Estimated Effort**: 3 hours
**Files to Create**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/tests/state_persistence_chaos.rs`
**Depends On**: Tasks 15.1, 15.2

#### Implementation Steps
1. Test crash-during-save: write executor state, corrupt the file (truncate to half), verify `load_state()` detects corruption and falls back to initial state (not panic)
2. Test missing state file: verify `load_state()` on missing file returns clean initial state
3. Test state with 100 completed tasks: verify save/load roundtrip preserves all task statuses
4. Test concurrent save: two instances saving to the same file simultaneously, verify last-writer-wins (no interleaved data)
5. Test malformed JSON recovery: write `{` to state file, verify load handles gracefully

#### Verification Criteria
- [ ] Corrupted state files produce fallback, not panic
- [ ] No data loss under normal roundtrip
- [ ] Concurrent writes do not produce corrupted output

---

## Phase 9: CLI Command Known-Good Output Tests (Tasks 15.33-15.36)

Snapshot tests that capture CLI output and detect unexpected changes.

**Dependencies**: Phase 1 (Tasks 15.1-15.2)

### Task 15.33: CLI help text snapshot tests
**Priority**: P1
**Estimated Effort**: 3 hours
**Files to Create**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/tests/help_snapshots.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/tests/snapshots/` (directory)
**Depends On**: Tasks 15.1, 15.2

#### Implementation Steps
1. Generate `--help` output for each top-level command: `run`, `plan`, `prd`, `chat`, `serve`, `status`, `doctor`, `init`, `config`, `learn`, `knowledge`, `research`, `agent`, `job`, `explain`, `dashboard`, `bench`, `deploy`, `replay`, `history`, `up`
2. Generate `--help` for key subcommands: `plan list`, `plan show`, `plan create`, `plan run`, `plan validate`, `config show`, `config providers`, `config models`
3. Store snapshots in `crates/roko-cli/tests/snapshots/help/` as `.txt` files
4. Compare output against stored snapshots using simple string comparison
5. Fail on unexpected changes (forces intentional help text updates)
6. Provide an update mechanism: set `UPDATE_SNAPSHOTS=1` env var to overwrite stored snapshots

#### Design Guidance
Do NOT use `insta` (not in workspace dependencies). Use simple `assert_eq!` with file read/write. The update mechanism: if `std::env::var("UPDATE_SNAPSHOTS").is_ok()`, write current output to snapshot file and pass; otherwise, read snapshot file and compare.

#### Verification Criteria
- [ ] Every top-level command has a help snapshot
- [ ] Key subcommands have snapshots
- [ ] Adding a new flag without updating snapshots causes test failure
- [ ] `UPDATE_SNAPSHOTS=1` updates all snapshot files

---

### Task 15.34: CLI JSON output snapshot tests
**Priority**: P1
**Estimated Effort**: 3 hours
**Files to Create**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/tests/json_snapshots.rs`
**Depends On**: Tasks 15.1, 15.2

#### Implementation Steps
1. Test `roko status --json` on seeded workspace: output is valid JSON, contains expected top-level keys
2. Test `roko learn episodes --json` on seeded episodes.jsonl: output is valid JSON
3. Test `roko learn router --json` on seeded cascade-router.json: output is valid JSON
4. Test `roko config show --json` (if supported): output is valid JSON matching roko.toml structure
5. Test `roko plan list --json` on seeded plans: output is valid JSON array
6. Validate output parses with `serde_json::from_str::<Value>()` (schema validation via key presence)
7. Store expected key sets as test fixtures

#### Verification Criteria
- [ ] Every `--json` command produces valid JSON (parseable by serde_json)
- [ ] Output contains all expected fields
- [ ] No JSON output is mixed with log lines on stdout

---

### Task 15.35: CLI exit code tests
**Priority**: P0
**Estimated Effort**: 2 hours
**Files to Create**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/tests/exit_codes.rs`
**Depends On**: Tasks 15.1, 15.2

#### Implementation Steps
1. Success cases (exit 0): `roko init`, `roko --version`, `roko --help`, `roko config show` (valid config), `roko status` (initialized workspace), `roko plan validate <valid-plan>`
2. Failure cases (exit non-zero): `roko run` (no prompt), `roko plan run nonexistent/`, `roko plan validate <invalid-toml>`, `roko config show` (no roko.toml), unknown subcommand
3. Verify specific exit codes where applicable (1 for general error, 2 for usage error)

#### Verification Criteria
- [ ] All success cases exit 0
- [ ] All failure cases exit non-zero
- [ ] No command silently succeeds when it should fail

---

### Task 15.36: CLI stderr/stdout separation tests
**Priority**: P1
**Estimated Effort**: 2 hours
**Files to Create**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/tests/output_channels.rs`
**Depends On**: Tasks 15.1, 15.2

#### Implementation Steps
1. Test `roko --help`: output on stdout, nothing (or only log lines) on stderr
2. Test `roko status --json`: JSON on stdout, any logs on stderr only
3. Test `roko plan validate <invalid>`: error message on stderr, nothing meaningful on stdout
4. Test `roko config show`: config on stdout, nothing on stderr
5. Test `roko run` (no args): error on stderr, usage hint on stderr
6. Test `--quiet` flag (if exists): suppresses informational output but not errors

#### Verification Criteria
- [ ] Normal output goes to stdout
- [ ] Errors go to stderr
- [ ] JSON output on stdout is not mixed with log lines

---

## Phase 10: Extended ACP and Protocol Tests (Tasks 15.37-15.38)

**Dependencies**: Phase 2 (Task 15.8)

### Task 15.37: ACP session recovery tests
**Priority**: P1
**Estimated Effort**: 4 hours
**Files to Create**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-acp/tests/session_recovery.rs`
**Depends On**: Task 15.8

#### Context
`AcpSession` at `/Users/will/dev/nunchi/roko/roko/crates/roko-acp/src/session.rs` (line 237). The `TestHarness` pattern from `protocol_conformance.rs` uses `DuplexStream` for in-process testing.

#### Implementation Steps
1. Test session persistence: create session, add 5 turns of history, save, reload from disk, verify history intact
2. Test session resume after server restart: create session, create new `TestHarness`, load session by ID, verify context preserved
3. Test session cleanup: cancel session, verify resources are released, session ID is no longer loadable
4. Test session listing: create 3 sessions, list, verify all 3 present with correct metadata
5. Test session limit: if a `max_sessions` config exists, verify exceeding it returns appropriate error
6. Test stale session detection: create session, verify that sessions have a TTL or staleness marker

#### Verification Criteria
- [ ] Session state survives simulated server restarts
- [ ] History is preserved with exact content
- [ ] Cancelled sessions are cleaned up
- [ ] Session listing is consistent

---

### Task 15.38: ACP telemetry integration tests
**Priority**: P1
**Estimated Effort**: 4 hours
**Files to Create**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-acp/tests/telemetry_verification.rs`
**Depends On**: Task 15.8

#### Context
Extends existing `telemetry_integration.rs`. Verifies that ACP pipeline runs produce correct telemetry events (gate progress, token usage, phase transitions, file changes, cost tracking).

#### Implementation Steps
1. Test gate progress events: during a pipeline run, verify the client receives gate start, gate pass/fail, gate complete notifications
2. Test token usage events: verify the client receives token usage updates during agent streaming
3. Test phase transition events: verify the client receives events for each pipeline phase
4. Test file change events: verify the client receives file change notifications after agent writes
5. Test cost tracking events: verify the client receives cost updates with running totals
6. Test event ordering: verify events arrive in logical order (gate_start before gate_pass, phase_start before phase_end)

#### Verification Criteria
- [ ] Every event type is verified
- [ ] Event ordering is correct
- [ ] Token counts are non-negative
- [ ] Events are delivered as JSON-RPC notifications (not responses)

---

## Dependency Graph

```
Phase 1 (Tasks 15.1-15.3)
  |
  +--- Phase 2 (Tasks 15.4-15.11) [parallel within phase]
  |      |
  |      +--- Phase 4 (Tasks 15.17-15.19) [depends on 15.4]
  |      +--- Phase 5 (Tasks 15.20-15.23) [depends on 15.5, 15.6]
  |      +--- Phase 6 (Tasks 15.24-15.26) [depends on 15.5]
  |      +--- Phase 8 (Tasks 15.30-15.32) [depends on 15.4, 15.7]
  |      +--- Phase 10 (Tasks 15.37-15.38) [depends on 15.8]
  |
  +--- Phase 3 (Tasks 15.12-15.16) [parallel within phase]
  |
  +--- Phase 7 (Tasks 15.27-15.29) [depends on 15.1-15.3, 15.21]
  |
  +--- Phase 9 (Tasks 15.33-15.36) [depends on 15.1-15.2]
```

Phases 2, 3, 7, 9 can all start once Phase 1 completes.
Phases 4, 5, 6, 8, 10 require specific Phase 2 tasks.

---

## Execution Estimates

| Phase | Tasks | Estimated Days | Parallelizable |
|---|---|---|---|
| 1: Harness | 15.1-15.3 | 2 | No (sequential) |
| 2: Subsystem integration | 15.4-15.11 | 4-5 | Yes (8 tasks, all independent) |
| 3: E2E smoke | 15.12-15.16 | 2-3 | Yes (5 tasks, all independent) |
| 4: Gate parity | 15.17-15.19 | 1-2 | Partially (15.17 first) |
| 5: Benchmarks | 15.20-15.23 | 2-3 | Yes (mostly independent) |
| 6: Learning persistence | 15.24-15.26 | 1-2 | Partially (15.24 first) |
| 7: Performance regression | 15.27-15.29 | 1-2 | Yes (all independent) |
| 8: Chaos/fault | 15.30-15.32 | 2 | Yes (all independent) |
| 9: CLI snapshots | 15.33-15.36 | 1-2 | Yes (all independent) |
| 10: ACP extended | 15.37-15.38 | 1 | Partially |
| **Total** | **38** | **16-22** | Critical path: Phase 1 -> Phase 2 -> Phase 4 (~8 days) |

---

## Test Run Configuration

### CI Integration

All tests runnable via:

```bash
# Full suite
cargo test --workspace

# Per-phase
cargo test -p roko-test-harness                    # Phase 1
cargo test -p roko-gate -- gate_subsystem          # Phase 2, Task 15.4
cargo test -p roko-learn -- learning_subsystem     # Phase 2, Task 15.5
cargo test -p roko-cli -- smoke                    # Phase 3
cargo test -p roko-gate -- parity                  # Phase 4
cargo test -p roko-cli -- bench_smoke              # Phase 5
cargo test -p roko-learn -- roundtrip              # Phase 6
cargo test -p roko-cli -- perf                     # Phase 7
cargo test -p roko-gate -- edge_cases              # Phase 8
cargo test -p roko-cli -- snapshot                 # Phase 9
cargo test -p roko-acp -- conformance              # Phase 10
```

### Test Markers

Tests requiring external resources use `#[ignore]`:
- Tests requiring real API keys (live provider tests)
- Tests requiring large datasets (SWE-bench full)
- Tests with long execution times (> 60 seconds)
- Tests requiring Docker or network

### No-API-Key Guarantee

All 38 tasks in this plan are designed to work WITHOUT API keys. They use mock agents, mock providers, and canned responses. CI runs the full suite without credential configuration.

---

## Success Criteria

The testing infrastructure is complete when:

1. **Coverage**: Every crate has at least one integration test file (target: 18/18 crates, currently 14/18)
2. **Gate parity**: `GateService` produces equivalent verdicts to legacy paths for rungs 0-2
3. **CLI stability**: Every CLI subcommand has at least one test verifying no crash
4. **Learning integrity**: Every learning artifact type survives write-restart-read cycle
5. **Performance baselines**: Regression detection catches 20%+ degradation in critical paths
6. **Chaos resilience**: Agent crashes, gate timeouts, state corruption produce graceful recovery
7. **CI green**: `cargo test --workspace` passes with all 38 tasks implemented
