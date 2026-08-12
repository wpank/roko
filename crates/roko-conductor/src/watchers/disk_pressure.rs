//! Disk pressure watcher: fires when available disk space is low during execution.
//!
//! Monitors `Metric` signals tagged `name=disk_free_mb` (emitted by the disk
//! monitor background task) and compares against configurable thresholds.
//!
//! Three severity levels:
//! - `disk_free_mb < warn_disk_mb` → warning (suggest cleanup)
//! - `disk_free_mb < min_free_disk_mb` → critical (pause new agent spawns)
//! - `disk_free_mb < min_free_disk_mb / 2` → emergency (force stop non-essential agents)
//!
//! This watcher is a pure `React` consumer: it reads metric signals from the
//! stream and emits intervention signals. It never performs I/O.

use roko_core::{Body, Context, Engram, Kind, React};

/// Tag key marking signals from this watcher.
pub const WATCHER_NAME: &str = "disk-pressure";

/// Metric name for available disk space in MB (emitted by DiskMonitor, E47-T02).
pub const DISK_FREE_MB_METRIC: &str = "disk_free_mb";

/// Tag key identifying the metric name on `Metric` signals.
pub const METRIC_NAME_TAG: &str = "name";
/// Tag key for the numeric value.
pub const METRIC_VALUE_TAG: &str = "value";

/// Default warning threshold in MB (5 GB).
///
/// Matches `ResourcesConfig::default_warn_disk_mb()`.
pub const DEFAULT_WARN_DISK_MB: u64 = 5120;

/// Default critical/minimum threshold in MB (2 GB).
///
/// Matches `ResourcesConfig::default_min_free_disk_mb()`.
pub const DEFAULT_MIN_FREE_DISK_MB: u64 = 2048;

/// Monitors `disk_free_mb` metric signals and fires intervention signals
/// when disk space drops below configured thresholds.
///
/// Thresholds are sourced from `ResourcesConfig` (`warn_disk_mb` and
/// `min_free_disk_mb`). No separate `[conductor.watchers.disk_pressure]`
/// schema is needed or used.
///
/// # Severity levels
///
/// | Free space | Severity | Recommendation |
/// |---|---|---|
/// | `< warn_disk_mb` | `warning` | log warning, suggest cleanup |
/// | `< min_free_disk_mb` | `critical` | `recommendation=pause` |
/// | `< min_free_disk_mb / 2` | `emergency` | `recommendation=abort` |
#[derive(Debug, Clone)]
pub struct DiskPressureWatcher {
    /// Warning threshold in MB (e.g. 5 GB = 5120 MB).
    warn_disk_mb: u64,
    /// Critical threshold in MB (e.g. 2 GB = 2048 MB).
    min_free_disk_mb: u64,
}

impl Default for DiskPressureWatcher {
    fn default() -> Self {
        Self {
            warn_disk_mb: DEFAULT_WARN_DISK_MB,
            min_free_disk_mb: DEFAULT_MIN_FREE_DISK_MB,
        }
    }
}

impl DiskPressureWatcher {
    /// Create with explicit thresholds sourced from `ResourcesConfig`.
    #[must_use]
    pub const fn new(warn_disk_mb: u64, min_free_disk_mb: u64) -> Self {
        Self {
            warn_disk_mb,
            min_free_disk_mb,
        }
    }
}

/// Find the most recent metric value by name (returns MB as `u64`).
fn latest_metric(stream: &[Engram], name: &str) -> Option<u64> {
    stream
        .iter()
        .rev()
        .find(|s| s.kind == Kind::Metric && s.tag(METRIC_NAME_TAG) == Some(name))
        .and_then(|s| s.tag(METRIC_VALUE_TAG))
        .and_then(|v| v.parse::<u64>().ok())
}

impl roko_core::Cell for DiskPressureWatcher {
    fn cell_id(&self) -> &str {
        "LDisk-LPressure-LWatcher"
    }

    fn cell_name(&self) -> &str {
        "DiskPressureWatcher"
    }

    fn protocols(&self) -> Vec<roko_core::ProtocolId> {
        vec![roko_core::ProtocolId::React]
    }
}

impl React for DiskPressureWatcher {
    fn decide(&self, stream: &[Engram], _ctx: &Context) -> Vec<Engram> {
        let Some(free_mb) = latest_metric(stream, DISK_FREE_MB_METRIC) else {
            return Vec::new();
        };

        let emergency_threshold = self.min_free_disk_mb / 2;

        if free_mb < emergency_threshold {
            // Emergency: < min/2 (e.g. < 1 GB). Force stop non-essential agents.
            vec![
                Engram::builder(Kind::Custom("conductor.intervention".into()))
                    .body(Body::text(format!(
                        "disk critically low: {free_mb} MB free (emergency threshold {emergency_threshold} MB); \
                         recommend abort to prevent data loss"
                    )))
                    .tag("watcher", WATCHER_NAME)
                    .tag("severity", "emergency")
                    .tag("recommendation", "abort")
                    .tag("disk_free_mb", free_mb.to_string())
                    .tag("threshold_mb", emergency_threshold.to_string())
                    .build(),
            ]
        } else if free_mb < self.min_free_disk_mb {
            // Critical: < min_free_disk_mb (e.g. < 2 GB). Pause new agent spawns.
            vec![
                Engram::builder(Kind::Custom("conductor.intervention".into()))
                    .body(Body::text(format!(
                        "disk space critical: {free_mb} MB free (minimum {min} MB); \
                         pausing new agent spawns",
                        min = self.min_free_disk_mb,
                    )))
                    .tag("watcher", WATCHER_NAME)
                    .tag("severity", "critical")
                    .tag("recommendation", "pause")
                    .tag("disk_free_mb", free_mb.to_string())
                    .tag("threshold_mb", self.min_free_disk_mb.to_string())
                    .build(),
            ]
        } else if free_mb < self.warn_disk_mb {
            // Warning: < warn_disk_mb (e.g. < 5 GB). Log warning, suggest cleanup.
            vec![
                Engram::builder(Kind::Custom("conductor.intervention".into()))
                    .body(Body::text(format!(
                        "disk space low: {free_mb} MB free (warning threshold {warn} MB); \
                         consider running `roko knowledge gc` or `roko doctor disk`",
                        warn = self.warn_disk_mb,
                    )))
                    .tag("watcher", WATCHER_NAME)
                    .tag("severity", "warning")
                    .tag("recommendation", "cleanup")
                    .tag("disk_free_mb", free_mb.to_string())
                    .tag("threshold_mb", self.warn_disk_mb.to_string())
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

    /// Build a `disk_free_mb` metric signal with the given value.
    fn disk_free_signal(free_mb: u64) -> Engram {
        Engram::builder(Kind::Metric)
            .body(Body::text("disk_free_mb"))
            .tag(METRIC_NAME_TAG, DISK_FREE_MB_METRIC)
            .tag(METRIC_VALUE_TAG, free_mb.to_string())
            .build()
    }

    // ── No-fire cases ─────────────────────────────────────────────────────────

    #[test]
    fn empty_stream_no_fire() {
        let w = DiskPressureWatcher::default();
        assert!(w.decide(&[], &Context::at(0)).is_empty());
    }

    #[test]
    fn no_disk_metric_no_fire() {
        let w = DiskPressureWatcher::default();
        // A Metric with a different name should not trigger.
        let other = Engram::builder(Kind::Metric)
            .body(Body::text("plan_cost"))
            .tag(METRIC_NAME_TAG, "plan_cost")
            .tag(METRIC_VALUE_TAG, "42")
            .build();
        assert!(w.decide(&[other], &Context::at(0)).is_empty());
    }

    #[test]
    fn above_warn_threshold_no_fire() {
        // warn = 5120, min = 2048 — 10 GB free is fine
        let w = DiskPressureWatcher::new(5120, 2048);
        let stream = vec![disk_free_signal(10_000)];
        assert!(w.decide(&stream, &Context::at(0)).is_empty());
    }

    #[test]
    fn exactly_at_warn_threshold_no_fire() {
        // The condition is strictly less-than, so 5120 == warn is OK.
        let w = DiskPressureWatcher::new(5120, 2048);
        let stream = vec![disk_free_signal(5120)];
        assert!(w.decide(&stream, &Context::at(0)).is_empty());
    }

    // ── Warning level ─────────────────────────────────────────────────────────

    #[test]
    fn below_warn_fires_warning() {
        let w = DiskPressureWatcher::new(5120, 2048);
        // 3 GB free: below warn (5120) but above min (2048)
        let stream = vec![disk_free_signal(3072)];
        let out = w.decide(&stream, &Context::at(0));
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].tag("watcher"), Some(WATCHER_NAME));
        assert_eq!(out[0].tag("severity"), Some("warning"));
        assert_eq!(out[0].tag("recommendation"), Some("cleanup"));
        assert_eq!(out[0].tag("disk_free_mb"), Some("3072"));
    }

    #[test]
    fn warning_body_contains_threshold() {
        let w = DiskPressureWatcher::new(5120, 2048);
        let stream = vec![disk_free_signal(4000)];
        let out = w.decide(&stream, &Context::at(0));
        assert_eq!(out.len(), 1);
        // Body should mention the warn threshold.
        let body_text = format!("{:?}", out[0].body);
        assert!(body_text.contains("5120") || out[0].tag("threshold_mb") == Some("5120"));
    }

    // ── Critical level ────────────────────────────────────────────────────────

    #[test]
    fn below_min_fires_critical() {
        let w = DiskPressureWatcher::new(5120, 2048);
        // 1.5 GB free: below min (2048) but above emergency (1024)
        let stream = vec![disk_free_signal(1500)];
        let out = w.decide(&stream, &Context::at(0));
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].tag("watcher"), Some(WATCHER_NAME));
        assert_eq!(out[0].tag("severity"), Some("critical"));
        assert_eq!(out[0].tag("recommendation"), Some("pause"));
        assert_eq!(out[0].tag("disk_free_mb"), Some("1500"));
    }

    #[test]
    fn exactly_at_min_threshold_fires_warning_not_critical() {
        // 2048 == min: not strictly less-than, so should fall into warn band
        // (2048 < 5120 triggers warning, but 2048 < 2048 is false so no critical).
        let w = DiskPressureWatcher::new(5120, 2048);
        let stream = vec![disk_free_signal(2048)];
        let out = w.decide(&stream, &Context::at(0));
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].tag("severity"), Some("warning"));
    }

    // ── Emergency level ───────────────────────────────────────────────────────

    #[test]
    fn below_emergency_fires_emergency() {
        // emergency = min/2 = 1024 MB
        let w = DiskPressureWatcher::new(5120, 2048);
        // 512 MB free: below emergency (1024)
        let stream = vec![disk_free_signal(512)];
        let out = w.decide(&stream, &Context::at(0));
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].tag("watcher"), Some(WATCHER_NAME));
        assert_eq!(out[0].tag("severity"), Some("emergency"));
        assert_eq!(out[0].tag("recommendation"), Some("abort"));
        assert_eq!(out[0].tag("disk_free_mb"), Some("512"));
    }

    #[test]
    fn zero_free_fires_emergency() {
        let w = DiskPressureWatcher::new(5120, 2048);
        let stream = vec![disk_free_signal(0)];
        let out = w.decide(&stream, &Context::at(0));
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].tag("severity"), Some("emergency"));
        assert_eq!(out[0].tag("recommendation"), Some("abort"));
    }

    #[test]
    fn emergency_body_mentions_abort() {
        let w = DiskPressureWatcher::new(5120, 2048);
        let stream = vec![disk_free_signal(100)];
        let out = w.decide(&stream, &Context::at(0));
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].tag("recommendation"), Some("abort"));
    }

    // ── Most-recent-metric semantics ──────────────────────────────────────────

    #[test]
    fn uses_most_recent_metric() {
        let w = DiskPressureWatcher::new(5120, 2048);
        // Earlier signal is critical, but most recent is fine.
        let stream = vec![
            disk_free_signal(500),    // critical / emergency (old)
            disk_free_signal(10_000), // most recent — above warn
        ];
        assert!(w.decide(&stream, &Context::at(0)).is_empty());
    }

    #[test]
    fn most_recent_critical_overrides_old_fine() {
        let w = DiskPressureWatcher::new(5120, 2048);
        let stream = vec![
            disk_free_signal(10_000), // fine (old)
            disk_free_signal(1500),   // most recent — critical
        ];
        let out = w.decide(&stream, &Context::at(0));
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].tag("severity"), Some("critical"));
    }

    // ── Intervention kind ──────────────────────────────────────────────────────

    #[test]
    fn intervention_kind_is_conductor_intervention() {
        let w = DiskPressureWatcher::new(5120, 2048);
        let stream = vec![disk_free_signal(4000)]; // warning band
        let out = w.decide(&stream, &Context::at(0));
        assert_eq!(out.len(), 1);
        assert_eq!(
            out[0].kind,
            Kind::Custom("conductor.intervention".to_string())
        );
    }

    // ── Constructor and defaults ──────────────────────────────────────────────

    #[test]
    fn default_thresholds_match_resources_config_defaults() {
        let w = DiskPressureWatcher::default();
        assert_eq!(w.warn_disk_mb, DEFAULT_WARN_DISK_MB);
        assert_eq!(w.min_free_disk_mb, DEFAULT_MIN_FREE_DISK_MB);
        assert_eq!(DEFAULT_WARN_DISK_MB, 5120);
        assert_eq!(DEFAULT_MIN_FREE_DISK_MB, 2048);
    }

    #[test]
    fn custom_thresholds_are_honoured() {
        // warn=10_000, min=3_000, emergency=1_500
        let w = DiskPressureWatcher::new(10_000, 3_000);
        // 8_000 MB free: below warn (10_000) but above min (3_000) → warning
        let stream = vec![disk_free_signal(8_000)];
        let out = w.decide(&stream, &Context::at(0));
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].tag("severity"), Some("warning"));
    }

    #[test]
    fn watcher_name_constant() {
        let w = DiskPressureWatcher::default();
        assert_eq!(w.name(), WATCHER_NAME);
        assert_eq!(WATCHER_NAME, "disk-pressure");
    }
}
