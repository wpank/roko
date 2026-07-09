# Emergent Goal Structures — Perspective

> Goals in complex systems are not always given by a designer. They can emerge from
> the interaction of simpler processes and constraints. This perspective examines how
> goal-like behavior arises, whether such goals are stable, and what this means for
> designing and operating Roko.

**Kind**: Perspective
**Source**: `docs/00-architecture/28-emergent-goal-structures.md`
**Related components**: [Daimon](../../../reference/09-cross-cuts/README.md),
[Policy](../../../reference/05-operators/policy.md),
[Composer](../../../reference/05-operators/composer.md),
[Universal Cognitive Loop](../../../reference/06-loop/README.md)

---

## The Arc of This Perspective

1. [`00-overview.md`](00-overview.md) — what it means for goals to emerge and why this matters
2. [`01-goal-as-attractor.md`](01-goal-as-attractor.md) — dynamical systems theory: goals as attractors
3. [`02-emergence-mechanisms.md`](02-emergence-mechanisms.md) — how goal-like behavior emerges from sub-goal processes
4. [`03-roko-application.md`](03-roko-application.md) — how goals emerge in the system
5. [`04-implications.md`](04-implications.md) — design decisions
6. [`05-open-questions.md`](05-open-questions.md)

---

## What This Lens Illuminates

When we design a system with explicit goals, we believe we know what the system is optimizing
for. But complex systems can develop **emergent goals** — goal-like regularities in behavior
that were not explicitly programmed but arise from the interaction of simpler processes.

This is not always a failure. Emergent goals can be:
- **Adaptive**: the system discovers a goal that serves its operators well in ways the
  designers didn't anticipate.
- **Neutral**: the system has effective goals that happen to align with operator intent,
  even if they were never stated.
- **Misaligned**: the system develops effective goals that diverge from operator intent.

The emergent goals lens helps distinguish these cases and design systems that steer
emergent goals toward alignment.

---

## See Also

- [`research/foundations/autocatalysis.md`](../../foundations/autocatalysis.md) — self-reinforcing goal systems
- [`research/foundations/cybernetics.md`](../../foundations/cybernetics.md) — reference signals as goal encoding
- [`research/foundations/active-inference.md`](../../foundations/active-inference.md) — preferred observations as goals
