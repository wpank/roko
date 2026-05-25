# M137 — Reputation Score Cell with EMA + TraceRank Pipeline

**[BLOCKED:chain]** -- Requires M132 (ReputationStoreCell), M012 (Cell trait), M007 (Score protocol). Chain deployment is Tier 6.

## Objective
Formalize the reputation system as a Score Cell with EMA internals and a three-stage peer-scoring Pipeline. The existing `ReputationRegistry` (7-domain EMA) becomes the internal state of a `ReputationScoreCell`. The `TraceRank` model (PageRank-style propagation over payment edges) becomes a Pipeline of three Score Cells: direct feedback, graph propagation, and temporal decay. A `ReputationReactCell` subscribes to settlement and gate events to submit reputation feedback.

## Scope
- Crates: `roko-chain`
- Files:
  - `crates/roko-chain/src/reputation_cell.rs` (new)
  - `crates/roko-chain/src/lib.rs` (add module + re-exports)
- Depth doc: `tmp/unified-depth/18-registries/04-reputation-and-peer-scoring.md`

## Steps
1. Verify existing reputation infrastructure and exact APIs:
   ```bash
   grep -rn 'pub struct ReputationRegistry' crates/roko-chain/src/reputation_registry.rs
   grep -rn 'pub fn register_agent\|pub fn submit_feedback\|pub fn submit_feedback_weighted\|pub fn get_score\|pub fn get_all_scores\|pub fn feedback_weight\|pub fn slash\|pub fn discipline_state\|pub fn ban_agent' crates/roko-chain/src/reputation_registry.rs
   grep -rn 'pub enum DisciplineState' crates/roko-chain/src/reputation_registry.rs
   grep -rn 'REPUTATION_DOMAINS' crates/roko-chain/src/reputation_registry.rs
   ```
   **Expected**: `ReputationRegistry` at `reputation_registry.rs:493` (stores per-agent `AgentReputation` keyed by `u256` passport_id). Key methods:
   - `register_agent(passport_id: u256, now: u64)` -- initializes 7-domain scores at 0.5
   - `submit_feedback(passport_id: u256, domain: &str, quality: f64, now: u64)` -- EMA update, quality in [-1.0, 1.0]
   - `submit_feedback_weighted(passport_id: u256, domain: &str, quality: f64, weight: f64, now: u64)` -- feedback with explicit weight
   - `get_score(passport_id: u256, domain: &str, now: u64) -> f64` -- decay-adjusted EMA score
   - `get_all_scores(passport_id: u256, now: u64) -> HashMap<String, f64>` -- all 7 domains
   - `feedback_weight(passport_id: u256, now: u64) -> f64` -- influence weight (may be diluted by collusion)
   - `slash(passport_id: u256, violation: ReputationViolation, domain: &str, now: u64)` -- apply penalty
   - `discipline_state(passport_id: u256, now: u64) -> DisciplineState` -- current discipline
   - `ban_agent(passport_id: u256, now: u64)` -- permanent ban

   `DisciplineState` at `reputation_registry.rs:63` (enum: `GoodStanding`, `Probation`, `Suspended`, `Banned` -- unit variants, no fields).

   `REPUTATION_DOMAINS` at `reputation_registry.rs:51` = `["coding", "security", "research", "chain", "knowledge", "operations", "strategy"]` (7 domains, NOT "coding, security, research, deployment, coordination, review, teaching").

2. Verify existing TraceRank infrastructure:
   ```bash
   grep -rn 'pub struct TraceRank\|pub fn compute\|pub fn record_payment\|pub fn blend_reputation\|pub fn normalized_rank' crates/roko-chain/src/trace_rank.rs
   grep -rn 'pub struct TraceRankConfig\|pub struct TraceRankResult\|pub struct PaymentEdge' crates/roko-chain/src/trace_rank.rs
   ```
   **Expected**: `TraceRank` at `trace_rank.rs:113` with methods:
   - `new() -> Self`, `with_config(TraceRankConfig) -> Self`
   - `record_payment(edge: PaymentEdge)` -- add edge to payment graph
   - `edge_count() -> usize`
   - `compute() -> TraceRankResult` -- run PageRank iteration over payment graph
   - `blend_reputation(ema_score: f64, trace_rank_score: f64) -> f64` -- weighted blend
   - `normalized_rank(result: &TraceRankResult, agent: u256) -> f64` -- normalize agent's rank to [0, 1]

   `TraceRankConfig` at `trace_rank.rs:65` with fields: `damping: f64`, `max_iterations: usize`, `convergence_threshold: f64`, `blend_weight: f64`.
   `TraceRankResult` at `trace_rank.rs:96` with fields: `ranks: HashMap<u256, f64>`, `iterations: usize`, `converged: bool`.
   `PaymentEdge` at `trace_rank.rs:43` with fields: `from: u256`, `to: u256`, `amount: f64`, `quality: f64`, `block: u64`.

3. Verify Cell, Score, and React protocol traits:
   ```bash
   grep -rn 'pub trait Cell' crates/roko-core/src/cell.rs
   grep -rn 'pub trait Score' crates/roko-core/src/traits.rs
   grep -rn 'pub trait React' crates/roko-core/src/traits.rs
   ```
   **Expected**: `Cell` at `cell.rs:14`. `Score` at `traits.rs:167` (sync: `score(&Engram, &Context) -> ScoreValue`, `name() -> &'static str`). `React` at `traits.rs:339` (sync: `decide(&[Engram], &Context) -> Vec<Engram>`, `name() -> &str`).

4. Verify PassportTier (used for tier progression):
   ```bash
   grep -rn 'pub enum PassportTier' crates/roko-chain/src/phase2.rs
   ```
   **Expected**: `PassportTier` at `phase2.rs:476` = `Protocol`, `Sovereign`, `Worker`, `Edge` (4 tiers, NOT 5; NOT Gray/Copper/Silver/Gold/Amber).

5. Create `crates/roko-chain/src/reputation_cell.rs`:

   **ReputationScoreCell** (Cell + Score):
   ```rust
   use crate::reputation_registry::{ReputationRegistry, DisciplineState, REPUTATION_DOMAINS};
   use crate::trace_rank::TraceRank;
   use crate::phase2::u256;
   use roko_core::cell::{Cell, CellId};
   use roko_core::score::Score as ScoreValue;
   use roko_core::traits::Score;
   use roko_core::{Context, Engram};

   /// Configuration for the ReputationScoreCell.
   #[derive(Debug, Clone)]
   pub struct ReputationScoreCellConfig {
       /// Weight of EMA score in final blend (default 0.7).
       pub ema_weight: f64,
       /// Weight of TraceRank score in final blend (default 0.3).
       pub trace_rank_weight: f64,
       /// Half-life in seconds for temporal decay (default 30 * 86400 = 30 days).
       pub decay_half_life_secs: u64,
   }

   impl Default for ReputationScoreCellConfig {
       fn default() -> Self {
           Self {
               ema_weight: 0.7,
               trace_rank_weight: 0.3,
               decay_half_life_secs: 30 * 86400,
           }
       }
   }

   /// Discipline penalty mapping.
   /// GoodStanding -> 0.0, Probation -> 0.2, Suspended -> 0.8, Banned -> 1.0
   pub fn discipline_penalty(state: &DisciplineState) -> f64 {
       match state {
           DisciplineState::GoodStanding => 0.0,
           DisciplineState::Probation => 0.2,
           DisciplineState::Suspended => 0.8,
           DisciplineState::Banned => 1.0,
       }
   }

   /// A Score Cell wrapping ReputationRegistry + TraceRank.
   pub struct ReputationScoreCell {
       id: CellId,
       config: ReputationScoreCellConfig,
       registry: parking_lot::RwLock<ReputationRegistry>,
       trace_rank: parking_lot::RwLock<TraceRank>,
   }

   impl ReputationScoreCell {
       pub fn new(id: CellId, config: ReputationScoreCellConfig) -> Self {
           Self {
               id,
               config,
               registry: parking_lot::RwLock::new(ReputationRegistry::new()),
               trace_rank: parking_lot::RwLock::new(TraceRank::new()),
           }
       }

       /// Compute the reputation score for a passport in a domain.
       ///
       /// Formula: (ema * ema_weight + trace_rank * trace_rank_weight) * (1.0 - discipline_penalty) * feedback_weight
       pub fn compute_score(&self, passport_id: u256, domain: &str, now: u64) -> f64 {
           let reg = self.registry.read();
           let ema = reg.get_score(passport_id, domain, now);
           let fw = reg.feedback_weight(passport_id, now);
           let ds = reg.discipline_state(passport_id, now);
           let penalty = discipline_penalty(&ds);
           drop(reg);

           let tr = self.trace_rank.read();
           let tr_result = tr.compute();
           let tr_score = tr.normalized_rank(&tr_result, passport_id);
           let blended = ema * self.config.ema_weight + tr_score * self.config.trace_rank_weight;
           blended * (1.0 - penalty) * fw
       }
   }
   ```
   - Cell: `cell_name` = "reputation-score", `protocols` = `&["Score"]`
   - Score: `score(engram, ctx)` extracts `passport_id` and `domain` from engram metadata, calls `compute_score()`, returns ScoreValue

   **Three-stage peer-scoring Pipeline** (all Cell + Score):
   ```rust
   /// Stage 1: Direct feedback. Raw EMA update from submit_feedback.
   pub struct DirectFeedbackCell {
       id: CellId,
       registry: Arc<parking_lot::RwLock<ReputationRegistry>>,
   }

   /// Stage 2: Graph propagation. TraceRank (PageRank over payment edges).
   pub struct GraphPropagationCell {
       id: CellId,
       trace_rank: Arc<parking_lot::RwLock<TraceRank>>,
   }

   /// Stage 3: Temporal decay. Half-life decay toward neutral (0.5).
   pub struct TemporalDecayCell {
       id: CellId,
       half_life_secs: u64,
   }
   ```
   - DirectFeedbackCell: `score()` calls `registry.get_score(passport_id, domain, now)`
   - GraphPropagationCell: `score()` calls `trace_rank.compute()` + `trace_rank.normalized_rank()`
   - TemporalDecayCell: `score()` applies `score + (0.5 - score) * (1.0 - 2^(-elapsed / half_life))`

   **ReputationReactCell** (Cell + React):
   ```rust
   /// Reacts to settlement and gate events by submitting reputation feedback.
   pub struct ReputationReactCell {
       id: CellId,
       registry: Arc<parking_lot::RwLock<ReputationRegistry>>,
   }
   ```
   - Cell: `cell_name` = "reputation-react", `protocols` = `&["React"]`
   - React: `decide(stream, ctx)` scans engram stream for:
     - `job.settled` events -> submit positive feedback (quality based on gate pass rate)
     - `gate.completed` events -> submit feedback (positive if passed, negative if failed)
     - `dispute.resolved` events -> submit negative feedback + apply slash per `ReputationViolation`
   - Returns new Engrams representing the reputation update events

6. Add module to lib.rs:
   ```rust
   pub mod reputation_cell;
   pub use reputation_cell::{
       ReputationScoreCell, ReputationScoreCellConfig, discipline_penalty,
       DirectFeedbackCell, GraphPropagationCell, TemporalDecayCell,
       ReputationReactCell,
   };
   ```

7. Write tests:
   - EMA update with positive feedback (quality=0.8) increases score above 0.5
   - EMA update with negative feedback (quality=-0.5) decreases score below 0.5
   - Temporal decay toward neutral (0.5) over time: score of 1.0 decays toward 0.5
   - TraceRank propagation: agent B pays agent A (via record_payment), A's trace_rank score increases
   - discipline_penalty: GoodStanding -> 0.0, Probation -> 0.2, Suspended -> 0.8, Banned -> 1.0
   - compute_score blends EMA and TraceRank correctly with configured weights
   - ReputationReactCell processes settlement engram and produces feedback engrams
   - Verify 7 domains match REPUTATION_DOMAINS constant

## Verification
```bash
cargo check -p roko-chain
cargo clippy -p roko-chain --no-deps -- -D warnings
cargo test -p roko-chain -- reputation_cell
```

## What NOT to do
- Do NOT replace the existing ReputationRegistry -- wrap it as internal state behind parking_lot::RwLock
- Do NOT replace the existing TraceRank -- use it from GraphPropagationCell
- Do NOT invent new tier names (Gray/Copper/Silver/Gold/Amber) -- use PassportTier (Protocol/Sovereign/Worker/Edge)
- Do NOT invent new domain names -- use REPUTATION_DOMAINS (coding/security/research/chain/knowledge/operations/strategy)
- Do NOT add LLM calls -- all reputation computation is rule-based
- Do NOT implement on-chain attestation submission -- test with in-memory state
