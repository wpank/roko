# W2-D: Per-Server MCP Discovery Timeout

## Context

The MCP discovery timeout is hardcoded at 5 seconds (`DEFAULT_MCP_DISCOVERY_TIMEOUT_SECS`
in roko-core/defaults.rs:241). This constant is applied twice per server: once for
`client.initialize()` and once for `client.list_tools()` — so worst case is 10s per server.

This is:
- Too long for fast local servers that fail immediately (user waits 5-10s for nothing)
- Too short for slow remote servers or servers that need compilation on startup

Making it configurable per-server lets clients optimize for their setup.

## Prerequisites

- **W2-B must be completed** — setup_session_mcp_tools already has the per-server loop
  with `let discovery_timeout = Duration::from_secs(DEFAULT_MCP_DISCOVERY_TIMEOUT_SECS);`
  that we'll move inside the loop

## File Locations

Two files:
1. `/Users/will/dev/nunchi/roko/roko/crates/roko-acp/src/types.rs` — McpServerConfig struct
2. `/Users/will/dev/nunchi/roko/roko/crates/roko-acp/src/bridge_events.rs` — setup_session_mcp_tools timeout usage

## Change 1: Add discovery_timeout_ms to McpServerConfig

**File:** `crates/roko-acp/src/types.rs`

The current McpServerConfig (lines 255-263):
```rust
/// MCP server configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpServerConfig {
    /// MCP server name.
    pub name: String,
    /// Transport configuration for the MCP server.
    pub transport: McpTransport,
}
```

FIND (lines 255-263):
```rust
/// MCP server configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpServerConfig {
    /// MCP server name.
    pub name: String,
    /// Transport configuration for the MCP server.
    pub transport: McpTransport,
}
```

REPLACE WITH:
```rust
/// MCP server configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpServerConfig {
    /// MCP server name.
    pub name: String,
    /// Transport configuration for the MCP server.
    pub transport: McpTransport,
    /// Per-server discovery timeout in milliseconds.
    /// If absent, uses the system default (5000ms = `DEFAULT_MCP_DISCOVERY_TIMEOUT_SECS`).
    /// Applied independently to `initialize()` and `list_tools()` calls.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub discovery_timeout_ms: Option<u64>,
}
```

## Change 2: Use per-server timeout in setup_session_mcp_tools

**File:** `crates/roko-acp/src/bridge_events.rs`

After W2-B, the function has the timeout declared outside the loop (line 1977):
```rust
    let discovery_timeout = Duration::from_secs(DEFAULT_MCP_DISCOVERY_TIMEOUT_SECS);
```

This line needs to move INSIDE the per-server `for` loop and use the per-server value.

FIND (line 1977, before the `for server in mcp_servers` loop):
```rust
    let discovery_timeout = Duration::from_secs(DEFAULT_MCP_DISCOVERY_TIMEOUT_SECS);

    for server in mcp_servers {
```

REPLACE WITH:
```rust
    for server in mcp_servers {
        let discovery_timeout = server
            .discovery_timeout_ms
            .map(Duration::from_millis)
            .unwrap_or_else(|| Duration::from_secs(DEFAULT_MCP_DISCOVERY_TIMEOUT_SECS));
```

**Important:** This moves the `discovery_timeout` binding from outside the loop to inside it.
The rest of the loop body (which uses `discovery_timeout` for both `tokio::time::timeout`
calls) remains unchanged. Make sure only the declaration line is moved/changed — do NOT
duplicate or remove the `for server in mcp_servers {` line.

## Wire Format

IDE clients can now pass per-server timeouts:

```json
{
  "jsonrpc": "2.0",
  "method": "session/new",
  "id": 1,
  "params": {
    "mcpServers": [
      {
        "name": "fast-local",
        "transport": {"type": "stdio", "command": "fast-mcp", "args": []},
        "discoveryTimeoutMs": 1000
      },
      {
        "name": "slow-remote",
        "transport": {"type": "stdio", "command": "slow-mcp", "args": []},
        "discoveryTimeoutMs": 30000
      }
    ]
  }
}
```

Servers without `discoveryTimeoutMs` use the 5s default. The field is omitted from JSON
serialization when absent (`skip_serializing_if = "Option::is_none"`).

## What NOT to Change

- Do NOT change `DEFAULT_MCP_DISCOVERY_TIMEOUT_SECS` in defaults.rs (5s is fine as default)
- Do NOT add `eager_mcp_discovery` to SessionNewParams (defer to follow-up)
- Do NOT modify handler.rs

## Verification

After Phase 2:
```bash
# Test with very short timeout — nonexistent binary should fail faster
echo '{"jsonrpc":"2.0","method":"session/new","id":1,"params":{"mcpServers":[{"name":"test","transport":{"type":"stdio","command":"/nonexistent","args":[]},"discoveryTimeoutMs":500}]}}' \
  | timeout 5 cargo run -p roko-cli -- acp --quiet --no-serve 2>/dev/null \
  | head -5
# Should get response within ~1s, not 5s

# Test without timeout — should use 5s default
echo '{"jsonrpc":"2.0","method":"session/new","id":1,"params":{"mcpServers":[{"name":"test","transport":{"type":"stdio","command":"/nonexistent","args":[]}}]}}' \
  | timeout 10 cargo run -p roko-cli -- acp --quiet --no-serve 2>/dev/null \
  | head -5
```

## Estimated Effort

15 minutes. Two small changes: one struct field, one line moved inside a loop.
