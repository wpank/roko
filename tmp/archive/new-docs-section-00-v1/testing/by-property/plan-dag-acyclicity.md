# Plan DAG Acyclicity

> A plan's task dependency graph is always a DAG. A task cannot depend on itself, directly or transitively.

**Crate**: `roko-orchestrator`
**Test type**: Property-based (proptest)
**Enforcement**: `PlanDag::add_dependency`, topological sort validation
**Last reviewed**: 2026-04-19

---

## Statement

For all valid plan DAGs P: P contains no directed cycle in the dependency-of relation.

`task A depends_on task B → task B does not transitively depend_on task A`

---

## Why It Matters

The `ParallelExecutor` uses topological sort to determine dispatch order. A cycle in the DAG would make topological sort non-terminating. Tasks in a cycle could never be dispatched (each waiting for the other), causing the plan to deadlock.

---

## Enforcement

`PlanDag::add_dependency(task_a, depends_on: task_b)` checks for cycles by running DFS from `task_b`. If `task_a` is reachable from `task_b`, the dependency would create a cycle and `Err(CycleDetected)` is returned.

---

## Property Test

```rust
proptest! {
    #[test]
    fn plan_dag_never_has_cycles(
        dag in arb_plan_dag(max_tasks = 20),
    ) {
        // arb_plan_dag generates valid DAGs by construction
        let topo_order = dag.topological_sort();
        prop_assert!(topo_order.is_ok(), "A valid plan DAG must have a topological sort");

        // Verify that the sort respects all dependencies
        if let Ok(order) = topo_order {
            let position: HashMap<_, _> = order.iter().enumerate()
                .map(|(i, t)| (t, i)).collect();
            for (task, dep) in dag.all_dependencies() {
                prop_assert!(
                    position[&dep] < position[&task],
                    "Dependency {:?} must appear before {:?} in topological order",
                    dep, task
                );
            }
        }
    }
}
```

---

## Related Properties

- [lineage-acyclicity.md](lineage-acyclicity.md) — same structural invariant in Engram lineage
- [crash-recovery-consistency.md](crash-recovery-consistency.md) — crash recovery assumes valid DAG

## See also

- [../by-subsystem/subsystem-orchestrator.md](../by-subsystem/subsystem-orchestrator.md)
