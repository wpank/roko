# PAD_01: Consolidate stream-json parsers into single canonical module

## Task
Replace 4+ copies of Claude stream-json parsing with a single canonical parser in `roko-agent`.

## Runner Context
Runner PAD (Stream Parser Consolidation), batch 1 of 3. No dependencies.

## Problem
DP-1 anti-pattern: "Four parsers, four bugs." There are 4+ copies of stream-json parsing logic scattered across the codebase. When the Claude CLI output format changes, all copies need updating independently — a maintenance hazard.

## Current Parsers (VERIFIED locations)

1. **Canonical** — `crates/roko-agent/src/provider/claude_cli/stream.rs:127` — `parse_stream_line()`. Emits `AgentRuntimeEvent::MessageDelta`. Well-structured.
2. **Thin wrapper** — `crates/roko-cli/src/runner/agent_stream.rs:114-120` — Delegates to #1. Not a real copy; just re-exports. OK.
3. **Legacy inline** — `crates/roko-agent/src/claude_cli_agent.rs:519` — Inline `content_block_delta` matching inside the older `ClaudeCliAgent`. Separate parsing path.
4. **Translator** — `crates/roko-agent/src/translate/claude.rs:50-56,292` — Claude translator handles `content_block_delta` events. Third parsing path.
5. **ACP bridge** — `crates/roko-acp/src/bridge_events.rs:47-49` — Wire types mirroring the stream-json protocol for `claude_cli` fallback.
6. **dispatch_direct** — `crates/roko-cli/src/dispatch_direct.rs:74` — Spawns `claude --output-format stream-json` and extracts metadata.
7. **dispatch_v2** — `crates/roko-cli/src/dispatch_v2.rs:309,821,829` — Constructs `stream-json` args, produces `DispatchEvent::MessageDelta`.

## Exact Changes

### Step 1: Verify canonical parser covers all event types

Read `crates/roko-agent/src/provider/claude_cli/stream.rs` and confirm it handles:
- `content_block_delta` (text content)
- `message_start` / `message_stop` (lifecycle)
- `result` (final result with model metadata, usage)
- `tool_use` (tool call blocks)

If any are missing, add them to the canonical parser.

### Step 2: Make canonical parser output generic events

If the canonical parser emits `AgentRuntimeEvent`, ensure callers that need `DispatchEvent` or `CognitiveEvent` can map easily:

```rust
// In stream.rs, add a method that returns parsed events without coupling to AgentRuntimeEvent:
pub struct StreamEvent {
    pub kind: StreamEventKind,
    pub raw_json: serde_json::Value,
}

pub enum StreamEventKind {
    MessageStart { model: Option<String> },
    ContentDelta { text: String },
    ToolUse { id: String, name: String, input: serde_json::Value },
    Result { model: String, usage: Option<TokenUsage> },
    MessageStop,
    Unknown,
}

pub fn parse_stream_line(line: &str) -> Option<StreamEvent> { ... }
```

### Step 3: Replace legacy inline parser (#3)

In `claude_cli_agent.rs:519`, replace inline `content_block_delta` matching with a call to the canonical parser:

```rust
// BEFORE:
// inline serde_json matching for content_block_delta
// AFTER:
use crate::provider::claude_cli::stream::parse_stream_line;
if let Some(event) = parse_stream_line(line) {
    match event.kind {
        StreamEventKind::ContentDelta { text } => { /* existing handling */ }
        _ => {}
    }
}
```

### Step 4: Replace translator parser (#4)

In `translate/claude.rs`, replace `content_block_delta` handling with canonical parser calls.

### Step 5: Update dispatch_direct and dispatch_v2 (#6, #7)

These construct stream-json args and parse output. Replace their inline parsing with canonical parser calls.

## Write Scope
- `crates/roko-agent/src/provider/claude_cli/stream.rs` (enhance if needed)
- `crates/roko-agent/src/claude_cli_agent.rs` (replace inline parser)
- `crates/roko-agent/src/translate/claude.rs` (replace inline parser)
- `crates/roko-cli/src/dispatch_direct.rs` (use canonical parser)
- `crates/roko-cli/src/dispatch_v2.rs` (use canonical parser)

## Read-Only Context
- `crates/roko-acp/src/bridge_events.rs` (ACP wire types — don't change, just note for future)


## Verify
```bash
cargo build --workspace 2>&1 | head -30
cargo test --workspace 2>&1 | tail -20
```
## Acceptance Criteria
- Single canonical `parse_stream_line()` in `roko-agent/src/provider/claude_cli/stream.rs`
- All CLI-based dispatch paths use the canonical parser
- `StreamEvent`/`StreamEventKind` types available for callers needing different output types
- No inline `content_block_delta` JSON matching outside the canonical module
- Existing behavior unchanged (same events emitted, same data extracted)

## Do NOT
- Change the ACP bridge_events.rs (it's wire types, not parsing)
- Remove the agent_stream.rs re-export (it's a thin wrapper, not a copy)
- Change the stream-json format itself
