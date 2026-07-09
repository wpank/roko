# Provider Error UX

## Problem

User has no Anthropic API key, uses OpenAI-compatible providers (default: `gpt54-mini`).
Despite this, roko constantly warns about missing ANTHROPIC_API_KEY.

## Root Causes

### A. Doctor check is unconditional

`crates/roko-cli/src/doctor.rs:186`:
```rust
checks.push(check_anthropic_api_key());  // Always runs regardless of config
```

`doctor.rs:720-747` — `check_anthropic_api_key()`:
- Checks `ANTHROPIC_API_KEY` env var regardless of whether anthropic is configured
- Returns WARN if missing
- Should only warn if anthropic is actually a configured provider

### B. Error message is anthropic-biased

`crates/roko-cli/src/unified.rs:279`:
```
"no authentication configured — run `roko config init` or set ANTHROPIC_API_KEY"
```

This appears when ALL auth methods fail but specifically names Anthropic.
Should be provider-agnostic.

### C. Auth detection fallback chain

`crates/roko-cli/src/auth_detect.rs:121-163`:
1. `claude` CLI (binary on PATH)
2. `ANTHROPIC_API_KEY`
3. `ZAI_API_KEY`
4. `OPENAI_API_KEY`
5. `NeedsSetup`

The chain itself works correctly (checks everything), but individual check failures
may produce log noise about missing Anthropic keys.

## What Actually Works

The dispatch system is fine — it correctly routes to configured providers:
- `bootstrap.rs:87-134` — `validate_provider_available()` returns Ok if ANY provider found
- `run.rs:2075-2085` — only uses Anthropic API path if `command == "claude" && has_anthropic_api_key()`
- Provider pre-flight (`provider/pre_flight.rs:95-131`) — only checks configured providers

The errors are purely UX/messaging bugs, not architectural issues.

## Fix Plan

### Fix 1: Dynamic doctor checks (~15 min)

`crates/roko-cli/src/doctor.rs`:

Replace unconditional `check_anthropic_api_key()` with:
```rust
// Only check API keys for providers actually configured in roko.toml
if let Ok(config) = load_config() {
    for (name, provider) in &config.providers {
        if let Some(env_var) = &provider.api_key_env {
            if std::env::var(env_var).is_err() {
                checks.push(warn(format!("{} not set (provider: {})", env_var, name)));
            }
        }
    }
}
```

### Fix 2: Provider-agnostic error message (~5 min)

`crates/roko-cli/src/unified.rs:279`:

Change to:
```
"no authentication configured — run `roko setup` or set a provider API key (OPENAI_API_KEY, GEMINI_API_KEY, etc.)"
```

### Fix 3: Suppress noise for unconfigured providers (~30 min)

`crates/roko-cli/src/auth_detect.rs`:
- Don't log warnings about individual auth method failures during detection
- Only log at DEBUG level
- Final NeedsSetup path should list what was checked without implying any specific provider

### Fix 4: Add available provider detection to doctor

New check: report all detected providers (configured + env vars + CLIs on PATH):
```
[ok] providers_detected: 3 providers available (openai, gemini-cli, claude-cli)
```

## Files to Modify

| File | Change |
|------|--------|
| `crates/roko-cli/src/doctor.rs:186,720-747` | Conditional provider key checks |
| `crates/roko-cli/src/unified.rs:279` | Provider-agnostic error message |
| `crates/roko-cli/src/auth_detect.rs:121-163` | Suppress individual failure noise |
| `crates/roko-cli/src/bootstrap.rs` | Consistent detection logic |
