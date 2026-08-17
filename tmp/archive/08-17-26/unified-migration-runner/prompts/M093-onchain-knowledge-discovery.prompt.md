# M093 — On-Chain Knowledge Discovery

**[BLOCKED:depth]** -- This item depends on M078 (Rust chain clients) and M076/M077 (deployed InsightStore contract). Also requires the HDC precompile to be available on the target chain.

## Objective
Implement on-chain knowledge discovery: query the InsightStore for Signals similar to a local query using HDC fingerprint similarity (via the HTC precompile). Download matching Signals and import them with provenance tracking (on-chain source). This extends cross-agent knowledge sharing from workspace-local (M092) to global.

## Scope
- Crates: `roko-chain`
- Files: `crates/roko-chain/src/knowledge_discovery.rs` (new), `crates/roko-chain/src/lib.rs`
- Phase ref: `tmp/unified-migration/04-PHASE-3-ECONOMY.md` SS3.7

## Steps
1. Read the InsightStore client from M078:
   ```bash
   grep -rn 'InsightStore\|insight.*client\|query.*insight' crates/roko-chain/src/ --include='*.rs' | head -10
   ```

2. Implement discovery in `crates/roko-chain/src/knowledge_discovery.rs`:
   ```rust
   pub struct OnChainDiscovery {
       client: Arc<InsightStoreClient>,
       similarity_threshold: f64,
   }

   pub struct DiscoveryResult {
       pub signal_id: String,
       pub similarity: f64,
       pub publisher: String,
       pub chain_provenance: ChainProvenance,
   }

   pub struct ChainProvenance {
       pub contract_address: String,
       pub tx_hash: String,
       pub block_number: u64,
   }

   impl OnChainDiscovery {
       /// Query InsightStore for Signals similar to the given HDC vector.
       pub async fn discover(&self, query_fingerprint: &[f32], top_k: usize) -> Result<Vec<DiscoveryResult>>;
       /// Download a discovered Signal's full metadata.
       pub async fn fetch(&self, signal_id: &str) -> Result<ExportedSignal>;
       /// Import with chain provenance tracking.
       pub async fn import(&self, signal: ExportedSignal, provenance: ChainProvenance) -> Result<()>;
   }
   ```

3. Write tests:
   - Query InsightStore with HDC vector -> get matching Signals
   - Import with chain provenance is tracked
   - Below-threshold results are filtered out

## Verification
```bash
cargo check -p roko-chain
cargo clippy -p roko-chain --no-deps -- -D warnings
cargo test -p roko-chain -- knowledge_discovery
```

## What NOT to do
- Do NOT proceed without M078 chain clients and deployed InsightStore
- Do NOT implement the HDC precompile -- it must be available on-chain already
- Do NOT import without provenance tracking -- all chain-sourced knowledge must be attributed
- Do NOT query on-chain for every knowledge lookup -- use local store first, chain as fallback
