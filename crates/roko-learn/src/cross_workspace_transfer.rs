//! P4-25: Cross-workspace prompt learning transfer.
//!
//! Exports section effectiveness, playbook rules, and gate threshold data
//! from one workspace and imports them into another with decay applied.
//! This enables bootstrapping new workspaces from mature ones.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// Default decay factor applied to transferred data.
const DEFAULT_TRANSFER_DECAY: f64 = 0.5;

/// A transferable learning bundle from one workspace.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningBundle {
    /// Source workspace identifier.
    pub source_workspace: String,
    /// Export timestamp.
    pub exported_at: String,
    /// Section effectiveness data keyed by (section, role).
    pub section_effects: Vec<TransferredSectionEffect>,
    /// Playbook rules.
    pub playbook_rules: Vec<TransferredPlaybook>,
    /// Gate threshold priors.
    pub gate_priors: HashMap<u32, f64>,
    /// Model routing preferences.
    pub model_preferences: Vec<TransferredModelPreference>,
}

/// Section effectiveness data suitable for transfer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferredSectionEffect {
    /// Section name.
    pub section_name: String,
    /// Role.
    pub role: String,
    /// Observed lift.
    pub lift: f64,
    /// Number of trials (used for weighting).
    pub trials: u64,
}

/// Playbook suitable for transfer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferredPlaybook {
    /// Playbook name/identifier.
    pub name: String,
    /// When condition.
    pub when_pattern: String,
    /// Then action.
    pub then_action: String,
    /// Historical success rate.
    pub success_rate: f64,
    /// Number of times applied.
    pub applications: u64,
}

/// Model preference suitable for transfer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferredModelPreference {
    /// Task category.
    pub task_category: String,
    /// Preferred model slug.
    pub model: String,
    /// Reward observed.
    pub reward: f64,
    /// Number of observations.
    pub observations: u64,
}

/// Import options controlling how transferred data is applied.
#[derive(Debug, Clone)]
pub struct ImportOptions {
    /// Decay factor (0.0 to 1.0): how much to discount transferred data.
    pub decay: f64,
    /// Whether to override existing data or only fill gaps.
    pub override_existing: bool,
    /// Minimum trials required to import a section effect.
    pub min_trials: u64,
    /// Minimum applications required to import a playbook.
    pub min_applications: u64,
}

impl Default for ImportOptions {
    fn default() -> Self {
        Self {
            decay: DEFAULT_TRANSFER_DECAY,
            override_existing: false,
            min_trials: 10,
            min_applications: 3,
        }
    }
}

/// Result of importing a learning bundle.
#[derive(Debug, Clone, Default)]
pub struct ImportResult {
    /// Number of section effects imported.
    pub section_effects_imported: usize,
    /// Number of playbooks imported.
    pub playbooks_imported: usize,
    /// Number of gate priors imported.
    pub gate_priors_imported: usize,
    /// Number of model preferences imported.
    pub model_preferences_imported: usize,
    /// Number of items skipped (below thresholds).
    pub skipped: usize,
}

/// Export a learning bundle from the given workspace path.
///
/// # Errors
///
/// Returns an error if the workspace learning data cannot be read.
pub fn export_bundle(
    workspace_path: &Path,
    workspace_id: &str,
) -> Result<LearningBundle, std::io::Error> {
    let learn_dir = workspace_path.join(".roko/learn");

    // Load section effects.
    let section_effects = load_section_effects(&learn_dir.join("section-effects.json"))?;

    // Load gate thresholds.
    let gate_priors = load_gate_priors(&learn_dir.join("gate-thresholds.json"))?;

    Ok(LearningBundle {
        source_workspace: workspace_id.to_string(),
        exported_at: chrono::Utc::now().to_rfc3339(),
        section_effects,
        playbook_rules: Vec::new(), // Filled by caller from playbook store.
        gate_priors,
        model_preferences: Vec::new(), // Filled by caller from router.
    })
}

fn load_section_effects(path: &Path) -> Result<Vec<TransferredSectionEffect>, std::io::Error> {
    let contents = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(e),
    };

    // Parse the section effects snapshot format.
    let value: serde_json::Value = serde_json::from_str(&contents)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

    let mut effects = Vec::new();
    if let Some(entries) = value.get("entries").and_then(|e| e.as_array()) {
        for entry in entries {
            let section_name = entry
                .get("section_name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let role = entry
                .get("role")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let effect = entry.get("effect");
            let included_trials = effect
                .and_then(|e| e.get("included_trials"))
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            let included_passes = effect
                .and_then(|e| e.get("included_passes"))
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            let excluded_trials = effect
                .and_then(|e| e.get("excluded_trials"))
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            let excluded_passes = effect
                .and_then(|e| e.get("excluded_passes"))
                .and_then(|v| v.as_u64())
                .unwrap_or(0);

            let inc_rate = if included_trials > 0 {
                included_passes as f64 / included_trials as f64
            } else {
                0.0
            };
            let exc_rate = if excluded_trials > 0 {
                excluded_passes as f64 / excluded_trials as f64
            } else {
                0.0
            };
            let lift = inc_rate - exc_rate;
            let trials = included_trials + excluded_trials;

            effects.push(TransferredSectionEffect {
                section_name,
                role,
                lift,
                trials,
            });
        }
    }

    Ok(effects)
}

fn load_gate_priors(path: &Path) -> Result<HashMap<u32, f64>, std::io::Error> {
    let contents = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(HashMap::new()),
        Err(e) => return Err(e),
    };

    let value: serde_json::Value = serde_json::from_str(&contents)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

    let mut priors = HashMap::new();
    if let Some(rungs) = value.get("rungs").and_then(|r| r.as_object()) {
        for (rung_str, stats) in rungs {
            if let Ok(rung) = rung_str.parse::<u32>() {
                if let Some(ema) = stats.get("ema_pass_rate").and_then(|v| v.as_f64()) {
                    priors.insert(rung, ema);
                }
            }
        }
    }

    Ok(priors)
}

/// Apply decay to transferred data values.
fn apply_decay(value: f64, decay: f64) -> f64 {
    // Move the value toward neutral (0.5) by the decay factor.
    let neutral = 0.5;
    neutral + (value - neutral) * decay
}

/// Filter a bundle through import options.
///
/// Returns a filtered bundle with only items that meet the thresholds,
/// and an [`ImportResult`] with statistics.
#[must_use]
pub fn filter_bundle(
    bundle: &LearningBundle,
    options: &ImportOptions,
) -> (LearningBundle, ImportResult) {
    let mut result = ImportResult::default();
    let mut filtered = bundle.clone();

    // Filter section effects.
    filtered.section_effects.retain(|effect| {
        if effect.trials >= options.min_trials {
            result.section_effects_imported += 1;
            true
        } else {
            result.skipped += 1;
            false
        }
    });

    // Apply decay to lifts.
    for effect in &mut filtered.section_effects {
        effect.lift = apply_decay(effect.lift, options.decay);
    }

    // Filter playbooks.
    filtered.playbook_rules.retain(|pb| {
        if pb.applications >= options.min_applications {
            result.playbooks_imported += 1;
            true
        } else {
            result.skipped += 1;
            false
        }
    });

    // Apply decay to playbook success rates.
    for pb in &mut filtered.playbook_rules {
        pb.success_rate = apply_decay(pb.success_rate, options.decay);
    }

    // Gate priors are imported directly (with decay).
    for prior in filtered.gate_priors.values_mut() {
        *prior = apply_decay(*prior, options.decay);
        result.gate_priors_imported += 1;
    }

    result.model_preferences_imported = filtered.model_preferences.len();

    (filtered, result)
}

/// Save a learning bundle to a JSON file.
///
/// # Errors
///
/// Returns an error if the bundle cannot be serialized or written.
pub fn save_bundle(path: &Path, bundle: &LearningBundle) -> Result<(), std::io::Error> {
    roko_fs::atomic_write_json(path, bundle)
}

/// Load a learning bundle from a JSON file.
///
/// # Errors
///
/// Returns an error if the file cannot be read or parsed.
pub fn load_bundle(path: &Path) -> Result<LearningBundle, std::io::Error> {
    let contents = std::fs::read_to_string(path)?;
    serde_json::from_str(&contents)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decay_moves_toward_neutral() {
        assert!((apply_decay(1.0, 0.5) - 0.75).abs() < 1e-10);
        assert!((apply_decay(0.0, 0.5) - 0.25).abs() < 1e-10);
        assert!((apply_decay(0.5, 0.5) - 0.5).abs() < 1e-10);
    }

    #[test]
    fn filter_bundle_respects_thresholds() {
        let bundle = LearningBundle {
            source_workspace: "test".into(),
            exported_at: "now".into(),
            section_effects: vec![
                TransferredSectionEffect {
                    section_name: "high".into(),
                    role: "impl".into(),
                    lift: 0.3,
                    trials: 50,
                },
                TransferredSectionEffect {
                    section_name: "low".into(),
                    role: "impl".into(),
                    lift: 0.1,
                    trials: 2,
                },
            ],
            playbook_rules: Vec::new(),
            gate_priors: HashMap::new(),
            model_preferences: Vec::new(),
        };

        let options = ImportOptions::default();
        let (filtered, result) = filter_bundle(&bundle, &options);
        assert_eq!(result.section_effects_imported, 1);
        assert_eq!(result.skipped, 1);
        assert_eq!(filtered.section_effects.len(), 1);
        assert_eq!(filtered.section_effects[0].section_name, "high");
    }
}
