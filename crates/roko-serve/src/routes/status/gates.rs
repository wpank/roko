//! Verify summary and history endpoints.

use std::collections::BTreeMap;
use std::sync::Arc;

use axum::Json;
use axum::extract::{Path, Query, State};
use roko_core::dashboard_snapshot::DashboardEvent;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::error::ApiError;
use crate::projection_contract::{ProjectionQuery, RuntimeProjectionSet};
use crate::state::AppState;

use super::helpers::{
    MAX_JSONL_RESULTS, extract_gate_duration_ms, extract_gate_name, extract_gate_passed,
    is_gate_result_kind, read_jsonl_entries,
};

/// `GET /api/gates/summary` — aggregate gate verdicts from canonical projections.
pub async fn gate_summary(State(state): State<Arc<AppState>>) -> Result<Json<Value>, ApiError> {
    let projections = RuntimeProjectionSet::load(&state).await?;
    Ok(Json(projections.gate_summary(&ProjectionQuery::default())))
}

#[derive(Debug, Deserialize, Default)]
pub struct GateHistoryQuery {
    #[serde(default)]
    gate: Option<String>,
    #[serde(default)]
    limit: Option<usize>,
    /// Pass `format=waterfall` to get items grouped by task_id with nested
    /// rungs — the shape expected by the demo `GateWaterfall` component.
    #[serde(default)]
    format: Option<String>,
}

/// `GET /api/gates/history` — recent gate verdicts across all gates.
///
/// When called with `?format=waterfall`, items are grouped by task_id into
/// `GateRun` objects with nested `rungs` — the shape the demo GateWaterfall
/// component expects.
pub async fn gates_history(
    State(state): State<Arc<AppState>>,
    Query(query): Query<GateHistoryQuery>,
) -> Result<Json<Value>, ApiError> {
    if query.format.as_deref() == Some("waterfall") {
        return gates_history_waterfall(&state, query.limit).await;
    }
    let projections = RuntimeProjectionSet::load(&state).await?;
    let query = ProjectionQuery {
        gate: query.gate,
        limit: query.limit.map(|limit| limit.min(MAX_JSONL_RESULTS)),
        ..ProjectionQuery::default()
    };
    Ok(Json(projections.gate_history(&query)))
}

/// `GET /api/gates/:gate_name/history` — time series of pass/fail results for one gate.
pub async fn gate_history(
    State(state): State<Arc<AppState>>,
    Path(gate_name): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let projections = RuntimeProjectionSet::load(&state).await?;
    let query = ProjectionQuery {
        gate: Some(gate_name.clone()),
        limit: Some(MAX_JSONL_RESULTS),
        ..ProjectionQuery::default()
    };
    let history = projections.gate_history(&query);
    let total = history.get("total").and_then(Value::as_u64).unwrap_or(0);
    if total == 0 {
        return Err(ApiError::not_found(format!("gate '{gate_name}' not found")));
    }

    Ok(Json(history))
}

// ── private helpers ──────────────────────────────────────────────────

/// The gate history's raw entries: gate signals in `.roko/signals.jsonl`,
/// then the `DashboardEvent::GateResult` lines Graph runs write to
/// `.roko/events.jsonl` for each verify step, newest first (backlog 2119).
async fn read_gate_entries(workdir: &std::path::Path) -> Result<Vec<Value>, ApiError> {
    let roko = workdir.join(".roko");
    let mut entries = read_jsonl_entries(&roko.join("signals.jsonl")).await?;
    let events = read_jsonl_entries(&roko.join("events.jsonl")).await?;
    entries.extend(events.iter().rev().filter_map(graph_gate_entry));
    Ok(entries)
}

/// The gate history entry of a `DashboardEvent::GateResult` line. The line
/// carries no time, so neither does the entry.
fn graph_gate_entry(event: &Value) -> Option<Value> {
    if event.get("type").and_then(Value::as_str) != Some("gate_result") {
        return None;
    }
    let DashboardEvent::GateResult {
        plan_id,
        task_id,
        gate,
        passed,
        ..
    } = serde_json::from_value(event.clone()).ok()?
    else {
        return None;
    };
    Some(json!({
        "id": Value::Null,
        "created_at_ms": Value::Null,
        "kind": "gate_verdict",
        "tags": {
            "gate": gate,
            "passed": passed.to_string(),
            "plan_id": plan_id,
            "task_id": task_id,
        },
        "body": {
            "data": {
                "gate": gate,
                "passed": passed,
                "plan_id": plan_id,
                "task_id": task_id,
            }
        }
    }))
}

fn build_recent_gate_history(entries: &[Value], gate_filter: Option<&str>) -> Vec<Value> {
    let mut history: Vec<Value> = entries
        .iter()
        .filter(|entry| {
            let Some(kind) = entry.get("kind").and_then(Value::as_str) else {
                return false;
            };
            if !is_gate_result_kind(kind) {
                return false;
            }
            match gate_filter {
                Some(gate) => extract_gate_name(entry).as_deref() == Some(gate),
                None => true,
            }
        })
        .filter_map(|entry| {
            let gate = extract_gate_name(entry)?;
            let passed = extract_gate_passed(entry)?;
            Some(json!({
                "signal_id": entry.get("id").cloned().unwrap_or(Value::Null),
                "created_at_ms": entry.get("created_at_ms").cloned().unwrap_or(Value::Null),
                "gate": gate,
                "passed": passed,
                "duration_ms": extract_gate_duration_ms(entry).unwrap_or(0),
                "plan_id": entry.pointer("/tags/plan_id")
                    .cloned()
                    .or_else(|| entry.pointer("/body/data/plan_id").cloned())
                    .unwrap_or(Value::Null),
                "task_id": entry.pointer("/tags/task_id")
                    .cloned()
                    .or_else(|| entry.pointer("/body/data/task_id").cloned())
                    .unwrap_or(Value::Null),
                "rung": entry.pointer("/tags/rung")
                    .cloned()
                    .or_else(|| entry.pointer("/body/data/rung").cloned())
                    .unwrap_or(Value::Null),
                "kind": entry.get("kind").cloned().unwrap_or(Value::Null),
            }))
        })
        .collect();

    history.sort_by(|a, b| {
        let a_ts = a
            .get("created_at_ms")
            .and_then(Value::as_i64)
            .unwrap_or(i64::MIN);
        let b_ts = b
            .get("created_at_ms")
            .and_then(Value::as_i64)
            .unwrap_or(i64::MIN);
        b_ts.cmp(&a_ts).then_with(|| {
            let a_id = a.get("signal_id").and_then(Value::as_str).unwrap_or("");
            let b_id = b.get("signal_id").and_then(Value::as_str).unwrap_or("");
            b_id.cmp(a_id)
        })
    });

    history
}

// ── waterfall format ─────────────────────────────────────────────────

/// Map numeric rung IDs to the names the frontend expects.
pub fn rung_id_to_name(rung: u32) -> &'static str {
    match rung {
        0 => "compile",
        1 => "clippy",
        2 => "test",
        3 => "diff",
        4 => "fmt",
        5 => "custom",
        6 => "judge",
        _ => "unknown",
    }
}

/// `?format=waterfall` — group gate history items by task_id into `GateRun`
/// objects with nested `GateRung` arrays.  This is the shape the demo
/// `GateWaterfall` component expects:
///
/// ```json
/// [
///   {
///     "task_id": "...",
///     "timestamp": "...",
///     "rungs": [
///       { "name": "compile", "rung": 0, "status": "passed", "duration_ms": 123 }
///     ]
///   }
/// ]
/// ```
async fn gates_history_waterfall(
    state: &AppState,
    limit: Option<usize>,
) -> Result<Json<Value>, ApiError> {
    let entries = read_gate_entries(&state.workdir).await?;
    let flat = build_recent_gate_history(&entries, None);

    // Group by task_id.
    let mut by_task: BTreeMap<String, Vec<Value>> = BTreeMap::new();
    for item in &flat {
        let task_id = item
            .get("task_id")
            .and_then(Value::as_str)
            .unwrap_or("unknown")
            .to_string();
        by_task.entry(task_id).or_default().push(item.clone());
    }

    let limit = limit.unwrap_or(20).min(MAX_JSONL_RESULTS);
    let runs: Vec<Value> = by_task
        .into_iter()
        .rev()
        .take(limit)
        .map(|(task_id, items)| {
            let timestamp = items
                .first()
                .and_then(|i| i.get("created_at_ms"))
                .cloned()
                .unwrap_or(Value::Null);

            let rungs: Vec<Value> = items
                .iter()
                .map(|item| {
                    let rung_num = item.get("rung").and_then(Value::as_u64).unwrap_or(0) as u32;
                    let passed = item.get("passed").and_then(Value::as_bool).unwrap_or(false);
                    let duration_ms = item.get("duration_ms").and_then(Value::as_u64).unwrap_or(0);
                    let name = item
                        .get("gate")
                        .and_then(Value::as_str)
                        .unwrap_or_else(|| rung_id_to_name(rung_num));

                    json!({
                        "name": name,
                        "rung": rung_num,
                        "status": if passed { "passed" } else { "failed" },
                        "duration_ms": duration_ms,
                    })
                })
                .collect();

            json!({
                "task_id": task_id,
                "timestamp": timestamp,
                "rungs": rungs,
            })
        })
        .collect();

    Ok(Json(json!(runs)))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// backlog 2119: the gate history reads the `GateResult` lines Graph
    /// runs write to `.roko/events.jsonl`, newest first, and skips the
    /// run's other events and Runner-v2's `gate.completed` lines.
    #[tokio::test]
    async fn gate_history_reads_graph_gate_results() {
        let dir = tempfile::tempdir().expect("tempdir");
        let roko = dir.path().join(".roko");
        std::fs::create_dir_all(&roko).expect("create .roko");
        let gate_result = |task_id: &str, gate: &str, passed: bool| {
            let event = DashboardEvent::GateResult {
                plan_id: "plan-a".to_string(),
                task_id: task_id.to_string(),
                gate: gate.to_string(),
                passed,
                output_text: Some("ok".to_string()),
            };
            serde_json::to_string(&event).expect("serialize the event")
        };
        let runner_v2 = json!({
            "type": "gate.completed",
            "plan_id": "plan-a",
            "task_id": "T0",
            "verdicts": [{"gate": "compile", "passed": true}],
        });
        let lines = [
            r#"{"type":"plan_started","plan_id":"plan-a","tasks_total":2}"#.to_string(),
            runner_v2.to_string(),
            gate_result("T1", "compile", true),
            gate_result("T2", "test", false),
        ];
        std::fs::write(roko.join("events.jsonl"), lines.join("\n") + "\n").expect("write");

        let entries = read_gate_entries(dir.path()).await.expect("read");
        let history = build_recent_gate_history(&entries, None);

        assert_eq!(history.len(), 2, "{history:?}");
        let passed = history
            .iter()
            .filter(|item| item["passed"] == true)
            .count();
        assert_eq!((passed, history.len() - passed), (1, 1));
        assert_eq!(history[0]["gate"], "test", "the latest line comes first");
        assert_eq!(history[0]["task_id"], "T2");
        assert_eq!(history[0]["plan_id"], "plan-a");
        assert_eq!(history[0]["passed"], false);
        assert_eq!(history[1]["gate"], "compile");
        assert_eq!(history[1]["task_id"], "T1");
        let test_only = build_recent_gate_history(&entries, Some("test"));
        assert_eq!(test_only.len(), 1);
    }
}
