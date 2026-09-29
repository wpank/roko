# 33-06 -- Conflict Arbitration

> **Parent**: [33-CROSS-CUTS](../../33-CROSS-CUTS.md)
>
> When Memory, Daimon, and Dreams produce conflicting recommendations for the same
> decision, the system resolves the conflict through a two-layer protocol: fixed
> priority hierarchy followed by VCG tiebreaker. This depth file covers the
> recommendation format, priority rules, VCG mechanics, the full arbitrator pipeline,
> and the legacy greedy composition strategy.

**Authority**: `crates/roko-compose/src/auction.rs`

---

## 1. Recommendation Format

Cross-cut recommendations are encoded as Signal tags following a shared contract.
`CrossCutRecommendation::from_signal()` parses them:

| Tag | Meaning | Values |
|-----|---------|--------|
| `recommendation_source` | Which cross-cut | "memory", "daimon", "dreams" |
| `decision_kind` | Type of decision | "route", "compose", "act", "other" |
| `decision_key` | Conflict group key | Arbitrary stable string |
| `recommendation_value` | Concrete recommendation | Domain-specific |
| `recommendation_confidence` | Truthful bid | [0.0, 1.0] |
| `priority_level` | Arbitration level | 1 (Daimon) / 2 (Memory) / 3 (Dreams) |
| `safety_critical` | Daimon override flag | "true" or absent |
| `knowledge_tier` | Memory validation tier | "transient"/"working"/"consolidated"/"persistent" |

Two recommendations **conflict** when they share the same `decision_key` but have
different `recommendation_value`.

---

## 2. Layer 1: Priority Hierarchy

`resolve_by_priority()` checks two conditions in order:

### 2.1 Daimon Safety Override

If any recommendation has `source == Daimon` and `safety_critical == true`, it wins
immediately. This covers risk-gating deferrals in the ACT phase. The confidence value
is irrelevant -- safety-critical Daimon overrides always win.

### 2.2 Consolidated Memory Override

If a Memory recommendation has `knowledge_tier` at `Consolidated` or `Persistent` and
conflicts with a Dreams recommendation, Memory wins. The rationale: validated knowledge
at high tiers represents empirically confirmed understanding, while Dreams output is
speculative hypothesis.

If neither condition applies, `resolve_by_priority()` returns `None` and the system
falls through to VCG.

---

## 3. Layer 2: VCG Tiebreaker

`resolve_by_vcg()` applies a classical second-price auction mechanism.

### 3.1 Eligibility

A recommendation is VCG-eligible when:

1. `confidence > 0.5` (low-confidence bids are ignored)
2. `decision_kind` is `Route` or `Compose` (not `Act` or `Other`)
3. There exists another eligible recommendation from a *different* source with the
   *same* priority level that conflicts on the same `decision_key`

### 3.2 Resolution

Eligible recommendations are sorted by confidence (descending), with source ID as
tiebreaker. The highest-confidence recommendation wins. The **attention cost** is the
second-highest confidence -- the VCG payment.

```rust
CrossCutArbitrationResult::Resolved {
    winner: CrossCutId,
    recommendation: CrossCutRecommendation,
    attention_cost: f64,        // second-highest confidence
    runner_up: Option<CrossCutId>,
    mechanism: ArbitrationMechanism::Vcg,
}
```

### 3.3 Truthfulness Guarantee

The VCG mechanism ensures truthful bidding through mechanism design (Vickrey 1961):
a cross-cut gains nothing by inflating its confidence because the price it pays is
determined by the *second-highest* bid, not its own. Overbidding risks winning when
the recommendation is low-quality; underbidding risks losing when it should win.
Truthful reporting is the dominant strategy.

### 3.4 When VCG Produces NoConflict

VCG returns `NoConflict` when:
- No two eligible recommendations share a `decision_key` with different values
- Conflicting recommendations are at different priority levels (handled by priority)
- All candidates have confidence <= 0.5

---

## 4. The CrossCutArbitrator Pipeline

`CrossCutArbitrator` composes the full resolution pipeline:

```rust
pub struct CrossCutArbitrator {
    pub memory: Arc<MemoryFunctor>,
    pub daimon: Arc<DaimonFunctor>,
    pub dreams: Arc<DreamsFunctor>,
    safety_filter: Arc<dyn CrossCutFunctor<CrossCutContext>>,
}
```

The `arbitrate()` method:

1. Memory pre-enrichment (inject knowledge recommendations)
2. Daimon pre-enrichment (inject PAD/somatic/deferral recommendations)
3. Dreams pre-enrichment (identity -- no injection)
4. Safety pre-filter (remove capability-violating Signals)
5. Parse `CrossCutRecommendation` from surviving Signals
6. `resolve_by_priority()` -- if resolved, return
7. `resolve_by_vcg()` -- final resolution or NoConflict

The ordering ensures Safety runs *after* enrichment but *before* recommendation
collection. A forbidden recommendation is removed before it can enter the bidding.

---

## 5. Prompt-Budget Composition Strategy

Orthogonal to cross-cut arbitration, the `PromptComposer` allocates prompt tokens
using a `CompositionStrategy`:

| Strategy | Mechanism | When used |
|----------|-----------|-----------|
| `Auto` | Switches from DensityGreedy to Vcg after warmup | Default |
| `DensityGreedy` | Sort by score/tokens, include greedily | Cold start, explicit |
| `WeightedSum` | Alias for DensityGreedy | Legacy compat |
| `Vcg` | VCG allocation with affect modulation | After warmup |

### 5.1 VCG Prompt Allocation

`vcg_allocate()` in `auction.rs` handles prompt-budget allocation:

1. Bids are sorted by value density (`adjusted_bid / tokens`), descending
2. Sections are included greedily until the budget is exhausted
3. VCG payments are computed: each winner's payment is the highest excluded bid
   that would have fit in the winner's token footprint
4. Pareto optimality is checked via the simple swap condition
5. Full diagnostics are returned (welfare, payments, utilization, displacement)

### 5.2 Affect Modulation

`AffectModulation` derives bid adjustments from the PAD state:
- `urgency_multiplier = (1.0 + arousal * 0.5).clamp(0.5, 2.0)`
- `affect_weight = pleasure.clamp(-1.0, 1.0)`
- `adjusted_bid = base_bid * urgency * (1 + affect_weight * valence)`

High arousal increases urgency (more context). Positive pleasure biases toward
positive-valence entries. The modulation is applied before density sorting.

### 5.3 LearningBidder

`LearningBidder` maintains per-section Beta-posterior parameters and updates them
on task outcomes:

- Gate pass: alpha += 1 (success count)
- Gate fail: beta += 1 (failure count)
- Bids use a Thompson-like deterministic approximation (posterior mean + variance-scaled offset)
- Cost-effectiveness factors reward cheap, effective sections and penalize expensive, ineffective ones

The bidder also tracks `round_observations` independently of section outcomes,
preventing a consistently excluded bidder from staying cold forever.

---

## 6. Key Tests

| Test | What it verifies |
|------|-----------------|
| `priority_layer_protects_safety_and_consolidated_memory` | Safety-critical Daimon wins; Consolidated Memory beats Dreams |
| `cross_cut_vcg_is_same_level_high_confidence_and_second_price` | Same-level conflict resolved; different-level returns NoConflict; winner pays second price |
| `safety_prefilter_removes_forbidden_bid_before_collection` | Shell-requiring recommendation filtered before arbitration |
| `learning_bidder_updates_posterior` | Gate pass increases bid value |
| `vcg_allocate_value_density_wins` | High value-density section allocated first |

---

## References

- Vickrey, W. (1961). "Counterspeculation, Auctions, and Competitive Sealed Tenders." *J. Finance*.
- Clarke, E. H. (1971). "Multipart Pricing of Public Goods." *Public Choice*.
- Groves, T. (1973). "Incentives in Teams." *Econometrica*.
