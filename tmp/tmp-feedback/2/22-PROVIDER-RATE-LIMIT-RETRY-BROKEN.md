# Provider Rate Limit Retry Never Executes

## Problem

Rate limit retries are built into the OpenAI-compatible provider but never execute. All HTTP
errors (including 429 Rate Limit) are mapped to `LlmError::Network`, but the retry loop only
retries on `LlmError::Provider` with a rate limit flag.

Additionally, Gemini native provider has no streaming support.

## Root Cause

### A. Error classification mismatch

**File:** `crates/roko-agent/src/provider/openai_compat.rs`

```rust
// HTTP error handling:
let response = client.post(url).json(&body).send().await
    .map_err(|e| LlmError::Network(e.to_string()))?;  // ← ALL errors become Network

if !response.status().is_success() {
    let status = response.status();
    let body = response.text().await.unwrap_or_default();
    return Err(LlmError::Network(format!("{status}: {body}")));  // ← 429 → Network
}
```

```rust
// Retry loop (different file):
match provider.complete(request).await {
    Err(LlmError::Provider { rate_limited: true, .. }) => {
        // retry with backoff  ← NEVER reached because 429 → Network, not Provider
    },
    Err(e) => return Err(e),  // ← all errors fall through here
}
```

The fix is trivial: check the HTTP status code and map 429 to `LlmError::Provider { rate_limited: true }`.

### B. Gemini native: no streaming

**File:** `crates/roko-agent/src/provider/gemini.rs`

The Gemini native provider implements `complete()` but not `stream()`. When streaming is
requested, it falls back to non-streaming, which means:
- No incremental output in the TUI during Gemini tasks
- No progress indication — the agent appears frozen until completion
- Timeout may kill long Gemini responses

## Fix

### Fix 1: Classify 429 as rate-limited Provider error (~10 min)

**File:** `crates/roko-agent/src/provider/openai_compat.rs`

```rust
if !response.status().is_success() {
    let status = response.status();
    let body = response.text().await.unwrap_or_default();
    if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
        return Err(LlmError::Provider {
            message: format!("Rate limited: {body}"),
            rate_limited: true,
            status_code: Some(429),
        });
    }
    if status.is_server_error() {
        return Err(LlmError::Provider {
            message: format!("{status}: {body}"),
            rate_limited: false,
            status_code: Some(status.as_u16()),
        });
    }
    return Err(LlmError::Network(format!("{status}: {body}")));
}
```

### Fix 2: Add Gemini streaming (~30 min)

**File:** `crates/roko-agent/src/provider/gemini.rs`

Implement `stream()` using Gemini's `streamGenerateContent` endpoint. The response format
uses SSE with `data:` lines containing JSON chunks.

## Files to Modify

| File | Change |
|------|--------|
| `crates/roko-agent/src/provider/openai_compat.rs` | Classify 429 as rate-limited |
| `crates/roko-agent/src/provider/gemini.rs` | Add streaming support |

## Priority

**P1** — Rate limit retries are a table-stakes feature for any LLM integration. Without them,
a single 429 response kills the entire plan execution. The fix is 10 lines.
