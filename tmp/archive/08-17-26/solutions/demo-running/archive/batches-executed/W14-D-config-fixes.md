# W14-D: Config System Fixes

**Priority**: P2 -- correctness and usability
**Effort**: 2-3 hours
**Files to modify**: 3 files
**Dependencies**: None
**IMPROVEMENTS**: 13.1, 13.2, 13.3, 13.4, 13.5

## Problem

Five issues in the config system:

1. **13.1**: Module docstring says `ROKO__*` provides field-level overrides. `apply_env_overrides: bool` is defined and defaults to `true`. But `apply_process_env()` only handles 12 named env vars (`ROKO_MODEL`, `ROKO_BACKEND`, etc.). There is no hierarchical `ROKO__SECTION__FIELD` override system. The documentation is misleading.

2. **13.2**: Deprecated `load_config` / `load_config_strict` use `load_config_impl`, which does NOT merge global config, does NOT apply env overrides, does NOT walk ancestors. Callers using the deprecated functions get silently different behavior.

3. **13.3**: `merge_global_into` only merges `providers`, `models`, and 2 `agent` fields from `~/.roko/config.toml`. A global `[budget]`, `[gates]`, `[serve]`, `[conductor]` are silently ignored. Users cannot set global defaults for most settings.

4. **13.4**: `collect_diagnostics` runs on the post-env-override config. A diagnostic like "model 'opus' references provider 'anthropic' which is not configured" may be wrong if `ROKO_PROVIDER=anthropic` was set. No env-var context in messages.

5. **13.5**: `interpolate_env_vars_with` only expands `${VAR}` in provider config strings (`base_url`, `api_key_env`, `command`, `extra_headers`). A user writing `command = "${CLAUDE_PATH}"` in `[agent]` gets the literal string.

## Root Cause

The config system was unified from 12+ separate loaders. The module docstring was written aspirationally. The deprecated paths were left as-is instead of being delegated to the new unified loader.

## Exact Code to Change

### Fix 13.1 -- Correct ROKO__* documentation

**File**: `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/config/loader.rs`
**Lines**: 1-15

**Find this code:**
```rust
//! Unified config loader for all Roko binaries (CLI, serve, ACP, agent-server).
//!
//! Before this module, 12+ separate `load_roko_config` functions existed across
//! the codebase, each with different behavior around global config merging,
//! `ROKO_CONFIG` env var, `ROKO__*` overrides, and validation. This module
//! provides a **single entry point** that all callsites should use.
//!
//! # Precedence (highest wins)
//!
//! 1. Process environment `ROKO__*` overrides (field-level)
//! 2. `ROKO_CONFIG` env var → load that file instead of ancestor walk
//! 3. Project `roko.toml` (found via ancestor walk from workdir)
//! 4. Global `~/.roko/config.toml` (providers/models merged)
//! 5. Built-in defaults ([`RokoConfig::default()`])
```

**Replace with:**
```rust
//! Unified config loader for all Roko binaries (CLI, serve, ACP, agent-server).
//!
//! Before this module, 12+ separate `load_roko_config` functions existed across
//! the codebase, each with different behavior around global config merging,
//! `ROKO_CONFIG` env var, env overrides, and validation. This module provides
//! a **single entry point** that all callsites should use.
//!
//! # Precedence (highest wins)
//!
//! 1. Named env var overrides (see list below)
//! 2. `ROKO_CONFIG` env var -> load that file instead of ancestor walk
//! 3. Project `roko.toml` (found via ancestor walk from workdir)
//! 4. Global `~/.roko/config.toml` (providers/models/agent defaults merged)
//! 5. Built-in defaults ([`RokoConfig::default()`])
//!
//! # Supported environment variable overrides
//!
//! | Variable | Config field |
//! |---|---|
//! | `ROKO_MODEL` | `agent.default_model` |
//! | `ROKO_BACKEND` | `agent.default_backend` |
//! | `ROKO_EFFORT` | `agent.default_effort` |
//! | `ROKO_CONTEXT_LIMIT_K` | `agent.context_limit_k` |
//! | `ROKO_MAX_AGENTS` | `conductor.max_agents` |
//! | `ROKO_BUDGET_USD` | `budget.max_plan_usd` |
//! | `ROKO_PARALLEL` | `conductor.parallel_enabled` |
//! | `ROKO_EXPRESS` | `conductor.express_mode` |
//! | `ROKO_SKIP_TESTS` | `gates.skip_tests` |
//! | `ROKO_CLIPPY` | `gates.clippy_enabled` |
//! | `ROKO_PROVIDER` | synthesized model profile provider |
//! | `ROKO_MODEL_SLUG` | synthesized model profile slug |
//!
//! **Note**: Hierarchical `ROKO__SECTION__FIELD` overrides are not currently
//! implemented. Only the named variables listed above are supported.
```

---

**Same file, lines 29-30:**

**Find this code:**
```rust
    /// Apply `ROKO__*` env var overrides.
    pub apply_env_overrides: bool,
```

**Replace with:**
```rust
    /// Apply named env var overrides (ROKO_MODEL, ROKO_BACKEND, etc.).
    pub apply_env_overrides: bool,
```

### Fix 13.2 -- Deprecated load_config delegates to unified loader

**File**: `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/config/mod.rs`
**Lines**: 95-114

**Find this code:**
```rust
/// Load the workspace configuration from `workdir/roko.toml`.
///
/// **Deprecated**: Use [`loader::load_config_unified`] or
/// [`loader::load_config_validated`] instead. This function skips ancestor
/// walk, global config merge, `ROKO_CONFIG` env var, and `ROKO__*` overrides.
#[deprecated(note = "use roko_core::config::loader::load_config_validated() instead")]
pub fn load_config(workdir: &Path) -> Result<ValidatedConfig, LoadConfigError> {
    load_config_impl(workdir, ConfigTrust::Local)
}

/// Load the workspace configuration with strict safety validation.
///
/// **Deprecated**: Use [`loader::load_config_with_options`] with
/// [`loader::LoadOptions::strict()`] instead.
#[deprecated(
    note = "use roko_core::config::loader::load_config_with_options(workdir, &LoadOptions::strict()) instead"
)]
pub fn load_config_strict(workdir: &Path) -> Result<ValidatedConfig, LoadConfigError> {
    load_config_impl(workdir, ConfigTrust::Shared)
}
```

**Replace with:**
```rust
/// Load the workspace configuration from `workdir/roko.toml`.
///
/// **Deprecated**: Use [`loader::load_config_validated`] instead.
/// This function now delegates to the unified loader with default options.
#[deprecated(note = "use roko_core::config::loader::load_config_validated() instead")]
pub fn load_config(workdir: &Path) -> Result<ValidatedConfig, LoadConfigError> {
    tracing::debug!(workdir = %workdir.display(), "deprecated load_config -> unified loader");
    loader::load_config_validated_with_options(workdir, &loader::LoadOptions::default())
}

/// Load the workspace configuration with strict safety validation.
///
/// **Deprecated**: Use [`loader::load_config_with_options`] with
/// [`loader::LoadOptions::strict()`] instead.
/// This function now delegates to the unified loader with strict options.
#[deprecated(
    note = "use roko_core::config::loader::load_config_with_options(workdir, &LoadOptions::strict()) instead"
)]
pub fn load_config_strict(workdir: &Path) -> Result<ValidatedConfig, LoadConfigError> {
    tracing::debug!(workdir = %workdir.display(), "deprecated load_config_strict -> unified loader");
    loader::load_config_validated_with_options(
        workdir,
        &loader::LoadOptions {
            merge_global: true,
            apply_env_overrides: true,
            strict_validation: true,
        },
    )
}
```

**Note**: The `load_config_impl` function and `ConfigTrust` enum will become dead code. They can remain with `#[allow(dead_code)]` or be removed in a follow-up cleanup. If tests reference them (e.g., `#[allow(deprecated)]` test blocks), leave them for now.

### Fix 13.3 -- Extend merge_global_into with more sections

**File**: `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/config/loader.rs`
**Lines**: 388-401 (the merge section after `global` is parsed)

**Find this code:**
```rust
    for (name, provider) in global.providers {
        config.providers.entry(name).or_insert(provider);
    }
    for (name, model) in global.models {
        config.models.entry(name).or_insert(model);
    }

    // Merge agent defaults when the project config doesn't set them.
    if config.agent.default_model.is_empty() && !global.agent.default_model.is_empty() {
        config.agent.default_model = global.agent.default_model;
    }
    if config.agent.default_backend.is_empty() && !global.agent.default_backend.is_empty() {
        config.agent.default_backend = global.agent.default_backend;
    }
}
```

**Replace with:**
```rust
    // -- Providers and models (always merge, project wins) --
    for (name, provider) in global.providers {
        config.providers.entry(name).or_insert(provider);
    }
    for (name, model) in global.models {
        config.models.entry(name).or_insert(model);
    }

    // -- Agent defaults (fill gaps only) --
    if config.agent.default_model.is_empty() && !global.agent.default_model.is_empty() {
        tracing::debug!(model = %global.agent.default_model, "merged global agent.default_model");
        config.agent.default_model = global.agent.default_model;
    }
    if config.agent.default_backend.is_empty() && !global.agent.default_backend.is_empty() {
        tracing::debug!(backend = %global.agent.default_backend, "merged global agent.default_backend");
        config.agent.default_backend = global.agent.default_backend;
    }
    if config.agent.default_effort.is_empty() && !global.agent.default_effort.is_empty() {
        tracing::debug!(effort = %global.agent.default_effort, "merged global agent.default_effort");
        config.agent.default_effort = global.agent.default_effort.clone();
    }

    // -- Budget defaults (fill when project uses default values) --
    // BudgetConfig::default().max_plan_usd = 25.0
    let default_max_plan_usd: f32 = 25.0;
    if (config.budget.max_plan_usd - default_max_plan_usd).abs() < f32::EPSILON
        && (global.budget.max_plan_usd - default_max_plan_usd).abs() > f32::EPSILON
    {
        tracing::debug!(
            max_plan_usd = global.budget.max_plan_usd,
            "merged global budget.max_plan_usd"
        );
        config.budget.max_plan_usd = global.budget.max_plan_usd;
    }

    // -- Conductor defaults (fill when project uses defaults) --
    // ConductorConfig default max_agents = 8
    let default_max_agents: usize = 8;
    if config.conductor.max_agents == default_max_agents
        && global.conductor.max_agents != default_max_agents
    {
        tracing::debug!(
            max_agents = global.conductor.max_agents,
            "merged global conductor.max_agents"
        );
        config.conductor.max_agents = global.conductor.max_agents;
    }
}
```

### Fix 13.4 -- Add env-var context to diagnostics

**File**: `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/config/loader.rs`
**Lines**: 260-263 (just before `diagnostics` is returned at the end of `collect_diagnostics`)

**Find this code:**
```rust
    diagnostics
}
```

(This is the last line of the `collect_diagnostics` function, right before the closing brace.)

**Replace with:**
```rust
    // If env overrides were applied, add a note so users understand that
    // some diagnostics may refer to env-injected values.
    let env_vars_present = std::env::var("ROKO_MODEL").is_ok()
        || std::env::var("ROKO_BACKEND").is_ok()
        || std::env::var("ROKO_PROVIDER").is_ok()
        || std::env::var("ROKO_CONFIG").is_ok();

    if env_vars_present && !diagnostics.is_empty() {
        diagnostics.push(ConfigDiagnostic {
            key: "_env_override_note".to_string(),
            message: "one or more ROKO_* env vars are set; some diagnostics above \
                      may reflect env-injected values rather than roko.toml contents"
                .to_string(),
        });
    }

    diagnostics
}
```

### Fix 13.5 -- Document interpolation scope limitation

**File**: `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/config/schema.rs`
**Lines**: 493-496

**Find this code:**
```rust
    /// Interpolate `${VAR}` patterns in provider config strings.
    pub fn interpolate_env_vars(&mut self) {
        Self::interpolate_env_vars_with(&mut self.providers, &|key| std::env::var(key).ok());
    }
```

**Replace with:**
```rust
    /// Interpolate `${VAR}` patterns in provider config strings.
    ///
    /// **Scope**: Interpolation currently only applies to provider fields:
    /// `base_url`, `api_key_env`, `command`, and `extra_headers`. Other
    /// config sections (agent, budget, gates, etc.) do NOT support `${VAR}`
    /// syntax -- literal strings are used as-is.
    ///
    /// To set non-provider fields dynamically, use the named environment
    /// variable overrides (e.g., `ROKO_MODEL`, `ROKO_BACKEND`) instead.
    pub fn interpolate_env_vars(&mut self) {
        Self::interpolate_env_vars_with(&mut self.providers, &|key| std::env::var(key).ok());
    }
```

---

**Same file, line 498:**

**Find this code:**
```rust
    fn interpolate_env_vars_with(
```

**Replace with:**
```rust
    /// Internal: walk provider config strings and expand `${VAR}` references.
    ///
    /// Only provider fields are walked. This is intentional -- expanding
    /// arbitrary config fields risks unintended side effects (e.g., a model
    /// slug containing `${...}` should be literal, not interpolated).
    fn interpolate_env_vars_with(
```

## Verification

```bash
# 1. Compile the config module
cargo check -p roko-core

# 2. Run config tests
cargo test -p roko-core -- config

# 3. Verify deprecated functions delegate to unified loader
grep -n 'load_config_impl\|load_config_validated_with_options' crates/roko-core/src/config/mod.rs
# Should show load_config_validated_with_options, not load_config_impl (for the deprecated fns)

# 4. Verify ROKO__* documentation is corrected
grep -n 'ROKO__' crates/roko-core/src/config/loader.rs
# Should show "not currently implemented" note, not the old precedence claim

# 5. Verify merge_global_into handles budget/conductor
grep -n 'max_plan_usd\|max_agents' crates/roko-core/src/config/loader.rs
# Should show the new merge sections

# 6. Verify env context in diagnostics
grep -n 'env_override_note' crates/roko-core/src/config/loader.rs
# Should show the diagnostic key
```

## Agent Prompt

```
You are implementing W14-D: five config system fixes in the roko codebase.
Workspace root: /Users/will/dev/nunchi/roko/roko/

Read the batch file at /Users/will/dev/nunchi/roko/roko/tmp/solutions/demo-running/batches/W14-D-config-fixes.md for full instructions.

## Files to modify

1. `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/config/loader.rs`
   - Fix 13.1 (lines 1-15): Replace module docstring to list all 12 supported ROKO_* env vars and note ROKO__SECTION__FIELD is NOT implemented
   - Fix 13.1 (line 29): Update LoadOptions.apply_env_overrides comment
   - Fix 13.3 (lines 388-401): Extend merge_global_into to also merge agent.default_effort, budget.max_plan_usd (when default 25.0), conductor.max_agents (when default 8)
   - Fix 13.4 (line 262): Add env-awareness diagnostic note at end of collect_diagnostics when ROKO_* env vars are detected

2. `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/config/mod.rs`
   - Fix 13.2 (lines 95-114): Change both deprecated load_config and load_config_strict to delegate to loader::load_config_validated_with_options

3. `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/config/schema.rs`
   - Fix 13.5 (line 493): Add scope documentation to interpolate_env_vars explaining only provider fields are interpolated
   - Fix 13.5 (line 498): Add doc comment to interpolate_env_vars_with explaining provider-only scope

## Key context
- AgentConfig has `default_effort: String` field (in config/agent.rs)
- BudgetConfig has `max_plan_usd: f32` with default 25.0 (in config/budget.rs)
- ConductorConfig has `max_agents: usize` with default 8 (in config/schema.rs line 1006)
- LoadOptions struct has fields: merge_global, apply_env_overrides, strict_validation
- `load_config_validated_with_options(workdir, opts)` is at line 106 in loader.rs

## Key details
- The batch file has exact "Find this code:" / "Replace with:" pairs for every change
- Read each source file FIRST to verify line numbers before editing
- Add `tracing::debug!` instrumentation at merge points and deprecated function calls
- Use f32 epsilon comparison for budget defaults (not ==)
- Do NOT run cargo build/test/clippy/fmt -- compilation is deferred
```

## Commit

This batch is committed with all Wave 14 batches together. Do not commit individually.

## Checklist

- [ ] 13.1: Module docstring lists all 12 supported env vars in a table
- [ ] 13.1: Module docstring notes `ROKO__SECTION__FIELD` is not implemented
- [ ] 13.1: `LoadOptions.apply_env_overrides` comment updated (no `ROKO__*`)
- [ ] 13.2: `load_config` delegates to `loader::load_config_validated_with_options`
- [ ] 13.2: `load_config_strict` delegates to unified loader with strict options
- [ ] 13.2: `tracing::debug!` at both deprecated function entries
- [ ] 13.3: `merge_global_into` merges `agent.default_effort`
- [ ] 13.3: `merge_global_into` merges `budget.max_plan_usd` when project uses default (25.0)
- [ ] 13.3: `merge_global_into` merges `conductor.max_agents` when project uses default (8)
- [ ] 13.3: `tracing::debug!` at each merge point
- [ ] 13.4: `collect_diagnostics` adds `_env_override_note` when ROKO_* vars detected
- [ ] 13.5: `interpolate_env_vars` doc comment describes scope limitation
- [ ] 13.5: `interpolate_env_vars_with` doc comment explains provider-only scope
- [ ] Pre-commit checks pass

## Audit Status

Audited: 2026-05-05. PASS no changes needed.
