//! P4-18: Adaptive model-specific tool degradation threshold learner.
//!
//! Replaces the static `max_tools_before_degrade` with an adaptive threshold
//! that learns per-model tool-count-vs-success-rate relationships.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// Default static cap when no learned data exists.
const DEFAULT_TOOL_CAP: usize = 25;

/// Minimum observations before a learned cap is trusted.
const MIN_OBSERVATIONS_FOR_CAP: u64 = 20;

/// Success rate drop that signals degradation.
const DEGRADATION_DROP_THRESHOLD: f64 = 0.15;

/// Per-model observation of tool count vs success.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct ModelToolObservation {
    /// Bucketed (tool_count_bucket, success_count, total_count).
    buckets: Vec<ToolCountBucket>,
}

/// Bucket tracking success rate at a given tool count range.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ToolCountBucket {
    /// Lower bound of tool count (inclusive).
    min_tools: usize,
    /// Upper bound of tool count (inclusive).
    max_tools: usize,
    /// Successful completions in this bucket.
    success_count: u64,
    /// Total completions in this bucket.
    total_count: u64,
}

impl ToolCountBucket {
    fn success_rate(&self) -> f64 {
        if self.total_count == 0 {
            return 1.0;
        }
        self.success_count as f64 / self.total_count as f64
    }
}

/// Learns per-model tool degradation thresholds.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ToolCapLearner {
    /// Per-model observations.
    models: HashMap<String, ModelToolObservation>,
    /// Learned caps per model.
    learned_caps: HashMap<String, usize>,
}

impl ToolCapLearner {
    /// Create a new empty learner.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Record an observation: model used N tools and succeeded or failed.
    pub fn observe(&mut self, model: &str, tool_count: usize, success: bool) {
        let obs = self.models.entry(model.to_string()).or_default();

        // Find or create the right bucket (buckets of 5).
        let bucket_min = (tool_count / 5) * 5;
        let bucket_max = bucket_min + 4;

        let bucket = if let Some(b) = obs
            .buckets
            .iter_mut()
            .find(|b| b.min_tools == bucket_min)
        {
            b
        } else {
            obs.buckets.push(ToolCountBucket {
                min_tools: bucket_min,
                max_tools: bucket_max,
                success_count: 0,
                total_count: 0,
            });
            obs.buckets.last_mut().unwrap()
        };

        bucket.total_count += 1;
        if success {
            bucket.success_count += 1;
        }

        // Recompute learned cap for this model.
        self.recompute_cap(model);
    }

    fn recompute_cap(&mut self, model: &str) {
        let obs = match self.models.get(model) {
            Some(o) => o,
            None => return,
        };

        // Sort buckets by min_tools.
        let mut sorted_buckets: Vec<_> = obs.buckets.iter().collect();
        sorted_buckets.sort_by_key(|b| b.min_tools);

        // Find the first bucket where success rate drops significantly
        // compared to the lowest-tool-count bucket.
        let baseline_rate = sorted_buckets
            .first()
            .map(|b| b.success_rate())
            .unwrap_or(1.0);

        for bucket in &sorted_buckets {
            if bucket.total_count < MIN_OBSERVATIONS_FOR_CAP {
                continue;
            }
            let drop = baseline_rate - bucket.success_rate();
            if drop > DEGRADATION_DROP_THRESHOLD {
                self.learned_caps
                    .insert(model.to_string(), bucket.min_tools);
                return;
            }
        }

        // No degradation detected; remove any previous cap.
        self.learned_caps.remove(model);
    }

    /// Get the effective tool cap for a model.
    ///
    /// Returns the learned cap if available, otherwise the static default.
    #[must_use]
    pub fn effective_cap(&self, model: &str) -> usize {
        self.learned_caps
            .get(model)
            .copied()
            .unwrap_or(DEFAULT_TOOL_CAP)
    }

    /// Check whether a model has a learned (non-default) cap.
    #[must_use]
    pub fn has_learned_cap(&self, model: &str) -> bool {
        self.learned_caps.contains_key(model)
    }

    /// Get all learned caps.
    #[must_use]
    pub fn all_caps(&self) -> &HashMap<String, usize> {
        &self.learned_caps
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
    fn default_cap_when_no_data() {
        let learner = ToolCapLearner::new();
        assert_eq!(learner.effective_cap("claude"), DEFAULT_TOOL_CAP);
    }

    #[test]
    fn learns_cap_on_degradation() {
        let mut learner = ToolCapLearner::new();

        // Low tool counts: high success.
        for _ in 0..30 {
            learner.observe("test-model", 3, true);
        }

        // High tool counts: low success.
        for _ in 0..25 {
            learner.observe("test-model", 27, false);
        }

        assert!(learner.has_learned_cap("test-model"));
        assert!(learner.effective_cap("test-model") < DEFAULT_TOOL_CAP);
    }

    #[test]
    fn no_cap_when_all_good() {
        let mut learner = ToolCapLearner::new();
        for tool_count in [3, 8, 13, 18, 23] {
            for _ in 0..25 {
                learner.observe("good-model", tool_count, true);
            }
        }
        assert!(!learner.has_learned_cap("good-model"));
    }
}
