//! Integration tests for example graph parsing.

use roko_graph::{EdgeCondition, FailureStrategy, GraphMode};

/// Helper: resolve the workspace-root `examples/graphs/` directory.
fn examples_dir() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent() // crates/
        .unwrap()
        .parent() // workspace root
        .unwrap()
        .join("examples")
        .join("graphs")
}

/// Helper: load an example graph by filename.
fn load_example(filename: &str) -> roko_graph::Graph {
    let path = examples_dir().join(filename);
    let content = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
    roko_graph::loader::load_from_str(&content)
        .unwrap_or_else(|e| panic!("{} failed to parse: {e}", path.display()))
}

// ─── Blanket loader test ─────────────────────────────────────────────────────

#[test]
fn all_example_graphs_parse_successfully() {
    let mut failed = Vec::new();
    for entry in std::fs::read_dir(examples_dir()).expect("examples/graphs/ must exist") {
        let path = entry.unwrap().path();
        if path.extension().and_then(|e| e.to_str()) == Some("toml") {
            let content = std::fs::read_to_string(&path)
                .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
            if let Err(e) = roko_graph::loader::load_from_str(&content) {
                failed.push(format!("{}: {}", path.display(), e));
            }
        }
    }

    if !failed.is_empty() {
        panic!(
            "{} example graph(s) failed to parse:\n{}",
            failed.len(),
            failed.join("\n")
        );
    }
}

// ─── task-execution.toml structural assertions ──────────────────────────────

#[test]
fn task_execution_metadata() {
    let graph = load_example("task-execution.toml");
    assert_eq!(graph.metadata.name, "task-execution");
    assert_eq!(
        graph.metadata.description.as_deref(),
        Some("Standard compose -> agent -> gate pipeline with retry on failure")
    );
    assert_eq!(graph.metadata.version.as_deref(), Some("1.0.0"));
}

#[test]
fn task_execution_policy() {
    let graph = load_example("task-execution.toml");
    assert_eq!(graph.policy.mode, GraphMode::OneShot);
    assert_eq!(graph.policy.failure_strategy, FailureStrategy::FailFast);
    assert_eq!(graph.policy.max_concurrent_nodes, 2);
    assert_eq!(graph.policy.timeout_ms, Some(900_000));
}

#[test]
fn task_execution_node_and_edge_counts() {
    let graph = load_example("task-execution.toml");
    assert_eq!(graph.node_count(), 7);
    assert_eq!(graph.edge_count(), 7);
}

#[test]
fn task_execution_nodes_present() {
    let graph = load_example("task-execution.toml");
    let expected = [
        ("compose_task", "compose"),
        ("agent_implement", "agent"),
        ("compile_gate", "gate"),
        ("test_gate", "gate"),
        ("success_report", "compose"),
        ("retry_compose", "compose"),
        ("retry_agent", "agent"),
    ];
    for (id, cell_type) in expected {
        let node = graph
            .get_node(id)
            .unwrap_or_else(|| panic!("node '{id}' missing"));
        assert_eq!(node.cell_type, cell_type, "cell_type mismatch for '{id}'");
    }
}

#[test]
fn task_execution_failure_edge() {
    let graph = load_example("task-execution.toml");
    // The compile_gate -> retry_compose edge must carry a Failure condition.
    let failure_edges: Vec<_> = graph
        .inner
        .edge_weights()
        .filter(|e| e.from == "compile_gate" && e.to == "retry_compose")
        .collect();
    assert_eq!(failure_edges.len(), 1);
    assert_eq!(
        failure_edges[0].condition,
        Some(EdgeCondition::Failure),
        "compile_gate -> retry_compose must be a failure edge"
    );
}

// ─── conditional-branch.toml structural assertions ──────────────────────────

#[test]
fn conditional_branch_metadata() {
    let graph = load_example("conditional-branch.toml");
    assert_eq!(graph.metadata.name, "conditional-branch");
    assert_eq!(
        graph.metadata.description.as_deref(),
        Some("Route execution based on quality gate score")
    );
    assert_eq!(graph.metadata.version.as_deref(), Some("1.0.0"));
}

#[test]
fn conditional_branch_policy() {
    let graph = load_example("conditional-branch.toml");
    assert_eq!(graph.policy.mode, GraphMode::OneShot);
    assert_eq!(graph.policy.failure_strategy, FailureStrategy::FailFast);
    assert_eq!(graph.policy.max_concurrent_nodes, 2);
    assert_eq!(graph.policy.timeout_ms, Some(600_000));
}

#[test]
fn conditional_branch_node_and_edge_counts() {
    let graph = load_example("conditional-branch.toml");
    assert_eq!(graph.node_count(), 6);
    assert_eq!(graph.edge_count(), 7);
}

#[test]
fn conditional_branch_nodes_present() {
    let graph = load_example("conditional-branch.toml");
    let expected = [
        ("compose_prompt", "compose"),
        ("agent_review", "agent"),
        ("high_quality_path", "compose"),
        ("medium_quality_path", "agent"),
        ("low_quality_path", "agent"),
        ("notify", "compose"),
    ];
    for (id, cell_type) in expected {
        let node = graph
            .get_node(id)
            .unwrap_or_else(|| panic!("node '{id}' missing"));
        assert_eq!(node.cell_type, cell_type, "cell_type mismatch for '{id}'");
    }
}

#[test]
fn conditional_branch_fan_out_sequence() {
    let graph = load_example("conditional-branch.toml");
    // agent_review fans out to three quality paths, all with Always condition.
    let fan_out: Vec<_> = graph
        .inner
        .edge_weights()
        .filter(|e| e.from == "agent_review")
        .collect();
    assert_eq!(fan_out.len(), 3, "agent_review must fan out to 3 paths");
    for edge in &fan_out {
        assert_eq!(
            edge.condition,
            Some(EdgeCondition::Always),
            "fan-out edge {} -> {} must be Always",
            edge.from,
            edge.to
        );
    }

    // All three quality paths converge into notify.
    let fan_in: Vec<_> = graph
        .inner
        .edge_weights()
        .filter(|e| e.to == "notify")
        .collect();
    assert_eq!(fan_in.len(), 3, "notify must receive from 3 paths");
}
