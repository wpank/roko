# M081 — Implement Event Indexer

**[BLOCKED:depth]** -- This item depends on `tmp/unified-depth/18-registries/` depth docs and M076/M077 (deployed contracts with event schemas).

## Objective
Implement a chain event indexer that subscribes to contract events, normalizes them into Pulses on Bus, and stores them in PostgreSQL for query. A REST API exposes the indexed data. This is the read-only bridge between on-chain state and the local runtime, enabling the system to react to on-chain events.

## Scope
- Crates: `roko-chain`
- Files: `crates/roko-chain/src/indexer/` (new directory), `crates/roko-chain/src/lib.rs`
- Phase ref: `tmp/unified-migration/04-PHASE-3-ECONOMY.md` SS3.4
- Spec ref: `tmp/unified/18-ON-CHAIN-REGISTRIES.md` SS9

## Steps
1. Check for existing indexer code:
   ```bash
   grep -rn 'indexer\|Indexer\|index.*event' crates/roko-chain/src/ --include='*.rs' | head -10
   ```

2. Implement the indexer in `crates/roko-chain/src/indexer/mod.rs`:
   ```rust
   pub struct EventIndexer {
       provider: Arc<Provider>,
       contracts: Vec<(Address, String)>,  // (address, contract_name)
       bus: Arc<dyn Bus>,
       db: PgPool,
       last_block: u64,
   }

   impl EventIndexer {
       pub async fn new(config: IndexerConfig, bus: Arc<dyn Bus>) -> Result<Self>;
       pub async fn run(&mut self, cancel: CancellationToken) -> Result<()>;
       pub async fn index_block(&mut self, block_number: u64) -> Result<usize>;
   }
   ```

3. Event normalization:
   - Each contract event type maps to a Pulse topic: `chain:{contract}:{event_name}`
   - Event data is decoded into structured JSON
   - Pulse carries: block number, tx hash, log index, decoded data

4. PostgreSQL storage:
   ```sql
   CREATE TABLE chain_events (
       id SERIAL PRIMARY KEY,
       block_number BIGINT NOT NULL,
       tx_hash TEXT NOT NULL,
       contract_address TEXT NOT NULL,
       event_name TEXT NOT NULL,
       data JSONB NOT NULL,
       indexed_at TIMESTAMPTZ DEFAULT NOW()
   );
   ```

5. REST API for querying indexed data:
   ```rust
   // GET /api/chain/events?contract=<name>&event=<name>&from_block=<n>
   // GET /api/chain/events/{tx_hash}
   // GET /api/chain/stats  (indexed blocks, event counts)
   ```

6. Write tests:
   - Contract event emitted -> Pulse on Bus -> queryable via REST
   - Indexer resumes from last processed block on restart
   - Event data is correctly decoded and stored

## Verification
```bash
cargo check -p roko-chain
cargo clippy -p roko-chain --no-deps -- -D warnings
cargo test -p roko-chain -- indexer
```

## What NOT to do
- Do NOT write to chain from the indexer -- it is strictly read-only
- Do NOT implement reorg handling in this batch -- simple linear indexing is sufficient
- Do NOT require PostgreSQL for tests -- use SQLite or mock for unit tests
- Do NOT proceed without contract ABIs and event schemas from depth docs
