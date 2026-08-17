//! Dedicated outbound relay bridge for presence, card hosting, and messaging.

use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use std::time::Duration;

use agent_relay::protocol::{AgentHello, AgentInboundFrame, RelayOutboundFrame};
use anyhow::{Result, anyhow};
use async_trait::async_trait;
use futures::{SinkExt, StreamExt};
use serde_json::{Value, json};
use tokio::sync::{Semaphore, mpsc, watch};
use tokio::time::Instant;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, connect_async, tungstenite::Message};
use tokio_util::sync::CancellationToken;

use roko_core::wire_protocol::{RelayEnvelope, SnapshotMessage, SubscribeMessage};

use crate::registration::AgentCard;
use crate::state::{AgentState, DispatchError};

type RelaySocket = WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>;

const COMMAND_CAPACITY: usize = 256;
const DELIVERY_CAPACITY: usize = 64;
const RESPONSE_CAPACITY: usize = 32;
/// Maximum number of relay rooms restored atomically for one client.
pub const MAX_DESIRED_ROOMS: usize = 64;
const MAX_DESIRED_FEEDS: usize = 64;
const MAX_RELAY_FEED_FIELD_BYTES: usize = 512;
const MAX_MESSAGE_DISPATCHES: usize = 16;
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const BACKOFF_INITIAL: Duration = Duration::from_millis(250);
const BACKOFF_MAX: Duration = Duration::from_secs(30);

/// Callback interface for receiving topic messages from the relay.
#[async_trait]
pub trait TopicHandler: Send + Sync + 'static {
    /// Load the last cursor already committed by durable handler storage.
    async fn durable_cursor(&self) -> Result<u64> {
        Ok(0)
    }

    /// Commit one topic message durably. Returning success is the only event
    /// that permits the relay client to advance its cursor and emit ACK.
    async fn on_topic_message(
        &self,
        topic: &str,
        msg_type: &str,
        payload: serde_json::Value,
        publisher_id: Option<&str>,
        seq: u64,
    ) -> Result<()>;

    /// Apply a relay snapshot durably before its cursor becomes resumable.
    async fn on_snapshot(&self, _snapshot: &SnapshotMessage) -> Result<SnapshotDisposition> {
        Ok(SnapshotDisposition::ReconciliationRequired)
    }

    /// Persist an authoritative replay watermark, including quiet-room gaps.
    async fn on_replay_complete(&self, _from_seq: u64, _to_seq: u64) -> Result<()> {
        Err(anyhow!(
            "topic handler does not support durable replay checkpoints"
        ))
    }
}

/// Result of durably inspecting a generic relay snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnapshotDisposition {
    /// The snapshot is proven equivalent to replayed topic state.
    AppliedEquivalent,
    /// Durable reconciliation was recorded and automatic execution must halt.
    ReconciliationRequired,
}

/// Handle returned from [`connect`] that allows sending pub/sub frames.
///
/// The handle is cheap to clone — each clone shares the same underlying
/// sender channel.  If the background relay task stops (e.g. the relay
/// disconnects), subsequent sends return an error but will never panic.
#[derive(Clone)]
pub struct RelayHandle {
    command_tx: mpsc::Sender<ClientCommand>,
    shutdown: CancellationToken,
    status_rx: watch::Receiver<RelayClientStatus>,
}

/// Observable supervisor state for health/status surfaces.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RelayClientStatus {
    /// The supervisor has an active websocket and its current durable cursor.
    Connected {
        /// Highest globally ordered sequence committed by the handler.
        durable_cursor: u64,
        /// Number of rooms restored on reconnect.
        desired_rooms: usize,
    },
    /// The websocket is unavailable and the supervisor will retry.
    Disconnected {
        /// Cursor that will be used by the next atomic subscription restore.
        durable_cursor: u64,
    },
    /// A generic snapshot could not safely substitute for topic replay.
    ReconciliationRequired {
        /// Relay sequence attached to the rejected snapshot.
        snapshot_seq: u64,
    },
    /// Another connection took ownership of the same agent id.
    Superseded {
        /// Relay instance identifier that superseded this client.
        by_instance: String,
    },
    /// The client was explicitly shut down.
    Stopped,
}

#[derive(Clone)]
enum ClientCommand {
    Subscribe(String),
    Unsubscribe(String),
    RegisterFeed(agent_relay::protocol::FeedDescriptor),
    Publish {
        topic: String,
        msg_type: String,
        payload: Value,
    },
}

impl RelayHandle {
    /// Subscribe to a topic on the relay.
    ///
    /// # Errors
    ///
    /// Returns an error when the relay connection has been closed.
    pub fn subscribe(&self, topic: impl Into<String>) -> Result<()> {
        let topic = topic.into();
        validate_room(&topic)?;
        self.enqueue(ClientCommand::Subscribe(topic))
    }

    /// Unsubscribe from a previously subscribed topic.
    ///
    /// # Errors
    ///
    /// Returns an error when the relay connection has been closed.
    pub fn unsubscribe(&self, topic: impl Into<String>) -> Result<()> {
        let topic = topic.into();
        validate_room(&topic)?;
        self.enqueue(ClientCommand::Unsubscribe(topic))
    }

    /// Register a feed with the relay.
    ///
    /// # Errors
    ///
    /// Returns an error when the relay connection has been closed.
    pub fn register_feed(
        &self,
        feed_id: impl Into<String>,
        topic: impl Into<String>,
        name: impl Into<String>,
        description: impl Into<String>,
        kind: impl Into<String>,
        rate: impl Into<String>,
    ) -> Result<()> {
        let feed = agent_relay::protocol::FeedDescriptor {
            feed_id: feed_id.into(),
            topic: topic.into(),
            name: name.into(),
            description: description.into(),
            kind: kind.into(),
            rate: rate.into(),
            schema: None,
        };
        validate_feed(&feed)?;
        validate_room(&feed.topic)?;
        self.enqueue(ClientCommand::RegisterFeed(feed))
    }

    /// Publish a message to a topic.  The relay fans the message out to all
    /// current subscribers of that topic.
    ///
    /// # Errors
    ///
    /// Returns an error when the relay connection has been closed.
    pub fn publish(
        &self,
        topic: impl Into<String>,
        msg_type: impl Into<String>,
        payload: serde_json::Value,
    ) -> Result<()> {
        let topic = topic.into();
        let msg_type = msg_type.into();
        RelayEnvelope {
            seq: 0,
            ts: 0,
            room: topic.clone(),
            msg_type: msg_type.clone(),
            payload: payload.clone(),
            publisher_id: None,
        }
        .validate()?;
        self.enqueue(ClientCommand::Publish {
            topic,
            msg_type,
            payload,
        })
    }

    /// Cancel the supervisor and close the active relay connection.
    pub fn shutdown(&self) {
        self.shutdown.cancel();
    }

    #[must_use]
    /// Return the latest observable supervisor state.
    pub fn status(&self) -> RelayClientStatus {
        self.status_rx.borrow().clone()
    }

    #[must_use]
    /// Subscribe to supervisor state changes for health/status surfaces.
    pub fn subscribe_status(&self) -> watch::Receiver<RelayClientStatus> {
        self.status_rx.clone()
    }

    fn enqueue(&self, command: ClientCommand) -> Result<()> {
        self.command_tx
            .try_send(command)
            .map_err(|error| match error {
                mpsc::error::TrySendError::Full(_) => anyhow!("relay command queue is full"),
                mpsc::error::TrySendError::Closed(_) => anyhow!("relay client stopped"),
            })
    }
}

/// Configuration for connecting an agent runtime to `agent-relay`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelayClientConfig {
    /// Relay base URL or `/relay`-scoped base URL.
    pub base_url: String,
    initial_rooms: Vec<String>,
}

impl RelayClientConfig {
    /// Build relay-client configuration from a base URL.
    #[must_use]
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            initial_rooms: Vec::new(),
        }
    }

    /// Validate and install the complete room set restored on first connect.
    ///
    /// # Errors
    ///
    /// Returns an error if any room is invalid, the set is empty, or the
    /// deduplicated set exceeds [`MAX_DESIRED_ROOMS`].
    pub fn with_initial_rooms(mut self, rooms: impl IntoIterator<Item = String>) -> Result<Self> {
        let mut unique = HashSet::new();
        for room in rooms {
            validate_room(&room)?;
            unique.insert(room);
        }
        if unique.is_empty() || unique.len() > MAX_DESIRED_ROOMS {
            return Err(anyhow!(
                "relay initial room set must contain 1-{MAX_DESIRED_ROOMS} rooms"
            ));
        }
        self.initial_rooms = unique.into_iter().collect();
        self.initial_rooms.sort();
        Ok(self)
    }

    /// Build the externally visible card URI for `agent_id`.
    ///
    /// # Errors
    ///
    /// Returns an error when the configured base URL does not use an HTTP(S)
    /// or WS(S) scheme.
    pub fn card_uri(&self, agent_id: &str) -> Result<String> {
        let http_base = http_base_url(&self.base_url)?;
        Ok(format!("{}/cards/{agent_id}", relay_base_path(&http_base)))
    }

    fn websocket_url(&self) -> Result<String> {
        let ws_base = websocket_base_url(&self.base_url)?;
        Ok(format!("{}/agents/ws", relay_base_path(&ws_base)))
    }
}

/// Connect an agent runtime to the relay and keep servicing relay messages.
///
/// Returns a [`RelayHandle`] that can be used to subscribe to topics,
/// unsubscribe, and publish messages after the connection is established.
/// Pass `topic_handler` to receive incoming [`RelayOutboundFrame::TopicMessage`]
/// frames; pass `None` if pub/sub delivery is not required.
///
/// # Errors
///
/// Returns an error if the websocket connection or hello handshake fails.
pub async fn connect(
    config: RelayClientConfig,
    state: Arc<AgentState>,
    card: AgentCard,
    topic_handler: Option<Arc<dyn TopicHandler>>,
) -> Result<RelayHandle> {
    let initial_rooms = config.initial_rooms.clone();
    let identity = RelayIdentity {
        websocket_url: config.websocket_url()?,
        hello: AgentHello {
            agent_id: state.agent_id().to_string(),
            name: Some(card.name.clone()),
            capabilities: card.capabilities.clone(),
            rest_endpoint: public_rest_endpoint(&card),
            card: None,
            card_uri: Some(config.card_uri(state.agent_id())?),
            metadata: json!({
                "transport": "relay",
                "version": card.version,
            }),
        },
        card: serde_json::to_value(&card)?,
        card_uri: config.card_uri(state.agent_id())?,
    };
    let durable_cursor = if let Some(handler) = &topic_handler {
        handler.durable_cursor().await?
    } else {
        0
    };
    let socket = establish_connection_bounded(&identity, CONNECT_TIMEOUT).await?;
    let (command_tx, command_rx) = mpsc::channel(COMMAND_CAPACITY);
    let shutdown = CancellationToken::new();
    let initial_status = if initial_rooms.is_empty() {
        RelayClientStatus::Connected {
            durable_cursor,
            desired_rooms: 0,
        }
    } else {
        RelayClientStatus::Disconnected { durable_cursor }
    };
    let (status_tx, status_rx) = watch::channel(initial_status);
    tokio::spawn(supervise(
        Some(socket),
        identity,
        state,
        topic_handler,
        command_rx,
        shutdown.clone(),
        status_tx,
        durable_cursor,
        initial_rooms,
    ));
    Ok(RelayHandle {
        command_tx,
        shutdown,
        status_rx,
    })
}

#[derive(Clone)]
struct RelayIdentity {
    websocket_url: String,
    hello: AgentHello,
    card: Value,
    card_uri: String,
}

#[derive(Default)]
struct DesiredRelayState {
    rooms: HashSet<String>,
    feeds: HashMap<String, agent_relay::protocol::FeedDescriptor>,
    /// Cursor for the relay's single global sequence stream. Delivery work is
    /// committed by one FIFO worker, so this never advances past unfinished
    /// lower-sequence work from another room.
    durable_cursor: u64,
}

impl DesiredRelayState {
    fn new(initial_rooms: Vec<String>, durable_cursor: u64) -> Self {
        Self {
            rooms: initial_rooms.into_iter().collect(),
            feeds: HashMap::new(),
            durable_cursor,
        }
    }
}

enum RunExit {
    Disconnected(anyhow::Error),
    Superseded(String),
    ReconciliationRequired(u64),
    Shutdown,
}

async fn supervise(
    mut initial_socket: Option<RelaySocket>,
    identity: RelayIdentity,
    state: Arc<AgentState>,
    topic_handler: Option<Arc<dyn TopicHandler>>,
    mut command_rx: mpsc::Receiver<ClientCommand>,
    shutdown: CancellationToken,
    status_tx: watch::Sender<RelayClientStatus>,
    durable_cursor: u64,
    initial_rooms: Vec<String>,
) {
    let mut desired = DesiredRelayState::new(initial_rooms, durable_cursor);
    let mut attempt = 0u32;
    loop {
        if shutdown.is_cancelled() {
            return;
        }
        let socket = if let Some(socket) = initial_socket.take() {
            socket
        } else {
            match tokio::select! {
                () = shutdown.cancelled() => return,
                result = tokio::time::timeout(CONNECT_TIMEOUT, establish_connection(&identity)) => result,
            } {
                Ok(Ok(socket)) => socket,
                Ok(Err(error)) => {
                    let _ = status_tx.send(RelayClientStatus::Disconnected {
                        durable_cursor: desired.durable_cursor,
                    });
                    tracing::warn!(%error, attempt, "relay reconnect failed");
                    if wait_backoff(attempt, &identity.hello.agent_id, &shutdown).await {
                        return;
                    }
                    attempt = attempt.saturating_add(1);
                    continue;
                }
                Err(_) => {
                    let _ = status_tx.send(RelayClientStatus::Disconnected {
                        durable_cursor: desired.durable_cursor,
                    });
                    tracing::warn!(attempt, "relay reconnect timed out");
                    if wait_backoff(attempt, &identity.hello.agent_id, &shutdown).await {
                        return;
                    }
                    attempt = attempt.saturating_add(1);
                    continue;
                }
            }
        };
        let connected_at = Instant::now();
        if desired.rooms.is_empty() {
            let _ = status_tx.send(RelayClientStatus::Connected {
                durable_cursor: desired.durable_cursor,
                desired_rooms: 0,
            });
        } else {
            let _ = status_tx.send(RelayClientStatus::Disconnected {
                durable_cursor: desired.durable_cursor,
            });
        }
        let exit = run_connection(
            socket,
            Arc::clone(&state),
            topic_handler.clone(),
            &mut command_rx,
            &mut desired,
            &shutdown,
            &status_tx,
        )
        .await;
        if connected_at.elapsed() >= Duration::from_secs(30) {
            attempt = 0;
        } else {
            attempt = attempt.saturating_add(1);
        }
        match exit {
            RunExit::Shutdown => {
                let _ = status_tx.send(RelayClientStatus::Stopped);
                return;
            }
            RunExit::Superseded(by) => {
                let _ = status_tx.send(RelayClientStatus::Superseded {
                    by_instance: by.clone(),
                });
                tracing::warn!(%by, "relay client superseded; supervisor stopped");
                return;
            }
            RunExit::ReconciliationRequired(seq) => {
                let _ =
                    status_tx.send(RelayClientStatus::ReconciliationRequired { snapshot_seq: seq });
                tracing::error!(
                    seq,
                    "relay snapshot requires reconciliation; supervisor halted"
                );
                return;
            }
            RunExit::Disconnected(error) => {
                let _ = status_tx.send(RelayClientStatus::Disconnected {
                    durable_cursor: desired.durable_cursor,
                });
                tracing::warn!(%error, cursor = desired.durable_cursor, "relay disconnected; reconnecting");
                if wait_backoff(attempt, &identity.hello.agent_id, &shutdown).await {
                    return;
                }
            }
        }
    }
}

async fn establish_connection(identity: &RelayIdentity) -> Result<RelaySocket> {
    let (mut socket, _) = connect_async(identity.websocket_url.as_str()).await?;
    send_frame(
        &mut socket,
        AgentInboundFrame::Hello(identity.hello.clone()),
    )
    .await?;
    await_hello_ack(&mut socket).await?;
    send_frame(
        &mut socket,
        AgentInboundFrame::Card {
            card: identity.card.clone(),
            card_uri: Some(identity.card_uri.clone()),
        },
    )
    .await?;
    Ok(socket)
}

async fn establish_connection_bounded(
    identity: &RelayIdentity,
    timeout: Duration,
) -> Result<RelaySocket> {
    tokio::time::timeout(timeout, establish_connection(identity))
        .await
        .map_err(|_| anyhow!("relay connection timed out"))?
}

async fn restore_desired(socket: &mut RelaySocket, desired: &DesiredRelayState) -> Result<()> {
    if !desired.rooms.is_empty() {
        let mut rooms = desired.rooms.iter().cloned().collect::<Vec<_>>();
        rooms.sort();
        send_frame(
            socket,
            AgentInboundFrame::Subscribe {
                request: SubscribeMessage {
                    rooms,
                    last_seq: Some(desired.durable_cursor),
                },
                topic: None,
            },
        )
        .await?;
    }
    let mut feeds = desired.feeds.values().cloned().collect::<Vec<_>>();
    feeds.sort_by(|left, right| left.feed_id.cmp(&right.feed_id));
    for feed in feeds {
        send_frame(socket, AgentInboundFrame::RegisterFeed { feed }).await?;
    }
    Ok(())
}

#[allow(clippy::too_many_lines)]
async fn run_connection(
    mut socket: RelaySocket,
    state: Arc<AgentState>,
    topic_handler: Option<Arc<dyn TopicHandler>>,
    command_rx: &mut mpsc::Receiver<ClientCommand>,
    desired: &mut DesiredRelayState,
    shutdown: &CancellationToken,
    status_tx: &watch::Sender<RelayClientStatus>,
) -> RunExit {
    if let Err(error) = restore_desired(&mut socket, desired).await {
        return RunExit::Disconnected(error);
    }
    let expected_subscription_ack = if desired.rooms.is_empty() {
        None
    } else {
        let mut rooms = desired.rooms.iter().cloned().collect::<Vec<_>>();
        rooms.sort();
        Some(format!("subscribed:{}", rooms.join(",")))
    };
    let mut subscriptions_admitted = expected_subscription_ack.is_none();
    let (delivery_tx, delivery_rx) = mpsc::channel(DELIVERY_CAPACITY);
    let (completion_tx, mut completion_rx) = mpsc::channel(DELIVERY_CAPACITY);
    let delivery_worker = tokio::spawn(delivery_worker(topic_handler, delivery_rx, completion_tx));
    let (response_tx, mut response_rx) = mpsc::channel(RESPONSE_CAPACITY);
    let dispatches = Arc::new(Semaphore::new(MAX_MESSAGE_DISPATCHES));
    let mut dispatch_tasks = tokio::task::JoinSet::new();
    let mut commands_open = true;

    let exit = loop {
        tokio::select! {
            () = shutdown.cancelled() => break RunExit::Shutdown,
            incoming = socket.next() => {
                let frame = match incoming {
                    Some(Ok(Message::Text(text))) => match serde_json::from_str::<RelayOutboundFrame>(text.as_str()) {
                        Ok(frame) => frame,
                        Err(error) => break RunExit::Disconnected(error.into()),
                    },
                    Some(Ok(Message::Ping(payload))) => {
                        if let Err(error) = socket.send(Message::Pong(payload)).await {
                            break RunExit::Disconnected(error.into());
                        }
                        continue;
                    }
                    Some(Ok(Message::Close(_))) | None => break RunExit::Disconnected(anyhow!("relay websocket closed")),
                    Some(Ok(_)) => continue,
                    Some(Err(error)) => break RunExit::Disconnected(error.into()),
                };
                match frame {
                    RelayOutboundFrame::Message { message_id, message } => {
                        let Ok(permit) = Arc::clone(&dispatches).try_acquire_owned() else {
                            let frame = AgentInboundFrame::Error {
                                message_id: Some(message_id),
                                error: "relay message dispatch capacity reached".to_owned(),
                            };
                            if let Err(error) = send_frame(&mut socket, frame).await {
                                break RunExit::Disconnected(error);
                            }
                            continue;
                        };
                        let response_tx = response_tx.clone();
                        let state = Arc::clone(&state);
                        dispatch_tasks.spawn(async move {
                            let _permit = permit;
                            let frame = match dispatch_relay_message(&state, message).await {
                                Ok(response) => AgentInboundFrame::Response { message_id, response },
                                Err(error) => AgentInboundFrame::Error { message_id: Some(message_id), error },
                            };
                            let _ = response_tx.send(frame).await;
                        });
                    }
                    RelayOutboundFrame::TopicMessage { topic, msg_type, payload, publisher_id, seq, .. } => {
                        // Frames at or below the global cursor are ignored and
                        // never synthesize a room ACK for a newly-added room.
                        if needs_durable_delivery(seq, desired.durable_cursor)
                            && delivery_tx.try_send(DeliveryWork::Topic { topic, msg_type, payload, publisher_id, seq }).is_err()
                        {
                            break RunExit::Disconnected(anyhow!("relay durable delivery lane is full"));
                        }
                    }
                    RelayOutboundFrame::Snapshot(snapshot) => {
                        if snapshot.seq < desired.durable_cursor {
                            break RunExit::ReconciliationRequired(snapshot.seq);
                        }
                        if snapshot.seq > desired.durable_cursor
                            && delivery_tx.try_send(DeliveryWork::Snapshot(snapshot)).is_err()
                        {
                            break RunExit::Disconnected(anyhow!("relay snapshot lane is full"));
                        }
                    }
                    RelayOutboundFrame::ReplayComplete { from_seq, to_seq } => {
                        if to_seq > desired.durable_cursor
                            && delivery_tx.try_send(DeliveryWork::ReplayComplete { from_seq, to_seq }).is_err()
                        {
                            break RunExit::Disconnected(anyhow!("relay checkpoint lane is full"));
                        }
                    }
                    RelayOutboundFrame::ResumeRequired { last_available_seq } => {
                        break RunExit::Disconnected(anyhow!("relay requested recovery at watermark {last_available_seq}"));
                    }
                    RelayOutboundFrame::Superseded(notice) => break RunExit::Superseded(notice.by_instance),
                    RelayOutboundFrame::Error { error, .. } => {
                        break RunExit::Disconnected(anyhow!("relay rejected connection state: {error}"));
                    }
                    RelayOutboundFrame::Ack { event } => {
                        if expected_subscription_ack.as_deref() == Some(event.as_str()) {
                            subscriptions_admitted = true;
                            let _ = status_tx.send(RelayClientStatus::Connected {
                                durable_cursor: desired.durable_cursor,
                                desired_rooms: desired.rooms.len(),
                            });
                        }
                    }
                    RelayOutboundFrame::Pong => {}
                }
            }
            completion = completion_rx.recv() => {
                let Some(completion) = completion else {
                    break RunExit::Disconnected(anyhow!("relay durable delivery worker stopped"));
                };
                match completion {
                    DeliveryCompletion::Committed { cursor, ack } => {
                        desired.durable_cursor = desired.durable_cursor.max(cursor);
                        if subscriptions_admitted {
                            let _ = status_tx.send(RelayClientStatus::Connected {
                                durable_cursor: desired.durable_cursor,
                                desired_rooms: desired.rooms.len(),
                            });
                        }
                        if let Some((room, seq)) = ack
                            && let Err(error) = send_frame(&mut socket, AgentInboundFrame::Ack { room, seq }).await
                        {
                            break RunExit::Disconnected(error);
                        }
                    }
                    DeliveryCompletion::Failed(error) => break RunExit::Disconnected(error),
                    DeliveryCompletion::ReconciliationRequired { seq } => {
                        break RunExit::ReconciliationRequired(seq);
                    }
                }
            }
            response = response_rx.recv() => {
                let Some(response) = response else {
                    break RunExit::Disconnected(anyhow!("relay response lane stopped"));
                };
                if let Err(error) = send_frame(&mut socket, response).await {
                    break RunExit::Disconnected(error);
                }
            }
            completed = dispatch_tasks.join_next(), if !dispatch_tasks.is_empty() => {
                if let Some(Err(error)) = completed {
                    tracing::warn!(%error, "relay dispatch task failed");
                }
            }
            command = command_rx.recv(), if commands_open => {
                match command {
                    Some(command) => {
                        if let Err(error) = apply_command(&mut socket, desired, command).await {
                            break RunExit::Disconnected(error);
                        }
                    }
                    None => commands_open = false,
                }
            }
        }
    };
    delivery_worker.abort();
    abort_dispatch_tasks(&mut dispatch_tasks).await;
    let _ = socket.close(None).await;
    exit
}

async fn abort_dispatch_tasks(tasks: &mut tokio::task::JoinSet<()>) {
    tasks.abort_all();
    while tasks.join_next().await.is_some() {}
}

fn needs_durable_delivery(seq: u64, durable_cursor: u64) -> bool {
    seq > durable_cursor
}

async fn apply_command(
    socket: &mut RelaySocket,
    desired: &mut DesiredRelayState,
    command: ClientCommand,
) -> Result<()> {
    match command {
        ClientCommand::Subscribe(room) => {
            if !desired.rooms.contains(&room) && desired.rooms.len() >= MAX_DESIRED_ROOMS {
                return Err(anyhow!("relay desired subscription capacity reached"));
            }
            if desired.rooms.insert(room.clone()) {
                return Err(anyhow!(
                    "relay subscription set changed; reconnecting with atomic cursor"
                ));
            }
        }
        ClientCommand::Unsubscribe(room) => {
            if desired.rooms.remove(&room) {
                return Err(anyhow!(
                    "relay subscription set changed; reconnecting with atomic cursor"
                ));
            }
        }
        ClientCommand::RegisterFeed(feed) => {
            if !desired.feeds.contains_key(&feed.feed_id)
                && desired.feeds.len() >= MAX_DESIRED_FEEDS
            {
                return Err(anyhow!("relay desired feed capacity reached"));
            }
            desired.feeds.insert(feed.feed_id.clone(), feed.clone());
            send_frame(socket, AgentInboundFrame::RegisterFeed { feed }).await?;
        }
        ClientCommand::Publish {
            topic,
            msg_type,
            payload,
        } => {
            send_frame(
                socket,
                AgentInboundFrame::Publish {
                    topic,
                    msg_type,
                    payload,
                },
            )
            .await?;
        }
    }
    Ok(())
}

enum DeliveryWork {
    Topic {
        topic: String,
        msg_type: String,
        payload: Value,
        publisher_id: Option<String>,
        seq: u64,
    },
    Snapshot(SnapshotMessage),
    ReplayComplete {
        from_seq: u64,
        to_seq: u64,
    },
}

#[derive(Debug)]
enum DeliveryCompletion {
    Committed {
        cursor: u64,
        ack: Option<(String, u64)>,
    },
    Failed(anyhow::Error),
    ReconciliationRequired {
        seq: u64,
    },
}

async fn delivery_worker(
    handler: Option<Arc<dyn TopicHandler>>,
    mut delivery_rx: mpsc::Receiver<DeliveryWork>,
    completion_tx: mpsc::Sender<DeliveryCompletion>,
) {
    while let Some(work) = delivery_rx.recv().await {
        let completion = process_delivery(handler.as_deref(), work).await;
        if completion_tx.send(completion).await.is_err() {
            return;
        }
    }
}

async fn process_delivery(
    handler: Option<&dyn TopicHandler>,
    work: DeliveryWork,
) -> DeliveryCompletion {
    let Some(handler) = handler else {
        return DeliveryCompletion::Failed(anyhow!(
            "relay delivered subscription data without a durable topic handler"
        ));
    };
    let result = match work {
        DeliveryWork::Topic {
            topic,
            msg_type,
            payload,
            publisher_id,
            seq,
        } => handler
            .on_topic_message(&topic, &msg_type, payload, publisher_id.as_deref(), seq)
            .await
            .map(|()| DeliveryCompletion::Committed {
                cursor: seq,
                ack: Some((topic, seq)),
            }),
        DeliveryWork::Snapshot(snapshot) => {
            handler
                .on_snapshot(&snapshot)
                .await
                .map(|disposition| match disposition {
                    SnapshotDisposition::AppliedEquivalent => DeliveryCompletion::Committed {
                        cursor: snapshot.seq,
                        ack: None,
                    },
                    SnapshotDisposition::ReconciliationRequired => {
                        DeliveryCompletion::ReconciliationRequired { seq: snapshot.seq }
                    }
                })
        }
        DeliveryWork::ReplayComplete { from_seq, to_seq } => handler
            .on_replay_complete(from_seq, to_seq)
            .await
            .map(|()| DeliveryCompletion::Committed {
                cursor: to_seq,
                ack: None,
            }),
    };
    result.unwrap_or_else(DeliveryCompletion::Failed)
}

async fn wait_backoff(attempt: u32, agent_id: &str, shutdown: &CancellationToken) -> bool {
    let delay = reconnect_delay(attempt, agent_id);
    tokio::select! {
        () = shutdown.cancelled() => true,
        () = tokio::time::sleep(delay) => false,
    }
}

fn reconnect_delay(attempt: u32, agent_id: &str) -> Duration {
    let multiplier = 1u32.checked_shl(attempt.min(7)).unwrap_or(u32::MAX);
    let base = BACKOFF_INITIAL.saturating_mul(multiplier).min(BACKOFF_MAX);
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    agent_id.hash(&mut hasher);
    attempt.hash(&mut hasher);
    let jitter_percent = 75 + (hasher.finish() % 51) as u32;
    base.saturating_mul(jitter_percent) / 100
}

fn validate_room(room: &str) -> Result<()> {
    RelayEnvelope {
        seq: 0,
        ts: 0,
        room: room.to_owned(),
        msg_type: "subscription".to_owned(),
        payload: Value::Null,
        publisher_id: None,
    }
    .validate()?;
    Ok(())
}

fn validate_feed(feed: &agent_relay::protocol::FeedDescriptor) -> Result<()> {
    validate_feed_field("feed_id", &feed.feed_id, true)?;
    validate_feed_field("name", &feed.name, true)?;
    validate_feed_field("description", &feed.description, false)?;
    validate_feed_field("kind", &feed.kind, false)?;
    validate_feed_field("rate", &feed.rate, false)
}

fn validate_feed_field(name: &str, value: &str, required: bool) -> Result<()> {
    if value.len() > MAX_RELAY_FEED_FIELD_BYTES
        || value.bytes().any(|byte| byte.is_ascii_control())
        || (required && value.trim().is_empty())
    {
        return Err(anyhow!(
            "relay feed {name} must be {}-{MAX_RELAY_FEED_FIELD_BYTES} non-control bytes",
            usize::from(required)
        ));
    }
    Ok(())
}

async fn dispatch_relay_message(state: &AgentState, message: Value) -> Result<Value, String> {
    let prompt = extract_prompt(&message)
        .ok_or_else(|| "relay message did not contain a string prompt".to_string())?;
    let response = state
        .dispatch_prompt(&prompt)
        .await
        .map_err(dispatch_error)?;
    Ok(json!({
        "response": response.content,
        "reasoning": response.reasoning,
        "usage": response.usage,
        "finish_reason": format_finish_reason(response.finish_reason),
        "session": {
            "session_id": response.session.session_id,
            "thread_id": response.session.thread_id,
            "conversation_id": response.session.conversation_id,
        }
    }))
}

fn dispatch_error(error: DispatchError) -> String {
    match error {
        DispatchError::NotConfigured => "agent has no configured dispatcher".to_string(),
        DispatchError::DispatchFailed(reason) => format!("dispatch failed: {reason}"),
    }
}

fn extract_prompt(message: &Value) -> Option<String> {
    match message {
        Value::String(prompt) => Some(prompt.clone()),
        Value::Object(_) => message
            .get("prompt")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned)
            .or_else(|| {
                message
                    .pointer("/messages/0/content")
                    .and_then(Value::as_str)
                    .map(ToOwned::to_owned)
            }),
        _ => None,
    }
}

fn format_finish_reason(finish_reason: roko_agent::chat_types::FinishReason) -> String {
    match finish_reason {
        roko_agent::chat_types::FinishReason::Stop => "stop".to_string(),
        roko_agent::chat_types::FinishReason::Length => "length".to_string(),
        roko_agent::chat_types::FinishReason::ToolCalls => "tool_calls".to_string(),
        roko_agent::chat_types::FinishReason::ContentFilter => "content_filter".to_string(),
        roko_agent::chat_types::FinishReason::Error(reason) => reason,
    }
}

fn public_rest_endpoint(card: &AgentCard) -> Option<String> {
    let endpoint = card.endpoints.rest.as_ref()?;
    is_public_endpoint(endpoint).then(|| endpoint.clone())
}

fn is_public_endpoint(endpoint: &str) -> bool {
    let normalized = endpoint.to_ascii_lowercase();
    ![
        "http://127.0.0.1",
        "https://127.0.0.1",
        "http://localhost",
        "https://localhost",
        "http://0.0.0.0",
        "https://0.0.0.0",
        "http://[::1]",
        "https://[::1]",
    ]
    .iter()
    .any(|prefix| normalized.starts_with(prefix))
}

fn relay_base_path(base_url: &str) -> String {
    let trimmed = base_url.trim_end_matches('/');
    if trimmed.ends_with("/relay") {
        trimmed.to_string()
    } else {
        format!("{trimmed}/relay")
    }
}

fn websocket_base_url(base_url: &str) -> Result<String> {
    if let Some(rest) = base_url.strip_prefix("http://") {
        return Ok(format!("ws://{}", rest.trim_end_matches('/')));
    }
    if let Some(rest) = base_url.strip_prefix("https://") {
        return Ok(format!("wss://{}", rest.trim_end_matches('/')));
    }
    if base_url.starts_with("ws://") || base_url.starts_with("wss://") {
        return Ok(base_url.trim_end_matches('/').to_string());
    }
    Err(anyhow!(
        "relay base_url must use http(s) or ws(s): {base_url}"
    ))
}

fn http_base_url(base_url: &str) -> Result<String> {
    if base_url.starts_with("http://") || base_url.starts_with("https://") {
        return Ok(base_url.trim_end_matches('/').to_string());
    }
    if let Some(rest) = base_url.strip_prefix("ws://") {
        return Ok(format!("http://{}", rest.trim_end_matches('/')));
    }
    if let Some(rest) = base_url.strip_prefix("wss://") {
        return Ok(format!("https://{}", rest.trim_end_matches('/')));
    }
    Err(anyhow!(
        "relay base_url must use http(s) or ws(s): {base_url}"
    ))
}

async fn await_hello_ack(socket: &mut RelaySocket) -> Result<()> {
    while let Some(frame) = socket.next().await {
        match frame? {
            Message::Text(text) => {
                let outbound: RelayOutboundFrame = serde_json::from_str(text.as_str())?;
                match outbound {
                    RelayOutboundFrame::Ack { event } if event == "hello" => return Ok(()),
                    RelayOutboundFrame::Error { error, .. } => {
                        return Err(anyhow!("relay hello rejected: {error}"));
                    }
                    RelayOutboundFrame::Pong | RelayOutboundFrame::Ack { .. } => {}
                    RelayOutboundFrame::Message { .. } => {
                        return Err(anyhow!("relay sent a message before hello ack"));
                    }
                    // A TopicMessage before hello ack is unexpected but harmless.
                    RelayOutboundFrame::TopicMessage { .. } => {}
                    RelayOutboundFrame::Snapshot(_)
                    | RelayOutboundFrame::ReplayComplete { .. }
                    | RelayOutboundFrame::ResumeRequired { .. }
                    | RelayOutboundFrame::Superseded(_) => {}
                }
            }
            Message::Ping(payload) => {
                socket.send(Message::Pong(payload)).await?;
            }
            Message::Close(_) => return Err(anyhow!("relay websocket closed before hello ack")),
            _ => {}
        }
    }
    Err(anyhow!("relay websocket ended before hello ack"))
}

async fn send_frame(socket: &mut RelaySocket, frame: AgentInboundFrame) -> Result<()> {
    let payload = serde_json::to_string(&frame)?;
    socket.send(Message::Text(payload.into())).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::registration::AgentCardEndpoints;

    struct TestHandler {
        topic_error: bool,
        checkpoint_error: bool,
        snapshot: SnapshotDisposition,
        durable_cursor: u64,
    }

    struct HangingDispatcher {
        started_tx: mpsc::UnboundedSender<()>,
        dropped: Arc<std::sync::atomic::AtomicBool>,
    }

    struct DispatchDropSignal(Arc<std::sync::atomic::AtomicBool>);

    impl Drop for DispatchDropSignal {
        fn drop(&mut self) {
            self.0.store(true, std::sync::atomic::Ordering::SeqCst);
        }
    }

    #[async_trait]
    impl crate::state::DispatchLike for HangingDispatcher {
        async fn dispatch(
            &self,
            _request: roko_agent::chat_types::ChatRequest,
        ) -> std::result::Result<roko_agent::chat_types::ChatResponse, DispatchError> {
            let _signal = DispatchDropSignal(Arc::clone(&self.dropped));
            let _ = self.started_tx.send(());
            std::future::pending().await
        }
    }

    #[async_trait]
    impl TopicHandler for TestHandler {
        async fn durable_cursor(&self) -> Result<u64> {
            Ok(self.durable_cursor)
        }
        async fn on_topic_message(
            &self,
            _topic: &str,
            _msg_type: &str,
            _payload: Value,
            _publisher_id: Option<&str>,
            _seq: u64,
        ) -> Result<()> {
            if self.topic_error {
                Err(anyhow!("journal persistence failed"))
            } else {
                Ok(())
            }
        }

        async fn on_snapshot(&self, _snapshot: &SnapshotMessage) -> Result<SnapshotDisposition> {
            Ok(self.snapshot)
        }

        async fn on_replay_complete(&self, _from_seq: u64, _to_seq: u64) -> Result<()> {
            if self.checkpoint_error {
                Err(anyhow!("cursor persistence failed"))
            } else {
                Ok(())
            }
        }
    }

    async fn run_until_server_frame(frame: RelayOutboundFrame) -> RunExit {
        run_until_server_frame_with(frame, 0, None).await
    }

    async fn run_until_server_frame_with(
        frame: RelayOutboundFrame,
        durable_cursor: u64,
        handler: Option<Arc<dyn TopicHandler>>,
    ) -> RunExit {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind control test");
        let address = listener.local_addr().expect("control address");
        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.expect("accept control socket");
            let mut socket = tokio_tungstenite::accept_async(stream)
                .await
                .expect("accept control websocket");
            socket
                .send(Message::Text(
                    serde_json::to_string(&frame)
                        .expect("serialize control")
                        .into(),
                ))
                .await
                .expect("send control");
        });
        let (socket, _) = connect_async(format!("ws://{address}"))
            .await
            .expect("connect control websocket");
        let state = Arc::new(AgentState::new(
            "agent-a".to_owned(),
            None,
            "1.0.0".to_owned(),
            Vec::new(),
            None,
            None,
            None,
        ));
        let (_command_tx, mut command_rx) = mpsc::channel(1);
        let mut desired = DesiredRelayState {
            durable_cursor,
            ..DesiredRelayState::default()
        };
        let (status_tx, _status_rx) = watch::channel(RelayClientStatus::Stopped);
        let exit = run_connection(
            socket,
            state,
            handler,
            &mut command_rx,
            &mut desired,
            &CancellationToken::new(),
            &status_tx,
        )
        .await;
        server.await.expect("control server task");
        exit
    }

    #[test]
    fn card_uri_uses_relay_path() {
        let config = RelayClientConfig::new("https://relay.example.test");
        assert_eq!(
            config.card_uri("agent-1").expect("card uri"),
            "https://relay.example.test/relay/cards/agent-1"
        );
    }

    #[test]
    fn relay_scoped_base_url_is_preserved() {
        let config = RelayClientConfig::new("https://relay.example.test/relay");
        assert_eq!(
            config.websocket_url().expect("ws url"),
            "wss://relay.example.test/relay/agents/ws"
        );
    }

    #[test]
    fn initial_rooms_are_validated_deduplicated_sorted_and_capacity_bounded() {
        let config = RelayClientConfig::new("https://relay.example.test")
            .with_initial_rooms([
                "room:b".to_owned(),
                "room:a".to_owned(),
                "room:b".to_owned(),
            ])
            .expect("valid initial rooms");
        assert_eq!(config.initial_rooms, ["room:a", "room:b"]);
        assert!(
            RelayClientConfig::new("https://relay.example.test")
                .with_initial_rooms(Vec::new())
                .is_err()
        );
        assert!(
            RelayClientConfig::new("https://relay.example.test")
                .with_initial_rooms([" ".to_owned()])
                .is_err()
        );
        assert!(
            RelayClientConfig::new("https://relay.example.test")
                .with_initial_rooms((0..=MAX_DESIRED_ROOMS).map(|index| format!("room:{index}")))
                .is_err()
        );
        let desired = DesiredRelayState::new(config.initial_rooms, 41);
        assert_eq!(desired.rooms.len(), 2);
        assert_eq!(desired.durable_cursor, 41);
    }

    #[test]
    fn loopback_rest_endpoints_are_not_advertised() {
        let card = AgentCard {
            name: "agent-1".to_string(),
            capabilities: vec!["messaging".to_string()],
            endpoints: AgentCardEndpoints {
                rest: Some("http://127.0.0.1:8080".to_string()),
                websocket: None,
                a2a: None,
                mcp: None,
            },
            domain_tags: vec!["roko".to_string()],
            version: "1.0.0".to_string(),
        };
        assert_eq!(public_rest_endpoint(&card), None);
    }

    #[test]
    fn prompt_is_extracted_from_supported_message_shapes() {
        assert_eq!(
            extract_prompt(&json!({ "prompt": "hello" })),
            Some("hello".to_string())
        );
        assert_eq!(
            extract_prompt(&json!({
                "messages": [{ "content": "hello from transcript" }]
            })),
            Some("hello from transcript".to_string())
        );
        assert_eq!(
            extract_prompt(&Value::String("hello from string".to_string())),
            Some("hello from string".to_string())
        );
    }

    #[test]
    fn relay_handle_subscribe_returns_error_on_closed_channel() {
        let (tx, rx) = mpsc::channel(1);
        drop(rx);
        let (_status_tx, status_rx) = watch::channel(RelayClientStatus::Stopped);
        let handle = RelayHandle {
            command_tx: tx,
            shutdown: CancellationToken::new(),
            status_rx,
        };
        assert!(handle.subscribe("topic").is_err());
        assert!(handle.unsubscribe("topic").is_err());
        assert!(
            handle
                .publish("topic", "rate_update", serde_json::json!({}))
                .is_err()
        );
    }

    #[test]
    fn relay_handle_command_queue_is_bounded() {
        let (tx, _rx) = mpsc::channel(1);
        let (_status_tx, status_rx) = watch::channel(RelayClientStatus::Stopped);
        let handle = RelayHandle {
            command_tx: tx,
            shutdown: CancellationToken::new(),
            status_rx,
        };
        handle.subscribe("room:first").expect("first command");
        assert!(handle.subscribe("room:second").is_err());
    }

    #[test]
    fn relay_handle_rejects_unbounded_feed_fields_before_enqueue() {
        let (tx, mut rx) = mpsc::channel(8);
        let (_status_tx, status_rx) = watch::channel(RelayClientStatus::Stopped);
        let handle = RelayHandle {
            command_tx: tx,
            shutdown: CancellationToken::new(),
            status_rx,
        };
        for (feed_id, name, description, kind, rate) in [
            (
                " ".to_owned(),
                "name".to_owned(),
                String::new(),
                String::new(),
                String::new(),
            ),
            (
                "id".to_owned(),
                "\n".to_owned(),
                String::new(),
                String::new(),
                String::new(),
            ),
            (
                "id".to_owned(),
                "name".to_owned(),
                "x".repeat(MAX_RELAY_FEED_FIELD_BYTES + 1),
                String::new(),
                String::new(),
            ),
            (
                "id".to_owned(),
                "name".to_owned(),
                String::new(),
                "x".repeat(MAX_RELAY_FEED_FIELD_BYTES + 1),
                String::new(),
            ),
            (
                "id".to_owned(),
                "name".to_owned(),
                String::new(),
                String::new(),
                "x".repeat(MAX_RELAY_FEED_FIELD_BYTES + 1),
            ),
        ] {
            assert!(
                handle
                    .register_feed(feed_id, "room:a", name, description, kind, rate)
                    .is_err()
            );
        }
        assert!(rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn durable_handler_failure_produces_no_ack_or_cursor_commit() {
        let handler = TestHandler {
            topic_error: true,
            checkpoint_error: false,
            snapshot: SnapshotDisposition::AppliedEquivalent,
            durable_cursor: 0,
        };
        let completion = process_delivery(
            Some(&handler),
            DeliveryWork::Topic {
                topic: "room:a".to_owned(),
                msg_type: "event".to_owned(),
                payload: Value::Null,
                publisher_id: None,
                seq: 7,
            },
        )
        .await;
        assert!(matches!(completion, DeliveryCompletion::Failed(_)));
    }

    #[tokio::test]
    async fn durable_handler_success_is_the_only_topic_ack_disposition() {
        let handler = TestHandler {
            topic_error: false,
            checkpoint_error: false,
            snapshot: SnapshotDisposition::AppliedEquivalent,
            durable_cursor: 0,
        };
        let completion = process_delivery(
            Some(&handler),
            DeliveryWork::Topic {
                topic: "room:a".to_owned(),
                msg_type: "event".to_owned(),
                payload: Value::Null,
                publisher_id: None,
                seq: 7,
            },
        )
        .await;
        assert!(matches!(
            completion,
            DeliveryCompletion::Committed {
                cursor: 7,
                ack: Some((room, 7))
            } if room == "room:a"
        ));
    }

    #[tokio::test]
    async fn snapshot_reconciliation_never_advances_cursor() {
        let handler = TestHandler {
            topic_error: false,
            checkpoint_error: false,
            snapshot: SnapshotDisposition::ReconciliationRequired,
            durable_cursor: 0,
        };
        let completion = process_delivery(
            Some(&handler),
            DeliveryWork::Snapshot(SnapshotMessage {
                seq: 99,
                state: json!({}),
            }),
        )
        .await;
        assert!(matches!(
            completion,
            DeliveryCompletion::ReconciliationRequired { seq: 99 }
        ));
    }

    #[tokio::test]
    async fn restarted_relay_snapshot_below_durable_cursor_requires_reconciliation() {
        let handler: Arc<dyn TopicHandler> = Arc::new(TestHandler {
            topic_error: false,
            checkpoint_error: false,
            snapshot: SnapshotDisposition::AppliedEquivalent,
            durable_cursor: 77,
        });
        let exit = run_until_server_frame_with(
            RelayOutboundFrame::Snapshot(SnapshotMessage {
                seq: 0,
                state: json!({"relay": "restarted"}),
            }),
            77,
            Some(handler),
        )
        .await;
        assert!(matches!(exit, RunExit::ReconciliationRequired(0)));
    }

    #[tokio::test]
    async fn failed_quiet_room_checkpoint_does_not_commit_cursor() {
        let handler = TestHandler {
            topic_error: false,
            checkpoint_error: true,
            snapshot: SnapshotDisposition::AppliedEquivalent,
            durable_cursor: 0,
        };
        let completion = process_delivery(
            Some(&handler),
            DeliveryWork::ReplayComplete {
                from_seq: 8,
                to_seq: 42,
            },
        )
        .await;
        assert!(matches!(completion, DeliveryCompletion::Failed(_)));
    }

    #[tokio::test]
    async fn process_restart_loads_initial_cursor_from_durable_handler() {
        let handler = TestHandler {
            topic_error: false,
            checkpoint_error: false,
            snapshot: SnapshotDisposition::AppliedEquivalent,
            durable_cursor: 77,
        };
        assert_eq!(handler.durable_cursor().await.expect("durable cursor"), 77);
        let desired = DesiredRelayState {
            durable_cursor: handler.durable_cursor().await.expect("restart cursor"),
            ..DesiredRelayState::default()
        };
        assert_eq!(desired.durable_cursor, 77);
    }

    #[test]
    fn reconnect_backoff_is_jittered_and_bounded() {
        let first = reconnect_delay(0, "agent-a");
        let repeated = reconnect_delay(0, "agent-a");
        assert_eq!(first, repeated);
        assert!(first >= Duration::from_millis(187));
        assert!(first <= Duration::from_millis(312));
        assert!(reconnect_delay(100, "agent-a") <= BACKOFF_MAX.saturating_mul(125) / 100);
    }

    #[test]
    fn stale_duplicate_does_not_enter_delivery_or_ack_lane() {
        assert!(!needs_durable_delivery(40, 41));
        assert!(!needs_durable_delivery(41, 41));
        assert!(needs_durable_delivery(42, 41));
    }

    #[tokio::test]
    async fn initial_websocket_handshake_is_timeout_bounded() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind timeout test");
        let address = listener.local_addr().expect("timeout address");
        let server = tokio::spawn(async move {
            let (_stream, _) = listener.accept().await.expect("accept raw tcp");
            std::future::pending::<()>().await;
        });
        let identity = RelayIdentity {
            websocket_url: format!("ws://{address}"),
            hello: AgentHello {
                agent_id: "agent-a".to_owned(),
                name: None,
                capabilities: Vec::new(),
                rest_endpoint: None,
                card: None,
                card_uri: None,
                metadata: Value::Null,
            },
            card: Value::Null,
            card_uri: "http://relay.test/card".to_owned(),
        };
        let result = establish_connection_bounded(&identity, Duration::from_millis(20)).await;
        assert!(result.is_err());
        server.abort();
    }

    #[tokio::test]
    async fn disconnect_aborts_and_drains_pending_dispatch_tasks() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind dispatch test");
        let address = listener.local_addr().expect("dispatch address");
        let (started_tx, mut started_rx) = mpsc::unbounded_channel();
        let dropped = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.expect("accept dispatch socket");
            let mut socket = tokio_tungstenite::accept_async(stream)
                .await
                .expect("accept dispatch websocket");
            socket
                .send(Message::Text(
                    serde_json::to_string(&RelayOutboundFrame::Message {
                        message_id: "message-1".to_owned(),
                        message: json!({"prompt": "hang"}),
                    })
                    .expect("serialize dispatch")
                    .into(),
                ))
                .await
                .expect("send dispatch");
            started_rx.recv().await.expect("dispatch started");
            socket.close(None).await.expect("close dispatch socket");
        });
        let (socket, _) = connect_async(format!("ws://{address}"))
            .await
            .expect("connect dispatch websocket");
        let state = Arc::new(
            AgentState::new(
                "agent-a".to_owned(),
                None,
                "1.0.0".to_owned(),
                Vec::new(),
                None,
                None,
                None,
            )
            .with_message_dispatcher(Arc::new(HangingDispatcher {
                started_tx,
                dropped: Arc::clone(&dropped),
            })),
        );
        let (_command_tx, mut command_rx) = mpsc::channel(1);
        let mut desired = DesiredRelayState::default();
        let (status_tx, _status_rx) = watch::channel(RelayClientStatus::Stopped);

        let exit = run_connection(
            socket,
            state,
            None,
            &mut command_rx,
            &mut desired,
            &CancellationToken::new(),
            &status_tx,
        )
        .await;
        server.await.expect("dispatch server");
        assert!(matches!(exit, RunExit::Disconnected(_)));
        assert!(dropped.load(std::sync::atomic::Ordering::SeqCst));
    }

    #[tokio::test]
    async fn resume_required_disconnects_for_cursor_recovery() {
        let exit = run_until_server_frame(RelayOutboundFrame::ResumeRequired {
            last_available_seq: 12,
        })
        .await;
        assert!(matches!(
            exit,
            RunExit::Disconnected(error) if error.to_string().contains("watermark 12")
        ));
    }

    #[tokio::test]
    async fn superseded_notice_stops_reconnect_ownership() {
        let exit = run_until_server_frame(RelayOutboundFrame::Superseded(
            roko_core::wire_protocol::SupersededNotice {
                agent_id: "agent-a".to_owned(),
                by_instance: "instance-b".to_owned(),
            },
        ))
        .await;
        assert!(matches!(exit, RunExit::Superseded(by) if by == "instance-b"));
    }

    #[tokio::test]
    async fn reconnect_backoff_is_cancellation_safe() {
        let shutdown = CancellationToken::new();
        shutdown.cancel();
        assert!(wait_backoff(100, "agent-a", &shutdown).await);
    }

    #[tokio::test]
    async fn reconnect_restore_resubscribes_with_durable_cursor_and_feeds() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind websocket test");
        let address = listener.local_addr().expect("test address");
        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.expect("accept websocket");
            let mut socket = tokio_tungstenite::accept_async(stream)
                .await
                .expect("accept websocket handshake");
            let mut frames = Vec::new();
            for _ in 0..2 {
                let Message::Text(text) = socket
                    .next()
                    .await
                    .expect("client frame")
                    .expect("valid frame")
                else {
                    panic!("expected text frame");
                };
                frames.push(
                    serde_json::from_str::<AgentInboundFrame>(text.as_str())
                        .expect("decode inbound frame"),
                );
            }
            frames
        });
        let (mut socket, _) = connect_async(format!("ws://{address}"))
            .await
            .expect("connect websocket test");
        let feed = agent_relay::protocol::FeedDescriptor {
            feed_id: "feed-a".to_owned(),
            topic: "room:feed".to_owned(),
            name: "feed".to_owned(),
            description: String::new(),
            kind: String::new(),
            rate: String::new(),
            schema: None,
        };
        let desired = DesiredRelayState {
            rooms: HashSet::from(["room:a".to_owned(), "room:b".to_owned()]),
            feeds: HashMap::from([(feed.feed_id.clone(), feed.clone())]),
            durable_cursor: 41,
        };
        restore_desired(&mut socket, &desired)
            .await
            .expect("restore desired relay state");
        let frames = server.await.expect("server task");
        assert!(matches!(
            &frames[0],
            AgentInboundFrame::Subscribe { request, topic: None }
                if request.last_seq == Some(41)
                    && request.rooms == ["room:a".to_owned(), "room:b".to_owned()]
        ));
        assert!(matches!(
            &frames[1],
            AgentInboundFrame::RegisterFeed { feed: restored } if restored == &feed
        ));
    }

    #[tokio::test]
    async fn rejected_subscription_restore_is_a_supervised_disconnect() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind websocket test");
        let address = listener.local_addr().expect("test address");
        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.expect("accept websocket");
            let mut socket = tokio_tungstenite::accept_async(stream)
                .await
                .expect("accept websocket handshake");
            let _subscribe = socket.next().await.expect("subscribe frame");
            let rejection = RelayOutboundFrame::Error {
                message_id: None,
                error: "subscription rejected".to_owned(),
            };
            socket
                .send(Message::Text(
                    serde_json::to_string(&rejection)
                        .expect("serialize rejection")
                        .into(),
                ))
                .await
                .expect("send rejection");
        });
        let (socket, _) = connect_async(format!("ws://{address}"))
            .await
            .expect("connect websocket test");
        let state = Arc::new(AgentState::new(
            "agent-a".to_owned(),
            None,
            "1.0.0".to_owned(),
            Vec::new(),
            None,
            None,
            None,
        ));
        let (_command_tx, mut command_rx) = mpsc::channel(1);
        let mut desired = DesiredRelayState {
            rooms: HashSet::from(["room:a".to_owned()]),
            feeds: HashMap::new(),
            durable_cursor: 5,
        };
        let (status_tx, status_rx) = watch::channel(RelayClientStatus::Stopped);
        let exit = run_connection(
            socket,
            state,
            None,
            &mut command_rx,
            &mut desired,
            &CancellationToken::new(),
            &status_tx,
        )
        .await;
        assert!(matches!(
            exit,
            RunExit::Disconnected(error) if error.to_string().contains("subscription rejected")
        ));
        assert!(!matches!(
            status_rx.borrow().clone(),
            RelayClientStatus::Connected { .. }
        ));
        server.await.expect("server task");
    }

    #[tokio::test]
    async fn exact_subscription_ack_is_the_only_room_admission_signal() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind admission test");
        let address = listener.local_addr().expect("admission address");
        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.expect("accept admission socket");
            let mut socket = tokio_tungstenite::accept_async(stream)
                .await
                .expect("accept admission websocket");
            let _subscribe = socket.next().await.expect("subscribe frame");
            for frame in [
                RelayOutboundFrame::ReplayComplete {
                    from_seq: 0,
                    to_seq: 0,
                },
                RelayOutboundFrame::Ack {
                    event: "subscribed:room:a".to_owned(),
                },
                RelayOutboundFrame::Superseded(roko_core::wire_protocol::SupersededNotice {
                    agent_id: "agent-a".to_owned(),
                    by_instance: "test-instance".to_owned(),
                }),
            ] {
                socket
                    .send(Message::Text(
                        serde_json::to_string(&frame)
                            .expect("serialize admission frame")
                            .into(),
                    ))
                    .await
                    .expect("send admission frame");
            }
        });
        let (socket, _) = connect_async(format!("ws://{address}"))
            .await
            .expect("connect admission websocket");
        let state = Arc::new(AgentState::new(
            "agent-a".to_owned(),
            None,
            "1.0.0".to_owned(),
            Vec::new(),
            None,
            None,
            None,
        ));
        let (_command_tx, mut command_rx) = mpsc::channel(1);
        let mut desired = DesiredRelayState::new(vec!["room:a".to_owned()], 0);
        let (status_tx, status_rx) =
            watch::channel(RelayClientStatus::Disconnected { durable_cursor: 0 });
        let exit = run_connection(
            socket,
            state,
            None,
            &mut command_rx,
            &mut desired,
            &CancellationToken::new(),
            &status_tx,
        )
        .await;

        assert!(matches!(exit, RunExit::Superseded(_)));
        assert_eq!(
            status_rx.borrow().clone(),
            RelayClientStatus::Connected {
                durable_cursor: 0,
                desired_rooms: 1,
            }
        );
        server.await.expect("admission server");
    }
}
