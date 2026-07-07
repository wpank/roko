# Cognitive Energy Model — Perspective

> If cognitive processing is analogous to physical work, then it has an energy budget.
> This perspective develops the thermodynamic and biological metaphors — ATP, mitochondria,
> metabolic load — and asks what they imply for compute allocation, Router costs, and
> Policy throttling in Roko.

**Kind**: Perspective
**Source**: `docs/00-architecture/29-cognitive-energy-model.md`
**Related components**: [Router](../../../reference/05-operators/router.md),
[Policy](../../../reference/05-operators/policy.md),
[Three cognitive speeds](../../../reference/07-speeds/README.md),
[Universal Cognitive Loop](../../../reference/06-loop/README.md)

---

## The Arc of This Perspective

1. [`00-overview.md`](00-overview.md) — what the energy metaphor means and why it matters
2. [`01-cognitive-energy.md`](01-cognitive-energy.md) — the budget, ATP metaphor, mitochondria analogy
3. [`02-allocation-dynamics.md`](02-allocation-dynamics.md) — how energy is allocated, conserved, and depleted
4. [`03-roko-application.md`](03-roko-application.md) — mapping to compute allocation, Router costs, Policy throttling
5. [`04-implications.md`](04-implications.md) — design decisions
6. [`05-open-questions.md`](05-open-questions.md)

---

## What This Lens Illuminates

The energy model lens makes visible the **total cost of processing** in ways that compute
metrics (CPU time, memory, latency) partially capture but do not fully represent.

Physical energy models have been refined over centuries of thermodynamics and biology.
They provide:
- A vocabulary for describing **energy states** (high/low energy), **transitions** (work),
  and **dissipation** (heat, entropy).
- The concept of **efficiency**: how much useful work per unit energy input?
- The concept of **limits**: Carnot efficiency, Landauer's principle (information erasure
  costs energy), metabolic constraints on sustained performance.
- The concept of **recovery**: energy depleted through work must be replenished.

These concepts translate naturally to cognitive processing and provide a framework for
thinking about compute allocation that complements pure performance metrics.

---

## What This Lens Does Not Illuminate

The energy metaphor is weakest when:
- The "energy" cost of computation is not well-defined (what is the energy unit?).
- Cognitive "work" does not have a clear analog to physical work.
- The metaphor is taken too literally (brains actually run on ATP, but Roko runs on
  electricity — the analogy is conceptual, not physical).

---

## See Also

- [`research/perspectives/attention-as-currency/README.md`](../attention-as-currency/README.md) — closely related (attention = allocation)
- [`reference/07-speeds/README.md`](../../../reference/07-speeds/README.md) — the three speeds as energy tiers
