# W4-B: Structured Provider Readiness Reporting

## Context

Provider readiness is reported as freeform description strings:
- "Ready" / "API key env ANTHROPIC_API_KEY is not set" / "Unavailable"

The IDE has to parse these strings to determine if a provider is usable. A structured
`ready: bool` field lets clients filter and display provider state directly without
string parsing.

The readiness check already exists: `RokoConfig::is_provider_available()` (schema.rs:368-379)
returns `bool`. We just need to wire it into the JSON response.

## Prerequisites

None. This batch modifies independent code paths.

## File Locations

Two files:
1. `/Users/will/dev/nunchi/roko/roko/crates/roko-acp/src/types.rs` — ConfigOptionValue struct
2. `/Users/will/dev/nunchi/roko/roko/crates/roko-acp/src/session.rs` — build_config_options (provider options)

## Change 1: Add `ready` field to ConfigOptionValue

**File:** `crates/roko-acp/src/types.rs`

The current ConfigOptionValue (lines 586-597):
```rust
/// One selectable config option value.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigOptionValue {
    /// Serialized value.
    pub value: String,
    /// User-facing value name.
    pub name: String,
    /// Optional value description.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}
```

FIND (lines 586-597):
```rust
/// One selectable config option value.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigOptionValue {
    /// Serialized value.
    pub value: String,
    /// User-facing value name.
    pub name: String,
    /// Optional value description.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}
```

REPLACE WITH:
```rust
/// One selectable config option value.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigOptionValue {
    /// Serialized value.
    pub value: String,
    /// User-facing value name.
    pub name: String,
    /// Optional value description.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Whether this option is usable (e.g., provider has API key configured).
    /// When `true`, serialized in JSON. When `false`, omitted from JSON (default).
    /// Old clients that don't know about this field see no change.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub ready: bool,
}
```

**Serde behavior:**
- `#[serde(default)]` → deserializes as `false` when absent (backward compatible for reading)
- `skip_serializing_if = "std::ops::Not::not"` → skips serialization when `!ready` is true
  (i.e., when ready is false). This means `"ready": true` is always in JSON, but `"ready": false`
  is omitted. Old clients see no new fields for non-ready providers.

## Change 2: Set `ready` from is_provider_available in provider options

**File:** `crates/roko-acp/src/session.rs`

In `build_config_options` (lines 948-957), provider options are constructed:

```rust
    // ── Provider options from [providers.*] in roko.toml, with availability status ──
    let mut provider_options: Vec<ConfigOptionValue> = roko_config
        .providers
        .iter()
        .map(|(key, provider)| ConfigOptionValue {
            value: key.clone(),
            name: capitalize_model_key(key),
            description: provider_option_description(roko_config, provider),
        })
        .collect();
    provider_options.sort_by(|a, b| a.value.cmp(&b.value));
```

FIND (lines 948-958):
```rust
    // ── Provider options from [providers.*] in roko.toml, with availability status ──
    let mut provider_options: Vec<ConfigOptionValue> = roko_config
        .providers
        .iter()
        .map(|(key, provider)| ConfigOptionValue {
            value: key.clone(),
            name: capitalize_model_key(key),
            description: provider_option_description(roko_config, provider),
        })
        .collect();
    provider_options.sort_by(|a, b| a.value.cmp(&b.value));
```

REPLACE WITH:
```rust
    // ── Provider options from [providers.*] in roko.toml, with availability status ──
    let mut provider_options: Vec<ConfigOptionValue> = roko_config
        .providers
        .iter()
        .map(|(key, provider)| ConfigOptionValue {
            value: key.clone(),
            name: capitalize_model_key(key),
            description: provider_option_description(roko_config, provider),
            ready: roko_config.is_provider_available(provider),
        })
        .collect();
    provider_options.sort_by(|a, b| a.value.cmp(&b.value));
```

## Change 3: Set `ready` on model options too

Models should also have a `ready` field based on whether their provider is ready.

In `build_config_options` (lines 960-971), model options are constructed. After W4-A
modifies the description, the map closure looks like:

```rust
        .map(|(key, profile)| {
            let max_out = profile.effective_max_output();
            ConfigOptionValue {
                value: key.clone(),
                name: capitalize_model_key(key),
                description: Some(format!("{} (max output: {})", profile.slug, max_out)),
            }
        })
```

If W4-A has been applied, FIND:
```rust
        .map(|(key, profile)| {
            let max_out = profile.effective_max_output();
            ConfigOptionValue {
                value: key.clone(),
                name: capitalize_model_key(key),
                description: Some(format!("{} (max output: {})", profile.slug, max_out)),
            }
        })
```

REPLACE WITH:
```rust
        .map(|(key, profile)| {
            let max_out = profile.effective_max_output();
            let provider_ready = roko_config
                .providers
                .get(&profile.provider)
                .map(|p| roko_config.is_provider_available(p))
                .unwrap_or(false);
            ConfigOptionValue {
                value: key.clone(),
                name: capitalize_model_key(key),
                description: Some(format!("{} (max output: {})", profile.slug, max_out)),
                ready: provider_ready,
            }
        })
```

If W4-A has NOT been applied, FIND (the original code):
```rust
        .map(|(key, profile)| ConfigOptionValue {
            value: key.clone(),
            name: capitalize_model_key(key),
            description: Some(profile.slug.clone()),
        })
```

REPLACE WITH:
```rust
        .map(|(key, profile)| {
            let provider_ready = roko_config
                .providers
                .get(&profile.provider)
                .map(|p| roko_config.is_provider_available(p))
                .unwrap_or(false);
            ConfigOptionValue {
                value: key.clone(),
                name: capitalize_model_key(key),
                description: Some(profile.slug.clone()),
                ready: provider_ready,
            }
        })
```

## Change 4: Add `ready: false` to all other ConfigOptionValue construction sites

Search the codebase for other places that construct `ConfigOptionValue`. Each one needs
`ready: false` (or an appropriate readiness check) added to the struct literal.

```bash
grep -rn 'ConfigOptionValue {' crates/roko-acp/src/ --include='*.rs' | grep -v target/
```

Any sites building ConfigOptionValue for non-provider/non-model options (like effort,
temperament, routing_mode) should use `ready: true` since those options are always available:

```rust
ConfigOptionValue {
    value: "...".to_owned(),
    name: "...".to_owned(),
    description: Some("...".to_owned()),
    ready: true,  // always available
}
```

## Wire Format

After this change, the IDE receives:
```json
{
  "id": "provider",
  "options": [
    {"name": "Openai", "value": "openai", "description": "Ready", "ready": true},
    {"name": "Anthropic", "value": "anthropic", "description": "API key env ANTHROPIC_API_KEY is not set"}
  ]
}
```

Note: `"ready": false` is omitted from JSON entirely (due to `skip_serializing_if`).
Providers where `ready` is true have `"ready": true` in the response.

## What NOT to Change

- Do NOT modify `ConfigOption` struct (the parent that holds `Vec<ConfigOptionValue>`)
- Do NOT modify `is_provider_available()` in schema.rs
- Do NOT modify `provider_option_description()` — keep the text description alongside the bool
- Do NOT remove the description field — it still provides useful context to the IDE

## Verification

After Phase 2:
```bash
echo '{"jsonrpc":"2.0","method":"session/new","id":1,"params":{}}' \
  | cargo run -p roko-cli -- acp --quiet --no-serve --config ~/.nunchi/roko/roko.toml 2>/dev/null \
  | head -1 | python3 -c "
import sys,json; d=json.load(sys.stdin)
for o in d['result'].get('configOptions') or []:
  if o['id']=='provider':
    for opt in o.get('options', []):
      print(f\"{opt['value']}: ready={opt.get('ready', False)}\")
"
# Should show: "openai: ready=True", "anthropic: ready=False", etc.
# Providers with API keys set show ready=True
```

## Estimated Effort

15-20 minutes. One struct field + setting it at 2-3 construction sites.
