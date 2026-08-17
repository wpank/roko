# 05 — Execution Engine

> A single runtime for ALL Graphs. Manages Flow lifecycle, Hot Graph execution, deterministic replay via the Workflow/Activity split, failure strategies, resumability, budget enforcement, and concurrency. Every lifecycle event is a Pulse on Bus.

**Subsumes**: PlanRunner, DAG executor, orchestrate.rs, ProcessSupervisor.

---

## 1. Engine Overview

The execution engine is the single runtime that interprets all Graphs. Plans, agent pipelines, learning loops, dream cycles, trigger chains, and verification suites are all Graphs executed by the same engine. There is no separate executor per specialization.

```rust
/// The execution engine. Interprets Graphs, manages Flows, enforces
/// budgets, handles failures, and publishes lifecycle Pulses.
pub struct Engine {
    /// Cell registry: all registered Cells available for execution.
    pub registry: Arc<CellRegistry>,

    /// Bus for lifecycle Pulses.
    pub bus: Arc<dyn Bus>,

    /// Store for Signal persistence.
    pub store: Arc<dyn Store>,

    /// Active Flows (both standard and Hot).
    pub flows: DashMap<RunId, FlowHandle>,

    /// Global budget tracker.
    pub budget: Arc<BudgetTracker>,

    /// Concurrency limiter.
    pub semaphore: Arc<tokio::sync::Semaphore>,

    /// Cancellation token for engine-wide shutdown.
    pub cancel: CancellationToken,
}

pub struct FlowHandle {
    pub flow: Arc<RwLock<Flow>>,
    pub join_handle: JoinHandle<Result<Vec<Signal>, CellError>>,
    pub cancel: CancellationToken,
}
```

### Engine API

```rust
impl Engine {
    /// Start a new Flow from a Graph definition.
    pub async fn start(&self, graph: Graph, input: Vec<Signal>) -> Result<RunId>;

    /// Resume a Flow from a snapshot.
    pub async fn resume(&self, snapshot: FlowSnapshot) -> Result<RunId>;

    /// Cancel a running Flow.
    pub async fn cancel(&self, run_id: &RunId, reason: &str) -> Result<()>;

    /// Pause a running Flow (Hot Flows only — standard Flows cannot pause mid-node).
    pub async fn pause(&self, run_id: &RunId, reason: &str) -> Result<()>;

    /// Get the current state of a Flow.
    pub async fn status(&self, run_id: &RunId) -> Result<FlowStatus>;

    /// List all active Flows.
    pub async fn list_active(&self) -> Vec<(RunId, FlowStatus)>;

    /// Register a Hot Graph. Starts its clock-driven execution loop.
    pub async fn register_hot(&self, graph: Graph, initial_state: Vec<Signal>) -> Result<RunId>;

    /// Deregister a Hot Graph. Flushes state and stops ticking.
    pub async fn deregister_hot(&self, run_id: &RunId) -> Result<Vec<Signal>>;
}

pub struct FlowStatus {
    pub run_id: RunId,
    pub graph_name: String,
    pub state: FlowState,
    pub nodes_completed: usize,
    pub nodes_total: usize,
    pub cost_so_far: Cost,
    pub elapsed: Duration,
    pub current_nodes: Vec<NodeId>,
}
```

---

## 2. Flow Lifecycle

Every Graph execution follows a standard lifecycle, published as Pulses on Bus.

```
Created ──► Running ──► Completed
                │
                ├──► Failed
                │
                ├──► Cancelled
                │
                └──► Paused ──► Running (resume)
```

```rust
pub enum FlowState {
    /// Flow has been created but execution has not started.
    Created,

    /// Flow is actively executing nodes.
    Running,

    /// Flow execution is paused (can be resumed).
    /// Only Hot Flows and human-in-loop Flows can pause.
    Paused { reason: String },

    /// All exit nodes completed successfully.
    Completed { outputs: Vec<Signal> },

    /// A node failed and the failure strategy could not recover.
    Failed { error: CellError, failed_node: NodeId },

    /// Flow was cancelled by external request.
    Cancelled { reason: String, cancelled_by: Option<AgentId> },
}
```

### Lifecycle Pulses

Every state transition publishes a Pulse on Bus. These Pulses are the sole source of execution observability — there is no separate monitoring channel.

| Transition | Topic | Graduates? | Body |
|---|---|---|---|
| Created | `flow.{run_id}.created` | Yes | Graph name, input count, policy |
| Running | `flow.{run_id}.started` | Yes | Start timestamp |
| Node started | `flow.{run_id}.node.{node_id}.started` | No | Node label |
| Node completed | `flow.{run_id}.node.{node_id}.completed` | Yes (batch) | Output count, cost, duration |
| Node failed | `flow.{run_id}.node.{node_id}.failed` | Yes | Error, attempt count |
| Node retrying | `flow.{run_id}.node.{node_id}.retrying` | No | Attempt number, strategy |
| Paused | `flow.{run_id}.paused` | Yes | Reason |
| Resumed | `flow.{run_id}.resumed` | Yes | Snapshot reference |
| Completed | `flow.{run_id}.completed` | Yes | Output refs, total cost, duration |
| Failed | `flow.{run_id}.failed` | Yes | Error, failed node |
| Cancelled | `flow.{run_id}.cancelled` | Yes | Reason, cancelled by |
| Budget warning | `flow.{run_id}.budget.warning` | Yes | Spent, remaining, threshold |
| Budget exhausted | `flow.{run_id}.budget.exhausted` | Yes | Spent, limit |
| Snapshot taken | `flow.{run_id}.snapshot` | No | Snapshot path |

---

## 3. Node Execution

The engine executes nodes in topological order, respecting edge conditions and the Graph's parallelism limits.

### Execution loop (per node)

```
1. Check preconditions:
   - All upstream nodes completed (or quorum met for FanIn)
   - Edge conditions evaluate to true
   - Budget sufficient for estimated cost
   - Cancellation token not triggered

2. Check execution class (Workflow/Activity split):
   - If Activity AND resuming from snapshot AND output exists:
     → Return recorded output (no re-execution)
   - Otherwise: proceed to step 3

3. Apply input mappings:
   - Transform upstream outputs via edge Mapping expressions

4. Pre-verify (if Verify protocol configured for this scope):
   - Call verify_pre() with input Signals and action plan
   - If hard_pass = false: skip node (PreVerifyVeto error)

5. Publish prediction (predict-publish-correct):
   - Cell publishes prediction Pulse on prediction.{block_id}

6. Execute the Cell:
   - Pass input Signals + CellContext to block.execute()
   - Start verify_stream() polling if configured
   - Enforce node-level timeout
   - Track cost via BudgetTracker

7. Post-verify (if Verify protocol configured):
   - Call verify_post() with input + output Signals
   - Record Verdict (reward, criteria, evidence)

8. Record output:
   - If Activity: write output to runs/<run-id>/activities/<node-id>.json
   - Publish completion Pulse
   - Snapshot state (if policy.snapshot_after_each_node)

9. Propagate:
   - Evaluate downstream edge conditions
   - Enqueue eligible downstream nodes
```

### Parallel execution

Nodes with satisfied preconditions and no mutual ordering constraints execute concurrently, up to `policy.max_parallelism`. FanOut nodes explicitly fork execution. The engine uses a Tokio task-per-node model with a shared semaphore for concurrency limiting.

```rust
pub struct ConcurrencyController {
    /// Maximum concurrent node executions.
    pub semaphore: Arc<tokio::sync::Semaphore>,

    /// Currently executing nodes.
    pub active: DashMap<NodeId, JoinHandle<Result<Vec<Signal>, CellError>>>,
}
```

---

## 4. Hot Graph Execution

Hot Graphs ([doc-03](03-GRAPH.md) S8) stay resident in memory and re-fire on each tick of their bound clock. The engine manages Hot Graphs as long-lived tasks.

### Tick loop

```
1. Wait for next clock tick (or external trigger)

2. Prepare entry node inputs:
   - Previous tick's exit outputs (retained in memory)
   - New external inputs since last tick
   - Updated CorticalState (if Agent-owned)

3. Execute the Graph (same node execution loop as standard Flows)

4. Retain exit node outputs for next tick

5. Optionally checkpoint state to disk (periodic, not every tick)

6. Publish tick Pulse on agent:{id}.tick.completed
```

### State retention

Between ticks, the Hot Flow retains:
- All node outputs from the previous tick
- Graph-level variables
- Accumulated cost
- CorticalState updates (via atomic writes)

### Teardown

When the owning Agent enters the Terminal lifecycle state ([doc-04](04-SPECIALIZATIONS.md) S10), the Hot Flow:
1. Completes the current tick (if in progress)
2. Flushes retained state to Store
3. Publishes a final `flow.{run_id}.terminated` Pulse
4. Releases all resources

---

## 5. Workflow/Activity Split for Deterministic Replay

The Workflow/Activity split ([doc-03](03-GRAPH.md) S6) enables resumability from snapshots without re-executing non-deterministic operations.

### Replay procedure

When `Engine::resume(snapshot)` is called:

```
1. Load FlowSnapshot from disk

2. For each node in topological order:
   a. If node.state == Completed:
      - If Workflow: skip (output already in snapshot)
      - If Activity: load output from runs/<run-id>/activities/<node-id>.json
      - Mark as completed in the resumed Flow

   b. If node.state == Running:
      - If Workflow: re-execute (deterministic, same result)
      - If Activity: re-execute (was interrupted, output not recorded)

   c. If node.state == Pending:
      - Execute normally (standard execution loop)

3. Resume normal execution from first non-completed node
```

### Activity recording

Every Activity node's output is written to persistent storage immediately after execution:

```rust
pub struct ActivityRecord {
    pub node_id: NodeId,
    pub run_id: RunId,
    pub output: Vec<Signal>,
    pub cost: Cost,
    pub duration: Duration,
    pub completed_at: DateTime<Utc>,
    /// SHA-256 of the serialized output, for integrity verification.
    pub output_hash: ContentHash,
}
```

Recording is synchronous (flushed before proceeding to downstream nodes). This ensures that a crash between Activity completion and downstream execution does not lose the Activity output.

---

## 6. Failure Strategies

When a node fails, the engine applies the failure strategy from the node's override or the Graph-level policy. Strategies are ordered from simplest to most complex.

### Fail

Terminate the entire Flow immediately. The failed node's error propagates to the Flow state.

### Retry

Re-execute the failed node up to N times with configurable backoff. Each retry publishes a `retrying` Pulse.

```rust
pub struct RetryState {
    pub attempts: u32,
    pub max_retries: u32,
    pub backoff: BackoffStrategy,
    pub errors: Vec<CellError>,
}
```

### RetryWithEscalation

Retry, but on each attempt, use the Route protocol to select a more capable (and typically more expensive) Cell or model. The first attempt might use a fast/cheap model; subsequent attempts escalate to slower/better ones.

```
Attempt 1: claude-haiku → fail
Attempt 2: Route selects claude-sonnet → fail
Attempt 3: Route selects claude-opus → succeed
```

The Route protocol's `alternatives` field from the initial routing decision provides the escalation candidates.

### Decompose

Break the failed task into smaller sub-tasks and retry each. An LLM generates the decomposition from the failure context. The decomposed sub-tasks form a new sub-Graph that replaces the failed node.

```rust
pub struct DecomposeState {
    pub original_node: NodeId,
    pub sub_graph: Graph,
    pub depth: u32,
    pub max_depth: u32,
}
```

### Skip

Skip the failed node and continue execution. Downstream nodes receive empty input from the skipped node. Useful for non-critical nodes (e.g., optional telemetry).

### Compensate

Execute a compensation Graph to undo partial work from the failed node. The compensation Graph receives the failed node's partial output and error as input.

### Replan

Generate a new plan from the failure context and execute it. Uses the same plan-generation pipeline as `roko prd plan`. The replan receives:
- The original task description
- The failure error and context
- Prior attempt outputs
- Available Heuristics about the failure mode

```rust
pub struct ReplanState {
    pub original_task: Signal,
    pub failure_context: Value,
    pub prior_attempts: Vec<(Vec<Signal>, CellError)>,
    pub replan_count: u32,
    pub max_replans: u32,
}
```

### HumanResolve

Pause the Flow and publish a `human.resolution.requested` Pulse on Bus. The human provides resolution input through the TUI, CLI, or API. If the timeout expires without human input, the failure escalates to the next strategy.

---

## 7. Resumability

The engine supports resuming any Flow from a snapshot. Snapshots are written:
- After each node completion (if `policy.snapshot_after_each_node = true`)
- Periodically (configurable interval)
- On graceful shutdown

### Snapshot format

```rust
pub struct FlowSnapshot {
    pub run_id: RunId,
    pub graph_id: GraphId,
    pub graph_version: Version,
    pub created_at: DateTime<Utc>,
    pub snapshot_at: DateTime<Utc>,

    /// Per-node execution state.
    pub node_states: BTreeMap<NodeId, NodeState>,

    /// Graph-level variables.
    pub variables: BTreeMap<String, Value>,

    /// Total cost spent so far.
    pub cost_so_far: Cost,

    /// Total elapsed wall-clock time.
    pub elapsed: Duration,

    /// Budget remaining at snapshot time.
    pub budget_remaining: Cost,

    /// References to Activity records for replay.
    pub activity_records: Vec<ContentHash>,
}
```

### CLI integration

```bash
# Start a plan
roko plan run plans/my-plan/

# Resume from the latest snapshot
roko plan run plans/my-plan/ --resume .roko/state/executor.json

# Resume a specific snapshot
roko plan run plans/my-plan/ --resume .roko/runs/<run-id>/state.json
```

---

## 8. Cancellation Propagation

Cancellation propagates from parent to child, from Flow to nodes, and from Agent to all owned Flows.

```rust
/// Cancellation propagation chain:
///
/// Engine shutdown
///   └─► All Flows cancelled
///       └─► All running nodes cancelled
///           └─► Cell.execute() checks cancel token
///               └─► Sub-Graphs cancelled recursively
///
/// Agent.terminate()
///   └─► All Agent-owned Flows cancelled
///       └─► Hot Flow teardown
///
/// Flow.cancel(reason)
///   └─► All running nodes receive CancellationToken signal
///       └─► Cells check token between steps
```

Cancellation is cooperative: Cells check the `CancellationToken` between steps. Long-running Cells (LLM calls) also register abort handlers with the underlying HTTP client.

```rust
impl CellContext {
    /// Check if cancellation has been requested.
    /// Cells SHOULD call this between significant operations.
    pub fn is_cancelled(&self) -> bool {
        self.cancel.is_cancelled()
    }
}
```

---

## 9. Budget Enforcement

The engine enforces budgets at three levels: per-node, per-Flow, and per-Agent (global). Budget tracking is demurrage-aware: the cost of holding Signals in context (via demurrage) is included in the budget.

```rust
pub struct BudgetTracker {
    /// Per-Flow budget ledgers.
    pub flow_budgets: DashMap<RunId, BudgetLedger>,

    /// Per-Agent budget ledgers.
    pub agent_budgets: DashMap<AgentId, BudgetLedger>,
}

pub struct BudgetLedger {
    pub initial: Cost,
    pub spent: Cost,
    pub reserved: Cost,
    pub warning_threshold: f64,

    /// Per-Cell cost breakdown.
    pub block_costs: BTreeMap<CellRef, Cost>,

    /// Per-node cost breakdown.
    pub node_costs: BTreeMap<NodeId, Cost>,

    /// Demurrage costs (cost of holding Signals in context).
    pub demurrage_costs: Cost,
}

impl BudgetLedger {
    pub fn remaining(&self) -> Cost {
        Cost(self.initial.0.saturating_sub(self.spent.0 + self.reserved.0))
    }

    pub fn utilization(&self) -> f64 {
        self.spent.0 as f64 / self.initial.0 as f64
    }

    pub fn should_warn(&self) -> bool {
        self.utilization() >= self.warning_threshold
    }
}
```

### Budget enforcement behavior

| Condition | Action |
|---|---|
| Estimated cost > remaining budget | Cell not started; `BudgetExhausted` error |
| Actual cost exceeds estimate by >2x | Warning Pulse published |
| Budget utilization > warning threshold (default 80%) | `budget.warning` Pulse published |
| Budget exhausted mid-Flow | Flow fails with `BudgetExhausted` |
| Agent vitality < 0.05 | Agent enters Terminal phase; all Flows wind down |

---

## 10. Human-in-the-Loop

`HumanInput` nodes pause execution and wait for human input. The engine publishes a request Pulse and blocks the node until input is received or the timeout expires.

```rust
pub struct HumanInputRequest {
    pub run_id: RunId,
    pub node_id: NodeId,
    pub prompt: String,
    pub input_schema: Option<TypeSchema>,
    pub timeout: Option<Duration>,
    pub requested_at: DateTime<Utc>,
}

pub struct HumanInputResponse {
    pub run_id: RunId,
    pub node_id: NodeId,
    pub input: Vec<Signal>,
    pub responded_at: DateTime<Utc>,
    pub responder: Author,
}
```

Human input can arrive via:
- **TUI**: Interactive prompt in the dashboard
- **CLI**: `roko flow respond <run-id> <node-id> <payload>`
- **API**: `POST /flows/{run_id}/nodes/{node_id}/input`
- **Slack**: via the Slack MCP connector

---

## 11. Concurrency and Parallelism

The engine uses Tokio for async execution with explicit concurrency controls.

### Parallelism levels

| Level | Control | Default |
|---|---|---|
| Engine-wide | `Engine.semaphore` | 32 concurrent nodes |
| Per-Flow | `GraphPolicy.max_parallelism` | 8 concurrent nodes |
| Per-Agent | `Agent.slots.max_slots` | 4 concurrent slots |
| Per-FanOut | Number of downstream edges | Unbounded (limited by Flow) |

### Ordering guarantees

- **Within a node**: sequential execution (Cells are not internally parallelized by the engine).
- **Within a Flow**: topological order respected. Parallel nodes are truly concurrent.
- **Across Flows**: no ordering guarantees. Flows are independent.
- **Hot Graph ticks**: sequential (tick N completes before tick N+1 starts).

---

## 12. Episode Logging

Every completed Flow produces an Episode Signal persisted to `.roko/episodes.jsonl`. Episodes feed the learning loops ([doc-10](10-LEARNING-LOOPS.md)).

```rust
pub struct Episode {
    pub run_id: RunId,
    pub graph_name: String,
    pub agent_id: Option<AgentId>,
    pub started_at: DateTime<Utc>,
    pub completed_at: DateTime<Utc>,

    /// Per-node summaries.
    pub node_summaries: Vec<NodeSummary>,

    /// Final Verdict (if Verify was invoked).
    pub verdict: Option<Verdict>,

    /// Total cost.
    pub total_cost: Cost,

    /// Vitality at start and end.
    pub vitality_start: Option<f64>,
    pub vitality_end: Option<f64>,

    /// Somatic state at end.
    pub somatic: Option<PadVector>,

    /// HDC fingerprint of this episode (for similarity search).
    pub hdc_fingerprint: HdcVector,

    /// Model(s) used.
    pub models: Vec<String>,

    /// Calibration data: predictions vs outcomes.
    pub calibration: Vec<CalibrationUpdate>,
}

pub struct NodeSummary {
    pub node_id: NodeId,
    pub block_ref: CellRef,
    pub execution_class: ExecutionClass,
    pub cost: Cost,
    pub duration: Duration,
    pub verdict: Option<Verdict>,
}
```

---

## 13. Cascade Router Integration

The engine integrates with the Route protocol ([doc-02](02-CELL.md) S2.4) for model selection at every LLM-backed Cell execution. The cascade router uses EFE (Expected Free Energy, Friston 2006), not LinUCB, to balance exploration and exploitation.

### Progressive cascade

```
T0: Pattern match (pure Rust, no LLM, ~80% of ticks, $0 cost)
  │
  ├─ Match found → execute reflex → verify → done
  │
  └─ No match → escalate to T1

T1: Fast/cheap model (Haiku-class, <$0.001 per call)
  │
  ├─ EFE score below threshold → execute → verify → done
  │
  └─ EFE score above threshold → escalate to T2

T2: Capable model (Sonnet/Opus-class, $0.01-0.10 per call)
  │
  └─ Execute → verify → done
```

The EFE threshold adapts based on:
- **Regime**: Crisis mode forces T0/T1 only
- **Vitality**: Low vitality reduces exploration
- **Task complexity**: Simple tasks stay at T0/T1
- **Historical performance**: Cells that consistently fail at lower tiers are directly routed to higher tiers

### Route context construction

```rust
pub fn build_route_context(
    agent: &Agent<Active>,
    task: &Signal,
) -> RouteContext {
    RouteContext {
        regime: agent.cortical_state.regime(),
        vitality: agent.vitality.ratio(),
        complexity: estimate_complexity(task),
        urgency: compute_urgency(task, &agent.vitality),
        budget_remaining: agent.vitality.remaining_budget,
        context_signals: vec![], // filled by Compose
    }
}
```

---

## 14. Lens Integration

The engine publishes all lifecycle events as Pulses on Bus. Lenses ([doc-04](04-SPECIALIZATIONS.md) S5) subscribe to these events and produce observation Signals for surfaces (TUI, web, API, Slack, audit).

### Built-in engine Lenses

| Lens | Subscribes to | Produces |
|---|---|---|
| `FlowProgressLens` | `flow.*.node.*.completed` | Progress percentages per Flow |
| `CostLens` | `cost.charged`, `flow.*.budget.*` | Real-time cost telemetry |
| `FailureLens` | `flow.*.node.*.failed` | Failure analysis with suggested strategies |
| `LatencyLens` | `flow.*.node.*.completed` | Per-node and per-Flow latency distributions |
| `ConcurrencyLens` | `flow.*.started`, `flow.*.completed` | Active Flows and parallelism utilization |

---

## 15. Cost and Time Estimation

Before executing a Graph, the engine provides cost and time estimates by summing Cell-level estimates along the critical path.

```rust
pub struct GraphEstimate {
    /// Estimated total cost (sum of all node costs on critical path).
    pub cost: CostEstimate,

    /// Estimated wall-clock time (critical path duration).
    pub duration: DurationEstimate,

    /// Per-node estimates.
    pub node_estimates: BTreeMap<NodeId, NodeEstimate>,

    /// Whether the estimate is reliable (all Cells provided estimates).
    pub complete: bool,
}

pub struct CostEstimate {
    pub lower: Cost,
    pub expected: Cost,
    pub upper: Cost,
}

pub struct DurationEstimate {
    pub lower: Duration,
    pub expected: Duration,
    pub upper: Duration,
}

pub struct NodeEstimate {
    pub cost: Option<Cost>,
    pub duration: Option<Duration>,
    /// Whether this node is on the critical path.
    pub critical: bool,
}
```

```rust
impl Engine {
    /// Estimate cost and time for a Graph without executing it.
    pub fn estimate(&self, graph: &Graph) -> GraphEstimate;
}
```

---

## 16. Acceptance Criteria

| Criterion | Verification |
|---|---|
| Engine starts a Flow from a Graph and returns a RunId | Integration test |
| Flow lifecycle transitions publish correct Pulses on Bus | Integration test: subscribe, start Flow, verify Pulses |
| All lifecycle Pulses match documented topics and graduation policies | Integration test |
| Node execution follows topological order | Unit test: 3-node chain, verify execution order |
| Parallel nodes execute concurrently up to `max_parallelism` | Integration test: FanOut with 4 branches, max_parallelism=2 |
| Hot Graph re-fires on clock tick | Integration test: register Hot Graph, advance clock, verify two executions |
| Hot Graph retains state between ticks | Integration test: first tick sets variable, second tick reads it |
| Hot Graph teardown flushes state to Store | Integration test: deregister, verify Store contains state |
| Activity replay returns recorded output (no re-execution) | Integration test: execute Flow, resume from snapshot, verify no LLM call on Activity node |
| Workflow replay re-executes deterministically | Integration test: Branch node produces same result on replay |
| Failure strategy Fail terminates Flow immediately | Unit test |
| Failure strategy Retry retries N times with backoff | Integration test with mock Cell that fails then succeeds |
| Failure strategy RetryWithEscalation uses Route for alternatives | Integration test: verify different Cell used on retry |
| Failure strategy Decompose creates sub-Graph | Integration test |
| Failure strategy Replan generates new plan | Integration test |
| Failure strategy HumanResolve publishes request and waits | Integration test with mock human input |
| Snapshot serializes and deserializes round-trip | Unit test |
| Resume from snapshot skips completed nodes | Integration test: 3-node chain, snapshot after node 2, resume executes only node 3 |
| Cancellation propagates from Flow to running nodes | Integration test: cancel Flow, verify node receives cancellation |
| Budget enforcement halts Flow when exhausted | Integration test: set tight budget, verify BudgetExhausted error |
| Budget warning published at 80% utilization | Integration test |
| Demurrage costs tracked in BudgetLedger | Unit test |
| Human-in-loop: HumanInput node pauses and resumes on input | Integration test |
| Episode logged after Flow completion | Integration test: complete Flow, verify episode in `.roko/episodes.jsonl` |
| Episode includes HDC fingerprint | Unit test |
| Cascade router uses EFE: T0 -> T1 -> T2 escalation | Integration test with mocked models |
| Cascade router respects regime (Crisis -> T0/T1 only) | Unit test |
| Lens receives lifecycle Pulses and produces observation Signals | Integration test |
| Cost estimate computed before execution | Unit test: estimate on Graph with known Cell costs |
| Time estimate identifies critical path | Unit test: parallel Graph, verify critical path is longest branch |
| Engine-wide concurrency limited by semaphore | Integration test: start 64 Flows with semaphore=32, verify max 32 concurrent |
