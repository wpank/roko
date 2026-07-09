# Issue: Provider Readiness Reporting

## Problem Statement

The `session/new` response includes provider options with descriptions like
"Ready" or "API key env X is not set". This is informational but:

1. There's no structured way for the client to know which providers are usable
2. A provider marked "Ready" may still fail at runtime (network, quota, model not found)
3. The IDE has no way to validate a provider works before the user sends a prompt

## Observed Behavior

From `session/new` response:
```json
{
  "options": [
    {"name": "Anthropic", "value": "anthropic", "description": "API key env ANTHROPIC_API_KEY is not set"},
    {"name": "Claude_cli", "value": "claude_cli", "description": "Ready"},
    {"name": "Openai", "value": "openai", "description": "Ready"},
    {"name": "Openrouter", "value": "openrouter", "description": "API key env OPENROUTER_API_KEY is not set"}
  ]
}
```

### What "Ready" means

From `crates/roko-acp/src/session.rs` (`provider_option_description`):
- "Ready" = the API key env var is set, OR the provider kind doesn't need one (claude_cli)
- "API key env X is not set" = the env var doesn't exist in the process environment

### What "Ready" does NOT mean

- The API key is valid
- The provider's API endpoint is reachable
- The requested model exists on that provider
- The account has quota/credits

## Test Results

All "Ready" providers passed in our testing:
- openai (gpt-4o): PASS
- openai (gpt-4o-mini): PASS
- claude_cli (claude-sonnet-4-6): PASS
- zai: PASS
- moonshot: PASS

However, during earlier testing, claude_cli returned empty responses when run
from within a Claude Code parent process (detected via CLAUDE_CODE env var).
This failure mode is invisible to the readiness check.

## Proposed Solution

### 1. Add `ready` boolean to provider options

```json
{
  "name": "Openai",
  "value": "openai",
  "description": "Ready",
  "ready": true,
  "readyReason": "api_key_set"
}
```

Clients can filter on `ready: true` without parsing description strings.

### 2. Optional provider health check endpoint

Add a method that actually tests the provider:

```json
{"method": "provider/check", "params": {"provider": "openai"}}
```

Response:
```json
{
  "result": {
    "provider": "openai",
    "status": "ok",         // "ok" | "error" | "timeout"
    "latency_ms": 234,
    "error": null,
    "models_available": ["gpt-4o", "gpt-4o-mini"]
  }
}
```

This makes a lightweight API call (e.g., models/list) to verify the provider works.

### 3. Structured readiness enum

Instead of freeform description strings:

```rust
#[derive(Serialize)]
enum ProviderReadiness {
    Ready,
    MissingApiKey { env_var: String },
    CommandNotFound { command: String },  // for claude_cli
    NetworkError { message: String },
    Disabled,
}
```

## Implementation Location

| File | Change |
|------|--------|
| `crates/roko-acp/src/session.rs:944+` | Add `ready` field to ConfigOptionValue |
| `crates/roko-acp/src/handler.rs` | Add `provider/check` method |
| `crates/roko-acp/src/types.rs` | Add ProviderReadiness enum |

## Priority

Low-medium. Current behavior works for known-good configs. Becomes important
when users switch between providers or when the IDE wants to auto-select the
best available provider.
