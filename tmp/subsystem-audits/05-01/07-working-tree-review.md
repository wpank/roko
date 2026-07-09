# 07 — Working Tree Review (wp-arch2 branch)

Review of the uncommitted changes (+654 / -218 lines across 9 files).

---

## CRITICAL: ContentBlock::Text renamed to `"content"` type tag

**File:** `crates/roko-acp/src/types.rs:361`

```rust
#[serde(rename = "content", alias = "text")]
Text { text: String },
```

**Before:** `ContentBlock::Text` serialized as `{ "type": "text", "text": "..." }`
**After:** Serializes as `{ "type": "content", "text": "..." }`

The `alias = "text"` allows deserializing old payloads but doesn't help serialization. Zed's ACP client reads the `"type"` field to determine what kind of content block it is. Changing this from `"text"` to `"content"` likely breaks all streamed message display.

**This is the most likely cause of "things don't work" in the screenshot.** The editor receives session updates but can't render them because the content block type is unrecognized.

The test at line 962 was updated to expect `"type": "content"`, which passes in isolation but doesn't match the Zed client's expectations.

---

## HIGH: `_history_context` computed but unused

**File:** `crates/roko-acp/src/bridge_events.rs:993`

```rust
let _history_context = if should_resolve_context {
    session.build_history_context_for_cli()
} else {
    String::new()
};
```

The old `run_claude_cognitive_task` prepended `history_context` and `file_context` to the prompt text for Claude CLI. The new `run_anthropic_cognitive_task` takes structured `messages` instead.

**Risk:** If `messages` doesn't include conversation history (only the current prompt), then history is silently dropped after this refactor. The variable was suppressed with `_` prefix to silence the unused warning, which hides the fact that history injection may be broken.

**Need to verify:** Check that `session.build_messages_for_api()` (or equivalent) includes prior turns from `session.conversation_history`.

---

## HIGH: New Anthropic API dispatch sends model key, not slug

The `run_anthropic_cognitive_task` receives `slug` from the model resolution, but it's the roko.toml `slug` field (e.g., `"claude-sonnet-4-20250514"`), which should be correct. However, if a user's roko.toml has `slug = "sonnet"` (short form), the Anthropic API will reject it.

**File:** `crates/roko-acp/src/bridge_events.rs` (working tree)

```rust
let mut body = serde_json::json!({
    "model": slug,  // Must be full Anthropic model ID
    ...
});
```

The model resolution at `resolved.slug` should expand short names, but if resolution fails, `FALLBACK_MODEL = "sonnet"` will be used directly as the API model ID, which is invalid.

---

## MEDIUM: Provider dropdown added but model filtering may be incomplete

**File:** `crates/roko-acp/src/session.rs:562+`

New `"provider"` config option added. When provider changes, models are re-filtered:

```rust
let model_belongs = roko_config
    .models
    .get(&self.config_state.model)
    .is_some_and(|p| p.provider == s);
if !model_belongs {
    // Pick first model matching the new provider
    self.config_state.model = first_key;
}
```

**Issue:** If no models match the new provider, `first_key` is `None` and the model stays unchanged. This leaves a model/provider mismatch that will cause dispatch failures.

---

## LOW: Temperament config option removed from UI

The `build_config_options` function was refactored to replace "Temperament" with "Workflow" in the config options list. Temperament was previously visible in the ACP settings panel. If any code still reads `state.temperament`, it will use the default "balanced" value but the user can no longer change it.

Not inherently broken, but worth noting that temperament is now a hidden default.
