//! P4-16: Trigger outcome learning.
//!
//! Tracks per-trigger success/failure ratio, mean flow duration, and outcome
//! trends. Emits cooldown signals when failure rate exceeds threshold.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// Default failure rate threshold that triggers cooldown extension.
const DEFAULT_FAILURE_THRESHOLD: f64 = 0.5;

/// Default debounce extension factor on cooldown.
const DEFAULT_DEBOUNCE_EXTENSION_FACTOR: f64 = 2.0;

/// Minimum observations before cooldown is considered.
const MIN_OBSERVATIONS_FOR_COOLDOWN: u64 = 5;

/// Per-trigger outcome statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerOutcomeStats {
    /// Trigger binding ID.
    pub binding_id: String,
    /// Total firings.
    pub total_firings: u64,
    /// Successful firings.
    pub success_count: u64,
    /// Failed firings.
    pub failure_count: u64,
    /// Mean flow duration in milliseconds.
    pub mean_duration_ms: f64,
    /// Current debounce multiplier (1.0 = no extension).
    pub debounce_multiplier: f64,
    /// Whether this trigger is currently in cooldown.
    pub in_cooldown: bool,
    /// Rolling sum of durations for EMA computation.
    #[serde(default)]
    duration_sum_ms: f64,
}

impl TriggerOutcomeStats {
    fn new(binding_id: &str) -> Self {
        Self {
            binding_id: binding_id.to_string(),
            total_firings: 0,
            success_count: 0,
            failure_count: 0,
            mean_duration_ms: 0.0,
            debounce_multiplier: 1.0,
            in_cooldown: false,
            duration_sum_ms: 0.0,
        }
    }

    /// Failure rate as a fraction in [0, 1].
    #[must_use]
    pub fn failure_rate(&self) -> f64 {
        if self.total_firings == 0 {
            return 0.0;
        }
        self.failure_count as f64 / self.total_firings as f64
    }

    /// Success rate as a fraction in [0, 1].
    #[must_use]
    pub fn success_rate(&self) -> f64 {
        1.0 - self.failure_rate()
    }
}

/// Result of recording a trigger outcome.
#[derive(Debug, Clone)]
pub enum TriggerCooldownAction {
    /// No action needed.
    None,
    /// Extend the trigger's debounce by the given multiplier.
    ExtendDebounce {
        /// Trigger binding ID.
        binding_id: String,
        /// New debounce multiplier.
        multiplier: f64,
    },
    /// Clear cooldown (recovery detected).
    ClearCooldown {
        /// Trigger binding ID.
        binding_id: String,
    },
}

/// Learns trigger outcome patterns and recommends cooldown actions.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TriggerOutcomeLearner {
    /// Per-trigger statistics.
    triggers: HashMap<String, TriggerOutcomeStats>,
    /// Failure rate threshold for cooldown.
    #[serde(default = "default_failure_threshold")]
    failure_threshold: f64,
    /// Debounce extension factor.
    #[serde(default = "default_extension_factor")]
    extension_factor: f64,
}

fn default_failure_threshold() -> f64 {
    DEFAULT_FAILURE_THRESHOLD
}

fn default_extension_factor() -> f64 {
    DEFAULT_DEBOUNCE_EXTENSION_FACTOR
}

impl TriggerOutcomeLearner {
    /// Create a new learner with default thresholds.
    #[must_use]
    pub fn new() -> Self {
        Self {
            triggers: HashMap::new(),
            failure_threshold: DEFAULT_FAILURE_THRESHOLD,
            extension_factor: DEFAULT_DEBOUNCE_EXTENSION_FACTOR,
        }
    }

    /// Create a learner with custom thresholds.
    #[must_use]
    pub fn with_thresholds(failure_threshold: f64, extension_factor: f64) -> Self {
        Self {
            triggers: HashMap::new(),
            failure_threshold: failure_threshold.clamp(0.0, 1.0),
            extension_factor: extension_factor.max(1.0),
        }
    }

    /// Record a trigger outcome and return any recommended action.
    pub fn record_outcome(
        &mut self,
        binding_id: &str,
        success: bool,
        duration_ms: u64,
    ) -> TriggerCooldownAction {
        let stats = self
            .triggers
            .entry(binding_id.to_string())
            .or_insert_with(|| TriggerOutcomeStats::new(binding_id));

        stats.total_firings += 1;
        if success {
            stats.success_count += 1;
        } else {
            stats.failure_count += 1;
        }
        stats.duration_sum_ms += duration_ms as f64;
        stats.mean_duration_ms = stats.duration_sum_ms / stats.total_firings as f64;

        // Check for cooldown conditions.
        if stats.total_firings < MIN_OBSERVATIONS_FOR_COOLDOWN {
            return TriggerCooldownAction::None;
        }

        if stats.failure_rate() > self.failure_threshold && !stats.in_cooldown {
            stats.in_cooldown = true;
            stats.debounce_multiplier *= self.extension_factor;
            return TriggerCooldownAction::ExtendDebounce {
                binding_id: binding_id.to_string(),
                multiplier: stats.debounce_multiplier,
            };
        }

        // Check for recovery (success rate recovered above threshold).
        if stats.in_cooldown && stats.failure_rate() < self.failure_threshold * 0.5 {
            stats.in_cooldown = false;
            stats.debounce_multiplier = 1.0;
            return TriggerCooldownAction::ClearCooldown {
                binding_id: binding_id.to_string(),
            };
        }

        TriggerCooldownAction::None
    }

    /// Get statistics for a trigger.
    #[must_use]
    pub fn stats(&self, binding_id: &str) -> Option<&TriggerOutcomeStats> {
        self.triggers.get(binding_id)
    }

    /// Get all trigger statistics.
    #[must_use]
    pub fn all_stats(&self) -> &HashMap<String, TriggerOutcomeStats> {
        &self.triggers
    }

    /// Load from disk, or return a new learner if missing/invalid.
    #[must_use]
    pub fn load_or_new(path: &Path) -> Self {
        std::fs::read_to_string(path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    /// Save to disk using atomic write.
    ///
    /// # Errors
    ///
    /// Returns an error if the learner cannot be serialized or written.
    pub fn save(&self, path: &Path) -> Result<(), std::io::Error> {
        roko_fs::atomic_write_json(path, self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_trigger_starts_clean() {
        let mut learner = TriggerOutcomeLearner::new();
        let action = learner.record_outcome("trigger-1", true, 100);
        assert!(matches!(action, TriggerCooldownAction::None));
    }

    #[test]
    fn cooldown_on_high_failure_rate() {
        let mut learner = TriggerOutcomeLearner::new();
        // Run up to MIN_OBSERVATIONS_FOR_COOLDOWN - 1 failures silently; the
        // *Nth* failure (where N = MIN_OBSERVATIONS_FOR_COOLDOWN) is the first
        // observation that can trigger cooldown.
        for _ in 0..(MIN_OBSERVATIONS_FOR_COOLDOWN - 1) {
            let _ = learner.record_outcome("trigger-bad", false, 100);
        }
        // This is the MIN_OBSERVATIONS_FOR_COOLDOWN-th observation.
        // failure_rate = 1.0 > DEFAULT_FAILURE_THRESHOLD → ExtendDebounce.
        let action = learner.record_outcome("trigger-bad", false, 100);
        assert!(
            matches!(action, TriggerCooldownAction::ExtendDebounce { .. }),
            "expected ExtendDebounce at {MIN_OBSERVATIONS_FOR_COOLDOWN} failures, got {action:?}"
        );
    }

    #[test]
    fn recovery_clears_cooldown() {
        let mut learner = TriggerOutcomeLearner::new();
        // Push into cooldown.
        for _ in 0..6 {
            learner.record_outcome("trigger-recover", false, 100);
        }
        // Then lots of successes to recover.
        for _ in 0..20 {
            learner.record_outcome("trigger-recover", true, 100);
        }
        let stats = learner.stats("trigger-recover").unwrap();
        assert!(!stats.in_cooldown);
    }
}
