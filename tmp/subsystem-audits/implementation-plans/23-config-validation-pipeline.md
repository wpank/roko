# 23 — Config Validation Pipeline (T5-38 expanded)

The strict TOML validator (T1-12) catches root-level dangerous bypass.
The next layer is **semantic validation** + **provenance tracking**:

- Where did each config field come from? (shared / local / env / CLI / default)
- Are provider auth credentials present and well-formed?
- Are model slugs unique across providers?
- Are gate thresholds in `[0, 1]`?
- Are dangerous local overrides typed (reason / scope / expiry / ack)?

Source: doc 35 § Config and safety, doc 39 (config schema phantom fields),
doc 41 T5-38, doc 25 (config-safety-telemetry plan).

---

## Today's State (verified 2026-05-01)

- `roko_core::config::provenance` has `ResolvedConfig` and
  `ValidatedConfig` skeleton types (C1, C2 in packet ledger).
- `validate_strict_config_toml` runs in `load_config` (T1-12).
- `DangerousPermissionOverride` type exists with reason/scope/expiry/source/
  acknowledgement_env (C4).
- A `roko config doctor` command skeleton exists (C9).
- **What's missing**: end-to-end use of these types. `load_config` returns a
  bare `RokoConfig`; consumers don't see provenance; semantic checks (model
  uniqueness, threshold ranges, provider auth resolution) are scattered.

---

## Anti-Patterns

1. **No "fallback to defaults" on validation failure.** Failed validation =
   typed error to caller. Caller decides whether to surface to user or
   abort.
2. **No silent provenance loss.** A field's source is preserved through
   the load, validation, and consumer paths.
3. **No new "config schema" file outside `roko-core`.** All config types
   live in `roko-core/src/config/`.
4. **No env-var read outside the config loader.** Consumers receive
   resolved values; they don't read env vars themselves.
5. **No `dangerously_skip_permissions = true` outside a typed override.**
   The strict validator catches it; do not bypass.

---

## Plan

### Phase 1: Define `ResolvedConfig` and `ValidatedConfig`

**File**: `crates/roko-core/src/config/provenance.rs` (already exists per
audit; expand)

```rust
/// Where each config field came from.
#[derive(Debug, Clone, PartialEq)]
pub enum ConfigSource {
    Default,
    SharedFile(PathBuf),
    LocalFile(PathBuf),
    EnvVar(String),
    CliFlag(String),
}

#[derive(Debug, Clone)]
pub struct FieldProvenance {
    pub source: ConfigSource,
    pub raw_value: Option<String>,    // for diagnostics / `roko config doctor`
}

/// All fields tagged with their source. Not yet semantically validated.
#[derive(Debug, Clone)]
pub struct ResolvedConfig {
    pub config: RokoConfig,
    pub provenance: HashMap<String, FieldProvenance>,
    /// Local overrides that bypass shared-file restrictions (typed,
    /// audited).
    pub local_overrides: Vec<DangerousPermissionOverride>,
}

/// `ResolvedConfig` after semantic checks pass. Newtype.
#[derive(Debug, Clone)]
pub struct ValidatedConfig(Arc<ResolvedConfig>);

impl ValidatedConfig {
    pub fn config(&self) -> &RokoConfig { &self.0.config }
    pub fn provenance(&self) -> &HashMap<String, FieldProvenance> { &self.0.provenance }
    pub fn local_overrides(&self) -> &[DangerousPermissionOverride] { &self.0.local_overrides }
}
```

### Phase 2: Implement semantic validators

**File**: `crates/roko-core/src/config/validation.rs` (already exists)

Add validators (each as a free function returning `Result<(), ValidationError>`):

```rust
pub fn validate_provider_auth(cfg: &RokoConfig) -> Result<(), ValidationError> {
    for (id, provider) in &cfg.providers {
        match provider.kind {
            ProviderKind::AnthropicApi
            | ProviderKind::OpenAi
            | ProviderKind::Cerebras
            | ProviderKind::Cursor
            | ProviderKind::Gemini
            | ProviderKind::Perplexity => {
                if provider.api_key.is_empty()
                    && std::env::var(&provider.api_key_env).is_err()
                {
                    return Err(ValidationError::MissingProviderAuth {
                        provider: id.clone(),
                        env_var: provider.api_key_env.clone(),
                    });
                }
            }
            ProviderKind::ClaudeCli | ProviderKind::Ollama | ProviderKind::Local => {
                // No API key needed
            }
        }
    }
    Ok(())
}

pub fn validate_unique_model_slugs(cfg: &RokoConfig) -> Result<(), ValidationError> {
    let mut seen: HashMap<&str, &str> = HashMap::new();
    for (slug, model) in &cfg.models {
        if let Some(prev) = seen.insert(slug.as_str(), model.provider.as_str()) {
            if prev != model.provider {
                return Err(ValidationError::AmbiguousModelSlug {
                    slug: slug.clone(),
                    providers: vec![prev.into(), model.provider.clone()],
                });
            }
        }
    }
    Ok(())
}

pub fn validate_gate_thresholds(cfg: &RokoConfig) -> Result<(), ValidationError> {
    for (rung, threshold) in &cfg.gates.thresholds {
        if !(0.0..=1.0).contains(threshold) {
            return Err(ValidationError::InvalidGateThreshold {
                rung: rung.clone(),
                value: *threshold,
            });
        }
    }
    Ok(())
}

pub fn validate_local_overrides(overrides: &[DangerousPermissionOverride]) -> Result<(), ValidationError> {
    for o in overrides {
        if o.reason.trim().is_empty() {
            return Err(ValidationError::EmptyOverrideReason);
        }
        if let Some(expiry) = o.expiry {
            if expiry < chrono::Utc::now() {
                return Err(ValidationError::ExpiredOverride { reason: o.reason.clone() });
            }
        }
        if std::env::var(&o.acknowledgement_env).is_err() {
            return Err(ValidationError::OverrideAcknowledgementMissing {
                env_var: o.acknowledgement_env.clone(),
            });
        }
    }
    Ok(())
}
```

### Phase 3: Update `load_config` to return `ValidatedConfig`

**File**: `crates/roko-core/src/config/mod.rs`

```rust
pub fn load_config(workdir: &Path) -> Result<ValidatedConfig, LoadConfigError> {
    let shared_path = workdir.join("roko.toml");
    let local_path = local_config_path(); // ~/.config/roko/local.toml

    let mut resolved = ResolvedConfig::default();

    // 1. Defaults (provenance: Default)
    resolved.config = RokoConfig::default();
    record_provenance(&mut resolved, "/", ConfigSource::Default);

    // 2. Shared file
    if shared_path.exists() {
        let raw = std::fs::read_to_string(&shared_path)?;
        validate_strict_config_toml(&raw, &StrictConfigSource::SharedFile)
            .map_err(|e| LoadConfigError::Validation(e.to_string()))?;
        let shared: RokoConfig = toml::from_str(&raw)?;
        merge_into(&mut resolved.config, &shared);
        record_provenance(&mut resolved, "shared", ConfigSource::SharedFile(shared_path.clone()));
    }

    // 3. Local file (may include DangerousPermissionOverrides)
    if let Some(local_path) = &local_path {
        if local_path.exists() {
            let raw = std::fs::read_to_string(local_path)?;
            validate_strict_config_toml(&raw, &StrictConfigSource::LocalFile)?;
            let local: LocalConfig = toml::from_str(&raw)?;
            merge_local_into(&mut resolved.config, &local);
            resolved.local_overrides = local.dangerous_overrides;
            record_provenance(&mut resolved, "local", ConfigSource::LocalFile(local_path.clone()));
        }
    }

    // 4. Env var bindings (e.g. ROKO_SERVE_AUTH_API_KEY)
    apply_env_bindings(&mut resolved)?;

    // 5. Semantic validation
    validate_provider_auth(&resolved.config)
        .map_err(LoadConfigError::Validation)?;
    validate_unique_model_slugs(&resolved.config)
        .map_err(LoadConfigError::Validation)?;
    validate_gate_thresholds(&resolved.config)
        .map_err(LoadConfigError::Validation)?;
    validate_local_overrides(&resolved.local_overrides)
        .map_err(LoadConfigError::Validation)?;

    Ok(ValidatedConfig::new(Arc::new(resolved)))
}
```

### Phase 4: Migrate consumers (one crate per commit)

Search for `load_config()` callers:

```bash
rg 'load_config\(' crates/ -g '*.rs'
```

For each, update the binding type:

```rust
// Before
let config: RokoConfig = roko_core::config::load_config(&workdir)?;

// After
let validated: ValidatedConfig = roko_core::config::load_config(&workdir)?;
let config: &RokoConfig = validated.config();
```

Most consumers only need `validated.config()`. A few need provenance for
display (`roko config doctor`, TUI config view).

Migrate per crate:

| Order | Crate | Notes |
|---|---|---|
| 1 | `roko-cli` | Most callers |
| 2 | `roko-serve` | Server bootstrap |
| 3 | `roko-acp` | Session config |
| 4 | `roko-runtime` | Workflow engine config |
| 5 | `roko-agent` | Provider resolution config |

### Phase 5: Update `roko config doctor`

The skeleton exists. Add:

- Per-field provenance display: where does `agent.default_model` come
  from? Show "shared file `roko.toml:42`" or "env `ROKO_AGENT_DEFAULT_MODEL`."
- Validation warnings: missing provider auth, ambiguous model slugs,
  invalid thresholds, expired overrides.
- Local overrides table: reason, scope, expiry, source, ack env state.

### Phase 6: Tests

```rust
#[test]
fn validation_rejects_missing_provider_auth() {
    let cfg = RokoConfig {
        providers: hashmap! {
            "anthropic".into() => ProviderConfig {
                kind: ProviderKind::AnthropicApi,
                api_key: "".into(),
                api_key_env: "TEST_NEVER_SET".into(),
                ..Default::default()
            }
        },
        ..Default::default()
    };
    let err = validate_provider_auth(&cfg).unwrap_err();
    assert!(matches!(err, ValidationError::MissingProviderAuth { .. }));
}

#[test]
fn provenance_tracks_shared_to_local_override() {
    let workdir = tempdir().unwrap();
    let shared = workdir.path().join("roko.toml");
    std::fs::write(&shared, r#"[agent]\ndefault_model = "claude-sonnet-4-6""#).unwrap();
    // local override
    let _local = with_local_override(r#"[agent]\ndefault_model = "opus""#);

    let validated = load_config(workdir.path()).unwrap();
    assert_eq!(validated.config().agent.default_model, "opus");
    assert!(matches!(
        validated.provenance().get("agent.default_model").map(|p| &p.source),
        Some(ConfigSource::LocalFile(_))
    ));
}

#[test]
fn dangerous_override_requires_acknowledgement_env() {
    let override = DangerousPermissionOverride {
        reason: "local debugging session".into(),
        scope: OverrideScope::Workspace("/tmp/test".into()),
        expiry: Some(chrono::Utc::now() + chrono::Duration::hours(1)),
        source: ConfigSource::LocalFile("...".into()),
        acknowledgement_env: "TEST_ACK_NEVER_SET".into(),
    };
    let err = validate_local_overrides(&[override]).unwrap_err();
    assert!(matches!(err, ValidationError::OverrideAcknowledgementMissing { .. }));
}
```

### Verify

```bash
cargo test -p roko-core config --lib
cargo test --workspace

# Provenance display
cargo run -p roko-cli -- config doctor
# Should show per-field source and any warnings

# Wrong threshold rejected
cargo run -p roko-cli -- config doctor --check 'gates.thresholds.compile=1.5'
# Errors with InvalidGateThreshold
```

---

## Phase 7: Lock dangerous overrides to local-only path

**File**: `crates/roko-core/src/config/mod.rs`

The strict validator already rejects `dangerously_skip_permissions = true`
in `SharedFile`. Confirm the validator allows it in `LocalFile` **only
when wrapped in a typed `DangerousPermissionOverride`**.

```rust
// In LocalConfig schema:
pub struct LocalConfig {
    /// Standard config fields (subset of RokoConfig that's overridable locally)
    #[serde(flatten)]
    pub overrides: PartialRokoConfig,

    /// Dangerous-permission bypasses, one per scope.
    #[serde(default)]
    pub dangerous_overrides: Vec<DangerousPermissionOverride>,
}

// In strict validator for LocalFile:
fn validate_local_dangerous_skip(raw: &str) -> Result<(), ValidationError> {
    // Reject `runner.dangerously_skip_permissions = true` at the bare
    // top level — it must come through `dangerous_overrides`.
    let parsed: toml::Value = toml::from_str(raw)?;
    if parsed.get("runner").and_then(|r| r.get("dangerously_skip_permissions")).and_then(|v| v.as_bool()) == Some(true) {
        return Err(ValidationError::BareDangerousSkipInLocal);
    }
    Ok(())
}
```

The runner reads `dangerously_skip_permissions` from
`validated.local_overrides()` and matches scope (workspace path).

---

## Combined Verification

```bash
cargo test --workspace
cargo run -p roko-cli -- config doctor

# All callers receive ValidatedConfig
rg 'fn load_config' crates/roko-core/src/config/
# Returns ValidatedConfig

rg 'load_config\(' crates/ -g '*.rs' | rg -v 'crates/roko-core/'
# All callers compile against ValidatedConfig

# Strict validator catches bypass
echo 'dangerously_skip_permissions = true' >> /tmp/test-roko.toml
ROKO_WORKDIR=/tmp/test cargo run -p roko-cli -- config doctor
# Errors out
```

---

## Status

- [ ] Phase 1 — Define `ResolvedConfig` / `ValidatedConfig`
- [ ] Phase 2 — Implement semantic validators
- [ ] Phase 3 — `load_config` returns `ValidatedConfig`
- [ ] Phase 4 — Migrate consumers (one crate per commit)
- [ ] Phase 5 — `roko config doctor` shows provenance + warnings
- [ ] Phase 6 — Tests
- [ ] Phase 7 — Lock dangerous overrides to typed local-only

**Estimated effort**: 12-20 hours total.
