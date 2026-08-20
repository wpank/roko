# 82 — Graph Stub Cell Warnings and Execution Transparency

**Priority**: P2 — users can't tell if graph execution did real work or was a pass-through no-op
**Size**: S (half day to 1 day)
**Crates**: `crates/roko-graph` (paths: `src/cell.rs`, `src/cells/stubs.rs`, `src/engine.rs`), `crates/roko-cli` (path: `src/commands/graph.rs`)
**Depends on**: None

---

## Background

The `roko graph run` command runs a graph and prints results like:

```
  [     ok] sense (signal-reader) (66µs)
  [     ok] assess (relevance-scorer) (42µs)
```

What's not visible: `signal-reader` and `relevance-scorer` both resolve to `PassthroughCell` — a stub implementation that returns its input signals unchanged and does nothing else. A user looking at this output has no way to know whether real computation happened or whether the graph was a pass-through exercise. Sub-millisecond execution times and `ok` statuses look the same whether cells are real or stubs.

This is a trust problem. Developers testing their graph definitions can't distinguish "my graph ran correctly" from "my graph ran but all cells were stubs." The fix involves: (1) adding an `is_stub()` method to the `Cell` trait, (2) warning in `GraphOutput::summary()` when stub cells were executed, and (3) annotating `roko graph show` output with resolved cell types.

## Current State

1. **`PassthroughCell` is the backend for 7 cell types** — `/Users/will/dev/nunchi/roko/roko/crates/roko-graph/src/cells/stubs.rs` line 69 defines `COGNITIVE_LOOP_STUBS: &[&str]` with entries: `signal-reader`, `relevance-scorer`, `system-prompt-builder`, `claude-agent`, `gate-pipeline`, `store-writer`, `event-publisher`. In `default_registry()` at `/Users/will/dev/nunchi/roko/roko/crates/roko-graph/src/engine.rs` lines 2203-2208, each is registered as a `PassthroughCell` factory. `PassthroughCell::execute()` at line 56 of `stubs.rs` logs a `tracing::info!` message and returns input unchanged.

2. **`NoopCell` is another stub variant** — `/Users/will/dev/nunchi/roko/roko/crates/roko-graph/src/engine.rs` line 2219 defines `NoopCell` (private struct, same file). It's registered as `"noop"` at line 2167. `NoopCell::execute()` at line 2260 returns input unchanged with no logging.

3. **`Cell` trait has no stub indicator** — `/Users/will/dev/nunchi/roko/roko/crates/roko-graph/src/cell.rs` defines the `Cell` trait at line 143. It has 12 methods (lines 145-211) but no method to indicate whether the cell is a placeholder. There is no way to ask an instantiated `Box<dyn Cell>` whether it performs real work.

4. **`GraphOutput::summary()` doesn't warn about stubs** — `/Users/will/dev/nunchi/roko/roko/crates/roko-graph/src/engine.rs` lines 215-244 print per-node results. The struct `NodeResult` (defined near line 200) has fields `node_id`, `cell_type`, `status`, `duration`, and `error` — but no `is_stub` field. The summary function has no stub-count path.

5. **`cmd_graph_show()` doesn't resolve cell types to implementations** — `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/commands/graph.rs` lines 194-239 print node IDs and cell types from the raw graph TOML (line 212: `println!("  {node_id}  [cell_type: {}]", node.cell_type)`). The function loads the graph TOML but never consults the registry to see what implementation each cell type maps to.

6. **`score-compose.toml` has a stale header comment** — `/Users/will/dev/nunchi/roko/roko/examples/graphs/score-compose.toml` says cells are "backed by no-op implementations" but `score`, `compose`, and `act` are registered in `default_registry()` as real cognitive cells: `AssessCell`, `CognitiveComposeCell`, and `ActCell` from `crates/roko-graph/src/cells/cognitive.rs`.

7. **`cognitive-loop.toml` has no explanation that its cells are stubs** — `/Users/will/dev/nunchi/roko/roko/examples/graphs/cognitive-loop.toml` uses all 7 stub cell types with no header explaining this.

## Implementation Plan

### Step 1 — Add `is_stub()` to the `Cell` trait

File: `/Users/will/dev/nunchi/roko/roko/crates/roko-graph/src/cell.rs`

Add a default-false method to the trait at line 143. Insert after `cell_version()` (line 150-153):

```rust
/// Returns `true` if this cell is a placeholder that performs no real work.
///
/// Stub cells pass input signals through unchanged. When a graph contains
/// stub cells, `GraphOutput::summary()` emits a warning. Override to return
/// `true` in `PassthroughCell` and `NoopCell`.
fn is_stub(&self) -> bool {
    false
}
```

### Step 2 — Override `is_stub()` in `PassthroughCell`

File: `/Users/will/dev/nunchi/roko/roko/crates/roko-graph/src/cells/stubs.rs`

In the `impl Cell for PassthroughCell` block (line 31), add:

```rust
fn is_stub(&self) -> bool {
    true
}
```

### Step 3 — Override `is_stub()` in `NoopCell`

File: `/Users/will/dev/nunchi/roko/roko/crates/roko-graph/src/engine.rs`

In the `impl Cell for NoopCell` block (line 2241), add:

```rust
fn is_stub(&self) -> bool {
    true
}
```

### Step 4 — Add `is_stub` to `NodeResult` and populate it in the engine

File: `/Users/will/dev/nunchi/roko/roko/crates/roko-graph/src/engine.rs`

Find the `NodeResult` struct (near line 200) and add a field:

```rust
pub struct NodeResult {
    pub node_id: NodeId,
    pub cell_type: String,
    pub status: NodeStatus,
    pub duration: Duration,
    pub error: Option<String>,
    /// True if the resolved cell is a stub (PassthroughCell, NoopCell).
    pub is_stub: bool,
}
```

In the execution loop where `NodeResult` is constructed, set `is_stub: cell.is_stub()` after instantiating the cell from the registry.

### Step 5 — Add stub warning to `GraphOutput::summary()`

File: `/Users/will/dev/nunchi/roko/roko/crates/roko-graph/src/engine.rs`

In `GraphOutput::summary()` (lines 215-244), after the per-node loop, count stub nodes and emit a warning:

```rust
let stub_count = self.node_results.iter().filter(|r| r.is_stub).count();
if stub_count > 0 {
    let _ = writeln!(s);
    let _ = writeln!(
        s,
        "  warning: {stub_count} of {} cell(s) are stubs — no real computation was performed.",
        self.node_results.len()
    );
    let _ = writeln!(
        s,
        "  Use `roko graph show <path>` to see resolved cell types."
    );
}
```

### Step 6 — Annotate `roko graph show` with resolved cell types

File: `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/commands/graph.rs`

In `cmd_graph_show()` (lines 194-239), after loading the graph, build the default registry and attempt to resolve each node's cell type. The function currently prints node info at line 212. Extend it:

```rust
use roko_graph::engine::default_registry;

let registry = default_registry();
// ...
for (node_id, idx) in &graph.node_map {
    let node = &graph.inner[*idx];
    let stub_label = match registry.create(&node.cell_type, toml::Value::Table(Default::default())) {
        Ok(cell) if cell.is_stub() => format!("  (stub: {})", cell.cell_name()),
        Ok(_) => String::new(),
        Err(_) => "  (unregistered)".to_string(),
    };
    println!("  {node_id}  [cell_type: {}]{}", node.cell_type, stub_label);
    // ...existing inputs/outputs print...
}
```

This requires checking how `CellRegistry::create()` works in `/Users/will/dev/nunchi/roko/roko/crates/roko-graph/src/registry.rs`. The registry returns `Result<Box<dyn Cell>, RegistryError>`.

### Step 7 — Fix stale comments in example TOML files

File: `/Users/will/dev/nunchi/roko/roko/examples/graphs/score-compose.toml`

Update the header comment (first 5-8 lines) to reflect that the cells are now real implementations:

```toml
# score-compose.toml — demonstrates the score → compose → act pipeline.
#
# These cells resolve to real cognitive Cell implementations
# (AssessCell, CognitiveComposeCell, ActCell) in the default registry.
```

File: `/Users/will/dev/nunchi/roko/roko/examples/graphs/cognitive-loop.toml`

Add a header comment explaining stub status:

```toml
# cognitive-loop.toml — demonstrates graph structure and edge routing.
#
# NOTE: The cell types in this graph (signal-reader, relevance-scorer, etc.)
# resolve to PassthroughCell stubs in the default registry. This example
# shows how a cognitive loop is wired, not how real cells execute.
# See single-gate.toml for a graph with real gate cell execution.
```

## Acceptance Criteria

1. The `Cell` trait has an `is_stub() -> bool` method with a default `false` implementation.
2. `PassthroughCell` and `NoopCell` both return `true` from `is_stub()`.
3. `roko graph run examples/graphs/cognitive-loop.toml` prints a warning that all 7 cells are stubs.
4. `roko graph run examples/graphs/single-gate.toml` does NOT print a stub warning (its cell is a real `ShellCell` or equivalent).
5. `roko graph show examples/graphs/cognitive-loop.toml` annotates each node with `(stub: PassthroughCell)`.
6. `roko graph show examples/graphs/score-compose.toml` annotates no nodes as stubs.
7. The `score-compose.toml` header no longer says "no-op implementations."
8. The `cognitive-loop.toml` header explains that cells are stubs.

## Verification Checklist

- [ ] `cargo test -p roko-graph` — all existing tests pass after adding `is_stub` field to `NodeResult`
- [ ] `roko graph run examples/graphs/cognitive-loop.toml` — stub warning appears in output
- [ ] `roko graph run examples/graphs/observed-cost.toml` — stub warning appears (noop cell)
- [ ] `roko graph run examples/graphs/score-compose.toml` — no stub warning (real cognitive cells)
- [ ] `roko graph show examples/graphs/cognitive-loop.toml` — each node annotated with `(stub: PassthroughCell)`
- [ ] `roko graph show examples/graphs/single-gate.toml` — no stub annotation

## Files to Modify

| File | Change |
|---|---|
| `/Users/will/dev/nunchi/roko/roko/crates/roko-graph/src/cell.rs` | Add `fn is_stub(&self) -> bool { false }` to `Cell` trait (after line 153) |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-graph/src/cells/stubs.rs` | Override `is_stub()` to return `true` in `impl Cell for PassthroughCell` |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-graph/src/engine.rs` | Override `is_stub()` to return `true` in `impl Cell for NoopCell`; add `is_stub: bool` field to `NodeResult`; populate it in execution loop; add stub-count warning in `GraphOutput::summary()` |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/commands/graph.rs` | In `cmd_graph_show()`, resolve cell types via `default_registry()` and annotate stubs in node output |
| `/Users/will/dev/nunchi/roko/roko/examples/graphs/score-compose.toml` | Fix stale "no-op" comment in header |
| `/Users/will/dev/nunchi/roko/roko/examples/graphs/cognitive-loop.toml` | Add header comment explaining cells are stubs |
