# Lost in the Middle: U-Shaped Attention Optimization

> **Depth file for [06-COMPOSITION.md](../../06-COMPOSITION.md)**
> Source: `crates/roko-compose/src/prompt.rs` -- Placement enum
> v1 source: `docs/v1/03-composition/06-lost-in-the-middle-u-shape.md`
> Primary citation: Liu et al. (2023), "Lost in the Middle: How Language Models
> Use Long Contexts," TACL 2024, arXiv:2307.03172

---

## Overview

Language models attend to information at the beginning and end of their context
far more effectively than information in the middle. This U-shaped attention
curve, documented by Liu, Lin, Hewitt, Paranjape, Bevilacqua, Petroni, and
Liang (2023), directly constrains scaffold design: critical sections must be
placed at prompt boundaries, not buried in the middle. Roko implements this
through the Placement enum (Start/Middle/End) and the PromptComposer's U-shape
ordering phase.

---

## 1. The Phenomenon

```
Performance
    |
    | ####                                        ####
    | #####                                     ######
    | ######                                  ########
    | ########                              ##########
    | ##########                          ############
    | ############                      ##############
    | ##############                  ################
    | ################            ####################
    | ####################################################
    +-------------------------------------------------------->
      Beginning      Middle positions        End         Position
```

- **Beginning (primacy):** Models attend most strongly to the first tokens.
  Information placed at the start is used effectively.
- **End (recency):** Models attend second-most strongly to the last tokens.
  Information placed at the end is used well.
- **Middle (degradation):** Information in the middle of long contexts is
  largely ignored. Performance degrades over 30% when relevant information is
  positioned mid-context.

This is a **positional problem**, not a capacity problem. The same information
the model ignores in position 10 of 20 might be used correctly in position 1
or position 20.

---

## 2. Empirical Evidence

### Liu et al. (2023) -- The Original Finding

Tested on multi-document QA and key-value retrieval across GPT-3.5-turbo,
Claude (v1), and MPT-30B-Instruct:
- 20 documents retrieved, answer at varying positions
- Performance highest at position 1 (beginning) or position 20 (end)
- Performance lowest at positions 8-14 (middle)
- Degradation occurs even in models designed for long contexts

### Context Rot (Chroma 2025)

Tested 18 frontier models. All exhibit U-shaped attention. Semantically close
distractors (documents that look relevant but contain wrong information) are
far more harmful than obviously irrelevant documents.

### Du et al. (EMNLP 2025) -- Even Whitespace Hurts

Whitespace and formatting overhead degrades performance by 13.9-85%. Middle
zone degradation reflects information dilution -- more tokens between relevant
content means more opportunities for the model to lose the thread.

### Shi et al. (ICML 2023) -- Irrelevant Context Actively Harms

Irrelevant context does not merely dilute performance -- it **actively harms**
it. Models perform worse with irrelevant context than with no context at all.

---

## 3. Why It Is Architectural, Not Learned

A 2025 paper (arXiv:2603.10123) proved the U-shaped bias is an **algebraic
property** of causal decoder architectures, present at initialization before
any training or positional encoding:

- **Causal masking guarantees primacy bias.** Early tokens lie on
  exponentially more computational paths through the residual network.
- **Residual connections guarantee recency bias.** Late tokens maintain direct
  short-path connections to the output through the residual stream.

This means the bias **cannot be trained away.** Positional encodings (RoPE,
ALiBi) modulate the curve shape but cannot eliminate it. Any scaffold that
places critical information in the middle is fighting the architecture.

---

## 4. Roko's Implementation

### 4.1 The Placement Enum

```rust
pub enum Placement {
    Start,   // Highest attention zone (primacy)
    Middle,  // Lowest attention zone
    End,     // Second-highest attention zone (recency)
}
```

### 4.2 Section-to-Placement Mapping

| Section | Placement | Rationale |
|---------|-----------|-----------|
| Role identity | **Start** | Identity first |
| Conventions | **Start** | Safety rules need primacy |
| Task description | **Start** | Core task at the beginning |
| Workspace map | **Middle** | Supporting, not critical path |
| PRD extract | **Middle** | Reference material |
| Cross-plan context | **Middle** | Background information |
| Research memo | **Middle** | Supporting evidence |
| Gate errors | **End** | Recent failure needs recency |
| Anti-patterns | **End** | Prohibitions need recency |
| Affect guidance | **End** | Behavioral modulation near output |
| Constraints reminder | **End** | Dual-position safety pattern |

### 4.3 U-Shape Ordering in PromptComposer

After budget fitting, the PromptComposer reorders included sections:

```
final_order = [
    sections.filter(placement == Start),    // Highest attention
    sections.filter(placement == Middle),   // Lowest attention
    sections.filter(placement == End),      // Second-highest attention
]
```

Within each placement group, CacheLayer ordering is preserved for cache
stability.

---

## 5. Interaction with Cache Alignment

U-shape placement and cache alignment are partially in tension:
- **Cache alignment** wants stable content first (System -> Session -> Task)
- **U-shape** wants high-value content at beginning and end

Resolution: within each cache tier, sections are placed according to their
Placement hint:

```
Cache Layer 0 (System) -- all Start placement
    Role identity, Conventions, Safety

Cache Layer 1 (Session) -- Middle placement
    Workspace map, Cross-plan context

Cache Layer 2 (Task) -- Start + Middle placement
    Task description (Start), PRD extract (Middle)

Cache Layer 3 (Dynamic) -- End placement
    Gate errors, Anti-patterns, Affect guidance
```

This achieves both goals: stable cached prefix AND highest-attention positions
contain the most critical information.

---

## 6. Position-Aware Scoring

### Attention Multiplier Model

```rust
pub struct PositionAttentionModel {
    pub primacy_weight: f64,   // default: 0.35
    pub primacy_decay: f64,    // default: 0.15
    pub recency_weight: f64,   // default: 0.30
    pub recency_decay: f64,    // default: 0.20
    pub baseline: f64,         // default: 0.35
}

impl PositionAttentionModel {
    pub fn attention_at(&self, normalized_pos: f64) -> f64 {
        let primacy = self.primacy_weight
            * (-self.primacy_decay * normalized_pos).exp();
        let recency = self.recency_weight
            * (-self.recency_decay * (1.0 - normalized_pos)).exp();
        (primacy + recency + self.baseline).min(1.0)
    }
}
```

### Placement-Adjusted Scoring

```rust
pub fn placement_adjusted_score(base_score: f64, placement: Placement) -> f64 {
    match placement {
        Placement::Start  => base_score * 1.0,   // primacy: full value
        Placement::End    => base_score * 0.95,   // recency: ~95% value
        Placement::Middle => base_score * 0.70,   // degradation: ~70% value
    }
}
```

A Medium-priority section at Start is effectively scored higher than a
High-priority section in Middle.

---

## 7. Design Implications

1. **Never bury critical information in the middle.** Placing a critical
   section in Middle is equivalent to reducing its effective priority by 30%+.

2. **Constraints at both edges.** Following Devin's dual-position pattern,
   safety constraints appear at both beginning (Layer 2) and end (Layer 7).

3. **Error context at the end.** Gate errors always go at End. The model's
   last impression before generating is "these are the mistakes to avoid."

4. **Supporting context in the middle.** Low-criticality content (workspace
   maps, cross-plan context) occupies the Middle. Acceptable because partial
   attention loss does not seriously affect task success.

---

## 8. Academic Foundations

- **Liu et al. (2023)**, TACL 2024, arXiv:2307.03172. The foundational paper.
- **"Lost in the Middle at Birth"**, arXiv:2603.10123, 2025. Algebraic proof.
- **"Found in the Middle"**, He et al., ACL Findings 2024, arXiv:2406.16008.
  Calibration without retraining. Up to 15pp improvement.
- **LLMLingua / LongLLMLingua**, Jiang et al., ACL 2024, arXiv:2310.06839.
  Semantic density ranking. Up to 21.4% improvement at 4x compression.
- **Shi et al.** (ICML 2023). Irrelevant context actively harms performance.
- **Du et al.** (EMNLP 2025). Even whitespace degrades 13.9-85%.
- **Chroma** (2025), "Context Rot". All 18 frontier models degrade.
- **Serial Position Effects**, arXiv:2406.15981, 2024. Primacy, recency, and
  middle loss variation across model size and task type.

---

## 9. Implementation Status

| Aspect | Status |
|--------|--------|
| Placement enum (Start/Middle/End) | **Shipped** |
| Section-to-Placement mapping | **Shipped** |
| U-shape ordering in PromptComposer | **Shipped** |
| Constraints at both edges | **Shipped** |
| Position attention model | **Designed** |
| Dynamic placement from density scoring | **Designed** |
| Per-model attention curves | **Not yet** |

---

## Cross-References

- [prompt-composer.md](prompt-composer.md) -- Assembly algorithm
- [system-prompt-builder-9-layer.md](system-prompt-builder-9-layer.md) -- Layer ordering
- [token-budget-management.md](token-budget-management.md) -- Budget constraints
- [active-inference-context-selection.md](active-inference-context-selection.md) -- Scoring
- `crates/roko-compose/src/prompt.rs` -- Placement enum
