# Cell Trait Contract

> Full documentation of the Core Cell trait from `roko-core`, including every
> method, its default behavior, and its role in the execution pipeline.

**Source**: `crates/roko-core/src/cell.rs`

---

## 1. The Trait

```rust
#[async_trait]
pub trait Cell: Send + Sync + 'static {
    fn cell_id(&self) -> &str;
    fn cell_name(&self) -> &str;
    fn cell_version(&self) -> CellVersion { (0, 1, 0) }
    fn protocols(&self) -> Vec<ProtocolId> { Vec::new() }
    fn has_protocol(&self, id: ProtocolId) -> bool { ... }
    fn capabilities(&self) -> Capabilities { Capabilities::default() }
    fn estimated_cost(&self) -> Option<f64> { None }
    fn estimated_duration(&self) -> Option<Duration> { None }
    fn cost_estimate(&self) -> Option<CostEstimate> { ... }
    fn input_schema(&self) -> Option<&TypeSchema> { None }
    fn output_schema(&self) -> Option<&TypeSchema> { None }
    fn predict(&self, input: &[Signal]) -> Option<PredictionRecord> { None }
    fn correct(&self, prediction: &PredictionRecord, actual: &[Signal]) { }
    async fn execute(&self, input: Vec<Signal>, ctx: &CellContext) -> Result<Vec<Signal>>;
}
```

Object safety: `Cell` is `Send + Sync + 'static`. It can be stored as
`Arc<dyn Cell>` in the `CoreCellRegistry` or passed across async
boundaries. The trait requires `async_trait` for the `execute` method.

---

## 2. Method Reference

### 2.1 Identity Methods

**`cell_id(&self) -> &str`** (required)

Unique identifier for this Cell instance. Used as the key in
`CoreCellRegistry` and for logging/tracing. Must be stable across
the Cell's lifetime. Convention: lowercase with hyphens
(e.g., `"gate-compile"`, `"plan-composer"`).

**`cell_name(&self) -> &str`** (required)

Human-readable display name. Used in TUI, dashboard surfaces, and log
messages. Can differ from `cell_id` -- the ID is for machines, the name
is for humans.

**`cell_version(&self) -> CellVersion`** (default: `(0, 1, 0)`)

Semantic version tuple `(major, minor, patch)` of this Cell's
implementation. Used for schema migration and compatibility checks.
`CellVersion` is a type alias for `(u32, u32, u32)`.

### 2.2 Protocol and Capability Methods

**`protocols(&self) -> Vec<ProtocolId>`** (default: empty)

The set of protocol conformances this Cell declares. See
`protocol-ids-9.md` for the nine values. The return value tells the
engine and introspection tools what kind of work this Cell performs.

**`has_protocol(&self, id: ProtocolId) -> bool`** (default: `contains` check)

Convenience method that checks if `protocols()` contains the given ID.
Rarely overridden.

**`capabilities(&self) -> Capabilities`** (default: all-false)

Declares what runtime resources this Cell requires. The engine uses this
to verify sufficient capabilities before dispatch. Returning
`Capabilities::default()` (all false) is the safe sandbox default.

```rust
pub struct Capabilities {
    pub network: bool,       // outbound HTTP, DNS, WebSocket
    pub file_system: bool,   // filesystem reads/writes
    pub subprocess: bool,    // child process spawning
    pub llm_access: bool,    // LLM API invocation
    pub chain_access: bool,  // on-chain contract/RPC
    pub bus_publish: bool,   // Bus Pulse publication
    pub store_write: bool,   // durable Signal persistence
}
```

`Capabilities::is_subset_of(other)` checks that every capability
required by `self` is granted by `other`. `Capabilities::union(other)`
merges two sets.

### 2.3 Cost Methods

**`estimated_cost(&self) -> Option<f64>`** (default: `None`)

Estimated USD cost per invocation. `None` means unknown. Used for
budget enforcement and cost projection.

**`estimated_duration(&self) -> Option<Duration>`** (default: `None`)

Estimated wall-clock time per invocation. `None` means unknown. Used
for deadline enforcement.

**`cost_estimate(&self) -> Option<CostEstimate>`** (default: derived)

Rich cost estimate combining cost, duration, token counts, API calls,
and confidence. The default implementation derives from
`estimated_cost()` and `estimated_duration()`. Override to provide
full token counts and confidence values.

```rust
pub struct CostEstimate {
    pub usd_cost: f64,       // estimated USD
    pub token_input: u64,    // estimated input tokens
    pub token_output: u64,   // estimated output tokens
    pub api_calls: u32,      // outbound API calls
    pub wall_clock_ms: u64,  // estimated duration
    pub confidence: f64,     // [0.0, 1.0]; 1.0 = measured, 0.0 = guess
}
```

### 2.4 Schema Methods

**`input_schema(&self) -> Option<&TypeSchema>`** (default: `None`)

Describes the input type this Cell expects. `None` means untyped (accepts
any input). Used by `Graph::validate_edges()` to check type compatibility.

**`output_schema(&self) -> Option<&TypeSchema>`** (default: `None`)

Describes the output type this Cell produces. `None` means untyped.

`TypeSchema` has three variants:
- `Any` -- compatible with everything
- `OfKind(Kind)` -- accepts Signals of a specific Kind
- `JsonSchema(String)` -- exact schema string match

See `02-CELL.md` section 4.1 for compatibility rules.

### 2.5 Predict-Correct Methods

**`predict(&self, input: &[Signal]) -> Option<PredictionRecord>`** (default: `None`)

Called by the engine **before** `execute()`. Returns a prediction of the
expected outcome for online learning. `None` means this Cell does not
participate in the predict-correct lifecycle.

```rust
pub struct PredictionRecord {
    pub cell_id: String,
    pub predicted_outcome: serde_json::Value,
    pub confidence: f64,
    pub timestamp_ms: i64,
}
```

**`correct(&self, prediction: &PredictionRecord, actual: &[Signal])`** (default: no-op)

Called by the engine **after** `execute()` with the prediction and actual
output. Override to update internal models from prediction errors.

### 2.6 Execute Method

**`async fn execute(&self, input: Vec<Signal>, ctx: &CellContext) -> Result<Vec<Signal>>`**

The core computation. Default returns `Err(RokoError::Invalid(...))`.
Every non-stub Cell must override this.

The engine calls `execute()` in topological order within a Graph, feeding
upstream outputs as inputs to downstream cells. The return value becomes
the input to all downstream edges.

---

## 3. CellContext (Core)

Runtime context passed to `execute()`. Provides infrastructure handles
without cells needing their own:

```rust
pub struct CellContext {
    pub bus:              Arc<dyn BusErased>,
    pub store:            Arc<dyn Substrate>,
    pub cancel:           CancellationToken,
    pub trace_id:         Option<String>,
    pub run_id:           Option<String>,
    pub budget_remaining: Option<f64>,
    pub deadline_ms:      Option<i64>,
    pub parent_graph_id:  Option<String>,
    pub cell_id:          Option<String>,
}
```

| Field | Purpose |
|---|---|
| `bus` | Pub/sub transport for ephemeral Pulses (type-erased `BusErased`) |
| `store` | Durable storage for Signals (`Substrate` trait) |
| `cancel` | `tokio_util::CancellationToken` for cooperative shutdown |
| `trace_id` | Distributed tracing context identifier |
| `run_id` | Graph/Flow run identifier for grouping |
| `budget_remaining` | USD budget remaining (checked via `is_over_budget()`) |
| `deadline_ms` | Unix ms deadline (checked via `time_remaining_ms()`) |
| `parent_graph_id` | Enclosing Graph ID for nested execution tracing |
| `cell_id` | ID of the executing Cell (set by engine before dispatch) |

### Helper Methods

- `is_over_budget()` -- `true` when budget is `Some(x)` and `x <= 0.0`
- `time_remaining_ms()` -- milliseconds until deadline; negative if past
- Builder methods: `with_deadline()`, `with_parent_graph()`, `with_cell_id()`

---

## 4. CoreCellRegistry

Instance-based registry for live `Arc<dyn Cell>` management:

```rust
pub struct CoreCellRegistry {
    cells: HashMap<CellId, Arc<dyn Cell>>,
}
```

| Method | Signature | Purpose |
|---|---|---|
| `register` | `(&mut self, cell: Arc<dyn Cell>)` | Register by `cell_id()`; replaces existing |
| `get` | `(&self, id: &str) -> Option<Arc<dyn Cell>>` | Look up by ID |
| `list` | `(&self) -> Vec<(CellId, String)>` | All `(id, name)` pairs |
| `remove` | `(&mut self, id: &str) -> Option<Arc<dyn Cell>>` | Remove and return |
| `len` | `(&self) -> usize` | Count of registered cells |
| `is_empty` | `(&self) -> bool` | Whether registry is empty |

This registry manages already-constructed instances. It is distinct from
`roko-graph`'s `CellRegistry` which uses factory functions for
TOML-driven graph node instantiation.

---

## 5. Protocol Traits as Cell Supertraits

Each of the nine protocol traits (Store, Score, Verify, Route, Compose,
React, Observe, Connect, Trigger) requires `Cell` as a supertrait. This
means every protocol implementation automatically gets identity, cost
estimation, and protocol introspection.

```rust
// Example: the Score trait requires Cell
#[async_trait]
pub trait Score: Cell {
    async fn score(&self, signal: &Signal, ctx: &Context) -> Result<ScoreValue>;
}
```

When implementing a protocol trait, you implement both the protocol
methods and the Cell methods on the same struct.

---

## 6. Implementing a Cell

Minimal implementation:

```rust
struct MyCell;

#[async_trait]
impl Cell for MyCell {
    fn cell_id(&self) -> &str { "my-cell" }
    fn cell_name(&self) -> &str { "My Cell" }

    async fn execute(&self, input: Vec<Signal>, ctx: &CellContext)
        -> Result<Vec<Signal>>
    {
        // Transform input signals
        Ok(input)
    }
}
```

Production implementation with full metadata:

```rust
struct CompileGateCell { /* ... */ }

#[async_trait]
impl Cell for CompileGateCell {
    fn cell_id(&self) -> &str { "gate-compile" }
    fn cell_name(&self) -> &str { "Compile Gate" }
    fn cell_version(&self) -> CellVersion { (1, 0, 0) }
    fn protocols(&self) -> Vec<ProtocolId> { vec![ProtocolId::Verify] }
    fn capabilities(&self) -> Capabilities {
        Capabilities { subprocess: true, file_system: true, ..Default::default() }
    }
    fn estimated_duration(&self) -> Option<Duration> {
        Some(Duration::from_secs(30))
    }
    fn input_schema(&self) -> Option<&TypeSchema> {
        Some(&TypeSchema::OfKind(Kind::Task))
    }
    fn output_schema(&self) -> Option<&TypeSchema> {
        Some(&TypeSchema::OfKind(Kind::GateVerdict))
    }

    async fn execute(&self, input: Vec<Signal>, ctx: &CellContext)
        -> Result<Vec<Signal>>
    {
        // Run cargo check, produce GateVerdict signal
        todo!()
    }
}
```

---

## 7. Verification Commands

```bash
# Confirm Cell trait has expected methods
grep 'fn ' crates/roko-core/src/cell.rs | grep -v '//' | grep -v test

# Run Cell-related tests
cargo test -p roko-core cell -- --nocapture

# List all Cell implementations in the codebase
grep -rn 'impl Cell for' crates/ --include='*.rs' | grep -v target/ | grep -v test
```

---

## 8. Source Files

| File | Contents |
|---|---|
| `crates/roko-core/src/cell.rs` | Cell trait, CellContext, Capabilities, CostEstimate, PredictionRecord, TypeSchema, CoreCellRegistry |
| `crates/roko-core/src/traits.rs` | Protocol traits (Store, Score, Verify, Route, Compose, React, Observe, Connect, Trigger) |
| `crates/roko-graph/src/cell.rs` | Graph Cell trait (separate, lighter-weight; see `cell-context-dual-implementations.md`) |
| `crates/roko-graph/src/registry.rs` | CellRegistry (factory-based; see `cell-context-dual-implementations.md`) |
