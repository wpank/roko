# 03 — Tool Output Visibility

## The Problem

When Claude uses tools (Read, Bash, Edit, etc.), it emits `"type": "tool"` events in
stream-json containing the full output. Mori captures these and shows them in a dedicated
panel. Roko was silently dropping them all.

---

## How Mori Handles Tool Output

### Event Flow
```
claude --output-format stream-json
  ↓ {"type":"tool", "tool":"Read", "content":"fn main() { ... }"}
  ↓
parse_claude_event()  [connection.rs:3212-3237]
  ↓ Extracts val.get("content") OR val.get("output")
  ↓ Truncates to 4096 bytes (char-boundary safe)
  ↓
AgentEvent::CommandOutput { role, instance, content }
  ↓
TUI event bus → command_output.rs widget
  ↓
Separate "Gate Output" panel with scrollbar
  - Pass/fail status colored (green for ok, red for errors)
  - Scrollable with viewport
```

### Codex Tool Output
```
JSON-RPC notification: "item/commandExecution/outputDelta"
  ↓ params.delta → AgentEvent::CommandOutput
```

### Cursor Tool Output
```
JSON-RPC notification: "tool_call_update" with status=completed
  ↓ update.rawOutput.content → AgentEvent::CommandOutput
```

All three backends emit the same `AgentEvent::CommandOutput` — the TUI doesn't care
which backend produced it.

---

## What Roko Was Doing (Before Fix)

### BackendResponse::extract_text() [translate/mod.rs:169-181]
```rust
Self::StreamJson(events) => {
    for ev in events {
        // ONLY looks for text deltas
        if let Some(delta) = ev.pointer("/delta/text") { ... }
        else if let Some(text) = ev.pointer("/content_block/text") { ... }
    }
    // Tool events silently dropped — no "type" check at all
}
```

### extract_clean_text() [chat.rs:430-484]
```rust
match event_type {
    Some("result") => { ... }
    Some("assistant") => { ... }
    _ => { /* generic fallback — Tool events fall here, content not extracted */ }
}
// No Some("tool") branch existed
```

### dispatch_claude_cli() [dispatch_direct.rs:75-104]
```rust
// Only extracted model and token usage from events
// No event type checking at all
if let Some(m) = event.pointer("/message/model") { model = m; }
if let Some(usage) = event.pointer("/message/usage") { /* tokens */ }
// Tool events completely ignored
```

---

## What Was Fixed (Partial)

### 1. dispatch_direct.rs — Tool event capture
`dispatch_claude_cli()` now has `match event_type` with:
- `Some("tool")` → extracts content/output, truncates to 4KB, stores in `tool_outputs: Vec<ToolOutput>`
- `Some("result")` → captures `session_id`, usage
- `_` → existing model/usage extraction

`DispatchResult` now carries:
- `tool_outputs: Vec<ToolOutput>` (tool_name + content)
- `session_id: Option<String>`

### 2. translate/mod.rs — StreamJson extraction
`BackendResponse::extract_text()` now includes Tool events inline as `[toolname]\ncontent`.
New methods:
- `extract_tool_outputs()` → `Vec<(Option<String>, String)>`
- `extract_session_id()` → `Option<String>`

### 3. chat.rs — JSONL parsing
`extract_clean_text()` now has `Some("tool")` branch that includes tool output inline.

### 4. chat_inline.rs — TUI rendering
`push_tool_outputs()` renders tool outputs above agent response:
```
  ⚙ Read  fn main() { ... } (+12 lines)
  ⚙ Bash  cargo build (+5 lines)
```

---

## What's Still Broken

### 1. Tool output only in chat direct path
Only `dispatch_direct.rs` → `dispatch_claude_cli()` captures tool outputs.
The other 5+ dispatch paths in run.rs don't capture them.

### 2. No separate panel
Mori shows tool outputs in a dedicated scrollable panel.
Roko shows them inline as one-line summaries before the response.
Full tool output content is not accessible (only first line shown).

### 3. Not in run.rs Claude CLI path
`run.rs:514-568` (Claude CLI subprocess) calls `spawn_agent` which doesn't
emit tool output events to the CLI. The stream-json parsing there is separate
from dispatch_direct.rs.

### 4. Not in orchestrate.rs
`orchestrate.rs:dispatch_agent_with()` dispatches through the full agent system
which has its own event handling. Tool outputs may be captured there but are not
surfaced to the TUI or episode records.

### 5. No tool output in episode records
Tool outputs are not persisted to `.roko/episodes.jsonl`.
Mori persists everything for replay.

### 6. HTTP response path
When chat goes through HTTP (sidecar or serve), the HTTP response is already
processed text. Tool outputs from the agent are lost in the HTTP layer.

---

## What Needs to Happen

1. **Unify stream-json parsing** into one function used by all Claude CLI paths
2. **Emit tool outputs as events** (like mori's `AgentEvent::CommandOutput`)
3. **Surface in all UIs**: inline chat, TUI dashboard, HTTP responses
4. **Persist to episodes** for replay and learning
5. **Support all backends**: not just Claude CLI, also Codex and OpenAI-compat tool calls
