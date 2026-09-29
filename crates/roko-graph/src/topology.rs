//! Production plan execution topology (#256).
//!
//! `ProductionPlanTopology` builds the canonical per-task subgraph:
//!
//! ```text
//! [TaskContextCell] --> [KnowledgeCell]    --+
//!                  --> [EpisodesCell]      --|
//!                  --> [PlaybookCell]      --|-> [ComposeCell] -> [TaskExecutorCell] -> [GateCell] -> [SuccessBoundary]
//!                  --> [ModulationCell]    --|
//!                  --> [SafetyCell]        --|
//!                  --> [ExperimentCell]    --+
//! ```
//!
//! All enrichment edges are parallel (same wave). ComposeCell receives 7 inputs:
//! 6 enrichment Signals + 1 TaskContext Signal. Inter-task dependencies connect
//! the predecessor's `SuccessBoundary` to the dependent's `TaskContextCell`.
//!
//! "Controller effects" (completion sinks, delivery, resource release, terminal
//! state write) run in the host's `drive_controller` loop *outside* the graph
//! engine. They are not graph nodes.

use std::collections::{HashMap, HashSet};

use crate::types::{Edge, EdgeCondition, ExecutionClass, Graph, GraphError, GraphMetadata, Node};

// ─── Node ID conventions ─────────────────────────────────────────────────────

/// Enricher cell types in the per-task subgraph (all run in parallel).
const ENRICHER_SUFFIXES: &[&str] = &[
    "knowledge",
    "episodes",
    "playbook",
    "modulation",
    "safety",
    "experiment",
];

/// Build a namespaced node ID: `task.<task_id>.<suffix>`.
fn task_node_id(task_id: &str, suffix: &str) -> String {
    format!("task.{task_id}.{suffix}")
}

// ─── Task info ───────────────────────────────────────────────────────────────

/// Minimal task information needed to build a production topology.
///
/// This mirrors [`crate::convert::PlanTaskInfo`] but is specific to the
/// enriched topology. The converter constructs this from its source types.
#[derive(Debug, Clone)]
pub struct TopologyTaskInfo {
    /// Task ID (unique within the plan).
    pub task_id: String,
    /// Human-readable task title.
    pub title: String,
    /// Optional detailed description.
    pub description: Option<String>,
    /// Requested agent role.
    pub role: Option<String>,
    /// Complexity tier label.
    pub tier: String,
    /// Model hint for task dispatch.
    pub model_hint: Option<String>,
    /// Files expected to be in scope.
    pub files: Vec<String>,
    /// Task IDs this task depends on (same plan).
    pub depends_on: Vec<String>,
    /// Per-task timeout in seconds.
    pub timeout_secs: u64,
    /// Maximum retry attempts.
    pub max_retries: u32,
    /// Work domain.
    pub domain: Option<String>,
    /// Definition order index.
    pub sequence: usize,
    /// Full serialized task config as JSON.
    pub full_config_json: serde_json::Value,
}

// ─── ProductionPlanTopology ─────────────────────────────────────────────────

/// The production plan execution graph layout.
///
/// Each task gets a subgraph of 11 nodes (context + 6 enrichers + compose +
/// executor + gate + success-boundary). Inter-task edges connect predecessor
/// success boundaries to dependent task context nodes.
///
/// This is a pure data structure -- it builds a `Graph` but does not execute
/// it. The caller passes the resulting `Graph` to `GraphEngine` for execution.
#[derive(Debug)]
pub struct ProductionPlanTopology {
    /// Plan identifier.
    pub plan_id: String,
    /// Source plan directory.
    pub plan_dir: String,
    /// Maximum concurrent tasks.
    pub max_parallel: usize,
}

/// Report produced after topology construction.
#[derive(Debug, Clone)]
pub struct TopologyReport {
    /// Total number of nodes in the graph.
    pub total_nodes: usize,
    /// Total number of edges in the graph.
    pub total_edges: usize,
    /// Number of tasks that became subgraphs.
    pub task_count: usize,
    /// Entry task IDs (no predecessors).
    pub entry_tasks: Vec<String>,
    /// Exit task IDs (no dependents).
    pub exit_tasks: Vec<String>,
}

impl ProductionPlanTopology {
    /// Create a new topology builder.
    #[must_use]
    pub fn new(
        plan_id: impl Into<String>,
        plan_dir: impl Into<String>,
        max_parallel: usize,
    ) -> Self {
        Self {
            plan_id: plan_id.into(),
            plan_dir: plan_dir.into(),
            max_parallel: max_parallel.max(1),
        }
    }

    /// Build the production graph from a list of tasks.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - A task's `depends_on` references a task ID not present in the plan
    /// - The resulting graph contains a cycle
    /// - Two tasks share the same ID
    pub fn build(&self, tasks: &[TopologyTaskInfo]) -> Result<(Graph, TopologyReport), GraphError> {
        let known_ids: HashSet<&str> = tasks.iter().map(|t| t.task_id.as_str()).collect();

        // Validate all dependencies exist.
        for task in tasks {
            for dep in &task.depends_on {
                if !known_ids.contains(dep.as_str()) {
                    return Err(GraphError::InvalidGraph {
                        reason: format!(
                            "task '{}' in plan '{}' depends on '{}', which does not exist",
                            task.task_id, self.plan_id, dep
                        ),
                    });
                }
            }
        }

        let metadata = GraphMetadata {
            name: self.plan_id.clone(),
            description: Some(format!(
                "Production plan topology for '{}' ({} tasks)",
                self.plan_id,
                tasks.len()
            )),
            version: Some("2".to_string()),
            labels: {
                let mut labels = HashMap::new();
                labels.insert("source".to_string(), "production-plan-topology".to_string());
                labels.insert("plan_id".to_string(), self.plan_id.clone());
                labels.insert("plan_dir".to_string(), self.plan_dir.clone());
                labels
            },
        };

        let mut graph = Graph::new(metadata);
        graph.policy.max_concurrent_nodes = self.max_parallel;

        // Phase 1: Build per-task subgraphs.
        for task in tasks {
            self.add_task_subgraph(&mut graph, task)?;
        }

        // Phase 2: Wire inter-task dependencies.
        // Predecessor's success boundary -> dependent's context node.
        for task in tasks {
            let dep_context = task_node_id(&task.task_id, "context");
            for dep_task_id in &task.depends_on {
                let dep_success = task_node_id(dep_task_id, "success");
                graph.add_edge(Edge {
                    from: dep_success,
                    to: dep_context.clone(),
                    condition: Some(EdgeCondition::Success),
                })?;
            }
        }

        // Phase 3: Validate (cycle detection).
        crate::topo::topological_order(&graph)?;

        // Compute report.
        let entry_tasks: Vec<String> = tasks
            .iter()
            .filter(|t| t.depends_on.is_empty())
            .map(|t| t.task_id.clone())
            .collect();

        let dependents: HashSet<&str> = tasks
            .iter()
            .flat_map(|t| t.depends_on.iter().map(String::as_str))
            .collect();
        let exit_tasks: Vec<String> = tasks
            .iter()
            .filter(|t| !dependents.contains(t.task_id.as_str()))
            .map(|t| t.task_id.clone())
            .collect();

        let report = TopologyReport {
            total_nodes: graph.node_count(),
            total_edges: graph.edge_count(),
            task_count: tasks.len(),
            entry_tasks,
            exit_tasks,
        };

        Ok((graph, report))
    }

    /// Add the 11-node subgraph for a single task.
    #[allow(clippy::too_many_lines)]
    fn add_task_subgraph(
        &self,
        graph: &mut Graph,
        task: &TopologyTaskInfo,
    ) -> Result<(), GraphError> {
        let tid = &task.task_id;

        // 1. TaskContext node (root of per-task subgraph).
        let context_id = task_node_id(tid, "context");
        let context_config = self.build_context_config(task);
        graph.add_node(Node {
            id: context_id.clone(),
            cell_type: "plan.task-context".to_string(),
            config: context_config,
            inputs: vec![],
            outputs: vec![],
            execution_class: ExecutionClass::Workflow,
            exclusive: vec![],
        })?;

        // 2. Enricher nodes (all parallel, all Workflow class).
        for suffix in ENRICHER_SUFFIXES {
            let enricher_id = task_node_id(tid, suffix);
            let cell_type = format!("plan.enricher.{suffix}");
            graph.add_node(Node {
                id: enricher_id.clone(),
                cell_type,
                config: toml::Value::Table(toml::map::Map::new()),
                inputs: vec![],
                outputs: vec![],
                execution_class: ExecutionClass::Workflow,
                exclusive: vec![],
            })?;

            // Edge: context -> enricher (unconditional).
            graph.add_edge(Edge {
                from: context_id.clone(),
                to: enricher_id,
                condition: Some(EdgeCondition::Always),
            })?;
        }

        // 3. Compose node (fan-in from all enrichers + context).
        let compose_id = task_node_id(tid, "compose");
        graph.add_node(Node {
            id: compose_id.clone(),
            cell_type: "plan.compose".to_string(),
            config: toml::Value::Table(toml::map::Map::new()),
            inputs: vec![],
            outputs: vec![],
            execution_class: ExecutionClass::Workflow,
            exclusive: vec![],
        })?;

        // Edges: each enricher -> compose.
        for suffix in ENRICHER_SUFFIXES {
            let enricher_id = task_node_id(tid, suffix);
            graph.add_edge(Edge {
                from: enricher_id,
                to: compose_id.clone(),
                condition: Some(EdgeCondition::Always),
            })?;
        }
        // Edge: context -> compose (direct, for the 7th input).
        graph.add_edge(Edge {
            from: context_id.clone(),
            to: compose_id.clone(),
            condition: Some(EdgeCondition::Always),
        })?;

        // 4. TaskExecutor node (Activity: non-deterministic LLM dispatch).
        let executor_id = task_node_id(tid, "executor");
        let executor_config = self.build_executor_config(task);
        graph.add_node(Node {
            id: executor_id.clone(),
            cell_type: "task-executor".to_string(),
            config: executor_config,
            inputs: vec![],
            outputs: vec![],
            execution_class: ExecutionClass::Activity,
            // The executor writes the task's files and the gate checks them:
            // both hold the files, so no overlapping task edits them meanwhile.
            exclusive: task.files.clone(),
        })?;

        // Edge: compose -> executor.
        graph.add_edge(Edge {
            from: compose_id,
            to: executor_id.clone(),
            condition: Some(EdgeCondition::Always),
        })?;

        // 5. Gate node (Activity: runs gate pipeline).
        let gate_id = task_node_id(tid, "gate");
        let gate_config = self.build_gate_config(task);
        graph.add_node(Node {
            id: gate_id.clone(),
            cell_type: "plan.gate".to_string(),
            config: gate_config,
            inputs: vec![],
            outputs: vec![],
            execution_class: ExecutionClass::Activity,
            exclusive: task.files.clone(),
        })?;

        // Edge: executor -> gate (on success only).
        graph.add_edge(Edge {
            from: executor_id,
            to: gate_id.clone(),
            condition: Some(EdgeCondition::Success),
        })?;

        // 6. Success boundary (Workflow: no-op passthrough anchor).
        let success_id = task_node_id(tid, "success");
        graph.add_node(Node {
            id: success_id.clone(),
            cell_type: "plan.success-boundary".to_string(),
            config: toml::Value::Table(toml::map::Map::new()),
            inputs: vec![],
            outputs: vec![],
            execution_class: ExecutionClass::Workflow,
            exclusive: vec![],
        })?;

        // Edge: gate -> success boundary (on success only).
        graph.add_edge(Edge {
            from: gate_id,
            to: success_id,
            condition: Some(EdgeCondition::Success),
        })?;

        Ok(())
    }

    /// Build the TOML config for a TaskContext node.
    fn build_context_config(&self, task: &TopologyTaskInfo) -> toml::Value {
        let mut table = toml::map::Map::new();
        table.insert(
            "plan_id".to_string(),
            toml::Value::String(self.plan_id.clone()),
        );
        table.insert(
            "plan_dir".to_string(),
            toml::Value::String(self.plan_dir.clone()),
        );
        table.insert(
            "task_id".to_string(),
            toml::Value::String(task.task_id.clone()),
        );
        table.insert("title".to_string(), toml::Value::String(task.title.clone()));
        if let Some(ref desc) = task.description {
            table.insert("description".to_string(), toml::Value::String(desc.clone()));
        }
        if let Some(ref role) = task.role {
            table.insert("role".to_string(), toml::Value::String(role.clone()));
        }
        table.insert("tier".to_string(), toml::Value::String(task.tier.clone()));
        if let Some(ref hint) = task.model_hint {
            table.insert("model_hint".to_string(), toml::Value::String(hint.clone()));
        }
        if let Some(ref domain) = task.domain {
            table.insert("domain".to_string(), toml::Value::String(domain.clone()));
        }
        table.insert(
            "timeout_secs".to_string(),
            toml::Value::Integer(task.timeout_secs.min(i64::MAX as u64) as i64),
        );
        table.insert(
            "sequence".to_string(),
            toml::Value::Integer(task.sequence as i64),
        );

        let files_arr: Vec<toml::Value> = task
            .files
            .iter()
            .map(|f| toml::Value::String(f.clone()))
            .collect();
        table.insert("files".to_string(), toml::Value::Array(files_arr));

        toml::Value::Table(table)
    }

    /// Build the TOML config for a TaskExecutor node.
    fn build_executor_config(&self, task: &TopologyTaskInfo) -> toml::Value {
        let mut table = toml::map::Map::new();
        table.insert(
            "plan_id".to_string(),
            toml::Value::String(self.plan_id.clone()),
        );
        table.insert(
            "plan_dir".to_string(),
            toml::Value::String(self.plan_dir.clone()),
        );
        table.insert("title".to_string(), toml::Value::String(task.title.clone()));
        if let Some(ref desc) = task.description {
            table.insert("description".to_string(), toml::Value::String(desc.clone()));
        }
        if let Some(ref role) = task.role {
            table.insert("role".to_string(), toml::Value::String(role.clone()));
        }
        table.insert("tier".to_string(), toml::Value::String(task.tier.clone()));
        if let Some(ref hint) = task.model_hint {
            table.insert("model_hint".to_string(), toml::Value::String(hint.clone()));
        }
        if let Some(ref domain) = task.domain {
            table.insert("domain".to_string(), toml::Value::String(domain.clone()));
        }
        table.insert(
            "timeout_secs".to_string(),
            toml::Value::Integer(task.timeout_secs.min(i64::MAX as u64) as i64),
        );
        table.insert(
            "max_retries".to_string(),
            toml::Value::Integer(i64::from(task.max_retries)),
        );
        table.insert(
            "sequence".to_string(),
            toml::Value::Integer(task.sequence as i64),
        );

        let files_arr: Vec<toml::Value> = task
            .files
            .iter()
            .map(|f| toml::Value::String(f.clone()))
            .collect();
        table.insert("files".to_string(), toml::Value::Array(files_arr));

        table.insert(
            "task_def_json".to_string(),
            toml::Value::String(task.full_config_json.to_string()),
        );
        // The task's `plan.gate` judges the attempt's own checkout, so the
        // executor hands it on instead of releasing it.
        table.insert("keep_workspace".to_string(), toml::Value::Boolean(true));

        toml::Value::Table(table)
    }

    /// Build the TOML config for a PlanGateCell node.
    fn build_gate_config(&self, task: &TopologyTaskInfo) -> toml::Value {
        let mut table = toml::map::Map::new();
        table.insert(
            "task_id".to_string(),
            toml::Value::String(task.task_id.clone()),
        );
        table.insert(
            "plan_id".to_string(),
            toml::Value::String(self.plan_id.clone()),
        );
        table.insert(
            "plan_dir".to_string(),
            toml::Value::String(self.plan_dir.clone()),
        );
        table.insert("title".to_string(), toml::Value::String(task.title.clone()));
        let files_arr: Vec<toml::Value> = task
            .files
            .iter()
            .map(|f| toml::Value::String(f.clone()))
            .collect();
        table.insert("files".to_string(), toml::Value::Array(files_arr));
        toml::Value::Table(table)
    }
}

/// Register the production plan topology cells in a registry.
///
/// Wires real implementations for the three core topology cells:
/// - `plan.task-context` → [`TaskContextCell`]: assembles task metadata and predecessor state.
/// - `plan.compose` → [`PlanComposeCell`]: fan-in enricher merge into a single Prompt signal.
/// - `plan.gate` → [`PlanGateCell`]: runs the gate pipeline via `SharedGateEvaluator`.
///
/// The six enricher cells (`plan.enricher.*`) and the `plan.success-boundary` anchor
/// remain [`PassthroughCell`] stubs; real enricher implementations are injected by
/// host adapters that have access to the knowledge store, episode log, etc.
pub fn register_topology_cells(registry: &mut crate::registry::CellRegistry) {
    use crate::cells::stubs::PassthroughCell;
    use crate::registry::CellDescriptor;

    // TaskContext: collects task metadata and prior attempt state.
    registry.register_with_descriptor(
        "plan.task-context",
        CellDescriptor {
            id: "plan.task-context".to_string(),
            version: (0, 2, 0),
            input_schema: None,
            output_schema: None,
            is_stub: false,
            protocols: Vec::new(),
            is_predictive: false,
            display_name: Some("TaskContext".to_string()),
        },
        |config| Box::new(crate::cells::TaskContextCell::new(&config)),
    );

    // Enricher cells: each consumes TaskContext output and produces enrichment signals.
    for suffix in ENRICHER_SUFFIXES {
        let cell_type = format!("plan.enricher.{suffix}");
        let cell_type_clone = cell_type.clone();
        let display = format!("{}Enricher", suffix[..1].to_uppercase() + &suffix[1..]);
        registry.register_with_descriptor(
            // leak the string for 'static lifetime -- these are registered once at startup
            Box::leak(cell_type.clone().into_boxed_str()),
            CellDescriptor {
                id: cell_type.clone(),
                version: (0, 1, 0),
                input_schema: None,
                output_schema: None,
                is_stub: true,
                protocols: Vec::new(),
                is_predictive: false,
                display_name: Some(display),
            },
            move |_config| Box::new(PassthroughCell::new(cell_type_clone.clone())),
        );
    }

    // Compose: fan-in from enrichers + context -> single Prompt signal.
    registry.register_with_descriptor(
        "plan.compose",
        CellDescriptor {
            id: "plan.compose".to_string(),
            version: (0, 2, 0),
            input_schema: None,
            output_schema: None,
            is_stub: false,
            protocols: vec![roko_core::ProtocolId::Compose],
            is_predictive: false,
            display_name: Some("PlanCompose".to_string()),
        },
        |_config| Box::new(crate::cells::PlanComposeCell::new()),
    );

    // Gate: runs the gate pipeline on executor output via SharedGateEvaluator.
    registry.register_with_descriptor(
        "plan.gate",
        CellDescriptor {
            id: "plan.gate".to_string(),
            version: (0, 1, 0),
            input_schema: None,
            output_schema: None,
            is_stub: false,
            protocols: vec![roko_core::ProtocolId::Verify],
            is_predictive: false,
            display_name: Some("PlanGate".to_string()),
        },
        |config| Box::new(crate::cells::PlanGateCell::from_config(&config)),
    );

    // Success boundary: no-op passthrough, dependency anchor only.
    registry.register_with_descriptor(
        "plan.success-boundary",
        CellDescriptor {
            id: "plan.success-boundary".to_string(),
            version: (0, 1, 0),
            input_schema: None,
            output_schema: None,
            is_stub: true,
            protocols: Vec::new(),
            is_predictive: false,
            display_name: Some("SuccessBoundary".to_string()),
        },
        |_config| Box::new(PassthroughCell::new("plan.success-boundary")),
    );
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_task(id: &str, depends_on: &[&str]) -> TopologyTaskInfo {
        TopologyTaskInfo {
            task_id: id.to_string(),
            title: format!("Task {id}"),
            description: None,
            role: Some("implementer".to_string()),
            tier: "mechanical".to_string(),
            model_hint: None,
            files: vec![],
            depends_on: depends_on.iter().map(|s| s.to_string()).collect(),
            timeout_secs: 300,
            max_retries: 2,
            domain: None,
            sequence: 0,
            full_config_json: serde_json::json!({"id": id}),
        }
    }

    #[test]
    fn single_task_produces_11_nodes() {
        let topo = ProductionPlanTopology::new("test-plan", "/tmp", 1);
        let tasks = vec![make_task("T1", &[])];
        let (graph, report) = topo.build(&tasks).unwrap();

        // 1 context + 6 enrichers + 1 compose + 1 executor + 1 gate + 1 success = 11
        assert_eq!(graph.node_count(), 11);
        assert_eq!(report.task_count, 1);
        assert_eq!(report.entry_tasks, vec!["T1"]);
        assert_eq!(report.exit_tasks, vec!["T1"]);
    }

    #[test]
    fn single_task_edge_count() {
        let topo = ProductionPlanTopology::new("test-plan", "/tmp", 1);
        let tasks = vec![make_task("T1", &[])];
        let (graph, _) = topo.build(&tasks).unwrap();

        // Intra-task edges:
        // context -> 6 enrichers = 6
        // 6 enrichers -> compose = 6
        // context -> compose = 1  (direct 7th input)
        // compose -> executor = 1
        // executor -> gate = 1
        // gate -> success = 1
        // Total = 16
        assert_eq!(graph.edge_count(), 16);
    }

    #[test]
    fn two_task_linear_chain() {
        let topo = ProductionPlanTopology::new("chain", "/tmp", 1);
        let tasks = vec![make_task("T1", &[]), make_task("T2", &["T1"])];
        let (graph, report) = topo.build(&tasks).unwrap();

        assert_eq!(graph.node_count(), 22); // 11 * 2
        // Intra-task: 16 * 2 = 32
        // Inter-task: T1.success -> T2.context = 1
        assert_eq!(graph.edge_count(), 33);
        assert_eq!(report.entry_tasks, vec!["T1"]);
        assert_eq!(report.exit_tasks, vec!["T2"]);
    }

    #[test]
    fn diamond_dependency_topology() {
        let topo = ProductionPlanTopology::new("diamond", "/tmp", 2);
        let tasks = vec![
            make_task("T1", &[]),
            make_task("T2", &["T1"]),
            make_task("T3", &["T1"]),
            make_task("T4", &["T2", "T3"]),
        ];
        let (graph, report) = topo.build(&tasks).unwrap();

        assert_eq!(graph.node_count(), 44); // 11 * 4
        // Intra-task: 16 * 4 = 64
        // Inter-task: T1->T2, T1->T3, T2->T4, T3->T4 = 4
        assert_eq!(graph.edge_count(), 68);
        assert_eq!(report.entry_tasks, vec!["T1"]);
        assert_eq!(report.exit_tasks, vec!["T4"]);
        assert_eq!(report.task_count, 4);
    }

    #[test]
    fn parallel_tasks_no_deps() {
        let topo = ProductionPlanTopology::new("parallel", "/tmp", 3);
        let tasks = vec![
            make_task("T1", &[]),
            make_task("T2", &[]),
            make_task("T3", &[]),
        ];
        let (graph, report) = topo.build(&tasks).unwrap();

        assert_eq!(graph.node_count(), 33); // 11 * 3
        assert_eq!(graph.edge_count(), 48); // 16 * 3, no inter-task edges
        assert_eq!(report.entry_tasks.len(), 3);
        assert_eq!(report.exit_tasks.len(), 3);
    }

    #[test]
    fn missing_dependency_errors() {
        let topo = ProductionPlanTopology::new("bad", "/tmp", 1);
        let tasks = vec![make_task("T1", &["T_MISSING"])];
        let result = topo.build(&tasks);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("T_MISSING"));
    }

    #[test]
    fn duplicate_task_id_errors() {
        let topo = ProductionPlanTopology::new("dup", "/tmp", 1);
        let tasks = vec![make_task("T1", &[]), make_task("T1", &[])];
        let result = topo.build(&tasks);
        assert!(matches!(result, Err(GraphError::DuplicateNode(_))));
    }

    #[test]
    fn cycle_detected() {
        let topo = ProductionPlanTopology::new("cyclic", "/tmp", 1);
        let tasks = vec![make_task("T1", &["T2"]), make_task("T2", &["T1"])];
        let result = topo.build(&tasks);
        assert!(matches!(result, Err(GraphError::CycleDetected)));
    }

    #[test]
    fn node_ids_follow_convention() {
        let topo = ProductionPlanTopology::new("conv", "/tmp", 1);
        let tasks = vec![make_task("T1", &[])];
        let (graph, _) = topo.build(&tasks).unwrap();

        // Verify all expected node IDs exist.
        assert!(graph.get_node("task.T1.context").is_some());
        assert!(graph.get_node("task.T1.knowledge").is_some());
        assert!(graph.get_node("task.T1.episodes").is_some());
        assert!(graph.get_node("task.T1.playbook").is_some());
        assert!(graph.get_node("task.T1.modulation").is_some());
        assert!(graph.get_node("task.T1.safety").is_some());
        assert!(graph.get_node("task.T1.experiment").is_some());
        assert!(graph.get_node("task.T1.compose").is_some());
        assert!(graph.get_node("task.T1.executor").is_some());
        assert!(graph.get_node("task.T1.gate").is_some());
        assert!(graph.get_node("task.T1.success").is_some());
    }

    #[test]
    fn executor_node_is_activity_class() {
        let topo = ProductionPlanTopology::new("class", "/tmp", 1);
        let tasks = vec![make_task("T1", &[])];
        let (graph, _) = topo.build(&tasks).unwrap();

        let executor = graph.get_node("task.T1.executor").unwrap();
        assert_eq!(executor.execution_class, ExecutionClass::Activity);

        let gate = graph.get_node("task.T1.gate").unwrap();
        assert_eq!(gate.execution_class, ExecutionClass::Activity);

        // Enrichers are Workflow (deterministic).
        let context = graph.get_node("task.T1.context").unwrap();
        assert_eq!(context.execution_class, ExecutionClass::Workflow);

        let knowledge = graph.get_node("task.T1.knowledge").unwrap();
        assert_eq!(knowledge.execution_class, ExecutionClass::Workflow);

        let success = graph.get_node("task.T1.success").unwrap();
        assert_eq!(success.execution_class, ExecutionClass::Workflow);
    }

    #[test]
    fn max_parallel_clamped_to_one() {
        let topo = ProductionPlanTopology::new("serial", "/tmp", 0);
        assert_eq!(topo.max_parallel, 1);
    }

    #[test]
    fn context_config_contains_task_metadata() {
        let topo = ProductionPlanTopology::new("meta", "/work", 1);
        let tasks = vec![TopologyTaskInfo {
            task_id: "T1".to_string(),
            title: "Build feature".to_string(),
            description: Some("A detailed description".to_string()),
            role: Some("implementer".to_string()),
            tier: "focused".to_string(),
            model_hint: Some("claude-sonnet-4-20250514".to_string()),
            files: vec!["src/lib.rs".to_string()],
            depends_on: vec![],
            timeout_secs: 600,
            max_retries: 3,
            domain: Some("coding".to_string()),
            sequence: 0,
            full_config_json: serde_json::json!({"id": "T1"}),
        }];
        let (graph, _) = topo.build(&tasks).unwrap();

        let context = graph.get_node("task.T1.context").unwrap();
        let table = context.config.as_table().unwrap();
        assert_eq!(table["plan_id"].as_str().unwrap(), "meta");
        assert_eq!(table["plan_dir"].as_str().unwrap(), "/work");
        assert_eq!(table["task_id"].as_str().unwrap(), "T1");
        assert_eq!(table["title"].as_str().unwrap(), "Build feature");
        assert_eq!(table["role"].as_str().unwrap(), "implementer");
        assert_eq!(table["tier"].as_str().unwrap(), "focused");
    }

    #[test]
    fn graph_validates_with_topology_registry() {
        use crate::engine::GraphEngine;

        let topo = ProductionPlanTopology::new("valid", "/tmp", 2);
        let tasks = vec![make_task("T1", &[]), make_task("T2", &["T1"])];
        let (graph, _) = topo.build(&tasks).unwrap();

        let mut registry = crate::engine::default_registry();
        register_topology_cells(&mut registry);

        let engine = GraphEngine::new(graph, registry).with_allow_test_stubs(true);
        let issues = engine.validate();
        assert!(issues.is_empty(), "validation issues: {issues:?}");
    }

    /// bug-50caf2: the executor hands its checkout on to the gate, and the
    /// gate knows the task it judges.
    #[test]
    fn executor_keeps_its_workspace_for_the_gate() {
        let topo = ProductionPlanTopology::new("keep", "/tmp", 1);
        let (graph, _) = topo.build(&[make_task("T1", &[])]).unwrap();

        let executor = graph.get_node("task.T1.executor").unwrap();
        let spec = crate::cells::TaskExecutionSpec::from_config(&executor.config);
        assert!(spec.keep_workspace);
        let gate = graph.get_node("task.T1.gate").unwrap();
        let table = gate.config.as_table().unwrap();
        assert_eq!(table["title"].as_str(), Some("Task T1"));
        assert_eq!(table["task_id"].as_str(), Some("T1"));
    }

    /// gap-439794: a task's executor writes its files and its gate checks
    /// them, so both hold the files. No other node of the subgraph does.
    #[test]
    fn executor_and_gate_hold_the_task_files() {
        let topo = ProductionPlanTopology::new("files", "/tmp", 2);
        let mut task = make_task("T1", &[]);
        task.files = vec!["src/lib.rs".to_string()];
        let (graph, _) = topo.build(&[task]).unwrap();

        for node in graph.inner.node_weights() {
            let holds = node.id == "task.T1.executor" || node.id == "task.T1.gate";
            let expected: &[&str] = if holds { &["src/lib.rs"] } else { &[] };
            assert_eq!(node.exclusive, expected, "{}", node.id);
        }
    }
}
