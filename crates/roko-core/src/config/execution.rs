//! Execution-time configuration sections: dreams, daimon, and repos.
//!
//! These are schema-only definitions that live in `roko-core` so that the
//! unified config loader can parse, validate, and track provenance for
//! fields that were previously CLI-only.
//!
//! `roko-core` must NOT depend on `roko-dreams` or `roko-daimon`.  Conversion
//! to runtime types (`DreamSchedulePolicy`, `StrategySpaceDefinition`) remains
//! in CLI adapter code until the runtime construction moves to layer 3.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::subscriptions::SubscriptionConfig;

// ---- [dreams] ---------------------------------------------------------------

/// Automatic dream-cycle settings for daemon mode.
///
/// Mirrors the fields previously defined in `roko-cli`'s `DreamsConfig`.
/// The CLI adapter converts this to `roko_dreams::DreamSchedulePolicy`
/// at runtime construction time.
///
/// Named `DreamScheduleConfig` to avoid collision with
/// `learning::DreamsConfig` (which covers `[learning.dreams]`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DreamScheduleConfig {
    /// Enable the automatic dream cycle.
    #[serde(default = "DreamScheduleConfig::default_auto_dream")]
    pub auto_dream: bool,
    /// Idle duration threshold, in minutes, before a dream can run.
    #[serde(default = "DreamScheduleConfig::default_idle_threshold_mins")]
    pub idle_threshold_mins: u64,
    /// Minimum number of new episodes required before dreaming.
    #[serde(default = "DreamScheduleConfig::default_min_episodes_for_dream")]
    pub min_episodes_for_dream: usize,
    /// Optional seven-field cron expression used as a fallback cadence.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scheduled_cron: Option<String>,
    /// Trigger as soon as this many unconsolidated episodes exist. Zero
    /// disables the episode-count trigger; the idle/cron minimum still uses
    /// `min_episodes_for_dream`.
    #[serde(default)]
    pub episode_count_trigger: usize,
    /// Idle-delay multiplier after a high-quality dream cycle.
    #[serde(default = "DreamScheduleConfig::default_quality_gain")]
    pub quality_gain: f64,
    /// Idle-delay multiplier after a low-quality cycle.
    #[serde(default = "DreamScheduleConfig::default_quality_penalty")]
    pub quality_penalty: f64,
}

impl DreamScheduleConfig {
    const fn default_auto_dream() -> bool {
        true
    }

    const fn default_idle_threshold_mins() -> u64 {
        15
    }

    const fn default_min_episodes_for_dream() -> usize {
        5
    }

    const fn default_quality_gain() -> f64 {
        0.75
    }

    const fn default_quality_penalty() -> f64 {
        1.25
    }

    /// Validate the scheduling fields.
    ///
    /// # Errors
    ///
    /// Returns an error for non-positive/non-finite quality multipliers or
    /// an empty cron expression. This is a schema-level check that does not
    /// require the `roko-dreams` crate.
    pub fn validate(&self) -> Result<(), String> {
        if !self.quality_gain.is_finite() || self.quality_gain <= 0.0 {
            return Err("dreams.quality_gain must be finite and greater than zero".to_string());
        }
        if !self.quality_penalty.is_finite() || self.quality_penalty <= 0.0 {
            return Err("dreams.quality_penalty must be finite and greater than zero".to_string());
        }
        if let Some(ref cron) = self.scheduled_cron
            && cron.trim().is_empty()
        {
            return Err("dreams.scheduled_cron must not be empty".to_string());
        }
        Ok(())
    }
}

impl Default for DreamScheduleConfig {
    fn default() -> Self {
        Self {
            auto_dream: Self::default_auto_dream(),
            idle_threshold_mins: Self::default_idle_threshold_mins(),
            min_episodes_for_dream: Self::default_min_episodes_for_dream(),
            scheduled_cron: None,
            episode_count_trigger: 0,
            quality_gain: Self::default_quality_gain(),
            quality_penalty: Self::default_quality_penalty(),
        }
    }
}

// ---- [daimon] ---------------------------------------------------------------

/// Daimon affect-engine configuration.
///
/// Mirrors the fields previously defined in `roko-cli`'s `DaimonConfig`.
/// The CLI adapter converts `StrategySpaceConfig` to
/// `roko_daimon::StrategySpaceDefinition` at runtime construction time.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DaimonConfig {
    /// Domain-specific strategy-space registration for somatic markers.
    #[serde(default)]
    pub strategy_space: StrategySpaceConfig,
}

impl Default for DaimonConfig {
    fn default() -> Self {
        Self {
            strategy_space: StrategySpaceConfig::default(),
        }
    }
}

/// Strategy-space definition (schema-only mirror of `StrategySpaceDefinition`).
///
/// The fixed 8-dimension array is represented as a `Vec` here because
/// `roko-core` cannot depend on `roko-daimon` (which owns the
/// `STRATEGY_DIMENSIONS` constant). The CLI adapter validates the length
/// when converting to `StrategySpaceDefinition`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StrategySpaceConfig {
    /// Domain identifier for this strategy-space mapping.
    #[serde(default = "StrategySpaceConfig::default_domain")]
    pub domain: String,
    /// Human-readable labels for the fixed 8 dimensions.
    #[serde(default = "StrategySpaceConfig::default_dimensions")]
    pub dimensions: Vec<String>,
}

impl StrategySpaceConfig {
    fn default_domain() -> String {
        "coding".to_string()
    }

    fn default_dimensions() -> Vec<String> {
        vec![
            "complexity".to_string(),
            "risk".to_string(),
            "novelty".to_string(),
            "confidence".to_string(),
            "time_pressure".to_string(),
            "scope".to_string(),
            "reversibility".to_string(),
            "dependency_depth".to_string(),
        ]
    }
}

impl Default for StrategySpaceConfig {
    fn default() -> Self {
        Self {
            domain: Self::default_domain(),
            dimensions: Self::default_dimensions(),
        }
    }
}

// ---- [[repos]] --------------------------------------------------------------

/// Per-repository declarative configuration inside `roko.toml`.
///
/// Repo-specific subscriptions are additive: they sit alongside the global
/// subscription set and can narrow behavior for one checkout.
///
/// This is the schema-only definition. Runtime behavior (`RepoEntry`,
/// `RepoRegistry`, filesystem canonicalization) remains in CLI code.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepoConfig {
    /// Human-readable repo name.
    pub name: String,
    /// Filesystem path to the repo root.
    pub path: PathBuf,
    /// Branch name tracked for this repo.
    pub branch: String,
    /// Template names active for this repo.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub templates: Vec<String>,
    /// Repo-specific subscriptions to load in addition to the global set.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub subscriptions: Vec<SubscriptionConfig>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dreams_default_validates() {
        DreamScheduleConfig::default()
            .validate()
            .expect("default dreams config should be valid");
    }

    #[test]
    fn dreams_invalid_quality_gain() {
        let mut dreams = DreamScheduleConfig::default();
        dreams.quality_gain = -1.0;
        assert!(dreams.validate().is_err());
    }

    #[test]
    fn dreams_invalid_quality_penalty() {
        let mut dreams = DreamScheduleConfig::default();
        dreams.quality_penalty = f64::NAN;
        assert!(dreams.validate().is_err());
    }

    #[test]
    fn dreams_empty_cron_rejected() {
        let mut dreams = DreamScheduleConfig::default();
        dreams.scheduled_cron = Some("  ".to_string());
        assert!(dreams.validate().is_err());
    }

    #[test]
    fn strategy_space_default_has_eight_dims() {
        let ss = StrategySpaceConfig::default();
        assert_eq!(ss.dimensions.len(), 8);
        assert_eq!(ss.domain, "coding");
    }

    #[test]
    fn daimon_default_roundtrip() {
        let original = DaimonConfig::default();
        let toml_text = toml::to_string_pretty(&original).expect("serialize");
        let deserialized: DaimonConfig = toml::from_str(&toml_text).expect("deserialize");
        assert_eq!(original, deserialized);
    }

    #[test]
    fn dreams_roundtrip() {
        let original = DreamScheduleConfig::default();
        let toml_text = toml::to_string_pretty(&original).expect("serialize");
        let deserialized: DreamScheduleConfig = toml::from_str(&toml_text).expect("deserialize");
        assert_eq!(original, deserialized);
    }

    #[test]
    fn repo_config_roundtrip() {
        let original = RepoConfig {
            name: "test-repo".to_string(),
            path: "/tmp/test".into(),
            branch: "main".to_string(),
            templates: vec!["code".to_string()],
            subscriptions: Vec::new(),
        };
        let toml_text = toml::to_string_pretty(&original).expect("serialize");
        let deserialized: RepoConfig = toml::from_str(&toml_text).expect("deserialize");
        assert_eq!(original, deserialized);
    }
}
