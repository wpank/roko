# M044 — `roko plan migrate` CLI Command

## Objective
Implement the `roko plan migrate` CLI command that converts existing `tasks.toml` plan files to the new Graph TOML format. This enables incremental migration: existing plans continue to work while new plans use the Graph format. The converter maps `[[task]]` entries to `[[nodes]]` with `kind = "Cell"`, maps `depends_on` to `[[edges]]`, and preserves all task metadata (prompt, agent config, Verify config).

## Scope
- Crates: `roko-cli`
- Files: `crates/roko-cli/src/migrate.rs` (new), `crates/roko-cli/src/main.rs` or `crates/roko-cli/src/cli.rs` (add subcommand)
- Phase ref: `tmp/unified-migration/03-PHASE-2-ENGINE.md` SS2.4
- Spec ref: `tmp/unified/21-ROADMAP.md` SS7.2 (Plan-to-Graph Migration)

## Steps
1. Read the current task/plan format:
   ```bash
   grep -rn 'struct Task\|TaskConfig\|PlanConfig' crates/roko-orchestrator/src/ --include='*.rs' | head -15
   grep -rn 'tasks.toml\|plan.*toml' crates/roko-cli/src/ --include='*.rs' | head -10
   ls .roko/plans/ 2>/dev/null || ls tmp/plans/ 2>/dev/null || echo "no plans dir found"
   ```

2. Read an existing plan file if available to understand the format:
   ```bash
   find . -name 'tasks.toml' -not -path './target/*' | head -3
   ```

3. Implement the migration logic in `crates/roko-cli/src/migrate.rs`:
   ```rust
   pub fn migrate_plan_to_graph(plan_path: &Path) -> Result<Graph> {
       // 1. Parse tasks.toml
       // 2. For each [[task]], create a Node { kind: Cell, ... }
       // 3. For each depends_on reference, create an Edge { from, to }
       // 4. Identify entry nodes (no dependencies) and exit nodes (nothing depends on them)
       // 5. Carry over metadata: prompt -> node metadata, agent_config -> node execution_class
       // 6. Carry over Verify config -> policy fields
       // 7. Generate GraphId from content
       // 8. Return the Graph
   }

   pub fn write_graph_toml(graph: &Graph, output_path: &Path) -> Result<()>;
   ```

4. Register the CLI subcommand:
   ```
   roko plan migrate <input-dir> [--output-dir <dir>] [--dry-run]
   ```
   - `--dry-run`: print the converted TOML without writing
   - Default output: same directory with `.graph.toml` suffix

5. Ensure the migration preserves:
   - Task names -> Node labels
   - Task prompts -> Node metadata
   - Task dependencies -> Edges
   - Agent config (model, temperature) -> Node execution_class
   - Verify config (gates, thresholds) -> GraphPolicy or per-node config

6. Write tests:
   - Migrate a simple 3-task plan with linear dependencies
   - Migrate a plan with parallel tasks (diamond dependency pattern)
   - Round-trip: migrate plan, verify all tasks appear as nodes, all dependencies appear as edges

## Verification
```bash
cargo check -p roko-cli
cargo clippy -p roko-cli --no-deps -- -D warnings
cargo test -p roko-cli -- migrate
# Manual test if a plan exists:
# cargo run -p roko-cli -- plan migrate tmp/plans/ --dry-run
```

## What NOT to do
- Do NOT delete or modify existing plan files -- migration creates new files alongside
- Do NOT break the existing `roko plan run` command -- both formats must work
- Do NOT implement auto-migration on plan load -- that is M045's responsibility
- Do NOT add a dependency on the Graph executor -- this is purely a format conversion
