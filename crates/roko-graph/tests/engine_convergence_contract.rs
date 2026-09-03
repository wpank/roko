//! Engine Convergence Contract Tests (Backlog #242)
//!
//! These tests verify the frozen golden schema for engine convergence fixtures.
//! Every fixture loads through both graph TOML loading and plan-to-graph
//! conversion. The `expected.json` schema is enforced via `deny_unknown_fields`.
//!
//! No test invokes a live provider, git operation, feedback sink, or
//! publication port. All Activity outputs are pre-recorded in `activities.jsonl`.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

// ─── Frozen Golden Schema Types ─────────────────────────────────────────────
//
// These types implement the exact `expected.json` schema from the contract
// document (docs/v2/31-ENGINE-CONVERGENCE-CONTRACT.md). Missing fields fail
// deserialization. Unknown fields are denied.

/// Top-level expected outcome for a convergence fixture.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ExpectedOutcome {
    schema_version: u32,
    fixture_id: String,
    graph_fingerprint: String,
    request_fingerprints: Vec<String>,
    prompt_fingerprints: Vec<String>,
    tasks: Vec<ExpectedTask>,
    events: Vec<ExpectedEvent>,
    receipts: Vec<ExpectedReceipt>,
    #[serde(rename = "final")]
    final_state: ExpectedFinal,
}

/// Per-task expected state in the golden output.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ExpectedTask {
    task_id: String,
    dependencies: Vec<String>,
    status: String,
    attempts: u32,
    provider: String,
    model: String,
    role: String,
    effort: String,
    workspace_fingerprint: String,
    input_tokens: u64,
    output_tokens: u64,
    cost_micro_usd: u64,
}

/// Normalized event in the golden output (timestamps removed, identity retained).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ExpectedEvent {
    sequence: u64,
    event_type: String,
    source: String,
    payload: serde_json::Value,
}

/// Receipt in the golden output (sorted by idempotency_key).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ExpectedReceipt {
    idempotency_key: String,
    owner: String,
    state: String,
    evidence_fingerprint: String,
}

/// Terminal plan state in the golden output.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ExpectedFinal {
    plan_status: String,
    completed_task_ids: Vec<String>,
    skipped_task_ids: Vec<String>,
    failed_task_ids: Vec<String>,
    total_input_tokens: u64,
    total_output_tokens: u64,
    total_cost_micro_usd: u64,
    merge_state: String,
    publication_state: String,
    terminal_event_id: String,
}

// ─── Fixture helpers ─────────────────────────────────────────────────────────

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("engine_convergence")
}

fn load_expected(fixture_name: &str) -> ExpectedOutcome {
    let path = fixtures_dir().join(fixture_name).join("expected.json");
    let content = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
    serde_json::from_str(&content)
        .unwrap_or_else(|e| panic!("expected.json parse failed for {fixture_name}: {e}"))
}

fn load_graph_toml(fixture_name: &str) -> roko_graph::Graph {
    let path = fixtures_dir().join(fixture_name).join("graph.toml");
    let content = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
    roko_graph::loader::load_from_str(&content)
        .unwrap_or_else(|e| panic!("graph.toml parse failed for {fixture_name}: {e}"))
}

/// Parse a fixture's tasks.toml into plan task info and convert to a Graph.
fn load_plan_as_graph(fixture_name: &str) -> roko_graph::Graph {
    let path = fixtures_dir().join(fixture_name).join("tasks.toml");
    let content = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));

    // Parse the TOML structure manually (we cannot use roko-cli's TasksFile
    // from this crate, so we extract the minimal plan task info).
    let raw: toml::Value = content.parse().unwrap();
    let meta = raw.get("meta").expect("tasks.toml must have [meta]");
    let plan_id = meta
        .get("plan")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");
    let max_parallel = meta
        .get("max_parallel")
        .and_then(|v| v.as_integer())
        .unwrap_or(1) as u32;

    let tasks_array = raw
        .get("task")
        .and_then(|v| v.as_array())
        .expect("tasks.toml must have [[task]]");

    let mut plan_tasks: Vec<(String, roko_graph::convert::PlanTaskInfo)> = Vec::new();
    for (seq, task_val) in tasks_array.iter().enumerate() {
        let table = task_val.as_table().expect("each task must be a table");
        let id = table["id"].as_str().unwrap().to_string();
        let title = table["title"].as_str().unwrap().to_string();
        let role = table.get("role").and_then(|v| v.as_str()).map(String::from);
        let tier = table
            .get("tier")
            .and_then(|v| v.as_str())
            .unwrap_or("mechanical")
            .to_string();
        let files: Vec<String> = table
            .get("files")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();
        let depends_on: Vec<String> = table
            .get("depends_on")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        let info = roko_graph::convert::PlanTaskInfo {
            title: title.clone(),
            description: None,
            role,
            tier,
            model_hint: None,
            files,
            depends_on,
            depends_on_plan: vec![],
            timeout_secs: 300,
            max_retries: 2,
            domain: None,
            sequence: seq,
            full_config_json: serde_json::json!({
                "id": &id,
                "title": &title,
            }),
        };
        plan_tasks.push((id, info));
    }

    roko_graph::convert::plan_to_graph(
        plan_id,
        &fixtures_dir().join(fixture_name).to_string_lossy(),
        &plan_tasks,
        max_parallel,
    )
    .unwrap_or_else(|e| panic!("plan_to_graph failed for {fixture_name}: {e}"))
}

// ─── Schema contract tests ──────────────────────────────────────────────────

#[test]
fn expected_schema_version_must_be_one() {
    for fixture in &["diamond_success", "gate_replan_cap", "cancel_resume_budget"] {
        let expected = load_expected(fixture);
        assert_eq!(
            expected.schema_version, 1,
            "fixture {fixture} must have schema_version = 1"
        );
    }
}

#[test]
fn expected_tasks_sorted_by_task_id() {
    for fixture in &["diamond_success", "gate_replan_cap", "cancel_resume_budget"] {
        let expected = load_expected(fixture);
        let ids: Vec<&str> = expected.tasks.iter().map(|t| t.task_id.as_str()).collect();
        let mut sorted = ids.clone();
        sorted.sort();
        assert_eq!(
            ids, sorted,
            "fixture {fixture}: tasks must be sorted by task_id"
        );
    }
}

#[test]
fn expected_receipts_sorted_by_idempotency_key() {
    for fixture in &["diamond_success", "gate_replan_cap", "cancel_resume_budget"] {
        let expected = load_expected(fixture);
        let keys: Vec<&str> = expected
            .receipts
            .iter()
            .map(|r| r.idempotency_key.as_str())
            .collect();
        let mut sorted = keys.clone();
        sorted.sort();
        assert_eq!(
            keys, sorted,
            "fixture {fixture}: receipts must be sorted by idempotency_key"
        );
    }
}

#[test]
fn expected_events_sequential() {
    for fixture in &["diamond_success", "gate_replan_cap", "cancel_resume_budget"] {
        let expected = load_expected(fixture);
        for (i, event) in expected.events.iter().enumerate() {
            assert_eq!(
                event.sequence, i as u64,
                "fixture {fixture}: event at index {i} has sequence {}, expected {i}",
                event.sequence
            );
        }
    }
}

#[test]
fn expected_final_completed_tasks_consistent_with_task_list() {
    for fixture in &["diamond_success", "gate_replan_cap", "cancel_resume_budget"] {
        let expected = load_expected(fixture);
        for task_id in &expected.final_state.completed_task_ids {
            let task = expected
                .tasks
                .iter()
                .find(|t| &t.task_id == task_id)
                .unwrap_or_else(|| {
                    panic!("fixture {fixture}: completed_task_id '{task_id}' not in tasks list")
                });
            assert_eq!(
                task.status, "completed",
                "fixture {fixture}: task '{task_id}' listed as completed in final but has status '{}'",
                task.status
            );
        }
    }
}

#[test]
fn expected_missing_field_fails_deserialization() {
    // Verify that removing a required field causes serde to reject the JSON.
    let json = serde_json::json!({
        "schema_version": 1,
        "fixture_id": "test",
        "graph_fingerprint": "abc",
        "request_fingerprints": [],
        "prompt_fingerprints": [],
        "tasks": [],
        "events": [],
        "receipts": []
        // "final" is missing
    });
    let result = serde_json::from_value::<ExpectedOutcome>(json);
    assert!(
        result.is_err(),
        "missing 'final' field must cause deserialization failure"
    );
}

#[test]
fn expected_unknown_field_denied() {
    // Verify that an unknown field at the top level is rejected.
    let json = serde_json::json!({
        "schema_version": 1,
        "fixture_id": "test",
        "graph_fingerprint": "abc",
        "request_fingerprints": [],
        "prompt_fingerprints": [],
        "tasks": [],
        "events": [],
        "receipts": [],
        "final": {
            "plan_status": "completed",
            "completed_task_ids": [],
            "skipped_task_ids": [],
            "failed_task_ids": [],
            "total_input_tokens": 0,
            "total_output_tokens": 0,
            "total_cost_micro_usd": 0,
            "merge_state": "not_attempted",
            "publication_state": "not_attempted",
            "terminal_event_id": "x"
        },
        "unexpected_extra_field": true
    });
    let result = serde_json::from_value::<ExpectedOutcome>(json);
    assert!(
        result.is_err(),
        "unknown field 'unexpected_extra_field' must be denied"
    );
}

#[test]
fn expected_unknown_field_in_task_denied() {
    let json = serde_json::json!({
        "task_id": "X",
        "dependencies": [],
        "status": "completed",
        "attempts": 1,
        "provider": "p",
        "model": "m",
        "role": "implementer",
        "effort": "mechanical",
        "workspace_fingerprint": "ws",
        "input_tokens": 0,
        "output_tokens": 0,
        "cost_micro_usd": 0,
        "bonus_field": "surprise"
    });
    let result = serde_json::from_value::<ExpectedTask>(json);
    assert!(
        result.is_err(),
        "unknown field 'bonus_field' in task must be denied"
    );
}

#[test]
fn expected_unknown_field_in_final_denied() {
    let json = serde_json::json!({
        "plan_status": "completed",
        "completed_task_ids": [],
        "skipped_task_ids": [],
        "failed_task_ids": [],
        "total_input_tokens": 0,
        "total_output_tokens": 0,
        "total_cost_micro_usd": 0,
        "merge_state": "not_attempted",
        "publication_state": "not_attempted",
        "terminal_event_id": "x",
        "extra": 42
    });
    let result = serde_json::from_value::<ExpectedFinal>(json);
    assert!(
        result.is_err(),
        "unknown field 'extra' in final must be denied"
    );
}

// ─── Graph loading tests ────────────────────────────────────────────────────

#[test]
fn diamond_success_graph_toml_loads() {
    let graph = load_graph_toml("diamond_success");
    assert_eq!(graph.metadata.name, "diamond-success");
    assert_eq!(graph.node_count(), 4);
    assert_eq!(graph.edge_count(), 4);

    // Verify DAG structure: A -> {B, C} -> D
    assert!(graph.get_node("A").is_some());
    assert!(graph.get_node("B").is_some());
    assert!(graph.get_node("C").is_some());
    assert!(graph.get_node("D").is_some());
}

#[test]
fn gate_replan_cap_graph_toml_loads() {
    let graph = load_graph_toml("gate_replan_cap");
    assert_eq!(graph.metadata.name, "gate-replan-cap");
    assert_eq!(graph.node_count(), 2);
    assert_eq!(graph.edge_count(), 1);
}

#[test]
fn cancel_resume_budget_graph_toml_loads() {
    let graph = load_graph_toml("cancel_resume_budget");
    assert_eq!(graph.metadata.name, "cancel-resume-budget");
    assert_eq!(graph.node_count(), 3);
    assert_eq!(graph.edge_count(), 2);
}

// ─── Plan conversion tests ─────────────────────────────────────────────────

#[test]
fn diamond_success_plan_converts_to_graph() {
    let graph = load_plan_as_graph("diamond_success");
    assert_eq!(graph.metadata.name, "diamond-success");
    assert_eq!(graph.node_count(), 4);
    assert_eq!(graph.edge_count(), 4);

    // All nodes should be task-executor cells.
    for node_id in &["A", "B", "C", "D"] {
        let node = graph.get_node(node_id).unwrap();
        assert_eq!(node.cell_type, "task-executor");
        assert_eq!(
            node.execution_class,
            roko_graph::ExecutionClass::Activity
        );
    }
}

#[test]
fn gate_replan_cap_plan_converts_to_graph() {
    let graph = load_plan_as_graph("gate_replan_cap");
    assert_eq!(graph.metadata.name, "gate-replan-cap");
    assert_eq!(graph.node_count(), 2);
    assert_eq!(graph.edge_count(), 1);
}

#[test]
fn cancel_resume_budget_plan_converts_to_graph() {
    let graph = load_plan_as_graph("cancel_resume_budget");
    assert_eq!(graph.metadata.name, "cancel-resume-budget");
    assert_eq!(graph.node_count(), 3);
    assert_eq!(graph.edge_count(), 2);
}

// ─── Cross-loading parity tests ─────────────────────────────────────────────
//
// Both graph.toml loading and plan conversion must produce structurally
// equivalent graphs (same node count, edge count, and topology).

#[test]
fn diamond_success_plan_and_graph_structurally_equivalent() {
    let from_graph = load_graph_toml("diamond_success");
    let from_plan = load_plan_as_graph("diamond_success");

    assert_eq!(from_graph.node_count(), from_plan.node_count());
    assert_eq!(from_graph.edge_count(), from_plan.edge_count());

    // Same node IDs in both.
    for node_id in &["A", "B", "C", "D"] {
        assert!(
            from_graph.get_node(node_id).is_some(),
            "graph.toml missing node {node_id}"
        );
        assert!(
            from_plan.get_node(node_id).is_some(),
            "plan conversion missing node {node_id}"
        );
    }
}

#[test]
fn gate_replan_cap_plan_and_graph_structurally_equivalent() {
    let from_graph = load_graph_toml("gate_replan_cap");
    let from_plan = load_plan_as_graph("gate_replan_cap");

    assert_eq!(from_graph.node_count(), from_plan.node_count());
    assert_eq!(from_graph.edge_count(), from_plan.edge_count());
}

#[test]
fn cancel_resume_budget_plan_and_graph_structurally_equivalent() {
    let from_graph = load_graph_toml("cancel_resume_budget");
    let from_plan = load_plan_as_graph("cancel_resume_budget");

    assert_eq!(from_graph.node_count(), from_plan.node_count());
    assert_eq!(from_graph.edge_count(), from_plan.edge_count());
}

// ─── Expected outcome consistency tests ─────────────────────────────────────

#[test]
fn diamond_success_expected_outcome_consistent() {
    let expected = load_expected("diamond_success");
    assert_eq!(expected.fixture_id, "diamond_success");
    assert_eq!(expected.tasks.len(), 4);
    assert_eq!(expected.final_state.plan_status, "completed");
    assert_eq!(expected.final_state.completed_task_ids.len(), 4);
    assert!(expected.final_state.failed_task_ids.is_empty());
    assert!(expected.final_state.skipped_task_ids.is_empty());
    assert_eq!(expected.final_state.merge_state, "not_attempted");

    // Token totals must equal sum of per-task tokens.
    let total_in: u64 = expected.tasks.iter().map(|t| t.input_tokens).sum();
    let total_out: u64 = expected.tasks.iter().map(|t| t.output_tokens).sum();
    let total_cost: u64 = expected.tasks.iter().map(|t| t.cost_micro_usd).sum();
    assert_eq!(expected.final_state.total_input_tokens, total_in);
    assert_eq!(expected.final_state.total_output_tokens, total_out);
    assert_eq!(expected.final_state.total_cost_micro_usd, total_cost);
}

#[test]
fn gate_replan_cap_expected_outcome_consistent() {
    let expected = load_expected("gate_replan_cap");
    assert_eq!(expected.fixture_id, "gate_replan_cap");
    assert_eq!(expected.tasks.len(), 2);
    assert_eq!(expected.final_state.plan_status, "failed");
    assert_eq!(expected.final_state.merge_state, "not_attempted");

    // Both gate receipts should be in failed state.
    assert_eq!(expected.receipts.len(), 2);
    for receipt in &expected.receipts {
        assert_eq!(receipt.state, "failed");
        assert_eq!(receipt.owner, "gate-pipeline");
    }
}

#[test]
fn cancel_resume_budget_expected_outcome_consistent() {
    let expected = load_expected("cancel_resume_budget");
    assert_eq!(expected.fixture_id, "cancel_resume_budget");
    assert_eq!(expected.tasks.len(), 3);
    assert_eq!(expected.final_state.plan_status, "failed");

    // A completed, B failed (cancelled), C skipped.
    assert_eq!(expected.final_state.completed_task_ids, vec!["A"]);
    assert_eq!(expected.final_state.failed_task_ids, vec!["B"]);
    assert_eq!(expected.final_state.skipped_task_ids, vec!["C"]);

    // Task C has zero tokens (never dispatched).
    let task_c = expected
        .tasks
        .iter()
        .find(|t| t.task_id == "C")
        .expect("task C must exist");
    assert_eq!(task_c.attempts, 0);
    assert_eq!(task_c.input_tokens, 0);
    assert_eq!(task_c.output_tokens, 0);
}

// ─── Graph engine validation tests ──────────────────────────────────────────

#[test]
fn diamond_success_graph_validates_in_engine() {
    let graph = load_graph_toml("diamond_success");
    let registry = roko_graph::default_registry();
    let engine = roko_graph::GraphEngine::new(graph, registry);
    let issues = engine.validate();
    // task-executor may not be in default_registry, so we check for
    // specific unexpected issues rather than empty.
    for issue in &issues {
        // UnknownCellType for "task-executor" is expected in graph-only tests
        // (the cell is registered by the CLI runner, not the graph crate).
        assert!(
            issue.contains("task-executor"),
            "unexpected validation issue: {issue}"
        );
    }
}

#[test]
fn diamond_success_graph_validates_with_task_executor_cell() {
    let graph = load_graph_toml("diamond_success");
    let mut registry = roko_graph::default_registry();
    registry.register("task-executor", |config| {
        Box::new(roko_graph::cells::TaskExecutorCell::dry_run(config))
    });
    let engine = roko_graph::GraphEngine::new(graph, registry);
    let issues = engine.validate();
    assert!(
        issues.is_empty(),
        "diamond_success should validate cleanly with task-executor: {issues:?}"
    );
}

// ─── Activity recording parity test ─────────────────────────────────────────

#[test]
fn activities_jsonl_loads_for_all_fixtures() {
    for fixture in &["diamond_success", "gate_replan_cap", "cancel_resume_budget"] {
        let path = fixtures_dir().join(fixture).join("activities.jsonl");
        assert!(
            path.exists(),
            "activities.jsonl must exist for fixture {fixture}"
        );

        // Each line must be valid JSON (even if the full RecordEntry schema
        // requires roko-core Signal types that may not roundtrip perfectly
        // through fixture JSON, the lines must at least parse as JSON).
        let content = std::fs::read_to_string(&path).unwrap();
        for (i, line) in content.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            let parsed: Result<serde_json::Value, _> = serde_json::from_str(trimmed);
            assert!(
                parsed.is_ok(),
                "fixture {fixture}: activities.jsonl line {i} is not valid JSON: {}",
                parsed.unwrap_err()
            );
        }
    }
}

// ─── Fixture inventory completeness ─────────────────────────────────────────

#[test]
fn all_fixture_directories_contain_required_files() {
    let required_files = ["tasks.toml", "graph.toml", "expected.json", "activities.jsonl"];

    for fixture in &["diamond_success", "gate_replan_cap", "cancel_resume_budget"] {
        let dir = fixtures_dir().join(fixture);
        assert!(dir.is_dir(), "fixture directory {fixture} must exist");

        for file in &required_files {
            let path = dir.join(file);
            assert!(
                path.exists(),
                "fixture {fixture} missing required file: {file}"
            );
        }
    }
}

// ─── Graph fingerprint stability ────────────────────────────────────────────

#[test]
fn graph_fingerprint_is_deterministic_for_fixture() {
    for fixture in &["diamond_success", "gate_replan_cap", "cancel_resume_budget"] {
        let graph = load_graph_toml(fixture);
        let fp1 = roko_graph::graph_execution_fingerprint(&graph)
            .expect("fingerprint must succeed");
        let fp2 = roko_graph::graph_execution_fingerprint(&graph)
            .expect("fingerprint must succeed");
        assert_eq!(
            fp1, fp2,
            "fixture {fixture}: fingerprint must be deterministic"
        );
        assert!(
            !fp1.is_empty(),
            "fixture {fixture}: fingerprint must not be empty"
        );
    }
}
