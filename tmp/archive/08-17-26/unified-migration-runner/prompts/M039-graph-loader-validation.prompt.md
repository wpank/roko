# M039 — Graph Loader with TypeSchema Validation

## Objective
Implement the Graph loader that reads Graph TOML files from disk and validates them before execution. Validation includes: TypeSchema compatibility on all edges (output of source node must match input of target node), capability intersection with Graph allow-list, cycle detection (classifying intentional loops vs errors), and structural integrity (all referenced NodeIds exist, entry/exit nodes are valid).

## Scope
- Crates: `roko-orchestrator`
- Files: `crates/roko-orchestrator/src/graph/loader.rs` (new), `crates/roko-orchestrator/src/graph/mod.rs`
- Phase ref: `tmp/unified-migration/03-PHASE-2-ENGINE.md` SS2.1
- Spec ref: `tmp/unified/03-GRAPH.md` SS5 (Validation), `tmp/unified/05-EXECUTION-ENGINE.md` SS3

## Steps
1. Check what TypeSchema looks like in the codebase:
   ```bash
   grep -rn 'TypeSchema' crates/roko-core/src/ --include='*.rs' | head -10
   grep -rn 'TypeSchema' crates/roko-orchestrator/src/ --include='*.rs' | head -10
   ```

2. Read the Graph schema types from M038:
   ```bash
   cat crates/roko-orchestrator/src/graph/schema.rs
   ```

3. Implement `GraphLoader` in `crates/roko-orchestrator/src/graph/loader.rs`:
   ```rust
   pub struct GraphLoader {
       search_paths: Vec<PathBuf>,
   }

   impl GraphLoader {
       pub fn new(search_paths: Vec<PathBuf>) -> Self;
       pub fn load(&self, path: &Path) -> Result<Graph, GraphLoadError>;
       pub fn load_and_validate(&self, path: &Path) -> Result<Graph, GraphLoadError>;
   }
   ```

4. Implement validation as a separate `validate(graph: &Graph) -> Result<(), Vec<GraphValidationError>>`:
   - **Structural**: all edge `from`/`to` NodeIds exist in nodes list; entry nodes exist; exit nodes exist; no orphan nodes (unreachable from entry).
   - **Type compatibility**: for each edge, source node's output TypeSchema is compatible with target node's input TypeSchema. If TypeSchema is `None` (untyped), skip check.
   - **Cycle detection**: use Tarjan's SCC algorithm. Nodes with `kind = Loop` may participate in cycles; other cycles are errors.
   - **Capability check**: if Graph has an allow-list, each Cell node's declared capabilities must be a subset.

5. Define `GraphLoadError` and `GraphValidationError` enums with clear messages:
   ```rust
   pub enum GraphValidationError {
       MissingNode { edge_index: usize, node_id: String },
       OrphanNode { node_id: String },
       TypeMismatch { edge: String, source_type: String, target_type: String },
       UnintentionalCycle { nodes: Vec<String> },
       CapabilityViolation { node_id: String, capability: String },
   }
   ```

6. Export from graph/mod.rs.

7. Write tests:
   - Valid Graph loads successfully
   - Edge referencing nonexistent node produces `MissingNode` error
   - Type-incompatible edge produces `TypeMismatch` error
   - Unintentional cycle produces `UnintentionalCycle` error
   - Graph with `Loop` node in cycle passes validation

## Verification
```bash
cargo check -p roko-orchestrator
cargo clippy -p roko-orchestrator --no-deps -- -D warnings
cargo test -p roko-orchestrator -- graph::loader
cargo test -p roko-orchestrator -- validation
```

## What NOT to do
- Do NOT implement Graph execution -- that is M040
- Do NOT load Cells from registry here -- loader validates schema only, not runtime Cell availability
- Do NOT modify the Graph schema defined in M038
- Do NOT add SubGraph resolution (recursive loading) yet -- keep it single-level for now
