# Mood-Congruent Memory

> Depth file for [11-AFFECT.md](../../11-AFFECT.md) -- v1 source: `docs/v1/09-daimon/09-mood-congruent-memory.md`

---

## Overview

Emotional state biases what agents remember. This is not a bug -- it is an adaptive
mechanism grounded in Bower's (1981) associative network theory and validated by
Emotional RAG (2024, arXiv:2410.23041). An anxious agent should retrieve memories
of past dangers; a confident agent should retrieve memories of past successes.

The four-factor retrieval model integrates emotional congruence as a first-class
retrieval signal alongside recency, importance, and semantic relevance. This
document specifies how emotional tags are attached to Signals (knowledge entries),
how the scoring model computes retrieval priority, how PAD cosine similarity
captures emotional direction, and how the dream system interacts with emotional
memory.

---

## Theoretical Foundation

### Bower (1981)

Bower ("Mood and Memory," *American Psychologist*, 36(2), 129-148) proposed that
emotions function as nodes in an associative memory network. When an emotion is
activated, it spreads activation to connected memories. Mood-congruent recall
boosts accuracy by 5-30% for memories encoded under matching emotional states.

### Phelps (2004)

Phelps ("Human emotion and memory: interactions of the amygdala and hippocampal
complex," *Current Opinion in Neurobiology*, 14(2), 198-202) demonstrated that the
amygdala modulates hippocampal memory consolidation based on emotional arousal.
Emotionally arousing events are preferentially consolidated, producing stronger
and more durable memory traces. This neurobiological finding supports the Daimon's
arousal-based consolidation priority.

---

## Emotional Tags on Signals

Every Signal in the Neuro knowledge store carries an optional emotional tag:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmotionalTag {
    /// PAD vector at creation time.
    pub pad: PadVector,
    /// Plutchik label (e.g., "moderate_fear", "mild_joy").
    pub emotion: String,
    /// Emotional intensity at encoding time [0.0, 1.0].
    pub intensity: f32,
    /// Appraisal trigger description (e.g., "gate_fail:rung_2:task_abc").
    pub trigger: String,
    /// Mood snapshot at creation time.
    pub mood_snapshot: PadVector,
}
```

Signals created before the Daimon is enabled or during T0 ticks that skip appraisal
have `emotional_tag: None`. The emotional component defaults to a neutral factor
(0.5) in the retrieval score for untagged entries.

The storage schema includes affect provenance columns:

```
affect_pleasure     REAL    -- PAD pleasure at discovery
affect_arousal      REAL    -- PAD arousal at discovery
affect_dominance    REAL    -- PAD dominance at discovery
discovery_emotion   TEXT    -- Plutchik label
```

---

## Four-Factor Retrieval Scoring

Every knowledge retrieval scores candidates using four factors:

### Factor 1: Recency (Ebbinghaus 1885)

```
recency = exp(-t / half_life)
```

Recently accessed entries score higher. Half-life is type-dependent (Warnings: 7
days; Insights: 30 days; Facts: 365 days) and tier-multiplied (Transient: 0.1x;
Working: 0.5x; Consolidated: 1.0x; Persistent: 5.0x).

### Factor 2: Importance (Shinn et al. 2023)

```
quality = confidence x (validated / (validated + contradicted + 1))
```

Knowledge validated through operational use earns higher confidence. This implements
Reflexion's core insight: self-reflection on past performance improves future
decisions.

### Factor 3: Relevance (Standard RAG)

Cosine similarity between query embedding and entry embedding. The baseline
retrieval signal -- semantic closeness to the current task.

### Factor 4: Emotional Congruence (Bower 1981)

PAD cosine similarity between current emotional state and entry affect provenance:

```rust
pub fn score_entry(
    entry: &KnowledgeEntry,
    query_embedding: &[f32],
    current_pad: &PadVector,
    current_tick: u64,
    weights: &RetrievalWeights,
) -> f64 {
    let recency = (-((current_tick - entry.last_accessed_at) as f64)
        / weights.recency_half_life).exp();
    let importance = entry.quality_score();
    let relevance = cosine_similarity(query_embedding, &entry.embedding);
    let emotional_congruence = pad_cosine_similarity(current_pad, &entry.affect_pad());

    weights.recency * recency
        + weights.importance * importance
        + weights.relevance * relevance
        + weights.emotional * emotional_congruence
}
```

### Weight Distribution

Initial weights: recency 0.20, importance 0.25, relevance 0.35, emotional 0.20.

The 20% emotional weight is significant but not dominant. It biases retrieval
toward mood-congruent entries without overwhelming semantic relevance. An
emotionally congruent but semantically irrelevant entry will not surface.

Weights are learned over time -- the self-tuning system adjusts weights based on
which factor combinations correlate with positive task outcomes.

---

## PAD Cosine Similarity

The PAD cosine similarity function captures the *direction* of emotional state
rather than its *magnitude*:

```rust
pub fn pad_cosine_similarity(a: &PadVector, b: &PadVector) -> f64 {
    let dot = a.pleasure * b.pleasure + a.arousal * b.arousal + a.dominance * b.dominance;
    let mag_a = (a.pleasure.powi(2) + a.arousal.powi(2) + a.dominance.powi(2)).sqrt();
    let mag_b = (b.pleasure.powi(2) + b.arousal.powi(2) + b.dominance.powi(2)).sqrt();
    if mag_a == 0.0 || mag_b == 0.0 { return 0.5; }
    (dot / (mag_a * mag_b) + 1.0) / 2.0
}
```

Mild anxiety (P:-0.2, A:+0.1, D:-0.1) and strong anxiety (P:-0.8, A:+0.5, D:-0.4)
have cosine similarity 0.99. They are both anxious -- memories from either state
are relevant. Euclidean distance would give 0.74, incorrectly suggesting low
similarity.

---

## The Full Retrieval Pipeline

```
Phase 1: CANDIDATE GENERATION
    HNSW approximate nearest neighbors (3x overfetch)

Phase 2: FOUR-FACTOR RE-RANKING
    Score each candidate with recency x importance x relevance x emotional

Phase 3: CONTRARIAN INJECTION (if tracker says inject)
    Invert pleasure and dominance; add opposite-valence entries

Phase 4: RETRIEVAL STRENGTHENING (Testing Effect)
    Mark retrieved entries as accessed (Roediger & Karpicke 2006)

Phase 5: RETURN top-k results
```

### Cross-Emotional Retrieval

Explicit cross-emotional retrieval is available for specific situations:

| Situation | Query | Rationale |
|---|---|---|
| Anxious agent, approaching deadline | Retrieve confident memories | Coping: recall successful strategies |
| Overconfident agent, novel territory | Retrieve cautious memories | Humility: recall overconfidence failures |
| Stuck agent, no progress | Retrieve diverse contexts | Divergence: break patterns |

---

## Emotional Consolidation Bias

When the Daimon is enabled, dream consolidation applies an emotional salience boost:

```rust
pub fn consolidation_priority(episode: &Episode) -> f64 {
    let base = episode.importance * episode.novelty;
    let arousal_boost = episode.emotional_tag
        .as_ref()
        .map(|t| t.pad.arousal.abs() as f64 * 0.3)
        .unwrap_or(0.0);
    base * (1.0 + arousal_boost)
}
```

The 0.3 scaling ensures emotional salience is additive, not dominant. Among episodes
of comparable importance and novelty, emotional intensity breaks the tie.

---

## Emotional Provenance

When episodes consolidate into Insights, emotional provenance transfers:

```rust
pub struct EmotionalProvenance {
    pub average_pad: PadVector,
    pub discovery_emotion: String,
    pub validation_arc: Option<ValidationArc>,
    pub emotional_diversity: f64,  // Shannon entropy across supporting episodes
}

pub enum ValidationArc {
    Redemptive,     // adversity to positive outcome -- most transferable
    Contaminating,  // initial success to failure -- cautionary
    Stable,         // consistent tone -- reliable
    Progressive,    // gradual improvement -- successful learning
}
```

### Emotional Diversity as Quality Signal

An Insight validated across diverse emotional states is more reliable:

```rust
pub fn emotional_diversity(supporting_episodes: &[Episode]) -> f64 {
    // Normalized Shannon entropy of emotional labels
    // 1.0 = max diversity (every episode had different emotion)
    // 0.0 = no diversity (all same emotion)
}
```

---

## The Dream-Memory-Emotion Triangle

Dreams, memory, and emotion form a three-way interaction:

1. **Emotion -> Memory**: emotional state biases retrieval and consolidation
2. **Memory -> Emotion**: retrieved memories influence current emotional state
3. **Dreams -> Memory + Emotion**: consolidation reorganizes memory, depotentiation
   reduces arousal on charged memories, new somatic markers are created

This triangle is self-regulating when all three mechanisms operate. Without dreams,
the triangle degenerates: emotional memories accumulate without processing, and
contrarian retrieval becomes the only defense against rumination.

---

## Academic Foundations

- Bower, G.H. (1981). "Mood and Memory." *American Psychologist*, 36(2), 129-148.
- Phelps, E.A. (2004). "Human emotion and memory." *Current Opinion in
  Neurobiology*, 14(2), 198-202.
- Faul, L. & LaBar, K.S. (2022). "Mood-Congruent Memory Revisited."
  *Psychological Review*.
- Emotional RAG. (2024). arXiv:2410.23041.
- Ebbinghaus, H. (1885). *Memory: A Contribution to Experimental Psychology*.
- Shinn, N. et al. (2023). "Reflexion." *NeurIPS*.
- Roediger, H.L. & Karpicke, J.D. (2006). "Test-enhanced learning." *Psychological
  Science*, 17(3), 249-255.
- McGaugh, J.L. (2004). "The Amygdala Modulates Consolidation." *Annual Review of
  Neuroscience*, 27.

---

## Cross-References

- `pad-vector.md` -- PAD vector structure and cosine similarity
- `15-percent-contrarian-retrieval.md` -- contrarian injection details
- `somatic-markers-damasio.md` -- somatic landscape integration
- `alma-three-layer-temporal.md` -- how mood layer drives retrieval bias
