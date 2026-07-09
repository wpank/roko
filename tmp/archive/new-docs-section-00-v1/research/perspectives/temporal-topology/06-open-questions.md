# Open Questions — Temporal Knowledge Topology

**Kind**: Perspective
**Source**: `docs/00-architecture/27-temporal-knowledge-topology.md`

---

## Unresolved Design Questions

1. **Which topology is the right topology?** The hop distance topology, the HDC semantic
   distance topology, the temporal co-occurrence topology, and the epistemic inference
   topology all induce different shapes. Which one should be primary? Should they be
   combined, and if so, how?

2. **How expensive is topological analysis?** Computing topological invariants (connected
   components, diameter, persistent homology) at scale is non-trivial. What is the
   computational budget for topological health monitoring, and at what frequency can it
   be run?

3. **Can topological centrality be computed incrementally?** Full centrality computation
   on the Engram graph scales poorly. Can incremental updates be computed as Engrams are
   added or removed, without recomputing the full graph?

4. **Is topological degradation a useful early warning signal?** Does degrading topology
   predict declining reasoning quality? If so, topological health metrics would serve as
   leading indicators of system performance.

5. **What is the relationship between topological structure and HDC similarity?** The HDC
   distance topology and the Engram graph topology are defined independently. Do they
   correlate? Are topologically distant Engrams (in the graph) also semantically distant
   (in HDC space)?

---

## Open Research Questions

1. **Sheaf theory for knowledge bases**: a *sheaf* is a mathematical object that captures
   local-global consistency — when local data patches agree, they can be extended to a
   global consistent structure. A sheaf-theoretic treatment of knowledge bases would make
   the conditions for global consistency (no contradictions) formally precise. Is this
   tractable at the scale of Roko's Engram stores?

2. **Information geometry**: the space of probability distributions over propositions has
   a natural Riemannian metric (Fisher information metric). Can Roko's confidence-weighted
   Engram space be analyzed using information geometry, and does this provide better
   optimization targets for decay and consolidation?

3. **How does the [energy model perspective](../energy-model/README.md) interact with
   topology?** Energy-based decay (high-energy Engrams require more resources to maintain)
   may produce topologically different degradation patterns than time-based decay. The
   intersection of these two perspectives is unexplored.
