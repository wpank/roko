# 16: resource_link Content Block Crashes ACP

## Problem

When attaching a folder or file as context in Zed (via the file tree or @-mention), the ACP crashes with:
```
invalid params for 'session/prompt': unknown variant resource_link,
expected one of content, text, resource, diff
```

## Root Cause

Zed sends `resource_link` content blocks (ACP spec type for "reference to accessible resource without embedded content"), but roko's `ContentBlock` enum in `types.rs` only had `Text`, `Resource`, and `Diff`.

The ACP spec (agentclientprotocol.com) defines 5 content block types:
1. `text` — plain text
2. `image` — base64 image data
3. `resource` — embedded resource content
4. `resource_link` — reference to a resource (URI + name + metadata)
5. `audio` — audio data

Roko was missing `resource_link`, `image`, and `audio`.

## Fix Applied (2026-05-06)

Added `ResourceLink` and `Image` variants to `ContentBlock` in `types.rs`:

```rust
ResourceLink {
    uri: String,
    name: String,
    mime_type: Option<String>,
    title: Option<String>,
    description: Option<String>,
    size: Option<u64>,
},
Image {
    data: String,       // base64
    mime_type: String,   // e.g. "image/png"
},
```

Updated all match sites:
- `event_forward.rs:163` `summarize_content_block` — renders as `"link: {name} ({uri})"`
- `bridge_events.rs:3477` `extract_prompt_text` — renders as `"[resource: {name} ({uri})]"`
- `bridge_events.rs:3498` `extract_resource_uris` — includes `ResourceLink` URIs
- `bridge_events.rs:3560` `resolve_context_items` — resolves `ResourceLink` URIs as file content (same as `Resource`)

Also flipped `image: false` → `image: true` in `handler.rs:287` so Zed shows the image upload button.

## What Should Have Prevented This

1. **ACP spec compliance**: The `ContentBlock` enum should match the full ACP spec. Any new ACP content types should be added with `#[serde(other)]` fallback so unknown types don't crash deserialization.

2. **Graceful unknown handling**: Add a catch-all variant:
   ```rust
   #[serde(other)]
   Unknown,
   ```
   This prevents hard crashes when the ACP spec adds new types.

3. **Integration test with Zed**: Test sending prompts with resource_link, image, and audio blocks to verify they're at least accepted (even if not fully processed).

## Remaining Work

- `Image` blocks are accepted by the type system but not yet passed through to the LLM API — the `extract_prompt_text` function just renders `[attached image: {mime_type}]` as text. Need to wire image data through to the OpenAI/Anthropic API calls for actual vision support.
- `audio` type not added (low priority — no current use case).

## Files Modified

| File | Change |
|------|--------|
| `crates/roko-acp/src/types.rs:453` | Added `ResourceLink` and `Image` to `ContentBlock` |
| `crates/roko-acp/src/handler.rs:287` | `image: true` |
| `crates/roko-acp/src/event_forward.rs:163` | Handle new variants in `summarize_content_block` |
| `crates/roko-acp/src/bridge_events.rs` | Handle new variants in `extract_prompt_text`, `extract_resource_uris`, `resolve_context_items` |
