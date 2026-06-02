# Fix: Wire ACP Session Updates to Serve SSE/WebSocket

## Summary

ACP (Agent Communication Protocol) sessions run as stdio subprocesses communicating
with editors. Their `CognitiveEvent` stream (token chunks, tool calls, completions)
never reaches the serve event bus. This doc describes how to bridge them.

## Current Architecture

```
Editor (VS Code / Cursor)
    │ stdio (JSON-RPC)
    ▼
roko-acp process
    │ CognitiveEvent (internal)
    │   → TokenChunk, ThinkingChunk, ToolCallStart,
    │     ToolCallComplete, PlanUpdate, Complete, Failure
    │
    └──→ session/update notification → back to editor via stdio
         (NEVER reaches roko-serve)
```

### What roko-acp emits (bridge_events.rs)

```rust
pub enum CognitiveEvent {
    TokenChunk(String),
    ThinkingChunk(String),
    ToolCallStart { tool_call_id, title, kind, locations },
    ToolCallComplete { tool_call_id, status, content },
    PlanUpdate { entries },
    Complete { stop_reason, usage },
    Failure { message },
    MaxTokens,
}
```

### What roko-serve expects

```rust
// events.rs — relevant variants
ServerEvent::AgentOutput { agent_id, run_id, content, done, metadata }
ServerEvent::AgentTrace { agent_id, run_id, content, tool_calls, reasoning, usage, done }
ServerEvent::InferenceStarted { request_id, model, agent_id, auto_routed }
ServerEvent::InferenceCompleted { request_id, model, agent_id, input_tokens, output_tokens, cost_usd, duration_ms }
```

## Proposed Solution: HTTP Event Sink in ACP

### Option A: Direct HTTP POST (Recommended)

Add an optional HTTP event sink to roko-acp that POSTs events to the serve ingest
endpoint when `ROKO_SERVE_URL` is set.

```rust
// New file: crates/roko-acp/src/event_sink.rs

/// Optional HTTP sink for forwarding ACP events to roko-serve.
/// Activated when ROKO_SERVE_URL env var is set.
pub struct HttpEventSink {
    client: reqwest::Client,
    base_url: String,
    agent_id: String,
    session_id: String,
}

impl HttpEventSink {
    pub fn from_env(agent_id: &str, session_id: &str) -> Option<Self> {
        let url = std::env::var("ROKO_SERVE_URL").ok()?;
        Some(Self {
            client: reqwest::Client::new(),
            base_url: url,
            agent_id: agent_id.to_string(),
            session_id: session_id.to_string(),
        })
    }

    pub async fn emit(&self, event: &CognitiveEvent) {
        let server_event = self.map_to_server_event(event);
        // Fire-and-forget POST, don't block the ACP main loop
        let _ = self.client
            .post(format!("{}/api/events/ingest", self.base_url))
            .json(&server_event)
            .send()
            .await;
    }

    fn map_to_server_event(&self, event: &CognitiveEvent) -> serde_json::Value {
        match event {
            CognitiveEvent::TokenChunk(text) => json!({
                "type": "agent_output",
                "agentId": self.agent_id,
                "runId": self.session_id,
                "content": text,
                "done": false,
            }),
            CognitiveEvent::Complete { stop_reason, usage } => json!({
                "type": "agent_output",
                "agentId": self.agent_id,
                "runId": self.session_id,
                "content": "",
                "done": true,
                "metadata": { "stop_reason": stop_reason, "usage": usage },
            }),
            CognitiveEvent::ToolCallStart { tool_call_id, title, kind, .. } => json!({
                "type": "agent_trace",
                "agentId": self.agent_id,
                "runId": self.session_id,
                "content": format!("Tool call: {} ({})", title, kind),
                "toolCalls": [{ "id": tool_call_id, "name": title, "status": "started" }],
                "done": false,
            }),
            CognitiveEvent::ToolCallComplete { tool_call_id, status, content } => json!({
                "type": "agent_trace",
                "agentId": self.agent_id,
                "runId": self.session_id,
                "content": content,
                "toolCalls": [{ "id": tool_call_id, "status": status }],
                "done": false,
            }),
            CognitiveEvent::Failure { message } => json!({
                "type": "error",
                "message": format!("ACP session {}: {}", self.session_id, message),
            }),
            _ => json!({ "type": "agent_output", "agentId": self.agent_id, "content": "", "done": false }),
        }
    }
}
```

### Option B: Shared memory / Unix socket

Heavier, but avoids HTTP overhead. Only warranted if ACP sessions produce >1000
events/sec, which they don't (Claude's token rate is ~100 tok/s max).

**Recommendation: Option A.** HTTP is simple, stateless, and matches the CLI sink pattern.

## Serve-Side Ingest Endpoint

Add a new route to roko-serve:

```rust
// routes/events.rs (or new ingest.rs)
/// POST /api/events/ingest — accepts ServerEvent JSON, publishes to bus
async fn ingest_event(
    State(state): State<AppState>,
    Json(event): Json<ServerEvent>,
) -> StatusCode {
    state.event_bus.publish(event);
    StatusCode::ACCEPTED
}
```

This is the universal event sink — used by ACP, CLI subprocesses, and PTY sessions.

## Wiring in ACP

In the ACP session runner (wherever `CognitiveEvent` is produced), add:

```rust
let sink = HttpEventSink::from_env(&agent_id, &session_id);

// In the event loop:
if let Some(sink) = &sink {
    sink.emit(&cognitive_event).await;
}
```

## Environment Variable Injection

When roko-serve spawns or manages ACP sessions, inject:
- `ROKO_SERVE_URL=http://127.0.0.1:6677`
- `ROKO_ACP_AGENT_ID={agent_id}` (for identification)

## Event Mapping Summary

| CognitiveEvent | ServerEvent |
|---------------|-------------|
| TokenChunk | AgentOutput (done=false) |
| ThinkingChunk | AgentTrace (reasoning field) |
| ToolCallStart | AgentTrace (tool_calls field) |
| ToolCallComplete | AgentTrace (tool_calls field) |
| PlanUpdate | AgentTrace (content = plan summary) |
| Complete | AgentOutput (done=true) + InferenceCompleted |
| Failure | Error |
| MaxTokens | Error |

## Verification

1. Start `roko serve`
2. Set `ROKO_SERVE_URL=http://127.0.0.1:6677` and start an ACP session
3. Send a prompt through ACP
4. Confirm SSE stream shows `agent_output` events with ACP session content
5. Confirm `agent_trace` events appear for tool calls
