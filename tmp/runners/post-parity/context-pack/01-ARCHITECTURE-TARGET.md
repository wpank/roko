# Post-Parity Architecture Target

## Goal

Make existing wired code actually work end-to-end. No new abstractions, no deep refactors.
Targeted wiring that fixes what users see and feel.

## Core Principle: One Shared Client

```
Process Startup
    │
    ▼
SharedHttpClient::new()          ← one reqwest::Client, one connection pool
    │
    ├──▶ ClaudeApiAgent          ← Arc<dyn HttpPoster>
    ├──▶ ClaudeCliAgent          ← (uses CLI subprocess, not HTTP)
    ├──▶ HealthCheck             ← Arc<dyn HttpPoster>
    ├──▶ roko-serve routes       ← Arc<dyn HttpPoster>
    └──▶ roko-agent-server       ← Arc<dyn HttpPoster>
```

## Core Principle: Chat Dispatch Chain

```
User Input
    │
    ▼
ChatAgentSession                 ← holds system_message, tools, history, model, effort
    │
    ▼
build_dispatch_request()         ← assembles all session state into one request
    │
    ▼
Adapter Layer                    ← ClaudeCliAgent / ClaudeApiAgent / ModelCallService
    │
    ├──▶ system prompt           ✓ included
    ├──▶ tools                   ✓ included
    ├──▶ message history         ✓ included
    ├──▶ model + effort          ✓ included
    └──▶ MCP config              ✓ included
    │
    ▼
StreamingState.append()          ← live token deltas wired to TUI
    │
    ▼
Response → session.history       ← assistant response appended
```

## Core Principle: Slash Commands Apply

```
/system "You are a Rust expert"
    │
    ▼
session.system_message = Some(...)   ← stored
    │
    ▼
Next dispatch includes system prompt ← APPLIED (currently broken)

/effort high
    │
    ▼
session.effort = Some("high")        ← stored
    │
    ▼
Next dispatch uses effort level      ← APPLIED (currently broken)

/gate enable compile
    │
    ▼
roko.toml updated                    ← WRITTEN (currently just prints)

/config set agent.model claude-sonnet
    │
    ▼
roko.toml updated                    ← WRITTEN (currently just prints)
```

## Key Structs

### ReqwestPoster (existing — make it shared)

```rust
// crates/roko-agent/src/http.rs
pub struct ReqwestPoster {
    client: reqwest::Client,  // ← THIS is the connection pool
    base_url: Option<String>,
}

// Usage: Arc::new(ReqwestPoster::new()) at startup, clone Arc everywhere
```

### ChatAgentSession (existing — make dispatch use it)

```rust
// crates/roko-cli/src/chat_session.rs
pub struct ChatAgentSession {
    pub model: Option<String>,
    pub provider: Option<String>,
    pub effort: Option<String>,
    pub system_message: Option<String>,
    pub tools: Vec<ToolSpec>,
    pub mcp_config: Option<PathBuf>,
    pub messages: Vec<ChatMessage>,
    pub session_id: Option<String>,
    // ...
}
```

### StreamingState (existing — call append())

```rust
// crates/roko-cli/src/inline/primitives/streaming.rs
pub struct StreamingState {
    pub buffer: String,
    pub token_count: usize,
    pub is_streaming: bool,
}

impl StreamingState {
    pub fn append(&mut self, text: &str) { ... }  // ← never called with live deltas
}
```
