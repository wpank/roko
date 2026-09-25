# Depth: 7-Domain EMA Reputation

> Full derivation of the Exponential Moving Average reputation scoring
> algorithm: adaptive alpha, 30-day half-life decay, feedback normalization,
> and convergence behavior.

**Parent chapter:** [37-SHARED-ECONOMY](../../37-SHARED-ECONOMY.md) Section 1
**Source:** `crates/roko-chain/src/reputation_registry.rs`

---

## EMA Update Formula

When feedback `F` arrives for agent _i_ in domain _d_:

```
R_new = alpha * F + (1 - alpha) * R_old
```

This is the standard exponential moving average. The weight `alpha` determines
how much a new observation moves the score:
- `alpha = 1.0` would make the score equal to the latest observation (no memory)
- `alpha = 0.0` would make the score immovable (infinite memory)
- The adaptive alpha (0.04 - 0.30) provides a tunable compromise

### Mathematical Properties

The EMA has several properties that make it suitable for reputation:

1. **Bounded output:** If `F` is in [0, 1] and `R_old` is in [0, 1], then
   `R_new` is guaranteed to be in [0, 1].

2. **Exponential forgetting:** The influence of observation _k_ steps ago on
   the current score is `alpha * (1 - alpha)^k`. After enough observations,
   old scores have negligible influence.

3. **Steady-state convergence:** If an agent consistently delivers quality `q`,
   the score converges to `q` regardless of the starting point.

4. **No storage overhead:** Only the current score, job count, and timestamp
   are stored -- not the full history.

---

## Adaptive Alpha Derivation

The learning rate varies with experience to balance responsiveness and stability:

```rust
fn adaptive_alpha(job_count: u64) -> f64 {
    match job_count {
        0..=10   => 0.30,  // First 10 jobs: high sensitivity
        11..=50  => 0.15,  // Building track record
        51..=200 => 0.08,  // Established
        _        => 0.04,  // Veteran: very stable
    }
}
```

### Why These Values

**alpha = 0.30 (0-10 jobs):** A single observation moves the score by up to
30% of the distance between old score and new observation. This allows rapid
convergence to an agent's true quality level within the first few jobs.

After 10 observations at quality `q` starting from neutral (0.5):
```
R_10 = 0.5 * (1 - 0.30)^10 + q * (1 - (1 - 0.30)^10)
     = 0.5 * 0.028 + q * 0.972
     ≈ q  (97.2% converged)
```

**alpha = 0.04 (200+ jobs):** A veteran agent's score moves by only 4% per
observation. To move a veteran's score from 0.80 to 0.50 would require
approximately:
```
0.80 * (1 - 0.04)^n + 0.20 * (1 - (1 - 0.04)^n) = 0.50
Solving: n ≈ 22 consecutive bad jobs
```

This means it takes ~22 consecutive low-quality jobs (F=0.2) to drop a
veteran from 0.80 to 0.50. A single bad day cannot destroy years of
good work.

### Worked Examples

**New agent (5 jobs, alpha=0.30):**
- Starting at 0.80, one bad job (F=0.2):
  `0.30 * 0.2 + 0.70 * 0.80 = 0.06 + 0.56 = 0.62` (drop of 0.18)

**Established agent (100 jobs, alpha=0.08):**
- Starting at 0.80, one bad job (F=0.2):
  `0.08 * 0.2 + 0.92 * 0.80 = 0.016 + 0.736 = 0.752` (drop of 0.048)

**Veteran agent (500 jobs, alpha=0.04):**
- Starting at 0.80, one bad job (F=0.2):
  `0.04 * 0.2 + 0.96 * 0.80 = 0.008 + 0.768 = 0.776` (drop of 0.024)

---

## 30-Day Half-Life Decay

Reputation scores decay toward neutral (0.5) when an agent is inactive:

```rust
fn apply_decay(score: f64, elapsed_secs: f64) -> f64 {
    let neutral = 0.5;
    let half_life_secs = 30.0 * 24.0 * 3600.0;  // 2,592,000 seconds
    let decay_factor = 0.5_f64.powf(elapsed_secs / half_life_secs);
    neutral + (score - neutral) * decay_factor
}
```

### Convergence Analysis

The decay is symmetric: scores above 0.5 decay downward, scores below 0.5
recover upward. The neutral point 0.5 is the fixed point.

| Elapsed | Decay Factor | R=0.90 | R=0.30 | R=0.10 |
|---|---|---|---|---|
| 0 days | 1.000 | 0.900 | 0.300 | 0.100 |
| 15 days | 0.707 | 0.783 | 0.359 | 0.218 |
| 30 days | 0.500 | 0.700 | 0.400 | 0.300 |
| 60 days | 0.250 | 0.600 | 0.450 | 0.400 |
| 90 days | 0.125 | 0.550 | 0.475 | 0.450 |
| 180 days | 0.016 | 0.506 | 0.497 | 0.494 |
| 365 days | 0.0002 | 0.500 | 0.500 | 0.500 |

After one year of inactivity, any starting score converges to approximately
0.500 (within floating-point precision).

### Design Rationale

The half-life is applied on read, not as a background tick. This is important:
- No background process needed
- Exact decay calculation at query time
- No accumulation of floating-point drift from repeated small updates

---

## Feedback Score Normalization

Different feedback sources produce different score ranges. All are normalized
to [0.0, 1.0] before being fed to the EMA:

### Gate Results

```rust
fn gates_to_feedback(gate_results: &[GateResult]) -> f64 {
    let passed = gate_results.iter().filter(|g| g.passed).count();
    let total = gate_results.len();
    if total == 0 { return 0.5; }

    let gate_score = passed as f64 / total as f64;
    let weighted_score = gate_results.iter()
        .map(|g| if g.passed { g.weight } else { 0.0 })
        .sum::<f64>()
        / gate_results.iter().map(|g| g.weight).sum::<f64>();

    weighted_score * 0.7 + gate_score * 0.3
}
```

The 70/30 split means gate importance weights dominate but raw pass rate
provides a floor. Passing all gates produces F=1.0. Failing all gates
produces F=0.0. Passing only low-weight gates produces a low score.

### Peer Review

| Rating | Score |
|---|---|
| Excellent (5/5) | 1.0 |
| Good (4/5) | 0.8 |
| Adequate (3/5) | 0.6 |
| Poor (2/5) | 0.3 |
| Failure (1/5) | 0.1 |

Note the asymmetry: the gap between Poor and Failure (0.2) is larger than the
gap between Excellent and Good (0.2), and Adequate is above neutral (0.5).
This reflects the observation that most completed work is at least adequate;
truly poor work is a stronger negative signal.

### Feedback Weight Dilution

When an agent is under collusion dilution, their feedback as a rater carries
reduced weight:

```
effective_alpha = adaptive_alpha(job_count) * rater_feedback_weight
```

If the rater has 50% dilution, the effective alpha is halved. Multiple
dilutions stack multiplicatively.

---

## Connection to C-Factor

The network-level aggregation of individual reputation scores connects to the
C-Factor (Woolley et al., 2010):

```
domain_health(d) = mean(R_i for all active agents in domain d)
network_health = mean(domain_health(d) for all domains)
```

The 1/sqrt(N*t) collective calibration scaling cited in documentation is an
upper bound under idealized independence assumptions. Real performance depends
on error correlation, information flow, and knowledge sharing effectiveness.
