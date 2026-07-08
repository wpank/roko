# SOURCE-INDEX — Verified Anchors For 02-Agents Parity Refresh

Rechecked against the current tree on 2026-04-18.

Prefer `rg` if a line number drifts.

---

## Core Agent Surface

| File | Anchor | Why it matters |
|---|---|---|
| `crates/roko-agent/src/agent.rs` | `9` | `AgentResult` |
| `crates/roko-agent/src/agent.rs` | `120` | `Agent` trait |
| `crates/roko-agent/src/introspection.rs` | `12` | `AgentIdentity` |
| `crates/roko-agent/src/task_runner.rs` | `73` | local `AgentEvent` enum |
| `crates/roko-learn/src/events.rs` | `15` | learning/runtime `AgentEvent` enum |
| `crates/roko-core/src/agent.rs` | `35` | `ProviderKind` |
| `crates/roko-core/src/agent.rs` | `81` | `AgentBackend` |

---

## Provider And Agent Construction

| File | Anchor | Why it matters |
|---|---|---|
| `crates/roko-agent/src/provider/mod.rs` | `89` | `adapter_for_kind()` |
| `crates/roko-agent/src/provider/mod.rs` | `104` | `create_agent_for_model()` |
| `crates/roko-agent/src/provider/mod.rs` | `255` | `build_tool_dispatcher()` |
| `crates/roko-agent/src/provider/mod.rs` | `347` | `ProviderAdapter` trait |
| `crates/roko-agent/src/provider/openai_compat.rs` | `355` | tool-capable OpenAI-compatible path uses `ToolLoopAgent` |
| `crates/roko-agent/src/provider/anthropic_api.rs` | `52` | tool-capable Anthropic path switches to provider-specific tool loop |
| `crates/roko-agent/src/provider/anthropic_api/tool_loop.rs` | `24` | Anthropic tool-loop agent construction |
| `crates/roko-agent/src/provider/anthropic_api/tool_loop.rs` | `176` | `AnthropicMessagesBackend` |
| `crates/roko-agent/src/provider/anthropic_api/tool_loop.rs` | `324` | `impl LlmBackend for AnthropicMessagesBackend` |
| `crates/roko-agent/src/gemini/adapter.rs` | `159` | Gemini path chooses native agent vs tool loop |
| `crates/roko-agent/src/perplexity/adapter.rs` | `136` | Perplexity tool-loop construction seam |
| `crates/roko-agent/src/perplexity/adapter.rs` | `216` | `supports_tools` switch for Perplexity path |
| `crates/roko-agent/src/provider/claude_cli.rs` | `58` | Claude CLI adapter threads `mcp_config` |
| `crates/roko-agent/src/provider/cursor_acp.rs` | `28` | Cursor ACP stays on direct agent path |

---

## Tool Runtime

| File | Anchor | Why it matters |
|---|---|---|
| `crates/roko-agent/src/tool_loop/mod.rs` | `61` | `LlmBackend` |
| `crates/roko-agent/src/tool_loop/mod.rs` | `121` | `StopReason` |
| `crates/roko-agent/src/tool_loop/mod.rs` | `164` | `ToolLoop` |
| `crates/roko-agent/src/dispatcher/mod.rs` | `80` | `ToolDispatcher` |
| `crates/roko-agent/src/tool_loop/backends/mod.rs` | `76` | shared `create_tool_loop_backend()` scope |
| `crates/roko-agent/src/openai_compat_backend.rs` | `40` | `OpenAiCompatLlmBackend` |
| `crates/roko-agent/src/openai_compat_backend.rs` | `281` | `impl LlmBackend for OpenAiCompatLlmBackend` |
| `crates/roko-agent/src/tool_loop/backends/gemini_native.rs` | `27` | `GeminiNativeBackend` |
| `crates/roko-agent/src/tool_loop/backends/gemini_native.rs` | `154` | `impl LlmBackend for GeminiNativeBackend` |
| `crates/roko-agent/src/perplexity/tool_loop.rs` | `143` | `PerplexityToolLoopAgent` |
| `crates/roko-std/src/tool/builtin/mod.rs` | `41` | `TOOL_COUNT = 16` |
| `crates/roko-std/src/tool/builtin/mod.rs` | `47` | `ROKO_BUILTIN_TOOLS` |

---

## MCP, Lifecycle, And Sidecar

| File | Anchor | Why it matters |
|---|---|---|
| `crates/roko-agent/src/mcp/config.rs` | `54` | `find_mcp_config()` |
| `crates/roko-cli/src/config.rs` | `160` | CLI `AgentConfig` |
| `crates/roko-cli/src/config.rs` | `194` | `agent.mcp_config` |
| `crates/roko-cli/src/orchestrate.rs` | `2567` | `PlanRunner` |
| `crates/roko-cli/src/orchestrate.rs` | `3420` | `PlanRunner::setup_mcp` |
| `crates/roko-cli/src/orchestrate.rs` | `3548` | `PlanRunner::resolve_mcp_config_path` |
| `crates/roko-cli/src/orchestrate.rs` | `4633` | supervisor accessors |
| `crates/roko-runtime/src/process.rs` | `374` | `ProcessSupervisor` |
| `crates/roko-agent-server/src/lib.rs` | `52` | `AgentServer` |
| `crates/roko-agent-server/src/state.rs` | `447` | `AgentState` |
| `crates/roko-agent-server/src/features/messaging.rs` | `390` | dispatcher-backed messaging test |
| `crates/roko-agent-server/src/features/messaging.rs` | `480` | dispatcher-backed streaming test |
| `crates/roko-agent-server/tests/relay_registration.rs` | `158` | relay registration test |
| `crates/roko-agent-server/tests/relay_registration.rs` | `225` | wallet-backed relay registration test |
| `crates/roko-cli/tests/smoke.rs` | `204` | MCP passthrough smoke coverage |

---

## Routing And Events

| File | Anchor | Why it matters |
|---|---|---|
| `crates/roko-learn/src/active_inference.rs` | `17` | `BeliefState` |
| `crates/roko-learn/src/cascade_router.rs` | `994` | `CascadeRouter` |
| `crates/roko-learn/src/cascade_router.rs` | `1213` | `select_tier_with_active_inference()` |
| `crates/roko-learn/src/runtime_feedback.rs` | `378` | runtime loads `CascadeRouter` |
| `crates/roko-cli/src/orchestrate.rs` | `11186` | routed task dispatch uses `CascadeRouter` |
| `crates/roko-cli/src/main.rs` | `1770` | CLI route explanation path |
| `crates/roko-runtime/src/event_bus.rs` | `105` | `RokoEvent::PlanRevision` |
| `crates/roko-runtime/src/event_bus.rs` | `120` | `RokoEvent::PrdPublished` |

---

## Reminder

Two guardrails matter for this batch:

- a runtime path can be wider than the shared helper that resembles it
- “code exists” is not the same as “default orchestrator path uses it”
