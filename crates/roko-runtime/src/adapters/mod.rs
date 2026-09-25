//! Stub [`ChatBridge`] adapters for messaging platforms (#409–#413, #225).
//!
//! Each adapter validates its [`PlatformConfig`] at construction time and
//! returns `Ok(Self)` so it can be registered with the [`PlatformRegistry`].
//! All `send_*` methods log the outbound message at `DEBUG` level and return
//! a synthetic [`MessageId`]; no real network connection is made.
//!
//! Real API connections are future work.  These stubs let the control plane,
//! registry, and routing logic be exercised end-to-end without requiring live
//! credentials.
//!
//! # Factory
//!
//! Use [`platform_bridge_for_config`] to instantiate the right adapter from a
//! [`PlatformConfig`] without pattern-matching on the `kind` field yourself.
//!
//! ```rust,ignore
//! use roko_runtime::adapters::platform_bridge_for_config;
//! use roko_runtime::platforms::PlatformConfig;
//! use std::sync::Arc;
//!
//! let bridge = platform_bridge_for_config(&config)?;
//! registry.register(config, Some(Arc::from(bridge))).await;
//! ```

pub mod discord;
pub mod mattermost;
pub mod matrix;
pub mod slack;
pub mod telegram;
pub mod whatsapp;

pub use discord::DiscordBridge;
pub use mattermost::MattermostBridge;
pub use matrix::MatrixBridge;
pub use slack::SlackBridge;
pub use telegram::TelegramBridge;
pub use whatsapp::WhatsAppBridge;

use roko_core::{Result, RokoError};

use crate::platforms::{ChatBridge, PlatformConfig};

// ---------------------------------------------------------------------------
// Factory
// ---------------------------------------------------------------------------

/// Instantiate the right stub adapter for the given [`PlatformConfig`].
///
/// Returns `Err` when:
/// - `config.kind` is not a recognised platform string, or
/// - the adapter's own `connect` validation fails (e.g. missing token).
///
/// All six adapters produced by this function are stubs — they log messages
/// but never make network calls.
pub fn platform_bridge_for_config(config: &PlatformConfig) -> Result<Box<dyn ChatBridge>> {
    match config.kind.as_str() {
        "discord" => {
            let bridge = DiscordBridge::connect(config)?;
            Ok(Box::new(bridge))
        }
        "slack" => {
            let bridge = SlackBridge::connect(config)?;
            Ok(Box::new(bridge))
        }
        "matrix" => {
            let bridge = MatrixBridge::connect(config)?;
            Ok(Box::new(bridge))
        }
        "whatsapp" => {
            let bridge = WhatsAppBridge::connect(config)?;
            Ok(Box::new(bridge))
        }
        "mattermost" => {
            let bridge = MattermostBridge::connect(config)?;
            Ok(Box::new(bridge))
        }
        "telegram" => {
            let bridge = TelegramBridge::connect(config)?;
            Ok(Box::new(bridge))
        }
        other => Err(RokoError::Invalid(format!(
            "unknown platform kind `{other}`; supported: discord, slack, matrix, whatsapp, mattermost, telegram"
        ))),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platforms::{PlatformConfig, PlatformSecretRef};
    use std::collections::HashMap;

    fn make_config(kind: &str) -> PlatformConfig {
        PlatformConfig {
            id: format!("{kind}-test"),
            kind: kind.to_string(),
            token: Some(PlatformSecretRef::Literal("tok".to_string())),
            description: None,
            enabled: true,
            max_retries: 3,
            retry_delay_ms: 1_000,
            extra: HashMap::new(),
            identity: None,
        }
    }

    #[test]
    fn factory_produces_all_known_kinds() {
        for kind in &["discord", "slack", "matrix", "whatsapp", "mattermost", "telegram"] {
            let cfg = make_config(kind);
            let bridge = platform_bridge_for_config(&cfg)
                .unwrap_or_else(|e| panic!("factory failed for `{kind}`: {e}"));
            assert_eq!(bridge.platform_id(), *kind);
        }
    }

    #[test]
    fn factory_rejects_unknown_kind() {
        let cfg = make_config("irc");
        assert!(platform_bridge_for_config(&cfg).is_err());
    }

    #[test]
    fn factory_requires_token_for_all_kinds() {
        for kind in &["discord", "slack", "matrix", "whatsapp", "mattermost", "telegram"] {
            let cfg = PlatformConfig {
                id: format!("{kind}-no-tok"),
                kind: kind.to_string(),
                token: None,
                description: None,
                enabled: true,
                max_retries: 3,
                retry_delay_ms: 1_000,
                extra: HashMap::new(),
                identity: None,
            };
            assert!(
                platform_bridge_for_config(&cfg).is_err(),
                "expected error for `{kind}` without token"
            );
        }
    }
}
