//! P3-24: Timeout auto-adjustment tracker.
//!
//! Tracks actual dispatch/gate/test durations in a rolling window and
//! computes EMA-based timeout suggestions. Surface via `roko learn inspect
//! timeouts`.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// Rolling-window duration tracker for a named operation category.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeoutTracker {
    /// Per-operation rolling windows.
    pub categories: HashMap<String, DurationWindow>,
}

/// Rolling window of observed durations for one operation category.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DurationWindow {
    /// Exponential moving average of duration in milliseconds.
    pub ema_ms: f64,
    /// Maximum observed duration in milliseconds.
    pub max_ms: u64,
    /// Number of observations.
    pub count: u64,
    /// EMA smoothing factor.
    #[serde(default = "default_alpha")]
    pub alpha: f64,
}

fn default_alpha() -> f64 {
    0.2
}

impl Default for DurationWindow {
    fn default() -> Self {
        Self {
            ema_ms: 0.0,
            max_ms: 0,
            count: 0,
            alpha: default_alpha(),
        }
    }
}

impl DurationWindow {
    /// Record a new observation.
    pub fn record(&mut self, duration_ms: u64) {
        let value = duration_ms as f64;
        if self.count == 0 {
            self.ema_ms = value;
        } else {
            self.ema_ms = self.alpha * value + (1.0 - self.alpha) * self.ema_ms;
        }
        self.max_ms = self.max_ms.max(duration_ms);
        self.count += 1;
    }

    /// Suggested timeout: P99 approximation = 2.5 * EMA (simple heuristic).
    #[must_use]
    pub fn suggested_timeout_ms(&self) -> u64 {
        let p99_approx = self.ema_ms * 2.5;
        // At least 5s, at most observed max + 50%.
        let min_timeout = 5_000.0;
        let max_timeout = self.max_ms as f64 * 1.5;
        p99_approx
            .max(min_timeout)
            .min(max_timeout.max(min_timeout)) as u64
    }

    /// Headroom: how much slack between the suggested timeout and configured.
    #[must_use]
    pub fn headroom_ratio(&self, configured_ms: u64) -> f64 {
        let suggested = self.suggested_timeout_ms() as f64;
        if suggested <= 0.0 {
            return 1.0;
        }
        configured_ms as f64 / suggested
    }
}

/// A timeout suggestion for one operation category.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeoutSuggestion {
    /// Operation category name.
    pub category: String,
    /// Current EMA in milliseconds.
    pub ema_ms: f64,
    /// Maximum observed in milliseconds.
    pub max_ms: u64,
    /// Suggested timeout in milliseconds.
    pub suggested_ms: u64,
    /// Currently configured timeout in milliseconds.
    pub configured_ms: u64,
    /// Headroom ratio (configured / suggested).
    pub headroom: f64,
    /// Number of observations.
    pub observations: u64,
}

impl Default for TimeoutTracker {
    fn default() -> Self {
        Self {
            categories: HashMap::new(),
        }
    }
}

impl TimeoutTracker {
    /// Create a new empty tracker.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a duration for a category.
    pub fn record(&mut self, category: &str, duration_ms: u64) {
        self.categories
            .entry(category.to_string())
            .or_default()
            .record(duration_ms);
    }

    /// Generate suggestions for all tracked categories.
    pub fn suggestions(&self, configured: &HashMap<String, u64>) -> Vec<TimeoutSuggestion> {
        self.categories
            .iter()
            .map(|(category, window)| {
                let configured_ms = configured.get(category).copied().unwrap_or(60_000);
                TimeoutSuggestion {
                    category: category.clone(),
                    ema_ms: window.ema_ms,
                    max_ms: window.max_ms,
                    suggested_ms: window.suggested_timeout_ms(),
                    configured_ms,
                    headroom: window.headroom_ratio(configured_ms),
                    observations: window.count,
                }
            })
            .collect()
    }

    /// Load from a JSON file.
    pub fn load(path: &std::path::Path) -> std::io::Result<Self> {
        let contents = std::fs::read_to_string(path)?;
        serde_json::from_str(&contents)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }

    /// Persist to a JSON file.
    pub fn save(&self, path: &std::path::Path) -> std::io::Result<()> {
        let contents = serde_json::to_string_pretty(self).map_err(|e| std::io::Error::other(e))?;
        std::fs::write(path, contents)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_and_suggest() {
        let mut tracker = TimeoutTracker::new();
        for _ in 0..10 {
            tracker.record("dispatch", 1_000);
        }
        let window = tracker.categories.get("dispatch").unwrap();
        assert!(window.ema_ms > 900.0 && window.ema_ms <= 1_000.0);
        assert!(window.suggested_timeout_ms() >= 2_500);
    }

    #[test]
    fn headroom_ratio() {
        let mut window = DurationWindow::default();
        for _ in 0..10 {
            window.record(2_000);
        }
        // configured = 10_000, suggested ~= 5_000 -> headroom ~= 2.0
        let headroom = window.headroom_ratio(10_000);
        assert!(headroom > 1.5 && headroom < 2.5);
    }
}
