# Integration Points

> Depth file for [11-AFFECT.md](../../11-AFFECT.md) -- v1 source: `docs/v1/09-daimon/10-integration-points.md`

---

## Overview

The Daimon's PAD vector is not a display value. It is a control signal that drives
four systems simultaneously. Each system reads a different projection of the same
PAD state:

1. **Behavioral state selection** -- maps PAD to one of six discrete states
2. **Tier routing bias** -- modulates CascadeRouter prediction error thresholds
3. **VCG auction bidding** -- biases context window allocation
4. **Somatic landscape querying** -- fast heuristic pre-evaluation

These are not independent features -- they are different consumers of the same
signal. Changing the PAD vector changes all four simultaneously. This is the
Daimon's architectural contribution: a single emotional state creates coherent
behavioral change across the entire cognitive pipeline.

---

## Integration Point 1: Behavioral State Selection

### Mechanism

```
PAD -> classify_behavioral_state() -> BehavioralState
```

### What It Drives

- **Self-model**: the agent's internal representation of its cognitive state,
  injected into the `<daimon>` context block in LLM prompts via SystemPromptBuilder
- **TUI display**: behavioral state label in the dashboard
- **Conversational tone**: each state maps to a PAD octant, which maps to a
  conversational style (anxious agents hedge extensively; confident agents use
  definitive language)

### Implementation

`DaimonState::modulate()` implements behavioral state selection through PAD
threshold checks, selecting DispatchStrategy, adjusting turn limits, and
promoting/demoting models.

---

## Integration Point 2: Tier Routing Bias

### Mechanism

```
PAD -> behavioral_state -> adjusted_thresholds(state) -> CascadeRouter.select_tier()
```

### What It Drives

- **Model selection**: which LLM processes this operation
- **Compute cost**: T2 costs approximately 60x T0
- **Response quality**: stronger models produce better results on complex tasks
- **Latency**: T2 takes 10-30 seconds vs. T0 at ~1ms

### Cost Impact

| Transition | Cost Change | Trigger |
|---|---|---|
| Engaged -> Struggling | ~3.5x increase | Sustained failures |
| Engaged -> Coasting | ~0.4x decrease | Sustained successes |
| Engaged -> Resting | ~0.5x decrease | Idle periods |
| Struggling -> Engaged | ~0.3x decrease | Recovery |
| Coasting -> Engaged | ~2.5x increase | Harder problems |

---

## Integration Point 3: VCG Auction Bidding

### Mechanism

The VCG (Vickrey-Clarke-Groves) auction allocates the limited context window among
competing subsystems. The Daimon biases bidding:

```
bid = expected_value x urgency x affect_weight

where:
  urgency = 1 + arousal x 0.5
  affect_weight = 1 + 0.3 x abs(pleasure - 0.5)
```

### How Affect Modulates Bidding

**High arousal -> increased urgency**: at arousal = 0.8, urgency = 1.4 (40% boost).
Urgent situations receive richer context.

**Extreme pleasure -> increased affect weight**: both very positive and very
negative states increase emotionally-relevant context weight:

- High pleasure (P = 0.8): affect_weight = 1.09 (modest)
- Low pleasure (P = -0.3): affect_weight = 1.24 (stronger -- failure increases
  diagnostic context weight)

### Per-Subsystem Bidding

| Subsystem | High Arousal | Low Dominance | Low Pleasure |
|---|---|---|---|
| **Neuro** (knowledge) | Safety knowledge prioritized | Exploratory knowledge boosted | Warning knowledge boosted |
| **Daimon** (affect context) | Bid increases | Bid increases | Bid increases |
| **Iteration memory** | Boost for recent failures | Neutral | **Strong boost** |
| **Code intelligence** | Safety-critical code paths | Neutral | Neutral |
| **Playbook rules** | Proven playbooks | Novel playbooks | Conservative playbooks |
| **Research artifacts** | Neutral | **Strong boost** | Neutral |
| **Task context** | Deadline plan sections | Neutral | Neutral |

### VCG Truthfulness

The VCG mechanism uses second-price payment: each winner pays the next-highest
loser's bid. The Daimon's affect modulation changes the actual value of context
(not just the bid), so it does not undermine truthfulness.

**Citation**: Vickrey (1961), Clarke (1971), Groves (1973).

---

## Integration Point 4: Somatic Landscape Querying

### Mechanism

```
Strategy coords -> SomaticLandscape.query() -> SomaticSignal -> pre-analytical bias
```

### What It Drives

- **Pre-filter on strategy space**: narrows actions before analytical reasoning
- **Model tier suggestion**: negative valence -> suggest T2; positive -> T0/T1
- **Review scrutiny**: negative valence increases gate rung level
- **Explore vs. exploit**: negative -> proven playbooks; positive -> permit exploration

### Timing in the Cognitive Pipeline

The somatic query is the fastest decision signal:

```
1. SOMATIC QUERY       (< 1ms)     -- k-d tree nearest neighbor
2. PREDICTION PROBES   (< 5ms)     -- 16 deterministic probes
3. TIER SELECTION      (~0ms)      -- threshold comparison
4. CONTEXT ASSEMBLY    (~10ms)     -- VCG auction, retrieval
5. MODEL INFERENCE     (~2-30s)    -- LLM call at selected tier
```

The somatic query can preempt tier selection: a strongly negative somatic signal
forces T2 before the prediction error probes run. This is the System 1 fast path.

---

## Integration Map

```
                    PAD Vector
                  /    |    \
                 /     |     \
                /      |      \
               /       |       \
    Behavioral      Tier         VCG        Somatic
     State         Routing      Auction     Landscape
       |             |            |            |
       v             v            v            v
   Self-model    CascadeRouter  Context     Pre-filter
   TUI display   Model select   assembly    Fast bias
   Tone map      Cost control   Token alloc  Strategy
```

All four paths read the same PAD state. A change to pleasure, arousal, or dominance
cascades through all four integration points simultaneously.

---

## Event Emission

| Event | Trigger | Consumers |
|---|---|---|
| `MoodUpdate` | PAD Euclidean delta > 0.15 | TUI, episode logger, clients |
| `DaimonAppraisal` | Every appraisal completes | TUI, episode logger, metrics |
| `SomaticMarkerFired` | strong match (valence > 0.3, intensity > 0.5) | WS/SSE, TUI |
| `EmotionalShift` | Dominant Plutchik emotion changes | TUI, notifications |

The 0.15 threshold prevents event flooding. A Relaxed-to-Anxious shift (distance
~1.2) always emits. Within-octant fluctuations (distance ~0.05) do not.

---

## Implementation Status

- **Point 1 (behavioral state)**: fully implemented in `roko-daimon`
- **Point 2 (tier routing)**: live PAD feeds CascadeRouter bias and
  SystemPromptBuilder affect guidance
- **Point 3 (VCG auction)**: partially implemented -- orchestration passes live PAD
  into PromptComposer, urgency/affect weighting and per-bidder modulation are wired
- **Point 4 (somatic landscape)**: partially implemented -- the 8D k-d tree is
  live, task execution projects strategies, and the somatic signal feeds routing,
  prompting, and events

---

## Cross-References

- `six-behavioral-states.md` -- behavioral state definitions
- `behavioral-state-to-tier-routing.md` -- tier routing details
- `somatic-markers-damasio.md` -- somatic landscape
- `coding-agent-integration.md` -- coding-specific integration
- `daimon-state-and-affect-engine.md` -- the DaimonState implementation
