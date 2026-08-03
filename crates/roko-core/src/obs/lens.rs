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
//! | **Collector** | Gathers raw metrics from a source (e.g. `MetricRegistry`) | `CollectorLens` (E13-T02) |
//! | **Transform** | Derives a metric from upstream Lens output (trend, anomaly) | `TrendLens` (future) |
//! | **Export**     | Serialises Lens output for an external sink (Prometheus, OTLP) | `PrometheusExportLens` (future) |
//!
//! This module defines ONLY the trait and its supporting types.  Concrete lens
//! implementations live alongside the subsystem they adapt (e.g.
//! `CollectorLens` wrapping `MetricRegistry` lives in this `obs` module).

use std::borrow::Cow;

use serde::{Deserialize, Serialize};

use super::metrics::MetricSnapshot;

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
