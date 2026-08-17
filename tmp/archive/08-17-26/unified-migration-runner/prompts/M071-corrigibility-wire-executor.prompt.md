# M071 — Wire Corrigibility into Graph Executor

## Objective
Wire the 5-head corrigibility chain (M069) into the Graph executor so that every Cell execution passes through a mandatory pre/post corrigibility check. The chain cannot be removed by Graph authors -- it is enforced by the execution engine itself. This ensures that all automated actions are subject to safety verification regardless of the Graph definition.

## Scope
- Crates: `roko-orchestrator`
- Files: `crates/roko-orchestrator/src/graph/safety.rs` (new), `crates/roko-orchestrator/src/graph/executor.rs` (modify)
- Phase ref: `tmp/unified-migration/04-PHASE-3-ECONOMY.md` SS3.2
- Spec ref: `tmp/unified/17-SECURITY-MODEL.md` SS4.3

## Steps
1. Read the Graph executor code:
   ```bash
   grep -rn 'execute_node\|run_node\|cell.*execute' crates/roko-orchestrator/src/graph/executor.rs | head -15
   ```

2. Read the corrigibility chain from M069:
   ```bash
   grep -rn 'CorrigibilityChain\|evaluate' crates/roko-gate/src/corrigibility.rs | head -10
   ```

3. Create `crates/roko-orchestrator/src/graph/safety.rs`:
   ```rust
   pub struct SafetyWrapper {
       corrigibility: CorrigibilityChain,
       camel_monitor: CamelMonitor,
       recursive_monitor: RecursiveSafetyMonitor,
   }

   impl SafetyWrapper {
       /// Wrap a Cell execution with safety checks.
       pub async fn wrap_execution(
           &self,
           cell_id: &str,
           input: &Signal,
           execute_fn: impl Future<Output = Result<Signal>>,
       ) -> Result<Signal> {
           // 1. Pre-check: corrigibility chain evaluates proposed action
           // 2. If vetoed: return error with veto reason
           // 3. Execute the Cell
           // 4. Post-check: verify output tags (CaMeL)
           // 5. Return result
       }
   }
   ```

4. Inject SafetyWrapper into the executor's node execution path:
   - The executor creates SafetyWrapper at startup (non-optional)
   - Every `execute_node` call goes through `safety.wrap_execution`
   - The SafetyWrapper is NOT part of the Graph definition -- it is engine-level

5. Ensure the safety wrapper is:
   - Non-removable: Graph authors cannot opt out
   - Non-modifiable: Extensions cannot alter the corrigibility chain
   - Always-present: even internal/system Graphs pass through it

6. Write tests:
   - Every Cell execution in a Graph passes through corrigibility check
   - Vetoed Cell does not execute (execution function is never called)
   - SafetyWrapper is present even for simple 1-node Graphs
   - Post-execution CaMeL check catches tag violations

## Verification
```bash
cargo check -p roko-orchestrator
cargo clippy -p roko-orchestrator --no-deps -- -D warnings
cargo test -p roko-orchestrator -- graph::safety
```

## What NOT to do
- Do NOT make safety checks optional via Graph config
- Do NOT add performance shortcuts that skip checks for "trusted" Cells
- Do NOT implement the safety wrapper as an Extension (it must be engine-level)
- Do NOT add asynchronous/deferred checking -- checks must complete before execution proceeds
