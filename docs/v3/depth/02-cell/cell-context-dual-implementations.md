# Cell Context: Dual Implementations

> Why Roko has two separate `Cell` traits and `CellContext` structs in two
> different crates, what each carries, and when to use which.

**Sources**: `crates/roko-core/src/cell.rs`, `crates/roko-graph/src/cell.rs`,
`crates/roko-graph/src/registry.rs`

---

## 1. The Split

Roko defines two `Cell` traits:

1. **Core Cell** in `roko-core::cell::Cell`
2. **Graph Cell** in `roko-graph::cell::Cell`

This is intentional, not accidental. The two traits share the same method
names, type schemas, protocol IDs, and predict/correct lifecycle, but
differ in their context payload, registry model, and intended consumers.

---

## 2. Why Two Traits?

The fundamental constraint is **crate dependency direction**:

```
roko-core  (kernel, depended on by everything)
    |
    v
roko-graph (engine, depends on roko-core)
```

`roko-core` cannot depend on `roko-graph` because `roko-graph` is
downstream. But the two crates need different things from their Cell
context:

- **roko-core** Cells need infrastructure handles (Bus, Store,
  CancellationToken) because they are used by protocol trait
  implementations that interact with the kernel's pub/sub and storage.

- **roko-graph** Cells need execution-scoped metadata (wave index,
  capability intersection, shared resources, atomic flags) because they
  are managed by the graph engine which provides these at execution time.

Putting both sets of fields into one trait would force `roko-core` to
depend on graph-engine types, creating a circular dependency.

---

## 3. Side-by-Side Comparison

### Trait Methods

| Method | Core Cell | Graph Cell |
|---|---|---|
| `cell_id()` | required | required |
| `cell_name()` | required | required |
| `cell_version()` | default `(0,1,0)` | default `(0,1,0)` |
| `protocols()` | default empty | default empty |
| `has_protocol()` | default contains | default contains |
| `capabilities()` | returns `Capabilities` | -- |
| `is_stub()` | -- | default `false` |
| `estimated_cost()` | default `None` | default `None` |
| `estimated_duration()` | default `None` | default `None` |
| `cost_estimate()` | default derived | -- |
| `input_schema()` | default `None` | default `None` |
| `output_schema()` | default `None` | default `None` |
| `predict()` | default `None` | default `None` |
| `calibration_error()` | -- | default `None` |
| `correct()` | default no-op | default no-op |
| `execute()` | default error | required |

Key differences:
- **`capabilities()`**: Core only. Core Cells declare their own
  capabilities. Graph Cells receive capabilities through context injection.
- **`is_stub()`**: Graph only. The graph engine uses this to tag stub nodes
  and warn operators. Core does not track stubs.
- **`calibration_error()`**: Graph only. Computes a normalized error
  in `[0.0, 1.0]` for calibration tracking. Core only has `correct()`.
- **`cost_estimate()`**: Core only. The richer cost estimate struct
  with token counts and confidence.
- **`execute()` default**: Core returns `Err(RokoError::Invalid(...))`.
  Graph has no default -- the method is required.

### CellContext Fields

| Field | Core | Graph | Purpose |
|---|---|---|---|
| `bus` | `Arc<dyn BusErased>` | -- | Pub/sub for Pulses |
| `store` | `Arc<dyn Substrate>` | -- | Durable Signal storage |
| `cancel` | `CancellationToken` | -- | Cooperative shutdown (tokio) |
| `cancel_flag` | -- | `Option<Arc<AtomicBool>>` | Cooperative shutdown (atomic) |
| `pause_flag` | -- | `Option<Arc<AtomicBool>>` | Cooperative pause |
| `trace_id` | `Option<String>` | `Option<String>` | Observability |
| `run_id` | `Option<String>` | `Option<String>` | Graph run identifier |
| `budget_remaining` | `Option<f64>` | `Option<f64>` | USD budget |
| `deadline_ms` | `Option<i64>` | `Option<i64>` | Unix ms deadline |
| `parent_graph_id` | `Option<String>` | `Option<String>` | Enclosing Graph |
| `cell_id` | `Option<String>` | `Option<String>` | Executing Cell ID |
| `capabilities` | -- | `Option<CapabilitySet>` | Effective capability intersection |
| `wave_index` | -- | `Option<u32>` | Current wave in parallel execution |
| `total_waves` | -- | `Option<u32>` | Total wave count |
| `resources` | -- | `CellResources` | Shared service handles |

The Core context carries **infrastructure handles** (Bus, Store,
CancellationToken). These are heavyweight resources that protocol
trait implementations need.

The Graph context carries **execution metadata** (capabilities, waves,
resources, atomic flags). These are lightweight values set by the graph
engine at dispatch time.

---

## 4. Registry Models

### CoreCellRegistry (roko-core)

Instance-based. Stores live `Arc<dyn Cell>` instances indexed by
`CellId` (a `String`):

```rust
pub struct CoreCellRegistry {
    cells: HashMap<CellId, Arc<dyn Cell>>,
}
```

You register already-constructed cells:
```rust
registry.register(Arc::new(my_cell));
let cell = registry.get("my-cell-id").unwrap();
```

### CellRegistry (roko-graph)

Factory-based. Maps cell type name strings to `CellFactory` closures
that produce `Box<dyn Cell>` from TOML config:

```rust
pub type CellFactory = Box<dyn Fn(toml::Value) -> Box<dyn Cell> + Send + Sync>;

pub struct CellRegistry {
    entries: HashMap<String, CellEntry>,  // descriptor + factory
}
```

You register a factory function and a descriptor:
```rust
registry.register_with_descriptor(
    "gate.compile",
    CellDescriptor::new("gate.compile", (1, 0, 0), input_schema, output_schema),
    |config| Box::new(CompileGateCell::from_config(config)),
);
```

When the engine encounters `cell_type = "gate.compile"` in a TOML
node definition, it looks up the factory and calls it with the node
config to instantiate a Cell.

### CellDescriptor

Side-effect-free metadata for graph cell types. Used for edge
validation without constructing live cells:

```rust
pub struct CellDescriptor {
    pub id: String,
    pub version: CellVersion,
    pub input_schema: Option<TypeSchema>,
    pub output_schema: Option<TypeSchema>,
    pub is_stub: bool,
    pub protocols: Vec<ProtocolId>,
    pub is_predictive: bool,
    pub display_name: Option<String>,
}
```

Production registrations must provide a `CellDescriptor` with typed
schemas. Test-only registrations can use `CellDescriptor::test_stub()`.
Production starts reject graphs containing stub descriptors.

---

## 5. When to Use Which

### Use Core Cell (`roko_core::cell::Cell`) when:

- Implementing a protocol trait (Store, Score, Verify, Route, Compose,
  React, Observe, Connect, Trigger) -- because the protocol traits
  require `Cell` as a supertrait.
- Building infrastructure components that need Bus/Store access through
  their context.
- Registering in `CoreCellRegistry` for kernel-level dispatch.

### Use Graph Cell (`roko_graph::cell::Cell`) when:

- Implementing a graph node that will be registered in `CellRegistry`
  via a factory function.
- Building cells that are instantiated from TOML config
  (`cell_type = "my.cell"` in a graph definition).
- Needing wave-aware execution context (wave_index, total_waves).
- Needing capability injection from the engine.
- Needing cooperative pause/cancel via atomic flags.

### General rule:

If the Cell lives in the kernel or a protocol crate -> Core Cell.
If the Cell lives in the graph engine or is defined by TOML config -> Graph Cell.

---

## 6. Shared Types

Both traits share types from `roko-core`:

| Type | Crate | Used by |
|---|---|---|
| `Signal` | `roko-core` | Input and output of `execute()` |
| `ProtocolId` | `roko-core` | `protocols()` return type |
| `TypeSchema` | `roko-core` | `input_schema()`, `output_schema()` |
| `PredictionRecord` | `roko-core` | `predict()`, `correct()`, `calibration_error()` |
| `CellVersion` | both (type alias) | `cell_version()` return type |

This sharing ensures that Signals, schemas, and protocol IDs are
consistent across both Cell trait implementations.

---

## 7. CellResources (Graph Only)

Shared service handles injected by the graph engine:

```rust
#[derive(Clone, Default)]
pub struct CellResources {
    pub gates: Option<Arc<dyn SharedGateEvaluator>>,
}
```

Currently only carries the shared gate evaluator. This is how
`PlanGateCell` accesses the gate pipeline without owning the evaluator.
Additional services (cost tracker, telemetry sink, etc.) can be added
as fields without changing the Cell trait.

---

## 8. Cooperative Control (Graph Only)

The Graph CellContext provides atomic flags for cooperative lifecycle
control:

```rust
// Cancellation
pub cancel_flag: Option<Arc<AtomicBool>>,

// Pause/suspend
pub pause_flag: Option<Arc<AtomicBool>>,
```

Helper methods:
- `ctx.is_cancelled()` -- check if cancellation was requested
- `ctx.is_paused()` -- check if the executor is paused

Cells should check these periodically during long-running operations.
The graph engine sets these flags when `plan pause` or `plan cancel`
commands are received.

The Core CellContext uses `tokio_util::CancellationToken` instead,
which provides async-aware cancellation via `.cancelled().await`.

---

## 9. Verification Commands

```bash
# Confirm exactly two Cell trait definitions
grep -rn 'pub trait Cell' crates/ --include='*.rs' | grep -v target/ | grep -v test
# Expected: crates/roko-core/src/cell.rs and crates/roko-graph/src/cell.rs

# Compare CellContext fields
grep 'pub ' crates/roko-core/src/cell.rs | grep -A1 'CellContext'
grep 'pub ' crates/roko-graph/src/cell.rs | grep -A1 'CellContext'

# Run tests for both
cargo test -p roko-core cell -- --nocapture
cargo test -p roko-graph cell -- --nocapture
```

---

## 10. Source Files

| File | Contents |
|---|---|
| `crates/roko-core/src/cell.rs` | Core Cell trait, Core CellContext, Capabilities, CoreCellRegistry |
| `crates/roko-graph/src/cell.rs` | Graph Cell trait, Graph CellContext, CellResources |
| `crates/roko-graph/src/registry.rs` | CellRegistry, CellDescriptor, CellFactory |
| `crates/roko-core/src/traits.rs` | Protocol traits requiring Core Cell as supertrait |
