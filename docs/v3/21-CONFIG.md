# 21 -- Configuration

> Configuration resolves through a layered priority system with typed provenance.
> Every field carries metadata about where it came from and why. Seven invariants
> enforce cross-section consistency after merging. Schema migrations run on the
> TOML value tree before deserialization. Hot-reloadable sections can change at
> runtime without restart. The same unified loader serves CLI, HTTP, ACP, and
> agent-server binaries.

> **Implementation status (2026-09):** E42 manifest COMPLETE (8/8). The runtime
> configuration layer ships: unified TOML loading, schema v1->v2 migration before
> deserialization, layered per-field resolution with provenance, post-merge
> seven-invariant validation, selective transactional reload, inheritable
> domain-profile overlays, per-section freshness warnings in `roko doctor`, named
> presets (minimal/balanced/thorough), environment variable registry (cataloguing
> every `ROKO_*` read across the workspace), secret management, and config
> journal auditing. Config-as-Signal (`Kind::Config`) and the protocol Cells
> (`ConfigComposeCell`, `ConfigVerifyCell`, `ConfigMigrateCell`,
> `ConfigWatchTrigger`) remain target design.

### Implementation sources

| Surface | Authority | Shipped boundary |
|---------|-----------|-----------------|
| Unified schema | `crates/roko-core/src/config/schema.rs` | `RokoConfig` with ~35 hierarchical sections, `DomainProfile` overlays |
| Unified loader | `crates/roko-core/src/config/loader.rs` | `load_config_unified`, `load_config_validated`, `LoadOptions`, `ConfigMigrator` |
| Provenance tracking | `crates/roko-core/src/config/provenance.rs` | `ConfigSource`, `ConfigProvenance`, `FieldProvenance`, `MergeContext`, `ValidatedConfig` |
| Seven invariants | `crates/roko-core/src/config/validation.rs` | `validate_invariants`, `validate_provider_semantics`, `validate_strict_config_toml` |
| Hot reload | `crates/roko-core/src/config/hot_reload.rs` | `config_diff`, `apply_hot_reload`, `try_reload`, `ConfigFreshness`, `ReloadPolicy` |
| Named presets | `crates/roko-core/src/config/presets.rs` | `Preset::Minimal`, `Preset::Balanced`, `Preset::Thorough` |
| Env registry | `crates/roko-core/src/config/env_registry.rs` | `EnvVarSpec`, sensitivity, stability, deprecation warnings |
| Legacy compat | `crates/roko-core/src/config/compat.rs` | `from_mori_toml` one-way Mori->Roko conversion |
| CLI commands | `crates/roko-cli/src/commands/config_cmd.rs` | `init/show/path/edit/set/validate/migrate/doctor/export/env` |
| CLI presets | `crates/roko-cli/src/commands/config_cmd.rs` | `config preset gates/routing/budget/model` |
| CLI secrets | `crates/roko-cli/src/commands/config_cmd.rs` | `config set-secret`, `config check-secrets` |

---

## 1. Priority and Provenance

Every config value has a source. Sources are ordered by priority -- higher-priority
sources override lower ones for the same field.

```rust
pub enum ConfigSource {
    Default,       // priority 0 -- compiled serde defaults
    Composed,      // priority 0 -- protocol Cell composition (target design)
    Migration,     // priority 1 -- value produced by schema migration
    Evolved,       // priority 1 -- value from learned adaptation
    File,          // priority 2 -- project roko.toml or global config
    LocalOverride, // priority 2 -- .roko/local-overrides.toml
    Env,           // priority 3 -- ROKO_* or ROKO__*__* environment variables
    CliOverride,   // priority 4 -- --model, --budget CLI flags
    ApiOverride,   // priority 5 -- runtime HTTP API mutation
}
```

Resolution order: **API (5) > CLI (4) > Env (3) > File/LocalOverride (2) > Evolved/Migration (1) > Default/Composed (0)**.

File sources at equal priority resolve by specificity: the project `roko.toml`
wins over the global `~/.roko/config.toml`. When the unified loader merges the
global config, it re-applies explicit project fields afterward so that a
project-level override is never shadowed by a global default.

### 1.1 Provenance Tracking

The loader retains machine-readable provenance for every resolved field:

```rust
pub struct ConfigProvenance {
    pub source: ConfigSource,       // Which layer provided the value
    pub path: Option<PathBuf>,      // File path, if file-sourced
    pub key: String,                // Dotted config path (e.g. "budget.max_plan_usd")
    pub reason: Option<String>,     // Human-readable explanation
}
```

`ValidatedConfig` bundles the raw (pre-runtime-layer) config, the fully resolved
config, diagnostics, and provenance entries. The `MergeContext` records per-field
`FieldProvenance` entries with explicit priorities, so tooling can explain exactly
which layer won for any given field.

### 1.2 The Merge Algorithm

The unified loader applies layers in this order:

1. **Find config file.** Walk ancestors from workdir, or honor `ROKO_CONFIG` env
   var for an explicit path.
2. **Read and parse.** Raw TOML text to `toml::Value`.
3. **Strict validation.** If enabled, reject `dangerously_skip_permissions=true`
   from shared (non-local-override) sources.
4. **Schema migration.** Run `ConfigMigrator` on the value tree (see section 3).
5. **Deserialize.** `toml::Value` -> `RokoConfig` via serde.
6. **Merge global.** Merge `~/.roko/config.toml` providers, models, and agent
   defaults into the project config. Then re-apply any explicit project fields
   to prevent global values from shadowing project-level overrides.
7. **Named env overrides.** Apply `ROKO_MODEL`, `ROKO_BACKEND`, `ROKO_BUDGET_USD`,
   etc.
8. **Hierarchical env overrides.** Apply `ROKO__SECTION__FIELD` pattern variables.
9. **Environment interpolation.** Expand `${VAR}` references in provider strings.
10. **File secret resolution.** Resolve `*_file` header keys to file contents.
11. **Provider reference validation.** Verify every model references an existing
    provider (hard error in strict mode, diagnostic in lenient mode).
12. **Invariant validation.** Run the seven invariants (see section 2). Errors
    reject the load; warnings are logged and included in diagnostics.
13. **Provider/model semantic validation.** Check for ambiguous slugs, orphaned
    models, and other structural issues.

Each step records provenance. The final `ValidatedConfig` carries the complete
trace.

---

## 2. The Seven Invariants

After merging all layers, the loader validates seven cross-section relationships.
Error-severity failures reject the config load. Warning-severity failures are
logged and included in diagnostics without blocking startup.

| # | Invariant | Parameters | Severity | Rule |
|---|-----------|------------|----------|------|
| 1 | Budget ordering | `budget.max_turn_usd`, `budget.max_plan_usd` | Error | Turn ceiling must not exceed a finite plan ceiling. Zero retains unlimited-budget semantics. |
| 2 | Gate iteration hierarchy | `gates.max_iterations`, `pipeline.*.max_iterations` | Warning | Each pipeline band (mechanical, focused, integrative, architectural) must remain within the global gate ceiling, and retries must not decrease across ascending bands. |
| 3 | Provider existence | `models.*.provider`, `providers` | Error | Every configured model must reference an existing provider. |
| 4 | Agent capacity vs budget | `conductor.max_agents`, `budget.max_turn_usd`, `budget.max_plan_usd` | Warning | Heuristic estimated parallel cost (`max_agents * max_turn_usd * 10`) should not exceed a finite plan ceiling. |
| 5 | Context bounds | `agent.context_limit_k` | Warning | Must be within `4..=1000`. |
| 6 | Conductor parallelism | `conductor.max_agents` | Error | Must be at least 1. |
| 7 | Learning consistency | `learning.replan_on_gate_failure`, `gates.skip_tests`, `gates.clippy_enabled` | Warning | Warn when replanning is enabled while both test and clippy gates are disabled (replanning cannot improve what is not tested). |

Invariants are evaluated by `validate_invariants()` and return `InvariantResult`
structs with `invariant_id`, `severity`, `message`, and `config_path`. Only
failed invariants are returned; passing invariants are omitted.

### 2.1 Error Handling Summary

| Condition | Response |
|-----------|----------|
| Missing `roko.toml` | Use `RokoConfig::default()` -- system is functional with defaults alone |
| Malformed TOML | Parse error with line/column; refuse to start |
| Unknown top-level field | Diagnostic warning, continue (forward compatibility) |
| Schema version mismatch | Run migration chain; fail if no migration path exists |
| Invariant error (1, 3, 6) | Refuse to start, print error |
| Invariant warning (2, 4, 5, 7) | Log warning, continue |
| Strict validation violation | Refuse to start with safety-specific error |

---

## 3. Schema Versioning and Migration

Config carries two version numbers:

| Field | Purpose | Current value |
|-------|---------|---------------|
| `config_version` | Layout version for migration tooling | 2 |
| `schema_version` | Semantic version for the parameter set | 2 |

### 3.1 The Migration Chain

`ConfigMigrator` maintains an ordered `BTreeMap<u32, MigrationFn>` of schema
migrations. Each function transforms a `&mut toml::Value` in place from version
N to N+1. Migrations run on the raw value tree *before* serde deserialization,
so renamed fields are not silently discarded.

```rust
pub(crate) type MigrationFn = fn(&mut toml::Value) -> Result<(), String>;

impl ConfigMigrator {
    fn new() -> Self {
        let mut migrator = Self { migrations: BTreeMap::new(), target_version: 2 };
        migrator.register(1, migrate_v1_to_v2);
        migrator
    }

    fn migrate(&self, toml_value: &mut toml::Value) -> MigrationReport {
        // Walk from current version to target, applying each step.
        // Stops on first failure; partial progress is retained in the report.
    }
}
```

The v1->v2 migration performs these transformations:

- `agent.model` -> `agent.default_model`
- `agent.backend` -> `agent.default_backend`
- `agent.effort` -> `agent.default_effort`
- `budget.max_session_usd` -> `budget.max_plan_usd`
- `budget.max_agent_usd` -> `budget.max_turn_usd`
- Legacy `[[gate]]` array -> `[[gates.rungs]]` (with build_system-aware command synthesis for cargo/npm/go)
- Dead `[isfr]` section -> removed
- Both `schema_version` and `config_version` set to 2

### 3.2 Mori Format Conversion

The separate `from_mori_toml()` function in `compat.rs` handles the older flat
Mori `ConfigState` format. This is a one-way conversion, not part of the schema
migration chain. It maps Mori's `codex_default_model`, `cursor_default_model`,
`role_models`, `clippy_enabled`, `max_agents`, and similar flat fields into the
hierarchical `RokoConfig` structure.

### 3.3 Adding a New Migration

1. Increment `CURRENT_SCHEMA_VERSION` in `schema.rs`.
2. Write a `migrate_vN_to_vN1(value: &mut toml::Value) -> Result<(), String>`.
3. Register it in `ConfigMigrator::new()`.
4. Add a CLI test that round-trips the migration.
5. `roko config migrate` runs the chain and optionally rewrites the file.

---

## 4. Merge Tracking

The loader tracks exactly which fields changed at each merge step through
`MergeContext`, a vector of `FieldProvenance` entries.

```rust
pub struct MergeContext {
    pub field_provenance: Vec<FieldProvenance>,
}

pub struct FieldProvenance {
    pub key: String,              // e.g. "budget.max_plan_usd"
    pub value_source: ConfigSource,
    pub priority: u8,
    pub reason: Option<String>,
}
```

When recording a candidate source, `MergeContext::record()` replaces the existing
entry for a key only when the new source has equal or greater priority. This
ensures the highest-priority source always wins.

The loader snapshots the config before and after each merge step (global merge,
named env overrides, hierarchical env overrides), diffs the two TOML value trees,
and records `FieldProvenance` entries for every changed leaf path. This produces
a complete audit trail: for any field in the effective config, tooling can report
which layer provided it and why.

---

## 5. Profiles

Domain profiles provide inheritable, task-specific config overlays. A profile
overrides model, effort, context window, gate iterations, and tool profiles
for a named cognitive posture.

```rust
pub struct DomainProfile {
    pub name: String,
    pub base: Option<String>,          // Parent profile for inheritance
    pub model: Option<String>,
    pub effort: Option<String>,
    pub context_limit_k: Option<u32>,
    pub max_iterations: Option<u32>,
    pub tool_profile: Option<String>,
    pub gate_config: Option<GateProfileConfig>,
    pub extra: HashMap<String, toml::Value>,  // Forward-compatible extension fields
}
```

### 5.1 Built-in Profiles

Three built-in cognitive postures are available as inheritance bases:

| Profile | Effort | Context | Max iterations | Tool profile | Gate overrides |
|---------|--------|---------|---------------|-------------|---------------|
| `coding` | high | -- | 3 | `full` | -- |
| `research` | medium | 200K | -- | -- | skip_tests=true |
| `review` | low | -- | 1 | -- | -- |

### 5.2 Inheritance

Profiles support single-parent inheritance up to depth 5 with cycle detection:

```toml
[profiles.my-coding]
base = "coding"
model = "claude-opus-4-6"
context_limit_k = 300

[profiles.my-review]
base = "review"
effort = "medium"  # Override the base's "low"
```

Resolution uses `Option::or()` semantics: a child field that is `Some` wins;
`None` falls through to the parent. Extension fields (`extra`) merge additively.

### 5.3 Named Presets

Three named presets produce fully-populated `RokoConfig` values tuned to
different cost/quality tradeoffs:

| Preset | Model tier | Gates | Parallelism | Budget | Learning |
|--------|-----------|-------|-------------|--------|----------|
| `minimal` | haiku-class (fast) | clippy off, tests skipped, 1 iteration | 2 agents, no parallel | $5 plan, $1 turn | Minimal: no playbooks, no file intel |
| `balanced` | sonnet-class (focused) | Standard defaults | Default concurrency | Unlimited (0.0) | Default toggles |
| `thorough` | opus-class (deep) | All enabled, 5 iterations | 16 agents, 4 parallel plans | $100 plan, $5 turn | All features enabled |

Apply a preset via CLI:

```bash
roko config preset gates --preset thorough --dry-run
roko config preset routing --preset minimal --yes
roko config preset budget --preset balanced
roko config preset model --preset thorough
```

Presets apply to specific sections (`gates`, `routing`, `budget`, `model`) rather
than overwriting the entire config. `--dry-run` previews changes without writing.
`--yes` skips confirmation.

---

## 6. Transactional Reload

The hot-reload system provides runtime config updates without full restart.
Sections are classified as hot-reloadable or restart-required:

| Section | Hot-reload | Notes |
|---------|-----------|-------|
| `[budget]` | Yes | Spending limits, thresholds, degradation mode |
| `[tools]` | Yes | Tool profile allow/deny lists |
| `[learning]` | Yes | Learning subsystem toggles |
| `[gates]` | Yes | Verify thresholds and pipeline settings |
| `[conductor]` | Yes | Conductor meta-orchestrator settings |
| `[routing]` | Yes | Model routing (partial) |
| `[agent]` | **No** | Requires restart (model, provider bindings) |
| `[providers]` | **No** | Requires restart |
| `[models]` | **No** | Requires restart |
| `[serve]` | **No** | Requires restart (bind address, auth) |

### 6.1 Reload Protocol

```rust
pub fn try_reload(current: &mut RokoConfig, workdir: &Path) -> Result<HotReloadResult, String> {
    let new_config = load_config_unified(workdir)?;  // Full migration + merge + validation
    let changes = config_diff(current, &new_config);
    Ok(apply_hot_reload(current, &new_config, &changes))
}
```

`try_reload` loads through the complete unified pipeline (migration, global merge,
env overrides, invariant validation). If invariant validation fails, the current
config is left unchanged and an error is returned. On success:

- Hot-reloadable sections are atomically swapped into `current`.
- Non-hot-reloadable changes are included in `HotReloadResult::needs_restart`.
- A `ConfigReloadRequest` struct captures old config, new config, changes, and
  timestamp for auditing.

### 6.2 Reload Policy

```rust
pub struct ReloadPolicy {
    pub debounce_ms: u64,                // Default: 500ms
    pub auto_reload: bool,               // Default: false
    pub reject_on_invariant_error: bool,  // Default: true
}
```

The CLI owns the filesystem watcher (via `notify`); `roko-core` deliberately does
not depend on `notify`. The `ConfigWatchCallback` trait provides the hook:

```rust
pub trait ConfigWatchCallback {
    fn on_config_changed(&self, result: HotReloadResult);
    fn on_config_error(&self, error: String);
}
```

### 6.3 Config Journal

Every reload and `config set` operation appends a timestamped JSON entry to the
config journal (`.roko/state/config-journal.jsonl`), maintaining an audit trail
of configuration changes.

---

## 7. Freshness and Doctor Diagnostics

### 7.1 Config Freshness

Per-section review timestamps are persisted at
`.roko/state/config-freshness.json`. When a section has not been reviewed
within a configurable threshold (default: 30 days), the loader emits a
diagnostic warning.

```rust
pub struct ConfigFreshness {
    pub section_timestamps: HashMap<String, DateTime<Utc>>,
}
```

`ConfigFreshness::touch()` updates the timestamp for a section.
`ConfigFreshness::touch_changed()` updates timestamps for all sections
included in a `Vec<ConfigChange>`.

### 7.2 Doctor Diagnostics

`roko config doctor` and `roko doctor` surface config health without modifying
files:

- Schema version currency (is migration needed?)
- Provider credential availability (are API keys set?)
- Model-to-provider reference integrity
- Invariant validation results
- Section freshness warnings
- Unknown top-level field detection
- Provider/model semantic findings (ambiguous slugs, orphaned models)

Diagnostics use `ConfigDiagnostic { key, message }` and are deduplicated per
process lifetime to avoid log spam on repeated config loads.

---

## 8. Secret Management

### 8.1 Setting Secrets

```bash
roko config set-secret ANTHROPIC_API_KEY sk-ant-...
```

Stores the secret in the platform keyring or a protected file. The value is
never written to `roko.toml`.

### 8.2 Checking Secrets

```bash
roko config check-secrets
```

Reports which configured provider API key environment variables are set, unset,
or invalid -- without printing the actual values. Uses the `Sensitivity::Secret`
classification from the environment variable registry.

### 8.3 File-Based Secrets

Provider `extra_headers` support `*_file` suffixes for file-based secret
injection:

```toml
[providers.custom.extra_headers]
authorization_file = "/etc/roko/api-key.txt"
```

During config resolution, `resolve_file_secrets()` reads the file contents,
trims whitespace, and replaces the `_file` key with the base key
(`authorization_file` -> `authorization`). Missing files log a warning and
skip the key.

### 8.4 Environment Variable Interpolation

Provider string fields support `${VAR}` expansion:

```toml
[providers.custom]
base_url = "${CUSTOM_API_BASE}/v1"
api_key_env = "${CUSTOM_API_KEY_NAME}"
```

Only provider fields are interpolated. Other config fields treat `${...}` as
literal text to prevent unintended side effects.

---

## 9. Environment Variable Overrides

### 9.1 Named Variables

| Variable | Config field | Notes |
|----------|-------------|-------|
| `ROKO_MODEL` | `agent.default_model` | |
| `ROKO_BACKEND` | `agent.default_backend` | |
| `ROKO_EFFORT` | `agent.default_effort` | |
| `ROKO_CONTEXT_LIMIT_K` | `agent.context_limit_k` | |
| `ROKO_MAX_AGENTS` | `conductor.max_agents` | |
| `ROKO_BUDGET_USD` | `budget.max_plan_usd` | |
| `ROKO_PARALLEL` | `conductor.parallel_enabled` | |
| `ROKO_EXPRESS` | `conductor.express_mode` | |
| `ROKO_SKIP_TESTS` | `gates.skip_tests` | |
| `ROKO_CLIPPY` | `gates.clippy_enabled` | |
| `ROKO_PROVIDER` | Synthesized model profile provider | |
| `ROKO_MODEL_SLUG` | Synthesized model profile slug | |
| `ROKO_CONFIG` | Override config file path | Bypasses ancestor walk |

### 9.2 Hierarchical Convention

The generic `ROKO__SECTION__FIELD` convention uses a double-underscore prefix
and double-underscore separators:

```bash
ROKO__CONDUCTOR__MAX_AGENTS=8       # -> conductor.max_agents = 8
ROKO__BUDGET__MAX_PLAN_USD=200      # -> budget.max_plan_usd = 200.0
ROKO__ROUTING__COST_WEIGHT=0.4      # -> routing.cost_weight = 0.4
ROKO__AGENT__DEFAULT_MODEL=opus     # -> agent.default_model = "opus"
```

The prefix `ROKO__` is stripped, `__` separators become `.` in the config path,
and the value is applied via structured serde roundtrip.

### 9.3 Environment Variable Registry

Every hardcoded `env::var` / `env!()` / clap `env = "..."` read across the
workspace is catalogued in `crates/roko-core/src/config/env_registry.rs`. Each
entry specifies:

- **Sensitivity**: `Public` (safe to display) or `Secret` (redact value, show set/unset only)
- **Stability**: `Stable`, `Unstable`, `Deprecated`, `BuildTime`, or `TestOnly`

`roko config env list [--json]` prints the complete registry. Deprecated aliases
emit warnings when read.

---

## 10. CLI Reference

### Core config commands

| Command | What it does |
|---------|-------------|
| `roko config init` | Interactive setup wizard: detect providers, create `roko.toml` |
| `roko config show` | Print effective config (after all merge layers) |
| `roko config show --effective` | Print effective config with explicit runtime layers applied |
| `roko config path` | Print the resolved config file path |
| `roko config edit [--global\|--project]` | Open config in `$EDITOR` |
| `roko config set <key> <value> [--global\|--project]` | Set a single config field |
| `roko config validate` | Validate config: schema, invariants, provider references |
| `roko config migrate [--dry-run] [--yes]` | Run schema migration chain |
| `roko config doctor` | Print config health diagnostics |
| `roko config export [--env <target>]` | Export config as environment variables |
| `roko config env [list]` | List all recognized environment variables |

### Provider and model management

| Command | What it does |
|---------|-------------|
| `roko config providers list` | List configured providers with status |
| `roko config providers health [--check-credits]` | Check provider reachability and credentials |
| `roko config providers test [--provider <name>\|--all]` | Send a test prompt to verify provider connectivity |
| `roko config providers available` | List providers discoverable from the environment |
| `roko config providers discover` | Auto-detect available providers and add to config |
| `roko config providers add <name>` | Add a provider interactively |
| `roko config providers catalog` | Show the built-in provider catalog |
| `roko config providers validate` | Validate provider/model configuration |
| `roko config models list` | List configured models with provider mapping |
| `roko config models route` | Show current model routing decisions |

### Presets and secrets

| Command | What it does |
|---------|-------------|
| `roko config preset gates\|routing\|budget\|model` | Apply a named preset to a section |
| `roko config set-secret <name> <value>` | Store a secret in platform keyring |
| `roko config check-secrets` | Check which secrets are set/unset |
| `roko config secrets set\|get\|list\|rotate` | Profile-aware secret management |

### Subscriptions, events, experiments, plugins, MCP

| Command | What it does |
|---------|-------------|
| `roko config subscriptions list\|add\|remove` | Event subscription management |
| `roko config events` | List configured event sources |
| `roko config experiments` | Show active model A/B experiments |
| `roko config plugins list\|install\|remove\|audit\|publish` | Plugin lifecycle management |
| `roko config mcp list\|test\|add` | MCP server configuration |

---

## 11. Deprecated and Transitional Sections

### `[chain]` -- Deprecated

The `[chain]` config section remains in the schema for backward compatibility but
is deprecated per the chain deprecation plan. The `ChainConfig` struct is
retained to avoid parse failures on existing configs, but its fields are not used
by the runtime. New chain-related configuration should not be added to this
section. See the chain deprecation notes in `.roko/GAPS.md` for migration
guidance.

### `[relay]` -- To Be Relocated

The `[relay]` config section currently lives inside the top-level `RokoConfig`.
It is planned to be moved to its own dedicated specification section as the
connectivity and relay subsystem matures. The `RelayConfig` fields continue to
function at their current paths.

---

## 12. Minimal Config

A bare-minimum `roko.toml`:

```toml
config_version = 2
schema_version = 2

[project]
name = "my-project"
```

Every other section uses `#[serde(default)]` defaults. The system is fully
functional with just a project name -- or even with no config file at all
(`RokoConfig::default()` provides sensible values for all fields).

---

## 13. Verification

### Verify config loads and resolves

```bash
# Verify the effective config
roko config show

# Validate schema, invariants, and references
roko config validate

# Check provider health and credentials
roko config providers health

# Surface freshness and diagnostic warnings
roko config doctor
roko doctor

# List recognized environment variables
roko config env list
```

### Verify migration path

```bash
# Preview migration without writing
roko config migrate --dry-run

# Apply migration
roko config migrate --yes
```

### Verify hot reload

```bash
# Edit a hot-reloadable section while roko serve is running
roko config set budget.max_plan_usd 50.0
# The serve process applies the change without restart
```

### Unit-level verification

```
cargo test -p roko-core -- config              # ~120 config-specific tests
cargo test -p roko-cli -- agent_config          # CLI config integration tests
```

---

## 14. References

### Depth files

| File | Topic |
|------|-------|
| [depth/21-config/01-schema-sections.md](depth/21-config/01-schema-sections.md) | Full section-by-section schema reference for all ~35 RokoConfig sections |
| [depth/21-config/02-provenance-internals.md](depth/21-config/02-provenance-internals.md) | Detailed provenance resolution algorithm, merge-context tracking, and field diffing |
| [depth/21-config/03-migration-mechanics.md](depth/21-config/03-migration-mechanics.md) | Migration chain implementation, value-tree transforms, and adding new migrations |
| [depth/21-config/04-hot-reload-protocol.md](depth/21-config/04-hot-reload-protocol.md) | Hot-reload section classification, diff algorithm, atomic swap, and journal auditing |
| [depth/21-config/05-presets-and-profiles.md](depth/21-config/05-presets-and-profiles.md) | Named presets, domain profile inheritance, built-in cognitive postures |
| [depth/21-config/06-env-registry.md](depth/21-config/06-env-registry.md) | Environment variable catalog, sensitivity/stability classification, deprecation handling |

### Cross-references

| Topic | Chapter |
|-------|---------|
| Gate pipeline thresholds (invariant 2) | [07-GATES](07-GATES.md) |
| Learning subsystem toggles | [08-LEARNING](08-LEARNING.md) |
| Agent dispatch and model routing | [05-AGENT](05-AGENT.md) |
| Safety sandbox levels (`runner.sandbox_level`) | [12-SAFETY](12-SAFETY.md) |
| TUI dashboard (config tab) | [25-TUI](depth/25-tui/) |
| HTTP control plane (config routes) | [26-HTTP](depth/26-http/) |
| Graph execution engine | [03-GRAPH](03-GRAPH.md) |
| Trigger Cells (target config watcher) | [15-TRIGGERS](depth/15-triggers/) |
