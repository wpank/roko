//! Worktree-count watcher: fires when too many git worktrees are live.
//!
//! Monitors `Metric` signals tagged `name=worktree_count` and compares
//! against the configured `max_live` threshold. The metric is produced by
//! the E47-T09 WorktreeManager adapter on both the legacy orchestrate and
//! runner-v2 paths. This watcher is a pure `React` consumer and never
//! queries git or the filesystem directly.

use roko_core::{Body, Context, Engram, Kind, React};

/// Tag key marking signals from this watcher.
pub const WATCHER_NAME: &str = "worktree-count";

/// Metric name for the active worktree count (emitted by E47-T09).
pub const WORKTREE_COUNT_METRIC: &str = "worktree_count";

/// Tag key identifying the metric name on `Metric` signals.
pub const METRIC_NAME_TAG: &str = "name";
/// Tag key for the numeric value.
pub const METRIC_VALUE_TAG: &str = "value";

/// Default maximum number of live worktrees before a warning is emitted.
pub const DEFAULT_MAX_LIVE: usize = 8;

/// Fires a `severity=warning` intervention when the active worktree count
/// exceeds `max_live`.
///
/// This is a pure `React` consumer: it reads the latest `Kind::Metric` Engram
/// tagged `name=worktree_count` from the stream and emits at most one
/// `conductor.intervention` Engram per call.
#[derive(Debug, Clone)]
pub struct WorktreeCountWatcher {
    /// Maximum number of live worktrees before a warning is emitted.
    max_live: usize,
}

impl Default for WorktreeCountWatcher {
    fn default() -> Self {
        Self {
            max_live: DEFAULT_MAX_LIVE,
        }
    }
}

impl WorktreeCountWatcher {
    /// Create with a custom maximum live worktree count.
    #[must_use]
    pub const fn new(max_live: usize) -> Self {
        Self { max_live }
    }
}

/// Find the most recent metric value by name.
fn latest_metric(stream: &[Engram], name: &str) -> Option<usize> {
    stream
        .iter()
        .rev()
        .find(|s| s.kind == Kind::Metric && s.tag(METRIC_NAME_TAG) == Some(name))
        .and_then(|s| s.tag(METRIC_VALUE_TAG))
        .and_then(|v| v.parse::<usize>().ok())
}

impl React for WorktreeCountWatcher {
    fn decide(&self, stream: &[Engram], _ctx: &Context) -> Vec<Engram> {
        let Some(count) = latest_metric(stream, WORKTREE_COUNT_METRIC) else {
            return Vec::new();
        };

        if count > self.max_live {
            vec![
                Engram::builder(Kind::Custom("conductor.intervention".into()))
                    .body(Body::text(format!(
                        "active worktree count {count} exceeds maximum {max_live}",
                        max_live = self.max_live,
                    )))
                    .tag("watcher", WATCHER_NAME)
                    .tag("severity", "warning")
                    .tag("count", count.to_string())
                    .tag("max_live", self.max_live.to_string())
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

    fn worktree_count_signal(count: usize) -> Engram {
        Engram::builder(Kind::Metric)
            .body(Body::text("worktree_count"))
            .tag(METRIC_NAME_TAG, WORKTREE_COUNT_METRIC)
            .tag(METRIC_VALUE_TAG, count.to_string())
            .build()
    }

    #[test]
    fn worktree_count_empty_stream_no_fire() {
        let w = WorktreeCountWatcher::default();
        assert!(w.decide(&[], &Context::at(0)).is_empty());
    }

    #[test]
    fn worktree_count_no_metric_no_fire() {
        let w = WorktreeCountWatcher::default();
        // A Metric signal with a different name should not trigger.
        let other = Engram::builder(Kind::Metric)
            .body(Body::text("disk_free_mb"))
            .tag(METRIC_NAME_TAG, "disk_free_mb")
            .tag(METRIC_VALUE_TAG, "100")
            .build();
        assert!(w.decide(&[other], &Context::at(0)).is_empty());
    }

    #[test]
    fn worktree_count_at_limit_no_fire() {
        let w = WorktreeCountWatcher::new(8);
        let stream = vec![worktree_count_signal(8)];
        assert!(w.decide(&stream, &Context::at(0)).is_empty());
    }

    #[test]
    fn worktree_count_below_limit_no_fire() {
        let w = WorktreeCountWatcher::new(8);
        let stream = vec![worktree_count_signal(3)];
        assert!(w.decide(&stream, &Context::at(0)).is_empty());
    }

    #[test]
    fn worktree_count_above_limit_fires_warning() {
        let w = WorktreeCountWatcher::new(8);
        let stream = vec![worktree_count_signal(10)];
        let out = w.decide(&stream, &Context::at(0));
        assert_eq!(out.len(), 1);
        let intervention = &out[0];
        assert_eq!(intervention.tag("watcher"), Some(WATCHER_NAME));
        assert_eq!(intervention.tag("severity"), Some("warning"));
        assert_eq!(intervention.tag("count"), Some("10"));
        assert_eq!(intervention.tag("max_live"), Some("8"));
    }

    #[test]
    fn worktree_count_uses_most_recent_metric() {
        let w = WorktreeCountWatcher::new(8);
        // Earlier signal exceeds limit, but most recent is below.
        let stream = vec![
            worktree_count_signal(15), // exceeds limit
            worktree_count_signal(6),  // most recent — below limit
        ];
        assert!(w.decide(&stream, &Context::at(0)).is_empty());
    }

    #[test]
    fn worktree_count_custom_max_live_threshold() {
        let w = WorktreeCountWatcher::new(5);
        // Count of 6 should exceed custom max of 5.
        let stream = vec![worktree_count_signal(6)];
        let out = w.decide(&stream, &Context::at(0));
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].tag("max_live"), Some("5"));
    }

    #[test]
    fn worktree_count_default_max_live_is_8() {
        assert_eq!(WorktreeCountWatcher::default().max_live, DEFAULT_MAX_LIVE);
        assert_eq!(DEFAULT_MAX_LIVE, 8);
    }

    #[test]
    fn worktree_count_intervention_kind() {
        let w = WorktreeCountWatcher::new(2);
        let stream = vec![worktree_count_signal(5)];
        let out = w.decide(&stream, &Context::at(0));
        assert_eq!(out.len(), 1);
        assert_eq!(
            out[0].kind,
            Kind::Custom("conductor.intervention".to_string())
        );
    }
}
