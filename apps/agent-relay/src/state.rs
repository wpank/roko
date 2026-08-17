use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

use parking_lot::{Mutex, RwLock};
use roko_core::wire_protocol::SupersededNotice;
use serde_json::Value;
use tokio::sync::{OwnedSemaphorePermit, Semaphore, broadcast, oneshot};
use uuid::Uuid;

use crate::bus::{RelayMailbox, SharedRelayFrame, TopicBus, TopicBusConfig};
use crate::protocol::{
    AgentHello, ConnectedAgent, ConnectedWorkspace, FeedDescriptor, RelayEvent,
    RelayMessageRequest, RelayMessageResponse, RelayOutboundFrame, WorkspaceHello,
};
use crate::registry::RegistryStore;

struct ConnectedAgentHandle {
    session_id: Uuid,
    agent: ConnectedAgent,
    tx: RelayMailbox,
}

struct PendingResponse {
    agent_id: String,
    session_id: Uuid,
    tx: oneshot::Sender<Result<Value, String>>,
}

#[derive(Default)]
struct RelayStateInner {
    agents: HashMap<String, ConnectedAgentHandle>,
    cards: HashMap<String, Value>,
    pending: HashMap<String, PendingResponse>,
    workspaces: HashMap<String, ConnectedWorkspace>,
    /// Feeds registered by each agent, keyed by agent_id.
    feeds: HashMap<String, Vec<FeedDescriptor>>,
}

/// Shared in-memory relay state for directory, cards, and pending replies.
pub struct RelayState {
    inner: RwLock<RelayStateInner>,
    session_mutations: Mutex<()>,
    agent_socket_admission: std::sync::Arc<Semaphore>,
    events_socket_admission: std::sync::Arc<Semaphore>,
    events_tx: broadcast::Sender<RelayEvent>,
    /// Topic-based pub/sub bus. Agents subscribe/publish via WebSocket frames.
    pub bus: TopicBus,
    registry: Option<std::sync::Arc<RegistryStore>>,
}

pub const MAX_RELAY_CONNECTIONS: usize = 4_096;
pub const MAX_RELAY_EVENT_CONNECTIONS: usize = 1_024;
pub const MAX_PENDING_RESPONSES: usize = 4_096;
pub const MAX_RELAY_WORKSPACES: usize = 4_096;
pub const MAX_FEEDS_PER_AGENT: usize = 64;
pub const MAX_RELAY_TOTAL_FEEDS: usize = 16_384;
pub const MAX_RELAY_CARD_BYTES: usize = 256 * 1024;
pub const MAX_RELAY_METADATA_BYTES: usize = 64 * 1024;
pub const MAX_RELAY_FEED_SCHEMA_BYTES: usize = 64 * 1024;
pub const MAX_RELAY_SNAPSHOT_BYTES: usize = 4 * 1024 * 1024;
const MAX_RELAY_FIELD_BYTES: usize = 512;
const MAX_RELAY_CAPABILITIES: usize = 128;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegisterAgentError {
    Capacity,
    InvalidHello,
}

impl Default for RelayState {
    fn default() -> Self {
        Self::new()
    }
}

impl RelayState {
    #[must_use]
    pub fn new() -> Self {
        Self::try_with_config(TopicBusConfig::default())
            .expect("default relay bus configuration must be valid")
    }

    pub fn try_with_config(config: TopicBusConfig) -> roko_core::Result<Self> {
        let (events_tx, _) = broadcast::channel(256);
        Ok(Self {
            inner: RwLock::new(RelayStateInner::default()),
            session_mutations: Mutex::new(()),
            agent_socket_admission: std::sync::Arc::new(Semaphore::new(MAX_RELAY_CONNECTIONS)),
            events_socket_admission: std::sync::Arc::new(Semaphore::new(
                MAX_RELAY_EVENT_CONNECTIONS,
            )),
            events_tx,
            bus: TopicBus::try_new(config)?,
            registry: None,
        })
    }

    /// Create relay state with a persistent extension registry.
    #[must_use]
    pub fn with_registry(registry: RegistryStore) -> Self {
        Self::try_with_registry_config(registry, TopicBusConfig::default())
            .expect("default relay bus configuration must be valid")
    }

    pub fn try_with_registry_config(
        registry: RegistryStore,
        config: TopicBusConfig,
    ) -> roko_core::Result<Self> {
        let mut state = Self::try_with_config(config)?;
        state.registry = Some(std::sync::Arc::new(registry));
        Ok(state)
    }

    /// Configured extension registry, if this relay advertises one.
    #[must_use]
    pub fn registry(&self) -> Option<&std::sync::Arc<RegistryStore>> {
        self.registry.as_ref()
    }

    #[must_use]
    pub fn list_agents(&self) -> Vec<ConnectedAgent> {
        let mut agents = self
            .inner
            .read()
            .agents
            .values()
            .map(|entry| entry.agent.clone())
            .collect::<Vec<_>>();
        agents.sort_by(|left, right| left.agent_id.cmp(&right.agent_id));
        agents
    }

    #[must_use]
    pub fn card(&self, agent_id: &str) -> Option<Value> {
        self.inner.read().cards.get(agent_id).cloned()
    }

    #[must_use]
    pub fn subscribe_events(&self) -> broadcast::Receiver<RelayEvent> {
        self.events_tx.subscribe()
    }

    pub fn register_agent(
        &self,
        hello: AgentHello,
        tx: RelayMailbox,
    ) -> Result<RegisteredAgent, RegisterAgentError> {
        if !valid_agent_hello(&hello) {
            return Err(RegisterAgentError::InvalidHello);
        }
        let _session_guard = self.session_mutations.lock();
        let session_id = Uuid::new_v4();
        let card_uri = hello
            .card
            .as_ref()
            .map(|_| relay_card_uri(&hello.agent_id))
            .or_else(|| hello.card_uri.clone());
        let agent = ConnectedAgent {
            agent_id: hello.agent_id.clone(),
            name: hello.name,
            capabilities: hello.capabilities,
            rest_endpoint: hello.rest_endpoint,
            card_uri: card_uri.clone(),
            connected_at_ms: now_ms(),
            relay_backed: true,
        };

        let (superseded, superseded_pending) = {
            let mut inner = self.inner.write();
            if !inner.agents.contains_key(&agent.agent_id)
                && inner.agents.len() >= MAX_RELAY_CONNECTIONS
            {
                return Err(RegisterAgentError::Capacity);
            }
            if let Some(card) = hello.card {
                inner.cards.insert(agent.agent_id.clone(), card);
            }
            let previous = inner
                .agents
                .insert(
                    agent.agent_id.clone(),
                    ConnectedAgentHandle {
                        session_id,
                        agent: agent.clone(),
                        tx,
                    },
                )
                .map(|previous| (previous.session_id, previous.tx));
            let mut superseded_pending = Vec::new();
            if let Some((previous_session_id, _)) = &previous {
                let ids = inner
                    .pending
                    .iter()
                    .filter(|(_, pending)| {
                        pending.agent_id == agent.agent_id
                            && pending.session_id == *previous_session_id
                    })
                    .map(|(message_id, _)| message_id.clone())
                    .collect::<Vec<_>>();
                for message_id in ids {
                    if let Some(pending) = inner.pending.remove(&message_id) {
                        superseded_pending.push(pending);
                    }
                }
            }
            (previous.map(|(_, tx)| tx), superseded_pending)
        };
        self.bus.unsubscribe_all(&agent.agent_id);
        for pending in superseded_pending {
            let _ = pending.tx.send(Err("agent session superseded".to_owned()));
        }
        if let Some(previous) = superseded {
            previous.supersede(SupersededNotice {
                agent_id: agent.agent_id.clone(),
                by_instance: format!("inst_{}", session_id.simple()),
            });
        }

        let _ = self.events_tx.send(RelayEvent::AgentConnected {
            agent: agent.clone(),
        });
        if let Some(card_uri) = card_uri {
            let _ = self.events_tx.send(RelayEvent::CardUpdated {
                agent_id: agent.agent_id.clone(),
                card_uri,
            });
        }

        Ok(RegisteredAgent {
            session_id,
            agent_id: agent.agent_id,
        })
    }

    #[must_use]
    pub fn is_current_session(&self, agent_id: &str, session_id: Uuid) -> bool {
        self.inner
            .read()
            .agents
            .get(agent_id)
            .is_some_and(|entry| entry.session_id == session_id)
    }

    /// Execute one socket action while preventing duplicate registration from
    /// replacing its session between validation and mutation.
    pub fn with_current_session<T>(
        &self,
        agent_id: &str,
        session_id: Uuid,
        action: impl FnOnce() -> T,
    ) -> Option<T> {
        let _session_guard = self.session_mutations.lock();
        self.is_current_session(agent_id, session_id).then(action)
    }

    pub(crate) fn try_admit_agent_socket(&self) -> Option<OwnedSemaphorePermit> {
        std::sync::Arc::clone(&self.agent_socket_admission)
            .try_acquire_owned()
            .ok()
    }

    pub(crate) fn try_admit_events_socket(&self) -> Option<OwnedSemaphorePermit> {
        std::sync::Arc::clone(&self.events_socket_admission)
            .try_acquire_owned()
            .ok()
    }

    pub fn update_card(&self, agent_id: &str, card: Value, card_uri: Option<String>) -> bool {
        if json_size(&card) > MAX_RELAY_CARD_BYTES
            || card_uri
                .as_ref()
                .is_some_and(|uri| uri.len() > MAX_RELAY_FIELD_BYTES)
        {
            return false;
        }
        let resolved_card_uri = card_uri.unwrap_or_else(|| relay_card_uri(agent_id));
        {
            let mut inner = self.inner.write();
            if !inner.agents.contains_key(agent_id) {
                return false;
            }
            inner.cards.insert(agent_id.to_string(), card);
            if let Some(agent) = inner.agents.get_mut(agent_id) {
                agent.agent.card_uri = Some(resolved_card_uri.clone());
            }
        }
        let _ = self.events_tx.send(RelayEvent::CardUpdated {
            agent_id: agent_id.to_string(),
            card_uri: resolved_card_uri,
        });
        true
    }

    pub fn unregister_agent(&self, agent_id: &str, session_id: Uuid) {
        let _session_guard = self.session_mutations.lock();
        let (pending, removed_feeds) = {
            let mut inner = self.inner.write();
            let Some(current) = inner.agents.get(agent_id) else {
                return;
            };
            if current.session_id != session_id {
                return;
            }
            inner.agents.remove(agent_id);
            inner.cards.remove(agent_id);
            let mut pending = Vec::new();
            for (message_id, pending_response) in std::mem::take(&mut inner.pending) {
                if pending_response.agent_id == agent_id {
                    pending.push(pending_response);
                } else {
                    inner.pending.insert(message_id, pending_response);
                }
            }
            let removed_feeds = inner.feeds.remove(agent_id).unwrap_or_default();
            drop(inner);
            (pending, removed_feeds)
        };

        for pending in pending {
            let _ = pending.tx.send(Err("agent disconnected".to_string()));
        }
        // Clean up all topic subscriptions for this agent.
        self.bus.unsubscribe_all(agent_id);
        // Emit FeedUnregistered events for each removed feed.
        for feed in &removed_feeds {
            let _ = self.events_tx.send(RelayEvent::FeedUnregistered {
                agent_id: agent_id.to_string(),
                feed_id: feed.feed_id.clone(),
            });
        }
        let _ = self.events_tx.send(RelayEvent::AgentDisconnected {
            agent_id: agent_id.to_string(),
        });
    }

    pub fn begin_message(
        self: &std::sync::Arc<Self>,
        request: RelayMessageRequest,
    ) -> Result<PendingMessage, BeginMessageError> {
        if !valid_field(&request.agent_id) || json_size(&request.message) > MAX_RELAY_CARD_BYTES {
            return Err(BeginMessageError::InvalidRequest);
        }
        let message_id = Uuid::new_v4().to_string();
        let timeout_ms = request.timeout_ms();
        let (response_tx, response_rx) = oneshot::channel();

        let agent_tx = {
            let mut inner = self.inner.write();
            let Some(agent) = inner.agents.get(&request.agent_id) else {
                return Err(BeginMessageError::UnknownAgent);
            };
            if inner.pending.len() >= MAX_PENDING_RESPONSES {
                return Err(BeginMessageError::Capacity);
            }
            let agent_tx = agent.tx.clone();
            let session_id = agent.session_id;
            inner.pending.insert(
                message_id.clone(),
                PendingResponse {
                    agent_id: request.agent_id.clone(),
                    session_id,
                    tx: response_tx,
                },
            );
            agent_tx
        };

        if !matches!(
            agent_tx.send(RelayOutboundFrame::Message {
                message_id: message_id.clone(),
                message: request.message,
            }),
            Ok(crate::bus::DeliveryOutcome::Queued)
        ) {
            self.inner.write().pending.remove(&message_id);
            return Err(BeginMessageError::NotConnected);
        }

        let _ = self.events_tx.send(RelayEvent::MessageDelivered {
            agent_id: request.agent_id.clone(),
            message_id: message_id.clone(),
        });

        Ok(PendingMessage {
            agent_id: request.agent_id,
            message_id,
            timeout_ms,
            response_rx,
            state: std::sync::Arc::downgrade(self),
            finished: false,
        })
    }

    pub fn resolve_response(
        &self,
        agent_id: &str,
        session_id: Uuid,
        message_id: &str,
        result: Result<Value, String>,
    ) -> bool {
        let mut inner = self.inner.write();
        let Some(expected) = inner.pending.get(message_id) else {
            return false;
        };
        if expected.agent_id != agent_id || expected.session_id != session_id {
            return false;
        }
        let pending = inner
            .pending
            .remove(message_id)
            .expect("checked pending response must exist");
        drop(inner);

        let event = match &result {
            Ok(_) => RelayEvent::MessageResponded {
                agent_id: pending.agent_id.clone(),
                message_id: message_id.to_string(),
            },
            Err(error) => RelayEvent::AgentError {
                agent_id: pending.agent_id.clone(),
                message_id: Some(message_id.to_string()),
                error: error.clone(),
            },
        };
        let _ = pending.tx.send(result);
        let _ = self.events_tx.send(event);
        true
    }

    pub fn agent_error(
        &self,
        agent_id: &str,
        session_id: Uuid,
        message_id: Option<String>,
        error: String,
    ) {
        if let Some(message_id) = message_id {
            let _ = self.resolve_response(agent_id, session_id, &message_id, Err(error.clone()));
        }
        let _ = self.events_tx.send(RelayEvent::AgentError {
            agent_id: agent_id.to_string(),
            message_id: None,
            error,
        });
    }

    // ── Workspace directory ──────────────────────────────────────────

    #[must_use]
    pub fn list_workspaces(&self) -> Vec<ConnectedWorkspace> {
        let mut workspaces = self
            .inner
            .read()
            .workspaces
            .values()
            .cloned()
            .collect::<Vec<_>>();
        workspaces.sort_by(|a, b| a.workspace_id.cmp(&b.workspace_id));
        workspaces
    }

    pub fn register_workspace(&self, hello: WorkspaceHello) -> bool {
        if !valid_workspace_hello(&hello) {
            return false;
        }
        let now = now_ms();
        let workspace = ConnectedWorkspace {
            workspace_id: hello.workspace_id.clone(),
            name: hello.name,
            url: hello.url,
            version: hello.version,
            owner_wallet: hello.owner_wallet,
            agents_count: hello.agents_count,
            connected_at_ms: now,
            last_heartbeat_ms: now,
        };
        let mut inner = self.inner.write();
        if !inner.workspaces.contains_key(&hello.workspace_id)
            && inner.workspaces.len() >= MAX_RELAY_WORKSPACES
        {
            return false;
        }
        inner
            .workspaces
            .insert(hello.workspace_id, workspace.clone());
        drop(inner);
        let _ = self
            .events_tx
            .send(RelayEvent::WorkspaceConnected { workspace });
        true
    }

    pub fn workspace_heartbeat(&self, workspace_id: &str, agents_count: u32) {
        let mut inner = self.inner.write();
        if let Some(ws) = inner.workspaces.get_mut(workspace_id) {
            ws.last_heartbeat_ms = now_ms();
            ws.agents_count = agents_count;
        }
        drop(inner);
        let _ = self.events_tx.send(RelayEvent::WorkspaceHeartbeat {
            workspace_id: workspace_id.to_string(),
            agents_count,
        });
    }

    pub fn unregister_workspace(&self, workspace_id: &str) {
        self.inner.write().workspaces.remove(workspace_id);
        let _ = self.events_tx.send(RelayEvent::WorkspaceDisconnected {
            workspace_id: workspace_id.to_string(),
        });
    }

    /// Send a frame to a connected agent by ID.
    ///
    /// Returns `true` if the agent was found and the send succeeded,
    /// `false` if the agent is unknown or has disconnected.
    pub fn send_to_agent(&self, agent_id: &str, frame: RelayOutboundFrame) -> bool {
        let inner = self.inner.read();
        if let Some(handle) = inner.agents.get(agent_id) {
            matches!(
                handle.tx.send(frame),
                Ok(crate::bus::DeliveryOutcome::Queued
                    | crate::bus::DeliveryOutcome::Coalesced
                    | crate::bus::DeliveryOutcome::Sampled
                    | crate::bus::DeliveryOutcome::DroppedOldest)
            )
        } else {
            false
        }
    }

    /// Publish a validated envelope and fan it out in global sequence order.
    pub fn try_publish_topic(
        &self,
        envelope: crate::protocol::TopicEnvelope,
        skip_agent: Option<&str>,
    ) -> roko_core::Result<(u64, usize)> {
        let mut delivered = 0usize;
        let (seq, subscribers) =
            self.bus
                .try_publish_with(envelope, |seq, subscribers, envelope| {
                    let inner = self.inner.read();
                    let frame = SharedRelayFrame::prepare(RelayOutboundFrame::TopicMessage {
                        topic: envelope.topic.clone(),
                        msg_type: envelope.msg_type.clone(),
                        payload: envelope.payload.clone(),
                        publisher_id: envelope.publisher_id.clone(),
                        seq,
                        timestamp_ms: envelope.timestamp_ms,
                    })
                    .expect("validated relay frame must serialize");
                    for subscriber in subscribers {
                        if skip_agent.is_some_and(|skip| skip == subscriber) {
                            continue;
                        }
                        let Some(agent) = inner.agents.get(subscriber) else {
                            continue;
                        };
                        if agent.tx.send_shared(std::sync::Arc::clone(&frame)).is_ok() {
                            delivered = delivered.saturating_add(1);
                        }
                    }
                })?;
        debug_assert!(delivered <= subscribers.len());
        Ok((seq, delivered))
    }

    /// Canonical state used when a replay cursor has fallen out of the ring.
    #[must_use]
    pub fn relay_snapshot(&self) -> Value {
        self.relay_snapshot_with_budget(MAX_RELAY_SNAPSHOT_BYTES)
    }

    /// Build a snapshot whose serialized state fits the caller's frame budget.
    #[must_use]
    pub fn relay_snapshot_with_budget(&self, max_bytes: usize) -> Value {
        let max_bytes = max_bytes.clamp(256, MAX_RELAY_SNAPSHOT_BYTES);
        let snapshot_content_budget = max_bytes.saturating_sub(256);

        let mut all_rooms = self.bus.topic_stats();
        all_rooms.sort_by(|left, right| left.0.cmp(&right.0));
        let inner = self.inner.read();
        let agent_count = inner.agents.len();
        let feed_count = inner.feeds.values().map(Vec::len).sum::<usize>();
        let room_count = all_rooms.len();
        let mut estimated_bytes = 0usize;
        let mut truncated = false;

        let mut all_agents = inner
            .agents
            .values()
            .map(|entry| &entry.agent)
            .collect::<Vec<_>>();
        all_agents.sort_by(|left, right| left.agent_id.cmp(&right.agent_id));
        let mut agents = Vec::new();
        for agent in all_agents {
            let entry_bytes = serde_json::to_vec(agent).map_or(usize::MAX, |bytes| bytes.len() + 1);
            if estimated_bytes.saturating_add(entry_bytes) > snapshot_content_budget {
                truncated = true;
                break;
            }
            estimated_bytes = estimated_bytes.saturating_add(entry_bytes);
            agents.push(agent);
        }

        let mut feeds = Vec::new();
        for (agent_id, descriptors) in &inner.feeds {
            for descriptor in descriptors {
                let entry = serde_json::json!({
                    "agent_id": agent_id,
                    "feed": descriptor,
                });
                let entry_bytes = json_size(&entry).saturating_add(1);
                if estimated_bytes.saturating_add(entry_bytes) > snapshot_content_budget {
                    truncated = true;
                    break;
                }
                estimated_bytes = estimated_bytes.saturating_add(entry_bytes);
                feeds.push(entry);
            }
            if truncated {
                break;
            }
        }
        let mut rooms = Vec::new();
        for (room, _) in all_rooms {
            let entry_bytes = serde_json::to_vec(&room).map_or(usize::MAX, |bytes| bytes.len() + 1);
            if estimated_bytes.saturating_add(entry_bytes) > snapshot_content_budget {
                truncated = true;
                break;
            }
            estimated_bytes = estimated_bytes.saturating_add(entry_bytes);
            rooms.push(room);
        }
        let snapshot = serde_json::json!({
            "agents": agents,
            "feeds": feeds,
            "rooms": rooms,
            "truncated": truncated,
            "agent_count": agent_count,
            "feed_count": feed_count,
            "room_count": room_count,
        });
        if json_size(&snapshot) <= max_bytes {
            snapshot
        } else {
            serde_json::json!({
                "truncated": true,
                "agent_count": agent_count,
                "feed_count": feed_count,
                "room_count": room_count,
            })
        }
    }

    /// Remove workspaces that haven't sent a heartbeat in `stale_ms`.
    pub fn expire_stale_workspaces(&self, stale_ms: u64) -> Vec<String> {
        let now = now_ms();
        let mut expired = Vec::new();
        let mut inner = self.inner.write();
        inner.workspaces.retain(|id, ws| {
            if now.saturating_sub(ws.last_heartbeat_ms) > stale_ms {
                expired.push(id.clone());
                false
            } else {
                true
            }
        });
        drop(inner);
        for id in &expired {
            let _ = self.events_tx.send(RelayEvent::WorkspaceDisconnected {
                workspace_id: id.clone(),
            });
        }
        expired
    }

    // ── Feed registration ────────────────────────────────────────────

    pub fn register_feed(&self, agent_id: &str, feed: FeedDescriptor) -> bool {
        if !valid_feed(&feed) {
            return false;
        }
        let mut inner = self.inner.write();
        if !inner.agents.contains_key(agent_id) {
            return false;
        }
        let total: usize = inner.feeds.values().map(Vec::len).sum();
        let replacing = inner.feeds.get(agent_id).is_some_and(|feeds| {
            feeds
                .iter()
                .any(|existing| existing.feed_id == feed.feed_id)
        });
        if !replacing && total >= MAX_RELAY_TOTAL_FEEDS {
            return false;
        }
        let feeds = inner.feeds.entry(agent_id.to_string()).or_default();
        // Replace existing feed with same id, or append.
        if let Some(existing) = feeds.iter_mut().find(|f| f.feed_id == feed.feed_id) {
            *existing = feed.clone();
        } else {
            if feeds.len() >= MAX_FEEDS_PER_AGENT {
                return false;
            }
            feeds.push(feed.clone());
        }
        drop(inner);
        let _ = self.events_tx.send(RelayEvent::FeedRegistered {
            agent_id: agent_id.to_string(),
            feed,
        });
        true
    }

    pub fn unregister_feed(&self, agent_id: &str, feed_id: &str) -> bool {
        let mut inner = self.inner.write();
        let Some(feeds) = inner.feeds.get_mut(agent_id) else {
            return false;
        };
        let before = feeds.len();
        feeds.retain(|f| f.feed_id != feed_id);
        if feeds.len() == before {
            return false;
        }
        if feeds.is_empty() {
            inner.feeds.remove(agent_id);
        }
        drop(inner);
        let _ = self.events_tx.send(RelayEvent::FeedUnregistered {
            agent_id: agent_id.to_string(),
            feed_id: feed_id.to_string(),
        });
        true
    }

    #[must_use]
    pub fn list_feeds(&self) -> Vec<(String, Vec<FeedDescriptor>)> {
        let inner = self.inner.read();
        let mut result: Vec<(String, Vec<FeedDescriptor>)> = inner
            .feeds
            .iter()
            .map(|(agent_id, feeds)| (agent_id.clone(), feeds.clone()))
            .collect();
        result.sort_by(|a, b| a.0.cmp(&b.0));
        result
    }

    #[must_use]
    pub fn agent_feeds(&self, agent_id: &str) -> Vec<FeedDescriptor> {
        self.inner
            .read()
            .feeds
            .get(agent_id)
            .cloned()
            .unwrap_or_default()
    }
}

/// Live registration metadata returned after an agent hello succeeds.
pub struct RegisteredAgent {
    pub session_id: Uuid,
    pub agent_id: String,
}

/// Pending HTTP request waiting on an agent response.
pub struct PendingMessage {
    pub agent_id: String,
    pub message_id: String,
    pub timeout_ms: u64,
    response_rx: oneshot::Receiver<Result<Value, String>>,
    state: std::sync::Weak<RelayState>,
    finished: bool,
}

impl PendingMessage {
    pub async fn await_response(mut self) -> Result<RelayMessageResponse, AwaitMessageError> {
        let result = match tokio::time::timeout(
            std::time::Duration::from_millis(self.timeout_ms),
            &mut self.response_rx,
        )
        .await
        {
            Ok(Ok(Ok(response))) => Ok(RelayMessageResponse {
                message_id: self.message_id.clone(),
                agent_id: self.agent_id.clone(),
                response,
            }),
            Ok(Ok(Err(error))) => Err(AwaitMessageError::Agent(error)),
            Ok(Err(_)) => Err(AwaitMessageError::Agent(
                "response channel closed".to_string(),
            )),
            Err(_) => Err(AwaitMessageError::Timeout),
        };
        self.cleanup();
        result
    }

    fn cleanup(&mut self) {
        if self.finished {
            return;
        }
        if let Some(state) = self.state.upgrade() {
            state.inner.write().pending.remove(&self.message_id);
        }
        self.finished = true;
    }
}

impl Drop for PendingMessage {
    fn drop(&mut self) {
        self.cleanup();
    }
}

/// Errors returned while queuing a relay message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BeginMessageError {
    UnknownAgent,
    NotConnected,
    Capacity,
    InvalidRequest,
}

/// Errors returned while waiting for an agent response.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AwaitMessageError {
    Timeout,
    Agent(String),
}

#[must_use]
pub fn relay_card_uri(agent_id: &str) -> String {
    format!("/relay/cards/{agent_id}")
}

fn now_ms() -> u64 {
    u64::try_from(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis(),
    )
    .unwrap_or(u64::MAX)
}

fn json_size(value: &Value) -> usize {
    serde_json::to_vec(value).map_or(usize::MAX, |bytes| bytes.len())
}

fn valid_field(value: &str) -> bool {
    !value.trim().is_empty()
        && value.len() <= MAX_RELAY_FIELD_BYTES
        && !value.bytes().any(|byte| byte.is_ascii_control())
}

fn valid_agent_id(value: &str) -> bool {
    !value.trim().is_empty()
        && value.len() <= 256
        && !value
            .bytes()
            .any(|byte| byte.is_ascii_control() || byte.is_ascii_whitespace())
}

fn valid_optional_field(value: Option<&str>) -> bool {
    value.is_none_or(|value| value.len() <= MAX_RELAY_FIELD_BYTES)
}

fn valid_agent_hello(hello: &AgentHello) -> bool {
    valid_agent_id(&hello.agent_id)
        && valid_optional_field(hello.name.as_deref())
        && valid_optional_field(hello.rest_endpoint.as_deref())
        && valid_optional_field(hello.card_uri.as_deref())
        && hello.capabilities.len() <= MAX_RELAY_CAPABILITIES
        && hello.capabilities.iter().all(|value| valid_field(value))
        && hello
            .card
            .as_ref()
            .is_none_or(|card| json_size(card) <= MAX_RELAY_CARD_BYTES)
        && json_size(&hello.metadata) <= MAX_RELAY_METADATA_BYTES
}

fn valid_workspace_hello(hello: &WorkspaceHello) -> bool {
    valid_field(&hello.workspace_id)
        && hello.name.as_deref().is_none_or(valid_field)
        && hello.version.as_deref().is_none_or(valid_field)
        && valid_field(&hello.url)
        && valid_optional_field(hello.owner_wallet.as_deref())
}

fn valid_feed(feed: &FeedDescriptor) -> bool {
    valid_field(&feed.feed_id)
        && valid_field(&feed.name)
        && valid_optional_field(Some(&feed.description))
        && valid_optional_field(Some(&feed.kind))
        && valid_optional_field(Some(&feed.rate))
        && roko_core::wire_protocol::RelayEnvelope {
            seq: 0,
            ts: 0,
            room: feed.topic.clone(),
            msg_type: "feed_data".to_owned(),
            payload: Value::Null,
            publisher_id: None,
        }
        .validate()
        .is_ok()
        && feed
            .schema
            .as_ref()
            .is_none_or(|schema| json_size(schema) <= MAX_RELAY_FEED_SCHEMA_BYTES)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hello(agent_id: &str) -> AgentHello {
        AgentHello {
            agent_id: agent_id.to_owned(),
            name: None,
            capabilities: Vec::new(),
            rest_endpoint: None,
            card: None,
            card_uri: None,
            metadata: Value::Null,
        }
    }

    #[tokio::test]
    async fn duplicate_agent_supersedes_old_session_without_erasing_new_subscriptions() {
        let state = RelayState::new();
        let old_mailbox = state.bus.delivery_mailbox();
        let old = state
            .register_agent(hello("agent-a"), old_mailbox.clone())
            .expect("old registration");
        state
            .bus
            .try_subscribe("agent-a", "room:old")
            .expect("old subscription");

        let new_mailbox = state.bus.delivery_mailbox();
        let new = state
            .register_agent(hello("agent-a"), new_mailbox)
            .expect("replacement registration");
        let notice = old_mailbox.recv().await.expect("superseded notice");
        assert!(matches!(
            notice,
            RelayOutboundFrame::Superseded(SupersededNotice { agent_id, .. })
                if agent_id == "agent-a"
        ));
        assert!(!state.is_current_session("agent-a", old.session_id));
        assert!(state.is_current_session("agent-a", new.session_id));

        state
            .bus
            .try_subscribe("agent-a", "room:new")
            .expect("new subscription");
        state.unregister_agent("agent-a", old.session_id);
        assert!(state.bus.rooms_for("agent-a").contains("room:new"));
        assert!(state.is_current_session("agent-a", new.session_id));
    }

    #[test]
    fn duplicate_registration_cannot_interleave_with_session_bound_mutation() {
        let state = std::sync::Arc::new(RelayState::new());
        let old = state
            .register_agent(hello("agent-a"), state.bus.delivery_mailbox())
            .expect("old registration");
        let old_session = old.session_id;
        let (mutation_started_tx, mutation_started_rx) = std::sync::mpsc::channel();
        let (release_mutation_tx, release_mutation_rx) = std::sync::mpsc::channel();
        let mutation_state = std::sync::Arc::clone(&state);
        let mutation = std::thread::spawn(move || {
            mutation_state
                .with_current_session("agent-a", old_session, || {
                    mutation_started_tx.send(()).expect("signal mutation");
                    release_mutation_rx.recv().expect("release mutation");
                    assert!(mutation_state.update_card(
                        "agent-a",
                        serde_json::json!({"owner": "old-before-replacement"}),
                        None,
                    ));
                })
                .is_some()
        });
        mutation_started_rx.recv().expect("mutation acquired gate");

        let (replacement_tx, replacement_rx) = std::sync::mpsc::channel();
        let replacement_state = std::sync::Arc::clone(&state);
        let replacement = std::thread::spawn(move || {
            let registered = replacement_state
                .register_agent(hello("agent-a"), replacement_state.bus.delivery_mailbox())
                .expect("replacement registration");
            replacement_tx
                .send(registered.session_id)
                .expect("replacement result");
        });
        assert!(
            replacement_rx
                .recv_timeout(std::time::Duration::from_millis(20))
                .is_err(),
            "replacement must wait for the in-flight session mutation"
        );
        release_mutation_tx.send(()).expect("release old mutation");
        assert!(mutation.join().expect("mutation thread"));
        let new_session = replacement_rx.recv().expect("replacement completed");
        replacement.join().expect("replacement thread");

        assert!(state.is_current_session("agent-a", new_session));
        assert!(
            state
                .with_current_session("agent-a", old_session, || {
                    state.update_card(
                        "agent-a",
                        serde_json::json!({"owner": "stale-after-replacement"}),
                        None,
                    )
                })
                .is_none()
        );
        assert_eq!(
            state.card("agent-a"),
            Some(serde_json::json!({"owner": "old-before-replacement"}))
        );
    }

    #[tokio::test]
    async fn topic_fanout_shares_cached_wire_encoding_through_writer_boundary() {
        let state = RelayState::new();
        let first_mailbox = state.bus.delivery_mailbox();
        let second_mailbox = state.bus.delivery_mailbox();
        state
            .register_agent(hello("agent-a"), first_mailbox.clone())
            .expect("first registration");
        state
            .register_agent(hello("agent-b"), second_mailbox.clone())
            .expect("second registration");
        state
            .bus
            .try_subscribe_many("agent-a", &["room:a".to_owned()])
            .expect("first subscription");
        state
            .bus
            .try_subscribe_many("agent-b", &["room:a".to_owned()])
            .expect("second subscription");

        state
            .try_publish_topic(
                crate::protocol::TopicEnvelope::new(
                    "room:a",
                    "event",
                    serde_json::json!({"data": "x".repeat(64 * 1024)}),
                ),
                None,
            )
            .expect("publish shared frame");
        let first = first_mailbox.recv_shared().await.expect("first delivery");
        let second = second_mailbox.recv_shared().await.expect("second delivery");
        assert!(std::sync::Arc::ptr_eq(&first, &second));
        let first_wire = first.encoded().clone();
        let second_wire = second.encoded().clone();
        assert_eq!(first_wire.as_str().as_ptr(), second_wire.as_str().as_ptr());
        assert!(matches!(
            first.frame(),
            RelayOutboundFrame::TopicMessage { seq: 1, .. }
        ));
    }

    #[tokio::test]
    async fn pending_response_is_session_bound_and_removed_on_timeout() {
        let state = std::sync::Arc::new(RelayState::new());
        let mailbox = state.bus.delivery_mailbox();
        let registered = state
            .register_agent(hello("agent-a"), mailbox)
            .expect("registration");
        let pending = state
            .begin_message(RelayMessageRequest {
                agent_id: "agent-a".to_owned(),
                message: serde_json::json!({"prompt": "hi"}),
                timeout_ms: Some(1),
            })
            .expect("pending message");
        assert!(!state.resolve_response(
            "agent-b",
            registered.session_id,
            &pending.message_id,
            Ok(Value::Null),
        ));
        assert_eq!(
            pending.await_response().await,
            Err(AwaitMessageError::Timeout)
        );
        assert!(state.inner.read().pending.is_empty());
    }

    #[test]
    fn hello_card_and_feed_bounds_fail_closed_and_disconnect_cleans_maps() {
        let state = RelayState::new();
        for agent_id in ["a".repeat(257), "agent with spaces".to_owned()] {
            assert!(matches!(
                state.register_agent(hello(&agent_id), state.bus.delivery_mailbox()),
                Err(RegisterAgentError::InvalidHello)
            ));
        }
        state
            .register_agent(hello(&"\"".repeat(256)), state.bus.delivery_mailbox())
            .expect("escaped canonical agent id");
        let mut invalid = hello("agent-a");
        invalid.metadata = Value::String("x".repeat(MAX_RELAY_METADATA_BYTES + 1));
        assert!(matches!(
            state.register_agent(invalid, state.bus.delivery_mailbox()),
            Err(RegisterAgentError::InvalidHello)
        ));

        let registered = state
            .register_agent(hello("agent-a"), state.bus.delivery_mailbox())
            .expect("registration");
        assert!(!state.update_card(
            "agent-a",
            Value::String("x".repeat(MAX_RELAY_CARD_BYTES + 1)),
            None,
        ));
        assert!(state.update_card("agent-a", serde_json::json!({"ok": true}), None));

        for index in 0..MAX_FEEDS_PER_AGENT {
            assert!(state.register_feed(
                "agent-a",
                FeedDescriptor {
                    feed_id: format!("feed-{index}"),
                    topic: "room:feed".to_owned(),
                    name: "feed".to_owned(),
                    description: String::new(),
                    kind: String::new(),
                    rate: String::new(),
                    schema: None,
                },
            ));
        }
        assert!(!state.register_feed(
            "agent-a",
            FeedDescriptor {
                feed_id: "one-too-many".to_owned(),
                topic: "room:feed".to_owned(),
                name: "feed".to_owned(),
                description: String::new(),
                kind: String::new(),
                rate: String::new(),
                schema: None,
            },
        ));
        state.unregister_agent("agent-a", registered.session_id);
        let inner = state.inner.read();
        assert!(!inner.cards.contains_key("agent-a"));
        assert!(!inner.feeds.contains_key("agent-a"));
    }

    #[test]
    fn pending_and_workspace_maps_enforce_global_capacity() {
        let state = std::sync::Arc::new(RelayState::new());
        state
            .register_agent(hello("agent-a"), state.bus.delivery_mailbox())
            .expect("registration");
        {
            let mut inner = state.inner.write();
            let session_id = inner.agents["agent-a"].session_id;
            for index in 0..MAX_PENDING_RESPONSES {
                let (tx, _rx) = oneshot::channel();
                inner.pending.insert(
                    format!("message-{index}"),
                    PendingResponse {
                        agent_id: "agent-a".to_owned(),
                        session_id,
                        tx,
                    },
                );
            }
        }
        assert!(matches!(
            state.begin_message(RelayMessageRequest {
                agent_id: "agent-a".to_owned(),
                message: Value::Null,
                timeout_ms: None,
            }),
            Err(BeginMessageError::Capacity)
        ));

        let mut inner = state.inner.write();
        for index in 0..MAX_RELAY_WORKSPACES {
            inner.workspaces.insert(
                format!("workspace-{index}"),
                ConnectedWorkspace {
                    workspace_id: format!("workspace-{index}"),
                    name: None,
                    url: "https://example.test".to_owned(),
                    version: None,
                    owner_wallet: None,
                    agents_count: 0,
                    connected_at_ms: 0,
                    last_heartbeat_ms: 0,
                },
            );
        }
        drop(inner);
        assert!(!state.register_workspace(WorkspaceHello {
            workspace_id: "one-too-many".to_owned(),
            name: None,
            url: "https://example.test".to_owned(),
            version: None,
            owner_wallet: None,
            agents_count: 0,
        }));
    }

    #[test]
    fn websocket_admission_caps_include_pre_hello_and_events_connections() {
        let state = RelayState::new();
        let agent_permits = (0..MAX_RELAY_CONNECTIONS)
            .map(|_| state.try_admit_agent_socket().expect("agent permit"))
            .collect::<Vec<_>>();
        assert!(state.try_admit_agent_socket().is_none());
        drop(agent_permits);
        assert!(state.try_admit_agent_socket().is_some());

        let event_permits = (0..MAX_RELAY_EVENT_CONNECTIONS)
            .map(|_| state.try_admit_events_socket().expect("event permit"))
            .collect::<Vec<_>>();
        assert!(state.try_admit_events_socket().is_none());
        drop(event_permits);
        assert!(state.try_admit_events_socket().is_some());
    }

    #[test]
    fn relay_snapshot_materialization_is_incrementally_bounded() {
        let state = RelayState::new();
        let capabilities = (0..MAX_RELAY_CAPABILITIES)
            .map(|index| format!("cap-{index}-{}", "x".repeat(480)))
            .collect::<Vec<_>>();
        for index in 0..80 {
            let mut agent = hello(&format!("agent-{index:03}"));
            agent.capabilities = capabilities.clone();
            state
                .register_agent(agent, state.bus.delivery_mailbox())
                .expect("bounded agent registration");
        }

        let snapshot = state.relay_snapshot();
        assert!(json_size(&snapshot) <= MAX_RELAY_SNAPSHOT_BYTES);
        assert_eq!(snapshot["agent_count"], 80);
        assert_eq!(snapshot["truncated"], true);
        assert!(
            snapshot["agents"]
                .as_array()
                .is_some_and(|agents| agents.len() < 80)
        );
        assert!(json_size(&state.relay_snapshot_with_budget(512)) <= 512);
    }

    #[tokio::test]
    async fn low_delivery_budget_installs_a_bounded_snapshot() {
        let state = RelayState::try_with_config(TopicBusConfig {
            delivery_byte_capacity: crate::bus::MIN_RELAY_DELIVERY_BYTES,
            ..TopicBusConfig::default()
        })
        .expect("small valid delivery budget");
        let mut large = hello("large-agent");
        large.capabilities = (0..MAX_RELAY_CAPABILITIES)
            .map(|index| format!("cap-{index}-{}", "x".repeat(64)))
            .collect();
        state
            .register_agent(large, state.bus.delivery_mailbox())
            .expect("large bounded registration");
        let mailbox = state.bus.delivery_mailbox();
        mailbox.pause_topics();
        state
            .bus
            .subscribe_and_recover(
                "snapshot-consumer",
                &["room:a".to_owned()],
                Some(99),
                |budget| state.relay_snapshot_with_budget(budget),
                &mailbox,
            )
            .expect("bounded snapshot install");
        let snapshot = mailbox.recv().await.expect("snapshot frame");
        assert!(matches!(snapshot, RelayOutboundFrame::Snapshot(_)));
        assert!(
            serde_json::to_vec(&snapshot)
                .is_ok_and(|bytes| bytes.len() <= crate::bus::MIN_RELAY_DELIVERY_BYTES)
        );
    }
}
