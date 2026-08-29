//! Cross-plan DAG wave computation using Kahn's algorithm (#117).
//!
//! Given a set of plans where each plan may declare cross-plan dependencies
//! via `depends_on_plan` fields in its tasks, [`CrossPlanDag::compute`]
//! groups plans into execution waves:
//!
//! - **Wave 0**: plans with no cross-plan dependencies
//! - **Wave 1**: plans that depend only on wave-0 plans
//! - **Wave N**: plans that depend only on plans in waves 0..N-1
//!
//! Plans within the same wave can execute in parallel. The module also
//! detects cycles and reports file-overlap warnings for parallel plans
//! in the same wave.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::fmt;

use serde::Serialize;
use thiserror::Error;

use super::plan_loader::Plan;

// ─── Error types ─────────────────────────────────────────────────────

/// Errors returned by cross-plan DAG computation.
#[derive(Debug, Error)]
pub enum CrossPlanDagError {
    /// A dependency cycle was detected among plans.
    #[error("dependency cycle detected: {}", format_cycle(cycle))]
    Cycle {
        /// Plan IDs participating in the cycle.
        cycle: Vec<String>,
    },
}

fn format_cycle(cycle: &[String]) -> String {
    if cycle.is_empty() {
        return String::new();
    }
    let mut parts: Vec<&str> = cycle.iter().map(String::as_str).collect();
    // Close the cycle for display
    if let Some(first) = cycle.first() {
        parts.push(first.as_str());
    }
    parts.join(" -> ")
}

// ─── Plan node ───────────────────────────────────────────────────────

/// A plan node in the cross-plan DAG.
#[derive(Debug, Clone, Serialize)]
pub struct PlanNode {
    /// Plan identifier (directory name).
    pub id: String,
    /// Wave index assigned by Kahn's algorithm.
    pub wave_index: usize,
    /// Cross-plan dependencies declared by this plan's tasks.
    pub depends_on_plans: Vec<String>,
    /// Number of tasks in this plan.
    pub task_count: usize,
    /// Crate directories touched by this plan's tasks.
    pub crates_touched: Vec<String>,
}

// ─── Wave info ───────────────────────────────────────────────────────

/// Summary of one execution wave.
#[derive(Debug, Clone, Serialize)]
pub struct WaveInfo {
    /// Zero-based wave index.
    pub index: usize,
    /// Plan IDs in this wave.
    pub plan_ids: Vec<String>,
    /// Parallelism width (number of plans in this wave).
    pub parallelism_width: usize,
    /// Total tasks across all plans in this wave.
    pub total_tasks: usize,
}

// ─── Crate overlap warning ──────────────────────────────────────────

/// Warning emitted when two plans in the same wave touch the same crate.
#[derive(Debug, Clone, Serialize)]
pub struct CrateOverlap {
    /// Crate name shared by multiple plans.
    pub crate_name: String,
    /// Plan IDs that both touch this crate.
    pub plans: Vec<String>,
    /// Wave index where the overlap occurs.
    pub wave_index: usize,
}

impl fmt::Display for CrateOverlap {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Wave {}: {} both modify crate {}",
            self.wave_index,
            self.plans.join(" and "),
            self.crate_name,
        )
    }
}

// ─── Dangling reference ─────────────────────────────────────────────

/// A plan references a dependency that does not exist in the plan set.
#[derive(Debug, Clone, Serialize)]
pub struct DanglingRef {
    /// The plan that declares the dependency.
    pub from_plan: String,
    /// The referenced plan ID that does not exist.
    pub references: String,
}

impl fmt::Display for DanglingRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} depends_on_plan \"{}\" which does not exist",
            self.from_plan, self.references,
        )
    }
}

// ─── DAG summary ────────────────────────────────────────────────────

/// Summary statistics for the cross-plan DAG (for `--dag` output).
#[derive(Debug, Clone, Serialize, Default)]
pub struct DagSummary {
    /// Total number of plans.
    pub total_plans: usize,
    /// Total number of tasks across all plans.
    pub total_tasks: usize,
    /// Total number of cross-plan dependency edges.
    pub total_edges: usize,
    /// Wave breakdown.
    pub waves: Vec<WaveInfo>,
    /// Plan IDs on the critical path (longest sequential wave chain).
    pub critical_path: Vec<String>,
    /// Estimated minutes along the critical path.
    pub critical_path_minutes: u32,
    /// Dangling dependency references.
    pub dangling_refs: Vec<DanglingRef>,
    /// Crate overlap warnings.
    pub crate_overlaps: Vec<CrateOverlap>,
}

// ─── CrossPlanDag ───────────────────────────────────────────────────

/// Cross-plan DAG with wave assignments computed via Kahn's algorithm.
#[derive(Debug, Clone)]
pub struct CrossPlanDag {
    /// All plan nodes, ordered by wave_index then plan id.
    pub plans: Vec<PlanNode>,
    /// Waves: wave_index -> list of plan IDs.
    pub waves: Vec<Vec<String>>,
    /// Critical path: the longest chain of sequential plan dependencies.
    pub critical_path: Vec<String>,
    /// Total estimated minutes along the critical path.
    pub critical_path_minutes: u32,
    /// Crate overlap warnings.
    pub crate_overlaps: Vec<CrateOverlap>,
    /// Dangling dependency references.
    pub dangling_refs: Vec<DanglingRef>,
}

impl CrossPlanDag {
    /// Compute wave assignments from a set of loaded plans.
    ///
    /// Each plan's cross-plan dependencies are extracted from the union of
    /// all `depends_on_plan` fields across its tasks. Kahn's algorithm
    /// assigns wave indices, with cycle detection.
    pub fn compute(plans: &[Plan]) -> Result<Self, CrossPlanDagError> {
        let plan_ids: HashSet<&str> = plans.iter().map(|p| p.id.as_str()).collect();

        // Extract per-plan cross-plan dependencies and crates touched.
        let mut plan_deps: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        let mut plan_crates: HashMap<String, BTreeSet<String>> = HashMap::new();
        let mut plan_task_counts: HashMap<String, usize> = HashMap::new();

        for plan in plans {
            let mut deps = BTreeSet::new();
            let mut crates = BTreeSet::new();
            for task in &plan.tasks.tasks {
                for dep in &task.depends_on_plan {
                    deps.insert(dep.clone());
                }
                // Extract crate names from file paths.
                for file in &task.files {
                    if let Some(rest) = file.strip_prefix("crates/") {
                        if let Some(crate_name) = rest.split('/').next() {
                            if !crate_name.is_empty() {
                                crates.insert(crate_name.to_string());
                            }
                        }
                    }
                }
            }
            plan_deps.insert(plan.id.clone(), deps);
            plan_crates.insert(plan.id.clone(), crates);
            plan_task_counts.insert(plan.id.clone(), plan.tasks.tasks.len());
        }

        // Detect dangling references.
        let mut dangling_refs = Vec::new();
        for (plan_id, deps) in &plan_deps {
            for dep in deps {
                if !plan_ids.contains(dep.as_str()) {
                    dangling_refs.push(DanglingRef {
                        from_plan: plan_id.clone(),
                        references: dep.clone(),
                    });
                }
            }
        }

        // Filter deps to only known plans for DAG computation.
        let mut filtered_deps: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        for (plan_id, deps) in &plan_deps {
            let known: BTreeSet<String> = deps
                .iter()
                .filter(|d| plan_ids.contains(d.as_str()))
                .cloned()
                .collect();
            filtered_deps.insert(plan_id.clone(), known);
        }

        // Detect cycles.
        let cycle_nodes = detect_cycles(&filtered_deps);
        if !cycle_nodes.is_empty() {
            return Err(CrossPlanDagError::Cycle { cycle: cycle_nodes });
        }

        // Kahn's algorithm for wave assignment.
        let mut in_degree: HashMap<&str, usize> = HashMap::new();
        let mut dependents: HashMap<&str, Vec<&str>> = HashMap::new();

        for plan_id in &plan_ids {
            in_degree.entry(plan_id).or_insert(0);
        }

        for (plan_id, deps) in &filtered_deps {
            for dep in deps {
                *in_degree.entry(plan_id.as_str()).or_insert(0) += 1;
                dependents
                    .entry(dep.as_str())
                    .or_default()
                    .push(plan_id.as_str());
            }
        }

        let mut waves: Vec<Vec<String>> = Vec::new();
        let mut wave_assignment: HashMap<String, usize> = HashMap::new();
        let mut remaining: HashSet<&str> = plan_ids.clone();

        while !remaining.is_empty() {
            // Collect nodes with zero in-degree.
            let mut wave: Vec<String> = remaining
                .iter()
                .filter(|&&id| *in_degree.get(id).unwrap_or(&0) == 0)
                .map(|&id| id.to_string())
                .collect();
            wave.sort(); // deterministic order

            if wave.is_empty() {
                // This shouldn't happen since we already detected cycles.
                break;
            }

            let wave_idx = waves.len();
            for plan_id in &wave {
                wave_assignment.insert(plan_id.clone(), wave_idx);
                remaining.remove(plan_id.as_str());
                // Decrement in-degree of dependents.
                if let Some(children) = dependents.get(plan_id.as_str()) {
                    for &child in children {
                        if let Some(deg) = in_degree.get_mut(child) {
                            *deg = deg.saturating_sub(1);
                        }
                    }
                }
            }

            waves.push(wave);
        }

        // Build plan nodes.
        let mut plan_nodes: Vec<PlanNode> = plans
            .iter()
            .map(|p| {
                let wave_index = wave_assignment.get(&p.id).copied().unwrap_or(0);
                let depends_on_plans: Vec<String> = plan_deps
                    .get(&p.id)
                    .map(|d| d.iter().cloned().collect())
                    .unwrap_or_default();
                let crates_touched: Vec<String> = plan_crates
                    .get(&p.id)
                    .map(|c| c.iter().cloned().collect())
                    .unwrap_or_default();
                PlanNode {
                    id: p.id.clone(),
                    wave_index,
                    depends_on_plans,
                    task_count: p.tasks.tasks.len(),
                    crates_touched,
                }
            })
            .collect();
        plan_nodes.sort_by(|a, b| {
            a.wave_index
                .cmp(&b.wave_index)
                .then_with(|| a.id.cmp(&b.id))
        });

        // Detect crate overlaps within waves.
        let mut crate_overlaps = Vec::new();
        for (wave_idx, wave_plans) in waves.iter().enumerate() {
            if wave_plans.len() < 2 {
                continue;
            }
            // Collect crate -> plans mapping for this wave.
            let mut crate_to_plans: BTreeMap<String, Vec<String>> = BTreeMap::new();
            for plan_id in wave_plans {
                if let Some(crates) = plan_crates.get(plan_id) {
                    for crate_name in crates {
                        crate_to_plans
                            .entry(crate_name.clone())
                            .or_default()
                            .push(plan_id.clone());
                    }
                }
            }
            for (crate_name, overlap_plans) in crate_to_plans {
                if overlap_plans.len() >= 2 {
                    crate_overlaps.push(CrateOverlap {
                        crate_name,
                        plans: overlap_plans,
                        wave_index: wave_idx,
                    });
                }
            }
        }

        // Compute critical path through the plan DAG.
        // Weight = total estimated minutes per plan (sum of task estimates).
        let plan_weights: HashMap<&str, u32> = plans
            .iter()
            .map(|p| {
                let weight: u32 = p
                    .tasks
                    .tasks
                    .iter()
                    .map(|t| {
                        t.estimated_minutes
                            .unwrap_or(super::task_dag::DEFAULT_ESTIMATED_MINUTES)
                    })
                    .sum();
                (p.id.as_str(), weight)
            })
            .collect();

        let (critical_path, critical_path_minutes) =
            compute_plan_critical_path(&filtered_deps, &plan_weights);

        Ok(CrossPlanDag {
            plans: plan_nodes,
            waves,
            critical_path,
            critical_path_minutes,
            crate_overlaps,
            dangling_refs,
        })
    }

    /// Get all plan IDs assigned to a given wave.
    #[must_use]
    pub fn plans_in_wave(&self, wave_index: usize) -> &[String] {
        self.waves.get(wave_index).map_or(&[], |v| v.as_slice())
    }

    /// Total number of execution waves.
    #[must_use]
    pub fn total_waves(&self) -> usize {
        self.waves.len()
    }

    /// Look up the wave index for a given plan ID.
    #[must_use]
    pub fn wave_for_plan(&self, plan_id: &str) -> Option<usize> {
        self.plans
            .iter()
            .find(|p| p.id == plan_id)
            .map(|p| p.wave_index)
    }

    /// Build a [`DagSummary`] for the `--dag` validate output.
    #[must_use]
    pub fn summary(&self) -> DagSummary {
        let total_plans = self.plans.len();
        let total_tasks: usize = self.plans.iter().map(|p| p.task_count).sum();
        let total_edges: usize = self.plans.iter().map(|p| p.depends_on_plans.len()).sum();

        let waves: Vec<WaveInfo> = self
            .waves
            .iter()
            .enumerate()
            .map(|(idx, plan_ids)| {
                let total_wave_tasks: usize = plan_ids
                    .iter()
                    .map(|id| {
                        self.plans
                            .iter()
                            .find(|p| &p.id == id)
                            .map(|p| p.task_count)
                            .unwrap_or(0)
                    })
                    .sum();
                WaveInfo {
                    index: idx,
                    plan_ids: plan_ids.clone(),
                    parallelism_width: plan_ids.len(),
                    total_tasks: total_wave_tasks,
                }
            })
            .collect();

        DagSummary {
            total_plans,
            total_tasks,
            total_edges,
            waves,
            critical_path: self.critical_path.clone(),
            critical_path_minutes: self.critical_path_minutes,
            dangling_refs: self.dangling_refs.clone(),
            crate_overlaps: self.crate_overlaps.clone(),
        }
    }
}

// ─── Internal helpers ───────────────────────────────────────────────

/// Detect cycles in a dependency graph using DFS coloring.
fn detect_cycles(deps: &BTreeMap<String, BTreeSet<String>>) -> Vec<String> {
    // 0 = unvisited, 1 = in stack, 2 = finished
    let mut state: HashMap<&str, u8> = HashMap::new();
    let mut cycle_nodes: BTreeSet<String> = BTreeSet::new();

    fn dfs<'a>(
        node: &'a str,
        deps: &'a BTreeMap<String, BTreeSet<String>>,
        state: &mut HashMap<&'a str, u8>,
        stack: &mut Vec<&'a str>,
        cycle_nodes: &mut BTreeSet<String>,
    ) {
        state.insert(node, 1);
        stack.push(node);

        if let Some(children) = deps.get(node) {
            for child in children {
                match state.get(child.as_str()).copied().unwrap_or(0) {
                    0 => dfs(child.as_str(), deps, state, stack, cycle_nodes),
                    1 => {
                        // Found a cycle — mark all nodes from child to end of stack.
                        if let Some(pos) = stack.iter().position(|&n| n == child.as_str()) {
                            for &entry in &stack[pos..] {
                                cycle_nodes.insert(entry.to_string());
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        stack.pop();
        state.insert(node, 2);
    }

    for node in deps.keys() {
        if state.get(node.as_str()).copied().unwrap_or(0) == 0 {
            let mut stack = Vec::new();
            dfs(
                node.as_str(),
                deps,
                &mut state,
                &mut stack,
                &mut cycle_nodes,
            );
        }
    }

    cycle_nodes.into_iter().collect()
}

/// Compute the critical path through the plan DAG (longest weighted path).
fn compute_plan_critical_path(
    deps: &BTreeMap<String, BTreeSet<String>>,
    weights: &HashMap<&str, u32>,
) -> (Vec<String>, u32) {
    if deps.is_empty() {
        return (Vec::new(), 0);
    }

    // Build in-degree for topological sort.
    let mut in_degree: HashMap<&str, usize> = HashMap::new();
    let mut dependents: HashMap<&str, Vec<&str>> = HashMap::new();

    for plan_id in deps.keys() {
        in_degree.entry(plan_id.as_str()).or_insert(0);
    }
    for (plan_id, plan_deps) in deps {
        for dep in plan_deps {
            *in_degree.entry(plan_id.as_str()).or_insert(0) += 1;
            dependents
                .entry(dep.as_str())
                .or_default()
                .push(plan_id.as_str());
        }
    }

    // Kahn's topological sort.
    let mut queue: Vec<&str> = in_degree
        .iter()
        .filter(|&(_, &deg)| deg == 0)
        .map(|(id, _)| *id)
        .collect();
    queue.sort();
    let mut topo_order: Vec<&str> = Vec::new();

    while let Some(node) = queue.pop() {
        topo_order.push(node);
        if let Some(children) = dependents.get(node) {
            for &child in children {
                if let Some(deg) = in_degree.get_mut(child) {
                    *deg = deg.saturating_sub(1);
                    if *deg == 0 {
                        queue.push(child);
                        queue.sort();
                    }
                }
            }
        }
    }

    // Forward pass: earliest finish for each plan.
    let mut earliest_finish: HashMap<&str, u32> = HashMap::new();
    let mut predecessor: HashMap<&str, &str> = HashMap::new();

    for &node in &topo_order {
        let weight = weights.get(node).copied().unwrap_or(0);

        let mut max_dep_finish = 0u32;
        let mut best_pred: Option<&str> = None;

        if let Some(node_deps) = deps.get(node) {
            for dep in node_deps {
                if let Some(&dep_finish) = earliest_finish.get(dep.as_str()) {
                    if dep_finish > max_dep_finish {
                        max_dep_finish = dep_finish;
                        best_pred = Some(dep.as_str());
                    }
                }
            }
        }

        let finish = max_dep_finish + weight;
        earliest_finish.insert(node, finish);
        if let Some(pred) = best_pred {
            predecessor.insert(node, pred);
        }
    }

    // Find end of critical path.
    let Some((&end_node, &total_minutes)) = earliest_finish.iter().max_by_key(|&(_, &v)| v) else {
        return (Vec::new(), 0);
    };

    // Trace back.
    let mut path = vec![end_node.to_string()];
    let mut current = end_node;
    while let Some(&pred) = predecessor.get(current) {
        path.push(pred.to_string());
        current = pred;
    }
    path.reverse();

    (path, total_minutes)
}

// ─── Tests ──────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runner::plan_loader::Plan;
    use crate::task_parser::{TaskDef, TaskMeta, TasksFile};

    /// Build a minimal plan with the given ID, task count, and dependencies.
    fn make_plan(id: &str, deps: &[&str], files: &[&str]) -> Plan {
        let tasks: Vec<TaskDef> = vec![TaskDef {
            id: format!("{id}-T1"),
            title: format!("task for {id}"),
            description: None,
            role: None,
            status: "ready".to_string(),
            tier: "focused".to_string(),
            frequency: None,
            model_hint: None,
            replan_strategy: None,
            max_loc: None,
            files: files.iter().map(|s| (*s).to_string()).collect(),
            allowed_tools: None,
            denied_tools: None,
            mcp_servers: None,
            depends_on: vec![],
            depends_on_plan: deps.iter().map(|s| (*s).to_string()).collect(),
            split_into: None,
            context: None,
            verify: vec![],
            timeout_secs: 60,
            max_retries: 1,
            acceptance: vec![],
            acceptance_contract: None,
            domain: None,
            estimated_minutes: Some(10),
            crates_touched: None,
            sequence: 0,
        }];

        Plan {
            id: id.to_string(),
            dir: std::path::PathBuf::from(format!("plans/{id}")),
            tasks: TasksFile {
                meta: TaskMeta {
                    plan: id.to_string(),
                    iteration: 0,
                    total: 1,
                    done: 0,
                    status: String::new(),
                    superseded_by: None,
                    max_parallel: 1,
                    estimated_total_minutes: 10,
                    skip_enrichment: false,
                    source_prd: None,
                },
                tasks,
            },
            prd_excerpt: String::new(),
        }
    }

    #[test]
    fn linear_chain_produces_sequential_waves() {
        // A -> B -> C
        let a = make_plan("A", &[], &[]);
        let b = make_plan("B", &["A"], &[]);
        let c = make_plan("C", &["B"], &[]);

        let dag = CrossPlanDag::compute(&[a, b, c]).unwrap();
        assert_eq!(dag.total_waves(), 3);
        assert_eq!(dag.plans_in_wave(0), &["A"]);
        assert_eq!(dag.plans_in_wave(1), &["B"]);
        assert_eq!(dag.plans_in_wave(2), &["C"]);
    }

    #[test]
    fn diamond_produces_two_waves() {
        // A and B have no deps; C depends on both
        let a = make_plan("A", &[], &[]);
        let b = make_plan("B", &[], &[]);
        let c = make_plan("C", &["A", "B"], &[]);

        let dag = CrossPlanDag::compute(&[a, b, c]).unwrap();
        assert_eq!(dag.total_waves(), 2);
        assert_eq!(dag.plans_in_wave(0), &["A", "B"]);
        assert_eq!(dag.plans_in_wave(1), &["C"]);
    }

    #[test]
    fn independent_plans_all_wave_zero() {
        let a = make_plan("A", &[], &[]);
        let b = make_plan("B", &[], &[]);
        let c = make_plan("C", &[], &[]);

        let dag = CrossPlanDag::compute(&[a, b, c]).unwrap();
        assert_eq!(dag.total_waves(), 1);
        assert_eq!(dag.plans_in_wave(0), &["A", "B", "C"]);
    }

    #[test]
    fn cycle_is_detected() {
        let a = make_plan("A", &["B"], &[]);
        let b = make_plan("B", &["A"], &[]);

        let result = CrossPlanDag::compute(&[a, b]);
        assert!(result.is_err());
        let err = result.unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("cycle"), "expected cycle error, got: {msg}");
    }

    #[test]
    fn dangling_ref_is_reported() {
        let a = make_plan("A", &["nonexistent"], &[]);

        let dag = CrossPlanDag::compute(&[a]).unwrap();
        assert_eq!(dag.dangling_refs.len(), 1);
        assert_eq!(dag.dangling_refs[0].references, "nonexistent");
    }

    #[test]
    fn crate_overlap_detected_in_same_wave() {
        let a = make_plan("A", &[], &["crates/roko-core/src/lib.rs"]);
        let b = make_plan("B", &[], &["crates/roko-core/src/types.rs"]);

        let dag = CrossPlanDag::compute(&[a, b]).unwrap();
        assert_eq!(dag.crate_overlaps.len(), 1);
        assert_eq!(dag.crate_overlaps[0].crate_name, "roko-core");
        assert_eq!(dag.crate_overlaps[0].wave_index, 0);
    }

    #[test]
    fn no_overlap_across_different_waves() {
        let a = make_plan("A", &[], &["crates/roko-core/src/lib.rs"]);
        let b = make_plan("B", &["A"], &["crates/roko-core/src/types.rs"]);

        let dag = CrossPlanDag::compute(&[a, b]).unwrap();
        assert!(dag.crate_overlaps.is_empty());
    }

    #[test]
    fn wave_for_plan_lookup() {
        let a = make_plan("A", &[], &[]);
        let b = make_plan("B", &["A"], &[]);

        let dag = CrossPlanDag::compute(&[a, b]).unwrap();
        assert_eq!(dag.wave_for_plan("A"), Some(0));
        assert_eq!(dag.wave_for_plan("B"), Some(1));
        assert_eq!(dag.wave_for_plan("Z"), None);
    }

    #[test]
    fn summary_has_correct_totals() {
        let a = make_plan("A", &[], &[]);
        let b = make_plan("B", &["A"], &[]);

        let dag = CrossPlanDag::compute(&[a, b]).unwrap();
        let summary = dag.summary();
        assert_eq!(summary.total_plans, 2);
        assert_eq!(summary.total_tasks, 2);
        assert_eq!(summary.total_edges, 1);
        assert_eq!(summary.waves.len(), 2);
        assert_eq!(summary.waves[0].parallelism_width, 1);
        assert_eq!(summary.waves[1].parallelism_width, 1);
    }

    #[test]
    fn critical_path_follows_longest_chain() {
        // A -> B -> C (each 10 min), D independent (10 min)
        let a = make_plan("A", &[], &[]);
        let b = make_plan("B", &["A"], &[]);
        let c = make_plan("C", &["B"], &[]);
        let d = make_plan("D", &[], &[]);

        let dag = CrossPlanDag::compute(&[a, b, c, d]).unwrap();
        assert_eq!(dag.critical_path, vec!["A", "B", "C"]);
        assert_eq!(dag.critical_path_minutes, 30);
    }
}
