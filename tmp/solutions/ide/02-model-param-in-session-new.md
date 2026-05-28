# Issue: session/new Ignores Model Parameter

## Problem Statement

The IDE sends `{"method":"session/new","params":{"model":"sonnet",...}}` expecting the
session to use the "sonnet" model. However, `SessionNewParams` has no `model` field —
the parameter is silently dropped during deserialization. The session's model is derived
entirely from config defaults.

## Reproduction

```bash
# Request model "nonexistent-model"
echo '{"jsonrpc":"2.0","method":"session/new","id":1,"params":{"model":"nonexistent-model"}}' \
  | roko acp --quiet --no-serve --config /tmp/test.toml

# Result: session/new SUCCEEDS
# configOptions shows currentValue is whatever the config defaults to
# (first model key alphabetically from HashMap iteration)
# No error about "nonexistent-model" being invalid
```

Tested: requesting `model: "nonexistent-model"` with a config that has only `[models.sonnet]`
resulted in the session defaulting to "gemini" provider (from built-in fallback, not config).

## Root Cause

`crates/roko-acp/src/types.rs` (lines 240-253):

```rust
pub struct SessionNewParams {
    pub session_name: Option<String>,
    pub client_capabilities: Option<ClientCapabilities>,
    pub mcp_servers: Vec<McpServerConfig>,
    // NOTE: no `model` field!
}
```

Serde's default behavior with `#[serde(deny_unknown_fields)]` not set means extra
fields like `model` are silently ignored during deserialization.

The model selection is derived in `SessionConfigState::from_roko_config()` (session.rs:177-204):

```rust
let default_model =
    if !configured_default.is_empty() && config.models.contains_key(configured_default) {
        Some(configured_default)
    } else {
        config.models.keys().next().map(String::as_str)  // HashMap first key = non-deterministic!
    };
```

## Impact

- IDE's model selection in session/new params has NO effect
- The IDE must send a separate `session/config/update` call after session creation
- Extra round-trip, race condition between session creation and config update
- If a consumer expects session/new to respect the model param (natural assumption), they get unexpected model behavior

## Proposed Solution

### Add `model` and `provider` to SessionNewParams

```rust
#[derive(Deserialize)]
pub struct SessionNewParams {
    pub session_name: Option<String>,
    pub client_capabilities: Option<ClientCapabilities>,
    pub mcp_servers: Vec<McpServerConfig>,

    // New fields:
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub provider: Option<String>,
    #[serde(default)]
    pub effort: Option<String>,
}
```

### Apply params during session creation

In `SessionManager::create_session()` (session.rs:753+):

```rust
pub fn create_session(&self, params: SessionNewParams) -> SessionNewResult {
    let mut config_state = SessionConfigState::from_roko_config(&self.config);

    // Override with params if provided
    if let Some(ref model) = params.model {
        if self.config.models.contains_key(model) {
            config_state.model = model.clone();
            // Also update provider to match
            if let Some(profile) = self.config.models.get(model) {
                config_state.provider = profile.provider.clone();
            }
        } else {
            // Return error: model not found
            // Or: include a warning in the response
        }
    }
    if let Some(ref provider) = params.provider {
        if self.config.providers.contains_key(provider) {
            config_state.provider = provider.clone();
        }
    }
    // ...
}
```

### Validation behavior

Two options:

**Option A (lenient — recommended for IDE use):** Accept unknown model, fall back to default,
include a `warnings` field in the response:

```json
{
  "result": {
    "sessionId": "sess_...",
    "configOptions": [...],
    "warnings": ["requested model 'foo' not found, using default 'sonnet'"]
  }
}
```

**Option B (strict):** Return JSON-RPC error for unknown model:

```json
{"error": {"code": -32602, "message": "model 'foo' is not configured"}}
```

Option A is better for IDE resilience — the session still starts, but the client
knows something was off.

## Implementation Location

| File | Change |
|------|--------|
| `crates/roko-acp/src/types.rs:240-253` | Add model/provider/effort to SessionNewParams |
| `crates/roko-acp/src/session.rs:753+` | Apply params during create_session |
| `crates/roko-acp/src/session.rs:177-204` | from_roko_config stays as default fallback |

## Backward Compatibility

- New fields are `Option<T>` with `#[serde(default)]` — old clients sending no model still work
- Existing behavior (use config defaults) is preserved when fields are absent
- Response schema extension (adding `warnings`) is non-breaking

## Verification

After implementing the fix, run:

```bash
cd tmp/solutions/ide/tests && ./test-models.sh
```

The following test should change from FAIL to PASS:
- "session/new respects model param"

Manual verification:
```bash
# Create a config with both sonnet and haiku models, then:
echo '{"jsonrpc":"2.0","method":"session/new","id":1,"params":{"model":"haiku"}}' \
  | roko acp --quiet --no-serve --config ~/.nunchi/roko/roko.toml 2>/dev/null \
  | head -1 | python3 -c "
import sys,json; d=json.load(sys.stdin)
for o in d['result']['configOptions']:
  if o['id']=='model': print(f\"currentValue: {o['currentValue']}\")
"

# BEFORE fix: prints "currentValue: sonnet" (ignores model param)
# AFTER fix: prints "currentValue: haiku"
```
