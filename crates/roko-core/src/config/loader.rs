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
//! ## Hierarchical `ROKO__SECTION__FIELD` overrides
//!
//! In addition to the named variables above, hierarchical overrides using
//! `ROKO__SECTION__FIELD` syntax are supported. The prefix `ROKO__` is stripped,
//! and `__` separators are converted to `.` in the config path. The value is then
//! applied to the serialized TOML representation via structured serde roundtrip.
//!
//! Examples:
//! - `ROKO__AGENT__DEFAULT_MODEL=gpt-4` -> `agent.default_model = "gpt-4"`
//! - `ROKO__CONDUCTOR__MAX_AGENTS=16` -> `conductor.max_agents = 16`
//! - `ROKO__GATES__SKIP_TESTS=true` -> `gates.skip_tests = true`

use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use parking_lot::Mutex;

use super::LoadConfigError;
use super::provenance::{
    ConfigDiagnostic, ConfigProvenance, ConfigSource, FieldProvenance, MergeContext,
    ValidatedConfig,
};
use super::schema::RokoConfig;
use super::validation::{InvariantSeverity, validate_invariants, validate_provider_semantics};

/// One in-place schema migration from version N to N+1.
pub(crate) type MigrationFn = fn(&mut toml::Value) -> Result<(), String>;

/// A successfully applied migration edge.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct MigrationStep {
    pub(crate) from: u32,
    pub(crate) to: u32,
    pub(crate) description: String,
}

/// Outcome of attempting to migrate one TOML value.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct MigrationReport {
    pub(crate) from_version: u32,
    pub(crate) to_version: u32,
    pub(crate) steps_applied: Vec<MigrationStep>,
    pub(crate) warnings: Vec<String>,
}

/// Ordered registry of schema migrations.
pub(crate) struct ConfigMigrator {
    pub(crate) migrations: BTreeMap<u32, MigrationFn>,
    pub(crate) target_version: u32,
}

impl ConfigMigrator {
    #[must_use]
    pub(crate) fn new() -> Self {
        let mut migrator = Self {
            migrations: BTreeMap::new(),
            target_version: super::schema::CURRENT_SCHEMA_VERSION,
        };
        migrator.register(1, migrate_v1_to_v2);
        migrator
    }

    pub(crate) fn register(&mut self, from_version: u32, migration: MigrationFn) {
        self.migrations.insert(from_version, migration);
    }

    /// Apply every available N -> N+1 migration up to the target schema.
    ///
    /// The report retains partial progress and warnings when a migration edge
    /// is unavailable or fails. Live loaders reject reports that do not reach
    /// `target_version`; tooling may inspect the partial report directly.
    pub(crate) fn migrate(&self, toml_value: &mut toml::Value) -> MigrationReport {
        let from_version = toml_value
            .get("schema_version")
            .and_then(toml::Value::as_integer)
            .and_then(|version| u32::try_from(version).ok())
            .unwrap_or(1);
        let mut current = from_version;
        let mut steps_applied = Vec::new();
        let mut warnings = Vec::new();

        while current < self.target_version {
            let Some(migration) = self.migrations.get(&current) else {
                warnings.push(format!(
                    "no config migration registered from schema version {current}"
                ));
                break;
            };
            if let Err(error) = migration(toml_value) {
                warnings.push(format!(
                    "config migration from schema version {current} failed: {error}"
                ));
                break;
            }
            let next = current + 1;
            if let Some(table) = toml_value.as_table_mut() {
                table.insert(
                    "schema_version".to_string(),
                    toml::Value::Integer(i64::from(next)),
                );
            }
            steps_applied.push(MigrationStep {
                from: current,
                to: next,
                description: format!("migrate config schema v{current} to v{next}"),
            });
            current = next;
        }

        MigrationReport {
            from_version,
            to_version: current,
            steps_applied,
            warnings,
        }
    }
}

impl Default for ConfigMigrator {
    fn default() -> Self {
        Self::new()
    }
}

/// Keys that config schema v2 renamed, as `(table, v1 name, v2 name)`.
///
/// The v1 -> v2 migration renames them when it loads an older file, and
/// config editors use the table to write the v2 name for a v1 key.
pub const V1_RENAMED_KEYS: &[(&str, &str, &str)] = &[
    ("agent", "model", "default_model"),
    ("agent", "backend", "default_backend"),
    ("agent", "effort", "default_effort"),
    ("budget", "max_session_usd", "max_plan_usd"),
    ("budget", "max_agent_usd", "max_turn_usd"),
];

fn migrate_v1_to_v2(value: &mut toml::Value) -> Result<(), String> {
    let root = value
        .as_table_mut()
        .ok_or_else(|| "config root must be a TOML table".to_string())?;

    for (table, old, new) in V1_RENAMED_KEYS {
        if let Some(section) = root.get_mut(*table).and_then(toml::Value::as_table_mut) {
            rename_if_absent(section, old, new);
        }
    }

    // Migrate top-level [[gate]] entries to [[gates.rungs]].
    //
    // Legacy format: top-level array of tables with tagged `kind` field
    //   (shell/compile/clippy/test), plus `program`, `args`, `timeout_ms`,
    //   and `build_system`.
    // Target format: `[gates]` section with `rungs` array of
    //   `{ name, command, timeout_secs, required, parallel_with }`.
    //
    // The [isfr] section is unconditionally removed -- it was a dead
    // rate-oracle vestige.
    if let Some(gate_array) = root.remove("gate")
        && let Some(entries) = gate_array.as_array()
    {
        let mut rungs = Vec::new();
        for entry in entries {
            if let Some(rung) = convert_legacy_gate_to_rung(entry) {
                rungs.push(rung);
            }
        }
        if !rungs.is_empty() {
            let gates = root
                .entry("gates")
                .or_insert_with(|| toml::Value::Table(toml::map::Map::new()));
            if let Some(gates_table) = gates.as_table_mut() {
                // Only insert if there are no existing rungs/custom_rungs.
                if !gates_table.contains_key("rungs") && !gates_table.contains_key("custom_rungs") {
                    gates_table.insert("rungs".to_string(), toml::Value::Array(rungs));
                }
            }
        }
    }

    // Remove dead [isfr] section during migration.
    root.remove("isfr");

    root.insert("schema_version".to_string(), toml::Value::Integer(2));
    root.insert("config_version".to_string(), toml::Value::Integer(2));
    Ok(())
}

/// Convert one legacy `[[gate]]` entry (tagged enum) to a `[[gates.rungs]]` table.
///
/// Returns `None` when the entry has no lossless representation (the spec says
/// to reject rather than guess when a legacy field has no target).
fn convert_legacy_gate_to_rung(entry: &toml::Value) -> Option<toml::Value> {
    let table = entry.as_table()?;
    let kind = table.get("kind")?.as_str()?;

    let mut rung = toml::map::Map::new();

    match kind {
        "shell" => {
            let program = table.get("program")?.as_str()?;
            let args: Vec<&str> = table
                .get("args")
                .and_then(|v| v.as_array())
                .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>())
                .unwrap_or_default();
            // Build a single shell command from program + args, quoting
            // arguments that contain whitespace or shell metacharacters.
            let command = if args.is_empty() {
                program.to_string()
            } else {
                let mut parts = vec![program.to_string()];
                for arg in &args {
                    if arg.contains(|c: char| c.is_whitespace() || "\"'\\$;|&<>()".contains(c)) {
                        parts.push(format!("'{}'", arg.replace('\'', "'\\''")));
                    } else {
                        parts.push((*arg).to_string());
                    }
                }
                parts.join(" ")
            };
            rung.insert("name".to_string(), toml::Value::String(program.to_string()));
            rung.insert("command".to_string(), toml::Value::String(command));
        }
        "compile" => {
            let build_system = table
                .get("build_system")
                .and_then(|v| v.as_str())
                .unwrap_or("cargo");
            let command = match build_system {
                "cargo" => "cargo check --workspace",
                "npm" => "npm run build",
                "go" => "go build ./...",
                other => {
                    tracing::warn!(
                        build_system = other,
                        "cannot migrate compile gate for unknown build system"
                    );
                    return None;
                }
            };
            rung.insert(
                "name".to_string(),
                toml::Value::String("compile".to_string()),
            );
            rung.insert(
                "command".to_string(),
                toml::Value::String(command.to_string()),
            );
        }
        "clippy" => {
            let build_system = table
                .get("build_system")
                .and_then(|v| v.as_str())
                .unwrap_or("cargo");
            let command = match build_system {
                "cargo" => "cargo clippy --workspace --no-deps -- -D warnings",
                other => {
                    tracing::warn!(
                        build_system = other,
                        "cannot migrate clippy gate for unknown build system"
                    );
                    return None;
                }
            };
            rung.insert(
                "name".to_string(),
                toml::Value::String("clippy".to_string()),
            );
            rung.insert(
                "command".to_string(),
                toml::Value::String(command.to_string()),
            );
        }
        "test" => {
            let build_system = table
                .get("build_system")
                .and_then(|v| v.as_str())
                .unwrap_or("cargo");
            let command = match build_system {
                "cargo" => "cargo test --workspace",
                "npm" => "npm test",
                "go" => "go test ./...",
                other => {
                    tracing::warn!(
                        build_system = other,
                        "cannot migrate test gate for unknown build system"
                    );
                    return None;
                }
            };
            rung.insert("name".to_string(), toml::Value::String("test".to_string()));
            rung.insert(
                "command".to_string(),
                toml::Value::String(command.to_string()),
            );
        }
        _ => return None, // Unknown kind: cannot migrate losslessly.
    }

    // Convert timeout_ms to timeout_secs (only when lossless: exact
    // millisecond values that are not whole seconds are rejected per spec).
    if let Some(timeout_ms) = table.get("timeout_ms").and_then(|v| v.as_integer())
        && timeout_ms > 0
    {
        if timeout_ms % 1000 != 0 {
            tracing::warn!(
                timeout_ms,
                "cannot losslessly convert gate timeout_ms to timeout_secs"
            );
            return None;
        }
        rung.insert(
            "timeout_secs".to_string(),
            toml::Value::Integer(timeout_ms / 1000),
        );
    }

    rung.insert("required".to_string(), toml::Value::Boolean(true));

    Some(toml::Value::Table(rung))
}

fn rename_if_absent(table: &mut toml::map::Map<String, toml::Value>, old: &str, new: &str) {
    if table.contains_key(new) {
        table.remove(old);
    } else if let Some(value) = table.remove(old) {
        table.insert(new.to_string(), value);
    }
}

/// Global dedup set for config diagnostic warnings.
/// Prevents the same warning from being logged on every config reload.
static EMITTED_DIAGNOSTICS: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();

/// Serializes tests that mutate or observe process-wide configuration variables.
#[cfg(test)]
pub(crate) static TEST_ENV_LOCK: Mutex<()> = Mutex::new(());

fn emitted_diagnostics() -> &'static Mutex<HashSet<String>> {
    EMITTED_DIAGNOSTICS.get_or_init(|| Mutex::new(HashSet::new()))
}

// ─── Load options ───────────────────────────────────────────────────────

/// Controls how the unified loader behaves.
#[derive(Clone, Debug)]
pub struct LoadOptions {
    /// Merge providers/models from `~/.roko/config.toml`.
    pub merge_global: bool,
    /// Apply named env var overrides (ROKO_MODEL, ROKO_BACKEND, etc.).
    pub apply_env_overrides: bool,
    /// Apply hierarchical `ROKO__SECTION__FIELD` env overrides.
    pub apply_hierarchical_env: bool,
    /// Apply strict safety validation (reject `dangerously_skip_permissions`).
    pub strict_validation: bool,
}

impl Default for LoadOptions {
    fn default() -> Self {
        Self {
            merge_global: true,
            apply_env_overrides: true,
            apply_hierarchical_env: true,
            strict_validation: false,
        }
    }
}

impl LoadOptions {
    /// Options for ACP / Zed integration: lenient, with global merge.
    ///
    /// Currently identical to `Default`, but kept as a named constructor so
    /// ACP-specific divergences (e.g. workspace-scoped overrides) can be
    /// added without touching every callsite.
    #[must_use]
    pub fn acp() -> Self {
        Self::default()
    }

    /// Options for strict / inherited config loading.
    #[must_use]
    pub fn strict() -> Self {
        Self {
            merge_global: false,
            apply_env_overrides: false,
            apply_hierarchical_env: false,
            strict_validation: true,
        }
    }
}

// ─── Public API ─────────────────────────────────────────────────────────

/// Load config with all defaults: global merge + env overrides, no strict validation.
///
/// This is the function that all `load_roko_config()` callsites should migrate to.
pub fn load_config_unified(workdir: &Path) -> Result<RokoConfig, LoadConfigError> {
    load_config_with_options(workdir, &LoadOptions::default())
}

/// Load config with custom options.
pub fn load_config_with_options(
    workdir: &Path,
    opts: &LoadOptions,
) -> Result<RokoConfig, LoadConfigError> {
    let path = find_config_path(workdir);
    load_from_resolved_path(&path, opts)
}

/// Load config from one explicit file path.
///
/// This bypasses `ROKO_CONFIG` and ancestor discovery but still applies the
/// processing requested by [`LoadOptions`]: optional global merge, `ROKO__*`
/// env overrides, interpolation, file secrets, and strict validation.
pub fn load_config_file(path: &Path, opts: &LoadOptions) -> Result<RokoConfig, LoadConfigError> {
    load_from_resolved_path(&Some(path.to_path_buf()), opts)
}

/// Resolve an already-parsed source config with the same runtime layers as a file load.
///
/// Transactional config editors use this to validate a prospective effective
/// value before committing source bytes. Runtime-only layers are applied to
/// the owned value returned here; the source representation remains separate.
pub fn resolve_config_source(
    source: RokoConfig,
    source_path: &Path,
    opts: &LoadOptions,
) -> Result<RokoConfig, LoadConfigError> {
    resolve_runtime_layers(source, &Some(source_path.to_path_buf()), opts)
}

/// Load config with full provenance tracking (for CLI `load_resolved_config` compatibility).
///
/// Returns a [`ValidatedConfig`] with diagnostics and provenance info.
pub fn load_config_validated(workdir: &Path) -> Result<ValidatedConfig, LoadConfigError> {
    load_config_validated_with_options(workdir, &LoadOptions::default())
}

/// Load config with provenance tracking and custom options.
///
/// Returns a [`ValidatedConfig`] where `raw` is the parsed-only config
/// (before env overrides and secret interpolation) and `migrated` is the
/// fully resolved config.
pub fn load_config_validated_with_options(
    workdir: &Path,
    opts: &LoadOptions,
) -> Result<ValidatedConfig, LoadConfigError> {
    let path = find_config_path(workdir);

    // Parse + validate (no env overrides or secret resolution yet).
    let parsed = parse_from_resolved_path(&path, opts)?;
    let raw = parsed.config;

    // Apply the same runtime layers as ordinary loading while retaining the
    // parsed source value independently for provenance and safe editing.
    let (migrated, merge_context) = resolve_runtime_layers_with_context(
        raw.clone(),
        &path,
        opts,
        Some(&parsed.explicit_fields),
    )?;

    let mut diagnostics = parsed.diagnostics;
    diagnostics.extend(collect_diagnostics(&migrated));
    diagnostics.extend(invariant_diagnostics(&migrated));

    let mut provenance = match &path {
        Some(p) => vec![ConfigProvenance::file(p.clone(), "roko.toml")],
        None => vec![ConfigProvenance::default(
            "roko.toml",
            "missing file; using built-in defaults",
        )],
    };

    if let Some(report) = parsed.migration_report {
        for step in report.steps_applied {
            provenance.push(ConfigProvenance::migration(
                "schema_version",
                step.description,
            ));
        }
    }

    // Record which hierarchical env overrides were applied.
    if opts.apply_hierarchical_env {
        let env_paths = collect_hierarchical_env_paths();
        for path_key in &env_paths {
            provenance.push(ConfigProvenance::env(
                path_key.clone(),
                format!(
                    "ROKO__{} env override",
                    path_key.to_ascii_uppercase().replace('.', "__")
                ),
            ));
        }
    }

    Ok(ValidatedConfig {
        raw,
        migrated,
        diagnostics,
        provenance,
        merge_context,
    })
}

struct ParsedConfig {
    config: RokoConfig,
    diagnostics: Vec<ConfigDiagnostic>,
    migration_report: Option<MigrationReport>,
    explicit_fields: Vec<String>,
}

/// Parse config from an already-resolved path (read + validate + parse only).
///
/// Does NOT apply global merge, env overrides, or secret interpolation.
/// Use this when you need the raw parsed config before mutations.
fn parse_from_resolved_path(
    path: &Option<PathBuf>,
    opts: &LoadOptions,
) -> Result<ParsedConfig, LoadConfigError> {
    // 1. Read the raw text once (returns default if no file).
    let raw_text = match path {
        Some(p) => Some(
            std::fs::read_to_string(p).map_err(|source| LoadConfigError::Read {
                path: p.clone(),
                source,
            })?,
        ),
        None => None,
    };

    // 2. Optionally apply strict validation on the raw text.
    if opts.strict_validation
        && let (Some(p), Some(text)) = (path, &raw_text)
    {
        let strict_source = super::validation::StrictConfigSource::shared(Some(p.clone()));
        super::validation::validate_strict_config_toml(text, &strict_source).map_err(|source| {
            LoadConfigError::Validation {
                path: p.clone(),
                source,
            }
        })?;
    }

    // 3. Parse to a value tree, migrate, then deserialize. Migrations must run
    // before serde so renamed fields are not silently discarded.
    match (&path, raw_text) {
        (Some(p), Some(text)) => {
            let mut value =
                text.parse::<toml::Value>()
                    .map_err(|source| LoadConfigError::Parse {
                        path: p.clone(),
                        source,
                    })?;
            refuse_readable_secrets(p, &value)?;
            let diagnostics = unknown_field_diagnostics(&value);
            for diagnostic in &diagnostics {
                tracing::warn!(
                    config_key = %diagnostic.key,
                    "config warning: {}",
                    diagnostic.message
                );
            }

            let migrator = ConfigMigrator::new();
            let report = migrator.migrate(&mut value);
            if report.to_version < migrator.target_version {
                return Err(LoadConfigError::Migration {
                    path: p.clone(),
                    message: report.warnings.join("; "),
                });
            }
            for warning in &report.warnings {
                tracing::warn!(path = %p.display(), "config migration warning: {warning}");
            }
            let explicit_fields = toml_leaf_paths(&value);
            // Strip unknown fields so that `deny_unknown_fields` on
            // RokoConfig (and its sub-structs) does not reject
            // forward-compatible or custom keys that were already
            // diagnosed above.
            let schema = build_schema_tree();
            strip_unknown_fields(&mut value, &schema, "");
            let config =
                value
                    .try_into::<RokoConfig>()
                    .map_err(|source| LoadConfigError::Parse {
                        path: p.clone(),
                        source,
                    })?;
            Ok(ParsedConfig {
                config,
                diagnostics,
                migration_report: Some(report),
                explicit_fields,
            })
        }
        _ => Ok(ParsedConfig {
            config: RokoConfig::default(),
            diagnostics: Vec::new(),
            migration_report: None,
            explicit_fields: Vec::new(),
        }),
    }
}

/// Internal: load config from an already-resolved path with full processing.
///
/// All public functions resolve the path once via [`find_config_path`] then
/// delegate here, avoiding double discovery and double file reads.
fn load_from_resolved_path(
    path: &Option<PathBuf>,
    opts: &LoadOptions,
) -> Result<RokoConfig, LoadConfigError> {
    let parsed = parse_from_resolved_path(path, opts)?;
    let config = resolve_runtime_layers_with_context(
        parsed.config,
        path,
        opts,
        Some(&parsed.explicit_fields),
    )?
    .0;

    // Emit diagnostics as warnings so callers don't need to opt into
    // load_config_validated() to see slug duplicates and orphaned models.
    // Deduplicated: each unique key is only logged once per process lifetime.
    {
        let mut emitted = emitted_diagnostics().lock();
        for diag in parsed
            .diagnostics
            .into_iter()
            .chain(collect_diagnostics(&config))
            .chain(invariant_diagnostics(&config))
        {
            if diag.key.starts_with('_') {
                // Skip the env-override meta-note; it's noise on the hot path.
                continue;
            }
            if emitted.insert(diag.key.clone()) {
                tracing::debug!(
                    config_key = %diag.key,
                    "config warning: {}",
                    diag.message
                );
            }
        }
    }

    Ok(config)
}

/// Apply runtime-only config layers to a parsed source value.
///
/// File loading, provenance loading, and pre-commit validation all delegate
/// here so their precedence and validation semantics cannot drift.
fn resolve_runtime_layers(
    config: RokoConfig,
    path: &Option<PathBuf>,
    opts: &LoadOptions,
) -> Result<RokoConfig, LoadConfigError> {
    resolve_runtime_layers_with_context(config, path, opts, None).map(|(config, _)| config)
}

fn resolve_runtime_layers_with_context(
    mut config: RokoConfig,
    path: &Option<PathBuf>,
    opts: &LoadOptions,
    explicit_fields: Option<&[String]>,
) -> Result<(RokoConfig, MergeContext), LoadConfigError> {
    let mut merge_context = MergeContext::default();
    if path.is_none() {
        merge_context.record(FieldProvenance::new(
            "roko.toml",
            ConfigSource::Default,
            Some("missing file; using built-in defaults".to_string()),
        ));
    }

    if opts.merge_global {
        let before = config.clone();
        merge_global_into(&mut config)?;
        if let Some(fields) = explicit_fields {
            reapply_explicit_fields(&mut config, &before, fields);
        }
        record_changed_fields(
            &mut merge_context,
            &before,
            &config,
            ConfigSource::File,
            "global config merge",
        );
    }
    if let Some(fields) = explicit_fields {
        for field in fields {
            merge_context.record(FieldProvenance::new(
                field.clone(),
                ConfigSource::File,
                Some(path.as_ref().map_or_else(
                    || "explicit config source".to_string(),
                    |path| format!("explicit field in {}", path.display()),
                )),
            ));
        }
    }
    if opts.apply_env_overrides {
        let before = config.clone();
        config.apply_process_env();
        record_changed_fields(
            &mut merge_context,
            &before,
            &config,
            ConfigSource::Env,
            "named ROKO_* environment override",
        );
    }
    if opts.apply_hierarchical_env {
        let before = config.clone();
        apply_hierarchical_env_overrides(&mut config);
        record_changed_fields(
            &mut merge_context,
            &before,
            &config,
            ConfigSource::Env,
            "hierarchical ROKO__* environment override",
        );
    }
    expand_secret_references(&mut config)?;
    config.interpolate_env_vars();
    config.resolve_file_secrets();
    // The process's secret scrubber (when one is installed) also redacts the
    // keys this config's providers read, and the secrets the config holds:
    // secret fields such as serve.auth.api_key, header values and file
    // secrets.
    crate::obs::add_secret_env_values(
        config
            .providers
            .values()
            .filter_map(|provider| provider.api_key_env.as_deref()),
    );
    if crate::obs::secret_scrubber().is_some() {
        let secrets = config_secret_values(&config);
        crate::obs::add_secret_values(
            secrets
                .iter()
                .map(|(field, value)| (field.as_str(), value.as_str())),
        );
    }

    // Post-merge provider reference validation.
    // When strict_validation is enabled in config, dangling model->provider
    // references become hard errors instead of warnings.
    if !config.models.is_empty() {
        validate_provider_references(&config, path)?;
    }

    let invariants = validate_invariants(&config);
    if let Some(error) = invariants
        .iter()
        .find(|result| result.severity == InvariantSeverity::Error)
    {
        return Err(LoadConfigError::InvariantViolation {
            invariant_id: error.invariant_id,
            message: error.message.clone(),
        });
    }
    for warning in invariants
        .iter()
        .filter(|result| result.severity == InvariantSeverity::Warning)
    {
        tracing::warn!(
            invariant_id = warning.invariant_id,
            config_path = %warning.config_path,
            "config invariant warning: {}",
            warning.message
        );
    }

    // Post-merge provider/model semantic validation (#370).
    // The loader warns rather than hard-errors so that inspection commands
    // (config show, config validate) still work. Dispatch preflight checks
    // are the authority for blocking.
    let semantic_findings = validate_provider_semantics(&config);
    for finding in &semantic_findings {
        match finding.severity {
            InvariantSeverity::Error => {
                tracing::warn!(
                    code = %finding.code,
                    path = %finding.path,
                    "provider/model semantic error: {}",
                    finding.message
                );
            }
            InvariantSeverity::Warning => {
                tracing::debug!(
                    code = %finding.code,
                    path = %finding.path,
                    "provider/model semantic warning: {}",
                    finding.message
                );
            }
        }
    }

    Ok((config, merge_context))
}

fn record_changed_fields(
    merge_context: &mut MergeContext,
    before: &RokoConfig,
    after: &RokoConfig,
    source: ConfigSource,
    reason: &str,
) {
    let (Ok(before), Ok(after)) = (
        toml::Value::try_from(before.clone()),
        toml::Value::try_from(after.clone()),
    ) else {
        return;
    };
    for key in changed_toml_paths(&before, &after) {
        merge_context.record(FieldProvenance::new(
            key,
            source.clone(),
            Some(reason.to_string()),
        ));
    }
}

fn changed_toml_paths(before: &toml::Value, after: &toml::Value) -> Vec<String> {
    let mut before_values = BTreeMap::new();
    let mut after_values = BTreeMap::new();
    flatten_toml(before, "", &mut before_values);
    flatten_toml(after, "", &mut after_values);
    before_values
        .keys()
        .chain(after_values.keys())
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .filter(|key| before_values.get(*key) != after_values.get(*key))
        .cloned()
        .collect()
}

fn toml_leaf_paths(value: &toml::Value) -> Vec<String> {
    let mut values = BTreeMap::new();
    flatten_toml(value, "", &mut values);
    values.into_keys().collect()
}

fn flatten_toml(value: &toml::Value, prefix: &str, output: &mut BTreeMap<String, toml::Value>) {
    match value {
        toml::Value::Table(table) if !table.is_empty() => {
            for (key, value) in table {
                let path = if prefix.is_empty() {
                    key.clone()
                } else {
                    format!("{prefix}.{key}")
                };
                flatten_toml(value, &path, output);
            }
        }
        _ if !prefix.is_empty() => {
            output.insert(prefix.to_string(), value.clone());
        }
        _ => {}
    }
}

fn reapply_explicit_fields(config: &mut RokoConfig, source: &RokoConfig, fields: &[String]) {
    let (Ok(mut effective), Ok(source)) = (
        toml::Value::try_from(config.clone()),
        toml::Value::try_from(source.clone()),
    ) else {
        return;
    };
    for field in fields {
        if let Some(value) = toml_value_at_path(&source, field).cloned() {
            set_toml_value(&mut effective, field, value);
        }
    }
    match effective.try_into::<RokoConfig>() {
        Ok(updated) => *config = updated,
        Err(error) => tracing::warn!(
            %error,
            "failed to reapply explicit project fields after global config merge"
        ),
    }
}

fn toml_value_at_path<'a>(value: &'a toml::Value, path: &str) -> Option<&'a toml::Value> {
    path.split('.')
        .try_fold(value, |current, segment| current.as_table()?.get(segment))
}

fn set_toml_value(root: &mut toml::Value, path: &str, value: toml::Value) {
    let segments = path.split('.').collect::<Vec<_>>();
    let Some((leaf, parents)) = segments.split_last() else {
        return;
    };
    let mut current = root;
    for segment in parents {
        let Some(table) = current.as_table_mut() else {
            return;
        };
        let Some(next) = table.get_mut(*segment) else {
            return;
        };
        current = next;
    }
    if let Some(table) = current.as_table_mut() {
        table.insert((*leaf).to_string(), value);
    }
}

/// Validate that all model profiles reference providers that exist in the
/// merged config. In strict mode, missing references become hard errors.
/// In lenient mode (default), they are logged as warnings only — the
/// `collect_diagnostics()` call handles the lenient warning path.
fn validate_provider_references(
    config: &RokoConfig,
    path: &Option<PathBuf>,
) -> Result<(), LoadConfigError> {
    if !config.validation.strict_validation {
        return Ok(());
    }

    for (model_key, model_profile) in &config.models {
        if !config.providers.contains_key(&model_profile.provider) {
            let msg = format!(
                "model '{}' references provider '{}' which does not exist in the merged config. \
                 Check roko.toml [models.{}] and ensure [providers.{}] is defined.",
                model_key, model_profile.provider, model_key, model_profile.provider
            );
            return Err(LoadConfigError::ProviderReference {
                path: path.clone().unwrap_or_default(),
                model_key: model_key.clone(),
                provider_key: model_profile.provider.clone(),
                message: msg,
            });
        }
    }
    Ok(())
}

/// Normalize agent-facing model aliases to canonical config keys and reject
/// ambiguity before a caller can construct an agent runtime.
///
/// Model keys are stable internal identities. Provider-facing slugs remain
/// ergonomic aliases only when exactly one configured model owns the slug.
/// An empty registry retains the legacy CLI-only dispatch path.
pub fn normalize_and_validate_dispatch_models(
    config: &mut RokoConfig,
) -> Result<(), LoadConfigError> {
    let index = DispatchModelIndex::from_config(config)?;
    normalize_dispatch_references(config, &index, true)
}

/// Normalize source and effective dispatch references against one runtime namespace.
///
/// Config editors must derive `effective` through the complete runtime-layer
/// pipeline before calling this helper. The effective model registry then
/// governs both projections, so a runtime-layer exact key cannot be mistaken
/// for a source-only slug alias. Only dispatch reference strings are changed in
/// `source`; providers, models, secrets, and other runtime overlays are never
/// copied into it. Source references masked by runtime field overrides remain
/// verbatim when unresolved, because only the effective projection governs
/// live dispatch validity.
pub fn normalize_source_and_effective_dispatch_models(
    source: &mut RokoConfig,
    effective: &mut RokoConfig,
) -> Result<(), LoadConfigError> {
    let index = DispatchModelIndex::from_config(effective)?;
    normalize_dispatch_references(effective, &index, true)?;
    normalize_dispatch_references(source, &index, false)
}

struct DispatchModelIndex {
    keys: HashSet<String>,
    aliases: std::collections::HashMap<String, String>,
}

impl DispatchModelIndex {
    fn from_config(config: &RokoConfig) -> Result<Self, LoadConfigError> {
        if config.models.is_empty() {
            return Ok(Self {
                keys: HashSet::new(),
                aliases: std::collections::HashMap::new(),
            });
        }

        let mut slug_owners = std::collections::BTreeMap::<String, Vec<String>>::new();
        for (key, profile) in &config.models {
            slug_owners
                .entry(profile.slug.trim().to_string())
                .or_default()
                .push(key.clone());
        }

        for (slug, owners) in &mut slug_owners {
            owners.sort_unstable();
            if owners.len() > 1 {
                return Err(LoadConfigError::AmbiguousModelSlug {
                    slug: slug.clone(),
                    model_keys: owners.join(", "),
                });
            }
        }

        let aliases = slug_owners
            .into_iter()
            .filter_map(|(slug, owners)| owners.into_iter().next().map(|key| (slug, key)))
            .collect::<std::collections::HashMap<_, _>>();
        let keys = config.models.keys().cloned().collect::<HashSet<_>>();

        Ok(Self { keys, aliases })
    }
}

fn normalize_dispatch_references(
    config: &mut RokoConfig,
    index: &DispatchModelIndex,
    reject_unresolved: bool,
) -> Result<(), LoadConfigError> {
    if index.keys.is_empty() {
        return Ok(());
    }

    normalize_model_reference(
        &mut config.agent.default_model,
        "agent.default_model",
        &index.keys,
        &index.aliases,
        reject_unresolved,
    )?;

    if let Some(fallback) = config.agent.fallback_model.as_mut() {
        normalize_model_reference(
            fallback,
            "agent.fallback_model",
            &index.keys,
            &index.aliases,
            reject_unresolved,
        )?;
    }

    let mut tiers = config.agent.tier_models.keys().cloned().collect::<Vec<_>>();
    tiers.sort_unstable();
    for tier in tiers {
        let model = config
            .agent
            .tier_models
            .get_mut(&tier)
            .expect("tier key was collected from the same map");
        normalize_model_reference(
            model,
            &format!("agent.tier_models.{tier}"),
            &index.keys,
            &index.aliases,
            reject_unresolved,
        )?;
    }

    let mut roles = config.agent.roles.keys().cloned().collect::<Vec<_>>();
    roles.sort_unstable();
    for role in roles {
        let role_config = config
            .agent
            .roles
            .get_mut(&role)
            .expect("role key was collected from the same map");
        if let Some(model) = role_config.model.as_mut() {
            normalize_model_reference(
                model,
                &format!("agent.roles.{role}.model"),
                &index.keys,
                &index.aliases,
                reject_unresolved,
            )?;
        }
    }

    Ok(())
}

fn normalize_model_reference(
    model: &mut String,
    field: &str,
    keys: &HashSet<String>,
    aliases: &std::collections::HashMap<String, String>,
    reject_unresolved: bool,
) -> Result<(), LoadConfigError> {
    let reference = model.trim();
    if keys.contains(reference) {
        if reference.len() != model.len() {
            *model = reference.to_string();
        }
        return Ok(());
    }
    if let Some(key) = aliases.get(reference) {
        *model = key.clone();
        return Ok(());
    }
    if !reject_unresolved {
        return Ok(());
    }
    Err(LoadConfigError::UnresolvedModel {
        field: field.to_string(),
        model: reference.to_string(),
    })
}

// ─── Hierarchical env override support ───────────────────────────────────

/// Convert a `ROKO__SECTION__FIELD` env var key to a dotted config path.
///
/// Strips the `ROKO__` prefix, lowercases, and replaces `__` with `.`.
/// Returns `None` if the key does not start with `ROKO__` or is empty after
/// stripping the prefix.
fn hierarchical_env_to_path(key: &str) -> Option<String> {
    let suffix = key.strip_prefix("ROKO__")?;
    if suffix.is_empty() {
        return None;
    }
    Some(suffix.to_ascii_lowercase().replace("__", "."))
}

/// The `ROKO__` variable that sets config field `path`, if one can.
///
/// `serve.auth.api_key` is set by `ROKO__SERVE__AUTH__API_KEY`. A variable
/// name holds only ASCII letters, digits and underscores, and the loader
/// reads it back in lowercase, so a field such as `providers.My-Key.api_key`
/// has no variable.
#[must_use]
pub fn env_override_name(path: &str) -> Option<String> {
    let name = format!("ROKO__{}", path.to_ascii_uppercase().replace('.', "__"));
    let valid = name
        .bytes()
        .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit() || byte == b'_');
    (valid && hierarchical_env_to_path(&name).as_deref() == Some(path)).then_some(name)
}

/// Collect all `ROKO__*` env vars that represent hierarchical config paths.
///
/// Returns the list of dotted config paths that were found in the environment.
fn collect_hierarchical_env_paths() -> Vec<String> {
    collect_hierarchical_env_paths_from(std::env::vars())
}

/// Collect hierarchical env paths from an iterator (testable).
fn collect_hierarchical_env_paths_from<I>(vars: I) -> Vec<String>
where
    I: IntoIterator<Item = (String, String)>,
{
    vars.into_iter()
        .filter_map(|(key, _)| hierarchical_env_to_path(&key))
        .collect()
}

/// Apply hierarchical `ROKO__SECTION__FIELD` env overrides to a config.
///
/// This works by serializing the config to TOML, applying the overrides to the
/// TOML value tree, then deserializing back. This approach uses serde's
/// structured handling rather than ad-hoc string edits.
fn apply_hierarchical_env_overrides(config: &mut RokoConfig) {
    apply_hierarchical_env_overrides_from(config, std::env::vars());
}

/// Apply hierarchical env overrides from a given set of vars (testable).
pub(crate) fn apply_hierarchical_env_overrides_from<I>(config: &mut RokoConfig, vars: I)
where
    I: IntoIterator<Item = (String, String)>,
{
    // Collect all ROKO__* vars and convert to dotted paths.
    let overrides: Vec<(String, String)> = vars
        .into_iter()
        .filter_map(|(key, value)| hierarchical_env_to_path(&key).map(|path| (path, value)))
        .collect();

    if overrides.is_empty() {
        return;
    }

    // Serialize current config to a TOML value tree.
    let mut toml_value = match toml::Value::try_from(config.clone()) {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!(error = %e, "failed to serialize config for hierarchical env overrides");
            return;
        }
    };

    // Apply each override to the TOML tree.
    for (path, value) in &overrides {
        set_toml_value_at_path(&mut toml_value, path, value);
    }

    // Deserialize back into RokoConfig.
    match toml_value.try_into::<RokoConfig>() {
        Ok(updated) => *config = updated,
        Err(e) => {
            tracing::warn!(
                error = %e,
                "failed to deserialize config after hierarchical env overrides; \
                 overrides will be partially applied"
            );
        }
    }
}

/// Set a value in a TOML value tree at a dotted path.
///
/// Creates intermediate tables as needed. The value is parsed as a TOML
/// literal (bool, integer, float) or stored as a string. A field that holds
/// a string keeps the value as a string, so a key such as `12345` does not
/// become a number that fails to load.
fn set_toml_value_at_path(root: &mut toml::Value, path: &str, raw_value: &str) {
    let segments: Vec<&str> = path.split('.').collect();
    if segments.is_empty() {
        return;
    }

    // Navigate to the parent table, creating intermediate tables if needed.
    let mut current = root;
    for segment in &segments[..segments.len() - 1] {
        let table = match current.as_table_mut() {
            Some(t) => t,
            None => return, // Path doesn't resolve to a table; skip this override.
        };
        current = table
            .entry(*segment)
            .or_insert_with(|| toml::Value::Table(toml::map::Map::new()));
    }

    // Set the leaf value.
    let leaf_key = segments[segments.len() - 1];
    if let Some(table) = current.as_table_mut() {
        let parsed_value = match table.get(leaf_key) {
            Some(toml::Value::String(_)) => toml::Value::String(raw_value.to_string()),
            _ => parse_env_value_to_toml(raw_value),
        };
        table.insert(leaf_key.to_string(), parsed_value);
    }
}

/// Parse a raw env var value into an appropriate TOML value type.
///
/// Attempts to parse as: bool (word-form only), integer, float; falls back to string.
/// Note: "0" and "1" are treated as integers, NOT booleans, to avoid breaking
/// numeric config fields. Only explicit words like "true"/"false"/"yes"/"no"
/// are treated as booleans.
fn parse_env_value_to_toml(raw: &str) -> toml::Value {
    // Try bool (word-form only; "0"/"1" are integers).
    match raw.to_ascii_lowercase().as_str() {
        "true" | "yes" | "on" => return toml::Value::Boolean(true),
        "false" | "no" | "off" => return toml::Value::Boolean(false),
        _ => {}
    }

    // Try integer.
    if let Ok(n) = raw.parse::<i64>() {
        return toml::Value::Integer(n);
    }

    // Try float (only if it contains a dot to avoid int->float coercion).
    if raw.contains('.')
        && let Ok(f) = raw.parse::<f64>()
    {
        return toml::Value::Float(f);
    }

    // Fall back to string.
    toml::Value::String(raw.to_string())
}

/// Collect semantic diagnostics from a fully-loaded config.
///
/// Checks: outdated config version, orphaned model→provider references,
/// duplicate model slugs.
fn collect_diagnostics(config: &RokoConfig) -> Vec<ConfigDiagnostic> {
    let mut diagnostics = Vec::new();

    if config.config_version < super::schema::CURRENT_CONFIG_VERSION {
        diagnostics.push(ConfigDiagnostic {
            key: "config_version".to_string(),
            message: format!(
                "config_version={} is older than current {}; consider running a migration",
                config.config_version,
                super::schema::CURRENT_CONFIG_VERSION,
            ),
        });
    }

    // Validate models reference existing providers.
    for (key, profile) in &config.models {
        if !config.providers.contains_key(&profile.provider) {
            diagnostics.push(ConfigDiagnostic {
                key: format!("models.{key}.provider"),
                message: format!(
                    "model '{}' references provider '{}' which is not configured",
                    key, profile.provider
                ),
            });
        }

        if let Some(max_output) = profile.max_output
            && max_output < 1_000
        {
            diagnostics.push(ConfigDiagnostic {
                key: format!("models.{key}.max_output"),
                message: format!(
                    "model '{}' sets max_output={} which is unusually low for IDE usage",
                    key, max_output
                ),
            });
        }
    }

    // Check for duplicate slugs.
    let mut slug_to_keys: std::collections::HashMap<&str, Vec<&str>> =
        std::collections::HashMap::new();
    for (key, profile) in &config.models {
        slug_to_keys
            .entry(profile.slug.as_str())
            .or_default()
            .push(key.as_str());
    }
    for (slug, keys) in &slug_to_keys {
        if keys.len() > 1 {
            // Collect the distinct providers for all keys that share this slug.
            let providers: std::collections::HashSet<&str> = keys
                .iter()
                .filter_map(|k| config.models.get(*k).map(|p| p.provider.as_str()))
                .collect();
            // Same-provider duplicates are intentional aliases — skip the
            // diagnostic. Cross-provider duplicates are genuine
            // misconfigurations and should still be reported.
            if providers.len() > 1 {
                diagnostics.push(ConfigDiagnostic {
                    key: format!("models.*.slug={slug}"),
                    message: format!(
                        "duplicate model slug '{}' defined by keys: {}",
                        slug,
                        keys.join(", ")
                    ),
                });
            }
        }
    }

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

fn invariant_diagnostics(config: &RokoConfig) -> Vec<ConfigDiagnostic> {
    let mut diags: Vec<ConfigDiagnostic> = validate_invariants(config)
        .into_iter()
        .filter(|result| result.severity == InvariantSeverity::Warning)
        .map(|result| ConfigDiagnostic {
            key: format!("invariant.{}.{}", result.invariant_id, result.config_path),
            message: result.message,
        })
        .collect();

    // Provider/model semantic warnings (errors are handled by the explicit
    // `roko config validate` CLI path, not here).
    diags.extend(
        super::validation::validate_provider_semantics(config)
            .into_iter()
            .filter(|f| f.severity == InvariantSeverity::Warning)
            .map(|f| ConfigDiagnostic {
                key: format!("semantic.{}", f.path),
                message: format!("[{}] {}", f.code, f.message),
            }),
    );

    diags
}

/// Sections whose keys are user-defined names (dynamic maps).
///
/// Keys under these dotted paths are treated as dynamic identifiers and are not
/// validated against the schema tree. Their *values* are validated against the
/// value schema of the map. A `*` segment matches any one user-defined key.
const DYNAMIC_MAP_SECTIONS: &[&str] = &[
    "providers",
    "providers.*.extra_headers",
    "models",
    "profiles",
    "agent.roles",
    "agent.tier_models",
    "gates.domain_gates",
    "gates.max_output_tokens",
    "retrieval.role_token_budgets",
    "tools.profiles",
];

/// Dynamic map sections whose entries are structs that deny unknown fields
/// (`ProviderConfig`, `ModelProfile`). Loading strips an unknown key inside
/// one of their entries, as it does anywhere else; serde itself ignores or
/// collects unknown keys in the other sections' entries.
const STRICT_ENTRY_SECTIONS: &[&str] = &["providers", "models"];

/// Whether the dotted `path` names a dynamic map section.
fn is_dynamic_section(path: &str) -> bool {
    !path.is_empty()
        && DYNAMIC_MAP_SECTIONS.iter().any(|pattern| {
            let mut keys = path.split('.');
            pattern
                .split('.')
                .all(|want| keys.next().is_some_and(|key| want == "*" || want == key))
                && keys.next().is_none()
        })
}

/// Sections that were removed from the schema and must produce a targeted
/// migration/removal diagnostic rather than a generic "unknown field" message.
const LEGACY_REMOVED_SECTIONS: &[(&str, &str)] = &[
    (
        "isfr",
        "the [isfr] section was removed (dead rate-oracle vestige); \
         delete it from your config",
    ),
    (
        "gate",
        "top-level [[gate]] is a legacy alias ignored by runner-v2; \
         migrate entries to [[gates.rungs]] via `roko config migrate` \
         or move them manually under [gates]",
    ),
];

/// Validate every path in the input TOML against the `RokoConfig` schema.
///
/// Builds an allowed-key tree by serializing a default `RokoConfig` to a
/// `toml::Value`, then walks the input recursively. Dynamic map sections
/// (providers, models, profiles, agent.roles, tools.profiles) treat their
/// keys as user-defined names and validate each value against the map's
/// value schema. Legacy removed sections produce targeted diagnostics.
///
/// This replaces the previous top-level-only `unknown_field_diagnostics`.
pub fn validate_known_config_paths(value: &toml::Value) -> Vec<ConfigDiagnostic> {
    let schema = build_schema_tree();
    let mut diagnostics = Vec::new();
    walk_config_paths(value, &schema, "", &mut diagnostics);
    diagnostics
}

/// The schema's value at the dotted config `path`, if the path is known.
///
/// Reads the schema tree that [`validate_known_config_paths`] checks against.
/// The value is a placeholder: only its TOML type is meaningful. Keys below a
/// dynamic map section (`providers.<name>`, `models.<name>`, ...) resolve
/// through the map's value template, as validation does; a map without a
/// template has no types to offer, so paths below it resolve to `None`.
#[must_use]
pub fn schema_value_for_path(path: &str) -> Option<toml::Value> {
    let schema = build_schema_tree();
    let mut node = &schema;
    let mut prefix = String::new();
    for segment in path.split('.') {
        let table = node.as_table()?;
        node = if is_dynamic_section(&prefix) {
            table.values().next()?
        } else {
            table.get(segment)?
        };
        if !prefix.is_empty() {
            prefix.push('.');
        }
        prefix.push_str(segment);
    }
    Some(node.clone())
}

/// Build a schema tree that includes all `RokoConfig` fields, including those
/// skipped by `skip_serializing_if` on the default value.
///
/// The default `RokoConfig` serializes most sections, but empty collections
/// with `skip_serializing_if = "Vec::is_empty"` (subscriptions, agents,
/// groups, repos) and conditional structs (watcher, profiles) are omitted.
/// We populate those with sentinel entries so the walker accepts them.
///
/// Loading strips every key this tree lacks, so a field that a default
/// config does not serialize (an unset `Option`, an empty collection with
/// `skip_serializing_if`) needs a sentinel here, or its value is silently
/// dropped from every roko.toml. `every_optional_config_key_survives_a_load`
/// covers the fields that have one.
fn build_schema_tree() -> toml::Value {
    use super::agent::{AgentBudget, AgentThresholds, RoleOverride, RoutingOverrides};
    use super::provider::{
        ModelProfile, ProviderConfig, ProviderLimits, ProviderNetworkPolicy, ProviderRouting,
    };
    use super::routing::RewardWeights;
    use super::schema::{DomainProfile, GateProfileConfig};
    use super::subscriptions::SubscriptionConfig;

    let mut config = RokoConfig::default();

    // Add a sentinel entry in each dynamic/conditional section so its keys
    // and value schemas appear in the serialized tree.
    // Provider sentinel: set Optional fields to Some so they appear in the
    // serialized schema tree. The values are never used at runtime.
    let sentinel_provider = ProviderConfig {
        // `extra_headers` maps header names to values (a dynamic map
        // section): one entry gives the type of its values.
        extra_headers: Some(HashMap::from([(
            "_schema_sentinel".to_string(),
            String::new(),
        )])),
        max_concurrent: Some(1),
        limits: Some(ProviderLimits {
            max_cpu_seconds: Some(0),
            max_rss_bytes: Some(0),
            max_processes: Some(0),
            network: ProviderNetworkPolicy::Deny,
            ..ProviderLimits::default()
        }),
        base_url: Some(String::new()),
        api_key_env: Some(String::new()),
        command: Some(String::new()),
        args: Some(Vec::new()),
        require_confirmation: true,
        ..ProviderConfig::default()
    };
    config
        .providers
        .insert("_schema_sentinel".to_string(), sentinel_provider);

    // Model sentinel: set all Optional and skip_serializing_if fields to
    // non-default values so every field appears in the serialized schema tree.
    let sentinel_model = ModelProfile {
        max_output: Some(0),
        supports_thinking: true,
        supports_vision: true,
        supports_web_search: true,
        supports_mcp_tools: true,
        supports_partial: true,
        supports_grounding: true,
        supports_code_execution: true,
        supports_caching: true,
        supports_search: true,
        supports_citations: true,
        supports_async: true,
        is_embedding_model: true,
        provider_routing: Some(ProviderRouting {
            sort: Some(String::new()),
            order: Some(Vec::new()),
            allow_fallbacks: Some(true),
            max_price: Some(0.0),
            require_parameters: Some(Vec::new()),
        }),
        cost_input_per_m: Some(0.0),
        cost_output_per_m: Some(0.0),
        cost_input_per_m_high: Some(0.0),
        cost_output_per_m_high: Some(0.0),
        cost_cache_read_per_m: Some(0.0),
        cost_cache_write_per_m: Some(0.0),
        cost_per_request: Some(0.0),
        thinking_level: Some(String::new()),
        max_tools: Some(0),
        max_tool_iterations: Some(0),
        tokenizer_ratio: Some(0.0),
        search_context_size: Some(String::new()),
        tier: Some(crate::agent::ModelTier::Standard),
        use_max_completion_tokens: true,
        ..ModelProfile::default()
    };
    config
        .models
        .insert("_schema_sentinel".to_string(), sentinel_model);
    // Profile and role sentinels: every optional field set, so config set
    // and validation know each key of a `[profiles.*]` and
    // `[agent.roles.*]` entry.
    let sentinel_profile = DomainProfile {
        base: Some(String::new()),
        model: Some(String::new()),
        effort: Some(String::new()),
        context_limit_k: Some(0),
        max_iterations: Some(0),
        tool_profile: Some(String::new()),
        gate_config: Some(GateProfileConfig {
            skip_tests: Some(false),
            clippy_enabled: Some(false),
            max_rung: Some(0),
        }),
        ..DomainProfile::default()
    };
    config
        .profiles
        .insert("_schema_sentinel".to_string(), sentinel_profile);
    let sentinel_role = RoleOverride {
        role: Some(String::new()),
        model: Some(String::new()),
        backend: Some(String::new()),
        effort: Some(String::new()),
        temperament: Some(Default::default()),
        context_limit_k: Some(0),
        tools: Some(Vec::new()),
        budget: Some(AgentBudget {
            max_tokens_per_turn: Some(0),
            max_cost_usd_cents_per_turn: Some(0),
        }),
        thresholds: Some(AgentThresholds {
            gate_pass_rate_floor: Some(0.0),
        }),
        routing_overrides: Some(RoutingOverrides {
            force_backend: Some(String::new()),
            force_tier: Some(String::new()),
        }),
        turn_budget_usd: Some(0.0),
        capability_requirements: Some(Vec::new()),
        default_effort: Some(String::new()),
        ..RoleOverride::default()
    };
    config
        .agent
        .roles
        .insert("_schema_sentinel".to_string(), sentinel_role);
    // Populate Optional/skip_serializing_if agent fields with non-default
    // values so they appear in the serialized schema tree and are not
    // stripped by `strip_unknown_fields`.
    config.agent.command = Some(String::new());
    config.agent.args = Some(Vec::new());
    config.agent.timeout_ms = Some(0);
    config.agent.env = Some(Vec::new());
    config.agent.env_passthrough = vec![String::new()];
    config.agent.data_llm = Some(super::agent::DataLlmConfig {
        // `output_schema` is free-form JSON. A scalar placeholder keeps any
        // value a file sets, because loading never descends into it.
        output_schema: Some(serde_json::Value::String(String::new())),
        ..Default::default()
    });
    config.agent.defaults.generic_agent_model = Some(String::new());
    config.agent.defaults.gate_judge_model = Some(String::new());
    config.agent.extensions = vec![String::new()];
    config.agent.mcp_config = Some(std::path::PathBuf::new());
    config.agent.default_agent_id = Some(String::new());
    config.agent.disabled_providers = vec![String::new()];
    config.agent.fallback_model = Some(String::new());
    // `tier_models` maps tier names to models (a dynamic map section): one
    // entry puts the table and the type of its values in the tree.
    config
        .agent
        .tier_models
        .insert("_schema_sentinel".to_string(), String::new());
    // Routing and gate lists skip serialization when empty; without these
    // sentinels `strip_unknown_fields` would silently drop them from every
    // roko.toml.
    config.routing.disabled_providers = vec![String::new()];
    config.routing.fallback_models = vec![String::new()];
    config.gates.env_passthrough = vec![String::new()];
    // Populate Optional/skip_serializing_if GitHubConfig fields so they
    // appear in the serialized schema tree and are not stripped.
    config.github.owner = Some(String::new());
    config.github.repo = Some(String::new());
    config.serve.port = Some(0);
    config.project.default_domain = Some(crate::task::TaskDomain::Code);
    config.subscriptions.push(SubscriptionConfig::default());

    // Optional keys of the other sections. The test
    // `every_accepted_config_field_is_in_the_schema_tree` fails when the
    // tree lacks one.
    // `domain_gates` maps domains to gate commands (a dynamic map section).
    config
        .gates
        .domain_gates
        .insert("_schema_sentinel".to_string(), Vec::new());
    config.gates.max_rung = Some(0);
    // `max_output_tokens` maps roles to output-token caps (a dynamic map
    // section).
    config
        .gates
        .max_output_tokens
        .insert("_schema_sentinel".to_string(), 0);
    // `weights` flattens its default `RewardWeights` and may override them
    // per tier.
    let sentinel_weights = RewardWeights {
        knowledge_bias: Some(0.0),
        provider_pass_rate_weight: Some(0.0),
        ..RewardWeights::default()
    };
    let weights = &mut config.routing.weights;
    weights.default = sentinel_weights;
    weights.mechanical = Some(sentinel_weights);
    weights.focused = Some(sentinel_weights);
    weights.integrative = Some(sentinel_weights);
    weights.architectural = Some(sentinel_weights);
    // Every watcher section is unset by default, and every field in it has
    // a serde default, so an empty table builds each one.
    let watchers = &mut config.conductor.watchers;
    watchers.compile_fail_repeat = from_empty_table();
    watchers.context_window_pressure = from_empty_table();
    watchers.cost_overrun = from_empty_table();
    watchers.ghost_turn = from_empty_table();
    watchers.iteration_loop = from_empty_table();
    watchers.review_loop = from_empty_table();
    watchers.spec_drift = from_empty_table();
    watchers.stuck_pattern = from_empty_table();
    watchers.test_failure_budget = from_empty_table();
    watchers.time_overrun = from_empty_table();
    watchers.worktree_count = from_empty_table();
    config.learning.override_learning_dampening = Some(0.0);
    let timeouts = &mut config.timeouts;
    timeouts.hard_run_secs = Some(0);
    timeouts.task_attempt_secs = Some(0);
    timeouts.gate_effect_secs = Some(0);
    timeouts.agent_silence_secs = Some(0);
    timeouts.scheduler_no_progress_secs = Some(0);
    config.serve.event_ingest_allowlist = vec![String::new()];
    config.serve.tracing.otlp_endpoint = Some(String::new());
    let auth = &mut config.serve.auth;
    auth.privy_app_id = Some(String::new());
    auth.privy_workspace_id = Some(String::new());
    auth.privy_allowed_roles = vec![String::new()];
    config.server.auth_token = Some(String::new());
    let deploy = &mut config.deploy;
    deploy.railway_api_token = Some(String::new());
    deploy.project_id = Some(String::new());
    deploy.environment_id = Some(String::new());
    deploy.worker_image = Some(String::new());
    deploy.default_region = Some(String::new());
    let perplexity = &mut config.perplexity;
    perplexity.default_search_model = Some(String::new());
    perplexity.default_research_model = Some(String::new());
    perplexity.default_reasoning_model = Some(String::new());
    perplexity.default_embed_model = Some(String::new());
    perplexity.auto_deep = true;
    let gemini = &mut config.gemini;
    gemini.default_model = Some(String::new());
    gemini.grounding_model = Some(String::new());
    gemini.code_exec_model = Some(String::new());
    gemini.embed_model = Some(String::new());
    let chain = &mut config.chain;
    chain.rpc_url = Some(String::new());
    chain.chain_id = Some(0);
    chain.wallet_key = Some(String::new());
    chain.identity_registry = Some(String::new());
    chain.reputation_registry = Some(String::new());
    chain.validation_registry = Some(String::new());
    chain.knowledge_registry = Some(String::new());
    chain.agent_registry = Some(String::new());
    chain.bounty_market = Some(String::new());
    chain.deployer = Some(String::new());
    chain.finality_confirmations = Some(0);
    let relay = &mut config.relay;
    relay.url = Some(String::new());
    relay.workspace_name = Some(String::new());
    relay.public_url = Some(String::new());
    config.runner.max_concurrent_tasks = Some(0);
    config.runner.max_concurrent_plans = Some(0);
    config.resources.per_plan_disk_budget_mb = Some(0);
    config.dreams.scheduled_cron = Some(String::new());
    // `role_token_budgets` maps roles to budgets (a dynamic map section).
    config
        .retrieval
        .role_token_budgets
        .insert("_schema_sentinel".to_string(), 0);

    let mut value =
        toml::Value::try_from(config).expect("sentinel RokoConfig must serialize to toml::Value");

    // `Vec` fields that skip serializing when empty and whose elements have
    // required fields. An empty schema array accepts every element.
    for path in [
        "serve.auth.api_keys",
        "serve.auth.jwks_providers",
        "serve.deploy.webhooks",
        "scheduler.cron",
    ] {
        insert_empty_array(&mut value, path);
    }

    // Sections backed by Vec<T> where T lacks Default or conditional
    // structs with `skip_serializing_if` are added to the schema tree so
    // their keys are accepted. Empty arrays accept any element; empty
    // tables accept nested subsections by name.
    if let Some(table) = value.as_table_mut() {
        for key in &["agents", "groups", "repos"] {
            table
                .entry((*key).to_string())
                .or_insert_with(|| toml::Value::Array(Vec::new()));
        }
        // `watcher` is skipped when empty. Add an empty table so
        // `[watcher]` and `[watcher.paths]` are accepted.
        table.entry("watcher".to_string()).or_insert_with(|| {
            let mut watcher_table = toml::map::Map::new();
            watcher_table.insert("paths".to_string(), toml::Value::Array(Vec::new()));
            toml::Value::Table(watcher_table)
        });
        // `tui.effects` is `Option<toml::Value>` and skipped when None.
        // Populate the known effects keys so the schema walker accepts
        // `[tui.effects]` sub-fields. The full `EffectsConfig` type lives
        // in `roko-cli`; we list the known leaf keys here manually.
        if let Some(tui) = table.get_mut("tui").and_then(|v| v.as_table_mut()) {
            tui.entry("effects".to_string()).or_insert_with(|| {
                let mut effects = toml::map::Map::new();
                for key in &[
                    "preset",
                    "screen_postfx",
                    "nerv_viz",
                    "particles",
                    "bloom_enabled",
                    "shadows_enabled",
                    "vfx_enabled",
                    "bloom_intensity",
                    "vignette_intensity",
                ] {
                    effects.insert((*key).to_string(), toml::Value::String(String::new()));
                }
                toml::Value::Table(effects)
            });
        }
    }

    value
}

/// A config section with every field at its serde default, for a sentinel.
///
/// `None` when the type has a field without a serde default; the schema
/// tree then lacks the section, and the guard test names it.
fn from_empty_table<T: serde::de::DeserializeOwned>() -> Option<T> {
    toml::Value::Table(toml::map::Map::new()).try_into().ok()
}

/// Put an empty array at the dotted `path` of the schema tree unless the
/// tree already has a value there.
fn insert_empty_array(tree: &mut toml::Value, path: &str) {
    let Some((parent, leaf)) = path.rsplit_once('.') else {
        return;
    };
    let table = parent
        .split('.')
        .try_fold(tree, |node, key| node.get_mut(key))
        .and_then(toml::Value::as_table_mut);
    if let Some(table) = table {
        table
            .entry(leaf.to_string())
            .or_insert_with(|| toml::Value::Array(Vec::new()));
    }
}

/// Walk the input TOML tree against the schema tree, collecting diagnostics
/// for unknown keys at every nesting level.
fn walk_config_paths(
    input: &toml::Value,
    schema: &toml::Value,
    prefix: &str,
    diagnostics: &mut Vec<ConfigDiagnostic>,
) {
    let Some(input_table) = input.as_table() else {
        return;
    };

    // Check if the current path is a dynamic map section.
    let is_dynamic = is_dynamic_section(prefix);

    if is_dynamic {
        // Keys are user-defined names. Validate each value against the schema
        // value template (the first value in the default, or the schema table
        // itself if it is empty).
        let value_schema = schema.as_table().and_then(|t| t.values().next()).cloned();
        if let Some(ref vs) = value_schema {
            for (key, val) in input_table {
                let child_path = if prefix.is_empty() {
                    key.clone()
                } else {
                    format!("{prefix}.{key}")
                };
                walk_config_paths(val, vs, &child_path, diagnostics);
            }
        }
        // If the schema map is empty (no default value template), accept all
        // keys without further validation.
        return;
    }

    let schema_table = match schema.as_table() {
        Some(t) => t,
        None => return,
    };

    let known_keys: Vec<&str> = schema_table.keys().map(String::as_str).collect();

    for (key, val) in input_table {
        let dotted = if prefix.is_empty() {
            key.clone()
        } else {
            format!("{prefix}.{key}")
        };

        // Check for legacy removed sections first.
        if let Some((_, message)) = LEGACY_REMOVED_SECTIONS
            .iter()
            .find(|(section, _)| *section == dotted)
        {
            diagnostics.push(ConfigDiagnostic {
                key: dotted,
                message: message.to_string(),
            });
            continue;
        }

        if schema_table.contains_key(key) {
            // Known key — check if both sides are tables/arrays and recurse.
            let schema_val = &schema_table[key];
            let child_path = if prefix.is_empty() {
                key.clone()
            } else {
                format!("{prefix}.{key}")
            };

            // For arrays of tables, validate each element against the schema
            // array's element template (first element of the default).
            if let (Some(input_arr), Some(schema_arr)) = (val.as_array(), schema_val.as_array()) {
                if let Some(element_schema) = schema_arr.first() {
                    for (i, element) in input_arr.iter().enumerate() {
                        let elem_path = format!("{child_path}[{i}]");
                        walk_config_paths(element, element_schema, &elem_path, diagnostics);
                    }
                }
                // Empty schema arrays (no template element) accept all entries.
            } else if is_likely_enum_table(schema_val, val) && !is_dynamic_section(&child_path) {
                // Serde-tagged enums serialize as single-key tables (e.g.
                // `{ "Prefix": "..." }`). When the schema has one variant
                // and the input has a different variant, accept it rather
                // than flagging the variant name as unknown.
                //
                // Dynamic map sections (providers, models, etc.) are excluded
                // because their schema sentinel key always differs from the
                // user-defined key, which would falsely trigger this heuristic.
            } else {
                walk_config_paths(val, schema_val, &child_path, diagnostics);
            }
        } else {
            // Unknown key. Suggest nearest match if edit distance is small.
            let suggestion = find_nearest_key(key, &known_keys);
            let msg = match suggestion {
                Some(nearest) => format!(
                    "unknown config key '{dotted}' (did you mean '{prefix_dot}{nearest}'?)",
                    prefix_dot = if prefix.is_empty() {
                        String::new()
                    } else {
                        format!("{prefix}.")
                    }
                ),
                None => format!("unknown config key '{dotted}'"),
            };
            diagnostics.push(ConfigDiagnostic {
                key: dotted,
                message: msg,
            });
        }
    }
}

/// Heuristic: detect serde-tagged enum tables.
///
/// When both the schema and input are tables with exactly one key each, and
/// those keys differ, the values likely represent two variants of a serde
/// externally-tagged enum. Returns `true` so the walker skips recursive
/// validation (because the schema variant differs from the input variant).
fn is_likely_enum_table(schema: &toml::Value, input: &toml::Value) -> bool {
    let (Some(st), Some(it)) = (schema.as_table(), input.as_table()) else {
        return false;
    };
    st.len() == 1 && it.len() == 1 && st.keys().next() != it.keys().next()
}

/// Find the nearest known key by edit distance (Levenshtein).
///
/// Returns `Some(key)` when the distance is at most 2 edits and the key is
/// at least 3 characters long (to avoid spurious suggestions for short keys).
pub fn find_nearest_key<'a>(input: &str, candidates: &[&'a str]) -> Option<&'a str> {
    if input.len() < 3 {
        return None;
    }
    let mut best: Option<(&str, usize)> = None;
    for &candidate in candidates {
        let dist = levenshtein_distance(input, candidate);
        if dist > 0 && dist <= 2 && best.as_ref().map_or(true, |b| dist < b.1) {
            best = Some((candidate, dist));
        }
    }
    best.map(|(key, _)| key)
}

/// Minimal Levenshtein distance for typo detection.
fn levenshtein_distance(a: &str, b: &str) -> usize {
    let a_chars: Vec<char> = a.chars().collect();
    let b_chars: Vec<char> = b.chars().collect();
    let (m, n) = (a_chars.len(), b_chars.len());

    // Early exit for large differences.
    if m.abs_diff(n) > 2 {
        return m.abs_diff(n);
    }

    let mut prev = (0..=n).collect::<Vec<_>>();
    let mut curr = vec![0; n + 1];

    for i in 1..=m {
        curr[0] = i;
        for j in 1..=n {
            let cost = if a_chars[i - 1] == b_chars[j - 1] {
                0
            } else {
                1
            };
            curr[j] = (prev[j] + 1).min(curr[j - 1] + 1).min(prev[j - 1] + cost);
        }
        std::mem::swap(&mut prev, &mut curr);
    }
    prev[n]
}

/// Backward-compatible wrapper that `parse_from_resolved_path` calls.
///
/// Uses [`validate_known_config_paths`] for full nested validation.
fn unknown_field_diagnostics(value: &toml::Value) -> Vec<ConfigDiagnostic> {
    validate_known_config_paths(value)
}

/// Serialize the effective (fully-resolved) config as TOML.
///
/// Useful for workspace creation (write resolved config, not blind copy)
/// and debugging (`roko config show --effective`).
pub fn serialize_effective(config: &RokoConfig) -> Result<String, toml::ser::Error> {
    toml::to_string_pretty(config)
}

/// The redaction marker replacing secret values in serialized output.
pub const REDACTED_MARKER: &str = "[REDACTED]";

/// Key name fragments that identify secret-bearing fields.
///
/// Any non-empty TOML leaf whose key contains one of these substrings
/// (case-insensitive) is replaced with [`REDACTED_MARKER`], except `_env`
/// metadata fields. Non-secret fields (model names, URLs without credentials,
/// booleans, numerics) are left intact.
const SECRET_KEY_FRAGMENTS: &[&str] = &[
    "api_key",
    "secret",
    "token",
    "password",
    "credential",
    "authorization",
    "auth_token",
    "private_key",
    "wallet_key",
    "passphrase",
];

/// Serialize the effective config as TOML with secret values redacted.
///
/// Identical to [`serialize_effective`] except that leaf values whose keys
/// match [`SECRET_KEY_FRAGMENTS`] are replaced with `[REDACTED]` before the
/// TOML is emitted.  This is the function used by `roko config show --effective`
/// so that resolved secrets are never leaked to stdout or logs.
pub fn serialize_effective_redacted(config: &RokoConfig) -> Result<String, toml::ser::Error> {
    let mut value = toml::Value::try_from(config)?;
    redact_secrets_in_toml(&mut value);
    Ok(toml::to_string_pretty(&value)
        .expect("re-serialization of a value that was already valid TOML cannot fail"))
}

/// Convenience wrapper: parse a raw TOML string, redact secret fields, and
/// return the re-serialized TOML. Useful in tests and CLI output paths that
/// operate on raw config text rather than a typed `RokoConfig`.
pub fn redact_secrets_in_toml_str(raw: &str) -> String {
    let mut value: toml::Value = raw
        .parse()
        .unwrap_or_else(|_| toml::Value::Table(Default::default()));
    redact_secrets_in_toml(&mut value);
    toml::to_string_pretty(&value).unwrap_or_else(|_| raw.to_string())
}

/// Walk a [`toml::Value`] tree in-place, replacing secret-like string leaves
/// with [`REDACTED_MARKER`].
///
/// A value is considered secret when its parent key contains any of the
/// substrings listed in [`SECRET_KEY_FRAGMENTS`] (compared case-insensitively).
/// Environment-reference fields ending in `_env` are metadata and remain
/// visible. Empty strings also remain unchanged so output still distinguishes
/// an unconfigured secret from a configured one. Tables under `extra_headers`
/// treat every non-empty string as secret regardless of key name, because
/// header values frequently carry bearer tokens and API keys.
fn redact_secrets_in_toml(value: &mut toml::Value) {
    match value {
        toml::Value::Table(table) => {
            for (key, child) in table.iter_mut() {
                redact_secrets_in_toml_keyed(key, child);
            }
        }
        toml::Value::Array(arr) => {
            for item in arr.iter_mut() {
                redact_secrets_in_toml(item);
            }
        }
        _ => {}
    }
}

/// Whether a config key names a secret-bearing field: it contains one of
/// [`SECRET_KEY_FRAGMENTS`] and is not an `*_env` field, which holds an
/// environment variable's name rather than a secret.
fn is_secret_key(key: &str) -> bool {
    let key = key.to_ascii_lowercase();
    !key.ends_with("_env")
        && SECRET_KEY_FRAGMENTS
            .iter()
            .any(|fragment| key.contains(fragment))
}

/// The dotted paths of the secrets in a config tree.
///
/// A secret is a non-empty string in a secret-named field ([`is_secret_key`])
/// or in provider `extra_headers`, or an `agent.env` value whose name looks
/// like a credential. A `${VAR}` reference is not a secret, and the loader
/// expands it in each of those places; nor is an `extra_headers` `*_file`
/// path.
#[must_use]
pub fn secret_fields(value: &toml::Value) -> Vec<String> {
    let mut fields: Vec<String> = Vec::new();
    visit_secrets(value, "", &mut |field, _| {
        // The strings of one array share a field.
        if fields.last().is_none_or(|last| last != field) {
            fields.push(field.to_string());
        }
    });
    fields
}

/// Call `found` with the dotted field and the value of each secret under
/// `value`, at dotted `path` ([`secret_fields`]).
fn visit_secrets(value: &toml::Value, path: &str, found: &mut dyn FnMut(&str, &str)) {
    let Some(table) = value.as_table() else {
        return;
    };
    for (key, child) in table {
        let child_path = if path.is_empty() {
            key.clone()
        } else {
            format!("{path}.{key}")
        };
        match child {
            toml::Value::String(text) if is_secret_key(key) && is_literal_secret(text) => {
                found(&child_path, text);
            }
            // An array of strings under a secret name; tables in it, such as
            // the hashed `serve.auth.api_keys`, hold no secret.
            toml::Value::Array(items) if is_secret_key(key) => {
                for text in items.iter().filter_map(toml::Value::as_str) {
                    if is_literal_secret(text) {
                        found(&child_path, text);
                    }
                }
            }
            toml::Value::Table(headers) if key.eq_ignore_ascii_case("extra_headers") => {
                for (name, header) in headers {
                    if !name.to_ascii_lowercase().ends_with("_file")
                        && let Some(text) = header.as_str()
                        && is_literal_secret(text)
                    {
                        found(&format!("{child_path}.{name}"), text);
                    }
                }
            }
            toml::Value::Array(pairs) if child_path == "agent.env" => {
                for pair in pairs {
                    if let Some([toml::Value::String(name), toml::Value::String(text)]) =
                        pair.as_array().map(Vec::as_slice)
                        && crate::child_env::is_secret_env_name(name)
                        && is_literal_secret(text)
                    {
                        found(&format!("{child_path}.{name}"), text);
                    }
                }
            }
            toml::Value::Table(_) => visit_secrets(child, &child_path, found),
            toml::Value::Array(items) => {
                for item in items {
                    visit_secrets(item, &child_path, found);
                }
            }
            _ => {}
        }
    }
}

/// The secrets a resolved config holds, as (field, value) pairs: the
/// [`secret_fields`] of its effective values, with references expanded and
/// file secrets read. A header value such as `Bearer <token>` adds the
/// credential alone as well, which a log may show without the scheme.
fn config_secret_values(config: &RokoConfig) -> Vec<(String, String)> {
    let Ok(tree) = toml::Value::try_from(config) else {
        return Vec::new();
    };
    let mut secrets = Vec::new();
    visit_secrets(&tree, "", &mut |field, value| {
        secrets.push((field.to_string(), value.to_string()));
        if field.contains(".extra_headers.")
            && let Some((scheme, credential)) = value.split_once(' ')
            && scheme.chars().all(|c| c.is_ascii_alphabetic())
        {
            secrets.push((field.to_string(), credential.to_string()));
        }
    });
    secrets
}

/// Whether a config string is a literal secret: not empty, and not an
/// environment reference such as `${OPENAI_API_KEY}`.
fn is_literal_secret(text: &str) -> bool {
    !text.is_empty() && !text.contains("${")
}

/// Whether a config tree holds a secret ([`secret_fields`]).
#[must_use]
pub fn holds_secrets(value: &toml::Value) -> bool {
    !secret_fields(value).is_empty()
}

/// Refuse a config file that agents may read when it holds a secret.
///
/// Any file but a key file such as `~/.roko/config.toml` counts: a grep of
/// the project would show the secret to agents. The error names each field
/// and where it belongs instead.
///
/// # Errors
///
/// [`LoadConfigError::SecretInConfig`] when `value`, the contents of
/// `path`, holds a secret ([`secret_fields`]).
pub fn refuse_readable_secrets(path: &Path, value: &toml::Value) -> Result<(), LoadConfigError> {
    if crate::child_env::is_key_file(path) {
        return Ok(());
    }
    let fields = secret_fields(value);
    if fields.is_empty() {
        return Ok(());
    }
    let fields = fields
        .iter()
        .map(|field| match env_override_name(field) {
            Some(variable)
                if !field.contains(".extra_headers.") && !field.starts_with("agent.env.") =>
            {
                format!("{field} (set {variable} in .roko/.env, or give a ${{VAR}} reference)")
            }
            _ => format!("{field} (give a ${{VAR}} reference or set it in the environment)"),
        })
        .collect::<Vec<_>>()
        .join(", ");
    Err(LoadConfigError::SecretInConfig {
        path: path.to_path_buf(),
        fields,
    })
}

/// Whether config text holds a secret ([`holds_secrets`]). Text that does
/// not parse is read line by line instead: a `key = "value"` line whose key
/// names a secret field counts.
#[must_use]
pub fn config_text_holds_secrets(text: &str) -> bool {
    let Ok(value) = text.parse::<toml::Value>() else {
        return text.lines().any(|line| {
            line.split_once('=').is_some_and(|(key, value)| {
                let key = key.trim().trim_matches(['"', '\'']);
                let value = value.trim_start();
                is_secret_key(key.rsplit('.').next().unwrap_or(key))
                    && (value.starts_with('"') || value.starts_with('\''))
                    && !(value.starts_with("\"\"") || value.starts_with("''"))
                    && !value.contains("${")
            })
        });
    };
    holds_secrets(&value)
}

/// Expand the `${VAR}` references in a config's secret fields from the
/// process environment.
///
/// A secret field may name the variable that holds its secret instead of
/// holding it (`serve.auth.api_key = "${ROKO_SERVE_KEY}"`), which is how a
/// file agents can read keeps a secret out ([`secret_fields`] exempts such a
/// reference). Provider fields, headers included, are expanded with
/// [`RokoConfig::interpolate_env_vars`].
///
/// # Errors
///
/// [`LoadConfigError::SecretReference`] when a reference names a variable
/// that is not set.
fn expand_secret_references(config: &mut RokoConfig) -> Result<(), LoadConfigError> {
    expand_secret_references_with(config, &|name| std::env::var(name).ok())
}

fn expand_secret_references_with(
    config: &mut RokoConfig,
    env: &dyn Fn(&str) -> Option<String>,
) -> Result<(), LoadConfigError> {
    let mut tree = match toml::Value::try_from(&*config) {
        Ok(tree) => tree,
        Err(error) => {
            tracing::warn!(%error, "cannot serialize the config to expand secret references");
            return Ok(());
        }
    };
    let mut expanded = Vec::new();
    expand_references_in(&mut tree, "", env, &mut expanded)?;
    if expanded.is_empty() {
        return Ok(());
    }
    *config = tree
        .try_into()
        .map_err(|error| LoadConfigError::SecretReference {
            field: expanded.join(", "),
            reason: format!("the expanded config does not load: {error}"),
        })?;
    Ok(())
}

/// Expand the references in the secret fields under `value`, at dotted
/// `path`, the fields [`secret_fields`] reads, and add each to `expanded`.
fn expand_references_in(
    value: &mut toml::Value,
    path: &str,
    env: &dyn Fn(&str) -> Option<String>,
    expanded: &mut Vec<String>,
) -> Result<(), LoadConfigError> {
    let Some(table) = value.as_table_mut() else {
        return Ok(());
    };
    for (key, child) in table.iter_mut() {
        let child_path = if path.is_empty() {
            key.clone()
        } else {
            format!("{path}.{key}")
        };
        match child {
            toml::Value::String(text) if is_secret_key(key) => {
                expand_reference(text, &child_path, env, expanded)?;
            }
            toml::Value::Array(items) if is_secret_key(key) => {
                for item in items {
                    if let toml::Value::String(text) = item {
                        expand_reference(text, &child_path, env, expanded)?;
                    }
                }
            }
            // Expanded with the other provider fields.
            toml::Value::Table(_) if key.eq_ignore_ascii_case("extra_headers") => {}
            toml::Value::Array(pairs) if child_path == "agent.env" => {
                for pair in pairs {
                    if let Some([toml::Value::String(name), toml::Value::String(text)]) =
                        pair.as_array_mut().map(Vec::as_mut_slice)
                        && crate::child_env::is_secret_env_name(name)
                    {
                        let field = format!("{child_path}.{name}");
                        expand_reference(text, &field, env, expanded)?;
                    }
                }
            }
            toml::Value::Table(_) => expand_references_in(child, &child_path, env, expanded)?,
            toml::Value::Array(items) => {
                for item in items {
                    expand_references_in(item, &child_path, env, expanded)?;
                }
            }
            _ => {}
        }
    }
    Ok(())
}

/// Replace each `${VAR}` in `text`, secret field `field`, with the value of
/// `VAR`, and add the field to `expanded`. A `${` that starts no reference
/// fails without showing the text, which may be a secret.
fn expand_reference(
    text: &mut String,
    field: &str,
    env: &dyn Fn(&str) -> Option<String>,
    expanded: &mut Vec<String>,
) -> Result<(), LoadConfigError> {
    if !text.contains("${") {
        return Ok(());
    }
    let unexpandable = |reason: String| LoadConfigError::SecretReference {
        field: field.to_string(),
        reason,
    };
    let mut value = String::with_capacity(text.len());
    let mut rest = text.as_str();
    while let Some(start) = rest.find("${") {
        value.push_str(&rest[..start]);
        let Some((name, after)) = rest[start + 2..]
            .split_once('}')
            .filter(|(name, _)| is_env_name(name))
        else {
            return Err(unexpandable(
                "holds a `${` that starts no `${VAR}` reference".to_string(),
            ));
        };
        let resolved = env(name).ok_or_else(|| {
            unexpandable(format!(
                "`${{{name}}}` is not set; set {name}, for example in .roko/.env"
            ))
        })?;
        value.push_str(&resolved);
        rest = after;
    }
    value.push_str(rest);
    *text = value;
    expanded.push(field.to_string());
    Ok(())
}

/// Whether `name` can name an environment variable in a `${VAR}` reference:
/// ASCII letters, digits and underscores, not starting with a digit.
fn is_env_name(name: &str) -> bool {
    name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
        && name.chars().next().is_some_and(|c| !c.is_ascii_digit())
}

/// Recursive inner helper that knows the current key name.
fn redact_secrets_in_toml_keyed(key: &str, value: &mut toml::Value) {
    let secret_key = is_secret_key(key);
    let is_extra_headers = key.eq_ignore_ascii_case("extra_headers");

    match value {
        toml::Value::String(secret) if secret_key && !secret.is_empty() => {
            *value = toml::Value::String(REDACTED_MARKER.to_string());
        }
        toml::Value::Table(table) if is_extra_headers => {
            // Redact all non-empty string values inside extra_headers.
            for (_k, child) in table.iter_mut() {
                if child.as_str().is_some_and(|secret| !secret.is_empty()) {
                    *child = toml::Value::String(REDACTED_MARKER.to_string());
                }
            }
        }
        toml::Value::Table(table) => {
            for (child_key, child_val) in table.iter_mut() {
                redact_secrets_in_toml_keyed(child_key, child_val);
            }
        }
        toml::Value::Array(arr) => {
            for item in arr.iter_mut() {
                if secret_key {
                    if item.as_str().is_some_and(|secret| !secret.is_empty()) {
                        *item = toml::Value::String(REDACTED_MARKER.to_string());
                    }
                } else if !(key == "env" && redact_env_pair(item)) {
                    redact_secrets_in_toml(item);
                }
            }
        }
        _ => {}
    }
}

/// Redact an `agent.env` pair `[NAME, value]` whose name looks like a
/// credential: its value is a secret, or a `${VAR}` reference expanded to
/// one. False when `item` is not such a pair.
fn redact_env_pair(item: &mut toml::Value) -> bool {
    let Some([toml::Value::String(name), toml::Value::String(value)]) =
        item.as_array_mut().map(Vec::as_mut_slice)
    else {
        return false;
    };
    if crate::child_env::is_secret_env_name(name) && !value.is_empty() {
        *value = REDACTED_MARKER.to_string();
    }
    true
}

// ─── Path discovery ─────────────────────────────────────────────────────

/// Find the config file to load. Checks, in order:
///
/// 1. `ROKO_CONFIG` env var (explicit path override)
/// 2. Ancestor walk from `workdir` (find nearest `roko.toml`)
///
/// Returns `None` if no config file is found (defaults will be used).
fn find_config_path(workdir: &Path) -> Option<PathBuf> {
    // 1. ROKO_CONFIG env var takes precedence.
    if let Ok(env_path) = std::env::var("ROKO_CONFIG") {
        let p = PathBuf::from(&env_path);
        if p.is_file() {
            return Some(p);
        }
        tracing::warn!(
            path = %env_path,
            "ROKO_CONFIG env var set but file not found; falling back to discovery"
        );
    }

    // 2. Ancestor walk from workdir (also checks workdir itself).
    discover_project_config(workdir)
}

/// Walk up from `start` looking for `roko.toml`. Returns the first hit.
#[must_use]
pub fn discover_project_config(start: &Path) -> Option<PathBuf> {
    let mut cur = start
        .canonicalize()
        .ok()
        .unwrap_or_else(|| start.to_path_buf());
    loop {
        let candidate = cur.join("roko.toml");
        if candidate.is_file() {
            return Some(candidate);
        }
        if !cur.pop() {
            return None;
        }
    }
}

/// Canonical global config path: `~/.roko/config.toml`, with legacy
/// `$XDG_CONFIG_HOME/roko/config.toml` fallback.
///
/// Returns `None` when the user's home directory cannot be determined
/// (`HOME` and `USERPROFILE` are both unset). Callers should treat
/// `None` as "no global config available" rather than silently falling
/// back to the working directory.
#[must_use]
pub fn global_config_path() -> Option<PathBuf> {
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .ok()?;
    if home.is_empty() {
        return None;
    }

    let canonical = PathBuf::from(&home).join(".roko").join("config.toml");

    if canonical.exists() {
        return Some(canonical);
    }

    // Legacy: $XDG_CONFIG_HOME/roko/config.toml or ~/.config/roko/config.toml
    let legacy = if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
        if !xdg.is_empty() {
            PathBuf::from(xdg).join("roko").join("config.toml")
        } else {
            PathBuf::from(&home)
                .join(".config")
                .join("roko")
                .join("config.toml")
        }
    } else {
        PathBuf::from(&home)
            .join(".config")
            .join("roko")
            .join("config.toml")
    };

    if legacy.exists() {
        return Some(legacy);
    }

    // Neither exists — return canonical for new installs.
    Some(canonical)
}

// ─── Internal helpers ───────────────────────────────────────────────────

/// Merge providers, models, and agent defaults from the global config file.
///
/// Project providers take precedence by key. Project models take precedence by
/// both key and provider-facing slug: a local model deliberately claiming a
/// slug shadows a differently-keyed global model with the same slug. This
/// keeps the merged registry consistent with dispatch validation, where a slug
/// may have only one owner.
pub fn merge_global_into(config: &mut RokoConfig) -> Result<(), super::LoadConfigError> {
    let Some(global_path) = global_config_path() else {
        return Ok(());
    };
    if !global_path.exists() {
        return Ok(());
    }

    let text = match std::fs::read_to_string(&global_path) {
        Ok(t) => t,
        Err(source) => {
            tracing::error!(
                path = %global_path.display(),
                error = %source,
                "global config file exists but could not be read"
            );
            return Err(super::LoadConfigError::GlobalConfigRead {
                path: global_path.clone(),
                source,
            });
        }
    };

    // The legacy ~/.config/roko/config.toml is no key file, so it may not
    // hold a secret any more than roko.toml may.
    if let Ok(value) = text.parse::<toml::Value>() {
        refuse_readable_secrets(&global_path, &value)?;
    }

    let global = match deserialize_migrated_toml(&text) {
        Ok(g) => g,
        Err(source) => {
            tracing::error!(
                path = %global_path.display(),
                error = %source,
                "global config file exists but could not be parsed"
            );
            return Err(super::LoadConfigError::GlobalConfigParse {
                path: global_path.clone(),
                detail: source,
            });
        }
    };

    merge_global_config_into(config, global);
    Ok(())
}

fn deserialize_migrated_toml(text: &str) -> Result<RokoConfig, String> {
    let mut value = text
        .parse::<toml::Value>()
        .map_err(|error| error.to_string())?;
    let migrator = ConfigMigrator::new();
    let report = migrator.migrate(&mut value);
    if report.to_version < migrator.target_version {
        return Err(report.warnings.join("; "));
    }
    // Strip unknown fields from sub-tables so that global configs with
    // forward-looking or custom fields do not fail `deny_unknown_fields`
    // validation on sub-structs. The schema tree knows every valid key at
    // every nesting level; anything not in the tree is silently dropped.
    let schema = build_schema_tree();
    strip_unknown_fields(&mut value, &schema, "");
    value
        .try_into::<RokoConfig>()
        .map_err(|error| error.to_string())
}

/// Recursively remove keys from `input` that are absent in `schema`.
///
/// Dynamic map sections (providers, models, etc.) are walked using the
/// sentinel template value so user-defined map keys are preserved. Inside a
/// provider or model entry ([`STRICT_ENTRY_SECTIONS`]) extra fields are
/// stripped too, as they are in every fixed section.
fn strip_unknown_fields(input: &mut toml::Value, schema: &toml::Value, prefix: &str) {
    let Some(input_table) = input.as_table_mut() else {
        return;
    };

    let is_dynamic = is_dynamic_section(prefix);

    if is_dynamic {
        let value_schema = schema.as_table().and_then(|t| t.values().next()).cloned();
        if let Some(ref vs) = value_schema {
            let strict = STRICT_ENTRY_SECTIONS.contains(&prefix);
            for (key, val) in input_table.iter_mut() {
                // A strict section's entry is a plain table below the section
                // (`providers.<name>`), stripped against the template.
                let entry_prefix = if strict {
                    format!("{prefix}.{key}")
                } else {
                    prefix.to_string()
                };
                strip_unknown_fields(val, vs, &entry_prefix);
            }
        }
        return;
    }

    let Some(schema_table) = schema.as_table() else {
        return;
    };

    // Collect keys to remove (cannot mutate while iterating).
    let to_remove: Vec<String> = input_table
        .keys()
        .filter(|k| !schema_table.contains_key(k.as_str()))
        .cloned()
        .collect();

    for key in &to_remove {
        tracing::debug!(
            prefix = prefix,
            key = key.as_str(),
            "stripping unknown field from global config"
        );
        input_table.remove(key);
    }

    // Recurse into surviving sub-tables.
    for (key, val) in input_table.iter_mut() {
        if let Some(schema_val) = schema_table.get(key) {
            let child_prefix = if prefix.is_empty() {
                key.clone()
            } else {
                format!("{prefix}.{key}")
            };
            strip_unknown_fields(val, schema_val, &child_prefix);
        }
    }
}

/// Merge an already-parsed global layer beneath a project config.
///
/// This is the deterministic counterpart to [`merge_global_into`] for callers
/// that already loaded the global layer (and for tests that must not depend on
/// the process-wide `HOME`). Project entries always win. Model identity has two
/// user-visible forms -- the table key and the provider-facing slug -- so
/// either collision shadows the global entry. Duplicate slugs originating
/// entirely within one layer remain present and are still rejected by
/// [`normalize_and_validate_dispatch_models`].
pub fn merge_global_config_into(config: &mut RokoConfig, global: RokoConfig) {
    // -- Providers (project wins by stable config key) --
    for (name, provider) in global.providers {
        config.providers.entry(name).or_insert(provider);
    }

    // Capture the project layer before inserting anything from the global
    // layer. This is intentionally not updated during the loop: two duplicate
    // slugs in the global file must survive the merge so dispatch validation
    // can report the invalid global config instead of silently picking one.
    let mut project_slug_owners = std::collections::HashMap::<String, Vec<String>>::new();
    for (key, model) in &config.models {
        project_slug_owners
            .entry(model.slug.trim().to_string())
            .or_default()
            .push(key.clone());
    }
    for owners in project_slug_owners.values_mut() {
        owners.sort_unstable();
    }

    // -- Models (project wins by key or normalized provider slug) --
    let mut shadowed_global_model_keys = std::collections::HashMap::<String, String>::new();
    for (name, model) in global.models {
        if config.models.contains_key(&name) {
            tracing::debug!(model = %name, "project model shadows same-key global model");
            continue;
        }

        let normalized_slug = model.slug.trim();
        if let Some(project_keys) = project_slug_owners.get(normalized_slug) {
            if let [project_key] = project_keys.as_slice() {
                shadowed_global_model_keys.insert(name.clone(), project_key.clone());
            }
            tracing::debug!(
                global_model = %name,
                slug = %normalized_slug,
                project_models = %project_keys.join(", "),
                "project model slug shadows global model"
            );
            continue;
        }

        config.models.insert(name, model);
    }

    // -- Agent defaults (fill gaps only) --
    if config.agent.default_model.is_empty() && !global.agent.default_model.is_empty() {
        tracing::debug!(model = %global.agent.default_model, "merged global agent.default_model");
        config.agent.default_model = shadowed_global_model_keys
            .get(global.agent.default_model.trim())
            .cloned()
            .unwrap_or(global.agent.default_model);
    }
    if config.agent.default_backend.is_empty() && !global.agent.default_backend.is_empty() {
        tracing::debug!(backend = %global.agent.default_backend, "merged global agent.default_backend");
        config.agent.default_backend = global.agent.default_backend;
    }
    if config.agent.default_effort.is_empty() && !global.agent.default_effort.is_empty() {
        tracing::debug!(effort = %global.agent.default_effort, "merged global agent.default_effort");
        config.agent.default_effort = global.agent.default_effort.clone();
    }

    // -- Budget defaults (fill when project uses default, i.e. 0.0 = unlimited) --
    if config.budget.max_plan_usd == 0.0 && global.budget.max_plan_usd != 0.0 {
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

    // Post-merge: validate model->provider references now that both layers are present.
    for (model_key, profile) in &config.models {
        if !config.providers.contains_key(&profile.provider) {
            tracing::warn!(
                model = %model_key,
                provider = %profile.provider,
                "model references provider '{}' which is missing after global+project merge",
                profile.provider,
            );
        }
    }
}

#[cfg(test)]
#[allow(unsafe_code)]
mod tests {
    use super::*;

    #[test]
    fn config_migrator_applies_v1_to_v2_and_renames_legacy_fields() {
        let mut value = r#"
schema_version = 1
config_version = 1
[agent]
model = "legacy-model"
effort = "high"
[budget]
max_session_usd = 25.0
max_agent_usd = 2.0
"#
        .parse::<toml::Value>()
        .expect("parse v1 value");

        let report = ConfigMigrator::new().migrate(&mut value);
        assert_eq!(report.from_version, 1);
        assert_eq!(report.to_version, 2);
        assert_eq!(report.steps_applied.len(), 1);
        assert_eq!(value["schema_version"].as_integer(), Some(2));
        assert_eq!(value["config_version"].as_integer(), Some(2));
        assert_eq!(
            value["agent"]["default_model"].as_str(),
            Some("legacy-model")
        );
        assert!(value["agent"].get("model").is_none());
        assert_eq!(value["budget"]["max_plan_usd"].as_float(), Some(25.0));
    }

    #[test]
    fn config_migrator_skips_current_schema() {
        let mut value = "schema_version = 2\nconfig_version = 2\n"
            .parse::<toml::Value>()
            .expect("parse current value");

        let report = ConfigMigrator::new().migrate(&mut value);
        assert_eq!(report.from_version, 2);
        assert_eq!(report.to_version, 2);
        assert!(report.steps_applied.is_empty());
    }

    #[test]
    fn config_migrator_defaults_missing_version_to_v1() {
        let mut value = "[agent]\nmodel = 'legacy'\n"
            .parse::<toml::Value>()
            .expect("parse unversioned value");

        let report = ConfigMigrator::new().migrate(&mut value);
        assert_eq!(report.from_version, 1);
        assert_eq!(report.to_version, 2);
        assert_eq!(value["agent"]["default_model"].as_str(), Some("legacy"));
    }

    #[test]
    fn loader_migrates_before_deserializing() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(
            dir.path().join("roko.toml"),
            "schema_version = 1\nconfig_version = 1\n[agent]\nmodel = 'migrated-model'\n",
        )
        .expect("write config");

        let loaded = load_config_validated_with_options(
            dir.path(),
            &LoadOptions {
                merge_global: false,
                apply_env_overrides: false,
                apply_hierarchical_env: false,
                strict_validation: false,
            },
        )
        .expect("load migrated config");
        assert_eq!(loaded.config().agent.default_model, "migrated-model");
        assert_eq!(loaded.config().schema_version, 2);
        assert!(
            loaded
                .provenance()
                .iter()
                .any(|entry| entry.source == ConfigSource::Migration)
        );
    }

    #[test]
    fn loader_rejects_error_invariant_after_merge() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(
            dir.path().join("roko.toml"),
            "schema_version = 2\nconfig_version = 2\n[budget]\nmax_plan_usd = 1.0\nmax_turn_usd = 2.0\n",
        )
        .expect("write config");

        let error = load_config_with_options(
            dir.path(),
            &LoadOptions {
                merge_global: false,
                apply_env_overrides: false,
                apply_hierarchical_env: false,
                strict_validation: false,
            },
        )
        .expect_err("budget invariant must reject load");
        assert!(matches!(
            error,
            LoadConfigError::InvariantViolation {
                invariant_id: 1,
                ..
            }
        ));
    }

    #[test]
    fn loader_allows_warning_invariant_and_retains_diagnostic() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(
            dir.path().join("roko.toml"),
            "schema_version = 2\nconfig_version = 2\n[agent]\ncontext_limit_k = 2\n",
        )
        .expect("write config");

        let loaded = load_config_validated_with_options(
            dir.path(),
            &LoadOptions {
                merge_global: false,
                apply_env_overrides: false,
                apply_hierarchical_env: false,
                strict_validation: false,
            },
        )
        .expect("warning must not reject load");
        assert!(
            loaded
                .diagnostics()
                .iter()
                .any(|diagnostic| diagnostic.key.starts_with("invariant.5."))
        );
    }

    #[test]
    fn validated_loader_warns_for_unknown_top_level_field() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(
            dir.path().join("roko.toml"),
            "schema_version = 2\nconfig_version = 2\nfuture_section = 'ignored'\n",
        )
        .expect("write config");

        let loaded = load_config_validated_with_options(
            dir.path(),
            &LoadOptions {
                merge_global: false,
                apply_env_overrides: false,
                apply_hierarchical_env: false,
                strict_validation: false,
            },
        )
        .expect("unknown field must remain forward compatible");
        assert!(loaded.diagnostics().iter().any(|diagnostic| {
            diagnostic.key == "future_section" && diagnostic.message.contains("unknown")
        }));
    }

    /// bug-ab8118: a misspelled key inside a `[providers.*]` or `[models.*]`
    /// entry is diagnosed and stripped like one in any other section, so the
    /// load succeeds and keeps the entry's other keys.
    #[test]
    fn a_typo_inside_a_provider_or_model_entry_is_handled_like_any_other() {
        const TEXT: &str = r#"schema_version = 2
config_version = 2

[agent]
default_efort = "high"

[providers.local]
kind = "openai_compat"
base_ulr = "http://localhost:11434/v1"
api_key_env = "LOCAL_KEY"

[models.local-model]
provider = "local"
slug = "llama3"
contxt_window = 8192
"#;
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join("roko.toml"), TEXT).expect("write config");
        let options = LoadOptions {
            merge_global: false,
            apply_env_overrides: false,
            apply_hierarchical_env: false,
            strict_validation: false,
        };

        let loaded = load_config_validated_with_options(dir.path(), &options)
            .expect("a typo in any section leaves the load working");

        let unknown: Vec<&str> = loaded
            .diagnostics()
            .iter()
            .filter(|diagnostic| diagnostic.message.contains("unknown"))
            .map(|diagnostic| diagnostic.key.as_str())
            .collect();
        for key in [
            "agent.default_efort",
            "providers.local.base_ulr",
            "models.local-model.contxt_window",
        ] {
            assert!(unknown.contains(&key), "{key}: {unknown:?}");
        }
        let config = loaded.config();
        assert_eq!(
            config.providers["local"].api_key_env.as_deref(),
            Some("LOCAL_KEY")
        );
        assert_eq!(config.models["local-model"].slug, "llama3");
        // The global config's loader strips the same keys.
        let global = deserialize_migrated_toml(TEXT).expect("the global loader strips them too");
        assert!(global.providers.contains_key("local"));
    }

    #[test]
    fn validated_loader_records_default_field_provenance_without_file() {
        let dir = tempfile::tempdir().expect("tempdir");
        let loaded = load_config_validated_with_options(
            dir.path(),
            &LoadOptions {
                merge_global: false,
                apply_env_overrides: false,
                apply_hierarchical_env: false,
                strict_validation: false,
            },
        )
        .expect("load defaults");
        assert!(loaded.merge_context.field_provenance.iter().any(|field| {
            field.key == "roko.toml" && field.value_source == ConfigSource::Default
        }));
    }

    #[test]
    fn load_without_merge_returns_default_when_no_config() {
        let dir = tempfile::tempdir().unwrap();
        let opts = LoadOptions {
            merge_global: false,
            apply_env_overrides: false,
            apply_hierarchical_env: false,
            strict_validation: false,
        };
        let config = load_config_with_options(dir.path(), &opts).unwrap();
        assert_eq!(config, RokoConfig::default());
    }

    #[test]
    fn load_unified_reads_roko_toml() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("roko.toml"),
            r#"
config_version = 2
schema_version = 2

[providers.test-prov]
kind = "openai_compat"
base_url = "https://example.com/v1"
"#,
        )
        .unwrap();

        let config = load_config_unified(dir.path()).unwrap();
        assert!(config.providers.contains_key("test-prov"));
    }

    #[test]
    fn in_memory_source_resolution_matches_explicit_file_loading() {
        let dir = tempfile::tempdir().expect("tempdir");
        let secret_path = dir.path().join("authorization.secret");
        std::fs::write(&secret_path, "resolved-secret\n").expect("write secret");
        let config_path = dir.path().join("roko.toml");
        let source_text = format!(
            r#"config_version = 2

[providers.test]
kind = "openai_compat"
base_url = "https://source.invalid/v1"

[providers.test.extra_headers]
authorization_file = "{}"
"#,
            secret_path.display()
        );
        std::fs::write(&config_path, &source_text).expect("write config");
        let source: RokoConfig = toml::from_str(&source_text).expect("parse source");
        let retained_source = source.clone();
        let opts = LoadOptions {
            merge_global: false,
            apply_env_overrides: false,
            apply_hierarchical_env: false,
            strict_validation: false,
        };

        let from_file = load_config_file(&config_path, &opts).expect("load file");
        let from_memory =
            resolve_config_source(source, &config_path, &opts).expect("resolve source");

        assert_eq!(from_memory, from_file);
        assert_eq!(
            from_memory.providers["test"]
                .extra_headers
                .as_ref()
                .and_then(|headers| headers.get("authorization"))
                .map(String::as_str),
            Some("resolved-secret")
        );
        assert!(
            retained_source.providers["test"]
                .extra_headers
                .as_ref()
                .is_some_and(|headers| headers.contains_key("authorization_file")),
            "resolving an owned clone must not mutate the retained source"
        );
    }

    #[test]
    fn load_validated_rejects_orphaned_models_as_invariant_error() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("roko.toml"),
            r#"
config_version = 2
schema_version = 2

[models.orphan]
provider = "nonexistent"
slug = "orphan-v1"
context_window = 4096

[agent]
default_model = "orphan"
"#,
        )
        .unwrap();

        let error = load_config_validated(dir.path()).expect_err("orphan must be rejected");
        assert!(matches!(
            error,
            LoadConfigError::InvariantViolation {
                invariant_id: 3,
                ..
            }
        ));
    }

    #[test]
    fn load_validated_rejects_duplicate_slugs() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("roko.toml"),
            r#"
config_version = 2
schema_version = 2

[providers.prov]
kind = "openai_compat"
base_url = "https://example.com/v1"

[models.model-a]
provider = "prov"
slug = "same-slug"
context_window = 4096

[models.model-b]
provider = "prov"
slug = "same-slug"
context_window = 4096

[agent]
default_model = "model-a"
"#,
        )
        .unwrap();

        let mut config = load_config_with_options(
            dir.path(),
            &LoadOptions {
                merge_global: false,
                apply_env_overrides: false,
                apply_hierarchical_env: false,
                strict_validation: false,
            },
        )
        .unwrap();
        let error = normalize_and_validate_dispatch_models(&mut config).unwrap_err();
        assert!(matches!(error, LoadConfigError::AmbiguousModelSlug { .. }));
        assert!(error.to_string().contains("model-a, model-b"));
    }

    #[test]
    fn model_slug_alias_is_normalized_to_canonical_key() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("roko.toml"),
            r#"
[providers.prov]
kind = "openai_compat"
base_url = "https://example.com/v1"

[models.focused]
provider = "prov"
slug = "provider-model-v1"

[agent]
default_model = "provider-model-v1"
"#,
        )
        .unwrap();

        let mut config = load_config_with_options(
            dir.path(),
            &LoadOptions {
                merge_global: false,
                apply_env_overrides: false,
                apply_hierarchical_env: false,
                strict_validation: false,
            },
        )
        .unwrap();
        normalize_and_validate_dispatch_models(&mut config).unwrap();
        assert_eq!(config.agent.default_model, "focused");
    }

    #[test]
    fn unresolved_default_model_is_fatal() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("roko.toml"),
            r#"
[providers.prov]
kind = "openai_compat"
base_url = "https://example.com/v1"

[models.focused]
provider = "prov"
slug = "provider-model-v1"

[agent]
default_model = "missing"
"#,
        )
        .unwrap();

        let mut config = load_config_with_options(
            dir.path(),
            &LoadOptions {
                merge_global: false,
                apply_env_overrides: false,
                apply_hierarchical_env: false,
                strict_validation: false,
            },
        )
        .unwrap();
        let error = normalize_and_validate_dispatch_models(&mut config).unwrap_err();
        assert!(matches!(error, LoadConfigError::UnresolvedModel { .. }));
        assert!(error.to_string().contains("agent.default_model"));
    }

    fn dispatch_config_with_model(key: &str, slug: &str) -> RokoConfig {
        let mut config = RokoConfig::default();
        config.models.insert(
            key.to_string(),
            super::super::schema::ModelProfile {
                provider: "provider".to_string(),
                slug: slug.to_string(),
                ..Default::default()
            },
        );
        config
    }

    #[test]
    fn project_model_slug_shadows_differently_keyed_global_model() {
        let mut project = RokoConfig::from_toml(
            r#"
[providers.claude_cli]
kind = "claude_cli"
command = "claude"

[models.claude-sonnet-4-6]
provider = "claude_cli"
slug = "claude-sonnet-4-6"
context_window = 200000

[agent]
default_model = "claude-sonnet-4-6"
"#,
        )
        .unwrap();
        let global = RokoConfig::from_toml(
            r#"
[providers.claude_cli]
kind = "claude_cli"
command = "claude"

[models.claude-sonnet]
provider = "claude_cli"
slug = "claude-sonnet-4-6"
context_window = 100000

[agent]
default_model = "claude-sonnet"
"#,
        )
        .unwrap();

        merge_global_config_into(&mut project, global);

        assert!(project.models.contains_key("claude-sonnet-4-6"));
        assert!(
            !project.models.contains_key("claude-sonnet"),
            "a differently-keyed global alias must not survive a local slug override"
        );
        assert_eq!(
            project.models["claude-sonnet-4-6"].context_window, 200_000,
            "the project profile must win in full"
        );
        normalize_and_validate_dispatch_models(&mut project).unwrap();
        assert_eq!(project.agent.default_model, "claude-sonnet-4-6");
    }

    #[test]
    fn duplicate_slugs_within_global_layer_are_not_silently_collapsed() {
        let mut project = RokoConfig::default();
        let mut global = dispatch_config_with_model("global-a", "shared-provider-slug");
        global.models.insert(
            "global-b".to_string(),
            super::super::schema::ModelProfile {
                provider: "another-provider".to_string(),
                slug: "shared-provider-slug".to_string(),
                ..Default::default()
            },
        );

        merge_global_config_into(&mut project, global);

        assert_eq!(project.models.len(), 2);
        let error = normalize_and_validate_dispatch_models(&mut project).unwrap_err();
        assert!(matches!(error, LoadConfigError::AmbiguousModelSlug { .. }));
        assert!(error.to_string().contains("global-a, global-b"));
    }

    #[test]
    fn inherited_global_default_tracks_local_slug_override() {
        let mut project = dispatch_config_with_model("project-model", "provider-model");
        project.agent.default_model.clear();
        let mut global = dispatch_config_with_model("global-model", "provider-model");
        global.agent.default_model = "global-model".to_string();

        merge_global_config_into(&mut project, global);

        assert_eq!(project.agent.default_model, "project-model");
        assert!(!project.models.contains_key("global-model"));
        normalize_and_validate_dispatch_models(&mut project).unwrap();
    }

    #[test]
    fn canonical_model_key_wins_over_another_models_slug() {
        let mut config = dispatch_config_with_model("stable-key", "provider-stable");
        config.models.insert(
            "alias-owner".to_string(),
            super::super::schema::ModelProfile {
                provider: "provider".to_string(),
                slug: "stable-key".to_string(),
                ..Default::default()
            },
        );
        config.agent.default_model = " stable-key ".to_string();

        normalize_and_validate_dispatch_models(&mut config).unwrap();

        assert_eq!(config.agent.default_model, "stable-key");
    }

    #[test]
    fn effective_exact_key_wins_when_normalizing_retained_source() {
        let mut source = dispatch_config_with_model("local", "shared");
        source.agent.default_model = " shared ".to_string();
        let mut effective = source.clone();
        effective.models.insert(
            "shared".to_string(),
            super::super::schema::ModelProfile {
                provider: "global-provider".to_string(),
                slug: "global-model".to_string(),
                ..Default::default()
            },
        );

        normalize_source_and_effective_dispatch_models(&mut source, &mut effective).unwrap();

        assert_eq!(source.agent.default_model, "shared");
        assert_eq!(effective.agent.default_model, "shared");
        assert_eq!(
            source.models.len(),
            1,
            "effective models leaked into source"
        );
        assert!(!source.models.contains_key("shared"));
    }

    #[test]
    fn effective_namespace_still_canonicalizes_unique_source_alias() {
        let mut source = dispatch_config_with_model("local", "provider-model");
        source.agent.default_model = " provider-model ".to_string();
        let mut effective = source.clone();
        effective.models.insert(
            "global".to_string(),
            super::super::schema::ModelProfile {
                provider: "global-provider".to_string(),
                slug: "global-model".to_string(),
                ..Default::default()
            },
        );

        normalize_source_and_effective_dispatch_models(&mut source, &mut effective).unwrap();

        assert_eq!(source.agent.default_model, "local");
        assert_eq!(effective.agent.default_model, "local");
        assert!(!source.models.contains_key("global"));
    }

    #[test]
    fn masked_unresolved_source_reference_is_retained_after_effective_validation() {
        let mut source = dispatch_config_with_model("local", "local-model");
        source.agent.default_model = "masked-source-model".to_string();
        let mut effective = source.clone();
        effective.agent.default_model = "local".to_string();

        normalize_source_and_effective_dispatch_models(&mut source, &mut effective).unwrap();

        assert_eq!(source.agent.default_model, "masked-source-model");
        assert_eq!(effective.agent.default_model, "local");
    }

    #[test]
    fn all_dispatch_model_references_normalize_to_canonical_keys() {
        let mut config = dispatch_config_with_model("focused", "provider-model-v1");
        config.agent.default_model = "provider-model-v1".to_string();
        config.agent.fallback_model = Some(" provider-model-v1 ".to_string());
        config
            .agent
            .tier_models
            .insert("mechanical".to_string(), "provider-model-v1".to_string());
        config.agent.roles.insert(
            "reviewer".to_string(),
            super::super::schema::RoleOverride {
                model: Some("provider-model-v1".to_string()),
                ..Default::default()
            },
        );

        normalize_and_validate_dispatch_models(&mut config).unwrap();

        assert_eq!(config.agent.default_model, "focused");
        assert_eq!(config.agent.fallback_model.as_deref(), Some("focused"));
        assert_eq!(
            config
                .agent
                .tier_models
                .get("mechanical")
                .map(String::as_str),
            Some("focused")
        );
        assert_eq!(
            config
                .agent
                .roles
                .get("reviewer")
                .and_then(|role| role.model.as_deref()),
            Some("focused")
        );
    }

    #[test]
    fn unresolved_nested_dispatch_models_report_their_fields() {
        let mut fallback = dispatch_config_with_model("focused", "provider-model-v1");
        fallback.agent.default_model = "focused".to_string();
        fallback.agent.fallback_model = Some("missing".to_string());
        let error = normalize_and_validate_dispatch_models(&mut fallback).unwrap_err();
        assert!(matches!(error, LoadConfigError::UnresolvedModel { .. }));
        assert_eq!(
            error.to_string(),
            "agent.fallback_model references unresolved model 'missing'"
        );

        let mut tier = dispatch_config_with_model("focused", "provider-model-v1");
        tier.agent.default_model = "focused".to_string();
        tier.agent
            .tier_models
            .insert("mechanical".to_string(), "missing".to_string());
        let error = normalize_and_validate_dispatch_models(&mut tier).unwrap_err();
        assert!(matches!(error, LoadConfigError::UnresolvedModel { .. }));
        assert_eq!(
            error.to_string(),
            "agent.tier_models.mechanical references unresolved model 'missing'"
        );

        let mut role = dispatch_config_with_model("focused", "provider-model-v1");
        role.agent.default_model = "focused".to_string();
        role.agent.roles.insert(
            "reviewer".to_string(),
            super::super::schema::RoleOverride {
                model: Some("missing".to_string()),
                ..Default::default()
            },
        );

        let error = normalize_and_validate_dispatch_models(&mut role).unwrap_err();

        assert!(matches!(error, LoadConfigError::UnresolvedModel { .. }));
        assert_eq!(
            error.to_string(),
            "agent.roles.reviewer.model references unresolved model 'missing'"
        );
    }

    #[test]
    fn unresolved_nested_dispatch_models_have_deterministic_first_error() {
        for reverse_insertion in [false, true] {
            for _ in 0..64 {
                let mut tier = dispatch_config_with_model("focused", "provider-model-v1");
                tier.agent.default_model = "focused".to_string();
                let entries = if reverse_insertion {
                    [("zeta", "missing-z"), ("alpha", "missing-a")]
                } else {
                    [("alpha", "missing-a"), ("zeta", "missing-z")]
                };
                for (name, model) in entries {
                    tier.agent
                        .tier_models
                        .insert(name.to_string(), model.to_string());
                }
                let error = normalize_and_validate_dispatch_models(&mut tier).unwrap_err();
                assert_eq!(
                    error.to_string(),
                    "agent.tier_models.alpha references unresolved model 'missing-a'"
                );

                let mut role = dispatch_config_with_model("focused", "provider-model-v1");
                role.agent.default_model = "focused".to_string();
                for (name, model) in entries {
                    role.agent.roles.insert(
                        name.to_string(),
                        super::super::schema::RoleOverride {
                            model: Some(model.to_string()),
                            ..Default::default()
                        },
                    );
                }
                let error = normalize_and_validate_dispatch_models(&mut role).unwrap_err();
                assert_eq!(
                    error.to_string(),
                    "agent.roles.alpha.model references unresolved model 'missing-a'"
                );
            }
        }
    }

    #[test]
    fn empty_model_registry_preserves_legacy_dispatch_references() {
        let mut config = RokoConfig::default();
        config.agent.default_model = "legacy-default".to_string();
        config.agent.fallback_model = Some("legacy-fallback".to_string());
        config
            .agent
            .tier_models
            .insert("mechanical".to_string(), "legacy-tier".to_string());
        config.agent.roles.insert(
            "reviewer".to_string(),
            super::super::schema::RoleOverride {
                model: Some("legacy-role".to_string()),
                ..Default::default()
            },
        );
        let original = config.clone();

        normalize_and_validate_dispatch_models(&mut config).unwrap();

        assert_eq!(config, original);
    }

    #[test]
    fn serialize_effective_roundtrips() {
        let config = RokoConfig::default();
        let toml_str = serialize_effective(&config).unwrap();
        let reparsed: RokoConfig = toml::from_str(&toml_str).unwrap();
        assert_eq!(config, reparsed);
    }

    #[test]
    fn serialize_effective_redacts_secrets() {
        use crate::agent::ProviderKind;
        use crate::config::schema::ProviderConfig;
        use std::collections::HashMap;

        let mut config = RokoConfig::default();

        // Seed a provider with secret-bearing fields.
        let mut headers = HashMap::new();
        headers.insert(
            "authorization".to_string(),
            "Bearer sk-live-1234".to_string(),
        );
        headers.insert("x-custom-token".to_string(), "tok_secret_val".to_string());
        headers.insert("x-trace-id".to_string(), "non-secret-value".to_string());
        config.providers.insert(
            "test".into(),
            ProviderConfig {
                kind: ProviderKind::OpenAiCompat,
                base_url: Some("https://api.example.com/v1".into()),
                api_key_env: Some("ANTHROPIC_API_KEY".into()),
                command: None,
                args: None,
                timeout_ms: None,
                ttft_timeout_ms: None,
                connect_timeout_ms: None,
                extra_headers: Some(headers),
                max_concurrent: None,
                limits: None,
                require_confirmation: false,
            },
        );

        let redacted = serialize_effective_redacted(&config).unwrap();

        // Secret values must be replaced.
        assert!(
            !redacted.contains("sk-live-1234"),
            "authorization header value leaked: {redacted}"
        );
        assert!(
            !redacted.contains("tok_secret_val"),
            "extra_headers value leaked: {redacted}"
        );
        // The redaction marker must appear in place of secrets.
        assert!(
            redacted.contains(REDACTED_MARKER),
            "redaction marker missing from output: {redacted}"
        );

        // Non-secret values must be preserved.
        assert!(
            redacted.contains("https://api.example.com/v1"),
            "base_url should not be redacted: {redacted}"
        );
        assert!(
            redacted.contains("ANTHROPIC_API_KEY"),
            "api_key_env metadata should not be redacted: {redacted}"
        );

        // TOML structure is still valid.
        let reparsed: toml::Value =
            toml::from_str(&redacted).expect("redacted output must be valid TOML");
        assert!(reparsed.is_table());
    }

    #[test]
    fn redaction_preserves_empty_values_and_env_references() {
        let redacted = redact_secrets_in_toml_str(
            r#"
api_key = ""
auth_token = "live-token"
auth_token_env = "ROKO_SERVER_AUTH_TOKEN"

[chain]
wallet_key = "live-wallet-key"

[providers.test]
api_key_env = "OPENAI_API_KEY"

[providers.test.extra_headers]
authorization = ""
x-api-key = "live-header-key"
"#,
        );

        assert!(redacted.contains("api_key = \"\""), "{redacted}");
        assert!(redacted.contains("authorization = \"\""), "{redacted}");
        assert!(redacted.contains("ROKO_SERVER_AUTH_TOKEN"), "{redacted}");
        assert!(redacted.contains("OPENAI_API_KEY"), "{redacted}");
        assert!(!redacted.contains("live-token"), "{redacted}");
        assert!(!redacted.contains("live-wallet-key"), "{redacted}");
        assert!(!redacted.contains("live-header-key"), "{redacted}");
    }

    #[test]
    fn discover_project_config_walks_up() {
        let dir = tempfile::tempdir().unwrap();
        let nested = dir.path().join("a").join("b").join("c");
        std::fs::create_dir_all(&nested).unwrap();
        std::fs::write(dir.path().join("roko.toml"), "config_version = 2\n").unwrap();

        let found = discover_project_config(&nested);
        assert!(found.is_some());
        assert!(found.unwrap().ends_with("roko.toml"));
    }

    #[test]
    fn load_strict_rejects_dangerous_permissions() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("roko.toml"),
            "[runner]\ndangerously_skip_permissions = true\n",
        )
        .unwrap();

        let opts = LoadOptions::strict();
        let result = load_config_with_options(dir.path(), &opts);
        assert!(result.is_err());
    }

    #[test]
    fn load_without_global_merge() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("roko.toml"), "config_version = 2\n").unwrap();

        let opts = LoadOptions {
            merge_global: false,
            apply_env_overrides: false,
            apply_hierarchical_env: false,
            strict_validation: false,
        };
        let config = load_config_with_options(dir.path(), &opts).unwrap();
        // Without global merge, only project-level providers exist.
        // (No assertion on specific providers since global config varies per machine.)
        assert_eq!(config.config_version, 2);
    }

    #[test]
    fn hierarchical_env_to_path_parses_correctly() {
        assert_eq!(
            super::hierarchical_env_to_path("ROKO__AGENT__DEFAULT_MODEL"),
            Some("agent.default_model".to_string())
        );
        assert_eq!(
            super::hierarchical_env_to_path("ROKO__CONDUCTOR__MAX_AGENTS"),
            Some("conductor.max_agents".to_string())
        );
        assert_eq!(super::hierarchical_env_to_path("ROKO__"), None);
        assert_eq!(super::hierarchical_env_to_path("OTHER_VAR"), None);
        assert_eq!(super::hierarchical_env_to_path("ROKO_MODEL"), None);
    }

    #[test]
    fn parse_env_value_to_toml_types() {
        use super::parse_env_value_to_toml;
        assert_eq!(parse_env_value_to_toml("true"), toml::Value::Boolean(true));
        assert_eq!(
            parse_env_value_to_toml("false"),
            toml::Value::Boolean(false)
        );
        assert_eq!(parse_env_value_to_toml("yes"), toml::Value::Boolean(true));
        assert_eq!(parse_env_value_to_toml("no"), toml::Value::Boolean(false));
        // "0" and "1" are integers, not booleans (avoids breaking numeric fields).
        assert_eq!(parse_env_value_to_toml("0"), toml::Value::Integer(0));
        assert_eq!(parse_env_value_to_toml("1"), toml::Value::Integer(1));
        assert_eq!(parse_env_value_to_toml("42"), toml::Value::Integer(42));
        assert_eq!(parse_env_value_to_toml("3.14"), toml::Value::Float(3.14));
        assert_eq!(
            parse_env_value_to_toml("hello"),
            toml::Value::String("hello".to_string())
        );
    }

    #[test]
    fn hierarchical_env_overrides_apply_to_config() {
        let mut config = RokoConfig::default();
        let vars = vec![
            (
                "ROKO__AGENT__DEFAULT_MODEL".to_string(),
                "test-model".to_string(),
            ),
            ("ROKO__CONDUCTOR__MAX_AGENTS".to_string(), "16".to_string()),
            ("ROKO__GATES__SKIP_TESTS".to_string(), "true".to_string()),
        ];

        super::apply_hierarchical_env_overrides_from(&mut config, vars);

        assert_eq!(config.agent.default_model, "test-model");
        assert_eq!(config.conductor.max_agents, 16);
        assert!(config.gates.skip_tests);
    }

    #[test]
    fn hierarchical_env_and_named_env_precedence() {
        // Hierarchical overrides run after named overrides in the loader,
        // so ROKO__AGENT__DEFAULT_MODEL should win over ROKO_MODEL when
        // both are applied. This test exercises the internal function only.
        let mut config = RokoConfig::default();
        config.agent.default_model = "from-named-env".to_string();

        let vars = vec![(
            "ROKO__AGENT__DEFAULT_MODEL".to_string(),
            "from-hierarchical".to_string(),
        )];

        super::apply_hierarchical_env_overrides_from(&mut config, vars);
        assert_eq!(config.agent.default_model, "from-hierarchical");
    }

    /// bug-524a3b: `roko config set` stores a secret in its `ROKO__`
    /// variable, which must set the field it names, digits and all.
    #[test]
    fn env_override_names_set_their_field() {
        assert_eq!(
            super::env_override_name("serve.auth.api_key").as_deref(),
            Some("ROKO__SERVE__AUTH__API_KEY")
        );
        assert_eq!(
            super::env_override_name("server.auth_token").as_deref(),
            Some("ROKO__SERVER__AUTH_TOKEN")
        );
        for unnamed in ["providers.My-Key.api_key", "providers.MyKey.api_key"] {
            assert_eq!(super::env_override_name(unnamed), None, "{unnamed}");
        }

        let mut config = RokoConfig::default();
        let vars = [
            ("ROKO__SERVE__AUTH__API_KEY", "12345"),
            ("ROKO__CONDUCTOR__MAX_AGENTS", "16"),
        ]
        .map(|(name, value)| (name.to_string(), value.to_string()));
        super::apply_hierarchical_env_overrides_from(&mut config, vars);
        assert_eq!(config.serve.auth.api_key, "12345");
        assert_eq!(config.conductor.max_agents, 16);
    }

    #[test]
    fn validated_loader_records_hierarchical_env_provenance() {
        let _env_guard = super::TEST_ENV_LOCK.lock();
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("roko.toml"), "config_version = 2\n").unwrap();

        // Set a hierarchical env var for this test.
        // Note: this test is isolated so env var pollution is acceptable.
        // SAFETY: test is single-threaded; no other thread reads this env var.
        unsafe { std::env::set_var("ROKO__AGENT__DEFAULT_MODEL", "env-test-model") };
        let opts = LoadOptions {
            merge_global: false,
            apply_env_overrides: false,
            apply_hierarchical_env: true,
            strict_validation: false,
        };
        let validated = load_config_validated_with_options(dir.path(), &opts).unwrap();
        // SAFETY: test is single-threaded; no other thread reads this env var.
        unsafe { std::env::remove_var("ROKO__AGENT__DEFAULT_MODEL") };

        assert_eq!(validated.config().agent.default_model, "env-test-model");
        // Should have provenance entry for the env override.
        let has_env_provenance = validated
            .provenance()
            .iter()
            .any(|p| p.key == "agent.default_model");
        assert!(has_env_provenance, "expected env provenance entry");
        assert!(
            validated
                .merge_context
                .field_provenance
                .iter()
                .any(|field| {
                    field.key == "agent.default_model" && field.value_source == ConfigSource::Env
                })
        );
    }

    #[test]
    fn strict_validation_rejects_dangling_provider_reference() {
        let dir = tempfile::tempdir().expect("tempdir");
        let toml_text = r#"
config_version = 2

[validation]
strict_validation = true

[providers.anthropic]
kind = "anthropic_api"
api_key_env = "ANTHROPIC_API_KEY"

[models.fast]
provider = "nonexistent_provider"
slug = "claude-sonnet-4-20250514"
"#;
        std::fs::write(dir.path().join("roko.toml"), toml_text).expect("write roko.toml");

        let result = load_config_with_options(
            dir.path(),
            &LoadOptions {
                merge_global: false,
                apply_env_overrides: false,
                apply_hierarchical_env: false,
                strict_validation: false,
            },
        );
        assert!(
            result.is_err(),
            "strict mode should reject dangling provider ref"
        );
        let err = result.unwrap_err();
        let err_msg = err.to_string();
        assert!(
            err_msg.contains("nonexistent_provider"),
            "error should mention the missing provider: {err_msg}"
        );
        assert!(
            err_msg.contains("fast"),
            "error should mention the model key: {err_msg}"
        );
    }

    #[test]
    fn invariant_validation_rejects_dangling_provider_reference_in_lenient_mode() {
        let dir = tempfile::tempdir().expect("tempdir");
        let toml_text = r#"
config_version = 2

[providers.anthropic]
kind = "anthropic_api"
api_key_env = "ANTHROPIC_API_KEY"

[models.fast]
provider = "nonexistent_provider"
slug = "claude-sonnet-4-20250514"
"#;
        std::fs::write(dir.path().join("roko.toml"), toml_text).expect("write roko.toml");

        let result = load_config_with_options(
            dir.path(),
            &LoadOptions {
                merge_global: false,
                apply_env_overrides: false,
                apply_hierarchical_env: false,
                strict_validation: false,
            },
        );
        assert!(matches!(
            result,
            Err(LoadConfigError::InvariantViolation {
                invariant_id: 3,
                ..
            })
        ));
    }

    #[test]
    fn empty_models_skips_provider_validation() {
        let dir = tempfile::tempdir().expect("tempdir");
        let toml_text = r#"
config_version = 2

[validation]
strict_validation = true
"#;
        std::fs::write(dir.path().join("roko.toml"), toml_text).expect("write roko.toml");

        let result = load_config_with_options(
            dir.path(),
            &LoadOptions {
                merge_global: false,
                apply_env_overrides: false,
                apply_hierarchical_env: false,
                strict_validation: false,
            },
        );
        assert!(
            result.is_ok(),
            "empty models should not trigger validation: {:?}",
            result.err()
        );
    }

    #[test]
    fn global_config_path_returns_none_when_home_unset() {
        let _lock = TEST_ENV_LOCK.lock();

        // Save current values.
        let saved_home = std::env::var("HOME").ok();
        let saved_userprofile = std::env::var("USERPROFILE").ok();

        // SAFETY: single-threaded under TEST_ENV_LOCK.
        unsafe {
            std::env::remove_var("HOME");
            std::env::remove_var("USERPROFILE");
        }

        let result = global_config_path();
        assert!(
            result.is_none(),
            "expected None when HOME and USERPROFILE are both unset, got: {result:?}"
        );

        // Restore.
        unsafe {
            if let Some(h) = saved_home {
                std::env::set_var("HOME", h);
            }
            if let Some(u) = saved_userprofile {
                std::env::set_var("USERPROFILE", u);
            }
        }
    }

    #[test]
    fn global_config_path_returns_some_when_home_set() {
        let _lock = TEST_ENV_LOCK.lock();

        // HOME is normally set in CI and dev environments.
        if std::env::var("HOME").is_ok() || std::env::var("USERPROFILE").is_ok() {
            let result = global_config_path();
            assert!(
                result.is_some(),
                "expected Some when HOME or USERPROFILE is set"
            );
            let path = result.unwrap();
            assert!(
                path.ends_with("config.toml"),
                "expected path ending in config.toml, got: {path:?}"
            );
        }
    }

    #[test]
    fn merge_global_into_returns_err_on_invalid_toml() {
        let dir = tempfile::tempdir().expect("tempdir");
        let _lock = TEST_ENV_LOCK.lock();

        // Point HOME at the temp dir so global_config_path finds our file.
        let saved_home = std::env::var("HOME").ok();
        unsafe {
            std::env::set_var("HOME", dir.path().as_os_str());
        }

        let global_dir = dir.path().join(".roko");
        std::fs::create_dir_all(&global_dir).expect("create .roko dir");
        std::fs::write(
            global_dir.join("config.toml"),
            "this is not valid toml {{{{",
        )
        .expect("write invalid global config");

        let mut config = RokoConfig::default();
        let result = merge_global_into(&mut config);
        assert!(result.is_err(), "expected Err on invalid TOML, got Ok");
        let err = result.unwrap_err();
        assert!(
            matches!(err, super::LoadConfigError::GlobalConfigParse { .. }),
            "expected GlobalConfigParse error, got: {err}"
        );

        // Restore HOME.
        unsafe {
            if let Some(h) = saved_home {
                std::env::set_var("HOME", h);
            }
        }
    }

    // ---- #340: config schema integrity and live config repair ----

    #[test]
    fn validate_paths_detects_unknown_top_level_key() {
        let value: toml::Value = "schema_version = 2\nconfig_version = 2\nbogus = 'x'\n"
            .parse()
            .unwrap();
        let diags = validate_known_config_paths(&value);
        assert!(
            diags
                .iter()
                .any(|d| d.key == "bogus" && d.message.contains("unknown")),
            "expected diagnostic for top-level 'bogus', got: {diags:?}"
        );
    }

    #[test]
    fn validate_paths_detects_unknown_nested_key() {
        let value: toml::Value =
            "schema_version = 2\nconfig_version = 2\n[budget]\nmax_plna_usd = 5.0\n"
                .parse()
                .unwrap();
        let diags = validate_known_config_paths(&value);
        assert!(
            diags
                .iter()
                .any(|d| d.key == "budget.max_plna_usd" && d.message.contains("max_plan_usd")),
            "expected typo suggestion for 'budget.max_plna_usd', got: {diags:?}"
        );
    }

    #[test]
    fn schema_value_for_path_types_known_keys() {
        assert!(matches!(
            schema_value_for_path("budget.max_plan_usd"),
            Some(toml::Value::Float(_))
        ));
        assert!(matches!(
            schema_value_for_path("agent.default_model"),
            Some(toml::Value::String(_))
        ));
        // Dynamic map keys resolve through the map's value template.
        assert!(matches!(
            schema_value_for_path("providers.zai.timeout_ms"),
            Some(toml::Value::Integer(_))
        ));
        assert!(matches!(
            schema_value_for_path("providers.zai.extra_headers.X-Title"),
            Some(toml::Value::String(_))
        ));
        assert!(matches!(
            schema_value_for_path("agent.roles.implementer.model"),
            Some(toml::Value::String(_))
        ));
        assert_eq!(schema_value_for_path("budget.max_plna_usd"), None);
        // v1 names are not v2 keys.
        assert_eq!(schema_value_for_path("agent.model"), None);
    }

    /// bug-12153c: loading strips every key that `build_schema_tree` lacks,
    /// so an optional field without a sentinel was dropped from roko.toml
    /// with no error. A default config serializes none of these keys.
    #[test]
    fn every_optional_config_key_survives_a_load() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("roko.toml");
        std::fs::write(
            &path,
            r#"
schema_version = 2
config_version = 2

[project]
default_domain = "research"

[agent]
fallback_model = "claude-haiku-4-5"
command = "claude"
args = ["--verbose"]
timeout_ms = 1234
env = [["ROKO_TEST_VAR", "1"]]
env_passthrough = ["AWS_*"]
extensions = ["ext-a"]
mcp_config = ".mcp.json"
default_agent_id = "agent-a"
disabled_providers = ["gemini"]

[agent.tier_models]
mechanical = "claude-haiku-4-5"
architectural = "claude-opus-4-6"

[agent.data_llm]
model = "data-model"

[routing]
disabled_providers = ["openai"]
fallback_models = ["claude-haiku-4-5"]

[gates]
env_passthrough = ["CARGO_*"]

[github]
owner = "nunchi"
repo = "roko"

[serve]
port = 7788
"#,
        )
        .expect("write roko.toml");

        let config = load_config_file(
            &path,
            &LoadOptions {
                merge_global: false,
                apply_env_overrides: false,
                apply_hierarchical_env: false,
                strict_validation: false,
            },
        )
        .expect("load roko.toml");

        // The keys that had no sentinel.
        let domain = config.project.default_domain.clone();
        assert_eq!(domain, Some(crate::task::TaskDomain::Research));
        let agent = &config.agent;
        assert_eq!(agent.fallback_model.as_deref(), Some("claude-haiku-4-5"));
        // Exactly the file's entries: the schema sentinel never reaches a config.
        let tiers = HashMap::from([
            ("mechanical".to_string(), "claude-haiku-4-5".to_string()),
            ("architectural".to_string(), "claude-opus-4-6".to_string()),
        ]);
        assert_eq!(agent.tier_models, tiers);
        assert_eq!(config.serve.port, Some(7788));

        // The keys that already had one.
        assert_eq!(agent.command.as_deref(), Some("claude"));
        assert_eq!(agent.args, Some(vec!["--verbose".to_string()]));
        assert_eq!(agent.timeout_ms, Some(1234));
        let env = vec![("ROKO_TEST_VAR".to_string(), "1".to_string())];
        assert_eq!(agent.env, Some(env));
        assert_eq!(agent.env_passthrough, vec!["AWS_*".to_string()]);
        assert_eq!(agent.extensions, vec!["ext-a".to_string()]);
        let mcp_config = agent.mcp_config.as_deref();
        assert_eq!(mcp_config, Some(std::path::Path::new(".mcp.json")));
        assert_eq!(agent.default_agent_id.as_deref(), Some("agent-a"));
        assert_eq!(agent.disabled_providers, vec!["gemini".to_string()]);
        let data_model = agent.data_llm.as_ref().map(|llm| llm.model.as_str());
        assert_eq!(data_model, Some("data-model"));
        let routing = &config.routing;
        assert_eq!(routing.disabled_providers, vec!["openai".to_string()]);
        let fallbacks = vec!["claude-haiku-4-5".to_string()];
        assert_eq!(routing.fallback_models, fallbacks);
        assert_eq!(config.gates.env_passthrough, vec!["CARGO_*".to_string()]);
        assert_eq!(config.github.owner.as_deref(), Some("nunchi"));
        assert_eq!(config.github.repo.as_deref(), Some("roko"));
    }

    /// bug-647249: a load keeps the optional keys that `build_schema_tree`
    /// used to lack, among them the serve auth settings.
    #[test]
    fn documented_optional_keys_survive_a_load() {
        let dir = tempfile::tempdir().expect("tempdir");
        // A key file, as ~/.roko/config.toml is, since it holds a secret
        // (server.auth_token) that roko.toml may not.
        std::fs::create_dir_all(dir.path().join(".roko")).expect("create .roko");
        let path = dir.path().join(".roko").join("config.toml");
        std::fs::write(
            &path,
            r#"
schema_version = 2
config_version = 2

[serve.auth]
privy_app_id = "privy-app"

[[serve.auth.api_keys]]
name = "ci"
key_hash = "hash"
created_at = "2026-09-29T00:00:00Z"

[[serve.auth.jwks_providers]]
url = "https://issuer.example/jwks"
expected_issuer = "https://issuer.example"

[[serve.deploy.webhooks]]
owner = "nunchi"
repo = "roko"

[server]
auth_token = "server-token"

[timeouts]
hard_run_secs = 99

[conductor.watchers.compile_fail_repeat]
max_repeats = 5

[retrieval.role_token_budgets]
implementer = 4000

[gates]
max_rung = 2

[gates.domain_gates]
docs = ["shell:true"]

[dreams]
scheduled_cron = "0 0 3 * * * *"

[learning]
override_learning_dampening = 0.5

[runner]
max_concurrent_plans = 3
"#,
        )
        .expect("write config");

        let config = load_config_file(
            &path,
            &LoadOptions {
                merge_global: false,
                apply_env_overrides: false,
                apply_hierarchical_env: false,
                strict_validation: false,
            },
        )
        .expect("load config");

        let auth = &config.serve.auth;
        assert_eq!(auth.privy_app_id.as_deref(), Some("privy-app"));
        let api_key_names = auth
            .api_keys
            .iter()
            .map(|key| key.name.as_str())
            .collect::<Vec<_>>();
        assert_eq!(api_key_names, ["ci"]);
        assert_eq!(auth.jwks_providers.len(), 1);
        assert_eq!(config.serve.deploy.webhooks.len(), 1);
        assert_eq!(config.server.auth_token.as_deref(), Some("server-token"));
        assert_eq!(config.timeouts.hard_run_secs, Some(99));
        let repeats = config
            .conductor
            .watchers
            .compile_fail_repeat
            .as_ref()
            .map(|watcher| watcher.max_repeats);
        assert_eq!(repeats, Some(5));
        let budget = config.retrieval.role_token_budgets.get("implementer");
        assert_eq!(budget, Some(&4000));
        assert_eq!(config.gates.max_rung, Some(2));
        let docs_gates = config.gates.domain_gates.get("docs").cloned();
        assert_eq!(docs_gates, Some(vec!["shell:true".to_string()]));
        let cron = config.dreams.scheduled_cron.as_deref();
        assert_eq!(cron, Some("0 0 3 * * * *"));
        assert_eq!(config.learning.override_learning_dampening, Some(0.5));
        assert_eq!(config.runner.max_concurrent_plans, Some(3));
    }

    /// gap-e9660f: agents can read roko.toml, so a grep of the project would
    /// show them a secret in it. The loader refuses the file and names where
    /// each secret belongs; from there it still reaches the config.
    #[test]
    fn a_secret_in_the_project_roko_toml_is_moved_or_refused() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("roko.toml");
        let opts = LoadOptions {
            merge_global: false,
            apply_env_overrides: false,
            apply_hierarchical_env: false,
            strict_validation: false,
        };
        let write = |path: &Path, text: &str| std::fs::write(path, text).expect("write config");

        write(
            &path,
            "[serve.auth]\nenabled = true\napi_key = \"sk-serve-test\"\n",
        );
        let error = load_config_file(&path, &opts).expect_err("a secret in roko.toml");
        assert!(
            matches!(error, LoadConfigError::SecretInConfig { .. }),
            "{error}"
        );
        let message = error.to_string();
        assert!(message.contains("serve.auth.api_key"), "{message}");
        assert!(message.contains("ROKO__SERVE__AUTH__API_KEY"), "{message}");
        assert!(!message.contains("sk-serve-test"), "{message}");

        // Moved to the environment, where roko loads .roko/.env, the key
        // still reaches the config.
        write(&path, "[serve.auth]\nenabled = true\n");
        let mut config = load_config_file(&path, &opts).expect("load roko.toml");
        let moved = ("ROKO__SERVE__AUTH__API_KEY", "sk-serve-test");
        apply_hierarchical_env_overrides_from(
            &mut config,
            [(moved.0.to_string(), moved.1.to_string())],
        );
        assert_eq!(config.serve.auth.api_key, "sk-serve-test");

        // Every secret field is named, an agent variable too.
        write(
            &path,
            "[server]\nauth_token = \"t\"\n\n[agent]\nenv = [[\"OPENAI_API_KEY\", \"sk-x\"], [\"RUST_LOG\", \"debug\"]]\n",
        );
        let message = load_config_file(&path, &opts)
            .expect_err("secrets in roko.toml")
            .to_string();
        assert!(message.contains("server.auth_token"), "{message}");
        assert!(message.contains("agent.env.OPENAI_API_KEY"), "{message}");
        assert!(!message.contains("RUST_LOG"), "{message}");

        // A reference is no secret, and a key file may hold one.
        write(
            &path,
            "[providers.x]\nkind = \"openai_compat\"\nbase_url = \"https://x.invalid/v1\"\n\n\
             [providers.x.extra_headers]\nAuthorization = \"Bearer ${X_API_KEY}\"\n\
             token_file = \"/run/secrets/x\"\n",
        );
        load_config_file(&path, &opts).expect("references are not secrets");
        std::fs::create_dir_all(dir.path().join(".roko")).expect("create .roko");
        let key_file = dir.path().join(".roko").join("config.toml");
        write(&key_file, "[serve.auth]\napi_key = \"sk-serve-test\"\n");
        load_config_file(&key_file, &opts).expect("a key file may hold a secret");
    }

    /// bug-8f8704: `${VAR}` was expanded only in provider fields, so a
    /// reference in `serve.auth.api_key` loaded as a literal key. A secret
    /// field that holds a reference, which the loader accepts in a readable
    /// file, now gets the variable's value, and an unset variable fails the
    /// load.
    #[test]
    fn serve_auth_api_key_expands_env_references() {
        let _env_guard = super::TEST_ENV_LOCK.lock();
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("roko.toml");
        let opts = LoadOptions {
            merge_global: false,
            apply_env_overrides: false,
            apply_hierarchical_env: false,
            strict_validation: false,
        };
        let (set, unset) = ("ROKO_TEST_SERVE_KEY_8F8704", "ROKO_TEST_UNSET_8F8704");
        // SAFETY: serialized by TEST_ENV_LOCK; no other test reads these.
        unsafe { std::env::set_var(set, "sk-serve-test") };
        let reference =
            |name: &str| format!("[serve.auth]\nenabled = true\napi_key = \"${{{name}}}\"\n");
        std::fs::write(&path, reference(set)).expect("write config");
        let loaded = load_config_file(&path, &opts);
        std::fs::write(&path, reference(unset)).expect("write config");
        let refused = load_config_file(&path, &opts);
        // SAFETY: serialized by TEST_ENV_LOCK.
        unsafe { std::env::remove_var(set) };
        assert_eq!(
            loaded.expect("a reference is no secret").serve.auth.api_key,
            "sk-serve-test"
        );
        let message = refused.expect_err("an unset variable").to_string();
        assert!(message.contains("serve.auth.api_key"), "{message}");
        assert!(message.contains(unset), "{message}");

        // Every secret field and secret agent variable is expanded; other
        // fields keep their text, and the effective config shows no secret.
        let env = |name: &str| (name == "KEY").then(|| "sk-test".to_string());
        let mut config = RokoConfig::from_toml(
            "[server]\nauth_token = \"Bearer ${KEY}\"\n\n\
             [webhooks.github]\nsecret = \"${KEY}\"\n\n\
             [agent]\ndefault_model = \"${KEY}\"\n\
             env = [[\"OPENAI_API_KEY\", \"${KEY}\"], [\"RUST_LOG\", \"${KEY}\"]]\n",
        )
        .expect("parse config");
        expand_secret_references_with(&mut config, &env).expect("expand");
        assert_eq!(config.server.auth_token.as_deref(), Some("Bearer sk-test"));
        assert_eq!(config.webhooks.github.secret, "sk-test");
        assert_eq!(config.agent.default_model, "${KEY}");
        let agent_env = config.agent.env.clone().expect("agent.env");
        let openai = ("OPENAI_API_KEY".to_string(), "sk-test".to_string());
        let rust_log = ("RUST_LOG".to_string(), "${KEY}".to_string());
        assert!(agent_env.contains(&openai), "{agent_env:?}");
        assert!(agent_env.contains(&rust_log), "{agent_env:?}");
        let shown = serialize_effective_redacted(&config).expect("serialize");
        assert!(!shown.contains("sk-test"), "{shown}");
    }

    /// bug-5a6636: the process scrubber learned only the keys providers read
    /// through `api_key_env`, so a header value, a file secret or
    /// `serve.auth.api_key` reached records and logs unredacted.
    #[test]
    fn the_scrubber_knows_every_config_secret() {
        let _scrubber_guard = crate::obs::scrub::PROCESS_SCRUBBER_LOCK.lock();
        let dir = tempfile::tempdir().expect("tempdir");
        let secret_file = dir.path().join("header.secret");
        std::fs::write(&secret_file, "file-secret-5a6636\n").expect("write secret file");
        // A key file may hold literal secrets.
        std::fs::create_dir_all(dir.path().join(".roko")).expect("create .roko");
        let path = dir.path().join(".roko").join("config.toml");
        let config = format!(
            "[serve.auth]\napi_key = \"serve-key-5a6636\"\n\n\
             [providers.x]\nkind = \"openai_compat\"\nbase_url = \"https://x.invalid/v1\"\n\n\
             [providers.x.extra_headers]\nAuthorization = \"Bearer header-token-5a6636\"\n\
             token_file = \"{}\"\n\n\
             [agent]\nenv = [[\"GITHUB_TOKEN\", \"agent-token-5a6636\"], \
             [\"RUST_LOG\", \"plain-setting-5a6636\"]]\n",
            secret_file.display()
        );
        std::fs::write(&path, config).expect("write config");
        let opts = LoadOptions {
            merge_global: false,
            apply_env_overrides: false,
            apply_hierarchical_env: false,
            strict_validation: false,
        };

        let scrubber = std::sync::Arc::new(crate::obs::LogScrubber::new());
        let previous = crate::obs::install_secret_scrubber(Some(std::sync::Arc::clone(&scrubber)));
        let loaded = load_config_file(&path, &opts);
        crate::obs::install_secret_scrubber(previous);
        loaded.expect("a key file may hold secrets");

        let record = "serve-key-5a6636 Bearer header-token-5a6636 header-token-5a6636 \
                      file-secret-5a6636 agent-token-5a6636 plain-setting-5a6636";
        let scrubbed = scrubber.scrub_literals(record);
        for secret in [
            "serve-key-5a6636",
            "header-token-5a6636",
            "file-secret-5a6636",
            "agent-token-5a6636",
        ] {
            assert!(!scrubbed.contains(secret), "{scrubbed}");
        }
        assert!(
            scrubbed.contains("[REDACTED:serve.auth.api_key]"),
            "{scrubbed}"
        );
        assert!(scrubbed.contains("plain-setting-5a6636"), "{scrubbed}");
    }

    /// Dotted paths of every table in `value` below `prefix`.
    fn collect_table_paths(
        value: &toml::Value,
        prefix: &mut Vec<String>,
        out: &mut Vec<Vec<String>>,
    ) {
        let Some(table) = value.as_table() else {
            return;
        };
        for (key, child) in table {
            if child.is_table() {
                prefix.push(key.clone());
                out.push(prefix.clone());
                collect_table_paths(child, prefix, out);
                prefix.pop();
            }
        }
    }

    fn table_at<'a>(value: &'a toml::Value, path: &[String]) -> Option<&'a toml::Table> {
        path.iter()
            .try_fold(value, |node, key| node.get(key.as_str()))?
            .as_table()
    }

    fn table_at_mut<'a>(
        value: &'a mut toml::Value,
        path: &[String],
    ) -> Option<&'a mut toml::Table> {
        path.iter()
            .try_fold(value, |node, key| node.get_mut(key.as_str()))?
            .as_table_mut()
    }

    /// The keys serde accepts for struct `T`, aliases included: a derived
    /// `Deserialize` hands them to `deserialize_struct` before it reads.
    fn struct_fields<T: serde::de::DeserializeOwned>() -> &'static [&'static str] {
        struct Capture<'a>(&'a mut &'static [&'static str]);

        impl<'de> serde::de::Deserializer<'de> for Capture<'_> {
            type Error = serde::de::value::Error;

            fn deserialize_any<V: serde::de::Visitor<'de>>(
                self,
                _visitor: V,
            ) -> std::result::Result<V::Value, Self::Error> {
                Err(serde::de::Error::custom("not a struct"))
            }

            fn deserialize_struct<V: serde::de::Visitor<'de>>(
                self,
                _name: &'static str,
                fields: &'static [&'static str],
                _visitor: V,
            ) -> std::result::Result<V::Value, Self::Error> {
                *self.0 = fields;
                Err(serde::de::Error::custom("fields captured"))
            }

            serde::forward_to_deserialize_any! {
                bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
                bytes byte_buf option unit unit_struct newtype_struct seq tuple
                tuple_struct map enum identifier ignored_any
            }
        }

        let mut fields: &'static [&'static str] = &[];
        let _ = <T as serde::Deserialize>::deserialize(Capture(&mut fields));
        fields
    }

    /// bug-647249: loading strips every key `build_schema_tree` lacks, so a
    /// field the tree misses is silently dropped from roko.toml. A section
    /// that rejects unknown keys names every key it accepts, so the test
    /// plants `__probe__` in each table of the tree and requires each named
    /// key to be there. A table that keeps an unknown key through a load
    /// and a save is a map with user-defined keys, so it must be a dynamic
    /// section. The templates of dynamic maps whose types ignore unknown
    /// keys are compared with the fields serde reports for them.
    #[test]
    fn every_accepted_config_field_is_in_the_schema_tree() {
        use super::super::agent::RoleOverride;
        use super::super::provider::{ProviderLimits, ProviderRouting};

        const UNKNOWN_PROBE: &str = "unknown field `__probe__`, expected ";
        // Aliases serde accepts that the tree leaves out on purpose:
        // validation asks for the canonical key.
        const ALIASES: &[&str] = &[
            "agent.model",
            "agent.effort",
            "budget.tier_multipliers.focused",
            "budget.tier_multipliers.integrative",
            "budget.tier_multipliers.architectural",
            "gates.custom_rungs",
        ];
        // Tables that take any key but are checked against a fixed key set:
        // `tui.effects` is free-form TOML, and a profile collects unknown
        // keys in its flattened `extra` map.
        const FIXED_KEY_TABLES: &[&str] = &["tui.effects", "profiles._schema_sentinel"];

        let schema = build_schema_tree();
        let parsed = schema.clone().try_into::<RokoConfig>();
        assert!(
            parsed.is_ok(),
            "the schema tree must load as a config: {parsed:?}"
        );

        let mut tables = vec![Vec::new()];
        collect_table_paths(&schema, &mut Vec::new(), &mut tables);
        let mut missing = Vec::new();
        let mut unregistered_maps = Vec::new();
        for path in &tables {
            let dotted = path.join(".");
            let known = table_at(&schema, path).expect("a table of the tree");

            let mut probe = schema.clone();
            let table = table_at_mut(&mut probe, path).expect("a table of the tree");
            table.insert("__probe__".to_string(), toml::Value::Integer(0));
            if let Err(err) = probe.try_into::<RokoConfig>()
                && let Some(expected) = err.message().strip_prefix(UNKNOWN_PROBE)
            {
                for field in expected.split('`').skip(1).step_by(2) {
                    let key = if dotted.is_empty() {
                        field.to_string()
                    } else {
                        format!("{dotted}.{field}")
                    };
                    if !known.contains_key(field) && !ALIASES.contains(&key.as_str()) {
                        missing.push(key);
                    }
                }
                continue;
            }

            if is_dynamic_section(&dotted) || FIXED_KEY_TABLES.contains(&dotted.as_str()) {
                continue;
            }
            let samples = [
                toml::Value::String(String::new()),
                toml::Value::Integer(0),
                toml::Value::Array(Vec::new()),
                toml::Value::Table(toml::Table::new()),
            ];
            for sample in samples {
                let mut probe = schema.clone();
                let table = table_at_mut(&mut probe, path).expect("a table of the tree");
                table.insert("__probe__".to_string(), sample);
                let saved = probe
                    .try_into::<RokoConfig>()
                    .ok()
                    .and_then(|config| toml::Value::try_from(config).ok());
                if saved
                    .as_ref()
                    .and_then(|saved| table_at(saved, path))
                    .is_some_and(|table| table.contains_key("__probe__"))
                {
                    unregistered_maps.push(dotted.clone());
                    break;
                }
            }
        }

        // Dynamic-map templates whose types ignore unknown keys.
        let templates = [
            (
                "agent.roles._schema_sentinel",
                struct_fields::<RoleOverride>(),
            ),
            (
                "providers._schema_sentinel.limits",
                struct_fields::<ProviderLimits>(),
            ),
            (
                "models._schema_sentinel.provider_routing",
                struct_fields::<ProviderRouting>(),
            ),
        ];
        for (template, fields) in templates {
            assert!(!fields.is_empty(), "no serde fields for {template}");
            let path: Vec<String> = template.split('.').map(str::to_string).collect();
            let known = table_at(&schema, &path).expect("a template of the tree");
            for field in fields {
                if !known.contains_key(*field) {
                    missing.push(format!("{template}.{field}"));
                }
            }
        }

        assert!(
            missing.is_empty(),
            "keys serde accepts that build_schema_tree() lacks: {missing:?}"
        );
        assert!(
            unregistered_maps.is_empty(),
            "map tables missing from DYNAMIC_MAP_SECTIONS: {unregistered_maps:?}"
        );
    }

    #[test]
    fn validate_paths_accepts_dynamic_provider_model_profile_keys() {
        let value: toml::Value = r#"
schema_version = 2
config_version = 2
[providers.my-custom-provider]
kind = "openai_compat"
base_url = "https://example.com"
[models.my-custom-model]
provider = "my-custom-provider"
slug = "custom-v1"
context_window = 8000
[profiles.my-domain]
name = "my-domain"
"#
        .parse()
        .unwrap();
        let diags = validate_known_config_paths(&value);
        let unexpected: Vec<_> = diags
            .iter()
            .filter(|d| {
                d.key.starts_with("providers.")
                    || d.key.starts_with("models.")
                    || d.key.starts_with("profiles.")
            })
            .collect();
        assert!(
            unexpected.is_empty(),
            "dynamic map keys must not produce diagnostics: {unexpected:?}"
        );
    }

    #[test]
    fn validate_paths_detects_unknown_key_inside_dynamic_value() {
        let value: toml::Value = r#"
schema_version = 2
config_version = 2
[providers.my-prov]
kind = "openai_compat"
bogus_field = "x"
"#
        .parse()
        .unwrap();

        let diags = validate_known_config_paths(&value);
        assert!(
            diags
                .iter()
                .any(|d| d.key == "providers.my-prov.bogus_field" && d.message.contains("unknown")),
            "expected diagnostic for 'providers.my-prov.bogus_field', got: {diags:?}"
        );
    }

    #[test]
    fn validate_paths_diagnoses_isfr_as_removed() {
        let value: toml::Value = "schema_version = 2\nconfig_version = 2\n[isfr]\nkey = 1\n"
            .parse()
            .unwrap();
        let diags = validate_known_config_paths(&value);
        assert!(
            diags
                .iter()
                .any(|d| d.key == "isfr" && d.message.contains("removed")),
            "expected removal diagnostic for [isfr], got: {diags:?}"
        );
    }

    #[test]
    fn validate_paths_diagnoses_legacy_gate_with_migration_guidance() {
        let value: toml::Value =
            "schema_version = 2\nconfig_version = 2\n[[gate]]\nprogram = 'test'\n"
                .parse()
                .unwrap();
        let diags = validate_known_config_paths(&value);
        assert!(
            diags
                .iter()
                .any(|d| d.key == "gate" && d.message.contains("gates.rungs")),
            "expected migration guidance for [[gate]], got: {diags:?}"
        );
    }

    #[test]
    fn validate_paths_accepts_clean_default_config() {
        let value =
            toml::Value::try_from(RokoConfig::default()).expect("default config must serialize");
        let diags = validate_known_config_paths(&value);
        assert!(
            diags.is_empty(),
            "default config must produce no diagnostics, got: {diags:?}"
        );
    }

    #[test]
    fn context_pressure_enabled_removed_from_schema() {
        // The field was removed from ConductorConfig. Config files that still
        // contain it should now report it as an unknown key via path validation.
        let value: toml::Value = r#"
schema_version = 2
config_version = 2
[conductor]
context_pressure_enabled = true
"#
        .parse()
        .unwrap();
        let diags = validate_known_config_paths(&value);
        let flagged: Vec<_> = diags
            .iter()
            .filter(|d| d.key.contains("context_pressure_enabled"))
            .collect();
        assert!(
            !flagged.is_empty(),
            "context_pressure_enabled should be flagged as unknown after removal"
        );
    }

    #[test]
    fn routing_failover_lists_survive_unknown_field_stripping() {
        let text = r#"
[routing]
disabled_providers = ["openai"]
fallback_models = ["kimi-k2-5", "glm51"]
exhaustion_cooldown_secs = 600
"#;
        let value: toml::Value = text.parse().expect("parse routing toml");
        assert!(validate_known_config_paths(&value).is_empty());
        let config = deserialize_migrated_toml(text).expect("load routing config");
        assert_eq!(config.routing.disabled_providers, ["openai"]);
        assert_eq!(config.routing.fallback_models, ["kimi-k2-5", "glm51"]);
        assert_eq!(config.routing.exhaustion_cooldown_secs, 600);
    }

    #[test]
    fn env_passthrough_lists_survive_unknown_field_stripping() {
        let text = r#"
[agent]
env_passthrough = ["AWS_*"]

[gates]
env_passthrough = ["DATABASE_URL", "PGHOST"]
"#;
        let value: toml::Value = text.parse().expect("parse passthrough toml");
        assert!(validate_known_config_paths(&value).is_empty());
        let config = deserialize_migrated_toml(text).expect("load passthrough config");
        assert_eq!(config.agent.env_passthrough, ["AWS_*"]);
        assert_eq!(config.gates.env_passthrough, ["DATABASE_URL", "PGHOST"]);
    }

    #[test]
    fn checked_in_config_loads_cleanly_with_nonzero_budgets() {
        // Load the actual workspace roko.toml and verify it produces no
        // unknown-path diagnostics and has the intended budget values.
        let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let workspace_root = manifest_dir
            .parent()
            .and_then(|p| p.parent())
            .expect("workspace root");
        let config_path = workspace_root.join("roko.toml");
        if !config_path.exists() {
            // Skip in CI or alternate build contexts where roko.toml may
            // not be available from the crate manifest dir.
            return;
        }
        let text = std::fs::read_to_string(&config_path).expect("read roko.toml");
        let value: toml::Value = text.parse().expect("parse roko.toml");
        let diags = validate_known_config_paths(&value);
        assert!(
            diags.is_empty(),
            "checked-in roko.toml must produce no unknown-path diagnostics, got: {diags:?}"
        );

        let config: RokoConfig = value.try_into().expect("deserialize roko.toml");
        assert!(
            config.budget.max_plan_usd > 0.0,
            "checked-in budget.max_plan_usd must be nonzero, got: {}",
            config.budget.max_plan_usd
        );
        assert!(
            config.budget.max_turn_usd > 0.0,
            "checked-in budget.max_turn_usd must be nonzero, got: {}",
            config.budget.max_turn_usd
        );
    }

    #[test]
    fn validate_paths_suggests_nearest_key_for_typos() {
        let value: toml::Value = "schema_version = 2\nconfig_version = 2\nbudegt = 5\n"
            .parse()
            .unwrap();
        let diags = validate_known_config_paths(&value);
        assert!(
            diags
                .iter()
                .any(|d| d.key == "budegt" && d.message.contains("budget")),
            "expected typo suggestion 'budget' for 'budegt', got: {diags:?}"
        );
    }

    #[test]
    fn validate_paths_forward_compatible_ordinary_load() {
        // Unknown future sections must NOT prevent loading. This is the
        // forward-compatibility contract: ordinary loads warn, don't reject.
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(
            dir.path().join("roko.toml"),
            "schema_version = 2\nconfig_version = 2\nfuture_v3_section = 'new'\n",
        )
        .expect("write config");

        let config = load_config_with_options(
            dir.path(),
            &LoadOptions {
                merge_global: false,
                apply_env_overrides: false,
                apply_hierarchical_env: false,
                strict_validation: false,
            },
        )
        .expect("forward-compatible load must succeed");
        // Verify the config loaded successfully with defaults.
        assert_eq!(config.schema_version, 2);
    }

    #[test]
    fn levenshtein_distance_basic() {
        assert_eq!(levenshtein_distance("budget", "budegt"), 2);
        assert_eq!(levenshtein_distance("budget", "budget"), 0);
        assert_eq!(levenshtein_distance("abc", "xyz"), 3);
        assert_eq!(levenshtein_distance("", "abc"), 3);
        assert_eq!(levenshtein_distance("abc", ""), 3);
    }
}
