# 05-agent/mcp-integration -- MCP Integration

> Model Context Protocol integration: config discovery, CLI passthrough,
> HTTP clients and resolvers, tool conversion, multi-server dedup,
> DynamicToolRegistry, and the fail-closed policy.

**Parent:** [05-AGENT](../../05-AGENT.md)

**Source:** `crates/roko-agent/src/process/mcp.rs` (launch normalization),
`crates/roko-agent/src/mcp.rs` (McpRuntime)

---

## 1. What Is MCP

The Model Context Protocol (MCP) is a standard for connecting LLM agents to
external tools and data sources via JSON-RPC over stdio. An MCP server exposes
tools (with JSON schema definitions), and an MCP client discovers and invokes
them at runtime. This allows dynamic tool registration without recompiling.

Roko integrates MCP at two levels: CLI passthrough for subprocess providers,
and HTTP clients/resolvers for API-based providers.

---

## 2. Two Integration Paths

### CLI passthrough

For CLI-based providers (ClaudeCli, CodexCli, GeminiCli), MCP config is passed
directly via the `--mcp-config` flag. The provider's internal MCP client handles
server lifecycle and tool execution.

The config file at `.roko/mcp-config.json` is authored by
`PlanRunner::resolve_mcp_config_path` in `crates/roko-cli/src/runner/event_loop.rs`.

```rust
// crates/roko-agent/src/process/mcp.rs
pub fn normalize_mcp_launch(launch: McpLaunch, probe_root: &Path) -> McpLaunch;
```

`normalize_mcp_launch` resolves relative paths and validates the MCP server
configuration before launch. This ensures consistent behavior regardless of
the working directory.

### HTTP clients and resolvers

For HTTP-based providers (AnthropicApi, OpenAiCompat, GeminiApi, PerplexityApi,
CerebrasApi), retained live MCP clients and resolvers handle tool loops. The
`McpRuntime` in `crates/roko-agent/src/mcp.rs` manages client lifecycles.

---

## 3. MCP Config Discovery

MCP server configurations are discovered from config files:

```json
{
    "mcpServers": {
        "filesystem": {
            "command": "mcp-server-filesystem",
            "args": ["--root", "/project"],
            "env": {}
        },
        "github": {
            "command": "mcp-server-github",
            "args": [],
            "env": { "GITHUB_TOKEN": "..." }
        }
    }
}
```

Search paths for config discovery:

1. Explicit `mcp_config` path in `roko.toml`
2. `.roko/mcp-config.json` in the workspace
3. Project root `.mcp.json`
4. User home `~/.mcp.json`

The `roko.toml` configuration:

```toml
[agent]
mcp_config = ".mcp.json"
```

Auto-discovery means MCP "just works" for projects with an `.mcp.json` file.

---

## 4. Tool Definitions

Roko ships 35 tool definitions by default:

| Category | Count | Source |
|----------|-------|--------|
| Executable local tools | 16 | `roko-std` built-in handlers |
| GitHub MCP tools | 19 | GitHub MCP server definitions |
| Typed optional-chain placeholders | 17 | Typed but not yet executable |
| **Total** | **52** | Including placeholders |

The `roko-std` crate (`crates/roko-std/`) provides canonical built-in handlers.
The dispatcher resolves handlers through a `HandlerResolver` trait, keeping
`roko-agent` free of the `roko-std` dependency (decoupling learned from
MISTAKES-LEARNED.md M19).

---

## 5. Tool Conversion

MCP tool definitions are converted to Roko's canonical `ToolDef` format:

```rust
pub fn mcp_to_tool_def(mcp_tool: &McpToolDef) -> ToolDef {
    ToolDef::new(
        &mcp_tool.name,
        &mcp_tool.description,
        ToolCategory::Custom,
        ToolPermission::read_only(),
    )
    .with_schema(mcp_tool.input_schema.clone())
}
```

### Permission assignment

MCP tools default to `read_only()` permissions. External tools registered via
MCP are untrusted by default. The SafetyLayer enforces this -- even if an MCP
tool tries to access the filesystem or network, the path policy and network
policy block it unless explicitly granted higher permissions in config.

---

## 6. Multi-Server Deduplication

When multiple MCP servers expose tools with the same name:

1. If a tool name is unique across all servers, keep it as-is.
2. If a tool name appears in multiple servers, prefix with the server name
   (e.g., `filesystem:read_file` vs `github:read_file`).
3. If a tool name collides with a built-in Roko tool, the built-in takes
   precedence and the MCP tool is prefixed.

---

## 7. DynamicToolRegistry

The registry composes static built-in tools with dynamically discovered MCP
tools:

```rust
pub struct DynamicToolRegistry {
    static_tools: Vec<ToolDef>,
    mcp_tools: Vec<ToolDef>,
}
```

It implements the `ToolRegistry` trait, so the `ToolDispatcher` uses it
transparently -- it does not know whether a tool came from the built-in catalog
or from an MCP server:

```rust
impl ToolRegistry for DynamicToolRegistry {
    fn get(&self, name: &str) -> Option<&ToolDef> {
        self.static_tools.iter().find(|t| t.name == name)
            .or_else(|| self.mcp_tools.iter().find(|t| t.name == name))
    }
}
```

Static tools take precedence over MCP tools with the same name.

---

## 8. MCP in the ToolLoop

For HTTP-based agents using Roko's ToolLoop, the MCP integration pipeline:

```
1. discover_mcp_servers()        -> Vec<McpServerConfig>
2. connect_and_list_tools()      -> Vec<(server_name, McpToolDef)>
3. dedup_tools()                 -> Vec<McpToolDef>
4. mcp_to_tool_def()             -> Vec<ToolDef>
5. DynamicToolRegistry::new()    -> merges built-in + MCP tools
6. ToolLoop::new(translator, dispatcher, backend)
7. loop.run(system, user, all_tools, ctx)
```

The `ToolDispatcher` handles MCP tool calls by routing through the
`HandlerResolver`. MCP handlers wrap the MCP client for tool execution:

```
ToolDispatcher -> HandlerResolver -> McpHandler -> McpClient -> MCP Server
```

---

## 9. Fail-Closed Policy

**Definition-only MCP advertisements fail closed.** Servers that declare tools
but do not provide execution endpoints are rejected by the ToolDispatcher.
A tool call against a definition-only MCP server results in a
`ToolError::HandlerNotFound` rather than silent failure.

This policy ensures that:
- No tool call can be silently dropped
- The agent always receives explicit feedback about tool availability
- Partially configured MCP servers do not create confusion

---

## 10. MCP Configuration via CLI

```toml
# roko.toml
[mcp]
servers = [
    { name = "code-intel", command = "roko-mcp-code", args = [] },
    { name = "github", command = "mcp-server-github", args = [] },
]

# Or via CLI:
# roko config mcp list    -- List configured MCP servers
# roko config mcp test    -- Test MCP server connectivity
# roko config mcp add     -- Add a new MCP server
```

---

## 11. Plugin MCP Integration

The `roko-plugin` crate provides MCP integration for plugins:

- Claude/Codex MCP passthrough
- Cursor/Hermes ACP integration
- Native authenticated Gemini CLI MCP

Plugin-provided MCP tools go through the same `DynamicToolRegistry` and
`ToolDispatcher` pipeline as built-in tools, with the addition of plugin
admission checks (signed manifests, capability policy).

---

## 12. Citations

1. `crates/roko-agent/src/process/mcp.rs` -- normalize_mcp_launch, MCP launch
   configuration.
2. `crates/roko-agent/src/mcp.rs` -- McpRuntime, client lifecycle management.
3. `crates/roko-std/` -- 35 built-in tool definitions and handlers.
4. `crates/roko-agent/src/dispatcher/mod.rs` -- HandlerResolver, fail-closed
   tool dispatch.
5. `crates/roko-cli/src/runner/event_loop.rs` -- resolve_mcp_config_path.
6. `crates/roko-plugin/` -- Plugin MCP integration.
