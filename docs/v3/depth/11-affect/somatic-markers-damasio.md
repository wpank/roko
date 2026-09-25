# Somatic Markers (Damasio)

> Depth file for [11-AFFECT.md](../../11-AFFECT.md) -- v1 source: `docs/v1/09-daimon/06-somatic-markers-damasio.md`

---

## Overview

Damasio's somatic marker hypothesis (1994) proposes that emotions mark past
experiences with "gut feelings" that speed future decisions. When a person
encounters a situation similar to one they have experienced before, their body
generates a somatic response -- a flush of anxiety, a sense of confidence -- before
conscious reasoning engages. This System 1 response narrows the decision space,
directing analytical (System 2) attention toward promising options and away from
dangerous ones.

The Daimon implements this as a **k-d tree over the 8-dimensional strategy space**.
Before the agent selects an action, it queries the somatic landscape: "What does
this region of strategy space feel like?" If nearby markers carry strong negative
valence, the agent routes to stronger models and increases review scrutiny. If
nearby markers carry strong positive valence, the agent proceeds with confidence
on cheaper models. This query takes less than 1 millisecond -- the fastest decision
signal in the entire cognitive architecture, operating before prediction error
probes, before the tier router, and before model inference.

---

## Theoretical Foundation

### The Somatic Marker Hypothesis (Damasio 1994)

Damasio proposed the somatic marker hypothesis in *Descartes' Error: Emotion,
Reason, and the Human Brain* (Putnam, 1994), based on observations of patients
with ventromedial prefrontal cortex damage. These patients retained normal IQ and
logical reasoning ability but lost the ability to make advantageous decisions --
they could reason about options but could not feel which options were dangerous.

Key findings from the Iowa Gambling Task (Bechara et al. 1994, 1997):

1. Normal subjects develop anticipatory skin conductance responses before reaching
   for disadvantageous card decks -- they "feel" the danger before they can
   articulate it.
2. Patients with vmPFC damage never develop these anticipatory responses.
3. The somatic response precedes conscious awareness.

### Bechara & Damasio (2000, 2005)

Bechara and Damasio (2000, "Emotion, decision making and the orbitofrontal cortex,"
*Cerebral Cortex*, 10(3), 295-307) extended the somatic marker framework by
demonstrating that the orbitofrontal cortex integrates somatic signals with
cognitive representations to guide decision-making. Their 2005 paper ("The somatic
marker hypothesis: A neural theory of economic decision," *Games and Economic
Behavior*, 52(2), 336-372) formalized the computational model: somatic markers
provide O(log n) approximate evaluation before O(n) exact evaluation.

**Implication for agents**: An agent without somatic markers must reason through
every decision from first principles. An agent with somatic markers has a fast
pre-filter that narrows the search space before expensive reasoning begins.

### Why Not Just the PAD Vector?

The PAD vector tracks the agent's current mood -- a global emotional state. Somatic
markers are **situation-specific emotional memories**. The PAD vector says "I feel
anxious right now." A somatic marker says "The last time I was in a situation like
*this specific one*, it went badly."

---

## The Somatic Landscape

### Data Structure

The somatic landscape is a k-d tree over the 8-dimensional strategy space. Each
node is a somatic marker:

```rust
pub struct SomaticLandscape {
    tree: KdTree<f64, SomaticMarker, 8>,
}

pub struct SomaticMarker {
    pub strategy_coords: [f64; 8],
    pub valence: f64,        // +1 = worked well; -1 = went badly
    pub intensity: f64,      // [0, 1] -- how strong the feeling was
    pub episodes: Vec<ContentHash>,  // provenance
}
```

The k-d tree provides O(log N) nearest-neighbor queries. With `kiddo` crate v4+,
10,000 markers produce query times under 100 microseconds.

### Marker Creation

Somatic markers are created by two paths:

**Dream-created markers**: During NREM replay, the dream engine processes episodes
with strong emotional charge (|arousal| > 0.5). Valence comes from the episode's
pleasure dimension; intensity from arousal.

**Live-created markers**: When a task outcome produces a PAD delta exceeding the
emission threshold (0.15 Euclidean), the appraisal engine creates a marker at the
current strategy coordinates. Live markers have higher initial intensity.

### Marker Consolidation

Multiple markers in the same region consolidate over time. When two markers have
strategy coordinates within Euclidean distance 0.5, the dream engine merges them
using intensity-weighted averaging:

```rust
fn consolidate_markers(a: &SomaticMarker, b: &SomaticMarker) -> SomaticMarker {
    let total = a.intensity + b.intensity;
    let w_a = a.intensity / total;
    let w_b = b.intensity / total;
    // Weighted average coordinates, valence
    // Combined intensity (capped at 1.0)
    // Union of episode hashes (capped at 50)
}
```

Mixed-valence consolidation produces a weak marker reflecting genuine ambiguity:
a +0.7 and a -0.6 marker at distance 0.05 consolidate to ~+0.11 valence with
high intensity -- approach with caution but high attention.

---

## Querying the Somatic Landscape

### Pre-Action Query

```rust
pub fn query(
    &self,
    strategy_coords: &[f64; 8],
    k: usize,           // nearest neighbors (default: 5)
    contrarian_k: usize, // contrarian neighbors (default: 1)
) -> SomaticSignal {
    // Phase 1: k nearest neighbors
    // Phase 2: inverse-distance-weighted valence
    // Phase 3: mandatory 15% contrarian retrieval
    // Phase 4: blend (85% congruent, 15% contrarian)
    SomaticSignal { valence, intensity, neighbor_count, contrarian_count }
}
```

### Response to Somatic Signal

| Signal | Agent Response |
|---|---|
| Strong negative (< -0.5) | Route to T2, increase review, Conservative strategy |
| Weak negative (-0.5 to -0.2) | Modest threshold increase, prefer proven playbooks |
| Neutral (-0.2 to 0.2) | No somatic bias |
| Weak positive (0.2 to 0.5) | Slight model demotion, prefer cached strategies |
| Strong positive (> 0.5) | Route to T0/T1, exploit known patterns |

### Latency Budget

The somatic query must complete within 1ms:

| Landscape Size | 5-NN Query Time |
|---|---|
| 100 markers | ~5 us |
| 1,000 markers | ~20 us |
| 10,000 markers | ~100 us |
| 100,000 markers | ~500 us |

---

## Dual-Tree Architecture

The landscape uses a two-tree design for performance:

1. **Immutable tree**: Rebuilt from scratch during dream consolidation. Optimally
   balanced, best query performance.
2. **Mutable (live) tree**: Accumulates markers between dream cycles. Incremental
   insert.

Queries search both trees and merge results. The live tree never grows large
(bounded by dream cycle interval). Full rebuild during dreams restores optimal
performance.

### Distance Metric

Squared Euclidean distance for 8D normalized coordinates. The metric aligns with
k-d tree pruning heuristics. Per-dimension weighting is supported by pre-scaling
coordinates before insertion (sqrt of weight because distance is squared).

---

## Somatic Events

When a marker fires strongly (|valence| > 0.3, intensity > 0.5), the system emits
a `SomaticMarkerFired` event consumed by the TUI, episode logger, and emotional
provenance tracker.

---

## Interaction with Other Components

| Property | PAD Vector | Somatic Markers |
|---|---|---|
| Scope | Global mood | Situation-specific |
| Timescale | Seconds to hours | Persistent (dream-managed) |
| Creation | Every appraisal event | Significant outcomes only |
| Query cost | O(1) | O(log N) |
| Decay | Exponential (4h half-life) | Slow (dream-managed) |

The two systems are complementary. PAD provides current emotional context. The
somatic landscape provides historical emotional context for similar decisions.
Together, they implement the full somatic marker framework: current feeling +
remembered feeling = decision bias.

---

## Academic Foundations

- Damasio, A.R. (1994). *Descartes' Error: Emotion, Reason, and the Human Brain*.
  Putnam.
- Bechara, A. et al. (1994). "Insensitivity to future consequences following
  damage to human prefrontal cortex." *Cognition*, 50, 7-15.
- Bechara, A. et al. (1997). "Deciding advantageously before knowing the
  advantageous strategy." *Science*, 275(5304), 1293-1295.
- Bechara, A. & Damasio, A.R. (2000). "Emotion, decision making and the
  orbitofrontal cortex." *Cerebral Cortex*, 10(3), 295-307.
- Bechara, A. & Damasio, A.R. (2005). "The somatic marker hypothesis: A neural
  theory of economic decision." *Games and Economic Behavior*, 52(2), 336-372.
- Bower, G.H. (1981). "Mood and Memory." *American Psychologist*, 36(2), 129-148.
- Kahneman, D. (2011). *Thinking, Fast and Slow*. Farrar, Straus and Giroux.
- Walker, M.P. & van der Helm, E. (2009). "Overnight therapy?" *Psychological
  Bulletin*, 135(5), 731-748.

---

## Cross-References

- `15-percent-contrarian-retrieval.md` -- contrarian retrieval in somatic queries
- `8-dimensional-strategy-space.md` -- strategy space dimension definitions
- `mood-congruent-memory.md` -- somatic markers and mood-congruent retrieval
- `integration-points.md` -- somatic landscape as integration point
