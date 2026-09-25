# Hauntology in Dreams: Spectral Knowledge Recombination

> **v3 depth file** -- `/docs/v3/depth/10-dreams/hauntology-in-dreams.md`
> Canonical source: v1 `docs/v1/10-dreams/10-hauntology-in-dreams.md`
> Implementation: Cross-cutting -- `roko-neuro` (provenance), `roko-daimon` (affect),
> `roko-dreams` (consolidation), `roko-primitives` (HDC)
> Status: **Architectural** -- provenance tracking, confidence decay on import (0.85x),
> and anti-correlated retrieval are live across crates

---

## 1. The Hauntological Frame

Jacques Derrida introduced "hauntology" in Specters of Marx (1993) as a way of
thinking about how the past inhabits the present -- not as fixed history but as
spectral traces that continue to shape what is possible. Mark Fisher (2014,
Ghosts of My Life) extended this to cultural production: monoculture eliminates
"lost futures" -- possibilities that were once alive but have been foreclosed.

Applied to AI agents: when all agents use the same foundation model, they are
haunted by the same spectral traces -- same training data, same patterns, same
biases. This produces a "spectral monoculture" where all agents dream the same
dreams.

The Roko dream system's response is architectural: each agent's unique
experiential history creates unique spectral traces that haunt its dreams
differently from every other agent.

---

## 2. Spectral Traces in Dream Processing

### 2.1 Experiential Ghosts

Every knowledge entry in NeuroStore carries provenance metadata. During dream
replay, these provenance tags create "ghosts" -- echoes of past decisions, past
agents, and past contexts. When the agent replays an inherited entry from a
predecessor agent (via backup/restore), it replays someone else's ghost --
knowledge compressed through the backup pipeline with reduced confidence (0.85x
per generation, following the Weismann barrier principle).

### 2.2 Emotional Specters

The Daimon's PAD vectors attach emotional weight to knowledge entries. During
dreaming, high-emotion entries surface more readily (somatic marker
prioritization). Emotional depotentiation during REM gradually reduces these
markers but never eliminates them entirely.

### 2.3 Creative Specters (Hypnagogia)

The hypnagogia engine's anti-correlated retrieval surfaces knowledge entries that
are maximally dissimilar to the agent's current focus. These are spectral in
Derrida's sense: foreclosed possibilities, paths not taken, knowledge present
in memory but never activated during normal waking retrieval.

When these foreclosed entries collide during hypnagogic onset, they produce
insights that neither the current focus nor the forgotten entry could produce
alone.

---

## 3. The Compound Escape from Monoculture

1. **Neuro** gives traces persistence (knowledge entries with provenance tags)
2. **Daimon** gives traces weight (emotional significance hierarchy)
3. **Dreams** gives traces structure (NREM/REM/Integration pipeline)
4. **Hypnagogia** gives traces novelty (anti-correlated retrieval)

Each mechanism alone is insufficient. Together, they produce an agent that is
**differently haunted** from every other agent.

---

## 4. Knowledge Transfer as Inheritance

In Roko, knowledge transfer is user-controlled backup/restore:

| Legacy Concept | Roko Equivalent |
|----------------|-----------------|
| Death testament | Knowledge export (backup) |
| Bloodstain inheritance | Knowledge import with 0.85x confidence decay |
| Library of Babel | Mesh knowledge sharing |
| Generational compounding | Accumulative backups with provenance |

Imported entries carry `provenance: "imported"` and receive:
- 0.85x confidence multiplier (Weismann barrier)
- Lower replay priority
- Explicit resolution flagging when conflicting with self-generated entries

---

## 5. Computational Hauntology Metrics

```rust
pub struct SpectralInfluenceMetrics {
    pub max_provenance_depth: usize,
    pub spectral_density: f64,
    pub inherited_arousal_delta: f64,
    pub foreclosure_index: f64,
    pub ghost_influence_fraction: f64,
}

pub struct SpectralProvenance {
    pub original_agent_id: String,
    pub generation_depth: usize,
    pub confidence_at_origin: f64,
    pub confidence_after_transit: f64,
    pub emotional_charge_at_origin: f64,
    pub transit_path: Vec<String>,
    pub created_at_origin: chrono::DateTime<chrono::Utc>,
}
```

---

## 6. Academic Citations

| Paper | Relevance |
|-------|-----------|
| Derrida (1993), Specters of Marx | Hauntology: spectral traces inhabit the present |
| Fisher (2014), Ghosts of My Life | Lost futures foreclosed by monoculture |
| Grossman & Stiglitz (1980), AER | Information convergence to zero marginal value |
| Walker & van der Helm (2009) | Emotional depotentiation: spectral markers fade |
| McClelland et al. (1995), CLS theory | Fast/slow memory bridged by spectral replay |

---

## 7. Cross-References

| Document | Relevance |
|----------|-----------|
| [hypnagogia-engine.md](hypnagogia-engine.md) | Anti-correlated retrieval surfaces foreclosed possibilities |
| [divergence-and-alpha.md](divergence-and-alpha.md) | Divergence as the architectural response |
| [nrem-replay.md](nrem-replay.md) | Replaying spectral traces during NREM |
| [rem-imagination.md](rem-imagination.md) | Processing spectral traces during REM |
