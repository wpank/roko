# Phase 2: Graph + Engine — Build From Scratch

## Overview

This is the architectural center of v2. A Graph is a typed DAG of Cells. The Engine
interprets Graphs. Together they replace the procedural Runner v2 event loop with
declarative composition.

**Build this as new code.** Don't refactor Runner v2. Build alongside it, wire to a
new CLI path, then migrate.

## New Crate: `roko-graph`

### Graph — Typed DAG of Cells

```rust
/// A typed DAG of Cells connected by edges.
///
/// Graphs are themselves Cells (fractal composition): a Graph can contain
/// sub-Graphs as nodes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Graph {
    pub id: GraphId,
    pub name: String,
    pub version: (u32, u32, u32),
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
    pub entry: Vec<NodeId>,
    pub exits: Vec<NodeId>,
    pub input_schema: Option<TypeSchema>,
    pub output_schema: Option<TypeSchema>,
    pub policy: GraphPolicy,
}

pub type GraphId = String;
pub type NodeId = String;
```

### Node — What sits at each vertex

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    pub id: NodeId,
    pub cell_ref: CellRef,
    pub kind: NodeKind,
    pub execution_class: ExecutionClass,
}

/// How to find the Cell implementation for this node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CellRef {
    /// Look up by name in the CellRegistry.
    Named(String),
    /// Inline sub-graph.
    SubGraph(Box<Graph>),
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum NodeKind {
    Cell,
    SubGraph,
    Branch,
    FanOut,
    FanIn,
    Noop,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum ExecutionClass {
    /// Deterministic — can be replayed without re-executing.
    Workflow,
    /// Non-deterministic (LLM calls, network I/O) — must record output for replay.
    Activity,
}
```

### Edge — What connects nodes

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Edge {
    pub from: NodeId,
    pub to: NodeId,
    pub condition: Option<String>,    // Expression evaluated at runtime
    pub mapping: Option<Mapping>,     // Transform signals between nodes
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Mapping {
    /// Pass all signals through.
    Identity,
    /// Select signals by kind.
    FilterKind(Vec<Kind>),
    /// Select specific signal fields.
    Project(Vec<String>),
}
```

### GraphPolicy — Execution constraints

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphPolicy {
    /// Maximum budget (USD) for this graph execution.
    pub max_budget: Option<f64>,
    /// Maximum wall-clock time.
    pub deadline: Option<Duration>,
    /// Maximum concurrent nodes.
    pub max_parallelism: Option<usize>,
    /// What to do when a node fails.
    pub failure_strategy: FailureStrategy,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum FailureStrategy {
    /// Stop the entire graph on first failure.
    FailFast,
    /// Continue executing non-dependent nodes.
    ContinueOnFailure,
    /// Retry the failed node up to N times.
    Retry(usize),
}
```

### CellRegistry — How the Engine finds Cells

```rust
/// Registry of Cell implementations available to the Engine.
pub struct CellRegistry {
    cells: HashMap<String, Arc<dyn Cell>>,
}

impl CellRegistry {
    pub fn new() -> Self { Self { cells: HashMap::new() } }

    pub fn register(&mut self, name: &str, cell: Arc<dyn Cell>) {
        self.cells.insert(name.to_string(), cell);
    }

    pub fn get(&self, name: &str) -> Option<Arc<dyn Cell>> {
        self.cells.get(name).cloned()
    }
}
```

### TOML definition format

Graphs are defined in TOML (same as current plan tasks):

```toml
[graph]
name = "score-compose-verify"
version = "0.1.0"

[[graph.nodes]]
id = "scorer"
cell = "relevance-scorer"
kind = "Cell"

[[graph.nodes]]
id = "composer"
cell = "prompt-composer"
kind = "Cell"

[[graph.nodes]]
id = "gate"
cell = "compile-gate"
kind = "Cell"

[[graph.edges]]
from = "scorer"
to = "composer"

[[graph.edges]]
from = "composer"
to = "gate"

[graph.policy]
max_parallelism = 4
failure_strategy = "FailFast"
```

## New Crate: `roko-engine` (or module within `roko-graph`)

### Engine — Universal Graph interpreter

```rust
/// Interprets Graph definitions by executing Cells and routing Signals along edges.
pub struct Engine {
    pub registry: Arc<CellRegistry>,
    pub bus: Arc<dyn Bus>,
    pub store: Arc<dyn Store>,
    flows: DashMap<FlowId, FlowHandle>,
    cancel: CancellationToken,
}

impl Engine {
    /// Start executing a graph with initial input signals.
    pub async fn start(&self, graph: &Graph, input: Vec<Signal>) -> Result<FlowId> {
        let flow_id = FlowId::new();
        let ctx = CellContext::new(
            self.bus.clone(),
            self.store.clone(),
            self.cancel.child_token(),
        );

        let handle = tokio::spawn(self.execute_graph(graph.clone(), input, ctx));
        self.flows.insert(flow_id.clone(), FlowHandle { handle, cancel: ctx.cancel });
        Ok(flow_id)
    }

    /// Wait for a flow to complete and return its output.
    pub async fn await_flow(&self, flow_id: &FlowId) -> Result<Vec<Signal>> { ... }

    /// Cancel a running flow.
    pub async fn cancel(&self, flow_id: &FlowId) -> Result<()> { ... }

    /// Get the status of a flow.
    pub fn status(&self, flow_id: &FlowId) -> Option<FlowStatus> { ... }
}
```

### Graph execution algorithm

```rust
impl Engine {
    async fn execute_graph(
        &self,
        graph: Graph,
        input: Vec<Signal>,
        ctx: CellContext,
    ) -> Result<Vec<Signal>> {
        // 1. Topological sort of nodes
        let topo_order = topological_sort(&graph.nodes, &graph.edges)?;

        // 2. Initialize: entry nodes receive input signals
        let mut node_outputs: HashMap<NodeId, Vec<Signal>> = HashMap::new();
        for entry in &graph.entry {
            node_outputs.insert(entry.clone(), input.clone());
        }

        // 3. Execute nodes in topological order
        for node_id in &topo_order {
            let node = graph.node(node_id)?;

            // Collect inputs from all incoming edges
            let inputs = graph.incoming_edges(node_id)
                .iter()
                .filter(|edge| self.evaluate_condition(edge, &node_outputs))
                .flat_map(|edge| {
                    let signals = node_outputs.get(&edge.from).cloned().unwrap_or_default();
                    self.apply_mapping(&edge.mapping, signals)
                })
                .collect::<Vec<Signal>>();

            if inputs.is_empty() && !graph.entry.contains(node_id) {
                continue; // Skip nodes with no active inputs
            }

            // Resolve Cell from registry
            let cell = match &node.cell_ref {
                CellRef::Named(name) => self.registry.get(name)
                    .ok_or_else(|| anyhow!("Cell not found: {name}"))?,
                CellRef::SubGraph(sub) => {
                    // Recursive: execute sub-graph as a cell
                    let output = self.execute_graph(*sub.clone(), inputs.clone(), ctx.clone()).await?;
                    node_outputs.insert(node_id.clone(), output);
                    continue;
                }
            };

            // Execute the cell
            let output = cell.execute(inputs, &ctx).await?;
            node_outputs.insert(node_id.clone(), output);
        }

        // 4. Collect outputs from exit nodes
        let mut result = Vec::new();
        for exit in &graph.exits {
            if let Some(signals) = node_outputs.get(exit) {
                result.extend(signals.clone());
            }
        }

        Ok(result)
    }
}
```

### Flow — Runtime instance of a Graph

```rust
pub type FlowId = String;

pub struct FlowHandle {
    handle: JoinHandle<Result<Vec<Signal>>>,
    cancel: CancellationToken,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FlowStatus {
    Running,
    Completed { output_count: usize },
    Failed { error: String },
    Cancelled,
}
```

## Wiring Plan

### Step 1: New CLI command — `roko graph run <file.toml>`

This is the wire target. Before building Graph or Engine, add the CLI command:

```rust
// In commands/plan.rs or a new commands/graph.rs
async fn cmd_graph_run(path: &Path) -> Result<()> {
    let graph = roko_graph::load_graph(path)?;
    let registry = build_default_registry()?;
    let bus = Arc::new(PulseBus::new(4096));
    let store = Arc::new(MemorySubstrate::new());
    let engine = Engine::new(registry, bus, store);
    let flow_id = engine.start(&graph, vec![]).await?;
    let output = engine.await_flow(&flow_id).await?;
    println!("Output: {} signals", output.len());
    Ok(())
}
```

### Step 2: Build the simplest possible Graph + Engine

Start with:
- Linear pipelines only (no branches, no fan-out, no loops)
- CellRef::Named only (no sub-graphs)
- No parallelism (sequential execution)
- No snapshots/resume

This should be ~500 LOC total.

### Step 3: Register existing implementations as Cells

```rust
fn build_default_registry() -> Result<CellRegistry> {
    let mut reg = CellRegistry::new();
    reg.register("compile-gate", Arc::new(CompileGate::new()));
    reg.register("test-gate", Arc::new(TestGate::new()));
    reg.register("clippy-gate", Arc::new(ClippyGate::new()));
    // ... add more as needed
    Ok(reg)
}
```

### Step 4: Write a test graph and run it

```toml
# test-graph.toml
[graph]
name = "hello-graph"
version = "0.1.0"

[[graph.nodes]]
id = "gate"
cell = "compile-gate"
kind = "Cell"

[graph.policy]
failure_strategy = "FailFast"
```

```bash
cargo run -p roko-cli -- graph run test-graph.toml
```

### Step 5: Incrementally add features

Only add features when you have a graph that needs them:
1. **Fan-out/fan-in** — when you need parallel gate execution
2. **Branches** — when you need conditional paths (gate pass/fail)
3. **Sub-graphs** — when graphs get large enough to compose
4. **Snapshots/resume** — when graphs take long enough to need recovery
5. **Hot Graphs** — when you need persistent agent loops

## How Runner v2 Maps to Graphs

The current Runner v2 event loop does roughly:

```
for each task in topological order:
    1. spawn agent with system prompt (Compose)
    2. wait for agent output (Act)
    3. run gate pipeline (Verify)
    4. if gate fails: retry or replan
    5. if gate passes: persist + merge
```

As a Graph:

```toml
[[graph.nodes]]
id = "compose"
cell = "system-prompt-builder"
kind = "Cell"

[[graph.nodes]]
id = "act"
cell = "claude-agent"
kind = "Cell"
execution_class = "Activity"  # Non-deterministic

[[graph.nodes]]
id = "verify"
cell = "gate-pipeline"
kind = "Cell"

[[graph.nodes]]
id = "persist"
cell = "store-writer"
kind = "Cell"

[[graph.edges]]
from = "compose"
to = "act"

[[graph.edges]]
from = "act"
to = "verify"

[[graph.edges]]
from = "verify"
to = "persist"
condition = "verdict.hard_pass == true"
```

This mapping shows the Engine CAN replace Runner v2. But don't do it in Phase 2 —
do it in Phase 4 after the Engine is proven on simpler graphs.

## Files to Create

| File | What |
|------|------|
| `crates/roko-graph/Cargo.toml` | New crate manifest |
| `crates/roko-graph/src/lib.rs` | Graph, Node, Edge, GraphPolicy types |
| `crates/roko-graph/src/engine.rs` | Engine, FlowHandle, FlowStatus |
| `crates/roko-graph/src/registry.rs` | CellRegistry |
| `crates/roko-graph/src/loader.rs` | TOML → Graph parser |
| `crates/roko-cli/src/commands/graph.rs` | CLI command: `roko graph run` |

## What NOT to Do

- Don't implement Hot Graphs yet. That's Phase 3-4.
- Don't build expression evaluation for edge conditions yet. Start with always-true edges.
- Don't add workflow/activity split yet. Start with always-execute.
- Don't migrate Runner v2. Build alongside, prove it works, then migrate.
- Don't build a graph visual editor or graph-to-TOML generator.
