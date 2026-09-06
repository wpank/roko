//! P4-06: Cumulative regret tracking for LinUCB router.
//!
//! Tracks the regret incurred by the contextual bandit model router:
//! `regret(t) = best_arm_reward(t) - selected_arm_reward(t)`.
//! Logs warnings when cumulative regret growth exceeds O(sqrt(T)).

use serde::{Deserialize, Serialize};
use std::path::Path;

/// Maximum history length before truncation.
const MAX_REGRET_HISTORY: usize = 10_000;

/// Tracks cumulative regret for the LinUCB model router.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CumulativeRegretTracker {
    /// Total cumulative regret.
    pub cumulative_regret: f64,
    /// Number of observations.
    pub observations: u64,
    /// Per-observation regret history (bounded).
    #[serde(default)]
    regret_history: Vec<f64>,
    /// Best observed reward per arm for oracle regret computation.
    #[serde(default)]
    best_arm_rewards: Vec<(String, f64)>,
    /// Running sum of squared regrets for variance.
    #[serde(default)]
    sum_squared_regret: f64,
}

impl Default for CumulativeRegretTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl CumulativeRegretTracker {
    /// Create a new empty tracker.
    #[must_use]
    pub fn new() -> Self {
        Self {
            cumulative_regret: 0.0,
            observations: 0,
            regret_history: Vec::new(),
            best_arm_rewards: Vec::new(),
            sum_squared_regret: 0.0,
        }
    }

    /// Record one observation's regret.
    ///
    /// `best_reward` is the reward of the best arm (in hindsight or by oracle).
    /// `selected_reward` is the reward of the arm that was actually selected.
    pub fn observe(&mut self, best_reward: f64, selected_reward: f64) {
        let regret = (best_reward - selected_reward).max(0.0);
        self.cumulative_regret += regret;
        self.observations += 1;
        self.sum_squared_regret += regret * regret;

        self.regret_history.push(regret);
        if self.regret_history.len() > MAX_REGRET_HISTORY {
            self.regret_history.remove(0);
        }
    }

    /// Update the best known reward for an arm.
    pub fn update_best_arm(&mut self, arm: &str, reward: f64) {
        if let Some(entry) = self.best_arm_rewards.iter_mut().find(|(a, _)| a == arm) {
            if reward > entry.1 {
                entry.1 = reward;
            }
        } else {
            self.best_arm_rewards.push((arm.to_string(), reward));
        }
    }

    /// Get the best known reward across all arms.
    #[must_use]
    pub fn best_known_reward(&self) -> f64 {
        self.best_arm_rewards
            .iter()
            .map(|(_, r)| *r)
            .fold(0.0_f64, f64::max)
    }

    /// Average per-observation regret.
    #[must_use]
    pub fn average_regret(&self) -> f64 {
        if self.observations == 0 {
            return 0.0;
        }
        self.cumulative_regret / self.observations as f64
    }

    /// Check whether cumulative regret growth exceeds O(sqrt(T)).
    ///
    /// Returns `true` if the regret is growing faster than expected,
    /// indicating the router may not be converging.
    #[must_use]
    pub fn is_regret_excessive(&self) -> bool {
        if self.observations < 50 {
            return false; // Too few observations to judge.
        }
        let t = self.observations as f64;
        let expected_bound = 2.0 * t.sqrt(); // Generous O(sqrt(T)) bound.
        self.cumulative_regret > expected_bound
    }

    /// Regret growth rate (regret per sqrt(observation)).
    #[must_use]
    pub fn normalized_regret(&self) -> f64 {
        if self.observations < 2 {
            return 0.0;
        }
        self.cumulative_regret / (self.observations as f64).sqrt()
    }

    /// Load from disk, or return a new tracker if missing/invalid.
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
    /// Returns an error if the tracker cannot be serialized or written.
    pub fn save(&self, path: &Path) -> Result<(), std::io::Error> {
        roko_fs::atomic_write_json(path, self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn regret_tracks_cumulative_difference() {
        let mut tracker = CumulativeRegretTracker::new();
        tracker.observe(1.0, 0.8);
        tracker.observe(1.0, 0.5);
        assert!((tracker.cumulative_regret - 0.7).abs() < 1e-10);
        assert_eq!(tracker.observations, 2);
    }

    #[test]
    fn regret_clamps_negative_to_zero() {
        let mut tracker = CumulativeRegretTracker::new();
        tracker.observe(0.5, 1.0); // Selected arm was better than "best".
        assert!((tracker.cumulative_regret - 0.0).abs() < 1e-10);
    }

    #[test]
    fn excessive_regret_detected() {
        let mut tracker = CumulativeRegretTracker::new();
        // Generate 100 observations with high regret.
        for _ in 0..100 {
            tracker.observe(1.0, 0.0);
        }
        assert!(tracker.is_regret_excessive());
    }

    #[test]
    fn normal_regret_not_flagged() {
        let mut tracker = CumulativeRegretTracker::new();
        // Generate 100 observations with low regret.
        for _ in 0..100 {
            tracker.observe(1.0, 0.95);
        }
        assert!(!tracker.is_regret_excessive());
    }

    #[test]
    fn best_arm_tracking() {
        let mut tracker = CumulativeRegretTracker::new();
        tracker.update_best_arm("claude", 0.8);
        tracker.update_best_arm("gpt4", 0.9);
        tracker.update_best_arm("claude", 0.85);
        assert!((tracker.best_known_reward() - 0.9).abs() < 1e-10);
    }
}
