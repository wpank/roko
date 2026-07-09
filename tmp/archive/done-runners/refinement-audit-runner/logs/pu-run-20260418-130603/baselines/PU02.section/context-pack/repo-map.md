# Repo Map — Shared Agent Context

Quick reference for agents working on `02` agent parity.

## Workspace Root

`/Users/will/dev/nunchi/roko/roko/`

## High-Value Paths

| What | Path | Why It Matters In Batch 02 |
|------|------|----------------------------|
| Agent trait and result types | `crates/roko-agent/src/agent.rs` | core agent contract |
| Shared response types | `crates/roko-agent/src/chat_types.rs`, `translate/mod.rs`, `usage.rs` | duplication and ownership seam |
| Provider factory | `crates/roko-agent/src/provider/mod.rs` | adapter selection, scoped dispatcher wiring |
| Tool loop | `crates/roko-agent/src/tool_loop/` | runtime tool execution stack |
| Dispatcher + safety | `crates/roko-agent/src/dispatcher/`, `safety/` | validate-before-executing path |
| MCP pipeline | `crates/roko-agent/src/mcp/` | dynamic tool discovery and handler resolution |
| Agent pools | `crates/roko-agent/src/pool.rs`, `multi_pool.rs` | useful context, but runtime activation usually belongs to `01` |
| Agent introspection | `crates/roko-agent/src/introspection.rs` | temperament string, monitor wiring |
| CLI single-run path | `crates/roko-cli/src/run.rs` | cleaner reference path |
| CLI plan path | `crates/roko-cli/src/orchestrate.rs` | main runtime gap |
| CLI entrypoint helpers | `crates/roko-cli/src/agent_spawn.rs`, `main.rs` | creation-site consolidation |
| Shared config/types | `crates/roko-core/src/agent.rs`, `config/schema.rs` | temperament and shared response ownership |
| Routing implementation | `crates/roko-learn/src/cascade_router.rs`, `model_router.rs` | temperament propagation and routing behavior |
| Agent docs | `docs/02-agents/` | source material being checked |
| Parity batch | `tmp/docs-parity/02/` | execution contract and findings |

## Important Corrections

Use these instead of the older stale anchors:

- `Usage` lives in `crates/roko-agent/src/usage.rs`, not `roko-core/src/agent.rs`.
- `LlmBackend` lives in `crates/roko-agent/src/tool_loop/mod.rs`, not only in `tool_loop/backends/mod.rs`.
- there is no `crates/roko-agent/src/process.rs`; supervision code lives in `crates/roko-runtime/src/process.rs`.
- `find_mcp_config` is the current MCP config discovery helper in `crates/roko-agent/src/mcp/config.rs`.

## Search Priorities

Before editing, search these first:

```bash
rg -n "pub struct ChatResponse|pub struct ResponseMetadata|pub struct Usage" crates/roko-agent crates/roko-core
rg -n "create_agent_for_model|build_tool_dispatcher|spawn_agent_scoped|spawn_agent_with_layer" crates/roko-agent crates/roko-cli
rg -n "ToolDispatcher|ToolLoopAgent|MetacognitiveMonitor" crates/roko-agent crates/roko-cli
rg -n "max_tools|max_tools_before_degrade" crates/roko-agent crates/roko-core
rg -n "temperament|AgentIdentity|ModelTier|RoutingAlgorithm" crates/roko-agent crates/roko-core crates/roko-learn crates/roko-cli
rg -n "Anthropic|create_tool_loop_backend|LlmBackend" crates/roko-agent/src/tool_loop crates/roko-agent/src/provider
```

## Build Commands

```bash
cargo build --workspace
cargo test --workspace
cargo clippy --workspace --no-deps -- -D warnings
```

## Practical Rules

1. Reuse the good `run.rs` path where possible.
2. Do not widen a type-ownership cleanup into a large semantic redesign.
3. If `orchestrate.rs` starts ballooning, prove one production path and stop.
4. If a task really belongs to verification, learning, or orchestration, record the handoff and stop.
