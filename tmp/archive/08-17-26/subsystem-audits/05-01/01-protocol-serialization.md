# 01 — Protocol Serialization Issues

## CRITICAL: SessionUpdate double-nesting in `send_session_update`

**File:** `crates/roko-acp/src/bridge_events.rs:2945-2963`

```rust
async fn send_session_update<R, W>(
    transport: &mut StdioTransport<R, W>,
    session_id: &str,
    update: SessionUpdate,
) -> Result<()> {
    let update_value = serde_json::to_value(update)?;
    let params = serde_json::json!({
        "sessionId": session_id,
        "update": update_value,  // ← BUG: wraps under "update" key
    });
    transport.send_notification("session/update", params).await
}
```

**What Zed expects:**
```json
{
  "sessionId": "abc123",
  "sessionUpdate": "agent_message_chunk",
  "content": { "type": "text", "text": "hello" }
}
```

**What roko sends:**
```json
{
  "sessionId": "abc123",
  "update": {
    "sessionUpdate": "agent_message_chunk",
    "content": { "type": "text", "text": "hello" }
  }
}
```

`SessionUpdate` is already tagged with `#[serde(tag = "sessionUpdate")]` in `types.rs:400`, so `serde_json::to_value(update)` produces a flat object with `"sessionUpdate"` as the discriminant. But wrapping it under `"update"` key creates double-nesting that Zed can't parse.

**Fix:** Merge the session ID into the serialized update value:
```rust
let mut params = serde_json::to_value(update)?;
params["sessionId"] = serde_json::Value::String(session_id.to_string());
transport.send_notification("session/update", params).await
```

---

## CRITICAL: ContentBlock type tag renamed from `"text"` to `"content"`

**File:** `crates/roko-acp/src/types.rs:358-365`

**Current (working tree):**
```rust
#[serde(rename_all = "snake_case", tag = "type")]
pub enum ContentBlock {
    #[serde(rename = "content", alias = "text")]
    Text { text: String },
```

This change renames the wire format `"type"` tag from `"text"` to `"content"`. The `alias = "text"` allows *deserialization* of old `"text"` payloads, but *serialization* now always emits `"content"`.

**Wire impact:**
```json
// Before: { "type": "text", "text": "hello" }
// After:  { "type": "content", "text": "hello" }
```

Zed's ACP client almost certainly expects `"type": "text"`. This will cause all message chunks to be silently dropped or rejected.

**This is very likely the reason "things don't work" in the screenshot** — the editor receives updates but can't parse the content blocks because the type tag is wrong.

**Fix:** Revert the rename. If the ACP spec actually uses `"content"`, then Zed's client needs to be updated first:
```rust
#[serde(rename_all = "snake_case", tag = "type")]
pub enum ContentBlock {
    Text { text: String },  // Serializes as { "type": "text", "text": "..." }
```

---

## HIGH: `CerebrasApi` provider slug may not map to Anthropic API

**File:** `crates/roko-acp/src/bridge_events.rs:1200-1240`

The provider dispatch match arms route `ClaudeCli | AnthropicApi` to `run_anthropic_cognitive_task`, which calls `https://api.anthropic.com/v1/messages` with the model `slug` from roko.toml.

If a model profile has `provider = "anthropic"` but `slug = "cerebras-8b"`, the Anthropic API will reject the slug. The model slug must be a valid Anthropic model ID (e.g., `claude-sonnet-4-20250514`).

**Fix:** Validate that the slug is a valid Anthropic model ID when dispatching to the Anthropic API, or fall through to OpenAI-compat for non-Anthropic slugs.

---

## MEDIUM: Protocol version hardcoded with no negotiation

**File:** `crates/roko-acp/src/types.rs:4-9`

```rust
pub const ACP_PROTOCOL_VERSION: u32 = 1;
pub const ACP_SPEC_VERSION: &str = "0.12.2";
```

The `initialize` handler sends these values but doesn't check what the client supports. If Zed ships with ACP spec 0.13+ and makes breaking changes, roko will silently send incompatible payloads.
