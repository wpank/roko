# M134 — Verifiable HDC Verify Cells (ZK, Optimistic, TEE, Binius)

**[BLOCKED:chain]** -- Requires M133 (HdcPrecompileCell), M131 (ChainConnector), M008 (Verify protocol). Heavily blocked on external infrastructure (ZK verifier contracts, TEE attestation, Binius prover).

## Objective
Define four interchangeable Verify Cells for HDC computation proofs, plus a Route Cell that selects among them. Each Verify Cell takes a claim ("vector A has similarity 0.73 to vector B") and produces a Verdict (pass/fail with proof). The four approaches trade off gas cost, latency, and trust model. A Route Cell selects the cheapest strategy that meets the claim's trust requirements.

## Scope
- Crates: `roko-chain`
- Files:
  - `crates/roko-chain/src/hdc_verify.rs` (new)
  - `crates/roko-chain/src/lib.rs` (add module + re-exports)
- Depth doc: `tmp/unified-depth/18-registries/02-hdc-on-chain-and-verification.md` SS4-5

## Steps
1. Verify the Verify and Route traits exist:
   ```bash
   grep -rn 'pub trait Verify' crates/roko-core/src/traits.rs
   grep -rn 'pub trait Route' crates/roko-core/src/traits.rs
   grep -rn 'Verdict' crates/roko-core/src/ --include='*.rs' | head -10
   ```
   **Expected**: `Verify` at `traits.rs:214` (async: `verify(&Engram, &Context) -> Verdict`, `name() -> &str`). `Route` at `traits.rs:242` (sync: `select(&[Engram], &Context) -> Option<Selection>`, `feedback(&Outcome)`, `name() -> &str`). `Verdict` is defined in roko-core (`verdict.rs:51`, struct with `passed: bool`, `reason: String`, `gate: String`, `score: f32`, `detail: Option<String>`, etc.).

2. Verify Cell and HdcPrecompileCell from M133:
   ```bash
   grep -rn 'pub trait Cell' crates/roko-core/src/cell.rs
   grep -rn 'pub struct HdcPrecompileCell' crates/roko-chain/src/hdc_precompile.rs
   ```

3. Create `crates/roko-chain/src/hdc_verify.rs`:

   **Shared types**:
   ```rust
   use roko_core::ContentHash;
   use roko_primitives::HdcVector;

   /// An HDC computation claim to be verified.
   #[derive(Debug, Clone)]
   pub struct HdcClaim {
       /// The operation that was performed.
       pub operation: HdcOperation,
       /// Content hashes of the input vectors.
       pub input_hashes: Vec<ContentHash>,
       /// The claimed output.
       pub output: HdcClaimOutput,
   }

   /// HDC operation types.
   #[derive(Debug, Clone, Copy, PartialEq, Eq)]
   pub enum HdcOperation {
       Similarity,
       Bind,
       Bundle,
       TopK { k: usize },
   }

   /// Claimed output of an HDC operation.
   #[derive(Debug, Clone)]
   pub enum HdcClaimOutput {
       Similarity(f32),
       Vector(HdcVector),
       TopK(Vec<(ContentHash, f32)>),
   }

   /// Trust model for verification.
   #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
   pub enum TrustLevel {
       /// Economically secured (optimistic, slashable bond).
       Economic,
       /// Hardware-secured (TEE enclave attestation).
       Hardware,
       /// Cryptographically secured (ZK/Binius proofs).
       Cryptographic,
   }

   /// Verification strategy metadata for the router.
   #[derive(Debug, Clone)]
   pub struct VerifyStrategy {
       /// Human name of the strategy.
       pub name: &'static str,
       /// Approximate gas cost per verification.
       pub gas_cost: u64,
       /// Approximate latency in milliseconds.
       pub latency_ms: u64,
       /// Trust model provided.
       pub trust_level: TrustLevel,
   }
   ```

   **Four Verify Cells** (all implement Cell + Verify):
   ```rust
   use crate::connector::ChainConnector;
   use roko_core::cell::{Cell, CellId};
   use roko_core::traits::Verify;
   use roko_core::{Context, Engram, Verdict};

   /// ZK proof verification via on-chain verifier contract. ~250K gas.
   pub struct ZkHdcVerifyCell {
       id: CellId,
       connector: Arc<ChainConnector>,
       verifier_address: String,
   }

   /// Optimistic verification: post claim, wait challenge window (100 blocks). ~3K gas.
   pub struct OptimisticHdcVerifyCell {
       id: CellId,
       connector: Arc<ChainConnector>,
       challenge_window_blocks: u64,
   }

   /// TEE attestation verification via on-chain verifier. ~3K gas.
   pub struct TeeHdcVerifyCell {
       id: CellId,
       connector: Arc<ChainConnector>,
       tee_verifier_address: String,
   }

   /// Binius (binary-field STARK) proof verification. ~150K gas.
   pub struct BiniusHdcVerifyCell {
       id: CellId,
       connector: Arc<ChainConnector>,
       binius_verifier_address: String,
   }
   ```
   Each Cell impl: `cell_name()`, `protocols()` = `&["Verify"]`. Each Verify impl: `verify(engram, ctx) -> Verdict` extracts HdcClaim from engram metadata, encodes verification calldata, calls `connector.eth_call()` on the verifier contract, decodes boolean result, returns `Verdict { passed, gate: self.name(), .. }`. `name()` returns strategy name.

   **HdcVerificationRouter** (Cell + Route):
   ```rust
   /// Routes to the cheapest verification strategy that meets constraints.
   pub struct HdcVerificationRouter {
       id: CellId,
       strategies: Vec<(VerifyStrategy, Arc<dyn Verify + 'static>)>,
   }

   impl HdcVerificationRouter {
       pub fn new(id: CellId) -> Self { ... }
       pub fn add_strategy(&mut self, strategy: VerifyStrategy, cell: Arc<dyn Verify + 'static>) { ... }
   }
   ```
   - Route: `select(candidates, ctx)` filters strategies by gas_budget and min_trust_level from context metadata, selects cheapest eligible, tie-breaks by trust level (prefer stronger)

4. Add module to lib.rs:
   ```rust
   pub mod hdc_verify;
   pub use hdc_verify::{
       HdcClaim, HdcClaimOutput, HdcOperation, TrustLevel, VerifyStrategy,
       ZkHdcVerifyCell, OptimisticHdcVerifyCell, TeeHdcVerifyCell, BiniusHdcVerifyCell,
       HdcVerificationRouter,
   };
   ```

5. Write unit tests:
   - HdcClaim round-trip: construct -> serialize fields -> reconstruct
   - ZkHdcVerifyCell returns Verdict { passed: true } when mock eth_call returns `[0x01]`
   - ZkHdcVerifyCell returns Verdict { passed: false } when mock eth_call returns `[0x00]`
   - HdcVerificationRouter selects cheapest eligible strategy (OptimisticHdcVerifyCell at 3K gas before ZkHdcVerifyCell at 250K gas)
   - HdcVerificationRouter returns None when no strategy fits (e.g., gas_budget < 3K)
   - Use MockChainClient.with_call_result() for canned verifier responses

## Verification
```bash
cargo check -p roko-chain
cargo clippy -p roko-chain --no-deps -- -D warnings
cargo test -p roko-chain -- hdc_verify
```

## What NOT to do
- Do NOT implement actual ZK proof generation -- that is off-chain infrastructure
- Do NOT implement TEE enclave code -- only the verification calldata encoding
- Do NOT implement Binius prover -- only the on-chain verification path
- Do NOT add RISC Zero / SP1 / Binius dependencies -- encode calldata manually
