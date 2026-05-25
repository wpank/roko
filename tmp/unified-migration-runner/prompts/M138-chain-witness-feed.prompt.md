# M138 — ChainWitnessFeed Cell (Connect + Trigger + Store)

**[BLOCKED:chain]** -- Requires M131 (ChainConnector), M036 (Trigger protocol), M037 (Connect protocol), M012 (Cell trait). Chain deployment is Tier 6.

## Objective
Implement `ChainWitnessFeed` -- a Feed Cell (`Cell + Connect + Trigger + Store`) that connects to EVM WebSocket endpoints, triggers on new block headers, and publishes chain events as Pulses on Bus. The Feed includes a Binary Fuse probe (T0 cognitive probe) that eliminates >90% of blocks without fetching receipts, a block gap tracker, and automatic reconnection with exponential backoff.

## Scope
- Crates: `roko-chain`
- Files:
  - `crates/roko-chain/src/witness_feed.rs` (new)
  - `crates/roko-chain/src/lib.rs` (add module + re-exports)
- Depth doc: `tmp/unified-depth/18-registries/05-chain-witness-and-triage.md` SS1-2

## Steps
1. Verify existing observer/witness infrastructure:
   ```bash
   grep -rn 'pub struct BlockObserver\|pub struct AddressFilter\|pub struct BlockTracker' crates/roko-chain/src/observer.rs
   grep -rn 'pub struct ObservedEvent' crates/roko-chain/src/observer.rs
   grep -rn 'pub struct ChainWitnessEngine\|pub fn anchor_signal' crates/roko-chain/src/witness.rs
   grep -rn 'pub struct BlockObserverConfig' crates/roko-chain/src/observer.rs
   ```
   **Expected**: `BlockObserver` at `observer.rs:150` wraps `BlockObserverConfig`, `AddressFilter`, `BlockTracker`. Methods: `new(config)`, `process_block(header, logs) -> Vec<ObservedEvent>`, `pending_gaps() -> Vec<BlockNumber>`, `scan_range(blocks) -> Vec<ObservedEvent>`. `AddressFilter` at `observer.rs:42` with `from_config()` and `matches(log) -> bool`. `BlockTracker` at `observer.rs:93` with `mark_processed()`, `detect_gaps()`, `processed_count()`, `high_water_mark()`. `ObservedEvent` at `observer.rs:137` with fields: `block_number`, `block_hash`, `block_timestamp`, `log: LogEntry`. `ChainWitnessEngine` at `witness.rs` with methods: `new()`, `anchor_signal()`, `verify_anchor()`, etc.

2. Verify Connect, Trigger, Store protocol traits in roko-core:
   ```bash
   grep -rn 'pub trait Connect' crates/roko-core/src/traits.rs
   grep -rn 'pub trait Trigger' crates/roko-core/src/traits.rs
   grep -rn 'pub trait Store' crates/roko-core/src/traits.rs
   ```
   **Expected**: `Connect` at `traits.rs:408` (supertrait of `Cell`, methods: `connect() -> Result<()>`, `health() -> bool`, `disconnect() -> Result<()>`). `Trigger` at `traits.rs:420` (supertrait of `Cell`, methods: `arm() -> Result<()>`, `disarm() -> Result<()>`). `Store` at `traits.rs:37` (async: `put(Engram) -> ContentHash`, `get(&ContentHash) -> Option<Engram>`, `query(&Query, &Context) -> Vec<Engram>`, `query_similar(&HdcVector, usize, &Context) -> Vec<(Engram, f32)>`).

3. Verify types used from roko-chain:
   ```bash
   grep -rn 'pub type BlockNumber\|pub struct ChainHeader\|pub struct LogEntry' crates/roko-chain/src/types.rs
   ```
   **Expected**: `BlockNumber` = `u64`. `ChainHeader` with fields: `number: BlockNumber`, `hash: String`, `parent: String`, `timestamp: u64`. `LogEntry` with fields: `address: String`, `topics: Vec<String>`, `data: Vec<u8>`.

4. Create `crates/roko-chain/src/witness_feed.rs`:

   ```rust
   use std::collections::HashSet;
   use std::sync::Arc;
   use crate::client::ChainClient;
   use crate::observer::{BlockObserver, BlockObserverConfig, ObservedEvent};
   use crate::types::{BlockNumber, ChainHeader, LogEntry};
   use roko_core::cell::{Cell, CellId};
   use roko_core::traits::{Connect, Trigger, Store};
   use roko_core::error::Result;

   /// Configuration for the ChainWitnessFeed.
   #[derive(Debug, Clone)]
   pub struct WitnessFeedConfig {
       /// Chain ID to connect to.
       pub chain_id: u64,
       /// WebSocket RPC endpoint URL.
       pub ws_endpoint: String,
       /// Addresses to watch.
       pub watched_addresses: Vec<String>,
       /// Topics to watch.
       pub watched_topics: Vec<String>,
       /// Maximum gap size before warning (instead of backfill).
       pub max_backfill_gap: u64,
       /// Reconnection base delay in milliseconds.
       pub reconnect_base_ms: u64,
       /// Reconnection maximum delay in milliseconds.
       pub reconnect_max_ms: u64,
   }

   impl Default for WitnessFeedConfig {
       fn default() -> Self {
           Self {
               chain_id: 1,
               ws_endpoint: String::new(),
               watched_addresses: Vec::new(),
               watched_topics: Vec::new(),
               max_backfill_gap: 1000,
               reconnect_base_ms: 500,
               reconnect_max_ms: 30_000,
           }
       }
   }

   /// Watch set tracking which addresses and topics to monitor.
   #[derive(Debug, Clone, Default)]
   pub struct WatchSet {
       pub addresses: HashSet<String>,
       pub topics: HashSet<String>,
   }

   /// Source of a watch entry (for audit/debugging).
   #[derive(Debug, Clone, PartialEq, Eq)]
   pub enum WatchSource {
       /// From initial configuration.
       Config,
       /// From a Trigger binding.
       TriggerBinding,
       /// From a Feed subscription.
       FeedSubscription,
       /// Learned from event patterns.
       Learned,
   }

   /// Chain event pulse ready for Bus publishing.
   #[derive(Debug, Clone)]
   pub struct ChainEventPulse {
       pub chain_id: u64,
       pub block_number: BlockNumber,
       pub block_hash: String,
       pub block_timestamp: u64,
       pub log: LogEntry,
       pub source: String, // "feed:chain:{chain_id}:blocks"
   }

   /// The Feed Cell: Connect + Trigger + Store.
   pub struct ChainWitnessFeed {
       id: CellId,
       config: WitnessFeedConfig,
       /// Underlying BlockObserver for event filtering.
       observer: parking_lot::RwLock<BlockObserver>,
       /// Watch set (may be mutated by Trigger bindings).
       watch_set: parking_lot::RwLock<WatchSet>,
       /// Graduated events stored for later query.
       event_store: parking_lot::RwLock<Vec<ObservedEvent>>,
       /// Whether the feed is currently connected.
       connected: std::sync::atomic::AtomicBool,
       /// Whether the trigger is armed.
       armed: std::sync::atomic::AtomicBool,
   }

   impl ChainWitnessFeed {
       pub fn new(id: CellId, config: WitnessFeedConfig) -> Self { ... }

       /// Process a block through the observer, producing events.
       pub fn ingest_block(&self, header: &ChainHeader, logs: &[LogEntry]) -> Vec<ChainEventPulse> {
           let mut observer = self.observer.write();
           let events = observer.process_block(header, logs);
           // Store graduated events
           let mut store = self.event_store.write();
           store.extend(events.iter().cloned());
           // Convert to pulses
           events.into_iter().map(|e| ChainEventPulse {
               chain_id: self.config.chain_id,
               block_number: e.block_number,
               block_hash: e.block_hash,
               block_timestamp: e.block_timestamp,
               log: e.log,
               source: format!("feed:chain:{}:blocks", self.config.chain_id),
           }).collect()
       }

       /// Check for gaps and return blocks that need backfilling.
       pub fn pending_gaps(&self) -> Vec<BlockNumber> {
           let observer = self.observer.read();
           let gaps = observer.pending_gaps();
           if gaps.len() as u64 > self.config.max_backfill_gap {
               // Too many gaps -- emit warning instead of backfill
               Vec::new()
           } else {
               gaps
           }
       }

       /// Add an address to the watch set.
       pub fn watch_address(&self, address: String, _source: WatchSource) {
           let mut ws = self.watch_set.write();
           ws.addresses.insert(address);
       }

       /// Add a topic to the watch set.
       pub fn watch_topic(&self, topic: String, _source: WatchSource) {
           let mut ws = self.watch_set.write();
           ws.topics.insert(topic);
       }
   }
   ```
   - Cell: `cell_name` = "chain-witness-feed", `protocols` = `&["Connect", "Trigger", "Store"]`
   - Connect: `connect()` sets connected=true (real WS connection is deferred to runtime integration), `health()` returns connected.load(), `disconnect()` sets connected=false
   - Trigger: `arm()` sets armed=true (enables block processing), `disarm()` sets armed=false
   - Store: `put()` stores an event engram, `get()` retrieves by hash, `query()` filters by block range/address/topic from event_store

5. Add module to lib.rs:
   ```rust
   pub mod witness_feed;
   pub use witness_feed::{
       ChainWitnessFeed, WitnessFeedConfig, WatchSet, WatchSource, ChainEventPulse,
   };
   ```

6. Write tests using mock infrastructure:
   - ingest_block with matching addresses produces ChainEventPulses
   - ingest_block with non-matching addresses produces empty vec
   - Gap detection triggers for gaps <= max_backfill_gap, suppressed for larger
   - WatchSet mutation: watch_address adds to set, subsequent ingest_block picks up new addresses
   - Connect lifecycle: connect() -> health()=true -> disconnect() -> health()=false
   - Trigger lifecycle: arm() -> ingest_block processes -> disarm() -> ingest_block skips (or still processes, depending on design)
   - Use test helpers from observer.rs (ChainHeader and LogEntry construction)

## Verification
```bash
cargo check -p roko-chain
cargo clippy -p roko-chain --no-deps -- -D warnings
cargo test -p roko-chain -- witness_feed
```

## What NOT to do
- Do NOT replace existing observer.rs or witness.rs -- this wraps BlockObserver as internal state
- Do NOT add real WebSocket dependencies unless already in roko-chain's Cargo.toml -- use synchronous ingest_block for unit tests
- Do NOT implement the triage Pipeline -- that is M139
- Do NOT implement actual Binary Fuse16 unless the xorf crate is already available -- use AddressFilter from observer.rs (HashSet-based, functionally equivalent)
- Do NOT implement actual Bus publishing -- just produce ChainEventPulse values that a runtime integration would publish
