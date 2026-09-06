//! P4-09: Attention curve fitting pipeline.
//!
//! Reads section outcome telemetry, groups by model, and fits per-model
//! `PositionAttentionModel` parameters. Produces curves showing higher
//! attention at the start and end of prompts (primacy/recency effects).

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// Parameters for a U-shaped attention model.
///
/// Models the empirical observation that LLMs attend more strongly to the
/// beginning and end of prompts (primacy and recency effects), with a dip
/// in the middle.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PositionAttentionModel {
    /// Model slug this curve was fit for.
    pub model: String,
    /// Primacy coefficient (attention boost at start).
    pub primacy: f64,
    /// Recency coefficient (attention boost at end).
    pub recency: f64,
    /// Valley depth (attention dip in the middle).
    pub valley: f64,
    /// Number of observations used to fit this curve.
    pub observations: u64,
    /// Goodness of fit (R-squared or similar).
    pub fit_quality: f64,
}

impl Default for PositionAttentionModel {
    fn default() -> Self {
        Self {
            model: String::new(),
            primacy: 1.2,
            recency: 1.1,
            valley: 0.8,
            observations: 0,
            fit_quality: 0.0,
        }
    }
}

impl PositionAttentionModel {
    /// Compute the attention weight at a given relative position in [0, 1].
    ///
    /// Returns a multiplicative weight (1.0 = average attention).
    #[must_use]
    pub fn attention_at(&self, position: f64) -> f64 {
        let position = position.clamp(0.0, 1.0);
        // U-shaped curve: high at 0 and 1, low in the middle.
        // Using a simple quadratic: w(p) = valley + (1 - valley) * (2p - 1)^2
        // then scaled by primacy/recency.
        let base = self.valley + (1.0 - self.valley) * (2.0 * position - 1.0).powi(2);

        // Apply primacy/recency asymmetry.
        let asymmetry = if position < 0.5 {
            self.primacy
        } else {
            self.recency
        };

        (base * asymmetry).clamp(0.1, 3.0)
    }
}

/// Observation for fitting: section position and outcome.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttentionObservation {
    /// Model used.
    pub model: String,
    /// Relative position of the section in [0, 1].
    pub relative_position: f64,
    /// Whether the task with this section at this position succeeded.
    pub success: bool,
    /// Section name for grouping.
    #[serde(default)]
    pub section_name: String,
}

/// Collection of attention curves per model.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AttentionCurveCollection {
    /// Per-model fitted curves.
    pub curves: HashMap<String, PositionAttentionModel>,
}

impl AttentionCurveCollection {
    /// Create a new empty collection.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Fit curves from a set of observations.
    pub fn fit(&mut self, observations: &[AttentionObservation]) {
        // Group by model.
        let mut by_model: HashMap<String, Vec<&AttentionObservation>> = HashMap::new();
        for obs in observations {
            by_model
                .entry(obs.model.clone())
                .or_default()
                .push(obs);
        }

        for (model, model_obs) in &by_model {
            let curve = fit_single_model(model, model_obs);
            self.curves.insert(model.clone(), curve);
        }
    }

    /// Get the attention curve for a model, falling back to a default if not available.
    #[must_use]
    pub fn curve_for(&self, model: &str) -> PositionAttentionModel {
        self.curves
            .get(model)
            .cloned()
            .unwrap_or_else(|| PositionAttentionModel {
                model: model.to_string(),
                ..PositionAttentionModel::default()
            })
    }

    /// Load from disk, or return a new collection if missing/invalid.
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
    /// Returns an error if the collection cannot be serialized or written.
    pub fn save(&self, path: &Path) -> Result<(), std::io::Error> {
        roko_fs::atomic_write_json(path, self)
    }
}

/// Fit a U-shaped attention model for a single model from observations.
fn fit_single_model(model: &str, observations: &[&AttentionObservation]) -> PositionAttentionModel {
    if observations.is_empty() {
        return PositionAttentionModel {
            model: model.to_string(),
            ..PositionAttentionModel::default()
        };
    }

    // Bucket positions into 5 bins and compute success rate per bin.
    let mut bins = [(0u64, 0u64); 5]; // (success, total)
    for obs in observations {
        let bin = ((obs.relative_position * 5.0).floor() as usize).min(4);
        bins[bin].1 += 1;
        if obs.success {
            bins[bin].0 += 1;
        }
    }

    let rates: Vec<f64> = bins
        .iter()
        .map(|(s, t)| {
            if *t == 0 {
                0.5
            } else {
                *s as f64 / *t as f64
            }
        })
        .collect();

    // Estimate primacy from first bin, recency from last bin, valley from middle.
    let avg_rate = rates.iter().sum::<f64>() / rates.len() as f64;
    let primacy = if avg_rate > 0.0 {
        (rates[0] / avg_rate).clamp(0.5, 2.0)
    } else {
        1.2
    };
    let recency = if avg_rate > 0.0 {
        (rates[4] / avg_rate).clamp(0.5, 2.0)
    } else {
        1.1
    };
    let valley = if avg_rate > 0.0 {
        (rates[2] / avg_rate).clamp(0.3, 1.0)
    } else {
        0.8
    };

    // Compute a simple R-squared as fit quality.
    let predicted: Vec<f64> = (0..5)
        .map(|i| {
            let pos = (i as f64 + 0.5) / 5.0;
            let base = valley + (1.0 - valley) * (2.0 * pos - 1.0).powi(2);
            base * if pos < 0.5 { primacy } else { recency }
        })
        .collect();

    let ss_res: f64 = rates
        .iter()
        .zip(predicted.iter())
        .map(|(a, p)| (a - p).powi(2))
        .sum();
    let ss_tot: f64 = rates.iter().map(|r| (r - avg_rate).powi(2)).sum();
    let r_squared = if ss_tot > 0.0 {
        (1.0 - ss_res / ss_tot).clamp(0.0, 1.0)
    } else {
        0.0
    };

    PositionAttentionModel {
        model: model.to_string(),
        primacy,
        recency,
        valley,
        observations: observations.len() as u64,
        fit_quality: r_squared,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_curve_has_u_shape() {
        let curve = PositionAttentionModel::default();
        let start = curve.attention_at(0.0);
        let middle = curve.attention_at(0.5);
        let end = curve.attention_at(1.0);

        assert!(start > middle, "start ({start}) should be higher than middle ({middle})");
        assert!(end > middle, "end ({end}) should be higher than middle ({middle})");
    }

    #[test]
    fn fit_from_observations() {
        let mut collection = AttentionCurveCollection::new();
        let mut observations = Vec::new();

        // Strong primacy: high success at start, low in middle.
        for _ in 0..20 {
            observations.push(AttentionObservation {
                model: "test-model".to_string(),
                relative_position: 0.1,
                success: true,
                section_name: "section_a".to_string(),
            });
        }
        for _ in 0..20 {
            observations.push(AttentionObservation {
                model: "test-model".to_string(),
                relative_position: 0.5,
                success: false,
                section_name: "section_b".to_string(),
            });
        }
        for _ in 0..15 {
            observations.push(AttentionObservation {
                model: "test-model".to_string(),
                relative_position: 0.9,
                success: true,
                section_name: "section_c".to_string(),
            });
        }

        collection.fit(&observations);
        let curve = collection.curve_for("test-model");
        assert!(curve.observations > 0);
        assert!(curve.primacy > 1.0, "primacy should be elevated");
    }

    #[test]
    fn unknown_model_gets_default() {
        let collection = AttentionCurveCollection::new();
        let curve = collection.curve_for("unknown-model");
        assert_eq!(curve.model, "unknown-model");
        assert_eq!(curve.observations, 0);
    }
}
