# M141 — ISFR Score Cell + ClearingHouse Pipeline + Settlement Verify

**[BLOCKED:chain]** -- Requires M140 (Payment Connect Cells), M132 (ReputationStoreCell), M131 (ChainConnector). Chain deployment is Tier 6.

## Objective
Implement three components that complete the payment and settlement layer: (1) `IsfrScoreCell` -- a Score Cell that computes the Individual Service-Flow Reliability score from payment history, (2) `ClearingHousePipeline` -- a set of Cells that net bilateral obligations and produce a minimal settlement set, and (3) `SettlementVerifyCell` -- a Verify Cell that validates settlement transactions against expected amounts. Together these enable efficient multi-party clearing with trust-weighted batching.

## Scope
- Crates: `roko-chain`
- Files:
  - `crates/roko-chain/src/clearing.rs` (new)
  - `crates/roko-chain/src/lib.rs` (add module + re-exports)
- Depth doc: `tmp/unified-depth/18-registries/06-payments-and-settlement.md` SS3-5

## Steps
1. Check existing ISFR implementation:
   ```bash
   grep -rn 'pub struct IsfrConfig\|pub struct IsfrRegistry\|pub enum ClearingPhase\|pub fn weighted_median' crates/roko-chain/src/isfr.rs
   ```
   **Expected**: `IsfrConfig` at `isfr.rs:29` (fields: `epoch_duration_secs: u64` (default 28800 = 8h), `max_kkt_residual: f64`, `min_submissions_for_clearing: usize`, `min_reputation: f64`, `max_rate_bound: Option<f64>`, `outlier_sigma: f64`). `ClearingPhase` at `isfr.rs:69` (enum: `Commit`, `Reveal`, `Solve`, `Certificate`, `Verify`, `Settle`). The ISFR system computes a weighted median rate from agent submissions with 3-sigma outlier exclusion.

2. Verify existing phase2 types used by ISFR:
   ```bash
   grep -rn 'pub struct FactClaim\|pub struct ClearingCertificate\|pub struct Allocation\|pub enum FactValue' crates/roko-chain/src/phase2.rs | head -10
   ```
   **Expected**: `FactClaim` and `ClearingCertificate` in phase2.rs. These are used by the legacy QP solver path in isfr.rs.

3. Verify Cell, Score, Store, Compose, Route, and Verify protocol traits:
   ```bash
   grep -rn 'pub trait Cell' crates/roko-core/src/cell.rs
   grep -rn 'pub trait Score' crates/roko-core/src/traits.rs
   grep -rn 'pub trait Store' crates/roko-core/src/traits.rs
   grep -rn 'pub trait Compose' crates/roko-core/src/traits.rs
   grep -rn 'pub trait Route' crates/roko-core/src/traits.rs
   grep -rn 'pub trait Verify' crates/roko-core/src/traits.rs
   ```
   **Expected**: `Cell` at `cell.rs:14`. `Score` at `traits.rs:167` (sync: `score(&Engram, &Context) -> ScoreValue`). `Store` at `traits.rs:37` (async: `put`, `get`, `query`, `query_similar`). `Compose` at `traits.rs:285` (sync: `compose(&[Engram], &Budget, &dyn Score, &Context) -> Result<Engram>`). `Route` at `traits.rs:242` (sync: `select`, `feedback`, `name`). `Verify` at `traits.rs:214` (async: `verify(&Engram, &Context) -> Verdict`).

4. Verify phase2 DisputeLevel for multi-level dispute:
   ```bash
   grep -rn 'pub enum DisputeLevel\|pub enum DisputeOutcome' crates/roko-chain/src/phase2.rs
   ```
   **Expected**: `DisputeLevel` (enum: `Automated`, `Arbiter`, `Court`, `DaoVote`). `DisputeOutcome` (enum with variants for each resolution type).

5. Create `crates/roko-chain/src/clearing.rs`:

   **IsfrScoreCell** (Cell + Score):
   ```rust
   use crate::phase2::u256;
   use roko_core::cell::{Cell, CellId};
   use roko_core::score::Score as ScoreValue;
   use roko_core::traits::Score;
   use roko_core::{Context, Engram};

   /// Individual payment history record for ISFR computation.
   #[derive(Debug, Clone)]
   pub struct PaymentHistory {
       /// Agent passport ID.
       pub passport_id: u256,
       /// Total payment attempts.
       pub total_attempts: u64,
       /// Successful completions.
       pub successful: u64,
       /// On-time completions (within agreed deadline).
       pub on_time: u64,
       /// Number of disputes initiated.
       pub disputes: u64,
       /// Variance of payment amounts (for consistency scoring).
       pub amount_variance: f64,
   }

   /// ISFR breakdown for transparency.
   #[derive(Debug, Clone)]
   pub struct IsfrBreakdown {
       /// Payment completion rate = successful / total_attempts.
       pub completion_rate: f64,
       /// Timeliness = on_time / successful.
       pub timeliness: f64,
       /// Dispute rate = disputes / successful.
       pub dispute_rate: f64,
       /// Value consistency = 1.0 - normalized_variance.
       pub value_consistency: f64,
       /// Composite ISFR score in [0.0, 1.0].
       pub composite: f64,
   }

   impl IsfrBreakdown {
       /// Compute composite ISFR from the 4 factors.
       /// Weights: completion(0.35) + timeliness(0.25) + (1-dispute_rate)(0.25) + consistency(0.15)
       pub fn compute(history: &PaymentHistory) -> Self {
           let completion_rate = if history.total_attempts > 0 {
               history.successful as f64 / history.total_attempts as f64
           } else { 0.0 };
           let timeliness = if history.successful > 0 {
               history.on_time as f64 / history.successful as f64
           } else { 0.0 };
           let dispute_rate = if history.successful > 0 {
               history.disputes as f64 / history.successful as f64
           } else { 0.0 };
           let value_consistency = (1.0 - history.amount_variance.min(1.0)).max(0.0);

           let composite = 0.35 * completion_rate
               + 0.25 * timeliness
               + 0.25 * (1.0 - dispute_rate).max(0.0)
               + 0.15 * value_consistency;

           Self { completion_rate, timeliness, dispute_rate, value_consistency, composite: composite.clamp(0.0, 1.0) }
       }
   }

   /// Score Cell computing ISFR from payment history.
   pub struct IsfrScoreCell {
       id: CellId,
       /// Payment histories keyed by passport_id.
       histories: parking_lot::RwLock<std::collections::HashMap<u256, PaymentHistory>>,
   }

   impl IsfrScoreCell {
       pub fn new(id: CellId) -> Self { ... }
       /// Record a payment event for an agent.
       pub fn record_payment(&self, passport_id: u256, successful: bool, on_time: bool, amount: f64) { ... }
       /// Record a dispute for an agent.
       pub fn record_dispute(&self, passport_id: u256) { ... }
       /// Get the full ISFR breakdown for an agent.
       pub fn breakdown(&self, passport_id: u256) -> IsfrBreakdown { ... }
   }
   ```
   - Cell: `cell_name` = "isfr-score", `protocols` = `&["Score"]`
   - Score: `score(engram, ctx)` extracts `passport_id` from engram metadata, looks up PaymentHistory, computes `IsfrBreakdown::compute()`, returns composite as ScoreValue

   **ClearingHouse Pipeline Cells**:

   ```rust
   /// A bilateral obligation between two agents.
   #[derive(Debug, Clone)]
   pub struct Obligation {
       /// Debtor passport ID.
       pub from: u256,
       /// Creditor passport ID.
       pub to: u256,
       /// Amount owed.
       pub amount: u128,
       /// Source reference (e.g. job flow_id).
       pub reference: roko_core::ContentHash,
   }

   /// Net position after bilateral netting.
   #[derive(Debug, Clone)]
   pub struct NetPosition {
       /// Payer (net debtor).
       pub from: u256,
       /// Payee (net creditor).
       pub to: u256,
       /// Net amount owed.
       pub net_amount: u128,
   }

   /// Settlement batch grouped by trust tier.
   #[derive(Debug, Clone)]
   pub struct SettlementBatch {
       /// Batch identifier.
       pub batch_id: u32,
       /// Trust tier of this batch (high-ISFR agents clear faster).
       pub min_isfr: f64,
       /// Net positions in this batch.
       pub positions: Vec<NetPosition>,
       /// Total value to settle.
       pub total_value: u128,
   }

   /// Stage 1: Collect pending obligations.
   pub struct ObligationCollectCell {
       id: CellId,
       obligations: parking_lot::RwLock<Vec<Obligation>>,
   }

   /// Stage 2: Bilateral netting -- reduce pairs to net positions.
   pub struct BilateralNettingCell {
       id: CellId,
   }

   /// Stage 3: Multilateral netting -- detect cycles and further reduce.
   pub struct MultilateralNettingCell {
       id: CellId,
   }

   /// Stage 4: Group into settlement batches by ISFR trust tier.
   pub struct SettlementBatchCell {
       id: CellId,
       /// ISFR thresholds for batch tiers.
       tier_thresholds: Vec<f64>, // e.g. [0.8, 0.5, 0.0]
   }
   ```
   - ObligationCollectCell (Cell + Store): `put()` adds obligation, `query()` returns all pending
   - BilateralNettingCell (Cell + Compose): `compose()` takes obligation engrams, computes net positions per pair (A owes B 100, B owes A 60 -> net: A owes B 40), returns engram with net positions
   - MultilateralNettingCell (Cell + Compose): `compose()` detects cycles in obligation graph via DFS, reduces cycle amounts (A->B->C->A: reduce by min amount in cycle), returns further reduced positions
   - SettlementBatchCell (Cell + Route): `select()` groups positions by ISFR tier and returns settlement batches

   **SettlementVerifyCell** (Cell + Verify):
   ```rust
   /// A settlement claim to verify.
   #[derive(Debug, Clone)]
   pub struct SettlementClaim {
       /// Payer.
       pub from: u256,
       /// Payee.
       pub to: u256,
       /// Expected amount.
       pub expected_amount: u128,
       /// Transaction hash of the settlement.
       pub tx_hash: String,
   }

   /// Multi-level dispute resolution.
   #[derive(Debug, Clone, Copy, PartialEq, Eq)]
   pub enum SettlementDisputeLevel {
       /// Level 1: Automated re-check (re-run netting, verify arithmetic).
       Automated,
       /// Level 2: Arbiter (selected by ReputationStore, Sovereign+ tier).
       Arbiter,
       /// Level 3: Court (3-juror panel, same mechanism as job dispute in M136).
       Court,
       /// Level 4: DAO vote (reserved, not implemented).
       DaoVote,
   }

   pub struct SettlementVerifyCell {
       id: CellId,
       connector: Option<Arc<crate::connector::ChainConnector>>,
   }
   ```
   - Cell: `cell_name` = "settlement-verify", `protocols` = `&["Verify"]`
   - Verify: `verify(engram, ctx) -> Verdict` extracts SettlementClaim from engram, queries ChainConnector for tx receipt (if available), checks amount matches expected. Returns `Verdict { passed: true }` if match, `Verdict { passed: false }` with amount mismatch message if not.
   - Without ChainConnector (None), verifies arithmetic only (Level 1 automated check)

6. Add module to lib.rs:
   ```rust
   pub mod clearing;
   pub use clearing::{
       IsfrScoreCell, IsfrBreakdown, PaymentHistory,
       Obligation, NetPosition, SettlementBatch,
       ObligationCollectCell, BilateralNettingCell, MultilateralNettingCell, SettlementBatchCell,
       SettlementClaim, SettlementDisputeLevel, SettlementVerifyCell,
   };
   ```

7. Write tests:
   - IsfrBreakdown::compute: all perfect (1.0 completion, 1.0 timeliness, 0 disputes, 0 variance) -> composite = 1.0
   - IsfrBreakdown::compute: 50% completion, 50% timeliness -> composite ~0.475
   - Bilateral netting: A owes B 100, B owes A 60 -> net: A owes B 40
   - Bilateral netting: symmetric obligations cancel out (net = 0, no position emitted)
   - Multilateral netting: cycle A->B(100)->C(80)->A(50) reduces by 50 (min in cycle)
   - SettlementVerifyCell passes when mock receipt amount matches expected
   - SettlementVerifyCell fails with mismatch message when amounts differ
   - Trust-weighted batching: agent with ISFR 0.9 grouped in high tier, agent with ISFR 0.3 in low tier

## Verification
```bash
cargo check -p roko-chain
cargo clippy -p roko-chain --no-deps -- -D warnings
cargo test -p roko-chain -- clearing
```

## What NOT to do
- Do NOT replace existing isfr.rs -- wrap its types and config in the Cell interface; the IsfrScoreCell adds a new scoring layer on top
- Do NOT implement DAO-level dispute resolution -- mark as `todo!("DAO vote not implemented")`
- Do NOT implement actual on-chain settlement -- use mock ChainConnector or None path
- Do NOT add graph algorithm dependencies -- implement simple cycle detection via DFS for multilateral netting
