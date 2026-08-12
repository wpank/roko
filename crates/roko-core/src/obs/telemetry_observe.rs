//! TelemetryObserve trait and PeriodicObserver (E33-T05).
//!
//! This module defines the **observation** side of the telemetry pipeline: the
//! [`TelemetryObserve`] trait and the concrete [`PeriodicObserver`] that
//! snapshots all lenses registered in a [`LensRegistry`] on each call.
//!
//! # Naming note
//!
//! An `Observe` trait already exists in `roko_core::traits` (re-exported from
//! `lib.rs`) with a different signature (`fn observe(&self) -> Vec<Engram>`).
//! This module intentionally uses the distinct name **`TelemetryObserve`** to
//! avoid any collision with that trait.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::lens::LensRegistry;

// ─── TelemetryObservation ────────────────────────────────────────────

/// A single observation record produced by a [`TelemetryObserve`] implementor.
///
/// Each observation captures the output of one [`Lens`](super::lens::Lens)
/// at a point in time, serialised as an opaque JSON value so that consumers
/// don't need to know the concrete lens type.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetryObservation {
    /// The name of the lens that produced this observation.
    pub lens_name: String,
    /// Wall-clock time at which the observation was taken.
    pub timestamp: DateTime<Utc>,
    /// Lens snapshot serialised as JSON.
    pub data: Value,
}

// ─── TelemetryObserve trait ──────────────────────────────────────────

/// A read-only observer that snapshots one or more lenses from a
/// [`LensRegistry`] and returns the results as a [`Vec<TelemetryObservation>`].
///
/// Implementors may filter, transform, or aggregate lens snapshots before
/// returning them.  The trait is deliberately synchronous (no async) because
/// [`Lens::snapshot`](super::lens::Lens::snapshot) is guaranteed cheap.
pub trait TelemetryObserve: Send + Sync {
    /// Observe the registry and return a set of observations.
    ///
    /// Implementors should call [`LensRegistry::snapshot_all`] (or a filtered
    /// subset) and convert each [`LensSnapshot`](super::lens::LensSnapshot)
    /// into a [`TelemetryObservation`].
    fn observe(&self, registry: &LensRegistry) -> Vec<TelemetryObservation>;
}

// ─── PeriodicObserver ────────────────────────────────────────────────

/// A [`TelemetryObserve`] implementation that snapshots **all** lenses in the
/// registry on each call to [`observe`](TelemetryObserve::observe).
///
/// This is the default observer for most use cases: record a snapshot of
/// every registered lens, attach a `Utc::now()` timestamp, and serialise the
/// [`LensSnapshot`](super::lens::LensSnapshot) as JSON.
///
/// # Usage
///
/// ```rust
/// # use std::sync::Arc;
/// # use roko_core::obs::metrics::MetricRegistry;
/// # use roko_core::obs::lens::default_registry;
/// # use roko_core::obs::telemetry_observe::{PeriodicObserver, TelemetryObserve};
/// let metrics = Arc::new(MetricRegistry::new());
/// let registry = default_registry(metrics);
/// let observer = PeriodicObserver::new();
/// let observations = observer.observe(&registry);
/// assert_eq!(observations.len(), 3); // token-usage, latency, cost
/// ```
#[derive(Debug, Default, Clone)]
pub struct PeriodicObserver;

impl PeriodicObserver {
    /// Construct a new `PeriodicObserver`.
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl TelemetryObserve for PeriodicObserver {
    fn observe(&self, registry: &LensRegistry) -> Vec<TelemetryObservation> {
        let now = Utc::now();
        registry
            .snapshot_all()
            .into_iter()
            .map(|(name, snap)| {
                let data = serde_json::to_value(&snap).unwrap_or(Value::Null);
                TelemetryObservation {
                    lens_name: name,
                    timestamp: now,
                    data,
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::obs::lens::{CollectorLens, LensScope, default_registry};
    use crate::obs::metrics::{LabelSet, MetricRegistry};
    use std::sync::Arc;

    #[test]
    fn periodic_observer_snapshots_all_default_lenses() {
        let metrics = Arc::new(MetricRegistry::new());
        let registry = default_registry(Arc::clone(&metrics));
        let observer = PeriodicObserver::new();
        let observations = observer.observe(&registry);
        // default_registry provides 3 lenses.
        assert_eq!(observations.len(), 3);
        let names: Vec<&str> = observations.iter().map(|o| o.lens_name.as_str()).collect();
        assert!(names.contains(&"token-usage"));
        assert!(names.contains(&"latency"));
        assert!(names.contains(&"cost"));
    }

    #[test]
    fn periodic_observer_observations_have_timestamps() {
        let metrics = Arc::new(MetricRegistry::new());
        let registry = default_registry(metrics);
        let observer = PeriodicObserver::new();
        let observations = observer.observe(&registry);
        for obs in &observations {
            // Timestamp should be close to now — just verify it's not the epoch.
            assert!(
                obs.timestamp.timestamp() > 0,
                "timestamp should be after Unix epoch"
            );
        }
    }

    #[test]
    fn periodic_observer_data_is_valid_json() {
        let metrics = Arc::new(MetricRegistry::new());
        // Register a metric so there is something to project.
        metrics
            .register_counter("roko_tokens_total", "tokens", LabelSet::new())
            .inc_by(7);
        let registry = default_registry(metrics);
        let observer = PeriodicObserver::new();
        let observations = observer.observe(&registry);
        for obs in &observations {
            assert!(
                !obs.data.is_null(),
                "data should not be null for lens '{}'",
                obs.lens_name
            );
        }
    }

    #[test]
    fn periodic_observer_empty_registry_returns_empty_vec() {
        let registry = LensRegistry::new();
        let observer = PeriodicObserver::new();
        let observations = observer.observe(&registry);
        assert!(observations.is_empty());
    }

    #[test]
    fn periodic_observer_custom_registry() {
        let metrics = Arc::new(MetricRegistry::new());
        let mut registry = LensRegistry::new();
        registry.register(
            "collector",
            Box::new(CollectorLens::new(Arc::clone(&metrics), LensScope::Global)),
        );
        let observer = PeriodicObserver::new();
        let observations = observer.observe(&registry);
        assert_eq!(observations.len(), 1);
        assert_eq!(observations[0].lens_name, "collector");
    }

    #[test]
    fn telemetry_observation_roundtrips_via_json() {
        let obs = TelemetryObservation {
            lens_name: "test-lens".into(),
            timestamp: Utc::now(),
            data: serde_json::json!({"version": 0, "metrics": []}),
        };
        let json = serde_json::to_string(&obs).expect("serialize");
        let back: TelemetryObservation = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.lens_name, "test-lens");
        assert_eq!(back.data["version"], 0);
    }

    #[test]
    fn periodic_observer_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<PeriodicObserver>();
    }
}
