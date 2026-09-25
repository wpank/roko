# Parallel Executor Waves

> Depth file for [04-EXECUTION.md](../../04-EXECUTION.md) section 6.
> Preserves and updates content from v1 `01-orchestration/03-parallel-executor.md`.

---

## Overview

The `GraphEngine` is the sole execution engine. It topologically sorts
nodes and groups them into waves where every node in a wave is independent
of every other. Within each wave, nodes execute concurrently up to a
configured parallelism bound.

**Source:** `crates/roko-graph/src/engine.rs`, `crates/roko-graph/src/topo.rs`

---

## Topological Sort

`topological_order()` uses `petgraph::algo::toposort` to produce a linear
execution order over the internal `DiGraph`. If the graph contains a
cycle, `GraphError::CycleDetected` is returned.

The sort is deterministic: given the same graph definition, it always
produces the same node order. This is important for reproducible
execution and snapshot consistency. Two runs of the same plan will
produce identical wave assignments and execution sequences.

---

## Wave Computation

`topological_waves()` groups the sorted nodes into parallel waves. Within
each wave, all nodes have no edges between them and can execute
concurrently.

### Algorithm

1. Compute the topological order.
2. For each node, compute its wave index as:
   `wave(node) = max(wave(predecessor) + 1 for all predecessors)`.
   Nodes with no predecessors have wave index 0.
3. Group nodes by wave index.
4. Sort nodes within each wave for deterministic ordering.

This is equivalent to computing the "depth" or "layer" of each node in
the DAG, which determines the earliest wave at which it can execute.

### Example

For a production topology with 3 tasks (A, B depends on A, C independent):

```
Wave 0:  [A.context, C.context]
Wave 1:  [A.knowledge, A.episodes, A.playbook, A.modulation, A.safety, A.experiment,
          C.knowledge, C.episodes, C.playbook, C.modulation, C.safety, C.experiment]
Wave 2:  [A.compose, C.compose]
Wave 3:  [A.executor, C.executor]
Wave 4:  [A.gate, C.gate]
Wave 5:  [A.boundary, C.boundary]
Wave 6:  [B.context]
Wave 7:  [B.knowledge, B.episodes, B.playbook, B.modulation, B.safety, B.experiment]
Wave 8:  [B.compose]
Wave 9:  [B.executor]
Wave 10: [B.gate]
Wave 11: [B.boundary]
```

Tasks A and C run in parallel (same waves). Task B starts only after A
completes (its context node is in wave 6, after A's boundary in wave 5).

Note that the enricher nodes for A and C (wave 1) contain 12 nodes total.
If `max_concurrent_nodes` is 8, the engine executes 8 of them immediately
and the remaining 4 wait for a semaphore permit.

---

## Bounded Parallelism

The graph's `policy.max_concurrent_nodes` caps how many nodes execute
simultaneously within a wave. The engine uses a Tokio semaphore for
concurrency limiting.

```rust
pub struct GraphPolicy {
    pub max_concurrent_nodes: usize,   // default: 8
    pub timeout: Option<Duration>,
    pub fail_fast: bool,
}
```

When a wave has more ready nodes than `max_concurrent_nodes`, overflow
nodes wait for a semaphore permit before starting. The semaphore ensures
resource bounds are respected even when the DAG has high inherent
parallelism.

---

## Conditional Routing

Edges may carry conditions that determine whether downstream nodes
execute:

```rust
pub enum EdgeCondition {
    Success,         // downstream runs only if upstream succeeded
    Failure,         // downstream runs only if upstream failed
    Always,          // downstream always runs (default)
    OutputEquals {    // downstream runs if upstream output matches
        key: String,
        value: String,
    },
}
```

After each node completes, the engine evaluates conditions on outgoing
edges. Nodes whose incoming conditions are all unmet receive
`NodeStatus::ConditionSkipped` -- a successful no-op, not a failure.
Condition-skipped nodes do not propagate failure downstream.

### Use cases

Conditional routing enables patterns that were impossible in Runner-v2:

- **Skip enrichment when cached**: if a knowledge cell's output is
  already in the prompt cache, the edge condition can skip re-querying.
- **Error-path handling**: failure edges can route to diagnostic or
  cleanup nodes.
- **Output-dependent branching**: an executor's output can determine
  which downstream path to follow.

---

## Node Execution Loop

For each node in topological order:

```
1. Resolve activation:
   - Root node                    -> use root_inputs
   - All upstream edges satisfied -> Ready(collected_outputs)
   - No conditional route selected -> ConditionSkipped
   - Required upstream failed      -> UpstreamFailed (Skipped)

2. If Activity AND replayer has recorded output:
   -> return recorded output (skip re-execution)

3. Look up Cell in CellRegistry by cell_type.

4. Execute the Cell with inputs + CellContext.

5. If Activity AND recorder present:
   -> append output to JSONL activity log.

6. Track budget (microdollars via AtomicU64).

7. Update node status and propagate to downstream edges.
```

Steps 2 and 5 implement the replay infrastructure. On a fresh run, step 2
never fires (no recorded outputs) and step 5 records everything. On
resume, step 2 fires for completed Activity nodes and step 5 records only
newly executed Activities.

### NodeStatus

```rust
pub enum NodeStatus {
    Pending,           // not yet started
    Running,           // currently executing
    Complete,          // finished successfully
    Failed,            // execution error
    Skipped,           // upstream failure
    ConditionSkipped,  // no conditional route selected (not a failure)
}
```

`ConditionSkipped` is distinct from `Skipped`. A condition-skipped node
was not needed (its conditional route was not selected). A skipped node
was needed but could not execute because an upstream dependency failed.

---

## CellRegistry

The `CellRegistry` maps `cell_type` strings to `Cell` implementations:

```rust
pub trait Cell: Send + Sync {
    fn execute(
        &self,
        inputs: Vec<Signal>,
        ctx: &CellContext,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<Signal>, CellError>> + Send>>;
}
```

The default registry includes cells for all production topology node types
plus the simple `task-executor` cell used by `plan_to_graph()`. Custom
cells can be registered for authored graphs.

---

## Budget Enforcement

The `BudgetTracker` monitors resource consumption during execution:

```rust
pub struct BudgetTracker {
    tokens_used: AtomicU64,
    cost_microdollars: AtomicU64,   // 1 USD = 1,000,000 microdollars
    start_time: Instant,
    limits: BudgetLimits,
    breakdown: Mutex<Vec<NodeCost>>,
}
```

Before each Activity node execution, the engine checks whether any budget
limit (tokens, cost, deadline) would be exceeded. If so, the node receives
`BudgetExceeded` and is not executed.

Atomic reservations are persisted before dispatch. On resume, the schema-v2
cost sidecar restores the exact budget state. Missing, corrupt, or
mismatched cost state fails closed -- the engine refuses to resume rather
than risking over-spend.

---

## Fail-Fast vs. Complete Modes

The `GraphPolicy.fail_fast` flag controls behavior on node failure:

- **fail_fast = true**: when any node fails, the engine cancels all
  running nodes and skips all pending nodes. Useful during development
  when early failure detection saves cost.

- **fail_fast = false**: the engine continues executing nodes that do
  not depend on the failed node. Independent branches continue to
  completion. Useful for maximizing progress when some tasks are
  independent.

---

## Process Supervision

The `ProcessSupervisor` (from `roko-runtime`) tracks spawned agent
processes. When a plan completes or fails, the supervisor ensures all
child processes are terminated. The `GuaranteedFinallyController`
invokes supervisor shutdown as part of its cleanup guarantee.

The shutdown sequence is: cancel token, SIGTERM, 10-second grace period,
SIGKILL. This ensures that runaway agent processes do not outlive their
parent execution.

---

## Observability

Each node execution emits structured events through the telemetry sink:

| Event | When | Data |
|---|---|---|
| `node.started` | Node begins execution | node_id, cell_type, wave |
| `node.completed` | Node finishes successfully | node_id, duration, cost |
| `node.failed` | Node fails | node_id, error, duration |
| `node.skipped` | Node skipped (upstream failure) | node_id, reason |
| `wave.started` | Wave begins | wave_index, node_count |
| `wave.completed` | Wave finishes | wave_index, duration |

These events feed into the TUI dashboard, HTTP SSE endpoint, and
telemetry Lens runtime. They provide per-node visibility that was not
available in Runner-v2, where enrichment stages were opaque.

---

## Verification Commands

```bash
# Execute a plan through the Graph engine
cargo run -p roko-cli -- plan run plans/<dir>

# Inspect a graph definition
cargo run -p roko-cli -- graph inspect plans/<dir>

# Run a standalone graph
cargo run -p roko-cli -- graph run <graph-file>
```
