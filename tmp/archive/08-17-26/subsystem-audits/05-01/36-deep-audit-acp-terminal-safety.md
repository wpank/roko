# 36 — Deep Audit: ACP, Terminal, Safety, orchestrate.rs

Scope: follow-up to the 35-doc audit series. Focuses on root-cause analysis for
the Zed ACP integration failure, terminal/demo brittleness, safety layer
defaults, and orchestrate.rs structural issues. No changes — catalog only.

Generated: 2026-05-01

---

## A. ACP + Zed Integration — Root Cause Chain

The screenshot symptom: user sends "hello" in Zed, sees "Prior knowledge - 3 results"
but no response text. The root cause is a 5-link failure chain where each link must
be fixed for ACP to work.

### A1. Config discovery fails (CRITICAL — primary blocker)

**What happens:** Zed invokes `roko acp` with no `--workdir` argument. The default
is `"."` which resolves relative to Zed's process cwd (typically `/` or `$HOME` on
macOS GUI apps), not the project directory.

| File | Line | Issue |
|---|---|---|
| `~/.config/zed/settings.json` | — | `"args": ["acp"]` — no `--workdir` passed |
| `crates/roko-cli/src/main.rs` | 483 | `--workdir` defaults to `"."` — resolves to Zed's cwd |
| `crates/roko-core/src/config/mod.rs` | 97-100 | `load_config` only checks `workdir/roko.toml`, no parent-dir walk |
| `crates/roko-acp/src/config.rs` | 43-50 | `load_roko_config()` ignores `self.config_path` entirely — the `--config` CLI arg is wired through to `AcpConfig.config_path` but never used |
| `crates/roko-acp/src/config.rs` | 60 | Default log path `.roko/acp.log` is relative, also resolves against Zed's cwd |

**Result:** `RokoConfig::default()` is used → providers=0, models=0.

**ACP log evidence:**
```
loaded roko.toml configuration providers=0 models=0
```

### A2. Provider kind mismatch (CRITICAL — blocks dispatch even with correct config)

**What happens:** Even when `roko.toml` IS found, the dispatch path fails because
the provider definition and the dispatch function disagree on provider kind.

| File | Line | Issue |
|---|---|---|
| `roko.toml` | 41-45 | `[providers.anthropic]` has `kind = "claude_cli"` |
| `crates/roko-acp/src/bridge_events.rs` | 1218 | `ProviderKind::ClaudeCli` routes to `run_anthropic_cognitive_task` |
| `crates/roko-acp/src/bridge_events.rs` | 1476 | `anthropic_model_call_config()` searches for `kind == AnthropicApi` only |

**Chain:** `sonnet` model → `providers.anthropic` → `kind = "claude_cli"` →
`run_anthropic_cognitive_task` → `anthropic_model_call_config()` → searches
for `AnthropicApi` kind → not found → returns `None` → "Anthropic provider
is not configured for ACP dispatch."

### A3. Error rendering invisible (HIGH — user never sees the error)

**What happens:** The dispatch failure is emitted as a `ToolCall` card, not as
`AgentMessageChunk` text. Zed renders tool calls differently (often collapsed
or in a side panel). The user sees the knowledge card but not the error.

| File | Line | Issue |
|---|---|---|
| `crates/roko-acp/src/bridge_events.rs` | 821 | `dispatch_failure_update()` creates `ToolCallStart` + `ToolCallComplete`, not `AgentMessageChunk` |

### A4. Non-session errors kill the server (HIGH — silent disconnect)

| File | Line | Issue |
|---|---|---|
| `crates/roko-acp/src/bridge_events.rs` | 161 | `rpc_error()` returns `None` for Pipeline, Serialize, Transport, TaskJoin errors |
| `crates/roko-acp/src/handler.rs` | 181-185 | Non-`SessionBusy` errors propagate as fatal loop exit → Zed sees EOF |

### A5. Process restart required after rebuild (OPERATIONAL)

Zed does not auto-restart ACP servers when the binary changes. After
`cargo build -p roko-cli`, the user must either:
- Kill the old `roko acp` process: `pkill -f "roko acp"`
- Restart Zed entirely
- Re-open the assistant panel

There is no file-watch or version-check mechanism.

### What a proper design looks like

**Config discovery chain:**
```
1. If $ROKO_CONFIG env is set → load that file directly
2. If --config flag was passed → load that file (currently ignored!)
3. If $ROKO_WORKDIR env is set → use as workdir
4. If --workdir flag was passed → use as workdir
5. Walk up from workdir looking for roko.toml (parent traversal)
6. Check ~/.roko/roko.toml as user-level fallback
7. Use RokoConfig::default() as last resort — but log a WARNING visible to the user
```

**Provider dispatch:** Remove `run_anthropic_cognitive_task` and
`run_openai_compat_cognitive_task` entirely. All dispatch should go through
`ModelCallService` which already handles provider kind internally. The ACP
layer should build a `ModelCallRequest` and call `ModelCallService::stream()`:

```rust
// Replace the match on provider_kind with:
let service = ModelCallService::new(&roko_config);
let request = model_call_request_from_acp_messages(&model_key, &messages);
let stream = service.stream(request).await?;
// Map ModelStreamEvent → CognitiveEvent → session/update notifications
```

**Error rendering:** Dispatch failures should emit `AgentMessageChunk` with
error text (visible in Zed's chat panel), not `ToolCall` cards. The
`stop_reason` should be `StopReason::Error`, not `StopReason::Refusal`.

**Error propagation:** `BridgeEventsError::rpc_error()` should map ALL error
variants to JSON-RPC error codes instead of returning `None` and crashing:
```rust
Self::Pipeline(e) => Some((-32603, format!("dispatch failed: {e}"))),
Self::Serialize(e) => Some((-32700, format!("serialization: {e}"))),
Self::Transport(e) => Some((-32600, format!("transport: {e}"))),
Self::TaskJoin(e) => Some((-32603, format!("internal: {e}"))),
```

---

## B. Terminal + Demo — Structural Issues

### B1. Terminal lifecycle broken by design

**Current design:** WebSocket connect = create PTY. WebSocket disconnect = kill PTY.
Reconnect creates a brand new shell. All state lost.

| File | Line | Issue |
|---|---|---|
| `crates/roko-serve/src/terminal.rs` | 509-510 | `ws_terminal` destroys existing session before creating new one |
| `demo/demo-app/src/hooks/useTerminal.ts` | 303-309 | `onclose` auto-reconnects after 500ms → new PTY, all state lost |
| `crates/roko-serve/src/terminal.rs` | 515 | Hardcoded 80x24 at creation; resize arrives asynchronously |

**Correct design:** Decouple PTY lifecycle from WebSocket lifecycle. Sessions
persist across disconnects. WebSocket attaches/detaches from existing sessions.
Session has explicit `create` (POST) and `destroy` (DELETE) APIs.

### B2. Prompt scraping is the automation truth source (MEDIUM)

| File | Line | Issue |
|---|---|---|
| `demo/demo-app/src/hooks/useTerminal.ts` | 12 | `PROMPT_RE` matches `>`, `%`, `#`, `$` — false positives on output containing these chars |
| `demo/demo-app/src/hooks/useTerminal.ts` | 194-212 | `waitForPrompt` polls `outBuf` every 30ms with 120ms debounce — fragile timing |
| `demo/demo-app/src/lib/scenario-runners/prd-pipeline.ts` | 192 | `showCmd` uses `waitForPrompt` not markers |

**Correct design:** All automated commands should use `execCmd` (marker-based)
or a typed server-side command execution protocol that reports exit codes.
`waitForPrompt` should only exist for interactive/display use, never for
automation correctness.

### B3. Security issues in terminal routes

| Severity | File | Line | Issue |
|---|---|---|---|
| CRITICAL | `terminal.rs` | 237-256 | Arbitrary command execution via `CreateSessionRequest.command` — no allowlist |
| HIGH | `terminal.rs` | 231-233 | Workdir path traversal — `workdir` field used verbatim as `cmd.cwd()` |
| HIGH | `terminal.rs` | 485-501 | `POST /api/terminal/sessions/{id}/input` — write to any session, no ownership |
| HIGH | `terminal.rs` | 509-510 | WebSocket to any session ID destroys existing session |
| MEDIUM | `terminal.rs` | 276-279 | ZDOTDIR temp dirs never cleaned up (`PtySession` doesn't store the path) |
| MEDIUM | `terminal.rs` | 566 | Fragile JSON detect: `starts_with("{\"type\":\"resize\"")` — 2 hardcoded variants |
| MEDIUM | `terminal.rs` | 235 | Session ID is 8 hex chars — birthday collision after ~65K sessions |
| MEDIUM | `terminal.rs` | 9, 313 | Generation counter resets on restart — reconnect can kill new session |
| LOW | `terminal.rs` | 595-611 | No CSRF on WebSocket, no rate limiting on session creation |

### B4. Demo app specific issues

| File | Line | Issue |
|---|---|---|
| `prd-pipeline.ts` | 111-127 | `writeFileViaPty` pipes base64 through PTY — fragile, visible in output, can hit arg length limits |
| `useTerminal.ts` | 271-277 | Resize sent in `onopen` callback — races against shell startup output |
| `serve-url.ts` | 9 | Hardcoded port 6677 |

**Correct design for demo:** Replace file operations through PTY with REST
endpoints (`PUT /api/workspaces/{id}/files/{path}`). Replace prompt scraping
with server-side command execution protocol that emits typed exit events.

---

## C. Safety Layer — Fail-Open Defaults

### C1. Every permissive() site (verified current state)

| File | Line | What | Risk |
|---|---|---|---|
| `roko-agent/src/safety/mod.rs` | 256 | `SafetyLayer::with_defaults()` → `AgentContract::permissive("default")` | CRITICAL — every SafetyLayer starts permissive |
| `roko-agent/src/safety/mod.rs` | 875 | `contract_for_role()` fallback for configured roles with missing YAML → `permissive(role)` | CRITICAL — roles in config but without contracts get full access |
| `roko-agent/src/claude_cli_agent.rs` | 128 | `ClaudeCliAgent::new()` hardcodes `dangerously_skip_permissions: true` | CRITICAL — all Claude CLI agents bypass permission prompts |
| `roko-cli/src/runner/types.rs` | 1394 | `RunnerConfig` default hardcodes `dangerously_skip_permissions: true` | CRITICAL — runner default is to skip permissions |
| `roko-agent/src/dispatcher/hook_chain.rs` | 190, 229, 250 | `HallucinationDetector::permissive()` in test code | LOW — confirmed test-only |

### C2. The inversion problem

The fundamental issue: the safety layer is opt-in, not opt-out.

- `SafetyLayer::with_defaults()` → permissive (must call `with_role()` to restrict)
- `contract_for_role()` → permissive fallback for one code path, restricted for another
- `ClaudeCliAgent::new()` → permissions bypassed (must call builder to restrict)

**Correct design:** Invert all defaults:

```rust
// SafetyLayer::new() requires a contract — no permissive default
SafetyLayer::new(contract: AgentContract)

// AgentContract::permissive() only in #[cfg(test)]
#[cfg(test)]
pub fn permissive(role: &str) -> Self { ... }

// ClaudeCliAgent::new() defaults to false
pub fn new(...) -> Self {
    Self { dangerously_skip_permissions: false, ... }
}

// contract_for_role() uses RestrictedFallback for ALL paths
fn contract_for_role(&self, role: &str) -> AgentContract {
    AgentContract::load_for_role_with_mode(role, ContractLoadMode::RestrictedFallback)
}
```

### C3. MaxCostPerTurn is per-call, not cumulative

| File | Line | Issue |
|---|---|---|
| `roko-agent/src/safety/contract.rs` | 481-483 | TODO comment: per-turn cost enforcement needs cumulative tracking |

An agent can exceed budget by issuing many small calls. The safety layer tracks
individual call cost but not cumulative turn cost.

---

## D. orchestrate.rs — God File Analysis

### D1. Verified function sizes

| Function | Lines | Start | Issues |
|---|---|---|---|
| `build_context_assembler_sections` | ~4,107 | 18530 | Longest function in the file |
| `dispatch_agent_with` | ~3,976 | 14554 | 8 parameters, 15+ distinct operations |
| `dispatch_action` | ~3,566 | 8205 | Match on action type with massive arms |
| `attempt_replan` | ~2,783 | 11771 | Plan failure handling and retry |

**Total file:** ~22,637 lines.

### D2. Hardcoded model names (22+ sites)

| Model | Locations |
|---|---|
| `"claude-opus-4-6"` | 5366, 9929, 11809, 12927, 14657 |
| `"claude-sonnet-4-6"` | 10333, 12920, 13724, 14634, 14641, 15119 |
| `"claude-haiku-4-5"` | 13723, 14211, 15062 |

`roko.toml` has `fast_task_model`, `standard_task_model`, `complex_task_model`
config keys (lines 820-822) but the orchestrator ignores them in most places.

### D3. Duplicate spawn blocks (4 sites)

Lines 1584, 1645, 1695, 1745 — four near-identical `spawn_agent_with_layer`
blocks differing only in `SpawnAgentSpec` field values.

### D4. Silent error drops (29 `let _ =` sites)

Critical ones:
- `self.daimon.appraise(...)` at 7672, 7924, 8411, 9261, 9310, 9335, 10881, 11224, 13326
- `self.conductor.decide(...)` at 14793

If the affect engine or conductor fails, roko silently stops modulating dispatch.

### D5. 60 unwrap() calls

Every `.unwrap()` is a potential panic in the plan runner process.

### D6. Parameter explosion

`dispatch_agent_with` takes 8 parameters (3 are `Option` overrides).
Should be a `DispatchRequest` builder struct.

### What decomposition should look like

```
crates/roko-cli/src/orchestrate/
  mod.rs              — PlanRunner struct, run loop, public API
  dispatch.rs         — dispatch_agent_with → 4 focused functions:
                         resolve_dispatch_model()
                         build_dispatch_context()
                         execute_dispatch()
                         record_dispatch_outcome()
  replan.rs           — attempt_replan
  context.rs          — build_context_assembler_sections
  actions.rs          — dispatch_action
  models.rs           — model_for_complexity(band) reads config
  spawn.rs            — unified spawn_agent helper (replaces 4 duplicates)
  feedback.rs         — learning pipeline writes (replaces let _ = patterns)
```

---

## E. Cross-Cutting Issues (New Findings)

### E1. Two CORS builders that don't agree

| File | Line | What |
|---|---|---|
| `roko-serve/src/routes/middleware.rs` | 436-462 | `cors_layer()` — used by main router |
| `roko-serve/src/lib.rs` | 724-736 | `build_cors_layer()` — uses `CorsLayer::permissive()` when origins empty |

Both use `allow_methods(Any)` and `allow_headers(Any)`. They are independently
configured and independently fail-open.

### E2. Auth disabled by default in serve

`roko.toml:954-956`: `serve.auth.enabled = false`, `api_key = ""`. Combined
with permissive CORS and `0.0.0.0` bind on PORT detection, a deployed instance
is a fully open HTTP API with terminal access.

### E3. Agent sidecar binds to 0.0.0.0

`roko-agent-server/src/lib.rs:349`: default bind is `"0.0.0.0:0"`.

### E4. `roko.toml` provider naming confusion

`[providers.anthropic]` has `kind = "claude_cli"`. The provider name "anthropic"
suggests the Anthropic API, but the kind says Claude CLI subprocess. This naming
mismatch is what causes A2 (the dispatch path that searches for `AnthropicApi`
kind under the "anthropic" provider name and fails).

---

## F. Issue Priority Matrix

### Tier 0 — Blocking ACP from working at all
1. A1: Config discovery — no parent-walk, `--config` flag ignored
2. A2: Provider kind mismatch — `claude_cli` ≠ `anthropic_api`
3. A3: Error rendering — failures invisible as tool cards
4. A4: Non-session errors kill server — Zed sees EOF

### Tier 1 — Security (exploitable on any network-accessible instance)
5. B3-1: Arbitrary command execution via terminal `command` field
6. B3-2: Workdir path traversal in terminal
7. B3-3: No session ownership — write to any PTY
8. C1: Safety layer defaults to permissive
9. E2: Auth disabled + permissive CORS + public bind = open API

### Tier 2 — Structural (blocks reliable operation)
10. D1-D6: orchestrate.rs decomposition
11. B1: Terminal lifecycle tied to WebSocket (state lost on reconnect)
12. B2: Prompt scraping as automation truth
13. C2: Safety opt-in instead of opt-out
14. E1: Two competing CORS builders
15. E4: Provider naming confusion in roko.toml

### Tier 3 — Correctness (wrong behavior that's not immediately visible)
16. C3: MaxCostPerTurn is per-call not cumulative
17. B4: Demo file writes via PTY
18. D4: 29 silent error drops in orchestrate.rs
19. D2: 22+ hardcoded model names ignoring config
