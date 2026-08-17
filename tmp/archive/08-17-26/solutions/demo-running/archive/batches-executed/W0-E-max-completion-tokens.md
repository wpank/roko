# W0-E: Fix `max_tokens` vs `max_completion_tokens` for Newer OpenAI Models

**Priority**: P0 — `roko run --model gpt54-mini` returns HTTP 400 on every request
**Effort**: 15 minutes
**Files to modify**: 2 files
**Dependencies**: None

## Problem

`roko run "Build a CLI calculator in Rust" --model gpt54-mini` fails with:
```
HTTP 400: max_tokens is not supported with this model. Use max_completion_tokens instead.
```

OpenAI's newer models (GPT-5.4-mini, o1, o3, etc.) reject the `max_tokens` parameter and require `max_completion_tokens` instead.

## Root Cause

### Issue 1: Railway config missing the flag

**File**: `docker/railway.roko.toml` — gpt54-mini model config (line 52-60)

The `gpt54-mini` model definition is missing `use_max_completion_tokens = true`:
```toml
[models.gpt54-mini]
provider = "openai"
slug = "gpt-5.4-mini"
context_window = 128000
max_output = 16384
supports_tools = true
supports_thinking = false
supports_vision = true
tool_format = "openai_json"
# MISSING: use_max_completion_tokens = true
```

The main `roko.toml` has this flag, but the Railway config doesn't.

### Issue 2: Perplexity and Cerebras backends missing the flag

**File**: `crates/roko-agent/src/tool_loop/backends/mod.rs`

The `create_openai_compat_backend()` function correctly passes `.with_use_max_completion_tokens()` for the `OpenAiCompat` provider kind (line 62), but the Perplexity (lines 87-97) and Cerebras (lines 112-125) branches are missing it:

```rust
// OpenAiCompat — HAS the flag (line 62):
.with_use_max_completion_tokens(model.use_max_completion_tokens)

// PerplexityApi — MISSING the flag (lines 87-97):
OpenAiCompatBackend::new(api_key, model.slug.clone())
    .with_provider_id(...)
    .with_base_url(...)
    // ... no .with_use_max_completion_tokens()

// CerebrasApi — MISSING the flag (lines 112-125):
OpenAiCompatBackend::new(api_key, model.slug.clone())
    .with_provider_id(...)
    .with_base_url(...)
    // ... no .with_use_max_completion_tokens()
```

## Exact Code to Change

### Fix 1: Add flag to Railway config

**File**: `docker/railway.roko.toml` — line 60

**After** `tool_format = "openai_json"` in `[models.gpt54-mini]`, add:
```toml
use_max_completion_tokens = true
```

Also add to `[models.sonnet]` since the Anthropic API backend handles this separately, but for safety:
```toml
# No change needed — Anthropic API backend uses its own parameter name
```

### Fix 2: Add flag to Perplexity backend

**File**: `crates/roko-agent/src/tool_loop/backends/mod.rs` — line 94 (before `.with_ttft_timeout_ms`)

**Add:**
```rust
                    .with_use_max_completion_tokens(model.use_max_completion_tokens)
```

### Fix 3: Add flag to Cerebras backend

**File**: `crates/roko-agent/src/tool_loop/backends/mod.rs` — line 122 (before `.with_ttft_timeout_ms`)

**Add:**
```rust
                    .with_use_max_completion_tokens(model.use_max_completion_tokens)
```

## How the Fix Works

The `OpenAiCompatBackend::post_json()` at `openai_compat_backend.rs:268-274` already has correct conditional logic:
```rust
if let Some(max_tokens) = self.max_tokens {
    let key = if self.use_max_completion_tokens {
        "max_completion_tokens"
    } else {
        "max_tokens"
    };
    body_obj.insert(key.to_string(), Value::from(max_tokens));
}
```

The fix is just ensuring the flag reaches the backend from the config.

## Agent Prompt

```
Read /Users/will/dev/nunchi/roko/roko/tmp/solutions/demo-running/batches/W0-E-max-completion-tokens.md and implement all changes. Add `use_max_completion_tokens = true` to gpt54-mini in docker/railway.roko.toml. Add `.with_use_max_completion_tokens(model.use_max_completion_tokens)` to Perplexity and Cerebras backends in tool_loop/backends/mod.rs. Do NOT run cargo build/test/clippy/fmt. Mark the checklist items as done.
```

## Commit

This batch is committed with Wave 0 (critical pipeline fixes). Do not commit individually.

## Checklist

- [x] Add `use_max_completion_tokens = true` to `[models.gpt54-mini]` in `docker/railway.roko.toml`
- [x] Add `.with_use_max_completion_tokens()` to Perplexity backend in `tool_loop/backends/mod.rs`
- [x] Add `.with_use_max_completion_tokens()` to Cerebras backend in `tool_loop/backends/mod.rs`
- [ ] Verify: gpt54-mini requests send `max_completion_tokens` not `max_tokens`
