# M165 — Wire Chain Agent Heartbeat as Hot Flow

## Objective
Wire the chain agent heartbeat as a Hot Flow in `roko-chain`. The `ChainHeartbeatExtension` in `heartbeat_ext.rs` already implements SIMULATE + VALIDATE steps, but there is no periodic liveness ping loop that actually fires heartbeats on-chain. Create a `ChainHeartbeatFlow` that sends a liveness ping every 100 blocks (configurable), tracks heartbeat regularity, emits `chain.heartbeat.sent` Pulses on the Bus, and emits `chain.heartbeat.missed` when the heartbeat lapses beyond 200 blocks for conductor alerting.

## Scope
- Crates: `roko-chain`
- Files:
  - `/Users/will/dev/nunchi/roko/roko/crates/roko-chain/src/heartbeat.rs` (new file — the hot flow)
  - `/Users/will/dev/nunchi/roko/roko/crates/roko-chain/src/heartbeat_ext.rs` (existing extension, read-only reference)
  - `/Users/will/dev/nunchi/roko/roko/crates/roko-chain/src/lib.rs` (re-export)
- Depth doc: `tmp/unified-depth/18-registries/08-simulation-and-liveness.md`

## Steps
1. Read existing chain heartbeat extension to understand the interface:
   ```bash
   grep -n 'pub struct\|pub fn\|pub async fn\|impl ' /Users/will/dev/nunchi/roko/roko/crates/roko-chain/src/heartbeat_ext.rs | head -20
   ```

2. Confirm no existing heartbeat flow:
   ```bash
   grep -rn 'ChainHeartbeatFlow\|heartbeat_flow\|liveness_ping' /Users/will/dev/nunchi/roko/roko/crates/roko-chain/src/ | head -10
   ls /Users/will/dev/nunchi/roko/roko/crates/roko-chain/src/heartbeat.rs 2>/dev/null
   ```

3. Create `heartbeat.rs` with the `ChainHeartbeatFlow` struct:
   ```rust
   /// Hot Flow that periodically pings on-chain liveness.
   ///
   /// Default interval: every 100 blocks. If heartbeat lapses (>200 blocks
   /// since last successful ping), emits a missed Pulse for conductor alerting.
   pub struct ChainHeartbeatFlow {
       agent_id: AgentId,
       interval_blocks: u64,        // default: 100
       lapse_threshold: u64,        // default: 200
       last_heartbeat_block: AtomicU64,
       consecutive_misses: AtomicU32,
       bus_sender: Option<BusSender>,
   }

   impl ChainHeartbeatFlow {
       pub fn new(agent_id: AgentId) -> Self { ... }
       pub fn with_interval(mut self, blocks: u64) -> Self { ... }
       pub fn with_lapse_threshold(mut self, blocks: u64) -> Self { ... }
       pub fn with_bus(mut self, sender: BusSender) -> Self { ... }
   }
   ```

4. Implement the heartbeat tick logic:
   ```rust
   impl ChainHeartbeatFlow {
       /// Called each block (or polled periodically) to check if heartbeat is due.
       pub async fn tick(&self, current_block: u64) -> HeartbeatAction {
           let last = self.last_heartbeat_block.load(Ordering::Relaxed);
           let elapsed = current_block.saturating_sub(last);

           if elapsed >= self.lapse_threshold {
               self.consecutive_misses.fetch_add(1, Ordering::Relaxed);
               self.emit_pulse("chain.heartbeat.missed", current_block).await;
               return HeartbeatAction::Missed { elapsed, consecutive: self.consecutive_misses.load(Ordering::Relaxed) };
           }

           if elapsed >= self.interval_blocks {
               self.last_heartbeat_block.store(current_block, Ordering::Relaxed);
               self.consecutive_misses.store(0, Ordering::Relaxed);
               self.emit_pulse("chain.heartbeat.sent", current_block).await;
               return HeartbeatAction::Sent { block: current_block };
           }

           HeartbeatAction::Waiting { blocks_until_next: self.interval_blocks - elapsed }
       }

       async fn emit_pulse(&self, topic: &str, block: u64) { ... }
   }
   ```

5. Define action enum:
   ```rust
   #[derive(Debug, Clone, PartialEq)]
   pub enum HeartbeatAction {
       Sent { block: u64 },
       Missed { elapsed: u64, consecutive: u32 },
       Waiting { blocks_until_next: u64 },
   }
   ```

6. Add regularity tracking:
   ```rust
   impl ChainHeartbeatFlow {
       /// Regularity score: ratio of on-time heartbeats to total expected.
       pub fn regularity(&self) -> f64 { ... }

       /// Total heartbeats sent since creation.
       pub fn total_sent(&self) -> u64 { ... }
   }
   ```

7. Re-export from `lib.rs`:
   ```rust
   pub mod heartbeat;
   pub use heartbeat::{ChainHeartbeatFlow, HeartbeatAction};
   ```

8. Write unit tests:
   - Tick at interval boundary emits Sent
   - Tick before interval returns Waiting
   - Tick past lapse threshold emits Missed
   - Consecutive misses increment correctly
   - Successful heartbeat resets miss counter
   - Regularity score computes correctly

## Verification
```bash
cargo check -p roko-chain
cargo clippy -p roko-chain --no-deps -- -D warnings
cargo test -p roko-chain -- heartbeat
```

## What NOT to do
- Do NOT modify `heartbeat_ext.rs` — that is the SIMULATE+VALIDATE extension, this is the liveness flow
- Do NOT implement actual on-chain transaction submission — the `tick()` method is called by the runtime
- Do NOT add a tokio spawn loop here — the caller (runtime or conductor) drives the tick
- Do NOT depend on real block subscription — accept `current_block` as parameter
- Do NOT wire into orchestrate.rs — this is chain-internal, driven by chain event listeners
