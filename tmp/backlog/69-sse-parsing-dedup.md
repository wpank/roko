# 69 — Deduplicate SSE Parsing Across Provider Adapters

**Priority**: P2 — maintainability: four independent SSE parsers with behavioral divergences cause bugs and make adding new providers error-prone
**Size**: S (1 day)
**Crates**: `crates/roko-agent/src/` (roko-agent crate only)
**Depends on**: None

---

## Background

Server-Sent Events (SSE) is a streaming protocol where each line is prefixed with `data:` and the stream ends with a `[DONE]` sentinel. The provider adapters in `roko-agent` each parse these lines independently. This duplication has already caused behavioral divergences: Gemini uses a literal `"data: "` (with trailing space) while all other parsers use `"data:"` followed by `trim_start()`, meaning Gemini silently drops any line emitted as `data:{"key":"value"}` (no space after colon) that other parsers accept correctly.

The idiomatic fix is a two-level abstraction: a low-level `strip_sse_frame()` function handles the raw line framing (prefix, `[DONE]`, comment lines, empty lines), and the existing `parse_sse_line()` handles schema-specific field extraction for OpenAI-compatible responses. Both levels are placed in the shared `streaming.rs` module so every backend calls the same code. The result is a single deserialization path per line instead of the current double-parse.

---

## Current State

Four independent SSE parsing implementations exist in `crates/roko-agent/src/`:

1. **`streaming.rs` lines 307–358**: `parse_sse_line(line: &str) -> Option<StreamChunk>` — the canonical OpenAI-compatible parser. Strips `"data:"` then calls `.trim_start()`, checks for `[DONE]`, deserializes JSON, and extracts `choices/0/delta` content, reasoning, and tool-call deltas. Also extracts `usage` and `finish_reason`. Returns `None` for lines it cannot parse.

2. **`openai_compat_backend.rs` lines 403–476**:
   - `push_stream_line()` (line 403): calls `parse_sse_line()` from `streaming.rs` — correct.
   - `capture_stream_metadata()` (line 419): independently re-strips `"data:"`, re-checks `[DONE]`, re-deserializes JSON to extract `id`, `session_id`, `thread_id`. This is a **double parse** of every SSE line.
   - `stream_response_to_json()` (lines 444–476): assembles a JSON response from a `ChatResponse` + `StreamResponseMetadata`.

3. **`cursor_agent.rs` lines 329–411**:
   - `capture_stream_metadata()` (line 329): character-identical to `openai_compat_backend.rs:419`. Same double-parse.
   - `push_stream_line()` (line 354): calls `parse_sse_line()` then also emits a `tracing::warn!` for malformed Cursor SSE frames.
   - `stream_response_to_json()` (lines 379–411): verbatim copy of `openai_compat_backend.rs:444`.

4. **`tool_loop/backends/gemini_native.rs` lines 322–413**: inline SSE parser inside `stream_turn()`. Strips `"data: "` (with trailing space — diverges from others). Checks `line.starts_with(':')` for SSE comments (others do not handle comments). Reads `candidates/0/finishReason` and `candidates/0/content/parts` (Gemini-specific schema). Does not call `parse_sse_line()` at all.

**Verified divergences** (from reading the source):
- Gemini: `strip_prefix("data: ")` vs. others: `strip_prefix("data:").then trim_start()`
- `openai_compat_backend.rs` and `cursor_agent.rs` each deserialize every line twice
- `stream_response_to_json()` is identical in two files (lines 444 and 379 respectively)
- `capture_stream_metadata()` is nearly identical in two files (lines 419 and 329 respectively)
- Gemini is the only parser that handles SSE comment lines (`:` prefix) and empty lines explicitly; others silently ignore or drop them

---

## Implementation Plan

### Step 1: Add `SseFrame`, `strip_sse_frame()`, and `SseMetadata` to `streaming.rs`

In `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/streaming.rs`, add these types and function **above** the existing `parse_sse_line()`:

```rust
/// Metadata fields extracted from an SSE JSON payload.
#[derive(Debug, Clone, Default)]
pub struct SseMetadata {
    pub response_id: Option<String>,
    pub session_id: Option<String>,
    pub thread_id: Option<String>,
}

/// The framing classification of a raw SSE line.
pub enum SseFrame {
    /// A `data:` line whose payload was extracted and deserialized.
    Json(serde_json::Value, SseMetadata),
    /// The `[DONE]` terminal marker.
    Done,
    /// Comment line (starts with `:`), empty line, or non-`data:` line — skip.
    Skip,
}

/// Strip SSE framing from a raw line and return the parsed payload.
///
/// Normalizes whitespace: strips `data:` prefix then calls `trim_start()` on
/// the remainder, so both `data:{"k":1}` and `data: {"k":1}` are handled
/// identically. Returns `SseFrame::Skip` for empty lines, comment lines
/// (starting with `:`), and lines that do not start with `data:`.
pub fn strip_sse_frame(line: &str) -> SseFrame {
    // Empty lines and SSE comment lines (spec section 6.2).
    let trimmed = line.trim_end_matches(['\r', '\n']);
    if trimmed.is_empty() || trimmed.starts_with(':') {
        return SseFrame::Skip;
    }

    let Some(data) = trimmed.strip_prefix("data:").map(str::trim_start) else {
        return SseFrame::Skip;
    };

    if data == "[DONE]" {
        return SseFrame::Done;
    }

    let Ok(json) = serde_json::from_str::<serde_json::Value>(data) else {
        return SseFrame::Skip;
    };

    // Extract response-level metadata in a single pass.
    let metadata = SseMetadata {
        response_id: json.get("id").and_then(|v| v.as_str()).map(str::to_string),
        session_id: json
            .get("session_id")
            .and_then(|v| v.as_str())
            .map(str::to_string),
        thread_id: json
            .get("thread_id")
            .and_then(|v| v.as_str())
            .map(str::to_string),
    };

    SseFrame::Json(json, metadata)
}
```

Then rewrite `parse_sse_line()` to delegate to `strip_sse_frame()`:
```rust
pub fn parse_sse_line(line: &str) -> Option<StreamChunk> {
    match strip_sse_frame(line) {
        SseFrame::Done => Some(StreamChunk::Done(FinishReason::Stop)),
        SseFrame::Json(json, _) => parse_stream_chunk_from_json(&json),
        SseFrame::Skip => None,
    }
}

/// Extract a `StreamChunk` from a pre-deserialized JSON payload.
/// Factored out so both `parse_sse_line` and Gemini can call it.
fn parse_stream_chunk_from_json(json: &serde_json::Value) -> Option<StreamChunk> {
    let delta = json.pointer("/choices/0/delta").unwrap_or(&serde_json::Value::Null);
    // ... identical logic to current parse_sse_line body ...
}
```

### Step 2: Move `stream_response_to_json()` into `streaming.rs`

Cut `stream_response_to_json()` from `openai_compat_backend.rs` (lines 444–476) and from `cursor_agent.rs` (lines 379–411), and place a single canonical copy in `streaming.rs` as a `pub fn`. Both files then call `crate::streaming::stream_response_to_json(response, metadata)`.

The function signature is:
```rust
pub fn stream_response_to_json(
    response: crate::chat_types::ChatResponse,
    metadata: SseMetadata,
) -> Result<serde_json::Value, crate::error::LlmError>
```

### Step 3: Replace `capture_stream_metadata()` in both backends

In `openai_compat_backend.rs`, replace `capture_stream_metadata()` with a call to `strip_sse_frame()`:
```rust
// Before (double parse):
fn capture_stream_metadata(line: &[u8], metadata: &mut StreamResponseMetadata) {
    let line = String::from_utf8_lossy(line);
    let line = line.trim_end_matches(['\r', '\n']);
    let Some(line) = line.strip_prefix("data:").map(str::trim_start) else { return; };
    if line == "[DONE]" { return; }
    let Ok(json) = serde_json::from_str::<Value>(line) else { return; };
    // ...extract fields...
}

// After (single parse via strip_sse_frame):
fn capture_stream_metadata(line: &[u8], meta: &mut StreamResponseMetadata) {
    let line = String::from_utf8_lossy(line);
    if let SseFrame::Json(_, sse_meta) = strip_sse_frame(&line) {
        if let Some(id) = sse_meta.response_id { meta.response_id = Some(id); }
        if let Some(sid) = sse_meta.session_id { meta.session_id = Some(sid); }
        if let Some(tid) = sse_meta.thread_id { meta.thread_id = Some(tid); }
    }
}
```

Apply the same change to `cursor_agent.rs` (same function, same pattern).

### Step 4: Fix the Gemini prefix-stripping divergence

In `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/tool_loop/backends/gemini_native.rs`, the inline SSE parser (lines 322–413) currently uses:
```rust
let data = if let Some(rest) = line.strip_prefix("data: ") { rest } else { continue; };
```

Replace this with `strip_sse_frame()`:
```rust
let data_json = match strip_sse_frame(&line) {
    SseFrame::Json(json, _) => json,
    SseFrame::Done => {
        if !sent_done {
            sent_done = true;
            let _ = tx.send(Ok(StreamEvent::now(StreamEventKind::Done {
                finish_reason: "stop".to_string(),
            }))).await;
        }
        break;
    }
    SseFrame::Skip => continue,
};
```

Then use `data_json` directly instead of calling `serde_json::from_str(data)`. The Gemini-specific extraction of `candidates/0/finishReason` and `candidates/0/content/parts` follows unchanged, just operating on `data_json` instead of `data`.

Note: the comment-line handling (`line.starts_with(':')`) and empty-line handling that Gemini does explicitly are now handled inside `strip_sse_frame()` returning `SseFrame::Skip`, so those guards can be removed from the Gemini inline parser.

### Step 5: Update imports

In `openai_compat_backend.rs` and `cursor_agent.rs`, add:
```rust
use crate::streaming::{strip_sse_frame, stream_response_to_json, SseFrame, SseMetadata};
```

In `gemini_native.rs`:
```rust
use crate::streaming::{strip_sse_frame, SseFrame};
```

---

## Acceptance Criteria

1. `openai_compat_backend.rs` and `cursor_agent.rs` no longer contain their own `capture_stream_metadata()` functions. Both call `strip_sse_frame()` from `streaming.rs` to extract metadata.

2. `stream_response_to_json()` exists only in `streaming.rs`. Both `openai_compat_backend.rs` and `cursor_agent.rs` call `crate::streaming::stream_response_to_json()`.

3. Every SSE line is deserialized at most once (no double-parse). The JSON value from `strip_sse_frame()` is threaded into both event emission and metadata extraction in a single pass.

4. The Gemini native backend calls `strip_sse_frame()` for prefix stripping and `[DONE]` detection. It no longer uses `strip_prefix("data: ")` (with the literal trailing space).

5. `strip_sse_frame()` correctly handles:
   - `"data:{"key":"val"}"` (no space) — returns `SseFrame::Json`
   - `"data: {"key":"val"}"` (one space) — returns `SseFrame::Json`
   - `"data:  {"key":"val"}"` (two spaces) — returns `SseFrame::Json`
   - `"data:[DONE]"` and `"data: [DONE]"` — returns `SseFrame::Done`
   - `":heartbeat"` — returns `SseFrame::Skip`
   - `""` (empty line) — returns `SseFrame::Skip`

6. All existing tests pass: `cargo test -p roko-agent`.

7. SSE fixtures in `tests/fixtures/hermes/http/` (if any reference the SSE parser) continue to parse correctly through the shared code path.

---

## Verification Checklist

- [ ] `grep -rn "capture_stream_metadata" crates/roko-agent/src/` returns only one definition (in `streaming.rs`) and zero definitions in `openai_compat_backend.rs` or `cursor_agent.rs`
- [ ] `grep -rn "stream_response_to_json" crates/roko-agent/src/` returns one definition (in `streaming.rs`)
- [ ] `grep -rn "strip_prefix(\"data: \")" crates/roko-agent/src/` returns zero results (the literal-space form is eliminated)
- [ ] `cargo test -p roko-agent` passes with zero failures
- [ ] `cargo clippy -p roko-agent --no-deps -- -D warnings` is clean
- [ ] Run a live Gemini streaming request (or replay fixture) and confirm content is extracted correctly

---

## Files to Modify

| File | Change |
|---|---|
| `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/streaming.rs` | Add `SseFrame`, `SseMetadata`, `strip_sse_frame()`, `stream_response_to_json()`, refactor `parse_sse_line()` to delegate |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/openai_compat_backend.rs` | Replace `capture_stream_metadata()` with `strip_sse_frame()` call; delete `stream_response_to_json()`, call shared version |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/cursor_agent.rs` | Same as above |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/tool_loop/backends/gemini_native.rs` | Replace inline `strip_prefix("data: ")` with `strip_sse_frame()` call |

---

## Not in Scope

- Replacing hand-rolled SSE framing with an external crate (e.g., `reqwest-eventsource`, `eventsource-stream`). That is a larger dependency decision.
- Streaming support for the Anthropic Messages API: `ClaudeAgent` uses non-streaming JSON today and is unaffected.
- The demo-app TypeScript SSE parser (`demo/demo-app/src/transport/sse.ts`), which is a separate frontend concern.
- Claude CLI JSON-Lines parsing (`ClaudeCliParser`), which uses bare JSON per line without `data:` prefix.
