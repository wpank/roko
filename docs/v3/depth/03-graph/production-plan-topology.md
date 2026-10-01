# Production Plan Topology

> ProductionPlanTopology builds the canonical per-task 11-node subgraph that
> transforms a `tasks.toml` plan into an executable Graph. This file documents
> the subgraph structure, node roles, inter-task wiring and cell
> registration.

---

## Source Files

| File | What |
|---|---|
| `crates/roko-graph/src/topology.rs` | ProductionPlanTopology, TopologyTaskInfo, register_topology_cells |

---

## Per-Task Subgraph (11 Nodes, 16 Edges)

Each task in a plan expands into an 11-node subgraph:

```
  [TaskContextCell]----+---> [KnowledgeEnricher]  ---+
       (Workflow)      |---> [EpisodesEnricher]   ---|
                       |---> [PlaybookEnricher]   ---|---> [PlanComposeCell] --> [TaskExecutorCell] --> [PlanGateCell] --> [SuccessBoundary]
                       |---> [ModulationEnricher] ---|     (Workflow)            (Activity)             (Activity)        (Workflow)
                       |---> [SafetyEnricher]     ---|
                       +---> [ExperimentEnricher] ---+
                       |                             |
                       +-----------------------------+  (direct 7th input: context -> compose)
```

### Node Roles

| Node | Cell Type | ExecutionClass | Purpose |
|---|---|---|---|
| TaskContext | `plan.task-context` | Workflow | Collects task metadata, prior attempt state |
| KnowledgeEnricher | `plan.enricher.knowledge` | Workflow | Queries durable knowledge store |
| EpisodesEnricher | `plan.enricher.episodes` | Workflow | Retrieves relevant episodes |
| PlaybookEnricher | `plan.enricher.playbook` | Workflow | Matches when/then playbook rules |
| ModulationEnricher | `plan.enricher.modulation` | Workflow | Applies affect/daimon modulation |
| SafetyEnricher | `plan.enricher.safety` | Workflow | Injects safety context |
| ExperimentEnricher | `plan.enricher.experiment` | Workflow | Applies A/B experiment assignment |
| PlanCompose | `plan.compose` | Workflow | Fan-in: assembles prompt from 7 inputs |
| TaskExecutor | `task-executor` | Activity | Non-deterministic LLM dispatch |
| PlanGate | `plan.gate` | Activity | Runs gate pipeline (compile, test, clippy, diff) |
| SuccessBoundary | `plan.success-boundary` | Workflow | No-op anchor for inter-task edges |

### Node ID Convention

All node IDs follow the pattern `task.<task_id>.<suffix>`:
- `task.T1.context`
- `task.T1.knowledge`
- `task.T1.executor`
- `task.T1.gate`
- `task.T1.success`

### Intra-Task Edge Count (16)

| Edge Pattern | Count | Condition |
|---|---|---|
| context -> 6 enrichers | 6 | `Always` |
| 6 enrichers -> compose | 6 | `Always` |
| context -> compose (direct 7th input) | 1 | `Always` |
| compose -> executor | 1 | `Always` |
| executor -> gate | 1 | `Success` |
| gate -> success | 1 | `Success` |
| **Total** | **16** | |

---

## Inter-Task Wiring

Predecessor's `SuccessBoundary` connects to dependent's `TaskContextCell`
via a `Success` edge. This means a task's 6 enrichers begin only after all
predecessor tasks have passed their gates.

```
task.T1.success ---(Success)---> task.T2.context
```

### Scale Examples

| Plan Shape | Tasks | Nodes | Intra-Task Edges | Inter-Task Edges | Total Edges |
|---|---|---|---|---|---|
| Single task | 1 | 11 | 16 | 0 | 16 |
| Linear chain (T1->T2) | 2 | 22 | 32 | 1 | 33 |
| Diamond (T1->{T2,T3}->T4) | 4 | 44 | 64 | 4 | 68 |
| 3 parallel (no deps) | 3 | 33 | 48 | 0 | 48 |

---

## Construction Phases

`ProductionPlanTopology::build(tasks)` executes four phases:

### Phase 1: Validate Dependencies

Every `depends_on` reference must name a task present in the plan. Missing
references fail with `GraphError::InvalidGraph`:

```
task 'T2' in plan 'my-plan' depends on 'T_MISSING', which does not exist
```

### Phase 2: Build Per-Task Subgraphs

For each task, `add_task_subgraph()` adds 11 nodes and 16 intra-task edges.
Each node receives a TOML config blob built from `TopologyTaskInfo` fields.

### Phase 3: Wire Inter-Task Dependencies

Predecessor's `task.<dep_id>.success` connects to dependent's
`task.<task_id>.context` via a `Success` edge.

### Phase 4: Cycle Detection

`topo::topological_order()` runs on the complete graph. Cycles fail with
`GraphError::CycleDetected`.

---

## TopologyTaskInfo

Minimal task information needed to build a topology:

```rust
pub struct TopologyTaskInfo {
    pub task_id:          String,
    pub title:            String,
    pub description:      Option<String>,
    pub role:             Option<String>,
    pub tier:             String,
    pub model_hint:       Option<String>,
    pub files:            Vec<String>,
    pub depends_on:       Vec<String>,
    pub timeout_secs:     u64,
    pub max_retries:      u32,
    pub domain:           Option<String>,
    pub sequence:         usize,
    pub full_config_json: serde_json::Value,
}
```

The converter constructs this from `TaskDef` entries parsed from `tasks.toml`.

---

## TopologyReport

Returned after construction:

```rust
pub struct TopologyReport {
    pub total_nodes:  usize,
    pub total_edges:  usize,
    pub task_count:   usize,
    pub entry_tasks:  Vec<String>,   // no predecessors
    pub exit_tasks:   Vec<String>,   // no dependents
}
```

---

## Cell Registration

`register_topology_cells(registry)` registers all production plan cell types:

| Cell Type | Implementation | `is_stub` |
|---|---|---|
| `plan.task-context` | `TaskContextCell` | false |
| `plan.enricher.knowledge` | `PassthroughCell` (stub) | true |
| `plan.enricher.episodes` | `PassthroughCell` (stub) | true |
| `plan.enricher.playbook` | `PassthroughCell` (stub) | true |
| `plan.enricher.modulation` | `PassthroughCell` (stub) | true |
| `plan.enricher.safety` | `PassthroughCell` (stub) | true |
| `plan.enricher.experiment` | `PassthroughCell` (stub) | true |
| `plan.compose` | `PlanComposeCell` | false |
| `plan.gate` | `PlanGateCell` | false |
| `plan.success-boundary` | `PassthroughCell` (stub) | true |

Enricher stubs pass through input signals unchanged. Real implementations
are provided by host adapters. The stubs allow the graph to load, validate,
and execute in test environments. Production starts with
`allow_test_stubs = false` will reject graphs containing these stubs.

---

## Cleanup

A `GuaranteedFinallyController` was drafted in `crates/roko-graph/src/finally.rs`, but
roko-graph never compiled it (its `lib.rs` declared no `mod finally`), and it was deleted
on 2026-10-01 (gap-ff6e83). A plan run's cleanup lives in `run_one_plan`
(`crates/roko-cli/src/graph_execution/plan_runner.rs`).

---

## Verification

```bash
cargo test -p roko-graph --lib topology::tests
```

Topology test coverage:
- Single task produces 11 nodes, 16 edges.
- Two-task linear chain: 22 nodes, 33 edges.
- Diamond dependency: 44 nodes, 68 edges.
- Parallel tasks (no deps): 33 nodes, 48 edges.
- Missing dependency errors.
- Duplicate task ID errors.
- Cycle detected.
- Node IDs follow `task.<id>.<suffix>` convention.
- Executor and gate are Activity class; enrichers are Workflow.
- Max parallel clamped to at least 1.
- Context config contains task metadata.
- Graph validates with topology registry.
