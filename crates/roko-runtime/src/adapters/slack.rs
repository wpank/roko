//! Stub Slack [`ChatBridge`] adapter (#410).
//!
//! Slack supports its own `mrkdwn` notation (a subset of Markdown), reactions,
//! and threaded replies.  Attachments are supported via Block Kit but are not
//! declared as a capability flag here because the stub does not implement them.
//!
//! # Config
//!
//! ```toml
//! [[platforms]]
//! id    = "slack-ops"
//! kind  = "slack"
//! token = { env = "SLACK_BOT_TOKEN" }
//! ```
//!
//! # Future work
//!
//! Replace stub bodies with calls to the Slack Web API
//! (`https://slack.com/api/chat.postMessage`).  The `incoming` stream should
//! drive the Events API or Socket Mode WebSocket.

use std::pin::Pin;

use async_trait::async_trait;
use futures::Stream;
use roko_core::{Result, RokoError};

use crate::platforms::{
    ChannelId, ChatBridge, IncomingMessage, MessageId, PlatformCapabilities, PlatformConfig,
    RichMessage,
};

// ---------------------------------------------------------------------------
// SlackBridge
// ---------------------------------------------------------------------------

/// Stub Slack adapter.
///
/// Supports `mrkdwn` formatting, reactions, and threads.
/// All outbound sends are logged at `DEBUG` level; no real Slack API call
/// is made.  Incoming messages are an empty stream.
pub struct SlackBridge {
    /// Resolved bot token (validated non-empty at construction).
    #[allow(dead_code)]
    token: String,
}

impl SlackBridge {
    /// Construct and validate a [`SlackBridge`] from a [`PlatformConfig`].
    ///
    /// Resolves the bot token (literal or env var) and returns an error if no
    /// token is configured or the env var is unset.
    pub fn connect(config: &PlatformConfig) -> Result<Self> {
        let token = config
            .token
            .as_ref()
            .ok_or_else(|| {
                RokoError::Invalid(format!(
                    "slack platform `{}` requires a token",
                    config.id
                ))
            })?
            .resolve()
            .map_err(|e| RokoError::Invalid(format!("slack `{}`: {e}", config.id)))?;

        if token.is_empty() {
            return Err(RokoError::Invalid(format!(
                "slack platform `{}` token must not be empty",
                config.id
            )));
        }

        Ok(Self { token })
    }
}

#[async_trait]
impl ChatBridge for SlackBridge {
    fn platform_id(&self) -> &str {
        "slack"
    }

    async fn send_message(&self, channel: &str, text: &str) -> Result<MessageId> {
        tracing::debug!(
            platform = "slack",
            channel = channel,
            "stub send_message: {}",
            &text[..text.len().min(120)]
        );
        Ok(MessageId::new(format!("slack-stub-{}", ChannelId::new(channel))))
    }

    async fn send_rich(&self, channel: &str, msg: RichMessage) -> Result<MessageId> {
        let body = msg.markdown.as_deref().unwrap_or(&msg.text);
        tracing::debug!(
            platform = "slack",
            channel = channel,
            "stub send_rich: {}",
            &body[..body.len().min(120)]
        );
        Ok(MessageId::new(format!("slack-stub-rich-{}", ChannelId::new(channel))))
    }

    fn incoming(&self) -> Pin<Box<dyn Stream<Item = IncomingMessage> + Send>> {
        Box::pin(futures::stream::empty())
    }

    fn capabilities(&self) -> PlatformCapabilities {
        PlatformCapabilities {
            // Slack uses mrkdwn — counted as markdown-capable.
            markdown: true,
            attachments: false,
            edit_messages: true,
            threads: true,
            interactive_components: true,
            reactions: true,
            max_message_length: Some(40_000),
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
            id: "slack-test".to_string(),
            kind: "slack".to_string(),
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
        assert!(SlackBridge::connect(&config(None)).is_err());
    }

    #[test]
    fn connect_rejects_empty_token() {
        let cfg = config(Some(PlatformSecretRef::Literal(String::new())));
        assert!(SlackBridge::connect(&cfg).is_err());
    }

    #[test]
    fn connect_succeeds_with_literal_token() {
        let cfg = config(Some(PlatformSecretRef::Literal("xoxb-token".to_string())));
        assert!(SlackBridge::connect(&cfg).is_ok());
    }

    #[test]
    fn platform_id_is_slack() {
        let bridge = SlackBridge::connect(&config(Some(PlatformSecretRef::Literal(
            "tok".to_string(),
        ))))
        .unwrap();
        assert_eq!(bridge.platform_id(), "slack");
    }

    #[test]
    fn capabilities_match_slack() {
        let bridge = SlackBridge::connect(&config(Some(PlatformSecretRef::Literal(
            "tok".to_string(),
        ))))
        .unwrap();
        let caps = bridge.capabilities();
        assert!(caps.markdown);
        assert!(caps.reactions);
        assert!(caps.threads);
        assert!(!caps.attachments);
    }

    #[tokio::test]
    async fn send_message_returns_ok() {
        let bridge = SlackBridge::connect(&config(Some(PlatformSecretRef::Literal(
            "tok".to_string(),
        ))))
        .unwrap();
        let id = bridge.send_message("#general", "hello slack").await.unwrap();
        assert!(id.as_str().contains("slack-stub"));
    }

    #[tokio::test]
    async fn send_rich_returns_ok() {
        let bridge = SlackBridge::connect(&config(Some(PlatformSecretRef::Literal(
            "tok".to_string(),
        ))))
        .unwrap();
        let msg = RichMessage::markdown("hello", "*hello*");
        let id = bridge.send_rich("#general", msg).await.unwrap();
        assert!(id.as_str().contains("slack-stub-rich"));
    }
}
