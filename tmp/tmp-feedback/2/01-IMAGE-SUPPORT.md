# Image/Vision Support

## Problem

Dropping images into roko agents doesn't work anywhere:
- **ACP/Zed**: hardcodes `image: false` in PromptCapabilities → IDE blocks upload
- **CLI** (`roko chat`, `roko run`): no `--image <path>` flag
- **HTTP** (`roko serve`): no multipart POST for images

## What's Already Built

The entire backend plumbing exists:

### ACP Wire Types (`crates/roko-acp/src/types.rs:450-491`)
```rust
pub enum ContentBlock {
    Text { text: String },
    Image { data: String, mime_type: String },  // base64 + MIME — READY
    Resource { resource: ResourceRef },
    Diff { ... },
}
```

### Provider Format Converters (`crates/roko-acp/src/bridge_events.rs:4349-4405`)
- `build_openai_content_parts()` — ContentBlock::Image → OpenAI `image_url` data URIs
- `build_anthropic_content_parts()` — ContentBlock::Image → Anthropic base64 source blocks
- Message assembly at lines 1221-1233 replaces text with multipart when images present

### Core Chat Types (`crates/roko-core/src/chat_types.rs:54-80`)
```rust
pub enum ContentBlock {
    Text { text: String },
    ImageUrl { image_url: ImageUrl },  // URL or data:image/*;base64,...
}
```

### Model Vision Tracking
- `ModelProfile.supports_vision: bool` per model (`crates/roko-core/src/config/provider.rs:381`)
- `TaskRequirements.needs_vision: bool` for dispatch filtering (`crates/roko-core/src/agent.rs:415`)
- Scoring logic filters non-vision models (`agent.rs:447-448`)
- Claude Opus/Sonnet have `supports_vision: true` in registry

## What's Broken

### Single line blocks everything

`crates/roko-acp/src/handler.rs:286-290`:
```rust
prompt_capabilities: PromptCapabilities {
    image: false,        // ← HARDCODED FALSE
    audio: false,
    embedded_context: true,
},
```

IDE receives `image: false` during `initialize` → prevents image upload → shows
"This model does not support images yet" error in Zed.

## Fix Plan

### Phase 1: ACP (primary surface) — ~30 min

1. **Make `image` capability dynamic** (`handler.rs:286-290`):
   - During `initialize`, check if the default model has `supports_vision: true`
   - Set `PromptCapabilities.image` accordingly

2. **Update on model switch** (`session.rs`):
   - When `session/config/update` changes the model, re-evaluate vision support
   - Push updated capabilities via `server/config_sources_update` notification

3. **Verify message assembly** (`bridge_events.rs:1221-1233`):
   - The conversion code exists but may not be exercised — add test

### Phase 2: CLI — ~1 hr

1. **Add `--image <path>` to `roko chat` and `roko run`**:
   - Read file, detect MIME type, base64-encode
   - Wrap as `ContentBlock::Image { data, mime_type }`
   - Prepend to user message

2. **Files to modify**:
   - `crates/roko-cli/src/chat.rs` (add --image flag, read + encode)
   - `crates/roko-cli/src/run.rs` (same)
   - `crates/roko-cli/src/main.rs` (clap arg)

### Phase 3: HTTP API — ~1 hr

1. **Add multipart POST support to relevant routes** in `crates/roko-serve/`
2. **Or**: accept JSON with base64 `image` field in prompt body

## Models with Vision Support

| Model | supports_vision | Provider |
|-------|----------------|----------|
| claude-opus-4-6 | true | anthropic |
| claude-sonnet-4-6 | true | anthropic |
| gpt-4o | true | openai |
| gemini-2.5-pro | false (should be true) | gemini |
| gemini-2.5-flash | false (should be true) | gemini |
| gpt54-mini | unknown | openai |

**Note**: Gemini models have `supports_vision: false` in the builtin registry
(`crates/roko-core/src/config/registry.rs`) but actually support vision. Fix the
registry entries.
