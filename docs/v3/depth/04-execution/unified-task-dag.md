# Unified Task DAG

> Depth file for [04-EXECUTION.md](../../04-EXECUTION.md) section 5.
> Preserves and updates content from v1 `01-orchestration/02-unified-task-dag.md`.

---

## Overview

Within a single plan, the task DAG is embedded in the `Graph` via edges
(see `plan-to-graph-conversion.md`). Across plans, the `CrossPlanDag`
computes a dependency graph from frontmatter `depends_on` declarations,
groups plans into execution waves, and detects crate overlaps.

This two-level design separates concerns: the `CrossPlanDag` handles
plan-level ordering, and the `GraphEngine` handles task-level ordering
within each plan's graph.

**Source:** `crates/roko-cli/src/runner/plan_dag.rs`

---

## CrossPlanDag

The `CrossPlanDag` answers the cross-plan scheduling question: given N
plans with inter-plan dependencies, which plans can run in parallel and
which must be serialized?

### Construction

```rust
impl CrossPlanDag {
    pub fn compute(plans: &[Plan]) -> Result<Self, CrossPlanDagError>
}
```

Construction proceeds in four steps:

1. **Collect plan nodes.** Each discovered plan becomes a `PlanNode` with
   its task count and crate-touched metadata.

2. **Resolve dependencies.** For each task's `depends_on_plan` list and
   each plan's frontmatter `depends_on` list, directed edges are added
   from the dependency plan to the dependent plan.

3. **Cycle detection.** Kahn's algorithm is applied to the plan-level
   graph. If the topological sort does not visit all nodes, a cycle
   exists and `CrossPlanDagError::Cycle` is returned. The error includes
   the cycle path for diagnosis.

4. **Wave assignment.** Plans are grouped into execution waves using
   BFS layering.

### PlanNode

```rust
pub struct PlanNode {
    pub id: String,
    pub wave_index: usize,
    pub depends_on_plans: Vec<String>,
    pub task_count: usize,
    pub crates_touched: Vec<String>,
}
```

---

## Wave Computation

The `CrossPlanDag` groups plans into waves using Kahn's algorithm:

1. Compute in-degrees for all plan nodes.
2. Wave 0 contains all zero-in-degree plans (no dependencies).
3. For each wave, mark all plans as "executed" and decrement in-degrees
   of their dependents.
4. Plans whose in-degree reaches zero form the next wave.
5. Continue until all plans are assigned.

### WaveInfo

```rust
pub struct WaveInfo {
    pub index: usize,
    pub plan_ids: Vec<String>,
    pub parallelism_width: usize,
    pub total_tasks: usize,
}
```

Plans within the same wave can execute in parallel. Plans in wave N+1
depend only on plans in waves 0..N.

### Example

```
Plan A: depends_on = []           -> Wave 0
Plan B: depends_on = ["A"]        -> Wave 1
Plan C: depends_on = []           -> Wave 0
Plan D: depends_on = ["A", "C"]   -> Wave 1
Plan E: depends_on = ["B", "D"]   -> Wave 2
```

Waves:
- Wave 0: [A, C] -- independent, run in parallel
- Wave 1: [B, D] -- depend on wave-0 plans
- Wave 2: [E]    -- depends on wave-1 plans

The minimum execution time is 3 sequential steps (3 waves), regardless
of how many workers are available.

---

## Crate Overlap Detection

When multiple plans in the same wave declare the same crate in
`crates_touched`, the DAG emits a `CrateOverlapWarning`:

```rust
pub struct CrateOverlapWarning {
    pub crate_name: String,
    pub plan_a: String,
    pub plan_b: String,
    pub wave_index: usize,
}
```

Crate overlaps do not prevent execution. They are advisory warnings that
help operators identify potential merge conflicts. The merge queue (see
`merge-queue.md`) handles actual file-level conflicts at integration time.

The distinction between crate-level overlap (advisory, at scheduling
time) and file-level conflict (enforced, at merge time) is intentional.
Crate-level is too coarse to block execution -- two plans modifying
different files in the same crate are safe to run in parallel. File-level
is precise but only known after agents have produced their changes.

---

## Dangling Reference Detection

When a plan's `depends_on` references a plan ID that does not exist in
the discovered set, the DAG logs a warning rather than failing. This
allows plans to reference future plans that have not yet been written
and enables incremental plan development.

---

## Critical Path Estimation

The DAG computes basic statistics for capacity planning:

```rust
pub struct DagStats {
    pub total_plans: usize,
    pub total_waves: usize,
    pub total_tasks: usize,
    pub max_wave_width: usize,
    pub critical_path_length: usize,
}
```

The critical path length is the number of waves -- the minimum number
of sequential steps regardless of parallelism. It bounds the minimum
wall-clock time for the entire plan set.

---

## Intra-Plan Dependencies

Within a single plan, task dependencies are encoded as edges in the
converted `Graph`. The `CrossPlanDag` does not handle intra-plan
dependencies -- those are the responsibility of `plan_to_graph()` and
`ProductionPlanTopology::build()`.

### GlobalTaskId (v1 concept)

In the v1 `UnifiedTaskDag`, tasks were identified by composite keys in
the format `"plan_id:task_id"`. In the current architecture, the Graph
engine uses plain node IDs within a single graph. Cross-plan references
use the plan ID as the scoping mechanism. The GlobalTaskId concept is
preserved only in the cross-plan DAG where plan IDs serve as the node
identifiers.

### File-Overlap Inference (v1 concept)

The v1 `UnifiedTaskDag` inferred dependency edges from file overlaps
between tasks in different plans: if two tasks from different plans
declared the same files in their `files` field, a synthetic edge was
added to prevent concurrent execution. The direction was deterministic
(lexicographic plan ordering) to prevent deadlocks.

In the current architecture, file-level conflict detection is handled by
the merge queue at integration time rather than by the DAG at scheduling
time. This change was motivated by practical experience: file overlap
inference at scheduling time was too conservative (blocking execution
of plans that would have merged cleanly) and too imprecise (file lists
in `tasks.toml` were often incomplete).

---

## Differences from v1 UnifiedTaskDag

| Aspect | v1 UnifiedTaskDag | Current CrossPlanDag |
|---|---|---|
| Scope | All tasks across all plans | Plans only (tasks are in-graph) |
| File-overlap edges | Inferred at task level | Advisory at crate level |
| Synthetic `__whole__` nodes | Created for cross-plan deps | Not needed (plan-level) |
| Wave computation | Task-level | Plan-level |
| Critical path | Task-level DP | Plan-level wave count |
| Cycle detection | Kahn's algorithm over tasks | Kahn's algorithm over plans |

The current design separates concerns: the `CrossPlanDag` handles
plan-level ordering, and the `GraphEngine` handles task-level ordering
within each plan's graph.

---

## CLI Integration

```bash
# Show plans with wave assignments
cargo run -p roko-cli -- plan list --waves

# Output includes:
#   Wave 0 (2 plans, 15 tasks):
#     01-workspace-scaffold (8 tasks)
#     02-core-traits (7 tasks)
#   Wave 1 (1 plan, 12 tasks):
#     03-agent-dispatch (12 tasks)
```

---

## Error Types

```rust
pub enum CrossPlanDagError {
    Cycle { cycle: Vec<String> },
}
```

Cycle detection is critical for safety -- a cyclic dependency among plans
would cause the executor to deadlock, with plans waiting for each other
indefinitely. The error message includes the cycle path for diagnosis.
