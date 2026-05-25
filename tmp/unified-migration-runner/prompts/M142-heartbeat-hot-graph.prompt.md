# M142 — Wire Heartbeat as Hot Flow

## Objective
Wire the existing `HeartbeatPolicy` struct in `roko-runtime` as an active Hot Flow that emits gamma/theta/delta tick Pulses on the Bus. The heartbeat clock already has adaptive interval computation (`compute_gamma_interval`, `compute_theta_interval`, `adaptive_interval`) but no `tokio::spawn` loop that actually ticks. Create a `spawn_heartbeat()` function that runs the three-speed clock as concurrent tasks and wire it into the orchestrate.rs event loop so the heartbeat is alive during plan execution.

## Scope
- Crates: `roko-runtime`, `roko-cli`
- Files:
  - `/Users/will/dev/nunchi/roko/roko/crates/roko-runtime/src/heartbeat.rs` (add spawn logic)
  - `/Users/will/dev/nunchi/roko/roko/crates/roko-runtime/src/lib.rs` (re-export)
  - `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/orchestrate.rs` (wire into event loop)
- Depth doc: `tmp/unified-depth/05-heartbeat/` (adaptive clock algorithms)

## Steps
1. Read existing HeartbeatPolicy and confirm it has no spawn logic:
   ```bash
   grep -n 'spawn\|tokio::spawn\|async fn.*tick\|pub async fn' /Users/will/dev/nunchi/roko/roko/crates/roko-runtime/src/heartbeat.rs | head -15
   grep -n 'HeartbeatPolicy' /Users/will/dev/nunchi/roko/roko/crates/roko-runtime/src/lib.rs
   ```

2. Read the bus sender type and event enum:
   ```bash
   grep -n 'pub enum RokoEvent\|BusSender' /Users/will/dev/nunchi/roko/roko/crates/roko-runtime/src/event_bus.rs | head -10
   grep -n 'Heartbeat\|HeartbeatTick' /Users/will/dev/nunchi/roko/roko/crates/roko-runtime/src/event_bus.rs | head -10
   ```

3. Add `spawn_heartbeat()` to `HeartbeatPolicy` that spawns three concurrent tick loops:
   ```rust
   impl HeartbeatPolicy {
       /// Spawn the three-speed heartbeat clock as background tasks.
       ///
       /// Returns a `JoinHandle` for the supervisor task that manages all three speeds.
       /// Cancels cleanly when the `CancelToken` fires.
       pub fn spawn(self: Arc<Self>) -> tokio::task::JoinHandle<()> {
           let this = self.clone();
           tokio::spawn(async move {
               tokio::select! {
                   _ = this.cancel.cancelled() => {},
                   _ = this.run_gamma_loop() => {},
               }
           });
           // ... similar for theta and delta
       }
   }
   ```

4. Implement `run_gamma_loop()`:
   - Read `gamma_interval_millis` atomically each iteration
   - Emit `RokoEvent` on bus with topic `heartbeat.gamma`
   - Formula: interval = base / (1 + violations * 0.3), clamped to config min/max
   - Use `tokio::time::interval` with `MissedTickBehavior::Skip`

5. Implement `run_theta_loop()`:
   - Read `theta_interval_millis` atomically
   - Emit `RokoEvent` on bus with topic `heartbeat.theta`
   - Interval = base * regime_multiplier (Calm=2.0, Exploration=1.0, Crisis=0.5)

6. Implement `run_delta_loop()`:
   - Trigger on: episode_count >= threshold OR idle_time >= delta_timeout
   - Emit `RokoEvent` on bus with topic `heartbeat.delta`
   - Check conditions every 60s (poll, not pure timer)

7. Wire into orchestrate.rs:
   ```bash
   grep -n 'HeartbeatPolicy\|heartbeat\|CorticalState' /Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/orchestrate.rs | head -10
   ```
   After creating HeartbeatPolicy, call `policy.clone().spawn()` and store the JoinHandle for cleanup.

8. Ensure the CancelToken is shared with the plan runner so heartbeat stops when plan completes.

## Verification
```bash
cargo check -p roko-runtime
cargo clippy -p roko-runtime --no-deps -- -D warnings
cargo test -p roko-runtime -- heartbeat
cargo check -p roko-cli
```

## What NOT to do
- Do NOT rewrite HeartbeatPolicy — add spawn methods to the existing struct
- Do NOT add new dependencies — tokio and the bus sender are already available
- Do NOT make the tick loops block on anything — they must be purely async with atomic reads
- Do NOT wire delta to a real cron scheduler — a simple poll loop checking conditions is sufficient
- Do NOT modify CorticalState in this batch — that is M144
