# Temporal Knowledge Topology — Overview

**Kind**: Perspective
**Source**: `docs/00-architecture/27-temporal-knowledge-topology.md`

---

## The Problem

A knowledge base is usually thought of as a **set of propositions** — a flat list of facts,
each either believed or not. This representation is convenient but impoverished: it misses
the *structure* of knowledge.

Consider two agents:
- Agent A knows that Paris is the capital of France, and also knows that France is in Europe,
  and that the EU is headquartered in Brussels, and that Brussels is in Belgium, and that
  Belgium borders France.
- Agent B knows that Paris is the capital of France.

Propositionally, both agents know the same *first* fact. But Agent A's knowledge is
*connected* — each fact is linked to others in a dense graph. Agent B's knowledge is an
isolated point.

When Agent A learns that the EU headquarters might move, they can immediately reason about
consequences for multiple connected facts. Agent B cannot. The **topology** of the knowledge
base — its shape, connectivity, density — determines the agent's reasoning capacity
independently of any individual fact.

---

## What Topology Studies

**Topology** is the branch of mathematics that studies properties of spaces preserved under
continuous deformation — properties that survive stretching and bending but not tearing or
gluing. A topologist distinguishes a sphere from a torus (donut), but does not distinguish
a circle from an ellipse.

The topological properties of a space include:
- **Connectedness**: is the space in one piece, or multiple disconnected components?
- **Compactness**: does the space have "holes" that you can't contract?
- **Dimension**: how many independent directions can you move in?
- **Boundary**: where does the space end?

For knowledge graphs, the analogous topological properties include:
- **Connectivity**: are facts linked to each other, or isolated?
- **Clustering**: are there dense sub-networks (concept clusters)?
- **Diameter**: what is the maximum shortest path between any two facts? (Small diameter
  means any two facts are "close" in reasoning distance.)
- **Bottlenecks**: are there narrow connections that, if severed, would disconnect large
  parts of the knowledge space?

---

## Adding Time

Standard topology studies static spaces. A temporal knowledge topology studies spaces
that **evolve over time**:
- New facts are added (nodes and edges added → local connectivity increases)
- Facts expire or are contradicted (nodes and edges removed → local connectivity decreases)
- Confidence changes (edge weights change → effective connectivity shifts)
- Facts are consolidated (separate sub-graphs are linked by new bridging facts)

The temporal evolution of the knowledge topology is governed by:
1. The **ingestion process**: what new Engrams enter, at what rate, from what sources.
2. The **decay process**: how older Engrams lose confidence and eventually expire.
3. The **consolidation process**: how Dreams distills and links existing knowledge.
4. The **contradiction process**: how contradicting evidence reshapes the confidence landscape.

Understanding these four processes as **topological operators** — each changing the shape
of the knowledge space in specific ways — is the core contribution of this perspective.

---

## Why This Matters for AI Systems

For AI systems specifically, knowledge topology has operational consequences:

1. **Retrieval quality**: a densely connected knowledge topology enables richer context
   assembly. A sparse, disconnected topology produces context that is impoverished
   regardless of the individual quality of facts.

2. **Robustness**: a knowledge space with many paths between any two facts is robust to
   node removal (forgetting individual facts). A space with bottlenecks is brittle.

3. **Graceful degradation**: when knowledge decays, does the topology degrade smoothly
   (uniform connectivity decrease) or catastrophically (bottleneck removal disconnects
   large components)?

4. **Knowledge integration**: when new information arrives, does it connect to existing
   knowledge (integrates smoothly into the topology) or arrive as an isolated fact
   (increases fragmentation)?

These consequences are directly relevant to Roko's Neuro architecture and its decay models.
