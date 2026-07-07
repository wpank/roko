# Roko Application — Attention as Currency

**Kind**: Perspective
**Source**: `docs/00-architecture/25-attention-as-currency.md`

---

## How the Lens Maps to Roko

The attention-as-currency perspective maps directly onto three operators and one cross-cut
in Roko's architecture. This page traces the mapping.

---

## The Scorer as Bid Generator

The [Scorer](../../../reference/05-operators/scorer.md) produces a 7-axis appraisal of each
Engram. In the attention economy framing:

**The Scorer generates the attention bid.**

Each Score axis is, in effect, a component of the total bid an Engram makes for processing
attention:

| Score Axis | Economic Role |
|-----------|---------------|
| Relevance | Base bid — how important is this to current goals? |
| Urgency | Time-adjusted bid — value decays if action is delayed |
| Novelty | Information value — how much new evidence does this carry? |
| Coherence | Bundle complementarity — how well does this fit the current context? |
| Affect valence | Risk premium — negative valence signals threat/cost |
| Confidence | Bid precision — how reliable is this bid? |
| Source trust | Credibility discount — should this bid be trusted? |

The composite score is not a single bid price but a **multi-dimensional bid** over attention
allocation dimensions. The allocation mechanism must aggregate these dimensions into an
ordering.

**Design implication**: The Scorer's 7-axis structure implicitly defines a multi-dimensional
attention market. Any aggregation of these axes into a scalar priority score is a choice
of **social welfare function** — a controversial choice in mechanism design. Different
aggregations produce different allocation outcomes. The current aggregation function should
be made explicit and justified.

---

## The Gate as Reserve Price

The [Gate](../../../reference/05-operators/gate.md) rejects Engrams that fall below a
threshold. In the attention economy framing:

**The Gate implements the reserve price.**

An Engram that cannot exceed the Gate threshold receives zero attention — it is not admitted
to the allocation market at all. The Gate's role is to prevent the allocation mechanism from
being overwhelmed by low-value bids, maintaining the signal-to-noise ratio for downstream
processing.

**Properties of Roko's reserve price:**
- The Gate threshold may be context-dependent (higher reserve price under high load).
- Different Gate implementations can impose different admission criteria (safety gates, quality
  gates, relevance gates).
- The reserve price must be calibrated carefully: too high, and important signals are excluded
  (attention poverty); too low, and the allocation mechanism is flooded (attention monopoly
  risk).

The Gate also implements **non-price rationing**: it can exclude categories of Engrams
regardless of their bid (safety-relevant exclusions, source-trust minimums). This
corresponds to regulatory floors in market design.

---

## The Router as Allocation Mechanism

The [Router](../../../reference/05-operators/router.md) determines which processing path
each admitted Engram follows. In the attention economy framing:

**The Router is the allocation mechanism — it decides who gets attention and how much.**

The Router maps Engrams to processing tiers:
- **T0** (fast path): minimal attention, stereotyped response
- **T1** (standard path): moderate attention, heuristic-guided response
- **T2** (deliberate path): high attention, full reasoning

This is a **tiered market**: attention is not homogeneous. "T0 attention" is a different
good from "T2 attention" — cheaper, less accurate, faster. The Router decides which tier's
attention each Engram merits.

**Efficiency criterion for the Router**: The efficient allocation routes each Engram to the
lowest-cost tier where the expected value of the response still exceeds the cost of a
higher-tier response. This is the cognitive equivalent of the economic principle: produce
at the point where marginal cost equals marginal benefit.

**Current implementation gap**: The Router currently uses rule-based tier assignment rather
than explicit value-cost tradeoff calculations. The attention economy framing suggests that
explicit cost modeling (how much does a T2 response cost?) and explicit value modeling
(how much better is a T2 response for this Engram class?) would improve allocation efficiency.

---

## The Composer as Bundle Allocator

The [Composer](../../../reference/05-operators/composer.md) assembles a synthesis context —
a bundle of Engrams to be processed together. In the attention economy framing:

**The Composer solves a combinatorial attention allocation problem.**

The context window is the total budget. The Composer must select a bundle of Engrams such
that:
1. The bundle fits in the context window (budget constraint).
2. The bundle maximizes the expected quality of the synthesis output (welfare criterion).
3. Synergistic Engrams (those that complement each other) are preferred to redundant ones.

This is the combinatorial auction problem described in [02-market-mechanics.md](02-market-mechanics.md).
VCG provides the theoretical ideal; the Composer's actual selection heuristic (e.g., greedy
by score) is a tractable approximation.

**Specific design implications:**
- Engrams that are **complements** in synthesis (each makes the other more valuable when
  present) should be scored as a bundle, not individually.
- Engrams that are **substitutes** (each covers the same ground) should be penalized for
  co-inclusion; the system should pick one rather than duplicating.
- The Composer should track **attention budget consumption** across the context window, not
  just token count — a high-information-density Engram may deserve more "space" even if it
  has the same token length as a low-density one.

---

## Policy as Macro-Economic Policy

The [Policy operator](../../../reference/05-operators/policy.md) sets top-level rules about
attention allocation. In the attention economy framing:

**Policy is macro-economic policy for the attention market.**

Just as central banks set interest rates and reserve requirements to shape overall economic
behavior, the Policy operator sets:
- **Priority domains**: certain categories of Engrams always receive priority attention
  (safety signals, user-facing events)
- **Attention caps**: no single topic or process can consume more than \( X\% \) of total
  attention
- **Throttling rules**: under high load, certain categories are delayed or dropped
- **Budget allocation**: how much of the total compute budget is allocated to T0 vs T1 vs T2

Policy interventions in the attention market are justified when market mechanisms would
produce socially suboptimal outcomes: when individual attention bids would, if followed,
produce outcomes that the system-level policy deems unacceptable.

---

## Attention and Daimon (Affect)

The [Daimon](../../../reference/09-cross-cuts/README.md) affect cross-cut modulates Score
weights based on the agent's current affective state. In the attention economy framing:

**Daimon implements attention biases — systematic departures from neutral bid evaluation.**

High arousal amplifies the attention value of urgent, novel stimuli. High valence focus
increases the attention value of goal-relevant stimuli. These biases are not bugs — they
are intentional mechanisms for ensuring that the attention market does not treat all bid
equally when the context demands differential treatment.

The economic analogy is a **preference shock**: the agent's preferences change based on
its state, causing the same Engrams to bid differently in different states. A crisis event
that raises arousal changes the preference ordering over all subsequent Engrams.
