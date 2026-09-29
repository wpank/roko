//! Stub WhatsApp [`ChatBridge`] adapter (#412).
//!
//! WhatsApp Business API supports plain text messages only (no Markdown
//! rendering in most clients).  Messages are capped at 4 096 characters.
//! Reactions, threads, and interactive components are not advertised by this
//! stub.
//!
//! # Config
//!
//! ```toml
//! [[platforms]]
//! id    = "whatsapp-support"
//! kind  = "whatsapp"
//! token = { env = "WHATSAPP_API_TOKEN" }
//! ```
//!
//! The `extra` table may carry `phone_number_id` or `waba_id` for future
//! adapters targeting the WhatsApp Business Cloud API.
//!
//! # Future work
//!
//! Replace stub bodies with WhatsApp Business Cloud API calls
//! (`POST /{phone-number-id}/messages`).  The `incoming` stream should
//! be driven by a webhook subscription registered with Meta's Graph API.

use std::pin::Pin;

use async_trait::async_trait;
use futures::Stream;
use roko_core::{Result, RokoError};

use crate::platforms::{
    ChannelId, ChatBridge, IncomingMessage, MessageId, PlatformCapabilities, PlatformConfig,
    RichMessage,
};

/// Maximum message body length enforced by WhatsApp Business API.
pub const WHATSAPP_MAX_MESSAGE_LEN: usize = 4_096;

// ---------------------------------------------------------------------------
// WhatsAppBridge
// ---------------------------------------------------------------------------

/// Stub WhatsApp Business adapter.
///
/// Plain text only; messages over 4 096 characters are truncated with a
/// `…` suffix in the stub log line (real adapters must split or reject them).
/// All outbound sends are logged at `DEBUG` level; no real API call is made.
/// Incoming messages are an empty stream.
pub struct WhatsAppBridge {
    /// Resolved API token (validated non-empty at construction).
    #[allow(dead_code)]
    token: String,
}

impl WhatsAppBridge {
    /// Construct and validate a [`WhatsAppBridge`] from a [`PlatformConfig`].
    ///
    /// Resolves the API token (literal or env var) and returns an error if no
    /// token is configured or the env var is unset.
    pub fn connect(config: &PlatformConfig) -> Result<Self> {
        let token = config
            .token
            .as_ref()
            .ok_or_else(|| {
                RokoError::Invalid(format!(
                    "whatsapp platform `{}` requires a token",
                    config.id
                ))
            })?
            .resolve()
            .map_err(|e| RokoError::Invalid(format!("whatsapp `{}`: {e}", config.id)))?;

        if token.is_empty() {
            return Err(RokoError::Invalid(format!(
                "whatsapp platform `{}` token must not be empty",
                config.id
            )));
        }

        Ok(Self { token })
    }
}

#[async_trait]
impl ChatBridge for WhatsAppBridge {
    fn platform_id(&self) -> &str {
        "whatsapp"
    }

    async fn send_message(&self, channel: &str, text: &str) -> Result<MessageId> {
        let preview = &text[..text.len().min(120)];
        tracing::debug!(
            platform = "whatsapp",
            channel = channel,
            "stub send_message: {}",
            preview
        );
        Ok(MessageId::new(format!("whatsapp-stub-{}", ChannelId::new(channel))))
    }

    async fn send_rich(&self, channel: &str, msg: RichMessage) -> Result<MessageId> {
        // WhatsApp has no rich formatting — always use the plain text field.
        let text = &msg.text;
        let preview = &text[..text.len().min(120)];
        tracing::debug!(
            platform = "whatsapp",
            channel = channel,
            "stub send_rich (plain fallback): {}",
            preview
        );
        Ok(MessageId::new(format!("whatsapp-stub-rich-{}", ChannelId::new(channel))))
    }

    fn incoming(&self) -> Pin<Box<dyn Stream<Item = IncomingMessage> + Send>> {
        Box::pin(futures::stream::empty())
    }

    fn capabilities(&self) -> PlatformCapabilities {
        PlatformCapabilities {
            markdown: false,
            attachments: false,
            edit_messages: false,
            threads: false,
            interactive_components: false,
            reactions: false,
            max_message_length: Some(WHATSAPP_MAX_MESSAGE_LEN),
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
            id: "whatsapp-test".to_string(),
            kind: "whatsapp".to_string(),
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
        assert!(WhatsAppBridge::connect(&config(None)).is_err());
    }

    #[test]
    fn connect_rejects_empty_token() {
        let cfg = config(Some(PlatformSecretRef::Literal(String::new())));
        assert!(WhatsAppBridge::connect(&cfg).is_err());
    }

    #[test]
    fn connect_succeeds_with_literal_token() {
        let cfg = config(Some(PlatformSecretRef::Literal("EAABs…token".to_string())));
        assert!(WhatsAppBridge::connect(&cfg).is_ok());
    }

    #[test]
    fn platform_id_is_whatsapp() {
        let bridge = WhatsAppBridge::connect(&config(Some(PlatformSecretRef::Literal(
            "tok".to_string(),
        ))))
        .unwrap();
        assert_eq!(bridge.platform_id(), "whatsapp");
    }

    #[test]
    fn capabilities_are_plain_text_only() {
        let bridge = WhatsAppBridge::connect(&config(Some(PlatformSecretRef::Literal(
            "tok".to_string(),
        ))))
        .unwrap();
        let caps = bridge.capabilities();
        assert!(!caps.markdown);
        assert!(!caps.attachments);
        assert!(!caps.reactions);
        assert!(!caps.threads);
        assert_eq!(caps.max_message_length, Some(WHATSAPP_MAX_MESSAGE_LEN));
    }

    #[tokio::test]
    async fn send_message_returns_ok() {
        let bridge = WhatsAppBridge::connect(&config(Some(PlatformSecretRef::Literal(
            "tok".to_string(),
        ))))
        .unwrap();
        let id = bridge
            .send_message("+15551234567", "hello whatsapp")
            .await
            .unwrap();
        assert!(id.as_str().contains("whatsapp-stub"));
    }

    #[tokio::test]
    async fn send_rich_uses_plain_text_fallback() {
        let bridge = WhatsAppBridge::connect(&config(Some(PlatformSecretRef::Literal(
            "tok".to_string(),
        ))))
        .unwrap();
        let msg = RichMessage::markdown("plain text", "**ignored markdown**");
        let id = bridge.send_rich("+15551234567", msg).await.unwrap();
        assert!(id.as_str().contains("whatsapp-stub-rich"));
    }
}
