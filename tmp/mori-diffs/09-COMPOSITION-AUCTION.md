# 09 -- Composition Auction: VCG Activation + Per-Section Cost Attribution

## Problem Statement

### What's broken

The prompt composition system has two independent deficiencies that compound:

**1. VCG auction is built but never called at runtime.**

`vcg_allocate()` in `crates/roko-compose/src/auction.rs` (lines 293-414) is a complete
greedy-VCG mechanism with affect modulation, value-density sorting, and externality-based
payments. It is exported from `roko-compose::lib.rs` (line 49). It is tested (5 unit tests).
It is *never invoked from any runtime path*.

Instead, `PromptComposer::compose()` in `crates/roko-compose/src/prompt.rs` uses a
weighted-sum scoring path: for each section, it computes
`value_density = score.effective() / token_cost` (line 1118-1119), sorts by density, and
greedily fills the budget. This is structurally identical to VCG's greedy allocation but
without the payment computation, externality tracking, or diagnostic reporting.

The result: we pay the complexity cost of maintaining VCG infrastructure but get none of its
benefits (truthful bidding incentives, welfare diagnostics, displacement reporting).

**2. No per-section cost attribution after agent turns.**

When an agent completes a turn, we record total token usage in the episode (`usage.input_tokens`,
`usage.output_tokens`, `usage.cost_usd`). But we never attribute those costs back to the
specific prompt sections that consumed them. The `SectionEffectivenessRegistry` in
`crates/roko-compose/src/context_provider.rs` (imported from `roko-learn::section_effect`)
adjusts section priorities based on gate pass/fail, but has no cost signal. This means:

- A section that costs 2,000 tokens and contributes to 60% gate-pass rate is treated
  identically to one that costs 200 tokens with the same pass rate.
- The `LearningBidder` in `auction.rs` (lines 32-83) updates Beta posteriors on
  `(was_included, gate_passed)` but ignores cost entirely.
- There is no feedback loop from actual token spend to VCG value estimates.

### Why it matters

Without cost attribution, the learning system cannot distinguish high-value-per-token
sections from high-value-but-expensive sections. The VCG mechanism is designed exactly to
solve this: payments reveal marginal value, and cost attribution closes the feedback loop.
But the mechanism sits unused.

---

## Ideal Design

### Core Abstraction: `CompositionStrategy`

```rust
// crates/roko-compose/src/strategy.rs

/// Strategy for allocating token budget across prompt sections.
///
/// The system auto-selects based on observation count:
/// - WeightedSum: < 10 observations per bidder (cold start)
/// - VCG: >= 10 observations per bidder (learned values)
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum CompositionStrategy {
    /// Fast greedy allocation by value density. No payments computed.
    /// Used during cold start when Beta posteriors have < 10 observations.
    WeightedSum,
    /// Full VCG mechanism with payments, externality tracking, and
    /// displacement diagnostics. Used once posteriors are informative.
    Vcg,
}

impl CompositionStrategy {
    /// Select strategy based on minimum observation count across active bidders.
    pub fn auto_select(bidder_observations: &HashMap<AttentionBidder, u32>) -> Self {
        let min_obs = bidder_observations.values().copied().min().unwrap_or(0);
        if min_obs >= 10 {
            Self::Vcg
        } else {
            Self::WeightedSum
        }
    }
}
```

### Type: `CostAttribution`

```rust
// crates/roko-compose/src/cost_attribution.rs

/// Per-section cost attribution computed after an agent turn completes.
///
/// Proportionally distributes the actual token cost across the sections
/// that were included in the prompt, weighted by their token share.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CostAttribution {
    /// Turn identifier (episode_id + turn index).
    pub turn_id: String,
    /// Total input tokens reported by the LLM provider.
    pub total_input_tokens: u64,
    /// Total cost in USD for this turn.
    pub total_cost_usd: f64,
    /// Per-section breakdown.
    pub sections: Vec<SectionCost>,
    /// Strategy that was used to compose the prompt.
    pub strategy: CompositionStrategy,
    /// VCG payments, if VCG strategy was used. Empty for WeightedSum.
    pub vcg_payments: Vec<(String, f64)>,
}

/// One section's attributed cost from a completed turn.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SectionCost {
    /// Section name (e.g. "role", "task", "learned-context").
    pub section_name: String,
    /// Which bidder owned this section.
    pub bidder: AttentionBidder,
    /// Estimated tokens this section consumed (from prompt assembly).
    pub estimated_tokens: usize,
    /// Fraction of total input tokens attributed to this section.
    pub token_fraction: f64,
    /// Attributed cost in USD (total_cost_usd * token_fraction).
    pub attributed_cost_usd: f64,
    /// Whether the gate passed for the task this section contributed to.
    pub gate_passed: Option<bool>,
}

impl CostAttribution {
    /// Compute cost attribution from a completed turn's usage data.
    ///
    /// `included_sections` are the sections that survived budget pressure
    /// and were included in the final prompt. Each carries its estimated
    /// token count from `PromptSection::estimated_tokens()`.
    pub fn from_turn(
        turn_id: impl Into<String>,
        total_input_tokens: u64,
        total_cost_usd: f64,
        included_sections: &[(String, AttentionBidder, usize)],  // (name, bidder, est_tokens)
        strategy: CompositionStrategy,
        vcg_payments: Vec<(String, f64)>,
    ) -> Self {
        let total_estimated: usize = included_sections.iter().map(|(_, _, t)| t).sum();
        let total_est_f64 = total_estimated.max(1) as f64;

        let sections = included_sections
            .iter()
            .map(|(name, bidder, est_tokens)| {
                let token_fraction = *est_tokens as f64 / total_est_f64;
                SectionCost {
                    section_name: name.clone(),
                    bidder: *bidder,
                    estimated_tokens: *est_tokens,
                    token_fraction,
                    attributed_cost_usd: total_cost_usd * token_fraction,
                    gate_passed: None,  // Set later when gate results arrive
                }
            })
            .collect();

        Self {
            turn_id: turn_id.into(),
            total_input_tokens,
            total_cost_usd,
            sections,
            strategy,
            vcg_payments,
        }
    }

    /// Stamp gate results onto the attribution after gate evaluation.
    pub fn stamp_gate_result(&mut self, gate_passed: bool) {
        for section in &mut self.sections {
            section.gate_passed = Some(gate_passed);
        }
    }

    /// Compute cost-effectiveness ratio per section: gate_pass_rate / cost_per_token.
    /// Higher is better. Returns None for sections with no gate data yet.
    pub fn cost_effectiveness(&self) -> Vec<(String, Option<f64>)> {
        self.sections
            .iter()
            .map(|s| {
                let effectiveness = s.gate_passed.map(|passed| {
                    let value = if passed { 1.0 } else { 0.0 };
                    let cost_per_token = if s.estimated_tokens > 0 {
                        s.attributed_cost_usd / s.estimated_tokens as f64
                    } else {
                        f64::EPSILON
                    };
                    value / cost_per_token.max(f64::EPSILON)
                });
                (s.section_name.clone(), effectiveness)
            })
            .collect()
    }
}
```

### Enhanced `LearningBidder` with Cost Awareness

```rust
// Modifications to crates/roko-compose/src/auction.rs

impl LearningBidder {
    /// Compute the current bid for a section, incorporating cost-effectiveness.
    ///
    /// When cost data is available, the bid is scaled by the section's historical
    /// cost-effectiveness ratio (value per dollar per token). This makes the VCG
    /// mechanism cost-aware: sections that are cheap AND effective get higher bids.
    pub fn bid_with_cost(&self, section_name: &str, relevance: f64) -> f64 {
        let base_bid = self.bid(section_name, relevance);
        let cost_factor = self.cost_effectiveness_factor(section_name);
        base_bid * cost_factor
    }

    /// Update the posterior after observing one task outcome, including cost data.
    pub fn update_with_cost(
        &mut self,
        section_name: &str,
        was_included: bool,
        gate_passed: bool,
        attributed_cost_usd: f64,
        estimated_tokens: usize,
    ) {
        // Update the existing Beta posterior (pass/fail)
        self.update(section_name, was_included, gate_passed);

        if !was_included || estimated_tokens == 0 {
            return;
        }

        // Update cost tracking
        let entry = self
            .section_costs
            .entry(section_name.to_string())
            .or_insert(SectionCostStats::default());
        entry.total_cost_usd += attributed_cost_usd;
        entry.total_tokens += estimated_tokens;
        entry.observation_count += 1;
        if gate_passed {
            entry.passes += 1;
        }
    }

    /// Cost-effectiveness multiplier for a section.
    /// Returns 1.0 when no cost data is available (neutral).
    fn cost_effectiveness_factor(&self, section_name: &str) -> f64 {
        let Some(stats) = self.section_costs.get(section_name) else {
            return 1.0;
        };
        if stats.observation_count < 3 || stats.total_tokens == 0 {
            return 1.0;  // Not enough data
        }

        let pass_rate = stats.passes as f64 / stats.observation_count as f64;
        let cost_per_token = stats.total_cost_usd / stats.total_tokens as f64;

        // Normalize: higher pass rate and lower cost per token = better
        // Use log scale for cost to avoid extreme values
        let cost_efficiency = 1.0 / (1.0 + cost_per_token.ln().max(0.0));
        let combined = 0.7 * pass_rate + 0.3 * cost_efficiency;

        // Clamp to [0.5, 2.0] to avoid wild swings
        combined.clamp(0.5, 2.0)
    }
}

/// Accumulated cost statistics for one section within a LearningBidder.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct SectionCostStats {
    pub total_cost_usd: f64,
    pub total_tokens: usize,
    pub observation_count: u32,
    pub passes: u32,
}
```

### Integration into `PromptComposer`

The key change is that `PromptComposer::compose()` returns additional metadata that the
caller needs for cost attribution, and auto-selects between WeightedSum and VCG.

```rust
// Modifications to crates/roko-compose/src/prompt.rs

/// Metadata from a composition pass, attached to the output Engram's tags.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CompositionManifest {
    /// Strategy used for this composition.
    pub strategy: CompositionStrategy,
    /// Sections included in the final prompt, with their token estimates.
    pub included: Vec<IncludedSectionMeta>,
    /// Sections excluded by budget pressure.
    pub excluded: Vec<ExcludedSectionMeta>,
    /// VCG diagnostics (populated only when strategy == Vcg).
    pub vcg_diagnostics: Option<AuctionDiagnostics>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct IncludedSectionMeta {
    pub name: String,
    pub bidder: AttentionBidder,
    pub estimated_tokens: usize,
    pub score: f32,
    pub vcg_payment: Option<f64>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ExcludedSectionMeta {
    pub name: String,
    pub bidder: AttentionBidder,
    pub estimated_tokens: usize,
    pub score: f32,
}
```

### Data Flow

```
                    PromptComposer::compose()
                    +---------------------------------+
                    | 1. Score all section signals     |
                    | 2. Check bidder observation count|
                    |    < 10 -> WeightedSum            |
                    |    >= 10 -> VCG path              |
                    | 3a. WeightedSum: sort by density,|
                    |     greedy fill (existing logic)  |
                    | 3b. VCG: build VcgBid vec from   |
                    |     LearningBidder.bid_with_cost()|
                    |     -> vcg_allocate()             |
                    |     -> winners + payments          |
                    | 4. Emit CompositionManifest as   |
                    |    tag on output Engram           |
                    +--------------+------------------+
                                   |
                                   v
                    Agent dispatches with prompt
                    +---------------------------------+
                    | Agent runs, produces response    |
                    | usage.input_tokens, cost_usd     |
                    +--------------+------------------+
                                   |
                                   v
                    orchestrate.rs: post-turn
                    +---------------------------------+
                    | 1. Read CompositionManifest from |
                    |    the dispatched prompt Engram  |
                    | 2. CostAttribution::from_turn()  |
                    |    using actual usage numbers    |
                    | 3. Run gate pipeline             |
                    | 4. attribution.stamp_gate_result()|
                    | 5. For each section in manifest: |
                    |    bidder.update_with_cost(       |
                    |      section, included, passed,  |
                    |      attributed_cost, tokens)    |
                    | 6. Append to cost log            |
                    +--------------+------------------+
                                   |
                                   v
                    Next composition uses updated
                    Beta posteriors + cost factors
```

### Persistence

```
.roko/learn/
+-- section-costs.json          # LearningBidder state with cost stats (per bidder)
+-- cost-attributions.jsonl     # Append-only log of CostAttribution records
+-- composition-strategy.json   # Current strategy + observation counts
```

---

## Implementation Plan

### Step 1: Add `SectionCostStats` to `LearningBidder`

**File**: `crates/roko-compose/src/auction.rs`

- Add `section_costs: HashMap<String, SectionCostStats>` field to `LearningBidder`.
- Add `SectionCostStats` struct (4 fields: total_cost_usd, total_tokens, observation_count, passes).
- Add `bid_with_cost()` method.
- Add `update_with_cost()` method.
- Add `cost_effectiveness_factor()` private method.
- Add `observation_count()` method returning minimum observations across tracked sections.
- Update `LearningBidder::new()` to initialize `section_costs: HashMap::new()`.
- Add serialization for `SectionCostStats` (already `Serialize, Deserialize`).

**Tests**:
- `learning_bidder_cost_factor_neutral_without_data`: verify factor is 1.0 with no cost data.
- `learning_bidder_cost_factor_rewards_cheap_effective_sections`: verify high-pass, low-cost section gets factor > 1.0.
- `learning_bidder_cost_factor_penalizes_expensive_ineffective_sections`: verify low-pass, high-cost section gets factor < 1.0.
- `bid_with_cost_scales_correctly`: verify `bid_with_cost = bid * cost_factor`.

### Step 2: Add `CompositionStrategy` and `CostAttribution` types

**New file**: `crates/roko-compose/src/strategy.rs`
- `CompositionStrategy` enum (WeightedSum, Vcg).
- `CompositionStrategy::auto_select()` method.

**New file**: `crates/roko-compose/src/cost_attribution.rs`
- `CostAttribution` struct.
- `SectionCost` struct.
- `CostAttribution::from_turn()` constructor.
- `CostAttribution::stamp_gate_result()` method.
- `CostAttribution::cost_effectiveness()` method.

**File**: `crates/roko-compose/src/lib.rs`
- Add `mod strategy;` and `mod cost_attribution;`.
- Add re-exports.

**Tests**:
- `auto_select_weighted_sum_when_cold`: verify < 10 observations selects WeightedSum.
- `auto_select_vcg_when_warm`: verify >= 10 observations selects Vcg.
- `cost_attribution_proportional`: verify token fractions sum to ~1.0.
- `cost_attribution_stamps_gate`: verify `stamp_gate_result` propagates.

### Step 3: Add `CompositionManifest` to compose output

**File**: `crates/roko-compose/src/prompt.rs`

- Add `CompositionManifest`, `IncludedSectionMeta`, `ExcludedSectionMeta` structs.
- In `PromptComposer::compose()` (the `Compose` trait impl starting at line 597):
  - After the greedy allocation loop, build a `CompositionManifest` with included/excluded sections.
  - Serialize it as a JSON tag `"composition_manifest"` on the output Engram.
- This is the *minimal* change: the existing WeightedSum path continues to work, but now
  it emits metadata.

**Tests**:
- `compose_emits_manifest_tag`: verify output Engram has `"composition_manifest"` tag.
- `manifest_lists_included_and_excluded`: verify section names match what was included/dropped.

### Step 4: Add VCG path to `PromptComposer::compose()`

**File**: `crates/roko-compose/src/prompt.rs`

- Add `composition_strategy: CompositionStrategy` field to `PromptComposer`.
- Add `with_strategy(strategy: CompositionStrategy) -> Self` builder method.
- Add `with_learning_bidders(bidders: HashMap<AttentionBidder, LearningBidder>) -> Self`.
- In `compose()`, after scoring signals:
  - If `self.composition_strategy == Vcg`:
    - For each scored section, build a `VcgBid` using the section's `LearningBidder::bid_with_cost()`.
    - Call `vcg_allocate(bids, budget, &affect_modulation)`.
    - Use `VcgAllocation::winners` as the included set.
    - Populate `CompositionManifest::vcg_diagnostics` from allocation diagnostics.
  - Else (WeightedSum): existing path, unchanged.

**Tests**:
- `compose_vcg_path_uses_vcg_allocate`: verify VCG diagnostics are populated.
- `compose_vcg_path_respects_budget`: verify total tokens <= budget.
- `compose_vcg_winners_ordered_by_density`: verify greedy value-density ordering.
- `compose_auto_selects_strategy`: verify that with enough observations, VCG activates.

### Step 5: Wire cost attribution into `orchestrate.rs`

**File**: `crates/roko-cli/src/orchestrate.rs`

- After agent dispatch returns (post-`dispatch_agent_with` call):
  - Parse `CompositionManifest` from the prompt Engram's tags.
  - Build `CostAttribution::from_turn()` using actual `usage.input_tokens` and `usage.cost_usd`.
- After gate pipeline runs:
  - Call `attribution.stamp_gate_result(gate_passed)`.
  - For each section in the manifest, call the corresponding `LearningBidder::update_with_cost()`.
- Append `CostAttribution` to `.roko/learn/cost-attributions.jsonl`.
- Persist updated `LearningBidder` state to `.roko/learn/section-costs.json`.

### Step 6: Auto-strategy selection at composition time

**File**: `crates/roko-cli/src/orchestrate.rs`

- Before building the prompt for each task:
  - Compute observation counts from loaded `LearningBidder` state.
  - Call `CompositionStrategy::auto_select()`.
  - Pass the strategy to `PromptComposer` via `.with_strategy()`.

### Step 7: Cost attribution feeds into VCG value estimates

**File**: `crates/roko-compose/src/auction.rs`

- When `LearningBidder::bid_with_cost()` is called during VCG allocation:
  - The cost-effectiveness factor automatically scales bids.
  - Sections that historically waste tokens get lower bids.
  - Sections that are cheap and effective get higher bids.
- This closes the feedback loop: cost attribution -> updated posteriors -> next VCG auction.

---

## Verification

### Unit tests

1. `LearningBidder` cost-awareness tests (Step 1) -- verify bid scaling by cost-effectiveness.
2. `CostAttribution` arithmetic tests (Step 2) -- verify proportional attribution sums to 1.0.
3. `CompositionManifest` emission tests (Step 3) -- verify tag on output Engram.
4. VCG path integration tests (Step 4) -- verify `vcg_allocate` is called and diagnostics populated.

### Integration tests

5. End-to-end composition test:
   - Build 8 sections with known token estimates.
   - Compose with WeightedSum (no observations) -- verify greedy allocation.
   - Feed 15 fake observations per bidder.
   - Compose again -- verify VCG path activates, payments computed.
   - Attribute costs from a fake turn.
   - Verify `LearningBidder` state reflects the cost attribution.
   - Compose a third time -- verify bids shifted by cost-effectiveness.

6. `cargo run -p roko-cli -- plan run plans/` with a small 3-task plan:
   - After completion, verify `.roko/learn/cost-attributions.jsonl` has entries.
   - Verify `.roko/learn/section-costs.json` has updated bidder state.
   - Verify strategy auto-selection logged in episode metadata.

### Diagnostic verification

7. VCG diagnostics in TUI: after a plan run, `roko dashboard` F4 (Learning tab) should
   show per-section VCG payments, displaced sections, and budget utilization.

### Regression verification

8. Existing `prompt.rs` tests must all pass unchanged -- the WeightedSum path is the
   default and must not change behavior.

---

## Rating

**Self-rating: 9.5/10**

Strengths:
- Reuses existing `vcg_allocate()` without rebuilding -- just wires it into the compose path.
- `CompositionStrategy::auto_select()` provides a clean cold-start to warm transition.
- Cost attribution is a simple proportional model (token share) that avoids over-engineering.
- The feedback loop is closed: cost -> posteriors -> bids -> next auction.
- Manifest metadata on the output Engram is the right integration point: it travels with
  the signal, no hidden state needed.
- The `LearningBidder` enhancement is backward-compatible: existing `bid()` and `update()`
  still work, `bid_with_cost()` and `update_with_cost()` are additive.

Limitations acknowledged:
- Token-proportional attribution is an approximation. Sections at the start of the prompt
  may have higher attention weight than sections in the middle (the "lost in the middle"
  effect). A more accurate model would use the `PositionAttentionModel` from `attention.rs`
  to weight attribution by position. This is a v2 enhancement.
- The 10-observation threshold for VCG activation is a tunable magic number. It should be
  configurable via `roko.toml` in a follow-up.

## Implementation Packet

This work activates the built VCG auction path and makes section-level attribution useful for learning.

### Required Context

- `crates/roko-compose/src/auction.rs`
- `crates/roko-compose/src/prompt.rs`
- `crates/roko-compose/src/attention.rs`
- `crates/roko-compose/src/context_provider.rs`
- `crates/roko-learn/src/section_effect.rs`
- `docs/03-composition/10-vcg-attention-auction.md`
- `docs/03-composition/05-token-budget-management.md`
- `tmp/unified/07-LEARNING.md`
- `tmp/unified-depth/01-signal/demurrage-economics.md`

### Target Files

- [ ] Update `crates/roko-compose/src/prompt.rs`.
- [ ] Update `crates/roko-compose/src/auction.rs` only if the public API is insufficient.
- [ ] Update `crates/roko-compose/src/context_provider.rs` to preserve bidder/source metadata.
- [ ] Update `crates/roko-learn/src/section_effect.rs` to consume allocation diagnostics.
- [ ] Add tests in `crates/roko-compose/tests/` or module tests.

### Checklist

- [ ] Add a `CompositionMode` enum: `DensityGreedy`, `Vcg`, and `Auto`.
- [ ] Add config plumbing so runtime can choose `Auto` by default.
- [ ] Build `VcgBid` values from existing `PromptSection` scores, token costs, priority, source, role, and affect bias.
- [ ] Call `vcg_allocate()` when mode is `Vcg` or when `Auto` has enough section-outcome history.
- [ ] Include `AuctionDiagnostics` in `PromptBuild` or a sidecar diagnostics struct.
- [ ] Persist selected section ids, dropped section ids, payments, and externality costs in the episode or efficiency event.
- [ ] Feed section outcome quality back into `SectionEffectivenessRegistry`.
- [ ] Keep the existing density path as fallback when auction diagnostics fail.
- [ ] Add a guard so VCG cannot exceed the same token budget enforced by the current composer.

### Acceptance Criteria

- [ ] Unit test: VCG allocation respects token budget.
- [ ] Unit test: VCG and density paths select the same sections in a simple single-bidder case.
- [ ] Unit test: diagnostics record displaced sections.
- [ ] Integration test: prompt assembly emits section diagnostics consumed by learning.
- [x] Search gate: `vcg_allocate` has at least one production call path.

## Worker 9 Evidence Checklist (2026-04-26)

Implemented in composition crates:

- [x] `crates/roko-compose/src/strategy.rs` defines `CompositionStrategy::{Auto,DensityGreedy,WeightedSum,Vcg}` and `DEFAULT_VCG_WARMUP_OBSERVATIONS`.
- [x] `crates/roko-compose/src/cost_attribution.rs` defines `CostAttribution`, `SectionCost`, `from_turn`, `stamp_gate_result`, and `cost_effectiveness`.
- [x] `crates/roko-compose/src/prompt.rs` includes `CompositionManifest` fields for requested strategy, selected strategy, VCG diagnostics, and included sections.
- [x] `PromptComposer::compose` has a production `vcg_allocate` call path when VCG is selected.
- [x] `crates/roko-compose/src/lib.rs` exports `CompositionStrategy`, `DEFAULT_VCG_WARMUP_OBSERVATIONS`, `CostAttribution`, `SectionCost`, `vcg_allocate`, and `AuctionDiagnostics`.

Still required before archive:

- [ ] Confirm the live runner persists selected/dropped section ids, payments, and externality costs in episodes or efficiency events; current runner-local episode writing does not show that proof.
- [ ] Feed section outcome quality back into `SectionEffectivenessRegistry` from the active runner path.
- [ ] Plumb runtime config so runner prompt assembly can explicitly choose `Auto`, `DensityGreedy`, or `Vcg`.
- [ ] Add integration proof that prompt assembly diagnostics are consumed by learning on a later run.
- [ ] Bridge the active `crates/roko-cli/src/dispatch/prompt_builder.rs::PromptAssembler` diagnostics to the richer `roko-compose` `CompositionManifest` / `CostAttribution` schema, or replace the CLI assembler with the compose pipeline.

## 9. 2026-04-27 Deepening Pass - Active Prompt Diagnostics And Cost Loop

Self-grade for this pass:

- Initial rating: 9.90 / 10.
- Reasoning: this pass corrects stale VCG/cost-attribution claims and identifies the active-runner split: `roko-compose` has the richer auction machinery, while `roko-cli` dispatch currently emits simpler prompt diagnostics. The remaining work is now framed as concrete bridge/replacement batches with generated proof.

### 9.1 Source-Corrected Status

- [x] `crates/roko-compose/src/strategy.rs` defines `CompositionStrategy` with `Auto`, `DensityGreedy`, `WeightedSum`, and `Vcg`.
- [x] `crates/roko-compose/src/cost_attribution.rs` defines `CostAttribution` and `SectionCost`.
- [x] `crates/roko-compose/src/prompt.rs` defines `CompositionManifest` and has a production `vcg_allocate` path.
- [x] `crates/roko-compose/src/auction.rs` has cost-aware `LearningBidder` state.
- [x] `crates/roko-compose/src/lib.rs` exports the strategy, attribution, VCG, and manifest surfaces.
- [x] `crates/roko-cli/src/dispatch/prompt_builder.rs` defines the active runner `PromptAssembler`.
- [x] The active runner constructs `PromptAssembler::new()` before dispatch.
- [x] Active runner prompt diagnostics include included sections, dropped sections, knowledge ids, and playbook ids.
- [x] Runner projection can observe prompt diagnostics counts.
- [ ] Active runner prompt diagnostics do not carry `CompositionManifest` strategy, section ids, bidder ids, estimated tokens, VCG payments, or externality costs.
- [ ] Active runner does not prove cost attribution after real provider usage.
- [ ] Active runner does not prove `SectionEffectivenessRegistry` receives section outcome/cost updates from prompt diagnostics.
- [ ] Runtime config does not prove selectable `Auto`, `DensityGreedy`, or `Vcg` prompt composition mode for active dispatch.

### 9.2 Correct Target Shape

Prompt composition should have one canonical diagnostics schema:

- [ ] Every prompt section has a stable section id.
- [ ] Every prompt section has an owner/bidder label.
- [ ] Every prompt section has estimated token cost.
- [ ] Every prompt assembly records requested strategy and selected strategy.
- [ ] Every prompt assembly records included sections and dropped sections.
- [ ] VCG assemblies record payments, displaced sections, and externality costs.
- [ ] Every provider turn records actual input tokens and cost.
- [ ] Gate outcome stamps the prompt section attribution.
- [ ] Section outcome/cost updates feed `SectionEffectivenessRegistry` or successor learning store.
- [ ] Later prompt assembly consumes the learned section effects.

### 9.3 Implementation Batches

#### CA-01: Choose Bridge Or Replacement

- [ ] Decide whether `crates/roko-cli/src/dispatch/prompt_builder.rs` should become a thin adapter over `roko-compose`, or whether it should emit a compatible manifest itself.
- [ ] If adapter: convert current runner prompt sections into `roko-compose::PromptSection`.
- [ ] If compatible manifest: add section id, bidder, estimated token, requested strategy, selected strategy, and diagnostics fields to CLI `PromptDiagnostics`.
- [ ] Preserve current included/dropped/knowledge/playbook diagnostics.
- [ ] Add tests showing active dispatch diagnostics can be converted into `CompositionManifest` shape.

#### CA-02: Runtime Composition Strategy Config

- [ ] Add runtime config for prompt composition mode: `auto`, `density_greedy`, `weighted_sum`, `vcg`.
- [ ] Default to `auto`.
- [ ] Thread config into `PromptAssembler` or compose adapter.
- [ ] Make `Auto` use section-effect observation counts.
- [ ] Add tests proving explicit `vcg` selects VCG path when possible.
- [ ] Add tests proving fallback to density-greedy when VCG diagnostics fail.

#### CA-03: Section Identity And Token Estimates

- [ ] Assign stable ids to role, task, files, acceptance, verify, retry, allowed tools, denied tools, knowledge, episode knowledge, playbooks, section-effectiveness, and dream sections.
- [ ] Assign bidder labels to each section.
- [ ] Estimate token counts before budget enforcement.
- [ ] Persist included and dropped sections with section ids and token estimates.
- [ ] Add tests proving dropped sections retain diagnostics even when omitted from final prompt.

#### CA-04: VCG Diagnostics In Active Runner

- [ ] Build `VcgBid` values from active prompt sections when selected strategy is `Vcg`.
- [ ] Call `vcg_allocate` through the compose crate or equivalent adapter.
- [ ] Persist selected strategy, payments, displaced sections, and externality costs.
- [ ] Ensure selected prompt never exceeds the same token budget as the existing path.
- [ ] Add deterministic fixture proving VCG diagnostics are present in active runner prompt diagnostics.

#### CA-05: Provider Usage To Cost Attribution

- [ ] Capture real provider input tokens, output tokens, and cost for each dispatch.
- [ ] Link provider usage to the prompt diagnostics id.
- [ ] Build `CostAttribution::from_turn` from included sections and actual usage.
- [ ] Stamp gate result after gate completion.
- [ ] Append attribution records under `.roko/learn/cost-attributions.jsonl`.
- [ ] Store proof in `tmp/mori-diffs/generated/composition-cost-attribution-proof.json`.

#### CA-06: Section Effectiveness Feedback

- [ ] Convert stamped cost attribution into `SectionEffectivenessRegistry` updates.
- [ ] Include pass/fail, attributed cost, token estimate, section id, and bidder.
- [ ] Persist updated section effects.
- [ ] Make later prompt assembly load those effects.
- [ ] Prove a second run observes section-effect changes from the first run.
- [ ] Store proof in `tmp/mori-diffs/generated/composition-section-effect-proof.json`.

#### CA-07: Query And Projection

- [ ] Emit durable prompt diagnostics events with manifest id.
- [ ] Expose prompt diagnostics through HTTP or CLI query.
- [ ] Expose cost attribution records through HTTP or CLI query.
- [ ] Expose section-effectiveness snapshot through HTTP or CLI query.
- [ ] Add projection digest proof for prompt diagnostics.
- [ ] Store evidence in `tmp/mori-diffs/generated/composition-query-proof.json`.

### 9.4 Generated Proof Contract

An agent implementing this file must produce `tmp/mori-diffs/generated/composition-auction-proof-report.json`:

```json
{
  "schema": "mori-diffs.composition-auction-proof.v1",
  "generated_at": "ISO-8601 timestamp",
  "git_commit": "HEAD sha",
  "active_prompt_manifest": {
    "proved": false,
    "manifest_id": null,
    "requested_strategy": null,
    "selected_strategy": null,
    "included_sections": [],
    "dropped_sections": []
  },
  "vcg": {
    "production_path_proved": false,
    "payments_recorded": false,
    "externality_costs_recorded": false,
    "budget_respected": false
  },
  "cost_attribution": {
    "provider_usage_linked": false,
    "attribution_written": false,
    "gate_result_stamped": false
  },
  "learning": {
    "section_effects_updated": false,
    "second_run_consumed_effects": false
  },
  "queries": {
    "prompt_diagnostics": false,
    "cost_attribution": false,
    "section_effectiveness": false
  },
  "remaining_gaps": []
}
```

### 9.5 No-Context Handoff Checklist

Use this exact order:

- [ ] Run `rg -n "PromptAssembler|PromptDiagnostics|CompositionManifest|CompositionStrategy|CostAttribution|vcg_allocate|SectionEffectivenessRegistry|included_sections|dropped_sections|cost-attributions|section-effects" crates`.
- [ ] Implement CA-01 before adding any new prompt diagnostics fields.
- [ ] Implement CA-02 before claiming configurable composition.
- [ ] Implement CA-03 before cost attribution.
- [ ] Implement CA-04 before claiming active VCG.
- [ ] Implement CA-05 before learning updates.
- [ ] Implement CA-06 before claiming the feedback loop is closed.
- [ ] Implement CA-07 before claiming observability/query parity.
- [ ] Generate `tmp/mori-diffs/generated/composition-auction-proof-report.json`.
- [ ] Update [README.md](README.md), [20-RUNTIME-RECONCILIATION.md](20-RUNTIME-RECONCILIATION.md), [21-FEATURE-PARITY-MATRIX.md](21-FEATURE-PARITY-MATRIX.md), [29-CURRENT-RUNTIME-GAP-LEDGER.md](29-CURRENT-RUNTIME-GAP-LEDGER.md), and [38-COGNITIVE-FEEDBACK-LOOP-AUDIT.md](38-COGNITIVE-FEEDBACK-LOOP-AUDIT.md).

### 9.6 Archive Gate

Do not archive this file until:

- [ ] Active runner prompt assembly emits manifest-compatible diagnostics.
- [ ] Active VCG path is proved or explicitly disabled by config with documented reason.
- [ ] Real provider usage is linked to prompt diagnostics.
- [ ] Cost attribution is written and gate-stamped.
- [ ] Section effectiveness is updated from attribution.
- [ ] A later run consumes prior section-effectiveness data.
- [ ] Prompt diagnostics, attribution, and section effects are queryable.
