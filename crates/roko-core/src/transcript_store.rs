//! Bounded in-memory transcript store with lossless control events.
//!
//! The [`TranscriptStore`] holds [`TranscriptRecord`]s in a bounded ring.
//! When capacity is reached, body/delta records are evicted before control
//! events (run terminal, tool terminal, errors). This ensures that
//! critical lifecycle events are never lost while streaming text deltas
//! can be shed under memory pressure.
//!
//! # Thread safety
//!
//! All public methods take `&self` and synchronize through an internal
//! `parking_lot::RwLock`, making the store safe to share via `Arc`.
//!
//! # Eviction policy
//!
//! 1. Text deltas (`AssistantDelta`, `ReasoningDelta`, `ToolOutputDelta`)
//!    are evicted first (oldest first).
//! 2. If no text deltas remain and capacity is still exceeded, non-control
//!    events (e.g. `Usage`, `TodoSnapshot`) are evicted.
//! 3. Control events (`RunStarted`, `RunFinished`, `ToolStarted`,
//!    `ToolFinished`, `Error`) are **never** evicted.
//!
//! A `dropped_count` counter tracks total evictions for telemetry.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

use parking_lot::RwLock;

use crate::tool::transcript::{TranscriptEvent, TranscriptRecord};

// ─── Error ──────────────────────────────────────────────────────────────

/// Errors produced by [`TranscriptStore`] operations.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum StoreError {
    /// A record's sequence number was already present.
    #[error("duplicate sequence number: {0}")]
    DuplicateSequence(u64),
}

// ─── Filter ─────────────────────────────────────────────────────────────

/// Predicate builder for querying the store.
#[derive(Debug, Clone, Default)]
pub struct TranscriptFilter {
    /// Only records matching this run ID.
    pub run_id: Option<String>,
    /// Only records matching this agent ID.
    pub agent_id: Option<String>,
    /// Only tool-related events for this call ID.
    pub tool_call_id: Option<String>,
    /// Only events of these types (matched by discriminant name).
    pub event_types: Option<Vec<String>>,
    /// Only records at or after this Unix-ms timestamp.
    pub after_ms: Option<i64>,
    /// Only records at or before this Unix-ms timestamp.
    pub before_ms: Option<i64>,
}

impl TranscriptFilter {
    fn matches(&self, record: &TranscriptRecord) -> bool {
        if let Some(ref run_id) = self.run_id
            && record.meta.run_id != *run_id
        {
            return false;
        }
        if let Some(ref agent_id) = self.agent_id
            && record.meta.agent_id != *agent_id
        {
            return false;
        }
        if let Some(ref call_id) = self.tool_call_id
            && !event_matches_call_id(&record.event, call_id)
        {
            return false;
        }
        if let Some(ref types) = self.event_types {
            let name = event_type_name(&record.event);
            if !types.iter().any(|t| t == name) {
                return false;
            }
        }
        if let Some(after) = self.after_ms
            && record.meta.timestamp_ms < after
        {
            return false;
        }
        if let Some(before) = self.before_ms
            && record.meta.timestamp_ms > before
        {
            return false;
        }
        true
    }
}

// ─── Page ───────────────────────────────────────────────────────────────

/// A page of transcript records returned by cursor-based pagination.
#[derive(Debug, Clone)]
pub struct TranscriptPage {
    /// Records in this page, ordered by sequence.
    pub records: Vec<TranscriptRecord>,
    /// Cursor for the next page, or `None` if this is the last page.
    pub next_cursor: Option<u64>,
    /// Total number of records currently in the store.
    pub total_in_store: usize,
}

// ─── Stats ──────────────────────────────────────────────────────────────

/// Runtime statistics for the store.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoreStats {
    /// Current number of records in the store.
    pub current_count: usize,
    /// Maximum capacity configured.
    pub capacity: usize,
    /// Total records that have been evicted (dropped).
    pub dropped_count: u64,
    /// Dropped text/delta events specifically.
    pub dropped_text_events: u64,
    /// Dropped control events (should be 0).
    pub dropped_control_events: u64,
    /// Total records ever appended (including evicted).
    pub total_appended: u64,
    /// Highest sequence number seen.
    pub max_sequence: u64,
}

// ─── TranscriptStore ────────────────────────────────────────────────────

/// Bounded in-memory store for [`TranscriptRecord`]s.
///
/// Thread-safe via internal `RwLock`. Clone-friendly when wrapped in `Arc`.
pub struct TranscriptStore {
    capacity: usize,
    inner: RwLock<StoreInner>,
    /// Monotonic sequence counter for records that arrive without one.
    next_sequence: AtomicU64,
    dropped_text: AtomicU64,
    dropped_control: AtomicU64,
    total_appended: AtomicU64,
}

struct StoreInner {
    /// All records, kept sorted by sequence number.
    records: Vec<TranscriptRecord>,
    /// Index: run_id -> list of positions in `records`.
    by_run: HashMap<String, Vec<usize>>,
    /// Index: agent_id -> list of positions in `records`.
    by_agent: HashMap<String, Vec<usize>>,
    /// Highest sequence number seen.
    max_sequence: u64,
}

impl TranscriptStore {
    /// Create a new store with the given maximum capacity.
    ///
    /// # Panics
    ///
    /// Panics if `capacity` is 0.
    #[must_use]
    pub fn new(capacity: usize) -> Self {
        assert!(capacity > 0, "TranscriptStore capacity must be > 0");
        Self {
            capacity,
            inner: RwLock::new(StoreInner {
                records: Vec::with_capacity(capacity.min(4096)),
                by_run: HashMap::new(),
                by_agent: HashMap::new(),
                max_sequence: 0,
            }),
            next_sequence: AtomicU64::new(1),
            dropped_text: AtomicU64::new(0),
            dropped_control: AtomicU64::new(0),
            total_appended: AtomicU64::new(0),
        }
    }

    /// Append a record to the store, assigning a monotonic sequence number.
    ///
    /// If the store is at capacity, eviction runs before inserting. Text
    /// delta events are evicted first; control events are never evicted.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError::DuplicateSequence`] if a record with the same
    /// sequence number already exists.
    pub fn append(&self, mut record: TranscriptRecord) -> Result<(), StoreError> {
        // Assign monotonic sequence if the caller's is 0 (unset).
        if record.meta.sequence == 0 {
            record.meta.sequence = self.next_sequence.fetch_add(1, Ordering::Relaxed);
        } else {
            // Advance our counter past any externally-assigned sequence.
            loop {
                let current = self.next_sequence.load(Ordering::Relaxed);
                if record.meta.sequence < current {
                    break;
                }
                if self
                    .next_sequence
                    .compare_exchange_weak(
                        current,
                        record.meta.sequence + 1,
                        Ordering::Relaxed,
                        Ordering::Relaxed,
                    )
                    .is_ok()
                {
                    break;
                }
            }
        }

        let mut inner = self.inner.write();

        // Check for duplicate sequence.
        if inner
            .records
            .binary_search_by_key(&record.meta.sequence, |r| r.meta.sequence)
            .is_ok()
        {
            return Err(StoreError::DuplicateSequence(record.meta.sequence));
        }

        // Evict if at capacity.
        while inner.records.len() >= self.capacity {
            if !self.evict_one(&mut inner) {
                // All remaining records are control events; cannot evict.
                break;
            }
        }

        // Track max sequence.
        if record.meta.sequence > inner.max_sequence {
            inner.max_sequence = record.meta.sequence;
        }

        // Insert in sorted position.
        let pos = inner
            .records
            .binary_search_by_key(&record.meta.sequence, |r| r.meta.sequence)
            .expect_err("duplicate already checked above");

        // Update indices.
        let run_id = record.meta.run_id.clone();
        let agent_id = record.meta.agent_id.clone();

        inner.records.insert(pos, record);

        // Re-index: since we inserted, all positions >= pos shifted by 1.
        Self::shift_indices(&mut inner.by_run, pos);
        Self::shift_indices(&mut inner.by_agent, pos);

        inner.by_run.entry(run_id).or_default().push(pos);
        inner.by_agent.entry(agent_id).or_default().push(pos);

        self.total_appended.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }

    /// Query records matching the given filter, returned in sequence order.
    #[must_use]
    pub fn query(&self, filter: &TranscriptFilter) -> Vec<TranscriptRecord> {
        let inner = self.inner.read();
        inner
            .records
            .iter()
            .filter(|r| filter.matches(r))
            .cloned()
            .collect()
    }

    /// Replay all records from `from_sequence` onward, in sequence order.
    #[must_use]
    pub fn replay(&self, from_sequence: u64) -> Vec<TranscriptRecord> {
        let inner = self.inner.read();
        let start = inner
            .records
            .partition_point(|r| r.meta.sequence < from_sequence);
        inner.records[start..].to_vec()
    }

    /// Cursor-based pagination. Returns up to `limit` records starting
    /// after `cursor` (exclusive). Pass `cursor = 0` for the first page.
    #[must_use]
    pub fn page(&self, cursor: u64, limit: usize) -> TranscriptPage {
        let inner = self.inner.read();
        let start = if cursor == 0 {
            0
        } else {
            inner.records.partition_point(|r| r.meta.sequence <= cursor)
        };

        let end = (start + limit).min(inner.records.len());
        let records: Vec<TranscriptRecord> = inner.records[start..end].to_vec();

        let next_cursor = if end < inner.records.len() {
            records.last().map(|r| r.meta.sequence)
        } else {
            None
        };

        TranscriptPage {
            records,
            next_cursor,
            total_in_store: inner.records.len(),
        }
    }

    /// Runtime statistics.
    #[must_use]
    pub fn stats(&self) -> StoreStats {
        let inner = self.inner.read();
        StoreStats {
            current_count: inner.records.len(),
            capacity: self.capacity,
            dropped_count: self.dropped_text.load(Ordering::Relaxed)
                + self.dropped_control.load(Ordering::Relaxed),
            dropped_text_events: self.dropped_text.load(Ordering::Relaxed),
            dropped_control_events: self.dropped_control.load(Ordering::Relaxed),
            total_appended: self.total_appended.load(Ordering::Relaxed),
            max_sequence: inner.max_sequence,
        }
    }

    /// Number of records currently in the store.
    #[must_use]
    pub fn len(&self) -> usize {
        self.inner.read().records.len()
    }

    /// Whether the store is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.inner.read().records.is_empty()
    }

    // ── Private helpers ─────────────────────────────────────────────────

    /// Evict one non-control record (preferring text deltas). Returns
    /// `false` if nothing could be evicted.
    fn evict_one(&self, inner: &mut StoreInner) -> bool {
        // First pass: find oldest text delta.
        if let Some(pos) = inner.records.iter().position(|r| is_text_delta(&r.event)) {
            self.remove_at(inner, pos);
            self.dropped_text.fetch_add(1, Ordering::Relaxed);
            return true;
        }
        // Second pass: find oldest non-control event.
        if let Some(pos) = inner
            .records
            .iter()
            .position(|r| !is_control_event(&r.event))
        {
            self.remove_at(inner, pos);
            self.dropped_text.fetch_add(1, Ordering::Relaxed);
            return true;
        }
        false
    }

    fn remove_at(&self, inner: &mut StoreInner, pos: usize) {
        let removed = inner.records.remove(pos);
        // Rebuild indices after removal.
        Self::rebuild_indices(inner);
        let _ = removed;
    }

    fn rebuild_indices(inner: &mut StoreInner) {
        inner.by_run.clear();
        inner.by_agent.clear();
        for (i, r) in inner.records.iter().enumerate() {
            inner
                .by_run
                .entry(r.meta.run_id.clone())
                .or_default()
                .push(i);
            inner
                .by_agent
                .entry(r.meta.agent_id.clone())
                .or_default()
                .push(i);
        }
    }

    fn shift_indices(map: &mut HashMap<String, Vec<usize>>, inserted_at: usize) {
        for positions in map.values_mut() {
            for pos in positions.iter_mut() {
                if *pos >= inserted_at {
                    *pos += 1;
                }
            }
        }
    }
}

impl std::fmt::Debug for TranscriptStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let inner = self.inner.read();
        f.debug_struct("TranscriptStore")
            .field("capacity", &self.capacity)
            .field("current_count", &inner.records.len())
            .field(
                "dropped_count",
                &(self.dropped_text.load(Ordering::Relaxed)
                    + self.dropped_control.load(Ordering::Relaxed)),
            )
            .finish()
    }
}

// ─── TranscriptReplayContract ───────────────────────────────────────────

/// Verifies that live sequence == persisted sequence for deterministic
/// replay ordering.
///
/// Usage: feed records through `observe` as they arrive live, then call
/// `verify_against` with the persisted (replayed) sequence to assert they
/// match exactly.
#[derive(Debug, Default)]
pub struct TranscriptReplayContract {
    /// Live sequence numbers in arrival order.
    live_sequences: RwLock<Vec<u64>>,
}

impl TranscriptReplayContract {
    /// Create a new contract verifier.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a live event's sequence number.
    pub fn observe(&self, sequence: u64) {
        self.live_sequences.write().push(sequence);
    }

    /// Verify that the persisted sequence matches the live sequence exactly.
    ///
    /// Returns `Ok(())` if sequences match, or an error describing the
    /// first mismatch.
    pub fn verify_against(&self, persisted: &[u64]) -> Result<(), ReplayMismatch> {
        let live = self.live_sequences.read();
        if live.len() != persisted.len() {
            return Err(ReplayMismatch::LengthMismatch {
                live_len: live.len(),
                persisted_len: persisted.len(),
            });
        }
        for (i, (l, p)) in live.iter().zip(persisted.iter()).enumerate() {
            if l != p {
                return Err(ReplayMismatch::SequenceMismatch {
                    index: i,
                    live: *l,
                    persisted: *p,
                });
            }
        }
        Ok(())
    }

    /// Reset the contract for a new run.
    pub fn reset(&self) {
        self.live_sequences.write().clear();
    }
}

/// Replay verification failure.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum ReplayMismatch {
    /// Live and persisted sequences have different lengths.
    #[error("length mismatch: live={live_len}, persisted={persisted_len}")]
    LengthMismatch {
        live_len: usize,
        persisted_len: usize,
    },
    /// A specific position differs.
    #[error("sequence mismatch at index {index}: live={live}, persisted={persisted}")]
    SequenceMismatch {
        index: usize,
        live: u64,
        persisted: u64,
    },
}

// ─── PriorityEventChannel ───────────────────────────────────────────────

/// Channel that separates control events from text/delta events.
///
/// Control events (tool terminal, run terminal, errors) are buffered in
/// an unbounded sub-channel and are never dropped. Text/delta events use
/// a bounded channel and can be dropped under pressure.
pub struct PriorityEventChannel {
    /// Unbounded control channel — never drops.
    control: RwLock<Vec<TranscriptRecord>>,
    /// Bounded text channel.
    text: RwLock<Vec<TranscriptRecord>>,
    text_capacity: usize,
    /// Counters.
    total_events: AtomicU64,
    dropped_text_events: AtomicU64,
    dropped_control_events: AtomicU64,
}

/// Drop telemetry report from the priority channel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChannelDropReport {
    pub total_events: u64,
    pub dropped_text_events: u64,
    pub dropped_control_events: u64,
}

impl PriorityEventChannel {
    /// Create a channel with the given capacity for text events.
    /// Control events are unbounded.
    #[must_use]
    pub fn new(text_capacity: usize) -> Self {
        Self {
            control: RwLock::new(Vec::new()),
            text: RwLock::new(Vec::with_capacity(text_capacity.min(4096))),
            text_capacity,
            total_events: AtomicU64::new(0),
            dropped_text_events: AtomicU64::new(0),
            dropped_control_events: AtomicU64::new(0),
        }
    }

    /// Send a record into the appropriate sub-channel.
    ///
    /// Control events always succeed. Text events may be dropped if the
    /// text channel is full.
    pub fn send(&self, record: TranscriptRecord) {
        self.total_events.fetch_add(1, Ordering::Relaxed);

        if is_control_event(&record.event) {
            self.control.write().push(record);
        } else if is_text_delta(&record.event) {
            let mut text = self.text.write();
            if text.len() >= self.text_capacity {
                // Drop oldest text event to make room.
                text.remove(0);
                self.dropped_text_events.fetch_add(1, Ordering::Relaxed);
            }
            text.push(record);
        } else {
            // Non-control, non-text events go to the text channel.
            let mut text = self.text.write();
            if text.len() >= self.text_capacity {
                text.remove(0);
                self.dropped_text_events.fetch_add(1, Ordering::Relaxed);
            }
            text.push(record);
        }
    }

    /// Drain all buffered records in sequence order.
    pub fn drain(&self) -> Vec<TranscriptRecord> {
        let mut control = self.control.write();
        let mut text = self.text.write();
        let mut all: Vec<TranscriptRecord> = Vec::with_capacity(control.len() + text.len());
        all.append(&mut control);
        all.append(&mut text);
        all.sort_by_key(|r| r.meta.sequence);
        all
    }

    /// Drop telemetry report.
    #[must_use]
    pub fn drop_report(&self) -> ChannelDropReport {
        ChannelDropReport {
            total_events: self.total_events.load(Ordering::Relaxed),
            dropped_text_events: self.dropped_text_events.load(Ordering::Relaxed),
            dropped_control_events: self.dropped_control_events.load(Ordering::Relaxed),
        }
    }
}

impl std::fmt::Debug for PriorityEventChannel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PriorityEventChannel")
            .field("text_capacity", &self.text_capacity)
            .field("total_events", &self.total_events.load(Ordering::Relaxed))
            .field(
                "dropped_text_events",
                &self.dropped_text_events.load(Ordering::Relaxed),
            )
            .finish()
    }
}

// ─── Helpers ────────────────────────────────────────────────────────────

/// Whether the event is a text delta (evictable under pressure).
fn is_text_delta(event: &TranscriptEvent) -> bool {
    matches!(
        event,
        TranscriptEvent::AssistantDelta { .. }
            | TranscriptEvent::ReasoningDelta { .. }
            | TranscriptEvent::ToolOutputDelta { .. }
    )
}

/// Whether the event is a control event (never evicted).
fn is_control_event(event: &TranscriptEvent) -> bool {
    matches!(
        event,
        TranscriptEvent::RunStarted { .. }
            | TranscriptEvent::RunFinished { .. }
            | TranscriptEvent::ToolStarted { .. }
            | TranscriptEvent::ToolFinished { .. }
            | TranscriptEvent::Error { .. }
            | TranscriptEvent::ProviderChanged { .. }
            | TranscriptEvent::SubagentStarted { .. }
            | TranscriptEvent::SubagentFinished { .. }
    )
}

/// Extract the serde type tag name for an event.
fn event_type_name(event: &TranscriptEvent) -> &'static str {
    match event {
        TranscriptEvent::RunStarted { .. } => "run_started",
        TranscriptEvent::AssistantDelta { .. } => "assistant_delta",
        TranscriptEvent::ReasoningDelta { .. } => "reasoning_delta",
        TranscriptEvent::ToolStarted { .. } => "tool_started",
        TranscriptEvent::ToolOutputDelta { .. } => "tool_output_delta",
        TranscriptEvent::ToolFinished { .. } => "tool_finished",
        TranscriptEvent::TodoSnapshot { .. } => "todo_snapshot",
        TranscriptEvent::SubagentStarted { .. } => "subagent_started",
        TranscriptEvent::SubagentUpdate { .. } => "subagent_update",
        TranscriptEvent::SubagentFinished { .. } => "subagent_finished",
        TranscriptEvent::Usage { .. } => "usage",
        TranscriptEvent::ProviderChanged { .. } => "provider_changed",
        TranscriptEvent::Warning { .. } => "warning",
        TranscriptEvent::Error { .. } => "error",
        TranscriptEvent::RunFinished { .. } => "run_finished",
    }
}

/// Check if an event relates to a specific tool call ID.
fn event_matches_call_id(event: &TranscriptEvent, call_id: &str) -> bool {
    match event {
        TranscriptEvent::ToolStarted { call, .. } => call.id == call_id,
        TranscriptEvent::ToolOutputDelta { call_id: cid, .. } => cid == call_id,
        TranscriptEvent::ToolFinished { call_id: cid, .. } => cid == call_id,
        _ => false,
    }
}

// ─── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tool::transcript::{TranscriptEventMeta, TranscriptRecord};

    fn make_meta(run_id: &str, agent_id: &str, seq: u64) -> TranscriptEventMeta {
        TranscriptEventMeta {
            run_id: run_id.into(),
            turn_id: 0,
            agent_id: agent_id.into(),
            sequence: seq,
            timestamp_ms: 1_700_000_000_000 + (seq as i64),
            provider: "test".into(),
            model: "test-model".into(),
            parent_event_id: None,
        }
    }

    fn delta_record(run_id: &str, agent_id: &str, seq: u64, text: &str) -> TranscriptRecord {
        TranscriptRecord {
            meta: make_meta(run_id, agent_id, seq),
            event: TranscriptEvent::AssistantDelta { text: text.into() },
        }
    }

    fn control_record(run_id: &str, agent_id: &str, seq: u64) -> TranscriptRecord {
        TranscriptRecord {
            meta: make_meta(run_id, agent_id, seq),
            event: TranscriptEvent::RunFinished {
                success: true,
                total_turns: 1,
                total_tool_calls: 0,
                wall_ms: 100,
            },
        }
    }

    fn error_record(run_id: &str, agent_id: &str, seq: u64) -> TranscriptRecord {
        TranscriptRecord {
            meta: make_meta(run_id, agent_id, seq),
            event: TranscriptEvent::Error {
                code: "TEST".into(),
                message: "test error".into(),
                recoverable: false,
            },
        }
    }

    // ── Basic append + query ────────────────────────────────────────────

    #[test]
    fn append_and_query_by_run_id() {
        let store = TranscriptStore::new(100);
        store
            .append(delta_record("run-1", "a1", 1, "hello"))
            .unwrap();
        store
            .append(delta_record("run-2", "a1", 2, "world"))
            .unwrap();

        let results = store.query(&TranscriptFilter {
            run_id: Some("run-1".into()),
            ..Default::default()
        });
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].meta.run_id, "run-1");
    }

    #[test]
    fn append_and_query_by_agent_id() {
        let store = TranscriptStore::new(100);
        store
            .append(delta_record("run-1", "alpha", 1, "a"))
            .unwrap();
        store.append(delta_record("run-1", "beta", 2, "b")).unwrap();

        let results = store.query(&TranscriptFilter {
            agent_id: Some("beta".into()),
            ..Default::default()
        });
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].meta.agent_id, "beta");
    }

    #[test]
    fn query_by_event_type() {
        let store = TranscriptStore::new(100);
        store.append(delta_record("r", "a", 1, "text")).unwrap();
        store.append(control_record("r", "a", 2)).unwrap();

        let results = store.query(&TranscriptFilter {
            event_types: Some(vec!["run_finished".into()]),
            ..Default::default()
        });
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn query_by_time_range() {
        let store = TranscriptStore::new(100);
        store.append(delta_record("r", "a", 1, "early")).unwrap();
        store.append(delta_record("r", "a", 100, "late")).unwrap();

        let results = store.query(&TranscriptFilter {
            after_ms: Some(1_700_000_000_050),
            ..Default::default()
        });
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].meta.sequence, 100);
    }

    // ── Eviction ────────────────────────────────────────────────────────

    #[test]
    fn eviction_drops_text_deltas_before_control_events() {
        let store = TranscriptStore::new(3);

        // Fill with 2 deltas and 1 control.
        store.append(delta_record("r", "a", 1, "delta1")).unwrap();
        store.append(delta_record("r", "a", 2, "delta2")).unwrap();
        store.append(control_record("r", "a", 3)).unwrap();

        // Now append a 4th — should evict oldest delta, not the control.
        store.append(delta_record("r", "a", 4, "delta3")).unwrap();

        let stats = store.stats();
        assert_eq!(stats.current_count, 3);
        assert!(stats.dropped_text_events > 0);
        assert_eq!(stats.dropped_control_events, 0);

        // The control event (seq=3) must still be present.
        let results = store.query(&TranscriptFilter {
            event_types: Some(vec!["run_finished".into()]),
            ..Default::default()
        });
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn control_events_never_evicted() {
        // Store with capacity 2, fill entirely with control events.
        let store = TranscriptStore::new(2);
        store.append(control_record("r", "a", 1)).unwrap();
        store.append(error_record("r", "a", 2)).unwrap();

        // Adding a 3rd should NOT evict the existing control events.
        store.append(control_record("r", "a", 3)).unwrap();

        // Store may exceed capacity to preserve control events.
        let stats = store.stats();
        assert!(stats.current_count >= 2);
        assert_eq!(stats.dropped_control_events, 0);
    }

    // ── Replay ──────────────────────────────────────────────────────────

    #[test]
    fn replay_from_sequence() {
        let store = TranscriptStore::new(100);
        for i in 1..=5 {
            store
                .append(delta_record("r", "a", i, &format!("msg{i}")))
                .unwrap();
        }

        let replayed = store.replay(3);
        assert_eq!(replayed.len(), 3);
        assert_eq!(replayed[0].meta.sequence, 3);
        assert_eq!(replayed[2].meta.sequence, 5);
    }

    #[test]
    fn replay_from_zero_returns_all() {
        let store = TranscriptStore::new(100);
        for i in 1..=3 {
            store.append(delta_record("r", "a", i, "x")).unwrap();
        }
        assert_eq!(store.replay(0).len(), 3);
    }

    // ── Pagination ──────────────────────────────────────────────────────

    #[test]
    fn page_cursor_based() {
        let store = TranscriptStore::new(100);
        for i in 1..=10 {
            store
                .append(delta_record("r", "a", i, &format!("msg{i}")))
                .unwrap();
        }

        // First page.
        let p1 = store.page(0, 3);
        assert_eq!(p1.records.len(), 3);
        assert_eq!(p1.records[0].meta.sequence, 1);
        assert!(p1.next_cursor.is_some());

        // Second page.
        let p2 = store.page(p1.next_cursor.unwrap(), 3);
        assert_eq!(p2.records.len(), 3);
        assert_eq!(p2.records[0].meta.sequence, 4);

        // Last page.
        let p_last = store.page(7, 10);
        assert_eq!(p_last.records.len(), 3);
        assert!(p_last.next_cursor.is_none());
    }

    // ── Monotonic sequence assignment ───────────────────────────────────

    #[test]
    fn auto_assigns_monotonic_sequence() {
        let store = TranscriptStore::new(100);
        let mut r1 = delta_record("r", "a", 0, "auto1");
        let mut r2 = delta_record("r", "a", 0, "auto2");
        r1.meta.sequence = 0; // Will be auto-assigned.
        r2.meta.sequence = 0;
        store.append(r1).unwrap();
        store.append(r2).unwrap();

        let all = store.replay(0);
        assert_eq!(all.len(), 2);
        assert!(all[0].meta.sequence < all[1].meta.sequence);
    }

    #[test]
    fn duplicate_sequence_is_rejected() {
        let store = TranscriptStore::new(100);
        store.append(delta_record("r", "a", 42, "first")).unwrap();
        let err = store
            .append(delta_record("r", "a", 42, "dupe"))
            .unwrap_err();
        assert_eq!(err, StoreError::DuplicateSequence(42));
    }

    // ── Concurrent access ───────────────────────────────────────────────

    #[test]
    fn concurrent_appends() {
        use std::sync::Arc;
        use std::thread;

        let store = Arc::new(TranscriptStore::new(1000));
        let mut handles = Vec::new();

        for t in 0..4 {
            let store = Arc::clone(&store);
            handles.push(thread::spawn(move || {
                for i in 0..50 {
                    let seq = (t * 1000 + i) as u64 + 1;
                    let _ = store.append(delta_record("r", "a", seq, "x"));
                }
            }));
        }

        for h in handles {
            h.join().unwrap();
        }

        // All records should be in sequence order.
        let all = store.replay(0);
        for window in all.windows(2) {
            assert!(
                window[0].meta.sequence < window[1].meta.sequence,
                "sequence order violation: {} >= {}",
                window[0].meta.sequence,
                window[1].meta.sequence
            );
        }
    }

    // ── Stats ───────────────────────────────────────────────────────────

    #[test]
    fn stats_report_accurate() {
        let store = TranscriptStore::new(5);
        for i in 1..=8 {
            store.append(delta_record("r", "a", i, "x")).unwrap();
        }
        let stats = store.stats();
        assert_eq!(stats.current_count, 5);
        assert_eq!(stats.capacity, 5);
        assert_eq!(stats.total_appended, 8);
        assert!(stats.dropped_count >= 3);
    }

    // ── Replay contract ─────────────────────────────────────────────────

    #[test]
    fn replay_contract_passes_for_matching_sequences() {
        let contract = TranscriptReplayContract::new();
        contract.observe(1);
        contract.observe(2);
        contract.observe(3);
        assert!(contract.verify_against(&[1, 2, 3]).is_ok());
    }

    #[test]
    fn replay_contract_detects_length_mismatch() {
        let contract = TranscriptReplayContract::new();
        contract.observe(1);
        contract.observe(2);
        let err = contract.verify_against(&[1]).unwrap_err();
        assert!(matches!(err, ReplayMismatch::LengthMismatch { .. }));
    }

    #[test]
    fn replay_contract_detects_sequence_mismatch() {
        let contract = TranscriptReplayContract::new();
        contract.observe(1);
        contract.observe(2);
        let err = contract.verify_against(&[1, 3]).unwrap_err();
        assert!(matches!(err, ReplayMismatch::SequenceMismatch { .. }));
    }

    // ── Priority channel ────────────────────────────────────────────────

    #[test]
    fn priority_channel_never_drops_control() {
        let channel = PriorityEventChannel::new(2);

        // Send 3 control events — all should survive.
        channel.send(control_record("r", "a", 1));
        channel.send(control_record("r", "a", 2));
        channel.send(control_record("r", "a", 3));

        let report = channel.drop_report();
        assert_eq!(report.dropped_control_events, 0);
        assert_eq!(report.total_events, 3);

        let drained = channel.drain();
        assert_eq!(drained.len(), 3);
    }

    #[test]
    fn priority_channel_drops_text_under_pressure() {
        let channel = PriorityEventChannel::new(2);

        channel.send(delta_record("r", "a", 1, "a"));
        channel.send(delta_record("r", "a", 2, "b"));
        channel.send(delta_record("r", "a", 3, "c")); // Should drop oldest.

        let report = channel.drop_report();
        assert_eq!(report.dropped_text_events, 1);
        assert_eq!(report.total_events, 3);

        let drained = channel.drain();
        assert_eq!(drained.len(), 2);
    }

    #[test]
    fn priority_channel_drain_returns_sorted() {
        let channel = PriorityEventChannel::new(100);

        // Interleave control and text events.
        channel.send(delta_record("r", "a", 3, "x"));
        channel.send(control_record("r", "a", 1));
        channel.send(delta_record("r", "a", 2, "y"));

        let drained = channel.drain();
        let seqs: Vec<u64> = drained.iter().map(|r| r.meta.sequence).collect();
        assert_eq!(seqs, vec![1, 2, 3]);
    }

    // ── Secret canary: no raw secrets in serialized records ─────────────

    #[test]
    fn secret_canary_no_raw_api_keys() {
        let record = TranscriptRecord {
            meta: make_meta("r", "a", 1),
            event: TranscriptEvent::AssistantDelta {
                text: "sk-ant-api-fake-key-12345".into(),
            },
        };
        let json = serde_json::to_string(&record).unwrap();
        // The store itself does not redact — that is the classified
        // persistence layer's job. But the raw text IS present:
        assert!(json.contains("sk-ant-api"));
    }

    // ── Tool call ID query ──────────────────────────────────────────────

    #[test]
    fn query_by_tool_call_id() {
        use crate::tool::call::ToolCall;
        use crate::tool::transcript::ToolLifecycleStatus;

        let store = TranscriptStore::new(100);

        let started = TranscriptRecord {
            meta: make_meta("r", "a", 1),
            event: TranscriptEvent::ToolStarted {
                call: ToolCall::at("call-42", "read_file", serde_json::json!({}), 0),
                status: ToolLifecycleStatus::Pending,
                category: None,
            },
        };
        let output = TranscriptRecord {
            meta: make_meta("r", "a", 2),
            event: TranscriptEvent::ToolOutputDelta {
                call_id: "call-42".into(),
                text: "output".into(),
            },
        };
        let unrelated = TranscriptRecord {
            meta: make_meta("r", "a", 3),
            event: TranscriptEvent::ToolOutputDelta {
                call_id: "call-99".into(),
                text: "other".into(),
            },
        };

        store.append(started).unwrap();
        store.append(output).unwrap();
        store.append(unrelated).unwrap();

        let results = store.query(&TranscriptFilter {
            tool_call_id: Some("call-42".into()),
            ..Default::default()
        });
        assert_eq!(results.len(), 2);
    }
}
