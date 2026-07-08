# Temporal Shape — How Knowledge Evolves

**Kind**: Perspective
**Source**: `docs/00-architecture/27-temporal-knowledge-topology.md`

---

## Knowledge as a Dynamic System

The knowledge topology is not static. At each moment, the knowledge space has a particular
shape; over time, that shape changes. Understanding the dynamics of this evolution —
how the topology changes in response to ingestion, decay, consolidation, and contradiction —
is essential for designing systems that behave correctly over time.

Four processes drive the temporal evolution of knowledge topology:

---

## 1. Ingestion: Expansion at the Frontier

When new Engrams are added to the knowledge base, the topology changes at the **frontier** —
the boundary between known and unknown.

**Good ingestion**: a new Engram connects to existing knowledge (it references concepts
already in the graph, extends a chain of reasoning, confirms or refines existing beliefs).
This expands the topology **continuously**: the new Engram attaches smoothly to the existing
structure, increasing local connectivity.

**Poor ingestion**: a new Engram arrives as an isolated fact with no connections to existing
knowledge. This creates a new connected component — it increases fragmentation rather than
connectivity.

The quality of ingestion from a topological perspective depends on:
- **Richness of provenance metadata**: Engrams with detailed origin information can be linked
  to other Engrams from the same source, same event, or same reasoning chain.
- **Semantic tagging**: HDC vectors that encode semantic content enable semantic-distance
  connections even without explicit linkage.
- **Cross-referencing in the Composer**: synthesis operations that link multiple Engrams
  create explicit topological connections.

---

## 2. Decay: Erosion and Disconnection

Decay is the process by which Engrams lose confidence and eventually expire. Topologically,
decay is an **erosion** of the knowledge space:
- As individual Engrams decay, their edge weights decrease.
- When Engrams expire, their nodes and edges are removed.
- If a decaying Engram was a hub or bridge, its decay creates disconnection.

The four decay models in Roko ([reference/10-types/decay.md](../../../reference/10-types/decay.md))
have different topological effects:

| Decay Model | Topological Effect |
|-------------|-------------------|
| **Exponential** | Uniform erosion; edges thin steadily, no sharp transitions |
| **Step** | Phase transition at expiry: abrupt removal of nodes and edges |
| **Linear** | Gradual erosion; slower at first, faster near expiry |
| **Plateau** | Stable region followed by rapid decay; knowledge persists then vanishes |

The choice of decay model determines whether the topology degrades **smoothly** (exponential,
linear) or **discontinuously** (step, plateau). Smooth degradation is more robust: the
topology retains connectivity until confidence is very low. Discontinuous degradation can
produce catastrophic disconnection at the expiry threshold.

### Graceful Degradation Under Decay

An ideal knowledge system degrades gracefully: as specific facts fade, they are replaced
by more general summaries that preserve the topological structure at a coarser resolution.
This is the biological analog of semantic memory: specific episodic memories fade but
general semantic knowledge persists.

In Roko, graceful degradation requires the Dreams consolidation process to produce
**summaries before sources decay**: creating bridging Engrams that preserve connectivity
even after the specific Engrams that originally created it have expired.

---

## 3. Consolidation: Structural Enrichment

Consolidation (the Dreams process) is the topological dual of decay: it enriches the
topology rather than eroding it.

In biological terms, sleep consolidation transforms episodic memory (specific event records)
into semantic memory (general knowledge structures). In topological terms, this is:
- **Local clustering increase**: consolidation creates new connections among related facts.
- **Bridge creation**: summary Engrams link domains that were previously loosely connected.
- **Dimensionality reduction**: many specific facts are summarized by fewer general facts,
  reducing the number of nodes while preserving connectivity.
- **Confidence elevation**: consolidated knowledge achieves higher confidence than the
  individual Engrams from which it was derived.

The topological ideal for consolidation: the post-consolidation topology should be a
**topological simplification** of the pre-consolidation topology — homeomorphic to the
original (same large-scale structure) but with lower complexity (fewer nodes, denser
connections).

---

## 4. Contradiction: Topological Tearing

When an Engram contradicts existing high-confidence knowledge, the topology is "torn" at
the site of contradiction: the contradicted fact can no longer serve as a bridge between
facts that depend on it.

**Example**: If a bridge fact B links domains D1 and D2, and B is contradicted, then D1
and D2 become topologically separated until either:
1. B is replaced by an updated fact B' that connects them differently.
2. An alternative path connecting D1 and D2 is found.
3. The contradiction is resolved in favor of B (the contradicting Engram is rejected).

Unresolved contradictions create **topological holes** — facts that exist on both sides of
a gap but cannot be reconciled without tearing the topology. The cognitive analog is
cognitive dissonance: two beliefs that cannot simultaneously be held without contradiction.

The [Dreams consolidation](../../../reference/09-cross-cuts/README.md) process should
actively detect and resolve topological holes: identify contradicted facts, assess the
confidence of the contradiction, and either update the topology (remove the old fact) or
quarantine the contradiction (mark it as unresolved for human review).
