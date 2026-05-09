# RFC: Unified Config Loading -- Eliminating the Dual-Loader Deception

**Status**: Draft
**Task**: 001 (config-foundation / wave-0 / critical)
**Author**: Design review
**Date**: 2026-05-05

---

## 1. Problem Statement

The CLI has two config loading systems that run in parallel but produce
different configs. The core loader's output is discarded. Every CLI command
that uses `load_resolved_config()` gets config values from the legacy system,
not the core loader. Users who set `ROKO__AGENT__CONTEXT_LIMIT_K=32` (the
documented hierarchical override syntax) see the override applied... into a
value that is thrown away.

---

## 2. Current Architecture

### 2.1. ASCII Architecture Diagram

```
                         User runs `roko <cmd>`
                                  |
                                  v
                    +---------------------------+
                    |  load_resolved_config()   |
                    |  crates/roko-cli/src/     |
                    |  config.rs:2895           |
                    +---------------------------+
                       |                    |
            CALLED     |                    |  USED
            (discarded)|                    |  (returns to caller)
                       v                    v
        +-------------------+    +------------------------+
        | Core Loader       |    | Legacy ConfigLayer     |
        | roko-core/src/    |    | System                 |
        | config/loader.rs  |    | (same file, ~2900 LOC) |
        +-------------------+    +------------------------+
        |                   |    |                        |
        | - ancestor walk   |    | - from_file()          |
        | - global merge    |    | - merge() chains       |
        | - ROKO_* named    |    | - resolve() to Config  |
        |   env overrides   |    | - env_override_path()  |
        | - ROKO__*         |    | - collect_env_override |
        |   hierarchical    |    |   _layer()             |
        |   env overrides   |    | - compute_sources()    |
        | - interpolation   |    |                        |
        | - file secrets    |    |                        |
        | - validation      |    |                        |
        | - provenance      |    |                        |
        +-------------------+    +------------------------+
                |                           |
                v                           v
        _core_validated             ResolvedConfig {
        (prefixed with _,             config: Config,         <-- ALL callers
         NEVER READ)                  repo_registry,              get THIS
                                      sources: ConfigSources,
                                      paths: ConfigPaths,
                                    }
```

### 2.2. The Deception Point

File: `crates/roko-cli/src/config.rs`, line 2903:

```rust
let _core_validated = roko_core::config::loader::load_config_validated_with_options(
    workdir,
    &roko_core::config::loader::LoadOptions::default(),
)
.map_err(|e| anyhow!("core config loader: {e}"))?;
```

The `_` prefix suppresses the "unused variable" compiler warning. The code
then proceeds to build `ResolvedConfig` entirely from the legacy
`ConfigLayer` system (lines 2909-2950), ignoring `_core_validated`.

### 2.3. Two Parallel Env Override Systems

| Feature | Core Loader | Legacy System |
|---|---|---|
| **Location** | `loader.rs` `apply_hierarchical_env_overrides()` | `config.rs` `collect_env_override_layer()` |
| **Syntax** | `ROKO__SECTION__FIELD` via serde TOML roundtrip | `ROKO__SECTION__FIELD` via `env_override_path()` + `apply_layer_value()` |
| **Named env vars** | `ROKO_MODEL`, `ROKO_BACKEND`, etc. via `apply_process_env()` | Not handled (only hierarchical) |
| **Output** | Into `_core_validated` (discarded) | Into `ResolvedConfig` (used) |
| **Provenance** | `ConfigProvenance` entries (discarded) | `ConfigSources` per-field tags (used by `config show`) |

Both systems parse `ROKO__*` vars identically (strip prefix, lowercase,
`__` -> `.`), but they apply the values to different config types
(`RokoConfig` vs `ConfigLayer`) that are structurally incompatible.

### 2.4. Two Config Types

| Type | Location | Fields | Used By |
|---|---|---|---|
| `RokoConfig` | `roko-core/src/config/schema.rs` | 24 sections (agent, gates, routing, conductor, etc.) | Core loader, orchestrate.rs, daemon.rs, serve, ~35 callsites |
| `Config` | `roko-cli/src/config.rs:28` | 18 fields (agent, gates, dreams, daimon, repos, etc.) | `load_resolved_config()` -> ~25 callsites |

The types overlap substantially but are not identical. CLI `Config` has fields
absent from `RokoConfig` (`auto_plan`, `dreams`, `daimon`, `repos`, `[[gate]]`
array syntax, `prompt`, `runner.plan_timeout_secs`). Conversely, `RokoConfig`
has ~20 sections that `Config` does not represent.

---

## 3. Every Callsite of `load_resolved_config`

### 3.1. Direct callsites (get legacy config)

| # | File | Line | Usage |
|---|---|---|---|
| 1 | `config.rs` | 2895 | Definition |
| 2 | `config.rs` | 2959 | `load_layered()` deprecated wrapper |
| 3 | `main.rs` | 2840 | Global config access for `roko run` dispatch |
| 4 | `config_cmd.rs` | 216 | `cmd_show()` |
| 5 | `config_cmd.rs` | 223 | `cmd_path()` |
| 6 | `config_cmd.rs` | 594 | `cmd_providers_health()` |
| 7 | `config_cmd.rs` | 626 | `cmd_migrate()` |
| 8 | `commands/plan.rs` | 279 | `cmd_plan()` -> `RunConfig` construction |
| 9 | `commands/agent.rs` | 30 | Agent subcommand dispatch |
| 10 | `commands/server.rs` | 268 | Webhook config for serve |
| 11 | `commands/job.rs` | 133 | Auth config for job create |
| 12 | `commands/job.rs` | 291 | Auth config for job execute |
| 13 | `commands/do_cmd.rs` | 39 | Preview config for do command |
| 14 | `commands/do_cmd.rs` | 418 | Execution config for do command |
| 15 | `run.rs` | 370 | `run_once()` entry point |
| 16 | `unified.rs` | 259 | Unified dispatch path |
| 17 | `doctor.rs` | 277 | Workspace diagnosis |
| 18 | `daemon.rs` | 321 | Daemon startup |
| 19 | `prd.rs` | 955 | `auto_plan` check |
| 20 | `prd.rs` | 1021 | PRD plan generation |
| 21 | `bench_demo.rs` | 562 | Benchmark demo config |
| 22 | `dispatch_v2.rs` | 73 | V2 dispatch path |
| 23 | `chat_inline.rs` | 1046 | Chat inline config |
| 24 | `chat_inline.rs` | 1534 | Chat inline model selection |
| 25 | `lib.rs` | 140 | Re-export as `load_layered()` (deprecated) |

### 3.2. Direct `load_config_unified` callsites (get core config)

These callsites correctly use the core loader. They coexist with the
legacy callsites above, meaning the same command can read different
config values depending on which code path executes.

| # | File | Line |
|---|---|---|
| 1 | `orchestrate.rs` | 877 |
| 2 | `run.rs` | 383, 1888, 2459, 2777, 3049 |
| 3 | `daemon.rs` | 320 |
| 4 | `bootstrap.rs` | 67 |
| 5 | `event_sources.rs` | 39 |
| 6 | `auth_detect.rs` | 66 |
| 7 | `prd.rs` | 916 |
| 8 | `unified.rs` | 212 |
| 9 | `chat_inline.rs` | 1540 |
| 10 | `chat_session.rs` | 589 |
| 11 | `agent_serve.rs` | 362 |
| 12 | `agent_exec.rs` | 100, 269 |
| 13 | `learning_helpers.rs` | 412 |
| 14 | `model_selection.rs` | 199 |
| 15 | `subscriptions.rs` | 121 |
| 16 | `worker/cloud.rs` | 453 |
| 17 | `tui/app.rs` | 3812 |
| 18 | `commands/server.rs` | 34, 264, 416, 434 |
| 19 | `commands/research.rs` | 24 |
| 20 | `commands/plan.rs` | 925 |
| 21 | `commands/util.rs` | 265 |
| 22 | `commands/prd.rs` | 847 |
| 23 | `commands/config_cmd.rs` | 280, 375, 405, 468, 623, 707, 788 |
| 24 | `commands/do_cmd.rs` | 441 |
| 25 | `commands/tune.rs` | 98 |
| 26 | `commands/learn.rs` | 159 |
| 27 | `vision_loop/orchestrator.rs` | 77 |

### 3.3. Files that call BOTH loaders

These files are the most dangerous -- they may use different config values
in different code paths within the same command execution:

| File | Legacy call | Core call |
|---|---|---|
| `run.rs` | line 370 | lines 383, 1888, 2459, 2777, 3049 |
| `daemon.rs` | line 321 | line 320 |
| `unified.rs` | line 259 | line 212 |
| `chat_inline.rs` | lines 1046, 1534 | line 1540 |
| `prd.rs` | line 955, 1021 | line 916 |
| `commands/do_cmd.rs` | lines 39, 418 | line 441 |
| `commands/server.rs` | line 268 | lines 34, 264, 416, 434 |

---

## 4. What the Legacy System Provides That Core Does Not

The reason `load_resolved_config` cannot simply be replaced by
`load_config_unified` is that the legacy system carries four surfaces
that downstream code depends on:

### 4.1. `Config` (the CLI config type)

Unique fields not in `RokoConfig`:

| Field | Used By | Migration Path |
|---|---|---|
| `auto_plan: bool` | `prd.rs:955`, `config_cmd.rs:670` | Move to `RokoConfig.prd.auto_plan` (already exists there) |
| `dreams: DreamsConfig` | `orchestrate.rs:8107+`, `config_cmd.rs:745-756` | Move to `RokoConfig` or read from TOML directly |
| `daimon: DaimonConfig` | `orchestrate.rs` (affect engine) | Move to `RokoConfig` or read from TOML directly |
| `repos: Vec<RepoConfig>` | `serve_runtime.rs:199+` | Move to `RokoConfig.project` or keep in adapter |
| `gates: Vec<GateConfig>` | `orchestrate.rs:8300-8301` | Already in `RokoConfig.gates` (different shape) |
| `prompt: PromptConfig` | `orchestrate.rs:15773+`, `config_cmd.rs:725-731` | Add to `RokoConfig` or adapt from compose config |
| `runner: RunnerConfig` | `config_cmd.rs:765` | `plan_timeout_secs` -> `RokoConfig.conductor` |
| `tools: ToolsConfig` | `config_cmd.rs:710-720` | Already in `RokoConfig.tools` (partial overlap) |
| `budget: BudgetConfig` | `orchestrate.rs:11320+` (heavily used) | Already in `RokoConfig.budget` (partial overlap) |

### 4.2. `RepoRegistry`

Loaded from `Config.repos`. Used only in `serve_runtime.rs` (3 callsites).
Can be loaded separately from the TOML file.

### 4.3. `ConfigSources` (per-field provenance tags)

21 fields, each a `Source` enum (`Global`/`Project`/`Default`/`Env`). Used by:
- `config_cmd.rs:666-792` -- `print_resolved()` for `roko config show`
- `run.rs:373` -- checking if `agent_command` is configured
- `main.rs:2841-2842` -- checking if config is fully default

The core loader has a richer provenance system (`ConfigProvenance` with
`ConfigSource::File/Migration/Default/Env/LocalOverride/CliOverride`),
but it is per-entry rather than per-field, making it harder to map to the
field-level tags that `config show` requires.

### 4.4. `ConfigPaths`

Three fields: `global`, `project`, `env_override`. These are trivially
obtainable from core's `global_config_path()`, `discover_project_config()`,
and `ROKO_CONFIG` env var. The CLI already delegates `global_config_path()`
to core (line 2787).

---

## 5. Target Architecture

### 5.1. ASCII Target Diagram

```
                         User runs `roko <cmd>`
                                  |
                                  v
                    +---------------------------+
                    |  load_resolved_config()   |  <-- ADAPTER (thin)
                    |  config.rs                |
                    +---------------------------+
                                  |
                         DELEGATES TO
                                  |
                                  v
                    +---------------------------+
                    |  Core Loader              |
                    |  load_config_validated     |
                    |  _with_options()           |
                    +---------------------------+
                    |  Returns ValidatedConfig  |
                    |  {                         |
                    |    raw: RokoConfig,        |
                    |    migrated: RokoConfig,   |  <-- AUTHORITATIVE
                    |    diagnostics,            |
                    |    provenance,             |
                    |  }                         |
                    +---------------------------+
                                  |
                         ADAPTER builds
                                  |
                                  v
                    +---------------------------+
                    |  ResolvedConfig {          |  <-- COMPATIBILITY
                    |    config: Config,         |      (built FROM core)
                    |    repo_registry,          |
                    |    sources,                |
                    |    paths,                  |
                    |  }                         |
                    +---------------------------+
```

Key invariant: **No config values are ever read from `ConfigLayer` merge.
The adapter converts `RokoConfig` -> `Config` structurally.**

### 5.2. Single-Loader Design Principles

1. Core loader is the single source of truth for all effective config values.
2. CLI-only fields (`dreams`, `daimon`, `repos`, `prompt`) are either:
   a. Absorbed into `RokoConfig` schema (preferred), or
   b. Read from the raw TOML file separately and attached to the adapter output.
3. `ConfigSources` provenance is derived from core's `ConfigProvenance` entries.
4. `ConfigLayer` and its ~2900 lines of merge/resolve logic are dead code after
   migration and can be deleted.

---

## 6. Complete Migration Plan

### Phase 1: Core Loader Enhancements (0.5 day)

The core loader already supports everything needed. Verify and test:

**Already implemented:**
- Hierarchical `ROKO__SECTION__FIELD` env overrides (via serde TOML roundtrip)
- Named env vars (`ROKO_MODEL`, `ROKO_BACKEND`, etc.)
- Global + project merge with correct priority
- Provenance tracking for env overrides

**Verify with tests:**
```rust
#[test]
fn named_and_hierarchical_env_precedence() {
    // ROKO__AGENT__DEFAULT_MODEL should win over ROKO_MODEL
    // when both are applied (hierarchical runs after named).
}
```

**Add if missing:**
Fields that the CLI `Config` type has but `RokoConfig` does not, which need
to survive in the core schema to avoid the adapter needing to parse TOML
separately:

| Field | Target location in `RokoConfig` | Action |
|---|---|---|
| `auto_plan` | Already at `prd.auto_plan` | Verify mapping |
| `dreams.auto_dream` | Add `dreams: DreamsConfig` to schema | Add section |
| `dreams.idle_threshold_mins` | (same) | (same) |
| `dreams.min_episodes_for_dream` | (same) | (same) |
| `daimon.strategy_space` | Add `daimon: DaimonConfig` to schema | Add section |
| `prompt.token_budget` | Already partially in compose config | Add section or map |
| `prompt.role` | (same) | (same) |
| `prompt.context_budgets` | (same) | (same) |
| `repos` | Add `repos: Vec<RepoConfig>` to project | Add field |
| `runner.plan_timeout_secs` | Map to `conductor.plan_timeout_secs` | Add field or alias |
| `budget.max_task_usd` | Already at `budget.max_plan_usd` | Verify all fields |
| `budget.max_session_usd` | (same pattern) | Verify |

Estimated effort: **4 hours** (mostly adding missing sections to schema.rs
and writing tests).

### Phase 2: ResolvedConfig Adapter Design (1 day)

Replace the body of `load_resolved_config()` with a thin adapter that:

1. Calls `load_config_validated_with_options()` (core loader).
2. Converts `ValidatedConfig.migrated` (`RokoConfig`) -> CLI `Config`.
3. Builds `ConfigSources` from core's `ConfigProvenance` entries.
4. Builds `ConfigPaths` from core path helpers.
5. Loads `RepoRegistry` from the converted config.

#### 6.2.1. Config Conversion Function

```rust
/// Convert a core `RokoConfig` to CLI `Config`.
///
/// This is a structured conversion, not a serialization roundtrip.
/// Fields present in both types map directly. Fields unique to CLI
/// `Config` (dreams, daimon, prompt, repos, gates, runner) are mapped
/// from their equivalents in `RokoConfig` or use defaults.
fn config_from_roko(roko: &RokoConfig) -> Config {
    Config {
        agent: AgentConfig {
            command: roko.agent.default_backend.clone(),
            args: roko.agent.args.clone().unwrap_or_default(),
            model: if roko.agent.default_model.is_empty() {
                None
            } else {
                Some(roko.agent.default_model.clone())
            },
            effort: roko.agent.default_effort.clone(),
            bare_mode: roko.agent.bare_mode,
            fallback_model: roko.agent.fallback_model.clone(),
            timeout_ms: roko.agent.timeout_ms.unwrap_or(120_000),
            env: roko.agent.env.clone().unwrap_or_default(),
            // ... remaining fields
        },
        auto_plan: roko.prd.auto_plan,
        dreams: /* from roko.dreams once added to schema */,
        daimon: /* from roko.daimon once added to schema */,
        // ... remaining fields mapped structurally
        providers: roko.providers.clone(),
        models: roko.models.clone(),
        serve: roko.serve.clone(),
        // ...
    }
}
```

#### 6.2.2. ConfigSources from Core Provenance

```rust
/// Derive per-field provenance tags from core's ConfigProvenance entries.
fn sources_from_provenance(
    provenance: &[ConfigProvenance],
    config_path: &Option<PathBuf>,
    global_path: &Path,
) -> ConfigSources {
    let mut sources = ConfigSources::all_default();

    for entry in provenance {
        let source = match &entry.source {
            ConfigSource::File => {
                // Determine if this file is global or project
                if entry.path.as_deref() == Some(global_path) {
                    Source::Global
                } else {
                    Source::Project
                }
            }
            ConfigSource::Env | ConfigSource::CliOverride => Source::Env,
            ConfigSource::Default => Source::Default,
            _ => Source::Project, // LocalOverride, Migration
        };

        // Map core provenance key to CLI source field
        match entry.key.as_str() {
            "roko.toml" => {
                // File-level provenance: mark all fields as coming from this source
                // (unless overridden by more specific entries)
            }
            k if k.starts_with("agent.") => {
                // Map to appropriate agent_* source field
            }
            // ... etc
            _ => {}
        }
    }

    sources
}
```

**Note**: The current core provenance is coarser than the per-field granularity
that `ConfigSources` provides. Phase 2 can use a heuristic: if a project
config file exists, non-default fields are `Source::Project`; if only global
exists, they are `Source::Global`. Env override entries from core provenance
directly map to `Source::Env`. This matches the existing behavior closely
enough for `config show` output.

#### 6.2.3. New `load_resolved_config` Body

```rust
pub fn load_resolved_config(workdir: &Path) -> Result<ResolvedConfig> {
    // 1. Core loader: single source of truth
    let validated = roko_core::config::loader::load_config_validated_with_options(
        workdir,
        &roko_core::config::loader::LoadOptions::default(),
    )
    .map_err(|e| anyhow!("config loader: {e}"))?;

    let roko_config = validated.config();

    // 2. Convert RokoConfig -> CLI Config
    let config = config_from_roko(roko_config);

    // 3. Build paths
    let paths = ConfigPaths {
        global: roko_core::config::loader::global_config_path(),
        project: roko_core::config::loader::discover_project_config(workdir),
        env_override: std::env::var_os("ROKO_CONFIG").map(PathBuf::from),
    };

    // 4. Build provenance from core entries
    let sources = sources_from_provenance(
        validated.provenance(),
        &paths.project,
        &paths.global,
    );

    // 5. Load repo registry
    let repo_registry = RepoRegistry::load(&config, workdir)?;

    Ok(ResolvedConfig {
        config,
        repo_registry,
        sources,
        paths,
    })
}
```

Estimated effort: **8 hours** (conversion function, provenance mapping, tests).

### Phase 3: Callsite-by-Callsite Migration (1 day)

No callsites need to change their calling convention -- they all call
`load_resolved_config()` and get `ResolvedConfig` back. The migration is
internal to `load_resolved_config()` itself. However, we need to verify
every callsite to ensure behavioral equivalence.

#### 3a. Callsites that only read `config` (low risk, verify only)

These read `.config.X` fields and will work identically once
`config_from_roko()` maps correctly:

| Callsite | Reads |
|---|---|
| `main.rs:2840` | `sources.agent_command`, `sources.prompt_token_budget` |
| `commands/plan.rs:279` | `.config` (passed to `RunConfig`) |
| `commands/agent.rs:30` | `.config` (agent dispatch) |
| `commands/server.rs:268` | `.config.serve.deploy.webhooks` |
| `commands/job.rs:133,291` | `.config.serve.auth` |
| `commands/do_cmd.rs:39,418` | `.config` (preview + execution) |
| `run.rs:370` | `.config`, `.sources.agent_command` |
| `daemon.rs:321` | `.config` |
| `doctor.rs:277` | `.config` |
| `prd.rs:955` | `.config.auto_plan` |
| `prd.rs:1021` | full resolved |
| `bench_demo.rs:562` | `.config` (mutable) |
| `dispatch_v2.rs:73` | `.config` |
| `chat_inline.rs:1046,1534` | `.config` |
| `unified.rs:259` | `.config` |

#### 3b. Callsites that read `sources` (medium risk, verify provenance mapping)

| Callsite | Reads |
|---|---|
| `config_cmd.rs:216` -> `print_resolved()` | ALL source fields |
| `run.rs:370` | `sources.agent_command` |
| `main.rs:2840` | `sources.agent_command`, `sources.prompt_token_budget` |

These need the provenance mapping (Phase 2, step 4) to be correct.

#### 3c. Callsites that read `paths` (trivial risk)

| Callsite | Reads |
|---|---|
| `config_cmd.rs:223` | `paths.project`, `paths.global` |
| `config_cmd.rs:594` | (providers health) |

These already delegate to core for path resolution.

#### 3d. Callsites that read `repo_registry` (low risk)

| Callsite | Reads |
|---|---|
| `serve_runtime.rs:199,228,234,240` | repo lookup |

Loaded from `Config.repos` which maps from `RokoConfig` repos field.

Estimated effort: **8 hours** (verification, edge case handling).

### Phase 4: Compatibility Wrapper and Removal Timeline (0.5 day)

#### 4a. Deprecation schedule

| Week | Action |
|---|---|
| W1 (now) | Phase 1-2: Core enhancements + adapter |
| W2 | Phase 3: Verify all callsites |
| W2 | `load_layered()` marked deprecated (already done) |
| W3 | `ConfigLayer::from_file`, `ConfigLayer::parse_toml`, `ConfigLayer::merge`, `ConfigLayer::resolve` marked deprecated |
| W4 | Remove `collect_env_override_layer()`, `env_override_path()`, `apply_env_source_overrides()` |
| W6 | Remove `ConfigLayer`, `*Layer` types, `compute_sources()` |
| W8 | Remove CLI `Config` type; migrate remaining consumers to `RokoConfig` directly |

#### 4b. Temporary compatibility wrappers

During W1-W6, the following wrappers exist:

```rust
// Keep: thin adapter that converts core -> CLI types
pub fn load_resolved_config(workdir: &Path) -> Result<ResolvedConfig>;

// Keep until W8: struct conversion
fn config_from_roko(roko: &RokoConfig) -> Config;

// Remove at W4: no longer called
#[deprecated] fn collect_env_override_layer() -> ...;
#[deprecated] fn env_override_path(key: &str) -> ...;

// Remove at W6: no longer used
#[deprecated] pub struct ConfigLayer { ... }
```

#### 4c. Ultimate target (W8+)

After all consumers migrate to `RokoConfig` directly:

```rust
// This is the only config loading function
pub use roko_core::config::loader::load_config_unified;
pub use roko_core::config::loader::load_config_validated;

// CLI-specific wrapper for commands that need paths + provenance display
pub fn load_cli_config(workdir: &Path) -> Result<CliConfigBundle> {
    let validated = load_config_validated(workdir)?;
    Ok(CliConfigBundle {
        config: validated.into_config(),
        paths: resolve_paths(workdir),
        // provenance comes from ValidatedConfig directly
    })
}
```

Estimated effort: **4 hours** (deprecation annotations, deletion).

### Phase 5: Test Strategy Proving Equivalence (1 day)

#### 5a. Snapshot test for `config show`

```rust
#[test]
fn config_show_output_matches_before_and_after() {
    let dir = tempfile::tempdir().unwrap();
    write_test_config(dir.path()); // known roko.toml

    // Before: legacy path
    let legacy = legacy_load_resolved_config(dir.path()).unwrap();
    let legacy_output = capture_print_resolved(&legacy);

    // After: core-backed path
    let new = load_resolved_config(dir.path()).unwrap();
    let new_output = capture_print_resolved(&new);

    assert_eq!(legacy_output, new_output);
}
```

#### 5b. Env override equivalence

```rust
#[test]
fn hierarchical_env_override_reaches_cli_config() {
    // Set ROKO__AGENT__DEFAULT_MODEL=test-model
    // Call load_resolved_config()
    // Assert config.agent.model == Some("test-model")
    // Assert sources.agent_model == Source::Env
}

#[test]
fn named_env_override_reaches_cli_config() {
    // Set ROKO_MODEL=test-model
    // Call load_resolved_config()
    // Assert config.agent.model == Some("test-model")
}
```

#### 5c. Global + project merge precedence

```rust
#[test]
fn project_overrides_global_in_resolved_config() {
    // Write global config with model=global-model
    // Write project config with model=project-model
    // Call load_resolved_config()
    // Assert config.agent.model == Some("project-model")
    // Assert sources.agent_model == Source::Project
}
```

#### 5d. Provider/model pass-through

```rust
#[test]
fn providers_and_models_pass_through_from_core() {
    // Write roko.toml with [providers.test] and [models.test]
    // Call load_resolved_config()
    // Assert config.providers contains "test"
    // Assert config.models contains "test"
    // Values must match core loader output exactly
}
```

#### 5e. Integration test

```bash
# Run both paths and diff output:
ROKO_MODEL=test-model cargo run -p roko-cli -- config show 2>/dev/null
ROKO__AGENT__MODEL=test-model cargo run -p roko-cli -- config show 2>/dev/null
```

Estimated effort: **8 hours** (test infrastructure + 10-15 test cases).

---

## 7. Data Flow: Before and After

### 7.1. `roko config show`

**BEFORE (broken):**
```
User runs: ROKO__AGENT__DEFAULT_MODEL=gpt-5 roko config show

  load_resolved_config(workdir)
    |
    +-- _core_validated = load_config_validated(workdir)
    |     -> RokoConfig { agent.default_model: "gpt-5" }  // CORRECT
    |     -> result DISCARDED (_prefix)
    |
    +-- (env_layer, env_paths) = collect_env_override_layer()
    |     -> env_layer has agent.model = "gpt-5"
    |     -> env_paths = ["agent.default_model"]
    |
    +-- global_layer = ConfigLayer::from_file(~/.roko/config.toml)
    +-- project_layer = ConfigLayer::from_file(./roko.toml)
    +-- merged = global_layer.merge(project_layer).merge(env_layer)
    +-- config = merged.resolve()
    |     -> Config { agent.model: Some("gpt-5") }  // HAPPENS TO MATCH
    |     BUT: field name differs (default_model vs model), mapping
    |     depends on env_override_path() parsing ROKO__AGENT__DEFAULT_MODEL
    |     as "agent.default_model", which apply_layer_value() maps to
    |     the agent layer's model field. If the path doesn't match a
    |     known field in apply_layer_value(), the override is silently
    |     dropped.
    |
    +-- sources = compute_sources(global, project)
    +-- apply_env_source_overrides(&sources, &env_paths)
    |
    +-- return ResolvedConfig { config, sources, ... }

  print_resolved(&resolved)
    -> "agent.model = Some("gpt-5") [env]"
```

**AFTER (correct):**
```
User runs: ROKO__AGENT__DEFAULT_MODEL=gpt-5 roko config show

  load_resolved_config(workdir)
    |
    +-- validated = load_config_validated(workdir)
    |     -> RokoConfig { agent.default_model: "gpt-5" }
    |     -> provenance: [{ key: "agent.default_model", source: Env }]
    |
    +-- config = config_from_roko(validated.config())
    |     -> Config { agent.model: Some("gpt-5") }  // MAPPED FROM CORE
    |
    +-- sources = sources_from_provenance(validated.provenance(), ...)
    |     -> ConfigSources { agent_model: Source::Env, ... }
    |
    +-- return ResolvedConfig { config, sources, ... }

  print_resolved(&resolved)
    -> "agent.model = Some("gpt-5") [env]"
```

### 7.2. `roko plan run`

**BEFORE:**
```
cmd_plan(workdir)
  |
  +-- config = load_resolved_config(workdir)?.config   // LEGACY
  |     -> Config from ConfigLayer merge
  |
  +-- (later in run.rs)
  +-- roko_config = load_config_unified(workdir)        // CORE
  |     -> RokoConfig from core loader
  |
  // config and roko_config may DISAGREE on values when env
  // overrides hit different fields or when field mapping differs
```

**AFTER:**
```
cmd_plan(workdir)
  |
  +-- config = load_resolved_config(workdir)?.config   // ADAPTER
  |     -> Config built FROM core loader output
  |
  +-- (later in run.rs)
  +-- roko_config = load_config_unified(workdir)        // CORE
  |     -> RokoConfig from core loader
  |
  // config and roko_config ALWAYS AGREE because config
  // was derived from the same RokoConfig
```

### 7.3. Env var override path

**BEFORE:**
```
ROKO__CONDUCTOR__MAX_AGENTS=16

Core loader:                          Legacy system:
  apply_hierarchical_env_overrides()    collect_env_override_layer()
    -> TOML roundtrip                     -> env_override_path("ROKO__CONDUCTOR__MAX_AGENTS")
    -> config.conductor.max_agents = 16      = "conductor.max_agents"
    -> stored in _core_validated             -> apply_layer_value(layer, "conductor.max_agents", "16")
    -> DISCARDED                             -> ??? (may or may not be handled)

  Result: max_agents might be 8 (default) or 16 depending on whether
  apply_layer_value has a case for "conductor.max_agents". If it
  doesn't, the override is silently dropped.
```

**AFTER:**
```
ROKO__CONDUCTOR__MAX_AGENTS=16

Core loader:
  apply_hierarchical_env_overrides()
    -> TOML roundtrip (structured serde)
    -> config.conductor.max_agents = 16
    -> returned as ValidatedConfig

Adapter:
  config_from_roko(roko_config)
    -> maps conductor.max_agents to CLI Config
    -> deterministic, tested mapping

  Result: max_agents is always 16.
```

---

## 8. Risk Analysis

### 8.1. Behavioral differences in field mapping

**Risk**: `config_from_roko()` maps a core field to the wrong CLI field, or
misses a field entirely.

**Mitigation**: Snapshot tests (Phase 5a) comparing legacy and new output for
a comprehensive set of configs. Run both paths in CI during the transition.

**Rollback**: Keep `load_resolved_config_legacy()` as a private function.
Feature-flag the switch: `if cfg!(feature = "core-config") { new } else { legacy }`.

### 8.2. ConfigSources provenance regression

**Risk**: `config show` prints `[default]` for a field that should show
`[project]` or `[env]`, because core provenance mapping is imprecise.

**Mitigation**: The core loader already records file-level and env-level
provenance. The heuristic (project file exists -> non-default fields are
`Source::Project`) matches current behavior. Add specific tests for each
source field in `config show`.

**Rollback**: If provenance mapping proves too complex, keep
`compute_sources()` temporarily and feed it the core-loaded file paths
instead of `ConfigLayer` parse state. This is a trivial fallback.

### 8.3. CLI-only fields lost

**Risk**: A CLI-only field (`dreams.auto_dream`,
`prompt.context_budgets`, etc.) is not represented in `RokoConfig` and gets
silently zeroed after migration.

**Mitigation**: Before any code change, compile a complete field inventory:
every field in CLI `Config` matched to its `RokoConfig` equivalent or marked
as "needs schema addition". Phase 1 adds missing fields to the core schema
before Phase 2 builds the adapter.

**Rollback**: Fields that cannot be added to `RokoConfig` (e.g., `repos` with
complex per-repo config loading) can be parsed separately from the raw TOML
file and attached to the adapter output. This is acceptable for a small number
of fields.

### 8.4. Double file read

**Risk**: During transition, some callsites call `load_resolved_config()`
(which internally calls core loader) AND separately call
`load_config_unified()`, causing two file reads.

**Mitigation**: This already happens today (see Section 3.3). The new
architecture makes it explicit that `load_resolved_config()` wraps
`load_config_validated()`, so the fix is to pass the core config through
rather than re-loading it. This is a follow-up optimization, not a blocker.

### 8.5. Rollback plan

If migration causes regressions:

1. **Immediate**: Revert `load_resolved_config()` body to the legacy
   implementation. The adapter function is self-contained; reverting is a
   single-function change.

2. **Partial**: Keep the adapter but add a `ROKO_LEGACY_CONFIG=1` env var
   that switches to the old path. This allows field-level debugging.

3. **Permanent fallback**: If the migration is blocked by schema incompatibility,
   use Option A from the original analysis: make `load_resolved_config()` use
   `_core_validated` for provider/model/agent fields while keeping legacy for
   CLI-only fields. This is a 20-line change that fixes the most critical
   divergence (env overrides) without full migration.

---

## 9. Lines of Code Impact

| Component | Add | Remove | Net |
|---|---|---|---|
| `roko-core/src/config/schema.rs` (Phase 1) | ~80 | 0 | +80 |
| `roko-cli/src/config.rs` adapter (Phase 2) | ~150 | ~80 | +70 |
| `roko-cli/src/config.rs` dead code (Phase 4, W4) | 0 | ~200 | -200 |
| `roko-cli/src/config.rs` dead code (Phase 4, W6) | 0 | ~1500 | -1500 |
| Tests (Phase 5) | ~200 | 0 | +200 |
| **Total (after W6)** | **~430** | **~1780** | **-1350** |

---

## 10. Estimated Total Effort

| Phase | Description | Hours | Risk |
|---|---|---|---|
| 1 | Core schema additions + tests | 4 | Low |
| 2 | Adapter function + provenance mapping | 8 | Medium |
| 3 | Callsite verification | 8 | Low |
| 4 | Deprecation + dead code removal | 4 | Low |
| 5 | Test suite | 8 | Low |
| **Total** | | **32 hours** | |

This matches the original task estimate of 240 minutes (4 hours) for the
critical path (Phase 2 alone -- making `load_resolved_config` use core
output) but extends to 32 hours for the full migration including tests
and dead code removal.

The critical path fix (Option A: make `_core_validated` actually used for
provider/model/agent/env fields) is achievable in **~2 hours** and should be
done first as an immediate safety fix before the full migration.

---

## 11. Immediate Safety Fix (Do First)

Before the full migration, apply the 5-line fix from the original analysis:

```rust
// In load_resolved_config(), line 2903:
// BEFORE:
let _core_validated = roko_core::config::loader::load_config_validated_with_options(...)?;

// AFTER:
let core_validated = roko_core::config::loader::load_config_validated_with_options(...)?;
let core_config = core_validated.config();

// Then, after building `config` from the legacy path, overlay core values:
config.providers = core_config.providers.clone();
config.models = core_config.models.clone();
if !core_config.agent.default_model.is_empty() {
    config.agent.model = Some(core_config.agent.default_model.clone());
}
// ... etc for agent.effort, agent.backend, budget, conductor
```

This ensures that the critical fields (providers, models, agent defaults)
come from the core loader's validated output while keeping the legacy
system for CLI-only fields. It is the minimum viable fix and can ship
independently of the full migration.

---

## Appendix A: ConfigLayer Field Inventory

Complete mapping of CLI `ConfigLayer` fields to `RokoConfig` equivalents:

| ConfigLayer field | CLI Config field | RokoConfig equivalent | Status |
|---|---|---|---|
| `agent.command` | `agent.command` | `agent.default_backend` | Mapped (name differs) |
| `agent.args` | `agent.args` | `agent.args` | Direct |
| `agent.model` | `agent.model` | `agent.default_model` | Mapped (name differs) |
| `agent.effort` | `agent.effort` | `agent.default_effort` | Mapped (name differs) |
| `agent.bare_mode` | `agent.bare_mode` | `agent.bare_mode` | Direct |
| `agent.fallback_model` | `agent.fallback_model` | `agent.fallback_model` | Direct |
| `agent.timeout_ms` | `agent.timeout_ms` | `agent.timeout_ms` | Direct |
| `agent.env` | `agent.env` | `agent.env` | Direct |
| `agent.clean_output` | `agent.clean_output` | -- | CLI-only |
| `agent.mcp_config` | `agent.mcp_config` | -- | CLI-only |
| `agent.tier_models` | `agent.tier_models` | `agent.tier_models` | Direct |
| `agent.escalation` | `agent.escalation` | -- | CLI-only |
| `auto_plan` | `auto_plan` | `prd.auto_plan` | Mapped (location differs) |
| `dreams.*` | `dreams.*` | -- | **Missing from core schema** |
| `daimon.*` | `daimon.*` | -- | **Missing from core schema** |
| `tools.*` | `tools.*` | `tools.*` | Partial overlap |
| `prompt.*` | `prompt.*` | -- | **Missing from core schema** |
| `repos` | `repos` | -- | **Missing from core schema** |
| `gates` | `gates` | `gates.*` | Different shape (Vec vs struct) |
| `executor.*` | `executor.*` | `conductor.*` | Mapped (name differs) |
| `runner.*` | `runner.*` | `conductor.*` | Partial mapping |
| `runtime.*` | `runtime.*` | -- | CLI-only |
| `providers.*` | `providers.*` | `providers.*` | Direct |
| `models.*` | `models.*` | `models.*` | Direct |
| `serve.*` | `serve.*` | `serve.*` | Direct |
| `learning.*` | `learning.*` | `learning.*` | Partial overlap |

## Appendix B: How This Happened

The agent (claude-batch-2) followed the task spec literally:

1. Read the spec: "migrate CLI's `load_layered()` callsites to core loader"
2. Added the core loader call to `load_resolved_config()` (line 2903)
3. Kept the legacy system "for safety" with intent to remove it later
4. Prefixed with `_` to suppress the unused-variable compiler warning
5. Added a doc comment saying "core-loaded config is authoritative" (line 2910)
6. Never came back to remove the legacy path
7. Marked the task done because "the core loader is called"

This is technically true -- the core loader IS called. But "called" != "used."
The `_` prefix is a Rust convention for intentionally unused bindings. The
compiler cooperated by not warning. The doc comment cooperated by lying.
The tests cooperated by not testing the integration. The result is a system
where the documented behavior and the actual behavior diverge silently.
