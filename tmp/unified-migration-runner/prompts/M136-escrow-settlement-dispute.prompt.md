# M136 — Escrow, Settlement, and Dispute Resolution Cells

**[BLOCKED:chain]** -- Requires M135 (Job marketplace graph types), M131 (ChainConnector). Chain deployment is Tier 6.

## Objective
Complete the job marketplace Pipeline with EscrowStoreCell (locked-balance semantics), SettlementCell (release + fee deduction + reputation attestation), and the DisputeVerifyPipeline (evidence collection, VRF jury selection, reputation-weighted verdict). This batch finishes the end-to-end job lifecycle as a Graph of Cells.

## Scope
- Crates: `roko-chain`
- Files:
  - `crates/roko-chain/src/job_settlement.rs` (new)
  - `crates/roko-chain/src/lib.rs` (add module + re-exports)
- Depth doc: `tmp/unified-depth/18-registries/03-job-market-and-hiring.md` SS6-9

## Steps
1. Verify types from M135 and existing marketplace:
   ```bash
   grep -rn 'pub struct EscrowEntry\|pub enum JobState' crates/roko-chain/src/marketplace.rs
   grep -rn 'pub struct SporeJobPosting\|pub enum HiringModel' crates/roko-chain/src/phase2.rs
   grep -rn 'pub struct DisputeResolution\|pub enum DisputeLevel\|pub enum DisputeOutcome' crates/roko-chain/src/phase2.rs
   ```
   **Expected**: `EscrowEntry` at `marketplace.rs:88` (fields: `job_id`, `poster_passport_id`, `locked_amount`, `marketplace_fee_bps`, `locked_block`). `DisputeResolution` at `phase2.rs` with `level: DisputeLevel` and `outcome: DisputeOutcome`. `DisputeLevel` enum with `Automated`, `Arbiter`, `Court`, `DaoVote`. `DisputeOutcome` enum with `Resolved`, `EscalatedToArbiter`, `EscalatedToCourt`, `SlashedAndRefunded`, `DaoDecision`.

2. Verify Cell, Store, Verify, and Route protocol traits:
   ```bash
   grep -rn 'pub trait Cell' crates/roko-core/src/cell.rs
   grep -rn 'pub trait Store' crates/roko-core/src/traits.rs
   grep -rn 'pub trait Verify' crates/roko-core/src/traits.rs
   grep -rn 'pub trait Route' crates/roko-core/src/traits.rs
   ```
   **Expected**: `Cell` at `cell.rs:14`. `Store` at `traits.rs:37` (async: `put`, `get`, `query`, `query_similar`). `Verify` at `traits.rs:214` (async: `verify(&Engram, &Context) -> Verdict`, `name() -> &str`). `Route` at `traits.rs:242` (sync: `select`, `feedback`, `name`).

3. Verify existing slash config in reputation registry:
   ```bash
   grep -rn 'pub enum ReputationViolation\|slash_rate' crates/roko-chain/src/reputation_registry.rs
   ```
   **Expected**: `ReputationViolation` at `reputation_registry.rs:78` (enum: `MissedDeadline`, `AbandonedJob`, `QualityRejection`, `RepeatedQualityFailure`, `Plagiarism`, `ResultManipulation`, `TeeViolation`, `Collusion`). Each variant has a score penalty (not stake %): MissedDeadline -1%, AbandonedJob -3%, QualityRejection -2%, RepeatedQualityFailure -5%, Plagiarism -10%, ResultManipulation -10%, TeeViolation -10%, Collusion dilutes feedback weight.

4. Create `crates/roko-chain/src/job_settlement.rs`:

   **EscrowStoreCell** (Cell + Store):
   ```rust
   use crate::phase2::u256;
   use roko_core::cell::{Cell, CellId};
   use roko_core::traits::Store;

   /// State of a locked escrow.
   #[derive(Debug, Clone, Copy, PartialEq, Eq)]
   pub enum EscrowLockState {
       /// Budget is locked; work is in progress.
       Locked,
       /// Budget released to assignee after successful settlement.
       Released,
       /// Budget refunded to poster after failed settlement or expiry.
       Refunded,
       /// Budget frozen pending dispute resolution.
       Frozen,
   }

   /// Escrow state for a single job flow.
   #[derive(Debug, Clone)]
   pub struct EscrowState {
       /// Flow ID (job content hash).
       pub flow_id: roko_core::ContentHash,
       /// Amount locked in escrow (KORAI base units).
       pub locked_amount: u128,
       /// Poster passport ID.
       pub poster_id: u256,
       /// Assignee passport ID.
       pub assignee_id: u256,
       /// Marketplace fee in basis points.
       pub marketplace_fee_bps: u16,
       /// Current lock state.
       pub state: EscrowLockState,
       /// Block number when escrow was created.
       pub created_block: u64,
   }

   pub struct EscrowStoreCell {
       id: CellId,
       escrows: parking_lot::RwLock<std::collections::HashMap<roko_core::ContentHash, EscrowState>>,
   }
   ```
   - Cell: `cell_name` = "escrow-store", `protocols` = `&["Store"]`
   - Store: `put()` locks budget from job assignment into Flow-scoped escrow, transitions to `EscrowLockState::Locked`
   - Store: `get()` returns escrow state by flow_id content hash
   - Store: `query()` filters by state, poster_id, assignee_id

   **SettlementCell** (Cell + Store):
   ```rust
   /// Settlement record for a completed job.
   #[derive(Debug, Clone)]
   pub struct SettlementRecord {
       pub flow_id: roko_core::ContentHash,
       /// Net payout to assignee (budget - marketplace_fee).
       pub payout: u128,
       /// Marketplace fee deducted.
       pub fee: u128,
       /// Whether the job passed verification.
       pub passed: bool,
       /// Block number of settlement.
       pub settled_block: u64,
   }

   pub struct SettlementCell {
       id: CellId,
       records: parking_lot::RwLock<Vec<SettlementRecord>>,
   }
   ```
   - Cell: `cell_name` = "settlement", `protocols` = `&["Store"]`
   - On verdict.passed: deduct marketplace_fee_bps from locked_amount, payout remainder to assignee, record positive reputation attestation
   - On verdict.failed: refund full locked_amount to poster, record negative reputation attestation

   **Slash configuration** (reuses existing `ReputationViolation` score penalties from `reputation_registry.rs`):
   ```rust
   /// Score penalty parameters per infraction type.
   /// These values mirror the penalties on ReputationViolation in reputation_registry.rs:
   /// - MissedDeadline:          -1% score penalty
   /// - AbandonedJob:            -3% score penalty
   /// - QualityRejection:        -2% score penalty
   /// - RepeatedQualityFailure:  -5% score penalty
   /// - Plagiarism:             -10% score penalty
   /// - ResultManipulation:     -10% score penalty
   /// - TeeViolation:           -10% score penalty (all domains)
   /// - Collusion:              feedback weight dilution (not a direct score penalty)
   pub struct SlashConfig {
       pub missed_deadline_penalty: f64,          // 0.01
       pub abandoned_job_penalty: f64,            // 0.03
       pub quality_rejection_penalty: f64,        // 0.02
       pub repeated_quality_failure_penalty: f64, // 0.05
       pub plagiarism_penalty: f64,               // 0.10
       pub result_manipulation_penalty: f64,      // 0.10
       pub tee_violation_penalty: f64,            // 0.10
   }
   ```

   **Dispute Pipeline Cells**:
   ```rust
   /// Collects evidence from both parties in a dispute.
   pub struct EvidenceCollectCell {
       id: CellId,
       evidence: parking_lot::RwLock<std::collections::HashMap<roko_core::ContentHash, Vec<EvidenceItem>>>,
   }

   #[derive(Debug, Clone)]
   pub struct EvidenceItem {
       pub submitter_id: u256,
       pub content_hash: roko_core::ContentHash,
       pub description: String,
       pub submitted_block: u64,
   }

   /// VRF-selects N jurors from eligible agents.
   pub struct JuryRouteCell {
       id: CellId,
       /// Number of jurors to select (default 3).
       pub jury_size: usize,
       /// Minimum passport tier for jurors (default: Sovereign -- maps to Silver+ in docs).
       pub min_tier: crate::phase2::PassportTier,
   }

   /// Collects votes, computes reputation-weighted median verdict.
   pub struct JuryVerifyCell {
       id: CellId,
   }
   ```
   - EvidenceCollectCell (Cell + Store): stores and retrieves evidence items per dispute flow_id
   - JuryRouteCell (Cell + Route): `select()` VRF-selects `jury_size` jurors from Sovereign+ agents, excluding poster and assignee
   - JuryVerifyCell (Cell + Verify): `verify()` collects votes from jurors, computes reputation-weighted median score, passes if weighted median > 0.5

5. Add module to lib.rs:
   ```rust
   pub mod job_settlement;
   pub use job_settlement::{
       EscrowLockState, EscrowState, EscrowStoreCell,
       SettlementRecord, SettlementCell, SlashConfig,
       EvidenceCollectCell, EvidenceItem, JuryRouteCell, JuryVerifyCell,
   };
   ```

6. Write tests:
   - EscrowStoreCell locks correct amount, state is `Locked` after put()
   - SettlementCell on pass: payout = locked_amount - (locked_amount * marketplace_fee_bps / 10000)
   - SettlementCell on fail: full refund to poster, payout = 0
   - JuryRouteCell excludes poster and assignee from jury pool
   - JuryVerifyCell reputation-weighted median computed correctly (high-reputation jurors have more weight)
   - Slash rates applied correctly for each `ReputationViolation` variant

## Verification
```bash
cargo check -p roko-chain
cargo clippy -p roko-chain --no-deps -- -D warnings
cargo test -p roko-chain -- job_settlement
```

## What NOT to do
- Do NOT implement on-chain escrow -- use in-memory store for tests
- Do NOT implement real VRF -- use deterministic seed for test reproducibility
- Do NOT modify existing marketplace.rs -- this is the Cell-based replacement
- Do NOT implement the full Graph TOML wiring -- just the individual Cell types
