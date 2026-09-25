//! Chat platform abstraction layer (#414 + #224).
//!
//! [`ChatBridge`] is the universal interface for messaging-platform adapters.
//! Every adapter (Discord, Slack, Matrix, Telegram, …) implements this trait so
//! the rest of the runtime can send and receive messages without knowing which
//! wire protocol is in use.
//!
//! [`PlatformRegistry`] manages a keyed set of live [`ChatBridge`] instances
//! and exposes status snapshots used by the HTTP control plane.
//!
//! # Config
//!
//! Platforms are declared in `roko.toml` under an array of tables and parsed
//! into [`PlatformConfig`] (defined in `roko-core`):
//!
//! ```toml
//! [[platforms]]
//! id     = "discord-main"
//! kind   = "discord"
//! token  = { env = "DISCORD_TOKEN" }
//!
//! [[platforms]]
//! id     = "slack-ops"
//! kind   = "slack"
//! token  = { env = "SLACK_BOT_TOKEN" }
//! ```
//!
//! At serve startup, call [`PlatformRegistry::from_configs`] to pre-populate
//! the registry from the parsed config, then attach concrete bridges via
//! [`PlatformRegistry::register`].  Platform implementations live in separate
//! crates; this module owns only the contracts and registry.
//!
//! # Design principles
//!
//! - No domain types leak through this interface.  A `ChatBridge` knows only
//!   channels, text, and structured payloads.
//! - Capability negotiation is done at construction time via
//!   [`PlatformCapabilities`]; callers must not assume rich formatting is
//!   available.
//! - Unknown or unsupported platform `kind` values fail closed: the registry
//!   records a `Stub` entry with status `Unsupported` rather than silently
//!   ignoring the config.

use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use futures::Stream;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use roko_core::{Result, RokoError};
// Re-export the config types defined in roko-core so callers only need one
// import path for the full platform API surface.
pub use roko_core::config::platforms::{AgentIdentity, PlatformConfig, PlatformSecretRef};

// ---------------------------------------------------------------------------
// Primitive identifiers
// ---------------------------------------------------------------------------

/// Opaque message identifier returned by a successful send.
///
/// The encoding is platform-specific.  Callers must not parse this value;
/// store it for deduplication or threading only.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MessageId(pub String);

impl MessageId {
    /// Construct a `MessageId` from any string-like value.
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Return the inner string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for MessageId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Stable identifier for a channel, room, or conversation.
///
/// Semantics are platform-specific: on Discord this is a snowflake channel ID;
/// on Slack it is a channel name or ID (`#general`, `C01AB…`).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ChannelId(pub String);

impl ChannelId {
    /// Construct a `ChannelId` from any string-like value.
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Return the inner string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for ChannelId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

// ---------------------------------------------------------------------------
// Capability flags
// ---------------------------------------------------------------------------

/// Capabilities advertised by a platform adapter.
///
/// Callers should check the relevant flags before constructing rich payloads
/// to avoid silently degraded output.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlatformCapabilities {
    /// The platform renders Markdown text formatting.
    pub markdown: bool,
    /// The platform supports file or image attachments.
    pub attachments: bool,
    /// Messages can be edited after they are sent.
    pub edit_messages: bool,
    /// The platform supports threaded replies.
    pub threads: bool,
    /// The platform supports interactive UI components (buttons, menus).
    pub interactive_components: bool,
    /// The platform supports reactions / emoji responses.
    pub reactions: bool,
    /// Maximum message body length in Unicode code points.
    /// `None` means no known limit (treat as very large).
    pub max_message_length: Option<usize>,
}

impl Default for PlatformCapabilities {
    fn default() -> Self {
        Self {
            markdown: false,
            attachments: false,
            edit_messages: false,
            threads: false,
            interactive_components: false,
            reactions: false,
            max_message_length: None,
        }
    }
}

impl PlatformCapabilities {
    /// Minimal capability set — plain text only, no attachments or rich
    /// formatting.  Appropriate for unknown or stub adapters.
    pub fn minimal() -> Self {
        Self::default()
    }

    /// Rich capability set suitable for Slack/Discord-class platforms.
    pub fn rich() -> Self {
        Self {
            markdown: true,
            attachments: true,
            edit_messages: true,
            threads: true,
            interactive_components: true,
            reactions: true,
            max_message_length: Some(4_000),
        }
    }
}

// ---------------------------------------------------------------------------
// Rich message payload
// ---------------------------------------------------------------------------

/// Structured attachment included in a [`RichMessage`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageAttachment {
    /// MIME type of the attachment (e.g. `"text/plain"`, `"image/png"`).
    pub mime_type: String,
    /// Human-readable file name.
    pub file_name: String,
    /// Raw bytes — kept in memory for simplicity; stream if large.
    #[serde(skip)]
    pub data: Vec<u8>,
    /// Optional publicly accessible URL for platforms that prefer remote URLs.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

/// A structured outbound message that may carry rich formatting and attachments.
///
/// Adapters are expected to degrade gracefully: if a platform does not support
/// attachments, [`MessageAttachment`]s are silently dropped; if it does not
/// support Markdown, `text` is sent as-is.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RichMessage {
    /// Plain-text fallback always sent, even when rich formatting is available.
    pub text: String,
    /// Optional Markdown-formatted body (used when the platform supports it).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub markdown: Option<String>,
    /// Attachments to include.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub attachments: Vec<MessageAttachment>,
    /// Optional thread or parent message to reply to.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reply_to: Option<MessageId>,
    /// Optional emoji reaction to attach immediately after send (platform
    /// capability check required before use).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub react_with: Option<String>,
}

impl RichMessage {
    /// Construct a plain-text message.
    pub fn plain(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            markdown: None,
            attachments: Vec::new(),
            reply_to: None,
            react_with: None,
        }
    }

    /// Construct a message with Markdown formatting.
    pub fn markdown(text: impl Into<String>, md: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            markdown: Some(md.into()),
            attachments: Vec::new(),
            reply_to: None,
            react_with: None,
        }
    }
}

// ---------------------------------------------------------------------------
// Incoming message
// ---------------------------------------------------------------------------

/// A message received from the platform.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IncomingMessage {
    /// Platform-assigned message identifier.
    pub id: MessageId,
    /// Channel or room the message arrived in.
    pub channel: ChannelId,
    /// Sender identifier (platform-specific format).
    pub sender_id: String,
    /// Display name of the sender, if available.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sender_name: Option<String>,
    /// Plain-text message content.
    pub text: String,
    /// Raw platform-specific payload, if the adapter wishes to expose it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw: Option<serde_json::Value>,
    /// Wall-clock timestamp when the message was created on the platform.
    pub timestamp: DateTime<Utc>,
}

// ---------------------------------------------------------------------------
// ChatBridge trait
// ---------------------------------------------------------------------------

/// Universal interface for all messaging platform adapters.
///
/// Each concrete adapter (Discord, Slack, Matrix, …) implements this trait.
/// The runtime talks to platforms exclusively through this interface, so
/// swapping or adding adapters never requires changes outside the adapter crate.
///
/// # Error handling
///
/// Methods return [`RokoError`] so adapters integrate naturally with the kernel
/// error model.  Transient errors (rate limits, network hiccups) should map to
/// [`RokoError::Transport`]; permanent failures (bad credentials, missing
/// channels) should map to [`RokoError::Invalid`].
///
/// # Cancellation
///
/// Implementations must respect Tokio cancellation — futures returned from
/// async methods must yield promptly when dropped.
#[async_trait]
pub trait ChatBridge: Send + Sync {
    /// Stable platform identifier string (e.g. `"discord"`, `"slack"`, `"matrix"`).
    ///
    /// Must be lowercase ASCII with optional hyphens.  Used as a metrics label
    /// and log prefix; must be unique within a [`PlatformRegistry`].
    fn platform_id(&self) -> &str;

    /// Send a plain-text message to a channel or room.
    ///
    /// Returns the opaque platform-assigned [`MessageId`] on success.
    async fn send_message(&self, channel: &str, text: &str) -> Result<MessageId>;

    /// Send a structured message with optional rich formatting and attachments.
    ///
    /// Adapters should degrade gracefully: strip unsupported fields and fall
    /// back to the plain `text` field rather than returning an error.
    async fn send_rich(&self, channel: &str, msg: RichMessage) -> Result<MessageId>;

    /// Return a stream of messages received from the platform.
    ///
    /// The stream should run until the underlying connection is closed or the
    /// platform is unregistered.  Callers are responsible for driving and
    /// dropping the stream.
    fn incoming(&self) -> Pin<Box<dyn Stream<Item = IncomingMessage> + Send>>;

    /// Report the capabilities available on this platform instance.
    fn capabilities(&self) -> PlatformCapabilities;
}

// ---------------------------------------------------------------------------
// Registry status types
// ---------------------------------------------------------------------------

/// Connection state for a registered platform.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlatformStatus {
    /// The bridge is connected and healthy.
    Connected,
    /// The bridge has not yet been started.
    Pending,
    /// The bridge encountered an error and is not processing messages.
    Degraded,
    /// The platform `kind` has no registered adapter factory.
    Unsupported,
    /// The connection was shut down cleanly.
    Disconnected,
}

/// A read-only snapshot of a registered platform, safe to return via the API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformSnapshot {
    /// User-chosen platform connection ID.
    pub id: String,
    /// Platform kind string.
    pub kind: String,
    /// Current connection status.
    pub status: PlatformStatus,
    /// Human-readable description.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Advertised capability flags.
    pub capabilities: PlatformCapabilities,
    /// Whether the platform is marked enabled in config.
    pub enabled: bool,
    /// Timestamp when the entry was last updated.
    pub updated_at: DateTime<Utc>,
}

// ---------------------------------------------------------------------------
// Registry
// ---------------------------------------------------------------------------

/// Entry stored in the registry.
struct RegistryEntry {
    bridge: Option<Arc<dyn ChatBridge>>,
    status: PlatformStatus,
    config: PlatformConfig,
    updated_at: DateTime<Utc>,
}

/// Manages multiple [`ChatBridge`] instances indexed by their config `id`.
///
/// The registry is the single access point for the serve layer.  It is
/// cheaply cloneable via `Arc`.
///
/// # Usage
///
/// ```rust,ignore
/// let registry = PlatformRegistry::default();
/// registry.register(config, Some(Arc::new(my_bridge))).await;
/// let snapshot = registry.snapshot("discord-main").await;
/// ```
pub struct PlatformRegistry {
    entries: RwLock<HashMap<String, RegistryEntry>>,
}

impl Default for PlatformRegistry {
    fn default() -> Self {
        Self {
            entries: RwLock::new(HashMap::new()),
        }
    }
}

/// A thread-safe shared reference to a [`PlatformRegistry`].
pub type SharedPlatformRegistry = Arc<PlatformRegistry>;

impl PlatformRegistry {
    /// Construct an empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Construct a shared (Arc-wrapped) empty registry.
    pub fn shared() -> SharedPlatformRegistry {
        Arc::new(Self::new())
    }

    /// Build a registry from a slice of [`PlatformConfig`] entries.
    ///
    /// Each entry is registered with no bridge (`None`) and status `Pending`
    /// (if enabled) or `Disconnected` (if disabled).  Callers that have
    /// concrete adapter factories should call [`register`](Self::register)
    /// afterwards to attach live bridges.
    pub fn from_configs(configs: &[PlatformConfig]) -> Self {
        let registry = Self::new();
        {
            let mut entries = registry.entries.try_write().expect("no contention at init");
            for cfg in configs {
                let status = if cfg.enabled {
                    PlatformStatus::Pending
                } else {
                    PlatformStatus::Disconnected
                };
                entries.insert(
                    cfg.id.clone(),
                    RegistryEntry {
                        bridge: None,
                        status,
                        config: cfg.clone(),
                        updated_at: Utc::now(),
                    },
                );
            }
        } // guard dropped here
        registry
    }

    /// Register or replace a platform connection.
    ///
    /// If `bridge` is `Some` the status is set to `Connected`.
    /// If `bridge` is `None` the status is set to `Pending`.
    pub async fn register(&self, config: PlatformConfig, bridge: Option<Arc<dyn ChatBridge>>) {
        let status = if bridge.is_some() {
            PlatformStatus::Connected
        } else {
            PlatformStatus::Pending
        };
        let mut entries = self.entries.write().await;
        entries.insert(
            config.id.clone(),
            RegistryEntry {
                bridge,
                status,
                config,
                updated_at: Utc::now(),
            },
        );
    }

    /// Mark a platform as `Unsupported` (its `kind` has no registered factory).
    pub async fn mark_unsupported(&self, id: &str) {
        let mut entries = self.entries.write().await;
        if let Some(entry) = entries.get_mut(id) {
            entry.status = PlatformStatus::Unsupported;
            entry.updated_at = Utc::now();
        }
    }

    /// Mark a platform as `Degraded`.
    pub async fn mark_degraded(&self, id: &str) {
        let mut entries = self.entries.write().await;
        if let Some(entry) = entries.get_mut(id) {
            entry.status = PlatformStatus::Degraded;
            entry.updated_at = Utc::now();
        }
    }

    /// Mark a platform as cleanly disconnected.
    pub async fn mark_disconnected(&self, id: &str) {
        let mut entries = self.entries.write().await;
        if let Some(entry) = entries.get_mut(id) {
            entry.status = PlatformStatus::Disconnected;
            entry.bridge = None;
            entry.updated_at = Utc::now();
        }
    }

    /// Remove a platform entry from the registry.
    pub async fn remove(&self, id: &str) {
        self.entries.write().await.remove(id);
    }

    /// Look up a live bridge by platform connection ID.
    pub async fn get_bridge(&self, id: &str) -> Option<Arc<dyn ChatBridge>> {
        self.entries
            .read()
            .await
            .get(id)
            .and_then(|e| e.bridge.clone())
    }

    /// Return a status snapshot for a single platform entry.
    pub async fn snapshot(&self, id: &str) -> Option<PlatformSnapshot> {
        self.entries.read().await.get(id).map(entry_to_snapshot)
    }

    /// Return status snapshots for all registered platforms.
    pub async fn list_snapshots(&self) -> Vec<PlatformSnapshot> {
        self.entries
            .read()
            .await
            .values()
            .map(entry_to_snapshot)
            .collect()
    }

    /// Return the number of registered platforms.
    pub async fn len(&self) -> usize {
        self.entries.read().await.len()
    }

    /// Return `true` if no platforms are registered.
    pub async fn is_empty(&self) -> bool {
        self.entries.read().await.is_empty()
    }
}

fn entry_to_snapshot(entry: &RegistryEntry) -> PlatformSnapshot {
    let capabilities = entry
        .bridge
        .as_ref()
        .map(|b| b.capabilities())
        .unwrap_or_default();
    PlatformSnapshot {
        id: entry.config.id.clone(),
        kind: entry.config.kind.clone(),
        status: entry.status,
        description: entry.config.description.clone(),
        capabilities,
        enabled: entry.config.enabled,
        updated_at: entry.updated_at,
    }
}

// ---------------------------------------------------------------------------
// Stub bridge (for testing and unsupported kinds)
// ---------------------------------------------------------------------------

/// A no-op [`ChatBridge`] that records calls but never delivers messages.
///
/// Used by the registry for unsupported platform `kind` values and in tests.
pub struct StubChatBridge {
    id: String,
}

impl StubChatBridge {
    /// Construct a stub with the given platform ID.
    pub fn new(id: impl Into<String>) -> Self {
        Self { id: id.into() }
    }
}

#[async_trait]
impl ChatBridge for StubChatBridge {
    fn platform_id(&self) -> &str {
        &self.id
    }

    async fn send_message(&self, _channel: &str, _text: &str) -> Result<MessageId> {
        Err(RokoError::Transport(format!(
            "platform `{}` is a stub and cannot send messages",
            self.id
        )))
    }

    async fn send_rich(&self, _channel: &str, _msg: RichMessage) -> Result<MessageId> {
        Err(RokoError::Transport(format!(
            "platform `{}` is a stub and cannot send messages",
            self.id
        )))
    }

    fn incoming(&self) -> Pin<Box<dyn Stream<Item = IncomingMessage> + Send>> {
        Box::pin(futures::stream::empty())
    }

    fn capabilities(&self) -> PlatformCapabilities {
        PlatformCapabilities::minimal()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn discord_config() -> PlatformConfig {
        PlatformConfig {
            id: "discord-main".to_string(),
            kind: "discord".to_string(),
            token: Some(PlatformSecretRef::Literal("tok".to_string())),
            description: Some("Main Discord server".to_string()),
            enabled: true,
            max_retries: 3,
            retry_delay_ms: 1_000,
            extra: HashMap::new(),
            identity: None,
        }
    }

    fn slack_config() -> PlatformConfig {
        PlatformConfig {
            id: "slack-ops".to_string(),
            kind: "slack".to_string(),
            token: Some(PlatformSecretRef::Env {
                env: "SLACK_BOT_TOKEN".to_string(),
            }),
            description: None,
            enabled: false,
            max_retries: 3,
            retry_delay_ms: 1_000,
            extra: HashMap::new(),
            identity: None,
        }
    }

    #[test]
    fn message_id_display() {
        let id = MessageId::new("123456");
        assert_eq!(id.to_string(), "123456");
    }

    #[test]
    fn channel_id_display() {
        let ch = ChannelId::new("#general");
        assert_eq!(ch.to_string(), "#general");
    }

    #[test]
    fn rich_message_plain_fallback() {
        let msg = RichMessage::plain("hello world");
        assert_eq!(msg.text, "hello world");
        assert!(msg.markdown.is_none());
        assert!(msg.attachments.is_empty());
    }

    #[test]
    fn rich_message_with_markdown() {
        let msg = RichMessage::markdown("hello", "**hello**");
        assert_eq!(msg.text, "hello");
        assert_eq!(msg.markdown.as_deref(), Some("**hello**"));
    }

    #[test]
    fn capabilities_defaults() {
        let caps = PlatformCapabilities::default();
        assert!(!caps.markdown);
        assert!(!caps.attachments);
        assert!(!caps.threads);
    }

    #[test]
    fn capabilities_rich() {
        let caps = PlatformCapabilities::rich();
        assert!(caps.markdown);
        assert!(caps.attachments);
        assert!(caps.threads);
        assert_eq!(caps.max_message_length, Some(4_000));
    }

    #[tokio::test]
    async fn registry_from_configs_sets_pending_and_disconnected() {
        let cfgs = vec![discord_config(), slack_config()];
        let registry = PlatformRegistry::from_configs(&cfgs);

        let discord = registry.snapshot("discord-main").await.unwrap();
        assert_eq!(discord.status, PlatformStatus::Pending);
        assert!(discord.enabled);

        let slack = registry.snapshot("slack-ops").await.unwrap();
        assert_eq!(slack.status, PlatformStatus::Disconnected);
        assert!(!slack.enabled);
    }

    #[tokio::test]
    async fn registry_register_bridge_marks_connected() {
        let registry = PlatformRegistry::new();
        let cfg = discord_config();
        let bridge: Arc<dyn ChatBridge> = Arc::new(StubChatBridge::new("discord"));
        registry.register(cfg.clone(), Some(bridge)).await;

        let snap = registry.snapshot("discord-main").await.unwrap();
        assert_eq!(snap.status, PlatformStatus::Connected);
        assert_eq!(snap.kind, "discord");
    }

    #[tokio::test]
    async fn registry_register_no_bridge_marks_pending() {
        let registry = PlatformRegistry::new();
        let cfg = discord_config();
        registry.register(cfg, None).await;

        let snap = registry.snapshot("discord-main").await.unwrap();
        assert_eq!(snap.status, PlatformStatus::Pending);
    }

    #[tokio::test]
    async fn registry_mark_degraded() {
        let registry = PlatformRegistry::from_configs(&[discord_config()]);
        registry.mark_degraded("discord-main").await;
        let snap = registry.snapshot("discord-main").await.unwrap();
        assert_eq!(snap.status, PlatformStatus::Degraded);
    }

    #[tokio::test]
    async fn registry_mark_unsupported() {
        let registry = PlatformRegistry::from_configs(&[discord_config()]);
        registry.mark_unsupported("discord-main").await;
        let snap = registry.snapshot("discord-main").await.unwrap();
        assert_eq!(snap.status, PlatformStatus::Unsupported);
    }

    #[tokio::test]
    async fn registry_mark_disconnected() {
        let registry = PlatformRegistry::new();
        let cfg = discord_config();
        let bridge: Arc<dyn ChatBridge> = Arc::new(StubChatBridge::new("discord"));
        registry.register(cfg, Some(bridge)).await;
        registry.mark_disconnected("discord-main").await;
        let snap = registry.snapshot("discord-main").await.unwrap();
        assert_eq!(snap.status, PlatformStatus::Disconnected);
        // bridge should be cleared
        assert!(registry.get_bridge("discord-main").await.is_none());
    }

    #[tokio::test]
    async fn registry_remove() {
        let registry = PlatformRegistry::from_configs(&[discord_config()]);
        registry.remove("discord-main").await;
        assert!(registry.snapshot("discord-main").await.is_none());
    }

    #[tokio::test]
    async fn registry_list_snapshots() {
        let cfgs = vec![discord_config(), slack_config()];
        let registry = PlatformRegistry::from_configs(&cfgs);
        let snaps = registry.list_snapshots().await;
        assert_eq!(snaps.len(), 2);
    }

    #[tokio::test]
    async fn registry_len_and_is_empty() {
        let registry = PlatformRegistry::new();
        assert!(registry.is_empty().await);
        assert_eq!(registry.len().await, 0);
        registry
            .register(discord_config(), None)
            .await;
        assert!(!registry.is_empty().await);
        assert_eq!(registry.len().await, 1);
    }

    #[tokio::test]
    async fn stub_bridge_send_returns_error() {
        let bridge = StubChatBridge::new("test-stub");
        assert!(bridge.send_message("#general", "hello").await.is_err());
        assert!(bridge
            .send_rich("#general", RichMessage::plain("hi"))
            .await
            .is_err());
    }

    #[tokio::test]
    async fn stub_bridge_capabilities_are_minimal() {
        let bridge = StubChatBridge::new("test-stub");
        let caps = bridge.capabilities();
        assert!(!caps.markdown);
        assert!(!caps.attachments);
    }

    #[tokio::test]
    async fn stub_bridge_incoming_is_empty() {
        use futures::StreamExt;
        let bridge = StubChatBridge::new("test-stub");
        let mut stream = bridge.incoming();
        // Fused empty stream returns None immediately.
        assert!(stream.next().await.is_none());
    }
}
