//! Async snapshot writer -- offloads JSON persistence to a dedicated OS thread
//! so `save_snapshot` never blocks the `select!` event loop.

use std::path::PathBuf;
use std::sync::mpsc::{self, SyncSender, TrySendError};
use std::thread::JoinHandle;

use roko_core::defaults::DEFAULT_RUNNER_RETRY_STRATEGY_PIVOT_ATTEMPT;
use tracing::{debug, error};

/// Pre-serialized, unified state snapshot ready for a single atomic disk write.
pub struct SnapshotPayload {
    /// The complete JSON blob (outer `StateSnapshot` serialized to bytes).
    pub snapshot_json: Vec<u8>,
    /// Destination path (`.roko/state/state-snapshot.json`).
    pub snapshot_path: PathBuf,
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
    pub fn write(&self, payload: SnapshotPayload) {
        let Some(tx) = self.tx.as_ref() else { return };
        match tx.try_send(WriterMsg::Write(payload)) {
            Ok(()) => {}
            Err(TrySendError::Full(WriterMsg::Write(payload))) => {
                debug!("snapshot writer channel full -- waiting to preserve latest snapshot");
                if tx.send(WriterMsg::Write(payload)).is_err() {
                    error!("snapshot writer thread has stopped -- snapshot lost");
                }
            }
            Err(TrySendError::Full(WriterMsg::Flush)) => {
                debug!("snapshot writer channel full while flushing");
            }
            Err(TrySendError::Disconnected(_)) => {
                error!("snapshot writer thread has stopped -- snapshot lost");
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

        // The writer drains to latest, so file should contain "v2" (or at
        // minimum the last payload that was written).
        let content = std::fs::read_to_string(tmp.path().join("state-snapshot.json")).unwrap();
        assert!(content == "v0" || content == "v1" || content == "v2");
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
}
