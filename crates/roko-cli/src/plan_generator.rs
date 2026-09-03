//! Shared `PlanGenerator` service — consolidates PRD, prompt, backlog, and
//! replan plan-generation behind one RuntimeServices-backed,
//! model/role/tool-policy-aware contract.
//!
//! This module owns:
//! - Request/outcome types with source, role, overrides, tool policy, budget,
//!   provenance, and validation evidence.
//! - The extract-validate-repair pipeline (reusing `plan_generate` prompt
//!   builders, `task_parser::repair_toml`, and `validate_and_fix_generated_plan`).
//! - Bounded retry with model escalation and a configurable repair cap.
//! - The adapter contract (`PlanGeneratorAdapter`) that each host implements
//!   for persistence, execution, and rendering.
//!
//! This module does NOT:
//! - Execute or persist plans (that is the adapter's job).
//! - Edit any current call site (that is #283's job).

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use anyhow::{Result, anyhow};
use indexmap::IndexMap;
use roko_core::config::schema::ModelProfile;
use roko_learn::runtime_feedback::{ArtifactValidationReport, GenerationOutcome};
use serde::{Deserialize, Serialize};

use crate::plan_generate::PlanTemplateKind;
use crate::plan_policy::PlanExecutionPolicy;
use crate::task_parser::TasksFile;

// ---------------------------------------------------------------------------
// Call-site manifest (§ "Mechanical Call-Site Manifest" from #280)
// ---------------------------------------------------------------------------

/// Each row from the #280 manifest. Callers reference these keys to identify
/// themselves when building adapter implementations for #283.
pub mod adapter_keys {
    /// `prd.rs::generate_plan_from_prd` — default PRD path.
    pub const PRD_DEFAULT: &str = "prd_default";
    /// `generate_plan_from_prd_with_model` — PRD + explicit model override.
    pub const PRD_MODEL: &str = "prd_model";
    /// `generate_plan_from_prd_with_failure_context` — PRD + gate-failure context.
    pub const PRD_REPLAN: &str = "prd_replan";
    /// `commands/plan.rs` — plan generate from prompt or file.
    pub const PLAN_GENERATE: &str = "plan_generate";
    /// `commands/do_cmd.rs` — direct plan-generation path.
    pub const DO_STANDARD: &str = "do_standard";
    /// `do_cmd.rs` — PRD-first path.
    pub const DO_COMPLEX: &str = "do_complex";
    /// `serve_runtime.rs::generate_plan_from_prd` (CLI-side).
    pub const CLI_SERVE_RUNTIME: &str = "cli_serve_runtime";
    /// `roko-serve/src/runtime.rs::generate_plan_from_prd` + `job_runner.rs`.
    pub const SERVE_RUNTIME: &str = "serve_runtime";
    /// `roko-serve/src/routes/plans.rs::generate_plan`.
    pub const SERVE_HTTP: &str = "serve_http";
    /// `runner/event_loop.rs::build_gate_failure_plan_revision` —
    /// owned by #252/#275, never a direct provider call after migration.
    pub const GATE_REPLAN: &str = "gate_replan";
}

// ---------------------------------------------------------------------------
// Request / Outcome types
// ---------------------------------------------------------------------------

/// Where the generation request originates.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlanSource {
    /// Generated from a published PRD file.
    Prd {
        /// PRD slug (used as plan directory name).
        slug: String,
        /// Path to the PRD markdown file.
        prd_path: PathBuf,
    },
    /// Generated from a user-supplied prompt string.
    Prompt {
        /// Freeform prompt text.
        prompt: String,
    },
    /// Generated from a local file (notes, spec, etc.).
    File {
        /// Path to the source file.
        path: PathBuf,
    },
    /// Replan: previous generation failed and this is a corrective pass.
    Replan {
        /// Original PRD slug.
        slug: String,
        /// Path to the PRD file.
        prd_path: PathBuf,
        /// Failure context injected into the system prompt.
        failure_context: String,
    },
}

impl PlanSource {
    /// Canonical slug for plan directory naming.
    #[must_use]
    pub fn slug(&self) -> &str {
        match self {
            Self::Prd { slug, .. } | Self::Replan { slug, .. } => slug,
            Self::Prompt { .. } => "prompt",
            Self::File { path } => path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("file"),
        }
    }

    /// Whether this source carries failure context for replanning.
    #[must_use]
    pub fn has_failure_context(&self) -> bool {
        matches!(self, Self::Replan { .. })
    }

    /// Failure context text, if present.
    #[must_use]
    pub fn failure_context(&self) -> Option<&str> {
        match self {
            Self::Replan {
                failure_context, ..
            } => Some(failure_context.as_str()),
            _ => None,
        }
    }
}

/// Model/role/tool-policy overrides the caller can supply.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PlanGeneratorOverrides {
    /// Explicit model key (skips cascade routing selection).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// Override the default `"strategist"` role.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    /// Allowed tools for the generation agent (comma-separated).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub allowed_tools: Option<String>,
    /// Plan template kind override.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub template: Option<String>,
    /// Maximum USD budget for the generation call.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub budget_usd: Option<f64>,
    /// Maximum repair/retry attempts (defaults to [`DEFAULT_REPAIR_CAP`]).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repair_cap: Option<u32>,
}

/// The canonical input to `PlanGenerator::generate`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanGeneratorRequest {
    /// Where the plan content originates.
    pub source: PlanSource,
    /// Workspace root directory.
    pub workdir: PathBuf,
    /// Caller-specific adapter key from [`adapter_keys`].
    pub adapter_key: String,
    /// Optional overrides for model, role, tools, template, budget.
    #[serde(default)]
    pub overrides: PlanGeneratorOverrides,
    /// Whether this is a dry-run (no persistence).
    #[serde(default)]
    pub dry_run: bool,
}

/// Validation evidence produced by the extraction pipeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationEvidence {
    /// Number of TOML extraction attempts.
    pub extraction_attempts: u32,
    /// Whether model escalation was triggered.
    pub model_escalated: bool,
    /// Final model used for generation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub final_model: Option<String>,
    /// Repair operations applied to the TOML.
    pub repairs_applied: Vec<String>,
    /// Policy violations detected (may be empty if all passed).
    pub policy_violations: Vec<String>,
}

/// The normalized outcome from a generation request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanGeneratorOutcome {
    /// The validated tasks.toml content (absent on total failure).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tasks_toml: Option<String>,
    /// Optional plan.md narrative content.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plan_md: Option<String>,
    /// Slug used for the plan directory.
    pub slug: String,
    /// Generation outcome (process + artifact status).
    pub outcome: GenerationOutcome,
    /// Validation evidence from the extraction pipeline.
    pub evidence: ValidationEvidence,
    /// Task count in the generated plan (0 on failure).
    pub task_count: usize,
    /// Estimated complexity label.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub estimated_complexity: Option<String>,
    /// Provenance: which adapter key triggered this generation.
    pub adapter_key: String,
}

impl PlanGeneratorOutcome {
    /// True only when generation produced a valid, policy-conforming plan.
    #[must_use]
    pub fn is_success(&self) -> bool {
        self.tasks_toml.is_some() && self.outcome.fully_successful()
    }

    /// True when the process ran but the artifact failed validation.
    #[must_use]
    pub fn is_partial(&self) -> bool {
        self.outcome.process_success && !self.outcome.artifact_valid
    }
}

// ---------------------------------------------------------------------------
// Adapter trait (host callback contract for #283)
// ---------------------------------------------------------------------------

/// Contract that each call-site adapter implements for #283.
///
/// The `PlanGenerator` service owns generation, repair, validation, and
/// normalized outcome. Each adapter owns:
/// - Source loading and authorization
/// - Persistence destination
/// - Execution trigger
/// - User rendering / progress reporting
///
/// This trait is object-safe so adapters can be boxed.
pub trait PlanGeneratorAdapter: Send + Sync {
    /// Adapter key from [`adapter_keys`].
    fn adapter_key(&self) -> &str;

    /// Persist the generated plan to disk or storage.
    ///
    /// Called only when the outcome contains valid `tasks_toml`.
    /// Returns the path where the plan was written.
    fn persist(
        &self,
        outcome: &PlanGeneratorOutcome,
        workdir: &Path,
    ) -> Result<PathBuf>;

    /// Optional post-generation execution trigger.
    ///
    /// Called after successful persistence. Adapters that auto-execute plans
    /// implement this; others return `Ok(())`.
    fn on_persisted(
        &self,
        _outcome: &PlanGeneratorOutcome,
        _plan_path: &Path,
    ) -> Result<()> {
        Ok(())
    }

    /// Report progress or status to the user.
    ///
    /// Called at various stages of generation for adapters that render UI.
    fn report(&self, _message: &str) {}
}

// ---------------------------------------------------------------------------
// Default repair cap
// ---------------------------------------------------------------------------

/// Default maximum repair/extraction retry attempts.
pub const DEFAULT_REPAIR_CAP: u32 = 2;

/// Default escalation chain: haiku -> sonnet -> opus.
const DEFAULT_ESCALATION_CHAIN: &[&str] =
    &["claude-haiku-4-5", "claude-sonnet-4-6", "claude-opus-4-6"];

// ---------------------------------------------------------------------------
// PlanGenerator service
// ---------------------------------------------------------------------------

/// The shared plan-generation service.
///
/// Encapsulates the extract-validate-repair pipeline that all callers share.
/// Does NOT execute, persist, or render — those are adapter responsibilities.
pub struct PlanGenerator {
    /// Configured model profiles (for escalation filtering).
    configured_models: HashSet<String>,
    /// Tier model overrides from config.
    tier_models: HashMap<String, String>,
    /// Whether model escalation is enabled.
    escalation_enabled: bool,
    /// Model profiles for validation.
    model_profiles: IndexMap<String, ModelProfile>,
    /// Default model from config.
    default_model: Option<String>,
}

impl PlanGenerator {
    /// Create a new `PlanGenerator` from resolved workspace config.
    #[must_use]
    pub fn new(
        model_profiles: IndexMap<String, ModelProfile>,
        tier_models: HashMap<String, String>,
        escalation_enabled: bool,
        default_model: Option<String>,
    ) -> Self {
        let configured_models: HashSet<String> = model_profiles
            .iter()
            .flat_map(|(key, profile)| {
                std::iter::once(key.clone()).chain(std::iter::once(profile.slug.clone()))
            })
            .collect();

        Self {
            configured_models,
            tier_models,
            escalation_enabled,
            model_profiles,
            default_model,
        }
    }

    /// Validate raw TOML output from a generation agent.
    ///
    /// Applies the full extract-validate-repair pipeline:
    /// 1. Extract fenced TOML block from agent output
    /// 2. Apply deterministic TOML repair (`task_parser::repair_toml`)
    /// 3. Validate structural fields (meta, task, verify)
    /// 4. Fix known LLM artifacts (model_hint removal, placeholder replacement,
    ///    auto-verify insertion)
    /// 5. Validate against plan policy budgets
    ///
    /// Returns the validated TOML string on success, or an error description.
    pub fn validate_raw_output(
        &self,
        raw_output: &str,
        slug: &str,
        template_kind: PlanTemplateKind,
    ) -> std::result::Result<ValidatedPlan, String> {
        // 1. Extract fenced block
        let toml_content = extract_fenced_block(raw_output, "toml")
            .or_else(|| extract_fenced_block(raw_output, "tasks.toml"))
            .or_else(|| extract_toml_content_fallback(raw_output));

        let toml_content = toml_content
            .ok_or_else(|| "no TOML block found in agent output".to_string())?;

        // 2. Structural pre-check
        if !toml_content.contains("[meta]") {
            return Err("TOML block is missing the required [meta] section".to_string());
        }
        if !toml_content.contains("[[task]]") {
            return Err("TOML block is missing required [[task]] entries".to_string());
        }

        // 3. Full validation and repair
        let mut repairs = Vec::new();

        let repaired = crate::task_parser::repair_toml(toml_content);
        if repaired != toml_content {
            repairs.push("deterministic TOML repair".to_string());
        }

        let validated = validate_and_fix_plan_toml(
            &repaired,
            slug,
            &self.model_profiles,
            self.default_model.as_deref(),
            &mut repairs,
        )
        .map_err(|e| format!("{e:#}"))?;

        // 4. Runtime parsing check
        let parsed = TasksFile::parse_str(&validated).map_err(|error| {
            format!("generated plan failed runtime parsing after repair: {error:#}")
        })?;

        // 5. Policy budget validation
        let policy = PlanExecutionPolicy::generated_for_environment(
            template_kind.max_task_count(),
        );
        let policy_issues = crate::plan_policy::validate_plan_budgets(&parsed, policy);
        if !policy_issues.is_empty() {
            return Err(format!(
                "generated plan violates the `{}` structural budget:\n{}",
                template_kind.label(),
                policy_issues
                    .iter()
                    .map(|issue| format!("  - {issue}"))
                    .collect::<Vec<_>>()
                    .join("\n")
            ));
        }

        // 6. Extract plan.md if present
        let plan_md = extract_fenced_block(raw_output, "plan.md")
            .or_else(|| extract_fenced_block(raw_output, "markdown"))
            .or_else(|| extract_fenced_block(raw_output, "md"))
            .map(String::from);

        Ok(ValidatedPlan {
            tasks_toml: validated,
            plan_md,
            task_count: parsed.tasks.len(),
            repairs,
            policy_violations: Vec::new(),
        })
    }

    /// Return the next-tier model for escalation on validation failures.
    ///
    /// Returns `None` if escalation is disabled, the current model is at the
    /// highest tier, or no configured model is available at a higher tier.
    #[must_use]
    pub fn next_escalation_model(&self, current: Option<&str>) -> Option<String> {
        if !self.escalation_enabled {
            return None;
        }
        next_tier_model(current, &self.tier_models, &self.configured_models)
    }

    /// Effective repair cap from overrides or default.
    #[must_use]
    pub fn effective_repair_cap(overrides: &PlanGeneratorOverrides) -> u32 {
        overrides.repair_cap.unwrap_or(DEFAULT_REPAIR_CAP)
    }

    /// Resolve the effective template kind from overrides or PRD metadata.
    #[must_use]
    pub fn resolve_template(overrides: &PlanGeneratorOverrides, prd_template: Option<&str>) -> PlanTemplateKind {
        let template_name = overrides.template.as_deref().or(prd_template);
        PlanTemplateKind::resolve(template_name)
    }

    /// Build a `PlanGeneratorOutcome` for a successful generation.
    #[must_use]
    pub fn success_outcome(
        request: &PlanGeneratorRequest,
        validated: ValidatedPlan,
        evidence: ValidationEvidence,
        validation_report: Option<ArtifactValidationReport>,
    ) -> PlanGeneratorOutcome {
        PlanGeneratorOutcome {
            tasks_toml: Some(validated.tasks_toml),
            plan_md: validated.plan_md,
            slug: request.source.slug().to_string(),
            outcome: GenerationOutcome {
                process_success: true,
                artifact_valid: true,
                validation_report,
            },
            evidence,
            task_count: validated.task_count,
            estimated_complexity: None,
            adapter_key: request.adapter_key.clone(),
        }
    }

    /// Build a `PlanGeneratorOutcome` for a failed generation.
    #[must_use]
    pub fn failure_outcome(
        request: &PlanGeneratorRequest,
        evidence: ValidationEvidence,
    ) -> PlanGeneratorOutcome {
        PlanGeneratorOutcome {
            tasks_toml: None,
            plan_md: None,
            slug: request.source.slug().to_string(),
            outcome: GenerationOutcome {
                process_success: false,
                artifact_valid: false,
                validation_report: None,
            },
            evidence,
            task_count: 0,
            estimated_complexity: None,
            adapter_key: request.adapter_key.clone(),
        }
    }

    /// Build a `PlanGeneratorOutcome` for a partial success (process ran but
    /// artifact validation failed).
    #[must_use]
    pub fn partial_outcome(
        request: &PlanGeneratorRequest,
        evidence: ValidationEvidence,
    ) -> PlanGeneratorOutcome {
        PlanGeneratorOutcome {
            tasks_toml: None,
            plan_md: None,
            slug: request.source.slug().to_string(),
            outcome: GenerationOutcome {
                process_success: true,
                artifact_valid: false,
                validation_report: None,
            },
            evidence,
            task_count: 0,
            estimated_complexity: None,
            adapter_key: request.adapter_key.clone(),
        }
    }
}

/// Intermediate result from `validate_raw_output`.
#[derive(Debug, Clone)]
pub struct ValidatedPlan {
    /// The validated and repaired TOML content.
    pub tasks_toml: String,
    /// Optional plan.md narrative.
    pub plan_md: Option<String>,
    /// Number of tasks in the plan.
    pub task_count: usize,
    /// Repairs applied during validation.
    pub repairs: Vec<String>,
    /// Policy violations (empty if all passed).
    pub policy_violations: Vec<String>,
}

// ---------------------------------------------------------------------------
// Internal helpers (extracted from prd.rs for reuse)
// ---------------------------------------------------------------------------

// Known field sets for validation.
const KNOWN_META_FIELDS: &[&str] = &[
    "plan",
    "iteration",
    "total",
    "done",
    "status",
    "max_parallel",
    "estimated_total_minutes",
    "skip_enrichment",
];

const REQUIRED_META_FIELDS: &[&str] = &["plan", "total", "status"];

const KNOWN_TASK_FIELDS: &[&str] = &[
    "id",
    "title",
    "description",
    "role",
    "status",
    "tier",
    "max_loc",
    "files",
    "allowed_tools",
    "denied_tools",
    "mcp_servers",
    "depends_on",
    "context",
    "verify",
    "model_hint",
    "frequency",
    "replan_strategy",
    "prompt",
    "acceptance",
    "domain",
    "gate_rung",
];

const REQUIRED_TASK_FIELDS: &[&str] = &["id", "title", "status", "role", "tier"];

const KNOWN_VERIFY_FIELDS: &[&str] = &["phase", "command", "fail_msg", "timeout_ms"];
const REQUIRED_VERIFY_FIELDS: &[&str] = &["phase", "command"];

/// Common LLM field typos and their corrections.
const FIELD_TYPO_CORRECTIONS: &[(&str, &str)] = &[
    ("pha", "phase"),
    ("phas", "phase"),
    ("stat", "status"),
    ("statu", "status"),
    ("descr", "description"),
    ("descrption", "description"),
    ("titl", "title"),
    ("comand", "command"),
    ("commnd", "command"),
    ("name", "plan"),
];

/// Extract a fenced code block with the given tag from text.
fn extract_fenced_block<'a>(text: &'a str, tag: &str) -> Option<&'a str> {
    let fence_plain = format!("```{tag}");
    let fence_angle = format!("```<{tag}>");

    let start_marker = if let Some(pos) = text.find(&fence_plain) {
        pos + fence_plain.len()
    } else if let Some(pos) = text.find(&fence_angle) {
        pos + fence_angle.len()
    } else {
        return None;
    };

    // Skip to end of the opening line.
    let content_start = text[start_marker..].find('\n')? + start_marker + 1;

    // Find closing ``` on its own line.
    let remaining = &text[content_start..];
    let mut close_pos = None;
    for (i, line) in remaining.lines().enumerate() {
        if line.trim_start().starts_with("```") && !line.trim_start().starts_with("````") {
            // Compute byte offset from the line index.
            let offset: usize = remaining
                .lines()
                .take(i)
                .map(|l| l.len() + 1) // +1 for newline
                .sum();
            close_pos = Some(offset);
            break;
        }
    }

    let close_pos = close_pos?;
    let content = &remaining[..close_pos];
    if content.trim().is_empty() {
        return None;
    }
    Some(content.trim_end())
}

/// Fallback: try to extract TOML starting from `[meta]`.
fn extract_toml_content_fallback(output: &str) -> Option<&str> {
    let meta_start = output.find("[meta]")?;
    let line_start = output[..meta_start].rfind('\n').map_or(meta_start, |p| p + 1);
    let task_marker = output[line_start..].find("[[task]]")?;
    let last_task_end = output[line_start..].rfind("[[task]]")?;

    // Find the end of the last task block: look for a blank line after the
    // last [[task]] or end of string.
    let search_from = line_start + last_task_end;
    let end = output[search_from..]
        .find("\n\n")
        .map_or(output.len(), |p| search_from + p);

    let candidate = &output[line_start..end];
    // Require at least both [meta] and [[task]] in the candidate.
    if candidate.contains("[meta]") && candidate.contains("[[task]]") {
        Some(candidate)
    } else {
        let _ = task_marker;
        None
    }
}

/// Suggest a correction for a possibly-misspelled field name.
fn suggest_field_correction(field: &str, known: &[&str]) -> Option<String> {
    // Check explicit typo table first.
    for &(typo, correction) in FIELD_TYPO_CORRECTIONS {
        if field == typo {
            return Some(correction.to_string());
        }
    }
    // Prefix match: if the field is a prefix of exactly one known field,
    // suggest that.
    let prefix_matches: Vec<&&str> = known.iter().filter(|k| k.starts_with(field)).collect();
    if prefix_matches.len() == 1 {
        return Some(prefix_matches[0].to_string());
    }
    None
}

/// Return the next-tier model for escalation on validation failures.
fn next_tier_model(
    current: Option<&str>,
    tier_models: &HashMap<String, String>,
    configured_models: &HashSet<String>,
) -> Option<String> {
    let chain: Vec<&str> = if tier_models.is_empty() {
        DEFAULT_ESCALATION_CHAIN.to_vec()
    } else {
        ["haiku", "sonnet", "opus"]
            .iter()
            .filter_map(|k| tier_models.get(*k).map(String::as_str))
            .collect()
    };

    let current_slug = current.unwrap_or("");
    let pos = chain.iter().position(|m| *m == current_slug);

    let candidates: &[&str] = match pos {
        Some(i) if i + 1 < chain.len() => &chain[i + 1..],
        None if !chain.is_empty() => &chain,
        _ => return None,
    };

    if configured_models.is_empty() {
        candidates.first().map(|m| (*m).to_string())
    } else {
        candidates
            .iter()
            .find(|m| configured_models.contains(**m))
            .map(|m| (*m).to_string())
    }
}

/// Validate and fix generated plan TOML.
///
/// On fixable issues the TOML is patched and repairs are recorded.
/// On unfixable issues an error is returned.
fn validate_and_fix_plan_toml(
    toml_str: &str,
    slug: &str,
    _models: &IndexMap<String, ModelProfile>,
    _default_model: Option<&str>,
    repairs: &mut Vec<String>,
) -> Result<String> {
    // 1. Parse syntax.
    let mut root: toml::Value =
        toml::from_str(toml_str).map_err(|e| anyhow!("generated plan has invalid TOML: {e}"))?;

    let root_table = root
        .as_table_mut()
        .ok_or_else(|| anyhow!("generated plan TOML root is not a table"))?;

    let mut errors: Vec<String> = Vec::new();

    // -- [meta] validation ---------------------------------------------------
    if let Some(meta_val) = root_table.get_mut("meta") {
        if let Some(meta) = meta_val.as_table_mut() {
            let meta_keys: Vec<String> = meta.keys().cloned().collect();
            for key in &meta_keys {
                if !KNOWN_META_FIELDS.contains(&key.as_str()) {
                    if let Some(correction) = suggest_field_correction(key, KNOWN_META_FIELDS) {
                        if let Some(value) = meta.remove(key.as_str()) {
                            repairs.push(format!("[meta] field '{key}' corrected to '{correction}'"));
                            meta.insert(correction, value);
                        }
                    }
                }
            }
            for &required in REQUIRED_META_FIELDS {
                match meta.get(required) {
                    None => errors.push(format!("[meta] is missing required field '{required}'")),
                    Some(v) if v.as_str().is_some_and(|s| s.trim().is_empty()) => {
                        errors.push(format!("[meta].{required} is empty"));
                    }
                    _ => {}
                }
            }
            // Fix meta.plan if truncated or wrong.
            if let Some(plan_val) = meta.get("plan") {
                if let Some(plan_str) = plan_val.as_str() {
                    if plan_str != slug {
                        repairs.push(format!(
                            "meta.plan '{plan_str}' corrected to '{slug}'"
                        ));
                        meta.insert("plan".to_string(), toml::Value::String(slug.to_string()));
                    }
                }
            }
        }
    } else {
        errors.push("[meta] section is missing".to_string());
    }

    // -- [[task]] validation --------------------------------------------------
    if let Some(tasks_val) = root_table.get_mut("task") {
        if let Some(tasks) = tasks_val.as_array_mut() {
            if tasks.is_empty() {
                errors.push("[[task]] array is present but empty".to_string());
            }
            for (i, task_val) in tasks.iter_mut().enumerate() {
                if let Some(task) = task_val.as_table_mut() {
                    let task_id_label: String = task
                        .get("id")
                        .and_then(toml::Value::as_str)
                        .map(String::from)
                        .unwrap_or_else(|| format!("task #{}", i + 1));

                    // Flag unknown task fields.
                    let task_keys: Vec<String> = task.keys().cloned().collect();
                    for key in &task_keys {
                        if !KNOWN_TASK_FIELDS.contains(&key.as_str()) {
                            if let Some(correction) =
                                suggest_field_correction(key, KNOWN_TASK_FIELDS)
                            {
                                if let Some(value) = task.remove(key.as_str()) {
                                    repairs.push(format!(
                                        "{task_id_label}: field '{key}' corrected to '{correction}'"
                                    ));
                                    task.insert(correction, value);
                                }
                            }
                        }
                    }

                    // Check required task fields.
                    for &required in REQUIRED_TASK_FIELDS {
                        match task.get(required) {
                            None => errors.push(format!(
                                "{task_id_label} is missing required field '{required}'"
                            )),
                            Some(v) if v.as_str().is_some_and(|s| s.trim().is_empty()) => {
                                errors.push(format!(
                                    "{task_id_label}: field '{required}' is empty"
                                ));
                            }
                            _ => {}
                        }
                    }

                    // Validate status.
                    if let Some(status_val) = task.get("status").cloned() {
                        if let Some(s) = status_val.as_str() {
                            const VALID_STATUSES: &[&str] = &[
                                "ready", "pending", "blocked", "in_progress", "done", "skipped",
                            ];
                            if !VALID_STATUSES.contains(&s) {
                                repairs.push(format!(
                                    "{task_id_label}: status '{s}' corrected to 'ready'"
                                ));
                                task.insert(
                                    "status".to_string(),
                                    toml::Value::String("ready".to_string()),
                                );
                            }
                        }
                    }

                    // Validate role.
                    if let Some(role_val) = task.get("role").cloned() {
                        if let Some(r) = role_val.as_str() {
                            const VALID_ROLES: &[&str] = &[
                                "implementer", "architect", "researcher", "strategist",
                                "scribe", "quick-reviewer",
                            ];
                            if !VALID_ROLES.contains(&r) {
                                repairs.push(format!(
                                    "{task_id_label}: role '{r}' corrected to 'implementer'"
                                ));
                                task.insert(
                                    "role".to_string(),
                                    toml::Value::String("implementer".to_string()),
                                );
                            }
                        }
                    }

                    // Strip model_hint.
                    if let Some(hint_val) = task.remove("model_hint") {
                        let hint = hint_val.as_str().unwrap_or("<unknown>");
                        repairs.push(format!(
                            "{task_id_label}: removed model_hint '{hint}'"
                        ));
                    }

                    // Validate [[task.verify]] sub-entries.
                    if let Some(verify_val) = task.get_mut("verify") {
                        if let Some(steps) = verify_val.as_array_mut() {
                            for (si, step_val) in steps.iter_mut().enumerate() {
                                if let Some(step) = step_val.as_table_mut() {
                                    let step_keys: Vec<String> = step.keys().cloned().collect();
                                    for key in &step_keys {
                                        if !KNOWN_VERIFY_FIELDS.contains(&key.as_str()) {
                                            if let Some(correction) =
                                                suggest_field_correction(key, KNOWN_VERIFY_FIELDS)
                                            {
                                                if let Some(value) = step.remove(key.as_str()) {
                                                    repairs.push(format!(
                                                        "{task_id_label} verify[{si}]: \
                                                         field '{key}' corrected to '{correction}'"
                                                    ));
                                                    step.insert(correction, value);
                                                }
                                            }
                                        }
                                    }
                                    for &required in REQUIRED_VERIFY_FIELDS {
                                        if step
                                            .get(required)
                                            .and_then(toml::Value::as_str)
                                            .is_none_or(|s| s.trim().is_empty())
                                        {
                                            errors.push(format!(
                                                "{task_id_label} verify[{si}]: \
                                                 missing required field '{required}'"
                                            ));
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // Auto-add verify for implementer tasks.
                    if task.get("verify").is_none() {
                        let role = task
                            .get("role")
                            .and_then(toml::Value::as_str)
                            .unwrap_or("implementer");
                        let files: Vec<String> = task
                            .get("files")
                            .and_then(toml::Value::as_array)
                            .map(|arr| {
                                arr.iter()
                                    .filter_map(toml::Value::as_str)
                                    .map(String::from)
                                    .collect()
                            })
                            .unwrap_or_default();

                        if role == "implementer" && !files.is_empty() {
                            let crate_name = infer_crate_from_paths(&files);
                            let compile_cmd = match &crate_name {
                                Some(c) => format!("cargo check -p {c}"),
                                None => "cargo check --workspace".to_string(),
                            };
                            let auto_verify = vec![make_verify_entry(
                                "compile",
                                &compile_cmd,
                                &format!(
                                    "{} must compile",
                                    crate_name.as_deref().unwrap_or("workspace"),
                                ),
                            )];
                            task.insert(
                                "verify".to_string(),
                                toml::Value::Array(auto_verify),
                            );
                            repairs.push(format!(
                                "{task_id_label}: auto-added compile verify"
                            ));
                        }
                    }
                }
            }
        }
    } else {
        errors.push("[[task]] array is missing".to_string());
    }

    if !errors.is_empty() {
        let joined = errors.join("\n  - ");
        return Err(anyhow!(
            "generated plan TOML has {n} validation error(s):\n  - {joined}",
            n = errors.len()
        ));
    }

    // Serialize back.
    let mut serialized = toml::to_string_pretty(&root)
        .map_err(|e| anyhow!("failed to re-serialize fixed plan TOML: {e}"))?;

    // Angle-bracket placeholder replacement.
    let path_default = format!("crates/{slug}/src/lib.rs");
    let replacements: &[(&str, &str)] = &[
        ("<relevant-lib>", slug),
        ("<binary-crate>", slug),
        ("<crate>", slug),
        ("<module>", "lib"),
        ("<path>", &path_default),
        ("<file>", &path_default),
        ("<test_name>", "test_placeholder"),
    ];
    for &(placeholder, replacement) in replacements {
        if serialized.contains(placeholder) {
            repairs.push(format!(
                "replaced placeholder '{placeholder}' with '{replacement}'"
            ));
            serialized = serialized.replace(placeholder, replacement);
        }
    }

    // Re-verify TOML validity after replacements.
    if replacements.iter().any(|(ph, _)| toml_str.contains(*ph)) {
        let _: toml::Value = toml::from_str(&serialized)
            .map_err(|e| anyhow!("TOML became invalid after placeholder replacement: {e}"))?;
    }

    Ok(serialized)
}

/// Infer the crate name from a list of file paths.
fn infer_crate_from_paths(files: &[String]) -> Option<String> {
    for f in files {
        if let Some(rest) = f.strip_prefix("crates/") {
            if let Some(crate_name) = rest.split('/').next() {
                return Some(crate_name.to_string());
            }
        }
    }
    None
}

/// Build a `[[task.verify]]` TOML table value.
fn make_verify_entry(phase: &str, command: &str, fail_msg: &str) -> toml::Value {
    let mut table = toml::value::Table::new();
    table.insert("phase".to_string(), toml::Value::String(phase.to_string()));
    table.insert(
        "command".to_string(),
        toml::Value::String(command.to_string()),
    );
    table.insert(
        "fail_msg".to_string(),
        toml::Value::String(fail_msg.to_string()),
    );
    toml::Value::Table(table)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // ---- PlanSource tests ----

    #[test]
    fn plan_source_prd_slug() {
        let source = PlanSource::Prd {
            slug: "my-feature".to_string(),
            prd_path: PathBuf::from("/tmp/prd.md"),
        };
        assert_eq!(source.slug(), "my-feature");
        assert!(!source.has_failure_context());
        assert!(source.failure_context().is_none());
    }

    #[test]
    fn plan_source_prompt_slug() {
        let source = PlanSource::Prompt {
            prompt: "build a widget".to_string(),
        };
        assert_eq!(source.slug(), "prompt");
    }

    #[test]
    fn plan_source_file_slug() {
        let source = PlanSource::File {
            path: PathBuf::from("/tmp/my-notes.md"),
        };
        assert_eq!(source.slug(), "my-notes");
    }

    #[test]
    fn plan_source_replan_has_failure_context() {
        let source = PlanSource::Replan {
            slug: "my-feature".to_string(),
            prd_path: PathBuf::from("/tmp/prd.md"),
            failure_context: "gate failure on T2".to_string(),
        };
        assert_eq!(source.slug(), "my-feature");
        assert!(source.has_failure_context());
        assert_eq!(source.failure_context(), Some("gate failure on T2"));
    }

    // ---- PlanGeneratorOutcome tests ----

    #[test]
    fn outcome_success_requires_toml_and_valid_outcome() {
        let outcome = PlanGeneratorOutcome {
            tasks_toml: Some("[meta]\nplan = \"test\"\n".to_string()),
            plan_md: None,
            slug: "test".to_string(),
            outcome: GenerationOutcome {
                process_success: true,
                artifact_valid: true,
                validation_report: None,
            },
            evidence: ValidationEvidence {
                extraction_attempts: 1,
                model_escalated: false,
                final_model: None,
                repairs_applied: vec![],
                policy_violations: vec![],
            },
            task_count: 2,
            estimated_complexity: None,
            adapter_key: "test".to_string(),
        };
        assert!(outcome.is_success());
        assert!(!outcome.is_partial());
    }

    #[test]
    fn outcome_partial_when_process_succeeded_but_artifact_invalid() {
        let outcome = PlanGeneratorOutcome {
            tasks_toml: None,
            plan_md: None,
            slug: "test".to_string(),
            outcome: GenerationOutcome {
                process_success: true,
                artifact_valid: false,
                validation_report: None,
            },
            evidence: ValidationEvidence {
                extraction_attempts: 3,
                model_escalated: true,
                final_model: Some("claude-opus-4-6".to_string()),
                repairs_applied: vec![],
                policy_violations: vec!["too many tasks".to_string()],
            },
            task_count: 0,
            estimated_complexity: None,
            adapter_key: "test".to_string(),
        };
        assert!(!outcome.is_success());
        assert!(outcome.is_partial());
    }

    #[test]
    fn outcome_failure_when_process_failed() {
        let outcome = PlanGeneratorOutcome {
            tasks_toml: None,
            plan_md: None,
            slug: "test".to_string(),
            outcome: GenerationOutcome {
                process_success: false,
                artifact_valid: false,
                validation_report: None,
            },
            evidence: ValidationEvidence {
                extraction_attempts: 1,
                model_escalated: false,
                final_model: None,
                repairs_applied: vec![],
                policy_violations: vec![],
            },
            task_count: 0,
            estimated_complexity: None,
            adapter_key: "test".to_string(),
        };
        assert!(!outcome.is_success());
        assert!(!outcome.is_partial());
    }

    // ---- extract_fenced_block tests ----

    #[test]
    fn extract_fenced_block_finds_toml() {
        let text = "Some text\n```toml\n[meta]\nplan = \"test\"\n```\nMore text";
        let block = extract_fenced_block(text, "toml").unwrap();
        assert!(block.contains("[meta]"));
        assert!(block.contains("plan = \"test\""));
    }

    #[test]
    fn extract_fenced_block_returns_none_for_missing() {
        assert!(extract_fenced_block("no blocks here", "toml").is_none());
    }

    #[test]
    fn extract_fenced_block_returns_none_for_empty() {
        let text = "```toml\n\n```\n";
        assert!(extract_fenced_block(text, "toml").is_none());
    }

    #[test]
    fn extract_fenced_block_handles_angle_bracket_tag() {
        let text = "Output:\n```<plan.md>\n# My Plan\n\nSteps here.\n```\n";
        let block = extract_fenced_block(text, "plan.md").unwrap();
        assert!(block.contains("# My Plan"));
    }

    // ---- suggest_field_correction tests ----

    #[test]
    fn suggest_correction_finds_typos() {
        assert_eq!(
            suggest_field_correction("pha", KNOWN_VERIFY_FIELDS),
            Some("phase".to_string())
        );
        assert_eq!(
            suggest_field_correction("stat", KNOWN_TASK_FIELDS),
            Some("status".to_string())
        );
        assert_eq!(
            suggest_field_correction("zzzzunknown", KNOWN_TASK_FIELDS),
            None
        );
    }

    // ---- next_tier_model tests ----

    #[test]
    fn next_tier_escalates_from_haiku() {
        let tier_models = HashMap::new();
        let configured = HashSet::new();
        let next = next_tier_model(
            Some("claude-haiku-4-5"),
            &tier_models,
            &configured,
        );
        assert_eq!(next, Some("claude-sonnet-4-6".to_string()));
    }

    #[test]
    fn next_tier_escalates_from_sonnet() {
        let tier_models = HashMap::new();
        let configured = HashSet::new();
        let next = next_tier_model(
            Some("claude-sonnet-4-6"),
            &tier_models,
            &configured,
        );
        assert_eq!(next, Some("claude-opus-4-6".to_string()));
    }

    #[test]
    fn next_tier_returns_none_at_top() {
        let tier_models = HashMap::new();
        let configured = HashSet::new();
        let next = next_tier_model(
            Some("claude-opus-4-6"),
            &tier_models,
            &configured,
        );
        assert!(next.is_none());
    }

    #[test]
    fn next_tier_respects_configured_models() {
        let tier_models = HashMap::new();
        let mut configured = HashSet::new();
        configured.insert("claude-opus-4-6".to_string());
        // Haiku should skip sonnet (not configured) and go to opus.
        let next = next_tier_model(
            Some("claude-haiku-4-5"),
            &tier_models,
            &configured,
        );
        assert_eq!(next, Some("claude-opus-4-6".to_string()));
    }

    // ---- validate_raw_output golden fixtures ----

    fn test_generator() -> PlanGenerator {
        PlanGenerator::new(
            IndexMap::new(),
            HashMap::new(),
            true,
            None,
        )
    }

    const GOLDEN_VALID_PLAN: &str = r#"```toml
[meta]
plan = "test-feature"
total = 2
done = 0
status = "ready"
max_parallel = 1

[[task]]
id = "T1"
title = "Add struct definition"
description = "Define the new struct."
status = "ready"
tier = "mechanical"
max_loc = 20
files = ["crates/roko-core/src/lib.rs"]
allowed_tools = ["read_file", "grep"]
denied_tools = []
depends_on = []
role = "implementer"

[[task.verify]]
phase = "compile"
command = "cargo check -p roko-core"

[[task]]
id = "T2"
title = "Wire struct into CLI"
description = "Import and use the new struct."
status = "ready"
tier = "focused"
max_loc = 50
files = ["crates/roko-cli/src/lib.rs"]
allowed_tools = ["read_file", "write_file"]
denied_tools = []
depends_on = ["T1"]
role = "implementer"

[[task.verify]]
phase = "compile"
command = "cargo check -p roko-cli"
```
"#;

    #[test]
    fn validate_golden_valid_plan() {
        let gen = test_generator();
        let result = gen.validate_raw_output(GOLDEN_VALID_PLAN, "test-feature", PlanTemplateKind::Default);
        let validated = result.expect("golden plan should validate");
        assert_eq!(validated.task_count, 2);
        assert!(validated.tasks_toml.contains("test-feature"));
        assert!(validated.plan_md.is_none());
    }

    #[test]
    fn validate_plan_with_model_hint_strips_it() {
        let plan_with_hint = r#"```toml
[meta]
plan = "strip-hint"
total = 1
done = 0
status = "ready"
max_parallel = 1

[[task]]
id = "T1"
title = "Task with hint"
description = "Should have model_hint stripped."
status = "ready"
tier = "focused"
max_loc = 50
files = ["crates/roko-core/src/lib.rs"]
allowed_tools = ["read_file"]
denied_tools = []
depends_on = []
role = "implementer"
model_hint = "claude-3-opus"

[[task.verify]]
phase = "compile"
command = "cargo check -p roko-core"
```
"#;
        let gen = test_generator();
        let result = gen.validate_raw_output(plan_with_hint, "strip-hint", PlanTemplateKind::Default);
        let validated = result.expect("plan with model_hint should validate after stripping");
        assert!(!validated.tasks_toml.contains("model_hint"));
        assert!(validated.repairs.iter().any(|r| r.contains("model_hint")));
    }

    #[test]
    fn validate_plan_with_wrong_slug_corrects_it() {
        let plan_wrong_slug = r#"```toml
[meta]
plan = "wrong-slug"
total = 1
done = 0
status = "ready"
max_parallel = 1

[[task]]
id = "T1"
title = "Task"
description = "Task description."
status = "ready"
tier = "focused"
max_loc = 50
files = ["crates/roko-core/src/lib.rs"]
allowed_tools = ["read_file"]
denied_tools = []
depends_on = []
role = "implementer"

[[task.verify]]
phase = "compile"
command = "cargo check -p roko-core"
```
"#;
        let gen = test_generator();
        let result = gen.validate_raw_output(plan_wrong_slug, "correct-slug", PlanTemplateKind::Default);
        let validated = result.expect("plan with wrong slug should be corrected");
        assert!(validated.tasks_toml.contains("correct-slug"));
        assert!(validated.repairs.iter().any(|r| r.contains("corrected")));
    }

    #[test]
    fn validate_plan_missing_meta_errors() {
        let plan_no_meta = r#"```toml
[[task]]
id = "T1"
title = "Orphan task"
description = "No meta section."
status = "ready"
tier = "focused"
role = "implementer"

[[task.verify]]
phase = "compile"
command = "cargo check"
```
"#;
        let gen = test_generator();
        let result = gen.validate_raw_output(plan_no_meta, "no-meta", PlanTemplateKind::Default);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("[meta]"));
    }

    #[test]
    fn validate_plan_missing_tasks_errors() {
        let plan_no_tasks = r#"```toml
[meta]
plan = "no-tasks"
total = 0
done = 0
status = "ready"
```
"#;
        let gen = test_generator();
        let result = gen.validate_raw_output(plan_no_tasks, "no-tasks", PlanTemplateKind::Default);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("[[task]]"));
    }

    #[test]
    fn validate_plan_invalid_status_corrected() {
        let plan_bad_status = r#"```toml
[meta]
plan = "bad-status"
total = 1
done = 0
status = "ready"
max_parallel = 1

[[task]]
id = "T1"
title = "Task"
description = "Bad status."
status = "running"
tier = "focused"
max_loc = 50
files = ["crates/roko-core/src/lib.rs"]
depends_on = []
role = "implementer"

[[task.verify]]
phase = "compile"
command = "cargo check -p roko-core"
```
"#;
        let gen = test_generator();
        let result = gen.validate_raw_output(plan_bad_status, "bad-status", PlanTemplateKind::Default);
        let validated = result.expect("invalid status should be corrected to 'ready'");
        assert!(validated.tasks_toml.contains("ready"));
        assert!(validated.repairs.iter().any(|r| r.contains("status")));
    }

    #[test]
    fn validate_plan_invalid_role_corrected() {
        let plan_bad_role = r#"```toml
[meta]
plan = "bad-role"
total = 1
done = 0
status = "ready"
max_parallel = 1

[[task]]
id = "T1"
title = "Task"
description = "Bad role."
status = "ready"
tier = "focused"
max_loc = 50
files = ["crates/roko-core/src/lib.rs"]
depends_on = []
role = "developer"

[[task.verify]]
phase = "compile"
command = "cargo check -p roko-core"
```
"#;
        let gen = test_generator();
        let result = gen.validate_raw_output(plan_bad_role, "bad-role", PlanTemplateKind::Default);
        let validated = result.expect("invalid role should be corrected to 'implementer'");
        assert!(validated.tasks_toml.contains("implementer"));
        assert!(validated.repairs.iter().any(|r| r.contains("role")));
    }

    #[test]
    fn validate_plan_auto_adds_verify_for_implementer() {
        let plan_no_verify = r#"```toml
[meta]
plan = "auto-verify"
total = 1
done = 0
status = "ready"
max_parallel = 1

[[task]]
id = "T1"
title = "Task"
description = "Missing verify."
status = "ready"
tier = "focused"
max_loc = 50
files = ["crates/roko-core/src/lib.rs"]
depends_on = []
role = "implementer"
```
"#;
        let gen = test_generator();
        let result = gen.validate_raw_output(plan_no_verify, "auto-verify", PlanTemplateKind::Default);
        let validated = result.expect("should auto-add verify for implementer");
        assert!(validated.tasks_toml.contains("cargo check -p roko-core"));
        assert!(validated.repairs.iter().any(|r| r.contains("auto-added")));
    }

    #[test]
    fn validate_plan_placeholder_replacement() {
        let plan_with_placeholders = r#"```toml
[meta]
plan = "placeholder-test"
total = 1
done = 0
status = "ready"
max_parallel = 1

[[task]]
id = "T1"
title = "Task"
description = "Has placeholders."
status = "ready"
tier = "focused"
max_loc = 50
files = ["crates/<crate>/src/lib.rs"]
depends_on = []
role = "implementer"

[[task.verify]]
phase = "compile"
command = "cargo check -p <relevant-lib>"
```
"#;
        let gen = test_generator();
        let result = gen.validate_raw_output(
            plan_with_placeholders,
            "placeholder-test",
            PlanTemplateKind::Default,
        );
        let validated = result.expect("placeholders should be replaced");
        assert!(!validated.tasks_toml.contains("<crate>"));
        assert!(!validated.tasks_toml.contains("<relevant-lib>"));
        assert!(validated.tasks_toml.contains("placeholder-test"));
    }

    #[test]
    fn validate_no_fenced_block_errors() {
        let gen = test_generator();
        let result = gen.validate_raw_output(
            "I will now implement the feature. Here is my plan.",
            "no-block",
            PlanTemplateKind::Default,
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("no TOML block"));
    }

    #[test]
    fn validate_plan_with_plan_md() {
        let plan_with_md = r#"```toml
[meta]
plan = "with-md"
total = 1
done = 0
status = "ready"
max_parallel = 1

[[task]]
id = "T1"
title = "Task"
description = "Has plan.md."
status = "ready"
tier = "focused"
max_loc = 50
files = ["crates/roko-core/src/lib.rs"]
depends_on = []
role = "implementer"

[[task.verify]]
phase = "compile"
command = "cargo check -p roko-core"
```

```plan.md
# Plan: with-md

This plan adds a widget.
```
"#;
        let gen = test_generator();
        let result = gen.validate_raw_output(plan_with_md, "with-md", PlanTemplateKind::Default);
        let validated = result.expect("plan with plan.md should validate");
        assert!(validated.plan_md.is_some());
        assert!(validated.plan_md.as_ref().unwrap().contains("widget"));
    }

    // ---- PlanGenerator service tests ----

    #[test]
    fn generator_new_collects_configured_models() {
        let mut profiles = IndexMap::new();
        profiles.insert(
            "sonnet".to_string(),
            ModelProfile {
                slug: "claude-sonnet-4-6".to_string(),
                ..Default::default()
            },
        );
        let gen = PlanGenerator::new(profiles, HashMap::new(), true, None);
        assert!(gen.configured_models.contains("sonnet"));
        assert!(gen.configured_models.contains("claude-sonnet-4-6"));
    }

    #[test]
    fn generator_escalation_disabled() {
        let gen = PlanGenerator::new(IndexMap::new(), HashMap::new(), false, None);
        assert!(gen.next_escalation_model(Some("claude-haiku-4-5")).is_none());
    }

    #[test]
    fn generator_escalation_enabled() {
        let gen = PlanGenerator::new(IndexMap::new(), HashMap::new(), true, None);
        let next = gen.next_escalation_model(Some("claude-haiku-4-5"));
        assert_eq!(next, Some("claude-sonnet-4-6".to_string()));
    }

    #[test]
    fn effective_repair_cap_uses_override() {
        let overrides = PlanGeneratorOverrides {
            repair_cap: Some(5),
            ..Default::default()
        };
        assert_eq!(PlanGenerator::effective_repair_cap(&overrides), 5);
    }

    #[test]
    fn effective_repair_cap_uses_default() {
        let overrides = PlanGeneratorOverrides::default();
        assert_eq!(
            PlanGenerator::effective_repair_cap(&overrides),
            DEFAULT_REPAIR_CAP
        );
    }

    #[test]
    fn resolve_template_uses_override() {
        let overrides = PlanGeneratorOverrides {
            template: Some("compact".to_string()),
            ..Default::default()
        };
        let template = PlanGenerator::resolve_template(&overrides, Some("strict"));
        assert_eq!(template, PlanTemplateKind::Compact);
    }

    #[test]
    fn resolve_template_falls_back_to_prd() {
        let overrides = PlanGeneratorOverrides::default();
        let template = PlanGenerator::resolve_template(&overrides, Some("strict"));
        assert_eq!(template, PlanTemplateKind::Strict);
    }

    #[test]
    fn resolve_template_defaults_without_any_hint() {
        let overrides = PlanGeneratorOverrides::default();
        let template = PlanGenerator::resolve_template(&overrides, None);
        assert_eq!(template, PlanTemplateKind::Default);
    }

    // ---- Outcome builder tests ----

    #[test]
    fn success_outcome_builder() {
        let request = PlanGeneratorRequest {
            source: PlanSource::Prd {
                slug: "demo".to_string(),
                prd_path: PathBuf::from("/tmp/prd.md"),
            },
            workdir: PathBuf::from("/tmp"),
            adapter_key: adapter_keys::PRD_DEFAULT.to_string(),
            overrides: PlanGeneratorOverrides::default(),
            dry_run: false,
        };
        let validated = ValidatedPlan {
            tasks_toml: "[meta]\nplan = \"demo\"\n".to_string(),
            plan_md: None,
            task_count: 3,
            repairs: vec![],
            policy_violations: vec![],
        };
        let evidence = ValidationEvidence {
            extraction_attempts: 1,
            model_escalated: false,
            final_model: None,
            repairs_applied: vec![],
            policy_violations: vec![],
        };
        let outcome = PlanGenerator::success_outcome(&request, validated, evidence, None);
        assert!(outcome.is_success());
        assert_eq!(outcome.task_count, 3);
        assert_eq!(outcome.slug, "demo");
        assert_eq!(outcome.adapter_key, adapter_keys::PRD_DEFAULT);
    }

    #[test]
    fn failure_outcome_builder() {
        let request = PlanGeneratorRequest {
            source: PlanSource::Prompt {
                prompt: "build a widget".to_string(),
            },
            workdir: PathBuf::from("/tmp"),
            adapter_key: adapter_keys::DO_STANDARD.to_string(),
            overrides: PlanGeneratorOverrides::default(),
            dry_run: false,
        };
        let evidence = ValidationEvidence {
            extraction_attempts: 3,
            model_escalated: true,
            final_model: Some("claude-opus-4-6".to_string()),
            repairs_applied: vec![],
            policy_violations: vec!["too many tasks".to_string()],
        };
        let outcome = PlanGenerator::failure_outcome(&request, evidence);
        assert!(!outcome.is_success());
        assert!(!outcome.is_partial());
        assert_eq!(outcome.task_count, 0);
    }

    #[test]
    fn partial_outcome_builder() {
        let request = PlanGeneratorRequest {
            source: PlanSource::Prd {
                slug: "demo".to_string(),
                prd_path: PathBuf::from("/tmp/prd.md"),
            },
            workdir: PathBuf::from("/tmp"),
            adapter_key: adapter_keys::PRD_MODEL.to_string(),
            overrides: PlanGeneratorOverrides::default(),
            dry_run: false,
        };
        let evidence = ValidationEvidence {
            extraction_attempts: 2,
            model_escalated: false,
            final_model: None,
            repairs_applied: vec![],
            policy_violations: vec![],
        };
        let outcome = PlanGenerator::partial_outcome(&request, evidence);
        assert!(!outcome.is_success());
        assert!(outcome.is_partial());
    }

    // ---- Budget enforcement ----

    #[test]
    fn validate_plan_exceeding_compact_budget_errors() {
        // Compact template allows max 4 tasks.
        let mut tasks = String::from(
            "[meta]\nplan = \"big-plan\"\ntotal = 5\ndone = 0\nstatus = \"ready\"\nmax_parallel = 1\n\n",
        );
        for i in 1..=5 {
            tasks.push_str(&format!(
                "[[task]]\nid = \"T{i}\"\ntitle = \"Task {i}\"\n\
                 description = \"Task.\"\nstatus = \"ready\"\ntier = \"focused\"\n\
                 max_loc = 50\nfiles = [\"crates/roko-core/src/lib.rs\"]\n\
                 depends_on = []\nrole = \"implementer\"\n\n\
                 [[task.verify]]\nphase = \"compile\"\ncommand = \"cargo check\"\n\n"
            ));
        }
        let fenced = format!("```toml\n{tasks}```\n");
        let gen = test_generator();
        let result = gen.validate_raw_output(&fenced, "big-plan", PlanTemplateKind::Compact);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("budget"));
    }

    // ---- Adapter key manifest completeness ----

    #[test]
    fn adapter_keys_manifest_complete() {
        // Verify all 10 adapter keys from the #280 manifest exist.
        let keys = [
            adapter_keys::PRD_DEFAULT,
            adapter_keys::PRD_MODEL,
            adapter_keys::PRD_REPLAN,
            adapter_keys::PLAN_GENERATE,
            adapter_keys::DO_STANDARD,
            adapter_keys::DO_COMPLEX,
            adapter_keys::CLI_SERVE_RUNTIME,
            adapter_keys::SERVE_RUNTIME,
            adapter_keys::SERVE_HTTP,
            adapter_keys::GATE_REPLAN,
        ];
        assert_eq!(keys.len(), 10, "manifest must have exactly 10 adapter keys");
        // No duplicates.
        let mut seen = HashSet::new();
        for key in &keys {
            assert!(seen.insert(*key), "duplicate adapter key: {key}");
        }
    }

    // ---- infer_crate_from_paths ----

    #[test]
    fn infer_crate_from_crates_path() {
        let files = vec!["crates/roko-core/src/lib.rs".to_string()];
        assert_eq!(infer_crate_from_paths(&files), Some("roko-core".to_string()));
    }

    #[test]
    fn infer_crate_returns_none_for_non_crate_paths() {
        let files = vec!["src/main.rs".to_string()];
        assert_eq!(infer_crate_from_paths(&files), None);
    }

    // ---- Typo correction in verify fields ----

    #[test]
    fn validate_plan_with_verify_typo_corrects_it() {
        let plan_verify_typo = r#"```toml
[meta]
plan = "verify-typo"
total = 1
done = 0
status = "ready"
max_parallel = 1

[[task]]
id = "T1"
title = "Task"
description = "Has verify typo."
status = "ready"
tier = "focused"
max_loc = 50
files = ["crates/roko-core/src/lib.rs"]
depends_on = []
role = "implementer"

[[task.verify]]
pha = "compile"
command = "cargo check -p roko-core"
```
"#;
        let gen = test_generator();
        let result = gen.validate_raw_output(plan_verify_typo, "verify-typo", PlanTemplateKind::Default);
        let validated = result.expect("verify typo should be corrected");
        assert!(validated.tasks_toml.contains("phase"));
        assert!(validated.repairs.iter().any(|r| r.contains("corrected")));
    }
}
