# Depth: Pheromone Kinds

> Parent: [16-COORDINATION](../../16-COORDINATION.md) -- Section 2

---

## Three-Tier Taxonomy

Every digital pheromone in Roko carries a `PheromoneKind` that determines its
semantic meaning, default decay profile, and the behavioral response it
triggers. The kind system is organized into three tiers:

1. **Universal kinds** (3): Present in every domain, every agent, every scope
2. **Domain-specific kinds** (4): Common across domains but with
   domain-dependent interpretation
3. **Custom kinds** (unbounded): User-defined via `Custom(String)`

This design balances standardization (universal kinds ensure a common signal
vocabulary) with extensibility (custom kinds allow domain-specific
coordination without modifying the core type system).

The three-tier structure is inspired by the hierarchy of pheromone types in
social insects [Wilson, E.O. *The Insect Societies*. Belknap Press, 1971]:

- Primer pheromones (long-term physiological changes) -> Wisdom
- Releaser pheromones (immediate behavioral responses) -> Threat, Opportunity
- Informational pheromones (contextual signals) -> Alpha, Pattern, Anomaly

---

## The PheromoneKind Enum

```rust
pub enum PheromoneKind {
    // -- Universal Kinds --
    Threat,         // 2h half-life. Alarm pheromone analog.
    Opportunity,    // 4h half-life. Recruitment pheromone analog.
    Wisdom,         // 24h half-life. Established trail analog.

    // -- Domain-Specific Kinds --
    Alpha,          // 1h half-life. Ephemeral first-mover edge.
    Pattern,        // 12h half-life. Recurring structure detected.
    Anomaly,        // 6h half-life. Deviation from expected behavior.
    Consensus,      // 48h half-life. Collective agreement.

    // -- Custom Kinds --
    Custom(String), // User-defined, domain-specific signals.
}
```

---

## Decay Model

Every pheromone kind decays according to an exponential half-life model:

```
intensity(t) = initial_intensity x 2^(-t / half_life)
```

Where `t` is elapsed time since deposit (not since last read). Decay is
computed lazily: the stored intensity is the value at deposit time, and any
read computes current intensity from the deposit timestamp.

```rust
/// Base-2 exponential decay. Half-life has exact intuitive meaning:
/// after exactly half_life seconds, intensity is exactly 50%.
pub fn current_intensity(
    initial_intensity: f64,
    half_life: Duration,
    elapsed: Duration,
) -> f64 {
    if half_life.is_zero() { return 0.0; }
    let exponent = -(elapsed.as_secs_f64() / half_life.as_secs_f64());
    initial_intensity * 2.0_f64.powf(exponent)
}

pub fn is_evaporated(intensity: f64, threshold: f64) -> bool {
    intensity < threshold   // Default threshold: 0.01
}
```

### Confirmation Extension

When another agent confirms a pheromone, the effective half-life extends.
The extension formula for standard (non-Alpha) kinds uses logarithmic scaling
for diminishing returns:

```
half_life_extended = half_life_base x (1 + 0.15 x ln(1 + confirmations))
```

| Confirmations | Multiplier | Effective half-life (base = 12h) |
|---------------|-----------|--------------------------------|
| 0 | 1.00 | 12.0h |
| 1 | 1.10 | 13.2h |
| 3 | 1.21 | 14.5h |
| 5 | 1.27 | 15.2h |
| 10 | 1.36 | 16.3h |

The `0.15` coefficient ensures 10 confirmations extend half-life by ~36%,
keeping even heavily-confirmed pheromones mortal.

```rust
pub fn confirmed_half_life(
    base_half_life: Duration,
    confirmations: u32,
) -> Duration {
    let multiplier = 1.0 + 0.15 * (1.0 + confirmations as f64).ln();
    Duration::from_secs_f64(base_half_life.as_secs_f64() * multiplier)
}
```

---

## Universal Kinds in Detail

### Threat

The alarm signal. Triggers immediate attention and prioritized response.

| Property | Value |
|----------|-------|
| Default half-life | 2 hours |
| Default initial intensity | 1.0 |
| Agent response | Stop current task, investigate, remediate |
| Confirmation threshold | 2 (confirmed -> high-priority) |
| Escalation | If intensity > 0.8 and confirmations > 3, escalate to broader scope |

**Intensity scaling**:

| Intensity | Severity | Example |
|-----------|----------|---------|
| 0.1-0.3 | Low | Minor style violation |
| 0.4-0.6 | Medium | Test flakiness, moderate performance regression |
| 0.7-0.8 | High | Test failure, security vulnerability in dev |
| 0.9-1.0 | Critical | Build failure, production security vulnerability |

**Gate interaction**: Ambient Threat intensity at Mesh scope influences
adaptive gate thresholds. High Threat -> thresholds tighten (collective
immune response).

### Opportunity

Recruitment pheromone analog. Recruits agents toward productive work.

| Property | Value |
|----------|-------|
| Default half-life | 4 hours |
| Default initial intensity | 0.8 (lower than Threat) |
| Agent response | Evaluate, add to task queue if aligned |
| Confirmation threshold | 1 |

### Wisdom

Validated, durable knowledge. Insights tested and confirmed through
operational experience.

| Property | Value |
|----------|-------|
| Default half-life | 24 hours |
| Default initial intensity | 0.9 |
| Agent response | Integrate into knowledge base, apply to work |
| Confirmation threshold | 3 (requires strong collective validation) |
| Promotion | 5+ confirmations may promote to permanent Signal |

**Creation pathway**: Wisdom typically emerges through a pipeline:

```
Agent observes pattern -> deposits Pattern pheromone
    |  (confirmed by multiple agents)
Agent validates through testing
    |
Agent deposits Wisdom with Pattern as parent
    |  (confirmed by others)
At 5+ confirmations, may promote to permanent Signal
```

---

## Domain-Specific Kinds in Detail

### Alpha

The most ephemeral signal. Named for the financial concept of "alpha" --
a temporary edge that disappears as more participants discover it.

| Property | Value |
|----------|-------|
| Default half-life | 1 hour |
| Default initial intensity | 1.0 |
| Agent response | Act immediately or discard |
| Confirmation effect | **Paradoxical**: confirmation *reduces* value |

**Alpha paradox**: Unlike other kinds where confirmation increases value,
confirmation of Alpha indicates the edge is eroding:

```
tau_effective(Alpha) = tau_base x max(0.5, 1 - confirmations x 0.2)
```

| Confirmations | Multiplier | Effective half-life (base = 1h) |
|---------------|-----------|-------------------------------|
| 0 | 1.0 | 60 min |
| 1 | 0.8 | 48 min |
| 2 | 0.6 | 36 min |
| 3 | 0.4 | 24 min |
| 5+ | 0.5 (floor) | 30 min |

```rust
pub fn alpha_effective_half_life(
    base_half_life: Duration,
    confirmations: u32,
) -> Duration {
    let multiplier = (1.0 - confirmations as f64 * 0.2).max(0.5);
    Duration::from_secs_f64(base_half_life.as_secs_f64() * multiplier)
}
```

### Pattern

Signals that an agent has detected a recurring structure or regularity.

| Property | Value |
|----------|-------|
| Default half-life | 12 hours |
| Default initial intensity | 0.7 |
| Confirmation threshold | 2 |

### Anomaly

Flags deviations from expected behavior. Unlike Threat (known danger),
Anomaly signals the unknown -- something that warrants investigation.

| Property | Value |
|----------|-------|
| Default half-life | 6 hours |
| Default initial intensity | 0.8 |
| Escalation | If investigation confirms danger -> re-deposit as Threat |

**Triage flow**: Investigate -> Classify (Threat/Opportunity/noise) ->
Re-deposit or let decay.

### Consensus

Encodes collective agreement. The most persistent domain-specific kind.

| Property | Value |
|----------|-------|
| Default half-life | 48 hours |
| Default initial intensity | 0.9 |
| Creation | Usually via confirmation cascade, not direct deposit |

---

## Kind Interactions

### Threat Suppression

`Threat` at high intensity (> 0.7) suppresses `Opportunity` in the same scope.
Agents in threat-response mode should not be distracted.

### Promotion Cascade

```
Pattern -[3+ confirms, age > 50% half-life]-> Wisdom
Wisdom  -[4+ confirms]-----------------------> Consensus
Consensus -[5+ confirms]--------------------> Permanent Signal (optional)
```

```rust
pub struct PromotionConfig {
    pub pattern_to_wisdom_confirmations: u32,        // Default: 3
    pub pattern_to_wisdom_min_age_fraction: f64,     // Default: 0.5
    pub wisdom_to_consensus_confirmations: u32,      // Default: 4
    pub consensus_to_engram_confirmations: u32,      // Default: 5
    pub auto_promote: bool,                          // Default: true
}
```

The `PheromonePromoter` runs as a background task inside the Curator cycle
(every 50 ticks). Promotion is idempotent: if a Wisdom with the same parent
hash already exists, the duplicate is skipped.

### Anomaly Resolution

`Anomaly` pheromones resolve into either `Threat` (danger confirmed),
`Opportunity` (hidden value), or natural decay (noise).

### Consensus Stability

`Consensus` resists contradiction. To contradict a Consensus pheromone, an
agent must deposit a `Threat` of equal or greater intensity with explicit
evidence.

---

## Custom Kinds

The `Custom(String)` variant enables domain-specific pheromone kinds.

### Validation

```rust
pub fn validate_custom_kind(id: &str) -> Result<(), String> {
    // ASCII alphanumeric + underscores only
    // 1-64 characters
    // Must not start with '_' (reserved)
    // Must not collide with built-in kind names
}
```

### Configuration

```toml
[pheromone.custom_kinds.code_coverage_gap]
half_life_secs = 28800  # 8 hours
description = "Code coverage below threshold in a module"

[pheromone.custom_kinds.model_drift]
half_life_secs = 7200   # 2 hours
description = "ML model predictions diverging from outcomes"
```

### Scope Isolation

Custom kinds are scoped by `(domain, kind_id)`. Two agents in different
domains can deposit custom pheromones with the same string identifier without
interference.

---

## Pheromone-Driven Task Allocation

Pheromone kinds drive emergent task allocation via response thresholds, inspired
by division of labor in social insects [Bonabeau, E., Theraulaz, G. &
Deneubourg, J.-L. "Fixed Response Thresholds." *Bull. Math. Biology*,
60(4):753-807, 1998].

### Response Threshold Model

```
P(respond to kind k) = I_k^n / (I_k^n + theta_k^n)
```

Where `I_k` = intensity, `theta_k` = agent's threshold, `n` = Hill
coefficient (default: 2).

Thresholds adapt over time:
- Successfully responding to a kind **lowers** its threshold (reinforcement)
- Ignoring a kind **raises** its threshold (habituation)

This produces emergent division of labor without explicit assignment.

---

## Summary Table

| Kind | Half-Life | Intensity | Confirmation Effect | Biological Analog |
|------|-----------|-----------|--------------------|--------------------|
| Threat | 2h | 1.0 | Extends (standard) | Alarm pheromone |
| Opportunity | 4h | 0.8 | Extends (standard) | Recruitment pheromone |
| Wisdom | 24h | 0.9 | Extends; promotes at 4+ | Trail pheromone |
| Alpha | 1h | 1.0 | **Reduces** (paradox) | Ephemeral scent mark |
| Pattern | 12h | 0.7 | Extends; promotes at 3+ | Territorial marking |
| Anomaly | 6h | 0.8 | Extends (standard) | Novel scent detection |
| Consensus | 48h | 0.9 | Extends; resists contradiction | Colony odor |
| Custom | User-defined | User-defined | Standard | Domain-specific |

---

## References

- [Bonabeau, Theraulaz & Deneubourg 1998] Fixed Response Thresholds, *Bull. Math. Biology*
- [Nealson, Platt & Hastings 1970] Quorum sensing, *J. Bacteriology*
- [Parunak, Brueckner & Sauter 2005] Digital pheromones, *E4MAS*
- [Wilson 1971] *The Insect Societies*, Belknap Press
