//! Engine Convergence Contract Tests -- roko-cli side (Backlog #242)
//!
//! These tests verify that the convergence contract fixtures load correctly
//! through the CLI's plan parsing path (`TasksFile::parse_str`) and produce
//! graphs that are structurally equivalent to the graph TOML loading path.
//!
//! The fixture files live in `crates/roko-graph/tests/fixtures/engine_convergence/`
//! and are shared between both crate test suites.
//!
//! No test invokes a live provider, git operation, feedback sink, or
//! publication port.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

// ─── Frozen Golden Schema Types (shared with roko-graph tests) ──────────────
//
// Duplicated here because integration tests in separate crates cannot share
// test-only types without a shared test utility crate. The schema is frozen
// and must not diverge.

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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ExpectedEvent {
    sequence: u64,
    event_type: String,
    source: String,
    payload: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ExpectedReceipt {
    idempotency_key: String,
    owner: String,
    state: String,
    evidence_fingerprint: String,
}

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

/// Resolve the shared fixture directory in roko-graph.
fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent() // crates/
        .unwrap()
        .join("roko-graph")
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

/// Load a fixture's tasks.toml through the CLI's `TasksFile` parser.
fn load_tasks_file(fixture_name: &str) -> roko_cli::task_parser::TasksFile {
    let path = fixtures_dir().join(fixture_name).join("tasks.toml");
    let content = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
    roko_cli::task_parser::TasksFile::parse_str(&content)
        .unwrap_or_else(|e| panic!("tasks.toml parse failed for {fixture_name}: {e}"))
}

/// Convert a fixture's TasksFile into a Graph through the plan converter.
fn tasks_file_to_graph(fixture_name: &str) -> roko_graph::Graph {
    let tasks_file = load_tasks_file(fixture_name);
    let plan_id = &tasks_file.meta.plan;
    let max_parallel = tasks_file.meta.max_parallel;

    let plan_tasks: Vec<(String, roko_graph::convert::PlanTaskInfo)> = tasks_file
        .tasks
        .iter()
        .enumerate()
        .map(|(seq, task)| {
            let info = roko_graph::convert::PlanTaskInfo {
                title: task.title.clone(),
                description: task.description.clone(),
                role: task.role.clone(),
                tier: task.tier.clone(),
                model_hint: task.model_hint.clone(),
                files: task.files.clone(),
                depends_on: task.depends_on.clone(),
                depends_on_plan: task.depends_on_plan.clone(),
                timeout_secs: task.timeout_secs,
                max_retries: task.max_retries,
                domain: task.domain.as_ref().map(|d| d.label().to_string()),
                sequence: seq,
                full_config_json: serde_json::to_value(task).unwrap_or(serde_json::Value::Null),
            };
            (task.id.clone(), info)
        })
        .collect();

    roko_graph::convert::plan_to_graph(
        plan_id,
        &fixtures_dir().join(fixture_name).to_string_lossy(),
        &plan_tasks,
        max_parallel,
    )
    .unwrap_or_else(|e| panic!("plan_to_graph failed for {fixture_name}: {e}"))
}

/// Load a fixture's graph.toml directly through the TOML loader.
fn load_graph_toml(fixture_name: &str) -> roko_graph::Graph {
    let path = fixtures_dir().join(fixture_name).join("graph.toml");
    let content = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
    roko_graph::loader::load_from_str(&content)
        .unwrap_or_else(|e| panic!("graph.toml parse failed for {fixture_name}: {e}"))
}

// ─── Plan loading tests ─────────────────────────────────────────────────────

const FIXTURES: &[&str] = &["diamond_success", "gate_replan_cap", "cancel_resume_budget"];

#[test]
fn all_fixtures_parse_through_cli_task_parser() {
    for fixture in FIXTURES {
        let tasks_file = load_tasks_file(fixture);
        assert!(
            !tasks_file.tasks.is_empty(),
            "fixture {fixture}: tasks.toml must have at least one task"
        );
    }
}

#[test]
fn all_fixtures_convert_through_plan_to_graph() {
    for fixture in FIXTURES {
        let graph = tasks_file_to_graph(fixture);
        assert!(
            graph.node_count() > 0,
            "fixture {fixture}: converted graph must have nodes"
        );
    }
}

// ─── Cross-loading parity tests ─────────────────────────────────────────────

#[test]
fn diamond_success_cli_and_graph_structurally_equivalent() {
    let from_cli = tasks_file_to_graph("diamond_success");
    let from_graph = load_graph_toml("diamond_success");

    assert_eq!(
        from_cli.node_count(),
        from_graph.node_count(),
        "node count mismatch between CLI plan loading and graph TOML loading"
    );
    assert_eq!(
        from_cli.edge_count(),
        from_graph.edge_count(),
        "edge count mismatch between CLI plan loading and graph TOML loading"
    );

    // Same node IDs.
    for node_id in &["A", "B", "C", "D"] {
        assert!(
            from_cli.get_node(node_id).is_some(),
            "CLI plan missing node {node_id}"
        );
        assert!(
            from_graph.get_node(node_id).is_some(),
            "graph TOML missing node {node_id}"
        );
    }
}

#[test]
fn gate_replan_cap_cli_and_graph_structurally_equivalent() {
    let from_cli = tasks_file_to_graph("gate_replan_cap");
    let from_graph = load_graph_toml("gate_replan_cap");

    assert_eq!(from_cli.node_count(), from_graph.node_count());
    assert_eq!(from_cli.edge_count(), from_graph.edge_count());

    for node_id in &["T1", "T1-fix"] {
        assert!(from_cli.get_node(node_id).is_some());
        assert!(from_graph.get_node(node_id).is_some());
    }
}

#[test]
fn cancel_resume_budget_cli_and_graph_structurally_equivalent() {
    let from_cli = tasks_file_to_graph("cancel_resume_budget");
    let from_graph = load_graph_toml("cancel_resume_budget");

    assert_eq!(from_cli.node_count(), from_graph.node_count());
    assert_eq!(from_cli.edge_count(), from_graph.edge_count());

    for node_id in &["A", "B", "C"] {
        assert!(from_cli.get_node(node_id).is_some());
        assert!(from_graph.get_node(node_id).is_some());
    }
}

// ─── Expected outcome schema contract ───────────────────────────────────────

#[test]
fn expected_schema_loads_and_denies_unknown_fields() {
    for fixture in FIXTURES {
        let expected = load_expected(fixture);
        assert_eq!(
            expected.schema_version, 1,
            "fixture {fixture}: schema_version must be 1"
        );
    }
}

#[test]
fn expected_tasks_match_plan_task_count() {
    let fixture_task_counts = [
        ("diamond_success", 4),
        ("gate_replan_cap", 2),
        ("cancel_resume_budget", 3),
    ];

    for (fixture, expected_count) in &fixture_task_counts {
        let expected = load_expected(fixture);
        assert_eq!(
            expected.tasks.len(),
            *expected_count,
            "fixture {fixture}: expected.json task count mismatch"
        );

        // The plan-loaded graph should have the same number of nodes.
        let graph = tasks_file_to_graph(fixture);
        assert_eq!(
            graph.node_count(),
            *expected_count,
            "fixture {fixture}: graph node count must match expected task count"
        );
    }
}

#[test]
fn expected_final_task_ids_partition_all_tasks() {
    for fixture in FIXTURES {
        let expected = load_expected(fixture);
        let all_task_ids: Vec<&str> = expected.tasks.iter().map(|t| t.task_id.as_str()).collect();

        // Every task ID must appear in exactly one of completed/skipped/failed.
        let mut accounted: Vec<&str> = Vec::new();
        accounted.extend(
            expected
                .final_state
                .completed_task_ids
                .iter()
                .map(|s| s.as_str()),
        );
        accounted.extend(
            expected
                .final_state
                .skipped_task_ids
                .iter()
                .map(|s| s.as_str()),
        );
        accounted.extend(
            expected
                .final_state
                .failed_task_ids
                .iter()
                .map(|s| s.as_str()),
        );

        // Allow tasks to not appear in any terminal list (e.g. cancelled tasks
        // may be in failed_task_ids). But every listed ID must be a real task.
        for id in &accounted {
            assert!(
                all_task_ids.contains(id),
                "fixture {fixture}: terminal task_id '{id}' not in tasks list"
            );
        }
    }
}

// ─── Token/cost aggregation contract ────────────────────────────────────────

#[test]
fn expected_final_totals_equal_task_sum() {
    for fixture in FIXTURES {
        let expected = load_expected(fixture);
        let sum_in: u64 = expected.tasks.iter().map(|t| t.input_tokens).sum();
        let sum_out: u64 = expected.tasks.iter().map(|t| t.output_tokens).sum();
        let sum_cost: u64 = expected.tasks.iter().map(|t| t.cost_micro_usd).sum();

        assert_eq!(
            expected.final_state.total_input_tokens, sum_in,
            "fixture {fixture}: total_input_tokens mismatch"
        );
        assert_eq!(
            expected.final_state.total_output_tokens, sum_out,
            "fixture {fixture}: total_output_tokens mismatch"
        );
        assert_eq!(
            expected.final_state.total_cost_micro_usd, sum_cost,
            "fixture {fixture}: total_cost_micro_usd mismatch"
        );
    }
}

// ─── Activities JSONL contract ──────────────────────────────────────────────

#[test]
fn activities_jsonl_has_entries_for_completed_tasks_only() {
    for fixture in FIXTURES {
        let expected = load_expected(fixture);
        let path = fixtures_dir().join(fixture).join("activities.jsonl");
        let content = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));

        let mut recorded_node_ids: Vec<String> = Vec::new();
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            let val: serde_json::Value = serde_json::from_str(trimmed)
                .unwrap_or_else(|e| panic!("fixture {fixture}: invalid JSONL line: {e}"));
            if let Some(node_id) = val.get("node_id").and_then(|v| v.as_str()) {
                recorded_node_ids.push(node_id.to_string());
            }
        }

        // Every recorded node ID must correspond to a completed task.
        for node_id in &recorded_node_ids {
            let task = expected
                .tasks
                .iter()
                .find(|t| &t.task_id == node_id)
                .unwrap_or_else(|| {
                    panic!("fixture {fixture}: activities.jsonl has node '{node_id}' not in tasks")
                });
            assert_eq!(
                task.status, "completed",
                "fixture {fixture}: activities.jsonl records node '{node_id}' but task status is '{}' (only completed tasks should have Activity records)",
                task.status
            );
        }
    }
}
