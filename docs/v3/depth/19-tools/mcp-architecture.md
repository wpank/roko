# MCP Architecture

> Depth file for [19-TOOLS-PLUGINS](../../19-TOOLS-PLUGINS.md) -- Model Context
> Protocol: JSON-RPC 2.0, stdio transport, tool conversion, dynamic registry,
> capabilities negotiation, and MCP targets.

---

## 1. Overview

The Model Context Protocol (MCP) is a JSON-RPC 2.0 protocol for extending LLM
agents with external tools at runtime. MCP provides dynamic tool discovery
from external servers -- complementing the statically compiled builtins and
plugin-declared tools.

MCP integration lives in `crates/roko-agent/src/mcp/`. The implementation
includes:

- MCP client (JSON-RPC over stdio transport)
- Tool converter (MCP tool schema -> Roko ToolDef)
- Dynamic tool registry (merges MCP tools with builtins)
- Configuration passthrough (`agent.mcp_config` -> `--mcp-config`)
- Retained live clients/resolvers for provider tool loops

---

## 2. Protocol: JSON-RPC 2.0 over stdio

The primary transport is newline-delimited JSON-RPC over stdin/stdout:

```
Client (roko-agent)          Server (roko-mcp-github)
       |                              |
       |  {"method":"initialize"}     |
       | ---------------------------> |
       |  {"result":{caps...}}        |
       | <--------------------------- |
       |                              |
       |  {"method":"tools/list"}     |
       | ---------------------------> |
       |  {"result":{"tools":[...]}}  |
       | <--------------------------- |
       |                              |
       |  {"method":"tools/call",     |
       |   "params":{"name":"..."}}   |
       | ---------------------------> |
       |  {"result":{"content":[...]}}|
       | <--------------------------- |
```

Transport advantages:
- **Process isolation** -- crashing server does not affect the agent
- **Language agnostic** -- servers in any language
- **Security boundary** -- untrusted tools in their own process

---

## 3. Initialization Handshake

The client sends `initialize` with protocol version and capabilities. The
server responds with its capabilities. The client then sends
`notifications/initialized`.

```rust
pub struct InitializeRequest {
    pub protocol_version: String,  // "2025-11-25"
    pub capabilities: ClientCapabilities,
    pub client_info: ClientInfo,
}

pub struct ServerCapabilities {
    pub tools: Option<ToolsCapability>,
    pub resources: Option<ResourcesCapability>,
    pub prompts: Option<PromptsCapability>,
    pub logging: Option<LoggingCapability>,
}
```

Feature usage is gated by server capabilities -- the client does not call
`resources/list` if the server lacks the `resources` capability.

---

## 4. Tool Conversion

MCP tool schemas are converted to Roko `ToolDef` format:

- Tools are namespaced by server name: `github.get_pr`, `scripts.pm_sync`
- MCP `readOnly: true` maps to `ToolPermission::read_only()`
- MCP `readOnly: false` or absent maps to `ToolPermission::writes()`
  (conservative default)
- `idempotent: true` marks the tool as safe to retry on failure

### Tool Annotations (Spec 2025-03-26+)

```rust
pub struct McpToolAnnotations {
    pub read_only: Option<bool>,
    pub open_world: Option<bool>,
    pub idempotent: Option<bool>,
    pub title: Option<String>,
}
```

| MCP Annotation | Roko Mapping |
|---|---|
| `readOnly: true` | `ToolPermission::read_only()` |
| `readOnly: false` | `ToolPermission::writes()` |
| `openWorld: true` | Network access flagged |
| `idempotent: true` | Safe to retry |

---

## 5. Dynamic Registry Merge

The merged registry combines static, plugin, and MCP tools with precedence:
**static > plugin > MCP**. Name collisions are resolved by precedence order.

Tool change notifications (`notifications/tools/list_changed`) trigger
re-discovery and registry update.

```rust
pub struct DynamicToolRegistry {
    tools: Vec<ToolDef>,
    sandbox_configs: HashMap<String, SandboxConfig>,
}
```

---

## 6. MCP Targets

### Claude and Codex CLI MCP

When dispatching to Claude CLI or Codex CLI, Roko passes MCP config via
`--mcp-config`. The CLI spawns configured MCP servers alongside the agent
session and composes live tool definitions into the audited tool loop.

### Cursor and Hermes ACP

The ACP in `roko-acp` provides editor integration. Cursor and Hermes consume
canonical tool handlers through authenticated per-call MCP. ACP advertises
resolved model capabilities and enforces mutation consent, experiments,
truthful capabilities, persisted USD budgets, and shared health/rate-aware
selection.

### Gemini CLI MCP

The Gemini CLI path uses native stream-JSON transport and task-scoped system
settings. MCP tools are composed via authenticated per-call MCP.

### Definition-Only MCP

HTTP-provider MCP discovery retains executable clients and resolvers for
Anthropic/OpenAI/Gemini/Perplexity/Cerebras tool loops. Definition-only MCP
advertisements -- servers that list tools but cannot execute them -- **fail
closed**. This prevents tools from being advertised without execution
capability.

---

## 7. Transport Layers

### stdio (Primary)

Default for local MCP servers. ~1ms latency. Client spawns server as child
process. Communication via newline-delimited JSON-RPC.

### Streamable HTTP (Spec 2025-03-26+)

Single HTTP endpoint for remote tools. ~10-50ms latency. Supports
`mcp-session-id` header for multi-request sessions.

```toml
[[agent.mcp_servers]]
name = "remote-tools"
transport = "http"
endpoint = "https://tools.example.com/mcp"
auth = { type = "bearer", token = "${MCP_AUTH_TOKEN}" }
```

---

## 8. Security

MCP tools are **untrusted by default**:

| Property | Builtins | MCP Tools |
|---|---|---|
| Compiled with Roko | Yes | No |
| Process isolation | No | Yes |
| Default permission | Per-tool | Write (conservative) |

Credentials are passed via environment variables, never via stdio:

```toml
[[agent.mcp_servers]]
name = "github"
env = { GITHUB_TOKEN = "${GITHUB_TOKEN}" }
```

---

## 9. Specification Reference

- **MCP Specification 2025-11-25** (Anthropic / Linux Foundation) -- current
  protocol version
- **MCP Specification 2025-03-26** -- introduced Streamable HTTP, tool
  annotations
- Roko targets the 2025-11-25 spec version

---

*Derived from: v1/18-tools/09-mcp-architecture.md. Sampling support section
retained as specification (not yet implemented). HTTP transport implementation
pending.*
