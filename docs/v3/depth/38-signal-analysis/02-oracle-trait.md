# The Oracle Trait -- Universal Prediction Interface

> **Parent:** [38-SIGNAL-ANALYSIS](../../38-SIGNAL-ANALYSIS.md) Section 2

---

## Full Trait Signature

```rust
pub trait Oracle: Send + Sync {
    /// Make a prediction about future state.
    ///
    /// The query encodes WHAT to predict. The context encodes the agent's
    /// current cognitive state (PAD vector, active knowledge, recent history).
    async fn predict(
        &self,
        query: &OracleQuery,
        ctx: &Context,
    ) -> Result<Prediction>;

    /// Evaluate a past prediction against the actual outcome.
    ///
    /// The outcome is an Engram produced by external verification.
    /// The returned PredictionAccuracy drives feedback into Router,
    /// Daimon, Neuro, and Gate subsystems.
    async fn evaluate(
        &self,
        prediction: &Prediction,
        outcome: &Engram,
    ) -> Result<PredictionAccuracy>;
}
```

### Design rationale

The Oracle trait deliberately does **not** include:

- `subscribe()` -- real-time streams are handled by `Substrate.query()` with watch semantics
- `calibrate()` -- calibration is a `Policy` concern; `CalibrationTracker` wraps any Oracle
- `batch_predict()` -- batch semantics are provided by the caller; implementations may batch internally

This follows Ousterhout's "deep module" principle (2018): the interface is narrow (2 methods), but the implementation depth is substantial.

## Full Type Definitions

### OracleQuery

```rust
pub struct OracleQuery {
    pub id: ContentHash,           // BLAKE3 content-addressed
    pub domain: OracleDomain,
    pub payload: QueryPayload,
    pub horizon: Duration,
    pub min_confidence: f64,       // below this, return Err(LowConfidence)
    pub tags: BTreeMap<String, String>,
    pub created_at_ms: i64,
}

#[non_exhaustive]
pub enum OracleDomain {
    Coding,
    Research,
    Operations,
    Custom(String),
}

pub enum QueryPayload {
    Coding(CodingQueryPayload),
    Research(ResearchQueryPayload),
    Operations(OperationsQueryPayload),
    Custom(serde_json::Value),
}

pub struct CodingQueryPayload {
    pub scope: CodingScope,
    pub metric: CodingMetric,
    pub change_context: Option<ChangeContext>,
}

pub struct ResearchQueryPayload {
    pub source: SourceReference,
    pub metric: ResearchMetric,
    pub claim_context: Option<String>,
}
```

### Prediction

```rust
pub struct Prediction {
    pub id: ContentHash,
    pub query_id: ContentHash,
    pub value: PredictedValue,
    pub confidence: f64,
    pub interval: Option<PredictionInterval>,
    pub created_at_ms: i64,
    pub resolve_by_ms: i64,
    pub provenance: PredictionProvenance,
    pub lineage: Vec<ContentHash>,
    pub outcome: Option<PredictionOutcome>,
}

pub enum PredictedValue {
    Numeric(f64),
    Probability(f64),
    Ordinal { label: String, rank: u32 },
    Binary(bool),
    Compound(BTreeMap<String, PredictedValue>),
}

pub struct PredictionInterval {
    pub lower: f64,
    pub upper: f64,
    pub coverage: f64,   // e.g. 0.90 for 90% coverage
}

pub struct PredictionOutcome {
    pub actual: PredictedValue,
    pub evidence_id: ContentHash,
    pub resolved_at_ms: i64,
    pub accuracy: PredictionAccuracy,
}
```

### PredictionAccuracy

```rust
pub struct PredictionAccuracy {
    pub prediction_id: ContentHash,
    pub outcome_id: ContentHash,
    pub accuracy: f64,          // [0.0, 1.0], 1.0 = perfect
    pub residual: f64,          // predicted - actual; positive = overestimate
    pub interval_hit: Option<bool>,
    pub resolution_lag_ms: i64,
    pub domain: OracleDomain,
    pub category: String,
}
```

## Integration with Core Traits

### Substrate integration

Predictions are persisted as Engrams with `kind: Kind::Prediction`:

```rust
let engram = Engram::builder()
    .kind(Kind::Prediction)
    .body(Body::Json(serde_json::to_value(&prediction)?))
    .tag("domain", prediction.provenance.domain.as_str())
    .score(Score {
        confidence: prediction.confidence,
        novelty: 0.5,
        utility: 0.0,  // accumulates after resolution
        ..Default::default()
    })
    .lineage(prediction.lineage.clone())
    .build();
substrate.put(engram).await?;
```

### Scorer integration -- PredictiveScorer

```rust
pub struct PredictiveScorer {
    calibration: Arc<CalibrationTracker>,
}

impl Scorer for PredictiveScorer {
    fn score(&self, engram: &Engram) -> Score {
        let model = engram.provenance.model_id();
        let category = engram.tag("task_category").unwrap_or("unknown");
        let calibration = self.calibration.get_accuracy(model, category);
        let mut score = engram.score.clone();
        score.confidence *= calibration.recent_accuracy;
        score
    }
}
```

### Router integration

Prediction accuracy feeds into `Router.feedback()`:

```rust
let accuracy = oracle.evaluate(&prediction, &outcome).await?;
router.feedback(&prediction.provenance.model_id, accuracy.accuracy)?;
```

### Gate integration

Prediction residuals calibrate adaptive gate thresholds via EMA.

### Composer integration -- EFE bidding

Oracle predictions participate in the VCG attention auction (Vickrey 1961,
Clarke 1971, Groves 1973):

```rust
let efe = pragmatic_value + epistemic_value - ambiguity;
let bid = efe * urgency * affect_weight;
composer.bid("oracle_predictions", bid, prediction_context);
```

## PredictionStore

```rust
pub struct PredictionStore {
    substrate: Arc<dyn Substrate>,
    pending: DashMap<ContentHash, Prediction>,
    resolved: DashMap<ContentHash, PredictionOutcome>,
}

impl PredictionStore {
    pub async fn register(&self, prediction: Prediction) -> Result<()>;
    pub async fn pending_resolutions(&self) -> Vec<Prediction>;
    pub async fn resolve(
        &self,
        prediction_id: &ContentHash,
        outcome: &Engram,
        oracle: &dyn Oracle,
    ) -> Result<PredictionAccuracy>;
    pub async fn accuracy_stats(
        &self,
        domain: &OracleDomain,
        category: &str,
    ) -> AccuracyStats;
}
```

## Oracle Composition

### Conformal prediction wrapper

```rust
pub struct ConformalOracle {
    base_oracle: Arc<dyn Oracle>,
    calibration_scores: Vec<f64>,
    alpha: f64,   // target miscoverage rate
}

impl ConformalOracle {
    pub async fn predict_set(
        &self,
        query: &OracleQuery,
        ctx: &Context,
    ) -> Result<PredictionSet> {
        let base = self.base_oracle.predict(query, ctx).await?;
        let n = self.calibration_scores.len();
        let quantile_idx = ((1.0 - self.alpha) * (n + 1) as f64).ceil() as usize;
        let threshold = self.calibration_scores
            .get(quantile_idx.min(n - 1))
            .copied()
            .unwrap_or(f64::MAX);

        Ok(PredictionSet {
            point_prediction: base,
            coverage_guarantee: 1.0 - self.alpha,
            prediction_interval: PredictionInterval {
                lower: base.value.as_numeric()? - threshold,
                upper: base.value.as_numeric()? + threshold,
                coverage: 1.0 - self.alpha,
            },
            n_calibration: n,
        })
    }
}
```

P(y_new in C(x_new)) >= 1 - alpha for any distribution, any sample size.
No distributional assumptions required -- only exchangeability (Vovk et al.,
2005; Angelopoulos & Bates, 2023, arXiv:2107.07511).

### Brier score decomposition

```rust
pub fn brier_decomposition(
    predictions: &[(f64, bool)],
    n_bins: usize,
) -> CalibrationQuality {
    // Brier = REL - RES + UNC
    // REL: (1/N) SUM n_k (f_bar_k - o_bar_k)^2
    // RES: (1/N) SUM n_k (o_bar_k - o_bar)^2
    // UNC: o_bar(1 - o_bar)
    // ...
}

pub struct CalibrationQuality {
    pub reliability: f64,      // lower is better (0 = perfect calibration)
    pub resolution: f64,       // higher is better
    pub uncertainty: f64,      // base rate uncertainty (irreducible)
    pub brier_score: f64,      // REL - RES + UNC
    pub ece: f64,              // Expected Calibration Error (Naeini et al., 2015)
    pub mce: f64,              // Maximum Calibration Error
    pub sharpness: f64,
}
```

### Composition strategies

```rust
pub enum CompositionStrategy {
    WeightedEnsemble,
    Conformal { target_coverage: f64 },
    IsotonicRecalibration,
    PlattScaling,
    TemperatureScaling { temperature: f64 },
}
```

## Implementing a Custom Oracle

```rust
pub struct DeploymentOracle {
    history: Arc<PredictionStore>,
    corrector: Arc<ResidualCorrector>,
}

#[async_trait]
impl Oracle for DeploymentOracle {
    async fn predict(&self, query: &OracleQuery, ctx: &Context) -> Result<Prediction> {
        let payload = query.payload.as_operations()?;
        let features = self.extract_features(payload, ctx).await?;
        let mut prediction = Prediction {
            value: PredictedValue::Probability(features.base_success_rate),
            confidence: features.sample_confidence,
            interval: Some(PredictionInterval {
                lower: features.base_success_rate - features.std_dev,
                upper: (features.base_success_rate + features.std_dev).min(1.0),
                coverage: 0.68,
            }),
            // ...
        };
        self.corrector.correct(&mut prediction);
        Ok(prediction)
    }

    async fn evaluate(&self, prediction: &Prediction, outcome: &Engram) -> Result<PredictionAccuracy> {
        let actual_success = outcome.tag("deployment_success")
            .map(|v| v == "true")
            .unwrap_or(false);
        let predicted_prob = prediction.value.as_probability()?;
        let actual_value = if actual_success { 1.0 } else { 0.0 };

        let accuracy = PredictionAccuracy {
            accuracy: 1.0 - (predicted_prob - actual_value).abs(),
            residual: predicted_prob - actual_value,
            // ...
        };
        self.corrector.update(&prediction.provenance.model_id, "deployment", accuracy.residual);
        Ok(accuracy)
    }
}
```

## Test Criteria

- **Conformal coverage**: Over 1000 test samples, empirical coverage >= (1-alpha) - 0.01.
- **Brier decomposition additivity**: REL - RES + UNC = Brier score within f64 epsilon.
- **Composition monotonicity**: Adding a well-calibrated oracle does not increase Brier score.
- **Temperature scaling idempotence**: Applying T=1.0 twice produces identical predictions.
