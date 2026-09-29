# DAG Topology

> DAG structure, node and edge types, topological sort, wave computation,
> and edge condition evaluation. Everything in roko-graph is a petgraph-backed
> directed acyclic graph of typed, conditional edges.

---

## Source Files

| File | What |
|---|---|
| `crates/roko-graph/src/types.rs` | Graph, Node, Edge, EdgeCondition, GraphPolicy, NodeOutput |
| `crates/roko-graph/src/topo.rs` | topological_order, topological_waves, root_nodes, leaf_nodes |
| `crates/roko-graph/src/condition.rs` | Condition, CompareOp, evaluate, resolve_field |

---

## The Graph Struct

```rust
pub struct Graph {
    pub metadata: GraphMetadata,              // name, description, version, labels
    pub policy:   GraphPolicy,                // mode, failure_strategy, max_concurrent_nodes, timeout, hot, capabilities
    pub lenses:   LensRegistry,               // telemetry Lens routing
    pub inner:    DiGraph<Node, Edge>,         // petgraph directed graph
    pub node_map: IndexMap<NodeId, GraphNodeIdx>,  // O(1) string-to-index lookup
}
```

`Graph` is a thin wrapper around `petgraph::graph::DiGraph`. The `node_map`
uses `IndexMap<String, NodeIndex>` to provide O(1) string-to-index lookup
while preserving insertion order (insertion order matters for deterministic
fingerprinting when node IDs collide in sort order).

### Construction Methods

| Method | Purpose |
|---|---|
| `Graph::new(metadata)` | Empty graph with default policy |
| `.with_policy(policy)` | Builder: override execution policy |
| `.with_lenses(lenses)` | Builder: attach telemetry Lens registry |
| `.add_node(node)` | Add a node; errors on duplicate ID |
| `.add_edge(edge)` | Add an edge; errors if either endpoint is missing |
| `.get_node(id)` | O(1) node lookup by string ID |
| `.node_count()` / `.edge_count()` | Cardinality queries |
| `.validate_edges(registry)` | Type-schema compatibility check (see below) |

---

## Node

```rust
pub struct Node {
    pub id:              NodeId,          // unique string within the graph
    pub cell_type:       String,          // registry key (e.g. "task-executor", "sense")
    pub config:          toml::Value,     // per-node configuration passed to the cell factory
    pub inputs:          Vec<String>,     // named input slots
    pub outputs:         Vec<String>,     // named output slots
    pub execution_class: ExecutionClass,  // Workflow | Activity
}
```

`NodeId` is `type NodeId = String`. Node IDs must be unique within a graph;
`add_node` returns `Err(GraphError::DuplicateNode)` on collision.

### ExecutionClass

```rust
pub enum ExecutionClass {
    Workflow,  // Deterministic: re-derived on replay, not recorded
    Activity,  // Non-deterministic: recorded to JSONL, replayed on resume
}
```

The default is `Activity` because most graph nodes involve external computation
(LLM calls, tool use, gate execution). Only routing, scoring, and composition
nodes should be marked `Workflow`.

---

## Edge

```rust
pub struct Edge {
    pub from:      NodeId,
    pub to:        NodeId,
    pub condition: Option<EdgeCondition>,
}
```

Edges carry an optional `EdgeCondition`:

| Variant | Semantics |
|---|---|
| `Success` | Fires only if the source node succeeded |
| `Failure` | Fires only if the source node failed |
| `OutputEquals { key, value }` | Fires when a named output tag matches a literal value |
| `Always` | Unconditional dependency (always fires) |
| `None` (absent) | Treated as unconditional -- same as `Always` |

The `EdgeCondition` enum uses `#[serde(tag = "type", content = "value")]`
for clean TOML/JSON serialization.

---

## Advanced Condition System

Beyond `EdgeCondition`, the `condition.rs` module provides a richer `Condition`
type used by graph definitions loaded from TOML:

```rust
pub enum Condition {
    Always,               // unconditional (default)
    OnSuccess,            // source succeeded
    OnFailure,            // source failed
    When {                // field-level comparison
        field: String,    // dot-separated JSON pointer path
        op: CompareOp,    // Eq, Ne, Gt, Gte, Lt, Lte, Contains
        value: toml::Value,
    },
}
```

`When` conditions support:
- **Dot-separated field paths** into the output JSON data (e.g. `"result.status"`).
- **Numeric comparisons** (Gt, Gte, Lt, Lte) via `f64` coercion.
- **String substring** and **array containment** checks via `Contains`.
- Missing fields evaluate to `false` (fail-closed).

---

## Topological Sort

The topological sort is in `crates/roko-graph/src/topo.rs`.

### `topological_order(graph) -> Result<Vec<NodeId>, GraphError>`

Delegates to `petgraph::algo::toposort`. If the sort fails (not all nodes
visited), returns `GraphError::CycleDetected`.

### `topological_waves(graph) -> Result<Vec<Vec<NodeId>>, GraphError>`

Groups nodes into parallel execution waves using longest-path depth:

**Algorithm:**

1. Compute topological order via `topological_order()`.
2. For each node in order, compute `depth = max(depth of predecessors) + 1`.
   Root nodes (no incoming edges) have depth 0.
3. Group nodes by depth level. Each group is one wave.

```
Wave 0: [roots]              -- all zero-in-degree nodes
Wave 1: [depends on wave 0]  -- unblocked after wave 0 completes
Wave 2: [depends on wave 1]  -- unblocked after wave 1 completes
...
```

Within each wave, all nodes are independent -- no edges connect them -- so they
may execute concurrently, bounded by `policy.max_concurrent_nodes`. The engine no
longer schedules by wave (since `445a60d0d`): each node starts once its own
predecessors settle ([03-GRAPH.md](../../03-GRAPH.md), "Parallel Execution: the
Ready Queue").

### Helper Functions

| Function | Returns |
|---|---|
| `dependencies(graph, node_id)` | Immediate predecessors (incoming neighbors) |
| `dependents(graph, node_id)` | Immediate successors (outgoing neighbors) |
| `root_nodes(graph)` | All nodes with zero in-degree |
| `leaf_nodes(graph)` | All nodes with zero out-degree |
| `is_dag(graph)` | `true` if the graph has no cycles |

---

## Edge Condition Evaluation at Runtime

After a source node completes, its outgoing edges are evaluated by the engine:

1. `Success` edges fire only if `NodeOutputStatus::Success`.
2. `Failure` edges fire only if `NodeOutputStatus::Failed`.
3. `OutputEquals { key, value }` edges fire if the output Signal carries a
   matching tag.
4. `Always` edges fire unconditionally.
5. `None` (no condition) edges fire unconditionally.

A downstream node whose incoming conditional edges all evaluate to false
receives status `ConditionSkipped` -- a successful no-op, not a failure.

### NodeOutputStatus

```rust
pub enum NodeOutputStatus {
    Success,
    Failed,
    Skipped,
}
```

Each variant provides `is_success()`, `is_failed()`, and `is_skipped()` helpers.

### NodeOutput

```rust
pub struct NodeOutput {
    pub node_id:     NodeId,
    pub status:      NodeOutputStatus,
    pub data:        serde_json::Value,    // structured output
    pub error:       Option<String>,
    pub tokens_used: u64,
    pub cost_usd:    f64,
    pub duration:    Duration,             // serialized as milliseconds
}
```

Constructors: `NodeOutput::success(id, data)`, `NodeOutput::failed(id, err)`,
`NodeOutput::skipped(id, reason)`.

---

## Edge Type-Schema Validation

Before execution begins, `GraphEngine::validate_for_start()` calls
`graph.validate_edges(&registry)` to check type-schema compatibility.

**Rules:**

1. Both nodes must exist in the graph. Missing source or target produces an
   error with "not found in graph".
2. Both cell types must exist in the registry. Missing cell types produce an
   error with "not in registry".
3. If either cell returns `None` for its schema (untyped/Any), the edge is
   always valid.
4. If both schemas are present, `source.output_schema.is_compatible_with(target.input_schema)`
   determines validity.
5. All errors are collected (no fail-fast). The final error is
   `GraphError::EdgeValidationFailed { count, first_error }`.

Validation uses side-effect-free `CellDescriptor` introspection -- no Cell
instances are constructed.

---

## GraphPolicy

```rust
pub struct GraphPolicy {
    pub mode:                 GraphMode,        // OneShot | Hot
    pub failure_strategy:     FailureStrategy,  // FailFast | SkipFailed | Retry { max_retries }
    pub max_concurrent_nodes: usize,            // default: 4
    pub timeout_ms:           Option<u64>,
    pub hot:                  Option<HotPolicy>,
    pub capabilities:         Vec<Capability>,
}
```

### FailureStrategy

| Variant | Behavior |
|---|---|
| `FailFast` (default) | Abort the entire graph on the first node failure |
| `SkipFailed` | Skip failed nodes, continue executing independent successors |
| `Retry { max_retries }` | Retry failed nodes up to `max_retries` times |

### Capabilities

The `capabilities` field declares what privileged authorities the graph
requires. Missing capabilities deserialize to an empty vector. `ReadFs` and
`Bus` come from normal workspace config; `WriteFs`, `Network`, `Shell`, `Llm`,
and `Secrets` require both a graph declaration and a workspace grant.

---

## GraphError

All graph-layer errors derive from `GraphError`:

| Variant | When |
|---|---|
| `DuplicateNode(id)` | `add_node` with an existing ID |
| `NodeNotFound(id)` | `add_edge` references a missing node |
| `CycleDetected` | Topological sort fails |
| `LoaderError(msg)` | TOML parsing or schema error |
| `UnknownCellType(name)` | Cell type not in registry at execution time |
| `NodeFailed { node_id, reason }` | A node failed during execution |
| `InvalidEdge { node_id }` | Edge references a non-existent node |
| `BudgetExceeded { reason }` | Token, cost, or deadline limit breached |
| `ConditionError { from, to, reason }` | Condition expression failed to evaluate |
| `InvalidGraph { reason }` | Structural validation failure |
| `EdgeValidationFailed { count, first_error }` | Type-schema mismatches found |

---

## Example: Diamond DAG

```
     [A]
    /   \
  [B]   [C]
    \   /
     [D]

Wave 0: [A]
Wave 1: [B, C]     <-- parallel
Wave 2: [D]
```

Within wave 1, B and C execute concurrently up to `max_concurrent_nodes`.

---

## Verification

```bash
# Run topological sort, wave, and edge validation tests
cargo test -p roko-graph --lib topo
cargo test -p roko-graph --lib types::tests
cargo test -p roko-graph --lib condition::tests
```
