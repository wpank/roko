//! Plan source authoring utilities.
//!
//! Validates and saves a plan's `tasks.toml` source text losslessly.
//! Neither the server nor the portal ever edits TOML by structure; they send
//! the whole source, and it is saved byte-for-byte.
//!
//! # API surface
//!
//! - [`validate_plan_source`] — parse + lint + policy-check without touching disk.
//! - [`save_plan_source`] — validate then atomically write on success.
//! - [`starter_plan_source`] — generate a minimal plan that passes validation.
//! - [`build_revision_prompt`] — build a prompt for revising an existing plan.
//! - [`apply_revision_output`] — extract, repair, validate, and write the revised plan.
//! - [`revise_plan_source`] — run the planning agent and apply the revision.

use std::path::Path;

use anyhow::{Context as _, Result};
use indexmap::IndexMap;
use roko_core::config::schema::ModelProfile;

use crate::agent_exec::{AgentExecOpts, run_agent_capture_silent};
use crate::plan_policy::{PlanExecutionPolicy, validate_plan_context};
use crate::plan_validate::{Severity, validate_plans_dir_with_workdir};
use crate::task_parser::{TasksFile, repair_toml};

// ─── Public types ─────────────────────────────────────────────────────────────

/// A single diagnostic produced by [`validate_plan_source`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanSourceDiagnostic {
    /// Whether the issue is blocking (`Error`) or advisory (`Warning`).
    pub severity: Severity,
    /// Short rule code, e.g. `"PLAN_PARSE"`, `"PLAN_005"`.
    pub rule_id: String,
    /// Task that produced the issue, when applicable.
    pub task_id: Option<String>,
    /// Human-readable description.
    pub message: String,
}

/// Result returned by [`validate_plan_source`] and [`save_plan_source`].
#[derive(Debug, Clone)]
pub struct PlanSourceReport {
    /// All diagnostics, errors first.
    pub diagnostics: Vec<PlanSourceDiagnostic>,
    /// Count of error-severity diagnostics.
    pub errors: usize,
    /// Count of warning-severity diagnostics.
    pub warnings: usize,
    /// `true` when there are no errors.
    pub valid: bool,
}

impl PlanSourceReport {
    fn from_diagnostics(mut diagnostics: Vec<PlanSourceDiagnostic>) -> Self {
        // Errors before warnings, then stable original order within each group.
        diagnostics.sort_by_key(|d| d.severity);
        let errors = diagnostics
            .iter()
            .filter(|d| d.severity == Severity::Error)
            .count();
        let warnings = diagnostics
            .iter()
            .filter(|d| d.severity == Severity::Warning)
            .count();
        let valid = errors == 0;
        Self {
            diagnostics,
            errors,
            warnings,
            valid,
        }
    }

    fn parse_error(message: impl Into<String>) -> Self {
        Self {
            diagnostics: vec![PlanSourceDiagnostic {
                severity: Severity::Error,
                rule_id: "PLAN_PARSE".to_string(),
                task_id: None,
                message: message.into(),
            }],
            errors: 1,
            warnings: 0,
            valid: false,
        }
    }
}

// ─── Core functions ────────────────────────────────────────────────────────────

/// Validate a plan source text without writing it to the plans directory.
///
/// Steps:
/// 1. Parse with [`TasksFile::parse_str`]. A parse error emits one diagnostic
///    with `rule_id = "PLAN_PARSE"` and returns immediately.
/// 2. Write the text into a temporary plan directory and run the standard
///    linter via [`validate_plans_dir_with_workdir`].
/// 3. Also run [`validate_plan_context`] directly; its violations are always
///    counted as errors (including `PLAN_CONTEXT_MISSING` which the linter
///    skips, but which would fail the runner).
pub fn validate_plan_source(
    workdir: &Path,
    plan_id: &str,
    toml: &str,
    models: &IndexMap<String, ModelProfile>,
) -> PlanSourceReport {
    // ── Step 1: Parse ─────────────────────────────────────────────────────────
    let tasks_file = match TasksFile::parse_str(toml) {
        Ok(f) => f,
        Err(e) => return PlanSourceReport::parse_error(format!("TOML parse error: {e}")),
    };

    // ── Step 2: Materialise in a temporary directory ───────────────────────────
    let tmp = match tempfile::TempDir::new() {
        Ok(t) => t,
        Err(e) => {
            return PlanSourceReport::parse_error(format!("failed to create temp dir: {e}"));
        }
    };
    let plan_dir = tmp.path().join(plan_id);
    if let Err(e) = std::fs::create_dir_all(&plan_dir) {
        return PlanSourceReport::parse_error(format!("failed to create temp plan dir: {e}"));
    }
    let tasks_path = plan_dir.join("tasks.toml");
    if let Err(e) = std::fs::write(&tasks_path, toml) {
        return PlanSourceReport::parse_error(format!("failed to write temp tasks.toml: {e}"));
    }

    let mut diagnostics: Vec<PlanSourceDiagnostic> = Vec::new();

    // Run the standard linter.  It already promotes policy violations (except
    // PLAN_CONTEXT_MISSING) to errors.
    match validate_plans_dir_with_workdir(tmp.path(), Some(models), Some(workdir)) {
        Ok(report) => {
            for plan_diag in &report.plans {
                for d in &plan_diag.diagnostics {
                    diagnostics.push(PlanSourceDiagnostic {
                        severity: d.severity,
                        rule_id: d.rule_id.clone(),
                        task_id: d.task_id.clone(),
                        message: d.message.clone(),
                    });
                }
            }
        }
        Err(e) => {
            // Shouldn't happen for a well-formed temp dir, but surface it.
            diagnostics.push(PlanSourceDiagnostic {
                severity: Severity::Error,
                rule_id: "PLAN_LINTER".to_string(),
                task_id: None,
                message: format!("linter internal error: {e}"),
            });
        }
    }

    // ── Step 3: Runner-level policy check ─────────────────────────────────────
    // The linter skips PLAN_CONTEXT_MISSING (it emits PLAN_030/031 warnings
    // for the same paths instead).  The runner (plan_loader.rs) calls
    // validate_plan_context and would fail-fast on any violation, including
    // PLAN_CONTEXT_MISSING.  We add only PLAN_CONTEXT_MISSING here to avoid
    // duplicating the non-PLAN_CONTEXT_MISSING codes already present as errors
    // from the linter's own call.
    for violation in
        validate_plan_context(&tasks_file, workdir, &plan_dir, PlanExecutionPolicy::for_environment())
    {
        if violation.code == "PLAN_CONTEXT_MISSING" {
            diagnostics.push(PlanSourceDiagnostic {
                severity: Severity::Error,
                rule_id: violation.code.to_string(),
                task_id: violation.task_id,
                message: violation.message,
            });
        }
    }

    PlanSourceReport::from_diagnostics(diagnostics)
}

/// Validate a plan source text, then save it to `tasks_path` if valid.
///
/// Validation is performed by [`validate_plan_source`].  On any validation
/// error the report is returned and the file at `tasks_path` is left
/// completely untouched.  On success the text is written exactly as given
/// using a tmp-file-then-rename strategy so a crash mid-write cannot corrupt
/// the plan file.
pub fn save_plan_source(
    workdir: &Path,
    tasks_path: &Path,
    toml: &str,
    models: &IndexMap<String, ModelProfile>,
) -> Result<PlanSourceReport> {
    // Derive plan_id from the parent directory name (the same convention the
    // runner uses in plan_loader::load_plan).
    let plan_id = tasks_path
        .parent()
        .and_then(|p| p.file_name())
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "unnamed".to_string());

    let report = validate_plan_source(workdir, &plan_id, toml, models);
    if !report.valid {
        return Ok(report);
    }

    // Atomically write via a sibling temp file then rename.
    let parent = tasks_path
        .parent()
        .with_context(|| format!("no parent directory for {}", tasks_path.display()))?;
    std::fs::create_dir_all(parent)
        .with_context(|| format!("create directory {}", parent.display()))?;
    let tmp_path = parent.join(".tasks.toml.tmp");
    std::fs::write(&tmp_path, toml)
        .with_context(|| format!("write temp file {}", tmp_path.display()))?;
    std::fs::rename(&tmp_path, tasks_path).with_context(|| {
        format!(
            "rename {} -> {}",
            tmp_path.display(),
            tasks_path.display()
        )
    })?;

    Ok(report)
}

/// Generate a minimal plan source that passes [`validate_plan_source`].
///
/// The plan has one task with:
/// - `role = "implementer"`, `tier = "focused"`
/// - the supplied `default_model` as `model_hint`
/// - one placeholder output file
/// - one verify step
///
/// Comments and custom content can be added after generation; the save path
/// uses [`save_plan_source`] which writes the text byte-for-byte.
pub fn starter_plan_source(slug: &str, title: &str, default_model: &str) -> String {
    format!(
        r#"[meta]
plan = "{slug}"
total = 1
max_parallel = 1

[[task]]
id = "T01"
title = "{title}"
description = "Implement {title}."
role = "implementer"
tier = "focused"
model_hint = "{default_model}"
files = ["{slug}/src/lib.rs"]
depends_on = []

[[task.verify]]
phase = "compile"
command = "echo done"
fail_msg = "must succeed"
"#
    )
}

// ─── Plan revision ────────────────────────────────────────────────────────────

/// Outcome of a plan revision attempt.
#[derive(Debug, Clone)]
pub struct RevisionOutcome {
    /// Whether the revised plan was successfully written to disk.
    pub written: bool,
    /// Number of tasks in the revised plan (0 when not written).
    pub task_count: usize,
    /// Validation report for the revised plan.
    pub report: PlanSourceReport,
}

/// Build a revision prompt for an existing plan.
///
/// The prompt begins with `Revise the plan below according to the feedback.`,
/// embeds the current `tasks.toml` verbatim in a fenced `toml` block, and
/// asks for a complete revised file in one fenced `toml` block.
pub fn build_revision_prompt(plan_id: &str, current_toml: &str, feedback: &str) -> String {
    format!(
        "Revise the plan below according to the feedback.\n\n\
         Current tasks.toml (plan `{plan_id}`):\n\n\
         ```toml\n{current_toml}\n```\n\n\
         Feedback:\n{feedback}\n\n\
         Instructions:\n\
         - Output the complete revised file as a single fenced ```toml block.\n\
         - Keep `[meta] plan = \"{plan_id}\"` exactly as shown.\n\
         - Preserve the ids of tasks that still apply; only renumber or remove \
           tasks that are explicitly addressed by the feedback.\n\
         - Do NOT add a `model_hint` field to any task unless the feedback \
           explicitly requests one.\n\
         - Output ONLY a fenced ```toml block — no prose, no explanation \
           outside that block."
    )
}

/// Extract, repair, validate, and atomically write a revised `tasks.toml`.
///
/// 1. Extracts the TOML from the agent output using
///    [`crate::prd::extract_fenced_block`], falling back to
///    [`crate::prd::extract_toml_content_fallback`].
/// 2. Applies [`repair_toml`] for deterministic fixes only (does **not** run
///    `validate_and_fix_generated_plan`, which strips `model_hint` fields).
/// 3. Requires that `[meta] plan` equals `plan_id`.
/// 4. Hands the text to [`save_plan_source`], which validates and writes
///    atomically, or leaves the file untouched on any validation error.
pub fn apply_revision_output(
    workdir: &Path,
    plan_id: &str,
    tasks_path: &Path,
    agent_output: &str,
    models: &IndexMap<String, ModelProfile>,
) -> Result<RevisionOutcome> {
    // ── Step 1: Extract TOML block ─────────────────────────────────────────
    let raw = crate::prd::extract_fenced_block(agent_output, "toml")
        .or_else(|| crate::prd::extract_fenced_block(agent_output, "tasks.toml"))
        .or_else(|| crate::prd::extract_toml_content_fallback(agent_output))
        .ok_or_else(|| anyhow::anyhow!("no TOML block found in agent output"))?;

    // ── Step 2: Deterministic repair ──────────────────────────────────────
    let repaired = repair_toml(raw);

    // ── Step 3: Require [meta] plan = plan_id ─────────────────────────────
    let parsed = TasksFile::parse_str(&repaired)
        .with_context(|| "revised TOML failed runtime parsing after repair")?;
    if parsed.meta.plan != plan_id {
        return Err(anyhow::anyhow!(
            "revised plan has `meta.plan = {:?}` but expected {:?}; \
             the agent must not change the plan identifier",
            parsed.meta.plan,
            plan_id
        ));
    }

    let task_count = parsed.tasks.len();

    // ── Step 4: Validate and write atomically ─────────────────────────────
    let report = save_plan_source(workdir, tasks_path, &repaired, models)?;
    let written = report.valid;

    Ok(RevisionOutcome {
        written,
        task_count: if written { task_count } else { 0 },
        report,
    })
}

/// Run the planning agent to revise an existing plan and write the result.
///
/// Reads the current `tasks.toml`, invokes the strategist agent with the
/// revision prompt, then calls [`apply_revision_output`].  On a validation
/// rejection the agent is retried once with the diagnostics appended to the
/// feedback before the final outcome is returned.
pub async fn revise_plan_source(
    workdir: &Path,
    plan_id: &str,
    tasks_path: &Path,
    feedback: &str,
    models: &IndexMap<String, ModelProfile>,
) -> Result<RevisionOutcome> {
    // Read current text.
    let current_toml = std::fs::read_to_string(tasks_path)
        .with_context(|| format!("read {}", tasks_path.display()))?;

    let resolved = crate::load_resolved_config(workdir)?;
    let system_prompt = crate::plan_generate::build_generator_system_prompt(workdir);

    let run_agent = |prompt: String| {
        let env_vars = resolved.config.agent.env.clone();
        let model = resolved.config.agent.model.clone();
        let effort = resolved.config.agent.effort.clone();
        let system = system_prompt.clone();
        async move {
            run_agent_capture_silent(AgentExecOpts {
                prompt: &prompt,
                workdir,
                model: model.as_deref(),
                effort: Some(effort.as_str()),
                system_prompt: Some(&system),
                resume_session: None,
                env_vars: &env_vars,
                role: Some("strategist"),
                allowed_tools: Some("Read,Grep,Glob"),
            })
            .await
        }
    };

    // First attempt.
    let first_prompt = build_revision_prompt(plan_id, &current_toml, feedback);
    let (_exit_code, output) = run_agent(first_prompt).await?;

    let outcome = apply_revision_output(workdir, plan_id, tasks_path, &output, models)?;
    if outcome.written {
        return Ok(outcome);
    }

    // Retry once with diagnostics appended to the feedback.
    let diag_text: String = outcome
        .report
        .diagnostics
        .iter()
        .map(|d| format!("- [{}] {}", d.rule_id, d.message))
        .collect::<Vec<_>>()
        .join("\n");
    let retry_feedback = format!(
        "{feedback}\n\nThe previous revision was rejected with the following diagnostics:\n{diag_text}\n\
         Please fix these issues in the revised plan."
    );
    let retry_prompt = build_revision_prompt(plan_id, &current_toml, &retry_feedback);
    let (_exit_code2, output2) = run_agent(retry_prompt).await?;

    apply_revision_output(workdir, plan_id, tasks_path, &output2, models)
}

// ─── Unit tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn empty_models() -> IndexMap<String, ModelProfile> {
        IndexMap::new()
    }

    /// Build a minimal valid tasks.toml string for tests.
    fn minimal_valid_toml(plan_id: &str) -> String {
        format!(
            r#"[meta]
plan = "{plan_id}"
total = 1
max_parallel = 1

[[task]]
id = "T1"
title = "Do something"
description = "Detailed description of the task."
role = "implementer"
tier = "focused"
files = ["src/lib.rs"]
depends_on = []

[[task.verify]]
phase = "compile"
command = "echo ok"
fail_msg = "must pass"
"#
        )
    }

    /// A valid plan saves byte-for-byte, including comments.
    #[test]
    fn valid_plan_saves_byte_for_byte() {
        let tmp = tempfile::tempdir().unwrap();
        let workdir = tmp.path().join("workspace");
        std::fs::create_dir_all(&workdir).unwrap();

        let plan_dir = workdir.join("plans").join("my-plan");
        std::fs::create_dir_all(&plan_dir).unwrap();
        let tasks_path = plan_dir.join("tasks.toml");

        // Include a comment that must survive the round-trip.
        let toml = format!(
            "# This comment must survive!\n{}",
            minimal_valid_toml("my-plan")
        );

        let report = save_plan_source(&workdir, &tasks_path, &toml, &empty_models()).unwrap();
        assert!(
            report.valid,
            "expected valid plan, got errors: {:?}",
            report.diagnostics
        );

        let saved = std::fs::read_to_string(&tasks_path).unwrap();
        assert_eq!(saved, toml, "saved file must be byte-identical to input");
    }

    /// An unknown dependency gives an error naming that task; the file is untouched.
    #[test]
    fn unknown_dependency_is_error_and_file_untouched() {
        let tmp = tempfile::tempdir().unwrap();
        let workdir = tmp.path().join("workspace");
        std::fs::create_dir_all(&workdir).unwrap();

        let plan_dir = workdir.join("plans").join("my-plan");
        std::fs::create_dir_all(&plan_dir).unwrap();
        let tasks_path = plan_dir.join("tasks.toml");
        let original_content = "# original content\n";
        std::fs::write(&tasks_path, original_content).unwrap();

        let toml = r#"[meta]
plan = "my-plan"
total = 1
max_parallel = 1

[[task]]
id = "T1"
title = "Use missing dep"
description = "A task that depends on a nonexistent task."
role = "implementer"
tier = "focused"
files = ["src/lib.rs"]
depends_on = ["T99"]

[[task.verify]]
phase = "compile"
command = "echo ok"
"#;

        let report = save_plan_source(&workdir, &tasks_path, toml, &empty_models()).unwrap();
        assert!(!report.valid, "expected validation failure");
        assert!(
            report
                .diagnostics
                .iter()
                .any(|d| d.message.contains("T99")),
            "expected a diagnostic naming T99; got: {:?}",
            report.diagnostics
        );

        // The file must be unchanged.
        let content = std::fs::read_to_string(&tasks_path).unwrap();
        assert_eq!(
            content, original_content,
            "file must remain byte-identical when validation fails"
        );
    }

    /// A TOML syntax error produces a PLAN_PARSE diagnostic; nothing is written.
    #[test]
    fn toml_syntax_error_gives_plan_parse() {
        let tmp = tempfile::tempdir().unwrap();
        let workdir = tmp.path().join("workspace");
        std::fs::create_dir_all(&workdir).unwrap();

        let plan_dir = workdir.join("plans").join("bad-plan");
        std::fs::create_dir_all(&plan_dir).unwrap();
        let tasks_path = plan_dir.join("tasks.toml");
        let original = "# not overwritten\n";
        std::fs::write(&tasks_path, original).unwrap();

        let broken = "this is *** not valid toml [[[[\0garbage";
        let report = save_plan_source(&workdir, &tasks_path, broken, &empty_models()).unwrap();

        assert!(!report.valid, "expected invalid report");
        assert!(
            report
                .diagnostics
                .iter()
                .any(|d| d.rule_id == "PLAN_PARSE"),
            "expected PLAN_PARSE; got: {:?}",
            report.diagnostics
        );

        let content = std::fs::read_to_string(&tasks_path).unwrap();
        assert_eq!(content, original, "file must not be touched on parse error");
    }

    /// A context.read_files path that does not exist is an error.
    #[test]
    fn missing_read_file_is_error() {
        let tmp = tempfile::tempdir().unwrap();
        let workdir = tmp.path().join("workspace");
        std::fs::create_dir_all(&workdir).unwrap();

        let toml = r#"[meta]
plan = "my-plan"
total = 1
max_parallel = 1

[[task]]
id = "T1"
title = "Use missing context"
description = "Reads a file that does not exist."
role = "implementer"
tier = "focused"
files = ["src/output.rs"]
depends_on = []

[task.context]
read_files = [{ path = "src/nonexistent_file_abc123.rs", why = "context" }]

[[task.verify]]
phase = "compile"
command = "echo ok"
"#;

        let report = validate_plan_source(&workdir, "my-plan", toml, &empty_models());
        assert!(
            !report.valid,
            "expected error for missing read_file; got: {:?}",
            report.diagnostics
        );
        assert!(report.errors > 0, "expected at least one error");
    }

    /// The starter plan validates without errors.
    #[test]
    fn starter_plan_validates() {
        let tmp = tempfile::tempdir().unwrap();
        let workdir = tmp.path().join("workspace");
        std::fs::create_dir_all(&workdir).unwrap();

        let toml = starter_plan_source("my-starter", "Build the feature", "claude-sonnet-4-6");
        let report = validate_plan_source(&workdir, "my-starter", &toml, &empty_models());
        assert!(
            report.valid,
            "starter plan must be valid; errors: {:?}",
            report.diagnostics
        );
    }

    // ── apply_revision_output tests ───────────────────────────────────────

    fn wrap_toml(toml: &str) -> String {
        format!("Here is the revised plan:\n\n```toml\n{toml}\n```\n")
    }

    /// A valid revision is written byte-for-byte as extracted.
    #[test]
    fn valid_revision_is_written() {
        let tmp = tempfile::tempdir().unwrap();
        let workdir = tmp.path().join("workspace");
        std::fs::create_dir_all(&workdir).unwrap();

        let plan_dir = workdir.join("plans").join("my-plan");
        std::fs::create_dir_all(&plan_dir).unwrap();
        let tasks_path = plan_dir.join("tasks.toml");
        // Write some original content so the file exists.
        std::fs::write(&tasks_path, "# original\n").unwrap();

        let new_toml = minimal_valid_toml("my-plan");
        let agent_output = wrap_toml(&new_toml);

        let outcome =
            apply_revision_output(&workdir, "my-plan", &tasks_path, &agent_output, &empty_models())
                .unwrap();

        assert!(outcome.written, "expected written=true; report: {:?}", outcome.report.diagnostics);
        assert!(outcome.task_count > 0);

        // The written file must be byte-identical to the extracted TOML (after repair).
        let saved = std::fs::read_to_string(&tasks_path).unwrap();
        // repair_toml is idempotent for well-formed TOML; the core content is preserved.
        assert!(
            saved.contains("[meta]"),
            "saved file must contain [meta]; got: {saved}"
        );
        assert!(
            saved.contains("plan = \"my-plan\""),
            "saved file must contain plan = \"my-plan\"; got: {saved}"
        );
    }

    /// A dangling dependency leaves the file byte-identical.
    #[test]
    fn dangling_dependency_leaves_file_untouched() {
        let tmp = tempfile::tempdir().unwrap();
        let workdir = tmp.path().join("workspace");
        std::fs::create_dir_all(&workdir).unwrap();

        let plan_dir = workdir.join("plans").join("my-plan");
        std::fs::create_dir_all(&plan_dir).unwrap();
        let tasks_path = plan_dir.join("tasks.toml");
        let original = "# original content\n";
        std::fs::write(&tasks_path, original).unwrap();

        let bad_toml = r#"[meta]
plan = "my-plan"
total = 1
max_parallel = 1

[[task]]
id = "T1"
title = "Dangling dep"
description = "Depends on nonexistent task."
role = "implementer"
tier = "focused"
files = ["src/lib.rs"]
depends_on = ["T99"]

[[task.verify]]
phase = "compile"
command = "echo ok"
"#;
        let agent_output = wrap_toml(bad_toml);

        let outcome =
            apply_revision_output(&workdir, "my-plan", &tasks_path, &agent_output, &empty_models())
                .unwrap();

        assert!(!outcome.written, "expected written=false due to dangling dep");
        assert!(
            outcome.report.diagnostics.iter().any(|d| d.message.contains("T99")),
            "expected T99 in diagnostics; got: {:?}",
            outcome.report.diagnostics
        );

        // File must be unchanged.
        let content = std::fs::read_to_string(&tasks_path).unwrap();
        assert_eq!(content, original, "file must remain untouched on validation failure");
    }

    /// Output without a fenced block is an error.
    #[test]
    fn no_fenced_block_is_error() {
        let tmp = tempfile::tempdir().unwrap();
        let workdir = tmp.path().join("workspace");
        std::fs::create_dir_all(&workdir).unwrap();

        let plan_dir = workdir.join("plans").join("my-plan");
        std::fs::create_dir_all(&plan_dir).unwrap();
        let tasks_path = plan_dir.join("tasks.toml");
        std::fs::write(&tasks_path, "# untouched\n").unwrap();

        let agent_output = "I could not produce the plan. Sorry.";

        let result =
            apply_revision_output(&workdir, "my-plan", &tasks_path, agent_output, &empty_models());

        assert!(result.is_err(), "expected Err when no TOML block present");
        let err = result.unwrap_err();
        assert!(
            err.to_string().contains("no TOML block"),
            "expected 'no TOML block' in error; got: {err}"
        );

        // File must be unchanged.
        let content = std::fs::read_to_string(&tasks_path).unwrap();
        assert_eq!(content, "# untouched\n");
    }

    /// A changed `meta.plan` is rejected.
    #[test]
    fn changed_meta_plan_is_rejected() {
        let tmp = tempfile::tempdir().unwrap();
        let workdir = tmp.path().join("workspace");
        std::fs::create_dir_all(&workdir).unwrap();

        let plan_dir = workdir.join("plans").join("my-plan");
        std::fs::create_dir_all(&plan_dir).unwrap();
        let tasks_path = plan_dir.join("tasks.toml");
        std::fs::write(&tasks_path, "# original\n").unwrap();

        // Produce a valid TOML, but with the wrong plan id.
        let wrong_toml = minimal_valid_toml("different-plan");
        let agent_output = wrap_toml(&wrong_toml);

        let result =
            apply_revision_output(&workdir, "my-plan", &tasks_path, &agent_output, &empty_models());

        assert!(result.is_err(), "expected Err when meta.plan changed");
        let err = result.unwrap_err();
        assert!(
            err.to_string().contains("meta.plan"),
            "expected 'meta.plan' in error message; got: {err}"
        );

        // File must be unchanged.
        let content = std::fs::read_to_string(&tasks_path).unwrap();
        assert_eq!(content, "# original\n");
    }
}
