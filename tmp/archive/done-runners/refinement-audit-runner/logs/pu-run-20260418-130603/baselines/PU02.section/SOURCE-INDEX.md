# SOURCE-INDEX — Verified Code Anchors For 02-Agents Parity

Code anchors used by batch `02`. These were re-checked against the current codebase on 2026-04-16.

Prefer `rg` over trusting any exact line number if the file has changed since then.

---

## Important Corrections

The previous source index had several stale or wrong anchors. Use these corrections:

- `AgentResult` is at `crates/roko-agent/src/agent.rs:9`, not near the end of the file.
- `Agent` trait is at `crates/roko-agent/src/agent.rs:120`.
- `ProviderKind` lives in `crates/roko-core/src/agent.rs:35`.
- `Usage` lives in `crates/roko-agent/src/usage.rs:11`.
- `LlmBackend` lives in `crates/roko-agent/src/tool_loop/mod.rs:61`.
- there is no `crates/roko-agent/src/process.rs`; supervision code lives in `crates/roko-runtime/src/process.rs`.
- `find_mcp_config` is the current MCP discovery helper in `crates/roko-agent/src/mcp/config.rs:54`.

---

## `crates/roko-agent/src/`

### Core Types

| File | What | Section |
|------|------|---------|
| `agent.rs:9` | `AgentResult` struct | A.02 |
| `agent.rs:120` | `Agent` trait | A.01 |
| `usage.rs:11` | `Usage` struct | A.03 |
| `chat_types.rs:92` | canonical richer `ChatResponse` | A.05 |
| `chat_types.rs:112` | canonical richer `ResponseMetadata` | A.07 |
| `chat_types.rs:123` | `FinishReason` enum | A.06 |
| `translate/mod.rs:59` | duplicate `ChatResponse` copy | A.05 |
| `translate/mod.rs:69` | duplicate `ResponseMetadata` copy | A.07 |
| `translate/mod.rs:98` | `Translator` trait | C.18-C.29 |
| `translate/mod.rs:131` | `RenderedTools` enum | E.18 |

### Provider System

| File | What | Section |
|------|------|---------|
| `provider/mod.rs:87` | `adapter_for_kind()` | B.08 |
| `provider/mod.rs:102` | `create_agent_for_model()` | B.09, D.14 |
| `provider/mod.rs:222` | `build_tool_dispatcher()` | C.17, E.18 |
| `provider/mod.rs:314` | `ProviderAdapter` trait | B.07 |
| `provider/mod.rs:333` | `AgentOptions` | B.10, E.11 |
| `provider/mod.rs:370` | `ProviderError` | B.11 |
| `provider/mod.rs:403` | `RetryAction` | B.11 |
| `provider/mod.rs:441` | `AgentCreationError` | B.12 |
| `provider/openai_compat.rs` | OpenAI-compatible adapter + OpenRouter routing injection | B.17-B.19 |
| `provider/claude_cli.rs` | Claude CLI adapter | B.08 |
| `provider/anthropic_api.rs` | Anthropic HTTP adapter | B.08, C.13 |
| `provider/cursor_acp.rs` | Cursor ACP adapter | B.08 |
| `perplexity/adapter.rs` | Perplexity adapter | B.15 |
| `gemini/adapter.rs` | Gemini adapter | B.16 |

### Tool Loop + Dispatcher

| File | What | Section |
|------|------|---------|
| `tool_loop/mod.rs:61` | `LlmBackend` trait | C.08 |
| `tool_loop/mod.rs:116` | `StopReason` enum | C.02 |
| `tool_loop/mod.rs:159` | `ToolLoop` struct | C.01 |
| `tool_loop/mod.rs:267` | `run_streaming()` | C.40 |
| `tool_loop/mod.rs:289` | `resume()` | C.07 |
| `tool_loop/mod.rs:308` | `run_inner()` | C.38 |
| `tool_loop/agent_wrapper.rs:18` | `ToolLoopAgent` | C.17, C.39 |
| `tool_loop/agent_wrapper.rs:90` | `impl Agent for ToolLoopAgent` | C.17, C.39 |
| `tool_loop/backends/mod.rs:80` | `create_tool_loop_backend()` | C.13 |
| `tool_loop/checkpoint.rs:19` | `Checkpoint` | C.07 |
| `dispatcher/mod.rs:80` | `ToolDispatcher` | C.14, E.18 |
| `safety/mod.rs:77` | `SafetyLayer` | C.16 |
| `safety/capabilities.rs:27` | `AgentWarrant` | A.16, F.10 |

### MCP + Lifecycle

| File | What | Section |
|------|------|---------|
| `mcp/client.rs:19` | `McpRequest` | D.07 |
| `mcp/client.rs:32` | `McpResponse` | D.07 |
| `mcp/client.rs:129` | `StdioTransport` | D.07 |
| `mcp/client.rs:213` | `McpClient` | D.07 |
| `mcp/client.rs:274` | `list_tools()` | D.07 |
| `mcp/config.rs:27` | `McpConfig` | D.08 |
| `mcp/config.rs:54` | `find_mcp_config()` | D.08, D.13 |
| `mcp/to_tool_def.rs:22` | `mcp_to_tool_def()` | D.09 |
| `mcp/dynamic_registry.rs:18` | `DynamicToolRegistry` | D.11 |
| `mcp/handler.rs:22` | `McpHandlerResolver` | D.12 |
| `mcp/handler.rs:61` | `McpToolHandler` | D.12 |
| `pool.rs:75` | `InstanceStatus` | D.03 |
| `pool.rs:148` | `AgentPool` | D.01-D.03 |
| `multi_pool.rs:48` | `MultiAgentPool` | D.04-D.06 |

### Advanced Agent Primitives

| File | What | Section |
|------|------|---------|
| `introspection.rs:12` | `AgentIdentity` | A.14, E.10, F.08 |
| `introspection.rs:90` | `MetacognitiveMonitor` | F.07 |
| `composition.rs:214` | `CompositeAgent` | A.13, F.06 |
| `composition.rs:451` | `impl Agent for CompositeAgent` | F.06 |
| `metamorphosis.rs:46` | `MorphableAgent` | A.15, F.11 |
| `pointer/gc.rs` | pointer GC policy | F.16 |
| `nl_to_format/mod.rs` | NL-to-format converter | F.15 |

---

## `crates/roko-core/src/`

| File | What | Section |
|------|------|---------|
| `agent.rs:35` | `ProviderKind` | B.02, F.02 |
| `agent.rs:234` | `ResolvedModel` | B.04 |
| `agent.rs:253` | `resolve_model()` | B.04 |
| `agent.rs:285` | `TaskRequirements` | B.13 |
| `agent.rs:392` | `select_model_for_task()` | B.13 |
| `agent.rs:445` | `ModelTier` | E.01 |
| `agent.rs:582` | `AgentRole` | A.10 |
| `config/schema.rs:452` | `effective_providers()` | B.05 |
| `config/schema.rs:510` | `effective_models()` | B.05 |
| `config/schema.rs:981` | `ProviderConfig` | B.01 |
| `config/schema.rs:1110` | `ModelProfile` | B.03 |
| `config/schema.rs:1256` | `AgentConfig` | E.10 |
| `config/schema.rs:1636` | `RoutingAlgorithm` | E.19 |
| `tool/handler.rs` | `ToolHandler` trait | C.14, F.01 |
| `tool/registry.rs` | `ToolRegistry` | D.11 |

---

## `crates/roko-learn/src/`

| File | What | Section |
|------|------|---------|
| `cascade_router.rs:994` | `CascadeRouter` | E.02, E.14 |
| `model_router.rs:449` | `ThompsonArm` | E.05 |
| `model_router.rs:655` | `LinUCBRouter` | E.03, E.14 |
| `anomaly.rs:20` | `AnomalyDetector` | E.06 |
| `active_inference.rs:17` | `BeliefState` | E.08 |
| `runtime_feedback.rs` | learning feedback loop | E.18, F.05, F.13 |
| `skill_library.rs:1010` | `SkillLibrary` | F.13 |

Note:

- there is currently no `shared_memory.rs` or `agent_archive.rs` in `crates/roko-learn/src/`;
- `F.12` and `F.14` are still conceptual gaps, not hidden files with stale paths.

---

## `crates/roko-plugin/src/`

| File | What | Section |
|------|------|---------|
| `lib.rs:39` | `FeedbackSignal` | F.05 |
| `lib.rs:55` | `EventSourceKind` | F.04 |
| `lib.rs:68` | `FileWatchEventSource` | F.04 |
| `lib.rs:121` | `EventSource` trait | F.04 |
| `lib.rs:134` | `CronEventSource` | F.04 |
| `lib.rs:586` | `FeedbackCollector` trait | F.05 |
| `lib.rs:601` | `PluginManifest` | F.03, F.05 |
| `lib.rs:609` | `PluginBuilder` | F.03 |

---

## `crates/roko-runtime/src/`

| File | What | Section |
|------|------|---------|
| `process.rs:76` | `SupervisionStrategy` | F.09 |
| `process.rs:545` | `restart_wave()` | F.09 |

---

## `crates/roko-cli/src/`

| File | What | Section |
|------|------|---------|
| `run.rs:327` | `spawn_agent_scoped()` callsite | C.17, D.14 |
| `run.rs:498` | explicit `ToolDispatcher` setup on single-run path | C.17, E.18 |
| `agent_spawn.rs:64` | `spawn_agent_scoped()` | D.14 |
| `agent_spawn.rs:78` | `spawn_agent_with_layer()` | D.14 |
| `orchestrate.rs:2135` | `PlanRunner` | D.01, E.18 |
| `orchestrate.rs:3896` | `run_conductor_check()` | E.18 |
| `main.rs:3691` | direct research `create_agent_for_model()` call | D.14 |
| `main.rs:3879` | direct Gemini research `create_agent_for_model()` call | D.14 |
| `main.rs:3981` | direct Perplexity research `create_agent_for_model()` call | D.14 |

Important negative finding:

- as of this verification pass, `ToolDispatcher` is still not referenced from `crates/roko-cli/src/orchestrate.rs`.
