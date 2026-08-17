# W9-C: Cost Tracking Fix -- model_profile Not Passed to ToolLoop + Usage Field Fallback

**Priority**: P0 -- cost always reports $0.00 for OpenAI-compat providers; no budget enforcement possible
**Effort**: 15 minutes
**Files to modify**: 2 files
**Dependencies**: None

## Problem

The OpenAI-compatible provider adapter (`openai_compat.rs`) constructs `ToolLoop::new()` without calling `.with_model_profile(model.clone())`. Without the model profile, the tool loop cannot compute per-turn costs because it does not know the model's pricing (cost_input_per_m, cost_output_per_m, cost_cache_read_per_m). Every other provider (Anthropic API, Gemini, Perplexity) already passes the model profile.

Additionally, `parse_usage_observation` in `translate/openai.rs` only checks for `prompt_tokens` in the usage response. Some OpenAI-compatible providers (notably newer ones and the OpenAI Responses API) use `input_tokens` instead. When the field name does not match, token counts are `None` and cost computation returns zero.

These two issues combine to make cost tracking report $0.00 for all OpenAI-compatible model runs, breaking budget enforcement entirely.

## Root Cause

### 1. Missing `.with_model_profile()` on ToolLoop construction
**File**: `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/provider/openai_compat.rs` (line 378)

`ToolLoop::new(translator, dispatcher, backend)` is called with `.with_max_iterations()` and `.with_context_token_limit()` but NOT `.with_model_profile(model.clone())`.

Compare with:
- Anthropic API at `provider/anthropic_api/tool_loop.rs:46` which includes `.with_model_profile(model.clone())`
- Gemini at `gemini/adapter.rs:68` which includes `.with_model_profile(model.clone())`
- Perplexity at `perplexity/adapter.rs:189` which includes `.with_model_profile(model.clone())`

### 2. No fallback for `input_tokens` field name
**File**: `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/translate/openai.rs` (lines 299-304)

`parse_usage_observation` reads `usage.get("prompt_tokens")` for input tokens and `usage.get("completion_tokens")` for output tokens. These are the OpenAI Chat Completions API field names. But some providers use `input_tokens` and `output_tokens` instead. Without a fallback, token counts are `None` for those providers.

Note: the `cache_read_tokens` parsing (lines 305-308) already has a fallback chain: it tries `prompt_tokens_details/cached_tokens` then falls back to `cached_tokens`. The input/output token parsing should follow the same pattern.

## Exact Code to Change

### File 1: `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/provider/openai_compat.rs`

#### Change 1.1: Add `.with_model_profile()` to ToolLoop construction (line 378)

**Find this code:**
```rust
            let tool_loop = ToolLoop::new(translator, dispatcher, backend)
                .with_max_iterations(tool_loop_max_iterations())
                .with_context_token_limit(
                    usize::try_from(model.context_window).unwrap_or(usize::MAX),
                );
```

**Replace with:**
```rust
            let tool_loop = ToolLoop::new(translator, dispatcher, backend)
                .with_max_iterations(tool_loop_max_iterations())
                .with_context_token_limit(
                    usize::try_from(model.context_window).unwrap_or(usize::MAX),
                )
                .with_model_profile(model.clone());
            tracing::debug!(
                model = %model.slug,
                context_window = model.context_window,
                "ToolLoop created with model profile for cost tracking"
            );
```

**Why this is safe**:
- `model` is a `&ModelProfile` parameter of `create_agent()` (line 353).
- `ModelProfile` is already imported on line 40: `use roko_core::config::schema::{ModelProfile, ProviderConfig};`
- `ToolLoop::with_model_profile(self, model_profile: ModelProfile) -> Self` takes an owned `ModelProfile`, so `model.clone()` is needed.
- `tracing` is already available via the standard prelude; if not, add `use tracing;` at the top.

### File 2: `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/translate/openai.rs`

#### Change 2.1: Add fallback for `input_tokens` and `output_tokens` field names (lines 299-304)

**Find this code:**
```rust
    let input_tokens = usage
        .get("prompt_tokens")
        .and_then(serde_json::Value::as_u64);
    let output_tokens = usage
        .get("completion_tokens")
        .and_then(serde_json::Value::as_u64);
```

**Replace with:**
```rust
    let input_tokens = usage
        .get("prompt_tokens")
        .or_else(|| usage.get("input_tokens"))
        .and_then(serde_json::Value::as_u64);
    let output_tokens = usage
        .get("completion_tokens")
        .or_else(|| usage.get("output_tokens"))
        .and_then(serde_json::Value::as_u64);
```

**Why this is correct**:
- `usage.get()` returns `Option<&Value>`. Chaining `.or_else(|| usage.get("input_tokens"))` tries the alternative field name if the first is absent.
- The pattern matches how `cache_read_tokens` already works on lines 305-308: it chains `.or_else(|| usage.get("cached_tokens"))`.
- No new imports needed -- `serde_json::Value::as_u64` is already in scope.

## Imports

### openai_compat.rs
No new imports needed. The change uses `tracing::debug!()` with the full crate path, which works because `tracing` is a dependency in `roko-agent`'s Cargo.toml. Do NOT add `use tracing;` -- that is not valid Rust syntax. If you prefer a shorter form, you could add `use tracing::debug;` at the top of the file, but the `tracing::debug!()` form in the change code works as-is.

### translate/openai.rs
No new imports needed -- all types are already in scope.

## Verification

```bash
# 1. Build
cd /Users/will/dev/nunchi/roko/roko
cargo check -p roko-agent 2>&1 | head -10

# 2. Run existing usage parsing tests
cargo test -p roko-agent --lib translate 2>&1 | tail -20

# 3. Run the full openai_compat test suite
cargo test -p roko-agent --lib provider::openai_compat 2>&1 | tail -20

# 4. Verify the model profile import is present
grep -n 'ModelProfile' crates/roko-agent/src/provider/openai_compat.rs | head -5

# 5. Verify other providers already do this (should show 3+ matches)
grep -rn 'with_model_profile' crates/roko-agent/src/provider/ crates/roko-agent/src/gemini/ crates/roko-agent/src/perplexity/

# 6. Clippy
cargo clippy -p roko-agent --no-deps -- -D warnings 2>&1 | tail -20
```

## Agent Prompt

```
You are implementing a 2-file cost tracking fix for the Roko agent toolkit.

## Context

Roko is a Rust agent toolkit. The OpenAI-compatible provider adapter constructs
ToolLoop without passing the model profile, so per-turn cost computation never
fires and costs report $0.00. Other providers (Anthropic, Gemini, Perplexity)
already pass it correctly. Additionally, the usage parser does not handle
alternative field names used by some providers.

## Architecture

The cost tracking chain is:

1. Provider adapter creates a `ToolLoop` in `create_agent()` (openai_compat.rs line 350)
2. `ToolLoop` has an optional `model_profile: Option<ModelProfile>` field
3. If set, after each turn it computes cost from usage * model pricing
4. `parse_usage_observation()` (openai.rs line 291) extracts token counts from the response

The fix has two parts:
- Part A: Pass `model.clone()` to `ToolLoop` via `.with_model_profile()`
- Part B: Add fallback field name lookups in `parse_usage_observation()`

## What to do

### Change 1: openai_compat.rs line 378

Current code:
```rust
            let tool_loop = ToolLoop::new(translator, dispatcher, backend)
                .with_max_iterations(tool_loop_max_iterations())
                .with_context_token_limit(
                    usize::try_from(model.context_window).unwrap_or(usize::MAX),
                );
```

Add `.with_model_profile(model.clone())` after `.with_context_token_limit(...)`.

The `model` parameter is `&ModelProfile` (line 353 of `create_agent`).
`ModelProfile` is imported on line 40.
`ToolLoop::with_model_profile` takes `ModelProfile` by value, so `.clone()` is needed.

Also add `tracing::debug!` after the ToolLoop creation to log that cost tracking
is enabled.

### Change 2: translate/openai.rs lines 299-304

Add `.or_else(|| usage.get("input_tokens"))` after `.get("prompt_tokens")`.
Add `.or_else(|| usage.get("output_tokens"))` after `.get("completion_tokens")`.

This matches the existing fallback pattern on lines 305-308 for `cache_read_tokens`.

### Verification
Run: `cargo check -p roko-agent && cargo test -p roko-agent --lib translate && cargo test -p roko-agent --lib provider::openai_compat && cargo clippy -p roko-agent --no-deps -- -D warnings`
```

## Commit

This batch is committed with Wave 9 (Systemic Pipeline Quality). Do not commit individually.

## Checklist

- [ ] `ToolLoop::new()` in openai_compat.rs line 378 has `.with_model_profile(model.clone())`
- [ ] `tracing::debug!` logs model slug and context window after ToolLoop creation
- [ ] `parse_usage_observation` has `.or_else(|| usage.get("input_tokens"))` fallback for input tokens
- [ ] `parse_usage_observation` has `.or_else(|| usage.get("output_tokens"))` fallback for output tokens
- [ ] `tracing::debug!` call compiles in openai_compat.rs (no explicit import needed -- uses full crate path)
- [ ] `cargo check -p roko-agent` passes
- [ ] `cargo test -p roko-agent --lib translate` passes
- [ ] `cargo test -p roko-agent --lib provider::openai_compat` passes
- [ ] `cargo clippy -p roko-agent --no-deps -- -D warnings` passes

## Audit Status

Audited: 2026-05-05. PASS no changes needed
