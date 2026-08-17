//! Restart-safe relay subscription execution for `roko serve`.
//!
//! The relay client emits ACK only after [`TopicHandler::on_topic_message`]
//! returns success. This module makes that success boundary durable: a
//! processing intent is persisted before agent execution, and the terminal
//! dispatch receipts, room/subscription cursors, and global cursor are then
//! committed in one atomic journal replacement. An interrupted intent is
//! never replayed blindly; it becomes an explicit reconciliation requirement.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::io::Read as _;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context as _, Result, anyhow};
use async_trait::async_trait;
use roko_agent_server::features::relay_client::{
    RelayClientStatus, SnapshotDisposition, TopicHandler,
};
use roko_core::config::schema::SubscriptionTrigger;
use roko_core::wire_protocol::{RelayEnvelope, SnapshotMessage};
use roko_core::{Body, ContentHash, Kind, Provenance, Signal};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::{Mutex, RwLock};
use utoipa::ToSchema;

use crate::dispatch::{
    AgentDispatcher, DispatchTerminal, Subscription, SubscriptionPermit, SubscriptionRegistry,
    dispatch_relay_subscription,
};
use crate::state::AppState;

const JOURNAL_SCHEMA_VERSION: u32 = 2;
const JOURNAL_MAX_ENTRIES: usize = 4_096;
const JOURNAL_MAX_BYTES: u64 = 4 * 1024 * 1024;
const JOURNAL_MAX_CURSOR_KEYS: usize = 4_096;
const JOURNAL_MAX_TEXT_BYTES: usize = 512;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RemoteSubscriptionPlan {
    pub rooms: Vec<String>,
    pub unsupported_triggers: Vec<String>,
    pub capacity_rejected_rooms: Vec<String>,
}

pub(crate) fn remote_subscription_plan(
    registry: &SubscriptionRegistry,
    room_capacity: usize,
) -> RemoteSubscriptionPlan {
    let mut rooms = BTreeSet::new();
    let mut unsupported_triggers = Vec::new();
    for subscription in registry.all() {
        if !subscription.enabled || is_local_only_trigger(&subscription) {
            continue;
        }
        if is_exact_relay_room(&subscription.trigger) {
            rooms.insert(subscription.trigger);
        } else {
            unsupported_triggers.push(format!("{}:{}", subscription.id, subscription.trigger));
        }
    }
    unsupported_triggers.sort();
    let rooms = rooms.into_iter().collect::<Vec<_>>();
    let capacity_rejected_rooms = if rooms.len() > room_capacity {
        rooms.clone()
    } else {
        Vec::new()
    };
    RemoteSubscriptionPlan {
        rooms,
        unsupported_triggers,
        capacity_rejected_rooms,
    }
}

fn is_local_only_trigger(subscription: &Subscription) -> bool {
    matches!(
        subscription.trigger_config,
        Some(SubscriptionTrigger::Cron { .. } | SubscriptionTrigger::FileWatch { .. })
    )
}

pub(crate) fn is_exact_relay_room(trigger: &str) -> bool {
    !trigger.contains(['*', '?'])
        && RelayEnvelope {
            seq: 0,
            ts: 0,
            room: trigger.to_string(),
            msg_type: "subscription".to_string(),
            payload: Value::Null,
            publisher_id: None,
        }
        .validate()
        .is_ok()
}

fn is_remote_subscription_eligible(subscription: &Subscription, topic: &str) -> bool {
    subscription.enabled
        && !is_local_only_trigger(subscription)
        && is_exact_relay_room(&subscription.trigger)
        && subscription.trigger == topic
}

/// Durable location of the relay subscription journal.
pub(crate) fn journal_path(workdir: &Path) -> PathBuf {
    workdir
        .join(".roko")
        .join("state")
        .join("subscription-relay-journal.json")
}

fn unix_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum JournalEntryState {
    Processing {
        expected_subscriptions: Vec<String>,
        #[serde(default)]
        partial_terminals: Vec<SubscriptionTerminal>,
    },
    Committed {
        terminals: Vec<SubscriptionTerminal>,
    },
    Failed {
        error: String,
        #[serde(default)]
        partial_terminals: Vec<SubscriptionTerminal>,
    },
    ReplayCheckpoint,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "outcome", rename_all = "snake_case")]
enum SubscriptionTerminal {
    Dispatched {
        #[serde(flatten)]
        receipt: DispatchTerminal,
    },
    Suppressed {
        subscription_id: String,
        policy: String,
        detail: String,
    },
}

impl SubscriptionTerminal {
    fn subscription_id(&self) -> &str {
        match self {
            Self::Dispatched { receipt } => &receipt.subscription_id,
            Self::Suppressed {
                subscription_id, ..
            } => subscription_id,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct JournalEntry {
    seq: u64,
    room: String,
    msg_type: String,
    payload_hash: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    publisher_id: Option<String>,
    created_at: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    completed_at: Option<u64>,
    state: JournalEntryState,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub(crate) struct ReconciliationRecord {
    pub seq: u64,
    pub reason: String,
    pub detected_at: u64,
    pub evidence_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub(crate) struct RelayStreamBinding {
    pub origin_hash: String,
    pub rooms: Vec<String>,
    pub room_set_hash: String,
    pub generation: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct JournalFile {
    schema_version: u32,
    integrity_hash: String,
    global_cursor: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    stream_binding: Option<RelayStreamBinding>,
    #[serde(default)]
    room_cursors: BTreeMap<String, u64>,
    #[serde(default)]
    subscription_cursors: BTreeMap<String, BTreeMap<String, u64>>,
    #[serde(default)]
    entries: VecDeque<JournalEntry>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    reconciliation_required: Option<ReconciliationRecord>,
}

impl Default for JournalFile {
    fn default() -> Self {
        Self {
            schema_version: JOURNAL_SCHEMA_VERSION,
            integrity_hash: String::new(),
            global_cursor: 0,
            stream_binding: None,
            room_cursors: BTreeMap::new(),
            subscription_cursors: BTreeMap::new(),
            entries: VecDeque::new(),
            reconciliation_required: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq, ToSchema)]
#[serde(tag = "state", rename_all = "snake_case")]
pub(crate) enum ServeRelayConnectionStatus {
    Unconfigured,
    Connecting,
    Connected {
        durable_cursor: u64,
        desired_rooms: usize,
    },
    Disconnected {
        durable_cursor: u64,
        #[serde(skip_serializing_if = "Option::is_none")]
        reason: Option<String>,
    },
    ReconciliationRequired {
        snapshot_seq: u64,
    },
    Superseded {
        by_instance: String,
    },
    Stopped,
}

impl From<RelayClientStatus> for ServeRelayConnectionStatus {
    fn from(value: RelayClientStatus) -> Self {
        match value {
            RelayClientStatus::Connected {
                durable_cursor,
                desired_rooms,
            } => Self::Connected {
                durable_cursor,
                desired_rooms,
            },
            RelayClientStatus::Disconnected { durable_cursor } => Self::Disconnected {
                durable_cursor,
                reason: None,
            },
            RelayClientStatus::ReconciliationRequired { snapshot_seq } => {
                Self::ReconciliationRequired { snapshot_seq }
            }
            RelayClientStatus::Superseded { by_instance } => Self::Superseded { by_instance },
            RelayClientStatus::Stopped => Self::Stopped,
        }
    }
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq, ToSchema)]
pub(crate) struct SubscriptionRelayStatus {
    pub connection: ServeRelayConnectionStatus,
    pub schema_version: u32,
    pub global_cursor: u64,
    pub stream_binding: Option<RelayStreamBinding>,
    pub room_cursors: BTreeMap<String, u64>,
    pub subscription_cursors: BTreeMap<String, BTreeMap<String, u64>>,
    pub retained_entries: usize,
    pub processing_entries: usize,
    pub failed_entries: usize,
    pub reconciliation_required: Option<ReconciliationRecord>,
    pub unsupported_triggers: Vec<String>,
    pub capacity_rejected_rooms: Vec<String>,
}

/// Shared serve-side runtime containing durable journal and connection state.
pub(crate) struct SubscriptionRelayRuntime {
    journal: SubscriptionRelayJournal,
    connection: RwLock<ServeRelayConnectionStatus>,
    unsupported_triggers: RwLock<Vec<String>>,
    capacity_rejected_rooms: RwLock<Vec<String>>,
}

impl SubscriptionRelayRuntime {
    pub(crate) fn open(workdir: &Path, configured: bool) -> Result<Self> {
        Ok(Self {
            journal: SubscriptionRelayJournal::open(journal_path(workdir))?,
            connection: RwLock::new(if configured {
                ServeRelayConnectionStatus::Connecting
            } else {
                ServeRelayConnectionStatus::Unconfigured
            }),
            unsupported_triggers: RwLock::new(Vec::new()),
            capacity_rejected_rooms: RwLock::new(Vec::new()),
        })
    }

    pub(crate) async fn set_connection(&self, status: ServeRelayConnectionStatus) {
        *self.connection.write().await = status;
    }

    pub(crate) async fn observe_client_status(&self, status: RelayClientStatus) {
        self.set_connection(status.into()).await;
    }

    pub(crate) async fn set_unsupported_triggers(&self, mut triggers: Vec<String>) {
        triggers.sort();
        triggers.dedup();
        *self.unsupported_triggers.write().await = triggers;
    }

    pub(crate) async fn set_subscription_plan_diagnostics(
        &self,
        mut unsupported_triggers: Vec<String>,
        mut capacity_rejected_rooms: Vec<String>,
    ) {
        unsupported_triggers.sort();
        unsupported_triggers.dedup();
        capacity_rejected_rooms.sort();
        capacity_rejected_rooms.dedup();
        *self.unsupported_triggers.write().await = unsupported_triggers;
        *self.capacity_rejected_rooms.write().await = capacity_rejected_rooms;
    }

    pub(crate) async fn status(&self) -> SubscriptionRelayStatus {
        let journal = self.journal.status().await;
        SubscriptionRelayStatus {
            connection: self.connection.read().await.clone(),
            unsupported_triggers: self.unsupported_triggers.read().await.clone(),
            capacity_rejected_rooms: self.capacity_rejected_rooms.read().await.clone(),
            ..journal
        }
    }

    pub(crate) async fn bind_stream(&self, origin_hash: &str, rooms: &[String]) -> Result<()> {
        self.journal.bind_stream(origin_hash, rooms).await
    }

    /// Fail closed before reconnecting when the configured stream would make
    /// an existing global cursor unsafe, including empty or over-capacity
    /// plans which cannot be installed on the client.
    pub(crate) async fn guard_stream_change(
        &self,
        origin_hash: &str,
        rooms: &[String],
    ) -> Result<()> {
        self.journal.guard_stream_change(origin_hash, rooms).await
    }
}

struct SubscriptionRelayJournal {
    path: PathBuf,
    state: Mutex<JournalFile>,
}

#[derive(Debug)]
enum BeginMessage {
    Duplicate,
    Dispatch,
}

impl SubscriptionRelayJournal {
    fn open(path: PathBuf) -> Result<Self> {
        let mut state = match std::fs::File::open(&path) {
            Ok(mut file) => {
                let before = file
                    .metadata()
                    .with_context(|| format!("inspect open relay journal {}", path.display()))?;
                if before.len() > JOURNAL_MAX_BYTES {
                    anyhow::bail!(
                        "relay subscription journal is {} bytes; limit is {JOURNAL_MAX_BYTES}",
                        before.len()
                    );
                }
                let mut bytes = Vec::with_capacity(before.len() as usize);
                (&mut file)
                    .take(JOURNAL_MAX_BYTES + 1)
                    .read_to_end(&mut bytes)
                    .with_context(|| format!("read relay journal {}", path.display()))?;
                let after = file
                    .metadata()
                    .with_context(|| format!("reinspect open relay journal {}", path.display()))?;
                if bytes.len() as u64 > JOURNAL_MAX_BYTES
                    || before.len() != after.len()
                    || after.len() != bytes.len() as u64
                {
                    anyhow::bail!("relay subscription journal changed size while being read");
                }
                serde_json::from_slice::<JournalFile>(&bytes)
                    .with_context(|| format!("parse relay journal {}", path.display()))?
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => JournalFile::default(),
            Err(error) => {
                return Err(error)
                    .with_context(|| format!("inspect relay journal {}", path.display()));
            }
        };
        if state.integrity_hash.is_empty() && !path.exists() {
            refresh_integrity(&mut state)?;
        }
        validate_journal(&state)?;

        let interrupted = state.entries.iter().find_map(|entry| {
            matches!(entry.state, JournalEntryState::Processing { .. }).then_some(entry.seq)
        });
        if let Some(seq) = interrupted {
            state.reconciliation_required = Some(ReconciliationRecord {
                seq,
                reason: "restart observed an interrupted dispatch intent".to_string(),
                detected_at: unix_timestamp(),
                evidence_hash: state
                    .entries
                    .iter()
                    .find(|entry| entry.seq == seq)
                    .map(|entry| entry.payload_hash.clone())
                    .unwrap_or_default(),
            });
            refresh_integrity(&mut state)?;
            persist_journal_sync(&path, &state)?;
        }

        Ok(Self {
            path,
            state: Mutex::new(state),
        })
    }

    async fn durable_cursor(&self) -> u64 {
        self.state.lock().await.global_cursor
    }

    async fn guard_stream_change(&self, origin_hash: &str, rooms: &[String]) -> Result<()> {
        if ContentHash::from_hex(origin_hash).is_none() {
            anyhow::bail!("relay origin hash must be a 64-character content hash");
        }
        if rooms.windows(2).any(|pair| pair[0] >= pair[1])
            || rooms.iter().any(|room| !is_exact_relay_room(room))
        {
            anyhow::bail!("proposed relay stream rooms must be exact, sorted, and unique");
        }
        let proposed_room_set_hash = room_set_hash(rooms)?;
        let mut guard = self.state.lock().await;
        if let Some(record) = &guard.reconciliation_required {
            anyhow::bail!(
                "relay subscription reconciliation required at sequence {}: {}",
                record.seq,
                record.reason
            );
        }
        let Some(binding) = &guard.stream_binding else {
            return Ok(());
        };
        let origin_changed = binding.origin_hash != origin_hash;
        let rooms_changed = binding.room_set_hash != proposed_room_set_hash;
        if !origin_changed && (!rooms_changed || guard.global_cursor == 0) {
            return Ok(());
        }

        let reason = if origin_changed {
            "relay origin identity changed; cursor reuse is unsafe"
        } else {
            "relay room set changed after the global cursor advanced"
        };
        let mut next = guard.clone();
        next.reconciliation_required = Some(ReconciliationRecord {
            seq: guard.global_cursor,
            reason: reason.to_string(),
            detected_at: unix_timestamp(),
            evidence_hash: ContentHash::of(
                format!("{origin_hash}:{proposed_room_set_hash}").as_bytes(),
            )
            .to_hex(),
        });
        self.persist_candidate(&mut next).await?;
        *guard = next;
        anyhow::bail!("{reason}")
    }

    async fn bind_stream(&self, origin_hash: &str, rooms: &[String]) -> Result<()> {
        if ContentHash::from_hex(origin_hash).is_none() {
            anyhow::bail!("relay origin hash must be a 64-character content hash");
        }
        if rooms.is_empty()
            || rooms.windows(2).any(|pair| pair[0] >= pair[1])
            || rooms.iter().any(|room| !is_exact_relay_room(room))
        {
            anyhow::bail!("relay stream rooms must be non-empty, exact, sorted, and unique");
        }
        let new_room_set_hash = room_set_hash(rooms)?;
        let mut guard = self.state.lock().await;
        if let Some(record) = &guard.reconciliation_required {
            anyhow::bail!(
                "relay subscription reconciliation required at sequence {}: {}",
                record.seq,
                record.reason
            );
        }
        if let Some(binding) = &guard.stream_binding {
            if binding.origin_hash == origin_hash && binding.room_set_hash == new_room_set_hash {
                return Ok(());
            }
        }

        let mut next = guard.clone();
        if let Some(binding) = &guard.stream_binding {
            let origin_changed = binding.origin_hash != origin_hash;
            if origin_changed || guard.global_cursor > 0 {
                let reason = if origin_changed {
                    "relay origin identity changed; cursor reuse is unsafe"
                } else {
                    "relay room set changed after the global cursor advanced"
                };
                next.reconciliation_required = Some(ReconciliationRecord {
                    seq: guard.global_cursor,
                    reason: reason.to_string(),
                    detected_at: unix_timestamp(),
                    evidence_hash: ContentHash::of(
                        format!("{origin_hash}:{new_room_set_hash}").as_bytes(),
                    )
                    .to_hex(),
                });
                self.persist_candidate(&mut next).await?;
                *guard = next;
                anyhow::bail!("{reason}");
            }
        }

        let generation = guard
            .stream_binding
            .as_ref()
            .map_or(1, |binding| binding.generation.saturating_add(1));
        next.stream_binding = Some(RelayStreamBinding {
            origin_hash: origin_hash.to_string(),
            rooms: rooms.to_vec(),
            room_set_hash: new_room_set_hash,
            generation,
        });
        next.room_cursors.clear();
        next.subscription_cursors.clear();
        next.entries.clear();
        self.persist_candidate(&mut next).await?;
        *guard = next;
        Ok(())
    }

    async fn status(&self) -> SubscriptionRelayStatus {
        let state = self.state.lock().await;
        SubscriptionRelayStatus {
            connection: ServeRelayConnectionStatus::Unconfigured,
            schema_version: state.schema_version,
            global_cursor: state.global_cursor,
            stream_binding: state.stream_binding.clone(),
            room_cursors: state.room_cursors.clone(),
            subscription_cursors: state.subscription_cursors.clone(),
            retained_entries: state.entries.len(),
            processing_entries: state
                .entries
                .iter()
                .filter(|entry| matches!(entry.state, JournalEntryState::Processing { .. }))
                .count(),
            failed_entries: state
                .entries
                .iter()
                .filter(|entry| matches!(entry.state, JournalEntryState::Failed { .. }))
                .count(),
            reconciliation_required: state.reconciliation_required.clone(),
            unsupported_triggers: Vec::new(),
            capacity_rejected_rooms: Vec::new(),
        }
    }

    async fn begin_message(
        &self,
        seq: u64,
        room: &str,
        msg_type: &str,
        payload_hash: &str,
        publisher_id: Option<&str>,
        expected_subscriptions: Vec<String>,
    ) -> Result<BeginMessage> {
        if seq == 0 {
            anyhow::bail!("relay sequence zero is not a deliverable cursor");
        }
        validate_text("room", room)?;
        validate_text("message type", msg_type)?;
        if let Some(publisher_id) = publisher_id {
            validate_text("publisher id", publisher_id)?;
        }

        let mut guard = self.state.lock().await;
        if let Some(record) = &guard.reconciliation_required {
            anyhow::bail!(
                "relay subscription reconciliation required at sequence {}: {}",
                record.seq,
                record.reason
            );
        }
        let room_is_bound = guard
            .stream_binding
            .as_ref()
            .is_some_and(|binding| binding.rooms.binary_search(&room.to_string()).is_ok());
        if !room_is_bound {
            let mut next = guard.clone();
            next.reconciliation_required = Some(ReconciliationRecord {
                seq,
                reason: format!("relay delivered unbound room '{room}'"),
                detected_at: unix_timestamp(),
                evidence_hash: payload_hash.to_string(),
            });
            self.persist_candidate(&mut next).await?;
            *guard = next;
            anyhow::bail!("relay delivered room outside the durable stream binding");
        }
        if seq <= guard.global_cursor {
            if let Some(existing) = guard.entries.iter().find(|entry| entry.seq == seq)
                && (existing.room != room
                    || existing.msg_type != msg_type
                    || existing.payload_hash != payload_hash
                    || existing.publisher_id.as_deref() != publisher_id)
            {
                anyhow::bail!("relay sequence {seq} conflicts with committed journal evidence");
            }
            return Ok(BeginMessage::Duplicate);
        }
        if guard.entries.iter().any(|entry| entry.seq == seq) {
            let mut next = guard.clone();
            next.reconciliation_required = Some(ReconciliationRecord {
                seq,
                reason: "relay redelivered an unfinished dispatch intent".to_string(),
                detected_at: unix_timestamp(),
                evidence_hash: payload_hash.to_string(),
            });
            self.persist_candidate(&mut next).await?;
            *guard = next;
            anyhow::bail!("relay sequence {seq} requires reconciliation after unfinished intent");
        }

        let unique_subscriptions = expected_subscriptions.iter().collect::<BTreeSet<_>>();
        if unique_subscriptions.len() != expected_subscriptions.len() {
            anyhow::bail!("relay dispatch plan contains duplicate subscription ids");
        }
        for subscription_id in &expected_subscriptions {
            validate_text("subscription id", subscription_id)?;
        }
        ensure_cursor_capacity(&guard, room, &expected_subscriptions)?;

        let mut next = guard.clone();
        make_entry_room(&mut next)?;
        next.entries.push_back(JournalEntry {
            seq,
            room: room.to_string(),
            msg_type: msg_type.to_string(),
            payload_hash: payload_hash.to_string(),
            publisher_id: publisher_id.map(ToString::to_string),
            created_at: unix_timestamp(),
            completed_at: None,
            state: JournalEntryState::Processing {
                expected_subscriptions,
                partial_terminals: Vec::new(),
            },
        });
        self.persist_candidate(&mut next).await?;
        *guard = next;
        Ok(BeginMessage::Dispatch)
    }

    async fn record_terminal(
        &self,
        seq: u64,
        room: &str,
        payload_hash: &str,
        terminal: SubscriptionTerminal,
    ) -> Result<()> {
        let mut guard = self.state.lock().await;
        let mut next = guard.clone();
        let entry = next
            .entries
            .iter_mut()
            .find(|entry| entry.seq == seq)
            .ok_or_else(|| anyhow!("missing relay processing intent for sequence {seq}"))?;
        if entry.room != room || entry.payload_hash != payload_hash {
            anyhow::bail!("relay terminal evidence does not match sequence {seq} intent");
        }
        let (expected, partial_terminals) = match &mut entry.state {
            JournalEntryState::Processing {
                expected_subscriptions,
                partial_terminals,
            } => (expected_subscriptions, partial_terminals),
            _ => anyhow::bail!("relay sequence {seq} is not processing"),
        };
        let next_expected = expected.get(partial_terminals.len()).ok_or_else(|| {
            anyhow!("relay sequence {seq} already has all planned terminal receipts")
        })?;
        if next_expected != terminal.subscription_id() {
            anyhow::bail!("relay sequence {seq} terminal receipt is out of plan order");
        }
        partial_terminals.push(terminal);
        self.persist_candidate(&mut next).await?;
        *guard = next;
        Ok(())
    }

    async fn commit_message(&self, seq: u64, room: &str, payload_hash: &str) -> Result<()> {
        let mut guard = self.state.lock().await;
        let mut next = guard.clone();
        let entry = next
            .entries
            .iter_mut()
            .find(|entry| entry.seq == seq)
            .ok_or_else(|| anyhow!("missing relay processing intent for sequence {seq}"))?;
        if entry.room != room || entry.payload_hash != payload_hash {
            anyhow::bail!("relay terminal evidence does not match sequence {seq} intent");
        }
        let (expected, terminals) = match &entry.state {
            JournalEntryState::Processing {
                expected_subscriptions,
                partial_terminals,
            } => (expected_subscriptions.clone(), partial_terminals.clone()),
            _ => anyhow::bail!("relay sequence {seq} is not processing"),
        };
        let actual = terminals
            .iter()
            .map(|terminal| terminal.subscription_id().to_string())
            .collect::<Vec<_>>();
        if expected != actual {
            anyhow::bail!("relay sequence {seq} terminal receipts do not match dispatch plan");
        }

        entry.completed_at = Some(unix_timestamp());
        entry.state = JournalEntryState::Committed {
            terminals: terminals.clone(),
        };
        next.global_cursor = seq;
        next.room_cursors.insert(room.to_string(), seq);
        for terminal in terminals {
            next.subscription_cursors
                .entry(terminal.subscription_id().to_string())
                .or_default()
                .insert(room.to_string(), seq);
        }
        self.persist_candidate(&mut next).await?;
        *guard = next;
        Ok(())
    }

    async fn fail_message(
        &self,
        seq: u64,
        room: &str,
        payload_hash: &str,
        error: &str,
    ) -> Result<()> {
        let mut guard = self.state.lock().await;
        let mut next = guard.clone();
        let entry = next
            .entries
            .iter_mut()
            .find(|entry| entry.seq == seq)
            .ok_or_else(|| anyhow!("missing relay processing intent for failed sequence {seq}"))?;
        if entry.room != room || entry.payload_hash != payload_hash {
            anyhow::bail!("relay failure evidence does not match sequence {seq} intent");
        }
        validate_text("dispatch error", error)?;
        let partial_terminals = match &entry.state {
            JournalEntryState::Processing {
                partial_terminals, ..
            } => partial_terminals.clone(),
            _ => anyhow::bail!("relay sequence {seq} is not processing"),
        };
        entry.completed_at = Some(unix_timestamp());
        entry.state = JournalEntryState::Failed {
            error: error.to_string(),
            partial_terminals,
        };
        let reason = truncate_error(&format!("dispatch failed before relay ACK: {error}"));
        next.reconciliation_required = Some(ReconciliationRecord {
            seq,
            reason,
            detected_at: unix_timestamp(),
            evidence_hash: payload_hash.to_string(),
        });
        self.persist_candidate(&mut next).await?;
        *guard = next;
        Ok(())
    }

    async fn mark_snapshot(&self, snapshot: &SnapshotMessage) -> Result<()> {
        let snapshot_bytes = serde_json::to_vec(snapshot).context("serialize relay snapshot")?;
        let evidence_hash = blake3::hash(&snapshot_bytes).to_hex().to_string();
        let mut guard = self.state.lock().await;
        let mut next = guard.clone();
        next.reconciliation_required = Some(ReconciliationRecord {
            seq: snapshot.seq,
            reason: "generic relay snapshot cannot reconstruct missed subscription execution"
                .to_string(),
            detected_at: unix_timestamp(),
            evidence_hash,
        });
        self.persist_candidate(&mut next).await?;
        *guard = next;
        Ok(())
    }

    async fn commit_replay_complete(&self, from_seq: u64, to_seq: u64) -> Result<()> {
        if from_seq > to_seq {
            anyhow::bail!("relay replay checkpoint range is reversed");
        }
        let mut guard = self.state.lock().await;
        if let Some(record) = &guard.reconciliation_required {
            anyhow::bail!(
                "relay subscription reconciliation required at sequence {}: {}",
                record.seq,
                record.reason
            );
        }
        if guard.stream_binding.is_none() {
            anyhow::bail!("relay replay checkpoint has no durable stream binding");
        }
        if to_seq <= guard.global_cursor {
            return Ok(());
        }
        if from_seq > guard.global_cursor.saturating_add(1) {
            anyhow::bail!(
                "relay replay checkpoint starts at {from_seq}, beyond durable cursor {}",
                guard.global_cursor
            );
        }

        let mut next = guard.clone();
        make_entry_room(&mut next)?;
        next.entries.push_back(JournalEntry {
            seq: to_seq,
            room: "__replay__".to_string(),
            msg_type: "replay_complete".to_string(),
            payload_hash: blake3::hash(format!("{from_seq}:{to_seq}").as_bytes())
                .to_hex()
                .to_string(),
            publisher_id: None,
            created_at: unix_timestamp(),
            completed_at: Some(unix_timestamp()),
            state: JournalEntryState::ReplayCheckpoint,
        });
        next.global_cursor = to_seq;
        self.persist_candidate(&mut next).await?;
        *guard = next;
        Ok(())
    }

    async fn persist_candidate(&self, candidate: &mut JournalFile) -> Result<()> {
        refresh_integrity(candidate)?;
        validate_journal(candidate)?;
        let bytes = serde_json::to_vec_pretty(candidate).context("serialize relay journal")?;
        if bytes.len() as u64 > JOURNAL_MAX_BYTES {
            anyhow::bail!("relay subscription journal would exceed {JOURNAL_MAX_BYTES} bytes");
        }
        let path = self.path.clone();
        tokio::task::spawn_blocking(move || roko_fs::atomic_write_bytes(&path, &bytes))
            .await
            .context("relay journal persistence task failed")??;
        Ok(())
    }
}

fn make_entry_room(state: &mut JournalFile) -> Result<()> {
    while state.entries.len() >= JOURNAL_MAX_ENTRIES {
        let evictable = state.entries.front().is_some_and(|entry| {
            matches!(
                entry.state,
                JournalEntryState::Committed { .. } | JournalEntryState::ReplayCheckpoint
            )
        });
        if !evictable {
            anyhow::bail!("relay subscription journal is full of unresolved entries");
        }
        state.entries.pop_front();
    }
    Ok(())
}

fn ensure_cursor_capacity(state: &JournalFile, room: &str, subscriptions: &[String]) -> Result<()> {
    let room_count = state.room_cursors.len() + usize::from(!state.room_cursors.contains_key(room));
    let existing_pairs = state
        .subscription_cursors
        .values()
        .map(BTreeMap::len)
        .sum::<usize>();
    let new_pairs = subscriptions
        .iter()
        .filter(|subscription| {
            !state
                .subscription_cursors
                .get(*subscription)
                .is_some_and(|rooms| rooms.contains_key(room))
        })
        .count();
    if room_count > JOURNAL_MAX_CURSOR_KEYS
        || existing_pairs.saturating_add(new_pairs) > JOURNAL_MAX_CURSOR_KEYS
    {
        anyhow::bail!("relay subscription cursor capacity reached");
    }
    Ok(())
}

fn validate_text(label: &str, value: &str) -> Result<()> {
    if value.is_empty() || value.len() > JOURNAL_MAX_TEXT_BYTES {
        anyhow::bail!("relay {label} must contain 1..={JOURNAL_MAX_TEXT_BYTES} UTF-8 bytes");
    }
    Ok(())
}

#[derive(Serialize)]
struct JournalIntegrityView<'a> {
    schema_version: u32,
    global_cursor: u64,
    stream_binding: &'a Option<RelayStreamBinding>,
    room_cursors: &'a BTreeMap<String, u64>,
    subscription_cursors: &'a BTreeMap<String, BTreeMap<String, u64>>,
    entries: &'a VecDeque<JournalEntry>,
    reconciliation_required: &'a Option<ReconciliationRecord>,
}

fn compute_integrity(state: &JournalFile) -> Result<String> {
    let canonical = serde_json::to_vec(&JournalIntegrityView {
        schema_version: state.schema_version,
        global_cursor: state.global_cursor,
        stream_binding: &state.stream_binding,
        room_cursors: &state.room_cursors,
        subscription_cursors: &state.subscription_cursors,
        entries: &state.entries,
        reconciliation_required: &state.reconciliation_required,
    })
    .context("serialize relay journal integrity projection")?;
    Ok(blake3::hash(&canonical).to_hex().to_string())
}

fn refresh_integrity(state: &mut JournalFile) -> Result<()> {
    state.integrity_hash = compute_integrity(state)?;
    Ok(())
}

pub(crate) fn relay_origin_hash(normalized_relay_url: &str) -> String {
    ContentHash::of(normalized_relay_url.as_bytes()).to_hex()
}

fn room_set_hash(rooms: &[String]) -> Result<String> {
    let canonical = serde_json::to_vec(rooms).context("serialize relay room set")?;
    Ok(ContentHash::of(&canonical).to_hex())
}

fn validate_journal(state: &JournalFile) -> Result<()> {
    if state.schema_version != JOURNAL_SCHEMA_VERSION {
        anyhow::bail!(
            "unsupported relay subscription journal schema {}",
            state.schema_version
        );
    }
    if state.integrity_hash != compute_integrity(state)? {
        anyhow::bail!("relay subscription journal integrity checksum mismatch");
    }
    if let Some(binding) = &state.stream_binding {
        if binding.origin_hash.len() != 64
            || ContentHash::from_hex(&binding.origin_hash).is_none()
            || binding.generation == 0
            || binding.rooms.is_empty()
            || binding.rooms.windows(2).any(|pair| pair[0] >= pair[1])
            || binding.rooms.iter().any(|room| !is_exact_relay_room(room))
            || binding.room_set_hash != room_set_hash(&binding.rooms)?
        {
            anyhow::bail!("relay subscription journal stream binding is invalid");
        }
        if state
            .room_cursors
            .keys()
            .chain(state.subscription_cursors.values().flat_map(BTreeMap::keys))
            .any(|room| binding.rooms.binary_search(room).is_err())
        {
            anyhow::bail!("relay subscription cursor references an unbound room");
        }
    } else if state.global_cursor != 0
        || !state.room_cursors.is_empty()
        || !state.subscription_cursors.is_empty()
        || !state.entries.is_empty()
    {
        anyhow::bail!("relay subscription cursors exist without a stream binding");
    }
    if state.entries.len() > JOURNAL_MAX_ENTRIES {
        anyhow::bail!("relay subscription journal entry capacity exceeded");
    }
    if state.room_cursors.len() > JOURNAL_MAX_CURSOR_KEYS
        || state
            .subscription_cursors
            .values()
            .map(BTreeMap::len)
            .sum::<usize>()
            > JOURNAL_MAX_CURSOR_KEYS
    {
        anyhow::bail!("relay subscription journal cursor capacity exceeded");
    }
    if state
        .room_cursors
        .values()
        .chain(
            state
                .subscription_cursors
                .values()
                .flat_map(BTreeMap::values),
        )
        .any(|cursor| *cursor > state.global_cursor)
    {
        anyhow::bail!("relay subscription cursor exceeds global durable cursor");
    }
    for rooms in state.subscription_cursors.values() {
        for (room, cursor) in rooms {
            let Some(room_cursor) = state.room_cursors.get(room) else {
                anyhow::bail!("subscription cursor has no corresponding room cursor");
            };
            if cursor > room_cursor {
                anyhow::bail!("subscription cursor exceeds its corresponding room cursor");
            }
        }
    }

    let mut seen = BTreeSet::new();
    for entry in &state.entries {
        if !seen.insert(entry.seq) {
            anyhow::bail!("relay subscription journal contains duplicate sequence ids");
        }
        validate_text("room", &entry.room)?;
        validate_text("message type", &entry.msg_type)?;
        if entry.room != "__replay__"
            && state
                .stream_binding
                .as_ref()
                .is_none_or(|binding| binding.rooms.binary_search(&entry.room).is_err())
        {
            anyhow::bail!("relay journal entry references an unbound room");
        }
        if let Some(publisher_id) = &entry.publisher_id {
            validate_text("publisher id", publisher_id)?;
        }
        match &entry.state {
            JournalEntryState::Processing {
                expected_subscriptions,
                partial_terminals,
            } => {
                if entry.seq <= state.global_cursor {
                    anyhow::bail!("processing relay intent is behind the durable cursor");
                }
                for subscription in expected_subscriptions {
                    validate_text("subscription id", subscription)?;
                }
                validate_terminal_prefix(expected_subscriptions, partial_terminals)?;
            }
            JournalEntryState::Committed { terminals } => {
                if entry.seq > state.global_cursor {
                    anyhow::bail!("committed relay entry exceeds the durable cursor");
                }
                for terminal in terminals {
                    validate_text("subscription id", terminal.subscription_id())?;
                }
            }
            JournalEntryState::Failed {
                error,
                partial_terminals,
            } => {
                validate_text("dispatch error", error)?;
                for terminal in partial_terminals {
                    validate_text("subscription id", terminal.subscription_id())?;
                }
            }
            JournalEntryState::ReplayCheckpoint => {
                if entry.seq > state.global_cursor {
                    anyhow::bail!("replay checkpoint exceeds the durable cursor");
                }
            }
        }
    }
    if let Some(record) = &state.reconciliation_required {
        validate_text("reconciliation reason", &record.reason)?;
    }
    Ok(())
}

fn validate_terminal_prefix(
    expected: &[String],
    partial_terminals: &[SubscriptionTerminal],
) -> Result<()> {
    if partial_terminals.len() > expected.len()
        || partial_terminals
            .iter()
            .zip(expected)
            .any(|(terminal, expected)| terminal.subscription_id() != expected)
    {
        anyhow::bail!("relay journal partial terminals are not a dispatch-plan prefix");
    }
    Ok(())
}

fn persist_journal_sync(path: &Path, state: &JournalFile) -> Result<()> {
    let mut durable = state.clone();
    refresh_integrity(&mut durable)?;
    validate_journal(&durable)?;
    let bytes = serde_json::to_vec_pretty(&durable).context("serialize relay journal")?;
    if bytes.len() as u64 > JOURNAL_MAX_BYTES {
        anyhow::bail!("relay subscription journal exceeds {JOURNAL_MAX_BYTES} bytes");
    }
    roko_fs::atomic_write_bytes(path, &bytes)
        .with_context(|| format!("persist relay journal {}", path.display()))
}

#[async_trait]
trait RelayDispatchExecutor: Send + Sync {
    async fn execute(&self, subscription: Subscription, signal: Signal)
    -> Result<DispatchTerminal>;
}

struct ServeDispatchExecutor {
    state: Arc<AppState>,
    dispatcher: Arc<dyn AgentDispatcher>,
}

#[async_trait]
impl RelayDispatchExecutor for ServeDispatchExecutor {
    async fn execute(
        &self,
        subscription: Subscription,
        signal: Signal,
    ) -> Result<DispatchTerminal> {
        dispatch_relay_subscription(
            Arc::clone(&self.state),
            subscription,
            signal,
            Arc::clone(&self.dispatcher),
        )
        .await
    }
}

/// Durable topic handler supplied to the supervised relay client.
pub(crate) struct ServeTopicHandler {
    runtime: Arc<SubscriptionRelayRuntime>,
    subscriptions: SubscriptionRegistry,
    executor: Arc<dyn RelayDispatchExecutor>,
}

impl ServeTopicHandler {
    pub(crate) fn new(
        runtime: Arc<SubscriptionRelayRuntime>,
        state: Arc<AppState>,
        dispatcher: Arc<dyn AgentDispatcher>,
    ) -> Self {
        Self {
            subscriptions: state.subscriptions.clone(),
            runtime,
            executor: Arc::new(ServeDispatchExecutor { state, dispatcher }),
        }
    }

    #[cfg(test)]
    fn with_executor(
        runtime: Arc<SubscriptionRelayRuntime>,
        subscriptions: SubscriptionRegistry,
        executor: Arc<dyn RelayDispatchExecutor>,
    ) -> Self {
        Self {
            runtime,
            subscriptions,
            executor,
        }
    }

    async fn execute_terminal(
        &self,
        subscription: Subscription,
        signal: &Signal,
        dedup_key: ContentHash,
    ) -> Result<SubscriptionTerminal> {
        let subscription_id = subscription.id.clone();
        let Some(_permit) = SubscriptionPermit::try_acquire(&self.subscriptions, &subscription)
        else {
            return Ok(suppressed_terminal(
                subscription_id,
                "concurrency_limit",
                "subscription concurrency capacity was exhausted",
            ));
        };
        if !self.subscriptions.check_cooldown(&subscription) {
            return Ok(suppressed_terminal(
                subscription_id,
                "cooldown",
                "subscription cooldown window is still active",
            ));
        }
        if !subscription.check_dedup_key(dedup_key) {
            return Ok(suppressed_terminal(
                subscription_id,
                "dedup",
                "relay content was already admitted within the deduplication window",
            ));
        }
        if !subscription.check_debounce() {
            return Ok(suppressed_terminal(
                subscription_id,
                "debounce",
                "signal arrived inside the configured debounce window",
            ));
        }
        self.executor
            .execute(subscription, signal.clone())
            .await
            .map(|receipt| SubscriptionTerminal::Dispatched { receipt })
    }
}

#[async_trait]
impl TopicHandler for ServeTopicHandler {
    async fn durable_cursor(&self) -> Result<u64> {
        Ok(self.runtime.journal.durable_cursor().await)
    }

    async fn on_topic_message(
        &self,
        topic: &str,
        msg_type: &str,
        payload: Value,
        publisher_id: Option<&str>,
        seq: u64,
    ) -> Result<()> {
        let envelope_identity =
            serde_json::to_vec(&(topic, msg_type, &payload, publisher_id.unwrap_or_default()))
                .context("serialize relay envelope identity")?;
        let payload_hash = ContentHash::of(&envelope_identity).to_hex();
        let dedup_key = ContentHash::of(&envelope_identity);
        let body = Body::from_json(&payload).context("convert relay payload to signal body")?;
        let mut builder = Signal::builder(Kind::Custom(topic.to_string()))
            .body(body)
            .provenance(Provenance::external(
                publisher_id.unwrap_or("roko-relay/unknown-publisher"),
            ))
            .tag("relay_room", topic)
            .tag("relay_message_type", msg_type)
            .tag("relay_seq", seq.to_string());
        if let Some(publisher_id) = publisher_id {
            builder = builder.tag("relay_publisher_id", publisher_id);
        }
        let signal = builder.build();

        let mut matched = self
            .subscriptions
            .all()
            .into_iter()
            .filter(|subscription| {
                is_remote_subscription_eligible(subscription, topic)
                    && subscription.matches(&signal)
            })
            .collect::<Vec<_>>();
        matched.sort_by(|left, right| left.id.cmp(&right.id));
        let expected = matched.iter().map(|sub| sub.id.clone()).collect();
        match self
            .runtime
            .journal
            .begin_message(seq, topic, msg_type, &payload_hash, publisher_id, expected)
            .await?
        {
            BeginMessage::Duplicate => return Ok(()),
            BeginMessage::Dispatch => {}
        }

        for subscription in matched {
            let terminal = match self
                .execute_terminal(subscription, &signal, dedup_key)
                .await
            {
                Ok(terminal) => terminal,
                Err(error) => {
                    let reason = truncate_error(&error.to_string());
                    self.runtime
                        .journal
                        .fail_message(seq, topic, &payload_hash, &reason)
                        .await
                        .context("persist relay dispatch failure")?;
                    return Err(error).context("relay subscription dispatch failed");
                }
            };
            self.runtime
                .journal
                .record_terminal(seq, topic, &payload_hash, terminal)
                .await
                .context("persist relay subscription terminal receipt")?;
        }

        self.runtime
            .journal
            .commit_message(seq, topic, &payload_hash)
            .await
            .context("commit relay dispatch terminal results")
    }

    async fn on_snapshot(&self, snapshot: &SnapshotMessage) -> Result<SnapshotDisposition> {
        self.runtime.journal.mark_snapshot(snapshot).await?;
        Ok(SnapshotDisposition::ReconciliationRequired)
    }

    async fn on_replay_complete(&self, from_seq: u64, to_seq: u64) -> Result<()> {
        self.runtime
            .journal
            .commit_replay_complete(from_seq, to_seq)
            .await
    }
}

fn suppressed_terminal(
    subscription_id: String,
    policy: &str,
    detail: &str,
) -> SubscriptionTerminal {
    SubscriptionTerminal::Suppressed {
        subscription_id,
        policy: policy.to_string(),
        detail: detail.to_string(),
    }
}

fn truncate_error(error: &str) -> String {
    if error.len() <= JOURNAL_MAX_TEXT_BYTES {
        return error.to_string();
    }
    let mut end = JOURNAL_MAX_TEXT_BYTES;
    while !error.is_char_boundary(end) {
        end -= 1;
    }
    error[..end].to_string()
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use roko_agent_server::features::relay_client::TopicHandler;
    use roko_core::config::schema::SubscriptionConfig;
    use tempfile::tempdir;

    use super::*;

    struct FakeExecutor {
        calls: AtomicUsize,
        fail_at: Option<usize>,
        signals: std::sync::Mutex<Vec<Signal>>,
    }

    struct BlockingExecutor {
        entered: tokio::sync::Notify,
    }

    #[async_trait]
    impl RelayDispatchExecutor for BlockingExecutor {
        async fn execute(
            &self,
            _subscription: Subscription,
            _signal: Signal,
        ) -> Result<DispatchTerminal> {
            self.entered.notify_one();
            std::future::pending::<Result<DispatchTerminal>>().await
        }
    }

    fn test_subscription(id: &str, trigger: &str) -> Subscription {
        let mut subscription = Subscription::from_config(SubscriptionConfig {
            template: id.to_string(),
            trigger: trigger.to_string(),
            ..SubscriptionConfig::default()
        });
        subscription.id = id.to_string();
        subscription
    }

    #[async_trait]
    impl RelayDispatchExecutor for FakeExecutor {
        async fn execute(
            &self,
            subscription: Subscription,
            signal: Signal,
        ) -> Result<DispatchTerminal> {
            let call = self.calls.fetch_add(1, Ordering::SeqCst) + 1;
            self.signals.lock().expect("signals lock").push(signal);
            if self.fail_at == Some(call) {
                anyhow::bail!("injected terminal dispatch failure");
            }
            Ok(DispatchTerminal {
                subscription_id: subscription.id,
                template: subscription.template,
                episode_id: "episode-1".to_string(),
                output_signal_hash: "ab".repeat(32),
                success: true,
            })
        }
    }

    async fn make_handler(
        workdir: &Path,
        fail: bool,
    ) -> (
        Arc<SubscriptionRelayRuntime>,
        ServeTopicHandler,
        Arc<FakeExecutor>,
    ) {
        let runtime = Arc::new(
            SubscriptionRelayRuntime::open(workdir, true).expect("open subscription relay"),
        );
        runtime
            .bind_stream(
                &relay_origin_hash("ws://relay.test"),
                &["feed:prices".to_string()],
            )
            .await
            .expect("bind relay stream");
        let mut subscription = Subscription::from_config(SubscriptionConfig {
            template: "worker".to_string(),
            trigger: "feed:prices".to_string(),
            ..SubscriptionConfig::default()
        });
        subscription.id = "prices-worker".to_string();
        let subscriptions = SubscriptionRegistry::with_subscriptions(vec![subscription]);
        let executor = Arc::new(FakeExecutor {
            calls: AtomicUsize::new(0),
            fail_at: fail.then_some(1),
            signals: std::sync::Mutex::new(Vec::new()),
        });
        let handler =
            ServeTopicHandler::with_executor(Arc::clone(&runtime), subscriptions, executor.clone());
        (runtime, handler, executor)
    }

    #[tokio::test]
    async fn committed_message_reloads_cursor_and_suppresses_duplicate_after_restart() {
        let dir = tempdir().expect("tempdir");
        let (runtime, handler, executor) = make_handler(dir.path(), false).await;
        handler
            .on_topic_message(
                "feed:prices",
                "tick",
                serde_json::json!({"price": 7}),
                None,
                11,
            )
            .await
            .expect("commit topic message");
        assert_eq!(handler.durable_cursor().await.expect("cursor"), 11);
        assert_eq!(executor.calls.load(Ordering::SeqCst), 1);
        drop(handler);
        drop(runtime);

        let (runtime, restarted, restarted_executor) = make_handler(dir.path(), false).await;
        assert_eq!(
            restarted.durable_cursor().await.expect("reloaded cursor"),
            11
        );
        restarted
            .on_topic_message(
                "feed:prices",
                "tick",
                serde_json::json!({"price": 7}),
                None,
                11,
            )
            .await
            .expect("duplicate is already durable");
        assert_eq!(restarted_executor.calls.load(Ordering::SeqCst), 0);
        let status = runtime.status().await;
        assert_eq!(status.room_cursors.get("feed:prices"), Some(&11));
        assert_eq!(
            status
                .subscription_cursors
                .get("prices-worker")
                .and_then(|rooms| rooms.get("feed:prices")),
            Some(&11)
        );
    }

    #[tokio::test]
    async fn terminal_failure_is_durable_and_does_not_advance_cursor() {
        let dir = tempdir().expect("tempdir");
        let (runtime, handler, executor) = make_handler(dir.path(), true).await;
        let error = handler
            .on_topic_message(
                "feed:prices",
                "tick",
                serde_json::json!({"price": 9}),
                None,
                12,
            )
            .await
            .expect_err("failure must block ACK");
        assert!(
            error
                .to_string()
                .contains("relay subscription dispatch failed")
        );
        assert_eq!(executor.calls.load(Ordering::SeqCst), 1);
        assert_eq!(handler.durable_cursor().await.expect("cursor"), 0);
        let status = runtime.status().await;
        assert_eq!(status.failed_entries, 1);
        assert_eq!(status.reconciliation_required.expect("reconcile").seq, 12);
    }

    #[tokio::test]
    async fn replay_complete_advances_quiet_room_watermark_durably() {
        let dir = tempdir().expect("tempdir");
        let (_runtime, handler, _) = make_handler(dir.path(), false).await;
        handler
            .on_replay_complete(1, 44)
            .await
            .expect("commit quiet replay watermark");
        assert_eq!(handler.durable_cursor().await.expect("cursor"), 44);

        let (_runtime, restarted, _) = make_handler(dir.path(), false).await;
        assert_eq!(
            restarted.durable_cursor().await.expect("reloaded cursor"),
            44
        );
    }

    #[tokio::test]
    async fn generic_snapshot_sets_reconciliation_without_advancing_cursor() {
        let dir = tempdir().expect("tempdir");
        let (runtime, handler, _) = make_handler(dir.path(), false).await;
        let snapshot = SnapshotMessage {
            seq: 71,
            state: serde_json::json!({"rooms": ["feed:prices"]}),
        };
        assert_eq!(
            handler
                .on_snapshot(&snapshot)
                .await
                .expect("inspect snapshot"),
            SnapshotDisposition::ReconciliationRequired
        );
        assert_eq!(handler.durable_cursor().await.expect("cursor"), 0);
        let status = runtime.status().await;
        let reconciliation = status
            .reconciliation_required
            .expect("reconciliation status");
        assert_eq!(reconciliation.seq, 71);
        assert!(reconciliation.reason.contains("cannot reconstruct"));
    }

    #[tokio::test]
    async fn later_dispatch_failure_retains_prior_terminal_receipt() {
        let dir = tempdir().expect("tempdir");
        let runtime = Arc::new(
            SubscriptionRelayRuntime::open(dir.path(), true).expect("open subscription relay"),
        );
        runtime
            .bind_stream(
                &relay_origin_hash("ws://relay.test"),
                &["feed:prices".to_string()],
            )
            .await
            .expect("bind relay stream");
        let subscriptions = ["a-worker", "b-worker"]
            .into_iter()
            .map(|id| {
                let mut subscription = Subscription::from_config(SubscriptionConfig {
                    template: id.to_string(),
                    trigger: "feed:prices".to_string(),
                    ..SubscriptionConfig::default()
                });
                subscription.id = id.to_string();
                subscription
            })
            .collect();
        let executor = Arc::new(FakeExecutor {
            calls: AtomicUsize::new(0),
            fail_at: Some(2),
            signals: std::sync::Mutex::new(Vec::new()),
        });
        let handler = ServeTopicHandler::with_executor(
            Arc::clone(&runtime),
            SubscriptionRegistry::with_subscriptions(subscriptions),
            executor,
        );
        handler
            .on_topic_message(
                "feed:prices",
                "tick",
                serde_json::json!({"price": 11}),
                None,
                20,
            )
            .await
            .expect_err("second dispatch fails");
        let state = runtime.journal.state.lock().await;
        let entry = state.entries.back().expect("failed journal entry");
        let JournalEntryState::Failed {
            partial_terminals, ..
        } = &entry.state
        else {
            panic!("expected failed journal entry");
        };
        assert_eq!(partial_terminals.len(), 1);
        assert_eq!(partial_terminals[0].subscription_id(), "a-worker");
        assert_eq!(state.global_cursor, 0);
    }

    #[tokio::test]
    async fn same_process_unfinished_redelivery_persists_reconciliation() {
        let dir = tempdir().expect("tempdir");
        let journal = SubscriptionRelayJournal::open(journal_path(dir.path())).expect("journal");
        journal
            .bind_stream(
                &relay_origin_hash("ws://relay.test"),
                &["feed:prices".to_string()],
            )
            .await
            .expect("bind relay stream");
        journal
            .begin_message(
                8,
                "feed:prices",
                "tick",
                &"ef".repeat(32),
                None,
                vec!["prices-worker".to_string()],
            )
            .await
            .expect("processing intent");
        let error = journal
            .begin_message(
                8,
                "feed:prices",
                "tick",
                &"ef".repeat(32),
                None,
                vec!["prices-worker".to_string()],
            )
            .await
            .expect_err("unfinished intent must fail closed");
        assert!(error.to_string().contains("requires reconciliation"));
        assert_eq!(
            journal
                .state
                .lock()
                .await
                .reconciliation_required
                .as_ref()
                .expect("reconciliation")
                .seq,
            8
        );
    }

    #[tokio::test]
    async fn debounce_suppression_is_terminal_and_releases_concurrency() {
        let dir = tempdir().expect("tempdir");
        let runtime = Arc::new(
            SubscriptionRelayRuntime::open(dir.path(), true).expect("open subscription relay"),
        );
        runtime
            .bind_stream(
                &relay_origin_hash("ws://relay.test"),
                &["feed:prices".to_string()],
            )
            .await
            .expect("bind relay stream");
        let mut subscription = Subscription::from_config(SubscriptionConfig {
            template: "worker".to_string(),
            trigger: "feed:prices".to_string(),
            concurrency_limit: 1,
            debounce_ms: 60_000,
            ..SubscriptionConfig::default()
        });
        subscription.id = "prices-worker".to_string();
        let registry = SubscriptionRegistry::with_subscriptions(vec![subscription]);
        let executor = Arc::new(FakeExecutor {
            calls: AtomicUsize::new(0),
            fail_at: None,
            signals: std::sync::Mutex::new(Vec::new()),
        });
        let handler = ServeTopicHandler::with_executor(
            Arc::clone(&runtime),
            registry.clone(),
            executor.clone(),
        );
        handler
            .on_topic_message(
                "feed:prices",
                "tick",
                serde_json::json!({"price": 1}),
                None,
                1,
            )
            .await
            .expect("first dispatch");
        handler
            .on_topic_message(
                "feed:prices",
                "tick",
                serde_json::json!({"price": 2}),
                None,
                2,
            )
            .await
            .expect("debounce suppression commits");
        assert_eq!(executor.calls.load(Ordering::SeqCst), 1);
        let state = runtime.journal.state.lock().await;
        let JournalEntryState::Committed { terminals } = &state.entries[1].state else {
            panic!("expected committed suppression");
        };
        assert!(matches!(
            &terminals[0],
            SubscriptionTerminal::Suppressed { policy, .. } if policy == "debounce"
        ));
        drop(state);
        let subscription = registry.get_by_id("prices-worker").expect("subscription");
        assert!(registry.check_concurrency_limit(&subscription));
        registry.release_concurrency(&subscription);
    }

    #[tokio::test]
    async fn stable_content_dedup_suppresses_identical_payload_at_new_sequence() {
        let dir = tempdir().expect("tempdir");
        let (runtime, handler, executor) = make_handler(dir.path(), false).await;
        for seq in [1, 2] {
            handler
                .on_topic_message(
                    "feed:prices",
                    "tick",
                    serde_json::json!({"price": 13}),
                    Some("publisher-a"),
                    seq,
                )
                .await
                .expect("commit message");
        }
        assert_eq!(executor.calls.load(Ordering::SeqCst), 1);
        assert_eq!(handler.durable_cursor().await.expect("cursor"), 2);
        let state = runtime.journal.state.lock().await;
        let JournalEntryState::Committed { terminals } = &state.entries[1].state else {
            panic!("expected committed duplicate suppression");
        };
        assert!(matches!(
            &terminals[0],
            SubscriptionTerminal::Suppressed { policy, .. } if policy == "dedup"
        ));
    }

    #[tokio::test]
    async fn duplicate_sequence_conflict_binds_message_type_and_publisher() {
        let dir = tempdir().expect("tempdir");
        let (_runtime, handler, executor) = make_handler(dir.path(), false).await;
        handler
            .on_topic_message(
                "feed:prices",
                "tick",
                serde_json::json!({"price": 21}),
                Some("publisher-a"),
                7,
            )
            .await
            .expect("initial message");
        let error = handler
            .on_topic_message(
                "feed:prices",
                "correction",
                serde_json::json!({"price": 21}),
                Some("publisher-b"),
                7,
            )
            .await
            .expect_err("same sequence with changed envelope must fail");
        assert!(error.to_string().contains("conflicts"));
        assert_eq!(executor.calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn relay_signal_is_external_and_preserves_publisher_provenance() {
        let dir = tempdir().expect("tempdir");
        let (_runtime, handler, executor) = make_handler(dir.path(), false).await;
        handler
            .on_topic_message(
                "feed:prices",
                "tick",
                serde_json::json!({"price": 34}),
                Some("market-source"),
                1,
            )
            .await
            .expect("dispatch message");
        let signals = executor.signals.lock().expect("signals lock");
        let signal = signals.first().expect("captured signal");
        assert_eq!(signal.provenance.author, "market-source");
        assert_eq!(format!("{:?}", signal.provenance.trust_origin), "External");
        assert!(signal.provenance.is_tainted());
    }

    #[tokio::test]
    async fn handler_and_room_plan_share_exact_remote_eligibility() {
        let dir = tempdir().expect("tempdir");
        let runtime = Arc::new(
            SubscriptionRelayRuntime::open(dir.path(), true).expect("open subscription relay"),
        );
        runtime
            .bind_stream(
                &relay_origin_hash("ws://relay.test"),
                &["feed:prices".to_string()],
            )
            .await
            .expect("bind relay stream");
        let exact = test_subscription("exact", "feed:prices");
        let wildcard = test_subscription("wildcard", "feed:*");
        let mut cron = test_subscription("cron", "feed:prices");
        cron.trigger_config = Some(SubscriptionTrigger::Cron {
            schedule: "0 * * * *".to_string(),
        });
        let registry = SubscriptionRegistry::with_subscriptions(vec![exact, wildcard, cron]);
        let plan = remote_subscription_plan(&registry, 64);
        assert_eq!(plan.rooms, vec!["feed:prices"]);
        assert_eq!(plan.unsupported_triggers, vec!["wildcard:feed:*"]);

        let executor = Arc::new(FakeExecutor {
            calls: AtomicUsize::new(0),
            fail_at: None,
            signals: std::sync::Mutex::new(Vec::new()),
        });
        let handler = ServeTopicHandler::with_executor(runtime, registry, executor.clone());
        handler
            .on_topic_message(
                "feed:prices",
                "tick",
                serde_json::json!({"price": 55}),
                None,
                1,
            )
            .await
            .expect("dispatch exact remote subscription only");
        assert_eq!(executor.calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn cancellation_drops_concurrency_reservation() {
        let dir = tempdir().expect("tempdir");
        let runtime = Arc::new(
            SubscriptionRelayRuntime::open(dir.path(), true).expect("open subscription relay"),
        );
        runtime
            .bind_stream(
                &relay_origin_hash("ws://relay.test"),
                &["feed:prices".to_string()],
            )
            .await
            .expect("bind relay stream");
        let mut subscription = test_subscription("blocking", "feed:prices");
        subscription.concurrency_limit = 1;
        let registry = SubscriptionRegistry::with_subscriptions(vec![subscription.clone()]);
        let executor = Arc::new(BlockingExecutor {
            entered: tokio::sync::Notify::new(),
        });
        let entered = executor.entered.notified();
        let handler = Arc::new(ServeTopicHandler::with_executor(
            runtime,
            registry.clone(),
            executor.clone(),
        ));
        let task_handler = Arc::clone(&handler);
        let task = tokio::spawn(async move {
            task_handler
                .on_topic_message(
                    "feed:prices",
                    "tick",
                    serde_json::json!({"price": 89}),
                    None,
                    1,
                )
                .await
        });
        tokio::time::timeout(std::time::Duration::from_secs(2), entered)
            .await
            .expect("executor entered");
        task.abort();
        let _ = task.await;

        assert!(registry.check_concurrency_limit(&subscription));
        registry.release_concurrency(&subscription);
    }

    #[tokio::test]
    async fn room_set_expansion_after_cursor_requires_durable_reconciliation() {
        let dir = tempdir().expect("tempdir");
        let (runtime, handler, _) = make_handler(dir.path(), false).await;
        handler
            .on_topic_message(
                "feed:prices",
                "tick",
                serde_json::json!({"price": 144}),
                None,
                9,
            )
            .await
            .expect("advance cursor");
        let proposed = vec!["feed:news".to_string(), "feed:prices".to_string()];
        let error = runtime
            .guard_stream_change(&relay_origin_hash("ws://relay.test"), &proposed)
            .await
            .expect_err("room expansion must not reuse global cursor");
        assert!(error.to_string().contains("room set changed"));
        let status = runtime.status().await;
        assert_eq!(status.global_cursor, 9);
        assert_eq!(
            status.stream_binding.expect("binding").rooms,
            vec!["feed:prices"]
        );
        assert!(status.reconciliation_required.is_some());
    }

    #[tokio::test]
    async fn zero_cursor_room_set_rebinds_with_new_generation() {
        let dir = tempdir().expect("tempdir");
        let runtime = SubscriptionRelayRuntime::open(dir.path(), true).expect("runtime");
        let origin = relay_origin_hash("ws://relay.test");
        runtime
            .bind_stream(&origin, &["feed:prices".to_string()])
            .await
            .expect("initial binding");
        runtime
            .bind_stream(
                &origin,
                &["feed:news".to_string(), "feed:prices".to_string()],
            )
            .await
            .expect("safe zero-cursor rebind");
        let binding = runtime.status().await.stream_binding.expect("binding");
        assert_eq!(binding.generation, 2);
        assert_eq!(binding.rooms, vec!["feed:news", "feed:prices"]);
    }

    #[tokio::test]
    async fn relay_origin_change_requires_reconciliation_even_at_zero_cursor() {
        let dir = tempdir().expect("tempdir");
        let runtime = SubscriptionRelayRuntime::open(dir.path(), true).expect("runtime");
        let rooms = vec!["feed:prices".to_string()];
        runtime
            .bind_stream(&relay_origin_hash("ws://relay-a.test"), &rooms)
            .await
            .expect("initial binding");
        runtime
            .guard_stream_change(&relay_origin_hash("ws://relay-b.test"), &rooms)
            .await
            .expect_err("origin change must fail closed");
        let status = runtime.status().await;
        assert_eq!(status.global_cursor, 0);
        assert!(
            status
                .reconciliation_required
                .expect("reconciliation")
                .reason
                .contains("origin identity changed")
        );
    }

    #[tokio::test]
    async fn capacity_and_unsupported_diagnostics_exist_before_connect() {
        let mut subscriptions = (0..65)
            .map(|index| test_subscription(&format!("exact-{index}"), &format!("feed:{index}")))
            .collect::<Vec<_>>();
        subscriptions.push(test_subscription("wild", "feed:*"));
        let registry = SubscriptionRegistry::with_subscriptions(subscriptions);
        let plan = remote_subscription_plan(&registry, 64);
        assert_eq!(plan.capacity_rejected_rooms.len(), 65);
        assert_eq!(plan.unsupported_triggers, vec!["wild:feed:*"]);

        let dir = tempdir().expect("tempdir");
        let runtime = SubscriptionRelayRuntime::open(dir.path(), true).expect("runtime");
        runtime
            .set_subscription_plan_diagnostics(
                plan.unsupported_triggers.clone(),
                plan.capacity_rejected_rooms.clone(),
            )
            .await;
        let status = runtime.status().await;
        assert_eq!(status.capacity_rejected_rooms.len(), 65);
        assert_eq!(status.unsupported_triggers, vec!["wild:feed:*"]);
    }

    #[tokio::test]
    async fn checksum_tampering_is_rejected_on_restart() {
        let dir = tempdir().expect("tempdir");
        let (runtime, handler, _) = make_handler(dir.path(), false).await;
        handler
            .on_topic_message(
                "feed:prices",
                "tick",
                serde_json::json!({"price": 233}),
                None,
                1,
            )
            .await
            .expect("commit message");
        drop(handler);
        drop(runtime);
        let path = journal_path(dir.path());
        let mut value: Value = serde_json::from_slice(&std::fs::read(&path).expect("read journal"))
            .expect("parse journal");
        value["global_cursor"] = serde_json::json!(99);
        std::fs::write(
            &path,
            serde_json::to_vec_pretty(&value).expect("serialize tamper"),
        )
        .expect("write tamper");
        let error = match SubscriptionRelayJournal::open(path) {
            Ok(_) => panic!("checksum mismatch must be rejected"),
            Err(error) => error,
        };
        assert!(error.to_string().contains("checksum mismatch"));
    }

    #[tokio::test]
    async fn subscription_cursor_requires_corresponding_bound_room_cursor() {
        let dir = tempdir().expect("tempdir");
        let path = journal_path(dir.path());
        let journal = SubscriptionRelayJournal::open(path.clone()).expect("journal");
        journal
            .bind_stream(
                &relay_origin_hash("ws://relay.test"),
                &["feed:news".to_string(), "feed:prices".to_string()],
            )
            .await
            .expect("bind stream");
        let mut state = journal.state.lock().await.clone();
        state.global_cursor = 5;
        state
            .subscription_cursors
            .entry("prices-worker".to_string())
            .or_default()
            .insert("feed:news".to_string(), 5);
        refresh_integrity(&mut state).expect("refresh checksum");
        std::fs::write(
            &path,
            serde_json::to_vec_pretty(&state).expect("serialize invalid projection"),
        )
        .expect("write invalid projection");
        let error = match SubscriptionRelayJournal::open(path) {
            Ok(_) => panic!("invalid cursor projection must be rejected"),
            Err(error) => error,
        };
        assert!(error.to_string().contains("no corresponding room cursor"));
    }

    #[test]
    fn oversized_journal_is_rejected_before_deserialization() {
        let dir = tempdir().expect("tempdir");
        let path = journal_path(dir.path());
        std::fs::create_dir_all(path.parent().expect("journal parent"))
            .expect("create journal parent");
        std::fs::write(&path, vec![b' '; JOURNAL_MAX_BYTES as usize + 1])
            .expect("write oversized journal");
        let error = match SubscriptionRelayJournal::open(path) {
            Ok(_) => panic!("oversized journal must be rejected"),
            Err(error) => error,
        };
        assert!(error.to_string().contains("limit"));
    }

    #[test]
    fn restart_marks_interrupted_processing_intent_for_reconciliation() {
        let dir = tempdir().expect("tempdir");
        let path = journal_path(dir.path());
        let mut state = JournalFile::default();
        let rooms = vec!["feed:prices".to_string()];
        state.stream_binding = Some(RelayStreamBinding {
            origin_hash: relay_origin_hash("ws://relay.test"),
            room_set_hash: room_set_hash(&rooms).expect("room set hash"),
            rooms,
            generation: 1,
        });
        state.entries.push_back(JournalEntry {
            seq: 5,
            room: "feed:prices".to_string(),
            msg_type: "tick".to_string(),
            payload_hash: "cd".repeat(32),
            publisher_id: None,
            created_at: 1,
            completed_at: None,
            state: JournalEntryState::Processing {
                expected_subscriptions: vec!["prices-worker".to_string()],
                partial_terminals: Vec::new(),
            },
        });
        persist_journal_sync(&path, &state).expect("persist interrupted intent");
        let journal = SubscriptionRelayJournal::open(path).expect("reopen journal");
        let status = journal
            .state
            .blocking_lock()
            .reconciliation_required
            .clone();
        assert_eq!(status.expect("reconciliation").seq, 5);
    }
}
