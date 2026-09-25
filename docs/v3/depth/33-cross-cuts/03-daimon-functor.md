# 33-03 -- Daimon Functor (F_daimon)

> **Parent**: [33-CROSS-CUTS](../../33-CROSS-CUTS.md)
>
> `DaimonFunctor` modulates the cognitive loop with affect, somatic markers, and
> prospect-theory valuation. It biases ASSESS with PAD state and escalates tiers
> under anxiety; it gates risky ACT decisions and appraises outcomes asymmetrically.
> This depth file covers the enrichment protocol, the behavioral-state vocabulary,
> and the prospect-theory parameters.

**Authority**: `crates/roko-compose/src/daimon_functor.rs`

---

## 1. Construction and State Binding

```rust
pub struct DaimonFunctor {
    state: Arc<RwLock<DaimonState>>,
    thresholds: BehavioralStateThresholds,
    somatic_config: SomaticRetrievalConfig,
}
```

The functor binds to the runtime `DaimonState` through a shared read-write lock.
The `thresholds` control when behavioral state transitions occur (e.g.,
`struggling_entry_dominance`). The `somatic_config` controls retrieval parameters
including the mandatory 15% contrarian fraction.

The lock is held for the minimum duration: reads are released before Signal
construction, and writes (in `act_post`) are released before the post-enrichment
Signal is built. Lock poisoning returns `ComposeError::Other` rather than panicking.

---

## 2. BehavioralState Vocabulary

The canonical `roko_core::BehavioralState` enum has six variants:

| Variant | Meaning | Typical PAD range |
|---------|---------|-------------------|
| `Engaged` | Normal productive state | Moderate pleasure, moderate arousal |
| `Struggling` | Difficulty, repeated failures | Low pleasure, high arousal, low dominance |
| `Coasting` | Low-effort, routine work | Moderate pleasure, low arousal |
| `Exploring` | Novelty-seeking, creative | High arousal, moderate dominance |
| `Focused` | Deep concentration | Low arousal, high dominance |
| `Resting` | Recovery, cool-down | Low arousal, low pleasure |

There is no `Neutral`, `Cautious`, or `Anxious` variant. In this chapter:
- "neutral" means the PAD short-circuit region (all three dimensions < 0.1 absolute)
- "cautious" means `Struggling` or dominance below the configured threshold
- "anxious" means high arousal + low dominance (the tier-escalation trigger)

---

## 3. ASSESS Enrichment

### 3.1 Pre-Enrichment: PAD and Somatic Injection

The pre-hook injects two metadata Signals:

**PAD Signal** (`roko.cross_cut.daimon.pad`):
```json
{
    "pleasure": -0.3,
    "arousal": 0.6,
    "dominance": -0.2,
    "behavioral_state": "struggling"
}
```

**Somatic Signal** (`roko.cross_cut.daimon.somatic`):
```json
{
    "valence": 0.4,
    "intensity": 0.7,
    "neighbor_count": 8,
    "contrarian_count": 2,
    "contrarian_applied": true,
    "contrarian_fraction_required": 0.15
}
```

Somatic retrieval uses `StrategyCoordinates` derived from input Signals, querying
the 8-dimensional k-d tree with eight axes: complexity, risk, novelty, confidence,
time_pressure, scope, reversibility, and dependency_depth.

### 3.2 Post-Enrichment: Tier Escalation

After ASSESS, the post-hook checks for high-anxiety conditions:

```
escalate = (arousal > 0.5) AND (dominance < struggling_entry_dominance)
```

When triggered, a tier-escalation Signal is injected requesting at minimum
`T2Reflective`. This prevents cheap, fast models from being used when the affect
state indicates the situation requires deliberate reasoning.

---

## 4. ACT Enrichment

### 4.1 Pre-Enrichment: Risk Gating

The deferral condition:

```
cautious = (behavioral_state == Struggling)
        OR (pad.dominance < struggling_entry_dominance)
```

When cautious, the functor scans input Signals for high-risk markers:
- Tag `risk_level` containing "high" or "critical" (case-insensitive)
- JSON body field `risk` with value > 0.5 or string "high"/"critical"

If any high-risk Signal is found, a deferral recommendation is injected with:
- `recommendation_source: "daimon"`
- `decision_kind: "act"`
- `recommendation_value: "defer"`
- `safety_critical: "true"`
- `priority_level: "1"`

The `safety_critical` and `priority_level: 1` tags ensure this deferral wins any
arbitration -- Daimon safety overrides are structurally at the highest level.

### 4.2 Post-Enrichment: Prospect-Theory Appraisal

After ACT, the post-hook extracts a reward pair from output Signals:
- `actual`: from JSON `reward` or `outcome` field
- `reference`: from JSON `expected_reward` or `reference` field (default: 0.5)

The prospect value is computed using the Tversky-Kahneman (1992) formula:

```
v(x) = x^alpha           if x >= 0    (alpha = 0.88)
v(x) = -lambda * |x|^alpha   if x < 0     (lambda = 2.25)
```

Where x = actual - reference. The asymmetry ensures that losses are felt 2.25x more
strongly than equivalent gains, matching the empirical loss-aversion coefficient.

The computed value drives `DaimonState::appraise(AffectEvent::TaskOutcome { ... })`,
which shifts the PAD vector and potentially transitions the behavioral state.

---

## 5. Short-Circuit Semantics

`should_short_circuit()` returns true when PAD is in the neutral region:

```rust
pad.pleasure.abs() < 0.1 && pad.arousal.abs() < 0.1 && pad.dominance.abs() < 0.1
```

This predicate over the continuous PAD space is not a `BehavioralState` variant.
When true, the Daimon has nothing meaningful to contribute (no bias, no gating, no
somatic markers of note), and the functor can be skipped.

---

## 6. Key Tests

| Test | What it verifies |
|------|-----------------|
| `losses_are_weighted_more_than_equal_gains` | `prospect_value(0.3, 0.5).abs() > prospect_value(0.7, 0.5) * 2.2` |
| `struggling_state_defers_high_risk_action` | Deferral Signal with safety_critical tag injected |
| `anxious_assessment_escalates_tier` | Tier-escalation Signal on high arousal + low dominance |

---

## References

- See [11-AFFECT](../../11-AFFECT.md) for the full Daimon specification.
- Tversky, A. & Kahneman, D. (1992). "Advances in Prospect Theory." *J. Risk Uncertainty*, 5(4).
- Damasio, A. (1994). *Descartes' Error*. Somatic marker hypothesis.
- Mehrabian, A. (1996). "Pleasure-Arousal-Dominance." *Current Psychology*, 14(4).
