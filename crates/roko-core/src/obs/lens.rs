//! Lens: the telemetry unification trait (v2 §15).
//!
//! A **Lens** is a read-only adapter that projects live telemetry state into
//! a uniform [`LensSnapshot`] shape.  Lenses observe without mutating: removing
//! every Lens from a running system changes nothing about its behaviour — only
//! visibility.
//!
//! Three specialisations share this trait:
//!
//! | Kind | Role | Example |
//! |------|------|---------|
//! | **Collector** | Gathers raw metrics from a source (e.g. `MetricRegistry`) | [`CollectorLens`] |
//! | **Transform** | Derives a metric from upstream Lens output (trend, anomaly) | `TrendLens` (future) |
//! | **Export**     | Serialises Lens output for an external sink (Prometheus, OTLP) | `PrometheusExportLens` (future) |
//!
//! The trait and its supporting types are defined first; [`CollectorLens`] is
//! the first concrete implementation, wrapping [`MetricRegistry`] as a
//! read-side adapter (E13-T02).
//!
//! ## Concrete collector lenses (E33-T03)
//!
//! Three domain-specific lenses project subsets of [`MetricRegistry`] state:
//!
//! | Lens | Metric families | Labels |
//! |------|----------------|--------|
//! | [`TokenUsageLens`] | `roko_llm_tokens_total` | `provider`, `model`, `direction` |
//! | [`LatencyLens`] | `roko_llm_ttft_seconds`, `roko_llm_request_duration_seconds` | `provider`, `model` |
//! | [`CostLens`] | `roko_llm_cost_usd_total` | `provider`, `model` |

use std::borrow::Cow;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use serde::{Deserialize, Serialize};

use super::metrics::{MetricRegistry, MetricSnapshot};

// ─── LensScope ──────────────────────────────────────────────────────

/// The observation scope a [`Lens`] is attached to.
///
/// Mirrors the v2 spec hierarchy (Cell → Graph → Agent → Space → Global)
/// but uses plain `String` identifiers rather than typed refs so the trait
/// can live in `roko-core` without pulling in graph or agent crate types.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LensScope {
    /// Observe a single named component.
    Component(String),
    /// Observe an entire plan/graph run.
    Graph(String),
    /// Observe one agent's full pipeline.
    Agent(String),
    /// Observe everything in a workspace/space.
    Space(String),
    /// Chain: observe another Lens's output.
    Lens(String),
    /// System-wide — observe all events.
    Global,
}

// ─── LensSnapshot ───────────────────────────────────────────────────

/// The uniform output shape of every [`Lens`].
///
/// A snapshot is a timestamped bag of [`MetricSnapshot`] entries that the
/// Lens projects from its source.  Downstream consumers (StateHub, TUI,
/// SSE) treat this as the sole data contract — they never read the source
/// directly.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LensSnapshot {
    /// Human-readable lens name (e.g. `"metric-collector"`).
    pub lens_name: Cow<'static, str>,
    /// The scope this snapshot covers.
    pub scope: LensScope,
    /// Monotonically increasing version within this lens instance.
    pub version: u64,
    /// Individual metric entries projected by the lens.
    pub metrics: Vec<MetricSnapshot>,
}

// ─── Lens trait ─────────────────────────────────────────────────────

/// A read-only telemetry adapter.
///
/// Implementors project live state from their source into a [`LensSnapshot`].
/// The trait is deliberately synchronous and infallible for the snapshot path
/// — the source (e.g. `MetricRegistry`) already handles its own locking and
/// error conditions.  An async/fallible `observe` method can be added later
/// for transform and export lenses without breaking this core contract.
pub trait Lens: Send + Sync {
    /// Human-readable name (stable across restarts).
    fn name(&self) -> &str;

    /// The scope this lens observes.
    fn scope(&self) -> &LensScope;

    /// Take a point-in-time snapshot of the source.
    ///
    /// This MUST be cheap (no I/O, no blocking beyond a read-lock).
    /// The caller is responsible for scheduling how often to poll.
    fn snapshot(&self) -> LensSnapshot;
}

// ─── CollectorLens ─────────────────────────────────────────────────

/// A read-only adapter that projects [`MetricRegistry`] state into the
/// [`Lens`] shape.
///
/// `CollectorLens` does **not** record metrics — it only reads them.
/// The registry continues to be the write-side owner; this lens is the
/// read-side projection consumed by StateHub, TUI, and SSE.
///
/// # Construction
///
/// ```rust
/// # use std::sync::Arc;
/// # use roko_core::obs::metrics::MetricRegistry;
/// # use roko_core::obs::lens::{CollectorLens, LensScope};
/// let registry = Arc::new(MetricRegistry::new());
/// let lens = CollectorLens::new(registry, LensScope::Global);
/// ```
pub struct CollectorLens {
    registry: Arc<MetricRegistry>,
    scope: LensScope,
    /// Monotonically increasing version bumped on every `snapshot()` call.
    version: AtomicU64,
}

impl CollectorLens {
    /// Wrap an existing [`MetricRegistry`] as a collector lens.
    ///
    /// The scope determines the observation boundary reported in every
    /// [`LensSnapshot`] produced by this lens.
    pub fn new(registry: Arc<MetricRegistry>, scope: LensScope) -> Self {
        Self {
            registry,
            scope,
            version: AtomicU64::new(0),
        }
    }
}

impl Lens for CollectorLens {
    fn name(&self) -> &str {
        "metric-collector"
    }

    fn scope(&self) -> &LensScope {
        &self.scope
    }

    fn snapshot(&self) -> LensSnapshot {
        let version = self.version.fetch_add(1, Ordering::Relaxed);
        LensSnapshot {
            lens_name: Cow::Borrowed("metric-collector"),
            scope: self.scope.clone(),
            version,
            metrics: self.registry.snapshot(),
        }
    }
}

// ─── TokenUsageLens ─────────────────────────────────────────────────

/// A [`Lens`] that projects token-usage counters from a [`MetricRegistry`].
///
/// Filters the registry snapshot to entries belonging to the canonical
/// `roko_llm_tokens_total` family (all `provider`/`model`/`direction`
/// label combinations that have been registered).  The filtered view
/// lets consumers focus purely on token accounting without noise from
/// other metric families.
///
/// # Construction
///
/// ```rust
/// # use std::sync::Arc;
/// # use roko_core::obs::metrics::{MetricRegistry, LabelSet};
/// # use roko_core::obs::lens::{TokenUsageLens, LensScope};
/// let registry = Arc::new(MetricRegistry::new());
/// let lens = TokenUsageLens::new(Arc::clone(&registry), LensScope::Global);
/// ```
pub struct TokenUsageLens {
    registry: Arc<MetricRegistry>,
    scope: LensScope,
    version: AtomicU64,
}

impl TokenUsageLens {
    /// Wrap an existing [`MetricRegistry`] as a token-usage lens.
    pub fn new(registry: Arc<MetricRegistry>, scope: LensScope) -> Self {
        Self {
            registry,
            scope,
            version: AtomicU64::new(0),
        }
    }
}

impl Lens for TokenUsageLens {
    fn name(&self) -> &str {
        "token-usage"
    }

    fn scope(&self) -> &LensScope {
        &self.scope
    }

    fn snapshot(&self) -> LensSnapshot {
        use super::schema::ROKO_LLM_TOKENS_TOTAL;
        let version = self.version.fetch_add(1, Ordering::Relaxed);
        let metrics: Vec<MetricSnapshot> = self
            .registry
            .snapshot()
            .into_iter()
            .filter(|m| m.name == ROKO_LLM_TOKENS_TOTAL)
            .collect();
        LensSnapshot {
            lens_name: Cow::Borrowed("token-usage"),
            scope: self.scope.clone(),
            version,
            metrics,
        }
    }
}

// ─── LatencyLens ─────────────────────────────────────────────────────

/// A [`Lens`] that projects LLM latency histograms from a [`MetricRegistry`].
///
/// Filters the registry snapshot to entries belonging to the canonical
/// `roko_llm_ttft_seconds` and `roko_llm_request_duration_seconds` families
/// (keyed by `provider` and `model`).  Downstream consumers (TUI, SSE,
/// dashboards) can display p50/p95/p99 without touching the raw registry.
///
/// # Construction
///
/// ```rust
/// # use std::sync::Arc;
/// # use roko_core::obs::metrics::MetricRegistry;
/// # use roko_core::obs::lens::{LatencyLens, LensScope};
/// let registry = Arc::new(MetricRegistry::new());
/// let lens = LatencyLens::new(Arc::clone(&registry), LensScope::Global);
/// ```
pub struct LatencyLens {
    registry: Arc<MetricRegistry>,
    scope: LensScope,
    version: AtomicU64,
}

impl LatencyLens {
    /// Wrap an existing [`MetricRegistry`] as a latency lens.
    pub fn new(registry: Arc<MetricRegistry>, scope: LensScope) -> Self {
        Self {
            registry,
            scope,
            version: AtomicU64::new(0),
        }
    }
}

impl Lens for LatencyLens {
    fn name(&self) -> &str {
        "latency"
    }

    fn scope(&self) -> &LensScope {
        &self.scope
    }

    fn snapshot(&self) -> LensSnapshot {
        use super::schema::{ROKO_LLM_REQUEST_DURATION_SECONDS, ROKO_LLM_TTFT_SECONDS};
        let version = self.version.fetch_add(1, Ordering::Relaxed);
        let metrics: Vec<MetricSnapshot> = self
            .registry
            .snapshot()
            .into_iter()
            .filter(|m| {
                m.name == ROKO_LLM_TTFT_SECONDS || m.name == ROKO_LLM_REQUEST_DURATION_SECONDS
            })
            .collect();
        LensSnapshot {
            lens_name: Cow::Borrowed("latency"),
            scope: self.scope.clone(),
            version,
            metrics,
        }
    }
}

// ─── CostLens ────────────────────────────────────────────────────────

/// A [`Lens`] that projects cumulative LLM cost counters from a
/// [`MetricRegistry`].
///
/// Filters the registry snapshot to entries belonging to the canonical
/// `roko_llm_cost_usd_total` family (keyed by `provider` and `model`).
/// Values are in USD and accumulate monotonically; downstream consumers
/// may diff consecutive snapshots to compute per-interval spend.
///
/// # Construction
///
/// ```rust
/// # use std::sync::Arc;
/// # use roko_core::obs::metrics::MetricRegistry;
/// # use roko_core::obs::lens::{CostLens, LensScope};
/// let registry = Arc::new(MetricRegistry::new());
/// let lens = CostLens::new(Arc::clone(&registry), LensScope::Global);
/// ```
pub struct CostLens {
    registry: Arc<MetricRegistry>,
    scope: LensScope,
    version: AtomicU64,
}

impl CostLens {
    /// Wrap an existing [`MetricRegistry`] as a cost lens.
    pub fn new(registry: Arc<MetricRegistry>, scope: LensScope) -> Self {
        Self {
            registry,
            scope,
            version: AtomicU64::new(0),
        }
    }
}

impl Lens for CostLens {
    fn name(&self) -> &str {
        "cost"
    }

    fn scope(&self) -> &LensScope {
        &self.scope
    }

    fn snapshot(&self) -> LensSnapshot {
        use super::schema::ROKO_LLM_COST_USD_TOTAL;
        let version = self.version.fetch_add(1, Ordering::Relaxed);
        let metrics: Vec<MetricSnapshot> = self
            .registry
            .snapshot()
            .into_iter()
            .filter(|m| m.name == ROKO_LLM_COST_USD_TOTAL)
            .collect();
        LensSnapshot {
            lens_name: Cow::Borrowed("cost"),
            scope: self.scope.clone(),
            version,
            metrics,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::obs::metrics::{LabelSet, MetricValue};

    /// Prove that `CollectorLens` faithfully projects `MetricRegistry` state
    /// into a `LensSnapshot` — the core shape contract of E13-T02.
    #[test]
    fn collector_lens_projects_registry_into_snapshot() {
        let registry = Arc::new(MetricRegistry::new());

        // Record some metrics through the registry (the write side).
        let counter = registry.register_counter("roko_test_total", "test counter", LabelSet::new());
        counter.inc_by(42);

        let gauge = registry.register_gauge(
            "roko_test_gauge",
            "test gauge",
            LabelSet::from_pairs(&[("env", "test")]),
        );
        gauge.set(7);

        // Create the read-side lens.
        let lens = CollectorLens::new(Arc::clone(&registry), LensScope::Global);

        // Verify trait methods.
        assert_eq!(lens.name(), "metric-collector");
        assert_eq!(lens.scope(), &LensScope::Global);

        // Take a snapshot via the Lens trait.
        let snap = lens.snapshot();
        assert_eq!(snap.lens_name, "metric-collector");
        assert_eq!(snap.scope, LensScope::Global);
        assert_eq!(snap.version, 0, "first snapshot is version 0");
        assert_eq!(snap.metrics.len(), 2, "two families registered");

        // Verify the counter was projected.
        let counter_snap = snap.metrics.iter().find(|m| m.name == "roko_test_total");
        assert!(counter_snap.is_some(), "counter metric must appear");
        match &counter_snap.unwrap().value {
            MetricValue::Counter(v) => assert_eq!(*v, 42),
            other => panic!("expected Counter, got {other:?}"),
        }

        // Verify the gauge was projected.
        let gauge_snap = snap.metrics.iter().find(|m| m.name == "roko_test_gauge");
        assert!(gauge_snap.is_some(), "gauge metric must appear");
        match &gauge_snap.unwrap().value {
            MetricValue::Gauge(v) => assert_eq!(*v, 7),
            other => panic!("expected Gauge, got {other:?}"),
        }
    }

    #[test]
    fn collector_lens_version_increments() {
        let registry = Arc::new(MetricRegistry::new());
        let lens = CollectorLens::new(registry, LensScope::Space("default".into()));

        let s0 = lens.snapshot();
        let s1 = lens.snapshot();
        let s2 = lens.snapshot();

        assert_eq!(s0.version, 0);
        assert_eq!(s1.version, 1);
        assert_eq!(s2.version, 2);
    }

    #[test]
    fn collector_lens_reflects_live_mutations() {
        let registry = Arc::new(MetricRegistry::new());
        let counter = registry.register_counter("roko_live_total", "live counter", LabelSet::new());
        let lens = CollectorLens::new(Arc::clone(&registry), LensScope::Global);

        // Snapshot before mutation.
        let before = lens.snapshot();
        let val_before = before
            .metrics
            .iter()
            .find(|m| m.name == "roko_live_total")
            .map(|m| match &m.value {
                MetricValue::Counter(v) => *v,
                _ => panic!("expected counter"),
            })
            .unwrap();
        assert_eq!(val_before, 0);

        // Mutate the registry (write side).
        counter.inc_by(10);

        // Snapshot after mutation — the lens sees the updated value.
        let after = lens.snapshot();
        let val_after = after
            .metrics
            .iter()
            .find(|m| m.name == "roko_live_total")
            .map(|m| match &m.value {
                MetricValue::Counter(v) => *v,
                _ => panic!("expected counter"),
            })
            .unwrap();
        assert_eq!(val_after, 10);
    }

    #[test]
    fn collector_lens_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<CollectorLens>();
    }

    // ─── TokenUsageLens tests ────────────────────────────────────────

    #[test]
    fn token_usage_lens_filters_to_token_counters_only() {
        use crate::obs::schema::ROKO_LLM_TOKENS_TOTAL;

        let registry = Arc::new(MetricRegistry::new());

        // Register the token counter under the canonical name.
        let input_labels = LabelSet::from_pairs(&[
            ("direction", "input"),
            ("model", "claude-sonnet-4-6"),
            ("provider", "anthropic"),
        ]);
        let output_labels = LabelSet::from_pairs(&[
            ("direction", "output"),
            ("model", "claude-sonnet-4-6"),
            ("provider", "anthropic"),
        ]);
        let input_counter = registry.register_counter(
            ROKO_LLM_TOKENS_TOTAL,
            "LLM tokens consumed/produced, by provider, model, direction",
            input_labels,
        );
        let output_counter = registry.register_counter(
            ROKO_LLM_TOKENS_TOTAL,
            "LLM tokens consumed/produced, by provider, model, direction",
            output_labels,
        );
        input_counter.inc_by(1_200);
        output_counter.inc_by(300);

        // Register an unrelated metric that must NOT appear in the lens.
        let _ = registry.register_counter("roko_plans_total", "plans", LabelSet::new());

        let lens = TokenUsageLens::new(Arc::clone(&registry), LensScope::Global);
        assert_eq!(lens.name(), "token-usage");
        assert_eq!(lens.scope(), &LensScope::Global);

        let snap = lens.snapshot();
        assert_eq!(snap.lens_name, "token-usage");
        assert_eq!(snap.version, 0);

        // Only token-usage entries appear.
        assert_eq!(
            snap.metrics.len(),
            2,
            "two token-direction entries expected"
        );
        for m in &snap.metrics {
            assert_eq!(m.name, ROKO_LLM_TOKENS_TOTAL);
        }

        // Verify input token value.
        let input_snap = snap
            .metrics
            .iter()
            .find(|m| {
                m.labels
                    .iter()
                    .any(|(k, v)| k == "direction" && v == "input")
            })
            .expect("input direction entry");
        match &input_snap.value {
            MetricValue::Counter(v) => assert_eq!(*v, 1_200),
            other => panic!("expected Counter, got {other:?}"),
        }
    }

    #[test]
    fn token_usage_lens_version_increments() {
        let registry = Arc::new(MetricRegistry::new());
        let lens = TokenUsageLens::new(registry, LensScope::Space("default".into()));

        let s0 = lens.snapshot();
        let s1 = lens.snapshot();
        assert_eq!(s0.version, 0);
        assert_eq!(s1.version, 1);
    }

    #[test]
    fn token_usage_lens_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<TokenUsageLens>();
    }

    // ─── LatencyLens tests ───────────────────────────────────────────

    #[test]
    fn latency_lens_filters_to_latency_histograms_only() {
        use crate::obs::histograms::LLM_LATENCY_BUCKETS;
        use crate::obs::schema::{ROKO_LLM_REQUEST_DURATION_SECONDS, ROKO_LLM_TTFT_SECONDS};

        let registry = Arc::new(MetricRegistry::new());

        let provider_model =
            LabelSet::from_pairs(&[("model", "claude-sonnet-4-6"), ("provider", "anthropic")]);

        let ttft = registry.register_histogram(
            ROKO_LLM_TTFT_SECONDS,
            "LLM time-to-first-token in seconds",
            provider_model.clone(),
            LLM_LATENCY_BUCKETS.to_vec(),
        );
        let duration = registry.register_histogram(
            ROKO_LLM_REQUEST_DURATION_SECONDS,
            "LLM total request duration in seconds",
            provider_model.clone(),
            LLM_LATENCY_BUCKETS.to_vec(),
        );
        ttft.observe(0.12);
        duration.observe(2.4);

        // Unrelated metric — must not appear.
        let _ = registry.register_counter("roko_tasks_total", "tasks", LabelSet::new());

        let lens = LatencyLens::new(Arc::clone(&registry), LensScope::Agent("coder".into()));
        assert_eq!(lens.name(), "latency");
        assert_eq!(lens.scope(), &LensScope::Agent("coder".into()));

        let snap = lens.snapshot();
        assert_eq!(snap.lens_name, "latency");
        assert_eq!(snap.version, 0);
        assert_eq!(snap.metrics.len(), 2, "ttft + request-duration entries");

        let names: Vec<&str> = snap.metrics.iter().map(|m| m.name.as_str()).collect();
        assert!(
            names.contains(&ROKO_LLM_TTFT_SECONDS),
            "ttft entry must be present"
        );
        assert!(
            names.contains(&ROKO_LLM_REQUEST_DURATION_SECONDS),
            "request-duration entry must be present"
        );
    }

    #[test]
    fn latency_lens_version_increments() {
        let registry = Arc::new(MetricRegistry::new());
        let lens = LatencyLens::new(registry, LensScope::Global);

        let s0 = lens.snapshot();
        let s1 = lens.snapshot();
        assert_eq!(s0.version, 0);
        assert_eq!(s1.version, 1);
    }

    #[test]
    fn latency_lens_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<LatencyLens>();
    }

    // ─── CostLens tests ──────────────────────────────────────────────

    #[test]
    fn cost_lens_filters_to_cost_counters_only() {
        use crate::obs::schema::ROKO_LLM_COST_USD_TOTAL;

        let registry = Arc::new(MetricRegistry::new());

        let anthropic_labels =
            LabelSet::from_pairs(&[("model", "claude-sonnet-4-6"), ("provider", "anthropic")]);
        let openai_labels = LabelSet::from_pairs(&[("model", "gpt-5.2"), ("provider", "openai")]);

        let anthropic_cost = registry.register_counter(
            ROKO_LLM_COST_USD_TOTAL,
            "Cumulative LLM spend in USD, by provider and model",
            anthropic_labels,
        );
        let openai_cost = registry.register_counter(
            ROKO_LLM_COST_USD_TOTAL,
            "Cumulative LLM spend in USD, by provider and model",
            openai_labels,
        );
        // Token counters are stored as integer micro-dollars (u64); we use
        // integer-cent precision here: 150 = $0.000150 if interpreted as
        // micro-dollars, but for the counter the value is opaque.
        anthropic_cost.inc_by(150);
        openai_cost.inc_by(420);

        // Unrelated metric — must not appear.
        let _ = registry.register_counter("roko_gate_verdicts_total", "verdicts", LabelSet::new());

        let lens = CostLens::new(Arc::clone(&registry), LensScope::Graph("plan-42".into()));
        assert_eq!(lens.name(), "cost");
        assert_eq!(lens.scope(), &LensScope::Graph("plan-42".into()));

        let snap = lens.snapshot();
        assert_eq!(snap.lens_name, "cost");
        assert_eq!(snap.version, 0);
        assert_eq!(snap.metrics.len(), 2, "one entry per provider/model pair");

        for m in &snap.metrics {
            assert_eq!(m.name, ROKO_LLM_COST_USD_TOTAL);
        }

        // Find and verify the Anthropic entry.
        let anthropic_snap = snap
            .metrics
            .iter()
            .find(|m| {
                m.labels
                    .iter()
                    .any(|(k, v)| k == "provider" && v == "anthropic")
            })
            .expect("anthropic cost entry");
        match &anthropic_snap.value {
            MetricValue::Counter(v) => assert_eq!(*v, 150),
            other => panic!("expected Counter, got {other:?}"),
        }
    }

    #[test]
    fn cost_lens_version_increments() {
        let registry = Arc::new(MetricRegistry::new());
        let lens = CostLens::new(registry, LensScope::Global);

        let s0 = lens.snapshot();
        let s1 = lens.snapshot();
        assert_eq!(s0.version, 0);
        assert_eq!(s1.version, 1);
    }

    #[test]
    fn cost_lens_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<CostLens>();
    }
}
