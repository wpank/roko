//! Event-driven fan-out from the runtime event bus into learning systems.
//!
//! Spawned by runner v2's event loop during plan execution. Provider health
//! is intentionally excluded: live dispatch call sites record it directly so
//! lossy or replayed bus events cannot duplicate circuit-breaker outcomes.
//!
//! The current event schema does not carry full turn identity on every event,
//! so this subscriber keeps the latest started turn in memory and uses it to
//! enrich later `TurnCompleted` and `ToolCallExecuted` events.

use std::collections::HashMap;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use chrono::{Duration, Utc};
use tokio::sync::{broadcast, mpsc};

use roko_agent::chat_types::FinishReason;
use roko_core::dashboard_snapshot::DashboardEvent;

use crate::aggregate::{self, EfficiencyBucket, JsonlCursor};
use crate::anomaly::AnomalyDetector;
use crate::calibration_policy::CalibrationPolicy;
use crate::cascade_router::CascadeRouter;
use crate::cost_table::CostTable;
use crate::costs_db::{CostsDb, create_cost_record};
use crate::efficiency::{AgentEfficiencyEvent, ToolCallMeta};
use crate::events::AgentEvent;
use crate::latency::LatencyRegistry;
use crate::verdict_scorer::{VerdictHistory, VerdictRecord};

#[derive(Debug, Clone)]
struct ActiveTurn {
    plan_id: String,
    task_id: String,
    attempt_id_base: String,
    model: String,
    provider: String,
    tool_calls: Vec<ToolCallMeta>,
}

impl ActiveTurn {
    fn from_started(task_id: &str, model: &str, provider: &str) -> Self {
        let mut parts = task_id.splitn(3, ':');
        let first = parts.next().unwrap_or_default();
        let second = parts.next();
        let third = parts.next();
        let (plan_id, task_id, attempt_id_base) = match (second, third) {
            (Some(task), Some(attempt)) => (
                first.to_string(),
                task.to_string(),
                format!("{task}:{attempt}"),
            ),
            (Some(task), None) => (first.to_string(), task.to_string(), task.to_string()),
            (None, _) => (String::new(), first.to_string(), first.to_string()),
        };
        Self {
            plan_id,
            task_id,
            attempt_id_base,
            model: model.to_string(),
            provider: provider.to_string(),
            tool_calls: Vec::new(),
        }
    }
}

/// Buffered efficiency event awaiting its corresponding `GateResult`.
///
/// Used in the runner-v2 async path where `TurnCompleted` fires before the
/// gate pipeline produces a verdict. Keyed by `task_id` in the subscriber
/// state and completed when the matching `GateResult` arrives.
struct PendingEfficiency {
    event: AgentEfficiencyEvent,
    /// Model slug, needed to call `router.record_confidence_outcome`.
    model: String,
}

/// Consume `AgentEvent`s and update the learning subsystems that depend on them.
///
/// When `dashboard_tx` is attached, each appended efficiency event also pushes
/// a refreshed efficiency trend and cascade-router snapshot to the TUI bridge
/// so connected-mode dashboards stay live without polling the learning files.
pub async fn run_learning_subscriber(
    mut rx: broadcast::Receiver<AgentEvent>,
    latency: Arc<LatencyRegistry>,
    router: Arc<CascadeRouter>,
    anomaly: Arc<Mutex<AnomalyDetector>>,
    costs: Arc<CostsDb>,
    efficiency_path: PathBuf,
    router_persist_path: Option<PathBuf>,
    dashboard_tx: Option<mpsc::UnboundedSender<DashboardEvent>>,
) {
    let cost_table = CostTable {
        models: HashMap::new(),
    }
    .with_defaults();
    let mut active_turn: Option<ActiveTurn> = None;
    let mut calibration_policy = CalibrationPolicy::new();
    let mut verdict_history = VerdictHistory::new();
    // Efficiency events waiting for their `GateResult` (runner-v2 async path).
    let mut pending_efficiency: HashMap<String, PendingEfficiency> = HashMap::new();
    // Incremental trend state for the dashboard push path. The cursor tracks
    // the efficiency log so each append costs O(new lines), not O(file).
    let mut trend_cursor = dashboard_tx
        .as_ref()
        .map(|_| JsonlCursor::new(&efficiency_path));
    let mut trend_buckets: Vec<EfficiencyBucket> = Vec::new();

    loop {
        let event = match rx.recv().await {
            Ok(event) => event,
            Err(broadcast::error::RecvError::Lagged(skipped)) => {
                tracing::warn!(skipped, "learning subscriber lagged behind event stream");
                continue;
            }
            Err(broadcast::error::RecvError::Closed) => break,
        };

        match event {
            AgentEvent::TurnStarted {
                ref task_id,
                ref model,
                ref provider,
                ..
            } => {
                active_turn = Some(ActiveTurn::from_started(task_id, model, provider));
                // Feed to calibration policy for predict-publish-correct loop (LEARN-09).
                let _ = calibration_policy.process_event(&event);
            }
            AgentEvent::TurnCompleted {
                turn,
                usage,
                tool_call_count,
                gate_passed,
                ref finish_reason,
            } => {
                // Feed calibration policy for predict-publish-correct loop (LEARN-09).
                if let Some(correction) = calibration_policy.process_event(&event) {
                    tracing::info!(
                        model = %correction.model,
                        category = %correction.category,
                        bias = correction.mean_bias,
                        sample_count = correction.sample_count,
                        "calibration correction triggered — applying to cascade router"
                    );

                    // Apply the correction to the cascade router's confidence stats.
                    router.apply_calibration_correction(&correction);

                    // Persist the updated router state so corrections survive restarts.
                    if let Some(ref persist_path) = router_persist_path {
                        if let Err(err) = router.save(persist_path) {
                            tracing::warn!(
                                path = %persist_path.display(),
                                error = %err,
                                "failed to persist cascade router after calibration correction"
                            );
                        }
                    }
                }

                let Some(turn_ctx) = active_turn.take() else {
                    continue;
                };

                // Always record the cost — the token spend is real regardless of
                // whether the gate result has arrived yet.
                let success_for_cost = gate_passed.unwrap_or(false);
                let cost_record = create_cost_record(
                    Utc::now().to_rfc3339(),
                    &turn_ctx.model,
                    &turn_ctx.provider,
                    "",
                    "",
                    &turn_ctx.task_id,
                    "",
                    &usage,
                    &cost_table,
                    usage.wall_ms,
                    success_for_cost,
                    "",
                );
                costs.insert(cost_record);

                let tools_used = tool_call_count.min(u32::MAX as usize) as u32;
                let attempt_id = format!("{}:{turn}", turn_ctx.attempt_id_base);
                // Clone before struct field moves consume these values.
                let task_id_key = turn_ctx.task_id.clone();
                let model_key = turn_ctx.model.clone();
                let efficiency_event = AgentEfficiencyEvent {
                    agent_id: format!("{}:{turn}", turn_ctx.task_id),
                    role: String::new(),
                    backend: turn_ctx.provider.clone(),
                    model: turn_ctx.model.clone(),
                    plan_id: turn_ctx.plan_id,
                    task_id: turn_ctx.task_id,
                    attempt_id,
                    input_tokens: u64::from(usage.input_tokens),
                    output_tokens: u64::from(usage.output_tokens),
                    reasoning_tokens: u64::from(usage.reasoning_tokens),
                    cache_read_tokens: u64::from(usage.cache_read_tokens),
                    cache_write_tokens: u64::from(usage.cache_create_tokens),
                    cost_usd: f64::from(usage.cost_usd),
                    cost_usd_without_cache: f64::from(usage.cost_usd),
                    prompt_sections: Vec::new(),
                    total_prompt_tokens: u64::from(usage.input_tokens),
                    system_prompt_tokens: 0,
                    tools_available: 0,
                    tools_used,
                    tool_calls: turn_ctx.tool_calls,
                    wall_time_ms: usage.wall_ms,
                    duration_ms: usage.wall_ms,
                    time_to_first_token_ms: 0,
                    was_warm_start: false,
                    iteration: turn,
                    turn_number: turn,
                    // Semantics: "final turn of the attempt". A gate verdict
                    // concludes the attempt, so the turn is final iff its gate
                    // outcome is already known here. Deferred events (`None`)
                    // are flipped to `true` when the `GateResult` lands.
                    is_final_turn: gate_passed.is_some(),
                    gate_passed,
                    outcome: match gate_passed {
                        Some(true) => "success".to_string(),
                        _ => finish_reason_label(&finish_reason).to_string(),
                    },
                    gate_errors: Vec::new(),
                    model_used: turn_ctx.model,
                    frequency: roko_core::OperatingFrequency::Theta,
                    strategy_attempted: String::new(),
                    timestamp: Utc::now().to_rfc3339(),
                };

                match gate_passed {
                    Some(passed) => {
                        // ACP inline path: gate result is known with TurnCompleted.
                        let _ = router.record_confidence_outcome(&model_key, passed);
                        if let Err(err) =
                            append_efficiency_event(&efficiency_path, &efficiency_event).await
                        {
                            tracing::warn!(
                                path = %efficiency_path.display(),
                                error = %err,
                                "failed to append efficiency event"
                            );
                        } else {
                            publish_learning_updates(
                                dashboard_tx.as_ref(),
                                &mut trend_cursor,
                                &mut trend_buckets,
                                &efficiency_path,
                                &router,
                            );
                        }
                    }
                    None => {
                        // runner-v2 async path: gate runs after TurnCompleted.
                        // Buffer the event until the matching GateResult arrives.
                        pending_efficiency.insert(
                            task_id_key,
                            PendingEfficiency {
                                event: efficiency_event,
                                model: model_key,
                            },
                        );
                    }
                }
            }
            AgentEvent::ProviderError { .. } => {}
            AgentEvent::ToolCallExecuted {
                tool_name,
                duration_ms,
                success,
                result_tokens,
            } => {
                if let Some(turn_ctx) = active_turn.as_mut() {
                    turn_ctx.tool_calls.push(ToolCallMeta {
                        tool_name,
                        duration_ms,
                        result_tokens,
                        succeeded: success,
                        advanced_task: success,
                        was_redundant: false,
                        error_category: (!success).then_some("tool_execution_failed".to_string()),
                    });
                    latency.record(
                        &turn_ctx.model,
                        &turn_ctx.provider,
                        duration_ms as f64,
                        duration_ms as f64,
                        result_tokens,
                    );
                }
            }
            AgentEvent::CostRecorded { cost_usd, .. } => {
                if let Ok(mut detector) = anomaly.lock() {
                    let _ = detector.check_cost(cost_usd);
                }
            }
            AgentEvent::GateResult {
                ref gate_name,
                passed,
                ref task_id,
                ..
            } => {
                // Feed verdict history for routing penalty computation (GATE-05).
                if let Some(turn_ctx) = &active_turn {
                    verdict_history.record(VerdictRecord {
                        model_slug: turn_ctx.model.clone(),
                        task_type: String::new(), // filled from context when available
                        target_crate: String::new(),
                        gate: gate_name.clone(),
                        passed,
                        timestamp_ms: Utc::now().timestamp_millis(),
                    });
                }

                // Flush the buffered efficiency event now that we know the gate outcome.
                if let Some(mut pending) = pending_efficiency.remove(task_id) {
                    pending.event.gate_passed = Some(passed);
                    // The gate verdict concludes the attempt, so this buffered
                    // turn is the final turn of the attempt.
                    pending.event.is_final_turn = true;
                    pending.event.outcome = if passed {
                        "success".to_string()
                    } else {
                        "gate_failed".to_string()
                    };
                    let _ = router.record_confidence_outcome(&pending.model, passed);
                    if let Err(err) =
                        append_efficiency_event(&efficiency_path, &pending.event).await
                    {
                        tracing::warn!(
                            path = %efficiency_path.display(),
                            error = %err,
                            task_id = %task_id,
                            "failed to write deferred efficiency event"
                        );
                    } else {
                        publish_learning_updates(
                            dashboard_tx.as_ref(),
                            &mut trend_cursor,
                            &mut trend_buckets,
                            &efficiency_path,
                            &router,
                        );
                    }
                }
            }
            AgentEvent::SafetyDenial {
                ref tool_name,
                ref denial_reason,
                ref task_id,
                timestamp,
            } => {
                let record = serde_json::json!({
                    "tool_name": tool_name,
                    "denial_reason": denial_reason,
                    "task_id": task_id,
                    "timestamp_ms": timestamp,
                });
                if let Ok(line) = serde_json::to_string(&record) {
                    let denial_path = efficiency_path
                        .parent()
                        .unwrap_or(std::path::Path::new("."))
                        .join("safety-denials.jsonl");
                    if let Some(parent) = denial_path.parent() {
                        let _ = tokio::fs::create_dir_all(parent).await;
                    }
                    let dp = denial_path.clone();
                    let _ = tokio::task::spawn_blocking(move || {
                        roko_fs::log_rotation::append_jsonl_line_sync(
                            &dp,
                            line.as_bytes(),
                            roko_core::config::ResourcesConfig::default().log_rotation_max_mb,
                        )
                    })
                    .await;
                }
                tracing::info!(
                    tool = %tool_name,
                    reason = %denial_reason,
                    task = %task_id,
                    "safety denial recorded"
                );
            }
            AgentEvent::AnomalyDetected { .. }
            | AgentEvent::ExperimentAssigned { .. }
            | AgentEvent::SessionEstablished { .. }
            | AgentEvent::ModelSelected { .. }
            | AgentEvent::SomaticMarkerFired { .. }
            | AgentEvent::StreamChunk { .. } => {}
        }
    }

    // Flush remaining buffered events that never received a GateResult (e.g. task
    // errored before gating, or subscriber shut down during a gate run). These
    // keep `is_final_turn: false`: no gate verdict ever concluded the attempt.
    for (task_id, pending) in pending_efficiency.drain() {
        tracing::debug!(task_id = %task_id, "flushing ungated efficiency event on shutdown");
        if let Err(err) = append_efficiency_event(&efficiency_path, &pending.event).await {
            tracing::warn!(
                path = %efficiency_path.display(),
                error = %err,
                task_id = %task_id,
                "failed to flush ungated efficiency event"
            );
        } else {
            publish_learning_updates(
                dashboard_tx.as_ref(),
                &mut trend_cursor,
                &mut trend_buckets,
                &efficiency_path,
                &router,
            );
        }
    }
}

fn finish_reason_label(reason: &FinishReason) -> &str {
    match reason {
        FinishReason::Stop => "stop",
        FinishReason::Length => "length",
        FinishReason::ToolCalls => "tool_calls",
        FinishReason::ContentFilter => "content_filter",
        FinishReason::Error(_) => "error",
    }
}

async fn append_efficiency_event(path: &Path, event: &AgentEfficiencyEvent) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }

    let line = serde_json::to_string(event)
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;
    let path = path.to_path_buf();
    tokio::task::spawn_blocking(move || {
        roko_fs::log_rotation::append_jsonl_line_sync(
            &path,
            line.as_bytes(),
            roko_core::config::ResourcesConfig::default().log_rotation_max_mb,
        )
    })
    .await
    .map_err(|error| io::Error::other(format!("efficiency append task failed: {error}")))??;
    Ok(())
}

/// Push refreshed learning snapshots to the TUI bridge after an efficiency
/// event lands on disk.
///
/// The efficiency trend is recomputed incrementally from a cursor over the
/// log (O(new lines) per append) and the cascade-router snapshot is forwarded
/// so connected-mode dashboards see confidence updates as they happen.
/// Best-effort: send failures just mean no dashboard consumer is attached.
fn publish_learning_updates(
    dashboard_tx: Option<&mpsc::UnboundedSender<DashboardEvent>>,
    trend_cursor: &mut Option<JsonlCursor>,
    trend_buckets: &mut Vec<EfficiencyBucket>,
    efficiency_path: &Path,
    router: &CascadeRouter,
) {
    let (Some(tx), Some(cursor)) = (dashboard_tx, trend_cursor.as_mut()) else {
        return;
    };
    match aggregate::efficiency_trend_with_cursor(cursor, trend_buckets, Duration::hours(1), 24) {
        Ok(buckets) => {
            *trend_buckets = buckets.clone();
            let buckets = buckets
                .iter()
                .map(|bucket| roko_core::dashboard_snapshot::EfficiencyBucket {
                    start: bucket.start,
                    turns: bucket.turns,
                    tokens_in: bucket.tokens_in,
                    tokens_out: bucket.tokens_out,
                    cost_usd_cents: bucket.cost_usd_cents,
                    latency_ms_avg: bucket.latency_ms_avg,
                })
                .collect();
            let _ = tx.send(DashboardEvent::EfficiencyTrendUpdated { buckets });
        }
        Err(err) => {
            tracing::warn!(
                path = %efficiency_path.display(),
                error = %err,
                "failed to refresh efficiency trend for dashboard push"
            );
        }
    }
    let _ = tx.send(DashboardEvent::CascadeRouterUpdated {
        snapshot_json: router.snapshot_json(),
    });
}

#[cfg(test)]
mod tests {
    use super::run_learning_subscriber;
    use std::sync::{Arc, Mutex};

    use tempfile::TempDir;
    use tokio::sync::broadcast;

    use crate::anomaly::AnomalyDetector;
    use crate::cascade_router::CascadeRouter;
    use crate::costs_db::CostsDb;
    use crate::events::AgentEvent;
    use crate::latency::LatencyRegistry;
    use crate::runtime_feedback::read_efficiency_events;
    use roko_agent::Usage;
    use roko_agent::chat_types::FinishReason;

    #[tokio::test]
    async fn event_subscriber_turn_completed_updates_router_costs_and_efficiency() {
        let (tx, rx) = broadcast::channel(16);
        let tempdir = TempDir::new().expect("tempdir");
        let efficiency_path = tempdir.path().join("efficiency.jsonl");
        let router_persist_path = tempdir.path().join("cascade-router.json");

        let latency = Arc::new(LatencyRegistry::new());
        let router = Arc::new(CascadeRouter::new(vec!["glm-5.1".to_string()]));
        let anomaly = Arc::new(Mutex::new(AnomalyDetector::new(1_700_000_000_000)));
        let costs = Arc::new(CostsDb::new());

        let handle = tokio::spawn(run_learning_subscriber(
            rx,
            Arc::clone(&latency),
            Arc::clone(&router),
            Arc::clone(&anomaly),
            Arc::clone(&costs),
            efficiency_path.clone(),
            Some(router_persist_path),
            None,
        ));

        tx.send(AgentEvent::TurnStarted {
            task_id: "task-2k22".into(),
            model: "glm-5.1".into(),
            provider: "zai".into(),
            timestamp_ms: 1_700_000_000_000,
        })
        .expect("turn started");
        tx.send(AgentEvent::ToolCallExecuted {
            tool_name: "Read".into(),
            duration_ms: 33,
            success: true,
            result_tokens: 128,
        })
        .expect("tool call");
        tx.send(AgentEvent::TurnCompleted {
            turn: 2,
            usage: Usage {
                input_tokens: 120,
                output_tokens: 45,
                cache_read_tokens: 10,
                cache_create_tokens: 2,
                reasoning_tokens: 0,
                cost_usd: 0.12,
                wall_ms: 850,
            },
            tool_call_count: 1,
            gate_passed: Some(true),
            finish_reason: FinishReason::Stop,
        })
        .expect("turn completed");

        drop(tx);
        handle.await.expect("subscriber task");

        let snapshot = router.confidence_snapshot();
        assert_eq!(snapshot.get("glm-5.1"), Some(&(1, 1)));

        let records = costs.all();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].model, "glm-5.1");
        assert_eq!(records[0].provider, "zai");
        assert_eq!(records[0].task_id, "task-2k22");
        assert!((records[0].cost_usd - 0.12).abs() < 1e-6);

        let events = read_efficiency_events(&efficiency_path)
            .await
            .expect("read efficiency events");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].model, "glm-5.1");
        assert_eq!(events[0].task_id, "task-2k22");
        assert_eq!(events[0].tools_used, 1);
        assert_eq!(events[0].tool_calls.len(), 1);
        assert_eq!(events[0].gate_passed, Some(true));
    }

    #[tokio::test]
    async fn event_subscriber_records_tool_latency_without_provider_health_side_effects() {
        let (tx, rx) = broadcast::channel(16);
        let tempdir = TempDir::new().expect("tempdir");
        let efficiency_path = tempdir.path().join("efficiency.jsonl");

        let latency = Arc::new(LatencyRegistry::new());
        let router = Arc::new(CascadeRouter::new(vec!["glm-5.1".to_string()]));
        let anomaly = Arc::new(Mutex::new(AnomalyDetector::new(1_700_000_000_000)));
        let costs = Arc::new(CostsDb::new());

        let handle = tokio::spawn(run_learning_subscriber(
            rx,
            Arc::clone(&latency),
            router,
            anomaly,
            costs,
            efficiency_path,
            None, // no persist path needed for this test
            None,
        ));

        tx.send(AgentEvent::TurnStarted {
            task_id: "task-latency".into(),
            model: "glm-5.1".into(),
            provider: "zai".into(),
            timestamp_ms: 1_700_000_000_000,
        })
        .expect("turn started");
        tx.send(AgentEvent::ToolCallExecuted {
            tool_name: "Read".into(),
            duration_ms: 50,
            success: true,
            result_tokens: 64,
        })
        .expect("tool call");
        drop(tx);
        handle.await.expect("subscriber task");

        let stats = latency.get("glm-5.1", "zai").expect("latency stats");
        assert_eq!(stats.observations, 1);
        assert_eq!(stats.recent_latencies, vec![50.0]);
    }

    #[tokio::test]
    async fn calibration_correction_applies_to_router_and_persists() {
        let (tx, rx) = broadcast::channel(64);
        let tempdir = TempDir::new().expect("tempdir");
        let efficiency_path = tempdir.path().join("efficiency.jsonl");
        let router_persist_path = tempdir.path().join("cascade-router.json");

        let latency = Arc::new(LatencyRegistry::new());
        let router = Arc::new(CascadeRouter::new(vec!["overconfident-model".to_string()]));
        let anomaly = Arc::new(Mutex::new(AnomalyDetector::new(1_700_000_000_000)));
        let costs = Arc::new(CostsDb::new());

        let handle = tokio::spawn(run_learning_subscriber(
            rx,
            Arc::clone(&latency),
            Arc::clone(&router),
            Arc::clone(&anomaly),
            Arc::clone(&costs),
            efficiency_path,
            Some(router_persist_path.clone()),
            None,
        ));

        // Send enough failing turns to trigger a calibration correction.
        // The default CalibrationPolicy triggers at 10 samples with bias > 0.15.
        // Model predicts 0.7 success probability but always fails → bias = 0.7.
        for i in 0..12 {
            tx.send(AgentEvent::TurnStarted {
                task_id: format!("cal-task-{i}"),
                model: "overconfident-model".into(),
                provider: "test-provider".into(),
                timestamp_ms: 1_700_000_000_000 + i,
            })
            .expect("send turn started");
            tx.send(AgentEvent::TurnCompleted {
                turn: 1,
                usage: Usage {
                    input_tokens: 100,
                    output_tokens: 50,
                    cache_read_tokens: 0,
                    cache_create_tokens: 0,
                    reasoning_tokens: 0,
                    cost_usd: 0.01,
                    wall_ms: 100,
                },
                tool_call_count: 0,
                gate_passed: Some(false), // always fails
                finish_reason: FinishReason::Stop,
            })
            .expect("send turn completed");
        }

        drop(tx);
        handle.await.expect("subscriber task");

        // Verify the router's confidence stats were adjusted.
        // Without calibration correction: 12 trials, 0 successes.
        // With calibration correction applied: extra synthetic trials injected.
        let snapshot = router.confidence_snapshot();
        let (trials, successes) = snapshot
            .get("overconfident-model")
            .expect("model should exist in snapshot");
        // The model had 12 trials with 0 successes from real data.
        // Calibration correction for overconfident model injects extra failures,
        // so trials > 12 and successes should still be 0.
        assert!(
            *trials > 12,
            "calibration correction should have injected synthetic trials (got {trials})"
        );
        assert_eq!(
            *successes, 0,
            "overconfident correction should not inject successes"
        );

        // Verify the router state was persisted to disk.
        assert!(
            router_persist_path.exists(),
            "cascade-router.json should have been written after calibration correction"
        );
        let persisted_content =
            std::fs::read_to_string(&router_persist_path).expect("read persisted router");
        assert!(
            persisted_content.contains("overconfident-model"),
            "persisted state should contain the corrected model"
        );
    }

    fn spawn_test_subscriber(
        rx: broadcast::Receiver<AgentEvent>,
        efficiency_path: std::path::PathBuf,
        dashboard_tx: Option<
            tokio::sync::mpsc::UnboundedSender<roko_core::dashboard_snapshot::DashboardEvent>,
        >,
    ) -> tokio::task::JoinHandle<()> {
        let latency = Arc::new(LatencyRegistry::new());
        let router = Arc::new(CascadeRouter::new(vec!["glm-5.1".to_string()]));
        let anomaly = Arc::new(Mutex::new(AnomalyDetector::new(1_700_000_000_000)));
        let costs = Arc::new(CostsDb::new());
        tokio::spawn(run_learning_subscriber(
            rx,
            latency,
            router,
            anomaly,
            costs,
            efficiency_path,
            None,
            dashboard_tx,
        ))
    }

    fn send_single_turn(
        tx: &broadcast::Sender<AgentEvent>,
        task_id: &str,
        gate_passed: Option<bool>,
    ) {
        tx.send(AgentEvent::TurnStarted {
            task_id: task_id.into(),
            model: "glm-5.1".into(),
            provider: "zai".into(),
            timestamp_ms: 1_700_000_000_000,
        })
        .expect("turn started");
        tx.send(AgentEvent::TurnCompleted {
            turn: 1,
            usage: Usage {
                input_tokens: 10,
                output_tokens: 5,
                cache_read_tokens: 0,
                cache_create_tokens: 0,
                reasoning_tokens: 0,
                cost_usd: 0.01,
                wall_ms: 100,
            },
            tool_call_count: 0,
            gate_passed,
            finish_reason: FinishReason::Stop,
        })
        .expect("turn completed");
    }

    #[tokio::test]
    async fn turn_with_inline_gate_result_is_marked_final() {
        let (tx, rx) = broadcast::channel(16);
        let tempdir = TempDir::new().expect("tempdir");
        let efficiency_path = tempdir.path().join("efficiency.jsonl");

        let handle = spawn_test_subscriber(rx, efficiency_path.clone(), None);
        send_single_turn(&tx, "task-inline", Some(true));
        drop(tx);
        handle.await.expect("subscriber task");

        let events = read_efficiency_events(&efficiency_path)
            .await
            .expect("read efficiency events");
        assert_eq!(events.len(), 1);
        assert!(
            events[0].is_final_turn,
            "a turn whose gate verdict is known inline concludes the attempt"
        );
        assert_eq!(events[0].gate_passed, Some(true));
    }

    #[tokio::test]
    async fn deferred_turn_is_marked_final_when_gate_result_arrives() {
        let (tx, rx) = broadcast::channel(16);
        let tempdir = TempDir::new().expect("tempdir");
        let efficiency_path = tempdir.path().join("efficiency.jsonl");

        let handle = spawn_test_subscriber(rx, efficiency_path.clone(), None);
        send_single_turn(&tx, "task-deferred", None);
        tx.send(AgentEvent::GateResult {
            gate_name: "test-gate".into(),
            passed: true,
            score: 1.0,
            duration_ms: 5,
            task_id: "task-deferred".into(),
        })
        .expect("gate result");
        drop(tx);
        handle.await.expect("subscriber task");

        let events = read_efficiency_events(&efficiency_path)
            .await
            .expect("read efficiency events");
        assert_eq!(events.len(), 1);
        assert!(
            events[0].is_final_turn,
            "the gate verdict concludes the attempt, so the flushed turn is final"
        );
        assert_eq!(events[0].gate_passed, Some(true));
        assert_eq!(events[0].outcome, "success");
    }

    #[tokio::test]
    async fn ungated_turn_flushed_on_shutdown_stays_non_final() {
        let (tx, rx) = broadcast::channel(16);
        let tempdir = TempDir::new().expect("tempdir");
        let efficiency_path = tempdir.path().join("efficiency.jsonl");

        let handle = spawn_test_subscriber(rx, efficiency_path.clone(), None);
        send_single_turn(&tx, "task-ungated", None);
        drop(tx);
        handle.await.expect("subscriber task");

        let events = read_efficiency_events(&efficiency_path)
            .await
            .expect("read efficiency events");
        assert_eq!(events.len(), 1);
        assert!(
            !events[0].is_final_turn,
            "no gate verdict ever arrived, so the record is not attempt-final"
        );
        assert_eq!(events[0].gate_passed, None);
    }

    #[tokio::test]
    async fn appended_events_push_trend_and_router_updates() {
        let (tx, rx) = broadcast::channel(16);
        let (dash_tx, mut dash_rx) = tokio::sync::mpsc::unbounded_channel();
        let tempdir = TempDir::new().expect("tempdir");
        let efficiency_path = tempdir.path().join("efficiency.jsonl");

        let handle = spawn_test_subscriber(rx, efficiency_path, Some(dash_tx));
        send_single_turn(&tx, "task-push", Some(true));
        drop(tx);
        handle.await.expect("subscriber task");

        let mut trend = None;
        let mut router_update = None;
        while let Ok(event) = dash_rx.try_recv() {
            match event {
                roko_core::dashboard_snapshot::DashboardEvent::EfficiencyTrendUpdated {
                    buckets,
                } => trend = Some(buckets),
                roko_core::dashboard_snapshot::DashboardEvent::CascadeRouterUpdated {
                    snapshot_json,
                } => router_update = Some(snapshot_json),
                other => panic!("unexpected dashboard event: {other:?}"),
            }
        }

        let trend = trend.expect("efficiency trend update pushed");
        assert_eq!(trend.len(), 24, "hourly buckets over the last 24 hours");
        let total_turns: u64 = trend.iter().map(|bucket| bucket.turns).sum();
        assert_eq!(total_turns, 1, "the appended turn is inside the 24h window");
        let tokens_in: u64 = trend.iter().map(|bucket| bucket.tokens_in).sum();
        assert_eq!(tokens_in, 10);

        let router_update = router_update.expect("cascade router update pushed");
        assert!(
            router_update.contains("glm-5.1"),
            "router snapshot should carry the observed model: {router_update}"
        );
    }
}
