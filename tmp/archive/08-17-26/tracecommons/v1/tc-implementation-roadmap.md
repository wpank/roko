# TraceCommons Implementation Roadmap

Self-contained implementation guide for seven feature areas. Each section
describes what the feature is, why it matters for TraceCommons, which existing
files and traits to extend, concrete Rust code sketches, PostgreSQL migrations,
and priority/complexity estimates.

## Background: TraceCommons Architecture

TraceCommons is a hosted server-side control plane for collecting, scoring,
de-duplicating, and storing LLM agent traces. The codebase is a Rust workspace
with six crates:

| Crate | Role |
|---|---|
| `trace-commons-gate-api` | Stable trait boundary: `PerplexityScorer`, `Embedder`, `VectorIndex`, `EnclaveGateOrchestratorConfig`, decision types |
| `trace-commons-gate-enclave` | Scoring implementations: chunker, chunk aggregator, orchestrator (`EnclaveGateOrchestrator`), mock + real scorers/embedders/indices |
| `trace-commons-protocol` | Wire types: `TraceContributionEnvelope`, redaction, privacy filters |
| `trace-commons-server` | Server binary + library: PostgreSQL storage (`TraceCorpusStore`), gate service (`TraceGateService`), dedup (simhash + cluster assignment), credit quality, artifact store, auth |
| `trace-commons-contributor` | Client-side trace submission |
| `trace-commons-operator-client` | Operator HTTP client for admin/review/worker binaries |

The gate pipeline today is a two-phase process:

1. **Perplexity scoring** -- chunk the envelope plaintext into bounded windows,
   score each chunk via a `PerplexityScorer` (local LLM or NEAR AI Cloud),
   aggregate into representative + peak perplexity.
2. **Novelty scoring** -- embed each chunk via an `Embedder` (fastembed/ONNX),
   query top-k nearest neighbors from a `VectorIndex` (usearch HNSW), compute
   novelty as `1 - max(cosine_similarity)`.

Both gates must pass for a trace to be accepted and its embeddings inserted
into the index. The orchestrator is `EnclaveGateOrchestrator<P, E, V>` in
`crates/trace-commons-gate-enclave/src/orchestrator.rs`.

Storage is PostgreSQL-only with row-level security (RLS) on all tenant-scoped
tables. The `trace_gate_decisions` table stores per-submission gate outcomes
with perplexity, novelty, and credit-quality scores in fixed-point micros.

---

## 1. Adaptive Scoring

**Priority: P0** | **Complexity: Medium-High** | **Estimated effort: 3-4 weeks**

### What and why

TraceCommons currently uses static threshold floors configured in
`EnclaveGateOrchestratorConfig` (fields `perplexity_floor_micros`,
`tail_fraction_floor_micros`, `novelty_floor_micros`). As the corpus grows,
the distribution of trace quality shifts: early traces are novel by default
(empty index), while a mature corpus has higher novelty baselines. Static
floors either reject too aggressively (killing early adoption) or too
permissively (admitting noise at scale).

Adaptive scoring solves this by:

- Tracking exponential moving averages (EMA) of gate signals to dynamically
  adjust floors.
- Detecting distribution shifts via CUSUM and BOCD so floors respond to
  genuine changes in trace quality, not just drift.
- Giving operators real-time visibility into threshold evolution.

### Files to extend

- `crates/trace-commons-gate-api/src/decision.rs` -- extend
  `EnclaveGateOrchestratorConfig` with adaptive threshold fields
- `crates/trace-commons-gate-enclave/src/orchestrator.rs` -- inject the
  threshold manager into the orchestrator
- `crates/trace-commons-server/src/trace_gate_service.rs` -- wire the
  manager into the service layer
- `crates/trace-commons-server/src/credit_quality.rs` -- adjust credit quality
  constants based on corpus maturity
- New file: `crates/trace-commons-gate-enclave/src/adaptive_threshold.rs`

### Rust implementation

#### Core types

```rust
// crates/trace-commons-gate-enclave/src/adaptive_threshold.rs

use std::sync::Mutex;

/// EMA state for one gate signal. Tracks the running mean and variance
/// using Welford's online algorithm alongside the exponential moving average.
#[derive(Debug, Clone, Copy)]
pub struct EmaState {
    /// Current EMA value in micros.
    pub ema_micros: f64,
    /// EMA of squared deviations (for variance tracking).
    pub ema_var: f64,
    /// Smoothing factor in (0, 1]. Smaller = slower adaptation.
    pub alpha: f64,
    /// Total observations seen (for cold-start guard).
    pub n: u64,
}

impl EmaState {
    pub fn new(alpha: f64) -> Self {
        Self {
            ema_micros: 0.0,
            ema_var: 0.0,
            alpha: alpha.clamp(0.001, 1.0),
            n: 0,
        }
    }

    /// Update the EMA with a new observation. Returns the updated EMA value.
    pub fn update(&mut self, value_micros: f64) -> f64 {
        if self.n == 0 {
            self.ema_micros = value_micros;
            self.ema_var = 0.0;
        } else {
            let delta = value_micros - self.ema_micros;
            self.ema_micros += self.alpha * delta;
            // EMA of variance: exponentially weighted squared deviation.
            self.ema_var = (1.0 - self.alpha) * (self.ema_var + self.alpha * delta * delta);
        }
        self.n += 1;
        self.ema_micros
    }

    /// Standard deviation derived from the EMA variance.
    pub fn std_dev(&self) -> f64 {
        self.ema_var.sqrt()
    }

    /// Adaptive floor: EMA minus k standard deviations, floored at
    /// `hard_minimum_micros` to prevent the threshold from going negative
    /// or dangerously low.
    pub fn adaptive_floor(&self, k_sigma: f64, hard_minimum_micros: f64) -> f64 {
        (self.ema_micros - k_sigma * self.std_dev()).max(hard_minimum_micros)
    }
}

/// CUSUM (Cumulative Sum) change-point detector. Detects sustained upward
/// or downward shifts in a signal by accumulating deviations from an
/// expected mean. When the cumulative sum exceeds a threshold, a change
/// point is signaled.
///
/// Reference: Page, E.S. (1954). "Continuous Inspection Schemes."
#[derive(Debug, Clone, Copy)]
pub struct CusumDetector {
    /// Expected process mean (updated periodically from EMA).
    pub target_mean: f64,
    /// Minimum shift magnitude to detect (slack parameter). Typically
    /// set to 0.5 * expected_shift_size.
    pub allowance: f64,
    /// Detection threshold. Alarm when cumulative sum exceeds this.
    pub threshold: f64,
    /// Cumulative sum for upward shifts.
    pub s_high: f64,
    /// Cumulative sum for downward shifts.
    pub s_low: f64,
}

impl CusumDetector {
    pub fn new(target_mean: f64, allowance: f64, threshold: f64) -> Self {
        Self {
            target_mean,
            allowance,
            threshold,
            s_high: 0.0,
            s_low: 0.0,
        }
    }

    /// Feed a new observation. Returns `Some(ShiftDirection)` if a change
    /// point is detected, `None` otherwise.
    pub fn observe(&mut self, value: f64) -> Option<ShiftDirection> {
        self.s_high = (self.s_high + value - self.target_mean - self.allowance).max(0.0);
        self.s_low = (self.s_low - value + self.target_mean - self.allowance).max(0.0);

        if self.s_high > self.threshold {
            self.s_high = 0.0; // Reset after detection.
            Some(ShiftDirection::Up)
        } else if self.s_low > self.threshold {
            self.s_low = 0.0;
            Some(ShiftDirection::Down)
        } else {
            None
        }
    }

    /// Reset the detector with a new target mean (e.g., after confirming
    /// and absorbing a shift).
    pub fn reset_target(&mut self, new_mean: f64) {
        self.target_mean = new_mean;
        self.s_high = 0.0;
        self.s_low = 0.0;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShiftDirection {
    Up,
    Down,
}

/// Bayesian Online Change-point Detection (Adams & MacKay, 2007).
///
/// Maintains a run-length distribution P(r_t | x_{1:t}) that gives the
/// probability of how long the current "run" (segment without a change
/// point) has lasted. When the probability mass concentrates on short
/// run lengths, a change point is likely.
///
/// This implementation uses a Gaussian predictive model with conjugate
/// Normal-Gamma prior, sufficient for the univariate gate signals
/// (perplexity, novelty in micros).
#[derive(Debug, Clone)]
pub struct BocdDetector {
    /// Hazard rate: prior probability of a change point at each step.
    /// Typically 1/expected_run_length (e.g., 1/200 for daily scoring
    /// with shifts expected every ~200 batches).
    pub hazard_rate: f64,
    /// Run-length distribution (log-probabilities for numerical stability).
    /// Entry i = log P(run_length = i | observations so far).
    pub run_length_log_probs: Vec<f64>,
    /// Sufficient statistics for each run length: (count, sum, sum_sq).
    /// Used to compute the Gaussian predictive probability.
    pub sufficient_stats: Vec<(u64, f64, f64)>,
    /// Prior mean for the Gaussian predictive model.
    pub prior_mean: f64,
    /// Prior precision (inverse variance) for the Gaussian predictive model.
    pub prior_precision: f64,
    /// Change-point probability threshold: when P(run_length < k) > this,
    /// declare a change point.
    pub cp_threshold: f64,
}

impl BocdDetector {
    pub fn new(hazard_rate: f64, prior_mean: f64, prior_precision: f64, cp_threshold: f64) -> Self {
        Self {
            hazard_rate,
            run_length_log_probs: vec![0.0], // P(r=0) = 1 initially.
            sufficient_stats: vec![(0, 0.0, 0.0)],
            prior_mean,
            prior_precision,
            cp_threshold,
        }
    }

    /// Feed a new observation. Returns `true` if a change point is detected.
    pub fn observe(&mut self, value: f64) -> bool {
        let n = self.run_length_log_probs.len();
        let mut pred_log_probs = Vec::with_capacity(n);

        // Compute predictive log-probabilities for each run length.
        for (count, sum, sum_sq) in &self.sufficient_stats {
            let pred_lp = self.gaussian_predictive_log_prob(value, *count, *sum, *sum_sq);
            pred_log_probs.push(pred_lp);
        }

        let log_h = self.hazard_rate.ln();
        let log_1mh = (1.0 - self.hazard_rate).ln();

        // Growth probabilities: existing runs that did NOT see a change point.
        let mut new_log_probs = Vec::with_capacity(n + 1);
        // Change-point probability: sum over all run lengths of
        // P(r_t) * P(x_t | r_t) * hazard.
        let mut cp_log_prob = f64::NEG_INFINITY;

        for i in 0..n {
            let lp = self.run_length_log_probs[i] + pred_log_probs[i];
            cp_log_prob = log_sum_exp(cp_log_prob, lp + log_h);
            new_log_probs.push(lp + log_1mh);
        }

        // Prepend the change-point run length (r=0).
        let mut final_log_probs = vec![cp_log_prob];
        final_log_probs.extend_from_slice(&new_log_probs);

        // Normalize.
        let log_evidence = final_log_probs
            .iter()
            .copied()
            .fold(f64::NEG_INFINITY, log_sum_exp);
        for lp in &mut final_log_probs {
            *lp -= log_evidence;
        }

        // Update sufficient statistics.
        let mut new_stats = vec![(0u64, 0.0f64, 0.0f64)]; // For r=0.
        for (count, sum, sum_sq) in &self.sufficient_stats {
            new_stats.push((count + 1, sum + value, sum_sq + value * value));
        }

        self.run_length_log_probs = final_log_probs;
        self.sufficient_stats = new_stats;

        // Detect: P(run_length < short_window) > threshold.
        let short_window = 3.min(self.run_length_log_probs.len());
        let short_mass: f64 = self.run_length_log_probs[..short_window]
            .iter()
            .map(|lp| lp.exp())
            .sum();
        short_mass > self.cp_threshold
    }

    /// Student-t predictive log-probability under conjugate Normal-Gamma prior.
    fn gaussian_predictive_log_prob(&self, x: f64, count: u64, sum: f64, sum_sq: f64) -> f64 {
        let n = count as f64;
        let kappa = self.prior_precision + n;
        let mu = (self.prior_precision * self.prior_mean + sum) / kappa;
        let alpha = 1.0 + n / 2.0;
        let beta = 1.0
            + 0.5 * (sum_sq - sum * sum / n.max(1.0))
            + 0.5 * self.prior_precision * n * (self.prior_mean - sum / n.max(1.0)).powi(2)
                / kappa;

        let variance = beta * (kappa + 1.0) / (alpha * kappa);
        let std_dev = variance.sqrt().max(1e-10);
        // Approximate with Gaussian for simplicity; Student-t has heavier
        // tails but for large n the difference is negligible.
        let z = (x - mu) / std_dev;
        -0.5 * z * z - std_dev.ln() - 0.5 * (2.0 * std::f64::consts::PI).ln()
    }
}

fn log_sum_exp(a: f64, b: f64) -> f64 {
    if a == f64::NEG_INFINITY {
        return b;
    }
    if b == f64::NEG_INFINITY {
        return a;
    }
    let max = a.max(b);
    max + ((a - max).exp() + (b - max).exp()).ln()
}

/// Manages adaptive thresholds for all gate signals. Thread-safe via
/// interior mutability. Persists state to PostgreSQL between restarts.
pub struct AdaptiveThresholdManager {
    inner: Mutex<AdaptiveThresholdState>,
}

struct AdaptiveThresholdState {
    perplexity_ema: EmaState,
    novelty_ema: EmaState,
    tail_fraction_ema: EmaState,

    perplexity_cusum: CusumDetector,
    novelty_cusum: CusumDetector,

    perplexity_bocd: BocdDetector,
    novelty_bocd: BocdDetector,

    config: AdaptiveThresholdConfig,
}

/// Configuration for the adaptive threshold system.
#[derive(Debug, Clone)]
pub struct AdaptiveThresholdConfig {
    /// EMA smoothing factor (0, 1]. Default 0.02 = ~50-observation half-life.
    pub ema_alpha: f64,
    /// Number of standard deviations below EMA for the adaptive floor.
    pub floor_k_sigma: f64,
    /// Hard minimum floors (never go below these regardless of EMA).
    pub hard_min_perplexity_micros: f64,
    pub hard_min_novelty_micros: f64,
    pub hard_min_tail_fraction_micros: f64,
    /// Minimum observations before the adaptive floor takes effect.
    /// Below this, the hard minimums are used exclusively.
    pub cold_start_n: u64,
    /// CUSUM allowance and threshold, calibrated per signal.
    pub cusum_allowance: f64,
    pub cusum_threshold: f64,
    /// BOCD hazard rate and change-point threshold.
    pub bocd_hazard_rate: f64,
    pub bocd_cp_threshold: f64,
}

impl Default for AdaptiveThresholdConfig {
    fn default() -> Self {
        Self {
            ema_alpha: 0.02,
            floor_k_sigma: 2.0,
            hard_min_perplexity_micros: 1_000_000.0, // 1.0 real perplexity
            hard_min_novelty_micros: 50_000.0,        // 0.05 real novelty
            hard_min_tail_fraction_micros: 0.0,
            cold_start_n: 100,
            cusum_allowance: 500_000.0,
            cusum_threshold: 3_000_000.0,
            bocd_hazard_rate: 1.0 / 200.0,
            bocd_cp_threshold: 0.5,
        }
    }
}

/// Snapshot of the current adaptive thresholds, returned for both
/// application (overriding `EnclaveGateOrchestratorConfig` floors)
/// and persistence.
#[derive(Debug, Clone, Copy)]
pub struct AdaptiveThresholdSnapshot {
    pub perplexity_floor_micros: u64,
    pub novelty_floor_micros: u64,
    pub tail_fraction_floor_micros: u64,
    pub perplexity_ema_micros: u64,
    pub novelty_ema_micros: u64,
    pub observation_count: u64,
    pub perplexity_shift_detected: bool,
    pub novelty_shift_detected: bool,
}

impl AdaptiveThresholdManager {
    pub fn new(config: AdaptiveThresholdConfig) -> Self {
        let alpha = config.ema_alpha;
        Self {
            inner: Mutex::new(AdaptiveThresholdState {
                perplexity_ema: EmaState::new(alpha),
                novelty_ema: EmaState::new(alpha),
                tail_fraction_ema: EmaState::new(alpha),
                perplexity_cusum: CusumDetector::new(0.0, config.cusum_allowance, config.cusum_threshold),
                novelty_cusum: CusumDetector::new(0.0, config.cusum_allowance, config.cusum_threshold),
                perplexity_bocd: BocdDetector::new(
                    config.bocd_hazard_rate,
                    0.0,
                    1.0,
                    config.bocd_cp_threshold,
                ),
                novelty_bocd: BocdDetector::new(
                    config.bocd_hazard_rate,
                    0.0,
                    1.0,
                    config.bocd_cp_threshold,
                ),
                config,
            }),
        }
    }

    /// Record a gate evaluation result and return updated thresholds.
    /// Called after every `EnclaveGateOrchestrator::evaluate`.
    pub fn record_and_snapshot(
        &self,
        perplexity_micros: u64,
        novelty_micros: u64,
        tail_fraction_micros: u64,
    ) -> AdaptiveThresholdSnapshot {
        let mut s = self.inner.lock().expect("AdaptiveThresholdManager poisoned");
        let pm = perplexity_micros as f64;
        let nm = novelty_micros as f64;
        let tm = tail_fraction_micros as f64;

        s.perplexity_ema.update(pm);
        s.novelty_ema.update(nm);
        s.tail_fraction_ema.update(tm);

        let perplexity_shift = s.perplexity_cusum.observe(pm).is_some()
            || s.perplexity_bocd.observe(pm);
        let novelty_shift = s.novelty_cusum.observe(nm).is_some()
            || s.novelty_bocd.observe(nm);

        // If a shift is detected, reset CUSUM targets to the new EMA.
        if perplexity_shift {
            s.perplexity_cusum.reset_target(s.perplexity_ema.ema_micros);
        }
        if novelty_shift {
            s.novelty_cusum.reset_target(s.novelty_ema.ema_micros);
        }

        let n = s.perplexity_ema.n;
        let use_adaptive = n >= s.config.cold_start_n;

        AdaptiveThresholdSnapshot {
            perplexity_floor_micros: if use_adaptive {
                s.perplexity_ema.adaptive_floor(
                    s.config.floor_k_sigma,
                    s.config.hard_min_perplexity_micros,
                ) as u64
            } else {
                s.config.hard_min_perplexity_micros as u64
            },
            novelty_floor_micros: if use_adaptive {
                s.novelty_ema.adaptive_floor(
                    s.config.floor_k_sigma,
                    s.config.hard_min_novelty_micros,
                ) as u64
            } else {
                s.config.hard_min_novelty_micros as u64
            },
            tail_fraction_floor_micros: if use_adaptive {
                s.tail_fraction_ema.adaptive_floor(
                    s.config.floor_k_sigma,
                    s.config.hard_min_tail_fraction_micros,
                ) as u64
            } else {
                s.config.hard_min_tail_fraction_micros as u64
            },
            perplexity_ema_micros: s.perplexity_ema.ema_micros as u64,
            novelty_ema_micros: s.novelty_ema.ema_micros as u64,
            observation_count: n,
            perplexity_shift_detected: perplexity_shift,
            novelty_shift_detected: novelty_shift,
        }
    }
}
```

#### Extending EnclaveGateOrchestratorConfig

In `crates/trace-commons-gate-api/src/decision.rs`, add:

```rust
/// When `adaptive_thresholds` is `true`, the orchestrator's floors are
/// treated as hard minimums and the `AdaptiveThresholdManager` can raise
/// them above these values based on corpus statistics.
pub struct EnclaveGateOrchestratorConfig {
    // ... existing fields ...
    pub adaptive_thresholds: bool,
    pub adaptive_ema_alpha: f64,
    pub adaptive_floor_k_sigma: f64,
    pub adaptive_cold_start_n: u64,
}
```

#### PostgreSQL migration: V42__adaptive_gate_thresholds.sql

```sql
-- Adaptive gate threshold state, one row per tenant.
-- Persisted after each scoring batch so restarts resume from the
-- last known EMA/CUSUM state rather than cold-starting.
CREATE TABLE trace_adaptive_thresholds (
    tenant_id         UUID NOT NULL REFERENCES trace_tenants(id),
    signal_name       TEXT NOT NULL CHECK (signal_name IN (
        'perplexity', 'novelty', 'tail_fraction'
    )),
    ema_value_micros  BIGINT NOT NULL DEFAULT 0,
    ema_variance      DOUBLE PRECISION NOT NULL DEFAULT 0.0,
    observation_count BIGINT NOT NULL DEFAULT 0,
    cusum_s_high      DOUBLE PRECISION NOT NULL DEFAULT 0.0,
    cusum_s_low       DOUBLE PRECISION NOT NULL DEFAULT 0.0,
    cusum_target_mean DOUBLE PRECISION NOT NULL DEFAULT 0.0,
    last_shift_at     TIMESTAMPTZ,
    updated_at        TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (tenant_id, signal_name)
);

ALTER TABLE trace_adaptive_thresholds ENABLE ROW LEVEL SECURITY;
ALTER TABLE trace_adaptive_thresholds FORCE ROW LEVEL SECURITY;

CREATE POLICY trace_adaptive_thresholds_tenant_policy
    ON trace_adaptive_thresholds
    USING (tenant_id = trace_current_tenant_id());

-- Change-point event log for operational visibility.
CREATE TABLE trace_threshold_shift_events (
    id                UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id         UUID NOT NULL REFERENCES trace_tenants(id),
    signal_name       TEXT NOT NULL,
    shift_direction   TEXT NOT NULL CHECK (shift_direction IN ('up', 'down')),
    detector          TEXT NOT NULL CHECK (detector IN ('cusum', 'bocd')),
    old_ema_micros    BIGINT NOT NULL,
    new_ema_micros    BIGINT NOT NULL,
    observation_count BIGINT NOT NULL,
    detected_at       TIMESTAMPTZ NOT NULL DEFAULT now()
);

ALTER TABLE trace_threshold_shift_events ENABLE ROW LEVEL SECURITY;
ALTER TABLE trace_threshold_shift_events FORCE ROW LEVEL SECURITY;

CREATE POLICY trace_threshold_shift_events_tenant_policy
    ON trace_threshold_shift_events
    USING (tenant_id = trace_current_tenant_id());
```

### Integration point

In the `EnclaveGateService` implementations in
`crates/trace-commons-server/src/trace_gate_service.rs`, after calling
`orchestrator.evaluate()`, call
`threshold_manager.record_and_snapshot(decision.perplexity_micros, ...)`.
The snapshot's adaptive floors are used for the NEXT evaluation. The
`trace_gate_service.rs` file already holds the `TraceGateService` trait and
its `InMemoryGateService` / `EnclaveLocalGateService` implementations -- the
threshold manager slots in as a new field on those structs.

---

## 2. Multi-Stage Gate Pipeline

**Priority: P0** | **Complexity: High** | **Estimated effort: 4-5 weeks**

### What and why

Today the gate is a monolithic two-phase process (perplexity then novelty)
inside `EnclaveGateOrchestrator::evaluate`. Every trace pays the full cost of
LLM inference and embedding, even when it could be rejected cheaply at an
earlier stage (syntax errors, exact duplicates, trivially low entropy).

A multi-stage pipeline introduces a sequence of increasingly expensive "rungs."
Traces that fail early rungs never reach the expensive TEE scoring, saving
compute costs that scale linearly with trace volume. The seven rungs are:

1. **Syntax check** -- is the envelope well-formed JSON with valid events?
2. **Dedup (Bloom + SimHash)** -- exact or near-exact duplicate?
3. **Perplexity (byte-entropy)** -- quick Shannon entropy pre-screen using
   the existing `ReferencePerplexityScorer` (no LLM needed).
4. **Novelty (MinHash-LSH)** -- approximate set similarity against the corpus,
   orders of magnitude cheaper than full embedding + HNSW lookup.
5. **Semantic cluster** -- does this trace add to an under-represented cluster,
   or is it the 500th variant of "hello world"?
6. **Full TEE scoring** -- the existing LLM perplexity + embedding novelty pass.
7. **Consensus** -- optional multi-scorer agreement for high-stakes decisions.

### Files to extend

- New file: `crates/trace-commons-gate-enclave/src/gate_pipeline.rs`
- New file: `crates/trace-commons-gate-enclave/src/rung/mod.rs` (plus one file
  per rung)
- `crates/trace-commons-gate-enclave/src/lib.rs` -- add `pub mod gate_pipeline;
  pub mod rung;`
- `crates/trace-commons-gate-enclave/src/orchestrator.rs` -- refactor `evaluate`
  to delegate to the pipeline
- `crates/trace-commons-server/src/trace_gate_service.rs` -- wire pipeline into
  the service

### Rust implementation

#### GateRung trait

```rust
// crates/trace-commons-gate-enclave/src/gate_pipeline.rs

use std::fmt;

/// Result of evaluating one rung of the gate pipeline.
#[derive(Debug, Clone)]
pub struct RungResult {
    /// Rung identifier for audit logging.
    pub rung_name: &'static str,
    /// Rung ordinal (0-indexed).
    pub rung_index: u8,
    /// Whether the trace passed this rung.
    pub passed: bool,
    /// Rung-specific score in micros (signal-dependent; higher = better).
    pub score_micros: u64,
    /// Human-readable reason when `passed == false`.
    pub rejection_reason: Option<String>,
    /// Wall-clock time spent in this rung, in microseconds.
    pub elapsed_micros: u64,
}

/// A single stage in the gate pipeline. Implementations are ordered by
/// cost: cheaper rungs run first.
pub trait GateRung: Send + Sync + fmt::Debug {
    /// Unique name for audit logging and metrics. Must be stable across
    /// releases (persisted in `trace_gate_rung_results`).
    fn name(&self) -> &'static str;

    /// Ordinal position in the pipeline (0 = cheapest).
    fn index(&self) -> u8;

    /// Evaluate a trace against this rung. The `context` carries
    /// accumulated state from prior rungs (e.g., parsed envelope,
    /// simhash, entropy estimate).
    fn evaluate(&self, context: &mut PipelineContext) -> anyhow::Result<RungResult>;
}

/// Accumulated state passed between rungs. Each rung may read from and
/// write to this context so downstream rungs can reuse upstream work
/// (e.g., the dedup rung computes simhash, which the cluster rung reuses).
#[derive(Debug)]
pub struct PipelineContext {
    /// Raw envelope plaintext bytes.
    pub plaintext: Vec<u8>,
    /// Tenant storage ref for index lookups.
    pub tenant_storage_ref: String,
    /// Parsed and rendered events (populated by syntax rung).
    pub rendered_events: Option<Vec<String>>,
    /// Simhash of the canonical text (populated by dedup rung).
    pub simhash: Option<u64>,
    /// Byte-entropy perplexity estimate (populated by entropy rung).
    pub entropy_perplexity_micros: Option<u64>,
    /// MinHash signature (populated by LSH rung).
    pub minhash_signature: Option<Vec<u64>>,
    /// Full embeddings per chunk (populated by TEE rung).
    pub chunk_embeddings: Option<Vec<Vec<f32>>>,
    /// The full OrchestrationDecision (populated by TEE rung).
    pub full_decision: Option<crate::OrchestrationDecision>,
}

impl PipelineContext {
    pub fn new(plaintext: Vec<u8>, tenant_storage_ref: String) -> Self {
        Self {
            plaintext,
            tenant_storage_ref,
            rendered_events: None,
            simhash: None,
            entropy_perplexity_micros: None,
            minhash_signature: None,
            chunk_embeddings: None,
            full_decision: None,
        }
    }
}

/// The complete pipeline result.
#[derive(Debug, Clone)]
pub struct PipelineDecision {
    /// Per-rung results, in execution order. The last entry is the rung
    /// that determined the final outcome.
    pub rung_results: Vec<RungResult>,
    /// Index of the rung that determined the final outcome.
    pub deciding_rung: usize,
    /// Overall pass/fail.
    pub accepted: bool,
    /// The full OrchestrationDecision if the trace reached the TEE rung.
    pub orchestration_decision: Option<crate::OrchestrationDecision>,
    /// Total wall-clock time across all rungs, in microseconds.
    pub total_elapsed_micros: u64,
}

/// Chain of rungs with short-circuit logic. A trace must pass every rung
/// to be accepted; the first failure stops the pipeline.
pub struct GatePipeline {
    rungs: Vec<Box<dyn GateRung>>,
}

impl GatePipeline {
    pub fn new(mut rungs: Vec<Box<dyn GateRung>>) -> Self {
        // Sort by index to enforce cost ordering regardless of insertion order.
        rungs.sort_by_key(|r| r.index());
        Self { rungs }
    }

    /// Evaluate a trace through the pipeline. Short-circuits on the first
    /// rung failure.
    pub fn evaluate(&self, mut context: PipelineContext) -> anyhow::Result<PipelineDecision> {
        let mut rung_results = Vec::with_capacity(self.rungs.len());
        let mut total_elapsed = 0u64;

        for rung in &self.rungs {
            let result = rung.evaluate(&mut context)?;
            total_elapsed += result.elapsed_micros;
            let passed = result.passed;
            rung_results.push(result);

            if !passed {
                return Ok(PipelineDecision {
                    deciding_rung: rung_results.len() - 1,
                    rung_results,
                    accepted: false,
                    orchestration_decision: context.full_decision,
                    total_elapsed_micros: total_elapsed,
                });
            }
        }

        Ok(PipelineDecision {
            deciding_rung: rung_results.len().saturating_sub(1),
            rung_results,
            accepted: true,
            orchestration_decision: context.full_decision,
            total_elapsed_micros: total_elapsed,
        })
    }
}
```

#### Example rung: SyntaxCheckRung

```rust
// crates/trace-commons-gate-enclave/src/rung/syntax.rs

use crate::chunker::parse_envelope_rendered_events;
use crate::gate_pipeline::{GateRung, PipelineContext, RungResult};

/// Rung 0: verify the envelope is parseable JSON with at least one event.
/// Cost: O(n) JSON parse, no I/O, no model inference.
#[derive(Debug)]
pub struct SyntaxCheckRung;

impl GateRung for SyntaxCheckRung {
    fn name(&self) -> &'static str { "syntax_check" }
    fn index(&self) -> u8 { 0 }

    fn evaluate(&self, ctx: &mut PipelineContext) -> anyhow::Result<RungResult> {
        let start = std::time::Instant::now();
        match parse_envelope_rendered_events(&ctx.plaintext) {
            Some(events) if !events.is_empty() => {
                ctx.rendered_events = Some(events);
                Ok(RungResult {
                    rung_name: self.name(),
                    rung_index: self.index(),
                    passed: true,
                    score_micros: 1_000_000, // Binary: 1.0 = valid.
                    rejection_reason: None,
                    elapsed_micros: start.elapsed().as_micros() as u64,
                })
            }
            _ => Ok(RungResult {
                rung_name: self.name(),
                rung_index: self.index(),
                passed: false,
                score_micros: 0,
                rejection_reason: Some(
                    "envelope is not valid JSON or contains no events".into(),
                ),
                elapsed_micros: start.elapsed().as_micros() as u64,
            }),
        }
    }
}
```

#### Example rung: DedupBloomRung

```rust
// crates/trace-commons-gate-enclave/src/rung/dedup.rs

use crate::gate_pipeline::{GateRung, PipelineContext, RungResult};
use sha2::{Digest, Sha256};
use std::sync::Mutex;

/// Rung 1: Bloom filter for exact-duplicate detection + simhash for
/// near-duplicate detection. Cost: O(n) hash, no model inference.
#[derive(Debug)]
pub struct DedupBloomRung {
    /// Bloom filter bit array. 2^20 bits = 128 KB, supports ~75K entries
    /// at 1% FPR with 7 hash functions.
    bloom_bits: Mutex<Vec<u64>>,
    bloom_num_hashes: usize,
    bloom_size_bits: usize,
    /// Simhash Hamming distance threshold for near-duplicate rejection.
    simhash_threshold: u32,
}

impl DedupBloomRung {
    pub fn new(bloom_size_bits: usize, num_hashes: usize, simhash_threshold: u32) -> Self {
        let words = (bloom_size_bits + 63) / 64;
        Self {
            bloom_bits: Mutex::new(vec![0u64; words]),
            bloom_num_hashes: num_hashes,
            bloom_size_bits,
            simhash_threshold,
        }
    }

    fn bloom_hash(data: &[u8], seed: usize) -> u64 {
        let mut h = Sha256::new();
        h.update((seed as u64).to_be_bytes());
        h.update(data);
        let out = h.finalize();
        let mut buf = [0u8; 8];
        buf.copy_from_slice(&out[0..8]);
        u64::from_be_bytes(buf)
    }
}

impl GateRung for DedupBloomRung {
    fn name(&self) -> &'static str { "dedup_bloom" }
    fn index(&self) -> u8 { 1 }

    fn evaluate(&self, ctx: &mut PipelineContext) -> anyhow::Result<RungResult> {
        let start = std::time::Instant::now();

        // Content hash for exact dedup.
        let content_hash = {
            let mut h = Sha256::new();
            h.update(&ctx.plaintext);
            h.finalize().to_vec()
        };

        // Check Bloom filter.
        let mut bits = self.bloom_bits.lock().expect("bloom poisoned");
        let mut all_set = true;
        let mut positions = Vec::with_capacity(self.bloom_num_hashes);
        for seed in 0..self.bloom_num_hashes {
            let hash = Self::bloom_hash(&content_hash, seed);
            let pos = (hash as usize) % self.bloom_size_bits;
            positions.push(pos);
            let word = pos / 64;
            let bit = pos % 64;
            if bits[word] & (1u64 << bit) == 0 {
                all_set = false;
            }
        }

        if all_set {
            return Ok(RungResult {
                rung_name: self.name(),
                rung_index: self.index(),
                passed: false,
                score_micros: 0,
                rejection_reason: Some("exact duplicate detected by Bloom filter".into()),
                elapsed_micros: start.elapsed().as_micros() as u64,
            });
        }

        // Insert into Bloom filter.
        for pos in &positions {
            let word = pos / 64;
            let bit = pos % 64;
            bits[word] |= 1u64 << bit;
        }
        drop(bits);

        // Compute simhash for near-duplicate detection (stored in context
        // for downstream rungs and the cluster assignment worker).
        let canonical_text = if let Some(ref events) = ctx.rendered_events {
            events.join("")
        } else {
            String::from_utf8_lossy(&ctx.plaintext).into_owned()
        };
        let simhash = crate::dedup_simhash_internal::trace_simhash(&canonical_text);
        ctx.simhash = Some(simhash);

        Ok(RungResult {
            rung_name: self.name(),
            rung_index: self.index(),
            passed: true,
            score_micros: 1_000_000,
            rejection_reason: None,
            elapsed_micros: start.elapsed().as_micros() as u64,
        })
    }
}
```

Note: The `DedupBloomRung` references `dedup_simhash_internal` -- in the actual
implementation, the simhash logic from
`crates/trace-commons-server/src/dedup_simhash.rs` should be factored into the
`gate-enclave` crate (or a shared utility) so the enclave-side pipeline can
use it without depending on the server crate.

#### PostgreSQL migration: V43__gate_pipeline_rung_results.sql

```sql
-- Per-rung audit trail for the multi-stage gate pipeline.
CREATE TABLE trace_gate_rung_results (
    id                UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    submission_id     UUID NOT NULL,
    tenant_id         UUID NOT NULL,
    rung_name         TEXT NOT NULL,
    rung_index        SMALLINT NOT NULL,
    passed            BOOLEAN NOT NULL,
    score_micros      BIGINT NOT NULL DEFAULT 0,
    rejection_reason  TEXT,
    elapsed_micros    BIGINT NOT NULL DEFAULT 0,
    created_at        TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_gate_rung_results_submission
    ON trace_gate_rung_results (submission_id);
CREATE INDEX idx_gate_rung_results_tenant_rung
    ON trace_gate_rung_results (tenant_id, rung_name, created_at);

ALTER TABLE trace_gate_rung_results ENABLE ROW LEVEL SECURITY;
ALTER TABLE trace_gate_rung_results FORCE ROW LEVEL SECURITY;

CREATE POLICY trace_gate_rung_results_tenant_policy
    ON trace_gate_rung_results
    USING (tenant_id = trace_current_tenant_id());

-- Aggregate rung rejection rates for operational dashboards.
-- Materialized view refreshed by the worker on a schedule.
CREATE MATERIALIZED VIEW IF NOT EXISTS trace_gate_rung_rejection_rates AS
SELECT
    tenant_id,
    rung_name,
    date_trunc('hour', created_at) AS hour,
    COUNT(*) AS total,
    COUNT(*) FILTER (WHERE NOT passed) AS rejected,
    AVG(elapsed_micros) AS avg_elapsed_micros
FROM trace_gate_rung_results
GROUP BY tenant_id, rung_name, date_trunc('hour', created_at);

CREATE UNIQUE INDEX idx_rung_rejection_rates_pk
    ON trace_gate_rung_rejection_rates (tenant_id, rung_name, hour);
```

---

## 3. HDC Fingerprinting

**Priority: P1** | **Complexity: Medium** | **Estimated effort: 2-3 weeks**

### What and why

Hyperdimensional Computing (HDC) provides a fundamentally different approach to
trace representation than dense embeddings. A MAP-B (Multiply-Add-Permute,
Binary) vector at 10,240 bits is:

- **Fast**: novelty queries via Hamming distance are O(n/64) with POPCNT
  intrinsics, orders of magnitude faster than cosine similarity on 256-dim
  float vectors.
- **Compositional**: role-filler binding lets you represent "tool=Bash AND
  argument_pattern=git_commit" as a single vector operation, preserving
  structural information that bag-of-tokens embeddings lose.
- **Privacy-preserving**: HDC vectors are inherently lossy projections. You
  cannot reconstruct the original trace from its fingerprint -- important for a
  system that handles contributed agent traces.
- **Complementary**: use HDC for fast pre-filtering (the MinHash-LSH rung in
  the pipeline) and dense embeddings for semantic depth.

### Files to extend

- New file: `crates/trace-commons-gate-enclave/src/hdc.rs`
- `crates/trace-commons-gate-enclave/src/lib.rs` -- add `pub mod hdc;`
- `crates/trace-commons-gate-api/src/lib.rs` -- add fingerprint type
- `crates/trace-commons-server/src/trace_corpus_storage.rs` -- add fingerprint
  column to submission records

### Rust implementation

```rust
// crates/trace-commons-gate-enclave/src/hdc.rs

/// Dimensionality of the binary hyperdimensional vector, in bits.
/// 10,240 = 160 x 64-bit words. High enough for low collision probability
/// under random projection, low enough for fast Hamming distance via POPCNT.
pub const HDC_DIM_BITS: usize = 10_240;
pub const HDC_DIM_WORDS: usize = HDC_DIM_BITS / 64;

/// A binary hyperdimensional vector stored as a packed array of u64 words.
/// Supports bind (XOR), bundle (majority vote), and permute (circular shift)
/// operations following the MAP-B algebra.
#[derive(Clone, PartialEq, Eq)]
pub struct HdcFingerprint {
    pub words: [u64; HDC_DIM_WORDS],
}

impl HdcFingerprint {
    pub fn zero() -> Self {
        Self { words: [0u64; HDC_DIM_WORDS] }
    }

    /// Generate a pseudo-random base vector from a seed string.
    /// Uses SHA-256 in counter mode to fill the vector deterministically.
    /// Identical seeds always produce identical vectors.
    pub fn from_seed(seed: &str) -> Self {
        use sha2::{Digest, Sha256};
        let mut words = [0u64; HDC_DIM_WORDS];
        let needed_bytes = HDC_DIM_WORDS * 8;
        let mut bytes = Vec::with_capacity(needed_bytes);
        let mut counter = 0u32;
        while bytes.len() < needed_bytes {
            let mut h = Sha256::new();
            h.update(b"tc_hdc_v1\n");
            h.update(counter.to_be_bytes());
            h.update(b"\n");
            h.update(seed.as_bytes());
            bytes.extend_from_slice(&h.finalize());
            counter += 1;
        }
        for (i, word) in words.iter_mut().enumerate() {
            let offset = i * 8;
            let mut buf = [0u8; 8];
            buf.copy_from_slice(&bytes[offset..offset + 8]);
            *word = u64::from_le_bytes(buf);
        }
        Self { words }
    }

    /// Bind operation (XOR). Used for role-filler binding:
    /// `bind(role_vector, filler_vector)` creates a composite that is
    /// approximately orthogonal to both inputs.
    pub fn bind(&self, other: &Self) -> Self {
        let mut result = Self::zero();
        for i in 0..HDC_DIM_WORDS {
            result.words[i] = self.words[i] ^ other.words[i];
        }
        result
    }

    /// Circular permute by `positions` bits. Used to create ordered
    /// sequences: permute(v, 1) for position 1, permute(v, 2) for
    /// position 2, etc.
    pub fn permute(&self, positions: usize) -> Self {
        let positions = positions % HDC_DIM_BITS;
        if positions == 0 {
            return self.clone();
        }
        let word_shift = positions / 64;
        let bit_shift = positions % 64;
        let mut result = Self::zero();
        for i in 0..HDC_DIM_WORDS {
            let src_idx = (i + HDC_DIM_WORDS - word_shift) % HDC_DIM_WORDS;
            if bit_shift == 0 {
                result.words[i] = self.words[src_idx];
            } else {
                let prev_idx = (src_idx + HDC_DIM_WORDS - 1) % HDC_DIM_WORDS;
                result.words[i] = (self.words[src_idx] << bit_shift)
                    | (self.words[prev_idx] >> (64 - bit_shift));
            }
        }
        result
    }

    /// Hamming distance: count of differing bits.
    pub fn hamming_distance(&self, other: &Self) -> u32 {
        let mut dist = 0u32;
        for i in 0..HDC_DIM_WORDS {
            dist += (self.words[i] ^ other.words[i]).count_ones();
        }
        dist
    }

    /// Normalized Hamming distance in [0.0, 1.0].
    pub fn normalized_distance(&self, other: &Self) -> f64 {
        self.hamming_distance(other) as f64 / HDC_DIM_BITS as f64
    }

    /// Cosine-like similarity in [-1.0, 1.0] derived from Hamming distance.
    /// `1 - 2 * normalized_distance`.
    pub fn similarity(&self, other: &Self) -> f64 {
        1.0 - 2.0 * self.normalized_distance(other)
    }

    /// Serialize to bytes for storage (1280 bytes = 160 words * 8 bytes).
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(HDC_DIM_WORDS * 8);
        for word in &self.words {
            bytes.extend_from_slice(&word.to_le_bytes());
        }
        bytes
    }

    /// Deserialize from bytes.
    pub fn from_bytes(bytes: &[u8]) -> anyhow::Result<Self> {
        anyhow::ensure!(
            bytes.len() == HDC_DIM_WORDS * 8,
            "HDC fingerprint must be {} bytes, got {}",
            HDC_DIM_WORDS * 8,
            bytes.len()
        );
        let mut words = [0u64; HDC_DIM_WORDS];
        for (i, word) in words.iter_mut().enumerate() {
            let offset = i * 8;
            let mut buf = [0u8; 8];
            buf.copy_from_slice(&bytes[offset..offset + 8]);
            *word = u64::from_le_bytes(buf);
        }
        Ok(Self { words })
    }
}

impl std::fmt::Debug for HdcFingerprint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let ones: u32 = self.words.iter().map(|w| w.count_ones()).sum();
        write!(f, "HdcFingerprint({ones}/{HDC_DIM_BITS} ones)")
    }
}

/// Accumulator for computing the bundle (majority vote) of multiple
/// binary HDC vectors. Used to build corpus prototype vectors for O(1)
/// novelty queries.
///
/// The bundle of N vectors has a 1 at bit position i if more than N/2
/// of the input vectors had a 1 there. This is the HDC analog of a
/// centroid in continuous space.
pub struct BundleAccumulator {
    /// Signed accumulator: +1 for each 1-bit, -1 for each 0-bit.
    accum: Vec<i32>,
    count: u64,
}

impl BundleAccumulator {
    pub fn new() -> Self {
        Self {
            accum: vec![0i32; HDC_DIM_BITS],
            count: 0,
        }
    }

    /// Add a vector to the accumulator.
    pub fn add(&mut self, v: &HdcFingerprint) {
        for (word_idx, &word) in v.words.iter().enumerate() {
            for bit in 0..64 {
                let global_bit = word_idx * 64 + bit;
                if global_bit >= HDC_DIM_BITS {
                    break;
                }
                if (word >> bit) & 1 == 1 {
                    self.accum[global_bit] += 1;
                } else {
                    self.accum[global_bit] -= 1;
                }
            }
        }
        self.count += 1;
    }

    /// Compute the bundled prototype vector (majority vote).
    pub fn to_fingerprint(&self) -> HdcFingerprint {
        let mut result = HdcFingerprint::zero();
        for (bit_idx, &val) in self.accum.iter().enumerate() {
            if val > 0 {
                let word_idx = bit_idx / 64;
                let bit = bit_idx % 64;
                result.words[word_idx] |= 1u64 << bit;
            }
        }
        result
    }

    pub fn count(&self) -> u64 {
        self.count
    }
}

/// Fingerprint a trace's rendered events using role-filler binding.
///
/// For each event: `bind(role_vec(event_type), filler_vec(content_hash))`
/// then permute by position index and bundle all events together.
///
/// This produces a single 10,240-bit vector that captures:
/// - Which tool types appear (via role vectors)
/// - What content patterns appear (via filler vectors)
/// - What order they appear in (via positional permutation)
pub fn fingerprint_trace(rendered_events: &[String]) -> HdcFingerprint {
    if rendered_events.is_empty() {
        return HdcFingerprint::zero();
    }

    let mut accum = BundleAccumulator::new();

    for (position, event_text) in rendered_events.iter().enumerate() {
        // Extract role from the rendered event text (e.g., "tool_call (Bash): ...")
        let role = event_text
            .split(':')
            .next()
            .unwrap_or("unknown")
            .trim();

        // Content hash for the filler vector (privacy-preserving: we hash,
        // not store, the content).
        let content_hash = {
            use sha2::{Digest, Sha256};
            let mut h = Sha256::new();
            h.update(event_text.as_bytes());
            hex::encode(h.finalize())
        };

        let role_vec = HdcFingerprint::from_seed(&format!("role:{role}"));
        let filler_vec = HdcFingerprint::from_seed(&format!("filler:{content_hash}"));
        let bound = role_vec.bind(&filler_vec);
        let positioned = bound.permute(position);
        accum.add(&positioned);
    }

    accum.to_fingerprint()
}
```

#### PostgreSQL migration: V44__hdc_fingerprints.sql

```sql
-- HDC fingerprints for trace submissions. 1280 bytes per fingerprint
-- (10,240 bits / 8). Stored as BYTEA for raw Hamming distance queries.
ALTER TABLE trace_corpus_submissions
    ADD COLUMN IF NOT EXISTS hdc_fingerprint BYTEA;

-- Index for bulk scans (used by the dream consolidation batch job).
-- B-tree on the first 8 bytes provides partition-level pruning.
CREATE INDEX IF NOT EXISTS idx_submissions_hdc_prefix
    ON trace_corpus_submissions (substring(hdc_fingerprint from 1 for 8))
    WHERE hdc_fingerprint IS NOT NULL;

-- Corpus prototype vectors per tenant (bundled from all accepted traces).
-- Updated incrementally by the adaptive threshold worker.
CREATE TABLE trace_hdc_corpus_prototypes (
    tenant_id       UUID PRIMARY KEY REFERENCES trace_tenants(id),
    prototype_bytes BYTEA NOT NULL,
    trace_count     BIGINT NOT NULL DEFAULT 0,
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

ALTER TABLE trace_hdc_corpus_prototypes ENABLE ROW LEVEL SECURITY;
ALTER TABLE trace_hdc_corpus_prototypes FORCE ROW LEVEL SECURITY;

CREATE POLICY trace_hdc_corpus_prototypes_tenant_policy
    ON trace_hdc_corpus_prototypes
    USING (tenant_id = trace_current_tenant_id());
```

---

## 4. Dream Consolidation / Offline Learning

**Priority: P1** | **Complexity: High** | **Estimated effort: 5-6 weeks**

### What and why

TraceCommons processes traces in real-time, but the most valuable insights
emerge from looking at the corpus holistically. Dream consolidation is an
offline batch process -- inspired by Complementary Learning Systems theory
(McClelland et al., 1995) -- that runs during low-traffic periods to:

- **Cluster traces** into behavioral patterns (HDBSCAN over embeddings).
- **Extract patterns** from clusters (common tool sequences, error patterns,
  successful strategies).
- **Recalibrate novelty baselines** based on cluster distribution shifts.
- **Build a knowledge graph** of agent behaviors and tool interactions.
- **Pre-compute** expensive operations (embeddings, cluster assignments) for
  the next scoring cycle.

The "sleep-time compute" concept (Lin et al., 2025) applies directly: use
off-peak hours to do work that makes peak-hour scoring faster and more
accurate.

### Files to extend

- New file: `crates/trace-commons-server/src/dream/mod.rs`
- New file: `crates/trace-commons-server/src/dream/clustering.rs`
- New file: `crates/trace-commons-server/src/dream/pattern_extraction.rs`
- New file: `crates/trace-commons-server/src/dream/novelty_recalibration.rs`
- New file: `crates/trace-commons-server/src/dream/scheduling.rs`
- `crates/trace-commons-server/src/lib.rs` -- add `pub mod dream;`
- `crates/trace-commons-server/src/bin/trace-commons-worker.rs` -- add dream
  consolidation worker command

### Rust implementation

#### Core types

```rust
// crates/trace-commons-server/src/dream/mod.rs

pub mod clustering;
pub mod novelty_recalibration;
pub mod pattern_extraction;
pub mod scheduling;

use chrono::{DateTime, Utc};
use uuid::Uuid;

/// A single dream consolidation run.
#[derive(Debug, Clone)]
pub struct DreamRun {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub status: DreamRunStatus,
    /// Number of traces processed in this run.
    pub traces_processed: u64,
    /// Number of clusters discovered or updated.
    pub clusters_affected: u64,
    /// Number of patterns extracted.
    pub patterns_extracted: u64,
    /// Summary of novelty baseline changes.
    pub novelty_recalibration: Option<NoveltyRecalibrationSummary>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DreamRunStatus {
    Running,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone)]
pub struct NoveltyRecalibrationSummary {
    pub old_ema_micros: u64,
    pub new_ema_micros: u64,
    pub clusters_merged: u32,
    pub clusters_split: u32,
    pub outliers_reclassified: u32,
}

/// A discovered behavioral pattern.
#[derive(Debug, Clone)]
pub struct DreamPattern {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub cluster_id: Uuid,
    pub pattern_type: PatternType,
    /// Canonical representation of the pattern (e.g., a tool sequence
    /// template, an error signature).
    pub canonical_form: String,
    /// How many traces in the cluster exhibit this pattern.
    pub frequency: u64,
    /// Confidence that this pattern is real (not noise). Derived from
    /// cluster density and pattern frequency.
    pub confidence: f64,
    pub discovered_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PatternType {
    /// Recurring tool call sequence (e.g., Read -> Edit -> Test).
    ToolSequence,
    /// Common error pattern (e.g., permission denied -> retry with sudo).
    ErrorRecovery,
    /// Successful problem-solving strategy.
    SolutionStrategy,
    /// Anti-pattern (correlates with negative outcomes).
    AntiPattern,
}

/// A behavioral cluster discovered by HDBSCAN.
#[derive(Debug, Clone)]
pub struct DreamCluster {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub centroid_embedding: Vec<f32>,
    /// HDC prototype for fast Hamming-distance membership queries.
    pub hdc_prototype: Option<Vec<u8>>,
    pub member_count: u64,
    /// HDBSCAN cluster stability score.
    pub stability: f64,
    /// Human-readable label (generated by pattern extraction).
    pub label: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
```

#### HDBSCAN clustering

```rust
// crates/trace-commons-server/src/dream/clustering.rs

use uuid::Uuid;

/// Pairwise distance matrix entry. HDBSCAN needs the full distance matrix
/// for the mutual reachability graph; for large corpora we subsample.
#[derive(Debug, Clone, Copy)]
pub struct DistanceEntry {
    pub i: usize,
    pub j: usize,
    pub distance: f32,
}

/// Configuration for the HDBSCAN clustering step.
#[derive(Debug, Clone)]
pub struct HdbscanConfig {
    /// Minimum cluster size. Clusters smaller than this are noise.
    pub min_cluster_size: usize,
    /// Minimum samples for core distance. Controls cluster density.
    pub min_samples: usize,
    /// Maximum traces to process per dream run. Subsampled if the corpus
    /// is larger (reservoir sampling preserves distribution).
    pub max_traces: usize,
    /// Distance metric: cosine (1 - dot product for unit-norm vectors).
    pub metric: DistanceMetric,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DistanceMetric {
    Cosine,
    Hamming,
}

impl Default for HdbscanConfig {
    fn default() -> Self {
        Self {
            min_cluster_size: 5,
            min_samples: 3,
            max_traces: 50_000,
            metric: DistanceMetric::Cosine,
        }
    }
}

/// HDBSCAN clustering result.
#[derive(Debug, Clone)]
pub struct ClusteringResult {
    /// Cluster assignments: `labels[i]` is the cluster index for trace i,
    /// or -1 for noise.
    pub labels: Vec<i32>,
    /// Number of clusters found (excluding noise).
    pub n_clusters: usize,
    /// Per-cluster stability scores.
    pub stabilities: Vec<f64>,
    /// Per-cluster member counts.
    pub sizes: Vec<usize>,
}

/// Run HDBSCAN over the provided embeddings.
///
/// This is a simplified implementation suitable for moderate corpus sizes
/// (up to ~50K traces). For larger corpora, use the subsampled variant
/// with core-distance approximation.
pub fn hdbscan_cluster(
    embeddings: &[Vec<f32>],
    config: &HdbscanConfig,
) -> ClusteringResult {
    let n = embeddings.len();
    if n < config.min_cluster_size {
        return ClusteringResult {
            labels: vec![-1; n],
            n_clusters: 0,
            stabilities: vec![],
            sizes: vec![],
        };
    }

    // Step 1: Compute core distances (distance to k-th nearest neighbor).
    let mut core_distances = vec![0.0f32; n];
    for i in 0..n {
        let mut dists: Vec<f32> = (0..n)
            .filter(|&j| j != i)
            .map(|j| pairwise_distance(&embeddings[i], &embeddings[j], config.metric))
            .collect();
        dists.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        core_distances[i] = dists[config.min_samples.min(dists.len()).saturating_sub(1)];
    }

    // Step 2: Compute mutual reachability distances.
    // Step 3: Build minimum spanning tree (Prim's algorithm).
    // Step 4: Build cluster hierarchy (single-linkage dendrogram).
    // Step 5: Extract flat clusters using HDBSCAN stability criterion.
    //
    // Full implementation omitted for brevity -- the key data structures
    // are defined above. A production implementation would use the
    // `linfa-clustering` crate or a purpose-built MST + dendrogram.

    // Placeholder: return noise labels (real implementation replaces this).
    ClusteringResult {
        labels: vec![-1; n],
        n_clusters: 0,
        stabilities: vec![],
        sizes: vec![],
    }
}

fn pairwise_distance(a: &[f32], b: &[f32], metric: DistanceMetric) -> f32 {
    match metric {
        DistanceMetric::Cosine => {
            let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
            1.0 - dot // Assumes unit-norm vectors.
        }
        DistanceMetric::Hamming => {
            // Treat f32 vectors as feature presence (> 0.5 = present).
            let mismatches: usize = a
                .iter()
                .zip(b.iter())
                .filter(|(&x, &y)| (x > 0.5) != (y > 0.5))
                .count();
            mismatches as f32 / a.len() as f32
        }
    }
}
```

#### PostgreSQL migration: V45__dream_consolidation.sql

```sql
-- Dream consolidation run log.
CREATE TABLE trace_dream_runs (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id           UUID NOT NULL REFERENCES trace_tenants(id),
    started_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    completed_at        TIMESTAMPTZ,
    status              TEXT NOT NULL DEFAULT 'running'
                        CHECK (status IN ('running', 'completed', 'failed', 'cancelled')),
    traces_processed    BIGINT NOT NULL DEFAULT 0,
    clusters_affected   BIGINT NOT NULL DEFAULT 0,
    patterns_extracted  BIGINT NOT NULL DEFAULT 0,
    recalibration_json  JSONB,
    error_message       TEXT
);

ALTER TABLE trace_dream_runs ENABLE ROW LEVEL SECURITY;
ALTER TABLE trace_dream_runs FORCE ROW LEVEL SECURITY;
CREATE POLICY trace_dream_runs_tenant_policy
    ON trace_dream_runs USING (tenant_id = trace_current_tenant_id());

-- Discovered behavioral clusters.
CREATE TABLE trace_dream_clusters (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id           UUID NOT NULL REFERENCES trace_tenants(id),
    dream_run_id        UUID NOT NULL REFERENCES trace_dream_runs(id),
    centroid_embedding  BYTEA,
    hdc_prototype       BYTEA,
    member_count        BIGINT NOT NULL DEFAULT 0,
    stability           DOUBLE PRECISION NOT NULL DEFAULT 0.0,
    label               TEXT,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_dream_clusters_tenant
    ON trace_dream_clusters (tenant_id, created_at DESC);

ALTER TABLE trace_dream_clusters ENABLE ROW LEVEL SECURITY;
ALTER TABLE trace_dream_clusters FORCE ROW LEVEL SECURITY;
CREATE POLICY trace_dream_clusters_tenant_policy
    ON trace_dream_clusters USING (tenant_id = trace_current_tenant_id());

-- Extracted behavioral patterns.
CREATE TABLE trace_dream_patterns (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id           UUID NOT NULL REFERENCES trace_tenants(id),
    cluster_id          UUID NOT NULL REFERENCES trace_dream_clusters(id),
    pattern_type        TEXT NOT NULL CHECK (pattern_type IN (
        'tool_sequence', 'error_recovery', 'solution_strategy', 'anti_pattern'
    )),
    canonical_form      TEXT NOT NULL,
    frequency           BIGINT NOT NULL DEFAULT 0,
    confidence          DOUBLE PRECISION NOT NULL DEFAULT 0.0,
    discovered_at       TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_dream_patterns_cluster
    ON trace_dream_patterns (cluster_id);
CREATE INDEX idx_dream_patterns_tenant_type
    ON trace_dream_patterns (tenant_id, pattern_type, confidence DESC);

ALTER TABLE trace_dream_patterns ENABLE ROW LEVEL SECURITY;
ALTER TABLE trace_dream_patterns FORCE ROW LEVEL SECURITY;
CREATE POLICY trace_dream_patterns_tenant_policy
    ON trace_dream_patterns USING (tenant_id = trace_current_tenant_id());

-- Cluster membership (which submissions belong to which cluster).
CREATE TABLE trace_dream_cluster_members (
    submission_id       UUID NOT NULL,
    cluster_id          UUID NOT NULL REFERENCES trace_dream_clusters(id),
    tenant_id           UUID NOT NULL,
    distance_to_centroid DOUBLE PRECISION NOT NULL,
    assigned_at         TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (submission_id, cluster_id)
);

CREATE INDEX idx_dream_cluster_members_cluster
    ON trace_dream_cluster_members (cluster_id);

ALTER TABLE trace_dream_cluster_members ENABLE ROW LEVEL SECURITY;
ALTER TABLE trace_dream_cluster_members FORCE ROW LEVEL SECURITY;
CREATE POLICY trace_dream_cluster_members_tenant_policy
    ON trace_dream_cluster_members USING (tenant_id = trace_current_tenant_id());
```

#### Worker integration

Add to `crates/trace-commons-server/src/bin/trace-commons-worker.rs`:

```rust
/// Dream consolidation subcommand. Runs as a scheduled worker job
/// (cron or Kubernetes CronJob) during off-peak hours.
#[derive(clap::Args, Debug)]
struct DreamConsolidateArgs {
    /// Maximum traces to process per run.
    #[arg(long, default_value = "50000")]
    max_traces: usize,
    /// HDBSCAN minimum cluster size.
    #[arg(long, default_value = "5")]
    min_cluster_size: usize,
    /// Dry run: compute clusters but don't persist.
    #[arg(long)]
    dry_run: bool,
}
```

---

## 5. Self-Learning Mechanisms

**Priority: P1** | **Complexity: Medium** | **Estimated effort: 3-4 weeks**

### What and why

TraceCommons uses fixed scoring configurations today. Self-learning lets the
system improve its own scoring quality over time by:

- **Predictive routing** (RouteLLM-style): learn which scorer backend works
  best for which trace type, routing cheap traces to the reference scorer and
  expensive traces to the full LLM.
- **Prompt evolution**: track which system prompts yield the best scorer
  accuracy and evolve them.
- **A/B testing**: run multiple scoring configurations in parallel and
  converge on the best one.
- **Efficiency tracking**: measure tokens spent vs. quality gained per scoring
  run to optimize cost/quality tradeoffs.
- **LinUCB bandits**: per-language, per-agent-model arm selection for scorer
  configuration.

### Files to extend

- New file: `crates/trace-commons-gate-enclave/src/scorer_routing.rs`
- New file: `crates/trace-commons-server/src/learning/mod.rs`
- New file: `crates/trace-commons-server/src/learning/bandit.rs`
- New file: `crates/trace-commons-server/src/learning/ab_testing.rs`
- New file: `crates/trace-commons-server/src/learning/efficiency.rs`
- `crates/trace-commons-server/src/lib.rs` -- add `pub mod learning;`

### Rust implementation

#### LinUCB Bandit for scorer selection

```rust
// crates/trace-commons-server/src/learning/bandit.rs

use std::collections::HashMap;

/// Linear Upper Confidence Bound bandit (Li et al., 2010).
///
/// Each "arm" is a scorer configuration. The context vector encodes trace
/// features (language, agent model, trace length, tool diversity). The
/// bandit learns a linear reward model per arm and balances exploration
/// (trying uncertain arms) with exploitation (using the best-known arm).
#[derive(Debug, Clone)]
pub struct LinUcbBandit {
    /// Per-arm state: (A_inverse, b) where A = d x d matrix, b = d x 1 vector.
    /// A_inverse is stored directly (updated via Sherman-Morrison) to avoid
    /// repeated matrix inversion.
    arms: HashMap<String, LinUcbArm>,
    /// Context dimensionality.
    d: usize,
    /// Exploration parameter. Higher = more exploration. Typical: 0.1 - 2.0.
    alpha: f64,
}

#[derive(Debug, Clone)]
struct LinUcbArm {
    /// Inverse of the design matrix A = I_d + sum(x_t * x_t^T).
    a_inv: Vec<Vec<f64>>,
    /// Reward-weighted context sum b = sum(r_t * x_t).
    b: Vec<f64>,
    /// Number of times this arm has been pulled.
    pulls: u64,
    /// Cumulative reward.
    total_reward: f64,
}

impl LinUcbBandit {
    /// Create a new bandit with the given context dimensionality and
    /// exploration parameter.
    pub fn new(d: usize, alpha: f64) -> Self {
        Self {
            arms: HashMap::new(),
            d,
            alpha,
        }
    }

    /// Ensure an arm exists for the given scorer configuration.
    pub fn register_arm(&mut self, arm_id: &str) {
        if !self.arms.contains_key(arm_id) {
            // Initialize A_inv = I_d (identity matrix).
            let mut a_inv = vec![vec![0.0; self.d]; self.d];
            for i in 0..self.d {
                a_inv[i][i] = 1.0;
            }
            self.arms.insert(
                arm_id.to_string(),
                LinUcbArm {
                    a_inv,
                    b: vec![0.0; self.d],
                    pulls: 0,
                    total_reward: 0.0,
                },
            );
        }
    }

    /// Select the best arm given a context vector. Returns the arm ID
    /// and its upper confidence bound.
    pub fn select(&self, context: &[f64]) -> Option<(String, f64)> {
        assert_eq!(context.len(), self.d);
        let mut best_arm = None;
        let mut best_ucb = f64::NEG_INFINITY;

        for (arm_id, arm) in &self.arms {
            // theta_hat = A_inv * b
            let theta: Vec<f64> = mat_vec_mul(&arm.a_inv, &arm.b);
            // p = theta^T * x + alpha * sqrt(x^T * A_inv * x)
            let predicted = dot(&theta, context);
            let a_inv_x = mat_vec_mul(&arm.a_inv, context);
            let uncertainty = dot(context, &a_inv_x).sqrt();
            let ucb = predicted + self.alpha * uncertainty;

            if ucb > best_ucb {
                best_ucb = ucb;
                best_arm = Some(arm_id.clone());
            }
        }

        best_arm.map(|id| (id, best_ucb))
    }

    /// Update the arm with an observed reward for the given context.
    pub fn update(&mut self, arm_id: &str, context: &[f64], reward: f64) {
        assert_eq!(context.len(), self.d);
        let arm = self.arms.get_mut(arm_id).expect("unknown arm");
        arm.pulls += 1;
        arm.total_reward += reward;

        // Sherman-Morrison update: A_inv -= (A_inv * x * x^T * A_inv) / (1 + x^T * A_inv * x)
        let a_inv_x = mat_vec_mul(&arm.a_inv, context);
        let denom = 1.0 + dot(context, &a_inv_x);
        for i in 0..self.d {
            for j in 0..self.d {
                arm.a_inv[i][j] -= a_inv_x[i] * a_inv_x[j] / denom;
            }
        }

        // b += reward * x
        for i in 0..self.d {
            arm.b[i] += reward * context[i];
        }
    }
}

fn dot(a: &[f64], b: &[f64]) -> f64 {
    a.iter().zip(b.iter()).map(|(x, y)| x * y).sum()
}

fn mat_vec_mul(mat: &[Vec<f64>], vec: &[f64]) -> Vec<f64> {
    mat.iter().map(|row| dot(row, vec)).collect()
}

/// Build a context vector from trace features for the scorer bandit.
///
/// Dimensions (d=8):
/// 0: log(trace_length_bytes) / 20.0  (normalized)
/// 1: event_count / 100.0             (normalized)
/// 2: unique_tool_count / 20.0        (normalized)
/// 3: is_code_heavy (0 or 1)
/// 4: is_conversation_heavy (0 or 1)
/// 5: estimated_language_diversity (0-1)
/// 6: has_error_events (0 or 1)
/// 7: bias term (always 1.0)
pub fn trace_context_vector(
    trace_length_bytes: usize,
    event_count: usize,
    unique_tool_count: usize,
    is_code_heavy: bool,
    is_conversation_heavy: bool,
    language_diversity: f64,
    has_error_events: bool,
) -> Vec<f64> {
    vec![
        (trace_length_bytes as f64).ln().max(0.0) / 20.0,
        event_count as f64 / 100.0,
        unique_tool_count as f64 / 20.0,
        if is_code_heavy { 1.0 } else { 0.0 },
        if is_conversation_heavy { 1.0 } else { 0.0 },
        language_diversity.clamp(0.0, 1.0),
        if has_error_events { 1.0 } else { 0.0 },
        1.0, // Bias term.
    ]
}
```

#### A/B testing framework

```rust
// crates/trace-commons-server/src/learning/ab_testing.rs

use chrono::{DateTime, Utc};
use uuid::Uuid;

/// An A/B experiment comparing two or more scoring configurations.
#[derive(Debug, Clone)]
pub struct ScoringExperiment {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub name: String,
    pub status: ExperimentStatus,
    pub variants: Vec<ExperimentVariant>,
    pub traffic_split: Vec<f64>,
    pub primary_metric: PrimaryMetric,
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub concluded_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExperimentStatus {
    Draft,
    Running,
    Concluded,
    Cancelled,
}

#[derive(Debug, Clone)]
pub struct ExperimentVariant {
    pub id: Uuid,
    pub name: String,
    pub config_overrides: ScoringConfigOverrides,
    pub observations: u64,
    pub metric_sum: f64,
    pub metric_sum_sq: f64,
}

#[derive(Debug, Clone)]
pub struct ScoringConfigOverrides {
    pub scorer_backend: Option<String>,
    pub perplexity_floor_micros: Option<u64>,
    pub novelty_floor_micros: Option<u64>,
    pub chunk_target_tokens: Option<usize>,
    pub ema_alpha: Option<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrimaryMetric {
    /// Credit quality score of accepted traces.
    CreditQuality,
    /// Acceptance rate (higher may not be better).
    AcceptanceRate,
    /// Cost per accepted trace (tokens spent / accepted count).
    CostEfficiency,
    /// Scorer agreement rate (when multiple scorers are available).
    ScorerAgreement,
}

impl ExperimentVariant {
    /// Record an observation and update running statistics.
    pub fn record(&mut self, metric_value: f64) {
        self.observations += 1;
        self.metric_sum += metric_value;
        self.metric_sum_sq += metric_value * metric_value;
    }

    /// Sample mean of the metric.
    pub fn mean(&self) -> f64 {
        if self.observations == 0 {
            return 0.0;
        }
        self.metric_sum / self.observations as f64
    }

    /// Sample standard error of the mean.
    pub fn std_error(&self) -> f64 {
        if self.observations < 2 {
            return f64::INFINITY;
        }
        let n = self.observations as f64;
        let variance = (self.metric_sum_sq - self.metric_sum * self.metric_sum / n) / (n - 1.0);
        (variance / n).sqrt()
    }
}

/// Welch's t-test for comparing two experiment variants.
/// Returns (t_statistic, approximate_p_value, significant_at_alpha).
pub fn welch_t_test(a: &ExperimentVariant, b: &ExperimentVariant, alpha: f64) -> (f64, f64, bool) {
    let mean_diff = a.mean() - b.mean();
    let se = (a.std_error().powi(2) + b.std_error().powi(2)).sqrt();
    if se == 0.0 || !se.is_finite() {
        return (0.0, 1.0, false);
    }
    let t = mean_diff / se;
    // Approximate p-value using the normal distribution (valid for
    // large sample sizes; use a t-distribution table for small n).
    let p = 2.0 * (1.0 - normal_cdf(t.abs()));
    (t, p, p < alpha)
}

/// Standard normal CDF approximation (Abramowitz & Stegun, 1964).
fn normal_cdf(x: f64) -> f64 {
    let t = 1.0 / (1.0 + 0.2316419 * x.abs());
    let d = 0.3989422804014327; // 1/sqrt(2*pi)
    let p = d * (-x * x / 2.0).exp();
    let poly = t * (0.319381530 + t * (-0.356563782 + t * (1.781477937 + t * (-1.821255978 + t * 1.330274429))));
    if x >= 0.0 {
        1.0 - p * poly
    } else {
        p * poly
    }
}
```

#### PostgreSQL migration: V46__self_learning.sql

```sql
-- Scorer routing bandit state, serialized per tenant.
CREATE TABLE trace_scorer_bandit_state (
    tenant_id           UUID PRIMARY KEY REFERENCES trace_tenants(id),
    bandit_json         JSONB NOT NULL,
    context_dim         INT NOT NULL DEFAULT 8,
    alpha               DOUBLE PRECISION NOT NULL DEFAULT 1.0,
    total_pulls         BIGINT NOT NULL DEFAULT 0,
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT now()
);

ALTER TABLE trace_scorer_bandit_state ENABLE ROW LEVEL SECURITY;
ALTER TABLE trace_scorer_bandit_state FORCE ROW LEVEL SECURITY;
CREATE POLICY trace_scorer_bandit_state_tenant_policy
    ON trace_scorer_bandit_state
    USING (tenant_id = trace_current_tenant_id());

-- A/B experiment definitions and results.
CREATE TABLE trace_scoring_experiments (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id           UUID NOT NULL REFERENCES trace_tenants(id),
    name                TEXT NOT NULL,
    status              TEXT NOT NULL DEFAULT 'draft'
                        CHECK (status IN ('draft', 'running', 'concluded', 'cancelled')),
    primary_metric      TEXT NOT NULL DEFAULT 'credit_quality',
    traffic_split       JSONB NOT NULL DEFAULT '[]',
    created_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    started_at          TIMESTAMPTZ,
    concluded_at        TIMESTAMPTZ
);

ALTER TABLE trace_scoring_experiments ENABLE ROW LEVEL SECURITY;
ALTER TABLE trace_scoring_experiments FORCE ROW LEVEL SECURITY;
CREATE POLICY trace_scoring_experiments_tenant_policy
    ON trace_scoring_experiments
    USING (tenant_id = trace_current_tenant_id());

CREATE TABLE trace_scoring_experiment_variants (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    experiment_id       UUID NOT NULL REFERENCES trace_scoring_experiments(id),
    tenant_id           UUID NOT NULL,
    name                TEXT NOT NULL,
    config_overrides    JSONB NOT NULL DEFAULT '{}',
    observations        BIGINT NOT NULL DEFAULT 0,
    metric_sum          DOUBLE PRECISION NOT NULL DEFAULT 0.0,
    metric_sum_sq       DOUBLE PRECISION NOT NULL DEFAULT 0.0,
    UNIQUE (experiment_id, name)
);

ALTER TABLE trace_scoring_experiment_variants ENABLE ROW LEVEL SECURITY;
ALTER TABLE trace_scoring_experiment_variants FORCE ROW LEVEL SECURITY;
CREATE POLICY trace_scoring_experiment_variants_tenant_policy
    ON trace_scoring_experiment_variants
    USING (tenant_id = trace_current_tenant_id());

-- Per-scoring-run efficiency tracking.
CREATE TABLE trace_scoring_efficiency (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id           UUID NOT NULL,
    submission_id       UUID NOT NULL,
    scorer_backend      TEXT NOT NULL,
    tokens_scored       BIGINT NOT NULL DEFAULT 0,
    wall_clock_micros   BIGINT NOT NULL DEFAULT 0,
    credit_quality_micros BIGINT,
    cost_estimate_micros BIGINT,
    experiment_variant_id UUID REFERENCES trace_scoring_experiment_variants(id),
    created_at          TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_scoring_efficiency_tenant
    ON trace_scoring_efficiency (tenant_id, created_at DESC);

ALTER TABLE trace_scoring_efficiency ENABLE ROW LEVEL SECURITY;
ALTER TABLE trace_scoring_efficiency FORCE ROW LEVEL SECURITY;
CREATE POLICY trace_scoring_efficiency_tenant_policy
    ON trace_scoring_efficiency
    USING (tenant_id = trace_current_tenant_id());
```

---

## 6. Stigmergic Coordination

**Priority: P2** | **Complexity: Medium** | **Estimated effort: 3-4 weeks**

### What and why

Stigmergy (Parunak, 2002) is indirect coordination through environmental
signals -- the digital equivalent of ant pheromone trails. When a contributor
submits traces in a particular domain (e.g., Rust debugging, API integration
testing), they leave a "pheromone" that:

- **Attracts** similar contributions by surfacing demand signals ("we need more
  traces like this").
- **Repels** over-represented areas by showing saturation ("we have 10,000
  hello-world traces already").
- **Builds capability maps** that visualize corpus coverage and gaps.
- **Coordinates** multiple contributors without explicit communication.

Pheromone evaporation (exponential decay) ensures the system stays responsive
to changing needs. Anti-evaporation for safety-critical patterns (security
bugs, crash recovery) prevents important but rare patterns from being forgotten.

### Files to extend

- New file: `crates/trace-commons-server/src/stigmergy/mod.rs`
- New file: `crates/trace-commons-server/src/stigmergy/pheromone.rs`
- New file: `crates/trace-commons-server/src/stigmergy/capability_map.rs`
- `crates/trace-commons-server/src/lib.rs` -- add `pub mod stigmergy;`

### Rust implementation

```rust
// crates/trace-commons-server/src/stigmergy/pheromone.rs

use chrono::{DateTime, Utc};
use uuid::Uuid;

/// A pheromone trail left by trace contributions in a domain.
#[derive(Debug, Clone)]
pub struct PheromoneTrail {
    pub id: Uuid,
    pub tenant_id: Uuid,
    /// Domain identifier (e.g., "rust:debugging", "python:api_testing",
    /// "security:injection_prevention"). Hierarchical, dot-separated.
    pub domain: String,
    /// Current pheromone strength in micros [0, 1_000_000].
    /// Decays exponentially; boosted by new contributions.
    pub strength_micros: i64,
    /// Base deposit per contribution. Adjusted by credit quality.
    pub deposit_rate_micros: i64,
    /// Half-life in hours. Strength halves every `half_life_hours`.
    pub half_life_hours: f64,
    /// Anti-evaporation floor: strength never drops below this.
    /// Non-zero for safety-critical domains.
    pub floor_micros: i64,
    /// Number of contributions that deposited pheromone here.
    pub contribution_count: u64,
    pub last_deposit_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

/// Pheromone configuration constants.
#[derive(Debug, Clone)]
pub struct PheromoneConfig {
    /// Default pheromone half-life in hours.
    pub default_half_life_hours: f64,
    /// Default deposit rate per contribution (micros).
    pub default_deposit_micros: i64,
    /// Deposit multiplier based on credit quality: `deposit * (1 + q * boost)`.
    pub quality_boost_factor: f64,
    /// Anti-evaporation floor for safety-critical domains (micros).
    pub safety_floor_micros: i64,
    /// Evaporation batch interval in minutes.
    pub evaporation_interval_minutes: u32,
    /// Maximum pheromone strength (prevents runaway accumulation).
    pub max_strength_micros: i64,
}

impl Default for PheromoneConfig {
    fn default() -> Self {
        Self {
            default_half_life_hours: 168.0, // 1 week.
            default_deposit_micros: 10_000, // 0.01 strength per contribution.
            quality_boost_factor: 2.0,      // High-quality traces deposit 3x.
            safety_floor_micros: 100_000,   // 0.1 floor for safety domains.
            evaporation_interval_minutes: 60,
            max_strength_micros: 1_000_000,
        }
    }
}

/// Deposit pheromone when a trace is accepted.
///
/// The deposit amount is `base_deposit * (1 + credit_quality * boost_factor)`,
/// clamped to the maximum strength.
pub fn deposit_pheromone(
    trail: &mut PheromoneTrail,
    credit_quality_micros: i64,
    config: &PheromoneConfig,
    now: DateTime<Utc>,
) {
    // First, apply evaporation since last deposit.
    let elapsed_hours = (now - trail.last_deposit_at).num_seconds() as f64 / 3600.0;
    if elapsed_hours > 0.0 {
        let decay_factor = (0.5_f64).powf(elapsed_hours / trail.half_life_hours);
        trail.strength_micros =
            ((trail.strength_micros as f64 * decay_factor) as i64).max(trail.floor_micros);
    }

    // Compute deposit.
    let quality_fraction = credit_quality_micros as f64 / 1_000_000.0;
    let deposit = trail.deposit_rate_micros as f64
        * (1.0 + quality_fraction * config.quality_boost_factor);
    trail.strength_micros = (trail.strength_micros + deposit as i64)
        .min(config.max_strength_micros);
    trail.contribution_count += 1;
    trail.last_deposit_at = now;
}

/// Apply evaporation to all trails for a tenant. Called by the
/// evaporation cron job.
pub fn evaporate_trails(
    trails: &mut [PheromoneTrail],
    now: DateTime<Utc>,
) {
    for trail in trails.iter_mut() {
        let elapsed_hours = (now - trail.last_deposit_at).num_seconds() as f64 / 3600.0;
        if elapsed_hours > 0.0 {
            let decay_factor = (0.5_f64).powf(elapsed_hours / trail.half_life_hours);
            trail.strength_micros =
                ((trail.strength_micros as f64 * decay_factor) as i64).max(trail.floor_micros);
        }
    }
}

/// Classify a trace's domain from its rendered events.
///
/// Extracts tool names, content patterns, and language indicators to
/// assign a hierarchical domain label.
pub fn classify_domain(rendered_events: &[String]) -> String {
    let mut tools: Vec<String> = Vec::new();
    let mut has_code = false;
    let mut has_error = false;

    for event in rendered_events {
        // Extract tool name from "tool_call (ToolName): content" format.
        if let Some(start) = event.find('(') {
            if let Some(end) = event.find("):") {
                let tool = event[start + 1..end].trim().to_lowercase();
                if !tools.contains(&tool) {
                    tools.push(tool);
                }
            }
        }
        let lower = event.to_lowercase();
        if lower.contains("```") || lower.contains("fn ") || lower.contains("def ") {
            has_code = true;
        }
        if lower.contains("error") || lower.contains("failed") || lower.contains("panic") {
            has_error = true;
        }
    }

    // Simple hierarchical classification.
    let primary = if has_code { "coding" } else { "conversation" };
    let secondary = if has_error { "debugging" } else { "general" };
    let tool_suffix = if tools.is_empty() {
        "no_tools".to_string()
    } else {
        tools[..tools.len().min(3)].join("+")
    };

    format!("{primary}:{secondary}:{tool_suffix}")
}
```

#### PostgreSQL migration: V47__stigmergy.sql

```sql
-- Pheromone trails for stigmergic coordination.
CREATE TABLE trace_pheromone_trails (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id           UUID NOT NULL REFERENCES trace_tenants(id),
    domain              TEXT NOT NULL,
    strength_micros     BIGINT NOT NULL DEFAULT 0,
    deposit_rate_micros BIGINT NOT NULL DEFAULT 10000,
    half_life_hours     DOUBLE PRECISION NOT NULL DEFAULT 168.0,
    floor_micros        BIGINT NOT NULL DEFAULT 0,
    contribution_count  BIGINT NOT NULL DEFAULT 0,
    last_deposit_at     TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (tenant_id, domain)
);

CREATE INDEX idx_pheromone_trails_strength
    ON trace_pheromone_trails (tenant_id, strength_micros DESC);

ALTER TABLE trace_pheromone_trails ENABLE ROW LEVEL SECURITY;
ALTER TABLE trace_pheromone_trails FORCE ROW LEVEL SECURITY;
CREATE POLICY trace_pheromone_trails_tenant_policy
    ON trace_pheromone_trails
    USING (tenant_id = trace_current_tenant_id());

-- Capability map: aggregate coverage per domain per tenant.
-- Materialized view refreshed hourly by the evaporation worker.
CREATE MATERIALIZED VIEW IF NOT EXISTS trace_capability_map AS
SELECT
    t.tenant_id,
    t.domain,
    t.strength_micros,
    t.contribution_count,
    split_part(t.domain, ':', 1) AS primary_category,
    split_part(t.domain, ':', 2) AS secondary_category,
    CASE
        WHEN t.strength_micros > 500000 THEN 'saturated'
        WHEN t.strength_micros > 100000 THEN 'well_covered'
        WHEN t.strength_micros > 10000  THEN 'sparse'
        ELSE 'gap'
    END AS coverage_level
FROM trace_pheromone_trails t
WHERE t.strength_micros > 0;

CREATE UNIQUE INDEX idx_capability_map_pk
    ON trace_capability_map (tenant_id, domain);

-- Pheromone deposit log (append-only, for auditing).
CREATE TABLE trace_pheromone_deposits (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id           UUID NOT NULL,
    trail_id            UUID NOT NULL REFERENCES trace_pheromone_trails(id),
    submission_id       UUID NOT NULL,
    deposit_micros      BIGINT NOT NULL,
    credit_quality_micros BIGINT NOT NULL,
    strength_after_micros BIGINT NOT NULL,
    deposited_at        TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_pheromone_deposits_trail
    ON trace_pheromone_deposits (trail_id, deposited_at DESC);

ALTER TABLE trace_pheromone_deposits ENABLE ROW LEVEL SECURITY;
ALTER TABLE trace_pheromone_deposits FORCE ROW LEVEL SECURITY;
CREATE POLICY trace_pheromone_deposits_tenant_policy
    ON trace_pheromone_deposits
    USING (tenant_id = trace_current_tenant_id());
```

---

## 7. Biological / Affective Mechanisms

**Priority: P2** | **Complexity: Medium-High** | **Estimated effort: 4-5 weeks**

### What and why

Biological metaphors provide effective engineering patterns for system-level
behaviors that are hard to specify explicitly:

- **Affect-modulated scoring**: compute a "trace affect" signal from behavioral
  patterns (error rates, retry frequency, completion speed). Traces with high
  negative affect (many errors, excessive retries) may indicate adversarial
  behavior or broken tooling -- information the gate should consider.
- **Somatic markers** (Damasio, 1994): flag traces that historically correlate
  with downstream problems (e.g., traces from a contributor whose submissions
  frequently get revoked).
- **Circadian scheduling**: schedule expensive scoring (full TEE) during
  off-peak hours and lightweight validation during peak hours, matching the
  natural load cycle.
- **Homeostatic regulation**: the system monitors its own health metrics
  (queue depth, latency, error rate) and adjusts throughput to maintain
  stability, similar to biological homeostasis.
- **Immune system**: anomaly detection for malicious or spam traces using
  learned "self" models.

### Files to extend

- New file: `crates/trace-commons-server/src/affect/mod.rs`
- New file: `crates/trace-commons-server/src/affect/trace_affect.rs`
- New file: `crates/trace-commons-server/src/affect/somatic_markers.rs`
- New file: `crates/trace-commons-server/src/affect/circadian.rs`
- New file: `crates/trace-commons-server/src/affect/homeostasis.rs`
- New file: `crates/trace-commons-server/src/affect/immune.rs`
- `crates/trace-commons-server/src/lib.rs` -- add `pub mod affect;`

### Rust implementation

#### Trace affect scoring

```rust
// crates/trace-commons-server/src/affect/trace_affect.rs

/// Affect dimensions for a trace, computed from behavioral signals.
/// All values in micros [0, 1_000_000] where higher = more intense.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TraceAffect {
    /// Frustration signal: error count, retry patterns, undo sequences.
    pub frustration_micros: i64,
    /// Flow signal: consistent progress, minimal backtracking.
    pub flow_micros: i64,
    /// Uncertainty signal: hesitation patterns, frequent tool switches.
    pub uncertainty_micros: i64,
    /// Urgency signal: speed of event generation, minimal pauses.
    pub urgency_micros: i64,
    /// Composite valence: positive (flow-dominant) or negative
    /// (frustration-dominant). Range [-1_000_000, 1_000_000].
    pub valence_micros: i64,
}

/// Compute affect from rendered trace events.
pub fn compute_trace_affect(events: &[TraceEvent]) -> TraceAffect {
    if events.is_empty() {
        return TraceAffect {
            frustration_micros: 0,
            flow_micros: 500_000, // Neutral.
            uncertainty_micros: 0,
            urgency_micros: 500_000,
            valence_micros: 0,
        };
    }

    let total = events.len() as f64;

    // Frustration: count error events, retries, and undo patterns.
    let error_count = events.iter().filter(|e| e.is_error).count() as f64;
    let retry_count = count_retries(events) as f64;
    let frustration = ((error_count + retry_count) / total).clamp(0.0, 1.0);

    // Flow: measure consistent forward progress (no backtracking).
    let unique_tools = events
        .iter()
        .filter_map(|e| e.tool_name.as_deref())
        .collect::<std::collections::HashSet<_>>()
        .len() as f64;
    let tool_diversity = (unique_tools / total.sqrt()).clamp(0.0, 1.0);
    let flow = (tool_diversity * (1.0 - frustration)).clamp(0.0, 1.0);

    // Uncertainty: frequent tool switches without progress.
    let switches = count_tool_switches(events) as f64;
    let uncertainty = (switches / total - 0.3).max(0.0).clamp(0.0, 1.0);

    // Urgency: inverse of mean inter-event time (if timestamps available).
    let urgency = 0.5; // Default when timestamps unavailable.

    let valence = flow - frustration;

    TraceAffect {
        frustration_micros: (frustration * 1_000_000.0).round() as i64,
        flow_micros: (flow * 1_000_000.0).round() as i64,
        uncertainty_micros: (uncertainty * 1_000_000.0).round() as i64,
        urgency_micros: (urgency * 1_000_000.0).round() as i64,
        valence_micros: (valence * 1_000_000.0).round() as i64,
    }
}

/// A simplified trace event for affect computation.
#[derive(Debug, Clone)]
pub struct TraceEvent {
    pub event_type: String,
    pub tool_name: Option<String>,
    pub is_error: bool,
    pub content_length: usize,
}

fn count_retries(events: &[TraceEvent]) -> usize {
    let mut retries = 0;
    for window in events.windows(2) {
        if window[0].tool_name == window[1].tool_name
            && window[0].tool_name.is_some()
            && window[0].is_error
        {
            retries += 1;
        }
    }
    retries
}

fn count_tool_switches(events: &[TraceEvent]) -> usize {
    let mut switches = 0;
    for window in events.windows(2) {
        if window[0].tool_name != window[1].tool_name {
            switches += 1;
        }
    }
    switches
}
```

#### Somatic markers

```rust
// crates/trace-commons-server/src/affect/somatic_markers.rs

use chrono::{DateTime, Utc};
use uuid::Uuid;

/// A somatic marker records a learned association between a trace feature
/// and a downstream outcome. Positive markers boost scoring; negative
/// markers trigger additional scrutiny.
#[derive(Debug, Clone)]
pub struct SomaticMarker {
    pub id: Uuid,
    pub tenant_id: Uuid,
    /// Feature that triggers the marker (e.g., "contributor:abc123",
    /// "tool_pattern:Bash+Read+Bash", "domain:coding:debugging").
    pub feature_key: String,
    /// Marker valence: positive (correlated with good outcomes) or
    /// negative (correlated with revocations, low quality).
    pub valence_micros: i64,
    /// Confidence based on observation count. Range [0, 1_000_000].
    pub confidence_micros: i64,
    /// Number of positive observations (accepted, not revoked).
    pub positive_observations: u64,
    /// Number of negative observations (revoked, quarantined).
    pub negative_observations: u64,
    pub last_updated_at: DateTime<Utc>,
}

/// Update a somatic marker with a new observation.
pub fn update_marker(
    marker: &mut SomaticMarker,
    positive: bool,
    now: DateTime<Utc>,
) {
    if positive {
        marker.positive_observations += 1;
    } else {
        marker.negative_observations += 1;
    }

    let total = (marker.positive_observations + marker.negative_observations) as f64;
    let positive_rate = marker.positive_observations as f64 / total;

    // Valence: centered at 0, positive rate > 0.5 = positive valence.
    marker.valence_micros = ((positive_rate - 0.5) * 2_000_000.0).round() as i64;

    // Confidence: increases with observation count, saturates via log.
    marker.confidence_micros = ((total.ln() / 10.0_f64.ln()).clamp(0.0, 1.0) * 1_000_000.0)
        .round() as i64;

    marker.last_updated_at = now;
}

/// Look up somatic markers for a trace and compute an aggregate modulation
/// factor. Returns a multiplier in [0.5, 1.5] that adjusts the credit
/// quality score.
pub fn somatic_modulation(markers: &[SomaticMarker]) -> f64 {
    if markers.is_empty() {
        return 1.0;
    }

    let weighted_sum: f64 = markers
        .iter()
        .map(|m| {
            let valence = m.valence_micros as f64 / 1_000_000.0;
            let confidence = m.confidence_micros as f64 / 1_000_000.0;
            valence * confidence
        })
        .sum();

    let avg = weighted_sum / markers.len() as f64;
    // Map [-1, 1] to [0.5, 1.5].
    (1.0 + avg * 0.5).clamp(0.5, 1.5)
}
```

#### Circadian scheduling

```rust
// crates/trace-commons-server/src/affect/circadian.rs

use chrono::{DateTime, Timelike, Utc};

/// Scoring intensity level based on time of day. Controls which gate
/// pipeline rungs are active and how deep the TEE scoring goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScoringIntensity {
    /// Full TEE scoring, dream consolidation, A/B experiments.
    /// Scheduled during off-peak hours (02:00-08:00 UTC).
    Deep,
    /// Standard pipeline: all rungs active, full TEE scoring.
    /// Scheduled during normal hours (08:00-22:00 UTC).
    Standard,
    /// Lightweight: skip TEE, use reference scorer only.
    /// Scheduled during peak hours if load exceeds threshold.
    Light,
}

/// Determine the scoring intensity for the current time and load.
pub fn current_intensity(
    now: DateTime<Utc>,
    queue_depth: usize,
    queue_depth_high_watermark: usize,
) -> ScoringIntensity {
    let hour = now.hour();

    // If queue is overloaded, drop to light regardless of time.
    if queue_depth > queue_depth_high_watermark {
        return ScoringIntensity::Light;
    }

    match hour {
        2..=7 => ScoringIntensity::Deep,
        8..=21 => ScoringIntensity::Standard,
        _ => ScoringIntensity::Standard,
    }
}

/// Circadian configuration.
#[derive(Debug, Clone)]
pub struct CircadianConfig {
    /// UTC hour range for deep processing [start, end).
    pub deep_start_hour: u32,
    pub deep_end_hour: u32,
    /// Queue depth threshold for automatic degradation to Light.
    pub queue_depth_high_watermark: usize,
    /// Whether dream consolidation runs during Deep periods.
    pub dream_during_deep: bool,
}

impl Default for CircadianConfig {
    fn default() -> Self {
        Self {
            deep_start_hour: 2,
            deep_end_hour: 8,
            queue_depth_high_watermark: 10_000,
            dream_during_deep: true,
        }
    }
}
```

#### Homeostatic regulation

```rust
// crates/trace-commons-server/src/affect/homeostasis.rs

use std::time::Duration;

/// System health metrics for homeostatic regulation.
#[derive(Debug, Clone, Copy)]
pub struct SystemVitals {
    /// Current scoring queue depth.
    pub queue_depth: usize,
    /// P99 scoring latency over the last 5 minutes.
    pub p99_latency: Duration,
    /// Error rate over the last 5 minutes (0.0 - 1.0).
    pub error_rate: f64,
    /// Memory utilization (0.0 - 1.0).
    pub memory_utilization: f64,
    /// CPU utilization (0.0 - 1.0).
    pub cpu_utilization: f64,
}

/// Homeostatic setpoints -- the "healthy" operating range.
#[derive(Debug, Clone)]
pub struct HomeostaticSetpoints {
    pub target_queue_depth: usize,
    pub max_p99_latency: Duration,
    pub max_error_rate: f64,
    pub max_memory_utilization: f64,
    pub max_cpu_utilization: f64,
}

impl Default for HomeostaticSetpoints {
    fn default() -> Self {
        Self {
            target_queue_depth: 100,
            max_p99_latency: Duration::from_secs(30),
            max_error_rate: 0.01,
            max_memory_utilization: 0.85,
            max_cpu_utilization: 0.80,
        }
    }
}

/// Homeostatic response: how the system should adjust.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HomeostaticResponse {
    /// Throughput multiplier: < 1.0 = slow down, > 1.0 = speed up.
    pub throughput_factor: f64,
    /// Whether to shed load (reject new submissions temporarily).
    pub shed_load: bool,
    /// Whether to trigger an emergency drain (process queue but
    /// accept no new work).
    pub emergency_drain: bool,
}

/// Compute the homeostatic response given current vitals and setpoints.
pub fn regulate(vitals: &SystemVitals, setpoints: &HomeostaticSetpoints) -> HomeostaticResponse {
    let mut throughput_factor = 1.0;
    let mut shed_load = false;
    let mut emergency_drain = false;

    // Queue depth regulation.
    if vitals.queue_depth > setpoints.target_queue_depth * 5 {
        shed_load = true;
        throughput_factor *= 0.5;
    } else if vitals.queue_depth > setpoints.target_queue_depth * 2 {
        throughput_factor *= 0.8;
    } else if vitals.queue_depth < setpoints.target_queue_depth / 2 {
        throughput_factor *= 1.2;
    }

    // Latency regulation.
    if vitals.p99_latency > setpoints.max_p99_latency * 2 {
        throughput_factor *= 0.5;
    } else if vitals.p99_latency > setpoints.max_p99_latency {
        throughput_factor *= 0.7;
    }

    // Error rate regulation.
    if vitals.error_rate > setpoints.max_error_rate * 10.0 {
        emergency_drain = true;
    } else if vitals.error_rate > setpoints.max_error_rate {
        throughput_factor *= 0.6;
    }

    // Memory regulation.
    if vitals.memory_utilization > 0.95 {
        emergency_drain = true;
    } else if vitals.memory_utilization > setpoints.max_memory_utilization {
        throughput_factor *= 0.7;
    }

    // CPU regulation.
    if vitals.cpu_utilization > setpoints.max_cpu_utilization {
        throughput_factor *= 0.8;
    }

    HomeostaticResponse {
        throughput_factor: throughput_factor.clamp(0.1, 2.0),
        shed_load,
        emergency_drain,
    }
}
```

#### Immune system: anomaly detection

```rust
// crates/trace-commons-server/src/affect/immune.rs

use uuid::Uuid;

/// An immune "memory cell" -- a learned pattern of malicious or spam traces.
#[derive(Debug, Clone)]
pub struct ImmuneMemoryCell {
    pub id: Uuid,
    pub tenant_id: Uuid,
    /// The anomaly signature (e.g., "rapid_submission:>100/min",
    /// "identical_content_hash:abc123", "known_spam_pattern:lorem_ipsum").
    pub signature: String,
    /// How many times this signature has been observed.
    pub observation_count: u64,
    /// Whether this is confirmed malicious (vs. suspected).
    pub confirmed: bool,
    /// Response: what action to take when this signature is matched.
    pub response: ImmuneResponse,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImmuneResponse {
    /// Flag for human review but allow scoring.
    Flag,
    /// Quarantine: accept but do not insert into the index.
    Quarantine,
    /// Reject outright (confirmed malicious pattern).
    Reject,
    /// Rate-limit the contributor.
    RateLimit,
}

/// Check a trace against the immune memory. Returns the most severe
/// matching response, or `None` if no signatures match.
pub fn immune_check(
    trace_features: &TraceImmunityFeatures,
    memory_cells: &[ImmuneMemoryCell],
) -> Option<ImmuneResponse> {
    let mut worst_response: Option<ImmuneResponse> = None;

    for cell in memory_cells {
        let matches = match cell.signature.split_once(':') {
            Some(("rapid_submission", _)) => {
                trace_features.submissions_last_minute > 50
            }
            Some(("identical_content_hash", hash)) => {
                trace_features.content_hash == hash
            }
            Some(("low_entropy_content", _)) => {
                trace_features.byte_entropy < 2.0
            }
            Some(("excessive_repetition", _)) => {
                trace_features.repetition_ratio > 0.8
            }
            _ => false,
        };

        if matches {
            let severity = match cell.response {
                ImmuneResponse::Flag => 0,
                ImmuneResponse::RateLimit => 1,
                ImmuneResponse::Quarantine => 2,
                ImmuneResponse::Reject => 3,
            };
            let current_severity = worst_response.map_or(-1, |r| match r {
                ImmuneResponse::Flag => 0,
                ImmuneResponse::RateLimit => 1,
                ImmuneResponse::Quarantine => 2,
                ImmuneResponse::Reject => 3,
            });
            if severity > current_severity {
                worst_response = Some(cell.response);
            }
        }
    }

    worst_response
}

/// Features extracted from a trace for immune system matching.
#[derive(Debug, Clone)]
pub struct TraceImmunityFeatures {
    pub content_hash: String,
    pub byte_entropy: f64,
    pub repetition_ratio: f64,
    pub submissions_last_minute: u64,
    pub contributor_id: Uuid,
}
```

#### PostgreSQL migration: V48__affect_system.sql

```sql
-- Trace affect scores (computed per submission).
ALTER TABLE trace_corpus_submissions
    ADD COLUMN IF NOT EXISTS affect_frustration_micros BIGINT,
    ADD COLUMN IF NOT EXISTS affect_flow_micros BIGINT,
    ADD COLUMN IF NOT EXISTS affect_valence_micros BIGINT;

-- Somatic markers (learned contributor/pattern associations).
CREATE TABLE trace_somatic_markers (
    id                      UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id               UUID NOT NULL REFERENCES trace_tenants(id),
    feature_key             TEXT NOT NULL,
    valence_micros          BIGINT NOT NULL DEFAULT 0,
    confidence_micros       BIGINT NOT NULL DEFAULT 0,
    positive_observations   BIGINT NOT NULL DEFAULT 0,
    negative_observations   BIGINT NOT NULL DEFAULT 0,
    last_updated_at         TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (tenant_id, feature_key)
);

ALTER TABLE trace_somatic_markers ENABLE ROW LEVEL SECURITY;
ALTER TABLE trace_somatic_markers FORCE ROW LEVEL SECURITY;
CREATE POLICY trace_somatic_markers_tenant_policy
    ON trace_somatic_markers
    USING (tenant_id = trace_current_tenant_id());

-- Immune memory cells (learned anomaly patterns).
CREATE TABLE trace_immune_memory (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id           UUID NOT NULL REFERENCES trace_tenants(id),
    signature           TEXT NOT NULL,
    observation_count   BIGINT NOT NULL DEFAULT 0,
    confirmed           BOOLEAN NOT NULL DEFAULT false,
    response            TEXT NOT NULL DEFAULT 'flag'
                        CHECK (response IN ('flag', 'quarantine', 'reject', 'rate_limit')),
    created_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (tenant_id, signature)
);

ALTER TABLE trace_immune_memory ENABLE ROW LEVEL SECURITY;
ALTER TABLE trace_immune_memory FORCE ROW LEVEL SECURITY;
CREATE POLICY trace_immune_memory_tenant_policy
    ON trace_immune_memory
    USING (tenant_id = trace_current_tenant_id());

-- Homeostatic vital signs log (sampled every minute by the worker).
CREATE TABLE trace_system_vitals (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    queue_depth         INT NOT NULL,
    p99_latency_micros  BIGINT NOT NULL,
    error_rate_micros   BIGINT NOT NULL,
    memory_pct_micros   BIGINT NOT NULL,
    cpu_pct_micros      BIGINT NOT NULL,
    throughput_factor_micros BIGINT NOT NULL,
    shed_load           BOOLEAN NOT NULL DEFAULT false,
    sampled_at          TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- No RLS on system vitals: this is a global operational table.
-- Partitioned by time for efficient cleanup.
CREATE INDEX idx_system_vitals_sampled
    ON trace_system_vitals (sampled_at DESC);
```

---

## Summary: Implementation Priority Matrix

| # | Feature | Priority | Complexity | Effort | Dependencies |
|---|---------|----------|------------|--------|--------------|
| 1 | Adaptive Scoring | P0 | Medium-High | 3-4 weeks | None |
| 2 | Multi-Stage Gate Pipeline | P0 | High | 4-5 weeks | None (can use adaptive scoring) |
| 3 | HDC Fingerprinting | P1 | Medium | 2-3 weeks | None |
| 4 | Dream Consolidation | P1 | High | 5-6 weeks | HDC (#3), Adaptive (#1) |
| 5 | Self-Learning | P1 | Medium | 3-4 weeks | Multi-Stage (#2), Adaptive (#1) |
| 6 | Stigmergic Coordination | P2 | Medium | 3-4 weeks | Dream (#4) for capability maps |
| 7 | Biological/Affective | P2 | Medium-High | 4-5 weeks | Somatic markers need Dream (#4) |

**Recommended implementation order:**

1. Adaptive Scoring (P0) -- unlocks dynamic threshold behavior.
2. Multi-Stage Gate Pipeline (P0) -- unlocks cost savings and extensibility.
3. HDC Fingerprinting (P1) -- provides fast filtering for the pipeline.
4. Self-Learning (P1) -- leverages pipeline stages for A/B testing.
5. Dream Consolidation (P1) -- leverages all of the above for offline learning.
6. Stigmergic Coordination (P2) -- builds on dream clusters for capability maps.
7. Biological/Affective (P2) -- cross-cutting, can be incrementally adopted.

Total estimated effort: 24-31 weeks for a single engineer. With parallelization
(P0 items can be concurrent; P1 items can start once their P0 dependencies
land), a two-engineer team could complete the P0+P1 items in 10-12 weeks and
the P2 items in the following 6-8 weeks.
