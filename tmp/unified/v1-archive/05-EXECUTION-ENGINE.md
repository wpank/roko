# 05 — Execution Engine

> The runtime that turns a Graph into a sequence of Block invocations, Signal productions, lifecycle Pulses, and a final output. Hot Graphs stay resident and re-fire per tick.

**Subsumes**: Workflow engine, state-graph runtime, run policy, checkpoint/resume, budget enforcement, Hot Graph execution, deterministic replay.

**Source**: Refactored from `tmp/workflow/05-execution-engine.md` with unified vocabulary.

---

## 1. Overview

The execution engine is the single runtime for all Graphs. It takes a resolved Graph, its input, and a GraphPolicy, and interprets the Graph's nodes and edges as a sequence of Block invocations. The engine handles:

- State-graph traversal with conditional edges
- Loops, fan-out / fan-in, sub-Graph composition
- **Hot Graph execution** — Flows that stay resident and re-fire per tick
- **Workflow/Activity split** — deterministic replay of Hot Graphs
- Human-in-loop pauses
- Failure strategies (retry, escalate, replan, compensate)
- Cancellation and budget enforcement (demurrage-aware)
- Resumability from snapshots
- Episode logging
- **Lifecycle Pulses on Bus** — all events flow through the ephemeral transport
- Lens integration (observability)
- Cascade router integration (model selection)

There is no second runtime. Every Graph — whether a plan pipeline, a verification chain, a dream cycle, or an agent's 9-step pipeline — runs through this engine.

---

## 2. Engine Inputs

The engine receives a **Flow** — a Graph at runtime (see [doc-04 §2](04-SPECIALIZATIONS.md)):

```rust
pub struct Flow {
    pub run_id: RunId,
    pub graph: ResolvedGraph,           // pinned Block versions
    pub input: Value,                   // validated against graph.schema.input
    pub macros: MacroBindings,          // resolved (if Graph is a Rack)
    pub slots: SlotBindings,            // resolved (if Graph is a Rack)
    pub trigger: Option<TriggerRef>,    // what started this Flow
    pub policy: GraphPolicy,            // overrides graph.policy if set
    pub resume_from: Option<RunSnapshot>, // for --resume
}
```

### GraphPolicy

Every Flow carries a policy governing execution behavior:

```rust
pub struct GraphPolicy {
    pub budget_usd: Option<f64>,
    pub deadline: Option<Duration>,
    pub on_failure: FailureStrategy,
    pub max_retries: u32,
    pub human_input_default: HumanInputDefault,
    pub parallelism_cap: u32,
    pub checkpoint_interval: Duration,
}
```

The policy can be set at three levels, with later overriding earlier:

1. **Graph definition** — default policy in TOML
2. **Trigger binding** — per-trigger overrides
3. **CLI invocation** — `--budget`, `--deadline`, `--parallelism` flags

---

## 3. Hot Graph Execution

A **Hot Graph** is a Flow that stays resident in memory between firings. Where a standard Flow progresses `Created -> Running -> Completed`, a Hot Graph cycles:

```
Loaded -> Resident -> [tick] -> Firing -> Quiescent -> [tick] -> Firing -> ...
                                                            ↘ Stopped
```

Hot Graphs bind to a clock — an Agent's adaptive clock (gamma/theta/delta) or a custom interval. On each tick, the engine fires the Graph, retains state if configured, and returns to Quiescent. The Agent's 9-step pipeline is always a Hot Graph.

### 3.1 Workflow / Activity Split

Within a Hot Graph (and applicable to standard Flows during replay), every node execution is classified as either **Workflow** or **Activity**:

| Category | Nature | Replay behavior |
|---|---|---|
| **Workflow** | Deterministic orchestration — pure Rust logic | Re-execute from code |
| **Activity** | Non-deterministic execution — side effects | Replay from recorded output |

**Workflow nodes**: Edge evaluation, Expr conditions, Branch decisions, FanOut iteration, merge strategy application, state transitions. These are pure functions of their inputs — given the same inputs, they produce the same outputs. During replay, the engine re-executes them.

**Activity nodes**: LLM calls, shell commands, HTTP requests, chain transactions, file I/O. These are non-deterministic — the same inputs can produce different outputs. During replay, the engine loads their recorded output from the event log instead of re-executing.

```rust
pub enum NodeOutput {
    Workflow(Value),                    // re-derivable from inputs
    Activity {                          // recorded, non-deterministic
        value: Value,
        recorded_at: DateTime<Utc>,
        cost: Cost,
    },
}
```

The engine tags each node's output at execution time. Block nodes that declare `Capability::Llm`, `Capability::Shell`, `Capability::Net`, `Capability::Chain`, or `Capability::FsWrite` are classified as Activities. All other nodes are Workflows.

### 3.2 Deterministic Replay

When resuming a Hot Graph or debugging a past execution:

1. Load the event log (Activity outputs + Workflow inputs)
2. Re-execute Workflow nodes from code — verify they produce the same outputs
3. Inject recorded Activity outputs at Activity nodes — do not re-execute
4. If a Workflow node produces a different output than recorded (state divergence), emit a `ReplayDivergence` Pulse and halt replay

This guarantees identical state transitions without incurring Activity costs (no LLM calls, no shell commands). Divergence detection catches bugs in Workflow logic.

### 3.3 Hot Graph State Retention

When `HotGraphConfig::retain_state = true`, node outputs persist across firings. The next tick sees prior outputs and can branch differently. This is how the Agent pipeline maintains continuity — CorticalState updated in Reflect (tick N) is visible to Observe (tick N+1).

State retention uses a ring buffer per node (configurable depth, default 3 firings). Old state is evicted; the current and recent firings are available to Expr evaluation via `prev.<node-id>` variables.

---

## 4. State Graph Semantics

### 4.1 Node Kinds

Node kinds are defined in [doc-03 §2](03-GRAPH.md). The engine's semantics per kind:

| Node | Engine Behavior |
|---|---|
| `Block` | Resolve Block by ref + version. Build `BlockInput` (project upstream output via edge mapping; layer Macro values). Acquire capabilities from Space. Invoke `Block::run`. Capture `BlockOutput`. Emit lifecycle Pulses on Bus (`block:{id}:started`, `block:{id}:completed`, `block:{id}:failed`). Charge budget. Log episode. Tag output as Workflow or Activity. |
| `SubGraph` | Recursively invoke engine on child Graph with its own RunId. Pulses bubble up to parent with `parent_run_id` breadcrumb. Output projects through edge mappings into parent's downstream nodes. Budget is carved from parent. |
| `Branch` | Evaluate `condition` Expr against current run state. Walk only edges whose condition evaluates true. Multiple matching edges fan out in parallel. Tagged as Workflow. |
| `FanOut` | Iterate `over` expression (must yield array). Spawn one child execution per element, capped by `max_parallelism`. Children execute downstream subgraph until next `FanIn`. Tagged as Workflow. |
| `FanIn` | Wait for all parallel branches launched by matching `FanOut`. Apply `MergeStrategy` (`Concat`, `FirstSuccess`, `AllOrFail`, `Vote`, `Custom`). Continue with merged state. Tagged as Workflow. |
| `Loop` | Repeat `body` subgraph. Evaluate `until` Expr each iteration. Bounded by `max_iterations`. Emit `LoopIteration` Pulses. |
| `HumanInput` | Persist state. Emit `HumanInputRequested` Pulse. Wait for response via dashboard/TUI/CLI. Validate against schema. Resume. Tagged as Activity. |
| `Wait` | Block until `WaitCondition` is satisfied (Signal arrives, Pulse received on Bus, time elapsed, sub-Graph completes). |
| `Slot` | Resolved at Flow start — replaced with the user-bound Block / sub-Graph / inline Graph. Engine never sees a raw Slot at runtime. |
| `Noop` | Pass-through synchronization point. Tagged as Workflow. |

### 4.2 Edge Evaluation

Edges carry data between nodes via `Mapping` declarations (see [doc-03 §3](03-GRAPH.md)):

- Edges with no condition are always traversed.
- Multiple matching edges from a `Branch` fan out in parallel.
- Zero matching edges from a non-exit node is a runtime error.
- Conditions are evaluated in source-node-completion order.

### 4.3 Expression Language

The Expr language is small, total, and deterministic. Defined in [doc-03 §4](03-GRAPH.md).

Variables in scope during engine evaluation:

| Variable | Meaning |
|---|---|
| `input` | Graph-level input |
| `output` | Last completed node's output |
| `<node-id>` | Any prior completed node's output by ID |
| `prev.<node-id>` | Prior firing's output (Hot Graphs only) |
| `macros` | Resolved Macro values (if in a Rack) |
| `slots` | Resolved Slot fillings (if in a Rack) |
| `run` | Run-level metadata: `id`, `started_at`, `elapsed`, `tick` (Hot Graphs) |

Expr evaluation has a **100ms timeout** per invocation. Long evaluation is a bug — the language is deliberately not Turing-complete.

---

## 5. Flow Lifecycle

A standard Flow progresses through a well-defined lifecycle:

```
Created -> Running -> Completed | Failed | Cancelled
               ↘ Paused (human input, budget) -> Resumed -> Running
```

A Hot Flow follows the resident lifecycle instead (see §3).

### 5.1 Created

Engine validates:
- All Block references resolve to installed Blocks at compatible versions
- All edge types check (upstream output schema matches downstream input schema)
- All required Slots are filled
- Budget and deadline are feasible (estimated cost < budget)
- Required capabilities are granted by the Space

If validation fails, the Flow enters `Failed` without executing any node.

### 5.2 Running

Engine traverses the state graph, executing nodes as they become ready. Concurrency is bounded by policy. At each node completion:

1. Output is validated against the Block's declared output schema
2. Output is tagged as Workflow or Activity
3. Output Signals marked for persistence are written to Store
4. Budget is charged (demurrage-aware — see §9)
5. Episode is logged
6. Snapshot is produced (per `checkpoint_interval`)
7. Lifecycle Pulses are published to Bus
8. Downstream edges are evaluated
9. Ready downstream nodes are scheduled

### 5.3 Paused

A Flow pauses when:
- A `HumanInput` node is reached
- Budget warning threshold is hit with `HumanInput` strategy
- A `Wait` node condition is not yet satisfied

Paused Flows persist their state and can survive daemon restarts.

### 5.4 Completed / Failed / Cancelled

Terminal states. The Flow's output (or error) is written to `output.json`. The manifest is updated. Trigger completion Pulses fire on Bus.

---

## 6. Lifecycle Pulses on Bus

All lifecycle events are Pulses on Bus ([doc-01](01-SIGNAL.md)) — not internal-only events. Every transition emits a Pulse that any subscriber can observe.

```rust
pub enum LifecyclePulse {
    // ── Block lifecycle ─────────────────────────────────────
    BlockStarted { block: BlockRef, run: RunId, input_hash: ContentHash },
    BlockCompleted { block: BlockRef, run: RunId, duration: Duration, cost: Cost },
    BlockFailed { block: BlockRef, run: RunId, error: BlockError },
    BlockRetried { block: BlockRef, run: RunId, attempt: u32, reason: String },
    BlockCancelled { block: BlockRef, run: RunId },

    // ── Graph lifecycle ─────────────────────────────────────
    GraphStarted { graph: GraphRef, run: RunId },
    GraphCompleted { graph: GraphRef, run: RunId, duration: Duration, cost: Cost },
    GraphFailed { graph: GraphRef, run: RunId, error: String },
    GraphCancelled { graph: GraphRef, run: RunId },
    GraphPaused { graph: GraphRef, run: RunId, reason: PauseReason },
    GraphResumed { graph: GraphRef, run: RunId },

    // ── Flow-specific ───────────────────────────────────────
    SnapshotCreated { run: RunId, seq: u64 },
    BudgetCharged { run: RunId, amount: f64, remaining: f64 },
    BudgetWarning { run: RunId, pct_used: f32 },
    BudgetExceeded { run: RunId, strategy: BudgetExceedStrategy },
    HumanInputRequested { run: RunId, node: NodeId, prompt: String },
    HumanInputReceived { run: RunId, node: NodeId },
    ReplanTriggered { run: RunId, failing_node: NodeId },
    LoopIteration { run: RunId, node: NodeId, iteration: u32 },

    // ── Hot Graph ───────────────────────────────────────────
    HotTick { graph: GraphRef, run: RunId, tick: u64 },
    HotQuiescent { graph: GraphRef, run: RunId },
    HotStateCarried { graph: GraphRef, run: RunId, nodes: Vec<NodeId> },
    HotStopped { graph: GraphRef, run: RunId },
    ReplayDivergence { graph: GraphRef, run: RunId, node: NodeId },

    // ── Signal lifecycle ────────────────────────────────────
    SignalCreated(Signal),
    SignalPersisted(SignalRef),
}
```

### Bus topic taxonomy for lifecycle

```
block:{id}:started          Block started
block:{id}:completed        Block completed
block:{id}:failed           Block failed
graph:{id}:events           Graph lifecycle events
flow:{run-id}:events        Flow-specific events (budget, snapshot, human-input)
hot:{graph}:tick             Hot Graph tick
hot:{graph}:quiescent        Hot Graph quiescent
hot:{graph}:stopped          Hot Graph stopped
```

### Graduation policy for lifecycle Pulses

Most lifecycle Pulses are ephemeral — they serve real-time consumers (TUI, dashboard, Lenses) and expire from the Bus ring buffer. Selected Pulses graduate to Signals for audit:

| Pulse | Graduate? | Rationale |
|---|---|---|
| `BlockCompleted` | Yes (as Episode) | Feeds learning loops |
| `GraphCompleted` / `GraphFailed` | Yes | Audit trail |
| `BudgetExceeded` | Yes | Accounting record |
| `HotStopped` | Yes | Agent lifecycle audit |
| `ReplayDivergence` | Yes | Bug detection record |
| `BlockStarted` | No | Transient, latest matters |
| `HotTick` / `HotQuiescent` | No | Too frequent |
| `SnapshotCreated` | No | Snapshot itself is the record |

---

## 7. Failure Strategies

Every Graph carries a `FailureStrategy` in its `GraphPolicy`. Individual nodes can override the Graph-level strategy.

```rust
pub enum FailureStrategy {
    /// Any Block failure fails the Flow immediately.
    Fail,

    /// Retry with configurable backoff.
    Retry { max: u32, backoff: Backoff },

    /// Retry, escalating model tier on each attempt.
    /// First attempt: configured model. Retry 1: next tier up.
    /// e.g., Haiku -> Sonnet -> Opus.
    RetryWithEscalation,

    /// Ask the Graph author to decompose the failing Block
    /// into smaller sub-Blocks.
    Decompose,

    /// Continue past failure, mark output as skipped.
    Skip { mark: bool },

    /// Run a cleanup Block, then continue.
    Compensate { compensator: BlockRef },

    /// Invoke a planner Block to revise the remaining graph.
    Replan,

    /// Pause for human decision.
    HumanResolve,
}

pub enum Backoff {
    Constant { ms: u64 },
    Exponential { base_ms: u64, factor: f64, max_ms: u64, jitter: bool },
}
```

### 7.1 Fail

The simplest strategy. Any `BlockError` from any node terminates the Flow. Appropriate for correctness-critical pipelines where partial results are worse than no results.

### 7.2 Retry

Configurable retries with backoff. The engine:

1. Catches the `BlockError`
2. Checks `attempt < max`
3. Sleeps per backoff strategy (with optional jitter to avoid thundering herd)
4. Re-invokes the Block with the same `BlockInput`
5. If all retries exhaust, falls through to `Fail`

### 7.3 RetryWithEscalation

The default for LLM-based Blocks. On each retry, the engine escalates to the next model tier via the cascade router:

```
Attempt 1: Haiku (fast, cheap)
Attempt 2: Sonnet (balanced)
Attempt 3: Opus (most capable)
Attempt 4: Fail
```

This naturally finds the cheapest model that can handle the task. Episodes from escalation feed back into the cascade router, improving future model selection.

### 7.4 Decompose

When a Block fails because the task is too complex for a single invocation (token limit, logical impasse), the engine signals the Graph author (human or planner agent) to decompose the Block into smaller steps. This is a design-time strategy, not a runtime recovery.

### 7.5 Skip

Continue past the failure. Downstream nodes receive a `skipped` marker in their input. Useful for optional enrichment steps (e.g., web research that might fail due to rate limits).

When `mark = true`, the Flow's output includes a list of skipped nodes for audit.

### 7.6 Compensate

Run a designated cleanup Block before continuing. Use cases:

- Roll back a partial database migration
- Delete temporary files created by the failing Block
- Notify downstream systems that the operation was abandoned

The compensator Block receives the failing Block's input and error as its own input.

### 7.7 Replan

The most powerful strategy. When a Block fails in a way that isn't retriable (logical impasse, schema violation, contradictory output), the engine:

1. Captures the current state graph, the failure, and all completed node outputs
2. Invokes a designated `planner` Block with this context
3. The planner produces a revised sub-graph for the remaining work
4. The engine hot-swaps the remaining nodes with the revised sub-graph
5. Execution continues from the revised entry point

This is how the system self-heals long-running pipelines. The planner can:
- Skip the failing step and reroute around it
- Break the failing step into smaller steps
- Try a completely different approach
- Insert additional validation steps

**Replan limits**: Max 3 replans per Flow (configurable). Each replan is logged as an episode. Cascading replans (replan-within-replan) are not permitted — a failed replan falls through to `HumanResolve`.

### 7.8 HumanResolve

Pause the Flow and present the failure to a human. The human can:
- Provide a fix and resume
- Skip the failing node
- Cancel the Flow
- Edit the Graph and resume with modifications

This is the fallback of last resort. All other strategies eventually fall through to `HumanResolve` if they cannot recover.

---

## 8. Resumability

Every Flow produces a snapshot at every node completion (throttled by `GraphPolicy::checkpoint_interval` for very fast Blocks). Snapshots persist to `.roko/runs/<run-id>/snapshot.json`.

```rust
pub struct RunSnapshot {
    pub run_id: RunId,
    pub graph: ResolvedGraphRef,
    pub input: Value,
    pub macros: MacroBindings,
    pub slots: SlotBindings,
    pub completed: Vec<NodeCompletion>,     // per-node output + metrics + Workflow/Activity tag
    pub in_flight: Vec<NodeId>,             // running at snapshot time
    pub queued: Vec<NodeId>,                // ready-to-run
    pub blocked: Vec<BlockedNode>,          // awaiting human input / wait conditions
    pub signals_produced: Vec<SignalRef>,   // Signals produced so far
    pub bus_seq: u64,                       // Bus high-water mark for Pulse replay
    pub policy: GraphPolicy,
    pub trigger: Option<TriggerRef>,
    pub started_at: DateTime<Utc>,
    pub last_checkpoint_at: DateTime<Utc>,
    pub budget_spent: f64,                  // USD consumed so far
    pub hot_state: Option<HotGraphState>,   // retained state for Hot Graphs
}
```

### Resume semantics

`roko plan run <dir> --resume <run-id>` (or dashboard "Resume" button) reloads the snapshot and continues from the queued nodes.

**In-flight nodes** at snapshot time are restarted from scratch. Blocks must be idempotent or carry their own internal checkpointing.

**Retry-from**: A failed Flow can be retried from the failing node onward (`--retry-from <node-id>`) without re-running upstream nodes. The engine loads the snapshot, finds the specified node, marks it as queued, and resumes.

**Hot Graph resume**: A Hot Graph that resumes loads its retained state and continues from the next tick. Activity outputs from the event log are available for replay; Workflow nodes re-execute from code. Bus Pulses since `bus_seq` are replayed via `Bus::replay_since` for subscribers that missed them.

### Snapshot retention

Historical snapshots are kept at `.roko/runs/<run-id>/snapshot.<seq>.json`. Retention is per-Space policy (default: keep last 1000 runs, GC older). The latest snapshot is always at `snapshot.json`.

---

## 9. Budget Enforcement

```rust
pub struct BudgetTracker {
    pub usd_limit: Option<f64>,
    pub usd_spent: f64,
    pub warn_at_pct: f32,               // emit BudgetWarn at this fraction (default 0.8)
    pub strategy: BudgetExceedStrategy,
    pub demurrage_cost: f64,            // cumulative demurrage charged to Signals in this Flow
}

pub enum BudgetExceedStrategy {
    /// Fail the Flow immediately.
    Cancel,
    /// Skip remaining nodes tagged `optional = true`.
    SkipOptional,
    /// Re-route remaining LLM calls to cheaper model tier.
    Downgrade,
    /// Pause and ask human whether to continue.
    HumanInput,
}
```

### Charging

Blocks that incur cost call `ctx.budget.charge(cost)` after each chargeable operation. The tracker deducts from the limit. At thresholds:

| Threshold | Action |
|---|---|
| `warn_at_pct` (default 80%) | Emit `BudgetWarning` Pulse on Bus (consumed by BudgetLens) |
| 100% | Execute `BudgetExceedStrategy` |

### Demurrage-aware cost tracking

The budget tracker accounts for demurrage costs alongside direct execution costs. When a Flow produces Signals that enter Store, their ongoing demurrage is attributed to the Flow's budget until ownership transfers (e.g., the Signal is cited by a different Agent). This prevents Flows from externalizing storage costs — a Flow that produces many low-value Signals pays for their decay.

The `demurrage_cost` field in `BudgetTracker` records cumulative demurrage charges. Lenses observe this via `BudgetCharged` Pulses, enabling dashboards to show both direct and storage costs.

### Budget hierarchy

Budgets are hierarchical:
- **Flow budget** — the top-level limit
- **Sub-Graph budget** — carved from parent Flow budget at sub-Graph entry
- **Block budget** — implicit (the Block's `estimate_cost` informs scheduling)

A sub-Graph cannot exceed its allocation. If a sub-Graph needs more budget than allocated, it follows its own `BudgetExceedStrategy`.

### Downgrade strategy

When `Downgrade` is active, the engine intercepts model selection at `ctx.model_router` and forces the cheapest viable tier. Episodes from downgraded runs are tagged so the cascade router doesn't treat cheap-model failures as representative.

---

## 10. Human-in-Loop

`HumanInput` nodes are first-class in the Graph (see [doc-03 §2](03-GRAPH.md)). When the engine reaches one:

1. Persist current state (snapshot)
2. Emit `HumanInputRequested` Pulse on Bus with prompt, schema, and timeout
3. The dashboard, TUI, or CLI surfaces the prompt to the user
4. The user provides input via:
   - Dashboard form
   - TUI prompt
   - CLI: `roko run respond <run-id> --node <node-id> --input <json>`
   - HTTP: `POST /runs/{run-id}/nodes/{node-id}/respond`
5. Engine receives `HumanInputReceived`, validates against schema, resumes

### Timeout behavior

Configurable per `HumanInput` node:

| Timeout action | Behavior |
|---|---|
| `Cancel` | Timeout aborts the Flow |
| `Default { value }` | Timeout uses a default value and continues |
| `Skip` | Timeout skips the node |
| `Escalate { channel }` | Timeout pings on a different channel (Slack, email) |

### Persistence across restarts

Human-input requests are persisted in the snapshot. A daemon that restarts after a crash sees pending human-input requests and continues serving them. The user's dashboard shows the pending request regardless of engine restarts.

---

## 11. Concurrency and Parallelism

The engine is fully async (`tokio`). Nodes execute concurrently when:

- Multiple edges from a `Branch` evaluate true
- A `FanOut` spawns child branches
- The state graph has natural parallelism (multiple independent subtrees)

### Parallelism bounds

Concurrency is bounded at four levels:

| Level | Config | Default |
|---|---|---|
| **Per-Flow** | `GraphPolicy::parallelism_cap` | 8 |
| **Per-FanOut** | `FanOut::max_parallelism` | 4 |
| **Per-Block** | Block's declared concurrency cap | 1 |
| **Per-Space** | `Space::max_concurrent_flows` | 16 |

The engine uses a central scheduler (tokio task pool with semaphores) so total in-flight Blocks across all Flows is bounded. This prevents resource exhaustion when multiple Flows run simultaneously.

### Scheduling order

When multiple nodes are ready, the engine schedules them by:

1. **Dependency order** — nodes closer to the exit are prioritized (reduces in-flight state)
2. **Cost** — cheaper nodes first (maximizes throughput per dollar)
3. **Priority** — user-assigned priority on nodes (override natural ordering)

---

## 12. Episode Logging

Every Block execution produces an **Episode** — a Signal of kind `Episode` written to `.roko/episodes.jsonl`:

```rust
pub struct Episode {
    pub episode_id: EpisodeId,
    pub run_id: RunId,
    pub graph: GraphRef,
    pub node_id: NodeId,
    pub block: BlockRef,
    pub node_class: NodeClass,           // Workflow | Activity
    pub input: Value,                    // truncated if large
    pub output: Value,
    pub model: Option<ModelRef>,
    pub temperature: Option<f32>,
    pub tokens_in: Option<u32>,
    pub tokens_out: Option<u32>,
    pub usd_cost: f64,
    pub wall_ms: u64,
    pub retries: u32,
    pub findings: Vec<Signal>,           // Finding-kind Signals
    pub success: bool,
    pub timestamp: DateTime<Utc>,
    pub hdc_fingerprint: HdcVector,      // for cross-domain resonance
}

pub enum NodeClass {
    Workflow,
    Activity,
}
```

Episodes are Signals — they carry all Signal properties (content hash, lineage, demurrage, HDC fingerprint). They feed the `roko-learn` infrastructure:

- **Cascade router updates** — success/failure per (role, Block, Graph) tuple
- **Prompt experiments** — A/B outcomes for prompt variants
- **Efficiency tracking** — tokens-per-task ratios
- **Gate threshold adaptation** — EMA updates from pass/fail rates

### Per-Graph learning

The cascade router selects models *per Block per Graph*, so the synthesizer in `doc-ingest` can have a different cost/quality trade-off than the synthesizer in `prd-draft`. More Graphs running -> more episodes -> better model selection across all related Graphs.

---

## 13. Cascade Router Integration

`ctx.model_router` in `BlockContext` resolves model selection at Block invocation time:

```rust
let model = ctx.model_router
    .select(
        role: "strategist",
        block: self.name(),
        graph: ctx.graph.name(),
        difficulty_hint: input.difficulty(),
        budget_remaining: ctx.budget.remaining(),
    )
    .await?;
```

The router consults, in order:

1. **Space-level model defaults** from `roko.toml`
2. **Graph-level overrides** from Macros
3. **Per-(role, Block, Graph) success-rate history** from episodes
4. **Cost/quality Pareto** from the EFE bandit state

When `force_backend` is set on a Block, the router bypasses learning and uses the specified model. (UX34: the router does not yet learn from manual overrides — a known gap.)

---

## 14. Lens Integration

The engine publishes lifecycle Pulses on Bus at every transition (see §6). Lenses attached to the Graph, its Blocks, or its parent Space subscribe to these Pulses without modifying execution.

### How Lenses consume Pulses

Lenses are Blocks implementing the Observe protocol (see [doc-04 §5](04-SPECIALIZATIONS.md)). They subscribe to Bus topics at different scopes:

| Scope | Bus topic pattern | What it observes |
|---|---|---|
| Block | `block:{id}:*` | Pulses from a single Block across all Flows |
| Graph | `graph:{id}:events` | Pulses from all Blocks and sub-Graphs within a Graph |
| Space | `graph:*:events` (within Space) | Pulses from all Graphs within a Space |

Multiple Lenses can observe the same target (stacking). A Lens can observe another Lens's output (chaining). The engine dispatches Pulses to Lenses asynchronously via Bus — Lens execution never blocks Graph execution.

### Built-in Lenses relevant to execution

| Lens | Subscribes to | Emits |
|---|---|---|
| CostLens | `BudgetCharged` | `CostReport` Signals per interval |
| LatencyLens | `BlockCompleted` | p50/p95/p99 Signals |
| ErrorLens | `BlockFailed`, `GraphFailed` | Classified error report Signals |
| BudgetLens | `BudgetWarning`, `BudgetExceeded` | Alert Signals |
| QualityLens | `SignalVerified` (from Verify Blocks) | Pass-rate Signals |

---

## 15. Loop Failure Recovery

When a Loop node's body fails, the engine applies failure strategies with Loop-specific semantics:

| Strategy | Loop behavior |
|---|---|
| `Fail` | Loop terminates. Flow fails. |
| `Retry` | Retry the current iteration's body. If all retries exhaust, Loop terminates. |
| `RetryWithEscalation` | Retry current iteration with escalated model. Common for iterative refinement Loops. |
| `Skip` | Skip current iteration, advance to next. Loop continues. |
| `Replan` | Invoke planner to revise the Loop body for remaining iterations. |
| `HumanResolve` | Pause the Loop. Human can fix and resume, skip iteration, or break out of Loop. |

### Convergence detection

Loops with a convergence condition (`until` Expr) can also stall — iterating without converging. The engine detects stalls via:

- **Max iterations** — hard cap, always enforced
- **Output similarity** — if the last 3 iterations produced outputs with >0.95 HDC similarity, emit a `LoopStalled` Pulse
- **Cost growth** — if cumulative Loop cost exceeds 3x the initial iteration cost, emit a `LoopExpensive` Pulse

Stall Pulses are consumed by Lenses and can trigger policy reactions (e.g., a React Block that cancels stalled Loops).

---

## 16. Cost and Time Estimation

Before execution, the engine produces a cost-time estimate by walking the state graph and summing `Block::estimate_cost`:

```rust
pub struct FlowEstimate {
    pub usd_min: f64,           // optimistic (all branches cheapest path)
    pub usd_expected: f64,      // expected (weighted by historical branch probabilities)
    pub usd_max: f64,           // pessimistic (all branches most expensive path)
    pub wall_min: Duration,
    pub wall_expected: Duration,
    pub wall_max: Duration,
    pub confidence: f64,        // 0.0..=1.0, based on how many Blocks have historical data
}
```

The estimate informs:
- **Pre-run UI confirmation** — dashboard shows expected cost before starting
- **Budget validation** — warn if estimate > budget limit
- **ETA display** — TUI / dashboard progress bars during execution

Deviations between estimate and actual feed back into Block estimators as a learning Loop.

---

## 17. Run Storage Layout

```
.roko/runs/<run-id>/
├── snapshot.json              # latest checkpoint
├── snapshot.<seq>.json        # historical checkpoints (retention configurable)
├── input.json                 # Graph input
├── output.json                # populated on completion
├── events.jsonl               # graduated Pulses (audit trail)
├── activities.jsonl           # Activity node recorded outputs (for replay)
├── artifacts/                 # artifact Signals produced
│   └── sig_<id>
├── episodes/                  # episode Signals for this run
│   └── ep_<id>.json
└── manifest.json              # status, timing, cost, error if any
```

### manifest.json

```json
{
  "run_id": "run_01HXYZ...",
  "graph": "doc-ingest@1.0.0",
  "status": "completed",
  "hot": false,
  "started_at": "2026-04-25T14:30:00Z",
  "completed_at": "2026-04-25T14:32:15Z",
  "wall_ms": 135000,
  "usd_cost": 0.47,
  "usd_demurrage": 0.003,
  "nodes_total": 6,
  "nodes_completed": 6,
  "nodes_skipped": 0,
  "nodes_failed": 0,
  "workflow_nodes": 3,
  "activity_nodes": 3,
  "episodes": 6,
  "signals_produced": 12,
  "trigger": "manual",
  "error": null
}
```

Retention is per-Space policy (default: keep last 1000 runs, GC older).

---

## 18. Sub-Graph Execution

When the engine encounters a `SubGraph` node:

1. **Resolve** the child Graph by `name@version`
2. **Carve budget** from the parent Flow's remaining budget
3. **Create child RunId** with parent breadcrumb
4. **Map input** from parent node's incoming edges via `Mapping` declarations
5. **Execute** the child Graph recursively through the same engine
6. **Map output** from child Graph's output via outgoing edge `Mapping` declarations
7. **Merge Pulses** — child Pulses are prefixed with child RunId but visible on the parent's Bus topics

Sub-Graphs get their own snapshot, their own episode log, and their own budget tracker. A sub-Graph failure follows the parent node's failure strategy.

### Depth limit

Sub-Graph nesting is capped at 8 levels (configurable). Deeper nesting is a design smell — the Graph should be flattened.

---

## 19. Acceptance Criteria

| # | Criterion | Verification |
|---|---|---|
| 1 | Linear Graph runs end-to-end with all lifecycle Pulses emitted on Bus | Integration test on a 3-Block sequential Graph |
| 2 | Conditional edge: Branch condition evaluation routes correctly | Test fixture with branching condition |
| 3 | FanOut/FanIn: parallel branches with merged output via `Concat` strategy | Test on 3-way parallel fan-out |
| 4 | Loop terminates at `until` predicate satisfaction | Test: predicate met after 2 iterations |
| 5 | Loop respects `max_iterations` | Test: predicate never met, cap kicks in |
| 6 | Sub-Graph: parent + child Pulses visible on Bus | Nested Graph test, verify `parent_run_id` |
| 7 | HumanInput: Flow pauses, resumes after `respond` invocation | Async integration test |
| 8 | Resume: kill engine mid-run; resume from snapshot; result identical to non-killed run | Property test on idempotent Graph |
| 9 | Retry-from: resume from a specific failing node | Test: fail node 3, retry-from node 3 |
| 10 | Cancellation: external cancel propagates to in-flight Blocks within 5s | Slow Block + cancel; Block reports cancelled |
| 11 | Budget `Cancel` strategy aborts on overage | Test: set low budget, run expensive Block |
| 12 | Budget `Downgrade` strategy re-routes to cheaper model | Test: hit budget, verify model tier change |
| 13 | Budget `SkipOptional` strategy skips optional nodes | Test: optional node skipped on budget hit |
| 14 | Episodes written for every Block run with correct cost/token/model/NodeClass fields | Verify episode count matches node count |
| 15 | Cascade router queries succeed and reflect prior episodes | Bandit state snapshot test |
| 16 | Lifecycle Pulses emitted on Bus for all transitions | Bus subscriber test, verify completeness |
| 17 | Replan: failing Block triggers planner, revised sub-graph executes | Integration test with mock planner |
| 18 | Loop stall detection: 3 similar iterations emit `LoopStalled` Pulse | HDC similarity test |
| 19 | Sub-Graph budget carving: child cannot exceed allocation | Test: child overruns, verify failure |
| 20 | Cost estimate produced before execution; deviation logged after | Compare estimate vs. actual |
| 21 | Hot Graph stays resident across multiple ticks with state retention | Integration test: 3 ticks, verify state carries |
| 22 | Workflow/Activity classification: Block with LLM cap tagged Activity | Unit test on capability detection |
| 23 | Deterministic replay: Workflow nodes re-execute, Activity nodes replay from log | Replay test with mock LLM Block |
| 24 | Replay divergence detection: altered Workflow logic emits `ReplayDivergence` | Tamper test |
| 25 | Demurrage cost tracked in BudgetTracker and visible in manifest | Produce Signals, verify demurrage_cost > 0 |
| 26 | Graduation policy: BlockCompleted graduates, HotTick does not | Bus + Store integration test |

---

## 20. Open Questions

- **Speculative execution**: Should the engine start downstream nodes before upstream completes, cancelling if upstream output invalidates? Powerful but complex; defer to v2.
- **Shadow runs**: Run a candidate Graph alongside production, compare outputs, never persist shadow Signals? Useful for A/B testing Graphs; defer.
- **Per-Block resource limits**: CPU, memory, file handles per Block? Likely via cgroups for native, fuel for WASM, OS limits for scripts. Specify in v1.1.
- **Multi-machine engine**: Fan-out across machines for very large Graphs? Out of scope for v1; Space federation would enable this.
- **Activity versioning**: When an Activity's implementation changes (e.g., new LLM version), should replay detect the mismatch and re-execute instead of replaying? Defer to v1.1.
