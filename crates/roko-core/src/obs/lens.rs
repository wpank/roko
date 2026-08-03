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
}
