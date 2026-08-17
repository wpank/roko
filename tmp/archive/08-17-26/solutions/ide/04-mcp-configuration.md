# Issue: MCP Configuration and Lifecycle Gaps

## Problem Statement

Several MCP-related behaviors are implicit, undocumented, or not configurable:

1. MCP discovery timeout is hardcoded (not configurable)
2. MCP tool namespacing uses `sanitize_tool_segment` — undocumented for consumers
3. MCP server lifecycle (startup, health, shutdown) has no protocol surface
4. No way to list which MCP servers are connected to a session after creation

## Observed Behaviors

### MCP Tool Naming

When nunchi-mcp exposes `tools_list`, the agent sees `nunchi_tools_list`:
```
base_name = sanitize_tool_segment("nunchi") + "_" + sanitize_tool_segment("tools_list")
         = "nunchi_tools_list"
```

The `sanitize_tool_segment` function (bridge_events.rs) replaces non-alphanumeric chars with `_`.
This is correct behavior but undocumented for MCP server authors.

### Discovery Timing

MCP tool discovery happens at first `session/prompt`, not at `session/new`.
The IDE sends `session/new` with mcpServers, gets back sessionId immediately,
then only when the first prompt is sent does `setup_session_mcp_tools()` run.

Observed sequence:
```
1. session/new → immediate response with sessionId + configOptions
2. session/prompt → triggers MCP spawn + initialize + tools/list
3. If discovery succeeds → tools available for that prompt
4. If discovery fails → "No MCP tools" message in agent output
```

### Discovery Timeout

In `bridge_events.rs`, the timeout is:
```rust
let discovery_timeout = Duration::from_secs(10);  // Hardcoded
```

For slow MCP servers (e.g., ones that need to compile or connect to remote services),
10 seconds may not be enough. For fast local servers, 10 seconds is wasted wait on failure.

## Proposed Solutions

### 1. Configurable Discovery Timeout

Add to `SessionNewParams` or make it a config option:

```rust
pub struct McpServerConfig {
    pub name: String,
    pub transport: McpTransport,
    #[serde(default = "default_discovery_timeout_ms")]
    pub discovery_timeout_ms: u64,  // Per-server timeout
}

fn default_discovery_timeout_ms() -> u64 { 10_000 }
```

### 2. Eager MCP Discovery (Option)

Allow clients to request eager discovery at session creation:

```rust
pub struct SessionNewParams {
    // ...existing fields...
    #[serde(default)]
    pub eager_mcp_discovery: bool,  // If true, discover tools before returning sessionId
}
```

When `eager_mcp_discovery: true`, session/new blocks until MCP tools are discovered
(or fail), and includes the status in the response. This is useful for IDEs that want
to show tool availability immediately.

When false (default), current behavior is preserved for backward compatibility.

### 3. Session MCP Status Query

Add a new method to query MCP status after session creation:

```rust
// New JSON-RPC method
"session/mcp_status" => {
    let params: SessionIdParams = parse_params(params, &method)?;
    let session = sessions.get(&params.session_id)?;
    Ok(json!({
        "servers": session.mcp_statuses(),  // Vec<McpServerStatus>
    }))
}
```

### 4. Document Tool Naming Convention

For MCP server authors, document:

```
Final tool name = sanitize(server_name) + "_" + sanitize(tool_name)

sanitize: replace [^a-zA-Z0-9] with "_", collapse runs, trim leading/trailing "_"

Example:
  server: "nunchi"  + tool: "tools/list"  → "nunchi_tools_list"
  server: "my-db"   + tool: "query.run"   → "my_db_query_run"
```

## Implementation Location

| File | Change |
|------|--------|
| `crates/roko-acp/src/types.rs` | Add `discovery_timeout_ms` to McpServerConfig |
| `crates/roko-acp/src/bridge_events.rs:1985` | Use per-server timeout instead of hardcoded |
| `crates/roko-acp/src/handler.rs` | Add `session/mcp_status` method |
| `crates/roko-acp/src/types.rs` | Add `eager_mcp_discovery` to SessionNewParams |
| Documentation | Tool naming convention for MCP authors |

## Priority

Medium. The current behavior works for local, fast MCP servers. This becomes important
when supporting remote/slow MCP servers or when IDEs need to show connection status.
