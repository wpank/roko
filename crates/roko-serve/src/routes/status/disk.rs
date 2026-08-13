//! Disk usage telemetry endpoint (E47-T10).
//!
//! Exposes a `GET /api/status/disk` route that returns current disk usage
//! statistics including mount-point-level free/total/used, plus sizes of
//! roko-specific directories (`.roko/`, `target/`, `.claude/worktrees/`).

use std::path::Path;
use std::sync::Arc;

use axum::Json;
use axum::extract::State;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::error::ApiError;
use crate::state::AppState;

/// Response payload for `GET /api/status/disk`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DiskUsageResponse {
    /// Available (free) space on the mount point, in MB.
    pub free_mb: u64,
    /// Total capacity of the mount point, in MB.
    pub total_mb: u64,
    /// Percentage of total capacity used (0-100).
    pub used_pct: u8,
    /// Size of the `.roko/` directory, in MB.
    pub roko_dir_mb: u64,
    /// Size of the `target/` directory, in MB.
    pub target_dir_mb: u64,
    /// Size of the `.claude/worktrees/` directory, in MB.
    pub worktrees_mb: u64,
    /// Disk pressure status from `DiskMonitor`.
    pub pressure: String,
}

/// `GET /api/status/disk` -- disk usage telemetry.
///
/// Returns mount-point disk usage plus sizes of key project directories.
pub async fn disk_usage(State(state): State<Arc<AppState>>) -> Result<Json<Value>, ApiError> {
    let workdir = state.workdir.clone();
    let response = tokio::task::spawn_blocking(move || compute_disk_usage(&workdir))
        .await
        .map_err(|e| ApiError::internal(format!("disk usage task panicked: {e}")))?
        .map_err(|e| ApiError::internal(format!("disk usage query failed: {e}")))?;

    Ok(Json(serde_json::to_value(&response).unwrap_or_else(
        |_| json!({ "error": "serialization failed" }),
    )))
}

/// Compute disk usage statistics synchronously (runs on a blocking thread).
fn compute_disk_usage(workdir: &Path) -> Result<DiskUsageResponse, std::io::Error> {
    let usage = roko_fs::get_disk_usage(workdir)?;

    let config = roko_core::config::schema::ResourcesConfig::default();
    let monitor = roko_fs::DiskMonitor::new(config.min_free_disk_mb, config.warn_disk_mb);

    let pressure = if let Some(_warning) = monitor.check_warning(workdir) {
        "warning"
    } else if monitor.check_pre_run(workdir).is_err() {
        "critical"
    } else {
        "normal"
    };

    let roko_dir = workdir.join(".roko");
    let target_dir = workdir.join("target");
    let worktrees_dir = workdir.join(".claude").join("worktrees");

    let roko_dir_mb = sync_dir_size_mb(&roko_dir);
    let target_dir_mb = sync_dir_size_mb(&target_dir);
    let worktrees_mb = sync_dir_size_mb(&worktrees_dir);

    Ok(DiskUsageResponse {
        free_mb: usage.available_mb,
        total_mb: usage.total_mb,
        used_pct: usage.percentage,
        roko_dir_mb,
        target_dir_mb,
        worktrees_mb,
        pressure: pressure.to_string(),
    })
}

/// Synchronous directory-size computation in MB, best-effort.
///
/// Returns 0 if the directory does not exist or cannot be read.
fn sync_dir_size_mb(path: &Path) -> u64 {
    if !path.is_dir() {
        return 0;
    }
    let bytes = sync_dir_size_bytes(path);
    bytes / (1024 * 1024)
}

/// Walk a directory tree and sum file sizes. Unreadable entries are skipped.
fn sync_dir_size_bytes(path: &Path) -> u64 {
    let mut total = 0u64;
    let mut stack = vec![path.to_path_buf()];

    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let Ok(meta) = entry.metadata() else {
                continue;
            };
            if meta.is_dir() {
                stack.push(entry.path());
            } else {
                total += meta.len();
            }
        }
    }

    total
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::sync::Arc;

    use axum::extract::State;
    use tempfile::tempdir;

    use crate::deploy::create_backend;
    use crate::runtime::NoOpRuntime;
    use crate::state::AppState;

    fn test_state() -> (tempfile::TempDir, Arc<AppState>) {
        let dir = tempdir().expect("tempdir");
        let workdir = dir.path().to_path_buf();
        let deploy_backend =
            Arc::from(create_backend("manual", None, None, None).expect("manual backend"));
        let state = Arc::new(
            AppState::new(
                workdir,
                Arc::new(NoOpRuntime),
                roko_core::config::schema::RokoConfig::default(),
                deploy_backend,
            )
            .expect("AppState::new"),
        );
        (dir, state)
    }

    // ── compute_disk_usage ──────────────────────────────────────────────

    #[test]
    fn disk_usage_returns_valid_values_for_tempdir() {
        let dir = tempdir().expect("tempdir");
        let result = compute_disk_usage(dir.path()).expect("compute_disk_usage");

        assert!(result.total_mb > 0, "total_mb should be > 0");
        assert!(result.free_mb > 0, "free_mb should be > 0");
        assert!(result.used_pct <= 100, "used_pct should be 0-100");
        assert_eq!(result.pressure, "normal");
    }

    #[test]
    fn disk_usage_reports_roko_dir_size_when_present() {
        let dir = tempdir().expect("tempdir");
        let roko_dir = dir.path().join(".roko");
        std::fs::create_dir_all(&roko_dir).expect("create .roko dir");

        // Write some test data.
        let test_file = roko_dir.join("test.jsonl");
        std::fs::write(&test_file, "a".repeat(2 * 1024 * 1024)).expect("write test file");

        let result = compute_disk_usage(dir.path()).expect("compute_disk_usage");

        // 2 MB file should register as at least 1 MB (truncation).
        assert!(
            result.roko_dir_mb >= 1,
            "roko_dir_mb should be >= 1, got {}",
            result.roko_dir_mb
        );
    }

    #[test]
    fn disk_usage_reports_zero_for_absent_directories() {
        let dir = tempdir().expect("tempdir");
        let result = compute_disk_usage(dir.path()).expect("compute_disk_usage");

        assert_eq!(result.roko_dir_mb, 0, "roko_dir_mb should be 0");
        assert_eq!(result.target_dir_mb, 0, "target_dir_mb should be 0");
        assert_eq!(result.worktrees_mb, 0, "worktrees_mb should be 0");
    }

    // ── sync_dir_size_mb ────────────────────────────────────────────────

    #[test]
    fn sync_dir_size_mb_returns_zero_for_nonexistent_dir() {
        let result = sync_dir_size_mb(Path::new("/nonexistent/path/unlikely"));
        assert_eq!(result, 0);
    }

    #[test]
    fn sync_dir_size_mb_returns_zero_for_empty_dir() {
        let dir = tempdir().expect("tempdir");
        let result = sync_dir_size_mb(dir.path());
        assert_eq!(result, 0);
    }

    #[test]
    fn sync_dir_size_mb_counts_nested_files() {
        let dir = tempdir().expect("tempdir");
        let nested = dir.path().join("a").join("b");
        std::fs::create_dir_all(&nested).expect("create nested dirs");

        // Write 3 MB across two files.
        std::fs::write(
            dir.path().join("a").join("file1.dat"),
            "x".repeat(1024 * 1024),
        )
        .expect("write file1");
        std::fs::write(nested.join("file2.dat"), "y".repeat(2 * 1024 * 1024)).expect("write file2");

        let result = sync_dir_size_mb(dir.path());
        assert!(
            result >= 2,
            "expected >= 2 MB for 3 MB of test data, got {result}"
        );
    }

    // ── DiskUsageResponse serde ────────────────────────────────────────

    #[test]
    fn disk_usage_response_serde_roundtrip() {
        let response = DiskUsageResponse {
            free_mb: 50_000,
            total_mb: 500_000,
            used_pct: 90,
            roko_dir_mb: 128,
            target_dir_mb: 4096,
            worktrees_mb: 512,
            pressure: "warning".to_string(),
        };

        let json = serde_json::to_string(&response).expect("serialize");
        let back: DiskUsageResponse = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(response, back);
    }

    // ── handler integration ─────────────────────────────────────────────

    #[tokio::test]
    async fn disk_usage_handler_returns_ok() {
        let (_dir, state) = test_state();
        let result = disk_usage(State(state)).await;
        assert!(result.is_ok(), "disk_usage handler should succeed");

        let json = result.unwrap().0;
        assert!(
            json["free_mb"].as_u64().is_some(),
            "free_mb should be present"
        );
        assert!(
            json["total_mb"].as_u64().is_some(),
            "total_mb should be present"
        );
        assert!(
            json["used_pct"].as_u64().is_some(),
            "used_pct should be present"
        );
        assert!(
            json["pressure"].as_str().is_some(),
            "pressure should be present"
        );
        // Directory sizes should be present (possibly 0).
        assert!(
            json["roko_dir_mb"].as_u64().is_some(),
            "roko_dir_mb should be present"
        );
        assert!(
            json["target_dir_mb"].as_u64().is_some(),
            "target_dir_mb should be present"
        );
        assert!(
            json["worktrees_mb"].as_u64().is_some(),
            "worktrees_mb should be present"
        );
    }

    #[tokio::test]
    async fn disk_usage_handler_with_roko_dir() {
        let (dir, state) = test_state();

        // Create .roko dir with test data.
        let roko_dir = dir.path().join(".roko");
        std::fs::create_dir_all(&roko_dir).expect("create .roko dir");
        std::fs::write(roko_dir.join("signals.jsonl"), "x".repeat(1024 * 1024))
            .expect("write test data");

        let result = disk_usage(State(state)).await;
        assert!(result.is_ok(), "disk_usage handler should succeed");

        let json = result.unwrap().0;
        // The .roko dir has 1 MB of test data.
        let roko_mb = json["roko_dir_mb"].as_u64().unwrap_or(0);
        assert!(
            roko_mb >= 1,
            "roko_dir_mb should be >= 1 for 1 MB of test data, got {roko_mb}"
        );
    }
}
