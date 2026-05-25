# M135 — Job Marketplace Graph Types (Posting, Matching, Hiring)

**[BLOCKED:chain]** -- Requires M012 (Cell trait), M040 (Graph executor), M131 (ChainConnector). Chain deployment is Tier 6.

## Objective
Define the job marketplace as a Graph of standard Cells. A job IS a Flow -- an instance of the marketplace Graph with a RunId. The seven v1 lifecycle states (POSTED through SETTLED) become positions in a Pipeline of Cells. This batch covers the first half: JobPostCell (Store), CapabilityMatchCell (Score), and three HiringRoute Cells (RandomVRF, BlindAuction, DirectHire).

## Scope
- Crates: `roko-chain`
- Files:
  - `crates/roko-chain/src/job_graph.rs` (new)
  - `crates/roko-chain/src/lib.rs` (add module + re-exports)
- Depth doc: `tmp/unified-depth/18-registries/03-job-market-and-hiring.md`

## Steps
1. Check existing marketplace implementation and types:
   ```bash
   grep -rn 'pub struct Marketplace\|pub struct MarketplaceJob\|pub enum JobState\|pub struct SporeJobPosting' crates/roko-chain/src/marketplace.rs crates/roko-chain/src/phase2.rs
   grep -rn 'pub enum HiringModel\|pub enum AuctionType' crates/roko-chain/src/phase2.rs
   grep -rn 'pub enum PassportTier' crates/roko-chain/src/phase2.rs
   ```
   **Expected**: `Marketplace` at `marketplace.rs:129`. `MarketplaceJob` at `marketplace.rs:49` with fields `job_id`, `posting`, `state`, `assignees`, `result_hash`, `created_block`. `JobState` at `marketplace.rs:29` (enum: `Posted`, `Bidding`, `Assigned`, `InProgress`, `Submitted`, `Settled`, `Expired`, `Disputed`). `SporeJobPosting` at `phase2.rs:1489` with fields `job_id`, `domain`, `required_capabilities: u64`, `budget: u256`, `deadline_block`, `hiring_model: HiringModel`, `min_reputation`, `min_tier: PassportTier`, `description_cid`, `poster_passport_id: u256`, `direct_hire_target`, `max_agents`. `HiringModel` at `phase2.rs:1518` (enum: `RandomVRF`, `BlindAuction { auction_duration_blocks, auction_type: AuctionType }`, `DirectHire { target_passport_id: u256 }`). `PassportTier` at `phase2.rs:476` (enum: `Protocol`, `Sovereign`, `Worker`, `Edge` -- ordered by privilege level, NOT Gray/Copper/Silver/Gold/Amber).

2. Verify Cell, Store, Score, and Route protocol traits:
   ```bash
   grep -rn 'pub trait Cell' crates/roko-core/src/cell.rs
   grep -rn 'pub trait Store' crates/roko-core/src/traits.rs
   grep -rn 'pub trait Score' crates/roko-core/src/traits.rs
   grep -rn 'pub trait Route' crates/roko-core/src/traits.rs
   ```
   **Expected**: `Cell` at `cell.rs:14`. `Store` at `traits.rs:37` (async: `put(Engram) -> ContentHash`, `get`, `query`, `query_similar`). `Score` at `traits.rs:167` (sync: `score(&Engram, &Context) -> ScoreValue`). `Route` at `traits.rs:242` (sync: `select(&[Engram], &Context) -> Option<Selection>`, `feedback(&Outcome)`, `name() -> &str`).

3. Create `crates/roko-chain/src/job_graph.rs`:

   **Core types** (wrapping existing phase2 types with Cell-friendly interfaces):
   ```rust
   use crate::phase2::{u256, PassportTier, SporeJobPosting, HiringModel, AuctionType};
   use roko_core::cell::{Cell, CellId};
   use roko_core::traits::{Store, Score, Route};
   use roko_core::{Context, Engram, ContentHash};

   /// Extended job posting for the Cell-based marketplace.
   /// Wraps SporeJobPosting from phase2 with additional Cell-layer metadata.
   pub struct JobPosting {
       /// The underlying phase2 posting.
       pub inner: SporeJobPosting,
       /// Platform fee (separate from budget).
       pub platform_fee: u64,
       /// Marketplace fee in basis points (e.g. 250 = 2.5%).
       pub marketplace_fee_bps: u16,
       /// Job type classification.
       pub job_type: JobType,
   }

   /// Job type for assignment routing.
   #[derive(Debug, Clone, PartialEq, Eq)]
   pub enum JobType {
       /// Single agent assignment.
       Solo,
       /// Pair programming / review pair.
       Pair,
       /// Multi-agent with subtask DAG.
       Consortium { subtask_edges: Vec<SubtaskEdge> },
       /// Collective work with coordinator.
       Collective { coordinator_tier: PassportTier },
   }

   /// Edge in a subtask dependency DAG (for Consortium jobs).
   #[derive(Debug, Clone, PartialEq, Eq)]
   pub struct SubtaskEdge {
       pub from_task: u32,
       pub to_task: u32,
   }

   /// Assignment result with hiring model metadata.
   #[derive(Debug, Clone)]
   pub struct JobAssignment {
       /// Assigned agent passport IDs.
       pub assignees: Vec<u256>,
       /// Hiring model used.
       pub model: HiringModel,
       /// Premium multiplier (1.0 = no premium, 1.5 = DirectHire premium).
       pub premium_multiplier: f64,
       /// Vickrey second-price payment if auction (winning bid pays second price).
       pub vickrey_payment: Option<u64>,
   }

   /// A scored candidate for matching.
   #[derive(Debug, Clone)]
   pub struct ScoredCandidate {
       /// Passport ID of the candidate.
       pub passport_id: u256,
       /// Composite match score (0.0 - 1.0).
       pub score: f64,
       /// Current workload factor (0.0 = idle, 1.0 = fully loaded).
       pub load_factor: f64,
       /// Domain reputation score.
       pub reputation: f64,
   }
   ```

   **JobPostCell** (Cell + Store):
   ```rust
   pub struct JobPostCell {
       id: CellId,
       /// In-memory job store (keyed by content hash).
       jobs: parking_lot::RwLock<std::collections::HashMap<ContentHash, JobPosting>>,
   }
   ```
   - Cell: `cell_name` = "job-post", `protocols` = `&["Store"]`
   - Store: `put()` validates poster budget (budget + platform_fee must be > 0), wraps JobPosting as Engram, stores locally, returns content hash
   - Store: `get()` retrieves by content hash
   - Store: `query()` filters by domain, min_tier, budget range

   **CapabilityMatchCell** (Cell + Score):
   ```rust
   pub struct CapabilityMatchCell {
       id: CellId,
       /// Matching strategy to use.
       strategy: MatchStrategy,
   }

   #[derive(Debug, Clone, Copy)]
   pub enum MatchStrategy {
       /// Bitwise AND between required and agent capabilities.
       BitwiseAnd,
       /// HDC cosine similarity between capability vectors.
       HdcSimilarity,
       /// Two-phase: BitwiseAnd filter then HdcSimilarity ranking.
       TwoPhase,
   }
   ```
   - Cell: `cell_name` = "capability-match", `protocols` = `&["Score"]`
   - Score: `score(engram, ctx)` extracts `required_capabilities: u64` and `agent_capabilities: u64` from engram metadata, applies strategy:
     - BitwiseAnd: `(required & agent) == required` -> 1.0, else 0.0
     - HdcSimilarity: cosine similarity between capability bitmasks (treat as vectors)
     - TwoPhase: BitwiseAnd filter, then HdcSimilarity rank

   **HiringRouteCell** (Cell + Route) -- meta-router delegating to three strategies:
   ```rust
   pub struct HiringRouteCell {
       id: CellId,
   }
   ```
   - Cell: `cell_name` = "hiring-route", `protocols` = `&["Route"]`
   - Route: `select(candidates, ctx)` reads hiring model from context, delegates to:
     - `RandomVRF`: power-of-two-choices from deterministic seed (pick 2 random candidates, select lower load_factor), O(1) assignment
     - `BlindAuction`: Vickrey reputation-adjusted scoring (bid / reputation) for truthful bidding. Winner pays second-highest adjusted price.
     - `DirectHire`: named agent by `target_passport_id`, 1.5x premium, must have `PassportTier::Worker` or lower (Edge/Worker only)
   - Selection logic: `DirectHire` if target specified and eligible, `RandomVRF` if budget < 5000 KORAI, `BlindAuction` otherwise

4. Add module to lib.rs:
   ```rust
   pub mod job_graph;
   pub use job_graph::{
       JobPosting, JobType, SubtaskEdge, JobAssignment, ScoredCandidate,
       JobPostCell, CapabilityMatchCell, MatchStrategy, HiringRouteCell,
   };
   ```

5. Write tests:
   - JobPostCell rejects posting with zero budget (Store.put returns error)
   - CapabilityMatchCell BitwiseAnd: required=0b1010, agent=0b1011 -> 1.0; agent=0b1000 -> 0.0
   - RandomVRF route picks lower load_factor of two probes (deterministic seed)
   - BlindAuction Vickrey scoring ranks reputation-adjusted bids correctly (winner pays second price)
   - DirectHire route rejects agents above Worker tier (Sovereign and Protocol are ineligible for DirectHire)

## Verification
```bash
cargo check -p roko-chain
cargo clippy -p roko-chain --no-deps -- -D warnings
cargo test -p roko-chain -- job_graph
```

## What NOT to do
- Do NOT modify the existing `marketplace.rs` -- this is a parallel Cell-based implementation
- Do NOT modify `phase2.rs` types -- reuse `SporeJobPosting`, `HiringModel`, `PassportTier`, `AuctionType` as-is
- Do NOT implement escrow or settlement -- that is M136
- Do NOT implement the full Graph TOML definition -- just the Cell types
- Do NOT add on-chain escrow operations -- use in-memory mock ledger for tests
