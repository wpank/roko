# Testing and Verification Infrastructure: Implementation Plan

> Comprehensive test suite to verify every subsystem, detect regressions,
> enforce protocol conformance, and benchmark agent performance. Each task
> includes exact file paths, specific acceptance criteria, and dependency
> ordering. Tasks are sized for agent execution (1-4 hours each).
>
> Verified against source on branch `wp-arch2`, 2026-04-29. Existing test
> inventory: 85 integration test files across 14 crates, ~95 inline test
> modules. Gaps identified from docs 05, 11, 13, 14, and 20.

---

## Scope

| Category | Tasks | Coverage Target |
|---|---|---|
| Test harness and helpers | 3 | Shared infrastructure for all test categories |
| Per-subsystem integration tests | 8 | Gate, learn, agent, runtime, ACP, serve, compose, orchestrator |
| End-to-end smoke tests | 5 | CLI commands, config lifecycle, plan lifecycle |
| Gate parity tests | 3 | Old (orchestrate.rs) vs new (GateService) verdict equivalence |
| Benchmark regression suite | 4 | SWE-bench proxy, HAL integration, perf baselines |
| CLI command tests | 4 | Known-good output for every major subcommand |
| Learning persistence tests | 3 | Write-read-update cycles for all learning artifacts |
| ACP protocol conformance tests | 2 | JSON-RPC spec compliance, session lifecycle |
| Performance regression tests | 3 | Latency, memory, throughput baselines |
| Chaos and fault injection tests | 3 | Agent crash, gate timeout, disk full, concurrent access |
| **Total** | **38** | |

---

## PHASE 1: Test Harness and Infrastructure (Tasks 1-3)

These tasks build shared infrastructure used by all subsequent test phases.
No other phase should start until Phase 1 is complete.

### Task 1: Build unified test harness crate

**File**: `crates/roko-test-harness/` (new crate)
**What**: Create a shared test-support crate with builders, mock providers,
temp workspace scaffolding, and assertion helpers. Consolidates patterns
currently duplicated across `crates/roko-cli/tests/common/mod.rs`,
`crates/roko-serve/tests/api_integration.rs` (TestRuntime), and
`crates/roko-gate/tests/` helpers.

**Steps**:
1. Create `crates/roko-test-harness/Cargo.toml` with `[lib]` only (no binary).
   Dependencies: `tempfile`, `assert_cmd`, `serde_json`, `tokio`, `roko-core`,
   `roko-agent`, `roko-learn`, `roko-gate`, `roko-compose`, `roko-runtime`.
2. Create `crates/roko-test-harness/src/lib.rs` with modules:
   - `workspace` -- `TestWorkspace::new()` creates tempdir with `.roko/` layout,
     `roko.toml`, `engrams.jsonl`, `learn/` subdirectories. Returns typed handle.
   - `mock_agent` -- `MockAgent` implementing `ProviderAdapter` that returns
     canned responses. Configurable: success/failure, token counts, delay.
   - `mock_gate` -- `MockGate` implementing `Verify` with configurable verdicts.
   - `fixtures` -- Sample `Episode`, `RoutingContext`, `GateConfig`, `TasksFile`,
     plan TOML, and `roko.toml` templates as constants.
   - `assertions` -- `assert_jsonl_has_field(path, field, value)`,
     `assert_file_has_n_lines(path, n)`, `assert_gate_verdicts(report, expected)`,
     `assert_episode_fields(episode, checks)`.
3. Add `roko-test-harness` to workspace `Cargo.toml` members.
4. Add `roko-test-harness` as `[dev-dependencies]` in `roko-cli`, `roko-serve`,
   `roko-gate`, `roko-learn`, `roko-runtime`, `roko-acp`.

**Acceptance criteria**:
- `cargo test -p roko-test-harness` passes (self-tests for each builder)
- `TestWorkspace::new()` creates all `.roko/learn/` subdirs
- `MockAgent` returns configurable responses with token counts
- No existing tests break from the new dev-dependency

### Task 2: Build CLI binary test runner

**File**: `crates/roko-test-harness/src/cli_runner.rs`
**What**: Typed wrapper around `assert_cmd::Command` for the `roko` binary
that handles workspace setup, environment isolation, and output capture.

**Steps**:
1. Create `CliRunner` struct wrapping `assert_cmd::Command::cargo_bin("roko")`
2. Methods:
   - `with_workspace(workspace: &TestWorkspace)` -- sets `--workdir`
   - `with_config(toml: &str)` -- writes config to workspace roko.toml
   - `run_init()` -- executes `roko init` and asserts success
   - `run_cmd(args: &[&str])` -- executes arbitrary subcommand
   - `run_and_capture(args: &[&str]) -> CapturedOutput` -- captures stdout/stderr/exit
   - `assert_success(args: &[&str])` -- runs and asserts exit 0
   - `assert_failure(args: &[&str])` -- runs and asserts non-zero exit
   - `assert_output_contains(args: &[&str], pattern: &str)` -- output check
3. `CapturedOutput` struct: stdout, stderr, exit_code, duration
4. Environment isolation: unset `ANTHROPIC_API_KEY`, `OPENAI_API_KEY`, etc.
   to prevent tests from hitting real APIs

**Acceptance criteria**:
- `CliRunner::new().run_init()` succeeds with isolated workspace
- `CliRunner::new().assert_output_contains(&["--help"], "Usage")` passes
- Environment variables are isolated between test runs

### Task 3: Build gate test scaffold

**File**: `crates/roko-test-harness/src/gate_scaffold.rs`
**What**: Utilities for testing gates against real Cargo projects without
requiring the full roko stack.

**Steps**:
1. `GateTestProject::new()` creates a tempdir with a minimal Cargo project:
   - `Cargo.toml` with `[package]` and `edition = "2021"`
   - `src/lib.rs` with a simple function and test
2. `GateTestProject::break_compile()` inserts a syntax error
3. `GateTestProject::break_clippy()` inserts a clippy-detectable issue
   (e.g., `let _ = vec![1,2,3].len() > 0;` -> `!is_empty()`)
4. `GateTestProject::break_test()` makes a `#[test]` fail
5. `GateTestProject::add_shell_gate(cmd: &str)` writes a shell script
6. `GateTestProject::workdir() -> &Path`
7. `run_gate_service(project: &GateTestProject, gates: &[&str]) -> GateReport`
   convenience function

**Acceptance criteria**:
- `GateTestProject::new()` compiles with `cargo check`
- `break_compile()` causes `CompileGate` to fail
- `break_clippy()` causes `ClippyGate` to fail
- `break_test()` causes `TestGate` to fail
- All three `break_*` functions are independently reversible

---

## PHASE 2: Per-Subsystem Integration Tests (Tasks 4-11)

Each task adds integration tests for one subsystem. Tests use the harness
from Phase 1. Tasks 4-11 can run in parallel.

**Dependencies**: Phase 1 (Tasks 1-3)

### Task 4: Gate subsystem integration tests

**File**: `crates/roko-gate/tests/gate_subsystem.rs`
**What**: Comprehensive gate tests covering all 16 gate types, composition
wrappers, and the unified GateService. Extends existing `gate_truth.rs`
(6 tests) and `rungs.rs` (9 tests) with coverage for untested gates.

**Steps**:
1. Test `GateService` rung ordering: configure gates out of order, verify
   they execute in canonical rung order (compile=0, clippy=1, test=2, etc.)
2. Test `GateService` short-circuit: first failure stops pipeline
3. Test `GateService` adaptive skip: after 20+ consecutive passes on clippy,
   verify `should_skip_rung(1)` returns true and clippy is skipped
4. Test compile-never-skip: even with 100 consecutive passes on compile,
   `should_skip_rung(0)` returns false
5. Test `ShellGate` with custom commands: success and failure cases
6. Test `DiffGate` detects stub/vacuous changes (no-op diff)
7. Test `FormatCheckGate` detects formatting violations
8. Test `ParallelGate` runs inner gates concurrently and takes minimum score
9. Test `VotingGate` with configurable threshold (2-of-3, 3-of-3)
10. Test `FallbackGate` tries primary, falls back on failure
11. Test `ComposedGatePipeline` in all 4 modes (Sequential, Parallel, Voting, Fallback)
12. Test `BenchmarkRegressionGate` stub behavior (currently always passes)
13. Test gate feedback generation: verify `GateFeedback` has structured
    errors, warnings, suggestions from compile output

**Acceptance criteria**:
- 15+ new tests, all passing
- Every gate type in `crates/roko-gate/src/` has at least one test
- Composition modes all tested with mock inner gates
- `cargo test -p roko-gate` runs in < 60 seconds

### Task 5: Learning subsystem integration tests

**File**: `crates/roko-learn/tests/learning_subsystem.rs`
**What**: Tests for all learning artifact types: write, read, update, persist.
Extends existing `learning_loop.rs` (4 tests) and `cascade_router_integration.rs`.

**Steps**:
1. Test `EpisodeLogger`: write 100 episodes, read back, verify ordering and
   field integrity. Test concurrent appends from multiple tokio tasks.
2. Test `CascadeRouter`: load_or_new, observe 50 outcomes, save, reload,
   verify observation counts and routing decisions change.
3. Test `SectionEffectivenessRegistry`: record positive/negative signals
   for 10 sections, verify weights shift, persist and reload.
4. Test `PlaybookStore`: write 5 playbooks, query by role/category, verify
   matching returns correct playbooks.
5. Test `EfficiencyWriter`: append events, verify flush writes to disk
   immediately (not buffered to process exit).
6. Test `CFactorSummary`: compute from episode history, verify composite
   score is in [0, 1] range.
7. Test `ErrorPatternStore`: record 10 error patterns, query by pattern,
   verify deduplication.
8. Test `DriftDetector`: feed declining pass rates, verify drift alert fires.
9. Test `LatencyRegistry`: record latencies, verify percentile computation.
10. Test cross-artifact consistency: episodes written -> cascade router
    observes -> section effects updated -> playbooks generated.

**Acceptance criteria**:
- 12+ new tests, all passing
- Every artifact type in `.roko/learn/` has write-read-update coverage
- Concurrent access tests pass without data corruption
- `cargo test -p roko-learn` runs in < 30 seconds

### Task 6: Agent dispatch integration tests

**File**: `crates/roko-agent/tests/dispatch_subsystem.rs`
**What**: Tests for agent dispatch through `ModelCallService` and
`create_agent_for_model()` with mock backends.

**Steps**:
1. Test `create_agent_for_model()` returns correct agent type for each
   `ProviderKind`: ClaudeCli, ClaudeApi, OpenAiCompat, Ollama, Gemini,
   Perplexity, Cerebras.
2. Test `ModelCallService` construction via `ServiceFactory::build()` with
   mock config. Verify all services are constructed (feedback, gateway events,
   knowledge store, cascade router, prompt assembly).
3. Test `ClaudeCliAgent::build_command()` includes all expected flags:
   `--print`, `--verbose`, `--output-format stream-json`, `--model`,
   `--effort`, `--settings`, `--append-system-prompt`.
4. Test settings JSON generation: verify safety hooks block `git checkout`,
   `git push`, `rm -rf`.
5. Test `AgentOptions` builder: model, effort, tools, MCP config, fallback
   model all set correctly.
6. Test timeout handling: mock agent that takes longer than configured
   timeout, verify clean shutdown.
7. Test tool allowlist enforcement: configure allowed tools, verify settings
   JSON only includes those tools.

**Acceptance criteria**:
- 8+ new tests, all passing
- Every `ProviderKind` variant has at least one construction test
- Safety hook coverage: all blocked commands verified
- `cargo test -p roko-agent` (excluding live API tests) runs in < 30 seconds

### Task 7: Runtime and workflow engine integration tests

**File**: `crates/roko-runtime/tests/workflow_subsystem.rs`
**What**: Tests for `WorkflowEngine`, `EffectDriver`, `PipelineStateV2`,
and `EventBus` integration.

**Steps**:
1. Test `PipelineStateV2` state machine transitions: Start -> StrategyPhase ->
   ImplementPhase -> GatePhase -> ReviewPhase -> CommitPhase -> Done.
2. Test `PipelineStateV2` error paths: agent failure at ImplementPhase
   triggers retry up to `max_autofix_attempts`.
3. Test `PipelineStateV2` gate failure: GatesFailed input triggers
   autofix iteration or failure.
4. Test `EventBus` publish-subscribe: register consumer, publish
   RuntimeEvent, verify consumer receives it.
5. Test `EventBus` with multiple consumers: verify fan-out.
6. Test `JsonlLogger` writes events to disk in JSONL format.
7. Test `ProcessSupervisor` tracks spawned processes and kills on cancel.
8. Test `WorkflowEngine` end-to-end with mock `EffectDriver`: prompt in,
   mock agent responds, mock gates pass, Done produced.

**Acceptance criteria**:
- 8+ new tests, all passing
- Every `PipelineStateV2` state transition is covered
- EventBus fan-out works with 3+ consumers
- `cargo test -p roko-runtime` runs in < 15 seconds

### Task 8: ACP protocol conformance tests

**File**: `crates/roko-acp/tests/acp_conformance.rs`
**What**: Extends existing `protocol_conformance.rs` (which tests basic
JSON-RPC) with full ACP session lifecycle and edge case coverage.

**Steps**:
1. Test session create -> load -> list -> cancel lifecycle.
2. Test mode switching: code -> plan -> research modes produce different
   system prompts.
3. Test config updates: model, effort, temperament, gates, workflow,
   review_strictness all modifiable mid-session.
4. Test slash command handling: `/model`, `/effort`, `/status`, `/diff`,
   `/plan` all produce local responses (no agent dispatch).
5. Test conversation history: multi-turn history accumulates, trimming
   kicks in at 40 turns / 64K chars.
6. Test cancellation: start a pipeline run, cancel mid-execution, verify
   cooperative shutdown.
7. Test error codes: `SESSION_NOT_FOUND` for invalid session IDs,
   `METHOD_NOT_FOUND` for unknown methods, `PARSE_ERROR` for malformed JSON.
8. Test protocol version negotiation: client sends protocolVersion 1,
   server responds with supported version.
9. Test notification delivery: server sends notifications for gate progress,
   file changes, phase transitions.

**Acceptance criteria**:
- 10+ new tests, all passing
- Every JSON-RPC error code exercised
- Session lifecycle fully covered (create, use, resume, cancel, cleanup)
- `cargo test -p roko-acp` runs in < 20 seconds

### Task 9: Serve HTTP API integration tests

**File**: `crates/roko-serve/tests/serve_subsystem.rs`
**What**: Extends existing `api_integration.rs` with route coverage for
major endpoint groups. Uses `tower::ServiceExt::oneshot` (no real HTTP server).

**Steps**:
1. Test status routes: `GET /api/status` returns valid JSON with expected fields.
2. Test plan routes: `POST /api/plans`, `GET /api/plans`, `GET /api/plans/:id`.
3. Test PRD routes: `GET /api/prds`, `POST /api/prds`, `GET /api/prds/:slug`.
4. Test agent routes: `GET /api/agents`, `POST /api/agents`, agent lifecycle.
5. Test job routes: `GET /api/jobs`, `POST /api/jobs`, `GET /api/jobs/:id`.
6. Test config routes: `GET /api/config`, `GET /api/config/providers`,
   `GET /api/config/models`.
7. Test learning routes: `GET /api/learning/episodes`, `GET /api/learning/router`,
   `GET /api/learning/efficiency`.
8. Test auth middleware: requests without API key return 401 when auth enabled.
9. Test SSE endpoint: `GET /api/events` returns SSE stream with `event:` prefix.
10. Test WebSocket endpoint: connect and verify handshake.
11. Test OpenAPI spec: `GET /api/openapi.json` returns valid OpenAPI 3.x.

**Acceptance criteria**:
- 12+ new tests, all passing
- Every major route group (agents, plans, prds, jobs, config, learning)
  has at least one test
- Auth middleware tested in both enabled and disabled modes
- `cargo test -p roko-serve` runs in < 30 seconds

### Task 10: Compose and prompt assembly tests

**File**: `crates/roko-compose/tests/compose_subsystem.rs`
**What**: Tests for `PromptAssemblyService`, template rendering, and
section effectiveness weighting.

**Steps**:
1. Test `PromptAssemblyService::assemble()` produces a system prompt
   with all 9 layers present.
2. Test knowledge injection: provide mock knowledge entries, verify they
   appear in assembled prompt.
3. Test episode injection: provide mock episodes, verify prior failure
   context appears in prompt.
4. Test playbook injection: provide matching playbook, verify guidance
   appears in prompt.
5. Test tool instructions: configure tool profiles, verify instructions
   in assembled prompt.
6. Test section effectiveness weighting: high-lift sections get more token
   budget, low-lift sections get less.
7. Test token budget enforcement: set budget to 1000 tokens, verify
   assembled prompt is within budget.
8. Test template rendering for each role: implementer, reviewer, strategist,
   researcher, tester. Verify role-specific content.

**Acceptance criteria**:
- 8+ new tests, all passing
- All 9 prompt layers verified
- Token budget enforcement tested
- `cargo test -p roko-compose` runs in < 10 seconds

### Task 11: Orchestrator DAG and executor tests

**File**: `crates/roko-orchestrator/tests/orchestrator_subsystem.rs`
**What**: Tests for `ParallelExecutor`, DAG dependency resolution, plan
loading, and state persistence.

**Steps**:
1. Test DAG construction from tasks.toml with dependencies.
2. Test topological ordering: tasks with no dependencies come first.
3. Test parallel readiness: tasks A and B with no deps are both ready;
   task C depending on A is not ready until A completes.
4. Test cycle detection: circular dependencies produce error.
5. Test plan validation: missing required fields produce specific errors.
6. Test TOML fence stripping: `extract_toml_payload()` handles markdown
   code fences (` ```toml ... ``` `).
7. Test state persistence: executor saves to JSON, reloads, and resumes
   from the correct task.
8. Test resume fingerprint validation: modified tasks.toml after snapshot
   produces fingerprint mismatch warning.

**Acceptance criteria**:
- 8+ new tests, all passing
- DAG ordering, parallelism, cycle detection all covered
- State persistence roundtrip verified
- `cargo test -p roko-orchestrator` runs in < 10 seconds

---

## PHASE 3: End-to-End Smoke Tests (Tasks 12-16)

Full CLI binary tests that spawn `roko` as a subprocess. These verify
the user-facing behavior end-to-end.

**Dependencies**: Phase 1 (Tasks 1-3)

### Task 12: CLI init and config smoke tests

**File**: `crates/roko-cli/tests/init_config_smoke.rs`
**What**: Verify `roko init`, `roko config`, and `roko doctor` produce
correct output and artifacts.

**Steps**:
1. Test `roko init <tmpdir>`:
   - Creates `.roko/` directory
   - Creates `roko.toml`
   - Creates `.roko/engrams.jsonl`
   - Creates `.roko/learn/` subdirectories
2. Test `roko init --demo <tmpdir>`: seeds demo data
3. Test `roko config show` outputs valid TOML
4. Test `roko config path` outputs the config file path
5. Test `roko config providers list` shows configured providers
6. Test `roko config models list` shows configured models
7. Test `roko config validate` on a valid config returns success
8. Test `roko config validate` on an invalid config returns error with
   specific validation message
9. Test `roko doctor` checks workspace bootstrap state and reports status
10. Test `roko --version` outputs version string

**Acceptance criteria**:
- 10 tests, all passing
- Every test uses isolated tempdir (no shared state)
- Tests pass without API keys set (no live API calls)
- Tests complete in < 30 seconds total

### Task 13: CLI plan lifecycle smoke tests

**File**: `crates/roko-cli/tests/plan_lifecycle_smoke.rs`
**What**: Verify the full plan lifecycle: create, validate, list, show.
Does NOT execute plans (no agent dispatch).

**Steps**:
1. Test `roko plan create <name>` creates plan directory with plan.md
2. Test `roko plan list` shows created plans
3. Test `roko plan show <name>` displays plan details
4. Test `roko plan validate <dir>` on valid tasks.toml returns success
5. Test `roko plan validate <dir>` on invalid TOML returns parse error
6. Test `roko plan validate <dir>` on TOML with missing required fields
   returns specific error
7. Test `roko plan validate <dir>` on TOML with circular dependencies
   returns cycle error
8. Test `roko plan validate <dir>` on TOML wrapped in markdown fences
   (` ```toml ... ``` `) strips fences and validates

**Acceptance criteria**:
- 8 tests, all passing
- Plan validation catches all known error classes
- Markdown fence stripping works for both ` ```toml ` and ` ```toml\n `
- Tests complete in < 20 seconds total

### Task 14: CLI knowledge and learn smoke tests

**File**: `crates/roko-cli/tests/knowledge_learn_smoke.rs`
**What**: Verify knowledge and learning CLI subcommands produce correct
output from pre-seeded data.

**Steps**:
1. Test `roko knowledge stats` on empty store returns zero counts
2. Test `roko knowledge query "test"` on empty store returns no results
3. Test `roko learn all` on empty learn directory returns empty state
4. Test `roko learn router` on empty cascade-router.json returns defaults
5. Test `roko learn experiments` on empty experiments.json returns empty
6. Test `roko learn efficiency` on empty efficiency.jsonl returns empty
7. Test `roko learn episodes` on empty episodes.jsonl returns empty
8. Pre-seed `.roko/learn/efficiency.jsonl` with 3 events, verify
   `roko learn efficiency` parses and displays them
9. Pre-seed `.roko/episodes.jsonl` with 3 episodes, verify
   `roko learn episodes` shows correct counts

**Acceptance criteria**:
- 9 tests, all passing
- All learn subcommands tested with empty and seeded data
- No panics on missing/empty files
- Tests complete in < 15 seconds total

### Task 15: CLI explain, status, and replay smoke tests

**File**: `crates/roko-cli/tests/misc_commands_smoke.rs`
**What**: Verify utility commands produce expected output.

**Steps**:
1. Test `roko explain agent` outputs concept explanation
2. Test `roko explain gate` outputs gate concept explanation
3. Test `roko explain agent --depth deep` produces longer output
4. Test `roko status` on initialized workspace shows signal/episode counts
5. Test `roko status --surfaces` shows surface inventory
6. Test `roko history list` on workspace with no sessions returns empty
7. Test `roko completions bash` outputs bash completion script
8. Test `roko completions zsh` outputs zsh completion script

**Acceptance criteria**:
- 8 tests, all passing
- Explain depth levels (brief, standard, deep) tested
- Completion scripts are non-empty and contain expected function names
- Tests complete in < 10 seconds total

### Task 16: CLI error handling smoke tests

**File**: `crates/roko-cli/tests/error_handling_smoke.rs`
**What**: Verify graceful error handling for common failure modes.

**Steps**:
1. Test `roko run "hello"` without init returns meaningful error
   (not panic, not raw Rust error)
2. Test `roko plan run nonexistent/` returns "directory not found" error
3. Test `roko config show` without roko.toml returns "not initialized" error
4. Test `roko prd list` without `.roko/prd/` returns empty list (not crash)
5. Test `roko knowledge stats` without `.roko/` returns error or empty
6. Test invalid subcommand returns usage help
7. Test `roko run` without prompt argument returns argument error
8. Test `roko config mcp list` -- verify this does NOT panic with
   `unreachable!()` (HOLLOW 1 from doc 05)

**Acceptance criteria**:
- 8 tests, all passing
- No test produces a Rust panic backtrace
- Every error has a human-readable message (not `Error: ...Debug format`)
- Tests complete in < 15 seconds total

---

## PHASE 4: Gate Parity Tests (Tasks 17-19)

Verify that the new GateService path produces equivalent verdicts to the
legacy orchestrate.rs path. Critical for the runner v2 migration.

**Dependencies**: Phase 1 (Tasks 1-3), Phase 2 Task 4

### Task 17: Gate verdict equivalence tests

**File**: `crates/roko-gate/tests/gate_parity.rs`
**What**: Run the same gate configurations through both GateService (new)
and the legacy `run_rung()` dispatch, compare verdicts.

**Steps**:
1. Create a `GateTestProject` that passes all gates
2. Run through `GateService::run_gates()` -- capture verdicts
3. Run through `run_rung()` / `run_canonical_rung()` -- capture verdicts
4. Assert: same pass/fail result per gate
5. Assert: same error_digest content (not exact match due to formatting)
6. Assert: test counts match for TestGate
7. Repeat for a project that fails compile, fails clippy, fails test

**Acceptance criteria**:
- Verdicts match for all 3 rungs (compile, clippy, test) across
  both passing and failing scenarios
- Gate ordering is identical (rung 0 before rung 1 before rung 2)
- Duration is within 2x of each other (no pathological difference)

### Task 18: Adaptive threshold parity tests

**File**: `crates/roko-gate/tests/threshold_parity.rs`
**What**: Verify AdaptiveThresholds produce identical skip decisions
regardless of which gate dispatch path calls them.

**Steps**:
1. Create `AdaptiveThresholds::new()`
2. Feed 25 consecutive passes on rung 1 (clippy)
3. Verify `should_skip_rung(1)` returns true
4. Feed 1 failure on rung 1
5. Verify skip decision resets (consecutive streak broken)
6. Save to JSON, reload, verify state is identical
7. Test temperament adjustments: Conservative never skips, Aggressive
   skips earlier
8. Test role-based overrides: high `gate_pass_rate_floor` prevents skipping

**Acceptance criteria**:
- Skip decision matches expected behavior at all streak counts
- JSON roundtrip preserves all state (EMA, CUSUM, streak)
- Temperament and role overrides work as documented in doc 20

### Task 19: Gate feedback parity tests

**File**: `crates/roko-gate/tests/feedback_parity.rs`
**What**: Verify `feedback_for_agent()` output matches expected structure
for each error category.

**Steps**:
1. Feed compile error output through `feedback_for_agent()`, verify:
   - `errors` contains error-classified lines
   - `warnings` contains warning-classified lines
   - `suggestions` contains help/note lines
2. Test noise filtering: cargo progress lines (Downloading, Compiling,
   Checking, Fresh, Running) are stripped
3. Test fallback: non-empty output with no classified lines produces
   at least one error entry
4. Test empty output: produces empty feedback (not crash)
5. Test `classify_gate_failure()` maps to correct `FailureClass`:
   - Syntax error -> `SyntaxError`
   - Missing import -> `ImportError`
   - Type mismatch -> `TypeError`
   - Borrow error -> `BorrowOrLifetime`

**Acceptance criteria**:
- Every `FailureClass` variant has at least one test input
- Noise filtering strips all known cargo progress patterns
- Feedback structure matches what orchestrate.rs produces

---

## PHASE 5: Benchmark Regression Suite (Tasks 20-23)

Build and wire the benchmark infrastructure for performance tracking
and agent evaluation.

**Dependencies**: Phase 1 (Tasks 1-3), Phase 2 (Tasks 5, 6)

### Task 20: SWE-bench proxy smoke tests

**File**: `crates/roko-cli/tests/bench_smoke.rs`
**What**: Test the existing SWE-bench proxy harness (`bench.rs`) with
the built-in smoke dataset. Verify scoring pipeline works without
real agent dispatch.

**Steps**:
1. Test `SweAgentMode::Gold` (plumbing validation): apply gold patch,
   run tests, verify pass
2. Test `SweAgentMode::Empty` (negative control): apply empty patch,
   verify fail
3. Test `SweAgentMode::PredictionFile`: write a JSONL predictions file,
   verify parsing and patch application
4. Test scoring: pass/fail/error counts are correct
5. Test learning integration: verify episodes are written after bench run
6. Test batch execution: run 2 tasks, verify both produce results
7. Test cost tracking: verify `BenchResult.cost_usd` field is populated
   (even if 0.0 for mock runs)

**Acceptance criteria**:
- 7 tests, all passing
- Gold mode passes, Empty mode fails (validates test integrity)
- JSONL prediction file parsing handles edge cases
- `cargo test -p roko-cli -- bench_smoke` runs in < 60 seconds

### Task 21: Performance baseline capture

**File**: `crates/roko-cli/tests/perf_baselines.rs`
**What**: Establish and enforce performance baselines for critical paths.

**Steps**:
1. Test gate pipeline latency: run compile+clippy+test on a minimal
   Cargo project, assert total time < 30 seconds
2. Test prompt assembly latency: assemble a 9-layer system prompt with
   knowledge/playbooks/episodes, assert time < 100ms
3. Test episode logger throughput: write 1000 episodes, assert time < 1s
4. Test TOML parsing throughput: parse a 50-task plan TOML, assert time < 50ms
5. Test config loading time: load a full roko.toml with 12 providers and
   31 models, assert time < 50ms
6. Test state persistence: save/load a 100-task executor state, assert < 100ms
7. Store baseline values in a `perf_baselines.json` fixture file for
   regression detection

**Acceptance criteria**:
- 6+ tests, all passing with generous initial baselines
- Baselines file committed as test fixture
- Each test prints actual measurement for CI visibility
- Tests use `std::time::Instant` for wall-clock measurement

### Task 22: HAL harness integration stub

**File**: `crates/roko-cli/tests/hal_integration.rs`
**What**: Build the scaffolding for HAL-compatible agent evaluation.
Does not require real HAL infrastructure -- tests the adapter layer.

**Steps**:
1. Define `HalAgentAdapter` struct that wraps roko's agent dispatch
   in HAL's expected interface (initialize, step, cleanup)
2. Test `HalAgentAdapter::initialize()` creates workspace and config
3. Test `HalAgentAdapter::step(task)` dispatches to mock agent and
   returns structured result
4. Test `HalAgentAdapter::cleanup()` collects cost/usage metrics
5. Test HAL-compatible result format: accuracy, cost, latency fields
6. Test multi-dimensional scoring: correctness (gate pass), quality
   (if judge available), cost (token spend), latency (wall time)

**Acceptance criteria**:
- Adapter struct compiles and passes mock-backed tests
- Result format matches HAL's expected schema
- Cost and latency are populated even in mock mode
- This is a foundation for future real HAL integration

### Task 23: Benchmark regression detection

**File**: `crates/roko-cli/tests/bench_regression.rs`
**What**: Compare current benchmark results against stored baselines
and detect regressions.

**Steps**:
1. Load baselines from `perf_baselines.json` (from Task 21)
2. Run each performance measurement
3. Compare against baseline with configurable threshold (default: 20% regression)
4. Report: measurement, baseline, delta percentage, pass/fail
5. Test detection: artificially inflate a baseline by 50%, verify the
   test catches the "regression"
6. Test threshold configuration: 0% threshold catches any increase,
   100% threshold catches nothing
7. Output regression report as structured JSON for CI consumption

**Acceptance criteria**:
- Regression detection catches 20%+ degradation
- Threshold is configurable per measurement
- Report is machine-parseable (JSON)
- No false positives on normal measurement variance (use 3-sigma)

---

## PHASE 6: Learning Persistence Tests (Tasks 24-26)

Verify the full learning write-read-update lifecycle across process restarts.

**Dependencies**: Phase 1 (Tasks 1-3), Phase 2 Task 5

### Task 24: Learning artifact roundtrip tests

**File**: `crates/roko-learn/tests/artifact_roundtrip.rs`
**What**: Every learning artifact type survives write -> process exit ->
read -> update -> write cycle.

**Steps**:
1. Test `cascade-router.json`: write with observations, drop the
   CascadeRouter, create new one from same file, verify observations
   are preserved and routing decisions are consistent.
2. Test `gate-thresholds.json`: write with per-rung EMA values,
   reload, verify EMA and CUSUM state preserved.
3. Test `section-effects.json`: write section weights, reload, verify
   weights and counts match.
4. Test `episodes.jsonl`: append 10 episodes in one process, append 10
   more in a second "process" (new logger instance), verify all 20 present.
5. Test `efficiency.jsonl`: same append-across-restarts pattern.
6. Test `playbooks/`: write 3 playbook files, reload PlaybookStore,
   verify all 3 are queryable.
7. Test `costs.jsonl`: append cost records, reload, verify totals.
8. Test `daimon/affect.json`: write affect state, reload, verify fields.

**Acceptance criteria**:
- All 8 artifact types survive the roundtrip
- No data loss across "process restarts" (new instances from same files)
- JSONL files support append without corrupting previous entries
- JSON files use atomic write (temp + rename)

### Task 25: Learning under concurrent access

**File**: `crates/roko-learn/tests/concurrent_learning.rs`
**What**: Test learning artifacts under concurrent read/write access
from multiple tokio tasks.

**Steps**:
1. Spawn 10 tokio tasks, each appending 100 episodes to the same
   `EpisodeLogger`. Verify total count is 1000 with no corruption.
2. Spawn 5 tokio tasks, each observing 20 routing outcomes to the same
   `CascadeRouter`. Verify total observations = 100.
3. Test file locking: two `AdaptiveThresholds` instances observing
   the same file. Verify no partial writes or corrupted JSON.
4. Test JSONL append atomicity: interrupt a write mid-line (simulate
   by writing partial data), verify reader skips malformed lines.

**Acceptance criteria**:
- No data corruption under concurrent access
- Total counts match expected values (no lost writes)
- Malformed JSONL lines are skipped, not fatal

### Task 26: Learning feedback loop integration test

**File**: `crates/roko-learn/tests/feedback_loop.rs`
**What**: End-to-end test that simulates the full learning feedback loop:
agent dispatches -> gate verdicts -> episode recording -> routing update ->
next dispatch uses updated routing.

**Steps**:
1. Create a `TestWorkspace` with clean learning state
2. Simulate 5 agent dispatches with varying outcomes:
   - Dispatch 1: model A, compile fail -> episode recorded, router observes failure
   - Dispatch 2: model B, all gates pass -> episode recorded, router observes success
   - Dispatch 3: router should now prefer model B (higher success rate)
   - Dispatch 4: model B again, test fail -> episode recorded
   - Dispatch 5: verify router adjusts (B's success rate declined)
3. After all dispatches, verify:
   - `episodes.jsonl` has 5 entries
   - `cascade-router.json` has observations for both models
   - Router's recommended model reflects observed outcomes
   - `gate-thresholds.json` has per-rung observations

**Acceptance criteria**:
- Full loop verified: dispatch -> gate -> episode -> router -> next dispatch
- Router recommendations change based on observed outcomes
- All learning artifacts are consistent at end of loop

---

## PHASE 7: Performance Regression Tests (Tasks 27-29)

**Dependencies**: Phase 1 (Tasks 1-3), Phase 5 Task 21

### Task 27: Memory usage regression tests

**File**: `crates/roko-cli/tests/memory_regression.rs`
**What**: Test that known memory issues (unbounded vectors, enrichment
artifacts) are bounded. Addresses the 9.5-11.5GB RSS leak from dogfood
(doc 05 Section 6).

**Steps**:
1. Test efficiency events vector is bounded: append 10000 events, verify
   the in-memory vector does not exceed a configured cap (e.g., 1000).
   After flush, vector is cleared.
2. Test episode logger does not accumulate in memory: write 10000 episodes,
   verify heap allocation stays flat (each write flushes to disk).
3. Test enrichment context is dropped after use: build enrichment for a
   task, verify the enrichment string is not held after dispatch completes.
4. Test executor state serialization does not grow unboundedly: 100-task
   plan with all tasks completed, state JSON is < 1MB.

**Acceptance criteria**:
- No unbounded vector growth (verified by explicit size checks)
- Memory measurements use `std::alloc` global allocator counter or
  process RSS sampling via `/proc/self/status` (Linux) or `mach_task_info` (macOS)
- Tests document the current measurements as baselines

### Task 28: Startup latency regression tests

**File**: `crates/roko-cli/tests/startup_latency.rs`
**What**: Verify CLI startup time stays within acceptable bounds.

**Steps**:
1. Test `roko --version` completes in < 500ms (cold start)
2. Test `roko --help` completes in < 500ms
3. Test `roko status --workdir <tmpdir>` completes in < 2s
   (includes config loading, signal reading)
4. Test `roko config show --workdir <tmpdir>` completes in < 1s
5. Run each measurement 3 times, take median to reduce variance

**Acceptance criteria**:
- All measurements within baseline thresholds
- Median of 3 runs used (not single measurement)
- Thresholds are generous for CI (2x local development machine)

### Task 29: Gate pipeline throughput tests

**File**: `crates/roko-gate/tests/gate_throughput.rs`
**What**: Verify gate pipeline throughput does not regress.

**Steps**:
1. Test `GateService` with 3 mock gates (instant pass): 100 iterations,
   assert total time < 1s (pipeline overhead < 10ms per run)
2. Test `AdaptiveThresholds` update speed: 10000 observations,
   assert total time < 1s
3. Test `SpcDetector` update speed: 10000 observations, assert < 2s
   (BOCPD is more expensive than CUSUM/EWMA)
4. Test `ComposedGatePipeline` with ParallelGate(3 mock gates):
   verify parallel execution is faster than sequential
5. Test `feedback_for_agent()` parsing speed: 1000 lines of mixed
   compile output, assert < 100ms

**Acceptance criteria**:
- Pipeline overhead is measurable and bounded
- Parallel gate execution provides speedup over sequential
- Statistical detectors scale linearly with observations

---

## PHASE 8: Chaos and Fault Injection Tests (Tasks 30-32)

**Dependencies**: Phase 1 (Tasks 1-3), Phase 2 (Tasks 4, 7)

### Task 30: Agent crash recovery tests

**File**: `crates/roko-cli/tests/agent_crash_recovery.rs`
**What**: Verify the system handles agent crashes, timeouts, and
unexpected termination gracefully.

**Steps**:
1. Test agent SIGKILL: spawn a mock agent, send SIGKILL, verify the
   runner detects the crash and records a failure episode.
2. Test agent timeout: configure 5-second timeout, spawn agent that
   sleeps 10 seconds, verify timeout fires and process is killed.
3. Test agent stderr output: mock agent writes to stderr, verify
   error output is captured in the episode.
4. Test agent invalid output: mock agent produces malformed JSON
   (not valid stream-json), verify parser handles it without panic.
5. Test agent exit code: mock agent exits with code 1/2/137, verify
   different exit codes produce appropriate error messages.
6. Test partial output recovery: agent produces valid output then
   crashes mid-stream, verify partial output is preserved.

**Acceptance criteria**:
- No panic in any crash scenario
- All crashes produce failure episodes with error context
- Timeout is enforced (process killed, not hung)
- Partial output preserved when possible

### Task 31: Gate failure edge cases

**File**: `crates/roko-gate/tests/gate_edge_cases.rs`
**What**: Verify gates handle edge cases: missing binaries, permission
errors, disk full, and concurrent gate execution.

**Steps**:
1. Test missing cargo binary: set PATH to empty, verify CompileGate
   returns a clear "cargo not found" error (not raw OS error).
2. Test read-only workdir: make workdir read-only, verify gates that
   need to write (GeneratedTestGate) produce clear error.
3. Test empty project: run gates on a directory with no Cargo.toml,
   verify clear error message.
4. Test very large output: mock gate that produces 1MB of stderr,
   verify output is truncated and not OOM.
5. Test shell gate with non-UTF-8 output: verify output is handled
   (lossy conversion, not panic).
6. Test concurrent gate execution: run 3 CompileGate instances on
   the same project simultaneously, verify no interference.

**Acceptance criteria**:
- Every edge case produces a clear error message
- No panics, no OOM, no hangs
- Concurrent execution is safe (may fail, but must not corrupt)

### Task 32: State persistence under failure

**File**: `crates/roko-cli/tests/state_persistence_chaos.rs`
**What**: Verify executor state persistence is resilient to crashes
and partial writes.

**Steps**:
1. Test crash-during-save: write executor state, corrupt the file
   (truncate to half), verify `load_state()` detects corruption and
   falls back to initial state (not panic).
2. Test missing state file: verify `load_state()` on missing file
   returns clean initial state.
3. Test atomic write: verify state is written via temp file + rename
   (check that a concurrent reader never sees partial data).
4. Test state with 1000 completed tasks: verify save/load roundtrip
   preserves all task statuses.
5. Test concurrent save: two executor instances saving to the same
   file, verify last-writer-wins (no interleaved data).
6. Test disk full simulation: write state to a tmpfs with 0 bytes
   free, verify error is reported (not silent data loss).

**Acceptance criteria**:
- Corrupted state files produce fallback, not panic
- Atomic write pattern verified
- No data loss under concurrent access
- Disk full produces explicit error

---

## PHASE 9: CLI Command Known-Good Output Tests (Tasks 33-36)

Snapshot tests that capture the output of every major CLI command and
detect unexpected changes.

**Dependencies**: Phase 1 (Tasks 1-3)

### Task 33: CLI help text snapshot tests

**File**: `crates/roko-cli/tests/help_snapshots.rs`
**What**: Capture `--help` output for every subcommand and verify it
matches committed snapshots.

**Steps**:
1. Generate `--help` output for each top-level command:
   `run`, `plan`, `prd`, `chat`, `serve`, `status`, `doctor`, `init`,
   `config`, `learn`, `knowledge`, `research`, `agent`, `job`, `explain`,
   `dashboard`, `bench`, `deploy`, `replay`, `history`, `up`
2. Generate `--help` for each plan subcommand:
   `plan list`, `plan show`, `plan create`, `plan run`, `plan validate`,
   `plan generate`, `plan regenerate`
3. Generate `--help` for each config subcommand:
   `config show`, `config path`, `config providers`, `config models`,
   `config secrets`, `config validate`
4. Store snapshots in `crates/roko-cli/tests/snapshots/help/`
5. Compare against snapshots using `insta` or simple string comparison
6. Fail on unexpected changes (forces intentional help text updates)

**Acceptance criteria**:
- Every top-level command has a help snapshot
- Every subcommand with sub-subcommands has snapshots
- Adding a new flag without updating snapshots causes test failure

### Task 34: CLI JSON output snapshot tests

**File**: `crates/roko-cli/tests/json_snapshots.rs`
**What**: Commands with `--json` output produce stable, parseable JSON.

**Steps**:
1. Test `roko status --json` on seeded workspace produces valid JSON
   with expected fields: `signals`, `episodes`, `cfactor`
2. Test `roko learn episodes --json` on seeded episodes.jsonl produces
   valid JSON array
3. Test `roko learn router --json` on seeded cascade-router.json produces
   valid JSON
4. Test `roko config show --json` produces valid JSON matching roko.toml
5. Test `roko plan list --json` on seeded plans produces valid JSON array
6. Store JSON schemas (or key fields) as test fixtures
7. Validate output against schemas

**Acceptance criteria**:
- Every `--json` command produces valid JSON (parseable by serde_json)
- Output contains all expected fields
- Adding/removing a field without updating fixtures causes test failure

### Task 35: CLI exit code tests

**File**: `crates/roko-cli/tests/exit_codes.rs`
**What**: Every command returns correct exit codes for success and failure.

**Steps**:
1. Success cases (exit 0):
   - `roko init <tmpdir>`
   - `roko --version`
   - `roko --help`
   - `roko config show` (with valid config)
   - `roko status` (with initialized workspace)
   - `roko plan validate <valid-plan>`
2. Failure cases (exit non-zero):
   - `roko run` (no prompt argument)
   - `roko plan run nonexistent/`
   - `roko plan validate <invalid-toml>`
   - `roko config show` (no roko.toml)
   - Unknown subcommand
3. Verify specific exit codes where applicable (1 for general error,
   2 for usage error)

**Acceptance criteria**:
- All success cases exit 0
- All failure cases exit non-zero
- No command silently succeeds when it should fail

### Task 36: CLI stderr/stdout separation tests

**File**: `crates/roko-cli/tests/output_channels.rs`
**What**: Verify that errors go to stderr and normal output goes to stdout.

**Steps**:
1. Test `roko --help`: output on stdout, nothing on stderr
2. Test `roko status --json`: JSON on stdout, logs on stderr
3. Test `roko plan validate <invalid>`: error message on stderr,
   nothing on stdout
4. Test `roko config show`: config on stdout, nothing on stderr
5. Test `roko run` (no args): error on stderr, usage hint on stderr
6. Verify that `--quiet` flag suppresses stdout but not errors

**Acceptance criteria**:
- Normal output always goes to stdout
- Errors always go to stderr
- JSON output is on stdout only (not mixed with log lines)
- `--quiet` suppresses informational output

---

## PHASE 10: Extended ACP and Protocol Tests (Tasks 37-38)

**Dependencies**: Phase 2 Task 8

### Task 37: ACP session recovery tests

**File**: `crates/roko-acp/tests/session_recovery.rs`
**What**: Test ACP session persistence, recovery after crashes, and
multi-session coordination.

**Steps**:
1. Test session persistence: create session, add 5 turns of history,
   save, reload from disk, verify history is intact.
2. Test session resume after server restart: create session, "restart"
   server (new TestHarness), load session by ID, verify context preserved.
3. Test session cleanup: cancel session, verify resources are released,
   session ID is no longer loadable.
4. Test session listing: create 3 sessions, list, verify all 3 present
   with correct metadata (mode, model, turn count).
5. Test session limit: if a max_sessions config exists, verify that
   exceeding it returns appropriate error.
6. Test stale session detection: create session, advance clock by 24h,
   verify session is marked stale.

**Acceptance criteria**:
- Session state survives simulated server restarts
- History is preserved with exact content
- Cancelled sessions are cleaned up
- Session listing is consistent

### Task 38: ACP telemetry integration tests

**File**: `crates/roko-acp/tests/telemetry_verification.rs`
**What**: Extends existing `telemetry_integration.rs` with verification
that ACP pipeline runs produce correct telemetry events.

**Steps**:
1. Test gate progress events: during a pipeline run, verify the client
   receives gate start, gate pass/fail, and gate complete notifications.
2. Test token usage events: verify the client receives token usage
   updates during agent streaming.
3. Test phase transition events: verify the client receives events
   for each pipeline phase (strategy, implement, gate, review, commit).
4. Test file change events: verify the client receives file change
   notifications after agent writes files.
5. Test cost tracking events: verify the client receives cost updates
   with running totals.
6. Test event ordering: verify events arrive in logical order
   (gate_start before gate_pass, phase_start before phase_end).

**Acceptance criteria**:
- Every event type is verified
- Event ordering is correct
- Token counts and cost values are non-zero for real dispatches
- Events are delivered as JSON-RPC notifications (not responses)

---

## Dependency Graph

```
Phase 1 (Tasks 1-3)
  |
  +--- Phase 2 (Tasks 4-11) [parallel within phase]
  |      |
  |      +--- Phase 4 (Tasks 17-19) [depends on Task 4]
  |      +--- Phase 5 (Tasks 20-23) [depends on Tasks 5, 6]
  |      +--- Phase 6 (Tasks 24-26) [depends on Task 5]
  |      +--- Phase 8 (Tasks 30-32) [depends on Tasks 4, 7]
  |      +--- Phase 10 (Tasks 37-38) [depends on Task 8]
  |
  +--- Phase 3 (Tasks 12-16) [parallel within phase]
  |
  +--- Phase 7 (Tasks 27-29) [depends on Tasks 1-3, Task 21]
  |
  +--- Phase 9 (Tasks 33-36) [depends on Tasks 1-3]
```

Phases 2, 3, 7, 9 can all start once Phase 1 completes.
Phase 4, 5, 6, 8, 10 require specific Phase 2 tasks.

---

## Execution Estimates

| Phase | Tasks | Estimated Days | Parallelizable |
|---|---|---|---|
| 1: Harness | 1-3 | 2 | No (sequential) |
| 2: Subsystem integration | 4-11 | 4-5 | Yes (8 tasks, all independent) |
| 3: E2E smoke | 12-16 | 2-3 | Yes (5 tasks, all independent) |
| 4: Gate parity | 17-19 | 1-2 | Partially (17 first, then 18-19) |
| 5: Benchmarks | 20-23 | 2-3 | Yes (4 tasks, mostly independent) |
| 6: Learning persistence | 24-26 | 1-2 | Partially (24 first, then 25-26) |
| 7: Performance regression | 27-29 | 1-2 | Yes (3 tasks, all independent) |
| 8: Chaos/fault | 30-32 | 2 | Yes (3 tasks, all independent) |
| 9: CLI snapshots | 33-36 | 1-2 | Yes (4 tasks, all independent) |
| 10: ACP extended | 37-38 | 1 | Partially |
| **Total** | **38** | **16-22** | Critical path: Phase 1 -> Phase 2 -> Phase 4 (~8 days) |

---

## Test Run Configuration

### CI Integration

All tests should be runnable via:

```bash
# Full suite
cargo test --workspace

# Per-phase
cargo test -p roko-test-harness             # Phase 1
cargo test -p roko-gate -- subsystem        # Phase 2, Task 4
cargo test -p roko-learn -- subsystem       # Phase 2, Task 5
cargo test -p roko-cli -- smoke             # Phase 3
cargo test -p roko-gate -- parity           # Phase 4
cargo test -p roko-cli -- bench             # Phase 5
cargo test -p roko-learn -- roundtrip       # Phase 6
cargo test -p roko-cli -- perf              # Phase 7
cargo test -p roko-gate -- chaos            # Phase 8
cargo test -p roko-cli -- snapshot          # Phase 9
cargo test -p roko-acp -- conformance       # Phase 10
```

### Test Markers

Tests that require external resources should use `#[ignore]` and be
runnable with `cargo test -- --ignored`:

- Tests requiring real API keys (live provider tests)
- Tests requiring large datasets (SWE-bench full)
- Tests with long execution times (> 60 seconds)
- Tests requiring specific system resources (Docker, network)

### No-API-Key Tests

All tests in this plan are designed to work WITHOUT API keys set.
They use mock agents, mock providers, and canned responses. This
ensures CI can run the full test suite without credential configuration.

---

## Success Criteria

The testing infrastructure is complete when:

1. **Coverage**: Every crate has at least one integration test file
   (currently 14/18 crates have integration tests).

2. **Gate parity**: GateService produces equivalent verdicts to
   orchestrate.rs for all rung 0-2 scenarios.

3. **CLI stability**: Every CLI subcommand has at least one test
   verifying it does not crash.

4. **Learning integrity**: Every learning artifact type survives
   the write-restart-read cycle.

5. **Performance baselines**: Regression detection catches 20%+
   degradation in critical paths.

6. **Chaos resilience**: Agent crashes, gate timeouts, and state
   corruption all produce graceful recovery.

7. **CI green**: `cargo test --workspace` passes with all 38 tasks
   implemented. No `#[ignore]` on any test that can run without
   external resources.

---

## Appendix: Existing Test Inventory

Current test coverage by crate (for context on what exists):

| Crate | Integration Test Files | Inline Test Modules | Notable Gaps |
|---|---|---|---|
| roko-cli | 21 files | ~15 | No plan lifecycle tests, no learn CLI tests |
| roko-agent | 23 files | ~10 | Good provider coverage, no dispatch integration |
| roko-gate | 4 files | ~8 | No composition tests, no parity tests |
| roko-learn | 4 files | ~8 | No concurrent access, no full feedback loop |
| roko-serve | 5 files | ~5 | Limited route coverage |
| roko-compose | 2 files | ~3 | No prompt assembly integration |
| roko-acp | 3 files | ~3 | Limited session lifecycle coverage |
| roko-runtime | 1 file | ~4 | No workflow engine integration |
| roko-orchestrator | 1 file | ~3 | No DAG/executor integration |
| roko-core | 2 files | ~10 | Good (stable kernel) |
| roko-std | 4 files | ~5 | Good (stable tools) |
| roko-primitives | 1 file | ~3 | Good |
| roko-conductor | 1 file | ~2 | Property tests only |
| roko-daimon | 1 file | ~2 | Regression test only |
| roko-chain | 1 file | ~1 | Live test only (alloy) |
| roko-plugin | 1 file | ~1 | SDK test only |
| roko-agent-server | 1 file | ~2 | Relay test only |
| roko-fs | 0 files | ~3 | No integration tests |
| roko-dreams | 0 files | ~2 | No integration tests |
| roko-neuro | 0 files | ~3 | No integration tests |
| roko-index | 0 files | ~2 | No integration tests |

Key files referenced in this plan:

| File | Role |
|---|---|
| `crates/roko-cli/tests/common/mod.rs` | Existing CLI test helpers |
| `crates/roko-gate/tests/gate_truth.rs` | Existing GateService integration tests |
| `crates/roko-gate/tests/rungs.rs` | Existing 7-rung pipeline tests |
| `crates/roko-serve/tests/api_integration.rs` | Existing HTTP API tests |
| `crates/roko-learn/tests/learning_loop.rs` | Existing learning loop tests |
| `crates/roko-acp/tests/protocol_conformance.rs` | Existing ACP protocol tests |
| `crates/roko-gate/src/gate_service.rs` | GateService (680 LOC) |
| `crates/roko-gate/src/adaptive_threshold.rs` | AdaptiveThresholds (957 LOC) |
| `crates/roko-gate/src/feedback.rs` | Gate feedback filter (393 LOC) |
| `crates/roko-learn/src/episode_logger.rs` | Episode JSONL logger |
| `crates/roko-learn/src/cascade_router.rs` | Bandit-based model routing |
| `crates/roko-runtime/src/workflow_engine.rs` | WorkflowEngine facade |
| `crates/roko-runtime/src/pipeline_state.rs` | PipelineStateV2 state machine |
| `crates/roko-cli/src/runner/event_loop.rs` | Plan runner V2 event loop |
