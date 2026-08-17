# S-config-2: ResolvedConfig + ValidatedConfig wrappers

## Task
Define `ResolvedConfig` (config + per-field provenance) and `ValidatedConfig` (post-validation newtype). Make `load_config` return `ValidatedConfig`. Wire env-var bindings + the four validators from S-config-1.

## Runner Context
Runner audit-2026-05-01, group S. Depends on S-config-1. Wave 2.

## Source plan
`tmp/subsystem-audits/implementation-plans/23-config-validation-pipeline.md` § Phase 1, 3.

## Read first

```bash
rg 'pub fn load_config|pub struct ResolvedConfig|pub struct ValidatedConfig|FieldProvenance' crates/roko-core/src/config/ -n
```

## Exact changes

### 1. Define types in `provenance.rs`

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum ConfigSource {
    Default,
    SharedFile(std::path::PathBuf),
    LocalFile(std::path::PathBuf),
    EnvVar(String),
    CliFlag(String),
}

#[derive(Debug, Clone)]
pub struct FieldProvenance {
    pub source: ConfigSource,
    pub raw_value: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ResolvedConfig {
    pub config: super::schema::RokoConfig,
    pub provenance: std::collections::HashMap<String, FieldProvenance>,
    pub local_overrides: Vec<DangerousPermissionOverride>,
}

impl Default for ResolvedConfig { ... }

#[derive(Debug, Clone)]
pub struct ValidatedConfig(std::sync::Arc<ResolvedConfig>);

impl ValidatedConfig {
    pub fn new(resolved: std::sync::Arc<ResolvedConfig>) -> Self { Self(resolved) }
    pub fn config(&self) -> &super::schema::RokoConfig { &self.0.config }
    pub fn provenance(&self) -> &std::collections::HashMap<String, FieldProvenance> { &self.0.provenance }
    pub fn local_overrides(&self) -> &[DangerousPermissionOverride] { &self.0.local_overrides }
    pub fn arc(&self) -> std::sync::Arc<ResolvedConfig> { self.0.clone() }
}
```

### 2. Update `load_config`

`crates/roko-core/src/config/mod.rs`:

```rust
pub fn load_config(workdir: &std::path::Path) -> Result<ValidatedConfig, LoadConfigError> {
    let mut resolved = ResolvedConfig::default();

    // 1. defaults
    record_provenance_root(&mut resolved, ConfigSource::Default);

    // 2. shared file
    let shared_path = workdir.join("roko.toml");
    if shared_path.exists() {
        let raw = std::fs::read_to_string(&shared_path)?;
        validate_strict_config_toml(&raw, &StrictConfigSource::SharedFile)
            .map_err(|e| LoadConfigError::Validation(e.to_string()))?;
        let shared: RokoConfig = toml::from_str(&raw)?;
        resolved.config = merge_into(resolved.config, &shared);
        record_provenance_for(&mut resolved, "shared", ConfigSource::SharedFile(shared_path.clone()));
    }

    // 3. local file
    if let Some(local_path) = local_config_path() {
        if local_path.exists() {
            let raw = std::fs::read_to_string(&local_path)?;
            validate_strict_config_toml(&raw, &StrictConfigSource::LocalFile)
                .map_err(|e| LoadConfigError::Validation(e.to_string()))?;
            let local: LocalConfig = toml::from_str(&raw)?;
            resolved.config = merge_local_into(resolved.config, &local);
            resolved.local_overrides = local.dangerous_overrides;
            record_provenance_for(&mut resolved, "local", ConfigSource::LocalFile(local_path));
        }
    }

    // 4. env var bindings
    apply_env_bindings(&mut resolved)?;

    // 5. semantic validation (S-config-1 validators)
    validation::validate_provider_auth(&resolved.config).map_err(LoadConfigError::Validation)?;
    validation::validate_unique_model_slugs(&resolved.config).map_err(LoadConfigError::Validation)?;
    validation::validate_gate_thresholds(&resolved.config).map_err(LoadConfigError::Validation)?;
    validation::validate_local_overrides(&resolved.local_overrides).map_err(LoadConfigError::Validation)?;

    Ok(ValidatedConfig::new(std::sync::Arc::new(resolved)))
}
```

### 3. Update `LoadConfigError`

```rust
#[derive(Debug, thiserror::Error)]
pub enum LoadConfigError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("toml: {0}")]
    Toml(#[from] toml::de::Error),
    #[error("validation: {0}")]
    Validation(String),    // accepts both ValidationError and string
}

// `From<ValidationError> for LoadConfigError` impl
impl From<ValidationError> for LoadConfigError {
    fn from(e: ValidationError) -> Self { LoadConfigError::Validation(e.to_string()) }
}
```

### 4. Tests

```rust
#[test]
fn validation_rejects_missing_provider_auth() {
    let workdir = tempdir().unwrap();
    let toml = r#"
[providers.anthropic]
kind = "anthropic_api"
api_key = ""
api_key_env = "TEST_NEVER_SET"
"#;
    std::fs::write(workdir.path().join("roko.toml"), toml).unwrap();
    std::env::remove_var("TEST_NEVER_SET");
    let err = load_config(workdir.path()).unwrap_err();
    assert!(matches!(err, LoadConfigError::Validation(_)));
}

#[test]
fn provenance_tracks_shared_then_local() {
    // Setup shared toml + local toml that overrides.
    // Assert ValidatedConfig::provenance() shows ConfigSource::LocalFile for the
    // overridden field.
}
```

## Write Scope
- `crates/roko-core/src/config/provenance.rs`
- `crates/roko-core/src/config/mod.rs`
- `crates/roko-core/src/config/validation.rs` (only error-type adjustments)

## Verify

```bash
rg 'pub fn load_config' crates/roko-core/src/config/mod.rs
# Should return Result<ValidatedConfig, LoadConfigError>

rg 'ResolvedConfig|ValidatedConfig' crates/roko-core/src/config/provenance.rs
# Expect: 4+ hits
```

## Acceptance Criteria

- `ResolvedConfig` / `ValidatedConfig` defined with provenance + local overrides.
- `load_config` returns `ValidatedConfig` with all 4 semantic validators called.
- 2 tests cover validation failure + provenance tracking.

## Do NOT

- Do NOT migrate consumers in this batch (S-config-3/4/5 do that).
- Do NOT silently downgrade invalid config to defaults.
- Do NOT make `load_config` async.
- Do NOT mutate config files.
