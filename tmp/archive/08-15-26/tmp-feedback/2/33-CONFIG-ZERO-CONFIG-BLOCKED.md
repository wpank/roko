# Zero-Config Blocked: Builtin Registry Not Consulted, Duplicate Slug Warnings

## Problem

The goal of zero-config is: install roko, set one API key, and everything works. Currently
blocked by two issues:

1. `preflight_provider_for_model()` doesn't consult the builtin model registry
2. 7 duplicate slug warnings printed on every command

## Root Cause

### A. Preflight doesn't consult builtin registry

**File:** `crates/roko-cli/src/commands/plan.rs` (and similar)

Before dispatching, preflight checks whether the model is valid:
```rust
fn preflight_provider_for_model(model: &str, config: &Config) -> Option<Provider> {
    // Only checks config.providers — models explicitly listed in roko.toml
    config.providers.iter()
        .find(|p| p.models.contains(&model.to_string()))
        .cloned()
}
```

**File:** `crates/roko-core/src/model_registry.rs`

The builtin registry knows about 50+ models and their providers:
```rust
pub fn lookup(model_id: &str) -> Option<ModelInfo> {
    BUILTIN_MODELS.get(model_id).cloned()
}
// e.g., "gpt-4o-mini" → { provider: "openai", context_window: 128000, ... }
```

But `preflight_provider_for_model` never calls `model_registry::lookup()`. So if a user
sets `model = "gpt-4o-mini"` without explicitly configuring an OpenAI provider, preflight
fails even though the builtin registry knows exactly which provider to use.

### B. Duplicate slug warnings

On every command invocation:
```
WARN roko_cli: Duplicate plan slug: dry-run-flag
WARN roko_cli: Duplicate plan slug: P06-process-management
... (7 warnings)
```

These come from `plan_index.rs` scanning the `plans/` directory. Some plans have both a
directory and a reference elsewhere, causing duplicate detection. The warnings are:
- Noisy (printed on every command, not just plan commands)
- Incorrect (some are false positives from path aliasing)
- Not actionable (user can't fix them without understanding internals)

## Fix

### Fix 1: Consult builtin registry in preflight (~10 min)

**File:** `crates/roko-cli/src/commands/plan.rs`

```rust
fn preflight_provider_for_model(model: &str, config: &Config) -> Option<Provider> {
    // First check explicit config
    if let Some(provider) = config.providers.iter()
        .find(|p| p.models.contains(&model.to_string())) {
        return Some(provider.clone());
    }

    // Then check builtin registry
    if let Some(model_info) = model_registry::lookup(model) {
        // Auto-configure provider from registry + available API keys
        if let Some(api_key) = env_api_key_for_provider(&model_info.provider) {
            return Some(Provider::from_registry(&model_info, &api_key));
        }
    }

    None
}
```

### Fix 2: Suppress duplicate slug warnings (~5 min)

**File:** `crates/roko-cli/src/plan_index.rs`

Option A: Deduplicate by canonical path before warning:
```rust
let canonical = plan_dir.canonicalize().unwrap_or(plan_dir.clone());
if seen.insert(canonical) {
    // not a duplicate
} else {
    // actually duplicate — but don't warn, just use the first one
}
```

Option B: Only run plan indexing when plan commands are invoked, not on every command.

### Fix 3: Auto-detect API keys for zero-config (~10 min)

**File:** `crates/roko-core/src/config/mod.rs`

```rust
pub fn env_api_key_for_provider(provider: &str) -> Option<String> {
    match provider {
        "openai" => std::env::var("OPENAI_API_KEY").ok(),
        "anthropic" => std::env::var("ANTHROPIC_API_KEY").ok(),
        "google" => std::env::var("GOOGLE_API_KEY").ok()
            .or_else(|| std::env::var("GEMINI_API_KEY").ok()),
        _ => None,
    }
}
```

## Files to Modify

| File | Change |
|------|--------|
| `crates/roko-cli/src/commands/plan.rs` | Consult model_registry in preflight |
| `crates/roko-cli/src/plan_index.rs` | Deduplicate or suppress false positive warnings |
| `crates/roko-core/src/config/mod.rs` | Auto-detect API keys |

## Priority

**P1** — Zero-config is the difference between "install and go" and "spend 20 minutes
configuring roko.toml." The builtin registry has all the knowledge needed.
