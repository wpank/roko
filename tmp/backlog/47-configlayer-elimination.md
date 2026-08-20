# 47 — ConfigLayer Elimination

**Priority**: P2 — reduces maintenance burden and eliminates dual-source-of-truth bugs in config loading
**Size**: L (3-5 days)
**Crates**: `crates/roko-cli` (`src/config.rs`), `crates/roko-core` (`src/config/schema.rs`, `src/config/loader.rs`, `src/config/project.rs`)
**Depends on**: None

---

## Background

Roko uses a configuration system built in two layers. The "core" layer, in `crates/roko-core`, is the modern, schema-validated approach that handles env var layering, ancestor directory walking, secret interpolation, and semantic validation. It is authoritative for `providers`, `models`, and `agent` fields.

The "CLI" layer, in `crates/roko-cli/src/config.rs`, is a legacy TOML deserialization system built around a struct called `ConfigLayer`. It handles fields that have historically only existed in the CLI: `auto_plan`, `repos`, `dreams`, `daimon`, and `runner.plan_timeout_secs`. After both systems run, a function called `apply_core_authoritative_overrides()` patches the CLI-produced `Config` struct with values from the core-validated result, ensuring the core loader wins for the shared fields.

The problem is that `ConfigLayer` is approximately 1,500 lines of parallel schema definitions — `ProviderLayer`, `ModelProfileLayer`, `ServeLayer`, `RunnerLayer`, `AgentLayer`, `DreamsLayer`, `DaimonLayer`, and a dozen more — that duplicate schema already defined in `roko-core`. Any new config field must be added to both schemas or explicitly documented as CLI-only. The `ConfigSources` provenance struct only covers fields resolved through `ConfigLayer`, so `roko config show` reports wrong or stale provenance for provider and model fields. The dual-loader architecture is a continual source of subtle bugs when the two systems diverge.

The goal is to eliminate `ConfigLayer` by migrating its remaining CLI-only fields into `RokoConfig` (the core schema), then shrinking `load_resolved_config()` to a thin wrapper around the single core loader call.

Note: `auto_plan` and `runner.plan_timeout_secs` already exist in `RokoConfig` — `auto_plan` is at `RokoConfig::prd.auto_plan` (`crates/roko-core/src/config/project.rs:56`) and `plan_timeout_secs` is at `RokoConfig::runner.plan_timeout_secs` (`crates/roko-core/src/config/schema.rs:2195`). The remaining CLI-only fields are `dreams`, `daimon`, and `repos`.

## Current State

1. `ConfigLayer` struct is defined at `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/config.rs:1042`. It has 12 optional fields and over a dozen sub-structs (`DreamsLayer` at line 655, `DaimonLayer` at line 720, `RunnerLayer` at line 2691, etc.).

2. `collect_env_override_layer()` at line 2385 parses `ROKO_*` env vars and populates a `ConfigLayer`. This path is separate from the core loader's hierarchical `ROKO__*` env var support.

3. `compute_sources()` at line 3293 and `sources_from_layer()` at line 3403 build the `ConfigSources` provenance struct from two `ConfigLayer` instances. `ConfigSources` (line 2974) has 23 fields, none of which track values arriving via the core loader — so `providers`, `models`, and `agent.*` always show stale provenance.

4. `apply_core_authoritative_overrides()` at line 3129 overlays core `providers`, `models`, and all `agent.*` fields onto the CLI-resolved `Config`. The comment at line 3109 explains why this overlay is necessary.

5. `load_resolved_config()` at line 3195 is the dual-loader entry point: it calls both `roko_core::config::loader::load_config_validated_with_options()` and the legacy `ConfigLayer` path, then calls `apply_core_authoritative_overrides()` to reconcile them.

6. The `KNOWN_CONFIG_KEYS` constant at line 3031 lists which top-level TOML keys exist in each schema. The comment at line 3071 identifies the CLI-only keys: `auto_plan`, `dreams`, `daimon`, `prompt`, `gate` (legacy alias), `executor`, `runtime`, `repos`.

7. In `RokoConfig` (`crates/roko-core/src/config/schema.rs`):
   - `auto_plan` already exists as `prd.auto_plan` (line 97, type defined in `crates/roko-core/src/config/project.rs:56`)
   - `runner.plan_timeout_secs` already exists in `CoreRunnerConfig` (line 2195)
   - `dreams` and `daimon` do NOT exist in `RokoConfig` — there are no `DreamsConfig` or `DaimonConfig` fields on the struct
   - `repos` (per-repository blocks) does NOT exist in `RokoConfig`

## Implementation Plan

**Phase 1: Add `dreams` and `daimon` to `RokoConfig`**

Read `DreamsLayer::resolve()` at `crates/roko-cli/src/config.rs:698` to understand the concrete `DreamsConfig` type it produces, and `DaimonLayer::resolve()` nearby to understand `DaimonConfig`. These types already exist in `crates/roko-cli/src/config.rs` — they're the output of the layer resolution. Move the `DreamsConfig` and `DaimonConfig` structs into `crates/roko-core` (or re-export them), then add `dreams: DreamsConfig` and `daimon: DaimonConfig` fields to `RokoConfig`.

Update the core loader (`crates/roko-core/src/config/loader.rs`) to read and validate these fields using the same env-var approach as other config sections.

**Phase 2: Add `repos` to `RokoConfig` or hoist `RepoConfig` to core**

`RepoConfig` is defined in `crates/roko-cli/src/config.rs`. Add a `repos: Vec<RepoConfig>` field to `RokoConfig` so the core loader reads it. Move or re-export `RepoConfig` to `roko-core` if needed.

**Phase 3: Replace `ConfigLayer` with a thin adapter**

Once the core loader handles all fields that were CLI-only, change `ConfigLayer::from_file()` to delegate to the core loader rather than maintaining its own deserialization chain. The `load_resolved_config()` function should become:

```rust
pub fn load_resolved_config(workdir: &Path) -> Result<ResolvedConfig> {
    let paths = resolve_paths(workdir);
    let core_validated = roko_core::config::loader::load_config_validated_with_options(
        workdir,
        &roko_core::config::loader::LoadOptions::default(),
    )?;
    let config = Config::from_roko_config(core_validated.config());
    let repo_registry = RepoRegistry::load(&config, workdir)?;
    let sources = ConfigSources::from_provenance(core_validated.provenance());
    Ok(ResolvedConfig { config, repo_registry, sources, paths })
}
```

**Phase 4: Delete dead code**

Remove: `ConfigLayer` and all sub-structs (`ProviderLayer`, `ModelProfileLayer`, `ServeLayer`, `RunnerLayer`, `AgentLayer`, `DreamsLayer`, `DaimonLayer`, `LearningLayer`, etc.), `collect_env_override_layer()`, `collect_env_override_layer_from()`, `compute_sources()`, `sources_from_layer()`, `apply_core_authoritative_overrides()`, `apply_layer_value()`, `warn_dropped_toml_keys()`, and the four helper functions `provider_layer_mut`, `model_layer_mut`, `serve_auth_layer_mut`, `serve_deploy_layer_mut`.

**Phase 5: Update `ConfigSources` provenance**

Replace `ConfigSources` with a wrapper around the core loader's provenance map (the `LoadDiagnostics` or equivalent output from `load_config_validated_with_options`). Update `roko config show` to display provenance from the core source.

## Acceptance Criteria

1. After Phase 4: `grep -rn 'ConfigLayer' crates/ --include='*.rs' | grep -v target/ | grep -v 'test'` returns zero hits.
2. `roko config show` correctly reports the provenance (file path or env var) for every displayed field, including `auto_plan`, `providers`, and `models`. The output is identical whether the config is loaded via `roko` (CLI entry point) or `roko serve` (server entry point).
3. Setting `auto_plan = true` in `roko.toml` and running `roko config show` displays `auto_plan: true (source: project)`. Setting `ROKO_AUTO_PLAN=false` overrides it and displays `auto_plan: false (source: env)`.
4. `cargo test --workspace` passes with zero failures after each phase.
5. `cargo clippy --workspace --no-deps -- -D warnings` is clean after Phase 4.
6. No existing `roko.toml` config file that was valid before Phase 1 produces a parse error after Phase 4. The migration must be backward-compatible.

## Verification Checklist

- [ ] Create a `roko.toml` with `auto_plan = true`, `[dreams]`, `[daimon]`, and `[[repos]]` sections and verify `roko config show` reads them without error
- [ ] Verify `ROKO_AUTO_PLAN=false roko config show` shows env-source provenance for `auto_plan`
- [ ] Verify `ROKO__RUNNER__PLAN_TIMEOUT_SECS=7200 roko config show` shows env-source provenance for `runner.plan_timeout_secs`
- [ ] Run `cargo test --workspace` after each phase — must pass
- [ ] Run `cargo clippy --workspace --no-deps -- -D warnings` after Phase 4 — must be clean
- [ ] Run `roko serve` and verify it starts without error using the same config

## Files to Modify

| File | Change |
|---|---|
| `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/config/schema.rs` | Add `dreams: DreamsConfig` and `daimon: DaimonConfig` fields to `RokoConfig`; add `repos: Vec<RepoConfig>` |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/config/loader.rs` | Parse and validate the new fields; add env var hooks |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/config/project.rs` | May need to host `DreamsConfig` / `DaimonConfig` / `RepoConfig` types if moved from CLI |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/config.rs` | Phases 3-5: replace `ConfigLayer` system with core delegation, delete ~1,500 LOC; update `ConfigSources` |
