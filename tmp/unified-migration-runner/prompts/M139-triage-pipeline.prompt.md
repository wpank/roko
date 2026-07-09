# M139 — Chain Event Triage Pipeline (4-Stage Score/Observe/Compose)

**[BLOCKED:chain]** -- Requires M138 (ChainWitnessFeed) and M012 (Cell trait). The Pipeline structure requires M040 (Graph executor) for runtime wiring, but the Cell types can be defined independently.

## Objective
Implement the four-stage triage Pipeline that processes chain events after they pass the T0 filter. The pipeline is: RuleClassifierCell (Score) -> MidasAnomalyLens (Score) -> ContextEnricherCell (Compose) -> CuriosityScorerCell (Score). All stages are rule-based, streaming, and constant-memory -- no LLM involved. Events scoring above the curiosity threshold graduate from Pulse to Signal.

## Scope
- Crates: `roko-chain`
- Files:
  - `crates/roko-chain/src/triage_pipeline.rs` (new)
  - `crates/roko-chain/src/lib.rs` (add module + re-exports)
- Depth doc: `tmp/unified-depth/18-registries/05-chain-witness-and-triage.md` SS3-4

## Steps
1. Check existing triage code to understand what is already implemented:
   ```bash
   grep -rn 'pub struct TriagePipeline\|pub struct TriageConfig\|pub struct MidasRScorer\|pub enum TriageAction\|pub struct TriageResult\|pub struct EventEnrichment' crates/roko-chain/src/triage.rs
   ```
   **Expected**: `TriageConfig` at `triage.rs:17` (fields: `anomaly_threshold: f64`, `curiosity_threshold: f64`, `known_contracts: HashMap<String, String>`, `known_topics: HashMap<String, String>`). `TriageResult` at `triage.rs:41` (fields: `event: ObservedEvent`, `rule_matched: bool`, `rule_label: Option<String>`, `anomaly_score: f64`, `is_anomalous: bool`, `enrichment: EventEnrichment`, `curiosity_score: f64`, `action: TriageAction`). `TriageAction` at `triage.rs:73` (enum: `IngestKnowledge`, `AlertConductor`, `MarketplaceHandler`, `Drop`). `MidasRScorer` at `triage.rs:89` (methods: `new(alpha)`, `observe(address)`, `score(address) -> f64`, `advance_window()`). `EventEnrichment` at `triage.rs:62` (fields: `contract_label`, `event_type_label`, `domain_tags: Vec<String>`). `TriagePipeline` at `triage.rs:160` (methods: `new(config)`, `triage(event) -> TriageResult`, `triage_batch(events) -> Vec<TriageResult>`).

   The existing `triage.rs` already implements the 4-stage pipeline (rule filter -> MIDAS-R -> enrichment -> curiosity scoring). **This Cell-based version wraps each stage as an independent Cell so they can be composed in a Graph.**

2. Verify Cell, Score, Compose, and Observe protocol traits:
   ```bash
   grep -rn 'pub trait Cell' crates/roko-core/src/cell.rs
   grep -rn 'pub trait Score' crates/roko-core/src/traits.rs
   grep -rn 'pub trait Compose' crates/roko-core/src/traits.rs
   grep -rn 'pub trait Observe' crates/roko-core/src/traits.rs
   ```
   **Expected**: `Cell` at `cell.rs:14`. `Score` at `traits.rs:167` (sync: `score(&Engram, &Context) -> ScoreValue`). `Compose` at `traits.rs:285` (sync: `compose(&[Engram], &Budget, &dyn Score, &Context) -> Result<Engram>`). `Observe` at `traits.rs:400` (supertrait of Cell, sync: `observe() -> Vec<Engram>`).

3. Create `crates/roko-chain/src/triage_pipeline.rs`:

   **Stage 1: RuleClassifierCell** (Cell + Score):
   ```rust
   use std::collections::HashMap;
   use roko_core::cell::{Cell, CellId};
   use roko_core::score::Score as ScoreValue;
   use roko_core::traits::Score;
   use roko_core::{Context, Engram};

   /// Classification of a chain event based on method selector or log topic.
   #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
   pub enum ChainEventKind {
       DeFiSwap,
       DeFiLiquidity,
       TokenTransfer,
       NftMint,
       ContractDeployment,
       GovernanceVote,
       BridgeDeposit,
       OracleUpdate,
       Unknown,
   }

   /// Configuration mapping method selectors and topics to event kinds.
   #[derive(Debug, Clone, Default)]
   pub struct RuleClassifierConfig {
       /// 4-byte method selector -> event kind.
       pub selector_rules: HashMap<[u8; 4], ChainEventKind>,
       /// 32-byte log topic -> event kind.
       pub topic_rules: HashMap<String, ChainEventKind>,
   }

   pub struct RuleClassifierCell {
       id: CellId,
       config: RuleClassifierConfig,
   }
   ```
   - Cell: `cell_name` = "rule-classifier", `protocols` = `&["Score"]`
   - Score: O(1) HashMap lookup on method selector (first 4 bytes of tx data) or log topic (first topic). Known kind -> relevance ScoreValue 0.5, Unknown -> 0.1

   **Stage 2: MidasAnomalyLens** (Cell + Score):
   ```rust
   /// Count-Min Sketch for constant-memory frequency estimation.
   pub struct CountMinSketch {
       /// 2D array: depth rows x width columns.
       table: Vec<Vec<u64>>,
       width: usize,
       depth: usize,
   }

   impl CountMinSketch {
       pub fn new(width: usize, depth: usize) -> Self { ... }
       pub fn increment(&mut self, key: &[u8]) { ... }
       pub fn estimate(&self, key: &[u8]) -> u64 { ... }
   }

   pub struct MidasAnomalyLens {
       id: CellId,
       /// Current window counts.
       current: CountMinSketch,
       /// Expected counts (EMA of past windows).
       expected: CountMinSketch,
       /// Window count for EMA updates.
       window_count: u64,
       /// EMA smoothing factor.
       alpha: f64,
   }
   ```
   - Cell: `cell_name` = "midas-anomaly", `protocols` = `&["Score"]`
   - Score: models event stream as temporal graph (nodes = addresses, edges = events). Uses CMS (width=1024, depth=4, ~32KB fixed). Chi-squared anomaly: `(observed - expected)^2 / expected`. Normalize to [0, 1] via sigmoid.
   - CMS width and depth are configurable; default values keep memory under 32KB

   **Stage 3: ContextEnricherCell** (Cell + Compose):
   ```rust
   pub struct ContextEnricherCell {
       id: CellId,
       /// Known contract addresses -> labels.
       contract_labels: HashMap<String, String>,
       /// Known topic hashes -> event type labels.
       topic_labels: HashMap<String, String>,
   }
   ```
   - Cell: `cell_name` = "context-enricher", `protocols` = `&["Compose"]`
   - Compose: `compose(engrams, budget, scorer, ctx)` enriches event engrams with contract metadata (label from known_contracts), prior interaction count (from context), domain tags. Returns a new enriched engram.

   **Stage 4: CuriosityScorerCell** (Cell + Score):
   ```rust
   /// Curiosity action thresholds.
   #[derive(Debug, Clone, Copy, PartialEq, Eq)]
   pub enum CuriosityAction {
       /// Below 0.2: ignore the event entirely.
       Ignore,
       /// 0.2-0.5: log silently for analytics.
       Silent,
       /// 0.5-0.8: graduate Pulse to Signal, publish alert.
       Alert,
       /// Above 0.8: graduate + emit Theta interrupt.
       Escalate,
   }

   /// Tracks exponential decay of seen event types for novelty.
   #[derive(Debug, Clone, Default)]
   pub struct NoveltyTracker {
       seen_counts: HashMap<ChainEventKind, f64>,
       decay_rate: f64,
   }

   /// Tracks KL-divergence for surprise scoring.
   #[derive(Debug, Clone, Default)]
   pub struct SurpriseTracker {
       predicted: HashMap<ChainEventKind, f64>,
       observed: HashMap<ChainEventKind, f64>,
   }

   /// Configuration for curiosity scoring weights.
   #[derive(Debug, Clone)]
   pub struct CuriosityConfig {
       pub relevance_weight: f64,  // default 0.30
       pub anomaly_weight: f64,    // default 0.25
       pub novelty_weight: f64,    // default 0.25
       pub surprise_weight: f64,   // default 0.20
   }

   pub struct CuriosityScorerCell {
       id: CellId,
       config: CuriosityConfig,
       novelty: parking_lot::RwLock<NoveltyTracker>,
       surprise: parking_lot::RwLock<SurpriseTracker>,
   }
   ```
   - Cell: `cell_name` = "curiosity-scorer", `protocols` = `&["Score"]`
   - Score: 4-axis weighted composite: relevance(0.30) + anomaly(0.25) + novelty(0.25) + surprise(0.20)
   - Maps composite to CuriosityAction: <0.2 Ignore, 0.2-0.5 Silent, 0.5-0.8 Alert, >0.8 Escalate
   - Novelty: exponential decay of seen event types; rare events score higher
   - Surprise: KL-divergence between predicted and observed event-kind distribution

   **Graduation logic** (helper function, not a separate Cell):
   ```rust
   /// Determine graduation action for a scored event.
   pub fn graduation_action(curiosity_score: f64) -> CuriosityAction {
       if curiosity_score >= 0.8 { CuriosityAction::Escalate }
       else if curiosity_score >= 0.5 { CuriosityAction::Alert }
       else if curiosity_score >= 0.2 { CuriosityAction::Silent }
       else { CuriosityAction::Ignore }
   }
   ```
   - Alert events: would graduate Pulse to Signal in Store, publish on `chain:alert:{chain_id}` (actual Bus publishing deferred to runtime)
   - Escalate events: graduate + emit Theta interrupt on `daimon.theta.interrupt` (deferred)

4. Add module to lib.rs:
   ```rust
   pub mod triage_pipeline;
   pub use triage_pipeline::{
       ChainEventKind, RuleClassifierCell, RuleClassifierConfig,
       CountMinSketch, MidasAnomalyLens,
       ContextEnricherCell,
       CuriosityAction, CuriosityConfig, CuriosityScorerCell,
       NoveltyTracker, SurpriseTracker,
       graduation_action,
   };
   ```

5. Write tests:
   - RuleClassifierCell classifies known selectors correctly (insert 0xa9059cbb -> TokenTransfer, then score)
   - MidasAnomalyLens detects burst activity: 2 events in window 1, 50 events in window 2 -> high anomaly
   - MidasAnomalyLens constant memory: CountMinSketch size does not grow with event count
   - CountMinSketch round-trip: increment N times, estimate returns approximately N
   - CuriosityScorerCell correctly applies weighted composite (0.30*rel + 0.25*anom + 0.25*nov + 0.20*surp)
   - graduation_action thresholds: 0.1 -> Ignore, 0.3 -> Silent, 0.6 -> Alert, 0.9 -> Escalate

## Verification
```bash
cargo check -p roko-chain
cargo clippy -p roko-chain --no-deps -- -D warnings
cargo test -p roko-chain -- triage_pipeline
```

## What NOT to do
- Do NOT modify existing triage.rs -- this is the Cell-based replacement using the same algorithm
- Do NOT add LLM calls -- the entire pipeline is rule-based and streaming
- Do NOT add external dependencies for Count-Min Sketch -- implement the simple 2D array version
- Do NOT implement the full Daimon theta interrupt integration -- just determine the CuriosityAction
