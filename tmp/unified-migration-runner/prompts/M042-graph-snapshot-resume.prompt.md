# M042 — Flow Snapshot and Resume

## Objective
Implement snapshot/resume for Graph execution Flows. A FlowSnapshot captures the complete state of a running Flow: lifecycle state, per-node completion status, intermediate Signals, and Bus subscription positions. Snapshots serialize to JSON for persistence. Resume from snapshot skips completed nodes, replays deterministic orchestration, and re-executes non-deterministic activities. This enables crash recovery and long-running Flow management.

## Scope
- Crates: `roko-orchestrator`
- Files: `crates/roko-orchestrator/src/graph/snapshot.rs` (new), `crates/roko-orchestrator/src/graph/executor.rs` (add resume method), `crates/roko-orchestrator/src/graph/mod.rs`
- Phase ref: `tmp/unified-migration/03-PHASE-2-ENGINE.md` SS2.2
- Spec ref: `tmp/unified/05-EXECUTION-ENGINE.md` SS7 (Resumability)

## Steps
1. Read the existing snapshot/resume approach for reference:
   ```bash
   grep -rn 'snapshot\|resume\|ExecutorState\|executor.json' crates/roko-orchestrator/src/ --include='*.rs' | head -15
   grep -rn 'snapshot\|resume' crates/roko-cli/src/orchestrate.rs | head -10
   ```

2. Define `FlowSnapshot` in `crates/roko-orchestrator/src/graph/snapshot.rs`:
   ```rust
   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub struct FlowSnapshot {
       pub run_id: RunId,
       pub graph: Graph,
       pub flow_state: FlowState,
       pub node_states: BTreeMap<NodeId, NodeState>,
       pub intermediate_signals: BTreeMap<NodeId, Vec<Signal>>,
       pub cost_so_far: CostLedger,
       pub started_at: DateTime<Utc>,
       pub snapshot_at: DateTime<Utc>,
       pub agent: Option<AgentId>,
   }
   ```

3. Implement snapshot creation on the Flow:
   ```rust
   impl Flow {
       pub fn snapshot(&self) -> FlowSnapshot;
   }
   ```

4. Implement snapshot persistence:
   ```rust
   impl FlowSnapshot {
       pub fn save(&self, path: &Path) -> Result<()>;   // JSON serialization
       pub fn load(path: &Path) -> Result<Self>;         // JSON deserialization
   }
   ```

5. Add `resume` to the Engine:
   ```rust
   impl Engine {
       pub async fn resume(&self, snapshot: FlowSnapshot) -> Result<RunId> {
           // 1. Reconstruct Flow from snapshot
           // 2. Mark completed nodes as Completed (skip re-execution)
           // 3. Recompute ready set from remaining incomplete nodes
           // 4. Continue normal execution loop
           // 5. Emit flow.resumed Pulse
       }
   }
   ```

6. Add periodic snapshot checkpointing: the executor saves a snapshot every N completed nodes (configurable via GraphPolicy) to `.roko/state/flow-{run_id}.json`.

7. Write tests:
   - Snapshot a running Flow, serialize to JSON, deserialize back, verify all fields match
   - Pause a 5-node Flow after 2 nodes complete, snapshot, resume, complete remaining 3 nodes
   - Resume from snapshot skips already-completed nodes (no double execution)
   - Snapshot path is deterministic given run_id

## Verification
```bash
cargo check -p roko-orchestrator
cargo clippy -p roko-orchestrator --no-deps -- -D warnings
cargo test -p roko-orchestrator -- snapshot
cargo test -p roko-orchestrator -- resume
```

## What NOT to do
- Do NOT break the existing executor.json snapshot format -- this is a parallel implementation for the new Graph executor
- Do NOT implement deterministic replay of non-deterministic activities -- mark them for re-execution
- Do NOT add database-backed snapshots -- JSON files are sufficient for now
- Do NOT couple snapshot format to any specific storage backend
