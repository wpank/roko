# Attention as Currency — Overview

**Kind**: Perspective
**Source**: `docs/00-architecture/25-attention-as-currency.md`

---

## The Central Claim

Cognitive attention is a finite, rivalrous resource. Multiple stimuli, tasks, and internal
processes compete for it simultaneously. Because it is finite and rivalrous — consuming it
for one purpose precludes using it for another — attention behaves economically: it can be
**allocated, priced, earned, wasted, and stolen**.

This perspective develops the consequences of taking that analogy seriously. It is not merely
a colorful description. When we treat attention as a currency:

- We can ask what the **exchange rate** between attention and other goods (task quality,
  response latency, safety coverage) is.
- We can ask whether the **allocation mechanism** (who gets attention and how much) is
  **efficient** in the economic sense — does it go to the uses with the highest marginal value?
- We can ask whether the mechanism is **strategy-proof** — can processes that compete for
  attention gain an advantage by misrepresenting their needs?
- We can identify **attention poverty** (important signals that cannot acquire enough
  attention to be acted upon) and **attention monopoly** (single processes that crowd out
  competitors).

---

## Why This Matters for Cognitive Architectures

Every cognitive architecture has an implicit attention allocation mechanism — the logic
that determines what gets processed when. In most architectures, this logic is implicit
in the scheduling, priority queues, and hard-coded rules that dispatch work. The attention-
as-currency lens makes this logic **explicit and analyzable**.

### The Invisible Hand Argument

Economists argue that markets, under certain conditions, produce efficient allocations
without central coordination. The "invisible hand" of prices coordinates individual
decisions into socially optimal outcomes. For cognitive architectures, the analogous claim
is: if processes bid for attention using prices that reflect their true value, the allocation
will be efficient without a central planner.

Current Roko uses a **central planner** model: the Router makes explicit decisions about
what to process, guided by Scorer outputs and Policy rules. The economic lens asks whether
a market mechanism would do better — or whether the conditions for market efficiency fail
in ways that favor the central planner.

### Kahneman's Two Systems Revisited

Kahneman's (2011) dual-process framework distinguishes System 1 (fast, automatic, cheap)
from System 2 (slow, deliberate, expensive). In economic terms:
- System 1 processing is **low-cost attention**: it consumes little of the finite resource.
- System 2 processing is **high-cost attention**: it consumes a significant share.

The question of when to invoke System 2 is therefore an economic question: when is the
marginal value of deliberate processing worth the cost? The T0/T1/T2 tier structure in
Roko ([reference/06-loop/](../../../reference/06-loop/README.md)) is an answer to this
question, but it is a rule-based rather than market-based answer.

---

## Historical Background

### William James and the Selective Nature of Attention

William James (1890) gave one of the first systematic accounts of attention as a selective
faculty: "Everyone knows what attention is. It is taking possession of the mind, in clear
and vivid form, of one out of what seem several simultaneously possible objects or trains
of thought." The key word is "one out of several simultaneously possible" — attention is
inherently selective, and selection implies resource constraints.

### Broadbent's Filter Theory

Broadbent (1958) formalized attention as a **filter**: of the many sensory channels
available, only one passes through the filter at a time. Subsequent work (Treisman, 1964;
Deutsch & Deutsch, 1963) complicated the filter model, but the core insight — that attention
is selective and resource-bounded — remained.

### Kahneman's Resource Model

Kahneman (1973) proposed that attention is a **general resource** (not just a filter on
channels) with a limited total capacity that can be allocated flexibly across tasks. Tasks
compete for this resource; performing two tasks simultaneously is possible up to the point
where their combined demand exceeds capacity. This framing is the closest to the currency
metaphor.

---

## The Scope of This Perspective

This folder does not argue that Roko should implement a literal market mechanism for
attention allocation. It argues that the economic framing:

1. Exposes implicit design choices in the current architecture that are otherwise invisible.
2. Provides a vocabulary for discussing tradeoffs (efficiency, fairness, incentive
   compatibility) that technical language alone does not provide.
3. Suggests specific design questions that might not arise from a purely engineering frame.

The goal is **illumination**, not prescription. The perspective folder ends with open
questions that the economic frame generates but does not answer.
