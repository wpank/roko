//! COMP-10: Budget prediction from task features and section influence scoring.
//!
//! [`BudgetPredictor`] predicts the optimal token budget for a task based on
//! its features (complexity, role, domain) and historical efficiency data. It
//! uses exponential moving average (EMA) of actual token usage per feature
//! combination to converge on a good budget without over- or under-allocating.
//!
//! [`SectionInfluence`] measures each prompt section's impact on task success
//! via a leave-one-out approximation: for each section, it tracks the success
//! rate of tasks that included the section vs the global baseline. Sections
//! with positive lift get higher weights; sections with negative lift get
//! lower weights or are dropped.
//!
//! # Integration
//!
//! The predictor is meant to be called from the composition layer before
//! assembling the prompt:
//!
//! 1. `BudgetPredictor::predict()` returns an estimated token budget.
//! 2. `SectionInfluence::weights()` returns per-section multipliers.
//! 3. These feed into `PromptComposer` to adjust section token caps and
//!    prioritization.
//!
//! # Persistence
//!
//! Both structs are serde-serializable and intended for storage in
//! `.roko/learn/budget-predictor.json` and `.roko/learn/section-influence.json`.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Task features
// ---------------------------------------------------------------------------

/// Features describing a task for budget prediction.
///
/// These are the inputs to the predictor. The combination of `role`,
/// `complexity`, and `domain` forms the feature key.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct TaskFeatures {
    /// Agent role (e.g., "Implementer", "Reviewer", "Researcher").
    pub role: String,
    /// Complexity band (e.g., "trivial", "standard", "complex").
    pub complexity: String,
    /// Task domain (e.g., "code", "research", "docs", "chain").
    pub domain: String,
}

impl TaskFeatures {
    /// The canonical key used for lookups.
    #[must_use]
    pub fn key(&self) -> String {
        format!("{}:{}:{}", self.role, self.complexity, self.domain)
    }

    /// Create features from component strings.
    #[must_use]
    pub fn new(
        role: impl Into<String>,
        complexity: impl Into<String>,
        domain: impl Into<String>,
    ) -> Self {
        Self {
            role: role.into(),
            complexity: complexity.into(),
            domain: domain.into(),
        }
    }
}

// ---------------------------------------------------------------------------
// BudgetPredictor
// ---------------------------------------------------------------------------

/// Observation record for one task execution.
#[derive(Clone, Debug, Serialize, Deserialize)]
struct BudgetObservation {
    /// EMA of actual tokens used for this feature combination.
    ema_tokens: f64,
    /// EMA of success rate (0.0..1.0).
    ema_success: f64,
    /// Number of observations.
    count: u32,
}

/// Predicts optimal token budgets from historical task data.
///
/// Uses per-feature-key EMA (exponential moving average) of actual token
/// usage, weighted by task success. When a task succeeds within budget,
/// the EMA converges toward the actual usage. When a task fails, the
/// predictor inflates the budget for that feature key.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BudgetPredictor {
    /// Per-feature-key observations.
    observations: HashMap<String, BudgetObservation>,
    /// EMA smoothing factor (0.0..1.0). Higher = more weight on recent data.
    /// Default: 0.3.
    #[serde(default = "default_alpha")]
    pub alpha: f64,
    /// Fallback budget when no history is available.
    #[serde(default = "default_fallback_tokens")]
    pub fallback_tokens: u64,
    /// Inflation factor applied when a task fails (budget was too small).
    #[serde(default = "default_failure_inflation")]
    pub failure_inflation: f64,
}

impl Default for BudgetPredictor {
    fn default() -> Self {
        Self {
            observations: HashMap::new(),
            alpha: default_alpha(),
            fallback_tokens: default_fallback_tokens(),
            failure_inflation: default_failure_inflation(),
        }
    }
}

fn default_alpha() -> f64 {
    0.3
}

fn default_fallback_tokens() -> u64 {
    100_000
}

fn default_failure_inflation() -> f64 {
    1.3
}

impl BudgetPredictor {
    /// Create a new predictor with default parameters.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Predict the optimal token budget for a task.
    ///
    /// Returns the EMA of actual usage for the feature key, inflated by
    /// 20% as a safety margin. If no history exists, returns the fallback
    /// budget.
    #[must_use]
    pub fn predict(&self, features: &TaskFeatures) -> u64 {
        let key = features.key();
        if let Some(obs) = self.observations.get(&key) {
            // Add 20% safety margin over the EMA.
            #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
            let predicted = (obs.ema_tokens * 1.2) as u64;
            predicted.max(1000) // minimum 1K tokens
        } else {
            // Try partial matches: same role+complexity, any domain.
            let partial_key = format!("{}:{}:", features.role, features.complexity);
            let partial_matches: Vec<&BudgetObservation> = self
                .observations
                .iter()
                .filter(|(k, _)| k.starts_with(&partial_key))
                .map(|(_, v)| v)
                .collect();

            if partial_matches.is_empty() {
                self.fallback_tokens
            } else {
                let avg_tokens: f64 = partial_matches.iter().map(|o| o.ema_tokens).sum::<f64>()
                    / partial_matches.len() as f64;
                #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
                let predicted = (avg_tokens * 1.2) as u64;
                predicted.max(1000)
            }
        }
    }

    /// Record the outcome of a task execution.
    ///
    /// Updates the EMA for the feature key based on actual token usage
    /// and whether the task succeeded.
    pub fn record(&mut self, features: &TaskFeatures, actual_tokens: u64, success: bool) {
        let key = features.key();
        #[allow(clippy::cast_precision_loss)]
        let actual = actual_tokens as f64;

        if let Some(obs) = self.observations.get_mut(&key) {
            // Existing observation: EMA update.
            let prev = obs.ema_tokens;
            obs.count += 1;
            obs.ema_tokens = self.alpha.mul_add(actual, (1.0 - self.alpha) * prev);
            let success_val = if success { 1.0 } else { 0.0 };
            obs.ema_success = self.alpha * success_val + (1.0 - self.alpha) * obs.ema_success;
        } else {
            // First observation: use actuals directly.
            self.observations.insert(
                key.clone(),
                BudgetObservation {
                    ema_tokens: actual,
                    ema_success: if success { 1.0 } else { 0.0 },
                    count: 1,
                },
            );
        }

        // If the task failed, inflate the EMA to encourage a larger budget next time.
        if !success && let Some(obs) = self.observations.get_mut(&key) {
            obs.ema_tokens *= self.failure_inflation;
        }
    }

    /// Number of unique feature keys with observations.
    #[must_use]
    pub fn observation_count(&self) -> usize {
        self.observations.len()
    }

    /// Whether there is any history for the given features.
    #[must_use]
    pub fn has_history(&self, features: &TaskFeatures) -> bool {
        self.observations.contains_key(&features.key())
    }

    /// Seed a feature key with a pre-computed EMA and success rate.
    ///
    /// Used by [`calibrate_from_efficiency`] to bootstrap the predictor from
    /// historical efficiency data without going through the incremental EMA
    /// update path.  Only inserts when the key does not already have an entry
    /// (existing observations have higher quality and should not be overwritten).
    pub(crate) fn seed_observation(
        &mut self,
        key: impl Into<String>,
        ema_tokens: f64,
        ema_success: f64,
        count: u32,
    ) {
        let key = key.into();
        self.observations.entry(key).or_insert(BudgetObservation {
            ema_tokens,
            ema_success,
            count,
        });
    }
}

// ---------------------------------------------------------------------------
// SectionInfluence
// ---------------------------------------------------------------------------

/// Tracks per-section success statistics for leave-one-out influence scoring.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
struct SectionRecord {
    /// Number of tasks that included this section and succeeded.
    successes_with: u32,
    /// Number of tasks that included this section and failed.
    failures_with: u32,
    /// Number of tasks that excluded this section and succeeded.
    successes_without: u32,
    /// Number of tasks that excluded this section and failed.
    failures_without: u32,
}

impl SectionRecord {
    /// Success rate when section is included.
    fn rate_with(&self) -> f64 {
        let total = self.successes_with + self.failures_with;
        if total == 0 {
            0.5 // neutral prior
        } else {
            f64::from(self.successes_with) / f64::from(total)
        }
    }

    /// Success rate when section is excluded.
    fn rate_without(&self) -> f64 {
        let total = self.successes_without + self.failures_without;
        if total == 0 {
            0.5 // neutral prior
        } else {
            f64::from(self.successes_without) / f64::from(total)
        }
    }

    /// Lift: how much the section improves success rate.
    ///
    /// Positive lift means the section helps; negative means it hurts.
    fn lift(&self) -> f64 {
        self.rate_with() - self.rate_without()
    }

    /// Total observations for this section.
    fn total_obs(&self) -> u32 {
        self.successes_with + self.failures_with + self.successes_without + self.failures_without
    }
}

/// Leave-one-out section influence scorer (COMP-10).
///
/// For each prompt section, tracks whether its presence correlates with
/// task success. Sections with positive lift should receive higher token
/// budgets; sections with negative lift should be dropped or deprioritized.
///
/// # Approximation
///
/// True leave-one-out requires re-running each task without each section,
/// which is prohibitively expensive. Instead, we observe natural variation:
/// some tasks include a section (e.g., because it was available), others
/// do not (e.g., because context was missing). Over many observations,
/// this approximates the causal effect.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SectionInfluence {
    /// Per-section statistics, keyed by section name (e.g., "prd2", "context").
    sections: HashMap<String, SectionRecord>,
    /// Minimum observations before influence scores are trusted.
    #[serde(default = "default_min_obs")]
    pub min_observations: u32,
}

impl Default for SectionInfluence {
    fn default() -> Self {
        Self {
            sections: HashMap::new(),
            min_observations: default_min_obs(),
        }
    }
}

fn default_min_obs() -> u32 {
    10
}

impl SectionInfluence {
    /// Create a new influence tracker.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Record the outcome of a task execution.
    ///
    /// `included_sections` is the set of section names that were present
    /// in the prompt. `all_sections` is the set of all known section names.
    /// `success` indicates whether the task succeeded.
    pub fn record(&mut self, included_sections: &[String], all_sections: &[String], success: bool) {
        let included_set: std::collections::HashSet<&String> = included_sections.iter().collect();

        for section in all_sections {
            let record = self.sections.entry(section.clone()).or_default();
            if included_set.contains(section) {
                if success {
                    record.successes_with += 1;
                } else {
                    record.failures_with += 1;
                }
            } else if success {
                record.successes_without += 1;
            } else {
                record.failures_without += 1;
            }
        }
    }

    /// Compute per-section weight multipliers.
    ///
    /// Returns a map from section name to a multiplier in `[0.5, 1.5]`.
    /// Sections with insufficient observations get 1.0 (neutral).
    /// Positive lift maps to >1.0; negative lift maps to <1.0.
    #[must_use]
    pub fn weights(&self) -> HashMap<String, f64> {
        self.sections
            .iter()
            .map(|(name, record)| {
                let weight = if record.total_obs() < self.min_observations {
                    1.0 // not enough data
                } else {
                    // Map lift [-1.0, 1.0] to weight [0.5, 1.5].
                    (1.0 + record.lift()).clamp(0.5, 1.5)
                };
                (name.clone(), weight)
            })
            .collect()
    }

    /// Get the raw lift value for a section (rate_with - rate_without).
    ///
    /// Returns `None` if the section has no observations.
    #[must_use]
    pub fn lift_for(&self, section: &str) -> Option<f64> {
        self.sections.get(section).map(SectionRecord::lift)
    }

    /// Number of tracked sections.
    #[must_use]
    pub fn section_count(&self) -> usize {
        self.sections.len()
    }
}

// ---------------------------------------------------------------------------
// Calibration from efficiency history
// ---------------------------------------------------------------------------

/// Minimal shape extracted from a single `efficiency.jsonl` row for calibration.
///
/// Only fields needed to compute the feature key and token/success observation
/// are extracted; all other fields are ignored. The `schema` field is used
/// to skip non-efficiency rows written by `FeedbackService`.
#[derive(serde::Deserialize)]
struct EfficiencyCalibrationRow {
    #[serde(default)]
    schema: String,
    #[serde(default)]
    role: String,
    #[serde(default)]
    input_tokens: u64,
    #[serde(default)]
    output_tokens: u64,
    #[serde(default)]
    gate_passed: Option<bool>,
}

/// Calibrate a fresh [`BudgetPredictor`] from historical efficiency data.
///
/// Reads every row from `efficiency_jsonl_path`, groups token totals by
/// `(role, "standard", "code")` feature key (the same key used by the
/// per-turn budget check), and sets the predictor EMA to the **median**
/// token total for each key.  Using the median rather than the mean makes
/// the bootstrap robust to outlier tasks with unusually large context windows.
///
/// Returns a predictor with zero observations when the file does not exist
/// or contains no parseable efficiency rows.
///
/// # Errors
///
/// Returns an error only if the file exists and cannot be opened (permission
/// errors, etc.).  Missing files and unparseable lines are silently ignored.
pub fn calibrate_from_efficiency(
    efficiency_jsonl_path: &std::path::Path,
) -> std::io::Result<BudgetPredictor> {
    let contents = match std::fs::read_to_string(efficiency_jsonl_path) {
        Ok(c) => c,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(BudgetPredictor::new()),
        Err(e) => return Err(e),
    };

    // Accumulate per-key token totals.
    let mut by_key: HashMap<String, Vec<u64>> = HashMap::new();
    // Track gate_passed per entry for the EMA success seeding.
    let mut success_by_key: HashMap<String, Vec<bool>> = HashMap::new();

    for line in contents.lines() {
        let Ok(row) = serde_json::from_str::<EfficiencyCalibrationRow>(line) else {
            continue;
        };
        // Only process agent efficiency events; skip feedback_event/v1 rows.
        if !row.schema.is_empty() && row.schema != AGENT_EFFICIENCY_EVENT_SCHEMA {
            continue;
        }
        let role = row.role.trim();
        if role.is_empty() {
            continue;
        }
        let total_tokens = row.input_tokens + row.output_tokens;
        if total_tokens == 0 {
            continue;
        }
        // Map to a normalised role label (Implementer, Reviewer, …).
        let normalised_role = normalise_role(role);
        let features = TaskFeatures::new(normalised_role, "standard", "code");
        let key = features.key();
        by_key.entry(key.clone()).or_default().push(total_tokens);
        // gate_passed = None means the event predates the field; treat as
        // neither success nor failure — skip the success accumulation so we
        // don't bias toward unknown outcomes.
        if let Some(passed) = row.gate_passed {
            success_by_key.entry(key).or_default().push(passed);
        }
    }

    let inflation = BudgetPredictor::new().failure_inflation;
    let mut predictor = BudgetPredictor::new();
    for (key, mut tokens) in by_key {
        tokens.sort_unstable();
        let median_tokens = median_u64(&tokens);
        // Seed the EMA with the median value.
        let success_rate = success_by_key
            .get(&key)
            .map(|v| v.iter().filter(|&&s| s).count() as f64 / v.len() as f64)
            .unwrap_or(0.8); // optimistic prior when no gate data
        let count = tokens.len() as u32;
        // If the majority of tasks failed, inflate the budget so the predictor
        // proactively allocates more headroom.
        #[allow(clippy::cast_precision_loss)]
        let ema_tokens = if success_rate < 0.5 {
            median_tokens as f64 * inflation
        } else {
            median_tokens as f64
        };
        predictor.seed_observation(key, ema_tokens, success_rate, count);
    }
    Ok(predictor)
}

/// Return the median of a **sorted** slice of `u64`.
fn median_u64(sorted: &[u64]) -> u64 {
    if sorted.is_empty() {
        return 0;
    }
    let mid = sorted.len() / 2;
    if sorted.len().is_multiple_of(2) {
        // even: average of the two middle values
        sorted[mid - 1] / 2 + sorted[mid] / 2
    } else {
        sorted[mid]
    }
}

/// Normalise a raw role string to a canonical label suitable for the feature key.
fn normalise_role(role: &str) -> &str {
    let r = role.trim();
    // Map common lower-case variants to title case labels used by the runner.
    if r.eq_ignore_ascii_case("implementer") {
        "Implementer"
    } else if r.eq_ignore_ascii_case("reviewer") {
        "Reviewer"
    } else if r.eq_ignore_ascii_case("researcher") || r.eq_ignore_ascii_case("research") {
        "Researcher"
    } else if r.eq_ignore_ascii_case("strategist") || r.eq_ignore_ascii_case("strategy") {
        "Strategist"
    } else if r.eq_ignore_ascii_case("scribe") {
        "Scribe"
    } else if r.eq_ignore_ascii_case("verifier") || r.eq_ignore_ascii_case("reviewer") {
        "Verifier"
    } else {
        r
    }
}

/// Agent-efficiency-event schema discriminator (mirrors the roko-learn value).
const AGENT_EFFICIENCY_EVENT_SCHEMA: &str = "agent_efficiency_event/v1";

/// Load a [`BudgetPredictor`] from the persisted JSON file, or calibrate a
/// fresh one from the efficiency JSONL when the JSON file does not yet exist.
///
/// This ensures that callers get historically-calibrated predictions on a
/// fresh workspace without requiring a full plan run to accumulate data in
/// `budget-predictor.json` first.
///
/// Resolution order:
///
/// 1. `{learn_dir}/budget-predictor.json` — fastest path; exact EMA state.
/// 2. Calibrate from `{learn_dir}/efficiency.jsonl` — slower; median-based.
/// 3. Return [`BudgetPredictor::new()`] — empty predictor with fallback budget.
///
/// # Errors
///
/// Returns an error if the JSON file exists but cannot be parsed, or if the
/// efficiency file exists but cannot be opened (permission errors, etc.).
pub fn load_or_calibrate(learn_dir: &std::path::Path) -> std::io::Result<BudgetPredictor> {
    // Fast path: pre-trained JSON file exists.
    match load_predictor(learn_dir) {
        Ok(Some(p)) => return Ok(p),
        Ok(None) => {}
        Err(e) => return Err(e),
    }
    // Slow path: calibrate from efficiency history.
    let efficiency_path = learn_dir.join("efficiency.jsonl");
    calibrate_from_efficiency(&efficiency_path)
}

// ---------------------------------------------------------------------------
// Persistence helpers
// ---------------------------------------------------------------------------

/// Default persistence path for the budget predictor.
pub const BUDGET_PREDICTOR_FILENAME: &str = "budget-predictor.json";

/// Default persistence path for section influence data.
pub const SECTION_INFLUENCE_FILENAME: &str = "section-influence.json";

/// Persist the budget predictor to a JSON file under `learn_dir`.
///
/// # Errors
///
/// Returns an error if serialization or I/O fails.
pub fn persist_predictor(
    predictor: &BudgetPredictor,
    learn_dir: &std::path::Path,
) -> std::io::Result<()> {
    std::fs::create_dir_all(learn_dir)?;
    let path = learn_dir.join(BUDGET_PREDICTOR_FILENAME);
    let json = serde_json::to_string_pretty(predictor)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    std::fs::write(path, json)
}

/// Load the budget predictor from a JSON file under `learn_dir`.
///
/// Returns `Ok(None)` if the file does not exist.
///
/// # Errors
///
/// Returns an error if the file exists but cannot be parsed.
pub fn load_predictor(learn_dir: &std::path::Path) -> std::io::Result<Option<BudgetPredictor>> {
    let path = learn_dir.join(BUDGET_PREDICTOR_FILENAME);
    if !path.exists() {
        return Ok(None);
    }
    let data = std::fs::read_to_string(&path)?;
    let predictor: BudgetPredictor = serde_json::from_str(&data)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    Ok(Some(predictor))
}

/// Persist the section influence data to a JSON file under `learn_dir`.
///
/// # Errors
///
/// Returns an error if serialization or I/O fails.
pub fn persist_influence(
    influence: &SectionInfluence,
    learn_dir: &std::path::Path,
) -> std::io::Result<()> {
    std::fs::create_dir_all(learn_dir)?;
    let path = learn_dir.join(SECTION_INFLUENCE_FILENAME);
    let json = serde_json::to_string_pretty(influence)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    std::fs::write(path, json)
}

/// Load the section influence data from a JSON file under `learn_dir`.
///
/// Returns `Ok(None)` if the file does not exist.
///
/// # Errors
///
/// Returns an error if the file exists but cannot be parsed.
pub fn load_influence(learn_dir: &std::path::Path) -> std::io::Result<Option<SectionInfluence>> {
    let path = learn_dir.join(SECTION_INFLUENCE_FILENAME);
    if !path.exists() {
        return Ok(None);
    }
    let data = std::fs::read_to_string(&path)?;
    let influence: SectionInfluence = serde_json::from_str(&data)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    Ok(Some(influence))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    // ── BudgetPredictor ──

    #[test]
    fn predict_returns_fallback_when_empty() {
        let predictor = BudgetPredictor::new();
        let features = TaskFeatures::new("Implementer", "standard", "code");
        assert_eq!(predictor.predict(&features), predictor.fallback_tokens);
    }

    #[test]
    fn predict_returns_ema_after_observations() {
        let mut predictor = BudgetPredictor::new();
        let features = TaskFeatures::new("Implementer", "standard", "code");

        predictor.record(&features, 80_000, true);
        predictor.record(&features, 120_000, true);

        let predicted = predictor.predict(&features);
        // Should be close to the EMA * 1.2 (safety margin).
        assert!(predicted > 80_000);
        assert!(predicted < 200_000);
    }

    #[test]
    fn failure_inflates_budget() {
        let mut predictor = BudgetPredictor::new();
        let features = TaskFeatures::new("Implementer", "standard", "code");

        predictor.record(&features, 100_000, true);
        let predicted_after_success = predictor.predict(&features);

        predictor.record(&features, 100_000, false);
        let predicted_after_failure = predictor.predict(&features);

        // After a failure, the budget should be inflated.
        assert!(predicted_after_failure > predicted_after_success);
    }

    #[test]
    fn partial_match_uses_same_role_complexity() {
        let mut predictor = BudgetPredictor::new();
        let code_features = TaskFeatures::new("Implementer", "standard", "code");
        predictor.record(&code_features, 90_000, true);

        let docs_features = TaskFeatures::new("Implementer", "standard", "docs");
        let predicted = predictor.predict(&docs_features);
        // Should use the partial match from code domain.
        assert!(predicted > 50_000);
        assert!(predicted < 200_000);
    }

    #[test]
    fn observation_count_tracks_keys() {
        let mut predictor = BudgetPredictor::new();
        assert_eq!(predictor.observation_count(), 0);

        predictor.record(&TaskFeatures::new("A", "s", "c"), 100_000, true);
        assert_eq!(predictor.observation_count(), 1);

        predictor.record(&TaskFeatures::new("B", "s", "c"), 100_000, true);
        assert_eq!(predictor.observation_count(), 2);

        // Same key, no new observation count.
        predictor.record(&TaskFeatures::new("A", "s", "c"), 100_000, true);
        assert_eq!(predictor.observation_count(), 2);
    }

    #[test]
    fn predictor_serializes_and_deserializes() {
        let mut predictor = BudgetPredictor::new();
        predictor.record(&TaskFeatures::new("R", "s", "d"), 80_000, true);

        let json = serde_json::to_string(&predictor).unwrap();
        let restored: BudgetPredictor = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.observation_count(), 1);
    }

    // ── SectionInfluence ──

    #[test]
    fn no_observations_returns_neutral_weights() {
        let influence = SectionInfluence::new();
        let weights = influence.weights();
        assert!(weights.is_empty());
    }

    #[test]
    fn section_with_positive_lift_gets_higher_weight() {
        let mut influence = SectionInfluence {
            min_observations: 2,
            ..SectionInfluence::default()
        };

        let all = vec!["prd2".into(), "context".into()];

        // Tasks with "prd2" succeed; tasks without it fail.
        for _ in 0..5 {
            influence.record(&["prd2".into(), "context".into()], &all, true);
            influence.record(&["context".into()], &all, false);
        }

        let weights = influence.weights();
        // "prd2" has positive lift (always present when success).
        assert!(weights["prd2"] > 1.0);
    }

    #[test]
    fn section_with_negative_lift_gets_lower_weight() {
        let mut influence = SectionInfluence {
            min_observations: 2,
            ..SectionInfluence::default()
        };

        let all = vec!["noise".into(), "core".into()];

        // Tasks with "noise" fail; tasks without it succeed.
        for _ in 0..5 {
            influence.record(&["noise".into(), "core".into()], &all, false);
            influence.record(&["core".into()], &all, true);
        }

        let weights = influence.weights();
        assert!(weights["noise"] < 1.0);
    }

    #[test]
    fn insufficient_observations_return_neutral() {
        let mut influence = SectionInfluence::new(); // min_observations = 10
        let all = vec!["sec".into()];

        // Only 4 observations, below threshold.
        for _ in 0..4 {
            influence.record(&["sec".into()], &all, true);
        }

        let weights = influence.weights();
        assert!((weights["sec"] - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn lift_for_returns_correct_value() {
        let mut influence = SectionInfluence {
            min_observations: 1,
            ..SectionInfluence::default()
        };
        let all = vec!["a".into()];

        // 3 successes with, 1 failure with, 0 without.
        influence.record(&["a".into()], &all, true);
        influence.record(&["a".into()], &all, true);
        influence.record(&["a".into()], &all, true);
        influence.record(&["a".into()], &all, false);

        let lift = influence.lift_for("a").unwrap();
        // rate_with = 3/4 = 0.75, rate_without = 0.5 (neutral prior), lift = 0.25
        assert!((lift - 0.25).abs() < 0.01);
    }

    #[test]
    fn influence_serializes_and_deserializes() {
        let mut influence = SectionInfluence::new();
        let all = vec!["sec".into()];
        influence.record(&["sec".into()], &all, true);

        let json = serde_json::to_string(&influence).unwrap();
        let restored: SectionInfluence = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.section_count(), 1);
    }

    // ── Persistence ──

    #[test]
    fn predictor_persist_and_load_roundtrips() {
        let dir = std::env::temp_dir().join("roko-test-bp");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        let mut predictor = BudgetPredictor::new();
        predictor.record(&TaskFeatures::new("R", "s", "d"), 80_000, true);
        persist_predictor(&predictor, &dir).unwrap();

        let loaded = load_predictor(&dir).unwrap().unwrap();
        assert_eq!(loaded.observation_count(), 1);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn influence_persist_and_load_roundtrips() {
        let dir = std::env::temp_dir().join("roko-test-si");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        let mut influence = SectionInfluence::new();
        influence.record(&["s".into()], &["s".into()], true);
        persist_influence(&influence, &dir).unwrap();

        let loaded = load_influence(&dir).unwrap().unwrap();
        assert_eq!(loaded.section_count(), 1);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn load_missing_files_returns_none() {
        let dir = std::env::temp_dir().join("roko-test-missing");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        assert!(load_predictor(&dir).unwrap().is_none());
        assert!(load_influence(&dir).unwrap().is_none());

        let _ = std::fs::remove_dir_all(&dir);
    }

    // ── calibrate_from_efficiency ──

    fn make_efficiency_row(
        role: &str,
        input: u64,
        output: u64,
        gate_passed: Option<bool>,
    ) -> String {
        let gp = match gate_passed {
            Some(true) => "true",
            Some(false) => "false",
            None => "null",
        };
        format!(
            r#"{{"schema":"agent_efficiency_event/v1","role":"{role}","input_tokens":{input},"output_tokens":{output},"gate_passed":{gp}}}"#
        )
    }

    #[test]
    fn calibrate_from_missing_efficiency_returns_empty_predictor() {
        let dir = std::env::temp_dir().join("roko-test-cal-empty");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        let predictor = calibrate_from_efficiency(&dir.join("efficiency.jsonl")).unwrap();
        assert_eq!(predictor.observation_count(), 0);
        // Should still fall back to the configured fallback budget.
        assert_eq!(
            predictor.predict(&TaskFeatures::new("Implementer", "standard", "code")),
            predictor.fallback_tokens,
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn calibrate_from_efficiency_seeds_median_token_budget() {
        let dir = std::env::temp_dir().join("roko-test-cal-seed");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        // Three Implementer success rows with different token totals.
        // Median of (50_000, 80_000, 120_000) = 80_000 total tokens.
        let rows = vec![
            make_efficiency_row("Implementer", 30_000, 20_000, Some(true)), // 50_000
            make_efficiency_row("Implementer", 50_000, 30_000, Some(true)), // 80_000
            make_efficiency_row("Implementer", 70_000, 50_000, Some(true)), // 120_000
        ];
        let path = dir.join("efficiency.jsonl");
        std::fs::write(&path, rows.join("\n") + "\n").unwrap();

        let predictor = calibrate_from_efficiency(&path).unwrap();
        // Expect exactly one feature key for the normalised role.
        assert_eq!(predictor.observation_count(), 1);
        let features = TaskFeatures::new("Implementer", "standard", "code");
        let predicted = predictor.predict(&features);
        // Median 80_000 tokens, with the 20% safety margin: 96_000.
        assert!(
            predicted >= 80_000 && predicted <= 150_000,
            "expected predicted tokens near 96_000, got {predicted}"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn calibrate_from_efficiency_inflates_on_majority_failures() {
        let dir = std::env::temp_dir().join("roko-test-cal-fail");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        let rows = vec![
            make_efficiency_row("Implementer", 40_000, 10_000, Some(false)),
            make_efficiency_row("Implementer", 40_000, 10_000, Some(false)),
            make_efficiency_row("Implementer", 40_000, 10_000, Some(true)),
        ];
        let path = dir.join("efficiency.jsonl");
        std::fs::write(&path, rows.join("\n") + "\n").unwrap();

        let predictor = calibrate_from_efficiency(&path).unwrap();
        let features = TaskFeatures::new("Implementer", "standard", "code");
        // Majority failures → budget should be inflated above the median.
        let predicted_fail = predictor.predict(&features);
        // 50_000 * 1.3 inflation * 1.2 safety = 78_000 (approx)
        assert!(predicted_fail > 50_000);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn calibrate_from_efficiency_skips_feedback_event_rows() {
        let dir = std::env::temp_dir().join("roko-test-cal-skip");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        // Mix of an efficiency event and a feedback_event/v1 row that should be ignored.
        let rows = vec![
            make_efficiency_row("Implementer", 50_000, 20_000, Some(true)),
            r#"{"schema":"feedback_event/v1","role":"Implementer","input_tokens":999999,"output_tokens":999999}"#.to_string(),
        ];
        let path = dir.join("efficiency.jsonl");
        std::fs::write(&path, rows.join("\n") + "\n").unwrap();

        let predictor = calibrate_from_efficiency(&path).unwrap();
        // Only 1 observation — the feedback_event row was skipped.
        assert_eq!(predictor.observation_count(), 1);
        let features = TaskFeatures::new("Implementer", "standard", "code");
        let predicted = predictor.predict(&features);
        // Should not include the 999_999 token outlier from the feedback row.
        assert!(
            predicted < 200_000,
            "feedback row should have been ignored; got {predicted}"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn load_or_calibrate_prefers_json_file_over_efficiency_jsonl() {
        let dir = std::env::temp_dir().join("roko-test-cal-prefer");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        // Write a pre-trained predictor to budget-predictor.json.
        let mut known = BudgetPredictor::new();
        known.record(
            &TaskFeatures::new("Implementer", "standard", "code"),
            42_000,
            true,
        );
        persist_predictor(&known, &dir).unwrap();

        // Also write an efficiency file with very different data.
        let rows = vec![make_efficiency_row(
            "Implementer",
            500_000,
            200_000,
            Some(true),
        )];
        let eff_path = dir.join("efficiency.jsonl");
        std::fs::write(&eff_path, rows.join("\n") + "\n").unwrap();

        // load_or_calibrate should use the JSON file, not calibrate from efficiency.
        let loaded = load_or_calibrate(&dir).unwrap();
        let features = TaskFeatures::new("Implementer", "standard", "code");
        let predicted = loaded.predict(&features);
        // From the JSON file: 42_000 * 1.2 = 50_400. Not the 840_000 from efficiency.
        assert!(
            predicted < 200_000,
            "expected JSON file to win; predicted {predicted}"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn load_or_calibrate_falls_back_to_efficiency_when_no_json() {
        let dir = std::env::temp_dir().join("roko-test-cal-fallback");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        // No budget-predictor.json; only an efficiency file.
        let rows = vec![
            make_efficiency_row("Implementer", 60_000, 40_000, Some(true)),
            make_efficiency_row("Implementer", 70_000, 30_000, Some(true)),
        ];
        let eff_path = dir.join("efficiency.jsonl");
        std::fs::write(&eff_path, rows.join("\n") + "\n").unwrap();

        let predictor = load_or_calibrate(&dir).unwrap();
        assert!(
            predictor.observation_count() > 0,
            "should have calibrated from efficiency.jsonl"
        );
        // Median of (100_000, 100_000) = 100_000; predicted = 100_000 * 1.2 = 120_000.
        let features = TaskFeatures::new("Implementer", "standard", "code");
        let predicted = predictor.predict(&features);
        assert!(
            predicted > 50_000,
            "calibrated predictor should give a non-fallback budget; got {predicted}"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }
}
