# 14: Image Support for Roko Agents

## Status: Partially Fixed (2026-05-06)

### What Was Done

1. **`image: true` in ACP initialize** — `handler.rs:287` changed from `false` to `true`. Zed now shows the image upload button.

2. **`Image` variant added to `ContentBlock`** — `types.rs:453` now has:
   ```rust
   Image {
       data: String,       // base64
       mime_type: String,   // e.g. "image/png"
   },
   ```
   All match sites handle it (event_forward, bridge_events extract/resolve functions).

3. **`ResourceLink` also added** — needed for Zed folder/file attachments (see doc 16).

### What Still Needs Wiring

The `Image` block is accepted by deserialization but NOT passed through to the LLM API. Currently `extract_prompt_text` renders it as `[attached image: image/png]` text — the actual base64 data is discarded.

To complete image support:

#### For OpenAI-compatible backends

The `openai_compat` backend already has `ImageUrl` content blocks:
```rust
// provider/openai_compat.rs:
ImageUrl { image_url: ImageUrlBlock { url: String } },
```

Need to convert `ContentBlock::Image { data, mime_type }` → `ImageUrl`:
```rust
ImageUrl {
    image_url: ImageUrlBlock {
        url: format!("data:{mime_type};base64,{data}"),
    }
}
```

#### For Anthropic API

Anthropic uses a different format:
```json
{
    "type": "image",
    "source": {
        "type": "base64",
        "media_type": "image/png",
        "data": "..."
    }
}
```

#### Wiring location

In `bridge_events.rs`, where the prompt is assembled into model messages (the `build_messages_array` path), image blocks need to be converted from ACP format to the backend's format instead of being stringified.

### Graceful Fallback for Non-Vision Models

If the user sends an image but the model doesn't support vision:
- Option A: Error with "Switch to a vision model (gpt54-mini, opus, sonnet)"
- Option B: Auto-escalate to a vision-capable model for that request
- Option C: Extract text from image (OCR) and send as text

Model configs have `supports_vision` field — gpt54-mini, gpt55, opus all have `supports_vision = true`.

## Files Modified So Far

| File | Change | Status |
|------|--------|--------|
| `crates/roko-acp/src/handler.rs:287` | `image: true` | Done |
| `crates/roko-acp/src/types.rs:453` | `Image` variant in `ContentBlock` | Done |
| `crates/roko-acp/src/event_forward.rs` | Handle `Image` in summarize | Done |
| `crates/roko-acp/src/bridge_events.rs` | Handle `Image` in extract/resolve | Done |

## Files Still Needed

| File | Change |
|------|--------|
| `crates/roko-acp/src/bridge_events.rs` | Convert Image blocks to backend format in message assembly |
| `crates/roko-agent/src/openai_compat_backend.rs` | Verify ImageUrl passthrough works |
| `crates/roko-agent/src/provider/anthropic_api.rs` | Add image block for Anthropic format |

## Priority

High — Zed now shows the upload button but images are silently discarded. Users will try to paste screenshots and get no useful response.
