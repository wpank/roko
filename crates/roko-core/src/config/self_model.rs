//! `[self_model]` in `roko.toml` (S04 §5): M3, the self-model that forecasts each routed
//! attempt and, in active mode, picks its start rung.
//!
//! It is off by default (decision 6102, question 5). `shadow` forecasts every routed attempt and
//! logs the rung the self-model would choose, while routing stays as it is; `active` lets it
//! choose the start rung while its calibration gate holds (6122). A person switches the mode:
//! nothing in roko writes this section. Holdout and ε are not here: they come from S03's route
//! table and `[routing] explore_epsilon`.

use serde::{Deserialize, Serialize};

/// The state file's default path, relative to the workspace.
pub const DEFAULT_STATE_PATH: &str = ".roko/learn/self-model/state-v1.json";

/// How the self-model takes part in routing (S04 §4.7).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SelfModelMode {
    /// It does not run.
    #[default]
    Off,
    /// It forecasts every routed attempt and logs the rung it would choose; routing is
    /// unchanged.
    Shadow,
    /// It chooses the start rung while its calibration gate holds.
    Active,
}

/// The policy that turns forecasts into a choice (S04 §4.3, §4.4).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SelfModelPolicy {
    /// Policy (b): start on the cheapest rung worth trying, and climb after failures.
    #[default]
    Cascade,
    /// Policy (a): the cheapest arm whose lower confidence bound on P(VS) meets the target.
    LcbAci,
    /// No choice of its own: the self-model forecasts, and the ladder's rung stands.
    Static,
}

/// `[self_model]` in `roko.toml` (S04 §5). Every key is optional.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct SelfModelConfig {
    /// `off` (the default), `shadow` or `active`.
    pub mode: SelfModelMode,
    /// `cascade` (the default), `lcb_aci` or `static`.
    pub policy: SelfModelPolicy,
    /// π*, the verified-success rate the policy aims for (D11), in (0, 1).
    pub target: f64,
    /// δ, the error rate of the lower confidence bound on P(VS), in (0, 1).
    pub delta: f64,
    /// Outcomes the calibration window needs before active mode may act.
    pub n_min: u32,
    /// The state file, relative to the workspace.
    pub state_path: String,
    /// Decision 6101 B1: a task may start below its tier's rung.
    pub allow_downward_start: bool,
    /// Decision 6101 B2: a climb may skip a rung. No by default: one rung per climb.
    pub allow_rung_skip: bool,
}

impl Default for SelfModelConfig {
    fn default() -> Self {
        Self {
            mode: SelfModelMode::Off,
            policy: SelfModelPolicy::Cascade,
            target: 0.80,
            delta: 0.20,
            n_min: 100,
            state_path: DEFAULT_STATE_PATH.to_string(),
            allow_downward_start: true,
            allow_rung_skip: false,
        }
    }
}

impl SelfModelConfig {
    /// Whether the self-model runs at all.
    #[must_use]
    pub fn enabled(&self) -> bool {
        self.mode != SelfModelMode::Off
    }

    /// Every problem with the section, as (key, problem). Whether active mode may act is not
    /// one: the calibration gate decides that at run time.
    #[must_use]
    pub fn problems(&self) -> Vec<(&'static str, String)> {
        let mut problems = Vec::new();
        for (key, value) in [("target", self.target), ("delta", self.delta)] {
            let inside = value > 0.0 && value < 1.0;
            if !inside {
                problems.push((
                    key,
                    format!("{key} ({value}) must lie strictly between 0 and 1"),
                ));
            }
        }
        if self.n_min == 0 {
            problems.push(("n_min", "n_min must be at least 1".to_string()));
        }
        if self.state_path.trim().is_empty() {
            problems.push(("state_path", "state_path must name a file".to_string()));
        }
        problems
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::schema::RokoConfig;

    /// 6127: the section is off by default, round-trips through TOML, rejects unknown keys, and
    /// its checks reject out-of-range values, through `validate_invariants` too.
    #[test]
    fn self_model_config_defaults_off() {
        let expected = SelfModelConfig {
            mode: SelfModelMode::Off,
            policy: SelfModelPolicy::Cascade,
            target: 0.80,
            delta: 0.20,
            n_min: 100,
            state_path: ".roko/learn/self-model/state-v1.json".to_string(),
            allow_downward_start: true,
            allow_rung_skip: false,
        };
        let config = SelfModelConfig::default();
        assert_eq!(config, expected);
        assert!(!config.enabled());
        assert!(config.problems().is_empty());
        assert_eq!(RokoConfig::default().self_model, expected);

        let text = toml::to_string(&config).expect("serialize the section");
        let back: SelfModelConfig = toml::from_str(&text).expect("parse it back");
        assert_eq!(back, config);
        let shadow: SelfModelConfig =
            toml::from_str("mode = \"shadow\"\npolicy = \"lcb_aci\"").expect("parse two keys");
        assert_eq!(shadow.mode, SelfModelMode::Shadow);
        assert_eq!(shadow.policy, SelfModelPolicy::LcbAci);
        assert!(shadow.enabled());
        assert!(toml::from_str::<SelfModelConfig>("mood = \"shadow\"").is_err());
        assert!(toml::from_str::<SelfModelConfig>("mode = \"on\"").is_err());
        let roko =
            RokoConfig::from_toml("[self_model]\nmode = \"active\"\n").expect("parse roko.toml");
        assert_eq!(roko.self_model.mode, SelfModelMode::Active);

        let bad = SelfModelConfig {
            target: 1.0,
            delta: 0.0,
            n_min: 0,
            ..SelfModelConfig::default()
        };
        let keys: Vec<&str> = bad.problems().into_iter().map(|(key, _)| key).collect();
        assert_eq!(keys, ["target", "delta", "n_min"]);
        let mut roko = RokoConfig::default();
        roko.self_model.delta = 1.5;
        let results = crate::config::validate_invariants(&roko);
        assert!(
            results
                .iter()
                .any(|result| result.config_path == "self_model.delta")
        );
    }
}
