# IDE Consumer Guide — ACP Protocol Reference

## Purpose

This document captures the correct ACP protocol usage as discovered through testing.
It serves as a reference for any client (IDE, CLI, or otherwise) consuming `roko acp`.

## Lifecycle

```
Client                          roko acp (stdio)
  │                                  │
  │─── session/new ─────────────────►│  (id: 1)
  │◄── result: {sessionId, config} ──│
  │◄── session/update: commands ─────│  (notification, no id)
  │                                  │
  │─── session/prompt ──────────────►│  (id: 2)
  │◄── session/update: chunk ────────│  (streaming text)
  │◄── session/update: chunk ────────│
  │◄── session/update: tool_call ────│  (if MCP tools used)
  │◄── session/update: chunk ────────│
  │◄── session/update: usage ────────│
  │◄── session/update: session_info ─│
  │◄── result: {stopReason} ─────────│  (id: 2, final response)
  │                                  │
  │─── [close stdin] ───────────────►│  (process exits cleanly)
```

## Methods

### session/new

Creates a new conversation session.

```json
{
  "jsonrpc": "2.0",
  "method": "session/new",
  "id": 1,
  "params": {
    "model": "sonnet",
    "mcpServers": [{
      "name": "nunchi",
      "transport": {
        "type": "stdio",
        "command": "/path/to/nunchi-mcp",
        "args": [],
        "env": {
          "BRIDGE_TOKEN": "...",
          "BRIDGE_URL": "http://127.0.0.1:6678"
        }
      }
    }]
  }
}
```

**IMPORTANT**: As of current roko, the `model` param is NOT used (see issue #02).
The session uses whatever `agent.model` resolves to in the config. To change model,
send `session/config/update` after session creation.

**Response:**
```json
{
  "id": 1,
  "result": {
    "sessionId": "sess_uuid-here",
    "configOptions": [...],
    "modes": { "availableModes": [...], "currentModeId": "code" }
  }
}
```

Followed by a notification:
```json
{
  "method": "session/update",
  "params": {
    "sessionId": "sess_...",
    "update": {
      "sessionUpdate": "available_commands_update",
      "availableCommands": [...]
    }
  }
}
```

### session/prompt

Sends a user message. This is the method for conversation turns.

```json
{
  "jsonrpc": "2.0",
  "method": "session/prompt",
  "id": 2,
  "params": {
    "sessionId": "sess_...",
    "prompt": [{"type": "text", "text": "Hello world"}]
  }
}
```

**NOT** `message/send` or `agent/message` — those return `-32601 method not supported`.

### session/config/update

Change a single config option (model, provider, effort, workflow, clippy, tests):

```json
{
  "jsonrpc": "2.0",
  "method": "session/config/update",
  "id": 3,
  "params": {
    "sessionId": "sess_...",
    "optionId": "model",
    "newValue": "haiku"
  }
}
```

**IMPORTANT**: This is a **flat struct**, NOT a batch `updates` array.
- Field: `optionId` (alias: `configId`) — the config option to change
- Field: `newValue` (alias: `value`) — the new value
- Alternative method name: `session/set_config_option`
- Unknown `optionId` values are silently accepted (no error)
- Invalid `newValue` for model is silently ignored (falls back to previous)

**Response**: Returns full updated `configOptions` array (same shape as session/new).

### session/list

List all active sessions:

```json
{"jsonrpc": "2.0", "method": "session/list", "id": 4, "params": {}}
```

### session/close

Close and remove a session (subsequent prompts will error):

```json
{"jsonrpc": "2.0", "method": "session/close", "id": 5, "params": {"sessionId": "sess_..."}}
```

### session/set_mode

Switch between code/plan/research modes:

```json
{"jsonrpc": "2.0", "method": "session/set_mode", "id": 6, "params": {"sessionId": "sess_...", "modeId": "plan"}}
```

### session/cancel (notification)

Cancel an in-progress prompt:

```json
{
  "jsonrpc": "2.0",
  "method": "session/cancel",
  "params": {"sessionId": "sess_..."}
}
```

## Streaming Update Types

All streaming data comes as `session/update` notifications with different `sessionUpdate` values:

| sessionUpdate | Content | Purpose |
|---------------|---------|---------|
| `agent_message_chunk` | `{content: {text: "...", type: "text"}}` | Streaming text from model |
| `tool_call` | `{title, toolCallId, status, content}` | MCP tool invocation |
| `usage_update` | `{size: number, used: number}` | Context window (e.g. size=128000, used=2900) |
| `session_info_update` | `{session_id, session_name}` | Session metadata |
| `available_commands_update` | `{availableCommands: [...]}` | Slash commands |

**Note**: Text chunks are in `content.text` (a content block), NOT a `delta` string.
Assemble the full response by concatenating all chunk `content.text` values.

## Error Handling

### Known error codes

| Code | Meaning | Example |
|------|---------|---------|
| -32601 | Method not found | Using wrong method name |
| -32602 | Invalid params | Missing required field |
| -32000 | Session not found | Wrong sessionId |

### Disconnect behavior

- Closing stdin causes the ACP process to exit cleanly
- No zombie processes or resource leaks observed
- In-flight LLM requests are abandoned (not cancelled upstream)

## Configuration

The config passed via `--config` must be version 2:

```toml
config_version = 2
schema_version = 2

[agent]
model = "sonnet"        # Must match a key in [models.*]
bare_mode = true        # Disables workspace features (PRDs, plans, etc.)
timeout_ms = 300000

[providers.openai]
kind = "openai_compat"
base_url = "https://api.openai.com/v1"
api_key_env = "OPENAI_API_KEY"

[models.sonnet]
provider = "openai"
slug = "gpt-4o"
supports_tools = true
context_window = 128000
max_output = 16000
```

### Critical config fields for IDE use

| Field | Purpose | Gotcha |
|-------|---------|--------|
| `agent.model` | Default model key | Must exist in [models.*] or defaults are random |
| `agent.bare_mode` | Disables workspace features | Set true for IDE — avoids PRD/plan/gate noise |
| `models.X.supports_tools` | Enable MCP tool use | Must be `true` for tool-calling models |
| `models.X.max_output` | Token limit | Default is None (falls back to 16,384). Set explicitly for clarity |
| `providers.X.api_key_env` | Env var name for API key | Loaded from process env or ~/.roko/.env |

## Multiple Sessions

ACP supports multiple concurrent sessions on one process:
- Each session has independent conversation history
- Sessions can use different models (via config/update)
- Prompting session 1 while session 2 exists works correctly
- Sessions are identified by UUID (sess_...)
