# Adaptive Signal Metabolism

> **Parent:** [38-SIGNAL-ANALYSIS](../../38-SIGNAL-ANALYSIS.md) Section 7

---

## The Signal as a 5-Tuple

```rust
pub struct AdaptiveSignal {
    pub id: SignalId,
    pub function: Box<dyn Fn(&EngineState) -> f64 + Send + Sync>,
    pub confidence: f64,
    pub hdc_vector: HdcVector,
    pub weight: f64,
    pub context: SignalContext,
    pub lineage: Vec<SignalId>,
    pub generation: u64,
    pub created_at_ms: i64,
    pub evaluation_count: u64,
    pub accuracy: ExponentialMovingAverage,
}
```

## Hebbian Learning -- Oja's Rule

```rust
/// Oja (1982): Delta_w = eta * y * (x - y * w)
pub fn hebbian_update(
    signal: &mut AdaptiveSignal,
    prediction: f64,
    outcome: f64,
    learning_rate: f64,
) {
    let delta = learning_rate * outcome * (prediction - outcome * signal.confidence);
    signal.confidence = (signal.confidence + delta).clamp(0.0, 1.0);
}
```

### Learning rate calibration

| Domain | Recommended eta | Rationale |
|---|---|---|
| Coding | 0.05 | Lower noise, faster adaptation |
| Research | 0.02 | Moderate noise |
| Operations | 0.03 | Variable noise |

Adaptive annealing: `eta = base_eta / (1.0 + 0.01 * evaluation_count)`.
Satisfies Robbins-Monro conditions for convergence.

## Replicator Dynamics

```rust
/// Taylor & Jonker (1978): dw_i/dt = w_i * (f_i - f_bar)
pub fn replicator_update(registry: &mut SignalRegistry, dt: f64) {
    let total_weight: f64 = registry.iter().map(|s| s.weight).sum();
    let avg_fitness: f64 = registry.iter()
        .map(|s| s.weight * s.fitness() / total_weight)
        .sum();
    for signal in registry.iter_mut() {
        let delta = signal.weight * (signal.fitness() - avg_fitness) * dt;
        signal.weight = (signal.weight + delta).max(0.001);
    }
    registry.normalize_weights();
}
```

### Numerical stability

Forward Euler is stable when `dt * max(|f_i - f_bar|) < 1.0`. If violated,
subdivide the step:

```rust
pub fn replicator_update_safe(registry: &mut SignalRegistry, dt: f64) {
    let max_deviation = /* ... */;
    let n_substeps = ((dt * max_deviation).ceil() as usize).max(1);
    let sub_dt = dt / n_substeps as f64;
    for _ in 0..n_substeps {
        replicator_update(registry, sub_dt);
    }
}
```

### Fisher's Fundamental Theorem

```rust
pub fn fisher_variance(registry: &SignalRegistry) -> f64 {
    let avg_fitness = registry.mean_fitness();
    registry.iter()
        .map(|s| s.weight * (s.fitness() - avg_fitness).powi(2))
        .sum()
}
```

When V approaches 0: ensemble converged, needs mutation injection.

### Fitness computation

```rust
impl AdaptiveSignal {
    pub fn fitness(&self) -> f64 {
        let accuracy_score = 1.0 - self.accuracy.value().min(1.0);
        let info_ratio = self.information_ratio.unwrap_or(0.5);
        accuracy_score * (1.0 + info_ratio)  // range: [0.0, 2.0]
    }
}
```

## Speciation

```rust
pub fn speciate(parent: &AdaptiveSignal, mutation_rate: f64, rng: &mut impl Rng) -> AdaptiveSignal {
    let noise = HdcVector::random_with_density(mutation_rate, rng);
    let mutated_hv = parent.hdc_vector.xor(&noise);
    AdaptiveSignal {
        id: SignalId::new(),
        confidence: 0.5,
        hdc_vector: mutated_hv,
        weight: 0.01,
        generation: parent.generation + 1,
        // ...
    }
}
```

### Adaptive mutation rate

```rust
/// mutation_rate = base_rate * (1 + rq_pressure) / (1 + fisher_v / fisher_scale)
pub fn adaptive_mutation_rate(base_rate: f64, red_queen_pressure: f64, fisher_variance: f64) -> f64 {
    (base_rate * (1.0 + red_queen_pressure) / (1.0 + fisher_variance / 0.1)).clamp(0.01, 0.3)
}
```

## Red Queen Dynamic

```rust
pub fn apply_red_queen_pressure(registry: &mut SignalRegistry, decay_rate: f64) {
    for signal in registry.iter_mut() {
        signal.weight *= 1.0 - decay_rate;
    }
}
```

## SignalRegistry

```rust
pub struct SignalRegistry {
    signals: HashMap<SignalId, AdaptiveSignal>,
    max_population: usize,           // default: 500
    speciation_rate: f64,            // probability per generation
    extinction_threshold: f64,       // minimum weight
    generation: u64,
}

impl SignalRegistry {
    pub fn evolve_step(&mut self, data: &[Engram], outcomes: &[Engram]) {
        // 1. Evaluate all signals
        // 2. Hebbian update
        // 3. Replicator dynamics
        // 4. Speciate top-k (k=5) with adaptive mutation rate
        // 5. Extinction: remove below threshold
        // 6. Red Queen pressure
        // 7. Enforce population cap
        // 8. If Fisher's V < 0.001: inject 10 random signals
        self.generation += 1;
    }
}
```

## Heartbeat State Machine

```
GAMMA (read-only):
  Evaluate all signals, collect predictions. No weight updates.

THETA (learning):
  1. Resolve predictions from last cycle
  2. Hebbian update of confidence
  3. Replicator dynamics update of weights
  4. Update Fisher's variance

DELTA (evolution):
  1. Full replicator with accumulated dt
  2. Speciate (k=5, adaptive mutation rate)
  3. Extinction (weight < 0.001)
  4. Red Queen (decay_rate = 0.001)
  5. Population cap (500)
  6. Fitness landscape stagnation check
  7. Diversity injection if Fisher's V < 0.001
```

## Error Handling

- Division by zero in replicator: skip step, log warning
- NaN in Hebbian update: skip that signal
- Population collapse below 10: inject random signals
- Infinite fitness: clamp to [0.0, 10.0]

## Test Criteria

- **Replicator conservation**: total weight unchanged before normalization
- **Replicator stability**: dt=1.0 with deviations < 1.0, no negative weights
- **Hebbian convergence**: always-correct signal reaches confidence ~1.0 in 100 updates
- **Speciation diversity**: parent-child similarity 0.9-0.99 at mutation_rate=0.05
- **Fisher monotonicity**: identical fitness implies V = 0.0
- **Normalize idempotence**: calling twice produces same result

## Academic Foundations

- Taylor, P. D., & Jonker, L. B. (1978). "Evolutionary Stable Strategies." *Math. Biosciences*, 40(1-2).
- Fisher, R. A. (1930). *The Genetical Theory of Natural Selection*.
- Wright, S. (1932). "Roles of Mutation, Inbreeding, Crossbreeding, and Selection."
- Van Valen, L. (1973). "A New Evolutionary Law." *Evolutionary Theory*, 1.
- Oja, E. (1982). "Simplified neuron model as a principal component analyzer." *J. Math. Biology*, 15(3).
- Hebb, D. O. (1949). *The Organization of Behavior*.
