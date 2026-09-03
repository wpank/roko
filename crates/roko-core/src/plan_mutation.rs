//! Executor-neutral plan mutation contract (v1).
//!
//! Defines the deterministic mutation kernel that the graph replan controller
//! (and any other executor) uses to transform plans. This module imports no
//! CLI, Runner, HTTP, tool, or Graph controller type.
//!
//! # Design
//!
//! All mutations are atomic: `apply_mutation` clones the base plan, verifies
//! its fingerprint, applies operations in listed order, validates the result,
//! and returns `(new_plan, result)`. Any error returns the original plan
//! unchanged.
//!
//! Fingerprints are canonical BLAKE3 hashes of the plan's deterministic JSON
//! serialization (sorted fields via `BTreeMap`/`BTreeSet`).

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashSet, VecDeque};

// ---------------------------------------------------------------------------
// Author
// ---------------------------------------------------------------------------

/// Who authored this mutation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MutationAuthorV1 {
    /// Whether the author is a human or an automated controller.
    pub kind: MutationAuthorKind,
    /// Opaque identifier for the author (user ID, controller name, etc.).
    pub id: String,
}

/// Discriminator for mutation authorship.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MutationAuthorKind {
    /// Human operator.
    User,
    /// Automated replan controller.
    Controller,
}

// ---------------------------------------------------------------------------
// Evidence
// ---------------------------------------------------------------------------

/// A piece of evidence justifying a mutation (gate failure, user request, etc.).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MutationEvidenceV1 {
    /// Machine-readable code (e.g. `"gate-failure"`, `"user-request"`).
    pub code: String,
    /// Human-readable description.
    pub message: String,
    /// Optional reference to the source of evidence (URL, file path, etc.).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_ref: Option<String>,
    /// Fingerprint of the evidence artifact for integrity verification.
    pub fingerprint: String,
}

// ---------------------------------------------------------------------------
// Mutable plan model
// ---------------------------------------------------------------------------

/// A lightweight, executor-neutral representation of a plan suitable for
/// mutation operations. This is deliberately separate from the CLI `TaskDef`
/// to avoid coupling; #252 owns conversion at the boundary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MutablePlanV1 {
    /// Unique plan identifier.
    pub plan_id: String,
    /// Ordered map of task ID to task definition.
    pub tasks: BTreeMap<String, MutableTaskV1>,
}

/// A single task within a mutable plan.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MutableTaskV1 {
    /// Stable identifier unique within the plan.
    pub id: String,
    /// Human-readable summary.
    pub title: String,
    /// Detailed description of the work.
    pub description: String,
    /// Task IDs that must complete before this one starts.
    pub dependencies: BTreeSet<String>,
    /// Free-form key-value metadata.
    pub metadata: BTreeMap<String, String>,
    /// Whether this task has been completed.
    pub completed: bool,
}

// ---------------------------------------------------------------------------
// Mutation operations
// ---------------------------------------------------------------------------

/// A single atomic operation within a plan mutation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum PlanMutationOpV1 {
    /// Insert a new task into the plan.
    AddTask {
        /// The task to add. Its `id` must not already exist in the plan.
        task: MutableTaskV1,
    },
    /// Remove a task by ID. The task must exist and not be completed.
    /// All incoming references (dependencies pointing to this task) must
    /// be explicitly removed/redirected by prior operations in the same
    /// mutation.
    RemoveTask {
        /// ID of the task to remove.
        task_id: String,
    },
    /// Replace a task with a new definition. The task must exist and not
    /// be completed. The replacement inherits the same task ID slot.
    ReplaceTask {
        /// ID of the task to replace.
        task_id: String,
        /// The replacement task definition. Its `id` field must match `task_id`.
        replacement: MutableTaskV1,
    },
    /// Split one task into multiple parts. The original task must exist and
    /// not be completed. Parts must be nonempty; their dependency wiring
    /// must be explicit in the supplied parts.
    SplitTask {
        /// ID of the task to split (will be removed).
        task_id: String,
        /// The replacement tasks. Each must have a unique ID not already in
        /// the plan (except the original task_id which is being removed).
        parts: Vec<MutableTaskV1>,
    },
    /// Merge two or more pending tasks into one. All source tasks must
    /// exist and not be completed. Requires at least two task IDs.
    MergeTasks {
        /// IDs of the tasks to merge (will be removed).
        task_ids: Vec<String>,
        /// The merged replacement task.
        merged: MutableTaskV1,
    },
    /// Add a dependency edge: `task_id` will depend on `depends_on`.
    AddDependency {
        /// The task gaining a new dependency.
        task_id: String,
        /// The task it will depend on.
        depends_on: String,
    },
    /// Remove a dependency edge.
    RemoveDependency {
        /// The task losing a dependency.
        task_id: String,
        /// The dependency to remove.
        depends_on: String,
    },
}

// ---------------------------------------------------------------------------
// Mutation envelope
// ---------------------------------------------------------------------------

/// A versioned mutation request containing one or more operations.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanMutationV1 {
    /// Schema version; must equal `1`.
    pub schema_version: u8,
    /// Unique identifier for this mutation.
    pub mutation_id: String,
    /// BLAKE3 fingerprint of the base plan this mutation was authored against.
    pub base_fingerprint: String,
    /// Who authored this mutation.
    pub author: MutationAuthorV1,
    /// Evidence justifying the mutation.
    #[serde(default)]
    pub evidence: Vec<MutationEvidenceV1>,
    /// The operations to apply, in order.
    pub operations: Vec<PlanMutationOpV1>,
}

// ---------------------------------------------------------------------------
// Result / Error
// ---------------------------------------------------------------------------

/// Successful result of applying a mutation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanMutationResultV1 {
    /// The mutation ID that produced this result.
    pub mutation_id: String,
    /// Fingerprint of the plan before mutation.
    pub before_fingerprint: String,
    /// Fingerprint of the plan after mutation.
    pub after_fingerprint: String,
    /// Sorted list of task IDs that were added, removed, or modified.
    pub changed_task_ids: Vec<String>,
}

/// Error variants for plan mutation failures.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[non_exhaustive]
pub enum PlanMutationErrorV1 {
    /// Schema version is not supported.
    SchemaIncompatible {
        expected: u8,
        actual: u8,
    },
    /// The base plan fingerprint does not match.
    FingerprintMismatch {
        expected: String,
        actual: String,
    },
    /// A task ID that should exist does not.
    MissingTaskId {
        task_id: String,
    },
    /// A task ID that should be unique already exists.
    DuplicateTaskId {
        task_id: String,
    },
    /// Attempted to mutate a completed task.
    ImmutableCompletedTask {
        task_id: String,
    },
    /// A dependency reference points to a nonexistent task.
    InvalidReference {
        task_id: String,
        references: String,
    },
    /// Split produced zero parts.
    EmptySplit {
        task_id: String,
    },
    /// Merge requires at least two task IDs.
    EmptyMerge,
    /// The resulting plan would exceed the task limit.
    TaskLimitExceeded {
        limit: usize,
        actual: usize,
    },
    /// The resulting DAG contains a cycle.
    CycleDetected {
        /// Task IDs involved in the cycle (best-effort).
        involved: Vec<String>,
    },
    /// Operations list is empty.
    EmptyOperations,
    /// ReplaceTask ID mismatch: the replacement task's id field does not
    /// match the task_id being replaced.
    ReplaceIdMismatch {
        task_id: String,
        replacement_id: String,
    },
}

impl std::fmt::Display for PlanMutationErrorV1 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SchemaIncompatible { expected, actual } => {
                write!(f, "schema version {actual} is not supported (expected {expected})")
            }
            Self::FingerprintMismatch { expected, actual } => {
                write!(
                    f,
                    "base fingerprint mismatch: expected {expected}, got {actual}"
                )
            }
            Self::MissingTaskId { task_id } => {
                write!(f, "task '{task_id}' does not exist in the plan")
            }
            Self::DuplicateTaskId { task_id } => {
                write!(f, "task '{task_id}' already exists in the plan")
            }
            Self::ImmutableCompletedTask { task_id } => {
                write!(f, "task '{task_id}' is completed and cannot be mutated")
            }
            Self::InvalidReference { task_id, references } => {
                write!(
                    f,
                    "task '{task_id}' references nonexistent task '{references}'"
                )
            }
            Self::EmptySplit { task_id } => {
                write!(f, "split of task '{task_id}' produced zero parts")
            }
            Self::EmptyMerge => write!(f, "merge requires at least two task IDs"),
            Self::TaskLimitExceeded { limit, actual } => {
                write!(
                    f,
                    "resulting plan has {actual} tasks, exceeding limit of {limit}"
                )
            }
            Self::CycleDetected { involved } => {
                write!(f, "cycle detected involving tasks: {}", involved.join(", "))
            }
            Self::EmptyOperations => write!(f, "mutation contains no operations"),
            Self::ReplaceIdMismatch {
                task_id,
                replacement_id,
            } => {
                write!(
                    f,
                    "replacement task id '{replacement_id}' does not match target '{task_id}'"
                )
            }
        }
    }
}

impl std::error::Error for PlanMutationErrorV1 {}

// ---------------------------------------------------------------------------
// Canonical fingerprint
// ---------------------------------------------------------------------------

/// Compute the canonical BLAKE3 fingerprint of a plan.
///
/// The fingerprint is computed from a deterministic JSON serialization of
/// the plan's ordered fields (`BTreeMap`/`BTreeSet` ensure key ordering).
#[must_use]
pub fn canonical_fingerprint(plan: &MutablePlanV1) -> String {
    let json = serde_json::to_string(plan).expect("MutablePlanV1 is always serializable");
    let hash = blake3::hash(json.as_bytes());
    hash.to_hex().to_string()
}

// ---------------------------------------------------------------------------
// Cycle detection
// ---------------------------------------------------------------------------

/// Kahn's algorithm for topological sort / cycle detection.
/// Returns `Ok(())` if acyclic, `Err(involved)` with cycle participants otherwise.
fn detect_cycle(tasks: &BTreeMap<String, MutableTaskV1>) -> Result<(), Vec<String>> {
    // Build in-degree map
    let mut in_degree: BTreeMap<&str, usize> = BTreeMap::new();
    let mut adj: BTreeMap<&str, Vec<&str>> = BTreeMap::new();

    for id in tasks.keys() {
        in_degree.entry(id.as_str()).or_insert(0);
        adj.entry(id.as_str()).or_default();
    }

    for task in tasks.values() {
        for dep in &task.dependencies {
            if tasks.contains_key(dep) {
                adj.entry(dep.as_str()).or_default().push(task.id.as_str());
                *in_degree.entry(task.id.as_str()).or_insert(0) += 1;
            }
        }
    }

    let mut queue: VecDeque<&str> = VecDeque::new();
    for (&id, &deg) in &in_degree {
        if deg == 0 {
            queue.push_back(id);
        }
    }

    let mut visited = 0usize;
    while let Some(node) = queue.pop_front() {
        visited += 1;
        if let Some(neighbors) = adj.get(node) {
            for &neighbor in neighbors {
                if let Some(deg) = in_degree.get_mut(neighbor) {
                    *deg -= 1;
                    if *deg == 0 {
                        queue.push_back(neighbor);
                    }
                }
            }
        }
    }

    if visited == tasks.len() {
        Ok(())
    } else {
        // Collect nodes still with nonzero in-degree (cycle participants)
        let involved: Vec<String> = in_degree
            .iter()
            .filter(|(_, &deg)| deg > 0)
            .map(|(&id, _)| id.to_string())
            .collect();
        Err(involved)
    }
}

/// Validate that all dependency references in the plan point to existing tasks.
fn validate_references(tasks: &BTreeMap<String, MutableTaskV1>) -> Result<(), PlanMutationErrorV1> {
    for task in tasks.values() {
        for dep in &task.dependencies {
            if !tasks.contains_key(dep) {
                return Err(PlanMutationErrorV1::InvalidReference {
                    task_id: task.id.clone(),
                    references: dep.clone(),
                });
            }
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// apply_mutation
// ---------------------------------------------------------------------------

/// Apply a mutation to a plan atomically.
///
/// Clones the base plan, verifies its canonical BLAKE3 fingerprint matches
/// `mutation.base_fingerprint`, applies each operation in order, validates
/// the result (references, acyclicity, task limits, completed-task
/// constraints), and returns `(new_plan, result)`.
///
/// On any error the original plan is unchanged and the error is returned.
/// `max_tasks` enforces an upper bound on the number of tasks in the
/// resulting plan.
pub fn apply_mutation(
    base: &MutablePlanV1,
    mutation: &PlanMutationV1,
    max_tasks: usize,
) -> Result<(MutablePlanV1, PlanMutationResultV1), PlanMutationErrorV1> {
    // --- Schema check ---
    if mutation.schema_version != 1 {
        return Err(PlanMutationErrorV1::SchemaIncompatible {
            expected: 1,
            actual: mutation.schema_version,
        });
    }

    // --- Operations must be nonempty ---
    if mutation.operations.is_empty() {
        return Err(PlanMutationErrorV1::EmptyOperations);
    }

    // --- Fingerprint check ---
    let before_fingerprint = canonical_fingerprint(base);
    if before_fingerprint != mutation.base_fingerprint {
        return Err(PlanMutationErrorV1::FingerprintMismatch {
            expected: mutation.base_fingerprint.clone(),
            actual: before_fingerprint,
        });
    }

    // --- Clone and apply operations ---
    let mut plan = base.clone();
    let mut changed_ids: HashSet<String> = HashSet::new();

    for op in &mutation.operations {
        apply_op(&mut plan, op, &mut changed_ids)?;
    }

    // --- Post-validation ---
    // Task limit
    if plan.tasks.len() > max_tasks {
        return Err(PlanMutationErrorV1::TaskLimitExceeded {
            limit: max_tasks,
            actual: plan.tasks.len(),
        });
    }

    // Reference integrity
    validate_references(&plan.tasks)?;

    // Acyclicity
    if let Err(involved) = detect_cycle(&plan.tasks) {
        return Err(PlanMutationErrorV1::CycleDetected { involved });
    }

    // --- Compute result ---
    let after_fingerprint = canonical_fingerprint(&plan);
    let mut sorted_changed: Vec<String> = changed_ids.into_iter().collect();
    sorted_changed.sort();

    let result = PlanMutationResultV1 {
        mutation_id: mutation.mutation_id.clone(),
        before_fingerprint,
        after_fingerprint,
        changed_task_ids: sorted_changed,
    };

    Ok((plan, result))
}

/// Apply a single operation to the working plan, tracking changed IDs.
fn apply_op(
    plan: &mut MutablePlanV1,
    op: &PlanMutationOpV1,
    changed: &mut HashSet<String>,
) -> Result<(), PlanMutationErrorV1> {
    match op {
        PlanMutationOpV1::AddTask { task } => {
            if plan.tasks.contains_key(&task.id) {
                return Err(PlanMutationErrorV1::DuplicateTaskId {
                    task_id: task.id.clone(),
                });
            }
            changed.insert(task.id.clone());
            plan.tasks.insert(task.id.clone(), task.clone());
        }

        PlanMutationOpV1::RemoveTask { task_id } => {
            let existing = plan
                .tasks
                .get(task_id)
                .ok_or_else(|| PlanMutationErrorV1::MissingTaskId {
                    task_id: task_id.clone(),
                })?;
            if existing.completed {
                return Err(PlanMutationErrorV1::ImmutableCompletedTask {
                    task_id: task_id.clone(),
                });
            }
            // Check that no remaining task depends on this one
            for other in plan.tasks.values() {
                if other.id != *task_id && other.dependencies.contains(task_id) {
                    return Err(PlanMutationErrorV1::InvalidReference {
                        task_id: other.id.clone(),
                        references: task_id.clone(),
                    });
                }
            }
            changed.insert(task_id.clone());
            plan.tasks.remove(task_id);
        }

        PlanMutationOpV1::ReplaceTask {
            task_id,
            replacement,
        } => {
            // Replacement ID must match the target slot
            if replacement.id != *task_id {
                return Err(PlanMutationErrorV1::ReplaceIdMismatch {
                    task_id: task_id.clone(),
                    replacement_id: replacement.id.clone(),
                });
            }
            let existing = plan
                .tasks
                .get(task_id)
                .ok_or_else(|| PlanMutationErrorV1::MissingTaskId {
                    task_id: task_id.clone(),
                })?;
            if existing.completed {
                return Err(PlanMutationErrorV1::ImmutableCompletedTask {
                    task_id: task_id.clone(),
                });
            }
            changed.insert(task_id.clone());
            plan.tasks.insert(task_id.clone(), replacement.clone());
        }

        PlanMutationOpV1::SplitTask { task_id, parts } => {
            if parts.is_empty() {
                return Err(PlanMutationErrorV1::EmptySplit {
                    task_id: task_id.clone(),
                });
            }
            let existing = plan
                .tasks
                .get(task_id)
                .ok_or_else(|| PlanMutationErrorV1::MissingTaskId {
                    task_id: task_id.clone(),
                })?;
            if existing.completed {
                return Err(PlanMutationErrorV1::ImmutableCompletedTask {
                    task_id: task_id.clone(),
                });
            }
            // Check for duplicate IDs among parts and against existing tasks
            // (excluding the task being split)
            let mut seen: HashSet<&str> = HashSet::new();
            for part in parts {
                if !seen.insert(part.id.as_str()) {
                    return Err(PlanMutationErrorV1::DuplicateTaskId {
                        task_id: part.id.clone(),
                    });
                }
                if part.id != *task_id && plan.tasks.contains_key(&part.id) {
                    return Err(PlanMutationErrorV1::DuplicateTaskId {
                        task_id: part.id.clone(),
                    });
                }
            }
            // Remove original, add parts
            changed.insert(task_id.clone());
            plan.tasks.remove(task_id);
            for part in parts {
                changed.insert(part.id.clone());
                plan.tasks.insert(part.id.clone(), part.clone());
            }
        }

        PlanMutationOpV1::MergeTasks { task_ids, merged } => {
            if task_ids.len() < 2 {
                return Err(PlanMutationErrorV1::EmptyMerge);
            }
            // All source tasks must exist and be pending
            for tid in task_ids {
                let existing = plan
                    .tasks
                    .get(tid)
                    .ok_or_else(|| PlanMutationErrorV1::MissingTaskId {
                        task_id: tid.clone(),
                    })?;
                if existing.completed {
                    return Err(PlanMutationErrorV1::ImmutableCompletedTask {
                        task_id: tid.clone(),
                    });
                }
            }
            // Check merged ID doesn't collide with non-source tasks
            let source_set: HashSet<&str> = task_ids.iter().map(|s| s.as_str()).collect();
            if !source_set.contains(merged.id.as_str())
                && plan.tasks.contains_key(&merged.id)
            {
                return Err(PlanMutationErrorV1::DuplicateTaskId {
                    task_id: merged.id.clone(),
                });
            }
            // Remove sources, add merged
            for tid in task_ids {
                changed.insert(tid.clone());
                plan.tasks.remove(tid);
            }
            changed.insert(merged.id.clone());
            plan.tasks.insert(merged.id.clone(), merged.clone());
        }

        PlanMutationOpV1::AddDependency {
            task_id,
            depends_on,
        } => {
            let task = plan
                .tasks
                .get_mut(task_id)
                .ok_or_else(|| PlanMutationErrorV1::MissingTaskId {
                    task_id: task_id.clone(),
                })?;
            changed.insert(task_id.clone());
            task.dependencies.insert(depends_on.clone());
        }

        PlanMutationOpV1::RemoveDependency {
            task_id,
            depends_on,
        } => {
            let task = plan
                .tasks
                .get_mut(task_id)
                .ok_or_else(|| PlanMutationErrorV1::MissingTaskId {
                    task_id: task_id.clone(),
                })?;
            changed.insert(task_id.clone());
            task.dependencies.remove(depends_on);
        }
    }

    Ok(())
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // -- Helpers --

    fn task(id: &str, title: &str) -> MutableTaskV1 {
        MutableTaskV1 {
            id: id.into(),
            title: title.into(),
            description: String::new(),
            dependencies: BTreeSet::new(),
            metadata: BTreeMap::new(),
            completed: false,
        }
    }

    fn task_with_deps(id: &str, title: &str, deps: &[&str]) -> MutableTaskV1 {
        let mut t = task(id, title);
        t.dependencies = deps.iter().map(|s| s.to_string()).collect();
        t
    }

    fn completed_task(id: &str, title: &str) -> MutableTaskV1 {
        let mut t = task(id, title);
        t.completed = true;
        t
    }

    fn simple_plan(tasks: Vec<MutableTaskV1>) -> MutablePlanV1 {
        let mut map = BTreeMap::new();
        for t in tasks {
            map.insert(t.id.clone(), t);
        }
        MutablePlanV1 {
            plan_id: "test-plan".into(),
            tasks: map,
        }
    }

    fn mutation(
        base: &MutablePlanV1,
        ops: Vec<PlanMutationOpV1>,
    ) -> PlanMutationV1 {
        PlanMutationV1 {
            schema_version: 1,
            mutation_id: "mut-1".into(),
            base_fingerprint: canonical_fingerprint(base),
            author: MutationAuthorV1 {
                kind: MutationAuthorKind::Controller,
                id: "test".into(),
            },
            evidence: vec![],
            operations: ops,
        }
    }

    // -----------------------------------------------------------------------
    // Fingerprint determinism
    // -----------------------------------------------------------------------

    #[test]
    fn fingerprint_is_deterministic() {
        let plan = simple_plan(vec![
            task("t1", "first"),
            task("t2", "second"),
        ]);
        let fp1 = canonical_fingerprint(&plan);
        let fp2 = canonical_fingerprint(&plan);
        assert_eq!(fp1, fp2);
        // BLAKE3 hex is 64 chars
        assert_eq!(fp1.len(), 64);
    }

    #[test]
    fn fingerprint_changes_with_content() {
        let plan1 = simple_plan(vec![task("t1", "first")]);
        let plan2 = simple_plan(vec![task("t1", "different")]);
        assert_ne!(canonical_fingerprint(&plan1), canonical_fingerprint(&plan2));
    }

    #[test]
    fn fingerprint_order_independent_of_insertion() {
        // BTreeMap ensures key ordering regardless of insertion order
        let mut tasks1 = BTreeMap::new();
        tasks1.insert("b".to_string(), task("b", "beta"));
        tasks1.insert("a".to_string(), task("a", "alpha"));

        let mut tasks2 = BTreeMap::new();
        tasks2.insert("a".to_string(), task("a", "alpha"));
        tasks2.insert("b".to_string(), task("b", "beta"));

        let plan1 = MutablePlanV1 {
            plan_id: "p".into(),
            tasks: tasks1,
        };
        let plan2 = MutablePlanV1 {
            plan_id: "p".into(),
            tasks: tasks2,
        };
        assert_eq!(canonical_fingerprint(&plan1), canonical_fingerprint(&plan2));
    }

    // -----------------------------------------------------------------------
    // Schema validation
    // -----------------------------------------------------------------------

    #[test]
    fn rejects_wrong_schema_version() {
        let plan = simple_plan(vec![task("t1", "x")]);
        let mut m = mutation(&plan, vec![PlanMutationOpV1::RemoveTask {
            task_id: "t1".into(),
        }]);
        m.schema_version = 2;
        let err = apply_mutation(&plan, &m, 100).unwrap_err();
        assert!(matches!(
            err,
            PlanMutationErrorV1::SchemaIncompatible {
                expected: 1,
                actual: 2
            }
        ));
    }

    #[test]
    fn rejects_empty_operations() {
        let plan = simple_plan(vec![task("t1", "x")]);
        let m = mutation(&plan, vec![]);
        let err = apply_mutation(&plan, &m, 100).unwrap_err();
        assert!(matches!(err, PlanMutationErrorV1::EmptyOperations));
    }

    // -----------------------------------------------------------------------
    // Fingerprint mismatch
    // -----------------------------------------------------------------------

    #[test]
    fn rejects_fingerprint_mismatch() {
        let plan = simple_plan(vec![task("t1", "x")]);
        let mut m = mutation(&plan, vec![PlanMutationOpV1::RemoveTask {
            task_id: "t1".into(),
        }]);
        m.base_fingerprint = "0".repeat(64);
        let err = apply_mutation(&plan, &m, 100).unwrap_err();
        assert!(matches!(err, PlanMutationErrorV1::FingerprintMismatch { .. }));
    }

    // -----------------------------------------------------------------------
    // AddTask
    // -----------------------------------------------------------------------

    #[test]
    fn add_task_succeeds() {
        let plan = simple_plan(vec![task("t1", "first")]);
        let m = mutation(
            &plan,
            vec![PlanMutationOpV1::AddTask {
                task: task("t2", "second"),
            }],
        );
        let (new_plan, result) = apply_mutation(&plan, &m, 100).unwrap();
        assert_eq!(new_plan.tasks.len(), 2);
        assert!(new_plan.tasks.contains_key("t2"));
        assert_eq!(result.changed_task_ids, vec!["t2"]);
        assert_ne!(result.before_fingerprint, result.after_fingerprint);
    }

    #[test]
    fn add_task_rejects_duplicate() {
        let plan = simple_plan(vec![task("t1", "first")]);
        let m = mutation(
            &plan,
            vec![PlanMutationOpV1::AddTask {
                task: task("t1", "dupe"),
            }],
        );
        let err = apply_mutation(&plan, &m, 100).unwrap_err();
        assert!(matches!(
            err,
            PlanMutationErrorV1::DuplicateTaskId { task_id } if task_id == "t1"
        ));
    }

    // -----------------------------------------------------------------------
    // RemoveTask
    // -----------------------------------------------------------------------

    #[test]
    fn remove_task_succeeds() {
        let plan = simple_plan(vec![task("t1", "first"), task("t2", "second")]);
        let m = mutation(
            &plan,
            vec![PlanMutationOpV1::RemoveTask {
                task_id: "t1".into(),
            }],
        );
        let (new_plan, result) = apply_mutation(&plan, &m, 100).unwrap();
        assert_eq!(new_plan.tasks.len(), 1);
        assert!(!new_plan.tasks.contains_key("t1"));
        assert_eq!(result.changed_task_ids, vec!["t1"]);
    }

    #[test]
    fn remove_task_rejects_missing() {
        let plan = simple_plan(vec![task("t1", "first")]);
        let m = mutation(
            &plan,
            vec![PlanMutationOpV1::RemoveTask {
                task_id: "nonexistent".into(),
            }],
        );
        let err = apply_mutation(&plan, &m, 100).unwrap_err();
        assert!(matches!(err, PlanMutationErrorV1::MissingTaskId { .. }));
    }

    #[test]
    fn remove_task_rejects_completed() {
        let plan = simple_plan(vec![completed_task("t1", "done")]);
        let m = mutation(
            &plan,
            vec![PlanMutationOpV1::RemoveTask {
                task_id: "t1".into(),
            }],
        );
        let err = apply_mutation(&plan, &m, 100).unwrap_err();
        assert!(matches!(
            err,
            PlanMutationErrorV1::ImmutableCompletedTask { .. }
        ));
    }

    #[test]
    fn remove_task_rejects_dangling_references() {
        let plan = simple_plan(vec![
            task("t1", "first"),
            task_with_deps("t2", "second", &["t1"]),
        ]);
        let m = mutation(
            &plan,
            vec![PlanMutationOpV1::RemoveTask {
                task_id: "t1".into(),
            }],
        );
        let err = apply_mutation(&plan, &m, 100).unwrap_err();
        assert!(matches!(err, PlanMutationErrorV1::InvalidReference { .. }));
    }

    #[test]
    fn remove_task_with_redirected_deps() {
        // Remove t1 after redirecting t2's dependency to t3
        let plan = simple_plan(vec![
            task("t1", "first"),
            task_with_deps("t2", "second", &["t1"]),
            task("t3", "third"),
        ]);
        let m = mutation(
            &plan,
            vec![
                PlanMutationOpV1::RemoveDependency {
                    task_id: "t2".into(),
                    depends_on: "t1".into(),
                },
                PlanMutationOpV1::AddDependency {
                    task_id: "t2".into(),
                    depends_on: "t3".into(),
                },
                PlanMutationOpV1::RemoveTask {
                    task_id: "t1".into(),
                },
            ],
        );
        let (new_plan, result) = apply_mutation(&plan, &m, 100).unwrap();
        assert_eq!(new_plan.tasks.len(), 2);
        assert!(
            new_plan
                .tasks
                .get("t2")
                .unwrap()
                .dependencies
                .contains("t3")
        );
        let mut expected_changed = vec!["t1".to_string(), "t2".to_string()];
        expected_changed.sort();
        assert_eq!(result.changed_task_ids, expected_changed);
    }

    // -----------------------------------------------------------------------
    // ReplaceTask
    // -----------------------------------------------------------------------

    #[test]
    fn replace_task_succeeds() {
        let plan = simple_plan(vec![task("t1", "original")]);
        let mut replacement = task("t1", "replaced");
        replacement.description = "new description".into();
        let m = mutation(
            &plan,
            vec![PlanMutationOpV1::ReplaceTask {
                task_id: "t1".into(),
                replacement,
            }],
        );
        let (new_plan, result) = apply_mutation(&plan, &m, 100).unwrap();
        assert_eq!(new_plan.tasks.get("t1").unwrap().title, "replaced");
        assert_eq!(result.changed_task_ids, vec!["t1"]);
    }

    #[test]
    fn replace_task_rejects_id_mismatch() {
        let plan = simple_plan(vec![task("t1", "original")]);
        let m = mutation(
            &plan,
            vec![PlanMutationOpV1::ReplaceTask {
                task_id: "t1".into(),
                replacement: task("t2", "wrong id"),
            }],
        );
        let err = apply_mutation(&plan, &m, 100).unwrap_err();
        assert!(matches!(
            err,
            PlanMutationErrorV1::ReplaceIdMismatch { .. }
        ));
    }

    #[test]
    fn replace_task_rejects_completed() {
        let plan = simple_plan(vec![completed_task("t1", "done")]);
        let m = mutation(
            &plan,
            vec![PlanMutationOpV1::ReplaceTask {
                task_id: "t1".into(),
                replacement: task("t1", "new"),
            }],
        );
        let err = apply_mutation(&plan, &m, 100).unwrap_err();
        assert!(matches!(
            err,
            PlanMutationErrorV1::ImmutableCompletedTask { .. }
        ));
    }

    #[test]
    fn replace_task_rejects_missing() {
        let plan = simple_plan(vec![task("t1", "first")]);
        let m = mutation(
            &plan,
            vec![PlanMutationOpV1::ReplaceTask {
                task_id: "t2".into(),
                replacement: task("t2", "ghost"),
            }],
        );
        let err = apply_mutation(&plan, &m, 100).unwrap_err();
        assert!(matches!(err, PlanMutationErrorV1::MissingTaskId { .. }));
    }

    // -----------------------------------------------------------------------
    // SplitTask
    // -----------------------------------------------------------------------

    #[test]
    fn split_task_succeeds() {
        let plan = simple_plan(vec![task("t1", "big task")]);
        let m = mutation(
            &plan,
            vec![PlanMutationOpV1::SplitTask {
                task_id: "t1".into(),
                parts: vec![
                    task("t1a", "part a"),
                    task_with_deps("t1b", "part b", &["t1a"]),
                ],
            }],
        );
        let (new_plan, result) = apply_mutation(&plan, &m, 100).unwrap();
        assert_eq!(new_plan.tasks.len(), 2);
        assert!(!new_plan.tasks.contains_key("t1"));
        assert!(new_plan.tasks.contains_key("t1a"));
        assert!(new_plan.tasks.contains_key("t1b"));
        let mut expected = vec!["t1".to_string(), "t1a".to_string(), "t1b".to_string()];
        expected.sort();
        assert_eq!(result.changed_task_ids, expected);
    }

    #[test]
    fn split_task_rejects_empty_parts() {
        let plan = simple_plan(vec![task("t1", "big")]);
        let m = mutation(
            &plan,
            vec![PlanMutationOpV1::SplitTask {
                task_id: "t1".into(),
                parts: vec![],
            }],
        );
        let err = apply_mutation(&plan, &m, 100).unwrap_err();
        assert!(matches!(err, PlanMutationErrorV1::EmptySplit { .. }));
    }

    #[test]
    fn split_task_rejects_completed() {
        let plan = simple_plan(vec![completed_task("t1", "done")]);
        let m = mutation(
            &plan,
            vec![PlanMutationOpV1::SplitTask {
                task_id: "t1".into(),
                parts: vec![task("t1a", "a")],
            }],
        );
        let err = apply_mutation(&plan, &m, 100).unwrap_err();
        assert!(matches!(
            err,
            PlanMutationErrorV1::ImmutableCompletedTask { .. }
        ));
    }

    #[test]
    fn split_task_rejects_duplicate_part_ids() {
        let plan = simple_plan(vec![task("t1", "big")]);
        let m = mutation(
            &plan,
            vec![PlanMutationOpV1::SplitTask {
                task_id: "t1".into(),
                parts: vec![task("t1a", "a"), task("t1a", "duplicate")],
            }],
        );
        let err = apply_mutation(&plan, &m, 100).unwrap_err();
        assert!(matches!(
            err,
            PlanMutationErrorV1::DuplicateTaskId { task_id } if task_id == "t1a"
        ));
    }

    #[test]
    fn split_task_rejects_colliding_ids() {
        let plan = simple_plan(vec![task("t1", "big"), task("t2", "existing")]);
        let m = mutation(
            &plan,
            vec![PlanMutationOpV1::SplitTask {
                task_id: "t1".into(),
                parts: vec![task("t2", "collides with existing")],
            }],
        );
        let err = apply_mutation(&plan, &m, 100).unwrap_err();
        assert!(matches!(
            err,
            PlanMutationErrorV1::DuplicateTaskId { task_id } if task_id == "t2"
        ));
    }

    #[test]
    fn split_task_can_reuse_original_id() {
        // Splitting t1 into [t1, t1a] is allowed: the original t1 slot
        // is vacated before parts are inserted.
        let plan = simple_plan(vec![task("t1", "big")]);
        let m = mutation(
            &plan,
            vec![PlanMutationOpV1::SplitTask {
                task_id: "t1".into(),
                parts: vec![task("t1", "part 1 reuses id"), task("t1a", "part 2")],
            }],
        );
        let (new_plan, _) = apply_mutation(&plan, &m, 100).unwrap();
        assert_eq!(new_plan.tasks.len(), 2);
        assert_eq!(
            new_plan.tasks.get("t1").unwrap().title,
            "part 1 reuses id"
        );
    }

    // -----------------------------------------------------------------------
    // MergeTasks
    // -----------------------------------------------------------------------

    #[test]
    fn merge_tasks_succeeds() {
        let plan = simple_plan(vec![task("t1", "a"), task("t2", "b"), task("t3", "c")]);
        let m = mutation(
            &plan,
            vec![PlanMutationOpV1::MergeTasks {
                task_ids: vec!["t1".into(), "t2".into()],
                merged: task("t12", "merged"),
            }],
        );
        let (new_plan, result) = apply_mutation(&plan, &m, 100).unwrap();
        assert_eq!(new_plan.tasks.len(), 2); // t3 + t12
        assert!(new_plan.tasks.contains_key("t12"));
        assert!(new_plan.tasks.contains_key("t3"));
        let mut expected = vec![
            "t1".to_string(),
            "t12".to_string(),
            "t2".to_string(),
        ];
        expected.sort();
        assert_eq!(result.changed_task_ids, expected);
    }

    #[test]
    fn merge_tasks_rejects_fewer_than_two() {
        let plan = simple_plan(vec![task("t1", "a")]);
        let m = mutation(
            &plan,
            vec![PlanMutationOpV1::MergeTasks {
                task_ids: vec!["t1".into()],
                merged: task("t1m", "merged"),
            }],
        );
        let err = apply_mutation(&plan, &m, 100).unwrap_err();
        assert!(matches!(err, PlanMutationErrorV1::EmptyMerge));
    }

    #[test]
    fn merge_tasks_rejects_completed() {
        let plan = simple_plan(vec![completed_task("t1", "done"), task("t2", "pending")]);
        let m = mutation(
            &plan,
            vec![PlanMutationOpV1::MergeTasks {
                task_ids: vec!["t1".into(), "t2".into()],
                merged: task("t12", "merged"),
            }],
        );
        let err = apply_mutation(&plan, &m, 100).unwrap_err();
        assert!(matches!(
            err,
            PlanMutationErrorV1::ImmutableCompletedTask { .. }
        ));
    }

    #[test]
    fn merge_tasks_rejects_missing() {
        let plan = simple_plan(vec![task("t1", "a")]);
        let m = mutation(
            &plan,
            vec![PlanMutationOpV1::MergeTasks {
                task_ids: vec!["t1".into(), "ghost".into()],
                merged: task("t1g", "merged"),
            }],
        );
        let err = apply_mutation(&plan, &m, 100).unwrap_err();
        assert!(matches!(err, PlanMutationErrorV1::MissingTaskId { .. }));
    }

    #[test]
    fn merge_tasks_can_reuse_source_id() {
        let plan = simple_plan(vec![task("t1", "a"), task("t2", "b")]);
        let m = mutation(
            &plan,
            vec![PlanMutationOpV1::MergeTasks {
                task_ids: vec!["t1".into(), "t2".into()],
                merged: task("t1", "merged reusing t1"),
            }],
        );
        let (new_plan, _) = apply_mutation(&plan, &m, 100).unwrap();
        assert_eq!(new_plan.tasks.len(), 1);
        assert_eq!(
            new_plan.tasks.get("t1").unwrap().title,
            "merged reusing t1"
        );
    }

    #[test]
    fn merge_rejects_colliding_id() {
        let plan = simple_plan(vec![
            task("t1", "a"),
            task("t2", "b"),
            task("t3", "c"),
        ]);
        let m = mutation(
            &plan,
            vec![PlanMutationOpV1::MergeTasks {
                task_ids: vec!["t1".into(), "t2".into()],
                merged: task("t3", "collides"),
            }],
        );
        let err = apply_mutation(&plan, &m, 100).unwrap_err();
        assert!(matches!(
            err,
            PlanMutationErrorV1::DuplicateTaskId { task_id } if task_id == "t3"
        ));
    }

    // -----------------------------------------------------------------------
    // AddDependency / RemoveDependency
    // -----------------------------------------------------------------------

    #[test]
    fn add_dependency_succeeds() {
        let plan = simple_plan(vec![task("t1", "a"), task("t2", "b")]);
        let m = mutation(
            &plan,
            vec![PlanMutationOpV1::AddDependency {
                task_id: "t2".into(),
                depends_on: "t1".into(),
            }],
        );
        let (new_plan, _) = apply_mutation(&plan, &m, 100).unwrap();
        assert!(
            new_plan
                .tasks
                .get("t2")
                .unwrap()
                .dependencies
                .contains("t1")
        );
    }

    #[test]
    fn add_dependency_rejects_missing_task() {
        let plan = simple_plan(vec![task("t1", "a")]);
        let m = mutation(
            &plan,
            vec![PlanMutationOpV1::AddDependency {
                task_id: "ghost".into(),
                depends_on: "t1".into(),
            }],
        );
        let err = apply_mutation(&plan, &m, 100).unwrap_err();
        assert!(matches!(err, PlanMutationErrorV1::MissingTaskId { .. }));
    }

    #[test]
    fn remove_dependency_succeeds() {
        let plan = simple_plan(vec![
            task("t1", "a"),
            task_with_deps("t2", "b", &["t1"]),
        ]);
        let m = mutation(
            &plan,
            vec![PlanMutationOpV1::RemoveDependency {
                task_id: "t2".into(),
                depends_on: "t1".into(),
            }],
        );
        let (new_plan, _) = apply_mutation(&plan, &m, 100).unwrap();
        assert!(
            new_plan
                .tasks
                .get("t2")
                .unwrap()
                .dependencies
                .is_empty()
        );
    }

    // -----------------------------------------------------------------------
    // Cycle detection
    // -----------------------------------------------------------------------

    #[test]
    fn rejects_direct_cycle() {
        let plan = simple_plan(vec![task("t1", "a"), task("t2", "b")]);
        let m = mutation(
            &plan,
            vec![
                PlanMutationOpV1::AddDependency {
                    task_id: "t1".into(),
                    depends_on: "t2".into(),
                },
                PlanMutationOpV1::AddDependency {
                    task_id: "t2".into(),
                    depends_on: "t1".into(),
                },
            ],
        );
        let err = apply_mutation(&plan, &m, 100).unwrap_err();
        assert!(matches!(err, PlanMutationErrorV1::CycleDetected { .. }));
    }

    #[test]
    fn rejects_transitive_cycle() {
        let plan = simple_plan(vec![
            task("t1", "a"),
            task("t2", "b"),
            task("t3", "c"),
        ]);
        let m = mutation(
            &plan,
            vec![
                PlanMutationOpV1::AddDependency {
                    task_id: "t2".into(),
                    depends_on: "t1".into(),
                },
                PlanMutationOpV1::AddDependency {
                    task_id: "t3".into(),
                    depends_on: "t2".into(),
                },
                PlanMutationOpV1::AddDependency {
                    task_id: "t1".into(),
                    depends_on: "t3".into(),
                },
            ],
        );
        let err = apply_mutation(&plan, &m, 100).unwrap_err();
        match err {
            PlanMutationErrorV1::CycleDetected { involved } => {
                assert_eq!(involved.len(), 3);
            }
            other => panic!("expected CycleDetected, got {other:?}"),
        }
    }

    #[test]
    fn self_referencing_dependency_is_cycle() {
        let plan = simple_plan(vec![task("t1", "a")]);
        let m = mutation(
            &plan,
            vec![PlanMutationOpV1::AddDependency {
                task_id: "t1".into(),
                depends_on: "t1".into(),
            }],
        );
        let err = apply_mutation(&plan, &m, 100).unwrap_err();
        assert!(matches!(err, PlanMutationErrorV1::CycleDetected { .. }));
    }

    // -----------------------------------------------------------------------
    // Task limit
    // -----------------------------------------------------------------------

    #[test]
    fn rejects_exceeding_task_limit() {
        let plan = simple_plan(vec![task("t1", "a")]);
        let m = mutation(
            &plan,
            vec![
                PlanMutationOpV1::AddTask {
                    task: task("t2", "b"),
                },
                PlanMutationOpV1::AddTask {
                    task: task("t3", "c"),
                },
            ],
        );
        let err = apply_mutation(&plan, &m, 2).unwrap_err();
        assert!(matches!(
            err,
            PlanMutationErrorV1::TaskLimitExceeded { limit: 2, actual: 3 }
        ));
    }

    // -----------------------------------------------------------------------
    // Invalid reference post-validation
    // -----------------------------------------------------------------------

    #[test]
    fn rejects_dangling_dep_after_add_task() {
        let plan = simple_plan(vec![task("t1", "a")]);
        let m = mutation(
            &plan,
            vec![PlanMutationOpV1::AddTask {
                task: task_with_deps("t2", "b", &["ghost"]),
            }],
        );
        let err = apply_mutation(&plan, &m, 100).unwrap_err();
        assert!(matches!(err, PlanMutationErrorV1::InvalidReference { .. }));
    }

    // -----------------------------------------------------------------------
    // Atomicity: original plan unchanged on error
    // -----------------------------------------------------------------------

    #[test]
    fn original_plan_unchanged_on_error() {
        let plan = simple_plan(vec![task("t1", "a"), task("t2", "b")]);
        let original_fp = canonical_fingerprint(&plan);

        // This mutation adds t3 then tries to remove nonexistent t99
        let m = mutation(
            &plan,
            vec![
                PlanMutationOpV1::AddTask {
                    task: task("t3", "c"),
                },
                PlanMutationOpV1::RemoveTask {
                    task_id: "t99".into(),
                },
            ],
        );
        let err = apply_mutation(&plan, &m, 100);
        assert!(err.is_err());
        // Original plan is unchanged
        assert_eq!(canonical_fingerprint(&plan), original_fp);
        assert_eq!(plan.tasks.len(), 2);
    }

    // -----------------------------------------------------------------------
    // Multi-operation mutations
    // -----------------------------------------------------------------------

    #[test]
    fn complex_multi_op_mutation() {
        let plan = simple_plan(vec![
            task("t1", "setup"),
            task_with_deps("t2", "implement", &["t1"]),
            task_with_deps("t3", "test", &["t2"]),
        ]);
        let m = mutation(
            &plan,
            vec![
                // Add a new task
                PlanMutationOpV1::AddTask {
                    task: task("t4", "docs"),
                },
                // Wire t4 after t3
                PlanMutationOpV1::AddDependency {
                    task_id: "t4".into(),
                    depends_on: "t3".into(),
                },
                // Replace t2 with a better description
                PlanMutationOpV1::ReplaceTask {
                    task_id: "t2".into(),
                    replacement: {
                        let mut r = task_with_deps("t2", "implement (revised)", &["t1"]);
                        r.description = "revised implementation".into();
                        r
                    },
                },
            ],
        );
        let (new_plan, result) = apply_mutation(&plan, &m, 100).unwrap();
        assert_eq!(new_plan.tasks.len(), 4);
        assert_eq!(
            new_plan.tasks.get("t2").unwrap().title,
            "implement (revised)"
        );
        assert!(
            new_plan
                .tasks
                .get("t4")
                .unwrap()
                .dependencies
                .contains("t3")
        );
        assert_eq!(result.mutation_id, "mut-1");
        // t2 and t4 were changed
        assert!(result.changed_task_ids.contains(&"t2".to_string()));
        assert!(result.changed_task_ids.contains(&"t4".to_string()));
    }

    // -----------------------------------------------------------------------
    // Serde golden tests
    // -----------------------------------------------------------------------

    #[test]
    fn mutation_serde_roundtrip() {
        let plan = simple_plan(vec![task("t1", "a")]);
        let m = PlanMutationV1 {
            schema_version: 1,
            mutation_id: "mut-golden".into(),
            base_fingerprint: canonical_fingerprint(&plan),
            author: MutationAuthorV1 {
                kind: MutationAuthorKind::User,
                id: "alice".into(),
            },
            evidence: vec![MutationEvidenceV1 {
                code: "gate-failure".into(),
                message: "clippy failed".into(),
                source_ref: Some("run-123".into()),
                fingerprint: "abc123".into(),
            }],
            operations: vec![
                PlanMutationOpV1::AddTask {
                    task: task("t2", "new"),
                },
                PlanMutationOpV1::AddDependency {
                    task_id: "t2".into(),
                    depends_on: "t1".into(),
                },
            ],
        };

        let json = serde_json::to_string_pretty(&m).unwrap();
        let parsed: PlanMutationV1 = serde_json::from_str(&json).unwrap();
        assert_eq!(m, parsed);
    }

    #[test]
    fn result_serde_roundtrip() {
        let r = PlanMutationResultV1 {
            mutation_id: "m1".into(),
            before_fingerprint: "a".repeat(64),
            after_fingerprint: "b".repeat(64),
            changed_task_ids: vec!["t1".into(), "t2".into()],
        };
        let json = serde_json::to_string(&r).unwrap();
        let parsed: PlanMutationResultV1 = serde_json::from_str(&json).unwrap();
        assert_eq!(r, parsed);
    }

    #[test]
    fn error_serde_roundtrip() {
        let errors = vec![
            PlanMutationErrorV1::SchemaIncompatible {
                expected: 1,
                actual: 2,
            },
            PlanMutationErrorV1::FingerprintMismatch {
                expected: "a".into(),
                actual: "b".into(),
            },
            PlanMutationErrorV1::MissingTaskId {
                task_id: "t1".into(),
            },
            PlanMutationErrorV1::DuplicateTaskId {
                task_id: "t1".into(),
            },
            PlanMutationErrorV1::ImmutableCompletedTask {
                task_id: "t1".into(),
            },
            PlanMutationErrorV1::InvalidReference {
                task_id: "t2".into(),
                references: "t1".into(),
            },
            PlanMutationErrorV1::EmptySplit {
                task_id: "t1".into(),
            },
            PlanMutationErrorV1::EmptyMerge,
            PlanMutationErrorV1::TaskLimitExceeded {
                limit: 10,
                actual: 20,
            },
            PlanMutationErrorV1::CycleDetected {
                involved: vec!["t1".into(), "t2".into()],
            },
            PlanMutationErrorV1::EmptyOperations,
            PlanMutationErrorV1::ReplaceIdMismatch {
                task_id: "t1".into(),
                replacement_id: "t2".into(),
            },
        ];
        for e in &errors {
            let json = serde_json::to_string(e).unwrap();
            let parsed: PlanMutationErrorV1 = serde_json::from_str(&json).unwrap();
            assert_eq!(&parsed, e, "roundtrip failed for {e:?}");
        }
    }

    #[test]
    fn plan_serde_roundtrip() {
        let plan = simple_plan(vec![
            task("t1", "setup"),
            task_with_deps("t2", "implement", &["t1"]),
        ]);
        let json = serde_json::to_string_pretty(&plan).unwrap();
        let parsed: MutablePlanV1 = serde_json::from_str(&json).unwrap();
        assert_eq!(plan, parsed);
        assert_eq!(
            canonical_fingerprint(&plan),
            canonical_fingerprint(&parsed)
        );
    }

    #[test]
    fn op_serde_tagged() {
        let op = PlanMutationOpV1::AddTask {
            task: task("t1", "a"),
        };
        let json = serde_json::to_string(&op).unwrap();
        assert!(json.contains(r#""op":"add_task""#));

        let op2 = PlanMutationOpV1::RemoveTask {
            task_id: "t1".into(),
        };
        let json2 = serde_json::to_string(&op2).unwrap();
        assert!(json2.contains(r#""op":"remove_task""#));
    }

    // -----------------------------------------------------------------------
    // Error Display
    // -----------------------------------------------------------------------

    #[test]
    fn error_display_covers_all_variants() {
        let errors = vec![
            PlanMutationErrorV1::SchemaIncompatible {
                expected: 1,
                actual: 2,
            },
            PlanMutationErrorV1::FingerprintMismatch {
                expected: "a".into(),
                actual: "b".into(),
            },
            PlanMutationErrorV1::MissingTaskId {
                task_id: "t1".into(),
            },
            PlanMutationErrorV1::DuplicateTaskId {
                task_id: "t1".into(),
            },
            PlanMutationErrorV1::ImmutableCompletedTask {
                task_id: "t1".into(),
            },
            PlanMutationErrorV1::InvalidReference {
                task_id: "t2".into(),
                references: "t1".into(),
            },
            PlanMutationErrorV1::EmptySplit {
                task_id: "t1".into(),
            },
            PlanMutationErrorV1::EmptyMerge,
            PlanMutationErrorV1::TaskLimitExceeded {
                limit: 10,
                actual: 20,
            },
            PlanMutationErrorV1::CycleDetected {
                involved: vec!["t1".into(), "t2".into()],
            },
            PlanMutationErrorV1::EmptyOperations,
            PlanMutationErrorV1::ReplaceIdMismatch {
                task_id: "t1".into(),
                replacement_id: "t2".into(),
            },
        ];
        for e in &errors {
            let s = e.to_string();
            assert!(!s.is_empty(), "Display should produce non-empty string for {e:?}");
        }
    }

    // -----------------------------------------------------------------------
    // Property tests
    // -----------------------------------------------------------------------

    #[test]
    fn same_base_and_mutation_produces_same_result() {
        let plan = simple_plan(vec![
            task("t1", "alpha"),
            task_with_deps("t2", "beta", &["t1"]),
            task("t3", "gamma"),
        ]);
        let m = mutation(
            &plan,
            vec![
                PlanMutationOpV1::AddTask {
                    task: task("t4", "delta"),
                },
                PlanMutationOpV1::AddDependency {
                    task_id: "t4".into(),
                    depends_on: "t3".into(),
                },
                PlanMutationOpV1::ReplaceTask {
                    task_id: "t3".into(),
                    replacement: {
                        let mut r = task("t3", "gamma revised");
                        r.description = "updated".into();
                        r
                    },
                },
            ],
        );

        let (plan_a, result_a) = apply_mutation(&plan, &m, 100).unwrap();
        let (plan_b, result_b) = apply_mutation(&plan, &m, 100).unwrap();

        assert_eq!(plan_a, plan_b);
        assert_eq!(result_a, result_b);
        assert_eq!(
            canonical_fingerprint(&plan_a),
            canonical_fingerprint(&plan_b)
        );
    }

    #[test]
    fn chained_mutations_preserve_fingerprints() {
        let plan0 = simple_plan(vec![task("t1", "initial")]);

        // First mutation: add t2
        let m1 = mutation(
            &plan0,
            vec![PlanMutationOpV1::AddTask {
                task: task("t2", "second"),
            }],
        );
        let (plan1, result1) = apply_mutation(&plan0, &m1, 100).unwrap();

        // Second mutation: add dependency, using plan1's fingerprint
        let m2 = PlanMutationV1 {
            schema_version: 1,
            mutation_id: "mut-2".into(),
            base_fingerprint: result1.after_fingerprint.clone(),
            author: MutationAuthorV1 {
                kind: MutationAuthorKind::Controller,
                id: "chain".into(),
            },
            evidence: vec![],
            operations: vec![PlanMutationOpV1::AddDependency {
                task_id: "t2".into(),
                depends_on: "t1".into(),
            }],
        };
        let (plan2, result2) = apply_mutation(&plan1, &m2, 100).unwrap();

        // Fingerprint chain is consistent
        assert_eq!(result1.after_fingerprint, result2.before_fingerprint);
        assert_eq!(canonical_fingerprint(&plan1), result2.before_fingerprint);
        assert_eq!(canonical_fingerprint(&plan2), result2.after_fingerprint);
    }

    // -----------------------------------------------------------------------
    // Adversarial edge cases
    // -----------------------------------------------------------------------

    #[test]
    fn diamond_dependency_is_valid() {
        // t1 -> t2 -> t4
        // t1 -> t3 -> t4
        let plan = simple_plan(vec![task("t1", "a"), task("t2", "b")]);
        let m = mutation(
            &plan,
            vec![
                PlanMutationOpV1::AddTask {
                    task: task("t3", "c"),
                },
                PlanMutationOpV1::AddTask {
                    task: task("t4", "d"),
                },
                PlanMutationOpV1::AddDependency {
                    task_id: "t2".into(),
                    depends_on: "t1".into(),
                },
                PlanMutationOpV1::AddDependency {
                    task_id: "t3".into(),
                    depends_on: "t1".into(),
                },
                PlanMutationOpV1::AddDependency {
                    task_id: "t4".into(),
                    depends_on: "t2".into(),
                },
                PlanMutationOpV1::AddDependency {
                    task_id: "t4".into(),
                    depends_on: "t3".into(),
                },
            ],
        );
        let (new_plan, _) = apply_mutation(&plan, &m, 100).unwrap();
        assert_eq!(new_plan.tasks.len(), 4);
    }

    #[test]
    fn split_and_merge_in_same_mutation() {
        let plan = simple_plan(vec![
            task("t1", "a"),
            task("t2", "b"),
            task("t3", "c"),
        ]);
        let m = mutation(
            &plan,
            vec![
                // Split t1 into t1a, t1b
                PlanMutationOpV1::SplitTask {
                    task_id: "t1".into(),
                    parts: vec![task("t1a", "a1"), task("t1b", "a2")],
                },
                // Merge t2, t3 into t23
                PlanMutationOpV1::MergeTasks {
                    task_ids: vec!["t2".into(), "t3".into()],
                    merged: task("t23", "merged"),
                },
            ],
        );
        let (new_plan, result) = apply_mutation(&plan, &m, 100).unwrap();
        assert_eq!(new_plan.tasks.len(), 3); // t1a, t1b, t23
        assert!(new_plan.tasks.contains_key("t1a"));
        assert!(new_plan.tasks.contains_key("t1b"));
        assert!(new_plan.tasks.contains_key("t23"));

        let mut expected = vec![
            "t1".to_string(),
            "t1a".to_string(),
            "t1b".to_string(),
            "t2".to_string(),
            "t23".to_string(),
            "t3".to_string(),
        ];
        expected.sort();
        assert_eq!(result.changed_task_ids, expected);
    }

    #[test]
    fn empty_plan_can_receive_tasks() {
        let plan = MutablePlanV1 {
            plan_id: "empty".into(),
            tasks: BTreeMap::new(),
        };
        let m = mutation(
            &plan,
            vec![
                PlanMutationOpV1::AddTask {
                    task: task("t1", "first"),
                },
                PlanMutationOpV1::AddTask {
                    task: task_with_deps("t2", "second", &["t1"]),
                },
            ],
        );
        let (new_plan, _) = apply_mutation(&plan, &m, 100).unwrap();
        assert_eq!(new_plan.tasks.len(), 2);
    }

    #[test]
    fn completed_tasks_survive_mutations() {
        let plan = simple_plan(vec![
            completed_task("t1", "done task"),
            task("t2", "pending"),
        ]);
        let m = mutation(
            &plan,
            vec![PlanMutationOpV1::ReplaceTask {
                task_id: "t2".into(),
                replacement: task("t2", "revised pending"),
            }],
        );
        let (new_plan, _) = apply_mutation(&plan, &m, 100).unwrap();
        // Completed task is unchanged
        assert!(new_plan.tasks.get("t1").unwrap().completed);
        assert_eq!(
            new_plan.tasks.get("t2").unwrap().title,
            "revised pending"
        );
    }

    #[test]
    fn metadata_preserved_through_mutation() {
        let mut t = task("t1", "with meta");
        t.metadata
            .insert("key".into(), "value".into());
        let plan = simple_plan(vec![t]);
        let m = mutation(
            &plan,
            vec![PlanMutationOpV1::AddTask {
                task: task("t2", "new"),
            }],
        );
        let (new_plan, _) = apply_mutation(&plan, &m, 100).unwrap();
        assert_eq!(
            new_plan
                .tasks
                .get("t1")
                .unwrap()
                .metadata
                .get("key")
                .unwrap(),
            "value"
        );
    }

    #[test]
    fn author_kind_serde() {
        let user = MutationAuthorKind::User;
        let controller = MutationAuthorKind::Controller;
        assert_eq!(
            serde_json::to_string(&user).unwrap(),
            "\"user\""
        );
        assert_eq!(
            serde_json::to_string(&controller).unwrap(),
            "\"controller\""
        );
        let parsed: MutationAuthorKind = serde_json::from_str("\"user\"").unwrap();
        assert_eq!(parsed, MutationAuthorKind::User);
    }

    #[test]
    fn evidence_with_no_source_ref() {
        let e = MutationEvidenceV1 {
            code: "test".into(),
            message: "msg".into(),
            source_ref: None,
            fingerprint: "fp".into(),
        };
        let json = serde_json::to_string(&e).unwrap();
        assert!(!json.contains("source_ref"));
        let parsed: MutationEvidenceV1 = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.source_ref, None);
    }
}

#[cfg(test)]
mod prop_tests {
    use super::*;
    use proptest::prelude::*;

    fn arb_task_id() -> impl Strategy<Value = String> {
        "[a-z][a-z0-9]{0,7}".prop_map(|s| s)
    }

    fn arb_task() -> impl Strategy<Value = MutableTaskV1> {
        (arb_task_id(), "[a-z ]{1,20}")
            .prop_map(|(id, title)| MutableTaskV1 {
                id,
                title,
                description: String::new(),
                dependencies: BTreeSet::new(),
                metadata: BTreeMap::new(),
                completed: false,
            })
    }

    proptest! {
        #[test]
        fn fingerprint_deterministic_prop(
            plan_id in "[a-z]{3,10}",
            tasks in proptest::collection::vec(arb_task(), 1..5)
        ) {
            let mut task_map = BTreeMap::new();
            for t in tasks {
                task_map.insert(t.id.clone(), t);
            }
            let plan = MutablePlanV1 { plan_id, tasks: task_map };
            let fp1 = canonical_fingerprint(&plan);
            let fp2 = canonical_fingerprint(&plan);
            prop_assert_eq!(fp1, fp2);
            prop_assert_eq!(fp1.len(), 64);
        }

        #[test]
        fn add_then_remove_is_identity(
            base_title in "[a-z ]{1,10}",
            new_title in "[a-z ]{1,10}"
        ) {
            let plan = simple_plan(vec![task("t1", &base_title)]);
            let fp_before = canonical_fingerprint(&plan);

            let m = mutation(&plan, vec![
                PlanMutationOpV1::AddTask { task: task("tnew", &new_title) },
            ]);
            let (plan_after_add, _) = apply_mutation(&plan, &m, 100).unwrap();

            let m2 = mutation(&plan_after_add, vec![
                PlanMutationOpV1::RemoveTask { task_id: "tnew".into() },
            ]);
            let (plan_restored, _) = apply_mutation(&plan_after_add, &m2, 100).unwrap();

            prop_assert_eq!(canonical_fingerprint(&plan_restored), fp_before);
        }

        #[test]
        fn schema_version_0_always_rejected(
            title in "[a-z]{1,10}"
        ) {
            let plan = simple_plan(vec![task("t1", &title)]);
            let mut m = mutation(&plan, vec![
                PlanMutationOpV1::AddTask { task: task("t2", "x") },
            ]);
            m.schema_version = 0;
            let result = apply_mutation(&plan, &m, 100);
            prop_assert!(result.is_err());
            match result.unwrap_err() {
                PlanMutationErrorV1::SchemaIncompatible { expected: 1, actual: 0 } => {},
                other => prop_assert!(false, "unexpected error: {:?}", other),
            }
        }
    }

    fn simple_plan(tasks: Vec<MutableTaskV1>) -> MutablePlanV1 {
        let mut map = BTreeMap::new();
        for t in tasks {
            map.insert(t.id.clone(), t);
        }
        MutablePlanV1 {
            plan_id: "prop-test".into(),
            tasks: map,
        }
    }

    fn task(id: &str, title: &str) -> MutableTaskV1 {
        MutableTaskV1 {
            id: id.into(),
            title: title.into(),
            description: String::new(),
            dependencies: BTreeSet::new(),
            metadata: BTreeMap::new(),
            completed: false,
        }
    }

    fn mutation(
        base: &MutablePlanV1,
        ops: Vec<PlanMutationOpV1>,
    ) -> PlanMutationV1 {
        PlanMutationV1 {
            schema_version: 1,
            mutation_id: "prop-mut".into(),
            base_fingerprint: canonical_fingerprint(base),
            author: MutationAuthorV1 {
                kind: MutationAuthorKind::Controller,
                id: "proptest".into(),
            },
            evidence: vec![],
            operations: ops,
        }
    }
}
