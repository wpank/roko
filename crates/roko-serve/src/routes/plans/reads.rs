//! Plan reads: list plans, and show a plan, its tasks, gates, costs and
//! estimate.

use super::*;

/// `GET /api/plans` — list plans by delegating to the runtime's plan discovery.
///
/// The previous implementation walked `.roko/plans/` and filtered by file
/// extension, which silently skipped every plan stored as a directory (the
/// normal layout). Delegating to `state.runtime.list_plans()` fixes that and
/// also surfaces richer status metadata the portal needs to colour plans.
pub(super) async fn list_plans(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Value>, ApiError> {
    let plans = state
        .runtime
        .list_plans(&state.workdir)
        .await
        .map_err(|e| {
            ApiError::internal(format!("list plans in {}: {e}", state.workdir.display()))
        })?;

    let summaries: Vec<Value> = plans
        .into_iter()
        .map(|dto| {
            let mut v = serde_json::to_value(&dto).unwrap_or(Value::Null);
            // Keep `completed_task_count` as an alias of `tasks_done` for older clients.
            if let Some(tasks_done) = v.get("tasks_done").and_then(Value::as_u64) {
                v["completed_task_count"] = tasks_done.into();
            }
            v
        })
        .collect();

    Ok(Json(Value::Array(summaries)))
}

/// `GET /api/plans/:id` — load a specific plan summary.
///
/// Delegates to `state.runtime.load_plan_summary()` so that directory-layout
/// plans (the normal layout) are discovered correctly. The previous
/// implementation called `find_plan`, which only probed flat `.json`/`.toml`
/// files and returned 404 for every directory plan.
pub(super) async fn get_plan(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    validate_path_segment(&id, "plan id")?;

    let dto = state
        .runtime
        .load_plan_summary(&state.workdir, &id)
        .await
        .map_err(|e| ApiError::internal(format!("load plan '{id}': {e}")))?
        .ok_or_else(|| ApiError::not_found(format!("plan '{id}' not found")))?;

    let mut v = serde_json::to_value(&dto)
        .map_err(|e| ApiError::internal(format!("serialize plan summary: {e}")))?;
    // Keep `completed_task_count` as an alias of `tasks_done` for older clients.
    if let Some(tasks_done) = v.get("tasks_done").and_then(Value::as_u64) {
        v["completed_task_count"] = tasks_done.into();
    }
    Ok(Json(v))
}

/// `GET /api/plans/:id/tasks` — return the task list for a specific plan.
///
/// Delegates to `state.runtime.load_plan_tasks()` so that directory-layout
/// plans (the normal layout) are discovered correctly. The previous
/// implementation called `find_plan`, which only probed flat `.json`/`.toml`
/// files and returned 404 for every directory plan.
///
/// Response envelope: `{ plan_id, task_count, tasks: [...] }`.
/// Each task carries `id`, `title`, `description`, `role`, `tier`,
/// `depends_on`, `files`, `completed`, `status`, and `verify_phases`.
pub(super) async fn plan_tasks(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    validate_path_segment(&id, "plan id")?;

    let dto = state
        .runtime
        .load_plan_tasks(&state.workdir, &id)
        .await
        .map_err(|e| ApiError::internal(format!("load tasks for plan '{id}': {e}")))?
        .ok_or_else(|| ApiError::not_found(format!("plan '{id}' not found")))?;

    let tasks: Vec<Value> = dto
        .tasks
        .iter()
        .map(|t| serde_json::to_value(t).unwrap_or(Value::Null))
        .collect();

    Ok(Json(json!({
        "plan_id": dto.plan_id,
        "task_count": dto.task_count,
        "title": dto.title,
        "max_parallel": dto.max_parallel,
        "tasks": tasks,
    })))
}

// ── Verify results query ──────────────────────────────────────────────

/// `GET /api/plans/:id/gates` — query gate results for a specific plan.
///
/// Returns gate verdicts from the materialized dashboard snapshot, filtered
/// to the requested plan.
pub(super) async fn plan_gates(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    validate_path_segment(&id, "plan id")?;

    // Verify the plan exists via the runtime (finds directory-layout plans).
    let _plan = resolve_plan(&state, &id).await?;

    // Pull gate results from the materialized snapshot.
    let snapshot = state.state_hub.current_snapshot();
    let gates: Vec<Value> = snapshot
        .gates
        .iter()
        .filter(|g| g.plan_id == id)
        .map(|g| {
            json!({
                "plan_id": g.plan_id,
                "task_id": g.task_id,
                "gate": g.gate,
                "passed": g.passed,
                "ts_millis": g.ts_millis,
            })
        })
        .collect();

    Ok(Json(json!({
        "plan_id": id,
        "gate_count": gates.len(),
        "passed": gates.iter().filter(|g| g["passed"] == true).count(),
        "failed": gates.iter().filter(|g| g["passed"] == false).count(),
        "gates": gates,
    })))
}

/// `GET /api/plans/{id}/costs` -- cost breakdown from efficiency events.
///
/// Reads `.roko/learn/efficiency.jsonl`, filters by plan_id, and returns
/// per-task cost breakdown, remaining-cost projection, and budget status.
pub(super) async fn plan_costs(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    validate_path_segment(&id, "plan id")?;

    // Verify the plan exists via the runtime (finds directory-layout plans).
    let plan = resolve_plan(&state, &id).await?;

    // Read efficiency events from the learning log.
    let efficiency_path = state
        .workdir
        .join(".roko")
        .join("learn")
        .join("efficiency.jsonl");
    let content = tokio::fs::read_to_string(&efficiency_path)
        .await
        .unwrap_or_default();

    #[derive(serde::Deserialize)]
    struct EffRow {
        #[serde(default)]
        plan_id: String,
        #[serde(default)]
        task_id: String,
        #[serde(default)]
        model: String,
        #[serde(default)]
        cost_usd: f64,
        #[serde(default)]
        input_tokens: u64,
        #[serde(default)]
        output_tokens: u64,
    }

    let mut task_costs: std::collections::HashMap<String, (f64, u64, u64, String)> =
        std::collections::HashMap::new();
    let mut total_cost = 0.0_f64;
    let mut total_input = 0_u64;
    let mut total_output = 0_u64;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let row: EffRow = match serde_json::from_str(trimmed) {
            Ok(r) => r,
            Err(_) => continue,
        };
        if row.plan_id != id {
            continue;
        }
        total_cost += row.cost_usd;
        total_input += row.input_tokens;
        total_output += row.output_tokens;
        let entry = task_costs
            .entry(row.task_id.clone())
            .or_insert((0.0, 0, 0, String::new()));
        entry.0 += row.cost_usd;
        entry.1 += row.input_tokens;
        entry.2 += row.output_tokens;
        if entry.3.is_empty() {
            entry.3 = row.model;
        }
    }

    let mut projector = CostProjector::new();
    for (task_id, (cost, _, _, model)) in &task_costs {
        // Spend recorded under an id that is not one of the plan's tasks (plan
        // generation's `generate`, revision's `revise`) is plan spend, not a
        // completed task.
        let Some(task) = plan.tasks.iter().find(|task| task.id == *task_id) else {
            continue;
        };
        projector.record_completed(&CompletedTask {
            tier: task.tier.clone(),
            model: model.clone(),
            cost_usd: *cost,
        });
    }
    let remaining = plan
        .tasks
        .iter()
        .filter(|task| !task.completed && !task_costs.contains_key(&task.id))
        .map(|task| RemainingTask {
            tier: task.tier.clone(),
            model_hint: task.model_hint.clone().unwrap_or_default(),
        })
        .collect::<Vec<_>>();
    let config = state.load_roko_config();
    let default_model = resolve_model(&config, &config.agent.default_model).slug;
    let projection = projector.project_remaining_cost(&remaining, &default_model);
    // Everything the plan has spent, generation and revision included, plus
    // the expected cost of its remaining tasks.
    let projected_total_usd = total_cost + projection.expected_usd;
    let budget_limit_usd = f64::from(config.budget.max_plan_usd);
    let budget_enabled = budget_limit_usd > 0.0;
    let budget_remaining_usd = budget_enabled.then(|| (budget_limit_usd - total_cost).max(0.0));
    let budget_utilization = budget_enabled.then(|| total_cost / budget_limit_usd);
    let budget_status = if !budget_enabled {
        "unlimited"
    } else if total_cost >= budget_limit_usd {
        "exceeded"
    } else if projected_total_usd > budget_limit_usd {
        "projected_exceeded"
    } else if total_cost / budget_limit_usd >= 0.8 {
        "warning"
    } else {
        "ok"
    };

    let mut tasks: Vec<Value> = plan
        .tasks
        .iter()
        .map(|task| {
            let (cost, input, output, model) =
                task_costs.get(&task.id).cloned().unwrap_or_default();
            let task_budget = config
                .budget
                .task_limit_usd(&task.tier, task.model_hint.as_deref());
            json!({
                "id": task.id,
                "task_id": task.id,
                "tier": task.tier,
                "spent": cost,
                "budget": (task_budget > 0.0).then_some(task_budget),
                "cost_usd": cost,
                "input_tokens": input,
                "output_tokens": output,
                "model": model,
                "budget_exhausted": task_budget > 0.0 && cost >= task_budget,
            })
        })
        .collect();
    tasks.sort_by(|a, b| a["task_id"].as_str().cmp(&b["task_id"].as_str()));

    let mut provider_health = state
        .provider_health_registry
        .snapshot()
        .into_values()
        .map(|health| {
            json!({
                "id": health.provider_id,
                "state": health.state,
                "cooldown_until_ms": health.cooldown_until,
            })
        })
        .collect::<Vec<_>>();
    provider_health.sort_by(|left, right| left["id"].as_str().cmp(&right["id"].as_str()));

    Ok(Json(json!({
        "plan_id": id,
        "plan_spent": total_cost,
        "plan_budget": budget_enabled.then_some(budget_limit_usd),
        "task_costs": tasks,
        "provider_health": provider_health,
        "total_cost_usd": total_cost,
        "total_input_tokens": total_input,
        "total_output_tokens": total_output,
        "task_count": tasks.len(),
        "tasks": tasks,
        "projection": {
            "optimistic_remaining_usd": projection.optimistic_usd,
            "expected_remaining_usd": projection.expected_usd,
            "pessimistic_remaining_usd": projection.pessimistic_usd,
            "projected_total_usd": projected_total_usd,
            "tasks_completed": projection.tasks_completed,
            "tasks_remaining": projection.tasks_remaining,
            "confidence": projection.confidence,
        },
        "budget": {
            "enabled": budget_enabled,
            "limit_usd": budget_enabled.then_some(budget_limit_usd),
            "remaining_usd": budget_remaining_usd,
            "utilization": budget_utilization,
            "status": budget_status,
            "projected_exceeded": budget_enabled && projected_total_usd > budget_limit_usd,
        },
    })))
}

// ── Cost estimation ─────────────────────────────────────────────────

/// `POST /api/plans/:id/estimate` — estimate cost and time for plan execution.
///
/// Reads historical efficiency events to build per-task estimates based on
/// past performance for similar roles and models.
pub(super) async fn plan_estimate(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let plan = resolve_plan(&state, &id).await?;

    // Load historical efficiency data.
    let efficiency_path = state
        .workdir
        .join(".roko")
        .join("learn")
        .join("efficiency.jsonl");
    let historical = load_efficiency_history(&efficiency_path).await;

    // Compute per-task estimates.
    let mut task_estimates = Vec::new();
    let mut total_input_tokens: u64 = 0;
    let mut total_output_tokens: u64 = 0;
    let mut total_cost_usd: f64 = 0.0;
    let mut total_duration_secs: f64 = 0.0;

    for task in &plan.tasks {
        if task.completed {
            continue;
        }
        // Find similar historical tasks (by matching plan role or averaging all).
        let (est_input, est_output, est_cost, est_duration) =
            estimate_task_from_history(&historical);

        total_input_tokens += est_input;
        total_output_tokens += est_output;
        total_cost_usd += est_cost;
        total_duration_secs += est_duration;

        task_estimates.push(json!({
            "task_id": task.id,
            "description": task.description,
            "estimated_input_tokens": est_input,
            "estimated_output_tokens": est_output,
            "estimated_cost_usd": format!("{:.4}", est_cost),
            "estimated_duration_secs": format!("{:.0}", est_duration),
        }));
    }

    let completed = plan.tasks.iter().filter(|t| t.completed).count();

    Ok(Json(json!({
        "plan_id": id,
        "total_tasks": plan.tasks.len(),
        "completed_tasks": completed,
        "remaining_tasks": plan.tasks.len() - completed,
        "estimate": {
            "total_input_tokens": total_input_tokens,
            "total_output_tokens": total_output_tokens,
            "total_cost_usd": format!("{:.4}", total_cost_usd),
            "total_duration_secs": format!("{:.0}", total_duration_secs),
        },
        "per_task": task_estimates,
        "confidence": if historical.is_empty() { "low" } else { "medium" },
        "note": if historical.is_empty() {
            "No historical data available; using default estimates"
        } else {
            "Based on historical efficiency events"
        },
    })))
}

/// A simplified efficiency record parsed from the JSONL log.
pub(super) struct HistoricalEfficiency {
    pub(super) input_tokens: u64,
    pub(super) output_tokens: u64,
    pub(super) cost_usd: f64,
    pub(super) duration_secs: f64,
}

/// Load efficiency history from the JSONL log file.
pub(super) async fn load_efficiency_history(path: &std::path::Path) -> Vec<HistoricalEfficiency> {
    let content = match tokio::fs::read_to_string(path).await {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };

    content
        .lines()
        .filter_map(|line| {
            let v: Value = serde_json::from_str(line).ok()?;
            Some(HistoricalEfficiency {
                input_tokens: v.get("input_tokens")?.as_u64()?,
                output_tokens: v.get("output_tokens")?.as_u64()?,
                cost_usd: v.get("cost_usd").and_then(|c| c.as_f64()).unwrap_or(0.0),
                duration_secs: v
                    .get("wall_clock_ms")
                    .and_then(|d| d.as_f64())
                    .unwrap_or(30_000.0)
                    / 1000.0,
            })
        })
        .collect()
}

/// Estimate a single task from historical averages, with fallback defaults.
pub(super) fn estimate_task_from_history(history: &[HistoricalEfficiency]) -> (u64, u64, f64, f64) {
    if history.is_empty() {
        // Default estimates for a single agent task.
        return (8_000, 4_000, 0.05, 60.0);
    }

    let n = history.len() as f64;
    let avg_input = (history.iter().map(|h| h.input_tokens).sum::<u64>() as f64 / n) as u64;
    let avg_output = (history.iter().map(|h| h.output_tokens).sum::<u64>() as f64 / n) as u64;
    let avg_cost = history.iter().map(|h| h.cost_usd).sum::<f64>() / n;
    let avg_duration = history.iter().map(|h| h.duration_secs).sum::<f64>() / n;

    (avg_input, avg_output, avg_cost, avg_duration)
}
