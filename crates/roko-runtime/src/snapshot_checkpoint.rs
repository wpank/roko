//! Periodic durable checkpoint for the [`StateHub`] materialized snapshot.
//!
//! This module provides a background writer that snapshots the in-memory
//! [`DashboardSnapshot`] to `.roko/state/statehub-snapshot.json` every
//! [`CHECKPOINT_INTERVAL`] seconds. On startup, [`load_checkpoint`] restores
//! the last persisted snapshot so StateHub does not need to replay the full
//! event log to recover dashboard state.
//!
//! # Design
//!
//! The checkpoint is a plain JSON file, not SQLite. The file is replaced
//! atomically (write to `.roko/state/.tmp-statehub-snapshot.json`, then
//! `rename`) so a mid-write crash leaves the previous checkpoint intact.
//!
//! # Migration path
//!
//! Once all dashboard consumers read exclusively from StateHub (Phase 5
//! complete), this checkpoint becomes the sole cold-start source of truth
//! and the legacy JSONL event log replay path can be removed.

use std::path::{Path, PathBuf};
use std::time::Duration;

use roko_core::dashboard_snapshot::DashboardSnapshot;

use crate::state_hub::SharedStateHub;

/// Checkpoint file path relative to the workspace root.
pub const STATEHUB_CHECKPOINT_RELATIVE_PATH: &str = ".roko/state/statehub-snapshot.json";

/// How often the background writer flushes the snapshot to disk.
pub const CHECKPOINT_INTERVAL: Duration = Duration::from_secs(5);

/// Schema version stored in the checkpoint envelope. Bump if the envelope
/// structure changes in a breaking way (snapshot itself is versioned by
/// `DashboardSnapshot`).
const CHECKPOINT_VERSION: u32 = 1;

/// Envelope wrapping the persisted snapshot with metadata for integrity checks.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct CheckpointEnvelope {
    /// Envelope schema version.
    version: u32,
    /// Wall-clock timestamp (seconds since Unix epoch) when the checkpoint was written.
    timestamp_s: u64,
    /// Monotonic StateHub provenance revision at write time.
    revision: u64,
    /// The materialized dashboard snapshot.
    snapshot: DashboardSnapshot,
}

/// Write `snapshot` atomically to `path`.
///
/// Writes to a sibling `.tmp-*` file then renames, so a mid-write crash
/// leaves the previous checkpoint intact.
fn write_checkpoint_atomic(path: &Path, envelope: &CheckpointEnvelope) -> std::io::Result<()> {
    let dir = path.parent().unwrap_or(Path::new("."));
    std::fs::create_dir_all(dir)?;

    let tmp_path = dir.join(format!(
        ".tmp-{}",
        path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("statehub-snapshot.json")
    ));

    let json = serde_json::to_string(envelope).map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("checkpoint serialization failed: {e}"),
        )
    })?;

    std::fs::write(&tmp_path, &json)?;
    std::fs::rename(&tmp_path, path)?;
    Ok(())
}

/// Load and decode a checkpoint from `path`.
///
/// Returns `None` when the file is absent, unreadable, or structurally
/// invalid. Callers should treat `None` as "start from empty snapshot"
/// rather than a fatal error.
pub fn load_checkpoint(path: &Path) -> Option<DashboardSnapshot> {
    let raw = std::fs::read_to_string(path).ok()?;
    let envelope: CheckpointEnvelope = serde_json::from_str(&raw).ok()?;
    if envelope.version != CHECKPOINT_VERSION {
        tracing::warn!(
            stored = envelope.version,
            expected = CHECKPOINT_VERSION,
            path = %path.display(),
            "statehub checkpoint version mismatch; ignoring stale checkpoint"
        );
        return None;
    }
    Some(envelope.snapshot)
}

/// Capture the current snapshot from `hub` and flush it to `path`.
///
/// Best-effort: any I/O or serialization error is logged at `debug` level
/// and silently swallowed so the background writer cannot crash the process.
pub fn flush_checkpoint(hub: &SharedStateHub, path: &Path) {
    let materialized = hub.current_snapshot_with_provenance();
    let envelope = CheckpointEnvelope {
        version: CHECKPOINT_VERSION,
        timestamp_s: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
        revision: materialized.provenance.revision,
        snapshot: materialized.snapshot,
    };
    if let Err(error) = write_checkpoint_atomic(path, &envelope) {
        tracing::debug!(
            %error,
            path = %path.display(),
            "statehub checkpoint flush failed (best-effort)"
        );
    }
}

/// Spawn a background Tokio task that flushes the StateHub snapshot to
/// `checkpoint_path` every [`CHECKPOINT_INTERVAL`].
///
/// The task holds a weak reference to the hub via the provided `Arc<StateHub>`
/// (through `SharedStateHub`'s `Deref` to `StateHub`). When the last strong
/// reference to the hub is dropped, the task exits on the next interval tick.
///
/// Returns a `tokio::task::JoinHandle` that callers may await for a clean
/// shutdown; dropping the handle detaches the task (it runs until hub drop).
pub fn start_checkpoint_writer(
    hub: SharedStateHub,
    checkpoint_path: PathBuf,
) -> tokio::task::JoinHandle<()> {
    // Flush once immediately so a crash on the first interval still has a
    // baseline checkpoint.
    flush_checkpoint(&hub, &checkpoint_path);

    tokio::task::spawn(async move {
        let mut interval = tokio::time::interval(CHECKPOINT_INTERVAL);
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        // Consume the first (immediate) tick that fires at t=0.
        interval.tick().await;
        loop {
            interval.tick().await;
            flush_checkpoint(&hub, &checkpoint_path);
        }
    })
}

/// Restore a previously persisted checkpoint into an already-bootstrapped hub.
///
/// This is called once at startup _before_ any live events are published:
/// it hydrates the snapshot from the JSON checkpoint so the TUI and REST
/// consumers have immediately-useful state rather than waiting for the event
/// log replay.
///
/// Returns `true` when a checkpoint was successfully loaded and applied.
/// Returns `false` when the file is absent/corrupt or the hub already has
/// live events (in which case the checkpoint is intentionally ignored to
/// avoid overwriting newer in-memory state).
pub fn bootstrap_from_checkpoint(hub: &SharedStateHub, checkpoint_path: &Path) -> bool {
    let Some(snapshot) = load_checkpoint(checkpoint_path) else {
        return false;
    };
    let applied = hub.hydrate_recovered_snapshot(|snap| {
        *snap = snapshot;
    });
    if applied {
        tracing::debug!(
            path = %checkpoint_path.display(),
            "statehub bootstrapped from JSON checkpoint"
        );
    }
    applied
}

/// Handle combining a checkpoint path with a weak `Arc` for the writer task.
///
/// Allows callers to hold the path and optionally abort the writer task
/// without needing to retain the `JoinHandle`.
pub struct CheckpointWriterHandle {
    /// Absolute path where checkpoints are flushed.
    pub path: PathBuf,
    /// Background task handle; dropping this detaches the task.
    pub handle: tokio::task::JoinHandle<()>,
}

impl CheckpointWriterHandle {
    /// Create a new handle by spawning the background writer.
    pub fn spawn(hub: SharedStateHub, checkpoint_path: PathBuf) -> Self {
        let handle = start_checkpoint_writer(hub, checkpoint_path.clone());
        Self {
            path: checkpoint_path,
            handle,
        }
    }

    /// Abort the background writer task.
    pub fn abort(&self) {
        self.handle.abort();
    }
}

impl Drop for CheckpointWriterHandle {
    fn drop(&mut self) {
        // Abort the background task when the handle is dropped so the writer
        // does not outlive its owner.
        self.handle.abort();
    }
}

/// Convenience: checkpoint path relative to a workspace root.
pub fn checkpoint_path_for_workdir(workdir: &Path) -> PathBuf {
    workdir.join(STATEHUB_CHECKPOINT_RELATIVE_PATH)
}

#[cfg(test)]
mod tests {
    use super::*;
    use roko_core::dashboard_snapshot::{DashboardEvent, DashboardSnapshot};
    use tempfile::tempdir;

    #[test]
    fn round_trips_snapshot_through_file() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("statehub-snapshot.json");

        let hub = SharedStateHub::new(crate::StateHub::default_capacity());
        // Insert a plan so the snapshot has non-trivial content.
        hub.publish(DashboardEvent::PlanStarted {
            plan_id: "plan-a".to_string(),
            tasks_total: 3,
        });

        // Flush.
        flush_checkpoint(&hub, &path);
        assert!(path.exists(), "checkpoint file must be created");

        // Load.
        let loaded = load_checkpoint(&path).expect("checkpoint must load");
        assert!(
            loaded.plans.contains_key("plan-a"),
            "restored snapshot must contain the plan"
        );
        assert_eq!(
            loaded.plans["plan-a"].tasks_total,
            3,
            "plan task count must survive round-trip"
        );
    }

    #[test]
    fn load_returns_none_for_missing_file() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("nonexistent.json");
        assert!(load_checkpoint(&path).is_none());
    }

    #[test]
    fn load_returns_none_for_corrupt_file() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("bad.json");
        std::fs::write(&path, b"NOT JSON").expect("write corrupt file");
        assert!(load_checkpoint(&path).is_none());
    }

    #[test]
    fn load_returns_none_for_version_mismatch() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("v99.json");
        let bad_envelope = serde_json::json!({
            "version": 99u32,
            "timestamp_s": 0u64,
            "revision": 0u64,
            "snapshot": DashboardSnapshot::default()
        });
        std::fs::write(&path, serde_json::to_string(&bad_envelope).unwrap())
            .expect("write bad version file");
        assert!(load_checkpoint(&path).is_none());
    }

    #[test]
    fn bootstrap_applies_snapshot_to_unboostrapped_hub() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("statehub-snapshot.json");

        // Write a checkpoint with a plan in it.
        let hub_src = SharedStateHub::new(crate::StateHub::default_capacity());
        hub_src.publish(DashboardEvent::PlanStarted {
            plan_id: "seeded".to_string(),
            tasks_total: 5,
        });
        flush_checkpoint(&hub_src, &path);

        // Create a fresh hub and bootstrap it.
        let hub_dst = SharedStateHub::new(crate::StateHub::default_capacity());
        // Mark bootstrapped so hydrate_recovered_snapshot works.
        hub_dst.mark_bootstrapped();
        let applied = bootstrap_from_checkpoint(&hub_dst, &path);
        assert!(applied, "bootstrap must succeed");
        let snap = hub_dst.current_snapshot();
        assert!(
            snap.plans.contains_key("seeded"),
            "bootstrapped hub must contain the seeded plan"
        );
    }
}
