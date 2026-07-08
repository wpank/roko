# Repo Map — Audited Agent Context

Quick reference for agents working on the narrowed `02` docs-parity pass.

## Workspace Facts

- workspace root: `/Users/will/dev/nunchi/roko/roko/`
- workspace members: `36`
- total Rust LOC: `322,088`
- live `RokoEvent` variants: exactly `2` (`PlanRevision`, `PrdPublished`)
- current built-in tool count in `roko-std`: `16`

## High-Value Paths

| What | Path | Why it matters in batch 02 |
|---|---|---|
| Agent trait and result types | `crates/roko-agent/src/agent.rs` | base agent contract and `impl Agent` search anchor |
| Response surface | `crates/roko-agent/src/chat_types.rs`, `translate/mod.rs`, `usage.rs` | duplicate ownership and consumer audit |
| Provider factory | `crates/roko-agent/src/provider/mod.rs` | current creation path and dispatcher factory anchor |
| Tool loop | `crates/roko-agent/src/tool_loop/` | shared tool execution stack |
| Dispatcher + safety | `crates/roko-agent/src/dispatcher/`, `safety/` | concrete validate-before-executing path |
| MCP pipeline | `crates/roko-agent/src/mcp/` | live config discovery and tool bridging |
| Agent pools | `crates/roko-agent/src/pool.rs`, `multi_pool.rs` | real primitives; runtime activation is usually a `01` handoff |
| Introspection | `crates/roko-agent/src/introspection.rs` | `AgentIdentity.temperament` and `MetacognitiveMonitor` boundary |
| Sidecar | `crates/roko-agent-server/` | per-agent HTTP server with messaging and relay coverage |
| CLI single-run path | `crates/roko-cli/src/run.rs` | cleanest live runtime reference |
| CLI scoped spawn helpers | `crates/roko-cli/src/agent_spawn.rs` | shared entrypoint helper for CLI creation |
| CLI plan path | `crates/roko-cli/src/orchestrate.rs` | main path that needs evidence-based wording |
| Shared config/types | `crates/roko-core/src/agent.rs`, `config/schema.rs` | current type/config boundary for models and temperament |
| Routing implementation | `crates/roko-learn/src/cascade_router.rs`, `active_inference.rs` | routing is live; active inference exists but needs careful wording |
| Runtime lifecycle | `crates/roko-runtime/src/process.rs`, `event_bus.rs` | `ProcessSupervisor` and the 2 live `RokoEvent` variants |
| Agent docs | `docs/02-agents/` | source material being checked |
| Parity batch | `tmp/docs-parity/02/` | execution contract and audit notes |

## Important Corrections

Use these instead of stale anchors:

- `Usage` lives in `crates/roko-agent/src/usage.rs`
- `LlmBackend` lives in `crates/roko-agent/src/tool_loop/mod.rs`
- there is no `crates/roko-agent/src/process.rs`; supervision code lives in `crates/roko-runtime/src/process.rs`
- `find_mcp_config` lives in `crates/roko-agent/src/mcp/config.rs`
- `orchestrate.rs` references scoped spawn helpers, but docs should still verify whether the downstream path actually reaches dispatcher/tool-loop behavior

## Search Priorities

Before editing, search these first:

```bash
rg -n "pub struct ChatResponse|pub struct ResponseMetadata|pub struct Usage" crates/roko-agent crates/roko-core
rg -n "spawn_agent_scoped|spawn_agent_with_layer|create_agent_for_model" crates/roko-cli/src crates/roko-agent/src
rg -n "ToolDispatcher|ToolLoopAgent|MetacognitiveMonitor" crates/roko-cli/src crates/roko-agent/src
rg -n "agent.mcp_config|find_mcp_config|setup_mcp|resolve_mcp_config_path" crates/roko-cli/src crates/roko-agent/src
rg -n "temperament|AgentIdentity|AgentConfig|RoutingAlgorithm" crates/roko-agent crates/roko-core crates/roko-learn crates/roko-cli
rg -n "Anthropic|Perplexity|Gemini|create_tool_loop_backend|LlmBackend" crates/roko-agent/src
```

## Practical Rules

1. Do not call a surface `wired` unless you can point to a runtime path.
2. Do not widen a type-ownership audit into a large migration plan.
3. If `orchestrate.rs` evidence gets fuzzy, record the uncertainty explicitly instead of smoothing it over.
4. If a finding really belongs to verification, learning, or orchestration, record the handoff and stop.
