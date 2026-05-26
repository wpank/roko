# M162 — Wire Gossip Networking as Bus Topic Federation

## Objective
Wire gossip networking as Bus topic federation in `roko-chain`. The gossip types (envelope, topic partitions, CRDTs, Dandelion++ config) already exist in `phase2.rs` but have no runtime bridge that replicates local Bus Pulses to network peers. Create a `GossipBridge` Connect Cell that subscribes to 8 local Bus topic partitions and relays matching Pulses to libp2p GossipSub peers, while ingesting remote Pulses into the local Bus. Use HyParView for membership management.

## Scope
- Crates: `roko-chain`
- Files:
  - `/Users/will/dev/nunchi/roko/roko/crates/roko-chain/src/gossip/mod.rs` (new module)
  - `/Users/will/dev/nunchi/roko/roko/crates/roko-chain/src/gossip/bridge.rs` (GossipBridge Cell)
  - `/Users/will/dev/nunchi/roko/roko/crates/roko-chain/src/gossip/hyparview.rs` (membership)
  - `/Users/will/dev/nunchi/roko/roko/crates/roko-chain/src/gossip/topics.rs` (8 partition defs)
  - `/Users/will/dev/nunchi/roko/roko/crates/roko-chain/src/lib.rs` (re-export gossip module)
  - `/Users/will/dev/nunchi/roko/roko/crates/roko-chain/src/phase2.rs` (existing gossip types)
- Depth doc: `tmp/unified-depth/18-registries/07-gossip-and-privacy.md`

## Steps
1. Read existing gossip types in phase2.rs to understand the envelope and topic structures:
   ```bash
   grep -n 'GossipEnvelope\|GossipTopic\|GossipTier\|Dandelion\|GCounter\|LWWRegister' /Users/will/dev/nunchi/roko/roko/crates/roko-chain/src/phase2.rs | head -25
   ```

2. Confirm no existing gossip module:
   ```bash
   grep -rn 'mod gossip\|pub mod gossip' /Users/will/dev/nunchi/roko/roko/crates/roko-chain/src/lib.rs
   ls /Users/will/dev/nunchi/roko/roko/crates/roko-chain/src/gossip/ 2>/dev/null
   ```

3. Create `gossip/topics.rs` defining the 8 topic partitions:
   ```rust
   /// The 8 federated gossip topic partitions.
   pub const TOPIC_PARTITIONS: &[&str] = &[
       "identity.*",
       "reputation.*",
       "knowledge.*",
       "jobs.*",
       "challenge.*",
       "pheromone.*",
       "heartbeat.*",
       "governance.*",
   ];
   ```
   Each partition maps to a GossipSub topic string (e.g., `korai/identity/v1`).

4. Create `gossip/hyparview.rs` implementing HyParView membership:
   - Active view (max 5 peers) + passive view (max 30 peers)
   - `join(peer)`, `disconnect(peer)`, `shuffle()` operations
   - Periodic shuffle timer (every 30s) to maintain overlay health
   - Store peer list in memory (no persistence needed for this batch)

5. Create `gossip/bridge.rs` with the `GossipBridge` struct:
   ```rust
   /// Connect Cell that federates local Bus Pulses to gossip network peers.
   ///
   /// Subscribes to 8 topic partitions on the local Bus and relays matching
   /// Pulses to GossipSub peers. Ingests remote Pulses into local Bus.
   pub struct GossipBridge {
       local_bus: BusSender,
       membership: HyParView,
       topic_filter: TopicFilter,
       dandelion_config: DandelionConfig,
   }

   impl GossipBridge {
       pub async fn relay_outbound(&self, pulse: &Pulse) -> Result<(), GossipError> { ... }
       pub async fn ingest_inbound(&self, envelope: GossipEnvelope) -> Result<(), GossipError> { ... }
       pub fn matches_partition(&self, topic: &str) -> bool { ... }
   }
   ```

6. Wire Dandelion++ for privacy-sensitive topics (identity.*, reputation.*):
   - Stem phase: forward to 1 random peer
   - Fluff phase: broadcast after stem_timeout (default 5s)
   - Use existing `DandelionConfig` from phase2.rs

7. Add `pub mod gossip;` to `lib.rs` and re-export key types.

8. Write unit tests:
   - Topic partition matching (glob pattern)
   - HyParView join/disconnect/shuffle
   - Dandelion++ stem→fluff transition
   - GossipBridge routes outbound Pulse to correct partition

## Verification
```bash
cargo check -p roko-chain
cargo clippy -p roko-chain --no-deps -- -D warnings
cargo test -p roko-chain -- gossip
```

## What NOT to do
- Do NOT add libp2p as a real dependency — use trait abstractions for the network layer (real libp2p wiring is Phase 2+)
- Do NOT modify existing phase2.rs gossip types — import and use them as-is
- Do NOT implement actual network I/O — the bridge should work with mock transports for testing
- Do NOT persist membership state to disk — in-memory is sufficient for this batch
- Do NOT wire into orchestrate.rs — this is a chain-internal module
