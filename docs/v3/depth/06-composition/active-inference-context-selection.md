# Active Inference for Context Selection

> **Depth file for [06-COMPOSITION.md](../../06-COMPOSITION.md)**
> Source: `crates/roko-compose/src/scorer.rs`, `crates/roko-compose/src/context_assembler.rs`
> v1 source: `docs/v1/03-composition/07-active-inference-context-selection.md`
> Primary citations: Friston (2006, 2010, 2022), Friston et al. (2015)

---

## Overview

Active inference provides a principled answer to "what should the scaffold
include?" by decomposing context value into pragmatic value (goal-seeking) and
epistemic value (information gain). An uncertain agent automatically explores
novel context; a confident agent automatically exploits proven context. No
separate exploration/exploitation tradeoff is needed -- the balance emerges
from the mathematics of expected free energy minimization.

---

## 1. The Free Energy Principle

Karl Friston (2006, 2010, 2022) established the free energy principle: all
self-organizing systems minimize variational free energy -- the gap between
their internal model and reality. Applied to agents: they act to bring their
model of the world into alignment with observations, while simultaneously
updating their model.

The key decomposition for context selection is **expected free energy (EFE)**:

```
G(section) = pragmatic_value(section) + epistemic_value(section) - ambiguity(section)
```

Where:

- **Pragmatic value:** "Will including this section help the agent succeed?"
  Measured by historical gate outcomes when this section was/was not included.
- **Epistemic value:** "Will including this section reduce the agent's
  uncertainty?" Measured by information gain -- how much does this section
  change the agent's beliefs about the task?
- **Ambiguity:** "How unclear is this section's contribution?" Measured by
  variance in outcomes when this section is included.

---

## 2. The EFE Formula

```
G(section) = pragmatic_value + epistemic_value - ambiguity

Where:
  pragmatic_value = E[task_success | section_included]
                  - E[task_success | section_excluded]

  epistemic_value = D_KL(P(state | section) || P(state))
                  = information gain from including section

  ambiguity       = Var[task_success | section_included]
```

The selection policy uses a softmax with inverse temperature gamma:

```
P(include section_i) = softmax(gamma * G(section_i))
                     = exp(gamma * G_i) / SUM_j exp(gamma * G_j)
```

With gamma = 8.0. Higher gamma makes selection more deterministic (greedy).
Lower gamma increases exploration.

---

## 3. Behavior Under Uncertainty

When the agent is **uncertain** (low track record, few observations):

- Epistemic value dominates the EFE score
- Agent prioritizes context that fills knowledge gaps -- architectural
  overviews, module interfaces, existing patterns
- Even tangentially related context may be selected if it resolves uncertainty

When the agent is **confident** (high track record, many successes):

- Pragmatic value dominates
- Agent grabs the highest-proven context -- relevant file content, specific
  type signatures, proven patterns
- Epistemic context is deprioritized

No hyperparameters control this balance. It emerges from the mathematics.

### Practical Example

Agent receives: "Implement HDC fingerprinting in roko-neuro."

Uncertainty assessment:
- HDC vectors: low uncertainty (50+ successful tasks)
- roko-neuro crate: HIGH uncertainty (new crate, no prior episodes)

Active inference result:
- 60% budget to roko-neuro architecture docs, interfaces, patterns (epistemic)
- 40% to HDC implementation patterns, fingerprinting algorithms (pragmatic)

Without active inference, the agent would grab the 50 highest-priority
HDC-related sections and miss the roko-neuro architecture that determines
where the code should live.

---

## 4. Scoring Mechanism

### Track Record Estimation

```rust
fn track_record(section_type: &str, task_category: &str) -> f64 {
    let pass_when_included = episodes
        .filter(|e| e.included_sections.contains(section_type))
        .filter(|e| e.task_category == task_category)
        .mean(|e| e.gate_passed as f64);

    let pass_when_excluded = episodes
        .filter(|e| !e.included_sections.contains(section_type))
        .filter(|e| e.task_category == task_category)
        .mean(|e| e.gate_passed as f64);

    pass_when_included - pass_when_excluded
}
```

### Belief Change (Bayesian Surprise)

From Itti and Baldi (NeurIPS 2005):

```
belief_change = D_KL(posterior || prior)
              = SUM_x posterior(x) * log(posterior(x) / prior(x))
```

In practice, approximated by HDC fingerprint novelty:

```rust
fn belief_change(section: &ContextChunk, agent_knowledge: &[ContextChunk]) -> f64 {
    let section_fp = text_fingerprint(&section.content);
    let max_similarity = agent_knowledge.iter()
        .map(|k| hamming_similarity(&section_fp, &text_fingerprint(&k.content)))
        .max_f64()
        .unwrap_or(0.0);
    1.0 - max_similarity  // High belief change = low similarity
}
```

### Uncertainty Estimation

```rust
fn uncertainty(task_category: &str, domain: &str) -> f64 {
    let episode_count = episodes
        .filter(|e| e.task_category == task_category && e.domain == domain)
        .count();
    let base = 1.0 / (1.0 + (episode_count as f64 / 10.0));
    let recent_accuracy = /* prediction accuracy for domain */;
    base + recent_accuracy.unwrap_or(0.5)
}
```

### Composite Score

```
score = track_record(entry) * belief_change(entry) / uncertainty
```

---

## 5. Current Scoring (Static Fallback)

The current `SectionScorer` in `crates/roko-compose/src/scorer.rs` uses static
priority-based scoring as the cold-start fallback:

```rust
confidence = priority_to_score(section.priority)  // 0.2 - 1.0
novelty = recency_decay(section.created_at)        // 1h fresh, 24h stale
utility = inverse_content_size(section.content)    // shorter = higher
reputation = trust_level(section.source)           // source trust
```

Active inference replaces these hand-tuned weights with learned ones after
sufficient calibration (~10 episodes per task category).

---

## 6. Affect Modulation

The active inference scorer is modulated by the Daimon's PAD state:

| PAD Dimension | Effect on EFE |
|--------------|--------------|
| High arousal (>= 0.35) | Increase pragmatic_value weight -> favor proven context |
| Low arousal (<= -0.35) | Increase epistemic_value weight -> favor novel context |
| Low pleasure (<= -0.35) | Increase weight on anti-knowledge and failure history |
| Low dominance (<= -0.35) | Favor explanatory context (agent seeks understanding) |
| High dominance (> 0.35) | Favor directive context (agent acts autonomously) |

This bridges the Daimon affect system and context selection. An anxious agent
automatically receives more cautionary context. A confident explorer
automatically receives more novel context.

---

## 7. Integration with 5-Stage Pipeline

Active inference scoring plugs into Stage 2 (Scoring):

```
Stage 1: Query  -> Candidate retrieval (HDC + keyword)
Stage 2: Score  -> Active inference EFE scoring   <- here
Stage 3: Dedup  -> Remove near-duplicates
Stage 4: Budget -> Fit to token budget
Stage 5: Format -> U-shaped placement
```

---

## 8. Connection to VCG Auction

Active inference and the VCG attention auction solve the same allocation
problem through different mechanisms:

| Aspect | Active Inference | VCG Auction |
|--------|-----------------|-------------|
| Setting | Single agent, centralized | Multi-subsystem, decentralized |
| Scoring | EFE: pragmatic + epistemic | Bid: expected_value x urgency x affect |
| Optimality | Maximizes expected free energy | Maximizes total welfare |
| Truthfulness | N/A (single scorer) | Guaranteed (VCG property) |

Both converge on the same allocation when bids are truthful and EFE estimates
are accurate.

---

## 9. Academic Foundations

- **Friston, K. (2006, 2010, 2022).** The Free Energy Principle. All
  self-organizing systems minimize variational free energy. Active inference
  extends this to agents: they minimize expected free energy, naturally
  balancing goal-seeking and information-seeking.

- **Friston, K. et al. (2015).** "Active Inference and Epistemic Value."
  Formal derivation of the EFE decomposition: G = pragmatic + epistemic.
  Applied to planning under uncertainty.

- **Itti, L. and Baldi, P. (2005).** "Bayesian Surprise Attracts Human
  Attention." NeurIPS. Surprise as KL divergence between posterior and prior.

- **Mehrabian, A. (1996).** PAD Model. Three-dimensional emotional space for
  affect modulation of the EFE scorer.

- **Sumers et al. (2023).** CoALA: Cognitive Architectures for Language Agents.
  Framework mapping active inference to agent context selection.

---

## 10. Implementation Status

| Aspect | Status |
|--------|--------|
| EFE formula specified | **Specified** |
| SectionScorer (static fallback) | **Shipped** (6 tests) |
| Active inference EFE scorer | **Designed, not wired** |
| Track record from episodes | **Designed** (episodes exist) |
| Belief change via HDC | **Designed** (HDC exists) |
| Softmax selection | **Designed** |
| PAD modulation of scoring | **Shipped** (affect_modifier in score_chunk) |
| Cold-start fallback to static | **Designed** |

---

## Cross-References

- [composer-trait.md](composer-trait.md) -- Scorer parameter in Compose trait
- [5-stage-assembly-pipeline.md](5-stage-assembly-pipeline.md) -- Stage 2
- [predictive-foraging-mvt.md](predictive-foraging-mvt.md) -- MVT stopping rule
- [vcg-attention-auction.md](vcg-attention-auction.md) -- Alternative mechanism
- [affect-modulated-retrieval.md](affect-modulated-retrieval.md) -- PAD integration
- `crates/roko-compose/src/scorer.rs` -- Static scorer
- `crates/roko-compose/src/context_assembler.rs` -- Scoring hook
