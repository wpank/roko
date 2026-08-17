//! Transport-neutral relay envelope and recovery contracts.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{Result, RokoError};

pub const MAX_WIRE_PAYLOAD_BYTES: usize = 1024 * 1024;

/// Common envelope used by relay and workspace WebSocket transports.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WireEnvelope {
    /// Monotonic sequence number within a relay stream.
    pub seq: u64,
    /// Server timestamp in Unix milliseconds.
    pub ts: u64,
    /// Subscription room receiving this event.
    pub room: String,
    /// Event discriminator on the wire.
    #[serde(rename = "type")]
    pub msg_type: String,
    /// Event-specific body.
    pub payload: Value,
    /// Optional publisher identity carried by relay transports.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub publisher_id: Option<String>,
}

/// Stable relay-facing name for the canonical wire envelope.
pub type RelayEnvelope = WireEnvelope;

impl WireEnvelope {
    /// Validate fields that are required for safe routing and dispatch.
    pub fn validate(&self) -> Result<()> {
        if self.room.trim().is_empty() || self.msg_type.trim().is_empty() {
            return Err(RokoError::invalid(
                "wire envelope room and type must not be empty",
            ));
        }
        if self.room.len() > 256
            || self
                .room
                .bytes()
                .any(|byte| byte.is_ascii_control() || byte.is_ascii_whitespace())
        {
            return Err(RokoError::invalid(
                "wire envelope room must be at most 256 bytes without whitespace",
            ));
        }
        if self.msg_type.len() > 128
            || self
                .msg_type
                .bytes()
                .any(|byte| byte.is_ascii_control() || byte.is_ascii_whitespace())
        {
            return Err(RokoError::invalid(
                "wire envelope type must be at most 128 bytes without whitespace",
            ));
        }
        if serde_json::to_vec(&self.payload)
            .map_or(true, |bytes| bytes.len() > MAX_WIRE_PAYLOAD_BYTES)
        {
            return Err(RokoError::invalid("wire envelope payload exceeds one MiB"));
        }
        if self.publisher_id.as_ref().is_some_and(|publisher| {
            publisher.trim().is_empty()
                || publisher.len() > 256
                || publisher
                    .bytes()
                    .any(|byte| byte.is_ascii_control() || byte.is_ascii_whitespace())
        }) {
            return Err(RokoError::invalid(
                "wire envelope publisher_id must be 1-256 bytes without whitespace",
            ));
        }
        Ok(())
    }
}

/// Request to add rooms to a connection's subscription set.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubscribeMessage {
    #[serde(default)]
    pub rooms: Vec<String>,
    /// Durable global cursor to recover atomically with subscription install.
    /// Omitted legacy requests establish the current relay head as baseline.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_seq: Option<u64>,
}

/// Request to remove rooms from a connection's subscription set.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnsubscribeMessage {
    #[serde(default)]
    pub rooms: Vec<String>,
}

/// Request to resume a stream strictly after `last_seq`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResumeMessage {
    pub last_seq: u64,
}

/// Stable relay-facing name for a replay cursor request.
pub type ReplayRequest = ResumeMessage;

/// Current relay state sent when the requested replay gap is unavailable.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SnapshotMessage {
    pub seq: u64,
    pub state: Value,
}

/// Canonical room-name constructors.
///
/// Identifier-bearing constructors allocate because the room is assembled at
/// runtime. The constant rooms are returned with the same owned type so callers
/// can build homogeneous subscription lists without conversions.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct RoomPattern;

impl RoomPattern {
    #[must_use]
    pub fn agent(id: &str) -> String {
        format!("agent:{id}")
    }

    #[must_use]
    pub fn agent_heartbeat(id: &str) -> String {
        format!("agent:{id}:heartbeat")
    }

    #[must_use]
    pub fn agent_output(id: &str) -> String {
        format!("agent:{id}:output")
    }

    #[must_use]
    pub fn plan(id: &str) -> String {
        format!("plan:{id}")
    }

    #[must_use]
    pub fn group(id: &str) -> String {
        format!("group:{id}")
    }

    #[must_use]
    pub fn chain(chain_id: u64) -> String {
        format!("chain:{chain_id}")
    }

    #[must_use]
    pub const fn system() -> &'static str {
        "system"
    }

    #[must_use]
    pub const fn learning() -> &'static str {
        "learning"
    }
}

/// Client-side connection/recovery state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "state")]
pub enum ReconnectionState {
    Disconnected,
    Connecting,
    Resuming { last_seq: u64 },
    Connected,
    Superseded { by_instance: String },
}

/// Recovery selected after comparing a resume cursor with retained history.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "action")]
pub enum GapAction {
    Replay { from_seq: u64, to_seq: u64 },
    Snapshot,
}

/// Stable relay-facing name for the selected replay recovery action.
pub type RelayRecoveryPolicy = GapAction;

impl GapAction {
    /// Reject inverted replay ranges before a recovery transport consumes them.
    pub fn validate(&self) -> Result<()> {
        if let Self::Replay { from_seq, to_seq } = self
            && from_seq > to_seq
        {
            return Err(RokoError::invalid("replay from_seq must not exceed to_seq"));
        }
        Ok(())
    }
}

/// Duplicate-agent ownership notice sent to the older instance.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SupersededNotice {
    pub agent_id: String,
    #[serde(rename = "by", alias = "by_instance")]
    pub by_instance: String,
}

/// Per-event overload policy declared for a relay stream.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "strategy")]
pub enum BackpressureStrategy {
    /// Retain only the latest event and emit at most once per interval.
    Coalesce { interval_ms: u64 },
    /// Bound memory and evict the oldest entry on overflow.
    DropOldest { ring_size: usize },
    /// Preserve every event and apply transport flow control.
    Lossless,
    /// Deliver one of every `every_nth` source events.
    Sample { every_nth: u64 },
}

/// Stable relay-facing name for live-delivery overload policy.
pub type RelayBackpressure = BackpressureStrategy;

impl BackpressureStrategy {
    /// Reject strategies that cannot make progress or retain any data.
    pub fn validate(&self) -> Result<()> {
        match self {
            Self::Coalesce { interval_ms } if *interval_ms == 0 => Err(RokoError::invalid(
                "coalesce interval_ms must be greater than zero",
            )),
            Self::DropOldest { ring_size } if *ring_size == 0 => Err(RokoError::invalid(
                "drop-oldest ring_size must be greater than zero",
            )),
            Self::Sample { every_nth } if *every_nth == 0 => Err(RokoError::invalid(
                "sample every_nth must be greater than zero",
            )),
            _ => Ok(()),
        }
    }
}

/// Strategy assigned to one wire event discriminator.
///
/// Recommended mappings are `heartbeat -> Coalesce { interval_ms: 500 }`,
/// `output_chunk -> DropOldest { ring_size: 1024 }`, `gate_result -> Lossless`,
/// and `feed_data -> Sample { every_nth: dynamic }`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventBackpressureConfig {
    pub event_type: String,
    pub strategy: BackpressureStrategy,
}

impl EventBackpressureConfig {
    pub fn validate(&self) -> Result<()> {
        if self.event_type.trim().is_empty() {
            return Err(RokoError::invalid(
                "backpressure event_type must not be empty",
            ));
        }
        self.strategy.validate()
    }
}

/// Exoskeleton features announced by a workspace.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExoskeletonStatus {
    pub mcp: bool,
    pub a2a: bool,
    pub erc8004_chain_id: Option<u64>,
    pub x402: bool,
}

/// Registration sent by `roko serve` when joining a relay.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceHello {
    pub workspace_id: String,
    pub name: String,
    pub url: Option<String>,
    pub version: String,
    pub capabilities: Vec<String>,
    pub owner_wallet: Option<String>,
    pub agents_count: u32,
    pub uptime_secs: u64,
    pub exoskeleton: ExoskeletonStatus,
}

impl WorkspaceHello {
    pub fn validate(&self) -> Result<()> {
        if self.workspace_id.trim().is_empty()
            || self.name.trim().is_empty()
            || self.version.trim().is_empty()
        {
            return Err(RokoError::invalid(
                "workspace id, name, and version must not be empty",
            ));
        }
        Ok(())
    }
}

/// Workspace directory entry returned by a relay.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceInfo {
    pub workspace_id: String,
    pub name: String,
    pub url: Option<String>,
    pub owner_wallet: Option<String>,
    pub agents_count: u32,
    pub online: bool,
    pub last_seen_ms: u64,
}

/// Presence transition published for workspace discovery.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum WorkspaceEvent {
    #[serde(rename = "workspace_connected")]
    Connected {
        workspace_id: String,
        url: Option<String>,
    },
    #[serde(rename = "workspace_disconnected")]
    Disconnected { workspace_id: String },
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn envelope_uses_reserved_type_wire_key() {
        let envelope = WireEnvelope {
            seq: 42,
            ts: 1_713_974_400_123,
            room: RoomPattern::agent_heartbeat("coder-1"),
            msg_type: "heartbeat".to_owned(),
            payload: json!({"tick": 7}),
            publisher_id: None,
        };

        let value = serde_json::to_value(&envelope).expect("serialize envelope");
        envelope.validate().expect("valid envelope");
        assert_eq!(value["seq"], 42);
        assert_eq!(value["room"], "agent:coder-1:heartbeat");
        assert_eq!(value["type"], "heartbeat");
        assert!(value.get("msg_type").is_none());
        assert_eq!(
            serde_json::from_value::<WireEnvelope>(value).expect("round trip"),
            envelope
        );
    }

    #[test]
    fn room_constructors_match_the_protocol() {
        assert_eq!(RoomPattern::agent("a"), "agent:a");
        assert_eq!(RoomPattern::agent_heartbeat("a"), "agent:a:heartbeat");
        assert_eq!(RoomPattern::agent_output("a"), "agent:a:output");
        assert_eq!(RoomPattern::plan("p"), "plan:p");
        assert_eq!(RoomPattern::group("g"), "group:g");
        assert_eq!(RoomPattern::chain(8453), "chain:8453");
        assert_eq!(RoomPattern::system(), "system");
        assert_eq!(RoomPattern::learning(), "learning");
    }

    #[test]
    fn recovery_and_backpressure_variants_round_trip() {
        let recovery = ReconnectionState::Resuming { last_seq: 99 };
        let recovery_json = serde_json::to_string(&recovery).expect("serialize recovery");
        assert_eq!(
            serde_json::from_str::<ReconnectionState>(&recovery_json).expect("restore recovery"),
            recovery
        );

        let policies = [
            BackpressureStrategy::Coalesce { interval_ms: 500 },
            BackpressureStrategy::DropOldest { ring_size: 1024 },
            BackpressureStrategy::Lossless,
            BackpressureStrategy::Sample { every_nth: 10 },
        ];
        for policy in policies {
            policy.validate().expect("valid strategy");
            let json = serde_json::to_string(&policy).expect("serialize policy");
            assert_eq!(
                serde_json::from_str::<BackpressureStrategy>(&json).expect("restore policy"),
                policy
            );
        }
    }

    #[test]
    fn adversarial_zero_capacity_policies_and_inverted_gaps_fail_closed() {
        for policy in [
            BackpressureStrategy::Coalesce { interval_ms: 0 },
            BackpressureStrategy::DropOldest { ring_size: 0 },
            BackpressureStrategy::Sample { every_nth: 0 },
        ] {
            assert!(policy.validate().is_err(), "accepted {policy:?}");
        }
        assert!(
            GapAction::Replay {
                from_seq: 100,
                to_seq: 99
            }
            .validate()
            .is_err()
        );
        assert!(
            WireEnvelope {
                seq: 1,
                ts: 1,
                room: " ".to_owned(),
                msg_type: "heartbeat".to_owned(),
                payload: Value::Null,
                publisher_id: None,
            }
            .validate()
            .is_err()
        );
        let mut invalid_publisher = WireEnvelope {
            seq: 1,
            ts: 1,
            room: "room:a".to_owned(),
            msg_type: "heartbeat".to_owned(),
            payload: Value::Null,
            publisher_id: Some(" ".to_owned()),
        };
        assert!(invalid_publisher.validate().is_err());
        invalid_publisher.publisher_id = Some("x".repeat(257));
        assert!(invalid_publisher.validate().is_err());
    }

    #[test]
    fn supersession_uses_the_documented_by_key() {
        let notice = SupersededNotice {
            agent_id: "coder-1".to_owned(),
            by_instance: "inst_new".to_owned(),
        };
        let value = serde_json::to_value(&notice).expect("serialize notice");
        assert_eq!(value["by"], "inst_new");
        assert!(value.get("by_instance").is_none());
        assert_eq!(
            serde_json::from_value::<SupersededNotice>(json!({
                "agent_id": "coder-1",
                "by_instance": "inst_old_shape"
            }))
            .expect("accept compatibility alias")
            .by_instance,
            "inst_old_shape"
        );
    }

    #[test]
    fn workspace_events_have_unambiguous_wire_discriminators() {
        let event = WorkspaceEvent::Connected {
            workspace_id: "ws-1".to_owned(),
            url: Some("https://example.test".to_owned()),
        };
        let value = serde_json::to_value(event).expect("serialize workspace event");
        assert_eq!(value["type"], "workspace_connected");
        assert_eq!(value["workspace_id"], "ws-1");
    }

    #[test]
    fn subscription_resume_snapshot_and_workspace_contracts_round_trip() {
        let subscription = SubscribeMessage {
            rooms: vec![
                RoomPattern::system().to_owned(),
                RoomPattern::agent("coder-1"),
            ],
            last_seq: Some(41),
        };
        let encoded = serde_json::to_string(&subscription).expect("serialize subscription");
        assert_eq!(
            serde_json::from_str::<SubscribeMessage>(&encoded).expect("restore subscription"),
            subscription
        );
        let unsubscribe = UnsubscribeMessage {
            rooms: vec![RoomPattern::agent("coder-1")],
        };
        assert_eq!(
            serde_json::from_value::<UnsubscribeMessage>(
                serde_json::to_value(&unsubscribe).expect("serialize unsubscribe")
            )
            .expect("restore unsubscribe"),
            unsubscribe
        );

        let resume = ResumeMessage { last_seq: 4_821 };
        assert_eq!(
            serde_json::from_value::<ResumeMessage>(
                serde_json::to_value(resume).expect("serialize resume")
            )
            .expect("restore resume"),
            resume
        );
        let snapshot = SnapshotMessage {
            seq: 71_042,
            state: json!({"agents": [], "rooms": ["system"]}),
        };
        assert_eq!(
            serde_json::from_str::<SnapshotMessage>(
                &serde_json::to_string(&snapshot).expect("serialize snapshot")
            )
            .expect("restore snapshot"),
            snapshot
        );

        let hello = WorkspaceHello {
            workspace_id: "ws-a1b2c3".to_owned(),
            name: "will-dev".to_owned(),
            url: Some("https://workspace.example".to_owned()),
            version: "0.1.0".to_owned(),
            capabilities: vec!["agents".to_owned(), "gateway".to_owned()],
            owner_wallet: Some("0xabc".to_owned()),
            agents_count: 3,
            uptime_secs: 3_600,
            exoskeleton: ExoskeletonStatus {
                mcp: true,
                a2a: true,
                erc8004_chain_id: Some(1),
                x402: true,
            },
        };
        hello.validate().expect("valid workspace hello");
        let hello_json = serde_json::to_string(&hello).expect("serialize hello");
        assert_eq!(
            serde_json::from_str::<WorkspaceHello>(&hello_json).expect("restore hello"),
            hello
        );

        let info = WorkspaceInfo {
            workspace_id: hello.workspace_id.clone(),
            name: hello.name.clone(),
            url: hello.url.clone(),
            owner_wallet: hello.owner_wallet.clone(),
            agents_count: hello.agents_count,
            online: true,
            last_seen_ms: 1_713_960_000_000,
        };
        assert_eq!(
            serde_json::from_value::<WorkspaceInfo>(
                serde_json::to_value(&info).expect("serialize workspace info")
            )
            .expect("restore workspace info"),
            info
        );

        let mut invalid = hello;
        invalid.workspace_id = " ".to_owned();
        assert!(invalid.validate().is_err());
    }
}
