# S-config-1: Add semantic config validators

## Task
Add four semantic validators to `crates/roko-core/src/config/validation.rs`: `validate_provider_auth`, `validate_unique_model_slugs`, `validate_gate_thresholds`, `validate_local_overrides`. Each returns `Result<(), ValidationError>`.

## Runner Context
Runner audit-2026-05-01, group S. No dependencies. Wave 1.

## Source plan
`tmp/subsystem-audits/implementation-plans/23-config-validation-pipeline.md` § Phase 2.

## Why
T1-12 wired the strict TOML validator (rejects bare `dangerously_skip_permissions = true`). The next layer is semantic checks: provider auth resolves, model slugs unique, gate thresholds in [0,1], typed local overrides well-formed.

## Read first

```bash
rg 'validate_strict_config_toml|pub enum ValidationError' crates/roko-core/src/config/validation.rs -n
rg 'pub struct DangerousPermissionOverride' crates/roko-core/ -n
```

## Exact changes

### 1. Extend `ValidationError`

`crates/roko-core/src/config/validation.rs`:

```rust
#[derive(Debug, thiserror::Error)]
pub enum ValidationError {
    #[error("strict toml: {0}")]
    Strict(#[from] StrictConfigValidationError),

    #[error("provider {provider} requires API key (env {env_var}); not set")]
    MissingProviderAuth { provider: String, env_var: String },

    #[error("model slug {slug} appears in multiple providers: {providers:?}")]
    AmbiguousModelSlug { slug: String, providers: Vec<String> },

    #[error("gate threshold for {rung}={value} outside [0.0, 1.0]")]
    InvalidGateThreshold { rung: String, value: f64 },

    #[error("local dangerous override has empty `reason`")]
    EmptyOverrideReason,

    #[error("local dangerous override expired: {reason} (expired {expiry})")]
    ExpiredOverride { reason: String, expiry: chrono::DateTime<chrono::Utc> },

    #[error("local dangerous override requires acknowledgement env var {env_var}; not set")]
    OverrideAcknowledgementMissing { env_var: String },
}
```

### 2. Implement validators

```rust
use crate::config::{RokoConfig, ProviderKind, DangerousPermissionOverride};

pub fn validate_provider_auth(cfg: &RokoConfig) -> Result<(), ValidationError> {
    for (id, provider) in &cfg.providers {
        if !provider.kind.requires_api_key() { continue; }
        let env_set = std::env::var(&provider.api_key_env).is_ok();
        let inline_set = !provider.api_key.is_empty();
        if !env_set && !inline_set {
            return Err(ValidationError::MissingProviderAuth {
                provider: id.clone(),
                env_var: provider.api_key_env.clone(),
            });
        }
    }
    Ok(())
}

pub fn validate_unique_model_slugs(cfg: &RokoConfig) -> Result<(), ValidationError> {
    let mut seen: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    for (slug, model) in &cfg.models {
        if let Some(prev) = seen.insert(slug.clone(), model.provider.clone()) {
            if prev != model.provider {
                return Err(ValidationError::AmbiguousModelSlug {
                    slug: slug.clone(),
                    providers: vec![prev, model.provider.clone()],
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
                return Err(ValidationError::ExpiredOverride { reason: o.reason.clone(), expiry });
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

### 3. Tests

Add unit tests for each validator (one happy + one failure path each).

## Write Scope
- `crates/roko-core/src/config/validation.rs`

## Read-Only Context
- `crates/roko-core/src/config/schema.rs`
- `crates/roko-core/src/config/provenance.rs`

## Verify

```bash
rg 'pub fn validate_provider_auth|pub fn validate_unique_model_slugs|pub fn validate_gate_thresholds|pub fn validate_local_overrides' crates/roko-core/src/config/validation.rs
# Expect: 4 hits
```

## Acceptance Criteria

- 4 validator functions implemented.
- `ValidationError` has typed variants for each failure mode.
- Tests cover happy + failure paths per validator.

## Do NOT

- Do NOT call validators from `load_config` here — that's S-config-2.
- Do NOT bundle with other S-config batches.
- Do NOT make the validators async.
- Do NOT validate at provider-construction time; this is a post-load check.
