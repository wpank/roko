//! Spec-quality gate configuration section (`[spec_quality]`, S07 §5).

use serde::{Deserialize, Serialize};

// ---- [spec_quality] ------------------------------------------------------

/// What the spec-quality gate does with a plan before `plan run` dispatches
/// any of its tasks (S07 §4.3, decision 3201).
///
/// In every mode but `off`, a hard fail refuses the plan: a verify step that
/// can never fail (HF2), or one that already passes on the unchanged base
/// (HF3) when the red-on-base check ran. Scores only advise until the rubric
/// is recalibrated. There is no per-run override, so every refusal follows
/// from `roko.toml`.
///
/// ```toml
/// [spec_quality]
/// mode = "advise"                # off | advise | enforce
/// allow_threshold = 70.0
/// block_threshold = 40.0
/// red_on_base = false
/// red_on_base_timeout_secs = 120
/// holdout_frac = 0.05
/// ```
///
/// Every field serializes, since the config loader drops any key that the
/// serialized default config lacks.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpecQualityConfig {
    /// What the gate acts on; `advise` by default.
    #[serde(default)]
    pub mode: SpecQualityMode,
    /// The score (0–100) at or above which a task goes to its agent as
    /// written.
    #[serde(default = "default_allow_threshold")]
    pub allow_threshold: f64,
    /// The score (0–100) below which `enforce` stops a task until its spec
    /// is refined. At most `allow_threshold`.
    #[serde(default = "default_block_threshold")]
    pub block_threshold: f64,
    /// Whether to run each implementer task's verify steps on the unchanged
    /// base before dispatch, to score SQ06 and find HF3. Off by default: in a
    /// fresh worktree most cargo steps hit the timeout and come out unknown.
    #[serde(default)]
    pub red_on_base: bool,
    /// The longest one verify step may run during the red-on-base check.
    #[serde(default = "default_red_on_base_timeout_secs")]
    pub red_on_base_timeout_secs: u64,
    /// The share (0–1) of score-based gate decisions skipped at random, so
    /// that the gate's effect can be measured (S03). A hard fail is never
    /// held out.
    #[serde(default = "default_holdout_frac")]
    pub holdout_frac: f64,
}

/// What the spec-quality gate acts on.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SpecQualityMode {
    /// No scoring, and nothing refused.
    Off,
    /// Hard fails refuse the plan; scores are reported and recorded only.
    #[default]
    Advise,
    /// Hard fails refuse the plan, and scores below `block_threshold` stop
    /// their tasks (benchmark runs).
    Enforce,
}

const fn default_allow_threshold() -> f64 {
    70.0
}

const fn default_block_threshold() -> f64 {
    40.0
}

const fn default_red_on_base_timeout_secs() -> u64 {
    120
}

const fn default_holdout_frac() -> f64 {
    0.05
}

impl Default for SpecQualityConfig {
    fn default() -> Self {
        Self {
            mode: SpecQualityMode::default(),
            allow_threshold: default_allow_threshold(),
            block_threshold: default_block_threshold(),
            red_on_base: false,
            red_on_base_timeout_secs: default_red_on_base_timeout_secs(),
            holdout_frac: default_holdout_frac(),
        }
    }
}

impl SpecQualityConfig {
    /// Whether the gate scores plans at all: every mode but `off`.
    #[must_use]
    pub fn is_on(&self) -> bool {
        self.mode != SpecQualityMode::Off
    }

    /// What is wrong with these settings, as (config path, message) pairs:
    /// a threshold outside 0–100, `block_threshold` above `allow_threshold`,
    /// or `holdout_frac` outside 0–1. `roko config validate` and the config
    /// loader reject each one.
    #[must_use]
    pub fn problems(&self) -> Vec<(&'static str, String)> {
        let mut problems = Vec::new();
        let thresholds = [
            ("spec_quality.allow_threshold", self.allow_threshold),
            ("spec_quality.block_threshold", self.block_threshold),
        ];
        for (path, value) in thresholds {
            if !(0.0..=100.0).contains(&value) {
                problems.push((
                    path,
                    format!("{path} ({value}) must be a score from 0 to 100"),
                ));
            }
        }
        if self.block_threshold > self.allow_threshold {
            problems.push((
                "spec_quality.block_threshold",
                format!(
                    "spec_quality.block_threshold ({}) must not exceed \
                     spec_quality.allow_threshold ({})",
                    self.block_threshold, self.allow_threshold
                ),
            ));
        }
        if !(0.0..=1.0).contains(&self.holdout_frac) {
            problems.push((
                "spec_quality.holdout_frac",
                format!(
                    "spec_quality.holdout_frac ({}) must be a share from 0 to 1",
                    self.holdout_frac
                ),
            ));
        }
        problems
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::schema::RokoConfig;
    use crate::config::validation::{InvariantSeverity, validate_invariants};

    /// 3210: the defaults serialize, every key included, and parse back
    /// equal; `[spec_quality]` reads from `roko.toml`; a threshold out of
    /// range, or a block threshold above the allow threshold, fails
    /// validation.
    #[test]
    fn spec_quality_config_defaults_round_trip() {
        let defaults = SpecQualityConfig::default();
        assert_eq!(defaults.mode, SpecQualityMode::Advise);
        assert!(defaults.is_on());
        assert!(defaults.problems().is_empty());

        let text = toml::to_string(&defaults).expect("serialize the defaults");
        for key in [
            "mode",
            "allow_threshold",
            "block_threshold",
            "red_on_base",
            "red_on_base_timeout_secs",
            "holdout_frac",
        ] {
            assert!(text.contains(key), "{key} missing from {text}");
        }
        let back: SpecQualityConfig = toml::from_str(&text).expect("parse the defaults");
        assert_eq!(back, defaults);

        let config = RokoConfig::default();
        assert_eq!(config.spec_quality, defaults);
        let config_text = toml::to_string(&config).expect("serialize the default config");
        assert!(config_text.contains("[spec_quality]"), "{config_text}");
        let reread = RokoConfig::from_toml(&config_text).expect("parse the default config");
        assert_eq!(reread.spec_quality, defaults);

        let set = RokoConfig::from_toml("[spec_quality]\nmode = \"off\"\nallow_threshold = 75\n")
            .expect("parse a [spec_quality] section");
        assert_eq!(set.spec_quality.mode, SpecQualityMode::Off);
        assert!(!set.spec_quality.is_on());
        assert!((set.spec_quality.allow_threshold - 75.0).abs() < f64::EPSILON);
        assert!(RokoConfig::from_toml("[spec_quality]\nmod = \"off\"\n").is_err());

        let mut invalid = RokoConfig::default();
        invalid.spec_quality.allow_threshold = 120.0;
        let errors: Vec<String> = validate_invariants(&invalid)
            .into_iter()
            .filter(|result| result.severity == InvariantSeverity::Error)
            .map(|result| result.config_path)
            .collect();
        assert_eq!(errors, ["spec_quality.allow_threshold"]);

        invalid.spec_quality.allow_threshold = 30.0;
        let problems = invalid.spec_quality.problems();
        assert_eq!(problems.len(), 1, "{problems:?}");
        assert_eq!(problems[0].0, "spec_quality.block_threshold");
    }
}
