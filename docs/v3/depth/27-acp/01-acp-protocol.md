# 27.01 -- ACP Protocol

> Depth file for [27-ACP.md](../../27-ACP.md).

---

## Wire Format

ACP uses JSON-RPC 2.0 over newline-delimited stdio. Every message is a single
JSON object terminated by `\n`.

### Message Envelope

```rust
pub enum JsonRpcMessage {
    Request(JsonRpcRequest),
    Response(JsonRpcResponse),
    Notification(JsonRpcNotification),
}
```

### Request

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "session/prompt",
  "params": { "sessionId": "abc-123", "prompt": "Fix the bug" }
}
```

### Response

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": { "status": "completed", "content": [...] }
}
```

### Error Response

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "error": {
    "code": -32000,
    "message": "session 'abc-123' was not found",
    "data": null
  }
}
```

### Notification (No Response Expected)

```json
{
  "jsonrpc": "2.0",
  "method": "session/cancel",
  "params": { "sessionId": "abc-123" }
}
```

## Protocol Constants

```rust
pub const ACP_PROTOCOL_VERSION: u32 = 1;
pub const ACP_SPEC_VERSION: &str = "0.12.2";
```

## Error Codes

| Code | Constant | Meaning |
|---|---|---|
| -32700 | `PARSE_ERROR` | JSON parse failure |
| -32600 | `INVALID_REQUEST` | Malformed JSON-RPC envelope |
| -32601 | `METHOD_NOT_FOUND` | Unknown method name |
| -32602 | `INVALID_PARAMS` | Bad or missing params for method |
| -32603 | `INTERNAL_ERROR` | Server internal error |
| -32000 | `SESSION_NOT_FOUND` | Session ID does not exist |
| -32001 | `SESSION_BUSY` | Session is already processing |
| -32002 | `SESSION_BUDGET_EXCEEDED` | USD budget exhausted |

## Request Methods

### `initialize`

Handshake. Returns agent capabilities, protocol version, config sources, and
startup warnings.

```
Client -> Server: initialize { protocolVersion: 1 }
Server -> Client: {
  protocolVersion: 1,
  agentCapabilities: {
    loadSession: true,
    promptCapabilities: { images: true, ... },
    mcpCapabilities: { http: true, sse: true }
  },
  agentInfo: { name: "roko", version: "0.1.0", title: "Roko" },
  configSources: ["global:/path", "project:/path"],
  configWarnings: []
}
```

### `session/new`

Create a new editing session. Returns session ID, available modes, config
options, and budget status.

### `session/list`

List all active and persisted sessions.

### `session/load`

Restore a previously persisted session by ID. Returns conversation history,
config state, and budget status.

### `session/resume`

Like `session/load` but also pushes slash commands and config option
notifications.

### `session/prompt`

Send a prompt to the active session. The server dispatches to the configured
LLM provider, runs the gate pipeline, and returns results.

### `session/close`

Close and clean up a session.

### `session/config/update` (alias: `session/set_config_option`)

Update a session configuration option (model, effort, sandbox level, etc.).

### `session/set_mode`

Switch the active editing mode (e.g., normal, plan, code-review).

## Notification Methods

### `session/cancel` (Client to Server)

Cancel the currently running prompt in a session. This is a notification (no
response). Sets the session's `CancelToken` to stop the active provider call.

### `session/update` (Server to Client)

Push session state updates to the IDE:

```json
{
  "method": "session/update",
  "params": {
    "sessionId": "abc-123",
    "update": {
      "sessionUpdate": "plan_entry_update",
      "planEntry": { "id": "task-1", "status": "running" }
    }
  }
}
```

Update types include:
- `available_commands_update` -- Slash command availability
- `config_option_update` -- Config option changes
- `plan_entry_update` -- Plan execution progress
- `tool_call_update` -- Tool invocation progress
- `file_change` -- File modification notification
- `cost_update` -- Running cost accumulation
- `content_block` -- Streaming content fragments

### `server/config_sources_update` (Server to Client)

Sent when a config file changes on disk and the effective source list differs:

```json
{
  "method": "server/config_sources_update",
  "params": { "configSources": ["global:/path", "project:/path"] }
}
```

## Transport Layer

The `StdioTransport` in `crates/roko-acp/src/transport.rs` handles the
newline-delimited JSON-RPC wire protocol:

```rust
pub struct StdioTransport<R = Stdin, W = Stdout> {
    reader: Arc<AsyncMutex<BufReader<R>>>,
    writer: Arc<AsyncMutex<W>>,
    next_id: Arc<AtomicU64>,
    pending_requests: Arc<Mutex<HashMap<u64, oneshot::Sender<JsonRpcResponse>>>>,
}
```

### Outbound Requests

The server can send requests to the client (e.g., permission prompts) via
`send_request()`, which registers a `oneshot::Sender` in the pending registry
and waits for the matching response. A `PendingRequestGuard` ensures the
registry entry is cleaned up if the request future is cancelled.

### Read/Write Methods

- `read_message()` -- Read one newline-delimited JSON-RPC message; returns
  `Ok(None)` at EOF
- `send_response(id, result)` -- Send success response
- `send_error(id, code, message)` -- Send error response
- `send_notification(method, params)` -- Send fire-and-forget notification
- `send_request(method, params)` -- Send request and await response
- `handle_incoming_response(response)` -- Route response to pending sender

All writes are serialized through the writer mutex and flushed immediately.

## Dispatch Loop

The main loop in `handler.rs` reads messages, dispatches to the appropriate
handler, and watches for config file changes:

```rust
loop {
    let message = transport.read_message().await?;
    match message {
        JsonRpcMessage::Request(request) => {
            if config_watcher.changed() {
                sessions.replace_roko_config(refreshed);
            }
            handle_request(transport, &mut sessions, request).await?;
        }
        JsonRpcMessage::Response(response) => {
            transport.handle_incoming_response(response);
        }
        JsonRpcMessage::Notification(notification) => {
            handle_notification(&mut sessions, notification);
        }
    }
}
```

Startup warnings (config parse errors, missing provider credentials) are
collected and returned in the `initialize` response.

## Source

- `crates/roko-acp/src/types.rs` -- Protocol types and constants
- `crates/roko-acp/src/transport.rs` -- Stdio JSON-RPC transport
- `crates/roko-acp/src/handler.rs` -- Main dispatch loop
