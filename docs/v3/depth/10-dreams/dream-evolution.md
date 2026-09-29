# Dream Evolution: The Fourth Phase

> **v3 depth file** -- `/docs/v3/depth/10-dreams/dream-evolution.md`
> Canonical source: v1 `docs/v1/10-dreams/05-dream-evolution.md`
> Implementation: `crates/roko-dreams/` (design-stable; EVOLUTION phase not yet live)
> Status: **Specified** -- HDC primitives (permutation, bundling, similarity) available
> in `roko-primitives` and `roko-learn`; runtime integration pending

---

## 1. The EVOLUTION Phase

Beyond the three core phases (NREM Replay, REM Imagination, Integration), the
dream system includes a fourth phase: **EVOLUTION**. This phase operates on
promoted knowledge entries -- those that have already passed through the staging
buffer and been validated by waking experience -- and applies evolutionary
selection pressures to generate higher-order strategies.

EVOLUTION is not triggered every dream cycle. It fires when the agent has
accumulated a sufficient body of promoted knowledge (configurable threshold,
default: 20 promoted entries since the last EVOLUTION cycle).

---

## 2. Three Operations

### 2.1 Memetic Selection

Heuristics and strategies in NeuroStore compete for survival. EVOLUTION evaluates
each promoted entry against the agent's recent performance:

- **High-fitness heuristics** (correlated with success) receive confidence boosts
  and have their half-life extended by 1.5x.
- **Low-fitness heuristics** (correlated with failure or never referenced) receive
  confidence penalties and begin accelerated decay.
- **Neutral heuristics** (no correlation) are left unchanged.

The fitness function:

```
fitness(heuristic) = success_rate_when_referenced / success_rate_when_not_referenced
```

This implements memetic evolution from Dawkins (1976, The Selfish Gene): ideas
compete for replication within the agent's cognitive architecture.

### 2.2 Strategy Evolution via Imagined Returns

EVOLUTION takes pairs of high-fitness heuristics and combines them to produce
candidate super-strategies. The compound strategies enter the staging buffer at
confidence 0.30.

### 2.3 Knowledge Recombination

EVOLUTION applies Wright's (1932) shifting balance theory: knowledge entries are
randomly paired and recombined using HDC permutation -- a cyclic bit-shift on the
10,240-bit BSC vector:

```rust
let recombined = HdcVector::bundle(&[
    &entry_a.hdc_vector,
    &entry_b.hdc_vector.permute(shift_amount),
]);
```

The permuted bundle creates a vector related to both parents but distinct from
either. Nearest neighbors identify potentially relevant but unconnected knowledge.

---

## 3. Bayesian Memetic Fitness

The naive fitness ratio has statistical weaknesses. The formalized approach uses
a Bayesian framework with uncertainty quantification:

```rust
pub struct BayesianMemeticFitness {
    pub prior_mean: f64,               // default: 1.0
    pub prior_std: f64,                // default: 0.5
    pub min_observations: usize,       // default: 5
    pub confidence_threshold: f64,     // default: 0.75
    pub control_for_confounders: bool, // default: true
    pub max_confounders: usize,        // default: 5
}

pub enum FitnessClassification {
    Beneficial,  // P(fitness > 1.0) > threshold
    Harmful,     // P(fitness < 1.0) > threshold
    Uncertain,   // insufficient evidence
}
```

Monte Carlo estimation samples from Beta-Binomial posteriors to estimate
P(fitness > 1.0 | data).

---

## 4. MAP-Elites for Strategy Evolution

Quality-diversity search via MAP-Elites (Mouret & Clune 2015) maintains a
diverse archive indexed by behavioral descriptors. Instead of converging on a
single "best" strategy, QD search produces a repertoire of diverse strategies.

```rust
pub struct MapElitesArchive {
    pub descriptor_dimensions: Vec<DescriptorDimension>,
    pub bins_per_dimension: usize,     // default: 10
    pub max_archive_size: usize,       // default: 1000
    pub mutation_rate: f64,            // default: 0.20
    pub hdc_descriptors: bool,         // default: true
    pub min_quality_threshold: f64,    // default: 0.30
}
```

---

## 5. The Dream-Prediction Feedback Loop

EVOLUTION closes a critical feedback loop:

```
DREAM -> Generate hypothesis H with predicted outcome P
WAKE  -> Observe actual outcome O
DREAM -> Compare P vs O
  If P ~ O: boost confidence, reinforce generating heuristics
  If P != O: reduce confidence, weaken generating heuristics
DREAM -> Next EVOLUTION considers updated fitness scores
```

---

## 6. Academic Citations

| Paper | Contribution |
|-------|-------------|
| Dawkins (1976), The Selfish Gene | Memetic evolution in cognitive architectures |
| Wright (1932) | Shifting balance theory for knowledge recombination |
| Kanerva (2009), Cognitive Computation | HDC permutation for knowledge exploration |
| Simonton (2010) | BVSR theory applied to strategy evolution |
| Mouret & Clune (2015), arXiv:1504.04909 | MAP-Elites quality-diversity search |
| DCRL-MAP-Elites (ACM TELO 2024) | Descriptor-conditioned actors for diverse solutions |
| Rainbow Teaming (NeurIPS 2024) | MAP-Elites for adversarial prompt diversity |

---

## 7. Cross-References

| Document | Relevance |
|----------|-----------|
| [consolidation-and-staging.md](consolidation-and-staging.md) | Promoted entries that EVOLUTION operates on |
| [hdc-counterfactual-synthesis.md](hdc-counterfactual-synthesis.md) | HDC operations for recombination |
| [advanced-dream-concepts.md](advanced-dream-concepts.md) | Extended evolutionary concepts |
