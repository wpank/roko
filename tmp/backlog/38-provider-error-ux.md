# Provider Error UX

**Priority**: P2 — poor user experience, support burden
**Size**: S (half day)
**Crates**: `crates/roko-agent/`, `crates/roko-core/`

---

## Problem

When an LLM provider returns an error (HTTP 401, 403, 429, 500, timeout), the raw error
message surfaces to the user with no guidance. Users see messages like:

```
Error: status: 401 Unauthorized
```

```
Error: reqwest::Error { kind: TimedOut, url: "https://api.anthropic.com/v1/messages" }
```

```
Error: status: 429 Too Many Requests {"error":{"type":"rate_limit_error"}}
```

These messages do not tell the user: which provider failed, which config key to check,
or what to try next. The user must already know the roko config schema, the provider's
error format, and which environment variable maps to which provider. This is the most
common source of first-run failures and support questions.

---

## Where to look

- `crates/roko-agent/src/provider/anthropic_api.rs` — Anthropic HTTP response handling
- `crates/roko-agent/src/provider/openai_compat.rs` — OpenAI-compatible provider
  (also used for Cerebras, local models)
- `crates/roko-agent/src/provider/hermes.rs` — Hermes provider
- `crates/roko-agent/src/provider/openclaw.rs` — OpenClaw provider
- `crates/roko-agent/src/provider/pre_flight.rs` — pre-flight provider validation
- `crates/roko-agent/src/dispatcher/mod.rs` — dispatch boundary where provider errors
  surface to the caller
- `crates/roko-core/src/error/retry.rs` — retry and error classification logic

---

## What to do

**Step 1.** At the dispatch boundary (`dispatcher/mod.rs`), wrap raw provider errors
with actionable context before returning them to the CLI/serve layer. The wrapper should
include:

- **Provider name** (e.g., "anthropic", "openai", "cerebras")
- **Relevant config section** (e.g., `roko.toml [providers.anthropic]`)
- **Suggested fix** based on the HTTP status code

**Step 2.** Implement status-code-specific error messages:

| Status | Message template |
|---|---|
| 401 | `Authentication failed for provider '{name}'. Check roko.toml [providers.{name}.api_key] or set {ENV_VAR} environment variable. Run 'roko doctor' to verify.` |
| 403 | `Access denied by provider '{name}'. Your API key may lack permissions for model '{model}'. Check provider dashboard for key restrictions.` |
| 429 | `Rate limited by provider '{name}'. Wait and retry, configure backoff in roko.toml [providers.{name}.retry], or switch models with 'roko config models route'.` |
| 500-599 | `Provider '{name}' returned a server error ({status}). This is usually transient. Retry, or check {provider_status_url} for outages.` |
| Timeout | `Provider '{name}' timed out after {duration}. Increase timeout in roko.toml [providers.{name}.timeout_ms] or try a faster model.` |
| Connection refused | `Cannot connect to provider '{name}' at {url}. Check your network connection and the provider URL in roko.toml [providers.{name}.base_url].` |

**Step 3.** Map each provider to its environment variable name and status page URL:

| Provider | Env var | Status page |
|---|---|---|
| Anthropic | `ANTHROPIC_API_KEY` | `status.anthropic.com` |
| OpenAI | `OPENAI_API_KEY` | `status.openai.com` |
| Cerebras | `CEREBRAS_API_KEY` | (none) |
| Gemini | `GEMINI_API_KEY` | `status.cloud.google.com` |
| Perplexity | `PERPLEXITY_API_KEY` | (none) |

**Step 4.** Ensure the `roko doctor` command already checks for missing/invalid API
keys. If it does, reference it in the error messages ("Run 'roko doctor' to diagnose").
If it does not, add a basic key-presence check there as part of this work.

---

## Acceptance criteria

- [ ] 401/403 errors name the provider and point to the config key or env var
- [ ] 429 errors suggest backoff config and model switching
- [ ] Timeout errors show the duration and point to timeout config
- [ ] 5xx errors mention the provider status page where available
- [ ] Connection errors name the provider and the URL that failed
- [ ] All error messages suggest `roko doctor` as a diagnostic step
- [ ] Raw HTTP bodies are still available in debug/trace logging (not lost)
- [ ] All existing tests pass (`cargo test --workspace`)

### Verify

1. Set `ANTHROPIC_API_KEY=invalid` and run `roko run "hello"`
2. Error should read: "Authentication failed for provider 'anthropic'. Check roko.toml
   [providers.anthropic.api_key] or set ANTHROPIC_API_KEY environment variable. Run
   'roko doctor' to verify."
3. Unset all API keys and run `roko doctor` — it should report which keys are missing

### Not in scope

- Automatic provider fallback on error (already handled by cascade router)
- Retry logic changes (existing retry in `error/retry.rs` is adequate)
- Provider-specific error body parsing beyond HTTP status codes

---

**Origin**: model-provider-audit.md (section 13), redesign-plan.md
