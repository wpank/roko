# M038 — Define Graph TOML Schema

## Objective
Define the Graph TOML schema in roko-orchestrator as the universal composition primitive. A Graph is a directed graph of Nodes connected by typed Edges, with entry/exit points, input/output TypeSchema, and a GraphPolicy governing execution. This replaces ad-hoc plan formats with a single, validated schema that all execution flows through.

## Scope
- Crates: `roko-orchestrator`
- Files: `crates/roko-orchestrator/src/graph/schema.rs` (new), `crates/roko-orchestrator/src/graph/mod.rs` (new or update), `crates/roko-orchestrator/src/lib.rs`
- Phase ref: `tmp/unified-migration/03-PHASE-2-ENGINE.md` SS2.1
- Spec ref: `tmp/unified/03-GRAPH.md` SS1-4

## Steps
1. Check what already exists in the orchestrator graph module:
   ```bash
   grep -rn 'graph\|Graph\|NodeKind\|GraphPolicy' crates/roko-orchestrator/src/ --include='*.rs' | head -20
   ls crates/roko-orchestrator/src/graph/ 2>/dev/null || echo "no graph dir"
   ```

2. Create `crates/roko-orchestrator/src/graph/` directory and `mod.rs` if missing.

3. Define the core types in `crates/roko-orchestrator/src/graph/schema.rs`:
   - `GraphId` (content-addressed from name + version + nodes + edges)
   - `Graph { id, name, version, nodes, edges, entry, exits, input_schema, output_schema, policy, metadata }`
   - `Node { id, label, kind, failure_strategy, max_retries, timeout, execution_class }`
   - `NodeKind` enum: `Cell`, `SubGraph`, `Branch`, `FanOut`, `FanIn`, `Loop`, `HumanInput`, `Wait`, `Slot`, `Noop`
   - `Edge { from, to, mapping }`
   - `TypeMapping` for edge data mapping
   - `GraphPolicy { budget, deadline, failure_strategy, parallelism, hot, clock_binding }`
   - `GraphMetadata { author, description, tags, created_at, updated_at }`

4. All structs must derive `Debug, Clone, Serialize, Deserialize`. Use TOML-friendly types (String, u64, bool, Option).

5. Write a sample Graph TOML in a doc comment or test:
   ```toml
   [graph]
   name = "code-review"
   version = "0.1.0"

   [[nodes]]
   id = "analyze"
   label = "Analyze Code"
   kind = "Cell"

   [[nodes]]
   id = "report"
   label = "Generate Report"
   kind = "Cell"

   [[edges]]
   from = "analyze"
   to = "report"

   [policy]
   parallelism = 4
   ```

6. Add a `Graph::from_toml(s: &str) -> Result<Graph>` parsing method using `toml::from_str`.

7. Export from `crates/roko-orchestrator/src/graph/mod.rs` and `crates/roko-orchestrator/src/lib.rs`.

8. Add a test that parses the sample TOML and validates all fields are populated.

## Verification
```bash
cargo check -p roko-orchestrator
cargo clippy -p roko-orchestrator --no-deps -- -D warnings
cargo test -p roko-orchestrator -- graph
```

## What NOT to do
- Do NOT implement the executor -- that is M040
- Do NOT implement validation/type-checking of edges -- that is M039
- Do NOT implement failure strategies -- that is M041
- Do NOT add snapshot/resume -- that is M042
- Do NOT change the existing plan format or executor -- migration is M044/M045
- Do NOT add runtime types (Flow, RunId) -- those belong to the executor
