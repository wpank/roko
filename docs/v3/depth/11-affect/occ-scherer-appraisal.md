# OCC and Scherer Appraisal Pipeline

> Depth file for [11-AFFECT.md](../../11-AFFECT.md) -- v1 source: `docs/v1/09-daimon/03-occ-scherer-appraisal.md`

---

## Overview

Appraisal theory holds that emotions are not random internal states but structured
evaluations of events relative to goals. The OCC model (Ortony, Clore, & Collins
1988) established that emotions arise from appraising events (desirable/undesirable
for goals), agents (praiseworthy/blameworthy), and objects (appealing/unappealing).
Scherer's Component Process Model (2001) refined this into a sequential checking
process with five checks: novelty, pleasantness, goal relevance, coping potential,
and norm compatibility.

The Daimon implements a hybrid 8-step appraisal pipeline that draws from both
theories: every `AffectEvent` is evaluated against the agent's current goals and
capabilities, producing a PAD delta that updates the mood state.

The critical design constraint is **grounding**: every emotion must have a trigger,
and every trigger must be grounded in a concrete metric. No emotion is generated
without an event that can be traced to a specific measurement.

---

## Theoretical Foundation

### OCC Model (Ortony, Clore, Collins 1988)

*The Cognitive Structure of Emotions* (Cambridge University Press) classifies
emotions based on what is being appraised:

| Appraisal Focus | Positive Valence | Negative Valence | Agent Mapping |
|---|---|---|---|
| **Events** (consequences for goals) | Joy, Hope, Relief | Distress, Fear, Disappointment | Gate results, task outcomes |
| **Agents** (actions relative to standards) | Pride, Admiration | Shame, Reproach | Self-evaluation via Dominance |
| **Objects** (attributes of things) | Liking, Attraction | Disliking, Aversion | Somatic landscape valence |

For the Daimon, the primary appraisal focus is **events** -- did the action produce
a good or bad outcome relative to task goals? Agent-focused appraisals emerge
indirectly through the Dominance dimension. Object-focused appraisals appear
through the somatic landscape.

### Scherer's Component Process Model (2001)

Scherer ("Appraisal considered as a process of multilevel sequential checking," in
Scherer, Schorr, & Johnstone, eds., *Appraisal Processes in Emotion*, Oxford
University Press) proposed sequential appraisal checks:

| Check | Question | Daimon Implementation |
|---|---|---|
| **Novelty** | Is this event new or expected? | Prediction accuracy |
| **Intrinsic pleasantness** | Inherently positive or negative? | Gate pass vs. fail |
| **Goal relevance** | Does this matter for my goals? | Always relevant (every event is task-context) |
| **Coping potential** | Can I handle this? | Dominance dimension |
| **Norm compatibility** | Does this align with standards? | Gate rung levels |

---

## The 8-Step Appraisal Pipeline

The appraisal pipeline is a deterministic function from events to PAD deltas:

```
AffectEvent arrives
  |
  v
Step 1: CLASSIFY -- identify event type
  |    (gate, task, blocker, time, queue, dream)
  |
  v
Step 2: GROUND -- verify the event is grounded in a concrete metric
  |    (gate has boolean pass/fail; task has boolean success/fail;
  |     time pressure has [0,1] proximity; blockers have a count)
  |
  v
Step 3: SCALE -- compute magnitude based on event parameters
  |    (rung_scale = 1.0 + min(rung, 3) x 0.15 for gate events;
  |     blocker_scale = max(1, min(5, n)) for blocked events)
  |
  v
Step 4: COMPUTE DELTA -- apply appraisal rules to produce PAD delta
  |    (pleasure, arousal, dominance, confidence adjustments)
  |
  v
Step 5: DECAY -- apply temporal decay to current mood before adding delta
  |    (factor = 0.5 ^ (elapsed_hours / half_life_hours))
  |
  v
Step 6: APPLY -- add delta to current mood with clamping to [-1, 1]
  |
  v
Step 7: PERSIST -- autosave updated state to disk
  |
  v
Step 8: EMIT -- if PAD change exceeds threshold (0.15 Euclidean),
         emit a MoodUpdate event for connected clients
```

Steps 1-3 are implemented in `AffectEngine::appraise()`. Steps 5-6 are handled by
`apply_delta()` and `AffectOctant::from_pad()`. Step 7 maps to the persistence
mechanism in `autosave()`. Step 8 is the MoodUpdate event emission.

---

## Appraisal Rule Set

### Gate Results

Gate evaluations are the most frequent appraisal trigger:

```rust
AffectEvent::GateResult { plan_id, task_id, passed, rung } => {
    let rung_scale = 1.0 + (rung.min(3) as f64 * 0.15);
    if passed {
        delta = (0.05 * rs, -0.01 * rs, 0.03 * rs, 0.03 * rs)
    } else {
        delta = (-0.10 * rs, 0.04 * rs, -0.08 * rs, -0.08 * rs)
    }
}
```

**Rung scaling**: rung-0 (compile only) has scale 1.0; rung-3 (compile + test +
clippy + diff review + symbol check + LLM judge) has scale 1.45. Higher-rung
outcomes are approximately 45% more emotionally significant.

**Asymmetry**: gate failures have 2x the pleasure impact of gate passes. This
follows prospect theory (Kahneman & Tversky 1979) -- losses loom larger than
gains.

### Task Outcomes

Task completion produces the largest emotional impact:

```rust
AffectEvent::TaskOutcome { task_id, succeeded } => {
    if succeeded { delta = (0.10, 0.00, 0.10, 0.08) }
    else         { delta = (-0.20, 0.00, -0.15, -0.15) }
}
```

Task outcomes do not affect arousal directly. Arousal tracks urgency and load,
driven by time pressure and blockers, not by success/failure.

### Blockers

Being blocked raises arousal and lowers dominance:

```rust
AffectEvent::Blocked { task_id, blocker_count } => {
    let n = blocker_count.max(1).min(5) as f64;
    delta = (0.0, n * 0.05, -(n * 0.08), -0.02 * n)
}
```

Blocker count is capped at 5 to prevent extreme states.

### Time Pressure

Deadline proximity is a pure arousal signal:

```rust
AffectEvent::TimePressure { task_id, deadline_proximity } => {
    let proximity = deadline_proximity.clamp(0.0, 1.0);
    delta = (0.0, proximity * 0.40, 0.0, 0.0)
}
```

At proximity = 1.0 (deadline imminent), arousal jumps by 0.40 -- triggering tier
routing changes (higher arousal -> lower T2 threshold -> stronger models).

### Queue Wait

Work waiting in a queue generates increasing arousal:

```rust
AffectEvent::QueueWait { task_id, wait_hours } => {
    let bump = if wait_hours <= 24.0 { 0.0 }
               else if wait_hours > 24.0 * 7.0 { 1.0 }
               else { ((wait_hours - 24.0) / 24.0 * 0.1).clamp(0.0, 1.0) };
    delta = (0.0, bump, 0.0, 0.0)
}
```

No arousal for work less than 24 hours old. After 24 hours, arousal ramps by 0.1
per day. After 7 days, arousal saturates at maximum.

### Dream Failure

Dream consolidation that finds repeated failures lowers confidence without
affecting the PAD vector:

```rust
AffectEvent::DreamFailure { task_type, failure_count } => {
    let failures = failure_count.max(1).min(5) as f64;
    let confidence_drop = -(0.07 * failures).min(0.35);
    delta = (0.0, 0.0, 0.0, confidence_drop)
}
```

Dream failures affect confidence, not pleasure or arousal, because dream
consolidation is a reflective process, not a reactive one.

---

## Emission Threshold

When the PAD state changes significantly (Euclidean delta > 0.15 from last emitted
state), the system emits a `MoodUpdate` event. The 0.15 threshold prevents event
flooding. A shift from Relaxed to Anxious (PAD distance ~1.2) always emits.
Micro-fluctuations within the same octant (distance ~0.05) do not.

---

## Future Appraisal Triggers

These triggers are specified but not yet fully wired:

| Trigger | Source | PAD Effect | Status |
|---|---|---|---|
| Prediction accuracy | CalibrationTracker | Accurate: +D; Inaccurate: -D, +A | Partial |
| Peer comparison | C-Factor metrics | Outperforming: +P, +D | Not implemented |
| Novel domain entry | Context assembly | -D (low confidence) | Not implemented |
| Repeated pattern success | Playbook match | +D (confidence boost) | Not implemented |
| Knowledge contradiction | AntiKnowledge | -D, +A (uncertainty spike) | Not implemented |

---

## Academic Foundations

- Ortony, A., Clore, G.L., & Collins, A. (1988). *The Cognitive Structure of
  Emotions*. Cambridge University Press.
- Scherer, K.R. (2001). "Appraisal considered as a process of multilevel sequential
  checking." In *Appraisal Processes in Emotion*. Oxford University Press.
- Kahneman, D. & Tversky, A. (1979). "Prospect Theory: An Analysis of Decision
  under Risk." *Econometrica*, 47(2), 263-291.
- Bechara, A. et al. (1994). "Insensitivity to future consequences following
  damage to human prefrontal cortex." *Cognition*, 50, 7-15.
- Shinn, N. et al. (2023). "Reflexion: Language Agents with Verbal Reinforcement
  Learning." *NeurIPS*.

---

## Cross-References

- `pad-vector.md` -- PAD vector structure and octant classification
- `alma-three-layer-temporal.md` -- how deltas interact with the three layers
- `somatic-markers-damasio.md` -- somatic marker check in the pipeline
- `six-behavioral-states.md` -- how accumulated mood maps to behavioral states
