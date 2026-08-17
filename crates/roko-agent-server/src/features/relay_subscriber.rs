//! Thin pub/sub wrapper around [`RelayHandle`].
//!
//! [`RelaySubscriber`] provides a higher-level interface over the raw
//! subscribe/unsubscribe/publish methods on [`RelayHandle`].  Callers can
//! bundle a handle + their topic subscriptions into a single value that is
//! easy to pass around without exposing the full relay-client API.

use std::sync::Arc;

use anyhow::{Result, anyhow};
use async_trait::async_trait;
use tokio::sync::{mpsc, oneshot};

use super::relay_client::{RelayHandle, TopicHandler};

/// A received topic message.
#[derive(Debug)]
pub struct TopicMessage {
    /// The topic the message arrived on.
    pub topic: String,
    /// Application-defined message type discriminant.
    pub msg_type: String,
    /// Arbitrary JSON payload.
    pub payload: serde_json::Value,
    /// Agent ID of the publisher, if the relay provided one.
    pub publisher_id: Option<String>,
    /// Monotonically increasing sequence number assigned by the relay bus.
    pub seq: u64,
    commit_tx: Option<oneshot::Sender<Result<()>>>,
}

impl TopicMessage {
    /// Report that this message has been durably committed. The relay client
    /// emits its consumer ACK only after this method succeeds.
    pub fn commit(mut self) -> Result<()> {
        self.commit_tx
            .take()
            .ok_or_else(|| anyhow!("relay topic message already completed"))?
            .send(Ok(()))
            .map_err(|_| anyhow!("relay durable handler stopped"))
    }

    /// Reject processing so the client reconnects from its previous cursor.
    pub fn reject(mut self, error: impl Into<anyhow::Error>) -> Result<()> {
        self.commit_tx
            .take()
            .ok_or_else(|| anyhow!("relay topic message already completed"))?
            .send(Err(error.into()))
            .map_err(|_| anyhow!("relay durable handler stopped"))
    }
}

/// Receives topic messages through an mpsc channel and forwards them to a
/// caller-supplied bounded [`mpsc::Sender<TopicMessage>`].
struct ChannelTopicHandler {
    tx: mpsc::Sender<TopicMessage>,
}

#[async_trait]
impl TopicHandler for ChannelTopicHandler {
    async fn on_topic_message(
        &self,
        topic: &str,
        msg_type: &str,
        payload: serde_json::Value,
        publisher_id: Option<&str>,
        seq: u64,
    ) -> Result<()> {
        let (commit_tx, commit_rx) = oneshot::channel();
        let msg = TopicMessage {
            topic: topic.to_owned(),
            msg_type: msg_type.to_owned(),
            payload,
            publisher_id: publisher_id.map(ToOwned::to_owned),
            seq,
            commit_tx: Some(commit_tx),
        };
        self.tx
            .send(msg)
            .await
            .map_err(|_| anyhow!("relay topic receiver dropped"))?;
        commit_rx
            .await
            .map_err(|_| anyhow!("relay topic delivery was dropped without commit"))?
    }
}

/// High-level pub/sub wrapper around [`RelayHandle`].
///
/// Use [`RelaySubscriber::make_handler`] to create a `(handler, receiver)` pair
/// before calling `relay_client::connect`.  Pass the handler to `connect`; keep
/// the receiver to consume incoming messages.  Then wrap the returned
/// `RelayHandle` with [`RelaySubscriber::from_handle`] for ergonomic
/// subscribe/publish calls.
///
/// # Example
///
/// ```no_run
/// use std::sync::Arc;
/// use roko_agent_server::features::relay_subscriber::RelaySubscriber;
/// // (relay_handle obtained from relay_client::connect)
/// # async fn example(relay_handle: roko_agent_server::features::relay_client::RelayHandle) -> anyhow::Result<()> {
/// let (handler, mut rx) = RelaySubscriber::make_handler();
/// // pass handler to relay_client::connect(…, Some(handler))
/// let subscriber = RelaySubscriber::from_handle(relay_handle);
/// subscriber.subscribe("agent:updates")?;
/// while let Some(msg) = rx.recv().await {
///     println!("topic={} seq={}", msg.topic, msg.seq);
///     msg.commit()?;
/// }
/// # Ok(())
/// # }
/// ```
pub struct RelaySubscriber {
    handle: RelayHandle,
}

impl RelaySubscriber {
    /// Create a `(TopicHandler, receiver)` pair for channel-based message delivery.
    ///
    /// Pass the returned `handler` to `relay_client::connect` as `topic_handler`.
    /// All incoming topic messages will be forwarded to the returned `receiver`.
    #[must_use]
    pub fn make_handler() -> (Arc<dyn TopicHandler>, mpsc::Receiver<TopicMessage>) {
        Self::make_handler_with_capacity(64)
    }

    /// Create a handler with an explicit bounded delivery capacity.
    #[must_use]
    pub fn make_handler_with_capacity(
        capacity: usize,
    ) -> (Arc<dyn TopicHandler>, mpsc::Receiver<TopicMessage>) {
        let (tx, rx) = mpsc::channel(capacity.clamp(1, 4_096));
        let handler: Arc<dyn TopicHandler> = Arc::new(ChannelTopicHandler { tx });
        (handler, rx)
    }

    /// Wrap an existing [`RelayHandle`] for ergonomic pub/sub calls.
    ///
    /// Use this after calling `relay_client::connect` to get a handle that
    /// provides named `subscribe`/`unsubscribe`/`publish` methods.
    #[must_use]
    pub fn from_handle(handle: RelayHandle) -> Self {
        Self { handle }
    }

    /// Subscribe to `topic` on the relay.
    ///
    /// # Errors
    ///
    /// Returns an error when the underlying relay connection has been closed.
    pub fn subscribe(&self, topic: impl Into<String>) -> Result<()> {
        self.handle.subscribe(topic)
    }

    /// Unsubscribe from `topic` on the relay.
    ///
    /// # Errors
    ///
    /// Returns an error when the underlying relay connection has been closed.
    pub fn unsubscribe(&self, topic: impl Into<String>) -> Result<()> {
        self.handle.unsubscribe(topic)
    }

    /// Publish `payload` to `topic` with the given `msg_type`.
    ///
    /// # Errors
    ///
    /// Returns an error when the underlying relay connection has been closed.
    pub fn publish(
        &self,
        topic: impl Into<String>,
        msg_type: impl Into<String>,
        payload: serde_json::Value,
    ) -> Result<()> {
        self.handle.publish(topic, msg_type, payload)
    }

    /// Access the underlying [`RelayHandle`] for ad-hoc frame sending.
    #[must_use]
    pub fn handle(&self) -> &RelayHandle {
        &self.handle
    }
}
