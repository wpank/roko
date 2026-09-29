# 15% Contrarian Retrieval

> Depth file for [11-AFFECT.md](../../11-AFFECT.md) -- v1 source: `docs/v1/09-daimon/07-15-percent-contrarian-retrieval.md`

---

## Overview

Mood-congruent memory is well-established in cognitive psychology: emotional states
bias which memories are retrieved. An anxious person retrieves anxious memories; a
confident person retrieves confident memories. For agents, this creates a dangerous
positive feedback loop: a failing agent retrieves memories of past failures, which
reinforces the negative emotional state, which biases retrieval further toward
failures. Left unchecked, this loop produces depressive rumination -- the
computational analog of learned helplessness (Seligman 1967).

The 15% contrarian retrieval mechanism breaks this loop by forcing a minimum
fraction of retrieved context to come from memories with opposite emotional valence.
An anxious agent always sees at least 15% of context from successful experiences.
An overconfident agent always sees at least 15% of context from failures. This is
implemented as a rolling window schedule across 200 ticks, not a fixed per-query
quota.

The mechanism is grounded in Bower's (1981) associative network theory and validated
by Emotional RAG (2024, arXiv:2410.23041).

---

## Theoretical Foundation: Bower (1981)

Bower ("Mood and Memory," *American Psychologist*, 36(2), 129-148) proposed that
emotions function as nodes in an associative memory network. When an emotion is
activated, it spreads activation to connected memories, concepts, and
interpretations. The key finding: mood-congruent recall boosts accuracy by 5-30%
for memories encoded under matching emotional states (Faul & LaBar 2022).

For natural organisms, this is mostly adaptive -- an anxious animal should recall
where predators lurk. But the mechanism has a failure mode: sustained negative mood
creates a retrieval bias that reinforces itself.

---

## The Agent Failure Mode

```
Step 1: Agent fails a gate check
        -> Appraisal: P: -0.10, A: +0.04, D: -0.08
        -> Emotional state shifts toward Anxious

Step 2: Agent retrieves context for next task
        -> Mood-congruent retrieval: anxious memories surface
        -> Context dominated by past failure cases

Step 3: Agent interprets situation through failure lens
        -> More cautious approach, lower confidence
        -> May over-hedge or fail to attempt viable strategies

Step 4: Cautious approach produces another gate failure
        -> State shifts further toward Anxious
        -> Return to Step 2 with stronger negative bias
```

Without intervention, this loop converges on a stable but maladaptive equilibrium.
The same problem occurs in the positive direction -- an overconfident agent
retrieves only success cases and may fail on edge cases.

---

## The Rolling Window Mechanism

### ContrarianTracker

```rust
pub struct ContrarianTracker {
    window: VecDeque<ContrarianEvent>,
    window_size: usize,              // default: 200
    min_contrarian_fraction: f64,    // default: 0.15
}

impl ContrarianTracker {
    pub fn should_inject(&self, current_tick: u64) -> bool {
        let window_start = current_tick.saturating_sub(self.window_size as u64);
        let recent: Vec<_> = self.window.iter()
            .filter(|e| e.tick >= window_start)
            .collect();

        if recent.len() < 10 {
            return current_tick % 7 == 0; // bootstrap: ~1 in 7
        }

        let contrarian_count = recent.iter().filter(|e| e.was_contrarian).count();
        let rate = contrarian_count as f64 / recent.len() as f64;
        rate < self.min_contrarian_fraction
    }
}
```

### Contrarian Retrieval Implementation

When injection is needed, the retrieval system inverts the pleasure and dominance
dimensions of the current PAD vector:

```rust
fn retrieve_contrarian(
    store: &dyn KnowledgeStore,
    query_embedding: &[f32],
    current_pad: &PadVector,
    limit: usize,
) -> Vec<ScoredEntry> {
    let inverted_pad = PadVector {
        pleasure:  -current_pad.pleasure,
        arousal:    current_pad.arousal,   // keep arousal (salience)
        dominance: -current_pad.dominance,
    };
    store.query_with_affect(query_embedding, &inverted_pad, limit)
}
```

**Why keep arousal**: Arousal tracks salience -- contrarian entries should still be
relevant to the current urgency level. An anxious agent retrieving contrarian
context should get important positive memories (high arousal, high pleasure), not
trivial ones (low arousal, high pleasure).

### Blending

The retrieval pipeline blends 85% congruent with 15% contrarian:

```
Phase 1: Candidate generation (3x overfetch)
Phase 2: Four-factor re-ranking (recency x importance x relevance x emotional)
Phase 3: Contrarian injection (if tracker says inject)
Phase 4: Testing effect (mark as accessed)
Phase 5: Return top-k
```

---

## Why 15%?

The 15% minimum is calibrated to be:

1. **Large enough to break feedback loops**: At 5%, the contrarian signal would be
   too weak. At 15%, the contrarian context creates productive tension.

2. **Small enough to preserve mood-congruent benefits**: Mood-congruent retrieval
   is adaptive in the common case. An agent facing a familiar failure type should
   retrieve past failure cases for relevant warnings and solutions.

3. **Empirically grounded**: Bower's (1981) findings show 5-30% accuracy boost
   from mood-congruent retrieval. The 15% contrarian rate preserves the core
   benefit (85% congruent) while creating a floor on diversity.

### Why a Rolling Window, Not Per-Query?

A per-query quota would be wasteful when the agent's mood is neutral. At neutral
mood, retrieval is already balanced. The rolling window only forces injection when
the cumulative rate drops below 15%, which happens primarily during sustained
emotional states.

---

## Three Complementary Loop-Breaking Mechanisms

| Mechanism | Level | Timescale | How It Works |
|---|---|---|---|
| **15% contrarian** | Memory access | Tick-level | Forces opposite-valence context |
| **REM depotentiation** | Memory storage | Hours | Reduces arousal of charged memories |
| **PAD decay** | Affect state | Hours to days | Pulls mood toward neutral baseline |

All three are necessary. Without contrarian retrieval, the loop forms immediately.
Without depotentiation, the loop's fuel accumulates. Without decay, baseline mood
drifts to an extreme.

---

## Somatic Landscape Contrarian

The 15% mechanism also applies to somatic marker queries. When querying the k-d
tree, 15% of returned markers are selected from the opposite-valence region:

```rust
fn query_contrarian(
    &self, coords: &[f64; 8], congruent_valence: f64, k: usize,
) -> ContrarianResult {
    let all = self.tree.nearest(coords, k * 5, &squared_euclidean);
    let contrarian: Vec<_> = all.iter()
        .filter(|(_, m)| is_contrarian(m.valence, congruent_valence))
        .take(k).collect();
    // ...
}
```

The somatic landscape always presents a mixed signal: "This region generally feels
positive, BUT there are cases where similar strategies failed."

---

## Edge Cases

### Insufficient Opposite-Valence Entries

Young agents may have few opposite-valence entries. Fallback retrieves high-arousal
(salient) entries regardless of valence direction -- these inject diversity because
high-arousal entries come from unusual or significant events.

### Near-Neutral Congruent Valence

When congruent valence is near 0.0, a dead zone prevents inappropriate filtering:

```rust
fn is_contrarian(marker_valence: f64, congruent_valence: f64) -> bool {
    const DEAD_ZONE: f64 = 0.05;
    if congruent_valence.abs() < DEAD_ZONE {
        return marker_valence.abs() > 0.20;
    }
    marker_valence.signum() != congruent_valence.signum()
}
```

### Configuration

```toml
[daimon.contrarian]
window_size = 200
min_contrarian_fraction = 0.15
somatic_blend_weight = 0.15
min_valence_delta = 0.10
contrarian_alpha = 0.5
```

### Persistence

The tracker serializes with `DaimonState`. On `--resume`, the ring buffer is
restored. On crash recovery, bootstrap behavior (inject 1 in 7) handles the
empty window gracefully. Snapshot size: ~3.6 KB (at most 400 events at 9 bytes
each).

---

## Mind Wandering

Approximately every 200 ticks, the system retrieves a random high-arousal episode
and re-appraises it in the current context. This serves as spontaneous contrarian
injection -- the randomly retrieved episode is unlikely to match the current
emotional state. Cost is zero (local database query, deterministic appraisal rules,
no LLM call).

---

## Academic Foundations

- Bower, G.H. (1981). "Mood and Memory." *American Psychologist*, 36(2), 129-148.
- Blaney, P.H. (1986). "Affect and Memory: A Review." *Psychological Bulletin*,
  99(2), 229-246.
- Faul, L. & LaBar, K.S. (2022). "Mood-Congruent Memory Revisited."
  *Psychological Review*.
- Emotional RAG. (2024). "Emotional RAG: Enhancing Role-Playing Agents through
  Emotional Retrieval." arXiv:2410.23041.
- Seligman, M.E.P. (1967). "Failure to escape traumatic shock." *Journal of
  Experimental Psychology*, 74(1), 1-9.
- Walker, M.P. & van der Helm, E. (2009). "Overnight therapy?" *Psychological
  Bulletin*, 135(5), 731-748.

---

## Cross-References

- `somatic-markers-damasio.md` -- somatic landscape query protocol
- `mood-congruent-memory.md` -- full four-factor retrieval model
- `alma-three-layer-temporal.md` -- PAD decay as loop-breaking mechanism
- `pad-vector.md` -- PAD cosine similarity for contrarian detection
