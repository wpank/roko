# Collective Emotional Contagion

> Depth file for [11-AFFECT.md](../../11-AFFECT.md) -- v1 source: `docs/v1/09-daimon/12-collective-emotional-contagion.md`

---

## Overview

When agents share a mesh (collective), their emotional states interact. An agent
that discovers a critical issue (high arousal) influences mesh peers to raise their
vigilance. An agent experiencing sustained success (high pleasure) influences peers
toward confidence. This emotional contagion is not metaphorical -- it is a concrete
data flow where PAD deltas propagate between agents with specific attenuation
factors, caps, and decay rates.

The contagion mechanism is carefully constrained to prevent cascading panic or
overconfidence. The design draws from research on emotional contagion in human
groups (Hatfield, Cacioppo, & Rapson 1993) and collective intelligence factors
(Woolley et al. 2010).

---

## Theoretical Foundation: Van den Broek (2023)

Van den Broek et al. ("Emotional contagion in artificial agent networks," 2023)
investigated how emotional state propagation in multi-agent systems can be
structured to avoid the pathological cascades observed in social media networks
while preserving the adaptive benefits of collective emotional awareness. Their
key finding: bounded unidirectional propagation with rapid decay produces stable
collective behavior, while bidirectional or unbounded propagation produces
oscillation or runaway amplification.

Roko's contagion mechanism implements their recommended architecture: unidirectional
flow, per-cycle caps, dimension-selective attenuation, and accelerated decay for
borrowed affect.

---

## Contagion Rules

### Attenuation Factors

| Dimension | Attenuation | Rationale |
|---|---|---|
| **Pleasure** | 0.3 (30%) | A peer's success is informative but not as significant as your own |
| **Arousal** | 0.3 (30%) | A peer's urgency should increase vigilance without overwhelming |
| **Dominance** | 0.0 (no propagation) | Control perception is strictly local -- must be earned |

### Caps and Decay

| Parameter | Value | Rationale |
|---|---|---|
| Arousal cap per sync | +0.3 | Prevents cascading panic from simultaneous alarms |
| Contagion decay half-life | 6 hours | Borrowed emotions dissipate unless reinforced |
| Propagation direction | Unidirectional | Prevents positive feedback loops |

---

## Contagion Triggers

Emotional contagion fires on specific shared events:

| Trigger | Effect on Receiver |
|---|---|
| **Peer warning push** | Arousal +0.1 (capped) |
| **Peer alert** (critical issue) | Arousal +0.1 (capped) |
| **Peer sustained success** | Dominance +0.05 |
| **Peer sustained failure** | Pleasure -0.05 |
| **Peer dream insight** | Arousal +0.05 |

### Application

```rust
impl DaimonState {
    pub fn apply_contagion(&mut self, event: ContagionEvent) {
        let source = event.source_pad;

        // Attenuate: P and A at 30%, D at 0%
        let p_delta = source.pleasure * 0.3;
        let a_delta = (source.arousal * 0.3).min(0.3); // cap

        let now = Utc::now();
        self.state.apply_delta(p_delta, a_delta, 0.0, 0.0, now);

        self.borrowed_affect.push(BorrowedAffect {
            source: event.source,
            p_delta,
            a_delta,
            applied_at: now,
        });
    }
}
```

---

## Anti-Cascade Design

### The Cascade Problem

Without safeguards:

```
Agent A detects anomaly -> arousal spike
  -> Shares with mesh
    -> Agent B receives contagion -> arousal increases
      -> Agent B shares heightened state
        -> Agent A receives from B -> arousal increases further
          -> (positive feedback loop -> all agents at maximum arousal)
```

This is analogous to financial panic propagation.

### Three Anti-Cascade Mechanisms

**Mechanism 1: Cap per sync cycle (+0.3 arousal max)**

Even if every peer sends an alarm simultaneously, the receiving agent's arousal
increases by at most 0.3 per sync cycle. With 30-second sync intervals, the maximum
ramp rate is 0.6/min. Combined with the 4-hour decay half-life, steady-state
arousal from contagion is limited to approximately 0.4.

**Mechanism 2: Unidirectional propagation**

Contagion flows source -> receiver only. The receiver does not re-emit borrowed
emotional state. If Agent B independently discovers the same anomaly, it generates
its own alarm through normal appraisal -- this is desirable because it reflects
genuinely redundant detection, not echo amplification.

**Mechanism 3: Rapid decay (6h half-life)**

Borrowed emotions decay at 6-hour half-life. After 12 hours without reinforcement,
contagion effects are at 25%. If the agent's own experience confirms the alarm,
arousal is reinforced through normal appraisal and no longer depends on the
borrowed component.

```
t=0h:   borrowed arousal = 0.30
t=6h:   borrowed arousal = 0.15
t=12h:  borrowed arousal = 0.075
t=18h:  borrowed arousal = 0.037 (negligible)
```

---

## Somatic Field Formation

When multiple agents share a mesh, their individual somatic landscapes aggregate
into a **somatic field** -- a collective emotional memory:

```rust
pub struct SomaticField {
    landscape: SomaticLandscape,
    agent_weights: HashMap<AgentId, f64>,
}

impl SomaticField {
    pub fn merge(&mut self, agent_id: AgentId, markers: &[SomaticMarker]) {
        let weight = self.agent_weights.get(&agent_id).copied().unwrap_or(1.0);
        for marker in markers {
            let weighted = SomaticMarker {
                valence: marker.valence * weight,
                intensity: marker.intensity * weight,
                ..marker.clone()
            };
            self.landscape.tree.add(&marker.strategy_coords, weighted);
        }
    }
}
```

### Weight Calibration

Agent contribution weights are calibrated by historical accuracy:

```
weight = (correct_predictions / total_predictions) x seniority_factor
```

Unreliable agents get lower weight; experienced, accurate agents contribute more.

### Privacy Boundary

Shared markers contain only strategy coordinates, valence, and intensity -- not
episode content, task details, or full PAD vectors. An agent queries the field and
learns "strategies in this region tend to go badly" without learning the specific
failures.

---

## C-Factor Integration

The C-Factor (Collective Intelligence Factor, Woolley et al. 2010) measures how
well the mesh performs beyond individual capabilities. Emotional contagion
contributes through:

1. **Collective vigilance**: one agent's alarm raises the entire mesh's vigilance
2. **Strategy diversification**: the somatic field helps agents avoid strategies
   peers have found unsuccessful, reducing redundant failure

---

## Observability

| Metric | Type | Description |
|---|---|---|
| `roko_daimon_contagion_received_total` | Counter | Contagion events received |
| `roko_daimon_contagion_arousal_cap_hits` | Counter | Times arousal cap reached |
| `roko_daimon_borrowed_affect_active` | Gauge | Active borrowed affect entries |

| Alert | Condition | Severity |
|---|---|---|
| Contagion saturation | Cap hit 3+ times in 24h | Info |
| Collective anxiety | >50% members in Struggling | Warning |
| Contagion isolation | No events received in 48h | Info |

---

## Academic Foundations

- Hatfield, E., Cacioppo, J.T., & Rapson, R.L. (1993). "Emotional contagion."
  *Current Directions in Psychological Science*, 2(3), 96-100.
- Van den Broek et al. (2023). "Emotional contagion in artificial agent networks."
- Woolley, A.W. et al. (2010). "Evidence for a Collective Intelligence Factor."
  *Science*, 330(6004), 686-688.
- Grasse, P.P. (1959). "La reconstruction du nid." *Insectes Sociaux*, 6(1), 41-80.

---

## Cross-References

- `pad-vector.md` -- PAD vector structure
- `somatic-markers-damasio.md` -- individual somatic landscape
- `integration-points.md` -- Daimon integration points
- `daimon-state-and-affect-engine.md` -- DaimonState with borrowed_affect field
