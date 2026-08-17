//! Async snapshot writer -- offloads JSON persistence to a dedicated OS thread
//! so `save_snapshot` never blocks the `select!` event loop.

use std::io::{self, Write};
use std::path::PathBuf;
use std::sync::mpsc::{self, SyncSender, TrySendError};
use std::thread::JoinHandle;

use roko_core::defaults::DEFAULT_RUNNER_RETRY_STRATEGY_PIVOT_ATTEMPT;
use roko_runtime::MAX_DURABLE_RUNNER_PROJECTION_BYTES;
use serde::Serialize;
use tracing::{debug, error};

/// Pre-serialized, unified state snapshot ready for a single atomic disk write.
pub struct SnapshotPayload {
    /// The complete JSON blob (outer `StateSnapshot` serialized to bytes).
    pub snapshot_json: Vec<u8>,
    /// Destination path (`.roko/state/state-snapshot.json`).
    pub snapshot_path: PathBuf,
}

struct BoundedJsonBuffer {
    bytes: Vec<u8>,
    limit: usize,
}

impl BoundedJsonBuffer {
    fn new(limit: usize) -> Self {
        Self {
            bytes: Vec::new(),
            limit,
        }
    }
}

impl Write for BoundedJsonBuffer {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if self.bytes.len().saturating_add(bytes.len()) > self.limit {
            return Err(io::Error::new(
                io::ErrorKind::FileTooLarge,
                format!("serialized snapshot exceeds {} byte budget", self.limit),
            ));
        }
        self.bytes.extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// Serialize one inner projection while charging a shared aggregate budget.
pub(super) fn serialize_inner_bounded<T: Serialize>(
    value: &T,
    remaining: &mut usize,
) -> anyhow::Result<String> {
    let mut buffer = BoundedJsonBuffer::new(*remaining);
    serde_json::to_writer_pretty(&mut buffer, value)?;
    *remaining = remaining.saturating_sub(buffer.bytes.len());
    String::from_utf8(buffer.bytes).map_err(Into::into)
}

/// Serialize the outer snapshot without ever allocating beyond the disk cap.
pub(super) fn serialize_snapshot_bounded<T: Serialize>(value: &T) -> anyhow::Result<Vec<u8>> {
    let mut buffer = BoundedJsonBuffer::new(MAX_DURABLE_RUNNER_PROJECTION_BYTES as usize);
    serde_json::to_writer_pretty(&mut buffer, value)?;
    Ok(buffer.bytes)
}

enum WriterMsg {
    Write(SnapshotPayload),
    /// Drain pending writes, then ack via the flush channel.
    Flush,
}

/// Result of draining the channel for pending `Write` messages.
struct DrainResult {
    /// The latest payload found while draining (if any).
    latest: Option<SnapshotPayload>,
    /// Whether a `Flush` message was consumed during the drain and needs acking.
    flush_consumed: bool,
}

/// Bounded-channel writer that persists snapshots on a dedicated OS thread.
///
/// If multiple snapshots queue up, intermediate ones are skipped -- the thread
/// always drains to the *latest* payload before writing.
pub struct SnapshotWriter {
    tx: Option<SyncSender<WriterMsg>>,
    handle: Option<JoinHandle<()>>,
    /// Ack channel used by `flush()` to block until the writer has drained.
    flush_rx: std::sync::mpsc::Receiver<()>,
}

impl SnapshotWriter {
    /// Spawn the writer thread. `capacity` bounds the channel (4 is plenty).
    pub fn new(capacity: usize) -> Self {
        let (tx, rx) = mpsc::sync_channel::<WriterMsg>(capacity);
        let (flush_tx, flush_rx) = mpsc::channel::<()>();
        let handle = std::thread::Builder::new()
            .name("snapshot-writer".into())
            .spawn(move || writer_loop(rx, flush_tx))
            .expect("failed to spawn snapshot-writer thread");

        Self {
            tx: Some(tx),
            handle: Some(handle),
            flush_rx,
        }
    }

    /// Enqueue a snapshot for async write.
    ///
    /// The hot path is non-blocking. Under disk backpressure, this applies
    /// backpressure instead of dropping the newest snapshot; stale intermediate
    /// snapshots are still coalesced by the writer thread.
    pub fn write(&self, payload: SnapshotPayload) -> bool {
        if payload.snapshot_json.len() as u64 > MAX_DURABLE_RUNNER_PROJECTION_BYTES {
            error!(
                bytes = payload.snapshot_json.len(),
                maximum = MAX_DURABLE_RUNNER_PROJECTION_BYTES,
                "refusing oversized unified state snapshot"
            );
            return false;
        }
        let Some(tx) = self.tx.as_ref() else {
            return false;
        };
        match tx.try_send(WriterMsg::Write(payload)) {
            Ok(()) => true,
            Err(TrySendError::Full(WriterMsg::Write(payload))) => {
                debug!("snapshot writer channel full -- waiting to preserve latest snapshot");
                if tx.send(WriterMsg::Write(payload)).is_err() {
                    error!("snapshot writer thread has stopped -- snapshot lost");
                    false
                } else {
                    true
                }
            }
            Err(TrySendError::Full(WriterMsg::Flush)) => {
                debug!("snapshot writer channel full while flushing");
                false
            }
            Err(TrySendError::Disconnected(_)) => {
                error!("snapshot writer thread has stopped -- snapshot lost");
                false
            }
        }
    }

    /// Block until the writer thread has drained all pending payloads.
    pub fn flush(&self) {
        let Some(tx) = self.tx.as_ref() else { return };
        if tx.send(WriterMsg::Flush).is_ok() {
            let _ = self.flush_rx.recv();
        }
    }
}

impl Drop for SnapshotWriter {
    fn drop(&mut self) {
        // Drop the sender to close the channel, then join the writer thread.
        self.tx.take();
        if let Some(handle) = self.handle.take() {
            if let Err(panic_payload) = handle.join() {
                let msg = panic_payload
                    .downcast_ref::<&str>()
                    .copied()
                    .or_else(|| panic_payload.downcast_ref::<String>().map(String::as_str))
                    .unwrap_or("unknown panic");
                error!(
                    panic = %msg,
                    "snapshot-writer thread panicked -- recent snapshots may be lost"
                );
            }
        }
    }
}

/// Writer thread main loop.
fn writer_loop(rx: std::sync::mpsc::Receiver<WriterMsg>, flush_tx: std::sync::mpsc::Sender<()>) {
    let mut fail_streak: u32 = 0;

    loop {
        let msg = match rx.recv() {
            Ok(msg) => msg,
            Err(_) => break, // sender dropped -- exit
        };

        match msg {
            WriterMsg::Write(payload) => {
                let drain = drain_writes(&rx);
                let to_write = drain.latest.unwrap_or(payload);
                write_payload(&to_write, &mut fail_streak);
                if drain.flush_consumed {
                    let _ = flush_tx.send(());
                }
            }
            WriterMsg::Flush => {
                let drain = drain_writes(&rx);
                if let Some(payload) = drain.latest {
                    write_payload(&payload, &mut fail_streak);
                }
                let _ = flush_tx.send(());
                // If another Flush was consumed during drain, ack it too.
                if drain.flush_consumed {
                    let _ = flush_tx.send(());
                }
            }
        }
    }
}

/// Non-blocking drain: consume all queued `Write` messages, return only the
/// latest payload. Stops on empty queue or disconnected channel. If a `Flush`
/// is encountered, it is consumed and flagged so the caller can ack it.
fn drain_writes(rx: &std::sync::mpsc::Receiver<WriterMsg>) -> DrainResult {
    let mut latest: Option<SnapshotPayload> = None;
    let mut flush_consumed = false;
    loop {
        match rx.try_recv() {
            Ok(WriterMsg::Write(payload)) => {
                latest = Some(payload);
            }
            Ok(WriterMsg::Flush) => {
                flush_consumed = true;
                break;
            }
            Err(mpsc::TryRecvError::Empty | mpsc::TryRecvError::Disconnected) => break,
        }
    }
    DrainResult {
        latest,
        flush_consumed,
    }
}

fn write_payload(payload: &SnapshotPayload, fail_streak: &mut u32) {
    if let Err(e) = write_all_files(payload) {
        *fail_streak += 1;
        if *fail_streak >= DEFAULT_RUNNER_RETRY_STRATEGY_PIVOT_ATTEMPT {
            error!(
                error = %e,
                streak = *fail_streak,
                "snapshot persistence degraded"
            );
        } else {
            error!(error = %e, "failed to write snapshot");
        }
    } else {
        *fail_streak = 0;
    }
}

/// Maximum number of rotated checkpoint files to retain per run.
const MAX_CHECKPOINTS: usize = 5;

fn write_all_files(payload: &SnapshotPayload) -> anyhow::Result<()> {
    use super::persist::atomic_write;
    // SH03-T02: Rotate the current snapshot to a timestamped checkpoint
    // before overwriting, so a crash or corruption can fall back to a
    // prior good state.
    if payload.snapshot_path.exists() {
        rotate_checkpoint(&payload.snapshot_path, MAX_CHECKPOINTS);
    }
    atomic_write(&payload.snapshot_path, &payload.snapshot_json)?;
    Ok(())
}

/// Copy the existing snapshot to `<name>.<unix_ms>.bak` and prune old
/// checkpoints so at most `max_keep` remain.
fn rotate_checkpoint(path: &std::path::Path, max_keep: usize) {
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let backup = path.with_file_name(format!(
        "{}.{ts}.bak",
        path.file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("snapshot")
    ));
    if let Err(e) = std::fs::copy(path, &backup) {
        tracing::debug!(error = %e, "failed to create snapshot checkpoint");
        return;
    }

    // Prune oldest checkpoints beyond the retention limit.
    let parent = match path.parent() {
        Some(p) => p,
        None => return,
    };
    let file_name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
    let mut checkpoints: Vec<std::path::PathBuf> = std::fs::read_dir(parent)
        .into_iter()
        .flatten()
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|p| {
            p.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| {
                    name.starts_with(file_name)
                        && name.ends_with(".bak")
                        && name.len() > file_name.len()
                })
        })
        .collect();

    if checkpoints.len() > max_keep {
        // Sort ascending by name (timestamp in extension makes this chronological).
        checkpoints.sort();
        let to_remove = checkpoints.len() - max_keep;
        for old in &checkpoints[..to_remove] {
            let _ = std::fs::remove_file(old);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_and_flush_persists_file() {
        let tmp = tempfile::tempdir().unwrap();
        let writer = SnapshotWriter::new(4);

        let payload = SnapshotPayload {
            snapshot_json: b"unified-snapshot".to_vec(),
            snapshot_path: tmp.path().join("state-snapshot.json"),
        };
        writer.write(payload);
        writer.flush();

        assert_eq!(
            std::fs::read_to_string(tmp.path().join("state-snapshot.json")).unwrap(),
            "unified-snapshot"
        );
    }

    #[test]
    fn latest_snapshot_wins_when_batched() {
        let tmp = tempfile::tempdir().unwrap();
        let writer = SnapshotWriter::new(4);

        for i in 0..3 {
            let label = format!("v{i}");
            writer.write(SnapshotPayload {
                snapshot_json: label.as_bytes().to_vec(),
                snapshot_path: tmp.path().join("state-snapshot.json"),
            });
        }
        writer.flush();

        // Flush drains everything queued before it, so the last payload wins.
        let content = std::fs::read_to_string(tmp.path().join("state-snapshot.json")).unwrap();
        assert_eq!(content, "v2");
    }

    #[test]
    fn bounded_writer_round_trips_through_canonical_projection_loader() {
        let tmp = tempfile::tempdir().unwrap();
        let executor = serde_json::json!({
            "schema_version": 1,
            "plan_states": {},
            "queue_order": [],
            "speculative_executions": {},
            "timestamp_ms": 42
        });
        let run_state = serde_json::json!({
            "schema_version": 1,
            "run_id": "writer-round-trip",
            "started_at_ms": 42,
            "timestamp_ms": 42,
            "tasks_total": 0,
            "tasks_completed": 0,
            "tasks_failed": 0,
            "total_tokens_in": 0,
            "total_tokens_out": 0,
            "total_cost_usd": 0.0,
            "total_agent_calls": 0
        });
        let snapshot = roko_runtime::StateSnapshot::new(
            42,
            executor.to_string(),
            serde_json::json!({
                "schema_version": 1,
                "executor": executor,
                "timestamp_ms": 42
            })
            .to_string(),
            run_state.to_string(),
            serde_json::json!({"rungs": {}}).to_string(),
        );
        let snapshot_json = serialize_snapshot_bounded(&snapshot).unwrap();
        let snapshot_path = tmp.path().join(roko_runtime::STATE_SNAPSHOT_RELATIVE_PATH);
        std::fs::create_dir_all(snapshot_path.parent().unwrap()).unwrap();

        let writer = SnapshotWriter::new(4);
        assert!(writer.write(SnapshotPayload {
            snapshot_json,
            snapshot_path: snapshot_path.clone(),
        }));
        writer.flush();

        let loaded = roko_runtime::load_durable_runner_projection(tmp.path())
            .unwrap()
            .expect("canonical projection");
        assert_eq!(
            loaded
                .snapshot
                .as_ref()
                .expect("unified snapshot")
                .timestamp_ms,
            42
        );
        assert_eq!(
            loaded.run_state.as_ref().expect("embedded run state")["run_id"],
            "writer-round-trip"
        );
        assert_eq!(loaded.source_path, snapshot_path);
    }

    #[test]
    fn drop_joins_thread() {
        let writer = SnapshotWriter::new(4);
        drop(writer);
        // If we get here without hanging, the thread joined successfully.
    }

    #[test]
    fn rotate_checkpoint_creates_backup_and_prunes() {
        let tmp = tempfile::tempdir().unwrap();
        let snapshot_path = tmp.path().join("state-snapshot.json");

        // Write several snapshots to accumulate checkpoints.
        let writer = SnapshotWriter::new(4);
        for i in 0..8 {
            std::fs::write(&snapshot_path, format!("v{i}")).unwrap();
            // Need a small delay so timestamps differ.
            std::thread::sleep(std::time::Duration::from_millis(2));
            writer.write(SnapshotPayload {
                snapshot_json: format!("v{}", i + 1).into_bytes(),
                snapshot_path: snapshot_path.clone(),
            });
            writer.flush();
        }

        // Count .bak files — should be at most MAX_CHECKPOINTS.
        let bak_count = std::fs::read_dir(tmp.path())
            .unwrap()
            .filter_map(Result::ok)
            .filter(|e| {
                e.path()
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .is_some_and(|ext| ext.ends_with("bak"))
            })
            .count();
        assert!(
            bak_count <= super::MAX_CHECKPOINTS,
            "expected at most {} checkpoints, found {}",
            super::MAX_CHECKPOINTS,
            bak_count
        );
        // The latest snapshot should be the last written value.
        let content = std::fs::read_to_string(&snapshot_path).unwrap();
        assert_eq!(content, "v8");
    }

    #[test]
    fn oversized_snapshot_is_rejected_without_replacing_last_generation() {
        let tmp = tempfile::tempdir().unwrap();
        let snapshot_path = tmp.path().join("state-snapshot.json");
        std::fs::write(&snapshot_path, b"last-readable-generation").unwrap();
        let writer = SnapshotWriter::new(1);
        assert!(!writer.write(SnapshotPayload {
            snapshot_json: vec![b'x'; (MAX_DURABLE_RUNNER_PROJECTION_BYTES + 1) as usize],
            snapshot_path: snapshot_path.clone(),
        }));
        writer.flush();
        assert_eq!(
            std::fs::read(&snapshot_path).unwrap(),
            b"last-readable-generation"
        );
        assert_eq!(
            std::fs::read_dir(tmp.path()).unwrap().count(),
            1,
            "rejection must happen before backup rotation"
        );
    }
}
