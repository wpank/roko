//! Plan CRUD, execution, generation, control, and estimation endpoints.
//!
//! `routes()` registers every plan route. The handlers live in four modules:
//! run control (`run_control`), authoring (`authoring`), reviews and diffs
//! (`merge`), and reads (`reads`). Helpers several of them share stay here.

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::{Value, json};
use validator::Validate;

use crate::error::{ApiError, validate_path_segment};
use crate::events::ServerEvent;
use crate::extract::{RequestPayload, ValidJson, validate_with_validator};
use crate::plan_types::{CreatePlanOutcome, Plan, PlanSourceDto, PlanTask, PlanValidationDto};
use crate::runtime::{PlanExecutionResult, PlanRunOptions, RunResult};
use crate::state::{AppState, OperationHandle, OperationStatus, PlanHandle};
use roko_core::agent::resolve_model;
use roko_learn::cost_projection::{CompletedTask, CostProjector, RemainingTask};
use roko_runtime::cancel::CancelToken;

mod authoring;
mod merge;
mod reads;
mod run_control;

use self::authoring::*;
use self::merge::*;
use self::reads::*;
use self::run_control::*;

// The MCP run tools (9115) start, generate and cancel what these routes do.
pub(super) use self::authoring::start_plan_generation;
pub(super) use self::run_control::{cancel_plan_run, start_plan_run_with};

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/plans", get(list_plans).post(create_plan))
        .route("/plans/{id}", get(get_plan))
        .route("/plans/{id}/tasks", get(plan_tasks))
        .route("/plans/{id}/execute", post(execute_plan))
        .route("/plans/{id}/status", get(plan_status))
        .route("/plans/{id}/pause", post(pause_plan))
        .route("/plans/{id}/resume", post(resume_plan))
        .route("/plans/{id}/cancel", post(cancel_plan))
        .route("/plans/{id}/gates", get(plan_gates))
        .route("/plans/{id}/costs", get(plan_costs))
        .route("/plans/{id}/reviews", get(list_reviews))
        .route("/plans/{id}/tasks/{task_id}/review", post(submit_review))
        .route("/plans/{id}/tasks/{task_id}/diff", get(task_diff))
        .route("/plans/{id}/chat", post(plan_chat))
        .route("/plans/{id}/estimate", post(plan_estimate))
        .route(
            "/plans/{id}/source",
            get(get_plan_source).put(put_plan_source),
        )
        .route("/plans/{id}/validate", post(validate_plan))
        .route("/plans/{id}/revise", post(revise_plan))
        .route("/plans/generate", post(generate_plan))
        .route("/plans/execute", post(execute_plans))
}

// ── helpers ──────────────────────────────────────────────────────────

/// Derive a status string from a task's completion state.
fn task_status(task: &PlanTask) -> &'static str {
    if task.completed {
        "completed"
    } else {
        "pending"
    }
}

/// Serialize a `Plan` into a `serde_json::Value`.
fn plan_to_json(plan: &Plan) -> Value {
    json!({
        "id": plan.id,
        "title": plan.title,
        "description": plan.description,
        "tasks": plan.tasks.iter().map(|t| json!({
            "id": t.id,
            "description": t.description,
            "tier": t.tier,
            "model_hint": t.model_hint,
            "depends_on": t.depends_on,
            "files": t.files,
            "completed": t.completed,
            "status": task_status(t),
        })).collect::<Vec<_>>(),
    })
}

/// Resolve the plans directory for the given workspace root: `plans/`, unless
/// the workspace keeps its plans in the legacy `.roko/plans/` (see
/// [`roko_fs::workspace_plans::workspace_plans_dir`]).
///
/// roko-cli's `plan::plans_dir` calls the same resolver, so the runtime writes
/// new plans (`create_plan`, `generate_plan_from_prompt`) where `list_plans` /
/// `get_plan` read them and where these handlers run them.
///
/// Note: this function only *probes* the filesystem — it never creates the
/// directory. In a new workspace `plans/` appears with the first plan.
fn plans_dir(workdir: &std::path::Path) -> std::path::PathBuf {
    roko_fs::workspace_plans::workspace_plans_dir(workdir)
}

/// Load a plan by id using the runtime's plan-discovery methods.
///
/// Returns 404 when the plan is absent, just like the legacy `find_plan`
/// helper did — but unlike `find_plan`, this resolves directory-layout
/// plans correctly by delegating to the runtime rather than probing for
/// flat `.json`/`.toml` files.
async fn resolve_plan(state: &Arc<AppState>, id: &str) -> Result<Plan, ApiError> {
    validate_path_segment(id, "plan id")?;

    let summary = state
        .runtime
        .load_plan_summary(&state.workdir, id)
        .await
        .map_err(|e| ApiError::internal(format!("load plan '{id}': {e}")))?
        .ok_or_else(|| ApiError::not_found(format!("plan '{id}' not found")))?;

    let tasks_dto = state
        .runtime
        .load_plan_tasks(&state.workdir, id)
        .await
        .map_err(|e| ApiError::internal(format!("load tasks for plan '{id}': {e}")))?
        .ok_or_else(|| ApiError::not_found(format!("plan '{id}' not found")))?;

    let mut plan = Plan::new(summary.id, summary.title, String::new());
    for t in tasks_dto.tasks {
        plan.add_task(PlanTask {
            id: t.id,
            description: t.description.unwrap_or_else(|| t.title.clone()),
            tier: t.tier,
            model_hint: t.model_hint,
            depends_on: t.depends_on,
            files: t.files,
            completed: t.completed,
        });
    }
    Ok(plan)
}

#[cfg(test)]
mod tests;
