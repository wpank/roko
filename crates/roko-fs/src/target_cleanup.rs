//! Rust build artifact cleanup for workspace and worktree `target/` directories.
//!
//! Each worktree checkout accumulates a `target/` directory that can consume
//! 2-10 GB. This module finds stale ones, reports their sizes, and removes them
//! when they exceed the configured age threshold.
//!
//! # Safety guards
//!
//! - The main workspace `target/` is never deleted if it was modified within
//!   the last 24 hours (a live build may be in progress).
//! - Symlinks are never followed into `target/` — only real directories.
//! - `cargo clean` calls across multiple worktrees are serialized to avoid
//!   Cargo lock contention.

use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

static CARGO_CLEAN_LOCK: OnceLock<tokio::sync::Mutex<()>> = OnceLock::new();

/// Metadata for a discovered `target/` directory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TargetDir {
    /// Absolute path to the `target/` directory.
    pub path: PathBuf,
    /// Total size of the directory tree in bytes.
    pub size_bytes: u64,
    /// Last-modified time of the directory itself (mtime).
    pub last_modified: SystemTime,
}

impl TargetDir {
    /// Age of this directory in whole days (rounded down).
    #[must_use]
    pub fn age_days(&self) -> u32 {
        let now = SystemTime::now();
        let elapsed = now
            .duration_since(self.last_modified)
            .unwrap_or(Duration::ZERO);
        u32::try_from(elapsed.as_secs() / 86_400).unwrap_or(u32::MAX)
    }
}

/// Summary of a target directory cleanup pass.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CleanupReport {
    /// Number of `target/` directories found during the scan.
    pub dirs_scanned: usize,
    /// Number of directories successfully removed.
    pub dirs_removed: usize,
    /// Total bytes freed by the removal.
    pub bytes_freed: u64,
}

/// Recursively scan `root` for all `target/` directories.
///
/// Scans the workspace root and any worktree checkouts discovered under it.
/// Symlinks are not followed — only real directories named `target` are
/// included. Returns results sorted by last-modified time (oldest first).
///
/// # Errors
///
/// Returns an error if `root` cannot be read. Unreadable subdirectories are
/// silently skipped so a single permission error does not abort the whole scan.
pub async fn scan_target_dirs(root: &Path) -> std::io::Result<Vec<TargetDir>> {
    let mut results = Vec::new();

    // Iterative DFS using an explicit stack to avoid async recursion.
    // We do not recurse into `target/` directories once found.
    let mut dirs_to_visit: Vec<PathBuf> = vec![root.to_path_buf()];
    // Hidden directories are skipped below, but canonical runner worktrees
    // live under `.roko/worktrees/` and are the primary cleanup target.
    // Seed that directory explicitly without opening the rest of `.roko/`.
    let canonical_worktrees = root.join(".roko").join("worktrees");
    if tokio::fs::symlink_metadata(&canonical_worktrees)
        .await
        .is_ok_and(|metadata| metadata.is_dir() && !metadata.file_type().is_symlink())
    {
        dirs_to_visit.push(canonical_worktrees);
    }

    while let Some(dir) = dirs_to_visit.pop() {
        let Ok(mut entries) = tokio::fs::read_dir(&dir).await else {
            continue;
        };

        while let Ok(Some(entry)) = entries.next_entry().await {
            // Check the directory entry type before metadata: `metadata()`
            // follows symlinks on supported platforms, which could otherwise
            // make a `target` symlink escape the workspace cleanup boundary.
            let Ok(file_type) = entry.file_type().await else {
                continue;
            };
            if file_type.is_symlink() || !file_type.is_dir() {
                continue;
            }
            let Ok(meta) = entry.metadata().await else {
                continue;
            };

            let name = entry.file_name();
            let name_str = name.to_string_lossy();

            if name_str == "target" {
                let size = compute_dir_size_bytes(&entry.path()).await;
                let last_modified = meta.modified().unwrap_or(UNIX_EPOCH);
                results.push(TargetDir {
                    path: entry.path(),
                    size_bytes: size,
                    last_modified,
                });
                // Do not recurse into target/ itself.
                continue;
            }

            // Skip hidden directories (e.g. .git) to avoid excessive scanning.
            if name_str.starts_with('.') {
                continue;
            }

            dirs_to_visit.push(entry.path());
        }
    }

    // Sort oldest-first so callers can process from least-recently-used.
    results.sort_by_key(|t| t.last_modified);
    Ok(results)
}

/// Remove `target/` directories that are older than `max_age_days`.
///
/// The main workspace's `target/` (the one directly under `root`) is
/// protected by a 24-hour safety guard: it is never deleted if it was
/// modified within the last 24 hours, even if `max_age_days` would select it.
///
/// Returns a `CleanupReport` describing how many directories were removed
/// and how many bytes were freed. Removal errors are logged but do not
/// abort subsequent removals.
///
/// # Errors
///
/// Returns an error if `scan_target_dirs` itself fails. Individual removal
/// failures do not propagate as errors — they are counted in the report.
pub async fn clean_stale_targets(root: &Path, max_age_days: u32) -> std::io::Result<CleanupReport> {
    let dirs = scan_target_dirs(root).await?;
    let main_target = root.join("target");
    let cutoff_secs = u64::from(max_age_days) * 86_400;
    let safety_window_secs: u64 = 24 * 3600;
    let now_secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::ZERO)
        .as_secs();

    let mut report = CleanupReport {
        dirs_scanned: dirs.len(),
        dirs_removed: 0,
        bytes_freed: 0,
    };

    for dir in dirs {
        let modified_secs = dir
            .last_modified
            .duration_since(UNIX_EPOCH)
            .unwrap_or(Duration::ZERO)
            .as_secs();
        let age_secs = now_secs.saturating_sub(modified_secs);

        // Age check: skip directories that are not stale yet.
        if age_secs < cutoff_secs {
            continue;
        }

        // Safety guard: never remove the main workspace target/ if it was
        // touched within the last 24 hours (a build may be active).
        if dir.path == main_target && age_secs < safety_window_secs {
            tracing::debug!(
                path = %dir.path.display(),
                age_secs,
                "skipping main workspace target/ (24h safety guard)"
            );
            continue;
        }

        tracing::info!(
            path = %dir.path.display(),
            size_mb = dir.size_bytes / (1024 * 1024),
            age_days = age_secs / 86_400,
            "removing stale target/ directory"
        );

        match tokio::fs::remove_dir_all(&dir.path).await {
            Ok(()) => {
                report.dirs_removed += 1;
                report.bytes_freed += dir.size_bytes;
            }
            Err(e) => {
                tracing::warn!(
                    path = %dir.path.display(),
                    error = %e,
                    "failed to remove stale target/ directory"
                );
            }
        }
    }

    Ok(report)
}

/// Run `cargo clean` in the given directory.
///
/// This invokes `cargo clean` as a subprocess, which removes the `target/`
/// directory for the workspace rooted at `workdir`. It is suitable for
/// precise cleanup of a single worktree after its gate has passed.
///
/// Calls are intentionally synchronous-looking from the caller's perspective
/// (the future awaits the subprocess). Do NOT call this concurrently for
/// multiple worktrees — Cargo may hold internal locks.
///
/// # Errors
///
/// Returns an error if the subprocess cannot be spawned, exits with a
/// non-zero status, or produces unexpected output.
pub async fn cargo_clean(workdir: &Path) -> std::io::Result<()> {
    let _clean_guard = CARGO_CLEAN_LOCK
        .get_or_init(|| tokio::sync::Mutex::new(()))
        .lock()
        .await;
    let output = tokio::process::Command::new("cargo")
        .arg("clean")
        .current_dir(workdir)
        .output()
        .await?;

    if output.status.success() {
        tracing::debug!(
            workdir = %workdir.display(),
            "cargo clean completed"
        );
        return Ok(());
    }

    let stderr = String::from_utf8_lossy(&output.stderr);
    Err(std::io::Error::other(format!(
        "cargo clean failed in {}: {}",
        workdir.display(),
        stderr.trim()
    )))
}

// ── private helpers ───────────────────────────────────────────────────────────

/// Compute the total size of a directory tree in bytes.
///
/// Best-effort: unreadable entries are silently skipped.
async fn compute_dir_size_bytes(path: &Path) -> u64 {
    let mut total = 0u64;
    let mut stack = vec![path.to_path_buf()];

    while let Some(dir) = stack.pop() {
        let Ok(mut entries) = tokio::fs::read_dir(&dir).await else {
            continue;
        };
        while let Ok(Some(entry)) = entries.next_entry().await {
            let Ok(file_type) = entry.file_type().await else {
                continue;
            };
            if file_type.is_symlink() {
                continue;
            }
            let Ok(meta) = entry.metadata().await else {
                continue;
            };
            if file_type.is_dir() {
                stack.push(entry.path());
            } else {
                total += meta.len();
            }
        }
    }

    total
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    /// Create a file with the given content inside `dir`.
    async fn write_file(dir: &Path, name: &str, content: &[u8]) {
        tokio::fs::write(dir.join(name), content).await.unwrap();
    }

    #[tokio::test]
    async fn scan_finds_target_dirs() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();

        // Create a target/ dir with one file.
        let target = root.join("target");
        tokio::fs::create_dir_all(&target).await.unwrap();
        write_file(&target, "artifact.rlib", b"binary data").await;

        // Create an unrelated directory — should not appear in results.
        tokio::fs::create_dir_all(root.join("src")).await.unwrap();

        let dirs = scan_target_dirs(root).await.unwrap();
        assert_eq!(dirs.len(), 1, "expected exactly one target/ dir");
        assert_eq!(dirs[0].path, target);
        assert!(dirs[0].size_bytes > 0, "size_bytes should be non-zero");
    }

    #[tokio::test]
    async fn scan_finds_nested_target_dirs() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();

        // Simulate a worktree: root/worktrees/plan-a/target/
        let worktree_target = root.join("worktrees").join("plan-a").join("target");
        tokio::fs::create_dir_all(&worktree_target).await.unwrap();
        write_file(&worktree_target, "deps", b"data").await;

        // Root-level target/ as well.
        let main_target = root.join("target");
        tokio::fs::create_dir_all(&main_target).await.unwrap();

        let dirs = scan_target_dirs(root).await.unwrap();
        assert_eq!(dirs.len(), 2, "should find both target/ directories");

        let paths: Vec<_> = dirs.iter().map(|d| d.path.clone()).collect();
        assert!(paths.contains(&main_target));
        assert!(paths.contains(&worktree_target));
    }

    #[tokio::test]
    async fn scan_enters_canonical_hidden_roko_worktrees() {
        let tmp = TempDir::new().unwrap();
        let target = tmp
            .path()
            .join(".roko")
            .join("worktrees")
            .join("plan-a")
            .join("target");
        tokio::fs::create_dir_all(&target).await.unwrap();
        write_file(&target, "artifact", b"data").await;

        let dirs = scan_target_dirs(tmp.path()).await.unwrap();
        assert_eq!(dirs.len(), 1);
        assert_eq!(dirs[0].path, target);
    }

    #[tokio::test]
    async fn scan_does_not_recurse_into_target() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();

        // A target/ dir containing another "target" dir inside it —
        // we should NOT recurse into the outer target/.
        let outer = root.join("target");
        let inner = outer.join("debug").join("target");
        tokio::fs::create_dir_all(&inner).await.unwrap();

        let dirs = scan_target_dirs(root).await.unwrap();
        // Only the outer target/ should appear.
        assert_eq!(dirs.len(), 1);
        assert_eq!(dirs[0].path, outer);
    }

    #[tokio::test]
    async fn scan_returns_empty_for_no_targets() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();

        tokio::fs::create_dir_all(root.join("src")).await.unwrap();
        tokio::fs::create_dir_all(root.join("tests")).await.unwrap();

        let dirs = scan_target_dirs(root).await.unwrap();
        assert!(dirs.is_empty());
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn scan_never_follows_target_symlink_outside_root() {
        use std::os::unix::fs::symlink;

        let root = TempDir::new().unwrap();
        let outside = TempDir::new().unwrap();
        tokio::fs::write(outside.path().join("do-not-delete"), b"important")
            .await
            .unwrap();
        symlink(outside.path(), root.path().join("target")).unwrap();

        let dirs = scan_target_dirs(root.path()).await.unwrap();
        assert!(dirs.is_empty());
        let report = clean_stale_targets(root.path(), 0).await.unwrap();
        assert_eq!(report.dirs_removed, 0);
        assert!(outside.path().join("do-not-delete").exists());
    }

    #[tokio::test]
    async fn cleanup_report_counts_scanned_dirs() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();

        // Create two target/ dirs.
        tokio::fs::create_dir_all(root.join("target"))
            .await
            .unwrap();
        tokio::fs::create_dir_all(root.join("a").join("target"))
            .await
            .unwrap();

        // Use a very large max_age_days so nothing gets deleted.
        let report = clean_stale_targets(root, 9999).await.unwrap();
        assert_eq!(report.dirs_scanned, 2);
        assert_eq!(report.dirs_removed, 0);
        assert_eq!(report.bytes_freed, 0);
    }

    #[tokio::test]
    async fn cleanup_removes_old_non_main_target() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();

        // Non-main target/ under a worktree directory.
        let worktree_target = root.join("worktrees").join("plan-x").join("target");
        tokio::fs::create_dir_all(&worktree_target).await.unwrap();
        write_file(&worktree_target, "artifact", b"big blob").await;

        // Use max_age_days=0: every directory is stale.
        let report = clean_stale_targets(root, 0).await.unwrap();

        assert_eq!(report.dirs_scanned, 1);
        assert_eq!(report.dirs_removed, 1);
        assert!(report.bytes_freed > 0);
        assert!(
            !worktree_target.exists(),
            "stale worktree target/ should be gone"
        );
    }

    #[tokio::test]
    async fn cleanup_applies_24h_guard_to_main_workspace() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();

        // Create the main workspace target/ (just-modified).
        let main_target = root.join("target");
        tokio::fs::create_dir_all(&main_target).await.unwrap();
        write_file(&main_target, "freshly_built.rlib", b"data").await;

        // max_age_days=0 would normally delete everything, but the 24h guard
        // should protect the main target/ since it was just touched.
        let report = clean_stale_targets(root, 0).await.unwrap();

        assert_eq!(report.dirs_scanned, 1);
        // The main target/ should NOT have been removed.
        assert_eq!(report.dirs_removed, 0, "main target/ should be protected");
        assert!(main_target.exists(), "main target/ must still exist");
    }

    #[tokio::test]
    async fn target_dir_age_days_is_zero_for_new_dir() {
        let t = TargetDir {
            path: PathBuf::from("/tmp/target"),
            size_bytes: 0,
            last_modified: SystemTime::now(),
        };
        assert_eq!(t.age_days(), 0);
    }

    #[tokio::test]
    async fn target_dir_age_days_reflects_old_mtime() {
        // Set mtime to 10 days ago.
        let ten_days_ago = SystemTime::now() - Duration::from_secs(10 * 86_400);
        let t = TargetDir {
            path: PathBuf::from("/tmp/target"),
            size_bytes: 0,
            last_modified: ten_days_ago,
        };
        assert_eq!(t.age_days(), 10);
    }
}
