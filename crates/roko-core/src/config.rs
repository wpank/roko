//! Roko runtime configuration.

use serde::{Deserialize, Serialize};

/// Current schema version for `RokoConfig`. Bump when adding incompatible fields.
pub const CURRENT_SCHEMA_VERSION: u32 = 1;

/// Root configuration type for the Roko runtime.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RokoConfig {
    /// Schema version — drives migration tooling (§39.5).
    #[serde(default = "default_schema_version")]
    pub schema_version: u32,

    /// Any additional top-level config sections live alongside this field.
    #[serde(flatten)]
    pub extra: toml::Table,
}

const fn default_schema_version() -> u32 {
    CURRENT_SCHEMA_VERSION
}

impl Default for RokoConfig {
    fn default() -> Self {
        Self {
            schema_version: CURRENT_SCHEMA_VERSION,
            extra: toml::Table::new(),
        }
    }
}

impl RokoConfig {
    /// Parses a `RokoConfig` from a TOML string.
    pub fn from_toml(s: &str) -> Result<Self, toml::de::Error> {
        toml::from_str(s)
    }

    /// Renders to TOML string.
    pub fn to_toml(&self) -> Result<String, toml::ser::Error> {
        toml::to_string(self)
    }

    /// Returns true when the config was loaded from an older schema version.
    pub const fn is_stale(&self) -> bool {
        self.schema_version < CURRENT_SCHEMA_VERSION
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_with_schema_version() {
        let cfg = RokoConfig::default();
        let text = cfg.to_toml().unwrap();
        let back: RokoConfig = RokoConfig::from_toml(&text).unwrap();
        assert_eq!(back.schema_version, CURRENT_SCHEMA_VERSION);
    }

    #[test]
    fn absent_schema_version_defaults_to_current() {
        let cfg: RokoConfig = RokoConfig::from_toml("").unwrap();
        assert_eq!(cfg.schema_version, CURRENT_SCHEMA_VERSION);
    }

    #[test]
    fn is_stale_detects_old_version() {
        let cfg = RokoConfig {
            schema_version: 0,
            extra: toml::Table::new(),
        };
        assert!(cfg.is_stale());
    }

    #[test]
    fn extra_fields_preserved() {
        let toml_str = r#"
schema_version = 1
[agents]
default_backend = "claude"
"#;
        let cfg = RokoConfig::from_toml(toml_str).unwrap();
        assert_eq!(cfg.schema_version, 1);
        assert!(cfg.extra.contains_key("agents"));
    }
}
