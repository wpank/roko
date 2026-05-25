# M045 — Wire Graph Executor into `roko plan run`

## Objective
Wire the new Graph executor into the `roko plan run` command so it can execute both old-format tasks.toml plans and new Graph TOML plans. The command auto-detects the format and dispatches to the appropriate executor. This is the critical integration point that makes the Graph engine operational through the CLI.

## Scope
- Crates: `roko-cli`, `roko-orchestrator`
- Files: `crates/roko-cli/src/orchestrate.rs`, `crates/roko-orchestrator/src/graph/mod.rs`
- Phase ref: `tmp/unified-migration/03-PHASE-2-ENGINE.md` SS2.4
- Spec ref: (integration task -- no single spec section)

## Steps
1. Read the current `roko plan run` implementation:
   ```bash
   grep -rn 'plan.*run\|PlanRunner\|execute_plan' crates/roko-cli/src/orchestrate.rs | head -20
   grep -rn 'fn plan\|plan_run\|plan_execute' crates/roko-cli/src/ --include='*.rs' | head -15
   ```

2. Implement format detection:
   ```rust
   pub enum PlanFormat {
       TasksToml,  // Legacy: [[task]] entries
       GraphToml,  // New: [[nodes]] + [[edges]]
   }

   pub fn detect_plan_format(path: &Path) -> Result<PlanFormat> {
       // Check for [[nodes]] or [[graph]] section -> GraphToml
       // Check for [[task]] section -> TasksToml
       // Fallback: TasksToml for backward compatibility
   }
   ```

3. Add a dispatch layer in orchestrate.rs:
   ```rust
   pub async fn run_plan(path: &Path, config: &Config) -> Result<()> {
       match detect_plan_format(path)? {
           PlanFormat::TasksToml => run_legacy_plan(path, config).await,
           PlanFormat::GraphToml => run_graph_plan(path, config).await,
       }
   }
   ```

4. Implement `run_graph_plan`:
   - Load Graph via `GraphLoader::load_and_validate`
   - Create Engine with Bus, Store, budget from config
   - Start Flow via `engine.start(graph, input)`
   - Poll for completion or attach to Bus for live status
   - Print progress (node completions, failures, retries)
   - On completion: print summary (nodes completed, cost, duration)

5. Ensure the existing `--resume` flag works with the new executor by loading FlowSnapshot.

6. Wire into the CLI argument parser so `roko plan run` dispatches correctly.

7. Write an integration test:
   - Create a simple Graph TOML file in a temp directory
   - Run `roko plan run <temp-dir>` with the Graph executor
   - Verify it completes successfully

## Verification
```bash
cargo check -p roko-cli
cargo clippy -p roko-cli --no-deps -- -D warnings
cargo test -p roko-cli -- plan_run
# Manual smoke test:
# cargo run -p roko-cli -- plan run <path-to-graph-toml-dir>
```

## What NOT to do
- Do NOT remove or modify the legacy plan executor -- it must continue to work
- Do NOT change the CLI argument structure -- same `roko plan run <dir>` interface
- Do NOT couple the Graph executor to CLI-specific types -- keep it in roko-orchestrator
- Do NOT hard-code model or budget defaults -- read from roko.toml config
