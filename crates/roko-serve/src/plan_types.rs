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
    #[serde(default)]
    pub verify_phases: Vec<String>,
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
