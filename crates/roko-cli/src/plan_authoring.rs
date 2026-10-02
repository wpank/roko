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
//! - [`plan_diff`] — what a revision changed, task by task.
//! - [`last_run_failure_context`] — how the plan's last run failed, with the
//!   failed steps' gate output, for a revision prompt.
//! - [`AuthoringSpend`] — record what the agent calls of a generation or revision cost.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use anyhow::{Context as _, Result};
use indexmap::IndexMap;
use roko_core::config::schema::ModelProfile;
use roko_learn::costs_db::CostRecord;
use roko_learn::efficiency::AgentEfficiencyEvent;

use crate::agent_exec::{AgentCapture, AgentExecOpts, run_agent_capture_silent_with_usage};
use crate::model_selection::resolve_planner_model;
use crate::plan_policy::{PlanExecutionPolicy, validate_plan_context};
use crate::plan_validate::{Severity, validate_plans_dir_with_workdir};
use crate::runner::tui_bridge::TuiBridge;
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
    for violation in validate_plan_context(
        &tasks_file,
        workdir,
        &plan_dir,
        PlanExecutionPolicy::for_environment(),
    ) {
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
    std::fs::rename(&tmp_path, tasks_path)
        .with_context(|| format!("rename {} -> {}", tmp_path.display(), tasks_path.display()))?;

    Ok(report)
}

/// Generate a minimal plan source that passes [`validate_plan_source`].
///
/// The plan has one task with:
/// - `role = "implementer"`, `tier = "focused"`, and no model hint: its tier
///   and role pick its model on the routing ladder
/// - one placeholder output file
/// - one verify step
///
/// Comments and custom content can be added after generation; the save path
/// uses [`save_plan_source`] which writes the text byte-for-byte.
pub fn starter_plan_source(slug: &str, title: &str) -> String {
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
    /// What the revision changed, task by task; set when it was written.
    pub diff: Option<PlanDiff>,
}

// ─── Plan diff ────────────────────────────────────────────────────────────────

/// What a revision changed in a plan, task by task (3216): the tasks it added
/// and removed, and for each task in both plans every key whose value
/// changed. A structural diff of the parsed tables, matched by task id, not a
/// line diff.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize)]
pub struct PlanDiff {
    /// `[meta]` keys whose value changed.
    pub meta: Vec<KeyChange>,
    /// Ids of the tasks the new plan adds, in its order.
    pub added: Vec<String>,
    /// Ids of the tasks the new plan drops, in the old plan's order.
    pub removed: Vec<String>,
    /// The tasks in both plans whose keys changed, in the new plan's order.
    pub changed: Vec<TaskChange>,
}

/// One key whose value changed, each side as TOML text (a string as its
/// text); `None` on the side where the key is absent.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct KeyChange {
    /// The key, such as `verify` or `files`.
    pub key: String,
    /// Its value in the old plan.
    pub before: Option<String>,
    /// Its value in the new plan.
    pub after: Option<String>,
}

/// The keys of one task that a revision changed.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct TaskChange {
    /// The task id.
    pub id: String,
    /// Its changed keys: the old task's keys in order, then the new ones.
    pub keys: Vec<KeyChange>,
}

impl PlanDiff {
    /// Whether the two plans are the same, key for key.
    pub fn is_empty(&self) -> bool {
        self.meta.is_empty()
            && self.added.is_empty()
            && self.removed.is_empty()
            && self.changed.is_empty()
    }

    /// The diff for a terminal: a count line, `+ T4` for each added task,
    /// `- T3` for each removed one, then `~ T1` (or `~ [meta]`) with each
    /// changed key's before and after under it.
    pub fn render_text(&self) -> String {
        if self.is_empty() {
            return "plan diff: no changes".to_string();
        }
        let mut lines = vec![format!(
            "plan diff: {} added, {} removed, {} changed{}",
            self.added.len(),
            self.removed.len(),
            self.changed.len(),
            if self.meta.is_empty() {
                ""
            } else {
                "; [meta] changed"
            }
        )];
        lines.extend(self.added.iter().map(|id| format!("  + {id}")));
        lines.extend(self.removed.iter().map(|id| format!("  - {id}")));
        let meta = (!self.meta.is_empty()).then_some(("[meta]", &self.meta));
        let tasks = self
            .changed
            .iter()
            .map(|task| (task.id.as_str(), &task.keys));
        for (name, keys) in meta.into_iter().chain(tasks) {
            lines.push(format!("  ~ {name}"));
            for change in keys {
                lines.push(format!(
                    "      {}: {} -> {}",
                    change.key,
                    change.before.as_deref().unwrap_or("(none)"),
                    change.after.as_deref().unwrap_or("(none)")
                ));
            }
        }
        lines.join("\n")
    }
}

/// The task-level diff from `old` to `new`, two `tasks.toml` texts. A text
/// that does not parse counts as an empty plan.
pub fn plan_diff(old: &str, new: &str) -> PlanDiff {
    let (old, new) = (parse_plan_table(old), parse_plan_table(new));
    let (old_tasks, new_tasks) = (plan_tasks(&old), plan_tasks(&new));
    let added = new_tasks
        .iter()
        .filter(|(id, _)| task_by_id(&old_tasks, id).is_none())
        .map(|(id, _)| id.clone())
        .collect();
    let removed = old_tasks
        .iter()
        .filter(|(id, _)| task_by_id(&new_tasks, id).is_none())
        .map(|(id, _)| id.clone())
        .collect();
    let changed = new_tasks
        .iter()
        .filter_map(|(id, task)| {
            let keys = key_changes(task_by_id(&old_tasks, id)?, task);
            (!keys.is_empty()).then(|| TaskChange {
                id: id.clone(),
                keys,
            })
        })
        .collect();
    let meta = |plan: &toml::Table| {
        let meta = plan.get("meta").and_then(toml::Value::as_table);
        meta.cloned().unwrap_or_default()
    };
    PlanDiff {
        meta: key_changes(&meta(&old), &meta(&new)),
        added,
        removed,
        changed,
    }
}

fn parse_plan_table(text: &str) -> toml::Table {
    toml::from_str(text).unwrap_or_default()
}

/// A plan's `[[task]]` tables with their ids, in order.
fn plan_tasks(plan: &toml::Table) -> Vec<(String, toml::Table)> {
    plan.get("task")
        .and_then(toml::Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(toml::Value::as_table)
        .map(|task| {
            let id = task.get("id").and_then(toml::Value::as_str);
            (id.unwrap_or_default().to_string(), task.clone())
        })
        .collect()
}

fn task_by_id<'a>(tasks: &'a [(String, toml::Table)], id: &str) -> Option<&'a toml::Table> {
    tasks
        .iter()
        .find(|(other, _)| other == id)
        .map(|(_, task)| task)
}

/// The keys whose values differ between two tables: the old table's keys in
/// order, then the keys only the new one has.
fn key_changes(old: &toml::Table, new: &toml::Table) -> Vec<KeyChange> {
    let mut keys: Vec<&String> = old.keys().collect();
    keys.extend(new.keys().filter(|key| !old.contains_key(key.as_str())));
    keys.into_iter()
        .filter_map(|key| {
            let (before, after) = (old.get(key), new.get(key));
            (before != after).then(|| KeyChange {
                key: key.clone(),
                before: before.map(value_text),
                after: after.map(value_text),
            })
        })
        .collect()
}

/// A value as TOML text; a string as its text.
fn value_text(value: &toml::Value) -> String {
    match value {
        toml::Value::String(text) => text.clone(),
        other => other.to_string(),
    }
}

/// Build a revision prompt for an existing plan.
///
/// The prompt begins with `Revise the plan below according to the feedback.`,
/// embeds the current `tasks.toml` verbatim in a fenced `toml` block, and
/// asks for a complete revised file in one fenced `toml` block. When the
/// plan's last run failed, `last_failure` says how, so the agent can target
/// it without the user pasting it into the feedback (gap-3bea93).
pub fn build_revision_prompt(
    plan_id: &str,
    current_toml: &str,
    feedback: &str,
    last_failure: Option<&str>,
) -> String {
    let failure = match last_failure {
        Some(failure) => format!(
            "The plan's last run failed. Unless the feedback says otherwise, \
             revise the plan so it does not fail this way again:\n{failure}\n\n"
        ),
        None => String::new(),
    };
    format!(
        "Revise the plan below according to the feedback.\n\n\
         Current tasks.toml (plan `{plan_id}`):\n\n\
         ```toml\n{current_toml}\n```\n\n\
         Feedback:\n{feedback}\n\n\
         {failure}\
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
    // The plan as it was, for the diff (3216).
    let before = std::fs::read_to_string(tasks_path).unwrap_or_default();

    // ── Step 4: Validate and write atomically ─────────────────────────────
    let report = save_plan_source(workdir, tasks_path, &repaired, models)?;
    let written = report.valid;

    Ok(RevisionOutcome {
        written,
        task_count: if written { task_count } else { 0 },
        report,
        diff: written.then(|| plan_diff(&before, &repaired)),
    })
}

/// Most failed tasks a revision prompt lists from the plan's last run.
const MAX_REVISION_FAILED_TASKS: usize = 5;

/// Most distinct failed verify steps a revision prompt lists per failed task.
const MAX_REVISION_FAILED_STEPS: usize = 3;

/// The fewest characters the last-run section of a revision prompt may take,
/// whatever the planner's context window.
pub const MIN_REVISION_FAILURE_CHARS: usize = 8_000;

/// The characters the last-run section of a revision prompt may take: a
/// quarter of the planner's context window at about four characters a token,
/// as plan generation budgets its source, and at least
/// [`MIN_REVISION_FAILURE_CHARS`].
pub fn revision_failure_budget(context_window: Option<u64>) -> usize {
    context_window
        .map_or(0, |window| usize::try_from(window).unwrap_or(usize::MAX))
        .max(MIN_REVISION_FAILURE_CHARS)
}

/// How the plan's last run failed, for a revision or regeneration prompt
/// (3215): for each failed task (at most 5), why it failed, its attempts and
/// the models they ran on, then each distinct failed verify step, newest
/// first and at most 3, with its command, its failure class and the gate's
/// output, from the report `roko diagnose` prints. At most `budget`
/// characters (never fewer than [`MIN_REVISION_FAILURE_CHARS`]), cut with a
/// visible marker. `None` when the plan has no failed run on record.
pub fn last_run_failure_context(workdir: &Path, plan_id: &str, budget: usize) -> Option<String> {
    use crate::commands::diagnose::{TaskState, build_report};

    let report = build_report(workdir, plan_id, false).ok()?;
    if report.status != "failed" {
        return None;
    }
    let sections: Vec<String> = report
        .tasks
        .iter()
        .filter(|task| task.state == TaskState::Failed)
        .take(MAX_REVISION_FAILED_TASKS)
        .map(failed_task_context)
        .collect();
    if sections.is_empty() {
        return None;
    }
    let text = sections.join("\n");
    let budget = budget.max(MIN_REVISION_FAILURE_CHARS);
    if text.chars().count() <= budget {
        return Some(text);
    }
    let marker = format!("\n[... the last-run failure is cut here at {budget} characters ...]");
    let kept: String = text
        .chars()
        .take(budget.saturating_sub(marker.chars().count()))
        .collect();
    Some(kept + &marker)
}

/// One failed task's part of [`last_run_failure_context`].
fn failed_task_context(task: &crate::commands::diagnose::TaskDiagnosis) -> String {
    use crate::commands::diagnose::enum_label;

    // The reason's first line only: its "last error" goes on to carry the
    // whole gate output, which the failure lines below already show once.
    let reason = task.reason.lines().next().unwrap_or_default();
    let mut lines = vec![format!("- task `{}`: {reason}", task.task_id)];
    let mut models: Vec<&str> = Vec::new();
    for attempt in &task.attempts {
        if !models.contains(&attempt.model.as_str()) {
            models.push(&attempt.model);
        }
    }
    lines.push(format!(
        "  attempts: {} ({} failed, {} timed out); models tried: {}",
        task.attempt_count,
        task.failed_attempts,
        task.timed_out_attempts,
        if models.is_empty() {
            "none recorded".to_string()
        } else {
            models.join(", ")
        }
    ));
    // Each distinct failure (by step and output), newest first, with how
    // often it happened.
    let mut distinct: Vec<(&crate::commands::diagnose::GateFailureInfo, usize)> = Vec::new();
    for failure in task.gate_failures.iter().rev() {
        let step = failure.verify_step.as_ref().map(|step| step.index);
        let seen = distinct.iter_mut().find(|(other, _)| {
            other.verify_step.as_ref().map(|step| step.index) == step
                && other.summary == failure.summary
        });
        match seen {
            Some((_, times)) => *times += 1,
            None => distinct.push((failure, 1)),
        }
    }
    for (failure, times) in distinct.into_iter().take(MAX_REVISION_FAILED_STEPS) {
        let step = failure.verify_step.as_ref();
        let name = step.map_or_else(
            || "a verify step".to_string(),
            |step| match &step.phase {
                Some(phase) => format!("verify step {} ({phase})", step.index + 1),
                None => format!("verify step {}", step.index + 1),
            },
        );
        let command = step
            .and_then(|step| step.command.as_deref())
            .map_or_else(String::new, |command| format!(": `{command}`"));
        lines.push(format!(
            "  failed {name}{command}, {times} time{}; class {}, {} failure; gate output:",
            if times == 1 { "" } else { "s" },
            enum_label(&failure.primary_class),
            enum_label(&failure.failure_kind)
        ));
        lines.extend(failure.summary.lines().map(|line| format!("    {line}")));
    }
    if task.gate_failures.is_empty()
        && let Some(error) = &task.last_error
    {
        lines.push(format!("  last error: {error}"));
    }
    lines.join("\n")
}

/// Run the planning agent to revise an existing plan and write the result.
///
/// Reads the current `tasks.toml`, invokes the strategist agent on the planner
/// model ([`resolve_planner_model`]) with the revision prompt, then calls
/// [`apply_revision_output`]. The prompt carries how the plan's last run failed
/// when it did ([`build_revision_prompt`]). On a validation rejection the agent
/// is asked again, with the diagnostics appended to the feedback, up to
/// `[serve] revision_max_retries` times (once by default) before the final
/// outcome is returned.
///
/// Every agent call's spend is recorded against the plan through
/// [`AuthoringSpend::revision`], and published on `live` when given.
pub async fn revise_plan_source(
    workdir: &Path,
    plan_id: &str,
    tasks_path: &Path,
    feedback: &str,
    models: &IndexMap<String, ModelProfile>,
    live: Option<TuiBridge>,
) -> Result<RevisionOutcome> {
    // Read current text.
    let current_toml = std::fs::read_to_string(tasks_path)
        .with_context(|| format!("read {}", tasks_path.display()))?;

    let resolved = crate::load_resolved_config(workdir)?;
    let max_retries = resolved.config.serve.revision_max_retries;
    let planner_model = resolve_planner_model(workdir, None, "plan revision")?;
    let system_prompt = crate::plan_generate::build_generator_system_prompt(workdir);
    let spend = AuthoringSpend::revision(workdir, plan_id, live);

    let run_agent = |prompt: String| {
        let env_vars = resolved.config.agent.env.clone();
        let model = planner_model.clone();
        let effort = resolved.config.agent.effort.clone();
        let system = system_prompt.clone();
        let spend = &spend;
        async move {
            let call = run_agent_capture_silent_with_usage(AgentExecOpts {
                prompt: &prompt,
                workdir,
                model: Some(model.as_str()),
                effort: Some(effort.as_str()),
                system_prompt: Some(&system),
                resume_session: None,
                env_vars: &env_vars,
                role: Some("strategist"),
                allowed_tools: Some("Read,Grep,Glob"),
            })
            .await?;
            spend.record(&call).await;
            Ok::<_, anyhow::Error>(call.output)
        }
    };

    let budget =
        revision_failure_budget(crate::prd::planner_context_window(models, &planner_model));
    let last_failure = last_run_failure_context(workdir, plan_id, budget);

    // First attempt.
    let first_prompt =
        build_revision_prompt(plan_id, &current_toml, feedback, last_failure.as_deref());
    let output = run_agent(first_prompt).await?;

    let mut outcome = apply_revision_output(workdir, plan_id, tasks_path, &output, models)?;

    // Ask again while the revision is rejected, each time with the last
    // rejection's diagnostics appended to the feedback (gap-b3e513).
    for _ in 0..max_retries {
        if outcome.written {
            break;
        }
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
        let retry_prompt = build_revision_prompt(
            plan_id,
            &current_toml,
            &retry_feedback,
            last_failure.as_deref(),
        );
        let output = run_agent(retry_prompt).await?;
        outcome = apply_revision_output(workdir, plan_id, tasks_path, &output, models)?;
    }
    Ok(outcome)
}

// ─── Spend accounting ─────────────────────────────────────────────────────────

/// Pseudo task id that plan generation spend is attributed to.
pub const GENERATION_SPEND_TASK_ID: &str = "generate";

/// Pseudo task id that plan revision spend is attributed to.
pub const REVISION_SPEND_TASK_ID: &str = "revise";

/// Role that plan generation and revision run their agents as.
const AUTHORING_ROLE: &str = "strategist";

/// Records the provider spend of one plan generation or revision, or of a
/// one-off agent operation outside any plan, one agent call at a time.
///
/// A task dispatch records its spend three ways: a cost record in
/// `.roko/learn/costs.jsonl`, an efficiency row in
/// `.roko/learn/efficiency.jsonl` (the dashboard snapshot seeds
/// `stats.cost_usd_total` from it), and live `efficiency_event`s on the
/// StateHub (the snapshot adds them to that total). Generation and revision
/// run their agents outside the Graph engine, so they record through this
/// instead: the same three records, attributed to the plan under a pseudo task
/// id ([`GENERATION_SPEND_TASK_ID`] or [`REVISION_SPEND_TASK_ID`]). Each call
/// is recorded as it returns, so a retry or a failed operation is counted too.
/// Research, `roko do` and PRD drafting record through
/// [`AuthoringSpend::operation`]: the same records, with no plan id, under the
/// operation's own task id and role.
pub struct AuthoringSpend {
    learn_dir: PathBuf,
    plan_id: String,
    task_id: String,
    role: String,
    live: Option<TuiBridge>,
    calls: AtomicU32,
}

impl AuthoringSpend {
    /// Spend of generating the plan `plan_id` in `workdir`.
    #[must_use]
    pub fn generation(workdir: &Path, plan_id: &str, live: Option<TuiBridge>) -> Self {
        Self::new(
            workdir,
            plan_id,
            GENERATION_SPEND_TASK_ID,
            AUTHORING_ROLE,
            live,
        )
    }

    /// Spend of revising the plan `plan_id` in `workdir`.
    #[must_use]
    pub fn revision(workdir: &Path, plan_id: &str, live: Option<TuiBridge>) -> Self {
        Self::new(
            workdir,
            plan_id,
            REVISION_SPEND_TASK_ID,
            AUTHORING_ROLE,
            live,
        )
    }

    /// Spend of a one-off agent operation in `workdir` outside any plan, such
    /// as research, `roko do` or PRD drafting, recorded under `task_id` and
    /// `role` (bug-86ff56).
    #[must_use]
    pub fn operation(workdir: &Path, task_id: &str, role: &str) -> Self {
        Self::new(workdir, "", task_id, role, None)
    }

    fn new(
        workdir: &Path,
        plan_id: &str,
        task_id: &str,
        role: &str,
        live: Option<TuiBridge>,
    ) -> Self {
        Self {
            learn_dir: roko_fs::RokoLayout::for_project(workdir).learn_dir(),
            plan_id: plan_id.to_string(),
            task_id: task_id.to_string(),
            role: role.to_string(),
            live,
            calls: AtomicU32::new(0),
        }
    }

    /// Record one agent call. Best-effort: a failed write is logged and never
    /// fails the operation.
    pub async fn record(&self, call: &AgentCapture) {
        let attempt = self.calls.fetch_add(1, Ordering::Relaxed) + 1;
        let usage = call.usage;
        let cost_usd = f64::from(usage.cost_usd);
        let input_tokens = u64::from(usage.input_tokens);
        let output_tokens = u64::from(usage.output_tokens);
        let cache_read_tokens = u64::from(usage.cache_read_tokens);
        let cache_write_tokens = u64::from(usage.cache_create_tokens);
        let succeeded = call.exit_code == 0;
        let timestamp = chrono::Utc::now().to_rfc3339();
        // The plan and its pseudo task, or an operation's task alone.
        let scope = if self.plan_id.is_empty() {
            self.task_id.clone()
        } else {
            format!("{}/{}", self.plan_id, self.task_id)
        };

        // The capture names no model profile: a known cost or the model's
        // built-in rate prices the call (backlog 2109).
        let priced = crate::dispatch_v2::usage_is_priced(&usage, None, &call.model);
        let cost_record = CostRecord {
            timestamp: timestamp.clone(),
            model: call.model.clone(),
            provider: call.provider.clone(),
            role: self.role.clone(),
            plan_id: self.plan_id.clone(),
            task_id: self.task_id.clone(),
            complexity_band: "standard".to_string(),
            input_tokens,
            output_tokens,
            cached_tokens: cache_read_tokens,
            cost_usd,
            duration_ms: call.duration_ms,
            success: succeeded,
            session_id: String::new(),
            // An `AgentCapture` does not say where its usage came from.
            cost_source: roko_learn::telemetry::CostSource::Unknown,
            priced: Some(priced),
        };
        self.append("costs.jsonl", &cost_record).await;

        let efficiency_event = AgentEfficiencyEvent {
            agent_id: scope.clone(),
            role: self.role.clone(),
            backend: call.provider.clone(),
            model: call.model.clone(),
            plan_id: self.plan_id.clone(),
            task_id: self.task_id.clone(),
            attempt_id: format!("{scope}/a{attempt}"),
            input_tokens,
            output_tokens,
            reasoning_tokens: u64::from(usage.reasoning_tokens),
            cache_read_tokens,
            cache_write_tokens,
            cost_usd,
            cost_usd_without_cache: cost_usd,
            prompt_sections: Vec::new(),
            total_prompt_tokens: input_tokens,
            system_prompt_tokens: 0,
            tools_available: 0,
            tools_used: 0,
            tool_calls: Vec::new(),
            wall_time_ms: call.duration_ms,
            duration_ms: call.duration_ms,
            time_to_first_token_ms: 0,
            was_warm_start: false,
            iteration: attempt,
            turn_number: 0,
            is_final_turn: true,
            gate_passed: None,
            outcome: if succeeded { "success" } else { "failure" }.to_string(),
            gate_errors: Vec::new(),
            model_used: call.model.clone(),
            frequency: roko_core::OperatingFrequency::Theta,
            strategy_attempted: String::new(),
            timestamp,
        };
        self.append("efficiency.jsonl", &efficiency_event).await;

        if let Some(live) = &self.live {
            live.token_usage(
                &self.plan_id,
                &self.task_id,
                input_tokens,
                output_tokens,
                cache_read_tokens,
                cache_write_tokens,
            );
            live.efficiency_event(&self.plan_id, &self.task_id, "cost_usd", cost_usd);
        }
    }

    async fn append(&self, file_name: &str, record: &impl serde::Serialize) {
        let path = self.learn_dir.join(file_name);
        let line = match serde_json::to_string(record) {
            Ok(line) => line,
            Err(error) => {
                tracing::warn!(path = %path.display(), %error, "authoring spend serialization failed");
                return;
            }
        };
        let max_mb = roko_core::config::ResourcesConfig::default().log_rotation_max_mb;
        let written = tokio::task::spawn_blocking({
            let path = path.clone();
            move || roko_fs::log_rotation::append_jsonl_line_sync(&path, line.as_bytes(), max_mb)
        })
        .await;
        if let Err(error) = written
            .map_err(std::io::Error::other)
            .and_then(|result| result.map(|_| ()))
        {
            tracing::warn!(
                path = %path.display(),
                plan_id = %self.plan_id,
                task_id = %self.task_id,
                %error,
                "authoring spend write failed (best-effort)"
            );
        }
    }
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
            report.diagnostics.iter().any(|d| d.message.contains("T99")),
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
            report.diagnostics.iter().any(|d| d.rule_id == "PLAN_PARSE"),
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

        let toml = starter_plan_source("my-starter", "Build the feature");
        let report = validate_plan_source(&workdir, "my-starter", &toml, &empty_models());
        assert!(
            report.valid,
            "starter plan must be valid; errors: {:?}",
            report.diagnostics
        );
        // gap-dbf2a6: the ladder picks its model; no hint pins one.
        assert!(!toml.contains("model_hint"), "{toml}");
    }

    // ── apply_revision_output tests ───────────────────────────────────────

    /// gap-3bea93: a revision prompt says how the plan's last run failed,
    /// before the instructions, and says nothing about a run when it has no
    /// failure to report.
    #[test]
    fn revision_prompt_carries_the_last_run_failure() {
        let current = minimal_valid_toml("my-plan");
        let failure = "- task `T1`: its verify step failed; last error: test parse ... FAILED";

        let prompt = build_revision_prompt("my-plan", &current, "split T1", Some(failure));
        let section = prompt.find("last run failed").expect("failure section");
        assert!(prompt.contains(failure), "{prompt}");
        assert!(section > prompt.find("split T1").expect("feedback"));
        assert!(section < prompt.find("Instructions:").expect("instructions"));

        let plain = build_revision_prompt("my-plan", &current, "split T1", None);
        assert!(!plain.contains("last run"), "{plain}");
    }

    /// 3215: the revision prompt carries the failed attempts' gate output,
    /// not a one-line summary. T2 failed `cargo test -p x parse` twice the
    /// same way: the prompt names the step's command, its failure class and
    /// an excerpt of its output, once, with how often it failed. A small
    /// budget cuts the section with a visible marker.
    #[test]
    fn revision_prompt_carries_failed_gate_output() {
        use crate::graph_checkpoint::{GraphCheckpointStatus, start_plan_checkpoint};
        use roko_gate::{FailureClass, GateFailureAction, GateFailureKind, GateFailureRecord};
        use roko_learn::telemetry::CostSource;

        let tmp = tempfile::tempdir().expect("tempdir");
        let workdir = tmp.path();
        let plan_dir = workdir.join("plans").join("my-plan");
        std::fs::create_dir_all(&plan_dir).expect("plan dir");
        let tasks = minimal_valid_toml("my-plan").replace("total = 1", "total = 2")
            + r#"
[[task]]
id = "T2"
title = "Parse the retry limit"
description = "Parse `retries` in `parse_config`."
role = "implementer"
tier = "focused"
files = ["src/parse.rs"]
depends_on = ["T1"]

[[task.verify]]
phase = "test"
command = "cargo test -p x parse"
"#;
        std::fs::write(plan_dir.join("tasks.toml"), &tasks).expect("write tasks.toml");
        let plan = crate::runner::plan_loader::Plan {
            id: "my-plan".to_string(),
            dir: plan_dir.clone(),
            tasks: TasksFile::parse(&plan_dir.join("tasks.toml")).expect("parse tasks.toml"),
            prd_excerpt: String::new(),
        };
        let mut checkpoint = start_plan_checkpoint(workdir, &plan).expect("checkpoint");
        checkpoint
            .take_cost_ledger()
            .persist(0, 0)
            .expect("cost ledger");
        checkpoint
            .finish_with_status(GraphCheckpointStatus::Failed)
            .expect("finish the run");

        let now = chrono::Utc::now();
        let attempt = |seconds: i64| CostRecord {
            timestamp: (now + chrono::Duration::seconds(seconds)).to_rfc3339(),
            model: "glm-4.7".into(),
            provider: "openai_compat".into(),
            role: "implementer".into(),
            plan_id: "my-plan".into(),
            task_id: "T2".into(),
            complexity_band: "focused".into(),
            input_tokens: 100,
            output_tokens: 50,
            cached_tokens: 0,
            cost_usd: 0.01,
            duration_ms: 60_000,
            success: false,
            session_id: String::new(),
            cost_source: CostSource::CliUsage,
            priced: None,
        };
        let output = "verify[0:test] (`cargo test -p x parse`) failed: exit status 101\n\
                      ---- parse::rejects_an_empty_limit stdout ----\n\
                      thread 'parse::rejects_an_empty_limit' panicked at src/parse.rs:12:5:\n\
                      assertion failed: limit.is_err()\n\
                      test result: FAILED. 3 passed; 1 failed";
        let failure = |seconds: i64| GateFailureRecord {
            plan_id: "my-plan".into(),
            task_id: "T2".into(),
            gate_name: "graph-verify".into(),
            rung: 0,
            failure_kind: GateFailureKind::Permanent,
            primary_class: FailureClass::TestExpectationFailure,
            summary: output.into(),
            recommended_action: GateFailureAction::Retry,
            cargo_fix_candidate: false,
            replan_candidate: false,
            error_count: 1,
            warning_count: 0,
            timestamp: now + chrono::Duration::seconds(seconds),
        };
        let learn = workdir.join(".roko/learn");
        std::fs::create_dir_all(&learn).expect("learn dir");
        let jsonl = |rows: Vec<String>| rows.join("\n") + "\n";
        let costs = [attempt(10), attempt(20)]
            .iter()
            .map(|row| serde_json::to_string(row).expect("serialize"))
            .collect();
        std::fs::write(learn.join("costs.jsonl"), jsonl(costs)).expect("costs");
        let failures = [failure(11), failure(21)]
            .iter()
            .map(|row| serde_json::to_string(row).expect("serialize"))
            .collect();
        std::fs::write(learn.join("gate-failures.jsonl"), jsonl(failures)).expect("failures");

        let context = last_run_failure_context(workdir, "my-plan", MIN_REVISION_FAILURE_CHARS)
            .expect("the failed run is on record");
        assert!(context.starts_with("- task `T2`:"), "{context}");
        assert!(context.contains("models tried: glm-4.7"), "{context}");
        assert!(
            context.contains("failed verify step 1 (test): `cargo test -p x parse`, 2 times"),
            "{context}"
        );
        assert!(
            context.contains("class test_expectation_failure, permanent failure"),
            "{context}"
        );
        assert!(
            context.contains("    assertion failed: limit.is_err()"),
            "{context}"
        );
        assert_eq!(
            context.matches("rejects_an_empty_limit stdout").count(),
            1,
            "{context}"
        );

        let prompt = build_revision_prompt("my-plan", &tasks, "split T2", Some(context.as_str()));
        assert!(
            prompt.contains("test result: FAILED. 3 passed; 1 failed"),
            "{prompt}"
        );

        // The budget is a quarter of the planner's window, never under the
        // floor; a section over it is cut with a marker.
        assert_eq!(revision_failure_budget(None), MIN_REVISION_FAILURE_CHARS);
        assert_eq!(revision_failure_budget(Some(200_000)), 200_000);
        let padded = "x".repeat(MIN_REVISION_FAILURE_CHARS * 2);
        std::fs::write(
            learn.join("gate-failures.jsonl"),
            jsonl(vec![
                serde_json::to_string(&GateFailureRecord {
                    summary: format!("verify[0:test] failed\n{padded}"),
                    ..failure(30)
                })
                .expect("serialize"),
            ]),
        )
        .expect("failures");
        let cut = last_run_failure_context(workdir, "my-plan", 0).expect("still failed");
        assert_eq!(cut.chars().count(), MIN_REVISION_FAILURE_CHARS);
        assert!(
            cut.ends_with("characters ...]"),
            "{}",
            &cut[cut.len() - 80..]
        );
    }

    /// A plan with no run on record has no failure to put in a revision
    /// prompt.
    #[test]
    fn a_plan_that_never_ran_has_no_last_run_failure() {
        let tmp = tempfile::tempdir().unwrap();
        let plan_dir = tmp.path().join("plans").join("my-plan");
        std::fs::create_dir_all(&plan_dir).unwrap();
        std::fs::write(plan_dir.join("tasks.toml"), minimal_valid_toml("my-plan")).unwrap();

        assert_eq!(
            last_run_failure_context(tmp.path(), "my-plan", MIN_REVISION_FAILURE_CHARS),
            None
        );
    }

    fn wrap_toml(toml: &str) -> String {
        format!("Here is the revised plan:\n\n```toml\n{toml}\n```\n")
    }

    /// 3216: a revision that removes T3, edits T1's verify command and adds
    /// T4 answers with a diff listing exactly those three changes, in the
    /// server's DTO and in the text a terminal prints.
    #[test]
    fn revise_response_includes_plan_diff() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let workdir = tmp.path().join("workspace");
        let plan_dir = workdir.join("plans").join("my-plan");
        std::fs::create_dir_all(&plan_dir).expect("plan dir");
        let tasks_path = plan_dir.join("tasks.toml");
        let task = |id: &str, command: &str| {
            format!(
                r#"
[[task]]
id = "{id}"
title = "Task {id}"
description = "Detailed description of task {id}."
role = "implementer"
tier = "focused"
files = ["src/{id}.rs"]
depends_on = []

[[task.verify]]
phase = "structural"
command = "{command}"
fail_msg = "must pass"
"#
            )
        };
        let meta = "[meta]\nplan = \"my-plan\"\ntotal = 3\nmax_parallel = 1\n";
        let before = [
            meta.to_string(),
            task("T1", "test -f src/T1.rs"),
            task("T2", "test -f src/T2.rs"),
            task("T3", "test -f src/T3.rs"),
        ]
        .concat();
        std::fs::write(&tasks_path, &before).expect("write the plan");
        let after = [
            meta.to_string(),
            task("T1", "grep -q retries src/T1.rs"),
            task("T2", "test -f src/T2.rs"),
            task("T4", "test -f src/T4.rs"),
        ]
        .concat();

        let outcome = apply_revision_output(
            &workdir,
            "my-plan",
            &tasks_path,
            &wrap_toml(&after),
            &empty_models(),
        )
        .expect("apply the revision");
        assert!(outcome.written, "{:?}", outcome.report.diagnostics);
        let dto = crate::serve_runtime::revision_to_dto(outcome.clone());
        assert!(dto.revised);
        let diff = dto.diff.expect("a written revision has a diff");
        assert_eq!(diff.added, ["T4"]);
        assert_eq!(diff.removed, ["T3"]);
        assert!(diff.meta.is_empty(), "{diff:?}");
        assert_eq!(diff.changed.len(), 1, "{diff:?}");
        assert_eq!(diff.changed[0].id, "T1");
        assert_eq!(diff.changed[0].keys.len(), 1, "{diff:?}");
        let change = &diff.changed[0].keys[0];
        assert_eq!(change.key, "verify");
        let before_text = change.before.as_deref().unwrap_or_default();
        let after_text = change.after.as_deref().unwrap_or_default();
        assert!(before_text.contains("test -f src/T1.rs"), "{change:?}");
        assert!(
            after_text.contains("grep -q retries src/T1.rs"),
            "{change:?}"
        );

        let text = outcome
            .diff
            .expect("the outcome has the diff")
            .render_text();
        assert!(
            text.starts_with("plan diff: 1 added, 1 removed, 1 changed\n"),
            "{text}"
        );
        for line in ["\n  + T4\n", "\n  - T3\n", "\n  ~ T1\n", "\n      verify: "] {
            assert!(text.contains(line), "{line:?} in {text}");
        }
        assert_eq!(plan_diff(&after, &after), PlanDiff::default());
        assert_eq!(
            plan_diff(&after, &after).render_text(),
            "plan diff: no changes"
        );
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

        let outcome = apply_revision_output(
            &workdir,
            "my-plan",
            &tasks_path,
            &agent_output,
            &empty_models(),
        )
        .unwrap();

        assert!(
            outcome.written,
            "expected written=true; report: {:?}",
            outcome.report.diagnostics
        );
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

        let outcome = apply_revision_output(
            &workdir,
            "my-plan",
            &tasks_path,
            &agent_output,
            &empty_models(),
        )
        .unwrap();

        assert!(
            !outcome.written,
            "expected written=false due to dangling dep"
        );
        assert!(
            outcome
                .report
                .diagnostics
                .iter()
                .any(|d| d.message.contains("T99")),
            "expected T99 in diagnostics; got: {:?}",
            outcome.report.diagnostics
        );

        // File must be unchanged.
        let content = std::fs::read_to_string(&tasks_path).unwrap();
        assert_eq!(
            content, original,
            "file must remain untouched on validation failure"
        );
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

        let result = apply_revision_output(
            &workdir,
            "my-plan",
            &tasks_path,
            agent_output,
            &empty_models(),
        );

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

        let result = apply_revision_output(
            &workdir,
            "my-plan",
            &tasks_path,
            &agent_output,
            &empty_models(),
        );

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

    fn read_jsonl(path: &Path) -> Vec<serde_json::Value> {
        std::fs::read_to_string(path)
            .unwrap_or_else(|error| panic!("read {}: {error}", path.display()))
            .lines()
            .map(|line| serde_json::from_str(line).expect("JSONL row"))
            .collect()
    }

    /// Each agent call of a generation lands where a task dispatch's spend
    /// does: a cost record, an efficiency row, and live efficiency events that
    /// the dashboard snapshot adds to its cost total. A failed call counts too.
    #[tokio::test]
    async fn authoring_spend_records_every_call_against_the_plan() {
        use roko_core::dashboard_snapshot::DashboardEvent;

        let tmp = tempfile::tempdir().unwrap();
        let hub = crate::state_hub::SharedStateHub::new_in_process();
        let spend =
            AuthoringSpend::generation(tmp.path(), "demo", Some(TuiBridge::new(hub.sender())));
        let call = |exit_code: i32, cost_usd: f32| AgentCapture {
            exit_code,
            output: String::new(),
            usage: roko_core::Usage {
                input_tokens: 1_200,
                output_tokens: 340,
                cache_read_tokens: 50,
                cache_create_tokens: 10,
                reasoning_tokens: 0,
                cost_usd,
                wall_ms: 0,
            },
            model: "claude-sonnet-4-6".to_string(),
            provider: "claude_cli".to_string(),
            duration_ms: 20_500,
        };

        spend.record(&call(1, 0.25)).await;
        spend.record(&call(0, 0.5)).await;

        let snapshot = hub.current_snapshot();
        assert_eq!(snapshot.stats.cost_usd_total, 0.75);
        assert_eq!(snapshot.stats.total_input_tokens, 2_400);
        assert_eq!(snapshot.stats.total_output_tokens, 680);
        let live_costs: Vec<(String, String, f64)> = hub
            .replay_from(0)
            .into_iter()
            .filter_map(|envelope| match envelope.payload {
                DashboardEvent::EfficiencyEvent {
                    plan_id,
                    task_id,
                    metric,
                    value,
                } if metric == "cost_usd" => Some((plan_id, task_id, value)),
                _ => None,
            })
            .collect();
        assert_eq!(
            live_costs,
            vec![
                ("demo".to_string(), "generate".to_string(), 0.25),
                ("demo".to_string(), "generate".to_string(), 0.5),
            ]
        );

        let learn_dir = tmp.path().join(".roko").join("learn");
        let costs = read_jsonl(&learn_dir.join("costs.jsonl"));
        assert_eq!(costs.len(), 2);
        for (record, (cost, success)) in costs.iter().zip([(0.25, false), (0.5, true)]) {
            assert_eq!(record["plan_id"], "demo");
            assert_eq!(record["task_id"], "generate");
            assert_eq!(record["role"], "strategist");
            assert_eq!(record["model"], "claude-sonnet-4-6");
            assert_eq!(record["provider"], "claude_cli");
            assert_eq!(record["input_tokens"], 1_200);
            assert_eq!(record["output_tokens"], 340);
            assert_eq!(record["cached_tokens"], 50);
            assert_eq!(record["cost_usd"], cost);
            assert_eq!(record["duration_ms"], 20_500);
            assert_eq!(record["success"], success);
        }

        let efficiency = read_jsonl(&learn_dir.join("efficiency.jsonl"));
        assert_eq!(efficiency.len(), 2);
        for (row, (cost, attempt_id)) in efficiency
            .iter()
            .zip([(0.25, "demo/generate/a1"), (0.5, "demo/generate/a2")])
        {
            assert_eq!(row["plan_id"], "demo");
            assert_eq!(row["task_id"], "generate");
            assert_eq!(row["attempt_id"], attempt_id);
            assert_eq!(row["cost_usd"], cost);
            assert_eq!(row["cache_write_tokens"], 10);
        }
    }

    /// Revision spend is attributed to the plan under its own pseudo task, and
    /// without a live hub it still reaches the cost logs.
    #[tokio::test]
    async fn revision_spend_without_a_hub_reaches_the_cost_logs() {
        let tmp = tempfile::tempdir().unwrap();
        let spend = AuthoringSpend::revision(tmp.path(), "demo", None);
        spend
            .record(&AgentCapture {
                exit_code: 0,
                output: String::new(),
                usage: roko_core::Usage {
                    cost_usd: 0.125,
                    ..roko_core::Usage::zero()
                },
                model: "claude-sonnet-4-6".to_string(),
                provider: "claude_cli".to_string(),
                duration_ms: 1,
            })
            .await;

        let learn_dir = tmp.path().join(".roko").join("learn");
        for file in ["costs.jsonl", "efficiency.jsonl"] {
            let rows = read_jsonl(&learn_dir.join(file));
            assert_eq!(rows.len(), 1, "{file}");
            assert_eq!(rows[0]["plan_id"], "demo", "{file}");
            assert_eq!(rows[0]["task_id"], REVISION_SPEND_TASK_ID, "{file}");
            assert_eq!(rows[0]["cost_usd"], 0.125, "{file}");
        }
    }
}
