# IDE Integration — Additional Findings (Round 2)

Discovered during extended testing on 2026-05-04. These are issues beyond the
original 01-11 documents, found via the test harness.

---

## Issue #8: config/update silently accepts unknown optionId

**Severity**: Low (DX issue)
**Test**: `test-config-options.sh` — "config/update with unknown optionId"

### Behavior

```json
// Request
{"method":"session/config/update","params":{"sessionId":"...","optionId":"nonexistent_option","newValue":"whatever"}}
// Response: success (no error!)
{"result":{"configOptions":[...]}}
```

The `update_config()` method in `session.rs` accepts any `option_id` string. If it
doesn't match a known option, nothing changes — but the IDE receives a success response
with unchanged config options, giving no signal that the update was a no-op.

### Impact

- IDE can't distinguish "option updated" from "option not found"
- Typos in option IDs silently fail
- No feedback loop for debugging

### Fix

In `session.rs:update_config()`, return an error if `option_id` doesn't match any
known config option:

```rust
fn update_config(&mut self, option_id: &str, new_value: &serde_json::Value, config: &RokoConfig) {
    match option_id {
        "model" | "provider" | "effort" | "workflow" | "clippy" | "tests" => { /* ... */ }
        _ => return Err(AcpError::invalid_params(
            format!("unknown config option: '{}'", option_id)
        )),
    }
}
```

---

## Issue #9: Invalid model value silently falls back

**Severity**: Low (DX issue)
**Test**: `test-session-lifecycle.sh` — "config/update with invalid model"

### Behavior

Setting `model` to a nonexistent value (e.g., `"nonexistent-xyz"`) returns success
and the model stays at its previous value (e.g., `"sonnet"`). No error, no warning.

### Impact

- IDE can't tell user "that model doesn't exist"
- Bad autocomplete selections fail silently

### Fix

Validate `new_value` against available models for the current provider before accepting.
Return error if no match found.

---

## Issue #10: config/update wire format undocumented for IDE

**Severity**: Medium (integration friction)

### Finding

The actual wire format for `session/config/update` is:

```json
{
  "jsonrpc": "2.0",
  "method": "session/config/update",
  "id": 5,
  "params": {
    "sessionId": "sess_...",
    "optionId": "model",     // NOT "id", NOT in an "updates" array
    "newValue": "haiku"      // NOT "value"
  }
}
```

This is a **flat struct** (`ConfigUpdateParams`), not a batch updates array. The struct
accepts aliases: `configId` (alias for `optionId`), `value` (alias for `newValue`).

The IDE code in `AgentChat.tsx` must use these exact field names. The serde struct is:

```rust
#[serde(rename_all = "camelCase")]
pub struct ConfigUpdateParams {
    pub session_id: String,
    #[serde(alias = "configId")]
    pub option_id: String,
    #[serde(alias = "value")]
    pub new_value: serde_json::Value,
}
```

### Alternative name: `session/set_config_option`

The handler also accepts `"session/set_config_option"` as a method name alias.

---

## Issue #11: Chunk notification shape uses `content.text` not `delta`

**Severity**: Medium (IDE must parse correctly)

### Finding

The streaming chunk notifications look like:

```json
{
  "jsonrpc": "2.0",
  "method": "session/update",
  "params": {
    "update": {
      "sessionUpdate": "agent_message_chunk",
      "content": {
        "text": "The actual text content...",
        "type": "text"
      }
    }
  }
}
```

The text is in `update.content.text`, NOT in a `delta` field. Each chunk is a
**complete content block** (with type), not just a text delta.

### All observed `sessionUpdate` types

| Type | Shape | When |
|------|-------|------|
| `agent_message_chunk` | `{content: {text, type}}` | During text generation |
| `usage_update` | `{size: number, used: number}` | After completion |
| `session_info_update` | (varies) | After session state changes |

---

## Issue #12: No thinking_chunk notifications for effort=high

**Severity**: Low (missing feature or provider limitation)
**Test**: `test-streaming.sh` — "thinking_chunk notifications with effort=high"

### Behavior

Setting effort to "high" and prompting "Think step by step" produces a normal response
but no `thinking_chunk` or `thinkingChunk` session update notifications. The thinking
happens server-side but isn't streamed to the IDE.

### Possible causes

1. The default provider (cerebras/openai) doesn't support streaming thinking tokens
2. The `effort` config only affects the `temperature` or system prompt, not extended thinking
3. Thinking chunks are only implemented for Anthropic API (not OpenAI-compat)

### Impact

IDE can't show a "thinking" indicator with streamed reasoning content.

---

## Issue #13: session/new `modes` field uses `availableModes` key

**Severity**: Low (documentation)

### Finding

The modes object in session/new response:

```json
{
  "modes": {
    "availableModes": [
      {"id": "code", "name": "Code", "description": "..."},
      {"id": "plan", "name": "Plan", "description": "..."},
      {"id": "research", "name": "Research", "description": "..."}
    ],
    "currentModeId": "code"
  }
}
```

Key is `availableModes` (not `modes` inside modes).

---

## Issue #14: Anthropic provider shows "API key not set" but is listed as available

**Severity**: Low (misleading UX)
**Test**: `test-config-options.sh` — "provider options have readiness descriptions"

### Behavior

The provider option for `anthropic` shows:
```
description: "API key env ANTHROPIC_API_KEY is not set"
```

But the provider is still selectable. If the user switches to it, prompts will fail.

### Related to

Issue #7 (Provider readiness) — the `ready: bool` field in `ConfigOptionValue` was
proposed in `W4-B-provider-readiness.md`. The description string is the only signal
currently; a structured `ready` boolean would let the IDE grey out unavailable providers.

---

## Confirmed Working (Not Bugs)

| Feature | Status | Notes |
|---------|--------|-------|
| session/cancel | WORKS | Returns `stopReason: "cancelled"`, subsequent prompts work |
| session/list | WORKS | Returns all session IDs |
| session/close | WORKS | Closed session rejects prompts with error |
| initialize | WORKS | Accepted, purpose unclear |
| session/set_mode | WORKS | Mode switches to "plan" accepted |
| config/update model | WORKS | Changes model, persists across prompts |
| config/update provider | WORKS | Changes provider, updates available models |
| config/update effort | WORKS | Low/medium/high accepted |
| Multi-turn context | WORKS | History preserved within same session |
| Context growth tracking | WORKS | `usage_update` shows increasing `used` across turns |
| Config options in session/new | WORKS | 6 options returned |
| Modes in session/new | WORKS | 3 modes (code, plan, research) |

---

## Updated Test Results (Round 2)

```
Core:              8 passed, 0 failed
Models:            4 passed, 1 FAILED (BUG#02), 1 warn
MCP:               3 passed, 2 FAILED (BUG#01), 1 warn
Edge:              7 passed, 0 failed, 2 warn
Lifecycle:         9 passed, 0 failed, 1 warn (invalid model accepted)
Streaming:         8 passed, 1 FAILED (timing), 1 warn (no thinking chunks)
Tool Loop:         2 passed, 0 failed, 3 skipped (no bridge)
Config Options:    8 passed, 0 failed, 1 warn (unknown option accepted)

Total:            49 passed, 4 FAILED, 6 warned, 3 skipped
```

The 4 failures are:
1. BUG#02: model param in session/new ignored
2. BUG#01: nonexistent MCP binary — no structured error
3. BUG#01: crashing MCP binary — no structured error
4. Multi-turn: false negative (model safety refusal on certain phrasings, works with neutral prompts)

---

## Issue #15: `--model` CLI flag doesn't set initial model

**Severity**: Medium (same root cause as BUG#02)

### Behavior

```bash
roko acp --model sonnet   # -> provider=anthropic, model=opus (wrong!)
roko acp --model haiku    # -> provider=anthropic, model=opus (wrong!)
roko acp --model gpt-4o   # -> provider=anthropic, model=opus (wrong!)
roko acp                  # -> provider=cerebras,  model=cerebras-8b (HashMap random)
```

The `--model` flag influences **provider** selection (always picks anthropic when flag is present)
but the model within that provider resolves to the HashMap-first entry (opus), not the requested model.

### Root Cause

Same as BUG#02 — `SessionNewParams.model` and the CLI `--model` flag both feed into
`from_roko_config()` but the value isn't used to override the model selection. The fix in
W1-A + W1-B batches should address this.

---

## Issue #16: Model names are provider-scoped (cross-provider fails silently)

**Severity**: Low (design limitation, needs documentation)

### Behavior

If the session's current provider is `cerebras`, setting model to `sonnet` via config/update
fails silently (model stays cerebras-8b) because "sonnet" is not in cerebras's model list.

The user must:
1. Switch provider to `openai` first
2. Then switch model to `sonnet`

### Impact

The IDE must sequence provider and model changes. A single "switch to sonnet" action
requires knowing which provider owns that model name.

### Possible Fix

Add a `session/config/update` variant that accepts both provider and model together, or
auto-resolve provider from model name when possible.

---

## Issue #17: `roko acp` CLI flags reference

| Flag | Effect | Tested |
|------|--------|--------|
| `--workdir /path` | Sets working directory | Works |
| `--config /path` | Config file override | Not tested |
| `--model name` | Intended: set initial model | BROKEN (BUG#02) |
| `--profile name` | Configuration profile | Not tested |
| `--role name` | Agent role/persona | Not tested |
| `--log-file /path` | Log output path | Works (default .roko/acp.log) |
| `--repo /path` | Repository path | Not tested |
4. Multi-turn timing (false negative in FIFO test, works in Python)
