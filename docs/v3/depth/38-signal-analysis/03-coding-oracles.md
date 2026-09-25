# Coding Oracles -- Signal Analysis for Software Engineering

> **Parent:** [38-SIGNAL-ANALYSIS](../../38-SIGNAL-ANALYSIS.md) Section 3

---

## CodingOracle Implementation

```rust
pub struct CodingOracle {
    workspace: Arc<WorkspaceAnalyzer>,
    metrics_cache: Arc<CodingMetricsCache>,
    vuln_scanner: Arc<VulnerabilityScanner>,
    prediction_store: Arc<PredictionStore>,
    corrector: Arc<ResidualCorrector>,
    calibration: Arc<CalibrationTracker>,
}

#[async_trait]
impl Oracle for CodingOracle {
    async fn predict(&self, query: &OracleQuery, ctx: &Context) -> Result<Prediction> {
        let coding_payload = query.payload.as_coding()?;
        match coding_payload.metric {
            CodingMetric::BuildTime => self.predict_build_time(coding_payload, ctx).await,
            CodingMetric::TestPassRate => self.predict_test_pass_rate(coding_payload, ctx).await,
            CodingMetric::ComplexityDelta => self.predict_complexity(coding_payload, ctx).await,
            CodingMetric::DependencyRisk => self.predict_dep_risk(coding_payload, ctx).await,
            CodingMetric::PerfRegression => self.predict_perf_regression(coding_payload, ctx).await,
            CodingMetric::CoverageImpact => self.predict_coverage(coding_payload, ctx).await,
        }
    }

    async fn evaluate(&self, prediction: &Prediction, outcome: &Engram) -> Result<PredictionAccuracy> {
        let actual = self.extract_coding_outcome(outcome)?;
        let accuracy = self.compute_accuracy(prediction, &actual);
        self.corrector.update(&prediction.provenance.model_id, &accuracy.category, accuracy.residual);
        Ok(accuracy)
    }
}
```

## Build Time Prediction

```rust
pub struct BuildTimePredictor {
    history: Vec<BuildTimeObservation>,
    ema: ExponentialMovingAverage,
    crate_models: HashMap<String, CrateCompileModel>,
}

pub struct BuildTimeObservation {
    pub timestamp_ms: i64,
    pub files_changed: usize,
    pub crates_affected: Vec<String>,
    pub incremental: bool,
    pub compile_time_ms: u64,
    pub success: bool,
}

impl BuildTimePredictor {
    pub fn predict(&self, change: &ChangeContext) -> (f64, f64) {
        let base = self.ema.current();
        let scope_factor = self.scope_adjustment(change);
        let crate_factor = self.crate_adjustment(&change.affected_crates);
        let incr_factor = if change.incremental { 1.0 } else { 3.5 };
        let predicted = base * scope_factor * crate_factor * incr_factor;
        let confidence = self.confidence_from_history(change);
        (predicted, confidence)
    }
}
```

## Test Failure Prediction

```rust
pub struct TestFailurePredictor {
    file_test_map: Arc<FileTestMap>,
    test_histories: HashMap<String, TestHistory>,
    flakiness: HashMap<String, f64>,
}

pub struct TestHistory {
    pub total_runs: u64,
    pub failures: u64,
    pub recent_rate: ExponentialMovingAverage,
    pub recent_results: VecDeque<bool>,
}

impl TestFailurePredictor {
    pub fn predict_pass_rate(&self, change: &ChangeContext) -> (f64, f64) {
        let affected_tests = self.file_test_map.tests_for_files(&change.files);
        if affected_tests.is_empty() {
            return (1.0, 0.9);
        }
        let mut expected_failures = 0.0;
        let total = affected_tests.len() as f64;
        for test in &affected_tests {
            if let Some(history) = self.test_histories.get(test) {
                let base_rate = history.recent_rate.current();
                let flakiness = self.flakiness.get(test).copied().unwrap_or(0.0);
                let adj_rate = base_rate * (1.0 - flakiness);
                expected_failures += adj_rate;
            } else {
                expected_failures += 0.1;  // unknown test: conservative
            }
        }
        let predicted_pass_rate = 1.0 - (expected_failures / total);
        let confidence = self.confidence_from_sample_size(total as u64);
        (predicted_pass_rate, confidence)
    }
}
```

## Complexity Drift Detection

Uses MACD-equivalent moving average crossovers:

```rust
pub struct ComplexityDriftDetector {
    module_histories: HashMap<String, Vec<ComplexityObservation>>,
    short_ema: ExponentialMovingAverage,  // 5 commits
    long_ema: ExponentialMovingAverage,   // 25 commits
}

pub struct ComplexityObservation {
    pub commit_hash: String,
    pub timestamp_ms: i64,
    pub cyclomatic_complexity: f64,
    pub cognitive_complexity: f64,
    pub lines_of_code: usize,
    pub function_count: usize,
}

impl ComplexityDriftDetector {
    pub fn detect(&self) -> ComplexityTrend {
        let short = self.short_ema.current();
        let long = self.long_ema.current();
        let macd = short - long;
        let signal = self.macd_signal_ema.current();
        ComplexityTrend {
            direction: macd.signum(),
            magnitude: macd.abs(),
            acceleration: macd - signal,
            confidence: self.confidence_from_history(),
        }
    }
}
```

## Dependency Risk Scoring

```rust
pub struct DependencyRisk {
    pub score: f64,
    pub factors: DependencyRiskFactors,
    pub confidence: f64,
}

pub struct DependencyRiskFactors {
    pub cve_risk: f64,           // known CVEs, severity-weighted
    pub maintenance_risk: f64,   // time since last commit, bus factor
    pub depth_risk: f64,         // deeper = harder to fix
    pub license_risk: f64,       // compatibility with project license
    pub popularity_risk: f64,    // popular = well-tested; unpopular = less scrutiny
}
```

## Performance Regression Forecasting

```rust
pub struct PerfRegressionPredictor {
    benchmarks: HashMap<String, Vec<BenchmarkResult>>,
    file_bench_map: Arc<FileBenchMap>,
    baselines: HashMap<String, BenchmarkBaseline>,
}

pub struct BenchmarkBaseline {
    pub median: f64,
    pub iqr: f64,
    pub n: u64,
}
```

Historical regression rate per benchmark, scaled by change size, produces
the probability of regression and expected magnitude.

## The 6 T0 Coding Probes

```rust
pub fn coding_probes() -> Vec<Box<dyn Probe>> {
    vec![
        Box::new(BuildHealthProbe::new()),       // last compile status + trend
        Box::new(TestRegressionProbe::new()),     // delta of passing test count
        Box::new(ComplexityDriftProbe::new()),     // MACD acceleration
        Box::new(DependencyRiskProbe::new()),      // new vulnerabilities
        Box::new(CoverageDeltaProbe::new()),       // coverage decrease
        Box::new(ErrorRateProbe::new()),           // gate failure trend
    ]
}
```

Each probe: `fn(state) -> f32`. Combined via weighted sum into a prediction
error scalar. For pure coding agents, only these 6 + 2 universal probes run.

## Tech Debt Feedback Loop

```rust
if complexity_trend.acceleration > 0.05 {
    neuro.store(KnowledgeEntry {
        kind: KnowledgeType::Warning,
        content: format!(
            "Complexity growth accelerating in module {}: d2C = {:.3}. \
             Historical pattern: modules with this acceleration rate \
             reach unmaintainability within {} commits.",
            module, complexity_trend.acceleration,
            self.estimated_commits_to_crisis(complexity_trend),
        ),
        confidence: complexity_trend.confidence,
        tier: KnowledgeTier::Working,
        ..Default::default()
    }).await?;
}
```

## Integration with roko-index

The coding oracle relies on `roko-index` for code intelligence:
- File -> symbol graph (function signatures, type definitions)
- File -> test mapping (which tests cover which files)
- File -> dependency mapping (which crates/modules depend on which)
- Module -> complexity metrics (cyclomatic, cognitive, LOC)
- Workspace -> HDC fingerprint (10,240-bit structural hash)

HDC fingerprints enable structural similarity search across codebases.

## Academic Foundations

- McCabe, T. J. (1976). "A Complexity Measure." *IEEE Trans. Software Engineering*, SE-2(4), 308-320.
- Lehman, M. M. (1980). "Programs, Life Cycles, and Laws of Software Evolution." *Proc. IEEE*, 68(9).
- Nagappan, N., & Ball, T. (2005). "Use of Relative Code Churn Measures." *ICSE 2005*.
- Chen, L., et al. (2023). "FrugalGPT." arXiv:2305.05176.
- Kleyko, D., et al. (2022). "A Survey on Hyperdimensional Computing." *ACM Computing Surveys*, 54(6).
