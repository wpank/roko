# M167 — Wire Config Hot-Reload as Trigger Cell

## Objective
Wire config hot-reload as a Trigger Cell in `roko-cli`. The `notify` crate is already used for TUI file watching (`tui/fs_watch.rs`), and `roko-serve` has a `config_watcher.rs` that polls every 2 seconds. Upgrade the CLI to use `notify::RecommendedWatcher` on `roko.toml` with 500ms debounce, re-parse on change, emit a `config.reloaded` Pulse on the Bus, and propagate the updated config Signal to all subscribed Cells.

## Scope
- Crates: `roko-cli`, `roko-core`
- Files:
  - `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/config_reload.rs` (new — Trigger Cell)
  - `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/lib.rs` (wire module)
  - `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/orchestrate.rs` (start watcher at plan run)
  - `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/config_watcher.rs` (reference for existing approach)
  - `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/config.rs` (config parsing)
- Depth doc: `tmp/unified-depth/14-config/02-layered-resolution-and-reload.md`

## Steps
1. Read existing config watcher in roko-serve:
   ```bash
   cat /Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/config_watcher.rs | head -60
   ```

2. Read TUI file watcher for notify usage pattern:
   ```bash
   grep -n 'notify\|RecommendedWatcher\|Watcher\|debounce' /Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/fs_watch.rs | head -15
   ```

3. Check notify dependency availability:
   ```bash
   grep -n 'notify' /Users/will/dev/nunchi/roko/roko/crates/roko-cli/Cargo.toml
   ```

4. Read config parsing to understand reload:
   ```bash
   grep -n 'pub fn.*load\|pub fn.*parse\|from_file\|RokoConfig' /Users/will/dev/nunchi/roko/roko/crates/roko-core/src/config.rs | head -15
   ```

5. Create `config_reload.rs` with the `ConfigReloadTrigger`:
   ```rust
   use notify::{RecommendedWatcher, RecursiveMode, Watcher, Event, EventKind};
   use std::time::{Duration, Instant};

   /// Trigger Cell that watches roko.toml for changes and emits reload Pulses.
   ///
   /// Uses notify::RecommendedWatcher with 500ms debounce. On valid config change,
   /// emits `config.reloaded` Pulse carrying the new config as Signal payload.
   pub struct ConfigReloadTrigger {
       watcher: Option<RecommendedWatcher>,
       config_path: PathBuf,
       debounce_ms: u64,          // default: 500
       last_reload: Instant,
       bus_sender: Option<BusSender>,
   }

   impl ConfigReloadTrigger {
       pub fn new(config_path: PathBuf) -> Self { ... }
       pub fn with_debounce(mut self, ms: u64) -> Self { ... }
       pub fn with_bus(mut self, sender: BusSender) -> Self { ... }

       /// Start watching. Returns a JoinHandle for the watcher task.
       pub fn start(self) -> Result<JoinHandle<()>, ConfigReloadError> { ... }
   }
   ```

6. Implement the watch loop:
   ```rust
   async fn watch_loop(
       config_path: PathBuf,
       debounce: Duration,
       bus: BusSender,
   ) -> Result<(), ConfigReloadError> {
       let (tx, mut rx) = tokio::sync::mpsc::channel(16);
       let mut watcher = RecommendedWatcher::new(
           move |res: Result<Event, _>| {
               if let Ok(event) = res {
                   if matches!(event.kind, EventKind::Modify(_)) {
                       let _ = tx.blocking_send(event);
                   }
               }
           },
           notify::Config::default(),
       )?;
       watcher.watch(&config_path, RecursiveMode::NonRecursive)?;

       let mut last_reload = Instant::now();
       while let Some(_event) = rx.recv().await {
           if last_reload.elapsed() < debounce {
               continue; // debounce
           }
           match RokoConfig::from_file(&config_path) {
               Ok(new_config) => {
                   last_reload = Instant::now();
                   // Emit config.reloaded Pulse with new config
                   bus.send(Pulse::new("config.reloaded", Signal::from(new_config)));
               }
               Err(e) => {
                   tracing::warn!("config reload failed (keeping previous): {e}");
               }
           }
       }
       Ok(())
   }
   ```

7. Wire into orchestrate.rs — start the watcher when `plan run` begins:
   ```bash
   grep -n 'pub async fn run_plan\|pub async fn orchestrate\|async fn main_loop' /Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/orchestrate.rs | head -10
   ```
   Call `ConfigReloadTrigger::new(config_path).with_bus(bus).start()` early in the orchestration setup.

8. Write unit tests:
   - Debounce prevents rapid re-parsing
   - Invalid TOML does not crash, logs warning
   - Valid change emits Pulse with new config
   - Watcher cleans up on drop

## Verification
```bash
cargo check -p roko-cli
cargo clippy -p roko-cli --no-deps -- -D warnings
cargo test -p roko-cli -- config_reload
cargo check -p roko-core
```

## What NOT to do
- Do NOT remove the existing roko-serve config_watcher.rs — it serves the HTTP server separately
- Do NOT use polling — use notify's native fs event API (the serve crate polls as a deliberate fallback)
- Do NOT make config reload synchronous — it must be async and non-blocking
- Do NOT reload on every file event — debounce at 500ms minimum
- Do NOT panic on invalid config — log and keep the previous valid config
