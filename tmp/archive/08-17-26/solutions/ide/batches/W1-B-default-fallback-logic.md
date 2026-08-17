# W1-B: Fix Default Model/Provider Fallback Logic

## Context

When `roko acp` starts a session, it picks a default model and provider using
`SessionConfigState::from_roko_config()`. If the configured `agent.default_model` doesn't
match any key in `[models.*]`, the fallback picks `config.models.keys().next()` — which
after W0-A (IndexMap) is the first TOML-declared model. This batch improves the fallback to
prefer a model whose provider is actually ready (has API keys).

## Prerequisite

**W0-A must be completed** — the IndexMap migration makes `.keys().next()` deterministic,
but this batch makes the fallback smarter.

## File Location

**One file:** `/Users/will/dev/nunchi/roko/roko/crates/roko-acp/src/session.rs`

## Change 1: Rewrite from_roko_config (lines 175-204)

FIND this exact function:
```rust
impl SessionConfigState {
    /// Create config state from roko.toml values.
    pub fn from_roko_config(config: &roko_core::config::schema::RokoConfig) -> Self {
        let configured_default = config.agent.default_model.trim();
        let default_model =
            if !configured_default.is_empty() && config.models.contains_key(configured_default) {
                Some(configured_default)
            } else {
                config.models.keys().next().map(String::as_str)
            };
        // Derive the default provider from the default model's profile.
        let default_provider = default_model
            .and_then(|model| config.models.get(model))
            .map(|profile| profile.provider.clone())
            .or_else(|| config.providers.keys().next().cloned())
            .unwrap_or_default();
        Self {
            agent_mode: "code".to_owned(),
            provider: default_provider,
            model: default_model.unwrap_or_default().to_owned(),
            effort: config.agent.default_effort.clone(),
            temperament: config.agent.temperament.label().to_owned(),
            routing_mode: config.routing.mode.clone(),
            clippy_enabled: config.gates.clippy_enabled,
            tests_enabled: !config.gates.skip_tests,
            workflow: "none".to_owned(),
            review_strictness: "none".to_owned(),
            max_iterations: 2,
        }
    }
}
```

REPLACE WITH:
```rust
impl SessionConfigState {
    /// Create config state from roko.toml values.
    pub fn from_roko_config(config: &roko_core::config::schema::RokoConfig) -> Self {
        let configured_default = config.agent.default_model.trim();
        let default_model =
            if !configured_default.is_empty() && config.models.contains_key(configured_default) {
                Some(configured_default)
            } else {
                if !configured_default.is_empty() {
                    tracing::warn!(
                        configured = configured_default,
                        "agent.default_model not found in [models.*], falling back"
                    );
                }
                // Prefer the first model (TOML declaration order) whose provider is ready.
                config
                    .models
                    .iter()
                    .find(|(_, profile)| {
                        config
                            .providers
                            .get(&profile.provider)
                            .map(|p| config.is_provider_available(p))
                            .unwrap_or(false)
                    })
                    .map(|(key, _)| key.as_str())
                    // Fall back to the first model regardless of provider readiness.
                    .or_else(|| config.models.keys().next().map(String::as_str))
            };
        // Derive the default provider from the default model's profile.
        let default_provider = default_model
            .and_then(|model| config.models.get(model))
            .map(|profile| profile.provider.clone())
            .or_else(|| {
                // Prefer the first ready provider in declaration order.
                config
                    .providers
                    .iter()
                    .find(|(_, p)| config.is_provider_available(p))
                    .map(|(k, _)| k.clone())
                    .or_else(|| config.providers.keys().next().cloned())
            })
            .unwrap_or_default();
        Self {
            agent_mode: "code".to_owned(),
            provider: default_provider,
            model: default_model.unwrap_or_default().to_owned(),
            effort: config.agent.default_effort.clone(),
            temperament: config.agent.temperament.label().to_owned(),
            routing_mode: config.routing.mode.clone(),
            clippy_enabled: config.gates.clippy_enabled,
            tests_enabled: !config.gates.skip_tests,
            workflow: "none".to_owned(),
            review_strictness: "none".to_owned(),
            max_iterations: 2,
        }
    }
}
```

## What Changed

1. When `agent.default_model` is set but doesn't match any `[models.*]` key, log a warning
2. Fallback model selection: prefer the first model whose provider is "ready" (API key set)
3. If no provider is ready, fall back to the first model in declaration order (IndexMap preserves TOML order)
4. Provider fallback: same pattern — prefer the first ready provider

## What NOT to Change

- Do NOT modify `update_config` (lines 560-652) — it uses `.min()` which is deterministic
- Do NOT modify `revalidate_config_state` (lines 655-704) — it delegates to from_roko_config
- Do NOT touch any other files

## Verification

After Phase 2:
```bash
# Create a config with agent.model pointing to nonexistent key, run 5 times
for i in {1..5}; do
  echo '{"jsonrpc":"2.0","method":"session/new","id":1,"params":{}}' \
    | cargo run -p roko-cli -- acp --quiet --no-serve --config ~/.nunchi/roko/roko.toml 2>/dev/null \
    | head -1 | python3 -c "
import sys,json; d=json.load(sys.stdin)
for o in d['result'].get('configOptions') or []:
  if o['id']=='model': print(o['currentValue'])
" 2>/dev/null
done | sort -u | wc -l
# Must print "1" (same default every time)
```

## Estimated Effort

15-20 minutes. One function rewrite.
