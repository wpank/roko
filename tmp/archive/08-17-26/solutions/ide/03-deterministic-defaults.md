# Issue: Non-Deterministic Model/Provider Defaults

## Problem Statement

When `session/new` creates a session and the config's `agent.model` (aka `default_model`)
doesn't match any key in `[models.*]`, the fallback picks `config.models.keys().next()`
— the first key from a HashMap. HashMap iteration order is non-deterministic in Rust.

This means the same config file can produce different default models across runs.

## Reproduction

Config with these models (no explicit `agent.model` set, or set to something invalid):
```toml
[models.sonnet]
provider = "openai"
slug = "gpt-4o"

[models.haiku]
provider = "openai"
slug = "gpt-4o-mini"

[models.gemini-2-5-flash]
provider = "gemini"
slug = "gemini-2.5-flash"
```

Observed: requesting `model: "nonexistent-model"` produced `currentValue: "gemini"`
as the provider — the session picked `gemini-2-5-flash` because it happened to be
the first HashMap key in that run.

## Root Cause

`crates/roko-acp/src/session.rs` (lines 182-185):

```rust
let default_model =
    if !configured_default.is_empty() && config.models.contains_key(configured_default) {
        Some(configured_default)
    } else {
        config.models.keys().next().map(String::as_str)  // Non-deterministic!
    };
```

And for provider (lines 186-190):
```rust
let default_provider = default_model
    .and_then(|model| config.models.get(model))
    .map(|profile| profile.provider.clone())
    .or_else(|| config.providers.keys().next().cloned())  // Also non-deterministic!
    .unwrap_or_default();
```

## Impact

- Different default model each time the process restarts
- IDE thinks it's talking to model X but might get model Y
- Impossible to write reliable integration tests
- Confusing UX: session says "currentValue: gemini" when user configured openai

## Proposed Solution

### Use BTreeMap or IndexMap for config maps

Replace `HashMap<String, ModelProfile>` and `HashMap<String, ProviderConfig>` with
ordered collections that preserve insertion order (IndexMap) or sort deterministically
(BTreeMap):

```rust
// In crates/roko-core/src/config/schema.rs
use indexmap::IndexMap;

pub struct RokoConfig {
    pub providers: IndexMap<String, ProviderConfig>,
    pub models: IndexMap<String, ModelProfile>,
    // ...
}
```

IndexMap with serde preserves TOML declaration order. BTreeMap sorts alphabetically.
Either is better than HashMap — IndexMap is preferred because it respects the user's
intent (first-declared model is the default).

### Add explicit fallback behavior

If `agent.model` is not set or doesn't resolve, use a well-defined priority:

```rust
let default_model = if !configured_default.is_empty()
    && config.models.contains_key(configured_default)
{
    Some(configured_default)
} else {
    // Priority: first model whose provider is ready, in declaration order
    config.models.iter()
        .find(|(_, profile)| provider_is_ready(&config.providers, &profile.provider))
        .map(|(key, _)| key.as_str())
        .or_else(|| config.models.keys().next().map(String::as_str))
};
```

### Emit a warning when falling back

When the configured default model doesn't exist:

```rust
if !configured_default.is_empty() && !config.models.contains_key(configured_default) {
    warn!(
        configured = configured_default,
        fallback = %default_model.unwrap_or("none"),
        "configured agent.model not found in [models.*], using fallback"
    );
}
```

## Implementation Location

| File | Change |
|------|--------|
| `crates/roko-core/src/config/schema.rs` | Change HashMap to IndexMap for providers/models |
| `crates/roko-acp/src/session.rs:177-204` | Explicit fallback logic with ready-check |
| `Cargo.toml` (roko-core) | Add `indexmap = { version = "2", features = ["serde"] }` |

## Migration

- IndexMap's serde impl is drop-in compatible with HashMap
- Existing TOML configs parse identically
- Only behavioral change: iteration order becomes deterministic
- No config file changes needed

## Alternatives Considered

| Approach | Tradeoff |
|----------|----------|
| BTreeMap | Deterministic but alphabetical — user can't control priority by declaration order |
| IndexMap | Preserves TOML order — user's first `[models.*]` is the natural default |
| Explicit `default_model` required | Breaking change, config migration needed |

Recommendation: IndexMap + require `agent.model` to be valid (warn if not, don't silently pick random).

## Verification

After implementing the fix, run:

```bash
cd tmp/solutions/ide/tests && ./test-models.sh
```

The "nonexistent model" test should change from WARN to PASS (returns error or warning).

Manual verification:
```bash
# Create config with agent.model pointing to nonexistent key
cat > /tmp/test_ordering.toml << 'EOF'
config_version = 2
schema_version = 2
[project]
name = "ordering-test"
[serve]
port = 6699
[agent]
command = "cat"
model = "does-not-exist"
timeout_ms = 60000
[providers.openai]
kind = "openai_compat"
base_url = "https://api.openai.com/v1"
api_key_env = "OPENAI_API_KEY"
[models.zebra]
provider = "openai"
slug = "gpt-4o"
[models.alpha]
provider = "openai"
slug = "gpt-4o-mini"
EOF

# Run 5 times, check if default is always the same
for i in {1..5}; do
  echo '{"jsonrpc":"2.0","method":"session/new","id":1,"params":{}}' \
    | roko acp --quiet --no-serve --config /tmp/test_ordering.toml 2>/dev/null \
    | head -1 | python3 -c "
import sys,json; d=json.load(sys.stdin)
for o in d['result']['configOptions']:
  if o['id']=='model': print(o['currentValue'])
"
done

# BEFORE fix: may print different values each run (non-deterministic)
# AFTER fix: always prints the same value (first declared in TOML)
```
