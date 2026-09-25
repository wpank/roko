# The 16 T0 Probes: Zero-LLM Cognitive Perception

> v3 depth file for chapter 29 (Heartbeat and Cognitive Loop).
> Source: v1/16-heartbeat/09-16-t0-probes.md.
> Parent: `docs/v3/29-HEARTBEAT.md` SS11.

---

## 1. Abstract

The 16 T0 probes are the agent's "peripheral vision" -- lightweight, deterministic
checks that run on every gamma tick (~5-15 seconds) with **zero LLM cost**. Each
probe is a pure function: `fn probe(state: &EngineState) -> f32`. No LLM inference,
no network calls for domain-agnostic probes.

The probe architecture implements FrugalGPT's (Chen et al. 2023, arXiv:2305.05176)
core insight: intelligent routing using cheap checks to determine when the expensive
model is necessary. The 16 probes are those cheap checks. They determine, with high
precision, whether the current observation is surprising enough to warrant LLM
deliberation.

8 probes are blockchain domain, 6 are coding domain, 2 are universal.

---

## 2. The Probe Trait

```rust
pub trait Probe: Send + Sync {
    fn evaluate(&self, state: &EngineState) -> f32;  // [0.0, 1.0]
    fn weight(&self) -> f32;
    fn name(&self) -> &str;
    fn domain(&self) -> ProbeDomain;
}

pub enum ProbeDomain {
    Chain,
    Coding,
    Research,
    Universal,
    Custom(String),
}
```

Probes MUST be: deterministic, fast (< 10ms), side-effect-free. Domain probes may
perform lightweight reads (RPC calls, file stats) but must not mutate state.

---

## 3. Blockchain Domain Probes (8)

### Probe 1: PriceDelta (weight: 0.15)

Detects significant price changes. Per-asset volatility-normalized thresholds.

```rust
impl Probe for PriceDeltaProbe {
    fn evaluate(&self, state: &EngineState) -> f32 {
        let max_delta = state.tracked_assets().iter()
            .map(|asset| {
                let delta = (asset.current_price - asset.last_tick_price).abs()
                    / asset.last_tick_price;
                let threshold = self.thresholds.get(&asset.id).copied().unwrap_or(0.02);
                (delta / threshold).min(1.0)
            })
            .fold(0.0f32, f32::max);
        max_delta
    }
    fn weight(&self) -> f32 { 0.15 }
}
```

### Probe 2: TvlDelta (weight: 0.10)

TVL changes across tracked protocols. 5% TVL change = maximum signal.

### Probe 3: PositionHealth (weight: 0.20)

Collateral ratios and liquidation distance. Highest-weight chain probe. < 1.2
health factor = critical (1.0). < 1.5 = warning (0.6). < 2.0 = moderate (0.2).

### Probe 4: GasSpike (weight: 0.05)

Gas price increases vs. EMA baseline. 3x baseline = maximum signal.

### Probe 5: CreditBalance (weight: 0.05)

Remaining balance. < 1 day = critical (1.0). < 7 days = warning (0.5).

### Probe 6: RSI (weight: 0.05)

14-period RSI. > 80 or < 20 = extreme (0.8). > 70 or < 30 = notable (0.4).

### Probe 7: MACD (weight: 0.05)

Momentum shifts. Crossover = 0.7. Strong divergence = 0.4.

### Probe 8: CircuitBreaker (weight: 0.10)

Exchange halts, protocol pauses, emergency shutdowns. Any active = 1.0.

---

## 4. Coding Domain Probes (6)

### Probe 9: BuildHealth (weight: 0.20)

Last compilation result and trend. Failure = 0.8. Warning(count) = count * 0.1.

### Probe 10: TestRegression (weight: 0.20)

Test pass count delta. Each failing test = 0.2.

```rust
impl Probe for TestRegressionProbe {
    fn evaluate(&self, state: &EngineState) -> f32 {
        let delta = state.test_pass_count_delta();
        if delta < 0 { ((-delta) as f32 * 0.2).min(1.0) } else { 0.0 }
    }
    fn weight(&self) -> f32 { 0.20 }
}
```

### Probe 11: ComplexityDrift (weight: 0.05)

Cyclomatic complexity moving average. 10% increase = maximum.

### Probe 12: DependencyRisk (weight: 0.10)

New vulnerability count from dependency scanning. 0 = 0.0, 1-2 = 0.4, 3-5 = 0.7,
6+ = 1.0.

### Probe 13: CoverageDelta (weight: 0.05)

Test coverage drop. 10% drop = maximum.

### Probe 14: ErrorRate (weight: 0.10)

Gate failure trend over last N tasks. > 50% = 0.8. > 30% = 0.4.

---

## 5. Universal Probes (2)

### Probe 15: WorldModelDrift (weight: 0.15)

Cosine distance between predicted and observed state vectors (Friston 2010).

```rust
impl Probe for WorldModelDriftProbe {
    fn evaluate(&self, state: &EngineState) -> f32 {
        let predicted = state.predicted_state_vector();
        let actual = state.actual_state_vector();
        cosine_distance(&predicted, &actual).clamp(0.0, 1.0)
    }
    fn weight(&self) -> f32 { 0.15 }
}
```

### Probe 16: CausalConsistency (weight: 0.10)

Lineage DAG integrity. Missing parents, hash mismatches, orphans. 0 issues = 0.0.
1-2 = 0.3. 3+ = 0.8.

---

## 6. Prediction Error Aggregation

```
prediction_error = SUM(probe_value * probe_weight)    capped at 1.0

error < 0.2  -> T0 (suppress, no LLM)     ~80% of ticks
error < 0.6  -> T1 (fast model, shallow)   ~15% of ticks
error >= 0.6 -> T2 (full model, deep)      ~5% of ticks
```

Probes compose per-domain: a chain agent registers 8 chain + 2 universal. A coding
agent registers 6 coding + 2 universal.

---

## 7. Extensibility

```rust
pub struct ProbeRegistry {
    probes: Vec<Box<dyn Probe>>,
}

impl ProbeRegistry {
    pub fn evaluate_all(&self, state: &EngineState) -> ProbeResults {
        let results: Vec<ProbeResult> = self.probes.iter()
            .map(|p| ProbeResult {
                name: p.name().to_string(),
                value: p.evaluate(state),
                weight: p.weight(),
                domain: p.domain(),
                is_anomalous: false, // set via z-score comparison
            })
            .collect();
        let aggregate = results.iter()
            .map(|r| r.value * r.weight)
            .sum::<f32>()
            .min(1.0);
        ProbeResults { results, aggregate }
    }

    pub fn register(&mut self, probe: Box<dyn Probe>) {
        self.probes.push(probe);
    }
}
```

New domains add probes by implementing `Probe` and registering at initialization.

---

## 8. Summary Table

| # | Probe | Domain | Weight | What it detects |
|---|---|---|---|---|
| 1 | PriceDelta | Chain | 0.15 | Significant price changes |
| 2 | TvlDelta | Chain | 0.10 | TVL changes across protocols |
| 3 | PositionHealth | Chain | 0.20 | Collateral ratios, liquidation distance |
| 4 | GasSpike | Chain | 0.05 | Sudden gas price increases |
| 5 | CreditBalance | Chain | 0.05 | Remaining balance |
| 6 | RSI | Chain | 0.05 | Overbought/oversold conditions |
| 7 | MACD | Chain | 0.05 | Momentum shifts |
| 8 | CircuitBreaker | Chain | 0.10 | Exchange halts, protocol pauses |
| 9 | BuildHealth | Coding | 0.20 | Last compilation result |
| 10 | TestRegression | Coding | 0.20 | Test pass count delta |
| 11 | ComplexityDrift | Coding | 0.05 | Cyclomatic complexity trend |
| 12 | DependencyRisk | Coding | 0.10 | New vulnerabilities |
| 13 | CoverageDelta | Coding | 0.05 | Test coverage drop |
| 14 | ErrorRate | Coding | 0.10 | Gate failure trend |
| 15 | WorldModelDrift | Universal | 0.15 | Predicted vs. observed state divergence |
| 16 | CausalConsistency | Universal | 0.10 | Lineage DAG integrity |

---

## 9. References

- **Chen et al. 2023** -- FrugalGPT (arXiv:2305.05176; published 2024 TMLR).
- **Friston 2010** -- "The Free-Energy Principle" (Nature Reviews Neuroscience 11(2)).
- **Kahneman 2011** -- "Thinking, Fast and Slow".
- **Sims 2003** -- "Implications of rational inattention" (Journal of Monetary
  Economics 50(3)).

---

## Cross-References

- `docs/v3/depth/29-heartbeat/dual-process-t0-t1-t2.md` -- How probes drive tiers
- `docs/v3/depth/29-heartbeat/gamma-reactive-loop.md` -- Probes in the SENSE step
- `docs/v3/depth/29-heartbeat/active-inference-compute-allocation.md` -- Theory
- `docs/v3/29-HEARTBEAT.md` -- Parent chapter
