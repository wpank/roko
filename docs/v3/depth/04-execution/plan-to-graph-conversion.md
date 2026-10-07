# Plan-to-Graph Conversion

> Depth file for [04-EXECUTION.md](../../04-EXECUTION.md) section 4.
> Preserves and updates content from v1 `01-orchestration/01-plan-discovery.md`
> and `01-orchestration/02-unified-task-dag.md`.

---

## Overview

Every plan starts as a directory on disk containing `plan.md` and `tasks.toml`.
Before the Graph engine can execute it, the plan must be converted into a typed
`Graph` -- a DAG of `Node` values connected by `Edge` values. Two converters
exist, serving different use cases: a simple one-node-per-task converter and a
production topology that builds a 5-node subgraph per task.

> **Status (2026-10-03):** the production topology's six `plan.enricher.*`
> nodes (knowledge, episodes, playbook, modulation, safety, experiment) were
> passthrough stubs that changed nothing, and were removed (9206). A task's
> subgraph is now context -> compose -> executor -> gate -> success: 5 nodes
> and 4 edges. The node counts, enricher nodes and wave figures in the
> production-converter sections below describe the removed 11-node layout.

**Source:** `crates/roko-graph/src/convert.rs`, `crates/roko-graph/src/topology.rs`

---

## Simple Converter: `plan_to_graph()`

The simple converter maps each task definition to a single graph node. It is
used when enrichment, composition, and gating are handled by the controller
outside the graph rather than expressed as nodes inside it.

### Signature

```rust
pub fn plan_to_graph(
    plan_id: &str,
    plan_dir: &str,
    tasks: &[(String, PlanTaskInfo)],
    max_parallel: u32,
) -> Result<Graph, GraphError>
```

### Conversion steps

1. **Build metadata.** A `GraphMetadata` value captures the plan ID, plan
   directory, source label (`"plan-converter"`), and max parallelism.

2. **Add nodes.** Each `(task_id, PlanTaskInfo)` pair becomes a `Node` with:
   - `cell_type = "task-executor"`
   - `execution_class = ExecutionClass::Activity` (non-deterministic LLM
     dispatch; outputs are recorded for snapshot/replay)
   - A `config` map carrying the plan ID, plan directory, task title,
     description, role, tier, files, timeout, and verification commands.
   - `exclusive` = the task's `files`. The engine never runs two nodes whose
     exclusive paths overlap at the same time (the same path, or a directory
     and a path inside it), so two tasks that declare overlapping files run
     one after the other even when both are ready. The task that waits holds
     no `max_concurrent_nodes` slot, and the engine logs which running task
     it waits for. A task with no `files` never waits. The paths are left
     out of the plan fingerprint, so checkpoints still resume. With
     per-task worktrees (the default) each task writes its own checkout, so
     the run clears every node's `exclusive` paths.

3. **Add edges.** For each task's `depends_on` list, a directed `Edge` is
   created from the dependency to the dependent. Unknown dependency IDs
   produce a `GraphError::MissingNode`.

4. **Handle cross-plan deps.** `depends_on_plan` entries reference plans
   outside this graph. They are logged as warnings and skipped -- cross-plan
   ordering is the responsibility of the `CrossPlanDag` (see
   `unified-task-dag.md`).

5. **Set policy.** `graph.policy.max_concurrent_nodes` is set from the
   `max_parallel` argument (minimum 1). A plan run passes `--max-tasks`, else
   the plan's `[meta] max_parallel`. A plan that omits `max_parallel` converts
   with 1, as it did before an omitted value meant "as wide as the DAG
   allows", so its checkpoint identity is unchanged. Once the identity is
   taken, the run raises the limit to the plan's task count if every task
   that can write declares its `files` (`plan_policy::plan_max_parallel`).
   Otherwise, it logs the task whose writes are unknown and runs one task at a
   time.

6. **Cycle check.** The graph validates that no cycles exist via
   `petgraph::algo::toposort` inside `Graph::validate()`.

### Node config map

The per-node `config` carries all the metadata the `task-executor` cell needs
at runtime:

| Key | Type | Purpose |
|---|---|---|
| `plan_id` | `String` | Identifies the owning plan |
| `plan_dir` | `String` | Filesystem path for worktree setup |
| `task_id` | `String` | Unique within the plan |
| `title` | `String` | Human-readable task name |
| `description` | `Option<String>` | Detailed task specification |
| `role` | `Option<String>` | Requested agent role |
| `tier` | `String` | Complexity tier (fast/standard/complex) |
| `model_hint` | `Option<String>` | Preferred model override |
| `files` | `Vec<String>` | Expected files in scope |
| `timeout_secs` | `u64` | Per-task timeout |
| `verify` | `Option<String>` | Verification command |

---

## Production Converter: `ProductionPlanTopology::build()`

The production converter builds a richer subgraph per task. Instead of a
single node, each task becomes a 5-node pipeline that expresses its context,
composition, execution, gating, and success boundary as explicit graph nodes.

**Source:** `crates/roko-graph/src/topology.rs`

### Per-task subgraph

```
[TaskContextCell] -> [ComposeCell] -> [TaskExecutorCell] -> [GateCell] -> [SuccessBoundary]
```

`ComposeCell` turns the `TaskContext` Signal into the task's prompt.

### Node ID conventions

Node IDs follow the pattern `task.<task_id>.<suffix>`:

| Suffix | Cell type | Execution class | Purpose |
|---|---|---|---|
| `context` | `plan.context` | Workflow | Emit task context Signal |
| `knowledge` | `plan.knowledge` | Workflow | Query neuro knowledge store |
| `episodes` | `plan.episodes` | Workflow | Query recent episodes |
| `playbook` | `plan.playbook` | Workflow | Query matching playbooks |
| `modulation` | `plan.modulation` | Workflow | Daimon affect modulation |
| `safety` | `plan.safety` | Workflow | Safety contract checks |
| `experiment` | `plan.experiment` | Workflow | A/B experiment assignment |
| `compose` | `plan.compose` | Workflow | Assemble system prompt |
| `executor` | `task-executor` | Activity | LLM agent dispatch |
| `gate` | `plan.gate` | Activity | Compile/test/clippy pipeline |
| `boundary` | `plan.success-boundary` | Workflow | Mark task complete |

Enricher nodes are Workflow (deterministic, never recorded). The executor
and gate are Activity (non-deterministic, always recorded).

The executor and the gate both take the task's `files` as their `exclusive`
paths: the executor writes them and the gate checks them. No other node of
the subgraph holds any paths. Each node holds the paths only while it runs,
so an overlapping task can still run between a task's executor and its gate.
That cannot change what the gate checks. The gate judges the attempt's own
isolated checkout, never the shared working tree, and fails closed when the
attempt ran in the shared tree (per-task worktrees are the default;
`--no-worktree-per-task` turns them off).

### Why the Workflow/Activity distinction matters

Workflow nodes can be re-derived from their inputs on resume. Their outputs
are never persisted to the activity log. This means a 10-task plan with
110 nodes only records 20 Activity outputs (10 executor + 10 gate) rather
than 110. On resume, the 90 Workflow outputs are recomputed, which is cheap
(knowledge queries, playbook lookups, prompt assembly).

Activity nodes invoke external systems (LLM providers, compilation
toolchains). Their outputs are recorded to JSONL after every execution and
replayed from the recording on resume, avoiding duplicate LLM calls.

### Inter-task dependencies

Dependencies between tasks are wired from the predecessor's
`SuccessBoundary` node to the dependent's `TaskContextCell` node. This
ensures a task only begins enrichment after all upstream tasks have fully
completed and passed their gates.

For a linear chain of three tasks, the graph structure is:

```
task.t1.context -> ... -> task.t1.boundary
                                  |
task.t2.context -> ... -> task.t2.boundary  <-- starts only after t1.boundary
                                  |
task.t3.context -> ... -> task.t3.boundary  <-- starts only after t2.boundary
```

### TopologyTaskInfo

The production converter uses its own input type rather than importing
`PlanTaskInfo` directly:

```rust
pub struct TopologyTaskInfo {
    pub task_id: String,
    pub title: String,
    pub description: Option<String>,
    pub role: Option<String>,
    pub tier: String,
    pub model_hint: Option<String>,
    pub files: Vec<String>,
    pub depends_on: Vec<String>,
    pub timeout_secs: u64,
    pub max_retries: u32,
    pub domain: Option<String>,
    pub sequence: usize,
    pub full_config_json: serde_json::Value,
}
```

### TopologyReport

`ProductionPlanTopology::build()` returns a `TopologyReport` alongside the
graph:

```rust
pub struct TopologyReport {
    pub total_nodes: usize,          // 11 * N for N tasks
    pub total_edges: usize,
    pub entry_tasks: Vec<String>,
    pub exit_tasks: Vec<String>,
    pub enricher_parallelism: usize, // always 6 per task
}
```

For a 10-task plan, the production topology produces 110 nodes and hundreds
of edges. For a 30-task plan with moderate dependency density, expect around
330 nodes and 1,000+ edges.

---

## Choosing Between Converters

| Criterion | `plan_to_graph()` | `ProductionPlanTopology` |
|---|---|---|
| Nodes per task | 1 | 11 |
| Enrichment | Controller-side | In-graph |
| Gate pipeline | Controller-side | In-graph |
| Prompt composition | Controller-side | In-graph |
| Resume granularity | Per-task | Per-enrichment-stage |
| Observability | Opaque (only task start/end) | Full (each stage visible) |
| Typical use | `roko graph run`, testing | `roko plan run` |

The production topology is the default for plan execution because it gives
the engine full observability into enrichment stages, enables per-stage
replay on resume, and allows the engine's conditional routing to skip
enrichment stages when their outputs are cached.

---

## Cross-Plan Dependencies

Both converters operate on a single plan. Cross-plan ordering is handled
by the `CrossPlanDag` in `crates/roko-cli/src/runner/plan_dag.rs`. When a
task declares `depends_on_plan = ["01-core"]`, the converter logs a warning
and skips the edge. The `CrossPlanDag` enforces the ordering at the plan
scheduling level instead.

This separation is intentional. A single `Graph` represents a single plan.
Multi-plan coordination happens at a higher level, where the controller
sequences plan graphs based on the cross-plan DAG.

---

## Error Handling

| Error | Cause | Recovery |
|---|---|---|
| `GraphError::MissingNode` | `depends_on` references unknown task ID | Fix `tasks.toml` |
| `GraphError::CycleDetected` | Circular dependency chain | Remove the cycle |
| `GraphError::DuplicateNode` | Two tasks share the same ID | Rename one task |
| `GraphError::UnknownCellType` | Node references unregistered cell type | Register the cell |

All errors are returned before execution begins. A malformed plan never
reaches the engine.

---

## Validation

The converted graph is validated before execution:

1. **Cycle detection**: `petgraph::algo::toposort` over the internal
   `DiGraph`. Cycles produce `GraphError::CycleDetected`.

2. **Connected components**: every node must be reachable from at least one
   entry node. Disconnected nodes are not an error but produce a warning.

3. **Cell registry check**: every `cell_type` referenced by a node must
   have a registered `Cell` implementation in the `CellRegistry`. Missing
   cells produce `GraphError::UnknownCellType`.

4. **Policy validation**: `max_concurrent_nodes` must be >= 1.

---

## Practical Example

Given a `tasks.toml` with three tasks:

```toml
[tasks.setup]
title = "Create module structure"
tier = "fast"

[tasks.implement]
title = "Implement core logic"
depends_on = ["setup"]
tier = "standard"

[tasks.test]
title = "Write tests"
depends_on = ["implement"]
tier = "fast"
```

The simple converter produces 3 nodes and 2 edges:

```
setup -> implement -> test
```

The production converter produces 33 nodes (11 per task) with inter-task
edges connecting `setup.boundary -> implement.context` and
`implement.boundary -> test.context`, plus intra-task edges for the
enrichment fan-out and fan-in within each task.

---

## Verification Commands

```bash
# Validate a plan's tasks.toml and check graph conversion
cargo run -p roko-cli -- plan validate plans/<dir>

# Convert and inspect the graph without executing
cargo run -p roko-cli -- graph inspect plans/<dir>
```
