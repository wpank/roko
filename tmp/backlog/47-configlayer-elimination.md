# ConfigLayer Elimination

**Priority**: P2
**Size**: L (3-5 days)

---

## Problem

The CLI config system has a dual-loader architecture where `load_resolved_config()` in
`crates/roko-cli/src/config.rs` runs two separate config loading paths and then merges
their results:

1. **Core loader**: `roko_core::config::loader::load_config_validated_with_options()` —
   the modern, schema-validated, provenance-tracked loader. Handles env-var layering,
   ancestor walk, `ROKO_CONFIG` env override, file secret resolution, and diagnostics.
   It is the authoritative source for `providers`, `models`, and `agent` fields.

2. **Legacy loader**: `ConfigLayer` struct + `collect_env_override_layer()` +
   `compute_sources()` — approximately 1,500 lines of TOML-parsing, merging, and source
   tracking code. Used to handle CLI-only fields that have no equivalent in `RokoConfig`:
   `auto_plan`, `repos` (per-repository blocks), `dreams`, `daimon`, and
   `runner.plan_timeout_secs`.

After both loaders run, `apply_core_authoritative_overrides()` (line ~3127) overlays the
core result's `providers`, `models`, and `agent` fields onto the `Config` struct produced
by the legacy `ConfigLayer` path. This overlay is necessary because the two loaders
otherwise produce independent, potentially conflicting results for those shared fields.

The dual-loader creates several problems:

- **Two sources of truth**: Any field that exists in both `ConfigLayer` and `RokoConfig`
  is resolved twice. Bugs in either path can cause the wrong value to win. The overlay
  function is the only thing preventing silent discards.
- **Maintenance burden**: New config fields must be added to both schemas and both
  resolution paths, or must be explicitly noted as CLI-only. The comment at line ~3026
  lists the known CLI-only keys to avoid redundant dual-parsing.
- **Provenance gaps**: `ConfigSources` (the struct that tracks where each config value
  came from) only covers the `ConfigLayer` path. Values arriving via the core loader
  have no provenance entry in `ConfigSources`. `roko config show` therefore shows
  stale/incorrect provenance for provider and model fields.
- **Dead code accumulation**: `ConfigLayer` has dozens of sub-structs (`ProviderLayer`,
  `ModelProfileLayer`, `ServeLayer`, `RunnerLayer`, etc.) that duplicate schema definitions
  already in `roko-core`. These structs are load-bearing only because `ConfigLayer.resolve()`
  must produce a `Config`, but the values are immediately overwritten by
  `apply_core_authoritative_overrides()`.

---

### What already exists

| Component | Location | Status |
|---|---|---|
| `ConfigLayer` struct | `crates/roko-cli/src/config.rs:1042` | EXISTS — legacy TOML loader with ~1,500 LOC of sub-structs and merge logic |
| `collect_env_override_layer()` | `crates/roko-cli/src/config.rs:2385` | EXISTS — parses `ROKO_*` env vars into a `ConfigLayer` |
| `compute_sources()` | `crates/roko-cli/src/config.rs:3291` | EXISTS — builds `ConfigSources` provenance from two `ConfigLayer` instances |
| `apply_core_authoritative_overrides()` | `crates/roko-cli/src/config.rs:3127` | EXISTS — overlays core `providers`/`models`/`agent` onto legacy-resolved `Config` |
| `load_resolved_config()` | `crates/roko-cli/src/config.rs:3193` | EXISTS — calls both loaders, applies overlay, returns `ResolvedConfig` |
| `load_config_validated_with_options()` | `crates/roko-core/src/config/loader.rs` | EXISTS — modern core loader, authoritative |
| `RokoConfig` schema | `crates/roko-core/src/config/schema.rs` | EXISTS — does not currently have `auto_plan`, `repos`, `dreams`, `daimon` |
| CLI-only key list | `crates/roko-cli/src/config.rs:3069` | EXISTS — comment listing fields that only exist in `ConfigLayer` |
| `ConfigSources` | `crates/roko-cli/src/config.rs:2975` | EXISTS — provenance tracking struct, covers legacy fields only |

---

### What to build (phased)

**Phase 1: Move CLI-only fields into `RokoConfig`**

Add the following fields to `RokoConfig` in `crates/roko-core/src/config/schema.rs`:

- `auto_plan: bool` (default false) — already has a `ConfigLayer` field and env-var
  parsing via `ROKO_AUTO_PLAN`.
- `repos: Vec<RepoConfig>` (default empty) — per-repository blocks; the type already
  exists in `crates/roko-cli/src/config.rs`.
- `dreams: DreamsConfig` — dream-cycle settings; the shape exists in `DreamsLayer`.
- `daimon: DaimonConfig` — daimon settings; the shape exists in `DaimonLayer`.
- `runner.plan_timeout_secs: u64` — already in `RunnerLayer`, not in core `RunnerConfig`.

Update the core loader (`loader.rs`) to read and validate each new field. Add
corresponding `ROKO__*` env var support for the hierarchical override path.

**Phase 2: Replace `ConfigLayer` with a thin adapter**

Once the core loader handles all fields, `ConfigLayer` should become a facade:
`ConfigLayer::from_file(path)` parses the TOML into a `serde_json::Value` and delegates
to the core loader rather than maintaining its own deserialization chain. The
`ConfigLayer::resolve()` method should call the core loader and map the result back into
the `Config` struct that CLI code depends on.

This means `compute_sources()` can be replaced with a call to the core loader's
`provenance` output (which already tracks each field's origin).

**Phase 3: Delete the legacy code**

Remove:
- The `ConfigLayer` struct and all sub-structs (`ProviderLayer`, `ModelProfileLayer`,
  `ServeLayer`, `RunnerLayer`, `AgentLayer`, `DreamsLayer`, `DaimonLayer`, etc.)
- `collect_env_override_layer()` and `collect_env_override_layer_from()`
- `compute_sources()` and `sources_from_layer()`
- `apply_core_authoritative_overrides()`
- `apply_layer_value()` (the key=value env var parser)
- `warn_dropped_toml_keys()` (superseded by core loader diagnostics)
- The `provider_layer_mut`, `model_layer_mut`, `serve_auth_layer_mut`,
  `serve_deploy_layer_mut` helper functions

Target: `load_resolved_config()` becomes a thin wrapper around the single core loader call
plus construction of `ConfigPaths` and `RepoRegistry`.

**Phase 4: Unify provenance tracking**

Replace `ConfigSources` with a wrapper around the core loader's `LoadDiagnostics` or
provenance map. Update `roko config show` to display provenance from the core source rather
than the now-deleted `ConfigSources` struct.

---

## Where to make changes

| File | Change |
|---|---|
| `crates/roko-core/src/config/schema.rs` | Add `auto_plan`, `repos`, `dreams`, `daimon`, `runner.plan_timeout_secs` |
| `crates/roko-core/src/config/loader.rs` | Parse and validate the new fields; add env var hooks |
| `crates/roko-cli/src/config.rs` | Phase 2–4: replace `ConfigLayer` system with core delegation, delete ~1,500 LOC |

---

## Acceptance criteria

1. `grep -rn 'ConfigLayer' crates/ --include='*.rs' | grep -v target/ | grep -v 'test'`
   returns zero hits after Phase 3.
2. `roko config show` correctly reports the provenance (file path or env var) for every
   displayed field, including `auto_plan`, `providers`, and `models`. The output is
   identical whether the config is loaded via `roko` (CLI entry point) or `roko serve`
   (server entry point).
3. Setting `auto_plan = true` in `roko.toml` and running `roko config show` displays
   `auto_plan: true (source: project)`. Setting `ROKO_AUTO_PLAN=false` overrides it and
   displays `auto_plan: false (source: env)`.
4. `cargo test --workspace` passes with zero failures after each phase.
5. `cargo clippy --workspace --no-deps -- -D warnings` is clean after Phase 3.
6. No existing `roko.toml` config file that was valid before Phase 1 produces a parse
   error after Phase 3. The migration must be backward-compatible.

---

## References

- `crates/roko-cli/src/config.rs:1042` — `ConfigLayer` struct definition
- `crates/roko-cli/src/config.rs:2385` — `collect_env_override_layer()` entry point
- `crates/roko-cli/src/config.rs:3069` — comment listing CLI-only keys
- `crates/roko-cli/src/config.rs:3109` — `apply_core_authoritative_overrides()` doc comment explaining the hybrid state
- `crates/roko-cli/src/config.rs:3193` — `load_resolved_config()`, the dual-loader entry point
- `crates/roko-core/src/config/schema.rs` — `RokoConfig` schema (fields to be added)
- `crates/roko-core/src/config/loader.rs` — `load_config_validated_with_options()`, the target single loader
