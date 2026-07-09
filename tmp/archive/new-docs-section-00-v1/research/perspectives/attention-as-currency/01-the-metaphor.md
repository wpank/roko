# Attention as a Finite Resource

**Kind**: Perspective
**Source**: `docs/00-architecture/25-attention-as-currency.md`

---

## The Resource Framing

Economists distinguish **rival** goods from **non-rival** goods. A rival good is one where
consumption by one agent reduces availability for others — physical goods like food and land
are rival. A non-rival good can be consumed by multiple agents without depletion — information,
once created, can be copied indefinitely.

**Attention is rival.** An agent's attention directed at task A cannot simultaneously be
directed at task B with full intensity. The attention consumed by processing one Engram is
not available to process another. This rivalry is not a software limitation to be engineered
away — it reflects the fundamental structure of sequential processing in a finite compute
environment.

Attention is also **excludable**: an agent can, in principle, refuse to attend to a stimulus.
The combination of rivalry and excludability makes attention a **private good** in economic
terminology — the most basic type of good for which market mechanisms apply.

---

## The Budget Constraint

An agent has a **total attention budget** \( A \) per unit time. This budget is determined
by:
- **Compute capacity**: clock cycles, memory bandwidth, parallelism
- **Context window**: for LLM-based reasoning, the effective working memory
- **Latency constraints**: tasks that must complete within deadlines

The budget is not fixed in general — it can be increased (more compute) or decreased
(energy-saving mode) — but at any given moment, it is finite and the agent must decide how
to allocate it.

A **budget constraint** forces choices. Without scarcity there is no allocation problem
and no need for attention mechanisms. It is precisely because the budget is binding that
the design of the allocation mechanism matters.

---

## Attention vs. Related Concepts

### Attention vs. Priority

**Priority** is an ordering: task A is more important than task B, therefore A is done first.
Priority does not require quantifying importance; it only requires ranking.

**Attention as currency** requires quantification: how much attention does task A merit,
and at what exchange rate does that translate into processing time, context budget, or
compute allocation? The distinction matters when tasks do not form a total order — when A
is more important than B in one dimension and less important in another.

### Attention vs. Focus

**Focus** is attention concentrated on a single object. High focus is the degenerate case
of attention allocation where one object receives all available attention. The currency
metaphor generalizes focus to the case where attention is distributed across multiple
objects simultaneously, each receiving a partial allocation.

### Attention vs. Relevance

**Relevance** is a property of a stimulus relative to a goal: a stimulus is relevant if
attending to it advances a goal. Relevance can be used as a bid in an attention market —
stimuli bid for attention with their relevance score. But relevance is not attention:
a highly relevant stimulus may receive zero attention if it cannot "afford" to bid
successfully against higher-stakes competitors.

The [Scorer](../../../reference/05-operators/scorer.md) in Roko produces a 7-axis appraisal
of an Engram, several axes of which (salience, relevance, urgency) are, in economic terms,
the bid that an Engram makes for processing attention.

---

## What "Spending" Attention Means

Spending attention on an Engram means:
1. **Context allocation**: including the Engram's content in the working context of a
   subsequent computation.
2. **Processing cycles**: routing it through the [Composer](../../../reference/05-operators/composer.md)
   for synthesis or the [Policy](../../../reference/05-operators/policy.md) for action.
3. **Memory consolidation**: storing the Engram in Neuro (durable knowledge) via the
   Dreams consolidation path.

Each of these has a cost. Context allocation reduces the space available for other Engrams.
Processing cycles are finite. Memory consolidation requires the Dreams subsystem to be
running and commits storage.

"Spending attention wisely" means allocating these resources to the Engrams where the
marginal value of processing is highest relative to its cost.

---

## Attention Poverty and Attention Monopoly

### Attention Poverty

An important, time-sensitive signal that cannot acquire sufficient attention score to be
acted upon is in **attention poverty**. It exists in the system but is never processed.
Sources of attention poverty:
- Low salience on relevant axes (the Scorer underestimates importance)
- Gate rejection despite high relevance (the Gate blocks based on non-relevance criteria)
- Router queuing depth exceeds deadlines (backpressure kills time-sensitive signals)

Attention poverty is one of the most dangerous failure modes for a safety-critical system:
the signal that the system is about to fail cannot acquire enough attention to trigger a
corrective response.

### Attention Monopoly

A process or signal that consistently acquires a disproportionate share of total attention
creates an **attention monopoly**. This crowds out other signals, including potentially
important ones. Sources of attention monopoly:
- A single high-salience topic (crisis event) that dominates context for extended periods
- A looping process that generates high-score Engrams continuously, crowding out
  background processes
- A Composer task that fills the context window, blocking other inputs

The [Policy operator](../../../reference/05-operators/policy.md) and the T0/T1/T2 tier
structure are partial defenses against attention monopoly: Policy can cap the attention
any single thread receives; tier routing ensures that fast T0 responses do not block T1/T2
paths.
