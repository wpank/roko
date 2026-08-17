# W11-C: Config Reference Validation on Load + Synthesized Profile Fallback

**Priority**: P0 -- invalid config loads silently, fails minutes later at dispatch time; synthesized fallback references non-existent providers
**Effort**: ~20 min
**Files to modify**: 1
**Dependencies**: None

## Problem

Two config validation gaps in `schema.rs`:

1. **`from_toml` skips reference validation**: `RokoConfig::from_toml()` deserializes the TOML and warns about schema version, but never calls `validate_references()`. A config with `models.fast.provider = "nonexistent"` loads successfully. The error surfaces minutes later when agent dispatch tries to resolve the provider.

2. **Synthesized model profile uses label as provider**: `synthesized_model_profile()` falls back to `expected_kind.label()` (e.g., `"claude"`) when no provider of the expected kind exists. This label may not be a key in `self.providers`, so the synthesized profile has a provider reference that is unresolvable.

## Exact Code to Change

### File 1: `crates/roko-core/src/config/schema.rs`

#### Change 1: Add `validate_references` call to `from_toml`

**Find this code** (line 164):
```rust
    pub fn from_toml(s: &str) -> Result<Self, toml::de::Error> {
        let config: Self = toml::from_str(s)?;
        // Only warn when the TOML text explicitly sets config_version (not when
        // the serde default of 1 kicks in for configs that omit the field, such
        // as the global config at ~/.roko/config.toml).
        if config.config_version <= 1 && text_has_config_version(s) {
            static WARNED: std::sync::Once = std::sync::Once::new();
            WARNED.call_once(|| {
                tracing::warn!(
                    "roko.toml uses config version 1 (no [providers] section)\n  hint: run `roko config migrate` to upgrade"
                );
            });
        }
        Ok(config)
    }
```

**Replace with:**
```rust
    pub fn from_toml(s: &str) -> Result<Self, toml::de::Error> {
        let config: Self = toml::from_str(s)?;
        // Only warn when the TOML text explicitly sets config_version (not when
        // the serde default of 1 kicks in for configs that omit the field, such
        // as the global config at ~/.roko/config.toml).
        if config.config_version <= 1 && text_has_config_version(s) {
            static WARNED: std::sync::Once = std::sync::Once::new();
            WARNED.call_once(|| {
                tracing::warn!(
                    "roko.toml uses config version 1 (no [providers] section)\n  hint: run `roko config migrate` to upgrade"
                );
            });
        }
        // Validate that model.provider references point to declared providers
        // and that agent.default_model / tier_models reference declared models.
        // Warnings here surface at load time instead of failing at dispatch.
        let warnings = validate_references(&config);
        for w in &warnings {
            tracing::warn!("config reference validation: {w}");
        }
        tracing::debug!(
            providers = config.providers.len(),
            models = config.models.len(),
            ref_warnings = warnings.len(),
            "config loaded and validated"
        );
        Ok(config)
    }
```

#### Change 2: Add tracing warning to synthesized profile provider fallback

**Find this code** (line 245):
```rust
        let provider = self
            .providers
            .iter()
            .find(|(_, p)| p.kind == expected_kind)
            .map(|(name, _)| name.as_str())
            .unwrap_or_else(|| expected_kind.label());
```

**Replace with:**
```rust
        let provider = match self
            .providers
            .iter()
            .find(|(_, p)| p.kind == expected_kind)
            .map(|(name, _)| name.as_str())
        {
            Some(p) => p,
            None => {
                tracing::warn!(
                    slug = %slug,
                    kind = ?expected_kind,
                    "no provider of kind {:?} configured for synthesized model '{}'; \
                     using label '{}' as fallback -- dispatch may fail",
                    expected_kind, slug, expected_kind.label()
                );
                expected_kind.label()
            }
        };
```

## Verification

```bash
cd /Users/will/dev/nunchi/roko/roko

# 1. Build the crate
cargo check -p roko-core

# 2. Run existing tests (some tests construct RokoConfig via from_toml --
#    they should still pass, though they may now emit warnings to stderr)
cargo test -p roko-core

# 3. Verify validate_references is now called from from_toml
grep -n 'validate_references' crates/roko-core/src/config/schema.rs
# Should show the call inside from_toml AND the function definition at line 937

# 4. Verify the unwrap_or_else is gone
grep -n 'unwrap_or_else.*expected_kind.label' crates/roko-core/src/config/schema.rs
# Should return 0 results

# 5. Integration: load a real config
cargo run -p roko-cli -- config show 2>&1 | head -20
```

## Agent Prompt

```
Fix two config validation gaps in `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/config/schema.rs`.

## Context

The file contains:
- `RokoConfig::from_toml()` at line 164 -- deserializes TOML config
- `pub fn validate_references(config: &RokoConfig) -> Vec<ValidationWarning>` at line 937 --
  checks that model.provider references point to declared providers, etc.
- `fn synthesized_model_profile(&self, slug: &str) -> ModelProfile` at line 236 -- creates
  a fallback model profile when no explicit config exists for a slug

## Fix 1: Call `validate_references` from `from_toml` (line ~164)

After the schema version check and before `Ok(config)`, add:
```rust
let warnings = validate_references(&config);
for w in &warnings {
    tracing::warn!("config reference validation: {w}");
}
tracing::debug!(
    providers = config.providers.len(),
    models = config.models.len(),
    ref_warnings = warnings.len(),
    "config loaded and validated"
);
```

The `validate_references` function already exists as `pub fn` at line 937 in the same file.
It returns `Vec<ValidationWarning>` where `ValidationWarning` implements `Display`.

## Fix 2: Replace `.unwrap_or_else(|| expected_kind.label())` with match + warning (line ~245)

In `synthesized_model_profile`, the provider lookup chain ends with:
```rust
.unwrap_or_else(|| expected_kind.label());
```

Replace with a `match` that logs a warning when falling back to the kind label:
```rust
let provider = match self.providers.iter()
    .find(|(_, p)| p.kind == expected_kind)
    .map(|(name, _)| name.as_str())
{
    Some(p) => p,
    None => {
        tracing::warn!(
            slug = %slug,
            kind = ?expected_kind,
            "no provider of kind {:?} configured for synthesized model '{}'; \
             using label '{}' as fallback -- dispatch may fail",
            expected_kind, slug, expected_kind.label()
        );
        expected_kind.label()
    }
};
```

This keeps the same runtime behavior (fallback to label) but makes the failure visible in logs.

Run `cargo check -p roko-core` and `cargo test -p roko-core` to verify.
```

## Commit

This batch is committed with Wave 11. Do not commit individually.

## Checklist

- [ ] `validate_references(&config)` called in `from_toml` before `Ok(config)`
- [ ] Warnings from `validate_references` emitted via `tracing::warn!`
- [ ] `tracing::debug!` added for load-time visibility
- [ ] `unwrap_or_else(|| expected_kind.label())` replaced with match + warning
- [ ] `cargo check -p roko-core` passes
- [ ] `cargo test -p roko-core` passes
- [ ] Existing tests that use `from_toml` still pass (may emit new warnings)

## Audit Status

Audited: 2026-05-05. PASS no changes needed.
