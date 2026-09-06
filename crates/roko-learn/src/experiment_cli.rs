//! P4-03 / P4-04: Experiment creation and automatic wiring.
//!
//! Provides the data types and helpers for creating prompt experiments
//! via CLI and for automatically creating experiments from gate outcome
//! data.

use serde::{Deserialize, Serialize};
use std::path::Path;

/// Experiment definition that can be created from CLI or automatically.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperimentDefinition {
    /// Unique experiment identifier.
    pub id: String,
    /// Human-readable name.
    pub name: String,
    /// Section being tested.
    pub section_name: String,
    /// Control variant (current section content).
    pub control_variant: String,
    /// Treatment variant (new section content).
    pub treatment_variant: String,
    /// Traffic allocation for treatment (0.0 to 1.0).
    pub traffic_share: f64,
    /// Whether the experiment is active.
    pub active: bool,
    /// Minimum observations before drawing conclusions.
    pub min_observations: u64,
    /// Creation source (cli, auto, gate_feedback).
    pub source: ExperimentSource,
}

/// How the experiment was created.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExperimentSource {
    /// Created via `roko experiment create` CLI.
    Cli,
    /// Automatically created from gate outcome analysis.
    Automatic,
    /// Created from gate feedback loop.
    GateFeedback,
}

/// Summary of an experiment's current state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperimentSummary {
    /// Experiment definition.
    pub definition: ExperimentDefinition,
    /// Control group observations.
    pub control_observations: u64,
    /// Control group success rate.
    pub control_success_rate: f64,
    /// Treatment group observations.
    pub treatment_observations: u64,
    /// Treatment group success rate.
    pub treatment_success_rate: f64,
    /// Whether we have enough data to conclude.
    pub conclusive: bool,
    /// Whether the treatment is significantly better.
    pub treatment_wins: bool,
}

impl ExperimentSummary {
    /// Compute effect size (treatment - control success rate).
    #[must_use]
    pub fn effect_size(&self) -> f64 {
        self.treatment_success_rate - self.control_success_rate
    }

    /// Whether we should promote the treatment to production.
    #[must_use]
    pub fn should_promote(&self) -> bool {
        self.conclusive && self.treatment_wins && self.effect_size() > 0.05
    }
}

/// Experiment store backed by a JSON file.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ExperimentStore {
    /// Active experiments.
    pub experiments: Vec<ExperimentDefinition>,
}

impl ExperimentStore {
    /// Load from disk, or return an empty store.
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
    /// Returns an error if the store cannot be serialized or written.
    pub fn save(&self, path: &Path) -> Result<(), std::io::Error> {
        roko_fs::atomic_write_json(path, self)
    }

    /// Add a new experiment.
    pub fn add(&mut self, experiment: ExperimentDefinition) {
        self.experiments.push(experiment);
    }

    /// Find an experiment by ID.
    #[must_use]
    pub fn get(&self, id: &str) -> Option<&ExperimentDefinition> {
        self.experiments.iter().find(|e| e.id == id)
    }

    /// List all active experiments.
    #[must_use]
    pub fn active_experiments(&self) -> Vec<&ExperimentDefinition> {
        self.experiments.iter().filter(|e| e.active).collect()
    }

    /// Deactivate an experiment by ID.
    pub fn deactivate(&mut self, id: &str) -> bool {
        if let Some(exp) = self.experiments.iter_mut().find(|e| e.id == id) {
            exp.active = false;
            true
        } else {
            false
        }
    }
}

/// Auto-generate experiment candidates from low-lift sections.
///
/// Scans section effects and proposes experiments for sections with
/// negative or marginal lift, testing shorter/different versions.
#[must_use]
pub fn auto_generate_candidates(
    low_lift_sections: &[(String, String, f64)], // (section_name, role, lift)
) -> Vec<ExperimentDefinition> {
    low_lift_sections
        .iter()
        .filter(|(_, _, lift)| *lift < 0.05) // Only experiment on low/negative lift.
        .map(|(section_name, _role, lift)| ExperimentDefinition {
            id: format!("auto-{section_name}-{}", chrono::Utc::now().timestamp()),
            name: format!("Auto-experiment: {section_name} (lift={lift:.3})"),
            section_name: section_name.clone(),
            control_variant: "current".to_string(),
            treatment_variant: "omitted".to_string(),
            traffic_share: 0.2, // 20% treatment.
            active: true,
            min_observations: 30,
            source: ExperimentSource::Automatic,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auto_generate_skips_positive_lift() {
        let sections = vec![
            ("good_section".into(), "impl".into(), 0.3),
            ("bad_section".into(), "impl".into(), -0.1),
        ];
        let candidates = auto_generate_candidates(&sections);
        assert_eq!(candidates.len(), 1);
        assert!(candidates[0].section_name.contains("bad_section"));
    }

    #[test]
    fn experiment_summary_promotion() {
        let summary = ExperimentSummary {
            definition: ExperimentDefinition {
                id: "test".into(),
                name: "test".into(),
                section_name: "sec".into(),
                control_variant: "a".into(),
                treatment_variant: "b".into(),
                traffic_share: 0.5,
                active: true,
                min_observations: 10,
                source: ExperimentSource::Cli,
            },
            control_observations: 50,
            control_success_rate: 0.6,
            treatment_observations: 50,
            treatment_success_rate: 0.8,
            conclusive: true,
            treatment_wins: true,
        };
        assert!(summary.should_promote());
    }

    #[test]
    fn store_operations() {
        let mut store = ExperimentStore::default();
        store.add(ExperimentDefinition {
            id: "exp-1".into(),
            name: "test".into(),
            section_name: "sec".into(),
            control_variant: "a".into(),
            treatment_variant: "b".into(),
            traffic_share: 0.5,
            active: true,
            min_observations: 10,
            source: ExperimentSource::Cli,
        });
        assert!(store.get("exp-1").is_some());
        assert_eq!(store.active_experiments().len(), 1);
        store.deactivate("exp-1");
        assert_eq!(store.active_experiments().len(), 0);
    }
}
