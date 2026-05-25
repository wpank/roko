# M132 — Registry Store Cells (Identity, Reputation, Validation)

**[BLOCKED:chain]** -- Requires M131 (ChainConnector) and M006 (Store protocol alias). Chain deployment is Tier 6.

## Objective
Wrap the three existing in-memory registries (`AgentRegistry`, `ReputationRegistry`, `ValidationRegistry`) as Store Cells with optional on-chain backing via ChainConnector. Each registry becomes a multi-protocol Cell: IdentityStoreCell (Cell + Store), ReputationStoreCell (Cell + Store + Score), ValidationStoreCell (Cell + Store + Verify). This unifies chain state access with the standard Cell/Protocol vocabulary -- the Graph composer can wire registries into any pipeline.

## Scope
- Crates: `roko-chain`, `roko-core`
- Files:
  - `crates/roko-chain/src/store_cells.rs` (new)
  - `crates/roko-chain/src/lib.rs` (add module + re-exports)
- Depth doc: `tmp/unified-depth/18-registries/01-chain-as-domain-plugin.md` SS5

## Steps
1. Verify existing registries and their public APIs:
   ```bash
   grep -rn 'pub struct AgentRegistry' crates/roko-chain/src/agent_registry.rs
   grep -rn 'pub fn mint\|pub fn get_passport\|pub fn has_capability\|pub fn update_tier' crates/roko-chain/src/agent_registry.rs
   grep -rn 'pub struct ReputationRegistry' crates/roko-chain/src/reputation_registry.rs
   grep -rn 'pub fn register_agent\|pub fn submit_feedback\|pub fn get_score\|pub fn get_all_scores\|pub fn discipline_state\|pub fn slash' crates/roko-chain/src/reputation_registry.rs
   grep -rn 'pub struct ValidationRegistry' crates/roko-chain/src/validation_registry.rs
   grep -rn 'pub fn submit_proof\|pub fn verify_proof\|pub fn records_for_passport' crates/roko-chain/src/validation_registry.rs
   ```
   **Expected**: `AgentRegistry` at `agent_registry.rs:103` (methods: `new`, `mint`, `get_passport`, `transfer`, `submit_prompt_update`, `execute_prompt_update`, `update_tier`, `passport_count`, `has_capability`). `ReputationRegistry` at `reputation_registry.rs:493` (methods: `new`, `register_agent`, `submit_feedback`, `submit_feedback_weighted`, `get_score`, `get_all_scores`, `feedback_weight`, `slash`, `discipline_state`, `ban_agent`, `amnesty_eligible`, `governance_amnesty`, `record_recovery_job`, `post_recovery_stake`, `pass_verification_challenge`, `recovery_status`, `attempt_recovery`, `agent_count`). `ValidationRegistry` at `validation_registry.rs:66` (methods: `new`, `submit_proof`, `verify_proof`, `records_for_passport`, `accepted_count`).

2. Verify the Cell, Store, Score, and Verify protocol traits:
   ```bash
   grep -rn 'pub trait Cell' crates/roko-core/src/cell.rs
   grep -rn 'pub trait Store' crates/roko-core/src/traits.rs
   grep -rn 'pub trait Score' crates/roko-core/src/traits.rs
   grep -rn 'pub trait Verify' crates/roko-core/src/traits.rs
   ```
   **Expected**: `Cell` at `cell.rs:14`. `Store` at `traits.rs:37` (async methods: `put(Engram) -> ContentHash`, `get(&ContentHash) -> Option<Engram>`, `query(&Query, &Context) -> Vec<Engram>`, `query_similar(&HdcVector, usize, &Context) -> Vec<(Engram, f32)>`). `Score` at `traits.rs:167` (sync: `score(&Engram, &Context) -> ScoreValue`). `Verify` at `traits.rs:214` (async: `verify(&Engram, &Context) -> Verdict`).

3. Create `crates/roko-chain/src/store_cells.rs` with three Cell wrappers:

   **IdentityStoreCell** (implements Cell + Store):
   ```rust
   use roko_core::cell::{Cell, CellId, CellVersion};
   use roko_core::traits::Store;
   use crate::agent_registry::AgentRegistry;
   use crate::connector::ChainConnector;

   pub struct IdentityStoreCell {
       id: CellId,
       registry: parking_lot::RwLock<AgentRegistry>,
       chain: Option<Arc<ChainConnector>>,
   }
   ```
   - Cell: `cell_id`, `cell_name` = "identity-store", `protocols` = `&["Store"]`
   - Store: `put()` wraps passport data as Engram, stores locally + optionally writes on-chain
   - Store: `get()` by content hash, cache-first, chain fallback if chain connector present
   - Store: `query()` delegates to `AgentRegistry` methods (by_owner, by_capability, by_tier predicates), returns matching passports wrapped as Engrams

   **ReputationStoreCell** (implements Cell + Store + Score):
   ```rust
   pub struct ReputationStoreCell {
       id: CellId,
       registry: parking_lot::RwLock<ReputationRegistry>,
       chain: Option<Arc<ChainConnector>>,
   }
   ```
   - Cell: `cell_id`, `cell_name` = "reputation-store", `protocols` = `&["Store", "Score"]`
   - Store: read/write/query reputation records as Engrams
   - Score: given an Engram with passport_id + domain metadata, call `registry.get_score()` and `registry.discipline_state()` to compute a ScoreValue:
     - `reputation`: from `get_score(passport_id, domain, now)` (decay-adjusted EMA)
     - `discipline_penalty`: `GoodStanding` -> 0.0, `Probation` -> 0.2, `Suspended` -> 0.8, `Banned` -> 1.0
     - `feedback_weight`: from `registry.feedback_weight(passport_id, now)` (may be diluted by collusion)
     - ScoreValue = reputation * (1.0 - discipline_penalty) * feedback_weight

   **ValidationStoreCell** (implements Cell + Store + Verify):
   ```rust
   pub struct ValidationStoreCell {
       id: CellId,
       registry: parking_lot::RwLock<ValidationRegistry>,
       chain: Option<Arc<ChainConnector>>,
   }
   ```
   - Cell: `cell_id`, `cell_name` = "validation-store", `protocols` = `&["Store", "Verify"]`
   - Store: read/write/query validation records as Engrams
   - Verify: given an Engram with job_hash + passport_id metadata, call `registry.verify_proof()` and map `VerificationResult` to `Verdict`

4. Add module to lib.rs:
   ```rust
   pub mod store_cells;
   pub use store_cells::{IdentityStoreCell, ReputationStoreCell, ValidationStoreCell};
   ```

5. Write tests:
   - IdentityStoreCell read/write round-trip without chain connector (chain = None)
   - ReputationStoreCell.score() returns correct discipline penalty per state:
     - GoodStanding -> penalty 0.0
     - Probation -> penalty 0.2 (push a domain below 0.4 via repeated low submit_feedback)
     - Suspended -> penalty 0.8 (push below 0.2 or 3+ slashes in 90-day window)
     - Banned -> penalty 1.0 (via ban_agent)
   - ValidationStoreCell.verify() returns pass for accepted records, fail for missing
   - Multi-protocol: ReputationStoreCell speaks both Store and Score (protocols() returns both)

## Verification
```bash
cargo check -p roko-chain
cargo clippy -p roko-chain --no-deps -- -D warnings
cargo test -p roko-chain -- store_cells
```

## What NOT to do
- Do NOT replace the existing registries -- wrap them (use `parking_lot::RwLock` or `std::sync::RwLock`)
- Do NOT implement on-chain operations if ChainConnector does not exist yet -- use `Option<Arc<ChainConnector>>` and test the None path only
- Do NOT add new dependencies to roko-chain for this batch (parking_lot is already in scope)
- Do NOT implement ChainStore (the full on-chain Signal store) -- that is separate from the registry wrappers
