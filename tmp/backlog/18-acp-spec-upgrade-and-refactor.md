# 18 — ACP Spec Upgrade and Refactor

**Priority**: P1 — maintainability, spec compliance, integration gaps
**Size**: XL (2–3 weeks)
**Crates**: `crates/roko-acp/`
**Depends on**: `tmp/backlog/17-acp-stability-hardening.md` (complete first)

---

## Background

Roko includes a crate called `roko-acp` that implements the Agent Client Protocol (ACP),
a JSON-RPC protocol used by code editors (Zed, Cursor, JetBrains) to interact with AI
agents. When Zed wants to send you a code suggestion from roko, it serializes the request
as ACP JSON-RPC and pipes it to the `roko acp` subprocess.

The ACP spec is maintained externally and releases new versions that add protocol features.
The roko implementation is currently at spec version `0.12.2` (constant in
`crates/roko-acp/src/types.rs:10`). The stable spec is now at version `0.13.6` (released
2026-07-21). The gap means roko does not advertise capabilities that editors expect, which
causes Zed and Cursor to disable features like session deletion, additional directories,
and message correlation.

Beyond the spec gap, the `bridge_events.rs` source file is 8,773 lines (the whole crate is
19,915 lines). It contains 191 definitions spanning 8 different concerns — model routing,
experiment assignment, episode logging, MCP tool setup, slash command execution, context
resolution, and streaming. Any change to any of these concerns means editing an 8,773-line
file and risking breakage in the other 7 concerns. This item restructures that file into
independently testable modules.

This item also covers specific integration gaps: the cascade router cannot learn from
manual model overrides, ACP sessions are invisible in the `roko dashboard` TUI, and no
HTTP API routes expose ACP session data.

**Complete item 17 (ACP Stability Hardening) before starting this work.** Item 17 fixes
panics and concurrency bugs that would interfere with the refactor.

## Current State

1. **`/Users/will/dev/nunchi/roko/roko/crates/roko-acp/src/types.rs` line 10** — `pub const ACP_SPEC_VERSION: &str = "0.12.2"`. No `SessionCapabilities`, `AuthCapabilities`, `SessionDeleteParams`, or `message_id` fields exist anywhere in `types.rs`. The `AgentCapabilities` struct (line 208) has a flat `load_session: bool` field (line 211), not the structured `session_capabilities: SessionCapabilities` that v0.13.5 requires.

2. **`/Users/will/dev/nunchi/roko/roko/crates/roko-acp/src/handler.rs`** — Handles `session/new`, `session/list`, `session/load`, `session/prompt`, `session/close`, `session/resume`, `session/cancel`, `session/set_mode`, and `session/config/update`. No `session/delete` or `logout` match arms exist (the `_` catch-all at line 468 would return "method not supported").

3. **`/Users/will/dev/nunchi/roko/roko/crates/roko-acp/src/session.rs`** — Has `gc_old_sessions` (line 1400) but no `delete_session` method. Sessions live in a `HashMap` inside `SessionManager`; there is no permanent deletion separate from `close_session`.

4. **`/Users/will/dev/nunchi/roko/roko/crates/roko-acp/src/bridge_events.rs`** — 8,773 lines. Houses `handle_session_prompt_inner` (starts around line 1651, approximately 737 lines), `run_slash_command` (starts around line 4298, approximately 673 lines), model routing, experiment assignment, episode logging, MCP tool setup, context resolution, and streaming. All test code is inline in this file too.

5. **`/Users/will/dev/nunchi/roko/roko/crates/roko-acp/src/config_watch.rs`** — 167 lines, zero test coverage. Used in the ACP server main loop to detect configuration file changes.

6. **`/Users/will/dev/nunchi/roko/roko/crates/roko-acp/src/runner.rs`** — 2,496 lines. The main ACP workflow entry points `run_with_workflow_engine()` and `run_workflow_pipeline()` have approximately 15 inline tests but the public entry points themselves are not integration-tested.

7. **`/Users/will/dev/nunchi/roko/roko/crates/roko-acp/src/acp_adapter.rs`** — 250 lines. The `publish()` method at line 164 uses `try_send` and silently drops events. Zero test coverage for `RuntimeEvent` variant mapping.

## Implementation Plan

This item has 5 independent parts. Each can be started and merged separately. The recommended
order is Part 1 (spec bump) → Part 2 (bridge_events refactor) → Part 3 (test coverage) →
Part 4 (integration gaps). Part 5 (editor compatibility) can be done in parallel with any other.

---

### Part 1: Spec Version Bump (v0.12.2 to v0.13.6) (~4 hours)

All changes are additive. The protocol wire version (1) does not change.

**Step 1.1** — Bump the version constant in `types.rs` line 10:
```rust
pub const ACP_SPEC_VERSION: &str = "0.13.6";
```

**Step 1.2** — Add new types to `types.rs` (after the existing `AgentCapabilities` struct):

```rust
// v0.13.1: session deletion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionDeleteParams {
    pub session_id: String,
}

// v0.13.3: auth/logout capabilities
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AuthCapabilities {
    pub logout: Option<LogoutCapabilities>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LogoutCapabilities {}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LogoutParams {}

// v0.13.5: structured session capabilities
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionCapabilities {
    pub close: Option<serde_json::Value>,
    pub resume: Option<serde_json::Value>,
    pub delete: Option<serde_json::Value>,
    pub list: Option<serde_json::Value>,
    pub additional_directories: Option<serde_json::Value>,
}
```

**Step 1.3** — Update `AgentCapabilities` in `types.rs` to add auth and structured session capabilities:

```rust
// Before:
pub struct AgentCapabilities {
    pub load_session: bool,
    // ...
}

// After (keep load_session for backward compat, add new fields):
pub struct AgentCapabilities {
    pub load_session: bool,   // kept for v0.12.x clients
    pub session_capabilities: Option<SessionCapabilities>,
    pub auth: Option<AuthCapabilities>,
    // ...existing fields...
}
```

**Step 1.4** — Update `InitializeResult` construction in `handler.rs` to populate the new fields:

```rust
let capabilities = AgentCapabilities {
    load_session: true,
    session_capabilities: Some(SessionCapabilities {
        close: Some(serde_json::json!({})),
        resume: Some(serde_json::json!({})),
        delete: Some(serde_json::json!({})),
        list: Some(serde_json::json!({})),
        additional_directories: Some(serde_json::json!({})),
    }),
    auth: Some(AuthCapabilities {
        logout: Some(LogoutCapabilities {}),
    }),
    // ...existing fields...
};
```

**Step 1.5** — Add `session/delete` handler to `handler.rs` before the `_` catch-all:

```rust
"session/delete" => {
    let params: SessionDeleteParams = match parse_params(params, &method) {
        Ok(params) => params,
        Err(error) => return send_error_response(transport, id, error).await,
    };
    sessions.delete_session(&params.session_id);
    send_success(transport, id, serde_json::json!({})).await
}
"logout" => {
    // roko has no persistent auth state to clear; acknowledge and return empty.
    send_success(transport, id, serde_json::json!({})).await
}
```

**Step 1.6** — Add `delete_session` to `SessionManager` in `session.rs`:

```rust
/// Permanently remove a session from active memory and persisted storage.
pub fn delete_session(&mut self, session_id: &str) {
    self.sessions.remove(session_id);
    // Also remove from disk if persisted.
    let session_path = self.sessions_dir.join(format!("{session_id}.json"));
    let _ = std::fs::remove_file(session_path);
}
```

**Step 1.7** — Add `message_id` fields to streaming types in `types.rs`. The fields are `Option<String>` so existing code does not need changes:

```rust
pub struct AgentMessageChunk {
    pub message_id: Option<String>,
    pub content: Vec<ContentBlock>,
    // ...existing fields...
}

pub struct AgentThoughtChunk {
    pub message_id: Option<String>,
    pub content: String,
    // ...existing fields...
}
```

The `message_id` values are not yet populated (they will remain `None`). A follow-up can add a per-session monotonic counter to bridge_events.

**Step 1.8** — Update the conformance test in `crates/roko-acp/tests/` (or inline in `handler.rs`) to assert that `InitializeResult` contains `session_capabilities` and that `session/delete` returns `{}`.

---

### Part 2: `bridge_events.rs` Decomposition

The goal is to reduce `bridge_events.rs` from 8,773 lines to approximately 800 lines by
extracting 10 cohesive modules. Do this incrementally — one module per commit, with
`cargo test -p roko-acp` passing after every step.

**Phase 1 — Extract shared types first** (lowest risk, pure moves)

Move these types from `bridge_events.rs` to `types.rs`. Add `pub use` re-exports in
`bridge_events.rs` so no external callers need updating:

| Type | Approx location in bridge_events.rs | Destination |
|---|---|---|
| `CognitiveEvent` | Line ~159 | `types.rs` |
| `PermissionRequestPayload` | Line ~206 | `types.rs` |
| `PermissionReplyChannel` | Line ~222 | `types.rs` |
| `StreamResult` | Line ~275 | `types.rs` |
| `BridgeEventsError` | Line ~86 | `types.rs` (or new `errors.rs`) |

Procedure: copy the type definition to `types.rs`, add `pub use crate::types::CognitiveEvent;`
to `bridge_events.rs`, run tests, delete original.

**Phase 2 — Extract leaf modules** (no dependencies on each other)

Create three new files in `crates/roko-acp/src/`:

- `experiments.rs` — extract `assign_acp_experiment`, `record_acp_experiment_outcome`, and the `AcpExperimentAssignment` struct (approx lines 774–910)
- `context_resolution.rs` — extract `resolve_context_items`, `extract_prompt_text`, `read_file_context`, `resolve_local_file_contents` (approx lines 5456–5810)
- `provenance.rs` — extract `build_provenance`, `render_provenance_card`, `ProvenanceChain`, `ProvenanceSource` (approx lines 3930–4296)

For each: create the file, move the code, add `mod experiments;` to `lib.rs`, add
`use crate::experiments::*;` imports to `bridge_events.rs`, run tests.

**Phase 3 — Extract provider modules**

- `anthropic_provider.rs` — extract `run_anthropic_cognitive_task`, `ModelStreamForward`, `ModelStreamForwardState` (approx lines 2388–2930)
- `openai_provider.rs` — extract `run_openai_compat_cognitive_task` (approx lines 2932–3445)
- `mcp_tools.rs` — extract `setup_session_mcp_tools`, `SessionMcpRuntime` (approx lines 3448–3928)

**Phase 4 — Extract domain modules**

- `episode_logging.rs` — extract `append_acp_episode`, `calculate_cost_for_model_slug`, usage helpers (approx lines 283–651)
- `model_routing.rs` — extract `cascade_select_model`, `resolve_acp_dispatch_model`, `AcpCascadeSelection` (approx lines 912–1157)
- `slash_commands.rs` — extract `run_slash_command` and the `SlashCommandStreamOutcome` type (approx lines 4298–5316)

**Phase 5 — Extract dispatch orchestration** (most dependencies, extract last)

- `dispatch.rs` — extract `handle_session_prompt`, `handle_session_prompt_inner` (the 737-line function starting at approx line 1651). This calls into all the modules above.

**Phase 6 — Consolidate duplicate logic**

After extraction, five patterns appear in two or more modules. Consolidate each:

1. Workdir-sandboxed path canonicalization — appears in `context_resolution.rs` in two functions. Extract `sandboxed_path(path, workdir) -> Result<(PathBuf, PathBuf)>`.

2. Interleaved stdout/stderr streaming — appears in `slash_commands.rs` in two functions. Extract a generic `interleave_process_output(stdout, stderr, cancel, on_line)` helper.

3. `UsageInfo` construction — replace the two `usage_info_from_*` functions with `From<TokenUsage>` and `From<roko_core::Usage>` impls.

4. Image content part building — merge `build_anthropic_content_parts` and `build_openai_content_parts` into a single function parameterized on provider format.

5. Raw `event_sender.send()` calls (50+ occurrences) — route through the existing `send_cognitive_event()` helper consistently.

**Phase 7 — Split oversized functions**

- `handle_session_prompt_inner` (737 lines): split into three named functions:
  - `resolve_dispatch_config(...)` — model selection, cascade, experiment
  - `build_dispatch_context(...)` — system prompt, knowledge, provenance, context items
  - `execute_and_record(...)` — provider dispatch + episode/efficiency recording

- `run_slash_command` (673 lines): split into `parse_slash_command()` returning a typed enum, plus per-command handlers `run_slash_plan_run()`, `run_slash_build_test_clippy()`, `run_slash_custom()`.

After Phase 7, eliminate the 6 `#[allow(clippy::too_many_arguments)]` annotations in
`bridge_events.rs` by grouping overlapping parameters into a `DispatchParams` struct.

**Phase 8 — Move tests into their modules**

Move each `#[cfg(test)]` block from `bridge_events.rs` into the module that owns the code
being tested. Keep the multi-concern `acp_conformance` test in an integration test file
under `tests/`.

---

### Part 3: Test Coverage

Add the following tests, in priority order:

**P0: `config_watch.rs` — currently 0% coverage**

Add to `crates/roko-acp/src/config_watch.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn config_watcher_changed_returns_false_when_no_events() {
        let tmp = TempDir::new().unwrap();
        let mut watcher = ConfigWatcher::new(&[tmp.path().to_path_buf()]).unwrap();
        assert!(!watcher.changed());
    }

    #[test]
    fn config_watcher_changed_returns_true_after_file_modification() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("roko.toml");
        std::fs::write(&path, "").unwrap();
        let mut watcher = ConfigWatcher::new(&[path.clone()]).unwrap();
        std::fs::write(&path, "modified = true").unwrap();
        std::thread::sleep(std::time::Duration::from_millis(100));
        assert!(watcher.changed());
    }

    // ... (5 more tests per the list in the Background section)
}
```

**P0: `runner.rs` — public entry point coverage**

Add integration tests for `run_with_workflow_engine()` and `run_workflow_pipeline()`:
- Workflow creation failure (invalid session config)
- Gate failure with autofix retry exhaustion
- Cost budget exhaustion mid-pipeline
- Cancel token propagation during active agent dispatch

**P1: `acp_adapter.rs` — RuntimeEvent variant coverage**

Add a test that sends each of the 11 untested `RuntimeEvent` variants through
`AcpWorkflowEventConsumer::publish()` and verifies the expected `CognitiveEvent` is
produced (or silently skipped for unsupported variants).

---

### Part 4: Integration Gaps

**Gap 2: force_backend override learning (2 hours, standalone)**

This is the most important gap to fix first — it does not depend on any other gap.

When a user manually selects a model via `session.model_selection_explicit == true`,
the cascade router does not record a learning signal. The router cannot learn which models
users prefer.

Find the post-dispatch path in `bridge_events.rs` near `record_cascade_observation` (around
line 2300). After the existing observation call, add:

```rust
// If user explicitly chose this model, record it as a strong positive signal.
if session.config_state.model_selection_explicit {
    record_cascade_observation(
        router_path.clone(),
        model_slug.clone(),
        routing_ctx.clone(),
        true,   // success = true
        wall_ms,
        output_tokens,
        model_slugs.clone(),
    );
    // Boosted reward: second call with a synthetic high-quality signal
    // so the router learns to prefer explicitly-chosen models.
}
```

The simpler approach: check `model_selection_explicit` before the existing observation and
use a multiplied reward (e.g., `1.0` instead of the computed value) in `compute_acp_reward`.

**Gap 1: ACP as event bus producer (4 hours)**

ACP sessions are invisible to `roko dashboard` and `roko serve` because no events are
published to the runtime event bus.

Add to `crates/roko-core/src/runtime_event.rs` (or wherever `RuntimeEvent` is defined):

```rust
pub enum RuntimeEvent {
    // ...existing variants...
    AcpSessionCreated { session_id: String, workdir: PathBuf },
    AcpPromptStarted { session_id: String, model: String },
    AcpPromptCompleted { session_id: String, cost_usd: f64, tokens: u64 },
    AcpSessionClosed { session_id: String },
}
```

Emit these from `SessionManager` in `session.rs` and from `handle_session_prompt()` in
`bridge_events.rs`. This requires the `SessionManager` to hold an `EventBus` sender.

**Gap 3: TUI dashboard ACP visibility (3 hours, requires Gap 1)**

After Gap 1 is wired, subscribe to ACP events in the TUI. The simpler approach (no Gap 1
dependency, ~3 hours): add a read-only display tab in `crates/roko-cli/src/tui/` that
polls `episodes.jsonl` and `efficiency.jsonl` filtered by ACP trigger kind.

**Gap 4: roko-serve ACP session routes (6 hours, requires Gap 1)**

Create `crates/roko-serve/src/routes/acp.rs` with three endpoints:
- `GET /api/acp/sessions` — list active sessions
- `GET /api/acp/sessions/:id` — session detail
- `GET /api/acp/metrics` — aggregate cost/token totals

---

### Part 5: Editor Compatibility (can be done in parallel)

**Quick wins (no testing required, ~1 hour each):**

1. Add `new_value` serde alias to `ConfigUpdateParams` in `types.rs`:
   ```rust
   #[serde(alias = "value")]
   pub new_value: serde_json::Value,
   ```

2. Add `roko acp --emit-config <editor>` subcommand to `crates/roko-cli/src/commands/` that
   prints the correct IDE configuration JSON for Zed, Cursor, or JetBrains.

**Test additions:**

- Zed wire format regression suite: `session/new`, `session/prompt`, `session/config/update`
  payloads as actually sent by current Zed stable
- Cursor `new_value` alias round-trip test
- Multi-turn conversation test (send 3 prompts to same session, verify coherent context)
- Stderr isolation test (verify panics/warnings do not corrupt the JSON-RPC stdout stream)

## Acceptance Criteria

### Part 1 (spec bump)
- [ ] `ACP_SPEC_VERSION` is `"0.13.6"` in `types.rs` line 10
- [ ] `session/delete` handler is wired; `SessionManager::delete_session()` removes from active map and disk
- [ ] `logout` handler wired; `auth.logout` capability advertised in `InitializeResult`
- [ ] `AgentCapabilities.session_capabilities` populated with `close`, `resume`, `delete`, `list`, `additional_directories`
- [ ] `AgentMessageChunk` and `AgentThoughtChunk` carry `pub message_id: Option<String>`
- [ ] Protocol conformance tests updated; 3 new conformance tests added (delete, logout, messageId presence)
- [ ] `cargo test -p roko-acp` passes (180+ tests)

### Part 2 (refactor)
- [ ] `bridge_events.rs` is at or under 1,000 lines after all extractions
- [ ] 10 new modules exist under `crates/roko-acp/src/` as listed above
- [ ] All 5 duplicate logic patterns consolidated
- [ ] `handle_session_prompt_inner` is split into 3 named functions, each under 300 lines
- [ ] `run_slash_command` is split into parser + per-command handlers
- [ ] All 6 `#[allow(clippy::too_many_arguments)]` in `bridge_events.rs` eliminated
- [ ] `cargo clippy -p roko-acp --no-deps -- -D warnings` passes clean after every extraction phase
- [ ] All existing 180 ACP tests pass throughout every extraction phase

### Part 3 (test coverage)
- [ ] `config_watch.rs` has at least 7 tests; estimate coverage reaches 70%
- [ ] `runner.rs` has at least 4 integration tests for public entry points
- [ ] All 11 `RuntimeEvent` variants in `acp_adapter.rs` have test coverage

### Part 4 (integration gaps)
- [ ] `record_cascade_observation()` called with boosted reward when `model_selection_explicit == true`
- [ ] `RuntimeEvent` has 4 new ACP lifecycle variants; ACP emits them
- [ ] `roko dashboard` shows ACP session activity
- [ ] `GET /api/acp/sessions` and `GET /api/acp/metrics` return valid data

### Part 5 (editor compat)
- [ ] `ConfigUpdateParams.new_value` accepts `"value"` alias
- [ ] `roko acp --emit-config zed` prints correct Zed config JSON
- [ ] Zed wire format regression suite added (3+ tests)
- [ ] Multi-turn conversation integration test added

## Verification Checklist

- [ ] `cargo test -p roko-acp` — all tests pass after every extraction phase
- [ ] `cargo clippy -p roko-acp --no-deps -- -D warnings` — clean
- [ ] `cargo test --workspace` — no regressions in other crates
- [ ] `wc -l crates/roko-acp/src/bridge_events.rs` — output is under 1000
- [ ] `grep -c '\.expect\|unreachable!' crates/roko-acp/src/bridge_events.rs` — 0 production hits
- [ ] Start `roko acp` and connect a Zed editor; verify `session/new`, `session/prompt`, and `session/delete` all succeed
- [ ] Check Zed shows roko in the AI provider list with the correct capabilities
- [ ] `roko acp --emit-config zed` prints valid JSON

## Files to Modify

| File | Change |
|---|---|
| `/Users/will/dev/nunchi/roko/roko/crates/roko-acp/src/types.rs` | Bump version constant; add `SessionCapabilities`, `AuthCapabilities`, `SessionDeleteParams`, `message_id` fields to streaming types |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-acp/src/handler.rs` | Add `session/delete` and `logout` match arms; populate new capabilities in `InitializeResult` |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-acp/src/session.rs` | Add `delete_session()` method |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-acp/src/bridge_events.rs` | Decomposed into 10 modules (Part 2); post-dispatch force_backend learning fix (Part 4 Gap 2) |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-acp/src/lib.rs` | Declare the 10 new extracted modules |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-acp/src/acp_adapter.rs` | Add log on event drop (already noted in item 17) |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-acp/src/config_watch.rs` | Add 7 unit tests |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-acp/src/runner.rs` | Add 4 integration tests for entry points |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/runtime_event.rs` | Add 4 ACP lifecycle variants (Part 4 Gap 1) |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/routes/acp.rs` | New file: 3 ACP HTTP endpoints (Part 4 Gap 4) |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/commands/mod.rs` | Wire `roko acp --emit-config` subcommand (Part 5) |
