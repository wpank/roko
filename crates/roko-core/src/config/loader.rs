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

use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use parking_lot::Mutex;

use super::LoadConfigError;
use super::provenance::{
    ConfigDiagnostic, ConfigProvenance, ConfigSource, FieldProvenance, MergeContext,
    ValidatedConfig,
};
use super::schema::RokoConfig;
use super::validation::{InvariantSeverity, validate_invariants};

/// One in-place schema migration from version N to N+1.
pub type MigrationFn = fn(&mut toml::Value) -> Result<(), String>;

/// A successfully applied migration edge.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MigrationStep {
    pub from: u32,
    pub to: u32,
    pub description: String,
}

/// Outcome of attempting to migrate one TOML value.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MigrationReport {
    pub from_version: u32,
    pub to_version: u32,
    pub steps_applied: Vec<MigrationStep>,
    pub warnings: Vec<String>,
}

/// Ordered registry of schema migrations.
pub struct ConfigMigrator {
    pub migrations: BTreeMap<u32, MigrationFn>,
    pub target_version: u32,
}

impl ConfigMigrator {
    #[must_use]
    pub fn new() -> Self {
        let mut migrator = Self {
            migrations: BTreeMap::new(),
            target_version: super::schema::CURRENT_SCHEMA_VERSION,
        };
        migrator.register(1, migrate_v1_to_v2);
        migrator
    }

    pub fn register(&mut self, from_version: u32, migration: MigrationFn) {
        self.migrations.insert(from_version, migration);
    }

    /// Apply every available N -> N+1 migration up to the target schema.
    ///
    /// The report retains partial progress and warnings when a migration edge
    /// is unavailable or fails. Live loaders reject reports that do not reach
    /// `target_version`; tooling may inspect the partial report directly.
    pub fn migrate(&self, toml_value: &mut toml::Value) -> MigrationReport {
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

fn migrate_v1_to_v2(value: &mut toml::Value) -> Result<(), String> {
    let root = value
        .as_table_mut()
        .ok_or_else(|| "config root must be a TOML table".to_string())?;

    if let Some(agent) = root.get_mut("agent").and_then(toml::Value::as_table_mut) {
        rename_if_absent(agent, "model", "default_model");
        rename_if_absent(agent, "backend", "default_backend");
        rename_if_absent(agent, "effort", "default_effort");
    }
    if let Some(budget) = root.get_mut("budget").and_then(toml::Value::as_table_mut) {
        rename_if_absent(budget, "max_session_usd", "max_plan_usd");
        rename_if_absent(budget, "max_agent_usd", "max_turn_usd");
    }

    root.insert("schema_version".to_string(), toml::Value::Integer(2));
    root.insert("config_version".to_string(), toml::Value::Integer(2));
    Ok(())
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
        if let Err(e) = merge_global_into(&mut config) {
            tracing::warn!(error = %e, "global config merge failed; continuing with project config only");
        }
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
    config.interpolate_env_vars();
    config.resolve_file_secrets();

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
/// literal (bool, integer, float) or stored as a string.
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
        let parsed_value = parse_env_value_to_toml(raw_value);
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
    validate_invariants(config)
        .into_iter()
        .filter(|result| result.severity == InvariantSeverity::Warning)
        .map(|result| ConfigDiagnostic {
            key: format!("invariant.{}.{}", result.invariant_id, result.config_path),
            message: result.message,
        })
        .collect()
}

fn unknown_field_diagnostics(value: &toml::Value) -> Vec<ConfigDiagnostic> {
    const TOP_LEVEL_FIELDS: &[&str] = &[
        "config_version",
        "schema_version",
        "project",
        "prd",
        "agent",
        "providers",
        "models",
        "profiles",
        "gate",
        "gates",
        "graduation",
        "isfr",
        "routing",
        "pipeline",
        "budget",
        "conductor",
        "watcher",
        "learning",
        "tui",
        "timeouts",
        "statehub",
        "serve",
        "scheduler",
        "webhooks",
        "github",
        "subscriptions",
        "server",
        "deploy",
        "perplexity",
        "gemini",
        "tools",
        "chain",
        "relay",
        "feed_agents",
        "runner",
        "agents",
        "validation",
        "cold_storage",
        "prompt",
        "resources",
    ];

    let Some(table) = value.as_table() else {
        return Vec::new();
    };
    table
        .keys()
        .filter(|key| !TOP_LEVEL_FIELDS.contains(&key.as_str()))
        .map(|key| ConfigDiagnostic {
            key: key.clone(),
            message: format!("unknown config field '{key}' was ignored"),
        })
        .collect()
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

/// Recursive inner helper that knows the current key name.
fn redact_secrets_in_toml_keyed(key: &str, value: &mut toml::Value) {
    let key_lower = key.to_ascii_lowercase();
    // `*_env` fields contain environment-variable names, not secret values.
    let is_env_reference = key_lower.ends_with("_env");
    let is_secret_key = !is_env_reference
        && SECRET_KEY_FRAGMENTS
            .iter()
            .any(|frag| key_lower.contains(frag));
    let is_extra_headers = key_lower == "extra_headers";

    match value {
        toml::Value::String(secret) if is_secret_key && !secret.is_empty() => {
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
                if is_secret_key {
                    if item.as_str().is_some_and(|secret| !secret.is_empty()) {
                        *item = toml::Value::String(REDACTED_MARKER.to_string());
                    }
                } else {
                    redact_secrets_in_toml(item);
                }
            }
        }
        _ => {}
    }
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
pub fn merge_global_into(config: &mut RokoConfig) -> Result<(), String> {
    let Some(global_path) = global_config_path() else {
        return Ok(());
    };
    if !global_path.exists() {
        return Ok(());
    }

    let text = match std::fs::read_to_string(&global_path) {
        Ok(t) => t,
        Err(e) => {
            tracing::warn!(
                path = %global_path.display(),
                error = %e,
                "failed to read global config"
            );
            return Ok(());
        }
    };

    let global = match deserialize_migrated_toml(&text) {
        Ok(g) => g,
        Err(e) => {
            tracing::error!(
                path = %global_path.display(),
                error = %e,
                "failed to parse global config"
            );
            return Err(format!(
                "failed to parse global config {}: {e}",
                global_path.display()
            ));
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
    value
        .try_into::<RokoConfig>()
        .map_err(|error| error.to_string())
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
            err.contains("failed to parse"),
            "error should mention parse failure: {err}"
        );

        // Restore HOME.
        unsafe {
            if let Some(h) = saved_home {
                std::env::set_var("HOME", h);
            }
        }
    }
}
