# Adversarial Signal Robustness

> **Parent:** [38-SIGNAL-ANALYSIS](../../38-SIGNAL-ANALYSIS.md) Section 10

---

## Adversarial Signal Decomposition

```rust
pub struct AdversarialDecomposer {
    robust_stats: RobustStatistics,
    causal_model: Arc<StructuralCausalModel>,
    attack_prototypes: Vec<HdcVector>,
    cross_verifier: CrossSourceVerifier,
}

pub struct SignalDecomposition {
    pub genuine: f64,
    pub adversarial: f64,
    pub noise: f64,
    pub confidence: f64,
    pub attack_match: Option<AttackPatternMatch>,
}
```

## HDC Prototype Matching

```rust
pub struct PrototypeMatcher {
    prototypes: Vec<PrototypeEntry>,
    threshold: f64,   // typically 0.6
}

pub struct PrototypeEntry {
    pub vector: HdcVector,
    pub name: String,
    pub domain: OracleDomain,
    pub severity: f64,
    pub response: AdversarialResponse,
}

pub enum AdversarialResponse {
    WidenIntervals(f64),
    SuppressAction(Duration),
    EscalateToT2,
    EmitWarning(String),
}

impl PrototypeMatcher {
    pub fn match_prototypes(&self, observation: &HdcVector) -> Vec<(f64, &PrototypeEntry)> {
        // ~10ns per comparison. 1,000 prototypes: ~10us total.
        self.prototypes.iter()
            .filter_map(|proto| {
                let sim = observation.hamming_similarity(&proto.vector);
                if sim > self.threshold { Some((sim, proto)) } else { None }
            })
            .sorted_by(|a, b| b.0.partial_cmp(&a.0).unwrap())
            .collect()
    }
}
```

### Coding domain prototypes

```rust
pub fn coding_attack_prototypes(codebook: &CodingCodebook) -> Vec<PrototypeEntry> {
    vec![
        PrototypeEntry {
            name: "dependency_confusion".into(),
            severity: 0.9,
            response: AdversarialResponse::EscalateToT2,
            // ...
        },
        PrototypeEntry {
            name: "typosquatting".into(),
            severity: 0.7,
            response: AdversarialResponse::EmitWarning("Possible typosquatting".into()),
            // ...
        },
        PrototypeEntry {
            name: "malicious_build_script".into(),
            severity: 0.95,
            response: AdversarialResponse::SuppressAction(Duration::from_secs(300)),
            // ...
        },
        PrototypeEntry {
            name: "test_suite_poisoning".into(),
            severity: 0.8,
            response: AdversarialResponse::EscalateToT2,
            // ...
        },
    ]
}
```

### Research domain prototypes

```rust
pub fn research_attack_prototypes(codebook: &ResearchCodebook) -> Vec<PrototypeEntry> {
    vec![
        PrototypeEntry { name: "p_value_clustering".into(), severity: 0.7, /* ... */ },
        PrototypeEntry { name: "selective_reporting".into(), severity: 0.6, /* ... */ },
        PrototypeEntry { name: "data_fabrication".into(), severity: 0.95, /* ... */ },
        PrototypeEntry { name: "citation_ring".into(), severity: 0.5, /* ... */ },
    ]
}
```

## Robust Statistics

### Breakdown-point-optimal estimators

| Estimator | Breakdown point | Use case |
|---|---|---|
| Median/MAD | 50% | Location and scale |
| MCD (Rousseeuw, 1999) | ~50% | Multivariate outliers |
| Huber M-estimator | variable | Bounded-influence regression |

```rust
pub struct RobustStatistics {
    pub median: f64,
    pub mad: f64,           // Median Absolute Deviation
    pub mcd: Option<MinimumCovarianceDeterminant>,
}

impl RobustStatistics {
    pub fn is_outlier(&self, value: f64) -> bool {
        (value - self.median).abs() / self.mad > 3.5
    }
}
```

## Certified Robustness

### Randomized smoothing (Cohen et al., 2019)

```rust
pub struct RandomizedSmoothing {
    pub base_oracle: Arc<dyn Oracle>,
    pub sigma: f64,         // noise scale
    pub n_samples: usize,   // number of noisy samples
    pub alpha: f64,          // confidence level
}

impl RandomizedSmoothing {
    pub fn certified_prediction(&self, query: &OracleQuery, ctx: &Context) -> CertifiedPrediction {
        // 1. Sample n_samples noisy versions of the query
        // 2. Get predictions for each
        // 3. Return majority prediction with certified L2 radius
    }
}

pub struct CertifiedPrediction {
    pub prediction: Prediction,
    pub certified_radius: f64,   // L2 radius of certified robustness
    pub confidence: f64,
}
```

### Lipschitz certification

```rust
pub struct LipschitzCertifier {
    pub lipschitz_constant: f64,  // maximum prediction change per unit input change
}

impl LipschitzCertifier {
    pub fn certification_radius(&self, margin: f64) -> f64 {
        margin / self.lipschitz_constant
    }
}
```

### Interval Bound Propagation (IBP)

Propagates input intervals through the oracle to bound output ranges.
Conservative but computationally efficient.

## Red-Team Dreaming

During Delta consolidation, the Dreams subsystem runs adversarial exercises:

```
1. Generate synthetic adversarial signals using current prototypes
2. Apply perturbations: slightly modify known attack patterns
3. Feed through the oracle pipeline
4. Measure which attacks evade detection
5. Successful synthetic attacks -> new defense training data
6. Form somatic markers for newly discovered attack patterns
```

## Adversarial Adaptation

```rust
pub struct AdversarialAdaptation {
    pub known_prototypes: Vec<PrototypeEntry>,
    pub prototype_age: HashMap<String, Duration>,
    pub detection_rates: HashMap<String, f64>,
}

impl AdversarialAdaptation {
    pub fn update_after_resolution(&mut self, attack_name: &str, detected: bool) {
        let rate = self.detection_rates.entry(attack_name.into()).or_insert(0.5);
        *rate = *rate * 0.9 + if detected { 0.1 } else { 0.0 };
    }
}
```

## Consensus robustness

When multiple oracles predict on the same query, their predictions are
compared for consensus:

```rust
pub fn consensus_check(predictions: &[Prediction]) -> ConsensusResult {
    let values: Vec<f64> = predictions.iter()
        .filter_map(|p| p.value.as_numeric().ok())
        .collect();
    let median = robust_median(&values);
    let mad = robust_mad(&values, median);
    let outliers: Vec<_> = predictions.iter()
        .filter(|p| {
            let v = p.value.as_numeric().unwrap_or(0.0);
            (v - median).abs() / mad > 3.5
        })
        .collect();
    ConsensusResult { median, mad, outlier_predictions: outliers }
}
```

## Test Criteria

- **Prototype detection**: known attack patterns are detected with >95% recall at threshold 0.6
- **Robust statistics**: median/MAD estimates are within 10% of true values with 40% contamination
- **Certified radius**: randomized smoothing radius is tight (within 20% of empirical minimum adversarial distance)
- **Red-team generation**: synthetic attacks have >10% evasion rate (otherwise prototypes are too narrow)

## Academic Foundations

- Cohen, J. M., et al. (2019). "Certified Adversarial Robustness via Randomized Smoothing." *ICML*.
- Rousseeuw, P. J. (1999). "Fast Algorithm for MCD Estimator." *Technometrics*, 41(3).
- Huber, P. J. (1964). "Robust Estimation of a Location Parameter." *Annals of Mathematical Statistics*, 35(1).
