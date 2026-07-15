//! Unified state hub: single source of truth for dashboard consumers.
//!
//! The [`StateHub`] bridges the event bus to a materialized
//! [`DashboardSnapshot`] via a `tokio::sync::watch` channel. Three consumer
//! interfaces:
//!
//! - **TUI** -- borrows the watch receiver at 60 fps.
//! - **WebSocket / SSE** -- subscribes to the broadcast channel for live events.
//! - **REST API** -- clones the current snapshot on demand.
//!
//! ```text
//! Orchestrator
//!     | publish(DashboardEvent)
//!     v
//! StateHub
//!     |-- watch<DashboardSnapshot>  <- TUI reads (60fps, zero-copy borrow)
//!     |-- broadcast<DashboardEvent> <- WebSocket/SSE clients subscribe
//!     +-- ring buffer (1024)        <- replay for late joiners
//! ```
//!
//! # Crate boundary
//!
//! `StateHub` lives in `roko-runtime` because it depends on
//! [`crate::event_bus::EventBus`] (a runtime primitive) while consuming
//! domain types (`DashboardEvent`, `DashboardSnapshot`) from `roko-core`.
//! This avoids the previous `#[path]`-include hack that compiled this file
//! inside `roko-serve` with a fake `extern crate self as roko_core` alias.

use std::fmt;
use std::io::Write;
use std::ops::Deref;
use std::path::Path;
use std::sync::{Arc, Mutex};

use crate::event_bus::{self, EventBus};
use tokio::sync::watch;

use roko_core::dashboard_snapshot::{DashboardEvent, DashboardSnapshot};

/// Append-only JSONL writer for persisting events to disk.
struct EventLogWriter {
    writer: std::io::BufWriter<std::fs::File>,
}

impl EventLogWriter {
    fn open(path: &Path) -> std::io::Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)?;
        Ok(Self {
            writer: std::io::BufWriter::new(file),
        })
    }

    fn append(&mut self, event: &DashboardEvent) {
        // Best-effort: log but don't propagate serialization or I/O errors.
        if let Ok(json) = serde_json::to_string(event) {
            let _ = writeln!(self.writer, "{json}");
            let _ = self.writer.flush();
        }
    }
}

/// Unified state hub driving all dashboard consumers from a single event
/// stream.
///
/// Events are published once via [`publish`]. Each call:
/// 1. Applies the event to the materialized snapshot so the TUI can borrow it.
/// 2. Optionally appends to the on-disk event log (`.roko/events.jsonl`).
/// 3. Records and broadcasts the sequenced event to live and replay consumers.
///
/// Snapshot commit, append, sequence assignment, and broadcast are serialized,
/// so a consumer receiving event N can observe snapshot state including N.
/// The JSONL append remains best-effort and is not a durable event-bus contract.
/// Shared event log handle that can be cloned into `StateHubSender`s.
type SharedEventLog = Arc<Mutex<EventLogWriter>>;

/// An atomic cursor/snapshot read of a [`StateHub`].
///
/// `next_seq` is the first sequence not represented by `snapshot` at the time
/// this value was captured. Snapshot-only mutations may also be present.
pub struct StateHubCursorSnapshot {
    /// Sequence that the next published event will receive.
    pub next_seq: u64,
    /// Materialized state after every event below `next_seq` was applied.
    pub snapshot: DashboardSnapshot,
}

/// Race-free replay/live handoff for a StateHub event consumer.
///
/// The live receiver is installed before the replay ring and snapshot cursor
/// are captured while publication is paused. Consequently, `replay` contains
/// only events below `next_seq`, while `live` starts at `next_seq`.
pub struct StateHubSubscription {
    /// Retained events at or after the requested sequence.
    pub replay: Vec<event_bus::Envelope<DashboardEvent>>,
    /// Live receiver for events published after the atomic capture.
    pub live: tokio::sync::broadcast::Receiver<event_bus::Envelope<DashboardEvent>>,
    /// Cursor and snapshot captured with the replay/live boundary.
    pub cursor: StateHubCursorSnapshot,
}

/// Central state management hub for dashboard snapshots, events, and projections.
pub struct StateHub {
    snapshot_tx: watch::Sender<DashboardSnapshot>,
    snapshot_rx: watch::Receiver<DashboardSnapshot>,
    event_bus: EventBus<DashboardEvent>,
    /// Serializes the snapshot commit, best-effort append, and sequence assignment
    /// so subscribers can never observe event N against state older than N.
    publish_lock: Arc<Mutex<()>>,
    /// Optional on-disk event log for persistence across restarts.
    event_log: Option<SharedEventLog>,
}

impl StateHub {
    /// Create a new hub with the given replay ring capacity.
    pub fn new(ring_capacity: usize) -> Self {
        let (snapshot_tx, snapshot_rx) = watch::channel(DashboardSnapshot::default());
        Self {
            snapshot_tx,
            snapshot_rx,
            event_bus: EventBus::new(ring_capacity),
            publish_lock: Arc::new(Mutex::new(())),
            event_log: None,
        }
    }

    /// Create a new hub with the default ring capacity (1024).
    pub fn default_capacity() -> Self {
        Self::new(1024)
    }

    /// Create a new hub that persists events to the given JSONL log file.
    ///
    /// Every call to [`publish`] also appends the event to disk so that
    /// future consumers (e.g. `roko dashboard` in standalone mode) can
    /// replay the log to reconstruct the snapshot.
    pub fn with_event_log(ring_capacity: usize, log_path: &Path) -> Self {
        let event_log = EventLogWriter::open(log_path)
            .map(|w| Arc::new(Mutex::new(w)))
            .ok();
        if event_log.is_none() {
            tracing::warn!(
                path = %log_path.display(),
                "failed to open event log; events will not be persisted"
            );
        }
        let (snapshot_tx, snapshot_rx) = watch::channel(DashboardSnapshot::default());
        Self {
            snapshot_tx,
            snapshot_rx,
            event_bus: EventBus::new(ring_capacity),
            publish_lock: Arc::new(Mutex::new(())),
            event_log,
        }
    }

    /// Enable event log persistence on an existing hub.
    pub fn enable_event_log(&mut self, log_path: &Path) {
        match EventLogWriter::open(log_path) {
            Ok(w) => self.event_log = Some(Arc::new(Mutex::new(w))),
            Err(e) => tracing::warn!(
                path = %log_path.display(),
                error = %e,
                "failed to open event log"
            ),
        }
    }

    /// Publish an event: apply it to the snapshot, append it to the optional
    /// best-effort log, then record and broadcast it on the replay bus.
    /// Returns the sequence number assigned on the internal event bus.
    pub fn publish(&self, event: DashboardEvent) -> u64 {
        let _publish = self
            .publish_lock
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        self.snapshot_tx.send_modify(|snap| snap.apply(&event));
        if let Some(log) = &self.event_log {
            if let Ok(mut writer) = log.lock() {
                writer.append(&event);
            }
        }
        self.event_bus.emit(event)
    }

    /// Publish a batch of events atomically (snapshot updates are visible
    /// together after the last event).
    pub fn publish_batch(&self, events: impl IntoIterator<Item = DashboardEvent>) {
        let events = events.into_iter().collect::<Vec<_>>();
        let _publish = self
            .publish_lock
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        self.snapshot_tx.send_modify(|snap| {
            for event in &events {
                snap.apply(event);
            }
        });
        for event in events {
            if let Some(log) = &self.event_log {
                if let Ok(mut writer) = log.lock() {
                    writer.append(&event);
                }
            }
            self.event_bus.emit(event);
        }
    }

    /// Replace the current materialized snapshot atomically.
    pub fn apply_snapshot(&self, snapshot: DashboardSnapshot) {
        let _publish = self
            .publish_lock
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let _ = self.snapshot_tx.send(snapshot);
    }

    /// Mutate selected snapshot fields atomically without a lossy
    /// read-modify-write round trip through [`current_snapshot`](Self::current_snapshot).
    pub fn update_snapshot(&self, update: impl FnOnce(&mut DashboardSnapshot)) {
        let _publish = self
            .publish_lock
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        self.snapshot_tx.send_modify(update);
    }

    /// Get a receiver for the materialized snapshot.
    ///
    /// The TUI calls `borrow()` or `borrow_and_update()` on this at render
    /// time for a zero-copy read of the current state.
    pub fn snapshot(&self) -> watch::Receiver<DashboardSnapshot> {
        self.snapshot_rx.clone()
    }

    /// Clone the current snapshot (for REST API responses).
    pub fn current_snapshot(&self) -> DashboardSnapshot {
        self.snapshot_rx.borrow().clone()
    }

    /// Subscribe to live events (for WebSocket / SSE streaming).
    pub fn subscribe_events(
        &self,
    ) -> tokio::sync::broadcast::Receiver<event_bus::Envelope<DashboardEvent>> {
        self.event_bus.subscribe()
    }

    /// Atomically install a live subscriber and capture retained replay state.
    ///
    /// This closes the reconnect race caused by calling [`replay_from`](Self::replay_from)
    /// and [`subscribe_events`](Self::subscribe_events) separately: no publish
    /// can occur between the two operations, and the returned live stream begins
    /// exactly at `cursor.next_seq`.
    pub fn subscribe_events_from(&self, next_seq: u64) -> StateHubSubscription {
        let _publish = self
            .publish_lock
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let live = self.event_bus.subscribe();
        let replay = self.event_bus.replay_from(next_seq);
        let cursor = StateHubCursorSnapshot {
            next_seq: self.event_bus.total_emitted(),
            snapshot: self.snapshot_rx.borrow().clone(),
        };
        StateHubSubscription {
            replay,
            live,
            cursor,
        }
    }

    /// Capture the materialized snapshot and its next event sequence atomically.
    pub fn cursor_snapshot(&self) -> StateHubCursorSnapshot {
        let _publish = self
            .publish_lock
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        StateHubCursorSnapshot {
            next_seq: self.event_bus.total_emitted(),
            snapshot: self.snapshot_rx.borrow().clone(),
        }
    }

    /// Replay events from the on-disk event log into the snapshot.
    ///
    /// Reads `.roko/events.jsonl`, deserializes each line as a
    /// [`DashboardEvent`], and applies it to the materialized snapshot.
    /// Returns the number of events replayed.
    pub fn replay_from_log(log_path: &Path) -> (Self, usize) {
        let mut hub = Self::default_capacity();
        let count = hub.ingest_log(log_path);
        (hub, count)
    }

    /// Ingest events from a log file into this hub's snapshot (without
    /// re-persisting them).
    pub fn ingest_log(&mut self, log_path: &Path) -> usize {
        let content = match std::fs::read_to_string(log_path) {
            Ok(c) => c,
            Err(_) => return 0,
        };
        let mut count = 0usize;
        self.snapshot_tx.send_modify(|snap| {
            for line in content.lines() {
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }
                if let Ok(event) = serde_json::from_str::<DashboardEvent>(line) {
                    snap.apply(&event);
                    count += 1;
                }
            }
        });
        count
    }

    /// Replay events from a log file into the snapshot (immutable `self`).
    ///
    /// Unlike [`ingest_log`] which requires `&mut self`, this method uses
    /// `snapshot_tx.send_modify()` which only requires `&self`. This allows
    /// callers who hold a `SharedStateHub` (which wraps `Arc<StateHub>`) to
    /// replay events via `Deref` without needing mutable access.
    pub fn replay_log_into_snapshot(&self, log_path: &Path) -> usize {
        let content = match std::fs::read_to_string(log_path) {
            Ok(c) => c,
            Err(_) => return 0,
        };
        let _publish = self
            .publish_lock
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let mut count = 0usize;
        self.snapshot_tx.send_modify(|snap| {
            for line in content.lines() {
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }
                if let Ok(event) = serde_json::from_str::<DashboardEvent>(line) {
                    snap.apply(&event);
                    count += 1;
                }
            }
        });
        count
    }

    /// Replay events from the ring buffer starting at `after_seq`.
    pub fn replay_from(&self, after_seq: u64) -> Vec<event_bus::Envelope<DashboardEvent>> {
        self.event_bus.replay_from(after_seq)
    }

    /// Get a clone-safe sender handle for subsystems that only produce events.
    pub fn sender(&self) -> StateHubSender {
        StateHubSender {
            snapshot_tx: self.snapshot_tx.clone(),
            bus_sender: self.event_bus.sender(),
            publish_lock: Arc::clone(&self.publish_lock),
            event_log: self.event_log.clone(),
        }
    }

    /// Total events ever published.
    pub fn total_published(&self) -> u64 {
        self.event_bus.total_emitted()
    }

    /// Current ring buffer length.
    pub fn ring_len(&self) -> usize {
        self.event_bus.ring_len()
    }
}

/// A clone-safe, send-safe handle for publishing events into a [`StateHub`].
///
/// Created via [`StateHub::sender`]. Safe to send across threads/tasks.
/// This is what gets passed to the orchestrator.
#[derive(Clone)]
pub struct StateHubSender {
    snapshot_tx: watch::Sender<DashboardSnapshot>,
    bus_sender: event_bus::BusSender<DashboardEvent>,
    publish_lock: Arc<Mutex<()>>,
    event_log: Option<SharedEventLog>,
}

impl StateHubSender {
    /// Publish an event through the hub.
    /// Returns the sequence number assigned on the internal event bus.
    pub fn publish(&self, event: DashboardEvent) -> u64 {
        let _publish = self
            .publish_lock
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        self.snapshot_tx.send_modify(|snap| snap.apply(&event));
        if let Some(log) = &self.event_log {
            if let Ok(mut writer) = log.lock() {
                writer.append(&event);
            }
        }
        self.bus_sender.emit(event)
    }
}

/// Shared reference-counted handle to a [`StateHub`].
#[derive(Clone)]
pub struct SharedStateHub(Arc<StateHub>);

impl SharedStateHub {
    /// Wrap an existing hub in a shared handle.
    pub fn new(state_hub: StateHub) -> Self {
        Self(Arc::new(state_hub))
    }

    /// Create a new in-process hub for standalone clients.
    pub fn new_in_process() -> Self {
        Self::new(StateHub::default_capacity())
    }

    /// Seed the materialized snapshot from a workspace root.
    ///
    /// Missing or unreadable sources are handled by loading an empty
    /// snapshot and warning, so standalone consumers keep running.
    pub fn bootstrap_from_workdir(&self, workdir: &Path) -> Result<(), std::io::Error> {
        match DashboardSnapshot::load_from_workdir(workdir) {
            Ok(snapshot) => {
                self.0.apply_snapshot(snapshot);
                Ok(())
            }
            Err(err) => {
                tracing::warn!(
                    error = %err,
                    workdir = %workdir.display(),
                    "failed to bootstrap dashboard snapshot from workdir; using empty snapshot"
                );
                self.0.apply_snapshot(DashboardSnapshot::default());
                Err(err)
            }
        }
    }
}

impl Deref for SharedStateHub {
    type Target = StateHub;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl AsRef<StateHub> for SharedStateHub {
    fn as_ref(&self) -> &StateHub {
        &self.0
    }
}

impl fmt::Debug for SharedStateHub {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SharedStateHub").finish_non_exhaustive()
    }
}

impl From<StateHub> for SharedStateHub {
    fn from(state_hub: StateHub) -> Self {
        Self::new(state_hub)
    }
}

impl From<Arc<StateHub>> for SharedStateHub {
    fn from(state_hub: Arc<StateHub>) -> Self {
        Self(state_hub)
    }
}

/// Create a new shared state hub with the default capacity.
pub fn shared_state_hub() -> SharedStateHub {
    SharedStateHub::new_in_process()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn publish_updates_snapshot_and_broadcasts() {
        let hub = StateHub::default_capacity();
        let mut rx = hub.snapshot();
        let mut event_rx = hub.subscribe_events();

        hub.publish(DashboardEvent::PlanStarted {
            plan_id: "p1".into(),
        });

        // Snapshot is updated synchronously.
        let snap = rx.borrow_and_update();
        assert_eq!(snap.stats.plans_active, 1);
        assert!(snap.plans.contains_key("p1"));
        drop(snap);

        // Broadcast also received the event.
        let envelope = event_rx.recv().await.unwrap();
        assert!(matches!(
            envelope.payload,
            DashboardEvent::PlanStarted { .. }
        ));
    }

    #[tokio::test]
    async fn broadcast_observes_committed_snapshot() {
        let hub = Arc::new(StateHub::default_capacity());
        let mut events = hub.subscribe_events();
        let observer = Arc::clone(&hub);
        let observed = tokio::spawn(async move {
            let envelope = events.recv().await.unwrap();
            let snapshot = observer.current_snapshot();
            (envelope.seq, snapshot.plans.contains_key("committed"))
        });

        assert_eq!(
            hub.publish(DashboardEvent::PlanStarted {
                plan_id: "committed".into(),
            }),
            0
        );

        let (seq, was_committed) = observed.await.unwrap();
        assert_eq!(seq, 0);
        assert!(was_committed, "event was visible before its state commit");
    }

    #[tokio::test]
    async fn replay_live_handoff_has_no_missing_boundary_event() {
        let hub = StateHub::new(8);
        for plan_id in ["before-0", "before-1"] {
            hub.publish(DashboardEvent::PlanStarted {
                plan_id: plan_id.into(),
            });
        }

        let subscription = hub.subscribe_events_from(1);
        assert_eq!(subscription.cursor.next_seq, 2);
        assert_eq!(
            subscription
                .replay
                .iter()
                .map(|event| event.seq)
                .collect::<Vec<_>>(),
            vec![1]
        );
        assert!(subscription.cursor.snapshot.plans.contains_key("before-1"));

        let mut live = subscription.live;
        hub.publish(DashboardEvent::PlanStarted {
            plan_id: "live-2".into(),
        });
        let live_event = live.recv().await.unwrap();
        assert_eq!(live_event.seq, 2);
        assert!(hub.current_snapshot().plans.contains_key("live-2"));
    }

    #[test]
    fn atomic_snapshot_update_preserves_published_state() {
        let hub = StateHub::default_capacity();
        hub.publish(DashboardEvent::PlanStarted {
            plan_id: "p1".into(),
        });

        hub.update_snapshot(|snapshot| {
            snapshot.cascade_router_json = "router-state".into();
        });

        let snapshot = hub.current_snapshot();
        assert!(snapshot.plans.contains_key("p1"));
        assert_eq!(snapshot.cascade_router_json, "router-state");
    }

    #[test]
    fn concurrent_publish_and_snapshot_mutation_preserve_every_update() {
        const OPERATIONS: usize = 32;
        let hub = Arc::new(StateHub::default_capacity());
        let barrier = Arc::new(std::sync::Barrier::new(OPERATIONS * 2));
        let mut workers = Vec::with_capacity(OPERATIONS * 2);

        for index in 0..OPERATIONS {
            let publish_hub = Arc::clone(&hub);
            let publish_barrier = Arc::clone(&barrier);
            workers.push(std::thread::spawn(move || {
                publish_barrier.wait();
                publish_hub.publish(DashboardEvent::PlanStarted {
                    plan_id: format!("plan-{index}"),
                });
            }));

            let update_hub = Arc::clone(&hub);
            let update_barrier = Arc::clone(&barrier);
            workers.push(std::thread::spawn(move || {
                update_barrier.wait();
                update_hub.update_snapshot(|snapshot| {
                    snapshot.cascade_router_json.push('x');
                });
            }));
        }

        for worker in workers {
            worker.join().unwrap();
        }

        let cursor = hub.cursor_snapshot();
        assert_eq!(cursor.next_seq, OPERATIONS as u64);
        assert_eq!(cursor.snapshot.plans.len(), OPERATIONS);
        assert_eq!(cursor.snapshot.cascade_router_json.len(), OPERATIONS);
    }

    #[test]
    fn sender_handle_publishes() {
        let hub = StateHub::default_capacity();
        let sender = hub.sender();

        sender.publish(DashboardEvent::AgentSpawned {
            agent_id: "a1".into(),
            role: "coder".into(),
            model: String::new(),
        });

        let snap = hub.current_snapshot();
        assert_eq!(snap.stats.agents_active, 1);
        assert!(snap.agents.contains_key("a1"));
    }

    #[test]
    fn replay_ring_works() {
        let hub = StateHub::new(16);
        hub.publish(DashboardEvent::PlanStarted {
            plan_id: "p1".into(),
        });
        hub.publish(DashboardEvent::PlanStarted {
            plan_id: "p2".into(),
        });

        let events = hub.replay_from(0);
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].seq, 0);
        assert_eq!(events[1].seq, 1);

        let partial = hub.replay_from(1);
        assert_eq!(partial.len(), 1);
    }

    #[test]
    fn batch_publish() {
        let hub = StateHub::default_capacity();
        hub.publish_batch(vec![
            DashboardEvent::PlanStarted {
                plan_id: "p1".into(),
            },
            DashboardEvent::TaskStarted {
                plan_id: "p1".into(),
                task_id: "t1".into(),
                title: String::new(),
                phase: "compose".into(),
            },
        ]);

        let snap = hub.current_snapshot();
        assert_eq!(snap.stats.plans_active, 1);
        assert_eq!(snap.stats.tasks_active, 1);
    }

    #[test]
    fn in_process_bootstrap_populates_live_receiver() {
        let tmpdir = tempfile::tempdir().expect("tempdir");
        let state_dir = tmpdir.path().join(".roko/state");
        std::fs::create_dir_all(&state_dir).expect("state dir");

        let executor_state = serde_json::json!({
            "plan_states": {
                "plan-a": {
                    "current_phase": { "kind": "implementing" },
                    "task_id": "task-a",
                    "assigned_agents": ["agent-a"],
                    "gate_results": [
                        {
                            "gate_name": "compile",
                            "passed": true,
                            "duration_ms": 42
                        }
                    ],
                    "last_error": "boom"
                }
            }
        });
        std::fs::write(
            state_dir.join("executor.json"),
            serde_json::to_vec(&executor_state).expect("executor json"),
        )
        .expect("write executor state");

        let hub = SharedStateHub::new_in_process();
        let rx = hub.snapshot();
        assert!(rx.borrow().plans.is_empty());

        hub.bootstrap_from_workdir(tmpdir.path())
            .expect("bootstrap workdir");

        let snapshot = rx.borrow();
        assert!(snapshot.plans.contains_key("plan-a"));
        assert!(snapshot.tasks.contains_key("plan-a/task-a"));
        assert!(snapshot.agents.contains_key("agent-a"));
        assert_eq!(snapshot.stats.gates_passed, 1);
        assert_eq!(snapshot.stats.errors_total, 1);
    }

    #[test]
    fn event_log_persists_and_replays() {
        let tmpdir = tempfile::tempdir().expect("tempdir");
        let log_path = tmpdir.path().join("events.jsonl");

        // Create hub with event log and publish events.
        let hub = StateHub::with_event_log(16, &log_path);
        hub.publish(DashboardEvent::AgentSpawned {
            agent_id: "a1".into(),
            role: "coder".into(),
            model: String::new(),
        });
        hub.publish(DashboardEvent::PlanStarted {
            plan_id: "p1".into(),
        });

        // Verify file was written.
        let content = std::fs::read_to_string(&log_path).expect("read event log");
        let lines: Vec<&str> = content.lines().filter(|l| !l.is_empty()).collect();
        assert_eq!(lines.len(), 2, "expected 2 lines, got: {content}");

        // Replay into a new hub and verify snapshot matches.
        let (replayed, count) = StateHub::replay_from_log(&log_path);
        assert_eq!(count, 2);
        let snap = replayed.current_snapshot();
        assert_eq!(snap.stats.agents_active, 1);
        assert!(snap.agents.contains_key("a1"));
        assert_eq!(snap.stats.plans_active, 1);
        assert!(snap.plans.contains_key("p1"));
    }

    #[test]
    fn sender_persists_to_event_log() {
        let tmpdir = tempfile::tempdir().expect("tempdir");
        let log_path = tmpdir.path().join("events.jsonl");

        let hub = StateHub::with_event_log(16, &log_path);
        let sender = hub.sender();

        sender.publish(DashboardEvent::AgentSpawned {
            agent_id: "s1".into(),
            role: "auditor".into(),
            model: String::new(),
        });

        let content = std::fs::read_to_string(&log_path).expect("read event log");
        assert!(
            content.contains("s1"),
            "sender should persist to event log: {content}"
        );
    }
}
