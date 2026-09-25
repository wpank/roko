//! `[[platforms]]` configuration section.
//!
//! Each entry in the `platforms` array declares a messaging-platform connection
//! that the serve layer will register with the `PlatformRegistry` at startup.
//!
//! # Example
//!
//! ```toml
//! [[platforms]]
//! id    = "discord-main"
//! kind  = "discord"
//! token = { env = "DISCORD_TOKEN" }
//!
//! [[platforms]]
//! id          = "slack-ops"
//! kind        = "slack"
//! token       = { env = "SLACK_BOT_TOKEN" }
//! description = "Engineering ops channel"
//! enabled     = false
//! ```

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// SecretRef — literal value or env-var indirection
// ---------------------------------------------------------------------------

/// A secret value: either a literal string (not recommended in production) or
/// an environment-variable name resolved at startup.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PlatformSecretRef {
    /// Literal plaintext value embedded in the config file.
    Literal(String),
    /// Environment variable whose value is read at startup.
    Env {
        /// Name of the environment variable (e.g. `"DISCORD_TOKEN"`).
        env: String,
    },
}

impl PlatformSecretRef {
    /// Resolve the secret: return the literal value or read the env var.
    ///
    /// Returns an error if the environment variable is not set.
    pub fn resolve(&self) -> Result<String, String> {
        match self {
            Self::Literal(s) => Ok(s.clone()),
            Self::Env { env } => std::env::var(env).map_err(|_| {
                format!("platform secret env var `{env}` is not set")
            }),
        }
    }
}

// ---------------------------------------------------------------------------
// PlatformConfig
// ---------------------------------------------------------------------------

fn default_true() -> bool {
    true
}

/// Default maximum number of webhook delivery retries (#418).
fn default_max_retries() -> u32 {
    3
}

/// Default base retry delay in milliseconds (#418).
fn default_retry_delay_ms() -> u64 {
    1_000
}

// ---------------------------------------------------------------------------
// AgentIdentity
// ---------------------------------------------------------------------------

/// Per-platform agent persona (#427).
///
/// Declares the display name, avatar, and platform user ID that roko should
/// present itself as when sending messages through this platform.  All fields
/// are optional: when absent the adapter uses whatever identity is implicit in
/// the token.
///
/// # Example
///
/// ```toml
/// [[platforms]]
/// id   = "discord-main"
/// kind = "discord"
///
/// [platforms.identity]
/// display_name     = "roko"
/// avatar_url       = "https://example.com/roko-avatar.png"
/// platform_user_id = "1234567890"
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct AgentIdentity {
    /// Human-readable display name shown alongside messages.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,

    /// URL of the avatar image.  The adapter uses this when the platform
    /// supports custom bot avatars (e.g. Discord webhook overrides).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub avatar_url: Option<String>,

    /// Platform-specific user or bot ID.  May be used by adapters to
    /// reference their own messages or to filter echoed incoming messages.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub platform_user_id: Option<String>,
}

// ---------------------------------------------------------------------------
// PlatformConfig
// ---------------------------------------------------------------------------

/// A single `[[platforms]]` entry from `roko.toml`.
///
/// Fields beyond `id` and `kind` are optional. Adapter-specific settings that
/// don't fit the common schema go into `extra`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlatformConfig {
    /// Stable user-chosen identifier for this connection (e.g. `"discord-main"`).
    ///
    /// Must be unique within the `[[platforms]]` array.
    pub id: String,

    /// Platform kind: `"discord"`, `"slack"`, `"matrix"`, `"telegram"`, etc.
    ///
    /// An unknown kind records the entry as `Unsupported` in the registry.
    pub kind: String,

    /// Bot/app token.  Adapters that require a token will fail to connect if
    /// this field is absent and no alternative auth is configured.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub token: Option<PlatformSecretRef>,

    /// Human-readable description surfaced by the status API.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Whether the platform connection should be started automatically.
    ///
    /// Disabled entries are registered but never connected (status remains
    /// `Disconnected`).  Defaults to `true`.
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Maximum number of outbound delivery retries after the first attempt (#418).
    ///
    /// A value of `0` disables retries (deliver once or dead-letter immediately).
    /// Defaults to `3`.
    #[serde(default = "default_max_retries")]
    pub max_retries: u32,

    /// Base delay between delivery retries in milliseconds (#418).
    ///
    /// The actual delay follows exponential backoff starting at this value,
    /// capped at 30 seconds.  Defaults to `1000` (1 second).
    #[serde(default = "default_retry_delay_ms")]
    pub retry_delay_ms: u64,

    /// Per-platform agent persona (#427).
    ///
    /// When set, adapters should use these values to identify the bot to the
    /// platform (display name, avatar, and user ID).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub identity: Option<AgentIdentity>,

    /// Adapter-specific key/value settings not covered by the common schema.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub extra: HashMap<String, serde_json::Value>,
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn platform_config_roundtrip_toml() {
        let raw = r#"
id    = "discord-main"
kind  = "discord"
token = { env = "DISCORD_TOKEN" }
description = "Main Discord server"
enabled = true
"#;
        let cfg: PlatformConfig = toml::from_str(raw).unwrap();
        assert_eq!(cfg.id, "discord-main");
        assert_eq!(cfg.kind, "discord");
        assert!(cfg.enabled);
        assert_eq!(
            cfg.token,
            Some(PlatformSecretRef::Env {
                env: "DISCORD_TOKEN".to_string()
            })
        );
        assert_eq!(cfg.description.as_deref(), Some("Main Discord server"));
    }

    #[test]
    fn platform_config_disabled_defaults() {
        let raw = r#"
id   = "slack-ops"
kind = "slack"
"#;
        let cfg: PlatformConfig = toml::from_str(raw).unwrap();
        assert!(cfg.enabled); // default = true
        assert!(cfg.token.is_none());
        assert!(cfg.description.is_none());
    }

    #[test]
    fn secret_ref_literal_resolve() {
        let s = PlatformSecretRef::Literal("mytoken".to_string());
        assert_eq!(s.resolve().unwrap(), "mytoken");
    }

    #[test]
    fn secret_ref_env_missing_returns_error() {
        let s = PlatformSecretRef::Env {
            env: "ROKO_TEST_ABSENT_PLATFORM_TOKEN_XYZ".to_string(),
        };
        assert!(s.resolve().is_err());
    }

    #[test]
    fn platform_config_with_identity_roundtrip() {
        let raw = r#"
id   = "discord-main"
kind = "discord"

[identity]
display_name     = "roko"
avatar_url       = "https://example.com/avatar.png"
platform_user_id = "987654321"
"#;
        let cfg: PlatformConfig = toml::from_str(raw).unwrap();
        let identity = cfg.identity.unwrap();
        assert_eq!(identity.display_name.as_deref(), Some("roko"));
        assert_eq!(identity.avatar_url.as_deref(), Some("https://example.com/avatar.png"));
        assert_eq!(identity.platform_user_id.as_deref(), Some("987654321"));
    }

    #[test]
    fn platform_config_identity_absent_by_default() {
        let raw = r#"
id   = "slack-ops"
kind = "slack"
"#;
        let cfg: PlatformConfig = toml::from_str(raw).unwrap();
        assert!(cfg.identity.is_none());
    }

    #[test]
    fn agent_identity_default_is_all_none() {
        let identity = AgentIdentity::default();
        assert!(identity.display_name.is_none());
        assert!(identity.avatar_url.is_none());
        assert!(identity.platform_user_id.is_none());
    }
}
