# Current State (2026-05-05)

This document replaces STATUS.md as the definitive source of truth. STATUS.md reported
what was implemented; this document reports what is actually wired and working at runtime.

---

## Executive Summary

46 backend batches were implemented (W0 through W15). The code compiles, tests pass,
and clippy is clean. However:

- The SCENARIO-REDESIGN.md (collapsing 14 to 5 scenarios, custom sidebar panels,
  `roko build`, SSE streaming from serve to demo) was **never implemented**. It is a
  design document that was never decomposed into batches.
- Of the 46 "completed" batches, audit shows roughly:
  - **~30% fully wired at runtime** — the code is called and produces visible behavior
  - **~30% partially wired** — the code exists and is called but the full path has gaps
  - **~40% created but never called** — structs, traits, and methods with zero callers
    outside their own crate's tests
- The demo app UX is **effectively unchanged** from before all this work. All 14 old
  scenarios remain. The sidebar shows a label and description, not live numbers.
- The real UX problem remains unsolved: the 35-subcommand CLI taxonomy does not match
  how anyone actually uses the tool.

---

## What Was Actually Delivered (Verified Working at Runtime)

These are changes that are called from the live code path and produce visible behavior.

### Plan Runner (v2 engine — `event_loop.rs`)

- **Cross-task output injection (W9-B)**: `dependency_outputs` is populated from
  `state.dependency_outputs(plan_id, &task_def.depends_on)` in `event_loop.rs:2386`
  and reaches the agent prompt via `prompt_builder.rs:595-607`. Predecessor files are
  listed in a `# Prior Task Outputs` section of every agent prompt.

- **Per-run gate semaphore (W12-A)**: `event_loop.rs:263` creates a `Semaphore` scoped
  to each run. `gate_dispatch.rs:48` acquires it before every rung. Concurrent plan runs
  no longer share gate slots.

- **Per-plan agent handles (W12-B)**: Multiple plans can run in parallel; each has its
  own agent factory and process lifecycle.

- **Atomic state writes (W13-D)**: `persist.rs:237` implements `atomic_write()` using
  write-to-temp-then-rename. All four persist helpers (`write_executor_snapshot`,
  `write_orchestrator_snapshot`, `write_agent_pids`, `write_run_state`) call it.

- **TOML repair pipeline (W13-A)**: LLM-generated TOML that fails `toml::from_str` is
  auto-repaired before the plan loader rejects it.

- **Fatal event handling (W11-A)**: Gate failures are now propagated as `Fatal` events
  to the executor. Plans no longer hang on gate failure.

- **Shell injection prevention (W11-D)**: Crate names extracted from task file paths are
  validated by `is_valid_crate_name()` in `plan_loader.rs` before being used in scaffold.

- **Error classification improvement (W13-E)**: `classify_error_pattern()` in
  `task_runner.rs` uses priority-ordered matching — Timeout first, then Infrastructure
  with specific keywords, then ToolCall, Compile, Test — instead of broad
  `.contains("test")` and `.contains("compile")` checks.

- **SafetyLayer required (W15-B)**: `ToolDispatcher` now holds `SafetyLayer` (not
  `Option<SafetyLayer>`). `new()` initializes with `SafetyLayer::with_defaults()`.
  Safety can no longer be accidentally omitted.

- **Health endpoint fix (W14-B)**: `GET /api/health` uses `try_read()` instead of
  `read()` to avoid blocking on RwLock contention during plan runs. Returns 503 when all
  providers are unhealthy, 207 when some are unhealthy.

- **Config unification (W14-D)**: Deprecated config loaders delegate to the unified
  loader. Old entry points still work.

- **LinUCB state persistence (W14-C)**: `LinUCBSnapshot` is serialized into the cascade
  router's JSON file on each persist call. The router survives restart with bandit state
  intact.

- **Env var parse warnings (W15-B)**: `apply_env()` in `schema.rs` now logs
  `tracing::warn!` when `ROKO_CONTEXT_LIMIT_K`, `ROKO_MAX_AGENTS`, or `ROKO_BUDGET_USD`
  contain non-parseable values instead of silently ignoring them.

- **Error drop logging (W15-B)**: All 12 `let _ = self.daimon.appraise(...)` sites
  in `orchestrate.rs` were converted to `if let Err(e) = ...` with `tracing::warn!`.

- **TuiBridge → SSE path**: `event_loop.rs` uses `TuiBridge` to emit `DashboardEvent`
  variants (TaskStarted, TaskCompleted, GateResult, AgentOutput, etc.) into the
  `StateHub`. The `StateHub` feeds the SSE endpoint at `/api/events`. This means
  `roko serve` + `plan run` together already stream progress events that a frontend
  could consume.

- **RuntimeEvent HTTP ingest (wp-arch2)**: `POST /api/events/ingest` and
  `/api/events/ingest/batch` now accept canonical `RuntimeEvent` JSON. `HttpEventSink`
  in `roko-runtime` forwards subprocess events to serve, PTY injects `ROKO_SERVE_URL`,
  and ACP maps `CognitiveEvent` to `RuntimeEvent`. E2E hardening remains tracked by
  taskrunner task 105.

### Schema additions (in roko-core, available to callers)

- **TimeoutConfig (W15-C)**: Added to `RokoConfig` as `config.timeouts`. Deserializes
  from `[timeouts]` in `roko.toml`. Tests pass. However, no code outside `roko-core`
  reads `config.timeouts.*` yet — it is a schema addition without consumers.

- **GateRungConfig / `effective_rungs()` (W15-E)**: Lives in `roko-core/src/config/gates.rs`
  and is exported from `roko-core`. Tests pass. The gate pipeline in the runner does
  not call `effective_rungs()` — it uses hardcoded rung integers.

- **`validate_against_schema()` on TasksFile (W13-E)**: Added to `task_parser.rs`.
  Tests pass. Not called from any plan loading code path.

### Demo app structural improvements (W15-D)

- `TimeoutConfig` interface and `DEFAULT_TIMEOUTS` in `terminal-session.ts` — timeouts
  are now named constants instead of inline numbers.
- `CommandFailureReason` type added to `terminal-session.ts` — machine-readable failure
  reasons instead of bare string errors.
- `AbortController` is used in `ScenarioSlot.tsx` to cancel in-flight commands on reset.
- `provider-race.ts` has `resetState()` called when the first command of a new race runs.

---

## What Was Created But Not Wired (Dead Code)

These were implemented, compile, and have unit tests, but have zero callers in the
production code path.

### `RunOutputSink` trait and `StderrSink` / `NoopSink` (W15-B)

- **File**: `crates/roko-cli/src/runner/output_sink.rs`
- **Status**: Module registered in `runner/mod.rs`. Trait and two implementations exist.
- **Gap**: `agent_events.rs` still contains the 10 inline `if stream_to_stderr { eprintln!(...) }`
  blocks that this trait was designed to replace. `handle_agent_event()` takes no sink
  parameter. The trait has zero callers.

### `Workspace` struct (W15-E)

- **File**: Exported from `roko-core` as `pub use workspace::Workspace`
- **Status**: Public workspace path boundary, now used from `roko-cli` in the `roko do`
  path and related command utilities.
- **Gap**: Migration is partial. `RokoLayout` remains live for roko-fs internals and many
  existing callsites, and raw `.join(".roko")` usage remains widespread. Taskrunner task
  004 tracks the phased migration.

### `AdaptiveBudget` and `adaptive_budget_for()` (W15-E)

- **File**: `crates/roko-compose/src/templates/common.rs`
- **Status**: Struct, impl, and free function exist with tests.
- **Gap**: No code outside `roko-compose` calls `adaptive_budget_for()`. Role templates
  still call `budget_for(role)` (the static fixed-size version). The dynamic scaling
  logic is unused at runtime.

### `ImplementerTemplate` as dispatch input (W9-A)

- **File**: `crates/roko-compose/src/templates/implementer.rs`
- **Status**: `ImplementerTemplate` struct and `RolePromptTemplate` impl exist. Used
  inside `roko-compose` for role identity string lookup (`role_prompts.rs:707`).
- **Gap**: The intent of W9-A was to pass an `ImplementerTemplate` instance into the
  dispatch pipeline so the template's `render()` method drives prompt assembly. This
  was not wired. `orchestrate.rs` has no import of `ImplementerTemplate` and does not
  pass it to `dispatch_agent_with`.

### `dispatch_and_record` helper (W15-B)

- **File**: `crates/roko-cli/src/orchestrate.rs` at line 17165
- **Status**: Method exists on `Orchestrator`.
- **Gap**: The 15 `dispatch_agent_with` call sites in `orchestrate.rs` still call
  `dispatch_agent_with` directly. `dispatch_and_record` has zero callers. The 11-site
  deduplication the batch described has not happened.

### `workspace_context()` function in orchestrate.rs (W15-A)

- **File**: `crates/roko-cli/src/orchestrate.rs:1278`
- **Status**: Function exists and is called from one site in the legacy `PlanRunner`
  path (`orchestrate.rs:15104`).
- **Gap**: The production v2 runner (`event_loop.rs`) builds workspace context via its
  own `DispatchContext` fields. It does not call `workspace_context()`. The function
  is legacy-only.

### `validate_against_schema()` on TasksFile (W13-E)

- **File**: `crates/roko-cli/src/task_parser.rs:830`
- **Status**: Method added, tests pass.
- **Gap**: Not called from `plan_loader.rs`, `validate.rs`, or any plan loading path.
  Schema violations are silently ignored at runtime.

### `GateRungConfig` / `effective_rungs()` (W15-E)

- **File**: `crates/roko-core/src/config/gates.rs`
- **Status**: Struct, impl, and `effective_rungs()` exist. Exported from `roko-core`.
- **Gap**: The runner's gate dispatch (`gate_dispatch.rs`) passes hardcoded rung
  integers to the gate pipeline. `effective_rungs()` has no callers outside `roko-core`
  tests.

### `TimeoutConfig` field in `RokoConfig` (W15-C)

- **File**: `crates/roko-core/src/config/schema.rs:106`
- **Status**: `pub timeouts: TimeoutConfig` added to schema. Deserializes from TOML.
- **Gap**: No code in `roko-cli`, `roko-serve`, or `roko-agent` reads `config.timeouts.*`.
  All timeout values in the runner and dispatcher are still hardcoded constants.

---

## What Was Partially Wired

### Observability instrumentation (W9-D)

- `tracing::info!` calls were added at startup, gate timing, run completion, and dispatch.
- These produce log lines but do not feed into structured metrics collection or the TUI.
- No histogram, counter, or span tracking was added.

### Safety layer coverage (W15-B)

- `ToolDispatcher` now has required `SafetyLayer`.
- The `ExecAgent`, `GeminiBackend`, and `CursorBackend` providers do not go through
  `ToolDispatcher`. They have their own dispatch paths. Safety is not enforced there.

### model_hint removal (W15-A)

- The `model_hint()` method that produced provider-specific model name contradictions
  was removed from one location.
- The `model_hint` field still exists on `DispatchContext` as a "user override" path
  (`event_loop.rs:2376`).

### Cost tracking (W9-C)

- Token counts are collected from agent events and propagated to efficiency events.
- Cost-to-USD conversion exists in the efficiency logger.
- The demo UI's cost display still depends on parsing `roko learn efficiency` output,
  not a live SSE feed.

---

## The Demo App Problem

The demo app was not redesigned as part of W0-W15. Its current state:

- All 14 original scenarios remain (SCENARIO-REDESIGN.md collapsed them to 5; never
  implemented)
- `ContextPanel` exists in `ScenarioSlot.tsx` but renders only a scenario label and
  description — no live numbers, no pipeline visualization, no comparison widgets
- `PrdPipelinePanel` and `KnowledgeFlowPanel` components exist but are only shown for
  specific scenario types (`prd-pipeline` and `knowledge-transfer`). The other 12
  scenarios get the plain ContextPanel
- `provider-race.ts` has module-level state (`const state = { costs, tokens, finishOrder... }`)
  that persists across resets unless `resetState()` is called. `resetState()` is called
  only when the first race command runs, not when the scenario is reset from the UI
- `gate-retry.ts` has module-level `let runOutcome` that persists across resets
- `knowledge-transfer.ts` has module-level `let betaWorkspaceDir` that persists
- The `providers` scenario swallows API errors with a regex check and returns `{ ok: true }`
  regardless — errors appear in the terminal buffer but do not propagate as failures
- Terminal output is the only visual for most scenarios
- No streaming output from `roko serve` SSE to scenario-specific sidebar panels
- No comparison widgets with live-updating deltas
- `roko do` now exists and routes through the existing WorkflowEngine, but the full
  medium/complex PRD -> plan pipeline and work-item resume semantics are not complete
- The ISFR scenario still runs 8 panes with 9 commands (not 4 panes / 3 commands)

---

## The CLI Problem

From multiple design documents (15-UX-PLAN.md, 09-UX-WORKFLOW-VISION.md,
42-workflow-redesign-suggestion.md):

- 35+ subcommands at 3-4 levels deep (`roko config secrets rotate`, `roko knowledge
  dream archive`, etc.)
- The intended user workflow — describe what you want, get working code — requires
  4 separate commands: `prd idea`, `prd draft`, `prd plan`, `plan run`
- `orchestrate.rs` remains legacy/feature-gated; the practical path is v2
  `event_loop.rs` / `WorkflowEngine`. CLI docs and taskrunner task 056 track remaining
  cleanup of stale legacy references.
- Multiple init paths: `roko init`, `roko config init`, `roko config set-secret`
- No intent detection — a typo fix and a major architectural rewrite go through the
  same 6-step pipeline at the same cost
- `roko doctor` and `roko status` overlap significantly
- Learning is invisible: `roko learn all` dumps raw JSONL, `roko learn router` dumps
  JSON — no summary, no trend, no actionable output
- "Confirmation theater": several commands print "Success" without doing anything
  observable (e.g. `roko config validate` with no errors prints nothing meaningful)

---

## What Should Happen Next (Priority Order)

### 1. Implement the scenario redesign

SCENARIO-REDESIGN.md is the specification. It has never been touched. The work:

- Collapse 14 scenarios to 5: Cost, Pipeline, Memory, ISFR, Oracle
- Add `roko build "<prompt>"` as a top-level CLI command (one-shot pipeline)
- Add `--no-cascade` flag to `roko run` (for Cost demo naive side)
- Add `--knowledge-from <dir>` to `roko build` (for Memory demo)
- Build 5 scenario-specific sidebar components: ComparisonPanel, PipelinePanel,
  TransferPanel, SwarmPanel, FlowPanel
- Wire the existing SSE events (`/api/events`) into the new sidebar panels
- Remove typing animation from command dispatch (pure dead time)

### 2. Wire the dead code

Items with the highest value-to-effort ratio:

- **`RunOutputSink`**: Add `sink: Arc<dyn RunOutputSink>` to `handle_agent_event()`,
  replace 10 `if stream_to_stderr` blocks with `sink.method()` calls. This unblocks
  JSON output mode and test-mode capture.
- **`effective_rungs()`**: Call from `gate_dispatch.rs` instead of hardcoded integers.
  Immediately makes gate pipeline configurable via `roko.toml [gates]`.
- **`validate_against_schema()`**: Call from the plan validation path in `plan_loader.rs`
  so schema errors surface before dispatch rather than causing confusing runtime failures.
- **`TimeoutConfig`**: Read `config.timeouts.agent_turn_secs` etc. from the runner
  instead of the hardcoded 300/600/900 constants.

### 3. Kill duplicate execution paths

- Converge on v2 (`event_loop.rs`) as the single execution engine
- Remove or clearly deprecate the `--engine legacy` flag
- Move any features that only exist in `orchestrate.rs` (e.g. workspace_context) into
  the v2 path, then delete the legacy path

### 4. CLI simplification

The "5 verbs" proposal from `tmp/subsystem-audits/05-01/42-workflow-redesign-suggestion.md`:
`do / think / show / tune / undo`. `roko do` is now the hero command, but it is currently
a WorkflowEngine template selector rather than the full PRD/plan pipeline.

### 5. SSE streaming to demo sidebar

The infrastructure exists: `TuiBridge` → `StateHub` → `/api/events`. What is missing
is the demo frontend consuming these events and routing them to scenario-specific
sidebar panels. The new PipelinePanel, ComparisonPanel, etc. need to subscribe to
`/api/events` and update on `TaskStarted`, `TaskCompleted`, `GateResult` events.

---

## Key Design Documents (For Next Implementation Phase)

All paths are relative to the workspace root `/Users/will/dev/nunchi/roko/roko/`.

| Document | Path | What |
|---|---|---|
| Scenario redesign | `tmp/solutions/demo-running/SCENARIO-REDESIGN.md` | 5-scenario spec with custom sidebar panels, `roko build`, SSE wiring |
| 5-verb CLI proposal | `tmp/subsystem-audits/05-01/42-workflow-redesign-suggestion.md` | "do/think/show/tune/undo" simplification |
| UX workflow vision | `tmp/solutions/roko/09-UX-WORKFLOW-VISION.md` | aggregate→funnel→execute workflow model |
| UX overhaul plan | `tmp/solutions/roko/15-UX-PLAN.md` | 6-phase UX implementation plan |
| Entry point convergence | `tmp/workflow/implementation-plans/11-entry-point-convergence.md` | Unified dispatch, killing dual-engine |
| Demo implementation plan | `tmp/demo-req/IMPLEMENTATION-PLAN.md` | Inline ratatui + Clack-style output primitives |
| Workflow engine audit | `tmp/mori-diffs/36-WORKFLOW-ENTRYPOINT-ORCHESTRATION-AUDIT.md` | WorkflowEngine convergence analysis |

---

## Build Gates (as of 2026-05-05)

```
cargo +nightly fmt --all -- --check    PASS
cargo clippy --workspace --no-deps     PASS (0 errors)
cargo test --workspace                 PASS (all tests green)
```

The codebase is clean. The gap is not code quality — it is wiring and UX.
