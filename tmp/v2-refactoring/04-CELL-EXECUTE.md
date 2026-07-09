# Phase 1A: Cell Gets execute() — The Universal Computation Interface

## What Changes

The `Cell` trait in `roko-core/src/cell.rs` currently provides only identity and metadata:

```rust
pub trait Cell: Send + Sync + 'static {
    fn cell_id(&self) -> &str;
    fn cell_name(&self) -> &str;
    fn cell_version(&self) -> CellVersion { (0, 1, 0) }
    fn protocols(&self) -> &[&str] { &[] }
    fn estimated_cost(&self) -> Option<f64> { None }
    fn estimated_duration(&self) -> Option<Duration> { None }
}
```

V2 adds the execution interface:

```rust
pub trait Cell: Send + Sync + 'static {
    // --- existing ---
    fn cell_id(&self) -> &str;
    fn cell_name(&self) -> &str;
    fn cell_version(&self) -> CellVersion { (0, 1, 0) }
    fn protocols(&self) -> &[&str] { &[] }
    fn estimated_cost(&self) -> Option<f64> { None }
    fn estimated_duration(&self) -> Option<Duration> { None }

    // --- new in v2 ---
    fn input_schema(&self) -> Option<&TypeSchema> { None }
    fn output_schema(&self) -> Option<&TypeSchema> { None }

    /// Execute this cell. Default returns an error — override in implementations.
    async fn execute(
        &self,
        input: Vec<Signal>,
        ctx: &CellContext,
    ) -> Result<Vec<Signal>> {
        let _ = (input, ctx);
        Err(Error::msg(format!("{}: execute() not implemented", self.cell_name())))
    }
}
```

## CellContext

Minimal first version — grow as needed:

```rust
pub struct CellContext {
    /// Pub/sub transport for ephemeral Pulses.
    pub bus: Arc<dyn Bus>,
    /// Durable storage for Signals.
    pub store: Arc<dyn Store>,
    /// Cancellation token for cooperative shutdown.
    pub cancel: CancellationToken,
    /// Trace context for observability.
    pub trace_id: Option<String>,
    /// Run identifier (if executing within a Graph/Flow).
    pub run_id: Option<String>,
    /// Remaining budget for this execution.
    pub budget_remaining: Option<f64>,
}

impl CellContext {
    pub fn new(bus: Arc<dyn Bus>, store: Arc<dyn Store>, cancel: CancellationToken) -> Self {
        Self {
            bus,
            store,
            cancel,
            trace_id: None,
            run_id: None,
            budget_remaining: None,
        }
    }
}
```

## TypeSchema

Minimal version for edge validation:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TypeSchema {
    /// Accepts any input.
    Any,
    /// Accepts signals of a specific kind.
    OfKind(Kind),
    /// Accepts signals matching a JSON schema.
    JsonSchema(String),
}

impl TypeSchema {
    /// Check if `self` is compatible as input to a cell expecting `target`.
    pub fn is_compatible_with(&self, target: &TypeSchema) -> bool {
        match (self, target) {
            (_, TypeSchema::Any) => true,
            (TypeSchema::Any, _) => true,
            (TypeSchema::OfKind(a), TypeSchema::OfKind(b)) => a == b,
            _ => false, // Conservative: reject unknown combinations
        }
    }
}
```

## What Gets Wired

### Existing protocol traits become Cell implementations

Every existing trait (Store, Score, Verify, Route, Compose, React) already requires
`Cell` as supertrait. After this change, they can ALSO implement `execute()`:

```rust
// Example: a gate that implements both Verify and Cell::execute
impl Cell for CompileGate {
    fn cell_id(&self) -> &str { "compile-gate" }
    fn cell_name(&self) -> &str { "Compile Gate" }
    fn protocols(&self) -> &[&str] { &["Verify"] }

    async fn execute(&self, input: Vec<Signal>, ctx: &CellContext) -> Result<Vec<Signal>> {
        // Delegate to the Verify protocol
        let verdict = self.verify(&input[0], ctx).await?;
        // Wrap verdict as a Signal
        Ok(vec![Signal::from_verdict(verdict)])
    }
}
```

This is **additive** — existing code that calls `gate.verify()` still works. The new
`execute()` is an additional entry point for when the gate is used as a Cell in a Graph.

### Wire target for Phase 1A

No new CLI command needed. The change is:
1. Cell trait gains `execute()` with a default impl (backwards compat)
2. Existing implementations optionally override `execute()`
3. Future Graph + Engine will call `execute()` to run any Cell

### Verification

```bash
cargo build --workspace  # Compiles with default impl
cargo test --workspace   # Existing tests pass (nothing calls execute() yet)
```

## Files to Change

| File | Change |
|------|--------|
| `crates/roko-core/src/cell.rs` | Add execute(), CellContext, TypeSchema |
| `crates/roko-core/src/lib.rs` | Export CellContext, TypeSchema |

That's it for the trait. Individual Cell implementations get execute() in later phases
as they're wired into Graphs.

## What NOT to Do

- Don't implement execute() on all existing types immediately. That's work without a
  wire target. Only implement when a Graph actually needs to call a particular Cell.
- Don't add fields to CellContext speculatively. Start minimal, add when a Cell
  actually needs them.
- Don't build CellRegistry yet. That comes with Graph + Engine in Phase 2.
