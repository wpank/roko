//! P4-23: DSPy-style prompt compiler scaffold.
//!
//! Uses coordinate ascent over section variants to find the optimal
//! combination of prompt sections. The initial implementation uses
//! exhaustive search over small variant sets.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Maximum number of sections to optimize simultaneously.
const MAX_SECTIONS: usize = 10;

/// Maximum number of variants per section for exhaustive search.
const MAX_VARIANTS_EXHAUSTIVE: usize = 5;

/// A variant of a prompt section.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SectionVariant {
    /// Variant identifier (e.g., "concise", "verbose", "structured").
    pub variant_id: String,
    /// The actual prompt text for this variant.
    pub content: String,
    /// Token count estimate.
    pub token_count: usize,
}

/// A section with multiple variants to optimize over.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizableSection {
    /// Section name (e.g., "role_context", "task_brief").
    pub section_name: String,
    /// Available variants for this section.
    pub variants: Vec<SectionVariant>,
}

/// Outcome observation for a specific section variant combination.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompilerObservation {
    /// Map of section_name -> variant_id that was used.
    pub assignment: HashMap<String, String>,
    /// Whether the task passed verification.
    pub passed: bool,
    /// Reward signal (0.0 to 1.0, gate pass rate or quality score).
    pub reward: f64,
    /// Cost of this attempt.
    pub cost_usd: f64,
}

/// Result of prompt compilation: the best section variant assignment found.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompilationResult {
    /// Optimal assignment of section_name -> variant_id.
    pub best_assignment: HashMap<String, String>,
    /// Predicted reward for the best assignment.
    pub predicted_reward: f64,
    /// Number of combinations evaluated.
    pub combinations_evaluated: u64,
    /// Method used for optimization.
    pub method: CompilationMethod,
}

/// Method used to find the optimal combination.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CompilationMethod {
    /// Exhaustive enumeration of all combinations.
    Exhaustive,
    /// Coordinate ascent (one section at a time).
    CoordinateAscent,
    /// Random search with limited budget.
    RandomSearch,
}

/// DSPy-style prompt compiler.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PromptCompiler {
    /// Accumulated observations.
    observations: Vec<CompilerObservation>,
    /// Per-(section, variant) aggregate reward.
    variant_rewards: HashMap<(String, String), VariantStats>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct VariantStats {
    total_reward: f64,
    count: u64,
}

impl VariantStats {
    fn mean_reward(&self) -> f64 {
        if self.count == 0 {
            return 0.5; // Prior.
        }
        self.total_reward / self.count as f64
    }
}

impl PromptCompiler {
    /// Create a new compiler.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Record an observation (outcome of a task with a specific variant assignment).
    pub fn observe(&mut self, observation: CompilerObservation) {
        for (section, variant) in &observation.assignment {
            let key = (section.clone(), variant.clone());
            let stats = self.variant_rewards.entry(key).or_default();
            stats.total_reward += observation.reward;
            stats.count += 1;
        }
        self.observations.push(observation);
    }

    /// Compile: find the best variant assignment for the given sections.
    ///
    /// Uses exhaustive search when the combinatorial space is small enough,
    /// otherwise falls back to coordinate ascent.
    #[must_use]
    pub fn compile(&self, sections: &[OptimizableSection]) -> CompilationResult {
        let total_combinations: u64 = sections
            .iter()
            .map(|s| s.variants.len().max(1) as u64)
            .product();

        if sections.len() <= MAX_SECTIONS
            && sections
                .iter()
                .all(|s| s.variants.len() <= MAX_VARIANTS_EXHAUSTIVE)
            && total_combinations <= 1000
        {
            self.compile_exhaustive(sections)
        } else {
            self.compile_coordinate_ascent(sections)
        }
    }

    fn compile_exhaustive(&self, sections: &[OptimizableSection]) -> CompilationResult {
        let mut best_assignment = HashMap::new();
        let mut best_reward = f64::NEG_INFINITY;
        let mut combinations = 0u64;

        // Generate all combinations via index vector.
        let sizes: Vec<usize> = sections.iter().map(|s| s.variants.len().max(1)).collect();
        let mut indices = vec![0usize; sections.len()];

        loop {
            let mut assignment = HashMap::new();
            let mut predicted = 0.0;
            let mut valid = true;

            for (i, section) in sections.iter().enumerate() {
                if section.variants.is_empty() {
                    valid = false;
                    break;
                }
                let variant = &section.variants[indices[i]];
                assignment.insert(section.section_name.clone(), variant.variant_id.clone());

                let key = (section.section_name.clone(), variant.variant_id.clone());
                predicted += self
                    .variant_rewards
                    .get(&key)
                    .map(|s| s.mean_reward())
                    .unwrap_or(0.5);
            }

            if valid {
                combinations += 1;
                let avg_predicted = predicted / sections.len().max(1) as f64;
                if avg_predicted > best_reward {
                    best_reward = avg_predicted;
                    best_assignment = assignment;
                }
            }

            // Increment index vector.
            let mut carry = true;
            for i in (0..indices.len()).rev() {
                if carry {
                    indices[i] += 1;
                    if indices[i] >= sizes[i] {
                        indices[i] = 0;
                    } else {
                        carry = false;
                    }
                }
            }
            if carry {
                break; // All combinations exhausted.
            }
        }

        CompilationResult {
            best_assignment,
            predicted_reward: best_reward,
            combinations_evaluated: combinations,
            method: CompilationMethod::Exhaustive,
        }
    }

    fn compile_coordinate_ascent(&self, sections: &[OptimizableSection]) -> CompilationResult {
        // Start with the best individual variant per section.
        let mut current = HashMap::new();
        let mut combinations = 0u64;

        for section in sections {
            let mut best_variant = section
                .variants
                .first()
                .map(|v| v.variant_id.clone())
                .unwrap_or_default();
            let mut best_reward = f64::NEG_INFINITY;

            for variant in &section.variants {
                let key = (section.section_name.clone(), variant.variant_id.clone());
                let reward = self
                    .variant_rewards
                    .get(&key)
                    .map(|s| s.mean_reward())
                    .unwrap_or(0.5);
                combinations += 1;
                if reward > best_reward {
                    best_reward = reward;
                    best_variant = variant.variant_id.clone();
                }
            }
            current.insert(section.section_name.clone(), best_variant);
        }

        // One pass of coordinate ascent: try improving each section holding others fixed.
        for section in sections {
            let mut best_variant = current[&section.section_name].clone();
            let mut best_total = f64::NEG_INFINITY;

            for variant in &section.variants {
                let mut trial = current.clone();
                trial.insert(section.section_name.clone(), variant.variant_id.clone());
                let total: f64 = trial
                    .iter()
                    .map(|(s, v)| {
                        self.variant_rewards
                            .get(&(s.clone(), v.clone()))
                            .map(|stats| stats.mean_reward())
                            .unwrap_or(0.5)
                    })
                    .sum();
                combinations += 1;
                if total > best_total {
                    best_total = total;
                    best_variant = variant.variant_id.clone();
                }
            }
            current.insert(section.section_name.clone(), best_variant);
        }

        let predicted = current
            .iter()
            .map(|(s, v)| {
                self.variant_rewards
                    .get(&(s.clone(), v.clone()))
                    .map(|stats| stats.mean_reward())
                    .unwrap_or(0.5)
            })
            .sum::<f64>()
            / sections.len().max(1) as f64;

        CompilationResult {
            best_assignment: current,
            predicted_reward: predicted,
            combinations_evaluated: combinations,
            method: CompilationMethod::CoordinateAscent,
        }
    }

    /// Load from disk, or return a new compiler if missing/invalid.
    #[must_use]
    pub fn load_or_new(path: &std::path::Path) -> Self {
        std::fs::read_to_string(path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    /// Save to disk using atomic write.
    ///
    /// # Errors
    ///
    /// Returns an error if the compiler cannot be serialized or written.
    pub fn save(&self, path: &std::path::Path) -> Result<(), std::io::Error> {
        roko_fs::atomic_write_json(path, self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_sections() -> Vec<OptimizableSection> {
        vec![
            OptimizableSection {
                section_name: "role_context".to_string(),
                variants: vec![
                    SectionVariant {
                        variant_id: "concise".to_string(),
                        content: "You are a concise implementer.".to_string(),
                        token_count: 6,
                    },
                    SectionVariant {
                        variant_id: "verbose".to_string(),
                        content: "You are an implementer. Be thorough and detailed.".to_string(),
                        token_count: 10,
                    },
                ],
            },
            OptimizableSection {
                section_name: "task_brief".to_string(),
                variants: vec![
                    SectionVariant {
                        variant_id: "minimal".to_string(),
                        content: "Fix the bug.".to_string(),
                        token_count: 3,
                    },
                    SectionVariant {
                        variant_id: "detailed".to_string(),
                        content: "Fix the bug by examining the error trace.".to_string(),
                        token_count: 8,
                    },
                ],
            },
        ]
    }

    #[test]
    fn exhaustive_search_finds_best() {
        let mut compiler = PromptCompiler::new();

        // Train: "concise" role + "detailed" task works best.
        for _ in 0..10 {
            compiler.observe(CompilerObservation {
                assignment: HashMap::from([
                    ("role_context".to_string(), "concise".to_string()),
                    ("task_brief".to_string(), "detailed".to_string()),
                ]),
                passed: true,
                reward: 0.9,
                cost_usd: 0.01,
            });
        }
        for _ in 0..10 {
            compiler.observe(CompilerObservation {
                assignment: HashMap::from([
                    ("role_context".to_string(), "verbose".to_string()),
                    ("task_brief".to_string(), "minimal".to_string()),
                ]),
                passed: false,
                reward: 0.2,
                cost_usd: 0.02,
            });
        }

        let result = compiler.compile(&make_sections());
        assert_eq!(result.method, CompilationMethod::Exhaustive);
        assert_eq!(
            result.best_assignment.get("role_context"),
            Some(&"concise".to_string())
        );
        assert_eq!(
            result.best_assignment.get("task_brief"),
            Some(&"detailed".to_string())
        );
    }

    #[test]
    fn empty_compiler_returns_defaults() {
        let compiler = PromptCompiler::new();
        let result = compiler.compile(&make_sections());
        assert_eq!(result.combinations_evaluated, 4); // 2x2.
        assert!(!result.best_assignment.is_empty());
    }
}
