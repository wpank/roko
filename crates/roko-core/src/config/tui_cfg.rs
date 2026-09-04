//! Terminal UI configuration.

use serde::{Deserialize, Serialize};

/// Terminal UI preferences.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TuiConfig {
    /// Refresh interval in milliseconds.
    #[serde(default = "default_refresh_rate")]
    pub refresh_rate_ms: u64,
    /// Visual effects sub-table parsed by the TUI crate.
    ///
    /// This field is intentionally stored as an opaque `toml::Value` because
    /// the full `EffectsConfig` type lives in `roko-cli` (which depends on
    /// `roko-core`, not the other way around). Keeping the field here lets the
    /// config schema validator accept `[tui.effects]` without a false-positive
    /// "unknown config key" diagnostic.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effects: Option<toml::Value>,
}

const fn default_refresh_rate() -> u64 {
    250
}

impl Default for TuiConfig {
    fn default() -> Self {
        Self {
            refresh_rate_ms: default_refresh_rate(),
            effects: None,
        }
    }
}
