# 39-25 Research to Runtime -- Annotated Reference Map

> Annotations are kept only for works whose descriptions were checked against the papers (listed in
> `tools/docs_integrity/content_audit.json` and in the banner of docs/v3/REFERENCES.md). The other entries are
> bare citations: their annotations were removed because they had not been checked against the papers.

> Target-state pipeline for translating academic citations into tested runtime
> heuristics, plus the foundational "starter kit" papers whose claims anchor
> existing architectural primitives.
>
> **v3 depth file** -- updated 2026-09-15.

---

## The Research-to-Runtime Pipeline

The pipeline translates academic citations into tested runtime heuristics in five
stages:

1. **Paper**: Published result with quantitative claim
2. **Claim**: Extracted, falsifiable assertion (e.g., "Ebbinghaus decay with
   half-life 7d retains more useful heuristics than flat storage")
3. **Heuristic**: Implemented runtime rule derived from the claim (e.g.,
   `decay_factor = 0.5^(elapsed / half_life)`)
4. **Trial**: A/B experiment measuring the heuristic against null baseline
   (e.g., prompt experiment comparing decayed vs. flat retrieval)
5. **Calibration**: Bayesian parameter tuning from production outcomes (e.g.,
   adjusting half-life from 7d to 5d based on gate pass rates)

Each step has a replication contract format (see REFERENCES.md gist for Rust
data shapes). The pipeline is designed to be self-improving: calibration
outcomes feed back into claim refinement.

---

## Starter Kit: Papers Anchoring Existing Primitives

These are the foundational papers whose claims are already implemented as
runtime primitives in the codebase.

### HDC and Knowledge Representation

**[Kanerva, 2009]** *Hyperdimensional Computing.* Cognitive Computation, 1(2), 139--159.

### Active Inference and Tier Routing

**[Friston, 2010]** *The Free-Energy Principle: A Unified Brain Theory?* Nature Reviews Neuroscience, 11(2), 127--138.

### Collective Intelligence

**[Woolley et al., 2010]** *Evidence for a Collective Intelligence Factor.* Science, 330(6004), 686--688.

### Reinforcement Learning Foundations

**[Sutton & Barto, 2018]** *Reinforcement Learning: An Introduction.* 2nd ed. MIT Press.

### Bandit Algorithms

**[Auer et al., 2002]** *Finite-time Analysis of the Multiarmed Bandit Problem.* Machine Learning, 47(2--3), 235--256.

### Prediction Markets

**[Hanson, 2003]** *Combinatorial Information Market Design.* Information Systems Frontiers, 5(1), 107--119.

### Cooperation Theory

**[Axelrod, 1984]** *The Evolution of Cooperation.* Basic Books.

### Groupthink Prevention

**[Janis, 1972]** *Victims of Groupthink.* Houghton Mifflin.

### Predictive Processing

**[Clark, 2013]** *Whatever Next? Predictive Brains, Situated Agents.* Behavioral and Brain Sciences, 36(3), 181--204.

### Demurrage Economics

**[Gesell, 1916]** *The Natural Economic Order.* 1916.

### Commons Governance

**[Ostrom, 1990]** *Governing the Commons.* Cambridge University Press.

### Sensemaking

**[Weick, 1995]** *Sensemaking in Organizations.* SAGE Publications.

---

## Replication Contract Format

Each research-to-runtime binding uses a structured replication contract:

```
ReplicationContract {
    paper_id: String,        // arXiv ID or DOI
    claim: String,           // falsifiable assertion
    heuristic: String,       // implemented rule (crate::module::function)
    baseline: String,        // null comparator
    metric: String,          // measured quantity
    threshold: f64,          // minimum improvement for adoption
    trial_id: Option<String> // prompt experiment ID if active
}
```

Active trials are tracked in `.roko/learn/experiments.json`. Completed
calibrations are persisted in `.roko/learn/cascade-router.json` and
`.roko/learn/gate-thresholds.json`.

---

## Pipeline Status

| Stage | Count | Notes |
|---|---|---|
| Papers cited | 440+ | Master bibliography in REFERENCES.md |
| Claims extracted | ~50 | Starter kit + core architectural primitives |
| Heuristics implemented | ~30 | NeuroStore decay, tier routing, VCG auction |
| Trials run | ~10 | Prompt experiments via runner dispatch |
| Calibrations completed | ~5 | CascadeRouter, gate thresholds |

The gap between papers cited (440+) and calibrations completed (~5) is the
research-to-runtime frontier. The pipeline exists; the throughput needs
production dogfood data to increase.

---

## Cross-References

- All domain sub-docs [00](./00-lifecycle-and-finite-agency.md)--[23](./23-generational-and-evolutionary.md) provide the paper-to-claim mapping
- Prompt experiments: depth/08-learning/
- Gate thresholds: depth/07-gates/
- CascadeRouter: depth/08-learning/
