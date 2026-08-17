# M133 — HDC Precompile Cell (On-Chain HDC Operations)

**[BLOCKED:chain]** -- Requires M131 (ChainConnector), M012 (Cell trait), and a deployed HDC precompile at 0xA01. Chain deployment is Tier 6.

## Objective
Implement `HdcPrecompileCell` -- a Cell that wraps the HDC native precompile (at address 0xA01 on Korai/Mirage) behind the Cell + Compose + Score protocols. This enables on-chain HDC operations (similarity, bind, bundle, top-K search) at 25-50x less gas than Solidity equivalents. Also implement the three-tier on-chain search Pipeline: BloomFilterCell -> ApproximateSearchCell -> ExactSearchCell.

## Scope
- Crates: `roko-chain`, `roko-primitives`
- Files:
  - `crates/roko-chain/src/hdc_precompile.rs` (new)
  - `crates/roko-chain/src/lib.rs` (add module + re-exports)
- Depth doc: `tmp/unified-depth/18-registries/02-hdc-on-chain-and-verification.md`

## Steps
1. Verify the existing HdcVector type in roko-primitives:
   ```bash
   grep -rn 'pub struct HdcVector' crates/roko-primitives/src/hdc.rs
   grep -rn 'pub fn similarity\|pub fn bind\|pub fn bundle' crates/roko-primitives/src/hdc.rs
   ```
   **Expected**: `HdcVector` at `crates/roko-primitives/src/hdc.rs:30` (a struct with `bits: [u64; 160]` -- 10240 bits packed into 160 u64 words = 1280 bytes). Methods: `similarity(&self, other: &Self) -> f32` at line ~223, `bind(&self, other: &Self) -> Self` at line ~113, `bundle(vectors: &[&Self]) -> Self` at line ~129.

2. Verify ChainConnector from M131 exists (or will exist after M131):
   ```bash
   grep -rn 'pub struct ChainConnector' crates/roko-chain/src/connector.rs
   ```
   **Note**: This file won't exist until M131 runs. Code should compile with ChainConnector behind a cfg gate or via a trait bound.

3. Verify Cell, Compose, and Score protocol traits:
   ```bash
   grep -rn 'pub trait Cell' crates/roko-core/src/cell.rs
   grep -rn 'pub trait Compose' crates/roko-core/src/traits.rs
   grep -rn 'pub trait Score' crates/roko-core/src/traits.rs
   ```
   **Expected**: `Cell` at `cell.rs:14`. `Compose` at `traits.rs:285` (sync: `compose(&[Engram], &Budget, &dyn Score, &Context) -> Result<Engram>`). `Score` at `traits.rs:167` (sync: `score(&Engram, &Context) -> ScoreValue`).

4. Create `crates/roko-chain/src/hdc_precompile.rs`:

   **Calldata encoding** (compact binary, not ABI):
   ```rust
   /// Precompile selectors for HDC operations at address 0xA01.
   pub const SELECTOR_SIMILARITY: u8 = 0x01;
   pub const SELECTOR_BIND: u8 = 0x02;
   pub const SELECTOR_BUNDLE: u8 = 0x03;
   pub const SELECTOR_TOPK: u8 = 0x04;

   /// Size of a serialized HdcVector in bytes (160 u64 words * 8 bytes = 1280 bytes).
   pub const HDC_VECTOR_BYTES: usize = 1280; // bits: [u64; 160] = 1280 bytes

   /// Encode a similarity call: selector(1) + vec_a(1280) + vec_b(1280).
   pub fn encode_similarity_call(a: &HdcVector, b: &HdcVector) -> Vec<u8>;
   /// Encode a bind call: selector(1) + vec_a(1280) + vec_b(1280).
   pub fn encode_bind_call(a: &HdcVector, b: &HdcVector) -> Vec<u8>;
   /// Encode a bundle call: selector(1) + count(2 LE) + N * vec(1280).
   pub fn encode_bundle_call(vectors: &[&HdcVector]) -> Vec<u8>;
   /// Encode a top-K call: selector(1) + query(1280) + k(2 LE).
   pub fn encode_topk_call(query: &HdcVector, k: u16) -> Vec<u8>;
   /// Decode similarity result: 4 bytes LE f32.
   pub fn decode_similarity_result(data: &[u8]) -> Option<f32>;
   /// Decode vector result: 1280 bytes -> HdcVector.
   pub fn decode_vector_result(data: &[u8]) -> Option<HdcVector>;
   /// Decode top-K result: count(2 LE) + N * (hash(32) + score(4 LE f32)).
   pub fn decode_topk_result(data: &[u8]) -> Vec<(roko_core::ContentHash, f32)>;
   ```

   **HdcPrecompileCell** (Cell + Score):
   ```rust
   use crate::connector::ChainConnector;
   use crate::types::TxRequest;
   use roko_core::cell::{Cell, CellId};
   use roko_core::traits::Score;
   use roko_primitives::HdcVector;

   /// The precompile address on Korai/Mirage.
   pub const HDC_PRECOMPILE_ADDRESS: &str = "0x0000000000000000000000000000000000000A01";

   pub struct HdcPrecompileCell {
       id: CellId,
       connector: Arc<ChainConnector>,
       precompile_address: String,
   }

   impl HdcPrecompileCell {
       pub fn new(id: CellId, connector: Arc<ChainConnector>) -> Self { ... }

       /// Compute similarity between two vectors via the on-chain precompile.
       pub async fn similarity(&self, a: &HdcVector, b: &HdcVector) -> crate::types::ChainResult<f32> {
           let calldata = encode_similarity_call(a, b);
           let request = TxRequest {
               to: Some(self.precompile_address.clone()),
               data: calldata,
               ..Default::default()
           };
           let result = self.connector.eth_call(&request, None).await?;
           decode_similarity_result(&result.output)
               .ok_or_else(|| crate::types::ChainError::Rpc("invalid similarity result".into()))
       }

       /// Bind two vectors via the on-chain precompile.
       pub async fn bind(&self, a: &HdcVector, b: &HdcVector) -> crate::types::ChainResult<HdcVector> { ... }
       /// Bundle vectors via the on-chain precompile.
       pub async fn bundle(&self, vectors: &[&HdcVector]) -> crate::types::ChainResult<HdcVector> { ... }
       /// Top-K search via the on-chain precompile.
       pub async fn topk(&self, query: &HdcVector, k: u16) -> crate::types::ChainResult<Vec<(roko_core::ContentHash, f32)>> { ... }
   }
   ```
   - Cell: `cell_id`, `cell_name` = "hdc-precompile", `protocols` = `&["Score"]`
   - Score: `score(engram, ctx)` extracts HDC fingerprint from engram metadata, computes similarity against a reference vector, returns ScoreValue

   **Three-tier search Cells** (all Cell + Score):
   ```rust
   /// Tier 0: Bloom filter pre-screen. Rejects definite misses cheaply.
   pub struct BloomFilterCell { id: CellId, bloom_bits: Vec<u64> }
   /// Tier 1: Approximate search -- compare first 128 bytes (16 words) of each vector.
   pub struct ApproximateSearchCell { id: CellId, connector: Arc<ChainConnector> }
   /// Tier 2: Exact search -- full 1280-byte similarity via precompile.
   pub struct ExactSearchCell { id: CellId, precompile: Arc<HdcPrecompileCell> }
   ```

5. Add module to lib.rs:
   ```rust
   pub mod hdc_precompile;
   pub use hdc_precompile::{HdcPrecompileCell, BloomFilterCell, ApproximateSearchCell, ExactSearchCell};
   ```

6. Write unit tests using mock ChainConnector:
   - Calldata encoding round-trip: encode_similarity_call -> decode_similarity_result produces original values
   - encode_bind_call produces correct selector + vector bytes
   - encode_bundle_call serializes count header + N vectors
   - Three-tier pipeline: Bloom rejects definite misses, approximate narrows, exact re-ranks
   - Use `MockChainClient` from `crates/roko-chain/src/mock.rs` with `with_call_result()` to provide canned precompile output

## Verification
```bash
cargo check -p roko-chain -p roko-primitives
cargo clippy -p roko-chain --no-deps -- -D warnings
cargo test -p roko-chain -- hdc_precompile
```

## What NOT to do
- Do NOT modify roko-primitives/src/hdc.rs -- the off-chain HdcVector stays as-is
- Do NOT implement the actual EVM precompile (that is Solidity/C code on the chain itself)
- Do NOT add real RPC calls -- test with MockChainClient.with_call_result() only
- Do NOT implement the ChainHdcIndex management (register/remove) -- focus on query operations
