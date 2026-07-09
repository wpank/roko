# 04 — Streaming Protocol Parsing

## The Problem

Mori uses a typed enum with serde deserialization for Claude's stream-json events.
Roko uses ad-hoc JSON pointer lookups scattered across multiple files with no
type safety or completeness.

---

## Mori's Typed Approach

### ClaudeStreamEvent Enum
**File**: `/Users/will/dev/uniswap/bardo/apps/mori/src/agent/protocol.rs:186-196`

```rust
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClaudeStreamEvent {
    System(serde_json::Value),
    Assistant(ClaudeAssistantEvent),
    Tool(serde_json::Value),
    Result(ClaudeResultEvent),
    #[serde(other)]
    Unknown,
}
```

### Supporting Types

```rust
// protocol.rs:140-150
struct ClaudeAssistantEvent {
    message: ClaudeMessage,
}
struct ClaudeMessage {
    content: Vec<ClaudeContentBlock>,
    usage: Option<ClaudeUsage>,
}

// protocol.rs:152-165
enum ClaudeContentBlock {
    Text { text: String },
    ToolUse { id: String, name: String, input: Value },
    #[serde(other)] Unknown,
}

// protocol.rs:175-184
struct ClaudeResultEvent {
    subtype: String,        // "success" | "error"
    session_id: String,     // For --resume
    is_error: bool,
    num_turns: u64,
    total_cost_usd: Option<f64>,
    usage: Option<ClaudeUsage>,
}

// protocol.rs:167-173
struct ClaudeUsage {
    input_tokens: u64,
    output_tokens: u64,
    cache_creation_input_tokens: Option<u64>,
    cache_read_input_tokens: Option<u64>,
}
```

### Deserialization
**File**: `connection.rs:2674-2676`

```rust
match serde_json::from_str::<ClaudeStreamEvent>(&line) {
    Ok(event) => parse_claude_event(role, instance, event, &tx),
    Err(e) => { /* error event */ }
}
```

One `from_str` call. Serde handles the `"type"` tag routing. Each variant gets
exactly the fields it needs. Unknown event types are silently ignored via `#[serde(other)]`.

### Event Handler
**File**: `connection.rs:3141-3243`

```rust
fn parse_claude_event(role, instance, event, tx) {
    match event {
        System(_) => { /* no-op */ }
        Assistant(a) => {
            for block in a.message.content {
                match block {
                    Text { text } => tx.send(MessageDelta { text }),
                    ToolUse { name, .. } => tx.send(ToolCall { name }),
                    Unknown => {}
                }
            }
            if let Some(usage) = a.message.usage {
                tx.send(TokenUsage { ... })
            }
        }
        Tool(val) => {
            // content or output field → CommandOutput event
            // 4096 byte truncation
        }
        Result(r) => {
            if r.is_error { tx.send(Error { ... }) }
            tx.send(TokenUsage { cost_usd: r.total_cost_usd })
            tx.send(TurnCompleted { thread_id: Some(r.session_id) })
        }
        Unknown => {}
    }
}
```

---

## Roko's Ad-Hoc Approach

### BackendResponse::extract_text() [translate/mod.rs]
```rust
// Ad-hoc JSON pointer lookups, no type tag routing:
ev.pointer("/delta/text")              // text deltas
ev.pointer("/content_block/text")      // content blocks
ev.get("type") == Some("tool")        // tool events (recently added)
```

### extract_clean_text() [chat.rs:430-484]
```rust
// String matching on event_type:
let event_type = obj.get("type").and_then(Value::as_str);
match event_type {
    Some("result") => { obj.get("result").as_str() }
    Some("assistant") => { obj.pointer("/message/content").as_array() }
    Some("tool") => { obj.get("content").as_str() }  // recently added
    _ => { /* generic fallback */ }
}
```

### dispatch_claude_cli() [dispatch_direct.rs:81-160]
```rust
// Another separate parser, also string matching:
let event_type = event.get("type").and_then(Value::as_str);
match event_type {
    Some("tool") => { /* extract content */ }
    Some("result") => { /* extract session_id */ }
    _ => {
        event.pointer("/message/model")   // model
        event.pointer("/message/usage")   // tokens
    }
}
```

### Problems with This Approach

1. **Three separate parsers** for the same protocol in three different files
2. **No type safety** — any field could be missing, wrong type, or renamed
3. **Easy to miss events** — each parser handles different subsets
4. **No unknown event handling** — unknown types silently fall through
5. **No cache token tracking** — `cache_creation_input_tokens` and `cache_read_input_tokens` not extracted
6. **No content block type routing** — doesn't distinguish Text vs ToolUse content blocks in assistant events
7. **No cost from Result** — `total_cost_usd` not extracted (only token counts)

---

## What Needs to Change

### Option A: Port mori's typed enum to roko
Create `ClaudeStreamEvent` in `roko-agent/src/translate/claude.rs` with serde deserialization.
Single parse function. All Claude CLI paths use it.

### Option B: Shared stream-json parser function
Less ambitious: one function `parse_claude_stream_event(line: &str) -> ParsedEvent` that
all callers use, even if it returns an enum of extracted data rather than raw types.

Either way, the three separate parsers in translate/mod.rs, chat.rs, and dispatch_direct.rs
must be consolidated into one.
