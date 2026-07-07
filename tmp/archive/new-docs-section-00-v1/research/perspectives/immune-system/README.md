# Cognitive Immune System — Perspective

> If a cognitive system is analogous to an organism, it faces threats: corrupted
> knowledge, adversarial inputs, drifting goals, cascading errors. The immune system
> lens asks how the organism recognizes "self" from "non-self" and mounts a defense.

**Kind**: Perspective
**Source**: `docs/00-architecture/26-cognitive-immune-system.md`
**Related components**: [Gate](../../../reference/05-operators/gate.md),
[Scorer](../../../reference/05-operators/scorer.md),
[Provenance](../../../reference/10-types/provenance.md),
[Policy](../../../reference/05-operators/policy.md)

---

## The Arc of This Perspective

1. [`00-overview.md`](00-overview.md) — what this lens is and why it matters
2. [`01-innate-vs-adaptive.md`](01-innate-vs-adaptive.md) — the two immune subsystems and their analogs
3. [`02-recognition-and-response.md`](02-recognition-and-response.md) — pattern recognition, tolerance, memory
4. [`03-roko-application.md`](03-roko-application.md) — how this maps to gates, safety, anomaly detection
5. [`04-implications.md`](04-implications.md) — design decisions
6. [`05-open-questions.md`](05-open-questions.md)

---

## What This Lens Illuminates

The immune system lens makes visible the **self/non-self distinction** problem in cognitive
systems. A cognitive system that accepts all inputs uncritically is vulnerable to corruption:
garbage data, adversarial prompts, goal-corrupting feedback, cascading errors. A system that
rejects too much is brittle and isolated.

The immune system, as a biological solution to this problem, provides a rich vocabulary:
- **Innate immunity**: fast, non-specific, pre-configured defenses
- **Adaptive immunity**: slow, specific, learned defenses
- **Tolerance**: the ability to recognize and not attack "self"
- **Memory cells**: long-term immune records that enable faster future responses
- **Autoimmunity**: pathological attack on self
- **Immunodeficiency**: failure to mount adequate defenses

Each of these has a cognitive analog in Roko's architecture. Working through the analogy
surfaces design requirements that might not be visible from a purely engineering perspective.

---

## What This Lens Does Not Illuminate

The immune metaphor is weakest when:
- The threat model involves deliberate adversaries who adapt their attacks to bypass
  defenses (arms races). Biological immune systems face evolving pathogens, but the
  analogy grows strained when adversaries are sophisticated agents.
- The "self" boundary is unclear (multi-agent systems with shared knowledge stores).
- The system's goal is to *accept* diverse inputs from diverse sources (creativity, learning)
  rather than to *screen* them.

---

## See Also

- [`research/foundations/cybernetics.md`](../../foundations/cybernetics.md) — Good Regulator theorem as immunity
- [`research/perspectives/attention-as-currency/README.md`](../attention-as-currency/README.md) — the Gate as reserve price
