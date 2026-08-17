# ACP: Code Quality & Clippy Fixes

> **Source**: All roko-acp source files (`crates/roko-acp/src/`)
> **Created**: 2026-08-15

## Clippy Errors (must fix)

Clippy cannot currently run against `roko-acp` because its dependency `roko-agent` has
two compilation errors that block the entire check. These must be fixed first before
any roko-acp-specific clippy analysis can proceed.

### Blocking upstream errors (in roko-agent, not roko-acp)

```
error[E0425]: cannot find type `Command` in this scope
 --> crates/roko-agent/src/harness/child_process_runner.rs:35:35
    pub fn apply(&self, cmd: &mut Command) {
                                  ^^^^^^^ not found in this scope
    help: consider importing this struct: use std::process::Command;

error[E0015]: cannot call non-const operator in constant functions
 --> crates/roko-agent/src/process/limits.rs:48:16
    || self.network == ProviderNetworkPolicy::Deny
       ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
    note: calls in constant functions are limited to constant functions

warning: unused import: `Command`
 --> crates/roko-agent/src/cursor_cli_agent.rs:32:29

warning: unused import: `tokio::process::Command`
 --> crates/roko-agent/src/exec.rs:27:5

warning: unused import: `std::process::Stdio`
 --> crates/roko-agent/src/openclaw/probe.rs:16:5
```

**Action required**: Fix the 2 errors + 3 warnings in `roko-agent` before re-running
`cargo clippy -p roko-acp --no-deps -- -D warnings`.

### Upstream warnings (in roko-plugin)

```
warning: `roko-plugin` (lib) generated 15 warnings
  (all: missing documentation for struct fields in crates/roko-plugin/src/registry.rs)
```

## Clippy Suppression Audit

| File | Line | Annotation | Function | Justified? | Action |
|---|---|---|---|---|---|
| `bridge_events.rs` | 325 | `#[allow(clippy::too_many_arguments)]` | `append_acp_episode` (12 params) | Partially -- the function takes many related params that could be grouped | Consider introducing an `EpisodeContext` struct |
| `bridge_events.rs` | 2387 | `#[allow(clippy::too_many_arguments)]` | `run_anthropic_cognitive_task` (12 params) | Partially -- common pattern for provider dispatch | Consider a `DispatchParams` struct |
| `bridge_events.rs` | 2473 | `#[allow(clippy::too_many_arguments)]` | `run_anthropic_tool_loop` (14 params) | Same as above | Same -- `DispatchParams` struct |
| `bridge_events.rs` | 2931 | `#[allow(clippy::too_many_arguments)]` | `run_openai_compat_cognitive_task` (13 params) | Same as above | Same -- `DispatchParams` struct |
| `bridge_events.rs` | 3065 | `#[allow(clippy::too_many_arguments)]` | `run_openai_compat_mcp_tool_loop` (14 params) | Same as above | Same -- `DispatchParams` struct |
| `bridge_events.rs` | 3250 | `#[allow(clippy::too_many_arguments)]` | `run_openai_compat_builtin_tool_loop` (14 params) | Same as above | Same -- `DispatchParams` struct |
| `runner.rs` | 1107 | `#[allow(clippy::too_many_arguments)]` | `run_workflow_pipeline` (11 params) | Yes -- orchestration entry point; a config struct exists (`PipelineConfig`) but session/channel params are runtime-only | Acceptable; could bundle session/channel args |
| `runner.rs` | 1952 | `#[allow(clippy::too_many_arguments)]` | `run_agent_phase` (10 params) | Same as above | Same |

**Summary**: 8 total `#[allow(clippy::*)]` annotations, all `too_many_arguments`. The 6 in
`bridge_events.rs` share overlapping parameter sets across Anthropic and OpenAI dispatch
paths -- a `DispatchParams` / `CognitiveTaskContext` struct would eliminate all 6 and
improve readability.

## Long Functions (>100 lines)

| File | Line | Function | Lines | Suggested split |
|---|---|---|---|---|
| `bridge_events.rs` | 1651 | `handle_session_prompt_inner` | **731** | **Critical** -- extract: model resolution, history assembly, tool setup, MCP setup, provider dispatch, episode logging, slash command routing |
| `bridge_events.rs` | 4298 | `run_slash_command` | **671** | **Critical** -- extract each slash-command handler into its own function (currently a giant match with inline logic) |
| `runner.rs` | 1108 | `run_workflow_pipeline` | **415** | **High** -- extract: gate execution, commit logic, plan-entry emission, per-phase dispatch |
| `bridge_events.rs` | 1451 | `stream_events_to_editor` | 172 | Medium -- reasonably cohesive event-loop |
| `bridge_events.rs` | 326 | `append_acp_episode` | 159 | Medium -- could extract model routing and cost computation |
| `bridge_events.rs` | 1202 | `request_permission` | 194 | Medium -- permission protocol is a single flow |
| `bridge_events.rs` | 2474 | `run_anthropic_tool_loop` | 196 | Low -- cohesive tool loop |
| `bridge_events.rs` | 3066 | `run_openai_compat_mcp_tool_loop` | 176 | Low -- cohesive tool loop |
| `bridge_events.rs` | 3503 | `setup_session_mcp_tools` | 158 | Medium -- extract MCP discovery and init into sub-functions |
| `bridge_events.rs` | 3992 | `build_provenance` | 172 | Medium -- provenance assembly is testable in isolation |
| `bridge_events.rs` | 4976 | `forward_slash_command_streams` | 173 | Low -- streaming loop |
| `bridge_events.rs` | 5205 | `run_shell_command` | 112 | Low -- shell execution is self-contained |
| `bridge_events.rs` | 3251 | `run_openai_compat_builtin_tool_loop` | 153 | Low -- cohesive tool loop |
| `runner.rs` | 686 | `consume` (EventConsumerImpl) | 200 | Medium -- could split per-event-type branches |
| `runner.rs` | 425 | `analyze_gate_failure` | 116 | Low -- forensic analysis is reasonably cohesive |
| `runner.rs` | 306 | `classify_gate_error` | 101 | Low -- sequential pattern matching |
| `handler.rs` | 270 | `handle_request` | 206 | Medium -- extract per-method handlers into named functions |
| `handler.rs` | 77 | `run_acp_server_with_transport` | 156 | Low -- main event loop |
| `pipeline.rs` | 195 | `step` | 188 | Low -- state machine transitions are best in one place |
| `session.rs` | 199 | `from_roko_config_with_warnings` | 110 | Low -- configuration logic |
| `session.rs` | 1456 | `build_config_options` | 208 | Medium -- extract per-option builders |
| `session.rs` | 1724 | `build_slash_commands` | 301 | Medium -- extract command groups |
| `acp_adapter.rs` | 38 | `map_event` | 112 | Low -- single match expression |

**Critical**: `handle_session_prompt_inner` at 731 lines and `run_slash_command` at 671 lines
are the largest functions in the entire crate. Both are in `bridge_events.rs` (8,430 lines total),
which is itself the largest file in the crate by far and should be considered for module extraction.

## TODO/FIXME/HACK Comments

No `TODO`, `FIXME`, or `HACK` comments found in any roko-acp source file.

## Dead Code / Unused

### No `#[allow(dead_code)]` annotations found.

### Potential unused code (manual audit)

- `bridge_events.rs` has `#[cfg(test)]` imports for `DEFAULT_TTFT_TIMEOUT_MS`,
  `DEFAULT_CONNECT_TIMEOUT_MS`, `DEFAULT_REQUEST_TIMEOUT_MS` -- these are test-only but
  properly gated.
- `types.rs:15` exports `INVALID_REQUEST` constant -- grep shows no usage outside the
  declaration. Likely dead code or future-proofing for the ACP spec.
- `knowledge.rs:128` defines `prepend_context()` -- used only from `runner.rs`. The public
  visibility is `pub(crate)` which is appropriate.
- `bridge_events.rs` line 152 has two `OnceLock<Mutex<()>>` statics (`CASCADE_ROUTER_IO_LOCK`,
  `EXPERIMENT_STORE_IO_LOCK`) -- verify these are actually used in the 8430-line file.

### Files with modified status (from `git status`)

The following roko-acp files have unstaged modifications on the current branch:
- `bridge_events.rs`
- `builtin_tools.rs`
- `handler.rs`
- `runner.rs`
- `session.rs`
- `transport.rs`
- `types.rs`
- `tests/telemetry_integration.rs`

## Missing Documentation

### Public functions missing `///` doc comment

| File | Line | Item |
|---|---|---|
| `event_forward.rs` | 31 | `pub fn new(sink, run_id, agent_id)` on `AcpEventForwarder` |
| `runner.rs` | 563 | `pub async fn run_with_workflow_engine(...)` |

### Public struct fields missing `///` doc comment

Most public struct fields in `types.rs`, `session.rs`, `pipeline.rs`, `workflow.rs`,
`config.rs`, and `runner.rs` **do** have doc comments. The crate uses `#![warn(missing_docs)]`
transitively via workspace lint config. Two notable gaps:

| File | Item | Notes |
|---|---|---|
| `runner.rs:49-62` | `PipelineConfig` struct fields | All 7 fields are `pub` but only `review_strictness` has a comment; the rest use field-name-only style |

## File Size Concerns

| File | Lines | Concern |
|---|---|---|
| `bridge_events.rs` | **8,430** | Far too large for a single module. Contains: error types, cognitive events, session prompt handling, provider dispatch (Anthropic + OpenAI), MCP setup, slash commands, permission protocol, provenance, episode logging, and ~2400 lines of tests. Should be split into at least 4-5 submodules. |
| `runner.rs` | **2,494** | Large but more cohesive (pipeline execution). Could extract git helpers, gate analysis, and agent phase dispatch. |
| `session.rs` | **2,839** | Large. Contains session state, config state, session manager, slash commands, config options, MCP discovery, and persistence. Could extract config/slash-command builders. |
| `types.rs` | **1,321** | Acceptable for a protocol types file. |
| `builtin_tools.rs` | **1,144** | Acceptable. Tool defs + handlers + tests. |

## Naming & Style

- Naming is consistent throughout: `snake_case` for functions/fields, `CamelCase` for types.
- All error types use `thiserror` derive macros consistently.
- Serde annotations use `rename_all = "camelCase"` consistently for ACP protocol types.
- No naming inconsistencies found.

## Cleanup

- [x] `bridge_events.rs.orig` does NOT exist on disk (was listed in git status snapshot but has since been cleaned up)
- [ ] Fix 2 compile errors in `roko-agent` to unblock clippy for `roko-acp`
- [ ] Re-run `cargo clippy -p roko-acp --no-deps -- -D warnings` after fixing `roko-agent`
- [ ] Introduce `DispatchParams` struct to eliminate 6 of 8 `#[allow(clippy::too_many_arguments)]` in `bridge_events.rs`
- [ ] Split `bridge_events.rs` (8,430 lines) into submodules: `dispatch/`, `slash_commands.rs`, `permission.rs`, `provenance.rs`, `episode.rs`
- [ ] Extract `handle_session_prompt_inner` (731 lines) into smaller functions
- [ ] Extract `run_slash_command` (671 lines) into per-command handlers
- [ ] Add doc comments to `AcpEventForwarder::new()` and `run_with_workflow_engine()`
- [ ] Add doc comments to `PipelineConfig` struct fields
- [ ] Audit `INVALID_REQUEST` constant for dead code
- [ ] Verify `CASCADE_ROUTER_IO_LOCK` and `EXPERIMENT_STORE_IO_LOCK` statics are used
