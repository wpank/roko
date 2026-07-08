# Roko Application — Temporal Knowledge Topology

**Kind**: Perspective
**Source**: `docs/00-architecture/27-temporal-knowledge-topology.md`

---

## Mapping Topology to Roko's Architecture

### Engram Graph as the Knowledge Space

The set of Engrams in the [Substrate](../../../reference/03-substrate/README.md) at any
moment is Roko's knowledge space. The topology of this space is determined by:
- **Lineage links**: the DAG edges connecting Engrams via the provenance graph
- **Content similarity**: semantic links derivable from HDC vector distances
- **Explicit cross-references**: Engrams that cite other Engrams in their content
- **Temporal co-occurrence**: Engrams created in the same time window from the same context

These four link types induce four overlapping topologies on the Engram space. The "true"
topology is the union (each link type contributes edges to the same underlying graph).

### HDC Vectors as Topological Coordinates

The [HDC fingerprint](../../../reference/10-types/hdc-fingerprint.md) on each Engram places
it in a 10,240-dimensional binary space. The Hamming distance between two vectors is a
metric, making the Engram collection a **metric space** (not just a graph).

This metric space admits topological analysis:
- **Density estimation**: how dense are different regions of HDC space? Sparse regions
  represent knowledge gaps.
- **Cluster analysis**: which Engrams form tight semantic clusters? These are the
  topological "domains" of Roko's knowledge.
- **Boundary detection**: where are the edges of semantic clusters? Engrams at the
  boundary between clusters are the bridges.

Neuro probing (querying by HDC similarity) is topological nearest-neighbor search: find
the Engrams topologically closest to the query vector.

### Decay Tier Matrix as Topological Policy

The [Decay tier matrix](../../../reference/10-types/decay.md) assigns decay models to Engram
types. From a topological perspective, this is a **topological policy**: it determines how
different regions of the knowledge space evolve over time.

A well-designed decay tier matrix should:
1. Apply short-lived decay to peripheral Engrams (low degree, isolated) — they have little
   topological impact when they expire.
2. Apply longer-lived decay or plateau models to hub Engrams — their expiry would
   significantly disrupt the topology.
3. Prioritize Dreams consolidation for Engrams with high bridging centrality
   (bridges between connected components).
4. Apply step decay only to time-bounded Engrams whose content has a hard expiry (event
   schedules, temporary authorizations).

The current decay tier matrix ([reference/10-types/decay.md](../../../reference/10-types/decay.md))
is organized by Engram Kind, not by topological role. Incorporating topological centrality
into decay policy decisions would produce more topologically stable knowledge bases.

### Dreams as Topological Surgery

The [Dreams consolidation](../../../reference/09-cross-cuts/README.md) process is, from
a topological perspective, **topological surgery**: it modifies the topology to preserve
essential structure while reducing complexity.

Specific topological surgery operations Dreams can perform:
- **Bridge reinforcement**: creating summary Engrams that become new bridges between components
  that were connected only through expiring Engrams.
- **Component merger**: linking two loosely-connected clusters with a new summary Engram that
  covers both.
- **Hole filling**: creating new Engrams in knowledge gaps identified by density analysis
  of HDC space.
- **Hub consolidation**: merging multiple high-degree Engrams on the same topic into a
  single denser hub.

### Temporal Depth and the Loop

The [Universal Cognitive Loop](../../../reference/06-loop/README.md)'s three speeds
(T0/T1/T2) correspond to different temporal depths of the knowledge topology:
- **T0 (fast)**: operates on the most recent, highest-confidence region of the topology —
  the "frontier" of just-acquired knowledge.
- **T1 (standard)**: operates on the stable, well-connected middle of the topology.
- **T2 (deliberate)**: can traverse the full topology, including deep historical and
  loosely-connected regions.

This mapping suggests that T2 quality is directly related to **topological reach**: the
system's ability to traverse distant, loosely-connected parts of the knowledge space. Gaps
in topological connectivity limit T2 quality.

### Neuro Probing as Topological Navigation

Neuro probing (querying Neuro with a probe vector and retrieving nearest-neighbor Engrams)
is **topological navigation**: finding the closest point in the knowledge space to the
query point.

The quality of Neuro retrieval depends directly on the topology:
- In a well-connected, dense topology, the nearest neighbor to any query is informative.
- In a sparse, fragmented topology, nearest neighbors may be far away (semantically distant)
  even if they are the closest available Engrams.
- Near topological holes, even the nearest neighbor may be uninformative.

This connects retrieval quality to topological health: a topologically healthy knowledge
base is one where any query vector is close to many relevant Engrams.
