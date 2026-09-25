# 39-25 Research to Runtime -- Annotated Reference Map

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
Claim: 10,000+ dimensional binary vectors provide sufficient capacity for practical knowledge systems. Runtime: 10,240-bit BSC vectors in `roko-primitives`. Calibration: dimensionality validated by orthogonality tests. (See [09](./09-hdc-vsa.md).)

### Active Inference and Tier Routing

**[Friston, 2010]** *The Free-Energy Principle: A Unified Brain Theory?* Nature Reviews Neuroscience, 11(2), 127--138.
Claim: perception, action, and learning minimize variational free energy. Runtime: EFE-based tier routing in CascadeRouter. Calibration: routing ratios (T0 80%, T1 15%, T2 5%) tuned from production outcomes. (See [16](./16-active-inference.md).)

### Collective Intelligence

**[Woolley et al., 2010]** *Evidence for a Collective Intelligence Factor.* Science, 330(6004), 686--688.
Claim: groups have measurable collective intelligence independent of individual IQ. Runtime: C-Factor metric in `roko-learn`. Calibration: threshold C-Factor > 1.0 for superorganism classification. (See [18](./18-collective-intelligence.md).)

### Reinforcement Learning Foundations

**[Sutton & Barto, 2018]** *Reinforcement Learning: An Introduction.* 2nd ed. MIT Press.
Claim: temporal-difference learning converges to optimal value estimates. Runtime: TD-based learning in CascadeRouter model routing. Calibration: learning rate and discount factor from production outcomes.

### Bandit Algorithms

**[Auer et al., 2002]** *Finite-time Analysis of the Multiarmed Bandit Problem.* Machine Learning, 47(2--3), 235--256.
Claim: UCB achieves logarithmic regret. Runtime: UCB-style exploration in CascadeRouter. Calibration: exploration bonus coefficient tuned from routing outcomes.

### Prediction Markets

**[Hanson, 2003]** *Combinatorial Information Market Design.* Information Systems Frontiers, 5(1), 107--119.
Claim: prediction markets aggregate distributed information efficiently. Runtime: informs collective knowledge pricing and confidence aggregation. Calibration: market microstructure parameters.

### Cooperation Theory

**[Axelrod, 1984]** *The Evolution of Cooperation.* Basic Books.
Claim: tit-for-tat dominates iterated prisoner's dilemma. Runtime: cooperative agent strategies in multi-agent coordination. Calibration: cooperation threshold from Agent Group outcomes.

### Groupthink Prevention

**[Janis, 1972]** *Victims of Groupthink.* Houghton Mifflin.
Claim: high-cohesion groups suppress dissent. Runtime: 15% contrarian retrieval mandate. Calibration: contrarian percentage from knowledge diversity metrics.

### Predictive Processing

**[Clark, 2013]** *Whatever Next? Predictive Brains, Situated Agents.* Behavioral and Brain Sciences, 36(3), 181--204.
Claim: brains are prediction machines minimizing prediction error. Runtime: prediction-error-driven T0 probes and tier escalation. Calibration: prediction error threshold from CalibrationTracker. (See [12](./12-signal-processing.md).)

### Demurrage Economics

**[Gesell, 1916]** *The Natural Economic Order.* 1916.
Claim: money that decays encourages circulation. Runtime: KORAI 1% annual demurrage, knowledge half-life decay. Calibration: demurrage rate from knowledge circulation metrics. (See [21](./21-mechanism-design.md).)

### Commons Governance

**[Ostrom, 1990]** *Governing the Commons.* Cambridge University Press.
Claim: commons can be governed without central authority using eight design principles. Runtime: Agent Group governance rules. Calibration: rule parameters from group performance. (See [21](./21-mechanism-design.md).)

### Sensemaking

**[Weick, 1995]** *Sensemaking in Organizations.* SAGE Publications.
Claim: organizations construct meaning retrospectively through ongoing narratives. Runtime: agent interpretation of ambiguous signals via episode narrative construction. Calibration: narrative quality from downstream task outcomes.

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
