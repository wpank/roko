//! Lightweight `.roko/state/status.json` writer.
//!
//! Emits a tiny (< 500 byte) file on every runner tick so external tools
//! can check workspace progress without parsing the full snapshot.  Writes
//! are debounced to at most once per second.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::Serialize;

/// Debounce interval in milliseconds.
const DEBOUNCE_MS: u64 = 1_000;

/// Last successful write timestamp, in epoch milliseconds.
static LAST_WRITE_MS: AtomicU64 = AtomicU64::new(0);

/// The lightweight status payload written to `status.json`.
#[derive(Debug, Clone, Serialize)]
pub struct RunnerStatusFile {
    pub run_id: String,
    pub phase: String,
    pub active_plans: usize,
    pub completed_plans: usize,
    pub total_plans: usize,
    pub active_agents: usize,
    pub elapsed_secs: u64,
    pub last_event: String,
}

/// Attempt to write `status.json` into the given `state_dir`.
///
/// Silently returns without writing if the debounce interval has not elapsed
/// since the last successful write.
pub fn write_status_debounced(state_dir: &Path, status: &RunnerStatusFile) {
    let now_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);

    let prev = LAST_WRITE_MS.load(Ordering::Relaxed);
    if now_ms.saturating_sub(prev) < DEBOUNCE_MS {
        return;
    }

    let Ok(payload) = serde_json::to_string(status) else {
        return;
    };

    let path = status_file_path(state_dir);
    if let Err(e) = atomic_write(&path, payload.as_bytes()) {
        tracing::trace!(error = %e, "failed to write status.json");
        return;
    }

    LAST_WRITE_MS.store(now_ms, Ordering::Relaxed);
}

/// Canonical path for the lightweight status file.
pub fn status_file_path(state_dir: &Path) -> PathBuf {
    state_dir.join("status.json")
}

/// Write `data` to a temporary file then rename, avoiding partial reads.
fn atomic_write(path: &Path, data: &[u8]) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, data)?;
    std::fs::rename(&tmp, path)?;
    Ok(())
}
