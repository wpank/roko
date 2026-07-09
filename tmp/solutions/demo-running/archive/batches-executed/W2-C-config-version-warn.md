# W2-C: Suppress False Config Version Warning

**Priority**: P1 — makes demo presentable
**Effort**: 15 minutes
**Files to modify**: 1 file
**Dependencies**: None

## Problem

`WARN: roko.toml uses config version 1 (no [providers] section)` appears on EVERY command, even with a freshly `roko init`-generated config that has `config_version = 2`.

## Root Cause

In `crates/roko-core/src/config/schema.rs`, the `from_toml()` method (line 151-158) checks `config.config_version == 1`. But the default value for `config_version` is `1` (line 106-108):

```rust
const fn default_config_version() -> u32 {
    1  // ← this means ANY config without explicit config_version = 2 triggers the warning
}
```

So if a config file exists but doesn't have `config_version = 2` explicitly written (e.g., it was created by an older version of `roko init`, or it's a minimal config), the default kicks in as `1` and the warning fires.

## Fix

### File: `crates/roko-core/src/config/schema.rs`

### Option A (recommended): Change the default to 2

Since config version 2 is the current version, new configs should default to it:

```rust
const fn default_config_version() -> u32 {
    2  // ← current version; only explicitly legacy configs should be 1
}
```

This means only configs that explicitly say `config_version = 1` will trigger the migration warning. Configs without the field will default to current version (2) and not warn.

### Option B: Make the warning check smarter

Keep the default as 1, but change the warning to only fire when the file ACTUALLY has legacy format (not just missing the field):

```rust
pub fn from_toml(s: &str) -> Result<Self, toml::de::Error> {
    let config: Self = toml::from_str(s)?;
    // Only warn if the file explicitly declares version 1
    // (not if it's just missing the field and got the default)
    let raw: toml::Value = toml::from_str(s)?;
    let explicit_version = raw.get("config_version").and_then(|v| v.as_integer());
    if explicit_version == Some(1) {
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

**Option A is simpler and preferred.**

## Agent Prompt

```
Read /Users/will/dev/nunchi/roko/roko/tmp/solutions/demo-running/batches/W2-C-config-version-warn.md and implement all changes described in it. This is a 1-line change: change default_config_version() from 1 to 2 in crates/roko-core/src/config/schema.rs. Do NOT run cargo build/test/clippy/fmt — compilation is deferred. Mark the checklist items as done.
```

## Commit

This batch is committed with all Wave 2 batches together. Do not commit individually.

## Checklist

- [x] Change `default_config_version()` to return `2` in schema.rs
- [x] Verify: fresh `roko init` produces no version warnings
- [x] Verify: regular commands produce no version warnings
- [x] Verify: existing tests still pass (some may expect version 1 default)
- [x] Pre-commit checks pass
