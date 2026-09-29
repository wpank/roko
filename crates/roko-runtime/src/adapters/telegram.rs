//! Stub Telegram [`ChatBridge`] adapter (#225).
//!
//! Telegram supports Markdown (via `MarkdownV2` or `HTML` parse modes) and
//! has a 4 096-character message limit.  It does not support reactions or
//! interactive components at the Bot API level advertised here.
//!
//! # Config
//!
//! ```toml
//! [[platforms]]
//! id    = "telegram-alerts"
//! kind  = "telegram"
//! token = { env = "TELEGRAM_BOT_TOKEN" }
//! ```
//!
//! The `extra` table may carry `chat_id` as a default destination for future
//! adapters.
//!
//! # Future work
//!
//! Replace stub bodies with Telegram Bot API calls
//! (`POST https://api.telegram.org/bot{token}/sendMessage`).  The `incoming`
//! stream can be driven by long-polling (`getUpdates`) or webhook delivery.

use std::pin::Pin;

use async_trait::async_trait;
use futures::Stream;
use roko_core::{Result, RokoError};

use crate::platforms::{
    ChannelId, ChatBridge, IncomingMessage, MessageId, PlatformCapabilities, PlatformConfig,
    RichMessage,
};

/// Maximum message body length enforced by the Telegram Bot API.
pub const TELEGRAM_MAX_MESSAGE_LEN: usize = 4_096;

// ---------------------------------------------------------------------------
// TelegramBridge
// ---------------------------------------------------------------------------

/// Stub Telegram Bot adapter.
///
/// Supports Markdown (MarkdownV2 parse mode) and the 4 096-character limit.
/// All outbound sends are logged at `DEBUG` level; no real Telegram API call
/// is made.  Incoming messages are an empty stream.
pub struct TelegramBridge {
    /// Resolved bot token (validated non-empty at construction).
    #[allow(dead_code)]
    token: String,
}

impl TelegramBridge {
    /// Construct and validate a [`TelegramBridge`] from a [`PlatformConfig`].
    ///
    /// Resolves the bot token (literal or env var) and returns an error if no
    /// token is configured or the env var is unset.
    pub fn connect(config: &PlatformConfig) -> Result<Self> {
        let token = config
            .token
            .as_ref()
            .ok_or_else(|| {
                RokoError::Invalid(format!(
                    "telegram platform `{}` requires a token",
                    config.id
                ))
            })?
            .resolve()
            .map_err(|e| RokoError::Invalid(format!("telegram `{}`: {e}", config.id)))?;

        if token.is_empty() {
            return Err(RokoError::Invalid(format!(
                "telegram platform `{}` token must not be empty",
                config.id
            )));
        }

        Ok(Self { token })
    }
}

#[async_trait]
impl ChatBridge for TelegramBridge {
    fn platform_id(&self) -> &str {
        "telegram"
    }

    async fn send_message(&self, channel: &str, text: &str) -> Result<MessageId> {
        let preview_len = text.len().min(120);
        tracing::debug!(
            platform = "telegram",
            channel = channel,
            "stub send_message: {}",
            &text[..preview_len]
        );
        Ok(MessageId::new(format!("telegram-stub-{}", ChannelId::new(channel))))
    }

    async fn send_rich(&self, channel: &str, msg: RichMessage) -> Result<MessageId> {
        let body = msg.markdown.as_deref().unwrap_or(&msg.text);
        let preview_len = body.len().min(120);
        tracing::debug!(
            platform = "telegram",
            channel = channel,
            "stub send_rich: {}",
            &body[..preview_len]
        );
        Ok(MessageId::new(format!("telegram-stub-rich-{}", ChannelId::new(channel))))
    }

    fn incoming(&self) -> Pin<Box<dyn Stream<Item = IncomingMessage> + Send>> {
        Box::pin(futures::stream::empty())
    }

    fn capabilities(&self) -> PlatformCapabilities {
        PlatformCapabilities {
            markdown: true,
            attachments: false,
            edit_messages: true,
            threads: false,
            interactive_components: false,
            reactions: false,
            max_message_length: Some(TELEGRAM_MAX_MESSAGE_LEN),
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
            id: "telegram-test".to_string(),
            kind: "telegram".to_string(),
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
        assert!(TelegramBridge::connect(&config(None)).is_err());
    }

    #[test]
    fn connect_rejects_empty_token() {
        let cfg = config(Some(PlatformSecretRef::Literal(String::new())));
        assert!(TelegramBridge::connect(&cfg).is_err());
    }

    #[test]
    fn connect_succeeds_with_literal_token() {
        let cfg = config(Some(PlatformSecretRef::Literal(
            "123456:ABC-DEF".to_string(),
        )));
        assert!(TelegramBridge::connect(&cfg).is_ok());
    }

    #[test]
    fn platform_id_is_telegram() {
        let bridge = TelegramBridge::connect(&config(Some(PlatformSecretRef::Literal(
            "tok".to_string(),
        ))))
        .unwrap();
        assert_eq!(bridge.platform_id(), "telegram");
    }

    #[test]
    fn capabilities_match_telegram() {
        let bridge = TelegramBridge::connect(&config(Some(PlatformSecretRef::Literal(
            "tok".to_string(),
        ))))
        .unwrap();
        let caps = bridge.capabilities();
        assert!(caps.markdown);
        assert!(!caps.attachments);
        assert!(!caps.reactions);
        assert!(!caps.threads);
        assert_eq!(caps.max_message_length, Some(TELEGRAM_MAX_MESSAGE_LEN));
    }

    #[tokio::test]
    async fn send_message_returns_ok() {
        let bridge = TelegramBridge::connect(&config(Some(PlatformSecretRef::Literal(
            "tok".to_string(),
        ))))
        .unwrap();
        let id = bridge
            .send_message("-100123456789", "hello telegram")
            .await
            .unwrap();
        assert!(id.as_str().contains("telegram-stub"));
    }

    #[tokio::test]
    async fn send_rich_returns_ok() {
        let bridge = TelegramBridge::connect(&config(Some(PlatformSecretRef::Literal(
            "tok".to_string(),
        ))))
        .unwrap();
        let msg = RichMessage::markdown("hello", "**hello**");
        let id = bridge
            .send_rich("-100123456789", msg)
            .await
            .unwrap();
        assert!(id.as_str().contains("telegram-stub-rich"));
    }
}
