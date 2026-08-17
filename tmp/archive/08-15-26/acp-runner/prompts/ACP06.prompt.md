# Batch ACP06 — Prompt handling + event streaming

## Goal

Implement the core bridge between Roko's cognitive pipeline and ACP session/update notifications. This is the central piece that streams agent output to the editor.

## Target files

- `crates/roko-acp/src/bridge_events.rs` — Event mapping and streaming

## Implementation details

### CognitiveEvent enum

Define the events that the cognitive loop produces:

```rust
pub enum CognitiveEvent {
    /// Agent text output chunk
    TokenChunk(String),
    /// Agent thinking/reasoning chunk
    ThinkingChunk(String),
    /// Tool call started
    ToolCallStart {
        tool_call_id: String,
        title: String,
        kind: ToolCallKind,
    },
    /// Tool call completed
    ToolCallComplete {
        tool_call_id: String,
        status: ToolCallStatus,
        content: Vec<ContentBlock>,
    },
    /// Gate started
    GateStarted {
        gate_name: String,
        tool_call_id: String,
    },
    /// Gate completed
    GateCompleted {
        gate_name: String,
        tool_call_id: String,
        passed: bool,
        summary: String,
        duration_ms: u64,
    },
    /// Phase transition in plan execution
    PhaseTransition {
        phase: String,
        entries: Vec<PlanEntry>,
    },
    /// Watcher triggered (from conductor)
    WatcherTriggered {
        watcher_name: String,
        action: String,
    },
    /// Prompt completed normally
    Complete {
        stop_reason: StopReason,
        usage: Option<UsageInfo>,
    },
    /// Hit token limit
    MaxTokens,
}
```

### Event streaming

```rust
/// Maps cognitive events to ACP session/update notifications and sends them
pub async fn stream_events_to_editor(
    transport: &mut StdioTransport,
    session_id: &str,
    mut events: tokio::sync::mpsc::Receiver<CognitiveEvent>,
    cancel_token: &CancelToken,
) -> Result<SessionPromptResult>
```

This function:
1. Reads events from the channel
2. Maps each to an ACP `SessionUpdate` variant
3. Sends via `transport.send_notification("session/update", ...)`
4. On `Complete` event, returns the `SessionPromptResult`
5. On cancellation, returns with `stop_reason: Cancelled`

### Mapping table

| CognitiveEvent | SessionUpdate |
|----------------|---------------|
| TokenChunk | AgentMessageChunk |
| ThinkingChunk | ThoughtMessageChunk |
| ToolCallStart | ToolCall (status: in_progress) |
| ToolCallComplete | ToolCallUpdate (status: completed/failed) |
| GateStarted | ToolCall (kind: other, status: in_progress) |
| GateCompleted | ToolCallUpdate (status: completed, markdown summary) |
| PhaseTransition | Plan (entries) |
| WatcherTriggered | ToolCall (kind: other) |
| Complete | (not a notification — ends the loop) |
| MaxTokens | (ends loop with stop_reason: MaxTokens) |

### handle_session_prompt

```rust
/// Handle a session/prompt request
pub async fn handle_session_prompt(
    transport: &mut StdioTransport,
    session: &mut AcpSession,
    params: SessionPromptParams,
) -> Result<SessionPromptResult>
```

This function:
1. Checks if session is busy (return SESSION_BUSY if so)
2. Sets session busy flag
3. Extracts prompt text from ContentBlocks
4. Creates an mpsc channel for events
5. Spawns the cognitive task (placeholder for now — just sends a TokenChunk with "Processing..." and Complete)
6. Calls `stream_events_to_editor` to stream events
7. Clears busy flag
8. Returns the result

## Verification

```bash
cargo check -p roko-acp
cargo clippy -p roko-acp --no-deps -- -D warnings
```

## Done when

- CognitiveEvent enum covers all event types
- stream_events_to_editor maps all events correctly
- handle_session_prompt handles busy check, cancellation
- Integration with transport.send_notification works
