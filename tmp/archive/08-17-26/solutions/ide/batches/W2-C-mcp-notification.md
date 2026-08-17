# W2-C: Emit MCP Status Notification via CognitiveEvent

## Context

After W2-B, `setup_session_mcp_tools` returns `(SessionMcpRuntime, Vec<McpServerStatus>)`.
Now we need to surface the statuses to the IDE client via a structured `session/update`
notification so it can show meaningful error messages ("MCP server 'nunchi' failed: binary not found").

The flow is:
1. `run_openai_compat_mcp_tool_loop` calls `setup_session_mcp_tools` → gets `(mcp_state, mcp_statuses)`
2. Emit `CognitiveEvent::McpStatus(mcp_statuses)` via the event sender
3. The event consumer in the streaming loop receives it and calls `map_event_to_update`
4. `map_event_to_update` converts it to a `SessionUpdate::McpStatusUpdate` notification
5. The transport layer serializes it and sends it to the IDE as a `session/update` JSON-RPC notification

## Prerequisites

- **W2-A must be completed** — McpServerStatus and McpInitStatus types exist
- **W2-B must be completed** — setup_session_mcp_tools returns tuple with statuses

## File Locations

Three files:
1. `/Users/will/dev/nunchi/roko/roko/crates/roko-acp/src/bridge_events.rs` — CognitiveEvent enum, caller, map_event_to_update
2. `/Users/will/dev/nunchi/roko/roko/crates/roko-acp/src/types.rs` — SessionUpdate enum

## Change 1: Add McpStatus variant to CognitiveEvent enum

**File:** `crates/roko-acp/src/bridge_events.rs`

The CognitiveEvent enum is at lines 207-236. It currently has 8 variants ending with MaxTokens.

FIND (lines 234-236):
```rust
    /// Prompt execution stopped because the token budget was exhausted.
    MaxTokens,
}
```

REPLACE WITH:
```rust
    /// Prompt execution stopped because the token budget was exhausted.
    MaxTokens,
    /// MCP server discovery results.
    McpStatus(Vec<crate::types::McpServerStatus>),
}
```

## Change 2: Emit McpStatus event in run_openai_compat_mcp_tool_loop

**File:** `crates/roko-acp/src/bridge_events.rs`

After W2-B, the call site at line ~1809 looks like:

```rust
    let (mcp_state, mcp_statuses) = setup_session_mcp_tools(session_id, mcp_servers, event_sender.clone()).await;
```

The current code after the call (lines 1810-1819) is:
```rust
    if mcp_state.tools.is_empty() {
        send_cognitive_event(
            &event_sender,
            CognitiveEvent::TokenChunk(
                "No MCP tools were discovered for this session; continuing without them.\n"
                    .to_string(),
            ),
        )
        .await;
        return Ok(false);
    }
```

FIND:
```rust
    let (mcp_state, mcp_statuses) = setup_session_mcp_tools(session_id, mcp_servers, event_sender.clone()).await;
    if mcp_state.tools.is_empty() {
        send_cognitive_event(
            &event_sender,
            CognitiveEvent::TokenChunk(
                "No MCP tools were discovered for this session; continuing without them.\n"
                    .to_string(),
            ),
        )
        .await;
        return Ok(false);
    }
```

REPLACE WITH:
```rust
    let (mcp_state, mcp_statuses) = setup_session_mcp_tools(session_id, mcp_servers, event_sender.clone()).await;

    // Always emit structured MCP status notification (for IDE clients).
    if !mcp_statuses.is_empty() {
        send_cognitive_event(
            &event_sender,
            CognitiveEvent::McpStatus(mcp_statuses),
        )
        .await;
    }

    if mcp_state.tools.is_empty() {
        send_cognitive_event(
            &event_sender,
            CognitiveEvent::TokenChunk(
                "No MCP tools were discovered for this session; continuing without them.\n"
                    .to_string(),
            ),
        )
        .await;
        return Ok(false);
    }
```

**Note:** The text fallback ("No MCP tools were discovered...") is kept for backward
compatibility with non-IDE consumers. IDE clients parse the structured `McpStatus` notification.

## Change 3: Add McpStatusUpdate variant to SessionUpdate enum

**File:** `crates/roko-acp/src/types.rs`

The SessionUpdate enum (lines 401-480) currently ends with SessionInfoUpdate. Add the new
variant before the closing brace.

FIND (lines 472-480):
```rust
    /// Session metadata update.
    SessionInfoUpdate {
        /// Session identifier.
        session_id: String,
        /// Optional session name.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        session_name: Option<String>,
    },
}
```

REPLACE WITH:
```rust
    /// Session metadata update.
    SessionInfoUpdate {
        /// Session identifier.
        session_id: String,
        /// Optional session name.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        session_name: Option<String>,
    },
    /// MCP server discovery results.
    McpStatusUpdate {
        /// Per-server status after initialization attempt.
        servers: Vec<McpServerStatus>,
    },
}
```

## Change 4: Handle McpStatus in map_event_to_update

**File:** `crates/roko-acp/src/bridge_events.rs`

The map_event_to_update function is at lines 3315-3354. The terminal events are the last
match arm. Add the McpStatus arm before the terminal events.

FIND (lines 3347-3353):
```rust
        CognitiveEvent::PlanUpdate { entries } => SessionUpdate::Plan { entries },
        CognitiveEvent::Complete { .. }
        | CognitiveEvent::Failure { .. }
        | CognitiveEvent::MaxTokens => {
            unreachable!("terminal cognitive events are handled before update mapping")
        }
    }
```

REPLACE WITH:
```rust
        CognitiveEvent::PlanUpdate { entries } => SessionUpdate::Plan { entries },
        CognitiveEvent::McpStatus(servers) => SessionUpdate::McpStatusUpdate { servers },
        CognitiveEvent::Complete { .. }
        | CognitiveEvent::Failure { .. }
        | CognitiveEvent::MaxTokens => {
            unreachable!("terminal cognitive events are handled before update mapping")
        }
    }
```

## Wire Format

After these changes, the IDE will receive a `session/update` notification like:

```json
{
  "jsonrpc": "2.0",
  "method": "session/update",
  "params": {
    "sessionId": "sess_abc123",
    "update": {
      "sessionUpdate": "mcp_status_update",
      "servers": [
        {
          "name": "nunchi",
          "status": "spawn_failed",
          "error": "No such file or directory (os error 2)"
        },
        {
          "name": "filesystem",
          "status": "ready",
          "toolsCount": 5
        }
      ]
    }
  }
}
```

Note: `sessionUpdate` value is `"mcp_status_update"` because serde `rename_all = "snake_case"`
converts `McpStatusUpdate` to `mcp_status_update`.

## What NOT to Change

- Do NOT modify `setup_session_mcp_tools` — that's W2-B
- Do NOT add a `session/mcp_status` query method yet (defer to follow-up)
- Do NOT modify the text fallback behavior — keep it for backward compatibility
- Do NOT modify `SessionMcpRuntime` struct

## Verification

After Phase 2:
```bash
# Build should pass
cargo build -p roko-acp 2>&1 | head -20

# Integration test: send a session/new with nonexistent MCP binary, look for mcp_status_update
echo '{"jsonrpc":"2.0","method":"session/new","id":1,"params":{"mcpServers":[{"name":"bad","transport":{"type":"stdio","command":"/nonexistent","args":[]}}]}}' \
  | timeout 10 cargo run -p roko-cli -- acp --quiet --no-serve 2>/dev/null \
  | grep -c 'mcp_status_update'
# Should print "1" (one notification with the failure status)
```

## Estimated Effort

30 minutes. Four changes across 2 files, all mechanical.
