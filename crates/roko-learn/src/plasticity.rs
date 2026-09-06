//! P4-21: Plasticity metric and monitoring.
//!
//! Tracks the learning system's plasticity (ability to learn new patterns)
//! vs stability (retention of existing knowledge). Emits alerts when
//! plasticity drops too low (system is over-fit) or too high (system is
//! unstable).

use serde::{Deserialize, Serialize};
use std::path::Path;

/// Default target plasticity range.
const MIN_PLASTICITY: f64 = 0.2;
/// Default maximum plasticity before instability warning.
const MAX_PLASTICITY: f64 = 0.8;
/// EMA decay for plasticity tracking.
const PLASTICITY_EMA_ALPHA: f64 = 0.1;

/// Plasticity observation: a single learning event.
#[derive(Debug, Clone)]
pub struct PlasticityObservation {
    /// Whether the learning system accepted a new pattern/rule.
    pub pattern_accepted: bool,
    /// Whether the new pattern contradicted an existing one.
    pub contradicted_existing: bool,
    /// Magnitude of the update (0.0 to 1.0).
    pub update_magnitude: f64,
}

/// Plasticity tracking state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlasticityMonitor {
    /// EMA of the acceptance rate for new patterns.
    pub acceptance_ema: f64,
    /// EMA of the contradiction rate.
    pub contradiction_ema: f64,
    /// EMA of the update magnitude.
    pub magnitude_ema: f64,
    /// Total observations.
    pub total_observations: u64,
    /// Computed plasticity score (0.0 = rigid, 1.0 = fully plastic).
    pub plasticity: f64,
    /// Lower bound of healthy plasticity range.
    pub min_threshold: f64,
    /// Upper bound of healthy plasticity range.
    pub max_threshold: f64,
}

impl Default for PlasticityMonitor {
    fn default() -> Self {
        Self {
            acceptance_ema: 0.5,
            contradiction_ema: 0.0,
            magnitude_ema: 0.5,
            total_observations: 0,
            plasticity: 0.5,
            min_threshold: MIN_PLASTICITY,
            max_threshold: MAX_PLASTICITY,
        }
    }
}

/// Alert emitted when plasticity is outside healthy range.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlasticityAlert {
    /// System is too rigid: not accepting new patterns.
    TooRigid,
    /// System is too unstable: contradicting too much.
    TooUnstable,
    /// System is in healthy plasticity range.
    Healthy,
}

impl PlasticityMonitor {
    /// Create a new monitor with default thresholds.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a monitor with custom plasticity thresholds.
    #[must_use]
    pub fn with_thresholds(min: f64, max: f64) -> Self {
        Self {
            min_threshold: min.clamp(0.0, 0.5),
            max_threshold: max.clamp(0.5, 1.0),
            ..Self::default()
        }
    }

    /// Record an observation and update plasticity.
    pub fn observe(&mut self, obs: &PlasticityObservation) {
        let accepted = if obs.pattern_accepted { 1.0 } else { 0.0 };
        let contradicted = if obs.contradicted_existing {
            1.0
        } else {
            0.0
        };

        if self.total_observations == 0 {
            self.acceptance_ema = accepted;
            self.contradiction_ema = contradicted;
            self.magnitude_ema = obs.update_magnitude;
        } else {
            self.acceptance_ema = PLASTICITY_EMA_ALPHA
                .mul_add(accepted, (1.0 - PLASTICITY_EMA_ALPHA) * self.acceptance_ema);
            self.contradiction_ema = PLASTICITY_EMA_ALPHA
                .mul_add(contradicted, (1.0 - PLASTICITY_EMA_ALPHA) * self.contradiction_ema);
            self.magnitude_ema = PLASTICITY_EMA_ALPHA
                .mul_add(obs.update_magnitude, (1.0 - PLASTICITY_EMA_ALPHA) * self.magnitude_ema);
        }

        self.total_observations += 1;

        // Plasticity = acceptance rate * magnitude, penalized by contradiction rate.
        self.plasticity =
            (self.acceptance_ema * self.magnitude_ema * (1.0 - self.contradiction_ema * 0.5))
                .clamp(0.0, 1.0);
    }

    /// Check the current plasticity against thresholds.
    #[must_use]
    pub fn check(&self) -> PlasticityAlert {
        if self.total_observations < 10 {
            return PlasticityAlert::Healthy; // Not enough data.
        }
        if self.plasticity < self.min_threshold {
            PlasticityAlert::TooRigid
        } else if self.plasticity > self.max_threshold {
            PlasticityAlert::TooUnstable
        } else {
            PlasticityAlert::Healthy
        }
    }

    /// Load from disk, or return a new monitor if missing/invalid.
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
    /// Returns an error if the monitor cannot be serialized or written.
    pub fn save(&self, path: &Path) -> Result<(), std::io::Error> {
        roko_fs::atomic_write_json(path, self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn healthy_plasticity() {
        let mut monitor = PlasticityMonitor::new();
        for _ in 0..20 {
            monitor.observe(&PlasticityObservation {
                pattern_accepted: true,
                contradicted_existing: false,
                update_magnitude: 0.5,
            });
        }
        assert_eq!(monitor.check(), PlasticityAlert::Healthy);
    }

    #[test]
    fn too_rigid_detected() {
        let mut monitor = PlasticityMonitor::new();
        for _ in 0..20 {
            monitor.observe(&PlasticityObservation {
                pattern_accepted: false,
                contradicted_existing: false,
                update_magnitude: 0.1,
            });
        }
        assert_eq!(monitor.check(), PlasticityAlert::TooRigid);
    }

    #[test]
    fn too_unstable_detected() {
        let mut monitor = PlasticityMonitor::new();
        for _ in 0..20 {
            monitor.observe(&PlasticityObservation {
                pattern_accepted: true,
                contradicted_existing: false,
                update_magnitude: 1.0,
            });
        }
        assert_eq!(monitor.check(), PlasticityAlert::TooUnstable);
    }
}
