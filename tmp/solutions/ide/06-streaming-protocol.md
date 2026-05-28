# Streaming Protocol Details

## Overview

After `session/prompt` is sent, the response streams as multiple `session/update`
notifications before the final JSON-RPC result (with `id` matching the request).

## Message Flow (Real Capture)

Below is an actual response from `roko acp` when asking the model to say "HELLO_WORLD_42":

```
← {"jsonrpc":"2.0","method":"session/update","params":{"sessionId":"sess_3c0331b2-...","update":{"content":{"text":"HELLO_WORLD_42","type":"text"},"sessionUpdate":"agent_message_chunk"}}}
← {"jsonrpc":"2.0","method":"session/update","params":{"sessionId":"sess_3c0331b2-...","update":{"sessionUpdate":"usage_update","size":128000,"used":2917}}}
← (empty line)
← {"jsonrpc":"2.0","method":"session/update","params":{"sessionId":"sess_3c0331b2-...","update":{"sessionUpdate":"session_info_update","session_id":"sess_3c0331b2-...","session_name":"Say exactly: HELLO_WORLD_42"}}}
← {"jsonrpc":"2.0","id":2,"result":{"stopReason":"end_turn"}}
```

## Key Observations

### 1. Chunks may be word-level or full-text

With short responses, the entire text arrives in one chunk. With longer responses,
text is tokenized into word/subword chunks:

```json
{"update":{"content":{"text":"Here","type":"text"},"sessionUpdate":"agent_message_chunk"}}
{"update":{"content":{"text":" are","type":"text"},"sessionUpdate":"agent_message_chunk"}}
{"update":{"content":{"text":" the","type":"text"},"sessionUpdate":"agent_message_chunk"}}
```

### 2. Empty lines appear in the stream

There are occasionally empty lines (`\n`) between JSON messages. Clients must
skip empty lines when parsing.

### 3. Tool calls are interleaved with text

When the model calls an MCP tool:

```json
// Tool call starts
{"update":{"content":[],"kind":"other","sessionUpdate":"tool_call","status":"in_progress","title":"nunchi_tiles_list","toolCallId":"call_aCNN..."}}

// Tool result arrives
{"update":{"content":[{"text":"{\"tiles\":[...]}","type":"text"}],"kind":"other","sessionUpdate":"tool_call","status":"completed","title":"nunchi_tiles_list","toolCallId":"call_aCNN..."}}

// Model continues with text based on tool result
{"update":{"content":{"text":"Here are the available tools...","type":"text"},"sessionUpdate":"agent_message_chunk"}}
```

### 4. Usage update always precedes final result

The `usage_update` notification (showing context window consumption) always
arrives before the final `result` message. Clients can use this to update
their context budget display.

### 5. Session name is auto-generated

The `session_info_update` notification includes a `session_name` derived from
the first prompt text. This is sent once per prompt.

## Parsing Strategy for IDE Clients

```typescript
interface StreamLine {
  jsonrpc: "2.0";
  id?: number;              // Present only on final result/error
  method?: string;          // "session/update" for notifications
  result?: { stopReason: string };
  error?: { code: number; message: string };
  params?: {
    sessionId: string;
    update: SessionUpdate;
  };
}

type SessionUpdate =
  | { sessionUpdate: "agent_message_chunk"; content: { text: string; type: "text" } }
  | { sessionUpdate: "tool_call"; title: string; toolCallId: string; status: string; content: any[] }
  | { sessionUpdate: "usage_update"; size: number; used: number }
  | { sessionUpdate: "session_info_update"; session_id: string; session_name: string }
  | { sessionUpdate: "available_commands_update"; availableCommands: Command[] }
  | { sessionUpdate: "mcp_status"; servers: McpServerStatus[] }  // Proposed in issue #01

function processLine(raw: string) {
  if (!raw.trim()) return;  // Skip empty lines
  const msg: StreamLine = JSON.parse(raw);

  if (msg.id !== undefined) {
    // Final response — resolve the pending promise
    if (msg.error) handleError(msg.error);
    else handleComplete(msg.result);
    return;
  }

  // Notification
  const update = msg.params?.update;
  if (!update) return;

  switch (update.sessionUpdate) {
    case "agent_message_chunk":
      appendToCurrentMessage(update.content.text);
      break;
    case "tool_call":
      handleToolCall(update);
      break;
    case "usage_update":
      updateContextBudget(update.size, update.used);
      break;
    // ...
  }
}
```

## Stop Reasons

The final result includes a `stopReason`:

| Value | Meaning |
|-------|---------|
| `end_turn` | Model finished naturally |
| `max_tokens` | Hit output token limit |
| `cancelled` | User sent session/cancel |
| `tool_use` | (shouldn't appear in final — internal to tool loop) |

## Concurrency Notes

- Only one `session/prompt` can be in-flight per session at a time
- Sending a second prompt before the first completes is undefined behavior
- Use `session/cancel` to abort, wait for final result, then send next prompt
- Different sessions can have concurrent in-flight prompts
