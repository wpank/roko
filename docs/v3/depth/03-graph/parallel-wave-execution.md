# Parallel Wave Execution

> How the Graph Engine executes cells in bounded parallel topological waves.
> Covers the sequential and parallel execution paths, NodeStatus lifecycle,
> NodeActivation decisions, FlowHandle for background execution, and the
> ValidatedGraph proof token.
>
> **Status (2026-09-29, at `7c556bc0a`): the wave scheduling below is stale.**
> Since `445a60d0d` (gap-4d835d) the parallel path is a ready queue
> (`GraphEngine::execute_ready_queue`): each node starts once its own
> predecessors have settled, without waiting for the rest of its wave. See
> [03-GRAPH.md](../../03-GRAPH.md), "Parallel Execution: the Ready Queue".

---

## Source Files

| File | What |
|---|---|
| `crates/roko-graph/src/engine.rs` | GraphEngine, execute, execute_parallel, start, resume_from |
| `crates/roko-graph/src/topo.rs` | topological_waves algorithm |

---

## Wave Computation

`topological_waves()` in `topo.rs` groups nodes by longest-path depth from
root nodes:

```
depth[node] = max(depth[predecessor] + 1 for each predecessor) or 0 if root
```

Nodes at the same depth have no edges between them and may run concurrently.
The algorithm:

1. Compute topological order (via petgraph `toposort`).
2. Walk nodes in order, computing each node's depth from its predecessors.
3. Group by depth. Each group is one wave.

**Invariant:** Within a wave, no node depends on any other node in the same wave.

---

## Execution Paths

The `GraphEngine` provides six execution methods, all requiring a
`ValidatedGraph` proof token from `validate_for_start()`:

| Method | Mode | Concurrency | Use |
|---|---|---|---|
| `execute(ctx)` | OneShot | Sequential | Simple single-threaded runs |
| `execute_parallel(ctx)` | OneShot | Bounded parallel waves | Production plan execution |
| `execute_at_tick(tick, ctx)` | Hot | Sequential, tick-driven | Hot Graph debugging |
| `execute_parallel_at_tick(tick, ctx)` | Hot | Parallel, tick-driven | Production cognitive loop |
| `start()` | OneShot | Background task with FlowHandle | Non-blocking execution |
| `resume_from(snapshot)` | OneShot | Parallel, skip completed | Restart recovery |

### Sequential Execution (`execute`)

Nodes execute one at a time in topological order. For each node:

1. Determine activation (root, ready, condition-skipped, or upstream-failed).
2. If ready: resolve the Cell from the registry, execute it, record if Activity.
3. Check budget after execution.
4. Record `NodeResult` with status, duration, output count.

### Parallel Execution (`execute_parallel`)

Nodes execute in topological waves with bounded concurrency:

1. Compute waves via `topological_waves()`.
2. For each wave:
   a. Evaluate all incoming edge conditions for each node.
   b. Spawn up to `policy.max_concurrent_nodes` concurrent tasks within the wave.
   c. Collect results from all tasks before advancing to the next wave.
3. Handle `FailureStrategy`:
   - `FailFast`: abort after first failure in any wave.
   - `SkipFailed`: mark dependents as `Skipped`, continue.
   - `Retry { max_retries }`: retry failed nodes within the wave.

**Bounded concurrency:** The `max_concurrent_nodes` field (default: 4) limits
how many cells may execute concurrently within a single wave. It does not limit
how many waves run in total. For the cognitive loop, this is set to 1
(strictly sequential).

---

## NodeStatus Lifecycle

```
Pending --> Running --> Complete
                   \-> Failed
                   \-> Skipped        (upstream failure, FailFast/SkipFailed)
                   \-> ConditionSkipped  (no incoming route selected)
```

The `NodeStatus` enum:

```rust
pub enum NodeStatus {
    Pending,           // Not yet started
    Running,           // Currently executing
    Complete,          // Completed successfully
    Failed,            // Failed during execution
    Skipped,           // Skipped because an upstream node failed
    ConditionSkipped,  // Skipped because no incoming conditional edge fired
}
```

`ConditionSkipped` is a successful no-op, not a failure. It does not propagate
downstream failure. A graph where all non-skipped nodes succeed is considered
successful.

---

## NodeActivation Decision

Before executing each node, the engine computes a `NodeActivation`:

```rust
enum NodeActivation {
    Root,                        // A root node, receives root_inputs
    Ready(Vec<Signal>),          // Selected, receives outputs from active edges
    ConditionSkipped(String),    // No conditional route selected the node
    UpstreamFailed(String),      // A required dependency did not complete
}
```

The decision logic:

1. **Root nodes** (no incoming edges): receive `root_inputs` supplied by the
   caller via `with_root_inputs()`.
2. **Non-root nodes**: iterate over all incoming edges.
   - Evaluate each edge's condition against the source node's output.
   - Collect outputs from edges whose conditions fire.
   - If no edge fires and all edges have explicit conditions: `ConditionSkipped`.
   - If an unconditional edge's source failed: `UpstreamFailed`.
3. `Ready` nodes receive the union of outputs from all active incoming edges.

---

## ValidatedGraph Proof Token

All engine entry points require a `ValidatedGraph` token:

```rust
pub struct ValidatedGraph {
    _private: (),  // zero-cost wrapper, cannot be constructed outside engine
}
```

`validate_for_start()` performs:

1. **Type-schema validation:** `graph.validate_edges(&registry)` checks every
   edge for source output / target input schema compatibility.
2. **Stub rejection:** In production mode (`allow_test_stubs = false`), any
   node whose registry descriptor has `is_stub = true` causes
   `GraphError::InvalidGraph`.
3. **Caching:** Once validated, `pre_validated` is set so Hot Graph tick loops
   skip re-validation.

---

## FlowHandle (Background Execution)

`GraphEngine::start()` returns a `FlowHandle` for non-blocking observation:

```rust
pub struct FlowHandle {
    pub run_id:     String,
    pub graph_id:   String,
    pub started_at: Instant,
    // internal: node_statuses, budget_consumed, cancel, result, join_handle
}
```

| Method | Purpose |
|---|---|
| `status()` | Point-in-time `FlowStatus`: per-node statuses, elapsed time, budget consumed |
| `cancel()` | Request early stop; currently-executing node finishes first |
| `await_completion()` | Async wait for final `GraphOutput` |
| `is_running()` | Check if the background task is still alive |

`FlowStatus` carries:
- `node_statuses: HashMap<NodeId, NodeStatus>` -- atomically updated.
- `elapsed: Duration` -- wall-clock since `started_at`.
- `budget_consumed_microdollars: u64` -- from an `AtomicU64`.

---

## GraphOutput

Every execution path returns a `GraphOutput`:

```rust
pub struct GraphOutput {
    pub graph_name:     String,
    pub success:        bool,
    pub node_results:   Vec<NodeResult>,    // in topological order
    pub total_duration: Duration,
}
```

`success` is `true` when all nodes are `Complete` or `ConditionSkipped`.
Failed and Skipped nodes make `success = false`.

### NodeResult

```rust
pub struct NodeResult {
    pub node_id:      NodeId,
    pub cell_type:    String,
    pub status:       NodeStatus,
    pub duration:     Duration,     // zero for skipped nodes
    pub error:        Option<String>,
    pub output_count: usize,
    pub is_stub:      bool,
}
```

`GraphOutput::summary()` formats a human-readable report including a stub
warning when any `is_stub` nodes were used.

---

## Engine Construction

```rust
let engine = GraphEngine::new(graph, registry)
    .with_root_inputs(inputs)          // Signals for root nodes
    .with_recorder(recorder)           // Activity recording to JSONL
    .with_replayer(replayer)           // Activity replay on resume
    .with_merge_queue(queue)           // Merge enqueue on success
    .with_telemetry(telemetry)         // Passive lifecycle sink (Lens)
    .with_event_sink(sink)             // Rich graph execution events (#246)
    .with_allow_test_stubs(false);     // Reject stubs in production
```

Additional engine state:

- `event_seq: EventSeqCounter` -- monotonic sequence for graph event emission.
- `tick_state: Mutex<HashMap<NodeId, Vec<Signal>>>` -- last complete per-node
  outputs for stateful Hot Graph ticks.
- `pre_validated: AtomicBool` -- cached validation result for tick loops.

### Tick State

For Hot Graphs, `restore_tick_state(state)` loads prior per-node outputs.
Unknown node IDs are rejected so a checkpoint from a drifted graph cannot
silently inject state. `tick_state_snapshot()` captures the current state
for checkpointing.

---

## Example: 3-Task Plan Execution

A 3-task plan (T1 -> T2, T1 -> T3) with `ProductionPlanTopology` (11 nodes
per task) produces 33 nodes in ~12 topological waves:

```
Wave 0:  [T1.context]
Wave 1:  [T1.knowledge] [T1.episodes] [T1.playbook] [T1.modulation] [T1.safety] [T1.experiment]
Wave 2:  [T1.compose]
Wave 3:  [T1.executor]  (Activity)
Wave 4:  [T1.gate]      (Activity)
Wave 5:  [T1.success]
Wave 6:  [T2.context] [T3.context]
Wave 7:  [T2.knowledge] [T3.knowledge]  <-- T2 and T3 enrichers run in parallel
         [T2.episodes]  [T3.episodes]
         ...
Wave 8:  [T2.compose]   [T3.compose]
Wave 9:  [T2.executor]  [T3.executor]   <-- parallel LLM dispatch
Wave 10: [T2.gate]      [T3.gate]
Wave 11: [T2.success]   [T3.success]
```

With `max_concurrent_nodes = 2`, waves 6-11 execute T2 and T3 nodes
concurrently, two at a time per wave.

---

## Verification

```bash
# Parallel execution tests
cargo test -p roko-graph --lib engine::tests
# Wave computation tests
cargo test -p roko-graph --lib topo::tests
```
