use chrono::Utc;
use roko_core::wire_protocol::{
    RelayEnvelope, ReplayRequest, SnapshotMessage, SubscribeMessage, SupersededNotice,
    UnsubscribeMessage,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Maximum message timeout the relay will accept from clients.
pub const MAX_MESSAGE_TIMEOUT_MS: u64 = 60_000;

/// Default message timeout for forwarding requests.
pub const DEFAULT_MESSAGE_TIMEOUT_MS: u64 = 15_000;

/// Initial agent hello frame sent over the relay websocket.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentHello {
    pub agent_id: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub capabilities: Vec<String>,
    #[serde(default)]
    pub rest_endpoint: Option<String>,
    #[serde(default)]
    pub card: Option<Value>,
    #[serde(default)]
    pub card_uri: Option<String>,
    #[serde(default)]
    pub metadata: Value,
}

/// Directory entry for a connected agent.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConnectedAgent {
    pub agent_id: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub capabilities: Vec<String>,
    #[serde(default)]
    pub rest_endpoint: Option<String>,
    #[serde(default)]
    pub card_uri: Option<String>,
    pub connected_at_ms: u64,
    pub relay_backed: bool,
}

/// HTTP request to forward a message to an agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelayMessageRequest {
    pub agent_id: String,
    pub message: Value,
    #[serde(default)]
    pub timeout_ms: Option<u64>,
}

impl RelayMessageRequest {
    #[must_use]
    pub fn timeout_ms(&self) -> u64 {
        self.timeout_ms
            .unwrap_or(DEFAULT_MESSAGE_TIMEOUT_MS)
            .clamp(1, MAX_MESSAGE_TIMEOUT_MS)
    }
}

/// HTTP response returned after a forwarded message completes.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RelayMessageResponse {
    pub message_id: String,
    pub agent_id: String,
    pub response: Value,
}

/// Descriptor for a data feed an agent can provide.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FeedDescriptor {
    pub feed_id: String,
    pub topic: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub kind: String,
    #[serde(default)]
    pub rate: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schema: Option<Value>,
}

/// Frames the relay receives from agents.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AgentInboundFrame {
    Hello(AgentHello),
    Card {
        card: Value,
        #[serde(default)]
        card_uri: Option<String>,
    },
    Response {
        message_id: String,
        response: Value,
    },
    Error {
        #[serde(default)]
        message_id: Option<String>,
        error: String,
    },
    Ping,
    /// Subscribe to a topic. Relay will forward matching TopicEnvelopes.
    Subscribe {
        #[serde(flatten)]
        request: SubscribeMessage,
        /// Compatibility field accepted from pre-R02 clients.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        topic: Option<String>,
    },
    /// Resume subscribed rooms strictly after the durable client cursor.
    Resume(ReplayRequest),
    /// Confirm that one room event is durably committed by the consumer.
    Ack {
        room: String,
        seq: u64,
    },
    /// Unsubscribe from a previously subscribed topic.
    Unsubscribe {
        #[serde(flatten)]
        request: UnsubscribeMessage,
        /// Compatibility field accepted from pre-R02 clients.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        topic: Option<String>,
    },
    /// Publish a message to a topic. Relay fans out to all subscribers.
    Publish {
        topic: String,
        msg_type: String,
        payload: serde_json::Value,
    },
    /// Register a data feed this agent provides.
    RegisterFeed {
        feed: FeedDescriptor,
    },
    /// Unregister a previously registered feed.
    UnregisterFeed {
        feed_id: String,
    },
}

/// Frames the relay sends to agents.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RelayOutboundFrame {
    Ack {
        event: String,
    },
    Message {
        message_id: String,
        message: Value,
    },
    Error {
        message_id: Option<String>,
        error: String,
    },
    Pong,
    /// A message published to a topic this agent is subscribed to.
    TopicMessage {
        topic: String,
        msg_type: String,
        payload: serde_json::Value,
        publisher_id: Option<String>,
        seq: u64,
        timestamp_ms: i64,
    },
    /// The requested cursor fell outside retained relay history.
    Snapshot(SnapshotMessage),
    /// Authoritative global cursor after all selected replay frames.
    ReplayComplete {
        from_seq: u64,
        to_seq: u64,
    },
    /// A lossless live queue overflowed; reconnect with the durable cursor.
    ResumeRequired {
        last_available_seq: u64,
    },
    /// Last-write-wins ownership notice for a duplicate agent id.
    Superseded(SupersededNotice),
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum RelayControlFrame {
    Ack {
        event: String,
    },
    Message {
        message_id: String,
        message: Value,
    },
    Error {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        message_id: Option<String>,
        error: String,
    },
    Pong,
    Snapshot(SnapshotMessage),
    ReplayComplete {
        from_seq: u64,
        to_seq: u64,
    },
    ResumeRequired {
        last_available_seq: u64,
    },
    Superseded(SupersededNotice),
}

#[derive(Serialize)]
struct RelayEnvelopeRef<'a> {
    seq: u64,
    ts: u64,
    room: &'a str,
    #[serde(rename = "type")]
    msg_type: &'a str,
    payload: &'a Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    publisher_id: Option<&'a str>,
}

#[derive(Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum RelayControlFrameRef<'a> {
    Ack {
        event: &'a str,
    },
    Message {
        message_id: &'a str,
        message: &'a Value,
    },
    Error {
        #[serde(skip_serializing_if = "Option::is_none")]
        message_id: Option<&'a str>,
        error: &'a str,
    },
    Pong,
    Snapshot(&'a SnapshotMessage),
    ReplayComplete {
        from_seq: u64,
        to_seq: u64,
    },
    ResumeRequired {
        last_available_seq: u64,
    },
    Superseded(&'a SupersededNotice),
}

impl Serialize for RelayOutboundFrame {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        if let Self::TopicMessage {
            topic,
            msg_type,
            payload,
            publisher_id,
            seq,
            timestamp_ms,
        } = self
        {
            return RelayEnvelopeRef {
                seq: *seq,
                ts: u64::try_from(*timestamp_ms).unwrap_or_default(),
                room: topic,
                msg_type,
                payload,
                publisher_id: publisher_id.as_deref(),
            }
            .serialize(serializer);
        }
        control_ref_from_outbound(self).serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for RelayOutboundFrame {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = Value::deserialize(deserializer)?;
        if value.get("seq").is_some()
            && value.get("room").is_some()
            && value.get("payload").is_some()
        {
            let envelope: RelayEnvelope =
                serde_json::from_value(value).map_err(serde::de::Error::custom)?;
            envelope.validate().map_err(serde::de::Error::custom)?;
            return Ok(Self::TopicMessage {
                topic: envelope.room,
                msg_type: envelope.msg_type,
                payload: envelope.payload,
                publisher_id: envelope.publisher_id,
                seq: envelope.seq,
                timestamp_ms: i64::try_from(envelope.ts).unwrap_or(i64::MAX),
            });
        }
        let control =
            serde_json::from_value::<RelayControlFrame>(value).map_err(serde::de::Error::custom)?;
        Ok(outbound_from_control(control))
    }
}

fn control_ref_from_outbound(frame: &RelayOutboundFrame) -> RelayControlFrameRef<'_> {
    match frame {
        RelayOutboundFrame::Ack { event } => RelayControlFrameRef::Ack { event },
        RelayOutboundFrame::Message {
            message_id,
            message,
        } => RelayControlFrameRef::Message {
            message_id,
            message,
        },
        RelayOutboundFrame::Error { message_id, error } => RelayControlFrameRef::Error {
            message_id: message_id.as_deref(),
            error,
        },
        RelayOutboundFrame::Pong => RelayControlFrameRef::Pong,
        RelayOutboundFrame::Snapshot(snapshot) => RelayControlFrameRef::Snapshot(snapshot),
        RelayOutboundFrame::ReplayComplete { from_seq, to_seq } => {
            RelayControlFrameRef::ReplayComplete {
                from_seq: *from_seq,
                to_seq: *to_seq,
            }
        }
        RelayOutboundFrame::ResumeRequired { last_available_seq } => {
            RelayControlFrameRef::ResumeRequired {
                last_available_seq: *last_available_seq,
            }
        }
        RelayOutboundFrame::Superseded(notice) => RelayControlFrameRef::Superseded(notice),
        RelayOutboundFrame::TopicMessage { .. } => {
            unreachable!("topic frames serialize through RelayEnvelope")
        }
    }
}

fn outbound_from_control(frame: RelayControlFrame) -> RelayOutboundFrame {
    match frame {
        RelayControlFrame::Ack { event } => RelayOutboundFrame::Ack { event },
        RelayControlFrame::Message {
            message_id,
            message,
        } => RelayOutboundFrame::Message {
            message_id,
            message,
        },
        RelayControlFrame::Error { message_id, error } => {
            RelayOutboundFrame::Error { message_id, error }
        }
        RelayControlFrame::Pong => RelayOutboundFrame::Pong,
        RelayControlFrame::Snapshot(snapshot) => RelayOutboundFrame::Snapshot(snapshot),
        RelayControlFrame::ReplayComplete { from_seq, to_seq } => {
            RelayOutboundFrame::ReplayComplete { from_seq, to_seq }
        }
        RelayControlFrame::ResumeRequired { last_available_seq } => {
            RelayOutboundFrame::ResumeRequired { last_available_seq }
        }
        RelayControlFrame::Superseded(notice) => RelayOutboundFrame::Superseded(notice),
    }
}

/// Internal representation of a published topic message.
/// Used within the relay bus; serialized as TopicMessage when sent to agents.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopicEnvelope {
    pub topic: String,
    pub msg_type: String,
    pub payload: serde_json::Value,
    pub publisher_id: Option<String>,
    pub seq: u64,
    pub timestamp_ms: i64,
}

impl TopicEnvelope {
    pub fn new(
        topic: impl Into<String>,
        msg_type: impl Into<String>,
        payload: serde_json::Value,
    ) -> Self {
        Self {
            topic: topic.into(),
            msg_type: msg_type.into(),
            payload,
            publisher_id: None,
            seq: 0,
            timestamp_ms: Utc::now().timestamp_millis(),
        }
    }

    pub fn with_publisher(mut self, id: impl Into<String>) -> Self {
        self.publisher_id = Some(id.into());
        self
    }

    pub fn with_seq(mut self, seq: u64) -> Self {
        self.seq = seq;
        self
    }

    /// Adapt the relay's delivery metadata to the canonical wire vocabulary.
    #[must_use]
    pub fn to_wire(&self) -> RelayEnvelope {
        RelayEnvelope {
            seq: self.seq,
            ts: u64::try_from(self.timestamp_ms).unwrap_or_default(),
            room: self.topic.clone(),
            msg_type: self.msg_type.clone(),
            payload: self.payload.clone(),
            publisher_id: self.publisher_id.clone(),
        }
    }

    /// Restore relay-specific publisher metadata around a canonical envelope.
    #[must_use]
    pub fn from_wire(envelope: RelayEnvelope, publisher_id: Option<String>) -> Self {
        let publisher_id = publisher_id.or(envelope.publisher_id);
        Self {
            topic: envelope.room,
            msg_type: envelope.msg_type,
            payload: envelope.payload,
            publisher_id,
            seq: envelope.seq,
            timestamp_ms: i64::try_from(envelope.ts).unwrap_or(i64::MAX),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn topic_envelope_adapts_to_the_canonical_wire_contract() {
        let relay = TopicEnvelope::new("agent:a", "heartbeat", serde_json::json!({ "tick": 7 }))
            .with_publisher("agent:b")
            .with_seq(42);
        let wire = relay.to_wire();
        wire.validate().expect("canonical relay envelope");
        assert_eq!(wire.room, "agent:a");
        assert_eq!(wire.msg_type, "heartbeat");
        assert_eq!(wire.seq, 42);
        assert_eq!(
            TopicEnvelope::from_wire(wire, relay.publisher_id.clone()).publisher_id,
            relay.publisher_id
        );
    }

    #[test]
    fn outbound_topic_uses_canonical_relay_envelope_on_wire() {
        let frame = RelayOutboundFrame::TopicMessage {
            topic: "room:a".to_owned(),
            msg_type: "gate_result".to_owned(),
            payload: serde_json::json!({"ok": true}),
            publisher_id: Some("agent-a".to_owned()),
            seq: 7,
            timestamp_ms: 42,
        };
        let value = serde_json::to_value(&frame).expect("serialize canonical event");
        assert_eq!(value["room"], "room:a");
        assert_eq!(value["type"], "gate_result");
        assert_eq!(value["ts"], 42);
        assert!(value.get("topic").is_none());
        assert!(value.get("msg_type").is_none());
        assert_eq!(
            serde_json::from_value::<RelayOutboundFrame>(value).expect("deserialize event"),
            frame
        );
    }
}

/// Workspace hello frame sent by roko-serve on startup.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceHello {
    pub workspace_id: String,
    #[serde(default)]
    pub name: Option<String>,
    /// Public URL of the roko instance (e.g. `https://my-roko.up.railway.app`).
    pub url: String,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub owner_wallet: Option<String>,
    #[serde(default)]
    pub agents_count: u32,
}

/// Directory entry for a connected workspace.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConnectedWorkspace {
    pub workspace_id: String,
    #[serde(default)]
    pub name: Option<String>,
    pub url: String,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub owner_wallet: Option<String>,
    pub agents_count: u32,
    pub connected_at_ms: u64,
    pub last_heartbeat_ms: u64,
}

/// Optional dashboard event stream payload.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RelayEvent {
    AgentConnected {
        agent: ConnectedAgent,
    },
    AgentDisconnected {
        agent_id: String,
    },
    CardUpdated {
        agent_id: String,
        card_uri: String,
    },
    MessageDelivered {
        agent_id: String,
        message_id: String,
    },
    MessageResponded {
        agent_id: String,
        message_id: String,
    },
    AgentError {
        agent_id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        message_id: Option<String>,
        error: String,
    },
    WorkspaceConnected {
        workspace: ConnectedWorkspace,
    },
    WorkspaceDisconnected {
        workspace_id: String,
    },
    WorkspaceHeartbeat {
        workspace_id: String,
        agents_count: u32,
    },
    FeedRegistered {
        agent_id: String,
        feed: FeedDescriptor,
    },
    FeedUnregistered {
        agent_id: String,
        feed_id: String,
    },
}
