//! Stub Matrix [`ChatBridge`] adapter (#411).
//!
//! Matrix (the open federated protocol) supports Markdown, reactions, and
//! threaded replies.  It does not natively support interactive components.
//!
//! # Config
//!
//! ```toml
//! [[platforms]]
//! id    = "matrix-main"
//! kind  = "matrix"
//! token = { env = "MATRIX_ACCESS_TOKEN" }
//! ```
//!
//! The `extra` table may carry additional Matrix-specific settings such as
//! `homeserver_url` and `room_id` for future adapters.
//!
//! # Future work
//!
//! Replace stub bodies with Matrix Client-Server API calls
//! (`PUT /_matrix/client/v3/rooms/{roomId}/send/{eventType}/{txnId}`).
//! The `incoming` stream should be driven by `/sync` long-polling.

use std::pin::Pin;

use async_trait::async_trait;
use futures::Stream;
use roko_core::{Result, RokoError};

use crate::platforms::{
    ChannelId, ChatBridge, IncomingMessage, MessageId, PlatformCapabilities, PlatformConfig,
    RichMessage,
};

// ---------------------------------------------------------------------------
// MatrixBridge
// ---------------------------------------------------------------------------

/// Stub Matrix adapter.
///
/// Supports Markdown (via `m.text` with `format: org.matrix.custom.html`),
/// reactions, and threads.  All outbound sends are logged at `DEBUG` level;
/// no real Matrix API call is made.  Incoming messages are an empty stream.
pub struct MatrixBridge {
    /// Resolved access token (validated non-empty at construction).
    #[allow(dead_code)]
    token: String,
}

impl MatrixBridge {
    /// Construct and validate a [`MatrixBridge`] from a [`PlatformConfig`].
    ///
    /// Resolves the access token (literal or env var) and returns an error if
    /// no token is configured or the env var is unset.
    pub fn connect(config: &PlatformConfig) -> Result<Self> {
        let token = config
            .token
            .as_ref()
            .ok_or_else(|| {
                RokoError::Invalid(format!(
                    "matrix platform `{}` requires a token",
                    config.id
                ))
            })?
            .resolve()
            .map_err(|e| RokoError::Invalid(format!("matrix `{}`: {e}", config.id)))?;

        if token.is_empty() {
            return Err(RokoError::Invalid(format!(
                "matrix platform `{}` token must not be empty",
                config.id
            )));
        }

        Ok(Self { token })
    }
}

#[async_trait]
impl ChatBridge for MatrixBridge {
    fn platform_id(&self) -> &str {
        "matrix"
    }

    async fn send_message(&self, channel: &str, text: &str) -> Result<MessageId> {
        tracing::debug!(
            platform = "matrix",
            channel = channel,
            "stub send_message: {}",
            &text[..text.len().min(120)]
        );
        Ok(MessageId::new(format!("matrix-stub-{}", ChannelId::new(channel))))
    }

    async fn send_rich(&self, channel: &str, msg: RichMessage) -> Result<MessageId> {
        let body = msg.markdown.as_deref().unwrap_or(&msg.text);
        tracing::debug!(
            platform = "matrix",
            channel = channel,
            "stub send_rich: {}",
            &body[..body.len().min(120)]
        );
        Ok(MessageId::new(format!("matrix-stub-rich-{}", ChannelId::new(channel))))
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
            interactive_components: false,
            reactions: true,
            max_message_length: None,
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
            id: "matrix-test".to_string(),
            kind: "matrix".to_string(),
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
        assert!(MatrixBridge::connect(&config(None)).is_err());
    }

    #[test]
    fn connect_rejects_empty_token() {
        let cfg = config(Some(PlatformSecretRef::Literal(String::new())));
        assert!(MatrixBridge::connect(&cfg).is_err());
    }

    #[test]
    fn connect_succeeds_with_literal_token() {
        let cfg = config(Some(PlatformSecretRef::Literal("syt_abc123".to_string())));
        assert!(MatrixBridge::connect(&cfg).is_ok());
    }

    #[test]
    fn platform_id_is_matrix() {
        let bridge = MatrixBridge::connect(&config(Some(PlatformSecretRef::Literal(
            "tok".to_string(),
        ))))
        .unwrap();
        assert_eq!(bridge.platform_id(), "matrix");
    }

    #[test]
    fn capabilities_match_matrix() {
        let bridge = MatrixBridge::connect(&config(Some(PlatformSecretRef::Literal(
            "tok".to_string(),
        ))))
        .unwrap();
        let caps = bridge.capabilities();
        assert!(caps.markdown);
        assert!(caps.attachments);
        assert!(caps.threads);
        assert!(caps.reactions);
        assert!(!caps.interactive_components);
        assert!(caps.max_message_length.is_none());
    }

    #[tokio::test]
    async fn send_message_returns_ok() {
        let bridge = MatrixBridge::connect(&config(Some(PlatformSecretRef::Literal(
            "tok".to_string(),
        ))))
        .unwrap();
        let id = bridge
            .send_message("!room:example.org", "hello matrix")
            .await
            .unwrap();
        assert!(id.as_str().contains("matrix-stub"));
    }

    #[tokio::test]
    async fn send_rich_returns_ok() {
        let bridge = MatrixBridge::connect(&config(Some(PlatformSecretRef::Literal(
            "tok".to_string(),
        ))))
        .unwrap();
        let msg = RichMessage::markdown("hello", "**hello**");
        let id = bridge
            .send_rich("!room:example.org", msg)
            .await
            .unwrap();
        assert!(id.as_str().contains("matrix-stub-rich"));
    }
}
