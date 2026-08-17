# ACP Protocol Primer

## What is ACP?

Agent Client Protocol (ACP) is a JSON-RPC 2.0 protocol over stdio that lets AI agents communicate with editors (JetBrains, Zed, Neovim, VS Code). It's the standard way editors spawn and interact with coding agents.

## Transport

- **Newline-delimited JSON** over stdin/stdout
- Each message is a single line of JSON followed by `\n`
- Agent reads from stdin, writes to stdout
- All logging goes to a file (stdout is the protocol channel)

## Message Types

### Request (bidirectional)
```json
{"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {...}}
```

### Response
```json
{"jsonrpc": "2.0", "id": 1, "result": {...}}
```
or
```json
{"jsonrpc": "2.0", "id": 1, "error": {"code": -32600, "message": "..."}}
```

### Notification (no id, no response expected)
```json
{"jsonrpc": "2.0", "method": "session/update", "params": {...}}
```

## Lifecycle

1. **Initialize**: Client sends `initialize` → Agent responds with capabilities
2. **Session**: Client sends `session/new` → Agent responds with session ID + config options
3. **Prompt**: Client sends `session/prompt` → Agent streams `session/update` notifications → Agent sends final response
4. **Cancel**: Client sends `session/cancel` notification → Agent stops current prompt
5. **Config**: Client sends `session/config/update` → Agent responds with updated options

## Bidirectional Flow

The agent can also send requests TO the editor:
- `fs/read_text_file` — Read a file through the editor
- `fs/write_text_file` — Write a file through the editor
- `terminal/create` — Create a terminal session
- `terminal/output` — Get terminal output
- `terminal/wait_for_exit` — Wait for command completion
- `terminal/release` — Release terminal resources
- `session/request_permission` — Ask user to approve an action
- `elicitation/create` — Show a structured form

## Protocol Version

```rust
pub const ACP_PROTOCOL_VERSION: u32 = 1;
pub const ACP_SPEC_VERSION: &str = "0.12.2";
```

## Key Conventions

- All JSON field names use `camelCase`
- Session IDs are prefixed: `sess_` + UUID
- Tool call IDs are descriptive: `gate_compile_001`, `write_main_rs`
- The `session/update` notification is the workhorse — it carries all streaming data via the `sessionUpdate` discriminator field
