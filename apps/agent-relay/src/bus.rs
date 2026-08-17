//! Topic-based pub/sub bus for the agent relay.
//!
//! Agents subscribe to topics via WebSocket frames. Published messages
//! are fanned out to all subscribers of the matching topic.

use crate::protocol::{RelayOutboundFrame, TopicEnvelope};
use axum::extract::ws::Utf8Bytes;
use parking_lot::{Mutex, RwLock};
use roko_core::wire_protocol::{
    BackpressureStrategy, EventBackpressureConfig, RelayRecoveryPolicy, SnapshotMessage,
    SupersededNotice,
};
use serde_json::Value;
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;
use tokio::sync::Notify;
use tokio::time::Instant;

pub const MAX_RELAY_RING_CAPACITY: usize = 65_536;
pub const MAX_RELAY_RING_BYTES: usize = 256 * 1024 * 1024;
pub const MAX_RELAY_DELIVERY_CAPACITY: usize = 4_096;
pub const MIN_RELAY_DELIVERY_CAPACITY: usize = 3;
pub const MAX_RELAY_DELIVERY_BYTES: usize = 64 * 1024 * 1024;
pub const MIN_RELAY_DELIVERY_BYTES: usize = 1024;
pub const MAX_RELAY_TOPICS: usize = 4_096;
pub const MAX_RELAY_SUBSCRIBERS_PER_TOPIC: usize = 4_096;
pub const MAX_RELAY_TOTAL_SUBSCRIPTIONS: usize = 16_384;

/// Configuration for the topic bus.
#[derive(Clone)]
pub struct TopicBusConfig {
    /// Max messages retained per topic for replay.
    pub ring_capacity: usize,
    /// Maximum serialized bytes retained by the global replay ring.
    pub ring_byte_capacity: usize,
    /// Maximum pending live frames retained for one connection.
    pub delivery_capacity: usize,
    /// Maximum logical serialized bytes queued by one subscriber.
    pub delivery_byte_capacity: usize,
    /// Per-event live delivery policies.
    pub backpressure: Vec<EventBackpressureConfig>,
}

impl Default for TopicBusConfig {
    fn default() -> Self {
        Self {
            ring_capacity: MAX_RELAY_RING_CAPACITY,
            ring_byte_capacity: 64 * 1024 * 1024,
            delivery_capacity: 256,
            delivery_byte_capacity: 8 * 1024 * 1024,
            backpressure: vec![
                EventBackpressureConfig {
                    event_type: "heartbeat".to_owned(),
                    strategy: BackpressureStrategy::Coalesce { interval_ms: 500 },
                },
                EventBackpressureConfig {
                    event_type: "output_chunk".to_owned(),
                    strategy: BackpressureStrategy::DropOldest { ring_size: 1_024 },
                },
                EventBackpressureConfig {
                    event_type: "gate_result".to_owned(),
                    strategy: BackpressureStrategy::Lossless,
                },
                EventBackpressureConfig {
                    event_type: "task_completed".to_owned(),
                    strategy: BackpressureStrategy::Lossless,
                },
            ],
        }
    }
}

impl TopicBusConfig {
    fn validate(&self) -> roko_core::Result<()> {
        if self.ring_capacity == 0 || self.ring_capacity > MAX_RELAY_RING_CAPACITY {
            return Err(roko_core::RokoError::invalid(format!(
                "relay ring capacity must be between 1 and {MAX_RELAY_RING_CAPACITY}"
            )));
        }
        if self.delivery_capacity < MIN_RELAY_DELIVERY_CAPACITY
            || self.delivery_capacity > MAX_RELAY_DELIVERY_CAPACITY
        {
            return Err(roko_core::RokoError::invalid(format!(
                "relay delivery capacity must be between {MIN_RELAY_DELIVERY_CAPACITY} and {MAX_RELAY_DELIVERY_CAPACITY}"
            )));
        }
        if self.ring_byte_capacity == 0 || self.ring_byte_capacity > MAX_RELAY_RING_BYTES {
            return Err(roko_core::RokoError::invalid(format!(
                "relay ring byte capacity must be between 1 and {MAX_RELAY_RING_BYTES}"
            )));
        }
        if self.delivery_byte_capacity < MIN_RELAY_DELIVERY_BYTES
            || self.delivery_byte_capacity > MAX_RELAY_DELIVERY_BYTES
        {
            return Err(roko_core::RokoError::invalid(format!(
                "relay delivery byte capacity must be between {MIN_RELAY_DELIVERY_BYTES} and {MAX_RELAY_DELIVERY_BYTES}"
            )));
        }
        for policy in &self.backpressure {
            policy.validate()?;
        }
        Ok(())
    }
}

/// Result of resolving a durable cursor against retained relay history.
#[derive(Debug, Clone)]
pub enum RelayRecovery {
    Replay {
        action: RelayRecoveryPolicy,
        envelopes: Vec<TopicEnvelope>,
    },
    Snapshot(SnapshotMessage),
}

enum RecoveryDecision {
    Replay {
        action: RelayRecoveryPolicy,
        envelopes: Vec<TopicEnvelope>,
    },
    Snapshot {
        seq: u64,
    },
}

impl RecoveryDecision {
    fn with_snapshot_state(self, state: Value) -> RelayRecovery {
        match self {
            Self::Replay { action, envelopes } => RelayRecovery::Replay { action, envelopes },
            Self::Snapshot { seq } => RelayRecovery::Snapshot(SnapshotMessage { seq, state }),
        }
    }
}

/// Topic-based pub/sub bus.
///
/// Thread-safe. Designed to be wrapped in `Arc` and shared with handler tasks.
pub struct TopicBus {
    /// topic → set of subscribed agent_ids.
    subscriptions: RwLock<HashMap<String, Vec<String>>>,
    /// Highest durable consumer cursor per (agent, room). Bounded by the
    /// global subscription cap.
    acknowledgements: RwLock<HashMap<(String, String), u64>>,
    /// One serialized global stream. Sequence assignment, ring append, and
    /// fanout all happen while this lock is held so concurrent publishers
    /// cannot expose sequence N+1 before sequence N.
    stream: Mutex<StreamState>,
    /// Max messages per topic ring.
    ring_capacity: usize,
    ring_byte_capacity: usize,
    delivery_capacity: usize,
    delivery_byte_capacity: usize,
    policies: Arc<HashMap<String, BackpressureStrategy>>,
}

struct StreamState {
    ring: VecDeque<TopicEnvelope>,
    ring_bytes: usize,
    next_seq: u64,
}

impl TopicBus {
    /// Create a new `TopicBus` with the given configuration.
    pub fn new(config: TopicBusConfig) -> Self {
        Self::try_new(config).expect("relay topic bus configuration must be valid")
    }

    pub fn try_new(config: TopicBusConfig) -> roko_core::Result<Self> {
        config.validate()?;
        let policies = config
            .backpressure
            .into_iter()
            .map(|policy| (policy.event_type, policy.strategy))
            .collect();
        Ok(Self {
            subscriptions: RwLock::new(HashMap::new()),
            acknowledgements: RwLock::new(HashMap::new()),
            stream: Mutex::new(StreamState {
                ring: VecDeque::with_capacity(config.ring_capacity),
                ring_bytes: 0,
                next_seq: 1,
            }),
            ring_capacity: config.ring_capacity,
            ring_byte_capacity: config.ring_byte_capacity,
            delivery_capacity: config.delivery_capacity,
            delivery_byte_capacity: config.delivery_byte_capacity,
            policies: Arc::new(policies),
        })
    }

    /// Subscribe an agent to a topic. Returns recent messages for replay.
    pub fn subscribe(&self, agent_id: &str, topic: &str) -> Vec<TopicEnvelope> {
        self.try_subscribe(agent_id, topic).unwrap_or_default()
    }

    /// Add a validated subscription while enforcing global and per-room caps.
    pub fn try_subscribe(
        &self,
        agent_id: &str,
        topic: &str,
    ) -> roko_core::Result<Vec<TopicEnvelope>> {
        self.try_subscribe_many(agent_id, std::slice::from_ref(&topic.to_owned()))?;
        Ok(self.peek_ring_limited(topic, self.delivery_capacity))
    }

    /// Atomically validate, reserve, and apply a subscription batch.
    pub fn try_subscribe_many(&self, agent_id: &str, topics: &[String]) -> roko_core::Result<()> {
        if agent_id.trim().is_empty() || agent_id.len() > 256 {
            return Err(roko_core::RokoError::invalid("invalid relay subscriber id"));
        }
        if topics.is_empty() || topics.len() > 64 {
            return Err(roko_core::RokoError::invalid(
                "relay subscription batch must contain 1-64 rooms",
            ));
        }
        let mut unique = HashSet::with_capacity(topics.len());
        for topic in topics {
            canonical_room(topic)?;
            unique.insert(topic.as_str());
        }

        let mut subs = self.subscriptions.write();
        let new_topics = unique
            .iter()
            .filter(|topic| !subs.contains_key(**topic))
            .count();
        if subs.len().saturating_add(new_topics) > MAX_RELAY_TOPICS {
            return Err(roko_core::RokoError::invalid("relay topic limit reached"));
        }
        let additions = unique
            .iter()
            .filter(|topic| {
                !subs
                    .get(**topic)
                    .is_some_and(|agents| agents.iter().any(|candidate| candidate == agent_id))
            })
            .count();
        let total: usize = subs.values().map(Vec::len).sum();
        if total.saturating_add(additions) > MAX_RELAY_TOTAL_SUBSCRIPTIONS {
            return Err(roko_core::RokoError::invalid(
                "relay total subscription limit reached",
            ));
        }
        for topic in &unique {
            let current = subs.get(*topic).map_or(0, Vec::len);
            let already_subscribed = subs
                .get(*topic)
                .is_some_and(|agents| agents.iter().any(|candidate| candidate == agent_id));
            if !already_subscribed && current >= MAX_RELAY_SUBSCRIBERS_PER_TOPIC {
                return Err(roko_core::RokoError::invalid(
                    "relay per-topic subscriber limit reached",
                ));
            }
        }
        for topic in unique {
            let agents = subs.entry(topic.to_owned()).or_default();
            if !agents.iter().any(|candidate| candidate == agent_id) {
                agents.push(agent_id.to_owned());
            }
        }
        Ok(())
    }

    /// Unsubscribe an agent from a topic.
    pub fn unsubscribe(&self, agent_id: &str, topic: &str) {
        let mut subs = self.subscriptions.write();
        if let Some(agents) = subs.get_mut(topic) {
            agents.retain(|id| id != agent_id);
            if agents.is_empty() {
                subs.remove(topic);
            }
        }
        self.acknowledgements
            .write()
            .remove(&(agent_id.to_owned(), topic.to_owned()));
    }

    pub fn try_unsubscribe(&self, agent_id: &str, topic: &str) -> roko_core::Result<()> {
        canonical_room(topic)?;
        self.unsubscribe(agent_id, topic);
        Ok(())
    }

    /// Remove all subscriptions for an agent (called on disconnect).
    pub fn unsubscribe_all(&self, agent_id: &str) {
        let mut subs = self.subscriptions.write();
        subs.retain(|_topic, agents| {
            agents.retain(|id| id != agent_id);
            !agents.is_empty()
        });
        self.acknowledgements
            .write()
            .retain(|(candidate, _), _| candidate != agent_id);
    }

    /// Advance one durable consumer cursor monotonically.
    pub fn acknowledge(&self, agent_id: &str, room: &str, seq: u64) -> roko_core::Result<()> {
        canonical_room(room)?;
        let subscribed = self
            .subscriptions
            .read()
            .get(room)
            .is_some_and(|agents| agents.iter().any(|candidate| candidate == agent_id));
        if !subscribed {
            return Err(roko_core::RokoError::invalid(
                "cannot acknowledge an unsubscribed relay room",
            ));
        }
        let latest = self.current_seq().saturating_sub(1);
        if seq > latest {
            return Err(roko_core::RokoError::invalid(
                "relay acknowledgement exceeds stream watermark",
            ));
        }
        let mut acknowledgements = self.acknowledgements.write();
        let cursor = acknowledgements
            .entry((agent_id.to_owned(), room.to_owned()))
            .or_default();
        *cursor = (*cursor).max(seq);
        Ok(())
    }

    #[must_use]
    pub fn acknowledged_cursor(&self, agent_id: &str, room: &str) -> Option<u64> {
        self.acknowledgements
            .read()
            .get(&(agent_id.to_owned(), room.to_owned()))
            .copied()
    }

    /// Publish a message to a topic.
    ///
    /// Assigns a monotonically increasing sequence number to the envelope,
    /// stores it in the ring buffer, and returns `(seq, subscriber_ids)`.
    /// The caller is responsible for actually delivering the frame to each agent.
    pub fn publish(&self, envelope: TopicEnvelope) -> (u64, Vec<String>) {
        self.try_publish(envelope)
            .expect("internally published relay envelope must be valid")
    }

    /// Validate and publish an untrusted client envelope.
    pub fn try_publish(&self, envelope: TopicEnvelope) -> roko_core::Result<(u64, Vec<String>)> {
        self.try_publish_with(envelope, |_seq, _subscribers, _envelope| {})
    }

    /// Publish and execute fanout under the same stream-ordering lock.
    pub fn try_publish_with<F>(
        &self,
        mut envelope: TopicEnvelope,
        fanout: F,
    ) -> roko_core::Result<(u64, Vec<String>)>
    where
        F: FnOnce(u64, &[String], &TopicEnvelope),
    {
        envelope.to_wire().validate()?;
        let mut stream = self.stream.lock();
        let seq = stream.next_seq;
        envelope.seq = seq;
        let envelope_bytes = serialized_topic_bytes(&envelope)?;
        if envelope_bytes > self.ring_byte_capacity {
            return Err(roko_core::RokoError::invalid(
                "relay envelope exceeds replay ring byte capacity",
            ));
        }
        stream.next_seq = stream.next_seq.saturating_add(1);
        while stream.ring.len() >= self.ring_capacity
            || stream.ring_bytes.saturating_add(envelope_bytes) > self.ring_byte_capacity
        {
            let Some(evicted) = stream.ring.pop_front() else {
                break;
            };
            stream.ring_bytes = stream
                .ring_bytes
                .saturating_sub(serialized_topic_bytes(&evicted).unwrap_or(0));
        }
        stream.ring.push_back(envelope.clone());
        stream.ring_bytes = stream.ring_bytes.saturating_add(envelope_bytes);

        let subscribers = {
            let subs = self.subscriptions.read();
            subs.get(&envelope.topic).cloned().unwrap_or_default()
        };
        fanout(seq, &subscribers, &envelope);
        Ok((seq, subscribers))
    }

    /// Read ring buffer contents for a topic without modifying subscriptions.
    ///
    /// Used by the HTTP metadata endpoint to inspect recent messages without
    /// creating a phantom subscription entry.
    pub fn peek_ring(&self, topic: &str) -> Vec<TopicEnvelope> {
        self.peek_ring_limited(topic, self.delivery_capacity)
    }

    /// Read at most `limit` newest room entries without cloning the full ring.
    pub fn peek_ring_limited(&self, topic: &str, limit: usize) -> Vec<TopicEnvelope> {
        let stream = self.stream.lock();
        let count_limit = limit.min(200).min(self.delivery_capacity);
        let mut entries = Vec::with_capacity(count_limit);
        let mut bytes = 0usize;
        for envelope in stream
            .ring
            .iter()
            .rev()
            .filter(|envelope| envelope.topic == topic)
        {
            if entries.len() >= count_limit {
                break;
            }
            let envelope_bytes = serialized_topic_bytes(envelope).unwrap_or(usize::MAX);
            if bytes.saturating_add(envelope_bytes) > self.delivery_byte_capacity {
                break;
            }
            bytes = bytes.saturating_add(envelope_bytes);
            entries.push(envelope.clone());
        }
        entries.reverse();
        entries
    }

    /// Rooms currently owned by one connected subscriber.
    pub fn rooms_for(&self, agent_id: &str) -> HashSet<String> {
        self.subscriptions
            .read()
            .iter()
            .filter(|(_, agents)| agents.iter().any(|candidate| candidate == agent_id))
            .map(|(topic, _)| topic.clone())
            .collect()
    }

    /// Select replay or snapshot without ever returning an incomplete gap.
    pub fn recover(&self, rooms: &HashSet<String>, last_seq: u64, state: Value) -> RelayRecovery {
        let stream = self.stream.lock();
        self.recovery_decision_locked(&stream, rooms, last_seq)
            .with_snapshot_state(state)
    }

    fn recovery_decision_locked(
        &self,
        stream: &StreamState,
        rooms: &HashSet<String>,
        last_seq: u64,
    ) -> RecoveryDecision {
        let latest = stream.next_seq.saturating_sub(1);
        if last_seq > latest {
            return RecoveryDecision::Snapshot { seq: latest };
        }
        if last_seq == latest {
            return RecoveryDecision::Replay {
                action: RelayRecoveryPolicy::Replay {
                    from_seq: last_seq,
                    to_seq: latest,
                },
                envelopes: Vec::new(),
            };
        }
        let requested = last_seq.saturating_add(1);
        let oldest = stream
            .ring
            .front()
            .map_or(stream.next_seq, |entry| entry.seq);
        if requested < oldest {
            return RecoveryDecision::Snapshot { seq: latest };
        }
        let completion_bytes = serialized_frame_bytes(&RelayOutboundFrame::ReplayComplete {
            from_seq: requested,
            to_seq: latest,
        })
        .unwrap_or(256);
        // Reserve room for the hello/subscription controls and replay terminal.
        let replay_count_limit = self.delivery_capacity.saturating_sub(3);
        let replay_byte_limit = self
            .delivery_byte_capacity
            .saturating_sub(completion_bytes.saturating_add(4 * 1024));
        let mut envelopes = Vec::new();
        let mut replay_bytes = 0usize;
        for entry in stream
            .ring
            .iter()
            .filter(|entry| entry.seq >= requested && rooms.contains(&entry.topic))
        {
            let entry_bytes = serialized_topic_bytes(entry).unwrap_or(usize::MAX);
            if envelopes.len() >= replay_count_limit
                || replay_bytes.saturating_add(entry_bytes) > replay_byte_limit
            {
                return RecoveryDecision::Snapshot { seq: latest };
            }
            replay_bytes = replay_bytes.saturating_add(entry_bytes);
            envelopes.push(entry.clone());
        }
        RecoveryDecision::Replay {
            action: RelayRecoveryPolicy::Replay {
                from_seq: requested,
                to_seq: latest,
            },
            envelopes,
        }
    }

    /// Atomically install rooms and recover their durable cursor before any
    /// newly published live frame can be fanned out to this connection.
    pub fn subscribe_and_recover<F>(
        &self,
        agent_id: &str,
        rooms: &[String],
        last_seq: Option<u64>,
        snapshot: F,
        mailbox: &RelayMailbox,
    ) -> roko_core::Result<RelayRecoveryPolicy>
    where
        F: FnOnce(usize) -> Value,
    {
        let stream = self.stream.lock();
        self.try_subscribe_many(agent_id, rooms)?;
        let Some(baseline) = last_seq else {
            if mailbox.install_recovery(Vec::new()).is_err() {
                self.unsubscribe_all(agent_id);
                return Err(roko_core::RokoError::invalid(
                    "relay mailbox closed during recovery",
                ));
            }
            let latest = stream.next_seq.saturating_sub(1);
            return Ok(RelayRecoveryPolicy::Replay {
                from_seq: latest,
                to_seq: latest,
            });
        };
        let room_set = rooms.iter().cloned().collect();
        let decision = self.recovery_decision_locked(&stream, &room_set, baseline);
        match decision {
            RecoveryDecision::Replay { action, envelopes } => self.install_subscription_recovery(
                agent_id,
                mailbox,
                RelayRecovery::Replay { action, envelopes },
            ),
            decision @ RecoveryDecision::Snapshot { .. } => {
                // The snapshot builder is output-budgeted. Keep the stream
                // barrier through materialization and mailbox installation so
                // its sequence cannot straddle a concurrent live publish.
                let snapshot_state_budget =
                    mailbox.inner.byte_capacity.saturating_sub(256).max(256);
                let state = snapshot(snapshot_state_budget);
                let recovery = decision.with_snapshot_state(state);
                self.install_subscription_recovery(agent_id, mailbox, recovery)
            }
        }
    }

    fn install_subscription_recovery(
        &self,
        agent_id: &str,
        mailbox: &RelayMailbox,
        recovery: RelayRecovery,
    ) -> roko_core::Result<RelayRecoveryPolicy> {
        let (action, frames) = recovery_frames(recovery);
        if mailbox.install_recovery(frames).is_err() {
            self.unsubscribe_all(agent_id);
            return Err(roko_core::RokoError::invalid(
                "relay mailbox closed during recovery",
            ));
        }
        Ok(action)
    }

    /// Create one bounded, policy-aware live delivery mailbox.
    pub fn delivery_mailbox(&self) -> RelayMailbox {
        RelayMailbox::new(
            self.delivery_capacity,
            self.delivery_byte_capacity,
            Arc::clone(&self.policies),
        )
    }

    /// Get all topics with their subscriber counts.
    pub fn topic_stats(&self) -> Vec<(String, usize)> {
        let subs = self.subscriptions.read();
        subs.iter()
            .map(|(topic, agents)| (topic.clone(), agents.len()))
            .collect()
    }

    /// Get subscriber count for a specific topic.
    pub fn subscriber_count(&self, topic: &str) -> usize {
        let subs = self.subscriptions.read();
        subs.get(topic).map(Vec::len).unwrap_or(0)
    }

    /// Current sequence number (for diagnostics).
    pub fn current_seq(&self) -> u64 {
        self.stream.lock().next_seq
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeliveryOutcome {
    Queued,
    Coalesced,
    Sampled,
    DroppedOldest,
    ResumeRequired,
}

#[derive(Debug)]
pub struct MailboxClosed;

#[derive(Debug)]
pub(crate) struct SharedRelayFrame {
    frame: RelayOutboundFrame,
    encoded: Utf8Bytes,
}

impl SharedRelayFrame {
    pub(crate) fn prepare(frame: RelayOutboundFrame) -> Result<Arc<Self>, MailboxClosed> {
        let encoded = serde_json::to_string(&frame).map_err(|_| MailboxClosed)?;
        Ok(Arc::new(Self {
            frame,
            encoded: encoded.into(),
        }))
    }

    pub(crate) fn frame(&self) -> &RelayOutboundFrame {
        &self.frame
    }

    pub(crate) fn encoded(&self) -> &Utf8Bytes {
        &self.encoded
    }

    fn bytes(&self) -> usize {
        self.encoded.len()
    }
}

struct QueuedFrame {
    frame: Arc<SharedRelayFrame>,
    ready_at: Instant,
    bytes: usize,
    lane: FrameLane,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum FrameLane {
    Control,
    Live,
    Recovery(u64),
}

struct MailboxState {
    queue: VecDeque<QueuedFrame>,
    queued_bytes: usize,
    recovery_generation: u64,
    sample_counts: HashMap<(String, String), u64>,
    resume_required: Option<u64>,
    topics_ready: bool,
    closed: bool,
}

impl Default for MailboxState {
    fn default() -> Self {
        Self {
            queue: VecDeque::new(),
            queued_bytes: 0,
            recovery_generation: 0,
            sample_counts: HashMap::new(),
            resume_required: None,
            topics_ready: true,
            closed: false,
        }
    }
}

struct MailboxInner {
    state: Mutex<MailboxState>,
    notify: Notify,
    capacity: usize,
    byte_capacity: usize,
    policies: Arc<HashMap<String, BackpressureStrategy>>,
}

/// Per-connection bounded live queue with deterministic overload behavior.
#[derive(Clone)]
pub struct RelayMailbox {
    inner: Arc<MailboxInner>,
}

impl RelayMailbox {
    fn new(
        capacity: usize,
        byte_capacity: usize,
        policies: Arc<HashMap<String, BackpressureStrategy>>,
    ) -> Self {
        Self {
            inner: Arc::new(MailboxInner {
                state: Mutex::new(MailboxState {
                    queue: VecDeque::with_capacity(capacity),
                    ..MailboxState::default()
                }),
                notify: Notify::new(),
                capacity,
                byte_capacity,
                policies,
            }),
        }
    }

    pub fn send(&self, frame: RelayOutboundFrame) -> Result<DeliveryOutcome, MailboxClosed> {
        self.send_shared(SharedRelayFrame::prepare(frame)?)
    }

    /// Queue a pre-encoded shared frame without cloning its payload for each subscriber.
    #[allow(clippy::too_many_lines)]
    pub(crate) fn send_shared(
        &self,
        frame: Arc<SharedRelayFrame>,
    ) -> Result<DeliveryOutcome, MailboxClosed> {
        let mut state = self.inner.state.lock();
        if state.closed {
            return Err(MailboxClosed);
        }
        let bytes = frame.bytes();
        if bytes > self.inner.byte_capacity {
            require_resume(&mut state, frame_seq(frame.frame()));
            return Ok(DeliveryOutcome::ResumeRequired);
        }
        let lane = if is_topic_frame(frame.frame()) {
            FrameLane::Live
        } else {
            FrameLane::Control
        };
        let strategy = frame_event_type(frame.frame())
            .and_then(|event_type| self.inner.policies.get(event_type))
            .cloned()
            .unwrap_or(BackpressureStrategy::Lossless);
        let now = Instant::now();
        let outcome = match strategy {
            BackpressureStrategy::Sample { every_nth } => {
                let key = frame_key(frame.frame()).unwrap_or_default();
                let count = state.sample_counts.entry(key).or_default();
                *count = count.saturating_add(1);
                if !(*count).is_multiple_of(every_nth) {
                    return Ok(DeliveryOutcome::Sampled);
                }
                enqueue_lossless(
                    &mut state,
                    frame,
                    now,
                    bytes,
                    lane,
                    self.inner.capacity,
                    self.inner.byte_capacity,
                )
            }
            BackpressureStrategy::Coalesce { interval_ms } => {
                let key = frame_key(frame.frame());
                if let Some(index) = state
                    .queue
                    .iter()
                    .rposition(|queued| matching_live_frame(queued, key.as_ref()))
                {
                    // Move the replacement to the tail. Replacing in-place can
                    // expose seq 3 before an intervening lossless seq 2.
                    remove_queued(&mut state, index);
                    if state.queue.len() >= self.inner.capacity
                        || state.queued_bytes.saturating_add(bytes) > self.inner.byte_capacity
                    {
                        require_resume(&mut state, frame_seq(frame.frame()));
                        return Ok(DeliveryOutcome::ResumeRequired);
                    }
                    state.queued_bytes = state.queued_bytes.saturating_add(bytes);
                    state.queue.push_back(QueuedFrame {
                        frame,
                        ready_at: now + std::time::Duration::from_millis(interval_ms),
                        bytes,
                        lane,
                    });
                    DeliveryOutcome::Coalesced
                } else if state.queue.len() >= self.inner.capacity
                    || state.queued_bytes.saturating_add(bytes) > self.inner.byte_capacity
                {
                    require_resume(&mut state, frame_seq(frame.frame()));
                    DeliveryOutcome::ResumeRequired
                } else {
                    state.queued_bytes = state.queued_bytes.saturating_add(bytes);
                    state.queue.push_back(QueuedFrame {
                        frame,
                        ready_at: now + std::time::Duration::from_millis(interval_ms),
                        bytes,
                        lane,
                    });
                    DeliveryOutcome::Queued
                }
            }
            BackpressureStrategy::DropOldest { ring_size } => {
                let key = frame_key(frame.frame());
                let matching = state
                    .queue
                    .iter()
                    .filter(|queued| matching_live_frame(queued, key.as_ref()))
                    .count();
                let mut dropped = false;
                if matching >= ring_size
                    && let Some(index) = state
                        .queue
                        .iter()
                        .position(|queued| matching_live_frame(queued, key.as_ref()))
                {
                    remove_queued(&mut state, index);
                    dropped = true;
                }
                while state.queue.len() >= self.inner.capacity
                    || state.queued_bytes.saturating_add(bytes) > self.inner.byte_capacity
                {
                    if let Some(index) = state
                        .queue
                        .iter()
                        .position(|queued| matching_live_frame(queued, key.as_ref()))
                    {
                        remove_queued(&mut state, index);
                        dropped = true;
                    } else {
                        require_resume(&mut state, frame_seq(frame.frame()));
                        return Ok(DeliveryOutcome::ResumeRequired);
                    }
                }
                state.queued_bytes = state.queued_bytes.saturating_add(bytes);
                state.queue.push_back(QueuedFrame {
                    frame,
                    ready_at: now,
                    bytes,
                    lane,
                });
                if dropped {
                    DeliveryOutcome::DroppedOldest
                } else {
                    DeliveryOutcome::Queued
                }
            }
            BackpressureStrategy::Lossless => enqueue_lossless(
                &mut state,
                frame,
                now,
                bytes,
                lane,
                self.inner.capacity,
                self.inner.byte_capacity,
            ),
        };
        drop(state);
        self.inner.notify.notify_one();
        Ok(outcome)
    }

    pub(crate) async fn recv_shared(&self) -> Option<Arc<SharedRelayFrame>> {
        loop {
            let notified = self.inner.notify.notified();
            let ready_at = {
                let mut state = self.inner.state.lock();
                if let Some(last_available_seq) = state.resume_required.take() {
                    state.closed = true;
                    return SharedRelayFrame::prepare(RelayOutboundFrame::ResumeRequired {
                        last_available_seq,
                    })
                    .ok();
                }
                if let Some(index) = state.queue.iter().position(|queued| {
                    queued.ready_at <= Instant::now()
                        && (state.topics_ready || !is_topic_frame(queued.frame.frame()))
                }) {
                    // A delayed coalesced frame must not head-of-line block a
                    // ready lossless frame. Dropping those older observations
                    // also prevents their lower sequence from being emitted
                    // after the ready frame.
                    for _ in 0..index {
                        if let Some(dropped) = state.queue.pop_front() {
                            state.queued_bytes = state.queued_bytes.saturating_sub(dropped.bytes);
                        }
                    }
                    return state.queue.pop_front().map(|queued| {
                        state.queued_bytes = state.queued_bytes.saturating_sub(queued.bytes);
                        queued.frame
                    });
                }
                if state.closed {
                    return None;
                }
                state
                    .queue
                    .iter()
                    .filter(|queued| state.topics_ready || !is_topic_frame(queued.frame.frame()))
                    .map(|queued| queued.ready_at)
                    .min()
            };
            if let Some(ready_at) = ready_at {
                tokio::select! {
                    () = tokio::time::sleep_until(ready_at) => {}
                    () = notified => {}
                }
            } else {
                notified.await;
            }
        }
    }

    /// Compatibility receive API for callers that require an owned frame.
    pub async fn recv(&self) -> Option<RelayOutboundFrame> {
        self.recv_shared().await.map(|frame| frame.frame.clone())
    }

    pub fn close(&self) {
        self.inner.state.lock().closed = true;
        self.inner.notify.notify_waiters();
    }

    /// Hold topic frames until subscription installation and cursor recovery
    /// are committed as one stream-ordered operation.
    pub fn pause_topics(&self) {
        self.inner.state.lock().topics_ready = false;
    }

    /// Replace queued live data with an ordered recovery batch. Callers hold
    /// the global stream lock, so subsequent live frames append after it.
    fn install_recovery(&self, frames: Vec<RelayOutboundFrame>) -> Result<(), MailboxClosed> {
        let mut state = self.inner.state.lock();
        if state.closed {
            return Err(MailboxClosed);
        }
        let encoded = frames
            .into_iter()
            .map(|frame| {
                SharedRelayFrame::prepare(frame).map(|frame| {
                    let bytes = frame.bytes();
                    (frame, bytes)
                })
            })
            .collect::<Result<Vec<_>, MailboxClosed>>()?;
        let control_count = state
            .queue
            .iter()
            .filter(|queued| queued.lane == FrameLane::Control)
            .count();
        let control_bytes = state
            .queue
            .iter()
            .filter(|queued| queued.lane == FrameLane::Control)
            .map(|queued| queued.bytes)
            .sum::<usize>();
        let recovery_bytes = encoded.iter().map(|(_, bytes)| *bytes).sum::<usize>();
        if control_count.saturating_add(encoded.len()) > self.inner.capacity
            || control_bytes.saturating_add(recovery_bytes) > self.inner.byte_capacity
        {
            return Err(MailboxClosed);
        }
        state
            .queue
            .retain(|queued| queued.lane == FrameLane::Control);
        state.queued_bytes = control_bytes;
        state.recovery_generation = state.recovery_generation.saturating_add(1);
        let generation = state.recovery_generation;
        state.resume_required = None;
        let now = Instant::now();
        for (frame, bytes) in encoded {
            state.queued_bytes = state.queued_bytes.saturating_add(bytes);
            state.queue.push_back(QueuedFrame {
                frame,
                ready_at: now,
                bytes,
                lane: FrameLane::Recovery(generation),
            });
        }
        state.topics_ready = true;
        drop(state);
        self.inner.notify.notify_waiters();
        Ok(())
    }

    /// Replace pending data with a terminal ownership notice and close writes.
    pub fn supersede(&self, notice: SupersededNotice) {
        let mut state = self.inner.state.lock();
        state.queue.clear();
        state.queued_bytes = 0;
        state.resume_required = None;
        let Ok(frame) = SharedRelayFrame::prepare(RelayOutboundFrame::Superseded(notice)) else {
            state.closed = true;
            drop(state);
            self.inner.notify.notify_waiters();
            return;
        };
        let bytes = frame.bytes();
        if bytes > self.inner.byte_capacity {
            state.closed = true;
            drop(state);
            self.inner.notify.notify_waiters();
            return;
        }
        state.queue.push_back(QueuedFrame {
            frame,
            ready_at: Instant::now(),
            bytes,
            lane: FrameLane::Control,
        });
        state.queued_bytes = bytes;
        state.closed = true;
        drop(state);
        self.inner.notify.notify_waiters();
    }
}

fn enqueue_lossless(
    state: &mut MailboxState,
    frame: Arc<SharedRelayFrame>,
    ready_at: Instant,
    bytes: usize,
    lane: FrameLane,
    capacity: usize,
    byte_capacity: usize,
) -> DeliveryOutcome {
    if state.queue.len() >= capacity || state.queued_bytes.saturating_add(bytes) > byte_capacity {
        require_resume(state, frame_seq(frame.frame()));
        return DeliveryOutcome::ResumeRequired;
    }
    state.queued_bytes = state.queued_bytes.saturating_add(bytes);
    state.queue.push_back(QueuedFrame {
        frame,
        ready_at,
        bytes,
        lane,
    });
    DeliveryOutcome::Queued
}

fn remove_queued(state: &mut MailboxState, index: usize) -> Option<QueuedFrame> {
    let removed = state.queue.remove(index)?;
    state.queued_bytes = state.queued_bytes.saturating_sub(removed.bytes);
    Some(removed)
}

fn require_resume(state: &mut MailboxState, seq: u64) {
    if state.resume_required.is_some() {
        return;
    }
    state.queue.clear();
    state.queued_bytes = 0;
    // This is a relay watermark, not an acknowledgement. The client resumes
    // from its own durable cursor.
    state.resume_required = Some(seq);
    state.closed = true;
}

fn topic_frame(envelope: TopicEnvelope) -> RelayOutboundFrame {
    RelayOutboundFrame::TopicMessage {
        topic: envelope.topic,
        msg_type: envelope.msg_type,
        payload: envelope.payload,
        publisher_id: envelope.publisher_id,
        seq: envelope.seq,
        timestamp_ms: envelope.timestamp_ms,
    }
}

fn recovery_frames(recovery: RelayRecovery) -> (RelayRecoveryPolicy, Vec<RelayOutboundFrame>) {
    match recovery {
        RelayRecovery::Replay { action, envelopes } => {
            let mut frames = envelopes.into_iter().map(topic_frame).collect::<Vec<_>>();
            let RelayRecoveryPolicy::Replay { from_seq, to_seq } = action else {
                unreachable!("replay result must carry replay action")
            };
            frames.push(RelayOutboundFrame::ReplayComplete { from_seq, to_seq });
            (RelayRecoveryPolicy::Replay { from_seq, to_seq }, frames)
        }
        RelayRecovery::Snapshot(snapshot) => (
            RelayRecoveryPolicy::Snapshot,
            vec![RelayOutboundFrame::Snapshot(snapshot)],
        ),
    }
}

fn frame_event_type(frame: &RelayOutboundFrame) -> Option<&str> {
    match frame {
        RelayOutboundFrame::TopicMessage { msg_type, .. } => Some(msg_type),
        _ => None,
    }
}

fn is_topic_frame(frame: &RelayOutboundFrame) -> bool {
    matches!(frame, RelayOutboundFrame::TopicMessage { .. })
}

fn frame_key(frame: &RelayOutboundFrame) -> Option<(String, String)> {
    match frame {
        RelayOutboundFrame::TopicMessage {
            topic, msg_type, ..
        } => Some((topic.clone(), msg_type.clone())),
        _ => None,
    }
}

fn matching_live_frame(queued: &QueuedFrame, key: Option<&(String, String)>) -> bool {
    queued.lane == FrameLane::Live && frame_key(queued.frame.frame()).as_ref() == key
}

fn frame_seq(frame: &RelayOutboundFrame) -> u64 {
    match frame {
        RelayOutboundFrame::TopicMessage { seq, .. } => *seq,
        _ => 0,
    }
}

fn serialized_frame_bytes(frame: &RelayOutboundFrame) -> Option<usize> {
    serde_json::to_vec(frame).ok().map(|bytes| bytes.len())
}

fn canonical_room(room: &str) -> roko_core::Result<()> {
    roko_core::wire_protocol::RelayEnvelope {
        seq: 0,
        ts: 0,
        room: room.to_owned(),
        msg_type: "subscription".to_owned(),
        payload: Value::Null,
        publisher_id: None,
    }
    .validate()
}

fn serialized_topic_bytes(envelope: &TopicEnvelope) -> roko_core::Result<usize> {
    serde_json::to_vec(envelope)
        .map(|bytes| bytes.len())
        .map_err(|error| {
            roko_core::RokoError::invalid(format!("relay envelope cannot be serialized: {error}"))
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn topic_frame(topic: &str, msg_type: &str, seq: u64, value: u64) -> RelayOutboundFrame {
        RelayOutboundFrame::TopicMessage {
            topic: topic.to_owned(),
            msg_type: msg_type.to_owned(),
            payload: json!(value),
            publisher_id: Some("publisher".to_owned()),
            seq,
            timestamp_ms: seq as i64,
        }
    }

    fn policy_bus(capacity: usize, event_type: &str, strategy: BackpressureStrategy) -> TopicBus {
        TopicBus::new(TopicBusConfig {
            ring_capacity: 16,
            ring_byte_capacity: 1024 * 1024,
            delivery_capacity: capacity,
            delivery_byte_capacity: 1024 * 1024,
            backpressure: vec![EventBackpressureConfig {
                event_type: event_type.to_owned(),
                strategy,
            }],
        })
    }

    #[test]
    fn subscribe_and_publish() {
        let bus = TopicBus::new(TopicBusConfig::default());

        // Subscribe agent — no replay yet.
        let replay = bus.subscribe("agent-1", "isfr:rates");
        assert!(replay.is_empty());

        // Publish one message.
        let envelope =
            TopicEnvelope::new("isfr:rates", "rate_update", serde_json::json!({"bps": 620}));
        let (seq, subscribers) = bus.publish(envelope);
        assert_eq!(seq, 1);
        assert_eq!(subscribers, vec!["agent-1"]);

        // New subscriber gets replay of existing message.
        let replay = bus.subscribe("agent-2", "isfr:rates");
        assert_eq!(replay.len(), 1);
        assert_eq!(replay[0].seq, 1);
    }

    #[test]
    fn unsubscribe_removes_from_fanout() {
        let bus = TopicBus::new(TopicBusConfig::default());
        bus.subscribe("agent-1", "chain:31337");
        bus.unsubscribe("agent-1", "chain:31337");

        let envelope = TopicEnvelope::new("chain:31337", "block", serde_json::json!({}));
        let (_seq, subscribers) = bus.publish(envelope);
        assert!(subscribers.is_empty());
    }

    #[test]
    fn ring_bounded() {
        let bus = TopicBus::new(TopicBusConfig {
            ring_capacity: 2,
            ..TopicBusConfig::default()
        });
        for i in 0..5_u64 {
            let env = TopicEnvelope::new("t", "x", serde_json::json!(i));
            bus.publish(env);
        }
        let replay = bus.subscribe("a", "t");
        assert_eq!(replay.len(), 2);
        // Seq starts at 1, so after 5 publishes the retained ones are seq 4 and 5.
        assert_eq!(replay[0].seq, 4);
        assert_eq!(replay[1].seq, 5);
    }

    #[test]
    fn legacy_subscription_replay_is_entry_and_byte_bounded() {
        let count_bounded = TopicBus::new(TopicBusConfig {
            ring_capacity: 8,
            delivery_capacity: 3,
            ..TopicBusConfig::default()
        });
        for value in 1..=4 {
            count_bounded.publish(TopicEnvelope::new("room:a", "event", json!(value)));
        }
        let replay = count_bounded
            .try_subscribe("agent-count", "room:a")
            .expect("bounded legacy replay");
        assert_eq!(
            replay.iter().map(|entry| entry.seq).collect::<Vec<_>>(),
            [2, 3, 4]
        );

        let template =
            TopicEnvelope::new("room:a", "event", json!({"data": "x".repeat(700)})).with_seq(1);
        let entry_bytes = serialized_topic_bytes(&template).expect("entry size");
        assert!(entry_bytes < MIN_RELAY_DELIVERY_BYTES);
        assert!(entry_bytes.saturating_mul(2) > MIN_RELAY_DELIVERY_BYTES);
        let byte_bounded = TopicBus::new(TopicBusConfig {
            ring_capacity: 8,
            delivery_capacity: 8,
            delivery_byte_capacity: MIN_RELAY_DELIVERY_BYTES,
            ..TopicBusConfig::default()
        });
        for _ in 0..3 {
            byte_bounded.publish(TopicEnvelope::new(
                "room:a",
                "event",
                json!({"data": "x".repeat(700)}),
            ));
        }
        let replay = byte_bounded
            .try_subscribe("agent-bytes", "room:a")
            .expect("byte-bounded legacy replay");
        assert_eq!(replay.len(), 1);
        assert_eq!(replay[0].seq, 3);
    }

    #[test]
    fn unsubscribe_all_on_disconnect() {
        let bus = TopicBus::new(TopicBusConfig::default());
        bus.subscribe("agent-1", "topic-a");
        bus.subscribe("agent-1", "topic-b");
        bus.unsubscribe_all("agent-1");
        assert_eq!(bus.subscriber_count("topic-a"), 0);
        assert_eq!(bus.subscriber_count("topic-b"), 0);
    }

    #[test]
    fn recovery_replays_available_gap_and_snapshots_evicted_gap() {
        let bus = TopicBus::new(TopicBusConfig {
            ring_capacity: 2,
            ..TopicBusConfig::default()
        });
        for value in 1..=5 {
            bus.publish(TopicEnvelope::new("room:a", "event", json!(value)));
        }
        let rooms = HashSet::from(["room:a".to_owned()]);
        match bus.recover(&rooms, 4, json!({ "materialized": 5 })) {
            RelayRecovery::Replay { action, envelopes } => {
                assert_eq!(
                    action,
                    RelayRecoveryPolicy::Replay {
                        from_seq: 5,
                        to_seq: 5
                    }
                );
                assert_eq!(envelopes.len(), 1);
                assert_eq!(envelopes[0].seq, 5);
            }
            RelayRecovery::Snapshot(_) => panic!("available cursor unexpectedly snapshotted"),
        }
        match bus.recover(&rooms, 1, json!({ "materialized": 5 })) {
            RelayRecovery::Snapshot(snapshot) => {
                assert_eq!(snapshot.seq, 5);
                assert_eq!(snapshot.state["materialized"], 5);
            }
            RelayRecovery::Replay { .. } => panic!("evicted cursor unexpectedly replayed"),
        }
    }

    #[test]
    fn replay_larger_than_live_mailbox_falls_back_to_snapshot() {
        let bus = TopicBus::new(TopicBusConfig {
            ring_capacity: 8,
            ring_byte_capacity: 1024 * 1024,
            delivery_capacity: MIN_RELAY_DELIVERY_CAPACITY,
            delivery_byte_capacity: 1024 * 1024,
            backpressure: Vec::new(),
        });
        for value in 1..=3 {
            bus.publish(TopicEnvelope::new("room:a", "event", json!(value)));
        }
        let rooms = HashSet::from(["room:a".to_owned()]);
        assert!(matches!(
            bus.recover(&rooms, 0, json!({ "materialized": 3 })),
            RelayRecovery::Snapshot(SnapshotMessage { seq: 3, .. })
        ));
    }

    #[test]
    fn replay_capacity_is_global_across_rooms() {
        let bus = TopicBus::new(TopicBusConfig {
            ring_capacity: 2,
            ..TopicBusConfig::default()
        });
        bus.publish(TopicEnvelope::new("room:a", "event", json!(1)));
        bus.publish(TopicEnvelope::new("room:b", "event", json!(2)));
        bus.publish(TopicEnvelope::new("room:c", "event", json!(3)));
        assert!(bus.peek_ring("room:a").is_empty());
        assert_eq!(bus.peek_ring("room:b").len(), 1);
        assert_eq!(bus.peek_ring("room:c").len(), 1);
    }

    #[tokio::test]
    async fn coalesce_delays_and_delivers_only_the_latest_live_frame() {
        let bus = policy_bus(
            4,
            "heartbeat",
            BackpressureStrategy::Coalesce { interval_ms: 20 },
        );
        let mailbox = bus.delivery_mailbox();
        assert_eq!(
            mailbox
                .send(topic_frame("agent:a", "heartbeat", 1, 1))
                .expect("queue heartbeat"),
            DeliveryOutcome::Queued
        );
        assert_eq!(
            mailbox
                .send(topic_frame("agent:a", "heartbeat", 2, 2))
                .expect("coalesce heartbeat"),
            DeliveryOutcome::Coalesced
        );
        assert!(
            tokio::time::timeout(std::time::Duration::from_millis(2), mailbox.recv())
                .await
                .is_err()
        );
        let delivered = tokio::time::timeout(std::time::Duration::from_millis(100), mailbox.recv())
            .await
            .expect("coalesce interval")
            .expect("latest heartbeat");
        assert_eq!(frame_seq(&delivered), 2);
    }

    #[tokio::test]
    async fn live_coalesce_cannot_replace_recovery_or_invalidate_replay_completion() {
        let bus = policy_bus(
            4,
            "heartbeat",
            BackpressureStrategy::Coalesce { interval_ms: 20 },
        );
        let mailbox = bus.delivery_mailbox();
        mailbox
            .install_recovery(vec![
                topic_frame("agent:a", "heartbeat", 1, 1),
                RelayOutboundFrame::ReplayComplete {
                    from_seq: 1,
                    to_seq: 1,
                },
            ])
            .expect("install recovery");

        assert_eq!(
            mailbox
                .send(topic_frame("agent:a", "heartbeat", 2, 2))
                .expect("queue live heartbeat"),
            DeliveryOutcome::Queued
        );
        assert_eq!(frame_seq(&mailbox.recv().await.expect("recovery frame")), 1);
        assert!(matches!(
            mailbox.recv().await,
            Some(RelayOutboundFrame::ReplayComplete {
                from_seq: 1,
                to_seq: 1
            })
        ));
        let delivered = tokio::time::timeout(std::time::Duration::from_millis(100), mailbox.recv())
            .await
            .expect("coalesce interval")
            .expect("live heartbeat");
        assert_eq!(frame_seq(&delivered), 2);
    }

    #[tokio::test]
    async fn drop_oldest_evicts_real_queued_frames() {
        let bus = policy_bus(
            4,
            "output_chunk",
            BackpressureStrategy::DropOldest { ring_size: 2 },
        );
        let mailbox = bus.delivery_mailbox();
        mailbox
            .send(topic_frame("agent:a", "output_chunk", 1, 1))
            .expect("first chunk");
        mailbox
            .send(topic_frame("agent:a", "output_chunk", 2, 2))
            .expect("second chunk");
        assert_eq!(
            mailbox
                .send(topic_frame("agent:a", "output_chunk", 3, 3))
                .expect("third chunk"),
            DeliveryOutcome::DroppedOldest
        );
        assert_eq!(
            frame_seq(&mailbox.recv().await.expect("second retained")),
            2
        );
        assert_eq!(frame_seq(&mailbox.recv().await.expect("third retained")), 3);
    }

    #[tokio::test]
    async fn live_drop_oldest_cannot_evict_recovery_or_invalidate_replay_completion() {
        let bus = policy_bus(
            3,
            "output_chunk",
            BackpressureStrategy::DropOldest { ring_size: 1 },
        );
        let mailbox = bus.delivery_mailbox();
        mailbox
            .install_recovery(vec![
                topic_frame("agent:a", "output_chunk", 1, 1),
                RelayOutboundFrame::ReplayComplete {
                    from_seq: 1,
                    to_seq: 1,
                },
            ])
            .expect("install recovery");

        assert_eq!(
            mailbox
                .send(topic_frame("agent:a", "output_chunk", 2, 2))
                .expect("queue first live chunk"),
            DeliveryOutcome::Queued
        );
        assert_eq!(
            mailbox
                .send(topic_frame("agent:a", "output_chunk", 3, 3))
                .expect("replace only live chunk"),
            DeliveryOutcome::DroppedOldest
        );
        assert_eq!(frame_seq(&mailbox.recv().await.expect("recovery frame")), 1);
        assert!(matches!(
            mailbox.recv().await,
            Some(RelayOutboundFrame::ReplayComplete {
                from_seq: 1,
                to_seq: 1
            })
        ));
        assert_eq!(
            frame_seq(&mailbox.recv().await.expect("latest live chunk")),
            3
        );
    }

    #[tokio::test]
    async fn sample_delivers_exactly_every_nth_live_frame() {
        let bus = policy_bus(
            4,
            "feed_data",
            BackpressureStrategy::Sample { every_nth: 2 },
        );
        let mailbox = bus.delivery_mailbox();
        for seq in 1..=4 {
            mailbox
                .send(topic_frame("feed:a", "feed_data", seq, seq))
                .expect("sample frame");
        }
        assert_eq!(frame_seq(&mailbox.recv().await.expect("second sample")), 2);
        assert_eq!(frame_seq(&mailbox.recv().await.expect("fourth sample")), 4);
    }

    #[tokio::test]
    async fn lossless_overflow_forces_resume_required_and_disconnect() {
        let bus = policy_bus(3, "gate_result", BackpressureStrategy::Lossless);
        let mailbox = bus.delivery_mailbox();
        mailbox
            .send(topic_frame("plan:a", "gate_result", 1, 1))
            .expect("first lossless");
        mailbox
            .send(topic_frame("plan:a", "gate_result", 2, 2))
            .expect("second lossless");
        mailbox
            .send(topic_frame("plan:a", "gate_result", 3, 3))
            .expect("third lossless");
        assert_eq!(
            mailbox
                .send(topic_frame("plan:a", "gate_result", 4, 4))
                .expect("resume required"),
            DeliveryOutcome::ResumeRequired
        );
        assert!(matches!(
            mailbox.recv().await,
            Some(RelayOutboundFrame::ResumeRequired {
                last_available_seq: 4
            })
        ));
        assert!(mailbox.recv().await.is_none());
    }

    #[tokio::test]
    async fn atomic_subscribe_recovery_orders_replay_completion_before_live() {
        let bus = TopicBus::new(TopicBusConfig {
            ring_capacity: 8,
            ring_byte_capacity: 1024 * 1024,
            delivery_capacity: 8,
            delivery_byte_capacity: 1024 * 1024,
            backpressure: Vec::new(),
        });
        bus.publish(TopicEnvelope::new("room:a", "event", json!(1)));
        let mailbox = bus.delivery_mailbox();
        mailbox.pause_topics();
        bus.subscribe_and_recover(
            "agent-a",
            &["room:a".to_owned()],
            Some(0),
            |_| json!({}),
            &mailbox,
        )
        .expect("atomic recovery");
        bus.try_publish_with(
            TopicEnvelope::new("room:a", "event", json!(2)),
            |seq, subscribers, envelope| {
                assert_eq!(subscribers, ["agent-a"]);
                mailbox
                    .send(topic_frame(&envelope.topic, &envelope.msg_type, seq, 2))
                    .expect("live frame");
            },
        )
        .expect("publish after barrier");

        assert_eq!(frame_seq(&mailbox.recv().await.expect("replay")), 1);
        assert!(matches!(
            mailbox.recv().await,
            Some(RelayOutboundFrame::ReplayComplete {
                from_seq: 1,
                to_seq: 1
            })
        ));
        assert_eq!(frame_seq(&mailbox.recv().await.expect("live")), 2);
    }

    #[tokio::test]
    async fn snapshot_is_lazy_and_built_inside_the_stream_barrier() {
        let bus = TopicBus::new(TopicBusConfig {
            ring_capacity: 8,
            ring_byte_capacity: 1024 * 1024,
            delivery_capacity: 8,
            delivery_byte_capacity: 1024 * 1024,
            backpressure: Vec::new(),
        });
        bus.publish(TopicEnvelope::new("room:a", "event", json!(1)));

        let replay_mailbox = bus.delivery_mailbox();
        replay_mailbox.pause_topics();
        bus.subscribe_and_recover(
            "agent-replay",
            &["room:a".to_owned()],
            Some(0),
            |_| panic!("replay must not construct a snapshot"),
            &replay_mailbox,
        )
        .expect("replay recovery");

        let current_mailbox = bus.delivery_mailbox();
        current_mailbox.pause_topics();
        bus.subscribe_and_recover(
            "agent-current",
            &["room:a".to_owned()],
            Some(1),
            |_| panic!("no-op recovery must not construct a snapshot"),
            &current_mailbox,
        )
        .expect("no-op recovery");

        let snapshot_mailbox = bus.delivery_mailbox();
        snapshot_mailbox.pause_topics();
        bus.subscribe_and_recover(
            "agent-snapshot",
            &["room:a".to_owned()],
            Some(99),
            |_| {
                assert!(
                    bus.stream.try_lock().is_none(),
                    "bounded snapshot materialization must retain the stream lock"
                );
                json!({"bounded": true})
            },
            &snapshot_mailbox,
        )
        .expect("snapshot recovery");
        assert!(matches!(
            snapshot_mailbox.recv().await,
            Some(RelayOutboundFrame::Snapshot(SnapshotMessage { seq: 1, .. }))
        ));
    }

    #[tokio::test]
    async fn snapshot_install_barrier_precedes_concurrent_live_publish() {
        let bus = Arc::new(TopicBus::new(TopicBusConfig {
            ring_capacity: 8,
            ring_byte_capacity: 1024 * 1024,
            delivery_capacity: 8,
            delivery_byte_capacity: 1024 * 1024,
            backpressure: Vec::new(),
        }));
        let mailbox = bus.delivery_mailbox();
        mailbox.pause_topics();
        let (snapshot_started_tx, snapshot_started_rx) = std::sync::mpsc::channel();
        let (release_snapshot_tx, release_snapshot_rx) = std::sync::mpsc::channel();
        let recovery_bus = Arc::clone(&bus);
        let recovery_mailbox = mailbox.clone();
        let recovery = std::thread::spawn(move || {
            recovery_bus
                .subscribe_and_recover(
                    "agent-snapshot",
                    &["room:a".to_owned()],
                    Some(99),
                    |_| {
                        snapshot_started_tx.send(()).expect("snapshot started");
                        release_snapshot_rx.recv().expect("release snapshot");
                        json!({"bounded": true})
                    },
                    &recovery_mailbox,
                )
                .expect("snapshot recovery");
        });
        snapshot_started_rx.recv().expect("snapshot holds barrier");

        let (publish_done_tx, publish_done_rx) = std::sync::mpsc::channel();
        let publish_bus = Arc::clone(&bus);
        let publish_mailbox = mailbox.clone();
        let publish = std::thread::spawn(move || {
            publish_bus
                .try_publish_with(
                    TopicEnvelope::new("room:a", "event", json!(1)),
                    |seq, _, envelope| {
                        publish_mailbox
                            .send(topic_frame(&envelope.topic, &envelope.msg_type, seq, 1))
                            .expect("queue live frame");
                    },
                )
                .expect("publish live frame");
            publish_done_tx.send(()).expect("publish completed");
        });
        assert!(
            publish_done_rx
                .recv_timeout(std::time::Duration::from_millis(20))
                .is_err(),
            "publisher must wait for snapshot installation"
        );
        release_snapshot_tx.send(()).expect("release snapshot");
        recovery.join().expect("recovery thread");
        publish_done_rx.recv().expect("publish after snapshot");
        publish.join().expect("publish thread");

        assert!(matches!(
            mailbox.recv().await,
            Some(RelayOutboundFrame::Snapshot(SnapshotMessage { seq: 0, .. }))
        ));
        assert_eq!(
            frame_seq(&mailbox.recv().await.expect("live after snapshot")),
            1
        );
    }

    #[tokio::test]
    async fn quiet_room_gets_authoritative_global_replay_completion() {
        let bus = TopicBus::new(TopicBusConfig::default());
        bus.publish(TopicEnvelope::new("room:busy", "event", json!(1)));
        let mailbox = bus.delivery_mailbox();
        mailbox.pause_topics();
        bus.subscribe_and_recover(
            "agent-quiet",
            &["room:quiet".to_owned()],
            Some(0),
            |_| json!({}),
            &mailbox,
        )
        .expect("quiet recovery");
        assert!(matches!(
            mailbox.recv().await,
            Some(RelayOutboundFrame::ReplayComplete { to_seq: 1, .. })
        ));
    }

    #[test]
    fn future_cursor_requires_snapshot() {
        let bus = TopicBus::new(TopicBusConfig::default());
        bus.publish(TopicEnvelope::new("room:a", "event", json!(1)));
        assert!(matches!(
            bus.recover(&HashSet::from(["room:a".to_owned()]), 99, json!({})),
            RelayRecovery::Snapshot(SnapshotMessage { seq: 1, .. })
        ));
    }

    #[test]
    fn subscription_batch_is_all_or_nothing() {
        let bus = TopicBus::new(TopicBusConfig::default());
        assert!(
            bus.try_subscribe_many("agent-a", &["room:valid".to_owned(), " ".to_owned()])
                .is_err()
        );
        assert!(bus.rooms_for("agent-a").is_empty());
    }

    #[tokio::test]
    async fn drop_oldest_never_evicts_a_lossless_frame() {
        let bus = TopicBus::new(TopicBusConfig {
            ring_capacity: 8,
            ring_byte_capacity: 1024 * 1024,
            delivery_capacity: 3,
            delivery_byte_capacity: 1024 * 1024,
            backpressure: vec![EventBackpressureConfig {
                event_type: "output_chunk".to_owned(),
                strategy: BackpressureStrategy::DropOldest { ring_size: 2 },
            }],
        });
        let mailbox = bus.delivery_mailbox();
        mailbox
            .send(topic_frame("room:a", "gate_result", 1, 1))
            .expect("first lossless");
        mailbox
            .send(topic_frame("room:a", "other_lossless", 2, 2))
            .expect("second lossless");
        mailbox
            .send(topic_frame("room:a", "third_lossless", 3, 3))
            .expect("third lossless");
        assert_eq!(
            mailbox
                .send(topic_frame("room:a", "output_chunk", 4, 4))
                .expect("overflow outcome"),
            DeliveryOutcome::ResumeRequired
        );
        assert!(matches!(
            mailbox.recv().await,
            Some(RelayOutboundFrame::ResumeRequired {
                last_available_seq: 4
            })
        ));
    }

    #[tokio::test]
    async fn coalescing_never_reorders_around_lossless_delivery() {
        let bus = policy_bus(
            4,
            "heartbeat",
            BackpressureStrategy::Coalesce { interval_ms: 50 },
        );
        let mailbox = bus.delivery_mailbox();
        mailbox
            .send(topic_frame("room:a", "heartbeat", 1, 1))
            .expect("heartbeat");
        mailbox
            .send(topic_frame("room:a", "gate_result", 2, 2))
            .expect("lossless");
        assert_eq!(frame_seq(&mailbox.recv().await.expect("lossless")), 2);
        assert!(
            tokio::time::timeout(std::time::Duration::from_millis(60), mailbox.recv())
                .await
                .is_err()
        );
    }

    #[test]
    fn replay_ring_enforces_aggregate_bytes_and_rejects_oversized_entry() {
        let template =
            TopicEnvelope::new("room:a", "event", json!({"data": "x".repeat(64)})).with_seq(1);
        let entry_bytes = serialized_topic_bytes(&template).expect("entry size");
        let bus = TopicBus::new(TopicBusConfig {
            ring_capacity: 100,
            ring_byte_capacity: entry_bytes * 2 + 8,
            ..TopicBusConfig::default()
        });
        for _ in 0..3 {
            bus.try_publish(TopicEnvelope::new(
                "room:a",
                "event",
                json!({"data": "x".repeat(64)}),
            ))
            .expect("bounded publish");
        }
        let stream = bus.stream.lock();
        assert!(stream.ring.len() <= 2);
        assert!(stream.ring_bytes <= bus.ring_byte_capacity);
        drop(stream);

        let too_small = TopicBus::new(TopicBusConfig {
            ring_capacity: 8,
            ring_byte_capacity: entry_bytes - 1,
            ..TopicBusConfig::default()
        });
        assert!(
            too_small
                .try_publish(TopicEnvelope::new(
                    "room:a",
                    "event",
                    json!({"data": "x".repeat(64)}),
                ))
                .is_err()
        );
        assert_eq!(too_small.current_seq(), 1);
        assert!(too_small.peek_ring("room:a").is_empty());
    }

    #[test]
    fn byte_heavy_replay_snapshots_before_cloning_mailbox_budget() {
        let bus = TopicBus::new(TopicBusConfig {
            ring_capacity: 8,
            ring_byte_capacity: 1024 * 1024,
            delivery_capacity: 8,
            delivery_byte_capacity: 10 * 1024,
            backpressure: Vec::new(),
        });
        for value in 0..2 {
            bus.publish(TopicEnvelope::new(
                "room:a",
                "event",
                json!({"value": value, "data": "x".repeat(4 * 1024)}),
            ));
        }
        assert!(matches!(
            bus.recover(&HashSet::from(["room:a".to_owned()]), 0, json!({})),
            RelayRecovery::Snapshot(SnapshotMessage { seq: 2, .. })
        ));
    }

    #[tokio::test]
    async fn recovery_generation_replaces_stale_lane_and_preserves_control() {
        let bus = TopicBus::new(TopicBusConfig {
            delivery_capacity: 8,
            delivery_byte_capacity: 1024 * 1024,
            ..TopicBusConfig::default()
        });
        let mailbox = bus.delivery_mailbox();
        mailbox
            .send(RelayOutboundFrame::Ack {
                event: "hello".to_owned(),
            })
            .expect("control");
        mailbox
            .install_recovery(vec![RelayOutboundFrame::ReplayComplete {
                from_seq: 1,
                to_seq: 1,
            }])
            .expect("first recovery");
        mailbox
            .install_recovery(vec![RelayOutboundFrame::Snapshot(SnapshotMessage {
                seq: 2,
                state: json!({"latest": true}),
            })])
            .expect("replacement recovery");
        assert!(matches!(
            mailbox.recv().await,
            Some(RelayOutboundFrame::Ack { event }) if event == "hello"
        ));
        assert!(matches!(
            mailbox.recv().await,
            Some(RelayOutboundFrame::Snapshot(SnapshotMessage { seq: 2, .. }))
        ));
        assert!(
            tokio::time::timeout(std::time::Duration::from_millis(5), mailbox.recv())
                .await
                .is_err()
        );
    }

    #[tokio::test]
    async fn failed_recovery_rolls_back_rooms_and_can_retry() {
        let bus = TopicBus::new(TopicBusConfig {
            ring_capacity: 8,
            ring_byte_capacity: 1024 * 1024,
            delivery_capacity: 3,
            delivery_byte_capacity: 1024 * 1024,
            backpressure: Vec::new(),
        });
        bus.publish(TopicEnvelope::new("room:a", "event", json!(1)));
        let mailbox = bus.delivery_mailbox();
        mailbox.pause_topics();
        for event in ["hello", "card", "feed"] {
            mailbox
                .send(RelayOutboundFrame::Ack {
                    event: event.to_owned(),
                })
                .expect("control");
        }
        assert!(
            bus.subscribe_and_recover(
                "agent-a",
                &["room:a".to_owned()],
                Some(0),
                |_| json!({}),
                &mailbox,
            )
            .is_err()
        );
        assert!(bus.rooms_for("agent-a").is_empty());
        for _ in 0..3 {
            assert!(matches!(
                mailbox.recv().await,
                Some(RelayOutboundFrame::Ack { .. })
            ));
        }
        bus.subscribe_and_recover(
            "agent-a",
            &["room:a".to_owned()],
            Some(0),
            |_| json!({}),
            &mailbox,
        )
        .expect("retry after rollback");
        assert!(bus.rooms_for("agent-a").contains("room:a"));
    }

    #[tokio::test]
    async fn shared_delivery_keeps_cached_encoding_through_writer_dequeue() {
        let bus = TopicBus::new(TopicBusConfig::default());
        let first = bus.delivery_mailbox();
        let second = bus.delivery_mailbox();
        let frame = SharedRelayFrame::prepare(topic_frame("room:a", "event", 1, 1))
            .expect("prepare canonical frame");
        first.send_shared(Arc::clone(&frame)).expect("first queue");
        second
            .send_shared(Arc::clone(&frame))
            .expect("second queue");
        let first_received = first.recv_shared().await.expect("first receive");
        let second_received = second.recv_shared().await.expect("second receive");
        assert!(Arc::ptr_eq(&frame, &first_received));
        assert!(Arc::ptr_eq(&frame, &second_received));
        let first_wire = first_received.encoded().clone();
        let second_wire = second_received.encoded().clone();
        assert_eq!(first_wire.as_str().as_ptr(), second_wire.as_str().as_ptr());
        assert_eq!(
            serde_json::from_str::<RelayOutboundFrame>(&first_wire).expect("cached wire frame"),
            *frame.frame()
        );
    }

    #[tokio::test]
    async fn mailbox_enforces_aggregate_serialized_byte_budget() {
        let mut frame = topic_frame("room:a", "gate_result", 1, 1);
        let RelayOutboundFrame::TopicMessage { payload, .. } = &mut frame else {
            unreachable!("topic helper")
        };
        *payload = json!({"data": "x".repeat(700)});
        let frame_bytes = serialized_frame_bytes(&frame).expect("frame size");
        assert!(frame_bytes < MIN_RELAY_DELIVERY_BYTES);
        assert!(frame_bytes.saturating_mul(2) > MIN_RELAY_DELIVERY_BYTES);
        let bus = TopicBus::new(TopicBusConfig {
            delivery_capacity: 8,
            delivery_byte_capacity: MIN_RELAY_DELIVERY_BYTES,
            ..TopicBusConfig::default()
        });
        let mailbox = bus.delivery_mailbox();
        mailbox.send(frame).expect("first frame");
        let mut second = topic_frame("room:a", "gate_result", 2, 2);
        let RelayOutboundFrame::TopicMessage { payload, .. } = &mut second else {
            unreachable!("topic helper")
        };
        *payload = json!({"data": "x".repeat(700)});
        assert_eq!(
            mailbox.send(second).expect("second outcome"),
            DeliveryOutcome::ResumeRequired
        );
        assert!(matches!(
            mailbox.recv().await,
            Some(RelayOutboundFrame::ResumeRequired {
                last_available_seq: 2
            })
        ));
    }

    #[test]
    fn invalid_or_excessive_bus_bounds_fail_closed() {
        for config in [
            TopicBusConfig {
                ring_capacity: 0,
                ..TopicBusConfig::default()
            },
            TopicBusConfig {
                delivery_capacity: MAX_RELAY_DELIVERY_CAPACITY + 1,
                ..TopicBusConfig::default()
            },
            TopicBusConfig {
                delivery_capacity: MIN_RELAY_DELIVERY_CAPACITY - 1,
                ..TopicBusConfig::default()
            },
            TopicBusConfig {
                delivery_byte_capacity: MIN_RELAY_DELIVERY_BYTES - 1,
                ..TopicBusConfig::default()
            },
        ] {
            assert!(TopicBus::try_new(config).is_err());
        }

        let bus = TopicBus::new(TopicBusConfig {
            ring_capacity: 8,
            ring_byte_capacity: 1024 * 1024,
            delivery_capacity: MIN_RELAY_DELIVERY_CAPACITY,
            delivery_byte_capacity: 1024 * 1024,
            backpressure: Vec::new(),
        });
        assert!(bus.try_subscribe("agent", " ").is_err());
        assert!(
            bus.try_publish(TopicEnvelope::new("room:a", " ", Value::Null))
                .is_err()
        );
    }

    #[tokio::test]
    async fn smallest_delivery_byte_budget_contains_mandatory_superseded_terminal() {
        let bus = TopicBus::new(TopicBusConfig {
            delivery_byte_capacity: MIN_RELAY_DELIVERY_BYTES,
            ..TopicBusConfig::default()
        });
        let mailbox = bus.delivery_mailbox();
        mailbox.supersede(SupersededNotice {
            agent_id: "\"".repeat(256),
            by_instance: "inst_0123456789abcdef0123456789abcdef".to_owned(),
        });
        assert!(mailbox.inner.state.lock().queued_bytes <= MIN_RELAY_DELIVERY_BYTES);
        assert!(matches!(
            mailbox.recv().await,
            Some(RelayOutboundFrame::Superseded(_))
        ));
    }

    #[tokio::test]
    async fn minimum_delivery_capacity_holds_control_recovery_and_subscribed_ack() {
        let bus = TopicBus::new(TopicBusConfig {
            delivery_capacity: MIN_RELAY_DELIVERY_CAPACITY,
            ..TopicBusConfig::default()
        });
        let mailbox = bus.delivery_mailbox();
        mailbox.pause_topics();
        mailbox
            .send(RelayOutboundFrame::Ack {
                event: "card".to_owned(),
            })
            .expect("card ack");
        bus.subscribe_and_recover(
            "agent-a",
            &["room:a".to_owned()],
            Some(0),
            |_| json!({}),
            &mailbox,
        )
        .expect("recovery terminal");
        mailbox
            .send(RelayOutboundFrame::Ack {
                event: "subscribed:room:a".to_owned(),
            })
            .expect("subscribed ack");

        assert!(matches!(
            mailbox.recv().await,
            Some(RelayOutboundFrame::Ack { event }) if event == "card"
        ));
        assert!(matches!(
            mailbox.recv().await,
            Some(RelayOutboundFrame::ReplayComplete {
                from_seq: 0,
                to_seq: 0
            })
        ));
        assert!(matches!(
            mailbox.recv().await,
            Some(RelayOutboundFrame::Ack { event }) if event == "subscribed:room:a"
        ));
    }
}
