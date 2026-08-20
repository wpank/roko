# 38 — Provider Error UX

**Priority**: P2 — first-run failures surface raw error strings without actionable guidance, causing unnecessary support burden
**Size**: S (half day)
**Crates**: `crates/roko-agent/`, `crates/roko-cli/`
**Depends on**: None

---

## Background

When an LLM provider returns an HTTP error or times out, users see messages like `"provider error: authentication failed"` or `"provider error: rate limited"`. These messages come from the `ProviderError` enum's `Display` implementation in `crates/roko-agent/src/provider/mod.rs` (lines 991-1007). They identify the error class but give no guidance: which config key to check, which environment variable to set, what to try next.

The infrastructure for rich, actionable error messages **already exists**. `map_provider_error()` in `crates/roko-agent/src/provider/mod.rs` (line 907) produces provider-specific, pattern-matched error strings that name the provider, the environment variable, the config section, and the suggested fix. For example, a 401 error produces: `"API key invalid for provider 'anthropic' (HTTP 401). Check $ANTHROPIC_API_KEY or roko.toml [providers.anthropic]."` This function is called from `crates/roko-agent/src/provider/anthropic_api/tool_loop.rs` (line 505) and from `crates/roko-agent/src/openai_compat_backend.rs` (line 290).

The gap is that `map_provider_error` is called in two specific code paths (Anthropic tool-loop and OpenAI-compat HTTP requests), but not at all other error boundaries. The Gemini, Hermes, OpenClaw, and Claude CLI providers each classify HTTP errors into `ProviderError` variants but do not call `map_provider_error`. The `ProviderError::Display` impl (lines 991-1007) is still what most users see because the rich decorator is not applied at the dispatch boundary.

Additionally, `roko doctor` already checks for missing API keys via `check_configured_provider_keys` at `doctor.rs` line 838. The error messages in `map_provider_error` suggest "Run 'roko doctor'" and that guidance is already accurate — doctor will report which keys are missing.

## Current State

1. `crates/roko-agent/src/provider/mod.rs` lines 980-1007 — `ProviderError` enum has variants: `RateLimit { retry_after_ms }`, `AuthFailure`, `Timeout`, `ServerError(u16)`, `ContentPolicy`, `ContextOverflow`, `ModelNotFound`, `Other(String)`. The `Display` impl emits terse strings like `"authentication failed"`, `"rate limited; retry after 5000 ms"`, `"server error 503"`.

2. `crates/roko-agent/src/provider/mod.rs` lines 907-977 — `map_provider_error(kind, provider_name, api_key_env, base_url, err) -> String` matches on error text patterns (401/unauthorized, 429/rate_limit, 404/model_not_found, connection_refused, ENOENT/binary_not_found) and returns actionable strings. It also handles the generic fallback: `"Provider '{name}' ({kind}) error: {raw}"`.

3. `crates/roko-agent/src/openai_compat_backend.rs` line 290 — `OpenAiCompatBackend::decorate_error(&self, raw_err)` calls `map_provider_error` with the backend's `provider_kind`, `provider_id`, `api_key_env`, and `base_url`. This is used for the HTTP request error path in the OpenAI-compatible provider (Cerebras, local models, OpenRouter).

4. `crates/roko-agent/src/provider/anthropic_api/tool_loop.rs` line 505 — `map_provider_error` is called when the Anthropic tool-loop streaming request fails.

5. `crates/roko-agent/src/gemini/adapter.rs` lines 208-231 — `classify_error(status, body)` returns `ProviderError` variants but does not call `map_provider_error`. The errors surface as `ProviderError::Display`.

6. `crates/roko-agent/src/provider/hermes.rs` — Hermes provider errors surface as `ProviderError` directly, no `map_provider_error` call.

7. `crates/roko-agent/src/provider/openclaw.rs` — same pattern as Hermes.

8. `crates/roko-agent/src/error.rs` lines 12-37 — the top-level `AgentError` enum has a `Provider(#[from] ProviderError)` variant, formatted as `"provider error: {0}"`. This is what callers outside `roko-agent` see. There is no provider-name context at this level.

9. `crates/roko-cli/src/doctor.rs` line 838 — `check_configured_provider_keys` already iterates over configured providers (`AnthropicApi`, `OpenAiCompat`, `PerplexityApi`, `GeminiApi`, `CerebrasApi`), checks each `api_key_env` variable, and emits `DoctorStatus::Warn` when keys are missing. The doctor check is complete; it just needs to be referenced in error messages (which `map_provider_error` already does).

## Implementation Plan

### Step 1: Add provider context to `ProviderError`

The `ProviderError` enum does not carry provider-name context. Add context at the point where `ProviderError` is converted to `AgentError` and displayed to the user.

In `crates/roko-agent/src/error.rs`, add a helper:
```rust
impl AgentError {
    /// Attach provider context to a bare ProviderError for user-facing display.
    pub fn with_provider_context(
        err: ProviderError,
        provider_name: &str,
        api_key_env: Option<&str>,
        base_url: Option<&str>,
        kind: ProviderKind,
    ) -> Self {
        // Convert ProviderError to a string via map_provider_error, then wrap
        // as AgentError::Other so the context string is preserved.
        let decorated = crate::provider::map_provider_error(
            kind, provider_name, api_key_env, base_url, &err,
        );
        AgentError::Other(decorated)
    }
}
```

### Step 2: Call `map_provider_error` in Gemini adapter

In `crates/roko-agent/src/gemini/adapter.rs`, add a `decorate_error` method analogous to `OpenAiCompatBackend::decorate_error`. Call it when converting HTTP responses to errors in the adapter's request methods. Pass `ProviderKind::GeminiApi`, the configured provider name from `ProviderConfig`, and `GEMINI_API_KEY` as the env var.

### Step 3: Call `map_provider_error` in Hermes and OpenClaw providers

In `crates/roko-agent/src/provider/hermes.rs` and `openclaw.rs`, identify where `ProviderError` variants are constructed from HTTP responses and wrap them with `map_provider_error` before returning.

### Step 4: Verify `roko doctor` API key check exists

`roko doctor` already implements API key presence checking via `check_configured_provider_keys` in `crates/roko-cli/src/doctor.rs` (line 838). It iterates over configured providers with `ProviderKind::AnthropicApi`, `OpenAiCompat`, `PerplexityApi`, `GeminiApi`, and `CerebrasApi`, checks each provider's `api_key_env` variable, and emits a `DoctorStatus::Warn` if keys are missing.

The error messages added in Steps 1-3 should reference `roko doctor` as a diagnostic step (they already do via `map_provider_error`). No additional doctor code is needed. Verify the wording in `map_provider_error` is consistent with the doctor output format.

### Step 5: Add status page URLs to rate-limit messages

In `map_provider_error` (provider/mod.rs line 934), extend the 429 message to include the provider's status page where applicable:

```
Rate limited by provider 'anthropic' (HTTP 429). Wait and retry, or switch providers.
Check https://status.anthropic.com for outages.
```

Add a helper `provider_status_url(provider_name: &str) -> Option<&'static str>` with a match arm for known providers.

## Acceptance Criteria

1. `ANTHROPIC_API_KEY=invalid roko run "hello"` produces: `"API key invalid for provider 'anthropic' (HTTP 401). Check $ANTHROPIC_API_KEY or roko.toml [providers.anthropic]."` (or equivalent from `map_provider_error`).
2. `roko doctor` already reports missing API keys (existing behavior, no changes needed). Error messages from `map_provider_error` reference `roko doctor` as the diagnostic step.
3. `grep -rn 'map_provider_error' crates/roko-agent/src/gemini/ crates/roko-agent/src/provider/hermes.rs crates/roko-agent/src/provider/openclaw.rs` returns at least one call site per file.
4. Raw HTTP bodies (the original response text) are still logged at `tracing::debug!` level so they are available in `.roko/roko.log` with `--verbose`.
5. `cargo test --workspace` passes with no regressions.

## Verification Checklist

- [ ] `ANTHROPIC_API_KEY=invalid cargo run -p roko-cli -- run "hello"` prints actionable auth error
- [ ] `unset ANTHROPIC_API_KEY; cargo run -p roko-cli -- doctor` reports the missing key (pre-existing check, should already pass)
- [ ] `cargo test -p roko-agent 2>&1 | tail -5` shows all tests passed
- [ ] `cargo test --workspace 2>&1 | tail -5` shows all tests passed
- [ ] `grep -rn 'map_provider_error' crates/roko-agent/src/gemini/` returns at least one line

## Files to Modify

| File | Change |
|---|---|
| `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/error.rs` | Add `with_provider_context` helper |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/provider/mod.rs` | Add `provider_status_url` helper; extend 429 message with status URL |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/gemini/adapter.rs` | Add `decorate_error` method; call it on HTTP error responses |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/provider/hermes.rs` | Call `map_provider_error` on HTTP errors |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/provider/openclaw.rs` | Call `map_provider_error` on HTTP errors |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/doctor.rs` | No changes needed — `check_configured_provider_keys` at line 838 already covers this |
