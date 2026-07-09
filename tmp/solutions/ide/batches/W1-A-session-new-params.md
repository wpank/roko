# W1-A: Add model/provider/effort to SessionNewParams

## Context

The `roko-acp` crate implements a JSON-RPC stdio server that IDEs connect to. When an IDE
creates a session via `session/new`, it sends a `model` parameter — but the server ignores
it because `SessionNewParams` has no `model` field. Serde silently drops unknown fields.

This batch adds `model`, `provider`, and `effort` to `SessionNewParams`, applies them
during session creation, and adds a `warnings` field to the response for soft errors.

## File Locations

All changes are in **one crate**: `/Users/will/dev/nunchi/roko/roko/crates/roko-acp/src/`

## Change 1: Extend SessionNewParams

**File:** `/Users/will/dev/nunchi/roko/roko/crates/roko-acp/src/types.rs`

FIND (lines 240-253):
```rust
/// Parameters for creating a new ACP session.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionNewParams {
    /// Optional client-supplied session name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_name: Option<String>,
    /// Optional client capabilities for the session.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub client_capabilities: Option<ClientCapabilities>,
    /// MCP servers attached to this session.
    #[serde(default)]
    pub mcp_servers: Vec<McpServerConfig>,
}
```

REPLACE WITH:
```rust
/// Parameters for creating a new ACP session.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionNewParams {
    /// Optional client-supplied session name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_name: Option<String>,
    /// Optional client capabilities for the session.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub client_capabilities: Option<ClientCapabilities>,
    /// MCP servers attached to this session.
    #[serde(default)]
    pub mcp_servers: Vec<McpServerConfig>,
    /// Model key (must match a key in `[models.*]` in roko.toml).
    /// If absent or invalid, the config default is used.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// Provider key (must match a key in `[providers.*]` in roko.toml).
    /// If absent, derived from the selected model's profile.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    /// Effort level: "low", "medium", "high", "max".
    /// If absent, uses the config default.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effort: Option<String>,
}
```

## Change 2: Extend SessionNewResult with warnings

**File:** `/Users/will/dev/nunchi/roko/roko/crates/roko-acp/src/types.rs`

FIND (lines 283-295):
```rust
/// Result returned from `session/new`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionNewResult {
    /// Server-generated session identifier.
    pub session_id: String,
    /// Available interaction modes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub modes: Option<ModesInfo>,
    /// Session configuration options.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub config_options: Option<Vec<ConfigOption>>,
}
```

REPLACE WITH:
```rust
/// Result returned from `session/new`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionNewResult {
    /// Server-generated session identifier.
    pub session_id: String,
    /// Available interaction modes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub modes: Option<ModesInfo>,
    /// Session configuration options.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub config_options: Option<Vec<ConfigOption>>,
    /// Non-fatal warnings from session creation (e.g., unknown model name).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<String>,
}
```

## Change 3: Apply params in create_session

**File:** `/Users/will/dev/nunchi/roko/roko/crates/roko-acp/src/session.rs`

FIND (lines 753-761):
```rust
    /// Creates and stores a new ACP session.
    pub fn create_session(&mut self, params: SessionNewParams) -> SessionNewResult {
        let mut session = AcpSession::new_with_config(params, &self.roko_config);
        session.cached_conventions = AcpSession::load_conventions(&self.workdir);
        session.always_allowed = AcpSession::load_workspace_trust(&self.workdir);
        let result = session.new_result();
        self.sessions.insert(session.session_id.clone(), session);
        result
    }
```

REPLACE WITH:
```rust
    /// Creates and stores a new ACP session.
    pub fn create_session(&mut self, params: SessionNewParams) -> SessionNewResult {
        let mut warnings: Vec<String> = Vec::new();
        let mut session = AcpSession::new_with_config(params.clone(), &self.roko_config);

        // Apply model override from params.
        if let Some(ref model_key) = params.model {
            if self.roko_config.models.contains_key(model_key.as_str()) {
                session.config_state.model = model_key.clone();
                // Derive provider from the model's profile.
                if let Some(profile) = self.roko_config.models.get(model_key.as_str()) {
                    session.config_state.provider = profile.provider.clone();
                }
            } else {
                warnings.push(format!(
                    "requested model '{}' not found in config, using default '{}'",
                    model_key, session.config_state.model
                ));
            }
        }

        // Apply explicit provider override (takes precedence over model-derived).
        if let Some(ref provider_key) = params.provider {
            if self.roko_config.providers.contains_key(provider_key.as_str()) {
                session.config_state.provider = provider_key.clone();
            } else {
                warnings.push(format!(
                    "requested provider '{}' not found in config, using '{}'",
                    provider_key, session.config_state.provider
                ));
            }
        }

        // Apply effort override.
        if let Some(ref effort) = params.effort {
            session.config_state.effort = effort.clone();
        }

        session.cached_conventions = AcpSession::load_conventions(&self.workdir);
        session.always_allowed = AcpSession::load_workspace_trust(&self.workdir);

        // Rebuild config options to reflect overrides.
        session.config_options = build_config_options(&session.config_state, &self.roko_config);

        let mut result = session.new_result();
        result.warnings = warnings;
        self.sessions.insert(session.session_id.clone(), session);
        result
    }
```

## Change 4: Ensure new_result() initializes warnings

Find `fn new_result(&self) -> SessionNewResult` in session.rs. It should construct the
result struct. After the change to SessionNewResult, you need to add `warnings: Vec::new()`
to the construction. Search for `SessionNewResult {` in session.rs and ensure the `warnings`
field is initialized:

```rust
SessionNewResult {
    session_id: self.session_id.clone(),
    modes: ...,
    config_options: ...,
    warnings: Vec::new(),  // ADD THIS LINE
}
```

## What NOT to Change

- Do NOT modify `from_roko_config` — that's W1-B's job
- Do NOT modify `build_config_options` — that's W4-A/B's job
- Do NOT modify any test files for this batch

## Verification

After Phase 2 (compilation), test manually:
```bash
echo '{"jsonrpc":"2.0","method":"session/new","id":1,"params":{"model":"haiku"}}' \
  | cargo run -p roko-cli -- acp --quiet --no-serve --config ~/.nunchi/roko/roko.toml 2>/dev/null \
  | head -1 | python3 -c "
import sys,json; d=json.load(sys.stdin)
for o in d['result'].get('configOptions') or []:
  if o['id']=='model': print(f\"model: {o['currentValue']}\")
print(f\"warnings: {d['result'].get('warnings', [])}\")
"
```

Expected: `model: haiku` (if haiku exists in config) or a warning message.

## Estimated Effort

30-40 minutes.
