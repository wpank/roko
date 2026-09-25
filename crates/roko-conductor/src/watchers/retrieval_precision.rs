//! Retrieval-precision watcher (RAG-17).
//!
//! Fires a warning intervention when the rolling retrieval precision
//! (fraction of settled retrieval outcomes where `gate_passed = true`)
//! drops below a configurable threshold.
//!
//! The watcher reads `Metric` signals tagged `name=retrieval_precision` from
//! the signal stream. If the precision value on the most recent such signal
//! is below the threshold, it emits a warning.  If no such signal is present
//! the watcher stays silent.
//!
//! # Integration
//!
//! Emit a `Metric` signal with `name=retrieval_precision` and
//! `value=<0.0–1.0>` into the conductor signal stream after each settled
//! retrieval batch.  The `roko-acp` / `roko-cli` runner already writes
//! retrieval outcome records; a periodic flush or per-dispatch signal is
//! sufficient.

use roko_core::{Body, Context, Kind, React, Signal};

/// Tag key identifying the metric name.
pub const METRIC_NAME_TAG: &str = "name";
/// Metric name for retrieval precision.
pub const RETRIEVAL_PRECISION_METRIC: &str = "retrieval_precision";
/// Tag key for the numeric value.
pub const METRIC_VALUE_TAG: &str = "value";

/// Tag key on emitted intervention signals.
pub const WATCHER_NAME: &str = "retrieval-precision";

/// Default threshold below which the watcher fires (0 – 1.0).
///
/// 0.60 = 60 % gate-pass rate before alerting.
pub const DEFAULT_ALERT_THRESHOLD: f64 = 0.60;

/// Fires when rolling retrieval precision drops below a threshold.
///
/// Reads the most recent `retrieval_precision` metric from the stream and
/// emits a warning when the value is below `alert_threshold`.
#[derive(Debug, Clone)]
pub struct RetrievalPrecisionWatcher {
    /// Precision ratio below which to alert (0.0 – 1.0).
    alert_threshold: f64,
}

impl Default for RetrievalPrecisionWatcher {
    fn default() -> Self {
        Self {
            alert_threshold: DEFAULT_ALERT_THRESHOLD,
        }
    }
}

impl RetrievalPrecisionWatcher {
    /// Create with a custom alert threshold (0.0 – 1.0).
    #[must_use]
    pub fn new(alert_threshold: f64) -> Self {
        Self {
            alert_threshold: alert_threshold.clamp(0.0, 1.0),
        }
    }

    /// The configured alert threshold.
    #[must_use]
    pub const fn threshold(&self) -> f64 {
        self.alert_threshold
    }
}

/// Find the most recent `retrieval_precision` metric value in the stream.
fn latest_precision(stream: &[Signal]) -> Option<f64> {
    stream
        .iter()
        .rev()
        .find(|s| {
            s.kind == Kind::Metric && s.tag(METRIC_NAME_TAG) == Some(RETRIEVAL_PRECISION_METRIC)
        })
        .and_then(|s| s.tag(METRIC_VALUE_TAG))
        .and_then(|v| v.parse::<f64>().ok())
}

impl roko_core::Cell for RetrievalPrecisionWatcher {
    fn cell_id(&self) -> &str {
        "LRetrievalPrecision-LWatcher"
    }

    fn cell_name(&self) -> &str {
        "RetrievalPrecisionWatcher"
    }

    fn protocols(&self) -> Vec<roko_core::ProtocolId> {
        vec![roko_core::ProtocolId::React]
    }
}

impl React for RetrievalPrecisionWatcher {
    fn decide(&self, stream: &[Signal], _ctx: &Context) -> Vec<Signal> {
        let Some(precision) = latest_precision(stream) else {
            return Vec::new();
        };

        if precision < self.alert_threshold {
            vec![
                Signal::builder(Kind::Custom("conductor.intervention".into()))
                    .body(Body::text(format!(
                        "retrieval precision {:.1}% is below alert threshold {:.1}%",
                        precision * 100.0,
                        self.alert_threshold * 100.0,
                    )))
                    .tag("watcher", WATCHER_NAME)
                    .tag("severity", "warning")
                    .tag("precision", format!("{precision:.4}"))
                    .tag("threshold", format!("{:.4}", self.alert_threshold))
                    .build(),
            ]
        } else {
            Vec::new()
        }
    }

    fn name(&self) -> &str {
        WATCHER_NAME
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn precision_signal(value: f64) -> Signal {
        Signal::builder(Kind::Metric)
            .body(Body::text("precision"))
            .tag(METRIC_NAME_TAG, RETRIEVAL_PRECISION_METRIC)
            .tag(METRIC_VALUE_TAG, &format!("{value}"))
            .build()
    }

    #[test]
    fn empty_stream_no_fire() {
        let w = RetrievalPrecisionWatcher::default();
        assert!(w.decide(&[], &Context::at(0)).is_empty());
    }

    #[test]
    fn above_threshold_no_fire() {
        let w = RetrievalPrecisionWatcher::default(); // 0.60
        let stream = vec![precision_signal(0.80)];
        assert!(w.decide(&stream, &Context::at(0)).is_empty());
    }

    #[test]
    fn at_threshold_no_fire() {
        let w = RetrievalPrecisionWatcher::default(); // 0.60
        let stream = vec![precision_signal(0.60)];
        // Exactly at threshold: should NOT fire (precision < threshold, not <=).
        assert!(w.decide(&stream, &Context::at(0)).is_empty());
    }

    #[test]
    fn below_threshold_fires_warning() {
        let w = RetrievalPrecisionWatcher::default(); // 0.60
        let stream = vec![precision_signal(0.40)];
        let out = w.decide(&stream, &Context::at(0));
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].tag("watcher"), Some(WATCHER_NAME));
        assert_eq!(out[0].tag("severity"), Some("warning"));
    }

    #[test]
    fn uses_most_recent_signal() {
        let w = RetrievalPrecisionWatcher::default(); // 0.60
        let stream = vec![
            precision_signal(0.20), // older — below threshold
            precision_signal(0.75), // most recent — above threshold
        ];
        // Most recent is above threshold → no fire.
        assert!(w.decide(&stream, &Context::at(0)).is_empty());
    }

    #[test]
    fn custom_threshold() {
        let w = RetrievalPrecisionWatcher::new(0.90);
        let stream = vec![precision_signal(0.85)]; // 0.85 < 0.90
        let out = w.decide(&stream, &Context::at(0));
        assert_eq!(out.len(), 1);
    }

    #[test]
    fn threshold_clamped_to_zero_one() {
        let w = RetrievalPrecisionWatcher::new(1.5);
        assert_eq!(w.threshold(), 1.0);

        let w2 = RetrievalPrecisionWatcher::new(-0.5);
        assert_eq!(w2.threshold(), 0.0);
    }
}
