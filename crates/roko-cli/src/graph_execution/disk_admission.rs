//! Disk headroom of Graph plan runs (reg-7cf6f9).
//!
//! - [`check_free_disk`] refuses to start a run whose workdir has less free
//!   space than `[resources] min_free_disk_mb` (`plan run --force` skips the
//!   check), and warns below `warn_disk_mb`.
//! - [`DiskAdmission`] reserves the space a task attempt's worktree is
//!   expected to grow by ([`WORKTREE_GROWTH_ESTIMATE_MB`]) before the attempt
//!   starts, and holds it until the attempt ends. The headroom left for a new
//!   attempt is the free space at the workdir, less `min_free_disk_mb`, less
//!   every reservation still held. While a new attempt would not fit,
//!   admission is serialised: it waits for the running attempts to end, and
//!   then admits one at a time. Each admission publishes the canonical
//!   `worktree_count` metric on the run's conductor ring, and logs it with
//!   the headroom left.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use roko_conductor::watchers::worktree_count::WORKTREE_COUNT_METRIC;
use roko_core::config::ResourcesConfig;
use roko_core::{Body, Kind, Signal};

use crate::runner::conductor_adapter::ConductorRing;

/// Space an attempt's worktree is expected to grow by, in MB: a checkout and
/// its build artifacts. Runner-v2 reserved the same.
pub const WORKTREE_GROWTH_ESTIMATE_MB: u64 = 3 * 1024;

/// How often an admission that waits for headroom measures the free space
/// again, in case space was freed outside the run.
const PRESSURE_RECHECK: Duration = Duration::from_secs(5);

/// Refuse to start a plan run in `workdir` while it has less free disk than
/// `resources.min_free_disk_mb`, unless `force` (`plan run --force`), and
/// warn below `resources.warn_disk_mb`. A free space that cannot be measured
/// passes.
///
/// # Errors
///
/// Returns an error naming the free space and the threshold when the run
/// must not start.
pub fn check_free_disk(
    workdir: &Path,
    resources: &ResourcesConfig,
    force: bool,
) -> anyhow::Result<()> {
    let monitor = roko_fs::DiskMonitor::new(resources.min_free_disk_mb, resources.warn_disk_mb);
    match monitor.check_pre_run(workdir) {
        Ok(()) => {
            if let Some(warning) = monitor.check_warning(workdir) {
                tracing::warn!(
                    free_mb = warning.free_mb,
                    warn_disk_mb = warning.threshold_mb,
                    "free disk space is low"
                );
            }
            Ok(())
        }
        Err(error) if force => {
            tracing::warn!(%error, "--force: starting the plan run despite low disk space");
            Ok(())
        }
        Err(error) => Err(anyhow::anyhow!(
            "{error} ([resources] min_free_disk_mb); free some space, lower the threshold, or \
             pass --force"
        )),
    }
}

/// Measures the free space, in MB, of the filesystem that holds a path.
type FreeSpaceProbe = dyn Fn(&Path) -> std::io::Result<u64> + Send + Sync;

/// Counts the attempt worktrees a run tracks.
type WorktreeCounter = dyn Fn() -> usize + Send + Sync;

/// Disk-headroom admission of task attempts; see the module docs. Clones
/// share their reservations.
#[derive(Clone)]
pub struct DiskAdmission {
    workdir: PathBuf,
    min_free_mb: u64,
    growth_mb: u64,
    free_space: Arc<FreeSpaceProbe>,
    worktree_count: Option<Arc<WorktreeCounter>>,
    ring: Option<ConductorRing>,
    state: Arc<AdmissionState>,
}

/// The reservations the clones of one [`DiskAdmission`] share.
#[derive(Debug, Default)]
struct AdmissionState {
    /// Reservations the running attempts hold.
    held: parking_lot::Mutex<u64>,
    /// Woken whenever a reservation is released.
    released: tokio::sync::Notify,
}

impl std::fmt::Debug for DiskAdmission {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DiskAdmission")
            .field("workdir", &self.workdir)
            .field("min_free_mb", &self.min_free_mb)
            .field("growth_mb", &self.growth_mb)
            .field("held", &*self.state.held.lock())
            .finish_non_exhaustive()
    }
}

/// The space one running attempt holds; dropping it releases the space.
#[derive(Debug)]
pub struct DiskReservation {
    state: Arc<AdmissionState>,
}

impl Drop for DiskReservation {
    fn drop(&mut self) {
        {
            let mut held = self.state.held.lock();
            *held = held.saturating_sub(1);
        }
        self.state.released.notify_waiters();
    }
}

/// The headroom one admission decision sees.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Headroom {
    /// Space left for new attempts, in MB.
    remaining_mb: u64,
    /// A new attempt would not fit.
    pressure: bool,
}

impl DiskAdmission {
    /// Admission of attempts in `workdir`, with `resources.min_free_disk_mb`
    /// as the floor of free space it keeps.
    #[must_use]
    pub fn new(workdir: &Path, resources: &ResourcesConfig) -> Self {
        Self {
            workdir: workdir.to_path_buf(),
            min_free_mb: resources.min_free_disk_mb,
            growth_mb: WORKTREE_GROWTH_ESTIMATE_MB,
            free_space: Arc::new(roko_fs::available_disk_mb),
            worktree_count: None,
            ring: None,
            state: Arc::default(),
        }
    }

    /// Measure free space with `free_space` instead of the filesystem.
    #[must_use]
    pub fn with_free_space(
        mut self,
        free_space: impl Fn(&Path) -> std::io::Result<u64> + Send + Sync + 'static,
    ) -> Self {
        self.free_space = Arc::new(free_space);
        self
    }

    /// Count the run's attempt worktrees with `worktree_count`, for the
    /// `worktree_count` metric.
    #[must_use]
    pub fn with_worktree_count(
        mut self,
        worktree_count: impl Fn() -> usize + Send + Sync + 'static,
    ) -> Self {
        self.worktree_count = Some(Arc::new(worktree_count));
        self
    }

    /// Publish each admission's metrics on `ring`.
    #[must_use]
    pub fn with_ring(mut self, ring: ConductorRing) -> Self {
        self.ring = Some(ring);
        self
    }

    /// Reserve the space of one attempt, waiting while it would not fit and
    /// other attempts still hold reservations. With none held, an attempt is
    /// admitted even under pressure, so the run moves on one attempt at a
    /// time.
    pub async fn admit(&self) -> DiskReservation {
        let mut waiting = false;
        loop {
            let released = self.state.released.notified();
            let (headroom, held) = {
                let mut held = self.state.held.lock();
                let headroom = self.headroom(*held);
                let admitted = !headroom.pressure || *held == 0;
                if admitted {
                    *held = held.saturating_add(1);
                }
                (headroom, admitted.then_some(*held))
            };
            if let Some(held) = held {
                self.publish(headroom.remaining_mb);
                if headroom.pressure {
                    tracing::warn!(
                        remaining_mb = headroom.remaining_mb,
                        estimate_mb = self.growth_mb,
                        "disk pressure admits attempts one at a time"
                    );
                }
                tracing::debug!(held, "disk headroom reserved for an attempt");
                return DiskReservation {
                    state: Arc::clone(&self.state),
                };
            }
            if !waiting {
                waiting = true;
                self.publish(headroom.remaining_mb);
                tracing::warn!(
                    remaining_mb = headroom.remaining_mb,
                    estimate_mb = self.growth_mb,
                    min_free_disk_mb = self.min_free_mb,
                    "disk pressure: the attempt waits for running attempts to end"
                );
            }
            tokio::select! {
                () = released => {}
                () = tokio::time::sleep(PRESSURE_RECHECK) => {}
            }
        }
    }

    /// The headroom left for one more attempt while `held` reservations are
    /// held. A free space that cannot be measured never blocks.
    fn headroom(&self, held: u64) -> Headroom {
        let free_mb = (self.free_space)(&self.workdir).unwrap_or_else(|error| {
            tracing::debug!(%error, "free disk space unknown; admission does not wait");
            u64::MAX
        });
        let remaining_mb = free_mb
            .saturating_sub(self.min_free_mb)
            .saturating_sub(held.saturating_mul(self.growth_mb));
        Headroom {
            remaining_mb,
            pressure: self.growth_mb > remaining_mb,
        }
    }

    /// Publish the `worktree_count` metric, and log it with the headroom
    /// left. The headroom stays off the conductor ring: no watcher reads it,
    /// and admission already acts on it (backlog 2127).
    fn publish(&self, remaining_mb: u64) {
        let worktrees = self
            .worktree_count
            .as_ref()
            .map(|count| u64::try_from(count()).unwrap_or(u64::MAX));
        tracing::info!(
            worktree_count = worktrees,
            disk_budget_remaining = remaining_mb,
            "attempt disk admission"
        );
        let (Some(ring), Some(count)) = (&self.ring, worktrees) else {
            return;
        };
        ring.push(
            Signal::builder(Kind::Metric)
                .body(Body::text(WORKTREE_COUNT_METRIC))
                .tag("name", WORKTREE_COUNT_METRIC)
                .tag("value", count.to_string())
                .build(),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Resources whose floor is `min_free_mb`.
    fn resources(min_free_mb: u64) -> ResourcesConfig {
        ResourcesConfig {
            min_free_disk_mb: min_free_mb,
            ..ResourcesConfig::default()
        }
    }

    /// The metrics `ring` holds, as `(name, value)`.
    fn metrics(ring: &ConductorRing) -> Vec<(String, String)> {
        ring.snapshot()
            .iter()
            .map(|signal| {
                let tag = |key| signal.tag(key).unwrap_or_default().to_string();
                (tag("name"), tag("value"))
            })
            .collect()
    }

    /// Under a low disk budget a second attempt waits until the first one
    /// ends. Each admission, and the wait, publishes the canonical
    /// `worktree_count` metric; the headroom stays off the ring (backlog
    /// 2127).
    #[tokio::test]
    async fn disk_admission_blocks_under_low_budget() {
        let ring = ConductorRing::new();
        // Room above the floor for one attempt, and 10 MB more.
        let admission = DiskAdmission::new(Path::new("/workdir"), &resources(1_000))
            .with_free_space(|_| Ok(1_000 + WORKTREE_GROWTH_ESTIMATE_MB + 10))
            .with_worktree_count(|| 1)
            .with_ring(ring.clone());
        let first = admission.admit().await;
        let worktree_count = ("worktree_count".to_string(), "1".to_string());
        assert_eq!(metrics(&ring), [worktree_count.clone()]);

        let second = admission.admit();
        tokio::pin!(second);
        let waited = tokio::time::timeout(Duration::from_millis(200), &mut second).await;
        assert!(waited.is_err(), "the second attempt must wait for headroom");
        assert_eq!(
            metrics(&ring),
            [worktree_count.clone(), worktree_count],
            "the wait publishes the worktree count, and no headroom"
        );

        drop(first);
        let second = tokio::time::timeout(Duration::from_secs(5), second)
            .await
            .expect("the second attempt starts once the first one ends");
        drop(second);
        assert_eq!(*admission.state.held.lock(), 0);
    }

    /// With ample disk, attempts never wait; with no headroom at all, one
    /// attempt still runs at a time.
    #[tokio::test]
    async fn disk_admission_serialises_only_under_pressure() {
        let ample = DiskAdmission::new(Path::new("/workdir"), &resources(1_000))
            .with_free_space(|_| Ok(1_000 + 10 * WORKTREE_GROWTH_ESTIMATE_MB));
        let _first = ample.admit().await;
        let second = tokio::time::timeout(Duration::from_millis(200), ample.admit()).await;
        assert!(second.is_ok(), "ample disk admits attempts side by side");

        let full = DiskAdmission::new(Path::new("/workdir"), &resources(1_000))
            .with_free_space(|_| Ok(500));
        let only = tokio::time::timeout(Duration::from_millis(200), full.admit()).await;
        assert!(only.is_ok(), "with nothing running, one attempt runs");
        let next = tokio::time::timeout(Duration::from_millis(200), full.admit()).await;
        assert!(next.is_err(), "a second attempt waits");

        let unknown = DiskAdmission::new(Path::new("/workdir"), &resources(1_000))
            .with_free_space(|_| Err(std::io::Error::other("no disk")));
        let _first = unknown.admit().await;
        let second = tokio::time::timeout(Duration::from_millis(200), unknown.admit()).await;
        assert!(second.is_ok(), "an unknown free space never blocks");
    }

    /// A run refuses to start below `min_free_disk_mb`, naming the free space
    /// and the threshold, unless it is forced.
    #[test]
    fn a_run_refuses_to_start_below_the_free_disk_threshold() {
        let dir = tempfile::tempdir().expect("tempdir");
        let error = check_free_disk(dir.path(), &resources(u64::MAX / 2), false)
            .expect_err("no disk has that much free space");
        let message = error.to_string();
        assert!(message.contains("MB available"), "{message}");
        assert!(
            message.contains(&format!("{} MB required", u64::MAX / 2)),
            "{message}"
        );
        assert!(message.contains("min_free_disk_mb"), "{message}");
        check_free_disk(dir.path(), &resources(u64::MAX / 2), true)
            .expect("--force starts the run");
        check_free_disk(dir.path(), &resources(0), false).expect("no floor, no refusal");
    }
}
