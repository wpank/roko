# M034 — Wire dream cycle to run automatically

## Objective
The dream cycle (NREM compression, REM imagination, integration) in roko-dreams works when manually invoked but has no automatic trigger. Add a configurable interval/cron trigger so the dream cycle runs periodically during `roko serve` and as a background task in the dashboard. Configuration lives in `roko.toml` under `[learning.dreams]`.

## Scope
- Crates: `roko-dreams`, `roko-serve`, `roko-cli`
- Files:
  - `crates/roko-dreams/src/runner.rs` (DreamRunner)
  - `crates/roko-serve/src/lib.rs` or `crates/roko-serve/src/startup.rs` (serve startup)
  - `crates/roko-cli/src/orchestrate.rs` (background task option)
  - `crates/roko-core/src/config/schema.rs` (config schema)
- Phase ref: `tmp/unified-migration/02-PHASE-1-KERNEL.md` §1.9
- Spec ref: `tmp/unified/21-ROADMAP.md` §2.6
- Architecture ref: `tmp/architecture/07-dreams.md`

## Steps
1. Read the DreamRunner API:
   ```bash
   grep -n 'pub fn\|pub async fn' crates/roko-dreams/src/runner.rs | head -20
   grep -n 'DreamRunner\|DreamEngine\|DreamLoopConfig' crates/roko-dreams/src/lib.rs
   ```

2. Check existing dream configuration:
   ```bash
   grep -rn 'dream\|Dream' crates/roko-core/src/config/ --include='*.rs' | head -15
   grep -rn '\[.*dream' roko.toml 2>/dev/null || echo "No dream config in roko.toml"
   ```

3. Add dream trigger configuration to the config schema:
   ```rust
   /// Dream cycle configuration.
   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub struct DreamConfig {
       /// Whether automatic dream cycles are enabled.
       #[serde(default)]
       pub enabled: bool,
       /// Interval between dream cycles (e.g., "6h", "12h", "24h").
       #[serde(default = "default_dream_interval")]
       pub interval: String,
       /// Minimum number of new episodes before a dream cycle triggers.
       #[serde(default = "default_min_episodes")]
       pub min_episodes_before_dream: u32,
   }

   fn default_dream_interval() -> String { "12h".into() }
   fn default_min_episodes_before_dream() -> u32 { 10 }
   ```

4. Create a dream trigger task that runs on `roko serve` startup:
   ```rust
   /// Spawn a background task that triggers dream cycles at configured intervals.
   pub fn spawn_dream_trigger(
       config: DreamConfig,
       runner: Arc<DreamRunner>,
   ) -> tokio::task::JoinHandle<()> {
       tokio::spawn(async move {
           let interval = parse_duration(&config.interval);
           loop {
               tokio::time::sleep(interval).await;
               if should_dream(&config) {
                   match runner.consolidate_now().await {
                       Ok(report) => tracing::info!(?report, "Dream cycle completed"),
                       Err(e) => tracing::warn!(?e, "Dream cycle failed"),
                   }
               }
           }
       })
   }
   ```

5. Wire into `roko serve` startup (in roko-serve):
   ```bash
   grep -n 'pub async fn serve\|fn start_server\|pub fn app' crates/roko-serve/src/ -r --include='*.rs' | head -10
   ```
   After the server starts, spawn the dream trigger if `config.learning.dreams.enabled`.

6. Wire into `roko dashboard` as a background task (optional — lower priority):
   ```bash
   grep -n 'dream\|background' crates/roko-cli/src/tui/ -r --include='*.rs' | head -10
   ```

7. Emit a Pulse on the Bus when a dream cycle starts/completes:
   - `dream.cycle.started` with cycle ID and timestamp
   - `dream.cycle.completed` with report summary

8. Add tests:
   ```rust
   #[tokio::test]
   async fn dream_trigger_respects_interval() {
       // Create config with 1-second interval, verify DreamRunner called
   }
   ```

## Verification
```bash
cargo check -p roko-dreams
cargo check -p roko-serve
cargo clippy --workspace --no-deps -- -D warnings
cargo test -p roko-dreams -- runner
# Verify config integration:
grep -rn 'DreamConfig\|dream.*enabled' crates/roko-core/src/config/ --include='*.rs'
```

## What NOT to do
- Do NOT add tokio_cron_scheduler as a dependency — simple `tokio::time::sleep` loop is sufficient for interval-based triggering
- Do NOT run dream cycles synchronously in the request path — always spawn as a background task
- Do NOT dream if there are insufficient new episodes — check min_episodes_before_dream
- Do NOT modify the DreamRunner internals — just call its existing public API
