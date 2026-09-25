# Somatic Landscape Integration

> **v3 depth -- 09-memory** | Source: v1/06-neuro/13

The Somatic Landscape integrates Damasio's somatic marker hypothesis into
Neuro's retrieval pipeline -- a k-d tree over an 8-dimensional strategy space
that provides fast emotional heuristics for knowledge selection, with
mandatory 15% contrarian retrieval to prevent confirmation bias.

---

## The SomaticLandscape

```rust
pub struct SomaticLandscape {
    tree: KdTree<f64, SomaticMarker, 8>,
}

pub struct SomaticMarker {
    pub strategy_coords: [f64; 8],
    pub valence: f64,       // -1 to +1
    pub intensity: f64,     // 0 to 1
    pub episodes: Vec<ContentHash>,
}
```

### How it works

1. **Before acting**: Map current situation to 8D strategy coordinates
2. **Query landscape**: Find nearest neighbors in k-d tree (< 1ms)
3. **Aggregate valence**: Weighted mean of nearby markers
4. **Route**: Negative valence -> stronger model (System 2); positive ->
   cheaper model (System 1)
5. **After acting**: Record outcome as new somatic marker

### 8-Dimensional Strategy Space (Coding Domain)

| Dim | Name | Low (0.0) | High (1.0) |
|-----|------|-----------|-----------|
| 1 | Complexity | Simple function change | Multi-file refactor |
| 2 | Risk | No tests could break | Core infrastructure |
| 3 | Novelty | Familiar pattern | Completely new territory |
| 4 | Confidence | Uncertain | High confidence |
| 5 | Time pressure | No deadline | Urgent, budget exhausted |
| 6 | Scope | Single file | Cross-crate |
| 7 | Reversibility | Easily reverted | Hard to undo |
| 8 | Dependency depth | Leaf code | Core trait, many implementors |

---

## Mood-Congruent Retrieval (Bower 1981)

```
retrieval_weight = base_weight * (1 + 0.15 * mood_congruence)
```

`mood_congruence` is the dot product between entry's emotional valence and
Daimon's current PAD vector. 0.15 coefficient means mood contributes at most
15% to retrieval weight.

### Mandatory 15% contrarian retrieval

```
For each retrieval batch (top 20 entries):
    85% by standard retrieval score
    15% (at least 3 entries) from the OPPOSITE valence
```

Prevents two failure modes:
1. **Panic lock-in**: Negative mood -> only negative knowledge -> spiral
2. **Overconfidence**: Positive mood -> only positive knowledge -> blind to
   risks

---

## PAD Vector Integration

| PAD Dimension | Low Value Effect | High Value Effect |
|---------------|-----------------|------------------|
| **Pleasure** | Bias toward Warnings/AntiKnowledge | Bias toward Heuristics |
| **Arousal** | Retrieve broadly (exploration) | Retrieve narrowly (focus) |
| **Dominance** | Bias toward exploratory entries | Bias toward execution entries |

### Arousal encoding (Yerkes-Dodson)

```
effective_limit = base_limit * (1 + 0.5 * (1 - |arousal - 0.5| * 2))
// Moderate arousal (0.5): broadest
// Extreme arousal (0 or 1): narrowest
```

### Emotional decay

- **Emotional half-life**: 3 days
- **Knowledge half-life**: Type-dependent (1 hour -- 90 days)

Emotions fade faster than knowledge. Each Dreams cycle reduces emotional
intensity by 0.3--0.5 on the valence scale (Walker & van der Helm 2009
SFSR model).

---

## Academic Foundations

- Damasio, A. R. (1994). *Descartes' Error*. Putnam.
- Bower, G. H. (1981). "Mood and Memory." *American Psychologist*, 36(2).
- Yerkes, R. M. & Dodson, J. D. (1908). *Journal of Comparative Neurology*.
- Walker, M. P. & van der Helm, E. (2009). *Psychological Bulletin*, 135(5).
- Mehrabian, A. (1996). "Pleasure-Arousal-Dominance." *Current Psychology*.

---

## Cross-References

- `six-knowledge-types.md` -- types interact with emotional retrieval
- `ebbinghaus-decay-with-tier.md` -- knowledge decay (distinct from emotional)
- `knowledge-query-api.md` -- retrieval API integrating somatic markers
