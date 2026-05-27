# W2-A: MCP Status Types

## Context

MCP spawn/discovery failures are silent — 6 `continue` sites in `setup_session_mcp_tools()`
(bridge_events.rs:1969-2085) only log warnings internally. The IDE has no structured way to
know that MCP failed or why. Before fixing the accumulation logic (W2-B), we need the types.

## Prerequisites

**None.** This batch adds new types only — no existing code is modified.

## File Locations

**One file:** `/Users/will/dev/nunchi/roko/roko/crates/roko-acp/src/types.rs`

## Change 1: Add McpServerStatus and McpInitStatus after McpServerConfig

The McpServerConfig struct ends at line 263. Add the new types after line 263, before
the McpTransport enum (which starts at line 265).

FIND (lines 263-265):
```rust
}

/// Supported MCP transport configurations.
```

REPLACE WITH:
```rust
}

/// Status of an MCP server after initialization attempt.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpServerStatus {
    /// Server name (matches McpServerConfig.name).
    pub name: String,
    /// Initialization outcome.
    pub status: McpInitStatus,
    /// Number of tools discovered (only when status is Ready).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tools_count: Option<usize>,
    /// Human-readable error message (only when status is not Ready).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// MCP server initialization status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum McpInitStatus {
    /// Server is running and tools are available.
    Ready,
    /// Failed to spawn the server process (binary not found, permission denied, etc.).
    SpawnFailed,
    /// Server process started but MCP initialize handshake failed.
    InitializeFailed,
    /// Server process started but MCP initialize timed out.
    InitializeTimeout,
    /// Initialize succeeded but tools/list call failed.
    ToolsListFailed,
    /// Initialize succeeded but tools/list call timed out.
    ToolsListTimeout,
    /// Transport type not supported (e.g., HTTP transport for session MCP).
    TransportUnsupported,
}

impl McpServerStatus {
    /// Create a Ready status with tool count.
    pub fn ready(name: String, tools_count: usize) -> Self {
        Self {
            name,
            status: McpInitStatus::Ready,
            tools_count: Some(tools_count),
            error: None,
        }
    }

    /// Create a failed status with error message.
    pub fn failed(name: String, status: McpInitStatus, error: impl std::fmt::Display) -> Self {
        Self {
            name,
            status,
            tools_count: None,
            error: Some(error.to_string()),
        }
    }

    pub fn is_ready(&self) -> bool {
        self.status == McpInitStatus::Ready
    }
}

/// Supported MCP transport configurations.
```

## What Changed

1. Added `McpServerStatus` struct — represents the result of attempting to initialize one MCP server
2. Added `McpInitStatus` enum — 7 variants covering every failure mode in `setup_session_mcp_tools`
3. Added convenience constructors `::ready()` and `::failed()` — used by W2-B when accumulating statuses
4. All types derive Serialize/Deserialize with `rename_all = "camelCase"` or `"snake_case"` for JSON wire format

## What NOT to Change

- Do NOT modify `McpServerConfig` (that's W2-D)
- Do NOT modify `McpTransport` enum
- Do NOT modify `bridge_events.rs` (that's W2-B and W2-C)
- Do NOT modify any other files

## Verification

After Phase 2:
```bash
# Types should compile clean
cargo build -p roko-acp 2>&1 | head -20
# No regressions
cargo test -p roko-acp 2>&1 | tail -10
```

## Estimated Effort

10 minutes. Pure type definitions, no behavioral changes.
