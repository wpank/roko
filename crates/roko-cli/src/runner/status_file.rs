//! Lightweight `.roko/state/status.json` writer and reader.
//!
//! Emits a tiny (< 500 byte) file on every runner tick so external tools
//! can check workspace progress without parsing the full snapshot.  Writes
//! are debounced to at most once per second.
//!
//! The reader path (`read_runner_status`) provides staleness detection:
//! if `status.json` is older than 60 seconds and the writer PID is dead,
//! the file is treated as stale.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

/// Debounce interval in milliseconds.
const DEBOUNCE_MS: u64 = 1_000;

/// A status file older than this (in milliseconds) is considered potentially
/// stale and requires PID liveness to be trusted.
const STALENESS_THRESHOLD_MS: u64 = 60_000;

/// Last successful write timestamp, in epoch milliseconds.
static LAST_WRITE_MS: AtomicU64 = AtomicU64::new(0);

/// The lightweight status payload written to `status.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunnerStatusFile {
    pub run_id: String,
    pub phase: String,
    /// Finer-grained phase label (e.g. "dispatch", "gate", "merge").
    #[serde(default)]
    pub current_phase: String,
    pub active_plans: usize,
    pub completed_plans: usize,
    pub total_plans: usize,
    pub active_agents: usize,
    pub elapsed_secs: u64,
    pub last_event: String,
    /// PID of the runner process that wrote this file.
    #[serde(default)]
    pub pid: u32,
    /// Unix epoch milliseconds when this file was last written.
    #[serde(default)]
    pub updated_at_ms: u64,
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

/// Write `status.json` immediately, bypassing the periodic-write debounce.
///
/// Terminal lifecycle projections must use this path: the final snapshot can
/// be produced less than one second after the preceding gate snapshot, and a
/// skipped terminal write leaves external readers believing the run is still
/// active.
pub fn write_status_immediate(state_dir: &Path, status: &RunnerStatusFile) {
    let Ok(payload) = serde_json::to_string(status) else {
        return;
    };

    let path = status_file_path(state_dir);
    if let Err(e) = atomic_write(&path, payload.as_bytes()) {
        tracing::trace!(error = %e, "failed to write terminal status.json");
        return;
    }

    let now_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
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

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// Result of reading `status.json` with staleness detection.
#[derive(Debug, Clone)]
pub enum RunnerStatusRead {
    /// The file was read and the writing process appears live.
    Live(RunnerStatusFile),
    /// The file was read but the writing process is dead or the file is
    /// older than 60 seconds with a dead PID.
    Stale(RunnerStatusFile),
    /// The file does not exist or could not be parsed.
    Missing,
}

impl RunnerStatusRead {
    /// Returns the status payload regardless of liveness.
    #[must_use]
    pub fn status(&self) -> Option<&RunnerStatusFile> {
        match self {
            Self::Live(s) | Self::Stale(s) => Some(s),
            Self::Missing => None,
        }
    }

    /// Whether the runner that wrote this file is still alive.
    #[must_use]
    pub fn is_live(&self) -> bool {
        matches!(self, Self::Live(_))
    }
}

/// Read `status.json` from `state_dir` and check staleness.
///
/// A status file is considered live when:
/// - `pid` is nonzero and the process is alive, OR
/// - `updated_at_ms` is within the staleness threshold (60s)
///
/// Legacy files without `pid`/`updated_at_ms` fields fall back to the
/// file modification time for the age check.
pub fn read_runner_status(state_dir: &Path) -> RunnerStatusRead {
    let path = status_file_path(state_dir);
    let bytes = match std::fs::read(&path) {
        Ok(b) => b,
        Err(_) => return RunnerStatusRead::Missing,
    };
    let status: RunnerStatusFile = match serde_json::from_slice(&bytes) {
        Ok(s) => s,
        Err(_) => return RunnerStatusRead::Missing,
    };

    let current_ms = now_ms();

    // Determine age from the embedded timestamp, falling back to file mtime.
    let age_ms = if status.updated_at_ms > 0 {
        current_ms.saturating_sub(status.updated_at_ms)
    } else {
        std::fs::metadata(&path)
            .and_then(|m| m.modified())
            .ok()
            .and_then(|mtime| mtime.elapsed().ok())
            .map(|elapsed| elapsed.as_millis() as u64)
            .unwrap_or(u64::MAX)
    };

    // If PID is available, check liveness directly.
    if status.pid > 0 {
        if process_is_alive(status.pid) {
            return RunnerStatusRead::Live(status);
        }
        return RunnerStatusRead::Stale(status);
    }

    // Legacy file without PID: trust the file if recent.
    if age_ms <= STALENESS_THRESHOLD_MS {
        RunnerStatusRead::Live(status)
    } else {
        RunnerStatusRead::Stale(status)
    }
}

/// Check if a process with the given PID is alive.
#[allow(unsafe_code)]
fn process_is_alive(pid: u32) -> bool {
    // Use kill(0) to check existence without sending a signal.
    // This avoids the overhead of sysinfo for a single PID check.
    #[cfg(unix)]
    {
        // SAFETY: kill(pid, 0) is a well-defined POSIX operation that checks
        // process existence without sending a signal. No memory safety concern.
        unsafe { libc::kill(pid as libc::pid_t, 0) == 0 }
    }
    #[cfg(not(unix))]
    {
        let _ = pid;
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_status() -> RunnerStatusFile {
        RunnerStatusFile {
            run_id: "run-1".to_string(),
            phase: "gate".to_string(),
            current_phase: "gate".to_string(),
            active_plans: 1,
            completed_plans: 0,
            total_plans: 1,
            active_agents: 0,
            elapsed_secs: 4,
            last_event: "task:plan-verify".to_string(),
            pid: std::process::id(),
            updated_at_ms: now_ms(),
        }
    }

    #[test]
    fn immediate_write_replaces_a_nonterminal_status() {
        let dir = tempfile::tempdir().expect("temporary status directory");
        let mut status = test_status();
        write_status_immediate(dir.path(), &status);

        status.phase = "completed".to_string();
        status.current_phase = "completed".to_string();
        status.active_plans = 0;
        status.completed_plans = 1;
        status.last_event = "run.completed".to_string();
        write_status_immediate(dir.path(), &status);

        let persisted: serde_json::Value = serde_json::from_slice(
            &std::fs::read(status_file_path(dir.path())).expect("read terminal status"),
        )
        .expect("parse terminal status");
        assert_eq!(persisted["phase"], "completed");
        assert_eq!(persisted["active_plans"], 0);
        assert_eq!(persisted["completed_plans"], 1);
        assert_eq!(persisted["last_event"], "run.completed");
    }

    #[test]
    fn read_runner_status_live_with_current_pid() {
        let dir = tempfile::tempdir().unwrap();
        let status = test_status();
        write_status_immediate(dir.path(), &status);

        let result = read_runner_status(dir.path());
        assert!(result.is_live());
        let read = result.status().unwrap();
        assert_eq!(read.run_id, "run-1");
        assert_eq!(read.pid, std::process::id());
    }

    #[test]
    fn read_runner_status_stale_with_dead_pid() {
        let dir = tempfile::tempdir().unwrap();
        let mut status = test_status();
        // Use an implausible PID that is almost certainly dead.
        status.pid = 4_000_000;
        write_status_immediate(dir.path(), &status);

        let result = read_runner_status(dir.path());
        assert!(!result.is_live());
        assert!(result.status().is_some());
    }

    #[test]
    fn read_runner_status_missing_returns_missing() {
        let dir = tempfile::tempdir().unwrap();
        let result = read_runner_status(dir.path());
        assert!(matches!(result, RunnerStatusRead::Missing));
    }

    #[test]
    fn deserialize_legacy_status_without_new_fields() {
        let legacy = r#"{"run_id":"r","phase":"idle","active_plans":0,"completed_plans":0,"total_plans":0,"active_agents":0,"elapsed_secs":0,"last_event":"none"}"#;
        let status: RunnerStatusFile = serde_json::from_str(legacy).unwrap();
        assert_eq!(status.pid, 0);
        assert_eq!(status.updated_at_ms, 0);
        assert!(status.current_phase.is_empty());
    }
}
