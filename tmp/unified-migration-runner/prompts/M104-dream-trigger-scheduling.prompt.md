# M104 — Dream Trigger Cell Scheduling

## Objective
Replace the standalone scheduling state machine in `roko-dreams/src/runner.rs` with Trigger-based scheduling that uses the same Trigger protocol as the rest of the system. Dream cycles are triggered by three conditions: (1) idle timer (cron-like periodic), (2) episode count threshold (event-driven), and (3) explicit CLI invocation. The Trigger protocol unifies these three into a single interface.

## Scope
- Crates: `roko-dreams`, `roko-cli`
- Files: `crates/roko-dreams/src/runner.rs` (existing scheduler), `crates/roko-dreams/src/dream_graph.rs` (from M103)
- Phase ref: depth doc 11-memory/06-dream-cycle-as-loop.md
- Depth doc: `tmp/unified-depth/11-memory/06-dream-cycle-as-loop.md`

## Steps
1. Discover current scheduling types in runner.rs:
   ```bash
   grep -n 'pub struct\|pub enum\|pub fn\|pub async fn' crates/roko-dreams/src/runner.rs | head -30
   grep -n 'DreamSchedulePolicy\|DreamTrigger\|PlanCompletionTrigger\|BusPulseTrigger' crates/roko-dreams/src/runner.rs | head -10
   ```

2. **Existing scheduling types** (in `crates/roko-dreams/src/runner.rs`):
   ```rust
   pub struct DreamSchedulePolicy { ... }      // idle_delay, cron_delay, allows, trigger_delay
   pub struct PlanCompletionTriggerPolicy { ... }  // should_trigger based on plan completion
   pub struct BusPulseTriggerConfig { ... }    // is_dream_worthy for bus pulse scores
   pub struct DreamRunner { ... }              // main runner with DreamLoopConfig
   pub struct DreamBudget { ... }              // token budget tracking
   ```
   **Important**: There is ALREADY a `pub enum DreamTrigger` in runner.rs with variants: `Idle`, `Scheduled`, `Manual`, `EpisodeCount`, `BusPulse { engram_hash }`, `CoordinationPattern { pattern_name, contributing_watchers }`. The existing enum represents "what triggered a dream" (past tense). The new `DreamTriggerCondition` below represents "conditions that CAN trigger a dream" (future tense, with thresholds). They complement each other -- `DreamTriggerCondition` defines when to fire, `DreamTrigger` records what fired.

3. Check if a core Trigger protocol exists:
   ```bash
   grep -rn 'trait Trigger\|TriggerBinding' crates/roko-core/src/ --include='*.rs' | head -10
   ```

4. Define unified dream trigger conditions in `crates/roko-dreams/src/runner.rs` (or a new file `trigger.rs`):
   ```rust
   /// Conditions that can trigger a dream cycle.
   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub enum DreamTriggerCondition {
       /// Periodic timer (e.g., every 6 hours of idle time)
       IdleTimer { idle_seconds: u64 },
       /// Episode count threshold (e.g., after 50 new episodes)
       EpisodeThreshold { min_new_episodes: usize },
       /// Explicit invocation (CLI or API)
       Manual,
       /// Budget-based: when token budget accumulates past threshold
       BudgetAccumulated { min_tokens: u64 },
   }

   /// Registry of active dream triggers.
   pub struct DreamTriggerRegistry {
       triggers: Vec<DreamTriggerCondition>,
       last_cycle: Option<DateTime<Utc>>,
       episodes_since_last: usize,
   }
   ```

4. Implement trigger evaluation:
   ```rust
   impl DreamTriggerRegistry {
       /// Check if any trigger condition is met.
       pub fn should_trigger(&self, context: &DreamTriggerContext) -> Option<DreamTriggerCondition> {
           for trigger in &self.triggers {
               match trigger {
                   DreamTriggerCondition::IdleTimer { idle_seconds } => {
                       if context.idle_duration.as_secs() >= *idle_seconds { return Some(trigger.clone()); }
                   }
                   DreamTriggerCondition::EpisodeThreshold { min_new_episodes } => {
                       if self.episodes_since_last >= *min_new_episodes { return Some(trigger.clone()); }
                   }
                   DreamTriggerCondition::Manual => {
                       if context.manual_requested { return Some(trigger.clone()); }
                   }
                   DreamTriggerCondition::BudgetAccumulated { min_tokens } => {
                       if context.accumulated_tokens >= *min_tokens { return Some(trigger.clone()); }
                   }
               }
           }
           None
       }

       /// Record that a cycle completed, resetting counters.
       pub fn cycle_completed(&mut self) { ... }

       /// Record that new episodes were added.
       pub fn episodes_added(&mut self, count: usize) { ... }
   }
   ```

5. Add a `DreamTriggerContext` struct:
   ```rust
   pub struct DreamTriggerContext {
       pub idle_duration: Duration,
       pub manual_requested: bool,
       pub accumulated_tokens: u64,
       pub current_time: DateTime<Utc>,
   }
   ```

6. Wire into the existing `DreamRunner` (in `runner.rs`) to use trigger registry alongside the existing `DreamSchedulePolicy`:
   ```bash
   grep -n 'pub struct DreamRunner' crates/roko-dreams/src/runner.rs | head -3
   grep -A 10 'pub struct DreamRunner' crates/roko-dreams/src/runner.rs
   ```
   Add `DreamTriggerRegistry` as a field on `DreamRunner` or as a companion to `DreamSchedulePolicy`.

7. Write tests:
   - IdleTimer triggers after sufficient idle time
   - EpisodeThreshold triggers after N episodes
   - Manual trigger always fires when requested
   - cycle_completed resets counters
   - No trigger fires when conditions are not met

## Verification
```bash
cargo check -p roko-dreams
cargo clippy -p roko-dreams --no-deps -- -D warnings
cargo test -p roko-dreams -- runner
```

## What NOT to do
- Do NOT remove or rename the existing `pub enum DreamTrigger` -- it records what already triggered; your new `DreamTriggerCondition` defines future conditions
- Do NOT remove the existing runner state machine -- add triggers alongside it
- Do NOT implement the actual dream execution (that is M103)
- Do NOT add a background thread or tokio task for scheduling -- just provide the evaluation logic
- Do NOT implement Bus subscriptions for trigger events -- keep it polling-based for now
