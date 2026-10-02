//! Strict config validation helpers for safety-sensitive settings.

// Several helpers here back the config doctor / config validate commands
// which call through the CLI but don't yet reach this module's low-level
// helpers in every compile path.
#![allow(dead_code)]

use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use toml::Value;

use super::provenance::ConfigSource;
use super::schema::RokoConfig;

/// Severity assigned to a semantic config invariant result.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InvariantSeverity {
    Error,
    Warning,
}

/// One failed semantic invariant. Passing invariants are omitted.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct InvariantResult {
    pub invariant_id: u8,
    pub severity: InvariantSeverity,
    pub message: String,
    pub config_path: String,
}

fn invariant(
    invariant_id: u8,
    severity: InvariantSeverity,
    config_path: impl Into<String>,
    message: impl Into<String>,
) -> InvariantResult {
    InvariantResult {
        invariant_id,
        severity,
        message: message.into(),
        config_path: config_path.into(),
    }
}

/// Validate cross-section relationships on a fully resolved config.
///
/// Only failed invariants are returned. Callers reject `Error` results and
/// may surface `Warning` results without preventing startup.
#[must_use]
pub fn validate_invariants(config: &RokoConfig) -> Vec<InvariantResult> {
    let mut results = Vec::new();

    // A zero ceiling means no cap, for the plan and the turn alike. Only a
    // finite turn ceiling above a finite plan ceiling contradicts it; a turn
    // with no cap of its own is still bounded by the plan ceiling.
    let budget = &config.budget;
    if budget.max_plan_usd > 0.0 && budget.max_turn_usd > budget.max_plan_usd {
        results.push(invariant(
            1,
            InvariantSeverity::Error,
            "budget.max_turn_usd",
            format!(
                "budget.max_turn_usd ({}) must not exceed budget.max_plan_usd ({})",
                budget.max_turn_usd, budget.max_plan_usd
            ),
        ));
    }

    // The live schema models rung retry overrides as pipeline complexity
    // bands. Each override must remain within the global gate retry ceiling.
    let pipeline_iterations = [
        ("mechanical", config.pipeline.mechanical.max_iterations),
        ("focused", config.pipeline.focused.max_iterations),
        ("integrative", config.pipeline.integrative.max_iterations),
        (
            "architectural",
            config.pipeline.architectural.max_iterations,
        ),
    ];
    for (name, iterations) in pipeline_iterations {
        if iterations > config.gates.max_iterations {
            results.push(invariant(
                2,
                InvariantSeverity::Warning,
                format!("pipeline.{name}.max_iterations"),
                format!(
                    "pipeline.{name}.max_iterations ({iterations}) exceeds gates.max_iterations ({})",
                    config.gates.max_iterations
                ),
            ));
        }
    }
    for pair in pipeline_iterations.windows(2) {
        let [
            (lower_name, lower_iterations),
            (higher_name, higher_iterations),
        ] = pair
        else {
            continue;
        };
        if lower_iterations > higher_iterations {
            results.push(invariant(
                2,
                InvariantSeverity::Warning,
                format!("pipeline.{higher_name}.max_iterations"),
                format!(
                    "pipeline retry hierarchy decreases from {lower_name} ({lower_iterations}) to {higher_name} ({higher_iterations})"
                ),
            ));
        }
    }

    for (model, profile) in &config.models {
        if !config.providers.contains_key(&profile.provider) {
            results.push(invariant(
                3,
                InvariantSeverity::Error,
                format!("models.{model}.provider"),
                format!(
                    "model '{model}' references provider '{}' which is not configured",
                    profile.provider
                ),
            ));
        }
    }

    let estimated_parallel_cost =
        config.conductor.max_agents as f64 * f64::from(config.budget.max_turn_usd) * 10.0;
    if config.budget.max_plan_usd > 0.0
        && estimated_parallel_cost > f64::from(config.budget.max_plan_usd)
    {
        results.push(invariant(
            4,
            InvariantSeverity::Warning,
            "conductor.max_agents",
            format!(
                "estimated parallel-agent cost ({estimated_parallel_cost:.2} USD) exceeds budget.max_plan_usd ({:.2} USD)",
                config.budget.max_plan_usd
            ),
        ));
    }

    if !(4..=1_000).contains(&config.agent.context_limit_k) {
        results.push(invariant(
            5,
            InvariantSeverity::Warning,
            "agent.context_limit_k",
            format!(
                "agent.context_limit_k ({}) is outside the supported range 4..=1000",
                config.agent.context_limit_k
            ),
        ));
    }

    if config.conductor.max_agents == 0 {
        results.push(invariant(
            6,
            InvariantSeverity::Error,
            "conductor.max_agents",
            "conductor.max_agents must be at least 1",
        ));
    }

    if config.learning.replan_on_gate_failure
        && config.gates.skip_tests
        && !config.gates.clippy_enabled
    {
        results.push(invariant(
            7,
            InvariantSeverity::Warning,
            "learning.replan_on_gate_failure",
            "replan_on_gate_failure is enabled while test and clippy gates are disabled",
        ));
    }

    // The data LLM reads untrusted content, so it stays tool-less, and each
    // of its calls is bounded in time and size (gap-b0d514).
    if let Some(data_llm) = &config.agent.data_llm {
        if !data_llm.strip_tool_calls {
            results.push(invariant(
                8,
                InvariantSeverity::Error,
                "agent.data_llm.strip_tool_calls",
                "agent.data_llm.strip_tool_calls must be true: the data LLM reads untrusted \
                 content, so it may not call tools",
            ));
        }
        if data_llm.timeout_ms == 0 {
            results.push(invariant(
                8,
                InvariantSeverity::Error,
                "agent.data_llm.timeout_ms",
                "agent.data_llm.timeout_ms must be at least 1",
            ));
        }
        if data_llm.max_input_bytes == 0 {
            results.push(invariant(
                8,
                InvariantSeverity::Error,
                "agent.data_llm.max_input_bytes",
                "agent.data_llm.max_input_bytes must be at least 1",
            ));
        }
    }

    // 3210: the spec-quality gate's thresholds are scores out of 100, the
    // block threshold sits at or below the allow threshold, and the holdout
    // is a share.
    for (config_path, message) in config.spec_quality.problems() {
        results.push(invariant(9, InvariantSeverity::Error, config_path, message));
    }

    results
}

/// Explicit source mode for strict config validation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StrictConfigSource {
    Shared { path: Option<PathBuf> },
    LocalOverride { path: PathBuf },
    TestFixture { path: PathBuf },
}

impl StrictConfigSource {
    #[must_use]
    pub fn shared(path: impl Into<Option<PathBuf>>) -> Self {
        Self::Shared { path: path.into() }
    }

    #[must_use]
    pub fn local_override(path: impl Into<PathBuf>) -> Self {
        Self::LocalOverride { path: path.into() }
    }

    #[must_use]
    pub fn test_fixture(path: impl Into<PathBuf>) -> Self {
        Self::TestFixture { path: path.into() }
    }

    fn allows_dangerous_permission_skip(&self) -> bool {
        matches!(self, Self::LocalOverride { .. } | Self::TestFixture { .. })
    }

    fn path(&self) -> Option<&Path> {
        match self {
            Self::Shared { path } => path.as_deref(),
            Self::LocalOverride { path } | Self::TestFixture { path } => Some(path.as_path()),
        }
    }
}

/// Strict config validation error.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum StrictConfigValidationError {
    #[error("parse config toml: {0}")]
    ParseToml(String),
    #[error("runner.dangerously_skip_permissions=true is not valid in shared config{path_suffix}")]
    DangerousRootPermission { path_suffix: String },
}

/// Validate safety-sensitive raw TOML settings without changing legacy load behavior.
///
/// This checks only explicitly configured values. An absent
/// `runner.dangerously_skip_permissions` key is accepted here so strict validation
/// can be introduced without changing deserialization defaults in this packet.
pub fn validate_strict_config_toml(
    toml_text: &str,
    source: &StrictConfigSource,
) -> Result<(), StrictConfigValidationError> {
    let value = toml_text
        .parse::<Value>()
        .map_err(|err| StrictConfigValidationError::ParseToml(err.to_string()))?;

    let dangerous_skip = value
        .get("runner")
        .and_then(|runner| runner.get("dangerously_skip_permissions"))
        .and_then(Value::as_bool)
        .unwrap_or(false);

    if dangerous_skip && !source.allows_dangerous_permission_skip() {
        let path_suffix = source
            .path()
            .map(|path| format!(" ({})", path.display()))
            .unwrap_or_default();
        return Err(StrictConfigValidationError::DangerousRootPermission { path_suffix });
    }

    Ok(())
}

/// Local-only permission bypass metadata.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DangerousPermissionOverride {
    pub enabled: bool,
    pub scope: String,
    pub reason: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<DateTime<Utc>>,
    pub ack_env: String,
    pub source: ConfigSource,
}

impl DangerousPermissionOverride {
    #[must_use]
    pub fn disabled() -> Self {
        Self {
            enabled: false,
            scope: String::new(),
            reason: String::new(),
            expires_at: None,
            ack_env: String::new(),
            source: ConfigSource::Default,
        }
    }

    pub fn validate_at(&self, now: DateTime<Utc>) -> Result<(), DangerousPermissionOverrideError> {
        if !self.enabled {
            return Ok(());
        }

        if self.reason.trim().is_empty() {
            return Err(DangerousPermissionOverrideError::MissingReason);
        }
        if self.scope.trim().is_empty() {
            return Err(DangerousPermissionOverrideError::MissingScope);
        }
        let expires_at = self
            .expires_at
            .ok_or(DangerousPermissionOverrideError::MissingExpiry)?;
        if expires_at <= now {
            return Err(DangerousPermissionOverrideError::Expired);
        }
        if self.source != ConfigSource::LocalOverride {
            return Err(DangerousPermissionOverrideError::NonLocalSource);
        }
        if self.ack_env.trim().is_empty() {
            return Err(DangerousPermissionOverrideError::MissingAcknowledgementEnv);
        }

        Ok(())
    }
}

/// Validation failure for a local dangerous permission override.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum DangerousPermissionOverrideError {
    #[error("dangerous permission override requires a non-empty reason")]
    MissingReason,
    #[error("dangerous permission override requires a non-empty scope")]
    MissingScope,
    #[error("dangerous permission override requires an expiry")]
    MissingExpiry,
    #[error("dangerous permission override has expired")]
    Expired,
    #[error("dangerous permission override source must be local_override")]
    NonLocalSource,
    #[error("dangerous permission override requires an acknowledgement env name")]
    MissingAcknowledgementEnv,
}

// ---- two-phase TOML unknown-field detection and repair ---------------

/// One unknown field discovered via two-phase TOML parsing.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct UnknownFieldReport {
    /// Dot-separated TOML path (e.g. `"agent.bogus_key"`).
    pub(crate) path: String,
    /// The raw TOML value at that path.
    pub(crate) value: String,
    /// A "did you mean X?" suggestion if edit distance is close, or `None`.
    pub(crate) suggestion: Option<String>,
}

/// Detect unknown fields in raw TOML text by attempting a full deserialize
/// through `RokoConfig`. Serde's `deny_unknown_fields` produces an error
/// message that names the unknown key and the expected alternatives. This
/// function collects those into structured reports.
///
/// Returns `Ok(vec![])` when the TOML is clean.
/// Returns `Ok(reports)` when unknown fields are found.
/// Returns `Err(msg)` only for truly unparseable TOML (syntax errors).
#[must_use]
pub(crate) fn detect_unknown_fields(toml_text: &str) -> Vec<UnknownFieldReport> {
    // Phase 1: try to deserialize as RokoConfig. If it succeeds, there are
    // no unknown fields in sections that have deny_unknown_fields.
    match toml::from_str::<RokoConfig>(toml_text) {
        Ok(_) => Vec::new(),
        Err(err) => {
            // Phase 2: parse the error message for unknown field info.
            let msg = err.to_string();
            parse_unknown_field_error(&msg)
        }
    }
}

/// Parse a serde/TOML error message to extract the unknown field path and suggestion.
fn parse_unknown_field_error(msg: &str) -> Vec<UnknownFieldReport> {
    // toml produces errors like:
    // "TOML parse error at line 3, column 1\n ... unknown field `bogus_key`, expected one of ..."
    // We extract the field name and any expected list.
    let mut reports = Vec::new();

    // Look for "unknown field `<name>`"
    if let Some(field_start) = msg.find("unknown field `") {
        let rest = &msg[field_start + 15..];
        if let Some(field_end) = rest.find('`') {
            let field_name = &rest[..field_end];

            // Try to extract expected fields for suggestion
            let suggestion = if let Some(expected_start) = rest.find("expected one of ") {
                let expected_rest = &rest[expected_start + 16..];
                // Extract the list of expected fields
                let expected_fields: Vec<&str> = expected_rest
                    .split('`')
                    .enumerate()
                    .filter_map(|(idx, part)| {
                        // Expected fields appear between backticks at odd indices
                        if idx % 2 == 1 { Some(part) } else { None }
                    })
                    .collect();
                find_closest(field_name, &expected_fields)
            } else if let Some(expected_start) = rest.find("expected ") {
                let expected_rest = &rest[expected_start + 9..];
                let expected_fields: Vec<&str> = expected_rest
                    .split('`')
                    .enumerate()
                    .filter_map(|(idx, part)| if idx % 2 == 1 { Some(part) } else { None })
                    .collect();
                find_closest(field_name, &expected_fields)
            } else {
                None
            };

            // Try to extract the TOML path from "at line X, column Y" + context
            let path = extract_toml_path_context(msg, field_name);

            reports.push(UnknownFieldReport {
                path,
                value: String::new(),
                suggestion,
            });
        }
    }

    reports
}

/// Extract a dotted TOML path from the error context. Falls back to the raw field name.
fn extract_toml_path_context(msg: &str, field_name: &str) -> String {
    // toml-rs errors sometimes include the table path; extract it if present
    // e.g. "in `agent`" or "in `serve.auth`"
    if let Some(in_start) = msg.find("in `") {
        let rest = &msg[in_start + 4..];
        if let Some(in_end) = rest.find('`') {
            let table = &rest[..in_end];
            return format!("{table}.{field_name}");
        }
    }
    field_name.to_string()
}

/// Find the closest match from a list of candidates using edit distance.
fn find_closest(needle: &str, candidates: &[&str]) -> Option<String> {
    if candidates.is_empty() || needle.is_empty() {
        return None;
    }
    let mut best_match = None;
    let mut best_distance = usize::MAX;
    for &candidate in candidates {
        let distance = levenshtein(needle, candidate);
        if distance < best_distance {
            best_distance = distance;
            best_match = Some(candidate);
        }
    }
    // Only suggest if edit distance is reasonable (at most 3)
    (best_distance <= 3).then(|| best_match.expect("distance implies candidate").to_string())
}

/// Simple Levenshtein distance for config key typo detection.
fn levenshtein(a: &str, b: &str) -> usize {
    if a == b {
        return 0;
    }
    if a.is_empty() {
        return b.len();
    }
    if b.is_empty() {
        return a.len();
    }
    let b_chars: Vec<char> = b.chars().collect();
    let mut costs: Vec<usize> = (0..=b_chars.len()).collect();
    for (i, a_ch) in a.chars().enumerate() {
        let mut prev_diag = costs[0];
        costs[0] = i + 1;
        for (j, &b_ch) in b_chars.iter().enumerate() {
            let ins = costs[j + 1] + 1;
            let del = costs[j] + 1;
            let sub = prev_diag + usize::from(a_ch != b_ch);
            prev_diag = costs[j + 1];
            costs[j + 1] = ins.min(del).min(sub);
        }
    }
    *costs.last().unwrap_or(&0)
}

/// Format a human-readable repair suggestion for CLI output.
#[must_use]
pub(crate) fn format_repair_suggestions(reports: &[UnknownFieldReport]) -> String {
    if reports.is_empty() {
        return "Config is clean: no unknown fields detected.".to_string();
    }
    let mut out = String::new();
    for report in reports {
        out.push_str(&format!("  unknown field: {}", report.path));
        if let Some(ref suggestion) = report.suggestion {
            out.push_str(&format!(" (did you mean `{suggestion}`?)"));
        }
        out.push('\n');
        out.push_str("    fix: remove this field from roko.toml\n");
    }
    out
}

// ---- Provider/model semantic validation (backlog #370) -------------------

/// Stable finding code for provider/model semantic validation.
///
/// Each variant maps to exactly one frozen validation rule. The `Display`
/// impl produces the canonical dotted-string form used in JSON output and
/// diagnostic messages.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SemanticFindingCode {
    ProviderEmptyApiKeyEnv,
    ProviderInvalidBaseUrl,
    ProviderInvalidTimeout,
    ProviderInvalidSearchContext,
    ModelUnknownProvider,
    ModelEmptySlug,
    ModelInvalidContext,
    ModelAmbiguousSlug,
    ModelInvalidToolFormat,
    RoutingUnresolvedModel,
}

impl std::fmt::Display for SemanticFindingCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::ProviderEmptyApiKeyEnv => "provider.empty_api_key_env",
            Self::ProviderInvalidBaseUrl => "provider.invalid_base_url",
            Self::ProviderInvalidTimeout => "provider.invalid_timeout",
            Self::ProviderInvalidSearchContext => "provider.invalid_search_context",
            Self::ModelUnknownProvider => "model.unknown_provider",
            Self::ModelEmptySlug => "model.empty_slug",
            Self::ModelInvalidContext => "model.invalid_context",
            Self::ModelAmbiguousSlug => "model.ambiguous_slug",
            Self::ModelInvalidToolFormat => "model.invalid_tool_format",
            Self::RoutingUnresolvedModel => "routing.unresolved_model",
        };
        f.write_str(s)
    }
}

/// One semantic finding from provider/model validation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SemanticFinding {
    pub code: SemanticFindingCode,
    pub severity: InvariantSeverity,
    /// Dot-path in the config (e.g. `providers.anthropic.base_url`).
    pub path: String,
    pub message: String,
}

/// Validate provider and model configuration semantically.
///
/// Runs all frozen rules from backlog #370 against the effective (post-merge)
/// config. Results are deterministically ordered: provider-sorted, then
/// model-sorted. Messages never contain env values, secrets, or URL
/// credentials.
#[must_use]
pub fn validate_provider_semantics(config: &RokoConfig) -> Vec<SemanticFinding> {
    use crate::agent::ProviderKind;
    use crate::tool::ToolFormat;

    let mut findings = Vec::new();

    // Sort provider names for deterministic output.
    let mut provider_names: Vec<&String> = config.providers.keys().collect();
    provider_names.sort();

    for name in &provider_names {
        let provider = &config.providers[*name];
        let is_cli = matches!(
            provider.kind,
            ProviderKind::ClaudeCli
                | ProviderKind::CodexCli
                | ProviderKind::CursorAcp
                | ProviderKind::CursorCli
                | ProviderKind::GeminiCli
                | ProviderKind::Hermes
                | ProviderKind::OpenClaw
        );

        // Rule: provider.empty_api_key_env
        if !is_cli {
            match provider.api_key_env.as_ref().map(|s| s.trim()) {
                Some("") => {
                    findings.push(SemanticFinding {
                        code: SemanticFindingCode::ProviderEmptyApiKeyEnv,
                        severity: InvariantSeverity::Error,
                        path: format!("providers.{name}.api_key_env"),
                        message: format!(
                            "HTTP provider '{name}' has empty api_key_env; \
                             set a valid env var name like ANTHROPIC_API_KEY"
                        ),
                    });
                }
                Some(env_name) if !is_valid_env_var_name(env_name) => {
                    findings.push(SemanticFinding {
                        code: SemanticFindingCode::ProviderEmptyApiKeyEnv,
                        severity: InvariantSeverity::Error,
                        path: format!("providers.{name}.api_key_env"),
                        message: format!(
                            "HTTP provider '{name}' has invalid api_key_env identifier; \
                             must match [A-Za-z_][A-Za-z0-9_]*"
                        ),
                    });
                }
                _ => {}
            }
        }

        // Rule: provider.invalid_base_url
        if let Some(ref url_str) = provider.base_url {
            let trimmed = url_str.trim();
            if !trimmed.is_empty()
                && let Err(reason) = validate_base_url(trimmed)
            {
                findings.push(SemanticFinding {
                    code: SemanticFindingCode::ProviderInvalidBaseUrl,
                    severity: InvariantSeverity::Error,
                    path: format!("providers.{name}.base_url"),
                    message: format!("provider '{name}': {reason}"),
                });
            }
        }

        // Rule: provider.invalid_timeout
        if let Some(0) = provider.timeout_ms {
            findings.push(SemanticFinding {
                code: SemanticFindingCode::ProviderInvalidTimeout,
                severity: InvariantSeverity::Error,
                path: format!("providers.{name}.timeout_ms"),
                message: format!("provider '{name}' has timeout_ms = 0; must be > 0"),
            });
        }
        if let Some(0) = provider.max_concurrent {
            findings.push(SemanticFinding {
                code: SemanticFindingCode::ProviderInvalidTimeout,
                severity: InvariantSeverity::Error,
                path: format!("providers.{name}.max_concurrent"),
                message: format!("provider '{name}' has max_concurrent = 0; must be > 0"),
            });
        }
    }

    // Model validation — sort for determinism.
    let mut model_names: Vec<&String> = config.models.keys().collect();
    model_names.sort();

    // Track slugs for ambiguity detection.
    let mut slug_owners: std::collections::HashMap<&str, Vec<&str>> =
        std::collections::HashMap::new();

    for name in &model_names {
        let profile = &config.models[*name];

        // Rule: model.unknown_provider
        if !config.providers.contains_key(&profile.provider) {
            findings.push(SemanticFinding {
                code: SemanticFindingCode::ModelUnknownProvider,
                severity: InvariantSeverity::Error,
                path: format!("models.{name}.provider"),
                message: format!(
                    "model '{name}' references provider '{}' which is not configured",
                    profile.provider
                ),
            });
        }

        // Rule: model.empty_slug
        if profile.slug.trim().is_empty() {
            findings.push(SemanticFinding {
                code: SemanticFindingCode::ModelEmptySlug,
                severity: InvariantSeverity::Error,
                path: format!("models.{name}.slug"),
                message: format!("model '{name}' has an empty slug"),
            });
        }

        // Rule: model.invalid_context
        if profile.context_window == 0 {
            findings.push(SemanticFinding {
                code: SemanticFindingCode::ModelInvalidContext,
                severity: InvariantSeverity::Error,
                path: format!("models.{name}.context_window"),
                message: format!("model '{name}' has context_window = 0; must be > 0"),
            });
        }
        if let Some(max_out) = profile.max_output {
            if max_out == 0 {
                findings.push(SemanticFinding {
                    code: SemanticFindingCode::ModelInvalidContext,
                    severity: InvariantSeverity::Error,
                    path: format!("models.{name}.max_output"),
                    message: format!("model '{name}' has max_output = 0; must be > 0"),
                });
            } else if max_out > profile.context_window {
                findings.push(SemanticFinding {
                    code: SemanticFindingCode::ModelInvalidContext,
                    severity: InvariantSeverity::Warning,
                    path: format!("models.{name}.max_output"),
                    message: format!(
                        "model '{name}' max_output ({max_out}) exceeds context_window ({})",
                        profile.context_window
                    ),
                });
            }
        }

        // Rule: model.invalid_tool_format
        let tf = profile.tool_format.trim();
        if !tf.is_empty() {
            // Accept any known ToolFormat variant name. Custom("...") values
            // contain a dot by convention; we accept those too.
            let known = matches!(
                tf,
                "openai_json"
                    | "anthropic_blocks"
                    | "hermes_json"
                    | "gemma4_tokens"
                    | "mistral_tokens"
                    | "pythonic"
                    | "qwen_xml"
                    | "react_text"
                    | "json_mode"
            );
            // Serde would also accept Custom(x) for any x containing a dot
            // or otherwise not matching a known variant.  We allow those as
            // intentional extensions.
            let is_custom_extension = !known
                && serde_json::from_value::<ToolFormat>(serde_json::Value::String(tf.to_string()))
                    .is_ok();

            if !known && !is_custom_extension {
                findings.push(SemanticFinding {
                    code: SemanticFindingCode::ModelInvalidToolFormat,
                    severity: InvariantSeverity::Error,
                    path: format!("models.{name}.tool_format"),
                    message: format!(
                        "model '{name}' has invalid tool_format '{}'; \
                         valid formats: openai_json, anthropic_blocks, hermes_json, \
                         gemma4_tokens, mistral_tokens, pythonic, qwen_xml, react_text, json_mode",
                        tf
                    ),
                });
            }
        }

        // Rule: provider.invalid_search_context (on model, not provider)
        if let Some(ref scs) = profile.search_context_size {
            let scs_trimmed = scs.trim();
            if !scs_trimmed.is_empty() && !matches!(scs_trimmed, "low" | "medium" | "high") {
                findings.push(SemanticFinding {
                    code: SemanticFindingCode::ProviderInvalidSearchContext,
                    severity: InvariantSeverity::Error,
                    path: format!("models.{name}.search_context_size"),
                    message: format!(
                        "model '{name}' has invalid search_context_size '{}'; \
                         must be exactly 'low', 'medium', or 'high'",
                        scs_trimmed
                    ),
                });
            }
        }

        // Collect slug for ambiguity check.
        let slug = profile.slug.trim();
        if !slug.is_empty() {
            slug_owners.entry(slug).or_default().push(name.as_str());
        }
    }

    // Rule: model.ambiguous_slug — check for duplicate slugs across providers.
    let mut ambiguous_slugs: Vec<&&str> = slug_owners
        .iter()
        .filter(|(_, owners)| owners.len() > 1)
        .map(|(slug, _)| slug)
        .collect();
    ambiguous_slugs.sort();
    for slug in ambiguous_slugs {
        let owners = &slug_owners[*slug];
        // Only report if the owners span different providers.
        let providers: std::collections::HashSet<&str> = owners
            .iter()
            .filter_map(|name| config.models.get(*name).map(|p| p.provider.as_str()))
            .collect();
        if providers.len() > 1 {
            findings.push(SemanticFinding {
                code: SemanticFindingCode::ModelAmbiguousSlug,
                severity: InvariantSeverity::Error,
                path: format!("models.*.slug={slug}"),
                message: format!(
                    "slug '{}' is used by models [{}] across different providers",
                    slug,
                    owners.join(", ")
                ),
            });
        }
    }

    // Rule: routing.unresolved_model — every model reference in agent/routing
    // config must resolve to a defined model key or a builtin.
    let effective_models = config.effective_models();
    let resolves = |model_ref: &str| -> bool {
        effective_models.contains_key(model_ref)
            || super::model_registry::builtin_model(model_ref).is_some()
    };

    let mut model_ref_checks: Vec<(&str, &str)> = Vec::new();

    let dm = config.agent.default_model.trim();
    if !dm.is_empty() {
        model_ref_checks.push(("agent.default_model", dm));
    }
    if let Some(ref fb) = config.agent.fallback_model {
        let fb = fb.trim();
        if !fb.is_empty() {
            model_ref_checks.push(("agent.fallback_model", fb));
        }
    }

    let fast = config.routing.fast_task_model.trim();
    if !fast.is_empty() {
        model_ref_checks.push(("routing.fast_task_model", fast));
    }
    let standard = config.routing.standard_task_model.trim();
    if !standard.is_empty() {
        model_ref_checks.push(("routing.standard_task_model", standard));
    }
    let complex = config.routing.complex_task_model.trim();
    if !complex.is_empty() {
        model_ref_checks.push(("routing.complex_task_model", complex));
    }

    // tier_models
    let mut tier_entries: Vec<(&String, &String)> = config.agent.tier_models.iter().collect();
    tier_entries.sort_by_key(|(k, _)| *k);

    for (tier, model_key) in &tier_entries {
        let mk = model_key.trim();
        if !mk.is_empty() && !resolves(mk) {
            findings.push(SemanticFinding {
                code: SemanticFindingCode::RoutingUnresolvedModel,
                severity: InvariantSeverity::Error,
                path: format!("agent.tier_models.{tier}"),
                message: format!("agent.tier_models.{tier} references unresolved model '{mk}'"),
            });
        }
    }

    for (field, model_ref) in &model_ref_checks {
        if !resolves(model_ref) {
            findings.push(SemanticFinding {
                code: SemanticFindingCode::RoutingUnresolvedModel,
                severity: InvariantSeverity::Error,
                path: field.to_string(),
                message: format!("{field} references unresolved model '{model_ref}'"),
            });
        }
    }

    findings
}

/// Check that a string is a valid environment variable identifier.
fn is_valid_env_var_name(name: &str) -> bool {
    let mut chars = name.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() || c == '_' => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// Validate a base URL string without pulling in a URL crate.
fn validate_base_url(url: &str) -> Result<(), String> {
    if !url.starts_with("http://") && !url.starts_with("https://") {
        return Err("scheme must be http or https".to_string());
    }
    let after_scheme = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))
        .unwrap_or(url);
    // Reject credentials (user:pass@host).
    if let Some(at_pos) = after_scheme.find('@') {
        // Only reject if @ appears before the first / (i.e., in authority).
        let slash_pos = after_scheme.find('/').unwrap_or(after_scheme.len());
        if at_pos < slash_pos {
            return Err("URL must not contain credentials/userinfo".to_string());
        }
    }
    // Must have a host (non-empty authority before / or end).
    let authority = after_scheme.split('/').next().unwrap_or("");
    let host = authority.split(':').next().unwrap_or("");
    if host.is_empty() {
        return Err("URL has no host".to_string());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use chrono::Duration;

    use super::*;

    fn valid_override(now: DateTime<Utc>) -> DangerousPermissionOverride {
        DangerousPermissionOverride {
            enabled: true,
            scope: "command:plan".to_string(),
            reason: "local smoke test".to_string(),
            expires_at: Some(now + Duration::hours(1)),
            ack_env: "ROKO_ACK_DANGEROUS_PERMISSIONS".to_string(),
            source: ConfigSource::LocalOverride,
        }
    }

    #[test]
    fn dangerously_skip_permissions_root_shared_config_fails() {
        let err = validate_strict_config_toml(
            "[runner]\ndangerously_skip_permissions = true\n",
            &StrictConfigSource::shared(Some(PathBuf::from("roko.toml"))),
        )
        .expect_err("shared config must reject dangerous skip");

        assert!(matches!(
            err,
            StrictConfigValidationError::DangerousRootPermission { .. }
        ));
    }

    #[test]
    fn dangerously_skip_permissions_false_or_absent_passes() {
        validate_strict_config_toml(
            "[runner]\ndangerously_skip_permissions = false\n",
            &StrictConfigSource::shared(Some(PathBuf::from("roko.toml"))),
        )
        .expect("false is allowed");

        validate_strict_config_toml(
            "[runner]\nplan_timeout_secs = 30\n",
            &StrictConfigSource::shared(None),
        )
        .expect("absent is allowed");
    }

    #[test]
    fn dangerously_skip_permissions_test_or_local_source_passes() {
        validate_strict_config_toml(
            "[runner]\ndangerously_skip_permissions = true\n",
            &StrictConfigSource::local_override(".roko/local-overrides.toml"),
        )
        .expect("local override source is explicit");

        validate_strict_config_toml(
            "[runner]\ndangerously_skip_permissions = true\n",
            &StrictConfigSource::test_fixture("tests/fixtures/dangerous.roko.toml"),
        )
        .expect("test fixture source is explicit");
    }

    #[test]
    fn dangerous_permission_override_accepts_valid_local_override() {
        let now = Utc::now();

        valid_override(now)
            .validate_at(now)
            .expect("valid local override should pass");
    }

    #[test]
    fn dangerous_permission_override_requires_reason() {
        let now = Utc::now();
        let mut override_policy = valid_override(now);
        override_policy.reason.clear();

        assert_eq!(
            override_policy.validate_at(now),
            Err(DangerousPermissionOverrideError::MissingReason)
        );
    }

    #[test]
    fn dangerous_permission_override_requires_scope() {
        let now = Utc::now();
        let mut override_policy = valid_override(now);
        override_policy.scope.clear();

        assert_eq!(
            override_policy.validate_at(now),
            Err(DangerousPermissionOverrideError::MissingScope)
        );
    }

    #[test]
    fn dangerous_permission_override_requires_future_expiry() {
        let now = Utc::now();
        let mut override_policy = valid_override(now);
        override_policy.expires_at = None;
        assert_eq!(
            override_policy.validate_at(now),
            Err(DangerousPermissionOverrideError::MissingExpiry)
        );

        override_policy.expires_at = Some(now - Duration::seconds(1));
        assert_eq!(
            override_policy.validate_at(now),
            Err(DangerousPermissionOverrideError::Expired)
        );
    }

    #[test]
    fn dangerous_permission_override_requires_local_source() {
        let now = Utc::now();
        let mut override_policy = valid_override(now);
        override_policy.source = ConfigSource::File;

        assert_eq!(
            override_policy.validate_at(now),
            Err(DangerousPermissionOverrideError::NonLocalSource)
        );
    }

    #[test]
    fn dangerous_permission_override_requires_ack_env() {
        let now = Utc::now();
        let mut override_policy = valid_override(now);
        override_policy.ack_env.clear();

        assert_eq!(
            override_policy.validate_at(now),
            Err(DangerousPermissionOverrideError::MissingAcknowledgementEnv)
        );
    }

    #[test]
    fn validate_invariants_rejects_budget_ordering() {
        let mut config = RokoConfig::default();
        config.budget.max_plan_usd = 5.0;
        config.budget.max_turn_usd = 6.0;

        let results = validate_invariants(&config);
        assert!(results.iter().any(|result| {
            result.invariant_id == 1 && result.severity == InvariantSeverity::Error
        }));
    }

    #[test]
    fn validate_invariants_treats_a_zero_turn_cap_as_no_cap() {
        // bug-8465a2: a budget with plan and task caps but no turn cap.
        let mut config = RokoConfig::default();
        config.budget.max_plan_usd = 10.0;
        config.budget.max_task_usd = 1.0;

        let results = validate_invariants(&config);
        assert!(
            !results.iter().any(|result| result.invariant_id == 1),
            "{results:?}"
        );
    }

    #[test]
    fn validate_invariants_rejects_missing_provider() {
        let mut config = RokoConfig::default();
        config.models.insert(
            "orphan".to_string(),
            super::super::provider::ModelProfile {
                provider: "missing".to_string(),
                slug: "orphan-v1".to_string(),
                ..Default::default()
            },
        );

        let results = validate_invariants(&config);
        assert!(results.iter().any(|result| {
            result.invariant_id == 3 && result.severity == InvariantSeverity::Error
        }));
    }

    #[test]
    fn validate_invariants_warns_on_parallel_capacity() {
        let mut config = RokoConfig::default();
        config.budget.max_plan_usd = 50.0;
        config.budget.max_turn_usd = 1.0;
        config.conductor.max_agents = 8;

        let results = validate_invariants(&config);
        assert!(results.iter().any(|result| {
            result.invariant_id == 4 && result.severity == InvariantSeverity::Warning
        }));
    }

    #[test]
    fn validate_invariants_accepts_default_config() {
        assert!(validate_invariants(&RokoConfig::default()).is_empty());
    }

    /// gap-b0d514: a data LLM that may call tools, or whose calls have no
    /// time or size bound, fails the config.
    #[test]
    fn validate_invariants_rejects_a_data_llm_with_tools_or_no_bounds() {
        use crate::config::DataLlmConfig;

        let mut config = RokoConfig::default();
        config.agent.data_llm = Some(DataLlmConfig::default());
        assert!(validate_invariants(&config).is_empty());

        config.agent.data_llm = Some(DataLlmConfig {
            strip_tool_calls: false,
            timeout_ms: 0,
            max_input_bytes: 0,
            ..DataLlmConfig::default()
        });
        let failed: Vec<_> = validate_invariants(&config)
            .into_iter()
            .filter(|result| result.invariant_id == 8)
            .collect();
        assert!(
            failed
                .iter()
                .all(|result| result.severity == InvariantSeverity::Error)
        );
        let paths: Vec<_> = failed
            .iter()
            .map(|result| result.config_path.as_str())
            .collect();
        assert_eq!(
            paths,
            [
                "agent.data_llm.strip_tool_calls",
                "agent.data_llm.timeout_ms",
                "agent.data_llm.max_input_bytes",
            ]
        );
    }

    // ---- unknown field detection tests ----

    #[test]
    fn detect_unknown_fields_clean_config() {
        let toml_text = r#"
            [budget]
            max_plan_usd = 10.0
            max_turn_usd = 0.5
        "#;
        let reports = detect_unknown_fields(toml_text);
        assert!(reports.is_empty(), "clean config should have no reports");
    }

    #[test]
    fn detect_unknown_fields_catches_typo() {
        let toml_text = r#"
            [budget]
            max_plan_usd = 10.0
            max_turn_usdd = 0.5
        "#;
        let reports = detect_unknown_fields(toml_text);
        assert!(
            !reports.is_empty(),
            "should detect unknown field 'max_turn_usdd'"
        );
        assert!(reports[0].path.contains("max_turn_usdd"));
    }

    #[test]
    fn detect_unknown_fields_catches_cold_storage_typo() {
        let toml_text = r#"
            [cold_storage]
            enabled = true
            max_agee_days = 7
        "#;
        let reports = detect_unknown_fields(toml_text);
        assert!(!reports.is_empty(), "should detect 'max_agee_days'");
        assert!(reports[0].path.contains("max_agee_days"));
    }

    #[test]
    fn detect_unknown_fields_nested_section() {
        let toml_text = r#"
            [serve.auth]
            enabled = true
            bogus_flag = true
        "#;
        let reports = detect_unknown_fields(toml_text);
        assert!(
            !reports.is_empty(),
            "should detect bogus_flag in serve.auth"
        );
    }

    #[test]
    fn detect_unknown_fields_timeouts_typo() {
        let toml_text = r#"
            [timeouts]
            agent_dispatch_secss = 600
        "#;
        let reports = detect_unknown_fields(toml_text);
        assert!(!reports.is_empty(), "should detect timeouts typo");
        // Should suggest the correct field
        if let Some(ref suggestion) = reports[0].suggestion {
            assert_eq!(suggestion, "agent_dispatch_secs");
        }
    }

    #[test]
    fn format_repair_suggestions_empty() {
        assert_eq!(
            format_repair_suggestions(&[]),
            "Config is clean: no unknown fields detected."
        );
    }

    #[test]
    fn format_repair_suggestions_with_suggestion() {
        let reports = vec![UnknownFieldReport {
            path: "timeouts.agent_dispatch_secss".to_string(),
            value: String::new(),
            suggestion: Some("agent_dispatch_secs".to_string()),
        }];
        let output = format_repair_suggestions(&reports);
        assert!(output.contains("did you mean"));
        assert!(output.contains("agent_dispatch_secs"));
        assert!(output.contains("remove this field"));
    }

    #[test]
    fn levenshtein_basic() {
        assert_eq!(levenshtein("abc", "abc"), 0);
        assert_eq!(levenshtein("abc", "abd"), 1);
        assert_eq!(levenshtein("", "abc"), 3);
        assert_eq!(levenshtein("abc", ""), 3);
        assert_eq!(levenshtein("kitten", "sitting"), 3);
    }

    // ---- provider/model semantic validation tests (backlog #370) ----

    use super::super::provider::{ModelProfile, ProviderConfig};
    use crate::agent::ProviderKind;

    fn api_provider(name: &str) -> (String, ProviderConfig) {
        (
            name.to_string(),
            ProviderConfig {
                kind: ProviderKind::AnthropicApi,
                api_key_env: Some("ANTHROPIC_API_KEY".to_string()),
                ..Default::default()
            },
        )
    }

    #[test]
    fn semantic_empty_api_key_env_is_error() {
        let mut config = RokoConfig::default();
        config.providers.clear();
        config.models.clear();
        config.providers.insert(
            "bad".to_string(),
            ProviderConfig {
                kind: ProviderKind::AnthropicApi,
                api_key_env: Some(String::new()),
                ..Default::default()
            },
        );

        let findings = validate_provider_semantics(&config);
        assert!(
            findings
                .iter()
                .any(|f| f.code == SemanticFindingCode::ProviderEmptyApiKeyEnv
                    && f.severity == InvariantSeverity::Error)
        );
    }

    #[test]
    fn semantic_invalid_env_var_name() {
        let mut config = RokoConfig::default();
        config.providers.clear();
        config.models.clear();
        config.providers.insert(
            "bad".to_string(),
            ProviderConfig {
                kind: ProviderKind::OpenAiCompat,
                api_key_env: Some("123INVALID".to_string()),
                ..Default::default()
            },
        );

        let findings = validate_provider_semantics(&config);
        assert!(
            findings
                .iter()
                .any(|f| f.code == SemanticFindingCode::ProviderEmptyApiKeyEnv)
        );
    }

    #[test]
    fn semantic_cli_provider_no_api_key_env_is_ok() {
        let mut config = RokoConfig::default();
        config.providers.clear();
        config.models.clear();
        config.providers.insert(
            "cli".to_string(),
            ProviderConfig {
                kind: ProviderKind::ClaudeCli,
                api_key_env: None,
                ..Default::default()
            },
        );

        let findings = validate_provider_semantics(&config);
        assert!(
            findings
                .iter()
                .all(|f| f.code != SemanticFindingCode::ProviderEmptyApiKeyEnv)
        );
    }

    #[test]
    fn semantic_invalid_base_url_no_scheme() {
        let mut config = RokoConfig::default();
        config.providers.clear();
        config.models.clear();
        let (name, mut prov) = api_provider("bad");
        prov.base_url = Some("example.com/v1".to_string());
        config.providers.insert(name, prov);

        let findings = validate_provider_semantics(&config);
        assert!(
            findings
                .iter()
                .any(|f| f.code == SemanticFindingCode::ProviderInvalidBaseUrl)
        );
    }

    #[test]
    fn semantic_base_url_with_credentials_rejected() {
        let mut config = RokoConfig::default();
        config.providers.clear();
        config.models.clear();
        let (name, mut prov) = api_provider("bad");
        prov.base_url = Some("https://user:pass@example.com/v1".to_string());
        config.providers.insert(name, prov);

        let findings = validate_provider_semantics(&config);
        assert!(
            findings
                .iter()
                .any(|f| f.code == SemanticFindingCode::ProviderInvalidBaseUrl
                    && f.message.contains("credentials"))
        );
    }

    #[test]
    fn semantic_valid_base_url_accepted() {
        let mut config = RokoConfig::default();
        config.providers.clear();
        config.models.clear();
        let (name, mut prov) = api_provider("good");
        prov.base_url = Some("https://api.example.com/v1".to_string());
        config.providers.insert(name, prov);

        let findings = validate_provider_semantics(&config);
        assert!(
            findings
                .iter()
                .all(|f| f.code != SemanticFindingCode::ProviderInvalidBaseUrl)
        );
    }

    #[test]
    fn semantic_zero_timeout_is_error() {
        let mut config = RokoConfig::default();
        config.providers.clear();
        config.models.clear();
        let (name, mut prov) = api_provider("slow");
        prov.timeout_ms = Some(0);
        config.providers.insert(name, prov);

        let findings = validate_provider_semantics(&config);
        assert!(
            findings
                .iter()
                .any(|f| f.code == SemanticFindingCode::ProviderInvalidTimeout)
        );
    }

    #[test]
    fn semantic_model_unknown_provider() {
        let mut config = RokoConfig::default();
        config.providers.clear();
        config.models.clear();
        config.models.insert(
            "orphan".to_string(),
            ModelProfile {
                provider: "nonexistent".to_string(),
                slug: "model-v1".to_string(),
                ..Default::default()
            },
        );

        let findings = validate_provider_semantics(&config);
        assert!(
            findings
                .iter()
                .any(|f| f.code == SemanticFindingCode::ModelUnknownProvider)
        );
    }

    #[test]
    fn semantic_model_empty_slug() {
        let mut config = RokoConfig::default();
        config.providers.clear();
        config.models.clear();
        let (pname, prov) = api_provider("anthropic");
        config.providers.insert(pname, prov);
        config.models.insert(
            "bad".to_string(),
            ModelProfile {
                provider: "anthropic".to_string(),
                slug: String::new(),
                ..Default::default()
            },
        );

        let findings = validate_provider_semantics(&config);
        assert!(
            findings
                .iter()
                .any(|f| f.code == SemanticFindingCode::ModelEmptySlug)
        );
    }

    #[test]
    fn semantic_model_zero_context_window() {
        let mut config = RokoConfig::default();
        config.providers.clear();
        config.models.clear();
        let (pname, prov) = api_provider("anthropic");
        config.providers.insert(pname, prov);
        config.models.insert(
            "bad".to_string(),
            ModelProfile {
                provider: "anthropic".to_string(),
                slug: "model-v1".to_string(),
                context_window: 0,
                ..Default::default()
            },
        );

        let findings = validate_provider_semantics(&config);
        assert!(
            findings
                .iter()
                .any(|f| f.code == SemanticFindingCode::ModelInvalidContext)
        );
    }

    #[test]
    fn semantic_model_max_output_exceeds_context() {
        let mut config = RokoConfig::default();
        config.providers.clear();
        config.models.clear();
        let (pname, prov) = api_provider("anthropic");
        config.providers.insert(pname, prov);
        config.models.insert(
            "big".to_string(),
            ModelProfile {
                provider: "anthropic".to_string(),
                slug: "model-v1".to_string(),
                context_window: 4096,
                max_output: Some(8192),
                ..Default::default()
            },
        );

        let findings = validate_provider_semantics(&config);
        assert!(
            findings
                .iter()
                .any(|f| f.code == SemanticFindingCode::ModelInvalidContext
                    && f.severity == InvariantSeverity::Warning)
        );
    }

    #[test]
    fn semantic_ambiguous_slug_across_providers() {
        let mut config = RokoConfig::default();
        config.providers.clear();
        config.models.clear();
        config.providers.insert(
            "prov_a".to_string(),
            ProviderConfig {
                kind: ProviderKind::AnthropicApi,
                api_key_env: Some("KEY_A".to_string()),
                ..Default::default()
            },
        );
        config.providers.insert(
            "prov_b".to_string(),
            ProviderConfig {
                kind: ProviderKind::OpenAiCompat,
                api_key_env: Some("KEY_B".to_string()),
                ..Default::default()
            },
        );
        config.models.insert(
            "model_a".to_string(),
            ModelProfile {
                provider: "prov_a".to_string(),
                slug: "shared-slug".to_string(),
                ..Default::default()
            },
        );
        config.models.insert(
            "model_b".to_string(),
            ModelProfile {
                provider: "prov_b".to_string(),
                slug: "shared-slug".to_string(),
                ..Default::default()
            },
        );

        let findings = validate_provider_semantics(&config);
        assert!(
            findings
                .iter()
                .any(|f| f.code == SemanticFindingCode::ModelAmbiguousSlug)
        );
    }

    #[test]
    fn semantic_same_slug_same_provider_is_ok() {
        let mut config = RokoConfig::default();
        config.providers.clear();
        config.models.clear();
        let (pname, prov) = api_provider("anthropic");
        config.providers.insert(pname, prov);
        config.models.insert(
            "alias_a".to_string(),
            ModelProfile {
                provider: "anthropic".to_string(),
                slug: "claude-sonnet".to_string(),
                ..Default::default()
            },
        );
        config.models.insert(
            "alias_b".to_string(),
            ModelProfile {
                provider: "anthropic".to_string(),
                slug: "claude-sonnet".to_string(),
                ..Default::default()
            },
        );

        let findings = validate_provider_semantics(&config);
        assert!(
            findings
                .iter()
                .all(|f| f.code != SemanticFindingCode::ModelAmbiguousSlug)
        );
    }

    #[test]
    fn semantic_default_config_is_clean() {
        // Default config may have built-in providers/models, should not error.
        let config = RokoConfig::default();
        let findings = validate_provider_semantics(&config);
        let errors: Vec<_> = findings
            .iter()
            .filter(|f| f.severity == InvariantSeverity::Error)
            .collect();
        assert!(
            errors.is_empty(),
            "default config should have no errors: {errors:?}"
        );
    }

    #[test]
    fn is_valid_env_var_name_cases() {
        assert!(is_valid_env_var_name("ANTHROPIC_API_KEY"));
        assert!(is_valid_env_var_name("_KEY"));
        assert!(is_valid_env_var_name("a"));
        assert!(!is_valid_env_var_name(""));
        assert!(!is_valid_env_var_name("123"));
        assert!(!is_valid_env_var_name("KEY WITH SPACE"));
    }

    #[test]
    fn validate_base_url_cases() {
        assert!(validate_base_url("https://api.example.com/v1").is_ok());
        assert!(validate_base_url("http://localhost:8080").is_ok());
        assert!(validate_base_url("ftp://example.com").is_err());
        assert!(validate_base_url("not-a-url").is_err());
        assert!(validate_base_url("https://user:pass@host.com").is_err());
        assert!(validate_base_url("https://").is_err());
    }

    // ── model.invalid_tool_format tests ──────────────────────────────

    #[test]
    fn semantic_invalid_tool_format_rejected() {
        let mut config = RokoConfig::default();
        config.providers.clear();
        config.models.clear();
        let (pname, prov) = api_provider("anthropic");
        config.providers.insert(pname, prov);
        config.models.insert(
            "bad".to_string(),
            ModelProfile {
                provider: "anthropic".to_string(),
                slug: "model-v1".to_string(),
                tool_format: "not_a_real_format".to_string(),
                ..Default::default()
            },
        );

        let findings = validate_provider_semantics(&config);
        assert!(
            findings
                .iter()
                .any(|f| f.code == SemanticFindingCode::ModelInvalidToolFormat),
            "expected invalid_tool_format finding, got: {findings:?}"
        );
    }

    #[test]
    fn semantic_valid_tool_formats_accepted() {
        let formats = [
            "openai_json",
            "anthropic_blocks",
            "hermes_json",
            "gemma4_tokens",
            "mistral_tokens",
            "pythonic",
            "qwen_xml",
            "react_text",
            "json_mode",
        ];
        for fmt in formats {
            let mut config = RokoConfig::default();
            config.providers.clear();
            config.models.clear();
            let (pname, prov) = api_provider("anthropic");
            config.providers.insert(pname, prov);
            config.models.insert(
                "good".to_string(),
                ModelProfile {
                    provider: "anthropic".to_string(),
                    slug: "model-v1".to_string(),
                    tool_format: fmt.to_string(),
                    ..Default::default()
                },
            );

            let findings = validate_provider_semantics(&config);
            assert!(
                findings
                    .iter()
                    .all(|f| f.code != SemanticFindingCode::ModelInvalidToolFormat),
                "tool_format '{fmt}' should be accepted, got: {findings:?}"
            );
        }
    }

    // ── provider.invalid_search_context tests ────────────────────────

    #[test]
    fn semantic_invalid_search_context_rejected() {
        let mut config = RokoConfig::default();
        config.providers.clear();
        config.models.clear();
        let (pname, prov) = api_provider("perplexity");
        config.providers.insert(pname, prov);
        config.models.insert(
            "sonar".to_string(),
            ModelProfile {
                provider: "perplexity".to_string(),
                slug: "sonar-pro".to_string(),
                search_context_size: Some("huge".to_string()),
                ..Default::default()
            },
        );

        let findings = validate_provider_semantics(&config);
        assert!(
            findings
                .iter()
                .any(|f| f.code == SemanticFindingCode::ProviderInvalidSearchContext),
            "expected invalid_search_context finding, got: {findings:?}"
        );
    }

    #[test]
    fn semantic_valid_search_context_accepted() {
        for ctx in ["low", "medium", "high"] {
            let mut config = RokoConfig::default();
            config.providers.clear();
            config.models.clear();
            let (pname, prov) = api_provider("perplexity");
            config.providers.insert(pname, prov);
            config.models.insert(
                "sonar".to_string(),
                ModelProfile {
                    provider: "perplexity".to_string(),
                    slug: "sonar-pro".to_string(),
                    search_context_size: Some(ctx.to_string()),
                    ..Default::default()
                },
            );

            let findings = validate_provider_semantics(&config);
            assert!(
                findings
                    .iter()
                    .all(|f| f.code != SemanticFindingCode::ProviderInvalidSearchContext),
                "search_context_size '{ctx}' should be accepted, got: {findings:?}"
            );
        }
    }

    #[test]
    fn semantic_none_search_context_is_ok() {
        let mut config = RokoConfig::default();
        config.providers.clear();
        config.models.clear();
        let (pname, prov) = api_provider("perplexity");
        config.providers.insert(pname, prov);
        config.models.insert(
            "sonar".to_string(),
            ModelProfile {
                provider: "perplexity".to_string(),
                slug: "sonar-pro".to_string(),
                search_context_size: None,
                ..Default::default()
            },
        );

        let findings = validate_provider_semantics(&config);
        assert!(
            findings
                .iter()
                .all(|f| f.code != SemanticFindingCode::ProviderInvalidSearchContext),
            "None search_context_size should be accepted, got: {findings:?}"
        );
    }

    // ── routing.unresolved_model tests ───────────────────────────────

    #[test]
    fn semantic_unresolved_routing_model_rejected() {
        let mut config = RokoConfig::default();
        config.providers.clear();
        config.models.clear();
        config.routing.fast_task_model = "nonexistent-fast".to_string();

        let findings = validate_provider_semantics(&config);
        assert!(
            findings
                .iter()
                .any(|f| f.code == SemanticFindingCode::RoutingUnresolvedModel
                    && f.path == "routing.fast_task_model"),
            "expected unresolved_model for fast_task_model, got: {findings:?}"
        );
    }

    #[test]
    fn semantic_builtin_routing_model_accepted() {
        let mut config = RokoConfig::default();
        config.providers.clear();
        config.models.clear();
        // Default routing models are builtin slugs, should be accepted.
        config.routing.fast_task_model = "claude-haiku-4-5".to_string();
        config.routing.standard_task_model = "claude-sonnet-4-6".to_string();
        config.routing.complex_task_model = "claude-opus-4-6".to_string();

        let findings = validate_provider_semantics(&config);
        assert!(
            findings
                .iter()
                .all(|f| f.code != SemanticFindingCode::RoutingUnresolvedModel),
            "builtin model slugs should not produce unresolved_model: {findings:?}"
        );
    }

    #[test]
    fn semantic_explicit_routing_model_accepted() {
        let mut config = RokoConfig::default();
        config.providers.clear();
        config.models.clear();
        let (pname, prov) = api_provider("anthropic");
        config.providers.insert(pname, prov);
        config.models.insert(
            "my-fast".to_string(),
            ModelProfile {
                provider: "anthropic".to_string(),
                slug: "my-fast-slug".to_string(),
                ..Default::default()
            },
        );
        config.routing.fast_task_model = "my-fast".to_string();

        let findings = validate_provider_semantics(&config);
        assert!(
            findings
                .iter()
                .all(|f| f.code != SemanticFindingCode::RoutingUnresolvedModel
                    || f.path != "routing.fast_task_model"),
            "explicit model key should resolve, got: {findings:?}"
        );
    }

    #[test]
    fn semantic_unresolved_tier_model_rejected() {
        let mut config = RokoConfig::default();
        config.providers.clear();
        config.models.clear();
        config
            .agent
            .tier_models
            .insert("mechanical".to_string(), "nonexistent-tier".to_string());

        let findings = validate_provider_semantics(&config);
        assert!(
            findings
                .iter()
                .any(|f| f.code == SemanticFindingCode::RoutingUnresolvedModel
                    && f.path == "agent.tier_models.mechanical"),
            "expected unresolved_model for tier_models, got: {findings:?}"
        );
    }

    #[test]
    fn semantic_unresolved_default_model_rejected() {
        let mut config = RokoConfig::default();
        config.providers.clear();
        config.models.clear();
        config.agent.default_model = "totally-fake-model".to_string();

        let findings = validate_provider_semantics(&config);
        assert!(
            findings
                .iter()
                .any(|f| f.code == SemanticFindingCode::RoutingUnresolvedModel
                    && f.path == "agent.default_model"),
            "expected unresolved_model for default_model, got: {findings:?}"
        );
    }
}
