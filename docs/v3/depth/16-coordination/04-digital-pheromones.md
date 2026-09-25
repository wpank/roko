# Depth: Digital Pheromones

> Parent: [16-COORDINATION](../../16-COORDINATION.md) -- Sections 2-3

---

## What Are Digital Pheromones?

Digital pheromones are software analogs of the chemical pheromones used by
social insects for indirect coordination. In Roko, a digital pheromone is a
typed Signal that lives in a shared Substrate and is announced as a Pulse on
the Bus. The same coordination fact has two faces in the two-fabric model:
durable storage (Substrate) and ephemeral announcement (Bus).

The concept was formalized by Parunak, Brueckner & Sauter (2005), who showed
how to replicate the key properties of biological pheromones in software
[Parunak, H.V.D., Brueckner, S.A. & Sauter, J.A. "Digital Pheromones for
Coordination of Unmanned Vehicles." *E4MAS*, LNCS 3374:246-263, Springer,
2005].

Roko extends Parunak's framework with three additions:

1. **Typed pheromones**: Each pheromone has a `PheromoneKind` that determines
   its semantic meaning and default decay profile.
2. **Scoped propagation**: Pheromones propagate through Local, Mesh, or Global
   scopes, controlling audience and persistence.
3. **Confirmation reinforcement**: Multiple independent deposits of the same
   type extend effective half-life, implementing a quorum-sensing mechanism
   analogous to bacterial autoinducer accumulation [Nealson, Platt & Hastings,
   *J. Bacteriology*, 1970].

---

## Signal-First Pheromone View

The primary durable object is a `Signal` (backed by the `Engram` struct).
The `Pheromone` struct is an implementation-facing view over that Signal's
tags and body when the system wants typed ergonomics. Storage stays
Signal-first; live notification stays Pulse-first.

```rust
/// A digital pheromone -- a typed Signal carrying coordination information.
pub struct Pheromone {
    /// The type of coordination signal. Determines default decay profile
    /// and semantic meaning. See PheromoneKind for the full taxonomy.
    pub kind: PheromoneKind,

    /// Current intensity. Range: [0.0, 1.0]. Starts at initial_intensity
    /// and decays exponentially. Below sensing threshold (0.01), eligible
    /// for garbage collection.
    pub intensity: f64,

    /// Half-life -- duration after which intensity drops to 50%.
    /// Different kinds have different default half-lives:
    ///   Threat: 2h, Opportunity: 4h, Wisdom: 24h, Alpha: 1h,
    ///   Pattern: 12h, Anomaly: 6h, Consensus: 48h
    pub decay_rate: Duration,

    /// The agent that deposited this pheromone.
    pub source: AgentId,

    /// Propagation scope.
    ///   Local(SubstrateId): visible only within the agent's own store
    ///   Mesh(CollectiveId): visible to all agents via MeshBus
    ///   Global: visible to all agents on the chain
    pub scope: PheromoneScope,
}
```

### Relationship to the Signal Type

The Signal's `tags` field carries metadata that `Substrate::query()` can
filter on:

| Tag Key | Example Value | Purpose |
|---------|--------------|---------|
| `pheromone_kind` | `"Threat"` | Filter by signal type |
| `pheromone_scope` | `"Mesh(collective-42)"` | Filter by propagation scope |
| `bus_topic` | `"mesh.pheromone.deposited"` | Topic used for Bus announcement |
| `pheromone_intensity` | `"0.87"` | Current intensity (updated on read) |
| `pheromone_confirmations` | `"3"` | Number of independent confirmations |
| `pheromone_domain` | `"code-quality"` | Domain-specific context |

---

## Exponential Decay

The most important property of digital pheromones is decay over time.
Biological pheromones evaporate through chemical degradation; digital
pheromones decay through an explicit exponential function.

### The Decay Formula

```rust
/// Compute the current intensity of a pheromone at time `now`.
///
/// Uses exponential decay with confirmation-extended half-life:
///
///   intensity(t) = base_intensity x e^(-0.693 x elapsed / tau_effective)
///
/// where:
///   tau_effective = tau_base x (1 + confirmations x 0.5)
///
/// This means:
///   0 confirmations: half-life = tau_base (e.g., 2 hours for Threat)
///   1 confirmation:  half-life = 1.5 x tau_base (3 hours for Threat)
///   2 confirmations: half-life = 2.0 x tau_base (4 hours for Threat)
///   4 confirmations: half-life = 3.0 x tau_base (6 hours for Threat)
///
/// The 0.693 constant is ln(2), producing exactly 50% intensity at
/// t = tau_effective.
///
/// Cite: Dorigo, M. "Ant Colony Optimization." IEEE SMC-B, 26(1), 1997.
pub fn pheromone_decay(
    base_intensity: f64,
    deposited_at: Instant,
    half_life: Duration,
    confirmations: u32,
) -> f64 {
    let effective_half_life = half_life.mul_f64(
        1.0 + confirmations as f64 * 0.5
    );
    let elapsed = deposited_at.elapsed();
    let decay_factor = (-0.693 * elapsed.as_secs_f64()
        / effective_half_life.as_secs_f64()).exp();
    base_intensity * decay_factor
}
```

### Why Exponential Decay?

| Property | Exponential | Linear | Step Function |
|----------|-----------|--------|---------------|
| **Smoothness** | Continuous, differentiable | Continuous, not differentiable at endpoint | Discontinuous |
| **Recency bias** | Strong initially, weakens | Constant rate | All-or-nothing |
| **Natural interpretation** | Half-life is intuitive | "Runs out in X seconds" | "Valid for X seconds" |
| **Biological fidelity** | Matches chemical kinetics | No analog | No analog |
| **Composition** | Product of exponentials = exponential | Sum of linears = linear | Min of steps = step |

The exponential decay function has the memoryless property: at any point, the
expected remaining time until a threshold depends only on current intensity,
not on how long the pheromone has existed.

### Bucketed Decay (Target Design)

For large-scale pheromone fields, per-signal continuous decay is replaced by
bucketed decay. Signals are grouped into time buckets (e.g., 5-minute
intervals), and the entire bucket is decayed in one operation:

```
Bucket decay:
  bucket_intensity = aggregate_intensity x 2^(-bucket_age / half_life)
```

This reduces per-tick computation from O(P) (P = active pheromones) to
O(B) (B = active buckets, B << P). Bucketed decay is specified but not yet
wired end-to-end.

### Decay Profiles by Kind

| Kind | Half-Life | Rationale |
|------|-----------|-----------|
| `Threat` | 2 hours | Threats need immediate response; stale threats should fade |
| `Opportunity` | 4 hours | Opportunities are moderately time-sensitive |
| `Wisdom` | 24 hours | Insights should persist long enough for multiple agents |
| `Alpha` | 1 hour | First-mover advantage is the most ephemeral signal |
| `Pattern` | 12 hours | Confirmed patterns should last through a dev cycle |
| `Anomaly` | 6 hours | Anomalies need investigation within a working day |
| `Consensus` | 48 hours | Collective agreement should persist across multiple cycles |

### Worked Example

A `Threat` pheromone deposited with `base_intensity = 1.0`, `half_life = 2h`:

| Time | 0 confirms | 1 confirm (tau=3h) | 3 confirms (tau=5h) |
|------|-----------|--------------------|---------------------|
| T+0h | 1.000 | 1.000 | 1.000 |
| T+1h | 0.707 | 0.794 | 0.871 |
| T+2h | 0.500 | 0.630 | 0.758 |
| T+3h | 0.354 | 0.500 | 0.660 |
| T+6h | 0.125 | 0.250 | 0.435 |
| T+12h | 0.016 | 0.063 | 0.189 |
| T+24h | 0.000 | 0.004 | 0.036 |

With 3 confirmations, a Threat that would be negligible at T+12h still has
19% intensity -- enough to influence behavior for a full working day.

---

## Confirmation Mechanics

Confirmation is the mechanism by which multiple agents reinforce a pheromone
signal.

### Confirmation Rules

1. **Independence**: The confirming agent must not be the original depositor.
2. **Same kind and scope**: Must match both `PheromoneKind` and
   `PheromoneScope`.
3. **Proximity**: The confirming deposit must be "near" the original in the
   Substrate's address space.
4. **Temporal window**: Must occur while the original's intensity exceeds the
   sensing threshold (0.01).
5. **Anti-spoofing**: Confirmation is weighted by the confirming agent's
   reputation score.

### Anti-Spoofing via Reputation Weighting

```rust
/// Compute effective confirmation count, weighted by confirmer reputation.
///
/// Prevents Sybil attacks where many low-reputation agents artificially
/// extend a pheromone's lifetime.
pub fn effective_confirmations(
    confirmations: &[(AgentId, f64)],  // (confirmer, reputation)
) -> f64 {
    confirmations.iter()
        .map(|(_, rep)| rep.clamp(0.0, 1.0))
        .sum()
}
```

The effective half-life then uses `effective_confirmations`:

```
tau_effective = tau_base x (1 + effective_confirmations x 0.5)
```

---

## The Pheromone Field

The aggregate of all active pheromones in a scope constitutes a **pheromone
field** -- a multi-dimensional signal landscape that agents navigate.

### Field Operations

| Operation | Description | Trait |
|-----------|-------------|-------|
| **Deposition** | Agent deposits a new pheromone Signal | `Substrate::store()` |
| **Sensing** | Agent queries for pheromones above threshold | `Substrate::query()` + `Scorer::score()` |
| **Reinforcement** | Agent confirms existing pheromone | `Substrate::store()` with proximity matching |
| **Evaporation** | Intensity decreases over time | Computed on-read via `pheromone_decay()` |

### Field Composition

When multiple pheromones of different kinds coexist, their combined effect:

```
composite_signal = sum(pheromone_i.intensity x kind_weight_i x relevance_i)
```

Where `kind_weight_i` is configurable per agent role and `relevance_i` is
scored by the `Scorer`.

---

## Pheromone Interference and Crosstalk

Multiple pheromone types can interfere with each other. The interference
is modeled as signal-to-interference-plus-noise ratio (SINR):

```
SINR_k = I_target_k / (sum_{j!=k} alpha_{jk} x I_j + N_0)
```

Where:
- `I_target_k` = intensity of target pheromone of kind k
- `I_j` = intensity of interfering pheromones of kind j
- `alpha_{jk}` = cross-kind interference coefficient
- `N_0` = background noise floor

### Default Interference Matrix

```rust
/// Default interference matrix for 7 universal kinds.
///   Threat -> Opportunity: 0.6 (alarm suppresses foraging, per Wilson 1971)
///   Threat -> Wisdom: 0.1 (knowledge resists alarm)
///   Opportunity -> Threat: 0.0 (opportunities don't mask threats)
///   Consensus -> all: 0.05 (consensus resists interference)
pub fn default_universal() -> InterferenceMatrix {
    //         Thr  Opp  Wis  Alp  Pat  Ano  Con
    let m = vec![
        vec![0.0, 0.6, 0.1, 0.3, 0.2, 0.1, 0.05], // Threat
        vec![0.0, 0.0, 0.0, 0.1, 0.05, 0.0, 0.0],  // Opportunity
        vec![0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],   // Wisdom
        vec![0.1, 0.1, 0.0, 0.0, 0.05, 0.0, 0.0],  // Alpha
        vec![0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],   // Pattern
        vec![0.2, 0.1, 0.0, 0.1, 0.1, 0.0, 0.0],   // Anomaly
        vec![0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],   // Consensus
    ];
    InterferenceMatrix { coefficients: m }
}
```

### SINR-Adjusted Sensing

```rust
/// Compute effective sensed intensity accounting for cross-kind interference.
///
/// Returns 0.0 if SINR falls below min_sinr (signal is undetectable).
pub fn sinr_adjusted_intensity(
    target_kind_idx: usize,
    target_intensity: f64,
    active_intensities: &[f64],
    matrix: &InterferenceMatrix,
    noise_floor: f64,       // Default: 0.01
    min_sinr: f64,          // Default: 1.0 (0 dB)
) -> f64 {
    let interference: f64 = active_intensities.iter()
        .enumerate()
        .filter(|&(j, _)| j != target_kind_idx)
        .map(|(j, &intensity)| {
            matrix.coefficients[j][target_kind_idx] * intensity
        })
        .sum();

    let sinr = target_intensity / (interference + noise_floor);
    if sinr < min_sinr { 0.0 }
    else { target_intensity * (sinr / (1.0 + sinr)) }
}
```

---

## Anti-Saturation Mechanisms

```rust
/// Anti-saturation configuration for the pheromone field.
pub struct AntiSaturationConfig {
    /// Soft threshold: above this, low-intensity pheromones decay 2x faster.
    /// Default: 500.
    pub soft_threshold: usize,

    /// Hard threshold: above this, only pheromones with
    /// intensity > hard_min_intensity are retained. Default: 2000.
    pub hard_threshold: usize,

    /// Minimum intensity to survive hard threshold GC. Default: 0.1.
    pub hard_min_intensity: f64,

    /// Maximum pheromones per kind per scope. Default: 100.
    pub max_per_kind_per_scope: usize,
}
```

---

## Context Enrichment Flow

Digital pheromones are integrated into Roko's context assembly at the
Composer layer:

```
Agent receives task assignment
    |
Composer queries Substrate for ambient pheromones
    |
Composer subscribes to mesh.pheromone.deposited Pulses for freshness
    |
Scorer rates each pheromone by intensity x relevance
    |
Router selects top-K pheromones (default K=5)
    |
Composer formats pheromone summary into system prompt:
    "## Ambient Signals
     - [THREAT 0.73] Regression in gate pipeline (scorer NaN handling)
     - [OPPORTUNITY 0.91] New API endpoint ready for integration
     - [WISDOM 0.95] NaN scores should be clamped to 0.0"
    |
Agent processes task with awareness of ambient signals
```

This enrichment happens automatically. The agent does not explicitly request
pheromone information -- it is part of the environment.

---

## Implementation Status

The pheromone system's Substrate/Bus architecture is wired: `roko-core`
defines `PheromoneKind`, `PheromoneScope`, decay, and confirmation;
`roko-fs` `FileSubstrate` persists pheromone Signals; the Bus announces
deposits; and the Composer enriches agent context with ambient pheromone
summaries. Bucketed decay, SINR-adjusted sensing, and the full interference
matrix remain target design.

---

## References

- [Dorigo, Maniezzo & Colorni 1996] Ant Colony Optimization, *IEEE SMC-B*
- [Nealson, Platt & Hastings 1970] Quorum sensing, *J. Bacteriology*
- [Parunak, Brueckner & Sauter 2005] Digital pheromones, *E4MAS*
- [Tse & Viswanath 2005] *Fundamentals of Wireless Communication*, Cambridge
- [Wilson 1971] *The Insect Societies*, Belknap Press
