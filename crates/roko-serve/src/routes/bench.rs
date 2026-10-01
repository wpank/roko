//! Bench run endpoints.
//!
//! Provides routes for starting, tracking, comparing, and analyzing
//! benchmark runs that exercise roko's `run_once()` pipeline.

use std::convert::Infallible;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::Context;
use axum::extract::{Path, Query, State};
use axum::response::IntoResponse;
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::routing::{delete, get, post};
use axum::{Json, Router};
use futures::stream::{self};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::bench::{
    self, BenchConfigOverrides, BenchRun, BenchRunIndexEntry, BenchRunKind, BenchRunStatus,
    BenchRunSummary, BenchStrategy, BenchSuite, BenchTaskResult, MatrixLaneConfig, MatrixRun,
    MatrixRunStatus,
};
use crate::error::ApiError;
use crate::events::ServerEvent;
use crate::state::{AppState, BenchRunHandle, MatrixRunHandle};
use roko_agent::CostTable;
use roko_core::Usage as CoreUsage;
use roko_core::metric::{ConfigHash, TaskMetric};
use roko_core::{Body, Kind, Signal, Verify};
use roko_gate::{GatePayload, ShellGate};
use roko_learn::baseline::compute_baseline;
use roko_learn::playbook::PlaybookStore;
use roko_learn::regression::{RegressionReport, RegressionThresholds, detect_regressions};
use roko_neuro::KnowledgeStore;

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/bench/provider-status", get(provider_status))
        .route("/bench/run", post(start_bench_run))
        .route("/bench/runs", post(start_bench_run))
        .route("/bench/run/{id}", get(get_bench_run))
        .route("/bench/runs/{id}", get(get_bench_run))
        .route("/bench/run/{id}/status", get(bench_run_status))
        .route("/bench/run/{id}", delete(delete_bench_run))
        .route("/bench/runs/{id}", delete(delete_bench_run))
        .route("/bench/runs/{id}/cancel", post(cancel_bench_run))
        .route("/bench/runs", get(list_bench_runs))
        .route("/bench/runs/compare", get(compare_bench_runs))
        .route("/bench/cost-summary", get(cost_summary))
        .route("/bench/suites", get(list_suites))
        .route("/bench/suites/{id}", get(get_suite))
        .route("/bench/suites", post(upload_suite))
        .route("/bench/models", get(list_models))
        .route("/bench/pareto", get(pareto_frontier))
        .route("/bench/matrix", post(start_matrix_run))
        .route("/bench/export/{id}", get(export_bench_run))
        .route("/bench/events", get(bench_events_sse))
}

/// `GET /api/bench/provider-status` -- check whether LLM providers are configured.
async fn provider_status(State(state): State<Arc<AppState>>) -> Json<Value> {
    let config = state.load_roko_config();
    let providers = config.effective_providers();
    let has_providers = !providers.is_empty();
    let has_api_keys = providers.values().any(|p| config.is_provider_available(p));
    Json(json!({
        "has_providers": has_providers,
        "has_api_keys": has_api_keys,
        "demo_available": true,
    }))
}

// ---------------------------------------------------------------------------
// Request / query types
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct StartBenchRequest {
    suite_id: String,
    #[serde(default)]
    label: Option<String>,
    #[serde(default)]
    overrides: BenchConfigOverrides,
}

#[derive(Deserialize)]
struct ListRunsQuery {
    #[serde(default)]
    suite_id: Option<String>,
    #[serde(default)]
    status: Option<String>,
    #[serde(default = "default_limit")]
    limit: usize,
    #[serde(default)]
    offset: usize,
}

fn default_limit() -> usize {
    50
}

#[derive(Deserialize)]
struct CompareQuery {
    ids: String,
}

/// A single lane in a matrix run request from the frontend.
#[derive(Deserialize)]
struct MatrixLaneRequest {
    model: String,
    #[serde(default)]
    backend: Option<String>,
    #[serde(default)]
    strategy: BenchStrategy,
    #[serde(default)]
    label: Option<String>,
    #[serde(default)]
    overrides: BenchConfigOverrides,
}

/// `POST /api/bench/matrix` request body.
#[derive(Deserialize)]
struct StartMatrixRequest {
    suite_id: String,
    lanes: Vec<MatrixLaneRequest>,
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

/// `POST /api/bench/run` -- start a new bench run.
async fn start_bench_run(
    State(state): State<Arc<AppState>>,
    Json(body): Json<StartBenchRequest>,
) -> Result<impl IntoResponse, ApiError> {
    // Ensure built-in suites exist.
    bench::ensure_builtin_suites(&state.workdir).await;

    let suite = bench::load_suite(&state.workdir, &body.suite_id)
        .await
        .ok_or_else(|| ApiError::not_found("suite not found"))?;

    let run_id = uuid::Uuid::new_v4().to_string();
    let started_at = now_secs();

    let run = BenchRun {
        id: run_id.clone(),
        suite_id: suite.id.clone(),
        suite_name: suite.name.clone(),
        kind: BenchRunKind::Manual,
        overrides: body.overrides.clone(),
        label: body.label.clone(),
        simulated: body.overrides.strategy.is_simulated(),
        status: BenchRunStatus::Running,
        started_at,
        finished_at: None,
        results: Vec::new(),
        summary: None,
        current_task_index: 0,
        total_tasks: suite.tasks.len(),
    };

    // Save initial state.
    if let Err(e) = bench::save_bench_run(&state.workdir, &run).await {
        tracing::warn!(error = %e, "failed to save initial bench run");
    }

    // Add index entry. The index feeds the run list, the pareto frontier and
    // the cost summary, so a simulated (Demo) run never enters it; its run
    // file, marked `simulated`, is still readable by id.
    if !run.simulated {
        let index_entry = BenchRunIndexEntry {
            id: run_id.clone(),
            suite_id: suite.id.clone(),
            suite_name: suite.name.clone(),
            status: BenchRunStatus::Running,
            started_at,
            finished_at: None,
            label: body.label.clone(),
            model: body.overrides.model.clone(),
            pass_rate: None,
            total_cost_usd: None,
        };
        if let Err(err) = bench::append_index_entry(&state.workdir, &index_entry).await {
            tracing::warn!(error = %err, "failed to append bench run index entry");
        }
    }

    // Publish start event.
    state.event_bus.publish(ServerEvent::BenchRunStarted {
        bench_id: run_id.clone(),
        suite_id: suite.id.clone(),
        total_tasks: suite.tasks.len(),
    });

    // Spawn background execution.
    let handle = tokio::spawn(execute_bench_run(
        Arc::clone(&state),
        run_id.clone(),
        suite,
        body.overrides,
        body.label,
        started_at,
    ));

    state.active_bench_runs.write().await.insert(
        run_id.clone(),
        BenchRunHandle {
            id: run_id.clone(),
            handle,
        },
    );

    Ok((
        axum::http::StatusCode::ACCEPTED,
        Json(json!({ "id": run_id })),
    ))
}

/// Background task that executes all tasks in a bench suite.
async fn execute_bench_run(
    state: Arc<AppState>,
    run_id: String,
    suite: BenchSuite,
    overrides: BenchConfigOverrides,
    label: Option<String>,
    started_at: u64,
) {
    let total_tasks = suite.tasks.len();
    let mut results = Vec::new();
    let mut _passed_count = 0usize;
    let mut _failed_count = 0usize;

    // Build a CostTable from the live config for accurate cost estimation.
    let cost_table = CostTable::from_config_with_defaults(&state.roko_config.load().models);
    // A Demo run's tokens and cost are simulated: it stays out of the index
    // (update_index_entry would add it) and out of regression checks.
    let simulated = overrides.strategy.is_simulated();

    let bench_workdir = match scaffold_bench_workdir(&suite, &run_id).await {
        Ok(path) => path,
        Err(err) => {
            tracing::warn!(
                error = %err,
                run_id = %run_id,
                suite_id = %suite.id,
                "failed to scaffold bench workdir"
            );

            let finished_at = now_secs();
            if let Ok(Some(mut run)) = bench::load_bench_run(&state.workdir, &run_id).await {
                run.status = BenchRunStatus::Failed;
                run.finished_at = Some(finished_at);
                if let Err(err) = bench::save_bench_run(&state.workdir, &run).await {
                    tracing::warn!(error = %err, run_id = %run_id, "failed to save failed bench run state");
                }
            }

            if !simulated {
                let failed_index_entry = BenchRunIndexEntry {
                    id: run_id.clone(),
                    suite_id: suite.id.clone(),
                    suite_name: suite.name.clone(),
                    status: BenchRunStatus::Failed,
                    started_at,
                    finished_at: Some(finished_at),
                    label: label.clone(),
                    model: overrides.model.clone(),
                    pass_rate: None,
                    total_cost_usd: None,
                };
                if let Err(err) =
                    bench::update_index_entry(&state.workdir, &failed_index_entry).await
                {
                    tracing::warn!(error = %err, run_id = %run_id, "failed to update index for failed bench run");
                }
            }

            state.active_bench_runs.write().await.remove(&run_id);
            return;
        }
    };
    let _bench_workdir_cleanup = BenchWorkdirCleanup::new(bench_workdir.clone());

    let learning_stores = if matches!(overrides.strategy, BenchStrategy::Minimal) {
        None
    } else {
        Some((
            PlaybookStore::new(state.workdir.join(".roko").join("learn").join("playbooks")),
            KnowledgeStore::for_workdir(&state.workdir),
        ))
    };
    let mut learning_totals =
        if let Some((playbook_store, knowledge_store)) = learning_stores.as_ref() {
            current_learning_totals(playbook_store, knowledge_store).await
        } else {
            None
        };

    for (idx, task) in suite.tasks.iter().enumerate() {
        // Publish task start.
        state.event_bus.publish(ServerEvent::BenchTaskStarted {
            bench_id: run_id.clone(),
            task_id: task.id.clone(),
            task_name: task.name.clone(),
            task_index: idx,
            total_tasks,
        });

        // Each task starts from its own copy of the scaffold. Its check runs
        // first on an untouched copy: a check that already passes there
        // cannot tell whether the agent did the task.
        let dirs = TaskDirs::new(&bench_workdir, idx, &task.id);
        let check = task_check(&suite.id, &task.id);
        let baseline = run_baseline_check(check.as_ref(), &dirs.baseline).await;

        let start = std::time::Instant::now();
        let result = state
            .runtime
            .run_once_with_config(dirs.workspace.as_path(), &task.prompt, &overrides)
            .await;
        let duration_ms = start.elapsed().as_millis() as u64;

        // The status comes from the executed check alone, never from the
        // agent's report of success or from text in its output.
        let grade = grade_task(check.as_ref(), baseline, &dirs.workspace).await;
        // Intentionally ignoring: best-effort removal of the graded workspace
        let _ = tokio::fs::remove_dir_all(&dirs.root).await;

        let task_result = match result {
            Ok(run_result) => {
                let (input_tokens, output_tokens) = run_result
                    .usage
                    .as_ref()
                    .map(|u| (u.input_tokens, u.output_tokens))
                    .unwrap_or((0, 0));

                let cost_usd = cost_table.calculate(
                    overrides.model.as_deref().unwrap_or(""),
                    &CoreUsage {
                        input_tokens: input_tokens as u32,
                        output_tokens: output_tokens as u32,
                        ..CoreUsage::default()
                    },
                );
                let output_preview = run_result
                    .output_text
                    .as_ref()
                    .map(|t| t.chars().take(500).collect());

                let mut gate_verdicts: Vec<serde_json::Value> = run_result
                    .gate_results
                    .iter()
                    .map(|gr| {
                        serde_json::json!({
                            "gate": gr.gate,
                            "passed": gr.passed,
                            "detail": gr.detail,
                        })
                    })
                    .collect();
                gate_verdicts.extend(grade.verdict());

                BenchTaskResult {
                    task_id: task.id.clone(),
                    task_name: task.name.clone(),
                    status: grade.status().into(),
                    duration_ms,
                    model: overrides.model.clone().unwrap_or_default(),
                    tokens_in: input_tokens,
                    tokens_out: output_tokens,
                    cost_usd,
                    gate_verdicts,
                    retries_used: 0,
                    output_preview,
                    error: None,
                    skip_reason: grade.skip_reason(),
                }
            }
            Err(e) => BenchTaskResult {
                task_id: task.id.clone(),
                task_name: task.name.clone(),
                status: grade.status().into(),
                duration_ms,
                model: overrides.model.clone().unwrap_or_default(),
                tokens_in: 0,
                tokens_out: 0,
                cost_usd: 0.0,
                gate_verdicts: grade.verdict().into_iter().collect(),
                retries_used: 0,
                output_preview: None,
                error: Some(format!("{e}")),
                skip_reason: grade.skip_reason(),
            },
        };

        if task_result.passed() {
            _passed_count += 1;
        } else {
            _failed_count += 1;
        }

        // Emit per-gate verdicts so the live UI can show gate pass/fail.
        for gv in &task_result.gate_verdicts {
            let gate_name = gv
                .get("gate")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let gate_passed = gv.get("passed").and_then(|v| v.as_bool()).unwrap_or(false);
            let gate_detail = gv.get("detail").and_then(|v| v.as_str()).map(String::from);
            state.event_bus.publish(ServerEvent::BenchGateVerdict {
                bench_id: run_id.clone(),
                task_id: task_result.task_id.clone(),
                gate: gate_name,
                passed: gate_passed,
                message: gate_detail,
                duration_ms: task_result.duration_ms,
            });
        }

        // Emit token velocity for throughput sparklines.
        let total_tokens = task_result.tokens_in + task_result.tokens_out;
        if task_result.duration_ms > 0 && total_tokens > 0 {
            let tps = (total_tokens as f64) / (task_result.duration_ms as f64 / 1000.0);
            state.event_bus.publish(ServerEvent::BenchTokenVelocity {
                bench_id: run_id.clone(),
                task_id: task_result.task_id.clone(),
                tokens_per_second: tps,
                tokens_in: task_result.tokens_in,
                tokens_out: task_result.tokens_out,
                duration_ms: task_result.duration_ms,
            });
        }

        // Publish task completion.
        state.event_bus.publish(ServerEvent::BenchTaskCompleted {
            bench_id: run_id.clone(),
            task_id: task_result.task_id.clone(),
            result: serde_json::to_value(&task_result).unwrap_or_default(),
        });

        let cost_so_far: f64 = results
            .iter()
            .map(|r: &BenchTaskResult| r.cost_usd)
            .sum::<f64>()
            + task_result.cost_usd;

        results.push(task_result);

        // Publish progress.
        state.event_bus.publish(ServerEvent::BenchProgress {
            bench_id: run_id.clone(),
            completed: results.len(),
            total: total_tasks,
            cost_so_far,
        });

        if let Some((playbook_store, knowledge_store)) = learning_stores.as_ref() {
            if let Some(current_totals) =
                current_learning_totals(playbook_store, knowledge_store).await
            {
                let previous_totals = learning_totals.replace(current_totals);
                let playbooks_created = previous_totals
                    .map(|previous| {
                        current_totals
                            .playbooks_total
                            .saturating_sub(previous.playbooks_total)
                    })
                    .unwrap_or(0);
                let anti_patterns_created = previous_totals
                    .map(|previous| {
                        current_totals
                            .anti_patterns_total
                            .saturating_sub(previous.anti_patterns_total)
                    })
                    .unwrap_or(0);

                state.event_bus.publish(ServerEvent::BenchLearningEvent {
                    bench_id: run_id.clone(),
                    task_id: task.id.clone(),
                    playbooks_created,
                    anti_patterns_created,
                    total_playbooks: current_totals.playbooks_total,
                    total_anti_patterns: current_totals.anti_patterns_total,
                });
            }
        }

        // Update on-disk state periodically.
        match bench::load_bench_run(&state.workdir, &run_id).await {
            Ok(Some(mut run)) => {
                run.results = results.clone();
                run.current_task_index = idx + 1;
                if let Err(err) = bench::save_bench_run(&state.workdir, &run).await {
                    tracing::warn!(error = %err, run_id = %run_id, "failed to save periodic bench run state");
                }
            }
            Ok(None) => {
                tracing::warn!(run_id = %run_id, "bench run file missing during periodic save");
            }
            Err(err) => {
                tracing::warn!(error = %err, run_id = %run_id, "failed to load bench run for periodic save");
            }
        }
    }

    // Finalize the run.
    let summary = BenchRunSummary::from_results(&results);
    let finished_at = now_secs();

    match bench::load_bench_run(&state.workdir, &run_id).await {
        Ok(Some(mut run)) => {
            run.status = BenchRunStatus::Completed;
            run.finished_at = Some(finished_at);
            run.results = results.clone();
            run.summary = Some(summary.clone());
            run.current_task_index = total_tasks;
            if let Err(err) = bench::save_bench_run(&state.workdir, &run).await {
                tracing::warn!(error = %err, run_id = %run_id, "failed to save completed bench run");
            }
        }
        Ok(None) => {
            tracing::warn!(run_id = %run_id, "bench run file missing at finalization");
        }
        Err(err) => {
            tracing::warn!(error = %err, run_id = %run_id, "failed to load bench run at finalization");
        }
    }

    // Update index entry.
    if !simulated {
        let index_entry = BenchRunIndexEntry {
            id: run_id.clone(),
            suite_id: suite.id.clone(),
            suite_name: suite.name.clone(),
            status: BenchRunStatus::Completed,
            started_at,
            finished_at: Some(finished_at),
            label,
            model: overrides.model.clone(),
            // A run with no graded task has no pass rate. Leaving it unset
            // keeps the run off the pareto frontier instead of plotting it at 0%.
            pass_rate: (summary.passed + summary.failed > 0).then_some(summary.pass_rate),
            total_cost_usd: Some(summary.total_cost_usd),
        };
        if let Err(err) = bench::update_index_entry(&state.workdir, &index_entry).await {
            tracing::warn!(error = %err, run_id = %run_id, "failed to update bench run index at completion");
        }
    }

    // Publish completion event.
    state.event_bus.publish(ServerEvent::BenchRunCompleted {
        bench_id: run_id.clone(),
        summary: serde_json::to_value(&summary).unwrap_or_default(),
    });

    // ── Regression detection ─────────────────────────────────────────
    //
    // Convert current bench results into TaskMetric records and compare
    // against a baseline computed from prior completed bench runs.
    if !simulated {
        run_bench_regression(&state, &run_id, &suite.id, &results, &overrides).await;
    }

    // Clean up handle.
    state.active_bench_runs.write().await.remove(&run_id);
}

/// `POST /api/bench/matrix` -- start a matrix (model x strategy) bench run.
///
/// Creates one bench run per lane and orchestrates them concurrently.
/// Emits `MatrixRunStarted` immediately and `MatrixLaneCompleted` /
/// `MatrixRunCompleted` as lanes finish.
async fn start_matrix_run(
    State(state): State<Arc<AppState>>,
    Json(body): Json<StartMatrixRequest>,
) -> Result<impl IntoResponse, ApiError> {
    if body.lanes.is_empty() {
        return Err(ApiError::bad_request("matrix must have at least one lane"));
    }

    // Ensure built-in suites exist and load the requested suite.
    bench::ensure_builtin_suites(&state.workdir).await;
    let suite = bench::load_suite(&state.workdir, &body.suite_id)
        .await
        .ok_or_else(|| ApiError::not_found("suite not found"))?;

    let matrix_id = uuid::Uuid::new_v4().to_string();
    let started_at = now_secs();

    // Build lane configs and assign lane IDs.
    let mut lane_configs = Vec::with_capacity(body.lanes.len());
    let mut lane_ids = Vec::with_capacity(body.lanes.len());

    for lane_req in &body.lanes {
        let lane_id = uuid::Uuid::new_v4().to_string();
        lane_ids.push(lane_id.clone());

        let mut overrides = lane_req.overrides.clone();
        // Ensure model/backend/strategy from the top-level lane fields are
        // reflected in the overrides so `execute_bench_run` picks them up.
        if overrides.model.is_none() && !lane_req.model.is_empty() {
            overrides.model = Some(lane_req.model.clone());
        }
        if overrides.backend.is_none() {
            overrides.backend = lane_req.backend.clone();
        }
        overrides.strategy = lane_req.strategy.clone();

        lane_configs.push(MatrixLaneConfig {
            model: lane_req.model.clone(),
            backend: lane_req.backend.clone(),
            strategy: lane_req.strategy.clone(),
            label: lane_req.label.clone(),
            overrides,
        });
    }

    // Persist the matrix run.
    let matrix_run = MatrixRun {
        id: matrix_id.clone(),
        suite_id: suite.id.clone(),
        suite_name: suite.name.clone(),
        lane_ids: lane_ids.clone(),
        lanes: lane_configs.clone(),
        status: MatrixRunStatus::Running,
        started_at,
        finished_at: None,
        label: None,
    };
    if let Err(e) = bench::save_matrix_run(&state.workdir, &matrix_run).await {
        tracing::warn!(error = %e, matrix_id = %matrix_id, "failed to save initial matrix run");
    }

    // Publish MatrixRunStarted event on the event bus.
    state.event_bus.publish(ServerEvent::MatrixRunStarted {
        matrix_id: matrix_id.clone(),
        suite_id: suite.id.clone(),
        lane_ids: lane_ids.clone(),
        total_lanes: lane_ids.len(),
    });

    // Spawn each lane as an independent bench run.
    let mut lane_handles = Vec::with_capacity(lane_ids.len());
    for (idx, lane_id) in lane_ids.iter().enumerate() {
        let lane_config = &lane_configs[idx];
        let run_id = lane_id.clone();
        let lane_started_at = now_secs();

        // Create a per-lane BenchRun.
        let run = BenchRun {
            id: run_id.clone(),
            suite_id: suite.id.clone(),
            suite_name: suite.name.clone(),
            kind: BenchRunKind::Manual,
            overrides: lane_config.overrides.clone(),
            label: lane_config.label.clone(),
            simulated: lane_config.overrides.strategy.is_simulated(),
            status: BenchRunStatus::Running,
            started_at: lane_started_at,
            finished_at: None,
            results: Vec::new(),
            summary: None,
            current_task_index: 0,
            total_tasks: suite.tasks.len(),
        };
        if let Err(e) = bench::save_bench_run(&state.workdir, &run).await {
            tracing::warn!(error = %e, lane_id = %run_id, "failed to save initial lane bench run");
        }

        // Add index entry for this lane, unless its figures are simulated.
        if !run.simulated {
            let index_entry = BenchRunIndexEntry {
                id: run_id.clone(),
                suite_id: suite.id.clone(),
                suite_name: suite.name.clone(),
                status: BenchRunStatus::Running,
                started_at: lane_started_at,
                finished_at: None,
                label: lane_config.label.clone(),
                model: lane_config.overrides.model.clone(),
                pass_rate: None,
                total_cost_usd: None,
            };
            if let Err(err) = bench::append_index_entry(&state.workdir, &index_entry).await {
                tracing::warn!(error = %err, lane_id = %run_id, "failed to append lane index entry");
            }
        }

        // Publish per-lane start event.
        state.event_bus.publish(ServerEvent::BenchRunStarted {
            bench_id: run_id.clone(),
            suite_id: suite.id.clone(),
            total_tasks: suite.tasks.len(),
        });

        // Spawn the lane's bench run.
        let handle = tokio::spawn(execute_bench_run(
            Arc::clone(&state),
            run_id.clone(),
            suite.clone(),
            lane_config.overrides.clone(),
            lane_config.label.clone(),
            lane_started_at,
        ));

        lane_handles.push(handle);
    }

    // Spawn a supervisor task that waits for all lanes to finish, then
    // emits MatrixLaneCompleted per lane and MatrixRunCompleted at the end.
    let monitor_matrix_id = matrix_id.clone();
    let monitor_lane_ids = lane_ids.clone();
    let monitor_state = Arc::clone(&state);

    let monitor_handle = tokio::spawn(async move {
        // Wait for all lane handles to finish.
        for handle in lane_handles {
            let _ = handle.await;
        }

        // Collect per-lane summaries and emit MatrixLaneCompleted events.
        let mut lane_summaries = Vec::new();
        for lane_id in &monitor_lane_ids {
            let (pass_rate, cost_usd) =
                match bench::load_bench_run(&monitor_state.workdir, lane_id).await {
                    Ok(Some(run)) => {
                        let pr = run.summary.as_ref().map(|s| s.pass_rate).unwrap_or(0.0);
                        let cu = run
                            .summary
                            .as_ref()
                            .map(|s| s.total_cost_usd)
                            .unwrap_or(0.0);
                        (pr, cu)
                    }
                    _ => (0.0, 0.0),
                };

            monitor_state
                .event_bus
                .publish(ServerEvent::MatrixLaneCompleted {
                    matrix_id: monitor_matrix_id.clone(),
                    lane_id: lane_id.clone(),
                    pass_rate,
                    cost_usd,
                });

            lane_summaries.push(serde_json::json!({
                "lane_id": lane_id,
                "pass_rate": pass_rate,
                "cost_usd": cost_usd,
            }));
        }

        // Update the matrix run on disk.
        if let Ok(Some(mut matrix_run)) =
            bench::load_matrix_run(&monitor_state.workdir, &monitor_matrix_id).await
        {
            matrix_run.status = MatrixRunStatus::Completed;
            matrix_run.finished_at = Some(now_secs());
            if let Err(e) = bench::save_matrix_run(&monitor_state.workdir, &matrix_run).await {
                tracing::warn!(
                    error = %e,
                    matrix_id = %monitor_matrix_id,
                    "failed to save completed matrix run"
                );
            }
        }

        // Emit MatrixRunCompleted.
        monitor_state
            .event_bus
            .publish(ServerEvent::MatrixRunCompleted {
                matrix_id: monitor_matrix_id.clone(),
                summary: lane_summaries,
            });

        // Clean up the matrix handle.
        monitor_state
            .active_matrix_runs
            .write()
            .await
            .remove(&monitor_matrix_id);
    });

    // Register the matrix supervisor handle.
    state.active_matrix_runs.write().await.insert(
        matrix_id.clone(),
        MatrixRunHandle {
            id: matrix_id.clone(),
            lane_handles: vec![monitor_handle],
        },
    );

    Ok((
        axum::http::StatusCode::ACCEPTED,
        Json(json!({
            "matrix_id": matrix_id,
            "lane_ids": lane_ids,
        })),
    ))
}

/// `GET /api/bench/run/:id` -- get full bench run details.
async fn get_bench_run(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let run = bench::load_bench_run(&state.workdir, &id)
        .await
        .map_err(|e| ApiError::internal(format!("failed to load run: {e}")))?
        .ok_or_else(|| ApiError::not_found("bench run not found"))?;
    Ok(Json(serde_json::to_value(run).unwrap_or_default()))
}

/// `GET /api/bench/run/:id/status` -- lightweight status poll.
async fn bench_run_status(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let run = bench::load_bench_run(&state.workdir, &id)
        .await
        .map_err(|e| ApiError::internal(format!("failed to load run: {e}")))?
        .ok_or_else(|| ApiError::not_found("bench run not found"))?;
    Ok(Json(json!({
        "id": run.id,
        "status": run.status,
        "current_task_index": run.current_task_index,
        "total_tasks": run.total_tasks,
        "passed": run.results.iter().filter(|r| r.passed()).count(),
        "failed": run.results.iter().filter(|r| !r.passed() && !r.skipped()).count(),
        "skipped": run.results.iter().filter(|r| r.skipped()).count(),
        "summary": run.summary,
    })))
}

/// `DELETE /api/bench/run/:id` -- cancel or delete a bench run.
async fn delete_bench_run(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    // Abort if running.
    let handle = state.active_bench_runs.write().await.remove(&id);
    if let Some(handle) = handle {
        handle.handle.abort();
    }
    // Mark as cancelled on disk if still running.
    match bench::load_bench_run(&state.workdir, &id).await {
        Ok(Some(mut run)) => {
            if run.status == BenchRunStatus::Running {
                run.status = BenchRunStatus::Cancelled;
                run.finished_at = Some(now_secs());
                if let Err(err) = bench::save_bench_run(&state.workdir, &run).await {
                    tracing::warn!(error = %err, bench_id = %id, "failed to save cancelled bench run state");
                }
            }
        }
        Ok(None) => {} // Already deleted, nothing to cancel
        Err(err) => {
            tracing::warn!(error = %err, bench_id = %id, "failed to load bench run for cancellation");
        }
    }
    Ok(axum::http::StatusCode::NO_CONTENT)
}

/// `POST /api/bench/runs/:id/cancel` -- cancel a running bench run.
///
/// Equivalent to DELETE but accepts POST (frontend convention).
async fn cancel_bench_run(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    // Abort if running.
    let handle = state.active_bench_runs.write().await.remove(&id);
    if let Some(handle) = handle {
        handle.handle.abort();
    }
    // Mark as cancelled on disk if still running.
    match bench::load_bench_run(&state.workdir, &id).await {
        Ok(Some(mut run)) => {
            if run.status == BenchRunStatus::Running {
                run.status = BenchRunStatus::Cancelled;
                run.finished_at = Some(now_secs());
                if let Err(err) = bench::save_bench_run(&state.workdir, &run).await {
                    tracing::warn!(error = %err, bench_id = %id, "failed to save cancelled bench run state");
                }
            }
        }
        Ok(None) => {}
        Err(err) => {
            tracing::warn!(error = %err, bench_id = %id, "failed to load bench run for cancellation");
        }
    }
    Ok((
        axum::http::StatusCode::OK,
        Json(json!({ "id": id, "status": "cancelled" })),
    ))
}

/// `GET /api/bench/runs` -- list bench runs.
async fn list_bench_runs(
    State(state): State<Arc<AppState>>,
    Query(query): Query<ListRunsQuery>,
) -> Json<Value> {
    let mut entries = bench::load_index_entries(&state.workdir).await;

    // Apply filters.
    if let Some(ref suite_id) = query.suite_id {
        entries.retain(|e| e.suite_id == *suite_id);
    }
    if let Some(ref status) = query.status {
        entries.retain(|e| {
            let s = serde_json::to_value(&e.status)
                .ok()
                .and_then(|v| v.as_str().map(String::from))
                .unwrap_or_default();
            s == *status
        });
    }

    // Reverse chronological.
    entries.sort_by_key(|entry| std::cmp::Reverse(entry.started_at));

    let total = entries.len();
    let page: Vec<_> = entries
        .into_iter()
        .skip(query.offset)
        .take(query.limit)
        .collect();

    Json(json!({
        "total": total,
        "offset": query.offset,
        "limit": query.limit,
        "runs": page,
    }))
}

/// `GET /api/bench/runs/compare?ids=a,b` -- compare multiple runs.
async fn compare_bench_runs(
    State(state): State<Arc<AppState>>,
    Query(query): Query<CompareQuery>,
) -> Result<Json<Value>, ApiError> {
    let ids: Vec<&str> = query.ids.split(',').map(str::trim).collect();
    let mut runs = Vec::new();
    for id in &ids {
        if let Some(run) = bench::load_bench_run(&state.workdir, id)
            .await
            .map_err(|e| ApiError::internal(format!("load error: {e}")))?
        {
            runs.push(run);
        }
    }
    if runs.is_empty() {
        return Err(ApiError::not_found("no runs found"));
    }
    Ok(Json(json!({ "runs": runs })))
}

/// `GET /api/bench/suites` -- list available suites.
async fn list_suites(State(state): State<Arc<AppState>>) -> Json<Value> {
    bench::ensure_builtin_suites(&state.workdir).await;
    let suites = bench::load_suites(&state.workdir).await;
    let listing: Vec<Value> = suites
        .iter()
        .map(|s| {
            json!({
                "id": s.id,
                "name": s.name,
                "description": s.description,
                "task_count": s.tasks.len(),
            })
        })
        .collect();
    Json(json!({ "suites": listing }))
}

/// `GET /api/bench/suites/:id` -- get full suite with tasks.
async fn get_suite(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    bench::ensure_builtin_suites(&state.workdir).await;
    let suite = bench::load_suite(&state.workdir, &id)
        .await
        .ok_or_else(|| ApiError::not_found("suite not found"))?;
    Ok(Json(serde_json::to_value(suite).unwrap_or_default()))
}

/// `POST /api/bench/suites` -- upload a custom suite.
async fn upload_suite(
    State(state): State<Arc<AppState>>,
    Json(suite): Json<BenchSuite>,
) -> Result<impl IntoResponse, ApiError> {
    if suite.id.is_empty() || suite.tasks.is_empty() {
        return Err(ApiError::bad_request(
            "suite must have an id and at least one task",
        ));
    }
    bench::save_suite(&state.workdir, &suite)
        .await
        .map_err(|e| ApiError::internal(format!("failed to save suite: {e}")))?;
    Ok((
        axum::http::StatusCode::CREATED,
        Json(json!({ "id": suite.id })),
    ))
}

/// `GET /api/bench/models` -- list available models from config.
async fn list_models(State(state): State<Arc<AppState>>) -> Json<Value> {
    let config = state.load_roko_config();
    let models = bench::list_models_from_config(&config);
    Json(json!({ "models": models }))
}

/// `GET /api/bench/pareto` -- compute pareto frontier.
async fn pareto_frontier(State(state): State<Arc<AppState>>) -> Json<Value> {
    let frontier = bench::compute_pareto_frontier(&state.workdir).await;
    Json(json!({ "frontier": frontier }))
}

/// `GET /api/bench/export/:id` -- export a bench run as JSON.
async fn export_bench_run(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let run = bench::load_bench_run(&state.workdir, &id)
        .await
        .map_err(|e| ApiError::internal(format!("failed to load run: {e}")))?
        .ok_or_else(|| ApiError::not_found("bench run not found"))?;
    Ok(Json(serde_json::to_value(run).unwrap_or_default()))
}

/// `GET /api/bench/cost-summary` -- aggregate cost by model across all runs.
async fn cost_summary(State(state): State<Arc<AppState>>) -> Json<Value> {
    let entries = bench::load_index_entries(&state.workdir).await;
    let mut model_stats: std::collections::HashMap<String, (f64, u64, u64)> =
        std::collections::HashMap::new();

    for entry in &entries {
        if let Ok(Some(run)) = bench::load_bench_run(&state.workdir, &entry.id).await {
            for result in &run.results {
                let stat = model_stats.entry(result.model.clone()).or_default();
                stat.0 += result.cost_usd;
                stat.1 += result.tokens_in + result.tokens_out;
                stat.2 += 1;
            }
        }
    }

    let models: Vec<Value> = model_stats
        .into_iter()
        .map(|(model, (cost_usd, tokens, tasks))| {
            json!({
                "model": model,
                "cost_usd": cost_usd,
                "tokens": tokens,
                "tasks": tasks,
            })
        })
        .collect();

    Json(json!({ "models": models }))
}

/// `GET /api/bench/events` -- SSE stream filtered to bench events.
async fn bench_events_sse(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let rx = state.event_bus.subscribe();
    let stream = stream::unfold(rx, |mut rx| async move {
        loop {
            match rx.recv().await {
                Ok(envelope) => {
                    // Only emit bench-related events.
                    let is_bench = matches!(
                        &envelope.payload,
                        ServerEvent::BenchRunStarted { .. }
                            | ServerEvent::BenchTaskStarted { .. }
                            | ServerEvent::BenchTaskCompleted { .. }
                            | ServerEvent::BenchLearningEvent { .. }
                            | ServerEvent::BenchProgress { .. }
                            | ServerEvent::BenchRunCompleted { .. }
                            | ServerEvent::BenchGateVerdict { .. }
                            | ServerEvent::BenchTokenVelocity { .. }
                            | ServerEvent::BenchAgentOutput { .. }
                            | ServerEvent::BenchRegressionReport { .. }
                            | ServerEvent::MatrixRunStarted { .. }
                            | ServerEvent::MatrixLaneCompleted { .. }
                            | ServerEvent::MatrixRunCompleted { .. }
                    );
                    if !is_bench {
                        continue;
                    }
                    let data = serde_json::to_string(&envelope.payload).unwrap_or_default();
                    let sse_event = Event::default().data(data).id(envelope.seq.to_string());
                    return Some((Ok::<_, Infallible>(sse_event), rx));
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                    tracing::warn!(n, "bench SSE client lagged");
                    continue;
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => return None,
            }
        }
    });
    let sse = Sse::new(super::sse::until_shutdown(stream, state.cancel.clone())).keep_alive(
        KeepAlive::new()
            .interval(std::time::Duration::from_secs(8))
            .text("keepalive"),
    );
    (super::sse::sse_response_headers(), sse)
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[derive(Debug, Clone, Copy)]
struct LearningTotals {
    playbooks_total: u32,
    anti_patterns_total: u32,
}

async fn current_learning_totals(
    playbook_store: &PlaybookStore,
    knowledge_store: &KnowledgeStore,
) -> Option<LearningTotals> {
    let playbooks_total = match playbook_store.list().await {
        Ok(playbooks) => playbooks.len().min(u32::MAX as usize) as u32,
        Err(err) => {
            tracing::warn!(error = %err, "failed to read bench playbook counts");
            return None;
        }
    };

    let anti_patterns_total = match knowledge_store.stats() {
        Ok(stats) => stats.anti_knowledge_count.min(u32::MAX as usize) as u32,
        Err(err) => {
            tracing::warn!(error = %err, "failed to read bench anti-pattern counts");
            return None;
        }
    };

    Some(LearningTotals {
        playbooks_total,
        anti_patterns_total,
    })
}

/// Check a finished run for regressions against earlier runs of its suite,
/// and publish the report.
async fn run_bench_regression(
    state: &AppState,
    run_id: &str,
    suite_id: &str,
    results: &[BenchTaskResult],
    overrides: &BenchConfigOverrides,
) {
    let Some(report) =
        bench_regression_report(&state.workdir, run_id, suite_id, results, overrides).await
    else {
        return;
    };

    if report.has_regressions {
        tracing::warn!(
            run_id,
            alerts = report.alerts.len(),
            "bench regression detected"
        );
    } else {
        tracing::info!(run_id, "no bench regression detected");
    }

    state.event_bus.publish(ServerEvent::BenchRegressionReport {
        bench_id: run_id.to_string(),
        has_regressions: report.has_regressions,
        report: serde_json::to_value(&report).unwrap_or_default(),
    });
}

/// Compare a finished run's graded results with those of earlier completed
/// runs of the same suite, read from where `bench::save_bench_run` stores
/// them. `None` when either side has too few graded results.
async fn bench_regression_report(
    workdir: &std::path::Path,
    run_id: &str,
    suite_id: &str,
    results: &[BenchTaskResult],
    overrides: &BenchConfigOverrides,
) -> Option<RegressionReport> {
    let thresholds = RegressionThresholds::default();
    let now = chrono::Utc::now().to_rfc3339();
    let model = overrides.model.as_deref().unwrap_or("unknown");

    // Ungraded tasks neither passed nor failed, so they stay out of the
    // comparison.
    let current: Vec<TaskMetric> = results
        .iter()
        .filter(|result| !result.skipped())
        .map(|result| bench_task_metric(run_id, suite_id, model, &now, result))
        .collect();
    if current.len() < thresholds.min_records {
        return None;
    }

    let runs = bench::load_bench_runs(workdir).await;
    let baseline_metrics: Vec<TaskMetric> = runs
        .iter()
        .filter(|run| {
            run.id != run_id
                && run.suite_id == suite_id
                && run.status == BenchRunStatus::Completed
                && !run.simulated
        })
        .flat_map(|run| {
            let model = run.overrides.model.as_deref().unwrap_or("unknown");
            run.results
                .iter()
                .filter(|result| !result.skipped())
                .map(move |result| bench_task_metric(&run.id, suite_id, model, "", result))
        })
        .collect();
    if baseline_metrics.len() < thresholds.min_records {
        tracing::debug!(
            run_id,
            baseline_count = baseline_metrics.len(),
            "not enough baseline data for bench regression detection"
        );
        return None;
    }

    let baseline = compute_baseline(&baseline_metrics, thresholds.min_records);
    Some(detect_regressions(&baseline, &current, &thresholds))
}

/// A graded bench result as a `TaskMetric` for the regression check.
fn bench_task_metric(
    run_id: &str,
    suite_id: &str,
    model: &str,
    timestamp: &str,
    result: &BenchTaskResult,
) -> TaskMetric {
    TaskMetric {
        timestamp: timestamp.to_string(),
        run_id: run_id.to_string(),
        config_hash: ConfigHash(format!("bench-{suite_id}")),
        plan_id: suite_id.to_string(),
        task_id: result.task_id.clone(),
        iteration: 1,
        role: "bench".to_string(),
        backend: "bench".to_string(),
        model: model.to_string(),
        complexity_band: "standard".to_string(),
        gate: "bench".to_string(),
        gate_passed: result.passed(),
        wall_time_ms: result.duration_ms,
        input_tokens: result.tokens_in,
        output_tokens: result.tokens_out,
        cached_tokens: 0,
        cost_usd: result.cost_usd,
        sections_included: 0,
        sections_dropped: 0,
        context_tokens: 0,
        cache_hit_rate: 0.0,
    }
}

struct BenchWorkdirCleanup {
    path: PathBuf,
}

impl BenchWorkdirCleanup {
    fn new(path: PathBuf) -> Self {
        Self { path }
    }
}

impl Drop for BenchWorkdirCleanup {
    fn drop(&mut self) {
        // Intentionally ignoring: best-effort cleanup of temporary bench workdir
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

async fn scaffold_bench_workdir(suite: &BenchSuite, run_id: &str) -> anyhow::Result<PathBuf> {
    let dir = std::env::temp_dir().join(format!("roko-bench-{run_id}"));

    if tokio::fs::try_exists(&dir)
        .await
        .context("check whether bench workdir already exists")?
    {
        tokio::fs::remove_dir_all(&dir)
            .await
            .with_context(|| format!("remove stale bench workdir {}", dir.display()))?;
    }

    if let Err(err) = scaffold_task_dirs(&dir, suite, run_id).await {
        // Intentionally ignoring: best-effort cleanup after scaffold failure
        let _ = tokio::fs::remove_dir_all(&dir).await;
        return Err(err);
    }

    Ok(dir)
}

/// Where one task runs and is graded, under the run's directory.
struct TaskDirs {
    /// Holds the two directories below; removed once the task is graded.
    root: PathBuf,
    /// The agent's workspace, graded after the agent finishes.
    workspace: PathBuf,
    /// An untouched copy of the scaffold that the task's check runs on first.
    baseline: PathBuf,
}

impl TaskDirs {
    fn new(run_dir: &std::path::Path, index: usize, task_id: &str) -> Self {
        let name: String = task_id
            .chars()
            .map(|ch| {
                if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                    ch
                } else {
                    '_'
                }
            })
            .collect();
        let root = run_dir.join(format!("{index:03}-{name}"));
        let workspace = root.join("workspace");
        let baseline = root.join("baseline");
        Self {
            root,
            workspace,
            baseline,
        }
    }
}

/// Give every task of `suite` its own workspace and baseline copy of the
/// scaffold under `run_dir`.
///
/// Tasks never share a workspace, so one task's edits cannot pass or fail
/// another task's check.
async fn scaffold_task_dirs(
    run_dir: &std::path::Path,
    suite: &BenchSuite,
    run_id: &str,
) -> anyhow::Result<()> {
    for (idx, task) in suite.tasks.iter().enumerate() {
        let dirs = TaskDirs::new(run_dir, idx, &task.id);
        scaffold_project(&dirs.workspace, &suite.id, &task.id, run_id).await?;
        scaffold_project(&dirs.baseline, &suite.id, &task.id, run_id).await?;
    }
    Ok(())
}

/// Write the suite's starter project for `task_id` into `dir`.
async fn scaffold_project(
    dir: &std::path::Path,
    suite_id: &str,
    task_id: &str,
    run_id: &str,
) -> anyhow::Result<()> {
    let src = dir.join("src");
    write_scaffold_file(&dir.join("Cargo.toml"), &bench_cargo_toml(suite_id, run_id)).await?;
    write_scaffold_file(&src.join("main.rs"), bench_main_contents()).await?;

    if suite_id == LEARNABLE_SUITE_ID {
        write_scaffold_file(&src.join("helpers.rs"), learnable_helpers_contents()).await?;
        write_scaffold_file(&src.join("lib.rs"), &learnable_rust_lib_for_task(task_id)).await?;
    } else {
        write_scaffold_file(&src.join("lib.rs"), generic_lib_contents()).await?;
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Grading
// ---------------------------------------------------------------------------

/// Suite whose scaffold and hidden checks this file defines.
const LEARNABLE_SUITE_ID: &str = "learnable-rust";

/// Hidden test that grades a learnable-rust task, relative to its workspace.
const LEARNABLE_CHECK_FILE: &str = "tests/roko_bench_check.rs";

/// Runs [`LEARNABLE_CHECK_FILE`] and nothing else, so tests the agent wrote
/// cannot change the grade.
const LEARNABLE_CHECK_COMMAND: &str = "cargo test --quiet --test roko_bench_check";

/// Most characters of check output kept in a task's verdict.
const CHECK_DETAIL_CHARS: usize = 2_000;

/// An executed check that grades one bench task.
#[derive(Debug, Clone, PartialEq, Eq)]
struct TaskCheck {
    /// Files written into the directory (relative path, contents) just
    /// before the command runs, so the agent never sees them.
    hidden_files: Vec<(&'static str, &'static str)>,
    /// Shell command whose exit status is the verdict: 0 passes.
    command: &'static str,
}

/// What one run of a check showed.
#[derive(Debug, Clone, PartialEq, Eq)]
enum CheckRun {
    /// The command exited 0.
    Passed,
    /// The command failed or timed out; holds the end of its output.
    Failed(String),
    /// The check could not be set up, so it measured nothing.
    NotRun(String),
}

/// How a bench task was graded.
#[derive(Debug, Clone, PartialEq, Eq)]
enum TaskGrade {
    /// The check failed on the untouched scaffold and passes on the agent's
    /// workspace.
    Pass,
    /// The check fails on the agent's workspace; holds the end of its output.
    Fail(String),
    /// No executed check measured the task; holds the reason.
    Ungraded(String),
}

impl TaskGrade {
    /// Wire status for `BenchTaskResult::status`.
    fn status(&self) -> &'static str {
        match self {
            Self::Pass => "pass",
            Self::Fail(_) => "fail",
            Self::Ungraded(_) => "skipped",
        }
    }

    /// The check's entry for `BenchTaskResult::gate_verdicts`; `None` when no
    /// check graded the task.
    fn verdict(&self) -> Option<Value> {
        match self {
            Self::Pass => Some(json!({
                "gate": "check",
                "passed": true,
                "detail": "fails on the untouched scaffold, passes after the agent's edits",
            })),
            Self::Fail(detail) => Some(json!({
                "gate": "check",
                "passed": false,
                "detail": detail,
            })),
            Self::Ungraded(_) => None,
        }
    }

    /// Why no check graded the task.
    fn skip_reason(&self) -> Option<String> {
        match self {
            Self::Ungraded(reason) => Some(reason.clone()),
            Self::Pass | Self::Fail(_) => None,
        }
    }
}

/// The executed check that grades a task, or `None` when nothing can.
///
/// Only the learnable-rust suite has checks. `expected_output` is not one:
/// an agent's output can say anything, and a build or an empty test run
/// prints "Finished" or "test result: ok" without any edit.
fn task_check(suite_id: &str, task_id: &str) -> Option<TaskCheck> {
    if suite_id != LEARNABLE_SUITE_ID {
        return None;
    }
    let test_source = match task_id {
        "grep-fix-todo" => LEARNABLE_CHECK_GREETING,
        "extract-helper-function" => LEARNABLE_CHECK_EXTRACT,
        "fix-broken-import" => LEARNABLE_CHECK_IMPORT,
        "generic-wrap-result" => LEARNABLE_CHECK_WRAP,
        "countup-iterator" => LEARNABLE_CHECK_COUNT_UP,
        _ => return None,
    };
    Some(TaskCheck {
        hidden_files: vec![(LEARNABLE_CHECK_FILE, test_source)],
        command: LEARNABLE_CHECK_COMMAND,
    })
}

/// Run a task's check on the untouched scaffold in `baseline_dir`, then
/// remove that directory so the agent never sees the check's hidden files.
async fn run_baseline_check(
    check: Option<&TaskCheck>,
    baseline_dir: &std::path::Path,
) -> Option<CheckRun> {
    let run = match check {
        Some(check) => Some(run_check(check, baseline_dir).await),
        None => None,
    };
    // Intentionally ignoring: best-effort removal of the baseline copy
    let _ = tokio::fs::remove_dir_all(baseline_dir).await;
    run
}

/// Grade a task on the agent's `workspace`.
///
/// A pass needs an executed check that failed on the untouched scaffold
/// (`baseline`) and passes after the agent's edits. Anything less leaves the
/// task ungraded, never passed.
async fn grade_task(
    check: Option<&TaskCheck>,
    baseline: Option<CheckRun>,
    workspace: &std::path::Path,
) -> TaskGrade {
    let Some(check) = check else {
        return TaskGrade::Ungraded("no executed check is defined for this task".to_string());
    };
    match baseline {
        Some(CheckRun::Failed(_)) => {}
        Some(CheckRun::Passed) => {
            return TaskGrade::Ungraded(
                "the check already passes on the untouched scaffold".to_string(),
            );
        }
        Some(CheckRun::NotRun(reason)) => {
            return TaskGrade::Ungraded(format!(
                "the check did not run on the untouched scaffold: {reason}"
            ));
        }
        None => {
            return TaskGrade::Ungraded(
                "the check did not run on the untouched scaffold".to_string(),
            );
        }
    }
    match run_check(check, workspace).await {
        CheckRun::Passed => TaskGrade::Pass,
        CheckRun::Failed(detail) => TaskGrade::Fail(detail),
        CheckRun::NotRun(reason) => TaskGrade::Ungraded(format!("the check did not run: {reason}")),
    }
}

/// Run `check` in `dir`: write its hidden files, then run its command.
///
/// The command runs code the agent wrote, so it gets the gate environment,
/// which holds no provider keys. It builds into `dir/target`, apart from any
/// other build, and with default compiler flags: an ambient
/// `RUSTFLAGS=-D warnings` (CI sets one) would turn any warning in the
/// scaffold into a failed check.
async fn run_check(check: &TaskCheck, dir: &std::path::Path) -> CheckRun {
    for (relative, contents) in &check.hidden_files {
        if let Err(err) = write_scaffold_file(&dir.join(relative), contents).await {
            return CheckRun::NotRun(format!("{err:#}"));
        }
    }

    let payload = GatePayload::in_dir(dir)
        .with_target_dir(dir.join("target"))
        .with_env("CARGO_ENCODED_RUSTFLAGS", "")
        .with_env("CARGO_TERM_COLOR", "never")
        .with_label("bench-check");
    let body = match Body::from_json(&payload) {
        Ok(body) => body,
        Err(err) => return CheckRun::NotRun(format!("build the check payload: {err}")),
    };
    let signal = Signal::builder(Kind::Task).body(body).build();
    let verdict = ShellGate::new("bash", vec!["-c".to_string(), check.command.to_string()])
        .with_name("bench-check")
        .verify(&signal, &roko_core::Context::now())
        .await;

    if verdict.passed {
        CheckRun::Passed
    } else {
        CheckRun::Failed(check_failure_detail(&verdict))
    }
}

/// A failed check in a few lines: the gate's reason, then the end of the
/// command's output.
fn check_failure_detail(verdict: &roko_core::Verdict) -> String {
    let output = verdict.detail.as_deref().unwrap_or_default().trim();
    let start = output
        .char_indices()
        .rev()
        .nth(CHECK_DETAIL_CHARS - 1)
        .map_or(0, |(idx, _)| idx);
    let tail = &output[start..];
    if tail.is_empty() {
        verdict.reason.clone()
    } else {
        format!("{}\n{tail}", verdict.reason)
    }
}

async fn write_scaffold_file(path: &std::path::Path, contents: &str) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .with_context(|| format!("create {}", parent.display()))?;
    }

    tokio::fs::write(path, contents)
        .await
        .with_context(|| format!("write {}", path.display()))?;
    Ok(())
}

/// The scaffold's manifest. The empty `[workspace]` makes the scaffold its own
/// workspace root, so a `Cargo.toml` above the temp dir cannot break a check.
fn bench_cargo_toml(suite_id: &str, run_id: &str) -> String {
    format!(
        r#"[package]
name = "roko-bench-scaffold"
version = "0.1.0"
edition = "2024"

# suite_id = "{suite_id}"
# run_id = "{run_id}"

[dependencies]

[workspace]
"#,
    )
}

fn bench_main_contents() -> &'static str {
    r#"fn main() {}
"#
}

fn generic_lib_contents() -> &'static str {
    r#"/// Minimal scaffold for non-learnable bench suites.
pub fn scaffold_marker() -> &'static str {
    "roko-bench"
}

#[cfg(test)]
mod tests {}
"#
}

fn learnable_helpers_contents() -> &'static str {
    r#"/// Type referenced by the broken import in `src/lib.rs`.
pub struct MissingType;
"#
}

fn learnable_rust_lib_contents() -> &'static str {
    r#"pub mod helpers;

/// Starter stub for task 1.
pub fn format_greeting(name: &str) -> String {
    // TODO: implement
    let _ = name;
    todo!("format the greeting")
}

/// Task 2 starter: the same loop appears twice so it can be extracted later.
pub fn total_message_bytes(messages: &[&str]) -> usize {
    let mut total = 0;
    for message in messages {
        if !message.trim().is_empty() {
            total += message.len();
        }
    }
    total
}

pub fn total_message_bytes_again(messages: &[&str]) -> usize {
    let mut total = 0;
    for message in messages {
        if !message.trim().is_empty() {
            total += message.len();
        }
    }
    total
}

/// Task 4 starter: keep the helper generic and preserve the incoming result.
pub fn wrap_result<T, E>(value: Result<T, E>) -> Result<T, E> {
    let _ = value;
    unimplemented!("wrap_result should return the input Result unchanged")
}

/// Task 5 starter: implement `Iterator` for this counter.
/// `CountUp` should yield numbers from 1 through `limit`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CountUp {
    /// Last yielded value.
    current: u64,
    /// Inclusive upper bound.
    limit: u64,
}

impl CountUp {
    pub fn new(limit: u64) -> Self {
        Self { current: 0, limit }
    }

    pub fn limit(&self) -> u64 {
        self.limit
    }
}
"#
}

/// Import `fix-broken-import` starts from: `MissingType` lives in `helpers`.
const LEARNABLE_BROKEN_IMPORT: &str = "pub use crate::types::MissingType;\n";

/// `src/lib.rs` for one learnable-rust task. `fix-broken-import` also gets
/// the broken import its prompt asks the agent to repair.
fn learnable_rust_lib_for_task(task_id: &str) -> String {
    let lib = learnable_rust_lib_contents();
    if task_id == "fix-broken-import" {
        lib.replacen(
            "pub mod helpers;\n",
            &format!("pub mod helpers;\n\n{LEARNABLE_BROKEN_IMPORT}"),
            1,
        )
    } else {
        lib.to_string()
    }
}

/// Hidden test for `grep-fix-todo`: the greeting stub is implemented.
const LEARNABLE_CHECK_GREETING: &str = r#"#[test]
fn format_greeting_mentions_the_name() {
    let greeting = roko_bench_scaffold::format_greeting("Ada");
    assert!(greeting.contains("Ada"), "the greeting should mention the name");
}
"#;

/// Hidden test for `extract-helper-function`: the repeated loop lives in one
/// place and both totals are unchanged.
const LEARNABLE_CHECK_EXTRACT: &str = r#"#[test]
fn repeated_loop_lives_in_one_place() {
    let copies = include_str!("../src/lib.rs")
        .matches("for message in messages")
        .count();
    assert!(copies <= 1, "the repeated loop should live in one helper");
}

#[test]
fn totals_are_unchanged() {
    let messages = ["ab", "  ", "", "cde"];
    assert_eq!(roko_bench_scaffold::total_message_bytes(&messages), 5);
    assert_eq!(roko_bench_scaffold::total_message_bytes_again(&messages), 5);
}
"#;

/// Hidden test for `fix-broken-import`: the crate builds and re-exports the
/// type from where it is defined.
const LEARNABLE_CHECK_IMPORT: &str = r#"#[test]
fn missing_type_is_reexported() {
    let _value: roko_bench_scaffold::MissingType = roko_bench_scaffold::MissingType;
}
"#;

/// Hidden test for `generic-wrap-result`: the wrapper returns its input.
const LEARNABLE_CHECK_WRAP: &str = r#"#[test]
fn wrap_result_returns_its_input() {
    assert_eq!(roko_bench_scaffold::wrap_result::<u8, String>(Ok(7)), Ok(7));
    assert_eq!(
        roko_bench_scaffold::wrap_result::<u8, String>(Err("bad".to_string())),
        Err("bad".to_string())
    );
}
"#;

/// Hidden test for `countup-iterator`: `CountUp` yields 1 through its limit.
/// `take` keeps a counter that never stops from hanging the check.
const LEARNABLE_CHECK_COUNT_UP: &str = r#"#[test]
fn count_up_yields_one_through_limit() {
    let values: Vec<u64> = roko_bench_scaffold::CountUp::new(3).take(10).collect();
    assert_eq!(values, vec![1, 2, 3]);
    assert_eq!(roko_bench_scaffold::CountUp::new(0).take(10).count(), 0);
}
"#;

// NOTE: A duplicate `run_bench_regression` was removed here — the canonical
// version lives above (near the end of `execute_bench_run`).

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // Scaffold content helpers
    // -----------------------------------------------------------------------

    #[test]
    fn bench_cargo_toml_embeds_suite_and_run_ids() {
        let toml = bench_cargo_toml("suite-abc", "run-xyz");
        assert!(
            toml.contains("suite-abc"),
            "Cargo.toml should embed the suite id"
        );
        assert!(
            toml.contains("run-xyz"),
            "Cargo.toml should embed the run id"
        );
        assert!(
            toml.contains("[package]"),
            "Cargo.toml should have a [package] section"
        );
        assert!(
            toml.contains("[dependencies]"),
            "Cargo.toml should have a [dependencies] section"
        );
    }

    #[test]
    fn bench_main_contents_is_valid_rust() {
        let main = bench_main_contents();
        assert!(
            main.contains("fn main()"),
            "main.rs should contain a main function"
        );
    }

    #[test]
    fn generic_lib_contents_has_intentional_empty_test_module() {
        let lib = generic_lib_contents();
        assert!(
            lib.contains("#[cfg(test)]"),
            "generic lib should contain a test cfg"
        );
        assert!(
            lib.contains("mod tests {}"),
            "generic lib should contain the intentional empty test module"
        );
        assert!(
            lib.contains("scaffold_marker"),
            "generic lib should contain the scaffold_marker function"
        );
    }

    #[test]
    fn learnable_rust_lib_contents_has_expected_stubs() {
        let lib = learnable_rust_lib_contents();
        assert!(
            lib.contains("format_greeting"),
            "learnable lib should contain format_greeting stub"
        );
        assert!(
            lib.contains("total_message_bytes"),
            "learnable lib should contain total_message_bytes stub"
        );
        assert!(
            lib.contains("wrap_result"),
            "learnable lib should contain wrap_result stub"
        );
        assert!(
            lib.contains("CountUp"),
            "learnable lib should contain CountUp struct"
        );
        assert!(
            lib.contains("pub mod helpers"),
            "learnable lib should declare the helpers module"
        );
    }

    #[test]
    fn learnable_helpers_contents_has_missing_type() {
        let helpers = learnable_helpers_contents();
        assert!(
            helpers.contains("MissingType"),
            "helpers should contain MissingType"
        );
    }

    // -----------------------------------------------------------------------
    // Grading
    // -----------------------------------------------------------------------

    /// The `format_greeting` stub body that `grep-fix-todo` replaces.
    const GREETING_STUB: &str = concat!(
        "    // TODO: implement\n",
        "    let _ = name;\n",
        "    todo!(\"format the greeting\")\n",
    );

    /// The loop body that `total_message_bytes` and its twin both repeat.
    const MESSAGE_TOTAL_BODY: &str = concat!(
        "    let mut total = 0;\n",
        "    for message in messages {\n",
        "        if !message.trim().is_empty() {\n",
        "            total += message.len();\n",
        "        }\n",
        "    }\n",
        "    total\n",
    );

    /// The `wrap_result` stub body that `generic-wrap-result` replaces.
    const WRAP_RESULT_STUB: &str = concat!(
        "    let _ = value;\n",
        "    unimplemented!(\"wrap_result should return the input Result unchanged\")\n",
    );

    /// The `Iterator` impl that `countup-iterator` adds.
    const COUNT_UP_ITERATOR: &str = concat!(
        "\nimpl Iterator for CountUp {\n",
        "    type Item = u64;\n",
        "\n",
        "    fn next(&mut self) -> Option<u64> {\n",
        "        if self.current >= self.limit {\n",
        "            return None;\n",
        "        }\n",
        "        self.current += 1;\n",
        "        Some(self.current)\n",
        "    }\n",
        "}\n",
    );

    /// `src/lib.rs` after a correct agent run of `task_id`, from the task's
    /// untouched `lib`.
    fn reference_lib(task_id: &str, lib: &str) -> String {
        let replace = |from: &str, to: &str, count: usize| {
            assert_eq!(
                lib.matches(from).count(),
                count,
                "{task_id}: the reference edit no longer matches the scaffold"
            );
            lib.replacen(from, to, count)
        };
        match task_id {
            "grep-fix-todo" => replace(GREETING_STUB, "    format!(\"Hello, {}!\", name)\n", 1),
            "extract-helper-function" => {
                let extracted = replace(MESSAGE_TOTAL_BODY, "    non_blank_bytes(messages)\n", 2);
                let helper = "fn non_blank_bytes(messages: &[&str]) -> usize {\n";
                format!("{extracted}\n{helper}{MESSAGE_TOTAL_BODY}}}\n")
            }
            "fix-broken-import" => replace(
                "pub use crate::types::MissingType;",
                "pub use crate::helpers::MissingType;",
                1,
            ),
            "generic-wrap-result" => replace(WRAP_RESULT_STUB, "    value\n", 1),
            "countup-iterator" => format!("{lib}{COUNT_UP_ITERATOR}"),
            other => panic!("no reference solution for {other}"),
        }
    }

    fn leave_unedited(_task_id: &str, _workspace: &std::path::Path) {}

    fn apply_reference_solution(task_id: &str, workspace: &std::path::Path) {
        let lib_path = workspace.join("src").join("lib.rs");
        let lib = std::fs::read_to_string(&lib_path).expect("read lib.rs");
        std::fs::write(&lib_path, reference_lib(task_id, &lib)).expect("write lib.rs");
    }

    /// Scaffold every learnable-rust task under `run_dir`, run each task's
    /// baseline check, let `edit` stand in for the agent, then grade.
    async fn grade_learnable_tasks(
        run_dir: &std::path::Path,
        edit: fn(&str, &std::path::Path),
    ) -> Vec<(String, TaskGrade)> {
        let suite = bench::builtin_learnable_rust_suite();
        scaffold_task_dirs(run_dir, &suite, "test-run")
            .await
            .expect("scaffold learnable tasks");
        let grades = suite
            .tasks
            .iter()
            .enumerate()
            .map(|(idx, task)| async move {
                let dirs = TaskDirs::new(run_dir, idx, &task.id);
                let check = task_check(LEARNABLE_SUITE_ID, &task.id);
                assert!(check.is_some(), "{} has no check", task.id);
                let baseline = run_baseline_check(check.as_ref(), &dirs.baseline).await;
                edit(&task.id, &dirs.workspace);
                let grade = grade_task(check.as_ref(), baseline, &dirs.workspace).await;
                (task.id.clone(), grade)
            });
        futures::future::join_all(grades).await
    }

    #[tokio::test]
    async fn unedited_learnable_scaffold_fails_grading() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let grades = grade_learnable_tasks(tmp.path(), leave_unedited).await;
        assert_eq!(grades.len(), 5, "every learnable task should be graded");
        for (task_id, grade) in grades {
            assert!(
                matches!(grade, TaskGrade::Fail(_)),
                "{task_id}: an unedited scaffold must fail its check, got {grade:?}"
            );
        }
    }

    #[tokio::test]
    async fn learnable_reference_solutions_pass_grading() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let grades = grade_learnable_tasks(tmp.path(), apply_reference_solution).await;
        assert_eq!(grades.len(), 5, "every learnable task should be graded");
        for (task_id, grade) in grades {
            assert_eq!(
                grade,
                TaskGrade::Pass,
                "{task_id}: the reference solution should pass its check"
            );
        }
    }

    #[tokio::test]
    async fn only_a_check_that_fails_first_can_pass_a_task() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let baseline = tmp.path().join("baseline");
        let workspace = tmp.path().join("workspace");
        std::fs::create_dir_all(&baseline).expect("create baseline");
        std::fs::create_dir_all(&workspace).expect("create workspace");

        // No check: the task is never a pass, whatever the agent reported.
        assert!(matches!(
            grade_task(None, None, &workspace).await,
            TaskGrade::Ungraded(_)
        ));

        // A check that passes on the untouched scaffold cannot grade anything.
        let vacuous = TaskCheck {
            hidden_files: Vec::new(),
            command: "true",
        };
        let vacuous_baseline = run_check(&vacuous, &baseline).await;
        assert_eq!(vacuous_baseline, CheckRun::Passed);
        assert!(matches!(
            grade_task(Some(&vacuous), Some(vacuous_baseline), &workspace).await,
            TaskGrade::Ungraded(_)
        ));

        // A real check fails before the change and passes after it. Its
        // hidden file is written only when the check runs.
        let real = TaskCheck {
            hidden_files: vec![("check.sh", "test -f done.txt\n")],
            command: "sh check.sh",
        };
        let real_baseline = run_check(&real, &baseline).await;
        assert!(matches!(real_baseline, CheckRun::Failed(_)));
        assert!(!workspace.join("check.sh").exists());
        assert!(matches!(
            grade_task(Some(&real), Some(real_baseline.clone()), &workspace).await,
            TaskGrade::Fail(_)
        ));
        std::fs::write(workspace.join("done.txt"), "done").expect("write done.txt");
        assert_eq!(
            grade_task(Some(&real), Some(real_baseline), &workspace).await,
            TaskGrade::Pass
        );
    }

    #[test]
    fn expected_output_is_not_a_check() {
        // Built-in suites carry expected_output hints; only learnable-rust
        // tasks have executed checks.
        assert_eq!(task_check("smoke", "hello"), None);
        assert_eq!(task_check("codegen", "fizzbuzz"), None);
        assert_eq!(task_check(LEARNABLE_SUITE_ID, "no-such-task"), None);
        for task in bench::builtin_learnable_rust_suite().tasks {
            assert!(
                task_check(LEARNABLE_SUITE_ID, &task.id).is_some(),
                "{} should have a check",
                task.id
            );
        }
    }

    #[test]
    fn task_dirs_are_distinct_and_stay_under_the_run_dir() {
        let run_dir = std::path::Path::new("/tmp/roko-bench-run");
        let first = TaskDirs::new(run_dir, 0, "../escape");
        let second = TaskDirs::new(run_dir, 1, "../escape");
        assert_ne!(first.root, second.root);
        assert_eq!(first.root.parent(), Some(run_dir));
        assert!(first.workspace.starts_with(&first.root));
        assert!(first.baseline.starts_with(&first.root));
        assert_ne!(first.workspace, first.baseline);
    }

    fn graded_result(idx: usize, status: &str) -> BenchTaskResult {
        BenchTaskResult {
            task_id: format!("task-{idx}"),
            task_name: format!("Task {idx}"),
            status: status.to_string(),
            duration_ms: 1_000,
            model: "test-model".to_string(),
            tokens_in: 100,
            tokens_out: 50,
            cost_usd: 0.01,
            gate_verdicts: Vec::new(),
            retries_used: 0,
            output_preview: None,
            error: None,
            skip_reason: None,
        }
    }

    /// A completed run of `suite` with five tasks, all graded `status`.
    fn stored_run(id: &str, status: &str, overrides: &BenchConfigOverrides) -> BenchRun {
        BenchRun {
            id: id.to_string(),
            suite_id: "suite".to_string(),
            suite_name: "Suite".to_string(),
            kind: BenchRunKind::Manual,
            overrides: overrides.clone(),
            label: None,
            simulated: false,
            status: BenchRunStatus::Completed,
            started_at: 1_700_000_000,
            finished_at: Some(1_700_000_100),
            results: (0..5).map(|idx| graded_result(idx, status)).collect(),
            summary: None,
            current_task_index: 5,
            total_tasks: 5,
        }
    }

    #[tokio::test]
    async fn bench_regression_reads_the_stored_runs() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let overrides = BenchConfigOverrides {
            model: Some("test-model".to_string()),
            ..BenchConfigOverrides::default()
        };
        // Every task passed in the stored run, where save_bench_run put it...
        let earlier = stored_run("earlier", "pass", &overrides);
        bench::save_bench_run(tmp.path(), &earlier)
            .await
            .expect("save the earlier run");

        // ...and every task fails now.
        let failing: Vec<BenchTaskResult> = (0..5).map(|idx| graded_result(idx, "fail")).collect();
        let report = bench_regression_report(tmp.path(), "now", "suite", &failing, &overrides)
            .await
            .expect("the stored run gives the check a baseline");
        assert!(report.sufficient_data);
        assert!(report.has_regressions, "{report:?}");
    }

    fn bench_state() -> (tempfile::TempDir, Arc<AppState>) {
        let dir = tempfile::tempdir().expect("tempdir");
        let deploy_backend = Arc::from(
            crate::deploy::create_backend("manual", None, None, None).expect("manual backend"),
        );
        let state = AppState::new(
            dir.path().to_path_buf(),
            Arc::new(crate::runtime::NoOpRuntime),
            roko_core::config::schema::RokoConfig::default(),
            deploy_backend,
        )
        .expect("AppState::new");
        (dir, Arc::new(state))
    }

    /// Start a smoke-suite run through the handler, wait until it is done,
    /// and return its id.
    async fn run_smoke_bench(state: &Arc<AppState>, strategy: &str, model: &str) -> String {
        let request: StartBenchRequest = serde_json::from_value(json!({
            "suite_id": "smoke",
            "overrides": { "model": model, "strategy": strategy }
        }))
        .expect("bench request");
        let response = start_bench_run(State(Arc::clone(state)), Json(request))
            .await
            .expect("start bench run")
            .into_response();
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("response body");
        let started: Value = serde_json::from_slice(&body).expect("JSON body");
        let run_id = started["id"].as_str().expect("run id").to_string();

        // execute_bench_run drops the run's handle as its last step.
        for _ in 0..1_500 {
            if !state.active_bench_runs.read().await.contains_key(&run_id) {
                return run_id;
            }
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
        panic!("bench run {run_id} did not finish");
    }

    #[tokio::test]
    async fn demo_bench_runs_stay_out_of_the_index() {
        let (_dir, state) = bench_state();
        let demo_id = run_smoke_bench(&state, "demo", "demo-model").await;
        let real_id = run_smoke_bench(&state, "minimal", "real-model").await;

        // The demo run is stored and readable by id, marked simulated...
        let demo = bench::load_bench_run(&state.workdir, &demo_id)
            .await
            .expect("load demo run")
            .expect("demo run stored");
        assert!(demo.simulated);
        assert_eq!(demo.status, BenchRunStatus::Completed);
        let real = bench::load_bench_run(&state.workdir, &real_id)
            .await
            .expect("load real run")
            .expect("real run stored");
        assert!(!real.simulated);

        // ...but only the measured run is indexed, so the run list, the
        // pareto frontier and the cost summary never see demo figures.
        let indexed: Vec<String> = bench::load_index_entries(&state.workdir)
            .await
            .into_iter()
            .map(|entry| entry.id)
            .collect();
        assert_eq!(indexed, vec![real_id]);
        let frontier = bench::compute_pareto_frontier(&state.workdir).await;
        assert!(frontier.iter().all(|point| point.run_id != demo_id));
        let Json(costs) = cost_summary(State(Arc::clone(&state))).await;
        let models: Vec<&str> = costs["models"]
            .as_array()
            .expect("models")
            .iter()
            .filter_map(|row| row["model"].as_str())
            .collect();
        assert_eq!(models, vec!["real-model"]);
    }

    #[test]
    fn only_fix_broken_import_starts_with_the_broken_import() {
        let broken = learnable_rust_lib_for_task("fix-broken-import");
        assert!(broken.contains(LEARNABLE_BROKEN_IMPORT));
        let intact = learnable_rust_lib_for_task("grep-fix-todo");
        assert!(!intact.contains(LEARNABLE_BROKEN_IMPORT));
    }

    // -----------------------------------------------------------------------
    // Request type parsing
    // -----------------------------------------------------------------------

    #[test]
    fn start_bench_request_deserializes_minimal() {
        let json = serde_json::json!({ "suite_id": "quick" });
        let req: StartBenchRequest = serde_json::from_value(json).expect("parse");
        assert_eq!(req.suite_id, "quick");
        assert!(req.label.is_none());
    }

    #[test]
    fn start_bench_request_deserializes_with_label_and_overrides() {
        let json = serde_json::json!({
            "suite_id": "quick",
            "label": "nightly-run",
            "overrides": { "model": "claude-3" }
        });
        let req: StartBenchRequest = serde_json::from_value(json).expect("parse");
        assert_eq!(req.suite_id, "quick");
        assert_eq!(req.label.as_deref(), Some("nightly-run"));
        assert_eq!(req.overrides.model.as_deref(), Some("claude-3"));
    }

    #[test]
    fn list_runs_query_defaults() {
        let json = serde_json::json!({});
        let query: ListRunsQuery = serde_json::from_value(json).expect("parse");
        assert!(query.suite_id.is_none());
        assert!(query.status.is_none());
        assert_eq!(query.limit, 50);
        assert_eq!(query.offset, 0);
    }

    #[test]
    fn compare_query_parses_ids() {
        let json = serde_json::json!({ "ids": "a,b,c" });
        let query: CompareQuery = serde_json::from_value(json).expect("parse");
        let ids: Vec<&str> = query.ids.split(',').collect();
        assert_eq!(ids, vec!["a", "b", "c"]);
    }

    #[test]
    fn matrix_lane_request_deserializes_minimal() {
        let json = serde_json::json!({ "model": "gpt-4" });
        let lane: MatrixLaneRequest = serde_json::from_value(json).expect("parse");
        assert_eq!(lane.model, "gpt-4");
        assert!(lane.backend.is_none());
        assert!(lane.label.is_none());
    }

    #[test]
    fn start_matrix_request_deserializes() {
        let json = serde_json::json!({
            "suite_id": "quick",
            "lanes": [
                { "model": "gpt-4" },
                { "model": "claude-3", "label": "lane-b" }
            ]
        });
        let req: StartMatrixRequest = serde_json::from_value(json).expect("parse");
        assert_eq!(req.suite_id, "quick");
        assert_eq!(req.lanes.len(), 2);
        assert_eq!(req.lanes[1].label.as_deref(), Some("lane-b"));
    }

    // -----------------------------------------------------------------------
    // Utility helpers
    // -----------------------------------------------------------------------

    #[test]
    fn now_secs_is_nonzero() {
        let ts = now_secs();
        assert!(ts > 1_700_000_000, "timestamp should be after 2023");
    }

    #[test]
    fn default_limit_is_50() {
        assert_eq!(default_limit(), 50);
    }

    // -----------------------------------------------------------------------
    // Route registration sanity
    // -----------------------------------------------------------------------

    /// Verify `routes()` returns a non-empty router that can be merged into
    /// a parent router without panicking.
    #[test]
    fn bench_routes_can_be_constructed() {
        // `routes()` must not panic during construction.
        let _router = routes();
    }
}
