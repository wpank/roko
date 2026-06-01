# W5-A: Auth Detection Uses Unified Config

**Priority**: P1 — prevents setup failures
**Effort**: 1-2 hours
**Files to modify**: 1 file
**Dependencies**: None

## Problem

Banner says "auth: glm-5.1 (OpenAI-compat)" but dispatch fails with "no API key for provider 'anthropic_api'". Two independent systems:
1. `detect_auth()` probes env vars independently (Claude CLI → ANTHROPIC_API_KEY → ZAI_API_KEY → OPENAI_API_KEY)
2. `ChatAgentSession` loads roko.toml, uses cascade router, picks default model which may point to a different provider

## Root Cause

`detect_auth()` in `crates/roko-cli/src/auth_detect.rs` (lines 69-111) probes env vars in a hardcoded priority order. It never reads `roko.toml`. The actual dispatch path reads `roko.toml` and picks a model/provider based on cascade routing. These systems never talk to each other.

## Fix

`detect_auth()` should load the unified config, check which configured providers have valid credentials, and return the provider that will ACTUALLY be used for dispatch.

### File: `crates/roko-cli/src/auth_detect.rs`

### New approach

```rust
use roko_core::config::schema::RokoConfig;

/// Detect which auth method will ACTUALLY be used for dispatch.
/// Loads roko.toml if available, checks configured providers' API keys.
pub fn detect_auth_from_config(workdir: &Path) -> AuthMethod {
    // 1. Try loading roko.toml config
    if let Ok(config) = roko_core::config::loader::load_config_unified(workdir) {
        if let Some(method) = detect_from_config(&config) {
            return method;
        }
    }

    // 2. Fallback to env var probing (for unconfigured workspaces)
    detect_auth_from_env()
}

fn detect_from_config(config: &RokoConfig) -> Option<AuthMethod> {
    // Check each configured provider for valid credentials
    for (name, provider) in &config.providers {
        let has_key = provider.api_key_env.as_ref()
            .map(|env| std::env::var(env).map(|k| !k.is_empty()).unwrap_or(false))
            .unwrap_or(false);

        if has_key {
            return Some(match provider.kind.as_deref() {
                Some("claude_cli") => AuthMethod::ClaudeCli,
                Some("anthropic_api") => AuthMethod::AnthropicApi {
                    key: std::env::var(provider.api_key_env.as_ref().unwrap()).unwrap(),
                    model: provider.default_model.clone(),
                },
                _ => AuthMethod::OpenAiCompat {
                    key: std::env::var(provider.api_key_env.as_ref().unwrap()).unwrap(),
                    base_url: provider.base_url.clone().unwrap_or_default(),
                    model: provider.default_model.clone(),
                },
            });
        }
    }
    None
}

/// Legacy: pure env var probing (no config dependency).
/// Rename existing detect_auth() to this.
pub fn detect_auth_from_env() -> AuthMethod {
    // ... existing detect_auth() body (lines 69-111), unchanged
}
```

### Update callers

Search for all callers of `detect_auth()`:
```bash
grep -rn 'detect_auth()' crates/roko-cli/src/ --include='*.rs'
```

Change each to `detect_auth_from_config(&workdir)` where workdir is available. If workdir isn't available at the call site, keep `detect_auth_from_env()`.

## Key Design Decision

This is a **proper fix** (config-aware detection), not a duck-tape fix (hardcoding more env vars). It ensures the auth banner matches what dispatch will actually use.

## Agent Prompt

```
Read /Users/will/dev/nunchi/roko/roko/tmp/solutions/demo-running/batches/W5-A-auth-detect-config.md and implement all changes described in it. Create detect_auth_from_config() in crates/roko-cli/src/auth_detect.rs that loads roko.toml, checks configured providers' API keys, and returns the first provider with valid credentials. Rename old detect_auth() to detect_auth_from_env() as fallback. Update all callers. Do NOT run cargo build/test/clippy/fmt — compilation is deferred. Mark the checklist items as done.
```

## Commit

This batch is committed with all Wave 5 batches together. Do not commit individually.

## Verification (deferred to Phase 2)

After compilation: auth banner matches actual dispatch provider.

## Checklist

- [x] Create `detect_auth_from_config(workdir: &Path)` that loads roko.toml
- [x] Check configured providers' API key env vars for valid credentials
- [x] Return the FIRST configured provider with valid credentials
- [x] Rename old `detect_auth()` to `detect_auth_from_env()` as fallback
- [x] Update all callers to use config-aware version
- [ ] Verify: auth banner matches actual dispatch provider
- [ ] Pre-commit checks pass
