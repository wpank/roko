//! Local plan data types for the HTTP routes.
//!
//! These are pure data structs mirroring `roko_cli::plan::{Plan, PlanTask}`
//! so that `roko-serve` can work with plans without depending on `roko-cli`.

use serde::{Deserialize, Serialize};

/// Wire-format summary of a plan, returned by the plan-list HTTP route.
///
/// Field names mirror `roko_cli::plan::PlanSummary` exactly so that callers
/// can deserialise either source with the same schema.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanSummaryDto {
    /// Stable plan identifier.
    pub id: String,
    /// Human-readable plan title.
    pub title: String,
    /// Total number of tasks in the plan.
    pub task_count: usize,
    /// Number of completed tasks.
    pub tasks_done: usize,
    /// Number of failed tasks.
    pub tasks_failed: usize,
    /// Whether all tasks have been completed.
    pub completed: bool,
    /// Lifecycle status string (e.g. `"done"`, `"ready"`, `"superseded"`).
    pub status: String,
    /// Replacement plan declared by `[meta].superseded_by`, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub superseded_by: Option<String>,
    /// Whether the plan's `tasks.toml` is missing modern fields.
    pub old_format: bool,
    /// Last error message from executor state, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_error: Option<String>,
    /// Plan set containing this plan, relative to the plans root and
    /// `/`-separated (e.g. `"portal-programme"`); absent for top-level plans.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group: Option<String>,
    /// Estimated wall-clock minutes for the whole plan; absent when not
    /// specified by `[meta].estimated_total_minutes` or per-task estimates.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub estimated_minutes: Option<u32>,
}

/// Wire-format representation of a single verification step within a task.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PlanTaskVerifyDto {
    /// Phase name (e.g. `"structural"`, `"compile"`, `"test"`).
    pub phase: String,
    /// Shell command to run; exit 0 = pass.
    pub command: String,
    /// Message to display on failure.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fail_msg: Option<String>,
    /// Timeout in milliseconds; absent when the step uses the default timeout.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout_ms: Option<u64>,
}

/// Wire-format representation of a single plan task.
///
/// Field names mirror the subset of `roko_cli::task_parser::TaskDef` that the
/// HTTP portal needs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanTaskDto {
    /// Stable task identifier within the plan.
    pub id: String,
    /// Short human-readable task title.
    pub title: String,
    /// Optional longer task description.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Agent role assigned to the task, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    /// Complexity tier (e.g. `"mechanical"`, `"focused"`, `"integrative"`).
    pub tier: String,
    /// Current execution status string.
    pub status: String,
    /// IDs of tasks that must complete before this task can start.
    #[serde(default)]
    pub depends_on: Vec<String>,
    /// Files or paths expected to be touched by the task.
    #[serde(default)]
    pub files: Vec<String>,
    /// Whether the task has been completed.
    pub completed: bool,
    /// Ordered verification phase names (e.g. `["cargo check", "cargo test"]`).
    /// Kept for backward compatibility with older readers; prefer `verify`.
    #[serde(default)]
    pub verify_phases: Vec<String>,
    /// Suggested model for this task.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_hint: Option<String>,
    /// Estimated wall-clock minutes for this task.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub estimated_minutes: Option<u32>,
    /// Ordered verification steps with full detail (phase, command, fail_msg, timeout_ms).
    #[serde(default)]
    pub verify: Vec<PlanTaskVerifyDto>,
}

/// Wire-format envelope returned by the plan-tasks HTTP route.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanTasksDto {
    /// Identifier of the plan these tasks belong to.
    pub plan_id: String,
    /// Total number of tasks in the plan.
    pub task_count: usize,
    /// Ordered list of task details.
    pub tasks: Vec<PlanTaskDto>,
    /// Human-readable plan title from `[meta].plan`, if present.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Maximum number of tasks that can run in parallel for this plan.
    #[serde(default)]
    pub max_parallel: u32,
}

/// Wire-format representation of a plan's raw TOML source.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanSourceDto {
    /// Stable plan identifier.
    pub id: String,
    /// Workspace-relative path to the `tasks.toml` file.
    pub path: String,
    /// Raw TOML source text, byte-identical to the file on disk.
    pub toml: String,
}

/// A single diagnostic produced by plan validation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanDiagnosticDto {
    /// Severity: `"error"` or `"warning"`.
    pub severity: String,
    /// Short rule code, e.g. `"PLAN_PARSE"`, `"PLAN_005"`.
    pub rule_id: String,
    /// Task that produced the issue, when applicable.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub task_id: Option<String>,
    /// Human-readable description.
    pub message: String,
}

/// Result of validating or saving a plan source.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanValidationDto {
    /// `true` when there are no error-level diagnostics.
    pub valid: bool,
    /// Count of error-severity diagnostics.
    pub errors: usize,
    /// Count of warning-severity diagnostics.
    pub warnings: usize,
    /// All diagnostics, errors first.
    #[serde(default)]
    pub diagnostics: Vec<PlanDiagnosticDto>,
}

/// Result of a plan revision request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RevisionDto {
    /// Whether the plan source was actually modified (written to disk).
    pub revised: bool,
    /// Number of tasks in the (potentially revised) plan.
    pub task_count: usize,
    /// Validation report for the revised source.
    pub validation: PlanValidationDto,
}

/// Outcome returned by `create_plan`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum CreatePlanOutcome {
    /// The plan was created successfully.
    Created {
        /// The plan slug (id).
        slug: String,
        /// Workspace-relative path to the plan directory.
        path: String,
    },
    /// A plan with this slug already exists.
    AlreadyExists {
        /// The slug of the existing plan.
        slug: String,
    },
    /// The generated starter source was rejected by the validator.
    Rejected {
        /// Validation report describing why creation was blocked.
        validation: PlanValidationDto,
    },
}

/// A full plan document.
#[derive(Debug, Clone)]
pub struct Plan {
    /// Stable plan identifier.
    pub id: String,
    /// Human-readable plan title.
    pub title: String,
    /// Longer description of the plan goal and scope.
    pub description: String,
    /// Ordered task list belonging to the plan.
    pub tasks: Vec<PlanTask>,
}

impl Plan {
    /// Construct an empty plan with the provided metadata.
    #[must_use]
    pub const fn new(id: String, title: String, description: String) -> Self {
        Self {
            id,
            title,
            description,
            tasks: Vec::new(),
        }
    }

    /// Append one task to the plan's ordered task list.
    pub fn add_task(&mut self, task: PlanTask) {
        self.tasks.push(task);
    }

    /// Validate the plan for common issues.
    ///
    /// # Errors
    ///
    /// Returns a list of validation errors when the plan ID or title is blank,
    /// or when any task has a blank ID.
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();
        if self.id.trim().is_empty() {
            errors.push("plan id must not be empty".into());
        }
        if self.title.trim().is_empty() {
            errors.push("plan title must not be empty".into());
        }
        for task in &self.tasks {
            if task.id.trim().is_empty() {
                errors.push(format!(
                    "task id must not be empty (description: {})",
                    task.description
                ));
            }
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

/// A plan task entry.
#[derive(Debug, Clone)]
pub struct PlanTask {
    /// Stable task identifier within the plan.
    pub id: String,
    /// Human-readable task description.
    pub description: String,
    /// Complexity tier used for cost projection and budget allocation.
    pub tier: String,
    /// Optional preferred model used when inferring an omitted tier.
    pub model_hint: Option<String>,
    /// IDs of tasks that must complete before this task can start.
    pub depends_on: Vec<String>,
    /// Files or paths expected to be touched by the task.
    pub files: Vec<String>,
    /// Whether the task has already been completed.
    pub completed: bool,
}
