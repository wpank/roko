//! Stub Discord [`ChatBridge`] adapter (#409).
//!
//! Discord supports rich markdown, reactions, threaded replies, and file
//! attachments. This stub validates the config and logs outbound messages
//! without making any real API calls.
//!
//! # Config
//!
//! ```toml
//! [[platforms]]
//! id    = "discord-main"
//! kind  = "discord"
//! token = { env = "DISCORD_BOT_TOKEN" }
//! ```
//!
//! # Future work
//!
//! Replace the stub body with real HTTP calls to the Discord REST API
//! (`https://discord.com/api/v10`). Gateway WebSocket events can back the
//! `incoming` stream.

use std::pin::Pin;

use async_trait::async_trait;
use futures::Stream;
use roko_core::{Result, RokoError};

use crate::platforms::{
    ChannelId, ChatBridge, IncomingMessage, MessageId, PlatformCapabilities, PlatformConfig,
    RichMessage,
};

// ---------------------------------------------------------------------------
// DiscordBridge
// ---------------------------------------------------------------------------

/// Stub Discord adapter.
///
/// Supports markdown, reactions, threads, and attachments.
/// All outbound sends are logged at `DEBUG` level; no real Discord API call
/// is made.  Incoming messages are an empty stream.
pub struct DiscordBridge {
    /// Resolved bot token (validated non-empty at construction).
    #[allow(dead_code)]
    token: String,
}

impl DiscordBridge {
    /// Construct and validate a [`DiscordBridge`] from a [`PlatformConfig`].
    ///
    /// Resolves the bot token from the config (literal or env var) and returns
    /// an error if no token is configured or the env var is not set.
    pub fn connect(config: &PlatformConfig) -> Result<Self> {
        let token = config
            .token
            .as_ref()
            .ok_or_else(|| {
                RokoError::Invalid(format!(
                    "discord platform `{}` requires a token",
                    config.id
                ))
            })?
            .resolve()
            .map_err(|e| RokoError::Invalid(format!("discord `{}`: {e}", config.id)))?;

        if token.is_empty() {
            return Err(RokoError::Invalid(format!(
                "discord platform `{}` token must not be empty",
                config.id
            )));
        }

        Ok(Self { token })
    }
}

#[async_trait]
impl ChatBridge for DiscordBridge {
    fn platform_id(&self) -> &str {
        "discord"
    }

    async fn send_message(&self, channel: &str, text: &str) -> Result<MessageId> {
        tracing::debug!(
            platform = "discord",
            channel = channel,
            "stub send_message: {}",
            &text[..text.len().min(120)]
        );
        Ok(MessageId::new(format!("discord-stub-{}", ChannelId::new(channel))))
    }

    async fn send_rich(&self, channel: &str, msg: RichMessage) -> Result<MessageId> {
        let body = msg.markdown.as_deref().unwrap_or(&msg.text);
        tracing::debug!(
            platform = "discord",
            channel = channel,
            attachments = msg.attachments.len(),
            "stub send_rich: {}",
            &body[..body.len().min(120)]
        );
        Ok(MessageId::new(format!("discord-stub-rich-{}", ChannelId::new(channel))))
    }

    fn incoming(&self) -> Pin<Box<dyn Stream<Item = IncomingMessage> + Send>> {
        Box::pin(futures::stream::empty())
    }

    fn capabilities(&self) -> PlatformCapabilities {
        PlatformCapabilities {
            markdown: true,
            attachments: true,
            edit_messages: true,
            threads: true,
            interactive_components: true,
            reactions: true,
            max_message_length: Some(2_000),
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platforms::{PlatformConfig, PlatformSecretRef, RichMessage};
    use std::collections::HashMap;

    fn config(token: Option<PlatformSecretRef>) -> PlatformConfig {
        PlatformConfig {
            id: "discord-test".to_string(),
            kind: "discord".to_string(),
            token,
            description: None,
            enabled: true,
            max_retries: 3,
            retry_delay_ms: 1_000,
            extra: HashMap::new(),
            identity: None,
        }
    }

    #[test]
    fn connect_requires_token() {
        assert!(DiscordBridge::connect(&config(None)).is_err());
    }

    #[test]
    fn connect_rejects_empty_token() {
        let cfg = config(Some(PlatformSecretRef::Literal(String::new())));
        assert!(DiscordBridge::connect(&cfg).is_err());
    }

    #[test]
    fn connect_succeeds_with_literal_token() {
        let cfg = config(Some(PlatformSecretRef::Literal("bot-token".to_string())));
        assert!(DiscordBridge::connect(&cfg).is_ok());
    }

    #[test]
    fn platform_id_is_discord() {
        let bridge = DiscordBridge::connect(&config(Some(PlatformSecretRef::Literal(
            "tok".to_string(),
        ))))
        .unwrap();
        assert_eq!(bridge.platform_id(), "discord");
    }

    #[test]
    fn capabilities_match_discord() {
        let bridge = DiscordBridge::connect(&config(Some(PlatformSecretRef::Literal(
            "tok".to_string(),
        ))))
        .unwrap();
        let caps = bridge.capabilities();
        assert!(caps.markdown);
        assert!(caps.attachments);
        assert!(caps.threads);
        assert!(caps.reactions);
        assert_eq!(caps.max_message_length, Some(2_000));
    }

    #[tokio::test]
    async fn send_message_returns_ok() {
        let bridge = DiscordBridge::connect(&config(Some(PlatformSecretRef::Literal(
            "tok".to_string(),
        ))))
        .unwrap();
        let id = bridge.send_message("123456789", "hello discord").await.unwrap();
        assert!(id.as_str().contains("discord-stub"));
    }

    #[tokio::test]
    async fn send_rich_returns_ok() {
        let bridge = DiscordBridge::connect(&config(Some(PlatformSecretRef::Literal(
            "tok".to_string(),
        ))))
        .unwrap();
        let msg = RichMessage::markdown("hello", "**hello**");
        let id = bridge.send_rich("123456789", msg).await.unwrap();
        assert!(id.as_str().contains("discord-stub-rich"));
    }
}
