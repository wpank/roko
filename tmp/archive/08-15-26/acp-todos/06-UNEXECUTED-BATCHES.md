# ACP: Unexecuted Batch Analysis (ACP09-ACP18)

> **Source**: `tmp/acp-runner/BATCHES.md`, `tmp/acp-runner/08-IMPLEMENTATION-PLAN.md`
> **References**: `tmp/acp-features/00-ACP-FEATURES.md`, `tmp/acp-runner/prompts/ACP09-18.prompt.md`
> **Created**: 2026-08-15

## Background

The ACP runner planned 18 Codex batches in 4 groups:

- **Scaffold** (ACP01-03): crate setup, JSON-RPC types, stdio transport
- **Core** (ACP04-08): handler dispatch, sessions, prompt handling, CLI command, conformance tests
- **Bridges** (ACP09-14): FS, terminal, permissions, gates, plans, usage -- editor-mediated I/O
- **Config** (ACP15-18): session config options, slash commands, elicitation forms, lifecycle tests

Batches ACP01-ACP08 were executed by the Codex runner. The `BATCHES.md` header says
"All 18 batches completed successfully" but that is misleading -- ACP09-18 were **not
executed as Codex batches**. Instead, much of their functionality was **built by hand**
through multiple manual development sessions directly in `roko-acp`, often with a
different (richer) design than the batch prompts specified.

## Batch Status Overview

| Batch | Planned | Now Exists? | Gap |
|---|---|---|---|
| ACP09 | File system bridge (editor-mediated read/write) | **Missing** | No `bridge_fs.rs`; no `fs/read_text_file` dispatch |
| ACP10 | Terminal bridge (editor-mediated shell exec) | **Missing** | No `bridge_terminal.rs`; no `terminal/create` dispatch |
| ACP11 | Permission bridge (editor approval for destructive ops) | **Exists** | Fully built in `bridge_events.rs` + `types.rs` |
| ACP12 | Gate result bridge (gate events -> tool_call cards) | **Exists** | Built in `runner.rs` + `event_forward.rs` |
| ACP13 | Plan phase bridge (phase transitions -> plan entries) | **Exists** | `build_plan_entries()` + `workflow_plan_entries()` in `runner.rs` |
| ACP14 | Usage/cost bridge (token/cost tracking + context warnings) | **Partial** | `UsageUpdate` + `CostInfo` emitted; no `AcpUsageBridge` accumulator or `ContextWarning` |
| ACP15 | Session config options (7 editor UI controls) | **Exists** (evolved) | `build_config_options()` returns 6 options (provider, model, effort, workflow, clippy, tests) -- different schema than planned |
| ACP16 | Slash commands (8 commands + dynamic filtering) | **Exists** (superseded) | 44 slash commands built vs 8 planned; implemented in `session.rs` not a separate `commands.rs` |
| ACP17 | Elicitation forms (structured editor forms) | **Missing** | No `elicitation.rs`; no `elicitation/create` dispatch |
| ACP18 | Lifecycle integration tests | **Partial** | 8 protocol conformance tests + 5 telemetry tests exist; no config-option or slash-command integration tests |

## Detailed Analysis

---

### ACP09: File System Bridge

- **Planned scope**: `AcpFileSystem` struct in `bridge_fs.rs` -- routes file reads/writes through the editor via `fs/read_text_file` / `fs/write_text_file` JSON-RPC, with local filesystem fallback. ~300 LOC.
- **Current state**: **Missing**. The `ClientCapabilities` type in `types.rs` has fields for `fs_read_text_file` and `fs_write_text_file` (lines 149-152), proving the protocol types were planned. But no bridge code exists that uses them.
- **Remaining work**: Build `AcpFileSystem` as planned. Low complexity -- the transport layer exists, capability detection is already parsed, and the fallback is trivial `tokio::fs`.
- **Effort**: Small (1-2 hours). Would be useful for editors that want file operations to go through their FS layer (e.g., Zed buffer integration, remote workspaces).
- **Priority**: Low. The local filesystem fallback is what roko uses today and works fine.

---

### ACP10: Terminal Bridge

- **Planned scope**: `AcpTerminal` struct in `bridge_terminal.rs` -- routes shell commands through editor terminal via `terminal/create` / `terminal/output` / `terminal/wait_for_exit` / `terminal/kill` / `terminal/release`, with local `tokio::process::Command` fallback. ~350 LOC.
- **Current state**: **Missing**. No `bridge_terminal.rs` exists. The slash command dispatch in `bridge_events.rs` spawns processes directly via `tokio::process::Command`, which is effectively the fallback behavior this batch would have provided.
- **Remaining work**: Build `AcpTerminal` as planned. Medium complexity -- need to manage terminal lifecycle (create, track, kill, release), handle async output streaming.
- **Effort**: Medium (2-4 hours). The local process execution already works; the value is routing through the editor's terminal for visibility.
- **Priority**: Low-Medium. Useful for Cursor/Zed integration where the user wants to see process output in the editor terminal panel.

---

### ACP11: Permission Bridge

- **Planned scope**: `AcpPermissionGate` struct in `permissions.rs` -- sends `session/request_permission` to editor, caches permanent allow/deny decisions. ~250 LOC.
- **Current state**: **Exists -- fully built, exceeds spec**. Implemented directly in `bridge_events.rs` as `request_permission()` (line 1202) and `request_permission_for_event()` (line 1401). The `types.rs` defines `PermissionAction` enum (FileEdit, FileCreate, TerminalCommand, GitOperation, etc.), `PermissionDecision` enum (Allow, AlwaysAllow, Reject), `RequestPermissionParams`, and `PermissionResponse` with `decision_from_option_id()`. Permanent allow caching via `trusted_actions: HashSet<PermissionAction>`. Has 5+ unit tests covering pregranted actions, always-allow persistence, and malformed response handling.
- **Remaining work**: None. The implementation is richer than what ACP11 specified.
- **Effort**: 0.

---

### ACP12: Gate Result Bridge

- **Planned scope**: Functions in `bridge_gates.rs` -- `gate_started_notification()`, `gate_completed_notification()`, `format_gate_summary()` mapping gate events to `ToolCall`/`ToolCallUpdate` session updates with markdown summaries. ~300 LOC.
- **Current state**: **Exists -- different architecture**. Gate results are bridged through two paths:
  1. `runner.rs` emits `CognitiveEvent::ToolCallStart` and `CognitiveEvent::ToolCallComplete` during pipeline execution, which get streamed as ACP `tool_call` / `tool_call_update` session updates.
  2. `event_forward.rs` (`AcpEventForwarder`) maps `ToolCallStart` -> `RuntimeEvent::GateStarted` and `ToolCallComplete` -> `RuntimeEvent::GatePassed`/`GateFailed` for the runtime event bus.
  The markdown summary format is simpler than the batch planned (no structured gate-specific markdown templates) but functional.
- **Remaining work**: The structured markdown summaries from the spec (with bullet points for compile errors, test failures, clippy warnings) would be nice-to-have. Not blocking.
- **Effort**: Small (1 hour) for the markdown templates.

---

### ACP13: Plan Phase Bridge

- **Planned scope**: Functions in `bridge_plan.rs` -- `build_plan_entries()`, `phase_transition_notification()`, `task_status_to_plan_status()` mapping Roko plan phases to ACP `PlanEntry` notifications. ~250 LOC.
- **Current state**: **Exists -- fully built in `runner.rs`**. Two implementations:
  1. `workflow_plan_entries()` (line 944) builds plan entries from workflow template + current phase.
  2. `build_plan_entries()` (line 1637) builds entries from `WorkflowRun` state with per-phase status mapping.
  Both emit `CognitiveEvent::PlanUpdate { entries }` which the session update stream converts to ACP plan notifications. The `PlanEntry` and `PlanStatus` types are defined in `types.rs`.
- **Remaining work**: None. The implementation covers both static template entries and dynamic phase tracking.
- **Effort**: 0.

---

### ACP14: Usage/Cost Bridge

- **Planned scope**: `AcpUsageBridge` struct in `bridge_usage.rs` -- accumulates token counts and costs, emits `UsageUpdate` notifications, provides context window utilization warnings at 75%/90%/95% thresholds. ~200 LOC.
- **Current state**: **Partial**. The `SessionUpdate::UsageUpdate` type exists with `used`, `size`, and `cost` fields. `CostInfo` has `amount` and `currency`. Usage updates are emitted after each cognitive task (line 2158 of `bridge_events.rs`). Per-session cost accumulation exists (`accumulated_cost_usd` on session, with budget enforcement). However:
  - No dedicated `AcpUsageBridge` accumulator struct
  - No `ContextWarning` enum or utilization threshold warnings
  - No `should_warn_context()` function
  - Cost tracking is scattered across session state rather than centralized
- **Remaining work**: Add context utilization warnings (the thresholds at 75/90/95% would be genuinely useful). Could be a helper function on `AcpSession` rather than a separate module.
- **Effort**: Small (1-2 hours). The data is already there; just needs threshold logic + notification emission.

---

### ACP15: Session Config Options

- **Planned scope**: `config_options.rs` with 7 options: agent_mode, model_tier, thinking, gate_pipeline, auto_correct, knowledge_store, daimon. Dependent updates (mode -> gate, model -> thinking). ~400 LOC.
- **Current state**: **Exists -- evolved design**. `build_config_options()` in `session.rs` (line 1456) returns 6 options: provider, model, effort (thinking), workflow, clippy, tests. The options are different from what was planned:
  - `provider` replaces `model_tier` (more granular -- select actual provider)
  - `model` added (select specific model within provider)
  - `workflow` added (none/express/standard/full/auto)
  - `clippy` and `tests` replace the single `gate_pipeline` toggle
  - `agent_mode` is handled separately via `session/set_mode` (code/plan/research)
  - `auto_correct`, `knowledge_store`, `daimon` not exposed as config options
  Dependent updates are implemented (provider change filters available models).
  The `ConfigOption`, `ConfigOptionType`, `ConfigOptionValue` types are in `types.rs`. Handler supports `session/config/update` and `session/set_config_option`.
- **Remaining work**: Could add the 4 missing options from the spec (auto_correct, knowledge_store, daimon, budget limit) if the editor UI benefits from them.
- **Effort**: Small (1 hour per option). The infrastructure is fully in place.

---

### ACP16: Slash Commands

- **Planned scope**: 8 slash commands (/plan, /gate, /learn, /inspect, /replay, /heuristics, /status, /budget) in `commands.rs` with dynamic filtering and command parsing. ~300 LOC.
- **Current state**: **Exists -- massively superseded**. 44 slash commands in `session.rs` `build_slash_commands()` covering: system (status, doctor, config, models), research (research, search, enhance-prd, analyze), specification (prd-*), planning (plan-*), implementation (run, do, develop, agents, agent-*), verification (build, test, clippy, fmt, gate, review), knowledge (knowledge, knowledge-stats, dream, replay), learning (learn, learn-router, learn-episodes, learn-tune), workflow (workflow, express, full, review-this, pipeline), and utility (index, explain, note, help, audit). Bare mode filtering exists (limits to 8 safe commands for non-roko workspaces). Command dispatch happens in `bridge_events.rs` `run_slash_command()` (line 4298) with real CLI delegation via `tokio::process::Command`.
- **Remaining work**: None for the batch scope. The current implementation far exceeds it. Dynamic command filtering per config state (as the batch specified, e.g., "hide /gate when gate_pipeline disabled") is not implemented, but the bare mode system provides similar workspace-aware filtering.
- **Effort**: 0 for batch scope.

---

### ACP17: Elicitation Forms

- **Planned scope**: `elicitation.rs` -- `request_elicitation()` function that sends `elicitation/create` to the editor with JSON Schema forms, returns form data or None. Pre-built schemas for gate pipeline config and research source selection. ~300 LOC.
- **Current state**: **Missing**. No elicitation support exists anywhere in the crate. The ACP protocol supports elicitation forms (structured UI forms in the editor) but roko-acp does not send them.
- **Remaining work**: Full implementation as planned. This would enable:
  - Asking the user "which gates to run?" before a pipeline
  - "which research sources?" before research dispatch
  - Structured prompts for PRD creation
  - Any structured input collection vs free-text prompts
- **Effort**: Medium (2-3 hours). The transport layer supports sending requests to the editor and awaiting responses. JSON Schema definition is straightforward.
- **Priority**: Medium. Would significantly improve UX for configuration-heavy operations.

---

### ACP18: Lifecycle Integration Tests

- **Planned scope**: 10 integration tests in `tests/lifecycle.rs` covering config option changes, slash command flow, session load/resume, multi-session concurrency, error cases (invalid session, unknown method, malformed JSON), config validation, legacy set_mode, and dynamic command updates. ~500 LOC.
- **Current state**: **Partial -- different test files**. Existing tests:
  - `tests/protocol_conformance.rs` (522 lines, 11 tests): test_initialize, test_session_new, test_session_list, test_session_prompt_basic, test_session_cancel, test_unknown_method, test_invalid_session, test_malformed_json, plus 3 more (session config, multi-session).
  - `tests/telemetry_integration.rs` (191 lines, 5 tests): telemetry forwarding tests.
  - Unit tests in `bridge_events.rs`: 14+ tests including permission request tests, slash command streaming tests.
  The protocol conformance tests cover 5 of the 10 planned test cases (initialize, session new/list, unknown method, invalid session, malformed JSON). Missing:
  - Config option change flow with dependent updates
  - Slash command flow (send `/status`, verify response)
  - Session load/resume
  - Config option validation (invalid option ID, invalid value)
  - Legacy set_mode verification
  - Dynamic command filtering after mode change
- **Remaining work**: 5 more integration test cases. The test infrastructure (TestHarness, TestClient) is already built and reusable.
- **Effort**: Medium (2-3 hours). Infrastructure exists; just need new test functions.

---

## Phase 4-6 Features (Aspirational vs Achievable)

The implementation plan defines 3 post-MVP phases. Phases 1-3 (workflow runner core, gate integration, review integration) are **done** via the `pipeline.rs` + `runner.rs` + `workflow.rs` modules built by hand.

### Phase 4: Multi-Task Plans

**Planned**: Wire `roko-orchestrator` plan executor into ACP for multi-task DAG execution with per-task pipeline phases, dependency ordering, and merge queue.

**Status**: NOT STARTED. The plan executor exists in `roko-orchestrator` and the CLI runner (`roko-cli/src/runner/event_loop.rs`) uses it, but ACP sessions cannot trigger multi-task plan execution. The `/plan-run` slash command delegates to the CLI binary via subprocess, which works but doesn't provide the fine-grained per-task streaming that Phase 4 envisions.

**Achievable now?**: Partially. The pieces exist:
- `roko-orchestrator` has the DAG executor
- ACP `PlanEntry` types support per-task status tracking
- `WorkflowRun` tracks pipeline state
- The gap is wiring them together inside the ACP process rather than delegating to a subprocess

**Effort**: Large (1-2 days). Requires bridging the orchestrator's event loop with ACP's session update stream.

---

### Phase 5: Custom Workflows

**Planned**: Users define pipeline templates in `.roko/workflows/*.toml` with step-based execution, role/model/gate per step, and a template registry.

**Status**: NOT STARTED. The `WorkflowTemplate` enum in `pipeline.rs` has 4 hardcoded templates (Express, Standard, Full, Auto). No TOML-based workflow definition exists. The `roko-runtime` crate has a `WorkflowConfig` struct and `WorkflowEngine` that the ACP runner already uses, but these are code-defined, not user-configurable.

**Achievable now?**: Partially. The workflow engine exists and could be extended:
- Add TOML deserialization for `WorkflowTemplate` definitions
- Add a discovery scan of `.roko/workflows/`
- Expose discovered workflows as config options in the editor dropdown

**Effort**: Medium-Large (1 day). The execution engine works; the gap is configuration + discovery.

---

### Phase 6: Triggers

**Planned**: Workflows fire automatically from events -- file watch, manual trigger, workflow-completion chaining, background execution.

**Status**: NOT STARTED. The `notify::RecommendedWatcher` exists in the TUI (`tui/fs_watch.rs`) but is not connected to ACP sessions. PRD auto-plan triggers exist in `roko-serve` (`prd_publish_subscriber`) but operate through the HTTP control plane, not ACP.

**Achievable now?**: The file watch component exists. Wiring it to trigger an ACP workflow would require:
- A trigger registry per session
- File pattern matching
- Background workflow execution (no active prompt needed)
- Result persistence to `.roko/runs/`

**Effort**: Large (2+ days). The individual pieces exist but connecting them is non-trivial.

---

## Summary: What's Still Needed

### Quick Wins (< 2 hours each)

| Item | From Batch | Effort | Value |
|---|---|---|---|
| Context utilization warnings (75/90/95%) | ACP14 | 1-2h | Prevents context overflow |
| Structured gate result markdown templates | ACP12 | 1h | Better gate failure readability |
| Additional config options (auto_correct, budget) | ACP15 | 1h/each | More editor control surface |

### Medium Tasks (2-4 hours each)

| Item | From Batch | Effort | Value |
|---|---|---|---|
| Elicitation forms system | ACP17 | 2-3h | Structured input for config-heavy ops |
| Lifecycle integration tests (5 remaining) | ACP18 | 2-3h | Test coverage for config/command flows |
| Terminal bridge | ACP10 | 2-4h | Editor terminal integration |

### Not Worth Building Now

| Item | From Batch | Why |
|---|---|---|
| File system bridge | ACP09 | Local FS works fine; editor FS routing is niche |
| Phase 4 multi-task plans in ACP | Impl Plan | Better to use `/plan-run` subprocess delegation |
| Phase 5 custom TOML workflows | Impl Plan | Hardcoded templates cover real use cases |
| Phase 6 triggers | Impl Plan | TUI file watcher + HTTP triggers cover this differently |
