//! The canonical sparse feature vector and its `features_hash`: backlog task 6113.
//!
//! The groups follow S04 §4.2. The task: a hashed bag of words (2^12 buckets) over the title,
//! description and verify commands, the family, tier and role, the attempt ordinal, whether an
//! earlier attempt failed and its error class. The arm: harness, model, effort and verify depth
//! one-hots, and the price tier when the snapshot lists the model. The history: the
//! (family, model) pass EMA. S07's spec features get a slot each that `spec_features.rs`
//! (S07.12) fills: a missing one is 0 with a missingness indicator.

use std::collections::BTreeMap;

use roko_core::pricing_snapshot::PriceSnapshot;
use serde::{Deserialize, Serialize};

use super::ArmKey;

/// The hashed bag of words' bucket count, 2^12.
pub const BOW_BUCKETS: u64 = 1 << 12;

/// The feature schema's version: `predictor_version` hashes it (6116), so a change to the
/// features starts a new predictor.
pub const FEATURE_SCHEMA: &str = "m3-features/1";

/// The S07 spec features (`spec_features@1`) the vector keeps a slot for.
pub const SPEC_FEATURES: [&str; 8] = [
    "spec_score",
    "has_acceptance_criteria",
    "n_acceptance_criteria",
    "n_verify_cmds",
    "has_hidden_hook",
    "ambiguity_terms",
    "spec_tokens",
    "example_io_present",
];

/// The (family, model) pass EMA's weight on the newest outcome.
pub const PASS_EMA_WEIGHT: f64 = 0.1;

/// A sparse feature vector: named features and their values, sorted by name.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct FeatureVector(BTreeMap<String, f64>);

impl FeatureVector {
    /// The vector of `pairs`, in any order; a repeated name keeps its last value.
    pub fn from_pairs<N: Into<String>>(pairs: impl IntoIterator<Item = (N, f64)>) -> Self {
        Self(
            pairs
                .into_iter()
                .map(|(name, value)| (name.into(), value))
                .collect(),
        )
    }

    /// Set feature `name` to `value`.
    pub fn set(&mut self, name: impl Into<String>, value: f64) {
        self.0.insert(name.into(), value);
    }

    /// The value of feature `name`, when set.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<f64> {
        self.0.get(name).copied()
    }

    /// The features, sorted by name.
    pub fn pairs(&self) -> impl Iterator<Item = (&str, f64)> {
        self.0.iter().map(|(name, value)| (name.as_str(), *value))
    }

    /// The number of features set.
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Whether no feature is set.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// `features_hash`: blake3 of the sorted (name, value) pairs, as hex.
    #[must_use]
    pub fn hash(&self) -> String {
        let mut hasher = blake3::Hasher::new();
        for (name, value) in &self.0 {
            hasher.update(name.as_bytes());
            hasher.update(b"=");
            hasher.update(&value.to_bits().to_le_bytes());
            hasher.update(b";");
        }
        hasher.finalize().to_hex().to_string()
    }
}

/// What the self-model knows about a task before an attempt runs.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct TaskFeatures {
    /// The task title.
    pub title: String,
    /// The task description.
    pub description: String,
    /// The task's verify commands.
    pub verify_commands: Vec<String>,
    /// The task family: the tier for plan tasks, `task.family` for benchmark rows.
    pub family: String,
    /// The difficulty tier.
    pub tier: String,
    /// The role dispatched.
    pub role: String,
    /// The 1-based attempt ordinal.
    pub attempt: u32,
    /// Whether an earlier attempt of the chain failed.
    pub has_prior_failure: bool,
    /// The previous attempt's gate error class, when it failed.
    pub error_class: Option<String>,
    /// S07's spec features by name, when `spec_features.rs` supplies them.
    pub spec: BTreeMap<String, f64>,
}

/// The per-(family, model) EMA of passes: the history group of the vector.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct PassEma(BTreeMap<String, f64>);

impl PassEma {
    /// The key of (`family`, `model`).
    fn key(family: &str, model: &str) -> String {
        format!("{family}|{model}")
    }

    /// The EMA for `model` on `family`, when it has seen an outcome.
    #[must_use]
    pub fn get(&self, family: &str, model: &str) -> Option<f64> {
        self.0.get(&Self::key(family, model)).copied()
    }

    /// Add an outcome of `model` on `family`.
    pub fn observe(&mut self, family: &str, model: &str, passed: bool) {
        let outcome = if passed { 1.0 } else { 0.0 };
        self.0
            .entry(Self::key(family, model))
            .and_modify(|ema| *ema += PASS_EMA_WEIGHT * (outcome - *ema))
            .or_insert(outcome);
    }
}

/// The model's price tier at `snapshot`: `cheap` below $1 of input per million tokens,
/// `mid` below $5, else `frontier`. `None` when the snapshot does not list the model.
#[must_use]
pub fn price_tier(snapshot: &PriceSnapshot, model: &str) -> Option<&'static str> {
    let input = snapshot.row(model)?.input;
    Some(if input < 1.0 {
        "cheap"
    } else if input < 5.0 {
        "mid"
    } else {
        "frontier"
    })
}

/// The bucket of a bag-of-words token.
fn bow_bucket(token: &str) -> u64 {
    let digest = blake3::hash(token.as_bytes());
    let mut bytes = [0_u8; 8];
    bytes.copy_from_slice(&digest.as_bytes()[..8]);
    u64::from_le_bytes(bytes) % BOW_BUCKETS
}

/// The canonical vector of `task` on `arm`, with the arm's `price_tier` when known and the
/// history's pass EMA.
#[must_use]
pub fn feature_vector(
    task: &TaskFeatures,
    arm: &ArmKey,
    price_tier: Option<&str>,
    history: &PassEma,
) -> FeatureVector {
    let mut vector = FeatureVector::default();
    vector.set("bias", 1.0);
    let text = [task.title.as_str(), task.description.as_str()]
        .into_iter()
        .chain(task.verify_commands.iter().map(String::as_str));
    for token in text
        .flat_map(|text| text.split(|c: char| !c.is_alphanumeric()))
        .filter(|token| !token.is_empty())
    {
        vector.set(format!("bow:{}", bow_bucket(&token.to_lowercase())), 1.0);
    }
    vector.set(format!("family:{}", task.family), 1.0);
    vector.set(format!("tier:{}", task.tier), 1.0);
    vector.set(format!("role:{}", task.role), 1.0);
    vector.set("attempt", f64::from(task.attempt.saturating_sub(1).min(5)));
    if task.has_prior_failure {
        vector.set("prior_failure", 1.0);
    }
    if let Some(class) = &task.error_class {
        vector.set(format!("error:{class}"), 1.0);
    }
    vector.set(format!("harness:{}", arm.harness), 1.0);
    vector.set(format!("model:{}", arm.model), 1.0);
    vector.set(format!("effort:{}", arm.effort), 1.0);
    match arm.depth {
        Some(depth) => vector.set(format!("depth:V{depth}"), 1.0),
        None => vector.set("depth:none", 1.0),
    }
    if let Some(tier) = price_tier {
        vector.set(format!("price_tier:{tier}"), 1.0);
    }
    match history.get(&task.family, &arm.model) {
        Some(ema) => vector.set("pass_ema", ema),
        None => vector.set("pass_ema_missing", 1.0),
    }
    for name in SPEC_FEATURES {
        match task.spec.get(name) {
            Some(value) => vector.set(format!("spec:{name}"), *value),
            None => vector.set(format!("spec_missing:{name}"), 1.0),
        }
    }
    vector
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn features_hash_ignores_the_order_of_pairs() {
        let forward = FeatureVector::from_pairs([("role:implementer", 1.0), ("attempt", 2.0)]);
        let backward = FeatureVector::from_pairs([("attempt", 2.0), ("role:implementer", 1.0)]);
        assert_eq!(forward.hash(), backward.hash());
        let changed = FeatureVector::from_pairs([("attempt", 3.0), ("role:implementer", 1.0)]);
        assert_ne!(forward.hash(), changed.hash());
    }

    #[test]
    fn the_vector_carries_every_group() {
        let task = TaskFeatures {
            title: "Fix the parser".to_string(),
            description: "Parse the header".to_string(),
            verify_commands: vec!["cargo test -p parser".to_string()],
            family: "focused".to_string(),
            tier: "focused".to_string(),
            role: "implementer".to_string(),
            attempt: 2,
            has_prior_failure: true,
            error_class: Some("verify:0/test".to_string()),
            spec: BTreeMap::from([("spec_score".to_string(), 0.7)]),
        };
        let arm = ArmKey::roko("cerebras", "gpt-oss-120b");
        let mut history = PassEma::default();
        history.observe("focused", "gpt-oss-120b", true);
        history.observe("focused", "gpt-oss-120b", false);
        let vector = feature_vector(&task, &arm, Some("cheap"), &history);
        assert_eq!(vector.get("attempt"), Some(1.0));
        assert_eq!(vector.get("prior_failure"), Some(1.0));
        assert_eq!(vector.get("error:verify:0/test"), Some(1.0));
        assert_eq!(vector.get("model:gpt-oss-120b"), Some(1.0));
        assert_eq!(vector.get("depth:V0"), Some(1.0));
        assert_eq!(vector.get("price_tier:cheap"), Some(1.0));
        assert_eq!(vector.get("pass_ema"), Some(0.9));
        assert_eq!(vector.get("spec:spec_score"), Some(0.7));
        assert_eq!(vector.get("spec_missing:spec_tokens"), Some(1.0));
        let words = vector
            .pairs()
            .filter(|(name, _)| name.starts_with("bow:"))
            .count();
        assert_eq!(words, 8, "eight distinct words");
        // The same task hashes the same, whatever the order its words came in.
        assert_eq!(
            vector.hash(),
            feature_vector(&task, &arm, Some("cheap"), &history).hash()
        );
    }
}
