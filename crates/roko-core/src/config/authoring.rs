//! Plan authoring configuration section.

use serde::{Deserialize, Serialize};

// ---- [authoring] ---------------------------------------------------------

/// Plan authoring settings: which model writes and revises plans.
///
/// Frontier models plan and cheap models execute, so the planner is chosen
/// apart from the models that run tasks (`[agent]`, `[routing]`). Every plan
/// generate and revise path reads it; a `--model` flag overrides it.
///
/// ```toml
/// [authoring]
/// planner_model = "claude-opus-4-6"   # a key from [models]
/// ```
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuthoringConfig {
    /// Model that generates and revises plans, as a key from `[models]`.
    ///
    /// Empty (the default) means unset: planning falls back to the
    /// strategist role's model, then `[agent] model`. It is a string rather
    /// than an `Option` so the key always serializes; the config loader drops
    /// any key that the serialized default config lacks.
    #[serde(default)]
    pub planner_model: String,
}

impl AuthoringConfig {
    /// The configured planner model key, or `None` when unset.
    #[must_use]
    pub fn planner_model_key(&self) -> Option<&str> {
        let model = self.planner_model.trim();
        (!model.is_empty()).then_some(model)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::loader::{LoadOptions, load_config_file};
    use crate::config::schema::RokoConfig;

    #[test]
    fn config_without_authoring_leaves_the_planner_unset() {
        let config = RokoConfig::from_toml("").expect("parse empty config");

        assert_eq!(config.authoring, AuthoringConfig::default());
        assert_eq!(config.authoring.planner_model_key(), None);
    }

    #[test]
    fn planner_model_is_read_and_trimmed() {
        let config = RokoConfig::from_toml("[authoring]\nplanner_model = \" claude-opus-4-6 \"\n")
            .expect("parse authoring section");

        assert_eq!(
            config.authoring.planner_model_key(),
            Some("claude-opus-4-6")
        );
    }

    #[test]
    fn blank_planner_model_counts_as_unset() {
        let config = RokoConfig::from_toml("[authoring]\nplanner_model = \"  \"\n")
            .expect("parse authoring section");

        assert_eq!(config.authoring.planner_model_key(), None);
    }

    #[test]
    fn authoring_rejects_unknown_keys() {
        assert!(RokoConfig::from_toml("[authoring]\nplaner_model = \"x\"\n").is_err());
    }

    /// The loader strips every key that the serialized default config lacks,
    /// so a field that serializes to nothing by default would never load.
    #[test]
    fn planner_model_survives_the_config_loader() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("roko.toml");
        std::fs::write(&path, "[authoring]\nplanner_model = \"claude-opus-4-6\"\n")
            .expect("write roko.toml");
        let opts = LoadOptions {
            merge_global: false,
            apply_env_overrides: false,
            apply_hierarchical_env: false,
            strict_validation: false,
        };

        let config = load_config_file(&path, &opts).expect("load roko.toml");

        assert_eq!(
            config.authoring.planner_model_key(),
            Some("claude-opus-4-6")
        );
    }
}
