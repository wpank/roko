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

use std::collections::{BTreeMap, VecDeque};
use std::fmt;
use std::io::Write;
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, Weak};
use std::time::Duration;

use crate::event_bus::{self, EventBus};
use chrono::{DateTime, Utc};
use fs2::FileExt;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::watch;

use roko_core::dashboard_snapshot::{DashboardEvent, DashboardSnapshot};
use roko_core::{LensScope, ObservableEvent};

/// Default maximum number of JSONL lines retained in `events.jsonl` after compaction.
const EVENT_LOG_MAX_LINES: usize = 50_000;

/// How often (in appended lines) to check whether compaction is needed.
const EVENT_LOG_COMPACT_CHECK_INTERVAL: usize = 500;

/// Default number of versions retained independently for each projection.
pub const DEFAULT_PROJECTION_HISTORY_CAPACITY: usize = 1_024;

/// Default age window retained for projection history (seven days).
pub const DEFAULT_PROJECTION_HISTORY_RETENTION: Duration = Duration::from_hours(168);

/// Append-only JSONL writer for persisting events to disk.
///
/// The writer compacts as soon as its known line count exceeds `max_lines` and
/// also recounts periodically to observe external appenders. Compaction keeps
/// a newest-line hysteresis window and rewrites it atomically.
/// Compaction always operates on complete JSONL lines, so replay never ingests
/// partial JSON caused by truncation.
struct EventLogWriter {
    path: PathBuf,
    /// Maximum number of JSONL lines to retain; 0 means unlimited.
    max_lines: usize,
    /// Complete, non-empty JSONL records known to be in the live file.
    lines_in_log: usize,
    /// Lines appended since the last compaction check.
    lines_since_check: usize,
}

impl EventLogWriter {
    fn open(path: &Path) -> std::io::Result<Self> {
        Self::open_with_max(path, EVENT_LOG_MAX_LINES)
    }

    fn open_with_max(path: &Path, max_lines: usize) -> std::io::Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)?;

        let lines_in_log = count_complete_lines(path)?;
        let mut writer = Self {
            path: path.to_path_buf(),
            max_lines,
            lines_in_log,
            lines_since_check: 0,
        };

        // Bound a log left oversized by a prior process before accepting new
        // events. This also makes the cap effective immediately after restart.
        if max_lines > 0 && lines_in_log > max_lines {
            writer.compact_event_log();
        }

        Ok(writer)
    }

    fn append<T: Serialize>(&mut self, record: &T) {
        // Best-effort: log but don't propagate serialization or I/O errors.
        if let Ok(json) = serde_json::to_string(record) {
            let log_lock = match lock_event_log(&self.path) {
                Ok(lock) => lock,
                Err(error) => {
                    tracing::debug!(%error, path = %self.path.display(), "event log append: failed to lock");
                    return;
                }
            };

            // Open by path for every append. A lifecycle rotation may rename
            // the live file between events; retaining an old file descriptor
            // would otherwise keep writing into the archived generation.
            let append_result = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&self.path)
                .and_then(|file| {
                    let mut writer = std::io::BufWriter::new(file);
                    writeln!(writer, "{json}")?;
                    writer.flush()
                });
            if let Err(error) = append_result {
                tracing::debug!(%error, path = %self.path.display(), "event log append failed");
                return;
            }

            self.lines_in_log = self.lines_in_log.saturating_add(1);
            self.lines_since_check += 1;
            let over_known_limit = self.max_lines > 0 && self.lines_in_log > self.max_lines;
            let periodic_check =
                self.max_lines > 0 && self.lines_since_check >= EVENT_LOG_COMPACT_CHECK_INTERVAL;
            if over_known_limit || periodic_check {
                self.lines_since_check = 0;
                self.compact_event_log_locked();
            }
            drop(log_lock);
        }
    }

    /// Compact the on-disk log by retaining only the newest `max_lines` lines.
    ///
    /// Reads the current file, counts complete JSONL records, drops the oldest
    /// ones, then atomically rewrites the file. The BufWriter is re-opened in
    /// append mode afterwards so subsequent writes continue normally.
    ///
    /// This is best-effort: any I/O error is logged at debug level and the
    /// writer continues normally with the uncompacted file. A successful
    /// compaction retains half the configured cap, providing hysteresis so a
    /// busy writer does not rewrite the entire log on every subsequent event.
    fn compact_event_log(&mut self) {
        let log_lock = match lock_event_log(&self.path) {
            Ok(lock) => lock,
            Err(error) => {
                tracing::debug!(%error, path = %self.path.display(), "event log compact: failed to lock");
                return;
            }
        };
        self.compact_event_log_locked();
        drop(log_lock);
    }

    fn compact_event_log_locked(&mut self) {
        let content = match std::fs::read_to_string(&self.path) {
            Ok(c) => c,
            Err(e) => {
                tracing::debug!(error = %e, "event log compact: failed to read");
                return;
            }
        };

        // Collect non-empty complete lines (valid JSONL records).
        let lines = complete_jsonl_lines(&content);
        self.lines_in_log = lines.len();

        if lines.len() <= self.max_lines {
            // Not above cap; nothing to do.
            return;
        }

        // Keep the newest half-cap. This ensures the file remains bounded
        // while amortizing the cost of the next compaction.
        let retain = (self.max_lines / 2).max(1);
        let keep = &lines[lines.len() - retain..];
        let new_content = keep.join("\n") + "\n";

        // Write to a sibling temp file then rename for atomicity.
        let tmp_path = self.path.with_extension("jsonl.tmp");
        if let Err(e) = std::fs::write(&tmp_path, &new_content) {
            tracing::debug!(error = %e, "event log compact: failed to write tmp");
            return;
        }
        if let Err(e) = std::fs::rename(&tmp_path, &self.path) {
            tracing::debug!(error = %e, "event log compact: failed to rename");
            let _ = std::fs::remove_file(&tmp_path);
            return;
        }
        self.lines_in_log = retain;
        tracing::debug!(
            retained = retain,
            dropped = lines.len().saturating_sub(retain),
            "event log compacted"
        );
    }
}

fn count_complete_lines(path: &Path) -> std::io::Result<usize> {
    Ok(complete_jsonl_lines(&std::fs::read_to_string(path)?).len())
}

fn complete_jsonl_lines(content: &str) -> Vec<&str> {
    content
        .split_inclusive('\n')
        .filter_map(|record| record.strip_suffix('\n'))
        .map(|record| record.strip_suffix('\r').unwrap_or(record))
        .filter(|record| !record.trim().is_empty())
        .collect()
}

/// Hold the same sibling advisory lock used by runner JSONL appenders and
/// filesystem log rotation. Keeping the lock on a separate inode makes it
/// survive rename-and-recreate of the live JSONL path.
fn lock_event_log(path: &Path) -> std::io::Result<std::fs::File> {
    let lock_path = path.with_extension("jsonl.lock");
    let lock = std::fs::OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .truncate(false)
        .open(lock_path)?;
    lock.lock_exclusive()?;
    Ok(lock)
}

/// Whether a [`DashboardEvent`] should be persisted to the on-disk event log.
///
/// High-volume heartbeat variants (`FeedTick`, `ChainBlock`) are broadcast to
/// live SSE/WS subscribers and applied to the in-memory snapshot, but skipped
/// for on-disk persistence to avoid unbounded growth of `.roko/events.jsonl`.
fn should_persist(event: &DashboardEvent) -> bool {
    !matches!(
        event,
        DashboardEvent::FeedTick { .. }
            | DashboardEvent::ChainBlock { .. }
            | DashboardEvent::ProjectionUpdated { .. }
    )
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

/// Resolve the durable projection-history log that accompanies an event log.
///
/// The production event log is `.roko/events.jsonl`, whose companion is the
/// documented `.roko/projection-history.jsonl`. Custom log names get a
/// name-scoped companion so independent hubs in one directory do not collide.
fn projection_history_log_path(event_log_path: &Path) -> PathBuf {
    if event_log_path.file_name().and_then(|name| name.to_str()) == Some("events.jsonl") {
        event_log_path.with_file_name("projection-history.jsonl")
    } else {
        let stem = event_log_path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or("events");
        event_log_path.with_file_name(format!("{stem}.projection-history.jsonl"))
    }
}

/// An atomic cursor/snapshot read of a [`StateHub`].
///
/// `next_seq` is the first sequence not represented by `snapshot` at the time
/// this value was captured. Snapshot-only mutations may also be present.
pub struct StateHubCursorSnapshot {
    /// Sequence that the next published event will receive.
    pub next_seq: u64,
    /// Materialized state after every event below `next_seq` was applied.
    pub snapshot: DashboardSnapshot,
    /// Durable generation captured with `snapshot` and `next_seq`.
    pub provenance: StateHubSnapshotProvenance,
    /// Number of replay-ring entries at the same cursor boundary.
    pub ring_len: usize,
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

/// Current materialized value for one typed telemetry projection.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ProjectionState {
    /// Stable projection identifier, such as `cohort_health`.
    pub id: String,
    /// Monotonic version incremented for every successful update.
    pub version: u64,
    /// Projection payload serialized independently of its schema type.
    pub data: Value,
    /// RFC 3339 update timestamp.
    pub updated_at: String,
    /// Lenses that have contributed to the current projection.
    pub source_lenses: Vec<String>,
}

#[derive(Default)]
struct ProjectionStore {
    current: BTreeMap<String, ProjectionState>,
    history: BTreeMap<String, VecDeque<ProjectionState>>,
    history_capacity: usize,
    history_retention: Duration,
}

impl ProjectionStore {
    fn new(history_capacity: usize, history_retention: Duration) -> Self {
        Self {
            history_capacity,
            history_retention,
            ..Self::default()
        }
    }
}

fn restore_projection_store(
    path: &Path,
    history_capacity: usize,
    history_retention: Duration,
) -> ProjectionStore {
    let content = match std::fs::read_to_string(path) {
        Ok(content) => content,
        Err(_) => return ProjectionStore::new(history_capacity, history_retention),
    };
    let mut restored = BTreeMap::<String, BTreeMap<u64, ProjectionState>>::new();
    for line in complete_jsonl_lines(&content) {
        let Ok(projection) = serde_json::from_str::<ProjectionState>(line) else {
            continue;
        };
        if projection.id.trim().is_empty()
            || projection.version == 0
            || DateTime::parse_from_rfc3339(&projection.updated_at).is_err()
        {
            continue;
        }
        restored
            .entry(projection.id.clone())
            .or_default()
            .insert(projection.version, projection);
    }

    let mut store = ProjectionStore::new(history_capacity, history_retention);
    let now = Utc::now();
    for (id, versions) in restored {
        let Some(current) = versions.last_key_value().map(|(_, value)| value.clone()) else {
            continue;
        };
        store.current.insert(id.clone(), current);
        if history_capacity > 0 {
            let retained = versions
                .into_values()
                .filter(|projection| projection_is_retained(projection, &now, history_retention))
                .collect::<Vec<_>>();
            let start = retained.len().saturating_sub(history_capacity);
            store.history.insert(
                id,
                retained.into_iter().skip(start).collect::<VecDeque<_>>(),
            );
        }
    }
    store
}

fn projection_is_retained(
    projection: &ProjectionState,
    now: &DateTime<Utc>,
    retention: Duration,
) -> bool {
    let Ok(updated_at) = DateTime::parse_from_rfc3339(&projection.updated_at) else {
        return false;
    };
    let Ok(retention) = chrono::Duration::from_std(retention) else {
        return true;
    };
    updated_at.with_timezone(&Utc) >= *now - retention
}

type SharedProjections = Arc<Mutex<ProjectionStore>>;

/// Operator-facing status for one Lens implementation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LensOperatorStatus {
    /// Declarative Lens registration name.
    pub lens: String,
    /// Whether operator controls currently permit invocation.
    pub enabled: bool,
    /// Current breaker stage (`active`, `sampled`, or `disabled`).
    pub breaker_stage: String,
    /// Total overhead-budget violations observed by the breaker.
    pub total_violations: u64,
    /// Lens invocations attempted by the executor.
    pub invocations: u64,
    /// Observations omitted by deterministic sampling.
    pub sampled_out: u64,
    /// Invocations that returned errors or failed as tasks.
    pub failures: u64,
    /// Signals emitted by successful invocations.
    pub emitted_signals: u64,
}

/// Bounded delivery-queue accounting for one Lens runtime.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LensQueueSnapshot {
    /// Maximum number of pending observations.
    pub capacity: usize,
    /// Current number of pending observations.
    pub depth: usize,
    /// Current observations being dispatched.
    pub in_flight: usize,
    /// Total observations accepted by this queue.
    pub enqueued: u64,
    /// Total observations fully dispatched.
    pub processed: u64,
    /// Pending observations evicted by drop-oldest backpressure.
    pub dropped_oldest: u64,
    /// Dispatches containing Lens or projection failures.
    pub failed_dispatches: u64,
    /// Stable backpressure policy label.
    pub backpressure: String,
    /// Most recent dispatch error, if any.
    pub last_error: Option<String>,
}

/// Complete operator snapshot for one queued Lens executor.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LensRuntimeSnapshot {
    /// Stable runtime identifier used by operator APIs.
    pub runtime_id: String,
    /// Delivery queue state and lifetime counters.
    pub queue: LensQueueSnapshot,
    /// Per-Lens breaker and execution state.
    pub lenses: Vec<LensOperatorStatus>,
}

/// Object-safe operator boundary implemented by queued Lens executors.
pub trait LensRuntimeControl: Send + Sync {
    /// Stable identifier for this live runtime.
    fn runtime_id(&self) -> &str;
    /// Capture current queue and Lens state.
    fn snapshot(&self) -> LensRuntimeSnapshot;
    /// Reset one Lens into enabled sampled recovery.
    fn reset_lens(&self, lens: &str) -> std::result::Result<(), String>;
    /// Explicitly enable or disable one Lens.
    fn set_lens_enabled(&self, lens: &str, enabled: bool) -> std::result::Result<(), String>;
    /// Enqueue a lifecycle observation for this live runtime.
    ///
    /// Returns `true` when drop-oldest backpressure replaced a pending event.
    fn enqueue_observable(
        &self,
        event: &ObservableEvent,
        ancestry: &[LensScope],
    ) -> std::result::Result<bool, String>;
}

type SharedLensRuntimes = Arc<Mutex<BTreeMap<String, Weak<dyn LensRuntimeControl>>>>;

/// Durable generation captured atomically with a materialized dashboard snapshot.
#[derive(Clone, Debug, Default)]
pub struct StateHubSnapshotProvenance {
    /// Whether a workspace bootstrap has completed, including a missing source.
    pub bootstrapped: bool,
    /// Whether the snapshot includes a successfully recovered durable baseline.
    pub recovered: bool,
    /// Exact verified Runner generation used to build the recovered baseline.
    pub runner: Option<crate::DurableRunnerProjection>,
    /// Stable source status when no verified Runner generation is available.
    pub source_status: Option<String>,
    /// Canonical source path used for missing/invalid diagnostics.
    pub source_path: Option<PathBuf>,
    /// Loader error retained instead of being discarded at bootstrap.
    pub source_error: Option<String>,
    /// Whether live dashboard events have been applied after recovery.
    pub live_events_applied: bool,
    /// Whether the legacy event log has already been folded into this baseline.
    pub event_log_replayed: bool,
    /// Monotonic materialized-state revision used for compare-and-install.
    pub revision: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct RecoveryInstallToken {
    bootstrapped: bool,
    live_events_applied: bool,
    revision: u64,
    generation: Option<String>,
}

impl From<&StateHubSnapshotProvenance> for RecoveryInstallToken {
    fn from(provenance: &StateHubSnapshotProvenance) -> Self {
        Self {
            bootstrapped: provenance.bootstrapped,
            live_events_applied: provenance.live_events_applied,
            revision: provenance.revision,
            generation: provenance
                .runner
                .as_ref()
                .map(|runner| runner.generation.clone()),
        }
    }
}

/// One coherent StateHub read: dashboard and the durable generation that built it.
#[derive(Clone, Debug)]
pub struct StateHubMaterializedSnapshot {
    /// Dashboard state captured under the StateHub publication lock.
    pub snapshot: DashboardSnapshot,
    /// Durable-source metadata captured at the same publication boundary.
    pub provenance: StateHubSnapshotProvenance,
}

/// Central state management hub for dashboard snapshots, events, and projections.
pub struct StateHub {
    snapshot_tx: watch::Sender<DashboardSnapshot>,
    snapshot_rx: watch::Receiver<DashboardSnapshot>,
    event_bus: EventBus<DashboardEvent>,
    /// Serializes the snapshot commit, best-effort append, and sequence assignment
    /// so subscribers can never observe event N against state older than N.
    publish_lock: Arc<Mutex<()>>,
    provenance: Arc<Mutex<StateHubSnapshotProvenance>>,
    /// Optional on-disk event log for persistence across restarts.
    event_log: Option<SharedEventLog>,
    /// Optional durable append log for typed projection versions.
    projection_history_log: Option<SharedEventLog>,
    /// Latest typed telemetry projection values, keyed by stable ID.
    projections: SharedProjections,
    /// Weak registrations for live queued Lens executors.
    lens_runtimes: SharedLensRuntimes,
}

impl StateHub {
    /// Create a new hub with the given replay ring capacity.
    pub fn new(ring_capacity: usize) -> Self {
        Self::with_projection_history_capacity(ring_capacity, DEFAULT_PROJECTION_HISTORY_CAPACITY)
    }

    /// Create a hub with explicit bounded per-projection history retention.
    pub fn with_projection_history_capacity(
        ring_capacity: usize,
        projection_history_capacity: usize,
    ) -> Self {
        Self::with_projection_history_policy(
            ring_capacity,
            projection_history_capacity,
            DEFAULT_PROJECTION_HISTORY_RETENTION,
        )
    }

    /// Create an in-process hub with explicit projection count and age retention.
    pub fn with_projection_history_policy(
        ring_capacity: usize,
        projection_history_capacity: usize,
        projection_history_retention: Duration,
    ) -> Self {
        let (snapshot_tx, snapshot_rx) = watch::channel(DashboardSnapshot::default());
        Self {
            snapshot_tx,
            snapshot_rx,
            event_bus: EventBus::new(ring_capacity),
            publish_lock: Arc::new(Mutex::new(())),
            provenance: Arc::new(Mutex::new(StateHubSnapshotProvenance::default())),
            event_log: None,
            projection_history_log: None,
            projections: Arc::new(Mutex::new(ProjectionStore::new(
                projection_history_capacity,
                projection_history_retention,
            ))),
            lens_runtimes: Arc::new(Mutex::new(BTreeMap::new())),
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
    ///
    /// The writer applies the default line cap ([`EVENT_LOG_MAX_LINES`]) and
    /// compacts on-disk to keep only the newest N lines when the cap is
    /// exceeded. Pass [`Self::with_event_log_capped`] to override the cap.
    pub fn with_event_log(ring_capacity: usize, log_path: &Path) -> Self {
        Self::with_event_log_and_projection_retention(
            ring_capacity,
            log_path,
            DEFAULT_PROJECTION_HISTORY_RETENTION,
        )
    }

    /// Create a durable hub with explicit time-based projection history retention.
    pub fn with_event_log_and_projection_retention(
        ring_capacity: usize,
        log_path: &Path,
        projection_history_retention: Duration,
    ) -> Self {
        Self::with_event_log_capped_and_projection_retention(
            ring_capacity,
            log_path,
            EVENT_LOG_MAX_LINES,
            projection_history_retention,
        )
    }

    /// Create a new hub that persists events to the given JSONL log file with
    /// a custom maximum line count.
    ///
    /// When `max_lines` is `0` the log grows without bound (no cap). Otherwise
    /// the writer compacts the file to at most `max_lines` complete JSONL
    /// records as soon as the cap is crossed. Periodic recounts incorporate
    /// records written by external appenders.
    pub fn with_event_log_capped(ring_capacity: usize, log_path: &Path, max_lines: usize) -> Self {
        Self::with_event_log_capped_and_projection_retention(
            ring_capacity,
            log_path,
            max_lines,
            DEFAULT_PROJECTION_HISTORY_RETENTION,
        )
    }

    fn with_event_log_capped_and_projection_retention(
        ring_capacity: usize,
        log_path: &Path,
        max_lines: usize,
        projection_history_retention: Duration,
    ) -> Self {
        let event_log = EventLogWriter::open_with_max(log_path, max_lines)
            .map(|w| Arc::new(Mutex::new(w)))
            .ok();
        if event_log.is_none() {
            tracing::warn!(
                path = %log_path.display(),
                "failed to open event log; events will not be persisted"
            );
        }
        let projection_history_path = projection_history_log_path(log_path);
        let projection_history_log = EventLogWriter::open(&projection_history_path)
            .map(|writer| Arc::new(Mutex::new(writer)))
            .ok();
        if projection_history_log.is_none() {
            tracing::warn!(
                path = %projection_history_path.display(),
                "failed to open projection history log; projection versions will not be persisted"
            );
        }
        let projection_store = restore_projection_store(
            &projection_history_path,
            DEFAULT_PROJECTION_HISTORY_CAPACITY,
            projection_history_retention,
        );
        let (snapshot_tx, snapshot_rx) = watch::channel(DashboardSnapshot::default());
        Self {
            snapshot_tx,
            snapshot_rx,
            event_bus: EventBus::new(ring_capacity),
            publish_lock: Arc::new(Mutex::new(())),
            provenance: Arc::new(Mutex::new(StateHubSnapshotProvenance::default())),
            event_log,
            projection_history_log,
            projections: Arc::new(Mutex::new(projection_store)),
            lens_runtimes: Arc::new(Mutex::new(BTreeMap::new())),
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
        let projection_history_path = projection_history_log_path(log_path);
        match EventLogWriter::open(&projection_history_path) {
            Ok(writer) => {
                self.projection_history_log = Some(Arc::new(Mutex::new(writer)));
            }
            Err(error) => tracing::warn!(
                path = %projection_history_path.display(),
                %error,
                "failed to open projection history log"
            ),
        }
    }

    /// Publish an event: apply it to the snapshot, append it to the optional
    /// best-effort log, then record and broadcast it on the replay bus.
    /// Returns the sequence number assigned on the internal event bus.
    ///
    /// High-volume heartbeat variants (`FeedTick`, `ChainBlock`) are broadcast
    /// to live subscribers and applied to the snapshot but **not** persisted to
    /// the on-disk event log.
    pub fn publish(&self, event: DashboardEvent) -> u64 {
        let _publish = self
            .publish_lock
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        self.snapshot_tx.send_modify(|snap| snap.apply(&event));
        self.mark_live_event();
        if should_persist(&event)
            && let Some(log) = &self.event_log
            && let Ok(mut writer) = log.lock()
        {
            writer.append(&event);
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
        if !events.is_empty() {
            self.mark_live_event();
        }
        for event in events {
            if should_persist(&event)
                && let Some(log) = &self.event_log
                && let Ok(mut writer) = log.lock()
            {
                writer.append(&event);
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
        let mut provenance = self
            .provenance
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let revision = provenance.revision.saturating_add(1);
        let _ = self.snapshot_tx.send(snapshot);
        *provenance = StateHubSnapshotProvenance {
            live_events_applied: true,
            revision,
            ..StateHubSnapshotProvenance::default()
        };
    }

    fn apply_recovered_snapshot_if_unchanged(
        &self,
        expected: &RecoveryInstallToken,
        snapshot: DashboardSnapshot,
        mut provenance: StateHubSnapshotProvenance,
    ) -> bool {
        let _publish = self
            .publish_lock
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let mut current = self
            .provenance
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if RecoveryInstallToken::from(&*current) != *expected {
            return false;
        }
        provenance.revision = current.revision.saturating_add(1);
        let _ = self.snapshot_tx.send(snapshot);
        *current = provenance;
        true
    }

    fn mark_live_event(&self) {
        let mut provenance = self
            .provenance
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        provenance.live_events_applied = true;
        provenance.revision = provenance.revision.saturating_add(1);
    }

    /// Mutate selected snapshot fields atomically without a lossy
    /// read-modify-write round trip through [`current_snapshot`](Self::current_snapshot).
    pub fn update_snapshot(&self, update: impl FnOnce(&mut DashboardSnapshot)) {
        let _publish = self
            .publish_lock
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        self.snapshot_tx.send_modify(update);
        let mut provenance = self
            .provenance
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        provenance.live_events_applied = true;
        provenance.revision = provenance.revision.saturating_add(1);
    }

    /// Add state recovered from other durable stores without converting a
    /// restart snapshot into a live-event snapshot.
    ///
    /// Returns `false` when bootstrap has not completed or a live mutation has
    /// already occurred, so persisted hydration can never overwrite live data.
    pub fn hydrate_recovered_snapshot(&self, update: impl FnOnce(&mut DashboardSnapshot)) -> bool {
        let _publish = self
            .publish_lock
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let mut provenance = self
            .provenance
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if !provenance.bootstrapped || provenance.live_events_applied {
            return false;
        }
        self.snapshot_tx.send_modify(update);
        provenance.revision = provenance.revision.saturating_add(1);
        true
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

    /// Capture dashboard and source provenance under the same publication lock.
    pub fn current_snapshot_with_provenance(&self) -> StateHubMaterializedSnapshot {
        let _publish = self
            .publish_lock
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        StateHubMaterializedSnapshot {
            snapshot: self.snapshot_rx.borrow().clone(),
            provenance: self
                .provenance
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .clone(),
        }
    }

    /// Replace one projection's current value and advance its version.
    pub fn update_projection(&self, id: &str, data: Value, source_lens: &str) {
        let _publish = self
            .publish_lock
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let projection = update_projection_store(&self.projections, id, data, source_lens);
        append_projection_history(self.projection_history_log.as_ref(), &projection);
        self.event_bus.emit(DashboardEvent::ProjectionUpdated {
            projection_id: projection.id,
            version: projection.version,
            source_lens: source_lens.to_string(),
        });
    }

    /// Clone the current state for a projection.
    pub fn get_projection(&self, id: &str) -> Option<ProjectionState> {
        get_projection_store(&self.projections, id)
    }

    /// Return a projection's current monotonic version.
    pub fn projection_version(&self, id: &str) -> Option<u64> {
        self.get_projection(id).map(|projection| projection.version)
    }

    /// Clone all current projection states in deterministic key order.
    pub fn projections(&self) -> BTreeMap<String, ProjectionState> {
        self.projections
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .current
            .clone()
    }

    /// Clone retained versions for one projection, oldest to newest.
    pub fn projection_history(&self, id: &str) -> Vec<ProjectionState> {
        projection_history_store(&self.projections, id)
    }

    /// Maximum retained versions per projection.
    pub fn projection_history_capacity(&self) -> usize {
        self.projections
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .history_capacity
    }

    /// Maximum age retained for historical projection versions.
    pub fn projection_history_retention(&self) -> Duration {
        self.projections
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .history_retention
    }

    /// Inspect every currently live queued Lens runtime.
    pub fn lens_runtime_snapshots(&self) -> Vec<LensRuntimeSnapshot> {
        live_lens_runtimes(&self.lens_runtimes)
            .into_iter()
            .map(|runtime| runtime.snapshot())
            .collect()
    }

    /// Inspect one live queued Lens runtime.
    pub fn lens_runtime_snapshot(&self, runtime_id: &str) -> Option<LensRuntimeSnapshot> {
        live_lens_runtime(&self.lens_runtimes, runtime_id).map(|runtime| runtime.snapshot())
    }

    /// Reset a Lens breaker and re-enable delivery in sampled mode.
    pub fn reset_lens_runtime(
        &self,
        runtime_id: &str,
        lens: &str,
    ) -> std::result::Result<(), String> {
        live_lens_runtime(&self.lens_runtimes, runtime_id)
            .ok_or_else(|| format!("unknown live Lens runtime `{runtime_id}`"))?
            .reset_lens(lens)
    }

    /// Explicitly enable or disable a Lens in a live runtime.
    pub fn set_lens_runtime_enabled(
        &self,
        runtime_id: &str,
        lens: &str,
        enabled: bool,
    ) -> std::result::Result<(), String> {
        live_lens_runtime(&self.lens_runtimes, runtime_id)
            .ok_or_else(|| format!("unknown live Lens runtime `{runtime_id}`"))?
            .set_lens_enabled(lens, enabled)
    }

    /// Fan one lifecycle observation out to every currently live Lens runtime.
    ///
    /// The returned strings identify queue delivery failures. An empty runtime
    /// registry is a normal no-subscriber condition and returns no errors.
    pub fn emit_observable(&self, event: &ObservableEvent, ancestry: &[LensScope]) -> Vec<String> {
        emit_observable_to_runtimes(&self.lens_runtimes, event, ancestry)
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
            provenance: self
                .provenance
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .clone(),
            ring_len: self.event_bus.ring_len(),
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
            provenance: self
                .provenance
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .clone(),
            ring_len: self.event_bus.ring_len(),
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
        let content = match crate::state_snapshot::read_bounded_text(log_path) {
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
        let content = match crate::state_snapshot::read_bounded_text(log_path) {
            Ok(c) => c,
            Err(_) => return 0,
        };
        let _publish = self
            .publish_lock
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let mut provenance = self
            .provenance
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if provenance.event_log_replayed
            || provenance
                .runner
                .as_ref()
                .is_some_and(|runner| runner.source == crate::RunnerProjectionSource::StateSnapshot)
        {
            return 0;
        }
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
        provenance.event_log_replayed = true;
        provenance.revision = provenance.revision.saturating_add(1);
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
            provenance: Arc::clone(&self.provenance),
            event_log: self.event_log.clone(),
            projection_history_log: self.projection_history_log.clone(),
            projections: Arc::clone(&self.projections),
            lens_runtimes: Arc::clone(&self.lens_runtimes),
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
    provenance: Arc<Mutex<StateHubSnapshotProvenance>>,
    event_log: Option<SharedEventLog>,
    projection_history_log: Option<SharedEventLog>,
    projections: SharedProjections,
    lens_runtimes: SharedLensRuntimes,
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
        {
            let mut provenance = self
                .provenance
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            provenance.live_events_applied = true;
            provenance.revision = provenance.revision.saturating_add(1);
        }
        if should_persist(&event)
            && let Some(log) = &self.event_log
            && let Ok(mut writer) = log.lock()
        {
            writer.append(&event);
        }
        self.bus_sender.emit(event)
    }

    /// Replace one projection's current value and advance its version.
    pub fn update_projection(&self, id: &str, data: Value, source_lens: &str) {
        let _publish = self
            .publish_lock
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let projection = update_projection_store(&self.projections, id, data, source_lens);
        append_projection_history(self.projection_history_log.as_ref(), &projection);
        self.bus_sender.emit(DashboardEvent::ProjectionUpdated {
            projection_id: projection.id,
            version: projection.version,
            source_lens: source_lens.to_string(),
        });
    }

    /// Clone the current state for a projection.
    pub fn get_projection(&self, id: &str) -> Option<ProjectionState> {
        get_projection_store(&self.projections, id)
    }

    /// Return a projection's current monotonic version.
    pub fn projection_version(&self, id: &str) -> Option<u64> {
        self.get_projection(id).map(|projection| projection.version)
    }

    /// Clone all current projection states in deterministic key order.
    pub fn projections(&self) -> BTreeMap<String, ProjectionState> {
        self.projections
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .current
            .clone()
    }

    /// Clone retained versions for one projection, oldest to newest.
    pub fn projection_history(&self, id: &str) -> Vec<ProjectionState> {
        projection_history_store(&self.projections, id)
    }

    /// Register a live runtime without extending its lifetime.
    pub fn register_lens_runtime(
        &self,
        runtime: &Arc<dyn LensRuntimeControl>,
    ) -> std::result::Result<(), String> {
        let runtime_id = runtime.runtime_id().trim();
        if runtime_id.is_empty() {
            return Err("Lens runtime ID must not be empty".to_string());
        }
        let mut runtimes = self
            .lens_runtimes
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if runtimes.get(runtime_id).and_then(Weak::upgrade).is_some() {
            return Err(format!("Lens runtime `{runtime_id}` is already registered"));
        }
        runtimes.insert(runtime_id.to_string(), Arc::downgrade(runtime));
        Ok(())
    }

    /// Fan one lifecycle observation out to every currently live Lens runtime.
    pub fn emit_observable(&self, event: &ObservableEvent, ancestry: &[LensScope]) -> Vec<String> {
        emit_observable_to_runtimes(&self.lens_runtimes, event, ancestry)
    }
}

fn update_projection_store(
    projections: &SharedProjections,
    id: &str,
    data: Value,
    source_lens: &str,
) -> ProjectionState {
    let mut projections = projections
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let previous = projections.current.get(id);
    let version = previous.map_or(1, |projection| projection.version.saturating_add(1));
    let mut source_lenses = previous
        .map(|projection| projection.source_lenses.clone())
        .unwrap_or_default();
    if !source_lens.is_empty() && !source_lenses.iter().any(|lens| lens == source_lens) {
        source_lenses.push(source_lens.to_string());
    }
    let projection = ProjectionState {
        id: id.to_string(),
        version,
        data,
        updated_at: Utc::now().to_rfc3339(),
        source_lenses,
    };
    projections
        .current
        .insert(id.to_string(), projection.clone());
    if projections.history_capacity > 0 {
        let capacity = projections.history_capacity;
        let retention = projections.history_retention;
        let now = Utc::now();
        let history = projections.history.entry(id.to_string()).or_default();
        history.retain(|projection| projection_is_retained(projection, &now, retention));
        while history.len() >= capacity {
            history.pop_front();
        }
        history.push_back(projection.clone());
    }
    projection
}

fn get_projection_store(projections: &SharedProjections, id: &str) -> Option<ProjectionState> {
    projections
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .current
        .get(id)
        .cloned()
}

fn projection_history_store(projections: &SharedProjections, id: &str) -> Vec<ProjectionState> {
    let mut projections = projections
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let retention = projections.history_retention;
    let now = Utc::now();
    let Some(history) = projections.history.get_mut(id) else {
        return Vec::new();
    };
    history.retain(|projection| projection_is_retained(projection, &now, retention));
    history.iter().cloned().collect()
}

fn append_projection_history(log: Option<&SharedEventLog>, projection: &ProjectionState) {
    if let Some(log) = log
        && let Ok(mut writer) = log.lock()
    {
        writer.append(projection);
    }
}

fn live_lens_runtime(
    runtimes: &SharedLensRuntimes,
    runtime_id: &str,
) -> Option<Arc<dyn LensRuntimeControl>> {
    let mut runtimes = runtimes
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let runtime = runtimes.get(runtime_id).and_then(Weak::upgrade);
    if runtime.is_none() {
        runtimes.remove(runtime_id);
    }
    runtime
}

fn live_lens_runtimes(runtimes: &SharedLensRuntimes) -> Vec<Arc<dyn LensRuntimeControl>> {
    let mut runtimes = runtimes
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let live = runtimes
        .values()
        .filter_map(Weak::upgrade)
        .collect::<Vec<_>>();
    runtimes.retain(|_, runtime| runtime.strong_count() > 0);
    live
}

fn emit_observable_to_runtimes(
    runtimes: &SharedLensRuntimes,
    event: &ObservableEvent,
    ancestry: &[LensScope],
) -> Vec<String> {
    live_lens_runtimes(runtimes)
        .into_iter()
        .filter_map(|runtime| {
            let runtime_id = runtime.runtime_id().to_string();
            match runtime.enqueue_observable(event, ancestry) {
                Ok(replaced_oldest) => {
                    if replaced_oldest {
                        tracing::warn!(
                            runtime = %runtime_id,
                            "Lens observation queue full; dropped oldest pending event"
                        );
                    }
                    None
                }
                Err(error) => Some(format!("Lens runtime `{runtime_id}`: {error}")),
            }
        })
        .collect()
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
        let expected =
            RecoveryInstallToken::from(&self.0.current_snapshot_with_provenance().provenance);
        if expected.bootstrapped || expected.live_events_applied {
            return Ok(());
        }
        match crate::state_snapshot::load_durable_dashboard_projection(workdir) {
            Ok(projection) => {
                let runner = projection.runner;
                self.0.apply_recovered_snapshot_if_unchanged(
                    &expected,
                    projection.dashboard,
                    StateHubSnapshotProvenance {
                        bootstrapped: true,
                        recovered: runner.is_some(),
                        source_status: Some(
                            runner
                                .as_ref()
                                .map_or("missing", |runner| runner.source.label())
                                .to_string(),
                        ),
                        source_path: Some(runner.as_ref().map_or_else(
                            || workdir.join(crate::STATE_SNAPSHOT_RELATIVE_PATH),
                            |runner| runner.source_path.clone(),
                        )),
                        runner,
                        source_error: None,
                        live_events_applied: false,
                        event_log_replayed: false,
                        revision: 0,
                    },
                );
                Ok(())
            }
            Err(err) => {
                tracing::warn!(
                    error = %err,
                    workdir = %workdir.display(),
                    "failed to bootstrap dashboard snapshot from workdir; using empty snapshot"
                );
                self.0.apply_recovered_snapshot_if_unchanged(
                    &expected,
                    DashboardSnapshot::default(),
                    StateHubSnapshotProvenance {
                        bootstrapped: true,
                        recovered: false,
                        runner: None,
                        source_status: Some("invalid".to_string()),
                        source_path: Some(workdir.join(crate::STATE_SNAPSHOT_RELATIVE_PATH)),
                        source_error: Some(err.to_string()),
                        live_events_applied: false,
                        event_log_replayed: false,
                        revision: 0,
                    },
                );
                Err(err)
            }
        }
    }

    /// Refresh a purely recovered hub from one newer atomic durable generation.
    /// Live event overlays are never discarded.
    pub fn refresh_recovered_from_workdir(&self, workdir: &Path) -> Result<(), std::io::Error> {
        let current = self.0.current_snapshot_with_provenance();
        if !current.provenance.bootstrapped || current.provenance.live_events_applied {
            return Ok(());
        }
        let projection = match crate::state_snapshot::load_durable_dashboard_projection(workdir) {
            Ok(projection) => projection,
            Err(error) => {
                self.0.apply_recovered_snapshot_if_unchanged(
                    &RecoveryInstallToken::from(&current.provenance),
                    DashboardSnapshot::default(),
                    StateHubSnapshotProvenance {
                        bootstrapped: true,
                        recovered: false,
                        runner: None,
                        source_status: Some("invalid".to_string()),
                        source_path: Some(workdir.join(crate::STATE_SNAPSHOT_RELATIVE_PATH)),
                        source_error: Some(error.to_string()),
                        live_events_applied: false,
                        event_log_replayed: false,
                        revision: 0,
                    },
                );
                return Err(error);
            }
        };
        let runner = projection.runner;
        let current_generation = current
            .provenance
            .runner
            .as_ref()
            .map(|runner| runner.generation.as_str());
        let next_generation = runner.as_ref().map(|runner| runner.generation.as_str());
        if current_generation == next_generation {
            return Ok(());
        }
        self.0.apply_recovered_snapshot_if_unchanged(
            &RecoveryInstallToken::from(&current.provenance),
            projection.dashboard,
            StateHubSnapshotProvenance {
                bootstrapped: true,
                recovered: runner.is_some(),
                source_status: Some(
                    runner
                        .as_ref()
                        .map_or("missing", |runner| runner.source.label())
                        .to_string(),
                ),
                source_path: Some(runner.as_ref().map_or_else(
                    || workdir.join(crate::STATE_SNAPSHOT_RELATIVE_PATH),
                    |runner| runner.source_path.clone(),
                )),
                runner,
                source_error: None,
                live_events_applied: false,
                event_log_replayed: false,
                revision: 0,
            },
        );
        Ok(())
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

    fn unified_fixture(plan_id: &str) -> crate::StateSnapshot {
        let executor = serde_json::json!({
            "schema_version": 1,
            "plan_states": {(plan_id): {"plan_id": plan_id, "current_phase": {"kind": "implementing"}}},
            "queue_order": [plan_id],
            "speculative_executions": {},
            "timestamp_ms": 42
        });
        crate::StateSnapshot::new(
            42,
            executor.to_string(),
            serde_json::json!({"schema_version": 1, "executor": executor, "timestamp_ms": 42})
                .to_string(),
            serde_json::json!({
                "schema_version": 1,
                "run_id": "run-1",
                "timestamp_ms": 42,
                "tasks_total": 0,
                "tasks_completed": 0,
                "tasks_failed": 0,
                "total_tokens_in": 0,
                "total_tokens_out": 0,
                "total_cost_usd": 0.0,
                "total_agent_calls": 0,
                "replan_ledger": {}
            })
            .to_string(),
            serde_json::json!({"rungs": {}}).to_string(),
        )
    }

    #[test]
    fn recovery_compare_install_never_overwrites_intervening_live_publish() {
        let hub = StateHub::default_capacity();
        let expected =
            RecoveryInstallToken::from(&hub.current_snapshot_with_provenance().provenance);
        hub.publish(DashboardEvent::PlanStarted {
            plan_id: "live".to_string(),
        });
        let mut stale = DashboardSnapshot::default();
        stale.plans.insert(
            "stale".to_string(),
            roko_core::dashboard_snapshot::PlanState {
                plan_id: "stale".to_string(),
                phase: "implementing".to_string(),
                tasks_total: 1,
                tasks_done: 0,
                tasks_failed: 0,
                active: true,
            },
        );
        assert!(!hub.apply_recovered_snapshot_if_unchanged(
            &expected,
            stale,
            StateHubSnapshotProvenance {
                bootstrapped: true,
                recovered: true,
                ..StateHubSnapshotProvenance::default()
            },
        ));
        let captured = hub.cursor_snapshot();
        assert!(captured.snapshot.plans.contains_key("live"));
        assert!(!captured.snapshot.plans.contains_key("stale"));
        assert!(captured.provenance.live_events_applied);
        assert_eq!(captured.next_seq, 1);
        assert_eq!(captured.ring_len, 1);
    }

    #[test]
    fn request_fallback_bootstrap_never_overwrites_preexisting_live_state() {
        let tmpdir = tempfile::tempdir().unwrap();
        let path = tmpdir.path().join(crate::STATE_SNAPSHOT_RELATIVE_PATH);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, serde_json::to_vec(&unified_fixture("disk")).unwrap()).unwrap();
        let hub = SharedStateHub::new_in_process();
        hub.publish(DashboardEvent::PlanStarted {
            plan_id: "live".to_string(),
        });
        hub.bootstrap_from_workdir(tmpdir.path()).unwrap();
        let captured = hub.cursor_snapshot();
        assert!(captured.snapshot.plans.contains_key("live"));
        assert!(!captured.snapshot.plans.contains_key("disk"));
        assert!(!captured.provenance.bootstrapped);
    }

    #[test]
    fn projection_updates_increment_version_and_preserve_sources() {
        let hub = StateHub::default_capacity();
        assert_eq!(hub.projection_version("cohort_health"), None);

        hub.update_projection(
            "cohort_health",
            serde_json::json!({"agent_count": 2}),
            "cohort",
        );
        hub.update_projection(
            "cohort_health",
            serde_json::json!({"agent_count": 3}),
            "vitality",
        );
        hub.update_projection(
            "cohort_health",
            serde_json::json!({"agent_count": 4}),
            "cohort",
        );

        let projection = hub.get_projection("cohort_health").unwrap();
        assert_eq!(projection.version, 3);
        assert_eq!(projection.data["agent_count"], 4);
        assert_eq!(projection.source_lenses, vec!["cohort", "vitality"]);
        assert!(chrono::DateTime::parse_from_rfc3339(&projection.updated_at).is_ok());
    }

    #[test]
    fn sender_and_hub_share_projection_store() {
        let hub = StateHub::default_capacity();
        let sender = hub.sender();
        sender.update_projection("cost_meter", serde_json::json!({"total_usd": 1.5}), "cost");

        assert_eq!(hub.projection_version("cost_meter"), Some(1));
        assert_eq!(
            sender.get_projection("cost_meter").unwrap().data["total_usd"],
            1.5
        );
    }

    #[test]
    fn projection_history_is_bounded_and_shared_with_sender() {
        let hub = StateHub::with_projection_history_capacity(16, 2);
        let sender = hub.sender();
        hub.update_projection("cost_meter", serde_json::json!({"total_usd": 1}), "cost");
        sender.update_projection("cost_meter", serde_json::json!({"total_usd": 2}), "cost");
        hub.update_projection("cost_meter", serde_json::json!({"total_usd": 3}), "trend");

        let history = hub.projection_history("cost_meter");
        assert_eq!(history.len(), 2);
        assert_eq!(
            history
                .iter()
                .map(|projection| projection.version)
                .collect::<Vec<_>>(),
            [2, 3]
        );
        assert_eq!(history[0].data["total_usd"], 2);
        assert_eq!(history[1].source_lenses, ["cost", "trend"]);
        assert_eq!(sender.projection_history("cost_meter"), history);
    }

    #[test]
    fn projection_history_survives_restart_and_versions_continue() {
        let tmpdir = tempfile::tempdir().expect("tempdir");
        let event_log_path = tmpdir.path().join("events.jsonl");
        {
            let hub = StateHub::with_event_log(16, &event_log_path);
            hub.update_projection("cost_meter", serde_json::json!({"total_usd": 1}), "cost");
            let sender = hub.sender();
            sender.update_projection("cost_meter", serde_json::json!({"total_usd": 2}), "trend");
            hub.update_projection(
                "cohort_health",
                serde_json::json!({"agent_count": 4}),
                "cohort",
            );
        }

        let restarted = StateHub::with_event_log(16, &event_log_path);
        assert_eq!(restarted.projection_version("cost_meter"), Some(2));
        assert_eq!(restarted.projection_version("cohort_health"), Some(1));
        let restored = restarted.projection_history("cost_meter");
        assert_eq!(
            restored
                .iter()
                .map(|projection| projection.version)
                .collect::<Vec<_>>(),
            [1, 2]
        );
        assert_eq!(restored[1].data["total_usd"], 2);
        assert_eq!(restored[1].source_lenses, ["cost", "trend"]);

        restarted.update_projection("cost_meter", serde_json::json!({"total_usd": 3}), "cost");
        assert_eq!(restarted.projection_version("cost_meter"), Some(3));
        assert_eq!(
            restarted
                .projection_history("cost_meter")
                .iter()
                .map(|projection| projection.version)
                .collect::<Vec<_>>(),
            [1, 2, 3]
        );

        let history_path = tmpdir.path().join("projection-history.jsonl");
        let durable_lines = std::fs::read_to_string(history_path)
            .expect("read durable projection history")
            .lines()
            .filter(|line| !line.trim().is_empty())
            .count();
        assert_eq!(durable_lines, 4);
        assert!(
            std::fs::read_to_string(event_log_path)
                .expect("read dashboard event log")
                .trim()
                .is_empty(),
            "projection invalidations must remain out of the dashboard replay log"
        );
    }

    #[test]
    fn projection_history_restore_ignores_malformed_and_truncated_records() {
        let tmpdir = tempfile::tempdir().expect("tempdir");
        let event_log_path = tmpdir.path().join("events.jsonl");
        let history_path = tmpdir.path().join("projection-history.jsonl");
        let projection = |version, value| ProjectionState {
            id: "cost_meter".to_string(),
            version,
            data: serde_json::json!({"total_usd": value}),
            updated_at: (Utc::now() + chrono::Duration::seconds(version as i64)).to_rfc3339(),
            source_lenses: vec!["cost".to_string()],
        };
        let newest = serde_json::to_string(&projection(3, 3)).unwrap();
        let oldest = serde_json::to_string(&projection(1, 1)).unwrap();
        let truncated = serde_json::to_string(&projection(9, 9)).unwrap();
        std::fs::write(
            &history_path,
            format!("{newest}\nnot-json\n{oldest}\n{truncated}"),
        )
        .expect("seed projection history");

        let hub = StateHub::with_event_log(16, &event_log_path);
        assert_eq!(hub.projection_version("cost_meter"), Some(3));
        assert_eq!(
            hub.projection_history("cost_meter")
                .iter()
                .map(|projection| projection.version)
                .collect::<Vec<_>>(),
            [1, 3]
        );
        hub.update_projection("cost_meter", serde_json::json!({"total_usd": 4}), "cost");
        assert_eq!(hub.projection_version("cost_meter"), Some(4));
    }

    #[test]
    fn projection_history_retention_prunes_old_versions_but_preserves_current_version() {
        let tmpdir = tempfile::tempdir().expect("tempdir");
        let event_log_path = tmpdir.path().join("events.jsonl");
        let history_path = tmpdir.path().join("projection-history.jsonl");
        let old = ProjectionState {
            id: "cost_meter".to_string(),
            version: 7,
            data: serde_json::json!({"total_usd": 7}),
            updated_at: (Utc::now() - chrono::Duration::days(2)).to_rfc3339(),
            source_lenses: vec!["cost".to_string()],
        };
        std::fs::write(
            &history_path,
            format!("{}\n", serde_json::to_string(&old).unwrap()),
        )
        .expect("seed old projection");

        let hub = StateHub::with_event_log_and_projection_retention(
            16,
            &event_log_path,
            Duration::from_secs(60 * 60),
        );
        assert_eq!(
            hub.projection_history_retention(),
            Duration::from_secs(3600)
        );
        assert_eq!(hub.projection_version("cost_meter"), Some(7));
        assert!(hub.projection_history("cost_meter").is_empty());

        hub.update_projection("cost_meter", serde_json::json!({"total_usd": 8}), "trend");
        assert_eq!(hub.projection_version("cost_meter"), Some(8));
        let history = hub.projection_history("cost_meter");
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].version, 8);
        assert_eq!(history[0].source_lenses, ["cost", "trend"]);
    }

    #[test]
    fn projection_history_retention_is_applied_during_live_updates() {
        let hub = StateHub::with_projection_history_policy(16, 16, Duration::from_secs(60));
        hub.update_projection("cost_meter", serde_json::json!({"total_usd": 1}), "cost");
        {
            let mut store = hub
                .projections
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            store.history.get_mut("cost_meter").unwrap()[0].updated_at =
                (Utc::now() - chrono::Duration::minutes(2)).to_rfc3339();
        }
        assert!(hub.projection_history("cost_meter").is_empty());
        assert_eq!(hub.projection_version("cost_meter"), Some(1));

        hub.update_projection("cost_meter", serde_json::json!({"total_usd": 2}), "cost");
        let history = hub.projection_history("cost_meter");
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].version, 2);
    }

    #[test]
    fn concurrent_projection_history_keeps_latest_monotonic_versions() {
        const OPERATIONS: usize = 32;
        const HISTORY_CAPACITY: usize = 8;
        let hub = Arc::new(StateHub::with_projection_history_capacity(
            64,
            HISTORY_CAPACITY,
        ));
        let barrier = Arc::new(std::sync::Barrier::new(OPERATIONS));
        let mut workers = Vec::with_capacity(OPERATIONS);
        for index in 0..OPERATIONS {
            let hub = Arc::clone(&hub);
            let barrier = Arc::clone(&barrier);
            workers.push(std::thread::spawn(move || {
                barrier.wait();
                hub.update_projection(
                    "cohort_health",
                    serde_json::json!({"operation": index}),
                    "concurrency-test",
                );
            }));
        }
        for worker in workers {
            worker.join().unwrap();
        }

        assert_eq!(
            hub.projection_version("cohort_health"),
            Some(OPERATIONS as u64)
        );
        let history = hub.projection_history("cohort_health");
        assert_eq!(history.len(), HISTORY_CAPACITY);
        assert_eq!(
            history
                .iter()
                .map(|projection| projection.version)
                .collect::<Vec<_>>(),
            (OPERATIONS as u64 - HISTORY_CAPACITY as u64 + 1..=OPERATIONS as u64)
                .collect::<Vec<_>>()
        );
    }

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
    async fn projection_updates_commit_before_broadcasting_invalidation() {
        let hub = StateHub::default_capacity();
        let mut event_rx = hub.subscribe_events();

        hub.update_projection("cost_meter", serde_json::json!({"total_usd": 1.5}), "cost");

        let envelope = event_rx.recv().await.unwrap();
        assert!(matches!(
            envelope.payload,
            DashboardEvent::ProjectionUpdated {
                ref projection_id,
                version: 1,
                ref source_lens,
            } if projection_id == "cost_meter" && source_lens == "cost"
        ));
        assert_eq!(hub.get_projection("cost_meter").unwrap().version, 1);
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
            plan_id: String::new(),
            task_id: String::new(),
            attempt: 0,
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
    fn bootstrap_prefers_verified_snapshot_and_fails_closed_on_corruption() {
        let tmpdir = tempfile::tempdir().expect("tempdir");
        let state_dir = tmpdir.path().join(".roko/state");
        std::fs::create_dir_all(&state_dir).expect("state dir");
        std::fs::write(
            state_dir.join("executor.json"),
            serde_json::json!({
                "plan_states": {"legacy": {"current_phase": {"kind": "implementing"}}}
            })
            .to_string(),
        )
        .expect("legacy executor");
        let mut unified = unified_fixture("canonical");
        std::fs::write(
            state_dir.join("state-snapshot.json"),
            serde_json::to_vec(&unified).unwrap(),
        )
        .expect("unified snapshot");

        let hub = SharedStateHub::new_in_process();
        hub.bootstrap_from_workdir(tmpdir.path()).unwrap();
        assert!(hub.current_snapshot().plans.contains_key("canonical"));
        assert!(!hub.current_snapshot().plans.contains_key("legacy"));
        let materialized = hub.current_snapshot_with_provenance();
        assert!(materialized.provenance.recovered);
        assert_eq!(
            materialized.provenance.source_status.as_deref(),
            Some("state_snapshot")
        );
        assert!(materialized.provenance.runner.is_some());

        unified.checksum = "0".repeat(64);
        std::fs::write(
            state_dir.join("state-snapshot.json"),
            serde_json::to_vec(&unified).unwrap(),
        )
        .expect("corrupt unified snapshot");
        let corrupt_hub = SharedStateHub::new_in_process();
        assert!(corrupt_hub.bootstrap_from_workdir(tmpdir.path()).is_err());
        assert!(corrupt_hub.current_snapshot().plans.is_empty());
    }

    #[test]
    fn canonical_baseline_does_not_replay_already_reflected_event_log() {
        let tmpdir = tempfile::tempdir().expect("tempdir");
        let state_dir = tmpdir.path().join(".roko/state");
        std::fs::create_dir_all(&state_dir).unwrap();
        std::fs::write(
            state_dir.join("state-snapshot.json"),
            serde_json::to_vec(&unified_fixture("canonical")).unwrap(),
        )
        .unwrap();
        let event_path = tmpdir.path().join(".roko/events.jsonl");
        std::fs::write(
            &event_path,
            format!(
                "{}\n",
                serde_json::to_string(&DashboardEvent::PlanStarted {
                    plan_id: "canonical".to_string(),
                })
                .unwrap()
            ),
        )
        .unwrap();

        let hub = SharedStateHub::new_in_process();
        hub.bootstrap_from_workdir(tmpdir.path()).unwrap();
        let before = hub.cursor_snapshot();
        assert_eq!(before.snapshot.stats.plans_active, 1);
        assert_eq!(hub.replay_log_into_snapshot(&event_path), 0);
        let after = hub.cursor_snapshot();
        assert_eq!(after.snapshot.stats.plans_active, 1);
        assert_eq!(after.provenance.revision, before.provenance.revision);
    }

    #[test]
    fn event_log_persists_and_replays() {
        let tmpdir = tempfile::tempdir().expect("tempdir");
        let log_path = tmpdir.path().join("events.jsonl");

        // Create hub with event log and publish events.
        let hub = StateHub::with_event_log(16, &log_path);
        hub.publish(DashboardEvent::AgentSpawned {
            agent_id: "a1".into(),
            plan_id: String::new(),
            task_id: String::new(),
            attempt: 0,
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
            plan_id: String::new(),
            task_id: String::new(),
            attempt: 0,
            role: "auditor".into(),
            model: String::new(),
        });

        let content = std::fs::read_to_string(&log_path).expect("read event log");
        assert!(
            content.contains("s1"),
            "sender should persist to event log: {content}"
        );
    }

    #[test]
    fn feed_tick_not_persisted_but_broadcast() {
        let tmpdir = tempfile::tempdir().expect("tempdir");
        let log_path = tmpdir.path().join("events.jsonl");

        let hub = StateHub::with_event_log(16, &log_path);
        let mut event_rx = hub.subscribe_events();

        // Publish a FeedTick -- should broadcast but NOT persist.
        hub.publish(DashboardEvent::FeedTick {
            agent_id: "feed-1".into(),
            feed_id: "f1".into(),
            topic: "price".into(),
            payload: serde_json::json!({"usd": 42}),
            timestamp_ms: 1000,
        });

        // Broadcast still receives the event.
        let envelope = event_rx.try_recv().expect("feed tick should be broadcast");
        assert!(matches!(envelope.payload, DashboardEvent::FeedTick { .. }));

        // File should be empty (no persisted lines).
        let content = std::fs::read_to_string(&log_path).unwrap_or_default();
        let lines: Vec<&str> = content.lines().filter(|l| !l.is_empty()).collect();
        assert!(
            lines.is_empty(),
            "FeedTick should not be persisted, but found: {content}"
        );
    }

    #[test]
    fn chain_block_not_persisted_but_broadcast() {
        let tmpdir = tempfile::tempdir().expect("tempdir");
        let log_path = tmpdir.path().join("events.jsonl");

        let hub = StateHub::with_event_log(16, &log_path);
        let mut event_rx = hub.subscribe_events();

        // Publish a ChainBlock -- should broadcast but NOT persist.
        hub.publish(DashboardEvent::ChainBlock {
            number: 12345,
            hash: "0xabc".into(),
            parent_hash: "0xdef".into(),
            timestamp: 1700000000,
            gas_used: 21000,
            gas_limit: 30000000,
            tx_count: 5,
            base_fee_per_gas: Some(1000000000),
        });

        // Broadcast still receives the event.
        let envelope = event_rx
            .try_recv()
            .expect("chain block should be broadcast");
        assert!(matches!(
            envelope.payload,
            DashboardEvent::ChainBlock { .. }
        ));

        // File should be empty (no persisted lines).
        let content = std::fs::read_to_string(&log_path).unwrap_or_default();
        let lines: Vec<&str> = content.lines().filter(|l| !l.is_empty()).collect();
        assert!(
            lines.is_empty(),
            "ChainBlock should not be persisted, but found: {content}"
        );
    }

    #[test]
    fn resume_critical_events_still_persisted() {
        let tmpdir = tempfile::tempdir().expect("tempdir");
        let log_path = tmpdir.path().join("events.jsonl");

        let hub = StateHub::with_event_log(16, &log_path);

        // Publish a mix of noisy and resume-critical events.
        hub.publish(DashboardEvent::PlanStarted {
            plan_id: "p1".into(),
        });
        hub.publish(DashboardEvent::FeedTick {
            agent_id: "feed-1".into(),
            feed_id: "f1".into(),
            topic: "price".into(),
            payload: serde_json::json!({}),
            timestamp_ms: 1000,
        });
        hub.publish(DashboardEvent::ChainBlock {
            number: 1,
            hash: "0x1".into(),
            parent_hash: "0x0".into(),
            timestamp: 1700000000,
            gas_used: 0,
            gas_limit: 30000000,
            tx_count: 0,
            base_fee_per_gas: None,
        });
        hub.publish(DashboardEvent::TaskStarted {
            plan_id: "p1".into(),
            task_id: "t1".into(),
            title: "Test task".into(),
            phase: "compose".into(),
        });

        // Only the resume-critical events should be on disk.
        let content = std::fs::read_to_string(&log_path).expect("read event log");
        let lines: Vec<&str> = content.lines().filter(|l| !l.is_empty()).collect();
        assert_eq!(
            lines.len(),
            2,
            "only PlanStarted and TaskStarted should be persisted, got: {content}"
        );
        assert!(content.contains("plan_started"));
        assert!(content.contains("task_started"));
        assert!(!content.contains("feed_tick"));
        assert!(!content.contains("chain_block"));

        // All 4 events should still be in the snapshot.
        let snap = hub.current_snapshot();
        assert_eq!(snap.stats.plans_active, 1);
        assert_eq!(snap.stats.tasks_active, 1);
    }

    #[test]
    fn sender_skips_noise_on_disk() {
        let tmpdir = tempfile::tempdir().expect("tempdir");
        let log_path = tmpdir.path().join("events.jsonl");

        let hub = StateHub::with_event_log(16, &log_path);
        let sender = hub.sender();

        sender.publish(DashboardEvent::FeedTick {
            agent_id: "s-feed".into(),
            feed_id: "f1".into(),
            topic: "rates".into(),
            payload: serde_json::json!({}),
            timestamp_ms: 2000,
        });
        sender.publish(DashboardEvent::PlanStarted {
            plan_id: "p2".into(),
        });

        let content = std::fs::read_to_string(&log_path).expect("read event log");
        let lines: Vec<&str> = content.lines().filter(|l| !l.is_empty()).collect();
        assert_eq!(
            lines.len(),
            1,
            "only PlanStarted should be persisted via sender, got: {content}"
        );
        assert!(content.contains("plan_started"));
        assert!(!content.contains("feed_tick"));
    }

    #[test]
    fn batch_publish_skips_noise_on_disk() {
        let tmpdir = tempfile::tempdir().expect("tempdir");
        let log_path = tmpdir.path().join("events.jsonl");

        let hub = StateHub::with_event_log(16, &log_path);
        hub.publish_batch(vec![
            DashboardEvent::PlanStarted {
                plan_id: "p1".into(),
            },
            DashboardEvent::FeedTick {
                agent_id: "feed-1".into(),
                feed_id: "f1".into(),
                topic: "price".into(),
                payload: serde_json::json!({}),
                timestamp_ms: 1000,
            },
            DashboardEvent::ChainBlock {
                number: 1,
                hash: "0x1".into(),
                parent_hash: "0x0".into(),
                timestamp: 1700000000,
                gas_used: 0,
                gas_limit: 30000000,
                tx_count: 0,
                base_fee_per_gas: None,
            },
            DashboardEvent::TaskStarted {
                plan_id: "p1".into(),
                task_id: "t1".into(),
                title: String::new(),
                phase: "compose".into(),
            },
        ]);

        let content = std::fs::read_to_string(&log_path).expect("read event log");
        let lines: Vec<&str> = content.lines().filter(|l| !l.is_empty()).collect();
        assert_eq!(
            lines.len(),
            2,
            "batch should persist only PlanStarted and TaskStarted, got: {content}"
        );
        assert!(!content.contains("feed_tick"));
        assert!(!content.contains("chain_block"));

        // All 4 events should still reach the snapshot.
        let snap = hub.current_snapshot();
        assert_eq!(snap.stats.plans_active, 1);
        assert_eq!(snap.stats.tasks_active, 1);
    }

    #[test]
    fn event_log_compacts_when_limit_exceeded() {
        let tmpdir = tempfile::tempdir().expect("tempdir");
        let log_path = tmpdir.path().join("events.jsonl");

        // Pre-populate the log with EVENT_LOG_MAX_LINES entries so the next
        // append triggers compaction.
        {
            let mut content = String::new();
            for i in 0..EVENT_LOG_MAX_LINES {
                let line = serde_json::to_string(&DashboardEvent::PlanStarted {
                    plan_id: format!("pre-{i}"),
                })
                .unwrap();
                content.push_str(&line);
                content.push('\n');
            }
            std::fs::write(&log_path, &content).expect("pre-populate");
        }

        // Open the hub (counts existing lines) and publish one more to exceed
        // the cap.
        let hub = StateHub::with_event_log(16, &log_path);
        hub.publish(DashboardEvent::PlanStarted {
            plan_id: "trigger".into(),
        });

        // After compaction the file should contain EVENT_LOG_MAX_LINES / 2
        // lines, all complete JSON.
        let content = std::fs::read_to_string(&log_path).expect("read after compact");
        let lines: Vec<&str> = content.lines().filter(|l| !l.is_empty()).collect();
        assert_eq!(
            lines.len(),
            EVENT_LOG_MAX_LINES / 2,
            "compaction should keep EVENT_LOG_MAX_LINES / 2 lines, got {}",
            lines.len(),
        );

        // Every retained line must be valid JSON.
        for (i, line) in lines.iter().enumerate() {
            assert!(
                serde_json::from_str::<DashboardEvent>(line).is_ok(),
                "line {i} is not valid DashboardEvent JSON: {line}"
            );
        }

        // The most recent event ("trigger") must be the last line.
        let last: DashboardEvent = serde_json::from_str(lines.last().unwrap()).unwrap();
        assert!(
            matches!(last, DashboardEvent::PlanStarted { ref plan_id } if plan_id == "trigger"),
            "last line should be the triggering event"
        );
    }

    #[test]
    fn event_log_replay_after_compaction() {
        let tmpdir = tempfile::tempdir().expect("tempdir");
        let log_path = tmpdir.path().join("events.jsonl");

        // Pre-populate past the limit so compaction fires on the next append.
        {
            let mut content = String::new();
            for i in 0..EVENT_LOG_MAX_LINES {
                let line = serde_json::to_string(&DashboardEvent::PlanStarted {
                    plan_id: format!("old-{i}"),
                })
                .unwrap();
                content.push_str(&line);
                content.push('\n');
            }
            std::fs::write(&log_path, &content).expect("pre-populate");
        }

        let hub = StateHub::with_event_log(16, &log_path);
        // Publish a task event that should survive compaction (it's the newest).
        hub.publish(DashboardEvent::TaskStarted {
            plan_id: "recent-plan".into(),
            task_id: "recent-task".into(),
            title: "Important".into(),
            phase: "compose".into(),
        });

        // Replay from the compacted log into a fresh hub.
        let (replayed, count) = StateHub::replay_from_log(&log_path);
        assert_eq!(count, EVENT_LOG_MAX_LINES / 2);
        let snap = replayed.current_snapshot();
        // The newest task event must be present after replay.
        assert!(
            snap.tasks.contains_key("recent-plan/recent-task"),
            "task from post-compaction log must be replayable"
        );
    }

    #[test]
    fn event_log_compaction_preserves_complete_lines() {
        let tmpdir = tempfile::tempdir().expect("tempdir");
        let log_path = tmpdir.path().join("events.jsonl");

        // Write exactly EVENT_LOG_MAX_LINES + 5 lines via the hub.
        let hub = StateHub::with_event_log(16, &log_path);
        for i in 0..=EVENT_LOG_MAX_LINES + 4 {
            hub.publish(DashboardEvent::PlanStarted {
                plan_id: format!("p-{i}"),
            });
        }

        // File should be compacted (not the original full size).
        let content = std::fs::read_to_string(&log_path).expect("read");
        let lines: Vec<&str> = content.lines().filter(|l| !l.is_empty()).collect();
        assert!(
            lines.len() <= EVENT_LOG_MAX_LINES,
            "log should be bounded after compaction, got {} lines",
            lines.len(),
        );

        // No partial JSON allowed.
        for (i, line) in lines.iter().enumerate() {
            assert!(
                serde_json::from_str::<DashboardEvent>(line).is_ok(),
                "partial JSON at line {i}: {line}"
            );
        }
    }

    /// Write more events than `max_lines`, then trigger compaction manually
    /// and verify that (a) the file is bounded and (b) every retained line is
    /// valid JSON that replays cleanly.
    #[test]
    fn event_log_caps_and_compacts_at_line_limit() {
        let tmpdir = tempfile::tempdir().expect("tempdir");
        let log_path = tmpdir.path().join("events.jsonl");

        // Use a tiny cap (10 lines) and bypass the check-interval by calling
        // compact_event_log directly through the internal lock after writing.
        let max_lines: usize = 10;
        let hub = StateHub::with_event_log_capped(16, &log_path, max_lines);

        // Publish 25 plan-started events.
        for i in 0..25usize {
            hub.publish(DashboardEvent::PlanStarted {
                plan_id: format!("plan-{i}"),
            });
        }

        // Force compaction by directly calling the writer's method.
        if let Some(log) = &hub.event_log {
            if let Ok(mut w) = log.lock() {
                w.compact_event_log();
            }
        }

        // Automatic compaction uses half-cap hysteresis; the file must remain
        // bounded and retain the newest records.
        let content = std::fs::read_to_string(&log_path).expect("read event log");
        let lines: Vec<&str> = content.lines().filter(|l| !l.trim().is_empty()).collect();
        assert!(
            lines.len() <= max_lines,
            "expected at most {max_lines} lines after compaction, got {}",
            lines.len(),
        );

        // Every retained line must be valid JSON (no partial records).
        for (idx, line) in lines.iter().enumerate() {
            assert!(
                serde_json::from_str::<DashboardEvent>(line).is_ok(),
                "line {idx} is not valid DashboardEvent JSON: {line}"
            );
        }

        // The newest event must be retained.
        let retained_ids: Vec<String> = lines
            .iter()
            .filter_map(|l| {
                serde_json::from_str::<DashboardEvent>(l)
                    .ok()
                    .and_then(|e| {
                        if let DashboardEvent::PlanStarted { plan_id } = e {
                            Some(plan_id)
                        } else {
                            None
                        }
                    })
            })
            .collect();
        assert!(retained_ids.contains(&"plan-24".to_string()));
    }

    /// After compaction the log must replay into a correct dashboard snapshot.
    #[test]
    fn event_log_capped_replays_correctly() {
        let tmpdir = tempfile::tempdir().expect("tempdir");
        let log_path = tmpdir.path().join("events.jsonl");

        let max_lines: usize = 5;
        let hub = StateHub::with_event_log_capped(16, &log_path, max_lines);

        // Publish an agent-spawned followed by several plan-started events.
        hub.publish(DashboardEvent::AgentSpawned {
            agent_id: "survivor".into(),
            plan_id: String::new(),
            task_id: String::new(),
            attempt: 0,
            role: "coder".into(),
            model: String::new(),
        });
        for i in 0..20usize {
            hub.publish(DashboardEvent::PlanStarted {
                plan_id: format!("p{i}"),
            });
        }

        // Force compaction.
        if let Some(log) = &hub.event_log {
            if let Ok(mut w) = log.lock() {
                w.compact_event_log();
            }
        }

        // File must be bounded.
        let content = std::fs::read_to_string(&log_path).expect("read event log");
        let line_count = content.lines().filter(|l| !l.trim().is_empty()).count();
        assert!(line_count <= max_lines);

        // Replay must produce a valid snapshot without panicking.
        let (replayed, count) = StateHub::replay_from_log(&log_path);
        assert_eq!(
            count, line_count,
            "replay should ingest every retained event"
        );
        let snap = replayed.current_snapshot();
        // At least some plans should be present (the 5 newest plan-started events).
        assert!(
            snap.stats.plans_active > 0,
            "snapshot must have active plans"
        );
    }
}
