# S-config-6: DangerousPermissionOverride typed local-only bypass

## Task
The strict validator rejects bare `runner.dangerously_skip_permissions = true` in shared `roko.toml`. Implement the **typed local-only override**: a `DangerousPermissionOverride { reason, scope, expiry, source, acknowledgement_env }` that's the only legal way to bypass safety in a local config.

## Runner Context
Runner audit-2026-05-01, group S. Depends on S-config-2. Wave 2.

## Source plan
`tmp/subsystem-audits/implementation-plans/23-config-validation-pipeline.md` § Phase 7.

## Read first

```bash
rg 'pub struct DangerousPermissionOverride|pub struct LocalConfig' crates/roko-core/src/config/ -n
```

If `DangerousPermissionOverride` exists (C4 already shipped a skeleton), extend it. Otherwise add.

## Exact changes

### 1. Define `DangerousPermissionOverride`

`crates/roko-core/src/config/local.rs` (or `provenance.rs`):

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DangerousPermissionOverride {
    /// Free-form reason; required and non-empty.
    pub reason: String,
    /// Scope: workspace path, agent name, or process-wide.
    pub scope: OverrideScope,
    /// Optional expiry (UTC). Past expiry rejects.
    #[serde(default)]
    pub expiry: Option<chrono::DateTime<chrono::Utc>>,
    /// Source for audit (which local.toml path produced this).
    #[serde(skip)]
    pub source: ConfigSource,
    /// Env var that must be set (e.g. ROKO_ACK_DANGEROUS=yes-i-know).
    pub acknowledgement_env: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "scope", rename_all = "snake_case")]
pub enum OverrideScope {
    Process,
    Workspace { path: std::path::PathBuf },
    Agent { name: String },
}
```

### 2. Define `LocalConfig`

```rust
#[derive(Debug, Default, Deserialize)]
pub struct LocalConfig {
    /// Standard config fields that local can override.
    #[serde(flatten)]
    pub overrides: PartialRokoConfig,
    /// Dangerous-permission bypasses, one per scope.
    #[serde(default)]
    pub dangerous_overrides: Vec<DangerousPermissionOverride>,
}

#[derive(Debug, Default, Deserialize)]
pub struct PartialRokoConfig {
    // Subset of RokoConfig that can be locally overridden.
    // Use #[serde(default)] for each field.
}
```

### 3. Strict validator: reject bare flag in local

In `validate_strict_config_toml`:

```rust
pub fn validate_strict_config_toml(raw: &str, source: &StrictConfigSource) -> Result<(), StrictConfigValidationError> {
    let parsed: toml::Value = toml::from_str(raw)?;

    let dangerous_skip = parsed.get("runner")
        .and_then(|r| r.get("dangerously_skip_permissions"))
        .and_then(|v| v.as_bool());

    if matches!(dangerous_skip, Some(true)) {
        match source {
            StrictConfigSource::SharedFile => return Err(StrictConfigValidationError::BareDangerousSkipInShared),
            StrictConfigSource::LocalFile => return Err(StrictConfigValidationError::BareDangerousSkipInLocal),
        }
    }
    Ok(())
}
```

The bare flag is rejected in **both** shared and local files. The legal path is `[[dangerous_overrides]]` array.

### 4. Runtime check

When the runner enters dispatch, it checks the typed overrides:

```rust
pub fn dangerous_skip_permitted(workspace: &Path, overrides: &[DangerousPermissionOverride]) -> bool {
    overrides.iter().any(|o| {
        if let Some(expiry) = o.expiry {
            if expiry < chrono::Utc::now() { return false; }
        }
        if std::env::var(&o.acknowledgement_env).is_err() { return false; }
        match &o.scope {
            OverrideScope::Process => true,
            OverrideScope::Workspace { path } => workspace.starts_with(path),
            OverrideScope::Agent { .. } => false,  // matched at agent dispatch
        }
    })
}
```

### 5. Tests

Cover: bare flag rejected in both files, typed override accepted with expiry/ack-env present, expired override rejected.

## Write Scope
- `crates/roko-core/src/config/local.rs` (new or extended)
- `crates/roko-core/src/config/validation.rs`
- `crates/roko-core/src/config/provenance.rs` (if `OverrideScope` lives here)

## Verify

```bash
rg 'DangerousPermissionOverride|OverrideScope|dangerous_overrides' crates/roko-core/src/config/
# Expect: 5+ hits

rg 'BareDangerousSkipInLocal|BareDangerousSkipInShared' crates/roko-core/src/config/validation.rs
# Expect: 2 hits
```

## Do NOT

- Do NOT allow bare `dangerously_skip_permissions = true` in any file.
- Do NOT skip `acknowledgement_env` check.
- Do NOT bundle with other S-config batches.
- Do NOT delete the existing `runner.dangerously_skip_permissions` field on `RunnerConfig` — runtime code still reads it (set only via the typed override path).
