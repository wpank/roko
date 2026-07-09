# Issue: MCP Spawn/Discovery Failures Are Silent

## Problem Statement

When `session/new` is called with `mcpServers`, failures at every stage
(binary not found, initialize timeout, tools/list failure) are logged at
warn level but never propagated to the client. The session succeeds and
the model later says "No MCP tools were discovered" as an informal text
message — not a structured error the client can act on.

## Reproduction

```bash
# Pass a nonexistent binary
echo '{"jsonrpc":"2.0","method":"session/new","id":1,"params":{
  "model":"sonnet",
  "mcpServers":[{"name":"nunchi","transport":{"type":"stdio",
    "command":"/nonexistent/binary","args":[],"env":{}}}]
}}' | roko acp --quiet --no-serve --config ~/.nunchi/roko/roko.toml

# Result: session/new SUCCEEDS with a valid sessionId
# No error is returned. Client has no idea MCP failed.
```

## Root Cause

In `crates/roko-acp/src/bridge_events.rs` (lines 1993-2003):

```rust
let transport = match McpStdioTransport::spawn(command, args) {
    Ok(transport) => transport,
    Err(error) => {
        warn!(session_id, server = %server.name, error = %error,
              "failed to spawn session MCP server");
        continue;  // <-- silently skips
    }
};
```

Same pattern for initialize timeout (lines 2007-2027) and tools/list (lines 2029-2049).
Every failure hits `continue` — no error accumulation, no notification to client.

The "No MCP tools" message is sent later in `run_openai_compat_mcp_tool_loop()` (line 1810)
as a `CognitiveEvent::TokenChunk` — effectively a chat message, not a structured event.

## Impact

- IDE cannot show a meaningful error ("MCP server failed to start: binary not found")
- IDE cannot retry or suggest fixes
- User sees the agent say "no tools found" without understanding why
- Debug requires reading roko's internal logs

## Proposed Solution

### A. Add `mcp_status` to session/new result

Extend the `session/new` response to include MCP initialization status:

```rust
// In the SessionNewResult (or as a session/update notification)
#[derive(Serialize)]
struct McpServerStatus {
    name: String,
    status: McpStatus,  // "ready" | "failed"
    tools_count: Option<usize>,
    error: Option<String>,
}

#[derive(Serialize)]
enum McpStatus {
    Ready,
    SpawnFailed,
    InitializeFailed,
    ToolsListFailed,
    Timeout,
}
```

### B. Send a structured session/update notification

After MCP setup, emit a notification the client can parse:

```rust
// Emit after setup_session_mcp_tools() returns
send_notification(transport, "session/update", json!({
    "sessionId": session_id,
    "update": {
        "sessionUpdate": "mcp_status",
        "servers": mcp_statuses  // Vec<McpServerStatus>
    }
})).await;
```

### C. Accumulate errors instead of skipping

```rust
// In setup_session_mcp_tools()
let mut statuses: Vec<McpServerStatus> = Vec::new();

for server in mcp_servers {
    let transport = match McpStdioTransport::spawn(command, args) {
        Ok(t) => t,
        Err(error) => {
            statuses.push(McpServerStatus {
                name: server.name.clone(),
                status: McpStatus::SpawnFailed,
                error: Some(error.to_string()),
                tools_count: None,
            });
            continue;
        }
    };
    // ... similar for initialize/tools_list ...

    statuses.push(McpServerStatus {
        name: server.name.clone(),
        status: McpStatus::Ready,
        tools_count: Some(tools.len()),
        error: None,
    });
}

// Return statuses alongside the runtime
(SessionMcpRuntime { tools, handlers }, statuses)
```

## Implementation Location

| File | Change |
|------|--------|
| `crates/roko-acp/src/bridge_events.rs:1993-2082` | Accumulate statuses in `setup_session_mcp_tools()` |
| `crates/roko-acp/src/bridge_events.rs:1809-1819` | Replace text chunk with structured notification |
| `crates/roko-acp/src/types.rs` | Add `McpServerStatus` and `McpStatus` types |
| `crates/roko-acp/src/handler.rs:168-177` | Thread statuses into response or notification |

## Backward Compatibility

- Adding fields to session/update is non-breaking (clients ignore unknown fields)
- The "No MCP tools" text message can remain as a fallback for non-IDE clients
- New notification type `mcp_status` is additive

## Verification

After implementing the fix, run:

```bash
cd tmp/solutions/ide/tests && ./test-mcp.sh
```

The following tests should change from FAIL to PASS:
- "nonexistent MCP binary -> structured error"
- "MCP binary that exits -> structured error"

Manual verification:
```bash
# Start ACP with bad MCP path
echo '{"jsonrpc":"2.0","method":"session/new","id":1,"params":{
  "mcpServers":[{"name":"bad","transport":{"type":"stdio",
    "command":"/nonexistent","args":[],"env":{}}}]
}}' | roko acp --quiet --no-serve --config ~/.nunchi/roko/roko.toml

# BEFORE fix: session succeeds silently
# AFTER fix: either session/new returns error, or a session/update
#   notification with sessionUpdate:"mcp_status" is emitted
```
