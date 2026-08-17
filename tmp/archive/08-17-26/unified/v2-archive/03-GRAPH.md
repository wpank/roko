# 03 — Graph

> The universal composition. Cells wired by typed edges into a directed acyclic (or cyclic) graph. TOML-defined, serializable, runtime-interpreted. **Hot Graphs** stay resident and re-fire per tick. The **Workflow/Activity split** separates deterministic orchestration from non-deterministic execution for replay.

**Subsumes**: Workflow, Pipeline, DAG, Plan, TickPipeline, AgentLoop, Chain.

---

## 1. Graph Struct

A Graph is a composition of Cells connected by typed edges. Graphs are data — defined in TOML, loaded at runtime, validated before execution. The execution engine ([doc-05](05-EXECUTION-ENGINE.md)) interprets Graphs; it does not compile them.

```rust
/// A Graph is the universal composition primitive.
/// Everything that composes Cells is a Graph: plans, agent pipelines,
/// learning loops, dream cycles, trigger chains.
pub struct Graph {
    /// Stable identity. Content-addressed from (name, version, nodes, edges).
    pub id: GraphId,

    /// Human-readable name.
    pub name: String,

    /// Semantic version.
    pub version: Version,

    /// The nodes in this Graph.
    pub nodes: Vec<Node>,

    /// The edges connecting nodes.
    pub edges: Vec<Edge>,

    /// Entry node(s) — where execution begins.
    pub entry: Vec<NodeId>,

    /// Exit node(s) — where execution ends and output is collected.
    pub exits: Vec<NodeId>,

    /// Combined input schema (union of entry node inputs).
    pub input_schema: Option<TypeSchema>,

    /// Combined output schema (union of exit node outputs).
    pub output_schema: Option<TypeSchema>,

    /// Execution policy: budget, deadline, failure strategy, parallelism.
    pub policy: GraphPolicy,

    /// Metadata for authoring, discovery, and marketplace listing.
    pub metadata: GraphMetadata,
}

pub struct GraphId(pub ContentHash);

pub struct GraphMetadata {
    pub author: Author,
    pub description: String,
    pub tags: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
```

---

## 2. Nodes

A Node wraps a computation source: a Cell reference, a sub-Graph, a branch, a fan-out/fan-in, a loop, a human input gate, a wait, a slot (late-bound), or a noop (pass-through).

```rust
pub struct Node {
    /// Unique within this Graph.
    pub id: NodeId,

    /// Human-readable label (shown in TUI, logs).
    pub label: String,

    /// What this node does.
    pub kind: NodeKind,

    /// Override the Graph-level failure strategy for this node.
    pub failure_strategy: Option<FailureStrategy>,

    /// Maximum retries before escalating to the Graph-level strategy.
    pub max_retries: Option<u32>,

    /// Timeout override for this node.
    pub timeout: Option<Duration>,

    /// Execution classification for replay (see S6).
    pub execution_class: ExecutionClass,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NodeId(pub String);

pub enum NodeKind {
    /// Execute a Cell by reference.
    Cell {
        block_ref: CellRef,
        /// Static parameters passed to the Cell (T1 config tier).
        params: BTreeMap<String, Value>,
    },

    /// Embed another Graph as a sub-computation.
    /// The sub-Graph runs as a nested Flow with its own RunId.
    SubGraph {
        graph_ref: GraphRef,
    },

    /// Conditional branch: evaluate an expression, route to one of
    /// several downstream edges based on the result.
    Branch {
        condition: Expr,
        arms: Vec<BranchArm>,
        default: Option<NodeId>,
    },

    /// Fork execution to multiple downstream nodes in parallel.
    /// All downstream edges from a FanOut node execute concurrently.
    FanOut {
        /// How to split input Signals among downstream nodes.
        split: SplitStrategy,
    },

    /// Join parallel branches. Waits for all (or a quorum of) upstream
    /// nodes to complete before proceeding.
    FanIn {
        /// Merge strategy for combining results.
        merge: MergeStrategy,
        /// Minimum upstream completions required (default: all).
        quorum: Option<usize>,
    },

    /// Feedback loop: re-execute a sub-graph until a condition is met
    /// or a maximum iteration count is reached.
    Loop {
        body: GraphRef,
        condition: Expr,
        max_iterations: u32,
    },

    /// Pause execution until a human provides input.
    /// Publishes a Pulse on `human.input.requested` and waits.
    HumanInput {
        prompt: String,
        input_schema: Option<TypeSchema>,
        timeout: Option<Duration>,
    },

    /// Pause execution for a duration or until a Bus event.
    Wait {
        until: WaitCondition,
    },

    /// Late-bound slot. Filled at runtime by Rack macros or
    /// dynamic Graph construction. Execution fails if unfilled.
    Slot {
        name: String,
        expected_schema: Option<TypeSchema>,
        expected_protocols: Vec<ProtocolId>,
    },

    /// Pass-through. Used for Graph structure (merge points, labels).
    Noop,
}

pub struct BranchArm {
    pub label: String,
    pub pattern: Expr,
    pub target: NodeId,
}

pub enum WaitCondition {
    Duration(Duration),
    BusEvent { filter: TopicFilter },
    Both { duration: Duration, filter: TopicFilter },
}
```

### Split Strategies

```rust
pub enum SplitStrategy {
    /// Send all input Signals to every downstream node (broadcast).
    Broadcast,

    /// Send one Signal to each downstream node (round-robin).
    RoundRobin,

    /// Partition Signals by an expression evaluating to a node ID.
    Partition { key: Expr },

    /// Duplicate input, adding an index field.
    Indexed,
}
```

### Merge Strategies

```rust
pub enum MergeStrategy {
    /// Concatenate all output Signals.
    Concat,

    /// Take the first successful output (race).
    First,

    /// Take the output with the highest Score.relevance.
    BestScore { dimension: ScoreDimension },

    /// Apply a custom Compose Cell to merge outputs.
    Compose { block_ref: CellRef },

    /// Collect into a single array Signal.
    Collect,

    /// Voting: majority result wins (for redundant verification).
    Majority,
}

pub enum ScoreDimension {
    Relevance,
    Quality,
    Confidence,
    Novelty,
    Utility,
}
```

---

## 3. Edges

Edges connect nodes with optional conditions and data mappings.

```rust
pub struct Edge {
    /// Source node.
    pub from: NodeId,

    /// Target node.
    pub to: NodeId,

    /// Optional condition — edge is traversed only if the expression
    /// evaluates to true. If `None`, the edge is always traversed.
    pub condition: Option<Expr>,

    /// Data mapping: transform output Signals from `from` before
    /// delivering to `to`. If `None`, Signals pass through unchanged.
    pub mapping: Option<Mapping>,

    /// Edge label for debugging and TUI display.
    pub label: Option<String>,

    /// Priority for tie-breaking when multiple edges are eligible.
    /// Lower number = higher priority.
    pub priority: u32,
}

/// Data mapping between nodes.
pub struct Mapping {
    /// Field-level mappings from source output to target input.
    pub fields: Vec<FieldMapping>,
}

pub struct FieldMapping {
    /// JSONPath-like expression selecting from source output.
    pub from: String,
    /// JSONPath-like expression for target input location.
    pub to: String,
    /// Optional transformation expression.
    pub transform: Option<Expr>,
}
```

---

## 4. Expr Language

Edges and branches use a small expression language for conditions and transformations. The language is **total** (no infinite loops), **deterministic** (same inputs -> same outputs), and **bounded** (100ms timeout enforced by the runtime).

```rust
/// A small, total, deterministic expression language.
/// Evaluated by the runtime — not compiled. 100ms timeout.
pub enum Expr {
    // ── Literals ──────────────────────────────
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    Str(String),

    // ── References ────────────────────────────
    /// Reference a field in the current Signal payload.
    /// Uses dot-notation: "payload.status", "score.quality".
    Field(String),

    /// Reference a node output by node ID.
    NodeOutput { node: NodeId, field: String },

    /// Reference a Graph-level variable.
    Var(String),

    // ── Comparison ────────────────────────────
    Eq(Box<Expr>, Box<Expr>),
    Ne(Box<Expr>, Box<Expr>),
    Lt(Box<Expr>, Box<Expr>),
    Le(Box<Expr>, Box<Expr>),
    Gt(Box<Expr>, Box<Expr>),
    Ge(Box<Expr>, Box<Expr>),

    // ── Logic ─────────────────────────────────
    And(Box<Expr>, Box<Expr>),
    Or(Box<Expr>, Box<Expr>),
    Not(Box<Expr>),

    // ── Arithmetic ────────────────────────────
    Add(Box<Expr>, Box<Expr>),
    Sub(Box<Expr>, Box<Expr>),
    Mul(Box<Expr>, Box<Expr>),
    Div(Box<Expr>, Box<Expr>),

    // ── Collection ────────────────────────────
    Contains(Box<Expr>, Box<Expr>),
    Len(Box<Expr>),
    Index(Box<Expr>, Box<Expr>),

    // ── String ────────────────────────────────
    Matches(Box<Expr>, String),  // regex match
    StartsWith(Box<Expr>, String),
    EndsWith(Box<Expr>, String),

    // ── Conditional ───────────────────────────
    If {
        cond: Box<Expr>,
        then: Box<Expr>,
        otherwise: Box<Expr>,
    },

    // ── Type ──────────────────────────────────
    KindIs(Kind),
    HasField(String),
}
```

**Design decisions**:
- **No loops**: The language is total by construction. Iteration is handled by `Loop` nodes.
- **No side effects**: Expressions cannot publish, store, or mutate. They are pure functions of their input.
- **100ms timeout**: Even deeply nested expressions are bounded. The runtime aborts evaluation and returns an error if the timeout fires.
- **Deterministic**: Given the same input Signals and variables, the same expression always produces the same result. This is essential for the Workflow/Activity split (S6).

---

## 5. TOML Authoring

Graphs are authored in TOML and loaded by the runtime. The TOML format is the primary authoring surface for plans, workflows, and agent pipelines.

```toml
# plans/code-review.toml — A code review Graph

[graph]
name = "code-review"
version = "0.1.0"
description = "Review a code change: lint, test, score, compose verdict."

[graph.policy]
max_budget_usd = 0.50
deadline_seconds = 300
failure_strategy = "retry_with_escalation"
max_parallelism = 4

# ── Nodes ──────────────────────────────────────────

[[nodes]]
id = "fetch-diff"
label = "Fetch diff from PR"
kind = "block"
block = "builtin://github-diff-fetcher"

[[nodes]]
id = "lint"
label = "Run clippy"
kind = "block"
block = "builtin://clippy-gate"

[[nodes]]
id = "test"
label = "Run tests"
kind = "block"
block = "builtin://test-gate"

[[nodes]]
id = "score"
label = "AI code quality score"
kind = "block"
block = "code-reviewer"
[nodes.params]
model = "claude-sonnet-4-20250514"

[[nodes]]
id = "fan-in"
label = "Merge verification results"
kind = "fan_in"
merge = "collect"

[[nodes]]
id = "compose-verdict"
label = "Compose final verdict"
kind = "block"
block = "builtin://verdict-composer"

[[nodes]]
id = "branch-pass"
label = "Pass or fail?"
kind = "branch"
default = "report-fail"

[[nodes.arms]]
label = "all-pass"
pattern = "all(payload.results, |r| r.passed)"
target = "report-pass"

[[nodes]]
id = "report-pass"
label = "Report pass"
kind = "block"
block = "builtin://pr-commenter"
[nodes.params]
template = "pass"

[[nodes]]
id = "report-fail"
label = "Report failure"
kind = "block"
block = "builtin://pr-commenter"
[nodes.params]
template = "fail"

# ── Edges ──────────────────────────────────────────

[[edges]]
from = "fetch-diff"
to = "lint"

[[edges]]
from = "fetch-diff"
to = "test"

[[edges]]
from = "fetch-diff"
to = "score"

[[edges]]
from = "lint"
to = "fan-in"

[[edges]]
from = "test"
to = "fan-in"

[[edges]]
from = "score"
to = "fan-in"

[[edges]]
from = "fan-in"
to = "compose-verdict"

[[edges]]
from = "compose-verdict"
to = "branch-pass"

# Entry and exit are inferred from topology if not specified.
# Entry: nodes with no incoming edges. Exit: nodes with no outgoing edges.
```

### TOML Node Kinds

| TOML `kind` | Rust `NodeKind` | Required fields |
|---|---|---|
| `"block"` | `Cell` | `block` (CellRef) |
| `"sub_graph"` | `SubGraph` | `graph` (GraphRef) |
| `"branch"` | `Branch` | `condition` or `arms[]` |
| `"fan_out"` | `FanOut` | `split` (optional, default: broadcast) |
| `"fan_in"` | `FanIn` | `merge` (optional, default: concat) |
| `"loop"` | `Loop` | `body` (GraphRef), `condition`, `max_iterations` |
| `"human_input"` | `HumanInput` | `prompt` |
| `"wait"` | `Wait` | `until` (duration or event filter) |
| `"slot"` | `Slot` | `name` |
| `"noop"` | `Noop` | none |

---

## 6. Workflow/Activity Split

Inspired by Temporal (https://temporal.io), the runtime classifies each node as either a **Workflow** node or an **Activity** node. This classification determines replay semantics.

```rust
/// Execution classification. Determines replay behavior.
pub enum ExecutionClass {
    /// Deterministic orchestration. Pure Rust, no side effects,
    /// no LLM calls. Replaying Workflow nodes re-executes the code
    /// and produces identical results.
    ///
    /// Examples: Branch, FanOut, FanIn, Noop, Expr evaluation,
    /// data mapping, merge strategy application.
    Workflow,

    /// Non-deterministic execution. LLM calls, shell commands,
    /// network requests, human input. Replaying Activity nodes
    /// returns the recorded output from the first execution.
    ///
    /// Examples: LLM-backed Cells, shell gates, API connectors,
    /// human input, any Cell with side effects.
    Activity,
}
```

### Classification rules

| Node Kind | Default Class | Override? |
|---|---|---|
| `Cell` (LLM call) | Activity | No |
| `Cell` (pure Rust, no I/O) | Workflow | No |
| `Cell` (shell execution) | Activity | No |
| `SubGraph` | Inherits from children | No |
| `Branch` | Workflow | No |
| `FanOut` | Workflow | No |
| `FanIn` | Workflow | No |
| `Loop` (condition eval) | Workflow | No |
| `HumanInput` | Activity | No |
| `Wait` | Activity | No |
| `Slot` | Depends on what fills it | No |
| `Noop` | Workflow | No |

### Replay semantics

When a Flow is resumed from a snapshot ([doc-05](05-EXECUTION-ENGINE.md)):

1. **Workflow nodes** are re-executed. Their code is deterministic, so the same inputs produce the same control-flow decisions. This reconstructs the execution DAG.
2. **Activity nodes** are NOT re-executed. Instead, the recorded output from the first execution is loaded from the run storage. This avoids re-running LLM calls, re-executing shell commands, or re-requesting human input.

This split makes resumability safe and predictable. The execution engine records Activity outputs in the run storage directory (`runs/<run-id>/activities/`).

---

## 7. Graph Policy

Every Graph carries a policy governing its execution.

```rust
pub struct GraphPolicy {
    /// Maximum budget (USD-equivalent) for the entire Graph execution.
    /// Includes all Cell costs, LLM calls, and sub-Graph costs.
    pub max_budget: Option<Cost>,

    /// Maximum wall-clock time for the entire Graph execution.
    pub deadline: Option<Duration>,

    /// Default failure strategy for nodes that don't override it.
    pub failure_strategy: FailureStrategy,

    /// Maximum number of nodes executing concurrently.
    /// Limits resource consumption for fan-out-heavy Graphs.
    pub max_parallelism: Option<usize>,

    /// Whether to snapshot state after each node completion
    /// (enables resumability at the cost of I/O).
    pub snapshot_after_each_node: bool,

    /// Whether this Graph is a Hot Graph (stays resident between firings).
    pub hot: bool,

    /// Clock binding for Hot Graphs (see S8).
    pub clock: Option<ClockBinding>,

    /// Demurrage: whether intermediate Signals produced by this Graph
    /// are subject to demurrage or exempt during execution.
    pub exempt_intermediate_demurrage: bool,
}
```

### Failure strategies

```rust
/// What to do when a node fails.
pub enum FailureStrategy {
    /// Fail the entire Graph immediately.
    Fail,

    /// Retry the failed node up to N times with backoff.
    Retry {
        max_retries: u32,
        backoff: BackoffStrategy,
    },

    /// Retry with a different (more capable) Cell on each retry.
    /// The Route protocol selects alternatives.
    RetryWithEscalation {
        max_retries: u32,
    },

    /// Decompose the failed task into smaller sub-tasks and retry each.
    /// Uses an LLM to generate the decomposition.
    Decompose {
        max_depth: u32,
    },

    /// Skip the failed node and continue execution.
    /// Downstream nodes receive empty input from the skipped node.
    Skip,

    /// Execute a compensation Graph to undo partial work.
    Compensate {
        compensation_graph: GraphRef,
    },

    /// Generate a new plan from the failure context and execute it.
    /// Uses the same plan-generation pipeline as `roko prd plan`.
    Replan {
        max_replans: u32,
    },

    /// Pause and wait for human resolution.
    HumanResolve {
        prompt: String,
        timeout: Option<Duration>,
    },
}

pub enum BackoffStrategy {
    /// Fixed delay between retries.
    Fixed(Duration),
    /// Exponential backoff with jitter.
    Exponential { base: Duration, max: Duration },
    /// Linear increase.
    Linear { step: Duration, max: Duration },
}
```

---

## 8. Hot Graphs

A **Hot Graph** is a Graph that stays resident in memory between firings. Instead of creating a new Flow for each invocation, a Hot Graph retains its state and re-fires on each tick of a bound clock. This is how Agent pipelines work: the Agent's 9-step processing pipeline is a Hot Graph that fires every tick.

```rust
/// Clock binding for Hot Graphs.
pub enum ClockBinding {
    /// Agent gamma clock (~100ms). Sensory/reflex processing.
    AgentGamma,

    /// Agent theta clock (~1s). Working memory, attention updates.
    AgentTheta,

    /// Agent delta clock (~10s). Planning, consolidation.
    AgentDelta,

    /// Custom clock with explicit period.
    Custom {
        period: Duration,
        name: String,
    },
}
```

### Hot Graph execution model

1. **Initialization**: The Hot Graph is loaded and its entry nodes are prepared. State slots are initialized.
2. **Tick**: On each clock tick, the entry nodes receive the current state (updated since the last tick) and execute. Execution propagates through the Graph.
3. **State retention**: Between ticks, node outputs are retained in memory. The next tick's entry nodes receive the previous tick's exit node outputs (plus any new external input).
4. **Teardown**: When the owning Agent enters the Terminal lifecycle state, the Hot Graph flushes state to Store and shuts down.

### Agent's 9-step pipeline as a Hot Graph

The Agent's core processing pipeline is defined as a Hot Graph bound to `AgentTheta` (~1s):

```toml
[graph]
name = "agent-pipeline"
version = "0.1.0"
hot = true
clock = "agent_theta"

[[nodes]]
id = "sense"
label = "1. Sense: gather input Pulses from Bus"
kind = "block"
block = "builtin://agent-sense"

[[nodes]]
id = "attend"
label = "2. Attend: filter by relevance + somatic markers"
kind = "block"
block = "builtin://agent-attend"

[[nodes]]
id = "retrieve"
label = "3. Retrieve: query Store for relevant context"
kind = "block"
block = "builtin://agent-retrieve"

[[nodes]]
id = "compose"
label = "4. Compose: VCG auction for context assembly"
kind = "block"
block = "builtin://agent-compose"

[[nodes]]
id = "route"
label = "5. Route: EFE model selection"
kind = "block"
block = "builtin://agent-route"

[[nodes]]
id = "act"
label = "6. Act: execute the selected Cell"
kind = "block"
block = "builtin://agent-act"

[[nodes]]
id = "verify"
label = "7. Verify: gate the output"
kind = "block"
block = "builtin://agent-verify"

[[nodes]]
id = "store"
label = "8. Store: persist results"
kind = "block"
block = "builtin://agent-store"

[[nodes]]
id = "react"
label = "9. React: emit Pulses, update state"
kind = "block"
block = "builtin://agent-react"

[[edges]]
from = "sense"
to = "attend"

[[edges]]
from = "attend"
to = "retrieve"

[[edges]]
from = "retrieve"
to = "compose"

[[edges]]
from = "compose"
to = "route"

[[edges]]
from = "route"
to = "act"

[[edges]]
from = "act"
to = "verify"

[[edges]]
from = "verify"
to = "store"

[[edges]]
from = "store"
to = "react"
```

---

## 9. Graph Validation

Before execution, the runtime validates a Graph. Validation catches structural errors at load time rather than runtime.

```rust
pub struct ValidationReport {
    pub errors: Vec<ValidationError>,
    pub warnings: Vec<ValidationWarning>,
}

pub enum ValidationError {
    /// A node references a Cell that is not registered.
    UnknownCell { node: NodeId, block_ref: CellRef },

    /// An edge connects incompatible TypeSchemas.
    TypeMismatch { edge: (NodeId, NodeId), output: TypeSchema, input: TypeSchema },

    /// The Graph has no entry nodes (no nodes without incoming edges).
    NoEntry,

    /// The Graph has no exit nodes (no nodes without outgoing edges,
    /// excluding loops).
    NoExit,

    /// A Slot node is unfilled.
    UnfilledSlot { node: NodeId, slot_name: String },

    /// A cycle exists outside of a Loop node.
    UnexpectedCycle { nodes: Vec<NodeId> },

    /// A required capability is missing from the effective capability set.
    MissingCapability { node: NodeId, capability: Capability },

    /// Budget is statically insufficient for estimated costs.
    InsufficientBudget { estimated: Cost, budgeted: Cost },

    /// FanIn quorum exceeds number of upstream nodes.
    InvalidQuorum { node: NodeId, quorum: usize, upstream: usize },
}

pub enum ValidationWarning {
    /// A node has no downstream edges (dead end, but not an exit node).
    DeadEnd { node: NodeId },

    /// Estimated cost is close to budget limit (>80%).
    NearBudget { estimated: Cost, budgeted: Cost },

    /// A node has no timeout and no Graph-level deadline.
    Unbounded { node: NodeId },

    /// An Activity node has no snapshot policy.
    ActivityWithoutSnapshot { node: NodeId },
}
```

---

## 10. Run Storage Layout

Each Graph execution (Flow) stores its artifacts in a structured directory under `.roko/runs/`.

```
.roko/runs/<run-id>/
├── graph.toml              # Frozen copy of the Graph definition
├── state.json              # Current execution state (for resumability)
├── activities/             # Recorded Activity outputs (for replay)
│   ├── <node-id>.json      # Output Signals from each Activity node
│   └── ...
├── events.jsonl            # Graduated Pulse snapshots (append-only)
├── artifacts/              # Files produced by Cell execution
│   ├── <node-id>/
│   │   ├── output.json     # Structured output
│   │   └── files/          # Generated files
│   └── ...
├── snapshots/              # Per-node state snapshots (if enabled)
│   ├── <node-id>.snap.json
│   └── ...
├── cost.json               # Cost accounting: per-node, per-Cell, total
└── verdict.json            # Final verdict (if the Graph includes Verify)
```

### State snapshot format

```rust
/// Serializable execution state for resumability.
pub struct FlowSnapshot {
    pub run_id: RunId,
    pub graph_id: GraphId,
    pub created_at: DateTime<Utc>,
    pub snapshot_at: DateTime<Utc>,

    /// Per-node execution status.
    pub node_states: BTreeMap<NodeId, NodeState>,

    /// Graph-level variables (set by Expr evaluations).
    pub variables: BTreeMap<String, Value>,

    /// Total cost so far.
    pub cost_so_far: Cost,

    /// Total wall-clock time so far.
    pub elapsed: Duration,
}

pub enum NodeState {
    Pending,
    Running { started_at: DateTime<Utc> },
    Completed {
        output: Vec<Signal>,
        duration: Duration,
        cost: Cost,
    },
    Failed {
        error: String,
        attempts: u32,
    },
    Skipped { reason: String },
    Cancelled,
}
```

---

## 11. GraphRef and Graph Composition

Graphs can be referenced and embedded inside other Graphs via `SubGraph` nodes. This enables hierarchical composition.

```rust
/// Reference to a Graph for embedding or invocation.
pub enum GraphRef {
    /// A Graph defined in the same workspace.
    Local { path: PathBuf },

    /// A Graph from the marketplace.
    Published { id: GraphId, version: Version },

    /// A built-in Graph shipped with Roko.
    Builtin { name: String },

    /// An inline Graph defined within the parent Graph's TOML.
    Inline(Box<Graph>),
}
```

```toml
# Embedding a sub-graph
[[nodes]]
id = "verification-suite"
label = "Run the full verification suite"
kind = "sub_graph"
graph = "plans/verify-suite.toml"
```

---

## 12. Graph as Data, Not Code

Graphs are serializable data structures, not code. This enables:

1. **TOML authoring** by domain experts (no Rust required).
2. **Agent-generated plans**: `roko prd plan <slug>` produces a Graph (as `tasks.toml`).
3. **Marketplace sharing**: Graphs are portable artifacts with declared schemas.
4. **Versioning**: Graphs are content-addressed. A frozen copy is stored with each run.
5. **Validation**: The runtime validates before executing. Type mismatches, missing Cells, and budget insufficiency are caught at load time.
6. **Visualization**: The TUI renders Graphs as node diagrams. The Generative Canvas ([doc-16](16-SURFACES.md)) enables visual editing.
7. **Evolution**: L4 evolution can propose Graph mutations (add/remove nodes, change edges, adjust policies). Human review required for safety-critical Graphs.

---

## 13. Acceptance Criteria

| Criterion | Verification |
|---|---|
| `Graph` struct compiles with `nodes`, `edges`, `entry`, `exits`, `policy` | Compile check |
| All 10 `NodeKind` variants compile | Compile check |
| `Edge` struct compiles with `condition: Option<Expr>` and `mapping: Option<Mapping>` | Compile check |
| `Expr` language is total: no construct can loop | Review: no recursion-enabling constructors |
| `Expr` evaluation respects 100ms timeout | Unit test with deeply nested expression |
| `Expr` is deterministic: same inputs -> same outputs | Unit test with random seed comparison |
| TOML Graph loads and validates successfully | Integration test with `code-review.toml` example |
| TypeSchema mismatch on edge detected at validation time | Unit test: connect `Kind::Diff` output to `Kind::Json` input |
| Unfilled Slot detected at validation time | Unit test |
| Unexpected cycle (outside Loop node) detected at validation time | Unit test |
| SubGraph nodes create nested Flows with separate RunIds | Integration test |
| Hot Graph retains state between ticks | Integration test: two ticks, second tick sees first tick's output |
| `ClockBinding` variants bind to correct periods | Unit test |
| Agent's 9-step pipeline loads as a valid Hot Graph | Integration test |
| Workflow/Activity classification is correct for all NodeKinds | Unit test per NodeKind |
| Activity replay returns recorded output, not re-execution | Integration test: resume Flow, verify no LLM call |
| Workflow replay re-executes and produces same control flow | Integration test: resume Flow, verify same branch taken |
| `FlowSnapshot` serializes and deserializes round-trip | Unit test |
| Run storage layout created on Flow start | Integration test: check directory structure |
| `MergeStrategy::Majority` correctly selects majority result | Unit test with 3 inputs (2 agree, 1 disagrees) |
| `FailureStrategy::RetryWithEscalation` uses Route to select alternative | Integration test |
| `FailureStrategy::Replan` generates new plan from failure context | Integration test |
| Graph validation produces both errors and warnings | Unit test with problematic Graph |
| `GraphRef::Inline` allows inline sub-graphs in TOML | Integration test |
| Budget enforcement: execution halts when budget exceeded | Integration test |
| Parallelism limit respected in FanOut execution | Integration test with `max_parallelism = 2` and 4 branches |
