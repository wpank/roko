# Attestation Patterns -- Generalized Data Ingestion

> **Parent:** [38-SIGNAL-ANALYSIS](../../38-SIGNAL-ANALYSIS.md) Section 5

---

## The Attestor Trait

```rust
pub trait Attestor: Send + Sync {
    async fn observe(&self, since: i64) -> Result<Vec<Engram>>;
    async fn subscribe(&self) -> Result<mpsc::Receiver<Engram>>;
    fn health(&self) -> AttestorHealth;
}

pub struct AttestorHealth {
    pub connected: bool,
    pub lag_ms: i64,
    pub observations_since_last: u64,
    pub errors_since_last: u64,
}
```

## Coding Attestor

```rust
pub struct CodingAttestor {
    fs_watcher: Arc<FsWatcher>,
    git: Arc<GitRepository>,
    ci: Arc<dyn CiPipeline>,
    metrics: Arc<CodeMetrics>,
}

#[async_trait]
impl Attestor for CodingAttestor {
    async fn observe(&self, since: i64) -> Result<Vec<Engram>> {
        let mut engrams = Vec::new();

        // Git changes since last observation
        for commit in self.git.commits_since(since).await? {
            engrams.push(Engram::builder()
                .kind(Kind::Observation)
                .body(Body::Json(serde_json::to_value(&commit)?))
                .tag("domain", "coding")
                .tag("event", "commit")
                .build());
        }

        // Latest build result
        if let Some(build) = self.ci.latest_build().await? {
            engrams.push(Engram::builder()
                .kind(Kind::Observation)
                .tag("domain", "coding")
                .tag("event", "build")
                .tag("status", if build.success { "pass" } else { "fail" })
                .build());
        }

        // Complexity metrics snapshot
        let complexity = self.metrics.workspace_complexity().await?;
        engrams.push(Engram::builder()
            .kind(Kind::Observation)
            .tag("domain", "coding")
            .tag("event", "complexity")
            .build());

        Ok(engrams)
    }
}
```

## Triage Pipeline

```rust
pub struct TriagePipeline {
    anomaly_detector: MidasR,       // O(1) memory, sub-us per update
    percentile_tracker: DdSketch,   // O(1) memory per sketch
    classifiers: Vec<Box<dyn ObservationClassifier>>,
    priority_scorer: PriorityScorer,
}

impl TriagePipeline {
    pub fn triage(&mut self, observations: &[Engram]) -> Vec<TriagedObservation> {
        observations.iter().filter_map(|obs| {
            let anomaly_score = self.anomaly_detector.score(obs);
            let percentile = self.percentile_tracker.rank(obs.numeric_value()?);
            let category = self.classify(obs);
            let priority = self.priority_scorer.score(anomaly_score, percentile, &category);
            if priority > 0.2 {
                Some(TriagedObservation { observation: obs.clone(), anomaly_score, percentile, category, priority })
            } else {
                None
            }
        }).collect()
    }
}
```

## CorticalState

```rust
pub struct CorticalState<const N: usize> {
    pub signals: [AtomicF64; N],
    pub prediction_error: AtomicF64,
    pub weights: [AtomicF64; N],
    pub behavioral_state: AtomicU8,
    pub last_update_ms: AtomicI64,
}

pub type CodingCorticalState = CorticalState<6>;
```

Updated at Gamma frequency. All operations atomic -- no locking, no
allocation, sub-microsecond latency.

## Three Cognitive Speeds

### Gamma (~5-15s)
```
Attestor.observe() -> new observations -> triage (MIDAS-R + DDSketch) ->
CorticalState update (atomic) -> T0 probes -> T0/T1/T2 routing
```
Cost: microseconds. No LLM.

### Theta (~75s)
```
Pending predictions resolved -> residuals computed -> CalibrationTracker
updated -> oracle re-predicts -> significant observations to Neuro
```

### Delta (hours)
```
Dreams replay observations -> NREM consolidation -> REM counterfactual ->
cross-domain pattern consolidation -> routing table updates
```

## Memory Architecture -- Three Timescales

| Timescale | Memory type | What it stores | Decay |
|---|---|---|---|
| **Gamma** (seconds) | CorticalState | Signal values, prediction error | Overwritten each tick |
| **Theta** (minutes) | Working Engrams | Observations, pending predictions | Hours (Ebbinghaus) |
| **Delta** (hours) | Neuro knowledge | Validated patterns, calibration | Days to months (tier) |

Complementary Learning Systems theory (McClelland et al., 1995): fast episodic
memory captures details, slow semantic memory captures patterns.

## Academic Foundations

- Bhatia, S., et al. (2020). "MIDAS." *AAAI 2020*.
- Masson, C., et al. (2019). "DDSketch." *PVLDB*, 12(12), 2195-2205.
- McClelland, J. L., et al. (1995). "Complementary learning systems." *Psychological Review*, 102(3), 419-457.
