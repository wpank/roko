# Three Cognitive Speeds — Overview

> A single loop running at three different tempos: fast-reactive for habits, slow-
> reflective for deliberation, and deep-offline for consolidation.

**Status**: Shipping
**Crate**: `roko-agent`
**Depends on**: [Cognitive Loop](../06-loop/README.md),
[Dual-Process](../06-loop/10-dual-process.md)
**Used by**: [Speed Coordination](04-speed-coordination.md), all agents
**Last reviewed**: 2026-04-19

---

## TL;DR

Roko agents run their cognitive loop at three distinct speeds, named after EEG
oscillation bands: **Gamma** (~10 s period, reactive), **Theta** (~75 s period,
reflective), and **Delta** (hours, consolidation). The speeds are not separate
systems — they are the same `loop_tick()` function running with different parameters.
What varies is context size, model selection, and the types of stages that execute.

---

## The Motivation

### Biological grounding

Neuroscience has long observed that the brain operates on multiple timescales
simultaneously. Gamma oscillations (30–80 Hz) correlate with local feature binding and
fast sensorimotor integration. Theta oscillations (4–8 Hz) correlate with spatial
navigation, working memory, and cross-regional coordination. Delta waves (0.5–4 Hz)
dominate deep sleep, during which the hippocampus replays and consolidates memories.

These are not merely metaphors in Roko — the speed tiers capture a real computational
truth: **not all cognition needs to happen at the same rate, and mixing rates is
inefficient.**

A system that uses its most expensive model for every stimulus will be slow and
costly. A system that uses only its cheapest model will fail on hard tasks. The
three-speed design threads this needle: cheap and fast for the routine, expensive and
thorough for the novel, and offline (free) for reorganization.

### Computational grounding

The three speeds correspond to three different cost-quality tradeoffs:

| Speed | Cost per tick | Quality ceiling | Use case |
|---|---|---|---|
| Gamma | < $0.001 | Good for known tasks | Routine queries, monitoring, execution |
| Theta | $0.01–$0.10 | Excellent for most tasks | Novel questions, multi-step reasoning |
| Delta | ~$0 (no model) | Memory reorganization only | Consolidation, pruning, index rebuild |

Running everything at Gamma keeps costs low but produces poor results on hard tasks.
Running everything at Theta gives good results but $0.10 per tick adds up at scale.
The multi-speed design achieves near-Theta quality at near-Gamma cost by routing the
majority of ticks to Gamma.

---

## The Three Speeds at a Glance

### Gamma (Reactive) — ~10 s period

Named after the fastest EEG band. Gamma ticks handle stimuli where the agent has high
routing confidence — situations it has "seen before" and knows how to handle. The loop
uses a small context window (4 096 tokens), a fast cheap model, and looks back only
60 s in the substrate.

Gamma is the steady-state operating speed. A healthy agent in a familiar environment
runs almost entirely at Gamma. Cost per tick is sub-cent.

### Theta (Reflective) — ~75 s period

Named after the theta band associated with working memory and deliberate reasoning.
Theta ticks handle stimuli with lower routing confidence, or stimuli where a prior
Gamma tick produced a VERIFY failure. The loop assembles a richer context (16 384
tokens), calls a more capable model, and looks back 10 minutes in the substrate.

Theta is the upgrade path from Gamma. It activates automatically when confidence drops
below 0.85. Cost per tick is one to two orders of magnitude higher than Gamma, but
the quality improvement is significant.

### Delta (Consolidation) — hours

Named after the slow-wave sleep band. Delta ticks are not real-time — they run on a
schedule (typically every few hours) or are triggered by high accumulated free energy.
Delta ticks execute only the QUERY, SCORE, and PERSIST stages. They do not call models
or produce external outputs. Instead, they reorganize the substrate: promoting Engrams
to higher-durability tiers, pruning stale knowledge, rebuilding HDC indexes, and
updating routing priors.

Delta is the agent's "sleep." It is when the agent integrates what it has learned into
a more efficient structure for future Gamma and Theta processing.

---

## How Speeds Are Selected

Speed selection happens in the ROUTE stage of `loop_tick()`:

```
routing confidence ≥ 0.85 → Gamma
routing confidence 0.60–0.85 → Theta
routing confidence < 0.60 → Defer / Delta
scheduled interval elapsed → Delta
free_energy rolling avg > threshold → Delta (emergency consolidation)
```

Speed selection is never permanent — the agent is always running at the speed
appropriate for the current stimulus. An agent that spends the morning answering
routine questions (Gamma) will shift to Theta when it receives a complex novel
question, then return to Gamma afterward.

---

## Interaction with the Full System

The three speeds interact with:

- **Active Inference** — prediction errors accumulate per tick; when the rolling
  average exceeds a threshold, a Delta consolidation is triggered.
- **Dual-Process** — the routing confidence threshold is the exact decision point
  between Gamma and Theta.
- **Cross-Cuts** — Neuro participates in all speeds (knowledge indexing). Daimon
  affects Gamma and Theta (emotional salience). Dreams runs exclusively at Delta.
- **Budget Controller** — each speed tier has a different cost cap. The daily budget
  is typically allocated as: 80% Gamma, 18% Theta, 2% Delta overhead.

---

## See also

- [Gamma details](01-gamma-reactive.md)
- [Theta details](02-theta-reflective.md)
- [Delta details](03-delta-consolidation.md)
- [How speeds coordinate](04-speed-coordination.md)
- [Dual-Process](../06-loop/10-dual-process.md) — the routing confidence decision
