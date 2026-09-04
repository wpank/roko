//! Graph-layer replan mutation adapter.
//!
//! Provides pure topology and mutation helper methods that the durable
//! `roko-execution` replan controller uses. This module contains only
//! neutral mutation/topology APIs -- no controller state, no strategy
//! selection, no cap enforcement.
//!
//! The controller calls these methods to inspect plan topology before
//! building mutations, and to validate results after applying them.

use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::hash::BuildHasher;

use roko_core::plan_mutation::{MutablePlanV1, MutableTaskV1, PlanMutationOpV1};

// ---------------------------------------------------------------------------
// Topology queries
// ---------------------------------------------------------------------------

/// Return the IDs of tasks that depend on the given task.
#[must_use]
pub fn downstream_tasks(plan: &MutablePlanV1, task_id: &str) -> Vec<String> {
    plan.tasks
        .values()
        .filter(|t| t.dependencies.contains(task_id))
        .map(|t| t.id.clone())
        .collect()
}

/// Return the IDs of tasks that the given task depends on.
#[must_use]
pub fn upstream_tasks(plan: &MutablePlanV1, task_id: &str) -> Vec<String> {
    plan.tasks
        .get(task_id)
        .map(|t| t.dependencies.iter().cloned().collect())
        .unwrap_or_default()
}

/// Find pending sibling tasks that share the same set of incoming
/// dependencies as the given task. Completed tasks are excluded.
///
/// "Sibling" means: same dependency set, different ID, not completed.
/// Results are sorted lexicographically for deterministic selection.
#[must_use]
pub fn pending_siblings<S: BuildHasher>(
    plan: &MutablePlanV1,
    task_id: &str,
    completed_ids: &HashSet<&str, S>,
) -> Vec<String> {
    let Some(target) = plan.tasks.get(task_id) else {
        return vec![];
    };

    let mut siblings: Vec<String> = plan
        .tasks
        .values()
        .filter(|t| {
            t.id != task_id
                && !t.completed
                && !completed_ids.contains(t.id.as_str())
                && t.dependencies == target.dependencies
        })
        .map(|t| t.id.clone())
        .collect();

    siblings.sort();
    siblings
}

// ---------------------------------------------------------------------------
// Mutation construction helpers
// ---------------------------------------------------------------------------

/// Build split-task mutation operations that correctly rewire edges.
///
/// Given a task to split:
/// 1. All incoming dependencies (tasks that `task_id` depends on) are
///    inherited by `<task_id>-part-1`.
/// 2. `<task_id>-part-2` depends on `<task_id>-part-1`.
/// 3. All downstream tasks that depended on `task_id` are rewired to
///    depend on `<task_id>-part-2` instead.
///
/// Returns the mutation operations and the two generated part IDs.
#[must_use]
pub fn build_split_with_rewiring(
    plan: &MutablePlanV1,
    task_id: &str,
    part1_title: &str,
    part1_description: &str,
    part2_title: &str,
    part2_description: &str,
) -> Option<(Vec<PlanMutationOpV1>, String, String)> {
    let task = plan.tasks.get(task_id)?;
    if task.completed {
        return None;
    }

    let part1_id = format!("{}-part-1", task_id);
    let part2_id = format!("{}-part-2", task_id);

    // Part 1 inherits incoming deps.
    let part1 = MutableTaskV1 {
        id: part1_id.clone(),
        title: part1_title.to_string(),
        description: part1_description.to_string(),
        dependencies: task.dependencies.clone(),
        metadata: BTreeMap::from([
            ("split_source".to_string(), task_id.to_string()),
            ("split_ordinal".to_string(), "1".to_string()),
        ]),
        completed: false,
    };

    // Part 2 depends on part 1.
    let part2 = MutableTaskV1 {
        id: part2_id.clone(),
        title: part2_title.to_string(),
        description: part2_description.to_string(),
        dependencies: BTreeSet::from([part1_id.clone()]),
        metadata: BTreeMap::from([
            ("split_source".to_string(), task_id.to_string()),
            ("split_ordinal".to_string(), "2".to_string()),
        ]),
        completed: false,
    };

    let mut ops = vec![PlanMutationOpV1::SplitTask {
        task_id: task_id.to_string(),
        parts: vec![part1, part2],
    }];

    // Rewire downstream: tasks that depended on task_id now depend on part2.
    let downstreams = downstream_tasks(plan, task_id);
    for ds_id in &downstreams {
        ops.push(PlanMutationOpV1::RemoveDependency {
            task_id: ds_id.clone(),
            depends_on: task_id.to_string(),
        });
        ops.push(PlanMutationOpV1::AddDependency {
            task_id: ds_id.clone(),
            depends_on: part2_id.clone(),
        });
    }

    Some((ops, part1_id, part2_id))
}

/// Build merge-task mutation operations for two tasks.
///
/// Merges `task_a` and `task_b` into a single task with ID `merged_id`.
/// The merged task inherits the union of both tasks' dependencies
/// (excluding references to the merged tasks themselves).
/// Downstream tasks that depended on either source are rewired to the
/// merged ID.
#[must_use]
pub fn build_merge_with_rewiring(
    plan: &MutablePlanV1,
    task_a_id: &str,
    task_b_id: &str,
    merged_id: &str,
    merged_title: &str,
    merged_description: &str,
) -> Option<Vec<PlanMutationOpV1>> {
    let task_a = plan.tasks.get(task_a_id)?;
    let task_b = plan.tasks.get(task_b_id)?;
    if task_a.completed || task_b.completed {
        return None;
    }

    // Union of dependencies, excluding self-references.
    let source_ids: HashSet<&str> = [task_a_id, task_b_id].into_iter().collect();
    let merged_deps: BTreeSet<String> = task_a
        .dependencies
        .union(&task_b.dependencies)
        .filter(|d| !source_ids.contains(d.as_str()))
        .cloned()
        .collect();

    let merged = MutableTaskV1 {
        id: merged_id.to_string(),
        title: merged_title.to_string(),
        description: merged_description.to_string(),
        dependencies: merged_deps,
        metadata: BTreeMap::from([(
            "merged_from".to_string(),
            format!("{},{}", task_a_id, task_b_id),
        )]),
        completed: false,
    };

    let mut ops = vec![PlanMutationOpV1::MergeTasks {
        task_ids: vec![task_a_id.to_string(), task_b_id.to_string()],
        merged,
    }];

    // Rewire downstream tasks.
    let mut downstreams: HashSet<String> = HashSet::new();
    downstreams.extend(downstream_tasks(plan, task_a_id));
    downstreams.extend(downstream_tasks(plan, task_b_id));
    // Exclude the two source tasks themselves.
    downstreams.remove(task_a_id);
    downstreams.remove(task_b_id);

    let mut sorted_ds: Vec<String> = downstreams.into_iter().collect();
    sorted_ds.sort();

    for ds_id in &sorted_ds {
        let ds_task = match plan.tasks.get(ds_id) {
            Some(t) => t,
            None => continue,
        };
        if ds_task.dependencies.contains(task_a_id) {
            ops.push(PlanMutationOpV1::RemoveDependency {
                task_id: ds_id.clone(),
                depends_on: task_a_id.to_string(),
            });
        }
        if ds_task.dependencies.contains(task_b_id) {
            ops.push(PlanMutationOpV1::RemoveDependency {
                task_id: ds_id.clone(),
                depends_on: task_b_id.to_string(),
            });
        }
        ops.push(PlanMutationOpV1::AddDependency {
            task_id: ds_id.clone(),
            depends_on: merged_id.to_string(),
        });
    }

    Some(ops)
}

/// Validate that a set of completed task IDs are preserved (not removed or
/// modified) by a sequence of mutation operations.
#[must_use]
pub fn completed_tasks_preserved<S: BuildHasher>(
    ops: &[PlanMutationOpV1],
    completed_ids: &HashSet<&str, S>,
) -> bool {
    for op in ops {
        match op {
            PlanMutationOpV1::RemoveTask { task_id } => {
                if completed_ids.contains(task_id.as_str()) {
                    return false;
                }
            }
            PlanMutationOpV1::ReplaceTask { task_id, .. } => {
                if completed_ids.contains(task_id.as_str()) {
                    return false;
                }
            }
            PlanMutationOpV1::SplitTask { task_id, .. } => {
                if completed_ids.contains(task_id.as_str()) {
                    return false;
                }
            }
            PlanMutationOpV1::MergeTasks { task_ids, .. } => {
                for tid in task_ids {
                    if completed_ids.contains(tid.as_str()) {
                        return false;
                    }
                }
            }
            PlanMutationOpV1::AddTask { .. }
            | PlanMutationOpV1::AddDependency { .. }
            | PlanMutationOpV1::RemoveDependency { .. } => {}
        }
    }
    true
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use roko_core::plan_mutation::canonical_fingerprint;

    use super::*;

    fn test_plan(tasks: &[(&str, &[&str])]) -> MutablePlanV1 {
        let mut task_map = BTreeMap::new();
        for (id, deps) in tasks {
            task_map.insert(
                id.to_string(),
                MutableTaskV1 {
                    id: id.to_string(),
                    title: format!("Task {}", id),
                    description: format!("Description for {}", id),
                    dependencies: deps.iter().map(|d| d.to_string()).collect(),
                    metadata: BTreeMap::new(),
                    completed: false,
                },
            );
        }
        MutablePlanV1 {
            plan_id: "test-plan".to_string(),
            tasks: task_map,
        }
    }

    #[test]
    fn downstream_and_upstream_queries() {
        let plan = test_plan(&[("a", &[]), ("b", &["a"]), ("c", &["a", "b"])]);

        let downstream_a = downstream_tasks(&plan, "a");
        assert!(downstream_a.contains(&"b".to_string()));
        assert!(downstream_a.contains(&"c".to_string()));
        assert_eq!(downstream_a.len(), 2);

        let upstream_c = upstream_tasks(&plan, "c");
        assert!(upstream_c.contains(&"a".to_string()));
        assert!(upstream_c.contains(&"b".to_string()));
        assert_eq!(upstream_c.len(), 2);

        assert!(downstream_tasks(&plan, "c").is_empty());
        assert!(upstream_tasks(&plan, "a").is_empty());
    }

    #[test]
    fn pending_siblings_finds_same_deps() {
        let plan = test_plan(&[
            ("root", &[]),
            ("a", &["root"]),
            ("b", &["root"]),
            ("c", &[]),
        ]);
        let completed = HashSet::new();

        let sibs = pending_siblings(&plan, "a", &completed);
        assert_eq!(sibs, vec!["b".to_string()]);

        // c has different deps (none vs ["root"]).
        assert!(!sibs.contains(&"c".to_string()));
    }

    #[test]
    fn pending_siblings_excludes_completed() {
        let mut plan = test_plan(&[("a", &[]), ("b", &[])]);
        plan.tasks.get_mut("b").unwrap().completed = true;
        let completed: HashSet<&str> = ["b"].into_iter().collect();

        let sibs = pending_siblings(&plan, "a", &completed);
        assert!(sibs.is_empty());
    }

    #[test]
    fn split_with_rewiring_rewires_downstream() {
        let plan = test_plan(&[("a", &[]), ("b", &["a"]), ("c", &["b"])]);

        let result =
            build_split_with_rewiring(&plan, "b", "Part 1", "First part", "Part 2", "Second part");
        assert!(result.is_some());
        let (ops, part1_id, part2_id) = result.unwrap();

        assert_eq!(part1_id, "b-part-1");
        assert_eq!(part2_id, "b-part-2");

        // Apply and verify.
        let fp = canonical_fingerprint(&plan);
        let mutation = roko_core::plan_mutation::PlanMutationV1 {
            schema_version: 1,
            mutation_id: "test-split".to_string(),
            base_fingerprint: fp,
            author: roko_core::plan_mutation::MutationAuthorV1 {
                kind: roko_core::plan_mutation::MutationAuthorKind::Controller,
                id: "test".to_string(),
            },
            evidence: vec![],
            operations: ops,
        };

        let (new_plan, _result) =
            roko_core::plan_mutation::apply_mutation(&plan, &mutation, 100).unwrap();

        // b is removed, part1 and part2 exist.
        assert!(!new_plan.tasks.contains_key("b"));
        assert!(new_plan.tasks.contains_key("b-part-1"));
        assert!(new_plan.tasks.contains_key("b-part-2"));

        // c now depends on b-part-2 instead of b.
        let c = &new_plan.tasks["c"];
        assert!(c.dependencies.contains("b-part-2"));
        assert!(!c.dependencies.contains("b"));

        // part1 inherits b's deps.
        let p1 = &new_plan.tasks["b-part-1"];
        assert!(p1.dependencies.contains("a"));

        // part2 depends on part1.
        let p2 = &new_plan.tasks["b-part-2"];
        assert!(p2.dependencies.contains("b-part-1"));
    }

    #[test]
    fn merge_with_rewiring_rewires_downstream() {
        let plan = test_plan(&[
            ("root", &[]),
            ("a", &["root"]),
            ("b", &["root"]),
            ("c", &["a", "b"]),
        ]);

        let result = build_merge_with_rewiring(&plan, "a", "b", "a", "Merged A+B", "Combined task");
        assert!(result.is_some());
        let ops = result.unwrap();

        let fp = canonical_fingerprint(&plan);
        let mutation = roko_core::plan_mutation::PlanMutationV1 {
            schema_version: 1,
            mutation_id: "test-merge".to_string(),
            base_fingerprint: fp,
            author: roko_core::plan_mutation::MutationAuthorV1 {
                kind: roko_core::plan_mutation::MutationAuthorKind::Controller,
                id: "test".to_string(),
            },
            evidence: vec![],
            operations: ops,
        };

        let (new_plan, _result) =
            roko_core::plan_mutation::apply_mutation(&plan, &mutation, 100).unwrap();

        // b is consumed, a remains (as the merged result).
        assert!(new_plan.tasks.contains_key("a"));
        assert!(!new_plan.tasks.contains_key("b"));

        // c now depends on a (the merged task).
        let c = &new_plan.tasks["c"];
        assert!(c.dependencies.contains("a"));
        assert!(!c.dependencies.contains("b"));
    }

    #[test]
    fn completed_tasks_preserved_check() {
        let completed: HashSet<&str> = ["done-task"].into_iter().collect();

        // Safe ops.
        let safe_ops = vec![PlanMutationOpV1::AddTask {
            task: MutableTaskV1 {
                id: "new".to_string(),
                title: "New".to_string(),
                description: "New task".to_string(),
                dependencies: BTreeSet::new(),
                metadata: BTreeMap::new(),
                completed: false,
            },
        }];
        assert!(completed_tasks_preserved(&safe_ops, &completed));

        // Unsafe: removing a completed task.
        let unsafe_ops = vec![PlanMutationOpV1::RemoveTask {
            task_id: "done-task".to_string(),
        }];
        assert!(!completed_tasks_preserved(&unsafe_ops, &completed));
    }

    #[test]
    fn split_completed_task_returns_none() {
        let mut plan = test_plan(&[("a", &[])]);
        plan.tasks.get_mut("a").unwrap().completed = true;

        let result = build_split_with_rewiring(&plan, "a", "Part 1", "First", "Part 2", "Second");
        assert!(result.is_none());
    }

    #[test]
    fn merge_completed_task_returns_none() {
        let mut plan = test_plan(&[("a", &[]), ("b", &[])]);
        plan.tasks.get_mut("b").unwrap().completed = true;

        let result = build_merge_with_rewiring(&plan, "a", "b", "a", "Merged", "Combined");
        assert!(result.is_none());
    }

    #[test]
    fn nonexistent_task_queries_are_empty() {
        let plan = test_plan(&[("a", &[])]);
        assert!(downstream_tasks(&plan, "nope").is_empty());
        assert!(upstream_tasks(&plan, "nope").is_empty());
        assert!(pending_siblings(&plan, "nope", &HashSet::new()).is_empty());
    }
}
