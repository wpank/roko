//! Stub Mattermost [`ChatBridge`] adapter (#413).
//!
//! Mattermost supports Markdown formatting and emoji reactions.  It does not
//! support interactive components or threads at the API level used here.
//!
//! # Config
//!
//! ```toml
//! [[platforms]]
//! id    = "mattermost-eng"
//! kind  = "mattermost"
//! token = { env = "MATTERMOST_BOT_TOKEN" }
//! ```
//!
//! The `extra` table may carry `server_url` and `team_name` for future
//! adapters targeting a specific Mattermost server.
//!
//! # Future work
//!
//! Replace stub bodies with Mattermost REST API calls
//! (`POST /api/v4/posts`).  The `incoming` stream can be driven by the
//! Mattermost WebSocket API (`/api/v4/websocket`).

use std::pin::Pin;

use async_trait::async_trait;
use futures::Stream;
use roko_core::{Result, RokoError};

use crate::platforms::{
    ChannelId, ChatBridge, IncomingMessage, MessageId, PlatformCapabilities, PlatformConfig,
    RichMessage,
};

// ---------------------------------------------------------------------------
// MattermostBridge
// ---------------------------------------------------------------------------

/// Stub Mattermost adapter.
///
/// Supports Markdown and emoji reactions.  All outbound sends are logged at
/// `DEBUG` level; no real Mattermost API call is made.  Incoming messages are
/// an empty stream.
pub struct MattermostBridge {
    /// Resolved bot token (validated non-empty at construction).
    #[allow(dead_code)]
    token: String,
}

impl MattermostBridge {
    /// Construct and validate a [`MattermostBridge`] from a [`PlatformConfig`].
    ///
    /// Resolves the bot token (literal or env var) and returns an error if no
    /// token is configured or the env var is unset.
    pub fn connect(config: &PlatformConfig) -> Result<Self> {
        let token = config
            .token
            .as_ref()
            .ok_or_else(|| {
                RokoError::Invalid(format!(
                    "mattermost platform `{}` requires a token",
                    config.id
                ))
            })?
            .resolve()
            .map_err(|e| RokoError::Invalid(format!("mattermost `{}`: {e}", config.id)))?;

        if token.is_empty() {
            return Err(RokoError::Invalid(format!(
                "mattermost platform `{}` token must not be empty",
                config.id
            )));
        }

        Ok(Self { token })
    }
}

#[async_trait]
impl ChatBridge for MattermostBridge {
    fn platform_id(&self) -> &str {
        "mattermost"
    }

    async fn send_message(&self, channel: &str, text: &str) -> Result<MessageId> {
        tracing::debug!(
            platform = "mattermost",
            channel = channel,
            "stub send_message: {}",
            &text[..text.len().min(120)]
        );
        Ok(MessageId::new(format!("mattermost-stub-{}", ChannelId::new(channel))))
    }

    async fn send_rich(&self, channel: &str, msg: RichMessage) -> Result<MessageId> {
        let body = msg.markdown.as_deref().unwrap_or(&msg.text);
        tracing::debug!(
            platform = "mattermost",
            channel = channel,
            "stub send_rich: {}",
            &body[..body.len().min(120)]
        );
        Ok(MessageId::new(format!("mattermost-stub-rich-{}", ChannelId::new(channel))))
    }

    fn incoming(&self) -> Pin<Box<dyn Stream<Item = IncomingMessage> + Send>> {
        Box::pin(futures::stream::empty())
    }

    fn capabilities(&self) -> PlatformCapabilities {
        PlatformCapabilities {
            markdown: true,
            attachments: true,
            edit_messages: true,
            threads: false,
            interactive_components: false,
            reactions: true,
            max_message_length: Some(16_383),
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
            id: "mattermost-test".to_string(),
            kind: "mattermost".to_string(),
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
        assert!(MattermostBridge::connect(&config(None)).is_err());
    }

    #[test]
    fn connect_rejects_empty_token() {
        let cfg = config(Some(PlatformSecretRef::Literal(String::new())));
        assert!(MattermostBridge::connect(&cfg).is_err());
    }

    #[test]
    fn connect_succeeds_with_literal_token() {
        let cfg = config(Some(PlatformSecretRef::Literal("mm-bot-token".to_string())));
        assert!(MattermostBridge::connect(&cfg).is_ok());
    }

    #[test]
    fn platform_id_is_mattermost() {
        let bridge = MattermostBridge::connect(&config(Some(PlatformSecretRef::Literal(
            "tok".to_string(),
        ))))
        .unwrap();
        assert_eq!(bridge.platform_id(), "mattermost");
    }

    #[test]
    fn capabilities_match_mattermost() {
        let bridge = MattermostBridge::connect(&config(Some(PlatformSecretRef::Literal(
            "tok".to_string(),
        ))))
        .unwrap();
        let caps = bridge.capabilities();
        assert!(caps.markdown);
        assert!(caps.reactions);
        assert!(caps.attachments);
        assert!(!caps.threads);
        assert!(!caps.interactive_components);
        assert_eq!(caps.max_message_length, Some(16_383));
    }

    #[tokio::test]
    async fn send_message_returns_ok() {
        let bridge = MattermostBridge::connect(&config(Some(PlatformSecretRef::Literal(
            "tok".to_string(),
        ))))
        .unwrap();
        let id = bridge
            .send_message("town-square", "hello mattermost")
            .await
            .unwrap();
        assert!(id.as_str().contains("mattermost-stub"));
    }

    #[tokio::test]
    async fn send_rich_returns_ok() {
        let bridge = MattermostBridge::connect(&config(Some(PlatformSecretRef::Literal(
            "tok".to_string(),
        ))))
        .unwrap();
        let msg = RichMessage::markdown("hello", "**hello**");
        let id = bridge.send_rich("town-square", msg).await.unwrap();
        assert!(id.as_str().contains("mattermost-stub-rich"));
    }
}
