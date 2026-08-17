# ACP Spec Upgrade and Refactor

**Priority**: P1 — maintainability, spec compliance, test coverage, integration gaps
**Size**: XL (2–3 weeks)
**Crate**: `crates/roko-acp/` (19,915 LOC, 15 modules)

## Prerequisite

Complete `tmp/backlog/17-acp-stability-hardening.md` (P0 panic fixes and concurrency
hardening) before starting this work. The upstream `roko-agent` compile errors must be
resolved so `cargo clippy -p roko-acp` can run clean.

## Source Analysis

This document consolidates findings from the 2026-08-15 audit:

- `tmp/archive/08-15-26/acp-todos/03-SPEC-VERSION-BUMP.md` — v0.12.2→v0.13.6 (7 releases, ~285 LOC change)
- `tmp/archive/08-15-26/acp-todos/05-BRIDGE-EVENTS-REFACTOR.md` — 8,430-line god file → 11 modules, detailed phase plan
- `tmp/archive/08-15-26/acp-todos/04-TEST-COVERAGE-GAPS.md` — per-module coverage estimates, missing integration tests
- `tmp/archive/08-15-26/acp-todos/09-INTEGRATION-GAPS.md` — 4 missing/partial integrations (~30–40h total)
- `tmp/archive/08-15-26/acp-todos/10-EDITOR-COMPATIBILITY.md` — Zed/Cursor/JetBrains test gaps

---

## Part 1: Spec Version Bump (v0.12.2 → v0.13.6)

### Current state

```rust
// crates/roko-acp/src/types.rs:9
pub const ACP_SPEC_VERSION: &str = "0.12.2";
pub const ACP_PROTOCOL_VERSION: u32 = 1;  // unchanged across all 0.12.x/0.13.x
```

The crate is 7 releases behind the stable spec (last stable: `schema-v1.20.0`, 2026-07-21).
All changes are additive. Protocol version stays at `1`. Wire format is backward-compatible.

### What is already implemented (no-ops)

- `session/close` — implemented in `handler.rs`
- `session/resume` — implemented via `session/load` fallback in `handler.rs`
- `UsageUpdate` — our existing `UsageUpdate { used, size, cost }` matches the v0.13.6 stabilized shape

### Changes required by release

**v0.13.0** — MCP-over-ACP types (`mcp/connect`, `mcp/message`, `mcp/disconnect`): unstable,
skip unless we want to act as an MCP gateway. Bump version constant only.

**v0.13.1** — `session/delete` (unstable at this point, stable in v0.13.6):

```rust
// types.rs: new struct
pub struct SessionDeleteParams {
    pub session_id: String,
}

// session.rs: new method
pub fn delete_session(&mut self, session_id: &str) -> bool { ... }

// handler.rs: new match arm
"session/delete" => {
    let params: SessionDeleteParams = ...;
    sessions.delete_session(&params.session_id);
    Ok(serde_json::json!({}))
}
```

**v0.13.3** — Stabilize `logout` + `auth` capabilities:

```rust
// types.rs additions (~15 lines)
pub struct AuthCapabilities { pub logout: Option<LogoutCapabilities> }
pub struct LogoutCapabilities {}
pub struct LogoutParams {}
// AgentCapabilities gains: pub auth: Option<AgentAuthCapabilities>

// handler.rs: populate auth capability in InitializeResult, add logout match arm
// (roko has no auth state to clear; the handler just returns {})
```

**v0.13.5** — Stabilize `additionalDirectories` (largest structural change):

```rust
// types.rs additions (~60 lines)
pub struct SessionCapabilities {
    pub close: Option<SessionCloseCapabilities>,
    pub resume: Option<SessionResumeCapabilities>,
    pub delete: Option<SessionDeleteCapabilities>,
    pub list: Option<SessionListCapabilities>,
    pub additional_directories: Option<SessionAdditionalDirectoriesCapabilities>,
}
// AgentCapabilities gains: pub session_capabilities: Option<SessionCapabilities>
// SessionNewParams gains: pub additional_directories: Option<Vec<String>>
// SessionLoadParams gains: pub additional_directories: Option<Vec<String>>
// SessionInfo gains: pub additional_directories: Option<Vec<String>>
// New: SessionResumeParams (distinct type from SessionLoadParams)
```

This is a **breaking change to our serialized `InitializeResult` shape**. The flat
`load_session: bool` field becomes structured `session_capabilities: SessionCapabilities`.
Zed and Cursor both parse via the official schema types and handle this gracefully, but
custom integrations may break. Test against current Zed stable before shipping.

**v0.13.6** — Stabilize message IDs + finalize `session/delete`:

```rust
// types.rs: add optional field to three streaming types
pub struct AgentMessageChunk { pub message_id: Option<String>, ... }
pub struct AgentThoughtChunk { pub message_id: Option<String>, ... }
// ToolCall update variant also gains message_id

// session.rs or bridge_events.rs: monotonic counter per session
// bridge_events.rs: pass message_id through streaming chunk emissions (~20 lines)
```

### Official crate alternative

`agent-client-protocol-schema` v1.6.0 (2026-07-21, Apache-2.0) is available on crates.io.
The audit recommends **keeping hand-maintained `types.rs` for now** — the 285-line manual
bump is faster (~4h) than a full crate migration (~8–12h including adapter code for
roko-specific extension types like `SessionBudgetStatus`, `McpStatusUpdate`, `BudgetStatusUpdate`,
`ConfigSources`, `ConfigWarnings`, `SessionInfoUpdate`). Revisit this when ACP v2 stabilizes
(expected late 2026); that is the natural migration point.

### All code changes for the spec bump

| File | Change | Lines |
|---|---|---|
| `types.rs` | Bump constant, new SessionCapabilities hierarchy, auth types, message_id fields | ~100 |
| `handler.rs` | New match arms (delete, logout), populate new capabilities in InitializeResult | ~50 |
| `session.rs` | `delete_session()`, store `additional_directories`, message ID counter | ~35 |
| `bridge_events.rs` | Thread `message_id` through streaming pipeline | ~25 |
| `tests/protocol_conformance.rs` | Update initialize assertions, add delete/logout/messageId tests | ~75 |

**Total: ~285 lines, ~4 hours of focused work.**

---

## Part 2: `bridge_events.rs` Decomposition

### Problem

`bridge_events.rs` is 8,430 of 19,915 total LOC in `roko-acp` (42.3%). It contains
191 definitions spanning 8 distinct concerns. The two largest functions are
`handle_session_prompt_inner` (737 lines, line 1651) and `run_slash_command` (673 lines,
line 4298). The file is a navigation and review obstacle, and every change risks
breaking unrelated code paths.

### Proposed module structure

After decomposition, `bridge_events.rs` shrinks from 8,430 to ~800 lines (the streaming,
permission, and event-mapping glue). The 10 extracted modules average ~350 lines each.

| Module | Est. LOC | Key public surface |
|---|---|---|
| `bridge_events.rs` (residual) | ~800 | `stream_events_to_editor`, `request_permission`, `handle_session_prompt` (re-export) |
| `dispatch.rs` | ~930 | `handle_session_prompt`, `handle_session_prompt_inner` |
| `model_routing.rs` | ~265 | `cascade_select_model`, `resolve_acp_dispatch_model` |
| `experiments.rs` | ~140 | `assign_acp_experiment`, `record_acp_experiment_outcome` |
| `episode_logging.rs` | ~370 | `append_acp_episode`, `calculate_cost_for_model_slug` |
| `anthropic_provider.rs` | ~570 | `run_anthropic_cognitive_task` |
| `openai_provider.rs` | ~540 | `run_openai_compat_cognitive_task` |
| `mcp_tools.rs` | ~480 | `setup_session_mcp_tools` |
| `provenance.rs` | ~370 | `build_provenance`, `render_provenance_card` |
| `slash_commands.rs` | ~1,020 | `run_slash_command` |
| `context_resolution.rs` | ~355 | `resolve_context_items`, `extract_prompt_text` |
| `tests/` (distributed) | ~2,607 | moved to companion modules |

### Types to relocate from `bridge_events.rs` before extraction

Moving these first makes each subsequent extraction independent:

| Type | Current location | Destination |
|---|---|---|
| `CognitiveEvent` | L159 | `types.rs` (used by every module) |
| `PermissionRequestPayload` | L206 | `types.rs` (pure data) |
| `PermissionReplyChannel` | L222 | `types.rs` (used by handler, builtin_tools, event_forward) |
| `StreamResult` | L275 | `types.rs` (pure data) |
| `BridgeEventsError` | L86 | `types.rs` or new `errors.rs` |
| `AcpCascadeSelection` | L969 | `model_routing.rs` (only used there) |
| `AcpExperimentAssignment` | L774 | `experiments.rs` (only used there) |
| `ProvenanceChain` / `ProvenanceSource` | L3958/L3965 | `provenance.rs` |
| `SlashCommandStreamOutcome` | L4971 | `slash_commands.rs` |
| `ModelStreamForward` / `ModelStreamForwardState` | L2762/L2768 | `anthropic_provider.rs` |
| `SessionMcpRuntime` | L3498 | `mcp_tools.rs` |

### 8-phase migration plan

Each phase ends with `cargo test -p roko-acp` to verify no regressions.
Reference: `tmp/archive/08-15-26/acp-todos/05-BRIDGE-EVENTS-REFACTOR.md` for exact line ranges.

**Phase 1** — Extract types (lowest risk, no logic changes)
Move `CognitiveEvent`, `PermissionRequestPayload`, `PermissionReplyChannel`, `StreamResult`,
`BridgeEventsError` into `types.rs`. Add `pub use` re-exports in `bridge_events.rs` so
external callers remain unaffected. Run tests.

**Phase 2** — Extract leaf modules (no cross-module deps)
1. `experiments.rs` — self-contained, no deps on other proposed modules (lines 774–910)
2. `context_resolution.rs` — only deps on `ContentBlock` from types (lines 5456–5810)
3. `provenance.rs` — only deps on types and external crates (lines 3930–4296)
Run tests after each extraction.

**Phase 3** — Extract provider modules
1. `anthropic_provider.rs` — deps: types, streaming helpers (lines 2388–2930, 5505–5531)
2. `openai_provider.rs` — deps: types, streaming, mcp_tools (lines 2932–3445, 5478–5500)
3. `mcp_tools.rs` — deps: types, CognitiveEvent (lines 3448–3928)
Run tests after each.

**Phase 4** — Extract domain modules
1. `episode_logging.rs` — deps: types, model_routing (lines 283–651)
2. `model_routing.rs` — deps: types (lines 912–1157)
3. `slash_commands.rs` — deps: types, streaming (lines 4298–5316)
Run tests after each.

**Phase 5** — Extract dispatch orchestration (most deps, extract last)
`dispatch.rs` — calls into all other modules. Contains `handle_session_prompt` and the
737-line `handle_session_prompt_inner` (lines 1627–2387).

**Phase 6** — Deduplicate logic (5 patterns identified)
1. Workdir-sandboxed path canonicalization: `read_file_context()` and `resolve_local_file_contents()` duplicate the same canonicalize-workdir-check-strip_prefix sequence. Extract `sandboxed_path(path, workdir) -> Result<(PathBuf, PathBuf)>` into `context_resolution.rs`.
2. Interleaved stdout/stderr streaming: `forward_slash_command_streams()` and `run_shell_command()` duplicate the same `BufReader` + `stdout_done`/`stderr_done` + `tokio::select!` pattern. Extract `interleave_process_output(stdout, stderr, cancel, on_line)` generic helper.
3. `UsageInfo` construction: replace `usage_info_from_model_usage()` and `usage_info_from_tool_loop_usage()` with `From<TokenUsage>` and `From<roko_core::Usage>` impls on `UsageInfo`.
4. Image content part building: merge `build_anthropic_content_parts()` and `build_openai_content_parts()` into a single parameterized function (provider kind discriminant or formatter closure).
5. Raw `event_sender.send()` calls: 50+ occurrences use raw sends with `let _ =`; consistently route through the existing `send_cognitive_event()` helper.

**Phase 7** — Break up oversized functions
- `handle_session_prompt_inner` (737 lines): split into (a) `resolve_dispatch_config()` — model, cascade, experiment resolution; (b) `build_dispatch_context()` — system prompt, knowledge, provenance, context items; (c) `execute_and_record()` — provider dispatch + episode/efficiency recording.
- `run_slash_command` (673 lines): split into `parse_slash_command()` → typed enum + per-command handlers `run_slash_plan_run()`, `run_slash_build_test_clippy()`, `run_slash_custom()`.
- Extract `parse_progress_line()` from `forward_slash_command_streams` (~L5018–5104).
- Extract `handle_request()` in `handler.rs` (206 lines) into per-method named functions.

**Phase 8** — Relocate tests
Move test functions to `#[cfg(test)] mod tests` blocks in their respective new modules.
Keep the multi-concern `acp_conformance` test (190 lines) in `bridge_events.rs` or promote
it to an integration test in `tests/`. Groupings are documented per-module in
`tmp/archive/08-15-26/acp-todos/05-BRIDGE-EVENTS-REFACTOR.md` section "Test module".

### Clippy suppressions to eliminate during refactor

8 `#[allow(clippy::too_many_arguments)]` annotations exist, all in `bridge_events.rs` and
`runner.rs`. Introducing a `DispatchParams` / `CognitiveTaskContext` struct eliminates the 6
in `bridge_events.rs` by grouping the overlapping parameter sets shared across Anthropic and
OpenAI dispatch paths.

---

## Part 3: Test Coverage

### Current coverage estimate

| Module | LOC | Tests | Est. Coverage |
|---|---|---|---|
| `bridge_events.rs` | 8,430 | 58 inline + 5 external | ~45% |
| `session.rs` | 2,839 | 34 inline + 2 external | ~55% |
| `runner.rs` | 2,494 | 5 inline | ~15% — CRITICAL |
| `types.rs` | 1,321 | 7 inline | ~40% |
| `builtin_tools.rs` | 1,144 | 10 inline | ~50% |
| `handler.rs` | 670 | 1 inline + 11 external | ~60% |
| `event_forward.rs` | 586 | 19 inline | ~85% |
| `pipeline.rs` | 538 | 10 inline | ~75% |
| `config.rs` | 521 | 8 inline + 3 external | ~70% |
| `knowledge.rs` | 412 | 3 inline | ~50% |
| `transport.rs` | 362 | 4 inline | ~45% |
| `acp_adapter.rs` | 250 | 3 inline | ~50% |
| `config_watch.rs` | 167 | 0 | **0% — zero coverage** |
| `workflow.rs` | 158 | 2 inline | ~40% |
| **Total** | **19,915** | **164 + 16** | **~40%** |

### Priority targets

**P0 — Must have (before next production Zed test)**

1. `config_watch.rs` — zero coverage; used in every ACP server loop iteration. Minimum 7 tests:
   - `config_watcher_changed_returns_false_when_no_events`
   - `config_watcher_changed_returns_true_after_file_modification`
   - `config_watcher_current_returns_none_when_cache_unavailable`
   - `watched_paths_includes_explicit_global_config`
   - `watched_paths_includes_roko_config_env_var`
   - `watched_paths_deduplicates_overlapping_paths`
   - `watch_config_path_deduplicates_same_target`

2. `runner.rs` public functions — `run_with_workflow_engine()` and `run_workflow_pipeline()` are
   the main ACP workflow entry points with zero test coverage despite ~500 LOC each. At minimum:
   - Workflow creation failure (invalid session config)
   - Gate failure with autofix retry exhaustion
   - Cost budget exhaustion mid-pipeline
   - Cancel token propagation during active agent dispatch

**P1 — Should have**

3. `handler.rs` untested request methods: `session/resume`, `session/set_mode`, `session/config/update`, `session/close`
4. `session.rs` untested methods: `pin_file`, `gc_old_sessions`, `close_session`, `revalidate_config_state`, `replace_roko_config`, `revalidate_all_sessions`
5. `acp_adapter.rs` — 11 `RuntimeEvent` variants currently produce no test coverage: `AgentSpawned`, `AgentCompleted`, `AgentFailed`, `GateStarted`, `GateFailed`, `InferenceStarted`, `InferenceCompleted`, `InferenceFailed`, `PhaseTransition`, `WorkflowCompleted(Success)`, `WorkflowCompleted(Halted)`
6. `workflow.rs` lifecycle: `is_done()`, `mark_complete()`, `elapsed()` all untested

**P2 — Nice to have**

7. `types.rs` serialization round-trips: `ContentBlock::Image`, `ContentBlock::Diff`, `ContentBlock::Resource`, all `SessionUpdate` variants, `ToolCallKind`, `ToolCallStatus`, `StopReason`
8. `transport.rs`: `send_response`, `send_error`, `send_request` round-trip with `handle_incoming_response`
9. `builtin_tools.rs`: actual tool execution paths (read_file, write_file, edit_file, list_files, grep, web_search)
10. `bridge_events.rs` internal helpers: pricing functions, experiment assignment, MCP config write

### Missing integration tests

Full list in `tmp/archive/08-15-26/acp-todos/04-TEST-COVERAGE-GAPS.md` section "Missing Integration Tests". Highest-priority gaps:

1. **End-to-end prompt flow with model dispatch**: no test exercises `session/prompt` → model selection → provider dispatch → streaming → episode logging → cascade router update. The telemetry tests use `MockResponse` which bypasses actual dispatch machinery.
2. **Config hot-reload during active session**: no test verifies `ConfigWatcher::changed()` triggers `replace_roko_config()` and `revalidate_all_sessions()` during an active prompt loop.
3. **Pipeline execution through all phases**: no integration test walks Start → Strategist → Implementer → Gates → Reviewer → Commit.
4. **Permission flow end-to-end**: no test exercises the full path from tool execution triggering a permission request through to the editor responding and execution continuing or aborting.
5. **Slash command execution through handler**: no integration test sends a slash command prompt through `session/prompt` and verifies streaming output.

---

## Part 4: Integration Gaps

The following four gaps are currently open in `tmp/archive/08-15-26/acp-todos/09-INTEGRATION-GAPS.md`.
They are P1 (no integration gap blocks self-hosting) but affect observability and routing quality.

**Gap 1: ACP as event bus producer (4h)**

ACP consumes events from the bus but publishes none. The TUI and roko-serve cannot discover
ACP activity. Fix: add `RuntimeEvent::AcpSessionCreated`, `AcpPromptStarted`,
`AcpPromptCompleted`, `AcpSessionClosed` variants to `roko-core::runtime_event.rs`,
emit from `SessionManager` and `handle_session_prompt()`.
This unblocks Gap 3 (TUI tab) and Gap 4 (roko-serve routes).

**Gap 2: force_backend override learning (2h)**

When `session.model_selection_explicit == true`, the user-chosen model is not recorded
as a positive learning signal in the cascade router. The router cannot learn from manual
preferences. Fix: in the post-dispatch path around `bridge_events.rs:2300`, check
`model_selection_explicit`; if true, call `record_cascade_observation()` with a boosted
reward (1.0). See `09-INTEGRATION-GAPS.md` section "Cascade Router Learning" for exact
line references.

**Gap 3: TUI dashboard ACP visibility (3–6h)**

Zero ACP references in `crates/roko-cli/src/tui/`. ACP sessions, costs, and metrics are
invisible in `roko dashboard`. Two approaches: (a) new TUI tab subscribing to the event
bus (requires Gap 1 first); (b) read-only display of `episodes.jsonl` and `efficiency.jsonl`
filtered by ACP trigger kind (no bus dep, 3h).

**Gap 4: roko-serve ACP session routes (6h)**

No `/api/acp/sessions`, `/api/acp/metrics`, or related endpoints exist. Add:
`GET /api/acp/sessions`, `GET /api/acp/sessions/:id`, `GET /api/acp/metrics`. Either
query episode logs or have the ACP process register with roko-serve at startup. Requires
a new `crates/roko-serve/src/routes/acp.rs` module.

Recommended order: Gap 2 (standalone, 2h) → Gap 1 (unblocks 3 + 4) → Gap 3 or 4.

---

## Part 5: Editor Compatibility

### Current test matrix

| Editor | Integration-tested | Known gaps |
|---|---|---|
| Zed | Yes (production) | Working directory issue (#46138), MCP passthrough (#52254), custom shell stdout pollution (#47991), conversation history not persisted |
| Cursor | Config documented, not integration-tested | No Cursor wire-format tests, `new_value` vs `value` field alias missing, no team-level MCP |
| JetBrains | Not tested | No `acp.json` generation, no JetBrains-specific tests |
| Neovim | Not tested | Listed in lib.rs docs but no config or tests |

### Required test additions

Full list in `tmp/archive/08-15-26/acp-todos/10-EDITOR-COMPATIBILITY.md` section "Test Gaps".
Minimum tests to add:

1. **Zed wire format regression suite**: `session/new`, `session/prompt`, and `session/config/update` payloads as actually sent by current Zed stable (one existing test: `permission_response_round_trip` in `types.rs:1235`)
2. **Cursor wire format tests**: `configId` alias round-trip, `new_value` alias (currently missing), Cursor launch sequence
3. **Multi-turn conversation test**: send multiple `session/prompt` requests to the same session; verify coherent multi-turn context
4. **Stderr isolation test**: verify non-JSON output (panics, warnings) does not leak to stdout and corrupt the JSON-RPC stream
5. **Session persistence across restart**: verify a session created in one ACP process can be loaded in another (editor restart scenario via `session/load` / `session/resume`)

### Quick wins (no editor-specific testing required)

- Add `new_value` alias to `ConfigUpdateParams` (matches `configId` / `option_id` precedent already set):
  ```rust
  #[serde(alias = "value")]
  pub new_value: serde_json::Value,
  ```
- Add `roko acp --emit-config <editor>` subcommand that prints the correct IDE config JSON for Zed, Cursor, or JetBrains. Eliminates manual setup errors.
- Document that `ROKO_LOG` env var controls log verbosity without changing the binary; surface in startup error message.

---

## Acceptance Criteria

### Spec upgrade (Part 1)
- [ ] `ACP_SPEC_VERSION` is `"0.13.6"` in `types.rs:9`
- [ ] `session/delete` handler wired; `SessionManager::delete_session()` removes from active map and persisted storage
- [ ] `logout` handler wired; `auth.logout` capability advertised in `InitializeResult`
- [ ] `AgentCapabilities.session_capabilities` populated with `close`, `resume`, `delete`, `list`, `additional_directories`
- [ ] `AgentMessageChunk`, `AgentThoughtChunk`, and `ToolCall` update variants carry optional `message_id`
- [ ] Protocol conformance tests updated for new `InitializeResult` shape; 3 new conformance tests added (delete, logout, messageId presence)
- [ ] End-to-end test against current Zed stable passes with new `InitializeResult` shape

### Bridge events refactor (Part 2)
- [ ] `bridge_events.rs` is ≤1,000 lines after extraction
- [ ] 10 new modules exist under `crates/roko-acp/src/` with public surfaces as specified above
- [ ] All 11 type relocations complete; no circular imports
- [ ] 5 duplicate logic patterns consolidated
- [ ] `handle_session_prompt_inner` split into 3 named phases, each ≤300 lines
- [ ] `run_slash_command` split into parser + per-command handlers
- [ ] All 6 `#[allow(clippy::too_many_arguments)]` in `bridge_events.rs` eliminated via `DispatchParams` struct
- [ ] `cargo clippy -p roko-acp --no-deps -- -D warnings` passes clean after refactor
- [ ] All 180 existing ACP tests pass throughout every extraction phase

### Test coverage (Part 3)
- [ ] `config_watch.rs` coverage ≥ 70% (7 new tests)
- [ ] `runner.rs` coverage ≥ 40% (minimum 4 integration tests for public entry points)
- [ ] `acp_adapter.rs` — all 11 untested `RuntimeEvent` variants covered
- [ ] `workflow.rs` — `mark_complete`, `is_done`, `elapsed`, `template_name` all tested
- [ ] End-to-end `session/prompt` integration test using `MockResponse` harness
- [ ] Config hot-reload integration test

### Integration gaps (Part 4)
- [ ] `RuntimeEvent` has 4 new ACP lifecycle variants; ACP emits them at session create/close and prompt start/end
- [ ] `record_cascade_observation()` called with boosted reward when `model_selection_explicit == true`
- [ ] `roko dashboard` shows ACP session activity (either from event bus subscription or episode log polling)
- [ ] `GET /api/acp/sessions` returns active session list; `GET /api/acp/metrics` returns aggregate cost/token data

### Editor compatibility (Part 5)
- [ ] Zed wire format regression test suite added (3+ tests)
- [ ] Cursor `new_value` field alias added; Cursor wire-format test added
- [ ] Multi-turn conversation integration test added
- [ ] Stderr isolation test added
- [ ] `roko acp --emit-config <editor>` subcommand implemented for Zed, Cursor, and JetBrains

## Not in Scope

- ACP v2 migration — the spec analysis recommends staying on v1 until v2 stabilizes (expected late 2026), then adopting the official `agent-client-protocol-schema` crate simultaneously
- Gate rungs 3–6 in the ACP pipeline (`runner.rs:2057–2145`) — deferred to a separate gate hardening effort
- Dream consolidation config knob (`dreams.episode_threshold`) — low priority, 2h standalone
- Windows/WSL compatibility — roko is macOS/Linux-focused; no Windows CI
- Marketplace and trigger system ACP slash commands (sections 9b–9c of `tmp/acp-features/00-ACP-FEATURES.md`) — future product scope
- P0 panic fixes — covered in `tmp/backlog/17-acp-stability-hardening.md` (prerequisite)
