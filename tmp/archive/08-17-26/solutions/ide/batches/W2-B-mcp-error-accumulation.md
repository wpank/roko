# W2-B: Refactor setup_session_mcp_tools to Accumulate Errors

## Context

The `roko-acp` crate has a function `setup_session_mcp_tools` that spawns MCP servers for
a session. It has **6 places** where failures are silently swallowed with `continue`. The
function only returns successfully-initialized tools. Callers have no way to know which
servers failed or why.

This batch changes the function to accumulate per-server status and return it alongside
the tools. W2-A (the types) must be done first.

## Prerequisite

**W2-A must be completed first** — it adds `McpServerStatus` and `McpInitStatus` to types.rs.

## File Location

All changes in ONE file:
`/Users/will/dev/nunchi/roko/roko/crates/roko-acp/src/bridge_events.rs`

## Change 1: Update function signature and add status accumulator

FIND the function signature and opening (around line 1969):
```rust
async fn setup_session_mcp_tools(
    session_id: &str,
    mcp_servers: &[crate::types::McpServerConfig],
    event_sender: mpsc::Sender<CognitiveEvent>,
) -> SessionMcpRuntime {
    let mut tools = Vec::new();
    let mut handlers: HashMap<String, Arc<dyn ToolHandler>> = HashMap::new();
    let mut used_names = HashSet::new();
    let discovery_timeout = Duration::from_secs(DEFAULT_MCP_DISCOVERY_TIMEOUT_SECS);
```

REPLACE WITH:
```rust
async fn setup_session_mcp_tools(
    session_id: &str,
    mcp_servers: &[crate::types::McpServerConfig],
    event_sender: mpsc::Sender<CognitiveEvent>,
) -> (SessionMcpRuntime, Vec<crate::types::McpServerStatus>) {
    let mut tools = Vec::new();
    let mut handlers: HashMap<String, Arc<dyn ToolHandler>> = HashMap::new();
    let mut used_names = HashSet::new();
    let mut statuses: Vec<crate::types::McpServerStatus> = Vec::new();
    let discovery_timeout = Duration::from_secs(DEFAULT_MCP_DISCOVERY_TIMEOUT_SECS);
```

## Change 2: Site 1 — HTTP transport unsupported

FIND:
```rust
            crate::types::McpTransport::Http { url } => {
                warn!(
                    session_id,
                    server = %server.name,
                    url = %url,
                    "skipping session MCP server with unsupported HTTP transport"
                );
                continue;
            }
```

REPLACE WITH:
```rust
            crate::types::McpTransport::Http { url } => {
                warn!(
                    session_id,
                    server = %server.name,
                    url = %url,
                    "skipping session MCP server with unsupported HTTP transport"
                );
                statuses.push(crate::types::McpServerStatus::failed(
                    server.name.clone(),
                    crate::types::McpInitStatus::TransportUnsupported,
                    format!("HTTP transport not supported for session MCP (url: {url})"),
                ));
                continue;
            }
```

## Change 3: Site 2 — spawn failure

FIND:
```rust
        let transport = match McpStdioTransport::spawn(command, args) {
            Ok(transport) => transport,
            Err(error) => {
                warn!(
                    session_id,
                    server = %server.name,
                    error = %error,
                    "failed to spawn session MCP server"
                );
                continue;
            }
        };
```

REPLACE WITH:
```rust
        let transport = match McpStdioTransport::spawn(command, args) {
            Ok(transport) => transport,
            Err(error) => {
                warn!(
                    session_id,
                    server = %server.name,
                    error = %error,
                    "failed to spawn session MCP server"
                );
                statuses.push(crate::types::McpServerStatus::failed(
                    server.name.clone(),
                    crate::types::McpInitStatus::SpawnFailed,
                    &error,
                ));
                continue;
            }
        };
```

## Change 4: Site 3 — initialize failure

FIND:
```rust
            Ok(Err(error)) => {
                warn!(
                    session_id,
                    server = %server.name,
                    error = %error,
                    "session MCP initialize failed"
                );
                continue;
            }
```

REPLACE WITH:
```rust
            Ok(Err(error)) => {
                warn!(
                    session_id,
                    server = %server.name,
                    error = %error,
                    "session MCP initialize failed"
                );
                statuses.push(crate::types::McpServerStatus::failed(
                    server.name.clone(),
                    crate::types::McpInitStatus::InitializeFailed,
                    &error,
                ));
                continue;
            }
```

## Change 5: Site 4 — initialize timeout

FIND:
```rust
            Err(_) => {
                warn!(
                    session_id,
                    server = %server.name,
                    timeout_secs = DEFAULT_MCP_DISCOVERY_TIMEOUT_SECS,
                    "session MCP initialize timed out"
                );
                continue;
            }
```

REPLACE WITH:
```rust
            Err(_) => {
                warn!(
                    session_id,
                    server = %server.name,
                    timeout_secs = DEFAULT_MCP_DISCOVERY_TIMEOUT_SECS,
                    "session MCP initialize timed out"
                );
                statuses.push(crate::types::McpServerStatus::failed(
                    server.name.clone(),
                    crate::types::McpInitStatus::InitializeTimeout,
                    format!("initialize timed out after {}s", discovery_timeout.as_secs()),
                ));
                continue;
            }
```

## Change 6: Site 5 — tools/list failure

FIND:
```rust
            Ok(Err(error)) => {
                warn!(
                    session_id,
                    server = %server.name,
                    error = %error,
                    "session MCP tools/list failed"
                );
                continue;
            }
```

REPLACE WITH:
```rust
            Ok(Err(error)) => {
                warn!(
                    session_id,
                    server = %server.name,
                    error = %error,
                    "session MCP tools/list failed"
                );
                statuses.push(crate::types::McpServerStatus::failed(
                    server.name.clone(),
                    crate::types::McpInitStatus::ToolsListFailed,
                    &error,
                ));
                continue;
            }
```

## Change 7: Site 6 — tools/list timeout

FIND:
```rust
            Err(_) => {
                warn!(
                    session_id,
                    server = %server.name,
                    timeout_secs = DEFAULT_MCP_DISCOVERY_TIMEOUT_SECS,
                    "session MCP tools/list timed out"
                );
                continue;
            }
```

REPLACE WITH:
```rust
            Err(_) => {
                warn!(
                    session_id,
                    server = %server.name,
                    timeout_secs = DEFAULT_MCP_DISCOVERY_TIMEOUT_SECS,
                    "session MCP tools/list timed out"
                );
                statuses.push(crate::types::McpServerStatus::failed(
                    server.name.clone(),
                    crate::types::McpInitStatus::ToolsListTimeout,
                    format!("tools/list timed out after {}s", discovery_timeout.as_secs()),
                ));
                continue;
            }
```

## Change 8: Add success status after tool collection

After the `info!` log and the `for tool in listed {` loop (which adds tools and handlers),
just BEFORE the closing `}` of the outer `for server in mcp_servers {` loop, add:

FIND the line after the inner tool loop ends (after all tools from one server are registered).
Look for the closing `}` of the `for tool in listed {` block. After it, add:

```rust
        statuses.push(crate::types::McpServerStatus::ready(
            server.name.clone(),
            listed.len(),
        ));
```

(Note: `listed` is still in scope here since the `for tool in listed` loop borrows it.)

Actually, `listed` is consumed by the `for tool in listed` loop. Instead, count before:

Before the `for tool in listed {` loop, add:
```rust
        let tool_count = listed.len();
```

Then after the `for tool in listed {` loop ends:
```rust
        statuses.push(crate::types::McpServerStatus::ready(
            server.name.clone(),
            tool_count,
        ));
```

## Change 9: Update return value

FIND (end of function):
```rust
    SessionMcpRuntime { tools, handlers }
}
```

REPLACE WITH:
```rust
    (SessionMcpRuntime { tools, handlers }, statuses)
}
```

## Change 10: Fix the caller

In `run_openai_compat_mcp_tool_loop` (around line 1810), FIND:
```rust
    let mcp_state = setup_session_mcp_tools(session_id, mcp_servers, event_sender.clone()).await;
```

REPLACE WITH:
```rust
    let (mcp_state, _mcp_statuses) = setup_session_mcp_tools(session_id, mcp_servers, event_sender.clone()).await;
```

(The `_mcp_statuses` will be used by W2-C to emit notifications. The underscore prevents
unused-variable warnings until W2-C is applied.)

## What NOT to Change

- Do NOT modify the "No MCP tools" text message — that's W2-C
- Do NOT add new CognitiveEvent variants — that's W2-C
- Do NOT change handler.rs — that's W2-C
- Keep all existing `warn!` calls — they're useful for server-side logs

## Verification

After Phase 2, run:
```bash
cargo build -p roko-acp 2>&1 | head -20
# Must compile without errors
```

## Estimated Effort

25-35 minutes. Mechanical insertion at 6 sites + return type change.
