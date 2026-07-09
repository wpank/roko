# Attention as Currency — Perspective

> If cognitive attention is a finite, rivalrous resource, and agents compete for it,
> then the economic metaphors of markets, prices, and allocation mechanisms apply.
> This folder develops that lens and asks what it implies for Roko's design.

**Kind**: Perspective
**Source**: `docs/00-architecture/25-attention-as-currency.md`
**Related components**: [Composer](../../../reference/05-operators/composer.md),
[Scorer](../../../reference/05-operators/scorer.md),
[Router](../../../reference/05-operators/router.md),
[Policy](../../../reference/05-operators/policy.md)

---

## The Arc of This Perspective

1. [`00-overview.md`](00-overview.md) — what this lens is and why it matters
2. [`01-the-metaphor.md`](01-the-metaphor.md) — attention as a finite, rivalrous resource
3. [`02-market-mechanics.md`](02-market-mechanics.md) — auction theory and VCG mechanisms
4. [`03-roko-application.md`](03-roko-application.md) — how this maps to Roko's Composer, Scorer, Router
5. [`04-implications.md`](04-implications.md) — design decisions that follow from the metaphor
6. [`05-open-questions.md`](05-open-questions.md)

---

## What This Lens Illuminates

The attention-as-currency lens makes visible the **allocation problem** that any complex
cognitive system must solve: among all the stimuli, tasks, and considerations competing for
processing resources, how does the system decide what to attend to?

Without this lens, allocation looks like a pure technical question — scheduling, prioritization,
resource management. The economic lens reveals that it is also a problem of **mechanism design**:
who gets to bid for attention, what they bid with, and how the "price" of attention is set
all shape what the system reliably does and does not notice. This has direct consequences
for safety, performance, and the kinds of failure modes the system exhibits.

---

## What This Lens Does Not Illuminate

The attention-as-currency metaphor is strongest when describing competition for a **shared,
finite resource** (compute cycles, working memory, context window). It is weaker when:

- Attention is not genuinely rivalrous (parallel execution paths may not compete)
- The "price" of attention has no natural unit (what is the exchange rate between
  Score axes?)
- Strategic behavior by "bidders" is not present (processes do not strategically misrepresent
  their attention needs in current Roko)

---

## See Also

- [`research/foundations/active-inference.md`](../../foundations/active-inference.md) — attention as precision weighting
- [`research/perspectives/energy-model/README.md`](../energy-model/README.md) — cognitive energy (closely related)
