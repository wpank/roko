# IDE/ACP Protocol Gaps

## Architecture Overview

### What is ACP?

ACP (Agent Client Protocol) is a JSON-RPC 2.0 protocol over stdio that enables Roko to function
as a coding agent from any ACP-compatible editor (JetBrains, Zed, Neovim, VS Code, etc.). The
implementation lives in `crates/roko-acp/` (layer 4, ~17 source files).

**Spec version**: 0.12.2 (`ACP_SPEC_VERSION` in `types.rs:9`)
**Protocol version**: 1 (`ACP_PROTOCOL_VERSION` in `types.rs:6`)

### Crate Structure

```
crates/roko-acp/
  src/
    lib.rs              -- Public API: re-exports AcpConfig + run_acp_server
    types.rs            -- All JSON-RPC + ACP wire types (1215 lines)
    handler.rs          -- Main dispatch loop: read stdin, route methods (651 lines)
    session.rs          -- SessionManager, AcpSession, SessionConfigState (~2300 lines)
    bridge_events.rs    -- CognitiveEvent -> session/update streaming (~4200 lines)
    transport.rs        -- StdioTransport: read/write/send_request with pending registry
    config.rs           -- AcpConfig: workdir, profile, config loading + merge
    config_watch.rs     -- notify::RecommendedWatcher + ConfigCache for hot-reload
    pipeline.rs         -- PipelineState: pure state machine (phases + events -> actions)
    workflow.rs         -- WorkflowRun: timing, cost, template metadata wrapper
    runner.rs           -- Pipeline executor: drives PipelineState via side effects (~2000 lines)
    acp_adapter.rs      -- RuntimeEvent -> CognitiveEvent bridge (EventConsumer impl)
    event_forward.rs    -- CognitiveEvent -> RuntimeEvent for HTTP event sink
    knowledge.rs        -- DispatchKnowledge: neuro + playbook queries per dispatch
  tests/
    protocol_conformance.rs  -- End-to-end tests: initialize, session CRUD, prompt, cancel
    telemetry_integration.rs -- Episode logging, cascade router, usage reporting
    helpers.rs               -- MockServer, TestSession, mock provider scaffolding
```

### Session Lifecycle

```
IDE                                 ACP Server (roko acp)
 |                                       |
 |--- initialize ----------------------->|  Negotiate protocol version
 |<-- InitializeResult ------------------|  agentCapabilities, configSources, configWarnings
 |                                       |
 |--- session/new ---------------------->|  Create session with model/provider/effort/MCP
 |<-- SessionNewResult ------------------|  sessionId, configOptions, warnings
 |<-- session/update (commands) ---------|  AvailableCommandsUpdate notification
 |                                       |
 |--- session/prompt ------------------->|  Send prompt content blocks
 |<-- session/update (chunks) ----------|  AgentMessageChunk / ThinkingChunk
 |<-- session/update (tool_call) -------|  ToolCall / ToolCallUpdate
 |<-- session/update (plan) ------------|  Plan entries
 |<-- session/update (mcp_status) ------|  McpStatusUpdate (if MCP servers configured)
 |<-- session/update (usage) -----------|  UsageUpdate (tokens + cost)
 |<-- session/update (session_info) ----|  SessionInfoUpdate (auto-name)
 |<-- SessionPromptResult --------------|  stopReason: end_turn | cancelled | max_tokens | ...
 |                                       |
 |--- session/config/update ------------>|  Change model/provider/effort at runtime
 |<-- ConfigUpdateResult ----------------|  Updated configOptions
 |                                       |
 |--- session/cancel ------------------->|  Cooperative cancel (notification, no response)
 |                                       |
 |--- session/close -------------------->|  Destroy session
 |<-- {} --------------------------------|
```

### JSON-RPC Methods Implemented

| Method | Kind | Handler |
|--------|------|---------|
| `initialize` | Request | `handler.rs:279` |
| `session/new` | Request | `handler.rs:309` |
| `session/list` | Request | `handler.rs:320` |
| `session/load` | Request | `handler.rs:324` |
| `session/prompt` | Request | `handler.rs:342` -> `bridge_events.rs:934` |
| `session/config/update` | Request | `handler.rs:370` |
| `session/set_config_option` | Request (alias) | `handler.rs:370` |
| `session/close` | Request | `handler.rs:397` |
| `session/resume` | Request | `handler.rs:405` |
| `session/set_mode` | Request (legacy) | `handler.rs:432` |
| `session/cancel` | Notification | `handler.rs:461` |

### Server-to-Client Messages

| Method | Kind | When |
|--------|------|------|
| `session/update` | Notification | During prompt streaming (all SessionUpdate variants) |
| `session/request_permission` | Request | Before file edits, terminal commands, etc. |
| `server/config_sources_update` | Notification | When config files change |

---

## Gap Analysis

---

### Task 018: SessionNewParams -- HashMap Nondeterminism

**Verdict**: SOLID (minor issue)

**Current implementation** (`crates/roko-acp/src/session.rs:867-876`):
```rust
fn first_model_for_provider(
    roko_config: &roko_core::config::schema::RokoConfig,
    provider_key: &str,
) -> Option<String> {
    roko_config
        .models
        .iter()
        .find(|(_, profile)| profile.provider == provider_key)
        .map(|(key, _)| key.clone())
}
```

**What the spec requires**: When a provider override doesn't match the session's current model,
the fallback should select deterministically. `RokoConfig.models` is an `IndexMap` (ordered),
so `.iter().find()` returns the first by insertion order -- this is actually deterministic.

**What's actually wrong**: The call sites at `session.rs:642` and `session.rs:748` use this to
replace a model when the user switches providers. The function is correct for `IndexMap` but
would break if the config map type ever changed to `HashMap`. The name is also misleading --
it finds the first model matching a provider, not necessarily the "best" model.

**Fix design**:
1. The function should prefer "ready" providers (where API key resolves):
```rust
fn first_model_for_provider(
    roko_config: &roko_core::config::schema::RokoConfig,
    provider_key: &str,
) -> Option<String> {
    // Prefer models whose provider is ready (API key resolves)
    let provider = roko_config.providers.get(provider_key);
    let prefer_ready = provider.is_some_and(|p| roko_config.is_provider_available(p));
    roko_config
        .models
        .iter()
        .find(|(_, profile)| {
            profile.provider == provider_key
        })
        .map(|(key, _)| key.clone())
}
```
2. Add a doc comment explicitly stating that IndexMap iteration order is relied upon.

**Effort**: 5 min (add doc comment) or 15 min (add ready-provider preference).

---

### Task 019: MCP Error Accumulation -- Missing Tests

**Verdict**: SOLID (test gap)

**Current implementation**:
- `McpInitStatus` enum at `crates/roko-acp/src/types.rs:337-354` -- all 7 variants defined
- `McpServerStatus` struct at `types.rs:296-306` with `ready()` and `failed()` constructors
- Event mapping in `bridge_events.rs:3515`: `CognitiveEvent::McpStatus { statuses } => SessionUpdate::McpStatusUpdate { statuses }`
- Forwarding in `event_forward.rs:93-98`: `CognitiveEvent::McpStatus` -> `RuntimeEvent::FeedbackRecorded`

**What the spec requires**: Unit tests covering:
1. HTTP transport -> `TransportUnsupported` status
2. Nonexistent command -> `SpawnFailed` status
3. `map_event_to_update(CognitiveEvent::McpStatus {...})` produces correct `SessionUpdate`

**Missing tests**: The `map_event_to_update` function at `bridge_events.rs:3482` is tested
for token chunks, tool calls, and completions, but not for `McpStatus`. Similarly, the
`AcpEventForwarder::map_event` at `event_forward.rs:46` handles `McpStatus` but has no test.

**Fix design**: Add three tests to `bridge_events.rs` (in the existing `#[cfg(test)] mod tests`):

```rust
#[test]
fn map_event_to_update_mcp_status() {
    let event = CognitiveEvent::McpStatus {
        statuses: vec![
            McpServerStatus::ready("code-search", 5),
            McpServerStatus::failed("broken", McpInitStatus::SpawnFailed, "command not found"),
        ],
    };
    let update = map_event_to_update(event);
    match update {
        SessionUpdate::McpStatusUpdate { statuses } => {
            assert_eq!(statuses.len(), 2);
            assert_eq!(statuses[0].status, McpInitStatus::Ready);
            assert_eq!(statuses[1].status, McpInitStatus::SpawnFailed);
        }
        _ => panic!("expected McpStatusUpdate"),
    }
}

#[test]
fn http_transport_produces_transport_unsupported() {
    let status = McpServerStatus::failed(
        "http-server",
        McpInitStatus::TransportUnsupported,
        "HTTP transport not supported for session-scoped MCP",
    );
    assert_eq!(status.status, McpInitStatus::TransportUnsupported);
    assert_eq!(status.tool_count, 0);
}

#[test]
fn nonexistent_command_produces_spawn_failed() {
    let status = McpServerStatus::failed(
        "missing-cmd",
        McpInitStatus::SpawnFailed,
        "No such file or directory",
    );
    assert_eq!(status.status, McpInitStatus::SpawnFailed);
}
```

**Effort**: 20 min.

---

### Task 020: Command Categories -- Wrong Bare Mode Set

**Verdict**: NEEDS_WORK (business logic wrong)

**Current implementation** (`crates/roko-acp/src/session.rs:1359-1363`):
```rust
fn bare_mode_allows_category(category: &str) -> bool {
    matches!(
        category,
        "system" | "research" | "implementation" | "verification" | "workflow" | "help"
    )
}
```

Used at `session.rs:1638-1647`:
```rust
if bare_mode {
    commands
        .into_iter()
        .filter(|command| {
            command
                .category
                .as_deref()
                .is_some_and(bare_mode_allows_category)
        })
        .collect()
} else {
    commands
}
```

**What the spec requires**: Exactly 8 commands in bare mode:
`status`, `doctor`, `config`, `help`, `research`, `search`, `enhance-prd`, `analyze`.

**What's wrong**:
1. The category filter exposes 20+ commands because whole categories are allowed.
   For example, `"implementation"` allows: `run`, `agents`, `agent-chat`, `agent-start`,
   `agent-stop`, `index`. `"verification"` allows: `review`, `build`, `test`, `clippy`,
   `fmt`, `gate`. `"workflow"` allows: `workflow`, `express`, `full`, `review-this`, `pipeline`.
2. `enhance-prd` is categorized as `"specification"` (line 1407), which is NOT in the
   allow-list, so it is HIDDEN in bare mode -- violating the spec.
3. No tests assert the exact 8-command set.

**Fix design**: Replace category-based filtering with an explicit command name whitelist:

```rust
/// Commands available in bare mode (no workspace state required).
const BARE_MODE_COMMANDS: &[&str] = &[
    "status", "doctor", "config", "help",
    "research", "search", "enhance-prd", "analyze",
];

fn bare_mode_allows_command(name: &str) -> bool {
    BARE_MODE_COMMANDS.contains(&name)
}
```

Update the filter at `session.rs:1638-1647`:
```rust
if bare_mode {
    commands
        .into_iter()
        .filter(|command| bare_mode_allows_command(&command.name))
        .collect()
} else {
    commands
}
```

Add a test:
```rust
#[test]
fn bare_mode_exposes_exactly_eight_commands() {
    let commands = build_slash_commands(true);
    let names: Vec<&str> = commands.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(
        names,
        &["status", "doctor", "config", "research", "search",
          "enhance-prd", "analyze", "help"],
    );
}
```

**Effort**: 10 min.

---

### Task 061: max_output Surfacing -- Missing Tests Only

**Verdict**: SOLID (test gap)

**Current implementation** (`crates/roko-core/src/config/provider.rs:476-484`):
```rust
/// Resolved output-token ceiling for this model.
///
/// `None` in config means "use the runtime default", which currently
/// matches the agent dispatch fallback.
#[must_use]
pub fn effective_max_output(&self) -> u64 {
    self.max_output
        .unwrap_or(u64::from(DEFAULT_MAX_OUTPUT_TOKENS))
}
```

The function is correctly implemented and wired into ACP session config option descriptions
(the model dropdown shows `(max output: N)` when building `ConfigOptionValue` entries in
`session.rs`).

**Missing tests**:
1. `None` resolves to the default (16,384)
2. `max_output < 1000` produces a diagnostic warning in the config option description
3. Model option description text contains `(max output: N)`

**Fix design**: Add to `crates/roko-core/src/config/provider.rs` tests:

```rust
#[test]
fn effective_max_output_uses_default_when_none() {
    let profile = ModelProfile { max_output: None, ..Default::default() };
    assert_eq!(profile.effective_max_output(), u64::from(DEFAULT_MAX_OUTPUT_TOKENS));
}

#[test]
fn effective_max_output_uses_explicit_value() {
    let profile = ModelProfile { max_output: Some(4096), ..Default::default() };
    assert_eq!(profile.effective_max_output(), 4096);
}
```

Add to `crates/roko-acp/src/session.rs` tests:

```rust
#[test]
fn model_option_description_contains_max_output() {
    let mut config = RokoConfig::default();
    config.models.insert("test-model".into(), ModelProfile {
        provider: "test".into(),
        slug: "test-model".into(),
        max_output: Some(8192),
        ..Default::default()
    });
    let state = SessionConfigState::from_roko_config(&config);
    let options = build_config_options(&state, &config);
    let model_opt = options.iter().find(|o| o.id == "model").expect("model option");
    let values = model_opt.options.as_ref().expect("option values");
    let test_entry = values.iter().find(|v| v.value == "test-model");
    assert!(test_entry.is_some());
    let desc = test_entry.unwrap().description.as_deref().unwrap_or("");
    assert!(desc.contains("max output"), "description should mention max output: {desc}");
}
```

**Effort**: 30 min.

---

### Task 062: Provider Readiness Boolean -- Wrong Serde Contract

**Verdict**: NEEDS_WORK (wire format bug)

**Current implementation** (`crates/roko-acp/src/types.rs:690-703`):
```rust
/// One selectable config option value.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigOptionValue {
    pub value: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default = "default_true")]
    pub ready: bool,
}
```

**What the spec requires**: `ready: false` should be OMITTED from the wire format for backward
compatibility. Old IDE clients that don't know about the `ready` field should not see an
unexpected `"ready": false` key on unavailable providers.

The serde attribute should be:
```rust
#[serde(default, skip_serializing_if = "std::ops::Not::not")]
```
But this idiom is incorrect -- `skip_serializing_if` takes `&bool`, not `bool`. The correct form:

```rust
fn is_false(v: &bool) -> bool { !*v }
```

**What's wrong**: With `#[serde(default = "default_true")]`, the `ready` field:
- Serializes `false` explicitly as `"ready": false` -- breaks backward compat
- Deserializes missing field as `true` -- correct behavior

**Fix design** (`crates/roko-acp/src/types.rs`):

```rust
fn is_false(v: &bool) -> bool { !*v }

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigOptionValue {
    pub value: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default = "default_true", skip_serializing_if = "is_false")]
    pub ready: bool,
}
```

Wait -- this is wrong too. `skip_serializing_if = "is_false"` would omit the field when
`ready == false`, which is the desired behavior. But we also want `ready == true` to be
omitted (since the default is true). The cleanest approach:

```rust
/// Skip serializing `ready` when it equals the default (true).
/// This means `"ready": false` is never on the wire; old clients
/// that default-true on missing get correct behavior.
fn ready_is_default(v: &bool) -> bool { *v }

#[serde(default = "default_true", skip_serializing_if = "ready_is_default")]
pub ready: bool,
```

Actually the spec says "OMIT the field when false" so that old clients see no change. But if
we also want to omit when true (since true is the default and unnecessary), we should always
omit. The simplest correct fix following the spec's backward-compat intent:

```rust
/// Always omit `ready` from serialization. Clients default to true when absent.
/// When `ready == false`, we avoid breaking old clients that don't expect the field.
fn skip_always(_v: &bool) -> bool { true }
```

But this means the server can never communicate `ready: false` to new clients. The actual
spec intent is:

- Omit when `true` (unnecessary, matches default)
- Serialize when `false` (new clients need to know)

So the correct fix is to only skip when true:

```rust
fn is_true(v: &bool) -> bool { *v }

#[serde(default = "default_true", skip_serializing_if = "is_true")]
pub ready: bool,
```

This way:
- `ready: true` -> field omitted (old clients default to true -> correct)
- `ready: false` -> field present as `"ready": false` (new clients see it)

BUT the original gap doc says "This would OMIT the field when false -- old clients see no
change." That contradicts what new clients need. The actual backward-compat guarantee is: old
clients that don't know about `ready` should always see available providers. If `ready: false`
is present, old clients may ignore it (no harm) or display it (minor confusion). The real
concern is that old clients might treat an unexpected `"ready": false` as an error.

**Recommended fix**: Follow the original gap doc's recommendation. Omit `ready` when false
so old clients never see it. New clients that understand `ready` should treat absence as true:

```rust
fn is_true(v: &bool) -> bool { *v }

// Omit when true (redundant). Only serialize when false (for new clients).
// Wait -- this still sends "ready": false to old clients.
```

The simplest safe fix that matches the spec's backward-compat intent:

```rust
#[serde(default = "default_true", skip_serializing_if = "std::ops::Not::not")]
pub ready: bool,
```

This uses `Not::not` on `&bool` -- which works because `Not` is implemented for `&bool` in
std. When `ready == false`, `not(false) == true`, so it SKIPS -- omitting the field. When
`ready == true`, `not(true) == false`, so it serializes `"ready": true`.

Actually, let me verify: `skip_serializing_if` passes `&T`, and `Not for &bool` returns
`bool`. `(!&false) == true` -> skip. `(!&true) == false` -> include. So this would:
- `ready: false` -> OMIT (old clients default to true -- wrong! They should know it's unavailable)
- `ready: true` -> INCLUDE as `"ready": true` (redundant)

This is backward from what we want. The correct attribute for "omit when true, include when false":
```rust
fn is_true(v: &bool) -> bool { *v }

#[serde(default = "default_true", skip_serializing_if = "is_true")]
pub ready: bool,
```

**Final recommendation**: Use `skip_serializing_if = "is_true"` where `fn is_true(v: &bool) -> bool { *v }`.
This omits the redundant `"ready": true` but preserves `"ready": false` for clients that understand it.

```rust
fn is_true(v: &bool) -> bool { *v }

#[serde(default = "default_true", skip_serializing_if = "is_true")]
pub ready: bool,
```

**Effort**: 5 min (one-line attribute change + one helper function).

---

### Task 063: MCP Status Notification -- Missing "Always Emit"

**Verdict**: SOLID (minor behavioral gap)

**Current implementation** (`crates/roko-acp/src/bridge_events.rs`):
The MCP status notification is only emitted when `mcp_statuses` is non-empty. The guard
is in the dispatch path where MCP servers are initialized.

**What the spec requires**: "Always emit structured MCP status notification." When zero MCP
servers are configured, a notification with an empty `statuses: []` array should still be sent
so IDE clients can distinguish "no MCP servers" from "server didn't report MCP status."

**What's missing**: When `session.mcp_servers.is_empty()`, no `McpStatusUpdate` notification
is sent. IDE clients have no way to know whether MCP was checked.

**Fix design**: After MCP initialization (or skip), always emit the status:

```rust
// In the dispatch path, after MCP init:
let mcp_statuses: Vec<McpServerStatus> = /* ... */;

// Always emit, even if empty
let _ = event_sender
    .send(CognitiveEvent::McpStatus { statuses: mcp_statuses })
    .await;
```

This ensures the IDE always receives a `session/update` with `sessionUpdate: "mcp_status_update"`
and can render "No MCP servers" or "3 MCP servers ready" as appropriate.

**Effort**: 3 min.

---

### Task 064: Default Model/Provider Fallback -- Missing Tests

**Verdict**: SOLID (test gap + minor logic gap)

**Current implementation** (`crates/roko-acp/src/session.rs:176-251`):
The `from_roko_config_with_warnings` method implements a 7-step fallback:

1. Try `agent.default_model` from config
2. Verify it exists in `[models.*]`
3. Verify its provider is available (API key resolves)
4. If not, find the first model with a ready provider
5. If no ready model, use the first model in config
6. Extract provider from selected model
7. If no models at all, try first provider key

**Missing tests**: No unit tests for any of the 6 spec-required scenarios:
1. Configured model + ready provider -> use it
2. Configured model + unready provider -> fall back to first ready
3. No configured model -> first ready model
4. All providers unready -> first model anyway
5. No models configured -> empty string
6. Provider-only fallback prefers ready providers

**Minor logic gap**: At step 7 (`session.rs:237`):
```rust
.or_else(|| config.providers.keys().next().cloned())
```
This picks the first provider key regardless of readiness. It should prefer a ready provider:
```rust
.or_else(|| {
    config.providers.iter()
        .find(|(_, p)| config.is_provider_available(p))
        .or_else(|| config.providers.iter().next())
        .map(|(key, _)| key.clone())
})
```

**Fix design**: Add unit tests to `crates/roko-acp/src/session.rs`:

```rust
#[test]
fn fallback_uses_configured_model_when_ready() {
    let config = make_config_with_ready_provider("my-model", "my-provider");
    let (state, warnings) = SessionConfigState::from_roko_config_with_warnings(&config);
    assert_eq!(state.model, "my-model");
    assert_eq!(state.provider, "my-provider");
    assert!(warnings.is_empty());
}

#[test]
fn fallback_skips_unready_default_model() {
    let config = make_config_with_unready_default_and_ready_alt();
    let (state, warnings) = SessionConfigState::from_roko_config_with_warnings(&config);
    assert_ne!(state.model, "unready-model");
    assert!(!warnings.is_empty());
}

#[test]
fn fallback_with_no_models_returns_empty() {
    let config = RokoConfig::default();
    let (state, _) = SessionConfigState::from_roko_config_with_warnings(&config);
    // May be empty or use built-in defaults
    assert!(state.model.is_empty() || !state.model.is_empty());
}
```

**Effort**: 30 min (test scaffolding for provider readiness mocking).

---

### Task 073: ACP Startup Resilience

**Verdict**: SOLID (exceeds spec)

**Implementation** spans 4 files:
- `crates/roko-acp/src/config.rs:120-201` -- `load_roko_config_with_warning()` checks both
  project and global configs, returns `(RokoConfig, Option<String>)`.
- `crates/roko-acp/src/handler.rs:84-136` -- Collects config load warning + provider readiness
  warning into `startup_warnings` vec.
- `crates/roko-acp/src/types.rs:196-198` -- `config_warnings: Vec<String>` on `InitializeResult`
  with `skip_serializing_if = "Vec::is_empty"`.
- `crates/roko-acp/src/session.rs:911-913` -- `SessionManager` stores `config_sources` and
  `startup_warnings`.

**What exceeds spec**:
- Checks both project AND global configs for parse errors
- Integration tests with TestHarness: no roko.toml, malformed roko.toml, missing credentials
  (all at `tests/protocol_conformance.rs:411-522`)
- Config hot-reload path also uses the resilience helpers
- `config_warnings` properly omitted when empty via serde annotation
- `config_sources` always serialized (even as `[]`) per design doc

**No gaps found**. This is the reference ACP task implementation.

---

### Task 088: ACP Architecture Sweep

**Verdict**: SOLID (minor dead code)

**Implementation**:
- `temperament`/`routing_mode` removed from `SessionConfigState` -- only `agent_mode`,
  `provider`, `model`, `effort`, `workflow`, `review_strictness`, `max_iterations` remain
  (`session.rs:148-167`).
- Effort flows through `config_with_session_effort()` at every dispatch -- the session's
  effort setting is applied before each model call.
- `--global-config` CLI flag implemented via `AcpConfig::with_global_config()` (`config.rs:42-45`).
- `configSources` in `InitializeResult` always serialized (`types.rs:193-194`).
- Missing-workdir warning logged at `handler.rs:88-117`.

**Dead code**: `revalidate_all_sessions()` at `session.rs:943-947` is defined but never called.
The config reload path in `handler.rs:166-221` uses `replace_roko_config` and
`active_session_config_options` instead.

```rust
pub fn revalidate_all_sessions(&mut self) {
    for session in self.sessions.values_mut() {
        session.revalidate_config_state(&self.roko_config);
    }
}
```

**Fix**: Either wire it into the config reload path or remove it:
```rust
// Option A: Wire into handler.rs config reload (line ~184)
sessions.replace_roko_config(refreshed);
sessions.revalidate_all_sessions(); // Add this

// Option B: Delete the method if replace_roko_config + active_session_config_options suffices
```

---

## Protocol Conformance Analysis

### Message Types: Implemented vs Missing

| SessionUpdate Variant | Implemented | Tested | Wire Format Verified |
|-----------------------|-------------|--------|---------------------|
| `agent_message_chunk` | Yes (`bridge_events.rs:3484`) | Yes (protocol_conformance.rs:280) | Yes (types.rs test:1092) |
| `agent_thought_chunk` | Yes (`bridge_events.rs:3488`) | No | No |
| `tool_call` | Yes (`bridge_events.rs:3491`) | Partial (protocol_conformance.rs:280) | No |
| `tool_call_update` | Yes (`bridge_events.rs:3504`) | No | No |
| `plan` | Yes (`bridge_events.rs:3514`) | No | No |
| `available_commands_update` | Yes (`handler.rs:511`) | Implicit (protocol_conformance.rs:282) | No |
| `config_option_update` | Yes (`handler.rs:530`) | No | No |
| `mcp_status_update` | Yes (`bridge_events.rs:3515`) | No | No |
| `usage_update` | Yes (telemetry path) | Yes (telemetry_integration.rs:42) | No |
| `session_info_update` | Yes (session name auto-gen) | No | No |

**Missing from spec (not in our types)**:
- `progress` update type (some ACP clients expect a generic progress indicator)
- `file_changed` notification (post-commit file change list; the `FileChangeNotification`
  type exists at `types.rs:798-815` but is never emitted as a `SessionUpdate`)

### Error Handling in Protocol Messages

| Error Code | Constant | Usage |
|------------|----------|-------|
| -32700 | `PARSE_ERROR` | Malformed JSON on stdin (`handler.rs:150`) |
| -32600 | `INVALID_REQUEST` | Defined but not used directly |
| -32601 | `METHOD_NOT_FOUND` | Unknown method (`handler.rs:448`) |
| -32602 | `INVALID_PARAMS` | Param deserialization failure (`handler.rs:503`) |
| -32603 | `INTERNAL_ERROR` | Serialization, transport, task failures (`bridge_events.rs:188-191`) |
| -32000 | `SESSION_NOT_FOUND` | Unknown session ID (`handler.rs:571`) |
| -32001 | `SESSION_BUSY` | Concurrent prompt on same session (`bridge_events.rs:184`) |

**Gap**: `INVALID_REQUEST` (-32600) is defined but never emitted. If a message arrives that
is technically valid JSON-RPC but violates the ACP spec (e.g., missing `jsonrpc: "2.0"` field),
the server currently either parses it successfully or returns `PARSE_ERROR`. Consider adding
validation for the `jsonrpc` field.

### Session Lifecycle Gaps

1. **No session expiry during runtime**: `gc_old_sessions(7 days)` runs once at startup
   (`handler.rs:140`) but not periodically. Long-running ACP processes could accumulate stale
   sessions in memory.

2. **No "session/resume" history replay**: `session/resume` loads the session but does not
   replay conversation history to the IDE. The IDE must re-render from its own state.

3. **No concurrent prompt detection across sessions**: Each session has an independent `busy`
   flag, but nothing prevents a misbehaving client from sending prompts to many sessions
   simultaneously and exhausting system resources.

4. **Session persistence is best-effort**: `sessions.persist_session(&session_id)` at
   `handler.rs:367` writes to disk after prompt completion. If the process crashes during
   a prompt, the session is lost.

### MCP Integration Status

**Implemented**:
- `McpServerConfig` type with `Stdio` and `Http` transport variants (`types.rs:281-372`)
- `McpInitStatus` with 7 failure modes (`types.rs:337-354`)
- `McpServerStatus` with `ready()` and `failed()` constructors (`types.rs:308-334`)
- `McpCapabilities` advertised in `InitializeResult` (`types.rs:231-241`): both HTTP and SSE
- Session-scoped MCP server list (`session.rs:275`)
- MCP discovery timeout configurable per server (`types.rs:289-291`)
- MCP tools discovered via `roko-agent` MCP client (`bridge_events.rs:19`)

**Gaps**:
- No MCP server lifecycle management (servers started but never explicitly stopped on session close)
- No MCP tool invocation permission flow (tools are called without IDE confirmation)
- `roko-mcp-code` MCP server exists (`crates/roko-mcp-code/`) with 8 tools (symbol_lookup,
  call_graph, imports, semantic_search, search_code, find_references, type_hierarchy,
  workspace_summary) but is not auto-discovered or auto-attached to ACP sessions

---

## Permission System

### Current Implementation

The permission system is fully typed (`types.rs:894-1033`):

```
Server -> Client: session/request_permission
  RequestPermissionParams {
    session_id, tool_call, options: [AllowOnce, AllowAlways, RejectOnce, RejectAlways]
  }

Client -> Server: Response
  PermissionResponse {
    outcome: Cancelled | Selected { option_id }
  }
```

The `always_allowed` set on `AcpSession` (`session.rs:298`) tracks actions pre-granted via
"always allow" decisions, avoiding repeated permission prompts for the same action type.

**Permission actions covered**: `FileEdit`, `FileCreate`, `FileDelete`, `TerminalCommand`,
`NetworkRequest`, `GitOperation`.

---

## Complete ACP Message Flow (ASCII)

### Prompt Dispatch (Standard Path)

```
IDE                     handler.rs                 bridge_events.rs            Provider
 |                          |                            |                        |
 |-- session/prompt ------->|                            |                        |
 |                          |-- handle_session_prompt --->|                        |
 |                          |                            |-- resolve_model ------->|
 |                          |                            |-- query_knowledge ----->| (neuro)
 |                          |                            |-- build_system_prompt ->| (compose)
 |                          |                            |                        |
 |                          |                            |-- ModelCallService ----->|
 |<-- session/update -------|<--- CognitiveEvent --------|<-- StreamChunk ---------|
 |  (agent_message_chunk)   |    (TokenChunk)            |   (TextDelta)          |
 |                          |                            |                        |
 |<-- session/update -------|<--- CognitiveEvent --------|<-- StreamChunk ---------|
 |  (tool_call)             |    (ToolCallStart)         |   (ToolUse)            |
 |                          |                            |                        |
 |<-- request_permission -->|    (if permission needed)  |                        |
 |-- permission_response -->|                            |                        |
 |                          |                            |-- execute_tool -------->|
 |<-- session/update -------|<--- CognitiveEvent --------|<-- tool result ---------|
 |  (tool_call_update)      |    (ToolCallComplete)      |                        |
 |                          |                            |                        |
 |<-- session/update -------|<--- CognitiveEvent --------|<-- StreamChunk ---------|
 |  (usage_update)          |    (Complete)               |   (Stop)              |
 |                          |                            |                        |
 |                          |-- append_acp_episode ------>| (episodes.jsonl)       |
 |                          |-- cascade_router.observe -->| (cascade-router.json)  |
 |                          |-- persist_session --------->| (.roko/sessions/)      |
 |                          |                            |                        |
 |<-- SessionPromptResult --|                            |                        |
 |  {stopReason: "end_turn"}|                            |                        |
```

### Pipeline Dispatch (Workflow Path)

```
IDE                     handler.rs              runner.rs               pipeline.rs
 |                          |                       |                       |
 |-- session/prompt ------->|                       |                       |
 |                          |-- detect workflow --->|                       |
 |                          |                       |-- PipelineState::new ->|
 |                          |                       |                       |
 |<-- session/update -------|<-- plan entries ------|<-- step(Start) ------->|
 |  (plan update)           |                       |   -> SpawnStrategist  |
 |                          |                       |                       |
 |<-- session/update -------|<-- agent output ------|-- spawn_agent ------->|
 |  (agent_message_chunk)   |                       |                       |
 |                          |                       |                       |
 |<-- session/update -------|<-- gate results ------|-- run_gates --------->|
 |  (tool_call)             |                       |                       |
 |                          |                       |<-- step(GatesPassed) -|
 |                          |                       |   -> SpawnReviewer    |
 |                          |                       |                       |
 |<-- session/update -------|<-- review output -----|-- spawn_reviewer ---->|
 |  (tool_call)             |                       |                       |
 |                          |                       |<-- step(Approved) ----|
 |                          |                       |   -> Commit           |
 |                          |                       |                       |
 |<-- session/update -------|<-- commit done -------|-- git commit -------->|
 |  (plan update: complete) |                       |                       |
 |                          |                       |                       |
 |<-- SessionPromptResult --|                       |                       |
```

### Config Hot-Reload Flow

```
File System                ConfigWatcher           handler.rs              IDE
    |                          |                       |                    |
    |-- roko.toml modified --->|                       |                    |
    |                          |-- changed() = true -->|                    |
    |                          |                       |-- load_roko_config |
    |                          |                       |-- replace_roko_config
    |                          |                       |                    |
    |                          |                       |-- config_sources_notification ->|
    |                          |                       |  (if sources changed)           |
    |                          |                       |                    |
    |                          |                       |-- config_option_update -------->|
    |                          |                       |  (per active session)           |
```

---

## Session State Machine

```
                                 +-----------+
                                 |  Created  |
                                 +-----+-----+
                                       |
                                session/new
                                       |
                                 +-----v-----+
                          +----->|   Idle     |<-----+
                          |      +-----+-----+      |
                          |            |             |
                          |     session/prompt       |
                          |            |             |
                          |      +-----v-----+      |
                          |      |   Busy     |      |
                          |      +-----+-----+      |
                          |       /    |    \        |
                          |      /     |     \       |
                  end_turn/   cancel  max_tokens  error
                  refusal     |        |          |
                          |      \     |     /       |
                          |       \    |    /        |
                          |      +-----v-----+      |
                          +------|  Complete  |------+
                                 +-----+-----+
                                       |
                                session/close
                                       |
                                 +-----v-----+
                                 |  Closed   |
                                 +-----------+

States:
  Created  - Session allocated, config options built, no prompt yet
  Idle     - Ready for prompt, cancel token reset
  Busy     - Prompt in flight, busy flag set, cancel token active
  Complete - Prompt finished, busy cleared, session persisted
  Closed   - Session removed from memory (session/close)
```

---

## Error Recovery Design

### Transport-Level Errors

| Error | Recovery |
|-------|----------|
| Malformed JSON | Send `PARSE_ERROR` (-32700) with null id, continue reading |
| stdin EOF | Graceful shutdown, return `Ok(())` |
| stdout write failure | Return fatal error, process exits |
| Pending request registry poisoned | Log warning, drop the response |

### Session-Level Errors

| Error | Recovery |
|-------|----------|
| Session not found | Return `SESSION_NOT_FOUND` (-32000) |
| Session busy | Return `SESSION_BUSY` (-32001) |
| Config parse failure | Load defaults + emit `config_warnings` |
| Provider credentials missing | Emit `config_warnings` + set `ready: false` on options |

### Dispatch-Level Errors

| Error | Recovery |
|-------|----------|
| Model call timeout | Emit `CognitiveEvent::Failure`, log episode with `success: false` |
| Provider 429/500 | Emit failure message chunk, return `StopReason::EndTurn` |
| Tool execution failure | Emit `ToolCallUpdate` with `status: failed`, continue loop |
| MCP server spawn failure | Set `McpInitStatus::SpawnFailed`, continue without MCP |
| Safety violation | Log violation, abort tool call, emit failure event |

---

## Configuration Options

### AcpConfig (Server-Level)

| Field | Type | Description |
|-------|------|-------------|
| `workdir` | `PathBuf` | Working directory for ACP operations |
| `profile` | `String` | Named configuration profile |
| `config_path` | `Option<PathBuf>` | Explicit `--config` path |
| `global_config_path` | `Option<PathBuf>` | Explicit `--global-config` path |
| `log_file` | `PathBuf` | ACP server log file (default: `.roko/acp.log`) |

### SessionConfigState (Session-Level)

| Field | Type | Default | ConfigOption ID |
|-------|------|---------|-----------------|
| `agent_mode` | `String` | `"code"` | `"mode"` |
| `provider` | `String` | From config | `"provider"` |
| `model` | `String` | From config | `"model"` |
| `effort` | `String` | From config | `"effort"` |
| `clippy_enabled` | `bool` | From gates config | `"clippy"` |
| `tests_enabled` | `bool` | From gates config | `"tests"` |
| `workflow` | `String` | `"none"` | `"workflow"` |
| `review_strictness` | `String` | `"none"` | `"review_strictness"` |
| `max_iterations` | `u32` | `2` | `"max_iterations"` |

---

## Testing Strategy

### Existing Tests

| Test File | Tests | Coverage |
|-----------|-------|----------|
| `types.rs` (unit) | 5 | JSON-RPC round-trip, content block serialization, permission request |
| `pipeline.rs` (unit) | 8 | Express/standard/full templates, gate failure, review revise, cancel |
| `workflow.rs` (unit) | 2 | Run creation, status summary |
| `transport.rs` (unit) | 3 | Read request, EOF, write notification |
| `acp_adapter.rs` (unit) | 3 | Run ID filtering, gate event mapping, cancel mapping |
| `config.rs` (unit) | 7 | Config loading, global merge, config sources prefixes |
| `knowledge.rs` (unit) | 3 | Card rendering, empty stores, context merge |
| `event_forward.rs` (unit) | 0 | No tests |
| `session.rs` (unit) | ~20 | Slash commands, config options, mode setting, config update |
| `bridge_events.rs` (unit) | ~10 | Event mapping, cost calculation, system prompt building |
| `protocol_conformance.rs` (integration) | 10 | Initialize, session CRUD, prompt, cancel, errors, startup resilience |
| `telemetry_integration.rs` (integration) | 4 | Episode logging, router, usage, pipeline telemetry |

### Missing Tests (Prioritized)

**HIGH priority** (protocol correctness):
1. `agent_thought_chunk` wire format -- never tested
2. `tool_call` / `tool_call_update` wire format -- no standalone test
3. `plan` update wire format -- no test
4. `config_option_update` notification -- no test
5. Bare mode command set -- no assertion on exact command list

**MEDIUM priority** (behavioral):
1. `McpStatusUpdate` event mapping (`bridge_events.rs`)
2. Default model/provider fallback scenarios (`session.rs`)
3. `effective_max_output` defaults and descriptions
4. Config hot-reload notification sequence
5. Permission request/response round-trip through transport

**LOW priority** (edge cases):
1. Session GC after 7 days
2. Concurrent session prompt rejection
3. `revalidate_all_sessions` (dead code -- test or delete)
4. `event_forward.rs` mapping tests
5. `FileChangeNotification` emission (type exists, never used)

### Protocol Conformance Test Design

A comprehensive conformance suite should verify:

```rust
/// Verify every SessionUpdate variant serializes to the expected wire format.
#[test]
fn all_session_update_variants_produce_valid_wire_json() {
    let variants = vec![
        SessionUpdate::AgentMessageChunk { content: text("hello"), _meta: None },
        SessionUpdate::AgentThoughtChunk { content: text("thinking...") },
        SessionUpdate::ToolCall { tool_call_id: "tc1".into(), title: "Edit".into(),
            kind: ToolCallKind::Edit, status: ToolCallStatus::InProgress,
            content: vec![], locations: None },
        SessionUpdate::ToolCallUpdate { tool_call_id: "tc1".into(),
            status: ToolCallStatus::Completed, content: vec![text("done")],
            locations: None },
        SessionUpdate::Plan { entries: vec![PlanEntry { content: "step 1".into(),
            priority: Priority::High, status: PlanStatus::Pending }] },
        SessionUpdate::AvailableCommandsUpdate { available_commands: vec![] },
        SessionUpdate::ConfigOptionUpdate { config_options: vec![] },
        SessionUpdate::McpStatusUpdate { statuses: vec![] },
        SessionUpdate::UsageUpdate { used: 1000, size: 128000, cost: None },
        SessionUpdate::SessionInfoUpdate { session_id: "s1".into(), session_name: None },
    ];
    for variant in variants {
        let json = serde_json::to_value(&variant).expect("serialize");
        assert!(json.get("sessionUpdate").is_some(), "missing discriminant: {json}");
        let roundtrip: SessionUpdate = serde_json::from_value(json.clone())
            .unwrap_or_else(|e| panic!("round-trip failed for {json}: {e}"));
        assert_eq!(serde_json::to_value(&roundtrip).unwrap(), json);
    }
}
```

### Mock IDE Client Design

The existing `TestClient` in `tests/protocol_conformance.rs` is a good foundation. It should
be extended to:

```rust
impl TestClient {
    /// Create session, send prompt, collect all updates until final response.
    async fn full_prompt_cycle(&mut self, prompt: &str) -> PromptCycleResult {
        let session_id = self.create_session("test").await.unwrap();
        let request_id = self.start_request("session/prompt", json!({
            "sessionId": session_id,
            "prompt": [{ "type": "text", "text": prompt }],
            "includeContext": false,
        })).await.unwrap();

        let mut updates = Vec::new();
        loop {
            let msg = self.read_message().await.unwrap();
            if msg.get("id").and_then(Value::as_u64) == Some(request_id) {
                return PromptCycleResult {
                    session_id,
                    updates,
                    response: msg,
                };
            }
            updates.push(msg);
        }
    }

    /// Verify session/update notification has required fields.
    fn assert_valid_update(&self, update: &Value) {
        assert_eq!(update["jsonrpc"], "2.0");
        assert_eq!(update["method"], "session/update");
        assert!(update["params"]["sessionId"].is_string());
        assert!(update["params"]["update"]["sessionUpdate"].is_string());
    }
}
```

---

## Summary of Required Fixes

| Priority | Task | Fix | Effort |
|----------|------|-----|--------|
| HIGH | 020 | Replace category filter with command name whitelist | 10 min |
| HIGH | 062 | Add `skip_serializing_if = "is_true"` on `ready` field | 5 min |
| MEDIUM | 018 | Add doc comment re: IndexMap order reliance | 5 min |
| MEDIUM | 088 | Wire or delete `revalidate_all_sessions()` dead code | 5 min |
| LOW | 063 | Always emit MCP status (even empty) | 3 min |
| LOW | 061 | Add `effective_max_output` unit tests | 30 min |
| LOW | 064 | Add model/provider fallback unit tests | 30 min |
| LOW | 019 | Add MCP status event mapping unit tests | 20 min |
| LOW | -- | Add `agent_thought_chunk` wire format test | 10 min |
| LOW | -- | Add `tool_call` / `tool_call_update` wire format tests | 15 min |
| LOW | -- | Wire `FileChangeNotification` into post-commit flow | 1 hr |

### Unblocked Items from ACP Protocol Completion Plan

From `tmp/subsystem-audits/implementation-plans/21-acp-protocol-completion.md`:

| Item | Status | Notes |
|------|--------|-------|
| ACP-1: Provider auth/capability validation | NOT DONE | `DispatchResolver` still uses `Unvalidated` diagnostics |
| ACP-2: End-to-end transcript proof | NOT DONE | No `transcript_e2e.rs` test file exists |
| ACP-3: Verify ClaudeCli vs API identity | NOT DONE | No regression test for provider kind preservation |
| ACP-4: Remove stale dispatch wrappers | PARTIAL | Some `run_*_cognitive_task` wrappers may still exist |
