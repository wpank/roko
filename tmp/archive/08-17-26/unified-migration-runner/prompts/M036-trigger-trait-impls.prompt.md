# M036 — Define Trigger trait and implement CronTrigger, BusTrigger, FileWatchTrigger

## Objective
Define the `Trigger` trait in roko-core as a formal protocol for event-driven execution. Then implement three builtin triggers: CronTrigger (periodic), BusTrigger (reacts to Bus Pulses), and FileWatchTrigger (reacts to filesystem changes). Each trigger publishes `trigger.fired.{id}` Pulses on the Bus when it fires.

## Scope
- Crates: `roko-core`, `roko-runtime`
- Files:
  - `crates/roko-core/src/traits.rs` (Trigger trait definition)
  - New: `crates/roko-runtime/src/triggers/` (directory)
  - New: `crates/roko-runtime/src/triggers/mod.rs`
  - New: `crates/roko-runtime/src/triggers/cron.rs`
  - New: `crates/roko-runtime/src/triggers/bus.rs`
  - New: `crates/roko-runtime/src/triggers/file_watch.rs`
  - `crates/roko-runtime/src/lib.rs` (module declaration)
- Phase ref: `tmp/unified-migration/02-PHASE-1-KERNEL.md` §1.11
- Spec ref: `tmp/unified/06-TRIGGER-SYSTEM.md` §1-4, §6

## Steps
1. Check if any trigger types already exist:
   ```bash
   grep -rn 'Trigger\|TriggerBinding\|TriggerHandle\|TriggerState\|TriggerSource' crates/roko-core/src/ --include='*.rs' | head -10
   grep -rn 'trigger\|Trigger' crates/roko-runtime/src/ --include='*.rs' | head -10
   ```

2. Check existing file watcher (known to exist for TUI):
   ```bash
   grep -rn 'RecommendedWatcher\|notify::' crates/ --include='*.rs' | grep -v target/ | head -10
   ```

3. Define the Trigger trait in `crates/roko-core/src/traits.rs`:
   ```rust
   /// Handle to an armed trigger.
   #[derive(Debug, Clone, PartialEq, Eq, Hash)]
   pub struct TriggerHandle(pub String);

   /// State of a trigger.
   #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
   pub enum TriggerState {
       Armed,
       Firing,
       Cooldown,
       Disarmed,
       Failed,
   }

   /// What kind of event source this trigger watches.
   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub enum TriggerSource {
       /// Periodic cron/interval schedule.
       Cron(String),
       /// Bus topic pattern.
       Bus(TopicFilter),
       /// Filesystem path pattern.
       FileWatch(String),
       /// Manual invocation.
       Manual,
   }

   /// Binding that connects a trigger event to an action.
   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub struct TriggerBinding {
       /// Unique identifier for this binding.
       pub id: String,
       /// The event source to watch.
       pub source: TriggerSource,
       /// Cooldown duration after firing (prevents rapid re-triggers).
       pub cooldown: Option<Duration>,
   }

   /// Protocol for event-driven execution triggers.
   ///
   /// See: tmp/unified/06-TRIGGER-SYSTEM.md §1-4.
   #[async_trait::async_trait]
   pub trait Trigger: Send + Sync {
       /// Arm a trigger with the given binding. Returns a handle for management.
       async fn arm(&mut self, binding: &TriggerBinding) -> Result<TriggerHandle>;

       /// Disarm a previously armed trigger.
       async fn disarm(&mut self, handle: &TriggerHandle) -> Result<()>;

       /// Check the current state of a trigger.
       fn state(&self, handle: &TriggerHandle) -> TriggerState;
   }
   ```

4. Create `crates/roko-runtime/src/triggers/mod.rs`:
   ```rust
   pub mod cron;
   pub mod bus;
   pub mod file_watch;

   pub use cron::CronTrigger;
   pub use bus::BusTrigger;
   pub use file_watch::FileWatchTrigger;
   ```

5. Implement `CronTrigger`:
   ```rust
   /// Fires at a configurable interval.
   pub struct CronTrigger {
       handles: HashMap<String, CronEntry>,
   }

   struct CronEntry {
       binding: TriggerBinding,
       state: TriggerState,
       last_fired: Option<Instant>,
       task: Option<JoinHandle<()>>,
   }
   ```
   Use `tokio::time::interval` for periodic firing. Publish `trigger.fired.{id}` Pulse on each fire.

6. Implement `BusTrigger`:
   - Subscribes to the Bus with the binding's TopicFilter
   - When a matching Pulse arrives, transitions to Firing state
   - Publishes `trigger.fired.{id}` Pulse

7. Implement `FileWatchTrigger`:
   - Uses `notify::RecommendedWatcher` (already a dependency for TUI)
   - Watches the configured path pattern
   - On file change, transitions to Firing and publishes Pulse
   - Check the existing TUI watcher for patterns to reuse:
     ```bash
     grep -rn 'RecommendedWatcher\|fs_watch' crates/roko-cli/src/tui/ --include='*.rs' | head -5
     ```

8. Add tests for each trigger type:
   ```rust
   #[tokio::test]
   async fn cron_trigger_fires_at_interval() { ... }

   #[tokio::test]
   async fn bus_trigger_fires_on_matching_pulse() { ... }

   #[tokio::test]
   async fn file_watch_trigger_fires_on_change() { ... }
   ```

## Verification
```bash
cargo check -p roko-core
cargo check -p roko-runtime
cargo clippy -p roko-runtime --no-deps -- -D warnings
cargo test -p roko-runtime -- trigger
```

## What NOT to do
- Do NOT add tokio_cron_scheduler — simple interval-based timing is sufficient for now
- Do NOT wire triggers into plan execution yet — just define the trait and implementations
- Do NOT share the TUI's existing file watcher instance — create a new one for triggers
- Do NOT make triggers synchronous — all methods should be async for consistency
- Do NOT implement ChainEvent or WebhookTrigger yet — those are Phase 2+
