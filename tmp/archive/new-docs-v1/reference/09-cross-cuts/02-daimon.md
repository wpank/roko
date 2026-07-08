# Daimon — Affect and Motivation Cross-Cut

> Daimon maintains the agent's emotional state (PAD vectors), translates it into
> behavioral influences, and marks Engrams with affective significance.

**Status**: Built
**Crate**: `roko-daimon` (L2)
**Depends on**: [Engram](../01-engram/README.md), [Score](../10-types/score.md),
[Pulse](../02-pulse/README.md)
**Used by**: [SCORE stage](../06-loop/02-stage-score.md),
[ROUTE stage](../06-loop/03-stage-route.md), [COMPOSE stage](../06-loop/04-stage-compose.md)
**Last reviewed**: 2026-04-19

> **Full implementation**: [`subsystems/daimon/`](../../subsystems/daimon/README.md)
> (populated in a later refactor cluster). This page covers Daimon's role in the loop.

---

## TL;DR

Daimon is the agent's affective engine. It models the agent's emotional state as a
PAD (Pleasure-Arousal-Dominance) vector and translates that state into concrete
influences on three loop stages: SCORE (valence axis), ROUTE (urgency-adjusted
confidence thresholds), and COMPOSE (emphasis modulation). Without Daimon, the agent
is emotionally flat — it weights all situations identically regardless of their
affective significance.

---

## The Idea

The name "Daimon" comes from the Greek concept of an inner guiding spirit — the
motivational force that drives action. In Roko, Daimon is the subsystem that
determines how the agent's current motivational state influences cognition.

Human cognition is not emotionally neutral. Fear elevates attention to threats.
Curiosity drives exploration. Urgency narrows focus to high-priority items. These
effects are not "biases" to be corrected — they are adaptive mechanisms.

Roko's Daimon implements a simplified model of these effects via the PAD model
(Mehrabian, 1980):

- **Pleasure** (P): positive/negative valence of the current state
- **Arousal** (A): activation level; high = alert; low = calm
- **Dominance** (D): feeling of control; high = confident; low = anxious

These three dimensions capture a surprisingly wide range of emotional states and have
solid empirical grounding from affective psychology.

---

## PAD Model

```rust
// source: crates/roko-daimon/src/pad.rs
pub struct PadVector {
    pub pleasure:   f32,   // -1.0 (displeasure) to +1.0 (pleasure)
    pub arousal:    f32,   // -1.0 (calm) to +1.0 (aroused/excited)
    pub dominance:  f32,   // -1.0 (submissive) to +1.0 (dominant)
}

impl PadVector {
    pub fn neutral() -> Self {
        PadVector { pleasure: 0.0, arousal: 0.0, dominance: 0.0 }
    }

    pub fn urgency_level(&self) -> f32 {
        // High arousal + moderate dominance = urgent but in control
        (self.arousal * 0.7 + (1.0 - self.dominance.abs()) * 0.3).clamp(0.0, 1.0)
    }
}
```

The PAD vector is updated by:
- Incoming Pulses with emotional tags (e.g., `urgency.high`, `error.critical`)
- Accumulated tick outcomes (repeated failures decrease dominance)
- Scheduled decay toward neutral (ALMA three-layer decay model)

---

## Behavioral States

Daimon translates the PAD vector into a discrete behavioral state label. These labels
are the user-facing abstraction over the continuous PAD space:

| Label | PAD signature | Loop effect |
|---|---|---|
| `Focused` | high P, moderate A, high D | narrow retrieval (lower candidate cap) |
| `Exploratory` | moderate P, moderate A, moderate D | wider retrieval, higher novelty weight |
| `Urgent` | low P, high A, moderate D | lower routing confidence threshold (faster escalation) |
| `Anxious` | low P, high A, low D | defer more often; lower cost threshold |
| `Fatigued` | low P, low A, low D | longer tick period; reduced context window |
| `Calm` | neutral | default parameters |

---

## Loop Participation

| Stage | How Daimon participates |
|---|---|
| SCORE | Provides `valence` axis value per Engram; adjusts `novelty` weight per behavioral state |
| ROUTE | Adjusts routing confidence thresholds based on `urgency_level` |
| COMPOSE | Adds behavioral state note to system prompt; adjusts token budget in urgency |
| PERSIST | Writes `affect_charge` to new Outcome Engrams; updates PAD based on verify result |

---

## Somatic Markers

Inspired by Damasio's Somatic Marker Hypothesis, Daimon can mark Engrams with an
`affect_charge` — a scalar encoding the emotional significance of that Engram at the
time it was created. These marks persist in the Engram and influence future SCORE
evaluations via the Valence axis.

Engrams created during high-urgency states receive a strong positive or negative
affect_charge. In future ticks, these Engrams surface with higher relevance (via the
Valence weight in SCORE), creating a learned emotional saliency map.

---

## Configuration

```toml
[daimon]
enabled               = false     # Built but not default-enabled
pad_decay_rate        = 0.05      # decay per tick toward neutral
urgency_threshold     = 0.70      # urgency_level > this → adjust routing
somatic_markers       = true      # write affect_charge to Engrams
```

---

## See also

- [`subsystems/daimon/`](../../subsystems/daimon/README.md) — full Daimon implementation
- [SCORE stage](../06-loop/02-stage-score.md) — Valence axis
- [ROUTE stage](../06-loop/03-stage-route.md) — urgency adjustment
- [Injection Model](04-injection-model.md) — how Daimon is injected into TickContext
- [research/perspectives/](../../research/perspectives/) — PAD model literature and extensions
