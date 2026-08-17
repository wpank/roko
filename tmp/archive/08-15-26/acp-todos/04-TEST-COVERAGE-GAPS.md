# 04 -- Test Coverage Gaps: roko-acp

Generated: 2026-08-15

## Coverage Summary

| Module | LOC | Tests (inline) | Tests (external) | Estimated Coverage | Notes |
|--------|-----|-----------------|-------------------|--------------------|-------|
| bridge_events.rs | 8430 | 58 | 5 (telemetry) | ~45% | Largest file; many internal helpers and streaming paths untested |
| session.rs | 2839 | 34 | 2 (conformance) | ~55% | Good coverage of config/budget; weak on persistence edge cases |
| runner.rs | 2494 | 5 | 0 | ~15% | Very low; `run_with_workflow_engine` and `run_workflow_pipeline` untested |
| types.rs | 1321 | 7 | 0 | ~40% | Serialization round-trips tested; many type methods untested |
| builtin_tools.rs | 1144 | 10 | 0 | ~50% | Safety policy + permissions tested; `execute_acp_builtin_tool` paths weak |
| handler.rs | 670 | 1 | 11 (conformance) | ~60% | External conformance tests cover most request methods; internal helpers untested |
| event_forward.rs | 586 | 19 | 0 | ~85% | Best inline coverage; all CognitiveEvent variants mapped and tested |
| pipeline.rs | 538 | 10 | 0 | ~75% | State machine transitions well-tested; some edge cases missing |
| config.rs | 521 | 8 | 3 (conformance) | ~70% | Config loading + sources well-tested; `load_roko_config_with_warning` error paths untested |
| knowledge.rs | 412 | 3 | 0 | ~50% | Card rendering and empty-store tested; actual store queries untested |
| transport.rs | 362 | 4 | 0 | ~45% | Read/write tested; `send_response`, `send_error`, `send_request` round-trip gaps |
| acp_adapter.rs | 250 | 3 | 0 | ~50% | Run ID filtering and basic mapping tested; many RuntimeEvent variants untested |
| config_watch.rs | 167 | 0 | 0 | 0% | **ZERO COVERAGE** |
| workflow.rs | 158 | 2 | 0 | ~40% | Constructor + summary tested; `mark_complete`, `is_done`, `elapsed` untested |
| lib.rs | 23 | 0 | 0 | N/A | Module declarations only |
| **TOTALS** | **19,915** | **164** | **16** | **~40%** | |

External test files:
- `tests/protocol_conformance.rs`: 11 tests (initialize, session/new, session/list, session/prompt, cancel, unknown method, invalid session, malformed JSON, no roko.toml, malformed toml, unavailable provider)
- `tests/telemetry_integration.rs`: 5 tests (episode logging, cascade router feed, usage reporting, pipeline combined telemetry, failed dispatch episode)
- `tests/helpers.rs`: Test utilities (MockResponse, TestSession, MockPhaseResponse)

---

## Zero-Coverage Modules

### config_watch.rs (167 LOC, 0 tests)

**Public API not tested:**
- `ConfigWatcher::start(config: &AcpConfig)` -- Creates `notify::RecommendedWatcher`, sets up mpsc channel, creates `ConfigCache`
- `ConfigWatcher::changed(&mut self) -> bool` -- Drains pending notifications from mpsc
- `ConfigWatcher::current(&self) -> Option<Arc<RokoConfig>>` -- Returns cached config via `ConfigCache`

**Internal functions not tested:**
- `watch_config_path()` -- Decides whether to watch file or parent dir, deduplicates watch targets
- `watched_paths()` -- Computes list of paths from config (global config, project config, `ROKO_CONFIG` env var)

**Specific gaps:**
1. `start()` with watcher creation failure (line 46-53): The `Err` branch that returns a `ConfigWatcher` with `_watcher: None` is never exercised
2. `start()` with `ConfigCache` creation failure (line 64-70): The `Err` branch logging a warning is never exercised
3. `changed()` draining behavior: No test verifies that `try_recv()` drains all pending notifications and returns `true`
4. `changed()` returns `false` when no events: Not verified
5. `current()` returns `None` when cache is absent: Not verified
6. `current()` returns `Some(config)` when cache is present: Not verified
7. `watch_config_path()` with missing file whose parent doesn't exist (line 112-114): `None` branch never hit
8. `watch_config_path()` deduplication via `BTreeSet` (line 119-122): Not verified
9. `watch_config_path()` with `watcher.watch()` failure (line 124): Not verified
10. `watched_paths()` with `ROKO_CONFIG` env var set: Not verified
11. `watched_paths()` deduplication (`.sort()` + `.dedup()`): Not verified

**Recommended tests:**
```
- config_watcher_changed_returns_false_when_no_events
- config_watcher_changed_returns_true_after_file_modification
- config_watcher_current_returns_none_when_cache_unavailable
- watched_paths_includes_explicit_global_config
- watched_paths_includes_roko_config_env_var
- watched_paths_deduplicates_overlapping_paths
- watch_config_path_deduplicates_same_target
```

---

## Under-Tested Modules

### runner.rs (2494 LOC, 5 tests) -- CRITICAL

Only 5 tests for the second-largest module. Tests cover safety layer assertions but NOT the actual pipeline execution.

**Public functions with no test coverage:**
- `run_with_workflow_engine(session, prompt, workdir, roko_config, transport)` (line 563) -- The main entry point for ACP workflow execution; drives the entire pipeline lifecycle. Zero test coverage despite being ~500 LOC of orchestration logic
- `run_workflow_pipeline(session, prompt, workdir, roko_config, transport, experiment_override)` (line 1108) -- Pipeline variant of workflow execution; also zero coverage

**Internal functions not tested:**
- `build_worktree_snapshot()` -- Only one test for dirty/untracked detection; no test for clean worktree or git failures
- `safety_layer_for_pipeline_role_with_sandbox()` -- Core safety layer construction; only test is via `cfg(test)` helper
- `log_safety_violations()` -- Never tested (logging function)
- Gate dispatch integration within pipeline phases
- Agent spawning and lifecycle management within workflow

**Untested error branches:**
1. WorkflowEngine creation failure (session config invalid)
2. Agent dispatch failure mid-pipeline
3. Gate failure with autofix retry exhaustion
4. Reviewer rejection after max iterations
5. Transport errors during streaming within pipeline
6. Cancel token propagation during active agent dispatch
7. Experiment assignment failure in pipeline mode
8. Cost budget exhaustion mid-pipeline
9. MCP tool setup failure during pipeline

**Edge cases:**
1. Empty prompt handling
2. Pipeline with zero max_iterations
3. Concurrent session prompts (session busy check is in `handle_session_prompt` but pipeline-level concurrency untested)
4. Worktree snapshot with very large git diffs

### bridge_events.rs (8430 LOC, 58 tests) -- HIGH

Despite 58 tests, many large code paths are untested:

**Public functions with no direct test:**
- `stream_events_to_editor()` has 3 tests but many internal branches untested:
  - Model stream with mixed content blocks (text + images)
  - Tool loop iteration limit reached
  - MCP tool execution failure mid-stream
  - Permission denied for tool during streaming
  - Multiple concurrent permission requests
- `handle_session_prompt()` has 2 tests (busy rejection + budget exhaustion) but:
  - No test for successful end-to-end dispatch with real provider routing
  - No test for experiment assignment during prompt handling
  - No test for knowledge card emission
  - No test for provenance chain building
  - No test for slash command routing
  - No test for context item resolution (though `resolve_context_items` has 1 test)

**Internal functions with zero test coverage:**
- `append_acp_episode()` (line 326, ~160 LOC) -- Has 2 tests but many branches untested (pipeline episode, dream consolidation trigger, efficiency event emission)
- `maybe_spawn_dream_consolidation()` (line 493) -- Never tested
- `emit_acp_efficiency_event()` (line 621) -- Never tested
- `acp_role_for_mode()` (line 653) -- Tested indirectly via `acp_routing_context_maps_modes_to_roles`
- `derive_acp_tool_capabilities()` (line 664) -- Never tested directly
- `acp_model_providers()` (line 921) -- Never tested
- `provider_near_rate_limit()` (line 934) -- Never tested
- `rate_aware_model_candidates()` (line 942) -- Never tested directly (tested via `rate_limit_provider_selection_prefers_healthy_capacity_and_honors_explicit_model`)
- `resolve_acp_dispatch_model()` (line 1071) -- Tested via `resolved_acp_dispatch_uses_the_cascade_config_key` but error branch not covered
- `compute_acp_reward()` (line 1097) -- Has 1 test for success/failure distinction; no edge cases
- `truncate_to_title()` (line 1159) -- Never tested
- `truncate_assistant_history()` (line 1179) -- Has 1 test; char boundary safety tested
- `run_anthropic_cognitive_task()` (line 2388) -- Never tested (requires mocking HTTP/provider)
- `run_anthropic_tool_loop()` (line 2474) -- Never tested
- `anthropic_model_call_config()` (line 2671) -- 2 tests but only legacy routing and explicit provider
- `model_call_request_from_acp_messages()` (line 2708) -- 1 test
- `model_call_chat_message_from_acp()` (line 2725) -- Never tested directly
- `stream_model_call_to_cognitive_events()` (line 2772) -- Never tested
- `forward_model_stream_event()` (line 2830) -- Never tested
- `run_openai_compat_cognitive_task()` (line 2932) -- Never tested
- `config_with_session_effort()` (line 3049) -- Never tested
- `openai_compat_tool_loop_supported()` (line 3058) -- Never tested
- `run_openai_compat_mcp_tool_loop()` (line 3066) -- Never tested
- `run_openai_compat_builtin_tool_loop()` (line 3251) -- Never tested
- `forward_tool_loop_stream_chunks()` (line 3405) -- Never tested
- `usage_info_from_tool_loop_usage()` (line 3427) -- Never tested
- `write_session_mcp_config()` (line 3448) -- Never tested
- `setup_session_mcp_tools()` (line 3503) -- Tested indirectly via `anthropic_session_mcp_tools`
- `sanitize_tool_segment()` (line 3662) -- Tested indirectly via `session_mcp_tool_names_are_provider_safe_and_unique`
- `unique_tool_name()` (line 3678) -- Tested indirectly
- `tool_result_from_mcp()` (line 3894) -- Never tested
- `mcp_result_text()` (line 3910) -- Never tested
- `tool_result_for_editor()` (line 3923) -- Never tested
- `build_provenance()` (line 3992) -- Has 2 tests
- `render_provenance_card()` (line 4190) -- Tested indirectly via `build_provenance_includes_all_source_types`
- `prompt_keywords()` (line 4263) -- Never tested
- `knowledge_tier_label()` (line 4281) -- Never tested
- `score_to_confidence()` (line 4290) -- Never tested
- `run_slash_command()` (line 4298, ~680 LOC) -- Has 5 streaming tests but internal branching heavily untested
- `forward_slash_command_streams()` (line 4976) -- Tested via slash command streaming tests
- `pop_progress_call()` (line 5150) -- Never tested directly
- `finish_slash_command_stream()` (line 5183) -- Never tested directly

**Untested error branches:**
1. `DispatchError::Provider` variant construction and RPC error extraction
2. `DispatchError::Transport` variant
3. `DispatchError::Safety` variant
4. Cost calculation for completely unknown models (fallback pricing)
5. Anthropic provider tool loop with tool execution failure
6. OpenAI compat provider with tool loop unsupported
7. MCP config write failure
8. MCP tool setup with server connection failure
9. Permission request with transport failure
10. Session efficiency cost recording overflow

### session.rs (2839 LOC, 34 tests) -- MODERATE

Good breadth of coverage but missing important paths:

**Untested public functions:**
- `SessionManager::replace_roko_config()` (line 1187) -- Config hot-reload
- `SessionManager::revalidate_all_sessions()` (line 1209) -- Bulk revalidation after config change
- `SessionManager::active_session_config_options()` (line 1224) -- Config options for all active sessions
- `SessionManager::persist_session()` (line 1312) -- Has 1 test via `persist_and_load_session_round_trips` but error paths not covered
- `SessionManager::list_sessions_with_persisted()` (line 1376) -- Has 1 test but filesystem edge cases untested
- `SessionManager::close_session()` (line 1391) -- Never tested directly
- `SessionManager::gc_old_sessions()` (line 1400) -- Never tested
- `AcpSession::pin_file()` (line 671) -- Never tested
- `AcpSession::unpin_file()` (line 686) -- Never tested
- `AcpSession::list_pinned()` (line 694) -- Never tested
- `AcpSession::begin_prompt()` (line 653) -- Never tested directly
- `AcpSession::finish_prompt()` (line 659) -- Never tested directly
- `AcpSession::is_busy()` (line 665) -- Tested indirectly via bridge_events busy check
- `AcpSession::revalidate_config_state()` (line 956) -- Never tested
- `resolve_bare_mode()` (line 1699) -- Never tested
- `build_slash_commands()` (line 1724) -- Has 1 test (`slash_commands_include_new_commands`)

**Untested error branches:**
1. `load_session()` with corrupted JSON file
2. `persist_session()` with filesystem write failure
3. `gc_old_sessions()` with fs::read_dir failure
4. `update_config()` with unknown option IDs (tested) but not with concurrent modifications
5. `build_system_prompt()` with very long conventions (truncation at 4096 tested, but near-boundary cases not)
6. `load_conventions()` with unreadable file
7. Session creation with duplicate session names

**Edge cases:**
1. `push_user_turn()` / `push_assistant_turn()` with empty strings
2. `build_messages_array()` with history exceeding token limits
3. `build_history_context_for_cli()` with special characters / unicode
4. Session pinned files with duplicate URIs
5. Mode change clearing history when no history exists
6. Cost budget at exact threshold (f64 precision edge)

### handler.rs (670 LOC, 1 inline test + 11 external) -- MODERATE

External conformance tests cover most request methods but internal functions lack unit tests:

**Untested internal functions:**
- `check_provider_readiness()` (line 236) -- Tested externally via conformance but no unit test for CLI-based provider short-circuit
- `handle_notification()` (line 477) -- No unit test for `session/cancel` notification or unknown notification types
- `setup_file_logging()` (line 624) -- No unit test; fallback logging path never tested
- `send_slash_commands_notification()` (line 531) -- Never tested
- `send_config_options_notification()` (line 550) -- Never tested
- `send_config_sources_notification()` (line 574) -- Never tested

**Untested request methods:**
- `session/resume` -- Never tested (neither inline nor conformance)
- `session/set_mode` -- Never tested
- `session/config/update` / `session/set_config_option` -- Never tested at handler level
- `session/close` -- Never tested at handler level

**Error branches:**
1. Config reload mid-request with config watcher detecting changes
2. Config sources change notification failure
3. Logging initialization failure with both primary and fallback paths

### types.rs (1321 LOC, 7 tests) -- MODERATE

**Untested public API:**
- `unsupported_prompt_content()` (line 249) -- Error constructor never tested
- `SessionUpdate::ready()` (line 347) -- Never tested
- `SessionUpdate::failed()` (line 358) -- Never tested
- `PermissionDecision::option_id()` (line 1050) -- Never tested
- `PermissionDecision::decision_from_option_id()` (line 1060) -- Never tested
- `PermissionOption::standard_options()` (line 1070) -- Never tested

**Untested serialization paths:**
1. `ContentBlock::Image` serialization/deserialization
2. `ContentBlock::Diff` serialization/deserialization
3. `ContentBlock::Resource` serialization/deserialization
4. `SessionUpdate` variants beyond `ready()`: `agent_message_chunk`, `agent_thought_chunk`, `tool_call_start`, `tool_call_end`, `usage_update`, etc.
5. `JsonRpcNotification` serialization round-trip
6. `ToolCallKind` enum serialization for all variants (Read, Write, Edit, Command, Other)
7. `ToolCallStatus` enum serialization for all variants
8. `StopReason` enum serialization for all variants

### acp_adapter.rs (250 LOC, 3 tests) -- MODERATE

**Untested RuntimeEvent mappings:**
- `AgentSpawned` -> `ToolCallStart`
- `AgentCompleted` -> `ToolCallComplete`
- `AgentFailed` -> `ToolCallComplete` (failed)
- `GateStarted` -> `ToolCallStart`
- `GateFailed` -> `ToolCallComplete` (failed)
- `InferenceStarted` -> `ToolCallStart`
- `InferenceCompleted` -> `ToolCallComplete`
- `InferenceFailed` -> `ToolCallComplete` (failed)
- `PhaseTransition` -> `TokenChunk`
- `WorkflowCompleted` with `Success` outcome
- `WorkflowCompleted` with `Halted` outcome

**Untested edge cases:**
1. `EventConsumer::consume()` with `try_send()` failure (channel full)
2. Multiple events with same run_id in sequence
3. Event with empty strings in fields

### workflow.rs (158 LOC, 2 tests) -- LOW-MODERATE

**Untested public methods:**
- `WorkflowRun::is_done()` -- Never tested
- `WorkflowRun::mark_complete()` -- Never tested; `completed_at` field setting unverified
- `WorkflowRun::phase()` -- Never tested
- `WorkflowRun::elapsed()` -- Never tested; `completed_at.unwrap_or_else(Utc::now)` branch untested
- `WorkflowRun::template_name()` for `Standard` and `Full` variants -- Never tested (only `Express` via `status_summary`)

**Untested structs:**
- `GateResult` -- Never constructed or serialized in tests
- `ReviewFinding` -- Never constructed or serialized in tests

### transport.rs (362 LOC, 4 tests) -- LOW-MODERATE

**Untested public methods:**
- `StdioTransport::send_response()` (line 133) -- Never tested directly
- `StdioTransport::send_error()` (line 149) -- Never tested directly
- `StdioTransport::send_request()` (line 185) -- Tested for cancellation but not for successful round-trip with `handle_incoming_response`
- `StdioTransport::handle_incoming_response()` (line 217) -- Never tested for successful response delivery

**Untested error handling:**
1. Read of oversized message (line buffer behavior)
2. Write failure during `send_notification` (io error)
3. Multiple concurrent `send_request` calls with interleaved responses

### knowledge.rs (412 LOC, 3 tests) -- LOW-MODERATE

No public functions (all items are `pub(crate)` or private), but:

**Untested paths:**
1. `query_dispatch_knowledge()` with actual KnowledgeStore present (only empty-store tested)
2. `query_dispatch_knowledge()` with actual PlaybookStore present
3. `DispatchKnowledge::card()` with mixed hits and playbooks of varying scores
4. `DispatchKnowledge::context_text()` with playbooks containing many steps
5. `append_context()` and `prepend_context()` are tested but only with simple strings

### builtin_tools.rs (1144 LOC, 10 tests) -- MODERATE

**Untested functions:**
- `execute_acp_builtin_tool()` (line 326, async) -- The main dispatch function; no unit test exercises actual tool execution (only safety policy blocking tested)
- `tool_needs_permission()` (line 383) -- Never tested
- `tool_permission_request()` (line 393) -- Never tested
- `slash_command_allowed_tools()` (line 424) -- Never tested

**Untested tool execution paths:**
1. `read_file` execution (success and failure)
2. `write_file` execution (success and failure)
3. `edit_file` execution (success and failure)
4. `list_files` / `glob` execution
5. `grep` execution
6. `web_search` execution (only safety blocking tested)

### pipeline.rs (538 LOC, 10 tests) -- GOOD

**Untested edge cases:**
1. `from_config()` with unknown string (returns `None`)
2. `auto_select()` boundary: exact word count thresholds
3. `has_strategy()` and `has_review()` for all template variants
4. Multiple consecutive gate failures with autofix retries
5. `phase_label()` for all phase variants
6. `step()` with unexpected events for the current phase (e.g., `GatesPassed` during `Strategizing`)

---

## Missing Integration Tests

### 1. End-to-end prompt flow with model dispatch
No test exercises the full path: `session/prompt` -> model selection -> provider dispatch -> streaming -> episode logging -> cascade router update. The telemetry tests use `MockResponse` which bypasses the actual dispatch machinery.

### 2. Config hot-reload during active session
No test verifies that `ConfigWatcher::changed()` triggers `replace_roko_config()` and `revalidate_all_sessions()` during an active session prompt loop.

### 3. Session persistence round-trip with model/provider state
`persist_and_load_session_round_trips` exists but does not verify that provider/model selection state survives a persist-load cycle correctly (the `load_session_resets_stale_persisted_provider_and_model` test covers the negative case but not the positive).

### 4. Pipeline execution through all phases
No integration test walks a pipeline through: Start -> Strategist -> Implementer -> Gates -> Reviewer -> Commit. The telemetry test uses `MockPhaseResponse` which mocks the pipeline phases.

### 5. Permission flow end-to-end
No test exercises the full permission flow: tool execution triggers permission request -> notification sent to editor -> editor responds via JSON-RPC response -> tool execution continues or aborts. The inline tests in bridge_events.rs test `request_permission` in isolation but not through the handler dispatch loop.

### 6. Concurrent session prompts
No test verifies behavior when two sessions are being prompted simultaneously.

### 7. Session close + GC lifecycle
No test verifies: create session -> prompt -> close -> GC (verifying persisted state is cleaned up after max age).

### 8. Slash command execution through handler
No integration test sends a slash command prompt (e.g., `/review`) through the `session/prompt` handler and verifies the streaming output.

### 9. MCP server integration in ACP sessions
No integration test verifies that MCP servers configured in `roko.toml` are discovered, connected, and their tools are made available in ACP sessions.

### 10. Event forwarding to HTTP sidecar
No test verifies that `AcpEventForwarder` successfully sends events to an `HttpEventSink` endpoint, though the mapping logic is well-tested.

### 11. Provider rate-limit awareness in model selection
`rate_limit_provider_selection_prefers_healthy_capacity_and_honors_explicit_model` tests the function in isolation but not through the full dispatch path where `ProviderHealthTracker` state is shared across sessions.

### 12. Experiment A/B assignment persistence
The `experiment_assignment_selects_applies_and_records_acp_variant` test verifies assignment logic but does not test persistence across session restarts or outcome recording through the full dispatch cycle.

---

## Priority Recommendations

**P0 -- Must have:**
1. Add tests for `config_watch.rs` (zero coverage, used in every ACP server loop iteration)
2. Add integration test for end-to-end prompt dispatch (the core user-facing path)
3. Add tests for `runner.rs` public functions (only 5 tests for 2494 LOC)

**P1 -- Should have:**
4. Add tests for `handler.rs` internal request handling (`session/resume`, `session/set_mode`, `session/close`, config update)
5. Add tests for `session.rs` untested methods (`pin_file`, `gc_old_sessions`, `close_session`, `revalidate_config_state`)
6. Add tests for `acp_adapter.rs` remaining RuntimeEvent mappings (11 variants currently untested)
7. Add tests for `workflow.rs` lifecycle methods (`mark_complete`, `is_done`, `elapsed`)

**P2 -- Nice to have:**
8. Add tests for `types.rs` serialization paths (ContentBlock variants, SessionUpdate variants)
9. Add tests for `transport.rs` send methods (response, error, request round-trip)
10. Add tests for `builtin_tools.rs` tool execution paths
11. Add tests for `bridge_events.rs` internal helpers (pricing, experiment, MCP config)
12. Add permission flow integration test
13. Add concurrent session integration test
