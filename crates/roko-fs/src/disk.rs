//! Cross-platform disk space checker and runtime monitor.
//!
//! Provides [`available_disk_mb`] for querying free disk space at a given
//! path, and [`DiskMonitor`] for threshold-based pre-run checks and
//! continuous background monitoring.
//!
//! # Platform support
//!
//! Uses [`sysinfo::Disks`] which is cross-platform (macOS, Linux, Windows).
//! Falls back to an `io::Error` with `ErrorKind::Unsupported` when no
//! matching mount point can be found.
//!
//! # Background watcher
//!
//! [`DiskMonitor::start_watching`] spawns a background `tokio` task that
//! polls disk space at a configurable interval and publishes [`DiskStatus`]
//! updates on a [`tokio::sync::watch`] channel. Callers hold a
//! `watch::Receiver<DiskStatus>` and can `borrow()` the latest value at any
//! time without blocking.

use std::{
    io,
    path::{Path, PathBuf},
    time::Duration,
};

use serde::{Deserialize, Serialize};
use sysinfo::Disks;
use tokio::sync::watch;
use tracing::{debug, warn};

/// Return available disk space at the mount point containing `path`, in MB.
///
/// Finds the sysinfo disk whose mount point is the longest prefix of `path`,
/// then returns its `available_space / (1024 * 1024)`.
///
/// # Errors
///
/// Returns `io::Error(ErrorKind::Unsupported)` when no mount point can be
/// matched for the given path (unlikely on supported platforms).
pub fn available_disk_mb(path: &Path) -> io::Result<u64> {
    let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());

    let disks = Disks::new_with_refreshed_list();
    let best = best_disk_for(&disks, &canonical)?;
    let bytes = best.available_space();
    Ok(bytes / (1024 * 1024))
}

/// Find the disk whose mount point is the longest prefix of `path`.
///
/// This correctly handles nested mount points (e.g. `/` and `/data`).
fn best_disk_for<'d>(disks: &'d Disks, path: &Path) -> io::Result<&'d sysinfo::Disk> {
    disks
        .iter()
        .filter(|d| path.starts_with(d.mount_point()))
        .max_by_key(|d| d.mount_point().as_os_str().len())
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::Unsupported,
                format!("no disk mount point found for path: {}", path.display()),
            )
        })
}

// ── DiskStatus ───────────────────────────────────────────────────────────────

/// Current disk space status relative to configured thresholds.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum DiskStatus {
    /// Free space is above the warning threshold — all clear.
    Ok {
        /// Available space in MB at the monitored path.
        free_mb: u64,
    },
    /// Free space is below the warning threshold but above the critical threshold.
    Warning {
        /// Available space in MB at the monitored path.
        free_mb: u64,
    },
    /// Free space is below the critical (minimum) threshold — plan runs will
    /// be refused.
    Critical {
        /// Available space in MB at the monitored path.
        free_mb: u64,
    },
}

impl DiskStatus {
    /// Free space value regardless of variant.
    #[must_use]
    pub fn free_mb(&self) -> u64 {
        match self {
            Self::Ok { free_mb } | Self::Warning { free_mb } | Self::Critical { free_mb } => {
                *free_mb
            }
        }
    }

    /// Returns `true` if the status indicates disk space is OK.
    #[must_use]
    pub fn is_ok(&self) -> bool {
        matches!(self, Self::Ok { .. })
    }

    /// Returns `true` if the status is `Critical`.
    #[must_use]
    pub fn is_critical(&self) -> bool {
        matches!(self, Self::Critical { .. })
    }
}

// ── DiskWarning ──────────────────────────────────────────────────────────────

/// A non-fatal warning about disk space; returned by [`DiskMonitor::check_warning`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiskWarning {
    /// Available space in MB.
    pub free_mb: u64,
    /// The warn threshold that was breached.
    pub threshold_mb: u64,
}

// ── DiskError ────────────────────────────────────────────────────────────────

/// Fatal disk-space error; returned by [`DiskMonitor::check_pre_run`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiskError {
    /// Available space in MB.
    pub free_mb: u64,
    /// The minimum required space that was not met.
    pub required_mb: u64,
    /// Path that was checked.
    pub path: PathBuf,
}

impl std::fmt::Display for DiskError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "insufficient disk space at {}: {} MB available, {} MB required",
            self.path.display(),
            self.free_mb,
            self.required_mb,
        )
    }
}

impl std::error::Error for DiskError {}

// ── DiskMonitor ──────────────────────────────────────────────────────────────

/// Monitors disk space at a given path and enforces configurable thresholds.
///
/// Constructed from [`roko_core::config::schema::ResourcesConfig`] thresholds:
/// - `min_free_disk_mb` — hard lower bound; [`check_pre_run`] returns `Err` below this.
/// - `warn_disk_mb` — soft threshold; [`check_warning`] returns `Some` below this.
///
/// [`check_pre_run`]: DiskMonitor::check_pre_run
/// [`check_warning`]: DiskMonitor::check_warning
#[derive(Debug, Clone)]
pub struct DiskMonitor {
    /// Minimum free disk space (MB) required to start a plan run.
    pub min_free_mb: u64,
    /// Free disk space (MB) at which a warning is emitted.
    pub warn_mb: u64,
}

impl DiskMonitor {
    /// Create a `DiskMonitor` with explicit thresholds.
    #[must_use]
    pub fn new(min_free_mb: u64, warn_mb: u64) -> Self {
        Self {
            min_free_mb,
            warn_mb,
        }
    }

    /// Check whether free disk space at `workdir` satisfies the minimum threshold.
    ///
    /// Returns `Ok(())` when space is sufficient, or `Err(DiskError)` when
    /// free space is below `min_free_mb`. Wraps any I/O error in an `io::Error`.
    ///
    /// # Errors
    ///
    /// Returns `Err(DiskError)` when free disk is below `min_free_mb`.
    /// Returns `Err` wrapped in an inner `io::Error` when the underlying
    /// syscall fails.
    pub fn check_pre_run(&self, workdir: &Path) -> Result<(), DiskError> {
        let free_mb = available_disk_mb(workdir).unwrap_or(u64::MAX);
        if free_mb < self.min_free_mb {
            Err(DiskError {
                free_mb,
                required_mb: self.min_free_mb,
                path: workdir.to_path_buf(),
            })
        } else {
            Ok(())
        }
    }

    /// Check whether free disk space at `workdir` is below the warning threshold.
    ///
    /// Returns `Some(DiskWarning)` when space is below `warn_mb` but
    /// above `min_free_mb`. Returns `None` when space is sufficient.
    pub fn check_warning(&self, workdir: &Path) -> Option<DiskWarning> {
        let free_mb = available_disk_mb(workdir).ok()?;
        if free_mb < self.warn_mb && free_mb >= self.min_free_mb {
            Some(DiskWarning {
                free_mb,
                threshold_mb: self.warn_mb,
            })
        } else {
            None
        }
    }

    /// Spawn a background task that polls disk space at `interval` and
    /// publishes [`DiskStatus`] updates on a watch channel.
    ///
    /// The returned `Receiver` can be cloned and shared across tasks. It
    /// always holds the most recent status; call `.borrow()` to read it
    /// without blocking.
    ///
    /// The background task runs until all `Receiver`s are dropped.
    #[must_use]
    pub fn start_watching(
        &self,
        workdir: &Path,
        interval: Duration,
    ) -> watch::Receiver<DiskStatus> {
        let workdir = workdir.to_path_buf();
        let min_free_mb = self.min_free_mb;
        let warn_mb = self.warn_mb;

        // Compute initial status before spawning (best-effort).
        let initial = available_disk_mb(&workdir)
            .map(|free_mb| classify_static(free_mb, min_free_mb, warn_mb))
            .unwrap_or(DiskStatus::Ok { free_mb: u64::MAX });

        let (tx, rx) = watch::channel(initial);

        tokio::spawn(async move {
            loop {
                tokio::time::sleep(interval).await;

                // Use spawn_blocking so we don't block the async runtime on
                // the synchronous sysinfo syscall.
                let workdir_clone = workdir.clone();
                let result =
                    tokio::task::spawn_blocking(move || available_disk_mb(&workdir_clone)).await;

                match result {
                    Ok(Ok(free_mb)) => {
                        let status = classify_static(free_mb, min_free_mb, warn_mb);
                        debug!(free_mb, "disk space poll");
                        // If all receivers are dropped, stop polling.
                        if tx.send(status).is_err() {
                            break;
                        }
                    }
                    Ok(Err(e)) => {
                        warn!(error = %e, "disk space poll failed");
                    }
                    Err(join_err) => {
                        warn!(error = %join_err, "disk space blocking task panicked");
                    }
                }
            }
        });

        rx
    }
}

/// Stateless classify helper (avoids borrowing `DiskMonitor` in `spawn`).
fn classify_static(free_mb: u64, min_free_mb: u64, warn_mb: u64) -> DiskStatus {
    if free_mb < min_free_mb {
        DiskStatus::Critical { free_mb }
    } else if free_mb < warn_mb {
        DiskStatus::Warning { free_mb }
    } else {
        DiskStatus::Ok { free_mb }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    /// Returns the root path of the current process's working directory, or `/tmp`.
    fn workdir() -> PathBuf {
        std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/tmp"))
    }

    // ── available_disk_mb ────────────────────────────────────────────────

    #[test]
    fn available_disk_mb_returns_nonzero() {
        let mb = available_disk_mb(&workdir()).expect("disk query should succeed");
        assert!(mb > 0, "available disk MB should be > 0, got {mb}");
    }

    #[test]
    fn available_disk_mb_tmp() {
        // /tmp is always present on Unix and is accessible for tests.
        let path = if cfg!(unix) {
            PathBuf::from("/tmp")
        } else {
            workdir()
        };
        let mb = available_disk_mb(&path).expect("disk query on /tmp should succeed");
        assert!(mb > 0, "available disk MB for /tmp should be > 0, got {mb}");
    }

    // ── DiskStatus classification ────────────────────────────────────────

    #[test]
    fn disk_status_ok_above_warn() {
        let status = classify_static(10_000, 2_048, 5_120);
        assert!(matches!(status, DiskStatus::Ok { free_mb: 10_000 }));
        assert!(status.is_ok());
        assert!(!status.is_critical());
        assert_eq!(status.free_mb(), 10_000);
    }

    #[test]
    fn disk_status_warning_between_thresholds() {
        // 3000 MB: below warn (5120) but above min (2048)
        let status = classify_static(3_000, 2_048, 5_120);
        assert!(matches!(status, DiskStatus::Warning { free_mb: 3_000 }));
        assert!(!status.is_ok());
        assert!(!status.is_critical());
        assert_eq!(status.free_mb(), 3_000);
    }

    #[test]
    fn disk_status_critical_below_min() {
        // 1000 MB: below min (2048)
        let status = classify_static(1_000, 2_048, 5_120);
        assert!(matches!(status, DiskStatus::Critical { free_mb: 1_000 }));
        assert!(!status.is_ok());
        assert!(status.is_critical());
        assert_eq!(status.free_mb(), 1_000);
    }

    #[test]
    fn disk_status_at_min_boundary_is_not_critical() {
        // Exactly at min is not critical (requires strictly <).
        let status = classify_static(2_048, 2_048, 5_120);
        assert!(
            !status.is_critical(),
            "exactly at min should not be critical"
        );
    }

    // ── DiskMonitor::check_pre_run ───────────────────────────────────────

    #[test]
    fn check_pre_run_succeeds_with_generous_threshold() {
        // Set threshold to 0 MB → should always pass.
        let monitor = DiskMonitor::new(0, 0);
        assert!(
            monitor.check_pre_run(&workdir()).is_ok(),
            "check_pre_run with 0 MB threshold should always succeed"
        );
    }

    #[test]
    fn check_pre_run_fails_with_extreme_threshold() {
        // Require more disk than any system has.
        let monitor = DiskMonitor::new(u64::MAX, u64::MAX);
        let err = monitor
            .check_pre_run(&workdir())
            .expect_err("should fail when requiring u64::MAX MB");
        assert!(err.free_mb < err.required_mb);
        // Display output should mention available and required space.
        let msg = err.to_string();
        assert!(msg.contains("MB available"), "message: {msg}");
        assert!(msg.contains("MB required"), "message: {msg}");
    }

    // ── DiskMonitor::check_warning ───────────────────────────────────────

    #[test]
    fn check_warning_none_when_space_sufficient() {
        // Very low warn threshold → no warning.
        let monitor = DiskMonitor::new(0, 0);
        assert!(
            monitor.check_warning(&workdir()).is_none(),
            "no warning when warn threshold is 0"
        );
    }

    #[test]
    fn check_warning_some_with_extreme_warn_threshold() {
        // Warn threshold = u64::MAX but min_free is 0
        // → free_mb < warn_mb and free_mb >= min_free_mb → Some.
        let monitor = DiskMonitor::new(0, u64::MAX);
        let warning = monitor.check_warning(&workdir());
        assert!(
            warning.is_some(),
            "should warn when warn threshold is u64::MAX"
        );
        if let Some(w) = warning {
            assert_eq!(w.threshold_mb, u64::MAX);
            assert!(w.free_mb > 0);
        }
    }

    #[test]
    fn check_warning_none_when_below_min() {
        // When free_mb < min_free_mb, check_warning returns None
        // (the situation is critical, not just a warning).
        let monitor = DiskMonitor::new(u64::MAX, u64::MAX);
        assert!(
            monitor.check_warning(&workdir()).is_none(),
            "no warning returned when space is already critical"
        );
    }

    // ── DiskMonitor::start_watching ──────────────────────────────────────

    #[tokio::test]
    async fn start_watching_initial_status_is_ok_with_zero_threshold() {
        let monitor = DiskMonitor::new(0, 0);
        let rx = monitor.start_watching(&workdir(), Duration::from_secs(60));
        let status = rx.borrow().clone();
        assert!(
            status.is_ok(),
            "initial status should be Ok with zero thresholds"
        );
    }

    #[tokio::test]
    async fn start_watching_initial_status_critical_with_extreme_threshold() {
        let monitor = DiskMonitor::new(u64::MAX, u64::MAX);
        let rx = monitor.start_watching(&workdir(), Duration::from_secs(60));
        let status = rx.borrow().clone();
        assert!(
            status.is_critical(),
            "initial status should be Critical with u64::MAX thresholds"
        );
    }

    // ── Serde round-trip ─────────────────────────────────────────────────

    #[test]
    fn disk_status_serde_roundtrip() {
        let statuses = vec![
            DiskStatus::Ok { free_mb: 8192 },
            DiskStatus::Warning { free_mb: 3072 },
            DiskStatus::Critical { free_mb: 512 },
        ];
        for status in statuses {
            let json = serde_json::to_string(&status).expect("serialize");
            let back: DiskStatus = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(status, back);
        }
    }

    #[test]
    fn disk_warning_serde_roundtrip() {
        let w = DiskWarning {
            free_mb: 4096,
            threshold_mb: 5120,
        };
        let json = serde_json::to_string(&w).expect("serialize");
        let back: DiskWarning = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(w, back);
    }

    #[test]
    fn disk_error_serde_roundtrip() {
        let e = DiskError {
            free_mb: 1024,
            required_mb: 2048,
            path: PathBuf::from("/workspace"),
        };
        let json = serde_json::to_string(&e).expect("serialize");
        let back: DiskError = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(e, back);
    }
}
