# M041 — Graph Failure Strategies

## Objective
Implement per-node failure strategies for the Graph executor: Fail (propagate), Retry with configurable backoff, Escalate to a higher-tier model, Compensate by running a compensation Graph, Skip for non-critical nodes, and Detour to an alternative Graph. The Graph-level default strategy is overridden per-node. This makes execution resilient without requiring manual intervention for transient failures.

## Scope
- Crates: `roko-orchestrator`
- Files: `crates/roko-orchestrator/src/graph/executor.rs` (modify), `crates/roko-orchestrator/src/graph/schema.rs` (add FailureStrategy if not present), `crates/roko-orchestrator/src/graph/strategies.rs` (new)
- Phase ref: `tmp/unified-migration/03-PHASE-2-ENGINE.md` SS2.2
- Spec ref: `tmp/unified/05-EXECUTION-ENGINE.md` SS6 (Failure Strategies)

## Steps
1. Check if FailureStrategy is already defined in the schema:
   ```bash
   grep -rn 'FailureStrategy\|failure_strategy\|Retry\|Escalate\|Compensate\|Detour' crates/roko-orchestrator/src/ --include='*.rs' | head -15
   ```

2. Define or update `FailureStrategy` enum in schema.rs:
   ```rust
   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub enum FailureStrategy {
       /// Propagate failure to the Flow level.
       Fail,
       /// Retry with exponential backoff.
       Retry { max: u32, backoff_ms: u64 },
       /// Escalate to a higher-tier model and retry.
       Escalate { to_model: String },
       /// Run a compensation Graph to undo side effects.
       Compensate { graph_id: String },
       /// Skip this node and continue (for non-critical nodes).
       Skip,
       /// Detour to an alternative Graph for this step.
       Detour { graph_id: String },
   }
   ```

3. Implement strategy execution in `crates/roko-orchestrator/src/graph/strategies.rs`:
   - `apply_failure_strategy(engine, flow, node_id, error, strategy) -> StrategyOutcome`
   - `StrategyOutcome` enum: `Propagated`, `Retried { attempt }`, `Escalated { model }`, `Compensated`, `Skipped`, `Detoured`
   - **Retry**: loop up to `max` times with exponential backoff (`backoff_ms * 2^attempt`), emit `node.retry` Pulse each attempt
   - **Escalate**: modify the Cell's execution context to use `to_model`, then retry once
   - **Skip**: mark node as `Skipped`, emit `node.skipped` Pulse, pass empty/default Signal to dependents
   - **Compensate**: load and execute the compensation Graph (if available); if compensation fails, propagate original error
   - **Detour**: load and execute the detour Graph in place of the failed node

4. Wire strategy application into the executor's node failure handler (in executor.rs).

5. Ensure the node's `failure_strategy` overrides the Graph's `policy.failure_strategy` default.

6. Write tests:
   - Node with `Retry { max: 3, backoff_ms: 100 }` retries 3 times before failing
   - Node with `Skip` bypasses failure and dependents execute with empty input
   - Node with `Fail` propagates error to Flow level immediately
   - Graph-level default applies when node has no override

## Verification
```bash
cargo check -p roko-orchestrator
cargo clippy -p roko-orchestrator --no-deps -- -D warnings
cargo test -p roko-orchestrator -- strategies
cargo test -p roko-orchestrator -- graph::executor -- --include-ignored  # retry timing tests
```

## What NOT to do
- Do NOT implement real model escalation (LLM calls) -- use mock Cells that simulate escalation
- Do NOT implement Graph loading for Compensate/Detour inline -- call the loader from M039
- Do NOT add circuit-breaker logic here -- that belongs in roko-conductor
- Do NOT modify the Graph TOML schema structure -- only add the FailureStrategy enum if missing
