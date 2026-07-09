# Model/Provider Routing: The Problem and Fix

## The Problem

Model routing silently falls back to providers you don't have keys for, then fails at dispatch time. This is the #1 usability issue.

### How it breaks

1. **Hardcoded fallback**: If no model is configured, the system defaults to `"claude-sonnet-4-6"` everywhere — `agent.rs:111`, `orchestrate.rs:14634`, `model_selection.rs:330`. This requires `ANTHROPIC_API_KEY` but doesn't check for it.

2. **No pre-flight key validation**: Provider API key availability is checked AFTER model selection, at dispatch time. By then the agent is already configured and launched.

3. **Wrong provider inference**: `AgentBackend::from_model(slug)` infers provider from model name prefix. Unknown slugs (typos, new models) fall through to `Codex` (OpenAI), which may not have a key either. (`crates/roko-core/src/agent.rs:126-141`)

4. **50+ models configured, most without keys**: `roko.toml` lists models for every provider. CascadeRouter can route to any of them. If only 4 providers have keys, the other ~30 models are landmines.

5. **CascadeRouter doesn't know about keys**: The learned router picks models by quality/cost/latency stats but has no concept of "this provider is unavailable."

6. **No provider health check**: `ProviderHealthRegistry` records post-hoc success/failure but is never consulted before routing.

### The failure cascade

```
User runs `roko plan run` →
  model_selection.rs picks model (cascade → role → default) →
    picks "gemini-2.5-pro" (cascade learned it's good for this role) →
      no GEMINI_API_KEY set →
        dispatch starts anyway →
          HTTP call fails →
            error: "no API key for provider 'gemini': set GEMINI_API_KEY"
```

The error comes too late. The task is already in-flight, state may be partially written.

## What to Fix (implementation plan)

### Fix 1: Provider availability gate at startup

**File**: `crates/roko-core/src/config/provider.rs`

Add a method that returns only providers with valid keys:

```rust
impl ProviderConfig {
    pub fn is_available(&self) -> bool {
        match &self.api_key_env {
            Some(env_name) => std::env::var(env_name).is_ok(),
            None => true, // local providers (ollama) don't need keys
        }
    }
}
```

### Fix 2: Filter models to available providers at config load

**File**: `crates/roko-core/src/config/mod.rs`

After loading config, strip models whose provider lacks a key:

```rust
impl RokoConfig {
    pub fn available_models(&self) -> Vec<&ModelConfig> {
        self.models.values()
            .filter(|m| self.provider_available(&m.provider))
            .collect()
    }

    pub fn provider_available(&self, provider_slug: &str) -> bool {
        self.providers.get(provider_slug)
            .map(|p| p.is_available())
            .unwrap_or(false)
    }
}
```

### Fix 3: CascadeRouter only considers available models

**File**: `crates/roko-learn/src/cascade_router.rs`

Pass available model set into routing:

```rust
pub fn route(&self, ctx: &RoutingContext, available: &HashSet<String>) -> CascadeModel {
    // filter candidates to `available` before UCB1 / confidence / static
}
```

### Fix 4: Pre-dispatch validation in orchestrate.rs

**File**: `crates/roko-cli/src/orchestrate.rs`

Before `dispatch_agent_with()`, validate:

```rust
let provider = self.config.provider_for_model(&selected_model);
if !self.config.provider_available(&provider) {
    return Err(anyhow!(
        "Model '{}' requires provider '{}' but {} is not set. \
         Available providers: {:?}",
        selected_model, provider,
        self.config.providers[&provider].api_key_env.as_deref().unwrap_or("(unknown)"),
        self.config.available_provider_names()
    ));
}
```

### Fix 5: `roko config providers health` shows real status

**File**: `crates/roko-cli/src/commands/config_cmd.rs`

Make `roko config providers health` actually check key availability:

```
$ roko config providers health
Provider        Key Env              Status
anthropic       ANTHROPIC_API_KEY    ✓ key set
openai          OPENAI_API_KEY       ✓ key set
perplexity      PERPLEXITY_API_KEY   ✓ key set
gemini          GEMINI_API_KEY       ✓ key set
ollama          (none / local)       ✓ available
moonshot        MOONSHOT_API_KEY     ✗ not set
cerebras        CEREBRAS_API_KEY     ✗ not set
zhipu           ZHIPU_API_KEY        ✗ not set
```

### Fix 6: Fallback chain only includes available providers

**File**: `crates/roko-agent/src/dispatch_resolver.rs`

`fallback_candidates()` should filter by availability:

```rust
fn fallback_candidates(config: &RokoConfig, primary: &str) -> Vec<String> {
    let mut fallbacks = Vec::new();
    // ... existing logic ...
    fallbacks.retain(|model| config.provider_available_for_model(model));
    fallbacks
}
```

### Fix 7: Startup banner shows available routing

On `roko serve` or `roko plan run`, print:

```
Available providers: anthropic (3 models), openai (4 models), perplexity (2 models), gemini (2 models), ollama (local)
Default model: claude-sonnet-4-6 (anthropic) ✓
Unavailable (no key): moonshot, cerebras, zhipu — 15 models disabled
```

## Workaround (no code changes)

Strip `roko.toml` to only list providers/models you have keys for. See `05-ROKO-TOML-PRODUCTION.md`.

## Files to Change

| File | Change |
|------|--------|
| `crates/roko-core/src/config/provider.rs` | Add `is_available()` |
| `crates/roko-core/src/config/mod.rs` | Add `available_models()`, `provider_available()` |
| `crates/roko-learn/src/cascade_router.rs` | Filter by available set |
| `crates/roko-cli/src/orchestrate.rs` | Pre-dispatch validation |
| `crates/roko-cli/src/model_selection.rs` | Filter candidates |
| `crates/roko-agent/src/dispatch_resolver.rs` | Filter fallback chain |
| `crates/roko-cli/src/commands/config_cmd.rs` | Real health output |
| `crates/roko-core/src/agent.rs` | Fix `from_model()` unknown fallback |
