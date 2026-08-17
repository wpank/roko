# ACP: bridge_events.rs Refactoring Plan

> **Source**: `crates/roko-acp/src/bridge_events.rs` (8,430 LOC)
> **References**: `tmp/acp-features/00-ACP-FEATURES.md`, `tmp/acp-runner/`
> **Created**: 2026-08-15

## Problem

bridge_events.rs is 8,430 of 19,915 total LOC in roko-acp (42.3%). It contains
191 definitions spanning 8 distinct concerns that share almost no state. The test
module alone is 2,607 lines (lines 5824-8430). Functions range up to 737 lines
(`handle_session_prompt_inner`). The file is difficult to navigate, review, and
modify safely.

## Proposed Module Split

### 1. `dispatch.rs` -- Agent dispatch orchestration

The main prompt-handling entry point and inner dispatch loop that wires together
model resolution, cascade selection, experiments, tool capabilities, and
streaming. This is the "spine" of the ACP bridge.

- **Functions/types**:
  - `handle_session_prompt()` (L1627-1649) -- public entry point
  - `handle_session_prompt_inner()` (L1651-2387) -- 737-line inner impl
  - `acp_role_for_mode()` (L653-662)
  - `derive_acp_tool_capabilities()` (L664-693)
  - `acp_routing_context()` (L695-760)
  - `acp_dispatch_succeeded()` (L761-772)
  - `config_with_session_effort()` (L3049-3056)
- **Lines**: ~653-760, 1627-2387, 3049-3056
- **LOC**: ~930
- **Dependencies**: types, streaming, anthropic_provider, openai_provider, model_routing, experiments, episode_logging, context_resolution, provenance

### 2. `model_routing.rs` -- Cascade router and model resolution

All cascade router integration, model slug resolution, provider-aware candidate
filtering, and reward/observation recording.

- **Functions/types**:
  - `AcpCascadeSelection` (L969-973)
  - `cascade_router_model_slugs()` (L912-919)
  - `acp_model_providers()` (L921-932)
  - `provider_near_rate_limit()` (L934-940)
  - `rate_aware_model_candidates()` (L942-967)
  - `acp_cascade_selection_enabled()` (L974-984)
  - `cascade_select_model()` (L986-1069)
  - `resolve_acp_dispatch_model()` (L1071-1095)
  - `compute_acp_reward()` (L1097-1117)
  - `record_cascade_observation()` (L1119-1157) -- async, spawns tokio task
  - `CASCADE_ROUTER_IO_LOCK` (L152)
- **Lines**: ~912-1157, 152, 969-984
- **LOC**: ~265
- **Dependencies**: types (AcpCascadeSelection struct could live here or in types)

### 3. `experiments.rs` -- Prompt A/B experiment support

Experiment assignment, applicability checks, rendering, and outcome recording.

- **Functions/types**:
  - `AcpExperimentAssignment` (L774-780)
  - `experiment_store_lock()` (L782-791)
  - `assign_acp_experiment()` (L793-817)
  - `experiment_model_key()` (L819-835)
  - `applicable_acp_experiment()` (L837-872)
  - `render_experiment_context()` (L874-882)
  - `record_acp_experiment_outcome()` (L884-910)
  - `EXPERIMENT_STORE_IO_LOCK` (L153)
- **Lines**: ~774-910, 153
- **LOC**: ~140
- **Dependencies**: none (self-contained)

### 4. `episode_logging.rs` -- Episode and efficiency event recording

Episode construction, efficiency event emission, dream consolidation triggering,
and cost calculation.

- **Functions/types**:
  - `pricing_table()` (L283-296)
  - `calculate_cost_for_model_slug()` (L297-309) -- pub
  - `calculate_cost_without_cache_for_model_slug()` (L311-324)
  - `append_acp_episode()` (L326-487) -- 162 lines
  - `DREAM_EPISODE_THRESHOLD` (L488)
  - `maybe_spawn_dream_consolidation()` (L493-564)
  - `acp_efficiency_event()` (L566-619)
  - `emit_acp_efficiency_event()` (L621-651)
- **Lines**: ~283-651
- **LOC**: ~370
- **Dependencies**: model_routing (for AcpCascadeSelection), types

### 5. `anthropic_provider.rs` -- Anthropic-specific dispatch path

The Anthropic cognitive task runner, tool loop, model call config building, and
model stream forwarding.

- **Functions/types**:
  - `run_anthropic_cognitive_task()` (L2388-2473)
  - `run_anthropic_tool_loop()` (L2474-2670) -- 197 lines
  - `anthropic_model_call_config()` (L2671-2707)
  - `model_call_request_from_acp_messages()` (L2708-2724)
  - `model_call_chat_message_from_acp()` (L2725-2760)
  - `ModelStreamForward` enum (L2762-2766)
  - `ModelStreamForwardState` struct (L2768-2770)
  - `stream_model_call_to_cognitive_events()` (L2772-2828)
  - `forward_model_stream_event()` (L2830-2888)
  - `usage_info_from_model_usage()` (L2890-2903)
  - `acp_stop_reason_from_model()` (L2905-2930)
  - `build_anthropic_content_parts()` (L5505-5531)
- **Lines**: ~2388-2930, 5505-5531
- **LOC**: ~570
- **Dependencies**: types, streaming

### 6. `openai_provider.rs` -- OpenAI-compat dispatch path

The OpenAI-compat cognitive task runner, MCP tool loop, builtin tool loop, and
stream chunk forwarding.

- **Functions/types**:
  - `run_openai_compat_cognitive_task()` (L2932-3048) -- 117 lines
  - `openai_compat_tool_loop_supported()` (L3058-3064)
  - `run_openai_compat_mcp_tool_loop()` (L3066-3250) -- 185 lines
  - `run_openai_compat_builtin_tool_loop()` (L3251-3403) -- 153 lines
  - `forward_tool_loop_stream_chunks()` (L3405-3425)
  - `usage_info_from_tool_loop_usage()` (L3427-3445)
  - `build_openai_content_parts()` (L5478-5500)
- **Lines**: ~2932-3445, 5478-5500
- **LOC**: ~540
- **Dependencies**: types, streaming, mcp_tools

### 7. `mcp_tools.rs` -- Session MCP tool setup and tool handlers

MCP server discovery, tool name sanitization, handler/resolver structs, and
builtin tool handler/resolver structs.

- **Functions/types**:
  - `write_session_mcp_config()` (L3448-3497)
  - `SessionMcpRuntime` struct (L3498-3501)
  - `setup_session_mcp_tools()` (L3503-3660)
  - `sanitize_tool_segment()` (L3662-3676)
  - `unique_tool_name()` (L3678-3695)
  - `AcpMcpHandlerResolver` (L3697-3705)
  - `AcpMcpToolHandler` (L3707-3767)
  - `AcpToolCancelToken` (L3769-3779)
  - `AcpBuiltinToolHandler` (L3782-3882)
  - `AcpBuiltinHandlerResolver` (L3884-3892)
  - `tool_result_from_mcp()` (L3894-3908)
  - `mcp_result_text()` (L3910-3921)
  - `tool_result_for_editor()` (L3923-3928)
- **Lines**: ~3448-3928
- **LOC**: ~480
- **Dependencies**: types, streaming (for CognitiveEvent in permission flow)

### 8. `provenance.rs` -- Provenance chain construction and rendering

Knowledge card emission, provenance source assembly, and card rendering for
the evidence chain surfaced in dispatch.

- **Functions/types**:
  - `emit_knowledge_card()` (L3930-3956)
  - `ProvenanceChain` struct (L3958-3963)
  - `ProvenanceSource` enum (L3965-3990)
  - `build_provenance()` (L3992-4164) -- 173 lines
  - `emit_provenance_card()` (L4166-4188)
  - `render_provenance_card()` (L4190-4261) -- 72 lines
  - `prompt_keywords()` (L4263-4279)
  - `knowledge_tier_label()` (L4281-4288)
  - `score_to_confidence()` (L4290-4296)
- **Lines**: ~3930-4296
- **LOC**: ~370
- **Dependencies**: types

### 9. `slash_commands.rs` -- Slash command execution and streaming

The `/plan-run`, `/build`, `/test`, `/clippy`, and custom slash command runner
plus all stdout/stderr stream forwarding and progress event correlation.

- **Functions/types**:
  - `run_slash_command()` (L4298-4970) -- **673 lines** (second largest fn)
  - `SlashCommandStreamOutcome` enum (L4971-4975)
  - `forward_slash_command_streams()` (L4976-5148) -- 173 lines
  - `pop_progress_call()` (L5150-5158)
  - `close_progress_calls()` (L5161-5181)
  - `finish_slash_command_stream()` (L5183-5201)
  - `run_shell_command()` (L5205-5316) -- 112 lines
- **Lines**: ~4298-5316
- **LOC**: ~1,020
- **Dependencies**: types, streaming

### 10. `context_resolution.rs` -- File/resource/mention context resolution

Prompt text extraction, image part building, resource URI extraction, file
reading with workdir sandboxing, @-mention parsing and resolution.

- **Functions/types**:
  - `extract_prompt_text()` (L5456-5473)
  - `inject_image_parts()` (L5537-5553)
  - `extract_resource_uris()` (L5556-5567)
  - `read_file_context()` (L5571-5611)
  - `resolve_context_items()` (L5618-5651) -- pub(crate)
  - `resolve_file_uri()` (L5653-5661)
  - `resolve_at_mention()` (L5663-5703)
  - `resolve_local_file_contents()` (L5706-5737)
  - `extract_at_mentions()` (L5739-5780)
  - `truncate_with_limit()` (L5782-5796)
  - `ensure_git_output_success()` (L5798-5810)
- **Lines**: ~5456-5810
- **LOC**: ~355
- **Dependencies**: types (ContentBlock)

### 11. Remaining in `bridge_events.rs` -- Core types, streaming, and event mapping

What stays in bridge_events.rs: the shared types, event enum, streaming loop,
permission flow, and event-to-update mapping. These are the glue that every
other module calls through.

- **Functions/types**:
  - `BridgeEventsError` enum + impl (L86-147)
  - `CognitiveEvent` enum (L159-204)
  - `PermissionRequestPayload` struct (L206-220)
  - `PermissionReplyChannel` struct + impl (L222-273)
  - `StreamResult` struct (L275-281)
  - `StreamAction` enum (L1467)
  - `request_permission()` (L1202-1399) -- 198 lines
  - `request_permission_for_event()` (L1401-1449)
  - `stream_events_to_editor()` (L1451-1625) -- 175 lines
  - `map_event_to_update()` (L5331-5372)
  - `dispatch_failure_update()` (L5374-5379)
  - `format_acp_error_for_user()` (L5383-5423)
  - `emit_dispatch_failure()` (L5425-5428)
  - `send_cognitive_event()` (L5430-5434)
  - `send_session_update()` (L5436-5454)
  - `truncate_to_title()` (L1159-1177)
  - `truncate_assistant_history()` (L1179-1200)
  - `workflow_template_name()` (L5812-5818)
  - `text_block()` (L5820-5822)
  - `MAX_HISTORY_ASSISTANT_BYTES` (L150)
- **Lines**: ~86-281, 1159-1625, 5318-5423, 5430-5822
- **LOC**: ~800 (from ~5,200 non-test lines, minus ~4,400 extracted)

### 12. `tests/` -- Test module (consider submodule files)

The `#[cfg(test)] mod tests` block is 2,607 lines (L5824-8430) containing 46
test functions and 4 test helpers. Tests should be split to follow their
corresponding modules.

- **Lines**: 5824-8430
- **LOC**: 2,607
- **Test groupings by proposed module**:
  - dispatch tests: `handle_session_prompt_rejects_busy_sessions`, `cost_budget_exhaustion_rejects_before_provider_dispatch`, `cost_budget_accumulates_exact_efficiency_event_cost`
  - model_routing tests: `cascade_select_model_*` (5 tests), `cascade_router_model_slugs_*` (2), `resolved_acp_dispatch_*` (2), `rate_limit_provider_selection_*`, `cascade_observation_*`
  - experiments tests: `experiment_assignment_*`, `acp_conformance` (partial)
  - episode_logging tests: `append_acp_episode_*` (2), `calculate_cost_*`, `acp_routing_context_*` (2), `acp_dispatch_reward_*`
  - anthropic_provider tests: `anthropic_model_call_config_*` (2), `model_stream_*` (3), `model_call_request_*`
  - mcp_tools tests: `anthropic_session_mcp_tools`, `session_mcp_tool_names_*`, `capabilities_reflect_session`, `acp_builtin_tool_handler_*` (2)
  - provenance tests: `build_provenance_*` (2)
  - slash_commands tests: `slash_command_streaming_*` (5), `slash_command_empty_output_*`
  - context_resolution tests: `extract_at_mentions_*`, `resolve_context_items_*`, `truncate_with_limit_*`
  - streaming/permission tests: `stream_events_to_editor_*` (4), `send_session_update_*`, `request_permission_*` (4), `permission_*` (5)
  - multi-concern (keep in bridge_events or move to integration): `acp_conformance` (190 lines, tests experiments+MCP+capabilities+permissions)

## Duplicated Logic

### 1. Workdir-sandboxed path canonicalization (2 copies)

`read_file_context()` (L5571-5611) and `resolve_local_file_contents()` (L5706-5737) both
perform the same canonicalize-workdir, canonicalize-path, starts_with check, strip_prefix
sequence. Extract into a shared `sandboxed_path(path, workdir) -> Result<(PathBuf, PathBuf)>`
that returns (canonical, relative).

### 2. Interleaved stdout/stderr streaming loop (2 copies)

`forward_slash_command_streams()` (L4976-5148) and `run_shell_command()` (L5205-5316) both
use the identical pattern: `BufReader::new(child.stdout.take())`, `BufReader::new(child.stderr.take())`,
`stdout_done`/`stderr_done` flags, `tokio::select! { biased; cancelled => ..., stdout => ..., stderr => ... }`.
The only differences are: (a) progress event parsing in forward_slash_command_streams, and (b) child.kill()
in run_shell_command. Extract a generic `interleave_process_output()` that takes a line-processor callback.

### 3. UsageInfo construction (2 converter functions)

`usage_info_from_model_usage()` (L2890-2903) converts `TokenUsage`, while
`usage_info_from_tool_loop_usage()` (L3427-3445) converts `roko_core::Usage`. These are
structurally identical mappings. Consider a `From<TokenUsage>` and `From<Usage>` impl on
UsageInfo instead.

### 4. Image content part building (parallel Anthropic/OpenAI functions)

`build_anthropic_content_parts()` (L5505-5531) and `build_openai_content_parts()` (L5478-5500)
share the same has-image check, iteration pattern, and text-part construction. Only the image
object shape differs. Consolidate into a single function that takes an image-formatter closure
or a `ProviderKind` discriminant (which `inject_image_parts` already does at the call site).

### 5. Repeated `event_sender.send(...).await` / `let _ = event_sender.send(...)` pattern

Appears 50+ times throughout the file. The `send_cognitive_event()` helper (L5430-5434) exists
but is only called in 2 places. Most call sites use raw `event_sender.send(...).await` with
`let _ =` to ignore errors. Consistently use the helper or an extension trait.

## Oversized Functions

| Function | Lines | Start | End | What it does | Suggested split |
|---|---|---|---|---|---|
| `handle_session_prompt_inner` | 737 | 1651 | 2387 | Full dispatch pipeline: validate -> resolve model -> cascade select -> experiment -> build context -> choose provider path -> dispatch -> record episode/efficiency -> record cascade observation | Split into phases: (1) `resolve_dispatch_config()` (~L1651-1900, model+cascade+experiment resolution), (2) `build_dispatch_context()` (~L1900-2100, system prompt, knowledge, provenance, context items), (3) `execute_and_record()` (~L2100-2387, provider dispatch + episode/efficiency recording) |
| `run_slash_command` | 673 | 4298 | 4970 | Parse slash command string, resolve subcommand (plan-run, build, test, clippy, custom), spawn child process, forward streams | Split into: (1) `parse_slash_command()` -> enum of recognized commands, (2) per-command handlers `run_slash_plan_run()`, `run_slash_build_test_clippy()`, (3) `run_slash_custom()` for user-defined commands. The common child-spawn+stream pattern is already handled by `forward_slash_command_streams`. |
| `run_anthropic_tool_loop` | 197 | 2474 | 2670 | Iterate tool calls from Anthropic model stream, dispatch via MCP or builtin handler, collect results, re-prompt | Acceptable size but could split tool-call dispatch and result collection into a helper |
| `run_openai_compat_mcp_tool_loop` | 185 | 3066 | 3250 | Same pattern as anthropic tool loop but for OpenAI-compat providers with MCP tools | Consider factoring common loop structure with anthropic tool loop |
| `stream_events_to_editor` | 175 | 1451 | 1625 | Read CognitiveEvent channel, handle permission requests, map to SessionUpdate, send via transport | Acceptable size, well-structured |
| `build_provenance` | 173 | 3992 | 4164 | Assemble provenance chain from playbooks, episodes, knowledge hits, dream patterns | Acceptable, but each source-type assembly could be a helper |
| `forward_slash_command_streams` | 173 | 4976 | 5148 | Parse ROKO_PROGRESS protocol lines, map to tool call events, interleave stdout/stderr | The progress-line parser (L5018-5104) could be extracted as `parse_progress_line()` |
| `setup_session_mcp_tools` | 158 | 3503 | 3660 | Connect to MCP servers, discover tools, build handler map | Acceptable but border-line |
| `append_acp_episode` | 162 | 326 | 487 | Construct Episode struct, compute HDC fingerprint, log episode, spawn dream consolidation | Could split HDC computation and dream triggering into separate helpers |
| `request_permission` | 198 | 1202 | 1399 | Send permission request to editor, await response or cancellation, persist always-allow | Could extract `persist_always_allow()` and `parse_permission_response()` as helpers |

## Types That Could Move

| Type | Current location | Suggested destination | Reason |
|---|---|---|---|
| `CognitiveEvent` | bridge_events.rs L159 | types.rs or new `events.rs` | Used by every module in the crate; core ACP event type |
| `PermissionRequestPayload` | bridge_events.rs L206 | types.rs | Pure data struct, no behavior |
| `PermissionReplyChannel` | bridge_events.rs L222 | types.rs | Used by handler.rs, builtin_tools.rs, event_forward.rs |
| `StreamResult` | bridge_events.rs L275 | types.rs | Pure data struct |
| `BridgeEventsError` | bridge_events.rs L86 | types.rs or errors.rs | Error type used by multiple modules |
| `AcpCascadeSelection` | bridge_events.rs L969 | model_routing.rs | Only used by model routing functions |
| `AcpExperimentAssignment` | bridge_events.rs L774 | experiments.rs | Only used by experiment functions |
| `ProvenanceChain` / `ProvenanceSource` | bridge_events.rs L3958/L3965 | provenance.rs | Only used by provenance functions |
| `SlashCommandStreamOutcome` | bridge_events.rs L4971 | slash_commands.rs | Only used by slash command functions |
| `ModelStreamForward` / `ModelStreamForwardState` | bridge_events.rs L2762/L2768 | anthropic_provider.rs | Only used by Anthropic model stream |
| `SessionMcpRuntime` | bridge_events.rs L3498 | mcp_tools.rs | Only used by MCP setup |

## Migration Steps

### Phase 1: Extract types (low risk, no logic changes)

1. Move `CognitiveEvent`, `PermissionRequestPayload`, `PermissionReplyChannel`,
   `StreamResult`, `BridgeEventsError`, and `Result<T>` into `types.rs` (or a
   new `events.rs`). Add `pub use` re-exports from `bridge_events.rs` so
   external callers are unaffected.
2. Run `cargo test -p roko-acp` to verify.

### Phase 2: Extract leaf modules (no cross-module deps)

3. Extract `experiments.rs` -- self-contained, no deps on other proposed modules.
4. Extract `context_resolution.rs` -- only depends on `ContentBlock` from types.
5. Extract `provenance.rs` -- only depends on types and external crates.
6. Run `cargo test -p roko-acp` after each extraction.

### Phase 3: Extract provider modules

7. Extract `anthropic_provider.rs` -- depends on types and streaming helpers.
8. Extract `openai_provider.rs` -- depends on types, streaming, and mcp_tools.
9. Extract `mcp_tools.rs` -- depends on types and CognitiveEvent.
10. Run `cargo test -p roko-acp` after each.

### Phase 4: Extract domain modules

11. Extract `episode_logging.rs` -- depends on types and model_routing (for
    AcpCascadeSelection).
12. Extract `model_routing.rs` -- depends on types.
13. Extract `slash_commands.rs` -- depends on types and streaming.
14. Run `cargo test -p roko-acp` after each.

### Phase 5: Extract dispatch orchestration

15. Extract `dispatch.rs` -- depends on all other modules. This is the "spine"
    that calls into everything else. Extract last because it has the most deps.
16. Run `cargo test -p roko-acp`.

### Phase 6: Deduplicate

17. Consolidate workdir-sandboxed canonicalization into a shared helper in
    `context_resolution.rs`.
18. Consolidate interleaved stdout/stderr streaming into a generic helper in
    `slash_commands.rs` (used by both `forward_slash_command_streams` and
    `run_shell_command`).
19. Consolidate `build_anthropic_content_parts` / `build_openai_content_parts`
    into a single parameterized function.
20. Replace raw `event_sender.send(...)` with consistent use of
    `send_cognitive_event()`.

### Phase 7: Break up oversized functions

21. Split `handle_session_prompt_inner` into 3 phases (see Oversized Functions
    table above).
22. Split `run_slash_command` into parser + per-command handlers.
23. Extract `parse_progress_line()` from `forward_slash_command_streams`.

### Phase 8: Relocate tests

24. Move test functions to `#[cfg(test)] mod tests` blocks within their
    respective new modules. Keep the multi-concern `acp_conformance` test in
    bridge_events.rs or promote it to an integration test in `tests/`.

### Expected result

After all phases, `bridge_events.rs` shrinks from 8,430 to ~800 lines (the
core streaming/permission/event-mapping glue). The 11 new modules average ~350
lines each. Test modules follow their source modules.

| Module | Est. LOC | Public surface |
|---|---|---|
| bridge_events.rs (residual) | ~800 | stream_events_to_editor, request_permission, handle_session_prompt (re-export) |
| dispatch.rs | ~930 | handle_session_prompt |
| model_routing.rs | ~265 | cascade_select_model, resolve_acp_dispatch_model |
| experiments.rs | ~140 | assign_acp_experiment, record_acp_experiment_outcome |
| episode_logging.rs | ~370 | append_acp_episode, calculate_cost_for_model_slug |
| anthropic_provider.rs | ~570 | run_anthropic_cognitive_task |
| openai_provider.rs | ~540 | run_openai_compat_cognitive_task |
| mcp_tools.rs | ~480 | setup_session_mcp_tools |
| provenance.rs | ~370 | build_provenance, render_provenance_card |
| slash_commands.rs | ~1,020 | run_slash_command |
| context_resolution.rs | ~355 | resolve_context_items, extract_prompt_text |
| tests (distributed) | ~2,607 | -- |
