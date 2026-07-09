# 05 — Execution Engine

> The runtime that turns a Graph into a sequence of Block invocations, Signal productions, lifecycle events, and a final output.

**Subsumes**: Workflow engine, state-graph runtime, run policy, checkpoint/resume, budget enforcement.

**Source**: Refactored from `tmp/workflow/05-execution-engine.md` with unified vocabulary.

---

## 1. Overview

The execution engine is the single runtime for all Graphs. It takes a resolved Graph, its input, and a GraphPolicy, and interprets the Graph's nodes and edges as a sequence of Block invocations. The engine handles:

- State-graph traversal with conditional edges
- Loops, fan-out / fan-in, sub-Graph composition
- Human-in-loop pauses
- Failure strategies (retry, escalate, replan, compensate)
- Cancellation and budget enforcement
- Resumability from snapshots
- Episode logging
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

## 3. State Graph Semantics

### 3.1 Node Kinds

Node kinds are defined in [doc-03 §2](03-GRAPH.md). The engine's semantics per kind:

| Node | Engine Behavior |
|---|---|
| `Block` | Resolve Block by ref + version. Build `BlockInput` (project upstream output via edge mapping; layer Macro values). Acquire capabilities from Space. Invoke `Block::run`. Capture `BlockOutput`. Emit `BlockStarted` / `BlockCompleted` / `BlockFailed` events. Charge budget. Log episode. |
| `SubGraph` | Recursively invoke engine on child Graph with its own RunId. Events bubble up to parent with `parent_run_id` breadcrumb. Output projects through edge mappings into parent's downstream nodes. Budget is carved from parent. |
| `Branch` | Evaluate `condition` Expr against current run state. Walk only edges whose condition evaluates true. Multiple matching edges fan out in parallel. |
| `FanOut` | Iterate `over` expression (must yield array). Spawn one child execution per element, capped by `max_parallelism`. Children execute downstream subgraph until next `FanIn`. |
| `FanIn` | Wait for all parallel branches launched by matching `FanOut`. Apply `MergeStrategy` (`Concat`, `FirstSuccess`, `AllOrFail`, `Vote`, `Custom`). Continue with merged state. |
| `Loop` | Repeat `body` subgraph. Evaluate `until` Expr each iteration. Bounded by `max_iterations`. Emit `LoopIteration` events. |
| `HumanInput` | Persist state. Emit `HumanInputRequested` event. Wait for response via dashboard/TUI/CLI. Validate against schema. Resume. |
| `Wait` | Block until `WaitCondition` is satisfied (Signal arrives, event received, time elapsed, sub-Graph completes). |
| `Slot` | Resolved at Flow start — replaced with the user-bound Block / sub-Graph / inline Graph. Engine never sees a raw Slot at runtime. |
| `Noop` | Pass-through synchronization point. |

### 3.2 Edge Evaluation

Edges carry data between nodes via `Mapping` declarations (see [doc-03 §3](03-GRAPH.md)):

- Edges with no condition are always traversed.
- Multiple matching edges from a `Branch` fan out in parallel.
- Zero matching edges from a non-exit node is a runtime error.
- Conditions are evaluated in source-node-completion order.

### 3.3 Expression Language

The Expr language is small, total, and deterministic. Defined in [doc-03 §4](03-GRAPH.md).

Variables in scope during engine evaluation:

| Variable | Meaning |
|---|---|
| `input` | Graph-level input |
| `output` | Last completed node's output |
| `<node-id>` | Any prior completed node's output by ID |
| `macros` | Resolved Macro values (if in a Rack) |
| `slots` | Resolved Slot fillings (if in a Rack) |
| `run` | Run-level metadata: `id`, `started_at`, `elapsed` |

Expr evaluation has a **100ms timeout** per invocation. Long evaluation is a bug — the language is deliberately not Turing-complete.

---

## 4. Flow Lifecycle

A Flow progresses through a well-defined lifecycle:

```
Created → Running → Completed | Failed | Cancelled
               ↘ Paused (human input, budget) → Resumed → Running
```

### 4.1 Created

Engine validates:
- All Block references resolve to installed Blocks at compatible versions
- All edge types check (upstream output schema matches downstream input schema)
- All required Slots are filled
- Budget and deadline are feasible (estimated cost < budget)
- Required capabilities are granted by the Space

If validation fails, the Flow enters `Failed` without executing any node.

### 4.2 Running

Engine traverses the state graph, executing nodes as they become ready. Concurrency is bounded by policy. At each node completion:

1. Output is validated against the Block's declared output schema
2. Output Signals marked for persistence are written to Store
3. Budget is charged
4. Episode is logged
5. Snapshot is produced (per `checkpoint_interval`)
6. Downstream edges are evaluated
7. Ready downstream nodes are scheduled

### 4.3 Paused

A Flow pauses when:
- A `HumanInput` node is reached
- Budget warning threshold is hit with `HumanInput` strategy
- A `Wait` node condition is not yet satisfied

Paused Flows persist their state and can survive daemon restarts.

### 4.4 Completed / Failed / Cancelled

Terminal states. The Flow's output (or error) is written to `output.json`. The manifest is updated. Trigger completion events fire.

---

## 5. Failure Strategies

Every Graph carries a `FailureStrategy` in its `GraphPolicy`. Individual nodes can override the Graph-level strategy.

```rust
pub enum FailureStrategy {
    /// Any Block failure fails the Flow immediately.
    Fail,

    /// Retry with configurable backoff.
    Retry { max: u32, backoff: Backoff },

    /// Retry, escalating model tier on each attempt.
    /// First attempt: configured model. Retry 1: next tier up.
    /// e.g., Haiku → Sonnet → Opus.
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

### 5.1 Fail

The simplest strategy. Any `BlockError` from any node terminates the Flow. Appropriate for correctness-critical pipelines where partial results are worse than no results.

### 5.2 Retry

Configurable retries with backoff. The engine:

1. Catches the `BlockError`
2. Checks `attempt < max`
3. Sleeps per backoff strategy (with optional jitter to avoid thundering herd)
4. Re-invokes the Block with the same `BlockInput`
5. If all retries exhaust, falls through to `Fail`

### 5.3 RetryWithEscalation

The default for LLM-based Blocks. On each retry, the engine escalates to the next model tier via the cascade router:

```
Attempt 1: Haiku (fast, cheap)
Attempt 2: Sonnet (balanced)
Attempt 3: Opus (most capable)
Attempt 4: Fail
```

This naturally finds the cheapest model that can handle the task. Episodes from escalation feed back into the cascade router, improving future model selection.

### 5.4 Decompose

When a Block fails because the task is too complex for a single invocation (token limit, logical impasse), the engine signals the Graph author (human or planner agent) to decompose the Block into smaller steps. This is a design-time strategy, not a runtime recovery.

### 5.5 Skip

Continue past the failure. Downstream nodes receive a `skipped` marker in their input. Useful for optional enrichment steps (e.g., web research that might fail due to rate limits).

When `mark = true`, the Flow's output includes a list of skipped nodes for audit.

### 5.6 Compensate

Run a designated cleanup Block before continuing. Use cases:

- Roll back a partial database migration
- Delete temporary files created by the failing Block
- Notify downstream systems that the operation was abandoned

The compensator Block receives the failing Block's input and error as its own input.

### 5.7 Replan

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

### 5.8 HumanResolve

Pause the Flow and present the failure to a human. The human can:
- Provide a fix and resume
- Skip the failing node
- Cancel the Flow
- Edit the Graph and resume with modifications

This is the fallback of last resort. All other strategies eventually fall through to `HumanResolve` if they cannot recover.

---

## 6. Resumability

Every Flow produces a snapshot at every node completion (throttled by `GraphPolicy::checkpoint_interval` for very fast Blocks). Snapshots persist to `.roko/runs/<run-id>/snapshot.json`.

```rust
pub struct RunSnapshot {
    pub run_id: RunId,
    pub graph: ResolvedGraphRef,
    pub input: Value,
    pub macros: MacroBindings,
    pub slots: SlotBindings,
    pub completed: Vec<NodeCompletion>,     // per-node output + metrics
    pub in_flight: Vec<NodeId>,             // running at snapshot time
    pub queued: Vec<NodeId>,                // ready-to-run
    pub blocked: Vec<BlockedNode>,          // awaiting human input / wait conditions
    pub signals_produced: Vec<SignalRef>,   // Signals produced so far
    pub events_offset: u64,                 // cursor into events.jsonl
    pub policy: GraphPolicy,
    pub trigger: Option<TriggerRef>,
    pub started_at: DateTime<Utc>,
    pub last_checkpoint_at: DateTime<Utc>,
    pub budget_spent: f64,                  // USD consumed so far
}
```

### Resume semantics

`roko plan run <dir> --resume <run-id>` (or dashboard "Resume" button) reloads the snapshot and continues from the queued nodes.

**In-flight nodes** at snapshot time are restarted from scratch. Blocks must be idempotent or carry their own internal checkpointing.

**Retry-from**: A failed Flow can be retried from the failing node onward (`--retry-from <node-id>`) without re-running upstream nodes. The engine loads the snapshot, finds the specified node, marks it as queued, and resumes.

### Snapshot retention

Historical snapshots are kept at `.roko/runs/<run-id>/snapshot.<seq>.json`. Retention is per-Space policy (default: keep last 1000 runs, GC older). The latest snapshot is always at `snapshot.json`.

---

## 7. Cancellation

A Flow may be cancelled at any time from multiple sources:

| Source | How |
|---|---|
| External CLI | `roko plan cancel <run-id>` |
| Dashboard | Cancel button |
| Budget exhaustion | `BudgetExceedStrategy::Cancel` |
| Deadline | `GraphPolicy::deadline` exceeded |
| Trigger replacement | `CancelRunning` concurrency policy on a re-firing trigger |
| Space lock | Space enters maintenance mode |

### Cancellation propagation

The engine shares a `CancellationToken` (from `tokio-util`) into every Block's `BlockContext`. When cancellation is requested:

1. The token is cancelled
2. In-flight Blocks detect cancellation via `ctx.cancel.is_cancelled()` or `tokio::select!`
3. Blocks return `BlockError::Cancelled`
4. The engine runs any registered compensators for completed nodes (if `Compensate` strategy is configured)
5. Final state is persisted as `Cancelled`
6. The Flow exits

**Propagation deadline**: If in-flight Blocks don't respond within 5 seconds of cancellation, the engine force-drops their tasks. Blocks should check cancellation at least every second during long-running operations.

---

## 8. Budget Enforcement

```rust
pub struct BudgetTracker {
    pub usd_limit: Option<f64>,
    pub usd_spent: f64,
    pub warn_at_pct: f32,               // emit BudgetWarn at this fraction (default 0.8)
    pub strategy: BudgetExceedStrategy,
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
| `warn_at_pct` (default 80%) | Emit `BudgetWarn` event (consumed by BudgetLens) |
| 100% | Execute `BudgetExceedStrategy` |

### Budget hierarchy

Budgets are hierarchical:
- **Flow budget** — the top-level limit
- **Sub-Graph budget** — carved from parent Flow budget at sub-Graph entry
- **Block budget** — implicit (the Block's `estimate_cost` informs scheduling)

A sub-Graph cannot exceed its allocation. If a sub-Graph needs more budget than allocated, it follows its own `BudgetExceedStrategy`.

### Downgrade strategy

When `Downgrade` is active, the engine intercepts model selection at `ctx.model_router` and forces the cheapest viable tier. Episodes from downgraded runs are tagged so the cascade router doesn't treat cheap-model failures as representative.

---

## 9. Human-in-Loop

`HumanInput` nodes are first-class in the Graph (see [doc-03 §2](03-GRAPH.md)). When the engine reaches one:

1. Persist current state (snapshot)
2. Emit `HumanInputRequested` event with prompt, schema, and timeout
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

## 10. Concurrency and Parallelism

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

## 11. Episode Logging

Every Block execution produces an **Episode** — a Signal of kind `Episode` written to `.roko/episodes.jsonl`:

```rust
pub struct Episode {
    pub episode_id: EpisodeId,
    pub run_id: RunId,
    pub graph: GraphRef,
    pub node_id: NodeId,
    pub block: BlockRef,
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
```

Episodes are Signals — they carry all Signal properties (content hash, lineage, decay, HDC fingerprint). They feed the `roko-learn` infrastructure:

- **Cascade router updates** — success/failure per (role, Block, Graph) tuple
- **Prompt experiments** — A/B outcomes for prompt variants
- **Efficiency tracking** — tokens-per-task ratios
- **Gate threshold adaptation** — EMA updates from pass/fail rates

### Per-Graph learning

The cascade router selects models *per Block per Graph*, so the synthesizer in `doc-ingest` can have a different cost/quality trade-off than the synthesizer in `prd-draft`. More Graphs running → more episodes → better model selection across all related Graphs.

---

## 12. Cascade Router Integration

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
4. **Cost/quality Pareto** from the LinUCB bandit state

When `force_backend` is set on a Block, the router bypasses learning and uses the specified model. (UX34: the router does not yet learn from manual overrides — a known gap.)

---

## 13. Lens Integration

The engine emits **ObservableEvents** at every lifecycle transition. Lenses attached to the Graph, its Blocks, or its parent Space observe these events without modifying execution.

### Events emitted by the engine

```rust
pub enum ObservableEvent {
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

    // ── Signal lifecycle ────────────────────────────────────
    SignalCreated(Signal),
    SignalPersisted(SignalRef),
}
```

### How Lenses consume events

Lenses are Blocks implementing the Observe protocol (see [doc-04 §5](04-SPECIALIZATIONS.md)). They attach at different scopes:

| Scope | What it observes |
|---|---|
| Block | Events from a single Block across all Flows |
| Graph | Events from all Blocks and sub-Graphs within a Graph |
| Space | Events from all Graphs within a Space |

Multiple Lenses can observe the same target (stacking). A Lens can observe another Lens's output (chaining). The engine dispatches events to Lenses asynchronously — Lens execution never blocks Graph execution.

### Built-in Lenses relevant to execution

| Lens | Observes | Emits |
|---|---|---|
| CostLens | `BudgetCharged` | `CostReport` Signals per interval |
| LatencyLens | `BlockCompleted` | p50/p95/p99 Signals |
| ErrorLens | `BlockFailed`, `GraphFailed` | Classified error report Signals |
| BudgetLens | `BudgetWarning`, `BudgetExceeded` | Alert Signals |
| QualityLens | `SignalVerified` (from Verify Blocks) | Pass-rate Signals |

---

## 14. Loop Failure Recovery

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
- **Output similarity** — if the last 3 iterations produced outputs with >0.95 HDC similarity, emit a `LoopStalled` event
- **Cost growth** — if cumulative Loop cost exceeds 3x the initial iteration cost, emit a `LoopExpensive` event

Stall events are consumed by Lenses and can trigger policy reactions (e.g., a React Block that cancels stalled Loops).

---

## 15. Cost and Time Estimation

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

## 16. Run Storage Layout

```
.roko/runs/<run-id>/
├── snapshot.json              # latest checkpoint
├── snapshot.<seq>.json        # historical checkpoints (retention configurable)
├── input.json                 # Graph input
├── output.json                # populated on completion
├── events.jsonl               # full event stream (ObservableEvents)
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
  "started_at": "2026-04-25T14:30:00Z",
  "completed_at": "2026-04-25T14:32:15Z",
  "wall_ms": 135000,
  "usd_cost": 0.47,
  "nodes_total": 6,
  "nodes_completed": 6,
  "nodes_skipped": 0,
  "nodes_failed": 0,
  "episodes": 6,
  "signals_produced": 12,
  "trigger": "manual",
  "error": null
}
```

Retention is per-Space policy (default: keep last 1000 runs, GC older).

---

## 17. Sub-Graph Execution

When the engine encounters a `SubGraph` node:

1. **Resolve** the child Graph by `name@version`
2. **Carve budget** from the parent Flow's remaining budget
3. **Create child RunId** with parent breadcrumb
4. **Map input** from parent node's incoming edges via `Mapping` declarations
5. **Execute** the child Graph recursively through the same engine
6. **Map output** from child Graph's output via outgoing edge `Mapping` declarations
7. **Merge events** — child events are prefixed with child RunId but visible in the parent's event timeline

Sub-Graphs get their own snapshot, their own episode log, and their own budget tracker. A sub-Graph failure follows the parent node's failure strategy.

### Depth limit

Sub-Graph nesting is capped at 8 levels (configurable). Deeper nesting is a design smell — the Graph should be flattened.

---

## 18. Acceptance Criteria

| # | Criterion | Verification |
|---|---|---|
| 1 | Linear Graph runs end-to-end with all events emitted in order | Integration test on a 3-Block sequential Graph |
| 2 | Conditional edge: Branch condition evaluation routes correctly | Test fixture with branching condition |
| 3 | FanOut/FanIn: parallel branches with merged output via `Concat` strategy | Test on 3-way parallel fan-out |
| 4 | Loop terminates at `until` predicate satisfaction | Test: predicate met after 2 iterations |
| 5 | Loop respects `max_iterations` | Test: predicate never met, cap kicks in |
| 6 | Sub-Graph: parent + child events visible in unified timeline | Nested Graph test, verify `parent_run_id` |
| 7 | HumanInput: Flow pauses, resumes after `respond` invocation | Async integration test |
| 8 | Resume: kill engine mid-run; resume from snapshot; result identical to non-killed run | Property test on idempotent Graph |
| 9 | Retry-from: resume from a specific failing node | Test: fail node 3, retry-from node 3 |
| 10 | Cancellation: external cancel propagates to in-flight Blocks within 5s | Slow Block + cancel; Block reports cancelled |
| 11 | Budget `Cancel` strategy aborts on overage | Test: set low budget, run expensive Block |
| 12 | Budget `Downgrade` strategy re-routes to cheaper model | Test: hit budget, verify model tier change |
| 13 | Budget `SkipOptional` strategy skips optional nodes | Test: optional node skipped on budget hit |
| 14 | Episodes written for every Block run with correct cost/token/model fields | Verify episode count matches node count |
| 15 | Cascade router queries succeed and reflect prior episodes | Bandit state snapshot test |
| 16 | Lens events emitted for all lifecycle transitions | Capture events, verify completeness |
| 17 | Replan: failing Block triggers planner, revised sub-graph executes | Integration test with mock planner |
| 18 | Loop stall detection: 3 similar iterations emit `LoopStalled` | HDC similarity test |
| 19 | Sub-Graph budget carving: child cannot exceed allocation | Test: child overruns, verify failure |
| 20 | Cost estimate produced before execution; deviation logged after | Compare estimate vs. actual |

---

## 19. Open Questions

- **Speculative execution**: Should the engine start downstream nodes before upstream completes, cancelling if upstream output invalidates? Powerful but complex; defer to v2.
- **Shadow runs**: Run a candidate Graph alongside production, compare outputs, never persist shadow Signals? Useful for A/B testing Graphs; defer.
- **Per-Block resource limits**: CPU, memory, file handles per Block? Likely via cgroups for native, fuel for WASM, OS limits for scripts. Specify in v1.1.
- **Multi-machine engine**: Fan-out across machines for very large Graphs? Out of scope for v1; Space federation would enable this.
