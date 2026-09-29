//! `[[channels]]` configuration table (#426).
//!
//! Each entry maps a messaging-platform channel to a Roko agent or a set of
//! trigger bindings.  This lets operators declaratively route messages from a
//! specific Slack channel, Discord server channel, or Matrix room to the
//! correct agent without writing code.
//!
//! # Example
//!
//! ```toml
//! [[channels]]
//! platform_id     = "slack-ops"
//! channel_id      = "C01AB2CD3EF"
//! agent_name      = "ops-agent"
//! trigger_bindings = ["deploy-webhook", "cron-daily"]
//!
//! [[channels]]
//! platform_id = "discord-main"
//! channel_id  = "987654321098765432"
//! agent_name  = "code-review-agent"
//! ```

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// ChannelConfig
// ---------------------------------------------------------------------------

/// A single `[[channels]]` entry from `roko.toml`.
///
/// Each entry binds a platform channel to an agent and/or a set of trigger
/// bindings.  At least one of `agent_name` or `trigger_bindings` should be
/// non-empty for the entry to be actionable.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChannelConfig {
    /// The `[[platforms]]` `id` value for the platform that owns this channel.
    ///
    /// For example, `"slack-ops"` or `"discord-main"`.  The referenced platform
    /// must appear in the same `roko.toml` (or a merged global config) or the
    /// entry is skipped at startup.
    pub platform_id: String,

    /// Platform-specific channel identifier.
    ///
    /// The encoding is platform-specific:
    /// - Slack: channel ID (`"C01AB2CD3EF"`) or name (`"#ops"`)
    /// - Discord: snowflake channel ID (`"987654321098765432"`)
    /// - Matrix: room alias or room ID (`"!room:server.tld"`)
    /// - Telegram: chat ID (numeric string, e.g. `"-100123456789"`)
    pub channel_id: String,

    /// Name of the agent to forward messages from this channel to.
    ///
    /// Must match an `[[agents]]` `name` in the same config.
    /// When absent, the channel is only used for trigger routing.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent_name: Option<String>,

    /// Names of trigger bindings that should fire when an event arrives in
    /// this channel.
    ///
    /// Each entry must correspond to a trigger binding in
    /// `.roko/triggers/<name>.toml`.  When a message arrives on the platform
    /// channel, the serve layer fires each listed trigger by publishing a
    /// [`TriggerEvent`](crate::trigger::TriggerEvent) pulse.
    ///
    /// Defaults to an empty list.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub trigger_bindings: Vec<String>,
}

impl ChannelConfig {
    /// Return `true` if the channel entry has at least one action configured
    /// (an agent name or at least one trigger binding).
    #[must_use]
    pub fn has_action(&self) -> bool {
        self.agent_name.is_some() || !self.trigger_bindings.is_empty()
    }

    /// Return a human-readable description of the channel, suitable for
    /// log output and doctor diagnostics.
    #[must_use]
    pub fn display_name(&self) -> String {
        format!("{}#{}", self.platform_id, self.channel_id)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_toml_full_entry() {
        let raw = r#"
platform_id      = "slack-ops"
channel_id       = "C01AB2CD3EF"
agent_name       = "ops-agent"
trigger_bindings = ["deploy-webhook", "cron-daily"]
"#;
        let cfg: ChannelConfig = toml::from_str(raw).unwrap();
        assert_eq!(cfg.platform_id, "slack-ops");
        assert_eq!(cfg.channel_id, "C01AB2CD3EF");
        assert_eq!(cfg.agent_name.as_deref(), Some("ops-agent"));
        assert_eq!(cfg.trigger_bindings, ["deploy-webhook", "cron-daily"]);
        assert!(cfg.has_action());
    }

    #[test]
    fn roundtrip_toml_minimal_entry_agent_only() {
        let raw = r#"
platform_id = "discord-main"
channel_id  = "987654321098765432"
agent_name  = "code-review-agent"
"#;
        let cfg: ChannelConfig = toml::from_str(raw).unwrap();
        assert_eq!(cfg.platform_id, "discord-main");
        assert_eq!(cfg.channel_id, "987654321098765432");
        assert_eq!(cfg.agent_name.as_deref(), Some("code-review-agent"));
        assert!(cfg.trigger_bindings.is_empty());
        assert!(cfg.has_action());
    }

    #[test]
    fn roundtrip_toml_triggers_only() {
        let raw = r#"
platform_id      = "matrix-general"
channel_id       = "!room:server.tld"
trigger_bindings = ["incident-alert"]
"#;
        let cfg: ChannelConfig = toml::from_str(raw).unwrap();
        assert!(cfg.agent_name.is_none());
        assert_eq!(cfg.trigger_bindings, ["incident-alert"]);
        assert!(cfg.has_action());
    }

    #[test]
    fn no_action_when_both_absent() {
        let cfg = ChannelConfig {
            platform_id: "slack-ops".to_string(),
            channel_id: "C0".to_string(),
            agent_name: None,
            trigger_bindings: Vec::new(),
        };
        assert!(!cfg.has_action());
    }

    #[test]
    fn display_name_format() {
        let cfg = ChannelConfig {
            platform_id: "slack-ops".to_string(),
            channel_id: "#general".to_string(),
            agent_name: None,
            trigger_bindings: Vec::new(),
        };
        assert_eq!(cfg.display_name(), "slack-ops##general");
    }

    #[test]
    fn serde_omits_empty_fields() {
        let cfg = ChannelConfig {
            platform_id: "test-plat".to_string(),
            channel_id: "ch1".to_string(),
            agent_name: None,
            trigger_bindings: Vec::new(),
        };
        let serialized = toml::to_string_pretty(&cfg).unwrap();
        // Optional empty fields should not appear in serialized output.
        assert!(!serialized.contains("agent_name"));
        assert!(!serialized.contains("trigger_bindings"));
    }

    #[test]
    fn serde_roundtrip_json() {
        let cfg = ChannelConfig {
            platform_id: "slack-ops".to_string(),
            channel_id: "C01AB2CD3EF".to_string(),
            agent_name: Some("ops-agent".to_string()),
            trigger_bindings: vec!["deploy-webhook".to_string()],
        };
        let json = serde_json::to_string(&cfg).unwrap();
        let restored: ChannelConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(cfg, restored);
    }
}
