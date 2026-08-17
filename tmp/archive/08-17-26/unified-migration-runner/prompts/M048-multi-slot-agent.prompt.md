# M048 — Multi-Slot Concurrent Execution

## Objective
Implement multi-slot execution state for Agents: each Agent manages N concurrent execution contexts (slots) with shared global limits (total budget, max concurrent, model pool). Each slot has its own CellContext but shares the Agent's Store and Bus. This enables an Agent to work on multiple tasks simultaneously while respecting shared resource constraints.

## Scope
- Crates: `roko-agent`
- Files: `crates/roko-agent/src/slots.rs` (new), `crates/roko-agent/src/lib.rs`
- Phase ref: `tmp/unified-migration/03-PHASE-2-ENGINE.md` SS2.5
- Spec ref: `tmp/unified/07-AGENT-RUNTIME.md` SS6 (Multi-Slot)

## Steps
1. Check for existing concurrency or slot-related code:
   ```bash
   grep -rn 'slot\|Slot\|concurrent\|parallel\|SlotManager' crates/roko-agent/src/ --include='*.rs' | head -15
   ```

2. Define slot types in `crates/roko-agent/src/slots.rs`:
   ```rust
   pub struct SlotManager {
       slots: Vec<Slot>,
       max_concurrent: usize,
       shared_budget: Arc<VitalityTracker>,
       semaphore: Arc<tokio::sync::Semaphore>,
   }

   pub struct Slot {
       pub index: usize,
       pub state: SlotState,
       pub current_task: Option<TaskInfo>,
       pub context: CellContext,
   }

   #[derive(Debug, Clone, PartialEq, Eq)]
   pub enum SlotState {
       Free,
       Running { task_id: String, started_at: DateTime<Utc> },
       Paused { task_id: String },
   }
   ```

3. Implement SlotManager:
   ```rust
   impl SlotManager {
       pub fn new(num_slots: usize, budget: Arc<VitalityTracker>) -> Self;
       pub fn acquire(&mut self) -> Option<&mut Slot>;  // Get a free slot
       pub fn release(&mut self, index: usize);         // Free a slot
       pub fn active_count(&self) -> usize;
       pub fn free_count(&self) -> usize;
       pub fn status(&self) -> Vec<SlotStatus>;
   }
   ```

4. Each slot execution acquires a semaphore permit, ensuring max_concurrent is respected globally.

5. Slots share:
   - Budget (via Arc<VitalityTracker> -- any slot's spend reduces shared budget)
   - Store (via Arc<dyn Store> -- knowledge is shared across slots)
   - Bus (via Arc<dyn Bus> -- all slots publish to the same Bus)

6. Slots do NOT share:
   - CellContext (each slot has its own execution context)
   - Intermediate signals (each slot's in-progress work is isolated)

7. Write tests:
   - Agent with 3 slots can acquire 3 slots concurrently
   - Acquiring a 4th slot when max_concurrent=3 returns None
   - Releasing a slot makes it available again
   - Budget spend in slot 1 is visible from slot 2 (shared VitalityTracker)

## Verification
```bash
cargo check -p roko-agent
cargo clippy -p roko-agent --no-deps -- -D warnings
cargo test -p roko-agent -- slots
```

## What NOT to do
- Do NOT implement task scheduling/assignment here -- slots are just execution contexts
- Do NOT add inter-slot communication -- slots share Store and Bus, that is sufficient
- Do NOT couple to the Graph executor -- slots are used by the Agent runtime, not by Graphs
- Do NOT implement slot preemption -- a running slot runs to completion or explicit cancellation
