# Temporal Knowledge Topology — Perspective

> Knowledge is not just a set of facts. It has a shape — a topology — defined by the
> relationships among facts, the times at which they became known, and the ways in which
> they change over time. This perspective develops the mathematics of knowledge topology
> and applies it to Roko's decay, consolidation, and retrieval architecture.

**Kind**: Perspective
**Source**: `docs/00-architecture/27-temporal-knowledge-topology.md`
**Related components**: [Decay variants](../../../reference/10-types/decay.md),
[Decay tier matrix](../../../reference/10-types/decay.md),
[Neuro cross-cut](../../../reference/09-cross-cuts/README.md),
[Engram](../../../reference/01-engram/README.md)

---

## The Arc of This Perspective

1. [`00-overview.md`](00-overview.md) — what knowledge topology means and why it matters
2. [`01-knowledge-as-topology.md`](01-knowledge-as-topology.md) — topological spaces, manifolds, connectivity
3. [`02-temporal-shape.md`](02-temporal-shape.md) — how knowledge evolves as a space over time
4. [`03-decay-as-topological-operator.md`](03-decay-as-topological-operator.md) — decay reshapes the topology
5. [`04-roko-application.md`](04-roko-application.md) — mapping to Engram stores, Neuro, decay models
6. [`05-implications.md`](05-implications.md) — design decisions
7. [`06-open-questions.md`](06-open-questions.md)

---

## What This Lens Illuminates

Knowledge has **structure beyond content**. Two agents that hold the same set of facts
may have very different knowledge topologies:
- One agent's facts form a dense, well-connected graph; the other's are isolated points.
- One agent's knowledge is concentrated in recent time periods; the other's is distributed.
- One agent has smooth gradients of confidence; the other has sharp boundaries.

These structural differences produce different reasoning behaviors, different failure modes,
and different responses to new information. Topology makes these differences precise and
computable.

The temporal dimension adds a further layer: the knowledge space changes over time. Facts
are added, confirmed, contradicted, and forgotten. The topology evolves. Understanding the
shape of this evolution is essential for designing systems that degrade gracefully rather
than catastrophically.

---

## What This Lens Does Not Illuminate

Topology describes **shape and connectivity** — it is silent about **content**. Two
knowledge graphs with different facts but the same topological structure are equivalent
under this lens. Content differences are irrelevant to the topological analysis.

This is a feature (it separates structural questions from content questions) and a
limitation (topological analysis cannot determine whether specific facts are true).

---

## See Also

- [`reference/10-types/decay.md`](../../../reference/10-types/decay.md) — the four decay models
- [`research/perspectives/energy-model/README.md`](../energy-model/README.md) — decay as energy dissipation
- [`research/foundations/active-inference.md`](../../foundations/active-inference.md) — temporal depth in generative models
