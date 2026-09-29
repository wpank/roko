# Production Plan Topology

> ProductionPlanTopology builds the canonical per-task 11-node subgraph that
> transforms a `tasks.toml` plan into an executable Graph. This file documents
> the subgraph structure, node roles, inter-task wiring, cell registration,
> and the GuaranteedFinallyController.

---

## Source Files

| File | What |
|---|---|
| `crates/roko-graph/src/topology.rs` | ProductionPlanTopology, TopologyTaskInfo, register_topology_cells |
| `crates/roko-graph/src/finally.rs` | GuaranteedFinallyController, TerminalReceipt, FinallyGuard |

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

## GuaranteedFinallyController

The controller wraps graph execution with an absolute guarantee that cleanup
runs regardless of how execution ends.

### Guarantees

| Exit Path | Terminal Receipt | Resource Release | Snapshot Flush |
|---|---|---|---|
| All tasks succeed | `TerminalOutcome::Success` | Yes | Yes |
| One or more tasks fail | `TerminalOutcome::Failure` | Yes | Yes |
| Operator cancels | `TerminalOutcome::Cancelled` | Yes | Yes |
| Process panic | Logged via `FinallyGuard::drop` | Requires caller retry | No |

### Construction

```rust
let controller = GuaranteedFinallyController::new("plan-id", "run-id")
    .with_releaser(releaser)     // host-provided ResourceReleaser
    .with_flusher(flusher)       // host-provided SnapshotFlusher
    .with_cancel(cancel_token);  // CancellationToken
```

### Resource Tracking

```rust
// Track resources as they're acquired during execution
controller.track(TrackedResource::WorkspaceLease { lease_id, plan_id, task_id });
controller.track(TrackedResource::AgentProcess { process_id, task_id });
controller.track(TrackedResource::LockFile { path });

// Early explicit release
controller.untrack(|r| matches!(r, TrackedResource::WorkspaceLease { lease_id, .. } if lease_id == "L1"));
```

### TrackedResource Types

| Type | Fields | What |
|---|---|---|
| `WorkspaceLease` | lease_id, plan_id, task_id | Git worktree lease |
| `AgentProcess` | process_id, task_id | Running agent subprocess |
| `LockFile` | path | Filesystem lock file |

### Execution

```rust
let (receipt, cleanup) = controller.execute_with_finally(engine, ctx).await;
```

1. Runs the graph engine (with cancellation support via `tokio::select!`).
2. Determines the terminal outcome from graph output.
3. Runs the finally block: release resources, flush snapshot, emit receipt.
4. The `FinallyGuard` is marked cleaned up.

### TerminalReceipt

Exactly one per execution:

```rust
pub struct TerminalReceipt {
    pub run_id:           String,
    pub plan_id:          String,
    pub outcome:          TerminalOutcome,   // Success | Failure | Cancelled
    pub duration:         Duration,
    pub tasks_succeeded:  usize,
    pub tasks_failed:     usize,
    pub tasks_skipped:    usize,
    pub tasks_cancelled:  usize,
    pub error:            Option<String>,
    pub created_at_ms:    u64,
}
```

Downstream consumers (FeedbackSettler, delivery, GitHub integration) drive
their workflows from this receipt.

### CleanupSummary

```rust
pub struct CleanupSummary {
    pub resources_released: usize,
    pub resources_failed:   usize,
    pub snapshot_flushed:   bool,
    pub receipt_emitted:    bool,
    pub errors:             Vec<String>,
}
```

### FinallyGuard

An explicit guard struct tracks whether cleanup has been performed. If dropped
without `mark_cleaned_up()` (programming error or panic), it logs a diagnostic
error via `tracing::error!`. This catches bugs where the caller forgets to
call the finally block.

### ResourceReleaser Trait

```rust
#[async_trait]
pub trait ResourceReleaser: Send + Sync + Debug {
    async fn release(&self, resource: &TrackedResource) -> Result<(), String>;
}
```

Implementations must be **idempotent**: releasing an already-released resource
is a no-op, not an error. Release failures are logged but never fatal -- they
do not prevent other resources from being released.

---

## Verification

```bash
cargo test -p roko-graph --lib topology::tests
cargo test -p roko-graph --lib finally::tests
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

Finally controller test coverage:
- Success produces terminal receipt with correct fields.
- Failure still produces receipt and cleans up resources.
- Cancel produces cancelled receipt and releases resources.
- Resource release failure is logged, not fatal.
- Snapshot flush failure is logged, not fatal.
- Track and untrack resources correctly.
- Receipt counts reflect graph output node results.
- All tracked resources released on success.
- Diamond DAG produces exactly one terminal receipt.
