# Implications — Temporal Knowledge Topology

**Kind**: Perspective
**Source**: `docs/00-architecture/27-temporal-knowledge-topology.md`

---

## Design Decisions from the Topology Lens

### 1. Add Topological Metrics to Substrate Monitoring

The current monitoring for the Substrate focuses on quantitative metrics: number of Engrams,
storage size, query latency. Topology adds **structural health metrics**:
- **Giant component size**: what fraction of all Engrams are in the largest connected
  component? A declining fraction indicates increasing fragmentation.
- **Mean topological diameter**: is the average shortest path between Engrams increasing
  (fragmentation) or decreasing (consolidation)?
- **Bridge Engram count**: how many Engrams are bridges? High bridge count → fragile topology.
- **HDC density map**: where are the sparse regions in HDC space? These are knowledge gaps.

These metrics should be computed periodically (e.g., during Dreams cycles) and exposed in
monitoring dashboards.

### 2. Incorporate Topological Centrality into Decay Policy

The current decay tier matrix assigns decay models by Engram Kind. Topological centrality
(degree, betweenness centrality, bridge status) should modulate the decay model applied:

- **High-centrality Engrams** → automatically upgrade to a longer-lived decay model or
  trigger Dreams consolidation before expiry.
- **Bridge Engrams** → never expire without first consolidating their bridging function
  into a more durable Engram.
- **Isolated Engrams** (no connections) → may expire more aggressively without topological
  harm.

### 3. Use Persistent Homology for Knowledge Gap Detection

Persistent homology can identify **topological holes** in the knowledge space: regions
that are bounded by Engrams but empty in the interior. These holes represent known unknowns —
the system has enough context to recognize that a gap exists, even if it cannot fill it.

Exposing these holes as first-class objects (a list of known knowledge gaps with their
topological extent) would enable targeted ingestion: fetch Engrams specifically to fill
the largest knowledge gaps.

### 4. Consolidation-Before-Expiry Scheduling

Given the topological analysis of decay, the Dreams scheduler should have explicit logic
for **consolidation-before-expiry**: when a high-centrality Engram approaches its expiry
(say, \( < 20\% \) of its lifetime remains), trigger a Dreams cycle targeted at that
Engram's neighborhood before it expires.

This prevents topological disconnection by ensuring that the knowledge structure is
preserved in a more durable form before the specific Engrams that created it are gone.

### 5. Decay Model Selection by Topological Role

Rather than selecting decay models only by Kind, consider adding a topological-role dimension:

| Topological Role | Recommended Decay Model |
|-----------------|------------------------|
| Hub (high degree) | Plateau or exponential with long \( \tau \) |
| Bridge (high betweenness) | Plateau (stable until consolidation) |
| Peripheral (low degree, no bridges) | Exponential with short \( \tau \) |
| Frontier (recently added, not yet connected) | Linear (grace period to accumulate connections) |
| Isolated (no connections) | Step (expire if still isolated after timeout) |
