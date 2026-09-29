# Research Oracles -- Prediction for Information Analysis

> **Parent:** [38-SIGNAL-ANALYSIS](../../38-SIGNAL-ANALYSIS.md) Section 4

---

## ResearchOracle Implementation

```rust
pub struct ResearchOracle {
    evaluator: Arc<SourceEvaluator>,
    citation_graph: Arc<CitationGraphAnalyzer>,
    contradiction_detector: Arc<ContradictionDetector>,
    prediction_store: Arc<PredictionStore>,
    corrector: Arc<ResidualCorrector>,
    calibration: Arc<CalibrationTracker>,
}

#[async_trait]
impl Oracle for ResearchOracle {
    async fn predict(&self, query: &OracleQuery, ctx: &Context) -> Result<Prediction> {
        let research_payload = query.payload.as_research()?;
        match research_payload.metric {
            ResearchMetric::Reliability => self.predict_reliability(research_payload, ctx).await,
            ResearchMetric::Completeness => self.predict_completeness(research_payload, ctx).await,
            ResearchMetric::ContradictionRisk => self.predict_contradiction(research_payload, ctx).await,
            ResearchMetric::ReplicationProbability => self.predict_replication(research_payload, ctx).await,
            ResearchMetric::CitationMomentum => self.predict_citation_momentum(research_payload, ctx).await,
        }
    }
}
```

## Source Reliability Estimation

```rust
pub struct SourceReliability {
    pub score: f64,
    pub factors: ReliabilityFactors,
    pub confidence: f64,
}

pub struct ReliabilityFactors {
    pub venue_quality: f64,           // top-tier journal = high, preprint = lower
    pub citation_momentum: f64,       // increasing = positive signal
    pub author_reliability: f64,      // historical accuracy
    pub methodology_quality: f64,     // sample size, rigor, preregistration
    pub internal_consistency: f64,    // no contradictions within source
    pub cross_source_agreement: f64,  // other sources confirm claims
}
```

## Completeness Assessment

Uses Charnov's marginal value theorem (1976) as stopping rule:

```rust
pub struct TopicCoverage {
    pub completeness: f64,
    pub covered: Vec<(String, f64)>,
    pub missing: Vec<(String, f64)>,
    pub coverage_entropy: f64,
}

impl CompletenessAssessor {
    pub fn should_continue_research(&self, topic: &str, cost_per_query: f64) -> bool {
        let marginal_gain = self.estimated_marginal_gain(self.coverage.get(topic));
        marginal_gain > cost_per_query
    }
}
```

## Contradiction Detection

```rust
pub struct Contradiction {
    pub claim_a: ContentHash,
    pub claim_b: ContentHash,
    pub claim_similarity: f64,
    pub confidence: f64,
    pub resolution: Option<ContradictionResolution>,
}
```

HDC encoding enables nanosecond detection: encode each claim as a 10,240-bit
vector, compute Hamming similarity, flag pairs with high similarity but
opposite conclusions.

## Replication Probability Estimation

```rust
pub struct ReplicationFeatures {
    pub statistical_power: f64,
    pub preregistered: bool,
    pub p_value_proximity: f64,    // distance to 0.05
    pub n_comparisons: usize,
    pub effect_size: f64,
    pub field_base_rate: f64,      // psychology ~36%, economics ~61%
}
```

Based on the Open Science Collaboration (2015): only 36% of psychology studies
replicated.

## Citation Momentum Analysis

```rust
impl CitationMomentumAnalyzer {
    pub fn compute_macd(&self, paper_id: &str) -> Option<CitationMacd> {
        // Short EMA (6 months) vs Long EMA (3 years)
        // Positive MACD -> accelerating citations -> growing influence
        // MACD crossover -> paradigm shift signal
    }
}
```

## p-Hacking Detection

```rust
pub struct PHackingAssessment {
    pub risk: f64,
    pub red_flags: Vec<PHackingRedFlag>,
    pub confidence: f64,
}

pub enum PHackingRedFlag {
    PValueClustering { count: usize, expected_by_chance: f64 },
    EffectSizeAnomaly { reported: f64, expected_range: (f64, f64) },
    MultipleComparisons { reported_tests: usize, likely_tests: usize },
    SelectiveReporting { missing_outcomes: Vec<String> },
}
```

## Verification Mechanisms

| Method | Strength | Latency | What it resolves |
|---|---|---|---|
| Cross-source agreement | Moderate | Immediate | 5 independent sources agree |
| Citation analysis | Moderate | Immediate | High-citation = more likely reliable |
| Logical consistency | Moderate | Immediate | Internal contradictions |
| Replication study | Strong | Months/years | Direct test of findings |
| Meta-analysis | Strong | Months/years | Statistical aggregation |

Research oracle predictions carry wider confidence intervals than coding
predictions. The CalibrationTracker learns this automatically.

## Academic Foundations

- Open Science Collaboration. (2015). "Estimating the reproducibility of psychological science." *Science*, 349(6251).
- Simmons, J. P., et al. (2011). "False-Positive Psychology." *Psychological Science*, 22(11).
- Ioannidis, J. P. A. (2005). "Why Most Published Research Findings Are False." *PLoS Medicine*, 2(8).
- Charnov, E. L. (1976). "Optimal foraging." *Theoretical Population Biology*, 9, 129-136.
- Kleyko, D., et al. (2022). "A Survey on Hyperdimensional Computing." *ACM Computing Surveys*, 54(6).
