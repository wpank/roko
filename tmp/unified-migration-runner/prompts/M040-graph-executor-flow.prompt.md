# M040 — Graph Executor with Flow Lifecycle

## Objective
Implement the Graph executor that interprets Graph definitions at runtime, managing Flow lifecycle (Created -> Running -> Completed/Failed/Cancelled/Paused). The executor walks the node graph: ready nodes execute in parallel (respecting a semaphore), completed nodes unlock dependents, and every lifecycle transition emits a Pulse on Bus. Flow is the universal runtime wrapper for all Graph executions.

## Scope
- Crates: `roko-orchestrator`
- Files: `crates/roko-orchestrator/src/graph/executor.rs` (new), `crates/roko-orchestrator/src/graph/flow.rs` (new), `crates/roko-orchestrator/src/graph/mod.rs`
- Phase ref: `tmp/unified-migration/03-PHASE-2-ENGINE.md` SS2.2
- Spec ref: `tmp/unified/05-EXECUTION-ENGINE.md` SS1-4, `tmp/unified/04-SPECIALIZATIONS.md` SS2 (Flow)

## Steps
1. Read the existing plan executor to understand current patterns:
   ```bash
   grep -rn 'PlanRunner\|executor\|DagExecutor' crates/roko-orchestrator/src/ --include='*.rs' | head -20
   grep -rn 'pub async fn run\|pub async fn execute' crates/roko-orchestrator/src/ --include='*.rs' | head -10
   ```

2. Read the Bus trait for Pulse emission:
   ```bash
   grep -rn 'pub trait Bus\|fn publish' crates/roko-core/src/ --include='*.rs' | head -10
   ```

3. Define `Flow` in `crates/roko-orchestrator/src/graph/flow.rs`:
   ```rust
   pub struct Flow {
       pub run_id: RunId,
       pub graph: Graph,
       pub state: FlowState,
       pub node_states: BTreeMap<NodeId, NodeState>,
       pub cost: CostLedger,
       pub started_at: DateTime<Utc>,
       pub agent: Option<AgentId>,
   }

   pub enum FlowState {
       Created,
       Running,
       Paused { reason: String },
       Completed { outputs: Vec<Signal> },
       Failed { error: String },
       Cancelled { reason: String },
   }

   pub enum NodeState {
       Pending,
       Ready,
       Running { started_at: DateTime<Utc> },
       Completed { output: Signal, duration: Duration },
       Failed { error: String, duration: Duration },
       Skipped { reason: String },
   }
   ```

4. Define `Engine` in `crates/roko-orchestrator/src/graph/executor.rs`:
   ```rust
   pub struct Engine {
       pub registry: Arc<CellRegistry>,
       pub bus: Arc<dyn Bus>,
       pub store: Arc<dyn Store>,
       pub flows: DashMap<RunId, FlowHandle>,
       pub budget: Arc<BudgetTracker>,
       pub semaphore: Arc<tokio::sync::Semaphore>,
       pub cancel: CancellationToken,
   }
   ```

5. Implement the core execution loop:
   - `start(graph, input) -> Result<RunId>`: create Flow, emit `flow.created` Pulse, begin execution
   - Walk nodes: compute ready set (all dependencies completed), execute ready nodes in parallel via `tokio::spawn` with semaphore permit
   - On node completion: emit `node.completed` Pulse, update NodeState, check if new nodes are ready
   - On node failure: delegate to failure strategy (M041 will implement strategies; for now, propagate failure)
   - On all exit nodes completed: transition to `Completed`, emit `flow.completed` Pulse

6. Implement Flow query methods:
   - `status(run_id) -> Result<FlowStatus>`
   - `cancel(run_id, reason) -> Result<()>`
   - `list_active() -> Vec<(RunId, FlowStatus)>`

7. Write tests:
   - A 5-node DAG Graph executes correctly with parallel middle nodes
   - Flow transitions through Created -> Running -> Completed
   - Cancellation transitions to Cancelled state
   - Pulses are emitted for each lifecycle transition

## Verification
```bash
cargo check -p roko-orchestrator
cargo clippy -p roko-orchestrator --no-deps -- -D warnings
cargo test -p roko-orchestrator -- graph::executor
cargo test -p roko-orchestrator -- graph::flow
```

## What NOT to do
- Do NOT implement failure strategies beyond simple propagation -- that is M041
- Do NOT implement snapshot/resume -- that is M042
- Do NOT implement Hot Flow -- that is M043
- Do NOT replace the existing PlanRunner yet -- that is M045
- Do NOT block on a real CellRegistry -- use mock Cells that return dummy Signals for testing
