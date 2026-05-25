# M043 — Hot Flow Variant

## Objective
Implement the Hot Flow variant: a Flow that stays resident in memory and re-fires on each clock tick. Hot Flows power Agent processing pipelines (the 9-step pipeline) and any long-lived reactive computation. State persists between ticks. Registration and deregistration are managed through the Engine. Hot Flows emit lifecycle Pulses like regular Flows.

## Scope
- Crates: `roko-orchestrator`
- Files: `crates/roko-orchestrator/src/graph/hot.rs` (new), `crates/roko-orchestrator/src/graph/executor.rs` (add register_hot/deregister_hot), `crates/roko-orchestrator/src/graph/schema.rs` (ensure `hot` + `clock_binding` in GraphPolicy)
- Phase ref: `tmp/unified-migration/03-PHASE-2-ENGINE.md` SS2.3
- Spec ref: `tmp/unified/05-EXECUTION-ENGINE.md` SS8 (Hot Graphs), `tmp/unified/04-SPECIALIZATIONS.md` SS1.1 (Hot Flow)

## Steps
1. Check existing Hot Flow or tick-based execution patterns:
   ```bash
   grep -rn 'hot\|Hot\|tick\|clock\|ClockBinding\|TickPipeline' crates/roko-orchestrator/src/ --include='*.rs' | head -15
   grep -rn 'AdaptiveClock\|clock' crates/roko-agent/src/ --include='*.rs' | head -10
   ```

2. Ensure `GraphPolicy` in schema.rs has:
   ```rust
   pub hot: bool,
   pub clock_binding: Option<ClockBinding>,
   ```
   And define:
   ```rust
   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub struct ClockBinding {
       pub kind: ClockKind,
       pub period_ms: u64,
       pub name: Option<String>,
   }

   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub enum ClockKind {
       Fixed,
       Adaptive,
       Custom,
   }
   ```

3. Implement `HotFlow` in `crates/roko-orchestrator/src/graph/hot.rs`:
   ```rust
   pub struct HotFlow {
       pub flow: Flow,
       pub clock: ClockBinding,
       pub tick_count: u64,
       pub retained_state: BTreeMap<NodeId, Signal>,
       cancel: CancellationToken,
   }
   ```

4. Implement the Hot Flow tick loop:
   - On each tick: feed retained state as input, execute the Graph, retain node outputs for next tick
   - Emit `hotflow.tick` Pulse with tick count and cost
   - Respect budget: if cost exceeds `policy.budget`, deregister automatically
   - State persists in memory between ticks; flush to disk on deregistration or periodic checkpoint

5. Add Engine methods:
   ```rust
   impl Engine {
       pub async fn register_hot(&self, graph: Graph, initial_state: Vec<Signal>) -> Result<RunId>;
       pub async fn deregister_hot(&self, run_id: &RunId) -> Result<Vec<Signal>>;
   }
   ```

6. Registration spawns a tokio task that ticks at `clock_binding.period_ms` intervals. Deregistration cancels via CancellationToken and returns final state.

7. Write tests:
   - Register a Hot Graph, send 3 ticks, confirm it executes 3 times
   - State carries over between ticks (output of tick N is input to tick N+1)
   - Deregistration stops ticking and returns final state
   - Budget exhaustion triggers automatic deregistration

## Verification
```bash
cargo check -p roko-orchestrator
cargo clippy -p roko-orchestrator --no-deps -- -D warnings
cargo test -p roko-orchestrator -- hot
```

## What NOT to do
- Do NOT implement the 9-step Agent pipeline here -- that is Agent-specific composition on top of Hot Flow
- Do NOT implement AdaptiveClock frequency adjustment -- just use fixed period for now
- Do NOT couple Hot Flow to the Agent runtime -- keep it as a pure Engine feature
- Do NOT block the tick loop on slow nodes -- use the same semaphore-based parallelism as regular Flows
