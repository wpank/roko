//! P4-05: Learning effectiveness meta-metrics for triple-loop learning.
//!
//! Computes and tracks meta-metrics about the learning system itself:
//! rule precision, cascade convergence rate, and section lift stability.
//! These metrics enable monitoring whether the learning subsystems are
//! actually improving over time.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// Meta-metrics about overall learning system effectiveness.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LearningEffectivenessReport {
    /// Fraction of extracted rules that were validated (not contradicted).
    pub rule_precision: f64,
    /// Number of rules extracted vs contradicted.
    pub rules_extracted: u64,
    /// Number of rules contradicted (false positives).
    pub rules_contradicted: u64,
    /// Whether the cascade router has converged (low exploration, stable selections).
    pub cascade_converged: bool,
    /// Cascade router observation count.
    pub cascade_observations: u64,
    /// Current exploration parameter (alpha).
    pub cascade_alpha: f64,
    /// Per-section lift stability: standard deviation of lift over recent windows.
    pub section_lift_stability: HashMap<String, f64>,
    /// Average section lift stability across all sections.
    pub avg_lift_stability: f64,
    /// Overall learning effectiveness score (0.0 to 1.0).
    pub overall_score: f64,
    /// Playbook success rate across all playbooks.
    pub playbook_success_rate: f64,
    /// Knowledge reuse rate (entries injected / entries available).
    pub knowledge_reuse_rate: f64,
    /// Number of closed feedback loops detected.
    pub closed_loops: u64,
}

impl LearningEffectivenessReport {
    /// Compute the overall effectiveness score from component metrics.
    #[must_use]
    pub fn compute_overall(&mut self) -> f64 {
        let precision_score = self.rule_precision;
        let convergence_score = if self.cascade_converged { 1.0 } else { 0.5 };
        let stability_score = (1.0 - self.avg_lift_stability).clamp(0.0, 1.0);
        let playbook_score = self.playbook_success_rate;

        self.overall_score =
            (precision_score * 0.3 + convergence_score * 0.2 + stability_score * 0.2 + playbook_score * 0.3)
                .clamp(0.0, 1.0);
        self.overall_score
    }
}

/// Tracker that accumulates section lift values over time windows
/// and computes stability metrics.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LiftStabilityTracker {
    /// Per-section rolling lift history (most recent N values).
    history: HashMap<String, Vec<f64>>,
    /// Maximum history length per section.
    max_history: usize,
}

impl LiftStabilityTracker {
    /// Create a new tracker with the given history window.
    #[must_use]
    pub fn new(max_history: usize) -> Self {
        Self {
            history: HashMap::new(),
            max_history: max_history.max(2),
        }
    }

    /// Record a lift observation for a section.
    pub fn record(&mut self, section: &str, lift: f64) {
        let history = self
            .history
            .entry(section.to_string())
            .or_default();
        history.push(lift);
        if history.len() > self.max_history {
            history.remove(0);
        }
    }

    /// Compute the standard deviation of lift for a section.
    #[must_use]
    pub fn stability(&self, section: &str) -> Option<f64> {
        let history = self.history.get(section)?;
        if history.len() < 2 {
            return None;
        }
        let mean = history.iter().sum::<f64>() / history.len() as f64;
        let variance = history.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / history.len() as f64;
        Some(variance.sqrt())
    }

    /// Compute stability for all tracked sections.
    #[must_use]
    pub fn all_stabilities(&self) -> HashMap<String, f64> {
        self.history
            .keys()
            .filter_map(|section| {
                self.stability(section).map(|s| (section.clone(), s))
            })
            .collect()
    }

    /// Average stability across all sections.
    #[must_use]
    pub fn average_stability(&self) -> f64 {
        let stabilities = self.all_stabilities();
        if stabilities.is_empty() {
            return 0.0;
        }
        stabilities.values().sum::<f64>() / stabilities.len() as f64
    }
}

/// Load a learning effectiveness report from disk.
///
/// # Errors
///
/// Returns an error if the file cannot be read or parsed.
pub fn load_report(path: &Path) -> Result<LearningEffectivenessReport, std::io::Error> {
    let contents = std::fs::read_to_string(path)?;
    serde_json::from_str(&contents).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
}

/// Save a learning effectiveness report to disk.
///
/// # Errors
///
/// Returns an error if the report cannot be serialized or written.
pub fn save_report(path: &Path, report: &LearningEffectivenessReport) -> Result<(), std::io::Error> {
    roko_fs::atomic_write_json(path, report)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lift_stability_tracker_computes_std_dev() {
        let mut tracker = LiftStabilityTracker::new(10);
        tracker.record("section_a", 0.5);
        tracker.record("section_a", 0.5);
        tracker.record("section_a", 0.5);

        let stability = tracker.stability("section_a").unwrap();
        assert!(stability < 0.001, "identical values should have near-zero std dev");
    }

    #[test]
    fn lift_stability_tracker_handles_varying_values() {
        let mut tracker = LiftStabilityTracker::new(10);
        tracker.record("section_b", 0.0);
        tracker.record("section_b", 1.0);

        let stability = tracker.stability("section_b").unwrap();
        assert!(stability > 0.4, "divergent values should have high std dev");
    }

    #[test]
    fn report_computes_overall_score() {
        let mut report = LearningEffectivenessReport {
            rule_precision: 0.8,
            cascade_converged: true,
            playbook_success_rate: 0.7,
            avg_lift_stability: 0.1,
            ..Default::default()
        };
        let score = report.compute_overall();
        assert!(score > 0.5, "good metrics should yield high overall score: {score}");
    }

    #[test]
    fn tracker_respects_max_history() {
        let mut tracker = LiftStabilityTracker::new(3);
        for i in 0..10 {
            tracker.record("s", i as f64);
        }
        assert_eq!(tracker.history["s"].len(), 3);
    }
}
