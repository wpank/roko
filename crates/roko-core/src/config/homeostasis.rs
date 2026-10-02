//! `[homeostasis]`: M1's mode and the controller's constants (S06 §5).
//!
//! M1, the ultrastable controller, runs `off`, in `shadow` or `on`. Shadow is
//! the default (decision 8102, D8): the whole loop runs and logs the moves it
//! would make, while dispatch keeps θ₀. Switching to `on` is a human action,
//! taken once S06 §7 A1–A3 are green: nothing in roko writes this key, and M1
//! never changes its own mode.
//!
//! The viability bounds are not config. M1 reads them, read-only, from the S5
//! policy file `.roko/policy/viability.toml`.

use serde::{Deserialize, Serialize};

/// How M1 runs (S06 §4.7).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HomeostasisMode {
    /// Nothing runs.
    Off,
    /// The loop runs and logs each move as `param.change` with
    /// `applied: false`; dispatch uses θ₀.
    #[default]
    Shadow,
    /// Moves apply: θ is swapped after its `param.change` row is written.
    On,
}

/// `[homeostasis]` in `roko.toml` (S06 §5). Every key is optional.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct HomeostasisConfig {
    /// `off`, `shadow` (the default) or `on`.
    pub mode: HomeostasisMode,
    /// Share of chains on the static holdout arm, which runs θ₀ throughout
    /// (S06 §4.8). At most 0.5, and above 0 while `mode = "on"`, or M1's
    /// effect cannot be measured.
    pub holdout: f64,
    /// Resolutions in each essential variable's estimator window (W).
    pub window: u32,
    /// Consecutive detector alarms that confirm a breach.
    pub confirm_k: u32,
    /// Resolutions a change dwells before it is kept or rolled back.
    pub dwell_resolutions: u32,
    /// Seconds a change dwells at least, besides `dwell_resolutions`.
    pub dwell_min_secs: u64,
    /// Changes an episode may make before M1 holds and alerts (N_max).
    pub max_changes_per_episode: u32,
    /// Resolutions after an episode before the same variable may open
    /// another one.
    pub refractory: u32,
    /// Resolutions every variable must stay inside its inner band before θ
    /// is committed as last-known-good (W_rec).
    pub recover_window: u32,
    /// In-bounds resolutions before M1 relaxes one notch toward θ₀
    /// (W_relax).
    pub relax_window: u32,
    /// Chance that a move is an Ashby random step instead of a guided one
    /// (ε_rand).
    pub random_step_prob: f64,
    /// Discount on older episodes' move posteriors (γ).
    pub posterior_discount: f64,
    /// Share of the pre-change drive a change must remove to be kept (δ).
    pub improve_delta_frac: f64,
    /// Adaptation spend, as a share of run spend, beyond which M1 holds
    /// (A_max).
    pub adaptation_spend_max_frac: f64,
    /// Seed the move priors from the self-model (M3).
    pub m3_prior: bool,
    /// Halve the detection threshold when M3 predicts a jump in the drive.
    /// An ablation arm, off by default.
    pub feedforward_prearm: bool,
}

impl Default for HomeostasisConfig {
    fn default() -> Self {
        Self {
            mode: HomeostasisMode::Shadow,
            holdout: 0.10,
            window: 20,
            confirm_k: 2,
            dwell_resolutions: 8,
            dwell_min_secs: 120,
            max_changes_per_episode: 6,
            refractory: 20,
            recover_window: 10,
            relax_window: 40,
            random_step_prob: 0.2,
            posterior_discount: 0.9,
            improve_delta_frac: 0.05,
            adaptation_spend_max_frac: 0.15,
            m3_prior: true,
            feedforward_prearm: false,
        }
    }
}

impl HomeostasisConfig {
    /// What makes this section unusable, as `(key, problem)` pairs; empty
    /// when the section is valid.
    #[must_use]
    pub fn problems(&self) -> Vec<(&'static str, String)> {
        let mut problems = Vec::new();
        if !(0.0..=0.5).contains(&self.holdout) {
            problems.push((
                "holdout",
                format!("holdout ({}) must be between 0 and 0.5", self.holdout),
            ));
        } else if self.holdout == 0.0 && self.mode == HomeostasisMode::On {
            problems.push((
                "holdout",
                "holdout must be above 0 while mode = \"on\": without a holdout arm M1's effect \
                 cannot be measured"
                    .to_string(),
            ));
        }
        let counts = [
            ("window", self.window),
            ("confirm_k", self.confirm_k),
            ("dwell_resolutions", self.dwell_resolutions),
            ("max_changes_per_episode", self.max_changes_per_episode),
            ("recover_window", self.recover_window),
            ("relax_window", self.relax_window),
        ];
        for (key, value) in counts {
            if value == 0 {
                problems.push((key, format!("{key} must be at least 1")));
            }
        }
        let fractions = [
            ("random_step_prob", self.random_step_prob),
            ("posterior_discount", self.posterior_discount),
            ("improve_delta_frac", self.improve_delta_frac),
            ("adaptation_spend_max_frac", self.adaptation_spend_max_frac),
        ];
        for (key, value) in fractions {
            if !(0.0..=1.0).contains(&value) {
                problems.push((key, format!("{key} ({value}) must be between 0 and 1")));
            }
        }
        problems
    }

    /// Check the section.
    ///
    /// # Errors
    ///
    /// Every problem of [`Self::problems`], joined with `; `.
    pub fn validate(&self) -> Result<(), String> {
        let problems = self.problems();
        if problems.is_empty() {
            return Ok(());
        }
        Err(problems
            .into_iter()
            .map(|(key, problem)| format!("homeostasis.{key}: {problem}"))
            .collect::<Vec<_>>()
            .join("; "))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::schema::RokoConfig;
    use crate::config::validation::{InvariantSeverity, validate_invariants};

    #[test]
    fn homeostasis_config_defaults_and_validation() {
        // The defaults are S06 §5's, with shadow mode (decision 8102).
        let defaults = HomeostasisConfig::default();
        assert_eq!(defaults.mode, HomeostasisMode::Shadow);
        assert_eq!(defaults.holdout, 0.10);
        assert_eq!(defaults.window, 20);
        assert_eq!(defaults.confirm_k, 2);
        assert_eq!(defaults.dwell_resolutions, 8);
        assert_eq!(defaults.dwell_min_secs, 120);
        assert_eq!(defaults.max_changes_per_episode, 6);
        assert_eq!(defaults.refractory, 20);
        assert_eq!(defaults.recover_window, 10);
        assert_eq!(defaults.relax_window, 40);
        assert_eq!(defaults.random_step_prob, 0.2);
        assert_eq!(defaults.posterior_discount, 0.9);
        assert_eq!(defaults.improve_delta_frac, 0.05);
        assert_eq!(defaults.adaptation_spend_max_frac, 0.15);
        assert!(defaults.m3_prior);
        assert!(!defaults.feedforward_prearm);
        assert_eq!(defaults.validate(), Ok(()));
        assert_eq!(RokoConfig::default().homeostasis, defaults);

        // No section, an empty one, and a partial one all load.
        let config = RokoConfig::from_toml("").expect("a config without the section loads");
        assert_eq!(config.homeostasis, defaults);
        let config = RokoConfig::from_toml("[homeostasis]\n").expect("an empty section loads");
        assert_eq!(config.homeostasis, defaults);
        let config = RokoConfig::from_toml("[homeostasis]\nmode = \"off\"\nwindow = 30\n")
            .expect("a partial section loads");
        assert_eq!(config.homeostasis.mode, HomeostasisMode::Off);
        assert_eq!(config.homeostasis.window, 30);
        assert_eq!(config.homeostasis.holdout, 0.10);
        assert!(RokoConfig::from_toml("[homeostasis]\nmode = \"auto\"\n").is_err());
        assert!(RokoConfig::from_toml("[homeostasis]\nmodes = \"on\"\n").is_err());

        // `roko config show` prints the section.
        let text = crate::config::loader::serialize_effective_redacted(&RokoConfig::default())
            .expect("config to TOML");
        assert!(text.contains("[homeostasis]"), "{text}");
        assert!(text.contains("mode = \"shadow\""), "{text}");

        // `on` with no holdout is rejected, here and by the loader's
        // invariants; other modes may set it to 0.
        let on_without_holdout = HomeostasisConfig {
            mode: HomeostasisMode::On,
            holdout: 0.0,
            ..defaults.clone()
        };
        assert!(on_without_holdout.validate().is_err());
        let mut config = RokoConfig::default();
        config.homeostasis = on_without_holdout;
        let rejected = validate_invariants(&config);
        assert!(
            rejected.iter().any(|result| result.invariant_id == 10
                && result.severity == InvariantSeverity::Error
                && result.config_path == "homeostasis.holdout"),
            "{rejected:?}"
        );
        let shadow_without_holdout = HomeostasisConfig {
            holdout: 0.0,
            ..defaults.clone()
        };
        assert_eq!(shadow_without_holdout.validate(), Ok(()));
        let on = HomeostasisConfig {
            mode: HomeostasisMode::On,
            ..defaults.clone()
        };
        assert_eq!(on.validate(), Ok(()));

        // Windows and dwell are positive, fractions lie in [0, 1], and the
        // holdout is at most 0.5.
        let broken = HomeostasisConfig {
            holdout: 0.6,
            window: 0,
            dwell_resolutions: 0,
            random_step_prob: 1.5,
            posterior_discount: f64::NAN,
            ..defaults
        };
        let keys: Vec<&str> = broken.problems().into_iter().map(|(key, _)| key).collect();
        assert_eq!(
            keys,
            [
                "holdout",
                "window",
                "dwell_resolutions",
                "random_step_prob",
                "posterior_discount"
            ]
        );
    }
}
