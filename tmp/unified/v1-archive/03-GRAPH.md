# 03 — Graph

> The universal composition. Every pipeline, workflow, recipe, dream cycle, and gate chain is a Graph. Hot Graphs stay resident and re-fire per tick.

**Subsumes**: Workflow, StateGraph, Extension chain, Recipe pipeline, 9-step agent pipeline, Gate pipeline, DreamCycle.

---

## 1. Definition

A **Graph** is a TOML-defined, serializable composition of Blocks wired by typed edges. The runtime interprets it — Graphs are data, not traits. This preserves declarative authoring: users compose Blocks visually or in TOML; the engine executes.

```rust
pub struct Graph {
    pub identity: GraphIdentity,
    pub description: String,
    pub tags: Vec<String>,
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
    pub entry: NodeId,
    pub exits: Vec<NodeId>,            // multiple terminal states allowed
    pub schema: GraphSchema,           // input/output types
    pub policy: GraphPolicy,           // failure, budget, parallelism
    pub hot: Option<HotGraphConfig>,   // if present, Graph stays resident (see §11)
}

pub struct GraphIdentity {
    pub name: String,                  // kebab-case, unique
    pub version: Version,              // semver
    pub publisher: Option<String>,     // marketplace handle
    pub forked_from: Option<SignalRef>, // lineage
}

pub struct GraphSchema {
    pub input: TypeSchema,             // Graph-level input
    pub output: TypeSchema,            // Graph-level output
}
```

---

## 2. Node Kinds

```rust
pub enum Node {
    /// Execute a Block.
    Block { id: NodeId, block: BlockRef, params: Value },

    /// Recursively execute a sub-Graph.
    SubGraph { id: NodeId, graph: GraphRef, params: Value },

    /// Conditional fan-out: evaluate condition, walk matching edges.
    Branch { id: NodeId, condition: Expr },

    /// Parallel fan-out: iterate expression, spawn one child per element.
    FanOut { id: NodeId, over: Expr, max_parallelism: usize },

    /// Merge parallel branches.
    FanIn { id: NodeId, strategy: MergeStrategy },

    /// Repeat body subgraph until predicate or max iterations.
    Loop { id: NodeId, body: NodeId, until: Expr, max_iterations: u32 },

    /// Pause for human input.
    HumanInput { id: NodeId, prompt: String, schema: TypeSchema, timeout: Option<Duration> },

    /// Wait for an external condition.
    Wait { id: NodeId, until: WaitCondition },

    /// Typed empty position — resolved at run start (see Rack in doc-04).
    Slot { id: NodeId, slot_ref: SlotRef },

    /// Pass-through synchronization point.
    Noop { id: NodeId },
}
```

### Engine semantics per node kind

| Node | Behavior |
|---|---|
| `Block` | Resolve Block by ref + version. Build `BlockInput` (project upstream output via edge mapping). Acquire capabilities. Invoke `Block::run`. Capture output. Emit lifecycle Pulses on Bus. |
| `SubGraph` | Recursively invoke engine on child Graph with its own RunId. Pulses bubble up to parent with `parent_run_id` breadcrumb. Output projects through edge mappings. |
| `Branch` | Evaluate `condition` Expr against current state. Walk only edges whose condition evaluates true. Multiple matching edges fan out in parallel. |
| `FanOut` | Iterate `over` expression (must yield array). Spawn one child per element, capped by `max_parallelism`. Children execute downstream subgraph until next `FanIn`. |
| `FanIn` | Wait for all parallel branches. Apply MergeStrategy: `Concat`, `FirstSuccess`, `AllOrFail`, `Vote`. Continue with merged state. |
| `Loop` | Repeat `body` subgraph. Evaluate `until` Expr each iteration. Bounded by `max_iterations`. Emit `LoopIteration` Pulses. |
| `HumanInput` | Persist state. Emit `HumanInputRequested` Pulse. Wait for response via dashboard/TUI/CLI. Validate against schema. Resume. |
| `Wait` | Block until WaitCondition is satisfied (Signal arrives, Pulse received, time elapsed, sub-Graph completes). |
| `Slot` | Resolved at run start — replaced with the user-bound Block/sub-Graph. Engine never sees a raw Slot at runtime. |
| `Noop` | Pass-through. Used as synchronization point between parallel paths. |

---

## 3. Edges

```rust
pub struct Edge {
    pub from: NodeId,
    pub to: NodeId,
    pub condition: Option<Expr>,       // None = unconditional
    pub maps: Vec<Mapping>,            // input projection from upstream output
}

pub struct Mapping {
    pub source: String,                // dotted path into upstream output
    pub target: String,                // dotted path into downstream input
    pub transform: Option<Expr>,       // optional value transformation
}
```

Edges carry data between nodes. The `maps` field projects specific fields from upstream output into downstream input. This is how Graphs wire typed data flows without requiring Blocks to know about each other.

### Edge evaluation

- Edges with no condition are always traversed.
- Multiple matching edges from a Branch fan out in parallel.
- Zero matching edges from a non-exit node is a runtime error.
- Conditions are evaluated in source-node-completion order.

---

## 4. Expression Language

The **Expr** language is small, total, and deterministic. Used for edge conditions, loop predicates, fan-out sources, macro transforms.

```
expr   := value | binop | unop | call | path | index
value  := bool | int | float | string | null
binop  := "==" | "!=" | "<" | "<=" | ">" | ">="
        | "AND" | "OR" | "+" | "-" | "*" | "/"
        | "in" | "matches"
unop   := "NOT" | "-"
path   := identifier ("." identifier)*
index  := path "[" expr "]"
call   := identifier "(" expr ("," expr)* ")"
```

### Built-in functions

`len`, `first`, `last`, `flatten`, `unique`, `sort`, `lower`, `upper`, `now`, `count_where`, `any`, `all`, `sum`, `max`, `min`.

### Variables in scope

| Variable | Meaning |
|---|---|
| `input` | Graph-level input |
| `output` | Last completed node's output |
| `<node-id>` | Any prior completed node's output by ID |
| `macros` | Resolved macro values (if in a Rack) |
| `slots` | Resolved slot fillings (if in a Rack) |
| `run` | Run-level metadata: `id`, `started_at`, `elapsed` |

### Safety

Expr evaluation has a **100ms timeout** per invocation. Long evaluation is a bug. The language is deliberately not Turing-complete — no recursion, no user-defined functions, no unbounded loops.

---

## 5. Sub-Graph Composition

A `SubGraph` node references another Graph by `name@version` and runs it as a node in the parent. The parent's state graph waits for the sub-Graph to terminate. Sub-Graph inputs and outputs are mapped via `Mapping` declarations on incoming and outgoing edges.

This is how Graphs compose. It is the same engine recursing. Sub-Graphs get their own RunId, their own Pulse stream on Bus, and their own budget (carved from the parent's budget).

```toml
# In parent Graph TOML
[[nodes]]
id = "verify"
type = "sub-graph"
graph = "code-quality-check@^1.0"
params = { strictness = "high" }
```

---

## 6. TOML Authoring

Graphs are authored as TOML files. The engine loads, validates types, resolves Block references, and executes.

```toml
[graph]
name = "doc-ingest"
version = "1.0.0"
description = "Ingest a document, analyze it, produce structured findings"

[schema]
input = { type = "object", fields = { url = "string", depth = "int" } }
output = { type = "object", fields = { findings = { type = "array", items = "Finding" } } }

# ── Nodes ──────────────────────────────────────────────────────

[[nodes]]
id = "fetch"
type = "block"
block = "web-fetcher@^1.0"

[[nodes]]
id = "parse"
type = "block"
block = "markdown-parser@^1.0"

[[nodes]]
id = "analyze"
type = "fan-out"
over = "parse.sections"
max_parallelism = 4

[[nodes]]
id = "analyze-section"
type = "block"
block = "llm-analyzer@^1.0"
params = { role = "analyst", temperature = 0.3 }

[[nodes]]
id = "merge"
type = "fan-in"
strategy = "concat"

[[nodes]]
id = "synthesize"
type = "block"
block = "llm-synthesizer@^1.0"

# ── Edges ──────────────────────────────────────────────────────

[[edges]]
from = "fetch"
to = "parse"

[[edges]]
from = "parse"
to = "analyze"

[[edges]]
from = "analyze"
to = "analyze-section"

[[edges]]
from = "analyze-section"
to = "merge"

[[edges]]
from = "merge"
to = "synthesize"

# ── Policy ─────────────────────────────────────────────────────

[policy]
budget_usd = 2.0
deadline_secs = 300
on_failure = "retry-with-escalation"
max_retries = 2
parallelism_cap = 8
```

---

## 7. Graph Policy

Every Graph carries a `GraphPolicy` governing execution behavior:

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

Failure strategies are defined in [doc-05 (Execution Engine)](05-EXECUTION-ENGINE.md):

| Strategy | Behavior |
|---|---|
| `Fail` | Any Block failure fails the Flow |
| `Retry` | Retry with backoff |
| `RetryWithEscalation` | Retry, escalate model tier on each retry |
| `Skip` | Continue past failure, mark output as skipped |
| `Compensate` | Run cleanup Block, then continue |
| `Replan` | Invoke planner Block to revise remaining graph |
| `HumanResolve` | Pause for human decision |

---

## 8. Merge Strategies

`FanIn` nodes merge parallel branches:

```rust
pub enum MergeStrategy {
    /// Concatenate all outputs into an array.
    Concat,
    /// Take the first branch that succeeds, cancel others.
    FirstSuccess,
    /// All branches must succeed; fail if any fails.
    AllOrFail,
    /// Majority vote on a specific output field.
    Vote { field: String, threshold: f64 },
    /// Custom merge via a Block.
    Custom { block: BlockRef },
}
```

---

## 9. Run Storage Layout

When a Graph executes as a Flow, state is stored at:

```
.roko/runs/<run-id>/
├── snapshot.json           # latest checkpoint
├── snapshot.<seq>.json     # historical checkpoints (retention configurable)
├── input.json              # Graph input
├── output.json             # populated on completion
├── events.jsonl            # graduated Pulses snapshot (audit trail)
├── artifacts/              # artifact Signals produced
│   └── sig_<id>
├── episodes/               # episode Signals for this run
│   └── ep_<id>.json
└── manifest.json           # status, timing, error if any
```

---

## 10. Relationship to Specializations

Several specializations ([doc-04](04-SPECIALIZATIONS.md)) are defined in terms of Graph:

| Specialization | What makes it special |
|---|---|
| **Flow** | A Graph at runtime — has a RunId, produces snapshots and events |
| **Hot Flow** | A Flow that stays resident between firings and re-fires per tick (see §11) |
| **Rack** | A Graph with Macros (knobs) and Slots (jacks) exposed to consumers |
| **Loop** | A Graph that feeds output back to input (self-referential edge) |
| **Agent pipeline** | The 9-step Graph inside every Agent — always a Hot Graph |
| **Verification pipeline** | A Graph of Verify-protocol Blocks producing a Verdict |

All use the same Graph type. The specialization is a convention — a pattern of node/edge usage — not a new data type.

---

## 11. Hot Graph

A **Hot Graph** is a Flow that stays resident in memory between firings. Instead of the standard lifecycle (Created -> Running -> Completed), a Hot Graph runs perpetually: it fires on each tick of its bound clock, retains state between firings, and only terminates when explicitly stopped or when its owner shuts down.

### Definition

```rust
pub struct HotGraphConfig {
    pub clock: ClockBinding,           // which clock drives re-firing
    pub retain_state: bool,            // carry node outputs across firings
    pub cooldown: Option<Duration>,    // minimum interval between firings
}

pub enum ClockBinding {
    AgentGamma,                        // fast perception (100ms - 2s)
    AgentTheta,                        // reflective planning (750ms - 16s)
    AgentDelta,                        // deep consolidation (60s - 10m)
    Custom { interval: Duration },     // fixed interval
    External { topic: Topic },         // fire on Bus Pulse arrival
}
```

### Lifecycle

```
Loaded → Resident → [tick] → Firing → Quiescent → [tick] → Firing → ...
                                                        ↘ Stopped
```

A Hot Graph does not complete after a single pass. It returns to Quiescent after each firing and waits for the next tick. State from completed nodes persists between firings when `retain_state = true` — the next tick sees prior outputs and can branch differently.

### Workflow / Activity Split

Within a Hot Graph, nodes fall into two categories:

| Category | Nature | Replay | Examples |
|---|---|---|---|
| **Workflow** | Deterministic orchestration | Replay from code — re-execute the pure Rust logic | Edge evaluation, Expr conditions, Branch decisions, FanOut iteration, merge, state transitions |
| **Activity** | Non-deterministic execution | Replay from recorded output — do not re-execute | LLM calls, shell commands, HTTP requests, chain transactions, file I/O |

The distinction matters for **deterministic replay**. When resuming or debugging a Hot Graph:

1. Workflow nodes re-execute their logic (pure, deterministic, cheap)
2. Activity nodes replay their recorded output from the event log (avoids re-calling LLMs, re-running shell commands)

This guarantees that replaying a Hot Graph produces identical state transitions without incurring Activity costs. The engine tags each node's output as Workflow or Activity at execution time.

```rust
pub enum NodeOutput {
    Workflow(Value),                   // re-derivable from inputs
    Activity {                         // recorded, non-deterministic
        value: Value,
        recorded_at: DateTime<Utc>,
        cost: Cost,
    },
}
```

### The 9-Step Agent Pipeline as a Hot Graph

The Agent's internal 9-step pipeline (Observe, Retrieve, Analyze, Gate, Simulate, Validate, Execute, Verify, Reflect) is a Hot Graph bound to the Agent's adaptive clock. It fires every tick. Between firings, CorticalState (goals, beliefs, attention) persists. The pipeline's Workflow nodes (Gate decision, state transitions) replay deterministically; its Activity nodes (LLM calls in Simulate/Execute, tool use in Execute) replay from their recorded output.

This is why the Agent pipeline spec in [doc-07 (Agent Runtime)](07-AGENT-RUNTIME.md) does not define a separate execution model — it uses the same Hot Graph machinery defined here.

### TOML authoring

```toml
[graph]
name = "agent-perception-loop"
version = "1.0.0"

[graph.hot]
clock = "agent-gamma"
retain_state = true
cooldown = "100ms"

[[nodes]]
id = "observe"
type = "block"
block = "inbox-scanner@^1.0"

[[nodes]]
id = "analyze"
type = "block"
block = "prediction-error-computer@^1.0"

[[nodes]]
id = "gate"
type = "block"
block = "efe-gating@^1.0"

# ... remaining pipeline nodes

[policy]
on_failure = "skip"
```

### Hot Graph events

Hot Graphs emit additional Pulses on Bus beyond standard Flow lifecycle:

| Pulse topic | When |
|---|---|
| `hot:{graph}:tick` | Clock tick received, firing begins |
| `hot:{graph}:quiescent` | Firing complete, waiting for next tick |
| `hot:{graph}:state_carried` | State retained from prior firing |
| `hot:{graph}:stopped` | Hot Graph terminated |

These Pulses are ephemeral. The graduation policy for Hot Graph Pulses is selective: tick and quiescent Pulses are not graduated (too frequent); state-carry and stop Pulses graduate to Signals for audit.

---

## 12. Acceptance Criteria

| Criterion | Verification |
|---|---|
| Graph loads from TOML with type-checked edges | Unit test: load valid TOML, verify all edges type-check |
| Type mismatch on edge rejected at load time | Unit test: wire `String -> i32`, expect schema error |
| SubGraph composition: parent + child Pulses visible in unified timeline | Integration test: nested Graph, verify Pulse parent_run_id |
| FanOut/FanIn: parallel execution with correct merge | Integration test: 3-way parallel, concat merge |
| Loop terminates at predicate or max_iterations | Two tests: early predicate, cap hit |
| Edge conditions: Branch fans out to multiple matching edges | Test: Branch with two matching conditions, both execute |
| Expr evaluation times out at 100ms | Test: expensive Expr, verify timeout error |
| Graph-level budget carves correctly into sub-Graph budgets | Integration test: parent budget, sub-Graph allocation |
| Slot resolution: unresolved required Slot errors at load time | Negative test: required Slot unfilled |
| TOML round-trip: serialize -> deserialize -> serialize produces identical output | Property test |
| Hot Graph stays resident across multiple ticks | Integration test: 3 ticks, verify state retention |
| Hot Graph Workflow nodes replay deterministically from code | Replay test: same inputs produce same state transitions |
| Hot Graph Activity nodes replay from recorded output | Replay test: LLM node replays without calling LLM |
| Hot Graph emits tick/quiescent Pulses on Bus | Bus subscriber test |
| Hot Graph stops cleanly on owner shutdown | Test: stop Agent, verify Hot Graph stopped Pulse |
| Hot Graph TOML with `[graph.hot]` loads and validates | Parse test |
